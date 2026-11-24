#!/usr/bin/env python3
"""Generate the official multi-layer skill effect rows for the unified pipeline.

This is the **official-table lane** of the unified skill effect pipeline: it
emits every layer of every effect-table entry that a pre-renewal skill uses, in
the order of the official table, validated against the shipped CondaRO GRFs.
The hand-written/migrated visuals, the final merged data file and the Rust
loader are owned by the pipeline lane (`tools/gen_skill_effects.py`); this
generator only produces the fragment it consumes:

    tools/skill_effects_official.ron

plus the machine-readable facts + coverage report:

    tools/skill_effects_official.json
    notes/skill-effects-official-data.md

Facts used (all committed under `tools/`, no GPL code is vendored):

* `windows_client_skill_effects.json` - skill -> official `SkillEffect` field ->
  effect ids (the exact field decides when/where the layers play),
* `windows_client_effect_layers.json` - effect id -> complete ordered layer
  list with `type`, `file`, `wav`, `head`, `attachedEntity`, duration/delay and
  sizes/colours (`extract_windows_client_effect_layers.py`),
* `windows_client_skill_actions.json` - the official caster action per skill,
* `windows_client_skill_sounds.json` - the official per-skill sounds,
* `windows_client_effect_whitelist.txt` / `windows_client_spr_whitelist.txt` -
  the `.str`/`.spr` effect files referenced by `CondaRO.exe`,
* `rathena_pre_re_learnable_skills.json` - every learnable pre-re skill and the
  classes that learn it (for coverage),
* the real GRF archives in the client's `game/` folder (asset validation).

Layer kinds emitted (schema v1 of `skill_effects.ron`, owned by the pipeline
lane): `Str` (`.str` screen sprite), `Spr` (`.spr` + `.act` actor sprite),
`Texture` (`.tga`/`.bmp`/... billboard/screen texture) and `Cylinder` (texture
ground aura). `FUNC`/`3D` hard-coded callbacks and other non-file layer types
are recorded as unsupported (the procedural lane already covers the primitives).

Rules:

* Every official layer is preserved, including deliberate duplicates (Increase
  AGI effect 37 has three `effect/ac_center2.tga` bursts with different
  duplicate/delay/alpha values plus `effect/agi_up.bmp`).
* Overhead is driven by the official `head` flag, never by a hand-coded skill
  set. `attachedEntity` is kept as a fact; the anchor hint comes from the
  official `SkillEffect` field.
* `wav` is resolved relative to `data\\wav\\` and emitted on its layer.
* Every referenced asset is validated: `.str` exists (and is whitelisted),
  `.spr` **and** `.act` exist, textures exist under `data\\texture\\`, wavs
  exist under `data\\wav\\`. Missing assets are dropped and reported, never
  invented.
* Skills with no official visual (passive/mastery skills) are reported as
  explicit `no_effect` entries in the JSON/report; the RON fragment only holds
  rows that actually spawn layers.

Usage:
    ./tools/gen_skill_effect_layers.py                 # regenerate fragment + JSON + report
    ./tools/gen_skill_effect_layers.py --check         # fail if any output is out of date
    ./tools/gen_skill_effect_layers.py --report        # print the coverage/gap report
    ./tools/gen_skill_effect_layers.py --game-dir /path/to/client/game
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
import zlib
from collections import Counter, defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

DEFAULT_GAME_DIR = "/home/phantom/Games/ragnarok/CondaRO-linux/client/game"
DEFAULT_EFFECTS = REPO / "tools/windows_client_skill_effects.json"
DEFAULT_LAYERS = REPO / "tools/windows_client_effect_layers.json"
DEFAULT_ACTIONS = REPO / "tools/windows_client_skill_actions.json"
DEFAULT_SOUNDS = REPO / "tools/windows_client_skill_sounds.json"
DEFAULT_STR_WHITELIST = REPO / "tools/windows_client_effect_whitelist.txt"
DEFAULT_SPR_WHITELIST = REPO / "tools/windows_client_spr_whitelist.txt"
DEFAULT_LEARNABLE = REPO / "tools/rathena_pre_re_learnable_skills.json"
DEFAULT_OUTPUT = REPO / "tools/skill_effects_official.ron"
DEFAULT_GRF2019 = REPO / "tools/grf2019_skill_facts.json"
DEFAULT_JSON = REPO / "tools/skill_effects_official.json"
DEFAULT_REPORT = REPO / "notes/skill-effects-official-data.md"

ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
GRF_HEADER_SIZE = 46

STR_PREFIX = "data\\texture\\effect\\"
SPRITE_PREFIX = "data\\sprite\\"
TEXTURE_PREFIX = "data\\texture\\"
WAV_PREFIX = "data\\wav\\"
EFFECT_SPRITE_ROOT = "이팩트"

#: Official `SkillEffect` field -> (client phase, anchor hint).
#:
#: The phase is when the layer plays; the anchor hint is where the official
#: field says it belongs. It is refined with the layer's `attachedEntity` /
#: `head` facts (never with a hand-coded skill list).
FIELD_SPECS = {
    "groundEffectId": ("Ground", "Ground"),
    "groundEffectIdByLevel": ("Ground", "Ground"),
    "beginCastEffectId": ("Cast", "Caster"),
    "beginCastEffectIdByLevel": ("Cast", "Caster"),
    "castAuraEffectId": ("Cast", "Caster"),
    "effectId": ("Success", "Auto"),
    "successEffectId": ("Success", "Auto"),
    "effectIdOnCaster": ("Success", "Caster"),
    "successEffectIdOnCaster": ("Success", "Caster"),
    "successEffectIdOnCasterByLevel": ("Success", "Caster"),
    "successEffectIdOnCasterByScreenDirection": ("Success", "Caster"),
    "successEffectIdOnCasterAimed": ("Success", "Caster"),
    "successEffectIdOnCasterAimed": ("Success", "Caster"),
    "successEffectIdOnTargetByLevel": ("Success", "Target"),
    "successEffectIdOnTargetByScreenSide": ("Success", "Target"),
    "releaseEffectId": ("Release", "Target"),
    "beforeHitEffectId": ("Release", "Target"),
    "beforeHitEffectIdOnCaster": ("Release", "Caster"),
    "hitEffectId": ("Hit", "Target"),
    "firstHitEffectId": ("Hit", "Target"),
    "hitEffectIdOnCaster": ("Hit", "Caster"),
    "damageEffectIdOnTarget": ("Hit", "Target"),
    "damageEffectIdOnCaster": ("Caster", "Caster"),
}

PHASE_ORDER = ["Ground", "Cast", "Success", "Release", "Hit", "Caster"]

#: World units per official effect unit (5 effect units per map cell, 10 world
#: units per cell - the same conversion the client's hand-written effect specs
#: and `gen_skill_procedural_effects.py` use).
OFFICIAL_EFFECT_UNIT = 2.0
#: Screen pixels to world units for the `yOffset` of a sprite effect.
PIXEL_TO_WORLD = 0.25
#: roBrowser effect size -> world units for billboard (hit flash) textures; the
#: same factor the migrated procedural rows use, so the visual magnitude matches
#: the existing client data (one map cell is 10 world units).
TEXTURE_SIZE_SCALE = 0.85
#: roBrowser effect size -> world units for projectile textures.
PROJECTILE_SIZE_SCALE = 0.09

#: The client spawns texture billboards as `HitFlash` bursts; the migrated rows
#: clamp them to a sane world size so a raw table value can never fill the map.
TEXTURE_SIZE_MIN = 4.0
TEXTURE_SIZE_MAX = 22.0
TEXTURE_SIZE_END_MIN = 2.0
TEXTURE_SIZE_END_MAX = 32.0

#: `blendMode` values that map onto the pipeline's Blend enum: 2 = additive
#: light (`ONE`), 8 = the default alpha blend (`ONE_MINUS_SRC_ALPHA`).
BLEND_MODES = {2: "Additive", 8: "Alpha"}

#: Official layer types that this lane cannot express as a file layer. `FUNC`
#: is a hard-coded callback (covered by the procedural lane), the rest are
#: client model/weather/primitive types.
UNSUPPORTED_TYPES = {"FUNC", "RSM", "QuadHorn", "WATERFALL", "PRIM", "AREA_SPAWN", "TRAIL"}

#: Effect-table keys that are alternative/derived and must not be emitted as
#: layers: `min` is the `/mineffect` (minified) `.str` variant.
IGNORED_KEYS = {"min"}


class Dropped(Exception):
    def __init__(self, reason: str, detail: str):
        super().__init__(detail)
        self.reason = reason
        self.detail = detail


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


class Assets:
    """Validated archive file maps (lower-case relative path -> real spelling)."""

    def __init__(self) -> None:
        self.str_files: dict[str, str] = {}
        self.sprites: dict[str, str] = {}
        self.actions: dict[str, str] = {}
        self.textures: dict[str, str] = {}
        self.wavs: dict[str, str] = {}

    @classmethod
    def collect(cls, game_dir: Path) -> "Assets":
        assets = cls()
        for archive in ARCHIVES:
            path = game_dir / archive
            if not path.is_file():
                continue
            for name in read_grf_names(path):
                lower = name.lower()
                if lower.startswith(STR_PREFIX) and lower.endswith(".str"):
                    assets.str_files.setdefault(name[len(STR_PREFIX) :].lower(), name[len(STR_PREFIX) :])
                elif lower.startswith(SPRITE_PREFIX):
                    relative = name[len(SPRITE_PREFIX) :]
                    if lower.endswith(".spr"):
                        assets.sprites.setdefault(relative.lower(), relative)
                    elif lower.endswith(".act"):
                        assets.actions.setdefault(relative.lower(), relative)
                elif lower.startswith(WAV_PREFIX) and lower.endswith(".wav"):
                    assets.wavs.setdefault(name[len(WAV_PREFIX) :].lower(), name[len(WAV_PREFIX) :])
                if lower.startswith(TEXTURE_PREFIX) and lower.endswith((".tga", ".bmp", ".png", ".jpg")):
                    assets.textures.setdefault(name[len(TEXTURE_PREFIX) :].lower(), name[len(TEXTURE_PREFIX) :])
        return assets


# ------------------------------------------------------------------ whitelists
def normalize(text: str) -> str:
    return re.sub(r"[^a-z0-9]", "", text.lower())


def load_str_whitelist(path: Path) -> tuple[set[str], list[str]]:
    concrete: set[str] = set()
    templates: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        basename = line.rsplit("\\", 1)[-1].rsplit("/", 1)[-1]
        if "%" in basename:
            templates.append(basename)
            continue
        stem = basename[: -len(".str")] if basename.lower().endswith(".str") else basename
        concrete.add(normalize(stem))
    return concrete, templates


def str_whitelist_allows(relative_path: str, whitelist: tuple[set[str], list[str]]) -> bool:
    concrete, templates = whitelist
    basename = relative_path.rsplit("\\", 1)[-1].rsplit("/", 1)[-1]
    # The whitelist was extracted from ASCII strings of CondaRO.exe, so a Korean
    # file name can never appear in it. Such a file is still official when the
    # official effect table references it and it ships in the GRF archives.
    if any(ord(character) > 127 for character in basename):
        return True
    stem = basename[: -len(".str")] if basename.lower().endswith(".str") else basename
    if normalize(stem) in concrete:
        return True
    for template in templates:
        pattern = re.escape(template).replace(r"%d", r"\d+").replace(r"%s", r".+")
        if re.fullmatch(pattern, basename, flags=re.IGNORECASE):
            return True
    return False


def load_spr_whitelist(path: Path) -> set[str]:
    allowed: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "%" in line:
            continue
        if line.startswith("\\"):
            stem = f"{EFFECT_SPRITE_ROOT}\\{line[1:]}"
        elif line.lower().startswith(EFFECT_SPRITE_ROOT.lower() + "\\"):
            stem = line
        else:
            continue
        if stem.lower().endswith(".spr"):
            stem = stem[:-4]
        allowed.add(stem.lower())
    return allowed


def spr_whitelist_allows(stem: str, allowed: set[str]) -> bool:
    """Whether a sprite stem is part of the official CondaRO.exe sprite whitelist.

    Korean sprite names cannot appear in the ASCII string list of the
    executable; the official effect table reference plus the GRF archive is the
    fact for those (same rule as the `.str` whitelist).
    """
    basename = stem.rsplit("\\", 1)[-1]
    if any(ord(character) > 127 for character in basename):
        return True
    return stem.lower() in allowed


# ------------------------------------------------------------------ resolving
def natural_key(text: str) -> list[object]:
    return [int(part) if part.isdigit() else part.lower() for part in re.split(r"(\d+)", text)]


def resolve_template(pattern_text: str, mapping: dict[str, str]) -> tuple[str, list[str]] | None:
    """Resolve a `%d`/`%s` template against a validated asset map."""
    escaped = re.escape(pattern_text)
    regex = re.compile("^" + escaped.replace(r"%d", r"\d+").replace(r"%s", r".+") + "$", re.IGNORECASE)
    matches = sorted((value for key, value in mapping.items() if regex.match(key)), key=natural_key)
    if not matches:
        return None
    return matches[0], matches


def normalize_table_path(value: str) -> str:
    return value.replace("/", "\\").strip()


def resolve_str(file_name: str, assets: Assets) -> tuple[str, list[str]]:
    """Resolve an EffectTable STR `file` value to `data\\texture\\effect\\<file>.str`."""
    candidate = normalize_table_path(file_name).lstrip("\\")
    if candidate.lower().startswith("effect\\"):
        candidate = candidate[len("effect\\") :]
    if candidate.lower().endswith(".str"):
        candidate = candidate[: -len(".str")]
    if not candidate:
        raise Dropped("missing_asset", f"empty STR file name for {file_name!r}")
    if "%" in candidate:
        resolved = resolve_template(f"{candidate}.str", assets.str_files)
        if resolved is None:
            raise Dropped("missing_asset", f"template {candidate}.str matches no archive file")
        return resolved
    actual = assets.str_files.get(f"{candidate}.str".lower())
    if actual is None:
        raise Dropped("missing_asset", f"{candidate}.str is not in the archives")
    return actual, [actual]


def resolve_spr(file_name: str, assets: Assets) -> tuple[str, list[str]]:
    """Resolve a sprite stem to an existing `.spr` + `.act` pair (no extension)."""
    candidate = normalize_table_path(file_name).strip().lstrip("\\")
    if candidate.lower().endswith((".spr", ".act")):
        candidate = candidate.rsplit(".", 1)[0]
    if candidate.lower().startswith(SPRITE_PREFIX):
        candidate = candidate[len(SPRITE_PREFIX) :]
    if not candidate:
        raise Dropped("missing_asset", f"empty SPR file name for {file_name!r}")
    stems: list[str] = []
    if candidate.lower().startswith(EFFECT_SPRITE_ROOT.lower() + "\\"):
        stems.append(candidate)
    elif "\\" in candidate:
        # An absolute sprite path (`absoluteSpriteName`) such as
        # `npc\\skel_archer_arrow` is already relative to `data\\sprite\\`.
        stems.append(candidate)
    else:
        stems.append(f"{EFFECT_SPRITE_ROOT}\\{candidate}")
        stems.append(candidate)
    if "%" in candidate:
        matches = [
            value[: -len(".spr")] for value in assets.sprites.values() if re.fullmatch(re.escape(candidate).replace(r"%d", r"\d+").replace(r"%s", r".+"), value[: -len(".spr")], re.IGNORECASE)
        ]
        matches = sorted(matches, key=natural_key)
        for stem in matches:
            if f"{stem.lower()}.spr" in assets.sprites and f"{stem.lower()}.act" in assets.actions:
                return stem, matches
        raise Dropped("missing_asset", f"sprite template {candidate} matches no .spr+.act pair")
    for stem in stems:
        if f"{stem.lower()}.spr" in assets.sprites and f"{stem.lower()}.act" in assets.actions:
            return stem, [stem]
    raise Dropped("missing_asset", f"{candidate}.spr/.act is not in the archives")


def resolve_texture(file_name: str, assets: Assets) -> tuple[str, list[str]]:
    """Resolve a texture name relative to `data\\texture\\`."""
    candidate = normalize_table_path(file_name).lstrip("\\")
    if candidate.lower().startswith("data\\"):
        candidate = candidate[len("data\\") :]
    if candidate.lower().startswith("texture\\"):
        candidate = candidate[len("texture\\") :]
    if not candidate:
        raise Dropped("missing_asset", f"empty texture name for {file_name!r}")
    if "%" in candidate:
        resolved = resolve_template(candidate, assets.textures)
        if resolved is None:
            raise Dropped("missing_asset", f"texture template {candidate} matches no archive file")
        return resolved
    actual = assets.textures.get(candidate.lower())
    if actual is not None:
        return actual, [actual]
    if not re.search(r"\.(tga|bmp|png|jpg)$", candidate, re.IGNORECASE):
        for extension in (".tga", ".bmp", ".png", ".jpg"):
            actual = assets.textures.get(f"{candidate}{extension}".lower())
            if actual is not None:
                return actual, [actual]
    raise Dropped("missing_asset", f"texture {candidate} is not in the archives")


def resolve_cylinder_texture(entry: dict, assets: Assets) -> tuple[str, list[str]]:
    file_name = entry.get("file") or entry.get("textureName")
    if not isinstance(file_name, str) or not file_name:
        raise Dropped("missing_asset", "cylinder layer has neither `file` nor `textureName`")
    candidate = normalize_table_path(file_name)
    if "\\" not in candidate and not re.search(r"\.(tga|bmp|png|jpg)$", candidate, re.IGNORECASE):
        candidate = f"effect\\{candidate}"
    return resolve_texture(candidate, assets)


def resolve_wav(wav_name: str, assets: Assets) -> tuple[str, list[str]]:
    candidate = normalize_table_path(wav_name).lstrip("\\")
    if candidate.lower().startswith("data\\wav\\"):
        candidate = candidate[len("data\\wav\\") :]
    if not candidate.lower().endswith(".wav"):
        candidate += ".wav"
    if "%" in candidate:
        resolved = resolve_template(candidate, assets.wavs)
        if resolved is None:
            raise Dropped("missing_audio", f"wav template {candidate} matches no archive file")
        return resolved
    actual = assets.wavs.get(candidate.lower())
    if actual is None:
        raise Dropped("missing_audio", f"{candidate} is not in the archives")
    return actual, [actual]


# ------------------------------------------------------------------- layering
def number(entry: dict, *keys: str) -> float | None:
    for key in keys:
        value = entry.get(key)
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return float(value)
    return None


def integer(entry: dict, *keys: str) -> int | None:
    value = number(entry, *keys)
    return None if value is None else int(value)


def tint_of(entry: dict) -> tuple[float, float, float] | None:
    red, green, blue = entry.get("red"), entry.get("green"), entry.get("blue")
    if all(isinstance(component, (int, float)) and not isinstance(component, bool) for component in (red, green, blue)):
        return (float(red), float(green), float(blue))
    return None


ANCHOR_BY_HINT = {
    "Ground": "Ground",
    "Caster": "Caster",
    "Target": "Target",
    # `releaseEffectId` / `beforeHitEffectId` are aimed at the target; only
    # procedural projectile primitives use `Travel`.
    "Travel": "Target",
    "Auto": "Auto",
}


def anchor_for(hint: str, phase: str) -> str:
    """Return the pipeline anchor of an official `SkillEffect` field.

    `effectId`/`successEffectId` map to `Auto` (target entity when the packet
    knows one, else target cell, else caster), which is what keeps a Blessing on
    an ally or a Heal on the healed target. `head: true` is emitted separately:
    the loader derives `vertical: Overhead` from it without overriding the
    anchor, so an overhead ally buff stays on the target entity.
    """
    if phase == "Ground":
        return "Ground"
    return ANCHOR_BY_HINT[hint]


def clamp_count(value: int | None, unmapped: "Counter[str]") -> int | None:
    if value is None:
        return None
    count = max(1, min(16, value))
    if count != value:
        unmapped["clamped.count"] += 1
    return count


def clamp_alpha(value: float | None, unmapped: "Counter[str]") -> float | None:
    if value is None:
        return None
    alpha = max(0.0, min(1.0, float(value)))
    if alpha != float(value):
        unmapped["clamped.alpha"] += 1
    return round(alpha, 3)


def clamp(value: float, minimum: float, maximum: float) -> float:
    return max(minimum, min(maximum, value))


def texture_size(entry: dict, layer_type: str | None) -> tuple[float, float]:
    """Convert an official 2D/3D texture `size` to world units (HitFlash)."""
    raw = number(entry, "size", "sizeStart", "sizeX")
    if raw is None:
        # 2D effects only define pixel ranges; the migrated rows use a mid-size
        # quad so the lens flare still reads as a hit flash.
        raw = 7.0 if layer_type == "2D" else 8.0
    size = clamp(raw * TEXTURE_SIZE_SCALE, TEXTURE_SIZE_MIN, TEXTURE_SIZE_MAX)
    raw_end = number(entry, "sizeEnd")
    if raw_end is not None:
        size_end = clamp(raw_end * TEXTURE_SIZE_SCALE, TEXTURE_SIZE_END_MIN, TEXTURE_SIZE_END_MAX)
    else:
        size_end = size * 1.15
    return round(size, 2), round(size_end, 2)


def projectile_size(entry: dict) -> tuple[float, float]:
    """Convert an official projectile `size` to world units (Projectile)."""
    raw = number(entry, "size", "sizeStart") or 100.0
    size = clamp(raw * PROJECTILE_SIZE_SCALE, 3.0, 24.0)
    return round(size, 2), round(size * 0.7, 2)


def world_position(entry: dict) -> tuple[float, float, float] | None:
    """Official (posx, posy, posz) -> world offset: y is the height (`posz`)."""
    posx, posy, posz = number(entry, "posx"), number(entry, "posy"), number(entry, "posz")
    if all(value in (None, 0.0) for value in (posx, posy, posz)):
        return None
    return (
        round((posx or 0.0) * OFFICIAL_EFFECT_UNIT, 3),
        round((posz or 0.0) * OFFICIAL_EFFECT_UNIT, 3),
        round((posy or 0.0) * OFFICIAL_EFFECT_UNIT, 3),
    )


def time_ms(value: int | None, unmapped: "Counter[str]") -> int | None:
    if value is None or value <= 0:
        return None
    if value > 30000:
        unmapped["clamped.duration_ms"] += 1
        value = 30000
    return value


def duration_of(entry: dict, unmapped: "Counter[str]") -> int | None:
    return time_ms(integer(entry, "duration"), unmapped)


def delay_of(entry: dict, unmapped: "Counter[str]", *keys: str) -> int | None:
    return time_ms(integer(entry, *keys), unmapped)


def blend_of(entry: dict) -> str | None:
    mode = integer(entry, "blendMode")
    if mode is None:
        return None
    # The official table uses webgl blendFunc source ONE for additive light
    # (`blendMode: 2`) and ONE_MINUS_SRC_ALPHA (default alpha) for the rest.
    return "Additive" if mode == 2 else "Alpha"


def common_layer_fields(entry: dict, assets: Assets, dropped: dict[str, list[str]], unmapped: "Counter[str]", context: str) -> dict:
    fields: dict = {}
    duration = duration_of(entry, unmapped)
    if duration is not None:
        fields["duration_ms"] = duration
    wav = entry.get("wav")
    if isinstance(wav, str) and wav:
        try:
            resolved, _alternatives = resolve_wav(wav, assets)
        except Dropped as error:
            dropped["missing_audio"].append(f"{context}: {error.detail}")
        else:
            fields["wav"] = resolved
    return fields


#: Official numeric/flag parameters that the v1 schema has no field for. They
#: are aggregated in the report instead of being emitted (unknown keys would be
#: rejected by the loader).
UNMAPPED_KEYS = (
    "repeat",
    "stackable",
    "direction",
    "stopAtEnd",
    "delayFrame",
    "delayLate",
    "frame",
    "zIndex",
    "fade",
    "fadeIn",
    "fadeOut",
    "sizeRand",
    "sizeSmooth",
    "sizeDelta",
    "sparkling",
    "sparkNumber",
    "shadowTexture",
    "playSprite",
    "rotateLate",
    "rotateLateDelta",
    "nbOfRotation",
    "poszStartRand",
    "poszEndRand",
    "poszStartRandMiddle",
    "poszEndRandMiddle",
    "posxStart",
    "posxEnd",
    "posyStart",
    "posyEnd",
    "poszStart",
    "poszEnd",
    "alphaMaxDelta",
)


def build_layer(
    entry: dict,
    phase: str,
    hint: str,
    assets: Assets,
    whitelists: tuple,
    dropped: dict[str, list[str]],
    unmapped: "Counter[str]",
    context: str,
) -> dict | None:
    """Turn one official effect-table entry into a pipeline layer (or None)."""
    layer_type = entry.get("type")
    file_name = entry.get("file")
    sprite_name = entry.get("spriteName")
    absolute_sprite = entry.get("absoluteSpriteName")
    file_list = entry.get("fileList")

    # Type-less entries are wav-only facts (delayed sounds) or one old texture;
    # they are not visual layers (the sound is collected as an effect fact).
    if layer_type is None:
        if not file_name:
            if entry.get("min"):
                unmapped["min_alternative_layer"] += 1
            return None
        if not str(file_name).lower().endswith((".tga", ".bmp", ".png", ".jpg")):
            return None
    if layer_type in UNSUPPORTED_TYPES:
        dropped["unsupported_layer_type"].append(f"{context}: type {layer_type}")
        return None
    if layer_type is not None and layer_type not in ("STR", "SPR", "3D", "2D", "CYLINDER"):
        dropped["unsupported_layer_type"].append(f"{context}: type {layer_type}")
        return None

    head = entry.get("head") is True
    anchor = anchor_for(hint, phase)

    # Projectile-style entries (`toSrc`/`fromSrc`) travel from the caster to the
    # target; the pipeline expresses them as a `Primitive`/`Projectile` layer
    # with the `Travel` anchor (the loader resolves both endpoints itself).
    if entry.get("toSrc") or entry.get("fromSrc"):
        if not isinstance(file_name, str) or not file_name:
            raise Dropped("unsupported_travel_sprite", "projectile layer uses a sprite, not a texture")
        resolved, _alternatives = resolve_texture(file_name, assets)
        layer = {"kind": "Primitive", "primitive": "Projectile", "anchor": "Travel", "file": resolved}
        size, size_end = projectile_size(entry)
        layer["size"] = size
        layer["size_end"] = size_end
        position = world_position(entry)
        if position is not None:
            layer["offset"] = position
        count = clamp_count(integer(entry, "duplicate"), unmapped)
        if count is not None:
            layer["count"] = count
        alpha = clamp_alpha(number(entry, "alphaMax"), unmapped)
        if alpha is not None:
            layer["alpha"] = alpha
        tint = tint_of(entry)
        if tint is not None:
            layer["tint"] = tint
        blend = blend_of(entry)
        if blend is not None:
            layer["blend"] = blend
        layer.update(common_layer_fields(entry, assets, dropped, unmapped, context))
        return finish_layer(layer, entry, dropped, unmapped, context, "Primitive")

    if layer_type == "STR":
        if not isinstance(file_name, str) or not file_name:
            return None
        resolved, alternatives = resolve_str(file_name, assets)
        if not str_whitelist_allows(resolved, whitelists[0]):
            dropped["not_whitelisted"].append(f"{context}: {resolved}")
        if len(alternatives) > 1:
            dropped["templates"].append(f"{context}: {file_name} resolved to {resolved} (candidates: {alternatives})")
        layer = {"kind": "Str", "anchor": anchor, "file": resolved}
        layer.update(common_layer_fields(entry, assets, dropped, unmapped, context))
        return finish_layer(layer, entry, dropped, unmapped, context, "Str")

    if layer_type == "SPR":
        if not isinstance(file_name, str) or not file_name:
            return None
        resolved, _alternatives = resolve_spr(file_name, assets)
        if not spr_whitelist_allows(resolved, whitelists[1]):
            dropped["not_whitelisted"].append(f"{context}: {resolved}")
        layer = {"kind": "Spr", "anchor": anchor, "files": [resolved]}
        layer.update(common_layer_fields(entry, assets, dropped, unmapped, context))
        y_offset = integer(entry, "yOffset")
        if y_offset is not None and not head:
            # The table value is in screen pixels, negative = above the actor.
            layer["offset"] = (0.0, round(max(0.0, -float(y_offset) * PIXEL_TO_WORLD), 3), 0.0)
        return finish_layer(layer, entry, dropped, unmapped, context, "Spr")

    if layer_type == "CYLINDER":
        resolved, _alternatives = resolve_cylinder_texture(entry, assets)
        layer = {"kind": "Cylinder", "anchor": anchor, "file": resolved}
        bottom = number(entry, "bottomSize")
        top = number(entry, "topSize")
        height = number(entry, "height")
        if bottom is not None:
            layer["bottom_radius"] = round(bottom * OFFICIAL_EFFECT_UNIT, 3)
        if top is not None:
            layer["top_radius"] = round(top * OFFICIAL_EFFECT_UNIT, 3)
        if height is not None:
            # Official cylinder sizes are in effect units (1 unit = 2 world units).
            layer["height"] = round(height * OFFICIAL_EFFECT_UNIT, 3)
        alpha = clamp_alpha(number(entry, "alphaMax"), unmapped)
        if alpha is not None:
            layer["alpha"] = alpha
        tint = tint_of(entry)
        if tint is not None:
            layer["tint"] = tint
        blend = blend_of(entry)
        if blend is not None:
            layer["blend"] = blend
        count = clamp_count(integer(entry, "duplicate"), unmapped)
        if count is not None:
            layer["count"] = count
        # `timeBetweenDupli` staggers the duplicates, `delayOffset` delays the
        # whole layer; the loader has one `delay_ms` slot (duplicate delay).
        delay = delay_of(entry, unmapped, "delayOffset", "timeBetweenDupli", "delayStart")
        if delay is not None:
            layer["delay_ms"] = delay
        animation = integer(entry, "animation")
        if animation is not None:
            layer["grow"] = {1: "Height", 2: "TopRadius", 4: "BothRadii"}.get(animation, "Constant")
        if entry.get("rotate") is True:
            layer["rotation_speed"] = 1.0
        position = world_position(entry)
        if position is not None:
            layer["offset"] = position
        layer.update(common_layer_fields(entry, assets, dropped, unmapped, context))
        return finish_layer(layer, entry, dropped, unmapped, context, "Cylinder")

    # 3D / 2D textures and sprite billboards embedded in a texture effect.
    if sprite_name or absolute_sprite:
        source = absolute_sprite if absolute_sprite else sprite_name
        if not isinstance(source, str) or not source:
            return None
        resolved, _alternatives = resolve_spr(source, assets)
        layer = {"kind": "Spr", "anchor": anchor, "files": [resolved]}
        count = clamp_count(integer(entry, "duplicate"), unmapped)
        if count is not None:
            layer["count"] = count
        size = number(entry, "size", "sizeX", "sizeStart")
        if size is not None:
            layer["size"] = round(float(size), 3)
        spread = number(entry, "posxRand", "posyRand")
        if spread is not None and spread > 0:
            layer["spread"] = round(abs(spread) * OFFICIAL_EFFECT_UNIT, 3)
        alpha = clamp_alpha(number(entry, "alphaMax"), unmapped)
        if alpha is not None:
            layer["alpha"] = alpha
        tint = tint_of(entry)
        if tint is not None:
            layer["tint"] = tint
        blend = blend_of(entry)
        if blend is not None:
            layer["blend"] = blend
        delay = delay_of(entry, unmapped, "delayOffset", "delayStart")
        if delay is not None:
            layer["delay_ms"] = delay
        layer.update(common_layer_fields(entry, assets, dropped, unmapped, context))
        return finish_layer(layer, entry, dropped, unmapped, context, "Spr")

    texture_files: list[str] = []
    if isinstance(file_name, str) and file_name:
        resolved, _alternatives = resolve_texture(file_name, assets)
        texture_files.append(resolved)
    if file_list:
        if not isinstance(file_list, list):
            raise Dropped("unsupported_file_list", "invalid fileList")
        for element in file_list:
            if isinstance(element, str) and element:
                resolved, _alternatives = resolve_texture(element, assets)
                if resolved not in texture_files:
                    texture_files.append(resolved)
    if not texture_files:
        dropped["layer_without_asset"].append(f"{context}: type {layer_type} has no file/spriteName")
        return None
    # One pipeline layer per official texture; `fileList` frames play in order.
    layers: list[dict] = []
    for texture in texture_files:
        layer = {"kind": "Texture", "anchor": anchor, "file": texture}
        count = clamp_count(integer(entry, "duplicate"), unmapped)
        if count is not None:
            layer["count"] = count
        # Sizes are converted to world units with the same factors the migrated
        # procedural rows use (the renderer treats `size` as world units).
        size, size_end = texture_size(entry, layer_type)
        layer["size"] = size
        layer["size_end"] = size_end
        spread = number(entry, "posxRand", "posyRand")
        if spread is not None and spread > 0:
            layer["spread"] = round(abs(spread) * OFFICIAL_EFFECT_UNIT, 3)
        position = world_position(entry)
        if position is not None:
            layer["offset"] = position
        alpha = clamp_alpha(number(entry, "alphaMax"), unmapped)
        if alpha is not None:
            layer["alpha"] = alpha
        tint = tint_of(entry)
        if tint is not None:
            layer["tint"] = tint
        blend = blend_of(entry)
        if blend is not None:
            layer["blend"] = blend
        delay = delay_of(entry, unmapped, "delayOffset", "delayStart")
        if delay is None:
            delay = delay_of(entry, unmapped, "timeBetweenDupli")
        if delay is not None:
            layer["delay_ms"] = delay
        layer.update(common_layer_fields(entry, assets, dropped, unmapped, context))
        layers.append(finish_layer(layer, entry, dropped, unmapped, context, "Texture"))
    if len(layers) == 1:
        return layers[0]
    return {"multi": layers}


def finish_layer(
    layer: dict,
    entry: dict,
    dropped: dict[str, list[str]],
    unmapped: "Counter[str]",
    context: str,
    kind: str,
) -> dict:
    # Facts the client uses for the per-entity budget/cleanup: always explicit.
    layer["attached_entity"] = entry.get("attachedEntity") is True
    if entry.get("head") is True:
        layer["head"] = True
    # Official playback parameters that the v1 schema has no field for are
    # aggregated in the report, never silently dropped.
    for key in UNMAPPED_KEYS:
        if key in entry:
            unmapped[f"{kind}.{key}"] += 1
    return layer


# --------------------------------------------------------------- fallback facts
def fallback_layers(effect_id: int, simplified: dict) -> list[dict]:
    """Minimal layer list from the simplified effects table (older extraction)."""
    entries = simplified.get(str(effect_id)) or []
    layers: list[dict] = []
    for entry in entries:
        layer_type = entry.get("type")
        file_name = entry.get("file")
        if not isinstance(file_name, str) or not file_name:
            continue
        if layer_type == "STR":
            layers.append({"type": "STR", "file": file_name, "fallback": True})
        elif layer_type == "SPR":
            layers.append({"type": "SPR", "file": file_name, "fallback": True})
        elif file_name.lower().endswith((".tga", ".bmp", ".png", ".jpg")):
            layers.append({"type": "3D", "file": file_name, "fallback": True})
    return layers


# ------------------------------------------------------------------- building
def resolve_official_sounds(sounds: dict, assets: Assets, dropped: dict[str, list[str]]) -> dict:
    """Validate the official per-skill sound facts against the archives."""
    result: dict[str, list[dict]] = {}
    for skill_text, rows in sorted(sounds.get("skills", {}).items(), key=lambda item: int(item[0])):
        candidates: list[dict] = []
        for row in rows:
            wav = row.get("wav")
            if not isinstance(wav, str) or not wav:
                continue
            try:
                resolved, _alternatives = resolve_wav(wav, assets)
            except Dropped as error:
                dropped["missing_audio"].append(f"skill {skill_text} sound: {error.detail}")
                continue
            candidate = {"wav": resolved, "field": row.get("field", ""), "effect_id": row.get("effect_id")}
            if candidate not in candidates:
                candidates.append(candidate)
        if candidates:
            result[skill_text] = candidates
    return result


#: The client spawns at most this many layers per phase; collapsing repeated
#: identical official layers keeps the generated data under the budget.
MAX_LAYERS_PER_PHASE = 32
#: The client clamps `count` (duplicates/burst quads) to this range.
MAX_LAYER_COUNT = 16


def coalesce_layers(layers: list[dict]) -> list[dict]:
    """Fold repeated identical layers of a phase into `count`/`delay_ms`.

    The official table and the per-field mapping can repeat the same layer (for
    example the 8 alternating `lens1.tga`/`lens2.tga` hit flashes, or the same
    element texture for several effect ids). The client renders `count` copies
    of one layer with `delay_ms` stagger, so folding identical layers keeps the
    visual but stays inside the per-phase spawn budget. Layers that differ in
    their official duplicate count stay separate (Increase AGI's three
    `ac_center2.tga` bursts with counts 7/3/10 are three layers).
    """
    order: list[dict] = []
    groups: dict[tuple, dict] = {}
    for layer in layers:
        key = tuple(sorted((field, repr(value)) for field, value in layer.items() if field not in ("delay_ms", "wav")))
        group = groups.get(key)
        if group is not None and "wav" in group and "wav" in layer and group["wav"] != layer["wav"]:
            key = key + (("wav", layer["wav"]),)
            group = groups.get(key)
        if group is None:
            group = dict(layer)
            groups[key] = group
            order.append(group)
        else:
            group["count"] = max(1, min(MAX_LAYER_COUNT, (group.get("count") or 1) + (layer.get("count") or 1)))
            if "delay_ms" not in group and "delay_ms" in layer:
                group["delay_ms"] = layer["delay_ms"]
            if "wav" not in group and "wav" in layer:
                group["wav"] = layer["wav"]
    return order


def build(
    facts: dict,
    layers_facts: dict,
    assets: Assets,
    whitelists: tuple,
    actions: dict,
    sounds: dict,
    learnable: dict | None,
) -> tuple[list[dict], dict]:
    str_whitelist, spr_whitelist = whitelists
    official_effects = layers_facts["effects"]
    simplified_effects = facts.get("effects", {})

    rows: dict[tuple[int, str], dict] = {}
    dropped: dict[str, list[str]] = defaultdict(list)
    unmapped: Counter[str] = Counter()
    effect_sounds: dict[str, list[dict]] = defaultdict(list)
    extras: Counter[str] = Counter()

    def emit(skill_id: int, phase: str, layer: dict) -> None:
        key = (skill_id, phase)
        row = rows.get(key)
        if row is None:
            row = {"skill": skill_id, "phase": phase, "layers": []}
            rows[key] = row
        row["layers"].append(layer)

    named_effects = layers_facts.get("named_effects", {})
    learnable_ids = {int(skill_id) for skill_id in (learnable or {}).get("skills", {})}

    for skill_text, fields in sorted(facts["skills"].items(), key=lambda item: int(item[0])):
        skill_id = int(skill_text)
        for field in fields:
            spec = FIELD_SPECS.get(field["field"])
            if spec is None:
                dropped["unknown_skill_effect_field"].append(f"skill {skill_id}: {field['field']}")
                continue
            phase, hint = spec

            # Sources of this phase: numeric effect ids, plus symbolic named
            # blocks (`ef_arrow_projectile`, `*_ground`, `*_sound`, ...) for
            # learnable pre-re skills. Symbolic references are a newer addition
            # of the reimplementation table, so they are never used to grow the
            # non-learnable (monster/renewal) part of the data.
            sources: list[tuple[str, list[dict]]] = []
            for effect_id in field.get("effects", []):
                effect_entry = official_effects.get(str(effect_id))
                if effect_entry is not None:
                    sources.append((str(effect_id), effect_entry["layers"]))
                    continue
                if effect_id == 0:
                    # Only reached when the extracted layer facts do not contain
                    # effect 0; the official EF_HIT1 block normally resolves.
                    dropped["effect_zero_standard_hit_flash"].append(
                        f"skill {skill_id} {phase}: effect 0 (official standard hit flash, procedural lane)"
                    )
                    continue
                fallback = fallback_layers(effect_id, simplified_effects)
                if fallback:
                    dropped["effect_facts_fallback"].append(
                        f"skill {skill_id} {phase}: effect {effect_id} uses simplified facts"
                    )
                    sources.append((str(effect_id), fallback))
                else:
                    dropped["unknown_effect"].append(f"skill {skill_id} {phase}: effect {effect_id} has no layer facts")
            for name in field.get("symbolic", []):
                if skill_id not in learnable_ids:
                    dropped["symbolic_skipped_non_learnable"].append(f"skill {skill_id} {phase}: {name}")
                    continue
                named_entry = named_effects.get(name)
                if named_entry is None:
                    dropped["symbolic_named_block_missing"].append(f"skill {skill_id} {phase}: {name}")
                    continue
                extras["symbolic_sources"] += 1
                extras["symbolic_layers"] += len(named_entry["layers"])
                sources.append((name, named_entry["layers"]))

            for source_name, official_layers in sources:
                context = f"skill {skill_id} {phase} effect {source_name}"
                built: list[dict] = []
                for entry in official_layers:
                    if entry.get("type") in UNSUPPORTED_TYPES:
                        dropped["unsupported_layer_type"].append(f"{context}: type {entry['type']}")
                        continue
                    try:
                        layer = build_layer(entry, phase, hint, assets, whitelists, dropped, unmapped, context)
                    except Dropped as error:
                        dropped[error.reason].append(f"{context}: {error.detail}")
                        continue
                    if layer is None:
                        continue
                    if "multi" in layer:
                        built.extend(layer["multi"])
                    else:
                        built.append(layer)
                # Effects can carry official sounds on entries without a file
                # (`wav`-only entries, sometimes delayed). The loader reads the
                # wav of the layers, so put the first one on the first visual
                # layer and keep all of them as validated sound facts.
                effect_wavs: list[str] = []
                for entry in official_layers:
                    wav = entry.get("wav")
                    if isinstance(wav, str) and wav and not (
                        entry.get("file") or entry.get("spriteName") or entry.get("absoluteSpriteName") or entry.get("textureName")
                    ):
                        wav_delay = integer(entry, "delayWav", "delayOffset")
                        effect_sounds[source_name].append({"wav": wav, **({"delay_ms": wav_delay} if wav_delay else {})})
                        try:
                            resolved_wav, _alternatives = resolve_wav(wav, assets)
                        except Dropped as error:
                            dropped["missing_audio"].append(f"{context}: {error.detail}")
                        else:
                            effect_wavs.append(resolved_wav)
                if built and effect_wavs and "wav" not in built[0]:
                    built[0]["wav"] = effect_wavs[0]
                for layer in built:
                    emit(skill_id, phase, layer)

    layers_before = sum(len(row["layers"]) for row in rows.values())
    for row in rows.values():
        row["layers"] = coalesce_layers(row["layers"])
    layers_after = sum(len(row["layers"]) for row in rows.values())
    extras["layers_before_coalesce"] = layers_before
    extras["layers_after_coalesce"] = layers_after
    for row in rows.values():
        if len(row["layers"]) > MAX_LAYERS_PER_PHASE:
            dropped["phase_over_layer_budget"].append(
                f"skill {row['skill']} {row['phase']}: {len(row['layers'])} layers (client cap {MAX_LAYERS_PER_PHASE})"
            )

    # Official caster actions are per skill: put the fact on the first phase row.
    action_facts = actions.get("actions", {})
    ordered: list[dict] = []
    for skill_id in sorted({key[0] for key in rows}):
        phases = sorted({key[1] for key in rows if key[0] == skill_id}, key=PHASE_ORDER.index)
        for phase in phases:
            row = rows[(skill_id, phase)]
            if row["layers"]:
                ordered.append(row)
        first = next((row for row in ordered if row["skill"] == skill_id), None)
        action = action_facts.get(str(skill_id))
        if first is not None and isinstance(action, dict) and action.get("action"):
            first["action"] = action["action"]
            if action.get("hold"):
                dropped["action_hold_not_mapped"].append(f"skill {skill_id}: official action hold=True")

    resolved_effect_sounds: dict[str, list[dict]] = {}
    for effect_id, entries in sorted(effect_sounds.items()):
        candidates: list[dict] = []
        for entry in entries:
            try:
                resolved, _alternatives = resolve_wav(entry["wav"], assets)
            except Dropped as error:
                dropped["missing_audio"].append(f"effect {effect_id} sound: {error.detail}")
                continue
            candidate = {"wav": resolved}
            if "delay_ms" in entry:
                candidate["delay_ms"] = entry["delay_ms"]
            if candidate not in candidates:
                candidates.append(candidate)
        if candidates:
            resolved_effect_sounds[str(effect_id)] = candidates

    stats = {
        "dropped": {reason: values for reason, values in sorted(dropped.items()) if values},
        "unmapped_parameters": dict(sorted(unmapped.items())),
        "effect_sounds": resolved_effect_sounds,
        "official_sounds": resolve_official_sounds(sounds, assets, dropped),
        "extras": dict(sorted(extras.items())),
    }
    return ordered, stats


# ------------------------------------------------------------------- coverage
def classify_gap(skill_id: int, facts: dict, layers_facts: dict) -> tuple[str, list[int], dict]:
    """Explain why a learnable skill has no emitted layer."""
    fields = facts["skills"].get(skill_id) or facts["skills"].get(str(skill_id))
    if not fields:
        return "no-official-effect-facts", [], {}
    effect_ids: list[int] = []
    layer_types: Counter[str] = Counter()
    absent = 0
    for field in fields:
        for effect_id in field["effects"]:
            effect_ids.append(effect_id)
            entry = layers_facts["effects"].get(str(effect_id))
            if entry is None:
                absent += 1
                continue
            for layer in entry["layers"]:
                layer_types[str(layer.get("type"))] += 1
    if not layer_types:
        return "effect-id-absent-from-extracted-effect-table", sorted(set(effect_ids)), {}
    if set(layer_types) <= UNSUPPORTED_TYPES | {"None"}:
        return "official-effect-is-hardcoded-procedural", sorted(set(effect_ids)), dict(sorted(layer_types.items()))
    projectile_sprites = 0
    file_layers = 0
    for effect_id in effect_ids:
        entry = layers_facts["effects"].get(str(effect_id))
        if entry is None:
            continue
        for layer in entry["layers"]:
            layer_type = str(layer.get("type"))
            if layer_type in UNSUPPORTED_TYPES or layer_type == "None":
                continue
            file_layers += 1
            if (layer.get("toSrc") or layer.get("fromSrc")) and (
                layer.get("spriteName") or layer.get("absoluteSpriteName")
            ) and not layer.get("file"):
                projectile_sprites += 1
    if file_layers and projectile_sprites == file_layers:
        return "official-projectile-sprite-unsupported", sorted(set(effect_ids)), dict(sorted(layer_types.items()))
    return "official-file-layers-dropped", sorted(set(effect_ids)), dict(sorted(layer_types.items()))


def build_coverage(rows: list[dict], learnable: dict | None, facts: dict, layers_facts: dict) -> dict:
    rows_by_skill: dict[int, list[dict]] = defaultdict(list)
    for row in rows:
        rows_by_skill[row["skill"]].append(row)
    phasis = Counter()
    kinds = Counter()
    for row in rows:
        phasis[row["phase"]] += 1
        for layer in row["layers"]:
            kinds[layer["kind"]] += 1

    coverage: dict = {
        "learnable_skills": 0,
        "skills_with_official_layers": 0,
        "skills_without_official_layers": 0,
        "layer_count_by_kind": dict(sorted(kinds.items())),
        "row_count_by_phase": {phase: phasis[phase] for phase in PHASE_ORDER if phasis[phase]},
        "no_effect": [],
        "missing_phases": [],
        "by_class": {},
        "learnable_rows": 0,
        "learnable_layers": 0,
        "learnable_kind_count": {},
    }
    if learnable is None:
        return coverage

    learnable_skills = {int(skill_id): skill for skill_id, skill in learnable["skills"].items()}
    coverage["learnable_skills"] = len(learnable_skills)
    coverage["learnable_rows"] = sum(1 for row in rows if row["skill"] in learnable_skills)
    learnable_kinds: Counter[str] = Counter()
    for row in rows:
        if row["skill"] not in learnable_skills:
            continue
        coverage["learnable_layers"] += len(row["layers"])
        for layer in row["layers"]:
            learnable_kinds[layer["kind"]] += 1
    coverage["learnable_kind_count"] = dict(sorted(learnable_kinds.items()))
    classes: dict[str, dict] = {}
    for class_name, class_entry in sorted(learnable["classes"].items()):
        class_skills = [int(skill_id) for skill_id in class_entry["skills"]]
        covered = [skill_id for skill_id in class_skills if rows_by_skill.get(skill_id)]
        missing = [skill_id for skill_id in class_skills if not rows_by_skill.get(skill_id)]
        classes[class_name] = {
            "learnable": len(class_skills),
            "covered": len(covered),
            "missing": len(missing),
            "layers": sum(len(row["layers"]) for skill_id in covered for row in rows_by_skill[skill_id]),
            "missing_skills": missing,
        }
    coverage["by_class"] = classes

    gap_reasons: Counter[str] = Counter()
    for skill_id, skill in sorted(learnable_skills.items()):
        skill_rows = rows_by_skill.get(skill_id, [])
        if skill_rows:
            coverage["skills_with_official_layers"] += 1
            expected_phases = {FIELD_SPECS[field["field"]][0] for field in (facts["skills"].get(skill_id) or facts["skills"].get(str(skill_id)) or []) if field["field"] in FIELD_SPECS}
            emitted_phases = {row["phase"] for row in skill_rows}
            for phase in PHASE_ORDER:
                if phase in expected_phases and phase not in emitted_phases:
                    coverage["missing_phases"].append(
                        {
                            "skill": skill_id,
                            "name": skill["name"],
                            "phase": phase,
                            "reason": "official phase exists but no layer asset could be resolved",
                        }
                    )
            continue
        coverage["skills_without_official_layers"] += 1
        reason, effect_ids, layer_types = classify_gap(skill_id, facts, layers_facts)
        gap_reasons[reason] += 1
        coverage["no_effect"].append(
            {
                "skill": skill_id,
                "name": skill["name"],
                "description": skill.get("description", ""),
                "type": skill.get("type", ""),
                "target_type": skill.get("target_type", ""),
                "jobs": skill.get("jobs", []),
                "reason": reason,
                "effect_ids": effect_ids,
                "official_layer_types": layer_types,
            }
        )
    coverage["gap_reasons"] = dict(sorted(gap_reasons.items()))
    return coverage


# --------------------------------------------------------------------- output
def escape_ron(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def ron_value(value) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return f"{value:.3f}"
    if isinstance(value, tuple):
        return "(" + ", ".join(ron_value(component) for component in value) + ")"
    if isinstance(value, list):
        return "[" + ", ".join(ron_value(component) for component in value) + "]"
    if isinstance(value, str):
        return f'"{escape_ron(value)}"'
    raise TypeError(f"cannot render {value!r}")


#: Layer fields that are Rust enums and must be rendered as bare identifiers.
RON_ENUM_FIELDS = {"kind", "anchor", "blend", "grow", "primitive"}

LAYER_FIELD_ORDER = [
    "kind",
    "anchor",
    "attached_entity",
    "head",
    "file",
    "files",
    "primitive",
    "offset",
    "scale",
    "size",
    "size_end",
    "duration_ms",
    "delay_ms",
    "count",
    "spread",
    "height",
    "bottom_radius",
    "top_radius",
    "rotation_speed",
    "grow",
    "alpha",
    "tint",
    "blend",
    "wav",
]


def render_ron(rows: list[dict], provenance: str) -> str:
    lines = [
        "// Official skill effect rows (multi-layer, multi-phase) for the unified pipeline.",
        "//",
        "// Generated by `tools/gen_skill_effect_layers.py` from the official Windows",
        "// client effect table facts; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_effect_layers.py`",
        "// Drift check:      `./tools/gen_skill_effect_layers.py --check`",
        "//",
        "// This is the official-table **fragment** consumed by the pipeline merge",
        "// generator (`tools/gen_skill_effects.py --official tools/skill_effects_official.ron`).",
        "// One descriptor per (skill, phase); every official layer is preserved in",
        "// table order. Anchors/overhead come from the official `SkillEffect` field,",
        "// `head` and `attachedEntity` facts, never from hand-coded skill lists.",
        "// Kinds: Str (.str screen sprite), Spr (.spr+.act), Texture (.tga/.bmp",
        "// billboard/ground), Cylinder (texture ground aura).",
        "// Every file/template was validated against the CondaRO GRFs at generation",
        "// time; assets that are not shipped were dropped (see the JSON report).",
        "(",
        f'    provenance: "{escape_ron(provenance)}",',
        "    version: 1,",
        "    effects: [",
    ]
    for row in rows:
        head = f"        (skill: {row['skill']}, phase: {row['phase']}"
        if "action" in row:
            head += f", action: {row['action']}"
        lines.append(head + ", layers: [")
        for layer in row["layers"]:
            parts = []
            for key in LAYER_FIELD_ORDER:
                if key not in layer:
                    continue
                if key in RON_ENUM_FIELDS:
                    parts.append(f"{key}: {layer[key]}")
                else:
                    parts.append(f"{key}: {ron_value(layer[key])}")
            lines.append("            (" + ", ".join(parts) + "),")
        lines.append("        ]),")
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_json(rows: list[dict], stats: dict, coverage: dict, payload: dict) -> str:
    document = {
        "provenance": {
            "official_skill_effect_facts": payload["official_effect_facts"],
            "effect_layer_facts": payload["effect_layer_facts"],
            "schema": "skill_effects.ron v1 (owned by the unified pipeline lane)",
            "fragment": "tools/skill_effects_official.ron",
            "boundary": (
                "This lane owns the official EffectTable file layers only. Hard-coded FUNC/callback visuals, "
                "procedural primitives, migrated hand-written layers, the merged data file and the Rust loader "
                "belong to the pipeline lane (tools/gen_skill_effects.py, skill_effects.ron)."
            ),
            "rules": {
                "anchor": {
                    "groundEffectId*": "Ground",
                    "beginCastEffectId*/castAuraEffectId": "Caster",
                    "effectId/successEffectId": "Auto (target entity when the packet knows one, else target cell, else caster)",
                    "*OnCaster*": "Caster",
                    "*OnTarget*": "Target",
                    "releaseEffectId/beforeHitEffectId": "Target (Travel is reserved for Primitive projectiles)",
                    "hitEffectId/damageEffectIdOnTarget": "Target",
                    "damageEffectIdOnCaster": "Caster",
                },
                "head": "official head flag, emitted as head:true; the client derives vertical:Overhead from it",
                "attached_entity": "always explicit, from the official attachedEntity flag (false when absent)",
                "kinds": {
                    "Str": "official type STR; file relative to data\\texture\\effect\\ and ending .str",
                    "Spr": "official type SPR / spriteName / absoluteSpriteName; stem relative to data\\sprite\\",
                    "Texture": "official type 3D/2D; file relative to data\\texture\\",
                    "Cylinder": "official type CYLINDER; texture relative to data\\texture\\; radii/height converted x2 to world units",
                },
                "kept_raw": "Texture size/size_end and offset(posx,posy,posz) are raw official values",
                "extractor_fixes": (
                    "firstHitEffectId is extracted; effect id 0 (EF_HIT1, effect\\\\pok3.tga) keeps its official "
                    "layer facts; symbolic SkillEffect references (ef_* / *_ground / *_sound blocks) are resolved "
                    "for learnable skills; named effect-table blocks are parsed as separate blocks; %d/%s "
                    "templates are expanded for .str/.spr/texture and wav names"
                ),
                "dropped": "assets not shipped in the CondaRO GRFs are dropped and listed in statistics.dropped",
            },
            "inputs": {name: sha256(Path(path)) for name, path in sorted(payload["inputs"].items())},
            "unified_loader": "korangar/src/world/library/skill_effects.rs (pipeline lane)",
        },
        "effects": rows,
        "statistics": {
            "rows": len(rows),
            "layers": sum(len(row["layers"]) for row in rows),
            "skills": len({row["skill"] for row in rows}),
            **stats,
        },
        "coverage": coverage,
    }
    return json.dumps(document, indent=1, ensure_ascii=False) + "\n"


PRIORITY_SKILLS = (34, 29, 33, 32, 78, 361, 28, 45)


def render_report(rows: list[dict], stats: dict, coverage: dict) -> str:
    layers = sum(len(row["layers"]) for row in rows)
    dropped = stats.get("dropped", {})
    lines = [
        "# Official multi-layer skill effect data",
        "",
        "Generated by `tools/gen_skill_effect_layers.py`; do not edit by hand.",
        "",
        "- fragment consumed by the unified pipeline: `tools/skill_effects_official.ron`",
        "- machine-readable facts + full gap list: `tools/skill_effects_official.json`",
        "- facade: `korangar/src/world/library/skill_effects.rs` (pipeline lane)",
        "",
        "## Headline",
        "",
        f"- rows (skill, phase): **{len(rows)}**",
        f"- layers: **{layers}**",
        f"- skills with official layers: **{len({row['skill'] for row in rows})}**",
        f"- learnable pre-re skills: **{coverage['learnable_skills']}**",
        f"- learnable skills with official layers: **{coverage['skills_with_official_layers']}**",
        f"- learnable skills without official layers: **{coverage['skills_without_official_layers']}**",
        f"- learnable skills with an official phase but no resolved layer: **{len(coverage.get('missing_phases', []))}** missing phases",
        "",
        "Sizes/offsets/radii are converted to world units with the same factors the migrated",
        "procedural rows use (`size * 0.85` billboards, `size * 0.09` projectiles, official",
        "positions/radii/cylinder heights `* 2`); `Primitive` layers carry an explicit",
        "`primitive` value. See the JSON `statistics.extras` for the coalescing counts.",
        "",
        "## Layers by kind",
        "",
        "| kind | layers |",
        "| --- | ---: |",
    ]
    for kind, count in coverage["layer_count_by_kind"].items():
        lines.append(f"| {kind} | {count} |")
    lines += ["", "## Rows by phase", "", "| phase | rows |", "| --- | ---: |"]
    for phase, count in coverage["row_count_by_phase"].items():
        lines.append(f"| {phase} | {count} |")

    rows_by_skill = {row["skill"]: row for row in rows if row["skill"] in PRIORITY_SKILLS}
    if rows_by_skill:
        lines += ["", "## Priority evidence (lead-requested skills)", ""]
        for skill_id in PRIORITY_SKILLS:
            row = rows_by_skill.get(skill_id)
            if row is None:
                lines.append(f"- skill {skill_id}: no row")
                continue
            lines.append(f"- **skill {skill_id}** ({row['phase']}): {len(row['layers'])} layers")
            for layer in row["layers"]:
                asset = layer.get("file") or ", ".join(layer.get("files", []))
                lines.append(
                    f"  - `{layer['kind']}` {asset} · anchor={layer.get('anchor', '-')}"
                    f" · head={str(layer.get('head', False)).lower()}"
                    f" · attached={str(layer.get('attached_entity', False)).lower()}"
                    f" · duration={layer.get('duration_ms', 0)}ms"
                    + (f" · wav={layer['wav']}" if "wav" in layer else "")
                )

    if dropped:
        lines += ["", "## Dropped / unmapped facts", "", "| reason | count |", "| --- | ---: |"]
        for reason, values in dropped.items():
            lines.append(f"| {reason} | {len(values)} |")

    gap_reasons = coverage.get("gap_reasons", {})
    if gap_reasons:
        lines += ["", "## Gap reasons (learnable skills without an official layer)", "", "| reason | skills |", "| --- | ---: |"]
        for reason, count in gap_reasons.items():
            lines.append(f"| {reason} | {count} |")

    classes = coverage.get("by_class", {})
    if classes:
        lines += [
            "",
            "## Coverage by class",
            "",
            "| class | learnable | covered | missing |",
            "| --- | ---: | ---: | ---: |",
        ]
        for class_name, entry in classes.items():
            lines.append(f"| {class_name} | {entry['learnable']} | {entry['covered']} | {entry['missing']} |")

    meaningful_gaps = [gap for gap in coverage.get("no_effect", []) if gap["reason"] != "no-official-effect-facts"]
    lines += [
        "",
        f"## Gap list: learnable skills without an official layer ({len(coverage.get('no_effect', []))})",
        "",
        f"- **{len(coverage.get('no_effect', [])) - len(meaningful_gaps)}** skills have no official effect facts at all "
        "(passive/mastery skills; the official client has no visual either)",
        f"- the other **{len(meaningful_gaps)}** are listed below; they are reported explicitly instead of inventing effects",
        "",
        "| skill | name | description | type | reason |",
        "| ---: | --- | --- | --- | --- |",
    ]
    for gap in meaningful_gaps:
        description = gap["description"].replace("|", "\\|")
        lines.append(f"| {gap['skill']} | {gap['name']} | {description} | {gap['type']} | {gap['reason']} |")

    missing_phases = coverage.get("missing_phases", [])
    if missing_phases:
        lines += ["", f"## Official phases without a resolved layer ({len(missing_phases)})", ""]
        for entry in missing_phases:
            lines.append(f"- skill {entry['skill']} ({entry['name']}) phase {entry['phase']}")

    missing_assets = dropped.get("missing_asset", [])
    if missing_assets:
        lines += ["", f"## Missing assets dropped ({len(missing_assets)})", ""]
        for value in missing_assets:
            lines.append(f"- {value}")

    missing_audio = dropped.get("missing_audio", [])
    if missing_audio:
        lines += ["", f"## Missing sounds dropped ({len(missing_audio)})", ""]
        for value in missing_audio:
            lines.append(f"- {value}")

    unmapped = stats.get("unmapped_parameters", {})
    if unmapped:
        lines += [
            "",
            "## Official layer parameters with no v1 schema field (aggregated)",
            "",
            "| parameter | occurrences |",
            "| --- | ---: |",
        ]
        for key, count in unmapped.items():
            lines.append(f"| {key} | {count} |")

    extras = stats.get("extras", {})
    if extras:
        lines += ["", "## Extractor-gap fixes included", ""]
        before = extras.get("layers_before_coalesce")
        after = extras.get("layers_after_coalesce")
        if before is not None and after is not None:
            lines.append(
                f"- repeated identical official layers folded into `count`/`delay_ms`: **{before} -> {after}** layer rows; "
                f"no phase exceeds **{max(len(row['layers']) for row in rows)}** layers (client cap 32)"
            )
        lines.append(
            f"- symbolic named effect blocks resolved for learnable skills: **{extras.get('symbolic_sources', 0)}** sources, "
            f"**{extras.get('symbolic_layers', 0)}** official layer facts (arrow/shield/spear projectiles, song ground effects, sound-only blocks)"
        )
        lines.append("- `firstHitEffectId` and effect **0** (`EF_HIT1`, `effect/pok3.tga`) are extracted and emitted")
        lines.append("- `%d`/`%s` templates are expanded for STR/SPR/texture **and wav** names")
        lines.append("- named effect-table blocks are parsed as their own blocks (no bleed into the preceding numeric effect)")
        lines.append("")

    lines += [
        "",
        "## Cross-check with the skill-audit baseline (`notes/skill-audit/`)",
        "",
        "The independent audit of the same official facts reports 457 learnable skills and 593",
        "official layer facts (110 STR + 205 TEXTURE + 20 SPR asset layers; 107 SOUND + 64 FUNC +",
        "59 HARDCODED + 28 PRIM non-file layers), of which 335 are visual asset layers and 305 were",
        "covered by the then-current client data. This lane emits every official file layer",
        "(repeated identical layers keep their count/delay; layers with different official",
        "duplicate counts stay separate) and adds the extractor-gap layers:",
        "",
        f"- learnable rows/layers in this fragment: **{coverage.get('learnable_rows', 0)} / {coverage.get('learnable_layers', 0)}** "
        f"({coverage.get('learnable_kind_count', {})})",
        "- audit baseline: 305/335 visual asset layers; this lane's only missing **learnable** visual",
        "  asset is the same one the audit lists: `shockwave.spr`/`.act` (skill 118, effect 145)",
        "- audit baseline missing learnable wavs: `_enemy_hit_wind1`, `bash3d`, `bash3d5` (reported below)",
        "- additions over the audit facts: `firstHitEffectId` + effect 0 (`pok3.tga`), symbolic `ef_*`",
        "  projectile/ground/sound blocks for learnable skills, and raw `%d` sound templates",
        "",
        "## How to regenerate / consume",
        "",
        "```sh",
        "./tools/extract_windows_client_effect_layers.py --check   # official effect-table layer facts",
        "./tools/extract_rathena_learnable_skills.py --check       # learnable pre-re skills + classes",
        "./tools/gen_skill_effect_layers.py                        # fragment + JSON + this report",
        "./tools/gen_skill_effect_layers.py --check                # drift guard",
        "./tools/gen_skill_effect_layers.py --report               # print this report",
        "./tools/test_skill_effect_layers.py                       # schema/coverage invariants",
        "# pipeline lane merge:",
        "./tools/gen_skill_effects.py --official tools/skill_effects_official.ron --validate-assets <game_dir>",
        "```",
        "",
        "All assets are validated against the CondaRO GRF archives; missing assets are dropped and",
        "listed above, never invented. Provenance is facts-only (numeric ids, field names, file",
        "names and playback parameters); no GPL source is vendored.",
    ]
    return "\n".join(lines) + "\n"


# ----------------------------------------------------------------------- main
def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if any committed output is out of date")
    parser.add_argument("--report", action="store_true", help="print the coverage/gap report")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--effects", default=str(DEFAULT_EFFECTS))
    parser.add_argument("--layers", default=str(DEFAULT_LAYERS))
    parser.add_argument("--actions", default=str(DEFAULT_ACTIONS))
    parser.add_argument("--sounds", default=str(DEFAULT_SOUNDS), help="official per-skill sound facts (optional)")
    parser.add_argument("--str-whitelist", default=str(DEFAULT_STR_WHITELIST))
    parser.add_argument("--spr-whitelist", default=str(DEFAULT_SPR_WHITELIST))
    parser.add_argument("--learnable", default=str(DEFAULT_LEARNABLE))
    parser.add_argument("--grf2019", default=str(DEFAULT_GRF2019), help="per-skill effect ids of the 2019 GRF tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--json", dest="json_output", default=str(DEFAULT_JSON))
    parser.add_argument("--report-output", default=str(DEFAULT_REPORT))
    args = parser.parse_args()

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")
    for path in (args.effects, args.layers, args.actions, args.str_whitelist, args.spr_whitelist):
        if not Path(path).is_file():
            raise SystemExit(f"required facts file not found: {path}")

    facts_payload = json.loads(Path(args.effects).read_text(encoding="utf-8"))
    layers_payload = json.loads(Path(args.layers).read_text(encoding="utf-8"))
    actions_payload = json.loads(Path(args.actions).read_text(encoding="utf-8"))
    sounds_payload = json.loads(Path(args.sounds).read_text(encoding="utf-8")) if Path(args.sounds).is_file() else {"skills": {}}
    learnable_payload = json.loads(Path(args.learnable).read_text(encoding="utf-8")) if Path(args.learnable).is_file() else None

    assets = Assets.collect(game_dir)
    if not assets.str_files or not assets.sprites or not assets.textures:
        raise SystemExit(f"no effect assets found in the archives under {game_dir}")
    whitelists = (load_str_whitelist(Path(args.str_whitelist)), load_spr_whitelist(Path(args.spr_whitelist)))

    facts = {"skills": {int(k): v for k, v in facts_payload["skills"].items()}, "effects": facts_payload["effects"]}

    # The 2019 GRF tables (`SKILL_EFFECT_INFO_LIST`) cover skills the community
    # dump does not list (newer jobs). Their per-skill effect ids are merged in
    # with the same field names so the same layer expansion applies; a skill
    # that both sources list keeps the community rows.
    GRF2019_FIELD = {
        "caster": "successEffectIdOnCaster",
        "target": "successEffectIdOnTarget",
        "ground": "groundEffectId",
        "begin": "beginCastEffectId",
    }
    grf2019_path = Path(args.grf2019)
    if grf2019_path.is_file():
        for skill_text, entry in json.loads(grf2019_path.read_text())["skills"].items():
            skill_id = int(skill_text)
            if skill_id in facts["skills"]:
                continue
            merged = [
                {"field": GRF2019_FIELD[kind], "effects": effect_ids}
                for kind, effect_ids in entry.get("effects", {}).items()
                if kind in GRF2019_FIELD and effect_ids
            ]
            if merged:
                facts["skills"][skill_id] = merged

    rows, stats = build(facts, layers_payload, assets, whitelists, actions_payload, sounds_payload, learnable_payload)

    if not rows:
        raise SystemExit("no official skill effect layers could be resolved against the archives/whitelist")

    # Defensive validation of every emitted asset (never trust the resolver).
    for row in rows:
        for layer in row["layers"]:
            if layer["kind"] == "Str":
                relative = layer["file"].lower()
                if relative not in assets.str_files:
                    raise SystemExit(f"skill {row['skill']}: .str not in archives: {layer['file']}")
            elif layer["kind"] == "Spr":
                for stem in layer["files"]:
                    if f"{stem.lower()}.spr" not in assets.sprites or f"{stem.lower()}.act" not in assets.actions:
                        raise SystemExit(f"skill {row['skill']}: sprite pair not in archives: {stem}")
            elif layer["kind"] in ("Texture", "Cylinder"):
                if layer["file"].lower() not in assets.textures:
                    raise SystemExit(f"skill {row['skill']}: texture not in archives: {layer['file']}")
            if "wav" in layer and layer["wav"].lower() not in assets.wavs:
                raise SystemExit(f"skill {row['skill']}: wav not in archives: {layer['wav']}")

    coverage = build_coverage(rows, learnable_payload, facts, layers_payload)
    provenance = (
        "official Windows client effect table (roBrowserLegacy RE dump of SkillEffect/EffectTable): "
        "multi-layer rows validated against CondaRO.exe whitelists and the CondaRO GRFs"
    )
    ron_content = render_ron(rows, provenance)
    json_content = build_json(
        rows,
        stats,
        coverage,
        {
            "official_effect_facts": facts_payload.get("provenance", {}),
            "effect_layer_facts": layers_payload.get("provenance", {}),
            "inputs": {
                "windows_client_skill_effects.json": args.effects,
                "windows_client_effect_layers.json": args.layers,
                "windows_client_skill_actions.json": args.actions,
                "windows_client_skill_sounds.json": args.sounds,
                "rathena_pre_re_learnable_skills.json": args.learnable,
                "windows_client_effect_whitelist.txt": args.str_whitelist,
                "windows_client_spr_whitelist.txt": args.spr_whitelist,
            },
        },
    )
    report_content = render_report(rows, stats, coverage)

    outputs = [(Path(args.output), ron_content), (Path(args.json_output), json_content), (Path(args.report_output), report_content)]
    if args.check:
        stale = [path for path, content in outputs if not path.is_file() or path.read_text(encoding="utf-8") != content]
        if stale:
            for path in stale:
                print(f"error: {path} is out of date; run ./tools/gen_skill_effect_layers.py", file=sys.stderr)
            return 1
        print(f"up to date: {len(rows)} rows, {sum(len(row['layers']) for row in rows)} layers")
        return 0

    for path, content in outputs:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    print(
        f"{args.output}: {len(rows)} rows, {sum(len(row['layers']) for row in rows)} layers, "
        f"{len({row['skill'] for row in rows})} skills"
    )
    if args.report:
        print(render_report(rows, stats, coverage))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
