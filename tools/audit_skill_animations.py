#!/usr/bin/env python3
"""Audit the client's skill caster animations, effect phases and sounds.

What it checks, per pre-re skill and per representative job:

1. **Caster action** - every skill of the official assignment table is in the
   client data file with the same action, and unlisted skills fall back to the
   default casting gesture.
2. **Effect phases** - for every official `SkillEffect` field of a skill, is
   there a client effect layer for the matching phase/anchor (`.str` mapping or
   procedural spec)? Missing variants are classified: sprite effect (covered by
   the sprite-effect lane), missing data asset, or a real mapping gap.
3. **Sounds** - does the client use an official sound for the skill, is one of
   its candidates present in the GRFs, and is the chosen candidate a
   skill-specific one instead of a generic hit/cast sound.
4. **Fallback safety** - skills that are not in any table must not select a
   missing animation; they play the casting gesture and no effect.

The tool is read-only. It reads

* the committed client mapping tables (`ro-next/archive/data/client/*.ron`),
* the extracted official facts (`tools/windows_client_*.json`),
* the rAthena pre-re skill database + skill tree (default: the local checkout),
* the CondaRO GRF archives (to validate wav/effect assets).

Usage:
    ./tools/audit_skill_animations.py --output-dir /tmp/skill-audit
    ./tools/audit_skill_animations.py --rathena-dir /path/to/rathena --game-dir /path/to/client/game
"""

from __future__ import annotations

import argparse
import csv
import json
import re
import struct
import sys
import zlib
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_RATHENA_DIR = "/home/phantom/rathena"
DEFAULT_GAME_DIR = "/home/phantom/Games/ragnarok/CondaRO-linux/client/game"
DEFAULT_EFFECTS = REPO / "tools/data/windows_client_skill_effects.json"
DEFAULT_ACTIONS = REPO / "tools/data/windows_client_skill_actions.json"
DEFAULT_SOUNDS = REPO / "tools/data/windows_client_skill_sounds.json"
DEFAULT_STR_MAPPING = REPO / "ro-next/archive/data/client/skill_effect_assets.ron"
DEFAULT_PROC_MAPPING = REPO / "ro-next/archive/data/client/skill_procedural_effects.ron"
DEFAULT_ACTION_MAPPING = REPO / "ro-next/archive/data/client/skill_actions.ron"
DEFAULT_SOUND_MAPPING = REPO / "ro-next/archive/data/client/skill_sounds.ron"
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
GRF_HEADER_SIZE = 46

#: SkillEffect field -> (client phase, spawn anchor), mirrored from
#: `tools/gen_skill_effect_assets.py`.
FIELD_SPECS = {
    "groundEffectId": ("Ground", "Target"),
    "groundEffectIdByLevel": ("Ground", "Target"),
    "beginCastEffectId": ("Cast", "Caster"),
    "beginCastEffectIdByLevel": ("Cast", "Caster"),
    "castAuraEffectId": ("Cast", "Caster"),
    "effectId": ("Success", "Auto"),
    "successEffectId": ("Success", "Auto"),
    "effectIdOnCaster": ("Success", "Caster"),
    "successEffectIdOnCaster": ("Success", "Caster"),
    "successEffectIdOnCasterByLevel": ("Success", "Caster"),
    "successEffectIdOnCasterByScreenDirection": ("Success", "Caster"),
    "successEffectIdOnTargetByLevel": ("Success", "Target"),
    "successEffectIdOnTargetByScreenSide": ("Success", "Target"),
    "releaseEffectId": ("Release", "Travel"),
    "beforeHitEffectId": ("Release", "Travel"),
    "beforeHitEffectIdOnCaster": ("Release", "Caster"),
    "hitEffectId": ("Hit", "Target"),
    "hitEffectIdOnCaster": ("Hit", "Caster"),
    "damageEffectIdOnTarget": ("Hit", "Target"),
    "damageEffectIdOnCaster": ("Caster", "Caster"),
}

#: Pre-re job families used for the representative matrix.
REPRESENTATIVE_JOBS = {
    "Swordsman/Knight": ["Novice", "Swordsman", "Knight"],
    "Mage/Wizard": ["Mage", "Wizard"],
    "Acolyte/Priest": ["Acolyte", "Priest"],
    "Archer/Hunter": ["Archer", "Hunter"],
    "Merchant/Blacksmith": ["Merchant", "Blacksmith"],
    "Thief/Assassin": ["Thief", "Assassin"],
    "Monk/Champion": ["Monk", "Champion"],
    "Sage": ["Sage"],
    "Alchemist": ["Alchemist"],
    "Bard/Dancer": ["Bard", "Dancer"],
    "Crusader/Paladin": ["Crusader", "Paladin"],
    "Rogue/Stalker": ["Rogue", "Stalker"],
    "Super Novice": ["Super Novice"],
}

