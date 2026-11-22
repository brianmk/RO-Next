#!/usr/bin/env python3
"""Read-only audit core: load rAthena pre-re skill data, official effect facts and
the current Korangar client mappings, then classify coverage.

Writes nothing; the driver `audit.py` writes the deliverables.
"""

from __future__ import annotations

import json
import re
import struct
import sys
import zlib
from pathlib import Path

import yaml

ROOT = Path("/home/phantom/Games/ragnarok/CondaRO-linux/korangar")
#: The task named /home/phantom/rathena, but only these rAthena 2019 checkouts exist
#: on this machine (they are byte-identical). Documented in summary.md.
RATHENA = Path("/home/phantom/popeng-llm-cpp/condaro-llm")
GAME_DIR = Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game")
NOTES = ROOT / "notes" / "skill-audit"

#: The official EffectTable.js dump (identical copy also under
#: CondaRO-linux/notes/korangar-windows-client-data/).
EFFECT_TABLE_JS = Path("/tmp/EffectTable.js")
EFFECT_TABLE_JS_FALLBACK = Path("/home/phantom/Games/ragnarok/CondaRO-linux/notes/korangar-windows-client-data/roBrowserLegacy_EffectTable.js")

#: Official SkillEffect field -> (client effect variant, spawn anchor), verbatim
#: from tools/gen_skill_effect_assets.py (FIELD_SPECS).
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

#: Official `SkillEffect` field order of the generator (fallback priority).
FIELD_ORDER = list(FIELD_SPECS)

#: The 8 skills whose data-driven layers are skipped by
#: `has_custom_completion` (korangar/src/world/skill_cast_effect.rs).
CUSTOM_COMPLETION_SKILLS = (27, 29, 33, 34, 45, 110, 270, 401)

#: Hand-written visuals from `skill_visuals` (skill_cast_effect.rs:277-384) and
#: the status aura registry (status_aura.rs:130-135). `None` = no completion.
CUSTOM_VISUALS = {
    27: {"completion": None, "note": "Warp Portal: cast circle only; the portal is a skill unit"},
    29: {"completion": "efst_c_blessings_of_soul\\blessingsofsoul.str", "overhead": True, "scale": 0.9},
    33: {"completion": "angelus.str", "overhead": True, "scale": 1.0},
    34: {"completion": "efst_angel_fluttering\\angel_fluttering.str", "overhead": True, "scale": 1.0},
    45: {"completion": "concentration.str", "overhead": True, "scale": 0.9},
    110: {"completion": "black_hammerfall.str", "overhead": False, "scale": 1.2},
    270: {"completion": None, "note": "Fury: procedural BEGIN_SPELL ring + 4 expanding rings"},
    401: {"completion": None, "note": "Zen: procedural BEGIN_SPELL ring + body flash"},
    28: {"completion": None, "note": "Heal: procedural target cylinders in addition to data rows"},
}

#: `status_aura_spec` registry (status_aura.rs) - EFST id -> status name.
STATUS_AURAS = {110: "Assumptio", 473: "Assumptio"}

#: Job tree name -> (class family, tier). Pre-re job families for the summary.
JOB_FAMILY = {}


