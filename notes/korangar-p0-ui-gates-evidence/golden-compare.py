#!/usr/bin/env python3
"""Compare two golden capture runs (before/after) screen by screen."""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def load_rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.int16)


def dilate(mask, iterations=2):
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


def animation_mask(first, second, threshold=6):
    if first.shape != second.shape:
        raise SystemExit(f"animation frame size mismatch: {first.shape} != {second.shape}")
    difference = np.abs(first - second).max(axis=2)
    return dilate(difference > threshold, iterations=2)


def relative_masks_to_pixels(relative_masks, width, height):
    mask = np.zeros((height, width), dtype=bool)
    for left, top, w, h in relative_masks:
        l = max(0, int(left * width))
        t = max(0, int(top * height))
        r = min(width, int((left + w) * width))
        b = min(height, int((top + h) * height))
        mask[t:b, l:r] = True
    return mask


def compare_variant(before_dir: Path, after_dir: Path, pixel_threshold=24, max_changed_ratio=0.004, max_rmse=4.0):
    before = json.loads((before_dir / "capture.json").read_text())
    after = json.loads((after_dir / "capture.json").read_text())
    after_captures = {entry["name"]: entry for entry in after["captures"]}
    failures = 0
    rows = []
    for entry in before["captures"]:
        name = entry["name"]
        other = after_captures.get(name)
        if other is None:
            rows.append((name, "MISSING", "", ""))
            failures += 1
            continue
        if entry["rect"] != other["rect"]:
            rows.append((name, "RECT", str(entry["rect"]), str(other["rect"])))
            failures += 1
            continue
        b1 = load_rgb(before_dir / f"{name}.png")
        b2 = load_rgb(before_dir / f"{name}.second.png")
        a1 = load_rgb(after_dir / f"{name}.png")
        a2 = load_rgb(after_dir / f"{name}.second.png")
        if not (b1.shape == b2.shape == a1.shape == a2.shape):
            rows.append((name, "SIZE", str(b1.shape), str(a1.shape)))
            failures += 1
            continue
        mask = animation_mask(b1, b2) | animation_mask(a1, a2)
        mask |= relative_masks_to_pixels(entry.get("masks", []), b1.shape[1], b1.shape[0])
        diff = np.abs(b1 - a1).max(axis=2)
        changed = (diff > pixel_threshold) & ~mask
        total = int((~mask).sum())
        changed_pixels = int(changed.sum())
        ratio = changed_pixels / max(total, 1)
        rmse = float(np.sqrt((((b1 - a1).astype(np.float64) ** 2) * (~mask)[..., None]).mean()))
        status = "OK" if ratio <= max_changed_ratio and rmse <= max_rmse else "DIFF"
        if status != "OK":
            failures += 1
        rows.append((name, status, f"changed={changed_pixels}/{total} ({ratio:.6f})", f"rmse={rmse:.4f}"))
    return rows, failures


def main():
    before_root = Path(sys.argv[1])
    after_root = Path(sys.argv[2])
    total_failures = 0
    for before_dir in sorted(p for p in before_root.iterdir() if p.is_dir() and (p / "capture.json").exists()):
        after_dir = after_root / before_dir.name
        if not (after_dir / "capture.json").exists():
            print(f"{before_dir.name}: after missing")
            total_failures += 1
            continue
        rows, failures = compare_variant(before_dir, after_dir)
        total_failures += failures
        print(f"== {before_dir.name}")
        for name, status, changed, rmse in rows:
            print(f"   {status:8} {name:22} {changed:45} {rmse}")
    print(f"\n{'PASS' if total_failures == 0 else 'FAIL'}: {total_failures} screen failure(s)")
    return 0 if total_failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
