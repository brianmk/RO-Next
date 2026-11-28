//! `NetworkEvent` handlers for the character family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::ResurrectPlayer`.
    pub(crate) fn handle_resurrect_player(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::ResurrectPlayer { entity_id } = event else {
            unreachable!("handle_resurrect_player called with a different event variant");
        };

        self.entity_lifecycle.on_resurrect(entity_id);

        // The server only revives the character, it does not send a separate
        // stand-up packet, so restore the idle animation ourselves. Without this
        // the player stayed in the dead pose after respawning.
        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_idle(client_tick);
        }

        // If the resurrected player is us, close the resurrect window.
        if self
            .client_state
            .try_follow(this_entity())
            .is_some_and(|player| player.get_entity_id() == entity_id)
        {
            self.interface.close_window_with_class(WindowClass::Respawn);
        }
    }

    /// `NetworkEvent::PlayerStandUp`.
    pub(crate) fn handle_player_stand_up(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::PlayerStandUp { entity_id } = event else {
            unreachable!("handle_player_stand_up called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_standing(client_tick);
        }
    }

    /// `NetworkEvent::PlayerSitDown`.
    pub(crate) fn handle_player_sit_down(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::PlayerSitDown { entity_id } = event else {
            unreachable!("handle_player_sit_down called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_sitting(client_tick);
        }
    }

    /// `NetworkEvent::CharacterList`.
    pub(crate) fn handle_character_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterList { characters } = event else {
            unreachable!("handle_character_list called with a different event variant");
        };

        self.session.clear_character_reconnect();

        self.play_main_menu_click_sound();

        self.client_state
            .follow_mut(client_state().character_slots())
            .set_characters(characters);

        // While the pincode prompt is up the official client blocks
        // character selection; a selection attempt would make the char
        // server disconnect the session (`chclif_parse` only allows
        // 0x08B8/0x08BE/0x08C5/0x09A1 while the PIN is not verified).
        let pincode_visible = self.client_state.follow(client_state().content_windows().pincode()).visible;

        if !pincode_visible && !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            // TODO: this will do one unnecessary restore_focus. check
            // if that will be problematic

            #[cfg(not(feature = "debug"))]
            self.interface.close_all_windows();

            #[cfg(feature = "debug")]
            self.interface.close_all_windows_except(DEBUG_WINDOWS);

            self.open_character_selection_window();
        }

        // A previous map entry may have been aborted because the map
        // data was missing; show the reason now that we are back at
        // the character selection screen.
        if let Some(message) = self.session.take_map_load_error() {
            self.interface.open_window(ErrorWindow::new(message));
        }
    }

    /// `NetworkEvent::CharacterSelectionFailed`.
    pub(crate) fn handle_character_selection_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterSelectionFailed { message, .. } = event else {
            unreachable!("handle_character_selection_failed called with a different event variant");
        };

        crate::logging::log_debug!("character selection: server rejected the selection: {message}");
        // The map server may still be restarting. Stay on the character
        // selection screen and refresh the list so selecting the character can
        // simply be retried once the map server is back.
        let _ = self.networking_system.request_character_list();

        // Aborting a map entry (e.g. because the map data is missing) can
        // make the server reject a follow-up request. The map error is the
        // actionable message, so don't cover it with "Rejected from server".
        if !self.session.map_entry_aborted() {
            self.interface.open_window(ErrorWindow::new(message.to_owned()));
        }
    }

    /// `NetworkEvent::CharacterDeleted`.
    pub(crate) fn handle_character_deleted(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterDeleted = event else {
            unreachable!("handle_character_deleted called with a different event variant");
        };

        if let Some(character_id) = self.client_state.follow_mut(client_state().currently_deleting()).take() {
            self.client_state
                .follow_mut(client_state().character_slots())
                .remove_with_id(character_id);
        }
        self.client_state.follow_mut(client_state().delete_character_email()).clear();
        self.interface.close_window_with_class(WindowClass::CharacterDeletion);
    }

    /// `NetworkEvent::CharacterDeletionFailed`.
    pub(crate) fn handle_character_deletion_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterDeletionFailed { message, .. } = event else {
            unreachable!("handle_character_deletion_failed called with a different event variant");
        };

        *self.client_state.follow_mut(client_state().currently_deleting()) = None;
        self.interface.close_window_with_class(WindowClass::CharacterDeletion);
        self.interface.open_window(ErrorWindow::new(message.to_owned()))
    }

    /// `NetworkEvent::CharacterSelected`.
    pub(crate) fn handle_character_selected(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::CharacterSelected { login_data, .. } = event else {
            unreachable!("handle_character_selected called with a different event variant");
        };

        crate::logging::log_debug!("character selection: server accepted the character, connecting to the map server");
        self.play_main_menu_click_sound();

        let Some(saved_login_data) = self.saved_login_data.as_ref() else {
            // The character selection cannot be processed without the
            // login data. Show the error instead of panicking.
            let message = self.render_localized(client_state().localization().error_character_selection_missing_login_data(), &[
            ]);
            self.interface.open_window(ErrorWindow::new(message));
            return;
        };
        let saved_login_data = *saved_login_data;
        self.networking_system.disconnect_from_character_server();
        self.networking_system
            .connect_to_map_server(self.saved_packet_version, &saved_login_data, login_data);
        // NOTE: Nothing else must be sent to the map server until it responds:
        // rAthena requires the first read of a session to contain exactly the
        // login packet and disconnects otherwise. The client tick is part of
        // the map server login success packet, so there is no need to request
        // it here.

        let Some(character_information) = self
            .client_state
            .follow(client_state().character_slots())
            .with_id(login_data.character_id)
            .cloned()
        else {
            let message = self.render_localized(client_state().localization().error_character_selection_missing_slot(), &[]);
            self.interface.open_window(ErrorWindow::new(message));
            return;
        };

        let mut player = Entity::Player(Player::new(
            &self.library,
            saved_login_data.account_id,
            &character_information,
            client_tick,
        ));

        *self.client_state.follow_mut(client_state().player_name()) = character_information.name;

        // Remember the local ids for the guild actions (leave/expel)
        // and start the guild state of this character from scratch.
        self.client_state
            .follow_mut(client_state().guild())
            .set_local_player(saved_login_data.account_id, character_information.character_id);

        let entity_id = player.get_entity_id();
        let entity_type = player.get_entity_type();
        let entity_part_files = player.get_entity_part_files(&self.library);

        if let Some(animation_data) = self
            .async_loader
            .request_animation_data_load(entity_id, entity_type, entity_part_files)
        {
            player.set_animation_data(animation_data);
        }

        if player.has_falcon()
            && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
        {
            player.set_falcon_animation(falcon_animation);
        }

        request_equipment_layers(&self.async_loader, &mut player);

        let layout = self.async_loader.request_skill_tree_layout_load(player.get_job_id(), client_tick);
        *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
        *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
        self.client_state
            .follow_mut(client_state().skill_tree_window().chosen_skill_level())
            .clear();

        self.client_state.follow_mut(client_state().entities()).push(player);
        self.retrack_local_player();

        self.interface.close_window_with_class(WindowClass::CharacterSelection);
        self.interface.open_window(MinimapWindow);
        self.map_window_cycle = MapWindowCycle::Minimap;

        self.interface.open_window(CharacterOverviewWindow::new(
            client_state().player_name(),
            // TODO: Check that manually asserting is fine. Technically this window should only
            // be open while the player is selected.
            this_player().manually_asserted(),
            client_state().interface_settings(),
        ));
        self.interface.open_window(ChatWindow::new(
            client_state().chat_window(),
            client_state().chat_messages(),
            client_state().interface_settings(),
        ));
        self.open_hotbar_window();
        self.open_notification_hud_window();

        // Put the dialog system in a well-defined state.
        self.client_state.follow_mut(client_state().dialog_window()).end();

        self.map = None;
        // A fresh character map entry must always take the full map
        // load path; the previous map name only describes the map
        // session we just left.
        self.current_map_name = None;
        // A storage window from a previous session must never show up
        // in the new character's game state.
        self.close_storage_window();
        self.close_cart_window();
        self.close_vending_browse();
        self.close_trade();
        // The map server sends the map change packet (and with it the
        // map data we have to load) right after the login handshake.
        // If the packet stream is desynchronized that packet would be
        // lost and the client would stay on a black screen forever; the
        // session machine arms the map-entry watchdog for this case.
        self.session.begin_map_entry(Instant::now());
        self.skill_sounds.clear();
        self.skill_sound_gate.clear();
        self.client_state.follow_mut(client_state().quest_log()).clear();
        self.client_state.follow_mut(client_state().navigation()).clear();
        self.client_state.follow_mut(client_state().monster_database()).clear();
        self.client_state.follow_mut(client_state().achievement()).clear();

        self.particle_holder.clear();
        self.effect_holder.clear();
        self.level_99_aura_entities.clear();
        self.status_aura_flags.clear();
        self.environment.clear_weather();
        self.point_light_manager.clear();
        self.audio_engine.clear_ambient_sound();
    }

    /// `NetworkEvent::CharacterCreated`.
    pub(crate) fn handle_character_created(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterCreated { character_information } = event else {
            unreachable!("handle_character_created called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().character_slots())
            .add_character(character_information);

        self.interface.close_window_with_class(WindowClass::CharacterCreation);
    }

    /// `NetworkEvent::CharacterCreationFailed`.
    pub(crate) fn handle_character_creation_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterCreationFailed { message, .. } = event else {
            unreachable!("handle_character_creation_failed called with a different event variant");
        };

        self.interface.open_window(ErrorWindow::new(message.to_owned()));
    }

    /// `NetworkEvent::CharacterSlotSwitched`.
    pub(crate) fn handle_character_slot_switched(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterSlotSwitched = event else {
            unreachable!("handle_character_slot_switched called with a different event variant");
        };

        *self.client_state.follow_mut(client_state().switch_request()) = None;
    }

    /// `NetworkEvent::CharacterSlotSwitchFailed`.
    pub(crate) fn handle_character_slot_switch_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CharacterSlotSwitchFailed = event else {
            unreachable!("handle_character_slot_switch_failed called with a different event variant");
        };

        let message = self.render_localized(client_state().localization().error_character_slot_switch_failed(), &[]);
        self.interface.open_window(ErrorWindow::new(message));
    }

    /// `NetworkEvent::QuestBanner`.
    pub(crate) fn handle_quest_banner(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::QuestBanner {
            quest_id,
            active,
            objective_names,
        } = event
        else {
            unreachable!("handle_quest_banner called with a different event variant");
        };

        if active {
            let text = match objective_names.first() {
                Some(objective) => format!("Quest started: {objective}"),
                None => format!("Quest started (Quest #{quest_id})"),
            };

            self.push_notification(NotificationKind::Quest, text, client_tick);
        }
    }

    /// `NetworkEvent::UpdateStat`.
    pub(crate) fn handle_update_stat(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::UpdateStat { stat_type } = event else {
            unreachable!("handle_update_stat called with a different event variant");
        };

        // The zeny amount is displayed by the inventory window, so keep it in
        // the client state as well (the player stats do not contain it).
        if let StatType::Zeny(value) = stat_type {
            crate::logging::log_debug!("zeny update: {}", value);
            *self.client_state.follow_mut(client_state().zeny()) = Some(u64::from(value));
        }

        // rAthena 2019 has no dedicated level-up packet; the client
        // derives the level-up indicator from the base/job level
        // stat update. Only an increase is a level up (a respawn or
        // map change re-sends the same level).
        let level_up = match &stat_type {
            StatType::BaseLevel(level) => self
                .client_state
                .try_follow(this_player())
                .and_then(|player| (*level as usize > player.base_level).then_some((true, *level as u16))),
            StatType::JobLevel(level) => self
                .client_state
                .try_follow(this_player())
                .and_then(|player| (*level as usize > player.job_level).then_some((false, *level as u16))),
            _ => None,
        };

        match self.client_state.try_follow_mut(this_player()) {
            Some(player) => player.update_stat(stat_type),
            // The initial stat burst can arrive before the local player entity
            // was created; buffer it instead of dropping it.
            None => self.pending_stat_updates.push(stat_type),
        }

        if let Some((base, level)) = level_up {
            self.push_notification(
                NotificationKind::LevelUp,
                {
                    match base {
                        true => format!("Base level up! Lv {level}"),
                        false => format!("Job level up! Lv {level}"),
                    }
                },
                client_tick,
            );
        }
    }

    /// `NetworkEvent::UpdateEquippedPosition`.
    pub(crate) fn handle_update_equipped_position(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UpdateEquippedPosition { index, equipped_position } = event else {
            unreachable!("handle_update_equipped_position called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().inventory())
            .update_equipped_position(index, equipped_position);
    }

    /// `NetworkEvent::ChangeJob`.
    pub(crate) fn handle_change_job(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::ChangeJob { account_id, job_id } = event else {
            unreachable!("handle_change_job called with a different event variant");
        };

        let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id().0 == account_id.0)
        else {
            // Not every job change belongs to an entity the client knows
            // about: the population engine also changes the jobs of its
            // in-memory shell players, whose ids are not the ids the client
            // received in the spawn packets. Ignore those updates instead of
            // panicking on them.
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] ignoring job change of unknown entity {:?}",
                "warning".yellow(),
                account_id
            );
            // The entity is not in the world yet (e.g. the server sent the job
            // change while the map was still loading). There is nothing to update.
            return;
        };

        // FIX: A job change does not automatically send packets for the
        // inventory and for unequipping items. We should probably manually
        // request a full list of items and the hotbar.

        if !self.library.contains::<crate::world::SkillTreeLayout>(job_id) {
            // Population/LLM shells change their job to jobs that this
            // client's trimmed skill-tree tables do not contain. Log the
            // offending job id (harmless; the local skill tree is only
            // updated for the own character) to make future investigations
            // of missing job tables easy.
            eprintln!(
                "[skill_tree] job {:?} (entity {:?}) is not in the client skill-tree tables",
                job_id, account_id
            );
        }

        entity.set_job(&self.library, job_id);
        self.entity_lifecycle.on_job_change(entity.get_entity_id(), job_id);

        if let Some(animation_data) = self.async_loader.request_animation_data_load(
            entity.get_entity_id(),
            entity.get_entity_type(),
            entity.get_entity_part_files(&self.library),
        ) {
            entity.set_animation_data(animation_data);
        }

        if entity.has_falcon()
            && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity.get_entity_id())
        {
            entity.set_falcon_animation(falcon_animation);
        }

        request_equipment_layers(&self.async_loader, entity);

        // Only the own character's job change affects the local skill tree
        // window. Population shells send job changes for jobs that do not
        // exist in this client's skill tree tables, which used to panic when
        // the skill tree layout was looked up for them.
        if self
            .client_state
            .try_follow(this_entity())
            .is_some_and(|player| player.get_entity_id().0 == account_id.0)
        {
            let layout = self.async_loader.request_skill_tree_layout_load(job_id, client_tick);
            *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
            *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
            self.client_state
                .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                .clear();
        }
    }

    /// `NetworkEvent::ChangeHair`.
    pub(crate) fn handle_change_hair(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChangeHair { account_id, hair_id } = event else {
            unreachable!("handle_change_hair called with a different event variant");
        };

        let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id().0 == account_id.0)
        else {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] ignoring hair change of unknown entity {:?}",
                "warning".yellow(),
                account_id
            );
            return;
        };

        entity.set_hair(hair_id as usize);

        if let Some(animation_data) = self.async_loader.request_animation_data_load(
            entity.get_entity_id(),
            entity.get_entity_type(),
            entity.get_entity_part_files(&self.library),
        ) {
            entity.set_animation_data(animation_data);
        }

        if entity.has_falcon()
            && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity.get_entity_id())
        {
            entity.set_falcon_animation(falcon_animation);
        }

        request_equipment_layers(&self.async_loader, entity);
    }

    /// `NetworkEvent::AppearanceChanged`.
    pub(crate) fn handle_appearance_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AppearanceChanged {
            account_id,
            sprite_type,
            value,
        } = event
        else {
            unreachable!("handle_appearance_changed called with a different event variant");
        };

        let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id().0 == account_id.0)
        else {
            return;
        };

        let Some(outcome) = entity.apply_sprite_change(sprite_type, value) else {
            return;
        };

        if outcome.body {
            // Sprite layers changed: reload the animation data so both the
            // world sprite and the equipment paper doll show the new appearance.
            if let Some(animation_data) = self.async_loader.request_animation_data_load(
                entity.get_entity_id(),
                entity.get_entity_type(),
                entity.get_entity_part_files(&self.library),
            ) {
                entity.set_animation_data(animation_data);
            }
        }

        if outcome.weapon || outcome.shield || outcome.robe {
            // Equipping/unequipping a weapon, shield or robe does not
            // change the body sprite; only the equipment layers change.
            request_equipment_layers(&self.async_loader, entity);
        }
    }

    /// `NetworkEvent::LocalAppearanceChanged`.
    pub(crate) fn handle_local_appearance_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::LocalAppearanceChanged { sprite_type, value } = event else {
            unreachable!("handle_local_appearance_changed called with a different event variant");
        };

        let Some(entity) = self.client_state.try_follow_mut(this_entity()) else {
            return;
        };

        let Some(outcome) = entity.apply_sprite_change(sprite_type, value) else {
            return;
        };

        if outcome.body {
            // Sprite layers changed: reload the animation data so that both the
            // world sprite and the equipment paper doll show the new appearance.
            if let Some(animation_data) = self.async_loader.request_animation_data_load(
                entity.get_entity_id(),
                entity.get_entity_type(),
                entity.get_entity_part_files(&self.library),
            ) {
                entity.set_animation_data(animation_data);
            }
        }

        if outcome.weapon || outcome.shield || outcome.robe {
            request_equipment_layers(&self.async_loader, entity);
        }
    }

    /// `NetworkEvent::SetHotkeyData`.
    pub(crate) fn handle_set_hotkey_data(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SetHotkeyData { tab, hotkeys } = event else {
            unreachable!("handle_set_hotkey_data called with a different event variant");
        };

        // The 2019 server sends 4 tabs with 38 entries each. Keep all of
        // them and distinguish skill hotkeys from item hotkeys (the old
        // code treated every entry as a skill id, so item hotkeys were
        // "cast" as skills and every tab but 0 was dropped).
        let tab_index = tab.0 as usize;

        if tab_index >= HOTBAR_TAB_COUNT {
            return;
        }

        let job_id = self.client_state.try_follow(this_entity()).map(Entity::get_job_id);

        for (index, hotkey) in hotkeys.into_iter().take(HOTBAR_ENTRIES_PER_TAB).enumerate() {
            match hotkey {
                HotkeyState::Bound(hotkey) => match hotkey.hotkey_type {
                    HotkeyType::Skill => {
                        let Some(job_id) = job_id else {
                            continue;
                        };

                        let skill_id = SkillId(hotkey.item_or_skill_id as u16);

                        let mut skill = self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick);
                        skill.maximum_level.0 = hotkey.quantity_or_skill_level;

                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .apply_skill(tab_index, index, Some(skill));
                    }
                    HotkeyType::Item => {
                        self.client_state.follow_mut(client_state().hotbar()).apply_item(
                            tab_index,
                            index,
                            ItemId(hotkey.item_or_skill_id),
                            hotkey.quantity_or_skill_level,
                        );
                    }
                },
                HotkeyState::Unbound => self.client_state.follow_mut(client_state().hotbar()).clear_entry(tab_index, index),
            }
        }

        // Refresh the visible slots when the active tab was updated
        // (also for tabs other than 0).
        self.client_state.follow_mut(client_state().hotbar()).refresh_visible_slots();
    }

    /// `NetworkEvent::StatUpFailed`.
    pub(crate) fn handle_stat_up_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::StatUpFailed = event else {
            unreachable!("handle_stat_up_failed called with a different event variant");
        };

        self.push_localized_chat(client_state().localization().chat_cannot_raise_stat(), &[]);
    }

    /// `NetworkEvent::SpiritOrbs`.
    pub(crate) fn handle_spirit_orbs(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SpiritOrbs { entity_id, amount } = event else {
            unreachable!("handle_spirit_orbs called with a different event variant");
        };

        self.client_state.follow_mut(client_state().spirit_orbs()).set(entity_id, amount);
    }

    /// `NetworkEvent::AttackRangeChanged`.
    pub(crate) fn handle_attack_range_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AttackRangeChanged { attack_range } = event else {
            unreachable!("handle_attack_range_changed called with a different event variant");
        };

        if let Some(player) = self.client_state.try_follow_mut(this_player()) {
            player.attack_range = attack_range;
        }
    }

    /// `NetworkEvent::CriticalWeight`.
    pub(crate) fn handle_critical_weight(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CriticalWeight { percentage } = event else {
            unreachable!("handle_critical_weight called with a different event variant");
        };

        if let Some(player) = self.client_state.try_follow_mut(this_player()) {
            player.critical_weight_percentage = percentage;
        }
    }
}
