# Korangar client — Phase 6 (P3): data-driven alias tables (jobs/monsters/skills)

**Lane:** `p3-alias-data` · **Branch:** `p3/alias-data` · **Base/integration HEAD:**
`7ce0f4cd582421514340988dcd4ae0ad34d2f4ef` (`feat(skills): data-driven skill behaviour and
procedural effect data`) · **Worktree:** `/home/phantom/Games/ragnarok/CondaRO-linux/korangar-p3-alias-data`
· **Build target:** `/home/phantom/cargo-target-p3-alias` (not the Lead-owned main tree target)
· **Date:** 2026-11-22

**Scope:** `notes/client-hardcoded-systems-audit.md` §4.10 and §5 row 31 (`job_alias.rs`,
`monster_sprite_alias.rs`, `skill_icon_alias.rs`: generated tables whose generators were kept
untracked; the Assumptio icon had to be hand-added, proving the drift).

**Commit:** the single lane commit at the tip of `p3/alias-data` (this report is part of it); see
`git log -1 --format=%H p3/alias-data`.

**Mode:** no Wine, no user characters, no game session. The client binary was only used for the
`--check-aliases` data CLI and the archive-backed unit tests (no window, no network).

---

## 1. Result

| before (`7ce0f4cd`) | after |
| --- | --- |
| `world/library/job_alias.rs` — 56-row `JOB_SKILL_TREE_FALLBACKS` Rust array | deleted; data row list in `aliases.ron`, `job_alias.rs` keeps `warn_once` + the lookup wrapper |
| `world/library/monster_sprite_alias.rs` — 232-row `MONSTER_SPRITE_ALIASES` + 7 `GENERIC_MONSTER_SPRITES` | module deleted; rows in `aliases.ron` |
| `world/library/skill_icon_alias.rs` — 241-row `SKILL_ICON_ALIASES` + 3 `GENERIC_SKILL_ICONS` | module deleted; rows in `aliases.ron` |
| generators `generate_{job,monster_sprite,skill_icon}_alias.py` **untracked** (in `/home/phantom/korangar-portals`) | committed extractor `tools/extract_alias_facts.py` + facts `tools/alias_facts.json` + generator `tools/gen_aliases.py --check` + tests `tools/test_gen_aliases.py` |
| no data file, no loader, no CLI | `korangar/archive/data/client/aliases.ron` (39.6 KiB) loaded via `GameFileLoader` with `include_str!` fallback, archive override, `deny_unknown_fields`, `--check-aliases` CLI |
| drift invisible to CI | generator `--check` + generator tests in `quality.yml`; `--check-aliases` schema step in the tests job; data/unit tests in `cargo test` |

The runtime alias data is byte-for-byte behaviour-preserving: `tools/alias_legacy_parity.json`
pins the three removed Rust tables at the integration HEAD and `tools/test_gen_aliases.py`
asserts the generated file reproduces them **entry-for-entry** (56 job fallbacks, 232 monster
aliases + 7 generics, 241 skill icon aliases + 3 generics). The archive-backed unit tests (run
serially against the real CondaRO GRFs) additionally prove that all **1008** rAthena mob ids and
all **1229** skill ids resolve through the new tables exactly as before (see §5).

**Note on the job sprite aliases:** `PLAYER_JOB_SPRITE_ALIASES` (63 rows) was already retired by
the merged P1 lane (`p1/jobs-appearance-data`, `432f59db`): it lives in
`archive/data/client/jobs.ron` as documented `source: Fallback` rows and `jobs.ron` is owned by
that lane. This lane therefore carries the *remaining* job alias table (skill-tree fallbacks) in
`aliases.ron` and does **not** duplicate the job/appearance registry.

---

## 2. Data pipeline

```
rAthena server tree (HEAD 89c8bfa6…)                    CondaRO client archives (GRF chain)
  src/common/mmo.hpp        enum e_job                   data.grf / CondaRO.grf / palettes.grf
  db/pre-re/mob_db.yml      mob ids + Aegis names          file tables (sprite/icon existence probe)
  db/re/mob_db.yml          renewal mob db (see 2.2)      jobidentity.lub      JTtbl identities
  db/pre-re/skill_tree.yml  job inheritance               jobinheritlist.lub   JOBID + inherit lists
  db/re/skill_tree.yml      (4th/expanded jobs)           skilltreeview.lub    client skill trees
                                                          skillid.lub          SKID name -> id
        └───────────────► tools/extract_alias_facts.py ◄───skillinfolist.lub    skill -> icon file name
                                   │
                       tools/alias_facts.json  (committed, 118 KiB; provenance + every row)
                                   │
                       tools/gen_aliases.py  ──►  korangar/archive/data/client/aliases.ron
                                   │                    (39.6 KiB, sorted/unique)
                       world/library/aliases.rs (include_str! fallback + archive override)
```

