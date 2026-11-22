# P1 — Data-driven job/appearance registry (`jobs.ron`)

**Lane:** `p1-appearance-data` · **Branch:** `p1/jobs-appearance-data` · **Base:** integration HEAD
`c954572d` (task start pin `bf53400b`+; `git log -1` at branch time was `c954572d docs: update
hardcoded-systems audit after weapon centralization`) · **Date:** 2026-11-22

**Scope:** audit §4 rows 1/2/3/8 and §8 Phase 1 items 4–6 — make the player appearance layer
(job → sprite/palette/hair/entity-type/flags/scale) data-driven, retire the 146-arm match and the
related hardcodes, and turn the ignored job audit into a CI-checkable appearance audit.

**Mode:** no Wine, no game session, no user characters. All runtime verification is against the
read-only CondaRO client archives and the rAthena server tree; client tests run on Linux/Xvfb-free
unit tests only.

---

## 1. Summary

| before (`c954572d`) | after |
| --- | --- |
| `entity/mod.rs`: 146-arm `get_sprite_path_for_player_job` match (144 unique ids) | **deleted**; `jobs.ron` row lookup (binary search) |
| 4 palette-stem overrides (7 job ids) in `get_body_palette_name` | **deleted**; `palette` field per job in `jobs.ron` |
| `MALE_HAIR_LOOKUP` / `FEMALE_HAIR_LOOKUP` + `resolve_head_id` | **deleted**; `appearance.hair_styles` + `resolve_head_id` in `world/jobs.rs` |
| body/head/palette/headgear path templates in Rust | **deleted**; `paths` templates in `jobs.ron` |
| `EntityType::from` job-id ranges in `entity/mod.rs` | **moved to data** (`entity_types` section) + documented fallback default |
| magic transcendent range `4001..=4023` | **deleted**; `transcendent` flag per job (rAthena classification) |
| `BABY_JOB_SCALE = 0.75` + client `IsBabyJob` lua table | **deleted**; `scale` field per job (0.75 baby / 1.0) |
| `job_alias.rs`: 63-entry sprite-alias table | **deleted**; folded into `jobs.ron` as documented `Fallback`/`Unmapped` rows |
| `job_audit.rs` ignored test | **replaced** by `korangar --check-appearance` + an ignored archive test that calls it |
| 26 of 144 client sprite mappings fell back because the sprite was "missing" | **1 fallback**: the archives do ship the 4th/expanded class sprites (`dragon_knight_남.spr`, …); the old probe compared the client's upper-case stems case-sensitively. 25 jobs now render their real class sprite |
| 0 committed job facts / generator / CI check | `tools/job_facts.json` (185 KB), `tools/extract_job_facts.py`, `tools/gen_jobs.py --check`, `tools/test_gen_jobs.py` (20 tests), CI step |

**Data file:** `korangar/archive/data/client/jobs.ron` (generated, 2615 lines): 181 real rAthena
`e_job` ids (the two sentinels `JOB_MAX_BASIC`/`JOB_MAX` omitted) — 143 mapped, 1 documented
fallback (`4014` → `로드나이트`, client stem `엔대운` missing), 37 jobs the client does not map at
all; 54 baby, 39 transcendent, 25 4th-class, 37 expanded-class rows; 16 jobs with the
sprite-embedded palette (`palette: None`); appearance defaults (13 hair styles per sex, head ids
1–42, default head id 1, default palette colors); entity type classification; and the 161 headgear
view ids whose accessory sprite is not part of this data set.

## 2. Data pipeline (facts → generator → runtime)

```
rAthena server tree (ssh)                        CondaRO client archives (GRF)
  src/common/mmo.hpp  enum e_job                   data.grf / CondaRO.grf / palettes.grf
  db/pre-re/skill_tree.yml  job trees              body/head sprite probe (spr+act)
  db/re/skill_tree.yml      all-job trees          body/hair palette probe
  src/map/pc.hpp            provenance             jobidentity.lub / npcidentity.lub
        │                                          accname.lub / accessoryid.lub (bytecode)
        └──────────────► tools/extract_job_facts.py ──────────────┘
                                   │
                       tools/job_facts.json  (committed, 185 KB)
                                   │
                       tools/gen_jobs.py  ──►  korangar/archive/data/client/jobs.ron
                                   │
                       world/jobs.rs (include_str! fallback + archive override)
```

