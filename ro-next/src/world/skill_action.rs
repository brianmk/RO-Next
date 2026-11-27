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

use ragnarok_packets::{ClientTick, JobId, SkillId};
use serde::Deserialize;

use crate::world::{AnimationState, EntityType, caster_action_for_skill, default_caster_action, use_caster_action_for_skill};

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
/// (`GameModePackets.cpp:12939-12948`), so an instant cast gets no pose from
/// the ack. `plays_sequence` exists for callers that need the base ACT block
/// of an official `SetFrameSequence` early; the production handler passes
/// `false` for sequence carriers, because the 0x09CB completion and the damage
/// packet set both the base block and the frame range themselves. Playing the
/// pose on an instant ack made Finger Offensive (267) restart its 0x60 frames
/// 3..4 windup before the damage packet replayed it.
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

/// Official `CPc::SetState(ST_SKILL)` frame range of the skill gesture.
///
/// `Player.cpp:4477-4499`: after `SetAction(JobActionBase(ST_SKILL), 6, 1)`
/// (block 96 for every job, `Player.cpp:1671-1746`) the client restricts the
/// skill block for a few job families:
///
/// * the monk-style jobs — `JT_MONK`, `JT_MONK_H` (Champion, job 4016),
///   `JT_MONK_B`, `JT_CRUSADER_H`, `JT_BARD_H`, `JT_DANCER_H`,
///   `JT_ROYAL_GUARD`, `JT_ROYAL_GUARD_H`, `JT_SURA`, `JT_SURA_H`, `JT_SURA_B`
///   and `JT_INQUISITOR` — call `SetSequenceInterval(4)` and
///   `SetFrameSequence(0x60, 0, 0, 0x60, 0, 1, 400, 0)`: block 96 frame 0 for
///   400 ms,
/// * `JT_NINJA` / `JT_NINJA_B`: block 96 frames 2..5 for 1000 ms,
/// * `JT_GUNSLINGER` / `JT_GUNSLINGER_B`: block 96 frames 0..3 for 1000 ms.
///
/// The monk range matters for the reported Champion defect: without it the
/// whole skill block plays, and block 96 frames 1..2 are exactly the Fury
/// gesture (`MO_EXPLOSIONSPIRITS` uses `SetFrameSequence(0x60, 1, 2, ...)`,
/// `GameModePackets.cpp:18839-18840`), so Blessing and Inc Agi looked like
/// Fury.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SkillGestureSequence {
    pub first: u8,
    pub last: u8,
    pub loop_to: u8,
    pub duration_ms: u32,
    pub interval_ticks: u8,
}

impl SkillGestureSequence {
    /// Duration of one sequence frame in milliseconds, from the preceding
    /// `SetSequenceInterval(4)` (`4 / 60 Hz` = 67 ms).
    pub fn step_ms(&self) -> Option<u32> {
        (self.interval_ticks > 0).then(|| (f32::from(self.interval_ticks) * 1000.0 / 60.0).round() as u32)
    }
}

/// Port job ids of the `CPc::SetState(ST_SKILL)` monk-style branch
/// (`Player.cpp:4482-4487`; the decomp's `JT_*` ids match `jobs.ron`).
const MONK_SKILL_GESTURE_JOBS: [u16; 12] = [
    15,   // JOB_MONK
    4015, // JOB_PALADIN (JT_CRUSADER_H)
    4016, // JOB_CHAMPION (JT_MONK_H)
    4020, // JOB_CLOWN (JT_BARD_H)
    4021, // JOB_GYPSY (JT_DANCER_H)
    4038, // JOB_BABY_MONK (JT_MONK_B)
    4066, // JOB_ROYAL_GUARD
    4070, // JOB_SURA
    4073, // JOB_ROYAL_GUARD_T (JT_ROYAL_GUARD_H)
    4077, // JOB_SURA_T (JT_SURA_H)
    4106, // JOB_BABY_SURA (JT_SURA_B)
    4262, // JOB_INQUISITOR
];
/// `JT_NINJA` (25) / `JT_NINJA_B` (4222): block 96 frames 2..5 for 1000 ms.
const NINJA_SKILL_GESTURE_JOBS: [u16; 2] = [25, 4222];
/// `JT_GUNSLINGER` (24) / `JT_GUNSLINGER_B` (4228): block 96 frames 0..3 for
/// 1000 ms.
const GUNSLINGER_SKILL_GESTURE_JOBS: [u16; 2] = [24, 4228];

