//! `NetworkEvent` handlers for the skills family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

/// Official Blade Stop lock pose (`CGameMode::ReceiveActorPairFreeze`,
/// `GameModeNotices.cpp:922-955`).
///
/// The caster (`firstAid`) holds skill block `0x60` frame 4 for
/// `99999999` ms while the lock is active
/// (`first->SetFrameSequence(96, 4, 4, 96, 4, 1, 99999999, 0)`, `:929`) and the
/// same frame for exactly 1 ms on release (`:942`); `SetFrameSequence` names
/// the ACT block itself, so the pose is block `96` (action index `12`)
/// regardless of the actor's current `render.state`. The rooted attacker
/// (`secondAid`) gets `render.isMotionFreezed = true` instead of a frame range
/// (`:934`).
const BLADE_STOP_POSE_ACTION_INDEX: usize = 12; // 0x60 / 8
const BLADE_STOP_POSE_FRAME: u8 = 4;
const BLADE_STOP_ACTIVE_HOLD_MS: u32 = 99_999_999;
const BLADE_STOP_RELEASE_HOLD_MS: u32 = 1;
/// Grey `SetColor(-1, 64, 64, 64)` applied to both actors (`:931-936`).
const BLADE_STOP_TINT: Color = Color::rgb_u8(64, 64, 64);

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

        // Cloned so the library lookup does not conflict with the mutable
        // entity borrow below.
        let library = self.library.clone();

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == source_entity_id)
        {
            // `CGameMode::NotifySkillCasting` calls `SetState` only after the
            // `delay == 0` early return (`GameModePackets.cpp:12940-12948`): an
            // instant cast acknowledgment **never** changes the caster's pose.
            // The authoritative pose then comes from the 0x09CB completion or
            // the damage packet, which apply both the official action and the
            // per-skill `SetFrameSequence` (their handlers set the base ACT
            // block themselves, so no ack pose is needed for the sequence).
            //
            // Playing the pose here made Finger Offensive (267) restart its
            // 0x60 frames 3..4 windup on the instant `ZC_USESKILL_ACK` and
            // again on the `ZC_NOTIFY_SKILL` damage packet.
            if let Some(action) = ack_caster_action(skill_id, cast_time, false) {
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
                    // Official `ST_STAND`: the neutral block 0 pose for every
                    // actor (`WeaponActionBase(ST_STAND) = 0`).
                    CasterAction::Stand => entity.set_stand(client_tick),
                    CasterAction::Action(action_index) => {
                        entity.set_custom_skill_action(action_index as usize, cast_animation_time, has_cast_time, client_tick)
                    }
                    // Only emitted by the 0x09CB use table.
                    CasterAction::Keep => {}
                }

                // Official monster skill motion (`GetMonsterSkillInfo`): the
                // official client applies it after the player-table `SetState`
                // and before `ApplyActorSkillState`
                // (`GameModePackets.cpp:12958-12961`).
                if let Some(monster_action) = monster_skill_action_for_entity(&library, entity, skill_id) {
                    entity.set_monster_action(monster_action.action_index as usize, monster_action.looping, client_tick);
                }

                // Official cast-start pose (`ApplyActorSkillState`,
                // `GameModePackets.cpp:12868-12897`): hold the frame range for
                // the whole cast, not only when the skill resolves.
                if has_cast_time && let Some(cast_sequence) = crate::world::cast_frame_sequence_for_skill(skill_id) {
                    entity.set_skill_frame_sequence_with_rate(
                        cast_sequence.first,
                        cast_sequence.last,
                        cast_sequence.loop_to,
                        cast_sequence.duration_ms(cast_time),
                        cast_sequence.step_ms(),
                        client_tick,
                    );
                }

                // The official per-skill frame range (`SetFrameSequence`,
                // `RenderObject.cpp:625-640`) belongs to the skill-use/damage
                // lane, which applies it together with its base ACT block
                // (`handle_skill_no_damage`, `handle_damage_effect`). The ack
                // lane has no pose to restrict for an instant cast.
            }

            // Targeted casts face their target, self-casts keep the current
            // direction.
            if destination_entity_id != source_entity_id {
                entity.rotate_towards(position);
            }
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

        // `CGameMode::ApplyActorSkillState` (`GameModePackets.cpp:12868-12897`,
        // called at `:12968` after `SetState` and the monster skill action)
        // launches the hard-coded cast-start effect of a few skills. The
        // official function is skipped for instant casts: `NotifySkillCasting`
        // returns at `:12940-12942` when the packet's `delay` is 0, so only a
        // cast with a cast time reaches the `LaunchEffect` calls.
        //
        // `CH_SOULCOLLECT` is the ported case (`EF_PORTAL5`, `:12883-12886`);
        // the rows live in `archive/data/client/status_effect_launches.ron`
        // and spawn a ported `PortedEffectHolder` body.
        if has_cast_time && let Some(launch) = crate::world::skill_launch(skill_id) {
            crate::logging::log_debug!(
                "skill launch: skill {} starts {} official effect(s) ({})",
                skill_id.0,
                launch.effects.len(),
                launch.source
            );

            for (ordinal, effect_id) in launch.effects.iter().enumerate() {
                self.spawn_launched_ported_effect(*effect_id, source_entity_id, None, ordinal as u32, client_tick);
            }
        }

        // Skills whose official dispatch sends `MM_QUAKE` themselves, without
        // a ported effect body to carry the request. These are the remaining
        // hand-written quake cases from `GameModePackets.cpp`; skills that have
        // a ported effect (Fury/Steel Body/SL_SKA via EF_GUMGANG2, Earth Spike,
        // Heaven's Drive, ...) must not be listed here or they would shake
        // twice.
        const DISPATCH_QUAKE_SKILLS: &[SkillId] = &[
            SkillId(444),  // SG_FUSION
            SkillId(735),  // NPC_MANDRAGORA
            SkillId(2492), // GN_MANDRAGORA
            SkillId(2029), // GC_COUNTERSLASH
            SkillId(2009), // RK_DRAGONHOWLING
            SkillId(2017), // RK_STORMBLAST
            SkillId(359),  // LK_BERSERK
            SkillId(8206), // MS_BERSERK
            SkillId(490),  // CR_ACIDDEMONSTRATION (EF_CHEMICAL2)
            SkillId(2476), // GN_CART_TORNADO (EF_CHEMICAL2DASH)
            SkillId(2477), // GN_CARTCANNON (EF_CHEMICAL2DASH2)
            SkillId(2217), // WL_TETRAVORTEX (EF_TETRA_GROUND)
            SkillId(659),  // NPC_DRAGONFEAR (EF_DRAGONFEAR)
            SkillId(83),   // WZ_METEOR (EF_METEORSTORM)
            SkillId(85),   // WZ_VERMILION (EF_LORD)
            SkillId(484),  // HW_GRAVITATION (EF_GRAVITATION)
            SkillId(2267), // NC_SELFDESTRUCTION (EF_SUI_EXPLOSION)
        ];

        if DISPATCH_QUAKE_SKILLS.contains(&skill_id) {
            self.player_camera.shake(0.65, 0.9);
        }

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

            let library = self.library.clone();

            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == source_entity_id)
            {
                // `ReceiveSkillUse` looks the monster motion up first and
                // skips the `GetSkillActionInfo2` switch when the official
                // table has a row (`GameModePackets.cpp:18371-18405`).
                let monster_action = monster_skill_action_for_entity(&library, entity, skill_id);
                match monster_action {
                    Some(monster_action) => {
                        entity.set_monster_action(monster_action.action_index as usize, monster_action.looping, client_tick);
                    }
                    None => match use_caster_action(skill_id) {
                        // Stance/buff skills keep the idle pose (see the
                        // `SkillCast` arm above).
                        CasterAction::Idle => entity.set_idle(client_tick),
                        // Official `ST_STAND` (see the `SkillCast` arm).
                        CasterAction::Stand => entity.set_stand(client_tick),
                        // This packet has no cast time: play the whole ACT
                        // skill animation once at its natural frame speed.
                        CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                        // Weapon swing, with the block chosen from the
                        // equipped weapon layer (see the `SkillCast` arm).
                        CasterAction::Attack => entity.set_attack(INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                        CasterAction::Pickup => entity.set_pickup(client_tick),
                        CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                        CasterAction::Action(action_index) => entity.set_custom_skill_action(action_index as usize, 0, false, client_tick),
                        CasterAction::Keep => {}
                    },
                }

                // Official frame range of the skill's damage animation.
                if monster_action.is_none()
                    && let Some(sequence) = crate::world::frame_sequence_for_skill(skill_id)
                {
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
                CasterAction::Stand => entity.set_stand(client_tick),
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

    /// `NetworkEvent::ActorPairFreeze` (`ZC_BLADESTOP`, 0x01D1).
    ///
    /// Official `CGameMode::ReceiveActorPairFreeze`
    /// (`GameModeNotices.cpp:922-955`): the Blade Stop caster (`first`)
    /// holds skill block `0x60` frame 4 for `99999999` ms with a grey tint
    /// and the rooted attacker (`second`) has `render.isMotionFreezed` set,
    /// both released by a `flag == 0` packet.
    pub(crate) fn handle_actor_pair_freeze(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::ActorPairFreeze {
            first_entity_id,
            second_entity_id,
            active,
        } = event
        else {
            unreachable!("handle_actor_pair_freeze called with a different event variant");
        };

        crate::logging::log_debug!("bladestop: actor pair freeze active={active} first={first_entity_id:?} second={second_entity_id:?}");

        if active {
            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == first_entity_id)
            {
                // `SetFrameSequence(96, 4, 4, 96, 4, 1, 99999999, 0)`: the
                // official call names the ACT block, so force skill block 96
                // before installing the held frame (the preceding 0x09CB may
                // have been skipped when its `result` was 0).
                entity.set_custom_skill_action(BLADE_STOP_POSE_ACTION_INDEX, 0, false, client_tick);
                entity.set_skill_frame_sequence(
                    BLADE_STOP_POSE_FRAME,
                    BLADE_STOP_POSE_FRAME,
                    BLADE_STOP_POSE_FRAME,
                    BLADE_STOP_ACTIVE_HOLD_MS,
                    client_tick,
                );
                // `SetColor(-1, 64, 64, 64)` with `render.colorLocked = 1`.
                entity.set_status_tint(Some(BLADE_STOP_TINT));
            }

            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == second_entity_id)
            {
                // `render.isMotionFreezed = true` + the same grey tint.
                entity.freeze_motion();
                entity.set_status_tint(Some(BLADE_STOP_TINT));
            }
        } else {
            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == first_entity_id)
            {
                // `SetFrameSequence(96, 4, 4, 96, 4, 1, 1, 0)` + white tint.
                entity.set_custom_skill_action(BLADE_STOP_POSE_ACTION_INDEX, 0, false, client_tick);
                entity.set_skill_frame_sequence(
                    BLADE_STOP_POSE_FRAME,
                    BLADE_STOP_POSE_FRAME,
                    BLADE_STOP_POSE_FRAME,
                    BLADE_STOP_RELEASE_HOLD_MS,
                    client_tick,
                );
                entity.set_status_tint(None);
            }

            if let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == second_entity_id)
            {
                entity.unfreeze_motion(client_tick);
                entity.set_status_tint(None);
            }
        }
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
    pub(crate) fn handle_add_skill_unit(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::AddSkillUnit {
            entity_id,
            unit_id,
            position,
            skill_level: _,
        } = event
        else {
            unreachable!("handle_add_skill_unit called with a different event variant");
        };

        // Warp Portal skill units have no sprite: the official client
        // launches the `EF_PORTAL*` body on the unit actor from
        // `CGameActor::MsgEffect` (`MsgEffect.cpp:871-888`). `WarpActive` is
        // the opening `EF_READYPORTAL2` wave, `WarpWaiting` the open
        // `EF_PORTAL2` portal; the server morphs the first into the second, so
        // the new packet replaces the previous visual of the same unit.
        if let Some(effect_id) = crate::client::effects::warp_unit_effect(&unit_id) {
            let Some(world_position) = self.map.as_ref().and_then(|map| map.get_world_position(position)) else {
                return;
            };

            self.effect_holder.remove_unit(entity_id);

            let seed = u64::from(client_tick.0) ^ (u64::from(entity_id.0) << 32) ^ (effect_id as u32 as u64);
            self.spawn_ported_effect(effect_id, world_position, Some(entity_id), None, seed);

            return;
        }

        // Persistent ground fields: Safety Wall, Sanctuary and Magnus are
        // skill units whose official visual is a body launched on the unit
        // actor (`MsgEffect.cpp`). The warp branch above handles the portal
        // units; these are the remaining field-bearing units.
        if let Some(effect_id) = crate::client::effects::ground_unit_effect(&unit_id) {
            let Some(map) = &self.map else {
                return;
            };
            let Some(world_position) = map.get_world_position(position) else {
                return;
            };

            self.effect_holder.remove_unit(entity_id);
            let seed = u64::from(client_tick.0) ^ (u64::from(entity_id.0) << 32) ^ (effect_id as u32 as u64);
            self.spawn_ported_effect(effect_id, world_position, Some(entity_id), None, seed);
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

/// Official monster skill motion of an actor (`CSession::GetMonsterSkillInfo`,
/// `Source/Contents/Monster/MonsterXmlActions.cpp:43`).
///
/// The official lookup is keyed by the monster's lower-cased job name and the
/// skill id (`GetJobName(...)` + `_strlwr`); the client's `JobIdentity` stores
/// the same job stem in upper case. Only real monster jobs use the table
/// (`IsMonsterActor` in `SessionPredicates.cpp:57`); pets, homunculi and
/// mercenaries do not.
pub(super) fn monster_skill_action_for_entity(
    library: &crate::world::Library,
    entity: &crate::world::Entity,
    skill_id: SkillId,
) -> Option<crate::world::MonsterSkillAction> {
    if entity.get_entity_type() != crate::world::EntityType::Monster {
        return None;
    }

    let identity = library.get::<crate::world::JobIdentity>(entity.get_job_id()).to_string();
    crate::world::monster_skill_action_for_name(&identity, skill_id)
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{BLADE_STOP_ACTIVE_HOLD_MS, BLADE_STOP_POSE_ACTION_INDEX, BLADE_STOP_POSE_FRAME, BLADE_STOP_RELEASE_HOLD_MS};
    use crate::world::{CasterAction, ack_caster_action, frame_sequence_for_skill};

    /// `CGameMode::NotifySkillCasting` returns before `SetState` when the
    /// packet's `delay` is zero (`GameModePackets.cpp:12940-12948`), so an
    /// instant skill acknowledgment must not start any caster pose.
    ///
    /// Finger Offensive (267) showed the defect: its 2019/rodecomp data row
    /// carries the official damage sequence (block 0x60, frames 3..4, 1100 ms,
    /// `SetSequenceInterval(10)`; `RenderObject.cpp:5802-5812`), and the ack
    /// used to play that same sequence a second time before the damage packet
    /// replayed it.
    #[test]
    fn instant_ack_never_plays_a_caster_pose() {
        for skill_id in [266u16, 267, 269, 263, 271, 268, 270] {
            assert_eq!(
                ack_caster_action(SkillId(skill_id), 0, false),
                None,
                "instant skill {skill_id} must not play an ack pose"
            );
        }

        // A cast-time acknowledgment keeps the official `GetSkillActionInfo`
        // pose (motion 8/block 96 unless the table says otherwise).
        assert_eq!(ack_caster_action(SkillId(266), 1000, false), Some(CasterAction::Action(12)));
        assert_eq!(ack_caster_action(SkillId(271), 1500, false), Some(CasterAction::Action(11)));
    }

    /// Blade Stop (`MO_BLADESTOP`, 269) has **two** official poses:
    ///
    /// * the skill-use sequence `SetFrameSequence(0x60, 3, 3, 0x60, 3, 1, 3000,
    ///   0)` (`GameModePackets.cpp:18827-18830`), which the 0x09CB data row
    ///   already carries, and
    /// * the pair-freeze lock `SetFrameSequence(96, 4, 4, 96, 4, 1, 99999999,
    ///   0)` / release `..., 1, 0` (`GameModeNotices.cpp:929`, `:942`), which
    ///   `handle_actor_pair_freeze` applies.
    #[test]
    fn blade_stop_models_the_official_lock_and_release_pose() {
        assert_eq!(BLADE_STOP_POSE_ACTION_INDEX * 8, 96, "0x60 skill block");
        assert_eq!(BLADE_STOP_POSE_FRAME, 4);
        assert_eq!(BLADE_STOP_ACTIVE_HOLD_MS, 99_999_999);
        assert_eq!(BLADE_STOP_RELEASE_HOLD_MS, 1);

        let cast = frame_sequence_for_skill(SkillId(269)).expect("Blade Stop cast sequence");
        assert_eq!((cast.first, cast.last, cast.loop_to), (3, 3, 3));
        assert_eq!(cast.duration_ms, 3000);
        assert_eq!(cast.interval_ticks, 0);
    }

    /// The two monk damage skills share the official damage sequence and the
    /// block it names (`RenderObject.cpp:5802-5812`): `SetSequenceInterval(10)`
    /// followed by `SetFrameSequence(0x60, 3, 4, 0x60, 4, 1, 1100, 0)`.
    #[test]
    fn finger_offensive_uses_the_official_damage_sequence() {
        for skill_id in [266u16, 267] {
            let sequence = frame_sequence_for_skill(SkillId(skill_id)).expect("official damage sequence");
            assert_eq!((sequence.first, sequence.last, sequence.loop_to), (3, 4, 4), "skill {skill_id}");
            assert_eq!(sequence.duration_ms, 1100, "skill {skill_id}");
            assert_eq!(sequence.step_ms(), Some(167), "10 ticks of the 60 Hz timer");
        }
    }
}
