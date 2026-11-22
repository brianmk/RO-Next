# Character selection redesign + paper-doll head placement + logout selection cooldown + re-entry black-screen fix

**Branch:** `feat/character-selection-redesign` (worktree `/home/phantom/korangar-char-select`)
**Base:** `integration/all-fixes-20261121` @ `189e6d26` ("ui(chat): welcome message uses RO-Next client name and version", the integration HEAD at task start)
**Commits:**
* `d8417c44` – responsive 3x3 character selection with real character previews, compact levels, paper-doll head fix, logout selection cooldown
* `2a096c14` – full map load on character re-entry after logout (black-screen fix)
* final commit – paper-doll fit scale for tall headgear, map-slot log spam fix, regression tests, this report (SHA handed with the task)

**Scope:** Rust only (client + one networking/enum addition). No game data changes except two localization RON entries (`please_wait_text` in en-US/de-DE), no server changes, no Wine. Verification used the muted lean Linux/Xvfb client only.

---

## 1. Summary of user-visible changes

| Area | Before | After |
|---|---|---|
| Slot grid | fixed 900 px window, 5 slots per row (`slot_count / 5` rows, broken for non-multiples of 5) | at most **9** characters in a **3x3** grid; character servers that report more slots (the test server reports 15) show the first nine; empty slots keep the create-character behaviour |
| Window size | fixed 900 px wide, content height fixed at 180 px/card, clipped vertically at 720p | responsive: window size derived from viewport, UI scale and the active theme's chrome; fits 1280x720, 2560x1440 and small screens at UI scales 0.75/1.0/1.25; centred on screen, clamped to a sensible min/max card size |
| Character preview | text only (name, base level, job level, map) | every occupied slot shows the **real composited character** (body, hair, headgear, hair/clothes palettes) in its idle front-facing frame, using the same animation data and composition as the world sprite and the equipment paper doll |
| Levels | separate `Base level` / `Job level` labels and values | compact **`99/70`** (base/job) at the bottom of each card, no labels |
| Name / map | name at top, map as two labelled rows | name at the top, map moved into the hover tooltip (`Name` + `Map: prontera`) |
| Switch/delete/create | context menu on right click, switch mode, create window | unchanged interactions; switch mode now only offers occupied destination cards (empty cards stay create cards and do not send an invalid switch) |
| Paper doll (Equipment) | head drawn at ~half character height (raw per-pair action offsets) | head/hair/headgear compose correctly on top of the body; tall headgear/wings are scaled down instead of being clipped by the canvas |
| Logout → quick re-select | selecting the just-played character immediately after logout could crash the client | 4 s cooldown for the just-played character, 2 s for the other characters, greyed cards with a localized `Please wait… N` countdown; rapid clicks are a no-op instead of a crash |
| Logout → wait → re-select | after ~6 s the same character stuck the client in `MapLoading` (black screen, 20 s watchdog + error/login window) | always takes the full map load path again; verified for same/other characters, short waits and double clicks (world renders Prontera normally) |

---

## 2. Character selection window

### 2.1 Files

