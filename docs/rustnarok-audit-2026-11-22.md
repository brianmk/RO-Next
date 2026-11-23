# rustnarok-client vs korangar/RO-Next audit and port report

**Date:** 2026-11-22
**Rustnarok source:** <https://github.com/kindraywind/rustnarok-client>
at commit `76600595363affc96d5995bf4baa790d05a9593c` (shallow clone of `main`).
**Our client:** `/home/phantom/Games/ragnarok/CondaRO-linux/korangar` (RO-Next),
integration branch.
**Machine-readable companion:** [`docs/rustnarok-inventory.json`](rustnarok-inventory.json)
(1,621 rustnarok effect ids, 429 motion-table skills, 313 status icons, each with its
our-side equivalent and covered/missing status).
**Mode:** source analysis only; no Wine, no user characters, no live server changes.
**Program scope:** the RO-Next client with the 2019 rAthena Pre-Renewal/Transcendence
content set. Rustnarok’s 4th-job/2025 additions are recorded in the inventory but are
out of scope for copying unless a primitive is also useful to our content.

---

## 1. Executive summary

Rustnarok is an *effect-first* client: its effect knowledge lives in a large
hand-maintained text table (`src/fx/effects.txt`, 5,572 lines) and a 13k-line
Rust implementation of procedural effect primitives (`src/fx/effects.rs`,
`src/fx/prims/*`). Our client is already *data-first* for skill visuals:
`archive/data/client/skill_effects.ron` has **437 phases / 608 layers**, and the
client has no runtime per-skill visual table.

Therefore a full “copy all effects” is not a file copy; the useful port is:
1. its **per-skill motion tables** (`src/skill_motions.rs`) → our
   `skill_actions.ron`/`CasterAction` (done for the representable gap in this batch);
2. its **status presentation/colour knowledge** (`src/status_icons.txt`,
   `effects.txt` `status`, body-tint code in `prims/*`) → our status data/tints/auras
   (Berserk + Adrenaline/Fury tints in this batch);
3. a documented backlog for the procedural primitives we do not have (below).

Counts at the audit commit:

| metric | rustnarok | RO-Next |
| --- | --- | --- |
| effect ids with a definition | 1,621 | 388 skill ids with effect rows (437 phases, 608 layers) |
| effect definition lines | 4,553 | — |
| skills with an effect mapping | 1,158 | 388 |
| status visual mappings (`status` lines) | 177 | 7 persistent auras + 3 status tints |
| status icons | 313 built-in | 609 status rows, 575 icons |
| skill-motion table entries | 429 skills | 242 action rows (after this batch) |
| protocol | 2025-era packets + MCP | 2019/2022-compatible `PACKETVER` |

**Coverage result:** of the **457 learnable Pre-Renewal skills** in our server dump,
**246 now have an effect row, 223 do not**; rustnarok supplies an effect mapping for
**211 of those 223**, but most of those are only its generic caster burst (`effect 16`)
or begin-spell (`12`) primitives and have no distinct asset of their own. The
distinct-asset gap is much smaller: rustnarok’s own effect table has no separate
`str/part/ring/spr` definition for those generic-cast skills either.

---

## 2. Architecture comparison

### 2.1 Effect system

**rustnarok**
- `src/fx/effects.txt` is the central table. Grammar in the file header (lines 1–30):
  `str`, `part`, `ring`, `spr`, `func`, `length`, `wav`, `hit`, `caster`, `cast`,
  `struck`, `used`, `ground`, `status`, `unit*`, `area`, `trap`, `rsm`, `minid`.
- `src/fx/effects.rs` (13k lines) parses the table (`Fx`/`Table`, status map at
  lines ~131–132, loading at ~377–379) and implements every non-data primitive in
  Rust, split over `src/fx/prims/*.rs` (aura, burst, rings, walls, vortex, songs,
  souls, …).
- Effects are selected by effect id; skills are wired to effect ids by the table.

**RO-Next**
- `archive/data/client/skill_effects.ron` (437 phases / 608 layers) is the central
  table: per skill, per phase (`Cast`, `Success`, `Hit`, `Release`, `Caster`),
  each layer carries kind (`Str`, `Spr`, `Texture`, `Cylinder`, `Primitive`,
  `BodyFlash`), anchor, timing, tint, blend mode and optional wav.
- `korangar/src/world/library/skill_effects.rs` loads it with `include_str!`
  fallback + archive override + `--check-skill-effects`; the runtime is
  `workspace/world/effect/*` and `skill_cast_effect.rs`.