def _init_job_family() -> dict[str, tuple[str, str]]:
    fam = {}
    base = {
        "Novice": "Novice", "Swordman": "Swordman", "Mage": "Mage", "Archer": "Archer",
        "Acolyte": "Acolyte", "Merchant": "Merchant", "Thief": "Thief",
    }
    for job, family in base.items():
        fam[job] = (family, "1st")
        fam[f"{job}_High"] = (family, "transcendent")
        fam[f"Baby_{job}"] = (family, "baby")
    second = {
        "Knight": "Swordman", "Knight2": "Swordman", "Crusader": "Swordman", "Crusader2": "Swordman",
        "Wizard": "Mage", "Sage": "Mage", "Hunter": "Archer", "Bard": "Archer", "Dancer": "Archer",
        "Priest": "Acolyte", "Monk": "Acolyte", "Blacksmith": "Merchant", "Alchemist": "Merchant",
        "Assassin": "Thief", "Rogue": "Thief",
        "Lord_Knight": "Swordman", "Lord_Knight2": "Swordman", "Paladin": "Swordman", "Paladin2": "Swordman",
        "High_Wizard": "Mage", "Professor": "Mage", "Sniper": "Archer", "Clown": "Archer", "Gypsy": "Archer",
        "High_Priest": "Acolyte", "Champion": "Acolyte", "Whitesmith": "Merchant", "Creator": "Merchant",
        "Assassin_Cross": "Thief", "Stalker": "Thief",
    }
    for job, family in second.items():
        tier = "transcendent" if job in ("Lord_Knight", "Lord_Knight2", "Paladin", "Paladin2", "High_Wizard",
                                         "Professor", "Sniper", "Clown", "Gypsy", "High_Priest", "Champion",
                                         "Whitesmith", "Creator", "Assassin_Cross", "Stalker") else "2nd"
        fam[job] = (family, tier)
        fam[f"Baby_{job}"] = (family, "baby")
    for job, family in (("Supernovice", "Novice"), ("Super_Baby", "Novice"), ("Baby", "Novice")):
        fam[job] = (family, "special")
    for job in ("Gunslinger", "Ninja", "Taekwon", "Star_Gladiator", "Star_Gladiator2", "Soul_Linker",
                "Gangsi", "Death_Knight", "Dark_Collector"):
        fam[job] = ("Extended", "extended")
    return fam


JOB_FAMILY = _init_job_family()


def scalar(value):
    """Normalize a skill_db scalar-or-per-level field to its first value."""
    if isinstance(value, list) and value:
        first = value[0]
        if isinstance(first, dict):
            return first.get("Time", first.get("Element", first.get("Area", first)))
        return first
    return value


def load_rathena():
    skill_db = yaml.safe_load((RATHENA / "db/pre-re/skill_db.yml").read_text())["Body"]
    tree_db = yaml.safe_load((RATHENA / "db/pre-re/skill_tree.yml").read_text())["Body"]
    status_db = yaml.safe_load((RATHENA / "db/pre-re/status.yml").read_text())["Body"]

    by_name = {}
    for skill in skill_db:
        by_name[skill["Name"]] = skill

    status_icon = {entry["Status"].lower(): entry.get("Icon") for entry in status_db if entry.get("Status")}

    learnable = {}          # skill id -> set(job names)
    job_of = {}
    for job_entry in tree_db:
        job = job_entry["Job"]
        for entry in job_entry.get("Tree") or []:
            if entry.get("MaxLevel", 1) == 0:
                continue
            skill = by_name[entry["Name"]]
            learnable.setdefault(skill["Id"], set()).add(job)
            job_of.setdefault(job, []).append(skill["Id"])

    skills = {}
    for skill_id, jobs in learnable.items():
        skill = next(s for s in skill_db if s["Id"] == skill_id)
        families = sorted({JOB_FAMILY.get(job, ("Unknown", "?"))[0] for job in jobs})
        skills[skill_id] = {
            "id": skill_id,
            "aegis": skill["Name"],
            "name": skill.get("Description", skill["Name"]),
            "max_level": skill.get("MaxLevel", 1),
            "type": skill.get("Type", "None"),
            "target_type": skill.get("TargetType", "Passive"),
            "element": scalar(skill.get("Element", "Neutral")),
            "cast_time_ms": scalar(skill.get("CastTime", 0)),
            "after_cast_act_delay_ms": scalar(skill.get("AfterCastActDelay", 0)),
            "duration_ms": scalar(skill.get("Duration1", 0)),
            "cooldown_ms": scalar(skill.get("Cooldown", 0)),
            "status": skill.get("Status"),
            "status_icon": status_icon.get((skill.get("Status") or "").lower()),
            "jobs": sorted(jobs),
            "families": families,
        }
    return skills, status_icon


