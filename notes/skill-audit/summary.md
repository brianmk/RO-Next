# Pre-renewal skill effect audit (Korangar vs official client data)

Audit of **all 457 learnable pre-renewal skills** (union of the rAthena pre-re skill trees)
against the extracted official Windows-client effect/audio facts and the current Korangar
client mapping. Read-only investigation: no client Rust code, archive data or GRF was modified.

| | |
|---|---|
| Client repo | `/home/phantom/Games/ragnarok/CondaRO-linux/korangar` @ `e87268af` |
| rAthena source | `/home/phantom/popeng-llm-cpp/condaro-llm/db/pre-re/` — **note:** the path named in the task (`/home/phantom/rathena`) does not exist on this machine; the two rAthena 2019 checkouts present are byte-identical (`db/pre-re/skill_db.yml` md5 `f577daa67fe495bd0a3bf5d55dc86d6f`), so the `condaro-llm` one is used. |
| Official effect facts | `tools/windows_client_skill_effects.json` (557 skills, 401 effect ids) + full layer table `/tmp/EffectTable.js` (`roBrowserLegacy_EffectTable.js`, 635 numeric effects, all fields) |
| Official sounds | layer `wav` facts of the same table (see method note on `windows_client_skill_sounds.json`) |
| Official sprite/action facts | `tools/windows_client_sprite_effects.json`, `tools/windows_client_skill_actions.json` |
| GRF check | `CondaRO.grf` + `data.grf` + `palettes.grf`, 653,299 file names |

## 1. Result counts

| Status | Skills | Share | Meaning |
|---|---:|---:|---|
| COMPLETE | 153 | 33.5% | every official asset layer is rendered by the client |
| PARTIAL | 4 | 0.9% | some official layers rendered, some missing |
| MISSING | 7 | 1.5% | official asset layers exist but the client renders none of them |
| WRONG_ASSET | 3 | 0.7% | a hand-written/placeholder asset is shown instead of the official layers |
| NO_OFFICIAL_EFFECT | 290 | 63.5% | official table defines no renderable asset layer (passive, hard-coded or sound-only) |
| **total** | **457** | 100% | |

Official layer rows across the 457 skills: **593** (FUNC 64, HARDCODED 59, PRIM 28, SOUND 107, SPR 20, STR 110, TEXTURE 205).
Of these, **335** are asset layers (STR/SPR/TEXTURE), **305** are covered by the client, **29** official layers are not covered, and **1** official asset layer name a file absent from the GRFs.

The `PARTIAL`/`MISSING`/`WRONG_ASSET` set is small (14 skills) because the generated data files
already cover the large majority of asset effects. The important gaps are concentrated in
hand-written buff visuals and a few projectile/spawn effects; the deeper systemic issues are the
`head`-flag propagation and the persistent status auras (sections 3.4 and 3.6).

## 2. Class / job-family breakdown

Each skill can belong to several job trees (inheritance), so the family columns sum to more
than the total. `Extended` = Super Novice / Gunslinger / Ninja / Taekwon / Star Gladiator /
Soul Linker / Gangsi / Death Knight / Dark Collector.

| family | COMPLETE | PARTIAL | MISSING | WRONG_ASSET | NO_OFFICIAL_EFFECT | total skills |
|---|---:|---:|---:|---:|---:|---:|
| Novice | 24 | 1 | 0 | 2 | 30 | 57 |
| Swordman | 28 | 0 | 0 | 0 | 22 | 50 |
| Mage | 29 | 2 | 0 | 0 | 33 | 64 |
| Archer | 13 | 1 | 0 | 0 | 50 | 64 |
| Acolyte | 33 | 0 | 0 | 2 | 24 | 59 |
| Merchant | 10 | 0 | 1 | 1 | 53 | 65 |
| Thief | 18 | 1 | 2 | 0 | 31 | 52 |
| Extended | 24 | 0 | 4 | 0 | 79 | 107 |

