# Item stats data pipeline (hardcoded-systems remediation, Phase 2 / P1)

Branch: `p1/item-stats` (integration base `f454127e`, the current
`integration/all-fixes-20261121` HEAD).
Worktree: `/home/phantom/korangar-item-stats`; own
`CARGO_TARGET_DIR=/home/phantom/cargo-target-p1-item-stats`. Main tree left
untouched.

Commits:

| commit | content |
| --- | --- |
| `e801d18f3edcd74e311528850d687bc91eff4465` | feat(item): data-driven item stats from the server item database |
| this report commit | docs: item stats data report |

No Wine, no user characters and no live server were used. The game archives
were only read offline (ignored `cargo test --ignored` check), no client window
was started.

## 1. Problem (audit §4.3 / §8 Phase 2)

`korangar/src/world/library/item_stats.rs` evaluated the item stat panel by
parsing the localized `iteminfo.lub` description text: seven Korean field labels
(`계열`/`공격`/`방어`/`무게`/`무기레벨`/`요구레벨`/`장착`) and thirteen Korean
conditional-bonus markers. The description is display data of one specific
translation, so a translated (English) iteminfo silently produces an empty or
wrong panel (the English label set was only partially understood: e.g.
`Level Requirement` did not match the parser's `Required Level`, and
`Class`/`Jobs` lines were not recognized at all).

A second problem showed up while validating against the real client data: the
`iteminfo.lub` text is **not** consistent with the pre-renewal server database
(the shipped descriptions are a mix of pre-renewal and renewal values). The
server is the authority for the numbers it uses; the old parser showed the
description text instead.

## 2. Data pipeline

### 2.1 Facts extractor — `tools/extract_item_facts.py`

Reads the CondaRO pre-renewal server tree
(`popeng-graft/scratch-condaro`):

* `db/pre-re/item_db.yml` — the import manifest; its `Footer.Imports` list is
  followed instead of hard-coding the file split
  (`item_db_usable.yml`, `item_db_equip.yml`, `item_db_etc.yml`),
* `src/config/renewal.hpp` and `src/custom/defines_pre.hpp` for the server mode
  and packet version.

Output: committed `tools/rathena_pre_re_item_facts.json` (2.6 MB) with
provenance (per-file sha256, manifest sha256, `PACKETVER`, renewal mode, item
counts) and one row per item:

```
id, aegis, type, subtype, weapon_type, slots, refineable, view,
equip_positions, usable, weight, attack, defense,
weapon_level, armor_level, required_level, flat_bonuses
```

Item script bonuses are extracted only when they are **provably
unconditional**:

* `//` comments are stripped, string literals (autobonus trigger payloads) are
  masked and never counted,
* bonus statements inside `{...}` blocks are skipped,
* a bonus statement attached to a brace-less `if`/`else`/`for`/`while` is
  skipped,
* only mapped flat stat constants are taken (`bStr`, `bAgi`, ..., `bMdf`,
  `bAspd`, plus the composite `bAllStats`/`bAgiVit`/`bAgiDexStr`);
  percentage/rate constants, race/element bonuses and everything else stay out.

`--check` compares a fresh extraction with the committed JSON. As the server
tree is not a git checkout, `server_head` is empty and provenance relies on the
per-file sha256 values plus `PACKETVER 20190605` / `pre-renewal (RENEWAL
disabled)`.

```
$ ./tools/extract_item_facts.py
tools/rathena_pre_re_item_facts.json: 6169 items (2039 equipment, 1905 usable, 1034 with flat bonuses)
$ ./tools/extract_item_facts.py --check
tools/rathena_pre_re_item_facts.json is up to date (6169 items)
```

### 2.2 Generator — `tools/gen_item_stats.py`

Input: the committed facts JSON (so the generator and its drift guard run
offline/reproducibly). Output:
`korangar/archive/data/client/item_stats.ron` (329 KB), a strictly ascending
list of `(item id, stats)` where default fields are omitted and `weight` is
always present:

