#!/usr/bin/env python3
"""Extract the facts needed for *coverage*: every learnable pre-re skill and
which pre-re classes can learn it.

The unified skill-effect data file covers **all** skills the official effect
table maps, but the acceptance criterion is "every learnable pre-renewal
skill", so the generator needs a stable, offline snapshot of:

* the full pre-re skill database (`db/pre-re/skill_db.yml`): id, Aegis name,
  description, skill type, target type and max level,
* the pre-re skill tree (`db/pre-re/skill_tree.yml`): which class learns which
  skill (after `Inherit` resolution).

rAthena is GPL-3.0; only plain facts are extracted here (no code, no comments),
into `tools/data/rathena_pre_re_learnable_skills.json`, so
`tools/gen_skill_effect_layers.py` stays deterministic without a local rAthena
checkout and `--check` works on machines that only have the client data.

Usage:
    ./tools/extract_rathena_learnable_skills.py --rathena-dir /path/to/rathena
    ./tools/extract_rathena_learnable_skills.py --check      # drift guard
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

DEFAULT_RATHENA_DIR = "/home/phantom/rathena"
DEFAULT_OUTPUT = Path("tools/data/rathena_pre_re_learnable_skills.json")
SKILL_DB = "db/pre-re/skill_db.yml"
SKILL_TREE = "db/pre-re/skill_tree.yml"


def load_yaml(path: Path) -> dict:
    import yaml

    return yaml.safe_load(path.read_text(encoding="utf-8"))


def resolve_job_skills(job: str, jobs: dict[str, dict], seen: set[str] | None = None) -> set[str]:
    """Return the skill names a class can learn, following `Inherit` parents."""
    seen = seen if seen is not None else set()
    if job in seen or job not in jobs:
        return set()
    seen.add(job)
    names = set(jobs[job]["tree"])
    for parent in jobs[job]["inherit"]:
        names |= resolve_job_skills(parent, jobs, seen)
    return names


def build_payload(rathena_dir: Path) -> dict:
    skill_db = load_yaml(rathena_dir / SKILL_DB)
    skill_tree = load_yaml(rathena_dir / SKILL_TREE)

    skills: dict[int, dict] = {}
    for skill in skill_db["Body"]:
        skills[skill["Id"]] = {
            "id": skill["Id"],
            "name": skill["Name"],
            "description": skill.get("Description", ""),
            "type": skill.get("Type", "None"),
            "target_type": skill.get("TargetType", "Passive"),
            "max_level": skill.get("MaxLevel", 1),
        }
    name_to_id = {skill["name"]: skill_id for skill_id, skill in skills.items()}

    jobs: dict[str, dict] = {}
    for node in skill_tree["Body"]:
        jobs[node["Job"]] = {
            "inherit": sorted((node.get("Inherit") or {}).keys()),
            "tree": sorted({entry["Name"] for entry in node.get("Tree") or [] if entry.get("MaxLevel", 1) != 0}),
        }

    # Effective learnable skill names per class (inheritance resolved).
    class_skills: dict[str, list[int]] = {}
    skill_jobs: dict[int, list[str]] = {}
    for job in sorted(jobs):
        names = resolve_job_skills(job, jobs)
        ids = sorted({name_to_id[name] for name in names if name in name_to_id})
        class_skills[job] = ids
        for skill_id in ids:
            skill_jobs.setdefault(skill_id, []).append(job)

    learnable = sorted(skill_jobs)
    payload_skills = {}
    for skill_id in learnable:
        entry = dict(skills[skill_id])
        entry["jobs"] = skill_jobs[skill_id]
        payload_skills[str(skill_id)] = entry

    return {
        "provenance": {
            "source": "rAthena pre-renewal skill database (facts only: ids, names, types, levels, class trees)",
            "files": [SKILL_DB, SKILL_TREE],
            "note": "learnable = listed with MaxLevel != 0 in at least one class skill tree (Inherit resolved)",
        },
        "classes": {job: {"inherit": jobs[job]["inherit"], "skills": class_skills[job]} for job in sorted(jobs)},
        "skills": payload_skills,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rathena-dir", default=DEFAULT_RATHENA_DIR)
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed fact file is out of date")
    args = parser.parse_args()

    rathena_dir = Path(args.rathena_dir)
    if not (rathena_dir / SKILL_DB).is_file():
        raise SystemExit(f"rAthena pre-re skill database not found: {rathena_dir / SKILL_DB}")

    payload = build_payload(rathena_dir)
    content = json.dumps(payload, indent=1, ensure_ascii=False, sort_keys=True) + "\n"
    output = Path(args.output)

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/extract_rathena_learnable_skills.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(payload['skills'])} learnable skills)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {len(payload['skills'])} learnable skills, {len(payload['classes'])} classes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
