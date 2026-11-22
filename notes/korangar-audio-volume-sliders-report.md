# Korangar / RO-Next — interactive audio volume sliders (Master / SFX / Music)

Owner: `audio-volume-ui` (subagent lane)
Branch: **`feat/audio-volume-sliders`**
Worktree: `/home/phantom/Games/ragnarok/CondaRO-linux/korangar-audio-volume`
Own build dir: `CARGO_TARGET_DIR=/home/phantom/korangar-audio-volume-target`
Base: integration HEAD **`12171234`** (`Merge branch 'fix/skill-animation-qa' into integration/all-fixes-20261121`)
Hand commit: **`d712cf4c6199c07aff7cb5338e230c15b291f825`** (`d712cf4c`, `feat(audio): interactive master/SFX/music volume sliders`)
Docs commit: this report on branch `feat/audio-volume-sliders` (same branch, directly on top of the code commit)
Tested release binary: `/home/phantom/korangar-audio-volume-target/release/korangar`
sha256 prefix **`a476cab6`** (`a476cab6bb9105063fe03cb5b2f21bc88a2e9b93ee9a815ed28e11094f8e3ca0`).
The in-client runs were made with release builds of this branch; the final
binary was smoke-tested in-client after the last rebuild (screenshot
`09-final-binary-smoke.png`, log `final-binary-smoke.log`).

---

## 1. Summary

The audio settings window used a fixed 10 % step **drop-down** for the background
music volume and had no master or sound-effect volume at all. This change adds a
reusable, interactive **volume slider** to the interface framework and uses it
for three live settings:

| setting | persisted field | default | effect |
| --- | --- | --- | --- |
| **Master** | `master_volume` | `100` % | gain of the master track; every category is multiplied with it |
| **Sound effects** | `sound_effect_volume` | `100` % | gain of the SFX track and the spatial/ambient SFX track |
| **Music** | `background_music_volume` | `50` % (old default) | gain of the background-music track |

The two existing checkboxes stay: *Mute audio on focus loss* and *Mute
background music*. The music mute overrides the music slider (it sets the music
track to silence but keeps the stored value), and a master volume of 0 % is the
"mute everything" state.

Old `client/audio_settings.ron` files keep loading: the old enum value
(`background_music_volume: Sixty`) is migrated to `60`, missing fields get the
sensible defaults above, out-of-range values are clamped, and a legacy file is
rewritten in the new numeric format immediately on load.

No game data, archives, shaders, assets or dependencies were added — the change
is Rust-only.

## 2. New slider component (`korangar-interface`)

New file `korangar-interface/src/components/slider.rs`:

* draws a compact rounded track, a rounded fill and the current value as
  centered percentage text (`[======= 60% -----]`), 
* **click** on the track jumps to the clicked position,
* **press + drag** keeps updating the value across frames and even when the
  pointer leaves the track (the drag ends on mouse release),
* **mouse wheel** over the slider changes the value in 5 % steps,
* the value is generic over a `SliderValue` trait (`u8` percent in `0..=100` or
  `f32` linear in `0.0..=1.0`); the component only ever writes clamped values,
* all colors/height/font/alignment come from a `SliderTheme<App>` block, so the
  component works with the flat dark theme, the classic RO skin and custom theme
  files; the built-in themes derive the slider colors from the existing design
  tokens (`surface_sunken`, `state_hover`, `text_accent`, `focus_ring`,
  `text_primary`, `radius.small`, `typography.font_label`),
* the theme field is optional in persisted theme files (`serde(default)`), so
  old `in_game_themes/*.ron` / `menu_themes/*.ron` keep loading.

Framework plumbing (small and generic, not audio specific):

* `WindowLayout::update`/`Interface::lay_out_windows` now receive the primary
  (left) mouse button state, exposed as `WindowLayout::primary_button_down()`;
  this lets any future drag-style element follow a held button during layout,