| File | Change |
|---|---|
| `korangar/src/interface/windows/character_selection.rs` | rewritten window: responsive metrics, 3x3 grid element, new slot card element (preview/name/level/cooldown), switch-mode rules, window position helper |
| `korangar/src/state/character_previews.rs` (**new**) | per-slot async preview state (`Empty`/`Requested`/`Ready`/`Unavailable`) keyed by character id; unit tests |
| `korangar/src/state/character_selection_cooldown.rs` (**new**) | anti-crash selection cooldown state machine; unit tests |
| `korangar/src/state/character_slots.rs` | `MAX_CHARACTER_SLOTS = 9`, `visible_character_slots()`, `character_in_slot()`; unit tests |
| `korangar/src/state/mod.rs` | hidden `character_previews` and `character_selection_cooldown` fields on `ClientState` |
| `korangar/src/loaders/async/mod.rs` | shared animation load path + `LoaderId::CharacterPreview(slot)` + `request_character_preview_load()` (reuses the animation cache and the in-flight coalescing) |
| `korangar/src/loaders/diagnostics.rs` | diagnostics for the new loader id |
| `korangar/src/world/entity/mod.rs` | `character_preview_part_files()` – the same body/head/headgear part composition a spawned `Player` uses |
| `korangar/src/lib.rs` | `Client::update_character_selection_previews()`, loader completion/failure routing, `open_character_selection_window()`, cooldown gating in `SelectCharacter`, cooldown repaint, `return_to_character_selection()` / `CharacterSelected` map-state reset, re-entry fix + regression tests |
| `korangar/src/interface/windows/mod.rs` | re-export of the window position helper |
| `korangar/archive/data/languages/{en-US,de-DE}.ron`, `state/localization/mod.rs` | new localized `please_wait_text` with serde default |
| `korangar/src/renderer/interface.rs` | `CustomInstruction::Character` now draws the *composed idle frame parts* (head placement fix); `CHARACTER_FEET_ANCHOR` is public within the crate |
| `korangar/src/interface/components/paper_doll.rs` | uses the fixed composition; sprite scale now fits tall headgear into the canvas (capped at the previous 3.2); module docs/tests |

### 2.2 Responsive layout

`character_selection_layout(slot_count, viewport, scaling, chrome)` (pure function, unit-tested) computes the window and card size:

* visible cards = `min(server_slots, 9)`, at least one card; columns = `min(cards, 3)`, rows = `ceil(cards / 3)`,
* available area = `viewport / ui_scale - 2 * 18 px` (the renderer scales the whole interface, so the layout works in unscaled interface pixels),
* card width is limited by both the available width and height, clamped to `[min(104, available), 240]` interface px, with a fixed width/height ratio of 0.95 (portrait cards),
* window size = theme border + card grid + theme gaps; the height budget includes the theme's title bar, border, gaps and the log-out button row, so the whole window (title + 9 cards + log-out) always fits,
* the window is opened with `Interface::open_window_at_auto_size` at a computed centred top-left position, so a position cached by an older client version (the old fixed 900x700 grid) can never push the resized window off screen.

Measured window sizes (window crop includes 4 px inset):

| screen | UI scale | theme | window | card |
|---|---|---|---|---|
| 1280x720 | 1.0 | default | 568x684 | 161x170 |
| 1280x720 | 0.75 | default | 597x685 (crop 589x685) | ~231x243 |
| 1280x720 | 1.25 | default | 540x667 (crop 532x667) | ~114x120 |
| 2560x1440 | 1.25 | classic | 967x1059 (crop) | — |
| 1280x720 | 0.75 | classic | 619x684 (crop) | 186x196 |

The layout tests cover: 3x3 for 9 cards, server 12 slots → 9 cards, 0/1/2/4-card grids, all three supported UI scales at 1280x720 and 2560x1440, small screens (1024x600, 800x600, 640x480), growth up to the maximum on large screens, and invalid/zero scaling or viewport inputs.

### 2.3 Character previews

* When the character selection window is open, `Client::update_character_selection_previews()` requests the composed idle animation of every visible occupied slot through the existing async animation loader (`LoaderId::CharacterPreview(slot)`). The loader's cache/coalescing is shared with world entities, so the same appearance is only loaded once and a character whose sprites are already cached appears immediately.
* Results update the hidden `CharacterPreviews` state (slot + character id); a stale completion for a slot that was deleted/switched is ignored, failed loads are remembered and not retried every frame, and the previews are dropped when the window closes.
* The card draws the animation data through `Renderer::add_character` (`CustomInstruction::Character`), which draws the idle front frame of the composed animation (action 0 / frame 0) with all sprite layers. The scale is computed from the composed idle frame size so the character fits the card without covering the name/level text (`PREVIEW_HEIGHT_FRACTION = 0.66`, width fraction 0.9), and is UI-scale aware because the layout instruction applies the interface scaling.
* Empty slots show the localized `Create Character` text (as before). A failed/missing preview leaves the name/level card without a sprite instead of a placeholder.

