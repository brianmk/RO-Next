#!/usr/bin/env python3
"""Regenerate ``docs/rustnarok-inventory.json`` from the committed rustnarok facts.

Inputs (committed by ``tools/extract_rustnarok_facts.py``):

    tools/rustnarok_effects_facts.json
    tools/rustnarok_motion_facts.json
    tools/rustnarok_status_facts.json
    tools/rustnarok_actor_facts.json

Output: ``docs/rustnarok-inventory.json`` — the machine-readable companion of
``docs/rustnarok-audit-2026-11-22.md``. The schema is kept compatible with the
original audit inventory: ``source``, ``summary``, ``effects``,
``skill_motions`` and ``statuses``; the status entries additionally carry the
looping visual effect ids from rustnarok's ``effects.txt``.

The script is deterministic and ``--check`` is the drift guard used in CI.

Usage:
    ./tools/gen_rustnarok_inventory.py
    ./tools/gen_rustnarok_inventory.py --check
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

DEFAULT_FACTS_DIR = Path("tools")
DEFAULT_OUTPUT = Path("docs/rustnarok-inventory.json")
SKILL_EFFECTS = Path("korangar/archive/data/client/skill_effects.ron")
SKILL_ACTIONS = Path("korangar/archive/data/client/skill_actions.ron")
STATUS_EFFECTS = Path("korangar/archive/data/client/status_effects.ron")
LEARNABLE = Path("tools/rathena_pre_re_learnable_skills.json")

MOTION_PRIORITY = ("ATTACK", "THROW", "THIRD_ATTACK", "PICK_UP", "KEEP", "CAST", "STATES")
TABLE_ACTION = {
    "ATTACK": "Attack",
    "THROW": "Attack",
    "THIRD_ATTACK": "Attack",
    "PICK_UP": "Pickup",
    "KEEP": "Idle",
}
CAST_ACTIONS = {0: "Idle", -1: "Idle", 7: "Skill", 2: "Attack", 12: "Attack"}


def git_head() -> str:
    result = subprocess.run(["git", "rev-parse", "HEAD"], text=True, capture_output=True, check=False)
    return result.stdout.strip() if result.returncode == 0 else "unknown"


def load_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def skill_effect_ids() -> set[int]:
    text = SKILL_EFFECTS.read_text(encoding="utf-8")
    return {int(value) for value in re.findall(r"skill: (\d+)", text)}


def status_rows() -> dict[int, str | None]:
    text = STATUS_EFFECTS.read_text(encoding="utf-8")
    rows: dict[int, str | None] = {}
    for match in re.finditer(r"\(\s*id: (\d+),((?:(?!\n\s*\),).)*?)\n\s*\)", text, re.S):
        icon = re.search(r'icon: "([^"]+)"', match.group(2))
        rows[int(match.group(1))] = icon.group(1) if icon else None
    return rows


def action_rows() -> dict[int, str]:
    rows: dict[int, str] = {}
    for line in SKILL_ACTIONS.read_text(encoding="utf-8").splitlines():
        match = re.match(r"\s*\(skill: (\d+), action: (\w+)\)", line)
        if match:
            rows[int(match.group(1))] = match.group(2)
    return rows


def learnable_skills() -> set[int]:
    payload = load_json(LEARNABLE)

    def walk(value: object) -> set[int]:
        found: set[int] = set()
        if isinstance(value, dict):
            if isinstance(value.get("id"), int):
                found.add(int(value["id"]))
            for child in value.values():
                found |= walk(child)
        elif isinstance(value, list):
            for child in value:
                found |= walk(child)
        return found

    return walk(payload)


def build_inventory(facts_dir: Path) -> dict[str, object]:
    effects_facts = load_json(facts_dir / "rustnarok_effects_facts.json")
    motion_facts = load_json(facts_dir / "rustnarok_motion_facts.json")
    status_facts = load_json(facts_dir / "rustnarok_status_facts.json")
    actor_facts = load_json(facts_dir / "rustnarok_actor_facts.json")

    our_skills = skill_effect_ids()
    our_statuses = status_rows()
    our_actions = action_rows()
    learnable = learnable_skills()

    rust_skill_links = {
        int(row["skill"]): set(int(effect) for effect in row["effect_ids"])
        for row in effects_facts.get("skill_links", [])
    }
    rust_status_links = {
        int(row["status"]): set(int(effect) for effect in row["effect_ids"])
        for row in effects_facts.get("status_links", [])
    }

    effects = []
    for row in effects_facts.get("effects", []):
        effect_id = int(row["id"])
        skills = {int(skill) for skill in row.get("skills", [])}
        statuses = {int(status) for status in row.get("statuses", [])}
        covered = bool(skills & our_skills or statuses & set(our_statuses))
        effects.append(
            {
                "id": effect_id,
                "definitions": [definition["text"] for definition in row.get("definitions", [])],
                "skills": sorted(skills),
                "statuses": sorted(statuses),
                "ours": "covered" if covered else "missing",
            }
        )

    motion_by_skill: dict[int, dict[str, object]] = {}
    for table in MOTION_PRIORITY:
        for row in motion_facts.get("tables", {}).get(table, []):
            skill = int(row["skill"])
            if skill in motion_by_skill:
                continue
            if table == "CAST":
                mapped = CAST_ACTIONS.get(int(row.get("value", 0)))
            else:
                mapped = TABLE_ACTION.get(table)
            motion_by_skill[skill] = {
                "skill": skill,
                "rustnarok": table,
                "mapped_action": mapped,
                "ours": our_actions.get(skill),
            }
            if table == "CAST":
                motion_by_skill[skill]["rustnarok_value"] = int(row.get("value", 0))
    motions = [motion_by_skill[skill] for skill in sorted(motion_by_skill)]

    statuses = []
    for row in status_facts.get("statuses", []):
        status_id = int(row["id"])
        statuses.append(
            {
                "id": status_id,
                "icon": row.get("icon"),
                "visual_effect_ids": sorted(int(value) for value in row.get("visual_effect_ids", [])),
                "ours_status": status_id in our_statuses,
                "ours_icon": our_statuses.get(status_id),
            }
        )

    covered_effects = sum(1 for row in effects if row["ours"] == "covered")
    covered_motions = sum(1 for row in motions if row["ours"] is not None)
    learnable_missing = sorted(
        skill
        for skill in learnable
        if skill in rust_skill_links and skill not in our_skills
    )

    summary = {
        "rustnarok_head": effects_facts.get("commit"),
        "rustnarok_effect_ids": len(effects),
        "rustnarok_effect_definitions": sum(len(row["definitions"]) for row in effects),
        "rustnarok_skills_with_effect_mapping": len(rust_skill_links),
        "rustnarok_status_visuals": len(rust_status_links),
        "rustnarok_status_icons": sum(1 for row in statuses if row["icon"] is not None),
        "rustnarok_motion_skills": len(motions),
        "our_effect_skills": len(our_skills),
        "our_status_rows": len(our_statuses),
        "our_action_rows": len(our_actions),
        "effect_ids_covered_heuristic": covered_effects,
        "effect_ids_missing_heuristic": len(effects) - covered_effects,
        "motion_rows_covered_after_port": covered_motions,
        "motion_rows_missing_after_port": len(motions) - covered_motions,
        "learnable_skills_missing_effects": len(learnable_missing),
        "learnable_skills_missing_effects_ids": learnable_missing,
        "rustnarok_actor_job_sprites": actor_facts.get("job_sprite_count"),
        "rustnarok_actor_cart_status": actor_facts.get("cart", {}).get("push_cart_status"),
        "rustnarok_actor_size_statuses": {
            "change_size": actor_facts.get("size_statuses", {}).get("change_size"),
            "change_size_monster": actor_facts.get("size_statuses", {}).get("change_size_monster"),
        },
    }

    return {
        "source": {
            "rustnarok_repo": effects_facts.get("repository"),
            "rustnarok_head": effects_facts.get("commit"),
            "our_repo": "CondaRO-linux/korangar",
            "our_head": git_head(),
        },
        "summary": summary,
        "effects": effects,
        "skill_motions": motions,
        "statuses": statuses,
        "actor": {
            "job_sprites": actor_facts.get("job_sprite_count"),
            "cart": actor_facts.get("cart"),
            "options": actor_facts.get("options"),
            "size_statuses": actor_facts.get("size_statuses"),
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--facts-dir", default=str(DEFAULT_FACTS_DIR))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed inventory is out of date")
    args = parser.parse_args()

    inventory = build_inventory(Path(args.facts_dir))
    output = Path(args.output)
    expected = json.dumps(inventory, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
    current = output.read_text(encoding="utf-8") if output.is_file() else ""

    if args.check:
        if current != expected:
            print(f"error: {output} is out of date; run ./tools/gen_rustnarok_inventory.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({inventory['summary']['rustnarok_effect_ids']} effects, "
              f"{inventory['summary']['rustnarok_motion_skills']} motion skills, "
              f"{inventory['summary']['rustnarok_status_icons']} status icons)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(expected, encoding="utf-8")
    print(f"{output}: {inventory['summary']['rustnarok_effect_ids']} effects, "
          f"{inventory['summary']['rustnarok_skills_with_effect_mapping']} skill mappings, "
          f"{inventory['summary']['rustnarok_motion_skills']} motion skills, "
          f"{inventory['summary']['rustnarok_status_icons']} status icons, "
          f"{inventory['summary']['our_status_rows']} our statuses")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
