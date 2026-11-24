#!/usr/bin/env python3
"""Audit the client's per-skill effects/animations against the official 2019 tables.

Input:
  /tmp/official_skill_tables.txt  (dumped from the GRF by the ignored
    `dump_official_skill_tables` test; flat lines:
    `skillid|id|NAME`, `effectid|id|NAME`, `actorstate|id|NAME`,
    `skill|skillid|FIELD|value` / `FIELD[n]|value`)

Compared against:
  korangar/archive/data/client/skill_effects.ron  (skill -> phase -> layers)
  korangar/archive/data/client/skill_sounds.ron   (skill -> wave files)
  korangar/archive/data/client/skill_actions.ron  (skill -> caster action)
  tools/windows_client_effect_layers.json         (effect id -> layer assets)

Output: a mismatch report on stdout and `tools/skill_parity_report.json`.
"""

from __future__ import annotations

import json
import re
from collections import defaultdict
from pathlib import Path

TABLES = Path("/tmp/official_skill_tables.txt")
EFFECTS = Path("korangar/archive/data/client/skill_effects.ron")
SOUNDS = Path("korangar/archive/data/client/skill_sounds.ron")
ACTIONS = Path("korangar/archive/data/client/skill_actions.ron")
LAYERS = Path("tools/windows_client_effect_layers.json")
REPORT = Path("tools/skill_parity_report.json")

# official ACTOR_STATE id -> our caster action
STATE_ACTION = {
    0: "Idle",   # ST_STAND
    2: "Attack", # ST_ATTACK
    7: "Skill",  # ST_SKILL
    8: "Skill",  # ST_BEGIN_SKILL
    5: "Pickup", # ST_PICKUP
}


def parse_official() -> tuple[dict[str, int], dict[int, dict[str, list]], dict[int, str], dict[str, int], dict[int, str]]:
    names: dict[str, int] = {}
    skill_fields: dict[int, dict[str, list]] = defaultdict(lambda: defaultdict(list))
    effect_names: dict[int, str] = {}
    actor_states: dict[str, int] = {}
    id_to_name: dict[int, str] = {}

    def decode(value: str) -> str:
        if value.startswith("hex:"):
            return bytes.fromhex(value[4:]).decode("cp949", "replace")
        return value

    for line in TABLES.read_text().splitlines():
        parts = [decode(part) for part in line.split("|")]
        if parts[0] == "skillid" and len(parts) == 3:
            name, skill_id = parts[2], int(parts[1])
            names[name] = skill_id
            id_to_name[skill_id] = name
        elif parts[0] == "effectid" and len(parts) == 3:
            effect_names[int(parts[1])] = parts[2]
        elif parts[0] == "actorstate" and len(parts) == 3:
            actor_states[parts[2]] = int(parts[1])
        elif parts[0] == "skill" and len(parts) == 4:
            skill_id = int(parts[1])
            field = re.sub(r"\[\d+\]$", "", parts[2])
            skill_fields[skill_id][field].append(parts[3])

    return names, skill_fields, effect_names, actor_states, id_to_name


def parse_ron_skills(path: Path, block_re: str) -> dict[int, list[str]]:
    text = path.read_text()
    result: dict[int, list[str]] = defaultdict(list)
    for match in re.finditer(block_re, text):
        skill_id = int(match.group(1))
        block = match.group(0)
        result[skill_id].append(block)
    return result


def unescape(path: str) -> str:
    """RON stores backslashes escaped (`effect\\x.tga`); give back the path."""
    return path.replace("\\\\", "\\")


def parse_effects() -> dict[int, dict]:
    text = EFFECTS.read_text()
    skills: dict[int, dict] = {}

    for match in re.finditer(r"\(\s*skill: (\d+),\s*phase: (\w+),.*?\n        \),", text, re.S):
        skill_id = int(match.group(1))
        phase = match.group(2)
        block = match.group(0)
        entry = skills.setdefault(skill_id, {"phases": defaultdict(lambda: {"files": [], "wavs": [], "kinds": []})})
        files = re.findall(r'file: "([^"]+)"', block) + [f for group in re.findall(r'files: \[([^\]]*)\]', block) for f in re.findall(r'"([^"]+)"', group)]
        frames = [f for group in re.findall(r'frames: \[([^\]]*)\]', block) for f in re.findall(r'"([^"]+)"', group)]
        wavs = re.findall(r'wav: "([^"]+)"', block)
        files = [unescape(f) for f in files]
        frames = [unescape(f) for f in frames]
        wavs = [unescape(f) for f in wavs]
        kinds = re.findall(r"kind: (\w+)", block)
        entry["phases"][phase]["files"].extend(files + frames)
        entry["phases"][phase]["wavs"].extend(wavs)
        entry["phases"][phase]["kinds"].extend(kinds)

    return skills


