#!/usr/bin/env python3
"""Extract the official skill -> caster action facts from the Windows client tables.

Background: the official Windows client does **not** play one generic "skill"
action for every skill. It keeps a hard-coded skill id -> entity action table
(`DB/Skills/SkillAction.js` in the community reimplementation roBrowserLegacy,
GPL-3.0) and looks the action up on every skill use. This script extracts only
the **facts** (numeric skill ids + the action name + whether the action is a
held pose) into a plain JSON file; none of the GPL source is copied.

The extracted JSON (`tools/data/windows_client_skill_actions.json`) is the input of
`gen_skill_actions.py`, which writes the client data file
`ro-next/archive/data/client/skill_actions.ron`.

Usage (needs network access to raw.githubusercontent.com):

    ./tools/extract_skill_actions.py --source-dir /tmp/robrowserlegacy

or let the script download the two tables itself.

Only read-only HTTP requests are made; the script writes a single JSON file.
"""

from __future__ import annotations

import argparse
import json
import re
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/data/windows_client_skill_actions.json")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"
FILES = {
    "skill_const": f"{BASE_URL}/Skills/SkillConst.js",
    "skill_action": f"{BASE_URL}/Skills/SkillAction.js",
}

#: Official action name -> client action category used by ro-next.
ACTION_CATEGORIES = {
    "ATTACK": "Attack",
    "ATTACK1": "Attack",
    "ATTACK2": "Attack",
    "ATTACK3": "Attack",
    "IDLE": "Idle",
    "PICKUP": "Pickup",
    "SKILL": "Skill",
    "READYFIGHT": "ReadyFight",
    # `ACTION` is the client's generic "use/operate" pose; the closest ro-next
    # action is the casting gesture.
    "ACTION": "Skill",
}


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as handle:
        return handle.read().decode("utf-8")


def read_sources(source_dir: str | None) -> dict[str, str]:
    sources: dict[str, str] = {}
    for key, url in FILES.items():
        if source_dir is not None:
            sources[key] = Path(source_dir, url.rsplit("/", 1)[-1]).read_text()
        else:
            sources[key] = fetch(url)
    return sources


def parse_skill_const(text: str) -> dict[str, int]:
    return {match.group(1): int(match.group(2)) for match in re.finditer(r"([A-Z0-9_]+)\s*:\s*(\d+)", text)}


def strip_comments(text: str) -> str:
    return re.sub(r"//[^\n]*", "", text)


def skip_whitespace(text: str, index: int) -> int:
    while index < len(text) and text[index] in " \t\r\n":
        index += 1
    return index


def skip_braced(text: str, index: int) -> int:
    """Return the index after the `{...}` block that starts at `index`."""
    depth = 0
    while index < len(text):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    raise SystemExit("unbalanced braces in SkillAction.js")


def parse_defaults(text: str) -> dict[str, str]:
    defaults: dict[str, str] = {}
    for match in re.finditer(r"SkillAction\['(DEFAULT[A-Z_]*)'\]\s*=\s*makeGenericSkillAction\('([A-Z0-9]+)'", text):
        defaults[match.group(1)] = ACTION_CATEGORIES.get(match.group(2), "Skill")
    return defaults


def parse_actions(text: str) -> dict[int, dict]:
    """Return skill id -> {"action": <category>, "hold": <bool>}."""
    text = strip_comments(text)
    actions: dict[int, dict] = {}
    assignment = re.compile(r"SkillAction\[SK\.([A-Z0-9_]+)\]\s*=\s*")
    position = 0

    while True:
        match = assignment.search(text, position)
        if match is None:
            break
        names = [match.group(1)]
        index = match.end()

        # A group is a chain of assignments (`A = B = ... = factory(...)`).
        while True:
            chained = re.match(r"\s*SkillAction\[SK\.([A-Z0-9_]+)\]\s*=\s*", text[index:])
            if chained is None:
                break
            names.append(chained.group(1))
            index += chained.end()

        index = skip_whitespace(text, index)
        rest = text[index:]

        if rest.startswith("makeAttackSkillAction"):
            category, hold = "Attack", False
            index += re.match(r"makeAttackSkillAction\([^)]*\)", rest).end()
        elif rest.startswith("makeGenericSkillAction"):
            factory = re.match(r"makeGenericSkillAction\('([A-Z0-9]+)'", rest)
            category, hold = ACTION_CATEGORIES.get(factory.group(1), "Skill"), False
            index += re.match(r"makeGenericSkillAction\([^)]*\)", rest).end()
        elif rest.startswith("makeSliceAttackAction"):
            category, hold = "Attack", False
            index += re.match(r"makeSliceAttackAction\([^)]*\)", rest).end()
        elif rest.startswith("false"):
            # "Prevent default skill action": the client keeps the current pose.
            category, hold = "Idle", False
            index += len("false")
        elif rest.startswith("function"):
            body_start = index + rest.index("{")
            body_end = skip_braced(text, body_start)
            body = text[body_start:body_end]
            action = re.search(r"action:\s*entity\.ACTION\.([A-Z0-9_]+)", body)
            play = re.search(r"play:\s*(true|false)", body)
            category = ACTION_CATEGORIES.get(action.group(1), "Skill") if action else "Skill"
            hold = bool(play and play.group(1) == "false")
            index = body_end
        else:
            raise SystemExit(f"unparsed SkillAction.js entry {names}: {rest[:60]!r}")

        position = index
        for name in names:
            actions[name] = {"action": category, "hold": hold}

    return actions


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", default=None, help="local checkout directory of the two JS tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    sources = read_sources(args.source_dir)
    skill_ids = parse_skill_const(sources["skill_const"])
    actions = parse_actions(sources["skill_action"])
    defaults = parse_defaults(sources["skill_action"])

    resolved: dict[int, dict] = {}
    unknown: list[str] = []
    for name, entry in actions.items():
        skill_id = skill_ids.get(name)
        if skill_id is None:
            unknown.append(name)
            continue
        resolved[skill_id] = entry

    payload = {
        "provenance": {
            "source": "roBrowserLegacy (GPL-3.0) RE dump of the official client tables; only numeric ids, action names and hold flags are extracted",
            "url": FILES["skill_action"],
            "note": "official skill -> entity action assignment, including the 'prevent default skill action' entries",
        },
        "defaults": defaults,
        "actions": {str(skill_id): entry for skill_id, entry in sorted(resolved.items())},
    }

    output = Path(args.output)
    output.write_text(json.dumps(payload, indent=1) + "\n")
    print(f"{output}: {len(resolved)} skills, defaults {defaults}")
    if unknown:
        print(f"warning: {len(unknown)} SkillAction symbols without a SkillConst id: {unknown[:10]}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
