#!/usr/bin/env python3
"""Generate the client data file with the procedural skill effect spec.

Many official skill visuals are **not** data files: they are hard-coded
procedural primitives of the Windows client (hit-flash billboards, cylinder
auras, projectile simulations). This generator turns the extracted facts about
those primitives (`tools/data/windows_client_procedural_effects.json`) into a data
file that the client loads at runtime through the regular `GameFileLoader`:

    korangar/archive/data/client/skill_procedural_effects.ron

The file is embedded with `include_str!` as the compiled fallback, so the data
file and the client cannot drift apart.

For every skill with a procedural component (the classification comes from the
committed audit table `tools/data/skill_procedural_components.csv`) the generator
maps each official effect id to one of the client's shared primitives:

* `HitFlash`   - a burst of camera-facing textured quads (official `2D`/`3D`
                 components, element hit flashes, melee impact flashes),
* `Projectile` - one or more textured quads flying from the caster to the
                 target (`fromSrc`/`toSrc` components) or a bolt rain falling
                 onto the target (`BoltRain`),
* `GroundAura` - a cylinder / ground ring built around a target cell (official
                 `CYLINDER` components, property ground fields).

The values (texture, duration, count/duplicates, size, alpha, colour, spread)
are taken from the official effect table where it has numeric values and from
documented defaults otherwise. Every referenced texture is validated against
the GRF archives; a component whose texture does not exist in the shipped data
set is dropped (the client then just shows the next layer / the generic cast
circle) instead of producing a broken effect.

Provenance / regeneration:

    ./tools/extract_windows_client_procedural_effects.py --source-dir <roBrowserLegacy src/DB>
    ./tools/gen_skill_procedural_effects.py                      # regenerate the RON
    ./tools/gen_skill_procedural_effects.py --check              # drift guard
    ./tools/gen_skill_procedural_effects.py --report             # coverage summary

Only facts from the roBrowserLegacy RE dump are used (never its GPL code); the
file format is documented in `korangar/src/world/library/skill_procedural_effects.rs`.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import struct
import sys
import zlib
from collections import Counter
from pathlib import Path

DEFAULT_GAME_DIR = "/home/phantom/Games/ragnarok/CondaRO-linux/client/game"
DEFAULT_FACTS = Path("tools/data/windows_client_procedural_effects.json")
DEFAULT_COMPONENTS = Path("tools/data/skill_procedural_components.csv")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_procedural_effects.ron")
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
TEXTURE_ROOT = "data\\texture\\"
EFFECT_PREFIX = TEXTURE_ROOT + "effect\\"
GRF_HEADER_SIZE = 46

#: Skill ids and phases follow the client's `SkillEffectVariant` enum.
#: The variant tells *when* the effect is spawned; `anchor` tells *where*.
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
    "successEffectIdOnCasterAimed": ("Success", "Caster"),
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

#: CSS order of the variants in the generated file (matches the client enum).
VARIANT_ORDER = ["Ground", "Cast", "Success", "Release", "Hit", "Caster"]

#: Primitive names understood by the client loader.
HIT_FLASH = "HitFlash"
PROJECTILE = "Projectile"
GROUND_AURA = "GroundAura"

#: Caps that keep a single cast from spawning an unbounded number of quads even
#: if the official table has many components. The client additionally applies
#: its own global effect budget and distance culling.
MAX_HIT_FLASH_COUNT = 8
MAX_PROJECTILE_COUNT = 6
MAX_RAIN_COUNT = 10
MAX_AURA_DUPLICATES = 8
MAX_ROWS_PER_VARIANT = 8

#: World units per official effect unit (5 effect units per map cell, the client
#: uses 10 world units per cell -> 2 world units per effect unit).
OFFICIAL_EFFECT_UNIT = 2.0

#: A ground effect cylinder is drawn per cell by the official client; scale its
#: `bottomSize`/`topSize` (in official effect units) up to a field that covers
#: roughly 3x3 map cells (one cell is 5 world units, a radius of 8 covers 3.2
#: cells across).
GROUND_FIELD_SCALE = 5.0
GROUND_FIELD_MIN_RADIUS = 8.0
#: The official per-cell values are tuned for a grid of many small cylinders;
#: our single field cylinder needs a stronger alpha to be visible.
GROUND_FIELD_ALPHA = 2.2
#: Minimum visible height of a target-anchored ground cylinder in world units.
GROUND_FIELD_MIN_HEIGHT = 12.0

#: World size factors for the billboard/projectile quads (official effect units
#: to world units, tuned so a hit flash covers roughly one map cell).
HIT_FLASH_SCALE = 0.85
PROJECTILE_SCALE = 0.09

ELEMENT_COLORS = {
    "cold": (0.35, 0.65, 1.0),
    "fire": (1.0, 0.42, 0.10),
    "wind": (0.55, 1.0, 0.70),
    "earth": (1.0, 0.80, 0.35),
    "holy": (1.0, 1.0, 0.72),
    "poison": (0.60, 1.0, 0.30),
    "dark": (0.65, 0.40, 1.0),
    "neutral": (1.0, 1.0, 1.0),
}

#: Element derived from the skill name; used for colour/texture defaults only.
ELEMENT_HINTS = (
    ("cold", ("COLD", "ICE", "FROST", "HYO", "FROZEN", "JACKFROST", "WATERBALL")),
    ("fire", ("FIRE", "FLAME", "BAKUEN", "KOUEN", "METEOR", "VOLCANO", "MAGMA", "EXPLOSION", "SELFDESTRUCT", "NAPALM")),
    ("wind", ("WIND", "THUNDER", "LIGHTNING", "RAI", "GALE", "JUPITEL", "STORM", "VERMILION")),
    ("earth", ("EARTH", "STONE", "QUAKE", "HEAVENDRIVE", "SIGHTRASHER")),
    ("holy", ("HOLY", "ANGEL", "BENEDICT", "SANCTU", "ASPER", "CRUCIS", "PNEUMA")),
    ("poison", ("POISON", "VENOM", "ACID")),
    ("dark", ("DARK", "UNDEAD", "CURSE", "EVIL")),
)

#: Fallback textures (relative to `data\texture\`, validated against the GRFs).
TEXTURE_DEFAULTS = {
    (HIT_FLASH, "cold"): "effect/ice.tga",
    (HIT_FLASH, "fire"): "effect/fireparticle.tga",
    (HIT_FLASH, "wind"): "effect/thunder_ball_0001.bmp",
    (HIT_FLASH, "earth"): "effect/pok3.tga",
    (HIT_FLASH, "holy"): "effect/lens1.tga",
    (HIT_FLASH, "poison"): "effect/pok3.tga",
    (HIT_FLASH, "dark"): "effect/lens1.tga",
    (HIT_FLASH, "neutral"): "effect/pok3.tga",
    (PROJECTILE, "cold"): "effect/icearrow.tga",
    (PROJECTILE, "fire"): "effect/fireparticle.tga",
    (PROJECTILE, "wind"): "effect/thunder_ball_0001.bmp",
    (PROJECTILE, "earth"): "effect/pok3.tga",
    (PROJECTILE, "holy"): "effect/lens1.tga",
    (PROJECTILE, "poison"): "effect/pok3.tga",
    (PROJECTILE, "dark"): "effect/lens1.tga",
    (PROJECTILE, "neutral"): "effect/lens1.tga",
    (GROUND_AURA, "cold"): "effect/ring_blue.tga",
    (GROUND_AURA, "fire"): "effect/ring_red.tga",
    (GROUND_AURA, "wind"): "effect/ring_blue.tga",
    (GROUND_AURA, "earth"): "effect/ring_yellow.tga",
    (GROUND_AURA, "holy"): "effect/ring_yellow.tga",
    (GROUND_AURA, "poison"): "effect/ring_red.tga",
    (GROUND_AURA, "dark"): "effect/ring_purple.tga",
    (GROUND_AURA, "neutral"): "effect/ring_yellow.tga",
}

#: Named (symbolic) effect ids that are bolt simulations or other FUNC based
#: effects but that we can express with a shared primitive.
BOLT_RAIN_ELEMENTS = {"cold": "cold", "fire": "fire", "ice": "cold", "water": "cold"}

#: FUNC token classification. `None` entries are recognised but deliberately not
#: implemented (cast auras that already exist, screen shake and the sprite-effect
#: family that is a documented follow-up).
FUNC_TOKEN_PRIMITIVES = {
    "BoltRain": None,  # handled separately (rain)
    "IceSpikes": HIT_FLASH,
    "spamEffect": HIT_FLASH,
    "bragiNoteEffect": HIT_FLASH,
    "QuadHorn": HIT_FLASH,
    "PropertyGround": GROUND_AURA,
    "GroundAura": GROUND_AURA,
    "SwirlingAura": GROUND_AURA,
    "SpiderWeb": GROUND_AURA,
    "BlueTile": GROUND_AURA,
    "Camera.setQuake": None,
    "CastCone": None,
    "CastRing": None,
    "CastCircle": None,
    "CastFunnel": None,
    "MagicRing": None,
    "MagicTarget": None,
    "LockOnTarget": None,
    "CasterMarks": None,
    "startCasterStrobe": None,
    "SpiritSphere": None,
    "Level99Bubble": None,
    "LPEffect": None,
    "WarlockSphere": None,
    "RotateLine": None,
}

#: Proc phase column -> variant for the generic fallback of a skill whose
#: components could not be classified (still better than showing nothing).
FALLBACK_VARIANTS = (
    ("hit_proc", "Hit", "Target", HIT_FLASH),
    ("release_proc", "Release", "Travel", PROJECTILE),
    ("ground_proc", "Ground", "Target", GROUND_AURA),
    ("success_proc", "Success", "Caster", HIT_FLASH),
    ("caster_proc", "Caster", "Caster", HIT_FLASH),
    ("cast_proc", "Cast", "Caster", HIT_FLASH),
)


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


def collect_textures(game_dir: Path, archive_dir: Path | None) -> dict[str, str]:
    """Return lower-case texture path (relative to data\\texture\\) -> archive spelling."""
    textures: dict[str, str] = {}
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if not lower.startswith(EFFECT_PREFIX):
                continue
            relative = name[len(TEXTURE_ROOT) :]
            textures.setdefault(relative.lower(), relative)
    if archive_dir is not None:
        effect_dir = archive_dir / "data" / "texture" / "effect"
        if effect_dir.is_dir():
            for path in effect_dir.rglob("*"):
                if path.is_file():
                    relative = str(path.relative_to(effect_dir))
                    textures.setdefault(relative.lower(), relative)
    return textures


def normalize_texture_candidate(candidate: str) -> str:
    candidate = candidate.replace("/", "\\").strip()
    if candidate.lower().startswith("data\\texture\\"):
        candidate = candidate[len("data\\texture\\") :]
    if not candidate.lower().startswith("effect\\"):
        candidate = "effect\\" + candidate
    return candidate


def resolve_texture(candidate: str, textures: dict[str, str]) -> str | None:
    """Resolve a `file:`/texture name from the official tables against the GRFs."""
    if not candidate:
        return None
    candidate = normalize_texture_candidate(candidate)
    lower = candidate.lower()

    variants = [lower]
    if "%d" in lower:
        variants = [lower.replace("%d", str(index)) for index in range(0, 9)]
    elif not re.search(r"\.(tga|bmp|jpg|png)$", lower):
        variants = [lower + suffix for suffix in (".tga", ".bmp", ".jpg", ".png")]

    for variant in variants:
        actual = textures.get(variant)
        if actual is not None:
            return actual

    return None


def skill_element(skill_name: str) -> str:
    upper = skill_name.upper()
    for element, hints in ELEMENT_HINTS:
        if any(hint in upper for hint in hints):
            return element
    return "neutral"


def clamp(value: float, minimum: float, maximum: float) -> float:
    return max(minimum, min(maximum, value))


def number(component: dict, *names: str) -> float | None:
    for name in names:
        value = component.get(name)
        if isinstance(value, (int, float)):
            return float(value)
    return None


def color_of(component: dict, element: str) -> tuple[float, float, float]:
    color = component.get("color")
    if isinstance(color, list) and len(color) == 3:
        return tuple(clamp(float(channel), 0.0, 1.0) for channel in color)
    return ELEMENT_COLORS[element]


def row(**kwargs) -> dict:
    defaults = {
        "skill": 0,
        "variant": "Success",
        "anchor": "Auto",
        "primitive": HIT_FLASH,
        "texture": "",
        "count": 1,
        "duration_ms": 350,
        "delay_ms": 0,
        "size": 16.0,
        "size_end": 16.0,
        "spread": 2.0,
        "height": 8.0,
        "bottom_radius": 0.0,
        "top_radius": 0.0,
        "alpha": 0.9,
        "color": (1.0, 1.0, 1.0),
        "additive": True,
        "rotation_speed": 0.0,
    }
    defaults.update(kwargs)
    return defaults


def texture_for(component: dict, primitive: str, element: str, textures: dict[str, str]) -> str | None:
    """Pick and validate a texture for a component, falling back per primitive."""
    candidates: list[str] = []
    for name in ("file", "textureFile", "texture", "textureName", "spriteName", "absoluteSpriteName"):
        value = component.get(name)
        if isinstance(value, str) and value:
            candidates.append(value)

    sprite_hints = {
        "arrow": "effect/arrown.tga",
        "spear": "effect/spear.bmp",
        "fireball": "effect/fireparticle.tga",
        "particle": "effect/pok3.tga",
        "ice": "effect/icearrow.tga",
    }
    for candidate in list(candidates):
        sprite_name = candidate.rsplit("/", 1)[-1].lower()
        for hint, texture in sprite_hints.items():
            if hint in sprite_name and texture not in candidates:
                candidates.append(texture)
                break

    candidates.append(TEXTURE_DEFAULTS[(primitive, element)])

    for candidate in candidates:
        resolved = resolve_texture(candidate, textures)
        if resolved is not None:
            return resolved
    return None


def hit_flash_row(component: dict, skill: dict, variant: str, anchor: str, textures: dict[str, str]) -> dict | None:
    texture = texture_for(component, HIT_FLASH, skill["element"], textures)
    if texture is None:
        return None

    component_type = component.get("type")
    duration = number(component, "duration", "durationRand")
    size_value = number(component, "size", "sizeStart")
    if size_value is None and component_type == "2D":
        # 2D effects define a pixel range (`sizeRandStartX/Y`); use a mid-size
        # world quad so the lens flare still reads as a hit flash.
        size_value = 7.0
    world_size = clamp((size_value if size_value is not None else 8.0) * HIT_FLASH_SCALE, 4.0, 22.0)

    size_end_value = number(component, "sizeEnd")
    size_end = world_size * 1.15
    if size_end_value is not None:
        size_end = clamp(size_end_value * HIT_FLASH_SCALE, 2.0, 32.0)

    spread = 2.0
    posx = number(component, "posxRand", "posxStart")
    posy = number(component, "posyRand")
    if posx is not None or posy is not None:
        spread = clamp(math.hypot(posx or 0.0, posy or 0.0) * OFFICIAL_EFFECT_UNIT, 0.0, 24.0)

    height = (number(component, "posz", "poszStart") or 0.0) * OFFICIAL_EFFECT_UNIT + 8.0
    alpha = clamp(number(component, "alphaMax") or 0.9, 0.05, 1.0)
    count = int(clamp(number(component, "duplicate") or 1, 1, MAX_HIT_FLASH_COUNT))

    blend_mode = number(component, "blendMode")
    additive = True if blend_mode is None else blend_mode == 2.0

    return row(
        skill=skill["id"],
        variant=variant,
        anchor=anchor,
        primitive=HIT_FLASH,
        texture=texture,
        count=count,
        duration_ms=int(clamp(duration if duration is not None else 350, 100, 4000)),
        size=round(world_size, 2),
        size_end=round(size_end, 2),
        spread=round(spread, 2),
        height=round(height, 2),
        alpha=round(alpha, 3),
        color=color_of(component, skill["element"]),
        additive=additive,
    )


def projectile_row(component: dict, skill: dict, variant: str, anchor: str, textures: dict[str, str]) -> dict | None:
    texture = texture_for(component, PROJECTILE, skill["element"], textures)
    if texture is None:
        return None

    duration = number(component, "duration") or 400.0
    size_value = number(component, "size", "sizeStart") or 100.0
    world_size = clamp(size_value * PROJECTILE_SCALE, 3.0, 24.0)
    height = (number(component, "posz", "poszStart") or 3.0) * OFFICIAL_EFFECT_UNIT + 4.0
    alpha = clamp(number(component, "alphaMax") or 1.0, 0.05, 1.0)

    return row(
        skill=skill["id"],
        variant=variant,
        anchor="Travel",
        primitive=PROJECTILE,
        texture=texture,
        count=int(clamp(number(component, "duplicate") or 1, 1, MAX_PROJECTILE_COUNT)),
        duration_ms=int(clamp(duration, 100, 2000)),
        delay_ms=int(clamp(number(component, "delayStart") or 0.0, 0.0, 3000.0)),
        size=round(world_size, 2),
        size_end=round(world_size * 0.7, 2),
        height=round(height, 2),
        alpha=round(alpha, 3),
        color=color_of(component, skill["element"]),
        additive=True,
    )


def rain_row(component: dict, skill: dict, variant: str, textures: dict[str, str]) -> dict | None:
    element = BOLT_RAIN_ELEMENTS.get(component.get("element", ""), skill["element"])
    texture = resolve_texture(TEXTURE_DEFAULTS[(PROJECTILE, element)], textures)
    if texture is None:
        return None

    return row(
        skill=skill["id"],
        variant=variant,
        anchor="Rain",
        primitive=PROJECTILE,
        texture=texture,
        count=6,
        duration_ms=600,
        size=7.0,
        size_end=6.0,
        spread=14.0,
        height=40.0,
        alpha=0.8,
        color=ELEMENT_COLORS[element],
        additive=True,
    )


def ground_aura_row(component: dict, skill: dict, variant: str, anchor: str, textures: dict[str, str]) -> dict | None:
    texture = texture_for(component, GROUND_AURA, skill["element"], textures)
    if texture is None:
        return None

    bottom_size = number(component, "bottomSize", "topSize") or 1.0
    top_size = number(component, "topSize", "bottomSize") or 1.0
    height = (number(component, "height") or 0.0) * OFFICIAL_EFFECT_UNIT

    # The official client draws ground skill effects as per-cell cylinders (the
    # flame-like `magic_*` textures are the cylinder walls). Keep that shape, but
    # enforce a minimum visible height: a very short cylinder is nearly
    # invisible from the top-down game camera.
    target_anchored = variant == "Ground" or anchor == "Target"
    if target_anchored and height > 0.0:
        height = max(height, GROUND_FIELD_MIN_HEIGHT)
    alpha = clamp(number(component, "alphaMax") or 0.5, 0.05, 1.0)
    blend_mode = number(component, "blendMode")

    # The official ground effects are drawn as one small cylinder per cell of
    # the skill area. The client only knows the target cell, not the area, so
    # scale a per-cell cylinder up to a field that covers several cells.
    radius_scale = GROUND_FIELD_SCALE if target_anchored else OFFICIAL_EFFECT_UNIT
    radius_min = GROUND_FIELD_MIN_RADIUS if target_anchored else 2.0

    return row(
        skill=skill["id"],
        variant=variant,
        anchor=anchor,
        primitive=GROUND_AURA,
        texture=texture,
        count=int(clamp(number(component, "duplicate") or 1, 1, MAX_AURA_DUPLICATES)),
        duration_ms=int(clamp(number(component, "duration") or 1500, 300, 6000)),
        delay_ms=int(clamp(number(component, "timeBetweenDupli") or 0.0, 0.0, 2000.0)),
        bottom_radius=round(clamp(bottom_size * radius_scale, radius_min, 80.0), 2),
        top_radius=round(clamp(top_size * radius_scale, radius_min, 80.0), 2),
        height=round(height, 2),
        alpha=round(clamp(alpha * (GROUND_FIELD_ALPHA if target_anchored else 1.0), 0.15, 0.85), 3),
        color=color_of(component, skill["element"]),
        additive=blend_mode == 2.0,
        rotation_speed=0.6 if component.get("rotate") else 0.0,
    )


def func_row(component: dict, skill: dict, variant: str, anchor: str, textures: dict[str, str]) -> list[dict]:
    tokens = set(component.get("func", ()))
    rows: list[dict] = []

    if "BoltRain" in tokens:
        rain = rain_row(component, skill, variant, textures)
        return [rain] if rain is not None else []

    for token in sorted(tokens):
        primitive = FUNC_TOKEN_PRIMITIVES.get(token)
        if primitive is None:
            continue
        if primitive == GROUND_AURA:
            aura = ground_aura_row(component, skill, variant, "Target", textures)
            if aura is not None:
                # Property ground fields are large fields around the cell.
                aura["bottom_radius"] = max(aura["bottom_radius"], 20.0)
                aura["top_radius"] = max(aura["top_radius"], 20.0)
                aura["height"] = max(aura["height"], 10.0)
                aura["duration_ms"] = max(aura["duration_ms"], 2000)
                rows.append(aura)
        elif primitive == HIT_FLASH:
            flash = hit_flash_row(component, skill, variant, anchor, textures)
            if flash is not None:
                rows.append(flash)

    if not tokens:
        # Unknown FUNC implementation: give damage phases a generic flash so the
        # hit is still visible, and leave the cast/utility phases alone.
        if variant in ("Hit", "Release") and anchor in ("Target", "Travel"):
            texture = resolve_texture(TEXTURE_DEFAULTS[(HIT_FLASH, skill["element"])], textures)
            if texture is not None:
                rows.append(
                    row(
                        skill=skill["id"],
                        variant=variant,
                        anchor="Target",
                        primitive=HIT_FLASH,
                        texture=texture,
                        color=ELEMENT_COLORS[skill["element"]],
                    )
                )
        elif variant == "Ground":
            aura = ground_aura_row({"type": "CYLINDER"}, skill, variant, "Target", textures)
            if aura is not None:
                rows.append(aura)

    return rows


def component_rows(component: dict, skill: dict, variant: str, anchor: str, textures: dict[str, str]) -> list[dict]:
    component_type = component.get("type")

    # Sound-only components have no type and no visual.
    if component_type is None:
        return []

    if component_type in ("3D", "2D"):
        projectile_like = bool(component.get("fromSrc") or component.get("toSrc"))
        if projectile_like:
            projectile = projectile_row(component, skill, variant, anchor, textures)
            return [projectile] if projectile is not None else []
        flash = hit_flash_row(component, skill, variant, anchor, textures)
        return [flash] if flash is not None else []

    if component_type == "CYLINDER":
        aura = ground_aura_row(component, skill, variant, anchor, textures)
        return [aura] if aura is not None else []

    if component_type == "FUNC":
        return func_row(component, skill, variant, anchor, textures)

    if component_type == "PRIM":
        flash = hit_flash_row(component, skill, variant, anchor, textures)
        return [flash] if flash is not None else []

    if component_type == "QuadHorn":
        flash = hit_flash_row(component, skill, variant, anchor, textures)
        return [flash] if flash is not None else []

    # `STR` is handled by the `.str` asset table, `SPR` is the documented
    # sprite-effect follow-up; the remaining types (RSM/WATERFALL/TRAIL/...) are
    # not implemented yet.
    return []


def effect_references(field: dict) -> list[dict]:
    return field["references"]


def merge_rows(rows: list[dict]) -> list[dict]:
    """Merge equivalent rows (e.g. the 8 alternating lens flashes of EF_HIT2)."""
    merged: dict[tuple, dict] = {}
    max_counts = {HIT_FLASH: MAX_HIT_FLASH_COUNT, PROJECTILE: MAX_PROJECTILE_COUNT, GROUND_AURA: MAX_AURA_DUPLICATES}

    for entry in rows:
        key = (
            entry["skill"],
            entry["variant"],
            entry["anchor"],
            entry["primitive"],
            entry["texture"],
            entry["duration_ms"],
            entry["delay_ms"],
            round(entry["size"], 1),
            round(entry["size_end"], 1),
            round(entry["spread"], 1),
            round(entry["height"], 1),
            round(entry["bottom_radius"], 1),
            round(entry["top_radius"], 1),
            round(entry["alpha"], 2),
            tuple(round(channel, 2) for channel in entry["color"]),
            entry["additive"],
            round(entry["rotation_speed"], 2),
        )
        if key in merged:
            merged[key]["count"] = min(merged[key]["count"] + entry["count"], max_counts[entry["primitive"]])
        else:
            merged[key] = dict(entry)

    return list(merged.values())


def fallback_rows(skill: dict, textures: dict[str, str]) -> list[dict]:
    """Guarantee at least one visible primitive for every procedural skill."""
    for column, variant, anchor, primitive in FALLBACK_VARIANTS:
        proc_types = skill["proc"].get(column)
        if not proc_types:
            continue

        if primitive == GROUND_AURA:
            texture = resolve_texture(TEXTURE_DEFAULTS[(GROUND_AURA, skill["element"])], textures)
            if texture is None:
                continue
            return [
                row(
                    skill=skill["id"],
                    variant=variant,
                    anchor=anchor,
                    primitive=GROUND_AURA,
                    texture=texture,
                    duration_ms=2000,
                    bottom_radius=20.0,
                    top_radius=20.0,
                    height=10.0,
                    alpha=0.45,
                    color=ELEMENT_COLORS[skill["element"]],
                    additive=False,
                )
            ]

        if primitive == PROJECTILE:
            texture = resolve_texture(TEXTURE_DEFAULTS[(PROJECTILE, skill["element"])], textures)
            if texture is None:
                continue
            return [
                row(
                    skill=skill["id"],
                    variant=variant,
                    anchor="Travel",
                    primitive=PROJECTILE,
                    texture=texture,
                    duration_ms=400,
                    size=10.0,
                    size_end=7.0,
                    height=10.0,
                    color=ELEMENT_COLORS[skill["element"]],
                )
            ]

        texture = resolve_texture(TEXTURE_DEFAULTS[(HIT_FLASH, skill["element"])], textures)
        if texture is None:
            continue
        return [
            row(
                skill=skill["id"],
                variant=variant,
                anchor=anchor,
                primitive=HIT_FLASH,
                texture=texture,
                duration_ms=350,
                size=14.0,
                size_end=18.0,
                alpha=0.75,
                color=ELEMENT_COLORS[skill["element"]],
            )
        ]

    return []


def build_rows(facts: dict, skills: list[dict], textures: dict[str, str]) -> tuple[list[dict], dict]:
    rows: list[dict] = []
    stats = {
        "texture_dropped": 0,
        "skills_with_rows": 0,
        "proc_only_total": 0,
        "proc_only_covered": 0,
        "data_proc_total": 0,
        "data_proc_covered": 0,
    }

    skills_by_id = {skill["id"]: skill for skill in skills}
    effects = facts["effects"]

    for skill in skills:
        skill_rows: list[dict] = []
        skill_facts = facts["skills"].get(str(skill["id"]), [])

        for field in skill_facts:
            variant, anchor = FIELD_SPECS[field["field"]]
            for reference in effect_references(field):
                key = str(reference["id"]) if "id" in reference else reference["name"]
                components = effects.get(key)
                if components is None:
                    continue
                for component in components:
                    new_rows = component_rows(component, skill, variant, anchor, textures)
                    if not new_rows:
                        stats["texture_dropped"] += 1
                    skill_rows.extend(new_rows)

        skill_rows = merge_rows(skill_rows)

        # Group by variant and keep the rows deterministic and bounded.
        by_variant: dict[str, list[dict]] = {}
        for entry in skill_rows:
            by_variant.setdefault(entry["variant"], []).append(entry)

        final_rows: list[dict] = []
        for variant in VARIANT_ORDER:
            variant_rows = by_variant.get(variant, [])
            variant_rows.sort(
                key=lambda entry: (
                    [GROUND_AURA, PROJECTILE, HIT_FLASH].index(entry["primitive"]),
                    entry["anchor"] != "Target",
                    entry["texture"],
                    entry["delay_ms"],
                )
            )
            final_rows.extend(variant_rows[:MAX_ROWS_PER_VARIANT])

        if not final_rows:
            final_rows = fallback_rows(skill, textures)

        if final_rows:
            stats["skills_with_rows"] += 1
            category = skills_by_id[skill["id"]]["category"]
            if category == "proc":
                stats["proc_only_covered"] += 1
            elif category == "data+proc":
                stats["data_proc_covered"] += 1
            rows.extend(final_rows)

    for skill in skills:
        category = skill["category"]
        if category == "proc":
            stats["proc_only_total"] += 1
        elif category == "data+proc":
            stats["data_proc_total"] += 1

    return rows, stats


def load_skills(path: Path) -> list[dict]:
    """Read the audit table and keep every skill with a procedural component."""
    with path.open(newline="", encoding="utf-8") as handle:
        reader = __import__("csv").DictReader(handle)
        skills: list[dict] = []
        for record in reader:
            proc = {
                f"{phase}_proc": record[f"{phase}_proc"].strip()
                for phase in ("ground", "success", "release", "hit", "caster", "cast")
            }
            if not any(proc.values()):
                continue
            skills.append(
                {
                    "id": int(record["skill_id"]),
                    "name": record["name"],
                    "category": record["category"],
                    "proc": proc,
                    "element": skill_element(record["name"]),
                }
            )
    return skills


def render(rows: list[dict], stats: dict) -> str:
    lines = [
        "// Procedural (hard-coded) skill effect spec for the client.",
        "//",
        "// Generated by `tools/gen_skill_procedural_effects.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_procedural_effects.py`",
        "// Drift check:      `./tools/gen_skill_procedural_effects.py --check`",
        "//",
        "// Facts extracted from the official client's effect tables by",
        "// `tools/extract_windows_client_procedural_effects.py` (roBrowserLegacy RE dump);",
        "// every texture is validated against the CondaRO GRF archives.",
        "//",
        "// Format: one effect per row. `variant` is the spawn phase (Ground, Cast,",
        "// Success, Release, Hit, Caster), `anchor` is where it spawns (Caster,",
        "// Target, Travel = caster -> target, Rain = from above onto the target,",
        "// Auto = target when the packet knows one, caster for self casts).",
        "// `primitive` is one of HitFlash, Projectile, GroundAura; the remaining",
        "// fields are the primitive parameters (texture relative to data\\texture\\).",
        "//",
        f"// Covered: {stats['proc_only_covered']}/{stats['proc_only_total']} procedural-only skills, "
        f"{stats['data_proc_covered']}/{stats['data_proc_total']} data+procedural skills.",
        "//",
        "// The file is loaded at runtime through the normal client file loader, so a",
        "// server can override it without rebuilding the client; the compiled fallback",
        "// embeds this same file with include_str!.",
        "(",
        '    provenance: "official procedural effect tables (roBrowserLegacy RE dump), textures validated against the CondaRO GRFs",',
        "    effects: [",
    ]

    for entry in rows:
        color = ", ".join(f"{channel:.3f}" for channel in entry["color"])
        lines.append(
            "        ("
            f"skill: {entry['skill']}, "
            f"variant: {entry['variant']}, "
            f"anchor: {entry['anchor']}, "
            f"primitive: {entry['primitive']}, "
            f'texture: "{entry["texture"].replace(chr(92), chr(92) * 2)}", '
            f"count: {entry['count']}, "
            f"duration_ms: {entry['duration_ms']}, "
            f"delay_ms: {entry['delay_ms']}, "
            f"size: {entry['size']:.2f}, "
            f"size_end: {entry['size_end']:.2f}, "
            f"spread: {entry['spread']:.2f}, "
            f"height: {entry['height']:.2f}, "
            f"bottom_radius: {entry['bottom_radius']:.2f}, "
            f"top_radius: {entry['top_radius']:.2f}, "
            f"alpha: {entry['alpha']:.3f}, "
            f"color: ({color}), "
            f"additive: {'true' if entry['additive'] else 'false'}, "
            f"rotation_speed: {entry['rotation_speed']:.2f}"
            "),"
        )

    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--report", action="store_true", help="print the coverage summary")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--archive-dir", default=None)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--components", default=str(DEFAULT_COMPONENTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")

    facts = json.loads(Path(args.facts).read_text(encoding="utf-8"))
    skills = load_skills(Path(args.components))
    if not skills:
        raise SystemExit(f"no procedural skills found in the audit table {args.components}")

    textures = collect_textures(game_dir, Path(args.archive_dir) if args.archive_dir else None)
    if not textures:
        raise SystemExit(f"no effect textures found in the archives under {game_dir}")

    rows, stats = build_rows(facts, skills, textures)
    if not rows:
        raise SystemExit("no procedural effects could be generated")

    rows.sort(
        key=lambda entry: (
            entry["skill"],
            VARIANT_ORDER.index(entry["variant"]),
            [GROUND_AURA, PROJECTILE, HIT_FLASH].index(entry["primitive"]),
            entry["texture"],
            entry["delay_ms"],
        )
    )

    content = render(rows, stats)
    output = Path(args.output)

    if args.report:
        primitive_counts = Counter(entry["primitive"] for entry in rows)
        variant_counts = Counter(entry["variant"] for entry in rows)
        covered_skills = {entry["skill"] for entry in rows}
        print(f"rows: {len(rows)} ({len(covered_skills)} skills)")
        print(f"primitives: {dict(sorted(primitive_counts.items()))}")
        print(f"variants: {dict(sorted(variant_counts.items()))}")
        print(
            f"coverage: procedural-only {stats['proc_only_covered']}/{stats['proc_only_total']}, "
            f"data+procedural {stats['data_proc_covered']}/{stats['data_proc_total']}"
        )
        uncovered = sorted(
            skill["id"] for skill in skills if skill["id"] not in covered_skills and skill["category"] == "proc"
        )
        print(f"uncovered procedural-only skills: {uncovered}")

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_procedural_effects.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} entries)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {len(rows)} entries ({len({entry['skill'] for entry in rows})} skills)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
