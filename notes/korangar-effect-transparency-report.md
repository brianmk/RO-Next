# Korangar / RO-Next — procedural skill effect transparency fix (black instead of transparent)

Owner: `effect-alpha` (subagent lane)
Branch: **`fix/effect-transparency`** (worktree `/home/phantom/Games/ragnarok/CondaRO-linux/korangar-effect-alpha`,
own `CARGO_TARGET_DIR=/home/phantom/korangar-effect-alpha-target`)
Base: integration HEAD **`efbe47dd`** (`fix(minimap): flip generated map texture vertically …`, includes
`feat/skill-procedural-effects` from `9b467333`)
Hand commit: **`4b20b4104914630fb73c2fbd18c8039198fc8fd0`** (`4b20b410`, `fix(effect): use premultiplied-alpha blend factors for world effects`)
Tested release binary: `/home/phantom/korangar-effect-alpha-target/release/korangar`
sha256 prefix `2c3eaf6edc9d987f` (built from the hand commit, `cargo build --release`)

## Symptom

During skill effects, parts that should be translucent rendered **dark/black**:

* **Procedural hit flash** (e.g. `@displayskill 10 10 3`, Inc Agi success rows: `effect\pok3.tga` burst): a
  soft white flash appeared as a **dark gray/black smudge** over the ground.
* **Storm Gust** (procedural `StormCloud` + `stormgust.str`): snow/cloud layers were **dimmed to a gray haze**
  instead of bright white/blue snow.
* **`.str` effects**: e.g. Magnus Exorcismus (`@displayskill 79 10 4`) drew the angel/wing layers as
  **opaque black silhouettes** instead of light translucent planes.
* Ground auras / rings (procedural `GroundAura` on `ring_*.tga`, `alpha_*.tga`) were darker than intended.

## Root cause

The effect renderer mixes **premultiplied alpha assets** with **straight-alpha blend factors**.

1. **Effect textures are premultiplied at load.** `TextureLoader::load_texture_data`
   (`korangar/src/loaders/texture/mod.rs`) runs `premultiply_alpha()` on every `.tga`/`.png` texture
   (`korangar/src/loaders/color.rs`). `.bmp` effect textures have no alpha channel; they are color-keyed
   (magenta key → `[0,0,0,0]`), so their alpha is only ever 0 or 1 — premultiplication is a no-op there.
