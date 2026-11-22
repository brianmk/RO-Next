#!/usr/bin/env python3
"""Extract **all** official effect-table layer facts from the Windows client tables.

Background: the official Windows client (`CondaRO.exe`) compiles its per-skill
effect mapping and the per-effect layer list into code. The effect **resources**
live in the GRFs. The community reimplementation `roBrowserLegacy` (GPL-3.0)
documents the same table in `src/DB/Effects/EffectTable.js`; the existing
extractor `extract_windows_client_skill_sprite_effects.py` only keeps the
`type: 'SPR'` layers and `extract_windows_client_skill_effects.py` only keeps
`effect id -> (file, type)` (losing `head`, `attachedEntity`, `duration`,
`delay`, `wav`, `min` and per-layer types).

This script extracts the **complete, ordered layer list** of every numeric
effect id as plain facts:

* `type` (`STR`, `SPR`, `3D`, `2D`, `CYLINDER`, `FUNC`, ...),
* `file` / `fileList` / `spriteName` / `absoluteSpriteName` / `textureName`
  (with JS string escapes decoded, Korean file names included),
* `wav` (official sound of the layer),
* `attachedEntity` (entity-attached vs ground/world anchored),
* `head` (overhead anchor),
* `duration`, `delayOffset`, `delayFrame`, `delayLate`, `frame`, `yOffset`,
  `alphaMax`, `size*`, `duplicate`, `pos*`, `blendMode`, `red`/`green`/`blue`,
  `repeat`/`stackable`/`direction` and every other scalar playback parameter,
* the official layer order (for example Blessing effect 42 has four layers: the
  `축복` sprite, two `particle6` bursts and `effect/pok2.tga`; Increase AGI
  effect 37 has four: three `effect/ac_center2.tga` bursts and
  `effect/agi_up.bmp`).

Only numeric/boolean/name facts are extracted into JSON; no GPL source is
copied into this repository. The JSON is the input of
`tools/gen_skill_effect_layers.py`, which validates every referenced file
against the CondaRO GRF archives and emits the official client data fragment.

Usage (needs network access to raw.githubusercontent.com unless
`--source-dir`/`--input` is given):

    ./tools/extract_windows_client_effect_layers.py
    ./tools/extract_windows_client_effect_layers.py --source-dir /tmp/robrowserlegacy
    ./tools/extract_windows_client_effect_layers.py --input /tmp/EffectTable.js

Only read-only HTTP requests are made; the script writes a single JSON file.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/windows_client_effect_layers.json")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"
EFFECT_TABLE_URL = f"{BASE_URL}/Effects/EffectTable.js"

#: Block headers look like `\n\t42: [` (numeric effect ids) or
#: `\n\tef_arrow_projectile: [` (named effect/sound blocks that
#: `SkillEffect.js` references symbolically).
BLOCK_START = re.compile(r"\n\t([A-Za-z_$][\w$]*|\d+): \[")

#: Scalar values of an object entry: single quoted strings, booleans, numbers.
STRING_VALUE = re.compile(r"'((?:[^'\\]|\\.)*)'")
NUMBER_VALUE = re.compile(r"-?\d+(?:\.\d+)?")


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read().decode("utf-8")


def decode_js_string(raw: str) -> str:
    """Decode a JS string literal value (`\\`, `\\xNN`, `\\uNNNN` escapes).

    roBrowserLegacy stores EUC-KR file-name bytes as individual `\\xNN`
    characters in the JS source; the official client loader reads them as
    EUC-KR, so the fact is the decoded file name.
    """
    if "\\" not in raw:
        return raw
    data = bytearray()
    index = 0
    while index < len(raw):
        character = raw[index]
        if character != "\\" or index + 1 >= len(raw):
            data.extend(character.encode("utf-8"))
            index += 1
            continue
        escape = raw[index + 1]
        if escape == "x" and index + 3 < len(raw):
            data.append(int(raw[index + 2 : index + 4], 16))
            index += 4
        elif escape == "u" and index + 5 < len(raw):
            data.extend(chr(int(raw[index + 2 : index + 6], 16)).encode("utf-8"))
            index += 6
        elif escape == "n":
            data.append(0x0A)
            index += 2
        elif escape == "t":
            data.append(0x09)
            index += 2
        else:
            data.extend(escape.encode("utf-8"))
            index += 2
    try:
        return data.decode("euc-kr")
    except UnicodeDecodeError:
        return data.decode("latin-1")


def decode_value(raw: str):
    raw = raw.strip()
    if raw.startswith("'") and raw.endswith("'"):
        return decode_js_string(raw[1:-1])
    if raw in ("true", "false"):
        return raw == "true"
    if NUMBER_VALUE.fullmatch(raw):
        number = float(raw)
        return int(number) if number.is_integer() else number
    return None


def skip_comment(text: str, index: int) -> int:
    """Return the index after a JS comment starting at `text[index]` (or index)."""
    if text.startswith("//", index):
        end = text.find("\n", index)
        return len(text) if end < 0 else end + 1
    if text.startswith("/*", index):
        end = text.find("*/", index + 2)
        return len(text) if end < 0 else end + 2
    return index


def split_top_level_values(value: str) -> list[str]:
    """Split an array literal body into top-level element literals."""
    parts: list[str] = []
    depth = 0
    start = 0
    index = 0
    while index < len(value):
        character = value[index]
        if character in "'\"`":
            quote = character
            index += 1
            while index < len(value):
                if value[index] == "\\":
                    index += 2
                    continue
                if value[index] == quote:
                    break
                index += 1
        elif character in "([{":
            depth += 1
        elif character in ")]}":
            depth -= 1
        elif character == "," and depth == 0:
            parts.append(value[start:index])
            start = index + 1
        index += 1
    if value[start:].strip():
        parts.append(value[start:])
    return parts


def parse_object(content: str) -> dict:
    """Parse the top-level scalar/array keys of a single effect-table object."""
    layer: dict = {}
    index = 0
    length = len(content)
    while index < length:
        if content.startswith("//", index) or content.startswith("/*", index):
            index = skip_comment(content, index)
            continue
        key_match = re.compile(r"\s*([A-Za-z_$][\w$]*)\s*:\s*").match(content, index)
        if key_match is None:
            index += 1
            continue
        key = key_match.group(1)
        index = key_match.end()
        if index >= length:
            break
        character = content[index]
        if character in "'\"`":
            quote = character
            end = index + 1
            while end < length:
                if content[end] == "\\":
                    end += 2
                    continue
                if content[end] == quote:
                    break
                end += 1
            layer[key] = decode_js_string(content[index + 1 : end])
            index = end + 1
        elif character == "[":
            depth = 0
            end = index
            while end < length:
                if content[end] in "([{":
                    depth += 1
                elif content[end] in ")]}":
                    depth -= 1
                    if depth == 0:
                        break
                end += 1
            elements = [decode_value(part) for part in split_top_level_values(content[index + 1 : end])]
            values = [element for element in elements if element is not None]
            if values:
                layer[key] = values
            index = end + 1
        elif character == "{":
            # Nested object: skip it (not part of the flat playback facts).
            depth = 0
            end = index
            while end < length:
                if content[end] == "{":
                    depth += 1
                elif content[end] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                end += 1
            index = end + 1
        else:
            token = re.match(r"[^,\n}]+", content[index:])
            raw = token.group(0).strip() if token is not None else ""
            value = decode_value(raw)
            if value is not None:
                layer[key] = value
            index += len(raw) if raw else 1
    return layer


def parse_effect_blocks(text: str) -> tuple[dict[int, dict], dict[str, dict]]:
    """Return (numeric effect id -> entry, named block -> entry).

    An entry is `{"name": official comment, "layers": [facts...]}`. Numeric ids
    are the official `SkillEffect` effects; named blocks (`ef_*`, `*_ground`,
    `*_sound`, ...) are referenced symbolically by newer `SkillEffect` rows and
    contain the same layer facts.
    """
    result: dict[int, dict] = {}
    named: dict[str, dict] = {}
    starts = list(BLOCK_START.finditer(text))
    for position, match in enumerate(starts):
        raw_key = match.group(1)
        effect_id = int(raw_key) if raw_key.isdigit() else None
        end = starts[position + 1].start() if position + 1 < len(starts) else len(text)
        block = text[match.end() : end]

        # The official effect comment names the effect (`//EF_INCAGILITY AGI Up`).
        name = ""
        comment = re.search(r"//\s*([^\n]*)", block)
        if comment is not None:
            name = comment.group(1).strip()

        layers: list[dict] = []
        index = 0
        block_length = len(block)
        while index < block_length:
            if block[index] in " \t\r\n,":
                index += 1
                continue
            if block.startswith("//", index) or block.startswith("/*", index):
                index = skip_comment(block, index)
                continue
            if block[index] == "{":
                depth = 0
                object_end = index
                while object_end < block_length:
                    if block[object_end] == "{":
                        depth += 1
                    elif block[object_end] == "}":
                        depth -= 1
                        if depth == 0:
                            break
                    object_end += 1
                layer = parse_object(block[index + 1 : object_end])
                if layer:
                    layers.append(layer)
                index = object_end + 1
                continue
            # Spread (`...endureEffect`) or inline expression: skip to the next
            # top-level comma.
            next_comma = block.find(",", index)
            if next_comma < 0:
                break
            index = next_comma + 1

        if layers:
            if effect_id is None:
                named[raw_key] = {"name": name, "layers": layers}
            else:
                result[effect_id] = {"name": name, "layers": layers}
    return result, named


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", default=None, help="local checkout directory containing EffectTable.js")
    parser.add_argument("--input", default=None, help="path to an already downloaded EffectTable.js")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed facts file is out of date")
    args = parser.parse_args()

    if args.input is not None:
        text = Path(args.input).read_text(encoding="utf-8", errors="replace")
    elif args.source_dir is not None:
        text = Path(args.source_dir, "EffectTable.js").read_text(encoding="utf-8", errors="replace")
    else:
        text = fetch(EFFECT_TABLE_URL)

    effects, named_effects = parse_effect_blocks(text)
    if not effects:
        raise SystemExit("no effect table entries found")

    payload = {
        "provenance": {
            "source": (
                "roBrowserLegacy (GPL-3.0) RE dump of the official client EffectTable.js; only numeric ids, "
                "field names, file names and playback parameters are extracted"
            ),
            "url": EFFECT_TABLE_URL,
            "note": (
                "js comments stripped before parsing; every layer (STR/SPR/3D/...) and its official playback "
                "facts are kept in table order; validated against the CondaRO GRFs by gen_skill_effect_layers.py"
            ),
        },
        "effects": {str(effect_id): entry for effect_id, entry in sorted(effects.items())},
        "named_effects": {name: entry for name, entry in sorted(named_effects.items())},
    }

    content = json.dumps(payload, indent=1, ensure_ascii=False) + "\n"
    output = Path(args.output)
    layer_count = sum(len(entry["layers"]) for entry in effects.values())

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/extract_windows_client_effect_layers.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(effects)} effects, {len(named_effects)} named blocks, {layer_count} layers)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {len(effects)} effects, {len(named_effects)} named blocks, {layer_count} layers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
