# How the official client generates skill effects, status effects and auras

Audit of the Ragnarok Online client against our korangar data, plus the fixes applied.

* **Ground truth 1:** [`rodecomp`](https://github.com/rodecomp/rodecomp) — a matching decompilation of
  Ragexe 2026-02-19 (x86, MSVC v142): the client's own C++ for every skill, effect and status.
* **Ground truth 2:** our own 2019 client's data files in `data.grf`
  (`data\luafiles514\lua files\skilleffectinfo\skilleffectinfolist.lub`, `skillid.lub`,
  `effectid.lub`, `skillinfoz\...`), i.e. the exact tables the CondaRO client reads.
* **Target:** `korangar/archive/data/client/skill_effects.ron` and
  `korangar/archive/data/client/status_effects.ron` (through their generators).

The audit answers one question: **for each skill, which visual effects does the official client
spawn, where, when, and what does each effect draw?**

---

## 1. The client has three separate sources for skill visuals

| # | source | data | scope |
| --- | --- | --- | --- |
| 1 | `CGameMode::ReceiveSkillUse` (skill-use packet) | C++ `case <SKILL>:` switch | 547 skills / 446 case bodies |
| 2 | `CSession::PlaySkillEffect` (target/damage path) | C++ `case <SKILL>:` switch | 217 skills (bolts, arrows, Grimtooth, ...) |
| 3 | `LaunchLuaSkillEffect` / `PlaySkillGroundEffect` | Lua `SKILL_EFFECT_INFO_LIST` | 66 skills (2015+ jobs: Rebellion, Star Emperor, Soul Reaper, Ninja, Kagerou/Oboro, Doram) |

For the pre-renewal skill set the CondaRO server uses, sources 1 and 2 are what matters: the 2019
Lua table in our own client lists only those 66 newer skills, and every classic skill is
hard-coded in the executable.

The lookup order differs per path: the damage path asks the Lua target table first and then the
C++ `PlaySkillEffect` switch; the skill-use path runs the C++ switch and its `default:` label
falls back to `LaunchLuaSkillEffect`.

### 1.1 `ReceiveSkillUse` (skill use)

1. Resolve `source` / `target` from the packet aids; the caster faces the target when they differ.
2. Set the caster pose (`SetAction` / `SetFrameSequence`, or `GetMonsterSkillInfo` for monsters).
3. Switch on `skillId`; each case spawns:
   * `source->LaunchEffect(EF_X, {0,0,0}, 0.0f)` — effect attached to the **caster**;
   * `target->LaunchEffect(EF_X, ...)` — effect attached to the **target**;
   * `g_effectMgr->Spawn(aid, EF_X, x, y, z, ...)` — effect at a **ground cell** (an all-zero
     position means "the actor's position");
   * `g_soundMgr->PlayWave(...)`, `MM_QUAKE`, `AM_DELETE_EFFECT` (the "delete then relaunch"
     pattern used by every Quicken/Overthrust buff).
4. Skills not listed fall through to `LaunchLuaSkillEffect` (source 3).

Statement counts (2026 client):

| statement | count |
| --- | ---: |
| `LaunchEffect` (caster/target) | 333 |
| `g_effectMgr->Spawn` (ground) | 232 |
| `PlayWave` | 164 |
| deletes | 28 |
| `MM_QUAKE` | 10 |
| pose/frame actions | 24 |

Examples with exact code locations:

```cpp
case AL_BLESSING:  target->LaunchEffect(EF_BLESSING, ...);            // GameModePackets.cpp:19786
case HP_ASSUMPTIO: target->LaunchEffect(EF_ASSUMPTIO, ...);           // + EF_ASSUMPTIO2
case KN_TWOHANDQUICKEN:
  source->SendMsg(nullptr, AM_DELETE_EFFECT, EF_TWOHANDQUICKEN, ...); // relaunch cleanly
  source->LaunchEffect(EF_TWOHANDQUICKEN, ...);
```

### 1.2 `PlaySkillEffect` (target / projectile effects)

235 `LaunchEffect`, 70 ground `Spawn`, 57 sounds. Characteristic shape: an effect is launched on
the **caster** and then moved to the target with
`effect->SendMsg(nullptr, AM_SETPOS, 0, &target->render.position, ...)`. In the extracted facts
each effect therefore carries a final `plays_on` anchor (174 target / 68 source of 242).

### 1.3 The 2019 Lua table (exact, from our own client)

`data\luafiles514\lua files\skilleffectinfo\skilleffectinfolist.lub` — 66 skills, read through
`skillinfo_f.lub` accessors. The `CondaRO.grf` copy contains the same 66 skills (the two files
differ only in compression).

| Lua field | anchor |
| --- | --- |
| `effectID` / `effectNum` / `waveFileName` | caster (`onTarget = true` moves it to the target) |
| `targetEffectID` / `targetEffectNum` / `targetWaveFileName` | target |
| `groundEffectID` / `groundEffectNum` | cast cell |
| `toTargetEffectID` / `toTargetEffectNum` | travelling projectile |
| `beginEffectID` / `beginMotionType` | cast begin (effect + pose) |
| `LaunchZC_USE_SKILL` | triggered on `ZC_USE_SKILL` instead of `ZC_NOTIFY_SKILL` |

Extracted facts: `tools/rodecomp_skill_facts.json` covers the C++ side; the Lua table is a
separate fact used to confirm which effect ids the 2019 client actually plays.

### 1.4 Ground scope decal

`CGroundScope::SkillScopeType(skillId)` maps a ground-targeted skill to a cell radius
(3/5/7/9/11/18/19; three special cases). The decal is `effect\magic_target.tga` except
`GC_POISONSMOKE`, `WL_COMET` and `CD_COMPETENTIA`.

---

## 2. What an effect actually draws

`CRagEffect::Init` stores the effect id, sets the **lifetime in frames** from
`EffectLifetime(id)` (`EffectTypes.cpp`, ~2459 entries, e.g. `EF_BLESSING = 150`,
`EF_SIGNUM = 999`) and selects the **`.str` screen effect** for ids that have one
(226 ids, e.g. `EF_PROVOKE → Provoke.str`, `EF_HAMMERFALL → CrashEarth.str`, including the
`_min` alternates).

The visual logic is a per-effect method (`CRagEffect::Blessing`, `CHealEffect`, ...) that runs
every frame and emits **primitives** through `LaunchEffectPrim(PP_*, pos)`:

| primitive | meaning |
| --- | --- |
| `PP_2DTEXTURE` (0), `PP_2DQUAD` (2), `PP_2DFLASH` (3) | screen-space textures |
| `PP_3DPARTICLE` (5) | 3D particle sprite (`.spr`/`.act`) |
| `PP_3DCIRCLE` (12) | ground circle |
| `PP_3DCYLINDER` (13) | cylinder |
| `PP_3DCROSSTEXTURE` (16) | camera-facing billboard |
| ... | 60+ primitive types (`EffectPrimId.h`) |

Example — the *complete* official visual of AL_BLESSING (`CRagEffect::Blessing`, 150 frames):

```
sound  effect\EF_Blessing.wav
once   PP_3DCIRCLE  texture effect\alpha_down.tga, radius 10, latitude 90,
                    rgb (32,176,232), alpha 0 -> 100 over 30 frames, fade last 30
%3<10  PP_3DPARTICLE sprite 이팩트\축복.spr/.act, offset {0,-25,0}   (head anchor)
%4     PP_3DPARTICLE sprite 이팩트\particle6.spr/.act, random spin, speed/accel, alpha 0 -> max
```

Our data played `축복`, `particle6`, a `pok2.tga` flash that the official effect does not
contain, and a hand-written `angel_fluttering.str` — and was missing the `alpha_down.tga` ground
circle. This is the class of error the audit fixes.

---

## 3. Status effects and auras

The server's status list (`ZC_MSG_STATE_CHANGE`) is stored in `g_activeSkillEffects`. The full
visual state is rebuilt by `CPlayer::ReapplySkillEffects` (called on map entry/respawn, on the
local player's actor-state rebuild, and on `ZC_NOTIFY_MAPPROPERTY2`); a single change is applied
incrementally through `AM_STATUS_CHANGE`.

`ReapplySkillEffects` is a switch over **144** statuses; **111** of them spawn a persistent
effect (61 `LaunchEffect` on the player, 50 ground/other `Spawn`). Classic ones:

| status | effect | kind |
| --- | --- | --- |
| Two-Hand Quicken (2) | `EF_MAKEBLUR` + colour key | weapon afterimage |
| Overthrust (25) / Overthrust Max (188) | `EF_MAKEBLUR` | afterimage |
| Energy Coat (31) | `EF_MAKEBLUR` | afterimage |
| Spear Quicken (68) | `EF_MAKEBLUR` | afterimage |
| One-Hand Quicken (161) | `EF_MAKEBLUR` | afterimage |
| Critical Explosion (86) | `EF_GUMGANG` | `super1..5.bmp` body loop |
| Steel Body (87) | `EF_STEELBODY` + `EF_SU_STOOP` | body loop |
| Aura Blade (103) | `EF_AURABLADE2` | render flag only |
| LK Concentration (105) | `EF_LKCONCENTRATION` | |
| Berserk (107) | `EF_REDBODY` | body tint |
| Assumptio (110, 473) | `EF_ASSUMPTIO` | render flag; visual is `EF_ASSUMPTIO2` |
| Magic Power (113) | `EF_LIGHTBLADE` | |
| Moon (123) | `EF_SPHEREWIND2` | `fire_ivy.bmp` |
| Soul Link (149) | `EF_ASURABODY` | |
| Kaite (153) | `EF_REFLECTBODY` | render flag only |
| SG Sun/Moon/Star Warm (165–167) | `EF_DOUBLEGUMGANG` + `EF_REDLIGHTBODY` | body loop |
| Daura (232), D Barrier (235) | same pairs | body loop |
| Hallucination Walk (334), Striking (445) | `EF_MAKEHALLU`/`2` | afterimage |

The weapon-blur colour is not data: `SETCOUNT_EFFECT_COLOR` carries an *effect id* that
`CRagEffect::SendMsg` maps to an RGB tint used for the afterimage:

| key | RGB |
| --- | --- |
| `EF_TWOHANDQUICKEN` | (200, 200, 0) |
| `EF_OVERTHRUST` | (250, 150, 150) |
| `EF_SPEARQUICKEN` | (200, 200, 0) |
| `EF_ENERGYCOAT` | (180, 180, 250) |
| `EF_FASTMOVE`, `EF_MAKEHALLU`, `EF_MAKEHALLU2`, `EF_MAKEFEINT` | (250, 250, 250) |

Two sentinels encode special cases for `EF_OVERTHRUST` and `EF_TWOHANDQUICKEN`, and
`EFST_NJ_BUNSINJYUTSU` passes a key with no mapping (no tint).

The **max-level aura** is a separate path (`CGameActor::UpdateMaxLevelAura`): 99/150 blue
(100,100,255), 160 yellow (255,255,100), 185 pink (255,155,155), 4th tier orange (255,155,0).

---

## 4. What was fixed

### 4.1 New official status auras (`tools/status_effect_custom.ron` → `status_effects.ron`)

The client data had **7** auras (5 of them procedural). The official table has **61** statuses
with a persistent effect; for the pre-renewal server **16** of them are now real data (15 textured,
plus the Berserk body tint):

| status | before | now |
| --- | --- | --- |
| Two-Hand Quicken (2) | none | `EF_MAKEBLUR` tint (200,200,0) |
| Overthrust (25) | none | `EF_MAKEBLUR` tint (250,150,150) |
| Energy Coat (31) | white/blue procedural | official `EF_ENERGYCOAT` tint (180,180,250) |
| Spear Quicken (68) | none | `EF_MAKEBLUR` tint (200,200,0) |
| Critical Explosion (86) | white/blue procedural | official `EF_GUMGANG` `super1..5.bmp` loop |
| Steel Body (87) | white/grey procedural | official `EF_STEELBODY` `super1..5.bmp` loop |
| Moon (123) | none | `EF_SPHEREWIND2` `fire_ivy.bmp` |
| One-Hand Quicken (161) | none | `EF_MAKEBLUR` tint (200,200,0) |
| Sun/Moon/Star Warm (165–167) | none | `EF_DOUBLEGUMGANG` `super1..5.bmp` loop |
| Overthrust Max (188) | none | `EF_MAKEBLUR` tint (250,150,150) |
| Daura (232) | none | `EF_DOUBLEGUMGANG` `super1..5.bmp` loop |
| D Barrier (235) | none | `EF_STEELBODY` `super1..5.bmp` loop |
| Berserk (**107**) | none | `EF_REDBODY` red body glow |
| Cold (437) | none | official `EF_COLDSTATUS` `얼음결정체1.tga` body aura |

Two data bugs were found and fixed on the way:

* the Berserk aura was attached to status **132**, which is `EFST_AUTOBERSERK`, not
  `EFST_BERSERK` (**107**) — the row for 107 did not exist and the status also had no name in the
  client table (a name override was added);
* `EFST_ENDURE` (1) has **no** persistent effect in the official client: its aura stays, but it is
  marked as procedural in this report, not official.

The `EF_MAKEBLUR` family has no texture of its own (the official client tints the weapon
afterimage), so those rows reproduce the **official tint** on the generic white-light texture.
Statuses whose official visual is a sprite-level flag are handled separately in section 4.4:
the colour tints and cloaking fades are real data, the weapon blend/blink and sprite outlines
are not reproduced yet.

### 4.2a Cast-begin effects (`GetSkillActionInfo`'s `effectId`)

The official client launches one more effect on the caster when a skill starts:
`CSession::GetSkillActionInfo` sets `effectId` and `LaunchSkillActorEffect`
plays it at the cast begin. 21 learnable skills have such a begin effect whose
assets were not in our data (the completion effect was there, the cast ring was
not), e.g. Concentration (`EF_INCAGIDEX`: `dex_agi_up.bmp` + `ac_center2`),
Arrow Vulcan (`EF_BEGINSPELL6`: white ring), Twilight Alchemy (`EF_TWILIGHT*`:
the thrown potion texture). They are now appended as `Cast`-phase layers on the
caster; the extracted table is `rodecomp_skill_begin_effects.json` in the audit
workspace and the merged overlay has 125 skills / 196 layers.

### 4.2 Skill-effect fidelity: new rows **and** appended official layers

`tools/gen_rodecomp_overlay.py` generates `tools/skill_effects_rodecomp.ron` (109 skills,
169 layers) from `tools/rodecomp_skill_facts.json`. Asset paths are normalised to lower case
(the official code spells the same directory `EFFECT\` and `effect\`). Skills with no client rows at all render
every effect either table maps for them (the 2019-era extras and the official ones do not
conflict because nothing is drawn today); skills with no 2019-era entry use the official ids;
the `EF_BOTTOM_DISSONANCE`/`EF_BOTTOM_UGLYDANCE` ids for `BA_DISSONANCE`/`DC_UGLYDANCE` are
curated, and `BD_ADAPTATION` is skipped because its only official mapping (`EF_DANCE2`) does not
match the skill. `tools/gen_skill_effects.py` merges it after
the official rows and **appends** to the `(skill, phase)` descriptor (it never removes a layer).

Selection rule: the official C++ switch plays an effect for the skill **and** the effect id is
also in the roBrowserLegacy-derived table our data already uses. That double confirmation keeps
the 2026-vs-2019 version risk out of the change.

Two kinds of rows are generated:

* **new descriptors** for skills that had no client row at all (55 skills / 96 layers: Soul
  Linker soul glyphs, Taekwon kicks, Star Gladiator, gunslinger/ninja projectiles, ...);
* **append rows** (`mode: append`) for skills whose descriptor existed but was missing official
  layers (37 skills / 54 layers). Appended assets are filtered per asset, not per layer, so a
  multi-texture layer contributes the frames the client does not render yet; UI item textures
  that the official client loads with `g_texMgr` (thrown potions, disarm weapons) are included,
  because they are the effect's real asset. The merge only appends, it never drops a tuned visual, so a
  skill moves closer to the official client without regressing. Examples: the official
  `alpha_down.tga` ground circle of Blessing, Magnum Break's `대폭발` sphere, Cold Bolt's
  `ring_blue`, the `fireball`/`창`/`waterball` projectiles, Grand Cross' ground rings, Aspersio's
  `particle1`.

Examples: `BS_OVERTHRUST` 113 (`EF_STONECURSE`-ring), `MO_TRIPLEATTACK` 263 (`cloud11`),
`LK_AURABLADE` 355 (`ring_white`+`ring_yellow`), `SL_*` linkers (`soul_*` textures),
`ST_FULLSTRIP` 476 (`shield`+`sword`), `GS_DISARM` 513 (weapon items), `CG_MARIONETTE` 396,
`TK_STORMKICK` 413.

---

### 4.3 Multi-frame texture layers (1 → 2 → 6 → 27 frame flipbooks)

The official client has effect components that draw a **sequence** of textures on one primitive
(`PP_3DTEXTURE`/`PP_3DCROSSTEXTURE` with `totalTexture > 1`): `EF_JUPITEL`'s
`thunder_ball_a..f`, `EF_FIREARROW`'s `불화살1..6`, the bard/dancer `w_bubble%02d` 27-frame
bubble, the `super1..5` GUMGANG body loop. The pipeline previously threw those layers away
because a `Texture` layer could only name one file.

`skill_effects.ron` now supports them:

```ron
(kind: Texture, frames: ["effect\불화살1.tga", "effect\불화살2.tga", ...], frame_ms: 80, duration_ms: 480)
```

* schema/loader: `frames` + `frame_ms` (16..=2000 ms, at most 64 frames, `.tga`/`.bmp` only;
  `file` and `frames` are mutually exclusive) in `skill_effects.rs`;
* renderer: new `world/effect/procedural/texture_sequence.rs` primitive - a camera-facing
  flipbook billboard that shows one frame per `frame_ms` and fades the whole sequence in/out;
* data: the generator expands `%02d` templates with the official `totalTexture` count and turns
  consecutive frame lists into `frames`, choosing `frame_ms` from the official lifetime;
* tests: `TextureSequence::frame_index` (advance + hold last frame), schema validation of
  `frames`/`frame_ms` (range, extension, `file`/`frames` exclusivity), and the overlay test
  suite checks every frame asset is committed. The flipbook is unit-tested but not yet verified
  in a live client session (same caveat as the other primitive layers).

Skills that now animate instead of showing nothing/one frame: `MG_FIREBOLT` (6-frame 불화살),
`HW_NAPALMVULCAN` (lens1/2), `SL_BLACKSMITH`/`SL_SOULLINKER`/`SL_KAIZEL`/`SL_HIGH` (soul
particles), `SL_SKA` (super body frames), `WZ_JUPITEL`/`NJ_RAIGEKISAI` where the effect id is
confirmed.

### 4.4 Sprite-level status visuals (tints, fades, and what a glow cannot do)

Not every persistent status visual is a glow. The official client also manipulates the actor's
sprite directly, through `SetColor` and the `render.flags` bits consumed by
`CPc::ApplyLayerEffects`:

| status | official visual | our data |
| --- | --- | --- |
| Berserk (107), Auto Berserk (132), Blood Sucker (464) | `EF_REDBODY`: `SetColor(-1,255,100,100)` + body blend flag 1 | `BodyTint (255,100,100)` |
| LK Concentration (105) | `EF_LKCONCENTRATION`: `SetColor(-1,255,255,160)` + a `.str` | `BodyTint (255,255,160)` |
| Saturday Night Fever (448) | `EF_ENERVATION7`: `SetColor(-1,182,89,255)` | `BodyTint (182,89,255)` |
| Beyond of Warcry (453) | `EF_STASIS`: `SetColor(-1,0,0,255)` | `BodyTint (0,0,255)` |
| Camouflage (360), Stealth Field (379) | `EF_CLOAKING2`: `SetColor(50,-1,-1,-1)` (alpha 50/255) | `BodyFade alpha 0.20` |
| Shadow Form (394) | `EF_CLOAKING3`: `SetColor(50,0,0,0)` | `BodyFade alpha 0.20` |
| Cold (437) | `EF_COLDSTATUS`: `TexturedAura(얼음결정체1.tga)` | `PersistentGlow` with the official texture |
| Aura Blade (103) | `EF_AURABLADE2`: `render.flags |= 2` → **weapon layer** blend 5 | not reproduced (needs weapon-layer blend) |
| Kaite (153) | `EF_REFLECTBODY`: `render.flags |= 0x200` | not reproduced |
| Magical Power (113) | `EF_LIGHTBLADE`: `render.flags |= 4` → weapon layer **blinks** every other frame | not reproduced |
| Soul Link (149) | `EF_ASURABODY`: `render.flags |= 0x800` → red sprite **outline** (`0x3737FF`) | not reproduced |
| Property Undead (97) | `EF_UNDEADBODY`: `render.flags |= 0x80000` → grey outline (`0x56405`) | not reproduced |

The tint/fade rows live in `korangar/archive/data/client/status_visual_effects.ron` (generated by
`tools/gen_status_visuals.py`, consumed by `World::refresh_entity_status_visual`, which sets
`Entity::set_status_tint`/`set_status_alpha`). The remaining four need new sprite renderer
features - weapon-layer additive blending, per-frame blink, and an outline pass - so they are
documented here instead of being faked with a glow.

## 5. Remaining gaps (deliberately not changed)

1. **Version-sensitive effect ids.** rodecomp is a 2026 client; 71 learnable skills have effect
   ids that differ from the roBrowser-derived table. Where the 2019 mapping is corroborated by the
   effect name it is used, with the implementation taken from the decompiled 2026 client for that
   old id: this is how the bard/dancer songs are resolved - our client plays the dedicated
   `EF_BOTTOM_LULLABY`/`RIchmankim`/... (278-294) sprites, not the 2026 `EF_BEGINSPELL_N*`
   `w_bubble%02d` swirl, and both ids still exist in the decompilation. The remaining drift
   skills are left untouched because the corroboration is missing (e.g. Enchant Poison: the
   official code says `EF_ENCHANTPOISON` (125), the 2019 table says `EF_PATTACK` (20)).
2. **Skills with rows that still miss official layers**: 10 of the 64 covered skills
   (`MO_EXPLOSIONSPIRITS` 270, `HW_SOULDRAIN` 364, `TK_COUNTER` 419, `SG_SUN/MOON/STAR_COMFORT`
   431-433, `SL_ASSASSIN` 457, `BS_ADRENALINE2` 459, `SL_HUNTER` 460, `SL_STUN` 468). These are
   drift cases whose mapping cannot be corroborated. Three drift skills *were* resolved because
   the official effect name matches the skill: `PR_TURNUNDEAD` -> `EF_TURNUNDEAD` (82),
   `AS_ENCHANTPOISON`/`AS_SPLASHER` -> `EF_ENCHANTPOISON` (125, the official switch groups the
   two). The 16 bard/dancer songs are likewise no longer a gap (their 2019 `EF_BOTTOM_*`
   sprites are rendered).
3. **Animated `.spr` frame sequences** still rely on the `.spr`/`.act` player; texture flipbooks
   (`frames`) are supported. The songs' 2026 `w_bubble%02d` swirl is a client rework and is not
   applied by design.
4. **Flag-only status auras** (Aura Blade, Kaite, Magical Power, Cloaking outlines, Undead) need
   body/weapon render changes, not a texture glow (section 4.4).
5. **Persistent `EF_MAKEBLUR` afterimages** are approximated by a tinted glow; exact reproduction
   needs an afterimage pass on the weapon sprite.
6. 21 skills with official effects are still uncovered. All of them fall in one of two groups:
   * **flag/tint-only effects** with no renderable asset: `LK_BERSERK` 359, `PF_MEMORIZE` 403,
     `SL_KAITE` 465, `SL_SKE` 471, `PF_DOUBLECASTING` 482, `CG_TAROTCARD` 489, `GS_FLING` 501,
     `GS_MADNESSCANCEL` 504, `GS_ADJUSTMENT` 505, `GS_GATLINGFEVER` 517, `CR_SHRINK` 1002
     (these need the sprite-level features of section 4.4, or a status-side tint row);
   * **uncorroborated drift**: `MO_STEELBODY` 268, `TK_TURNKICK` 417, `TK_HIGHJUMP` 426,
     `SG_SUN/MOON/STAR_WARM` 428-430, `SG_HATE` 434, `SL_KAUPE` 464, `CG_LONGINGFREEDOM` 487
     plus `BD_ADAPTATION` 304 (official `EF_DANCE2` does not match the skill, deliberately
     skipped).

---

## 5a. In-client verification

The accumulated data changes were verified in the real client against the live CondaRO server
with the built-in benchmark driver (headless, Xvfb, muted audio):

```sh
cd korangar/korangar
env -u WAYLAND_DISPLAY DISPLAY=:97 XDG_RUNTIME_DIR=/run/user/1000 \
    KORANGAR_MUTE_AUDIO=1 KORANGAR_BENCH=skills \
    KORANGAR_BENCH_USER=verify6 KORANGAR_BENCH_PASS=verifypass \
    ../target/release/korangar -v
```

The `skills` scenario logs in, injects 16 persistent status effects (the auras/fades) and then
drives 47 representative skills through the real `Client::spawn_skill_effect_phase` entry point,
450 ms apart, logging `[bench-skill] {"skill":N,"phase":"..."}` per spawn. `KORANGAR_BENCH_SKILLS`
(e.g. `"34,306,19,451,355"`) and `KORANGAR_BENCH_SKILL_DELAY` target individual effects.

Result of the verification run:

* 43 of the selected skills spawned layers (the rest are ground/hit phases without a
  descriptor); `effects_mean 25.8`, `effects_max 35` on screen, no missing-asset warning and no
  panic;
* the frames in `notes/rodecomp-verification/` show AL_BLESSING's official angel sprite above the
  head (the layer the earlier data suppressed) and a `.str` screen effect rendering through the
  same pipeline; the buff bar shows the injected auras;
* the client loaded all committed data through the real loaders (500+ descriptors, 616 statuses,
  20 auras, 15 status visuals, sounds) with only the pre-existing weather-texture warnings.

Known account quirks: `verify6`/`verify4` with `verifypass` reach the map; `verify5` stalls at
character selection and a second login on the same account reports "Already online" until the
previous session ends; the char server occasionally answers "Rejected from server" on the first
attempt (a retry works).

## 5b. Skill animations (caster motions)

The client picks the caster pose after a skill use with
`CSession::GetSkillActionInfo(skillId, effectId, motion, variant, level)` (and
`GetSkillActionInfo2` for the use path): `motion` is an **ACTORSTATEID**
(`ST_BEGIN_SKILL = 8` is the default in the first table, `ST_SKILL = 7` in the
second) and the client calls `source->SetState(motion, 1)`. The function also
overrides the *effect* id (`effectId`) for many skills.

Facts extracted from the decompilation: `tools/rodecomp_skill_motions.json`
(123 skills that override the default motion, with the state name; 35 of them
learnable pre-renewal). The table is an *audit* artifact, not client data: our
`CasterAction` (`Skill`/`Idle`/`Attack`/`Pickup`/`ReadyFight`/`Action(n)`) is an
**ACT action index**, while the official value is an actor *state*, and the
state-to-ACT-animation mapping lives in the renderer. A state can therefore only
be mapped to a named action where the semantics coincide:

| official state | plausible `CasterAction` |
| --- | --- |
| `STAND` | `Idle` |
| `ATTACK`, `ATTACKWAIT`, `THROW`, `THROW2` | `Attack` |
| `PICKUP` | `Pickup` |
| `SKILL` | `Skill` |
| `READYSHOT` | not `ReadyFight` - the archer ready-shot pose is a different animation |
| `DANCE`, `SING`, `NINJAREADY`, `GUNREADY`, `A_COUNTER`, `SG_TURN`, `SG_DOWN`, ... | job-specific poses without a named action |

Under that strict mapping only three learnable skills disagree with our table:
`AC_SHOWER` 47 and `CG_ARROWVULCAN` 394 (official `READYSHOT`, we play `Attack`),
`AM_BERSERKPITCHER` 446 (official `THROW`, we play `Idle`). They are left
unchanged on purpose: changing them needs the state-to-animation decision above,
and the in-client harness now exists to compare the poses before/after.

## 5c. Coverage audit tool

`tools/audit_rodecomp_coverage.py` joins the committed facts
(`tools/rodecomp_effect_coverage.json`: per-skill official/2019 effect ids and the version-correct
assets) with the learnable-skill snapshot and the committed data file, and classifies every
learnable pre-renewal skill:

```sh
./tools/audit_rodecomp_coverage.py              # summary
./tools/audit_rodecomp_coverage.py --list-gaps  # every partial/uncovered/flag-only skill
./tools/audit_rodecomp_coverage.py --json       # machine readable
```

Current result (457 learnable skills):

| bucket | skills | meaning |
| --- | ---: | --- |
| complete | 265 | every version-correct official asset is rendered |
| partial | 11 | has rows, misses one or more official assets |
| uncovered | 8 | official effect assets, no rows |
| flag_only | 13 | official effect renders no asset (blend/blink/outline/tint) |
| no_effect | 160 | no official effect (passive or sound-only) |

33 skills are flagged `drift` (the 2026 and 2019 effect id sets are disjoint); the songs and
`PR_TURNUNDEAD`/`AS_ENCHANTPOISON`/`AS_SPLASHER` are handled by the version-correct mapping
described above. `tools/test_rodecomp_coverage.py` keeps the facts and the classifier honest.

## 5d. Skill cast sounds

The same extraction covers the sounds the official client plays: `g_soundMgr->PlayWave(...)`
calls in the two skill switches (46 learnable skills). They are committed as
`tools/skill_sounds_rodecomp.ron` and merged by `tools/gen_skill_sounds.py` with priority
`curated > per-layer wav > rodecomp > roBrowser dump > generic`. Where the community dump and the
official client disagree the official call wins, and 15 skills that had no sound at all now play
theirs (Taekwon kicks, Soul Linker buffs, Moonlit Water Mill, ...). Every wav is validated
against the GRF archives by the generator.

## 6. Reproduction

The **sound** facts can be regenerated inside the repository from any rodecomp checkout:

```sh
./tools/extract_rodecomp_sounds.py --rodecomp /path/to/rodecomp          # rewrite the facts
./tools/extract_rodecomp_sounds.py --rodecomp /path/to/rodecomp --check  # drift guard
```

It parses `SkillId.h`, the `CGameMode::ReceiveSkillUse` and `CSession::PlaySkillEffect` bodies,
and reproduces the committed `tools/skill_sounds_rodecomp.ron` byte for byte.

The effect/asset extraction tooling lives outside this repository (it needs a rodecomp checkout
and the game GRFs):

* `rodecomp` checkout at the pin documented in `tools/rodecomp_skill_facts.json`
  (`provenance.source`);
* `skill_spawns.json` / `skill_spawns_play_skill.json` — per-skill effect statements from
  `ReceiveSkillUse` and `PlaySkillEffect` (anchor, sound, guards);
* `effect_defs.json` — 2481 effect ids: implementation, lifetime, `.str` resource, primitives,
  textures, sprites, colours;
* `status_auras.json` — 144 `ReapplySkillEffects` statuses, MAKEBLUR colour keys, level aura;
* `lua_skill_effect_info.json` — the 2019 client's own 66-skill Lua table, extracted from
  `data.grf` (32-bit Lua 5.1 is needed to load the `.lub` bytecode).

Checks in this repository:

```sh
./tools/gen_rodecomp_overlay.py --check     # overlay data up to date (67 skills)
./tools/gen_skill_effects.py --check        # 521 phases / 778 layers
./tools/gen_status_effects.py --check       # 19 aura rows / 20 statuses
./tools/gen_status_visuals.py --check      # 15 body tint/fade/loop rows
./tools/gen_skill_sounds.py --check        # 8 curated + 378 official sounds
python3 tools/test_rodecomp_overlay.py      # 6 overlay invariant tests
python3 tools/test_rodecomp_coverage.py     # 3 coverage audit tests
python3 tools/test_rodecomp_motions.py      # 3 animation-facts tests
# in-client verification (Xvfb + live server, test account):
#   KORANGAR_BENCH=skills KORANGAR_BENCH_USER=verify6 KORANGAR_BENCH_PASS=verifypass \
#   KORANGAR_BENCH_SKILLS="34,306,19,451,355" ../target/release/korangar
korangar --check-skill-effects              # 521 descriptors valid
korangar --check-status-effects             # 616 statuses valid
cargo test -p korangar --lib                # 567 passed (incl. the flipbook tests)
```

`cargo test -p korangar --test ui_consistency` still fails on
`no_hardcoded_button_words_in_user_facing_windows` and
`no_hardcoded_window_titles_in_user_facing_windows`. Both scan `src/interface`
only (untouched here) - pre-existing failures from the interface lanes in the
integration branch, not caused by this data change.
