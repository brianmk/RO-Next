//! `InputEvent` handlers for the windows family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ToggleMenuWindow`.
    pub(crate) fn handle_input_toggle_menu_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleMenuWindow = event else {
            unreachable!("handle_input_toggle_menu_window called with a different event variant");
        };

        // Escape closes the item info window and the character context
        // menu before it does anything else.
        if self.interface.is_window_with_class_open(WindowClass::SkillDescription) {
            self.interface.close_window_with_class(WindowClass::SkillDescription);
        } else if self.interface.is_window_with_class_open(WindowClass::ItemInfo) {
            self.interface.close_window_with_class(WindowClass::ItemInfo);
        } else if self.interface.is_window_with_class_open(WindowClass::ContextMenu) {
            self.interface.close_window_with_class(WindowClass::ContextMenu);
        // Escape cancels an active skill target selection instead
        // of opening the menu, like in the
        // original client.
        } else if self.interface.get_mouse_mode().skill_target().is_some() {
            self.interface.set_mouse_mode(MouseMode::Default);
        } else if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Menu) {
                true => self.interface.close_window_with_class(WindowClass::Menu),
                false => self.interface.open_window(MenuWindow),
            }
        }
    }

    /// `InputEvent::ToggleStatsWindow`.
    pub(crate) fn handle_input_toggle_stats_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleStatsWindow = event else {
            unreachable!("handle_input_toggle_stats_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Stats) {
                true => self.interface.close_window_with_class(WindowClass::Stats),
                false => self.interface.open_window(StatsWindow::new(this_player().manually_asserted())),
            }
        }
    }

    /// `InputEvent::ToggleGameSettingsWindow`.
    pub(crate) fn handle_input_toggle_game_settings_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleGameSettingsWindow = event else {
            unreachable!("handle_input_toggle_game_settings_window called with a different event variant");
        };

        match self.interface.is_window_with_class_open(WindowClass::GameSettings) {
            true => self.interface.close_window_with_class(WindowClass::GameSettings),
            false => self.interface.open_window(GameSettingsWindow::new(client_state().game_settings())),
        }
    }

    /// `InputEvent::ToggleInterfaceSettingsWindow`.
    pub(crate) fn handle_input_toggle_interface_settings_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleInterfaceSettingsWindow = event else {
            unreachable!("handle_input_toggle_interface_settings_window called with a different event variant");
        };

        match self.interface.is_window_with_class_open(WindowClass::InterfaceSettings) {
            true => {
                crate::logging::log_info!("interface: closing interface settings window");
                self.interface.close_window_with_class(WindowClass::InterfaceSettings);
            }
            false => {
                let interface_settings = self.client_state.follow(client_state().interface_settings()).clone();
                crate::logging::log_info!(
                    "interface: opening interface settings window (in_game_theme={:?}, scaling={})",
                    interface_settings.in_game_theme,
                    interface_settings.scaling.get_factor(),
                );
                self.interface.open_window(InterfaceSettingsWindow::new(
                    client_state().interface_settings(),
                    client_state().interface_settings_capabilities(),
                ));
            }
        }
    }

    /// `InputEvent::ToggleGraphicsSettingsWindow`.
    pub(crate) fn handle_input_toggle_graphics_settings_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleGraphicsSettingsWindow = event else {
            unreachable!("handle_input_toggle_graphics_settings_window called with a different event variant");
        };

        match self.interface.is_window_with_class_open(WindowClass::GraphicsSettings) {
            true => self.interface.close_window_with_class(WindowClass::GraphicsSettings),
            false => self.interface.open_window(GraphicsSettingsWindow::new(
                client_state().graphics_settings(),
                client_state().graphics_settings_capabilities(),
            )),
        }
    }

    /// `InputEvent::ToggleAudioSettingsWindow`.
    pub(crate) fn handle_input_toggle_audio_settings_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleAudioSettingsWindow = event else {
            unreachable!("handle_input_toggle_audio_settings_window called with a different event variant");
        };

        match self.interface.is_window_with_class_open(WindowClass::AudioSettings) {
            true => {
                // The volumes are saved on change already; save again on close so
                // the file is up to date even if the last change was not picked up
                // by `update_settings` yet.
                self.client_state.follow(client_state().audio_settings()).save();
                self.interface.close_window_with_class(WindowClass::AudioSettings);
            }
            false => self
                .interface
                .open_window(AudioSettingsWindow::new(client_state().audio_settings())),
        }
    }

    /// `InputEvent::ToggleEmotePickerWindow`.
    pub(crate) fn handle_input_toggle_emote_picker_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleEmotePickerWindow = event else {
            unreachable!("handle_input_toggle_emote_picker_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::EmotePicker) {
                true => self.interface.close_window_with_class(WindowClass::EmotePicker),
                false => self.interface.open_window(EmotePickerWindow),
            }
        }
    }

    /// `InputEvent::ToggleQuestWindow`.
    pub(crate) fn handle_input_toggle_quest_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleQuestWindow = event else {
            unreachable!("handle_input_toggle_quest_window called with a different event variant");
        };

        if self.interface.is_window_with_class_open(WindowClass::Quest) {
            self.interface.close_window_with_class(WindowClass::Quest);
        } else {
            self.interface.open_window(QuestWindow::new(client_state().quest_log()));
        }
    }

    /// `InputEvent::ToggleBankWindow`.
    pub(crate) fn handle_input_toggle_bank_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleBankWindow = event else {
            unreachable!("handle_input_toggle_bank_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Bank) {
                true => self.close_bank_window(),
                false => self.open_bank_window(),
            }
        }
    }
}
