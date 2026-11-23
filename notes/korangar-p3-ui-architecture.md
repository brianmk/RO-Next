# P3 — UI architecture cleanup (keybindings, paper-doll slots, protocol name lengths, localization lint)

Status: **implemented, gates green, branch committed**. Branch `p3/ui-architecture`,
base integration HEAD `8a0f46da` (`Merge branch 'p0/ui-gates-20261122' ...`, which includes the P0
UI token/localization work and the data-driven job/item/weapon/status/skill tables). Work was done
in `/home/phantom/Games/ragnarok/CondaRO-linux/korangar-p3-ui-arch` with its own target directory;
the main tree stayed Lead-owned. No Wine, no user characters; the only game-facing run was a
headless Xvfb/lavapipe golden-harness attempt against the live CondaRO server (see §6).

This is the Phase 5 item of `notes/client-hardcoded-systems-audit.md` (§8, P2/P3).

## Result at a glance

| Item | Before | After |
| --- | --- | --- |
| key bindings | ~20 hardcoded key checks in `input/mod.rs` | `archive/data/client/keybinds.ron` (16 shortcuts + 9 emotion + 10 hotbar + 2 tab + 2 confirm keys), loader with `include_str!` fallback + archive override + `deny_unknown_fields`, `korangar --check-keybinds [PATH]`, input handling wired through it, 5+3 unit tests |
| paper-doll slots | 11 slot definitions + canvas/scale constants in `interface/components/paper_doll.rs` | `archive/data/client/paper_doll_slots.ron` (11 slots + canvas geometry + character fitting), loader + `korangar --check-paper-doll [PATH]`, component reads the table, 5 unit tests + 3 CLI tests |
| duplicated name lengths | `4`/`24` in 4 windows + `lib.rs` + networking buffers | one documented module `ragnarok-packets/src/limits.rs` (`NAME_LENGTH`, `MIN_NAME_LENGTH`, `PASSWORD_LENGTH`, `USER_ID_LENGTH`), used by windows, `lib.rs`, `korangar-networking` and the packet structs; **protocol values unchanged** |
| localization lint | forbidden-word scan of `src/interface` only | scan of the whole client source (localization source + debug-only files exempt) **plus** a `fn text(&self)` / `fn text_key(&self)` check that also verifies every drop-down fallback literal exists in `state/localization/texts.rs`; detection proven with injected probes |
| `./scripts/ui-token-budget.sh` | 0/0 | **0/0, exit 0** |
| tests | — | 527 default-feature + 539 all-feature lib tests green, integration tests green (§7) |
| golden UI matrix | committed baseline stale at HEAD (pre-existing) | before/after harness run started; see §6 for the exact state |

## 1. Key binding table (`data\client\keybinds.ron`)

**Data file** `korangar/archive/data/client/keybinds.ron` — shipped in the client archive, embedded
with `include_str!` as the compiled fallback:

```ron
(
    provenance: "Korangar default key bindings, moved out of korangar/src/input/mod.rs ...",
    version: 1,
    shortcuts: [
        (action: CloseOrMenu, key: "Escape"),
        (action: ToggleInventoryWindow, key: "KeyE", modifiers: [Alt]),
        ...
        (action: CycleMapWindows, key: "KeyM", exact_modifiers: true),
        (action: ToggleInventoryWindow, key: "KeyI", exact_modifiers: true),
    ],
    emotion_keys: ["Digit1", ... "Digit9"],
    hotbar_keys: ["F1", ... "F10"],
    hotbar_tab_previous: "F11",
    hotbar_tab_next: "F12",
    chat_command_keys: ["Enter", "NumpadEnter"],
)
```

The defaults are **exactly** the bindings that were hardcoded before, in the same evaluation order,
so event order and conflicts are unchanged:

