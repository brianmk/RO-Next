#!/bin/bash
# Headless golden-screenshot matrix for the Korangar interface.
#
# Captures a fixed set of windows (login, server selection, character
# selection, inventory, equipment, character overview) for every combination of
# theme and interface scaling, so that theme/token/localization changes can be
# reviewed and regression-tested without a human taking screenshots.
#
# How it works:
#   * starts a private Xvfb display (no window manager needed),
#   * runs a lean, muted client (no shadows, no AA, 30 FPS) against the live
#     local server,
#   * drives login/character selection/window opening through XTEST input
#     (scripts/ui_golden_driver.py) - no server-side test hooks are used,
#   * captures only the window rectangles that the client dumps through
#     KORANGAR_UI_RECTS_DUMP, so the animated 3D world is never compared,
#   * compares the crops against the committed baseline matrix with a tolerant
#     metric and animation masks (scripts/ui_golden_compare.py).
#
# Credentials are never stored in the repository. Provide a test account:
#
#     export UI_GOLDEN_ACCOUNT=verify5
#     export UI_GOLDEN_PASSWORD='...'
#
# or point UI_GOLDEN_CREDENTIALS_FILE at a git-ignored file with the account on
# the first line and the password on the second line.
#
# Usage:
#     scripts/ui-golden.sh                        # capture + compare
#     scripts/ui-golden.sh --update-baseline      # capture + write new baseline
#     scripts/ui-golden.sh --themes classic --scales 1.0
#     scripts/ui-golden.sh --capture-only         # just capture, do not compare
#
# Environment:
#     UI_GOLDEN_ACCOUNT / UI_GOLDEN_PASSWORD / UI_GOLDEN_CREDENTIALS_FILE
#     UI_GOLDEN_BIN           client binary (default: target/release/korangar)
#     UI_GOLDEN_DISPLAY       X11 display (default: :121)
#     UI_GOLDEN_RES           Xvfb resolution (default: 1280x720)
#     UI_GOLDEN_WORK          output directory (default: target/ui-golden)
#     UI_GOLDEN_VK_ICD        Vulkan ICD json for software rendering
#     UI_GOLDEN_THEMES        default matrix themes (default: default,classic)
#     UI_GOLDEN_SCALES        default matrix scalings (default: 0.75,1.0)
#     UI_GOLDEN_KEEP_CLIENT   keep the client logs, do not delete the run dirs
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_DIR="$REPO_ROOT/korangar"

BIN="${UI_GOLDEN_BIN:-$REPO_ROOT/target/release/korangar}"
DISPLAY_NUM="${UI_GOLDEN_DISPLAY:-:121}"
RES="${UI_GOLDEN_RES:-1280x720}"
WORK="${UI_GOLDEN_WORK:-$REPO_ROOT/target/ui-golden}"
BASELINE="$REPO_ROOT/korangar/test-data/golden"
THEMES="${UI_GOLDEN_THEMES:-default,classic}"
SCALES="${UI_GOLDEN_SCALES:-0.75,1.0}"

UPDATE_BASELINE=0
CAPTURE_ONLY=0
SCREENS_FILTER=""
while [ $# -gt 0 ]; do
  case "$1" in
    --update-baseline) UPDATE_BASELINE=1; shift;;
    --capture-only) CAPTURE_ONLY=1; shift;;
    --themes) THEMES="$2"; shift 2;;
    --scales) SCALES="$2"; shift 2;;
    --binary) BIN="$2"; shift 2;;
    --display) DISPLAY_NUM="$2"; shift 2;;
    --work) WORK="$2"; shift 2;;
    --baseline) BASELINE="$2"; shift 2;;
    -h|--help) sed -n '2,50p' "${BASH_SOURCE[0]}"; exit 0;;
    *) echo "unknown argument: $1" >&2; exit 2;;
  esac
done

# --- credentials -----------------------------------------------------------
if [ -n "${UI_GOLDEN_CREDENTIALS_FILE:-}" ]; then
  UI_GOLDEN_ACCOUNT="$(sed -n '1p' "$UI_GOLDEN_CREDENTIALS_FILE")"
  UI_GOLDEN_PASSWORD="$(sed -n '2p' "$UI_GOLDEN_CREDENTIALS_FILE")"
