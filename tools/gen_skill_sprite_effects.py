#!/usr/bin/env python3
"""Generate the client-side skill -> sprite effect (`.spr`/`.act`) mapping.

Some official skill visuals are short-lived animated sprites from
`data\\sprite\\이팩트\\...` (`.spr` + `.act` pairs) instead of the screen-space
`.str` effects. The Windows client plays them through its hard-coded effect
table; this generator extracts the facts for those entries and validates every
referenced file against the shipped CondaRO data.

The mapping is shipped as a **data file** in the client archive:

    korangar/archive/data/client/skill_sprite_effects.ron

so it is loaded through the regular `GameFileLoader` (the same loader that
reads the GRFs) and can be overridden by a server without rebuilding the
client. The generated Rust fallback embeds the same file with `include_str!`,
so both stay in sync automatically. Format: see
`korangar/src/world/library/skill_sprite_effects.rs`.

Sources (facts only, no GPL code):

* `tools/windows_client_skill_effects.json` - numeric skill -> effect id mapping
  per phase (already used by `gen_skill_effect_assets.py`).
* `tools/windows_client_sprite_effects.json` - the `type: 'SPR'` effect table
  entries (file names, flags, duration/yOffset parameters) extracted from the
  Windows client tables (`extract_windows_client_skill_sprite_effects.py`).
* `tools/windows_client_spr_whitelist.txt` - the 152 `.spr` effect paths
  referenced by `CondaRO.exe` (the sprite counterpart of the `.str` whitelist).
* `tools/windows_client_skill_sprite_name_matches.json` - curated, high
  confidence skill-name matches for effects that the extracted skill->effect
  table does not map (mostly the `su_*` New World set); every file in it must
  still be part of the official `CondaRO.exe` sprite whitelist.

Validation (same pattern as the `.str` generator):

1. the resolved relative path must contain a `.spr` **and** an `.act` in the
   real GRF archives,
2. name-matched rows additionally have to be in the Windows client whitelist,
3. duplicate rows are removed, unknown/missing assets are dropped (the client
   then falls back to the generic element cast circle - it never panics).

Usage:
    ./tools/gen_skill_sprite_effects.py                    # regenerate the data file
    ./tools/gen_skill_sprite_effects.py --check            # fail if the file is out of date
    ./tools/gen_skill_sprite_effects.py --report           # print coverage/counts
    ./tools/gen_skill_sprite_effects.py --game-dir /path/to/client/game
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
DEFAULT_EFFECTS = Path("tools/windows_client_skill_effects.json")
DEFAULT_SPRITE_EFFECTS = Path("tools/windows_client_sprite_effects.json")
DEFAULT_WHITELIST = Path("tools/windows_client_spr_whitelist.txt")
DEFAULT_NAME_MATCHES = Path("tools/windows_client_skill_sprite_name_matches.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_sprite_effects.ron")
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
SPRITE_ROOT = "data\\sprite\\"
EFFECT_ROOT = "이팩트"
GRF_HEADER_SIZE = 46

# Phase group of the extracted skill table -> RON variant name.
VARIANTS = {
    "ground": "Ground",
    "cast": "Cast",
    "success": "Success",
    "release": "Release",
    "hit": "Hit",
    "caster": "Caster",
}

# A sprite attached to the actor's head is rendered around the head instead of
# the feet. The Windows client offsets it in screen pixels (`head`, `yOffset`);
# the values below convert that to the world-space offset used by the client
# (approximation: one world unit is roughly four pixels on the classic camera).
HEAD_WORLD_OFFSET = 30.0
PIXEL_TO_WORLD = 0.25


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


def collect_sprite_assets(game_dir: Path) -> tuple[dict[str, str], dict[str, str]]:
    """Return (spr, act) maps of lowercase relative path -> real path.

    The relative path is relative to `data\\sprite\\` and keeps the original
    extension, for example `이팩트\\su_cn_meteor\\su_cn_meteor.spr`.
    """
    sprites: dict[str, str] = {}
    actions: dict[str, str] = {}
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if not lower.startswith(SPRITE_ROOT):
                continue
            relative = name[len(SPRITE_ROOT):]
            if lower.endswith(".spr"):
                sprites.setdefault(relative.lower(), relative)
            elif lower.endswith(".act"):
                actions.setdefault(relative.lower(), relative)
    return sprites, actions


def decode_game_name(file_name: str) -> str:
    """Decode a roBrowser file name (EUC-KR bytes written as `\\xNN` escapes)."""
    try:
        raw = re.sub(r"\\x([0-9a-fA-F]{2})", lambda match: chr(int(match.group(1), 16)), file_name).encode("latin-1")
    except (UnicodeEncodeError, ValueError):
        return file_name
    for encoding in ("euc-kr", "utf-8"):
        try:
            return raw.decode(encoding)
        except UnicodeDecodeError:
            continue
    return file_name


def normalize_sprite_stem(file_name: str) -> str | None:
    """Convert an effect-table `file` value into a `data\\sprite\\` stem (no extension)."""
    decoded = decode_game_name(file_name).replace("/", "\\").strip()
    if not decoded or decoded.startswith(".."):
        return None
    decoded = decoded.lstrip("\\")
    if decoded.lower().endswith((".spr", ".act")):
        decoded = decoded.rsplit(".", 1)[0]
    if decoded.lower().endswith((".tga", ".bmp", ".jpg", ".png")):
        return None
    return decoded


def resolve_sprite(stem: str, sprites: dict[str, str], actions: dict[str, str]) -> str | None:
    """Find the real relative path (without extension) of an existing `.spr`+`.act` pair."""
    candidates = [f"{EFFECT_ROOT}\\{stem}", stem]
    for candidate in candidates:
        key = candidate.lower()
        if f"{key}.spr" in sprites and f"{key}.act" in actions:
            return candidate
    return None


def load_whitelist(path: Path) -> set[str]:
    """Return the lowercase `data\\sprite\\`-relative stems of the official effect sprites."""
    allowed: set[str] = set()
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "%" in line:
            continue
        if line.startswith("\\"):
            stem = f"{EFFECT_ROOT}\\{line[1:]}"
        elif line.lower().startswith(EFFECT_ROOT.lower() + "\\"):
            stem = line
        else:
            # Non-effect sprite roots (NPC/, homun/, roullette/, ...) are not
            # part of the effect-sprite whitelist.
            continue
        if stem.lower().endswith(".spr"):
            stem = stem[:-4]
        allowed.add(stem.lower())
    return allowed


def load_json(path: Path) -> dict:
    return json.loads(path.read_text())


def anchor_for_variant(variant: str, attached_entity: bool) -> str:
    """Choose the spawn anchor of an effect-table row.

    The extracted table only tells us when a sprite plays and whether it is
    attached to an entity. Ground skills are always placed at the target cell;
    hit/release effects land on the target; success/cast/caster effects are
    attached to the actor (or to the target when the official entry is a free
    standing sprite).
    """
    if variant == "Ground":
        return "Ground"
    if variant in ("Hit", "Release"):
        return "Target"
    return "Caster" if attached_entity else "Target"


def offset_for_entry(entry: dict) -> tuple[float, float, float]:
    height = 0.0
    if entry.get("head"):
        height += HEAD_WORLD_OFFSET
    y_offset = entry.get("yOffset")
    if isinstance(y_offset, (int, float)) and y_offset != 0:
        # The table value is in screen pixels, negative = above the actor.
        height = max(0.0, -float(y_offset) * PIXEL_TO_WORLD)
    return (0.0, round(height, 2), 0.0)


def duration_for_entry(entry: dict) -> int:
    duration = entry.get("duration")
    if isinstance(duration, (int, float)) and duration > 0:
        return int(duration)
    return 0


def build_rows(
    effects: dict,
    sprite_effects: dict,
    name_matches: dict,
    sprites: dict[str, str],
    actions: dict[str, str],
    whitelist: set[str],
) -> tuple[list[dict], dict[str, list[str]]]:
    rows: list[dict] = []
    dropped: dict[str, list[str]] = {"missing_asset": [], "not_whitelisted": [], "unknown_effect": []}

    def add(row: dict) -> None:
        rows.append(row)

    # 1. Authoritative rows: skill -> effect id (from the extracted skill table)
    #    -> first `type: 'SPR'` object of that effect that exists in the GRFs.
    for skill_text, fields in sorted(effects["skills"].items(), key=lambda item: int(item[0])):
        skill_id = int(skill_text)
        for group, variant in VARIANTS.items():
            effect_ids = fields.get(group, [])
            added = False
            for effect_id in effect_ids:
                entries = sprite_effects.get(str(effect_id))
                if not entries:
                    dropped["unknown_effect"].append(f"skill {skill_id} {variant}: effect {effect_id} is not an SPR entry")
                    continue
                for entry in entries:
                    stem = normalize_sprite_stem(entry.get("file", ""))
                    if stem is None:
                        continue
                    resolved = resolve_sprite(stem, sprites, actions)
                    if resolved is None:
                        dropped["missing_asset"].append(f"skill {skill_id} {variant}: {stem}")
                        continue
                    add(
                        {
                            "skill": skill_id,
                            "variant": variant,
                            "anchor": anchor_for_variant(variant, entry.get("attachedEntity", False)),
                            "files": [resolved],
                            "offset": offset_for_entry(entry),
                            "scale": 1.0,
                            "duration_ms": duration_for_entry(entry),
                            "source": "effect-table",
                        }
                    )
                    added = True
                    break
                if added:
                    break

    # 2. Curated high-confidence name matches (mostly `su_*` New World skills
    #    that the extracted skill table leaves empty). These must be part of the
    #    official `CondaRO.exe` sprite whitelist.
    for match in name_matches["matches"]:
        skill_id = int(match["skill"])
        variant = match["variant"]
        files: list[str] = []
        rejected: list[str] = []
        for file_name in match["files"]:
            stem = normalize_sprite_stem(file_name)
            if stem is None:
                rejected.append(file_name)
                continue
            resolved = resolve_sprite(stem, sprites, actions)
            if resolved is None:
                dropped["missing_asset"].append(f"skill {skill_id} {variant} (name match): {stem}")
                continue
            if resolved.lower() not in whitelist:
                dropped["not_whitelisted"].append(f"skill {skill_id} {variant}: {resolved}")
                continue
            files.append(resolved)
        if rejected:
            raise SystemExit(f"skill {skill_id} {variant}: invalid sprite file name in name-match table: {rejected}")
        if not files:
            continue
        offset = match.get("offset", [0.0, 0.0, 0.0])
        add(
            {
                "skill": skill_id,
                "variant": variant,
                "anchor": match.get("anchor", "Caster"),
                "files": files,
                "offset": (float(offset[0]), float(offset[1]), float(offset[2])),
                "scale": float(match.get("scale", 1.0)),
                "duration_ms": int(match.get("duration_ms", 0)),
                "source": "name-match",
            }
        )

    # 3. Remove duplicates and sort for the binary-search lookup in the client.
    unique: dict[tuple, dict] = {}
    for row in rows:
        key = (row["skill"], row["variant"], tuple(row["files"]), row["source"])
        unique.setdefault(key, row)
    ordered = sorted(unique.values(), key=lambda row: (row["skill"], row["variant"], row["files"], row["source"]))
    return ordered, dropped


def escape_ron(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def render(rows: list[dict]) -> str:
    lines = [
        "// Official skill -> sprite effect (`.spr`/`.act`) mapping for the client.",
        "//",
        "// Generated by `tools/gen_skill_sprite_effects.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_sprite_effects.py`",
        "// Drift check:      `./tools/gen_skill_sprite_effects.py --check`",
        "//",
        "// Format: one row per (skill, phase, sprite pair). `files` are candidate",
        "// stems relative to data\\sprite\\ (without the .spr/.act extension); the",
        "// first pair that exists in the archives is played once and the effect is",
        "// removed. `duration_ms: 0` uses the native ACT frame timing, a positive",
        "// value stretches the animation to that duration.",
        "// Phases: Ground, Cast, Success, Release, Hit, Caster (see the loader in",
        "// `korangar/src/world/library/skill_sprite_effects.rs`).",
        "(",
        '    provenance: "Windows client SPR effect table (roBrowserLegacy RE dump) + name-matched official sprite whitelist, validated against the CondaRO GRFs",',
        "    effects: [",
    ]
    for row in rows:
        files = ", ".join(f'"{escape_ron(file)}"' for file in row["files"])
        x, y, z = row["offset"]
        lines.append(
            "        ("
            f"skill: {row['skill']}, variant: {row['variant']}, anchor: {row['anchor']}, "
            f"files: [{files}], offset: ({x}, {y}, {z}), scale: {row['scale']}, "
            f"duration_ms: {row['duration_ms']}"
            "),"
        )
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--report", action="store_true", help="print coverage/counts")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--effects", default=str(DEFAULT_EFFECTS))
    parser.add_argument("--sprite-effects", default=str(DEFAULT_SPRITE_EFFECTS))
    parser.add_argument("--whitelist", default=str(DEFAULT_WHITELIST))
    parser.add_argument("--name-matches", default=str(DEFAULT_NAME_MATCHES))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")

    effects = load_json(Path(args.effects))
    sprite_effects = load_json(Path(args.sprite_effects))
    name_matches = load_json(Path(args.name_matches))
    whitelist = load_whitelist(Path(args.whitelist))
    sprites, actions = collect_sprite_assets(game_dir)
    if not sprites or not actions:
        raise SystemExit(f"no .spr/.act files found in the archives under {game_dir}")

    rows, dropped = build_rows(
        effects,
        sprite_effects["effects"],
        name_matches,
        sprites,
        actions,
        whitelist,
    )
    if not rows:
        raise SystemExit("no sprite effects could be resolved against the archives/whitelist")

    # Defensive re-validation of the final rows: every referenced pair must
    # exist, name-matched rows must be official.
    for row in rows:
        for file in row["files"]:
            key = file.lower()
            if f"{key}.spr" not in sprites or f"{key}.act" not in actions:
                raise SystemExit(f"skill {row['skill']} {row['variant']}: sprite pair does not exist: {file}")
            if row["source"] == "name-match" and key not in whitelist:
                raise SystemExit(f"skill {row['skill']} {row['variant']}: sprite is not in the Windows client whitelist: {file}")

    content = render(rows)
    output = Path(args.output)

    if args.report:
        skills = {row["skill"] for row in rows}
        variants: dict[str, int] = {}
        sources: dict[str, int] = {}
        for row in rows:
            variants[row["variant"]] = variants.get(row["variant"], 0) + 1
            sources[row["source"]] = sources.get(row["source"], 0) + 1
        print(f"rows: {len(rows)} ({len(skills)} skills)")
        print(f"sources: {dict(sorted(sources.items()))}")
        print(f"variants: {dict(sorted(variants.items()))}")
        print(f"anchors: {dict(sorted((anchor, sum(1 for row in rows if row['anchor'] == anchor)) for anchor in {row['anchor'] for row in rows}))}")
        drop_counts = {key: len(value) for key, value in dropped.items() if value}
        if drop_counts:
            print(f"dropped: {drop_counts}")
        for key, values in dropped.items():
            for value in values[:20]:
                print(f"  drop[{key}]: {value}")

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_sprite_effects.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} entries)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    print(f"{output}: {len(rows)} entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
