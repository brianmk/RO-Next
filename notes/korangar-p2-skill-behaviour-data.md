# Korangar client — Phase 4 (P2): data-driven skill behaviour + procedural effect data

Owner: `p2-skill-behaviour` (subagent lane)
Branch: **`p2/skill-behaviour-data`**, worktree
`/home/phantom/Games/ragnarok/CondaRO-linux/korangar-p2-skill-behaviour`
Commit: the single lane commit at the branch tip (this report is part of it); see
`git log -1 --format=%H p2/skill-behaviour-data`
Base/integration HEAD: **`6ed5d6d1`** (`Merge branch 'p1/jobs-appearance-data' (data-driven
jobs/appearance, retire 146-arm match)`) — includes the skill-effect pipeline, the weapon
registry, `jobs.ron` and the Phase 3 status data.
Own build target: `/home/phantom/cargo-target-p2-skill-behaviour` (seeded with hardlinks from the
P1 appearance lane target; **not** the Lead-owned main tree target). The main worktree was not
built or modified.
No Wine, no user characters, no game session; the client binary was only used for the
`--check-*` data CLI checks (no window, no network).
Reference: `notes/client-hardcoded-systems-audit.md` §4 rows 14/15/18, §4.6, §8 Phase 4.

---

## 1. Result

Phase 4 is implemented end-to-end. The remaining **skill-behaviour special cases** in
`korangar/src/lib.rs` and the hard-coded asset/geometry constants of the procedural effects are
now data, generated with a committed input file, a `--check`-ed generator, a
compiled `include_str!` fallback plus archive override, `deny_unknown_fields` validation, a
`--check-skills` CLI, source-grep/data tests and CI parity.

| before (`6ed5d6d1`) | after |
| --- | --- |
| `lib.rs`: `ROLLING_CUTTER_ID = SkillId(2036)` + `skill_id == ROLLING_CUTTER_ID` at the two self-cast paths and the hotbar `StopSkill` path | gone; `world::skills::channels(skill_id)` (data) |
| `lib.rs`: `FREE_CAST_SKILL_ID = SkillId(278)` + `FREE_CAST_JOB_IDS = [16, 4017]` + `has_free_cast()` | gone; skill flag in `skills.ron` + job capability in `jobs.ron` (`can_move_while_casting`) |
| `lib.rs`: `skill_id.0 == 27` provisional warp portal | gone; `world::skills::has_warp_target(skill_id)` |
| `lib.rs`: `STORM_CLOUD_PHASE` + `matches!(skill_id.0, 87 \| 89)` Storm Gust cloud spawn | gone; `world::skills::cloud(skill_id)` row (skills `[87, 89]`, phase `Success`) |
| `world/effect/storm_cloud.rs`: 7 `CLOUD_TEXTURES` + duration/fade/count/height/radius/size/spin/alpha/colour constants | data (`skills.ron` `clouds` row); the module only keeps the deterministic puff-distribution formula |
| `world/effect/cast_circle.rs`: `ring_liner.tga`/`alpha_down.tga` + height/rotation/scale constants | data (`skills.ron` `cast_circle`) |
| `world/effect/warp.rs`: `ring_blue.tga`/`alpha_down.tga`/`particle1.spr` + skill-portal radius/height/ring-count constants | data (`skills.ron` `warp_portal`); map-warp cylinder geometry stays code (documented) |
| `world/effect/aura.rs`: 6 texture constants (rings/glow/bubble/sparkle) | data (`skills.ron` `level_aura`); swirl/bubble geometry stays code (documented) |
| `world/skill_sound.rs`: curated `SKILL_SOUNDS` (6 skills / 12 paths) + `GENERIC_CAST_SOUND` | deleted; `tools/skill_sounds_custom.ron` merged by `gen_skill_sounds.py` into the data file's `curated`/`generic` sections (play priority unchanged) |
| no skill-behaviour data/CLI/tests | `archive/data/client/skills.ron` + `world/skills.rs` + `--check-skills` + `tools/gen_skills.py --check` + data/source tests |

The client data file is

    korangar/archive/data/client/skills.ron     (82 lines: 3 behaviour rows, 1 cloud row, 3 procedural sections)