* a fast click can press *and* release the button between two frames, so the
  slider also registers a normal click handler which applies the value from the
  track geometry recorded during the last layout pass.

The component is available as `slider! { value: <path> }` from the
`korangar_interface::prelude` component macros and is not tied to audio.

## 3. Settings and persistence (`korangar/src/settings/audio.rs`)

Volumes are stored as **`u8` percent in `0..=100`** (clean integer persistence,
exactly what the slider edits). Derived linear gains are
`percent as f32 / 100.0`.

```rust
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
    pub background_music_muted: bool,
    pub master_volume: u8,          // default 100
    pub sound_effect_volume: u8,    // default 100
    pub background_music_volume: u8 // default 50 (old drop-down default)
}
```

### Backward-compatible deserialization / migration

`AudioSettings` has a hand-written `Deserialize` through a raw struct:

* all fields are `#[serde(default ...)]`, so files written by older clients
  (or files missing the new master/SFX fields) still load,
* the music volume field accepts **three** representations through a small
  untagged compatibility type:
  * the legacy enum ident from the drop-down era (`Ten` … `Hundred`),
  * the new integer percent (`10` … `100`),
  * a hand-edited linear float (`0.0` … `1.0`),
* numeric values above 100 are clamped on load,
* an unknown legacy value is a clean deserialization error (falls back to the
  defaults through the existing `AudioSettings::new()` path),
* when `load()` detects a legacy file (no `master_volume` field), it writes the
  migrated numeric format right away, so the migration is persisted even if the
  client is killed before the next save.

Saving happens on every effective change (`update_settings`), when the audio
settings window is closed, and in `Drop` (clean logout/exit) — a change survives
a crash or forced exit. (The `Drop` saver is `#[cfg(not(test))]`: unit tests
construct and drop `AudioSettings` values, and without the guard a test run from
the crate directory would overwrite the developer's real
`client/audio_settings.ron` — observed and fixed during verification.)

### Effective volume math

| method | meaning |
| --- | --- |
| `master_volume_linear()` | master gain |
| `sound_effect_volume_linear()` | SFX category gain (without master) |
| `background_music_volume_linear()` | music category gain (without master) |
| `background_music_track_volume()` | music track gain: music volume, or 0 while muted |
| `effective_master_volume()` | master gain |
| `effective_sound_effect_volume()` | master × SFX |
| `effective_spatial_sound_effect_volume()` | master × SFX |
| `effective_background_music_volume()` | 0 when music muted, else master × music |

The mixer multiplies the master track with each category track, so the actual
heard gain is the product. The client sets the master track to the master
volume and the category tracks to the category gains (not the pre-multiplied
product), which avoids applying the master twice.

## 4. Live application to the audio engine

`korangar/src/lib.rs`:

* on startup the saved master/SFX/music volumes are applied to the engine and
  cached in `active_master_volume`, `active_sound_effect_volume`,
  `active_background_music_volume`,
* `update_settings()` compares the settings against the cached values and calls
  `set_main_volume`, `set_sound_effect_volume`, `set_spatial_sound_effect_volume`
  and `set_background_music_volume` live while the sliders are dragged,
* every applied change is logged (release-visible `log_debug!`, i.e. visible
  with `-v`/`KORANGAR_LOG_LEVEL>=1`) with both the category gain and the
  effective master × category gain — this is the mute-safe instrument used for
  verification because headless clients run with `KORANGAR_MUTE_AUDIO=1`,
* the audio settings window is closed through the menu toggle; closing saves
  the settings explicitly.

`korangar-audio`:

* **mute now preserves the master volume**: `EngineContext` stores
  `main_volume` and `muted`; `AudioEngine::mute(true/false)` only flips the
  mute override, so unmuting (focus regained) restores the configured master
  volume instead of unconditionally setting it to 1.0 (the old behaviour would
  have discarded a saved master volume),
