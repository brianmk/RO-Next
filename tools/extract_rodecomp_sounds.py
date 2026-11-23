#!/usr/bin/env python3
"""Extract the official client's per-skill cast sounds from the decompilation.

The official client plays a skill's cast sound with an explicit
`g_soundMgr->PlayWave("...")` inside the skill effect switches. This tool reads
those calls from a `rodecomp` checkout and writes (or checks)
`tools/skill_sounds_rodecomp.ron`, the input of `tools/gen_skill_sounds.py`.

Sources:
  * `Source/Contents/Skills/SkillId.h`            - skill name -> numeric id
  * `Source/Client/Mode/GameModePackets.cpp`      - CGameMode::ReceiveSkillUse
  * `Source/Contents/Jobs/JobInheritance.cpp`     - CSession::PlaySkillEffect

Usage:
    ./tools/extract_rodecomp_sounds.py --rodecomp /path/to/rodecomp
    ./tools/extract_rodecomp_sounds.py --rodecomp /path/to/rodecomp --check
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = REPO / "tools" / "skill_sounds_rodecomp.ron"

SWITCHES = (
    ("Source/Client/Mode/GameModePackets.cpp", "void CGameMode::ReceiveSkillUse"),
    ("Source/Contents/Jobs/JobInheritance.cpp", "void CSession::PlaySkillEffect"),
)

PLAY_WAVE = re.compile(r'PlayWave\(\s*"([^"]+\.wav)"')
CASE = re.compile(r"^\s*case\s+([A-Za-z_][A-Za-z0-9_]*)\s*:")
SKILL_ENUM = re.compile(r"^\s*([A-Z][A-Z0-9_]+)\s*=\s*(\d+)\s*,", re.M)


def read_skill_ids(rodecomp: Path) -> dict[str, int]:
    header = (rodecomp / "Source/Contents/Skills/SkillId.h").read_text(encoding="utf-8", errors="replace")
    ids = {name: int(value) for name, value in SKILL_ENUM.findall(header)}
    if not ids:
        raise SystemExit("no skill ids found in SkillId.h")
    return ids


def function_body(text: str, signature: str) -> str:
    start = text.index(signature)
    brace = text.index("{", start)
    depth = 0
    for index in range(brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[brace:index + 1]
    raise SystemExit(f"unbalanced braces after {signature}")


SWITCH_SKILL = re.compile(r"switch\s*\(\s*skillId\s*\)")


def switch_blocks(body: str) -> list[str]:
    """Bodies of every `switch (skillId) { ... }` in `body`."""
    blocks = []
    for match in SWITCH_SKILL.finditer(body):
        brace = body.find("{", match.end())
        if brace < 0:
            continue
        depth = 0
        for index in range(brace, len(body)):
            if body[index] == "{":
                depth += 1
            elif body[index] == "}":
                depth -= 1
                if depth == 0:
                    blocks.append(body[brace + 1:index])
                    break
    return blocks


def play_waves(body: str) -> dict[str, list[str]]:
    """case label -> ordered PlayWave files of its body (fallthrough aware)."""
    sounds: dict[str, list[str]] = {}
    for block in switch_blocks(body):
        sounds.update(play_waves_in_switch(block))
    return sounds


def play_waves_in_switch(body: str) -> dict[str, list[str]]:
    """case label -> ordered PlayWave files of its body (fallthrough aware)."""
    sounds: dict[str, list[str]] = {}
    pending: list[str] = []
    current: list[str] = []
    seen_statement = False
    depth = 0

    def flush() -> None:
        for name in pending:
            sounds.setdefault(name, []).extend(current)

    for line in body.splitlines():
        depth += line.count("{") - line.count("}")
        label = CASE.match(line)
        default = re.match(r"^\s*default\s*:", line)
        if label or default:
            if seen_statement:
                flush()
                pending, current = [], []
            pending = pending + ([label.group(1)] if label else [])
            seen_statement = False
            continue
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        seen_statement = True
        wave = PLAY_WAVE.search(line)
        if wave and pending:
            # The decompiled source spells the separator escaped (`effect\\x.wav`).
            current.append(wave.group(1).replace("\\\\", "\\"))
    flush()
    return {name: files for name, files in sounds.items() if files}


def learnable_skills() -> set[int]:
    document = json.loads((REPO / "tools" / "rathena_pre_re_learnable_skills.json").read_text(encoding="utf-8"))
    return {int(skill) for skill in document["skills"]}


def collect(rodecomp: Path) -> dict[int, list[str]]:
    ids = read_skill_ids(rodecomp)
    learnable = learnable_skills()
    collected: dict[int, list[str]] = {}
    for relative, signature in SWITCHES:
        text = (rodecomp / relative).read_text(encoding="utf-8", errors="replace")
        for name, files in play_waves(function_body(text, signature)).items():
            skill = ids.get(name)
            if skill is None or skill not in learnable:
                continue
            bucket = collected.setdefault(skill, [])
            for file in files:
                if file not in bucket:
                    bucket.append(file)
    return collected


def render(collected: dict[int, list[str]]) -> str:
    lines = [
        "// Skill cast sounds played by the official client (rodecomp).",
        "//",
        "// Generated by `tools/extract_rodecomp_sounds.py`; do not edit by hand.",
        "// Regenerate with: `./tools/extract_rodecomp_sounds.py --rodecomp /path/to/rodecomp`",
        "// Drift check:      `./tools/extract_rodecomp_sounds.py --rodecomp ... --check`",
        "//",
        "// These are the sounds the official C++ skill switches play with",
        "// `g_soundMgr->PlayWave(...)` in CGameMode::ReceiveSkillUse and",
        "// CSession::PlaySkillEffect, so they supersede the roBrowser effect-table",
        "// entries (community dump) where both exist. Every path is validated against",
        "// the CondaRO GRF archives by `tools/gen_skill_sounds.py`.",
        "(",
        '    provenance: "rodecomp (matching decompilation of the official client) PlayWave calls in the skill effect switches",',
        "    sounds: [",
    ]
    for skill in sorted(collected):
        rendered = ", ".join('"' + file.replace("\\", "\\\\") + '"' for file in collected[skill])
        lines.append(f"        (skill: {skill}, files: [{rendered}]),")
    lines += ["    ],", ")", ""]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--rodecomp", required=True, help="path to a rodecomp checkout")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed facts are out of date")
    args = parser.parse_args()

    rodecomp = Path(args.rodecomp)
    if not (rodecomp / "Source/Contents/Skills/SkillId.h").is_file():
        raise SystemExit(f"not a rodecomp checkout: {rodecomp}")

    content = render(collect(rodecomp))
    output = Path(args.output)
    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"{output} is out of date (run ./tools/extract_rodecomp_sounds.py)", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({content.count('(skill:')} skills)")
        return 0
    output.write_text(content, encoding="utf-8")
    print(f"wrote {output} ({content.count('(skill:')} skills)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
