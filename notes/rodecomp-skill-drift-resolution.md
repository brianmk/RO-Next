# Skill-effect id drift: corroborating the 2019 ids against our own client

Follow-up to `notes/rodecomp-skill-status-aura-audit.md` sections 4.2 and 5. The
2026 decompilation (`rodecomp`, Ragexe 2026-02-19) and the 2019-era skill table
our data uses (`tools/windows_client_skill_effects.json`, from roBrowserLegacy's
`SkillEffect.js`) map some skills to different effect ids. This note records, per
remaining drift skill, which id the **2019 CondaRO client** plays, and the
evidence from our own 2019 artifacts.

## Evidence sources (2019 side)

| source | what it proves | file |
| --- | --- | --- |
| `CondaRO.exe` ASCII strings | the 2019 client compiles the effect implementation whose asset paths appear | `/home/phantom/Games/ragnarok/CondaRO-linux/client/game/CondaRO.exe` |
| `CondaRO.exe` EUC-KR strings | Korean asset paths (`.spr`, `.str`) of the same implementations | same |
| `data.grf` → `data\luafiles514\lua files\skilleffectinfolist\effectid.lub` | the 2019 client's own effect-id enum for effects reachable from Lua (`EF_HITLINE4 = 429`, `EF_HITLINE5 = 430`, `EF_FLOWERCAST = 488`, `EF_BLUECASTING = 441`, `EF_BEGINSPELL = 12`, ...) | `data.grf` |
| roBrowserLegacy `SkillEffect.js` | per-skill field → effect id in the 2019-era table | `tools/windows_client_skill_effects.json` |
| roBrowserLegacy `EffectTable.js` | the 2019-era effect implementation + its own description of the effect (`name`) | `tools/windows_client_effect_layers.json` |

The effect **implementations** are taken from the decompiled 2026 client for the
chosen id (`effect_defs.json` in the audit workspace; every id still exists in
the decompilation). Asset string presence in `CondaRO.exe` proves the 2019 client
implements the effect that draws that asset; when the 2026 candidate's only asset
is shared between many effects it does not discriminate, so the effect's own
2019-era description (`EffectTable.name`) is used as the primary evidence.

## Decisions