- `archive/data/client/skill_actions.ron` carries the caster motion per skill;
  `skill_sounds.ron` carries the sound rows; `skills.ron` carries the three
  behaviour flags (Rolling Cutter, Warp Portal, Free Cast).
- Procedural effects we already have: warp portal (`world/effect/warp.rs`),
  spirit spheres (`world/effect/spirit_spheres.rs`), status auras
  (`world/effect/status_aura.rs`), cast circle, storm cloud, skill aura.

**Comparison:** our data schema is better for content patches; rustnarok’s
primitive library is broader (hundreds of effect families). The correct
integration is to port *missing primitives* behind our existing `Primitive`
layer kinds, not to replace the pipeline.

### 2.2 Status handling

**rustnarok:** `src/status_icons.txt` (313 EFST → icon) plus `status` lines in
`effects.txt` (177 EFST → looping effect ids), plus per-status code in
`fx/prims/*` for body tints/glows (e.g. `prims/walls.rs::body_tint` Berserk
`#FA7878`, `prims/transcend.rs`, `prims/songs.rs`), plus `fx/world_fx/status4.rs`
for 4th-job status objects.

**RO-Next:** `archive/data/client/status_effects.ron` has 609 statuses,
519 names, 575 icons, 7 value-label schemas, refresh/stack policies and
**7 auras** (Assumptio 110/473, Fury 86, Berserk 132, Energy Coat 31, Steel Body 87, Endure 1). Status packets are
tracked per entity (`state/status_effects.rs`); body tints are applied from the
status handler (`korangar/src/lib.rs`, status arm) and rendered through
`EntityCommon::sprite_tint`.

**Gap:** rustnarok knows 177 status *visual effects*; we have 4 aura specs + three
body tints. Most of the remaining 173 are short-lived procedural status glows that
need new primitives (song puffs, Soul Drain tint, etc.). This is the largest
remaining visual gap and is backlog item B1.

### 2.3 Skill animation/motion

**rustnarok:** `src/skill_motions.rs` (632 lines) has:
- `ATTACK` (105 ids), `THROW` (18), `THIRD_ATTACK` (9), `PICK_UP` (23),
  `KEEP` (73), `STATES` (149 `(id, generic action)`), `CAST` (292 `(id, i8)`).
- `CasterAction`-equivalent is richer: generic action indices (dance, song, kick,
  ninja, gunslinger, Madogear, Doram, …).

**RO-Next:** `CasterAction` = `Attack | Idle | Pickup | Skill | ReadyFight`;
`skill_actions.ron` had 205 rows. After this batch it has **242 rows**.

**Port decision:** map only where rustnarok’s motion maps cleanly onto our five
actions: `0/-1/Keep → Idle`, `7 → Skill`, `2/12/Attack/Throw/ThirdAttack →
Attack`, `5/PickUp → Pickup`. Special generic action indices (`STATES`, most of
`CAST`) are recorded in the inventory but not ported because our actor animation
API would need a generic action-index variant (backlog B2).

### 2.4 Packet handling / runtime

rustnarok targets a 2025 packet table with a large MCP/autohunt surface and its
own renderer stack (`src/net/*`, `src/ui/*`). RO-Next targets the 2019 rAthena
server with `PACKETVER` 2022-compatible packets and custom wgpu/rust_state
renderers. Packet behavior was **not** copied: the two protocol surfaces only
overlap for the classic `ZC_NOTIFY_SKILL`/`ZC_USE_SKILL`/`ZC_SKILL_ENTRY`
handling we already have and recently hardened (Leap `0x01FF`, Snap `0x08D2`,
skill-unit handling).

---

## 3. Coverage matrix

| area | rustnarok entries | RO-Next equivalent | covered | missing / notes |
| --- | ---: | --- | ---: | --- |
| effect ids with definitions | 1,621 | `skill_effects.ron` rows + primitives | 283 (heuristic: skill/status uses it) | 1,338, overwhelmingly 4th-job/2025-only; 211 learnable pre-re skills have no distinct effect file |
| learnable Pre-Re skills | 457 | 388 skill rows in `skill_effects.ron` | 246 | 211 (most only generic cast effects 12/16) |
| skill motion table | 429 skills | `skill_actions.ron` 242 | 183 | 246 (mostly 4th-job/special action indices) |
| status icons | 313 | 609 status rows / 575 icons | 313 built-ins present | 0 rustnarok icons missing; 45 icon-name differences (case/source table), not functional |
| status visual effects | 177 | 7 auras + 3 tint families | 10 | 167 (need primitives; see B1) |
| skill sounds | rustnarok `wav` rows | 8 curated + 363 official + fallback | broad | generic-cast sounds handled by our gate |

