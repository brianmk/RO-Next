# Korangar client — Phase 6 (P3): data-driven world / audio / client feedback

Owner: `p3-world-audio` (subagent lane)
Branch: **`p3/world-audio-data`**, worktree
`/home/phantom/Games/ragnarok/CondaRO-linux/korangar-p3-world-audio`
Commit: the single lane commit at the branch tip (this report is part of it); see
`git log -1 --format=%H p3/world-audio-data`
Base/integration HEAD: **`7ce0f4cd`** (`feat(skills): data-driven skill behaviour and procedural
effect data` — Phase 4) with the merged weapon registry, jobs/appearance, item stats, status
effects, message tables and the P0 localization fields already in.
Own build target: `/home/phantom/korangar-p3-world-audio-target` (**not** the Lead-owned main tree
target). The main worktree was never built or modified.
No Wine, no user characters, no game session; the client binary is only used for the `--check-*`
data CLI checks (and one muted Xvfb start-up smoke test, see §8).
Reference: `notes/client-hardcoded-systems-audit.md` §4.8 (maps/audio/quest icon),
§4.4 + §4.9 (client chat feedback), §8 Phase 6.

---

## 1. Result

Phase 6 is implemented. The debug map warp list, the login/default map + start-camera focus, the
quest-icon path/colours, the three UI audio paths and the remaining client chat feedback strings
are now data/configuration instead of Rust constants or literals:

| before (`7ce0f4cd`) | after |
| --- | --- |
| `interface/windows/maps.rs:19–44`: `const MAP_COUNT: usize = 23` + `MAP_WARPS: [(&str, TilePosition); 23]` + `std::array::from_fn` buttons | `archive/data/client/debug_maps.ron` + `client_config::debug_maps` (dynamic boxed button list, archive override, compiled fallback, `--check-debug-maps`); window stays `#[cfg(feature = "debug")]` |
| `lib.rs:193`: `const DEFAULT_MAP: &str = "geffen"` | `world_config.ron` → `client_config::default_map()` |
| `lib.rs:194`: `const START_CAMERA_FOCUS_POINT: Point3<f32> = Point3::new(600.0, 0.0, 240.0)` | `world_config.ron` → `client_config::start_camera_focus()` |
| `lib.rs:203`: `const UI_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3")` | `ui_audio.ron` → `client_config::background_music()` (`None` = silent menus) |
| `lib.rs:204`: `const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "버튼소리.wav"` | `ui_audio.ron` → `client_config::main_menu_click_sound()`; `Option<SoundEffectKey>` + no-op when unset |
| `lib.rs:207`: `const TELEPORT_VANISH_SOUND_EFFECT: &str = "effect\\ef_teleportation.wav"` | `ui_audio.ron` → `client_config::teleport_vanish_sound()`; `Option<SoundEffectKey>` + no-op when unset |
| `world/particles/mod.rs:354–362`: `format!("유저인터페이스\\minimap\\quest_{}_{}.bmp", effect_id, 1)` + `match QuestColor { … Color::rgb_u8(…) }` | `world_config.ron` → `client_config::quest_icon_texture(effect_id)` + `client_config::quest_icon_color(quest_color)`; a missing texture now skips the icon (logged) instead of panicking |
| `lib.rs`: 33 client feedback literal sites — 23 `push_chat_information` calls (18 unique texts), `Gained …` experience line + 4 labels, `You got …`, `Failed to sell items`, and 7 error-window strings | 32 new templates in `state/localization` + `en-US.ron`/`de-DE.ron` (25 chat + 7 error window), reused existing `not_in_party_text`, rendered through `korangar_messages::render_template` (shared placeholder renderer, **no** duplicate result-code table) |
| no data file / loader / CLI / tests for these systems | 3 data files + `client_config` loader module + `--check-world` / `--check-ui-audio` / `--check-debug-maps` + `korangar/tests/world_audio_data.rs` + CI parity |

The data files are

    korangar/archive/data/client/world_config.ron    (login map, start camera, quest icon path/colours)
    korangar/archive/data/client/ui_audio.ron        (menu music, menu click, teleport-vanish sound)
    korangar/archive/data/client/debug_maps.ron      (23 debug warp rows, debug-only convenience)

each loaded through `GameFileLoader` (`data\client\<file>`, so a server can override it inside its
own archive without a rebuild) and embedded with `include_str!` as the compiled fallback, exactly
like `skills.ron` / `jobs.ron` / `messages.ron`.