`tools/extract_alias_facts.py` reuses the committed GRF reader / Lua-5.1 bytecode helpers of
`tools/extract_job_facts.py` and is deterministic (all set iteration is sorted; verified by
re-running it against the same tree/archives: byte-identical output, SHA-256
`acf7ed88…`). The facts file records:

* the server HEAD (`89c8bfa61a48310d7ebbd18992ad30b7c2631a68`, same pin as `job_facts.json`) and
  SHA-256 of every server file read,
* the client archive sizes, file counts and SHA-256 (data.grf `6dcae744…`, CondaRO.grf
  `813c7213…`, palettes.grf `a18d0033…`) and the SHA-256 + form (source/bytecode) of the five
  client Lua tables,
* per-row resolution method: `variant-monster` (138), `family` (82), `id-override` (7),
  `compact-monster` (2), `fuzzy` (2), `variant-npc` (1) for monsters; `family` (213),
  `npc-token-suffix` (20), `explicit` (4), `variant` (4) for skill icons,
* the curated tables that the algorithm uses (monster name prefixes/suffixes and families, id
  overrides, skill icon family/explicit maps), so the extractor is self-describing.

### 2.1 Job skill-tree fallbacks (56 rows)

The client skill-tree tables (`jobinheritlist.lub` `JOBID`/`JOB_INHERIT_LIST{,2}` and
`skilltreeview.lub`) plus rAthena `enum e_job` are parsed and the resolver re-computes each
server job's client skill-tree size (own slots + inherited). The fallback table is **curated
client-engine content** (the kRO client executable hard-codes these job expansions), exactly like
the P1 lane's curated player-sprite table; the extractor instead *validates* every row:

* every row's job is a real (non-sentinel) `e_job` id and currently has an empty client tree,
* the fallback target is a real job whose client tree is non-empty (56/56 pass; number of
  resolved tree skills is recorded per row),
* a job with an empty client tree but no documented fallback makes extraction fail, and a stale
  row (job now has a tree) also fails — so table drift cannot go unnoticed,
* no fallback chains.

Rows map e.g. `13 JOB_KNIGHT2 -> 7 JOB_KNIGHT`, `4080 JOB_RUNE_KNIGHT2 -> 4054 JOB_RUNE_KNIGHT`,
`4252 JOB_DRAGON_KNIGHT -> 4054`, `4304 JOB_SHINKIRO -> 25 JOB_NINJA`,
`4302 JOB_SKY_EMPEROR -> 4239`, and the cosmetic jobs (`WEDDING/XMAS/SUMMER/…`, `GANGSI`,
`DEATH_KNIGHT`, `DARK_COLLECTOR`) to `JOB_NOVICE`.

**Documented legacy quirks (preserved for parity, not silently "fixed"):** two baby 4th-class rows
keep the values shipped by the old table — `4110 JOB_BABY_ROYAL_GUARD2 -> 4097 JOB_BABY_WARLOCK`
and `4112 JOB_BABY_MECHANIC2 -> 4099 JOB_BABY_ARCH_BISHOP` (the name-based base classes are
`4102 JOB_BABY_ROYAL_GUARD` / `4100 JOB_BABY_MECHANIC`). The old generator mapped the four
`*_2` baby ids by position; changing them here would alter rendered skill trees, so the parity
test pins the current behaviour and this note records the discrepancy for a future content lane.

### 2.2 Mob-id universe and the renewal supplement

The mob universe is the rAthena `db/pre-re/mob_db.yml` (1004 ids) plus four ids
(`2158 S_HORNET`, `2159 S_GIANT_HORNET`, `2160 S_LUCIOLA_VESPA`, `2308 KO_ZANZOU`) that the
client `jobidentity.lub` and the committed alias table cover but the current pre-renewal DB no
longer lists. The extractor resolves their Aegis names from the renewal `db/re/mob_db.yml` of the
same tree (recorded in `provenance.curated.mob_db_supplement`) and refuses to run if they are
missing from both. This keeps the table at the historical 1008 mob ids (the old generator's
`/tmp/mobdb.txt` was built from the same 1008-id snapshot).

### 2.3 Skill icons