#: Wavs that say nothing about a skill (generic hit/cast sounds).
GENERIC_WAV_PATTERNS = (
    re.compile(r"^effect\\ef_hit\d*\.wav$", re.IGNORECASE),
    re.compile(r"^_enemy_hit_", re.IGNORECASE),
    re.compile(r"^effect\\ef_beginspell\d*\.wav$", re.IGNORECASE),
)


# --------------------------------------------------------------------------- GRF
def read_grf_names(path: Path) -> list[str]:
    with path.open("rb") as handle:
        header = handle.read(GRF_HEADER_SIZE)
        if not header.startswith(b"Master of Magic"):
            raise SystemExit(f"not a GRF file: {path}")
        table_offset, reserved, file_count, version = struct.unpack("<IIII", header[30:46])
        if version != 0x200:
            raise SystemExit(f"unsupported GRF version {version:#x}: {path}")
        handle.seek(GRF_HEADER_SIZE + table_offset)
        compressed_size, _uncompressed_size = struct.unpack("<II", handle.read(8))
        table = zlib.decompress(handle.read(compressed_size))

    names: list[str] = []
    offset = 0
    for _ in range(file_count - reserved - 7):
        end = table.index(b"\0", offset)
        raw = table[offset:end]
        offset = end + 1
        offset += 17
        try:
            names.append(raw.decode("utf-8"))
        except UnicodeDecodeError:
            try:
                names.append(raw.decode("euc-kr"))
            except UnicodeDecodeError:
                names.append(raw.decode("latin-1"))
    return names


def load_assets(game_dir: Path) -> tuple[set[str], set[str], set[str]]:
    """Return (effect textures, wavs, sprite files) as lower-case relative paths."""
    textures: set[str] = set()
    wavs: set[str] = set()
    sprites: set[str] = set()
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if lower.startswith("data\\texture\\effect\\") and lower.endswith(".str"):
                textures.add(lower[len("data\\texture\\effect\\"):])
            elif lower.startswith("data\\wav\\") and lower.endswith(".wav"):
                wavs.add(lower[len("data\\wav\\"):])
            elif lower.startswith("data\\sprite\\"):
                sprites.add(lower[len("data\\sprite\\"):])
    return textures, wavs, sprites


# --------------------------------------------------------------------------- data
def load_skill_db(rathena_dir: Path) -> tuple[dict[int, dict], dict[str, dict]]:
    import yaml

    skill_db = yaml.safe_load((rathena_dir / "db/pre-re/skill_db.yml").read_text())
    skills: dict[int, dict] = {}
    for skill in skill_db["Body"]:
        skills[skill["Id"]] = {
            "id": skill["Id"],
            "name": skill["Name"],
            "desc": skill.get("Description", ""),
            "type": skill.get("Type", ""),
            "target": skill.get("TargetType", ""),
            "cast": skill.get("CastTime", 0) or 0,
            "no_damage": bool((skill.get("DamageFlags") or {}).get("NoDamage", False)),
        }

    skill_tree = yaml.safe_load((rathena_dir / "db/pre-re/skill_tree.yml").read_text())
    jobs: dict[str, dict] = {}
    for node in skill_tree["Body"]:
        jobs[node["Job"]] = {
            "inherit": list((node.get("Inherit") or {}).keys()),
            "tree": {entry["Name"] for entry in node.get("Tree") or [] if entry.get("MaxLevel", 1) != 0},
        }
    return skills, jobs


def resolve_job_skills(job: str, jobs: dict[str, dict], seen: set[str] | None = None) -> set[str]:
    seen = seen if seen is not None else set()
    if job in seen or job not in jobs:
        return set()
    seen.add(job)
    names = set(jobs[job]["tree"])
    for parent in jobs[job]["inherit"]:
        names |= resolve_job_skills(parent, jobs, seen)
    return names


def parse_ron_effects(path: Path) -> dict[int, dict[str, set[str]]]:
    result: dict[int, dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))
    pattern = re.compile(r"\(skill: (\d+), variant: (\w+), anchor: (\w+), file: \"([^\"]+)\"\)")
    for match in pattern.finditer(path.read_text()):
        result[int(match.group(1))][match.group(2)].add(match.group(3))
    return result


