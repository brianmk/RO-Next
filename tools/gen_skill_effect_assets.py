#!/usr/bin/env python3
"""Generate the client-side skill -> effect asset mapping.

The mapping is shipped as a **data file** in the client archive:

    ro-next/archive/data/client/skill_effect_assets.ron

so it is loaded through the regular `GameFileLoader` (the same loader that reads
the GRFs) and can be overridden by a server without rebuilding the client. The
generated Rust fallback embeds the same file with `include_str!`, so both stay
in sync automatically.

Source data:

* `tools/data/windows_client_skill_effects.json` - numeric skill/effect ids, the
  exact official `SkillEffect` field they come from and the effect file names
  extracted from the Windows client tables
  (`tools/extract_windows_client_skill_effects.py`).
* `tools/data/windows_client_effect_whitelist.txt` - the 401 `.str` effect paths
  referenced by `CondaRO.exe` (extracted from the shipped Windows client).

Every referenced file is validated against

1. the real GRF archives (`data\\texture\\effect\\` must contain the file), and
2. the Windows client effect whitelist (the file must be part of the official
   client effect set; `%d`/`%s` templates are supported).

Entries whose effect resource is not present in the CondaRO data set are
dropped, so the client falls back to the generic element cast circle for those
skills.

File format (RON, see `SkillEffectAssetFile` in
`ro-next/src/world/library/skill_effect_assets.rs`):

    (
        provenance: "...",
        effects: [
            (skill: 89, variant: Ground, anchor: Target, file: "stormgust.str"),
            ...
        ],
    )

`variant` tells the client *when* to play the effect:

* `Ground`  - ground-target skill completion
* `Cast`    - when a cast starts
* `Success` - when a no-damage/successful use completes
* `Release` - when a targeted skill lands
* `Hit`     - on skill damage hits
* `Caster`  - on the caster when its skill damage packet arrives

`anchor` tells the client *where* to play it (`Caster`, `Target`, `Auto` = the
target cell when the packet knows one and the caster otherwise, `Travel` = the
target cell, falling back to the caster). It comes from the exact official
field, e.g. `effectIdOnCaster` is anchored on the caster while `effectId` is
aimed at the target.

Usage:
    ./tools/gen_skill_effect_assets.py                     # regenerate the data file
    ./tools/gen_skill_effect_assets.py --check             # fail if the file is out of date
    ./tools/gen_skill_effect_assets.py --local /path/to/rathena --game-dir /path/to/client/game
"""

from __future__ import annotations

import argparse
import json
import re
import struct
import sys
import zlib
from pathlib import Path

DEFAULT_GAME_DIR = "/home/phantom/Games/ragnarok/CondaRO-linux/client/game"
DEFAULT_EFFECTS = Path("tools/data/windows_client_skill_effects.json")
DEFAULT_WHITELIST = Path("tools/data/windows_client_effect_whitelist.txt")
DEFAULT_OUTPUT = Path("ro-next/archive/data/client/skill_effect_assets.ron")
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
EFFECT_PREFIX = "data\\texture\\effect\\"
GRF_HEADER_SIZE = 46

#: Official `SkillEffect` field -> (client variant, spawn anchor).
#:
#: The anchor is what fixes the "on-caster effects played at the target cell"
#: bug: the old generator collapsed fields into coarse groups and only kept the
#: variant, so `effectIdOnCaster` (e.g. Pierce's `pierce.str`) was spawned at
#: the damaged target instead of on the caster.
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

#: Order of the variants in the generated file (matches the client enum).
VARIANT_ORDER = ["Ground", "Cast", "Success", "Release", "Hit", "Caster"]
ANCHOR_ORDER = ["Caster", "Target", "Travel", "Auto"]

#: A single variant layer should not spawn an unbounded number of .str files.
MAX_FILES_PER_VARIANT = 4