* `effective_main_volume(main, muted)` is a pure helper with unit tests,
* all public volume setters funnel through `sanitize_linear_volume` which
  clamps to `0.0..=1.0` and turns NaN/±∞ into silence, so no out-of-range or
  non-finite value can reach the mixer,
* the spatial/ambient SFX track receives the same sound-effect volume.

## 5. UI

`korangar/src/interface/windows/audio_settings.rs` now shows, top to bottom:

1. ☑ *Mute audio on focus loss* (existing),
2. ☐ *Mute background music* (existing),
3. `Master        [======      ]`,
4. `Sound effects [=========   ]`,
5. `Music         [=====       ]`.

The `AudioSettingsCapabilities` drop-down options are gone (they were only used
for the music volume drop-down). New localization keys `master_volume_text` and
`sound_effect_volume_text` were added to both language files (English/German);
`background_music_volume_text` was shortened to "Music"/"Musik" so all three
rows render at the same compact font size at every interface scaling. Both
language files pass the existing key-parity test; the new keys have serde
defaults so old language files still load.

### Mute interaction semantics

* **Music mute** overrides the music slider: the engine's music track is set to
  silence, the stored music volume is kept and restored on unmute. The SFX and
  master settings are unaffected.
* **Master 0 %** is the global mute: every effective gain is 0 (master ×
  category), which the UI expresses by driving the master slider to the left.
* **Mute audio on focus loss** is enforced by the same engine mute override
  (`AudioEngine::mute(true)` on focus loss / suspend, `false` on focus), which
  now preserves the master volume. No separate "global mute" setting was added.

## 6. Verification

### CI parity (own `CARGO_TARGET_DIR`, hand commit)

| check | result |
| --- | --- |
| `cargo fmt --all --check` | **pass** (includes a rustfmt-only hunk in `hotbar.rs`: the base `12171234` fails `fmt --check` without it) |
| `cargo clippy --workspace -- -Dwarnings` | **pass**, 0 warnings |
| `cargo clippy --workspace --all-features -- -Dwarnings` | **pass**, 0 warnings |
| `cargo test --all-features` | **pass** (`694 passed / 0 failed / 12 ignored across all 39 test binaries`) |
| `cargo build --release` (slangc from `tools/bin`) | **pass**, binary sha256 prefix `<SHA256 PREFIX>` |
| `scripts/ui-token-budget.sh` | **pass** (`colors=27 (budget 28), FontSize literals=12 (budget 12)`) |

New unit tests:

* `settings::audio` (13 tests): legacy/missing-field loading, all ten legacy
  enum values × 10 % migration, new numeric format, out-of-range clamping
  (255/150/101 → 100), linear float music volume, unknown legacy value
  rejected, percent→linear conversion, multiplicative master × category,
  music mute override, master 0 silence, RON round-trip of the new format,
  legacy-file detection.
* `korangar-interface` slider (3 tests): `u8`/`f32` `SliderValue` clamping and
  percent conversion, fill width clamping (0 %, 1 %, 100 %, degenerate widths,
  no NaN/negative).
* `korangar-audio` (2 tests): linear-volume sanitizing (clamp, NaN, ∞),
  mute overrides but does not discard the master volume, percent→dB mapping.

### In-client (muted lean Xvfb, GM test account only — no user characters, no audio devices)

Method: private **Xvfb `:93`** 1280×720, software Vulkan (lavapipe), 30 fps, no
shadows/MSAA, `KORANGAR_MUTE_AUDIO=1`, null ALSA default, UI scale 0.75 (dark)
and 1.0 (classic), GM test account **`verify4`** with its test character (the
classic `verify4`/`verifypass` test account; the user's client/character and the
real Pulse/PipeWire devices were never used). The client was driven only through
XTEST input and the client's `KORANGAR_UI_RECTS_DUMP` window rectangles; the
audio engine logged the gains because audio output itself is muted.