### 2.1 `tools/extract_job_facts.py` (new, committed)

* **Server facts:** `enum e_job` (the authoritative id assignment; this rAthena revision no longer
  has `db/pre-re/job_db.yml` — it was split into `job_stats/job_exp/job_aspd/job_basepoints`, so
  the enum is the job list), `db/pre-re/skill_tree.yml` + `db/re/skill_tree.yml` for the job
  trees, `src/map/pc.hpp` for provenance. Server HEAD + per-file sha256 are recorded.
* **Client facts:** probes the three CondaRO GRFs for existing body sprites (`.spr`+`.act`), body
  palettes, head/hair sprites, hair palettes and headgear accessory sprites; extracts the job
  identity tables (`jobidentity.lub`, `npcidentity.lub` — bytecode or plain source), the special
  entity job ids (`JT_WARPNPC 45`, `JT_HIDDEN_NPC 111`, `JT_HIDDEN_WARP_NPC 139`) and the headgear
  view-id/name/accessory-sprite coverage from `accessoryid.lub` + `accname.lub`.
* **Lua parsing:** the client `.lub` files are 32-bit Lua 5.1 bytecode. The extractor parses the
  top-level chunk constants for the flat `JT_*` tables and emulates the table-constructor
  instructions (`NEWTABLE`/`GETGLOBAL`/`GETTABLE`/`LOADK`/`SETTABLE`) for the accessory tables, so
  it reproduces the exact 1526-row view-id/name table the client loads — no Lua runtime needed.
* **Case-insensitive asset probe:** the archives store lower-case names and the client lowercases
  every asset lookup (`GameFileLoader`), so the probe compares lower-cased stems. This is how the
  existing 4th/expanded class body sprites (`kagerou`, `dragon_knight`, `sky_emperor`, …) were
  found; the previous case-sensitive probe had declared them missing and aliased 25 jobs to their
  3rd-class sprite.
* **Curated engine table:** `job id -> classic client player sprite stem` (144 entries) and the
  hard-coded fallbacks are client-engine data (the kRO client executable hard-codes this; it is not
  in any data file). The extractor records the cross-check against `CondaRO.exe` (120 of 144 stems
  appear there as CP949 strings; the rest are classes newer than this executable) and refuses a
  mapping that has neither a probed sprite nor an archive-verified fallback. Palette overrides are
  reduced to a verified sprite-stem → palette-stem map. The facts JSON records all of this, so
  `gen_jobs.py --check` runs offline in CI from committed data only.

Provenance pinned in `tools/job_facts.json`:

| source | value |
| --- | --- |
| rAthena server HEAD | `89c8bfa61a48310d7ebbd18992ad30b7c2631a68` |
| `src/common/mmo.hpp` | sha256 `cd490eed…9b9a80b` |
| `db/pre-re/skill_tree.yml` | sha256 `fafe8039…7c063c7f54` |
| `db/re/skill_tree.yml` | sha256 `b54fc9cc…` (209,371 bytes) |
| `src/map/pc.hpp` | sha256 `7641408d…59bce126` |
| `data.grf` | 3,167,729,595 B, 149,167 files, sha256 `6dcae744…de2d53a` |
| `CondaRO.grf` | 145,362,482 B, 34,565 files, sha256 `813c7213…bea3d0da` |
| `palettes.grf` | 204,600,439 B, 498,263 files, sha256 `a18d0033…b1b76170` |
| client executable cross-check | `CondaRO.exe` sha256 `0dd76a90…bea488fe` |

### 2.2 `tools/gen_jobs.py` (new, committed)

Builds and validates the data file, refuses drift:

* validates the facts themselves (every mapped/fallback/palette/hair/headgear entry must have been
  seen by the archive probe, count splits must add up, rows strictly ascending, scale matches the
  baby flag);
