#!/usr/bin/env python3
"""Extract the per-skill caster frame sequences of the decompiled client.

The original client plays a specific ACT block / frame range for many skills
(`CRenderObject::SetFrameSequence` in `RenderObject.cpp`). The skill data files
of that client do not carry this: it is hard-coded C++.

Some skill-use animations live in `CGameMode::ReceiveSkillUse`
(`GameModePackets.cpp`) instead, next to the hard-coded effect launches of the
skill. Only the skill-use sequences the port currently models are extracted
(`SKILL_USE_SEQUENCES` below); add a skill name together with its client-side
animation so this fact file stays reproducible.

Input (read-only): /home/phantom/Documents/deepseek-harness/default-workspace/rodecomp
Output: tools/data/rodecomp_skill_frame_sequences.json

Usage: ./tools/extract_rodecomp_frame_sequences.py
"""

from __future__ import annotations

import json
import re
from pathlib import Path

RODECOMP = Path("/home/phantom/Documents/deepseek-harness/default-workspace/rodecomp")
RENDER_OBJECT = RODECOMP / "Source/Contents/Actors/RenderObject.cpp"
GAME_MODE_PACKETS = RODECOMP / "Source/Client/Mode/GameModePackets.cpp"
SKILL_ID_HEADER = RODECOMP / "Source/Contents/Skills/SkillId.h"
SKID_DUMP = Path("tools/data/client_skill_tables_2019.txt")
OUTPUT = Path("tools/data/rodecomp_skill_frame_sequences.json")

SWITCH_START = 5740
SWITCH_END = 5940

#: Skill-use animations (`CGameMode::ReceiveSkillUse`) the port models, with
#: the source line of each `SetFrameSequence` call. The rest of that switch
#: is not wired yet; keeping the list explicit means a rerun of this script
#: never adds unreviewed animations.
SKILL_USE_SEQUENCES = {
    # GameModePackets.cpp:18829-18848
    "MO_BLADESTOP",
    "MO_EXPLOSIONSPIRITS",
    "MO_STEELBODY",
    # :18886 (ready-fight block 0x20, not the skill block)
    "LK_AURABLADE",
    # :18923-18932
    "DC_SCREAM",
    "BA_FROSTJOKE",
    # :19050 (attack block 0x58)
    "HW_SOULDRAIN",
    # :19561
    "BA_PANGVOICE",
    # :20170
    "SR_CURSEDCIRCLE",
    # :20805
    "AG_DEADLY_PROJECTION",
    "AG_SOUL_VC_STRIKE",
    # :20826-20868
    "IQ_POWERFUL_FAITH",
    "IQ_FIRM_FAITH",
    "IQ_SINCERE_FAITH",
    "IQ_JUDGE",
    "IQ_THIRD_EXOR_FLAME",
    # :20890-20897
    "IQ_EXPOSION_BLASTER",
    "IQ_MASSIVE_F_BLASTER",
    # :20977
    "AG_FROZEN_SLASH",
    # :21142-21156
    "TR_MUSICAL_INTERLUDE",
    "TR_NIPELHEIM_REQUIEM",
    "AG_RAIN_OF_CRYSTAL",
    "AG_ASTRAL_STRIKE",
    "TR_RETROSPECTION",
    # :21231
    "ABC_FROM_THE_ABYSS",
}


def extract_render_object(names: dict[str, int]) -> tuple[dict[str, dict], dict[str, dict]]:
    lines = RENDER_OBJECT.read_text().splitlines()
    sequences: dict[str, dict] = {}
    pending: list[str] = []
    interval: int | None = None

    case_re = re.compile(r"case ([A-Z0-9_]+):")
    seq_re = re.compile(r"SetFrameSequence\(([^)]*)\)")
    int_re = re.compile(r"SetSequenceInterval\((\d+)\)")

    for number in range(SWITCH_START, SWITCH_END):
        line = lines[number - 1].strip()

        case = case_re.search(line)
        if case:
            pending.append(case.group(1))
            if line.endswith("{"):
                pass
            continue

        interval_match = int_re.search(line)
        if interval_match:
            interval = int(interval_match.group(1))
            continue

        seq = seq_re.search(line)
        if seq:
            args = [argument.strip() for argument in seq.group(1).split(",")]
            # (block, first, last, block2, loopTo, loops, duration, flag)
            entry = {
                "block": args[0],
                "first": args[1],
                "last": args[2],
                "loop_block": args[3],
                "loop_to": args[4],
                "loops": args[5],
                "duration": args[6],
                "interval": interval,
                "source": f"RenderObject.cpp:{number}",
            }
            for name in pending:
                if name in names:
                    sequences[name] = dict(entry, skill=names[name])
                else:
                    sequences[name] = dict(entry, skill=None)
            pending = []
            interval = None
            continue

        if line.startswith("break;") or line == "}" or line.startswith("}"):
            pending = []
            interval = None

    unknown = {name: entry for name, entry in sequences.items() if entry["skill"] is None}
    return sequences, unknown