| keys | action | conflict reason |
| --- | --- | --- |
| `Escape` | close top window / open menu | conditional on `has_foreground_windows()` (unchanged) |
| `Alt+E/S/A/Z/Q/C` | inventory / skill tree / stats / friend list / equipment / chat | required modifiers are a subset, so `Ctrl+Alt+E` still opens the inventory like before |
| `Ctrl+S/I/G/A/H/M/Q` | settings windows, interface toggle, minimap, close window | `Ctrl+I` stays settings, `Ctrl+M` stays minimap |
| `M` / `I` | cycle map windows / inventory | `exact_modifiers: true`: only fire with **no** modifier held (classic client behaviour; a text input keeps the keys) |
| `Alt+1..9` | emotions 0..8 | position in the table is the emotion number (same `SendEmotion` payload as before) |
| `F1..F10` | hotbar slots 0..9 | modifiers are ignored, like before; press = `UseSkillInSlot`, release = `StopSkill` |
| `F11` / `F12` | previous / next hotkey tab | |
| `Enter` / `NumpadEnter` | confirm a focused text input | login form, server selection and (via the physical key) chat-command focus |

**Loader** `korangar/src/input/keybinds.rs`:
* `DATA_PATH = "data\\client\\keybinds.ron"`, `FALLBACK = include_str!("../../archive/data/client/keybinds.ron")`,
* `OnceLock` archive copy that wins over the compiled table; a malformed override logs a warning and
  falls back to the compiled table instead of losing keyboard control,
* `#[serde(deny_unknown_fields)]` on every data struct; unknown actions, unknown key names and
  duplicate combinations are schema errors,
* schema validation: non-empty shortcut list, exactly one `CloseOrMenu`, no duplicate
  (key, modifiers, exact) combination, 1..=9 emotion keys, 1..=10 unique hotbar keys, distinct tab
  keys, non-empty unique confirm keys,
* key names are the winit `KeyCode` variant names (`KeyE`, `Digit1`, `F1`, `NumpadEnter`, ...); the
  complete name table is generated in `korangar/src/input/key_names.rs` (194 keys), so a server
  override can remap to **any** physical key without a curated whitelist. I did not enable winit's
  optional `serde` feature: the name table keeps the dependency graph and the debug build size
  unchanged.
* load point: `Client::init` right after the protocol message table (`time_phase!("load keybinds")`);
  unit tests use the compiled fallback (`table()` returns `compiled()` when no archive override was
  loaded), so input behaviour is testable without game files.

**CLI** `korangar --check-keybinds [PATH]` → `korangar::check_keybinds`; without a path the compiled
fallback is validated, with a path an archive override. Exits 0/1 and prints the schema error.

**Wiring**: `InputSystem::handle_keyboard_input` now iterates the data table
(`keybinds::shortcuts()`), then the emotion/hotbar/tab lists and exposes
`chat_command_key_pressed()` for the `Enter`/`NumpadEnter` confirm keys used by `lib.rs` (login form,
server selection, chat-command focus). Debug-only camera/debug-window keys stay in the `debug`
feature code: they are developer shortcuts, not user bindings.

## 2. Paper-doll slot table (`data\client\paper_doll_slots.ron`)

**Data file** `korangar/archive/data/client/paper_doll_slots.ron` — 11 equipment slots in drawing
order plus the canvas/character geometry that used to live in the same file:

```ron
(
    provenance: "...", version: 1,
    canvas_width: 220.0, canvas_height: 342.0, slot_size: 42.0,
    character_max_scale: 3.2, character_top_margin: 6.0,
    slots: [
        (kind: HeadTop, left: 89.0, top: 4.0),
        (kind: Ammo, left: 166.0, top: 4.0),
        ... 11 entries ...
        (kind: Shoes, left: 89.0, top: 296.0),
    ],
)
```

* `kind` is the semantic slot kind and maps 1:1 to the protocol `EquipPosition` bitflag
  (`HeadTop` → `HEAD_TOP`, `RightAccessory` → `RIGTH_ACCESSORY` keeping the wire spelling, ...).
  That position selects the equipped item **and** the localized slot name/short label from
  `state::localization::PaperDollTexts` — i.e. the P0 localization keys are reused, not duplicated.
