#!/usr/bin/env bash
# Repeatable headless performance baseline for the korangar client.
#
# Runs the client on a muted Xvfb display with a fixed lean profile and a
# scripted scenario (login -> crowded town idle -> walking -> effect/damage
# number burst -> UI + buff bar churn -> map loads) and writes the measured
# per-stage frame CPU time, frame phase breakdown, allocation churn, object
# counts and async loader latencies as JSON.
#
# Usage:
#   PERF_USER=verify1 PERF_PASS=... scripts/perf/perf-baseline.sh [options]
#
# Options:
#   --repo <dir>        client repository root (default: two levels above this script)
#   --binary <path>     client binary (default: <repo>/target/release/korangar)
#   --out <file>        output JSON (default: scripts/perf/baseline.json)
#   --scenario <name>   benchmark scenario (default: full)
#   --runs <n>          number of runs, per-stage median is written (default: 2)
#   --display <:n>      Xvfb display to use (default: :97)
#   --res <WxH>         Xvfb screen and client resolution (default: 1024x600)
#   --timeout <sec>     hard limit per client run (default: 240)
#   --no-build          do not build the client first
#   --keep-logs         keep per-run logs (they are otherwise deleted)
#
# Credentials are taken from PERF_USER/PERF_PASS; KORANGAR_BENCH_USER/PASS are
# used as a fallback. No credentials are stored in this repository.
#
# The scenario stage names are: login, town_idle, town_walk, effects,
# ui_updates, map_change (see korangar/src/benchmark.rs). Map load samples are
# reported separately under "maps".
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

OUT="$SCRIPT_DIR/baseline.json"
BINARY=""
SCENARIO="full"
RUNS=2
DISPLAY_NUM=":97"
RES="1024x600"
TIMEOUT=240
BUILD=1
KEEP_LOGS=0

PERF_USER="${PERF_USER:-${KORANGAR_BENCH_USER:-}}"
PERF_PASS="${PERF_PASS:-${KORANGAR_BENCH_PASS:-}}"

while [ $# -gt 0 ]; do
    case "$1" in
        --repo) REPO_ROOT="$2"; shift 2;;
        --binary) BINARY="$2"; shift 2;;
        --out) OUT="$2"; shift 2;;
        --scenario) SCENARIO="$2"; shift 2;;
        --runs) RUNS="$2"; shift 2;;
        --display) DISPLAY_NUM="$2"; shift 2;;
        --res) RES="$2"; shift 2;;
        --timeout) TIMEOUT="$2"; shift 2;;
        --no-build) BUILD=0; shift;;
        --keep-logs) KEEP_LOGS=1; shift;;
        -h|--help) sed -n '2,40p' "${BASH_SOURCE[0]}"; exit 0;;
        *) echo "unknown argument: $1" >&2; exit 2;;
    esac
done

if [ -z "$PERF_USER" ] || [ -z "$PERF_PASS" ]; then
    echo "error: set PERF_USER and PERF_PASS (test account credentials) in the environment" >&2
    echo "       example: PERF_USER=verify1 PERF_PASS=... $0 --runs 2" >&2
    exit 2
fi

if [ -z "$BINARY" ]; then
    BINARY="$REPO_ROOT/target/release/korangar"
fi

if [ "$BUILD" = "1" ]; then
    echo "perf: building release client (incremental, a few minutes on a cold tree)"
    ( cd "$REPO_ROOT" && PATH="/home/phantom/Games/ragnarok/CondaRO-linux/tools/bin:$PATH" cargo build --release --quiet )
fi

if [ ! -x "$BINARY" ]; then
    echo "error: client binary not found: $BINARY (build it or pass --binary)" >&2
    exit 2
fi

RUNTIME_DIR="$REPO_ROOT/korangar"
if [ ! -d "$RUNTIME_DIR" ]; then
    echo "error: expected the client crate at $RUNTIME_DIR (run from the korangar repository)" >&2
    exit 2