def extract_skill_use(names: dict[str, int], wanted: set[str]) -> dict[str, dict]:
    """`SetFrameSequence` calls inside `CGameMode::ReceiveSkillUse`."""
    lines = GAME_MODE_PACKETS.read_text().splitlines()
    start = next((number for number, line in enumerate(lines, 1) if "void CGameMode::ReceiveSkillUse(" in line), None)
    if start is None:
        raise SystemExit("CGameMode::ReceiveSkillUse not found")

    end = next((number for number in range(start + 1, len(lines) + 1) if lines[number - 1].startswith("// OFFSET:")), len(lines) + 1)

    pending: list[str] = []
    interval: int | None = None
    sequences: dict[str, dict] = {}

    case_re = re.compile(r"case ([A-Z0-9_]+):")
    seq_re = re.compile(r"SetFrameSequence\(([^)]*)\)")
    int_re = re.compile(r"SetSequenceInterval\((\d+)\)")

    for number in range(start, end):
        line = lines[number - 1].strip()

        case = case_re.search(line)
        if case:
            pending.append(case.group(1))
            continue

        interval_match = int_re.search(line)
        if interval_match:
            interval = int(interval_match.group(1))
            continue

        seq = seq_re.search(line)
        if seq:
            args = [argument.strip() for argument in seq.group(1).split(",")]
            entry = {
                "block": args[0],
                "first": args[1],
                "last": args[2],
                "loop_block": args[3],
                "loop_to": args[4],
                "loops": args[5],
                "duration": args[6],
                "interval": interval,
                "source": f"GameModePackets.cpp:{number}",
            }
            for name in pending:
                if name in wanted and name in names:
                    sequences[name] = dict(entry, skill=names[name])
            pending = []
            interval = None
            continue

        if line.startswith("break;") or line == "}" or line.startswith("}"):
            pending = []
            interval = None

    return sequences


def load_skill_names() -> dict[str, int]:
    """Skill name -> id, preferring the 2019 client dump and filling newer jobs
    from the decompiled `SkillId.h` enum (the GRF dump predates 4th jobs)."""
    names = {
        name: int(skill)
        for skill, name in (line.split("|")[1:3] for line in SKID_DUMP.read_text().splitlines() if line.startswith("skillid|"))
    }
    if SKILL_ID_HEADER.is_file():
        for name, skill in re.findall(r"\b([A-Z][A-Z0-9_]+)\s*=\s*(\d+)\s*,", SKILL_ID_HEADER.read_text()):
            names.setdefault(name, int(skill))
    return names


def main() -> None:
    names = load_skill_names()

    sequences, unknown = extract_render_object(names)
    # RenderObject's damage sequences win when a skill has both kinds.
    for name, entry in extract_skill_use(names, SKILL_USE_SEQUENCES).items():
        sequences.setdefault(name, entry)

    data = {
        "provenance": {
            "source": "github.com/rodecomp/rodecomp - CRenderObject skill damage switch and CGameMode::ReceiveSkillUse SetFrameSequence calls",
            "file": "Source/Contents/Actors/RenderObject.cpp, Source/Client/Mode/GameModePackets.cpp",
            "note": "block 0x58 = attack block (88), 0x60 = skill block (96), 0x20 = ready-fight block (32); duration is milliseconds or a runtime skill-animation value; interval is the preceding SetSequenceInterval in 60 Hz ticks; only the reviewed skill-use sequences in extract_rodecomp_frame_sequences.py are extracted",
        },
        "sequences": {name: entry for name, entry in sorted(sequences.items()) if entry["skill"] is not None},
        "unmatched": unknown,
    }
    OUTPUT.write_text(json.dumps(data, indent=1) + "\n")
    print(f"wrote {OUTPUT}: {len(data['sequences'])} skills, {len(unknown)} unmatched labels")


if __name__ == "__main__":
    main()
