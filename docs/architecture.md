# Architecture

How the RO-Next workspace is laid out, what each crate owns, and where the
current structural debt sits. Keep this file current when a crate boundary or a
top-level module moves.

> Line counts, file lists and dependency claims below were last verified on
> 2026-11-24 with `wc -l` per crate/module, `cargo metadata` and a full
> `cargo test --workspace`; re-check them when you move code.

## Workspace

19 packages: the client (`ro-next`), ten supporting `ro-next-*` crates, four
nested proc-macro/component crates and four protocol crates that are usable
without the client.

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
| `ro-next` | 132.6k (132,644) | client loop, rendering, world simulation, interface, everything game-specific |
| `ro-next-interface` | 11.1k | retained-mode UI framework (elements, windows, themes, layout) |
| `ro-next-networking` | 10.7k | packet versioning, crypto, login/char/map sessions |
| `ragnarok-packets` | 14.8k | packet definitions, opcode registries, serialization |
| `ro-next-audio` | 5.2k | sound/music engine, volume control |
| `ro-next-collision` | 1.9k | AABB, KD-tree, spheres |
| `ro-next-container` | 1.8k | slab/cache containers |
| `ragnarok-formats` | 1.8k | GRF, GAT/GND/RSW/RFM, sprites, ACT/SPR |
| `ragnarok-bytes` | 1.6k | byte/euc-kr/packet IO primitives |
| `ragnarok-macros` | 1.1k | derive macros for the packet/format types |
| `ro-next-messages` | 1.3k | protocol result-code message table |
| `ro-next-video` | 1.3k | AV1 video playback |
| `ro-next-debug` | 1.1k | profiling + debug logging macros |
| `ro-next-components` | 26 | shared UI component macros (inside `ro-next/components`) |
| `ro-next-loaders` | 25 | the `FileLoader` trait |
| `interface-macros`, `interface-components`, `interface-component-macros`, `debug-macros` | | proc macros for the UI crates |

Dependency rules that hold today:

* `ragnarok-*` never depend on the client crate (`ro-next`); they are usable by
  other projects. Two leaks exist today: `ragnarok-packets` requires
  `ro-next-debug` for its logging macros, and `ragnarok-formats` +
  `ragnarok-packets` optionally depend on `ro-next-interface` behind their
  `interface` feature (packet/sprite browser support).
* Only `ro-next` depends on every other crate; no crate depends on `ro-next`.
* No dependency cycles.

## Inside `ro-next`

```
src/
  main.rs            CLI entry point (clap), calls Client::init / run
  lib.rs             module declarations, crate-wide `pub(crate) use` prelude and
                     the re-exports the binary needs (488 lines)
  client/            everything else that is Client behaviour (see below)
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
  login_server/      login-server address discovery and ping
  pincode/           PIN-code dialog state
  system/            OS-level helpers (window backend, desktop session)
  perf.rs            frame/profile counters
  benchmark.rs       benchmark entry points
  crash.rs           panic hook and crash reports
  logging.rs         log setup
  msgstringtable.rs  generated message-string table
```

`src/client/` is split by concern, one `impl Client` block per file:

| file | lines | contents |
| --- | --- | --- |
| `frame.rs` | 2,067 | everything per-frame: focus/pacing/fps logging, the winit `ApplicationHandler`, simulation (`update_entities`, camera, attack checks, buffered actions), overlay renderers, `update_and_render` |
| `mod.rs` | 1,000 | the `Client` struct and fields, constants, event-loop/tls/shutdown helpers, `PendingExperience`, message tables, frame profiler |
| `lifecycle.rs` | 917 | construction: `init`, `run`, client-state/settings updates |
| `gameplay.rs` | 767 | casting, aftercast timers, status/aura updates, skill-tree/experience bookkeeping |
| `render.rs` | 713 | `MapRenderContext` + per-frame overlay renderers |
| `effects.rs` | 711 | skill effect phases/layers, auras, projectiles, cast circles |
| `user_events.rs` | 471 | `process_user_events`: routing only (the 22 debug camera arms stay inline: they take `delta_time`) |
| `loaders.rs` | 450 | loader bookkeeping, transient-failure retries, character-selection previews |
| `hud.rs`, `navigation.rs`, `chat.rs` | 438 (167/188/83) | window toggles, nav routes, chat push helpers |
| `items.rs` | 364 | item menus, info windows, artwork, cutin/book loading |
| `windows.rs` | 288 | open/close trade, storage, bank, cart, item-service, refining, vending; character-selection re-entry |
| `network/mod.rs` | 281 | `handle_network_events`: routing only |
| `network/{session,character,world,skills,inventory,commerce,social,ui}.rs` | 5.9k | 224 handlers (24/20/37/21/17/52/29/24) |
| `input_*.rs` | 3.3k | 158 handlers in 13 files |

