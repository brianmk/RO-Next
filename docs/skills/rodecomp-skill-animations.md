# Skill caster animations — audit against the decompiled client

Date: 2026-11-24. Source: `github.com/rodecomp/rodecomp`
(`Source/Contents/Skills/SkillActionInfo.cpp`,
`Source/Contents/Actors/RenderObject.cpp`).

## What the official client uses

1. **Motion per skill** — `CSession::GetSkillActionInfo` (cast) and
   `GetSkillActionInfo2` (damage), stored in
   `tools/data/skill_actions_rodecomp.json` (462 skills). The motion is an
   `ACTOR_STATE`: 0 = `ST_STAND` (idle), 2 = `ST_ATTACK` (attack swing),
   5 = `ST_PICKUP`, 7 = `ST_SKILL`, 8 = `ST_BEGIN_SKILL`.
2. **Explicit frame sequence per skill** — `CRenderObject`'s skill damage
   switch calls `SetFrameSequence(block, first, last, block, loopTo, loops,
   duration, flag)` with an ACT block (`0x58` = attack, `0x60` = skill) and a
   millisecond duration. Extracted into
   `tools/data/rodecomp_skill_frame_sequences.json` (12 skills).

## Audit result

`tools/audit_skill_actions.py`:

```
skills with official motion facts: 462
  missing from skill_actions.ron:  0
  action mismatches:               0
  no single-action equivalent:    38
```

Every skill's caster action now matches the official motion, with the explicit
frame sequences applied on top:

| skill | block | frames | duration |
| --- | --- | --- | --- |
| MO_TRIPLEATTACK (263) | `0x58` attack | 0–7 loop 7 | skill anim value |
| MO_EXTREMITYFIST (271) | `0x58` attack | 0–7 loop 7 | 3500 ms |
| MO_CHAINCOMBO (272) | `0x60` skill | 5 | skill anim value |
| MO_COMBOFINISH (273) | `0x60` skill | 1–2 loop 2 | skill anim value |
| CH_PALMSTRIKE (370) | `0x60` skill | 3–4 loop 4 | 1 |
| CH_TIGERFIST (371) | `0x60` skill | 4 | skill anim value |
| SR_DRAGONCOMBO (2326) | `0x60` skill | 4 | skill anim value |
| SR_SKYNETBLOW (2327) | `0x60` skill | 5 | skill anim value |
| SR_EARTHSHAKER (2328) | `0x60` skill | 1–2 loop 2 | skill anim value |
| SR_FALLENEMPIRE (2329) | `0x60` skill | 3 | skill anim value |
| SR_TIGERCANNON (2330) | `0x60` skill | 5 | skill anim value |
| RL_FALLEN_ANGEL (2564) | `0x58` attack | 0–7 loop 7 | 3500 ms |

## Frame-sequence playback (implemented)

The client now plays the official range instead of the block start:

- **Data**: `skill_actions.ron` rows carry an optional
  `sequence: (first, last, loop_to, duration_ms)` written by
  `tools/gen_skill_actions.py` from `tools/data/rodecomp_skill_frame_sequences.json`.
- **Engine**: `AnimationState::frame_sequence` restricts the current action;
  `sequence_frame_index` plays `first..=last` and then loops
  `loop_to..=last`, while `duration_ms` bounds the total time (0 = one pass at
  the ACT frame speed). Both `get_frame` and `motion` use the same mapping.
- **Timing**: instant casts start the sequence at cast time, cast-time skills
  when the skill resolves (`SkillNoDamage`/`SkillGroundCast`), which is where
  `RenderObject.cpp` runs its damage switch.

Asura therefore plays attack frames 0..7 and then holds frame 7 for 3.5 s,
Chain Combo locks frame 5, Combo Finish frames 1..2 loop frame 2, Palm Strike
3..4, and the Star Emperor skills their own frames.

Remaining approximation: the official `isAsuraAttack` slide (route step 55,
`GameObject.cpp:2307`) is a *movement* detail the client cannot derive from the
animation data; the server-driven position updates are followed as usual.

## Support/buff skills: two lanes, one block

The buff class was re-checked after an in-client report that Blessing and
Increase Agility "play the wrong animation". The data was already right; the
block the local caster played for `GetSkillActionInfo` motion `0` was not.

`SkillActionInfo.cpp` has two independent switches:

* `GetSkillActionInfo` (`ZC_USESKILL_ACK` 0x0B1A) starts at `motion = 8`
  (`ST_BEGIN_SKILL`, block 96) at `:53`. The big magic/support group
  (`AL_HEAL`, `AL_DECAGI`, `AL_ANGELUS`, `PR_KYRIE`, `PR_GLORIA`, ...) keeps
  that default and only sets `effectId = EF_BEGINSPELL` (`:206-346`,
  `goto variant`). Two skills set a motion: `AL_BLESSING` `motion = 0`
  (`ST_STAND`) at `:184-188`, `AL_INCAGI` `motion = 7` (`ST_SKILL`) at
  `:796-815`.
* `GetSkillActionInfo2` (`ZC_USE_SKILL` 0x09CB) starts at `motion = 7` at
  `:1230`; `AL_INCAGI` sets `motion = 0` at `:1386-1391`, and `AL_HEAL`/
  `AL_BLESSING`/... set `motion = 7` at `:1853-1868`.

So Blessing and Incagi are the only support skills whose two lanes disagree:
Blessing is `Stand` on the cast ack and `Skill` on 0x09CB, Incagi the reverse.
`skill_actions.ron` carries exactly those rows (34: `action: Stand, use_action:
Skill`; 29: `action: Skill, use_action: Stand`), and neither has an
`ApplyActorSkillState` hold or a `SetFrameSequence` (`:12868-12897`).

The mismatch was in the block `CasterAction::Stand` rendered for the local
player. `CGameActor::SetState(ST_STAND)` plays `WeaponActionBase(ST_STAND) = 0`
(`RenderObject.cpp:1478`/`:3416`), and `CPc::SetState` only replaces it with the
armed `JobActionBase(ST_STAND) = 32` block when the previous state is
`ST_ATTACK`/`ST_DAMAGE`/`ST_BEGIN_SKILL`/`0x34..0x50` or while
`healthState != 0` (`Player.cpp:4404-4475`). A skill completion arrives from
`ST_SKILL`, so the official block is the neutral `0`, not the armed `32`.
`Idle` and `Stand` now both resolve to block 0; the armed stance is the explicit
`ReadyFight` block 32 and is the only standing block that draws the
weapon/shield layers (`IsReadyOrAttackAction`, `Player.cpp:1835`).

`world::skill_action::tests::support_buff_skills_apply_the_official_block_on_the_arriving_lane`
pins the ack/use lane and the applied block of Blessing (96 on 0x09CB, 0 if a
cast time ever carries the `Stand` ack), Incagi (96 on the cast ack, 0 on
completion), plus Gloria 75 as an instant-lane and Kyrie 73 as a cast-lane
control.
