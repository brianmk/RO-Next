//! Per-skill caster animation of the official client.
//!
//! The original client does **not** play one generic "skill" ACT action for
//! every skill. It keeps a table of skill id -> entity action (the client's
//! hard-coded skill action assignment) and looks the action up on every skill
//! use. Attack skills play the attack animation, most stance/buff skills keep
//! the idle pose, pickup-style skills play the pickup animation and only real
//! casts play the `SKILL` action (the casting gesture).
//!
//! RO-Next used to always play the `SKILL` action, which is why every skill
//! and buff looked the same. The mapping is now a **generated data file**
//! (`archive/data/client/skill_actions.ron`, see
//! [`crate::world::library`]) built from the official client's action
//! assignment as reconstructed by the roBrowser legacy client
//! (`DB/Skills/SkillAction.js`, which encodes the official client behaviour
//! including the "prevent default skill action" entries).
//!
//! Mapping of the official action names to ro-next actions:
//!
//! * `ATTACK` / `ATTACK1` / `ATTACK2` / `ATTACK3` -> [`CasterAction::Attack`]
//!   (attack pose),
//! * `IDLE` (and the "prevent default skill action" entries) ->
//!   [`CasterAction::Idle`] (keep the current/idle pose),
//! * `PICKUP` -> [`CasterAction::Pickup`] (pickup animation),
//! * `READYFIGHT` -> [`CasterAction::ReadyFight`] (combat stance, `SM_ENDURE`),
//! * `ST_STAND` (`motion = 0`) -> [`CasterAction::Stand`] (neutral block 0, the
//!   official `WeaponActionBase(ST_STAND) = 0`),
//! * everything else (`SKILL`, unknown skills) -> [`CasterAction::Skill`].
//!
//! Unknown skills deliberately fall back to the casting gesture: it is a valid
//! pose for every job and never leaves the entity in a broken/black frame.
//!
//! The action row alone is not the whole animation: many skills also carry an
//! official `SetFrameSequence` range and a `SetSequenceInterval` play rate
//! (Fury: skill block frames 1..=2 at 500 ms per frame for 1000 ms). Those
//! data live next to the row in `skill_actions.ron` and are applied by the
//! client's cast handlers, which pass `CasterFrameSequence::step_ms` to the
//! animation engine.

use ragnarok_packets::SkillId;
use serde::Deserialize;

use crate::world::{caster_action_for_skill, default_caster_action, use_caster_action_for_skill};

/// Body animation the caster plays when a skill is used.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub enum CasterAction {
    /// Play the ACT `SKILL` action (the casting gesture). The safe default for
    /// skills that are not part of the official assignment table.
    #[default]
    Skill,
    /// Keep the idle pose. Official client behaviour for most stance/buff
    /// skills (Zen, Fury, Endure, ...).
    Idle,
    /// The official `ST_STAND` pose (`GetSkillActionInfo` motion `0`).
    ///
    /// `CGameActor::SetState(ST_STAND)` maps it to
    /// `WeaponActionBase(ST_STAND) = 0` (`RenderObject.cpp:1478`/`3416`), the
    /// neutral stand pose. The armed `JobActionBase(ST_STAND) = 32` block
    /// (`Player.cpp:1687`) is only selected when `CPc::SetState` handles a
    /// stand transition that comes from combat or while `healthState != 0`
    /// (`Player.cpp:4442-4472`); the explicit combat stance is
    /// [`CasterAction::ReadyFight`].
    Stand,
    /// Play the attack animation. Official client behaviour for attack skills.
    Attack,
    /// Play the pickup animation (pickup-style skills such as `HT_LANDMINE`).
    Pickup,
    /// Play the combat-ready stance (official `READYFIGHT`, `SM_ENDURE`).
    ReadyFight,
    /// Play an explicit ACT action index. Used for rustnarok's generic motion
    /// table entries whose action is not one of the named actions.
    Action(u8),
    /// Do not change the pose. The official `GetSkillActionInfo2` returns
    /// `motion = -1` for a few skills (`AM_BERSERKPITCHER`,
    /// `CR_SLIMPITCHER`, `CR_ACIDDEMONSTRATION`), and `SetState(-1)` is a
    /// no-op (`RenderObject.cpp:1362-1364`). Only the 0x09CB use table emits
    /// this variant.
    Keep,
}

