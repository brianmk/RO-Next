# Unified, data-driven multi-layer skill effect pipeline

Owner: `skill-pipeline` (subagent lane). Worktree `/home/phantom/korangar-skill-pipeline`,
branch **`feat/skill-effect-pipeline`**, base integration HEAD **`e87268af`**
(`tweak(effect): lower overhead buff anchor (clearance 5) and Assumptio aura`),
own `CARGO_TARGET_DIR=/home/phantom/korangar-skill-pipeline-target`.

Coordinates with:

* `skill-data` — official multi-layer rows (`feat/skill-data-full-layers` @
  **`e3ae2dcef4368e0cd4733ee984bd866263dcc305`**, cherry-picked into this branch):
  `tools/skill_effects_official.ron` (351 rows / 511 layers / 319 skills), the facts
  extractor + generator + JSON + report `notes/skill-effects-official-data.md`.
* `skill-audit` — `notes/skill-audit/{skill_matrix.json,summary.md,recommended-data-schema.md}`
  (main tree): the P0 suppression finding, the `head`-flag propagation gap and the schema
  recommendation are all addressed here.
* `skill-verify` — in-client verification lane (baseline only; verify4/VerifyBot4 was used
  for this lane's captures, verify6 stays with the QA lane).

## 1. What was wrong

Skill visuals were spread over five hand-maintained systems with different models:

* `world/skill_cast_effect.rs`: a hard-coded `skill_visuals()` match per skill plus
  `CompletionEffect` / `cast_aura` / `instant_auras` / `target_auras` / `body_flash`, and a
  blanket `has_custom_completion()` bypass that made `spawn_skill_effect_variants()` **skip
  the official data table entirely** for Blessing, Angelus, Inc AGI, Concentration, Hammer
  Fall, Warp, Zen and Fury. The audit proved this is why the already-generated official rows
  (Blessing effect 42 `축복` SPR + particle6 + pok2.tga, Inc AGI effect 37 `ac_center2`×3 +
  `agi_up.bmp`, Hammer Fall effect 102 `crashearth`) never spawned.
* `world/library/skill_effect_assets.rs` (`.str` rows), `skill_sprite_effects.rs`
  (`.spr` rows), `skill_procedural_effects.rs` (primitives): three separate RON files,
  three loaders, three spawn loops and no way to combine their layers for one phase.
* The official `head` (overhead) flag was guessed by a hand-curated skill-id list in the
  legacy `.str` generator instead of being carried by the data; the audit found
  59 KN_SPEARBOOMERANG, 62 KN_BOWLINGBASH and 357 LK_CONCENTRATION rendering at the feet.
* The overhead anchor itself (`overhead_effect_height`: head + clearance, scaled, sitting
  aware) was a working behaviour that had to be preserved.

## 2. Design: one descriptor, N ordered layers per phase

New module **`korangar/src/world/library/skill_effects.rs`** loads one runtime data file:

```
korangar/archive/data/client/skill_effects.ron
```

through `GameFileLoader` (`data\client\skill_effects.ron` archive override, server
overridable without a rebuild) with the same file embedded via `include_str!` as the
compiled fallback. The file is a sorted `(skill, phase)` table of `SkillEffectDescriptor`s;
lookups are a binary search and return `&'static` slices (leaked process-lifetime strings),
so the spawn path does no per-frame or per-layer allocation.

Schema v1 (defined by this lane, consumed by `skill-data`):

```ron
(
    provenance: "...",
    version: 1,
    effects: [
        (
            skill: 34,
            phase: Success,                    // Ground | Cast | Success | Release | Hit | Caster
            cast_circle_color: (255, 230, 160),// optional u8 RGB cast-circle override
            action: Idle,                      // optional CasterAction override
            layers: [                          // EVERY layer of the phase, in order
                (kind: Str, anchor: Overhead, file: "efst_angel_fluttering\\angel_fluttering.str", scale: 1.0, light: (255, 235, 180), light_intensity: 45.0),
                (kind: Spr, anchor: Auto, attached_entity: true, head: true, files: ["이팩트\\축복"], duration_ms: 1500),
                (kind: Texture, anchor: Auto, attached_entity: true, file: "effect\\pok2.tga", size: 22.0, duration_ms: 2500, alpha: 0.3, tint: (0.1, 0.75, 1.0), blend: Additive, wav: "effect\\ef_blessing.wav"),
            ],
        ),
    ],
)
```

Layer kinds:

| kind | renderer | asset field |
| --- | --- | --- |
| `Str` | `.str` screen sprite (`spawn_entity_effect` / `spawn_position_effect`) | `file` (relative to `data\texture\effect\`) |
| `Spr` | animated `.spr`+`.act` (`SpriteEffect`) | `files` (stems relative to `data\sprite\`, first loadable wins) |
| `Texture` | texture billboard/screen effect (`HitFlash` primitive) | `file` (relative to `data\texture\`) |
| `Cylinder` | texture cylinder/ground aura (`SkillAura`) | `file` + radii/height/grow/rotation |
| `Primitive` | `HitFlash` / `Projectile` / `GroundAura` primitive | `file` + `primitive` (inferred by anchor when omitted) |
| `BodyFlash` | caster sprite tint (`Entity::start_body_flash`) | `tint` + `hold_ms` + `fade_ms` |

Anchors: `Caster`, `Target`, `Ground`, `Overhead`, `Body`, `Travel`, `Rain`, `Auto`, plus a
`vertical: Feet | Body | Overhead` override for entity-anchored layers. Placement is pure and
unit-tested (`SkillEffectLayer::placement`): target entity when the packet knows one, then
target cell, then caster; `Travel`/`Rain` are only endpoints for `Projectile` layers and fall
back to target placement for other kinds.

Common per-layer fields: `asset(s)`, `anchor`, `attached_entity`, `head`, `vertical`,
`offset`, `scale`, `size`/`size_end`, `duration_ms`, `delay_ms`, `count`, `spread`,
`height`, `bottom_radius`/`top_radius`/`rotation_speed`/`grow`, `alpha`, `tint`, `blend`,
`wav`, `action`, `light`/`light_intensity`. Optional fields accept both the bare RON spelling
(`anchor: Target`) and `Some(...)`; unknown fields are rejected (`deny_unknown_fields`), so a
typo in an archive override fails loudly instead of silently doing nothing.

`--check`-able schema: the loader exposes `validate`/`validate_compiled`, wired into the
client binary as:

```
korangar --check-skill-effects [PATH]
```

(no window, no network, no archives; exits 1 with the schema error). The generator
`tools/gen_skill_effects.py --check` is the drift guard for the committed file.

## 3. Spawn pipeline

`Client::spawn_skill_effect_phase(skill, phase, source_entity, target_entity, target_position, tick)`
is now the single spawn entry point used by all packet handlers (cast ack, ground cast,
no-damage completion, damage/release/hit/caster). Per phase it:

1. claims the phase in `SkillCompletionTracker` (per-cast de-duplication across duplicate
   packets; the same tracker used before),
2. caps the spawn at `MAX_LAYERS_PER_PHASE = 32` (defensive budget against a bad override;
   logs when truncating),
3. iterates the static layer slice in place and dispatches every layer to its renderer;
   entity-attached layers are added with `EffectHolder::add_unit` (per-entity cap, cleanup on
   entity removal, distance culling) and fixed-cell layers via `add_effect`.
   `Spr` layers honour `count` by spreading copies on the layer spread ring.

Anchors/heights: `Overhead` uses `overhead_effect_height` (21 standing / 12 sitting + 5
clearance, scaled — unchanged behaviour), `Body` uses the new `body_effect_height`
(13 units, sitting-scaled), both live in `world/skill_cast_effect.rs`; the data carries the
official `head` flag instead of any hard-coded skill list.

Integrated but separable:

* **Actions**: `caster_action()` first consults the descriptor `action` override, then the
  existing `skill_actions.ron` table, then the official default (no duplication of the action
  mapping in the effect data).
* **Sounds**: `play_completion_sound`/damage hits call
  `SkillSoundPlayer::play_skill_sound_for_phase`, whose candidates are
  curated `SKILL_SOUNDS` → per-layer `wav` of the phase descriptor → generated
  `skill_sounds.ron` → generic cast sound. The one-sound-lane-per-entity behaviour and the
  `SkillSoundGate` timing (never a pre-cast sound for instant skills) are unchanged.
* **Status auras** (Assumptio EFST 110/473): stay in the status-driven
  `world/effect/status_aura.rs` subsystem (not tied to skill packets); a test keeps it
  reachable and the in-client Assumptio sweep still spawns the one-shot `asum.str` layer plus
  the persistent aura.

## 4. Migration of the hand-written special cases

`tools/skill_effects_custom.ron` is the checked-in, human-readable home of the migrated
hand-written visuals; `tools/gen_skill_effects.py` merges it with the official rows and the
legacy per-kind tables into the single runtime file. Custom layers are additive but
asset-deduplicated: a phase can combine handcrafted and official layers without losing
either, and an official layer with the same asset supersedes a legacy approximation of it.

| old hand-written visual | new data |
| --- | --- |
| `SkillVisuals.completion` (Blessing angel / Angelus / Inc AGI / Concentration) | `Str` layers, `anchor: Overhead` (+ per-layer light), `wav` |
| `SkillVisuals.completion` of Hammer Fall | `Str` `black_hammerfall.str` at the feet + the official `crashearth.str`/hit flash rows (both now spawn) |
| `SkillVisuals.cast_aura` (Zen `EF_BEGINSPELL`) | `Cylinder` layer of the `Cast` phase (`grow: TopRadius`, additive) |
| `SkillVisuals.instant_auras` (Fury `EF_BEGINSPELL` + `EF_GUMGANG2`) | `Cylinder` layers of the `Cast`/`Success` phases (`count: 4`, `delay_ms: 100`, `grow: BothRadii`) |
| `SkillVisuals.target_auras` (Heal `EF_HEAL` columns) | `Cylinder` layers, `anchor: Target`, `attached_entity: true`, `grow: Height`, pale green tint |
| `SkillVisuals.body_flash` (Zen `EF_PORTAL5`) | `BodyFlash` layer of the `Cast` phase |
| `SkillVisuals.cast_circle_color` (Warp purple, buff yellow) | `cast_circle_color` on the phase descriptor |
| `SkillVisuals.light` | per-layer `light`/`light_intensity` |
| `has_custom_completion()` bypass | deleted; all official data rows spawn, so Blessing/Inc AGI/Angelus/Hammer Fall/Concentration/Fury/Zen combine data + handcrafted layers |
| `overhead` guessed by the legacy generator's `OVERHEAD_SKILLS` | official per-layer `head` flag; the audit's three missed skills (59/62/357) are fixed in the custom data (`head: true`), so they no longer render at the feet |

The three official-row tables remain checked in as the migration source of the merged file
(`skill_effect_assets/sprite/procedural.ron`); their old loaders were deleted. The official
fragment (`tools/skill_effects_official.ron`) now supersedes the legacy approximations per
(skill, phase), and `tools/skill_effects_custom.ron` keeps the handcrafted additive layers.

## 5. Files

| file | change |
| --- | --- |
| `korangar/src/world/library/skill_effects.rs` | **new** unified schema, loader, validation, binary-search lookup, placement logic, 16 tests |
| `korangar/archive/data/client/skill_effects.ron` | **new** generated unified data (436 phases / 604 layers) |
| `tools/gen_skill_effects.py` | **new** merge generator (official fragment + custom + legacy), `--check`, `--validate-assets` |
| `tools/skill_effects_custom.ron` | **new** migrated hand-written layers |
| `tools/skill_effects_official.ron`, `tools/windows_client_effect_layers.json`, `tools/gen_skill_effect_layers.py`, `tools/extract_windows_client_effect_layers.py`, `tools/extract_rathena_learnable_skills.py`, `tools/rathena_pre_re_learnable_skills.json`, `tools/skill_effects_official.json`, `tools/test_skill_effect_layers.py`, `tools/windows_client_skill_effects.json`, `notes/skill-effects-official-data.md` | cherry-picked from `skill-data` `e3ae2dce` (official multi-layer rows + generators) |
| `korangar/src/world/skill_cast_effect.rs` | rewritten: overhead/body anchors + cast circle radius only; all visuals moved to data |
| `korangar/src/lib.rs` | unified `spawn_skill_effect_phase` / `spawn_skill_effect_layer`, phase-aware sounds/actions, Spr duplicates, `--check-skill-effects` entry point; all `skill_visuals`/`has_custom_completion` call sites removed |
| `korangar/src/main.rs` | `--check-skill-effects [PATH]` CLI |
| `korangar/src/world/library/{skill_effect_assets,skill_sprite_effects,skill_procedural_effects}.rs` | deleted (folded into `skill_effects.rs`) |
| `korangar/src/world/{mod.rs,skill_action.rs,skill_sound.rs}` | exports + action/sound integration |
| `korangar/src/world/effect/{mod.rs,sprite_effect.rs,procedural/*,skill_aura.rs}` | primitives take borrowed `PrimitiveParams`/`SpriteEffectParams` (no per-spawn allocation), sprite tint/additive, cylinder grow from data |

## 6. Tests and evidence

Unit/integration tests (`cargo test -p korangar --lib`, full suite):

* unified loader: compiled fallback valid + broad (>250 phases), multi-layer phase of
  different kinds (Blessing Str+Spr+Texture from a fixture **and** the committed data),
  `head` → overhead anchor, anchor/placement fallback matrix, cylinder growth/blend/count,
  cast-circle/action/sound fields, loader fallback and malformed/unknown-field rejection,
  `deny_unknown_fields`, sorted/unique binary-search table, status-aura reachability.
* overhead/body anchor tests (`skill_cast_effect.rs`), sprite playback timing, primitive
  timing/fade tests, skill action/sound gating tests (kept).

Data/schema evidence:

| check | result |
| --- | --- |
| `./tools/gen_skill_effects.py --check` | `skill_effects.ron is up to date (436 phases, 604 layers)` |
| `./tools/gen_skill_effects.py --validate-assets /home/phantom/Games/ragnarok/CondaRO-linux/client/game` | clean (all Str/Spr/Texture/Cylinder/Primitive assets + wavs exist in the GRFs) |
| `korangar --check-skill-effects` (compiled fallback) | `436 (skill, phase) descriptors are valid` |
| `korangar --check-skill-effects <archive override>` | same for a merged/`--official` file, exits 1 on malformed data |
| `./tools/gen_skill_effect_layers.py --check` (skill-data) | `up to date: 351 rows, 511 layers` |
| `python3 tools/test_skill_effect_layers.py` | 9 tests OK |
| legacy generators `--check` (5 scripts) | all still up to date with the extended facts JSON |
| merge with official fragment | 436 phases / 604 layers, asset validation clean |

In-client (muted Linux/Xvfb, GM test characters only, no Wine) — see section 7.

## 7. In-client verification

In-client (muted Linux/Xvfb, GM test character `verify4`/`VerifyBot4`, no Wine, audio
muted, Xvfb `:94` 1280x720x24, 30 fps, no user character/client touched):

* All six phases of 16 skills (`34,29,33,45,28,110,270,401,59,62,357,361,89,14,5,70`) were
  spawned in the real client through the unified `spawn_skill_effect_phase` path in one
  session: 53 `skill effect:` spawn lines, no panic, no missing asset, all layers of the
  multi-layer phases appeared (Blessing: `.str` overhead angel + official `축복` SPR + 6-copy
  particle6 sprites ×2 + `pok2.tga`; Inc AGI: 3× `ac_center2.tga` + `agi_up.bmp` + hand-written
  sparkle; Heal: `ring_white` cylinders ×2 + `pok3` bursts; Hammer Fall: `crashearth.str` +
  `black_hammerfall.str`; Zen: `EF_BEGINSPELL` cylinder + body flash; Fury rings; Assumptio
  `asum.str`; Storm Gust `.str` + procedural cloud).
* Evidence directory: `notes/korangar-skill-effect-pipeline-evidence/`
  (`README.md`, `skill-sweep-annotated.jpg`, `skill-sweep-montage.jpg`,
  `skill-sweep-overhead-crops.jpg`, selected frames, `client-log-skill-sweep.txt`).
* Login/character selection ran through the committed benchmark driver; the XTEST chat command
  path was unreliable for this session (chat input focus), so a **temporary**
  `KORANGAR_SKILL_SWEEP` hook in `benchmark.rs` drove the real spawn function for the capture.
  The hook was removed before hand-off (`korangar/src/benchmark.rs` is unchanged in the final
  commit), matching the precedent of earlier lanes documenting a removed verification hook.
* Binary used for the capture: release build sha256 prefix `939baa960de3e6ae` (harness build,
  not the hand-off binary). The hand-off release binary (final code) is sha256 prefix
  `730f311f2fb0440f`.
* Assumptio persistent status aura: not part of this capture (status-driven subsystem); the
  one-shot `asum.str` layer spawned as expected and the status-aura unit tests stay green.


## 8. CI parity

| check | command | result |
| --- | --- | --- |
| formatting | `cargo fmt --all --check` | pass |
| clippy (default) | `cargo clippy --workspace -- -Dwarnings` | pass |
| clippy (all features) | `cargo clippy --workspace --all-features -- -Dwarnings` | pass |
| release build | `cargo build --release` | pass (binary sha256 `730f311f2fb0440f...`) |
| unit tests | `cargo test -p korangar --lib` | 435 passed, 0 failed, 11 ignored |
| tests (default) | `cargo test -p korangar -p korangar-networking -p ragnarok-packets` | pass (see note) |
| tests (all features) | `cargo test --all-features -p korangar -p korangar-networking -p ragnarok-packets` | pass (see note) |
| data drift | `python3 tools/gen_skill_effects.py --check` | up to date (436 phases, 604 layers) |
| official fragment drift | `python3 tools/gen_skill_effect_layers.py --check` | up to date (351 rows, 511 layers) |
| official fragment tests | `python3 tools/test_skill_effect_layers.py` | 9 tests OK |
| legacy generators | `tools/gen_skill_{effect_assets,sprite_effects,procedural_effects,sounds,actions}.py --check` | all up to date |
| schema CLI | `korangar --check-skill-effects` | `436 (skill, phase) descriptors are valid` |
| asset validation | `tools/gen_skill_effects.py --validate-assets <game dir>` | clean |

**Pre-existing integration failure (not caused by this branch):** the source-scan test
`korangar/tests/ui_consistency.rs::hardcoded_style_literals_do_not_exceed_the_token_budget`
reports **29** hardcoded color literals against a ratchet of **28** at the branch point
`e87268af` already, and also at the current integration HEAD (`e3ae2dce`) in the main tree:

```
$ git show e87268af:korangar/test-data/ui-token-budget.ron   # max_color_literals: 28
$ git grep -h "Color::rgb_u8\\|Color::rgba_u8\\|Color::monochrome_u8" e87268af -- korangar/src/interface | wc -l
29
$ git diff e87268af --stat -- korangar/src/interface        # empty for this branch
```

The extra literals come from other integration lanes (`interface/windows/chat.rs`,
`interface/windows/character_selection.rs`), not from this work. The test suites above were
run with that single pre-existing test skipped (`--skip
hardcoded_style_literals_do_not_exceed_the_token_budget`) to demonstrate everything else is
green; the ratchet needs a fix in the integration tree (attribute the literal to a design
token or raise the ratchet with a justification — the file says only lower it).

### Combined SHA / hand-off

* This branch (`feat/skill-effect-pipeline`) contains the pipeline feature commit plus the
  cherry-picked official-data commit **`e3ae2dcef4368e0cd4733ee984bd866263dcc305`**
  (`skill-data`, fragment + generators + facts). The hand-off commit is the branch tip; see the
  lane report for the exact SHA.
* Fast-forwarding `--official tools/skill_effects_official.ron` is already the
  generator default, so `./tools/gen_skill_effects.py` reproduces the committed
  `skill_effects.ron` from the committed fragment.


## 9. Out of scope / follow-ups

* Official hard-coded `FUNC`/`HARDCODED` components of the effect table are still covered by
  the procedural lane (`HitFlash`/`Projectile`/`GroundAura` approximations); the fragment
  intentionally omits CALLBACK/FUNC layers (they are not data assets).
* `min` (minimap/min-effect) alternates are not represented; `min` assets are not loaded.
* Official `duplicate`/`delay` on `Spr` layers is played as a spread ring of `count` copies;
  per-copy start delays are not modelled (no delayed-spawn effect exists in the pipeline).
* Persistent status auras for Blessing/Inc AGI/Angelus/Concentration (EFST 10/12/9/3) are not
  implemented — `status_aura.rs` only knows Assumptio (EFST 110/473). That is a status
  subsystem follow-up, not part of the skill packet pipeline; the audit calls it out in
  section 3.6.
* Visual tuning of the official sizes/tints was done by `skill-data` in world-unit factors;
  the audit's `WRONG_ASSET` places where a hand-written placeholder and the official layer
  now both spawn (Blessing angel_fluttering + official `축복`, Hammer Fall
  `black_hammerfall.str` + `crashearth.str`, Inc AGI sparkle + official textures) can be
  reduced to the official layer only with a one-line custom-data deletion once screenshots
  are signed off. Combined spawning is intentional for this migration (no visual loss), and
  the official layer is never suppressed.
