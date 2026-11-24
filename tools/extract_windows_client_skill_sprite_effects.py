#!/usr/bin/env python3
"""Extract the official sprite-effect (`.spr`/`.act`) facts from the Windows client tables.

Background: the official Windows client (`CondaRO.exe`) compiles its per-skill
effect mapping and the effect parameters into code. Sprite effects are played
from `data\\sprite\\이팩트\\...` (`.spr` + `.act` pairs). The community
reimplementation `roBrowserLegacy` (GPL-3.0) documents the same effect table in
`src/DB/Effects/EffectTable.js`; the skill -> effect id mapping is already
extracted into `tools/data/windows_client_skill_effects.json`.

This script extracts only the **facts** for `type: 'SPR'` effect-table entries
(ids, file names, flags and numeric playback parameters) and writes them to
`tools/data/windows_client_sprite_effects.json`. No GPL source is copied into this
repository; the JSON only contains numeric/boolean/name facts that
`tools/gen_skill_sprite_effects.py` validates against the real GRF archives
(the `.spr` **and** the `.act` file must exist).

The JavaScript comments are stripped before parsing: several entries contain a
commented-out `type: 'SPR'` line (for example `EF_SMOKE`) that is *not* a
sprite effect, and the old (comment-blind) extractor used to mis-classify them.

Usage (needs network access to raw.githubusercontent.com):

    ./tools/extract_windows_client_skill_sprite_effects.py --source-dir /tmp/robrowserlegacy

or let the script download the table itself:

    ./tools/extract_windows_client_skill_sprite_effects.py

Only read-only HTTP requests are made; the script writes a single JSON file.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/data/windows_client_sprite_effects.json")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"
EFFECT_TABLE_URL = f"{BASE_URL}/Effects/EffectTable.js"

# Numeric playback parameters of a sprite effect entry that we keep as facts.
NUMERIC_KEYS = ("duration", "delayFrame", "delayOffset", "frame", "yOffset", "alphaMax", "zIndex")
# Boolean entry flags that change where/how the sprite is played.
BOOLEAN_KEYS = ("head", "attachedEntity", "stopAtEnd", "direction", "repeat", "stackable")


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read().decode("utf-8")


def strip_comments(text: str) -> str:
    """Remove JS line and block comments (string literals are preserved)."""
    # Remove /* ... */ first; the effect-table blocks never contain comment
    # markers inside string literals.
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    # Remove // ... to the end of the line, but not the "//" inside a string
    # (effect file names use single slashes only, so this is safe here).
    text = re.sub(r"//[^\n]*", "", text)
    return text


def parse_effect_table(text: str) -> dict[int, list[dict]]:
    """Return effect id -> list of active `type: 'SPR'` entry objects (facts only)."""
    text = strip_comments(text)
    entries = list(re.finditer(r"\n\t(\d+): \[", text))
    result: dict[int, list[dict]] = {}

    for index, match in enumerate(entries):
        effect_id = int(match.group(1))
        start = match.end()
        end = entries[index + 1].start() if index + 1 < len(entries) else len(text)
        block = text[start:end]

        objects: list[dict] = []
        for object_match in re.finditer(r"\{([^{}]*)\}", block):
            body = object_match.group(1)

            type_match = re.search(r"\btype:\s*'([^']+)'", body)
            if type_match is None or type_match.group(1) != "SPR":
                continue

            file_match = re.search(r"\bfile:\s*'((?:[^'\\]|\\.)*)'", body)
            if file_match is None:
                continue

            entry: dict = {"file": file_match.group(1)}

            for key in NUMERIC_KEYS:
                number = re.search(rf"\b{key}:\s*(-?\d+(?:\.\d+)?)", body)
                if number is not None:
                    value = float(number.group(1))
                    entry[key] = int(value) if value.is_integer() else value

            for key in BOOLEAN_KEYS:
                flag = re.search(rf"\b{key}:\s*(true|false)", body)
                if flag is not None:
                    entry[key] = flag.group(1) == "true"

            objects.append(entry)

        if objects:
            result[effect_id] = objects

    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", default=None, help="local checkout directory of the EffectTable.js table")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    if args.source_dir is not None:
        text = Path(args.source_dir, "EffectTable.js").read_text()
    else:
        text = fetch(EFFECT_TABLE_URL)

    effects = parse_effect_table(text)
    if not effects:
        raise SystemExit("no SPR effect entries found in the effect table")

    payload = {
        "provenance": {
            "source": "roBrowserLegacy (GPL-3.0) RE dump of the official client EffectTable.js; "
            "only effect ids, file names and boolean/numeric playback parameters are extracted",
            "url": EFFECT_TABLE_URL,
            "note": "js comments stripped before parsing; validated against the CondaRO GRFs by gen_skill_sprite_effects.py",
        },
        "effects": {str(effect_id): objects for effect_id, objects in sorted(effects.items())},
    }

    output = Path(args.output)
    output.write_text(json.dumps(payload, indent=1, ensure_ascii=False) + "\n")
    print(f"{output}: {len(effects)} SPR effect entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