```
(
    version: 1,
    provenance: "...",
    items: [
        (501, (weight: 7)),
        ...
        (1104, (weight: 60, attack: 39, weapon_level: 1, required_level: 2, slots: 3, refineable: true)),
        (1123, (weight: 90, attack: 120, weapon_level: 3, required_level: 27, slots: 1, refineable: true, bonuses: (intelligence: 3))),
        ...
    ],
)
```

* `weight` = rAthena DB weight (0.1 unit) divided by 10, i.e. the whole client
  unit the server uses for its own display and calculations. Seven DB weights
  are not multiples of 10 (`1243`, `1535`, `2112`, `2352`, `2414`, `2510`,
  `5055`); like the server, the remainder is truncated.
* `attack`/`defense` are the DB `Attack`/`Defense` values (also for ammo, whose
  attack is the arrow attack), `weapon_level`/`armor_level` follow the rAthena
  defaults (1 for the matching type, 0 otherwise), `required_level` is
  `EquipLevelMin`, `slots` is `Slots`, `refineable` is `Refineable`.
* `bonuses` are the unconditional flat bonuses from the item script.

```
$ ./tools/gen_item_stats.py
korangar/archive/data/client/item_stats.ron: 6169 items, 1034 with flat bonuses, 329063 bytes
$ ./tools/gen_item_stats.py --check
korangar/archive/data/client/item_stats.ron is up to date (6169 items)
```

### 2.3 Loader — `world/library/item_stats.rs`

* Data file path `data\client\item_stats.ron` is loaded through
  `GameFileLoader` (archive override), the same file is embedded with
  `include_str!` as the compiled fallback. A malformed/missing override logs a
  warning and keeps the compiled table.
* `#[serde(deny_unknown_fields)]` on the file and on `ItemStats`/`StatBonuses`;
  the loader rejects unsupported versions, empty tables, item id 0,
  duplicate/out-of-order ids. Schema version is `1`.
* `ItemInfo::load` resolves stats data-first and starts with a debug summary:
  `item stats: <n> items from the item database data, <n> description
  fallbacks, <n> generic fallbacks`.
* Card slot count now prefers the server DB `Slots` (26 client
  `iteminfo.lub slotCount` values disagree with the server DB; the DB is
  authoritative). `iteminfo.lub` remains the fallback for client-only ids.
* Existing consumers (`world::ResourceMetadata::stats`,
  `interface/windows/item_info.rs`, `interface/components/equipment_stats.rs`)
  are unchanged; the item description text stays display-only.

### 2.4 Default path vs last-resort Korean parser

The Korean description grammar was moved unchanged into
`world/library/item_stats_fallback.rs`. It is only reached for item ids that
are not in the item database data (custom/client-only leftover ids), logs a
debug message per id, and the per-startup summary counts those items.

* Item in data → data (no description parsing; language irrelevant).
* Item not in data but with a description → last-resort parser (logged debug,
  counted in the startup summary).
* Item not in data and no description → `ItemStats::DEFAULT` generic fallback
  with a `warn_once` per item id, documented on `ItemStats::DEFAULT`; not a
  silent zero.
* Source is tracked as `ItemStatsSource::{Data, DescriptionFallback,
  GenericFallback}` and covered by tests.

### 2.5 `--check-items` CLI

`korangar --check-items [PATH]` validates the compiled fallback (no path) or an
archive override file, without starting a window, network or the game archives:

```
$ ./target/debug/korangar --check-items
item stats: 6169 item rows are valid
$ ./target/debug/korangar --check-items korangar/archive/data/client/item_stats.ron
item stats: 6169 item rows are valid
$ ./target/debug/korangar --check-items /tmp/item_stats_bad_field.ron
item stats: invalid data: ... Unexpected field named `attck` in `ItemStats`, ...
exit 1
$ ./target/debug/korangar --check-items /tmp/item_stats_bad_order.ron
item stats: invalid data: ... item ids must be strictly ascending and unique; 500 follows 501
exit 1
```

