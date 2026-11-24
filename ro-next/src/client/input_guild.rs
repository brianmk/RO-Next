//! `InputEvent` handlers for the guild family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ToggleClanWindow`.
    pub(crate) fn handle_input_toggle_clan_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleClanWindow = event else {
            unreachable!("handle_input_toggle_clan_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Clan) {
                true => self.interface.close_window_with_class(WindowClass::Clan),
                false => self.interface.open_window(ClanWindow),
            }
        }
    }

    /// `InputEvent::ToggleGuildWindow`.
    pub(crate) fn handle_input_toggle_guild_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleGuildWindow = event else {
            unreachable!("handle_input_toggle_guild_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Guild) {
                true => self.interface.close_window_with_class(WindowClass::Guild),
                false => {
                    self.interface.open_window(GuildWindow);

                    // Opening the window asks the server for the menu
                    // mask and the three tabs: basic info (0), member
                    // list (1) and positions (2). rAthena ignores the
                    // request when the player is not in a guild.
                    let _ = self.networking_system.request_guild_menu_interface();

                    for menu in [0, 1, 2] {
                        let _ = self.networking_system.request_guild_info(menu);
                    }
                }
            }
        }
    }

    /// `InputEvent::InviteToGuild`.
    pub(crate) fn handle_input_invite_to_guild(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::InviteToGuild { name } = event else {
            unreachable!("handle_input_invite_to_guild called with a different event variant");
        };

        let can_invite = self.client_state.follow(client_state().guild()).can_invite();

        if can_invite {
            let _ = self.networking_system.invite_to_guild(name.clone());
            self.client_state.follow_mut(client_state().guild()).status_message = format!("Invited {name}.");
        } else {
            self.client_state.follow_mut(client_state().guild()).status_message =
                "You do not have permission to invite members.".to_owned();
        }
    }

    /// `InputEvent::ExpelGuildMember`.
    pub(crate) fn handle_input_expel_guild_member(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ExpelGuildMember { name } = event else {
            unreachable!("handle_input_expel_guild_member called with a different event variant");
        };

        let outcome = {
            let guild = self.client_state.follow(client_state().guild());

            if !guild.can_expel() {
                Err("You do not have permission to expel members.".to_owned())
            } else {
                match guild.member_by_name(&name) {
                    Some(member) if Some(member.character_id) == guild.local_character_id => Err("You cannot expel yourself.".to_owned()),
                    Some(member) => Ok((guild.guild_id, member.account_id, member.character_id)),
                    None => Err(format!("No guild member named {name}.")),
                }
            }
        };

        match outcome {
            Ok((guild_id, account_id, character_id)) => {
                let _ = self
                    .networking_system
                    .expel_guild_member(guild_id, account_id, character_id, String::new());
                self.client_state.follow_mut(client_state().guild()).status_message = format!("Expelling {name}...");
            }
            Err(message) => {
                self.client_state.follow_mut(client_state().guild()).status_message = message;
            }
        }
    }

    /// `InputEvent::LeaveGuild`.
    pub(crate) fn handle_input_leave_guild(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LeaveGuild = event else {
            unreachable!("handle_input_leave_guild called with a different event variant");
        };

        let request = {
            let guild = self.client_state.follow(client_state().guild());

            match (guild.is_in_guild(), guild.local_account_id, guild.local_character_id) {
                (true, Some(account_id), Some(character_id)) => Ok((guild.guild_id, account_id, character_id)),
                _ => Err("You are not in a guild.".to_owned()),
            }
        };

        match request {
            Ok((guild_id, account_id, character_id)) => {
                let _ = self.networking_system.leave_guild(guild_id, account_id, character_id);
                self.client_state.follow_mut(client_state().guild()).status_message = "Leaving guild...".to_owned();
            }
            Err(message) => {
                self.client_state.follow_mut(client_state().guild()).status_message = message;
            }
        }
    }

    /// `InputEvent::SaveGuildNotice`.
    pub(crate) fn handle_input_save_guild_notice(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SaveGuildNotice { subject, notice } = event else {
            unreachable!("handle_input_save_guild_notice called with a different event variant");
        };

        let guild_id = {
            let guild = self.client_state.follow(client_state().guild());

            if !guild.can_edit_notice() { None } else { Some(guild.guild_id) }
        };

        match guild_id {
            Some(guild_id) => {
                let _ = self.networking_system.change_guild_notice(guild_id, subject, notice);
                self.client_state.follow_mut(client_state().guild()).status_message = "Notice saved.".to_owned();
            }
            None => {
                self.client_state.follow_mut(client_state().guild()).status_message =
                    "Only the guild master can change the notice.".to_owned();
            }
        }
    }
}