**No behaviour change:** all data values are the previous constants verbatim (asserted by the
tests), the English chat texts are byte-for-byte the previous literals (asserted by the tests) and
the fallbacks are strict-but-safe (bad override → warning + compiled fallback).

---

## 2. Data files, loaders and CLI

New module `korangar/src/client_config/` (`mod.rs` + `world_config.rs` + `ui_audio.rs` +
`debug_maps.rs`). Every file follows the established loader contract:

* path inside the archive (`DATA_PATH`, `data\client\…`), loaded once at start-up through
  `GameFileLoader` so an archive override wins,
* `include_str!("../../archive/data/client/<file>.ron")` as the compiled fallback,
* `deny_unknown_fields` plus explicit validation (see §2.4),
* a malformed override is logged (`log_warn!`) and the compiled fallback is used; the process
  never starts without the data,
* `korangar --check-<system> [PATH]` validates the compiled fallback (no path) or an override file.

### 2.1 `archive/data/client/world_config.ron`

```ron
(
    provenance: "hand-curated client defaults: lib.rs DEFAULT_MAP/START_CAMERA_FOCUS_POINT, world/particles/mod.rs quest icon path/colours; no game data extraction",
    version: 1,
    default_map: "geffen",
    start_camera_focus: (600.0, 0.0, 240.0),
    quest_icon: (
        texture: "유저인터페이스\\minimap\\quest_{effect}_{frame}.bmp",
        frame: 1,
        colors: [
            (color: Yellow, rgb: (200, 200, 30)),
            (color: Orange, rgb: (200, 100, 30)),
            (color: Green, rgb: (30, 200, 30)),
            (color: Purple, rgb: (200, 30, 200)),
        ],
    ),
)
```

Loader: `client_config::world_config` — `default_map()`, `start_camera_focus()`,
`quest_icon_texture(effect_id)` (substitutes `{effect}` and the configured `{frame}`),
`quest_icon_color(QuestColor)`.

### 2.2 `archive/data/client/ui_audio.ron`

```ron
(
    provenance: "hand-curated client defaults: lib.rs UI_BACKGROUND_MUSIC/MAIN_MENU_CLICK_SOUND_EFFECT/TELEPORT_VANISH_SOUND_EFFECT; no game data extraction",
    version: 1,
    ui_background_music: Some("bgm\\01.mp3"),
    main_menu_click_sound: "버튼소리.wav",
    teleport_vanish_sound: "effect\\ef_teleportation.wav",
)
```

Loader: `client_config::ui_audio` — `background_music() -> Option<&'static str>`,
`main_menu_click_sound() -> Option<&'static str>`, `teleport_vanish_sound() -> Option<&'static str>`.
Path resolution (archives, `data/mp3`, case-insensitive loose files) is unchanged and shared with
the map music; only the paths are data now.

### 2.3 `archive/data/client/debug_maps.ron`

23 rows of the previous `MAP_WARPS` array (`(map_name, x, y)`, data-file order preserved), e.g.
`(map_name: "geffen", x: 119, y: 59)` … `(map_name: "mid_camp", x: 180, y: 240)`.
Loader: `client_config::debug_maps` — `warps() -> &'static [DebugWarp]` with
`map_name()`, `position()`, `map_file_name()` (`"geffen.gat"`).

The Maps window (`interface/windows/maps.rs`) is a **debug-only** window
(`#[cfg(feature = "debug")] mod maps;`, listed in `ui_consistency::DEBUG_ONLY_FILES`); this lane
keeps it that way: the module is only compiled with the `debug` feature, the runtime warp registry
is only loaded (and the re-export only exists) under `debug`, and the file documents that it is a
debug convenience list, not user-facing content (hence no localization keys). Because the list can
be overridden with any number of rows, the previous fixed `[Button; 23]` array became a dynamic
boxed child list (`WarpList`, same pattern as `FriendList`/`ItemList`) so an archive override with
more or fewer maps still renders.

### 2.4 Validation (loud on typos, safe on bad overrides)

