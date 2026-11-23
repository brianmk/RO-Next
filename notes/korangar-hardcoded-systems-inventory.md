# RO-Next hardcoded-systems inventory (current)

**Revision:** 2026-11-22, integration branch after the P0–P3 migration and the
post-migration gameplay fixes (HEAD `6cbd1e77` at the time of writing).
**Scope:** current state of the client hardcoded-system remediation described in
`notes/client-hardcoded-systems-audit.md`; that report remains the detailed
line-level audit of the starting tree, this file is the up-to-date inventory and
status summary.
**Mode:** read-only inventory (no game session, no Wine, no user characters).

## 1. Data-driven runtime tables (archive override + compiled fallback)

`korangar/archive/data/client/` now contains **18** RON tables, all loaded through
`GameFileLoader` and covered by a generator `--check` and/or a unit test:

| table | content |
| --- | --- |
| `skill_effects.ron` | 437 phases / 608 layers of skill visuals (asset, anchor, timing, tint, blend, wav) |
| `skill_actions.ron` | caster animation actions per skill |
| `skill_sounds.ron` | 8 curated overrides + 363 official skill rows + generic fallback |
| `skill_effect_assets.ron` | effect asset paths |
| `skill_sprite_effects.ron` | sprite-based effects |
| `skill_procedural_effects.ron` | procedural effect geometry |
| `skills.ron` | 3 behaviour rows / cloud rows |
| `status_effects.ron` | 609 statuses: 519 names, 575 icons, 7 value labels, 3 auras |
| `jobs.ron` | 181 job rows, palettes, 1365 headgear view ids |
| `weapons.ron` | 30 kinds, 47 id ranges, 707 weapon ids, arrow/projectile |
| `item_stats.ron` | 6169 item stat rows |
| `aliases.ron` | 56 job skill-tree fallbacks, 232 monster sprites, 241 skill icons |
| `messages.ron` | 81 protocol result-code rows in 9 domains |
| `keybinds.ron` | 39 bindings |
| `paper_doll_slots.ron` | 11 equipment slots |
| `world_config.ron` | map/camera/quest-icon/client config |
| `ui_audio.ron` | interface sound paths |
| `debug_maps.ron` | debug map list |

Generators: **16** `tools/gen_*.py` tools with `--check`; 545 library tests and
the `--check-*` CLI gates are green. The interface token ratchet is at **0/0**
(colors and `FontSize`).

## 2. Resolved remediation phases

* **P0** — UI style-token and localization gates; protocol result-code message
  tables (`korangar-messages`); localized UI/paper-doll/dropdown text.
* **P1** — weapons (`weapons.ron`/`world/weapon.rs`), jobs/appearance
  (`jobs.ron`), item stats (`item_stats.ron`), hair/palette/headgear data.
* **P2** — status-effect data and auras (`status_effects.ron`); skill behaviour
  flags (`skills.ron`); no runtime skill-id chain for visuals.
* **P3** — aliases, world/audio config, keybinds/paper-doll slots, name limits,
  UI architecture and localization lint.

## 3. Remaining hardcoded content (known, triaged)

Measured with grep on `korangar/src` at this revision:

* Asset-path string literals in Rust: **68** (down from 607 at the audit start;
  most remaining are loader format prefixes, fallback files, or procedural
  effect helpers).
* Runtime skill-id branches in `lib.rs`: **3** (protocol/semantic cases, not
  content tables).
* Runtime status-id branches in `lib.rs`: **2** (the red status tints for
  Adrenaline Rush/Fury and the cart status fallback; both are documented
  protocol/visual semantics rather than content tables).
* UI text literals in `src/interface`: **0**.
* `item_stats.rs` still parses Korean field labels/grammar markers in item
  descriptions; translated server descriptions can produce empty stat panels.
  This is the largest remaining content-hardcode known.
* Job/appearance and headgear tables are data-driven, but some view-id gaps
  remain documented (161 missing headgear view ids; 16 missing jobs have both
  sexes).
* Procedural effect geometry (warp cylinder, spirit-sphere orbit, status aura
  ring sizes) is intentionally code; textures/colors/timings live in data.

## 4. Verification

* `cargo test -p korangar --lib`: 545 passed, 0 failed, 12 ignored.
* `./scripts/ui-token-budget.sh`: colors 0/0, FontSize 0/0.
* All `tools/gen_*.py --check` gates: up to date.
* Palindromic deployment: RO-Next 0.2.12 packaged, Windows payload manifest
  regenerated; `RO-Next.exe` sha256 `90de7af5edf2aec2679a242715f7c99ab566b3104d54cfc4a13a99bffd0505d6`.
