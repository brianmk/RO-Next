//! `NetworkEvent` handlers for the session family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::LoginServerConnected`.
    pub(crate) fn handle_login_server_connected(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::LoginServerConnected {
            character_servers,
            login_data,
        } = event
        else {
            unreachable!("handle_login_server_connected called with a different event variant");
        };

        self.play_main_menu_click_sound();

        // Remove `_m`/`_f` suffix from the username. The suffix is only for *creating*
        // an account and thus can (and needs to) be removed after the first successful
        // login.
        {
            let selected_service_path = SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
            let username_path = selected_service_path.username();

            let username = self.client_state.follow_mut(username_path);

            if let Some(stripped) = username.strip_suffix("_m") {
                *username = stripped.to_owned();
            } else if let Some(stripped) = username.strip_suffix("_f") {
                *username = stripped.to_owned();
            }
        }

        self.saved_login_data = Some(login_data);
        self.session.transition(SessionEvent::LoginSucceeded);

        // A previously established character/map connection (e.g. from a session
        // that was kicked or logged out) must not block the new server selection.
        // Only disconnect connections that actually exist: disconnecting an idle
        // connection emits a synthetic "closed by client" event, which would make
        // the disconnect handler close the server selection window we are about to
        // open and bounce the player back to the login screen.
        if self.networking_system.is_character_server_connected() {
            self.networking_system.disconnect_from_character_server();
        }

        if self.networking_system.is_map_server_connected() {
            self.networking_system.disconnect_from_map_server();
        }

        // A fresh login must not auto-reconnect to a previously selected character
        // server (map-restart recovery state): reset the retry state and the
        // remembered server so the Select Server window is shown and only the
        // player's click connects.
        self.session.character_server_disconnected();
        self.session.clear_character_reconnect();
        self.saved_character_server = None;

        *self.client_state.follow_mut(client_state().character_servers()) = character_servers;

        #[cfg(not(feature = "debug"))]
        self.interface.close_all_windows();

        #[cfg(feature = "debug")]
        self.interface.close_all_windows_except(DEBUG_WINDOWS);

        self.interface
            .open_window(ServerSelectionWindow::new(client_state().character_servers()));
    }

    /// `NetworkEvent::LoginServerConnectionFailed`.
    pub(crate) fn handle_login_server_connection_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::LoginServerConnectionFailed { message, .. } = event else {
            unreachable!("handle_login_server_connection_failed called with a different event variant");
        };

        self.networking_system.disconnect_from_login_server();

        self.interface.open_window(ErrorWindow::new(message.to_owned()));
    }

    /// `NetworkEvent::InternalError`.
    pub(crate) fn handle_internal_error(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::InternalError { message } = event else {
            unreachable!("handle_internal_error called with a different event variant");
        };

        // A client build/setup bug (e.g. duplicate packet handler).
        // Log it as an error and show the message, never panic.
        crate::logging::log_error!("internal client error: {message}");
        self.interface.open_window(ErrorWindow::new(message));
    }

    /// `NetworkEvent::LoginServerDisconnected`.
    pub(crate) fn handle_login_server_disconnected(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::LoginServerDisconnected { reason } = event else {
            unreachable!("handle_login_server_disconnected called with a different event variant");
        };

        if reason != DisconnectReason::ClosedByClient {
            // TODO: Make this an on-screen popup.
            #[cfg(feature = "debug")]
            print_debug!("Disconnection from the character server with error");

            let Some(socket_address) = self.saved_login_server_address else {
                // No address means the user never completed the login
                // flow; nothing to reconnect to.
                return;
            };
            self.networking_system.connect_to_login_server(
                self.saved_packet_version,
                socket_address,
                &self.saved_username,
                &self.saved_password,
            );
        }
    }

    /// `NetworkEvent::CharacterServerConnected`.
    pub(crate) fn handle_character_server_connected(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterServerConnected { normal_slot_count } = event else {
            unreachable!("handle_character_server_connected called with a different event variant");
        };

        crate::logging::log_debug!(
            "character server: connected ({} slots), requesting the character list",
            normal_slot_count
        );
        self.session.character_server_connected();
        if self.session.state() == SessionState::Recovering {
            self.session.transition(SessionEvent::CharacterServerConnected);
        }

        self.client_state
            .follow_mut(client_state().character_slots())
            .set_slot_count(normal_slot_count);

        let _ = self.networking_system.request_character_list();
    }

    /// `NetworkEvent::CharacterServerConnectionFailed`.
    pub(crate) fn handle_character_server_connection_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterServerConnectionFailed { message, .. } = event else {
            unreachable!("handle_character_server_connection_failed called with a different event variant");
        };

        self.session.character_server_disconnected();
        self.networking_system.disconnect_from_character_server();

        if self.map.is_none() && self.saved_login_data.is_some() {
            // The character server can be briefly unavailable (e.g. while the map
            // server restarts and the old map session is still registered).
            // Retry instead of ending up on a dead screen.
            self.session.schedule_character_reconnect(Instant::now());
        } else {
            self.interface.open_window(ErrorWindow::new(message.to_owned()));
        }
    }

    /// `NetworkEvent::CharacterServerDisconnected`.
    pub(crate) fn handle_character_server_disconnected(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterServerDisconnected { reason } = event else {
            unreachable!("handle_character_server_disconnected called with a different event variant");
        };

        self.session.character_server_disconnected();

        if reason != DisconnectReason::ClosedByClient {
            // TODO: Make this an on-screen popup.
            #[cfg(feature = "debug")]
            print_debug!("Disconnection from the character server with error");

            self.session.schedule_character_reconnect(Instant::now());
        } else if !self.networking_system.is_map_server_connected()
            && !self.networking_system.is_login_server_connected()
            && !self.interface.is_window_with_class_open(WindowClass::SelectServer)
        {
            // Only fall back to the login screen when the character server closed
            // while we are not in the middle of the login/server-selection flow.
            // During login the character server connection is intentionally dropped
            // (or never existed); a "closed by client" event must not wipe the
            // server selection window that was just opened.
            #[cfg(not(feature = "debug"))]
            self.interface.close_all_windows();

            #[cfg(feature = "debug")]
            self.interface.close_all_windows_except(DEBUG_WINDOWS);

            self.interface.open_window(LoginWindow::new(
                client_state().login_window(),
                client_state().login_settings(),
                client_state().client_info(),
            ));

            self.audio_engine
                .play_background_music_track(crate::client_config::background_music());
        }
    }

    /// `NetworkEvent::MapServerDisconnected`.
    pub(crate) fn handle_map_server_disconnected(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MapServerDisconnected { reason } = event else {
            unreachable!("handle_map_server_disconnected called with a different event variant");
        };

        // Guild emblems are only valid for the map session; the server may
        // have changed them and the rate limit should be reset as well.
        self.guild_emblems.clear();
        // Status effects belong to the map session as well.
        self.client_state.follow_mut(client_state().status_effects()).clear();
        self.client_state.follow_mut(client_state().spirit_orbs()).clear();
        self.client_state.follow_mut(client_state().mail()).clear();
        // The guild window data is only valid for this map session.
        self.client_state.follow_mut(client_state().guild()).reset();
        // Party-booking results, clan info and companion sheets are
        // map-session data too.
        self.client_state.follow_mut(client_state().party_booking()).clear();
        self.client_state.follow_mut(client_state().companion()).clear();

        if self.session.state() == SessionState::Shutdown {
            // We are quitting; the clean logout is done, don't go back to
            // the character selection screen.
            return;
        }

        if reason != DisconnectReason::ClosedByClient {
            // TODO: Make this an on-screen popup.
            #[cfg(feature = "debug")]
            print_debug!("Disconnection from the map server with error");
        }

        self.session.map_server_disconnected();
        self.return_to_character_selection();
    }

    /// `NetworkEvent::InitialStats`.
    pub(crate) fn handle_initial_stats(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::InitialStats {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
            attack,
            bonus_attack,
            magic_attack_max,
            magic_attack_min,
            defense,
            bonus_defense,
            magic_defense,
            bonus_magic_defense,
            hit,
            flee,
            perfect_dodge,
            critical,
            attack_speed,
        } = event
        else {
            unreachable!("handle_initial_stats called with a different event variant");
        };

        if let Some(player) = self.client_state.try_follow_mut(this_player()) {
            player.strength_stat_points_cost = strength_stat_points_cost;
            player.agility_stat_points_cost = agility_stat_points_cost;
            player.vitality_stat_points_cost = vitality_stat_points_cost;
            player.intelligence_stat_points_cost = intelligence_stat_points_cost;
            player.dexterity_stat_points_cost = dexterity_stat_points_cost;
            player.luck_stat_points_cost = luck_stat_points_cost;
            player.attack = attack as u32;
            player.bonus_attack = bonus_attack as u32;
            player.magic_attack_max = magic_attack_max as u32;
            player.magic_attack_min = magic_attack_min as u32;
            player.defense = defense as u32;
            player.bonus_defense = bonus_defense as u32;
            player.magic_defense = magic_defense as u32;
            player.bonus_magic_defense = bonus_magic_defense as u32;
            player.hit = hit as u32;
            player.flee = flee as u32;
            player.perfect_dodge = perfect_dodge as u32;
            player.critical = critical as u32;
            player.attack_speed = attack_speed as u32;
        }
    }

    /// `NetworkEvent::PlaySound`.
    pub(crate) fn handle_play_sound(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PlaySound { name, account_id, .. } = event else {
            unreachable!("handle_play_sound called with a different event variant");
        };

        let entity_id = EntityId(account_id.0);

        let position = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| entity.get_position());

        self.skill_sounds.play_network_sound(&name, account_id, position);
    }

    /// `NetworkEvent::DisplayEmotion`.
    pub(crate) fn handle_display_emotion(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::DisplayEmotion { entity_id, emotion } = event else {
            unreachable!("handle_display_emotion called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            && let Ok(sprite) = self.sprite_loader.get_or_load("이팩트\\emotion.spr", None)
            && let Ok(actions) = self.action_loader.get_or_load("이팩트\\emotion.act")
        {
            self.particle_holder
                .spawn_particle(Box::new(Emote::new(entity.get_position(), sprite, actions, emotion as usize)));
        }
    }

    /// `NetworkEvent::AccountId`.
    pub(crate) fn handle_account_id(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AccountId { .. } = event else {
            unreachable!("handle_account_id called with a different event variant");
        };
    }

    /// `NetworkEvent::UpdateClientTick`.
    pub(crate) fn handle_update_client_tick(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UpdateClientTick { client_tick, received_at } = event else {
            unreachable!("handle_update_client_tick called with a different event variant");
        };

        self.game_timer.set_client_tick(client_tick, received_at);
    }

    /// `NetworkEvent::ServerAnnouncement`.
    pub(crate) fn handle_server_announcement(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::ServerAnnouncement { text } = event else {
            unreachable!("handle_server_announcement called with a different event variant");
        };

        let text = text.replace(['\r', '\n'], " ");
        self.push_notification(NotificationKind::Announce, text, client_tick);
    }

    /// `NetworkEvent::PincodeState`.
    pub(crate) fn handle_pincode_state(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PincodeState {
            pincode_seed,
            account_id,
            state,
        } = event
        else {
            unreachable!("handle_pincode_state called with a different event variant");
        };

        let outcome = self
            .client_state
            .follow_mut(client_state().content_windows().pincode())
            .handle_state(pincode_seed, account_id, state);

        match outcome {
            crate::state::content_windows::PincodePromptOutcome::Open => {
                self.interface.close_window_with_class(WindowClass::Pincode);
                self.interface.close_window_with_class(WindowClass::CharacterSelection);
                self.interface
                    .open_window(PincodeWindow::new(client_state().content_windows().pincode()));
            }
            crate::state::content_windows::PincodePromptOutcome::Passed => {
                self.interface.close_window_with_class(WindowClass::Pincode);
                self.open_character_selection_if_closed();
            }
            crate::state::content_windows::PincodePromptOutcome::Unsupported(message) => {
                self.interface.close_window_with_class(WindowClass::Pincode);
                self.open_character_selection_if_closed();
                self.push_chat_message(ChatMessage::new(message.to_owned(), MessageColor::Error));
            }
        }
    }

    /// `NetworkEvent::LoggedOut`.
    pub(crate) fn handle_logged_out(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::LoggedOut = event else {
            unreachable!("handle_logged_out called with a different event variant");
        };

        // The server confirmed the logout; switch away from the map
        // music right away so there is no silent gap while the map
        // server disconnects and the menu map is loaded again.
        self.client_state.follow_mut(client_state().status_effects()).clear();
        self.client_state.follow_mut(client_state().spirit_orbs()).clear();
        self.client_state.follow_mut(client_state().mail()).clear();
        self.audio_engine
            .play_background_music_track(crate::client_config::background_music());
        self.networking_system.disconnect_from_map_server();
    }

    /// `NetworkEvent::UserCount`.
    pub(crate) fn handle_user_count(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UserCount { player_count } = event else {
            unreachable!("handle_user_count called with a different event variant");
        };

        self.push_localized_chat(client_state().localization().chat_players_online(), &[(
            "count",
            &player_count.to_string(),
        )]);
    }

    /// `NetworkEvent::MapServerBan`.
    pub(crate) fn handle_map_server_ban(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MapServerBan { error_code } = event else {
            unreachable!("handle_map_server_ban called with a different event variant");
        };

        self.push_chat_information(format_message(MessageDomain::MapServerBan, u16::from(error_code), &[]));
        self.session.map_server_disconnected();
        self.return_to_character_selection();
    }

    /// `NetworkEvent::RefuseEnter`.
    pub(crate) fn handle_refuse_enter(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RefuseEnter { error_code } = event else {
            unreachable!("handle_refuse_enter called with a different event variant");
        };

        self.push_localized_chat(client_state().localization().chat_map_server_refused_login(), &[(
            "error",
            &error_code.to_string(),
        )]);
    }

    /// `NetworkEvent::MapServerAuthFailed`.
    pub(crate) fn handle_map_server_auth_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MapServerAuthFailed { reason } = event else {
            unreachable!("handle_map_server_auth_failed called with a different event variant");
        };

        crate::logging::log_warn!("map server rejected the login (ZC_AUTH_FAIL reason {reason})");
        let message = self.render_localized(client_state().localization().error_map_server_auth_failed(), &[(
            "reason",
            &reason.to_string(),
        )]);
        self.interface.open_window(ErrorWindow::new(message));
    }

    /// `NetworkEvent::MessageString`.
    pub(crate) fn handle_message_string(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MessageString { message_id } = event else {
            unreachable!("handle_message_string called with a different event variant");
        };

        let text = match self.message_table.as_ref().and_then(|table| table.get(message_id)) {
            Some(text) => text.to_owned(),
            None => format!("Message #{message_id}"),
        };

        self.client_state
            .follow_mut(client_state().chat_messages())
            .push(ChatMessage::new(text, MessageColor::Rgb {
                red: 240,
                green: 220,
                blue: 120,
            }));
    }
}
