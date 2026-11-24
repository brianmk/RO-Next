#!/usr/bin/env python3
"""Extract the per-skill effect facts of the CondaRO client's 2019 tables.

Input: `tools/data/client_skill_tables_2019.txt`, the flat dump of the GRF Lua
tables `SKID` (skill names), `EFID` (effect names), `ACTOR_STATE` and
`SKILL_EFFECT_INFO_LIST` (per-skill effect ids / wave files / begin motion).
The dump is produced by the ignored client test
`world::library::skill_effects::tests::dump_official_skill_tables`, which loads
the tables from `data.grf` / `CondaRO.grf` through the client's patched Lua
loader (non-UTF-8 strings are stored as `hex:` + EUC-KR bytes).

Output: `tools/data/grf2019_skill_facts.json`
"""

from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path

TABLES = Path("tools/data/client_skill_tables_2019.txt")
OUTPUT = Path("tools/data/grf2019_skill_facts.json")

#: SKILL_EFFECT_INFO_LIST field -> fact key used by the generators.
FIELD_KIND = {
    "effectID": "caster",
    "targetEffectID": "target",
    "toTargetEffectID": "target",
    "groundEffectID": "ground",
    "beginEffectID": "begin",
}
WAVE_FIELDS = ("waveFileName", "targetWaveFileName", "beginWaveFileName")


def decode(value: str) -> str:
    if value.startswith("hex:"):
        return bytes.fromhex(value[4:]).decode("cp949", "replace")
    return value


def main() -> None:
    names: dict[str, int] = {}
    id_names: dict[int, str] = {}
    effect_names: dict[int, str] = {}
    states: dict[str, int] = {}
    skills: dict[int, dict] = defaultdict(lambda: {"effects": defaultdict(list), "wavs": [], "begin_motion": None})

    for line in TABLES.read_text().splitlines():
        parts = [decode(part) for part in line.split("|")]
        if not parts:
            continue
        if parts[0] == "skillid" and len(parts) == 3:
            names[parts[2]] = int(parts[1])
            id_names[int(parts[1])] = parts[2]
        elif parts[0] == "effectid" and len(parts) == 3:
            effect_names[int(parts[1])] = parts[2]
        elif parts[0] == "actorstate" and len(parts) == 3:
            states[parts[2]] = int(parts[1])
        elif parts[0] == "skill" and len(parts) == 4:
            skill = int(parts[1])
            field = parts[2].split("[")[0]
            value = parts[3]
            if field in FIELD_KIND:
                skills[skill]["effects"][FIELD_KIND[field]].append(int(value))
            elif field in WAVE_FIELDS:
                skills[skill]["wavs"].append(value)
            elif field == "beginMotionType":
                skills[skill]["begin_motion"] = int(value)

    output = {
        "provenance": {
            "source": "CondaRO 2019 client GRF tables: SKILL_EFFECT_INFO_LIST / SKID / EFID / ACTOR_STATE (data.grf + CondaRO.grf, identical copies)",
            "dump": "tools/data/client_skill_tables_2019.txt",
            "counts": {
                "skill_names": len(names),
                "effect_names": len(effect_names),
                "actor_states": len(states),
                "skills_with_effects": sum(1 for s in skills.values() if s["effects"]),
            },
        },
        "skills": {
            str(skill): {
                "name": id_names.get(skill, str(skill)),
                "effects": {kind: sorted(set(ids)) for kind, ids in data["effects"].items()},
                "wavs": data["wavs"],
                "begin_motion": data["begin_motion"],
            }
            for skill, data in sorted(skills.items())
        },
    }

    OUTPUT.write_text(json.dumps(output, indent=1) + "\n")
    print(f"wrote {OUTPUT}: {len(output['skills'])} skills, "
          f"{sum(1 for s in output['skills'].values() if s['effects'])} with effect ids")


if __name__ == "__main__":
    main()
