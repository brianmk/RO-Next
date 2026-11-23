# P0 — UI design-token and localization gates (RO-Next / korangar)

Status: **done, gates green**. Branch `p0/ui-gates-20261122` (base `f454127e`, the
integration HEAD when the lane started). Main tree stays Lead-owned; the work was
done in `/home/phantom/korangar-p0-gates` with its own target directory.

## Result at a glance

| Gate | Before | After |
| --- | --- | --- |
| `./scripts/ui-token-budget.sh` | **exit 1**: colors 29/28, `FontSize` 16/12 | **exit 0**: colors 0/0, `FontSize` 0/0 (budget *lowered*, never raised) |
| `cargo test -p korangar -p korangar-interface` | 446 tests | 447 tests, all green (new token/localization tests) |
| `cargo test -p korangar --test ui_consistency` | 3 tests | 3 tests, all green (lint now forbids 100 hardcoded UI phrases) |
| localization keys | 132 per language | **222** per language (`UiTexts` 123, `PaperDollTexts` 22, `DropdownTexts` 77) |
| interface hardcoded style literals | 29 colors + 16 `FontSize` | **0** in `korangar/src/interface/**` |
| visual behaviour | — | **24/24 golden screens pixel-identical** (default+classic × 0.75/1.0 scales), see §3 |
| `cargo fmt --all -- --check`, clippy (`-D warnings`), `cargo build --release` | — | green |

Nothing in `lib.rs` (chat feedback and protocol-result messages) was touched —
that is the separate P0 lane called out in the audit.

## 1. Design-token migration (`state/theme/tokens.rs`)

All 29 `Color::(rgb|rgba|monochrome)_u8` literals and all 16 `FontSize(..)`
sites in `korangar/src/interface/**` were migrated. The token budget was then
lowered from `28/12` to `0/0` with `scripts/ui-token-budget.sh --update` (the
script refuses raises; the audit's `818ab3e1` 23→28 bump is what made the gate
red in the first place).

### New semantic color tokens (default theme; classic inherits via `..default()`)

The replaced literals were theme-independent in the old code (they were drawn on
their own dark surfaces/overlays in *both* themes), so the new tokens have the
same value in the default and classic theme. That is enforced by
`migrated_overlay_tokens_are_theme_independent` and the exact legacy values are
pinned by `migrated_tokens_keep_the_legacy_values`.

| Token | Value | Replaced in |
| --- | --- | --- |
| `minimap_background` | `rgb(22,24,28)` | `components/minimap.rs` |
| `minimap_marker_fallback` | `rgb(80,255,110)` | `components/minimap.rs` |
| `minimap_marker_outline` | `rgba(0,0,0,200)` | `components/minimap.rs` (player + server markers) |
| `spirit_sphere_empty` | `rgba(24,34,46,190)` | `windows/spirit_spheres.rs` |
| `party_health_fill` | `rgb(84,196,104)` | `windows/party.rs` |
| `overlay_secondary_text` | `monochrome(200)` | minimap loading hint, frame inspector |
| `overlay_highlight` | `rgba(255,255,255,100)` | frame inspector hover block |
| `profiler_bar_hot` | `rgba(255,80,0,150)` | profiler hot frame bar |
| `text_hint` | `rgb(160,160,160)` | skill description tip |
| `chat_ghost_text` | `rgba(205,205,205,230)` | chat input ghost text |
| `item_slot_background` | `rgba(28,28,34,140)` | item info card slots |
| `item_slot_border` | `rgba(120,120,130,120)` | item info card slots |
| `item_slot_inner` | `rgba(0,0,0,70)` | item info card slots |
| `item_slot_unavailable_background` | `rgba(86,86,92,70)` | item info unavailable card slot |
| `item_slot_unavailable_border` | `rgba(120,120,126,60)` | item info unavailable card slot |
| `item_slot_unavailable_text` | `rgba(160,160,166,90)` | item info unavailable card slot |
| `item_slot_card_name` | `rgba(235,235,240,220)` | item info card name |

Literals that already had an exact existing token value were mapped to it
instead of adding a new token: `text_accent`, `text_disabled`, `border`,
`state_hover`, `state_active`, `surface_raised`, `text_overlay`,
`chat_input_focused_background`, `slot_*`, `canvas_background`, `refine_*`.

Typography sites were mapped to existing typography tokens (exact pixel sizes
kept): `font_xs` 9, `font_body` 12, `font_button` 13, `font_body_lg` 14,
`font_subtitle` 15, `font_title` 18.

### Typed helpers for genuinely dynamic values

Dynamic/data-driven style values are no longer written as raw constructors; they
have named, documented constructors:

* `Color::from_rgba_bytes([u8; 4])` / `Color::from_rgb_bytes([u8; 3])`
  (`graphics/color.rs`) for server/map-data colors: minimap marker colors
  (`state::minimap::MinimapMarker::color`) and the server supplied chat message
  color (`MessageColor::Rgb`) now go through these.
* `FontSize::from_style(f32)` (`loaders/font/mod.rs`) for progress-bar style
  values that are already part of a theme style.