Observation: the Merchant and Archer families have the highest `NO_OFFICIAL_EFFECT`
counts (many trap/ammo/support skills are hard-coded or sound-only in the official table),
while the Acolyte family holds two of the three wrong-asset buffs (Blessing, Increase AGI).

## 3. Findings, highest impact first

### 3.1 Wrong hand-written placeholder assets (3 skills, P0)

These skills are in `has_custom_completion` (`skill_cast_effect.rs:392-404`), which silently
discards every data-driven layer the generators produced. The hand-written completion then
shows a non-official file. The official data rows exist and are already generated, but are
skipped at runtime.

| Skill | Official effect/layers | Client now | Exact missing layers | Fix |
|---|---|---|---|---|
| 29 AL_INCAGI | TEXTURE `effect/ac_center2.tga` (attached, 1000 ms); TEXTURE `effect/ac_center2.tga` (attached, 1000 ms); TEXTURE `effect/ac_center2.tga` (attached, 1000 ms); TEXTURE `effect/agi_up.bmp` (attached, 1000 ms) | `data\texture\effect\efst_c_blessings_of_soul\blessingsofsoul.str` | TEXTURE `effect/ac_center2.tga` effect 37 *(generated row exists but is suppressed)*<br>TEXTURE `effect/ac_center2.tga` effect 37 *(generated row exists but is suppressed)*<br>TEXTURE `effect/ac_center2.tga` effect 37 *(generated row exists but is suppressed)*<br>TEXTURE `effect/agi_up.bmp` effect 37 *(generated row exists but is suppressed)* | Drop the custom arm/`has_custom_completion` entry; enable effect 37 = 3× `ac_center2.tga` bursts (1000 ms, attached, delayed/faded) + `agi_up.bmp` column. The generated rows already exist. |
| 34 AL_BLESSING | SPR `축복` (head, attached, 1500 ms); TEXTURE `particle6` (attached, 1200 ms); TEXTURE `particle6` (attached, 1200 ms); TEXTURE `effect/pok2.tga` (2500 ms) | `data\texture\effect\efst_angel_fluttering\angel_fluttering.str` | SPR `축복` effect 42<br>TEXTURE `particle6` effect 42<br>TEXTURE `particle6` effect 42<br>TEXTURE `effect/pok2.tga` effect 42 *(generated row exists but is suppressed)* | Drop the custom arm/`has_custom_completion` entry; enable the official layers: SPR `축복` (blessing, 1500 ms, head, attached, yOffset -120) + 2× sparkling `particle6` + `pok2.tga` glow. |
| 110 BS_HAMMERFALL | STR `crashearth` | `data\texture\effect\black_hammerfall.str` | STR `crashearth` effect 102 *(generated row exists but is suppressed)* | Replace the hand-written `black_hammerfall.str` with the official `crashearth.str` (already generated in `skill_effect_assets.ron`), then drop the custom arm. |

### 3.2 The three task-named buffs in detail

**Blessing (34, AL_BLESSING, Acolyte)** — official effect **42** (`EF_BLESSING`):

| # | type | file | head | attachedEntity | duration | delay | other |
|---|---|---|---|---|---|---|---|
| 1 | SPR | `축복` (blessing) | true | true | 1500 ms | delayFrame 30 | repeat, stackable, yOffset -120 (≈30 world units up) |
| 2 | 3D | `particle6` (sprite) | false | true | 1200 ms | delayOffset 300 | duplicate 6, sparkling, fade in/out, size 50 |
| 3 | 3D | `particle6` (sprite) | false | true | 1200 ms | delayOffset 400 | duplicate 6, fade in/out, size 50 |
| 4 | 3D | `effect/pok2.tga` | false | false | 2500 ms | — | blue additive glow, alphaMax 0.3, size 140 |

The **flying-angel layer is the SPR `축복`** (`data\sprite\이팩트\축복.spr` + `.act`, both present
in `CondaRO.grf`), not the `angel_fluttering.str` placeholder. The official sound is
`effect/ef_blessing` (client already maps it). The persistent `EFST_BLESSING` (id 10)
status visual is also not implemented (`status_aura.rs` only knows EFST 110/473).

