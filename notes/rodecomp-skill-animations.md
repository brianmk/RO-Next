# Skill caster animations — audit against the decompiled client

Date: 2026-11-24. Source: `github.com/rodecomp/rodecomp`
(`Source/Contents/Skills/SkillActionInfo.cpp`,
`Source/Contents/Actors/RenderObject.cpp`).

## What the official client uses

1. **Motion per skill** — `CSession::GetSkillActionInfo` (cast) and
   `GetSkillActionInfo2` (damage), stored in
   `tools/skill_actions_rodecomp.json` (462 skills). The motion is an
   `ACTOR_STATE`: 0 = `ST_STAND` (idle), 2 = `ST_ATTACK` (attack swing),
   5 = `ST_PICKUP`, 7 = `ST_SKILL`, 8 = `ST_BEGIN_SKILL`.
2. **Explicit frame sequence per skill** — `CRenderObject`'s skill damage
   switch calls `SetFrameSequence(block, first, last, block, loopTo, loops,
   duration, flag)` with an ACT block (`0x58` = attack, `0x60` = skill) and a
   millisecond duration. Extracted into
   `tools/rodecomp_skill_frame_sequences.json` (12 skills).

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

## Remaining gap

The client plays the **block start** of an action, not the official
`first..last` frame range with its loop target and per-skill duration. Skills
whose sequence begins at a later frame (e.g. `MO_CHAINCOMBO` frame 5,
`CH_TIGERFIST` frame 4, `SR_EARTHSHAKER` frames 1–2) therefore still show the
block's first frames. Emulating this needs a `frame_sequence` field in
`skill_actions.ron` and frame-range playback in the entity animation system.
