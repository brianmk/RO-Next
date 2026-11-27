#!/usr/bin/env python3
"""Extract the official status/skill -> effect launches the port wires.

Two independent official switches launch a persistent effect from state:

* `CGameActor::SendMsg`'s `case AM_STATUS_CHANGE:` (`GameObject.cpp`) launches
  the effect of an `EFST_*` status on every status change, and
  `CPlayer::ReapplySkillEffects` (`Player.cpp`) re-launches the same rows when
  the client rebuilds the actor (map entry, map property change).
* `CGameMode::ApplyActorSkillState` (`GameModePackets.cpp`) launches the cast
  start effect of a skill (`CH_SOULCOLLECT` -> `EF_PORTAL5`) when the cast
  actually starts (`NotifySkillCasting` returns early for `delay == 0`).

Only the triggers the port wires are extracted (`STATUS_TRIGGERS`,
`SKILL_TRIGGERS` below); add a name together with the source switch so a rerun
never adds unreviewed launches.

Numeric ids:
* status ids come from `tools/data/status_facts.json` (rAthena `efst_type`, the
  id the server sends in `ZC_MSG_STATE_CHANGE`), cross-checked against the
  decompiled `efst_type` enum;
* effect ids come from the decompiled `RagEffectID` enum
  (`Source/Contents/Effects/Core/EffectTypes.h`), the ids the official client
  passes to `LaunchEffect`;
* skill ids come from `tools/data/rathena_pre_re_learnable_skills.json`.

Input (read-only): /home/phantom/Documents/deepseek-harness/default-workspace/rodecomp
Output: tools/data/status_effect_launches_facts.json

Usage: ./tools/extract_status_effect_launches.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
RODECOMP = Path("/home/phantom/Documents/deepseek-harness/default-workspace/rodecomp")
EFFECT_TYPES = RODECOMP / "Source/Contents/Effects/Core/EffectTypes.h"
GAME_OBJECT = RODECOMP / "Source/Contents/Actors/GameObject.cpp"
PLAYER = RODECOMP / "Source/Contents/Actors/Player.cpp"
GAME_MODE_PACKETS = RODECOMP / "Source/Client/Mode/GameModePackets.cpp"
STATUS_FACTS = REPO / "tools/data/status_facts.json"
SKILL_FACTS = REPO / "tools/data/rathena_pre_re_learnable_skills.json"
OUTPUT = REPO / "tools/data/status_effect_launches_facts.json"

#: Statuses whose official launch rows are extracted. Each name must exist in
#: both the rAthena fact table and the decompiled `efst_type` enum.
STATUS_TRIGGERS = [
    # `case EFST_STEELBODY: case EFST_DBARRIER:` is one case group
    # (`GameObject.cpp:2684-2702`); `ReapplySkillEffects` has one case each
    # (`Player.cpp:3256-3259`, `Player.cpp:3322-3325`).
    "EFST_STEELBODY",
    "EFST_DBARRIER",
]

#: Skills whose `ApplyActorSkillState` launch rows are extracted.
SKILL_TRIGGERS = [
    # `GameModePackets.cpp:12883-12886`.
    "CH_SOULCOLLECT",
]

#: Source ranges of the three switches (1-based, exclusive end).
AM_STATUS_CHANGE = (GAME_OBJECT, 2535, 4399)
REAPPLY_SKILL_EFFECTS = (PLAYER, 3180, 3851)
APPLY_ACTOR_SKILL_STATE = (GAME_MODE_PACKETS, 12868, 12897)

LAUNCH_RE = re.compile(r"LaunchEffect\(\s*(EF_\w+)\s*,")
CASE_RE = re.compile(r"^(\s*)case\s+([A-Za-z_]\w*)\s*:\s*\{?\s*$")


def enum_values(path: Path, enum_name: str) -> dict[str, int]:
    """`name -> value` of a C++ enum, handling implicit increments."""
    source = path.read_text(encoding="utf-8")
    match = re.search(r"enum\s+" + re.escape(enum_name) + r"\s*:\s*[^{]+\{", source)
    if match is None:
        raise SystemExit(f"missing enum {enum_name} in {path}")
    end = source.find("};", match.end())
    body = source[match.end() : end]
    value = -1
    values: dict[str, int] = {}
    for raw in body.splitlines():
        line = raw.split("//")[0].strip().rstrip(",")
        if not line:
            continue
        entry = re.match(r"([A-Za-z_]\w*)(?:\s*=\s*(-?\d+))?$", line)
        if entry is None:
            continue
        value = int(entry.group(2)) if entry.group(2) is not None else value + 1
        name = entry.group(1)
        if name in values and values[name] != value:
            raise SystemExit(f"enum {enum_name}: duplicate {name} ({values[name]} vs {value})")
        values[name] = value
    return values


def case_block(lines: list[str], start: int, end: int, name: str) -> tuple[list[str], int, int] | None:
    """Returns `(case labels, first body line index, last body line index)`.

    A `case` group is every consecutive `case`/`default` label plus the body up
    to the next label at the same indentation. `idx` is 0-based over `lines`,
    which is the whole file.
    """
    index = start
    while index < end:
        match = CASE_RE.match(lines[index])
        if match is None or match.group(2) != name:
            index += 1
            continue

        indent = len(match.group(1))
        labels = [name]
        cursor = index + 1
        while cursor < end:
            sibling = CASE_RE.match(lines[cursor])
            if sibling is not None and len(sibling.group(1)) == indent and sibling.group(2) != "default":
                labels.append(sibling.group(2))
                cursor += 1
                continue
            if sibling is not None and len(sibling.group(1)) == indent and sibling.group(2) == "default":
                # `default:` is part of the previous case block only.
                cursor += 1
                continue
            break

        body = cursor
        while body < end:
            sibling = CASE_RE.match(lines[body])
            if sibling is not None and len(sibling.group(1)) <= indent:
                break
            if re.match(rf"^\s{{0,{indent}}}default\s*:\s*$", lines[body]):
                break
            body += 1
        return labels, cursor, body
    return None


def launches_in(lines: list[str], start: int, end: int) -> list[dict[str, object]]:
    result: list[dict[str, object]] = []
    for index in range(start, end):
        for effect in LAUNCH_RE.finditer(lines[index]):
            result.append({"effect": effect.group(1), "line": index + 1})
    return result


def extract_switch(path: Path, start: int, end: int, names: list[str]) -> dict[str, dict[str, object]]:
    """Per case name: the official `LaunchEffect` rows and the cleanup style."""
    lines = path.read_text(encoding="utf-8").splitlines()
    result: dict[str, dict[str, object]] = {}
    for name in names:
        block = case_block(lines, start - 1, min(end, len(lines)), name)
        if block is None:
            raise SystemExit(f"{path.name}: no `case {name}:` in lines {start}-{end}")
        labels, body_start, body_end = block
        body = "\n".join(lines[body_start:body_end])
        launches = launches_in(lines, body_start, body_end)
        if not launches:
            raise SystemExit(f"{path.name}: `case {name}:` has no LaunchEffect")
        result[name] = {
            "labels": labels,
            "launches": launches,
            # `while (this->HasId(EF_X)) { delete; }` right before the launch:
            # the official code replaces the previous instance.
            "deletes_before_launch": "while (this->HasId(" in body,
            "first_line": body_start + 1,
            "last_line": body_end,
        }
    return result


def main() -> None:
    if not RODECOMP.exists():
        raise SystemExit(f"missing decompilation: {RODECOMP}")

    effect_ids = enum_values(EFFECT_TYPES, "RagEffectID")
    decomp_efst = enum_values(EFFECT_TYPES, "efst_type")

    status_facts = json.loads(STATUS_FACTS.read_text(encoding="utf-8"))
    rathena_efst = {row["name"]: int(row["id"]) for row in status_facts["efst_enum"]}
    known_statuses = {int(row["id"]) for row in status_facts["statuses"]}

    skill_facts = json.loads(SKILL_FACTS.read_text(encoding="utf-8"))["skills"]
    rathena_skills = {info["name"]: int(info["id"]) for info in skill_facts.values()}

    apply_switch = extract_switch(*AM_STATUS_CHANGE, STATUS_TRIGGERS)
    reapply_switch = extract_switch(*REAPPLY_SKILL_EFFECTS, STATUS_TRIGGERS)

    statuses = []
    for name in STATUS_TRIGGERS:
        if name not in rathena_efst:
            raise SystemExit(f"{name} is not in the rAthena efst_type table")
        if name not in decomp_efst:
            raise SystemExit(f"{name} is not in the decompiled efst_type enum")
        status_id = rathena_efst[name]
        if status_id != decomp_efst[name]:
            raise SystemExit(f"{name}: rAthena id {status_id} != decompiled id {decomp_efst[name]}")
        if status_id not in known_statuses:
            raise SystemExit(f"{name}: id {status_id} has no row in status_facts.statuses")

        apply = apply_switch[name]
        reapply = reapply_switch[name]

        effects = []
        for launch in apply["launches"]:
            effect = launch["effect"]
            if effect not in effect_ids:
                raise SystemExit(f"{effect} is not in the decompiled RagEffectID enum")
            reapply_lines = [row["line"] for row in reapply["launches"] if row["effect"] == effect]
            if not reapply_lines:
                raise SystemExit(f"{name}: {effect} has no ReapplySkillEffects launch")
            effects.append(
                {
                    "effect": effect,
                    "effect_id": effect_ids[effect],
                    "apply_line": launch["line"],
                    "reapply_lines": reapply_lines,
                }
            )

        apply_effects = {row["effect"] for row in apply["launches"]}
        reapply_effects = {row["effect"] for row in reapply["launches"]}
        if apply_effects != reapply_effects:
            raise SystemExit(f"{name}: apply {sorted(apply_effects)} != reapply {sorted(reapply_effects)}")

        statuses.append(
            {
                "status": name,
                "status_id": status_id,
                "apply_range": [apply["first_line"], apply["last_line"]],
                "deletes_before_launch": apply["deletes_before_launch"],
                "effects": effects,
            }
        )

    skill_switch = extract_switch(*APPLY_ACTOR_SKILL_STATE, SKILL_TRIGGERS)
    skills = []
    for name in SKILL_TRIGGERS:
        if name not in rathena_skills:
            raise SystemExit(f"{name} is not in the rAthena skill table")
        skill = skill_switch[name]
        launches = skill["launches"]
        if len(launches) != 1:
            raise SystemExit(f"{name}: expected one LaunchEffect, found {len(launches)}")
        launch = launches[0]
        effect = launch["effect"]
        if effect not in effect_ids:
            raise SystemExit(f"{effect} is not in the decompiled RagEffectID enum")
        skills.append(
            {
                "skill": name,
                "skill_id": rathena_skills[name],
                "effect": effect,
                "effect_id": effect_ids[effect],
                "line": launch["line"],
            }
        )

    facts = {
        "schema": 1,
        "provenance": {
            "source": "github.com/rodecomp/rodecomp - CGameActor::SendMsg case AM_STATUS_CHANGE (GameObject.cpp), "
            "CPlayer::ReapplySkillEffects (Player.cpp) and CGameMode::ApplyActorSkillState (GameModePackets.cpp)",
            "note": "status/skill ids are the rAthena ids the server sends and match the decompiled enums; effect ids are the "
            "decompiled RagEffectID values. Only the triggers wired by the port are extracted "
            "(STATUS_TRIGGERS/SKILL_TRIGGERS in extract_status_effect_launches.py).",
        },
        "statuses": statuses,
        "skills": skills,
    }

    OUTPUT.write_text(json.dumps(facts, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"wrote {OUTPUT} ({len(statuses)} statuses, {len(skills)} skills)")

    for row in statuses:
        for effect in row["effects"]:
            print(
                f"  {row['status']} ({row['status_id']}) -> {effect['effect']} ({effect['effect_id']}) "
                f"apply:{effect['apply_line']} reapply:{effect['reapply_lines']}"
            )
    for row in skills:
        print(f"  {row['skill']} ({row['skill_id']}) -> {row['effect']} ({row['effect_id']}) line:{row['line']}")


if __name__ == "__main__":
    sys.exit(main())