# ---------------------------------------------------------------------------
# Official EffectTable.js parsing (all fields, not just file/type).
# ---------------------------------------------------------------------------

NUMERIC_KEYS = (
    "duration", "delayOffset", "delayLate", "delayFrame", "frame", "yOffset",
    "alphaMax", "alphaMin", "zIndex", "duplicate", "timeBetweenDupli", "height",
    "bottomSize", "topSize", "size", "sizeStart", "sizeEnd", "sizeX", "sizeY",
    "sizeRand", "sizeRandY", "sizeRandYMiddle", "posx", "posy", "posz", "poszStart",
    "poszEnd", "poszStartRand", "poszEndRand", "posxRand", "posyRand", "rotateSpeed",
    "animation", "blendMode", "red", "green", "blue", "angle", "sparkNumber",
)
BOOLEAN_KEYS = (
    "head", "attachedEntity", "stopAtEnd", "direction", "repeat", "stackable",
    "overlay", "fadeIn", "fadeOut", "fade", "rotate", "sparkling", "absoluteSpriteName",
    "sizeSmooth", "playSprite", "prim",
)


def strip_js_comments(text: str) -> str:
    """Remove // and /* */ comments; the table has no comment markers in strings."""
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    text = re.sub(r"//[^\n]*", "", text)
    return text


def parse_effect_table(path: Path) -> dict[int, list[dict]]:
    text = strip_js_comments(path.read_text(encoding="utf-8", errors="replace"))
    # Top-level keys are either numeric effect ids or symbolic `ef_*` names used
    # by the 4th-job effect folders; the symbolic blocks must not be merged into
    # the preceding numeric effect.
    blocks = list(re.finditer(r"\n\t(?:(\d+)|([A-Za-z_]\w*)): \[", text))
    result: dict[int, list[dict]] = {}
    for index, match in enumerate(blocks):
        if match.group(1) is None:
            continue
        effect_id = int(match.group(1))
        start = match.end()
        end = blocks[index + 1].start() if index + 1 < len(blocks) else len(text)
        block = text[start:end]
        # Entries are `\n\t\t{ ... \n\t\t},`; split on the two-tab closing brace.
        chunks = re.split(r"\n\t\t\}", block)
        entries = []
        for chunk in chunks:
            if "{" not in chunk:
                continue
            # Drop the leading whitespace up to the entry's opening brace.
            chunk = chunk[chunk.index("{") + 1:]
            entry: dict = {}
            type_match = re.search(r"\btype:\s*'([^']+)'", chunk)
            if type_match:
                entry["type"] = type_match.group(1)
            for key in ("file", "textureName", "spriteName", "wav", "min", "func"):
                value_match = re.search(rf"\b{key}:\s*'((?:[^'\\]|\\.)*)'", chunk)
                if value_match:
                    entry[key] = value_match.group(1)
            func_call = re.search(r"\bfunc:\s*function\s*\([^)]*\)\s*\{(.*?)\}", chunk, re.S)
            if func_call:
                calls = re.findall(r"(\w+(?:\.\w+)*)\s*\(", func_call.group(1))
                entry["func"] = calls or ["<anonymous>"]
            for key in NUMERIC_KEYS:
                number = re.search(rf"\b{key}:\s*(-?\d+(?:\.\d+)?)", chunk)
                if number is not None:
                    value = float(number.group(1))
                    entry[key] = int(value) if value.is_integer() else value
            for key in BOOLEAN_KEYS:
                flag = re.search(rf"\b{key}:\s*(true|false)", chunk)
                if flag is not None:
                    entry[key] = flag.group(1) == "true"
            rand = re.search(r"\brand:\s*\[\s*(-?\d+)\s*,\s*(-?\d+)\s*\]", chunk)
            if rand:
                entry["rand"] = [int(rand.group(1)), int(rand.group(2))]
            if entry:
                entries.append(entry)
        if entries:
            result[effect_id] = entries
    return result