| file | rejected |
| --- | --- |
| `world_config.ron` | unknown fields; empty/non-map-name `default_map`; non-finite camera components; quest texture template without `{effect}`/`{frame}` or without a `.bmp`/`.tga` extension; `frame == 0`; duplicate quest colour; missing `QuestColor` row |
| `ui_audio.ron` | unknown fields; empty paths; paths not ending in `.mp3`/`.wav`/`.flac` (the extensions `korangar-audio` can decode); `ui_background_music: None` is valid (silent menus) |
| `debug_maps.ron` | unknown fields; empty list; invalid/duplicate map names; zero coordinates |

Each loader also has an `emergency()` last-resort configuration for the only-in-theory case that
the compiled `include_str!` file itself is unreadable (a build/packaging bug): world config keeps
the previous `geffen`/camera/quest values, UI audio becomes silent, debug maps become empty. The
committed files are validated by the tests, so this path is never taken in a shipped build.

### 2.5 Client start-up wiring

`Client::init` now calls `crate::client_config::load(game_file_loader.as_ref())` (a
`time_phase!("load client config")` block right after the protocol message-table load), before the
audio engine, the default-map load and the start camera use the values.

---

## 3. Client chat feedback → localization

The audit's §4.9 / P0 note left the client's own feedback strings out of the protocol result-code
table ("*out of scope … the other client feedback strings*"). They are client-side validation
messages, not protocol result codes, so they went to the **localization** system
(`state/localization`) — the P0 message-table API (`.ron` result-code rows, `format_message`) is
**not** duplicated.

32 new localization fields with `#[serde(default = "default_…")]` (old/external language files keep
loading and fall back to English): 18 `chat_*` templates, 7 `chat_*` templates for the remaining
chat feedback (experience line + its 4 labels, item obtained, sell failure) and 7 `error_*`
templates for the error-window feedback. The already existing `not_in_party_text` is reused for the
party-chat warning. Templates use the same `{name}` placeholder syntax as the result messages and
are rendered by a new shared function:

```rust
// korangar-messages/src/lib.rs
pub fn render_template(template: &str, args: &[(&str, &str)]) -> String
```

(`render_named` was generalised to an optional result-code parameter; `format_message` behaviour is
unchanged and its 9 tests still pass). `lib.rs` has one helper
`push_localized_chat<P: rust_state::Path<ClientState, String>>(template, args)`, so the call sites
read `self.push_localized_chat(client_state().localization().chat_skill_passive(), &[("skill", …)])`.

| key | English text (byte-for-byte the previous literal) | previous site |
| --- | --- | --- |
| `chat_aura_style_set` | `Aura style set to "{style}".` | `set_aura_style` |
| `chat_party_invite_ignored` | `Ignoring party invitation from "{name}": you are already in a party.` | `PartyInvite` event |
| `chat_cannot_raise_stat` | `You cannot raise that stat right now.` | `StatUpFailed` event |
| `chat_players_online` | `{count} players are online.` | `UserCount` event |
| `chat_map_server_refused_login` | `The map server refused the login (error {error}).` | `RefuseEnter` event |
| `chat_skill_passive` | `Skill {skill} is a passive skill and cannot be cast.` | `use_skill` + `CastSkill` (2 sites) |
| `chat_aura_usage` | `Usage: /aura <style>. Available styles: {styles}.` | `/aura` command |
| `chat_aura_unknown` | `Unknown aura style "{style}". Available styles: {styles}.` | `/aura` command |
| `chat_party_usage` | `Usage: /p <message>` | `/p` command |
| `chat_hotbar_page` | `Hotbar page {page}/{pages}.` | hotbar tab cycle |
| `chat_hotkey_item_missing` | `The item of that hotkey is not in your inventory.` | hotbar item use |
| `chat_skill_not_learned` | `You have not learned skill {skill}.` | hotbar + skill tree (2 sites) |
| `chat_skill_target_out_of_range` | `Skill {skill}: the target is out of range.` | attack + support cast (2 sites) |
| `chat_skill_target_gone` | `Skill {skill}: the target is no longer in view.` | attack + support cast (2 sites) |
| `chat_skill_no_target` | `Skill {skill}: select a target first.` | attack cast |
| `chat_skill_cell_out_of_range` | `Skill {skill}: that cell is out of range.` | ground cast |
| `chat_skill_position_unknown` | `Skill {skill}: the player position is not known yet.` | ground cast |
| `chat_skill_click_ground` | `Skill {skill}: click a cell on the ground to cast it.` | ground cast |
| `not_in_party_text` (reused) | `You are not in a party.` | `/p` command |

