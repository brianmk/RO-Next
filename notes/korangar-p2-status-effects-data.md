# Korangar client — Phase 3 (P2): data-driven status effects

Owner: `p2-status-data` (subagent lane)
Branch: **`feat/p2-status-effects-data`**, worktree `/home/phantom/korangar-p2-status`
Commit: the single lane commit at the branch tip (this report is part of it); see
`git log -1 --format=%H feat/p2-status-effects-data`
Base/integration HEAD: **`f454127e992af85e555ee759b9afb2a9ca649103`** (`fix(session): return to
login instead of retrying stale character-server ids`)
Own build target: `/home/phantom/korangar-p2-target` (not the Lead-owned main tree target)
No Wine, no user characters, no game session; the client binary was only used for the
`--check-*` data CLI checks (no window/network).

Reference: `notes/client-hardcoded-systems-audit.md` §4 rows 16/17, §4.7, §8 Phase 3.

---

## 1. Result

Phase 3 is implemented end-to-end: status-effect **names, icons, categories, value-label
formats, refresh/stack policy and persistent status auras are now data**, generated from the
official server/client sources with a committed facts document, a `--check`-ed generator, a
compiled `include_str!` fallback plus archive override, `deny_unknown_fields` validation, a
loader, a `--check-status-effects` CLI and a test suite that proves parity with the tables
that were removed.

| before (f454127e) | after |
| --- | --- |
| `state/status_effect_names.rs` — 577 lines / 509 hardcoded names | deleted; names come from `status_effects.ron` (519 English names in data) |
| `state/status_effect_icons.rs` — 384 lines / 355 hardcoded icons | deleted; icons come from `status_effects.ron` (574 mapped icons in data) |
| `state/status_effects.rs` — `EXTRA_NAMES` (10), `ELEMENT_NAMES` (10), value-label match (6 arms), `SendVal` ids | deleted; extra names, element names and value semantics are data rows / data enum |
| `world/effect/status_aura.rs` — one `ASSUMPTIO` Rust constant + `match status_index { 110 | 473 }` | generic renderer; aura specs are data rows (Assumptio kept verbatim, 4 documented buff glows added) |
| no generator for the status tables (audit §4.10) | `tools/extract_status_facts.py` + `tools/gen_status_effects.py --check` |
| no status data CLI | `korangar --check-status-effects [PATH]` |

The client data file is

    korangar/archive/data/client/status_effects.ron     (128 KiB, 609 status rows, sorted/unique ids)

loaded through `GameFileLoader` (server-overridable inside the client archive) and embedded
with `include_str!` as the compiled fallback, exactly like the existing skill-effect pipeline.