def decode_game_name(file_name: str) -> str:
    """Decode EUC-KR bytes stored as `\\xNN` escapes in the dumped JS."""
    try:
        raw = re.sub(r"\\x([0-9a-fA-F]{2})", lambda m: chr(int(m.group(1), 16)), file_name).encode("latin-1")
    except (UnicodeEncodeError, ValueError):
        return file_name
    for encoding in ("euc-kr", "utf-8"):
        try:
            return raw.decode(encoding)
        except UnicodeDecodeError:
            continue
    return file_name


def load_official():
    skill_effects = json.loads((ROOT / "tools/windows_client_skill_effects.json").read_text())
    sounds = json.loads((ROOT / "tools/windows_client_skill_sounds.json").read_text())
    sprite_effects = json.loads((ROOT / "tools/windows_client_sprite_effects.json").read_text())
    effect_table = parse_effect_table(EFFECT_TABLE_JS if EFFECT_TABLE_JS.is_file() else EFFECT_TABLE_JS_FALLBACK)
    return skill_effects, sounds, sprite_effects, effect_table


# ---------------------------------------------------------------------------
# GRF asset index (read-only).
# ---------------------------------------------------------------------------

GRF_HEADER_SIZE = 46
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
SPRITE_ROOT = "data\\sprite\\"
TEXTURE_ROOT = "data\\texture\\"
EFFECT_ROOT = SPRITE_ROOT + "이팩트"


def read_grf_names(path: Path) -> list[str]:
    with path.open("rb") as handle:
        header = handle.read(GRF_HEADER_SIZE)
        if not header.startswith(b"Master of Magic"):
            raise SystemExit(f"not a GRF file: {path}")
        table_offset, reserved, file_count, version = struct.unpack("<IIII", header[30:46])
        if version != 0x200:
            raise SystemExit(f"unsupported GRF version {version:#x}: {path}")
        handle.seek(GRF_HEADER_SIZE + table_offset)
        compressed_size, _uncompressed = struct.unpack("<II", handle.read(8))
        table = zlib.decompress(handle.read(compressed_size))
    names = []
    offset = 0
    for _ in range(file_count - reserved - 7):
        end = table.index(b"\0", offset)
        raw = table[offset:end]
        offset = end + 1 + 17
        try:
            names.append(raw.decode("utf-8"))
        except UnicodeDecodeError:
            try:
                names.append(raw.decode("euc-kr"))
            except UnicodeDecodeError:
                names.append(raw.decode("latin-1"))
    return names


class AssetIndex:
    def __init__(self):
        self.names: set[str] = set()          # lower-case full paths
        self.by_ext: dict[str, dict[str, str]] = {}
        for archive in ARCHIVES:
            path = GAME_DIR / archive
            if not path.is_file():
                continue
            for name in read_grf_names(path):
                lower = name.lower()
                self.names.add(lower)
                ext = lower.rsplit(".", 1)[-1] if "." in lower else ""
                self.by_ext.setdefault(ext, {})[lower] = name

    def exists(self, path: str) -> bool:
        return path.replace("/", "\\").lower() in self.names

    def find_stem(self, rel_dir: str, stem: str, ext: str) -> str | None:
        for candidate in (f"{rel_dir}\\{stem}.{ext}", f"{stem}.{ext}"):
            key = candidate.replace("/", "\\").lower()
            if key in self.by_ext.get(ext, {}):
                return self.by_ext[ext][key]
        return None

    def template_stem(self, rel_dir: str, pattern: str, ext: str) -> str | None:
        regex = re.compile("^" + re.escape(pattern).replace(r"%d", r"\d+").replace(r"%s", r".+") + f"\\.{ext}$", re.I)
        for key, real in self.by_ext.get(ext, {}).items():
            if regex.match(key):
                return real
        return None