**Increase AGI (29, AL_INCAGI, Acolyte)** — official effect **37** (`EF_INCAGILITY`):

| # | type | file | attachedEntity | duration | delay | other |
|---|---|---|---|---|---|---|
| 1 | 3D | `effect/ac_center2.tga` | true | 1000 ms | delayLate 500 | duplicate 7, alphaMax 1, posz 1..6, fadeOut, sizeX 2.5 |
| 2 | 3D | `effect/ac_center2.tga` | true | 1000 ms | delayOffset 400 | duplicate 3, alphaMax .75, fadeOut |
| 3 | 3D | `effect/ac_center2.tga` | true | 1000 ms | — | duplicate 10, alphaMax 1, fadeOut |
| 4 | 3D | `effect/agi_up.bmp` | true | 1000 ms | — | fadeIn/out, overlay, size 100×45, posz 0.4→3, wav `ef_incagility` |

The generated `skill_procedural_effects.ron` already contains rows for these four layers
(skill 29) but `has_custom_completion(29)` makes `spawn_skill_effect_variants` return early
(`lib.rs:2457`). The client shows `efst_c_blessings_of_soul\blessingsofsoul.str` instead.

**Angelus (33, AL_ANGELUS, Acolyte)** — official effect **41** (`EF_ANGELUS`):
STR `angelus`, **head:true**, attachedEntity:true, wav `effect/ef_angelus`. The hand-written
completion uses the same `angelus.str` and anchors it overhead, so the visual result is
**COMPLETE** — but only by accident: the official `head:true` flag is *not* propagated by the
data pipeline (section 3.4), so replacing the hand-written arm with table data today would
regress it to a feet-anchored sprite.

### 3.3 Assumptio — the reference working multi-layer/status example

`HP_ASSUMPTIO` (361) is the only skill that combines all three layers of the modern client
model and is the template the other buffs should follow:

| Layer | Source | Detail |
|---|---|---|
| one-shot `.str` | `skill_effect_assets.ron` | effect 440 `asum.str`, Success/Auto, attachedEntity:true, wav `effect/아숨프티오` |
| persistent status aura | `status_aura.rs` (`ASSUMPTIO`) | EFST 110 / 473; blue-white ground glow + rotating `bubble1.bmp` ring + `round_sphere.bmp` body glow; procedural, follows the entity, scales/sits correctly |
| transparency | effect renderer | alpha-blended quads (report `notes/korangar-effect-transparency-report.md`), no opaque placeholder |

Status: **COMPLETE** (1/1 official asset layer + aura). By contrast Blessing, Increase AGI,
Angelus and Concentration apply EFSTs 10, 12, 9, 3 but have no aura registered.

### 3.4 Systemic: official `head` (overhead) flag is not propagated

The official table marks one-shot buff layers with `head: true` (+ `yOffset` screen offset).
The `.str` generator does **not** read that flag; it uses a hand-curated skill-id set
(`OVERHEAD_SKILLS`, 43 ids, in `tools/gen_skill_effect_assets.py`) and an exception set
(`BODY_AURA_SKILLS`, 14 ids). For the pre-renewal learnable set only **9 skills** have an
official `head:true` layer, and **6 of them are not in `OVERHEAD_SKILLS`**:

