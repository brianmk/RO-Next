//! Opening, repositioning and toggling the HUD windows.
//!
//! The windows themselves live in `interface/windows`; these helpers decide
//! *when* a window is opened or toggled from gameplay events.
use crate::*;

impl Client {
    /// Opens the party window on the right side of the screen.
    ///
    /// The default window position is the center of the screen, which overlaps
    /// the permanently open chat window - the "Leave party" button would be
    /// below the chat window and could not be clicked. Placing the party window
    /// next to the chat avoids that.
    pub(crate) fn open_party_window(&mut self) {
        if self.interface.is_window_with_class_open(WindowClass::Party) {
            return;
        }

        let window_size = *self.client_state.follow(client_state().window_size());
        let width = 380.0;
        let position = ScreenPosition {
            left: (window_size.width - width - 20.0).max(20.0),
            top: 100.0,
        };

        self.interface.open_window_at(PartyWindow, position);
    }

    /// Open (or re-open) the hotbar at the bottom center of the screen.
    ///
    /// The hotbar is a fixed-size HUD element: its size is derived from the ten
    /// large slots and it is centered by the client instead of using a cached
    /// window position, so it stays centered at every resolution and interface
    /// scale.
    pub(crate) fn open_hotbar_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Hotbar);

        let window_size = *self.client_state.follow(client_state().window_size());
        let scaling = self.client_state.follow(client_state().interface_settings().scaling()).get_factor();
        let width = HOTBAR_WINDOW_WIDTH * scaling;
        let height = HOTBAR_WINDOW_HEIGHT * scaling;
        let margin = 18.0 * scaling;
        let position = ScreenPosition {
            left: ((window_size.width - width) / 2.0).max(0.0),
            top: (window_size.height - height - margin).max(0.0),
        };

        self.interface.open_window_at_auto_size(
            HotbarWindow::new(
                client_state().hotbar().skills(),
                client_state().skill_tree().skills(),
                client_state().hotbar(),
            ),
            position,
        );
    }

    /// Open (or re-open) the transparent status effect strip on the right edge
    /// of the screen, below the minimap.
    pub(crate) fn open_buff_bar_window(&mut self) {
        let window_size = *self.client_state.follow(client_state().window_size());
        let scaling = self.client_state.follow(client_state().interface_settings().scaling()).get_factor();
        // The minimap is a fixed 250 interface pixel tall window at the top
        // right, so the status strip starts below it. The offset is computed
        // from the current scale instead of using a cached window position.
        let minimap_height = 250.0 * scaling;
        let top = 12.0 + minimap_height + 8.0 * scaling;
        let strip_width = BAR_STRIP_WIDTH * scaling;
        let position = ScreenPosition {
            left: (window_size.width - strip_width - 12.0 * scaling).max(0.0),
            top,
        };

        self.interface.close_window_with_class(WindowClass::BuffBar);
        self.interface.open_window_at_auto_size(BuffBarWindow, position);
    }

    /// Open (or re-open) the transparent notification overlay at the top
    /// center of the screen.
    pub(crate) fn open_notification_hud_window(&mut self) {
        let enabled = *self
            .client_state
            .follow(client_state().interface_settings().show_notification_hud());

        if !enabled || self.client_state.try_follow(this_entity()).is_none() {
            self.interface.close_window_with_class(WindowClass::NotificationHud);
            return;
        }

        let window_size = *self.client_state.follow(client_state().window_size());
        let scaling = self.client_state.follow(client_state().interface_settings().scaling()).get_factor();
        let width = NOTIFICATION_HUD_WIDTH * scaling;
        let position = ScreenPosition {
            left: ((window_size.width - width) / 2.0).max(0.0),
            top: 12.0 * scaling,
        };

        self.interface.close_window_with_class(WindowClass::NotificationHud);
        self.interface.open_window_at_auto_size(NotificationHudWindow, position);
    }

    /// Reposition the HUD overlay windows after the window size or the
    /// interface scale changed.
    pub(crate) fn reposition_hud_windows(&mut self) {
        let in_game = self.client_state.try_follow(this_entity()).is_some();

        if in_game {
            self.open_hotbar_window();
            self.open_notification_hud_window();
        }

        if self.interface.is_window_with_class_open(WindowClass::BuffBar) {
            self.interface.close_window_with_class(WindowClass::BuffBar);
        }
    }

    pub(crate) fn update_window_size(&mut self, screen_size: ScreenSize) {
        crate::logging::log_debug!("interface: window size changed to {}x{}", screen_size.width, screen_size.height,);
        *self.client_state.follow_mut(client_state().window_size()) = screen_size;
        self.interface.invalidate_layouts();
        self.graphics_engine.on_resize(screen_size);
        self.interface.update_window_size(screen_size);
        self.interface_renderer.update_window_size(screen_size);
        self.bottom_interface_renderer.update_window_size(screen_size);
        self.middle_interface_renderer.update_window_size(screen_size);
        self.top_interface_renderer.update_window_size(screen_size);
        self.effect_renderer.update_window_size(screen_size);
        self.reposition_hud_windows();
    }

    /// Open or close the achievement window.
    pub(crate) fn toggle_achievement_window(&mut self) {
        if self.interface.is_window_with_class_open(WindowClass::Achievement) {
            self.interface.close_window_with_class(WindowClass::Achievement);
        } else {
            self.interface.open_window(AchievementWindow);
        }
    }

    /// Open or close the navigation destination search window.
    pub(crate) fn toggle_navigation_window(&mut self) {
        if self.interface.is_window_with_class_open(WindowClass::Navigation) {
            self.interface.close_window_with_class(WindowClass::Navigation);
        } else {
            self.interface.open_window(NavigationWindow::new(self.library.navigation()));
        }
    }

    /// Open or close the monster database window.
    pub(crate) fn toggle_monster_database_window(&mut self) {
        if self.interface.is_window_with_class_open(WindowClass::MonsterDatabase) {
            self.interface.close_window_with_class(WindowClass::MonsterDatabase);
        } else {
            self.interface
                .open_window(MonsterDatabaseWindow::new(self.library.monster_database()));
        }
    }

    /// Open (or focus) the whisper conversation window with `partner`.
    pub(crate) fn open_whisper_window(&mut self, partner: &str) {
        self.client_state.follow_mut(client_state().whisper()).open(partner);

        if !self.interface.is_window_with_class_open(WindowClass::Whisper) {
            self.interface.open_window(WhisperWindow::new(client_state().whisper()));
        }
    }
}