# ---------------------------------------------------------------------------
# Current client mapping (RON files).
# ---------------------------------------------------------------------------

def parse_ron_rows(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    rows = []
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("(skill:"):
            rows.append(stripped)
    return rows


def ron_unescape(value: str) -> str:
    """Undo the RON string escapes used in the data files (`\\\\` -> `\\`)."""
    return value.replace("\\\\", "\\")


def parse_key_values(row: str) -> dict:
    """Parse a flat RON row `(key: value, ...)` into Python values."""
    result: dict = {}
    depth = 0
    current = ""
    parts = []
    in_string = False
    row = row.strip().rstrip(",")
    if row.endswith(")"):
        row = row[:-1]
    for char in row[1:] if row.startswith("(") else row:
        if char == '"' and (not current.endswith("\\")):
            in_string = not in_string
        if not in_string:
            if char in "([":
                depth += 1
            elif char in ")]":
                depth -= 1
            elif char == "," and depth == 0:
                parts.append(current)
                current = ""
                continue
        current += char
    if current.strip():
        parts.append(current)
    for part in parts:
        if ":" not in part:
            continue
        key, value = part.split(":", 1)
        key = key.strip()
        value = value.strip()
        if value.startswith('"') and value.endswith('"'):
            result[key] = ron_unescape(value[1:-1])
        elif value.startswith("[") and value.endswith("]"):
            result[key] = [ron_unescape(v.strip().strip('"')) for v in value[1:-1].split(",") if v.strip()]
        elif value.startswith("(") and value.endswith(")"):
            result[key] = tuple(float(v) for v in value[1:-1].split(",") if v.strip())
        elif value in ("true", "false"):
            result[key] = value == "true"
        else:
            try:
                result[key] = int(value)
            except ValueError:
                try:
                    result[key] = float(value)
                except ValueError:
                    result[key] = value
    return result


def load_client():
    client = {"str": [], "spr": [], "proc": [], "sounds": [], "actions": []}
    for line in parse_ron_rows(ROOT / "korangar/archive/data/client/skill_effect_assets.ron"):
        client["str"].append(parse_key_values(line))
    for line in parse_ron_rows(ROOT / "korangar/archive/data/client/skill_sprite_effects.ron"):
        client["spr"].append(parse_key_values(line))
    for line in parse_ron_rows(ROOT / "korangar/archive/data/client/skill_procedural_effects.ron"):
        client["proc"].append(parse_key_values(line))
    for line in parse_ron_rows(ROOT / "korangar/archive/data/client/skill_sounds.ron"):
        client["sounds"].append(parse_key_values(line))
    for line in parse_ron_rows(ROOT / "korangar/archive/data/client/skill_actions.ron"):
        client["actions"].append(parse_key_values(line))
    return client


# ---------------------------------------------------------------------------
# Layer extraction and coverage classification.
# ---------------------------------------------------------------------------

TEXTURE_KINDS = {"2D", "3D", "CYLINDER", "QuadHorn", "TRAIL", "WATERFALL", "AREA_SPAWN"}
PRIMITIVE_KINDS = {"PRIM"}
NON_VISUAL_KINDS = {"FUNC", "RSM"}


def _texture_candidates(file_name: str) -> list[str]:
    """Candidate texture paths (relative to data\\texture\\) for a table value."""
    name = decode_game_name(file_name).replace("/", "\\")
    candidates = []
    if "." in name.rsplit("\\", 1)[-1]:
        candidates.append(name)
    else:
        for ext in ("tga", "bmp"):
            candidates.append(f"{name}.{ext}")
    # The table sometimes prefixes `effect/`.
    for candidate in list(candidates):
        if candidate.lower().startswith("effect\\"):
            candidates.append(candidate[len("effect\\"):])
        else:
            candidates.append(f"effect\\{candidate}")
    return candidates


def resolve_texture(file_name: str, index: AssetIndex) -> str | None:
    for candidate in _texture_candidates(file_name):
        path = f"data\\texture\\{candidate}"
        if index.exists(path):
            return index.by_ext.get(path.rsplit(".", 1)[-1].lower(), {}).get(path.lower(), path)
    return None


def resolve_str(file_name: str, index: AssetIndex) -> tuple[str | None, bool]:
    """Resolve an official STR `file:` value. Returns (path|None, is_template)."""
    name = decode_game_name(file_name).replace("/", "\\")
    if name.lower().endswith(".str"):
        name = name[:-4]
    if "%" in name:
        pattern = f"{name}.str"
        regex = re.compile("^data\\\\texture\\\\effect\\\\" + re.escape(pattern).replace(r"%d", r"\d+").replace(r"%s", r".+") + "$", re.I)
        for key in index.by_ext.get("str", {}):
            if regex.match(key):
                return index.by_ext["str"][key], True
        return None, True
    path = f"data\\texture\\effect\\{name}.str"
    if index.exists(path):
        return index.by_ext["str"][path.lower()], False
    return None, False


def resolve_spr(file_name: str, index: AssetIndex) -> str | None:
    stem = decode_game_name(file_name).replace("/", "\\").strip().lstrip("\\")
    if stem.lower().endswith((".spr", ".act")):
        stem = stem.rsplit(".", 1)[0]
    for candidate in (f"{EFFECT_ROOT}\\{stem}", stem):
        spr = index.find_stem(candidate.rsplit("\\", 1)[0], candidate.rsplit("\\", 1)[-1], "spr")
        act = index.find_stem(candidate.rsplit("\\", 1)[0], candidate.rsplit("\\", 1)[-1], "act")
        if spr and act:
            return spr
    return None


def resolve_wav(wav: str, index: AssetIndex) -> tuple[str | None, bool]:
    """Resolve an official EffectTable `wav` value. Returns (path|None, template)."""
    name = decode_game_name(wav).replace("/", "\\")
    if name.lower().endswith(".wav"):
        name = name[:-4]
    if "%" in name:
        regex = re.compile("^data\\\\wav\\\\" + re.escape(name).replace(r"%d", r"\d+").replace(r"%s", r".+") + r"\.wav$", re.I)
        for key in index.by_ext.get("wav", {}):
            if regex.match(key):
                return index.by_ext["wav"][key], True
        return None, True
    path = f"data\\wav\\{name}.wav"
    if index.exists(path):
        return index.by_ext["wav"][path.lower()], False
    return None, False


def resolve_particle_spr(name: str, index: AssetIndex) -> str | None:
    for candidate in (f"{EFFECT_ROOT}\\{name}", name):
        spr = index.find_stem(candidate.rsplit("\\", 1)[0], candidate.rsplit("\\", 1)[-1], "spr")
        if spr:
            return spr
    return None


def classify_layer(entry: dict, effect_id: int, layer_index: int, index: AssetIndex) -> dict:
    """Turn one official EffectTable entry into a normalised layer record."""
    raw_type = entry.get("type")
    wav = entry.get("wav")
    layer = {
        "index": layer_index,
        "effect_id": effect_id,
        "type_raw": raw_type,
        "file": None,
        "kind": None,
        "asset": None,
        "asset_exists": None,
        "wav": wav,
        "wav_asset": None,
        "wav_asset_exists": None,
        "head": entry.get("head", False),
        "attached_entity": entry.get("attachedEntity"),
        "duration_ms": entry.get("duration"),
        "delay_ms": entry.get("delayOffset", entry.get("delayLate", entry.get("delayFrame"))),
        "y_offset": entry.get("yOffset"),
        "duplicate": entry.get("duplicate"),
        "func": entry.get("func"),
    }

    if wav:
        wav_asset, wav_template = resolve_wav(wav, index)
        layer["wav_asset"] = wav_asset or f"data\\wav\\{decode_game_name(wav).replace('/', '\\')}.wav"
        layer["wav_asset_exists"] = wav_asset is not None
        layer["wav_template"] = wav_template

    if raw_type == "STR":
        layer["kind"] = "STR"
        if entry.get("file"):
            layer["file"] = entry["file"]
            layer["asset"], layer["template"] = resolve_str(entry["file"], index)
            layer["asset_exists"] = layer["asset"] is not None
    elif raw_type == "SPR":
        layer["kind"] = "SPR"
        if entry.get("file"):
            layer["file"] = decode_game_name(entry["file"])
            layer["asset"] = resolve_spr(entry["file"], index)
            layer["asset_exists"] = layer["asset"] is not None
    elif raw_type in TEXTURE_KINDS:
        layer["kind"] = "TEXTURE"
        if entry.get("file"):
            layer["file"] = entry["file"]
            layer["asset"] = resolve_texture(entry["file"], index)
        elif entry.get("textureName"):
            layer["file"] = entry["textureName"]
            layer["asset"] = resolve_texture(entry["textureName"], index)
        if layer["asset"] is None and entry.get("spriteName"):
            layer["file"] = entry["spriteName"]
            layer["asset"] = resolve_particle_spr(entry["spriteName"], index)
        if layer["asset"] is None and not entry.get("file") and not entry.get("textureName") and not entry.get("spriteName"):
            layer["kind"] = "PRIM"
        layer["asset_exists"] = layer["asset"] is not None
    elif raw_type == "FUNC" or raw_type == "RSM" or raw_type == "PRIM":
        layer["kind"] = "FUNC"
        layer["asset_exists"] = True
    elif raw_type is None:
        # Entries without a type are additive facts (extra wav play) or a bare
        # parameter block; they carry no renderable layer of their own.
        if entry.get("file"):
            layer["kind"] = "TEXTURE"
            layer["file"] = entry["file"]
            layer["asset"] = resolve_texture(entry["file"], index)
            layer["asset_exists"] = layer["asset"] is not None
        else:
            layer["kind"] = "SOUND" if wav else "OTHER"
    else:
        layer["kind"] = "OTHER"

    if layer.get("file"):
        layer["file_decoded"] = decode_game_name(layer["file"])
    if layer["kind"] in ("STR", "SPR", "TEXTURE"):
        layer["visual"] = True
    else:
        layer["visual"] = False
    return layer


def official_layers_for_skill(skill_id: int, skill_effects: dict, effect_table: dict, index: AssetIndex,
                              procedural_facts: dict | None = None) -> list[dict]:
    fields = skill_effects["skills"].get(str(skill_id), [])
    layers = []
    for field in fields:
        spec = FIELD_SPECS.get(field["field"])
        if spec is None:
            continue
        variant, anchor = spec
        for effect_id in field["effects"]:
            entries = effect_table.get(effect_id)
            source = "EffectTable.js"
            if not entries and procedural_facts:
                entries = procedural_facts.get(str(effect_id))
                source = "windows_client_procedural_effects.json"
            if not entries:
                layers.append({
                    "index": len(layers), "effect_id": effect_id, "field": field["field"],
                    "variant": variant, "anchor": anchor, "kind": "HARDCODED", "visual": False,
                    "asset": None, "asset_exists": None, "source": "no EffectTable entry",
                })
                continue
            for entry in entries:
                layer = classify_layer(entry, effect_id, len(layers), index)
                layer.update({"field": field["field"], "variant": variant, "anchor": anchor, "source": source})
                layers.append(layer)
    return layers


def official_sounds_for_skill(skill_id: int, sounds: dict) -> list[dict]:
    return sounds["skills"].get(str(skill_id), [])


def custom_visual_asset(completion: str | None, index: AssetIndex) -> dict | None:
    if not completion:
        return None
    path = f"data\\texture\\effect\\{completion}"
    return {"path": path.replace("/", "\\"), "exists": index.exists(path)}


def current_assets_for_skill(skill_id: int, client: dict, index: AssetIndex) -> dict:
    """Asset paths the current client would spawn for this skill (data rows)."""
    str_assets = []
    for row in client["str"]:
        if row.get("skill") != skill_id:
            continue
        path = f"data\\texture\\effect\\{row['file']}"
        str_assets.append({
            "asset": path.replace("/", "\\"), "variant": row.get("variant"), "anchor": row.get("anchor"),
            "overhead": row.get("overhead", False), "exists": index.exists(path), "source": "skill_effect_assets.ron",
        })
    spr_assets = []
    for row in client["spr"]:
        if row.get("skill") != skill_id:
            continue
        for stem in row.get("files", []):
            path = f"data\\sprite\\{stem}.spr"
            spr_assets.append({
                "asset": path.replace("/", "\\"), "variant": row.get("variant"), "anchor": row.get("anchor"),
                "exists": index.exists(path), "source": "skill_sprite_effects.ron",
            })
    proc_assets = []
    for row in client["proc"]:
        if row.get("skill") != skill_id:
            continue
        texture = row.get("texture", "")
        path = f"data\\texture\\{texture}"
        proc_assets.append({
            "asset": path.replace("/", "\\"), "variant": row.get("variant"), "anchor": row.get("anchor"),
            "primitive": row.get("primitive"), "exists": index.exists(path), "source": "skill_procedural_effects.ron",
        })
    return {"str": str_assets, "spr": spr_assets, "proc": proc_assets}


def asset_matches(official_asset: str | None, current_asset: str, template: bool = False) -> bool:
    if official_asset is None:
        return False
    if template:
        regex = re.compile("^" + re.escape(official_asset).replace(r"%d", r"\d+").replace(r"%s", r".+") + "$", re.I)
        return regex.match(current_asset.replace("/", "\\")) is not None
    return official_asset.replace("/", "\\").lower() == current_asset.replace("/", "\\").lower()


RAW_SKILL_EFFECT_JS = Path("/home/phantom/Games/ragnarok/CondaRO-linux/notes/korangar-windows-client-data/roBrowserLegacy_SkillEffect.js")


def parse_raw_skill_effect(path: Path) -> dict[str, dict[str, list[int]]]:
    """Parse the raw `SkillEffect[SK.X] = {...}` table (Alias names -> field -> ids).

    The extracted `tools/windows_client_skill_effects.json` intentionally drops
    some facts (`firstHitEffectId`, and effect id 0 = standard hit flash). This
    parser keeps them so the audit can flag extractor gaps.
    """
    text = re.sub(r"//[^\n]*", "", path.read_text(encoding="utf-8", errors="replace"))
    raw: dict[str, str] = {}
    for match in re.finditer(r"SkillEffect\[SK\.([A-Z0-9_]+)\]\s*=\s*(\{(?:[^{}]|\{[^{}]*\})*\}|SkillEffect\[SK\.[A-Z0-9_]+\])\s*;", text, re.S):
        raw[match.group(1)] = match.group(2)

    result: dict[str, dict[str, list[int]]] = {}
    for name, value in raw.items():
        depth = 0
        while depth < 8:
            alias = re.fullmatch(r"SkillEffect\[SK\.([A-Z0-9_]+)\]", value.strip())
            if alias is None:
                break
            value = raw.get(alias.group(1), "{}")
            depth += 1
        fields: dict[str, list[int]] = {}
        for field_match in re.finditer(r"\b([A-Za-z]*[Ee]ffectId[A-Za-z]*)\s*:\s*(\[[^\]]*\]|\d+)", value):
            ids = [int(number) for number in re.findall(r"\b(\d{1,4})\b", field_match.group(2))]
            if ids:
                fields[field_match.group(1)] = ids
        if fields:
            result[name] = fields
    return result