loaded through `GameFileLoader` (server-overridable inside the client archive) and embedded with
`include_str!` as the compiled fallback, exactly like `skill_effects.ron` / `jobs.ron`.

**No gameplay change:** the data values are the previous constants verbatim, and the call sites
keep the same order/semantics (channeling packet path, provisional portal cell/lifetime, cast
circle spawn condition, `Free Cast` root-release, one cloud per Storm Gust `Success` phase).
Unknown skills/jobs keep their documented conservative fallbacks (no behaviour, no capability).

---

## 2. Data pipeline

### 2.1 `archive/data/client/skills.ron` — skill behaviour + procedural layers

Hand-authored input: `tools/skills_custom.ron` (single source, fully commented with the
provenance of every value). Generator: `tools/gen_skills.py` (523 lines, `--check` drift guard,
`--validate-assets GAME_DIR` optional GRF probe). Schema in `korangar/src/world/skills.rs`
(799 lines incl. tests). `deny_unknown_fields` everywhere; the archive override and the compiled
fallback are the same file.

```ron
(
    provenance: "...",
    version: 1,
    behaviours: [                       // removed skill-id branches from lib.rs
        (skill: 27, warp_target: true),
        (skill: 278, move_while_casting: true),
        (skill: 2036, channeling: true),
    ],
    clouds: [                           // procedural cloud layers (Storm Gust)
        (skills: [87, 89], phase: Success,
         textures: ["effect\\cloud1.tga", ... "effect\\cloud7.tga"],
         count: 18, radius: 22.0, base_height: 10.0, height_variation: 8.0,
         size_min: 16.0, size_variation: 16.0, spin_min: 0.15, spin_variation: 0.35,
         alpha: 0.55, color: (0.45, 0.48, 0.58),
         duration_ms: 3000, fade_in_ms: 250, fade_out_ms: 700),
    ],
    cast_circle: (ring_texture: "effect\\ring_liner.tga", glow_texture: "effect\\alpha_down.tga",
                  radius: 7.0, height: 0.2, ring_rotation_speed: 1.6,
                  inner_ring_rotation_speed: -2.3, inner_ring_scale: 0.62, glow_scale: 1.15),
    warp_portal: (streak_texture: "effect\\ring_blue.tga", gradient_texture: "effect\\alpha_down.tga",
                  particle_sprite: "이팩트\\particle1.spr", skill_radius: 8.0,
                  skill_height: 0.2, skill_ring_count: 2),
    level_aura: (ring_liner_texture: "effect\\ring_liner.tga", ring_blue_texture: "effect\\ring_blue.tga",
                 ring_red_texture: "effect\\ring_red.tga", bubble_texture: "effect\\whitelight.tga",
                 glow_textures: ["effect\\glow01.bmp" .. "effect\\glow04.bmp"],
                 sparkle_texture: "effect\\pikapika2.bmp"),
)
```

Validation (all loud errors, never a silently wrong table):

* behaviour rows: skill ≠ 0, unique, at least one flag enabled;
* clouds: non-empty texture list with `.tga`/`.bmp`, `1..=64` billboards, positive radius/size,
  finite non-negative spreads/height, alpha and colour in `0..=1`, positive duration, fades fit
  the duration, no duplicate skill id across rows;
* procedural sections: non-empty texture paths with the right suffix (`.spr` for the warp
  particle), positive radii/scales, `1..=8` warp rings;
* unknown fields are rejected (`deny_unknown_fields`), so a typo in a server override fails
  loudly and the compiled fallback is used with a warning.

API (`world::skills`): `behaviour`, `channels`, `has_warp_target`,
`grants_move_while_casting`, `cloud`, `cast_circle`, `warp_portal`, `level_aura`,
`CLOUD_DEDUP_PHASE` (the per-cast de-duplication key for the cloud spawn, previously
`STORM_CLOUD_PHASE`), `validate`/`validate_compiled`. The table is a process-wide static sorted
for binary-search lookups; the spawn path allocates nothing per layer/frame.

### 2.2 `jobs.ron` — `move_while_casting` job capability

