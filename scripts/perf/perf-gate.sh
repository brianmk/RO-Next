#!/usr/bin/env bash
# Compare a fresh korangar performance run against the committed baseline.
#
# Optional and non-blocking by default: it prints PASS/WARN lines and only
# returns a non-zero exit status when PERF_GATE_ENFORCE=1. This is the intended
# first stage of the CI integration (see scripts/perf/README.md); once the
# numbers are stable across runners the gate can be made blocking for the
# `perf`-labelled nightly job only.
#
# Usage:
#   PERF_USER=verify1 PERF_PASS=... scripts/perf/perf-gate.sh [options]
#
# Options:
#   --baseline <file>   baseline JSON (default: scripts/perf/baseline.json)
#   --current <file>    compare this JSON instead of running the benchmark
#   --binary <path>     client binary used when running the benchmark
#   --runs <n>          runs for the fresh measurement (default: 2)
#   --update            write the fresh measurement to the baseline file
#                       (same as running perf-baseline.sh --out <baseline>)
#   --enforce           return a non-zero exit status on regressions
#                       (also: PERF_GATE_ENFORCE=1)
#
# Tolerance environment variables (relative, 0.20 = 20%):
#   PERF_GATE_TOLERANCE_FRAME_MEAN   (default 0.15)
#   PERF_GATE_TOLERANCE_FRAME_P95    (default 0.25)
#   PERF_GATE_TOLERANCE_INTERFACE    (default 0.25)
#   PERF_GATE_TOLERANCE_WORLD        (default 0.25)
#   PERF_GATE_TOLERANCE_ALLOC        (default 0.30)
#   PERF_GATE_MIN_FRAMES             (default 60) stages with fewer measured
#                                    frames are reported but not compared,
#                                    because p95 is meaningless on e.g. the
#                                    28-frame map_change stage
#
# The committed baseline is measured on an idle-ish Xvfb lavapipe display with
# the fixed profile in scripts/perf/profile; absolute numbers are only
# meaningful between runs on the same machine class. The gate therefore
# compares relative changes of the same machine and reports the absolute values.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

BASELINE="$SCRIPT_DIR/baseline.json"
CURRENT=""
BINARY=""
RUNS="${PERF_GATE_RUNS:-2}"
UPDATE=0
ENFORCE="${PERF_GATE_ENFORCE:-0}"

TOLERANCE_FRAME_MEAN="${PERF_GATE_TOLERANCE_FRAME_MEAN:-0.15}"
TOLERANCE_FRAME_P95="${PERF_GATE_TOLERANCE_FRAME_P95:-0.25}"
TOLERANCE_INTERFACE="${PERF_GATE_TOLERANCE_INTERFACE:-0.25}"
TOLERANCE_WORLD="${PERF_GATE_TOLERANCE_WORLD:-0.25}"
TOLERANCE_ALLOC="${PERF_GATE_TOLERANCE_ALLOC:-0.30}"
MIN_FRAMES="${PERF_GATE_MIN_FRAMES:-60}"

USER_NAME="${PERF_USER:-${KORANGAR_BENCH_USER:-}}"
PASSWORD="${PERF_PASS:-${KORANGAR_BENCH_PASS:-}}"

while [ $# -gt 0 ]; do
    case "$1" in
        --baseline) BASELINE="$2"; shift 2;;
        --current) CURRENT="$2"; shift 2;;
        --binary) BINARY="$2"; shift 2;;
        --runs) RUNS="$2"; shift 2;;
        --update) UPDATE=1; shift;;
        --enforce) ENFORCE=1; shift;;
        -h|--help) sed -n '2,40p' "${BASH_SOURCE[0]}"; exit 0;;
        *) echo "unknown argument: $1" >&2; exit 2;;
    esac
done

if [ "$UPDATE" = "1" ]; then
    # Re-measure and replace the committed baseline.
    args=(--out "$BASELINE" --runs "$RUNS")
    [ -n "$BINARY" ] && args+=(--binary "$BINARY" --no-build)
    exec "$SCRIPT_DIR/perf-baseline.sh" "${args[@]}"