| test | result | evidence |
| --- | --- | --- |
| Audio settings shows **three sliders** (Master, SFX, Music) instead of the drop-down; both mute checkboxes remain | **pass** | `01-…legacy-migrated.png`, `02-…after-sliders.png`, `08-menu-audio-settings-button.png` |
| Legacy RON `background_music_volume: Sixty` migrates to `60`, master/SFX default to `100`, file rewritten in numeric format on load | **pass** | `startup-migration.log`, `legacy-audio_settings.ron`, `migrated-audio_settings.ron` |
| Clicking a slider sets the value and applies the engine gains live (category + effective logs) | **pass** — master 30 %, SFX 69 %, music 40 % | `02-…after-sliders.png`, `slider-interaction.log` |
| Dragging the master slider updates every frame (30 → 80 %) | **pass** — log shows 0.45 → 0.23 → … → 0.80 | `03-…after-drag.png`, `slider-interaction.log` |
| Mouse wheel on the music slider changes it in 5 % steps (40 → 35 %) | **pass** | `slider-interaction.log` |
| Music mute overrides the music slider but keeps its stored value (35 %) and leaves SFX alone | **pass** — music track gain 0.00, effective music 0.00, effective SFX 0.55, RON `background_music_volume: 35` + `background_music_muted: true` | `04-…music-muted.png`, `slider-interaction.log` |
| Master volume 0 % silences everything (effective master/SFX/spatial/music = 0.00) | **pass** | `05-…master-near-zero.png`, `slider-interaction.log` |
| Values persist across a clean restart (master 80, SFX 69, music 35 restored from disk and applied at startup, no migration log) | **pass** | `07-…after-restart-persisted.png`, `persistence-restart.log` |
| Focus loss mutes and focus regain restores the master volume (engine mute override) | **pass** — `audio focus mute (muted=true)` / `(muted=false)` | `focus-mute.log` |
| Theme/scale safe: classic light theme at 1.0 scale (skinned window, light track, dark text) and dark theme at 0.75 | **pass** | `06-classic-theme-scale-1.0.png`, `01/02/…` |
| No crash / no new errors in the muted lean session; clean logout | **pass** | client logs (`Shutdown -> Disconnected`) |

The applied-gain log line used as instrument (one per change):

```
settings: audio volumes applied (master=0.80, sound_effects=0.69,
background_music=0.35, music_muted=false; effective master=0.80,
sound_effects=0.55, spatial_sound_effects=0.55, background_music=0.28)
```

`0.55 = 0.80 × 0.69` and `0.28 = 0.80 × 0.35` confirm the multiplicative
combination; with music muted the same line shows `background_music=0.00` while
SFX stays at `0.55`.

## 7. Files changed

| file | change |
| --- | --- |
| `korangar-interface/src/components/slider.rs` | **new** reusable slider component, `SliderValue`, `SliderTheme`, 3 unit tests |
| `korangar-interface/src/components/mod.rs` | register `slider` module |
| `korangar-interface/components/src/lib.rs` | `slider!` component macro |
| `korangar-interface/src/theme.rs` | `ThemePathGetter::slider()` |
| `korangar-interface/src/lib.rs` | export `SliderThemePathExt` from the prelude |
| `korangar-interface/src/layout/mod.rs` | `primary_button_down` in `WindowLayout` (+ getter) |
| `korangar-interface/src/lib.rs` | pass primary button state through `lay_out_windows` |
| `korangar/src/settings/audio.rs` | u8 percent volumes, backward-compatible deserialization/migration, clamping, effective gains, 13 unit tests |
| `korangar/src/interface/windows/audio_settings.rs` | three sliders + labels, drop-down/capabilities removed, save on close |
| `korangar/src/state/theme/interface.rs` | optional token-derived `SliderTheme` field + defaults for dark/menu/classic themes |
| `korangar/src/state/mod.rs` | slider theme path getter, capabilities field removed |
| `korangar/src/state/localization/mod.rs` | `master_volume_text`, `sound_effect_volume_text` (+ serde defaults) |
| `korangar/archive/data/languages/en-US.ron`, `de-DE.ron` | new/shortened slider labels |
| `korangar/src/lib.rs` | live volume application + caching, save-on-change, migration/application/focus-mute logging, audio window wiring |
| `korangar-audio/src/lib.rs` | master volume tracking + mute override, sanitizing, spatial SFX gain, 2 unit tests |
| `korangar/src/interface/windows/hotbar.rs` | **rustfmt only** (base HEAD fails `cargo fmt --all --check`) |

