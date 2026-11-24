#!/usr/bin/env python3
"""Generate the client data file with the official caster action of every skill.

The official Windows client keeps a hard-coded skill id -> entity action table
and plays that action when a skill is used (attack skills swing the weapon,
stance/buff skills keep the idle pose, trap skills use the pickup animation,
...). The facts are extracted from the official client tables by
`tools/extract_skill_actions.py` into
`tools/windows_client_skill_actions.json`, and this generator turns them into a
data file that the client loads at runtime through the regular
`GameFileLoader`:

    korangar/archive/data/client/skill_actions.ron

The file is embedded with `include_str!` as the compiled fallback, so the data
file and the client cannot drift apart. A server can override the mapping
without rebuilding the client, exactly like the skill effect mapping.

Actions understood by the client (`world::CasterAction`):

* `Attack`     - play the weapon attack animation,
* `Idle`       - keep the idle pose (stance/buff skills and "prevent default"),
* `Pickup`     - play the pickup animation (trap/gather skills),
* `Skill`      - play the casting gesture (default for unlisted skills),
* `ReadyFight` - play the combat-ready stance (official `SM_ENDURE`),
* `Action(n)`  - play ACT action index `n` (rustnarok generic motion table).

Usage:
    ./tools/gen_skill_actions.py            # regenerate the data file
    ./tools/gen_skill_actions.py --check    # fail if the file is out of date
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_FACTS = Path("tools/windows_client_skill_actions.json")
DEFAULT_EXTRA = Path("tools/skill_actions_extra.json")
DEFAULT_RODECOMP = Path("tools/skill_actions_rodecomp.json")
DEFAULT_GRF2019 = Path("tools/grf2019_skill_facts.json")

#: `ACTOR_STATE` id of the 2019 client tables -> our caster action.
#:
#: `CGameActor::SetState` maps the actor state to an ACT action block; the
#: indices below are the blocks of that mapping (ST_ATTACK2 -> block 11,
#: ST_NINJASKILL2/ST_NINJAREADY -> the skill block 12).
GRF2019_MOTION_ACTION = {
    0: "Idle",        # ST_STAND
    2: "Attack",      # ST_ATTACK
    5: "Pickup",      # ST_PICKUP
    7: "Skill",       # ST_SKILL
    8: "Skill",       # ST_BEGIN_SKILL
    9: "Action(11)",  # ST_ATTACK2
    36: "Skill",      # ST_NINJASKILL2 -> block 96 (skill)
    37: "Skill",      # ST_NINJAREADY -> block 96 (skill)
}
DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_actions.ron")

#: Client action categories (must match `CasterAction` in
#: `korangar/src/world/skill_action.rs`).
ACTIONS = ("Attack", "Idle", "Pickup", "Skill", "ReadyFight")
ACTION_INDEX_PATTERN = re.compile(r"^Action\(\d+\)$")


def validate_action(skill_id: int, action: str, kind: str) -> None:
    if action not in ACTIONS and not ACTION_INDEX_PATTERN.match(action):
        raise SystemExit(f"skill {skill_id}: unsupported {kind} action {action!r}")


def load_facts(path: Path, extra_path: Path | None = None, rodecomp_path: Path | None = None) -> list[tuple[int, str]]:
    payload = json.loads(path.read_text())
    actions: dict[int, str] = {}
    for skill_id, entry in payload["actions"].items():
        action = entry["action"]
        validate_action(int(skill_id), action, "official")
        actions[int(skill_id)] = action

    if extra_path is not None and extra_path.is_file():
        extra = json.loads(extra_path.read_text())

        # Curated additions for skills the official roBrowser dump does not cover
        # (ported from rustnarok-client's per-skill motion tables where the motion
        # maps onto one of our five actions). They only fill gaps: an existing
        # official row always wins.
        for entry in extra.get("actions", []):
            skill_id = int(entry["skill"])
            action = entry["action"]
            validate_action(skill_id, action, "extra")
            actions.setdefault(skill_id, action)

        # Curated corrections of the roBrowser action dump. The official client
        # picks the caster pose from `CSession::GetSkillActionInfo`, whose
        # `motion` is an ACTORSTATEID, not one of the five roBrowser action
        # names. `CGameActor::SetState` maps the state to a (flat) ACT animation
        # index via `ActionBase`/`WeaponActionBase`; this section records the
        # resulting ACT action index where that mapping disagrees with the
        # roBrowser row. An override always wins and must target a skill that
        # some fact table already lists, so a typo cannot add a stray row.
        for entry in extra.get("overrides", []):
            skill_id = int(entry["skill"])
            action = entry["action"]
            validate_action(skill_id, action, "override")
            if skill_id not in actions:
                raise SystemExit(f"skill {skill_id}: override for a skill no fact table lists")
            actions[skill_id] = action

    # Official decompilation table (highest priority): the 2026 client's
    # `CSession::GetSkillActionInfo` motion for every learnable skill, mapped to
    # the ACT action our entity layer plays. This supersedes the roBrowser dump
    # wherever the two disagree (the dump plays the attack animation for ~125
    # skills; the official client plays the cast gesture for most of them).
    if rodecomp_path is not None and rodecomp_path.is_file():
        rodecomp = json.loads(rodecomp_path.read_text())
        for entry in rodecomp.get("overrides", []):
            skill_id = int(entry["skill"])
            action = entry["action"]
            validate_action(skill_id, action, "rodecomp")
            actions[skill_id] = action

    return sorted(actions.items())


def render(rows: list[tuple[int, str]], provenance: dict) -> str:
    lines = [
        "// Official skill -> caster action mapping for the client.",
        "//",
        "// Generated by `tools/gen_skill_actions.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_actions.py`",
        "// Drift check:      `./tools/gen_skill_actions.py --check`",
        "//",
        "// Facts extracted from the official client's skill action assignment",
        "// (`tools/windows_client_skill_actions.json`, roBrowserLegacy RE dump of the",
        "// Windows client tables). Only numeric ids and action names are extracted.",
        "//",
        "// Rows corrected by the curated `overrides` section of",
        "// `tools/skill_actions_extra.json`: the official client's",
        "// `CSession::GetSkillActionInfo` motion is an ACTORSTATEID, and",
        "// `CGameActor::SetState` maps it to the ACT action index recorded there.",
        "//",
        "// Format: (skill id, action). Actions: Attack, Idle, Pickup, Skill,",
        "// ReadyFight (see `CasterAction` in `korangar/src/world/skill_action.rs`).",
        "// Skills that are not listed use `Skill`, the client's default casting",
        "// gesture.",
        "//",
        "// The file is loaded at runtime through the normal client file loader, so a",
        "// server can override it without rebuilding the client; the compiled Rust",
        "// table is only the fallback for missing files.",
        "(",
        f'    provenance: "{provenance["source"]}",',
        "    default_action: Skill,",
        "    actions: [",
    ]
    for skill_id, action in rows:
        lines.append(f"        (skill: {skill_id}, action: {action}),")
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--extra", default=str(DEFAULT_EXTRA))
    parser.add_argument("--rodecomp", default=str(DEFAULT_RODECOMP))
    parser.add_argument("--grf2019", default=str(DEFAULT_GRF2019), help="begin-motion facts of the 2019 GRF tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    payload = json.loads(Path(args.facts).read_text())
    rows = load_facts(Path(args.facts), Path(args.extra), Path(args.rodecomp))

    # The 2019 GRF tables carry the begin motion of the skills they cover. They
    # only fill skills no other fact table lists (the curated/exe rows win).
    grf2019_path = Path(args.grf2019)
    if grf2019_path.is_file():
        known = dict(rows)
        for skill_id, entry in json.loads(grf2019_path.read_text())["skills"].items():
            skill = int(skill_id)
            motion = entry.get("begin_motion")
            if motion is None or skill in known:
                continue
            action = GRF2019_MOTION_ACTION.get(int(motion), "Skill")
            validate_action(skill, action, "grf2019")
            known[skill] = action
        rows = sorted(known.items())

    if not rows:
        raise SystemExit("no skill action facts found")

    content = render(rows, payload["provenance"])
    output = Path(args.output)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_actions.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} entries)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    summary = {action: sum(1 for _id, value in rows if value == action) for action in ACTIONS}
    print(f"{output}: {len(rows)} entries {summary}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