/// Official `CPc::SetState(ST_SKILL)` frame range of `job_id`, if the job has
/// one (`Player.cpp:4468-4487`).
pub(crate) fn skill_gesture_sequence(job_id: JobId) -> Option<SkillGestureSequence> {
    let (first, last, loop_to, duration_ms) = match job_id.0 {
        job if MONK_SKILL_GESTURE_JOBS.contains(&job) => (0, 0, 0, 400),
        job if NINJA_SKILL_GESTURE_JOBS.contains(&job) => (2, 5, 5, 1000),
        job if GUNSLINGER_SKILL_GESTURE_JOBS.contains(&job) => (0, 3, 3, 1000),
        _ => return None,
    };

    Some(SkillGestureSequence {
        first,
        last,
        loop_to,
        duration_ms,
        interval_ticks: 4,
    })
}

/// Applies the official skill-cast pose ([`CasterAction::Skill`]).
///
/// The pose is the player skill block 96. For the job families of
/// [`skill_gesture_sequence`] the official client restricts it with the
/// `SetState(ST_SKILL)` frame range; the Champion therefore holds block 96
/// frame 0 (the skill gesture) instead of playing Fury's frames 1..2. A cast
/// keeps the restricted pose for its whole duration, so the sequence lasts at
/// least the packet cast time (`looping` is the cast lane).
pub(crate) fn apply_skill_pose(
    animation_state: &mut AnimationState,
    entity_type: EntityType,
    job_id: JobId,
    cast_time: u32,
    looping: bool,
    client_tick: ClientTick,
) {
    animation_state.skill(entity_type, cast_time, looping, client_tick);

    if entity_type != EntityType::Player {
        return;
    }

    let Some(gesture) = skill_gesture_sequence(job_id) else {
        return;
    };

    let duration = match looping {
        true => cast_time.max(gesture.duration_ms),
        false => gesture.duration_ms,
    };

    animation_state.frame_sequence(
        gesture.first,
        gesture.last,
        gesture.loop_to,
        duration,
        gesture.step_ms(),
        client_tick,
    );
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, JobId, SkillId};

    use super::{CasterAction, ack_caster_action, apply_skill_pose, caster_action, skill_gesture_sequence, use_caster_action};
    use crate::world::{AnimationData, AnimationState, EntityType};

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
        // (Action(12) == block 96); Chain Crush Combo has no sequence at all
        // and keeps the begin-skill default.
        for skill_id in [266, 267, 272, 273, 370, 371] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Action(12), "skill {}", skill_id);
        }
        assert_eq!(caster_action(SkillId(372)), CasterAction::Skill, "skill 372");

        // Occult Impaction / Throw Spirit Sphere share the official damage
        // sequence `SetSequenceInterval(10); SetFrameSequence(0x60, 3, 4,
        // 0x60, 4, 1, 1100, 0)` (`RenderObject.cpp:5802-5812`).
        for skill_id in [266, 267] {
            let sequence = crate::world::frame_sequence_for_skill(SkillId(skill_id)).expect("official damage sequence");
            assert_eq!(
                (sequence.first, sequence.last, sequence.loop_to),
                (3, 4, 4),
                "skill {}",
                skill_id
            );
            assert_eq!(sequence.duration_ms, 1100, "skill {}", skill_id);
            assert_eq!(sequence.interval_ticks, 10, "skill {}", skill_id);
            // 10 ticks of the 60 Hz timer: one sequence frame every 167 ms.
            assert_eq!(sequence.step_ms(), Some(167), "skill {}", skill_id);
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
        // `delay == 0` (GameModePackets.cpp:12940-12942).
        assert_eq!(ack_caster_action(SkillId(34), 0, false), None);
        assert_eq!(ack_caster_action(SkillId(5), 0, false), None);
        // A cast-time ack still plays the `GetSkillActionInfo` pose.
        assert_eq!(ack_caster_action(SkillId(5), 500, false), Some(CasterAction::Attack));
        // The escape hatch still exists for callers that need the base block
        // early; the production handler passes `false` (see
        // `client::network::skills`), so instant casts never get an ack pose.
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
        // (`GameModePackets.cpp:12940-12948`). The visible gesture therefore
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

    #[test]
    fn detect_applies_the_skill_block_on_the_arriving_ground_lane() {
        // `HT_DETECTING` (130, `SkillId.h:134`) is instant and
        // `TargetType: Ground` (`db/pre-re/skill_db.yml:130`), so the packet
        // that arrives is `ZC_NOTIFY_GROUNDSKILL` (0x0117, 18 bytes): the map
        // server calls `clif_skill_poseffect` for every ground skill that is
        // not `INF_SELF_SKILL` (`src/map/skill.cpp:14381-14384`).
        //
        // The official ground path reads `GetSkillActionInfo2`
        // (`RenderObject.cpp:1010`), whose default motion is 7
        // (`SkillActionInfo.cpp:1228-1230`); `HT_DETECTING` is a bare `break`
        // case there (`:1693`), so motion stays 7 and the default branch runs
        // `Face` + `SetState(motion, 1)` (`RenderObject.cpp:1037-1039`) ->
        // player skill block 96. Detect therefore *does* animate the caster;
        // only the instant `ZC_USESKILL_ACK` lane is pose-less by design
        // (`GameModePackets.cpp:12940-12948`).
        assert_eq!(ack_caster_action(SkillId(130), 0, false), None);
        assert_eq!(applied_block(caster_action(SkillId(130)), 0, false), 96);
        // Both action tables agree on the gesture for Detect, so the lane the
        // client applies and the lane the decompilation documents are the same
        // pose.
        assert_eq!(use_caster_action(SkillId(130)), CasterAction::Skill);
        assert_eq!(applied_block(use_caster_action(SkillId(130)), 0, false), 96);

        // No official `SetFrameSequence` or `ApplyActorSkillState` hold exists
        // for HT_DETECTING: the `RenderObject.cpp:1096` case only launches the
        // effect, and the cast-start switches never name it.
        assert!(crate::world::frame_sequence_for_skill(SkillId(130)).is_none());
        assert!(crate::world::cast_frame_sequence_for_skill(SkillId(130)).is_none());
    }

    /// Applies the pose one packet lane selects, through the same entry points
    /// as `client::network::skills` (`0x0B1A` -> `handle_skill_cast` uses
    /// [`apply_skill_pose`]; `0x09CB` -> `handle_skill_no_damage`).
    fn applied_pose(action: Option<CasterAction>, cast_time: u32, job_id: JobId) -> AnimationState {
        let entity_type = EntityType::Player;
        let mut state = AnimationState::new(entity_type, ClientTick(0));
        if let Some(action) = action {
            match action {
                CasterAction::Idle => state.idle(entity_type, ClientTick(1)),
                CasterAction::Stand => state.stand(entity_type, ClientTick(1)),
                CasterAction::Skill => apply_skill_pose(&mut state, entity_type, job_id, cast_time, cast_time > 0, ClientTick(1)),
                CasterAction::Action(index) => state.custom(entity_type, usize::from(index), cast_time, cast_time > 0, ClientTick(1)),
                other => panic!("unexpected support-skill action: {other:?}"),
            }
        }
        state
    }

    const CHAMPION: JobId = JobId(4016);

    #[test]
    fn champion_skill_gesture_is_the_official_frame_zero_hold() {
        // `CPc::SetState(ST_SKILL)` for the monk family (`Player.cpp:4482-4487`):
        // `SetSequenceInterval(4); SetFrameSequence(0x60, 0, 0, 0x60, 0, 1, 400, 0)`.
        let gesture = skill_gesture_sequence(CHAMPION).expect("Champion has the monk skill-gesture range");
        assert_eq!(
            (
                gesture.first,
                gesture.last,
                gesture.loop_to,
                gesture.duration_ms,
                gesture.interval_ticks
            ),
            (0, 0, 0, 400, 4)
        );
        assert_eq!(gesture.step_ms(), Some(67), "4 ticks of the 60 Hz timer");

        let mut instant = AnimationState::new(EntityType::Player, ClientTick(0));
        apply_skill_pose(&mut instant, EntityType::Player, CHAMPION, 0, false, ClientTick(1));
        assert_eq!(instant.action_index() * 8, 96, "the skill block, restricted to frame 0");
        assert_eq!(instant.frame_sequence_range(), Some((0, 0, 0)));
        assert_eq!(instant.frame_sequence_step(), Some(67));
        assert_eq!(instant.sequence_duration(), Some(400));

        // The cast lane holds the same frame for the whole cast.
        let mut cast = AnimationState::new(EntityType::Player, ClientTick(0));
        apply_skill_pose(&mut cast, EntityType::Player, CHAMPION, 2000, true, ClientTick(1));
        assert_eq!(cast.action_index() * 8, 96);
        assert_eq!(cast.frame_sequence_range(), Some((0, 0, 0)));
        assert_eq!(cast.sequence_duration(), Some(2000), "the gesture lasts the whole cast");

        // Other jobs keep the unrestricted skill block; Ninja and Gunslinger
        // have the two other official `ST_SKILL` ranges (`:4488-4499`).
        for job in [JobId(0), JobId(7), JobId(4014), JobId(4218)] {
            assert!(skill_gesture_sequence(job).is_none(), "job {}", job.0);
        }
        let mut knight = AnimationState::new(EntityType::Player, ClientTick(0));
        apply_skill_pose(&mut knight, EntityType::Player, JobId(7), 0, false, ClientTick(1));
        assert!(knight.frame_sequence_range().is_none());

        let ninja = skill_gesture_sequence(JobId(25)).expect("Ninja range");
        assert_eq!((ninja.first, ninja.last, ninja.loop_to, ninja.duration_ms), (2, 5, 5, 1000));
        assert_eq!(skill_gesture_sequence(JobId(4222)), Some(ninja), "Baby Ninja shares the range");
        let gunslinger = skill_gesture_sequence(JobId(24)).expect("Gunslinger range");
        assert_eq!(
            (gunslinger.first, gunslinger.last, gunslinger.loop_to, gunslinger.duration_ms),
            (0, 3, 3, 1000)
        );
        assert_eq!(
            skill_gesture_sequence(JobId(4228)),
            Some(gunslinger),
            "Baby Gunslinger shares the range"
        );
    }

    #[test]
    fn champion_blessing_and_inc_agi_select_the_official_poses() {
        // 29 AL_INCAGI. `GetSkillActionInfo` motion 7 = ST_SKILL
        // (`SkillActionInfo.cpp:796-816`): `0x0B1A` plays the Champion skill
        // gesture (block 96 frame 0), not the unrestricted block whose frames
        // 1..2 are Fury.
        let incagi_cast = applied_pose(ack_caster_action(SkillId(29), 1000, false), 1000, CHAMPION);
        assert_eq!(incagi_cast.action_index() * 8, 96);
        assert_eq!(incagi_cast.frame_sequence_range(), Some((0, 0, 0)), "Champion frame 0");
        assert_eq!(incagi_cast.frame_sequence_step(), Some(67));
        assert_eq!(incagi_cast.sequence_duration(), Some(1000), "held for the cast");

        // The 0x09CB completion reads `GetSkillActionInfo2` motion 0 =
        // ST_STAND (`SkillActionInfo.cpp:1386-1392`): neutral block 0, and no
        // lingering hold, so the caster settles.
        let incagi_use = applied_pose(Some(use_caster_action(SkillId(29))), 0, CHAMPION);
        assert_eq!(use_caster_action(SkillId(29)), CasterAction::Stand);
        assert_eq!(incagi_use.action_index() * 8, 0);
        assert!(incagi_use.frame_sequence_range().is_none());
        assert!(incagi_use.is_looping());

        // 34 AL_BLESSING is instant: `NotifySkillCasting` returns before
        // `SetState` when the packet delay is zero
        // (`GameModePackets.cpp:12940-12942`), so no ack pose.
        assert_eq!(ack_caster_action(SkillId(34), 0, false), None);
        // If the ack carries a cast time it is `GetSkillActionInfo` motion 0 =
        // ST_STAND (`SkillActionInfo.cpp:184-188`).
        assert_eq!(ack_caster_action(SkillId(34), 500, false), Some(CasterAction::Stand));
        assert_eq!(
            applied_pose(ack_caster_action(SkillId(34), 500, false), 500, CHAMPION).action_index() * 8,
            0
        );
        // The 0x09CB completion is `GetSkillActionInfo2` motion 7 = ST_SKILL
        // (`SkillActionInfo.cpp:1853-1869`): the Champion gesture, not Fury.
        let blessing = applied_pose(Some(use_caster_action(SkillId(34))), 0, CHAMPION);
        assert_eq!(use_caster_action(SkillId(34)), CasterAction::Skill);
        assert_eq!(blessing.action_index() * 8, 96);
        assert_eq!(blessing.frame_sequence_range(), Some((0, 0, 0)));
        assert_eq!(blessing.sequence_duration(), Some(400));

        // Fury keeps its own official range (`SetFrameSequence(0x60, 1, 2,
        // 0x60, 2, 1, 1000, 0)`, `GameModePackets.cpp:18839-18840`), so the
        // Blessing gesture can never be confused with the Fury motion again.
        assert_eq!(use_caster_action(SkillId(270)), CasterAction::Action(12));
        let fury = crate::world::frame_sequence_for_skill(SkillId(270)).expect("Fury range");
        assert_eq!((fury.first, fury.last), (1, 2));
        assert_ne!(
            (blessing.frame_sequence_range(), blessing.sequence_duration()),
            (Some((fury.first, fury.last, fury.loop_to)), Some(fury.duration_ms))
        );
    }

    /// Synthetic player animation data mirroring the classic ACT layout: the
    /// neutral stand block 0 has the three head-bob frames of
    /// `몸통\남\챔피온_남.act` action 0.
    fn neutral_pose_animation_data() -> AnimationData {
        use crate::world::{Animation, AnimationFrame, AnimationFramePart, fallback_frame};

        let frame = |sprite_number: usize| AnimationFrame {
            frame_parts: vec![AnimationFramePart {
                sprite_number,
                ..Default::default()
            }],
            ..fallback_frame().clone()
        };

        let mut animations: Vec<Animation> = (0..104).map(|_| Animation { frames: vec![frame(0)] }).collect();
        for direction in 0..8 {
            animations[direction] = Animation {
                frames: vec![frame(1), frame(2), frame(3)],
            };
            animations[96 + direction] = Animation {
                frames: (4..10).map(frame).collect(),
            };
        }

        AnimationData {
            animation_pair: Vec::new(),
            animations,
            delays: vec![4.0; 104],
            entity_type: EntityType::Player,
        }
    }

    /// Regression for the reported endless head movement after Dangerous Soul
    /// Collect (401): the cast holds block 96 frame 0, then the `0x09CB`
    /// completion selects the official `ST_STAND` pose. `Stand` loops with no
    /// deadline; it must stay on the stable neutral frame instead of cycling
    /// the block-0 head-bob frames forever.
    #[test]
    fn soul_collect_cast_then_stand_leaves_no_head_loop() {
        use ragnarok_packets::Direction;

        use crate::world::PlayerCamera;

        let animation_data = neutral_pose_animation_data();
        let camera = PlayerCamera::new();

        let mut state = AnimationState::new(EntityType::Player, ClientTick(0));

        // 0x0B1A: 2000 ms cast, `GetSkillActionInfo` motion 8 = ST_BEGIN_SKILL
        // (`SkillActionInfo.cpp:289`; default motion 8 at `:53`) -> block 96
        // frame 0 for the whole cast.
        apply_skill_pose(&mut state, EntityType::Player, CHAMPION, 2000, true, ClientTick(1));
        // `ApplyActorSkillState` (`GameModePackets.cpp:12883-12886`):
        // `SetFrameSequence(96, 0, 0, 96, 0, 0, 0, 0)`, the same block/frame.
        state.frame_sequence(0, 0, 0, 2000, None, ClientTick(1));

        state.update(ClientTick(1001));
        let (action, frame) = animation_data.motion(&state, &camera, Direction::South).expect("animation data");
        assert_eq!(action, 12, "the cast holds the skill block");
        assert_eq!(frame, 0, "the cast holds frame 0");

        // The cast deadline ends the pose (the official hold has no deadline,
        // but the next `SetState` replaces it either way).
        state.update(ClientTick(2001));
        assert!(animation_data.is_animation_over(&state));

        // 0x09CB: `GetSkillActionInfo2` motion 0 = ST_STAND
        // (`SkillActionInfo.cpp:1397-1402`) -> the neutral block 0.
        state.stand(EntityType::Player, ClientTick(2001));
        for tick in [2002, 2201, 2401, 100_001] {
            state.update(ClientTick(tick));
            let (action, frame) = animation_data.motion(&state, &camera, Direction::South).expect("animation data");
            assert_eq!(action, 0, "the caster is back on the neutral stand block at t={tick}");
            assert_eq!(frame, 0, "the neutral stand must not bob the head at t={tick}");
        }
    }
}
