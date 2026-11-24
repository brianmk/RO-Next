#!/usr/bin/env python3
"""Extract the per-skill caster frame sequences of the decompiled client.

The original client plays a specific ACT block / frame range for many skills
(`CRenderObject::SetFrameSequence` in `RenderObject.cpp`). The skill data files
of that client do not carry this: it is hard-coded C++.

Input (read-only): /home/phantom/Documents/deepseek-harness/default-workspace/rodecomp
Output: tools/rodecomp_skill_frame_sequences.json

Usage: ./tools/extract_rodecomp_frame_sequences.py
"""

from __future__ import annotations

import json
import re
from pathlib import Path

RODECOMP = Path("/home/phantom/Documents/deepseek-harness/default-workspace/rodecomp")
RENDER_OBJECT = RODECOMP / "Source/Contents/Actors/RenderObject.cpp"
SKID_DUMP = Path("tools/client_skill_tables_2019.txt")
OUTPUT = Path("tools/rodecomp_skill_frame_sequences.json")

SWITCH_START = 5740
SWITCH_END = 5940


def main() -> None:
    names = {name: int(skill) for skill, name in (line.split("|")[1:3] for line in SKID_DUMP.read_text().splitlines() if line.startswith("skillid|"))}

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
    data = {
        "provenance": {
            "source": "github.com/rodecomp/rodecomp - CRenderObject skill damage switch, SetFrameSequence calls",
            "file": "Source/Contents/Actors/RenderObject.cpp",
            "note": "block 0x58 = attack block (88), 0x60 = skill block (96); duration is milliseconds or a runtime skill-animation value",
        },
        "sequences": {name: entry for name, entry in sorted(sequences.items()) if entry["skill"] is not None},
        "unmatched": unknown,
    }
    OUTPUT.write_text(json.dumps(data, indent=1) + "\n")
    print(f"wrote {OUTPUT}: {len(data['sequences'])} skills, {len(unknown)} unmatched labels")


if __name__ == "__main__":
    main()
