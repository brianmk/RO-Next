# Performance benchmark, budgets and regression gate

Repeatable headless measurement of the ro-next client frame cost, allocation
churn and async-loader latency, plus the documented runtime budgets that keep
crowded towns (population-engine shells, effect bursts) bounded.

This README is the operator documentation: how to run the benchmark, how to
read the numbers, how to update the baseline and how to run the regression gate.

---

## 1. What is measured

`scripts/perf/perf-baseline.sh` starts the **release** client on a muted Xvfb
display with a fixed lean profile and the `RO_NEXT_BENCH` scenario driver
(`ro-next/src/benchmark.rs`). The driver logs in with a test account and runs
these stages in order:

| Stage | Content |
|---|---|
| `login` | log in, select the first character server + character, enter the map |
| `town_idle` | stand in the town with the population engine crowd around (8 s) |
| `town_walk` | walk a fixed loop through the crowded plaza (12 s, a move every 700 ms) |
| `effects` | burst of 240 world effects (a third spawned far outside the culling distance) and 360 damage/heal/miss numbers; the second half lets them animate |
| `ui_updates` | open chat/hotbar + inventory/equipment/stats/skill tree/minimap, inject 10 known buffs, invalidate all window layouts every frame (8 s) - the worst case for buff bar labels + layout churn |
| `map_change` | request three map loads through the real async loader (`izlude`, `alberta`, `prontera`) and record completion latency |

For every stage the harness prints one `[bench-stage] {...}` JSON line with:

| Field | Meaning |
|---|---|
| `frames`, `duration_s` | rendered frames and wall duration of the stage |
| `frame_ms_mean/p50/p95/max` | **client CPU time of the frame loop** (not wall FPS; GPU wait is part of `gpu_ms`) |
| `wait_ms` | swapchain acquire wait (blocked on GPU/display) |
| `sim_ms` | network/event/state/entity/camera/light simulation |
| `interface_ms` | window layout + input + interface instruction generation |
| `world_ms` | map/entity/shadow/effect/overlay instruction generation |
| `gpu_ms` | `render_next_frame` submission |
| `allocs_per_frame`, `deallocs_per_frame`, `alloc_kib_per_frame` | heap allocations counted by the wrapper allocator in `ro-next/src/perf.rs` (enabled only in perf/benchmark mode) |
| `effects_mean`, `particles_mean`, `entities_mean`, `windows_mean` | average object counts |
| `effects_rendered_mean`, `effects_culled_mean`, `particles_rendered_mean`, `particles_culled_mean` | rendered vs. distance-culled objects per frame (budget/knob verification) |
| `effects_max`, `particles_max`, `entities_max`, `windows_max` | peaks; `effects_max` staying at the cap proves the budget is active |

Additionally:

* `[bench-load] {...}` per stage: async loader submissions/completions/failures,
  coalesced duplicate requests and queue/service p95 latencies per load kind
  (`map`, `animation`, `item_sprite`, `skill_sprite`, `skill_actions`).
  Queue latency is submit -> worker start (QoS: the map lane must stay at ~0
  even while sprite loads are queued); service latency is worker start ->
  completion.
* `[bench-map] {"map":...,"ms":...}` per map load: submit -> the frame that
  applied the loaded map.
* `[bench-end] {"ok":true,...}` when the scenario completed; missing means the
  run failed (the script fails the run in that case).
* `[budget] effects=... effects_dropped=... particles=... particles_dropped=...`
  every 2 s in `RO_NEXT_PERF_LOG` mode: live budget telemetry.
* `[perf]`/`[perf-phase]`/`[loader]` lines every 2 s (the pre-existing
  `RO_NEXT_PERF_LOG` interval logging, extended with the loader/budget lines).

The script repeats runs (default 2) and writes **per-stage medians** to JSON
together with the git SHA, binary SHA-256, host load average and the fixed
profile parameters.

### Fixed benchmark profile

Committed under `scripts/perf/profile/` and copied into `<repo>/ro-next/client`
before every run (the previous files are backed up once as `*.perf-backup`):

* 1024x600 window, 30 fps cap (also for background/unfocused), vsync off,
  immediate present mode
* no MSAA/SSAA/FXAA, hardest shadows/lowest shadow detail, classic lighting,
  no high-quality interface
