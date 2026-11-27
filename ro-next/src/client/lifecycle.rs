//! The client lifecycle: construction (`init`), the `run` loop and the state
//! updates that happen once per frame before rendering.
//!
//! The frame driver itself (input/network dispatch, simulation, render) lives
//! in `frame.rs`; this module only builds the client and drives its state.

use crate::*;

impl Client {
    pub fn init(sync_cache: bool) -> Option<Self> {
        crate::logging::log_info!("client: initializing (sync_cache={})", sync_cache);

        // We start a frame so that functions trying to start a measurement don't panic.
        #[cfg(feature = "debug")]
        let _measurement = threads::Main::start_frame();

        initialize_shutdown_signal();

        time_phase!("create global thread pool", {
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .start_handler(|_| init_tls_rand())
                .build_global()
                .unwrap();
        });

        time_phase!("seed main random instance", {
            init_tls_rand();
        });

        crate::logging::log_debug!("client: working directory is {:?}", std::env::current_dir());
        // Support logs have to say which effect backend is drawing; the value is
        // fixed for the session (`RO_NEXT_PORTED_FX=0` opts out of the official
        // bodies).
        crate::logging::log_info!(
            "client: ported effect backend {}",
            if crate::client::effects::ported_effects_enabled() { "enabled" } else { "disabled (RO_NEXT_PORTED_FX=0)" }
        );

        // Check if ro-next is in the correct working directory and if not, try to
        // correct it.
        // NOTE: This check might be temporary or feature gated in the future.
        time_phase!("adjust working directory", {
            if !std::fs::metadata("archive").is_ok_and(|metadata| metadata.is_dir()) {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to find archive directory, attempting to change working directory {}",
                    "warning".yellow(),
                    "ro-next".magenta()
                );

                if let Err(_error) = std::env::set_current_dir("ro-next") {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to change working directory: {:?}", "error".red(), _error);
                }
            }
        });

        time_phase!("load graphics settings", {
            let picker_value = Arc::new(AtomicU64::new(0));
            let directional_shadow_partitions = Arc::new(Mutex::new([DirectionalShadowPartition::default(); PARTITION_COUNT]));
            let input_system = InputSystem::new(picker_value.clone());
            let mut graphics_settings = GraphicsSettings::new();

            if graphics_settings.apply_environment_overrides() {
                println!(
                    "[framerate] environment overrides applied: max_fps={} background_max_fps={}",
                    graphics_settings.max_fps.get(),
                    graphics_settings.background_max_fps.get(),
                );
            }

            println!(
                "[framerate] frame rate limits: max_fps={} background_max_fps={}",
                graphics_settings.max_fps.get(),
                graphics_settings.background_max_fps.get(),
            );

            crate::logging::log_debug!(
                "client: graphics settings loaded (high_quality_interface={})",
                graphics_settings.high_quality_interface,
            );

            // Read this before `ClientState::new` loads (and thereby potentially
            // rewrites) the interface settings.
            let initialize_interface_scaling_from_dpi = !InterfaceSettings::exists();
        });

        let force_background =
            environment_flag(FORCE_UNFOCUSED_ENVIRONMENT_VARIABLE) || environment_flag(FORCE_UNFOCUSED_ALIAS_ENVIRONMENT_VARIABLE);

        if force_background {
            println!("[framerate] window is always treated as being in the background (environment override)");
        }

