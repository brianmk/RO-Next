# Data-driven weapon centralization

Owner: `weapon-centralize` (subagent lane).
Worktree: `/home/phantom/korangar-weapon-central`
Branch: **`feat/weapon-centralization`**, base integration HEAD **`935c3ecd`**
(`integration/all-fixes-20261121`).
Own build dir: `CARGO_TARGET_DIR=/home/phantom/korangar-weapon-central-target`.

Commits:

* **`f85b1433a44edc788c4603bf032da88c36203039`** — implementation + generated data +
  facts/generators + tests + in-client evidence.
* report commit (this file) on top of it.

## 1. What was scattered before

| Concern | Old location (base `935c3ecd`) | Old form |
|---|---|---|
| Weapon type → sprite suffix | `world/entity/mod.rs` | `weapon_suffix_candidates()` `match` arms `1..22` → `["단검"]`, `["양손검","검"]`, `["활"]`, … |
| Candidate paths | `world/entity/mod.rs` | `player_weapon_part_candidates()` built `인간족\{job}\{job}_{sex}_{numeric/suffix}` |
| Bowman detection | `lib.rs` | `WEAPON_TYPE_BOW = 11`, `BOW_ITEM_VIEW_RANGE = 1700..=1799`, `is_bow_weapon()` |
| Arrow assets | `lib.rs` | `const CANDIDATES: [&str; 4] = ["아이템\\화살", …]`, `arrow_assets_failed` flag |
| Arrow flight time | `world/particles/mod.rs` | `ArrowProjectile::FLIGHT_SECONDS = 0.28` |
| Weapon animation load | `lib.rs` ×5 | duplicated candidate/request/apply blocks in char login, NPC spawn, job change, hair change and the retry queue |
| Weapon layer gate | `world/entity/mod.rs` | inline "first 8 animations, any drawable frame" check |
| Equipment look | `korangar-networking` | `ZC_SPRITE_CHANGE` types `Weapon`/`Shield`/`Robe` were dropped (`_ => None`) |

## 2. Design: one module, one generated data file

### `korangar/src/world/weapon.rs` (new)

Single source of truth for weapon handling:

* `profile(weapon) -> &WeaponProfile` —
  `WeaponKind` (30 kinds, including the dual-wield display kinds), ordered
  classic sprite-family variants, authoritative rAthena attack range,
  ammo kind, visual projectile kind.
* `part_candidates(job_id, sex, weapon) -> Vec<EntityPartFile>` — candidate
  paths built **from data** (path templates + fallback chain), no match arms:
  * server weapon type (`W_*`, e.g. `W_BOW = 11`): per-job variant → classic
    `인간족\무기` layer,
  * item view id (population shells / custom servers): per-job numeric view id
    → per-job sprite family → classic numeric → classic family,
  * unknown numeric value: numeric per-job + classic attempts,
  * `weapon == 0` / unused `MAX_WEAPON_TYPE` marker: no candidates.
  Path templates use `{job_stem}` (from the existing job sprite metadata
  `get_sprite_path_for_player_job`, including job aliases), `{sex}`, `{variant}`.
* `projectile_for_weapon(weapon) -> Option<&ProjectileProfile>` and
  `projectile(kind)` — sprite candidates (preferred item sprites + last-resort
  effect sprites) and flight time.
* `animation_is_drawable(&AnimationData)` — shared gate for the weapon layer
  (placeholder-only ACTs are rejected instead of drawing nothing forever).
* `load(&GameFileLoader)` — loads `data\client\weapons.ron` from the client
  archive, compiled `include_str!` fallback, malformed override → warn + fallback.

### `korangar/archive/data/client/weapons.ron` (new, generated)

Schema v1 (see the file header):

* `paths`/`sex_suffix`/`fallbacks` — candidate path templates and per value-class
  fallback order,
* `server_types` — rAthena `enum weapon_type` value → kind (30 values;
  `W_DOUBLE_*` = 25…30 map to combined dual-wield kinds, `MAX_WEAPON_TYPE` 24
  is deliberately not a weapon),
