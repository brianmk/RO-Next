//! `InputEvent` handlers for the party family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::TogglePartyWindow`.
    pub(crate) fn handle_input_toggle_party_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TogglePartyWindow = event else {
            unreachable!("handle_input_toggle_party_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Party) {
                true => self.interface.close_window_with_class(WindowClass::Party),
                false => self.interface.open_window(PartyWindow),
            }
        }
    }

    /// `InputEvent::CreateParty`.
    pub(crate) fn handle_input_create_party(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CreateParty { name } = event else {
            unreachable!("handle_input_create_party called with a different event variant");
        };

        self.client_state.follow_mut(client_state().party().create_name_input()).clear();
        let _ = self.networking_system.create_party(name);
    }

    /// `InputEvent::InviteToParty`.
    pub(crate) fn handle_input_invite_to_party(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::InviteToParty { name } = event else {
            unreachable!("handle_input_invite_to_party called with a different event variant");
        };

        self.client_state.follow_mut(client_state().party().invite_name_input()).clear();
        let _ = self.networking_system.invite_to_party(name);
    }

    /// `InputEvent::AcceptPartyInvite`.
    pub(crate) fn handle_input_accept_party_invite(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::AcceptPartyInvite { party_id } = event else {
            unreachable!("handle_input_accept_party_invite called with a different event variant");
        };

        let _ = self.networking_system.reply_party_invite(party_id, true);

        let party = self.client_state.follow_mut(client_state().party());
        party.party_id = Some(party_id);
        party.invite = None;

        self.interface.close_window_with_class(WindowClass::PartyInvite);

        self.open_party_window();
    }

    /// `InputEvent::RejectPartyInvite`.
    pub(crate) fn handle_input_reject_party_invite(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RejectPartyInvite { party_id } = event else {
            unreachable!("handle_input_reject_party_invite called with a different event variant");
        };

        let _ = self.networking_system.reply_party_invite(party_id, false);
        *self.client_state.follow_mut(client_state().party().invite()) = None;
        self.interface.close_window_with_class(WindowClass::PartyInvite);
    }

    /// `InputEvent::LeaveParty`.
    pub(crate) fn handle_input_leave_party(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LeaveParty = event else {
            unreachable!("handle_input_leave_party called with a different event variant");
        };

        let _ = self.networking_system.leave_party();

        self.client_state.follow_mut(client_state().party()).clear();
        self.interface.close_window_with_class(WindowClass::Party);
    }

    /// `InputEvent::SetPartyBookingVisible`.
    pub(crate) fn handle_input_set_party_booking_visible(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::SetPartyBookingVisible { visible } = event else {
            unreachable!("handle_input_set_party_booking_visible called with a different event variant");
        };

        {
            let booking = self.client_state.follow_mut(client_state().party_booking());
            booking.visible = visible;

            if visible && booking.results.is_empty() {
                booking.status_message = "Searching the party board...".to_owned();
            }
        }

        // Opening the section runs an initial unfiltered search so the
        // board is visible without an extra click. A server without
        // party booking simply never answers.
        if visible && self.client_state.follow(client_state().party_booking()).results.is_empty() {
            let booking = self.client_state.follow(client_state().party_booking());
            let level = booking.search_level();
            let map_id = booking.search_map_id();
            let job = booking.search_job();
            let result_count = crate::state::party_booking::MAXIMUM_SEARCH_RESULTS;
            let _ = self.networking_system.search_party_booking(level, map_id, job, 0, result_count);
        }
    }

    /// `InputEvent::RegisterPartyBooking`.
    pub(crate) fn handle_input_register_party_booking(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RegisterPartyBooking = event else {
            unreachable!("handle_input_register_party_booking called with a different event variant");
        };

        let booking = self.client_state.follow(client_state().party_booking());
        let level = booking.register_level();
        let map_id = booking.register_map_id();
        let jobs = booking.parse_register_jobs();
        let _ = self.networking_system.register_party_booking(level, map_id, jobs);
    }

    /// `InputEvent::SearchPartyBooking`.
    pub(crate) fn handle_input_search_party_booking(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SearchPartyBooking { next_page } = event else {
            unreachable!("handle_input_search_party_booking called with a different event variant");
        };

        let booking = self.client_state.follow(client_state().party_booking());
        let level = booking.search_level();
        let map_id = booking.search_map_id();
        let job = booking.search_job();
        let last_index = match next_page {
            true => booking.last_index,
            false => 0,
        };
        let result_count = crate::state::party_booking::MAXIMUM_SEARCH_RESULTS;
        let _ = self
            .networking_system
            .search_party_booking(level, map_id, job, last_index, result_count);
    }

    /// `InputEvent::DeletePartyBooking`.
    pub(crate) fn handle_input_delete_party_booking(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::DeletePartyBooking = event else {
            unreachable!("handle_input_delete_party_booking called with a different event variant");
        };

        let _ = self.networking_system.delete_party_booking();
    }

    /// `InputEvent::UpdatePartyBooking`.
    pub(crate) fn handle_input_update_party_booking(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::UpdatePartyBooking = event else {
            unreachable!("handle_input_update_party_booking called with a different event variant");
        };

        let jobs = self.client_state.follow(client_state().party_booking()).parse_register_jobs();
        let _ = self.networking_system.update_party_booking(jobs);
    }

    /// `InputEvent::JoinPartyBooking`.
    pub(crate) fn handle_input_join_party_booking(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::JoinPartyBooking { index } = event else {
            unreachable!("handle_input_join_party_booking called with a different event variant");
        };

        let booking = self.client_state.follow(client_state().party_booking());

        if !booking.join_requests_enabled {
            self.push_chat_information(
                "The booking join request is disabled: the CondaRO 20190605 server does not parse 0x0AE6. Enable it only on a server with \
                 PACKETVER >= 20191204."
                    .to_owned(),
            );
            return;
        }

        let Some(entry) = booking.result(index) else {
            return;
        };
        let leader_name = entry.character_name.clone();

        // The 2019 search result has no account/character id. The
        // join request can only be sent while the leader is a visible
        // player entity (their appear packet carries both ids).
        let leader = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| {
                entity.get_entity_type() == EntityType::Player
                    && entity
                        .get_details()
                        .is_some_and(|details| details.split('#').next() == Some(leader_name.as_str()))
            })
            .map(|entity| (entity.get_character_id(), ragnarok_packets::AccountId(entity.get_entity_id().0)));

        match leader {
            Some((character_id, account_id)) => {
                let _ = self.networking_system.request_party_booking_join(character_id, account_id);
            }
            None => self.push_chat_information(format!(
                "{leader_name} is not visible on this map; the 2019 booking search has no character id, so the join request cannot be \
                 sent."
            )),
        }
    }

    /// `InputEvent::ReplyPartyBookingJoin`.
    pub(crate) fn handle_input_reply_party_booking_join(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ReplyPartyBookingJoin {
            character_id,
            account_id,
            accept,
        } = event
        else {
            unreachable!("handle_input_reply_party_booking_join called with a different event variant");
        };

        let _ = self.networking_system.reply_party_booking_join(character_id, account_id, accept);
        self.client_state
            .follow_mut(client_state().party_booking())
            .remove_join_request(character_id);
    }

    /// `InputEvent::ChangePartyOption`.
    pub(crate) fn handle_input_change_party_option(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ChangePartyOption {
            experience_option,
            item_pickup,
            item_share,
        } = event
        else {
            unreachable!("handle_input_change_party_option called with a different event variant");
        };

        let _ = self
            .networking_system
            .change_party_option(experience_option, item_pickup, item_share);

        let party = self.client_state.follow_mut(client_state().party());
        party.experience_option = experience_option;
        party.item_pickup = item_pickup;
        party.item_share = item_share;
    }

    /// `InputEvent::ChangePartyLeader`.
    pub(crate) fn handle_input_change_party_leader(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ChangePartyLeader { name } = event else {
            unreachable!("handle_input_change_party_leader called with a different event variant");
        };

        let account_id = self.client_state.follow(client_state().party()).member_account_id(&name);

        match account_id {
            Some(account_id) => {
                let _ = self.networking_system.change_party_leader(account_id);
            }
            None => self.push_chat_information(format!("No party member named {name}.")),
        }
    }
}