| Skill | Official head layer | Client handling | Result |
|---|---|---|---|
| 31 AL_HOLYWATER | SPR `성수뜨기` effect 39 | **NOT in** `OVERHEAD_SKILLS`; data rows via spr/proc | n/a (sprite layer has its own head offset) |
| 33 AL_ANGELUS | STR `angelus` effect 41 | **NOT in** `OVERHEAD_SKILLS`; hand-written arm (overhead=true) | custom visual overhead |
| 34 AL_BLESSING | SPR `축복` effect 42 | **NOT in** `OVERHEAD_SKILLS`; hand-written arm (overhead=true) | custom visual overhead |
| 59 KN_SPEARBOOMERANG | STR `spearboomerang` effect 151 | **NOT in** `OVERHEAD_SKILLS`; data row (overhead=False) | **rendered at the feet (bug)** |
| 60 KN_TWOHANDQUICKEN | STR `twohand` effect 130 | in `OVERHEAD_SKILLS`; data row (overhead=False) | body/weapon aura (`BODY_AURA_SKILLS`) — intentional |
| 62 KN_BOWLINGBASH | STR `bowling` effect 149 | **NOT in** `OVERHEAD_SKILLS`; data row (overhead=False) | **rendered at the feet (bug)** |
| 258 CR_SPEARQUICKEN | STR `twohand` effect 250 | in `OVERHEAD_SKILLS`; data row (overhead=False) | body/weapon aura (`BODY_AURA_SKILLS`) — intentional |
| 357 LK_CONCENTRATION | STR `twohand` effect 369 | **NOT in** `OVERHEAD_SKILLS`; data row (overhead=False) | **rendered at the feet (bug)** |
| 495 KN_ONEHAND | STR `twohand` effect 130 | in `OVERHEAD_SKILLS`; data row (overhead=False) | body/weapon aura (`BODY_AURA_SKILLS`) — intentional |

Skills affected today: **KN_SPEARBOOMERANG (59)**, **KN_BOWLINGBASH (62)** and
**LK_CONCENTRATION (357)** render the official buff `.str` at the feet; AL_HOLYWATER (31)
is fine because the sprite generator already applies `head`/`yOffset`; Angelus/Blessing are
masked by the hand-written arms. The fix belongs in the data schema: carry `head`/`yOffset`
per layer instead of guessing from a skill-id list (see `recommended-data-schema.md`).

### 3.5 Multi-layer / spawn-effect gaps (PARTIAL, 4 skills)

| Skill | Official layers | Missing layers | Note |
|---|---|---|---|
| 81 WZ_SIGHTRASHER | TEXTURE `sight` ×8; STR `firehit%d` | TEXTURE `sight` ×8 | some official asset layers are missing or use a different asset |
| 17 MG_FIREBALL | TEXTURE `effect/lens1.tga` ×4; TEXTURE `effect/lens2.tga` ×4; STR `firehit%d`; TEXTURE `fireball` ×4 | TEXTURE `fireball` ×4 | some official asset layers are missing or use a different asset |
| 140 AS_VENOMDUST | TEXTURE `particle3`; STR `venomdust` | TEXTURE `particle3` | some official asset layers are missing or use a different asset |
| 118 HT_SHOCKWAVE | SPR `shockwave`; STR `shockwavehit` | — | official SPR `shockwave` absent from the GRFs; STR `shockwavehit` is mapped |


Multi-layer coverage overall is good: **48** skills have two or more official asset
layers and **42** of them are COMPLETE. The largest official
effects are mostly fully mapped (e.g. 254 CR_GRANDCROSS 25 layers [COMPLETE], 17 MG_FIREBALL 13 layers [PARTIAL], 62 KN_BOWLINGBASH 9 layers [COMPLETE], 81 WZ_SIGHTRASHER 9 layers [PARTIAL], 267 MO_FINGEROFFENSIVE 9 layers [COMPLETE], 5 SM_BASH 8 layers [COMPLETE]); the asset-layer gaps are the four PARTIAL skills listed above.

Details: Fireball's official projectile is the animated `fireball.spr` (3D `playSprite`,
4 colour-keyed copies, 333 ms each) while the client fires `fireparticle.tga`; Sightrasher's
8 `sight.spr` 3D layers have no client counterpart; Venom Dust misses the `particle3.spr`
swirl. All three `.spr` files exist in `CondaRO.grf`, so these are pure mapping gaps.

### 3.6 Persistent status auras: only Assumptio is implemented

