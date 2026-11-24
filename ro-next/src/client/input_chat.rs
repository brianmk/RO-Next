//! `InputEvent` handlers for the chat family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ToggleChatWindow`.
    pub(crate) fn handle_input_toggle_chat_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleChatWindow = event else {
            unreachable!("handle_input_toggle_chat_window called with a different event variant");
        };

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

    /// `InputEvent::ToggleChatRoomWindow`.
    pub(crate) fn handle_input_toggle_chat_room_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleChatRoomWindow = event else {
            unreachable!("handle_input_toggle_chat_room_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::ChatRoom) {
                true => self.interface.close_window_with_class(WindowClass::ChatRoom),
                false => self
                    .interface
                    .open_window(ChatRoomWindow::new(client_state().content_windows().chat_room())),
            }
        }
    }

    /// `InputEvent::CreateChatRoom`.
    pub(crate) fn handle_input_create_chat_room(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CreateChatRoom { title, limit } = event else {
            unreachable!("handle_input_create_chat_room called with a different event variant");
        };

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

    /// `InputEvent::JoinChatRoom`.
    pub(crate) fn handle_input_join_chat_room(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::JoinChatRoom { chat_id, title, password } = event else {
            unreachable!("handle_input_join_chat_room called with a different event variant");
        };

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

    /// `InputEvent::LeaveChatRoom`.
    pub(crate) fn handle_input_leave_chat_room(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LeaveChatRoom = event else {
            unreachable!("handle_input_leave_chat_room called with a different event variant");
        };

        let _ = self.networking_system.leave_chat_room();
        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .leave_locally();
        self.interface.invalidate_layouts();
    }

    /// `InputEvent::SendChatRoomMessage`.
    pub(crate) fn handle_input_send_chat_room_message(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SendChatRoomMessage { text } = event else {
            unreachable!("handle_input_send_chat_room_message called with a different event variant");
        };

        let player_name = self.client_state.follow(client_state().player_name()).clone();

        if self.networking_system.send_chat_message(&player_name, &text).is_err() {
            self.client_state
                .follow_mut(client_state().content_windows().chat_room())
                .set_error("Not connected to the map server.".to_owned());
        }
    }

    /// `InputEvent::ChatMarkupClicked`.
    pub(crate) fn handle_input_chat_markup_clicked(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ChatMarkupClicked { link } = event else {
            unreachable!("handle_input_chat_markup_clicked called with a different event variant");
        };

        match link {
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
        }
    }

    /// `InputEvent::WhisperChat`.
    pub(crate) fn handle_input_whisper_chat(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::WhisperChat { name } = event else {
            unreachable!("handle_input_whisper_chat called with a different event variant");
        };

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

    /// `InputEvent::OpenWhisper`.
    pub(crate) fn handle_input_open_whisper(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenWhisper { partner } = event else {
            unreachable!("handle_input_open_whisper called with a different event variant");
        };

        self.open_whisper_window(&partner);
        self.interface.invalidate_layouts();
    }

    /// `InputEvent::SendWhisper`.
    pub(crate) fn handle_input_send_whisper(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SendWhisper { partner, text } = event else {
            unreachable!("handle_input_send_whisper called with a different event variant");
        };

        if text.is_empty() || partner.is_empty() {
            return;
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

    /// `InputEvent::SendMessage`.
    pub(crate) fn handle_input_send_message(&mut self, event: InputEvent, _input_report: &InputReport, client_tick: ClientTick) {
        let InputEvent::SendMessage { text } = event else {
            unreachable!("handle_input_send_message called with a different event variant");
        };

        // Battle mode (`/bm`, `/bm on`, `/bm off`) is handled
        // entirely client-side and never sent to the server.
        if let Some(command) = parse_battle_mode_command(&text) {
            let enabled = match command {
                BattleModeCommand::On => Some(true),
                BattleModeCommand::Off => Some(false),
                BattleModeCommand::Toggle => None,
            };
            self.input_event_buffer.push(InputEvent::ToggleBattleMode { enabled });
            return;
        }

        // Handle special client commands.
        if text.as_str() == "/nc" {
            let auto_attack = self.client_state.follow_mut(client_state().game_settings().auto_attack());
            *auto_attack = !*auto_attack;
            return;
        }

        // Local aura style selection (`/aura <style>`). This is handled
        // entirely client-side before the slash command translation and is
        // never sent to the server.
        if let Some(style_name) = text.trim().strip_prefix("/aura").map(str::trim) {
            // Only the first word is the style, extra words are ignored.
            let style_name = style_name.split_whitespace().next().unwrap_or_default();

            match AuraStyle::from_style_name(style_name) {
                Some(style) => self.set_aura_style(style),
                None if style_name.is_empty() => self.push_localized_chat(client_state().localization().chat_aura_usage(), &[(
                    "styles",
                    AuraStyle::style_names(),
                )]),
                None => self.push_localized_chat(client_state().localization().chat_aura_unknown(), &[
                    ("style", style_name),
                    ("styles", AuraStyle::style_names()),
                ]),
            }

            return;
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

                return;
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

            return;
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

            return;
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

            return;
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

    /// `InputEvent::SendEmotion`.
    pub(crate) fn handle_input_send_emotion(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SendEmotion { emotion } = event else {
            unreachable!("handle_input_send_emotion called with a different event variant");
        };

        let _ = self.networking_system.send_emotion(emotion);
    }
}