* `kinds` — per kind: ordered sprite-family variants, rAthena `SubType`,
  attack `range` (most common item `Range`), `ammo` and `projectile`,
* `item_ranges` — the 707 pre-renewal weapon item ids as **47 disjoint,
  ascending ranges** → kind,
* `item_views` — explicit rAthena `View:` overrides (empty for this database;
  kept for custom servers),
* `projectiles` — `Arrow`: 10 verified arrow item sprites + 2 last-resort
  effect sprites, `flight_ms: 280`.

The file is embedded with `include_str!` and override-able at runtime through
the normal client file loader, like the skill data files.

## 3. Provenance + generator pipeline

Authoritative facts are committed, so generation and `--check` run offline:

1. `tools/extract_weapon_items.py` (needs the server tree once) reads
   `db/pre-re/item_db_equip.yml` (`Type: Weapon`; 707 items / 22 subtypes /
   `Range` / `View` / `WeaponLevel`) and the `enum weapon_type` +
   `enum e_ammo_type` from `src/map/pc.hpp`, and writes
   `tools/rathena_pre_re_weapon_items.json`
   (`item_db_sha256 = d6604ac7…`, server HEAD `8767176…` at extraction).
2. `tools/extract_weapon_client_assets.py` scans the CondaRO GRF file tables
   (`data.grf` 149,167 / `CondaRO.grf` 34,565 / `palettes.grf` 498,263 files)
   and writes `tools/weapon_client_assets.json`: 5,228 per-job weapon variant
   files → 245 sprite families (151 numeric item ids, 94 classic names), 26
   projectile/ammo item sprites, and the documented absences:
   * `classic_weapon_directory.present = false` — **zero**
     `data\sprite\인간족\무기\*` files in this data set,
   * `weapon_info_table_present = false` — no `weapongrp.lub`.
3. `tools/gen_weapons.py` writes `korangar/archive/data/client/weapons.ron`
   and has a `--check` drift guard. It validates that every configured sprite
   family actually exists in the archive facts, that item ranges are disjoint
   and exactly cover the 707 weapon ids, and that arrow assets exist.
4. `tools/test_gen_weapons.py` — 12 invariant tests over the facts, generator
   output and `--check`.

Regenerate:

```
./tools/extract_weapon_items.py --item-db <rathena>/db/pre-re/item_db_equip.yml \
    --pc-hpp <rathena>/src/map/pc.hpp --server-head <sha>
./tools/extract_weapon_client_assets.py
./tools/gen_weapons.py
./tools/gen_weapons.py --check
./tools/test_gen_weapons.py
```

## 4. Callers routed through the module

* `world/entity/mod.rs`:
  * `Common::get_weapon_part_candidates()` → `weapon::part_candidates`;
  * `Common::set_weapon_animation()` → `weapon::animation_is_drawable`;
  * `EntityData.weapon` / `Common.weapon` docs corrected: server `W_*` type (or
    item view id for shells/custom servers), not "view id" only;
  * `apply_sprite_change()` now takes the packet's `u32` value and returns
    `SpriteChangeOutcome { body, weapon }`; handles `Weapon` (updates the weapon
    value + drops the layer for reload), `Shield`, `Robe`.
* `lib.rs`:
  * `request_weapon_animation(&async_loader, &mut entity)` replaces the four
    duplicated load blocks (character login, NPC spawn, job change, hair change)
    and logs the data-driven profile (`kind`, range, ammo, candidate count);
  * arrow spawn uses `weapon::projectile_for_weapon()`; the hard-coded bow type,
    item-view range, asset list and failed flag are gone;
  * `ensure_projectile_assets(kind)` loads sprite candidates and flight time
    from the data file, cached per projectile kind;
  * `AppearanceChanged`/`LocalAppearanceChanged` reload only the layers the
    outcome says changed (equipping a weapon no longer reloads the body).
* `loaders/async/mod.rs`: weapon layer doc points at the registry; the loader
  still owns the async/caching mechanics and accepts the candidate list.