`gen_jobs.py` now emits a `capabilities` section (curated constant
`MOVE_WHILE_CASTING_JOBS = (16, 4017)` with the rAthena `SA_FREECAST` provenance documented in the
generator) and `jobs.rs` validates that every referenced id is a real job row, sorted/unique:

```ron
capabilities: (
    move_while_casting: [16, 4017],
),
```

`jobs::can_move_while_casting(job_id)` is the job half of the old `FREE_CAST_JOB_IDS` check; the
skill half is the `move_while_casting: true` row of skill 278 in `skills.ron`. Both have to hold,
exactly like the old `job ∈ {16,4017} && learned 278`.

### 2.3 Skill sounds — curated Rust table folded into data

* New input `tools/skill_sounds_custom.ron` holds the six curated overrides **and** the generic
  cast fallback, with the original comments/provenance moved over verbatim.
* `gen_skill_sounds.py` (updated, still the same facts pipeline) merges the curated rows into
  `archive/data/client/skill_sounds.ron` as new top-level sections and keeps one row per skill
  (curated skills are excluded from the official table):

```ron
(
    provenance: "...",
    generic: ["effect\\ef_beginspell.wav", "effect\\em_spell_enchanting.wav"],
    curated: [ (skill: 27, files: [...]), (skill: 110, files: ["effect\\black_hammerfall.wav"]), ... ],
    sounds: [ ... 364 official rows ... ],
)
```

* `world/library/skill_sounds.rs` loads the three sections (compiled fallback + archive override,
  `deny_unknown_fields`, duplicate/overlap validation) and exposes
  `curated_skill_sound_files`/`skill_sound_files`/`generic_cast_sound_files`.
* `world/skill_sound.rs` consults them in the historical priority **curated → per-layer `wav` of
  `skill_effects.ron` → official generated → generic**; `SKILL_SOUNDS`/`GENERIC_CAST_SOUND` are
  deleted.

The file still resolves the same 370 rows (6 curated + 364 official, previously 370 generated
rows of which 6 duplicated the curated table).

---

## 3. Code changes

* `korangar/src/lib.rs` — removed the four skill-id constants and all six special-case sites:
  * both `SkillType::SelfCast` paths now call one `Client::cast_self_skill` helper that asks
    `world::skills::channels` and sends the channeling or ordinary cast packet;
  * hotbar `StopSkill` asks the same data lookup before `stop_channeling_skill`;
  * the provisional warp portal asks `world::skills::has_warp_target` (cell/lifetime/ID
    unchanged);
  * the Storm Gust cloud asks `world::skills::cloud` (skill/phase/texture/timing data, same
    one-per-cast claim via `CLOUD_DEDUP_PHASE`);
  * `has_free_cast` became `can_move_while_casting` = `jobs::can_move_while_casting` **and** a
    learned skill with the `move_while_casting` data flag (log line and rooted default unchanged);
  * `spawn_cast_circle` takes the radius from the data.
* `korangar/src/world/skills.rs` — new loader/schema/validation/API + 7 unit tests.
* `korangar/src/world/effect/{storm_cloud,cast_circle,warp,aura}.rs` — read textures/geometry
  from `world::skills`; no effect asset path literals remain in those modules.
* `korangar/src/world/skill_cast_effect.rs` — removed `CAST_CIRCLE_RADIUS` (data now).
* `korangar/src/main.rs` + `lib.rs` — `--check-skills [PATH]` CLI and `check_skills()`.
* `korangar/src/world/library/mod.rs` — loads `skills.ron` at client startup (same place as
  jobs/status/skill-effect data).
* `korangar/src/world/jobs.rs`, `jobs.ron`, `tools/gen_jobs.py` — capability schema/API/generator.
* `korangar/src/world/{library/skill_sounds.rs, skill_sound.rs}`, `skill_sounds.ron`,
  `tools/gen_skill_sounds.py`, `tools/skill_sounds_custom.ron` — curated sound merge.
* Tests: `korangar/tests/skill_behaviour.rs` (7 source-grep/data/CLI tests),
  `tools/test_skills_data.py` (9 tests), `tools/test_skill_sounds_data.py` (9 tests),
  `tools/test_gen_jobs.py` extended (capability row).
* CI: `.github/workflows/{quality,tests}.yml` (see §6).