Handler convention: a handler takes the whole event and destructures it inside
(`let NetworkEvent::X { .. } = event else { unreachable!() }`), so the dispatcher
never repeats field types and handlers are unit-testable without a `Client`
event buffer. `continue` in a moved arm body became `return` (same semantics:
skip the rest of this event).

## Other split modules

| module | layout |
| --- | --- |
| `ragnarok-packets` | `lib.rs` (1,362: traits, handler/limits, opcode registry, tests) re-exports `primitives`, `login`, `character`, `entity`, `map`, `inventory`, `skill`, `social`, `commerce`, `mail`, `ui`, `misc` |
| `ro-next-networking` | `lib.rs` (2,001) + `packet_versions/version_20220406.rs` (412) whose 219 registrations live in `version_20220406/registration/{session,world,inventory,skills,social,commerce,ui,misc}.rs`; unit tests in `src/tests/packet_handlers.rs` |
| `world/entity` | `mod.rs` (340, value types) + `common.rs` (1,562) + `actor.rs` (797) + `player.rs` (364) + `npc.rs` (115) |

## Tests

`cargo test --workspace` (unit tests, integration suites and doctests):

| crate / suite | tests |
| --- | --- |
| `ro-next` unit tests (inline `#[cfg(test)]`) | 719 (+21 ignored) |
| `ro-next-networking` unit tests | 82 |
| `ragnarok-packets` unit + `contract_2019` | 73 + 10 |
| `ragnarok-bytes`, `ragnarok-formats` | 47, 11 |
| `ro-next/tests/message_tables.rs` | 4 |
| `ro-next/tests/skill_behaviour.rs` | 7 |
| `ro-next/tests/world_audio_data.rs` | 7 |
| `ro-next/tests/ui_consistency.rs` | 5 (4 fail — see debt #6) |

The integration suites are repo-level guards: `message_tables`,
`skill_behaviour` and `world_audio_data` re-read the committed data files and
scan the dispatch sources; `ui_consistency` scans the window sources for
hardcoded strings and design tokens. All of them read the tree at test time, so
a guard keeps working when code moves to another module.
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

Ranked by cost after the module splits.

### 1. The handlers have no per-arm tests yet

The 382 per-variant handlers are plain `impl Client` methods that receive their
event, so the pure ones can be tested without a client instance. None are yet.

### 2. Files that are still large

| file | lines | note |
| --- | --- | --- |
| `ragnarok-packets/src/opcode_lengths_2019.rs` | 3,209 | generated table, leave it |
| `client/frame.rs` | 2,067 | the frame pipeline is cohesive; splitting it by phase (simulation / rendering / presentation) is the next candidate |
| `graphics/mod.rs` / `graphics/engine.rs` | 1,901 / 1,809 | four impls each, largest function 531 |
| `world/library/skill_effects.rs`, `world/jobs.rs` | 1,607 / 1,600 | data loaders + validation, cohesive |
| `ro-next-audio/src/lib.rs` | 1,572 | one engine |

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
| `no_hardcoded_window_titles_in_user_facing_windows` | 13 titles (vending, quest log, trade ×2, rodex, guild, notifications, bank, refine, chat room, emotes, pin, clan) |
| `no_hardcoded_button_words_in_user_facing_windows` | 21 button labels |
| `no_hardcoded_ui_words_outside_the_interface` | 4 literals (`input/keybinds.rs`, `state/item_service.rs`, `state/mod.rs` ×2) |
| `hardcoded_style_literals_do_not_exceed_the_token_budget` | 6 colour literals against a budget of 0 |

The fix is mechanical: add the strings to `state/localization/texts.rs` and both
`archive/data/languages/*.ron` files, then reference them from the windows; the
colour literals map onto existing theme tokens. Until then the strict gate
cannot be green — this is the only failing test suite in the workspace.

### 7. Housekeeping

* 147 local branches, 139 of them already merged into `HEAD`; the unmerged ones
  are old experiments kept as scratch worktrees.
* The GitHub fork is still named `brianmk/korangar`; renaming it there is a
  repository-settings change, not a git one.
