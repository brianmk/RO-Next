#!/usr/bin/env python3
"""Extract the official skill -> effect mapping facts from the Windows client tables.

Background: the official Windows client (`CondaRO.exe`) compiles its per-skill
effect mapping into code; its effect **resources** live in the GRFs. The
community reimplementation `roBrowserLegacy` (GPL-3.0) documents those mappings
in `src/DB/Skills/SkillEffect.js`, `src/DB/Effects/EffectTable.js` and
`src/DB/Skills/SkillConst.js`. This script only extracts **facts** (numeric
skill ids, effect ids and effect file names) into a plain JSON file; none of the
GPL source is copied into this repository.

The extracted JSON (`tools/windows_client_skill_effects.json`) is the input of
`gen_skill_effect_assets.py`, which validates every referenced effect file
against the CondaRO GRF archives and the `CondaRO.exe` effect whitelist.

Usage (needs network access to raw.githubusercontent.com):

    ./tools/extract_windows_client_skill_effects.py --source-dir /tmp/robrowserlegacy

or let the script download the three tables itself:

    ./tools/extract_windows_client_skill_effects.py

Only read-only HTTP requests are made; the script writes a single JSON file.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/windows_client_skill_effects.json")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"

FILES = {
    "skill_const": f"{BASE_URL}/Skills/SkillConst.js",
    "skill_effect": f"{BASE_URL}/Skills/SkillEffect.js",
    "effect_table": f"{BASE_URL}/Effects/EffectTable.js",
}

# SkillEffect fields we understand, grouped by the phase they belong to.
FIELD_GROUPS = {
    "ground": ("groundEffectId", "groundEffectIdByLevel"),
    "cast": ("beginCastEffectId", "beginCastEffectIdByLevel"),
    "success": (
        "effectId",
        "effectIdOnCaster",
        "successEffectId",
        "successEffectIdOnCaster",
        "successEffectIdOnCasterByLevel",
        "successEffectIdOnTargetByLevel",
        "successEffectIdOnCasterByScreenDirection",
    ),
    "release": ("releaseEffectId",),
    "hit": ("hitEffectId", "beforeHitEffectId", "damageEffectIdOnTarget"),
    "caster": ("damageEffectIdOnCaster",),
}


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read().decode("utf-8")


def read_sources(source_dir: str | None) -> dict[str, str]:
    sources: dict[str, str] = {}
    for key, url in FILES.items():
        if source_dir is not None:
            name = url.rsplit("/", 1)[-1]
            sources[key] = Path(source_dir, name).read_text()
        else:
            sources[key] = fetch(url)
    return sources


def parse_skill_const(text: str) -> dict[str, int]:
    return {match.group(1): int(match.group(2)) for match in re.finditer(r"([A-Z0-9_]+)\s*:\s*(\d+)", text)}


def numeric_ids(value: str) -> list[int]:
    """Return the numeric effect ids of a JS field value (numbers or arrays)."""
    ids = [int(number) for number in re.findall(r"\b(\d{1,4})\b", value)]
    # Field values like `effectId: 0` mean the standard hit flash, not a file.
    return [effect_id for effect_id in ids if effect_id != 0]


def parse_skill_effects(text: str, skill_ids: dict[str, int]) -> dict[int, dict]:
    """Return skill id -> {field: [effect ids]} for all known fields."""
    result: dict[int, dict] = {}
    # Entries are one-liners (`SkillEffect[SK.X] = { ... };`) or multi-line blocks.
    for match in re.finditer(r"SkillEffect\[SK\.([A-Z0-9_]+)\]\s*=\s*\{(.*?)\};", text, re.S):
        symbol = match.group(1)
        body = match.group(2)
        skill_id = skill_ids.get(symbol)
        if skill_id is None:
            continue

        fields: dict[str, list[int]] = {}
        for group, names in FIELD_GROUPS.items():
            ids: list[int] = []
            for field in names:
                field_match = re.search(rf"\b{field}\s*:\s*(\[[^\]]*\]|\d+|'[^']*')", body)
                if field_match is not None:
                    ids.extend(numeric_ids(field_match.group(1)))
            if ids:
                fields[group] = sorted(set(ids))

        if fields:
            result[skill_id] = fields
    return result


def parse_effect_table(text: str) -> dict[int, list[dict]]:
    """Return effect id -> [{type, file, min}] for the effect table entries."""
    result: dict[int, list[dict]] = {}
    entries = list(re.finditer(r"\n\t(\d+): \[", text))
    for index, match in enumerate(entries):
        effect_id = int(match.group(1))
        start = match.end()
        end = entries[index + 1].start() if index + 1 < len(entries) else len(text)
        block = text[start:end]
        files: list[dict] = []
        for file_match in re.finditer(r"file:\s*'([^']+)'", block):
            files.append({"file": file_match.group(1)})
        # The type belongs to the whole entry; attach it to every file in it.
        type_match = re.search(r"type:\s*'([A-Za-z]+)'", block)
        entry_type = type_match.group(1) if type_match else None
        for entry in files:
            entry["type"] = entry_type
        if files:
            result[effect_id] = files
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", default=None, help="local checkout directory of the three JS tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    sources = read_sources(args.source_dir)
    skill_ids = parse_skill_const(sources["skill_const"])
    skills = parse_skill_effects(sources["skill_effect"], skill_ids)
    effects = parse_effect_table(sources["effect_table"])

    payload = {
        "provenance": {
            "source": "roBrowserLegacy (GPL-3.0) RE dump of the official client tables; only numeric ids and file names are extracted",
            "url": FILES["skill_effect"],
            "packetver_note": "extracted facts; validated against CondaRO.exe + GRFs by gen_skill_effect_assets.py",
        },
        "skills": {str(skill_id): fields for skill_id, fields in sorted(skills.items())},
        "effects": {str(effect_id): files for effect_id, files in sorted(effects.items())},
    }

    output = Path(args.output)
    output.write_text(json.dumps(payload, indent=1, ensure_ascii=False) + "\n")
    print(f"{output}: {len(skills)} skills, {len(effects)} effect table entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