* The item icon shown in a slot is the equipped item's texture (there is **no** per-slot icon asset
  in this client; empty slots render their localized short label, exactly as before), so the table
  does not invent an unused icon field. This is the one part of the audit wording ("positions,
  icons, localized labels, slot kinds") that has no counterpart in the current implementation:
  P0 already moved the 13 name/label strings into `PaperDollTexts` and the icon is item data.
* The defaults are the former hardcoded values; `paper_doll_slots::tests::defaults_match_the_former_hardcoded_layout`
  pins all 11 positions + all five geometry constants, so the default/classic themes keep the exact
  layout at any interface scaling (0.75/1.0/1.25).

**Loader** `korangar/src/interface/components/paper_doll_slots.rs`: same pattern as the other data
systems (`include_str!` fallback, `OnceLock` archive override, `deny_unknown_fields`, schema errors,
`load()` from `Client::init`, `validate`/`validate_compiled`). Validation rejects a wrong slot count
(must be the 11 classic equipment positions), duplicate kinds, non-finite/negative geometry,
out-of-canvas slots and invalid character fitting. `korangar --check-paper-doll [PATH]` is the CLI.

**Wiring**: `interface/components/paper_doll.rs` no longer contains the `SLOTS` array,
`CANVAS_WIDTH/HEIGHT`, `SLOT_SIZE`, `CHARACTER_MAX_SCALE` or `CHARACTER_TOP_MARGIN`; it reads
`paper_doll_slots::table()` for the canvas, slot rectangles and the character fitting. `SLOT_COUNT`
documents the protocol shape (11 equipment positions) and comes from the table module.

## 3. Protocol name-length constants (`ragnarok-packets/src/limits.rs`)

One documented module, values unchanged:

| constant | value | wire shape |
| --- | --- | --- |
| `NAME_LENGTH` | 24 | fixed `name[24]` fields (`#[length(24)]` strings, `[u8; 24]` arrays): character, party, friend, account names |
| `MIN_NAME_LENGTH` | 4 | minimum character name length enforced by the character server |
| `PASSWORD_LENGTH` | 24 | login packet 0x0064 password string |
| `USER_ID_LENGTH` | 24 | 24-byte login user id (`UserId`) |

Re-exported at the crate root (`ragnarok_packets::NAME_LENGTH`, ...). Call sites switched over:
`windows/character_creation.rs`, `windows/friend_list.rs` (the "duplicated throughout the code base"
TODO is gone), `windows/login.rs`, `windows/party.rs`, the friend-add length check in `lib.rs`, the
`ZC_SOUND` / `SKILLDATA` / party-list test buffers in `korangar-networking/src/lib.rs`, plus
`UserId` and the deprecated login-success name array in `ragnarok-packets`.
`#[length(24)]` attribute arguments are left as literals because the packet derive macro interprets
a bare identifier as a *field name* (`self.<ident>`) — the limits module doc explains this; the
values are unchanged, which the new unit test pins. Chat-message (80) and account-e-mail (39) limits
stay local: they are single-site and not name fields.

## 4. Localization lint extension (`korangar/tests/ui_consistency.rs`)

Two new source-scan tests on top of the existing three:
1. `no_hardcoded_ui_words_outside_the_interface` — the existing forbidden-word list is now scanned
   across the whole client source, not only `src/interface`. Exempt: the localization source
   (`state/localization/`, which *defines* the literals), debug-only windows, and two data files
   whose literals are not UI labels (`input/key_names.rs` — physical key names such as `"Delete"`;
   `world/library/item_stats_fallback.rs` — Korean/English item-description field matching).
2. `drop_down_labels_are_localized` — every `fn text(&self)` that returns string literals must be
   paired with a `fn text_key(&self)` in the same `impl` (a stable localization key), and every
   fallback literal must exist in `state/localization/texts.rs`, so the untranslated fallback and
   the localized label cannot drift apart. Dynamic labels (e.g. the server display name) and the
   native language names in the language selector are allowed.
Both tests blank `#[cfg(test)]`-gated items while keeping line numbers (many files keep production
code after their test module) and were verified to **fail** on injected probes (a keyless
`fn text` literal and a `"Trade"` literal in `graphics/buffer.rs`) and pass after removing them.

`./scripts/ui-token-budget.sh` remains **0/0** (colors and `FontSize` literals are still gone).

## 5. New/changed files

New: `korangar/archive/data/client/keybinds.ron`,
`korangar/archive/data/client/paper_doll_slots.ron`, `korangar/src/input/keybinds.rs`,
`korangar/src/input/key_names.rs`, `korangar/src/interface/components/paper_doll_slots.rs`,
`ragnarok-packets/src/limits.rs`.
Changed: `input/mod.rs` (data-driven handling + `chat_command_key_pressed` + tests),
`interface/components/{mod,paper_doll}.rs`, `interface/windows/{character_creation,friend_list,login,party}.rs`,
`lib.rs` (loaders, `check_keybinds`, `check_paper_doll`, confirm-key wiring, tests), `main.rs`
(`--check-keybinds`, `--check-paper-doll`), `korangar-networking/src/lib.rs`,
`ragnarok-packets/src/lib.rs`, `tests/ui_consistency.rs`.

## 6. Visual verification — exact state

`scripts/ui-golden.sh` (headless Xvfb + lavapipe software Vulkan, muted client, live CondaRO server
`100.40.206.179:6900`, test account `verify5`) was used for the paper-doll appearance check:
a release binary of the **base commit** (`/tmp/korangar-before`) and of this branch
(`/tmp/korangar-after`) were built for a before/after matrix (default+classic × 0.75/1.0/1.25,
equipment window included).

The first capture attempt failed before any screenshot: the client panicked at
`Client::init` with `RequestDeviceError Core(Device(Lost))` because the harness was given the
lavapipe `.so` instead of its ICD JSON (`UI_GOLDEN_VK_ICD`), so the Vulkan device was lost. The
single-variant retry with the harness' auto-detected lavapipe path (`:137`, `default`/`0.75`,
`/tmp/golden-test2`) was still running when the Lead asked to stop exhaustive golden capture for
integration; it is **not** part of the committed evidence. The layout is instead pinned by
`defaults_match_the_former_hardcoded_layout` (all positions/geometry byte-equal to the former
Rust constants), and the refactor changes **no** drawing code: the same `Area`s are fed to the same
layout calls in the same order, so the before/after matrix is expected to be pixel-identical.
**Remaining work for a follow-up lane:** run
`UI_GOLDEN_BIN=/tmp/korangar-before scripts/ui-golden.sh --update-baseline --baseline /tmp/golden-baseline ...`
then the same capture with `UI_GOLDEN_BIN=/tmp/korangar-after` and the same `--baseline` to produce
the pixel-diff evidence (the committed baseline in `korangar/test-data/golden` is already stale at
the integration HEAD for unrelated reasons, as documented by the P0 lane).