* derives `Mapped` / `Fallback` / `Unmapped` per job from the probe (case-insensitive);
* renders deterministic RON (jobs sorted, sorted/unique headgear-missing list, escaped paths);
* `--check` fails when the committed file diverges; also used from `tools/test_gen_jobs.py`.

### 2.3 `korangar/archive/data/client/jobs.ron` (generated, committed)

```ron
(
    provenance: "...",
    version: 1,
    paths: (body, head, body_palette, head_palette, headgear),   // classic file templates
    sex_suffix: (male: "남", female: "여"),
    appearance: (
        hair_styles: (male: [2,2,1,7,5,4,3,6,8,9,10,12,11], female: [...]),  // network style -> head sprite id
        hair_style_range: (start: 0, end: 12),
        head_range: (start: 1, end: 42),
        default_head: 1,
        default_body_palette: 0,
        default_head_palette: 0,
    ),
    entity_types: (
        warp: [45], hidden: [111, 139],
        player_ranges: [...], npc_ranges: [...], monster_ranges: [...],
        default: Npc,
    ),
    headgear: (missing: [323, 342, ...]),   // documented data gap (161 ids)
    jobs: [
        (id, name, entity_type, sprite, client_sprite, palette, source,
         baby, transcendent, fourth, expanded, scale),
        ...
    ],
)
```

Every row documents *why* it uses its sprite: `source: Mapped` (client stem used, probe-verified),
`Fallback` (client stem missing from the archive, verified fallback used — only `4014`), `Unmapped`
(client does not map the job, verified fallback used — 37 rows). `client_sprite` keeps the client's
intended stem for provenance/audit. `palette: None` means the sprite-embedded palette is used.

## 3. Runtime rewire

### 3.1 New module `korangar/src/world/jobs.rs`

One registry/API for the subsystem (same pattern as `world/weapon.rs`):

* data: `DATA_PATH = data\client\jobs.ron`, `FALLBACK = include_str!(…)`, `OnceLock` archive copy,
  `deny_unknown_fields` schema structs, binary search by job id;
* schema validation for every pattern the code expands (`{sprite}`, `{sex}`, `{head}`, `{hair}`,
  `{palette}`, `{color}`, `{name}`), sorted/unique job ids, scale/baby consistency, sorted/unique
  headgear-missing ids, non-empty sprite/palette/client-sprite, no `missing` placeholder;
* API: `player_sprite_stem`, `body_part_path`, `head_part_path`, `body_palette_path`,
  `head_palette_path`, `headgear_part_path`, `resolve_head_id`, `entity_type`, `scale`,
  `is_transcendent`, `validate`/`validate_compiled`, `load`, `check_appearance`;
* `JobProfile` exposes the data for the audit; `profile`/`headgear_missing` are test-only helpers;
* malformed archive override logs and falls back to the compiled copy (never a black sprite);
  unknown job ids fall back to the novice sprite and log once per id (`job_alias::warn_once`), so a
  custom-server id is visible but never renders `missing.spr`.

### 3.2 Deleted hardcodes / call sites

* `korangar/src/world/entity/mod.rs`: the 146-arm match, `get_body_palette_name`, the two palette
  path helpers, both hair lookup arrays, `resolve_head_id`, the job-id ranges in
  `impl From<JobId> for EntityType`, the `4001..=4023` transcendent range and `BABY_JOB_SCALE` are
  gone. `get_entity_part_files` now composes parts from the registry templates/stems and
  `EntityType::from`, `is_transcendent_job`, `scale`/`set_job` delegate to `world::jobs`.
* `korangar/src/world/library/mod.rs`: `accessory_sprite_path` uses the data `paths.headgear`
  template (the names stay the data-driven `accname.lub` table); added
  `accessory_view_ids()` for the audit; loads `world::jobs` next to `world::weapon`; the
  `baby_job` module/table (client `baby-job-lookup.lua`) was removed.
* `korangar/src/world/library/job_alias.rs`: the 63-entry `PLAYER_JOB_SPRITE_ALIASES` table and its
  lookup were deleted; the generated skill-tree fallback table and `warn_once` remain (separate
  concern, see §7).