fi

if [ -z "$CURRENT" ]; then
    if [ -z "$USER_NAME" ] || [ -z "$PASSWORD" ]; then
        echo "error: set PERF_USER and PERF_PASS to run the benchmark for the gate" >&2
        exit 2
    fi

    CURRENT="$(mktemp /tmp/korangar-perf-gate.XXXXXX.json)"
    RUN_ARGS=(--out "$CURRENT" --runs "$RUNS")
    [ -n "$BINARY" ] && RUN_ARGS+=(--binary "$BINARY" --no-build)

    "$SCRIPT_DIR/perf-baseline.sh" "${RUN_ARGS[@]}"
fi

if [ ! -f "$BASELINE" ]; then
    echo "error: baseline not found: $BASELINE" >&2
    echo "       create it with: PERF_USER=... PERF_PASS=... $0 --update" >&2
    exit 2
fi

python3 - "$BASELINE" "$CURRENT" "$TOLERANCE_FRAME_MEAN" "$TOLERANCE_FRAME_P95" "$TOLERANCE_INTERFACE" "$TOLERANCE_WORLD" "$TOLERANCE_ALLOC" "$MIN_FRAMES" <<'PY'
import json
import sys

baseline_path, current_path = sys.argv[1], sys.argv[2]
min_frames = int(sys.argv[8])
tolerances = {
    "frame_ms_mean": float(sys.argv[3]),
    "frame_ms_p95": float(sys.argv[4]),
    "interface_ms": float(sys.argv[5]),
    "world_ms": float(sys.argv[6]),
    "allocs_per_frame": float(sys.argv[7]),
}

baseline = json.load(open(baseline_path))
current = json.load(open(current_path))

regressions = []
missing = []
checked = 0

for stage, expected in baseline.get("stages", {}).items():
    actual = current.get("stages", {}).get(stage)
    if actual is None:
        missing.append(f"stage '{stage}' missing from the current run (benchmark aborted?)")
        continue

    stage_frames = min(expected.get("frames") or 0, actual.get("frames") or 0)
    if stage_frames < min_frames:
        print(f"note: skipping stage '{stage}' ({stage_frames} frames < PERF_GATE_MIN_FRAMES={min_frames}); too few samples for p95")
        continue

    for field, tolerance in tolerances.items():
        expected_value = expected.get(field)
        actual_value = actual.get(field)
        if expected_value in (None, 0) or actual_value is None:
            continue
        checked += 1
        ratio = actual_value / expected_value
        if ratio > 1.0 + tolerance:
            regressions.append(
                f"{stage}.{field}: {actual_value:.3f} > baseline {expected_value:.3f} "
                f"(+{(ratio - 1.0) * 100.0:.1f}%, tolerance {tolerance * 100.0:.0f}%)"
            )

errors = current.get("errors") or []
if errors:
    regressions.append(f"benchmark reported {len(errors)} error(s): {errors[:3]}")

expected_stages = set(baseline.get("stages", {}))
actual_stages = set(current.get("stages", {}))
for stage in sorted(actual_stages - expected_stages):
    print(f"note: new stage '{stage}' in the current run is not gated (add it to the baseline)")

print()
print(f"perf gate: baseline={baseline_path}")
print(f"           current ={current_path}")
print(f"           compared {checked} metric(s) over {len(expected_stages & actual_stages)} stage(s)")
print()

for message in missing:
    print(f"MISSING: {message}")
for message in regressions:
    print(f"REGRESSION: {message}")

if not missing and not regressions:
    print("perf gate: PASS (no metric regressed beyond tolerance)")
    sys.exit(0)

print()
if missing or regressions:
    print("perf gate: FAIL")
sys.exit(1)
PY
status=$?

if [ "$status" -ne 0 ] && [ "$ENFORCE" != "1" ]; then
    echo "perf gate: non-blocking mode (set PERF_GATE_ENFORCE=1 / --enforce to fail the job)"
    exit 0
fi

exit "$status"
