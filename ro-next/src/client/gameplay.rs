//! In-game behaviour driven by the dispatchers: skill casting and aftercast
//! timers, entity detail requests, status-effect/aura updates and the
//! experience/skill-tree bookkeeping.

use crate::*;

impl Client {
    /// Updates every part of the client that depends on the size of the window.
    ///
    /// This is called for every resize event, but also once after the window
    /// was created. The window is created with the configured resolution,
    /// which can differ from [`INITIAL_SCREEN_SIZE`], and winit does not
    /// necessarily send a resize event for the initial size. Without this
    /// the interface and all renderers would keep using the initial size,
    /// which stretches the interface and breaks its hit testing.
    /// Request the names of all entities that should show a name label.
    ///
    /// Entity details (the name) are loaded lazily from the map server and were
    /// previously only requested for the entity under the mouse. That made the
    /// always on name labels only appear for entities that had been hovered
    /// once before.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn request_visible_entity_names(&mut self) {
        if self.map.is_none() {
            return;
        }

        let show_character_names = *self.client_state.follow(client_state().interface_settings().show_character_names());

        if !show_character_names {
            return;
        }

        // Limit the number of name requests per frame so entering a crowded map does
        // not burst a request per character/entity.
        const MAXIMUM_NAME_REQUESTS_PER_FRAME: usize = 8;

        let now = Instant::now();
        let mut entity_ids = Vec::new();

        for entity in self.client_state.follow_mut(client_state().entities()).iter_mut() {
            if entity_ids.len() >= MAXIMUM_NAME_REQUESTS_PER_FRAME {
                break;
            }

            if !matches!(entity.get_entity_type(), EntityType::Player | EntityType::Npc) {
                continue;
            }

            // `details_need_request` also handles the retry of a lost response
            // and gives up after the attempt budget (with a warning).
            if entity.details_need_request(now) {
                entity_ids.push(entity.get_entity_id());
            }
        }