163 learnable skills apply a status effect whose `EFST_*` icon is known from
`db/pre-re/status.yml`; `status_aura.rs` registers only **EFST 110/473** (Assumptio / party
Assumptio). Implemented: 361 HP_ASSUMPTIO.
The official client draws a persistent glow for most buffs (Blessing, Increase AGI, Angelus,
Improve Concentration, Two-Hand Quicken, Impositio, Suffragium, Aspersio, Kyrie, ...). The
aura registry was designed to be extended per status; the highest-visibility pre-renewal
candidates are EFST 10 (Blessing), 12 (Increase AGI), 9 (Angelus), 3 (Improve Concentration),
2 (Two-Hand Quicken), 15/16/17/18 (Impositio/Suffragium/Aspersio/Benedictio).

### 3.7 Missing asset layers (MISSING, 7 skills)

| Skill | Official layer | Expected asset |
|---|---|---|
| 152 TF_THROWSTONE | TEXTURE `유저인터페이스/item/돌.bmp` effect 308 | `data\texture\유저인터페이스\item\돌.bmp` |
| 230 AM_ACIDTERROR | TEXTURE `유저인터페이스/item/염산병.bmp` effect 298 | `data\texture\유저인터페이스\item\염산병.bmp` |
| 1004 AS_VENOMKNIFE | TEXTURE `유저인터페이스/item/베넘나이프.bmp` effect 600 | `data\texture\유저인터페이스\item\베넘나이프.bmp` |
| 523 NJ_SYURIKEN | TEXTURE `유저인터페이스/item/수리검.bmp` effect 613 | `data\texture\유저인터페이스\item\수리검.bmp` |
| 524 NJ_KUNAI | TEXTURE `유저인터페이스/item/쿠나이_독.bmp` effect 614 | `data\texture\유저인터페이스\item\쿠나이_독.bmp` |
| 525 NJ_HUUMA | TEXTURE `유저인터페이스/item/풍마_뇌우.bmp` effect 615 | `data\texture\유저인터페이스\item\풍마_뇌우.bmp` |
| 534 NJ_KOUENKA | STR `firehit` effect 618 | `data\texture\effect\firehit.str` |