**No gameplay change:** only presentation data/lookup changed. The packet/timer state machine,
the buff bar, the aura reconciliation (`World::update_status_auras`) and the warp behaviour
(`retain_for_entity` keeping the local player's statuses across a map change) are untouched.
Unknown/missing status ids keep the documented fallbacks (generic `Status effect` name, no
icon, no aura, raw value line); a malformed archive override is rejected with a warning and the
compiled fallback is used (never an empty table/panic).

---

## 2. Data pipeline

### 2.1 Extraction — `tools/extract_status_facts.py` (1,411 lines)

Self-contained (has its own read-only GRF reader ported from
`loaders/archive/native/{mod,mixcrypt}.rs`; no dependency outside the repo) and deterministic.
Sources read:

* rAthena map server (CondaRO 2019 tree `/home/phantom/popeng-hotspots/condaro`):
  * `src/map/status.hpp` — `enum efst_type` (packet ids; 1,467 entries parsed including the
    `XXX_EFST_WORLDSTORE_ACTIVE` entry) and `enum sc_type`,
  * `src/map/map.hpp` — `enum e_element` (10 element names used by the `Element` value format),
  * `db/pre-re/status.yml` — `Status`, `Icon`, `DurationLookup`, `MinDuration`, `Flags`
    (`DisplayPc`, `DisplayNpc`, `Debuff`, `SendVal1..3`, ...), `CalcFlags`.
* Official 2019 client Lua tables from the GRF chain (`archive/` > `palettes.grf` >
  `CondaRO.grf` > `data.grf`):
  * `stateiconinfo.lub` (English titles + title colours = buff/debuff/toggle/system category,
    `haveTimeLimit`/`posTimeLimitStr`), 597 symbols,
  * `stateiconimginfo.lub` (explicit status icon image names, priority groups), 318 entries,
  * `efstids.lub` (`EFST_* -> id`; used to record client/server id provenance/drift).
* `tools/status_info_robrowser.json` — **vendored facts** of roBrowserLegacy
  `src/DB/Status/StatusInfo.js` + `StatusConst.js` (the reverse-engineered classic hard-coded
  status icon/title table the old icon table was generated from), with upstream URL, commit
  `2f9a9d6cf5b04a8ac7a44077080772046e15bcb7`, and SHA-256 of both files. Regenerate with
  `--vendor-robrowser PATH/StatusInfo.js PATH/StatusConst.js --robrowser-commit <sha>`.
* `tools/status_effect_custom.ron` — the only hand-written status content left (see §2.2),
  SHA-256 recorded in the facts provenance.

Output: **`tools/status_facts.json`** (1.0 MiB, 609 merged status rows) containing the parsed
tables (`efst_enum`, `sc_enum`, `elements`, `status_yml`, `client_state_info`, `client_icons`,
`robrowser_icons`), the merged `statuses` rows and a provenance block with every source path +
SHA-256 (server files, Lua files + which archive they came from, vendored facts + commit,
custom overlay) and counts (including the 16 client-referenced icon files that the shipped GRFs
do not contain — those statuses fall back to the name; list in `provenance.dropped_icons`).

`--check` re-extracts and fails if the committed JSON is stale. The `--check` needs the server
tree + game data (manual/verifier run); CI only needs the committed JSON.

### 2.2 Curated overlay — `tools/status_effect_custom.ron` (266 lines)

Only facts the official sources cannot express, each row with a `source` string:

* `names` (10) — client-only food statuses (STR/AGI/VIT/DEX/INT/LUK Food), Fury
  (`Critical Explosion` 86), Monster transformation (665), Push cart (673), Bleeding (889);
  the 2019 client has no usable English title for them.
* `icons` (5) — the old table's deliberate generic `weapon_property.tga` for the four elemental
  weapon-enchant statuses (898–901) and the missing `.tga` suffix of `i_accuracy` (210).
* `categories` (1) — Bleeding (889) → `Debuff` (the health-state status has no title-table
  entry and no `status.yml` display icon).
* `value_labels` (7) — the semantics of the `SendVal1` statuses: Enchantarms/64 (element),
  cash EXP/item/job boosts 250/252/312 (percent), Push_Cart 673 (number), monster transforms
  621/665 (id).
* `auras` (6 rows) — see §4.

### 2.3 Generator — `tools/gen_status_effects.py` (383 lines)

Reads only `tools/status_facts.json`, validates it (schema version, strictly ascending ids,
ASCII names, `.tga` icons, known category/refresh/stack/value-format enums, aura ranges) and
renders the canonical RON file. `--check` is the CI drift guard (byte comparison with the
committed file). It runs without game data or the server tree.

### 2.4 Client file schema (`status_effects.ron`)

```ron
(
    provenance: "...",
    version: 1,
    value_enums: ( element: ["Neutral", "Water", ...] ),      // `Element` value format
    statuses: [
        (
            id: 10,                    // EFST id, strictly ascending, unique
            name: "Blessing",          // optional: omitted when no safe English name
            icon: "블레싱.tga",         // file inside data\texture\effect\
            category: Buff,            // Buff | Debuff | Toggle | System
            refresh: Replace,          // Replace | Keep
            stack: Single,
            value_labels: [ (slot: 1, label: "Element", format: Element) ],   // optional
            aura: (                    // optional
                kind: PersistentGlow,
                textures: (ground: "...", ring: "...", body_frames: ["..."], body_glow: "..."),
                tint: (ground: (r, g, b), body: (...), glow: (...)),
                radii: (ground: 11.0, ring: 7.5, body: 26.0, body_glow: 15.0),
                timing: (body_frame_rate: 8.0, ring_rotation_speed: 0.8, body_rotation_speed: 0.35,
                         body_camera_offset: 2.5, pulse_speed: 2.2, pulse_amount: 0.22),
                alpha: (ground: 0.42, ring: 0.65, body: 0.6, body_glow: 0.38),
                height: 13.0,
            ),
        ),
        ...
    ],
)
```

All structs use `#[serde(deny_unknown_fields)]`; optional fields accept both the bare and the
explicit `Some(...)` spelling; `Option` fields use the existing skill-pipeline
`BareOrOption` helper so the generated file stays readable.

### 2.5 Runtime loader

New `korangar/src/world/library/status_effects.rs` (1,002 lines incl. tests), the same pattern
as `world/library/skill_effects.rs`:

* compiled fallback (`include_str!`) parsed on first use, archive override loaded in
  `Library::new`, malformed override → warning + compiled fallback;
* process-wide static table with leaked process-lifetime strings; **binary search** by id,
  allocation-free lookups (the buff bar already caches its formatted tooltip strings);
* API used by the rest of the client:
  `status_effect_info(id)`, `status_aura_spec(id)`, `status_effect_value_lines(id, values)`,
  `status_effect_refresh_policy(id)`, plus the `--check` validation entry points;
* validation: duplicate ids, unknown fields, non-ASCII/# names, non-`.tga` icons, bad value
  slots/formats, aura texture extensions/ranges, unsupported aura kind;
* documents the fallback contract (missing id → `None`, caller keeps its generic fallback).

`state/status_effects.rs` now only keeps the packet/timer state machine + the generic fallback
name and resolves name/icon/category/value lines/refresh policy through the loader (the
`EXTRA_NAMES`/`ELEMENT_NAMES`/value `match` are gone). `world/effect/status_aura.rs` keeps the
procedural renderer and gets the `StatusAuraSpec` from the data; the hardcoded `ASSUMPTIO`
constant, the `EFST_ASSUMPTIO*` ids and the `match status_index` arm are deleted.

CLI: `korangar --check-status-effects [PATH]` (mirrors `--check-skill-effects`) and
`korangar::check_status_effects(Option<&Path>)`.

---

## 3. Coverage and parity evidence

Counts in the committed data (verified by tests, not hand-counted):

| item | count |
| --- | --- |
| status rows in `status_effects.ron` | **609** |
| rows with an English name | **519** |
| rows with an official status icon | **574** |
| categories | Buff 514, Debuff 82, Toggle 10, System 3 |
| `SendVal` statuses with value labels | **7** |
| persistent aura rows (status ids) | **6** (110, 473, 3, 9, 10, 12) |
| refresh/stack policy | all `Replace` / `Single` (the 2019 client model; fields are data so an override can change them) |
| client icon files referenced by the tables but absent from the shipped GRFs | 16 (documented in `provenance.dropped_icons`; those statuses fall back to the name) |

**Parity with the removed hand-generated tables** is a committed Rust test
(`world/library/status_effects::tests`), reading the frozen snapshot
`tools/status_effect_legacy_tables.ron` (test-only oracle, 509 names + 355 icons + 10 extra
names extracted from the deleted files at f454127e):

* **icons: 355/355 ids covered with the same file name** (case-insensitively — the GRF texture
  lookup is case-insensitive and the client tables disagree on ASCII case for a few names);
* **names: 509/509 covered**, with one documented correction. Ids **1064–1072** were labelled
  one status too early by the old generated table: its server-enum parser skipped the
  `XXX_EFST_WORLDSTORE_ACTIVE` enum entry, while the *real* server enum and the client
  `efstids.lub` agree on the ids. Evidence: with the correct enum parse, **1,005/1,005 common
  `EFST_*` symbols have the same id in `efstids.lub` and the server enum (0 mismatches)**, and
  the names are now at the real ids (`1065 = Infinity Drink`, `1073 = Curse of The Evil Eye`;
  `1064 = EFST_SP_SHA` has no English title and correctly keeps the generic fallback). The test
  maps the 9 legacy ids with an explicit `LEGACY_NAME_SHIFT` range, so the old labels are still
  proven to be reachable — just at the correct ids;
* the 10 curated extra names are present at their ids.

Other parsed-data facts: 1,467 server `efst_type` ids, 1,022 `sc_type` ids, 699 `status.yml`
entries, 597 client title entries, 318 explicit client icons, 313 symbol-verified classic
roBrowser icons. Classic icons are mapped by **symbol**, not by the drifted numeric id
(normalized `SC.*` ↔ `EFST_*`), so a client/server enum drift can never select the wrong icon.

---

## 4. Persistent status auras

`world/effect/status_aura.rs` is now a generic renderer (the `PersistentGlow` kind):
ground light pool + rotating ring + camera-facing animated body shield + body light, pulsing out
of phase, sitting-aware, attached to the existing effect budget/culling/cleanup path. The aura
spec (textures, tints, radii, timing, alpha, body height) is a data row; adding an aura is a
`tools/status_effect_custom.ron` row + regeneration, no Rust change.

* **Assumptio 110/473 — moved verbatim, first data row.** Every value of the deleted
  `const ASSUMPTIO` is in the data row and unit-tested for exact equality: textures
  `whitelight.tga` / `efst_waterfield2\bubble1.bmp` / `aurawaterfield1..4.tga`, tints
  (0.45, 0.8, 1.0)/(0.65, 0.88, 1.0)/(0.8, 0.95, 1.0), radii 11/7.5/26/15, alphas
  0.42/0.65/0.6/0.38, frame rate 8, rotations 0.8/0.35, camera offset 2.5, pulse 2.2/0.22,
  body height 13.0 (the old `BODY_HEIGHT`). The renderer body is the same code (only the
  lookup and the `body_height` source changed), so the recently fixed centered shield is
  preserved. `icon05.tga` for 110/473 is now produced by the official roBrowser
  `SC.ASSUMPTIO`/`SC.ASSUMPTIO2` facts (same icon), and the warp persistence behaviour is
  untouched (`retain_for_entity`).
* **Blessing 10 / Increase Agility 12 / Angelus 9 / Improve Concentration 3 — documented
  procedural additions.** The official 2019 client has **no persistent loop effect** for these
  (it only plays the one-shot cast effect; the extracted EffectTable has no `efst_*` loop data
  for them — see the pipeline notes). The rows are therefore marked
  `source: "procedural addition, documented: ..."` and reuse the official cast-effect
  textures/tints (`effect\pok2.tga`, `effect\ac_center2.tga`, `effect\agi_up.bmp`,
  `effect\lens1/2.tga`, `effect\ring_yellow.tga`, shared `whitelight.tga`, and the
  `efst_c_blessings_of_soul` sparkle frames), so they match the game's look without inventing
  new art. Removing the four rows removes the glows with no Rust change if the lead/verifier
  decides they are not wanted.
* Every aura texture is verified to exist in the GRFs by the extractor (extraction fails
  otherwise; all 26 texture paths verified during generation).

No in-client visual sweep was run in this lane (no Wine/user chars, no server session): the
Assumptio data row is value-for-value identical to the old constant and unit-tested, and the
renderer's drawing code is unchanged apart from the data lookup, so the visual risk is limited
to the four new procedural rows. A muted Xvfb/GM-character sweep of Assumptio + the four buffs
is the natural verifier follow-up (the prior Assumptio lane has the test-account recipe in
`notes/korangar-assumptio-aura-icon-report.md`).

---

## 5. Verification (commands and results)

Run from `/home/phantom/korangar-p2-status` with
`CARGO_TARGET_DIR=/home/phantom/korangar-p2-target PATH="$HOME/.local/bin:$PATH"`.

| check | command | result |
| --- | --- | --- |
| unit tests | `cargo test -p korangar --lib --all-features` | **465 passed, 0 failed, 11 ignored** |
| status test surface | `cargo test -p korangar --lib -- --list \| grep -cE "status_(effect\|aura)"` | 35 status tests (13 loader, 16 state, 5 aura, 1 skill-pipeline cross-check) |
| CI-style clippy | `cargo clippy -p korangar --all-features -- -D warnings` | pass |
| formatting | `cargo fmt --all -- --check` | pass |
| data drift | `./tools/gen_status_effects.py --check` | `… is up to date (609 statuses: 519 names, 574 icons, 7 value labels, 6 auras)` |
| extraction drift | `./tools/extract_status_facts.py --check --server-root /home/phantom/popeng-hotspots/condaro --game-dir ../client/game` | `… is up to date (609 statuses)` |
| Python data invariants | `./tools/test_status_effects_data.py` | `Ran 12 tests … OK` |
| schema CLI (compiled) | `korangar --check-status-effects` | `status effects: 609 statuses are valid` (exit 0) |
| schema CLI (committed file as override) | `korangar --check-status-effects korangar/archive/data/client/status_effects.ron` | 609 valid (exit 0) |
| schema CLI (malformed override) | bad `category: Bogus` / unknown `typo:` field | exit 1 with the serde error (`Unexpected variant named Bogus …`, `Unexpected field named typo …`) |
| skill pipeline untouched | `korangar --check-skill-effects` | `436 (skill, phase) descriptors are valid` |
| known pre-existing red gate | `cargo test -p korangar --test ui_consistency` | 2/3 pass; `hardcoded_style_literals_do_not_exceed_the_token_budget` fails with **29 colors > budget 28** — this is the design-token ratchet the audit already documents as failing at integration HEAD (`notes/client-hardcoded-systems-audit.md` §1/§9), untouched by this lane (no interface file changed) |

The Rust tests additionally cover: compiled-fallback validity, sorted/unique ids, legacy
name+icon+extra-name parity (above), known buffs names/icons/categories, every data category
present, value-label formatting (element/percent/number/id/generic/unknown element),
`SendVal` ids, aura lookup for 110/473/3/9/10/12, no aura for non-aura statuses/unknown ids,
Assumptio spec exact-value regression + size/alpha invariants, aura ranges valid for every
spec, malformed data (bad RON, unknown field, duplicate id, missing category, non-`.tga`
icon), unknown status fallbacks, refresh/stack policy, and the packet/timer state machine
(unchanged tests still green, including the warp `retain_for_entity` case).

CI parity: `.github/workflows/tests.yml` now runs

```yaml
- name: Check generated status-effect data
  run: |
    ./tools/gen_status_effects.py --check
    ./tools/test_status_effects_data.py
```

after the unit tests. Both steps need no game data/server tree.

Reproduce the full pipeline (extraction needs the server tree + client archives):

```sh
./tools/extract_status_facts.py \
    --server-root /home/phantom/popeng-hotspots/condaro \
    --game-dir ../client/game \
    --robrowser-commit 2f9a9d6cf5b04a8ac7a44077080772046e15bcb7
./tools/gen_status_effects.py
./tools/gen_status_effects.py --check
```

---

## 6. Files

| file | change |
| --- | --- |
| `tools/extract_status_facts.py` | **new** — GRF reader + rAthena/Lua/roBrowser parsers + curated merge + provenance JSON + `--check` + `--vendor-robrowser` |
| `tools/status_facts.json` | **new** — committed extracted facts, SHA-256 provenance |
| `tools/status_info_robrowser.json` | **new** — vendored roBrowserLegacy classic status facts (URL, commit, file hashes) |
| `tools/status_effect_custom.ron` | **new** — curated overlay (extra names, icon choices, category override, `SendVal` semantics, 6 aura specs) |
| `tools/gen_status_effects.py` | **new** — facts → client RON, validation, `--check` |
| `tools/status_effect_legacy_tables.ron` | **new** — frozen 509/355/10 snapshot, test-only parity oracle |
| `tools/test_status_effects_data.py` | **new** — 12 Python invariant/parity/drift tests |
| `korangar/archive/data/client/status_effects.ron` | **new** — generated client data (609 rows) |
| `korangar/src/world/library/status_effects.rs` | **new** — loader, schema, validation, value formatting, aura lookup, 13 tests |
| `korangar/src/state/status_effect_names.rs` | **deleted** (509-line hardcoded table) |
| `korangar/src/state/status_effect_icons.rs` | **deleted** (355-line hardcoded table) |
| `korangar/src/state/status_effects.rs` | `EXTRA_NAMES`/`ELEMENT_NAMES`/value match deleted; lookups via the loader; refresh policy from data |
| `korangar/src/world/effect/status_aura.rs` | generic `PersistentGlow` renderer; hardcoded Assumptio spec + match arm deleted; data-driven `body_height`; tests |
| `korangar/src/world/library/mod.rs`, `korangar/src/world/mod.rs`, `korangar/src/state/mod.rs` | module wiring / re-exports |
| `korangar/src/lib.rs` | `check_status_effects` + doc updates |
| `korangar/src/main.rs` | `--check-status-effects` CLI |
| `korangar/src/world/library/skill_effects.rs` | test-only path update after the aura lookup moved (no pipeline change) |
| `.github/workflows/tests.yml` | generator `--check` + data tests |

Diff: **+277 / −1,262** Rust lines (the deleted generated tables) plus the new data/tooling
files; no other subsystem touched.

---

## 7. Known limitations / follow-ups

* The 16 client-table icon references that the shipped GRFs do not contain are dropped (logged
  in the facts provenance); those statuses show their name instead of an icon. If a future data
  set ships those `.tga`s, re-running the extractor picks them up automatically.
* The four new buff glows are documented procedural additions (not official persistent-effect
  data) and are deliberately data rows so they can be removed/retuned without Rust changes.
* Official `ez2streffect` loop data for newer statuses (`efst_*.tga` files exist in the client
  data) is not wired: the extracted EffectTable facts have no `efst_*` effect names, so a
  generic "official loop effect" aura kind would need that extraction first. The schema's
  `kind` enum is the extension point.
* No in-client visual sweep in this lane (see §4); recommend the verifier run the muted
  Xvfb/GM-character sweep for Assumptio + the four new glows.
