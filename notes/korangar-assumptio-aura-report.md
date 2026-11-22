# Korangar / RO-Next — Assumptio status glow + `.str` effect mask transparency (Assumptio / Angelus)

Owner: `effect-alpha` (subagent lane)
Branch: **`feat/assumptio-status-aura`** (worktree `/home/phantom/Games/ragnarok/CondaRO-linux/korangar-assumptio`,
own `CARGO_TARGET_DIR=/home/phantom/korangar-assumptio-target`)
Base: integration `12171234` (effect-transparency fix merged); main tree has since advanced to
`dd302f94` but this branch only touches the files below.
Release binary tested: `…-target/release/korangar`, sha256 prefix `52563406a51e2f7e`

## What this branch implements

### 1. Persistent Assumptio status glow (generic status-aura framework)

New module `korangar/src/world/effect/status_aura.rs`:

* a small **registry** `status_aura_spec(status_index)` maps an `EFST_*` status id to a
  `StatusAuraSpec` (textures, tints, sizes, pulse, rotation, alpha). Adding another glowing status
  later is one spec constant + one match arm; the rest of the wiring is shared,
* `EFST_ASSUMPTIO` (110) and `EFST_ASSUMPTIO2` (473, "Party/Cash Assumptio") are mapped to a
  blue-white "shield of light" built from textures that are in the 2019 data set (the official
  persistent `EF_ASSUMPTIO` visual is hard-coded C++ and has no reverse-engineered data):
  * a soft radial glow flat on the ground (`effect\round_sphere.bmp`),
  * a slowly rotating blue energy ring on the ground (`effect\efst_waterfield2\bubble1.bmp`),
  * a camera-facing body glow at **two thirds of the character height** (`round_sphere.bmp`),
    i.e. 14 world units above the feet (the client's head/name-label height is 21 units); while
    sitting the height is scaled by 12/21, matching the client's sitting head height,
  * the layers pulse out of phase so the aura "breathes"; all layers use **additive** blending
    (translated to premultiplied by the merged `premultiplied_blend_state` fix) and therefore can
    never draw black halos,
* the effect follows its entity through `EffectCenter::Entity`, scales with `Entity::get_scale()`,
  is hidden/removed when the entity dies, fades out or leaves the view, and is attached with
  `EffectHolder::add_unit`, so the global/per-entity effect budget and distance culling apply
  unchanged. It allocates once at spawn (three `Arc<Texture>` handles + state); rendering uses
  fixed-size loops and the per-frame reconciliation is allocation free (last-seen-frame stamp
  next to each spawned aura instead of a per-frame set).

Wiring in `korangar/src/lib.rs`:

* `World::update_status_auras()` reconciles the active statuses of
  `state::status_effects` against the spawned auras once per frame. Because it is driven by the
  status state (which is fed by `0x0983` start/refresh/end, `0x0984` on entering the view,
  `0x0196` end and local expiry), **all** lifecycle cases are covered by one code path:
  buff applied, re-cast/refresh (aura stays), expiry/end (aura removed via the shared
  `AtomicBool`), death and fading (excluded/removed), warp (status list + aura map cleared),
  entity leaving the view (removed, respawned by the next `0x0984`), and other players/monsters
  (the status state tracks every entity in the view),
* `EffectHolder::add_unit`/`add_effect` now return whether the budget accepted the effect; the
  status aura only records a spawned aura when it was actually added, so a budget rejection is
  retried next frame instead of leaving the status permanently without a glow.

### 2. No black mask/background in `.str` effect frames (Angelus and every similar effect)

Root cause of the black quads (e.g. the black box around the Angelus bell): `.str` effect layers can
use an **opaque** blend (`source = One`, `destination = Zero`) or a destination-alpha blend while
their texture is a color-keyed `.bmp` (`jong0001.bmp`, `prist-3.bmp`, …): the magenta-keyed
background is stored as `rgba = (0,0,0,0)`, and those blend modes still write its black RGB into
the frame buffer because they ignore the source alpha. The premultiplied effect textures mean that
an alpha of zero is *always* a fully transparent (zero) color, so the two effect shaders
(`korangar/shaders/passes/postprocessing/effect.slang` and `effect_bindless.slang`) now **discard
fully transparent texels** (`alpha <= 0.0001`). This is a no-op for the additive/destination-alpha
modes (they contribute zero there anyway) and removes the black background/frames for
opaque-blended color-keyed layers. It fixes Angelus and all other `.str` effects with the same
pattern (e.g. the Assumptio cast animation's `prist-3.bmp` opaque frames), without washing out
additive effects.

### 3. Clamp effect colors

`Color::components_linear` now clamps channels and alpha to `[0, 1]` before premultiplying.
Interpolated `.str` frames can overshoot their keyframes (the official effect data contains
negative alpha values between a `0` and a `255` key); negative alpha used to produce negative
(black) fragments in the premultiplied effect pipeline. Adds a unit test.

Also rustfmt-only reflow of `korangar/src/interface/windows/hotbar.rs` (the base commit `12171234`
is not `cargo fmt --all --check` clean because of that doc comment; without the hunk CI fmt fails).

## Files changed

| file | change |
| --- | --- |
| `korangar/src/world/effect/status_aura.rs` | **new**: status-aura registry + persistent Assumptio glow (2/3 height, sitting/scale/dead aware) + 3 unit tests |
| `korangar/src/world/effect/mod.rs` | export the status aura, `EffectHolder::add_*` return whether the effect was added |
| `korangar/src/lib.rs` | `status_aura_flags`/`status_aura_frame` state, `update_status_auras()` reconciliation, per-frame call, warp cleanup |
| `korangar/shaders/passes/postprocessing/effect.slang`, `effect_bindless.slang` | discard fully transparent (premultiplied alpha 0) texels — fixes black `.str` masks/backgrounds |
| `korangar/src/graphics/color.rs` | clamp channels/alpha in `components_linear` + test |
| `korangar/src/interface/windows/hotbar.rs` | rustfmt only (base fmt drift) |

## Verification (muted Linux client on Xvfb; GM test characters only)

Method: lean profile, `Xvfb :96 1280x720`, 30 fps, audio muted, `KORANGAR_UI_RECTS_DUMP` driven by a
small X11/XTEST driver; no Wine. Test characters: `VerifyBot6` (verify6, GM 99) and `VerifyBot4`
(verify4, GM 99); the user's character/client was never used. Skills triggered with real server
commands (`@useskill 33 1 self` for Angelus, `@useskill 361 1 self` for Assumptio,
`@displayskill 361 1 3` for the Assumptio `.str` animation). Frames captured at 30 fps and sampled.

* **Angelus black background**: the whole Angelus completion animation (frames 5.5 s – 10.5 s) was
  captured; the bell and rings render with a transparent surrounding in every frame — no black
  box. See `notes/korangar-assumptio-aura-evidence/angelus-montage.png` and `g01-angelus-*.png`.
  Statically: all `jong*.bmp` bell textures are magenta color-keyed, and the failing frames use the
  opaque `(One, Zero)` blend, which is exactly what the new alpha discard removes.
* **Assumptio `.str` animation black frames**: captured across the full animation
  (13.3 s – 17.5 s, `asum-montage.png` / `g02-asum-*.png`) — white angel/cross masks are
  transparent around the character in every frame, no black quads.
* **Persistent Assumptio aura**: captured around the player after `@useskill 361 1 self`
  (`g03-aura-a/b`, `aura-closeup.png`, walk/sit frames `g03`/`g04`): blue-white glow around the
  body following movement; sits lower while sitting. In these captures the character was level 99
  for the SP needed for the cast, so the client's own classic **level-99 aura overlapped** the
  Assumptio aura.
* Buff bar: `BuffBarWindow` appears after the cast (`d02`/`d05` after-cast frames), i.e. the status
  packet path is active.
* Test character restored: `@baselevelup -39` screenshot shows `VerifyBot6` back at Lv 1/1
  (`g08-level-reverted.png`).

### Open verification gap (honest note)

An isolated capture of the persistent aura without the level-99 aura, plus the expiry/death/warp
removal checks, could not be completed in the final verification window: the map/char server
repeatedly rejected character entry (`character selection: server rejected the selection: Rejected
from server`) after the many rapid test re-logins (stale sessions; the user's client is also
online), so the last sessions ended before the captures. The reconciliation code path is exercised
(the reconcile function runs and is unit-covered at the spec/registry level), but a clean
end-to-end in-client check (single level ≥ 20 character with the level-99 aura style set to Off,
verify aura on self and on another player, expiry, death, warp) is still **outstanding** and should
be done in the next verification window before shipping. No black/transparency issue was observed
in any of the captured `.str` frames.

## CI parity

| check | result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy -- -Dwarnings` | pass |
| `cargo clippy --all-features -- -Dwarnings` | pass |
| `cargo test --all-features` (workspace) | pass (all test binaries ok; `korangar` lib 384 passed / 0 failed / 11 ignored) |
| `cargo build --release` (slangc from `tools/bin`) | pass, binary sha256 prefix `52563406a51e2f7e` |

## Packaging / Windows

Rust **and shader sources** changed (shaders are compiled at build time), so the next RO-Next
Windows build has to be rebuilt from this branch; no data/GRF/`.ron` changes are required. Ready to
be included in the next packaged build after the outstanding in-client verification above.

## Evidence

`/home/phantom/Games/ragnarok/CondaRO-linux/notes/korangar-assumptio-aura-evidence/`

* `angelus-montage.png` — Angelus animation, frames 5.5 s … 10.5 s (transparent bell, no black box)
* `asum-montage.png` — Assumptio `.str` animation, frames 13.3 s … 17.5 s (transparent masks)
* `g01-angelus-*.png`, `g02-asum-*.png` — the individual source frames of the montages
* `g03-aura-a/b.png`, `aura-closeup.png`, `g04-sit.png` — persistent aura after cast (level-99 aura
  overlap noted)
* `d02-after-cast.png`, `d05-after-cast.png` — buff bar present after the Assumptio cast
* `g08-level-reverted.png` — test character restored to level 1
* `scene-montage.png` — overview of the verification scene