The exact per-entry table is in `docs/rustnarok-inventory.json`:
`effects[*].ours`, `skill_motions[*].ours`, `statuses[*].ours_status/ours_icon`.

---

## 4. What was ported in this batch

Commits on the integration branch:
- **`478b7ea8`** — `feat(ui): classic status card ...` also contains the batch
  data: 37 new motion rows (`tools/skill_actions_extra.json`,
  `tools/gen_skill_actions.py`, `korangar/archive/data/client/skill_actions.ron`)
  and the Berserk status customization (`tools/status_effect_custom.ron`).
- **`6cbd1e77`** — status-driven body tints for Adrenaline Rush 23/147
  `(1.0,0.45,0.45)`, Fury 86 `(1.0,0.75,0.75)`; removed the wrong option-bit
  tint (`0x8` is `OPTION_CART`). Berserk 132 `#FA7878` added in this audit batch.
- The regenerated `status_effects.ron` / `status_facts.json` (7 auras: Assumptio
  110/473, Fury 86, Berserk 132, Energy Coat 31, Steel Body 87, Endure 1) are the
  data half of the same batch.
- Existing high-value ports retained intact: white Warp Portal pillar, 5 orbiting
  spirit spheres, Asura Strike impact/body flash, Fury animation/electric
  aura/waves, Leap `0x01FF`, Body Relocation `0x08D2`, cart (base + status
  fallback), Maximize Power sound, Ruwach sound, Dangerous Soul Collect cast
  start sound.

**Verified:** `cargo test -p korangar --lib` 545 passed / 0 failed / 12 ignored;
`cargo fmt --all` clean; `tools/gen_skill_actions.py --check` 242 entries;
`tools/gen_status_effects.py --check` 609 statuses / 7 auras;
`korangar --check-status-effects` data path covered by the same schema tests.

### Rejected / deliberately not copied
1. **4th-job and 2025-only effect families** (Climax, A-Machine, Shadow Exceed,
   Servant Weapon, frostbite, god marks, …): outside the 2019 Pre-Renewal server
   content; recorded in the inventory, not ported.
2. **Rustnarok’s procedural primitive implementations as-is**: they assume its
   renderer (`EffectMesh`/`EffectQuad`, its GRF loader and actor model); copying
   them would fork our renderer. Only the data/colour knowledge is portable now.
3. **Generic cast auras 12/16** as per-skill rows: they are rustnarok primitive
   defaults, not distinct assets; adding 200 duplicate rows would inflate data
   without improving the visual.
4. **2025 packet/MCP/autohunt surfaces**: no protocol compatibility with our 2019
   map server.
5. **Special generic actor action indices** (dance/song/kick/ninja/etc.): our
   `CasterAction` has no action-index variant; needs a renderer/actor change first.

---

## 5. Prioritized backlog

**B1 (high) — status visual primitives.** Port rustnarok’s 177 `status` mappings
into a new `status_visual_effects.ron` and implement the three reused primitive
families: body tint/fade (`prims/walls.rs`, `transcend.rs`, `songs.rs`), looping
STR/part status glow, and ground ring. Start with the classic 2019 set:
Concentration 105, Berserk 132 (aura done), Steel Body 87, Endure 8,
Energy Coat 83, Venom Dust 124, Safety Wall 13.

**B2 (high) — generic actor motion index.** Extend `CasterAction` with
`Action(u8)` or a `Motion` enum, then port rustnarok’s `STATES`/remaining `CAST`
rows (246 motion entries still missing). This unlocks dance/song/kick/shot
animations currently flattened to `Skill`.

**B3 (medium) — skill effect families with distinct assets.** Use the inventory
to select the subset of the 211 learnable missing skills where rustnarok has a
`str/part/ring/spr` definition (not just 12/16); port those rows into
`tools/skill_effects_custom.ron` with archive-verified files.

**B4 (medium) — status icon cleanup.** Resolve the 45 rustnarok-vs-ours icon-name
differences (case, missing `.tga`, Korean vs localized names) only where the
client’s actual archive file check fails; no functional gap today.

**B5 (low) — primitive library.** Port reusable primitives (vortex, wall,
waterball, souls) behind our `Primitive` layer kinds, one family per commit with
a golden test, only when a B3 row needs them.

**B6 (low) — inventory drift guard.** Add
`tools/check_rustnarok_inventory.py --check` so this JSON can be regenerated and
CI-checked when the rustnarok pin moves.
