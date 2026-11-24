//! `InputEvent` handlers for the map family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ZoomCamera`.
    pub(crate) fn handle_input_zoom_camera(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ZoomCamera { zoom_factor } = event else {
            unreachable!("handle_input_zoom_camera called with a different event variant");
        };

        self.player_camera.soft_zoom(zoom_factor)
    }

    /// `InputEvent::RotateCamera`.
    pub(crate) fn handle_input_rotate_camera(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RotateCamera { rotation } = event else {
            unreachable!("handle_input_rotate_camera called with a different event variant");
        };

        self.player_camera.soft_rotate(rotation)
    }

    /// `InputEvent::ResetCameraRotation`.
    pub(crate) fn handle_input_reset_camera_rotation(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ResetCameraRotation = event else {
            unreachable!("handle_input_reset_camera_rotation called with a different event variant");
        };

        self.player_camera.reset_rotation()
    }

    /// `InputEvent::ToggleMinimapWindow`.
    pub(crate) fn handle_input_toggle_minimap_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleMinimapWindow = event else {
            unreachable!("handle_input_toggle_minimap_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            self.interface.close_window_with_class(WindowClass::BigMap);

            match self.interface.is_window_with_class_open(WindowClass::Minimap) {
                true => {
                    self.interface.close_window_with_class(WindowClass::Minimap);
                    self.map_window_cycle = MapWindowCycle::Closed;
                }
                false => {
                    self.interface.open_window(MinimapWindow);
                    self.map_window_cycle = MapWindowCycle::Minimap;
                }
            }
        }
    }

    /// `InputEvent::CycleMapWindows`.
    pub(crate) fn handle_input_cycle_map_windows(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CycleMapWindows = event else {
            unreachable!("handle_input_cycle_map_windows called with a different event variant");
        };

        // Text inputs (chat, login, character name, ...) get the M
        // key, the map cycle must not steal it while they are focused.
        if !self.interface.has_focus() && self.client_state.try_follow(this_entity()).is_some() {
            Self::cycle_map_windows(&mut self.interface, &mut self.map_window_cycle);
        }
    }

    /// `InputEvent::ToggleBigMapWindow`.
    pub(crate) fn handle_input_toggle_big_map_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleBigMapWindow = event else {
            unreachable!("handle_input_toggle_big_map_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            self.interface.close_window_with_class(WindowClass::Minimap);

            match self.interface.is_window_with_class_open(WindowClass::BigMap) {
                true => {
                    self.interface.close_window_with_class(WindowClass::BigMap);
                    self.map_window_cycle = MapWindowCycle::Closed;
                }
                false => {
                    self.interface.open_window(BigMapWindow);
                    self.map_window_cycle = MapWindowCycle::BigMap;
                }
            }
        }
    }

    /// `InputEvent::ToggleAchievementWindow`.
    pub(crate) fn handle_input_toggle_achievement_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleAchievementWindow = event else {
            unreachable!("handle_input_toggle_achievement_window called with a different event variant");
        };

        self.toggle_achievement_window()
    }

    /// `InputEvent::ToggleNavigationWindow`.
    pub(crate) fn handle_input_toggle_navigation_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleNavigationWindow = event else {
            unreachable!("handle_input_toggle_navigation_window called with a different event variant");
        };

        self.toggle_navigation_window()
    }

    /// `InputEvent::ToggleMonsterDatabaseWindow`.
    pub(crate) fn handle_input_toggle_monster_database_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleMonsterDatabaseWindow = event else {
            unreachable!("handle_input_toggle_monster_database_window called with a different event variant");
        };

        self.toggle_monster_database_window()
    }

    /// `InputEvent::ClaimAchievementReward`.
    pub(crate) fn handle_input_claim_achievement_reward(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ClaimAchievementReward { achievement_id } = event else {
            unreachable!("handle_input_claim_achievement_reward called with a different event variant");
        };

        let _ = self.networking_system.request_achievement_reward(achievement_id);
        self.push_chat_information(format!("Achievement {achievement_id}: reward requested."));
    }

    /// `InputEvent::ClearNavigationDestination`.
    pub(crate) fn handle_input_clear_navigation_destination(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ClearNavigationDestination = event else {
            unreachable!("handle_input_clear_navigation_destination called with a different event variant");
        };

        self.clear_navigation();
        self.push_chat_information("Navigation destination cleared.".to_owned());
    }

    /// `InputEvent::SelectMonster`.
    pub(crate) fn handle_input_select_monster(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SelectMonster { job_id } = event else {
            unreachable!("handle_input_select_monster called with a different event variant");
        };

        self.client_state.follow_mut(client_state().monster_database()).set_selected(job_id);
    }

    /// `InputEvent::SelectWarpPoint`.
    pub(crate) fn handle_input_select_warp_point(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SelectWarpPoint { option } = event else {
            unreachable!("handle_input_select_warp_point called with a different event variant");
        };

        let destination = self
            .client_state
            .follow(client_state().dialog_window())
            .warp_point_destination(option);

        if let Some((skill_id, map_name)) = destination {
            let _ = self.networking_system.select_warp_point(skill_id, map_name);
        }

        // The official client closes the destination list as soon as the
        // option was picked; the server creates the portal units after
        // answering with `CZ_SELECT_WARPPOINT`.
        let dialog = self.client_state.follow_mut(client_state().dialog_window());
        dialog.clear_warp_point();
        dialog.clear_choices();
        self.interface.close_window_with_class(WindowClass::Dialog);
    }
}