fi
if [ -z "${UI_GOLDEN_ACCOUNT:-}" ] || [ -z "${UI_GOLDEN_PASSWORD:-}" ]; then
  echo "error: set UI_GOLDEN_ACCOUNT and UI_GOLDEN_PASSWORD (or UI_GOLDEN_CREDENTIALS_FILE)." >&2
  echo "       Use a dedicated verification account; credentials must not be committed." >&2
  exit 2
fi

# ServiceId of the CondaRO connection in archive/data/sclientinfo.xml. The
# remembered credentials are keyed by it in client/login_settings.ron. It is a
# Rust DefaultHasher over (display name, address, port), so it changes when the
# server address changes; override it with UI_GOLDEN_SERVICE_ID then.
SERVICE_ID="${UI_GOLDEN_SERVICE_ID:-3200853616523840849}"

[ -x "$BIN" ] || { echo "error: client binary not found: $BIN (build with: cargo build --release)" >&2; exit 2; }
[ -d "$APP_DIR" ] || { echo "error: app directory not found: $APP_DIR" >&2; exit 2; }
mkdir -p "$WORK"

# --- software Vulkan (lavapipe) -------------------------------------------
VULKAN_ENV=()
if [ -n "${UI_GOLDEN_VK_ICD:-}" ]; then
  VULKAN_ENV+=("VK_ICD_FILENAMES=$UI_GOLDEN_VK_ICD")
  LVP_LIB_PATH="$(dirname "$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["ICD"]["library_path"])' "$UI_GOLDEN_VK_ICD" 2>/dev/null || echo "")")"
  if [ -n "$LVP_LIB_PATH" ] && [ -d "$LVP_LIB_PATH" ]; then
    VULKAN_ENV+=("LD_LIBRARY_PATH=$LVP_LIB_PATH:${LVP_LIB_PATH}/x86_64-linux-gnu")
  fi
else
  LVP_DIR="/home/phantom/.local/share/flatpak/runtime/org.freedesktop.Platform.GL.default/x86_64/26.08/active/files"
  if [ -f "$LVP_DIR/lib/libvulkan_lvp.so" ]; then
    ICD_FILE="$WORK/lavapipe_icd.json"
    printf '{"ICD":{"api_version":"1.4.354","library_arch":"64","library_path":"%s/lib/libvulkan_lvp.so"},"file_format_version":"1.0.1"}\n' "$LVP_DIR" > "$ICD_FILE"
    VULKAN_ENV+=("VK_ICD_FILENAMES=$ICD_FILE" "LD_LIBRARY_PATH=$LVP_DIR/lib:$LVP_DIR/lib/x86_64-linux-gnu")
  fi
fi

# --- Xvfb ------------------------------------------------------------------
DISPLAY_NUM="${DISPLAY_NUM#:}"
if ! [ -e "/tmp/.X${DISPLAY_NUM}-lock" ]; then
  Xvfb ":$DISPLAY_NUM" -screen 0 "${RES}x24" -nolisten tcp >"$WORK/xvfb.log" 2>&1 &
  XVFB_PID=$!
  trap '[ -n "${XVFB_PID:-}" ] && kill "$XVFB_PID" 2>/dev/null || true' EXIT
  # Xvfb can take a few seconds on the first start (font cache); wait until it
  # actually accepts connections before launching the client.
  for _ in $(seq 1 40); do
    if DISPLAY=":$DISPLAY_NUM" xdpyinfo >/dev/null 2>&1; then
      break
    fi
    sleep 0.25
  done
  if ! DISPLAY=":$DISPLAY_NUM" xdpyinfo >/dev/null 2>&1; then
    echo "error: Xvfb did not come up on :$DISPLAY_NUM (see $WORK/xvfb.log)" >&2
    exit 2
  fi
fi

# Muted audio: a null ALSA device, so the client never blocks on sound.
cat > "$WORK/asound-null.conf" <<'EOF'
pcm.!default { type null }
ctl.!default { type null }
EOF

# --- helpers ---------------------------------------------------------------
THEME_VALUE() {
  case "$1" in
    default) printf '^000001default^000000\\0';;
    *) printf '%s' "$1";;
  esac
}

