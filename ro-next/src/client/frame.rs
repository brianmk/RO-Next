//! Frame pacing, window focus, logging, interface scaling and the session
//! lifecycle tick.
//!
//! These run once per frame from `Client::run`; they never touch packet or
//! input data.
use crate::*;

impl Client {
    /// Whether the client window has keyboard focus right now.
    pub(crate) fn window_has_focus(&self) -> bool {
        self.window.as_ref().is_some_and(|window| window.has_focus())
    }

    /// Whether the client window is currently considered to be in the
    /// background, and should therefore use the background frame rate limit.
    pub(crate) fn window_is_in_background(&self) -> bool {
        if self.force_background || self.window_occluded || self.window_minimized {
            return true;
        }

        self.window.is_some() && !self.window_has_focus()
    }

    /// Whether presenting frames has to be paused because the native Wayland
    /// window is in the background.
    ///
    /// Wayland does not report occlusion to clients. When the window is hidden
    /// (covered by another window, minimized or simply unfocused) KWin can stop
    /// releasing the swapchain images it still holds. NVIDIA's Vulkan Wayland
    /// WSI then blocks forever inside the blocking image acquisition
    /// (`drmSyncobjTimelineWait`), which freezes the whole client. To keep the
    /// native Wayland backend usable, the client does not touch the swapchain
    /// while the window is in the background; the next redraw after focus is
    /// regained presents normally again.
    ///
    /// Set `RO_NEXT_WAYLAND_BACKGROUND_RENDER=1` to keep rendering in the
    /// background at the configured background frame rate (this restores the
    /// old behavior, but exposes the driver deadlock again).
    pub(crate) fn should_pause_presentation(&self) -> bool {
        if !self.backend_is_wayland {
            return false;
        }

        if environment_flag("RO_NEXT_WAYLAND_BACKGROUND_RENDER") {
            return false;
        }

        // The very first frame must always be presented, otherwise the compositor never
        // maps the window (an unmapped window is invisible and cannot be
        // focused by a click).
        if !self.has_presented_frame {
            return false;
        }

        self.window_occluded || self.window_minimized || !self.window_has_focus()
    }

    /// Applies a pause of the presentation loop.
    ///
    /// Returns `true` when the frame was skipped, in which case the caller must
    /// not touch the swapchain.
    pub(crate) fn pause_presentation_if_needed(&mut self) -> bool {
        if !self.should_pause_presentation() {
            if self.presentation_paused {
                self.presentation_paused = false;
                crate::logging::log_info!("[surface] presentation resumed (native Wayland window is in the foreground)");
            }

            return false;
        }

        if !self.presentation_paused {
            self.presentation_paused = true;
            crate::logging::log_info!(
                "[surface] presentation paused while the native Wayland window is in the background (set \
                 RO_NEXT_WAYLAND_BACKGROUND_RENDER=1 to render in the background)"
            );
        }

        // The event loop keeps running so that network updates, input and focus changes
        // are still processed; a redraw is requested again after this frame.
        std::thread::sleep(Duration::from_millis(15));

        true
    }

    /// Applies the frame rate limit that matches the current window state and
    /// the persisted graphics settings.
    pub(crate) fn apply_frames_per_second_limit(&mut self) {
        let background = self.window_is_in_background();

        let frames_per_second = self
            .client_state
            .follow(client_state().graphics_settings())
            .effective_max_fps(background);

        self.set_frames_per_second_limit(frames_per_second, background);
    }

    /// Applies a frame rate limit to the graphics engine and logs the
    /// transition whenever the limit actually changes.
    pub(crate) fn set_frames_per_second_limit(&mut self, frames_per_second: u32, background: bool) {
        if frames_per_second == self.active_frames_per_second_limit {
            return;
        }

        self.active_frames_per_second_limit = frames_per_second;
        self.graphics_engine.set_framerate_limit(frames_per_second);

        let limit = match frames_per_second {
            0 => "unlimited".to_owned(),
            frames_per_second => format!("{frames_per_second} FPS"),
        };

        let reason = if background { "window in background" } else { "window focused" };

        println!("[framerate] frame rate limit: {limit} ({reason})");
    }