Additional feedback templates (the sites that did not go through `push_chat_information`):

| key | English text (previous literal) | previous site |
| --- | --- | --- |
| `chat_experience_gained` | `Gained {gains}.` | aggregated experience chat line (`PendingExperience`) |
| `chat_experience_base` / `_job` / `_quest_base` / `_quest_job` | `base EXP` / `job EXP` / `base EXP (quest)` / `job EXP (quest)` | the four labels of that line (`Localization::experience_labels`) |
| `chat_item_obtained` | `You got {name} ({quantity}).` | `ItemObtained` event (direct `ChatMessage`) |
| `chat_sell_failed` | `Failed to sell items` | `SellingCompleted::Error` (direct `ChatMessage`) |
| `error_map_load_failed` | `Failed to load the map '{map_name}'. Please try again.` | `SessionAction::MapLoadGaveUp` error window |
| `error_session_expired` | `Your session expired. Please log in again.` | `CharacterReconnectGaveUp` error window |
| `error_character_selection_missing_login_data` | `Character selection failed: missing login data.` | `CharacterSelected` guard |
| `error_character_selection_missing_slot` | `Character selection failed: the server did not send that character slot.` | `CharacterSelected` guard |
| `error_character_slot_switch_failed` | `Failed to switch character slots` | `CharacterSlotSwitchFailed` |
| `error_map_server_auth_failed` | `The map server rejected the connection (reason {reason}). Please try again.` | `MapServerAuthFailed` |
| `error_map_data_missing` | ``Map data missing: `{map_name}`\nThis map is not in the game archives. Please log out and choose another character.`` | fatal map asset failure |

`en-US.ron` gets the exact previous texts; `de-DE.ron` gets translations of all 32 keys (the
existing parity test `language_files_have_the_same_keys` proves every key is in both files).
33 call sites were migrated (25 chat `push_localized_chat`/direct pushes + 7 error windows + the
experience line); `push_chat_information(format_message(…))` for the protocol result codes is
untouched. `PendingExperience::take_message` now returns the four amounts
(`take_gains`), and `Localization::experience_labels` supplies the localized labels, so the
aggregation/formatting logic stays in `lib.rs` while the text is data.

**Note (structural limit found while adding the fields):** the `StateElement` derive renders a
named struct as a tuple of its fields and `korangar-interface` implements `Element` for tuples up
to 140 elements. `Localization` was at 133 fields; the 18 chat templates are marked
`#[hidden_element]` (they are not interface labels) to stay at 133. A further localization field
added without `#[hidden_element]` (or a nested sub-struct) will hit that arity limit; document this
in any follow-up localization work.

---

## 4. Tests

### 4.1 New regression test `korangar/tests/world_audio_data.rs` (7 tests)

* `config_constants_are_gone_and_data_accessors_are_used` — the five constants are gone from
  `lib.rs`; the five `client_config` accessors are used.
* `debug_map_window_is_data_driven_and_stays_debug_only` — no `MAP_WARPS`/`MAP_COUNT`/coordinate
  literals in `windows/maps.rs`; window still `#[cfg(feature = "debug")]`; data file documents the
  debug-only status.
* `quest_icon_is_data_driven` — no Korean path / `QuestColor` arms / RGB literals in
  `world/particles/mod.rs`; data file carries the previous template/tints.
* `world_and_audio_data_carry_the_previous_values` — previous map/camera/audio paths/23 warps.
* `data_files_are_loaded_through_the_archive_with_a_compiled_fallback` — path + `include_str!` +
  `deny_unknown_fields` per loader; load call in start-up.
* `data_cli_checks_accept_and_reject_files` — `check_*_config(Some(path))` accepts a valid
  override and rejects unknown fields / undecodable audio extension / duplicate debug warp (temp
  files).
* `chat_feedback_literals_are_localized` — the previous literals (33 sites, including the
  experience/item/sell and error-window strings) are gone from `lib.rs`; all 32 templates exist in
  `en-US.ron` with the exact previous text and in `de-DE.ron`; every key has a call site (the four
  experience labels through `Localization::experience_labels`); the code uses
  `korangar_messages::render_template`.

### 4.2 Unit tests inside the new/edited modules (15 pass)

