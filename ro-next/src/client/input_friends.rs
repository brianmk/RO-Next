//! `InputEvent` handlers for the friends family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ToggleFriendListWindow`.
    pub(crate) fn handle_input_toggle_friend_list_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleFriendListWindow = event else {
            unreachable!("handle_input_toggle_friend_list_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::FriendList) {
                true => self.interface.close_window_with_class(WindowClass::FriendList),
                false => self.interface.open_window(FriendListWindow::new(
                    client_state().friend_list_window(),
                    client_state().friend_list(),
                )),
            }
        }
    }

    /// `InputEvent::AddFriend`.
    pub(crate) fn handle_input_add_friend(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::AddFriend { character_name } = event else {
            unreachable!("handle_input_add_friend called with a different event variant");
        };

        if character_name.len() > ragnarok_packets::NAME_LENGTH {
            #[cfg(feature = "debug")]
            print_debug!("[{}] friend name {} is too long", "error".red(), character_name.magenta());
        } else {
            let _ = self.networking_system.add_friend(character_name);
        }
    }

    /// `InputEvent::RemoveFriend`.
    pub(crate) fn handle_input_remove_friend(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RemoveFriend { account_id, character_id } = event else {
            unreachable!("handle_input_remove_friend called with a different event variant");
        };

        let _ = self.networking_system.remove_friend(account_id, character_id);
    }

    /// `InputEvent::RejectFriendRequest`.
    pub(crate) fn handle_input_reject_friend_request(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RejectFriendRequest { account_id, character_id } = event else {
            unreachable!("handle_input_reject_friend_request called with a different event variant");
        };

        let _ = self.networking_system.reject_friend_request(account_id, character_id);
        self.interface.close_window_with_class(WindowClass::FriendRequest);
    }

    /// `InputEvent::AcceptFriendRequest`.
    pub(crate) fn handle_input_accept_friend_request(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::AcceptFriendRequest { account_id, character_id } = event else {
            unreachable!("handle_input_accept_friend_request called with a different event variant");
        };

        let _ = self.networking_system.accept_friend_request(account_id, character_id);
        self.interface.close_window_with_class(WindowClass::FriendRequest);
    }
}
