//! `NetworkEvent` handlers for the skills family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::SkillCast`.
    pub(crate) fn handle_skill_cast(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SkillCast {
            source_entity_id,
            destination_entity_id,
            position,
            skill_id,
            element,
            cast_time,
            attack_motion_time,
        } = event
        else {
            unreachable!("handle_skill_cast called with a different event variant");
        };

        // Other clients show the caster's skill-cast action for the cast
        // duration and a ground cast circle below the caster. The 2019
        // server does not send the animation separately, so this is the main
        // cue for ambient self-buffs, crafting, warp portal casts and the
        // skills of other players.
        let has_cast_time = cast_time > 0;
        // The server packet's cast time is authoritative. The
        // per-skill table only provides a lower bound for the
        // animation/pose duration when the packet has no usable
        // timing (the old code used one global 300 ms constant).
        let table_delays = skill_delays(skill_id, 1);
        let animation_minimum = table_delays
            .map(|delays| delays.total_cast_ms().max(delays.aftercast_act_ms))
            .unwrap_or(0);
        let cast_animation_time = cast_time.max(attack_motion_time).max(animation_minimum);
        // The cast circle tint is data-driven (a per-skill override in the
        // unified skill effect descriptors) and falls back to the element
        // color of the `ZC_USESKILL_ACK` packet.
        let cast_circle_color = crate::world::skill_cast_circle_color(skill_id).unwrap_or_else(|| crate::world::element_color(element));

        if let Some(delays) = table_delays {
            crate::logging::log_debug!(
                "skill delay table: skill={} cast={}ms fixed={}ms aftercast_act={}ms aftercast_walk={}ms cooldown={}ms",
                skill_id.0,
                delays.cast_ms,
                delays.fixed_cast_ms,
                delays.aftercast_act_ms,
                delays.aftercast_walk_ms,
                delays.cooldown_ms
            );
        }

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == source_entity_id)
        {
            let sequence = crate::world::frame_sequence_for_skill(skill_id);
            // `NotifySkillCasting` returns before `GetSkillActionInfo` when
            // the packet has no cast time (`GameModePackets.cpp:12951-12953`).
            // Do not invent a pose on an instant ack; the 0x09CB/damage
            // packet supplies it. Sequence carriers keep the ack pose so the
            // frame range has its base ACT block.
            if let Some(action) = ack_caster_action(skill_id, cast_time, sequence.is_some()) {
                match action {
                    // Stance/buff skills (Zen, Fury, Endure, ...) keep the idle
                    // pose in the official client; the hard-coded skill effect
                    // is the visual feedback instead of a casting gesture.
                    CasterAction::Idle => entity.set_idle(client_tick),
                    // Casts with a cast time loop the casting gesture until the
                    // cast finishes; instant casts play the action once.
                    CasterAction::Skill => entity.set_skill_cast(cast_animation_time, has_cast_time, client_tick),
                    // Attack skills play the normal weapon swing. `set_attack`
                    // picks the ACT block the equipped weapon layer actually
                    // animates (`attack_action()`), which is what the official
                    // `CPc::SetState` does when it upgrades `ST_ATTACK` to
                    // `ST_ATTACK2` for a weapon-allowed job (`Player.cpp:4545`).
                    CasterAction::Attack => entity.set_attack(cast_animation_time, false, client_tick),
                    // Pickup-style skills play the pickup animation.
                    CasterAction::Pickup => entity.set_pickup(client_tick),
                    CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                    CasterAction::Action(action_index) => {
                        entity.set_custom_skill_action(action_index as usize, cast_animation_time, has_cast_time, client_tick)
                    }
                    // Only emitted by the 0x09CB use table.
                    CasterAction::Keep => {}
                }

                // The official per-skill frame range (`SetFrameSequence`
                // in RenderObject.cpp) describes the *damage* animation;
                // for casts with a cast time it starts when the skill
                // resolves (see the damage arms below).
                if !has_cast_time && let Some(sequence) = sequence {
                    entity.set_skill_frame_sequence_with_rate(
                        sequence.first,
                        sequence.last,
                        sequence.loop_to,
                        sequence.duration_ms,
                        sequence.step_ms(),
                        client_tick,
                    );
                }
            }

            // Targeted casts face their target, self-casts keep the current
            // direction.
            if destination_entity_id != source_entity_id {
                entity.rotate_towards(position);
            }
        }

        // Warp Portal fallback: the server asks the caster for the
        // destination (ZC_WARPLIST) before it creates the portal skill
        // units. When the skill use packet already carries the target
        // cell, show the blue portal there immediately; the real
        // `AddSkillUnit` packet replaces it (same cell) when the warp
        // opens. This keeps the portal visible even when the warp
        // destination menu cannot be answered.
        if crate::world::skills::has_warp_target(skill_id)
            && let Some(world_position) = self.map.as_ref().and_then(|map| map.get_world_position(position))
        {
            let now = Instant::now();
            let lifetime = Duration::from_secs(20);
            let provisional_id = EntityId(0xFF00_0000 | ((position.x as u32) << 8) | position.y as u32);
            self.warp_skill_units
                .retain(|(unit_entity_id, ..)| *unit_entity_id != provisional_id);
            self.warp_skill_units.push((provisional_id, world_position, now, now + lifetime));
        }

        // Show the skill name above the caster's head, like the original
        // client (own character, other players and population shells).
        self.show_skill_name_text(source_entity_id, skill_id, client_tick);

        // Official cast-start effects of the unified descriptor table: the
        // cast-time body flash, the `EF_BEGINSPELL` cone and every other
        // layer of the `Cast` phase (per-cast de-duplicated).
        self.spawn_skill_effect_phase(
            skill_id,
            SkillPhase::Cast,
            source_entity_id,
            None,
            None,
            client_tick,
            cast_time as f32,
        );

        // Asura Strike shakes the screen for the local caster (the
        // original client draws no full-screen lettering: the kanji
        // are world layers of the skill effect data).
        if skill_id == SkillId(271) {
            let is_own_cast = self
                .client_state
                .try_follow(this_entity())
                .is_some_and(|entity| entity.get_entity_id() == source_entity_id);

            if is_own_cast {
                self.player_camera.shake(0.8, 1.1);
            }
        }

        // Local player cast bars are driven by this server-confirmed
        // cast-start packet (cast_time > 0), never by local requests.
        self.start_own_cast_bar(source_entity_id, skill_id, cast_time, client_tick);

        if has_cast_time {
            // The circle has to stay for the full cast: the cast time can be
            // shorter than one ACT cycle (the animation then plays a full
            // cycle) and it can be longer than the animation (the animation
            // loops), so use the longer of both.
            let animation_duration = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == source_entity_id)
                .map(|entity| entity.skill_animation_duration_ms())
                .unwrap_or(0);
            let circle_duration = cast_animation_time.max(animation_duration);

            self.spawn_cast_circle(source_entity_id, cast_circle_color, circle_duration);
        }

        // The 2019 server sends no sound packet for most casts, so the
        // client plays the mapped skill sound itself — but only when the
        // skill actually completes, never when the cast is merely
        // acknowledged. A cast acknowledgment is sent *before* the skill
        // takes effect, so playing here would be heard as a casting
        // pre-sound instead of the skill's effect sound (e.g. Gloria).
        // Cast-time skills play at their completion packet below; for an
        // instant skill the completion/damage packet claims the sound in
        // `SkillNoDamage`/`DamageEffect`.
        if !has_cast_time {
            self.skill_sound_gate.remember_instant_cast(source_entity_id, skill_id, client_tick);
        } else if crate::world::skill_sound_is_cast_start(skill_id)
            && self.skill_sound_gate.claim_cast_start(source_entity_id, skill_id, client_tick)
        {
            // Dangerous Soul Collect/Zen and similar long casts use
            // the generic begin-spell sound: it belongs at the
            // start of the cast, not after the cast finished.
            let sound_position = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == source_entity_id)
                .map(|entity| entity.get_position());
            self.skill_sounds.play_skill_sound_for_phase(
                skill_id,
                crate::world::SkillPhase::Success,
                Some(source_entity_id),
                sound_position,
            );
        }
    }

    /// `NetworkEvent::SkillNoDamage`.
    pub(crate) fn handle_skill_no_damage(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SkillNoDamage {
            source_entity_id,
            // The destination is used to place target-anchored success
            // effects (Heal, Resurrection, ...) at the ally that received
            // the skill instead of on the caster.
            destination_entity_id,
            skill_id,
            level,
            success,
        } = event
        else {
            unreachable!("handle_skill_no_damage called with a different event variant");
        };

        // 2019 `ZC_USE_SKILL` (0x09CB): the caster performed an
        // instant/complete skill. Play the full cast animation once and the
        // completion effect, so self-buffs and the blacksmith crafting loop
        // become visible.
        if success {
            if self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(source_entity_id) {
                self.cast_bars.clear();
                // Pace the next action with this skill's server-side
                // after-cast act delay instead of a global constant.
                if let Some(delays) = skill_delays(skill_id, level as u16) {
                    crate::logging::log_debug!(
                        "skill delay table: skill={} level={} cast={}ms fixed={}ms aftercast_act={}ms aftercast_walk={}ms cooldown={}ms",
                        skill_id.0,
                        level,
                        delays.cast_ms,
                        delays.fixed_cast_ms,
                        delays.aftercast_act_ms,
                        delays.aftercast_walk_ms,
                        delays.cooldown_ms
                    );
                }
                self.apply_aftercast_act_delay(skill_id, level as u16, client_tick);
            }

            let source_position = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == source_entity_id)
                .map(|entity| entity.get_position());

            // World position of the skill target, when the packet names
            // one and it is not the caster. Target/`Auto` anchored
            // success effects (for example the Heal sparkles) play
            // there; without it they fall back to the caster.
            let destination_position = if destination_entity_id == source_entity_id {
                None
            } else {
                self.client_state
                    .follow(client_state().entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == destination_entity_id)
                    .map(|entity| entity.get_tile_position())
                    .and_then(|tile_position| self.map.as_ref().and_then(|map| map.get_world_position(tile_position)))
            };

            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == source_entity_id)
            {
                match use_caster_action(skill_id) {
                    // Stance/buff skills keep the idle pose (see the
                    // `SkillCast` arm above).
                    CasterAction::Idle => entity.set_idle(client_tick),
                    // This packet has no cast time: play the whole ACT skill
                    // animation once at its natural frame speed.
                    CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                    // Weapon swing, with the block chosen from the
                    // equipped weapon layer (see the `SkillCast` arm).
                    CasterAction::Attack => entity.set_attack(INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                    CasterAction::Pickup => entity.set_pickup(client_tick),
                    CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                    CasterAction::Action(action_index) => entity.set_custom_skill_action(action_index as usize, 0, false, client_tick),
                    CasterAction::Keep => {}
                }

                // Official frame range of the skill's damage animation.
                if let Some(sequence) = crate::world::frame_sequence_for_skill(skill_id) {
                    entity.set_skill_frame_sequence_with_rate(
                        sequence.first,
                        sequence.last,
                        sequence.loop_to,
                        sequence.duration_ms,
                        sequence.step_ms(),
                        client_tick,
                    );
                }
            }

            self.show_skill_name_text(source_entity_id, skill_id, client_tick);

            // Every layer of the `Success` phase of the unified descriptor
            // table: hand-written layers (Blessing angel, ...) and the
            // official layers of the same phase spawn side by side. The
            // target entity is passed so `Target`/`Auto` anchored layers
            // (heal columns, target buffs) follow the affected entity.
            self.spawn_skill_effect_phase(
                skill_id,
                SkillPhase::Success,
                source_entity_id,
                (destination_entity_id != EntityId(0)).then_some(destination_entity_id),
                destination_position,
                client_tick,
                0.0,
            );

            // Instant/self-buff casts (0x09CB) have no sound packet on
            // the 2019 server either: play the mapped official sound once
            // the server confirms that the skill completed. Instant
            // skills claimed their cast here (the step above only
            // recorded the acknowledgment), cast-time skills play their
            // single sound now.
            self.play_completion_sound(source_entity_id, skill_id, SkillPhase::Success, source_position, client_tick);
        }

        // `ZC_USE_SKILL` (0x09CB) is sent for every no-damage skill
        // and its `level` field is the skill
        // level, not a healed amount. It
        // must not be rendered as a heal number (Blessing/AGI and
        // other self-buffs showed a green "10"
        // above the caster). Real heal
        // numbers come from `ZC_RECOVERY`/0x0A27
        // (`NetworkEvent::HealEffect`).
    }

    /// `NetworkEvent::SkillCastCancelled`.
    pub(crate) fn handle_skill_cast_cancelled(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SkillCastCancelled { entity_id } = event else {
            unreachable!("handle_skill_cast_cancelled called with a different event variant");
        };

        // `ZC_DISPEL` (0x01B9, rAthena `clif_skillcastcancel`): the
        // cast of this entity was interrupted (movement, damage,
        // another skill, ...). Stop the looping cast gesture and
        // clear the local player's cast bar and skill label so a
        // cancelled cast never leaves a stuck bar or animation.
        crate::logging::log_debug!("skill cast cancelled (ZC_DISPEL / 0x01B9): entity={}", entity_id.0);

        if self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(entity_id) {
            self.cast_bars.clear();
            self.skill_name_texts.remove_entity(entity_id);
        }

        // A cancelled cast must never play its sound later: drop the
        // pending acknowledgement (instant cast) and the played marker.
        self.skill_sound_gate.clear_entity(entity_id);

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.cancel_skill_cast(client_tick);
        }
    }

    /// `NetworkEvent::SkillGroundCast`.
    pub(crate) fn handle_skill_ground_cast(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SkillGroundCast {
            entity_id,
            skill_id,
            level: _,
            position,
        } = event
        else {
            unreachable!("handle_skill_ground_cast called with a different event variant");
        };

        // Ground-skill pose packet: the ambient crafting loop uses it as a
        // fall back so at least the hammer swing is visible and audible.
        // Play the action once and the completion effect at the target cell.
        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            match caster_action(skill_id) {
                CasterAction::Idle => entity.set_idle(client_tick),
                CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                CasterAction::Attack => entity.set_attack(INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                CasterAction::Pickup => entity.set_pickup(client_tick),
                CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                CasterAction::Action(action_index) => entity.set_custom_skill_action(action_index as usize, 0, false, client_tick),
                CasterAction::Keep => {}
            }

            // Official frame range of the skill animation.
            if let Some(sequence) = crate::world::frame_sequence_for_skill(skill_id) {
                entity.set_skill_frame_sequence_with_rate(
                    sequence.first,
                    sequence.last,
                    sequence.loop_to,
                    sequence.duration_ms,
                    sequence.step_ms(),
                    client_tick,
                );
            }
        }

        self.show_skill_name_text(entity_id, skill_id, client_tick);

        let world_position = self.map.as_ref().and_then(|map| map.get_world_position(position));

        // Every layer of the `Ground` and `Success` phases of the unified
        // descriptor table. `Success` follows the ground cell when one is
        // known (the official client plays e.g. Storm Gust's effect there);
        // hand-written completions are ordinary `Success` layers now.
        for phase in [SkillPhase::Ground, SkillPhase::Success] {
            self.spawn_skill_effect_phase(skill_id, phase, entity_id, None, world_position, client_tick, 0.0);
        }

        // Ground skills complete on this packet: play the mapped sound
        // once (instant ground casts claimed their acknowledgment above).
        self.play_completion_sound(entity_id, skill_id, SkillPhase::Ground, world_position, client_tick);
    }

    /// `NetworkEvent::SkillTree`.
    pub(crate) fn handle_skill_tree(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SkillTree { skill_information } = event else {
            unreachable!("handle_skill_tree called with a different event variant");
        };

        crate::logging::log_debug!("skills: initial skill list with {} entries", skill_information.len());

        if skill_information.is_empty() {
            // An empty list is valid for a character without skills, but if the
            // server DB has learned skills it means the list was lost (stream
            // desync) - make that visible instead of silently showing an empty
            // skill tree.
            crate::logging::log_warn!("skills: server sent an empty skill list (learned skills may be missing)");
        }
        *self.client_state.follow_mut(client_state().skill_tree().skills()) =
            skill_information.into_iter().map(LearnedSkill::new).collect();

        self.merge_learned_skills_into_skill_tree(client_tick);
    }

    /// `NetworkEvent::UpdateSkillInfo`.
    pub(crate) fn handle_update_skill_info(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UpdateSkillInfo {
            skill_id,
            skill_type,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        } = event
        else {
            unreachable!("handle_update_skill_info called with a different event variant");
        };

        // Partial skill update (level up, SP cost/range change). The
        // name is not part of this packet, so only known skills are
        // updated; the tree/hotbar read the new level from this entry.
        crate::logging::log_debug!("skills: update skill {} level {}", skill_id.0, skill_level.0);
        let skills = self.client_state.follow_mut(client_state().skill_tree().skills());

        if let Some(learned_skill) = skills.iter_mut().find(|learned_skill| learned_skill.skill_id == skill_id) {
            learned_skill.skill_type = skill_type;
            learned_skill.skill_level = skill_level;
            learned_skill.spell_point_cost = spell_point_cost;
            learned_skill.attack_range = attack_range;
            learned_skill.upgradable = upgradable;
        }
    }

    /// `NetworkEvent::AddSkill`.
    pub(crate) fn handle_add_skill(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::AddSkill { skill_information } = event else {
            unreachable!("handle_add_skill called with a different event variant");
        };

        // A skill that was learned/upgraded after the initial skill list.
        // Replace the old entry so the skill tree and the hotbar pick up
        // the new level immediately, without a relog.
        let learned_skill = LearnedSkill::new(skill_information);
        crate::logging::log_debug!(
            "skills: add skill {} level {}",
            learned_skill.skill_id.0,
            learned_skill.skill_level.0
        );
        let skills = self.client_state.follow_mut(client_state().skill_tree().skills());

        match skills.iter_mut().find(|existing| existing.skill_id == learned_skill.skill_id) {
            Some(existing) => *existing = learned_skill,
            None => skills.push(learned_skill),
        }

        self.merge_learned_skills_into_skill_tree(client_tick);
    }

    /// `NetworkEvent::AutoRunSkill`.
    pub(crate) fn handle_auto_run_skill(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AutoRunSkill { skill_id, skill_level } = event else {
            unreachable!("handle_auto_run_skill called with a different event variant");
        };

        // Skill items (fly wing, casting scrolls, ...): the server only
        // tells us which skill the item wants; the client has to send the
        // skill request itself (that is what the original client does).
        // Item skills are self-targeted on this server setup (fly wing's
        // AL_TELEPORT level 1), so cast it on the local player.
        let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) else {
            return;
        };

        let _ = self.networking_system.cast_skill(skill_id, skill_level, entity_id);
    }

    /// `NetworkEvent::AddSkillUnit`.
    pub(crate) fn handle_add_skill_unit(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AddSkillUnit {
            entity_id,
            unit_id,
            position,
            skill_level,
        } = event
        else {
            unreachable!("handle_add_skill_unit called with a different event variant");
        };

        // Warp portal skill units have no sprite in the game files; the
        // client draws the classic portal effect for them (like for warp
        // NPCs), see `render_warp_portals`. Ambient NPCs cast these in the
        // Prontera plaza.
        if matches!(unit_id, UnitId::WarpWaiting | UnitId::WarpActive) {
            let world_position = self.map.as_ref().and_then(|map| map.get_world_position(position));

            if let Some(world_position) = world_position {
                // `WarpWaiting` and `WarpActive` are separate packets for the
                // same unit, so replace the previous position (this also
                // prevents duplicate visuals for the same portal). Drop a
                // provisional portal of the same cell as well.
                self.warp_skill_units.retain(|(unit_entity_id, unit_position, ..)| {
                    *unit_entity_id != entity_id
                        && (*unit_entity_id >= EntityId(0xFF00_0000) || (*unit_position - world_position).magnitude() > 8.0)
                });

                // Pre-renewal Warp Portal lasts 5 s per skill level; the
                // expiry is only a safety net (the server sends a removal
                // packet when the unit ends).
                let lifetime = Duration::from_millis(5_000 * u64::from(skill_level.max(1)) + 1_000);
                self.warp_skill_units
                    .push((entity_id, world_position, Instant::now(), Instant::now() + lifetime));
            }

            return;
        }

        let Some(map) = &self.map else {
            return;
        };

        match unit_id {
            UnitId::Firewall => {
                let Some(position) = map.get_world_position(position) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                    return;
                };

                let Ok(effect) = self.effect_loader.get_or_load("firewall.str", &self.texture_loader) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to load firewall effect", "error".red());
                    return;
                };
                let frame_timer = effect.new_frame_timer();

                self.effect_holder.add_unit(
                    Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Position(position),
                        Vector3::new(0.0, 0.0, 0.0),
                        1.0,
                        PointLightId::new(unit_id as u32),
                        Vector3::new(0.0, 6.0, 0.0),
                        Color::rgb_u8(255, 30, 0),
                        60.0,
                        true,
                    )),
                    entity_id,
                );
            }
            UnitId::Pneuma => {
                let Some(position) = map.get_world_position(position) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                    return;
                };

                let Ok(effect) = self.effect_loader.get_or_load("pneuma1.str", &self.texture_loader) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to load pneuma effect", "error".red());
                    return;
                };
                let frame_timer = effect.new_frame_timer();

                self.effect_holder.add_unit(
                    Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Position(position),
                        Vector3::new(0.0, 0.0, 0.0),
                        1.0,
                        PointLightId::new(unit_id as u32),
                        Vector3::new(0.0, 6.0, 0.0),
                        Color::rgb_u8(83, 220, 108),
                        40.0,
                        false,
                    )),
                    entity_id,
                );
            }
            _ => {}
        }
    }

    /// `NetworkEvent::RemoveSkillUnit`.
    pub(crate) fn handle_remove_skill_unit(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RemoveSkillUnit { entity_id } = event else {
            unreachable!("handle_remove_skill_unit called with a different event variant");
        };

        self.effect_holder.remove_unit(entity_id);
        self.warp_skill_units.retain(|(unit_entity_id, ..)| *unit_entity_id != entity_id);
    }

    /// `NetworkEvent::AttackFailed`.
    pub(crate) fn handle_attack_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AttackFailed {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        } = event
        else {
            unreachable!("handle_attack_failed called with a different event variant");
        };

        if let Some(map) = &self.map
                    && self.client_state.try_follow_mut(this_entity()).is_some()
                    // Make sure that the entity is on screen.
                    && self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .any(|entity| entity.get_entity_id() == target_entity_id)
                    // Re-apply the PvP/GvG/PK targeting rules: the server
                    // can report an out-of-range attack for a player that
                    // this map does not make attackable.
                    && self.can_attack_entity(target_entity_id)
                    && let Some(path) =
                        self.path_finder
                            .find_walkable_path_in_range(&TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())), player_position, target_position, attack_range)
        {
            let nearest_tile = path.last().unwrap();

            let _ = self.networking_system.player_move(WorldPosition {
                x: nearest_tile.x,
                y: nearest_tile.y,
                direction: Direction::North,
            });

            *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity {
                entity_id: target_entity_id,
            });
        }
    }

    /// `NetworkEvent::UpdateSkill`.
    pub(crate) fn handle_update_skill(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::UpdateSkill {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        } = event
        else {
            unreachable!("handle_update_skill called with a different event variant");
        };

        self.client_state.follow_mut(client_state().skill_tree()).update_skill(
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        );
    }

    /// `NetworkEvent::RemoveSkill`.
    pub(crate) fn handle_remove_skill(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RemoveSkill { skill_id } = event else {
            unreachable!("handle_remove_skill called with a different event variant");
        };

        self.client_state.follow_mut(client_state().skill_tree()).remove_skill(skill_id);
        self.client_state
            .follow_mut(client_state().skill_tree_window().chosen_skill_level())
            .remove(&skill_id);
    }

    /// `NetworkEvent::SkillUseFailed`.
    pub(crate) fn handle_skill_use_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SkillUseFailed { skill_id, cause } = event else {
            unreachable!("handle_skill_use_failed called with a different event variant");
        };

        let skill_id_text = skill_id.0.to_string();
        let message = format_message(MessageDomain::SkillUseFailed, u16::from(cause), &[("skill_id", &skill_id_text)]);

        self.push_chat_information(message);
    }

    /// `NetworkEvent::AmmunitionAction`.
    pub(crate) fn handle_ammunition_action(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AmmunitionAction { action } = event else {
            unreachable!("handle_ammunition_action called with a different event variant");
        };

        self.push_chat_information(format_message(
            MessageDomain::AmmunitionAction,
            ammunition_action_code(&action),
            &[],
        ));
    }

    /// `NetworkEvent::AmmunitionEquipped`.
    pub(crate) fn handle_ammunition_equipped(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AmmunitionEquipped { inventory_index } = event else {
            unreachable!("handle_ammunition_equipped called with a different event variant");
        };

        crate::logging::log_debug!("ammunition equipped from inventory index {:?}", inventory_index);
    }

    /// `NetworkEvent::SkillCooldown`.
    pub(crate) fn handle_skill_cooldown(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::SkillCooldown { skill_id, until } = event else {
            unreachable!("handle_skill_cooldown called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().skill_cooldowns())
            .start(skill_id, until, client_tick);
    }
}