2. **Effect colors are premultiplied.** `Color::components_linear` (`korangar/src/graphics/color.rs`) returns
   `linear(rgb) * alpha` for the RGB channels (the doc comment says so explicitly: *"we use pre-multiplied
   alpha blending, we premultiply the alpha here too"*); the forward effect pass uploads exactly that value.
3. **The effect shader outputs the premultiplied product.** `korangar/shaders/passes/postprocessing/effect.slang`
   (`fs_main`) returns `texture.Sample(...) * input.color` — premultiplied texture × premultiplied color.
4. **The blend factors are straight-alpha.** The forward effect pass took the factors verbatim:
   * `.str` effect data supplies them per frame (`parse_blend_factor`, `korangar/src/loaders/effect/mod.rs`),
     written for the official DirectX straight-alpha pipeline;
   * the procedural primitives hard-code straight-alpha pairs:
     `blend_factors()` in `world/effect/procedural/mod.rs` (`SrcAlpha/One`, `SrcAlpha/OneMinusSrcAlpha`),
     `SkillAuraSpec::blend_factors()` and `StormCloud`.

With `SrcAlpha` as the **source** factor the already premultiplied source color is multiplied by its alpha a
second time:

```
intended (straight alpha):  out.rgb = src.rgb * src.a + dst.rgb * dst.a
actual (premultiplied src): out.rgb = (src.rgb * src.a) * src.a + dst.rgb * dst.a
```

Every semi-transparent texel loses one alpha factor (`a²` instead of `a`). Soft-alpha TGA effects — the white
`pok3.tga` flash, the `cloud*.tga` billboards, the `ring_*.tga`/`alpha_*.tga` auras, the `.str` angel layers —
therefore collapsed towards black: bright white flashes became dark gray/black blobs and translucent planes
became black silhouettes. Fully transparent texels (`a = 0`) and `.bmp` color-key texels (`a ∈ {0,1}`) were
mathematically unaffected, which is why only the soft-alpha parts were visibly broken.

Client data confirms the scope. Parsing **all 131 `.str` files** referenced by
`archive/data/client/skill_effect_assets.ron` gives these (source, destination) blend pairs over their frames:

| frames | source | destination | share |
| --- | --- | --- | --- |
| 32 651 | `SrcAlpha` | `DstAlpha` | 93.2 % |
| 2 253 | `SrcAlpha` | `OneMinusSrcAlpha` | 6.4 % |
| 444 | `One` | `Zero` (opaque) | 1.3 % |
| 35 | `One` | `One` (additive) | 0.1 % |

i.e. **~99.6 % of all `.str` effect frames** used a `SrcAlpha` source factor and were affected; every
procedural additive layer (`SrcAlpha`/`One`) and every procedural alpha layer (`SrcAlpha`/`OneMinusSrcAlpha`)
was affected too.

This is not a texture-format problem: textures already upload as `Rgba8UnormSrgb`/`Bc7RgbaUnormSrgb` with a
preserved alpha channel, the shader never discards alpha, and the pipeline does use blending. The bug is
strictly the alpha convention mismatch between the assets and the blend state.

## Fix

Translate the straight-alpha source factors from the effect data into premultiplied-alpha factors at the one
place where the forward effect pipeline is derived from an `EffectInstruction`:

`korangar/src/graphics/passes/forward/effect.rs`

```rust
fn premultiplied_blend_state(source: BlendFactor, destination: BlendFactor) -> (BlendFactor, BlendFactor) {
    let source = match source {
        BlendFactor::SrcAlpha | BlendFactor::SrcAlphaSaturated => BlendFactor::One,
        source => source,
    };
    (source, destination)
}
```

* `SrcAlpha` → `One` (the source color already contains the alpha factor exactly once).
* `SrcAlphaSaturated` → `One` (same reasoning; cannot be expressed with premultiplied source).
* All other source factors and **all destination factors are unchanged** — destination factors operate on the
  scene color and were already correct.
* Applied in **both** `Prepare` paths (bindless and per-texture) and to the batch key, so batching/pipeline
  reuse stays correct.
* `PRELOAD_PIPELINES` was updated to the translated combinations, including the now very common `(One, One)`
  used by the procedural additive primitives (hit flashes, projectiles, additive auras).
* Documentation added to `PRELOAD_PIPELINES`, `premultiplied_blend_state`, the procedural `blend_factors()`
  helper, `render_world_quad_blended` and `SkillAuraSpec::blend_factors()` so the straight-alpha convention of
  the data/primitive is explicit and future effects do not reintroduce the mismatch.

For the combinations the client actually uses the translation is mathematically exact:

```
straight:      src.rgb * src.a + dst.rgb * dst.a
premultiplied: (src.rgb * src.a) * 1 + dst.rgb * dst.a   // identical
```

`.bmp` color-key textures are untouched (alpha 0/1), **opaque world rendering is untouched** (only the
forward-effect pipeline's blend state changes), and no shader, texture-format, `.str` parser, effect-cap or
de-duplication code was modified.

### Files changed

| file | change |
| --- | --- |
| `korangar/src/graphics/passes/forward/effect.rs` | **the fix**: `premultiplied_blend_state`, applied in both prepare paths + preload list, 2 unit tests |
| `korangar/src/world/effect/procedural/mod.rs` | doc: `blend_factors` uses the straight-alpha convention of the data |
| `korangar/src/world/effect/skill_aura.rs` | doc: same, on `SkillAuraSpec::blend_factors` |
| `korangar/src/world/effect/mod.rs` | doc: `render_world_quad_blended` factors are translated centrally |
| `korangar/src/main.rs` | rustfmt only (HEAD `efbe47dd` left a doc comment unformatted; `cargo fmt --all --check` fails at the branch base without this hunk) |

### Rejected alternatives

* *Change texture upload to RGBA8 / stop premultiplying* — textures already upload RGBA8 with alpha; the
  premultiplication is shared by the premultiplied map/entity/model pipelines, so changing it would break
  opaque world rendering for no benefit.
* *Generate alpha from the BMP color key in the shader* — the loader already color-keys BMPs at load; the
  remaining problem is not the BMP alpha (0/1 is unaffected by double multiplication) but the TGA/PNG soft
  alpha and the blend convention.
* *Discard fully transparent texels* — `a = 0` texels were already invisible; the visible defect was in
  semi-transparent texels (alpha fringes/halos), so a discard would not fix it.
* *Unpremultiply in the shader and keep straight-alpha factors* — mathematically equivalent for the
  `SrcAlpha` factors, but it would need a second color path for all effect colors and adds a division /
  guard in the hot fragment shader; the blend-state translation fixes every existing instruction with a
  one-line change and keeps the premultiplied pipeline consistent.

## Verification

### CI parity (own `CARGO_TARGET_DIR`, commit `4b20b410`)

| check | result |
| --- | --- |
| `cargo fmt --all --check` | **pass** (after the `main.rs` rustfmt hunk; base `efbe47dd` fails this) |
| `cargo clippy -- -Dwarnings` | **pass**, 0 warnings |
| `cargo clippy --all-features -- -Dwarnings` | **pass**, 0 warnings |
| `cargo test --all-features` (workspace) | **pass** — korangar lib 372 passed / 0 failed / 11 ignored, all other test binaries green |
| new unit tests | `graphics::passes::forward::effect::tests::source_alpha_is_translated_to_one`, `…::factors_that_do_not_scale_by_source_alpha_are_unchanged` — pass |
| `cargo build --release` | **pass** (slangc from `tools/bin`), binary sha256 prefix `2c3eaf6edc9d987f` |

### In-client (muted lean Xvfb, GM test character only — no user characters)

Method: `scripts/run-korangar-test.sh`-style lean profile on **Xvfb `:96` 1280×720**, 30 fps, no shadows/MSAA,
audio null/muted, display `:96`, client working dir is the worktree's `korangar/` crate dir. Account
**`verify4` / `VerifyBot4`** (level-1 Novice, GM 99) — the user's `drcake` client/character was never used or
touched. Effects were triggered through the real server GM command `@displayskill <id> <level> <type>`
(`@displayskill 10 10 3` = Success/no-damage, `@displayskill 79 10 4` / `89 10 4` = ground), after
`@warp prt_fild08 200 200` (empty field) and with the camera zoomed in (mouse wheel), so the packet →
`NetworkEvent` → effect-spawn path is exercised. Raw frames were captured with `ffmpeg x11grab` at 30 fps.

Before binary: **`efbe47dd`** release build (sha256 prefix `0177e87a`). After binary: **`4b20b410`** (prefix
`2c3eaf6e`). Same display, profile, character, warp and command set.

| effect | before (`efbe47dd`) | after (`4b20b410`) | evidence |
| --- | --- | --- | --- |
| **Inc Agi success** (`10 10 3`; `pok3.tga` soft-alpha burst, alpha blend) | dark gray/black smudge: effect area **8.4 luma darker** than the surrounding grass | bright white glow: effect area **+43.9 luma brighter** than the surroundings | `compare-incagi_success.png`, `anim-incagi_success.gif`, frames `before-/after-incagi_success.png` |
| **Magnus Exorcismus ground** (`79 10 4`; `.str` angel layers + procedural aura) | two angel layers rendered as **opaque black silhouettes**; 1 435 near-black pixels in the angel band | all three angels render light/white; **4 near-black pixels**; 171 → 2 584 bright pixels | `compare-magnus_ground.png`, `magnus-animation-montage.png`, `anim-magnus_ground.gif` |
| **Storm Gust ground** (`89 10 4`; `.str` snow + procedural `StormCloud`) | dark gray haze, snow barely visible; mean luma 112.9, 4 371 bright px | bright blue-white snow/cloud; mean luma 125.2, **9 928 bright px** | `compare-stormgust_ground.png` |

Additional checks in the same sessions:

* **No crash / no new errors**: final client log has no panic/backtrace (`client-final.log`, 29 lines of normal
  startup/session output); the client ran the full command set and stayed logged in.
* **No black halos/background** remain around hit flashes, rings, clouds or `.str` planes in any after frame.
* **Effect caps / de-duplication unaffected**: no spawn/cap/culling code was touched (`EffectHolder`,
  `skill_completion_tracker` unchanged); crowded-plaza captures still show effects spawning and the per-entity
  / global budget behaviour unchanged.
* **Opaque world rendering unchanged**: only the forward-effect pipeline blend state changed; map, model and
  entity passes are untouched (same frames show normal shadows, terrain, entities).
* Perf: no extra draw calls, no extra state (the same six preloaded pipelines, translated); the client stayed
  at its configured 30 fps cap in the field and plaza sessions.

### `.tga` vs `.bmp`

* `.tga`/`.png` effect textures with soft alpha were the visibly broken ones and are fixed (`pok3`, `cloud*`,
  `ring_*`, `alpha_*`, `lens*`, `ice`, `fireparticle`, …).
* `.bmp` color-key textures (magenta key → alpha 0/1) are mathematically unaffected by the double alpha
  multiplication; they now match the official straight-alpha equation exactly as well (Magnus angel/wing BMPs
  that were drawn with `SrcAlpha` data factors now render correctly).
* No change to texture loading/format was needed: effect textures are already RGBA8 (or BC7 with alpha) and the
  alpha channel is preserved end to end.

## Windows / packaging note

The fix is **Rust-only** — no shader source change, no asset/GRF/`.str`/`.ron` data change, no new
dependencies. Windows users therefore need a **rebuilt `RO-Next.exe`** (same code, `cargo build --release`
with the Windows toolchain / CI release flow); nothing else in the game data needs to be repackaged. This is
ready to go into the next packaged build (0.1.3+). The shaders shipped with the Windows build are unchanged.

## Reproduce

```bash
# worktree
cd /home/phantom/Games/ragnarok/CondaRO-linux/korangar-effect-alpha
git checkout fix/effect-transparency        # 4b20b410

export PATH=/home/phantom/Games/ragnarok/CondaRO-linux/tools/bin:$PATH
export CARGO_TARGET_DIR=/home/phantom/korangar-effect-alpha-target
cargo build --release

# in-client (lean, muted, own Xvfb display; verify4/VerifyBot4 only)
scripts/run-korangar-test.sh --display :96 --clone <this worktree> \
    --binary $CARGO_TARGET_DIR/release/korangar
# in game:
#   @warp prt_fild08 200 200
#   @displayskill 10 10 3      # Inc Agi flash: was black smudge, now white glow
#   @displayskill 79 10 4      # Magnus: was black angel silhouettes, now light angels
#   @displayskill 89 10 4      # Storm Gust: was dark haze, now bright snow/cloud
```

## Evidence

`notes/korangar-effect-transparency-evidence/`

* `compare-incagi_success.png`, `compare-magnus_ground.png`, `compare-stormgust_ground.png` — labelled
  before/after crops at the effect peak (clean empty field, identical camera/warp/character).
* `before-*.png` / `after-*.png` — the unlabelled full 1280×720 frames used for the comparisons.
* `anim-incagi_success.gif`, `anim-magnus_ground.gif` — before/after animation (Prontera plaza recording of the
  same before/after binaries; the subsequent preload-list-only change does not alter rendering).
* `magnus-animation-montage.png` — before/after at +0.3 s … +3.0 s of the Magnus animation: the black
  silhouettes existed throughout the whole animation, the after frames are light throughout.
* `plaza-before-incagi_success.png`, `plaza-after-magnus_ground.png` — crowded Prontera plaza captures used for
  the cap/regression sanity pass.