#: Buff/support skills whose **entity-anchored completion effect** belongs above
#: the character's head instead of at the feet/torso.
#:
#: `.str` effects are screen sprites drawn around their anchor; when anchored at
#: the feet, buff animations appear at about half character height (Gloria,
#: Status Recovery, Impositio Manus, ...). The client anchors these rows with
#: the shared overhead anchor (`world::skill_cast_effect::overhead_effect_height`):
#: 21 world units standing / 12 sitting, plus clearance, scaled by the entity
#: sprite scale, so it also works for other players and sitting characters.
#:
#: Ground/target effects and on-caster attack effects keep their normal anchor.
OVERHEAD_SKILLS = {
    32,   # AL_CRUCIS (Signum Crucis: caster-centred cross effect)
    35,   # AL_CURE
    45,   # AC_CONCENTRATION (also has a hand-written visual)
    71,   # PR_SLOWPOISON
    54,   # ALL_RESURRECTION
    60,   # KN_TWOHANDQUICKEN
    66,   # PR_IMPOSITIO (Impositio Manus)
    67,   # PR_SUFFRAGIUM
    68,   # PR_ASPERSIO
    72,   # PR_STRECOVERY (Status Recovery)
    73,   # PR_KYRIE
    74,   # PR_MAGNIFICAT
    75,   # PR_GLORIA (Gloria)
    108,  # BS_REPAIRWEAPON
    112,  # BS_WEAPONPERFECT
    114,  # BS_MAXIMIZE
    139,  # AS_POISONREACT
    155,  # MC_LOUD
    157,  # MG_ENERGYCOAT
    255,  # CR_DEVOTION
    256,  # CR_PROVIDENCE
    258,  # CR_SPEARQUICKEN
    276,  # SA_MAGICROD
    361,  # HP_ASSUMPTIO
    384,  # WS_MELTDOWN
    387,  # WS_CARTBOOST
    390,  # ST_REJECTSWORD
    446,  # AM_BERSERKPITCHER
    495,  # KN_ONEHAND
    691,  # CASH_ASSUMPTIO
    698,  # ALL_WEWISH
    2001, # RK_ENCHANTBLADE
    2009, # RK_DRAGONHOWLING
    2011, # RK_MILLENNIUMSHIELD
    5005, # RK_LUXANIMA
    5008, # SO_ELEMENTAL_SHIELD
    5011, # AB_OFFERTORIUM
    5012, # WL_TELEKINESIS_INTENSE
    5014, # ALL_FULL_THROTTLE
    8006, # HAMI_DEFENCE
    8030, # MH_MIDNIGHT_FRENZY
    8222, # MER_MAGNIFICAT
    8238, # MER_KYRIE
}


#: Buff/support skills whose official completion effect is an aura around the
#: **body/weapon** and therefore has to stay at the body instead of moving above
#: the head. These are the data-level exceptions to the overhead default.
BODY_AURA_SKILLS = {
    60,   # KN_TWOHANDQUICKEN - weapon glow
    112,  # BS_WEAPONPERFECT - weapon glow
    114,  # BS_MAXIMIZE - body glow
    139,  # AS_POISONREACT - body tint
    157,  # MG_ENERGYCOAT - body aura
    258,  # CR_SPEARQUICKEN - weapon glow
    276,  # SA_MAGICROD - magic shield around the body
    384,  # WS_MELTDOWN - weapon glow
    387,  # WS_CARTBOOST - cart/body aura
    390,  # ST_REJECTSWORD - stance aura
    495,  # KN_ONEHAND - weapon glow
    2001, # RK_ENCHANTBLADE - weapon enchant
    2011, # RK_MILLENNIUMSHIELD - body shield
    5008, # SO_ELEMENTAL_SHIELD - body shield aura
}


def is_entity_success_effect(variant: str, anchor: str) -> bool:
    """Whether a row is a completion effect spawned on the caster entity."""
    return variant == "Success" and anchor in ("Caster", "Auto")


