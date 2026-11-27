//! `NetworkEvent` handlers for the world family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::AddEntity`.
    pub(crate) fn handle_add_entity(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::AddEntity { entity_data } = event else {
            unreachable!("handle_add_entity called with a different event variant");
        };

        let dead_sit = entity_data.dead_sit;

        if let Some(map) = &self.map
            && let Some(npc) = Npc::new(&self.library, map, &mut self.path_finder, entity_data, client_tick)
        {
            let mut npc = Entity::Npc(npc);

            // The idle-unit appear packet carries the spawn-time dead/sit
            // pose (`state`: 0 standing, 1 dead, 2 sitting). Only players
            // use it in practice.
            if dead_sit != 0 && npc.get_entity_type() == EntityType::Player {
                match dead_sit {
                    1 => npc.set_dead(client_tick),
                    2 => npc.set_sitting(client_tick),
                    _ => {}
                }
            }

            let entity_id = npc.get_entity_id();
            let entity_type = npc.get_entity_type();
            let entity_part_files = npc.get_entity_part_files(&self.library);

            // Explicit lifecycle bookkeeping: the server re-sends
            // `AddEntity` for entities that are already visible (job
            // change, respawn). The machine records the spawn and the
            // client replaces the old entity instead of duplicating it.
            let add_decision = self.entity_lifecycle.on_add_entity(entity_id);
            let spawn_outcome = self.entity_lifecycle.on_spawned(entity_id, npc.get_job_id());

            if spawn_outcome.previous_state.is_some() || spawn_outcome.job_changed {
                crate::logging::log_debug!(
                    "entity: replacing already visible entity {} (previous state {:?}, job changed {})",
                    entity_id.0,
                    spawn_outcome.previous_state,
                    spawn_outcome.job_changed
                );
            }

            let entities = self.client_state.follow_mut(client_state().entities());

            // If the entity was already visible, we use its old alpha value.
            if matches!(add_decision, AddEntityDecision::Replace { .. })
                && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id)
            {
                npc.inherit_fade_state(entity, client_tick);
            };

            // Remove the old copy of a replaced entity so it never
            // exists twice.
            entities.retain(|entity| entity.get_entity_id() != entity_id);

            if let Some(animation_data) = self
                .async_loader
                .request_animation_data_load(entity_id, entity_type, entity_part_files)
            {
                npc.set_animation_data(animation_data);
            }

            if npc.has_falcon()
                && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
            {
                npc.set_falcon_animation(falcon_animation);
            }

            request_equipment_layers(&self.async_loader, &mut npc);

            #[cfg(feature = "debug")]
            npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

            entities.push(npc);
        }
    }

    /// `NetworkEvent::RemoveEntity`.
    pub(crate) fn handle_remove_entity(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::RemoveEntity { entity_id, reason } = event else {
            unreachable!("handle_remove_entity called with a different event variant");
        };

        self.chat_bubbles.remove_entity(entity_id);
        self.skill_name_texts.remove_entity(entity_id);
        self.cast_bars.remove_entity(entity_id);
        self.skill_sounds.remove_entity(entity_id);
        self.skill_sound_gate.clear_entity(entity_id);
        self.client_state.follow_mut(client_state().spirit_orbs()).remove(entity_id);

        let remove_outcome = self.entity_lifecycle.on_remove_entity(entity_id, &reason);
        match remove_outcome {
            RemoveEntityOutcome::Known { state, died } => {
                crate::logging::log_debug!(
                    "entity lifecycle: remove entity {} -> {:?} (died: {})",
                    entity_id.0,
                    state,
                    died
                );
            }
            RemoveEntityOutcome::Unknown => {
                crate::logging::log_debug!("entity lifecycle: remove for untracked entity {}", entity_id.0);
            }
        }

        // The entity is gone; pending visual loads for it are moot.
        self.asset_retries.forget(&LoaderId::AnimationData(entity_id));
        self.asset_retries.forget(&LoaderId::FalconAnimationData(entity_id));
        self.asset_retries.forget(&LoaderId::WeaponAnimationData(entity_id));
        self.asset_retries.forget(&LoaderId::ShieldAnimationData(entity_id));
        self.asset_retries.forget(&LoaderId::RobeAnimationData(entity_id));

        // Classic teleport/logout behaviour: entities that leave via
        // teleport/logout play a warp effect and sound at their last
        // position. Out-of-sight and death removals stay plain.
        let disappearance_position = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| entity.get_position());

        if matches!(reason, DisappearanceReason::Teleported | DisappearanceReason::LoggedOut)
            && let Some(position) = disappearance_position
        {
            self.spawn_teleport_vanish(position);
        }

        // If the motive is dead, you need to set the player to dead.
        if reason == DisappearanceReason::Died {
            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
            {
                let entity_type = entity.get_entity_type();

                if entity_type == EntityType::Monster {
                    let mut entity = entity.clone();
                    entity.set_dead(client_tick);
                    entity.stop_movement();

                    // Remove the entity from the list of alive entities.
                    self.client_state
                        .follow_mut(client_state().entities())
                        .retain(|entity| entity.get_entity_id() != entity_id);

                    // Add the entity to the list of dead entities.
                    self.client_state.follow_mut(client_state().dead_entities()).push(entity);
                } else if entity_type == EntityType::Player {
                    entity.set_dead(client_tick);

                    // If the player is us, we need to open the respawn window.
                    if entity_id == self.client_state.follow(client_state().entities())[0].get_entity_id() {
                        self.interface.open_window(RespawnWindow);
                    }
                }
            }
        } else {
            // For non-death disappearances, start fading out the entity.
            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
            {
                entity.fade_out(reason, client_tick);
            }
        }

        // If the entity that was removed had an attack buffered we remove the entity
        // from the buffer.
        let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
        if buffered_action.is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id)) {
            *buffered_action = None;
        }
    }

    /// `NetworkEvent::EntityMove`.
    pub(crate) fn handle_entity_move(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::EntityMove {
            entity_id,
            origin,
            destination,
            starting_timestamp,
        } = event
        else {
            unreachable!("handle_entity_move called with a different event variant");
        };

        // Clone the dynamic cell overrides before borrowing the
        // entity list mutably (breakable walls must affect the walk
        // prediction too).
        let cell_overrides = self.client_state.follow(client_state().map_cell_overrides()).clone();
        let entities = self.client_state.follow_mut(client_state().entities());
        let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

        if let Some(entity) = entity
            && let Some(map) = &self.map
        {
            entity.move_from_to(
                map,
                &mut self.path_finder,
                &cell_overrides,
                origin.tile_position(),
                destination.tile_position(),
                starting_timestamp,
            );
            #[cfg(feature = "debug")]
            entity.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
        }
    }

    /// `NetworkEvent::PlayerMove`.
    pub(crate) fn handle_player_move(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PlayerMove {
            origin,
            destination,
            starting_timestamp,
        } = event
        else {
            unreachable!("handle_player_move called with a different event variant");
        };

        let cell_overrides = self.client_state.follow(client_state().map_cell_overrides()).clone();

        if let Some(map) = &self.map
            && let Some(player) = self.client_state.try_follow_mut(this_entity())
        {
            player.move_from_to(
                map,
                &mut self.path_finder,
                &cell_overrides,
                origin.tile_position(),
                destination.tile_position(),
                starting_timestamp,
            );
            #[cfg(feature = "debug")]
            player.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
        }
    }

    /// `NetworkEvent::ChangeMap`.
    pub(crate) fn handle_change_map(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::ChangeMap { map_name, position } = event else {
            unreachable!("handle_change_map called with a different event variant");
        };

        // A same-map "map move" is a teleport (fly wing, AL_TELEPORT,
        // random warp, ...). The server sends the normal map move packet
        // with the current map name; reloading the map for it would take
        // long and hide the teleport, so reposition the player directly,
        // play the warp effect/sound and let the server spawn the
        // surrounding entities again.
        if is_same_map_teleport(
            self.current_map_name.as_deref(),
            map_name.as_str(),
            self.map.is_some(),
            self.session.state(),
        ) {
            let old_position = self.client_state.try_follow(this_entity()).map(|entity| entity.get_position());

            // Drop the transient state of the old position.
            self.client_state.follow_mut(client_state().entities()).truncate(1);
            self.client_state.follow_mut(client_state().dead_entities()).clear();
            self.client_state.follow_mut(client_state().ground_items()).clear();
            *self.client_state.follow_mut(client_state().buffered_action()) = None;
            // A teleport is a warp as well: everything but the player is
            // re-spawned by the server, so drop the entity bookkeeping and
            // pending visual loads of the old position. Status effects stay
            // (a teleport must not remove buffs).
            self.entity_lifecycle.on_warp();
            self.asset_retries.clear();
            self.retrack_local_player();
            self.particle_holder.clear();
            self.effect_holder.clear();
            // The aura tracker must be reset together with the effects:
            // the player entity survives the fast-path teleport, but its
            // aura effect was cleared above. Keeping the id in the tracker
            // would make `update_level_99_auras` skip re-creating it.
            self.level_99_aura_entities.clear();
            self.chat_bubbles.clear();
            self.cast_bars.clear();
            self.skill_name_texts.clear();

            if let Some(position) = old_position {
                self.spawn_teleport_vanish(position);
            }

            let Some(map) = self.map.as_mut() else {
                return;
            };
            let player = self.client_state.follow_mut(this_entity().manually_asserted());
            player.set_position(map, position, client_tick);
            self.player_camera.set_focus_point(player.get_position());

            // Ask the server for the entities around the new position
            // (same load-end acknowledgement as after a real map change).
            let _ = self.networking_system.map_loaded();
            return;
        }

        self.map = None;
        self.skill_sounds.clear();
        self.skill_sound_gate.clear();
        self.client_state.follow_mut(client_state().quest_log()).clear();
        self.client_state.follow_mut(client_state().navigation()).clear();
        self.client_state.follow_mut(client_state().monster_database()).clear();
        self.client_state.follow_mut(client_state().achievement()).clear();
        // The server closes the storage on a map change; close the UI
        // and drop the transient list immediately so it can't go stale.
        self.close_storage_window();
        self.close_cart_window();
        self.close_vending_browse();
        self.close_trade();
        self.minimap_last_tile = None;
        self.client_state.follow_mut(client_state().minimap()).clear();
        self.client_state.follow_mut(client_state().spirit_orbs()).clear();
        self.particle_holder.clear();
        self.effect_holder.clear();
        self.level_99_aura_entities.clear();
        self.status_aura_flags.clear();
        if let Some(weather) = self.weather.as_mut() {
            weather.clear();
        }
        self.chat_bubbles.clear();
        self.pending_item_drag = None;
        self.pending_item_drag = None;
        self.skill_name_texts.clear();
        self.cast_bars.clear();
        self.aftercast_act_until = None;
        self.warp_skill_units.clear();
        self.point_light_manager.clear();
        self.audio_engine.clear_ambient_sound();

        // Dynamic cell overrides and the map properties belong to the
        // old map; the server re-sends them after the map entry.
        self.client_state.follow_mut(client_state().map_cell_overrides()).clear();
        *self.client_state.follow_mut(client_state().map_type()) = 0;
        *self.client_state.follow_mut(client_state().map_flags()) = 0;

        // Only the player must stay alive between map changes.
        self.client_state.follow_mut(client_state().entities()).truncate(1);
        self.client_state.follow_mut(client_state().dead_entities()).clear();
        self.client_state.follow_mut(client_state().ground_items()).clear();
        *self.client_state.follow_mut(client_state().buffered_action()) = None;

        // Everything else belongs to the old map session: forget the
        // entity bookkeeping and pending visual loads. The local
        // player's status effects are kept: the server only re-sends
        // statuses with the `DisplayPc`/`DisplayNpc` flag when the
        // player enters the new map, so clearing all of them would make
        // timed buffs (Assumptio, Blessing, ...) vanish from the buff
        // bar and lose their world aura until they are recast. Timed
        // statuses still expire locally and an end packet removes them;
        // statuses of other entities are dropped with the old session.
        self.entity_lifecycle.on_warp();
        self.asset_retries.clear();
        let own_entity_id = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id().0);
        if let Some(own_entity_id) = own_entity_id {
            self.client_state
                .follow_mut(client_state().status_effects())
                .retain_for_entity(own_entity_id);
        } else {
            self.client_state.follow_mut(client_state().status_effects()).clear();
        }
        self.retrack_local_player();

        // Close any remaining dialogs.
        self.interface.close_window_with_class(WindowClass::Dialog);

        self.session.map_change_received(map_name.clone(), Some(position), Instant::now());
        self.async_loader.request_map_load(map_name, Some(position));
    }

    /// `NetworkEvent::StatusEffect`.
    pub(crate) fn handle_status_effect(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::StatusEffect {
            entity_id,
            status_index,
            active,
            total_duration_in_milliseconds,
            remaining_in_milliseconds,
            values,
        } = event
        else {
            unreachable!("handle_status_effect called with a different event variant");
        };

        // Status effects of all entities are tracked (the server sends
        // area status packets); the buff bar only displays the player's
        // own effects.
        let definition = status_effect_definition(status_index);
        let icon = match (active, definition.icon_path()) {
            (true, Some(path)) => match self.texture_loader.get_or_load(&path, ImageType::Color) {
                Ok(icon) => Some(icon),
                Err(error) => {
                    crate::logging::log_warn!("buffbar: failed to load status icon '{}': {:?}", path, error);
                    None
                }
            },
            _ => None,
        };

        if active && definition.icon.is_none() {
            crate::logging::log_debug!(
                "status: status effect {} ({}) has no icon mapping",
                status_index,
                definition.name
            );
        }

        let client_tick = client_tick.0 as u64;
        self.client_state.follow_mut(client_state().status_effects()).apply(
            entity_id.0,
            status_index,
            active,
            total_duration_in_milliseconds,
            remaining_in_milliseconds,
            values,
            client_tick,
            definition.name.to_owned(),
            icon,
        );

        // Data-driven body visuals (tints/alpha); the table lives in
        // `archive/data/client/status_visual_effects.ron`.
        self.refresh_entity_status_visual(entity_id);

        // `EFST_CHANGE_SIZE` / `EFST_CHANGE_SIZE_MONSTER`
        // (1420/1421): the server `val1` is the size in percent.
        if matches!(status_index, 1420 | 1421)
            && let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_status_scale(active, values[0]);
        }

        // `EFST_PUSH_CART` (673) keeps the merchant cart visible even
        // when the server's entity option bit is missing.
        if status_index == 673
            && let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_cart_status(active, values[0]);
        }
    }

    /// `NetworkEvent::ChatMessage`.
    pub(crate) fn handle_chat_message(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChatMessage { text, color } = event else {
            unreachable!("handle_chat_message called with a different event variant");
        };

        // The server-side aura stylist NPC asks the client to change its
        // local aura style with a private chat marker. Apply the style and
        // hide the marker message from the chat window; normal chat is
        // passed through untouched.
        if let Some(marker) = parse_aura_style_marker(&text) {
            match marker {
                AuraStyleMarker::Style(style) => self.set_aura_style(style),
                AuraStyleMarker::UnknownStyle => {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] aura stylist NPC requested an unknown aura style", "error".red());
                }
            }

            return;
        }

        // Server announcements are a separate `ServerAnnouncement`
        // event (broadcast packets); a `Broadcast` color alone is
        // also used for normal player chat, so it must not open the
        // announce box.
        // While a chat room is open the same line is also shown in the
        // room window (own lines arrive as `ZC_NOTIFY_PLAYERCHAT`,
        // other members as `ZC_NOTIFY_CHAT`).
        if self.client_state.follow(client_state().content_windows().chat_room()).is_in_room() {
            self.client_state
                .follow_mut(client_state().content_windows().chat_room())
                .push_message(text.clone(), false);
            self.interface.invalidate_layouts();
        }

        self.client_state
            .follow_mut(client_state().chat_messages())
            .push(ChatMessage::new(text, color));
    }

    /// `NetworkEvent::OverheadChatMessage`.
    pub(crate) fn handle_overhead_chat_message(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::OverheadChatMessage { entity_id, text, color } = event else {
            unreachable!("handle_overhead_chat_message called with a different event variant");
        };

        if self.client_state.follow(client_state().content_windows().chat_room()).is_in_room() {
            self.client_state
                .follow_mut(client_state().content_windows().chat_room())
                .push_message(text.clone(), false);
            self.interface.invalidate_layouts();
        }

        self.chat_bubbles.push(entity_id, text, color, client_tick);
    }

    /// `NetworkEvent::UpdateEntityDetails`.
    pub(crate) fn handle_update_entity_details(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UpdateEntityDetails {
            entity_id,
            name,
            guild_name,
        } = event
        else {
            unreachable!("handle_update_entity_details called with a different event variant");
        };

        let entity = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id);

        let mut guild_id_to_request = None;

        if let Some(entity) = entity {
            entity.set_details(name);
            // The player details response also contains the guild name;
            // an empty name removes the guild label again (for example
            // after a player left their guild).
            entity.set_guild_name(guild_name);

            // The guild label is only known now, request the emblem image
            // for it (one request per guild per map session).
            if entity.get_guild_name().is_some() {
                guild_id_to_request = Some(entity.get_guild_id()).filter(|guild_id| *guild_id != 0);
            }
        }

        if let Some(guild_id) = guild_id_to_request
            && self.guild_emblems.request_if_needed(guild_id)
        {
            let _ = self.networking_system.request_guild_emblem(guild_id);
        }

        // The name was requested because the player right-clicked the
        // character before its name was loaded. Open the menu now.
        if let Some((pending_entity_id, position)) = self.pending_context_menu
            && pending_entity_id == entity_id
        {
            self.pending_context_menu = None;

            if open_character_context_menu(&self.client_state, &mut self.interface, entity_id, position) {
                let _ = self.networking_system.entity_details(entity_id);
            }
        }
    }

    /// `NetworkEvent::DamageEffect`.
    pub(crate) fn handle_damage_effect(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::DamageEffect {
            source_entity_id,
            destination_entity_id,
            damage_amount,
            damage_amount_2,
            hit_count,
            attack_duration,
            is_critical,
            blue_damage,
            skill_id,
            skill_start_tick: _,
        } = event
        else {
            unreachable!("handle_damage_effect called with a different event variant");
        };

        let target_position = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == destination_entity_id)
            .map(|entity| entity.get_tile_position());

        // A skill damage packet without a damage value is a miss/block: the
        // client spawns no number for it. Log it at debug level so a missing
        // damage number can be told apart from a missing packet.
        if let Some(skill_id) = skill_id
            && damage_amount.is_none()
        {
            crate::logging::log_debug!("damage skill: skill {} damage packet carried no damage", skill_id.0);
        }

        // Normal (auto) attacks: the official client plays the weapon/job
        // impact wave and spawns the target hit effect from the damage message
        // (`GameObject.cpp:1780-1885`). Skill damage packets (`skill_id`) have
        // their own sound/effect path above.
        if skill_id.is_none() {
            self.play_auto_attack_hit_feedback(
                source_entity_id,
                destination_entity_id,
                damage_amount.is_some(),
                is_critical,
                client_tick,
                0,
            );

            // Dual wield: the packet's `damage2` is a second damage message of
            // the same attack (`RenderObject.cpp:3700`, `:3856-3877`) with its
            // own impact wave and hit effect. It is queued until
            // `message.time`, i.e. 350 ms after the main-hand message
            // (`:3870`, `:4712-4721`).
            if damage_amount_2.is_some() {
                self.play_auto_attack_hit_feedback(
                    source_entity_id,
                    destination_entity_id,
                    true,
                    is_critical,
                    client_tick,
                    crate::world::dual_wield_hit_delay_ms(),
                );
            }
        }

        // Falcon attack flight (`CGameActor::statusEffects[0]`): the official
        // client sends `AM_REQ_ATTACK` to the falcon actor for
        // `IsFalconAttackSkill()` ids (Blitz Beat, Falcon Assault and the hawk
        // skills); `ZC_NOTIFY_SKILL` is the only packet carrying a skill id.
        // The falcon must already be summoned (`OPTION_FALCON`); otherwise
        // there is no actor to move.
        if let Some(skill_id) = skill_id
            && crate::world::is_falcon_attack_skill(skill_id)
            && let Some(target_world_position) =
                target_position.and_then(|tile_position| self.map.as_ref()?.get_world_position(tile_position))
        {
            let entities = self.client_state.follow_mut(client_state().entities());
            if let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == source_entity_id)
                && entity.has_falcon()
            {
                // The animation load is lazy (requested when the option first
                // appears); the damage packet can arrive while it is pending.
                if !entity.has_falcon_animation()
                    && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(source_entity_id)
                {
                    entity.set_falcon_animation(falcon_animation);
                }

                entity.start_falcon_flight(target_world_position, client_tick);
            }
        }

        // Instant damage skills (e.g. Bash, Magnum Break) send no
        // completion packet: their acknowledged cast waits for this hit
        // packet to play the official skill sound (so it is heard at the
        // impact, not as a pre-cast sound). Cast-time damage skills are
        // unchanged.
        if let Some(skill_id) = skill_id {
            // Instant skills have a pending cast and claim the hit
            // sound here. Cast-time damage skills (e.g. Asura
            // Strike) have no completion/no-damage packet, so their
            // mapped hit/effect sound has to be claimed from the
            // damage packet as well.
            let instant_claimed = self.skill_sound_gate.claim_damage_hit(source_entity_id, skill_id, client_tick);
            if instant_claimed || self.skill_sound_gate.claim_completion(source_entity_id, skill_id, client_tick) {
                let source_position = self
                    .client_state
                    .follow(client_state().entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == source_entity_id)
                    .map(|entity| entity.get_position());
                self.skill_sounds
                    .play_skill_sound_for_phase(skill_id, SkillPhase::Hit, Some(source_entity_id), source_position);
            }
        }

        // Skill damage attribution (`ZC_NOTIFY_SKILL`, 0x01DE): play
        // the official release/hit effect at the damaged target and the
        // caster effect on the caster. Attack packets carry no skill id,
        // so auto attacks can never trigger skill effects.
        if let Some(skill_id) = skill_id
            && self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(source_entity_id)
        {
            let target_world_position =
                target_position.and_then(|tile_position| self.map.as_ref().and_then(|map| map.get_world_position(tile_position)));
            if !crate::world::skill_has_effect(skill_id) {
                crate::logging::log_debug!("damage skill: skill {} has no effect mapping", skill_id.0);
            }

            // Hand-written and official layers are no longer mutually
            // exclusive: every phase of the unified descriptor table
            // spawns, at the damaged target and on the caster.
            for phase in [SkillPhase::Success, SkillPhase::Release, SkillPhase::Hit, SkillPhase::Caster] {
                self.spawn_skill_effect_phase(
                    skill_id,
                    phase,
                    source_entity_id,
                    Some(destination_entity_id),
                    target_world_position,
                    client_tick,
                    0.0,
                );
            }
        }

        // Auto attack logic.
        if self
            .client_state
            .try_follow(this_entity())
            .is_some_and(|player| player.get_entity_id() == source_entity_id)
        {
            let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
            let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

            if let Some(BufferedAction::AttackEntity { entity_id }) = *buffered_action {
                let _ = self.networking_system.player_attack(entity_id);

                if !auto_attack {
                    *buffered_action = None;
                }
            }
        }

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == source_entity_id)
        // TODO: Maybe also or_else this_entity?
        {
            if let Some(target_position) = target_position {
                entity.rotate_towards(target_position);
            }

            // Skill damage (`ZC_NOTIFY_SKILL`, 0x01DE) plays the official
            // damage pose: `CGameActor::ProcessSkillMessage` reads
            // `GetSkillActionInfo2` and then applies the per-skill
            // `SetFrameSequence` (`RenderObject.cpp:5522-5527`, `:5794-5812`).
            // The sequence names the ACT block itself (block 96 for Occult
            // Impaction/Throw Spirit Sphere), so it must replace the generic
            // weapon swing instead of sharing it.
            let skill_sequence = skill_id.and_then(crate::world::frame_sequence_for_skill);
            if let (Some(skill_id), Some(sequence)) = (skill_id, skill_sequence) {
                match caster_action(skill_id) {
                    CasterAction::Action(action_index) => entity.set_custom_skill_action(action_index as usize, 0, false, client_tick),
                    CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                    CasterAction::Attack => entity.set_attack(0, false, client_tick),
                    CasterAction::Stand => entity.set_stand(client_tick),
                    CasterAction::Idle => entity.set_idle(client_tick),
                    CasterAction::Pickup => entity.set_pickup(client_tick),
                    CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                    CasterAction::Keep => {}
                }

                entity.set_skill_frame_sequence_with_rate(
                    sequence.first,
                    sequence.last,
                    sequence.loop_to,
                    sequence.duration_ms,
                    sequence.step_ms(),
                    client_tick,
                );
            } else {
                entity.set_attack(attack_duration, is_critical, client_tick);
            }

            // A damage-carrying skill packet also resolves the monster
            // skill-motion table (`CGameMode::ReceiveActorSkill`,
            // `GameModePackets.cpp:4281-4286`; the old-layout
            // `ReceiveActorSkillAction` and the position variant at `:4356` /
            // `:4431` do the same): after the skill message the official calls
            // `GetMonsterSkillInfo` and `SetAction(action * 8 - 8, 1, loopType
            // == 0)`. `SetAction` clears the skill's frame sequence
            // (`RenderObject.cpp:527`), so the monster motion replaces the
            // generic damage pose. Only monsters have table rows, so player
            // casters are unchanged.
            if let Some(monster_action) =
                skill_id.and_then(|skill_id| super::skills::monster_skill_action_for_entity(&self.library, entity, skill_id))
            {
                entity.set_monster_action(monster_action.action_index as usize, monster_action.looping, client_tick);
            }
        }

        if let Some(entity) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == destination_entity_id)
            .or_else(|| self.client_state.try_follow(this_entity()))
        {
            // The official client splits a multi-hit damage packet into one
            // number per hit (`CGameActor::ProcessSkillMessage`):
            // `damage /= hitCount` and `hitCount` damage messages spaced
            // `g_multiHitInterval` = 200 ms apart (`RenderObject.cpp:100`,
            // `:5367-5368`, `:5931-5947`). Aegis-style Throw Spirit Sphere
            // sends one packet with `div = skill level`, so this is what makes
            // every hit visible. A `div` of 0 or above 36 is played as one hit,
            // exactly like the official guards (`:5359-5364`).
            let hit_count = match hit_count {
                0 => 1,
                count if count > 36 => 1,
                count => usize::from(count),
            };

            // `g_msgEffectShowSpecial` gates every number type in
            // `CMsgEffect::Render` (`MsgEffect.cpp:1586-1602`), including the
            // miss sprite. The hit feedback and the damage pose are unaffected.
            let show_damage_numbers = *self.client_state.follow(client_state().game_settings().show_damage_numbers());
            let target_is_player =
                self.client_state.try_follow(this_entity()).map(|player| player.get_entity_id()) == Some(destination_entity_id);
            // `AM_MAKE_MISS` uses white for monster attackers and red for
            // players (`GameObject.cpp:1774-1779`).
            let source_is_monster = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == source_entity_id)
                .is_some_and(|entity| entity.get_entity_type() == EntityType::Monster);

            let direction = entity.get_direction();
            let position = entity.get_position();

            if show_damage_numbers {
                match damage_amount {
                    Some(amount) => {
                        // `damage /= hitCount` (`RenderObject.cpp:5368`).
                        let per_hit = amount / hit_count;

                        for hit in 0..hit_count {
                            self.particle_holder.spawn_particle(Box::new(DamageNumber::damage(
                                destination_entity_id,
                                position,
                                per_hit as u64,
                                is_critical,
                                blue_damage,
                                target_is_player,
                                direction,
                                hit as u32 * crate::world::MULTI_HIT_INTERVAL_MS,
                            )));
                        }
                    }
                    None => {
                        let color = match source_is_monster {
                            true => DamageColor::White,
                            false => DamageColor::PlayerRed,
                        };
                        self.particle_holder.spawn_particle(Box::new(DamageNumber::miss(
                            destination_entity_id,
                            position,
                            color,
                            direction,
                        )));
                    }
                }

                // The off-hand value of a dual-wield attack is a separate
                // damage message (`message.damage = extraDamage`,
                // `RenderObject.cpp:3871`). The official colour/style only
                // depends on the message (`GameObject.cpp:1889-1943` selects
                // it from the target/attacker, never from the hit index), so
                // the second number uses the same style and colour; only its
                // `message.time` differs (`:3870`).
                if let Some(amount_2) = damage_amount_2 {
                    self.particle_holder.spawn_particle(Box::new(DamageNumber::damage(
                        destination_entity_id,
                        position,
                        amount_2 as u64,
                        is_critical,
                        blue_damage,
                        target_is_player,
                        direction,
                        crate::world::dual_wield_hit_delay_ms(),
                    )));
                }
            }
        }

        // Classic auto-attack projectile (arrows): the weapon data
        // file decides which weapons fire one, so the client loop
        // has no hard-coded bow type / item view id range. Only auto
        // attacks (attack packets carry no skill id) spawn it.
        if skill_id.is_none()
            && let Some(target_tile) = target_position
        {
            let attacker = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == source_entity_id)
                .map(|entity| (entity.get_weapon(), entity.get_position()));

            if let Some((weapon, attacker_position)) = attacker
                && let Some(projectile) = crate::world::weapon::projectile_for_weapon(weapon)
                && let Some(target_world_position) = self.map.as_ref().and_then(|map| map.get_world_position(target_tile))
            {
                self.ensure_projectile_assets(projectile.kind());

                if let Some(Some(projectile_assets)) = self.projectile_assets.get(&projectile.kind()).cloned() {
                    crate::logging::log_debug!(
                        "projectile: spawning {:?} from {:?} to {:?}",
                        projectile.kind(),
                        attacker_position,
                        target_world_position
                    );
                    // The arrow is a world-space quad rotated along
                    // its flight vector (the item sprite has no
                    // per-direction frames). The official impact wave for
                    // a bow is `_hit_arrow.wav` from the damage message
                    // (`GetWeaponHitWaveName`, `HitSounds.cpp:73-76`,
                    // played above by `play_auto_attack_hit_feedback`), so
                    // the visual must not play it a second time on landing.
                    if let Some(texture) = projectile_assets.sprite.textures.first().cloned() {
                        self.effect_holder.add_effect(Box::new(ArrowEffect::new(
                            texture,
                            attacker_position,
                            target_world_position,
                            projectile.flight_seconds(),
                            None,
                            Some(self.audio_engine.clone()),
                        )));
                    }
                }
            }
        }
    }

    /// `NetworkEvent::HealEffect`.
    pub(crate) fn handle_heal_effect(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::HealEffect { entity_id, heal_amount } = event else {
            unreachable!("handle_heal_effect called with a different event variant");
        };

        // `ZC_RECOVERY` has no entity id (the server only sends it to the
        // player themselves), so `None` resolves to the local player.
        let owner = entity_id.or_else(|| self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()));
        let position = owner
            .and_then(|entity_id| {
                self.client_state
                    .follow(client_state().entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == entity_id)
                    .map(|entity| entity.get_position())
            })
            .or_else(|| self.client_state.try_follow(this_entity()).map(|entity| entity.get_position()));

        if let (Some(owner), Some(position)) = (owner, position) {
            self.particle_holder
                .spawn_particle(Box::new(DamageNumber::heal(owner, position, heal_amount as u64)));
        }
    }

    /// `NetworkEvent::ChangeDirection`.
    pub(crate) fn handle_change_direction(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ChangeDirection {
            entity_id,
            head_direction,
            direction,
        } = event
        else {
            unreachable!("handle_change_direction called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_direction(direction);
            entity.set_head_direction(head_direction);
        }
    }

    /// `NetworkEvent::StateChange`.
    pub(crate) fn handle_state_change(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::StateChange {
            entity_id,
            body_state,
            health_state,
            effect_state,
            ..
        } = event
        else {
            unreachable!("handle_state_change called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_body_state(body_state);
            entity.set_health_state(health_state);
            // Applies hide/cloak/invisible by hiding the entity; the
            // remaining option bits (falcon/cart/riding/costumes) are
            // stored for the appearance workstream.
            entity.set_effect_state(effect_state, client_tick);
        }

        // Load the hunter falcon animation lazily the first time the
        // option is enabled for this entity.
        let falcon_entity_index = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .position(|entity| entity.get_entity_id() == entity_id && entity.has_falcon() && !entity.has_falcon_animation())
            .filter(|_| effect_state & 0x10 != 0);

        if let Some(index) = falcon_entity_index
            && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
            && let Some(entity) = self.client_state.follow_mut(client_state().entities()).get_mut(index)
        {
            entity.set_falcon_animation(falcon_animation);
        }
    }

    /// `NetworkEvent::UpdateEntityHealth`.
    pub(crate) fn handle_update_entity_health(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UpdateEntityHealth {
            entity_id,
            health_points,
            maximum_health_points,
        } = event
        else {
            unreachable!("handle_update_entity_health called with a different event variant");
        };

        let entity = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id);

        if let Some(entity) = entity {
            entity.update_health(health_points, maximum_health_points);
        }

        // Keep the own HP bar in the party window in sync. The server
        // does not send `ZC_NOTIFY_HP_TO_GROUPM` for the local player,
        // so the party window takes the HP from the local entity.
        let own_account_id = self.saved_login_data.as_ref().map(|login_data| login_data.account_id);
        let is_own_entity = self
            .client_state
            .try_follow(this_entity())
            .is_some_and(|entity| entity.get_entity_id() == entity_id);

        if let (true, Some(own_account_id)) = (is_own_entity, own_account_id) {
            self.client_state.follow_mut(client_state().party()).apply_health(
                own_account_id,
                health_points as i32,
                maximum_health_points as i32,
            );
        }
    }

    /// `NetworkEvent::PetStateChanged`.
    pub(crate) fn handle_pet_state_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::PetStateChanged {
            entity_id: _,
            state_type,
            value,
        } = event
        else {
            unreachable!("handle_pet_state_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().companion())
            .apply_pet_state_change(state_type, value);
    }

    /// `NetworkEvent::VisualEffect`.
    pub(crate) fn handle_visual_effect(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::VisualEffect { effect_path, entity_id } = event else {
            unreachable!("handle_visual_effect called with a different event variant");
        };

        self.spawn_entity_effect(effect_path, entity_id, Vector3::new(0.0, 9.0, 0.0), 1.0, Color::WHITE, 50.0);
    }

    /// `NetworkEvent::GainedExperience`.
    pub(crate) fn handle_gained_experience(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::GainedExperience {
            amount,
            experience_type,
            experience_source,
        } = event
        else {
            unreachable!("handle_gained_experience called with a different event variant");
        };

        self.pending_experience.add(amount, experience_type, experience_source);
    }

    /// `NetworkEvent::EntityStopped`.
    pub(crate) fn handle_entity_stopped(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::EntityStopped { entity_id, position } = event else {
            unreachable!("handle_entity_stopped called with a different event variant");
        };

        let Some(map) = &self.map else {
            return;
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.stop_movement();
            entity.set_position(map, position, client_tick);
        }
    }

    /// `NetworkEvent::EntityHighJumped`.
    pub(crate) fn handle_entity_high_jumped(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::EntityHighJumped { entity_id, position } = event else {
            unreachable!("handle_entity_high_jumped called with a different event variant");
        };

        // Taekwon Leap (`TK_HIGHJUMP`, ZC_HIGHJUMP 0x01FF): the
        // server has already moved the character; apply its
        // position immediately so the client does not keep the old
        // cell and then snap on the following stop-move packet.
        let Some(map) = &self.map else {
            return;
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.stop_movement();
            entity.set_position(map, position, client_tick);

            if entity_id == EntityId(self.saved_login_data.map_or(0, |data| data.account_id.0)) {
                self.player_camera.set_focus_point(entity.get_position());
            }
        }
    }

    /// `NetworkEvent::EntitySnapped`.
    pub(crate) fn handle_entity_snapped(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::EntitySnapped { entity_id, position } = event else {
            unreachable!("handle_entity_snapped called with a different event variant");
        };

        // Body Relocation / Snap (ZC_SNAP 0x08D2): apply the
        // server's new position immediately so the client does not
        // keep rendering the character at the old cell.
        let Some(map) = &self.map else {
            return;
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.stop_movement();
            entity.set_position(map, position, client_tick);

            if entity_id == EntityId(self.saved_login_data.map_or(0, |data| data.account_id.0)) {
                self.player_camera.set_focus_point(entity.get_position());
            }
        }
    }

    /// `NetworkEvent::SpecialEffect`.
    pub(crate) fn handle_special_effect(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SpecialEffect { effect_id, .. } = event else {
            unreachable!("handle_special_effect called with a different event variant");
        };

        // `ZC_NOTIFY_EFFECT2` (0x01F3) is the map weather packet
        // (snow/fog/cloud/sakura/...). Indoor maps use the indoor
        // map type from `ZC_NOTIFY_MAPPROPERTY2`; outdoor weather
        // effects are not drawn there.
        let map_type = *self.client_state.follow(client_state().map_type());
        if crate::state::is_indoor_map_type(map_type) {
            crate::logging::log_debug!("ignoring outdoor weather effect {effect_id} on an indoor map");
            if let Some(weather) = self.weather.as_mut() {
                weather.clear();
            }
        } else if let Some(weather) = self.weather.as_mut() {
            weather.set_effect(effect_id as i32, 0.0);
        }
    }

    /// `NetworkEvent::MinimapMarker`.
    pub(crate) fn handle_minimap_marker(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::MinimapMarker {
            npc_id,
            marker_type,
            position,
            id: _,
            color,
        } = event
        else {
            unreachable!("handle_minimap_marker called with a different event variant");
        };

        let minimap = self.client_state.follow_mut(client_state().minimap());

        match marker_type {
            MarkerType::RemoveMark => minimap.remove_marker(npc_id),
            MarkerType::DisplayFor15Seconds | MarkerType::DisplayUntilLeave => {
                let expires_at = match marker_type {
                    MarkerType::DisplayFor15Seconds => Some(client_tick.0.wrapping_add(15_000)),
                    MarkerType::DisplayUntilLeave => None,
                    MarkerType::RemoveMark => unreachable!(),
                };

                minimap.add_marker(crate::state::minimap::MinimapMarker {
                    entity_id: npc_id,
                    position: TilePosition {
                        x: position.x.min(u16::MAX as u32) as u16,
                        y: position.y.min(u16::MAX as u32) as u16,
                    },
                    color: [color.red, color.green, color.blue, color.alpha],
                    expires_at,
                });
            }
        }
    }

    /// `NetworkEvent::MapProperties`.
    pub(crate) fn handle_map_properties(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MapProperties { map_type, flags } = event else {
            unreachable!("handle_map_properties called with a different event variant");
        };

        *self.client_state.follow_mut(client_state().map_type()) = map_type;
        *self.client_state.follow_mut(client_state().map_flags()) = flags;
    }

    /// `NetworkEvent::MapCellChanged`.
    pub(crate) fn handle_map_cell_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::MapCellChanged {
            position,
            cell_type,
            map_name,
        } = event
        else {
            unreachable!("handle_map_cell_changed called with a different event variant");
        };

        // A delayed update for the map the player just left must not
        // patch collision on the new map.
        if let Some(current_map_name) = self.current_map_name.as_deref()
            && !current_map_name.eq_ignore_ascii_case(&map_name)
        {
            crate::logging::log_debug!("ignoring map cell update for `{map_name}` while on `{current_map_name}`");
            return;
        }

        // rAthena cell types: 0 walkable, 1 blocked, 2 water,
        // 3 water+walkable, 4 water+snipeable, 5 cliff+snipeable,
        // 6 cliff. Only 0 and 3 are walkable. The loaded map is an
        // immutable Arc<Map>, so the override is recorded in the
        // state and applied by `TraversableMap` in the path finder.
        let walkable = matches!(cell_type, 0 | 3);
        let overrides = self.client_state.follow_mut(client_state().map_cell_overrides());

        match overrides.iter_mut().find(|(tile, _)| *tile == position) {
            Some((_, current)) => *current = walkable,
            None => overrides.push((position, walkable)),
        }

        crate::logging::log_debug!(
            "map cell changed: {} ({}, {}) -> type {}",
            map_name,
            position.x,
            position.y,
            cell_type
        );
    }
}