    /// Refreshes the minimized state of the window.
    ///
    /// Minimizing is not reported through a dedicated window event on all
    /// platforms (and querying it is a synchronous round trip on X11), so this
    /// is only called when the window state may have changed.
    pub(crate) fn refresh_window_minimized(&mut self) {
        self.window_minimized = self.window.as_ref().and_then(|window| window.is_minimized()).unwrap_or(false);
    }

    /// Logs the effective frame rate when `RO_NEXT_FPS_LOG` is enabled.
    pub(crate) fn log_frames_per_second(&mut self) {
        let Some(deadline) = self.frames_per_second_log_deadline else {
            return;
        };

        let now = Instant::now();

        if now < deadline {
            return;
        }

        self.frames_per_second_log_deadline = Some(now + Duration::from_secs(FRAMERATE_LOG_INTERVAL_SECONDS));

        println!(
            "[fps] cap={} focused={} background={} effective_fps={:.1}",
            self.active_frames_per_second_limit,
            self.window_has_focus(),
            self.window_is_in_background(),
            self.graphics_engine.measured_frames_per_second(),
        );
    }

    /// Logs window layout / text shaping / frame statistics when
    /// `RO_NEXT_PERF_LOG` is enabled. Used to measure the effect of the layout
    /// and text caches.
    pub(crate) fn log_performance(&mut self) {
        let Some(deadline) = self.performance_log_deadline else {
            return;
        };

        let now = Instant::now();

        if now < deadline {
            return;
        }

        let interval = now.saturating_duration_since(deadline - Duration::from_secs(PERFORMANCE_LOG_INTERVAL_SECONDS));
        self.performance_log_deadline = Some(now + Duration::from_secs(PERFORMANCE_LOG_INTERVAL_SECONDS));

        let layout_profile = take_layout_profile();
        let text_statistics = take_layout_cache_statistics();
        let frame_profile = take_frame_phase_profile();

        let frames = layout_profile.frames.max(1) as f64;
        let frame_frames = frame_profile.frames.max(1) as f64;
        let interval_seconds = interval.as_secs_f64().max(0.001);
        let create_layout_ms = layout_profile.create_layout_nanos as f64 / 1_000_000.0;
        let lay_out_ms = layout_profile.lay_out_nanos as f64 / 1_000_000.0;
        let windows = layout_profile.windows as f64 / frames;
        let reused_percent = match layout_profile.windows {
            0 => 0.0,
            windows => layout_profile.reused_windows as f64 * 100.0 / windows as f64,
        };
        let text_hit_rate = match text_statistics.calls {
            0 => 0.0,
            calls => text_statistics.hits as f64 * 100.0 / calls as f64,
        };

        let frame_ms = |nanos: u64| nanos as f64 / 1_000_000.0 / frame_frames;

        println!(
            "[perf] fps={:.1} windows={:.1} layout_create={:.3}ms/frame layout={:.3}ms/frame layout_reused={:.0}% text_calls={:.0}/s \
             text_hits={:.1}% text_shaped={:.0}/s",
            self.graphics_engine.measured_frames_per_second(),
            windows,
            create_layout_ms / frames,
            lay_out_ms / frames,
            reused_percent,
            text_statistics.calls as f64 / interval_seconds,
            text_hit_rate,
            text_statistics.misses as f64 / interval_seconds,
        );

        if let Some(line) = crate::perf::loader_log_line() {
            println!("{line}");
        }

        // Budget telemetry: how many effects/particles are alive and how many
        // were dropped by the documented caps since startup. A crowded town
        // should show the counts staying at the caps instead of growing.
        println!(
            "[budget] effects={} effects_dropped={} particles={} particles_dropped={}",
            self.effect_holder.count(),
            self.effect_holder.dropped_effects(),
            self.particle_holder.count(),
            self.particle_holder.dropped_particles(),
        );
        println!(
            "[perf-phase] stage={} wait={:.2}ms sim={:.2}ms interface={:.2}ms world={:.2}ms gpu={:.2}ms frame_total={:.2}ms",
            crate::perf::current_stage().unwrap_or("-"),
            frame_ms(frame_profile.wait_nanos),
            frame_ms(frame_profile.sim_nanos),
            frame_ms(frame_profile.interface_nanos),
            frame_ms(frame_profile.world_nanos),
            frame_ms(frame_profile.gpu_nanos),
            (frame_profile.wait_nanos
                + frame_profile.sim_nanos
                + frame_profile.interface_nanos
                + frame_profile.world_nanos
                + frame_profile.gpu_nanos) as f64
                / 1_000_000.0
                / frame_frames,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_interface_scaling(&mut self, scaling: Scaling) {
        static LAST_INTERFACE_SCALING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let scaling_bits = scaling.get_factor().to_bits();
        if LAST_INTERFACE_SCALING.swap(scaling_bits, std::sync::atomic::Ordering::Relaxed) != scaling_bits {
            crate::logging::log_info!("interface: applying interface scaling {}", scaling.get_factor());
            self.interface.invalidate_layouts();
        }
        self.bottom_interface_renderer.update_scaling(scaling);
        self.middle_interface_renderer.update_scaling(scaling);
        self.top_interface_renderer.update_scaling(scaling);
    }

    /// Drive the explicit session lifecycle (P1-3): map entry watchdog, map
    /// load retries, map abort recovery and the character-server reconnect
    /// backoff.
    ///
    /// The [`SessionMachine`] only decides *what* has to happen; the client
    /// executes the returned actions here, so the transition logic stays
    /// testable without a window, a network or a game server.
    pub(crate) fn update_session_lifecycle(&mut self) {
        let now = Instant::now();

        let observation = SessionObservation {
            map_assets_loaded: self.map.is_some(),
            player_present: self.client_state.try_follow(this_entity()).is_some(),
            login_server_connected: self.networking_system.is_login_server_connected(),
            login_surface_visible: self.interface.is_window_with_class_open(WindowClass::Login)
                || self.interface.is_window_with_class_open(WindowClass::SelectServer),
        };

        let actions = self.session.tick(now, &observation);

        for action in actions {
            match action {
                SessionAction::AbortMapEntry { reason } => self.abort_map_entry(reason),
                SessionAction::RecoverToCharacterSelection => self.return_to_character_selection(),
                SessionAction::RetryMapLoad { map_name, position } => {
                    self.async_loader.request_map_load(map_name, position);
                }
                SessionAction::MapLoadGaveUp { map_name } => {
                    // Do not leave the client on a black screen: recover to the
                    // character selection and show the reason there.
                    let message = self.render_localized(client_state().localization().error_map_load_failed(), &[(
                        "map_name",
                        map_name.as_str(),
                    )]);
                    self.session.set_map_load_error(message.clone());
                    self.interface.open_window(ErrorWindow::new(message));
                    self.return_to_character_selection();
                }
                SessionAction::ReconnectCharacterServer => {
                    let Some(saved_login_data) = self.saved_login_data.as_ref() else {
                        self.session.clear_character_reconnect();
                        continue;
                    };
                    let Some(server) = self.saved_character_server.clone() else {
                        self.session.clear_character_reconnect();
                        continue;
                    };

                    self.networking_system
                        .connect_to_character_server(self.saved_packet_version, saved_login_data, server);
                }
                SessionAction::CharacterReconnectGaveUp => {
                    // The character-server session is tied to the login ids from
                    // the login server. Once the reconnect gives up (e.g. after a
                    // map-server restart), retrying with those stale ids keeps
                    // being rejected, so drop them and require a fresh login.
                    self.saved_login_data = None;
                    self.saved_character_server = None;
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system.disconnect_from_login_server();
                    self.session.reset_to_login();
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_characters(Vec::new());

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.interface.open_window(LoginWindow::new(
                        client_state().login_window(),
                        client_state().login_settings(),
                        client_state().client_info(),
                    ));
                    let message = self.render_localized(client_state().localization().error_session_expired(), &[]);
                    self.interface.open_window(ErrorWindow::new(message));
                }
            }
        }
    }

    /// Prints one line about the display backend and the window properties.
    ///
    /// This is intentional normal output (like the `[fps]` and `[framerate]`
    /// lines) so that it is possible to verify which backend the client picked
    /// and how the compositor scaled the window, also in release builds.
    #[inline(always)]
    pub(crate) fn log_display_information(&self, window: &Window) {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let backend = match window.window_handle().map(|handle| handle.as_raw()) {
            Ok(RawWindowHandle::Wayland(_)) => "wayland",
            Ok(RawWindowHandle::Xcb(_)) => "x11-xcb",
            Ok(RawWindowHandle::Xlib(_)) => "x11-xlib",
            Ok(_) => "unknown",
            Err(_) => "unknown",
        };

        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let fullscreen = window.fullscreen().is_some();
        let window_resolution = *self.client_state.follow(client_state().graphics_settings().window_resolution());

        println!(
            "[display] backend={backend} size={}x{} scale_factor={scale_factor:.2} fullscreen={fullscreen} \
             resolution={window_resolution:?}",
            size.width, size.height,
        );
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

    /// Resolve a floor click against the projected pick rectangles of ground
    /// items and (when enabled) monsters.
    ///
    /// The GPU picker stays pixel-exact. This pass only runs when it did not
    /// find an entity (`Nothing`/`Tile`): ground items are always assisted with
    /// the official 34 px item pick rectangle, so a small drop next to a
    /// monster can be clicked without hitting only its few opaque pixels. The
    /// monster padding/minimum-size assist is the `easy_monster_picking`
    /// setting. A raw entity target (human, NPC, item, or a monster already
    /// under the cursor) is never replaced.
    #[allow(clippy::too_many_arguments)]
    fn assist_pick(
        &self,
        input_report: &mut InputReport,
        screen_size: ScreenSize,
        currently_playing: bool,
        #[cfg(feature = "debug")] use_debug_camera: bool,
        client_tick: ClientTick,
    ) {
        // The raw picker already found an entity (including a human under the
        // cursor), so the assist cannot change the outcome and the projected
        // sprite bounds do not have to be computed.
        if !matches!(input_report.mouse_target, PickerTarget::Nothing | PickerTarget::Tile { .. }) {
            return;
        }

        let camera: &dyn Camera = match currently_playing {
            #[cfg(feature = "debug")]
            _ if use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        // The official client pads its projected pick rectangles in UI-scaled
        // pixels, so follow the current interface scale here as well.
        let scaling_factor = self.client_state.follow(client_state().interface_settings().scaling()).get_factor();

        // Items are always assisted, independent of the monster setting: the
        // GPU picker needs a fully opaque sprite pixel, and many item sprites
        // are only 8-24 px wide. The official client gives every item a 34 px
        // minimum pick rectangle (`Item.cpp:53-86`).
        let item_candidates: Vec<ItemPickCandidate> = self
            .client_state
            .follow(client_state().ground_items())
            .iter()
            .filter_map(|item| {
                item.pick_screen_bounds(camera, screen_size).map(|bounds| ItemPickCandidate {
                    entity_id: item.get_entity_id(),
                    bounds,
                    depth: camera.distance_to(item.world_position),
                })
            })
            .collect();

        // The monster assist is the setting: with it off the padded monster
        // rectangles are not considered at all, exactly as before the feature.
        let easy_monster_picking = *self.client_state.follow(client_state().game_settings().easy_monster_picking());

        let monster_candidates: Vec<MonsterPickCandidate> = match easy_monster_picking {
            false => Vec::new(),
            true => self
                .client_state
                .follow(client_state().entities())
                .iter()
                .filter(|entity| entity.get_entity_type() == EntityType::Monster && !entity.is_dead() && !entity.is_fading())
                .filter_map(|entity| {
                    entity_pick_screen_bounds(entity, camera, screen_size, client_tick).map(|bounds| MonsterPickCandidate {
                        entity_id: entity.get_entity_id(),
                        entity_type: entity.get_entity_type(),
                        bounds,
                        depth: camera.distance_to(entity.get_position()),
                    })
                })
                .collect(),
        };

        if item_candidates.is_empty() && monster_candidates.is_empty() {
            return;
        }

        let cursor = cgmath::Vector2::new(input_report.mouse_position.left, input_report.mouse_position.top);

        input_report.mouse_target = resolve_pick(
            input_report.mouse_target,
            cursor,
            &item_candidates,
            &monster_candidates,
            PickAssistConfig {
                item_minimum_size: ITEM_MIN_PICK_SIZE * scaling_factor,
                monster_padding: MONSTER_PICK_PADDING * scaling_factor,
                monster_minimum_size: MONSTER_MIN_PICK_SIZE * scaling_factor,
            },
        );
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

        // The pick assist has to run before any consumer of `mouse_target`
        // (details request, hover cursor, click handling, skill targeting).
        // It uses the current sprite animation of the monsters and the camera
        // from the previous frame, which is the same camera the picker pass
        // rendered the picker texture with. These are separate locals because
        // the frame's own `currently_playing`/`render_options` are read after
        // the network events below, as before.
        let pick_screen_size = self.graphics_engine.get_window_size().into();
        let pick_currently_playing = self.client_state.try_follow(this_player()).is_some();
        #[cfg(feature = "debug")]
        let pick_use_debug_camera = *self.client_state.follow(client_state().render_options().use_debug_camera());

        let mut input_report = self.input_system.update_delta(client_tick);
        self.assist_pick(
            &mut input_report,
            pick_screen_size,
            pick_currently_playing,
            #[cfg(feature = "debug")]
            pick_use_debug_camera,
            client_tick,
        );

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
        {
            // The ported effects record the caster's `SetColor`/`SetFrameSequence`
            // requests on their runtime while `EffectBase::update` only sees an
            // immutable slice; hand the holder the mutable entity list so it can
            // apply them after each step (`EffectHolder::update`).
            let entities = self.client_state.follow_mut(client_state().entities());
            self.effect_holder.update(entities.as_mut_slice(), delta_time as f32, client_tick);

            // `MM_QUAKE`: ported effect bodies count their screen shakes on
            // their runtime; apply the drained requests to the local camera.
            let shakes = self.effect_holder.take_shakes();
            if shakes > 0 {
                self.player_camera.shake(0.65, 0.9);
            }
        }

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

                                self.input_event_buffer.push(world_entity_click(entity_id, is_ground_item));
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
                weather: self.weather.as_ref(),
                spirit_sphere_texture: self.spirit_sphere_texture.as_ref(),
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
            effect_triangles: self.effect_renderer.get_triangle_instructions(),
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

/// The input event a left click on an entity produces.
///
/// Ground items turn into a pickup request for exactly the clicked item entity
/// id; every other entity keeps the interaction request. Kept as a free
/// function so the item -> pickup mapping is unit-testable without a `Client`.
pub(crate) fn world_entity_click(entity_id: EntityId, is_ground_item: bool) -> InputEvent {
    match is_ground_item {
        true => InputEvent::PickUpItem { entity_id },
        false => InputEvent::PlayerInteract { entity_id },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ground_item_click_produces_a_pickup_request_for_the_same_entity_id() {
        let entity_id = EntityId(1_234_567);

        match world_entity_click(entity_id, true) {
            InputEvent::PickUpItem { entity_id: picked_up } => assert_eq!(picked_up, entity_id),
            event => panic!("ground item click must produce PickUpItem, got {event:?}"),
        }
    }

    #[test]
    fn non_item_entity_click_still_produces_an_interaction_request() {
        let entity_id = EntityId(42);

        match world_entity_click(entity_id, false) {
            InputEvent::PlayerInteract { entity_id: interacted } => assert_eq!(interacted, entity_id),
            event => panic!("actor click must produce PlayerInteract, got {event:?}"),
        }
    }
}
