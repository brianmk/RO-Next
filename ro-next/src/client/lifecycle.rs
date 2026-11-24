//! The client lifecycle: construction (`init`), the winit application handler,
//! the per-frame update/render driver and the loader bookkeeping.
//!
//! `init`/`run` are the only public entry points.

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
            warp_skill_units: Vec::new(),
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

    /// Drain finished async loads from the loader thread and apply their
    /// results to client state. This is the only place that promotes
    /// `self.map` from `None` to `Some` (when a map load completes), so it
    /// must run before the `self.map` check in [`Self::update_and_render`].
    ///
    /// Called as late as possible in the frame to give the loader thread the
    /// maximum window to finish in-flight work.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_loaded_resources(&mut self, client_tick: ClientTick) {
        // Collect first so the borrowed loader can be released before the
        // (mutable) handling below.
        let completed_loads: Vec<LoadResult> = self.async_loader.take_completed().collect();

        for completed in completed_loads {
            let (id, resource) = match completed {
                LoadResult::Completed { id, resource } => (id, resource),
                LoadResult::Failed { id, error } => {
                    self.handle_failed_load(id, error);
                    continue;
                }
            };

            if matches!(
                id,
                LoaderId::AnimationData(_)
                    | LoaderId::FalconAnimationData(_)
                    | LoaderId::WeaponAnimationData(_)
                    | LoaderId::ShieldAnimationData(_)
                    | LoaderId::RobeAnimationData(_)
            ) {
                self.asset_retries.note_available(&id);
            }

            match (id, resource) {
                (LoaderId::AnimationData(entity_id), LoadableResource::AnimationData(animation_data)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().dead_entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.set_animation_data(animation_data);
                    }
                }
                (LoaderId::CharacterPreview(slot), LoadableResource::AnimationData(animation_data)) => {
                    self.client_state
                        .follow_mut(client_state().character_previews())
                        .set_ready(slot, animation_data);
                }
                (LoaderId::FalconAnimationData(entity_id), LoadableResource::FalconAnimation(falcon_animation)) => {
                    crate::logging::log_debug!("falcon animation: applying to entity {}", entity_id.0);
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }
                }
                (LoaderId::ImfData(entity_id, ..), LoadableResource::Imf(imf)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_imf(imf);
                    }
                }
                (LoaderId::CartAnimationData(entity_id), LoadableResource::CartAnimation(cart_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_cart_animation(cart_animation);
                    }
                }
                (LoaderId::WeaponAnimationData(entity_id), LoadableResource::WeaponAnimation(weapon_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_weapon_animation(weapon_animation);
                    }
                }
                (LoaderId::ShieldAnimationData(entity_id), LoadableResource::ShieldAnimation(shield_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_shield_animation(shield_animation);
                    }
                }
                (LoaderId::RobeAnimationData(entity_id), LoadableResource::RobeAnimation(robe_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_robe_animation(robe_animation);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture }) => {
                    self.client_state
                        .follow_mut(client_state().shop_items())
                        .iter_mut()
                        .filter(|item| item.item_id == item_id)
                        .for_each(|item| item.metadata.texture = Some(texture.clone()));

                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_item_sprite(item_id, texture.clone());

                    // Kafra storage items are loaded from a separate inventory
                    // state; without this update their icons stay blank when
                    // the item sprite finished loading while the storage window
                    // is open (the inventory may not contain the same item id).
                    self.client_state
                        .follow_mut(client_state().storage())
                        .update_item_sprite(item_id, texture);
                }
                (LoaderId::Map(map_name), LoadableResource::Map { map, position }) => {
                    match self.client_state.try_follow(this_entity()).is_none() {
                        true => {
                            // Load of main menu map
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine
                                .play_background_music_track(crate::client_config::background_music());
                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());

                            // The main menu map finished loading. Do not blindly open the
                            // character selection window here: the map load can finish after the
                            // player already logged in, which used to replace the freshly opened
                            // server selection window (or the login window) with an empty
                            // character selection screen. Show the window that matches the
                            // current login state instead.
                            let character_server_connected = self.networking_system.is_character_server_connected();
                            let login_server_connected = self.networking_system.is_login_server_connected();
                            let character_selection_open = self.interface.is_window_with_class_open(WindowClass::CharacterSelection);
                            let select_server_open = self.interface.is_window_with_class_open(WindowClass::SelectServer);
                            let login_open = self.interface.is_window_with_class_open(WindowClass::Login);

                            if character_server_connected && !character_selection_open {
                                self.open_character_selection_window();
                            } else if login_server_connected && !select_server_open && !character_selection_open {
                                self.interface
                                    .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                            } else if !character_server_connected
                                && !login_server_connected
                                && !login_open
                                && !select_server_open
                                && !character_selection_open
                            {
                                self.interface.open_window(LoginWindow::new(
                                    client_state().login_window(),
                                    client_state().login_settings(),
                                    client_state().client_info(),
                                ));
                            }

                            // The map entry may have been aborted because the map data
                            // was missing; tell the player why, now that the character
                            // selection window is back (opening the error last keeps it
                            // on top).
                            if let Some(message) = self.session.take_map_load_error() {
                                self.interface.open_window(ErrorWindow::new(message));
                            }

                            self.start_camera.set_focus_point(crate::client_config::start_camera_focus());
                        }
                        false => {
                            // Normal map switch: the map entry is complete.
                            self.session.map_assets_loaded();
                            self.current_map_name = Some(map_name.clone());
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(map.background_music_track_name());

                            if let Some(minimap) = map.get_minimap() {
                                self.client_state.follow_mut(client_state().minimap()).set_map(&map_name, minimap);
                            } else {
                                self.client_state.follow_mut(client_state().minimap()).clear();
                            }

                            if let Some(position) = position {
                                // `manually_asserted` is safe because we are in the branch where `this_player`
                                // is not `None`.
                                let player = self.client_state.follow_mut(this_entity().manually_asserted());

                                player.set_position(map, position, client_tick);
                                self.player_camera.set_focus_point(player.get_position());
                            }

                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                            self.on_navigation_map_loaded(&map_name);
                            let _ = self.networking_system.map_loaded();
                        }
                    }
                }
                (LoaderId::SkillSprite(skill_id), LoadableResource::SkillSprite { sprite }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar())
                        .update_skill_resource(skill_id, |skill| skill.sprite = Some(sprite.clone()));

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.sprite = Some(sprite);
                    }
                }
                (LoaderId::SkillActions(skill_id), LoadableResource::SkillActions { actions }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar())
                        .update_skill_resource(skill_id, |skill| skill.actions = Some(actions.clone()));

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.actions = Some(actions);
                    }
                }
                _ => {}
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    /// Reacts to an asynchronous load that failed.
    ///
    /// A failed map load is a dead end for the current map entry: the client
    /// has no map to render and would stay on a black screen forever. In
    /// that case the map connection is dropped, which sends the player back
    /// to the character selection screen (see
    /// `NetworkEvent::MapServerDisconnected`), and the error is shown
    /// there.
    /// Keep the character selection previews in sync with the character
    /// slots.
    ///
    /// Called every frame while the window is open. For every visible occupied
    /// slot whose preview is not loaded yet, the composed idle animation of the
    /// character is requested from the async loader; the result is applied by
    /// [`Self::update_loaded_resources`]. When the window is not open the
    /// loaded previews are dropped, so returning to the login screen does not
    /// keep character sprites alive.
    /// Repaint the character selection window while the anti-crash selection
    /// cooldown is running, so the countdown text updates and the cards become
    /// selectable again once the cooldown expired (the click handlers are only
    /// registered during a layout pass).
    pub(crate) fn update_character_selection_cooldown(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            return;
        }

        if self
            .client_state
            .follow(client_state().character_selection_cooldown())
            .is_repainting(Instant::now())
        {
            self.interface.invalidate_layouts();
        }
    }

    pub(crate) fn update_character_selection_previews(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            let previews = self.client_state.follow_mut(client_state().character_previews());
            if previews.slot_count() > 0 {
                previews.clear();
            }
            return;
        }

        let reported_slots = self.client_state.follow(client_state().character_slots()).get_slot_count();
        let slot_count = visible_character_slots(reported_slots);
        let previous_slot_count = self.client_state.follow(client_state().character_previews()).slot_count();

        self.client_state
            .follow_mut(client_state().character_previews())
            .set_slot_count(slot_count);

        // Log the clamp once per character list, not every frame the window is
        // open.
        if reported_slots > MAX_CHARACTER_SLOTS && previous_slot_count != slot_count {
            crate::logging::log_debug!(
                "character selection: server reports {reported_slots} slots, showing the first {MAX_CHARACTER_SLOTS}"
            );
        }

        for slot in 0..slot_count {
            // The character list can change while the window is open (delete,
            // slot switch); clone the row so the state borrow ends before the
            // loader is called.
            let character_information = self
                .client_state
                .follow(client_state().character_slots())
                .character_in_slot(slot)
                .cloned();

            let Some(character_information) = character_information else {
                self.client_state.follow_mut(client_state().character_previews()).clear_slot(slot);
                continue;
            };

            let character_id = character_information.character_id;
            let is_current = self
                .client_state
                .follow(client_state().character_previews())
                .is_current(slot, character_id);

            if is_current {
                continue;
            }

            let entity_part_files = character_preview_part_files(&self.library, &character_information);

            self.client_state
                .follow_mut(client_state().character_previews())
                .request(slot, character_id);

            // The appearance may already be cached (for example from a previous
            // visit of the character selection screen): in that case no
            // asynchronous load happens and the data can be applied right away.
            if let Some(animation_data) = self.async_loader.request_character_preview_load(slot, entity_part_files) {
                self.client_state
                    .follow_mut(client_state().character_previews())
                    .set_ready(slot, animation_data);
            }
        }
    }

    pub(crate) fn handle_failed_load(&mut self, id: LoaderId, error: LoadError) {
        // A missing preview sprite must not be requested again every frame: the
        // slot keeps its name/level and simply shows no character.
        if let LoaderId::CharacterPreview(slot) = &id {
            crate::logging::log_debug!("character preview unavailable for slot {slot}: {error:?}");
            self.client_state
                .follow_mut(client_state().character_previews())
                .set_unavailable(*slot);
            return;
        }

        // Missing/placeholder weapon sprites are expected in this data set (the
        // CondaRO archives have no drawable per-job weapon clips; see
        // `world::weapon`); the async loader caches the failure and the entity is
        // rendered without the weapon layer. Keep this out of the generic asset
        // failure reporting (and out of stderr) so map entry does not spam the
        // user or the log for every population-engine shell.
        if let LoaderId::WeaponAnimationData(entity_id)
        | LoaderId::ShieldAnimationData(entity_id)
        | LoaderId::RobeAnimationData(entity_id) = &id
        {
            crate::logging::log_debug!("equipment animation unavailable for entity {}: {error:?}", entity_id.0);
            return;
        }

        match asset_failure_severity(&id) {
            AssetFailureSeverity::Fatal => {
                let LoaderId::Map(map_name) = id else {
                    // Severity and variant classification disagree; treat it as
                    // a transient failure below instead of panicking.
                    self.report_transient_asset_failure(id, error);
                    return;
                };

                // Only report the first failure of a map entry; retries after the
                // client returned to the character selection must not spam.
                if !self.session.has_map_load_error() {
                    crate::logging::log_warn!("map load failed: {map_name}: {error:?}");
                }

                let message = self.render_localized(client_state().localization().error_map_data_missing(), &[(
                    "map_name",
                    map_name.as_str(),
                )]);

                self.abort_map_entry(message);
            }
            AssetFailureSeverity::Transient => {
                if matches!(
                    id,
                    LoaderId::AnimationData(_)
                        | LoaderId::FalconAnimationData(_)
                        | LoaderId::WeaponAnimationData(_)
                        | LoaderId::ShieldAnimationData(_)
                        | LoaderId::RobeAnimationData(_)
                ) {
                    self.asset_retries.note_failed(&id, Instant::now(), &format!("{error:?}"));
                }

                self.report_transient_asset_failure(id, error);
            }
        }
    }

    /// Report a recoverable asset failure: always at warn level (visible in
    /// release) and, at most a few times per session, as a non-modal chat line.
    pub(crate) fn report_transient_asset_failure(&mut self, id: LoaderId, error: LoadError) {
        let key = asset_failure_log_key(&id);
        crate::logging::warn_once(
            module_path!(),
            &key,
            format_args!("resource load failed for {key}: {error:?} (a placeholder is used instead)"),
        );

        if self.transient_asset_notices.should_notify() {
            let message = transient_asset_failure_message(&id, &error);
            self.push_chat_warning(message);
        }
    }
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // To be as portable as possible, winit recommends to initialize the window and
        // graphics backend after the first resume event is received.
        if self.window.is_none() {
            time_phase!("create window", {
                let reader = ImageReader::with_format(Cursor::new(ICON_DATA), ImageFormat::Png);
                let image_buffer = reader.decode().unwrap().to_rgba8();
                let image_data = image_buffer.as_bytes().to_vec();

                assert_eq!(image_buffer.width(), image_buffer.height(), "icon must be square");
                let icon = Icon::from_rgba(image_data, image_buffer.width(), image_buffer.height()).unwrap();

                // Create the window with the resolution that was loaded from the settings
                // file (or overridden through the environment) so that it does not have to
                // be resized right after it became visible.
                let (window_resolution, fullscreen) = {
                    let graphics_settings = self.client_state.follow(client_state().graphics_settings());
                    (graphics_settings.window_resolution, graphics_settings.fullscreen)
                };

                let initial_size = match window_resolution.size() {
                    Some((width, height)) => Size::Physical(PhysicalSize::new(width, height)),
                    None => Size::Logical(LogicalSize {
                        width: f64::from(INITIAL_SCREEN_SIZE.width),
                        height: f64::from(INITIAL_SCREEN_SIZE.height),
                    }),
                };

                let window_attributes = Window::default_attributes()
                    .with_inner_size(initial_size)
                    .with_maximized(!fullscreen && window_resolution == WindowResolution::Desktop)
                    .with_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)))
                    .with_title(CLIENT_NAME)
                    .with_window_icon(Some(icon))
                    .with_visible(false);
                let window = match event_loop.create_window(window_attributes) {
                    Ok(window) => Arc::new(window),
                    Err(error) => {
                        crate::logging::log_error!("[display] failed to create the client window: {error}");
                        crate::logging::log_error!(
                            "[display] hint: use RO_NEXT_X11=1 to force X11/XWayland or RO_NEXT_WAYLAND=1 to force native Wayland"
                        );
                        std::process::exit(1);
                    }
                };

                let backend_name = self.graphics_engine.get_backend_name();
                window.set_title(&format!("{CLIENT_NAME} ({})", str::to_uppercase(&backend_name)));
                window.set_cursor_visible(false);
                self.last_scale_factor = window.scale_factor();
                self.backend_is_wayland = window_is_wayland(&window);

                self.window = Some(window);

                if let Some(window) = self.window.as_ref() {
                    self.log_display_information(window);
                }

                #[cfg(feature = "debug")]
                print_debug!("created {}", "window".magenta());
            });
        }

        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if let Some(window) = self.window.as_ref() {
            let path = client_state().graphics_settings();
            let graphics_settings = self.client_state.follow(path);

            self.graphics_engine.on_resume(
                window.clone(),
                graphics_settings.triple_buffering,
                graphics_settings.vsync,
                graphics_settings.shadow_resolution,
                graphics_settings.texture_filtering,
                graphics_settings.msaa,
                graphics_settings.ssaa,
                graphics_settings.screen_space_anti_aliasing,
                graphics_settings.high_quality_interface,
            );

            // Update graphics settings capabilities based on the new surface.
            // We don't expect the capabilities to change on consecutive calls but we
            // can't get the present mode info when initializing the client, so
            // we do it here instead.
            self.client_state
                .follow_mut(client_state().graphics_settings_capabilities())
                .update(
                    self.graphics_engine.get_supported_msaa(),
                    self.graphics_engine.get_present_mode_info(),
                );

            window.set_visible(true);
        }

        // The window was created with the configured resolution, which can differ
        // from the initial screen size. Make sure the interface and the renderers
        // use the real window size from the first frame on.
        if let Some(inner_size) = self.window.as_ref().map(|window| window.inner_size()) {
            self.update_window_size(inner_size.max(PhysicalSize::new(1, 1)).into());
        }

        // On the first start there are no saved interface settings, so pick a
        // sensible interface scaling based on the DPI scale factor of the window.
        // The `RO_NEXT_UI_SCALE` environment variable always has precedence.
        if self.initialize_interface_scaling_from_dpi {
            self.initialize_interface_scaling_from_dpi = false;

            if std::env::var("RO_NEXT_UI_SCALE").is_err()
                && let Some(window) = self.window.as_ref()
            {
                let scale_factor = window.scale_factor() as f32;

                if (scale_factor - 1.0).abs() > f32::EPSILON {
                    let scaling = Scaling::closest(scale_factor.clamp(0.5, 2.0));
                    *self.client_state.follow_mut(client_state().interface_settings().scaling()) = scaling;
                }
            }
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            crate::logging::log_debug!("settings: audio focus mute disabled (window focused)");
            self.audio_engine.mute(false);
        }

        self.refresh_window_minimized();
        self.apply_frames_per_second_limit();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // Request a clean logout instead of exiting right away, so the server
                // does not keep the character/account online (see `update_and_render`).
                SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
            }
            WindowEvent::Resized(screen_size) => {
                self.update_window_size(screen_size.max(PhysicalSize::new(1, 1)).into());

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let ratio = if self.last_scale_factor > 0.0 {
                    scale_factor / self.last_scale_factor
                } else {
                    1.0
                };

                // The compositor reports cursor positions in the new coordinate space from now
                // on. Convert the last known position as well, otherwise a click that arrives
                // before the next pointer motion event would use stale coordinates.
                self.input_system.rescale_mouse_position(ratio as f32);
                self.last_scale_factor = scale_factor;

                crate::logging::log_info!("[display] window scale factor changed to {scale_factor:.2} (pointer rescaled by {ratio:.3})");
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.input_system.reset();
                    self.input_system.clear_mouse_buttons();
                }

                if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
                    crate::logging::log_debug!("settings: audio focus mute (muted={})", !focused);
                    self.audio_engine.mute(!focused);
                }

                self.refresh_window_minimized();
                self.apply_frames_per_second_limit();

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.window_occluded = occluded;
                self.refresh_window_minimized();
                self.apply_frames_per_second_limit();

                // Windows that are visible again need a redraw to restart the
                // rendering loop.
                if !occluded && let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                // On Wayland the pointer can leave the surface while a button is held, in which
                // case the release event may never be delivered. A button that is stuck in the
                // pressed state turns every following click into a drag and makes the interface
                // ignore clicks, so drop the button state here.
                self.input_system.clear_mouse_buttons();
                crate::logging::log_trace!("[input] pointer left the window");

                self.mouse_cursor.hide();
            }
            WindowEvent::CursorEntered { .. } => {
                crate::logging::log_trace!("[input] pointer entered the window");
                self.mouse_cursor.show();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if crate::logging::enabled(crate::logging::LEVEL_TRACE)
                    && let Some(window) = self.window.as_ref()
                {
                    let inner_size = window.inner_size();
                    crate::logging::log_trace!(
                        "[input] cursor moved raw=({:.2}, {:.2}) inner_size={}x{} scale_factor={:.2}",
                        position.x,
                        position.y,
                        inner_size.width,
                        inner_size.height,
                        window.scale_factor(),
                    );
                }

                self.input_system.update_mouse_position(position);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                crate::logging::log_debug!("[input] mouse button {button:?} {state:?}");
                self.input_system.update_mouse_buttons(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => self.input_system.update_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => {
                crate::logging::log_debug!("[input] keyboard {:?} {:?}", event.physical_key, event.state);

                if let PhysicalKey::Code(keycode) = event.physical_key {
                    self.input_system.update_keyboard(keycode, event.state);
                }

                // TODO: NHA We should also support IME in the long term (winit::event::Ime)
                if let Some(text) = event.text
                    && event.state.is_pressed()
                {
                    for char in text.chars() {
                        self.input_system.buffer_character(char);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(feature = "debug")]
                let _measurement = threads::Main::start_frame();

                self.update_and_render(event_loop);

                if let Some(window) = self.window.as_mut() {
                    window.request_redraw();
                }
            }
            _ignored => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.graphics_engine.on_suspended();

        if let Some(window) = self.window.as_ref() {
            window.set_visible(false);
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            crate::logging::log_debug!("settings: audio focus mute (window suspended)");
            self.audio_engine.mute(true);
        }

        self.window_minimized = false;
        self.apply_frames_per_second_limit();
    }
}

impl Client {
    pub(crate) fn update_main_camera(
        &mut self,
        window_size: ScreenSize,
        delta_time: f64,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        if self.client_state.try_follow(this_entity()).is_some() {
            self.player_camera.update(delta_time);
            self.player_camera.generate_view_projection(window_size);
        } else {
            self.start_camera.update(delta_time);
            self.start_camera.generate_view_projection(window_size);
        }

        #[cfg(feature = "debug")]
        self.interface_renderer.update_render_options(render_options);

        #[cfg(feature = "debug")]
        if render_options.use_debug_camera {
            self.debug_camera.generate_view_projection(window_size);
        }
    }

    /// Re-register the local player in the entity lifecycle after a warp.
    ///
    /// A warp clears all entity bookkeeping (only the player survives the map
    /// change), so the player has to be tracked again explicitly.
    pub(crate) fn retrack_local_player(&mut self) {
        let Some(player) = self.client_state.try_follow(this_entity()) else {
            return;
        };

        let entity_id = player.get_entity_id();
        let job_id = player.get_job_id();
        let _ = self.entity_lifecycle.on_add_entity(entity_id);
        self.entity_lifecycle.on_spawned(entity_id, job_id);
    }

    /// Find a visible entity by id.
    pub(crate) fn find_entity_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
    }

    /// Track visual resources that are still missing and retry loads whose
    /// deadline expired.
    ///
    /// This is a reconciliation loop: the [`AsyncLoader`] already deduplicates
    /// in-flight requests, so an entity whose animation data did not arrive is
    /// simply re-requested after the [`ResourcePolicy::ANIMATION_DATA`] timeout
    /// (bounded by its attempt budget). A load that failed or was lost can
    /// therefore no longer leave an entity without a sprite forever, and a
    /// resource that really is missing gives up with a warning instead of
    /// retrying silently every frame.
    pub(crate) fn update_asset_load_retries(&mut self, now: Instant) {
        if self.map.is_none() {
            return;
        }

        let mut missing: Vec<LoaderId> = Vec::new();
        for entity in self.client_state.follow(client_state().entities()).iter() {
            let entity_id = entity.get_entity_id();

            if entity.needs_animation_data() {
                missing.push(LoaderId::AnimationData(entity_id));
            }
            if entity.needs_falcon_animation() {
                missing.push(LoaderId::FalconAnimationData(entity_id));
            }
            if entity.needs_cart_animation() {
                missing.push(LoaderId::CartAnimationData(entity_id));
            }
            if entity.needs_imf() {
                missing.push(LoaderId::ImfData(
                    entity_id,
                    entity.get_job_id(),
                    matches!(entity.get_sex(), ragnarok_packets::Sex::Female),
                ));
            }
            if entity.needs_weapon_animation() {
                missing.push(LoaderId::WeaponAnimationData(entity_id));
            }
            if entity.needs_shield_animation() {
                missing.push(LoaderId::ShieldAnimationData(entity_id));
            }
            if entity.needs_robe_animation() {
                missing.push(LoaderId::RobeAnimationData(entity_id));
            }
        }

        for id in missing {
            if !self.asset_retries.is_tracked(&id) {
                self.asset_retries.note_request(id, now);
            }
        }

        for id in self.asset_retries.keys_due(now) {
            self.retry_visual_load(id, now);
        }
    }

    /// Re-request one visual resource (`animation data`, falcon or weapon
    /// layer).
    pub(crate) fn retry_visual_load(&mut self, id: LoaderId, now: Instant) {
        let entity_id = match id {
            LoaderId::AnimationData(entity_id)
            | LoaderId::FalconAnimationData(entity_id)
            | LoaderId::CartAnimationData(entity_id)
            | LoaderId::WeaponAnimationData(entity_id)
            | LoaderId::ShieldAnimationData(entity_id)
            | LoaderId::RobeAnimationData(entity_id) => entity_id,
            _ => {
                self.asset_retries.forget(&id);
                return;
            }
        };

        let entity_info = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| {
                (
                    entity.get_entity_type(),
                    entity.get_entity_part_files(&self.library),
                    entity.has_falcon(),
                    entity.has_cart(),
                    entity.get_weapon_part_candidates(),
                    entity.get_shield_part_candidates(),
                    entity.get_robe_part_candidates(),
                    entity.get_cart_part_candidates(),
                    entity.has_weapon_animation(),
                    entity.has_shield_animation(),
                    entity.has_robe_animation(),
                    entity.has_cart_animation(),
                )
            });

        let Some((
            entity_type,
            entity_part_files,
            has_falcon,
            has_cart,
            weapon_candidates,
            shield_candidates,
            robe_candidates,
            cart_candidates,
            has_weapon_animation,
            has_shield_animation,
            has_robe_animation,
            has_cart_animation,
        )) = entity_info
        else {
            self.asset_retries.forget(&id);
            return;
        };

        match id {
            LoaderId::AnimationData(_) => match self
                .async_loader
                .request_animation_data_load(entity_id, entity_type, entity_part_files)
            {
                Some(animation_data) => {
                    if let Some(entity) = self.find_entity_mut(entity_id) {
                        entity.set_animation_data(animation_data);
                    }
                    self.asset_retries.note_available(&LoaderId::AnimationData(entity_id));
                }
                None => self.asset_retries.note_request(LoaderId::AnimationData(entity_id), now),
            },
            LoaderId::FalconAnimationData(_) => {
                if !has_falcon {
                    self.asset_retries.note_available(&LoaderId::FalconAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_falcon_animation_data_load(entity_id) {
                    Some(falcon_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_falcon_animation(falcon_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::FalconAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::FalconAnimationData(entity_id), now),
                }
            }
            LoaderId::CartAnimationData(_) => {
                if !has_cart || has_cart_animation {
                    self.asset_retries.note_available(&LoaderId::CartAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_cart_animation_data_load(entity_id, cart_candidates) {
                    Some(cart_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_cart_animation(cart_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::CartAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::CartAnimationData(entity_id), now),
                }
            }
            LoaderId::ImfData(_, job_id, female) => {
                let sex = if female {
                    ragnarok_packets::Sex::Female
                } else {
                    ragnarok_packets::Sex::Male
                };
                match self.async_loader.request_imf_data_load(entity_id, job_id, sex) {
                    Some(imf) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_imf(imf);
                        }
                        self.asset_retries.note_available(&LoaderId::ImfData(entity_id, job_id, female));
                    }
                    None => self.asset_retries.note_request(LoaderId::ImfData(entity_id, job_id, female), now),
                }
            }
            LoaderId::WeaponAnimationData(_) => {
                if weapon_candidates.is_empty() || has_weapon_animation {
                    self.asset_retries.note_available(&LoaderId::WeaponAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_weapon_animation_data_load(entity_id, weapon_candidates) {
                    Some(weapon_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_weapon_animation(weapon_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::WeaponAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::WeaponAnimationData(entity_id), now),
                }
            }
            LoaderId::ShieldAnimationData(_) => {
                if shield_candidates.is_empty() || has_shield_animation {
                    self.asset_retries.note_available(&LoaderId::ShieldAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_shield_animation_data_load(entity_id, shield_candidates) {
                    Some(shield_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_shield_animation(shield_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::ShieldAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::ShieldAnimationData(entity_id), now),
                }
            }
            LoaderId::RobeAnimationData(_) => {
                if robe_candidates.is_empty() || has_robe_animation {
                    self.asset_retries.note_available(&LoaderId::RobeAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_robe_animation_data_load(entity_id, robe_candidates) {
                    Some(robe_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_robe_animation(robe_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::RobeAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::RobeAnimationData(entity_id), now),
                }
            }
            _ => self.asset_retries.forget(&id),
        }
    }

    /// Per-frame tick for all entities, dead entities, and ground items.
    ///
    /// Must be called after [`Self::handle_network_events`] so the entity
    /// set reflects the latest spawn/despawn/move packets. Running this
    /// with a stale entity list would tick entities that the network has
    /// already removed, or miss new ones that just appeared (and on a map
    /// transition, would tick entities from the previous map).
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_entities(
        &mut self,
        map: &Map,
        currently_playing: bool,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        self.client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .for_each(|entity| entity.update(&self.audio_engine, map, current_camera, client_tick));

        self.client_state
            .follow_mut(client_state().dead_entities())
            .iter_mut()
            .for_each(|entity| {
                entity.update(&self.audio_engine, map, current_camera, client_tick);

                if entity.is_death_animation_over() && !entity.is_fading() {
                    entity.fade_out(DisappearanceReason::Died, client_tick);
                }
            });

        self.client_state
            .follow_mut(client_state().ground_items())
            .iter_mut()
            .for_each(|item| item.update(client_tick));

        // Remove entities that have finished fading out and drop their
        // lifecycle/retry bookkeeping as well.
        let mut disappeared_entities = Vec::new();
        self.client_state.follow_mut(client_state().entities()).retain(|entity| {
            let keep = !entity.should_be_removed(client_tick);
            if !keep {
                disappeared_entities.push(entity.get_entity_id());
            }
            keep
        });

        let mut disappeared_dead_entities = Vec::new();
        self.client_state.follow_mut(client_state().dead_entities()).retain(|entity| {
            let keep = !entity.should_be_removed(client_tick);
            if !keep {
                disappeared_dead_entities.push(entity.get_entity_id());
            }
            keep
        });

        for entity_id in disappeared_entities.into_iter().chain(disappeared_dead_entities) {
            self.entity_lifecycle.on_entity_disappeared(entity_id);
            self.asset_retries.forget(&LoaderId::AnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::FalconAnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::WeaponAnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::ShieldAnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::RobeAnimationData(entity_id));
        }

        self.client_state
            .follow_mut(client_state().ground_items())
            .retain(|item| !item.should_be_removed(client_tick));
    }

    /// Whether the local player may attack `entity_id` under the current
    /// map's targeting rules.
    ///
    /// Monsters are attackable on every map (the official client's
    /// `IsMonsterClass` branch of `CGameMode::HandleActorPick`); players are
    /// subject to [`player_attack_decision`]. Entity types that are neither
    /// (NPCs, warps, pets, homunculi, mercenaries) are not attack targets of
    /// the local player's basic attack.
    pub(crate) fn can_attack_entity(&self, entity_id: EntityId) -> bool {
        let Some((entity_type, is_dead)) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| (entity.get_entity_type(), entity.is_dead()))
        else {
            return false;
        };

        match entity_type {
            EntityType::Monster => true,
            EntityType::Player => !is_dead && self.can_attack_player(entity_id),
            _ => false,
        }
    }

    /// Whether the local player may attack the player `entity_id`.
    ///
    /// This is the client-side half of the official
    /// `CGameMode::HandleActorPick`/`ProcessActorPick` player branch: the
    /// map's PvP/GvG/PK zone, the target's party/guild relation and the
    /// target's dead state decide whether an attack may be dispatched.
    pub(crate) fn can_attack_player(&self, entity_id: EntityId) -> bool {
        let Some((target_character_id, target_guild_id)) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| (entity.get_character_id(), entity.get_guild_id()))
        else {
            return false;
        };

        let Some(local) = self.client_state.try_follow(this_entity()) else {
            return false;
        };

        let local_entity_id = local.get_entity_id();
        let local_guild_id = local.get_guild_id();

        let zone = player_combat_zone(
            *self.client_state.follow(client_state().map_type()),
            *self.client_state.follow(client_state().map_flags()),
        );

        let is_party_member = self
            .client_state
            .follow(client_state().party())
            .is_character_member(target_character_id);

        // `GuildInfo::HasRelatedGuild` covers the own guild and allied
        // guilds; the own guild id from the local entity is used as a
        // fallback when the guild window data has not arrived yet.
        let guild = self.client_state.follow(client_state().guild());
        let is_related_guild = target_guild_id != 0 && (target_guild_id == local_guild_id || guild.is_related_guild(target_guild_id));

        player_attack_decision(zone, TargetRelations {
            is_self: entity_id == local_entity_id,
            is_party_member,
            is_related_guild,
            is_dead: false,
        })
        .is_allowed()
    }

    /// Fire any action that the player buffered while out of range or while
    /// still moving (attack, pick up item). Must be called after
    /// [`Self::update_entities`] so that the player's `stopped_moving` state
    /// reflects this frame.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn process_buffered_action(&mut self) {
        let Some(true) = self.client_state.try_follow(this_entity()).map(|player| player.stopped_moving()) else {
            return;
        };

        let Some(buffered_action) = self.client_state.follow_mut(client_state().buffered_action()).take() else {
            return;
        };

        match buffered_action {
            BufferedAction::AttackEntity { entity_id } => {
                // Re-check the targeting rule here: the buffered attack can
                // outlive a map change or the target can die while the player
                // walks into range.
                if !self.can_attack_entity(entity_id) {
                    return;
                }

                let _ = self.networking_system.player_attack(entity_id);

                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                if auto_attack {
                    *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity { entity_id });
                }
            }
            BufferedAction::PickUpItem { entity_id } => {
                if self
                    .client_state
                    .follow(client_state().ground_items())
                    .iter()
                    .any(|item| item.get_entity_id() == entity_id)
                {
                    let _ = self.networking_system.pick_up_item(entity_id);
                }
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_audio_engine(&self, current_camera: &dyn Camera) {
        // We set the listener roughly at ear height.
        const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
        let listener = current_camera.focus_point() + EAR_HEIGHT;

        self.audio_engine
            .set_spatial_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
        self.audio_engine.update();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn create_point_light_set<'a>(
        point_light_manager: &'a mut PointLightManager,
        point_light_set_buffer: &mut ResourceSetBuffer<LightSourceKey>,
        map: &Map,
        effect_holder: &EffectHolder,
        current_camera: &dyn Camera,
        lighting_mode: LightingMode,
    ) -> PointLightSet<'a> {
        point_light_manager.prepare();

        effect_holder.register_point_lights(point_light_manager, current_camera);

        map.register_point_lights(point_light_manager, point_light_set_buffer, current_camera);

        match lighting_mode {
            LightingMode::Classic => point_light_manager.create_point_light_set(0),
            LightingMode::Enhanced => point_light_manager.create_point_light_set(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS),
        }
    }

    /// Applies any buffered drag from the previous frame's input and draws the
    /// per-frame top-level overlays (FPS counter when in debug, mouse cursor).
    /// Must be called after the laid-out interface frame has been dropped so
    /// that `self.interface` is no longer borrowed.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn render_ui_overlays(
        &mut self,
        input_report: &InputReport,
        scaling: Scaling,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        if let Some(delta) = input_report.drag {
            // TODO: The scaling should be removed here.
            self.interface.handle_drag(delta, scaling.get_factor());
        }

        #[cfg(feature = "debug")]
        if render_options.show_frames_per_second {
            let world_theme = self.client_state.follow(client_state().world_theme());

            self.top_interface_renderer.render_text(
                &self.game_timer.last_frames_per_second().to_string(),
                world_theme.overlay.text_offset,
                world_theme.overlay.foreground_color,
                world_theme.overlay.font_size,
                AlignHorizontal::Left,
            );
        }

        if self.show_interface {
            self.mouse_cursor.render(
                &self.top_interface_renderer,
                input_report.mouse_position,
                self.interface.get_mouse_mode().grabbed(),
                *self.client_state.follow(client_state().world_theme().cursor().color()),
                self.client_state.follow(client_state().interface_settings().scaling()).get_factor(),
            );
        }
    }

    #[inline(always)]
    pub(crate) fn update_and_render(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        crate::perf::frame_begin();

        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            // The session machine owns the shutdown sequence and its deadline.
            self.session.request_shutdown();
        }

        if self.session.state() == SessionState::Shutdown {
            // Ask the map server for a clean disconnect (CZ_REQ_DISCONNECT) instead of
            // just dropping the TCP connection. The map server then saves the character
            // and reports the account offline to the character/login server, so an
            // immediate relogin is not rejected with "already online".
            if self.session.shutdown_deadline().is_none() {
                if self.networking_system.is_map_server_connected() {
                    let _ = self.networking_system.request_disconnect();
                    self.session
                        .set_shutdown_deadline(Some(Instant::now() + SHUTDOWN_DISCONNECT_TIMEOUT));
                } else if self.networking_system.is_character_server_connected() {
                    self.networking_system.disconnect_from_character_server();
                } else if self.networking_system.is_login_server_connected() {
                    self.networking_system.disconnect_from_login_server();
                } else {
                    self.session.finish_shutdown();
                    event_loop.exit();
                    return;
                }
            }

            // Exit once the map server disconnected us (clean logout done) or the
            // deadline expired. Close the remaining sockets in server order first so the
            // login server can clear its online state before the process exits.
            if !self.networking_system.is_map_server_connected()
                || self.session.shutdown_deadline().is_some_and(|deadline| Instant::now() >= deadline)
            {
                self.networking_system.disconnect_from_map_server();
                self.networking_system.disconnect_from_character_server();
                self.networking_system.disconnect_from_login_server();
                self.audio_engine.play_background_music_track(None);
                self.session.finish_shutdown();
                event_loop.exit();
                return;
            }
        }

        // Drive the explicit session lifecycle: map-entry watchdog, map load
        // retries, abort recovery and the character-server reconnect backoff.
        // The old loose timers (`map_entry_watchdog`, `map_entry_abort_deadline`,
        // `pending_map_load`, `character_server_reconnect_at`, …) are owned by
        // the session machine now.
        self.update_session_lifecycle();

        // Clear the previous render instructions so we can rendering the new frame.
        self.clear_render_instructions();

        // Aggregate experience notifications arrive per kill; show them as one
        // chat line once the burst is over.
        self.flush_experience_feedback();

        // It is important that we first apply any changes that were dispatched during
        // the last frame.
        self.update_client_state();

        // We can only apply the graphic changes and reconfigure the surface once the
        // previous image was presented. Moving this function to the end of the
        // function results in surface configuration errors under DX12.
        self.update_settings();

        // Requires the frame rate limit that was just applied, so it is done
        // after `update_settings`.
        self.log_frames_per_second();
        self.log_performance();

        // TODO: Shouldn't this happen later? After the scaling has been potentially
        // changed by the UI.
        let scaling = *self.client_state.follow(client_state().interface_settings().scaling());
        self.update_interface_scaling(scaling);

        let FrameTimers {
            delta_time,
            client_tick,
            animation_timer_ms,
        } = self.game_timer.update();

        if std::mem::take(&mut self.interface_layout_invalidation_pending) {
            self.interface.invalidate_layouts();
        }

        let input_report = self.input_system.update_delta(client_tick);

        self.request_entity_details(&input_report);
        self.request_visible_entity_names();

        self.handle_network_events(client_tick);
        self.update_status_effects(client_tick);
        self.update_asset_load_retries(Instant::now());

        let interface_has_focus = self.process_user_events(
            &input_report,
            client_tick,
            #[cfg(feature = "debug")]
            (delta_time as f32),
        );

        // The shop windows can be closed with their X button, which does not send the
        // close-shop packet by itself. If a shop session is still open on the server
        // but no shop window is open anymore, close it so that the map server stops
        // blocking player movement (it ignores walk requests while an NPC shop is
        // open).
        if self.shop_open
            && !self.interface.is_window_with_class_open(WindowClass::BuyOrSell)
            && !self.interface.is_window_with_class_open(WindowClass::Buy)
            && !self.interface.is_window_with_class_open(WindowClass::BuyCart)
            && !self.interface.is_window_with_class_open(WindowClass::Sell)
            && !self.interface.is_window_with_class_open(WindowClass::SellCart)
        {
            let _ = self.networking_system.close_shop();
            self.shop_open = false;
            self.client_state.follow_mut(client_state().buy_cart()).clear();
            self.client_state.follow_mut(client_state().sell_cart()).clear();
        }

        // The item service windows are answered with a request packet. Closing
        // one with its X button does not answer it, and the identify list in
        // particular leaves `WIP_DISABLE_ALL` on the server, which blocks item
        // and skill use until the request or the official "cancel" value
        // arrives. Send the matching cancel request once the window is gone.
        if !self.interface.is_window_with_class_open(WindowClass::ItemService) {
            let kind = *self.client_state.follow(client_state().item_service().kind());

            if let Some(kind) = kind {
                match kind {
                    ItemServiceKind::ItemIdentify => {
                        let _ = self.networking_system.cancel_identify_item();
                    }
                    ItemServiceKind::WeaponRefine => {
                        let _ = self.networking_system.cancel_weapon_refine();
                    }
                    ItemServiceKind::ItemRepair => {
                        let _ = self.networking_system.cancel_repair_item();
                    }
                    ItemServiceKind::CardComposition => {}
                }

                self.client_state.follow_mut(client_state().item_service()).clear();
            }
        }

        // The NPC refine UI is opened by a server packet, but closing the window
        // with its X button does not tell the server. rAthena keeps
        // `state.refineui_open` set and would then refuse to open the UI again
        // (`clif.cpp:22485`/`22495`), so send the close packet once the window
        // is gone.
        if *self.client_state.follow(client_state().item_service().refining().open())
            && !self.interface.is_window_with_class_open(WindowClass::Refining)
        {
            let _ = self.networking_system.close_refining_ui();
            self.client_state.follow_mut(client_state().item_service().refining()).clear();
        }

        // Some debug windows, such as the packet history or cache statistics, require
        // special update logic.
        #[cfg(feature = "debug")]
        self.update_debug_windows(delta_time);

        // We run this last to give the loader thread as much time as possible to
        // complete the loading. When starting the actual render process, we
        // can't modify resources anymore until the next frame.
        self.update_loaded_resources(client_tick);

        // The character previews are requested after the load results were
        // applied, so an animation that already finished loading in this frame
        // is not requested again.
        self.update_character_selection_previews();
        self.update_character_selection_cooldown();

        // Headless benchmark driver (`RO_NEXT_BENCH`, see `benchmark.rs`); a
        // no-op in normal runs. Runs after the load results were applied so the
        // measured map-change latency includes the actual completion frame.
        crate::benchmark::drive(self, client_tick.0 as u64);

        // Keep the minimap marker in sync with the player. The marker is only
        // updated once the map (and therefore the minimap) is loaded.
        let player_position = self
            .map
            .is_some()
            .then(|| self.client_state.try_follow(this_entity()).map(|entity| entity.get_tile_position()))
            .flatten();

        match player_position {
            Some(player_position) => {
                let previous_position = self.minimap_last_tile.replace(player_position);
                let current_direction = self.client_state.follow(client_state().minimap()).get_player_direction();
                let player_direction = previous_position
                    .and_then(|previous_position| direction_index_from_tile_delta(previous_position, player_position))
                    .unwrap_or(current_direction);

                self.client_state
                    .follow_mut(client_state().minimap())
                    .set_player_position(Some(player_position), player_direction);

                if previous_position != Some(player_position) {
                    self.refresh_navigation_route();
                }
            }
            None => {
                self.minimap_last_tile = None;
                self.client_state.follow_mut(client_state().minimap()).set_player_position(None, 0);
            }
        }

        #[cfg(feature = "debug")]
        let render_options = *self.client_state.follow(client_state().render_options());

        let screen_size = self.graphics_engine.get_window_size().into();
        let currently_playing = self.client_state.try_follow(this_player()).is_some();

        self.mouse_cursor.update(client_tick);

        // Do not touch the swapchain while the native Wayland window is in the
        // background. KWin can stop releasing swapchain images for hidden
        // windows and NVIDIA's Wayland WSI then blocks forever inside the image
        // acquisition.
        if self.pause_presentation_if_needed() {
            return;
        }

        // Acquire the swapchain image as late as possible so all CPU-side
        // preparation overlaps with the previous frame's GPU work. After this
        // point we may not reconfigure the surface (see `update_settings`).
        let before_wait = std::time::Instant::now();
        let maybe_frame = self.graphics_engine.wait_for_next_frame();
        let after_wait = std::time::Instant::now();
        let allocations_after_wait = crate::perf::thread_allocation_snapshot();

        // If we don't have a map, the rendering ends here.
        let Some(map) = self.map.clone() else {
            if let Some(frame) = maybe_frame {
                self.graphics_engine.render_next_frame(frame, Default::default());
                self.has_presented_frame = true;
            }

            return;
        };

        self.update_entities(
            &map,
            currently_playing,
            client_tick,
            #[cfg(feature = "debug")]
            &render_options,
        );

        self.update_level_99_auras();
        self.update_status_auras();
        self.update_status_visuals();

        self.process_buffered_action();

        self.update_main_camera(
            screen_size,
            delta_time,
            #[cfg(feature = "debug")]
            &render_options,
        );

        map.advance_videos(&self.queue, delta_time);

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            self.player_camera.set_smoothed_focus_point(player.get_position());
        }

        // Update the Asura Strike banner.
        // Update particles.
        self.particle_holder.update(delta_time as f32);
        self.effect_holder
            .update(self.client_state.follow(client_state().entities()), delta_time as f32);

        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        let (view_matrix, projection_matrix) = current_camera.view_projection_matrices();
        let camera_position = current_camera.camera_position().to_homogeneous();

        #[cfg(feature = "debug")]
        let update_shadow_camera_measurement = Profiler::start_measurement("update directional shadow camera");

        let lighting_mode = *self.client_state.follow(client_state().graphics_settings().lighting_mode());
        let shadow_resolution = *self.client_state.follow(client_state().graphics_settings().shadow_resolution());
        let shadow_method = *self.client_state.follow(client_state().graphics_settings().shadow_method());
        let shadow_detail = *self.client_state.follow(client_state().graphics_settings().shadow_detail());
        let sdsm_enabled = *self.client_state.follow(client_state().graphics_settings().sdsm());

        let use_sdsm = sdsm_enabled & !self.player_camera.is_rotating_or_zooming_fast();

        let (directional_light_direction, directional_light_color) = map.directional_light();

        match use_sdsm {
            true => {
                self.directional_shadow_camera.update_camera_sdsm(
                    directional_light_direction,
                    &view_matrix,
                    &projection_matrix,
                    shadow_resolution.directional_shadow_resolution(),
                    self.directional_shadow_partitions.lock().unwrap().deref(),
                );
            }
            false => {
                self.directional_shadow_camera.update_camera_pssm(
                    directional_light_direction,
                    &view_matrix,
                    &projection_matrix,
                    shadow_resolution.directional_shadow_resolution(),
                );
            }
        }

        #[cfg(feature = "debug")]
        update_shadow_camera_measurement.stop();

        self.update_audio_engine(current_camera);

        #[cfg(feature = "debug")]
        let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

        #[cfg(feature = "debug")]
        let hovered_marker_identifier = match input_report.mouse_target {
            PickerTarget::Marker(marker_identifier) => Some(marker_identifier),
            _ => None,
        };

        // Resolve the PvP/GvG/PK target rule before `point_light_manager` is
        // borrowed mutably; the cursor decision below only needs this flag.
        let hovered_entity_attackable = match input_report.mouse_target {
            PickerTarget::Entity(entity_id) => self.can_attack_entity(entity_id),
            _ => false,
        };

        let point_light_set = Self::create_point_light_set(
            &mut self.point_light_manager,
            &mut self.point_light_set_buffer,
            &map,
            &self.effect_holder,
            current_camera,
            lighting_mode,
        );

        #[cfg(feature = "debug")]
        prepare_frame_measurement.stop();

        let mut indicator_instruction = None;
        let mut water_instruction = None;

        let mouse_mode = self.interface.get_mouse_mode();
        let is_mouse_mode_default = mouse_mode.is_default();
        let last_walking_destination = mouse_mode.walk_destination();
        let skill_target = mouse_mode.skill_target();

        let allocations_after_sim = crate::perf::thread_allocation_snapshot();
        let after_sim = std::time::Instant::now();

        let mut interface_state_changed = false;

        let mut interface_frame = {
            #[cfg(feature = "debug")]
            profile_block!("user interface");

            let is_rotating_camera = mouse_mode.is_rotating_camera();
            let is_grabbing = mouse_mode.is_grabbing();
            let is_chat_open = self.interface.is_window_with_class_open(WindowClass::Chat);

            // While the interface settings window is open, show the move grip and resize
            // handles on every window (not just the hovered one) so the layout can be
            // arranged while tuning the interface.
            let interface_settings_open = self.interface.is_window_with_class_open(WindowClass::InterfaceSettings);
            self.interface.set_window_chrome_visible(interface_settings_open);

            let is_context_menu_open = self.interface.is_window_with_class_open(WindowClass::ContextMenu);
            let is_item_info_open = self.interface.is_window_with_class_open(WindowClass::ItemInfo);

            let mut interface_frame = self.interface.lay_out_windows(
                &self.client_state,
                scaling.get_factor(),
                input_report.mouse_position,
                input_report.left_mouse_button_down,
            );

            // We can only decide what to do with the user input once we know if the mouse
            // is hovering a window, so we buffer any actions for the next frame.

            let is_interface_hovered = interface_frame.is_interface_hovered();
            let is_context_menu_hovered = interface_frame.is_window_with_class_hovered(WindowClass::ContextMenu);
            let is_item_info_hovered = interface_frame.is_window_with_class_hovered(WindowClass::ItemInfo);

            // A click outside of the item info window closes it.
            if is_item_info_open && !is_item_info_hovered && input_report.mouse_click.is_some() {
                self.input_event_buffer.push(InputEvent::CloseItemInfo);
            }

            if crate::logging::enabled(crate::logging::LEVEL_TRACE) {
                crate::logging::log_trace!(
                    "[input] frame mouse=({:.2}, {:.2}) window={}x{} scaling={:.2} hovered={}",
                    input_report.mouse_position.left,
                    input_report.mouse_position.top,
                    screen_size.width,
                    screen_size.height,
                    scaling.get_factor(),
                    is_interface_hovered,
                );
            }

            // A click outside of the context menu closes it. Left clicks in the
            // world are swallowed so that closing the menu does not also move the
            // player or interact with an entity.
            let click_closes_context_menu = is_context_menu_open && !is_context_menu_hovered;

            if click_closes_context_menu && input_report.mouse_click.is_some() {
                self.input_event_buffer.push(InputEvent::CloseContextMenu);
            }

            let cursor_state = match input_report.mouse_target {
                _ if is_rotating_camera => MouseCursorState::RotateCamera,
                _ if is_grabbing => MouseCursorState::GrabResource,
                _ if skill_target.is_some() => MouseCursorState::Target,
                PickerTarget::Entity(entity_id) if !is_interface_hovered => {
                    if self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .any(|item| item.get_entity_id() == entity_id)
                    {
                        MouseCursorState::HoverItem
                    } else {
                        let entity_type = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| entity.get_entity_type());

                        match entity_type {
                            Some(EntityType::Npc) => MouseCursorState::Dialog,
                            Some(EntityType::Warp) => MouseCursorState::Warp,
                            Some(EntityType::Monster) => MouseCursorState::Attack,
                            // The official client only shows the attack cursor
                            // on a player when the PvP/GvG/PK rule allows the
                            // attack; otherwise it keeps the default cursor
                            // (`CGameMode::HandleActorPick` `SetCursorAction(0)`).
                            Some(EntityType::Player) if hovered_entity_attackable => MouseCursorState::Attack,
                            _ => MouseCursorState::Default,
                        }
                    }
                }
                _ => MouseCursorState::Default,
            };
            self.mouse_cursor.set_state(cursor_state, client_tick);

            if let Some(mouse_button) = input_report.mouse_click {
                interface_state_changed = true;

                if crate::logging::enabled(crate::logging::LEVEL_DEBUG) {
                    crate::logging::log_debug!(
                        "[input] click {mouse_button:?} at ({:.1}, {:.1}) hovered={is_interface_hovered}",
                        input_report.mouse_position.left,
                        input_report.mouse_position.top,
                    );
                }

                if is_interface_hovered && self.input_system.has_pointer_position() {
                    interface_frame.click(&self.client_state, mouse_button);
                } else if click_closes_context_menu && mouse_button == MouseButton::Left {
                    // The left click only closed the context menu.
                } else {
                    interface_frame.unfocus();

                    if let Some((_, skill_id, skill_type, level)) = skill_target {
                        // A skill target is being selected. Any click ends the
                        // selection, and a left click casts the skill at whatever
                        // is under the cursor. The mode is reset here explicitly
                        // because the release that starts a selection no longer
                        // resets it (the mode is sticky for one click).
                        interface_frame.set_mouse_mode(MouseMode::Default);

                        if mouse_button == MouseButton::Left {
                            self.input_event_buffer.push(InputEvent::CastSkill {
                                skill_id,
                                skill_type,
                                level,
                            });
                        }
                    } else if mouse_button == MouseButton::Left {
                        match input_report.mouse_target {
                            PickerTarget::Nothing => {}
                            PickerTarget::Entity(entity_id) => {
                                let is_ground_item = self
                                    .client_state
                                    .follow(client_state().ground_items())
                                    .iter()
                                    .any(|item| item.get_entity_id() == entity_id);

                                if is_ground_item {
                                    self.input_event_buffer.push(InputEvent::PickUpItem { entity_id })
                                } else {
                                    self.input_event_buffer.push(InputEvent::PlayerInteract { entity_id })
                                }
                            }
                            PickerTarget::Tile { x, y } => {
                                let destination = TilePosition { x, y };

                                interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });

                                self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                            }
                            #[cfg(feature = "debug")]
                            PickerTarget::Marker(marker_identifier) => {
                                self.input_event_buffer.push(InputEvent::OpenMarkerDetails { marker_identifier })
                            }
                        }
                    } else if mouse_button == MouseButton::Right && currently_playing {
                        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(!render_options.use_debug_camera))]
                        interface_frame.set_mouse_mode(MouseInputMode::RotateCamera);
                    } else if mouse_button == MouseButton::DoubleRight && currently_playing {
                        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(!render_options.use_debug_camera))]
                        self.input_event_buffer.push(InputEvent::ResetCameraRotation);
                    }
                }
            } else if let Some(last_destination) = last_walking_destination
                && let PickerTarget::Tile { x, y } = input_report.mouse_target
                && input_report.left_mouse_button_down
            {
                let destination = TilePosition { x, y };

                if last_destination != destination {
                    interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });
                    self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                }
            }

            // Open the character context menu when the right mouse button was
            // pressed and released over a character without dragging. A right
            // drag keeps rotating the camera like before.
            const RIGHT_CLICK_DRAG_THRESHOLD: f32 = 4.0;

            if input_report.right_mouse_button_released
                && is_rotating_camera
                && !is_interface_hovered
                && currently_playing
                && input_report.right_mouse_button_drag_distance < RIGHT_CLICK_DRAG_THRESHOLD
                && skill_target.is_none()
            {
                if is_context_menu_open {
                    self.input_event_buffer.push(InputEvent::CloseContextMenu);
                } else if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                    let is_character = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .is_some_and(|entity| {
                            matches!(
                                entity.get_entity_type(),
                                EntityType::Player | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary
                            )
                        });
                    let is_own_character = self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|entity| entity.get_entity_id() == entity_id);

                    if is_character && !is_own_character {
                        self.input_event_buffer.push(InputEvent::OpenCharacterContextMenu {
                            entity_id,
                            position: input_report.mouse_position,
                        });
                    }
                }
            }

            if input_report.mouse_button_released {
                // A click that starts a sticky mouse mode (targeted skill
                // selection) must not be cancelled by the release of that same
                // click; the mode stays until the next click.
                interface_frame.drop(&self.client_state, skill_target.is_none());
            }

            if let Some(delta) = input_report.scroll {
                if let Some((slot, skill_id, skill_type, level)) = skill_target {
                    // While selecting a skill target the mouse wheel adjusts the
                    // cast level instead of zooming, like in the original client.
                    let maximum_level = match slot {
                        Some(slot) => self
                            .client_state
                            .follow(client_state().hotbar())
                            .get_skill_in_slot(slot)
                            .as_ref()
                            .map(|skill| skill.maximum_level.0)
                            .unwrap_or(1),
                        None => self
                            .client_state
                            .follow(client_state().skill_tree().skills())
                            .iter()
                            .find(|learned_skill| learned_skill.skill_id == skill_id)
                            .map(|learned_skill| learned_skill.skill_level.0)
                            .unwrap_or(1),
                    };

                    let level = match delta > 0.0 {
                        true => SkillLevel(level.0.saturating_add(1).min(maximum_level)),
                        false => SkillLevel(level.0.saturating_sub(1).max(1)),
                    };

                    interface_frame.set_mouse_mode(MouseInputMode::TargetSkill {
                        slot,
                        skill_id,
                        skill_type,
                        level,
                    });
                } else if is_interface_hovered {
                    interface_state_changed = true;
                    interface_frame.scroll(&self.client_state, delta);
                } else {
                    #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(!render_options.use_debug_camera))]
                    self.input_event_buffer.push(InputEvent::ZoomCamera { zoom_factor: delta });
                }
            }

            // Focus the chat if the interface is not focused, no other element is capturing
            // the keyboard input, enter was pressed, and the chat
            // window is open. Enter arrives as the `\x0d` text character on most
            // platforms; the data-driven chat command key binding
            // (`data\client\keybinds.ron`) covers the platforms/backends where the
            // physical key does not produce a text character.
            if (!interface_has_focus || !interface_frame.input_characters(&self.client_state, &input_report.characters))
                && (input_report.characters.contains(&'\x0d') || self.input_system.chat_command_key_pressed())
                && is_chat_open
            {
                interface_frame.focus_element(ChatTextBox);
            }

            if interface_state_changed {
                // Clicks/scrolls are handled on the interface frame after the
                // layout was created, so the layouts have to be rebuilt next
                // frame (the interface itself is borrowed by the frame here).
                self.interface_layout_invalidation_pending = true;
            }

            interface_frame
        };

        let allocations_after_interface = crate::perf::thread_allocation_snapshot();
        let after_interface = std::time::Instant::now();

        // Drop warp portals whose server removal packet was lost (safety net; the
        // normal removal is the `RemoveSkillUnit` packet).
        {
            let now = std::time::Instant::now();
            self.warp_skill_units.retain(|(_, _, _, expires_at)| *expires_at > now);
        }

        {
            let mut render_context = MapRenderContext {
                map: &map,
                current_camera,
                point_light_set: &point_light_set,
                client_state: &self.client_state,
                library: &self.library,
                guild_emblems: &self.guild_emblems,
                mouse_position: input_report.mouse_position,
                mouse_target: input_report.mouse_target,
                screen_size,
                scaling,
                client_tick,
                animation_timer_ms,
                currently_playing,
                is_mouse_mode_default,
                is_interface_hovered: interface_frame.is_interface_hovered(),
                last_walking_destination,
                is_targeting_ground_skill: matches!(skill_target, Some((_, _, SkillType::Ground | SkillType::Trap, _))),
                skill_target_level: skill_target.map(|(_, _, _, level)| level),
                buffered_action: *self.client_state.follow(client_state().buffered_action()),
                #[cfg(feature = "debug")]
                render_options: &render_options,
                #[cfg(feature = "debug")]
                hovered_marker_identifier,
                #[cfg(feature = "debug")]
                pathing_texture_set: &self.pathing_texture_set,
                #[cfg(feature = "debug")]
                tile_texture_set: &self.tile_texture_set,
                #[cfg(feature = "debug")]
                player_camera: &self.player_camera,
                #[cfg(feature = "debug")]
                start_camera: &self.start_camera,
                model_batches: &mut self.model_batches,
                model_instructions: &mut self.model_instructions,
                entity_instructions: &mut self.entity_instructions,
                directional_shadow_camera: &mut self.directional_shadow_camera,
                directional_shadow_model_batches: &mut self.directional_shadow_model_batches,
                directional_shadow_model_instructions: &mut self.directional_shadow_model_instructions,
                directional_shadow_entity_instructions: &mut self.directional_shadow_entity_instructions,
                point_shadow_camera: &mut self.point_shadow_camera,
                point_shadow_model_instructions: &mut self.point_shadow_model_instructions,
                point_shadow_entity_instructions: &mut self.point_shadow_entity_instructions,
                point_light_with_shadow_instructions: &mut self.point_light_with_shadow_instructions,
                point_light_instructions: &mut self.point_light_instructions,
                directional_shadow_object_set_buffer: &mut self.directional_shadow_object_set_buffer,
                point_shadow_object_set_buffer: &mut self.point_shadow_object_set_buffer,
                deferred_object_set_buffer: &mut self.deferred_object_set_buffer,
                indicator_instruction: &mut indicator_instruction,
                water_instruction: &mut water_instruction,
                particle_holder: &mut self.particle_holder,
                effect_holder: &mut self.effect_holder,
                chat_bubbles: &self.chat_bubbles,
                head_marks: &self.head_marks,
                cast_bars: &self.cast_bars,
                skill_name_texts: &self.skill_name_texts,
                warp_portal: self.warp_portal.as_ref(),
                weather: self.weather.as_ref(),
                spirit_sphere_texture: self.spirit_sphere_texture.as_ref(),
                warp_skill_units: &self.warp_skill_units,
                effect_renderer: &mut self.effect_renderer,
                bottom_interface_renderer: &self.bottom_interface_renderer,
                middle_interface_renderer: &mut self.middle_interface_renderer,
                #[cfg(feature = "debug")]
                aabb_instructions: &mut self.aabb_instructions,
                #[cfg(feature = "debug")]
                circle_instructions: &mut self.circle_instructions,
                #[cfg(feature = "debug")]
                rectangle_instructions: &mut self.rectangle_instructions,
                #[cfg(feature = "debug")]
                bounding_box_object_set_buffer: &mut self.bounding_box_object_set_buffer,
                #[cfg(feature = "debug")]
                debug_marker_renderer: &mut self.debug_marker_renderer,
            };

            #[cfg(feature = "debug")]
            render_context.render_markers();
            render_context.render_directional_shadows();
            render_context.render_point_lights();
            render_context.render_geometry();
            #[cfg(feature = "debug")]
            render_context.render_bounding_boxes();
            render_context.render_world_overlays();
        }

        let in_game_theme_path = client_state().in_game_theme().tooltip();
        let menu_theme_path = client_state().menu_theme().tooltip();
        let tooltip_theme = match currently_playing {
            true => self.client_state.follow(in_game_theme_path),
            false => self.client_state.follow(menu_theme_path),
        };

        interface_frame.render(
            &self.client_state,
            &self.interface_renderer,
            tooltip_theme,
            input_report.mouse_position,
        );

        drop(interface_frame);

        self.render_ui_overlays(
            &input_report,
            scaling,
            #[cfg(feature = "debug")]
            &render_options,
        );

        let picker_position = ScreenPosition {
            left: input_report.mouse_position.left.clamp(0.0, screen_size.width),
            top: input_report.mouse_position.top.clamp(0.0, screen_size.height),
        };

        // Sky/background and scene fog from `mapskydata.lub`. Maps without sky
        // data keep the black background, so existing maps render exactly as
        // before. `BG_Fog` fades the geometry into the same colour (it is not
        // enabled by this data set, the guarded path stays nevertheless).
        let background_color = map.sky_background_color().unwrap_or(Color::BLACK);
        let scene_fog = map.scene_fog();

        let uniforms = Uniforms {
            view_matrix,
            projection_matrix,
            camera_position,
            animation_timer_ms,
            ambient_light_color: map.ambient_light_color(),
            fog_color: scene_fog.map(|fog| Color { alpha: 1.0, ..fog.color }).unwrap_or(Color::TRANSPARENT),
            fog_range: scene_fog.map(|fog| [fog.near, fog.far]).unwrap_or([0.0, 0.0]),
            enhanced_lighting: lighting_mode == LightingMode::Enhanced,
            shadow_method,
            shadow_detail,
            use_sdsm,
            sdsm_enabled,
        };

        let interface_instructions = self.interface_renderer.get_instructions();
        let bottom_layer_instructions = self.bottom_interface_renderer.get_instructions();
        let middle_layer_instructions = self.middle_interface_renderer.get_instructions();
        let top_layer_instructions = self.top_interface_renderer.get_instructions();

        let directional_light = DirectionalLightInstruction {
            view_projection_matrix: self.directional_shadow_camera.view_projection_matrix(),
            direction: directional_light_direction,
            color: directional_light_color,
        };

        let allocations_after_world = crate::perf::thread_allocation_snapshot();
        let after_world = std::time::Instant::now();

        let render_instruction = RenderInstruction {
            show_interface: self.show_interface,
            background_color,
            picker_position,
            uniforms,
            indicator: indicator_instruction,
            interface: interface_instructions.as_slice(),
            bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
            middle_layer_rectangles: middle_layer_instructions.as_slice(),
            top_layer_rectangles: top_layer_instructions.as_slice(),
            directional_light,
            directional_light_partitions: &self.directional_shadow_camera.get_partition_instructions(),
            point_light: &self.point_light_instructions,
            point_light_with_shadows: &self.point_light_with_shadow_instructions,
            model_batches: &self.model_batches,
            models: &mut self.model_instructions,
            entities: &mut self.entity_instructions,
            directional_shadow_model_batches: &self.directional_shadow_model_batches,
            directional_shadow_models: &self.directional_shadow_model_instructions,
            directional_shadow_entities: &mut self.directional_shadow_entity_instructions,
            point_shadow_models: &self.point_shadow_model_instructions,
            point_shadow_entities: &mut self.point_shadow_entity_instructions,
            effects: self.effect_renderer.get_instructions(),
            water: water_instruction,
            map_picker_tile_vertex_buffer: Some(map.get_tile_picker_vertex_buffer()),
            map_picker_tile_index_buffer: Some(map.get_tile_picker_index_buffer()),
            slug_font: Some(self.font_loader.get_slug_font()),
            #[cfg(feature = "debug")]
            render_options,
            #[cfg(feature = "debug")]
            aabb: &self.aabb_instructions,
            #[cfg(feature = "debug")]
            circles: &self.circle_instructions,
            #[cfg(feature = "debug")]
            rectangles: &self.rectangle_instructions,
            #[cfg(feature = "debug")]
            marker: self.debug_marker_renderer.get_instructions(),
        };

        if let Some(frame) = maybe_frame {
            self.graphics_engine.render_next_frame(frame, render_instruction);
            self.has_presented_frame = true;
        }

        let after_gpu = std::time::Instant::now();
        let allocations_after_gpu = crate::perf::thread_allocation_snapshot();

        crate::perf::sample_counts(
            self.effect_holder.count() as u32,
            self.particle_holder.count() as u32,
            self.client_state.follow(client_state().entities()).len() as u32,
        );

        let (rendered_effects, culled_effects) = self.effect_holder.render_counts();
        let (rendered_particles, culled_particles) = self.particle_holder.render_counts();
        crate::perf::sample_render_counts(rendered_effects, culled_effects, rendered_particles, culled_particles);

        record_frame_phase(
            after_wait.saturating_duration_since(before_wait),
            after_sim.saturating_duration_since(after_wait),
            after_interface.saturating_duration_since(after_sim),
            after_world.saturating_duration_since(after_interface),
            after_gpu.saturating_duration_since(after_world),
            [
                allocations_after_sim.saturating_sub(allocations_after_wait),
                allocations_after_interface.saturating_sub(allocations_after_sim),
                allocations_after_world.saturating_sub(allocations_after_interface),
                allocations_after_gpu.saturating_sub(allocations_after_world),
            ],
        );
    }
}