`skillinfolist.lub` rows (`SKID.X = { "file_name", … }`) are matched against the actual
`data\sprite\아이템\` file set: 988/1229 skills have an exact `.spr` (the old table's exact check
only requires `.spr`; aliases require **both** `.spr` and `.act`), and the remaining 241 rows are
resolved by action-suffix variants, npc-token matching, explicit fixes and documented family
fallbacks (Doram/pet/element-spirit skills).

---

## 3. Implementation (files)

New:

* `tools/extract_alias_facts.py` — extractor (server tree via `--ssh`/`--local`, GRF chain
  `--archive` repeatable, `--output`); fails loudly on any resolution/validation problem.
* `tools/alias_facts.json` — committed facts (118 KiB, provenance + 56/232/7/241/3 rows).
* `tools/gen_aliases.py` — renderer + `--check` drift guard; validates schema, sort order,
  id uniqueness, sprite-path folder/extension/placeholder rules, generic lists, no chained job
  fallbacks, non-empty fallback targets.
* `tools/test_gen_aliases.py` — 18 tests: legacy parity (all five tables), counts
  (56/232/7/241/3), sorted/unique, path rules, no chains, committed file up to date, determinism,
  provenance, method enums, and rejection of duplicate/unsorted/chained/bad-path/missing-generic/
  count-mismatch/unknown-schema facts.
* `tools/alias_legacy_parity.json` — the removed Rust tables, extracted from `git show
  7ce0f4cd:korangar/src/world/library/{job_alias,monster_sprite_alias,skill_icon_alias}.rs`;
  the parity oracle.
* `korangar/archive/data/client/aliases.ron` — generated runtime data file.
* `korangar/src/world/library/aliases.rs` — loader/schema/validation/API
  (`DATA_PATH = data\client\aliases.ron`, `FALLBACK = include_str!(…)`,
  `OnceLock` archive copy, `deny_unknown_fields`, sorted/unique build validation, binary-search
  lookups, `AliasesSummary`, 14 unit tests).

Rewired:

* `world/library/job_alias.rs` — `JOB_SKILL_TREE_FALLBACKS` array removed; `skill_tree_fallback`
  delegates to `aliases::job_skill_tree_fallback`; `warn_once` unchanged (log-once for unknown job
  ids is still used by job appearance/identity/skill tree/entity type).
* `world/library/monster_sprite.rs` — exact `몬스터\<identity>` → `aliases` table →
  `npc\<identity>` → first existing generic; documented fallback order unchanged.
* `world/library/skill_icon.rs` — exact `아이템\<file_name>` → `aliases` table → first existing
  generic; unchanged.
* `world/library/monster_sprite_alias.rs`, `world/library/skill_icon_alias.rs` — deleted.
* `world/library/mod.rs` — loads the alias table first in `Library::new` (the resolvers need it
  while the library is built) and re-exports the validation entry points.
* `world/mod.rs` — exports `AliasesSummary` / `validate_alias_data` / `validate_compiled_aliases`.
* `lib.rs` — `check_aliases(path)` + three CLI tests; `main.rs` — `--check-aliases [PATH]`
  (prints `aliases: 56 job skill-tree fallbacks, 232 monster sprite aliases and 241 skill icon
  aliases are valid`).

CI (`.github/workflows/quality.yml`):

* lint job "Generated content data is up to date":
  `python3 tools/gen_aliases.py --check` + `python3 tools/test_gen_aliases.py`,
* tests job: `cargo run -p korangar -- --check-aliases` (compiled-fallback schema),
* generated-data job: same offline pair + `py_compile` for the new tools.

---

## 4. Verification evidence

All commands in the lane worktree with `CARGO_TARGET_DIR=/home/phantom/cargo-target-p3-alias`.

```
$ python3 tools/extract_alias_facts.py --ssh rathena --server-root /home/phantom/rathena
tools/alias_facts.json: 56 job skill-tree fallbacks, 232 monster sprite aliases (+7 generic),
241 skill icon aliases (+3 generic)
server head 89c8bfa61a48310d7ebbd18992ad30b7c2631a68
# re-run: byte-identical (deterministic)

$ python3 tools/gen_aliases.py --check
…/korangar/archive/data/client/aliases.ron is up to date
Counts: 56 job skill-tree fallbacks, 232 monster sprite aliases (+7 generic), 241 skill icon aliases (+3 generic)

$ python3 tools/test_gen_aliases.py
Ran 18 tests … OK