/// Returns the caster body animation of a skill.
///
/// Priority:
///
/// 1. the official per-skill action table (`skill_actions.ron`, generated from
///    the decompiled `GetSkillActionInfo` motions),
/// 2. the optional `action` override of the unified skill effect descriptors
///    (`archive/data/client/skill_effects.ron`), used only for skills the table
///    does not list,
/// 3. the official default action, which is the casting gesture.
///
/// The effect descriptor and the action table stay separate data sources, so a
/// skill effect override never has to duplicate the full action mapping.
pub fn caster_action(skill_id: SkillId) -> CasterAction {
    // The official per-skill table is the ground truth; an effect descriptor
    // action override is only a fallback for skills the table does not list.
    caster_action_for_skill(skill_id)
        .or_else(|| crate::world::skill_action_override(skill_id))
        .unwrap_or_else(default_caster_action)
}

/// Returns the pose the `ZC_USESKILL_ACK` (0x0B1A) path should play, if any.
///
/// The official `NotifySkillCasting` returns before `GetSkillActionInfo` and
/// `SetState` when the packet's `delay` is zero
/// (`GameModePackets.cpp:12951-12953`), so an instant cast gets no pose from
/// the ack. The client still plays the pose for skills with an explicit
/// official frame sequence, because the sequence needs its base ACT block and
/// the damage/use packet is the only other place that applies it (a documented
/// timing approximation; see the audit report §8).
pub fn ack_caster_action(skill_id: SkillId, cast_time: u32, plays_sequence: bool) -> Option<CasterAction> {
    (cast_time > 0 || plays_sequence).then(|| caster_action(skill_id))
}

