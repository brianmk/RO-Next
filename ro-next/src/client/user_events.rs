//! The interface/user input dispatcher: one arm per `UserEvent`.
use crate::*;

impl Client {
    /// Returns whether or not the interface is focused.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn process_user_events(
        &mut self,
        input_report: &InputReport,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] delta_time: f32,
    ) -> bool {
        self.interface.process_events(&mut self.input_event_buffer);

        // Left-clicking an item box only prepares a drag; the actual drag starts once
        // the button was held for a moment or the mouse moved a few pixels while held.
        // A quick click releases the prepared state without picking the item up.
        const ITEM_DRAG_HOLD_TIME_MS: u128 = 500;
        const ITEM_DRAG_DISTANCE: f32 = 5.0;

        match self.interface.mouse_mode().prepare_item_drag() {
            Some((source, item)) => {
                let now = std::time::Instant::now();
                let pending = *self.pending_item_drag.get_or_insert((now, input_report.mouse_position));
                let held_time_ms = now.duration_since(pending.0).as_millis();
                let moved_distance = (input_report.mouse_position.left - pending.1.left)
                    .abs()
                    .max((input_report.mouse_position.top - pending.1.top).abs());

                if input_report.mouse_button_released || !input_report.left_mouse_button_down && moved_distance == 0.0 {
                    // The button was released before the thresholds were reached: this
                    // was a plain click, not a drag.
                    self.pending_item_drag = None;
                } else if held_time_ms >= ITEM_DRAG_HOLD_TIME_MS || moved_distance > ITEM_DRAG_DISTANCE {
                    self.pending_item_drag = None;

                    self.interface.set_mouse_mode(MouseMode::Custom {
                        mode: MouseInputMode::MoveItem { item, source },
                    });
                }
            }
            None => self.pending_item_drag = None,
        }

        let interface_has_focus = self.interface.has_focus();

        // Enter submits the login form and confirms the server selection, so the
        // menus can be used without reaching for the mouse. The keys are
        // data-driven (`chat_command_keys` in `data\client\keybinds.ron`).
        if self.input_system.chat_command_key_pressed() {
            if self.interface.is_window_with_class_open(WindowClass::Login) {
                let selected_service_path = SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                let service_id = *self.client_state.follow(client_state().login_window().selected_service());
                let username = self.client_state.follow(selected_service_path.username()).clone();
                let password = self.client_state.follow(selected_service_path.password()).clone();

                if !username.is_empty() && !password.is_empty() {
                    self.input_event_buffer.push(InputEvent::LogIn {
                        service_id,
                        username,
                        password,
                    });
                }
            } else if self.interface.is_window_with_class_open(WindowClass::SelectServer)
                && let Some(character_server_information) = self.client_state.follow(client_state().character_servers()).first().cloned()
            {
                self.input_event_buffer.push(InputEvent::SelectServer {
                    character_server_information,
                });
            }
        }

        if self.interface.get_mouse_mode().is_rotating_camera() {
            // TODO: Does this really need to be a InputEvent?
            let rotation = input_report.mouse_delta.width;
            self.input_event_buffer.push(InputEvent::RotateCamera { rotation });
        }

        // Battle mode and the key binding editor both change how keyboard
        // input is routed. The state is pushed into the input system before
        // the one keyboard handler runs.
        let battle_mode = *self.client_state.follow(client_state().game_settings().battle_mode());
        self.input_system.set_battle_mode(battle_mode);

        let rebind_action = self.client_state.follow(client_state().key_rebind()).capturing();
        self.input_system.set_key_capture(rebind_action.is_some());

        // Battle mode closes the chat line with Escape, like the official
        // client: the line is the only focused element here, so the key must
        // not reach the window/menu handling.
        if rebind_action.is_none() && battle_mode && interface_has_focus && self.input_system.key_pressed(winit::keyboard::KeyCode::Escape)
        {
            self.interface.unfocus();
        }

        if let Some(action) = rebind_action {
            // While a rebind waits for a key, the key is captured instead of
            // triggering its normal action. Escape cancels the capture.
            if let Some((key, modifiers)) = self.input_system.take_captured_key() {
                if key == winit::keyboard::KeyCode::Escape {
                    self.input_event_buffer.push(InputEvent::CancelRebind);
                } else {
                    self.input_event_buffer.push(InputEvent::RebindKey {
                        action,
                        key,
                        alt: modifiers.alt,
                        control: modifiers.control,
                        shift: modifiers.shift,
                    });
                }
            }
        } else if !interface_has_focus {
            self.input_system.handle_keyboard_input(
                &mut self.input_event_buffer,
                #[cfg(feature = "debug")]
                self.interface.get_mouse_mode().is_default(),
                #[cfg(feature = "debug")]
                *self.client_state.follow(client_state().render_options().use_debug_camera()),
            );
        }

        let input_events: Vec<InputEvent> = self.input_event_buffer.drain(..).collect();

        for event in input_events {
            crate::logging::log_trace!("input: dispatch {:?}", event);

            // Any user input can change UI state (window toggles, hotkeys,
            // settings, ...), so the cached element layouts are rebuilt on the
            // next layout pass.
            self.interface.invalidate_layouts();

            match event {
                InputEvent::LogIn {
                    service_id,
                    username,
                    password,
                } => {
                    // The selected service can be stale (persisted `recent_service_id`
                    // from an older client info) and its address can fail DNS
                    // resolution. Both must show an error window, never panic.
                    let services = self.client_state.follow(client_state().client_info().services()).clone();
                    let target = match crate::login_server::resolve_login_server(&services, service_id) {
                        Ok(target) => target,
                        Err(error) => {
                            crate::logging::log_warn!("cannot connect to the selected login server: {error:?}");
                            self.interface.open_window(ErrorWindow::new(error.user_message()));
                            continue;
                        }
                    };
                    let socket_address = target.socket_address;
                    let packet_version = target.packet_version;

                    self.saved_login_server_address = Some(socket_address);
                    self.saved_username = username.clone();
                    self.saved_password = password.clone();
                    self.saved_packet_version = packet_version;

                    self.session.transition(SessionEvent::LoginStarted);
                    self.networking_system
                        .connect_to_login_server(packet_version, socket_address, username, password);
                }
                InputEvent::SelectServer {
                    character_server_information,
                } => {
                    let Some(login_data) = self.saved_login_data else {
                        // The window was still open although we are not logged in (e.g.
                        // after a stale state). Go back to the login screen instead of
                        // panicking.
                        self.interface.close_window_with_class(WindowClass::SelectServer);
                        self.interface.open_window(LoginWindow::new(
                            client_state().login_window(),
                            client_state().login_settings(),
                            client_state().client_info(),
                        ));
                        continue;
                    };

                    self.saved_character_server = Some(character_server_information.clone());

                    self.networking_system.disconnect_from_login_server();

                    // `connect_to_character_server` replaces a stale connection itself, so
                    // clicking the server always starts a fresh character server session.
                    self.networking_system.connect_to_character_server(
                        self.saved_packet_version,
                        &login_data,
                        character_server_information,
                    );
                }
                InputEvent::Respawn => {
                    let _ = self.networking_system.respawn();
                    self.interface.close_window_with_class(WindowClass::Respawn);
                }
                InputEvent::LogOut => {
                    let _ = self.networking_system.log_out();
                }
                InputEvent::LogOutCharacter => {
                    // The next login is a fresh session, so the character
                    // selection cooldown must not leak into it.
                    self.client_state.follow_mut(client_state().character_selection_cooldown()).clear();
                    self.networking_system.disconnect_from_character_server();
                }
                InputEvent::Exit => SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst),
                InputEvent::ZoomCamera { zoom_factor } => self.player_camera.soft_zoom(zoom_factor),
                InputEvent::RotateCamera { rotation } => self.player_camera.soft_rotate(rotation),
                InputEvent::ResetCameraRotation => self.player_camera.reset_rotation(),
                InputEvent::ToggleMenuWindow => {
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
                InputEvent::ToggleInventoryWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Inventory) {
                            true => self.interface.close_window_with_class(WindowClass::Inventory),
                            false => self.interface.open_window(InventoryWindow::new(
                                client_state().inventory().items(),
                                client_state().interface_settings(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleEquipmentWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Equipment) {
                            true => self.interface.close_window_with_class(WindowClass::Equipment),
                            false => self.interface.open_window(EquipmentWindow::new(
                                client_state().inventory().items(),
                                this_player().manually_asserted(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleSkillTreeWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::SkillTree) {
                            true => self.interface.close_window_with_class(WindowClass::SkillTree),
                            false => self.interface.open_window(SkillTreeWindow::new(
                                client_state().skill_tree_window(),
                                client_state().skill_tree().layout(),
                                client_state().skill_tree().skills(),
                                this_player().manually_asserted().skill_points(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleStatsWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Stats) {
                            true => self.interface.close_window_with_class(WindowClass::Stats),
                            false => self.interface.open_window(StatsWindow::new(this_player().manually_asserted())),
                        }
                    }
                }
                InputEvent::ToggleGameSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GameSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GameSettings),
                    false => self.interface.open_window(GameSettingsWindow::new(client_state().game_settings())),
                },
                InputEvent::ToggleInterfaceSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::InterfaceSettings)
                {
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
                },
                InputEvent::ToggleGraphicsSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GraphicsSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GraphicsSettings),
                    false => self.interface.open_window(GraphicsSettingsWindow::new(
                        client_state().graphics_settings(),
                        client_state().graphics_settings_capabilities(),
                    )),
                },
                InputEvent::ToggleAudioSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::AudioSettings) {
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
                },
                InputEvent::ToggleMinimapWindow => {
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
                InputEvent::CycleMapWindows => {
                    // Text inputs (chat, login, character name, ...) get the M
                    // key, the map cycle must not steal it while they are focused.
                    if !self.interface.has_focus() && self.client_state.try_follow(this_entity()).is_some() {
                        Self::cycle_map_windows(&mut self.interface, &mut self.map_window_cycle);
                    }
                }
                InputEvent::ToggleFriendListWindow => {
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
                InputEvent::OpenMailboxWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Mail) {
                            true => self.interface.close_window_with_class(WindowClass::Mail),
                            false => {
                                self.interface.open_window(MailboxWindow::new(client_state().mail()));
                                // The mailbox list is only sent on request.
                                let _ = self.networking_system.open_mailbox();
                            }
                        }
                    }
                }
                InputEvent::RefreshMailbox => {
                    let _ = self.networking_system.refresh_mailbox();
                }
                InputEvent::ReadMail { mail_id, open_type } => {
                    let _ = self.networking_system.read_mail(mail_id, open_type);
                }
                InputEvent::ClaimMailZeny { mail_id, open_type } => {
                    let _ = self.networking_system.claim_mail_zeny(mail_id, open_type);
                }
                InputEvent::ClaimMailItems { mail_id, open_type } => {
                    let _ = self.networking_system.claim_mail_items(mail_id, open_type);
                }
                InputEvent::DeleteMail { mail_id, open_type } => {
                    let _ = self.networking_system.delete_mail(mail_id, open_type);
                }
                InputEvent::TogglePartyWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Party) {
                            true => self.interface.close_window_with_class(WindowClass::Party),
                            false => self.interface.open_window(PartyWindow),
                        }
                    }
                }
                InputEvent::CreateParty { name } => {
                    self.client_state.follow_mut(client_state().party().create_name_input()).clear();
                    let _ = self.networking_system.create_party(name);
                }
                InputEvent::InviteToParty { name } => {
                    self.client_state.follow_mut(client_state().party().invite_name_input()).clear();
                    let _ = self.networking_system.invite_to_party(name);
                }
                InputEvent::AcceptPartyInvite { party_id } => {
                    let _ = self.networking_system.reply_party_invite(party_id, true);

                    let party = self.client_state.follow_mut(client_state().party());
                    party.party_id = Some(party_id);
                    party.invite = None;

                    self.interface.close_window_with_class(WindowClass::PartyInvite);

                    self.open_party_window();
                }
                InputEvent::RejectPartyInvite { party_id } => {
                    let _ = self.networking_system.reply_party_invite(party_id, false);
                    *self.client_state.follow_mut(client_state().party().invite()) = None;
                    self.interface.close_window_with_class(WindowClass::PartyInvite);
                }
                InputEvent::LeaveParty => {
                    let _ = self.networking_system.leave_party();

                    self.client_state.follow_mut(client_state().party()).clear();
                    self.interface.close_window_with_class(WindowClass::Party);
                }
                InputEvent::ToggleClanWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Clan) {
                            true => self.interface.close_window_with_class(WindowClass::Clan),
                            false => self.interface.open_window(ClanWindow),
                        }
                    }
                }
                InputEvent::SetPartyBookingVisible { visible } => {
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
                InputEvent::RegisterPartyBooking => {
                    let booking = self.client_state.follow(client_state().party_booking());
                    let level = booking.register_level();
                    let map_id = booking.register_map_id();
                    let jobs = booking.parse_register_jobs();
                    let _ = self.networking_system.register_party_booking(level, map_id, jobs);
                }
                InputEvent::SearchPartyBooking { next_page } => {
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
                InputEvent::DeletePartyBooking => {
                    let _ = self.networking_system.delete_party_booking();
                }
                InputEvent::UpdatePartyBooking => {
                    let jobs = self.client_state.follow(client_state().party_booking()).parse_register_jobs();
                    let _ = self.networking_system.update_party_booking(jobs);
                }
                InputEvent::JoinPartyBooking { index } => {
                    let booking = self.client_state.follow(client_state().party_booking());

                    if !booking.join_requests_enabled {
                        self.push_chat_information(
                            "The booking join request is disabled: the CondaRO 20190605 server does not parse 0x0AE6. Enable it only on a \
                             server with PACKETVER >= 20191204."
                                .to_owned(),
                        );
                        continue;
                    }

                    let Some(entry) = booking.result(index) else {
                        continue;
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
                            "{leader_name} is not visible on this map; the 2019 booking search has no character id, so the join request \
                             cannot be sent."
                        )),
                    }
                }
                InputEvent::ReplyPartyBookingJoin {
                    character_id,
                    account_id,
                    accept,
                } => {
                    let _ = self.networking_system.reply_party_booking_join(character_id, account_id, accept);
                    self.client_state
                        .follow_mut(client_state().party_booking())
                        .remove_join_request(character_id);
                }
                InputEvent::ChangePartyOption {
                    experience_option,
                    item_pickup,
                    item_share,
                } => {
                    let _ = self
                        .networking_system
                        .change_party_option(experience_option, item_pickup, item_share);

                    let party = self.client_state.follow_mut(client_state().party());
                    party.experience_option = experience_option;
                    party.item_pickup = item_pickup;
                    party.item_share = item_share;
                }
                InputEvent::ChangePartyLeader { name } => {
                    let account_id = self.client_state.follow(client_state().party()).member_account_id(&name);

                    match account_id {
                        Some(account_id) => {
                            let _ = self.networking_system.change_party_leader(account_id);
                        }
                        None => self.push_chat_information(format!("No party member named {name}.")),
                    }
                }
                InputEvent::OpenCompanionInfo { entity_id } => {
                    let kind = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .and_then(|entity| match entity.get_entity_type() {
                            EntityType::Pet => Some(CompanionKind::Pet),
                            EntityType::Homunculus => Some(CompanionKind::Homunculus),
                            EntityType::Mercenary => Some(CompanionKind::Mercenary),
                            _ => None,
                        });

                    if let Some(kind) = kind {
                        self.interface.close_window_with_class(WindowClass::CompanionInfo);
                        self.interface.open_window(CompanionInfoWindow::new(entity_id, kind));
                    }
                }
                InputEvent::ToggleGuildWindow => {
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
                InputEvent::InviteToGuild { name } => {
                    let can_invite = self.client_state.follow(client_state().guild()).can_invite();

                    if can_invite {
                        let _ = self.networking_system.invite_to_guild(name.clone());
                        self.client_state.follow_mut(client_state().guild()).status_message = format!("Invited {name}.");
                    } else {
                        self.client_state.follow_mut(client_state().guild()).status_message =
                            "You do not have permission to invite members.".to_owned();
                    }
                }
                InputEvent::ExpelGuildMember { name } => {
                    let outcome = {
                        let guild = self.client_state.follow(client_state().guild());

                        if !guild.can_expel() {
                            Err("You do not have permission to expel members.".to_owned())
                        } else {
                            match guild.member_by_name(&name) {
                                Some(member) if Some(member.character_id) == guild.local_character_id => {
                                    Err("You cannot expel yourself.".to_owned())
                                }
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
                InputEvent::LeaveGuild => {
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
                InputEvent::SaveGuildNotice { subject, notice } => {
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
                InputEvent::ToggleBigMapWindow => {
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
                InputEvent::ToggleChatWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Chat) {
                            true => self.interface.close_window_with_class(WindowClass::Chat),
                            false => self.interface.open_window(ChatWindow::new(
                                client_state().chat_window(),
                                client_state().chat_messages(),
                                client_state().interface_settings(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleChatRoomWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::ChatRoom) {
                            true => self.interface.close_window_with_class(WindowClass::ChatRoom),
                            false => self
                                .interface
                                .open_window(ChatRoomWindow::new(client_state().content_windows().chat_room())),
                        }
                    }
                }
                InputEvent::ToggleEmotePickerWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::EmotePicker) {
                            true => self.interface.close_window_with_class(WindowClass::EmotePicker),
                            false => self.interface.open_window(EmotePickerWindow),
                        }
                    }
                }
                InputEvent::CreateChatRoom { title, limit } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .create_title = title.clone();

                    match self.networking_system.create_chat_room(title, String::new(), limit, 1) {
                        Ok(()) => {}
                        Err(_) => {
                            self.client_state
                                .follow_mut(client_state().content_windows().chat_room())
                                .set_error("Not connected to the map server.".to_owned());
                            self.interface.invalidate_layouts();
                        }
                    }
                }
                InputEvent::JoinChatRoom { chat_id, title, password } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .set_joining_title(title);

                    if self.networking_system.enter_chat_room(chat_id, password).is_err() {
                        self.client_state
                            .follow_mut(client_state().content_windows().chat_room())
                            .set_error("Not connected to the map server.".to_owned());
                        self.interface.invalidate_layouts();
                    }
                }
                InputEvent::LeaveChatRoom => {
                    let _ = self.networking_system.leave_chat_room();
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .leave_locally();
                    self.interface.invalidate_layouts();
                }
                InputEvent::SendChatRoomMessage { text } => {
                    let player_name = self.client_state.follow(client_state().player_name()).clone();

                    if self.networking_system.send_chat_message(&player_name, &text).is_err() {
                        self.client_state
                            .follow_mut(client_state().content_windows().chat_room())
                            .set_error("Not connected to the map server.".to_owned());
                    }
                }
                InputEvent::SubmitPincode { code } => {
                    let (seed, account_id, is_new) = {
                        let pincode = self.client_state.follow(client_state().content_windows().pincode());
                        (pincode.seed, pincode.account_id, pincode.is_new)
                    };

                    match crate::pincode::encrypt_pincode(seed, &code) {
                        Some(pin) => {
                            if self.networking_system.send_pincode(account_id, pin, is_new).is_ok() {
                                self.client_state
                                    .follow_mut(client_state().content_windows().pincode())
                                    .code
                                    .clear();
                            } else {
                                self.client_state.follow_mut(client_state().content_windows().pincode()).error =
                                    "Not connected to the character server.".to_owned();
                            }
                        }
                        None => {
                            self.client_state.follow_mut(client_state().content_windows().pincode()).error =
                                "Enter exactly 4 digits.".to_owned();
                        }
                    }
                }
                InputEvent::ClosePincodePrompt => {
                    self.client_state.follow_mut(client_state().content_windows().pincode()).dismiss();
                    self.interface.close_window_with_class(WindowClass::Pincode);
                }
                InputEvent::OpenBookPage { book_id, page } => {
                    let pages = load_book_pages(&self.game_file_loader, book_id);
                    self.interface.close_window_with_class(WindowClass::Book);
                    self.interface.open_window(BookWindow::new(book_id, page, pages));
                }
                InputEvent::CloseTopWindow => {
                    let pending_warp_point = self.client_state.follow(client_state().dialog_window()).warp_point_skill();
                    let dialog_was_open = self.interface.is_window_with_class_open(WindowClass::Dialog);
                    self.interface.close_top_window(&self.client_state);

                    // Closing the Warp Portal destination list without a choice
                    // answers the server with an invalid destination so it can drop
                    // the pending cast state (`skill_castend_map` clears the menu).
                    if !self.interface.is_window_with_class_open(WindowClass::Dialog)
                        && dialog_was_open
                        && let Some(skill_id) = pending_warp_point
                    {
                        let dialog = self.client_state.follow_mut(client_state().dialog_window());
                        dialog.clear_warp_point();
                        dialog.clear_choices();
                        let _ = self.networking_system.select_warp_point(skill_id, String::new());
                    }
                }
                InputEvent::OpenCartWindow => {
                    if !self.interface.is_window_with_class_open(WindowClass::Cart) {
                        self.interface.open_window(CartWindow);
                    }
                }
                InputEvent::BrowseVendingShop {
                    owner_account_id,
                    shop_name,
                } => {
                    self.client_state
                        .follow_mut(client_state().vending_browse())
                        .set_pending_shop_name(shop_name);
                    let _ = self.networking_system.request_vending_list(AccountId(owner_account_id));
                }
                InputEvent::BuyVendingItem { cart_index, amount, price } => {
                    let owner_account_id = self.client_state.follow(client_state().vending_browse()).owner_account_id();

                    if let Some(zeny) = *self.client_state.follow(client_state().zeny())
                        && zeny < u64::from(price)
                    {
                        self.push_chat_warning("You do not have enough zeny.".to_owned());
                        continue;
                    }

                    let _ = self
                        .networking_system
                        .purchase_from_vending(AccountId(owner_account_id), vec![(cart_index, amount)]);

                    // rAthena resets its "which shop was opened" state after every
                    // purchase request (`clif_parse_PurchaseReq`), so the next row
                    // would fail with "store incorrect". Request the (updated) list
                    // again to keep the window usable and its stock in sync. The
                    // server does not send a success ack; the bought item arrives
                    // through the regular inventory packets.
                    self.client_state
                        .follow_mut(client_state().vending_browse())
                        .reduce_stock(cart_index, amount);
                    let _ = self.networking_system.request_vending_list(AccountId(owner_account_id));
                }
                InputEvent::VendingOpen => {
                    let vending = self.client_state.follow(client_state().vending());
                    let shop_name = vending.shop_name().to_owned();
                    let items: Vec<ragnarok_packets::StoreItemEntry> = vending
                        .entries()
                        .iter()
                        .map(|entry| ragnarok_packets::StoreItemEntry {
                            inventory_index: entry.inventory_index,
                            amount: entry.amount,
                            price: entry.price,
                        })
                        .collect();
                    if !shop_name.is_empty() && !items.is_empty() {
                        let _ = self.networking_system.request_open_store(shop_name, items);
                    } else {
                        let _ = self.networking_system.cancel_open_store();
                    }
                    self.client_state.follow_mut(client_state().vending()).close();
                    self.interface.close_window_with_class(WindowClass::Vending);
                }
                InputEvent::VendingCancel => {
                    let _ = self.networking_system.cancel_open_store();
                    self.client_state.follow_mut(client_state().vending()).close();
                    self.interface.close_window_with_class(WindowClass::Vending);
                }
                InputEvent::ToggleQuestWindow => {
                    if self.interface.is_window_with_class_open(WindowClass::Quest) {
                        self.interface.close_window_with_class(WindowClass::Quest);
                    } else {
                        self.interface.open_window(QuestWindow::new(client_state().quest_log()));
                    }
                }
                InputEvent::ToggleAchievementWindow => self.toggle_achievement_window(),
                InputEvent::ToggleNavigationWindow => self.toggle_navigation_window(),
                InputEvent::ToggleMonsterDatabaseWindow => self.toggle_monster_database_window(),
                InputEvent::ClaimAchievementReward { achievement_id } => {
                    let _ = self.networking_system.request_achievement_reward(achievement_id);
                    self.push_chat_information(format!("Achievement {achievement_id}: reward requested."));
                }
                InputEvent::NavigateToDestination { name, map_name, position } => {
                    self.set_navigation_destination(Some(name), &map_name, position);
                }
                InputEvent::ClearNavigationDestination => {
                    self.clear_navigation();
                    self.push_chat_information("Navigation destination cleared.".to_owned());
                }
                InputEvent::SelectMonster { job_id } => {
                    self.client_state.follow_mut(client_state().monster_database()).set_selected(job_id);
                }
                InputEvent::TradeRequest { account_id } => {
                    let _ = self.networking_system.request_trade(ragnarok_packets::AccountId(account_id));
                }
                InputEvent::TradeAccept { account_id } => {
                    let _ = account_id;
                    let _ = self.networking_system.answer_trade(true);
                }
                InputEvent::TradeDecline => {
                    let trade = self.client_state.follow_mut(client_state().trade());
                    trade.clear_request();
                    self.interface.close_window_with_class(WindowClass::TradeRequest);
                    let _ = self.networking_system.answer_trade(false);
                }
                InputEvent::TradeAddItem { inventory_index, amount } => {
                    let _ = self.networking_system.add_trade_item(inventory_index, amount as u32);
                }
                InputEvent::TradeOfferZeny => {
                    let amount = self
                        .client_state
                        .follow(client_state().trade())
                        .zeny()
                        .trim()
                        .parse::<u32>()
                        .unwrap_or(0);
                    if amount > 0 {
                        let _ = self.networking_system.add_trade_item(0, amount);
                    }
                }
                InputEvent::TradeLock => {
                    let _ = self.networking_system.confirm_trade();
                    self.client_state.follow_mut(client_state().trade()).set_locked(true);
                }
                InputEvent::TradeCommit => {
                    let _ = self.networking_system.commit_trade();
                }
                InputEvent::TradeCancel => {
                    let _ = self.networking_system.cancel_trade();
                    self.close_trade();
                }
                InputEvent::ViewCharacterInfo { name, level, guild_name } => {
                    self.interface.open_window(CharacterInfoWindow::new(name, level, guild_name));
                }
                InputEvent::ToggleShowInterface => self.show_interface = !self.show_interface,
                InputEvent::SelectCharacter { slot } => {
                    // The cards of blocked characters do not register a click handler,
                    // but a click can race with the layout update that removes the
                    // handlers. Check the cooldown here as well so a fast double click
                    // can never send a select packet while the server is still
                    // cleaning up the previous session.
                    let character_id = self
                        .client_state
                        .follow(client_state().character_slots())
                        .character_in_slot(slot)
                        .map(|character_information| character_information.character_id);

                    if let Some(character_id) = character_id {
                        let now = Instant::now();
                        let remaining = self
                            .client_state
                            .follow(client_state().character_selection_cooldown())
                            .remaining(character_id, now);

                        if let Some(remaining) = remaining {
                            crate::logging::log_debug!(
                                "character selection: slot {slot} is still cooling down for {} ms, ignoring the click",
                                remaining.as_millis()
                            );
                            continue;
                        }
                    }

                    crate::logging::log_debug!(
                        "character selection: requesting slot {} (character server connected: {})",
                        slot,
                        self.networking_system.is_character_server_connected()
                    );

                    if let Some(character_id) = character_id {
                        self.client_state
                            .follow_mut(client_state().character_selection_cooldown())
                            .remember_selection(character_id);
                    }

                    if self.networking_system.select_character(slot).is_err() {
                        crate::logging::log_warn!("character selection: could not send the select packet (not connected)");
                    }
                }
                InputEvent::OpenCharacterCreationWindow { slot } => {
                    // Clear the name before opening the window.
                    self.client_state.follow_mut(client_state().create_character_name()).clear();

                    self.interface
                        .open_window(CharacterCreationWindow::new(client_state().create_character_name(), slot))
                }
                InputEvent::CreateCharacter { slot, name } => {
                    let _ = self.networking_system.create_character(slot, name);
                }
                InputEvent::OpenCharacterDeleteWindow { character_id } => {
                    // Never keep the e-mail of a previous attempt around.
                    self.client_state.follow_mut(client_state().delete_character_email()).clear();
                    self.interface.open_window(CharacterDeletionWindow::new(
                        character_id,
                        client_state().delete_character_email(),
                    ));
                }
                InputEvent::DeleteCharacter { character_id, email } => {
                    if self.client_state.follow(client_state().currently_deleting()).is_none() {
                        let _ = self.networking_system.delete_character(character_id, email);
                        *self.client_state.follow_mut(client_state().currently_deleting()) = Some(character_id);
                        self.interface.close_window_with_class(WindowClass::CharacterDeletion);
                    }
                }
                InputEvent::SwitchCharacterSlot {
                    origin_slot,
                    destination_slot,
                } => {
                    let _ = self.networking_system.switch_character_slot(origin_slot, destination_slot);
                }
                InputEvent::PlayerMove { destination } => {
                    if self.movement_blocked_by_cast(client_tick) {
                        // Classic cast rooting: while the local player is in a
                        // server-confirmed cast with a cast time, walk requests
                        // are ignored (the server would ignore them anyway).
                        // Instant skills never reach this branch because they
                        // do not start a cast bar.
                        continue;
                    }

                    if self.client_state.try_follow(this_entity()).is_some() {
                        let _ = self.networking_system.player_move(WorldPosition {
                            x: destination.x,
                            y: destination.y,
                            direction: Direction::North,
                        });
                    }

                    // Unbuffer any buffered action.
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;
                }
                InputEvent::OpenCharacterContextMenu { entity_id, position } => {
                    if open_character_context_menu(&self.client_state, &mut self.interface, entity_id, position) {
                        self.pending_context_menu = Some((entity_id, position));
                        let _ = self.networking_system.entity_details(entity_id);
                    }
                }
                InputEvent::CloseContextMenu => {
                    self.interface.close_window_with_class(WindowClass::ContextMenu);
                }
                InputEvent::OpenInventoryItemMenu { inventory_index } => {
                    open_inventory_item_menu(
                        &self.client_state,
                        &mut self.interface,
                        inventory_index,
                        input_report.mouse_position,
                    );
                }
                InputEvent::OpenStorageItemMenu { storage_index } => {
                    open_storage_item_menu(
                        &self.client_state,
                        &mut self.interface,
                        storage_index,
                        input_report.mouse_position,
                    );
                }
                InputEvent::DepositInventoryItem { inventory_index, amount } => {
                    let _ = self.networking_system.storage_deposit(inventory_index, amount);
                }
                InputEvent::WithdrawStorageItem { storage_index, amount } => {
                    // The server expects the storage slot encoded as `slot + 1`.
                    let _ = self.networking_system.storage_withdraw(storage_index.0, amount);
                }
                InputEvent::CloseStorage => {
                    let _ = self.networking_system.close_storage();
                    self.close_storage_window();
                }
                InputEvent::ToggleBankWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Bank) {
                            true => self.close_bank_window(),
                            false => self.open_bank_window(),
                        }
                    }
                }
                InputEvent::BankDeposit => {
                    let amount = parse_zeny_amount(self.client_state.follow(client_state().bank()).deposit_input());

                    match amount {
                        Some(amount) => {
                            if let Some(account_id) = self.own_account_id() {
                                let _ = self.networking_system.bank_deposit(account_id, amount);
                                self.client_state.follow_mut(client_state().bank()).set_status("Depositing...");
                            }
                        }
                        None => self
                            .client_state
                            .follow_mut(client_state().bank())
                            .set_status("Enter a valid zeny amount first."),
                    }
                }
                InputEvent::BankWithdraw => {
                    let amount = parse_zeny_amount(self.client_state.follow(client_state().bank()).withdraw_input());

                    match amount {
                        Some(amount) => {
                            if let Some(account_id) = self.own_account_id() {
                                let _ = self.networking_system.bank_withdraw(account_id, amount);
                                self.client_state.follow_mut(client_state().bank()).set_status("Withdrawing...");
                            }
                        }
                        None => self
                            .client_state
                            .follow_mut(client_state().bank())
                            .set_status("Enter a valid zeny amount first."),
                    }
                }
                InputEvent::CloseBank => self.close_bank_window(),
                InputEvent::ItemDoubleClick { source, item } => {
                    // Double clicking is the quick way to move a whole stack while
                    // the Kafra storage window is open.
                    if self.interface.is_window_with_class_open(WindowClass::Storage) {
                        match source {
                            ItemSource::Inventory => {
                                let _ = self.networking_system.storage_deposit(item.index, item_amount(&item).max(1));
                            }
                            ItemSource::Storage => {
                                let _ = self.networking_system.storage_withdraw(item.index.0, item_amount(&item).max(1));
                            }
                            ItemSource::Cart => {
                                let _ = self.networking_system.move_item_from_cart(item.index, item_amount(&item).max(1));
                            }
                            ItemSource::Equipment { .. } => {}
                        }
                        continue;
                    }

                    // While the cart window is open, double clicking moves a stack
                    // between the inventory and the pushcart.
                    if self.interface.is_window_with_class_open(WindowClass::Cart) {
                        match source {
                            ItemSource::Inventory => {
                                let _ = self.networking_system.move_item_to_cart(item.index, item_amount(&item).max(1));
                            }
                            ItemSource::Cart => {
                                let _ = self.networking_system.move_item_from_cart(item.index, item_amount(&item).max(1));
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Otherwise double clicking an inventory item uses consumables and
                    // tries to equip equipment; the server validates requirements.
                    if let ItemSource::Inventory = source {
                        match item_use_action(&item) {
                            Some(ItemUseAction::Use) if item.is_identified() => {
                                let _ = self.networking_system.use_item(item.index, item.item_id);
                            }
                            Some(ItemUseAction::Equip(equip_position)) => {
                                let _ = self.networking_system.request_item_equip(item.index, equip_position);
                            }
                            _ => {}
                        }
                    }
                }
                InputEvent::SubmitDialogInput { npc_id, string_input } => {
                    let text = {
                        let dialog = self.client_state.follow_mut(client_state().dialog_window());
                        std::mem::take(&mut dialog.input_text)
                    };
                    let result = if string_input {
                        self.networking_system.input_edit_dialog_str(npc_id, text)
                    } else {
                        let value = text.trim().parse::<i32>().unwrap_or(0);
                        self.networking_system.input_edit_dialog(npc_id, value)
                    };

                    if let Err(error) = result {
                        crate::logging::log_warn!("failed to send dialog input: {error:?}");
                    }
                }
                InputEvent::DropItem { inventory_index, amount } => {
                    if let Err(error) = self.networking_system.drop_item(inventory_index, amount) {
                        crate::logging::log_warn!("failed to send item drop: {error:?}");
                    }
                }
                InputEvent::OpenItemInfo { inventory_index, source } => {
                    open_item_info_window(
                        &self.client_state,
                        &mut self.interface,
                        inventory_index,
                        source,
                        &self.library,
                        &self.texture_loader,
                    );
                }
                InputEvent::CloseItemInfo => {
                    self.interface.close_window_with_class(WindowClass::ItemInfo);
                }
                InputEvent::ChatMarkupClicked { link } => match link {
                    ChatMarkupLink::Item { item_id, identified } => {
                        // Chat item links are not inventory items: synthesize a
                        // minimal identified stack so the normal item info
                        // window (artwork, stats, description, cards) can show
                        // it instead of doing nothing.
                        let item = ro_next_networking::InventoryItem {
                            metadata: ro_next_networking::NoMetadata,
                            index: InventoryIndex(0),
                            item_id,
                            item_type: 0,
                            slot: [0; 4],
                            hire_expiration_date: 0,
                            details: ro_next_networking::InventoryItemDetails::Regular {
                                amount: 1,
                                equipped_position: EquipPosition::empty(),
                                flags: match identified {
                                    true => ragnarok_packets::RegularItemFlags::IDENTIFIED,
                                    false => ragnarok_packets::RegularItemFlags::empty(),
                                },
                            },
                        };
                        let item = self.async_loader.request_inventory_item_metadata_load(item);

                        open_item_info_window_for_item(
                            &self.client_state,
                            &mut self.interface,
                            &item,
                            None,
                            &self.library,
                            &self.texture_loader,
                        );
                    }
                    ChatMarkupLink::Navi { map_name, x, y } => {
                        self.navigate_to_map_position(&map_name, x, y);
                    }
                    ChatMarkupLink::Url { url } => {
                        open_system_url(&url);
                    }
                    ChatMarkupLink::Whisper { partner } => {
                        self.open_whisper_window(&partner);
                    }
                },
                InputEvent::UseInventoryItem { inventory_index, item_id } => {
                    let _ = self.networking_system.use_item(inventory_index, item_id);
                }
                InputEvent::RefineWeapon { inventory_index } => {
                    // rAthena drops the request unless the `WS_WEAPONREFINE`
                    // menu skill is pending (`clif_parse_WeaponRefine`,
                    // `clif.cpp:13255`), i.e. unless the server sent the refine
                    // list. Explain that instead of sending a request that is
                    // silently ignored.
                    if *self.client_state.follow(client_state().item_service().kind()) != Some(ItemServiceKind::WeaponRefine) {
                        self.push_chat_warning("Use the Blacksmith Weapon Refine skill to start refining a weapon.".to_owned());
                    } else if let Err(error) = self.networking_system.refine_weapon(inventory_index) {
                        crate::logging::log_warn!("item service: could not send refine request: {error:?}");
                    }
                }
                InputEvent::RepairItem { inventory_index } => {
                    // The repair request is only valid while the
                    // `BS_REPAIRWEAPON` menu skill is pending (`clif.cpp:13231`).
                    if *self.client_state.follow(client_state().item_service().kind()) != Some(ItemServiceKind::ItemRepair) {
                        self.push_chat_warning("Use the Repair Weapon skill to repair damaged items.".to_owned());
                        continue;
                    }

                    // The item id/refine come from the repair list: when the
                    // player repairs another player's equipment the slot refers
                    // to *that* player's inventory, so a lookup in the local
                    // inventory would fail (or find an unrelated item). rAthena
                    // only reads the index from the request (`clif.cpp:13242`),
                    // so the other fields are informational and the card slots
                    // can stay zero.
                    let (item_id, refine) = {
                        let service = self.client_state.follow(client_state().item_service());
                        let from_list = service
                            .entries
                            .iter()
                            .find(|entry| entry.index == inventory_index)
                            .map(|entry| (entry.item_id, entry.refine));

                        from_list.unwrap_or_else(|| {
                            let inventory = self.client_state.follow(client_state().inventory().items());
                            let from_inventory = inventory.iter().find(|item| item.index == inventory_index).map(|item| {
                                let refine = match &item.details {
                                    ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
                                    ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
                                };
                                (item.item_id, refine)
                            });

                            from_inventory.unwrap_or((ItemId(0), 0))
                        })
                    };

                    if let Err(error) = self.networking_system.repair_item(inventory_index, item_id, refine, [0; 4]) {
                        crate::logging::log_warn!("item service: could not send repair request: {error:?}");
                    }
                }
                InputEvent::IdentifyItem { inventory_index } => {
                    // Answer to the identify list that a Magnifier produced.
                    if let Err(error) = self.networking_system.identify_item(inventory_index) {
                        crate::logging::log_warn!("item service: could not send identify request: {error:?}");
                    }
                }
                InputEvent::OneClickIdentify { inventory_index } => {
                    // The one-click identify consumes a Magnifier (or Novice
                    // Magnifier); rAthena silently ignores the request when the
                    // player has none (`clif.cpp:21397`), so check first and tell
                    // the player instead of doing nothing.
                    const MAGNIFIER_ITEM_ID: ItemId = ItemId(611);
                    const NOVICE_MAGNIFIER_ITEM_ID: ItemId = ItemId(12_325);

                    let has_magnifier = self
                        .client_state
                        .follow(client_state().inventory().items())
                        .iter()
                        .any(|item| item.item_id == MAGNIFIER_ITEM_ID || item.item_id == NOVICE_MAGNIFIER_ITEM_ID);

                    match has_magnifier {
                        true => {
                            if let Err(error) = self.networking_system.one_click_identify(inventory_index) {
                                crate::logging::log_warn!("item service: could not send one-click identify: {error:?}");
                            }
                        }
                        false => self.push_chat_warning("You need a Magnifier to identify items.".to_owned()),
                    }
                }
                InputEvent::RequestCardComposition { card_index } => {
                    // Remember the card: the answer list only contains the
                    // equipment indexes, so the insert request later needs the
                    // card data from here.
                    if let Some(card) = self.inventory_service_entry(card_index) {
                        *self.client_state.follow_mut(client_state().item_service().card()) = Some(card);
                    }

                    if let Err(error) = self.networking_system.request_card_composition_list(card_index) {
                        crate::logging::log_warn!("item service: could not request card composition list: {error:?}");
                    }
                }
                InputEvent::InsertCard { card_index, equip_index } => {
                    if let Err(error) = self.networking_system.insert_card(card_index, equip_index) {
                        crate::logging::log_warn!("item service: could not send card insert: {error:?}");
                    }
                }
                InputEvent::CloseItemService => self.close_item_service_window(),
                InputEvent::SelectRefiningItem { inventory_index } => {
                    // The server answers with `ZC_REFINING_MATERIAL_LIST`, which
                    // opens the material step of the window.
                    if let Err(error) = self.networking_system.select_refining_item(inventory_index) {
                        crate::logging::log_warn!("item service: could not select refining item: {error:?}");
                    }
                }
                InputEvent::RefineWithMaterial {
                    material_item_id,
                    blacksmith_blessing,
                } => {
                    let item_index = self
                        .client_state
                        .follow(client_state().item_service().refining().item())
                        .as_ref()
                        .map(|item| item.index);

                    if let Some(item_index) = item_index
                        && let Err(error) = self
                            .networking_system
                            .request_refining(item_index, material_item_id, blacksmith_blessing)
                    {
                        crate::logging::log_warn!("item service: could not send refine attempt: {error:?}");
                    }
                }
                InputEvent::CloseRefiningUi => {
                    let _ = self.networking_system.close_refining_ui();
                    self.close_refining_window();
                }
                InputEvent::EquipInventoryItem {
                    inventory_index,
                    equip_position,
                } => {
                    let _ = self.networking_system.request_item_equip(inventory_index, equip_position);
                }
                InputEvent::WhisperChat { name } => {
                    if !self.interface.is_window_with_class_open(WindowClass::Chat) {
                        self.interface.open_window(ChatWindow::new(
                            client_state().chat_window(),
                            client_state().chat_messages(),
                            client_state().interface_settings(),
                        ));
                    }

                    self.client_state
                        .follow_mut(client_state().chat_window())
                        .set_current_text(format!("/w {name} "));
                    self.interface.focus_element(ChatTextBox);
                }
                InputEvent::OpenWhisper { partner } => {
                    self.open_whisper_window(&partner);
                    self.interface.invalidate_layouts();
                }
                InputEvent::SendWhisper { partner, text } => {
                    if text.is_empty() || partner.is_empty() {
                        continue;
                    }

                    let _ = self.networking_system.send_whisper(&partner, &text);
                    self.client_state
                        .follow_mut(client_state().whisper())
                        .push_outgoing(&partner, text.clone());
                    self.push_chat_message(ChatMessage::whisper(
                        format!("[Whisper] You : {text}"),
                        MessageColor::Information,
                        partner,
                    ));
                    self.interface.invalidate_layouts();
                }
                InputEvent::PlayerInteract { entity_id } => {
                    // Copy everything out of the entity table before touching
                    // any other `ClientState` field: the targeting rule reads
                    // the map/party/guild state, and the attack dispatch needs
                    // `&mut self`.
                    let entity_type = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| entity.get_entity_type());

                    match entity_type {
                        Some(EntityType::Npc) => {
                            let _ = self.networking_system.start_dialog(entity_id);
                        }
                        Some(EntityType::Monster) => {
                            let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                            let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                            if auto_attack {
                                *buffered_action = Some(BufferedAction::AttackEntity { entity_id });
                            }

                            let _ = self.networking_system.player_attack(entity_id);
                        }
                        Some(EntityType::Player) => {
                            // PvP/GvG/PK targeting: only dispatch the attack
                            // when the map mode allows attacking this player
                            // (`CGameMode::HandleActorPick` player branch).
                            if self.can_attack_entity(entity_id) {
                                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                                let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                                if auto_attack {
                                    *buffered_action = Some(BufferedAction::AttackEntity { entity_id });
                                }

                                let _ = self.networking_system.player_attack(entity_id);
                            }
                        }
                        Some(EntityType::Warp) => {
                            let position = self
                                .client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id)
                                .map(|entity| entity.get_tile_position());

                            if let Some(position) = position {
                                let _ = self.networking_system.player_move(WorldPosition {
                                    x: position.x,
                                    y: position.y,
                                    direction: Direction::North,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                InputEvent::PickUpItem { entity_id } => {
                    self.mouse_cursor.set_state(MouseCursorState::PickUpItem, client_tick);

                    if let Some(map) = &self.map {
                        let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                        let item_position = self
                            .client_state
                            .follow(client_state().ground_items())
                            .iter()
                            .find(|item| item.get_entity_id() == entity_id)
                            .map(|item| item.get_tile_position());

                        if let (Some(player_position), Some(item_position)) = (player_position, item_position) {
                            if player_position
                                .x
                                .abs_diff(item_position.x)
                                .max(player_position.y.abs_diff(item_position.y))
                                <= ITEM_PICKUP_RANGE.0
                            {
                                let _ = self.networking_system.pick_up_item(entity_id);

                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                player_position,
                                item_position,
                                ITEM_PICKUP_RANGE,
                            ) && let Some(nearest_tile) = path.last()
                            {
                                let _ = self.networking_system.player_move(WorldPosition {
                                    x: nearest_tile.x,
                                    y: nearest_tile.y,
                                    direction: Direction::North,
                                });

                                *self.client_state.follow_mut(client_state().buffered_action()) =
                                    Some(BufferedAction::PickUpItem { entity_id });
                            } else {
                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::WarpToMap { map_name, position } => {
                    let _ = self.networking_system.warp_to_map(map_name, position);
                }
                InputEvent::SendMessage { text } => {
                    // Battle mode (`/bm`, `/bm on`, `/bm off`) is handled
                    // entirely client-side and never sent to the server.
                    if let Some(command) = parse_battle_mode_command(&text) {
                        let enabled = match command {
                            BattleModeCommand::On => Some(true),
                            BattleModeCommand::Off => Some(false),
                            BattleModeCommand::Toggle => None,
                        };
                        self.input_event_buffer.push(InputEvent::ToggleBattleMode { enabled });
                        continue;
                    }

                    // Handle special client commands.
                    if text.as_str() == "/nc" {
                        let auto_attack = self.client_state.follow_mut(client_state().game_settings().auto_attack());
                        *auto_attack = !*auto_attack;
                        continue;
                    }

                    // Local aura style selection (`/aura <style>`). This is handled
                    // entirely client-side before the slash command translation and is
                    // never sent to the server.
                    if let Some(style_name) = text.trim().strip_prefix("/aura").map(str::trim) {
                        // Only the first word is the style, extra words are ignored.
                        let style_name = style_name.split_whitespace().next().unwrap_or_default();

                        match AuraStyle::from_style_name(style_name) {
                            Some(style) => self.set_aura_style(style),
                            None if style_name.is_empty() => {
                                self.push_localized_chat(client_state().localization().chat_aura_usage(), &[(
                                    "styles",
                                    AuraStyle::style_names(),
                                )])
                            }
                            None => self.push_localized_chat(client_state().localization().chat_aura_unknown(), &[
                                ("style", style_name),
                                ("styles", AuraStyle::style_names()),
                            ]),
                        }

                        continue;
                    }

                    // Client-side navigation guide (`/navi <map> [<x> <y>]`).
                    // The 2019 server has no navigation packet, so this never
                    // reaches the server: it marks the destination and draws the
                    // local route on the minimap.
                    {
                        let mut words = text.split_whitespace();

                        if words.next() == Some("/navi") {
                            let arguments: Vec<&str> = words.collect();

                            match arguments.as_slice() {
                                [map_name] => {
                                    self.set_navigation_destination(None, map_name, None);
                                }
                                [map_name, x, y] => match (x.parse::<u16>(), y.parse::<u16>()) {
                                    (Ok(x), Ok(y)) => self.set_navigation_destination(None, map_name, Some(TilePosition { x, y })),
                                    _ => self.push_chat_information(
                                        "Usage: /navi <map> <x> <y> (coordinates are map cells, e.g. /navi prontera 155 183).".to_owned(),
                                    ),
                                },
                                _ => self.push_chat_information("Usage: /navi <map> [<x> <y>] (e.g. /navi prontera 155 183).".to_owned()),
                            }

                            continue;
                        }
                    }

                    // Whisper command, filled in by the character context menu:
                    // "/w <name> <message>". The original client sends a dedicated
                    // whisper packet for this instead of a global chat message.
                    if let Some(whisper) = text.strip_prefix("/w ").or_else(|| text.strip_prefix("/whisper ")) {
                        let mut parts = whisper.splitn(2, ' ');
                        let target_name = parts.next().unwrap_or_default().trim();
                        let message = parts.next().unwrap_or_default().trim_start();

                        if !target_name.is_empty() {
                            // Open the dedicated conversation window and record
                            // the outgoing line locally: the server does not echo
                            // whispers back to the sender.
                            self.open_whisper_window(target_name);

                            if !message.is_empty() {
                                let _ = self.networking_system.send_whisper(target_name, message);
                                self.client_state
                                    .follow_mut(client_state().whisper())
                                    .push_outgoing(target_name, message);
                                self.push_chat_message(ChatMessage::whisper(
                                    format!("[Whisper] You : {message}"),
                                    MessageColor::Information,
                                    target_name,
                                ));
                            }
                        }

                        continue;
                    }

                    // Party chat: "/p <message>" or "/party <message>". The server
                    // echoes the message back to the whole party (including the
                    // sender), so no local echo is necessary.
                    if let Some(message) = text.strip_prefix("/p ").or_else(|| text.strip_prefix("/party ")) {
                        let message = message.trim();

                        if message.is_empty() {
                            self.push_localized_chat(client_state().localization().chat_party_usage(), &[]);
                        } else if self.client_state.follow(client_state().party()).is_in_party() {
                            let player_name = self.client_state.follow(client_state().player_name()).clone();
                            let _ = self.networking_system.send_party_chat_message(&player_name, message);
                        } else {
                            self.push_localized_chat(client_state().localization().not_in_party_text(), &[]);
                        }

                        continue;
                    }

                    // Guild chat: "/g <message>" or "/guild <message>". The
                    // server sends the message to every guild member on the
                    // map, including the sender, so no local echo is added.
                    if let Some(message) = text.strip_prefix("/g ").or_else(|| text.strip_prefix("/guild ")) {
                        let message = message.trim();

                        if message.is_empty() {
                            self.push_chat_message(ChatMessage::new_channel(
                                "Usage: /g <message>".to_owned(),
                                MessageColor::Information,
                                ChatChannel::Guild,
                            ));
                        } else {
                            let player_name = self.client_state.follow(client_state().player_name()).clone();
                            let _ = self.networking_system.send_guild_chat_message(&player_name, message);
                        }

                        continue;
                    }

                    // Official-client style slash commands (`/go prontera`,
                    // `/warp ...`) are translated into rAthena atcommands
                    // (`@go prontera`, ...); everything else is sent unchanged.
                    let translated_command = translate_slash_command(&text);

                    let outgoing_text = translated_command.as_deref().unwrap_or(&text);

                    let _ = self
                        .networking_system
                        .send_chat_message(self.client_state.follow(client_state().player_name()), outgoing_text);

                    // The server sends ZC_NOTIFY_CHAT (0x008D) only to the
                    // surrounding players (AREA_CHAT_WOC excludes the speaker),
                    // so our own speech bubble has to be created locally.
                    // Commands produce no speech: their output is sent by the
                    // server as a regular chat message instead.
                    if translated_command.is_none()
                        && let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id())
                    {
                        self.chat_bubbles.push(entity_id, text, MessageColor::Broadcast, client_tick);
                    }
                }
                InputEvent::NextDialog { npc_id } => {
                    let _ = self.networking_system.next_dialog(npc_id);
                }
                InputEvent::CloseDialog { npc_id } => {
                    let _ = self.networking_system.close_dialog(npc_id);
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);
                }
                InputEvent::ChooseDialogOption { npc_id, option } => {
                    let _ = self.networking_system.choose_dialog_option(npc_id, option);

                    // The official client closes its menu window as soon as an
                    // option is picked; the dialog window itself stays open. In
                    // our combined dialog window this means that the choice
                    // buttons have to disappear right away. Otherwise the next
                    // menu (or the answer of the NPC) would be appended below
                    // the options of the previous menu.
                    self.client_state.follow_mut(client_state().dialog_window()).clear_choices();

                    if option == -1 {
                        self.interface.close_window_with_class(WindowClass::Dialog);
                    }
                }
                InputEvent::SelectWarpPoint { option } => {
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
                InputEvent::MoveItem { source, destination, item } => match (source, destination) {
                    (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                        let _ = self.networking_system.request_item_equip(item.index, position);
                    }
                    (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                        let _ = self.networking_system.request_item_unequip(item.index);
                    }
                    (ItemSource::Inventory, ItemSource::Storage) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.storage_deposit(item.index, amount);
                    }
                    (ItemSource::Storage, ItemSource::Inventory) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.storage_withdraw(item.index.0, amount);
                    }
                    (ItemSource::Inventory, ItemSource::Cart) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.move_item_to_cart(item.index, amount);
                    }
                    (ItemSource::Cart, ItemSource::Inventory) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.move_item_from_cart(item.index, amount);
                    }
                    (ItemSource::Storage, ItemSource::Cart) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.move_storage_item_to_cart(item.index.0, amount);
                    }
                    (ItemSource::Cart, ItemSource::Storage) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.move_cart_item_to_storage(item.index, amount);
                    }
                    _ => {}
                },
                InputEvent::SendEmotion { emotion } => {
                    let _ = self.networking_system.send_emotion(emotion);
                }
                InputEvent::MoveSkill {
                    source,
                    destination,
                    skill,
                } => match (source, destination) {
                    (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .update_slot(&mut self.networking_system, slot, skill);
                    }
                    (SkillSource::Hotbar { slot }, SkillSource::SkillTree) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .clear_slot(&mut self.networking_system, slot);
                    }
                    (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                        self.client_state.follow_mut(client_state().hotbar()).swap_slot(
                            &mut self.networking_system,
                            source_slot,
                            destination_slot,
                        );
                    }
                    _ => {}
                },
                InputEvent::OpenSkillDescription {
                    name,
                    learned_level,
                    maximum_level,
                    skill_type,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                    position,
                } => {
                    self.interface.close_window_with_class(WindowClass::SkillDescription);
                    self.interface.open_window_at(
                        SkillDescriptionWindow::new(
                            name,
                            learned_level,
                            maximum_level,
                            skill_type,
                            spell_point_cost,
                            attack_range,
                            upgradable,
                        ),
                        position,
                    );
                }
                InputEvent::CycleHotbarTab { delta } => {
                    self.client_state.follow_mut(client_state().hotbar()).cycle_tab(delta);

                    let tab = self.client_state.follow(client_state().hotbar()).active_tab();
                    self.push_localized_chat(client_state().localization().chat_hotbar_page(), &[
                        ("page", &(tab + 1).to_string()),
                        ("pages", &HOTBAR_TAB_COUNT.to_string()),
                    ]);
                }
                InputEvent::ToggleHotbarWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Hotbar) {
                            true => self.interface.close_window_with_class(WindowClass::Hotbar),
                            false => self.open_hotbar_window(),
                        }
                    }
                }
                InputEvent::SwitchHotbarBar => {
                    self.client_state.follow_mut(client_state().hotbar()).switch_bar();
                }
                InputEvent::RotateHotbarRows { delta } => {
                    self.client_state.follow_mut(client_state().hotbar()).rotate_rows(delta);
                }
                InputEvent::BeginRebind { action } => {
                    self.client_state.follow_mut(client_state().key_rebind()).begin(action);
                }
                InputEvent::CancelRebind => {
                    self.client_state.follow_mut(client_state().key_rebind()).cancel();
                }
                InputEvent::RebindKey {
                    action,
                    key,
                    alt,
                    control,
                    shift,
                } => {
                    crate::input::keybinds::store_override(action, key, alt, control, shift);
                    crate::input::keybinds::save_user_bindings();
                    self.client_state.follow_mut(client_state().key_rebind()).cancel();

                    let keys = crate::input::keybinds::binding_label(action);
                    self.push_localized_chat(client_state().localization().qol().chat_keybind_saved(), &[
                        ("action", action.label()),
                        ("keys", &keys),
                    ]);
                }
                InputEvent::ResetKeybinds => {
                    crate::input::keybinds::clear_overrides();
                    crate::input::keybinds::save_user_bindings();
                    self.client_state.follow_mut(client_state().key_rebind()).cancel();
                    self.push_localized_chat(client_state().localization().qol().chat_keybinds_reset(), &[]);
                }
                InputEvent::ToggleBattleMode { enabled } => {
                    let battle_mode_path = client_state().game_settings().battle_mode();
                    let battle_mode = self.client_state.follow_mut(battle_mode_path);
                    let enabled = enabled.unwrap_or(!*battle_mode);
                    *battle_mode = enabled;

                    self.client_state.follow_mut(client_state().game_settings()).save();
                    self.interface.unfocus();

                    if enabled {
                        self.push_localized_chat(client_state().localization().qol().chat_battle_mode_on(), &[]);
                    } else {
                        self.push_localized_chat(client_state().localization().qol().chat_battle_mode_off(), &[]);
                    }
                }
                InputEvent::UseSkillInSlot { slot } => {
                    // Item hotkeys use the first matching inventory item, like the
                    // original client uses the first stack of that item.
                    if let Some(item) = self.client_state.follow(client_state().hotbar()).item_in_slot(slot) {
                        let inventory_index = self
                            .client_state
                            .follow(client_state().inventory().items())
                            .iter()
                            .find(|inventory_item| inventory_item.item_id == item.item_id)
                            .map(|inventory_item| inventory_item.index);

                        match inventory_index {
                            Some(inventory_index) => {
                                let _ = self.networking_system.use_item(inventory_index, item.item_id);
                            }
                            None => self.push_localized_chat(client_state().localization().chat_hotkey_item_missing(), &[]),
                        }

                        continue;
                    }

                    // Skills that require a target switch to a target selection
                    // mouse mode, while self cast skills are used immediately,
                    // like in the original client.
                    let skill = self
                        .client_state
                        .follow(client_state().hotbar())
                        .get_skill_in_slot(slot)
                        .as_ref()
                        .and_then(|learnable_skill| {
                            self.client_state
                                .follow(client_state().skill_tree().skills())
                                .iter()
                                .find(|learned_skill| learned_skill.skill_id == learnable_skill.skill_id)
                                .map(|learned_skill| {
                                    // The character may have the skill at a lower level than
                                    // the hotbar slot was created for (the skill was
                                    // re-learned or copied onto a character that cannot max
                                    // it); in that case cast the level the character really
                                    // has instead of silently doing nothing.
                                    let level = SkillLevel(learned_skill.skill_level.0.min(learnable_skill.maximum_level.0));
                                    (learned_skill.skill_id, learned_skill.skill_type, level)
                                })
                        });

                    match skill {
                        Some((skill_id, skill_type, level)) => {
                            self.use_skill(Some(slot), skill_id, skill_type, level, client_tick);
                        }
                        None => {
                            if let Some(learnable_skill) =
                                self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
                            {
                                self.push_localized_chat(client_state().localization().chat_skill_not_learned(), &[(
                                    "skill",
                                    &learnable_skill.skill_id.0.to_string(),
                                )]);
                            }
                        }
                    }
                }
                InputEvent::UseSkill { skill_id, level } => {
                    // Double clicking a skill in the skill tree uses it, like in
                    // the original client. Unlearned and passive skills are
                    // ignored.
                    let skill = self
                        .client_state
                        .follow(client_state().skill_tree().skills())
                        .iter()
                        .find(|learned_skill| learned_skill.skill_id == skill_id)
                        .map(|learned_skill| (learned_skill.skill_type, SkillLevel(level.0.min(learned_skill.skill_level.0))));

                    match skill {
                        Some((skill_type, level)) => self.use_skill(None, skill_id, skill_type, level, client_tick),
                        None => self.push_localized_chat(client_state().localization().chat_skill_not_learned(), &[(
                            "skill",
                            &skill_id.0.to_string(),
                        )]),
                    }
                }
                InputEvent::CastSkill {
                    skill_id,
                    skill_type,
                    level,
                } => {
                    // The skill type comes from the skill the player selected (hotbar or
                    // skill tree); the learned list is only used for the range, with a
                    // fallback so a cast is never silently dropped just because the
                    // learned entry is missing (e.g. high-job/trimmed skill trees).
                    let attack_range = self
                        .client_state
                        .follow(client_state().skill_tree().skills())
                        .iter()
                        .find(|learned_skill| learned_skill.skill_id == skill_id)
                        .map(|learned_skill| learned_skill.attack_range)
                        .unwrap_or(AttackRange(1));

                    if crate::logging::enabled(crate::logging::LEVEL_DEBUG) {
                        crate::logging::log_debug!(
                            "cast skill {:?} level {} type {:?} range {} target {:?}",
                            skill_id,
                            level.0,
                            skill_type,
                            attack_range.0,
                            input_report.mouse_target
                        );
                    }

                    {
                        match skill_type {
                            SkillType::Passive => {
                                self.push_localized_chat(client_state().localization().chat_skill_passive(), &[(
                                    "skill",
                                    &skill_id.0.to_string(),
                                )]);
                            }
                            SkillType::Attack => {
                                if let PickerTarget::Entity(entity_id) = input_report.mouse_target
                                    && let Some(map) = &self.map
                                {
                                    let player_position =
                                        self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                                    let target_position = self
                                        .client_state
                                        .follow(client_state().entities())
                                        .iter()
                                        .find(|entity| entity.get_entity_id() == entity_id)
                                        .map(|entity| entity.get_tile_position());

                                    if let (Some(player_position), Some(target_position)) = (player_position, target_position) {
                                        if player_position
                                            .x
                                            .abs_diff(target_position.x)
                                            .max(player_position.y.abs_diff(target_position.y))
                                            <= attack_range.0
                                        {
                                            let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                        } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                            &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                            player_position,
                                            target_position,
                                            attack_range,
                                        ) && let Some(nearest_tile) = path.last()
                                        {
                                            // The target is out of range, so walk towards it and
                                            // send the cast right away. The server remembers skill
                                            // requests made while walking and casts as soon as the
                                            // target is in range, following the target's position
                                            // at every step.
                                            let _ = self.networking_system.player_move(WorldPosition {
                                                x: nearest_tile.x,
                                                y: nearest_tile.y,
                                                direction: Direction::North,
                                            });
                                            let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                        } else {
                                            self.push_localized_chat(client_state().localization().chat_skill_target_out_of_range(), &[(
                                                "skill",
                                                &skill_id.0.to_string(),
                                            )]);
                                        }
                                    } else {
                                        self.push_localized_chat(client_state().localization().chat_skill_target_gone(), &[(
                                            "skill",
                                            &skill_id.0.to_string(),
                                        )]);
                                    }
                                } else {
                                    crate::logging::log_debug!(
                                        "cast skill {:?}: no entity target or map, dropping the attack cast",
                                        skill_id
                                    );
                                    self.push_localized_chat(client_state().localization().chat_skill_no_target(), &[(
                                        "skill",
                                        &skill_id.0.to_string(),
                                    )]);
                                }
                            }
                            SkillType::Ground | SkillType::Trap => {
                                if let PickerTarget::Tile { x, y } = input_report.mouse_target {
                                    let target_position = TilePosition { x, y };

                                    if let Some(map) = &self.map {
                                        let player_position =
                                            self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());

                                        if let Some(player_position) = player_position {
                                            if player_position
                                                .x
                                                .abs_diff(target_position.x)
                                                .max(player_position.y.abs_diff(target_position.y))
                                                <= attack_range.0
                                            {
                                                let _ = self.networking_system.cast_ground_skill(skill_id, level, target_position);
                                            } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                                &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                                player_position,
                                                target_position,
                                                attack_range,
                                            ) && let Some(nearest_tile) = path.last()
                                            {
                                                // The target is out of range, so walk towards it and
                                                // send the cast right away. The server remembers
                                                // skill requests made while walking and casts as
                                                // soon as the position is in range.
                                                let _ = self.networking_system.player_move(WorldPosition {
                                                    x: nearest_tile.x,
                                                    y: nearest_tile.y,
                                                    direction: Direction::North,
                                                });
                                                let _ = self.networking_system.cast_ground_skill(skill_id, level, target_position);
                                            } else {
                                                self.push_localized_chat(client_state().localization().chat_skill_cell_out_of_range(), &[
                                                    ("skill", &skill_id.0.to_string()),
                                                ]);
                                            }
                                        } else {
                                            self.push_localized_chat(client_state().localization().chat_skill_position_unknown(), &[(
                                                "skill",
                                                &skill_id.0.to_string(),
                                            )]);
                                        }
                                    }
                                } else {
                                    crate::logging::log_debug!(
                                        "cast skill {:?}: ground skill without a tile target, dropping the cast",
                                        skill_id
                                    );
                                    self.push_localized_chat(client_state().localization().chat_skill_click_ground(), &[(
                                        "skill",
                                        &skill_id.0.to_string(),
                                    )]);
                                }
                            }
                            SkillType::SelfCast => self.cast_self_skill(skill_id, level),
                            SkillType::Support => {
                                if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                                    if let Some(map) = &self.map {
                                        let player_position =
                                            self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                                        let target_position = self
                                            .client_state
                                            .follow(client_state().entities())
                                            .iter()
                                            .find(|entity| entity.get_entity_id() == entity_id)
                                            .map(|entity| entity.get_tile_position());

                                        if let (Some(player_position), Some(target_position)) = (player_position, target_position) {
                                            if player_position
                                                .x
                                                .abs_diff(target_position.x)
                                                .max(player_position.y.abs_diff(target_position.y))
                                                <= attack_range.0
                                            {
                                                let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                            } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                                &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                                player_position,
                                                target_position,
                                                attack_range,
                                            ) && let Some(nearest_tile) = path.last()
                                            {
                                                // The target is out of range, so walk towards it and
                                                // send the cast right away. The server remembers skill
                                                // requests made while walking and casts as soon as the
                                                // target is in range, following the target's position
                                                // at every step.
                                                let _ = self.networking_system.player_move(WorldPosition {
                                                    x: nearest_tile.x,
                                                    y: nearest_tile.y,
                                                    direction: Direction::North,
                                                });
                                                let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                            } else {
                                                self.push_localized_chat(
                                                    client_state().localization().chat_skill_target_out_of_range(),
                                                    &[("skill", &skill_id.0.to_string())],
                                                );
                                            }
                                        } else {
                                            self.push_localized_chat(client_state().localization().chat_skill_target_gone(), &[(
                                                "skill",
                                                &skill_id.0.to_string(),
                                            )]);
                                        }
                                    }
                                } else {
                                    crate::logging::log_debug!(
                                        "cast skill {:?}: support skill without an entity target, casting on self",
                                        skill_id
                                    );

                                    let _ = self.networking_system.cast_skill(
                                        skill_id,
                                        level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                            }
                        }
                    }
                }
                InputEvent::StopSkill { slot } => {
                    if let Some(skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
                        && crate::world::skills::channels(skill.skill_id)
                    {
                        let _ = self.networking_system.stop_channeling_skill(skill.skill_id);
                    }
                }
                InputEvent::AddFriend { character_name } => {
                    if character_name.len() > ragnarok_packets::NAME_LENGTH {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] friend name {} is too long", "error".red(), character_name.magenta());
                    } else {
                        let _ = self.networking_system.add_friend(character_name);
                    }
                }
                InputEvent::RemoveFriend { account_id, character_id } => {
                    let _ = self.networking_system.remove_friend(account_id, character_id);
                }
                InputEvent::RejectFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.reject_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::AcceptFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.accept_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::BuyItems { items } => {
                    let _ = self.networking_system.purchase_items(items);
                }
                InputEvent::CloseShop => {
                    let _ = self.networking_system.close_shop();
                    self.shop_open = false;

                    // Clear the carts.
                    self.client_state.follow_mut(client_state().buy_cart()).clear();
                    self.client_state.follow_mut(client_state().sell_cart()).clear();

                    self.interface.close_window_with_class(WindowClass::Buy);
                    self.interface.close_window_with_class(WindowClass::BuyCart);
                    self.interface.close_window_with_class(WindowClass::Sell);
                    self.interface.close_window_with_class(WindowClass::SellCart);
                }
                InputEvent::BuyOrSell { shop_id, buy_or_sell } => {
                    let _ = self.networking_system.select_buy_or_sell(shop_id, buy_or_sell);
                    self.interface.close_window_with_class(WindowClass::BuyOrSell);
                }
                InputEvent::SellItems { items } => {
                    let _ = self.networking_system.sell_items(items);
                }
                InputEvent::StatUp { stat_type } => {
                    let _ = self.networking_system.request_stat_up(stat_type);
                }
                InputEvent::DistributePointsForSkill { skill_id } => {
                    if let Some(available_skill_points) = self.client_state.try_follow(this_player().skill_points()).copied() {
                        let job_id = self.client_state.follow(this_entity().manually_asserted()).get_job_id();
                        let pending_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .len();
                        let available_skill_points = (available_skill_points as usize).saturating_sub(pending_skill_points);

                        let skill_information = self.library.get::<SkillListInformation>(skill_id);
                        let learned_skills = self.client_state.follow(client_state().skill_tree().skills());

                        let mut new_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .clone();

                        let current_skill_level = learned_skills
                            .iter()
                            .find(|skill| skill.skill_id == skill_id)
                            .map(|skill| skill.skill_level.0)
                            .unwrap_or_default()
                            + new_skill_points
                                .iter()
                                .filter(|pending_skill_level| **pending_skill_level == skill_id)
                                .count() as u16;

                        // If the skill is already at max level we don't do anything
                        if current_skill_level < skill_information.maximum_level.0 {
                            let target_skill_level = SkillLevel(current_skill_level + 1);

                            bring_skill_to_level(
                                &mut new_skill_points,
                                &self.library,
                                learned_skills,
                                job_id,
                                skill_id,
                                target_skill_level,
                                available_skill_points,
                            );

                            *self
                                .client_state
                                .follow_mut(client_state().skill_tree_window().pending_skill_points()) = new_skill_points;
                        }
                    }
                }
                InputEvent::LevelUpSkills { skill_ids } => {
                    for skill_id in skill_ids {
                        if self.networking_system.level_up_skill(skill_id).is_err() {
                            break;
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ReloadLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    *self.client_state.follow_mut(client_state().localization()) =
                        Localization::load_language(&self.game_file_loader, language);
                }
                #[cfg(feature = "debug")]
                InputEvent::SaveLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    self.client_state.follow(client_state().localization()).save_language(language);
                }
                #[cfg(feature = "debug")]
                InputEvent::OpenMarkerDetails { marker_identifier } => {
                    if let Some(map) = &self.map {
                        match marker_identifier {
                            MarkerIdentifier::Object(key) => {
                                let inspecting_objects = self.client_state.follow_mut(client_state().inspecting_objects());
                                let object = map.get_object(key);
                                let object_path = state::prepare_object_inspection(inspecting_objects, object);

                                self.interface.open_state_window(object_path);
                            }
                            MarkerIdentifier::LightSource(key) => {
                                let inspecting_lights = self.client_state.follow_mut(client_state().inspecting_light_sources());
                                let light_source = map.get_light_source(key);
                                let light_source_path = state::prepare_light_source_inspection(inspecting_lights, light_source);

                                self.interface.open_state_window(light_source_path);
                            }
                            MarkerIdentifier::SoundSource(index) => {
                                let inspecting_sounds = self.client_state.follow_mut(client_state().inspecting_sound_sources());
                                let sound_source = map.get_sound_source(index);
                                let sound_source_path = state::prepare_sound_source_inspection(inspecting_sounds, sound_source);

                                self.interface.open_state_window(sound_source_path);
                            }
                            MarkerIdentifier::EffectSource(index) => {
                                let inspecting_effects = self.client_state.follow_mut(client_state().inspecting_effect_sources());
                                let effect_source = map.get_effect_source(index);
                                let effect_source_path = state::prepare_effect_source_inspection(inspecting_effects, effect_source);

                                self.interface.open_state_window(effect_source_path);
                            }
                            MarkerIdentifier::Particle(..) => {
                                // TODO:
                            }
                            MarkerIdentifier::Entity(index) => {
                                let entity_id = self
                                    .client_state
                                    .try_follow(client_state().entities().index(index as usize))
                                    .expect("entity should exist")
                                    .get_entity_id();

                                // This can technically still be `None`, violating the API but we handle this
                                // case in the state window.
                                let entity_path = client_state().entities().lookup(entity_id).manually_asserted();

                                self.interface.open_state_window(entity_path);
                            }
                            MarkerIdentifier::Shadow(..) => {
                                // TODO:
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleRenderOptionsWindow => match self.interface.is_window_with_class_open(WindowClass::RenderOptions) {
                    true => self.interface.close_window_with_class(WindowClass::RenderOptions),
                    false => self
                        .interface
                        .open_window(RenderOptionsWindow::new(client_state().render_options())),
                },
                #[cfg(feature = "debug")]
                InputEvent::OpenMapDataWindow => {
                    if let Some(map) = self.map.as_ref() {
                        let inspecting_maps = self.client_state.follow_mut(client_state().inspecting_maps());
                        let map_data_path = state::prepare_map_inspection(inspecting_maps, map.get_map_data());

                        self.interface.open_state_window(map_data_path);
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleClientStateInspectorWindow => {
                    match self.interface.is_window_with_class_open(WindowClass::ClientStateInspector) {
                        true => self.interface.close_window_with_class(WindowClass::ClientStateInspector),
                        false => self.interface.open_state_window_mut(client_state()),
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleMapsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Maps) {
                            true => self.interface.close_window_with_class(WindowClass::Maps),
                            false => self.interface.open_window(MapsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleCommandsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Commands) {
                            true => self.interface.close_window_with_class(WindowClass::Commands),
                            false => self.interface.open_window(CommandsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleThemeInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::ThemeInspector) {
                    true => self.interface.close_window_with_class(WindowClass::ThemeInspector),
                    false => self.interface.open_window(ThemeInspectorWindow::new(
                        client_state().theme_inspector_window(),
                        client_state().menu_theme(),
                        client_state().in_game_theme(),
                        client_state().world_theme(),
                    )),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleProfilerWindow => match self.interface.is_window_with_class_open(WindowClass::Profiler) {
                    true => self.interface.close_window_with_class(WindowClass::Profiler),
                    false => self.interface.open_window(ProfilerWindow::new(client_state().profiler_window())),
                },
                #[cfg(feature = "debug")]
                InputEvent::TogglePacketInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::PacketInspector) {
                    true => self.interface.close_window_with_class(WindowClass::PacketInspector),
                    false => self
                        .interface
                        .open_window(PacketInspectorWindow::new(client_state().packet_history())),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleCacheStatisticsWindow => match self.interface.is_window_with_class_open(WindowClass::CacheStatistics) {
                    true => self.interface.close_window_with_class(WindowClass::CacheStatistics),
                    false => self.interface.open_state_window(client_state().cache_statistics()),
                },
                #[cfg(feature = "debug")]
                InputEvent::CameraLookAround { offset } => self.debug_camera.look_around(offset),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveRight => self.debug_camera.move_right(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveUp => self.debug_camera.move_up(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                InputEvent::CameraDecelerate => self.debug_camera.decelerate(),
                #[cfg(feature = "debug")]
                InputEvent::InspectFrame { measurement } => self.interface.open_window(FrameInspectorWindow::new(measurement)),
            }
        }

        interface_has_focus
    }
}
