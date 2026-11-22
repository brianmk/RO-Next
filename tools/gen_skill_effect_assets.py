#!/usr/bin/env python3
"""Generate the client-side skill -> effect asset mapping.

The mapping is shipped as a **data file** in the client archive:

    korangar/archive/data/client/skill_effect_assets.ron

so it is loaded through the regular `GameFileLoader` (the same loader that reads
the GRFs) and can be overridden by a server without rebuilding the client. The
generated Rust fallback embeds the same file with `include_str!`, so both stay
in sync automatically.

Source data:

* `tools/windows_client_skill_effects.json` - numeric skill/effect ids and
  effect file names extracted from the Windows client tables
  (`tools/extract_windows_client_skill_effects.py`).
* `tools/windows_client_effect_whitelist.txt` - the 401 `.str` effect paths
  referenced by `CondaRO.exe` (extracted from the shipped Windows client).

Every referenced file is validated against

1. the real GRF archives (`data\\texture\\effect\\` must contain the file), and
2. the Windows client effect whitelist (the file must be part of the official
   client effect set; `%d`/`%s` templates are supported).

Entries whose effect resource is not present in the CondaRO data set are
dropped, so the client falls back to the generic element cast circle for those
skills.

File format (RON, see `SkillEffectAssetFile` in
`korangar/src/world/library/skill_effect_assets.rs`):

    (
        provenance: "...",
        effects: [
            (skill: 89, variant: Ground, file: "stormgust.str"),
            ...
        ],
    )

Variants tell the client *when* to play the effect:

* `Ground`  - ground-target skill completion, at the target cell
* `Cast`    - when a cast starts, on the caster
* `Success` - when a no-damage/successful use completes, on the caster
* `Release` - when a targeted skill lands, at the target
* `Hit`     - on skill damage hits, at the target
* `Caster`  - on the caster when its skill damage packet arrives

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
DEFAULT_EFFECTS = Path("tools/windows_client_skill_effects.json")
DEFAULT_WHITELIST = Path("tools/windows_client_effect_whitelist.txt")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_effect_assets.ron")
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
EFFECT_PREFIX = "data\\texture\\effect\\"

# JSON field group -> RON variant name used by the client.
VARIANTS = {
    "ground": "Ground",
    "cast": "Cast",
    "success": "Success",
    "release": "Release",
    "hit": "Hit",
    "caster": "Caster",
}

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
    for suffix in (".str",):
        path = f"{candidate}{suffix}".lower()
        if path in assets:
            return assets[path]
    return None


def load_effects(path: Path) -> dict:
    payload = json.loads(path.read_text())
    skills = {int(skill_id): fields for skill_id, fields in payload["skills"].items()}
    effects = {int(effect_id): files for effect_id, files in payload["effects"].items()}
    return {"skills": skills, "effects": effects}


def build_rows(effects: dict, assets: dict[str, str], whitelist: tuple[set[str], list[str]]) -> list[tuple[int, str, str]]:
    concrete, templates = whitelist
    rows: list[tuple[int, str, str]] = []
    skipped: list[tuple[int, str, str]] = []

    def files_for(effect_ids: list[int]) -> list[str]:
        resolved: list[str] = []
        for effect_id in effect_ids:
            for entry in effects["effects"].get(effect_id, ()):
                if entry.get("type") not in (None, "STR"):
                    continue
                file_name = entry["file"]
                resolved_path = resolve_effect_file(file_name, assets)
                if resolved_path is None:
                    continue
                if not whitelist_allows(resolved_path, concrete, templates):
                    continue
                if resolved_path not in resolved:
                    resolved.append(resolved_path)
        return resolved

    for skill_id, fields in sorted(effects["skills"].items()):
        for group, variant in VARIANTS.items():
            for file in files_for(fields.get(group, [])):
                rows.append((skill_id, variant, file))
                # The first resolvable file of a group is enough for our client;
                # further variants of the same group would spawn on top of it.
                break

    rows.sort(key=lambda row: (row[0], row[1], row[2]))
    return rows


def render(rows: list[tuple[int, str, str]]) -> str:
    lines = [
        "// Official skill -> effect mapping for the client.",
        "//",
        "// Generated by `tools/gen_skill_effect_assets.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_effect_assets.py`",
        "// Drift check:      `./tools/gen_skill_effect_assets.py --check`",
        "//",
        "// Format: (skill id, variant, effect path relative to data\\texture\\effect\\)",
        "// Variants: Ground, Cast, Success, Release, Hit, Caster (see the loader in",
        "// `korangar/src/world/library/skill_effect_assets.rs`).",
        "//",
        "// The mapping is loaded at runtime from this data file through the normal",
        "// client file loader, so a server can override it without rebuilding the",
        "// client; the compiled Rust table is only the fallback for missing files.",
        "(",
        '    provenance: "Windows client effect tables (roBrowserLegacy RE dump), validated against CondaRO.exe and the GRFs",',
        "    effects: [",
    ]
    for skill_id, variant, file in rows:
        escaped = file.replace("\\", "\\\\")
        lines.append(f'        (skill: {skill_id}, variant: {variant}, file: "{escaped}"),')
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

    effects = load_effects(Path(args.effects))
    assets = collect_effect_assets(game_dir, Path(args.archive_dir) if args.archive_dir else None)
    if not assets:
        raise SystemExit(f"no .str effect files found in the archives under {game_dir}")
    whitelist = load_whitelist(Path(args.whitelist))

    rows = build_rows(effects, assets, whitelist)
    if not rows:
        raise SystemExit("no official skill effects could be resolved against the archives/whitelist")

    # Re-validate the final rows (defensive: every file must exist and be official).
    valid_paths = {path.lower() for path in assets.values()}
    for skill_id, _variant, file in rows:
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
