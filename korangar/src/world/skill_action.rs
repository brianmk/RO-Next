//! Per-skill caster animation of the official client.
//!
//! The original client does **not** play one generic "skill" ACT action for
//! every skill. It keeps a table of skill id -> entity action (the client's
//! hard-coded skill action assignment) and looks the action up on every skill
//! use. Attack skills play the attack animation, most stance/buff skills keep
//! the idle pose, pickup-style skills play the pickup animation and only real
//! casts play the `SKILL` action (the casting gesture).
//!
//! Korangar used to always play the `SKILL` action, which is why every skill
//! and buff looked the same. The mapping is now a **generated data file**
//! (`archive/data/client/skill_actions.ron`, see
//! [`crate::world::library`]) built from the official client's action
//! assignment as reconstructed by the roBrowser legacy client
//! (`DB/Skills/SkillAction.js`, which encodes the official client behaviour
//! including the "prevent default skill action" entries).
//!
//! Mapping of the official action names to korangar actions:
//!
//! * `ATTACK` / `ATTACK1` / `ATTACK2` / `ATTACK3` -> [`CasterAction::Attack`]
//!   (attack pose),
//! * `IDLE` (and the "prevent default skill action" entries) ->
//!   [`CasterAction::Idle`] (keep the current/idle pose),
//! * `PICKUP` -> [`CasterAction::Pickup`] (pickup animation),
//! * `READYFIGHT` -> [`CasterAction::ReadyFight`] (combat stance, `SM_ENDURE`),
//! * everything else (`SKILL`, unknown skills) -> [`CasterAction::Skill`].
//!
//! Unknown skills deliberately fall back to the casting gesture: it is a valid
//! pose for every job and never leaves the entity in a broken/black frame.

use ragnarok_packets::SkillId;
use serde::Deserialize;

use crate::world::{caster_action_for_skill, default_caster_action};

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
    /// Play the attack animation. Official client behaviour for attack skills.
    Attack,
    /// Play the pickup animation (pickup-style skills such as `HT_LANDMINE`).
    Pickup,
    /// Play the combat-ready stance (official `READYFIGHT`, `SM_ENDURE`).
    ReadyFight,
}

/// Returns the caster body animation of a skill.
///
/// Priority:
///
/// 1. the optional `action` override of the unified skill effect descriptors
///    (`archive/data/client/skill_effects.ron`),
/// 2. the official per-skill action table (`skill_actions.ron`),
/// 3. the official default action, which is the casting gesture.
///
/// The effect descriptor and the action table stay separate data sources, so a
/// skill effect override never has to duplicate the full action mapping.
pub fn caster_action(skill_id: SkillId) -> CasterAction {
    crate::world::skill_action_override(skill_id)
        .or_else(|| caster_action_for_skill(skill_id))
        .unwrap_or_else(default_caster_action)
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{CasterAction, caster_action};

    #[test]
    fn attack_skills_play_the_attack_animation() {
        // Bash, Magnum Break, Pierce, Asura Strike, Meteor Assault.
        for skill_id in [5, 7, 56, 271, 406] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Attack, "skill {}", skill_id);
        }
    }

    #[test]
    fn stance_skills_keep_the_idle_pose() {
        // Basic Skill, Incagi, Summon Spirit Sphere.
        for skill_id in [1, 29, 261] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Idle, "skill {}", skill_id);
        }
        // Fury and Zen are data-overridden to play the casting gesture so the
        // user sees the character perform an animation.
        for skill_id in [270, 401] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Skill, "skill {}", skill_id);
        }
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
        assert_eq!(caster_action(SkillId(8)), CasterAction::ReadyFight);
    }

    #[test]
    fn casts_keep_the_casting_gesture() {
        // Blessing, Heal, Warp Portal.
        for skill_id in [34, 28, 27] {
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
}