* UI scale 0.75, English, default (unskinned) interface/menu themes
* audio muted (`RO_NEXT_MUTE_AUDIO=1` + null ALSA default), forced-unfocused
  window handling so Xvfb renders continuously

Everything is set through the committed profile plus the documented
environment overrides (`RO_NEXT_RESOLUTION`, `RO_NEXT_UI_SCALE`,
`RO_NEXT_MAX_FPS`, `RO_NEXT_BACKGROUND_MAX_FPS`, `RO_NEXT_MUTE_AUDIO`).

---

## 2. Prerequisites

* Linux with X11 tooling: `Xvfb`, `xdpyinfo`.
* Game data available in the client crate directory: `ro-next/data.grf`,
  `ro-next/CondaRO.grf`, `ro-next/palettes.grf` (symlinks to the client data
  are fine) and a `ro-next/client/` directory (created by the first client
  start).
* A test account on the development server. Credentials are **never** stored in
  this repository; pass them through the environment:

  ```bash
  export PERF_USER=...   # test account username
  export PERF_PASS=...   # test account password
  ```

  The scenario selects character slot 0 (`RO_NEXT_BENCH_SLOT` to change).
  Use a dedicated test character that can be left anywhere in Prontera; the
  population-engine shells make the town representative.

## 3. Running

```bash
# build (incremental) + 2 benchmark runs + per-stage medians
PERF_USER=... PERF_PASS=... scripts/perf/perf-baseline.sh

# same, without building, with a specific binary, more runs, verbose logs kept
PERF_USER=... PERF_PASS=... scripts/perf/perf-baseline.sh \
    --no-build --binary target/release/ro-next --runs 3 --keep-logs
```

Useful options (full list: `--help`): `--out <file>`, `--scenario <name>`,
`--runs <n>`, `--display :97`, `--res 1024x600`, `--timeout <seconds>`.

The script prints a stage table and writes the JSON baseline. Example table
(the committed baseline: integration HEAD `989d281` + the measurement harness,
2-run medians, lean Xvfb profile):

```
stage         frames      p50      p95      max      sim    iface    world      gpu  allocs/f    KiB/f  effects  culled particles  entities
login            980    34.21    45.18    73.60     0.04     0.01     0.25    20.07     486.0    177.8      0.0     0.0       0.0       0.0
town_idle        238    33.61    44.94    51.99     0.10     0.04     0.36    21.84    2625.9    879.1      1.0     0.0       0.0      28.9
town_walk        356    34.05    44.17    51.81     0.05     0.04     0.36    21.73    1201.6    544.4      1.0     0.0       0.0      29.4
effects          179    33.60    42.46    50.29     0.09     0.04     0.58    20.99    1018.5    568.2    237.8     0.0      86.6      29.6
ui_updates       238    34.14    43.65    51.30     0.07     0.17     0.59    20.30     821.0    535.4    197.3     0.0       0.0      31.3
map_change        28    33.81    46.80    56.54     1.17     0.09     0.48    19.79   23202.8  18493.3      3.2     0.0       0.0      30.1
```

The absolute values depend on the host (a heavily loaded machine raises the
p95/max columns and map load latencies). The gate only compares relative
changes against this committed baseline.

## 4. Regression gate

`scripts/perf/perf-gate.sh` runs the same benchmark and compares the fresh
per-stage medians against the committed `scripts/perf/baseline.json`.

```bash
# compare (non-blocking: prints regressions, exits 0)
PERF_USER=... PERF_PASS=... scripts/perf/perf-gate.sh

# make regressions fail the job (used by the optional nightly workflow)
PERF_USER=... PERF_PASS=... scripts/perf/perf-gate.sh --enforce

# compare an already measured run
scripts/perf/perf-gate.sh --current /tmp/ro-next-perf.json

# re-measure and replace the committed baseline (commit the JSON afterwards)
PERF_USER=... PERF_PASS=... scripts/perf/perf-gate.sh --update
```

Gated metrics and relative tolerances (override with environment variables):

| Metric | Env | Default |
|---|---|---|
| `frame_ms_mean` | `PERF_GATE_TOLERANCE_FRAME_MEAN` | +15 % |
| `frame_ms_p95` | `PERF_GATE_TOLERANCE_FRAME_P95` | +25 % |
| `interface_ms` | `PERF_GATE_TOLERANCE_INTERFACE` | +25 % |
| `world_ms` | `PERF_GATE_TOLERANCE_WORLD` | +25 % |
| `allocs_per_frame` | `PERF_GATE_TOLERANCE_ALLOC` | +30 % |

