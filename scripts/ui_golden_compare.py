#!/usr/bin/env python3
"""Tolerant, window-cropped comparison for the golden UI screenshots.

The harness captures every screen as a crop of the window that the screen is
about, so the animated 3D world behind the windows never has to be compared.
Inside the window there can still be animation (the paper doll on the
equipment window, buff timers, ...). The baseline step therefore takes two
captures a moment apart and stores the pixels that changed as a mask; those
pixels are ignored by the comparison later.

Usage:

    # create/update the baseline matrix from a capture run
    scripts/ui_golden_compare.py --update --baseline korangar/test-data/golden --current target/ui-golden/current

    # compare a new capture run against the baseline matrix
    scripts/ui_golden_compare.py --baseline korangar/test-data/golden --current target/ui-golden/current
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

try:
    import numpy as np
    from PIL import Image, ImageChops
except ImportError:  # pragma: no cover - clear error instead of a traceback
    sys.exit("Pillow and numpy are required for the golden UI comparison (pip install pillow numpy)")


def load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.int16)


def save_image(array: np.ndarray, path: Path) -> None:
    Image.fromarray(array.astype(np.uint8), mode="RGB").save(path)


def dilate(mask: np.ndarray, iterations: int = 2) -> np.ndarray:
    """Grow a boolean mask by one pixel per iteration (its 3x3 neighbourhood)."""
    result = mask.copy()
    for _ in range(iterations):
        grown = result.copy()
        grown[1:, :] |= result[:-1, :]
        grown[:-1, :] |= result[1:, :]
        grown[:, 1:] |= result[:, :-1]
        grown[:, :-1] |= result[:, 1:]
        grown[1:, 1:] |= result[:-1, :-1]
        grown[:-1, :-1] |= result[1:, 1:]
        grown[1:, :-1] |= result[:-1, 1:]
        grown[:-1, 1:] |= result[1:, :-1]
        result = grown
    return result


def animation_mask(first: np.ndarray, second: np.ndarray, threshold: int = 6) -> np.ndarray:
    """Return the mask of pixels that changed between two consecutive frames."""
    difference = np.abs(first - second).max(axis=2)
    return dilate(difference > threshold, iterations=2)


def relative_masks_to_pixels(relative_masks, width: int, height: int) -> np.ndarray:
    mask = np.zeros((height, width), dtype=bool)
    for relative_left, relative_top, relative_width, relative_height in relative_masks:
        left = max(0, int(relative_left * width))
        top = max(0, int(relative_top * height))
        right = min(width, int((relative_left + relative_width) * width))
        bottom = min(height, int((relative_top + relative_height) * height))
        mask[top:bottom, left:right] = True
    return mask


def compare_one(
    name: str,
    baseline_directory: Path,
    current_directory: Path,
    capture: dict,
    thresholds: argparse.Namespace,
    verbose: bool,
) -> dict:
    baseline_image_path = baseline_directory / f"{name}.png"
    baseline_mask_path = baseline_directory / f"{name}.mask.png"
    current_image_path = current_directory / f"{name}.png"

    if not baseline_image_path.exists():
        return {"name": name, "status": "missing-baseline", "message": f"{baseline_image_path} does not exist"}
    if not current_image_path.exists():
        return {"name": name, "status": "missing-capture", "message": f"{current_image_path} does not exist"}

    baseline = load_rgb(baseline_image_path)
    current = load_rgb(current_image_path)

    if baseline.shape != current.shape:
        # The window rectangle is derived from the client layout, which can
        # differ by a few pixels between runs (font metrics / auto-sized
        # windows). Small size differences are compared on the common area;
        # anything bigger is a real layout regression.
        height_drift = abs(baseline.shape[0] - current.shape[0])
        width_drift = abs(baseline.shape[1] - current.shape[1])
        if height_drift > 8 or width_drift > 8:
            return {
                "name": name,
                "status": "size-mismatch",
                "message": f"baseline is {baseline.shape[1]}x{baseline.shape[0]}, current is {current.shape[1]}x{current.shape[0]}",
            }
        common_height = min(baseline.shape[0], current.shape[0])
        common_width = min(baseline.shape[1], current.shape[1])
        baseline = baseline[:common_height, :common_width]
        current = current[:common_height, :common_width]

    height, width = baseline.shape[:2]
    ignored = np.zeros((height, width), dtype=bool)
    if baseline_mask_path.exists():
        stored_mask = np.asarray(Image.open(baseline_mask_path).convert("L"), dtype=np.uint8) > 127
        if stored_mask.shape == ignored.shape:
            ignored |= stored_mask
    ignored |= relative_masks_to_pixels(capture.get("masks", []), width, height)

    difference = np.abs(baseline - current).max(axis=2)
    changed = difference > thresholds.pixel_threshold
    compared = int((~ignored).sum())
    if compared == 0:
        return {"name": name, "status": "pass", "message": "every pixel is masked", "changed_ratio": 0.0, "rmse": 0.0}
    changed_count = int((changed & ~ignored).sum())
    changed_ratio = changed_count / compared

    compared_difference = np.abs(baseline - current).astype(np.float32)
    if compared:
        rmse = float(np.sqrt(np.mean((compared_difference**2)[~ignored])))
    else:  # pragma: no cover - guarded above
        rmse = 0.0

    status = "pass"
    if changed_ratio > thresholds.max_changed_ratio:
        status = "fail"
    if rmse > thresholds.max_rmse:
        status = "fail"

    if verbose or status != "pass":
        # Visualise: baseline / current / amplified diff, side by side.
        diff_visual = np.zeros_like(current)
        amplified = np.clip(difference * 4, 0, 255).astype(np.uint8)
        diff_visual[..., 0] = amplified
        diff_visual[..., 1] = (amplified * 0.4).astype(np.uint8)
        combined = np.concatenate([baseline.astype(np.uint8), current.astype(np.uint8), diff_visual.astype(np.uint8)], axis=1)
        save_image(combined, current_directory / f"diff-{name}.png")

    message = f"changed={changed_ratio * 100:.3f}% rmse={rmse:.2f}"
    if status != "pass":
        message += f" (limits: {thresholds.max_changed_ratio * 100:.2f}% / {thresholds.max_rmse:.1f})"
    return {
        "name": name,
        "status": status,
        "message": message,
        "changed_ratio": changed_ratio,
        "rmse": rmse,
        "rect": capture.get("rect"),
    }


def read_capture(capture_directory: Path) -> dict:
    capture_file = capture_directory / "capture.json"
    if not capture_file.exists():
        sys.exit(f"capture.json not found in {capture_directory}; run the capture step first")
    return json.loads(capture_file.read_text())


def update_baseline(baseline_directory: Path, current_directory: Path, capture: dict) -> int:
    manifest_screens = {}
    for screen in capture["captures"]:
        name = screen["name"]
        first_path = current_directory / f"{name}.png"
        second_path = current_directory / f"{name}.second.png"
        if not first_path.exists():
            print(f"warning: {first_path} is missing, skipping", file=sys.stderr)
            continue
        baseline_directory.mkdir(parents=True, exist_ok=True)
        (baseline_directory / f"{name}.png").write_bytes(first_path.read_bytes())

        if second_path.exists():
            first = load_rgb(first_path)
            second = load_rgb(second_path)
            if first.shape == second.shape:
                mask = animation_mask(first, second)
                Image.fromarray((mask * 255).astype(np.uint8), mode="L").save(baseline_directory / f"{name}.mask.png")
        manifest_screens[name] = {
            "capture": screen,
            "baseline_rect": screen["rect"],
        }
        print(f"  updated baseline {name}")

    manifest = {
        "metadata": capture.get("metadata", {}),
        "screens": manifest_screens,
    }
    (baseline_directory / "manifest.json").write_text(json.dumps(manifest, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--baseline", required=True, type=Path, help="baseline directory (korangar/test-data/golden)")
    parser.add_argument("--current", required=True, type=Path, help="directory of the capture run")
    parser.add_argument("--update", action="store_true", help="write the capture run as the new baseline")
    parser.add_argument("--pixel-threshold", type=int, default=24, help="per-channel difference that counts as changed (0-255)")
    parser.add_argument("--max-changed-ratio", type=float, default=0.004, help="maximum fraction of changed pixels")
    parser.add_argument("--max-rmse", type=float, default=4.0, help="maximum root mean square error")
    parser.add_argument("--verbose", action="store_true", help="always write diff images")
    args = parser.parse_args()

    capture = read_capture(args.current)

    if args.update:
        print(f"updating baseline matrix in {args.baseline}")
        return update_baseline(args.baseline, args.current, capture)

    print(f"comparing {args.current} against baseline {args.baseline}")
    results = []
    for screen in capture["captures"]:
        result = compare_one(screen["name"], args.baseline, args.current, screen, args, args.verbose)
        results.append(result)
        symbol = {"pass": "ok", "fail": "FAIL"}.get(result["status"], "ERROR")
        print(f"  [{symbol:5}] {screen['name']:20} {result['message']}")
        baseline_meta = args.baseline / "manifest.json"
        if baseline_meta.exists():
            manifest = json.loads(baseline_meta.read_text())
            stored = manifest.get("screens", {}).get(screen["name"], {}).get("baseline_rect")
            if stored and screen.get("rect"):
                drift = max(
                    abs(stored.get("x", 0) - screen["rect"].get("x", 0)),
                    abs(stored.get("y", 0) - screen["rect"].get("y", 0)),
                    abs(stored.get("width", 0) - screen["rect"].get("width", 0)),
                    abs(stored.get("height", 0) - screen["rect"].get("height", 0)),
                )
                if drift > 2:
                    result["status"] = "fail"
                    result["message"] += f" (window rectangle moved by {drift:.0f}px)"

    failures = [result for result in results if result["status"] != "pass"]
    summary = {
        "total": len(results),
        "passed": len(results) - len(failures),
        "failed": len(failures),
        "results": results,
    }
    (args.current / "compare.json").write_text(json.dumps(summary, indent=2))

    print(f"\n{summary['passed']}/{summary['total']} screens passed")
    if failures:
        print("failed screens:")
        for failure in failures:
            print(f"  - {failure['name']}: {failure['status']} ({failure['message']})")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