### 2.4 Interactions

* Left click (and `DoubleLeft`) on an occupied card selects the character; right click opens the existing context menu (Delete / Switch / Cancel).
* Switch mode keeps the existing behaviour: the origin card shows `Cancel`, occupied cards show `Switch slots`; empty cards are now shown as inactive create cards instead of registering an invalid switch (the server rejects switching into an empty slot).
* Delete opens the e-mail confirmation window, the empty `Create Character` card opens the character creation window.
* Tooltip: character name + `Map: <map>` (localized `map_text`), so the map information from the old window is preserved without cluttering the compact card.
* Interactions are verified live: hover highlight/tooltip, context menu, switch mode, delete confirmation, create-character window (see §5).

### 2.5 Logout selection cooldown (anti-crash)

`CharacterSelectionCooldown` (new state module) arms when the client returns from a map session (`return_to_character_selection()`), starts when the character selection window is shown again, and remembers the character that was selected last:

* the just-played character is blocked for **4 s**, every other character for **2 s**; the first login after starting the client is never blocked (the cooldown is only armed on a map-session return),
* blocked cards are greyed out with a localized `Please wait… N` countdown (N = remaining seconds, rounded up); their select/double-click/context-menu click handlers are not registered, so a rapid click or double click is a no-op instead of a crash,
* `InputEvent::SelectCharacter` additionally re-checks the cooldown in the client before sending the select packet, covering the one-frame race between a layout pass and a click,
* the character selection window is repainted while the cooldown runs (countdown) and for a short grace period after it expired, so the cards become selectable again,
* the cooldown is cleared when the player logs out to the login screen (fresh session) and the state machine is unit-tested (first login, same/other timers, re-arming per return, repaint window, clearing).

Live check: logout → click the same character (blocked, greyed countdown) → click another character (blocked) → other character selectable after 2 s → select → logout → same character blocked → selectable after 4 s; see §5.

---

## 3. Paper-doll head placement fix (Equipment window)

**Root cause:** `CustomInstruction::Character` used `AnimationPair::actions.render_sprite_frame(..., action 0, frame 0)` for every sprite pair. Those are the *raw* ACT clips of each part, i.e. the head/hair/headgear offsets are relative to their own sprite origin. The RO client positions those parts with the body/head attach points; the animation loader already resolves the attach points when it builds the composed `AnimationFrame`, but the interface instruction bypassed that data. The result: the head was drawn around half of the character height instead of on top of the body (and the effect depended on the job/hair/headgear).

**Fix (`renderer/interface.rs`):** `CustomInstruction::Character` now draws the **composed idle frame** (`AnimationData::animations[0].frames[0].frame_parts`), i.e. exactly the data the world renderer uses: each part carries its attach-point corrected offset and final size. Both the part size and offset are multiplied by the requested scale × interface scaling, and the parts are anchored at the feet anchor (0.80 of the area), so body/hair/headgear compose correctly for every job/sex/hair/headgear/palette and all UI scales. The equipment paper doll and the character-selection preview use this same instruction.

Additionally, the paper doll scale is now computed from the composed idle frame height so tall headgear (e.g. Angel Wing) is scaled down to stay inside the canvas instead of being clipped at the top; normal characters keep the previous 3.2 scale (`CHARACTER_MAX_SCALE = 3.2`, top margin 6 px). The old scale was a constant and tall headgear was cut off.

Evidence: equipment window paper dolls for male/female fixtures across both themes and 0.75/1.0/1.25 (see §5); the character-selection previews show the same composition.

---

## 4. Logout → re-select black-screen fix