fi

CLIENT_DIR="$RUNTIME_DIR/client"
if [ ! -d "$CLIENT_DIR" ]; then
    echo "error: runtime client directory not found: $CLIENT_DIR" >&2
    echo "       create it by running the client once (it writes its settings there)" >&2
    exit 2
fi

# --- fixed profile -----------------------------------------------------------
# The scenario must run on the same settings on every machine/run: fixed lean
# graphics (no AA/shadows/MSAA), 30 fps cap, muted audio, 0.75 UI scale and the
# default interface themes. The originals are backed up once.
for file in graphics_settings.ron interface_settings.ron audio_settings.ron; do
    if [ -f "$CLIENT_DIR/$file" ] && [ ! -f "$CLIENT_DIR/$file.perf-backup" ]; then
        cp "$CLIENT_DIR/$file" "$CLIENT_DIR/$file.perf-backup"
    fi
    cp "$SCRIPT_DIR/profile/$file" "$CLIENT_DIR/$file"
done

# --- Xvfb + muted ALSA -------------------------------------------------------
STARTED_XVFB=0
XVFB_PID=""
if ! xdpyinfo -display "$DISPLAY_NUM" >/dev/null 2>&1; then
    echo "perf: starting Xvfb $DISPLAY_NUM at ${RES}x24"
    Xvfb "$DISPLAY_NUM" -screen 0 "${RES}x24" -nolisten tcp >/tmp/korangar-perf-xvfb${DISPLAY_NUM#:}.log 2>&1 &
    XVFB_PID=$!
    STARTED_XVFB=1
    for _ in $(seq 1 50); do
        if xdpyinfo -display "$DISPLAY_NUM" >/dev/null 2>&1; then break; fi
        sleep 0.1
    done
fi

cleanup() {
    if [ "$STARTED_XVFB" = "1" ] && [ -n "$XVFB_PID" ]; then
        kill "$XVFB_PID" >/dev/null 2>&1 || true
    fi
}
trap cleanup EXIT

cat > /tmp/korangar-perf-asound.conf <<'EOF'
pcm.!default { type null }
ctl.!default { type null }
EOF

mkdir -p /tmp/korangar-perf
WORK_DIR="$(mktemp -d /tmp/korangar-perf/run.XXXXXX)"

RES_WIDTH="${RES%x*}"
RES_HEIGHT="${RES#*x}"

# --- runs --------------------------------------------------------------------
run_index=0
while [ "$run_index" -lt "$RUNS" ]; do
    run_index=$((run_index + 1))
    LOG="$WORK_DIR/run-$run_index.log"
    RAW="$WORK_DIR/run-$run_index.bench"
    rm -f "$RAW"

    echo "perf: run $run_index/$RUNS (scenario=$SCENARIO, display=$DISPLAY_NUM, res=$RES)"

    set +e
    (
        cd "$RUNTIME_DIR"
        env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
            DISPLAY="$DISPLAY_NUM" \
            WINIT_UNIX_BACKEND=x11 \
            KORANGAR_X11=1 \
            ALSA_CONFIG_PATH=/tmp/korangar-perf-asound.conf \
            KORANGAR_MUTE_AUDIO=1 \
            KORANGAR_RESOLUTION="$RES" \
            KORANGAR_UI_SCALE=0.75 \
            KORANGAR_MAX_FPS=30 \
            KORANGAR_BACKGROUND_MAX_FPS=30 \
            KORANGAR_FORCE_UNFOCUSED=1 \
            KORANGAR_PERF_LOG=1 \
            KORANGAR_BENCH="$SCENARIO" \
            KORANGAR_BENCH_USER="$PERF_USER" \
            KORANGAR_BENCH_PASS="$PERF_PASS" \
            KORANGAR_BENCH_OUT="$RAW" \
            timeout --signal=TERM --kill-after=15 "$TIMEOUT" "$BINARY" >"$LOG" 2>&1
        status=$?
        # A clean benchmark run exits through the client's shutdown path with
        # status 0; `timeout` returns 124 when the scenario hung.
        exit "$status"
    )
    status=$?
    set -e

    if [ "$status" -ne 0 ] && [ "$status" -ne 124 ]; then
        echo "perf: warning: run $run_index exited with status $status" >&2
    fi

    if ! grep -q '^\[bench-end\]' "$LOG"; then
        echo "perf: error: run $run_index did not finish the benchmark scenario" >&2
        echo "perf:        last log lines:" >&2
        tail -n 25 "$LOG" >&2 || true
        echo "perf:        full log: $LOG" >&2
        KEEP_LOGS=1
        if [ "$KEEP_LOGS" = "0" ]; then rm -rf "$WORK_DIR"; fi
        exit 1
    fi
done

# --- parse + aggregate -------------------------------------------------------
python3 - "$WORK_DIR" "$OUT" "$SCENARIO" "$RUNS" "$RES" "$REPO_ROOT" "$BINARY" <<'PY'
import glob
import hashlib
import json
import os
import statistics
import sys
from datetime import datetime, timezone

work_dir, out_path, scenario, runs, resolution, repo_root, binary = sys.argv[1:8]
runs = int(runs)

NUMERIC_FIELDS = [
    "frames", "duration_s",
    "frame_ms_mean", "frame_ms_p50", "frame_ms_p95", "frame_ms_max",
    "wait_ms", "sim_ms", "interface_ms", "world_ms", "gpu_ms",
    "allocs_per_frame", "main_allocs_per_frame", "pre_allocs_per_frame",
    "deallocs_per_frame",
    "sim_allocs_per_frame", "interface_allocs_per_frame", "world_allocs_per_frame",
    "gpu_allocs_per_frame",
    "alloc_kib_per_frame",
    "effects_mean", "particles_mean", "entities_mean", "windows_mean",
    "effects_rendered_mean", "effects_culled_mean",
    "particles_rendered_mean", "particles_culled_mean",
    "effects_max", "particles_max", "entities_max", "windows_max",
]

def parse_lines(log_path):
    stages = {}
    maps = {}
    errors = []
    ended = False
    with open(log_path, "r", errors="replace") as handle:
        for line in handle:
            line = line.rstrip("\n")
            if line.startswith("[bench-stage] "):
                data = json.loads(line[len("[bench-stage] "):])
                stages[data["stage"]] = data
            elif line.startswith("[bench-map] "):
                data = json.loads(line[len("[bench-map] "):])
                maps.setdefault(data["map"], []).append(data)
            elif line.startswith("[bench-error] "):
                errors.append(json.loads(line[len("[bench-error] "):]))
            elif line.startswith("[bench-end] "):
                ended = True
    return stages, maps, errors, ended

run_stages = []
all_maps = {}
errors = []
for path in sorted(glob.glob(os.path.join(work_dir, "run-*.log"))):
    stages, maps, run_errors, ended = parse_lines(path)
    errors.extend(run_errors)
    if not ended:
        errors.append({"reason": f"run did not finish: {path}"})
    run_stages.append(stages)
    for name, samples in maps.items():
        all_maps.setdefault(name, []).extend(samples)

stage_names = []
for stages in run_stages:
    for name in stages:
        if name not in stage_names:
            stage_names.append(name)

def median(values):
    values = [value for value in values if value is not None]
    return statistics.median(values) if values else None

stages_out = {}
for name in stage_names:
    entries = [stages[name] for stages in run_stages if name in stages]
    if not entries:
        continue
    aggregated = {}
    for field in NUMERIC_FIELDS:
        values = [entry.get(field) for entry in entries]
        aggregated[field] = round(median(values), 4) if median(values) is not None else None
    aggregated["runs"] = len(entries)
    stages_out[name] = aggregated

maps_out = {}
for name, samples in all_maps.items():
    loaded = [sample for sample in samples if sample.get("loaded")]
    maps_out[name] = {
        "loads": len(samples),
        "loaded": len(loaded),
        "ms_median": round(median([sample["ms"] for sample in loaded]), 2) if loaded else None,
        "ms_min": round(min(sample["ms"] for sample in loaded), 2) if loaded else None,
        "ms_max": round(max(sample["ms"] for sample in loaded), 2) if loaded else None,
    }

def binary_sha256(path):
    try:
        digest = hashlib.sha256()
        with open(path, "rb") as handle:
            for chunk in iter(lambda: handle.read(1 << 20), b""):
                digest.update(chunk)
        return digest.hexdigest()
    except OSError:
        return None

def git_sha():
    # Worktrees store the branch ref in the common git dir, so use git itself
    # and only fall back to parsing the worktree `.git` file if git is missing.
    try:
        import subprocess

        output = subprocess.check_output(["git", "-C", repo_root, "rev-parse", "HEAD"], text=True, stderr=subprocess.DEVNULL)
        return output.strip()
    except Exception:
        return None

def host_load():
    try:
        return round(os.getloadavg()[0], 2)
    except OSError:
        return None

baseline = {
    "meta": {
        "generated_utc": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "scenario": scenario,
        "runs": runs,
        "resolution": resolution,
        "ui_scale": 0.75,
        "fps_cap": 30,
        "git_sha": git_sha(),
        "binary_sha256": binary_sha256(binary),
        "host_load_1min": host_load(),
        "notes": "Per-stage medians over the runs. Frame times are client CPU time in the render loop, not wall FPS.",
    },
    "stages": stages_out,
    "maps": maps_out,
    "errors": errors,
}

with open(out_path, "w") as handle:
    json.dump(baseline, handle, indent=2, sort_keys=True)
    handle.write("\n")

print()
header = f"{'stage':<12}{'frames':>8}{'p50':>9}{'p95':>9}{'max':>9}{'sim':>9}{'iface':>9}{'world':>9}{'gpu':>9}{'allocs/f':>10}{'KiB/f':>9}{'effects':>9}{'culled':>8}{'particles':>10}{'entities':>10}"
print(header)
print("-" * len(header))
for name in stage_names:
    stage = stages_out.get(name)
    if not stage:
        continue
    print(
        f"{name:<12}{stage['frames'] or 0:>8.0f}"
        f"{stage['frame_ms_p50'] or 0:>9.2f}{stage['frame_ms_p95'] or 0:>9.2f}{stage['frame_ms_max'] or 0:>9.2f}"
        f"{stage['sim_ms'] or 0:>9.2f}{stage['interface_ms'] or 0:>9.2f}{stage['world_ms'] or 0:>9.2f}{stage['gpu_ms'] or 0:>9.2f}"
        f"{stage['allocs_per_frame'] or 0:>10.1f}{stage['alloc_kib_per_frame'] or 0:>9.1f}"
        f"{stage['effects_mean'] or 0:>9.1f}{stage['effects_culled_mean'] or 0:>8.1f}"
        f"{stage['particles_mean'] or 0:>10.1f}{stage['entities_mean'] or 0:>10.1f}"
    )
print()
if maps_out:
    print("map load latency (ms):")
    for name, data in maps_out.items():
        print(f"  {name:<12} median={data['ms_median']} min={data['ms_min']} max={data['ms_max']} ({data['loaded']}/{data['loads']} loaded)")
    print()
print(f"perf: baseline written to {out_path}")
if errors:
    print(f"perf: WARNING: {len(errors)} benchmark errors recorded: {errors[:3]}")

PY

if [ "$KEEP_LOGS" = "1" ]; then
    echo "perf: raw logs kept in $WORK_DIR"
else
    rm -rf "$WORK_DIR"
fi