* `FontSize::dynamic(f32)` for sizes computed from a component dimension
  (round button text `circular_size * 0.62`).
* `FontSize::scaled_by(f32)` for a token font size scaled by a component factor
  (item box favourite marker / amount text).
* `FontSize::clamped_to(computed, minimum)` for token-bounded responsive sizes
  (character selection card title/body/level fonts).

Migrated windows/components: `item_info`, `minimap`, `party`,
`spirit_spheres`, `skill_description`, `chat`, `character_selection`,
`progress_bar`, `item_box`, `round_button`, `zeny`, and the debug-only
`frame_inspector`/`profiler` (the token ratchet scans them too, so they use
`OverlayStyle` built from tokens).

`scripts/ui-token-budget.sh` counted with `grep | wc -l` under `set -o pipefail`,
so a fully migrated interface (zero matches) made the script fail before printing
anything. That is fixed: the two counts tolerate `grep`'s "no match" exit code
and the script now reports `colors=0 FontSize=0` and exits 0.

## 2. Localization sweep

### New `state/localization` groups (serde defaults, old files keep loading)

`Localization` gained three nested groups, each `#[serde(default = ...)]` with an
English `Default` impl, so a language file written before the group existed
loads and gets English fallbacks (test:
`language_files_without_new_text_groups_still_load`):

| Group | Fields | Content |
| --- | --- | --- |
| `ui` (`UiTexts`) | 123 | the 68 audited UI strings plus item/skill/equipment row labels, context-menu action labels and tooltips, disabled-tooltips, formatted templates (friend request, party invitation, level/sphere line, cart total, zeny unknown) and the debug-menu button texts |
| `paper_doll` (`PaperDollTexts`) | 22 | 11 equipment slot long names + 11 short labels (`Head top`/`TOP`, …) with `slot_name`/`slot_label` helpers |
| `dropdown` (`DropdownTexts`) | 77 | every settings drop-down option label (resolution, FPS, sampler, MSAA/SSAA/AA, shadow method/detail/resolution, aura style, UI scaling, lighting mode) |

`en-US.ron` and `de-DE.ron` were both updated (222 keys each, German
translations included; the key-parity test stays green). The pre-existing 132
flat keys are unchanged. `dropped` labels are not in the files; the group default
is the English fallback.

### Drop-down labels without changing the `DropDownItem` trait signature

Drop-down items live in `graphics/settings.rs`, `settings/interface.rs`,
`settings/graphic.rs` and `loaders/font/mod.rs` and only implement
`DropDownItem::text() -> &str`, which has no access to the state. The interface
crate therefore gained a small, generic resolution hook:

* `DropDownItem::text_key()` (default `None`) returns a stable key from
  `state::localization::dropdown_keys` (77 constants).
* `Application::localized_text(state, key)` (default `None`) resolves a key to
  the localized label of the current language; `ClientState` implements it via
  the new `state::localization::LOCALIZATION_PATH` (a `'static` path, so the
  borrowed string has the state lifetime and no allocation happens per frame).
* The drop-down component renders
  `text_key().and_then(localized_text).unwrap_or(text())`; applications without
  a localization (or unknown keys) keep the old English `text()` fallback.

No option list, setting value or behaviour changed; only the rendered label of
an option is looked up.

### Interface strings

All 68 audited non-debug literals plus the remaining user-facing literals found
by a broader scan (`label:`, `name:`, `disabled_tooltip:`, formatted texts) were
replaced with localization paths. The only quoted user-facing strings left in
non-debug `interface/**` are language-neutral stat abbreviations (`STR`, `AGI`,
… in the item/equipment bonus lines) and the em-dash placeholder.

The source-scan lint (`korangar/tests/ui_consistency.rs`) now forbids 100
hardcoded UI phrases (was 18) covering the migrated window titles, buttons,
labels, tooltips and the debug-menu entries. The token-budget test in the same
file now enforces `0` literals.

### Tests

* `every_dropdown_key_resolves_to_a_label` — all 77 `DropdownTexts::KEYS` resolve
  to a non-empty, and unknown keys return `None` (fallback path).
* `every_paper_doll_slot_has_a_name_and_a_label` — all 11 `EquipPosition`
  values the paper doll draws are covered.
* `text_group_defaults_are_complete` — group defaults serialize without empty
  strings; the groups keep their `StateElement` implementation for the debug
  state inspector.
* `language_files_have_the_same_keys`, `old_language_files_without_new_fields_still_load`
  (kept) and the new group back-compat test above.
* `migrated_tokens_keep_the_legacy_values`,
  `migrated_overlay_tokens_are_theme_independent` in `theme/tokens.rs`.

## 3. Appearance preservation — before/after screenshots

Two release binaries were built: `f454127e` (before) and the P0 branch (after),
same test character (`verify4`/`VerifyBot4`, live CondaRO server), same
lean/muted profile. The existing golden harness
(`scripts/ui-golden.sh`) was driven headless on Xvfb (lavapipe software Vulkan)
for **theme {default, classic} × scaling {0.75, 1.0}**, 6 screens each: login,
server select, character selection, inventory (item boxes), equipment (paper
doll + equipment stats panel), character overview (progress bars/HUD).