Because the client archive already includes the repository's
`korangar/archive/` (see the release workflow: `cp -r korangar/archive
staging/` and the `archive/` entry in `client/game_archives.ron`), no new
runtime archive mechanism is needed: the committed file is shipped as
`archive/data/client/item_stats.ron` and can be overridden by a server without
a rebuild. The compiled copy is the fallback.

## 3. Single item-type/category source

New `korangar-networking/src/item_type.rs` is the only definition of the
rAthena `enum item_types` values (`HEALING` .. `CASH`, including the
`CASH_OLD = 12` legacy note) plus the classification:

* `is_equipment(u8)` — used by the 2019 storage-item packet handler,
* `is_usable(u8)` — used by the inventory item use action,
* `category(u8) -> ItemTypeCategory` — used by the inventory tab classification.

The duplicate constants are removed:

* `korangar-networking/.../version_20220406.rs`: the four local
  `ITEM_TYPE_*` constants and their `matches!` are gone (protocol packet fields
  are unchanged).
* `korangar/src/state/inventory.rs`: the twelve local `pub mod item_type`
  constants are replaced by `pub use korangar_networking::item_type;` (the
  public path stays stable) and the predicates.

```
$ grep -rn "ITEM_TYPE_" --include="*.rs" .            # no matches
$ grep -rn "pub mod item_type" --include="*.rs" korangar/src   # no matches
```

## 4. Tests and verification

### Rust

| target | result |
| --- | --- |
| `cargo test -p korangar --lib` (default) | 453 passed, 0 failed, 12 ignored |
| `cargo test -p korangar --lib --all-features` | 465 passed, 0 failed, 12 ignored |
| `cargo test -p korangar-networking --all-features` | 64 passed, 0 failed |
| `cargo test -p ragnarok-packets --all-features` | 23 + 10 passed, 0 failed |
| `cargo fmt --all --check` | OK |
| `cargo clippy --workspace -- -Dwarnings` | OK |
| `cargo clippy --workspace --all-features -- -Dwarnings` | OK |

New Rust tests cover: compiled data validity/sortedness (6169 rows),
authoritative values for known items, data-first resolution with a
label-free description (the former failure mode), documented generic fallback,
logged description fallback, slot count from the DB, malformed data rejection
(missing version, unknown field, unsupported version, empty table,
duplicate/out-of-order ids, id 0, invalid RON), archive-less load, the item type
value/classification invariants and inventory tab/use-action mapping.

Known pre-existing CI failure (not caused by this change, audit §4.5): the
`ui_consistency::hardcoded_style_literals_do_not_exceed_the_token_budget`
integration test is red at the integration base as well
(29 colour literals vs budget 28). No interface/theme file or the budget file
was touched by this branch (`git diff f454127e -- korangar/src/interface
korangar/test-data` is empty).

Ignored end-to-end check on the real CondaRO client data
(`cargo test -p korangar --lib condaro_archives_are_covered -- --ignored
--nocapture`, run from the `korangar` crate root with the local archive
symlinks, no window/Wine):

```
item stats archive check: 5916/9647 iteminfo items covered by the item database data;
the description parser would disagree on 1706/5916
(attack 57, defense 713, weight 207, weapon level 86, required level 497, bonuses 662)
```

Interpretation:

* 5916 of the 9647 ids of the loaded `iteminfo.lub` are covered by the server
  DB data (the remaining 3731 are client-only ids and use the logged last-resort
  fallback; they used to be parsed from the description anyway).
* For 1706 of the covered items the (old) client description text disagrees
  with the pre-renewal server database. Spot checks show the shipped text
  contains renewal-era values, e.g. Sniping Suit (`2367`) description
  `방어 : 42` = renewal DB, pre-renewal DB = 5; Mystery Fruit Shell (`5050`)
  description 10 vs pre-re DB 5. The client is connected to the pre-renewal
  server, so the new data values are the ones the server actually uses.

### Python (`tools/test_item_stats.py`, 24 tests, `OK`)

* facts provenance (sha256s, counts, PACKETVER/renewal mode, sorted/unique ids,
  type/default invariants, known item facts),