* `korangar/src/world/weapon.rs`: the weapon registry now takes the job stem from
  `world::jobs::player_sprite_stem` (no entity-layer sprite table). Weapon path templates remain in
  `weapons.ron` (single source), so nothing is duplicated in the entity layer.
* `korangar/src/world/library/job_audit.rs` and `baby_job.rs` deleted; the old ignored test is
  superseded by the archive test in `world/jobs.rs` (calls the same `check_appearance`).

### 3.3 CLI + CI

* `korangar --check-jobs [PATH]` — validates the committed/override job data without archives,
  prints `jobs: 181 job rows are valid`, exit 1 on schema errors.
* `korangar --check-appearance` — the replacement for the ignored job audit and the Phase 1
  acceptance item: loads `client/game_archives.ron` and validates every job body sprite, body
  palette (or embedded-palette row), hair sprite/palette and head range entry, every headgear view
  id (sprite exists or is documented missing, and documented entries are still missing), every job
  has a non-empty skill tree, and logs (informational) the job ids without a `JobIdentity` entry.
  Exit 1 with a problem list on regression.
* `.github/workflows/quality.yml`: the “Generated content data is up to date” step now also runs
  `python3 tools/gen_jobs.py --check` and `python3 tools/test_gen_jobs.py` — facts/generator/data
  drift is caught offline, no game files needed.

## 4. Verification evidence

All commands run in the lane worktree
(`/home/phantom/korangar-p1-appearance`, `CARGO_TARGET_DIR=/home/phantom/cargo-target-p1-appearance`).

```
$ python3 tools/extract_job_facts.py --ssh rathena --server-root /home/phantom/rathena
tools/job_facts.json: 181 jobs (143 mapped / 1 fallback / 37 unmapped), 16 embedded palettes,
headgear 1365/1526 with sprites (161 documented missing)
server head 89c8bfa61a48310d7ebbd18992ad30b7c2631a68
# reproducible: re-running the extractor produces a byte-identical tools/job_facts.json

$ python3 tools/gen_jobs.py --check
korangar/archive/data/client/jobs.ron is up to date (181 job rows)

$ python3 tools/test_gen_jobs.py
Ran 20 tests ... OK

$ cargo test -p korangar --lib
test result: ok. 466 passed; 0 failed; 11 ignored

$ cargo test -p korangar --lib jobs::tests::every_server_job -- --ignored   # archive audit
test world::jobs::tests::every_server_job_has_a_sprite_palette_and_skill_tree ... ok

$ /home/phantom/cargo-target-p1-appearance/debug/korangar --check-jobs
jobs: 181 job rows are valid
$ …/korangar --check-jobs /tmp/bad_jobs.ron
jobs: invalid data: /tmp/bad_jobs.ron: schema error: 1:1-1:8: Expected struct `JobsFile` but found `garbage`   (exit 1)
$ …/korangar --check-appearance
appearance: 181 jobs (165 with both sexes), 16 embedded palettes, 26 hair style entries,
1365 headgear view ids (161 documented missing)
appearance: 27 job(s) without a client JobIdentity entry (NPC rendering only, informational): […]
(exit 0)

$ cargo fmt --all --check                       # clean
$ cargo clippy -p korangar --all-features -- -Dwarnings   # clean (exit 0)
```

Archive audit numbers match the committed facts exactly: 181 jobs, 165 body sprites present for both
sexes (the other 16 are gender-locked classes: bard/dancer mirror classes, 4th Troubadour/Trouvere,
Shinkiro/Shiranui), 16 embedded palettes, 13 hair styles per sex, 1365 of 1526 headgear view ids with
a sprite, 161 documented missing. The 27 jobs without a client `JobIdentity` entry are exactly the
27 ids the previous audit documented; identity is only used for NPC rendering.

## 5. The 4th/expanded sprite correction (behaviour change, verified)

The old alias table was generated with a case-sensitive probe against the lower-cased GRF file
list; the classic client table spells these stems `DRAGON_KNIGHT`, `KAGEROU`, `REBELLION`, … so they
looked missing and 26 jobs were aliased to their 3rd job. The archives actually ship
`dragon_knight_남.spr`/`.act`, `kagerou_남`, `sky_emperor_남`, etc., and korangar lower-cases every
lookup (`GameFileLoader::get`/`file_exists`, `SpriteLoader`), so those sprites render on this data
set. After the migration:

