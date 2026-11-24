#!/usr/bin/env python3
"""Report how well the committed skill data covers the official client effects.

The audit facts live in `tools/data/rodecomp_effect_coverage.json` (per skill: the
official C++ effect ids, the 2019-era ids the client data uses, and the
renderable assets of the official effect implementations). This tool joins them
with the learnable-skill snapshot and the committed
`korangar/archive/data/client/skill_effects.ron` and classifies every learnable
skill:

* `complete`     - every official asset is rendered by a data layer,
* `partial`      - has rows, but at least one official asset is missing,
* `uncovered`    - no rows, but the official effect has renderable assets,
* `no_effect`    - no official effect and no rows (passive/sound-only skills),
* `flag_only`    - official effect ids, but the implementation renders no asset
                   (sprite blend/blink/outline/body-tint effects),
* `drift`        - flagged on the skills where the 2026 and 2019 id sets are
                   disjoint (informational; followed by complete/partial/...).

Usage:
    ./tools/audit_rodecomp_coverage.py            # human readable report
    ./tools/audit_rodecomp_coverage.py --json     # machine readable
    ./tools/audit_rodecomp_coverage.py --list-gaps
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
FACTS = REPO / "tools" / "data" / "rodecomp_effect_coverage.json"
LEARNABLE = REPO / "tools" / "data" / "rathena_pre_re_learnable_skills.json"
MERGED = REPO / "korangar" / "archive" / "data" / "client" / "skill_effects.ron"


def load_skill_effects_module():
    spec = importlib.util.spec_from_file_location("gen_skill_effects", REPO / "tools" / "gen_skill_effects.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def asset_key(path: str) -> str:
    value = path.replace("/", "\\").lower()
    for prefix in ("data\\texture\\", "data\\sprite\\"):
        if value.startswith(prefix):
            value = value[len(prefix):]
    return value.rsplit(".", 1)[0]


# Renderer features our client implements for sprite-flag effects.
# `Player.cpp` maps the bits: 0x1 body blend, 0x2 weapon blend, 0x4 weapon blink
# (implemented as StatusVisualKind), SetColor -> BodyTint.
IMPLEMENTED_BEHAVIORS = {"tint", "body_blend", "weapon_blend", "weapon_blink"}

FLAG_BEHAVIORS = {
    0x1: "body_blend",
    0x2: "weapon_blend",
    0x4: "weapon_blink",
    0x8: "body_wave",
    0x10: "pulse_scale",
    0x40: "landing_effect",
    0x200: "reflect_body",
    0x400: "giant_body",
    0x800: "outline_blue",
    0x4000: "tint_overlay_blue",
    0x8000: "run",
    0x20000: "blur",
    0x40000: "tint_overlay_red",
    0x80000: "outline_green",
    0x100000: "shadow_form",
}


def effect_behaviors(effect_ids: list[int], flags: dict[str, dict]) -> set[str]:
    behaviors: set[str] = set()

    for effect_id in effect_ids or []:
        entry = flags.get(str(effect_id))
        if not entry:
            continue
        if entry.get("set_color"):
            behaviors.add("tint")
        for bit in entry.get("flags") or []:
            behaviors.add(FLAG_BEHAVIORS.get(int(bit), f"flag_0x{int(bit):x}"))

    return behaviors


def classify() -> tuple[dict[str, list[dict]], dict[str, int]]:
    gen = load_skill_effects_module()
    facts = json.loads(FACTS.read_text(encoding="utf-8"))["skills"]
    learnable = json.loads(LEARNABLE.read_text(encoding="utf-8"))["skills"]

    rendered: dict[int, set[str]] = {}
    for row in gen.read_effects(MERGED):
        assets = rendered.setdefault(int(row["skill"]), set())
        for layer in row.get("layers") or []:
            paths = ([layer["file"]] if layer.get("file") else []) + list(layer.get("files") or []) + list(layer.get("frames") or [])
            for path in paths:
                assets.add(asset_key(path))

    flags_facts = json.loads((REPO / "tools" / "data" / "rodecomp_effect_flags.json").read_text(encoding="utf-8"))["effects"]

    buckets: dict[str, list[dict]] = {name: [] for name in ("complete", "partial", "uncovered", "flag_only", "no_effect")}
    for skill_id, info in sorted(((int(key), value) for key, value in learnable.items())):
        entry = facts.get(str(skill_id))
        name = entry["name"] if entry else (info.get("name") or info.get("description") or str(skill_id))
        if entry is None:
            buckets["no_effect"].append({"skill": skill_id, "name": name})
            continue

        # Compare against the version-correct assets: the 2019 ids for the songs
        # (2026 reworked them), the official ids otherwise.
        used = entry.get("used_assets") or entry["official_assets"]
        # Assets the generator deliberately drops (the decompilation proves the
        # official effect does not draw them) are not required.
        dropped = {
            asset.rsplit(".", 1)[0].replace("/", "\\").lower()
            for (skill, _phase, asset), _reason in gen.DROPPED_LAYERS.items()
            if skill == skill_id
        }
        used = [
            asset
            for asset in used
            if asset.replace("/", "\\").lower().rsplit(".", 1)[0] not in dropped
        ]
        missing = sorted({asset_key(asset) for asset in used} - rendered.get(skill_id, set()))
        drift = bool(entry["official_effect_ids"] and entry["client_effect_ids_2019"]) and not (
            set(entry["official_effect_ids"]) & set(entry["client_effect_ids_2019"])
        )
        row = {
            "skill": skill_id,
            "name": name,
            "drift": drift,
            "official_effect_ids": entry["official_effect_ids"],
            "client_effect_ids_2019": entry["client_effect_ids_2019"],
            "missing_assets": missing,
        }
        if skill_id not in rendered:
            if used:
                buckets["uncovered"].append(row)
            elif entry["official_effect_ids"]:
                behaviors = effect_behaviors(entry["official_effect_ids"], flags_facts)
                row["behaviors"] = sorted(behaviors)
                row["needs_renderer_feature"] = sorted(behaviors - IMPLEMENTED_BEHAVIORS)
                buckets["flag_only"].append(row)
            else:
                buckets["no_effect"].append(row)
        elif missing:
            buckets["partial"].append(row)
        else:
            buckets["complete"].append(row)

    summary = {name: len(rows) for name, rows in buckets.items()}
    summary["flag_only_covered"] = sum(1 for row in buckets["flag_only"] if not row["needs_renderer_feature"])
    summary["flag_only_needs_feature"] = sum(1 for row in buckets["flag_only"] if row["needs_renderer_feature"])
    summary["learnable_skills"] = len(learnable)
    return buckets, summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--json", action="store_true", help="machine readable output")
    parser.add_argument("--list-gaps", action="store_true", help="list every partial/uncovered skill")
    args = parser.parse_args()

    buckets, summary = classify()
    drift = sum(1 for rows in buckets.values() for row in rows if row.get("drift"))

    if args.json:
        print(json.dumps({"summary": summary, "drift": drift, "buckets": buckets}, indent=1, ensure_ascii=False))
        return 0

    print(f"{summary['learnable_skills']} learnable pre-renewal skills, official effect facts for {FACTS.name}")
    print(f"  complete    {summary['complete']:4}  every official asset is rendered")
    print(f"  partial     {summary['partial']:4}  has rows, misses {sum(len(row['missing_assets']) for row in buckets['partial'])} official assets")
    print(f"  uncovered   {summary['uncovered']:4}  official effect assets, no rows")
    print(
        f"  flag_only   {summary['flag_only']:4}  official effect renders no asset "
        f"({summary['flag_only_covered']} covered by tint/blend/blink, {summary['flag_only_needs_feature']} need a renderer feature)"
    )
    print(f"  no_effect   {summary['no_effect']:4}  no official effect (passive/sound-only)")
    print(f"  drift       {drift:4}  skills whose 2026/2019 effect ids are disjoint")

    if args.list_gaps:
        for bucket in ("partial", "uncovered", "flag_only"):
            print(f"\n{bucket}:")
            for row in buckets[bucket]:
                assets = ", ".join(row["missing_assets"][:4]) or "-"
                drift_note = " [drift]" if row.get("drift") else ""
                print(f"  {row['skill']:5} {row['name'][:28]:28} {assets}{drift_note}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