        time_phase!("create adapter", {
            let instance = Instance::new(InstanceDescriptor {
                backends: Backends::all().with_env(),
                flags: InstanceFlags::from_build_config().with_env(),
                memory_budget_thresholds: MemoryBudgetThresholds::default(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::Automatic,
                        fence_behavior: GlFenceBehavior::Normal,
                        debug_fns: GlDebugFns::Auto,
                    },
                    dx12: Dx12BackendOptions {
                        shader_compiler: Dx12Compiler::Auto.with_env(),
                        presentation_system: Dx12SwapchainKind::DxgiFromHwnd,
                        latency_waitable_object: Dx12UseFrameLatencyWaitableObject::Wait,
                        force_shader_model: ForceShaderModelToken::default(),
                        agility_sdk: None,
                    },
                    noop: NoopBackendOptions { enable: false },
                },
                // On Vulkan, Metal and Dx12, this is currently unused.
                display: None,
            });

            let adapter = pollster::block_on(async { initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });

            #[cfg(feature = "debug")]
            {
                let adapter_info = adapter.get_info();
                print_debug!("using adapter {} ({})", adapter_info.name, adapter_info.backend);
                print_debug!("using device {} ({})", adapter_info.device, adapter_info.vendor);
                print_debug!("using driver {} ({})", adapter_info.driver, adapter_info.driver_info);
            }
        });

        time_phase!("create device", {
            let capabilities = Capabilities::from_adapter(&adapter);

            let (device, queue) = pollster::block_on(async {
                adapter
                    .request_device(&DeviceDescriptor {
                        label: None,
                        required_features: capabilities.get_required_features(),
                        required_limits: capabilities.get_required_limits(),
                        experimental_features: ExperimentalFeatures::disabled(),
                        memory_hints: MemoryHints::Performance,
                        trace: Trace::Off,
                    })
                    .await
                    .unwrap()
            });

            #[cfg(feature = "debug")]
            device.on_uncaptured_error(Arc::new(error_handler));

            #[cfg(feature = "debug")]
            print_debug!("received {} and {}", "queue".magenta(), "device".magenta());
        });

        time_phase!("create shader compiler", {
            let shader_compiler = ShaderCompiler::new(device.clone());
        });

        time_phase!("create game file loader", {
            let game_file_loader = Arc::new(GameFileLoader::default());

            game_file_loader.load_archives_from_settings();
            game_file_loader.load_patched_lua_files();
        });

        // The message table is small (a few hundred KiB) and is needed for every
        // `ZC_MSG` system message, so load it once at startup.
        time_phase!("load message table", {
            let message_table = load_message_table(game_file_loader.as_ref());
        });

        // The protocol result-code texts are data as well (see the
        // `ro-next-messages` crate); an archive override wins over the
        // compiled fallback.
        time_phase!("load protocol messages", {
            load_protocol_messages(game_file_loader.as_ref());
        });

        // Client configuration data (default/login map, start-camera focus,
        // quest-icon colours and UI audio paths). Each file is loaded through
        // the archive with the compiled fallback, so a server can override it
        // without a rebuild.
        time_phase!("load client config", {
            crate::client_config::load(game_file_loader.as_ref());
        });

        // Official damage-number digit/style/colour tables
        // (`data\client\damage_numbers.ron`, generated by
        // `tools/gen_damage_numbers.py`).
        time_phase!("load damage number tables", {
            crate::world::damage_number::load(game_file_loader.as_ref());
        });

        // The data-driven keyboard bindings (window shortcuts, hotbar/emotion
        // keys, confirm keys); an archive override wins over the compiled
        // fallback, see `input::keybinds`.
        time_phase!("load keybinds", {
            crate::input::keybinds::load(game_file_loader.as_ref());
        });

        // The data-driven paper-doll slot layout (equipment window); an archive
        // override wins over the compiled fallback, see
        // `interface::components::paper_doll_slots`.
        time_phase!("load paper doll slots", {
            crate::interface::components::paper_doll_slots::load(game_file_loader.as_ref());
        });

        time_phase!("calculate game file hash", {
            let game_file_hash = game_file_loader.calculate_hash();
            #[cfg(feature = "debug")]
            print_debug!("game file hash: {}", game_file_hash);
        });

        time_phase!("create audio engine", {
            let audio_engine = Arc::new(AudioEngine::new(game_file_loader.clone()));
        });

        let skill_sounds = SkillSoundPlayer::new(audio_engine.clone(), game_file_loader.clone());

        if skill_sounds.is_muted() {
            println!("[sound] muted mode active: skill sound playback is logged, not audible");
        }

        time_phase!("create resource managers", {
            std::fs::create_dir_all(MENU_THEMES_PATH).unwrap();
            std::fs::create_dir_all(IN_GAME_THEMES_PATH).unwrap();
            std::fs::create_dir_all(WORLD_THEMES_PATH).unwrap();

            let model_loader = Arc::new(ModelLoader::new(game_file_loader.clone(), capabilities.bindless_support()));
            let texture_loader = Arc::new(TextureLoader::new(
                device.clone(),
                queue.clone(),
                &shader_compiler,
                &capabilities,
                game_file_loader.clone(),
            ));
            let video_loader = Arc::new(VideoLoader::new(game_file_loader.clone(), texture_loader.clone()));
            // `NotoSans` only covers Latin/Greek/Cyrillic. The server sends free
            // text (chat, NPC dialogue) and the 2019 quest scripts use KS X 1001
            // symbols, so a Hangul/KS-capable fallback font is required to avoid
            // `.notdef` boxes. See `archive/data/font/README.md`.
            let font_loader = Arc::new(FontLoader::new(
                &["NotoSans".to_owned(), "NanumGothic".to_owned()],
                &game_file_loader,
                &device,
                &queue,
            ));
            let map_loader = Arc::new(MapLoader::new(
                device.clone(),
                queue.clone(),
                game_file_loader.clone(),
                audio_engine.clone(),
                capabilities.bindless_support(),
            ));
            let sprite_loader = Arc::new(SpriteLoader::new(game_file_loader.clone(), texture_loader.clone()));
            let action_loader = Arc::new(ActionLoader::new(game_file_loader.clone(), audio_engine.clone()));
            let effect_loader = Arc::new(EffectLoader::new(game_file_loader.clone()));
            let animation_loader = Arc::new(AnimationLoader::new());

            let library = Arc::new(Library::new(&game_file_loader).unwrap_or_else(|_| {
                // The library not being created correctly means that the lua files were
                // not valid. It's possible that the archive was copied from a
                // different machine with a different architecture, so the one thing
                // we can try is generating it again.

                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to execute lua files; attempting to fix it by re-patching",
                    "error".red()
                );

                game_file_loader.remove_patched_lua_files();
                game_file_loader.load_patched_lua_files();

                Library::new(&game_file_loader).unwrap()
            }));

            if sync_cache {
                sync_cache_archive(&game_file_loader, texture_loader, game_file_hash);
                return None;
            }

            game_file_loader.load_cache_archive(game_file_hash);

            let async_loader = Arc::new(AsyncLoader::new(
                action_loader.clone(),
                animation_loader.clone(),
                game_file_loader.clone(),
                map_loader.clone(),
                model_loader.clone(),
                sprite_loader.clone(),
                texture_loader.clone(),
                video_loader.clone(),
                library.clone(),
            ));

            let interface_renderer = InterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                font_loader.clone(),
                texture_loader.clone(),
                graphics_settings.high_quality_interface,
            );
            let bottom_interface_renderer = GameInterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                INITIAL_SCALING_FACTOR,
                font_loader.clone(),
                #[cfg(feature = "debug")]
                &texture_loader,
            );
            let middle_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let top_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let effect_renderer = EffectRenderer::new(INITIAL_SCREEN_SIZE);
            #[cfg(feature = "debug")]
            let debug_marker_renderer = DebugMarkerRenderer::new();

            #[cfg(feature = "debug")]
            let aabb_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let circle_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let rectangle_instructions = Vec::default();
            let model_batches = Vec::default();
            let model_instructions = Vec::default();
            let entity_instructions = Vec::default();
            let directional_shadow_model_batches = Default::default();
            let directional_shadow_model_instructions = Vec::default();
            let directional_shadow_entity_instructions = Default::default();
            let point_shadow_model_batches = Vec::default();
            let point_shadow_model_instructions = Vec::default();
            let point_shadow_entity_instructions = Vec::default();
            let point_light_with_shadow_instructions = Vec::default();
            let point_light_instructions = Vec::default();
        });

        time_phase!("load warp portal assets", {
            let warp_portal = WarpPortal::new(&texture_loader, &sprite_loader);
            let weather = crate::world::Weather::new(&texture_loader);
        });

        time_phase!("load spirit sphere assets", {
            let spirit_sphere_texture = texture_loader.get_or_load("effect\\thunder_center.bmp", ImageType::Color).ok();
        });

        time_phase!("create graphics engine", {
            let graphics_engine = GraphicsEngine::initialize(GraphicsEngineDescriptor {
                capabilities,
                adapter,
                instance,
                device: device.clone(),
                queue: queue.clone(),
                shader_compiler,
                texture_loader: texture_loader.clone(),
                picker_value,
                directional_shadow_partitions: directional_shadow_partitions.clone(),
            });
        });

        time_phase!("initialize interface", {
            let mut interface = Interface::new(font_loader.clone(), INITIAL_SCREEN_SIZE);
            let mouse_cursor = MouseCursor::new(&sprite_loader, &action_loader);
            let show_interface = true;
        });

        time_phase!("initialize timer", {
            let game_timer = GameTimer::new();
        });

        time_phase!("initialize camera", {
            #[cfg(feature = "debug")]
            let debug_camera = DebugCamera::new();
            let mut start_camera = StartCamera::new();
            let player_camera = PlayerCamera::new();
            let mut directional_shadow_camera = DirectionalShadowCamera::new();
            let point_shadow_camera = PointShadowCamera::new();
            start_camera.set_focus_point(crate::client_config::start_camera_focus());
        });

        // TODO: Move all of these to the ClientState
        let saved_login_data: Option<LoginServerLoginData> = None;
        let saved_character_server: Option<CharacterServerInformation> = None;
        let saved_login_server_address = None;
        let saved_password = String::new();
        let saved_username = String::new();
        let saved_packet_version = FALLBACK_PACKET_VERSION;

        time_phase!("initialize networking", {
            #[cfg(not(feature = "debug"))]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(ClientPacketCallback);

            #[cfg(feature = "debug")]
            let (packet_history, packet_history_callback) = PacketHistory::new();
            #[cfg(feature = "debug")]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(packet_history_callback);
        });

        time_phase!("create resources", {
            let input_event_buffer = Vec::new();

            // The official sprite damage font (loose folder in the client
            // archive). Without it the damage numbers fall back to text.
            crate::world::damage_font::install(crate::world::damage_font::DamageFont::load(&sprite_loader, &action_loader));

            let mut particle_holder = ParticleHolder::default();
            // Arrow impact sounds are played from the particle update, so the
            // holder needs the audio engine (`_hit_arrow.wav`).
            particle_holder.install_audio(audio_engine.clone());
            let point_light_manager = PointLightManager::new();
            let effect_holder = EffectHolder::default();
            let path_finder = PathFinder::default();

            let point_light_set_buffer = ResourceSetBuffer::default();
            let directional_shadow_object_set_buffer = ResourceSetBuffer::default();
            let point_shadow_object_set_buffer = ResourceSetBuffer::default();
            let deferred_object_set_buffer = ResourceSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ResourceSetBuffer::default();

            #[cfg(feature = "debug")]
            let pathing_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "pathing", &[
                "pathing_goal.png",
                "pathing_straight.png",
                "pathing_diagonal.png",
            ]);
            #[cfg(feature = "debug")]
            let pathing_texture_set = Arc::new(pathing_texture_set);

            #[cfg(feature = "debug")]
            let tile_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "tile", &[
                "tile_0.png",
                "tile_1.png",
                "tile_2.png",
                "tile_3.png",
                "tile_4.png",
                "tile_5.png",
                "tile_6.png",
            ]);
            #[cfg(feature = "debug")]
            let tile_texture_set = Arc::new(tile_texture_set);

            // Data-driven UI audio paths (archive override of
            // `data\client\ui_audio.ron`, see `client_config::ui_audio`).
            let main_menu_click_sound_effect = crate::client_config::main_menu_click_sound().map(|path| audio_engine.load(path));
            let teleport_vanish_sound_effect = crate::client_config::teleport_vanish_sound().map(|path| audio_engine.load(path));
        });

        time_phase!("load default map", {
            let map = map_loader
                .load(
                    crate::client_config::default_map().to_string(),
                    &model_loader,
                    texture_loader.clone(),
                    video_loader,
                    &library,
                )
                .expect("failed to load initial map");

            directional_shadow_camera.set_level_bound(map.get_level_bound());

            audio_engine.play_background_music_track(crate::client_config::background_music());
            map.set_ambient_sound_sources(&audio_engine);
        });

        time_phase!("create client state", {
            let mut client_state = State::new(ClientState::new(
                &game_file_loader,
                graphics_settings.clone(),
                #[cfg(feature = "debug")]
                packet_history,
            ));
        });

        // Generate the direction arrow textures of the minimap marker once at
        // startup. They are pre-rotated so the renderer only has to pick one.
        {
            let marker_textures = generate_direction_markers(&texture_loader);
            client_state
                .follow_mut(crate::client_state().minimap())
                .set_marker_textures(marker_textures);
        }

        let active_interface_settings = client_state.follow(crate::client_state().interface_settings()).clone();

        // The audio engine is created before the client state, so the saved
        // volumes can only be applied once the settings are loaded.
        let audio_settings = client_state.follow(crate::client_state().audio_settings());
        let active_master_volume = audio_settings.master_volume_linear();
        let active_sound_effect_volume = audio_settings.sound_effect_volume_linear();
        let active_background_music_volume = audio_settings.background_music_track_volume();

        audio_engine.set_main_volume(active_master_volume);
        audio_engine.set_sound_effect_volume(active_sound_effect_volume);
        audio_engine.set_spatial_sound_effect_volume(active_sound_effect_volume);
        audio_engine.set_background_music_volume(active_background_music_volume);

        crate::logging::log_debug!(
            "settings: applied saved audio volumes (master={:.2}, sound_effects={:.2}, background_music={:.2}, music_muted={}; effective \
             master={:.2}, sound_effects={:.2}, spatial_sound_effects={:.2}, background_music={:.2})",
            active_master_volume,
            active_sound_effect_volume,
            active_background_music_volume,
            audio_settings.background_music_muted,
            audio_settings.effective_master_volume(),
            audio_settings.effective_sound_effect_volume(),
            audio_settings.effective_spatial_sound_effect_volume(),
            audio_settings.effective_background_music_volume(),
        );

        crate::input::keybinds::load_user_bindings();

        interface.open_window(LoginWindow::new(
            crate::client_state().login_window(),
            crate::client_state().login_settings(),
            crate::client_state().client_info(),
        ));

        // Official status icon textures for the head mark overlay (opt1/opt2).
        let head_marks = HeadMarks::new(&texture_loader);

        Some(Self {
            game_file_loader,
            action_loader,
            #[cfg(feature = "debug")]
            animation_loader,
            async_loader,
            effect_loader,
            font_loader,
            #[cfg(feature = "debug")]
            map_loader,
            sprite_loader,
            texture_loader,
            library,
            interface_renderer,
            bottom_interface_renderer,
            middle_interface_renderer,
            top_interface_renderer,
            effect_renderer,
            #[cfg(feature = "debug")]
            debug_marker_renderer,
            #[cfg(feature = "debug")]
            aabb_instructions,
            #[cfg(feature = "debug")]
            circle_instructions,
            #[cfg(feature = "debug")]
            rectangle_instructions,
            model_batches,
            model_instructions,
            entity_instructions,
            directional_shadow_model_batches,
            directional_shadow_model_instructions,
            directional_shadow_entity_instructions,
            point_shadow_model_batches,
            point_shadow_model_instructions,
            point_shadow_entity_instructions,
            point_light_with_shadow_instructions,
            point_light_instructions,
            input_system,
            interface,
            mouse_cursor,
            show_interface,
            game_timer,
            #[cfg(feature = "debug")]
            debug_camera,
            start_camera,
            player_camera,
            directional_shadow_camera,
            directional_shadow_partitions,
            point_shadow_camera,
            input_event_buffer,
            network_event_buffer,
            saved_login_data,
            saved_character_server,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_packet_version,
            pending_stat_updates: PendingStatUpdates::new(),
            message_table,
            pending_experience: PendingExperience::default(),
            session: SessionMachine::new(),
            entity_lifecycle: EntityLifecycle::new(),
            asset_retries: ResourceRetryQueue::new(ResourcePolicy::ANIMATION_DATA),
            pending_context_menu: None,
            shop_open: false,
            guild_emblems: GuildEmblemCache::default(),
            transient_asset_notices: TransientAssetNoticeLimiter::default(),
            current_map_name: None,
            level_99_aura_style_cell: Arc::new(AtomicU8::new(AuraStyle::Auto.encode())),
            level_99_aura_entities: HashSet::new(),
            skill_sound_gate: SkillSoundGate::default(),
            status_aura_flags: HashMap::new(),
            status_aura_frame: 0,
            status_visual_flags: HashMap::new(),
            status_visual_frame: 0,
            map_window_cycle: MapWindowCycle::Closed,
            minimap_last_tile: None,
            particle_holder,
            projectile_assets: HashMap::new(),
            point_light_manager,
            effect_holder,
            chat_bubbles: ChatBubbles::default(),
            head_marks,
            pending_item_drag: None,
            skill_name_texts: SkillNameTexts::default(),
            cast_bars: CastBars::default(),
            skill_completion_tracker: SkillCompletionTracker::default(),
            aftercast_act_until: None,
            warp_portal,
            weather,
            spirit_sphere_texture,
            teleport_vanish_sound_effect,
            recent_teleport_vanishes: Vec::new(),
            path_finder,
            point_light_set_buffer,
            directional_shadow_object_set_buffer,
            point_shadow_object_set_buffer,
            deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            bounding_box_object_set_buffer,
            #[cfg(feature = "debug")]
            pathing_texture_set,
            #[cfg(feature = "debug")]
            tile_texture_set,
            main_menu_click_sound_effect,
            networking_system,
            audio_engine,
            skill_sounds,
            active_master_volume,
            active_sound_effect_volume,
            active_background_music_volume,
            active_interface_settings,
            active_graphics_settings: graphics_settings,
            graphics_engine,
            queue,
            #[cfg(feature = "debug")]
            device,
            window: None,
            backend_is_wayland: false,
            presentation_paused: false,
            has_presented_frame: false,
            window_occluded: false,
            window_minimized: false,
            force_background,
            active_frames_per_second_limit: 0,
            frames_per_second_log_deadline: frames_per_second_log_deadline(),
            performance_log_deadline: performance_log_deadline(),
            interface_layout_invalidation_pending: false,
            initialize_interface_scaling_from_dpi,
            last_scale_factor: 1.0,

            map: Some(map),
            client_state,
        })
    }

    pub fn run(&mut self) {
        let event_loop = match create_event_loop() {
            Ok(event_loop) => event_loop,
            Err(error) => {
                crate::logging::log_error!("[display] {error}");
                crate::logging::log_error!(
                    "[display] hint: use RO_NEXT_X11=1 to force X11/XWayland or RO_NEXT_WAYLAND=1 to force native Wayland"
                );
                std::process::exit(1);
            }
        };

        event_loop.set_control_flow(ControlFlow::Poll);

        if let Err(error) = event_loop.run_app(self) {
            crate::logging::log_error!("[display] the event loop stopped with an error: {error}");
            std::process::exit(1);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn clear_render_instructions(&mut self) {
        self.interface_renderer.clear();
        self.bottom_interface_renderer.clear();
        self.middle_interface_renderer.clear();
        self.top_interface_renderer.clear();
        self.effect_renderer.clear();
        #[cfg(feature = "debug")]
        self.debug_marker_renderer.clear();

        #[cfg(feature = "debug")]
        self.aabb_instructions.clear();
        #[cfg(feature = "debug")]
        self.circle_instructions.clear();
        #[cfg(feature = "debug")]
        self.rectangle_instructions.clear();
        self.model_batches.clear();
        self.model_instructions.clear();
        self.entity_instructions.clear();
        self.directional_shadow_model_batches.iter_mut().for_each(|batch| batch.clear());
        self.directional_shadow_model_instructions.clear();
        self.directional_shadow_entity_instructions
            .iter_mut()
            .for_each(|instructions| instructions.clear());
        self.point_shadow_model_batches.clear();
        self.point_shadow_model_instructions.clear();
        self.point_shadow_entity_instructions.clear();
        self.point_light_with_shadow_instructions.clear();
        self.point_light_instructions.clear();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_client_state(&mut self) {
        // Unset the highlighted skill just before applying. That way, if the skill is
        // still hovered the value will be the same as before, if not it will clear the
        // highlighted.
        *self.client_state.follow_mut(client_state().skill_tree_window().highlighted_skill()) = None;

        // Apply the game state after all the UI work + rendering is done.
        if let Err(_errors) = self.client_state.apply() {
            #[cfg(feature = "debug")]
            {
                print_debug!("[{}] failed to apply {} updates: ", "error".red(), _errors.len());
                _errors.into_iter().for_each(|error| print_debug!("path: {}", error.type_name));
            }
        }
    }

    /// Apply any graphics or interface setting changes that the user
    /// dispatched during the previous frame.
    ///
    /// May reconfigure the GPU surface (MSAA, SSAA, present mode, shadow
    /// resolution, etc.). Surface reconfiguration is only safe between
    /// presenting the previous frame and acquiring the next swapchain image,
    /// so this *must* be called after [`Self::update_client_state`] (so the
    /// user's dispatched changes are visible) and *before*
    /// `graphics_engine.wait_for_next_frame()`. Calling it after
    /// `wait_for_next_frame` has been observed to cause surface configuration
    /// errors under DX12.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_settings(&mut self) {
        let graphics_settings = self.client_state.follow(client_state().graphics_settings());
        let audio_settings = self.client_state.follow(client_state().audio_settings());

        let master_volume = audio_settings.master_volume_linear();
        let sound_effect_volume = audio_settings.sound_effect_volume_linear();
        let background_music_volume = audio_settings.background_music_track_volume();

        let mut audio_settings_changed = false;

        if (self.active_master_volume - master_volume).abs() > f32::EPSILON {
            self.audio_engine.set_main_volume(master_volume);
            self.active_master_volume = master_volume;
            audio_settings_changed = true;
        }

        if (self.active_sound_effect_volume - sound_effect_volume).abs() > f32::EPSILON {
            self.audio_engine.set_sound_effect_volume(sound_effect_volume);
            self.audio_engine.set_spatial_sound_effect_volume(sound_effect_volume);
            self.active_sound_effect_volume = sound_effect_volume;
            audio_settings_changed = true;
        }

        if (self.active_background_music_volume - background_music_volume).abs() > f32::EPSILON {
            self.audio_engine.set_background_music_volume(background_music_volume);
            self.active_background_music_volume = background_music_volume;
            audio_settings_changed = true;
        }

        if audio_settings_changed {
            // Persist immediately so a volume change survives a crash or a
            // forced exit, not only a clean logout (`Drop` saves as well).
            audio_settings.save();

            crate::logging::log_debug!(
                "settings: audio volumes applied (master={:.2}, sound_effects={:.2}, background_music={:.2}, music_muted={}; effective \
                 master={:.2}, sound_effects={:.2}, spatial_sound_effects={:.2}, background_music={:.2})",
                master_volume,
                sound_effect_volume,
                background_music_volume,
                audio_settings.background_music_muted,
                audio_settings.effective_master_volume(),
                audio_settings.effective_sound_effect_volume(),
                audio_settings.effective_spatial_sound_effect_volume(),
                audio_settings.effective_background_music_volume(),
            );
        }

        if self.active_graphics_settings.vsync != graphics_settings.vsync {
            self.graphics_engine.set_vsync(graphics_settings.vsync);
            self.active_graphics_settings.vsync = graphics_settings.vsync;
        }

        if self.active_graphics_settings.triple_buffering != graphics_settings.triple_buffering {
            self.graphics_engine.set_triple_buffering(graphics_settings.triple_buffering);
            self.active_graphics_settings.triple_buffering = graphics_settings.triple_buffering;
        }

        if self.active_graphics_settings.texture_filtering != graphics_settings.texture_filtering {
            self.graphics_engine.set_texture_sampler_type(graphics_settings.texture_filtering);
            self.active_graphics_settings.texture_filtering = graphics_settings.texture_filtering;
        }

        if self.active_graphics_settings.msaa != graphics_settings.msaa {
            self.graphics_engine.set_msaa(graphics_settings.msaa);
            self.active_graphics_settings.msaa = graphics_settings.msaa;
        }

        if self.active_graphics_settings.ssaa != graphics_settings.ssaa {
            self.graphics_engine.set_ssaa(graphics_settings.ssaa);
            self.active_graphics_settings.ssaa = graphics_settings.ssaa;
        }

        if self.active_graphics_settings.screen_space_anti_aliasing != graphics_settings.screen_space_anti_aliasing {
            self.graphics_engine
                .set_screen_space_anti_aliasing(graphics_settings.screen_space_anti_aliasing);
            self.active_graphics_settings.screen_space_anti_aliasing = graphics_settings.screen_space_anti_aliasing;
        }

        if self.active_graphics_settings.shadow_resolution != graphics_settings.shadow_resolution {
            self.graphics_engine.set_shadow_resolution(graphics_settings.shadow_resolution);
            self.active_graphics_settings.shadow_resolution = graphics_settings.shadow_resolution;
        }

        if self.active_graphics_settings.high_quality_interface != graphics_settings.high_quality_interface {
            self.interface_renderer
                .update_high_quality_interface(graphics_settings.high_quality_interface);
            self.graphics_engine
                .set_high_quality_interface(graphics_settings.high_quality_interface);
            self.active_graphics_settings.high_quality_interface = graphics_settings.high_quality_interface;
        }

        // Apply resolution and fullscreen changes live. The window resize generates a
        // `WindowEvent::Resized`, which is where the graphics surface and the interface
        // are updated, so there is no need to reconfigure anything here.
        let fullscreen_changed = self.active_graphics_settings.fullscreen != graphics_settings.fullscreen;
        let resolution_changed = self.active_graphics_settings.window_resolution != graphics_settings.window_resolution;

        if fullscreen_changed || resolution_changed {
            if let Some(window) = self.window.as_ref() {
                // Leaving fullscreen first, then restoring the window size works around
                // window managers that ignore inner size requests while a window is
                // still marked as fullscreen.
                if fullscreen_changed && !graphics_settings.fullscreen {
                    window.set_fullscreen(None);
                }

                if !graphics_settings.fullscreen {
                    match graphics_settings.window_resolution {
                        WindowResolution::Desktop => window.set_maximized(true),
                        window_resolution => {
                            if let Some((width, height)) = window_resolution.size() {
                                window.set_maximized(false);
                                let _ = window.request_inner_size(PhysicalSize::new(width, height));
                            }
                        }
                    }
                }

                if fullscreen_changed && graphics_settings.fullscreen {
                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
            }

            self.active_graphics_settings.fullscreen = graphics_settings.fullscreen;
            self.active_graphics_settings.window_resolution = graphics_settings.window_resolution;
        }

        let language = *self.client_state.follow(client_state().interface_settings().language());

        if self.active_interface_settings.language != language {
            *self.client_state.follow_mut(client_state().localization()) = Localization::load_language(&self.game_file_loader, language);
            self.active_interface_settings.language = language;
            self.interface.invalidate_layouts();
        }

        let interface_settings = self.client_state.follow_mut(client_state().interface_settings());

        if self.active_interface_settings.menu_theme != interface_settings.menu_theme {
            let menu_theme = interface_settings.menu_theme.clone();
            let theme = InterfaceTheme::load(state::theme::InterfaceThemeType::Menu, &menu_theme);
            *self.client_state.follow_mut(client_state().menu_theme()) = theme;
            self.active_interface_settings.menu_theme = menu_theme;
            self.interface.invalidate_layouts();
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.in_game_theme != interface_settings.in_game_theme {
            let in_game_theme = interface_settings.in_game_theme.clone();
            let theme = InterfaceTheme::load(InterfaceThemeType::InGame, &in_game_theme);
            *self.client_state.follow_mut(client_state().in_game_theme()) = theme;
            self.active_interface_settings.in_game_theme = in_game_theme;
            self.interface.invalidate_layouts();
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.world_theme != interface_settings.world_theme {
            let world_theme = interface_settings.world_theme.clone();
            let theme = WorldTheme::load(&world_theme);
            *self.client_state.follow_mut(client_state().world_theme()) = theme;
            self.active_interface_settings.world_theme = world_theme;
        }

        // The active frame rate limit depends on the persisted settings *and*
        // on whether the window is currently in the background, so it has to be
        // re-evaluated whenever the settings or the window state changed.
        self.apply_frames_per_second_limit();
    }

    #[cfg(feature = "debug")]
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_debug_windows(&mut self, delta_time: f64) {
        let is_packet_inspector_open = self.interface.is_window_with_class_open(WindowClass::PacketInspector);
        self.client_state
            .follow_mut(client_state().packet_history())
            .update(is_packet_inspector_open);

        self.client_state.follow_mut(client_state().cache_statistics()).update(
            delta_time,
            &self.map_loader,
            &self.texture_loader,
            &self.sprite_loader,
            &self.font_loader,
            &self.audio_engine,
            &self.action_loader,
            &self.animation_loader,
            &self.effect_loader,
        );
    }
}