* `26 → 1` documented fallbacks (only `4014 JOB_LORD_KNIGHT2`, client stem `엔대운`, is genuinely
  missing);
* `4252 DRAGON_KNIGHT`, `4256 CARDINAL`, `4263 TROUBADOUR`, `4302 SKY_EMPEROR`,
  `4307 HYPER_NOVICE`, … now use their real class sprites (with their own palettes where the data
  set ships one);
* gender-locked 4th classes keep their single-sex sprite (audit checks both sexes only where they
  exist, exactly like the old audit did).

## 6. Data-set gaps now made explicit (not hidden)

* **Headgear:** 161 of the 1526 accessory view ids of this client data set have no accessory sprite
  (custom CondaRO items in the name table, trimmed sprite archives). They are listed in
  `jobs.ron` (`headgear.missing`) and counted/reported by `--check-appearance`; the client already
  treats accessory layers as optional and skips them, so this is a documented data gap rather than
  a silent black sprite. The audit fails if a documented-missing id gains a sprite (stale data) or
  an undocumented id loses one.
* **Palettes:** 16 jobs (Crusader, Clown, Ranger line, costume jobs) have no body palette file in
  this data set; `palette: None` documents the sprite-embedded-palette fallback instead of a broken
  `.pal` path. The four classic palette-stem overrides (High Priest, Assassin Cross, Royal Guard,
  Rebellion) are preserved as the verified sprite-stem → palette-stem map, and alias rows now inherit
  the palette of the sprite they actually use (e.g. `4258 → 가드 → 로얄가드`).
* **Skill trees:** for 56 jobs the client tables have no/empty tree; the generated
  `JOB_SKILL_TREE_FALLBACKS` table (`job_alias.rs`) still handles that. It is client-table content
  (not rAthena content) and is a separate follow-up; the audit keeps asserting every job resolves to
  a non-empty tree.

## 7. Follow-ups (out of this lane)

1. Move the 56-entry client skill-tree fallback table into `jobs.ron` (needs the client
   `skilltreeview.lub`/`jobinheritlist.lub` bytecode parsed with the same instruction emulation the
   extractor now has for the accessory tables) — then `job_alias.rs` only keeps `warn_once`.
2. Shield/robe layers are still plumbed but not rendered (§4.2 residual); their view-id profiles
   belong next to `weapons.ron`/`jobs.ron`.
3. Weapon attack sound (`entity/mod.rs` TODO) still not data-driven.
4. `--check-appearance` is archive-backed and therefore not part of the default CI job (CI has no
   3.2 GB game data); the offline half of the gate is `gen_jobs.py --check` +
   `test_gen_jobs.py`. Run the archive audit in the release/QA environment.
5. After merge, update `notes/client-hardcoded-systems-audit.md` §4.1/rows 1/2/3/8/§8 Phase 1 to the
   resolved state (this lane adds a separate report to avoid conflicting with Lead-owned updates of
   that living document).

## 8. Files

New:

* `tools/extract_job_facts.py`, `tools/job_facts.json`
* `tools/gen_jobs.py`, `tools/test_gen_jobs.py`
* `korangar/archive/data/client/jobs.ron`
* `korangar/src/world/jobs.rs` (loader + audit + 18 tests)
* this report

Changed:

* `korangar/src/world/entity/mod.rs` (appearance composition, entity type, scale, transcendent)
* `korangar/src/world/library/mod.rs` (headgear template, audit hooks, jobs/weapon load)
* `korangar/src/world/library/job_alias.rs` (sprite aliases removed)
* `korangar/src/world/weapon.rs` (job stem from the registry; test-only `Matrix4` import fix)
* `korangar/src/world/mod.rs`, `korangar/src/lib.rs`, `korangar/src/main.rs`
* `.github/workflows/quality.yml`

Deleted:

* `korangar/src/world/library/job_audit.rs`, `korangar/src/world/library/baby_job.rs`
