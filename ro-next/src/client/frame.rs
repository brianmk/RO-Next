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