Result: **24/24 screens are pixel-identical** — `changed = 0` pixels, RMSE 0.0,
identical window rectangles. (`/tmp/compare_golden.py` output, window crops with
the harness' animation masks applied; the before run was re-captured once
because a stale equipment window overlapped the overview crop.)

The comparison summary is committed in
`notes/korangar-p0-ui-gates-evidence/golden-compare.txt`, together with
before/after crops of the equipment window (paper doll + equipment stats panel)
and the classic-theme character selection window; the two files of each pair are
byte-identical. An extra local driver variant for the always-open chat overlay
and the minimap was started but stopped at the Lead's request (the pixel-perfect
matrix above already proves the token migration changed no pixels); extending
the committed golden driver with those screens remains a follow-up.

The windows named in the task that need in-game state which the headless driver
cannot set up (item info needs a right-click on an item, party needs a second
member, spirit spheres need a Champion with active spheres, skill description
needs a right-click in the skill tree, frame inspector/profiler need a debug
build plus menu clicks) are not part of this screenshot matrix. Their migration
is a pure substitution of color/font *values* that are asserted byte-identical
by `migrated_tokens_keep_the_legacy_values`; no layout, spacing, visibility or
text changed (the diff contains no layout edits). Extending the golden driver
with those windows is a follow-up for the non-blocking harness.

Harness fixes found while doing this (small, independent of the migration):

* `scripts/ui_golden_driver.py` also matches the X11 `WM_CLASS` (contains
  `korangar`) when looking for the client window, so a display-name change
  (`Korangar` → `RO-Next`) no longer silently breaks the harness.
* `scripts/ui-golden.sh` accepts `UI_GOLDEN_WINDOW_NAME` and passes it to the
  driver.

Note: the committed golden baseline in `korangar/test-data/golden` predates
several integration merges (character-selection redesign, skill data, …), so the
harness' *compare* against the committed baseline is stale at the integration
HEAD independently of this lane. It was not updated here (that would encode
other lanes' UI changes in this commit); the before/after evidence was produced
with a fresh before-baseline from `f454127e`.

## 4. Verification commands and evidence

```
./scripts/ui-token-budget.sh           # exit 0, colors=0 FontSize=0
cargo fmt --all -- --check             # clean
cargo clippy -p korangar -p korangar-interface -- -D warnings   # clean
cargo test -p korangar -p korangar-interface                   # 447 + 3 tests green
cargo build --release -p korangar      # green
```

Screenshot evidence lives outside the repository in
`/home/phantom/p0-golden-before` (f454127e) and
`/home/phantom/p0-golden-after-final` (this branch); the comparison summary and
representative before/after crops are committed under
`notes/korangar-p0-ui-gates-evidence/` (the comparison script is included so the
result can be re-produced).

## 5. Files touched (high level)

* `korangar-interface`: `application.rs` (`Application::localized_text`),
  `components/drop_down.rs` (`text_key` + localized rendering helper).
* `korangar/src/state/theme/tokens.rs`: 17 new semantic color tokens + tests.
* `korangar/src/state/localization/texts.rs` (new): the three text groups,
  drop-down key constants, `template_text` selector helper, tests.
* `korangar/src/state/localization/mod.rs`: groups on `Localization`,
  serde defaults, `LOCALIZATION_PATH`, accessors, back-compat tests.
* `korangar/src/graphics/color.rs`, `korangar/src/graphics/settings.rs`,
  `korangar/src/settings/{interface,graphic}.rs`,
  `korangar/src/loaders/font/mod.rs`: byte-color/font helpers and drop-down
  keys.
* `korangar/src/interface/**`: 25 windows/components migrated to tokens and
  localization (incl. debug-only frame inspector/profiler).
* `korangar/archive/data/languages/{en-US,de-DE}.ron`: +90 keys each.
* `korangar/test-data/ui-token-budget.ron`: budget 28/12 → 0/0.
* `scripts/ui-token-budget.sh`: zero-literal handling.
* `scripts/ui-golden.sh`, `scripts/ui_golden_driver.py`: harness robustness.

Commit: `p0/ui-gates-20261122` (SHA in the hand-off message; the branch head
is the integration point).

## 6. Remaining/follow-up (out of this lane)

* `lib.rs` chat-feedback strings and protocol result-code → message mappings
  (separate P0 lane; untouched here by request).
* `msgstringtable`-backed item/skill text and the 26 login failure mappings.
* Stat abbreviations and the `—` placeholder were left language-neutral.
* The committed golden baseline needs a refresh from a fresh `f454127e`+ capture
  matrix, and the golden driver could grow scenarios for item info, party,
  spirit spheres, skill description and the debug windows.
* Main tree advanced to `6ed5d6d1` (`p1/jobs-appearance-data`) after this lane
  started; the branch is based on `f454127e` and should be rebased/merged by the
  Lead (no conflict-prone files overlap the weapon/job lanes).
* Scratch files for the extra minimap/chat captures were not committed; the
  extra screen coverage is a follow-up.
