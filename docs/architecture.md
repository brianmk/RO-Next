# Architecture

How the RO-Next workspace is laid out, what each crate owns, and where the
current structural debt sits. Keep this file current when a crate boundary or a
top-level module moves.

## Workspace

19 crates: the client (`ro-next`), nine supporting `ro-next-*` crates (plus four
nested proc-macro/component crates) and four protocol crates that are
independent of the client.

```
                        ragnarok-packets ── ragnarok-formats ── ragnarok-bytes
                                │                   │                │
                                └──────────┬────────┴────────────────┘
                                           ▼
   ro-next-interface ◄── ro-next-debug ◄── ro-next-container ◄── ro-next-collision
          ▲                   ▲                    ▲
          │                   │                    │
   ro-next-components   ro-next-networking ◄──── ro-next-audio
          ▲                   ▲                    ▲
          └───────────┬───────┴────────┬───────────┘
                      ▼                ▼
                  ro-next-messages  ro-next-loaders
                      │
                      ▼
                    ro-next  (binary + lib)
```

| crate | lines | owns |
| --- | --- | --- |
| `ro-next` | ~129k | client loop, rendering, world simulation, interface, everything game-specific |
| `ro-next-interface` | 11.1k | retained-mode UI framework (elements, windows, themes, layout) |
| `ro-next-networking` | 10.4k | packet versioning, crypto, login/char/map sessions |
| `ragnarok-packets` | 14.6k | packet definitions, opcode registries, serialization |
| `ro-next-audio` | 5.2k | sound/music engine, volume control |
| `ro-next-collision` | 1.9k | AABB, KD-tree, spheres |
| `ro-next-container` | 1.8k | slab/cache containers |
| `ragnarok-formats` | 1.8k | GRF, GAT/GND/RSW/RFM, sprites, ACT/SPR |
| `ragnarok-bytes` | 1.6k | byte/euc-kr/packet IO primitives |
| `ragnarok-macros` | 1.1k | derive macros for the packet/format types |
| `ro-next-messages` | 1.3k | protocol result-code message table |
| `ro-next-video` | 1.3k | AV1 video playback |
| `ro-next-debug` | 1.1k | profiling + debug logging macros |
| `ro-next-components` | 0.6k | shared UI component macros (inside `ro-next/components`) |
| `ro-next-loaders` | 25 | the `FileLoader` trait |
| `interface-macros`, `interface-components`, `interface-component-macros`, `debug-macros` | | proc macros for the UI crates |

Dependency rules that hold today:

* `ragnarok-*` never depend on `ro-next-*` (they are usable by other projects).
  The one exception is a deliberate `interface` feature on `ragnarok-formats`
  and `ragnarok-packets` for the packet browser.
* Only `ro-next` depends on every other crate; no crate depends on `ro-next`.
* No dependency cycles.

## Inside `ro-next`

```
src/
  main.rs            CLI entry point (clap), calls Client::init / run
  lib.rs             Client struct + the two event dispatchers (see debt below)
  client_config/     data-file loaders + `--check-*` validators (world, keybinds,
                     paper dolls, debug maps, ui audio, ...)
  networking/        map/login session glue on top of ro-next-networking
  loaders/           archive/GRF, fonts, maps, sprites
  renderer/          wgpu render pass orchestration
  graphics/          engine, passes, effects buffers, camera
  world/             entities, maps, skills, effects, animation, particles
  interface/         windows (one file per window) + components + markup
  state/             ClientState: every UI/world sub-state, localization, theme
  settings/          persisted settings files (`client/*.ron` next to archive/)
  input/             input mapping, chat commands, battle mode
  perf/ benchmark.rs profiling and the benchmark entry points
  msgstringtable.rs  generated message-string table
```

Data lives in `ro-next/archive/` (shipped next to the binary): `data/client/*.ron`
runtime tables, `data/languages/`, sprites and fonts. Every table is generated
from the game archives — see [`tools/README.md`](../tools/README.md) for the
extract → `tools/data/` → generate pipeline and the per-table inputs.

## Tests

* 720 unit tests inline in `ro-next` (`#[cfg(test)]` next to the code), 82 in
  `ro-next-networking`.
* Integration tests in `ro-next/tests/`: message tables, skill behaviour, world
  audio data, UI consistency (each one re-reads the committed data file).
* 12 Python suites for the data pipeline (`tools/test_*.py`) plus `--check` drift
  guards for all 19 generators.
* `cargo clippy --workspace -- -Dwarnings` and `--all-features` are part of the
  gate; an `#[allow]` needs a one-line reason on the attribute.

## CI

| workflow | purpose |
| --- | --- |
| `quality.yml` | format, clippy (default + all features), 2019 packet contract, tests (default + all features), `--check-*` data entry points, generated-data drift for the offline generators, Nix devShell |
| `release.yml` | tag `vX.Y.Z-YYYYMMDD` → builds Linux/Windows/macOS archives and publishes them |
| `ui-golden.yml` | nightly/manual golden-screenshot comparison (non-blocking) |

The fork previously carried the upstream `build.yml`/`formatting.yml`/`lint.yml`/
`tests.yml` as well; they only triggered on a `main` branch that does not exist
here and duplicated `quality.yml`, so they were removed (the Nix job moved into
`quality.yml`).

## Structural debt

Ranked by cost; the first two are the reason a small feature still touches a
14k-line file.

### 1. `ro-next/src/lib.rs` is a 14.9k-line file (11.5 % of the crate)

The `impl Client` block alone is 12.3k lines / 112 methods, and two functions
hold most of it:

| function | lines | what it is |
| --- | --- | --- |
| `handle_network_events` | ~4.3k | one `match` over `NetworkEvent`, bodies inline |
| `process_user_events` | ~2.4k | one `match` over `UserEvent`, bodies inline |

Proposed split (mechanical, compiler-checked, one module at a time):

```
src/client/
  mod.rs            Client struct, fields, init(), run loop
  frame.rs          update_* / log_* / scaling / presentation
  network/mod.rs    handle_network_events: dispatch only
  network/chat.rs   network handlers per family
  network/{party,guild,inventory,skill,entity,map,mail,trade}.rs
  input/mod.rs      process_user_events: dispatch only
  input/{chat,chat_room,skill_tree,inventory,storage,party,menu}.rs
  effects.rs        spawn_* helpers (skill/status/projectile/teleport effects)
  navigation.rs     nav route helpers
  hud.rs            open_*/toggle_* window helpers