$ cargo fmt --all --check                                  # clean
$ cargo clippy -p korangar --all-features -- -Dwarnings    # clean
$ cargo clippy -p korangar -- -Dwarnings                   # clean
$ cargo test -p korangar --lib
test result: ok. 509 passed; 0 failed; 12 ignored
$ cargo test -p korangar --tests
message_tables + skill_behaviour: 2 passed; 1 failed:
  ui_consistency::hardcoded_style_literals_do_not_exceed_the_token_budget
  → pre-existing red ratchet at the lane base 7ce0f4cd (29 colour literals vs budget 28,
    audit §1/§4.5, P0 lane); the concurrent P0 UI-gates merge (integration `8a0f46da`) fixes
    it. This lane does not touch `src/interface`.
$ cargo build --release                                    # ok
$ …/debug/korangar --check-aliases
aliases: 56 job skill-tree fallbacks, 232 monster sprite aliases and 241 skill icon aliases are valid
$ …/debug/korangar --check-aliases /tmp/bad.ron            # exit 1, "schema error"
```

Archive-backed verification (real CondaRO GRFs; the helper's transient `lua_files.7z` was kept for
the serial run, then removed; no committed test change):

```
$ cargo test -p korangar --lib -- --ignored --test-threads=1 \
      all_mob_ids_resolve_to_existing_sprites all_skill_ids_resolve_to_existing_icons reported_missing
test world::library::monster_sprite::tests::all_mob_ids_resolve_to_existing_sprites ... ok
test world::library::monster_sprite::tests::reported_missing_monsters_resolve_to_the_expected_aliases ... ok
test world::library::skill_icon::tests::all_skill_ids_resolve_to_existing_icons ... ok
test world::library::skill_icon::tests::reported_missing_skill_icons_resolve_to_the_expected_aliases ... ok
test result: ok. 4 passed
$ cargo test -p korangar --lib -- --ignored alias_table_entries_exist_in_archives
# both alias tables: every referenced sprite/icon exists in the archives (spr + act)
```

Data file checksums at commit time:

```
acf7ed88d089c35ddc4b17dd1f9ab4a00303b369a6af2e9b052c9cb438ab5733  tools/alias_facts.json
3f372f2c3ed9b381f106191f3bdbde355f62b6ae4c2e8aef9a8eb0f4343e9660  korangar/archive/data/client/aliases.ron
bdbb9c362faff1bd9cc8b76dce272d8bbb92163cb98b6db914af1cc52e07c8b9  tools/alias_legacy_parity.json
```

---

## 5. What did not change (behavioural parity)

* Resolution order and fallbacks are identical: exact path first, then the alias table, then the
  generic list; a malformed archive override logs a warning and uses the compiled copy; unknown
  job ids still log once via `job_alias::warn_once`.
* No gameplay/packet/state-machine change: only presentation lookups moved to data. The P2 status
  lane, the P1 job/appearance registry (`jobs.ron`), weapon/item data and interface code were not
  touched.
* The archive-backed tests assert the same representative mappings as before (e.g. `720` →
  `아이템\wl_jackfrost`, `740` → `아이템\rk_ignitionbreak`, `8414` → `아이템\mg_firebolt`, and the
  expected monster alias rows including `npc\pecopeco_egg` / `npc\4_treasure_box`).

## 6. Residual / follow-ups (not in this lane)

1. The two legacy baby-class fallbacks noted in §2.1 (`4110`, `4112`) are semantically off by one
   relative to the current job names; correcting them changes rendered skill trees and should be a
   deliberate content change with its own parity update.
2. `tools/extract_alias_facts.py` needs the server tree (ssh or `--local`) and the 3.4 GB archive
   chain, so it runs manually/like `extract_job_facts.py`; CI only needs the committed facts +
   `gen_aliases.py --check`.
3. The archive-backed `#[ignore]` library tests need the game data (same as the P1/P2 archive
   audits) and are run in the QA environment, not in the default CI job.
4. The UI token-budget test is red at HEAD (P0 lane, audit §4.5); unrelated to this lane.

## 7. Files

```
new:    tools/extract_alias_facts.py  tools/alias_facts.json
        tools/gen_aliases.py  tools/test_gen_aliases.py  tools/alias_legacy_parity.json
        korangar/archive/data/client/aliases.ron
        korangar/src/world/library/aliases.rs
modified: korangar/src/world/library/{job_alias,monster_sprite,skill_icon,mod}.rs
          korangar/src/world/mod.rs  korangar/src/lib.rs  korangar/src/main.rs
          .github/workflows/quality.yml
deleted: korangar/src/world/library/monster_sprite_alias.rs
         korangar/src/world/library/skill_icon_alias.rs
```