        for entity_id in entity_ids {
            if self.networking_system.entity_details(entity_id).is_ok()
                && let Some(entity) = self
                    .client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .find(|entity| entity.get_entity_id() == entity_id)
            {
                entity.set_details_requested(now);
            }
        }
    }

    /// Advance the `M` key cycle: small minimap -> big map -> closed.
    ///
    /// The stored [`MapWindowCycle`] is the explicit state of the cycle, but a
    /// map window can also be closed with its X button or by a map change, so
    /// the state is re-synchronized with the actually open windows before the
    /// next step is computed.
    pub(crate) fn cycle_map_windows(interface: &mut Interface<'static, ClientState>, map_window_cycle: &mut MapWindowCycle) {
        let current_state = if interface.is_window_with_class_open(WindowClass::BigMap) {
            MapWindowCycle::BigMap
        } else if interface.is_window_with_class_open(WindowClass::Minimap) {
            MapWindowCycle::Minimap
        } else {
            MapWindowCycle::Closed
        };

        let next_state = match current_state {
            MapWindowCycle::Closed => MapWindowCycle::Minimap,
            MapWindowCycle::Minimap => MapWindowCycle::BigMap,
            MapWindowCycle::BigMap => MapWindowCycle::Closed,
        };

        interface.close_window_with_class(WindowClass::Minimap);
        interface.close_window_with_class(WindowClass::BigMap);

        match next_state {
            MapWindowCycle::Closed => {}
            MapWindowCycle::Minimap => interface.open_window(MinimapWindow),
            MapWindowCycle::BigMap => interface.open_window(BigMapWindow),
        }

        *map_window_cycle = next_state;
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn request_entity_details(&mut self, input_report: &InputReport) {
        let now = Instant::now();

        if let PickerTarget::Entity(entity_id) = input_report.mouse_target
            && let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
            && entity.details_need_request(now)
            && self.networking_system.entity_details(entity_id).is_ok()
        {
            entity.set_details_requested(now);
        }
    }

    /// Shows the aggregated experience gain in the chat once the packet burst
    /// is over.
    ///
    /// The line and its labels are localization templates
    /// (`chat_experience_gained` + `chat_experience_*`), rendered with the same
    /// `{name}` placeholders as the other client feedback.
    pub(crate) fn flush_experience_feedback(&mut self) {
        let Some(last_packet) = self.pending_experience.last_packet else {
            return;
        };

        if last_packet.elapsed() < EXPERIENCE_FEEDBACK_DELAY {
            return;
        }

        let Some(gains) = self.pending_experience.take_gains() else {
            return;
        };

        let localization = self.client_state.follow(client_state().localization());
        let labels = localization.experience_labels();
        let mut parts = Vec::new();

        for (amount, label) in gains.into_iter().zip(labels) {
            if amount != 0 {
                parts.push(format!(
                    "{}{} {}",
                    if amount < 0 { "-" } else { "" },
                    format_amount(amount.unsigned_abs()),
                    label
                ));
            }
        }

        let template = self.client_state.follow(client_state().localization().chat_experience_gained());
        let message = ro_next_messages::render_template(template, &[("gains", &parts.join(", "))]);

        self.push_chat_information(message);
    }

    /// Spawns the classic level-99 aura below player entities whose base level
    /// is at least 99 and removes auras that are no longer valid.
    ///
    /// The aura itself re-checks the entity on every update (despawn, death,
    /// level drop, configured style), so this only has to create the effect
    /// once per entity. Entities whose base level is unknown (NPCs, population
    /// shells that are only sent via a spawn packet) are skipped gracefully.
    /// Adds learned skills that the character's (possibly fallback) job skill
    /// tree does not contain to a synthetic "Learned" tab.
    ///
    /// High/expanded job ids map to a fallback base job tree (or have a trimmed
    /// tree in this data set), so skills the server reports as learned can be
    /// missing from whatever tree the client resolved. The tree only iterates
    /// its layout skills, so without this those skills would be invisible
    /// even though they are learned (use/drag/right-click never had a slot
    /// to interact with).
    pub(crate) fn merge_learned_skills_into_skill_tree(&mut self, client_tick: ClientTick) {
        let learned_skill_ids: Vec<SkillId> = self
            .client_state
            .follow(client_state().skill_tree().skills())
            .iter()
            .map(|learned_skill| learned_skill.skill_id)
            .collect();

        if learned_skill_ids.is_empty() {
            return;
        }

        let Some(job_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_job_id()) else {
            return;
        };

        let existing_skill_ids: std::collections::HashSet<SkillId> = self
            .client_state
            .follow(client_state().skill_tree().layout())
            .tabs
            .iter()
            .flat_map(|tab| tab.skills.values().map(|skill| skill.skill_id))
            .collect();

        let missing_skill_ids: Vec<SkillId> = learned_skill_ids
            .into_iter()
            .filter(|skill_id| !existing_skill_ids.contains(skill_id))
            .collect();

        if missing_skill_ids.is_empty() {
            return;
        }

        let mut learned_tab = crate::state::skills::SkillTabLayout {
            name: "Learned".to_owned(),
            skills: Default::default(),
        };

        for (slot, skill_id) in missing_skill_ids.into_iter().enumerate() {
            learned_tab.skills.insert(
                slot,
                self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick),
            );
        }

        let layout = self.client_state.follow_mut(client_state().skill_tree().layout());
        layout.tabs.push(learned_tab);

        // Make the tab visible immediately: the tree shows the selected tab, so the
        // player does not have to guess where the missing skills went.
        let selected_tab = layout.tabs.len().saturating_sub(1);
        *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = selected_tab;
    }

    /// Shows the name of a used skill above the caster's head (`Blessing !!`).
    ///
    /// Called once per cast (never per animation frame). The label replaces a
    /// previous label of the same entity, so recasts refresh instead of
    /// stacking.
    /// Starts the local player's cast bar for a server-confirmed cast with a
    /// cast time.
    ///
    /// Only the local player gets a cast bar. Called from the cast-start packet
    /// handler, so a rejected/failed cast never shows one.
    pub(crate) fn start_own_cast_bar(&mut self, source_entity_id: EntityId, skill_id: SkillId, cast_time: u32, client_tick: ClientTick) {
        if cast_time == 0 {
            return;
        }

        let own_entity_id = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id());

        if own_entity_id != Some(source_entity_id) {
            return;
        }

        let skill_name = self.library.get::<SkillListInformation>(skill_id).name.clone();

        self.cast_bars.start(source_entity_id, skill_name, cast_time, client_tick);
    }

    /// Plays the official sound of a skill once its completion (or damage)
    /// packet arrives.
    ///
    /// The play timing is decided by [`SkillSoundGate`]; this only forwards the
    /// audible part so the gate stays unit-testable without audio.
    pub(crate) fn play_completion_sound(
        &mut self,
        entity_id: EntityId,
        skill_id: SkillId,
        phase: SkillPhase,
        position: Option<cgmath::Point3<f32>>,
        client_tick: ClientTick,
    ) {
        if self.skill_sound_gate.claim_completion(entity_id, skill_id, client_tick) {
            self.skill_sounds
                .play_skill_sound_for_phase(skill_id, phase, Some(entity_id), position);
        }
    }

    /// Whether the local player's movement is currently blocked by a
    /// server-confirmed cast with a cast time.
    ///
    /// Only the cast bar state is used: instant skills never start a bar (and
    /// therefore never root the player), and a cancelled or completed cast
    /// clears the bar, which releases the movement block immediately. Dead or
    /// sitting characters are never rooted by a stale bar.
    pub(crate) fn movement_blocked_by_cast(&self, client_tick: ClientTick) -> bool {
        let Some(player) = self.client_state.try_follow(this_entity()) else {
            return false;
        };

        if player.is_dead() || player.is_sitting() {
            return false;
        }

        if !self.cast_bars.is_casting(client_tick) {
            return false;
        }

        // Sage/Scholar/Professor with the `Free Cast` passive can move while
        // casting (skill flag + job capability, both data). Unknown jobs
        // default to "blocked".
        if can_move_while_casting(
            player.get_job_id(),
            self.client_state.follow(client_state().skill_tree().skills()),
        ) {
            return false;
        }

        crate::logging::log_debug!(
            "movement blocked: local player is casting (job={} free_cast=false)",
            player.get_job_id().0
        );
        true
    }

    /// Whether the local player's after-cast act delay is still running.
    ///
    /// The after-cast act delay only paces follow-up *actions* (skills/items);
    /// it deliberately does not block movement, which is controlled by the
    /// server and by the cast bar rooting above.
    pub(crate) fn aftercast_act_active(&self, client_tick: ClientTick) -> bool {
        self.aftercast_act_until.is_some_and(|until| {
            // `until` is in the future while `until - now` is less than half of
            // the wrapping u32 range.
            until.0.wrapping_sub(client_tick.0) < 0x8000_0000
        })
    }

    /// Start the after-cast act delay of a completed skill, using the per-skill
    /// value from the generated rAthena table.
    pub(crate) fn apply_aftercast_act_delay(&mut self, skill_id: SkillId, level: u16, client_tick: ClientTick) {
        let Some(delays) = skill_delays(skill_id, level) else {
            return;
        };

        if delays.aftercast_act_ms == 0 {
            return;
        }

        self.aftercast_act_until = Some(ClientTick(client_tick.0.wrapping_add(delays.aftercast_act_ms)));

        crate::logging::log_debug!(
            "aftercast act delay: skill={} level={} {}ms (server table)",
            skill_id.0,
            level,
            delays.aftercast_act_ms
        );
    }

    pub(crate) fn show_skill_name_text(&mut self, entity_id: EntityId, skill_id: SkillId, client_tick: ClientTick) {
        let is_character = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .any(|entity| entity.get_entity_id() == entity_id && entity.get_entity_type() == EntityType::Player);

        if !is_character {
            crate::logging::log_debug!("skill name text: entity {:?} is not a character, skipping", entity_id);
            return;
        }

        let skill_name = self.library.get::<SkillListInformation>(skill_id).name.clone();
        let text = format!("{skill_name} !!");

        crate::logging::log_debug!("skill name text: entity {:?} skill {:?} -> {:?}", entity_id, skill_id, text);

        self.skill_name_texts.push(entity_id, text, client_tick);
    }

    pub(crate) fn update_level_99_auras(&mut self) {
        let style = *self.client_state.follow(client_state().interface_settings().level_99_aura_style());

        self.level_99_aura_style_cell.store(style.encode(), Ordering::Relaxed);

        if style == AuraStyle::Off {
            for entity_id in self.level_99_aura_entities.drain() {
                self.effect_holder.remove_unit(entity_id);
            }

            return;
        }

        let mut valid_entities = HashSet::new();

        let player_ids: Vec<(EntityId, Point3<f32>, usize, bool)> = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .filter(|entity| !entity.is_dead())
            .filter_map(|entity| {
                let level = entity.get_base_level()?;

                (level >= crate::world::AURA_MINIMUM_LEVEL).then(|| {
                    (
                        entity.get_entity_id(),
                        entity.get_position(),
                        level,
                        entity.is_transcendent_job(),
                    )
                })
            })
            .collect();

        for (entity_id, position, level, transcendent) in player_ids {
            valid_entities.insert(entity_id);

            if self.level_99_aura_entities.contains(&entity_id) {
                continue;
            }

            let Some(mut aura) = crate::world::Aura::new(self.level_99_aura_style_cell.clone(), level, transcendent, &self.texture_loader)
            else {
                continue;
            };

            crate::logging::log_debug!(
                "level-99 aura: created for entity {:?} (level {}, transcendent {})",
                entity_id,
                level,
                transcendent
            );
            aura.attach_to_entity(entity_id, position);
            self.effect_holder.add_unit(Box::new(aura), entity_id);
            self.level_99_aura_entities.insert(entity_id);
        }

        // Forget auras of entities that are no longer valid (despawned, dead or
        // below the level threshold). The effect removes itself on its next
        // update; removing the id here only makes sure it is re-created when
        // the entity becomes eligible again.
        let stale = self
            .level_99_aura_entities
            .iter()
            .filter(|entity_id| !valid_entities.contains(entity_id))
            .copied()
            .collect::<Vec<_>>();
        for entity_id in stale {
            self.level_99_aura_entities.remove(&entity_id);
        }
    }

    /// Spawns, keeps alive and removes the persistent glow effects of active
    /// status effects (the Assumptio glow and the documented buff glows, see
    /// [`crate::world::status_aura_spec`]).
    ///
    /// The active statuses are reconciled every frame instead of being wired to
    /// the status packets: status start/refresh/end, local timer expiry, death,
    /// warp and entities entering/leaving the view then all behave the same,
    /// and adding another glowing status only needs a data row in
    /// `archive/data/client/status_effects.ron` (see
    /// `tools/data/status_effect_custom.ron`).
    ///
    /// The reconciliation itself does not allocate: the per-frame active set is
    /// encoded as the last-seen frame next to each spawned aura, so only the
    /// (rare) spawn path touches the allocator.
    pub(crate) fn update_status_auras(&mut self) {
        self.status_aura_frame = self.status_aura_frame.wrapping_add(1);
        let frame = self.status_aura_frame;

        // Official `CPc::ApplyLayerEffects` enables the white pulse outline
        // while EFST_ASSUMPTIO / EFST_ASSUMPTIO2 is active. It is a render
        // flag, not a spawned aura; reconcile it before the glow pass so an
        // ended status clears the outline immediately.
        let pulse_entity_ids: HashSet<u32> = {
            let statuses = self.client_state.follow(client_state().status_effects());
            statuses
                .effects()
                .iter()
                .filter(|status| matches!(status.status_index(), 110 | 473))
                .map(|status| status.entity_id())
                .collect()
        };
        {
            let entities = self.client_state.follow_mut(client_state().entities());
            for entity in entities.iter_mut() {
                let active = pulse_entity_ids.contains(&entity.get_entity_id().0) && !entity.is_dead() && !entity.is_fading();
                entity.set_status_pulse_outline(active);
            }
        }

        let statuses = self.client_state.follow(client_state().status_effects());
        let entities = self.client_state.follow(client_state().entities());

        for status in statuses.effects() {
            let status_index = status.status_index();
            if crate::world::status_aura_spec(status_index).is_none() {
                continue;
            }

            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id().0 == status.entity_id()) else {
                // The entity left the view; the server re-sends its statuses
                // with `ZC_EFST_SET_ENTER` when it comes back.
                continue;
            };
            let entity_id = entity.get_entity_id();

            if entity.is_dead() || entity.is_fading() {
                continue;
            }

            let key = (entity_id, status_index);
            if let Some((_, last_seen)) = self.status_aura_flags.get_mut(&key) {
                *last_seen = frame;
                continue;
            }

            let Some(spec) = crate::world::status_aura_spec(status_index) else {
                continue;
            };
            let is_active = Arc::new(AtomicBool::new(true));

            let Some(aura) = crate::world::StatusAura::new(&self.texture_loader, entity_id, spec, is_active.clone()) else {
                crate::logging::log_warn!("status aura: failed to load the glow textures of status {}", status_index);
                // Do not retry every frame if the textures are missing.
                self.status_aura_flags.insert(key, (Arc::new(AtomicBool::new(false)), frame));
                continue;
            };

            if self.effect_holder.add_unit(Box::new(aura), entity_id) {
                crate::logging::log_debug!(
                    "status aura: spawned glow for status {} on entity {}",
                    status_index,
                    entity_id.0
                );
                self.status_aura_flags.insert(key, (is_active, frame));
            }
            // If the effect budget rejected the aura, do not record it: the
            // spawn is retried on the next frame, when the budget may have
            // room.
        }

        // Deactivate auras whose status ended (or whose entity is no longer
        // eligible); the effect removes itself on its next update.
        self.status_aura_flags.retain(|_, (is_active, last_seen)| {
            if *last_seen == frame {
                return true;
            }

            is_active.store(false, Ordering::Relaxed);
            false
        });
    }

    /// Reconciles the looping/ground status visuals with the active statuses.
    pub(crate) fn update_status_visuals(&mut self) {
        use crate::world::StatusVisualKind;

        self.status_visual_frame = self.status_visual_frame.wrapping_add(1);
        let frame = self.status_visual_frame;

        let statuses = self.client_state.follow(client_state().status_effects());
        let entities = self.client_state.follow(client_state().entities());

        for status in statuses.effects() {
            let status_index = status.status_index();
            let Some(visual) = crate::world::status_visual(status_index) else {
                continue;
            };
            if !matches!(
                visual.kind,
                StatusVisualKind::LoopingEffect { .. } | StatusVisualKind::GroundRing { .. }
            ) {
                continue;
            }
            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id().0 == status.entity_id()) else {
                continue;
            };
            if entity.is_dead() || entity.is_fading() {
                continue;
            }
            let entity_id = entity.get_entity_id();
            let key = (entity_id, status_index);
            if let Some((_, last_seen)) = self.status_visual_flags.get_mut(&key) {
                *last_seen = frame;
                continue;
            }

            let active = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
            let spawned = match visual.kind {
                StatusVisualKind::LoopingEffect { file, anchor, scale, .. } => {
                    let Ok(effect) = self.effect_loader.get_or_load(file, &self.texture_loader) else {
                        crate::logging::log_warn!("status visual: failed to load looping effect '{}'", file);
                        self.status_visual_flags
                            .insert(key, (std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)), frame));
                        continue;
                    };
                    let offset = match anchor {
                        crate::world::StatusVisualAnchor::Body => cgmath::Vector3::new(0.0, 0.0, 0.0),
                        crate::world::StatusVisualAnchor::Overhead => cgmath::Vector3::new(0.0, 20.0, 0.0),
                        crate::world::StatusVisualAnchor::Ground => cgmath::Vector3::new(0.0, 1.0, 0.0),
                    };
                    let frame_timer = effect.new_frame_timer();
                    let inner = crate::world::EffectWithLight::new(
                        effect,
                        frame_timer,
                        crate::world::EffectCenter::Entity(entity_id, cgmath::Point3::new(0.0, 0.0, 0.0)),
                        offset,
                        scale,
                        crate::world::PointLightId::new(entity_id.0.wrapping_add(status_index as u32).wrapping_add(0x5000_0000)),
                        cgmath::Vector3::new(0.0, 20.0, 0.0),
                        crate::graphics::Color::WHITE,
                        0.0,
                        true,
                    );
                    self.effect_holder.add_unit(
                        Box::new(crate::world::StatusLoopingEffect::new(inner, active.clone())),
                        entity_id,
                    )
                }
                StatusVisualKind::GroundRing {
                    texture,
                    radius,
                    height,
                    color,
                    pulse,
                    ..
                } => crate::world::StatusGroundRing::new(
                    &self.texture_loader,
                    entity_id,
                    texture,
                    active.clone(),
                    radius,
                    height,
                    color,
                    pulse,
                )
                .map(|ring| self.effect_holder.add_unit(Box::new(ring), entity_id))
                .unwrap_or(false),
                _ => false,
            };

            if spawned {
                self.status_visual_flags.insert(key, (active, frame));
            }
        }

        self.status_visual_flags.retain(|_, (active, last_seen)| {
            if *last_seen == frame {
                return true;
            }
            active.store(false, std::sync::atomic::Ordering::Relaxed);
            false
        });
    }

    /// Updates the local status effect timers and shows/hides the buff bar.
    pub(crate) fn update_status_effects(&mut self, client_tick: ClientTick) {
        self.client_state
            .follow_mut(client_state().notifications())
            .retain_alive(client_tick.0);
        self.client_state.follow_mut(client_state().minimap()).prune_markers(client_tick.0);
        self.client_state.follow_mut(client_state().skill_cooldowns()).prune(client_tick);

        {
            let status_effects = self.client_state.follow_mut(client_state().status_effects());
            status_effects.set_client_tick(client_tick.0 as u64);
            status_effects.update(client_tick.0 as u64);
        }

        // The status effect state tracks all entities in the view, but the buff
        // bar is only shown for the player's own effects.
        let own_entity_id = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id().0);
        let is_empty = match own_entity_id {
            Some(entity_id) => self.client_state.follow(client_state().status_effects()).is_empty_for(entity_id),
            None => true,
        };
        let is_open = self.interface.is_window_with_class_open(WindowClass::BuffBar);

        match (is_empty, is_open) {
            (false, false) => self.open_buff_bar_window(),
            (true, true) => self.interface.close_window_with_class(WindowClass::BuffBar),
            _ => {}
        }
    }

    /// Recomputes the best status sprite visual of `entity_id` and applies it.
    ///
    /// The data-driven table is queried for every active status of the entity:
    /// the highest-priority `BodyTint` wins (it also carries the additive
    /// body/weapon scope), the first `BodyFade` is applied, and `WeaponBlink`
    /// is OR-ed. When no visual matches, the previous state is cleared so
    /// ending a status cannot leave a stale colour or blend behind.
    pub(crate) fn refresh_entity_status_visual(&mut self, entity_id: ragnarok_packets::EntityId) {
        let state = {
            let statuses = self.client_state.follow(client_state().status_effects());
            crate::world::resolve_status_visual_state(
                statuses
                    .effects()
                    .iter()
                    .filter(|effect| effect.entity_id() == entity_id.0)
                    .filter_map(|effect| crate::world::status_visual(effect.status_index()))
                    .map(|visual| &visual.kind),
            )
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.set_status_tint(state.tint);
            entity.set_status_alpha(state.alpha);
            entity.set_weapon_blink(state.weapon_blink);
            entity.set_status_additive(state.body_additive, state.weapon_additive);
        }
    }

    /// Applies stat updates that arrived before the local player entity
    /// existed.
    ///
    /// Called once per frame after the network events were processed; the
    /// buffer is normally empty, so this is almost free.
    pub(crate) fn apply_pending_stat_updates(&mut self) {
        if self.pending_stat_updates.is_empty() {
            return;
        }

        let pending_stat_updates: Vec<StatType> = self.pending_stat_updates.take().collect();

        // The zeny value is displayed by the inventory window, so keep it in the client
        // state (the player stats do not contain it).
        if let Some(zeny) = pending_stat_updates.iter().rev().find_map(|stat_type| match stat_type {
            StatType::Zeny(value) => Some(u64::from(*value)),
            _ => None,
        }) {
            *self.client_state.follow_mut(client_state().zeny()) = Some(zeny);
        }

        let Some(player) = self.client_state.try_follow_mut(this_player()) else {
            // Still no player; keep the updates buffered for a later frame.
            for stat_type in pending_stat_updates {
                self.pending_stat_updates.push(stat_type);
            }
            return;
        };

        for stat_type in pending_stat_updates {
            player.update_stat(stat_type);
        }
    }

    /// Uses a learned skill: self cast skills are cast immediately, skills that
    /// require a target switch to the target selection mouse mode, like in the
    /// original client. Passive skills are ignored.
    pub(crate) fn use_skill(
        &mut self,
        slot: Option<HotbarSlot>,
        skill_id: SkillId,
        skill_type: SkillType,
        level: SkillLevel,
        client_tick: ClientTick,
    ) {
        if self.aftercast_act_active(client_tick) {
            crate::logging::log_debug!("ignoring skill {}: after-cast act delay still running", skill_id.0);
            return;
        }

        match skill_type {
            SkillType::Passive => self.push_localized_chat(client_state().localization().chat_skill_passive(), &[(
                "skill",
                &skill_id.0.to_string(),
            )]),
            SkillType::SelfCast => self.cast_self_skill(skill_id, level),
            skill_type => self.interface.set_mouse_mode(MouseMode::Custom {
                mode: MouseInputMode::TargetSkill {
                    slot,
                    skill_id,
                    skill_type,
                    level,
                },
            }),
        }
    }

    /// Casts a self-cast skill at the local player.
    ///
    /// Channeling skills (Rolling Cutter 2036; data flag in `skills.ron`) use
    /// the channeling packet path, every other self cast uses the ordinary
    /// cast packet. The skill id list is data, not a branch at the call site.
    pub(crate) fn cast_self_skill(&mut self, skill_id: SkillId, level: SkillLevel) {
        let entity_id = self.client_state.follow(this_entity().manually_asserted()).get_entity_id();

        if crate::world::skills::channels(skill_id) {
            let _ = self.networking_system.cast_channeling_skill(skill_id, level, entity_id);
        } else {
            let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
        }
    }
}
