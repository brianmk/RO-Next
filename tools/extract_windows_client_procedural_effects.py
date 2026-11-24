#!/usr/bin/env python3
"""Extract the procedural (hard-coded) skill effect facts from the Windows client tables.

Background: most skill visuals of the official Windows client are **not** data
files. The client builds them from hard-coded effect primitives (`3D`/`2D`
billboards, `CYLINDER` auras, `FUNC` simulations, ...). The community
reimplementation `roBrowserLegacy` (GPL-3.0) documents the same tables in
`src/DB/Skills/SkillEffect.js` (skill id -> effect id per phase) and
`src/DB/Effects/EffectTable.js` (effect id -> primitive components).

Like `extract_windows_client_skill_effects.py`, this script only extracts
**facts** (numeric ids, effect/component types, numeric parameters, texture
names, symbolic effect names and the names of the JS constructors a `FUNC`
component uses). None of the GPL source is copied into this repository.

The extracted JSON (`tools/data/windows_client_procedural_effects.json`) is the input
of `gen_skill_procedural_effects.py`, which turns it into the client data file
`ro-next/archive/data/client/skill_procedural_effects.ron`.

The skill *symbol -> numeric id* mapping is read from the committed audit table
`tools/data/skill_procedural_components.csv` (so `SkillConst.js` is not needed).

Usage (needs the two JS tables; network or a local checkout):

    ./tools/extract_windows_client_procedural_effects.py --source-dir /tmp/robrowserlegacy/src/DB
    ./tools/extract_windows_client_procedural_effects.py            # downloads them

Only read-only HTTP requests are made; the script writes a single JSON file.
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import sys
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/data/windows_client_procedural_effects.json")
DEFAULT_COMPONENTS = Path("tools/data/skill_procedural_components.csv")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"
FILES = {
    "skill_effect": f"{BASE_URL}/Skills/SkillEffect.js",
    "effect_table": f"{BASE_URL}/Effects/EffectTable.js",
}

#: SkillEffect fields we know, mapped to the phase and the anchor they spawn on.
#: `anchor` is resolved later by the generator; here it only documents the field
#: semantics: `cast` fields are on the caster, `target` fields on the target,
#: `travel` fields fly from the caster to the target, `auto` fields play on
#: whoever the packet reports (target when known, caster for self casts).
KNOWN_FIELDS = (
    "groundEffectId",
    "groundEffectIdByLevel",
    "beginCastEffectId",
    "beginCastEffectIdByLevel",
    "castAuraEffectId",
    "effectId",
    "effectIdOnCaster",
    "successEffectId",
    "successEffectIdOnCaster",
    "successEffectIdOnCasterByLevel",
    "successEffectIdOnTargetByLevel",
    "successEffectIdOnCasterByScreenDirection",
    "successEffectIdOnCasterAimed",
    "successEffectIdOnTargetByScreenSide",
    "releaseEffectId",
    "beforeHitEffectId",
    "beforeHitEffectIdOnCaster",
    "hitEffectId",
    "hitEffectIdOnCaster",
    "damageEffectIdOnTarget",
    "damageEffectIdOnCaster",
)

#: Component fields that are extracted as-is (numbers) for the generator.
NUMERIC_COMPONENT_FIELDS = (
    "duration",
    "delayStart",
    "delayLate",
    "delayOffset",
    "duplicate",
    "timeBetweenDupli",
    "size",
    "sizeStart",
    "sizeEnd",
    "sizeX",
    "sizeRand",
    "sizeDelta",
    "sizeRandStartX",
    "sizeRandStartY",
    "sizeRandEndY",
    "alphaMax",
    "alphaMin",
    "blendMode",
    "angle",
    "angleRand",
    "animation",
    "height",
    "bottomSize",
    "topSize",
    "posZ",
    "posz",
    "poszStart",
    "posxStart",
    "posxRand",
    "posyRand",
    "spread",
    "width",
    "frames",
    "radiusSpeed",
    "alphaSpeed",
    "spin",
    "zIndex",
)

STRING_COMPONENT_FIELDS = (
    "type",
    "file",
    "textureName",
    "textureFile",
    "spriteName",
    "absoluteSpriteName",
    "texture",
    "prim",
)

BOOLEAN_COMPONENT_FIELDS = (
    "attachedEntity",
    "fromSrc",
    "toSrc",
    "rotateToTarget",
    "rotateWithCamera",
    "rotate",
    "fade",
    "fadeIn",
    "fadeOut",
    "overlay",
    "playSprite",
    "repeat",
    "sparkling",
)

#: JS constructors/methods a `FUNC` component may use; extracted as `func` tokens
#: so the generator can classify the primitive.
FUNC_TOKEN_PATTERN = re.compile(
    r"\bnew\s+([A-Z][A-Za-z0-9_]*)\s*\(|"
    r"\bCamera\.([A-Za-z]+)\s*\(|"
    r"\bthis\.(spamEffect)\s*\(|"
    r"\b(startCasterStrobe|bragiNoteEffect)\s*\("
)

#: Arguments of `new BoltRain(..., '<element>')` and similar element hints.
ELEMENT_PATTERN = re.compile(r"'(cold|fire|wind|earth|holy|poison|neutral|ice|water)'")

NUMBER_PATTERN = r"-?\d+(?:\.\d+)?"


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read().decode("utf-8")


def read_sources(source_dir: str | None) -> dict[str, str]:
    sources: dict[str, str] = {}
    for key, url in FILES.items():
        if source_dir is not None:
            name = url.rsplit("/", 1)[-1]
            path = Path(source_dir, name)
            if not path.is_file():
                # The audit data directory uses the `roBrowserLegacy_` prefix.
                path = Path(source_dir, f"roBrowserLegacy_{name}")
            sources[key] = path.read_text(encoding="utf-8")
        else:
            sources[key] = fetch(url)
    return sources


def strip_comments(text: str) -> str:
    """Remove `//` comments while preserving string literals (and line numbers)."""
    output: list[str] = []
    index = 0
    in_string: str | None = None

    while index < len(text):
        char = text[index]
        if in_string is not None:
            output.append(char)
            if char == "\\" and index + 1 < len(text):
                output.append(text[index + 1])
                index += 2
                continue
            if char == in_string:
                in_string = None
            index += 1
            continue
        if char in "'\"":
            in_string = char
            output.append(char)
            index += 1
            continue
        if char == "/" and index + 1 < len(text) and text[index + 1] == "/":
            end = text.find("\n", index)
            index = end if end >= 0 else len(text)
            continue
        output.append(char)
        index += 1

    return "".join(output)


def load_skill_names(path: Path) -> dict[str, int]:
    """Skill symbol -> numeric id from the committed audit table."""
    names: dict[str, int] = {}
    with path.open(newline="", encoding="utf-8") as handle:
        for row in csv.DictReader(handle):
            names[row["name"]] = int(row["skill_id"])
    return names


def split_entries(text: str) -> dict[str, str]:
    """Split an EffectTable/SkillEffect database into `key -> block` entries."""
    entries: dict[str, str] = {}
    matches = list(re.finditer(r"\n\t(\d+|[a-z][a-z0-9_]*): \[", text))

    for index, match in enumerate(matches):
        key = match.group(1)
        start = match.end()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        entries[key] = text[start:end]

    return entries


def split_components(block: str) -> list[str]:
    """Return the source of every top-level `{ ... }` object of an effect entry."""
    components: list[str] = []
    depth = 0
    start = None
    index = 0
    in_string: str | None = None

    while index < len(block):
        char = block[index]
        if in_string is not None:
            if char == "\\":
                index += 2
                continue
            if char == in_string:
                in_string = None
        else:
            if char in "'\"":
                in_string = char
            elif char == "{":
                if depth == 0:
                    start = index
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0 and start is not None:
                    components.append(block[start : index + 1])
        index += 1

    return components


def string_field(body: str, name: str) -> str | None:
    match = re.search(rf"\b{re.escape(name)}\s*:\s*'([^']*)'", body)
    return match.group(1) if match else None


def number_expression(body: str, name: str) -> float | None:
    match = re.search(rf"\b{re.escape(name)}\s*:\s*({NUMBER_PATTERN})(?:\s*,\s*|\s*$)", body)
    if match is not None:
        return float(match.group(1))
    # Simple arithmetic (`246 / 255`, `alpha / 2.55`) used for colours/alpha.
    match = re.search(
        rf"\b{re.escape(name)}\s*:\s*({NUMBER_PATTERN})\s*([/*])\s*({NUMBER_PATTERN})(?:\s*,\s*|\s*$)",
        body,
    )
    if match is None:
        return None
    left, operator, right = float(match.group(1)), match.group(2), float(match.group(3))
    if operator == "/" and right == 0.0:
        return None
    return left / right if operator == "/" else left * right


def boolean_field(body: str, name: str) -> bool | None:
    match = re.search(rf"\b{re.escape(name)}\s*:\s*(true|false)", body)
    return match.group(1) == "true" if match else None


def color_field(body: str) -> list[float] | None:
    red = number_expression(body, "red")
    green = number_expression(body, "green")
    blue = number_expression(body, "blue")
    if red is None or green is None or blue is None:
        return None
    return [round(red, 4), round(green, 4), round(blue, 4)]


def component_facts(component: str) -> dict:
    facts: dict = {}

    for name in STRING_COMPONENT_FIELDS:
        value = string_field(component, name)
        if value is not None:
            facts[name] = value

    for name in NUMERIC_COMPONENT_FIELDS:
        value = number_expression(component, name)
        if value is not None:
            # Integers stay integers so the committed JSON stays readable.
            facts[name] = int(value) if value.is_integer() else value

    for name in BOOLEAN_COMPONENT_FIELDS:
        value = boolean_field(component, name)
        if value is not None:
            facts[name] = value

    color = color_field(component)
    if color is not None:
        facts["color"] = color

    func_tokens: list[str] = []
    for match in FUNC_TOKEN_PATTERN.finditer(component):
        token = next((group for group in match.groups() if group), None)
        if token is None:
            continue
        func_tokens.append(f"Camera.{token}" if match.group(2) else token)
    if func_tokens:
        facts["func"] = sorted(set(func_tokens))

    elements = ELEMENT_PATTERN.findall(component)
    if elements:
        facts["element"] = elements[0]

    return facts


def split_value(value: str) -> list[dict]:
    """Split a SkillEffect field value into numeric ids / symbolic effect names."""
    references: list[dict] = []
    for match in re.finditer(r"'([^']*)'|(\d+)", value):
        if match.group(1) is not None:
            references.append({"name": match.group(1)})
        else:
            references.append({"id": int(match.group(2))})
    return references


def field_value(body: str, name: str) -> str | None:
    """Return the literal value of `name` in a skill effect entry.

    Values can be numbers, quoted symbolic names, flat arrays or (for the screen
    side variants) small objects. Only values that only contain effect
    references are extracted, so a nested `{}` is read as a flat list of ids.
    """
    match = re.search(rf"\b{re.escape(name)}\s*:", body)
    if match is None:
        return None
    rest = body[match.end() :].lstrip()

    if rest.startswith("["):
        depth = 0
        for index, char in enumerate(rest):
            if char == "[":
                depth += 1
            elif char == "]":
                depth -= 1
                if depth == 0:
                    return rest[: index + 1]
        return None

    if rest.startswith("{"):
        depth = 0
        for index, char in enumerate(rest):
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    return rest[: index + 1]
        return None

    match = re.match(rf"('(?:[^']*)'|{NUMBER_PATTERN})", rest)
    return match.group(1) if match else None


def parse_skill_effects(text: str, skill_names: dict[str, int]) -> dict[int, list[dict]]:
    """Return skill id -> [{field, references}] for every known field."""
    result: dict[int, list[dict]] = {}

    for match in re.finditer(r"SkillEffect\[SK\.([A-Z0-9_]+)\]\s*=\s*\{(.*?)\};", text, re.S):
        symbol = match.group(1)
        body = match.group(2)
        skill_id = skill_names.get(symbol)
        if skill_id is None:
            continue

        fields: list[dict] = []
        for field in KNOWN_FIELDS:
            value = field_value(body, field)
            if value is None:
                continue
            references = split_value(value)
            if references:
                fields.append({"field": field, "references": references})

        if fields:
            result[skill_id] = fields

    return result


def referenced_effects(skills: dict[int, list[dict]]) -> tuple[set[int], set[str]]:
    numeric: set[int] = set()
    symbolic: set[str] = set()
    for fields in skills.values():
        for field in fields:
            for reference in field["references"]:
                if "id" in reference:
                    numeric.add(reference["id"])
                else:
                    symbolic.add(reference["name"])
    return numeric, symbolic


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", default=None, help="local checkout directory of the two JS tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--components", default=str(DEFAULT_COMPONENTS))
    parser.add_argument(
        "--all-effects",
        action="store_true",
        help="extract every effect table entry instead of only the ones referenced by skills",
    )
    args = parser.parse_args()

    sources = read_sources(args.source_dir)
    skill_effects = strip_comments(sources["skill_effect"])
    effect_table = strip_comments(sources["effect_table"])
    skill_names = load_skill_names(Path(args.components))

    skills = parse_skill_effects(skill_effects, skill_names)
    numeric, symbolic = referenced_effects(skills)

    effect_entries = split_entries(effect_table)
    effects: dict[str, list[dict]] = {}
    for key, block in effect_entries.items():
        if not args.all_effects:
            if key.isdigit() and int(key) not in numeric:
                continue
            if not key.isdigit() and key not in symbolic:
                continue
        components = [facts for facts in (component_facts(component) for component in split_components(block)) if facts]
        if components:
            effects[key] = components

    payload = {
        "provenance": {
            "source": "roBrowserLegacy (GPL-3.0) RE dump of the official client effect tables; only facts (ids, types, "
            "numeric parameters, file/texture names, symbolic effect names and FUNC constructor names) are extracted",
            "url": FILES["skill_effect"],
            "packetver_note": "extracted facts; the procedural spec is validated against the CondaRO GRFs by "
            "gen_skill_procedural_effects.py",
        },
        "skills": {
            str(skill_id): fields
            for skill_id, fields in sorted(
                skills.items(),
                key=lambda item: item[0],
            )
        },
        "effects": {
            key: effects[key]
            for key in sorted(effects, key=lambda entry: (entry.isdigit(), int(entry) if entry.isdigit() else entry))
        },
    }

    output = Path(args.output)
    output.write_text(json.dumps(payload, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{output}: {len(skills)} skills, {len(effects)} effect table entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