write_settings() {
  local theme="$1" scale="$2" theme_value
  theme_value="$(THEME_VALUE "$theme")"
  mkdir -p "$APP_DIR/client"
  # Lean, deterministic graphics profile: no shadows, no AA, 30 FPS cap.
  cat > "$APP_DIR/client/graphics_settings.ron" <<EOF
(
    lighting_mode: Classic,
    vsync: false,
    max_fps: 30,
    background_max_fps: 30,
    triple_buffering: false,
    texture_filtering: Linear,
    msaa: Off,
    ssaa: Off,
    screen_space_anti_aliasing: Off,
    shadow_method: Hard,
    shadow_resolution: Normal,
    shadow_detail: Low,
    sdsm: false,
    high_quality_interface: false,
    window_resolution: R1280x720,
    fullscreen: false,
)
EOF
  cat > "$APP_DIR/client/interface_settings.ron" <<EOF
(
    language: English,
    scaling: $scale,
    menu_theme: "$theme_value",
    in_game_theme: "$theme_value",
    world_theme: "$theme_value",
    show_character_names: false,
    show_chat_bubbles: true,
    chat_bubble_background: true,
    show_skill_names_above_characters: true,
    transparent_chat: false,
    minimap_transparent_frame: false,
    character_overview_compact: false,
)
EOF
  # Remember the test credentials so the driver can log in with Enter (the
  # client supports submitting the login form from the keyboard). The file is
  # written at capture time and never committed - client/ is git-ignored.
  cat > "$APP_DIR/client/login_settings.ron" <<EOF
(
    service_settings: {
        ($SERVICE_ID): (
            username: "$UI_GOLDEN_ACCOUNT",
            password: "$UI_GOLDEN_PASSWORD",
            remember_username: true,
            remember_password: true,
        ),
    },
    recent_service_id: Some(($SERVICE_ID)),
)
EOF
  rm -f "$APP_DIR/client/window_cache.ron"
}

start_client() {
  local output="$1" rects="$2"
  local log="$output/client.log"
  (
    cd "$APP_DIR"
    exec env -u WAYLAND_DISPLAY -u WAYLAND_SOCKET \
      DISPLAY=":$DISPLAY_NUM" WINIT_UNIX_BACKEND=x11 KORANGAR_X11=1 \
      ALSA_CONFIG_PATH="$WORK/asound-null.conf" \
      KORANGAR_MUTE_AUDIO=1 KORANGAR_MAX_FPS=30 KORANGAR_BACKGROUND_MAX_FPS=30 \
      KORANGAR_FORCE_UNFOCUSED=1 KORANGAR_LOG_LEVEL=3 \
      KORANGAR_UI_RECTS_DUMP="$rects" \
      "${VULKAN_ENV[@]}" \
      nice -n 19 "$BIN"
  ) >"$log" 2>&1 &
  CLIENT_PID=$!
  echo $CLIENT_PID > "$output/client.pid"
}

stop_client() {
  # Only ever kill the exact client process this script started - a `pkill -f`
  # by binary name would terminate other agents' clients on the shared box.
  if [ -n "${CLIENT_PID:-}" ]; then
    kill "$CLIENT_PID" 2>/dev/null || true
    for _ in $(seq 1 20); do
      kill -0 "$CLIENT_PID" 2>/dev/null || break
      sleep 0.1
    done
    kill -9 "$CLIENT_PID" 2>/dev/null || true
    wait "$CLIENT_PID" 2>/dev/null || true
  fi
  CLIENT_PID=""
}