```

Each file gets `impl Client { ... }` (inherent impls may live in any module of
the crate) and its own imports; `lib.rs` keeps the module declarations, the
`Client` struct and the lifecycle. Start with `effects.rs` and `navigation.rs`
(contiguous, few `self` fields), then the two dispatchers.

### 2. The two dispatcher `match`es have no per-arm tests

Because the arm bodies are inline, only the whole-event path is testable.
Extracting each arm into `fn handle_<thing>(&mut self, ...)` makes the handlers
unit-testable without a client instance (pass the extracted data).

### 3. `ro-next-loaders` is a 25-line crate

It only carries `FileLoader` + `FileNotFoundError` and is a leaf in the graph.
Merging it into `ro-next-container` (also a leaf, no deps) would remove a crate
boundary without touching any call site's imports beyond the name.

### 4. Crate naming is split between `ro-next-*` and upstream names

`interface-macros`, `interface-components`, `interface-component-macros` and
`debug-macros` kept their upstream package names. Renaming them to
`ro-next-interface-macros` etc. is a `Cargo.toml` + `path` change with no code
impact; it has not been done because the churn outweighs the benefit today.

### 5. Tool scripts duplicate small helpers

`load_json`/`load_facts` are re-implemented in 12 scripts, `ron_string` in 3.
A `tools/lib/facts.py` + `tools/lib/ronfmt.py` would deduplicate them; the
scripts stay flat because their prefixes (`extract_`/`gen_`/`audit_`/`test_`)
already group them and every path is CWD-relative by design (they run from the
repository root, as in CI).

### 6. UI design gates are red (localization debt)

`ro-next/tests/ui_consistency.rs` enforces that user-facing windows use the
localization table and the design tokens. Four assertions fail today, all from
windows added by the gap/feature work without going through the table:

| test | violations |
| --- | --- |
| `no_hardcoded_window_titles_in_user_facing_windows` | 14 titles (vending, quest log, trade ×2, rodex, guild, notifications, bank, refine, ...) |
| `no_hardcoded_button_words_in_user_facing_windows` | 22 button labels |
| `no_hardcoded_ui_words_outside_the_interface` | literals in `ro-next/src` outside `interface/` |
| `hardcoded_style_literals_do_not_exceed_the_token_budget` | 6 colour literals against a budget of 0 |

The fix is mechanical: add the strings to `state/localization/texts.rs` and both
`archive/data/languages/*.ron` files, then reference them from the windows; the
colour literals map onto existing theme tokens. Until then the strict gate
cannot be green, so the two dispatcher functions currently carry the only
comfortable place to work without tripping it.

### 7. Housekeeping

* 147 local branches, 139 of them already merged into `HEAD`; the unmerged ones
  are old experiments kept as scratch worktrees.
* The GitHub fork is still named `brianmk/korangar`; renaming it there is a
  repository-settings change, not a git one.