### Remaining procedural constants (documented, intentionally code)

Only the *shape* formulas stay in Rust; everything content-like is data:

* `storm_cloud.rs`: golden angle `2.399963` and the four integer-hash puff distributions
  (`index*37%11/10`, `index*53%9/8`, `index*29%7/6`, `index*17%5/5`) — kept verbatim so the
  cloud is pixel-identical; textures/count/radius/height/size/spin/alpha/colour/timing are data.
* `warp.rs`: map-connection cylinder geometry (radius/height/segments/texture repeat), ground
  glow/particle geometry, skill-portal ring fade/radius factor/rotation/pulse colours; the
  textures and the flat skill-portal radius/height/ring count are data.
* `cast_circle.rs`: per-component colour multipliers and two ring angles; textures, height,
  radius, rotation speeds/scales are data.
* `aura.rs`: swirl bands, bubble columns/anchors/alpha formula, ground quads; all aura textures
  are data.
* Already data before this lane (not duplicated): status auras (`status_effects.ron` aura spec,
  Phase 3) and projectile primitives (`skill_effects.ron` `Primitive` layer `file`/size/count/
  timing/tint). `skill_effects.ron` is untouched (436 descriptors / 603 layers).
* `CLOUD_DEDUP_PHASE = u8::MAX - 1` is a de-duplication mechanism, not content.

---

## 4. Behaviour parity evidence

| behaviour | previous hardcode | data row |
| --- | --- | --- |
| Rolling Cutter channeling | `SkillId(2036)`, self-cast ×2 + stop-channel | `(skill: 2036, channeling: true)` |
| Warp Portal provisional portal | `skill_id.0 == 27` | `(skill: 27, warp_target: true)` |
| Free Cast move-while-casting | skill `278` + jobs `[16, 4017]` | `(skill: 278, move_while_casting: true)` + `move_while_casting: [16, 4017]` |
| Storm Gust cloud | `87 \| 89`, `Success`, `u8::MAX-1`, cloud1..7/timing consts | `skills: [87, 89]` row, same values |
| Curated sounds | `SKILL_SOUNDS` 6 skills/12 paths + generic 2 | `curated`/`generic` sections, identical candidate lists/order |

`tools/test_skills_data.py:test_behaviour_flags_are_exactly_the_previous_hardcoded_behaviour`
asserts the rows are exactly these skills/flags (no extra flag on any row) and the cloud/
procedural values equal the old constants; `tools/test_skill_sounds_data.py` asserts the curated
sections equal the deleted Rust table verbatim. The lib unit tests still cover
`can_move_while_casting` for Sage/Professor/non-Sage/unknown job/unknown skill.

---

## 5. Verification evidence

All commands run in this worktree with `CARGO_TARGET_DIR=/home/phantom/cargo-target-p2-skill-behaviour`.