## 7. Gates and evidence

```
./scripts/ui-token-budget.sh                                   # exit 0: colors=0 (budget 0), FontSize=0 (budget 0)
cargo fmt --all -- --check                                     # clean
cargo clippy --workspace -- -Dwarnings                         # clean
cargo clippy --workspace --all-features -- -Dwarnings          # clean
cargo test -p korangar -p korangar-interface                   # 527 lib (12 ignored) + 10 interface + 5 ui_consistency
                                                               #   + 4 message_tables + 7 skill_behaviour, all green
cargo test -p korangar --all-features --lib                    # 539 passed, 0 failed, 12 ignored
cargo test -p ragnarok-packets -p korangar-networking          # 25 + 66 + 10 passed, green
cargo build --release --features "unicode,debug"               # green
target/release/korangar --check-keybinds                       # "keybinds: 39 key bindings are valid", exit 0
target/release/korangar --check-paper-doll                     # "paper doll: 11 equipment slots are valid", exit 0
target/release/korangar --check-keybinds /tmp/bad-keybinds.ron # exit 1 with the RON schema error
target/release/korangar --check-keybinds <unknown-key file>    # exit 1: "unknown key name `Bogus` ..."
target/release/korangar --check-paper-doll <bad slots file>    # exit 1 with the RON schema error
```

Environment note: `cargo test --all-features --no-run` for the **binary test target** fails to link
in this lane's target directory with `rust-lld: relocation R_X86_64_32 out of range` in the DWARF
`.debug_str` of dependency archives (the all-feature debug build's `.debug_str` exceeds the 4 GiB
DWARF-32 limit in this copied target tree; the dependency archives are the pre-existing ones, this
lane's code adds no such data). The lane therefore ran the tests with the default-feature
`cargo test -p korangar -p korangar-interface` (the same command the P0 lane used) **and** the
all-feature lib tests (`--lib`) so the `debug`-feature code paths of the new keybinding tests are
also covered. `fmt`, both clippy configurations and the release build are green.

No Wine, no user characters and no game sessions were used; the only game-facing run was the
headless Xvfb capture attempt described above.