* flat bonus parser unit tests (conditional blocks, brace-less `if`,
  `autobonus` strings, comments, composite constants, percentages, negatives,
  zero values),
* generated RON coverage (every fact row exactly once, sorted), weight
  conversion, defaults omitted, flat bonus rendering, `--check` drift guard,
* no-Korean-label dependency: the data-first module, the generator, the
  extractor and the generated data contain no Hangul; the Korean grammar only
  exists in `item_stats_fallback.rs`.

### CI parity

`.github/workflows/quality.yml` now runs on every branch/PR:

* in the lint job: `tools/test_item_stats.py` + `tools/gen_item_stats.py
  --check` (Python venv with PyYAML),
* in the unit-test job: `cargo run -p korangar -- --check-items`.

The facts extractor itself is the only script that needs the server tree; CI
uses the committed facts and the two drift/schema guards.

## 5. Acceptance check

| acceptance item | evidence |
| --- | --- |
| English `iteminfo.lub` yields the same stat panel as Korean data | stats come from language-independent DB data; `data_path_does_not_depend_on_the_description_language` test; archive check resolves 5916 real iteminfo items through the data path |
| no Korean grammar dependency in the default path | Hangul only in `item_stats_fallback.rs`; Python and Rust tests enforce it |
| one item-type constant source | `korangar-networking/src/item_type.rs`, both users migrated, duplicate constants gone |
| unknown item id → documented fallback, not silent zero | `ItemStatsSource` tests + `warn_once` docs on `ItemStats::DEFAULT` |
| drift/schema CLI | `gen_item_stats.py --check`, `extract_item_facts.py --check`, `korangar --check-items [PATH]`, CI steps |

## 6. Files changed

Added:

* `tools/extract_item_facts.py`
* `tools/rathena_pre_re_item_facts.json` (committed facts, 2.6 MB)
* `tools/gen_item_stats.py`
* `tools/test_item_stats.py`
* `korangar/archive/data/client/item_stats.ron` (generated, 329 KB)
* `korangar-networking/src/item_type.rs`
* `korangar/src/world/library/item_stats_fallback.rs`

Modified:

* `korangar/src/world/library/item_stats.rs` (data-first loader/`Table`)
* `korangar/src/world/library/item_info.rs` (data-first stats, DB slot count,
  startup summary)
* `korangar/src/world/library/mod.rs` (module + load/exports)
* `korangar/src/lib.rs` (`check_item_stats`)
* `korangar/src/main.rs` (`--check-items`)
* `korangar/src/state/inventory.rs` (single item-type source, tests)
* `korangar-networking/src/lib.rs` / `packet_versions/version_20220406.rs`
* `.github/workflows/quality.yml` (CI parity)

## 7. Regeneration / verification quick start

```sh
# regenerate the facts (needs the CondaRO server tree)
./tools/extract_item_facts.py

# regenerate the data file / verify drift
./tools/gen_item_stats.py
./tools/gen_item_stats.py --check

# schema check of the compiled fallback or an override
cargo run -p korangar -- --check-items
cargo run -p korangar -- --check-items path/to/item_stats.ron

# data tests
./tools/test_item_stats.py
cargo test -p korangar --lib item_stats
cargo test -p korangar-networking item_type
```

## 8. Notes for the Lead

* Branch `p1/item-stats` is based on `f454127e` and is ready to merge into
  `integration/all-fixes-20261121`. The item stats code/data is additive and
  the new data file lives in the existing `korangar/archive/data/client/`
  mechanism; no new runtime archive setup is required.
* Merge-conflict hot spots: `world/library/mod.rs`,
  `world/library/item_info.rs`, `state/inventory.rs`, `main.rs` (CLI argument
  list) and `korangar-networking` (if another branch touches item type values).
* The item description text is left untouched (display only). It disagrees with
  the pre-renewal server DB for some items (the shipped `iteminfo.lub` mixes
  pre-renewal and renewal values); the stat rows now match the server.
* `ui_consistency` token-budget failure is pre-existing (audit §4.5) and
  unrelated to this branch.