Stages with fewer than `PERF_GATE_MIN_FRAMES` (default 60) measured frames are
reported but not compared: p95 over a ~30 frame `map_change` stage is dominated
by the host scheduler, not by the client. A missing stage, a benchmark error or
a missing `[bench-end]` line always fails the gate (even in non-blocking mode it is reported). Non-blocking is the default
because headless lavapipe frame times vary with host load; `--enforce` is meant
for a nightly job on a quiet runner.

### CI integration (optional, non-blocking first)

`.github/workflows/` is shared with the other P0 workstreams, so this lane is
intentionally not added here. The workflow job is:

```yaml
name: Performance regression gate
on:
  workflow_dispatch:
  schedule:
    - cron: "0 3 * * *"   # nightly

jobs:
  perf:
    runs-on: [self-hosted, linux, x11]   # needs Xvfb + game data + test account
    steps:
      - uses: actions/checkout@v4
      - name: Build release client
        run: cargo build --release
      - name: Run performance gate
        env:
          PERF_USER: ${{ secrets.PERF_USER }}
          PERF_PASS: ${{ secrets.PERF_PASS }}
          PERF_GATE_ENFORCE: ${{ vars.PERF_GATE_ENFORCE || '0' }}
        run: scripts/perf/perf-gate.sh
```

Start with `PERF_GATE_ENFORCE=0` (default) so it only reports; flip the variable
to `1` once several consecutive nightly runs are inside the tolerances.

---

## 5. Runtime budget knobs

The effect/particle budgets and distance culling are environment knobs so they
can be tuned per machine without a rebuild. `0` disables the respective budget
or the culling (unbounded - not recommended for crowded towns).

| Variable | Default | Meaning |
|---|---|---|
| `RO_NEXT_MAX_EFFECTS` | `200` | maximum simultaneously alive world effects (`.str` effects, auras, cast circles, warp/teleport effects); new effects are dropped and counted in `[budget]` |
| `RO_NEXT_MAX_EFFECTS_PER_ENTITY` | `24` | maximum effects attached to one entity, so one player/shell cannot use the whole global budget |
| `RO_NEXT_EFFECT_CULL_DISTANCE` | `1800` | world units (1 map cell = 5 units) beyond which effects are not rendered and their point lights are not registered |
| `RO_NEXT_MAX_PARTICLES` | `300` | maximum simultaneously alive particles (damage/heal/miss numbers, emotes); the oldest particle is recycled |
| `RO_NEXT_PARTICLE_CULL_DISTANCE` | `2200` | world units beyond which particles are not rendered |

Telemetry: `[budget]` in `RO_NEXT_PERF_LOG` mode, and
`effects_max`/`effects_culled_mean`/`particles_max`/`particles_culled_mean` in
the benchmark stage lines.

## 6. Updating the baseline

1. Measure on the same machine class/display as the previous baseline and make
   sure no other heavy build is running (`uptime`).
2. `PERF_USER=... PERF_PASS=... scripts/perf/perf-gate.sh --update` (2 runs).
3. Inspect the table: the change must be explainable (e.g. a new feature costs
   `world_ms`; that is expected and is why the baseline must be updated in the
   same PR as the feature). Do not update the baseline to hide a regression.
4. Commit `scripts/perf/baseline.json` together with the change and mention the
   reason in the commit message.

## 7. Troubleshooting

* **`.bench` file only has `[bench-start]`** - the client never reached the
  login window; check the game data symlinks and `ro-next/client/`.
* **login stage takes a long time** - the test account is still marked online
  from a previous hard-killed run, or the account is shared with another worker.
  Wait for the server session timeout or use another test account.
* **`[bench-error]` in the log** - the driver reports which stage failed and
  why; the script prints the last log lines and keeps the raw logs.
* **Frame times much higher than the baseline** - check host load (`uptime`) and
  that no other client/benchmark is running; the numbers are CPU-time based but
  the GPU thread is shared.
* **Missing map load samples** - the map name may not exist in the game data;
  the driver records `loaded:false` and the gate fails.
