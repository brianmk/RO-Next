#!/usr/bin/env python3
"""Report how well the committed skill data covers the official client effects.

The audit facts live in `tools/rodecomp_effect_coverage.json` (per skill: the
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
FACTS = REPO / "tools" / "rodecomp_effect_coverage.json"
LEARNABLE = REPO / "tools" / "rathena_pre_re_learnable_skills.json"
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
                buckets["flag_only"].append(row)
            else:
                buckets["no_effect"].append(row)
        elif missing:
            buckets["partial"].append(row)
        else:
            buckets["complete"].append(row)

    summary = {name: len(rows) for name, rows in buckets.items()}
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
    print(f"  flag_only   {summary['flag_only']:4}  official effect renders no asset (blend/blink/outline/tint)")
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