**Root cause (confirmed with `-vvv` + temporary network/server logging):** `NetworkEvent::ChangeMap` contains a same-map "teleport" fast path (fly wing, `AL_TELEPORT`, …) selected only by `self.current_map_name.as_deref() == Some(map_name)`. `return_to_character_selection()` clears `self.map` but not `current_map_name`, and after a logout the character selection window shows again while the client has no loaded (character) map. Selecting a character that lives on the same map as the menu background (Prontera) then arrives as `ChangeMap{map_name: "prontera"}` with `session = MapLoading` and `self.map = None` (or the menu background map). The fast path executed, cleared the entities and bailed out at `let Some(map) = self.map.as_mut() else { continue; };` without ever loading the map. The session stayed in `MapLoading`, the 20 s map-entry watchdog fired, and the client aborted to the login screen / black screen. The first selection after login starts with `current_map_name = None`, which is why only the re-entry path was affected; when the menu background map was still loading or already loaded the fast path bailed at the same place (black frame vs. stuck loading depended on the timing).

**Fix (`lib.rs`):**

* new pure guard `is_same_map_teleport(current_map_name, incoming_map_name, has_loaded_map, session_state)`; the fast path only runs when the map name matches **and** a map is loaded **and** the session is `SessionState::InGame`,
* `current_map_name` is cleared in `return_to_character_selection()` and on a fresh `CharacterSelected` map entry, so the next map entry always takes the full load path,
* regression tests: `tests::fresh_character_map_entry_is_not_a_same_map_teleport` (stale `MapLoading` + same name + loaded/missing map) and `tests::in_game_same_map_move_is_a_teleport` (real teleport still fast-paths; different map / missing map / not in game never do).

Live reproduction before the fix (`-vvv`, 1280x720, scale 1.0): select QaPreviewA (Prontera) → menu Logout → wait 6 s → select QaPreviewA → `CharacterSelectionWindow` stays for 20 s, then `map entry watchdog expired` → `ErrorWindow` + `LoginWindow`. After the fix the same sequence shows `CharacterOverviewWindow` within 2 s and renders Prontera normally; see §5 for the same/other/short/double-click matrix.

---

## 5. Live verification

Environment: Linux/Xvfb (muted, `KORANGAR_MUTE_AUDIO=1`, null ALSA device), lavapipe software Vulkan, 30 FPS cap, lean graphics profile, release client built from the worktree with its own `CARGO_TARGET_DIR`. Two throw-away test accounts created in the server DB for this task (`charselqa` male, `charselqa2` female, each with 3/2 fixture characters, 12 character slots reported) were used; no user characters were touched and the fixtures/accounts are removed after the run. User characters, the Lead-owned main tree and the user's client were never used.

### 5.1 Character selection matrix (both resolutions × all scales × both themes)

Captured with the redesigned window, 3 occupied + 6 empty slots, different jobs/headgear/hair/palettes and level combinations (`99/70`, `70/45`, `1/1`):

* `1280x720` / `2560x1440` × UI scale `0.75`, `1.0`, `1.25` × theme `default`, `classic` → 12 crops in `notes/korangar-character-selection-evidence/` (`select-<theme>-<scale>-<screen>.png`),
* interaction crops from the 1280x720/1.0/default run: hover + tooltip, empty-slot hover, context menu, switch mode, delete confirmation, create-character window,
* female fixture run (different sex + Angel Wing/Clown Nose headgear): `select-female-1.0-default.png`.

All windows fit fully on the screen (no clipped log-out button/cards), are horizontally and vertically centred, and show the theme-specific chrome (dark default theme, classic skin with classic cards). The test server reports **15** character slots; the window shows the first nine (the intended clamp) — visible in the client log: `character selection: server reports 15 slots, showing the first 9`.

### 5.2 Paper doll (Equipment window) after the head fix