`client_config::world_config::tests::*` (4), `client_config::ui_audio::tests::*` (4),
`client_config::debug_maps::tests::*` (3) plus the 3 language-file tests and the new
`korangar-messages` renderer test (9 tests in `korangar-messages`, all pass).

### 4.3 Verification commands (own `CARGO_TARGET_DIR`, pinned nightly, `slangc` on PATH)

| check | result |
| --- | --- |
| `cargo check -p korangar` (default) | clean, no warnings |
| `cargo check -p korangar --all-features` | clean, no warnings |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-features -- -Dwarnings` | **exit 0** |
| `cargo clippy --workspace -- -Dwarnings` | **exit 0** |
| `cargo test -p korangar --all-features --test world_audio_data` | **7 passed** |
| `cargo test -p korangar --test world_audio_data` (default features, CI matrix) | **7 passed** |
| `cargo test -p korangar --all-features --lib` | **526 passed, 0 failed, 12 ignored** |
| `cargo test -p korangar --all-features --lib -- client_config state::localization` | **15 passed** |
| `cargo test -p korangar-messages --all-features` | **9 passed** |
| `cargo test -p korangar --all-features --test message_tables --test skill_behaviour` | 4 + 7 passed (no regression of the P0/P4 guards) |
| `cargo run -p korangar -- --check-world` | `world config: default map "geffen" and 4 quest colours are valid` |
| `cargo run -p korangar -- --check-ui-audio` | `ui audio: menu music Some("bgm\\01.mp3"), 2 sound effects are valid` |
| `cargo run -p korangar -- --check-debug-maps` | `debug maps: 23 warps are valid` |

**Pre-existing red gate, not caused by this lane:** `ui_consistency::hardcoded_style_literals_do_not_exceed_the_token_budget`
fails at the integration base `7ce0f4cd` — the base tree has 29 `Color::*_u8` literals in
`korangar/src/interface` against a budget of 28 (`git grep` on `7ce0f4cd` gives 29; the Lead's
Phase 0 working tree lowers the budget to 0). This lane adds no interface colour/FontSize literal
(the Maps window change only moves data into a `.ron` file), so the token ratchet is unchanged and
the failure is the known P0-owned state recorded by the audit. The other two `ui_consistency`
tests pass.

### 4.4 Muted Xvfb start-up smoke test

The data files were exercised by a muted Xvfb start-up of the client (no Wine, no server
connection, no characters): `KORANGAR_MUTE_AUDIO=1`, Xvfb display, the worktree's
`archive/data/client/*.ron` loaded through the game-file loader. The log shows the three
`client_config` load lines (all **from the archive**), the default map load and the login window
creation, and contains **0 panic reports**; the run was killed by its timeout after sitting on the
login screen (exit 124). Details in §8.

---

## 5. CI parity

* `.github/workflows/quality.yml` (feature-matrix tests job, `default` features): new step
  **Client configuration data file schema (compiled fallbacks)** running `--check-world`,
  `--check-ui-audio`, `--check-debug-maps` next to the existing `--check-items`/`--check-skills`
  steps.
* `.github/workflows/tests.yml`: new step **Check client configuration data** running the same
  three CLI checks with `--all-features`; the new `world_audio_data` integration test runs in the
  existing `cargo test --all-features` step.
* No generator step: **no facts were extracted for this data** (see §6), so there is nothing to
  `--check` against a facts JSON or to regenerate; the drift guard is the Rust test.

---

## 6. Provenance / generator note

No game data was extracted for this phase: every value is a hand-curated copy of the previous
Rust constant/literal, so the data files carry a `provenance` string naming the code they came from
and there is no `tools/extract_*.py` / `tools/gen_*.py` generator (a generator without facts would
only reproduce the data file from itself). If the values ever need to be derived again (e.g. the
quest icon files or the official UI audio paths), the extraction should be added as
`tools/extract_*.py` + facts JSON + `tools/gen_<system>.py --check` following the weapon/skill
template — this is called out so it is not silently re-hand-edited.

The debug map list is a client/tester convenience (not official client data); its coordinates
come from the previous Korangar array.

---

## 7. Limitations and notes

* **Debug map list stays debug-only.** The window and the runtime registry are behind
  `#[cfg(feature = "debug")]`; the `--check-debug-maps` CLI works in every build (schema half is
  always compiled). The data file is intentionally not localized (debug content).
* **Quest icon fallback changed from panic to skip.** A missing/invalid quest-icon texture is now
  logged and the icon is skipped; previously `unwrap()` would panic. A malformed override therefore
  degrades visibly instead of crashing.
* **UI audio can be disabled per server** (`ui_background_music: None`, empty/absent sound
  paths→ no-op); the committed values make a default install identical to before.
* **`Localization` StateElement arity** — see the note in §3 (140-element tuple limit, 133 used).
* **No live visual verification of the login screen/maps window** beyond the start-up smoke test
  (the debug Maps window would need a GM/server connection and is a debug convenience; the audit
  explicitly allows skipping visual checks). The data is exercised by the CLI + tests and by the
  start-up load.
* **Pre-existing token-budget gate red at the base SHA** (see §4.3), owned by the P0 lane.
* Windows/packaging: Rust-only change plus three data files and the two localization RONs, all
  checked in; the next RO-Next Windows build picks them up, no patcher/server change.

---

## 8. Start-up smoke test (details)

Environment: Linux, Xvfb (`xvfb-run -a -s "-screen 0 1280x720x24"`), muted audio
(`KORANGAR_MUTE_AUDIO=1`), software/NVIDIA Vulkan, `karangar` binary built from this worktree with
its own `CARGO_TARGET_DIR`, game archives symlinked read-only from
`/home/phantom/Games/ragnarok/CondaRO-linux/client/game` (`data.grf`, `CondaRO.grf`,
`palettes.grf`) plus a local `client/game_archives.ron` and `archive/` folder (the repo data files).
The client is killed after the login screen is up; no login/character/network action is taken.

First attempt (only the tracked repo files, no generated runtime data): the three data files
loaded (`from the archive`) and then the client panicked in the pre-existing `load_client_info`
(`data\sclientinfo.xml` from `data.grf`: `duplicate field "filename"`) — a runtime-data setup
issue unrelated to this lane. After copying the generated (gitignored) runtime files that the main
worktree already has — `archive/data/sclientinfo.xml`, `lua_files.7z` and the tuned
`client/*_settings.ron` (read-only copies; **not** `login_settings.ron`/`friend_names.ron`, i.e. no
credentials) — the start-up completed:

    [DEBUG korangar::client_config::world_config] world config: loaded the default map "geffen" and 4 quest colours from the archive
    [DEBUG korangar::client_config::ui_audio] ui audio: loaded menu music Some("bgm\\01.mp3") and 2 sound effects from the archive
    [DEBUG korangar::client_config::debug_maps] debug maps: loaded 23 warps from the archive
    load client config (0ms)
    load default map → completed (876ms)
    create login window state → completed (0ms)
    … (login screen running, 0 panic reports, killed by its timeout: exit 124)

(`from the archive` because the worktree's `archive/` folder is mounted by the default archive
list; a build without that folder logs `from the compiled fallback`. The run never logs in, never
selects a character and never connects to a server, so no user data is touched.)

---

## 9. Files changed

New:

* `korangar/archive/data/client/world_config.ron`
* `korangar/archive/data/client/ui_audio.ron`
* `korangar/archive/data/client/debug_maps.ron`
* `korangar/src/client_config/mod.rs`, `world_config.rs`, `ui_audio.rs`, `debug_maps.rs`
* `korangar/tests/world_audio_data.rs`
* `notes/korangar-p3-world-audio-data.md` (this report)

Modified:

* `korangar/src/lib.rs` (constants removed, accessors/`Option<SoundEffectKey>` fields, client-config
  load, `push_localized_chat` helper, `check_world_config`/`check_ui_audio`/`check_debug_maps`)
* `korangar/src/main.rs` (`--check-world`, `--check-ui-audio`, `--check-debug-maps`)
* `korangar/src/interface/windows/maps.rs` (data-driven dynamic warp list)
* `korangar/src/world/particles/mod.rs` (quest icon from data, graceful skip)
* `korangar/src/state/localization/mod.rs` (32 client feedback templates + serde defaults +
  `hidden_element` + `experience_labels`)
* `korangar/archive/data/languages/en-US.ron`, `de-DE.ron` (the 32 templates)
* `korangar-messages/src/lib.rs` (`render_template`, shared placeholder renderer)
* `korangar-messages/src/tests.rs` (renderer test)
* `.github/workflows/quality.yml`, `.github/workflows/tests.yml` (CLI schema checks)