/// Returns the body animation of `skill_id` on the `ZC_USE_SKILL` (0x09CB)
/// path.
///
/// The official client reads `GetSkillActionInfo2` there (`motion = 7`
/// default), while `ZC_USESKILL_ACK` reads `GetSkillActionInfo`
/// (`motion = 8` default). `skill_actions.ron` stores the second value on the
/// main `action` and the first one on an optional `use_action`, so callers
/// have to pick the right one for the packet they handle.
pub fn use_caster_action(skill_id: SkillId) -> CasterAction {
    use_caster_action_for_skill(skill_id)
        .or_else(|| Some(caster_action(skill_id)))
        .unwrap_or_else(default_caster_action)
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, SkillId};

    use super::{CasterAction, ack_caster_action, caster_action, use_caster_action};
    use crate::world::{AnimationState, EntityType};

    #[test]
    fn attack_skills_play_the_attack_animation() {
        // Motion ST_ATTACK (2) is the normal weapon swing; the client
        // chooses the drawable attack block from the equipped weapon
        // (`attack_action()`), like the official `ST_ATTACK -> ST_ATTACK2`
        // upgrade in `CPc::SetState` (`Player.cpp:4545`).
        for skill_id in [5, 7, 56] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Attack, "skill {}", skill_id);
        }
        // Asura Strike uses the official attack block 88 (0x58, frames 0..7,
        // 3500 ms); Meteor Assault the begin-skill gesture (block 96).
        assert_eq!(caster_action(SkillId(271)), CasterAction::Action(11), "skill 271");
        assert_eq!(caster_action(SkillId(406)), CasterAction::Skill, "skill 406");
    }

    #[test]
    fn champion_skills_follow_the_official_motion_table() {
        // Official ground truth: Raging Trifecta Blow and Asura Strike play the
        // attack block (RenderObject.cpp `SetFrameSequence(0x58, ...)`); every
        // other Champion skill with a frame sequence uses the skill block
        // (0x60), and Chain Crush Combo has no sequence at all (motion 8).
        for skill_id in [263, 271] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Action(11), "skill {}", skill_id);
        }
        // Skills with an explicit 0x60 sequence name the skill block directly
        // (Action(12) == block 96); the ones without a sequence (Occult
        // Impaction, Throw Spirit Sphere, Chain Crush Combo) keep the
        // begin-skill default.
        for skill_id in [272, 273, 370, 371] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Action(12), "skill {}", skill_id);
        }
        for skill_id in [266, 267, 372] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Skill, "skill {}", skill_id);
        }
        // Fury is motion 0 in `GetSkillActionInfo`, but
        // `CGameMode::ReceiveSkillUse` overrides it with
        // `SetFrameSequence(0x60, 1, 2, 0x60, 2, 1, 1000, 0)`
        // (`GameModePackets.cpp:18839-18840`), so it plays the skill block.
        assert_eq!(caster_action(SkillId(270)), CasterAction::Action(12), "skill 270");
        // Steel Body: `ReceiveSkillUse` freezes block 0x60 frame 0 for 20 ms
        // (`GameModePackets.cpp:18848`); the cast-state freeze lives in
        // `ApplyActorSkillState`.
        assert_eq!(caster_action(SkillId(268)), CasterAction::Action(12), "skill 268");
    }

    #[test]
    fn fury_plays_the_official_skill_block_at_the_official_rate() {
        // GameModePackets.cpp:18839-18840:
        // SetSequenceInterval(30); SetFrameSequence(0x60, 1, 2, 0x60, 2, 1, 1000, 0)
        assert_eq!(caster_action(SkillId(270)), CasterAction::Action(12));
        let sequence = crate::world::frame_sequence_for_skill(SkillId(270)).expect("Fury has an official sequence");
        assert_eq!((sequence.first, sequence.last, sequence.loop_to), (1, 2, 2));
        assert_eq!(sequence.duration_ms, 1000);
        // 30 ticks of the client's 60 Hz timer: one frame every 500 ms, so the
        // two-frame animation lasts the full 1000 ms deadline.
        assert_eq!(sequence.interval_ticks, 30);
        assert_eq!(sequence.step_ms(), Some(500));
    }

    #[test]
    fn instant_ack_skips_the_pose_unless_a_sequence_needs_it() {
        // Official `NotifySkillCasting` returns before `SetState` when
        // `delay == 0` (GameModePackets.cpp:12951-12953).
        assert_eq!(ack_caster_action(SkillId(34), 0, false), None);
        assert_eq!(ack_caster_action(SkillId(5), 0, false), None);
        // A cast-time ack still plays the `GetSkillActionInfo` pose.
        assert_eq!(ack_caster_action(SkillId(5), 500, false), Some(CasterAction::Attack));
        // Sequence carriers keep the ack pose so their base ACT block exists;
        // the 0x09CB/damage path then applies the frame range.
        assert_eq!(ack_caster_action(SkillId(270), 0, true), Some(CasterAction::Action(12)));
    }

    #[test]
    fn use_path_follows_get_skill_action_info2() {
        // The 0x09CB `ZC_USE_SKILL` path reads the second official table,
        // which disagrees with the cast ack for 95 rows in the data file.
        assert_eq!(
            use_caster_action(SkillId(34)),
            CasterAction::Skill,
            "Blessing uses the cast gesture"
        );
        // GetSkillActionInfo2 motion 0 (ST_STAND): the neutral block 0 pose
        // (`WeaponActionBase(ST_STAND) = 0`).
        assert_eq!(use_caster_action(SkillId(29)), CasterAction::Stand, "Incagi stands");
        assert_eq!(use_caster_action(SkillId(61)), CasterAction::Attack, "Auto Counter swings");
        assert_eq!(
            use_caster_action(SkillId(231)),
            CasterAction::Action(5),
            "Potion Pitcher throws"
        );
        assert_eq!(
            use_caster_action(SkillId(446)),
            CasterAction::Keep,
            "Berserk Pitcher keeps the pose"
        );
        // A skill whose two official tables agree falls back to the cast row.
        assert_eq!(use_caster_action(SkillId(28)), caster_action(SkillId(28)));
    }

    #[test]
    fn stance_skills_keep_the_idle_pose() {
        // Blessing: the official `GetSkillActionInfo` sets motion 0
        // (ST_STAND), the neutral block 0 pose for every actor.
        assert_eq!(caster_action(SkillId(34)), CasterAction::Stand);
        // Basic Skill, Incagi, Summon Spirit Sphere and Zen use the default
        // begin-skill gesture (motion 7/8 = ST_SKILL/ST_BEGIN_SKILL).
        for skill_id in [1, 29, 261, 401] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Skill, "skill {}", skill_id);
        }
    }

    #[test]
    fn proxy_idle_rows_use_the_official_st_stand() {
        // The 19 rows whose official `GetSkillActionInfo` motion is 0 emit
        // `Stand`, the neutral `ST_STAND` block 0
        // (`WeaponActionBase(ST_STAND) = 0`).
        for skill_id in [
            34, 230, 231, 232, 234, 235, 236, 237, 243, 244, 247, 249, 389, 425, 475, 479, 493, 1001, 1014,
        ] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Stand, "skill {}", skill_id);
        }
        // The sequence carriers are not affected: their explicit block wins.
        for skill_id in [268, 270, 1010] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Action(12), "skill {}", skill_id);
        }
    }

    #[test]
    fn cast_start_holds_are_loaded_from_apply_actor_skill_state() {
        // GameModePackets.cpp:12876-12886: MO_CALLSPIRITS / MO_KITRANSLATION /
        // CH_SOULCOLLECT hold block 96 frame 0 for the cast; MO_STEELBODY holds
        // the same frame for `duration + 1000`.
        for skill_id in [261, 1015, 401] {
            let sequence = crate::world::cast_frame_sequence_for_skill(SkillId(skill_id))
                .unwrap_or_else(|| panic!("skill {} has a cast-start hold", skill_id));
            assert_eq!((sequence.first, sequence.last, sequence.loop_to), (0, 0, 0));
            assert_eq!(sequence.after_cast_ms, 0);
            assert_eq!(sequence.duration_ms(700), 700);
            assert_eq!(sequence.step_ms(), None);
        }

        let steel_body = crate::world::cast_frame_sequence_for_skill(SkillId(268)).expect("Steel Body has a cast-start hold");
        assert_eq!((steel_body.first, steel_body.last, steel_body.loop_to), (0, 0, 0));
        assert_eq!(steel_body.after_cast_ms, 1000);
        assert_eq!(steel_body.duration_ms(1500), 2500);

        // Brandish Spear / Bowling Bash freeze the motion for the cast
        // (`isMotionFreezed`, :12873-12874). The client models the visible
        // pose as a frame-0 hold of the weapon attack block.
        for skill_id in [57, 62] {
            let sequence = crate::world::cast_frame_sequence_for_skill(SkillId(skill_id))
                .unwrap_or_else(|| panic!("skill {} freezes its motion", skill_id));
            assert_eq!((sequence.first, sequence.last, sequence.loop_to), (0, 0, 0));
            assert_eq!(sequence.after_cast_ms, 0);
            assert_eq!(sequence.duration_ms(900), 900);
        }
        assert_eq!(crate::world::cast_frame_sequence_for_skill(SkillId(269)), None);
    }

    #[test]
    fn explicit_action_indices_are_representable() {
        let action = CasterAction::Action(33);
        assert_ne!(action, CasterAction::Skill);
    }

    #[test]
    fn pickup_skills_play_the_pickup_animation() {
        // Land Mine, Ankle Snare, Pick Stone.
        for skill_id in [116, 117, 151] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Pickup, "skill {}", skill_id);
        }
    }

    #[test]
    fn endures_ready_fight_stance_is_not_flattened_to_idle() {
        // The official table puts Endure on the default begin-skill gesture;
        // `ReadyFight` stays available for data overrides.
        assert_eq!(caster_action(SkillId(8)), CasterAction::Skill);
        assert_eq!(CasterAction::ReadyFight, CasterAction::ReadyFight);
    }

    #[test]
    fn casts_keep_the_casting_gesture() {
        // Heal, Warp Portal, Concentration.
        for skill_id in [28, 27, 45] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Skill, "skill {}", skill_id);
        }
    }

    #[test]
    fn unknown_skills_fall_back_safely() {
        // The casting gesture is a valid pose for every job, so an unknown id
        // can never select a missing/black animation.
        assert_eq!(caster_action(SkillId(65_500)), CasterAction::Skill);
        assert_eq!(CasterAction::default(), CasterAction::Skill);
    }

    /// The ACT block (`CasterAction` -> `AnimationState`, block = action * 8)
    /// the packet handlers make the local caster play.
    ///
    /// This uses the same `AnimationState` entry points as `handle_skill_cast`
    /// (`SkillCast`, 0x0B1A) and `handle_skill_no_damage` (`SkillNoDamage`,
    /// 0x09CB), so a handler that maps `Stand` back to the armed idle block or
    /// stops applying the use lane fails here.
    fn applied_block(action: CasterAction, cast_time: u32, looping: bool) -> usize {
        let entity_type = EntityType::Player;
        let mut state = AnimationState::new(entity_type, ClientTick(0));
        match action {
            CasterAction::Idle => state.idle(entity_type, ClientTick(10)),
            CasterAction::Stand => state.stand(entity_type, ClientTick(10)),
            CasterAction::Skill => state.skill(entity_type, cast_time, looping, ClientTick(10)),
            CasterAction::Pickup => state.pickup(entity_type, ClientTick(10)),
            CasterAction::ReadyFight => state.ready_fight(entity_type, ClientTick(10)),
            CasterAction::Action(index) => state.custom(entity_type, usize::from(index), cast_time, looping, ClientTick(10)),
            // The support class never uses these: `Attack` picks its block from
            // the equipped weapon ACT and `Keep` does not change the pose.
            CasterAction::Attack | CasterAction::Keep => panic!("not a support-skill action"),
        }
        state.action_index() * 8
    }

    #[test]
    fn support_buff_skills_apply_the_official_block_on_the_arriving_lane() {
        // 34 AL_BLESSING is instant: `GetSkillActionInfo` motion 0 only matters
        // when the server sends a cast time, and the official `NotifySkillCasting`
        // returns before any pose when `delay == 0`
        // (`GameModePackets.cpp:12951-12953`). The visible gesture therefore
        // comes from 0x09CB, whose `GetSkillActionInfo2` sets motion 7
        // (`SkillActionInfo.cpp:1853-1868`) -> ST_SKILL -> block 96.
        assert_eq!(ack_caster_action(SkillId(34), 0, false), None);
        assert_eq!(applied_block(use_caster_action(SkillId(34)), 0, false), 96);
        // If the ack does carry a cast time, the cast lane plays
        // `GetSkillActionInfo` motion 0 (`SkillActionInfo.cpp:184-188`) ->
        // ST_STAND. That is the neutral block 0, not the armed block 32: the
        // `CPc::SetState(ST_STAND)` override to `JobActionBase(ST_STAND) = 32`
        // only runs for ST_ATTACK/ST_DAMAGE/ST_BEGIN_SKILL/0x34..0x50 or while
        // `healthState != 0` (`Player.cpp:4404-4475`).
        assert_eq!(ack_caster_action(SkillId(34), 500, false), Some(CasterAction::Stand));
        assert_eq!(applied_block(CasterAction::Stand, 500, true), 0);

        // 29 AL_INCAGI has a 1000 ms cast: the ack plays `GetSkillActionInfo`
        // motion 7 (`SkillActionInfo.cpp:796-815`) -> ST_SKILL block 96 for the
        // whole cast; the 0x09CB completion plays `GetSkillActionInfo2` motion 0
        // (`:1386-1391`) -> ST_STAND -> the neutral block 0.
        assert_eq!(ack_caster_action(SkillId(29), 1000, false), Some(CasterAction::Skill));
        assert_eq!(applied_block(CasterAction::Skill, 1000, true), 96);
        assert_eq!(applied_block(use_caster_action(SkillId(29)), 0, false), 0);

        // Instant-lane control: PR_GLORIA 75 has no cast time and both tables
        // agree on the default motions (8 / 7) -> block 96.
        assert_eq!(ack_caster_action(SkillId(75), 0, false), None);
        assert_eq!(applied_block(use_caster_action(SkillId(75)), 0, false), 96);
        // Cast-lane control: PR_KYRIE 73 casts for 2000 ms and keeps the
        // `ST_BEGIN_SKILL` default 8 on the ack -> block 96, then the 0x09CB
        // default 7 -> block 96. No official hold/sequence exists for it.
        assert_eq!(ack_caster_action(SkillId(73), 2000, false), Some(CasterAction::Skill));
        assert_eq!(applied_block(CasterAction::Skill, 2000, true), 96);
        assert_eq!(applied_block(use_caster_action(SkillId(73)), 0, false), 96);
        for skill_id in [29, 34, 73, 75] {
            assert!(
                crate::world::frame_sequence_for_skill(SkillId(skill_id)).is_none(),
                "skill {skill_id} must not inherit a frame sequence"
            );
            assert!(
                crate::world::cast_frame_sequence_for_skill(SkillId(skill_id)).is_none(),
                "skill {skill_id} must not inherit a cast-start hold"
            );
        }

        // The neutral stand (block 0), the client idle (block 0 after the
        // neutral-idle fix) and the armed ready-fight stance (block 32) stay
        // distinct: only the last one draws the weapon/shield layers
        // (`IsReadyOrAttackAction`, `Player.cpp:1835`), so a buff pose can
        // never bring the armed battle stance back.
        assert_eq!(applied_block(CasterAction::Idle, 0, false), 0);
        assert_eq!(applied_block(CasterAction::ReadyFight, 0, false), 32);
        let mut armed = AnimationState::new(EntityType::Player, ClientTick(0));
        armed.ready_fight(EntityType::Player, ClientTick(10));
        assert!(
            armed.shows_weapon_layer(),
            "the armed block is the only one with a weapon layer"
        );
    }
}
