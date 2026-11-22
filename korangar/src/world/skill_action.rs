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
//! and buff looked the same. The table below is generated from the official
//! client's action assignment as reconstructed by the roBrowser legacy client
//! (`DB/Skills/SkillAction.js`, which encodes the official client behaviour
//! including the `DEFAULT_MONK` idle default). Skills that are not listed use
//! the client's default action, which is the `SKILL` casting gesture.
//!
//! Mapping of the client action names to korangar actions:
//!
//! * `ATTACK` / `ATTACK2` / `ATTACK3` -> [`CasterAction::Attack`] (attack
//!   pose),
//! * `IDLE` (and the "prevent default skill action" entries) ->
//!   [`CasterAction::Idle`] (keep the current/idle pose),
//! * `PICKUP` -> [`CasterAction::Pickup`] (pickup animation),
//! * `READYFIGHT` -> [`CasterAction::Idle`] (combat stance, closest idle pose),
//! * everything else (`SKILL`, unknown skills) -> [`CasterAction::Skill`].

use ragnarok_packets::SkillId;

/// Body animation the caster plays when a skill is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CasterAction {
    /// Play the ACT `SKILL` action (the casting gesture).
    Skill,
    /// Keep the idle pose. Official client behaviour for most stance/buff
    /// skills (Zen, Fury, Endure, ...).
    Idle,
    /// Play the attack animation. Official client behaviour for attack skills.
    Attack,
    /// Play the pickup animation (pickup-style skills such as `TF_PICKSTONE`).
    Pickup,
}

/// Returns the caster body animation of a skill, following the official
/// client's skill action table.
pub fn caster_action(skill_id: SkillId) -> CasterAction {
    match skill_id.0 {
        // Attack skills (`ATTACK` / `ATTACK2` / `ATTACK3`).
        5 | 7 | 42 | 46 | 47 | 52 | 56 | 57 | 58 | 59 | 61 | 62 | 110 | 136 | 148 | 149 | 152 | 153 | 212 | 214 | 219 | 229 | 230 | 231
        | 232 | 250 | 251 | 253 | 263 | 271 | 272 | 273 | 316 | 324 | 337 | 338 | 366 | 368 | 370 | 371 | 372 | 379 | 382 | 394 | 397
        | 398 | 399 | 406 | 413 | 415 | 417 | 419 | 421 | 480 | 490 | 502 | 503 | 512 | 513 | 514 | 515 | 516 | 518 | 519 | 520 | 521
        | 523 | 524 | 525 | 526 | 528 | 530 | 544 | 1004 | 1009 | 2002 | 2004 | 2005 | 2006 | 2008 | 2022 | 2023 | 2029 | 2030 | 2031
        | 2034 | 2036 | 2037 | 2233 | 2236 | 2257 | 2258 | 2259 | 2260 | 2261 | 2278 | 2279 | 2280 | 2284 | 2288 | 2307 | 2308 | 2309
        | 2310 | 2312 | 2314 | 2317 | 2320 | 2321 | 2323 | 2326 | 2327 | 2329 | 2330 | 2333 | 2343 | 2476 | 2493 | 5435 | 5437 => {
            CasterAction::Attack
        }
        // Stance/buff skills that keep the idle pose (`IDLE` and the entries
        // that explicitly prevent a default skill action).
        1 | 8 | 29 | 143 | 150 | 261 | 262 | 264 | 268 | 270 | 358 | 389 | 401 | 412 | 414 | 416 | 418 | 420 | 426 | 690 | 1015 | 2264
        | 2265 | 2334 | 2335 | 2338 | 2339 | 2340 | 2341 | 2344 | 2345 | 2346 | 2347 | 2348 | 5020 | 5022 => CasterAction::Idle,
        // Pickup-style skills (`PICKUP`).
        116 | 117 | 118 | 119 | 120 | 121 | 122 | 123 | 124 | 125 | 151 | 527 | 1013 | 2238 | 2239 | 2249 | 2250 | 2251 | 2252 | 2253
        | 2254 | 2328 => CasterAction::Pickup,
        // Everything else (explicit `SKILL` entries and unknown skills) uses the
        // casting gesture.
        _ => CasterAction::Skill,
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{CasterAction, caster_action};

    #[test]
    fn stance_skills_keep_the_idle_pose() {
        // Zen, Fury, Summon Spirit Sphere and the other Monk stance skills.
        for skill_id in [1, 8, 29, 261, 262, 264, 268, 270, 401, 1015] {
            assert_eq!(
                caster_action(SkillId(skill_id)),
                CasterAction::Idle,
                "skill {} should keep the idle pose",
                skill_id
            );
        }
    }

    #[test]
    fn attack_skills_play_the_attack_animation() {
        // Bash, Magnum Break, Triple Attack, Asura Strike, Meteor Assault.
        for skill_id in [5, 7, 263, 271, 406] {
            assert_eq!(caster_action(SkillId(skill_id)), CasterAction::Attack, "skill {}", skill_id);
        }
    }

    #[test]
    fn pickup_skills_play_the_pickup_animation() {
        assert_eq!(caster_action(SkillId(116)), CasterAction::Pickup);
    }

    #[test]
    fn casts_keep_the_casting_gesture() {
        // Blessing, Heal, Warp Portal and unknown ids use the casting gesture.
        assert_eq!(caster_action(SkillId(34)), CasterAction::Skill);
        assert_eq!(caster_action(SkillId(28)), CasterAction::Skill);
        assert_eq!(caster_action(SkillId(27)), CasterAction::Skill);
        assert_eq!(caster_action(SkillId(65_500)), CasterAction::Skill);
    }
}
