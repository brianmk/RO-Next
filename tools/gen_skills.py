#!/usr/bin/env python3
"""Generate the data-driven skill behaviour and procedural layer data file.

The unified skill effect pipeline (`korangar/archive/data/client/skill_effects.ron`)
describes what a skill *looks like*. A few behaviours and procedural effects
survived outside of it as hard-coded skill-id branches and effect constants:

* Rolling Cutter (2036) casts through the channeling protocol path,
* Warp Portal (27) shows a provisional portal before `ZC_WARPLIST`/skill-unit
  packets arrive,
* Free Cast (278) allows movement while casting on Sage (16) / Professor (4017),
* Storm Gust (87/89) spawns a procedural dark cloud (`cloud1..7.tga`, no data
  asset exists for it),
* the cast circle, warp portal and level aura are drawn from hard-coded
  effect texture names and geometry constants.

This generator turns `tools/data/skills_custom.ron` (the single hand-written input)
into the canonical runtime file

    korangar/archive/data/client/skills.ron

which is loaded at runtime through the regular `GameFileLoader` (so a server
can override it in its client archive) and embedded with `include_str!` as the
compiled fallback. The Rust schema lives in `korangar/src/world/skills.rs`;
`korangar --check-skills [PATH]` validates the committed file (or an archive
override).

The generator needs **no** game data or facts JSON: the file is hand-curated
data (like `tools/data/skill_effects_custom.ron`). `--check` is the drift guard and
runs offline in CI.

Usage:
    ./tools/gen_skills.py                     # regenerate the data file
    ./tools/gen_skills.py --check             # fail if the file is out of date
    ./tools/gen_skills.py --validate-assets /path/to/client/game
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

DEFAULT_OUTPUT = Path("korangar/archive/data/client/skills.ron")
DEFAULT_CUSTOM = Path("tools/data/skills_custom.ron")

#: Phases accepted by the `clouds` rows (matches the client `SkillPhase` enum).
PHASES = ("Ground", "Cast", "Success", "Release", "Hit", "Caster")
#: Behaviour flags in the order they are rendered.
BEHAVIOUR_FLAGS = ("channeling", "warp_target", "move_while_casting")

SCRIPT_DIR = Path(__file__).resolve().parent


def load_ron_helpers():
    """Reuse the minimal RON reader/writer of the skill effect generator."""
    spec = importlib.util.spec_from_file_location("gen_skill_effects", SCRIPT_DIR / "gen_skill_effects.py")
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load tools/gen_skill_effects.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ron = load_ron_helpers()


def fail(message: str) -> None:
    raise SystemExit(f"error: {message}")


def as_int(value: object, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        fail(f"{name} must be an integer, got {value!r}")
    return value


def as_float(value: object, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        fail(f"{name} must be a number, got {value!r}")
    return float(value)


def as_bool(value: object, name: str) -> bool:
    if not isinstance(value, bool):
        fail(f"{name} must be a boolean, got {value!r}")
    return value


def as_text(value: object, name: str) -> str:
    if not isinstance(value, str) or not value:
        fail(f"{name} must be a non-empty string, got {value!r}")
    return value


def as_tuple3(value: object, name: str) -> tuple[float, float, float]:
    if not isinstance(value, tuple) or len(value) != 3:
        fail(f"{name} must be a 3-tuple, got {value!r}")
    return tuple(as_float(component, name) for component in value)  # type: ignore[return-value]


def as_list(value: object, name: str) -> list[object]:
    if not isinstance(value, list):
        fail(f"{name} must be a list, got {value!r}")
    return value


def check_positive(value: float, name: str) -> float:
    if not value > 0.0:
        fail(f"{name} must be positive, got {value}")
    return value


def check_non_negative(value: float, name: str) -> float:
    if value < 0.0:
        fail(f"{name} must not be negative, got {value}")
    return value


def check_alpha(value: float, name: str) -> float:
    if not 0.0 <= value <= 1.0:
        fail(f"{name} must be in 0..=1, got {value}")
    return value


def check_texture(path: str, name: str, suffixes: tuple[str, ...] = (".tga", ".bmp")) -> str:
    if not path.lower().endswith(suffixes):
        fail(f"{name} must end in {'/'.join(suffixes)}, got {path!r}")
    return path


def parse_behaviours(document: dict[str, object]) -> list[dict[str, object]]:
    rows = as_list(document.get("behaviours"), "behaviours")
    result: list[dict[str, object]] = []
    seen: set[int] = set()
    for row in rows:
        if not isinstance(row, dict):
            fail(f"behaviour row is not a map: {row!r}")
        skill = as_int(row.get("skill"), "behaviour skill")
        if skill == 0:
            fail("a behaviour row uses skill id 0")
        if skill in seen:
            fail(f"duplicate behaviour row for skill {skill}")
        seen.add(skill)
        flags = {}
        for flag in BEHAVIOUR_FLAGS:
            value = row.get(flag)
            if value is not None:
                flags[flag] = as_bool(value, f"skill {skill} {flag}")
        unknown = set(row) - {"skill", *BEHAVIOUR_FLAGS}
        if unknown:
            fail(f"skill {skill}: unknown behaviour fields {sorted(unknown)}")
        if not any(flags.values()):
            fail(f"skill {skill}: behaviour row has no enabled flag")
        result.append({"skill": skill, **flags})
    if not result:
        fail("the data file contains no skill behaviours")
    result.sort(key=lambda row: row["skill"])
    return result


def parse_clouds(document: dict[str, object]) -> list[dict[str, object]]:
    rows = as_list(document.get("clouds"), "clouds")
    result: list[dict[str, object]] = []
    seen: set[int] = set()
    for row in rows:
        if not isinstance(row, dict):
            fail(f"cloud row is not a map: {row!r}")
        skills = sorted(as_int(skill, "cloud skill") for skill in as_list(row.get("skills"), "cloud skills"))
        if not skills:
            fail("a cloud row has no skill ids")
        if len(set(skills)) != len(skills):
            fail(f"cloud row has duplicate skill ids: {skills}")
        for skill in skills:
            if skill == 0:
                fail("a cloud row uses skill id 0")
            if skill in seen:
                fail(f"duplicate cloud row for skill {skill}")
            seen.add(skill)
        phase = as_text(row.get("phase"), "cloud phase")
        if phase not in PHASES:
            fail(f"cloud row for skills {skills}: unknown phase {phase!r}")
        textures = [check_texture(as_text(texture, "cloud texture"), "cloud texture") for texture in as_list(row.get("textures"), "cloud textures")]
        if not textures:
            fail(f"cloud row for skills {skills} has no textures")
        count = as_int(row.get("count"), "cloud count")
        if not 1 <= count <= 64:
            fail(f"cloud row for skills {skills}: count {count} is outside 1..=64")
        duration = as_int(row.get("duration_ms"), "cloud duration_ms")
        fade_in = as_int(row.get("fade_in_ms"), "cloud fade_in_ms")
        fade_out = as_int(row.get("fade_out_ms"), "cloud fade_out_ms")
        if duration <= 0:
            fail(f"cloud row for skills {skills}: duration_ms must be positive")
        if fade_in < 0 or fade_out < 0 or fade_in + fade_out > duration:
            fail(f"cloud row for skills {skills}: the fade times do not fit the duration")
        result.append(
            {
                "skills": skills,
                "phase": phase,
                "textures": textures,
                "count": count,
                "radius": check_positive(as_float(row.get("radius"), "cloud radius"), "cloud radius"),
                "base_height": check_non_negative(as_float(row.get("base_height"), "cloud base_height"), "cloud base_height"),
                "height_variation": check_non_negative(
                    as_float(row.get("height_variation"), "cloud height_variation"), "cloud height_variation"
                ),
                "size_min": check_positive(as_float(row.get("size_min"), "cloud size_min"), "cloud size_min"),
                "size_variation": check_non_negative(
                    as_float(row.get("size_variation"), "cloud size_variation"), "cloud size_variation"
                ),
                "spin_min": as_float(row.get("spin_min"), "cloud spin_min"),
                "spin_variation": check_non_negative(
                    as_float(row.get("spin_variation"), "cloud spin_variation"), "cloud spin_variation"
                ),
                "alpha": check_alpha(as_float(row.get("alpha"), "cloud alpha"), "cloud alpha"),
                "color": as_tuple3(row.get("color"), "cloud color"),
                "duration_ms": duration,
                "fade_in_ms": fade_in,
                "fade_out_ms": fade_out,
            }
        )
    result.sort(key=lambda row: row["skills"][0])
    return result


def parse_cast_circle(document: dict[str, object]) -> dict[str, object]:
    section = document.get("cast_circle")
    if not isinstance(section, dict):
        fail("missing cast_circle section")
    allowed = {
        "ring_texture",
        "glow_texture",
        "radius",
        "height",
        "ring_rotation_speed",
        "inner_ring_rotation_speed",
        "inner_ring_scale",
        "glow_scale",
    }
    if set(section) != allowed:
        fail(f"cast_circle fields do not match the schema: {sorted(set(section) ^ allowed)}")
    return {
        "ring_texture": check_texture(as_text(section["ring_texture"], "cast_circle ring_texture"), "cast_circle ring_texture"),
        "glow_texture": check_texture(as_text(section["glow_texture"], "cast_circle glow_texture"), "cast_circle glow_texture"),
        "radius": check_positive(as_float(section["radius"], "cast_circle radius"), "cast_circle radius"),
        "height": check_non_negative(as_float(section["height"], "cast_circle height"), "cast_circle height"),
        "ring_rotation_speed": as_float(section["ring_rotation_speed"], "cast_circle ring_rotation_speed"),
        "inner_ring_rotation_speed": as_float(section["inner_ring_rotation_speed"], "cast_circle inner_ring_rotation_speed"),
        "inner_ring_scale": check_positive(as_float(section["inner_ring_scale"], "cast_circle inner_ring_scale"), "cast_circle inner_ring_scale"),
        "glow_scale": check_positive(as_float(section["glow_scale"], "cast_circle glow_scale"), "cast_circle glow_scale"),
    }


def parse_warp_portal(document: dict[str, object]) -> dict[str, object]:
    section = document.get("warp_portal")
    if not isinstance(section, dict):
        fail("missing warp_portal section")
    allowed = {"streak_texture", "gradient_texture", "particle_sprite", "skill_radius", "skill_height", "skill_ring_count"}
    if set(section) != allowed:
        fail(f"warp_portal fields do not match the schema: {sorted(set(section) ^ allowed)}")
    ring_count = as_int(section["skill_ring_count"], "warp_portal skill_ring_count")
    if not 1 <= ring_count <= 8:
        fail(f"warp_portal skill_ring_count {ring_count} is outside 1..=8")
    particle = as_text(section["particle_sprite"], "warp_portal particle_sprite")
    if not particle.lower().endswith(".spr"):
        fail(f"warp_portal particle_sprite must be a .spr path, got {particle!r}")
    return {
        "streak_texture": check_texture(as_text(section["streak_texture"], "warp_portal streak_texture"), "warp_portal streak_texture"),
        "gradient_texture": check_texture(
            as_text(section["gradient_texture"], "warp_portal gradient_texture"), "warp_portal gradient_texture"
        ),
        "particle_sprite": particle,
        "skill_radius": check_positive(as_float(section["skill_radius"], "warp_portal skill_radius"), "warp_portal skill_radius"),
        "skill_height": check_non_negative(as_float(section["skill_height"], "warp_portal skill_height"), "warp_portal skill_height"),
        "skill_ring_count": ring_count,
    }


def parse_level_aura(document: dict[str, object]) -> dict[str, object]:
    section = document.get("level_aura")
    if not isinstance(section, dict):
        fail("missing level_aura section")
    allowed = {
        "ring_liner_texture",
        "ring_blue_texture",
        "ring_red_texture",
        "bubble_texture",
        "glow_textures",
        "sparkle_texture",
    }
    if set(section) != allowed:
        fail(f"level_aura fields do not match the schema: {sorted(set(section) ^ allowed)}")
    textures = [
        check_texture(as_text(texture, "level_aura glow texture"), "level_aura glow texture")
        for texture in as_list(section["glow_textures"], "level_aura glow_textures")
    ]
    if not textures:
        fail("level_aura has no glow textures")
    return {
        "ring_liner_texture": check_texture(
            as_text(section["ring_liner_texture"], "level_aura ring_liner_texture"), "level_aura ring_liner_texture"
        ),
        "ring_blue_texture": check_texture(
            as_text(section["ring_blue_texture"], "level_aura ring_blue_texture"), "level_aura ring_blue_texture"
        ),
        "ring_red_texture": check_texture(
            as_text(section["ring_red_texture"], "level_aura ring_red_texture"), "level_aura ring_red_texture"
        ),
        "bubble_texture": check_texture(as_text(section["bubble_texture"], "level_aura bubble_texture"), "level_aura bubble_texture"),
        "glow_textures": textures,
        "sparkle_texture": check_texture(as_text(section["sparkle_texture"], "level_aura sparkle_texture"), "level_aura sparkle_texture"),
    }


def parse_custom(path: Path) -> dict[str, object]:
    if not path.is_file():
        fail(f"missing custom skill data: {path}")
    document = ron.read_ron(path)
    if not isinstance(document, dict):
        fail(f"{path}: top level value is not a map")
    provenance = as_text(document.get("provenance"), "provenance")
    return {
        "provenance": provenance,
        "behaviours": parse_behaviours(document),
        "clouds": parse_clouds(document),
        "cast_circle": parse_cast_circle(document),
        "warp_portal": parse_warp_portal(document),
        "level_aura": parse_level_aura(document),
    }


def quote(text: str) -> str:
    return f'"{ron.escape(text)}"'


def render(data: dict[str, object]) -> str:
    lines: list[str] = []
    add = lines.append

    add("// Data-driven skill behaviour and procedural effect data for the client.")
    add("//")
    add("// Generated by `tools/gen_skills.py`; do not edit by hand.")
    add("// Regenerate with: `./tools/gen_skills.py`")
    add("// Drift check:      `./tools/gen_skills.py --check`")
    add("// Schema check:     `korangar --check-skills`")
    add("//")
    add("// Skill *visuals* live in `skill_effects.ron` (the unified effect pipeline).")
    add("// This file owns the behaviour flags that used to be skill-id branches in")
    add("// `korangar/src/lib.rs` and the procedural layers/effects that the official")
    add("// effect tables cannot express (Storm Gust cloud, cast circle, warp portal,")
    add("// level aura).")
    add("//")
    add("// Loader: `korangar/src/world/skills.rs`. The file is loaded through the")
    add("// normal client file loader (`data\\client\\skills.ron`), so a server can")
    add("// override it without rebuilding the client; the committed copy is also the")
    add("// compiled `include_str!` fallback.")
    add("(")
    add(f"    provenance: {quote(str(data['provenance']))},")
    add("    version: 1,")
    add("    // Per-skill behavioural flags (only the enabled flags are written).")
    add("    behaviours: [")
    behaviours = data["behaviours"]
    assert isinstance(behaviours, list)
    for row in behaviours:
        assert isinstance(row, dict)
        fields = [f"skill: {row['skill']}"]
        for flag in BEHAVIOUR_FLAGS:
            if flag in row:
                fields.append(f"{flag}: {'true' if row[flag] else 'false'}")
        add(f"        ({', '.join(fields)}),")
    add("    ],")
    add("    // Procedural cloud layers (Storm Gust): one row per skill id set.")
    add("    clouds: [")
    clouds = data["clouds"]
    assert isinstance(clouds, list)
    for row in clouds:
        assert isinstance(row, dict)
        add("        (")
        skills = ", ".join(str(skill) for skill in row["skills"])  # type: ignore[union-attr]
        add(f"            skills: [{skills}],")
        add(f"            phase: {row['phase']},")
        textures = ", ".join(quote(str(texture)) for texture in row["textures"])  # type: ignore[union-attr]
        add(f"            textures: [{textures}],")
        add(f"            count: {row['count']},")
        for field in (
            "radius",
            "base_height",
            "height_variation",
            "size_min",
            "size_variation",
            "spin_min",
            "spin_variation",
            "alpha",
        ):
            add(f"            {field}: {ron.number(row[field])},")
        color = row["color"]
        assert isinstance(color, tuple)
        add(f"            color: ({', '.join(ron.number(component) for component in color)}),")
        for field in ("duration_ms", "fade_in_ms", "fade_out_ms"):
            add(f"            {field}: {row[field]},")
        add("        ),")
    add("    ],")
    add("    // Ground cast circle drawn while a cast with a cast time is running.")
    add("    cast_circle: (")
    cast_circle = data["cast_circle"]
    assert isinstance(cast_circle, dict)
    for field in ("ring_texture", "glow_texture"):
        add(f"        {field}: {quote(str(cast_circle[field]))},")
    for field in ("radius", "height", "ring_rotation_speed", "inner_ring_rotation_speed", "inner_ring_scale", "glow_scale"):
        add(f"        {field}: {ron.number(cast_circle[field])},")
    add("    ),")
    add("    // Map connection warp and flat skill-unit warp portal.")
    add("    warp_portal: (")
    warp_portal = data["warp_portal"]
    assert isinstance(warp_portal, dict)
    for field in ("streak_texture", "gradient_texture", "particle_sprite"):
        add(f"        {field}: {quote(str(warp_portal[field]))},")
    for field in ("skill_radius", "skill_height"):
        add(f"        {field}: {ron.number(warp_portal[field])},")
    add(f"        skill_ring_count: {warp_portal['skill_ring_count']},")
    add("    ),")
    add("    // Level-99 / level-up aura textures.")
    add("    level_aura: (")
    level_aura = data["level_aura"]
    assert isinstance(level_aura, dict)
    for field in ("ring_liner_texture", "ring_blue_texture", "ring_red_texture", "bubble_texture"):
        add(f"        {field}: {quote(str(level_aura[field]))},")
    add("        glow_textures: [")
    for texture in level_aura["glow_textures"]:  # type: ignore[union-attr]
        add(f"            {quote(str(texture))},")
    add("        ],")
    add(f"        sparkle_texture: {quote(str(level_aura['sparkle_texture']))},")
    add("    ),")
    add(")")
    add("")
    return "\n".join(lines)


def validate_assets(data: dict[str, object], game_dir: Path) -> list[str]:
    """Check that every asset referenced by the data file exists in the archives."""
    spec = importlib.util.spec_from_file_location("gen_skill_effect_assets", SCRIPT_DIR / "gen_skill_effect_assets.py")
    if spec is None or spec.loader is None:
        return ["cannot load tools/gen_skill_effect_assets.py"]
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    names: list[str] = []
    for archive in ("CondaRO.grf", "data.grf", "palettes.grf"):
        path = game_dir / archive
        if path.is_file():
            names.extend(module.read_grf_names(path))
    if not names:
        return [f"no GRF archives found in {game_dir}"]
    available = {name.replace("/", "\\").lower() for name in names}

    errors: list[str] = []
    textures: list[str] = []
    for row in data["clouds"]:  # type: ignore[union-attr]
        textures.extend(str(texture) for texture in row["textures"])
    textures.extend(
        str(data["cast_circle"][field])  # type: ignore[index]
        for field in ("ring_texture", "glow_texture")
    )
    textures.extend(
        str(data["warp_portal"][field])  # type: ignore[index]
        for field in ("streak_texture", "gradient_texture")
    )
    textures.extend(
        str(data["level_aura"][field])  # type: ignore[index]
        for field in ("ring_liner_texture", "ring_blue_texture", "ring_red_texture", "bubble_texture", "sparkle_texture")
    )
    textures.extend(str(texture) for texture in data["level_aura"]["glow_textures"])  # type: ignore[index]
    for texture in sorted(set(textures)):
        candidate = f"data\\texture\\{texture}".replace("/", "\\").lower()
        if candidate not in available:
            errors.append(f"missing texture {candidate}")
    particle = str(data["warp_portal"]["particle_sprite"])  # type: ignore[index]
    candidate = f"data\\sprite\\{particle}".replace("/", "\\").lower()
    if candidate not in available:
        errors.append(f"missing sprite {candidate}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--custom", default=str(DEFAULT_CUSTOM), help="hand-written skill behaviour data")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="generated runtime data file")
    parser.add_argument("--validate-assets", default=None, metavar="GAME_DIR", help="validate every referenced asset against the GRF archives")
    args = parser.parse_args()

    data = parse_custom(Path(args.custom))
    content = render(data)
    output = Path(args.output)

    if args.validate_assets:
        errors = validate_assets(data, Path(args.validate_assets))
        if errors:
            for error in errors:
                print(f"error: {error}", file=sys.stderr)
            return 1

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skills.py", file=sys.stderr)
            return 1
        behaviours = len(data["behaviours"])  # type: ignore[arg-type]
        clouds = len(data["clouds"])  # type: ignore[arg-type]
        print(f"{output} is up to date ({behaviours} behaviour rows, {clouds} cloud rows)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    behaviours = len(data["behaviours"])  # type: ignore[arg-type]
    clouds = len(data["clouds"])  # type: ignore[arg-type]
    print(f"{output}: {behaviours} behaviour rows, {clouds} cloud rows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
