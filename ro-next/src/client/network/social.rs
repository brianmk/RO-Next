//! `NetworkEvent` handlers for the social family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::WhisperReceived`.
    pub(crate) fn handle_whisper_received(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::WhisperReceived {
            sender_name,
            message,
            is_admin,
        } = event
        else {
            unreachable!("handle_whisper_received called with a different event variant");
        };

        // A whisper belongs to its own conversation and to the
        // whisper chat tab, never to the public tab.
        self.client_state
            .follow_mut(client_state().whisper())
            .push_incoming(&sender_name, message.clone());

        let prefix = match is_admin {
            true => "[Whisper][GM]",
            false => "[Whisper]",
        };

        self.push_chat_message(ChatMessage::whisper(
            format!("{prefix} {sender_name} : {message}"),
            MessageColor::Information,
            sender_name,
        ));
    }

    /// `NetworkEvent::GuildChat`.
    pub(crate) fn handle_guild_chat(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildChat { message } = event else {
            unreachable!("handle_guild_chat called with a different event variant");
        };

        // `ZC_GUILD_CHAT` carries the full `<name> : <message>`
        // line (the same format party chat uses).
        let message = message.trim_end_matches('\0').trim();

        if !message.is_empty() {
            self.push_chat_message(ChatMessage::new_channel(
                format!("[Guild] {message}"),
                MessageColor::Rgb {
                    red: 150,
                    green: 210,
                    blue: 255,
                },
                ChatChannel::Guild,
            ));
        }
    }

    /// `NetworkEvent::ChatRoomCreated`.
    pub(crate) fn handle_chat_room_created(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomCreated { flag } = event else {
            unreachable!("handle_chat_room_created called with a different event variant");
        };

        match flag {
            0 => {
                let player_name = self.client_state.follow(client_state().player_name()).clone();
                self.client_state
                    .follow_mut(client_state().content_windows().chat_room())
                    .created(&player_name);
                self.interface.close_window_with_class(WindowClass::ChatRoom);
                self.interface
                    .open_window(ChatRoomWindow::new(client_state().content_windows().chat_room()));
            }
            1 => self
                .client_state
                .follow_mut(client_state().content_windows().chat_room())
                .set_error("The chat room limit was exceeded.".to_owned()),
            _ => self
                .client_state
                .follow_mut(client_state().content_windows().chat_room())
                .set_error("A chat room with the same name already exists.".to_owned()),
        }
    }

    /// `NetworkEvent::ChatRoomAnnounced`.
    pub(crate) fn handle_chat_room_announced(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomAnnounced {
            owner_id,
            chat_id,
            limit,
            users,
            room_type,
            title,
        } = event
        else {
            unreachable!("handle_chat_room_announced called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .announced(crate::state::content_windows::ChatRoomSummary {
                owner_id,
                chat_id,
                title,
                users,
                limit,
                room_type,
            });
        self.interface.invalidate_layouts();
    }

    /// `NetworkEvent::ChatRoomDestroyed`.
    pub(crate) fn handle_chat_room_destroyed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomDestroyed { chat_id } = event else {
            unreachable!("handle_chat_room_destroyed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .destroyed(chat_id);
        self.interface.invalidate_layouts();
    }

    /// `NetworkEvent::ChatRoomEnterRefused`.
    pub(crate) fn handle_chat_room_enter_refused(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomEnterRefused { result } = event else {
            unreachable!("handle_chat_room_enter_refused called with a different event variant");
        };

        let message = match result {
            0 => "The chat room is full.",
            1 => "The chat room password is wrong.",
            2 => "You were kicked from that chat room.",
            4 => "You do not have enough zeny for that chat room.",
            5 => "Your level is too low for that chat room.",
            6 => "Your level is too high for that chat room.",
            7 => "Your job class cannot enter that chat room.",
            _ => "The server refused to enter the chat room.",
        };
        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .set_error(message.to_owned());
        self.interface.invalidate_layouts();
    }

    /// `NetworkEvent::ChatRoomEntered`.
    pub(crate) fn handle_chat_room_entered(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomEntered { chat_id, members } = event else {
            unreachable!("handle_chat_room_entered called with a different event variant");
        };

        let members: Vec<(String, bool)> = members.iter().map(|member| (member.name.clone(), member.role == 0)).collect();
        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .entered(chat_id, &members);
        self.interface.close_window_with_class(WindowClass::ChatRoom);
        self.interface
            .open_window(ChatRoomWindow::new(client_state().content_windows().chat_room()));
    }

    /// `NetworkEvent::ChatRoomMemberJoined`.
    pub(crate) fn handle_chat_room_member_joined(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomMemberJoined { name, .. } = event else {
            unreachable!("handle_chat_room_member_joined called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .member_joined(&name);
        self.interface.invalidate_layouts();
    }

    /// `NetworkEvent::ChatRoomMemberLeft`.
    pub(crate) fn handle_chat_room_member_left(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomMemberLeft { name, kicked, .. } = event else {
            unreachable!("handle_chat_room_member_left called with a different event variant");
        };

        let local_player_name = self.client_state.follow(client_state().player_name()).clone();
        let local_left = self
            .client_state
            .follow_mut(client_state().content_windows().chat_room())
            .member_left(&name, kicked, &local_player_name);

        if local_left {
            self.interface.close_window_with_class(WindowClass::ChatRoom);
        } else {
            self.interface.invalidate_layouts();
        }
    }

    /// `NetworkEvent::ChatRoomChanged`.
    pub(crate) fn handle_chat_room_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomChanged {
            owner_id,
            chat_id,
            limit,
            users,
            room_type,
            title,
        } = event
        else {
            unreachable!("handle_chat_room_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .changed(chat_id, title, limit, room_type, owner_id, users);
        self.interface.invalidate_layouts();
    }

    /// `NetworkEvent::ChatRoomRoleChanged`.
    pub(crate) fn handle_chat_room_role_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatRoomRoleChanged { flag, name } = event else {
            unreachable!("handle_chat_room_role_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .role_changed(flag, &name);
        self.interface.invalidate_layouts();
    }

    /// `NetworkEvent::GuildEmblemImage`.
    pub(crate) fn handle_guild_emblem_image(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildEmblemImage {
            guild_id,
            emblem_id,
            emblem_data,
        } = event
        else {
            unreachable!("handle_guild_emblem_image called with a different event variant");
        };

        match decode_emblem(&emblem_data) {
            Ok(image) => {
                let name = format!("guild_emblem_{guild_id}_{emblem_id}");
                let texture = self.texture_loader.create_color(&name, image, true);
                self.guild_emblems.insert(guild_id, GuildEmblem { texture });
            }
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!("Failed to decode guild emblem of guild {guild_id} (emblem id {emblem_id}): {_error}");
            }
        }
    }

    /// `NetworkEvent::GuildMenuInterface`.
    pub(crate) fn handle_guild_menu_interface(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildMenuInterface { menu_flags } = event else {
            unreachable!("handle_guild_menu_interface called with a different event variant");
        };

        self.client_state.follow_mut(client_state().guild()).menu_flags = menu_flags;
    }

    /// `NetworkEvent::GuildInfo`.
    pub(crate) fn handle_guild_info(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildInfo { information } = event else {
            unreachable!("handle_guild_info called with a different event variant");
        };

        let (guild_id, guild_name) = {
            let guild = self.client_state.follow_mut(client_state().guild());
            guild.apply_info(&information);
            (guild.guild_id, guild.name.clone())
        };

        if guild_id != 0 {
            if !guild_name.trim().is_empty() {
                // The overhead guild label is otherwise only filled from
                // `ZC_ACK_REQNAMEALL` (hover/context menu). The guild window
                // just learned the local guild name, so apply it to every
                // nearby member of the same guild.
                for entity in self.client_state.follow_mut(client_state().entities()) {
                    if entity.get_guild_id() == guild_id {
                        entity.set_guild_name(Some(guild_name.clone()));
                    }
                }
            }

            // The emblem is downloaded on demand; the guild window is a
            // second place that can learn the guild id first.
            if self.guild_emblems.request_if_needed(guild_id) {
                let _ = self.networking_system.request_guild_emblem(guild_id);
            }
        }
    }

    /// `NetworkEvent::GuildMembers`.
    pub(crate) fn handle_guild_members(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildMembers { members } = event else {
            unreachable!("handle_guild_members called with a different event variant");
        };

        let mut name_requests = Vec::new();

        {
            let guild = self.client_state.follow_mut(client_state().guild());
            guild.apply_members(&members);

            // The 2019 member list has no name field. Ask for every
            // unknown name once per map session.
            let unknown: Vec<CharacterId> = guild
                .members
                .iter()
                .filter(|member| member.name.trim().is_empty() && member.character_id.0 != 0)
                .map(|member| member.character_id)
                .filter(|character_id| !guild.requested_names.contains(character_id))
                .collect();

            for character_id in unknown {
                guild.requested_names.push(character_id);
                name_requests.push(character_id);
            }
        }

        for character_id in name_requests {
            let _ = self.networking_system.request_guild_member_name(character_id);
        }
    }

    /// `NetworkEvent::GuildPositionNames`.
    pub(crate) fn handle_guild_position_names(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildPositionNames { positions } = event else {
            unreachable!("handle_guild_position_names called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().guild())
            .apply_position_names(&positions);
    }

    /// `NetworkEvent::GuildPositionRights`.
    pub(crate) fn handle_guild_position_rights(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildPositionRights { positions } = event else {
            unreachable!("handle_guild_position_rights called with a different event variant");
        };

        self.client_state.follow_mut(client_state().guild()).apply_position_info(&positions);
    }

    /// `NetworkEvent::GuildPositionChanged`.
    pub(crate) fn handle_guild_position_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildPositionChanged { positions } = event else {
            unreachable!("handle_guild_position_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().guild())
            .apply_position_changes(&positions);
    }

    /// `NetworkEvent::GuildMemberPositionChanged`.
    pub(crate) fn handle_guild_member_position_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildMemberPositionChanged { members } = event else {
            unreachable!("handle_guild_member_position_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().guild())
            .apply_member_position_changes(&members);
    }

    /// `NetworkEvent::GuildNotice`.
    pub(crate) fn handle_guild_notice(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildNotice { subject, notice } = event else {
            unreachable!("handle_guild_notice called with a different event variant");
        };

        self.client_state.follow_mut(client_state().guild()).apply_notice(subject, notice);
    }

    /// `NetworkEvent::GuildAlliances`.
    pub(crate) fn handle_guild_alliances(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildAlliances { alliances } = event else {
            unreachable!("handle_guild_alliances called with a different event variant");
        };

        self.client_state.follow_mut(client_state().guild()).apply_alliances(&alliances);
    }

    /// `NetworkEvent::GuildMemberName`.
    pub(crate) fn handle_guild_member_name(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildMemberName { character_id, name } = event else {
            unreachable!("handle_guild_member_name called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().guild())
            .apply_member_name(character_id, name);
    }

    /// `NetworkEvent::GuildMemberRemoved`.
    pub(crate) fn handle_guild_member_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildMemberRemoved { character_id, expelled } = event else {
            unreachable!("handle_guild_member_removed called with a different event variant");
        };

        let is_local = self
            .client_state
            .follow(client_state().guild())
            .local_character_id
            .is_some_and(|local_character_id| local_character_id == character_id);

        if is_local {
            self.client_state.follow_mut(client_state().guild()).leave_local_guild();
            self.interface.close_window_with_class(WindowClass::Guild);
        } else {
            let guild = self.client_state.follow_mut(client_state().guild());
            guild.remove_member(character_id);
            guild.status_message = match expelled {
                true => "A member was expelled from the guild.".to_owned(),
                false => "A member left the guild.".to_owned(),
            };
        }
    }

    /// `NetworkEvent::GuildInviteResult`.
    pub(crate) fn handle_guild_invite_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildInviteResult { result } = event else {
            unreachable!("handle_guild_invite_result called with a different event variant");
        };

        let guild = self.client_state.follow_mut(client_state().guild());
        guild.status_message = match result {
            0 => "The invitation failed: the player is already in a guild or busy.".to_owned(),
            1 => "The invitation failed: the player is offline.".to_owned(),
            3 => "The invitation failed: the guild is full.".to_owned(),
            result => format!("The invitation failed (result {result})."),
        };
    }

    /// `NetworkEvent::GuildCreateResult`.
    pub(crate) fn handle_guild_create_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildCreateResult { result } = event else {
            unreachable!("handle_guild_create_result called with a different event variant");
        };

        let message = match result {
            0 => "Guild created.",
            1 => "You are already in a guild.",
            2 => "That guild name is already taken.",
            3 => "You need one Emperium to create a guild.",
            _ => "Guild creation failed.",
        };

        self.push_chat_information(message.to_owned());
        self.client_state.follow_mut(client_state().guild()).status_message = message.to_owned();

        // A successful creation leaves the local player in a brand new guild.
        // Open the guild window when it is closed and ask for the same tabs the
        // Alt+G toggle requests, so the new guild shows up without a manual
        // refresh (the create button lives in the guild window, so this is
        // normally a refresh of the already open window).
        if result == 0 {
            if !self.interface.is_window_with_class_open(WindowClass::Guild) {
                self.interface.open_window(GuildWindow);
            }

            let _ = self.networking_system.request_guild_menu_interface();

            for menu in [0, 1, 2] {
                let _ = self.networking_system.request_guild_info(menu);
            }
        }
    }

    /// `NetworkEvent::GuildInviteRequest`.
    pub(crate) fn handle_guild_invite_request(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GuildInviteRequest { guild_id, guild_name } = event else {
            unreachable!("handle_guild_invite_request called with a different event variant");
        };

        let in_guild = self.client_state.follow(client_state().guild()).is_in_guild();

        if in_guild {
            let message = "You are already in a guild.";
            self.push_chat_information(message.to_owned());
            self.client_state.follow_mut(client_state().guild()).status_message = message.to_owned();
            return;
        }

        let guild_name = guild_name.trim().to_owned();

        self.client_state.follow_mut(client_state().guild()).pending_invite = Some(crate::state::guild::GuildInviteRequest {
            guild_id,
            guild_name: guild_name.clone(),
        });

        let message = format!("Guild invitation: {guild_name}. Open the Guild window (Alt+G) to accept.");
        self.push_chat_information(message.clone());
        self.client_state.follow_mut(client_state().guild()).status_message = message;
    }

    /// `NetworkEvent::FriendRequest`.
    pub(crate) fn handle_friend_request(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::FriendRequest { requestee } = event else {
            unreachable!("handle_friend_request called with a different event variant");
        };

        crate::state::friend_names::remember(requestee.character_id.0, &requestee.name);
        self.interface.open_window(FriendRequestWindow::new(requestee));
    }

    /// `NetworkEvent::PartyOption`.
    pub(crate) fn handle_party_option(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyOption {
            experience_option,
            item_pickup,
            item_share,
        } = event
        else {
            unreachable!("handle_party_option called with a different event variant");
        };

        let party = self.client_state.follow_mut(client_state().party());
        party.experience_option = experience_option;
        party.item_pickup = item_pickup != 0;
        party.item_share = item_share != 0;
    }

    /// `NetworkEvent::PartyList`.
    pub(crate) fn handle_party_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyList { party_name, members } = event else {
            unreachable!("handle_party_list called with a different event variant");
        };

        let was_in_party = self.client_state.follow(client_state().party()).is_in_party();

        let party = self.client_state.follow_mut(client_state().party());

        if members.is_empty() && !was_in_party {
            return;
        }

        party.apply_roster(party_name, &members);

        if members.is_empty() {
            party.clear();
            self.interface.close_window_with_class(WindowClass::Party);
        } else if !was_in_party {
            // The player just joined a party (login burst or accepted
            // invite): show the roster like the original client.
            self.open_party_window();
        }
    }

    /// `NetworkEvent::PartyMemberAdded`.
    pub(crate) fn handle_party_member_added(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyMemberAdded { member } = event else {
            unreachable!("handle_party_member_added called with a different event variant");
        };

        let was_in_party = self.client_state.follow(client_state().party()).is_in_party();

        self.client_state.follow_mut(client_state().party()).apply_member_details(&member);

        if !was_in_party {
            self.open_party_window();
        }
    }

    /// `NetworkEvent::PartyMemberRemoved`.
    pub(crate) fn handle_party_member_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyMemberRemoved {
            account_id,
            character_name,
            result,
        } = event
        else {
            unreachable!("handle_party_member_removed called with a different event variant");
        };

        let own_account_id = self.saved_login_data.as_ref().map(|login_data| login_data.account_id);
        let is_self = own_account_id == Some(account_id);

        self.client_state
            .follow_mut(client_state().party())
            .apply_member_removed(account_id);

        let message = format_message(MessageDomain::PartyMemberRemoved, u16::from(result), &[(
            "character_name",
            &character_name,
        )]);

        self.push_chat_information(message);

        if is_self {
            self.client_state.follow_mut(client_state().party()).clear();
            self.interface.close_window_with_class(WindowClass::Party);
        }
    }

    /// `NetworkEvent::PartyMemberHealth`.
    pub(crate) fn handle_party_member_health(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyMemberHealth {
            account_id,
            health_points,
            maximum_health_points,
        } = event
        else {
            unreachable!("handle_party_member_health called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().party())
            .apply_health(account_id, health_points, maximum_health_points);
    }

    /// `NetworkEvent::PartyMemberPosition`.
    pub(crate) fn handle_party_member_position(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyMemberPosition { account_id, position } = event else {
            unreachable!("handle_party_member_position called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().party())
            .apply_position(account_id, position);
    }

    /// `NetworkEvent::PartyMemberJobLevel`.
    pub(crate) fn handle_party_member_job_level(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyMemberJobLevel {
            account_id,
            job_id,
            base_level,
        } = event
        else {
            unreachable!("handle_party_member_job_level called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().party())
            .apply_job_level(account_id, job_id, base_level);
    }

    /// `NetworkEvent::PartyMemberAlive`.
    pub(crate) fn handle_party_member_alive(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyMemberAlive { account_id, is_dead } = event else {
            unreachable!("handle_party_member_alive called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().party())
            .apply_alive(account_id, is_dead);
    }

    /// `NetworkEvent::PartyInvite`.
    pub(crate) fn handle_party_invite(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyInvite { party_id, party_name } = event else {
            unreachable!("handle_party_invite called with a different event variant");
        };

        if self.client_state.follow(client_state().party()).is_in_party() {
            // Already in a party; the server will reject the invitation
            // anyway, but tell the user why nothing happens.
            self.push_localized_chat(client_state().localization().chat_party_invite_ignored(), &[(
                "name",
                party_name.as_str(),
            )]);
            return;
        }

        *self.client_state.follow_mut(client_state().party().invite()) =
            Some(crate::state::party::PartyInvitePrompt { party_id, party_name });

        self.interface.open_window(PartyInviteWindow::new(client_state().party().invite()));
    }

    /// `NetworkEvent::PartyInviteReply`.
    pub(crate) fn handle_party_invite_reply(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyInviteReply { character_name, result } = event else {
            unreachable!("handle_party_invite_reply called with a different event variant");
        };

        self.interface.close_window_with_class(WindowClass::PartyInvite);
        *self.client_state.follow_mut(client_state().party().invite()) = None;

        let message = format_message(MessageDomain::PartyInviteReply, result.clamp(0, i32::from(u16::MAX)) as u16, &[
            ("character_name", &character_name),
        ]);

        self.push_chat_information(message);
    }

    /// `NetworkEvent::PartyCreationResult`.
    pub(crate) fn handle_party_creation_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyCreationResult { result } = event else {
            unreachable!("handle_party_creation_result called with a different event variant");
        };

        let message = format_message(MessageDomain::PartyCreation, u16::from(result), &[]);

        self.push_chat_information(message);
    }

    /// `NetworkEvent::PartyChatMessage`.
    pub(crate) fn handle_party_chat_message(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyChatMessage { account_id, message } = event else {
            unreachable!("handle_party_chat_message called with a different event variant");
        };

        let name = self
            .client_state
            .follow(client_state().party())
            .member_name(account_id)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("Account #{}", account_id.0));

        // The server echoes the full "<name> : <message>" string in
        // `ZC_NOTIFY_CHAT_PARTY`, but the client already knows and
        // renders the name, so strip the redundant prefix.
        let message = message
            .strip_prefix(&format!("{name} : "))
            .unwrap_or(&message)
            .trim_end_matches('\0');

        self.push_chat_message(ChatMessage::new_channel(
            format!("[Party] {name}: {message}"),
            MessageColor::Rgb {
                red: 120,
                green: 230,
                blue: 140,
            },
            ChatChannel::Party,
        ));
    }

    /// `NetworkEvent::PartyConfig`.
    pub(crate) fn handle_party_config(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyConfig { allow_invites } = event else {
            unreachable!("handle_party_config called with a different event variant");
        };

        self.client_state.follow_mut(client_state().party()).allow_invites = allow_invites;
    }

    /// `NetworkEvent::PartyBookingRegisterResult`.
    pub(crate) fn handle_party_booking_register_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingRegisterResult { result } = event else {
            unreachable!("handle_party_booking_register_result called with a different event variant");
        };

        let status = {
            let booking = self.client_state.follow_mut(client_state().party_booking());
            booking.apply_register_result(result);
            booking.status_message.clone()
        };
        self.push_chat_information(status);
    }

    /// `NetworkEvent::PartyBookingSearchResult`.
    pub(crate) fn handle_party_booking_search_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingSearchResult { more_results, entries } = event else {
            unreachable!("handle_party_booking_search_result called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().party_booking())
            .apply_search(more_results, entries);
    }

    /// `NetworkEvent::PartyBookingDeleteResult`.
    pub(crate) fn handle_party_booking_delete_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingDeleteResult { result } = event else {
            unreachable!("handle_party_booking_delete_result called with a different event variant");
        };

        let status = {
            let booking = self.client_state.follow_mut(client_state().party_booking());
            booking.apply_delete_result(result);
            booking.status_message.clone()
        };
        self.push_chat_information(status);
    }

    /// `NetworkEvent::PartyBookingInserted`.
    pub(crate) fn handle_party_booking_inserted(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingInserted { entry } = event else {
            unreachable!("handle_party_booking_inserted called with a different event variant");
        };

        self.client_state.follow_mut(client_state().party_booking()).apply_inserted(entry);
    }

    /// `NetworkEvent::PartyBookingUpdated`.
    pub(crate) fn handle_party_booking_updated(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingUpdated { index, jobs } = event else {
            unreachable!("handle_party_booking_updated called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().party_booking())
            .apply_updated(index, jobs);
    }

    /// `NetworkEvent::PartyBookingDeleted`.
    pub(crate) fn handle_party_booking_deleted(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingDeleted { index } = event else {
            unreachable!("handle_party_booking_deleted called with a different event variant");
        };

        self.client_state.follow_mut(client_state().party_booking()).apply_deleted(index);
    }

    /// `NetworkEvent::PartyBookingJoinRequest`.
    pub(crate) fn handle_party_booking_join_request(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingJoinRequest {
            character_id,
            account_id,
            name,
            position,
        } = event
        else {
            unreachable!("handle_party_booking_join_request called with a different event variant");
        };

        let message = {
            let booking = self.client_state.follow_mut(client_state().party_booking());
            booking.apply_join_request(character_id, account_id, name, position);
            booking.join_requests.last().map(|request| request.name.clone())
        };

        if let Some(name) = message {
            self.push_chat_information(format!("{name} requested to join your party through the party search."));
        }

        self.open_party_window();
    }

    /// `NetworkEvent::PartyBookingJoinResult`.
    pub(crate) fn handle_party_booking_join_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PartyBookingJoinResult {
            player_name,
            party_name,
            account_id,
            refused,
        } = event
        else {
            unreachable!("handle_party_booking_join_result called with a different event variant");
        };

        let message = {
            let booking = self.client_state.follow_mut(client_state().party_booking());
            booking.apply_join_result(player_name, party_name, account_id, refused);
            booking.status_message.clone()
        };
        self.push_chat_information(message);
    }

    /// `NetworkEvent::ClanInfo`.
    pub(crate) fn handle_clan_info(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ClanInfo { info } = event else {
            unreachable!("handle_clan_info called with a different event variant");
        };

        self.client_state.follow_mut(client_state().clan()).apply_info(info);
    }

    /// `NetworkEvent::ClanOnlineCount`.
    pub(crate) fn handle_clan_online_count(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ClanOnlineCount {
            online_members,
            maximum_members,
        } = event
        else {
            unreachable!("handle_clan_online_count called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().clan())
            .apply_online_count(online_members, maximum_members);
    }

    /// `NetworkEvent::FriendRemoved`.
    pub(crate) fn handle_friend_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::FriendRemoved { account_id, character_id } = event else {
            unreachable!("handle_friend_removed called with a different event variant");
        };

        let friend_list = self.client_state.follow_mut(client_state().friend_list());
        if let Some(index) = friend_list
            .iter()
            .position(|friend| friend.account_id == account_id && friend.character_id == character_id)
        {
            friend_list.remove(index);
            let friend_online = self.client_state.follow_mut(client_state().friend_online());
            if index < friend_online.len() {
                friend_online.remove(index);
            }
        }
    }

    /// `NetworkEvent::FriendAdded`.
    pub(crate) fn handle_friend_added(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::FriendAdded { friend } = event else {
            unreachable!("handle_friend_added called with a different event variant");
        };

        crate::state::friend_names::remember(friend.character_id.0, &friend.name);
        self.client_state.follow_mut(client_state().friend_list()).push(friend);
        self.client_state.follow_mut(client_state().friend_online()).push(false);
    }

    /// `NetworkEvent::FriendOnlineState`.
    pub(crate) fn handle_friend_online_state(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::FriendOnlineState {
            account_id,
            character_id,
            online,
            name,
        } = event
        else {
            unreachable!("handle_friend_online_state called with a different event variant");
        };

        crate::logging::log_debug!("friend 0x0206: cid={} online={} name={:?}", character_id.0, online, name);
        crate::state::friend_names::remember(character_id.0, &name);

        let index = self
            .client_state
            .follow(client_state().friend_list())
            .iter()
            .position(|friend| friend.character_id == character_id || friend.account_id == account_id);

        if let Some(index) = index {
            {
                let friend_list = self.client_state.follow_mut(client_state().friend_list());
                if !name.trim().is_empty() {
                    friend_list[index].name = name.clone();
                } else if friend_list[index].name.trim().is_empty() {
                    friend_list[index].name = crate::state::friend_names::cached_name(character_id.0).unwrap_or_default();
                }
            }

            let friend_count = self.client_state.follow(client_state().friend_list()).len();
            let friend_online = self.client_state.follow_mut(client_state().friend_online());
            friend_online.resize(friend_count, false);
            friend_online[index] = online;
        } else {
            // The 0x0206 update can arrive before/without a 0x0201 entry
            // (for example right after accepting a friend request): add the
            // friend so the UI shows it immediately.
            let display_name = if name.trim().is_empty() {
                crate::state::friend_names::cached_name(character_id.0).unwrap_or_default()
            } else {
                name.clone()
            };

            let friend_count = {
                let friend_list = self.client_state.follow_mut(client_state().friend_list());
                friend_list.push(Friend {
                    account_id,
                    character_id,
                    name: display_name,
                });
                friend_list.len()
            };

            let friend_online = self.client_state.follow_mut(client_state().friend_online());
            friend_online.resize(friend_count, false);
            friend_online[friend_count - 1] = online;
        }
    }

    /// `NetworkEvent::SetFriendList`.
    pub(crate) fn handle_set_friend_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SetFriendList { friend_list } = event else {
            unreachable!("handle_set_friend_list called with a different event variant");
        };

        let friend_list = friend_list
            .into_iter()
            .map(|mut friend| {
                if friend.name.trim().is_empty() {
                    if let Some(name) = crate::state::friend_names::cached_name(friend.character_id.0) {
                        friend.name = name;
                    }
                } else {
                    crate::state::friend_names::remember(friend.character_id.0, &friend.name);
                }
                friend
            })
            .collect::<Vec<_>>();
        let friend_count = friend_list.len();

        crate::logging::log_debug!(
            "friend list: {} entries (names filled from the persistent cache where known)",
            friend_count
        );

        *self.client_state.follow_mut(client_state().friend_list()) = friend_list;
        *self.client_state.follow_mut(client_state().friend_online()) = vec![false; friend_count];
    }
}