def is_overhead_effect(skill_id: int, variant: str, anchor: str) -> bool:
    """Whether a data row is an overhead buff completion effect.

    Every caster-anchored buff/support completion effect is overhead by
    default; the body-aura exceptions and non-buff rows are written as explicit
    ``overhead: false`` overrides in the data file.
    """
    return is_entity_success_effect(variant, anchor) and skill_id in OVERHEAD_SKILLS and skill_id not in BODY_AURA_SKILLS

GRF_HEADER_SIZE = 46


def normalize(text: str) -> str:
    return re.sub(r"[^a-z0-9]", "", text.lower())


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


def load_whitelist(path: Path) -> tuple[set[str], list[str]]:
    concrete: set[str] = set()
    templates: list[str] = []
    for line in path.read_text().splitlines():
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


def whitelist_allows(relative_path: str, concrete: set[str], templates: list[str]) -> bool:
    basename = relative_path.rsplit("\\", 1)[-1].rsplit("/", 1)[-1]
    # The whitelist was extracted from the ASCII strings of `CondaRO.exe`, so a
    # Korean file name can never appear in it. Such a file is still official
    # when the official effect table references it and it ships in the GRF
    # archives (there is no other source that could have put it there under that
    # exact name); reject only non-ASCII names that are also absent.
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


def collect_effect_assets(game_dir: Path, archive_dir: Path | None) -> dict[str, str]:
    """Return normalized full effect file name -> path relative to `data\\texture\\effect\\`."""
    assets: dict[str, str] = {}
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if not lower.startswith(EFFECT_PREFIX) or not lower.endswith(".str"):
                continue
            relative = name[len(EFFECT_PREFIX):]
            assets.setdefault(relative.lower(), relative)
    if archive_dir is not None:
        effect_dir = archive_dir / "data" / "texture" / "effect"
        if effect_dir.is_dir():
            for path in effect_dir.rglob("*.str"):
                relative = str(path.relative_to(effect_dir))
                assets.setdefault(relative.lower(), relative)
    return assets


def resolve_effect_file(file_name: str, assets: dict[str, str]) -> str | None:
    """Resolve an EffectTable `file:` value (no extension, maybe `%d`) to a GRF path."""
    candidate = file_name.replace("/", "\\")
    if "%" in candidate:
        pattern = re.escape(candidate).replace(r"%d", r"\d+").replace(r"%s", r".+")
        regex = re.compile(f"^{pattern}\\.str$", re.IGNORECASE)
        for relative in assets.values():
            if regex.match(relative):
                return relative
        return None
    path = f"{candidate}.str".lower()
    return assets.get(path)


def load_facts(path: Path) -> dict:
    payload = json.loads(path.read_text())
    skills: dict[int, list[dict]] = {}
    for skill_id, fields in payload["skills"].items():
        skills[int(skill_id)] = fields
    effects = {int(effect_id): files for effect_id, files in payload.get("effects", {}).items()}
    return {"skills": skills, "effects": effects}


def build_rows(facts: dict, assets: dict[str, str], whitelist: tuple[set[str], list[str]]) -> list[tuple[int, str, str, bool, str]]:
    concrete, templates = whitelist
    rows: dict[tuple[int, str, str, bool, str], None] = {}

    def files_for(effect_ids: list[int]) -> list[str]:
        resolved: list[str] = []
        for effect_id in effect_ids:
            for entry in facts["effects"].get(effect_id, ()):
                if entry.get("type") not in (None, "STR"):
                    continue
                file_name = entry.get("file")
                if not file_name:
                    continue
                resolved_path = resolve_effect_file(file_name, assets)
                if resolved_path is None:
                    continue
                if not whitelist_allows(resolved_path, concrete, templates):
                    continue
                if resolved_path not in resolved:
                    resolved.append(resolved_path)
        return resolved

    for skill_id, fields in sorted(facts["skills"].items()):
        per_variant: dict[str, list[tuple[str, str]]] = {}
        for field in fields:
            spec = FIELD_SPECS.get(field["field"])
            if spec is None:
                continue
            variant, anchor = spec
            for file in files_for(field["effects"]):
                per_variant.setdefault(variant, [])
                entry = (anchor, file)
                if entry not in per_variant[variant]:
                    per_variant[variant].append(entry)
        for variant, entries in per_variant.items():
            # Deterministic order; the client spawns every kept file of a phase.
            entries.sort(key=lambda entry: (ANCHOR_ORDER.index(entry[0]), entry[1]))
            for anchor, file in entries[:MAX_FILES_PER_VARIANT]:
                overhead = is_overhead_effect(skill_id, variant, anchor)
                rows[(skill_id, variant, anchor, overhead, file)] = None

    return sorted(
        rows,
        key=lambda row: (row[0], VARIANT_ORDER.index(row[1]), ANCHOR_ORDER.index(row[2]), not row[3], row[4]),
    )