def parse_sounds() -> dict[int, list[str]]:
    text = SOUNDS.read_text()
    sounds: dict[int, list[str]] = defaultdict(list)
    for match in re.finditer(r"\(skill: (\d+), files: \[([^\]]*)\]\)", text):
        skill_id = int(match.group(1))
        sounds[skill_id].extend(unescape(wav) for wav in re.findall(r'"([^"]+)"', match.group(2)))
    return sounds


def parse_actions() -> dict[int, str]:
    text = ACTIONS.read_text()
    return {int(m.group(1)): m.group(2) for m in re.finditer(r"\(skill: (\d+), action: (\w+)\)", text)}


def effect_assets(effect_id: int, layers: dict) -> list[str]:
    entry = layers.get(str(effect_id))
    if not entry:
        return []
    result = []
    for layer in entry.get("layers", []):
        for key in ("file", "textureName", "strName"):
            if layer.get(key):
                result.append(layer[key])
        for key in ("files",):
            result.extend(layer.get(key) or [])
    return result


def main() -> None:
    names, official, effect_names, actor_states, id_to_name = parse_official()
    effects = parse_effects()
    sounds = parse_sounds()
    actions = parse_actions()
    layers = json.loads(LAYERS.read_text())["effects"]

    report = []
    for skill_id in sorted(official):
        fields = official[skill_id]
        name = id_to_name.get(skill_id, str(skill_id))
        entry = effects.get(skill_id, {"phases": {}})

        our_files = [f for phase in entry["phases"].values() for f in phase["files"]]
        our_wavs = [f for phase in entry["phases"].values() for f in phase["wavs"]] + sounds.get(skill_id, [])

        official_effect_ids = []
        for field in ("effectID", "targetEffectID", "groundEffectID", "beginEffectID", "toTargetEffectID"):
            official_effect_ids.extend(int(v) for v in fields.get(field, []))

        official_assets = []
        for effect_id in official_effect_ids:
            official_assets.extend(effect_assets(effect_id, layers))

        official_wavs = []
        for field in ("waveFileName", "targetWaveFileName", "beginWaveFileName"):
            official_wavs.extend(fields.get(field, []))

        # Both sides use Windows-style separators; normalize before taking the
        # stem so Linux `Path` does not treat the whole string as one name.
        ours_effect_names = {Path(f.replace("\\", "/")).stem.lower() for f in our_files}
        official_assets_names = {Path(a.replace("\\", "/")).stem.lower() for a in official_assets}

        def asset_present(asset: str) -> bool:
            if asset in ours_effect_names:
                return True
            # The tables use templates (`windhit%d`) for numbered files.
            if "%" in asset:
                pattern = re.escape(asset).replace("%d", r"\d+").replace("%s", r".+")
                return any(re.fullmatch(pattern, name) for name in ours_effect_names)
            return False

        missing_assets = sorted(a for a in official_assets_names if not asset_present(a))
        missing_wavs = sorted(w for w in official_wavs if Path(w).name.lower() not in {Path(x).name.lower() for x in our_wavs})

        official_motion = fields.get("beginMotionType")
        official_action = None
        if official_motion:
            official_action = STATE_ACTION.get(int(official_motion[0]))

        report.append({
            "skill": skill_id,
            "name": name,
            "official_effect_ids": official_effect_ids,
            "official_assets": sorted(set(official_assets)),
            "official_wavs": official_wavs,
            "our_files": sorted(set(our_files)),
            "our_wavs": sorted(set(our_wavs)),
            "missing_assets": missing_assets,
            "missing_wavs": missing_wavs,
            "official_action": official_action,
            "our_action": actions.get(skill_id),
            "has_descriptor": skill_id in effects,
        })

    REPORT.write_text(json.dumps(report, indent=1) + "\n")

    no_descriptor = [r for r in report if not r["has_descriptor"]]
    missing_assets = [r for r in report if r["missing_assets"]]
    missing_wavs = [r for r in report if r["missing_wavs"]]
    action_mismatch = [r for r in report if r["official_action"] and r["our_action"] and r["official_action"] != r["our_action"]]

    print(f"skills in the official 2019 effect table: {len(report)}")
    print(f"  no client descriptor:          {len(no_descriptor)}")
    print(f"  missing effect assets:         {len(missing_assets)}")
    print(f"  missing official sound:        {len(missing_wavs)}")
    print(f"  caster action mismatch:        {len(action_mismatch)}")
    print()
    for label, rows in (("NO DESCRIPTOR", no_descriptor), ("MISSING ASSETS", missing_assets), ("MISSING SOUND", missing_wavs), ("ACTION", action_mismatch)):
        for row in rows[:40]:
            print(f"{label:<15} {row['skill']:>5} {row['name']:<28} official={row['official_effect_ids']} assets={row['missing_assets']} wavs={row['missing_wavs']} action={row['official_action']}/{row['our_action']}")
    print(f"\nreport: {REPORT}")


if __name__ == "__main__":
    main()
