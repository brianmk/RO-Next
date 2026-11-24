#!/usr/bin/env python3
"""Extract the official skill -> effect mapping facts from the Windows client tables.

Background: the official Windows client (`CondaRO.exe`) compiles its per-skill
effect mapping into code; its effect **resources** live in the GRFs. The
community reimplementation `roBrowserLegacy` (GPL-3.0) documents those mappings
in `src/DB/Skills/SkillEffect.js`, `src/DB/Effects/EffectTable.js` and
`src/DB/Skills/SkillConst.js`. This script only extracts **facts** (skill ids,
effect ids, the exact `SkillEffect` field they come from and the effect files)
into a plain JSON file; none of the GPL source is copied into this repository.

The exact field name is kept because it decides **when** and **where** an
effect is played (for example `effectIdOnCaster` plays on the caster while
`successEffectIdOnTargetByLevel` plays at the target). Keeping the field in the
facts lets `gen_skill_effect_assets.py` write the correct spawn anchor instead
of guessing it from a coarse group.

The extracted JSON (`tools/data/windows_client_skill_effects.json`) is the input of
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
import sys
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/data/windows_client_skill_effects.json")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"

FILES = {
    "skill_const": f"{BASE_URL}/Skills/SkillConst.js",
    "skill_effect": f"{BASE_URL}/Skills/SkillEffect.js",
    "effect_table": f"{BASE_URL}/Effects/EffectTable.js",
}

#: `SkillEffect` fields we understand. The value is the official play moment of
#: the field; `gen_skill_effect_assets.py` maps it to a client spawn phase and
#: anchor. Order matters: it is the fallback priority of the generator.
FIELDS = (
    # ground target cell
    "groundEffectId",
    "groundEffectIdByLevel",
    # cast start on the caster
    "beginCastEffectId",
    "beginCastEffectIdByLevel",
    "castAuraEffectId",
    # successful use
    "effectId",
    "successEffectId",
    "effectIdOnCaster",
    "successEffectIdOnCaster",
    "successEffectIdOnCasterByLevel",
    "successEffectIdOnCasterByScreenDirection",
    "successEffectIdOnCasterAimed",
    "successEffectIdOnTargetByLevel",
    "successEffectIdOnTargetByScreenSide",
    # release / just before the hit
    "releaseEffectId",
    "beforeHitEffectId",
    "beforeHitEffectIdOnCaster",
    # damage hit / caster damage
    "hitEffectId",
    "firstHitEffectId",
    "hitEffectIdOnCaster",
    "damageEffectIdOnTarget",
    "damageEffectIdOnCaster",
)

#: SkillEffect values that are not effect ids but client instructions.
IGNORED_FLAGS = ("hideCastBar", "hideCastAura", "hideCastEffect")


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


def strip_comments(text: str) -> str:
    return re.sub(r"//[^\n]*", "", text)


def numeric_ids(value: str) -> list[int]:
    """Return the numeric effect ids of a JS field value (numbers or arrays).

    Effect id 0 is the official "standard hit flash" and is kept as a fact; the
    generator reports it explicitly (its visual is a hard-coded primitive, not
    an effect-table file).
    """
    return [int(number) for number in re.findall(r"\b(\d{1,4})\b", value)]


def symbolic_names(value: str) -> list[str]:
    """Return the symbolic effect-block names referenced by a field value."""
    return sorted(set(re.findall(r"'((?:[^'\\]|\\.)*)'", value)))


def parse_skill_effects(text: str, skill_ids: dict[str, int]) -> dict[int, list[dict]]:
    """Return skill id -> [{"field": <SkillEffect field>, "effects": [ids]}]."""
    text = strip_comments(text)
    # One-liners and multi-line blocks. The value can be a literal object or an
    # alias to another skill (`SkillEffect[SK.X] = SkillEffect[SK.Y];`).
    raw: dict[str, str] = {}
    for match in re.finditer(r"SkillEffect\[SK\.([A-Z0-9_]+)\]\s*=\s*(\{(?:[^{}]|\{[^{}]*\})*\}|SkillEffect\[SK\.[A-Z0-9_]+\])\s*;", text, re.S):
        raw[match.group(1)] = match.group(2)

    resolved: dict[str, str] = {}

    def resolve(symbol: str, depth: int = 0) -> str:
        if symbol in resolved:
            return resolved[symbol]
        value = raw.get(symbol, "")
        alias = re.fullmatch(r"SkillEffect\[SK\.([A-Z0-9_]+)\]", value)
        if alias and depth < 8:
            value = resolve(alias.group(1), depth + 1)
        resolved[symbol] = value
        return value

    result: dict[int, list[dict]] = {}
    for symbol in raw:
        skill_id = skill_ids.get(symbol)
        if skill_id is None:
            continue
        body = resolve(symbol)
        fields: list[dict] = []
        for field in FIELDS:
            field_match = re.search(rf"\b{field}\s*:\s*(\[[^\]]*\]|'[^']*'|\d+)", body)
            if field_match is None:
                continue
            value = field_match.group(1)
            ids = numeric_ids(value)
            names = symbolic_names(value)
            # `effects` is always present (possibly empty) so consumers that
            # predate symbolic references keep working unchanged.
            entry: dict = {"field": field, "effects": sorted(set(ids))}
            if names:
                entry["symbolic"] = names
            if ids or names:
                fields.append(entry)
        if fields:
            result[skill_id] = fields
    return result


def js_string_to_path(raw: str) -> str:
    """Decode the JS string escapes used for Korean file names.

    roBrowser Legacy stores EUC-KR file name bytes as individual `\\xNN`
    characters in the JS source; the game loader turns them back into EUC-KR.
    """
    if "\\x" not in raw:
        return raw
    data = re.sub(rb"\\x([0-9a-fA-F]{2})", lambda match: bytes([int(match.group(1), 16)]), raw.encode("latin-1"))
    try:
        return data.decode("euc-kr")
    except UnicodeDecodeError:
        return raw


def parse_effect_table(text: str) -> dict[int, list[dict]]:
    """Return effect id -> [{type, file}] for the effect table entries."""
    result: dict[int, list[dict]] = {}
    entries = list(re.finditer(r"\n\t(\d+): \[", text))
    for index, match in enumerate(entries):
        effect_id = int(match.group(1))
        start = match.end()
        end = entries[index + 1].start() if index + 1 < len(entries) else len(text)
        block = text[start:end]
        files: list[dict] = []
        for file_match in re.finditer(r"file:\s*'([^']+)'", block):
            files.append({"file": js_string_to_path(file_match.group(1))})
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
            "source": "roBrowserLegacy (GPL-3.0) RE dump of the official client tables; only numeric ids, field names and file names are extracted",
            "url": FILES["skill_effect"],
            "packetver_note": "extracted facts (numeric ids, symbolic named-effect references, field names, file names); validated against CondaRO.exe + the GRFs",
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
