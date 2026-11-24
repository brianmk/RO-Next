//! `InputEvent` handlers for the character family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ViewCharacterInfo`.
    pub(crate) fn handle_input_view_character_info(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ViewCharacterInfo { name, level, guild_name } = event else {
            unreachable!("handle_input_view_character_info called with a different event variant");
        };

        self.interface.open_window(CharacterInfoWindow::new(name, level, guild_name));
    }

    /// `InputEvent::ToggleShowInterface`.
    pub(crate) fn handle_input_toggle_show_interface(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleShowInterface = event else {
            unreachable!("handle_input_toggle_show_interface called with a different event variant");
        };

        self.show_interface = !self.show_interface
    }

    /// `InputEvent::SelectCharacter`.
    pub(crate) fn handle_input_select_character(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SelectCharacter { slot } = event else {
            unreachable!("handle_input_select_character called with a different event variant");
        };

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
                return;
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

    /// `InputEvent::OpenCharacterCreationWindow`.
    pub(crate) fn handle_input_open_character_creation_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::OpenCharacterCreationWindow { slot } = event else {
            unreachable!("handle_input_open_character_creation_window called with a different event variant");
        };

        // Clear the name before opening the window.
        self.client_state.follow_mut(client_state().create_character_name()).clear();

        self.interface
            .open_window(CharacterCreationWindow::new(client_state().create_character_name(), slot))
    }

    /// `InputEvent::CreateCharacter`.
    pub(crate) fn handle_input_create_character(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CreateCharacter { slot, name } = event else {
            unreachable!("handle_input_create_character called with a different event variant");
        };

        let _ = self.networking_system.create_character(slot, name);
    }

    /// `InputEvent::OpenCharacterDeleteWindow`.
    pub(crate) fn handle_input_open_character_delete_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::OpenCharacterDeleteWindow { character_id } = event else {
            unreachable!("handle_input_open_character_delete_window called with a different event variant");
        };

        // Never keep the e-mail of a previous attempt around.
        self.client_state.follow_mut(client_state().delete_character_email()).clear();
        self.interface.open_window(CharacterDeletionWindow::new(
            character_id,
            client_state().delete_character_email(),
        ));
    }

    /// `InputEvent::DeleteCharacter`.
    pub(crate) fn handle_input_delete_character(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::DeleteCharacter { character_id, email } = event else {
            unreachable!("handle_input_delete_character called with a different event variant");
        };

        if self.client_state.follow(client_state().currently_deleting()).is_none() {
            let _ = self.networking_system.delete_character(character_id, email);
            *self.client_state.follow_mut(client_state().currently_deleting()) = Some(character_id);
            self.interface.close_window_with_class(WindowClass::CharacterDeletion);
        }
    }

    /// `InputEvent::SwitchCharacterSlot`.
    pub(crate) fn handle_input_switch_character_slot(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SwitchCharacterSlot {
            origin_slot,
            destination_slot,
        } = event
        else {
            unreachable!("handle_input_switch_character_slot called with a different event variant");
        };

        let _ = self.networking_system.switch_character_slot(origin_slot, destination_slot);
    }

    /// `InputEvent::PlayerMove`.
    pub(crate) fn handle_input_player_move(&mut self, event: InputEvent, _input_report: &InputReport, client_tick: ClientTick) {
        let InputEvent::PlayerMove { destination } = event else {
            unreachable!("handle_input_player_move called with a different event variant");
        };

        if self.movement_blocked_by_cast(client_tick) {
            // Classic cast rooting: while the local player is in a
            // server-confirmed cast with a cast time, walk requests
            // are ignored (the server would ignore them anyway).
            // Instant skills never reach this branch because they
            // do not start a cast bar.
            return;
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

    /// `InputEvent::PlayerInteract`.
    pub(crate) fn handle_input_player_interact(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::PlayerInteract { entity_id } = event else {
            unreachable!("handle_input_player_interact called with a different event variant");
        };

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
}
