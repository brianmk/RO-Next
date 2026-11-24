#!/usr/bin/env python3
"""Extract the official per-skill sound facts from the Windows client tables.

The official client plays the `wav` of the effect that a skill spawns (the
effect table entry carries the sound next to its visual layers). The 2019 map
server rarely sends a sound packet for casts, so ro-next has to play the skill
sound itself. The name-matched `.str` mapping is not enough: many skills have
no effect *file* at all (their visual is a hard-coded primitive or a sprite
animation), but they still have an official sound.

This script extracts only **facts** (numeric skill/effect ids, the exact
`SkillEffect` field, and the effect table `wav` name) from the community
reimplementation roBrowserLegacy (GPL-3.0); none of the GPL source is copied.

The extracted JSON (`tools/data/windows_client_skill_sounds.json`) is the input of
`gen_skill_sounds.py`, which validates every wav against the CondaRO GRF
archives and writes `ro-next/archive/data/client/skill_sounds.ron`.

Usage (needs network access to raw.githubusercontent.com):

    ./tools/extract_skill_sounds.py --source-dir /tmp/robrowserlegacy

or let the script download the three tables itself.

Only read-only HTTP requests are made; the script writes a single JSON file.
"""

from __future__ import annotations

import argparse
import json
import re
import urllib.request
from pathlib import Path

DEFAULT_OUTPUT = Path("tools/data/windows_client_skill_sounds.json")
BASE_URL = "https://raw.githubusercontent.com/mrantares/robrowserlegacy/master/src/DB"
FILES = {
    "skill_const": f"{BASE_URL}/Skills/SkillConst.js",
    "skill_effect": f"{BASE_URL}/Skills/SkillEffect.js",
    "effect_table": f"{BASE_URL}/Effects/EffectTable.js",
}

#: Priority order of the `SkillEffect` fields for the single client-side cast
#: sound: the effect that the player sees/hears at the successful use first, the
#: cast-start and hit layers afterwards.
FIELD_PRIORITY = (
    "effectId",
    "successEffectId",
    "successEffectIdOnCaster",
    "effectIdOnCaster",
    "successEffectIdOnCasterByLevel",
    "successEffectIdOnCasterByScreenDirection",
    "successEffectIdOnTargetByLevel",
    "successEffectIdOnTargetByScreenSide",
    "groundEffectId",
    "groundEffectIdByLevel",
    # The cast-start cone/aura sound (`ef_beginspell`) is the last resort before
    # the caster-only layer: a skill-specific hit/release sound is more
    # distinctive than the generic cast sound for instant skills.
    "releaseEffectId",
    "beforeHitEffectId",
    "beforeHitEffectIdOnCaster",
    "hitEffectId",
    "hitEffectIdOnCaster",
    "damageEffectIdOnTarget",
    "beginCastEffectId",
    "beginCastEffectIdByLevel",
    "castAuraEffectId",
    "damageEffectIdOnCaster",
)


def fetch(url: str) -> str:
    with urllib.request.urlopen(url, timeout=30) as response:
        return response.read().decode("utf-8")


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


def numeric_ids(value: str) -> list[int]:
    return [int(number) for number in re.findall(r"\b(\d{1,4})\b", value) if int(number) != 0]


def parse_skill_effects(text: str, skill_ids: dict[str, int]) -> dict[int, dict[str, list[int]]]:
    """Return skill id -> {SkillEffect field: [effect ids]} (aliases resolved)."""
    text = strip_comments(text)
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

    result: dict[int, dict[str, list[int]]] = {}
    for symbol in raw:
        skill_id = skill_ids.get(symbol)
        if skill_id is None:
            continue
        body = resolve(symbol)
        fields: dict[str, list[int]] = {}
        for field in FIELD_PRIORITY:
            field_match = re.search(rf"\b{field}\s*:\s*(\[[^\]]*\]|\d+)", body)
            if field_match is None:
                continue
            ids = numeric_ids(field_match.group(1))
            if ids:
                fields[field] = sorted(set(ids))
        if fields:
            result[skill_id] = fields
    return result


def js_string_to_path(raw: str) -> str:
    """Decode the JS string escapes used for Korean wav file names."""
    if "\\x" not in raw:
        return raw
    data = re.sub(rb"\\x([0-9a-fA-F]{2})", lambda match: bytes([int(match.group(1), 16)]), raw.encode("latin-1"))
    try:
        return data.decode("euc-kr")
    except UnicodeDecodeError:
        return raw


def parse_effect_sounds(text: str) -> dict[int, list[str]]:
    """Return effect id -> ordered, de-duplicated wav names of its components."""
    result: dict[int, list[str]] = {}
    entries = list(re.finditer(r"\n\t(\d+): \[", text))
    for index, match in enumerate(entries):
        effect_id = int(match.group(1))
        start = match.end()
        end = entries[index + 1].start() if index + 1 < len(entries) else len(text)
        block = text[start:end]
        wavs: list[str] = []
        for wav_match in re.finditer(r"wav:\s*'([^']+)'", block):
            wav = js_string_to_path(wav_match.group(1))
            if wav not in wavs:
                wavs.append(wav)
        if wavs:
            result[effect_id] = wavs
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", default=None, help="local checkout directory of the three JS tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    sources = read_sources(args.source_dir)
    skill_ids = parse_skill_const(sources["skill_const"])
    skill_fields = parse_skill_effects(sources["skill_effect"], skill_ids)
    effect_sounds = parse_effect_sounds(sources["effect_table"])

    skills: dict[int, list[dict]] = {}
    for skill_id, fields in sorted(skill_fields.items()):
        rows: list[dict] = []
        for field in FIELD_PRIORITY:
            for effect_id in fields.get(field, ()):
                for wav in effect_sounds.get(effect_id, ()):
                    rows.append({"field": field, "effect_id": effect_id, "wav": wav})
        if rows:
            skills[skill_id] = rows

    payload = {
        "provenance": {
            "source": "roBrowserLegacy (GPL-3.0) RE dump of the official client tables; only numeric ids, field names and wav names are extracted",
            "url": FILES["skill_effect"],
            "note": "effect table `wav` facts in official play priority order; validated against the CondaRO GRFs by gen_skill_sounds.py",
        },
        "skills": {str(skill_id): rows for skill_id, rows in skills.items()},
    }

    output = Path(args.output)
    output.write_text(json.dumps(payload, indent=1, ensure_ascii=False) + "\n")
    print(f"{output}: {len(skills)} skills with effects, {len(effect_sounds)} effects with sounds")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
