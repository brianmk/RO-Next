#!/usr/bin/env python3
"""Audit the client's caster actions against the decompiled official motions.

Sources:
  tools/skill_actions_rodecomp.json        official motion per skill
                                           (CSession::GetSkillActionInfo/2)
  tools/rodecomp_skill_frame_sequences.json explicit ACT block per skill
                                           (CRenderObject::SetFrameSequence)
  korangar/archive/data/client/skill_actions.ron

Every skill listed in the decompilation facts must exist in the data file, and
its action must match the official motion mapping
(0=Idle, 2=Attack, 5=Pickup, 7/8=Skill) unless an explicit frame sequence
overrides it (0x58=Attack, 0x60=Skill).

Usage: ./tools/audit_skill_actions.py
"""

from __future__ import annotations

import json
import re
from pathlib import Path

FACTS = Path("tools/skill_actions_rodecomp.json")
SEQUENCES = Path("tools/rodecomp_skill_frame_sequences.json")
DATA = Path("korangar/archive/data/client/skill_actions.ron")

MOTION_ACTION = {0: "Idle", 2: "Attack", 5: "Pickup", 7: "Skill", 8: "Skill"}


def main() -> int:
    rows = json.loads(FACTS.read_text())["overrides"]
    sequences = json.loads(SEQUENCES.read_text())["sequences"] if SEQUENCES.is_file() else {}
    data = dict(re.findall(r"\(skill: (\d+), action: (\w+(?:\(\d+\))?)\)", DATA.read_text()))

    expected_by_skill: dict[int, str] = {}
    for name, entry in sequences.items():
        block = entry["block"]
        if block == "0x58":
            expected_by_skill[entry["skill"]] = "Attack"
        elif block == "0x60":
            expected_by_skill[entry["skill"]] = "Skill"

    missing: list[int] = []
    mismatches: list[tuple[int, str, str]] = []
    unchecked = 0

    for row in rows:
        skill = int(row["skill"])
        ours = data.get(str(skill))
        expected = expected_by_skill.get(skill) or MOTION_ACTION.get(row.get("motion"))

        if ours is None:
            missing.append(skill)
        elif expected is None:
            unchecked += 1
        elif ours != expected:
            mismatches.append((skill, ours, expected))

    print(f"skills with official motion facts: {len(rows)}")
    print(f"  missing from skill_actions.ron:  {len(missing)}")
    print(f"  action mismatches:               {len(mismatches)}")
    print(f"  no single-action equivalent:     {unchecked}")
    for skill, ours, expected in mismatches:
        print(f"    skill {skill}: ours {ours}, official {expected}")
    for skill in missing:
        print(f"    missing skill {skill}")

    return 1 if missing or mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