`--select` runs enter the game with the first fixture character, open the Equipment window (`Alt+Q`) and capture the window: `paperdoll/paperdoll-<theme>-<scale>.png` for 0.75/1.0/1.25 × default/classic, plus female fixtures (`paperdoll-female-<theme>.png`). Body/hair/headgear compose with the head on top of the body, feet at the bottom, no blank parts; tall headgear/wings are scaled to stay inside the canvas (the old fixed 3.2 scale clipped them at the top). The world sprite and the paper doll use the same composed idle frame; the in-world captures (`relogin/…-in-world.png`) show the same character.

Headgear coverage note: the character-selection previews exercise the headgear composition directly with the fixtures' appearance data (Wedding Veil + Glasses, Bunny Band + Bandit Beard, Wizard Hat, Angel Wing + Clown Nose on a female character) and use exactly the same `CustomInstruction::Character` path as the paper doll. The in-game paper doll of the fixture characters can show fewer headgear layers than the character list because the map server reports the own-character appearance from the equipped inventory; the fixture accounts have no equipped items, so adding real headgear there would need server-side inventory changes. The composition fix itself is shared and is verified through the paper doll's body/hair/body-part placement and through the previews.

### 5.3 Logout cooldown

`--logout-check` run (same client, 1280x720/1.0/default): select → in-game → menu Logout → immediately click the same character and another one: client stays on the character selection screen, both cards are greyed with the countdown (`cooldown_blocked_cards.png`); the other character becomes selectable after 2 s (enters the game, `cooldown_other_character_after_2s`), the same character is still blocked and becomes selectable after 4 s (`cooldown_same_character_after_4s`). No crash, no selection packet while blocked (client log shows the debug rejection).

### 5.4 Re-entry matrix (black-screen regression)

`--relogin-check` runs at 1280x720/1.0/default: select → in-game → menu Logout → wait → re-select, sampling the client every 2-10 s. All runs enter the world (`CharacterOverviewWindow`) within the first sample and stay in the world:

| run | result |
|---|---|
| same character, 6 s wait | in game after 2 s, world rendered (`relogin_after_6s.full.png`) |
| other character, 6 s wait | in game, world rendered |
| same character, 4.5 s wait (cooldown expired) | in game |
| other character, 2.5 s wait (cooldown expired) | in game |
| same character, 6 s wait, double click | in game |

Before the fix the same-character/6 s run reproduced the stuck `MapLoading` (20 s watchdog, no `CharacterOverviewWindow`); the regression tests pin the state-machine condition.

---

## 6. CI parity

Commands are run in the worktree with its own `CARGO_TARGET_DIR`, nightly toolchain pinned by `rust-toolchain.toml`, `PATH` containing the Slang compiler:

| Check | Result |
|---|---|
| `cargo fmt --all --check` | touched files formatted; the base HEAD already has one pre-existing fmt drift in `korangar/src/interface/windows/hotbar.rs` (untouched, left as is), so the workspace-wide check reports only that file |
| `rustfmt --check` on all touched files | clean |
| `cargo clippy -p korangar -- -Dwarnings` | clean (exit 0) |
| `cargo test -p korangar --lib` | **403 passed, 0 failed, 11 ignored** |
| `cargo build --release` | clean |
| `cargo fmt --all --check` on the base HEAD | pre-existing drift in `hotbar.rs` only (same as before this branch) |

New unit tests (all pass):

* `state::character_slots::tests::at_most_nine_slots_are_visible`
* `state::character_previews::tests::*` (ready/request/stale/failed/clear semantics)
* `state::character_selection_cooldown::tests::*` (first login, same/other timers, one start per return, re-arm, repaint, clear)
* `interface::windows::character_selection::tests::*` (3x3 grid, 9-card clamp, fits 720p/1440p at 0.75/1.0/1.25, small screens, grow-to-max, card geometry/aspect, invalid scaling/viewport)
* `interface::components::paper_doll::tests::*` (max scale for normal frames, tall headgear fits, helper/frame fallback)
* `tests::fresh_character_map_entry_is_not_a_same_map_teleport`, `tests::in_game_same_map_move_is_a_teleport`