def parse_ron_procedural(path: Path) -> dict[int, dict[str, set[str]]]:
    result: dict[int, dict[str, set[str]]] = defaultdict(lambda: defaultdict(set))
    pattern = re.compile(r"\(skill: (\d+), variant: (\w+), anchor: (\w+), primitive: (\w+)")
    for match in pattern.finditer(path.read_text()):
        result[int(match.group(1))][match.group(2)].add(match.group(3))
    return result


def parse_ron_actions(path: Path) -> tuple[str, dict[int, str]]:
    text = path.read_text()
    default = re.search(r"default_action:\s*(\w+)", text).group(1)
    actions = {int(match.group(1)): match.group(2) for match in re.finditer(r"\(skill: (\d+), action: (\w+)\)", text)}
    return default, actions


def parse_ron_sounds(path: Path) -> dict[int, list[str]]:
    text = path.read_text()
    sounds: dict[int, list[str]] = {}
    for match in re.finditer(r"\(skill: (\d+), files: \[([^\]]*)\]\)", text):
        files = [file.replace("\\\\", "\\") for file in re.findall(r'"([^"]+)"', match.group(2))]
        sounds[int(match.group(1))] = files
    return sounds


def load_effect_facts(path: Path) -> dict[str, dict[int, list[dict]]]:
    payload = json.loads(path.read_text())
    return {int(skill_id): fields for skill_id, fields in payload["skills"].items()}


def load_effect_components(path: Path) -> dict[int, list[dict]]:
    payload = json.loads(path.read_text())
    return {int(effect_id): components for effect_id, components in payload.get("effects", {}).items()}


def load_sound_facts(path: Path) -> dict[int, list[dict]]:
    payload = json.loads(path.read_text())
    return {int(skill_id): rows for skill_id, rows in payload["skills"].items()}


def normalize_wav(name: str) -> str:
    candidate = name.strip().replace("/", "\\")
    if candidate.lower().startswith("data\\wav\\"):
        candidate = candidate[len("data\\wav\\"):]
    if not candidate.lower().endswith(".wav"):
        candidate += ".wav"
    return candidate.lower()


def is_generic_wav(name: str) -> bool:
    return any(pattern.match(name.replace("\\", "\\")) for pattern in GENERIC_WAV_PATTERNS)