* `world/particles/mod.rs`: `ArrowProjectile::new(..., flight_seconds)` — flight
  time is data now; particle tests updated.
* `korangar-networking`: `ZC_SPRITE_CHANGE` (`0x01D7`) now forwards
  `Weapon`/`Shield`/`Robe` as `NetworkEvent::AppearanceChanged` (previously
  dropped), so weapon appearance changes reach the entity layer.

## 5. Behaviour notes (intentional, documented)

* **Dual wield** (`W_DOUBLE_DD/SS/AA/DS/DA/SA`, 25…30) previously fell through
  the `match` (`_ => &[]`) and produced no weapon layer. They now resolve to the
  combined archive variants (`단검_단검`, `검_검`, `단검_검`, …) with the
  single-weapon variant as fallback.
* **Instruments/whips** (`W_MUSICAL`, `W_WHIP`) use the server's arrow ammo
  (rAthena `pc_equipitem` ammo check), so they now spawn the arrow projectile
  like bows. Guns keep `ammo: Bullet` but no visual projectile: this data set
  has no verified flying bullet sprite (documented, no invented asset).
* **Sprite families** come from the archive-verified list only; unverified
  guesses from the old table (`한손검`, `스태프`, `철퇴`, `리볼버`, `소총`,
  `개틀링`, `수류탄`, `후우마`) were replaced by names that actually exist
  (`클럽/해머/스패너`, `롯드/캔디케인롯드`, `권총/handgun2`, `라이플`,
  `기관총/gun_gatling2`, `샷건`, `수리검`).
* **Attack action selection** is intentionally not per-weapon data: the RO ACT
  layout places the basic attack motion at the same action index for every
  weapon (the weapon ACT mirrors the body ACT), so only the projectile is
  weapon-dependent; this is documented in the module docs. `ActionEvent::Attack`
  (weapon attack sound) remains a separate TODO.
* **Item/equipment display** is unchanged: the equipment/inventory UI already
  renders weapon item icons from `iteminfo.lub`; no world-space icon fallback is
  invented.

## 6. Asset limitation kept honest

The CondaRO client archives have **no drawable weapon graphics**:

* zero files under the classic `data\sprite\인간족\무기\*` layer and no
  `weapongrp.lub` (GRF file-table scan),
* the per-job variants that do exist (`5,228` files) have ACT clips that are all
  hidden placeholders (`sprite_number == -1`, previous audit
  `notes/weapon-falcon-evidence/weapon-act-asset-audit.txt`).

The registry still attempts every candidate (so a complete data set renders
weapons without code changes), the loader skips missing files, and
`weapon::animation_is_drawable` rejects placeholder-only animations. Result: no
weapon layer, never a black box, no panic; the async loader caches failed candidate sets so
the archives are not re-scanned per retry/hit. No item/skill icon is drawn in the world as a
substitute.

## 7. Verification

### Unit / data tests

* `cargo test -p korangar --lib` → **446 passed, 0 failed** (includes 11 new
  `world::weapon` tests: server-type mapping for all 30 values, item view id
  ranges, per-job/classic fallback order, numeric-first item views, unknown
  numeric fallback, dual-wield variants, arrow/gun projectile & ammo, attack
  ranges, `--check` data validity, plus a synthetic **visible fake fixture**
  (`AnimationFramePart { sprite_number: 0 }`) vs placeholder-only fixture for the
  layer gate).
* `cargo test -p korangar-networking` → 59 passed; `cargo test -p ragnarok-packets`
  → 33 passed.
* `./tools/test_gen_weapons.py` → 12 passed (facts completeness, enum values,
  generator/`--check` parity, disjoint/complete id ranges, ammo/projectile
  rules, archive-verified sprite families, RON escaping).

### CI parity

* `cargo fmt --all --check` → OK.
* `cargo clippy --workspace -- -Dwarnings` → OK.
* `cargo clippy --workspace --all-features -- -Dwarnings` → OK.
* `cargo build --release` → OK.
* Python facts/generator tests need no extra packages; `gen_weapons.py --check`
  and `test_gen_weapons.py` were added to `.github/workflows/quality.yml`.