---

## 7. Limitations and notes

* **Server slot counts > 9** are intentionally hidden (requirement); the character server may report more (15 in the test environment, 12 for the fixture account) but only the first nine cards are shown and creatable.
* **Map name** moved from the card body to the hover tooltip to keep the card compact (name top / level bottom); the tooltip keeps the localized `Map` label.
* **Drag-to-switch** is not implemented: switching is done through the existing right-click → Switch → destination-card flow (as before). The requirement allowed keeping the supported path.
* Both single click (existing behaviour) and double click select a character; the cooldown decides whether a click is accepted.
* The window is deliberately re-centred every time it is shown (open at computed position, auto size) so a position/size cached by older client versions cannot off-screen the new responsive window; the cached position is not preserved for this menu window.
* The golden screenshot baseline (`korangar/test-data/golden`) contains `character_select` crops that intentionally changed; that optional/non-blocking harness needs `scripts/ui-golden.sh --update-baseline` (or a dedicated review of the new character-select crops) before it is re-run. The equipment/paper-doll baselines change for the same reason (head placement fix). The main CI (`fmt`/`clippy`/`tests`/`release`) does not depend on the golden matrix.
* **Windows / packaging:** the change is Rust-only with two localization RON updates, so the normal RO-Next Windows rebuild/packaging pipeline is enough (rebuild the release client and ship the repo files; no game data/patcher changes). The next RO-Next Windows build should include these commits.
* The pre-existing fmt drift in `hotbar.rs` and the `--all-targets` clippy issues in untouched test code were not introduced by this branch and were left untouched to keep the merge diff clean.
* A second logout in the *same* client session exposed a pre-existing char-server reconnect race (the client can land on the login window instead of the character list). It is unrelated to this branch and did not occur in the black-screen/re-entry matrix; the cooldown check is split into one-logout runs for that reason. Worth a separate follow-up.
* Paper-doll fixtures were created by editing the test characters' appearance columns; the map server normalises the own-character appearance on login, so headgear layers of the *in-game* paper doll depend on actually equipped items (the character-selection previews and the shared renderer verify the composition).

---

## 8. Evidence

Evidence directory: `/home/phantom/Games/ragnarok/CondaRO-linux/notes/korangar-character-selection-evidence/` (shared notes directory; this report itself is committed in the branch under `notes/`). Overview montages: `overview/select-matrix-1280.png`, `overview/select-matrix-2560.png`, `overview/paperdoll-matrix.png`, `overview/cooldown.png`, `overview/relogin.png`. Contents:

* `select-<theme>-<scale>-<screen>.png` – character selection window crops (12),
* `select-female-1.0-default.png`, `select-hover-tooltip.png`, `select-context-menu.png`, `select-switch-mode.png`, `select-delete-confirmation.png`, `select-create-character.png`,
* `paperdoll-<theme>-<scale>.png`, `paperdoll-female-*.png` – Equipment paper doll after the fix,
* `cooldown_blocked_cards.png`, `cooldown_other_character_after_2s.png`, `cooldown_same_character_after_4s.png` – selection cooldown,
* `relogin-after-logout.png`, `relogin-after-wait.png`, `relogin-after-6s-full.png` – re-entry after the black-screen fix,
* `logs/relogin-before-fix-samples.txt` (`in_game=False` on every sample, then `ErrorWindow`+`LoginWindow`) and `logs/relogin-after-fix-samples.txt` (`in_game=True` from the first 2 s sample) – driver samples of the same sequence,
* `logs/relogin-after-fix-session.log` – `-vvv` session excerpt of the second entry: `CharSelect -> MapLoading -> MapLoading -> InGame` (previously the map change was swallowed and the watchdog fired after 20 s).