run_variant() {
  local theme="$1" scale="$2"
  local variant="$theme-$scale"
  local output="$WORK/$variant"
  rm -rf "$output" && mkdir -p "$output"
  local rects="$output/rects.txt"

  echo "== golden UI: theme=$theme scale=$scale"
  write_settings "$theme" "$scale"
  start_client "$output" "$rects"
  sleep 3

  # The live test server and the shared box can be noisy (other agents may
  # restart clients), so retry a failed capture once before giving up.
  local attempt driver_status=1
  for attempt in 1 2 3; do
    if python3 "$SCRIPT_DIR/ui_golden_driver.py" \
        --display ":$DISPLAY_NUM" \
        --rects-file "$rects" \
        --output "$output" \
        --theme "$theme" \
        --scale "$scale" \
        --account "$UI_GOLDEN_ACCOUNT" \
        --password "$UI_GOLDEN_PASSWORD"; then
      driver_status=0
      break
    fi
    echo "   capture attempt $attempt failed, retrying"
    stop_client
    sleep 2
    rm -rf "$output"/*.png "$output/capture.json"
    start_client "$output" "$rects"
    sleep 3
  done
  if [ "$driver_status" -ne 0 ]; then
    echo "error: capture failed for variant $variant (see $output/client.log)" >&2
    stop_client
    return 1
  fi

  local metadata="$output/capture.json"
  local commit
  commit="$(git -C "$REPO_ROOT" log -1 --format=%H 2>/dev/null || echo unknown)"
  python3 - "$metadata" "$theme" "$scale" "$BIN" "$commit" <<'PY'
import json, sys
path, theme, scale, binary, commit = sys.argv[1:]
payload = json.load(open(path))
payload.setdefault("metadata", {})
payload["metadata"].update({"theme": theme, "scale": scale, "binary": binary, "commit": commit})
json.dump(payload, open(path, "w"), indent=2)
PY

  stop_client
  echo "   captures in $output"
}

cleanup() {
  stop_client
  [ -n "${XVFB_PID:-}" ] && kill "$XVFB_PID" 2>/dev/null || true
}
trap cleanup EXIT

# --- matrix ----------------------------------------------------------------
IFS=',' read -ra THEME_LIST <<< "$THEMES"
IFS=',' read -ra SCALE_LIST <<< "$SCALES"
VARIANT_DIRS=()
STATUS=0
for theme in "${THEME_LIST[@]}"; do
  for scale in "${SCALE_LIST[@]}"; do
    if run_variant "$theme" "$scale"; then
      VARIANT_DIRS+=("$WORK/$theme-$scale")
    else
      STATUS=1
    fi
  done
done

if [ "$CAPTURE_ONLY" -eq 1 ]; then
  echo "captured ${#VARIANT_DIRS[@]} variant(s) in $WORK"
  exit "$STATUS"
fi

# --- compare (or update) the whole matrix ---------------------------------
# Merge the per-variant captures into one flat directory with
# "<theme>-<scale>-<screen>.png" names, so the baseline layout is easy to
# review and the comparison step needs no per-variant logic.
MERGED="$WORK/current"
rm -rf "$MERGED" && mkdir -p "$MERGED"
python3 - "$WORK" "$MERGED" <<'PY'
import json, shutil, sys
from pathlib import Path
work, merged = Path(sys.argv[1]), Path(sys.argv[2])
captures = []
metadata = {}
for variant_directory in sorted(work.iterdir()):
    capture_file = variant_directory / "capture.json"
    if not variant_directory.is_dir() or not capture_file.exists():
        continue
    payload = json.loads(capture_file.read_text())
    variant = variant_directory.name
    metadata[variant] = payload.get("metadata", {})
    for screen in payload.get("captures", []):
        name = f"{variant}-{screen['name']}"
        for suffix in (".png", ".second.png", ".full.png"):
            source = variant_directory / f"{screen['name']}{suffix}"
            if source.exists():
                shutil.copyfile(source, merged / f"{name}{suffix}")
        screen = dict(screen)
        screen["name"] = name
        captures.append(screen)
json.dump({"metadata": metadata, "captures": captures}, open(merged / "capture.json", "w"), indent=2)
print(f"merged {len(captures)} captures from {len(metadata)} variant(s)")
PY

if [ "$UPDATE_BASELINE" -eq 1 ]; then
  python3 "$SCRIPT_DIR/ui_golden_compare.py" --update --baseline "$BASELINE" --current "$MERGED"
  echo "baseline matrix updated in $BASELINE"
else
  if [ ! -d "$BASELINE" ]; then
    echo "error: no baseline directory at $BASELINE; run with --update-baseline first" >&2
    exit 2
  fi
  python3 "$SCRIPT_DIR/ui_golden_compare.py" --baseline "$BASELINE" --current "$MERGED" || STATUS=1
fi

if [ "${UI_GOLDEN_KEEP_CLIENT:-0}" != "1" ]; then
  rm -rf "${VARIANT_DIRS[@]}" 2>/dev/null || true
fi
exit "$STATUS"