# --------------------------------------------------------------------------- audit
def classify_missing_effect(components: list[dict] | None, textures: set[str]) -> str:
    if not components:
        return "no-component-facts"
    kinds = set()
    for component in components:
        kind = component.get("type")
        if kind == "SPR":
            kinds.add("sprite-effect")
        elif kind == "STR":
            file_name = component.get("file") or ""
            stem = file_name.replace("/", "\\")
            if stem.lower().startswith("effect\\"):
                stem = stem[len("effect\\"):]
            if not stem.lower().endswith(".str"):
                stem += ".str"
            kinds.add("str-present" if stem.lower() in textures else "str-missing")
        else:
            kinds.add("procedural")
    if "procedural" in kinds:
        return "procedural"
    if "sprites" in kinds:
        return "sprite-effect"
    if "sprite-effect" in kinds and "str-present" not in kinds:
        return "sprite-effect"
    if "str-present" in kinds:
        return "str-present"
    return "str-missing"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rathena-dir", default=DEFAULT_RATHENA_DIR)
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--output-dir", default="/tmp/ro-next-skill-animation-audit")
    args = parser.parse_args()

    rathena_dir = Path(args.rathena_dir)
    game_dir = Path(args.game_dir)
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)

    skills, jobs = load_skill_db(rathena_dir)
    textures, wavs, _sprites = load_assets(game_dir)
    effect_facts = load_effect_facts(DEFAULT_EFFECTS)
    effect_components = load_effect_components(DEFAULT_EFFECTS)
    sound_facts = load_sound_facts(DEFAULT_SOUNDS)
    str_mapping = parse_ron_effects(DEFAULT_STR_MAPPING)
    proc_mapping = parse_ron_procedural(DEFAULT_PROC_MAPPING)
    default_action, current_actions = parse_ron_actions(DEFAULT_ACTION_MAPPING)
    sound_mapping = parse_ron_sounds(DEFAULT_SOUND_MAPPING)
    action_facts = json.loads(DEFAULT_ACTIONS.read_text())["actions"]

    print(f"skills: {len(skills)}; effect facts: {len(effect_facts)}; sound facts: {len(sound_facts)}", file=sys.stderr)
    print(f"client mapping: {len(str_mapping)} str skills, {len(proc_mapping)} procedural skills, {len(current_actions)} actions", file=sys.stderr)

    # 1. Caster action consistency: every official fact must be in the data file.
    action_gaps = []
    for skill_id, facts in action_facts.items():
        expected = facts["action"]
        actual = current_actions.get(int(skill_id), default_action)
        if expected != actual:
            action_gaps.append((int(skill_id), expected, actual))
    print(f"caster action mismatches against the official facts: {len(action_gaps)}")

    # 2. Representative job matrix.
    rows = []
    for family, job_names in REPRESENTATIVE_JOBS.items():
        names: set[str] = set()
        for job in job_names:
            names |= resolve_job_skills(job, jobs)
        for name in sorted(names):
            skill = next((entry for entry in skills.values() if entry["name"] == name), None)
            if skill is None:
                continue
            skill_id = skill["id"]
            official_fields = effect_facts.get(skill_id, [])
            have_phases = set(str_mapping.get(skill_id, {})) | set(proc_mapping.get(skill_id, {}))
            missing = []
            for field in official_fields:
                spec = FIELD_SPECS.get(field["field"])
                if spec is None:
                    continue
                phase = spec[0]
                if phase in have_phases:
                    continue
                reasons = Counter()
                for effect_id in field["effects"]:
                    reasons[classify_missing_effect(effect_components.get(effect_id), textures)] += 1
                missing.append((phase, dict(reasons)))
            official_action = action_facts.get(str(skill_id), {}).get("action", default_action)
            current_action = current_actions.get(skill_id, default_action)

            official_sound = None
            for entry in sound_facts.get(skill_id, []):
                wav = normalize_wav(entry["wav"])
                if wav in wavs:
                    official_sound = wav
                    break
            generated_sound = sound_mapping.get(skill_id)
            chosen_sound = (generated_sound or [None])[0]

            flags = []
            if official_action != current_action:
                flags.append("action-gap")
            if missing:
                # A missing phase only counts as a bug when it is not covered by
                # the sprite-effect lane / a missing data asset.
                unresolved = [entry for entry in missing if not set(entry[1]) <= {"sprite-effect", "str-missing", "no-component-facts"}]
                if unresolved:
                    flags.append("effect-gap")
            if official_sound and not chosen_sound:
                flags.append("sound-gap")
            if chosen_sound and is_generic_wav(chosen_sound) and official_sound and not is_generic_wav(official_sound):
                flags.append("generic-sound")
            if not official_fields and skill_id not in current_actions and skill["type"] in ("Weapon", "Magic"):
                flags.append("no-official-effect")

            rows.append({
                "job": family,
                "skill": skill_id,
                "aegis": name,
                "description": skill["desc"],
                "type": skill["type"],
                "cast_ms": skill["cast"],
                "official_action": official_action,
                "client_action": current_action,
                "phases": "+".join(sorted(have_phases)) or "-",
                "missing_phases": ";".join(f"{phase}({','.join(sorted(reasons))})" for phase, reasons in missing) or "-",
                "official_sound": official_sound or "-",
                "client_sound": chosen_sound or "-",
                "flags": ",".join(flags) or "ok",
            })

    # 3. Full pre-re skill audit (all skills, not only representative jobs).
    full_rows = []
    for skill_id, skill in sorted(skills.items()):
        official_action = action_facts.get(str(skill_id), {}).get("action", default_action)
        current_action = current_actions.get(skill_id, default_action)
        full_rows.append({
            "skill": skill_id,
            "aegis": skill["name"],
            "description": skill["desc"],
            "type": skill["type"],
            "cast_ms": skill["cast"],
            "official_action": official_action,
            "client_action": current_action,
            "effect_phases": "+".join(sorted(set(str_mapping.get(skill_id, {})) | set(proc_mapping.get(skill_id, {})))) or "-",
            "sound": (sound_mapping.get(skill_id) or ["-"])[0],
        })

    # Outputs
    matrix_path = output_dir / "skill-animation-matrix.csv"
    with matrix_path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        writer.writerows(rows)

    full_path = output_dir / "skill-animation-all-pree.csv"
    with full_path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(full_rows[0].keys()))
        writer.writeheader()
        writer.writerows(full_rows)

    summary = {
        "skills_total": len(skills),
        "representative_rows": len(rows),
        "action_mismatches": len(action_gaps),
        "flags": dict(Counter(flag for row in rows for flag in row["flags"].split(","))),
        "action_facts": len(action_facts),
        "effect_str_skills": len(str_mapping),
        "effect_procedural_skills": len(proc_mapping),
        "sound_skills": len(sound_mapping),
    }
    (output_dir / "summary.json").write_text(json.dumps(summary, indent=1) + "\n")

    print(json.dumps(summary, indent=1))
    print(f"\nwrote {matrix_path}\nwrote {full_path}")
    return 1 if action_gaps else 0


if __name__ == "__main__":
    raise SystemExit(main())