* Known **pre-existing** red test on the base integration HEAD: `cargo test -p
  korangar` also runs `tests/ui_consistency.rs`
  (`hardcoded_style_literals_do_not_exceed_the_token_budget`: 29 color literals
  vs budget 28). This scan only reads `korangar/src/interface` (untouched by this
  branch — diff has zero files there), so it is not caused by this work; the
  test passes only when the budget is bumped or the extra literal removed. All
  other test targets pass.

### In-client (muted Linux/Xvfb, GM test char only — no Wine, no user chars)

Lean runtime via `scripts/run-korangar-test.sh` (Xvfb `:90`, 1280x720, 30 fps,
audio muted, `KORANGAR_LOG_LEVEL=2`), test account `verify4` / `VerifyBot4`
(DB state backed up before and **restored afterwards**: class 0, level 1, no
items, GM group unchanged).

Evidence (`notes/korangar-weapon-centralization-evidence/`):

```
DEBUG korangar::world::weapon] weapons: loaded 30 kinds from data\client\weapons.ron
DEBUG korangar] weapon layer: entity 95000018 kind Bow (Bow) range 5 ammo Some(Arrow) -> 4 candidate(s)
DEBUG korangar] weapon layer: entity 95000011 kind Dagger (Dagger) range 1 ammo None -> 4 candidate(s)
DEBUG korangar] weapon layer: entity 95000107 kind Staff (Staff) range 1 ammo None -> 6 candidate(s)
DEBUG korangar] weapon layer: entity 95000123 kind Mace (Mace) range 1 ammo None -> 8 candidate(s)
DEBUG korangar] weapon layer: entity 95000029 kind Katar (Katar) range 1 ammo None -> 4 candidate(s)
DEBUG korangar] weapon layer: entity 95000023 kind OneHandedSword (1hSword) range 1 ammo None -> 4 candidate(s)
DEBUG korangar] weapon layer: entity 95000043 kind Staff (Staff) range 1 ammo None -> 6 candidate(s)
DEBUG korangar::loaders::r#async] weapon animation not in the archives, disabling the weapon layer for `인간족\스나이퍼\스나이퍼_여_1701`
```

* the generated data file is loaded through the archive override at runtime,
* 9 different weapon kinds observed resolving live (server types and item view
  ids, e.g. Bow from view id 1701 → `range 5`, `ammo Some(Arrow)`, 4 candidates;
  Staff/Mace/Katar from item view ids → 6/8/4 candidates; unarmed → 0),
* missing/placeholder candidates are skipped gracefully (no black sprite, no
  panic; log count 0 panic reports),
* screenshot `in-game-archer-weapon-layer.png` shows the test Archer in
  Prontera with the client running normally.

Not captured: a bow arrow flight frame series. The local player's `weapon`
value is only set by the server's `ZC_SPRITE_CHANGE` weapon look (character
info carries 0) and in this lean runtime the inventory window did not display
the DB/`@item`-added bow, so the test char could not actually attack with a bow
in-game. The arrow spawn path is covered by the module/data tests and compiles
into the `DamageEffect` handler; the previously documented
`notes/korangar-arrow-projectile-report.md` had the same automation blocker.
No user characters or the user's client were touched.

## 8. Follow-ups

1. Once drawable weapon assets exist (full `data.grf` or classic `인간족\무기`
   layer), re-run the in-client weapon-layer check; no code change should be
   needed — the candidates are already attempted.
2. Add a weapon attack sound to the profile when the ammunition for
   `ActionEvent::Attack` exists (weapon attack sounds are still a TODO in the
   entity animation event).
3. Optional: use the equipped ammo item sprite for the arrow projectile instead
   of the fixed arrow item sprite list (needs an ammo item id in the protocol).
4. Shield/robe layers can reuse the same `EquipmentLook` plumbing that weapon
   appearance now uses.