All are cheap wins: the assets are in the GRFs and only the data row is missing.
152/230/523/524/525/1004 are thrown-item 3D billboards using item icons from
`data\texture\유저인터페이스\item\` (stone, acid bottle, shuriken, kunai, huuma, venom knife);
534 NJ_KOUENKA misses the plain `firehit.str` (the client only has the `firehit%d` template).
The client currently falls back to a generic `pok3.tga` projectile.

### 3.8 Sound facts

| sound status | skills |
|---|---:|
| COVERED | 196 |
| PARTIAL | 5 |
| MISSING | 8 |
| NO_OFFICIAL | 248 |

Official wavs missing from the GRFs: **3** unique (`effect/_enemy_hit_wind1`, `effect/bash3d`, `effect/bash3d5`).

Client skills with an official sound but **no mapping** (8): MG_COLDBOLT (14), MG_LIGHTNINGBOLT (20), SA_LIGHTNINGLOADER (282), PA_SACRIFICE (368), GS_DUST (518), NJ_KOUENKA (534), NJ_HYOUSENSOU (537), SA_ELEMENTWIND (1019).
Partial sound mapping (5): MG_FROSTDIVER (15), MG_FIREBOLT (19), WZ_VERMILION (85), WZ_FROSTNOVA (88), WZ_STORMGUST (89) — each is missing only the official `_hit_fist%d` template. The client sound generator
(`tools/gen_skill_sounds.py`) does not expand `%d` templates and drops wavs that are not exact
GRF paths, so template sounds (`_hit_fist%d`, `ef_icearrow%d`, `ef_firearrow%d`) are lost.

**Data-quality note on the extracted sound table.** `windows_client_skill_sounds.json` and its
extractor merge the symbolic `ef_*` / `*_sound` blocks that follow a numeric effect into that
effect (the block delimiter is `\n\t(\d+): [`, so non-numeric keys are swallowed). Examples:
skill 16 MG_STONECURSE gets the symbolic `stonecurse_sound`, skills 142/378 get Hyper-Novice /
Spirit-Handler wavs, skills 52/53/57/136/138 get commented-out `//wav:` alternates, and skill 16
gets a symbolic-only sound. After decoding the EUC-KR paths, **8 learnable
skills** disagree with the layer-derived wav list. This audit therefore derives official sounds
from the effect layers themselves. The same block-delimiter bug exists in
`extract_windows_client_skill_sprite_effects.py` and `extract_skill_sounds.py`.

### 3.9 Extractor gaps that hide real official layers

7 learnable skills have facts in the raw `SkillEffect.js` that
`tools/windows_client_skill_effects.json` drops: `firstHitEffectId` is not in the extractor's
`FIELDS`, and effect id **0** (the standard `pok3.tga` hit flash) is filtered out by
`numeric_ids`. The client data is generated from the same JSON, so these layers are absent
from the client too:

| Skill | Dropped field | Effect ids |
|---|---|---|
| 5 SM_BASH | `hitEffectId` | [0, 1] |
| 11 MG_NAPALMBEAT | `hitEffectId` | [0, 1] |
| 13 MG_SOULSTRIKE | `hitEffectId` | [0] |
| 13 MG_SOULSTRIKE | `firstHitEffectId` | [1] |
| 21 MG_THUNDERSTORM | `hitEffectId` | [0] |
| 46 AC_DOUBLE | `hitEffectId` | [0] |
| 46 AC_DOUBLE | `firstHitEffectId` | [1] |
| 47 AC_SHOWER | `hitEffectId` | [0, 1] |
| 148 AC_CHARGEARROW | `hitEffectId` | [0] |

The clearest real loss is the `firstHitEffectId` lens flash of MG_SOULSTRIKE (13) and
AC_DOUBLE (46); the effect-0 omission is partly masked because the client's procedural fallback
already renders a generic `pok3.tga` hit flash for those skills.

Fix the extractor (add the `firstHitEffectId*` fields, keep 0 as the standard hit flash) and
regenerate the data files.

### 3.10 Double-spawn / phase notes

- **Heal (28)** is COMPLETE but renders the EF_HEAL cylinders twice: the hand-written
  `target_auras` in `skill_visuals` *and* the generated procedural rows for effect 312/320
  (`lib.rs:2457` only short-circuits skills in `has_custom_completion`; 28 is not in it).
  Its official Hit layers (effect 320) are also covered by Success-phase rows.
- **MC_CARTREVOLUTION (153)**: official Hit `cartrevolution.str` is mapped as a Cast row.
- Custom-skill data rows are dead weight today: 29/33/34/110/270/401 have generated rows
  that `has_custom_completion` skips (`lib.rs:2457,4571`).

## 4. Prioritized fix list

| Priority | Skill | Status | Why it matters | Concrete fix |
|---|---|---|---|---|
| P0 | 34 AL_BLESSING | WRONG_ASSET | Signature Acolyte buff; players expect the flying angel | Use official effect 42: SPR `축복` (head, 1500 ms, yOffset -120) + 2× `particle6` sparkle + `pok2.tga`; remove hand-written arm; add EFST 10 aura |
| P0 | 29 AL_INCAGI | WRONG_ASSET | Signature Acolyte buff; generated ac_center2/agi_up rows already exist | Remove hand-written arm + `has_custom_completion` entry; enable effect 37 layers; add EFST 12 aura |
| P0 | 110 BS_HAMMERFALL | WRONG_ASSET | Visible Blacksmith skill; official asset is in the GRF | Replace `black_hammerfall.str` with `crashearth.str`; drop custom arm so the generated row plays |
| P1 | 33 AL_ANGELUS | COMPLETE (fragile) | Works only through the hand-written arm | Propagate official `head:true`; then the regenerated row keeps the overhead anchor |
| P1 | 59/62/357 (Spear Boomerang, Bowling Bash, LK Concentration) | COMPLETE but feet-anchored | Official `head:true` `.str` buffs shown at the feet | Add official per-layer `head`/`yOffset` to the descriptor; regenerate |
| P1 | 152/230/523/524/525/1004 | MISSING | Official thrown-item billboards are in the GRFs | Add rows for effect 308/298/613/614/615/600 item-icon 3D layers |
| P2 | 17 MG_FIREBALL | PARTIAL | Missing animated fireball projectile | Add SPR/3D `fireball.spr` layer for effect 24 |
| P2 | 81 WZ_SIGHTRASHER | PARTIAL | Missing 8 `sight.spr` 3D layers | Add effect 62 sprite layers |
| P2 | 140 AS_VENOMDUST | PARTIAL | Missing `particle3.spr` swirl | Add effect 171 sprite layer |
| P2 | 118 HT_SHOCKWAVE | PARTIAL | Official `shockwave.spr` absent from GRF | Keep STR `shockwavehit.str`; note in missing-assets.txt |
| P2 | 534 NJ_KOUENKA | MISSING | Plain `firehit.str` exists, only `%d` template mapped | Add `firehit.str` row |
| P2 | buff skills with EFST | n/a | Only Assumptio has a persistent aura | Extend `status_aura_spec`: EFST 10, 12, 9, 3, 2, 15-18 |
| P2 | sound gaps (8 skills) | n/a | Official cast sounds not played | Expand `%d` wav templates in `gen_skill_sounds.py` |

## 5. Method / provenance

Reproduce: `cd notes/skill-audit && python3 audit.py && python3 report.py`
(`audit_core.py` holds the parsers; it reads the repo/tools/GRFs read-only).

- Learnable set: union of every job `Tree` entry with `MaxLevel != 0` in
  `db/pre-re/skill_tree.yml` (77 job trees, 457 unique skills, inheritance included).
- Official layers: `/tmp/EffectTable.js` (identical to
  `CondaRO-linux/notes/korangar-windows-client-data/roBrowserLegacy_EffectTable.js`),
  parsed with all fields (`type`, `file`/`textureName`/`spriteName`, `wav`, `head`,
  `attachedEntity`, `duration`, `delayOffset`/`delayLate`/`delayFrame`, `yOffset`,
  `duplicate`, `func`). Symbolic `ef_*` blocks are kept separate from numeric effect blocks.
- Official skill→effect ids: `tools/windows_client_skill_effects.json` (the pipeline input).
- Current client: `skill_effect_assets.ron` (189 rows), `skill_sprite_effects.ron` (48),
  `skill_procedural_effects.ron` (265), `skill_sounds.ron` (369), `skill_actions.ron` (205),
  plus the hand-written `skill_visuals`/`has_custom_completion` (8 skills) and `status_aura`
  (2 EFST ids) in the Rust sources.
- Asset existence: every referenced `.str`/`.spr`/`.tga`/`.bmp`/`.wav` looked up in the
  three GRF name tables (653,299 files). Template names match any concrete file.
- Action coverage: 457/457 MATCH (the client table and the official facts agree; the
  client plays the default `Skill` gesture where the table lists no action).
- Escaped Korean paths are EUC-KR (`\\xNN`) in the JS dump and are decoded before
  asset checks (e.g. `유저인터페이스\item\돌.bmp`).

### Caveats

- `NO_OFFICIAL_EFFECT` (290) means the official table defines no renderable asset layer: 84
  are passive skills; the rest are hard-coded primitives / sound-only effects in the
  official client (`official.hardcoded_layers` keeps the evidence, e.g. camera quake,
  bolt-rain FUNC effects). It does not mean the skill has no visible official behavior.
- The extracted `windows_client_skill_effects.json` omits effect id 0 and
  `firstHitEffectId`; see section 3.9.
- The official table is a community RE dump; a handful of entries are internally
  inconsistent (e.g. some 4th-job ids). Only facts for pre-renewal skills were used, and
  every asset fact was re-validated against the GRFs.
- `PARTIAL` counts layers, not visual quality: a covered layer can still differ in scale,
  alpha or timing from the official values (e.g. the built-in procedural HitFlash uses
  normalised sizes).