| skill | 2026 id | 2019 id | decision | evidence |
| --- | --- | --- | --- | --- |
| 268 `MO_STEELBODY` | 261 `EF_GUMGANG2` | 254 `EF_STEELBODY` | **254** | `EffectTable` names 254 *"Mental Strength (Visual Effect)"* (the skill's English name); the 2019 table maps the skill to 254; `CondaRO.exe` has `effect\super1..5.bmp` (`0x71F17B..0x71F1CB`) |
| 270 `MO_EXPLOSIONSPIRITS` | 203 + 261 | 12 + 261 | **add 203** | `GameModePackets.cpp:18837` launches `EF_GUMGANG` (203); `EffectTable` names 203 *"Fury (Visual Effect)"* (the skill is Fury); `super1..5.bmp` present in the exe |
| 364 `HW_SOULDRAIN` | 378 + 379 | 217 `EF_ENERGYDRAIN` | **217** | `EffectTable`: 217 = *"EF_ENERGYDRAIN (Wizard Soul Drain)"*; its implementation renders `이팩트\particle1` (same asset as 378); `CondaRO.exe` has the EUC-KR `이팩트` (0x6C8960) and `particle1.spr` (0x71DD5F) |
| 374 `PF_SOULCHANGE` | 384 + 385 + 407 | 384 + 385 | **add 407** | `GameModePackets.cpp:19078-19079` launches `EF_SOULCHANGE` on caster and target; the name matches the skill and its only asset is the `.str` `사람효과`, present once in `CondaRO.exe` (EUC-KR, 0x71D858) |
| 417 `TK_TURNKICK` | 429 `EF_HITLINE4` + 466 `EF_SPINEDBODY2` | 414 `EF_SPINEDBODY` | **render 429** | `JobInheritance.cpp:2440-2441` launches `EF_SPINEDBODY2` + `EF_HITLINE4` on the target; `EF_HITLINE4 = 429` is in the 2019 `effectid.lub`; the sibling `TK_DOWNKICK` (415) already renders the equivalent `EF_HITLINE6` `white02` layer. 414/466 are the same (versioned) sprite spin, which draws no texture |
| 419 `TK_COUNTER` | 415 + 430 | — (symbolic) | **render 430** | `JobInheritance.cpp:3176-3177` launches `EF_KICKEDBODY` + `EF_HITLINE5`; `EF_HITLINE5 = 430` is in the 2019 `effectid.lub`; the tuned `new_counter_slash` `.str` layers are kept |
| 431/432/433 `SG_*_COMFORT` | 488 + 543 | 441 `EF_BLUECASTING` | **441** | `EffectTable` names 441 *"Blue cast aura (Comfort skills, Star Emperor skills)"*; the 2019 table maps all three Comfort skills to 441; its `ring_blue` cylinder is already rendered by the committed rows. The 2026 `EF_HATED (543)` pok1/pok3 additions are 2026-only |
| 434 `SG_HATE` | 543 `EF_HATED` | 487 `EF_BLACKDEVIL` (ground) | **543** | `GameModePackets.cpp:19222` launches `EF_HATED` on the target and the effect name matches `SG_HATE`; `EF_BLACKDEVIL` has no name relation to Hate; `pok1.tga`/`pok3.tga` present in the exe (0x7195DF/0x7195EF) |
| 457 `SL_ASSASIN` | 424 + 503 | — (symbolic) | **render 503** | `GameModePackets.cpp:19433-19437` groups every SL_* link buff on `EF_SOULLINK` + `EF_ASURABODY`; the 2019 table itself maps the siblings `SL_BLACKSMITH`/`SL_SOULLINKER`/`SL_HIGH` to `[424, 503]`; `soul_i..u` are unique to `EF_SOULLINK` and present in the exe |
| 459 `BS_ADRENALINE2` | 98 `EF_HASTEUP` | — | **98** | `GameModePackets.cpp:18854` groups `BS_ADRENALINE`/`BS_ADRENALINE2` on `EF_HASTEUP`; the 2019 table maps `BS_ADRENALINE` to the same 98 and `EffectTable` names 98 *"Adrenaline Rush"* |
| 460 `SL_HUNTER` | 424 + 503 | — (symbolic) | **render 503** | same evidence as `SL_ASSASIN` |
| 468 `SL_STUN` | 429 + 555 | 555 `EF_STIN3` | **render 429** | `JobInheritance.cpp:2461-2466` launches `EF_STIN3` on the source and `EF_HITLINE4` on the target; `EF_HITLINE4 = 429` is in the 2019 `effectid.lub`; `EffectTable` names 555 *"Estun"* (the skill) |

The songs (306-313, 319-322, 327-330) and `PR_TURNUNDEAD` / `AS_ENCHANTPOISON` /
`AS_SPLASHER` were already resolved this way before this note (audit section 5).

## Data changes

* `tools/rodecomp_skill_facts.json` - 11 new/updated rows (268, 270, 364, 374,
  417, 419, 434, 457, 459, 460, 468); each row carries its evidence string.
* `tools/skill_effects_rodecomp.ron` - regenerated (135 skills / 212 layers).
* `korangar/archive/data/client/skill_effects.ron` - regenerated (524 phases /
  793 layers). Only layers are added or replaced by the official implementation;
  the 364 row's legacy `pok3` projectile approximation is superseded by the
  effect's real asset (`이팩트\particle1`), matching how existing overlay rows
  supersede legacy primitives.
* `tools/rodecomp_effect_coverage.json` - the version-correct `used_effect_ids`/
  `used_assets` for 268 (254), 364 (217), 431-433 (441).

Coverage (`./tools/audit_rodecomp_coverage.py`): complete **283** (was 269),
partial **0** (was 11), uncovered **5** (was 8), flag_only 12, no_effect 157.
The 5 remaining uncovered skills are deliberately unchanged (see below).

## Remaining unresolved

| skill | blocker | what would decide it |
| --- | --- | --- |
| 304 `BD_ADAPTATION` | only official mapping is `EF_DANCE2` (848), whose name/semantics do not match a cast-speed self-buff; the 2019 table has no entry | the 2019 skill switch, or a client recording |
| 428/429/430 `SG_*_WARM` | conflicting 2019-era evidence: the 2019 table maps them to `EF_FLOWERCAST` (488, sound `T_안락한마법` "comfort magic"), while the 2026 client uses `EF_DOUBLEGUMGANG`+`EF_REDLIGHTBODY`+`EF_HATED2` (418/544/572) and `EF_HATED2`'s sound is `T_따듯한마법` "warm magic" (present in `CondaRO.exe`); both effect ids exist in 2019 | the 2019 skill switch (`GameModePackets`) disassembly, or a recording of the three skills |
| 487 `CG_LONGINGFREEDOM` | 2019 table says `EF_CHEMICALBODY` (500, a body-tint flag with no asset), 2026 says `EF_BEGINSPELL_N6` (843, a member of the newer `EF_BEGINSPELL_N*` family; the 2019 client's cast effects are `EF_BEGINSPELL2..7` = 54..59). Neither name matches the skill | the 2019 skill switch, or the 2019 implementation of 500 |

`TK_HIGHJUMP` (426), `SL_KAUPE` (464) and the other `flag_only` skills remain
sprite-level effects (section 4.4), not an id-drift problem.