def render(rows: list[tuple[int, str, str, bool, str]]) -> str:
    lines = [
        "// Official skill -> effect mapping for the client.",
        "//",
        "// Generated by `tools/gen_skill_effect_assets.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_effect_assets.py`",
        "// Drift check:      `./tools/gen_skill_effect_assets.py --check`",
        "//",
        "// Format: (skill id, variant, anchor, overhead, effect path relative to",
        "// data\\texture\\effect\\)",
        "// Variants: Ground, Cast, Success, Release, Hit, Caster",
        "// Anchors:  Caster, Target, Travel (target cell, caster fallback), Auto",
        "//           (target cell when the packet knows one, caster otherwise);",
        "//           see the loader in `ro-next/src/world/library/skill_effect_assets.rs`.",
        "// `overhead` marks buff/support completion effects that are anchored above",
        "// the entity's head (21 units standing / 12 sitting, plus clearance, scaled",
        "// by the entity scale) instead of at the feet; ground/target effects keep",
        "// their normal anchor.",
        "//",
        "// The mapping is loaded at runtime from this data file through the normal",
        "// client file loader, so a server can override it without rebuilding the",
        "// client; the compiled Rust table is only the fallback for missing files.",
        "(",
        '    provenance: "Windows client effect tables (roBrowserLegacy RE dump), validated against CondaRO.exe and the GRFs",',
        "    effects: [",
    ]
    for skill_id, variant, anchor, overhead, file in rows:
        escaped = file.replace("\\", "\\\\")
        if is_entity_success_effect(variant, anchor):
            # Entity-anchored completion effects: overhead buffs plus explicit
            # body/feet overrides (the data-level exception mechanism).
            lines.append(
                f'        (skill: {skill_id}, variant: {variant}, anchor: {anchor}, overhead: {str(overhead).lower()}, file: "{escaped}"),'
            )
        else:
            lines.append(f'        (skill: {skill_id}, variant: {variant}, anchor: {anchor}, file: "{escaped}"),')
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--archive-dir", default=None)
    parser.add_argument("--effects", default=str(DEFAULT_EFFECTS))
    parser.add_argument("--whitelist", default=str(DEFAULT_WHITELIST))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")

    facts = load_facts(Path(args.effects))
    assets = collect_effect_assets(game_dir, Path(args.archive_dir) if args.archive_dir else None)
    if not assets:
        raise SystemExit(f"no .str effect files found in the archives under {game_dir}")
    whitelist = load_whitelist(Path(args.whitelist))

    rows = build_rows(facts, assets, whitelist)
    if not rows:
        raise SystemExit("no official skill effects could be resolved against the archives/whitelist")

    # Re-validate the final rows (defensive: every file must exist and be official).
    valid_paths = {path.lower() for path in assets.values()}
    for skill_id, _variant, _anchor, _overhead, file in rows:
        if file.lower() not in valid_paths:
            raise SystemExit(f"skill {skill_id}: effect does not exist in the archives: {file}")
        if not whitelist_allows(file, *whitelist):
            raise SystemExit(f"skill {skill_id}: effect is not in the Windows client whitelist: {file}")

    content = render(rows)
    output = Path(args.output)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_effect_assets.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} entries)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    print(f"{output}: {len(rows)} entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