| check | result |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace -- -Dwarnings` | pass |
| `cargo clippy --workspace --all-features -- -Dwarnings` | pass |
| `cargo test -p korangar --lib` | **502 passed**, 0 failed, 12 ignored |
| `cargo test -p korangar --all-features --lib` | **514 passed**, 0 failed, 12 ignored |
| `cargo test -p korangar --test skill_behaviour` | **7 passed** |
| `cargo test -p korangar-messages -p korangar-networking -p ragnarok-packets` | 8 + 66 + 23 + 10 passed, 0 failed |
| `cargo run -p korangar -- --check-skills` | `skills: 3 behaviour rows and 2 cloud rows are valid` |
| `cargo run -p korangar -- --check-skill-effects` | `436 (skill, phase) descriptors are valid` |
| `cargo run -p korangar -- --check-jobs` | `181 job rows are valid` |
| `cargo run -p korangar -- --check-status-effects` | `609 statuses are valid` |
| `--check-skills PATH` (committed file as override) | exit 0 |
| `--check-skills /tmp/bad-skills.ron` (missing `cast_circle`) | exit 1, `schema error: … Unexpected missing field named cast_circle` |
| `--check-skills /nonexistent/skills.ron` | exit 1, `failed to read` |
| `python3 tools/gen_skills.py --check` | `skills.ron is up to date (3 behaviour rows, 1 cloud rows)` |
| `python3 tools/gen_skills.py --check --validate-assets <client/game>` | exit 0 — every cloud/cast-circle/warp/level-aura texture and the `particle1.spr` sprite exists in the CondaRO GRFs |
| `python3 tools/gen_jobs.py --check` + `tools/test_gen_jobs.py` | up to date / 21 tests OK |
| `python3 tools/gen_skill_sounds.py --check` | `up to date (6 curated + 364 official skills)` (needs the GRFs, run manually) |
| `python3 tools/test_skills_data.py` / `tools/test_skill_sounds_data.py` | 9 + 9 tests OK (offline) |
| source grep (also enforced by the integration test) | `ROLLING_CUTTER_ID`, `FREE_CAST_SKILL_ID`, `FREE_CAST_JOB_IDS`, `STORM_CLOUD_PHASE`, `skill_id.0 == 27`, `matches!(skill_id.0, 87 \| 89)`, `SKILL_SOUNDS`, `GENERIC_CAST_SOUND` → **0** occurrences |

**Known pre-existing red gate (not caused by this lane):** `cargo test -p korangar` still fails
`tests/ui_consistency.rs::hardcoded_style_literals_do_not_exceed_the_token_budget`
(29 colour literals > budget 28, 16 `FontSize` > 12). Reproduced on the unmodified integration
tree: `bash scripts/ui-token-budget.sh` → exit 1 at `6ed5d6d1`. This lane changes **no**
`korangar/src/interface` file (the Phase 0 owner has to restore that ratchet). All other
`korangar` test targets are green.

**No visual/game-session check** (no Wine, no user characters): behaviour parity is enforced by
the exact-value data tests above and by keeping the call sites/queue order unchanged; the GRF
asset validation proves every newly data-driven texture exists in the shipped archives.

---

## 6. CI parity

* `.github/workflows/quality.yml`
  * `Generated content data is up to date`: adds `tools/gen_skills.py --check`,
    `tools/test_skills_data.py`, `tools/test_skill_sounds_data.py` (all offline).
  * `tests` job (default features): adds `cargo run -p korangar -- --check-skills`.
  * `generated-data` job: offline skill-data drift checks + `py_compile` of the new
    generators/tests.
* `.github/workflows/tests.yml`: adds `./tools/gen_skills.py --check`, `./tools/test_skills_data.py`
  and `./tools/test_skill_sounds_data.py` next to the status-effect generator check.
* `tools/gen_skill_sounds.py --check` still needs the CondaRO GRFs (it resolves official wav
  names against them), so it stays a manual/verifier run like before; the offline
  `test_skill_sounds_data.py` guards the curated merge (the part that moved from Rust to data).

---

## 7. Files

New:

* `korangar/archive/data/client/skills.ron` — runtime data (generated).
* `korangar/src/world/skills.rs` — loader/schema/validation/API + unit tests.
* `korangar/tests/skill_behaviour.rs` — source-grep/data/CLI regression tests.
* `tools/skills_custom.ron` — input data.
* `tools/gen_skills.py` — generator + `--check` + `--validate-assets`.
* `tools/skill_sounds_custom.ron` — curated sound input (moved from Rust).
* `tools/test_skills_data.py`, `tools/test_skill_sounds_data.py` — offline data tests.

Modified:

* `korangar/src/lib.rs`, `korangar/src/main.rs`, `korangar/src/world/mod.rs`,
  `korangar/src/world/library/mod.rs`, `korangar/src/world/jobs.rs`,
  `korangar/src/world/skill_sound.rs`, `korangar/src/world/skill_cast_effect.rs`,
  `korangar/src/world/effect/{storm_cloud,cast_circle,warp,aura}.rs`,
  `korangar/src/world/library/skill_sounds.rs`,
* `korangar/archive/data/client/jobs.ron`, `korangar/archive/data/client/skill_sounds.ron`,
* `tools/gen_jobs.py`, `tools/test_gen_jobs.py`, `tools/gen_skill_sounds.py`,
* `.github/workflows/quality.yml`, `.github/workflows/tests.yml`.

`korangar/archive/data/client/skill_effects.ron` is untouched (visual pipeline unchanged).