## 8. Reproduce

```bash
# worktree / build
cd /home/phantom/Games/ragnarok/CondaRO-linux/korangar-audio-volume
git checkout feat/audio-volume-sliders
export PATH=/home/phantom/Games/ragnarok/CondaRO-linux/tools/bin:$PATH
export CARGO_TARGET_DIR=/home/phantom/korangar-audio-volume-target
cargo test -p korangar --lib settings::audio
cargo test -p korangar-interface components::slider
cargo build --release

# in-client (muted lean Xvfb, GM test account only; no user client/audio device)
#   - Xvfb :93 1280x720, lavapipe, KORANGAR_MUTE_AUDIO=1, null ALSA default
#   - client working dir is the worktree's korangar/ crate dir
#   - settings: client/graphics_settings.ron (30 fps, no AA/shadows)
#               client/interface_settings.ron (English, scale 0.75/1.0, theme)
#               client/audio_settings.ron (old drop-down file to test migration)
#   - then: scripts in notes/korangar-audio-volume-evidence/verify-*.{sh,py}
```

The verification drivers in the evidence directory use the real client only
through XTEST and the window-rectangle dump: `verify-setup_run.sh` prepares the
settings, `verify-launch.sh` starts the muted lean client, `verify-drive.py`
logs in, opens the audio window and clicks/drags/scrolls the sliders, and
`verify-focus_test.py` moves the X input focus away and back to test the
focus-loss mute.

## 9. Evidence

`notes/korangar-audio-volume-evidence/` (next to this report on the box):

* `01-dark-audio-settings-legacy-migrated.png` … `09-final-binary-smoke.png`
  — dark theme (scale 0.75) and classic theme (scale 1.0) audio settings,
  migrated/edited/muted/master-zero/persisted states, and the final release
  binary (`a476cab6`) loading the persisted 80/69/35 settings after a restart,
* `compare-volume-sliders.png` — 2×2 comparison of migrated, edited, muted and
  master-near-zero states,
* `startup-migration.log`, `persistence-restart.log`, `slider-interaction.log`,
  `focus-mute.log`, `pointer-clicks.log`, `final-binary-smoke.log` — log excerpts
  from the muted in-client runs,
* `legacy-audio_settings.ron`, `migrated-audio_settings.ron`,
  `final-audio_settings.ron` — settings files before/after migration and after
  the interaction run,
* `verify-setup_run.sh`, `verify-launch.sh`, `verify-drive.py`,
  `verify-common.py`, `verify-focus_test.py` — the exact verification harness
  used (no credentials are stored in the repository; the test account is only
  referenced by name in the scripts).

## 10. Notes / packaging

* **Rust-only.** No shader source, game data, GRF, `.str`, `.ron` game-data
  asset, dependency or public API break: the change is a new interface
  component, new settings fields with migration and client wiring.
* Windows players need a **rebuilt `RO-Next.exe`** (same code, normal Windows
  release build); nothing else in the game data needs to be repackaged.
* Keyboard adjustment is intentionally not implemented (the task marked it
  optional); mouse wheel and click/drag cover the interaction. The slider
  percentage can always be set exactly by dragging to the track edge
  (1 % pixel resolution at the current compact size).
* The old drop-down enum (`BackgroundMusicVolume`) and its capabilities struct
  are gone from the client; the enum names are still accepted inside the
  deserializer for migration only.
