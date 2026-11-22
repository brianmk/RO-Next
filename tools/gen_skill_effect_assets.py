#!/usr/bin/env python3
"""Generate the skill -> completion-effect asset table used by the client.

The committed table (`korangar/src/world/library/skill_effect_assets.rs`) maps a
skill id to an official `.str` effect file from `data\\texture\\effect\\`, so the
client can play the real completion animation (the way `black_hammerfall.str`
plays the Hammer Fall hammer layer) instead of only the generic element-colored
cast circle.

Only **high-confidence** matches are generated, exactly the same matching rule
the audit used (see `notes/korangar-skill-effect-audit.md`):

* the skill's Aegis name (`WZ_STORMGUST`) normalized, or
* the skill's description (`Storm Gust`) normalized,

equals the effect file name (normalized) with common effect suffixes
(`_hit`, `_cast`, `_bottom`, `_ground`, ...) stripped. Prefix-stripped matches
(e.g. `MG_FIREBOLT` -> `firebolt`) are deliberately *not* generated: they are
the medium-confidence batch that still needs a manual review.

Every generated asset is checked against the real GRF archives; generation
fails if a mapped `.str` does not exist. Skills without a high-confidence match
are simply absent from the table and keep the current generic cast circle.

Usage:
    ./tools/gen_skill_effect_assets.py                  # regenerate the committed file
    ./tools/gen_skill_effect_assets.py --check          # fail if the file is out of date
    ./tools/gen_skill_effect_assets.py --ssh myhost --server-root /home/phantom/rathena
    ./tools/gen_skill_effect_assets.py --local /path/to/rathena --game-dir /path/to/client/game

Only read-only commands are run against the server tree.
"""

from __future__ import annotations

import argparse
import re
import struct
import subprocess
import sys
import zlib
from pathlib import Path

DEFAULT_SSH = "rathena"
DEFAULT_SERVER_ROOT = "/home/phantom/rathena"
DEFAULT_GAME_DIR = "/home/phantom/Games/ragnarok/CondaRO-linux/client/game"
DEFAULT_OUTPUT = Path("korangar/src/world/library/skill_effect_assets.rs")
SKILL_DB = "db/pre-re/skill_db.yml"
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
EFFECT_PREFIX = "data\\texture\\effect\\"

# Effect-file suffixes that are stripped to build the "base" name of an effect.
# Longer suffixes first so `_cast_bottom` is stripped as a whole.
EFFECT_SUFFIXES = (
    "_cast_bottom",
    "_cast",
    "_hit_bottom",
    "_hit",
    "_ground_bottom",
    "_ground",
    "_bottom",
    "_attack",
    "_waiting",
    "_loop",
    "_start",
    "_end",
    "_top",
    "_down",
    "_line",
    "_ring",
    "_wave",
    "_fire",
    "_water",
    "_wind",
    "_earth",
    "_dark",
    "_holy",
    "_a",
    "_b",
    "_c",
    "_d",
    "_1",
    "_2",
    "_3",
)

# Hand-curated additions where the effect name is clearly the same skill but the
# normalized name differs from the Aegis name / description (kept explicit so the
# table can still be regenerated and reviewed). Magnus Exorcismus -> `magnus.str`.
CURATED_EXTRAS = (
    (79, "magnus.str", "PR_MAGNUS"),
)

GRF_HEADER_SIZE = 46


def normalize(text: str) -> str:
    return re.sub(r"[^a-z0-9]", "", text.lower())


def strip_effect_suffix(stem: str) -> tuple[str, int]:
    """Return the suffix-free effect base name and the number of stripped suffixes."""
    base = stem.lower()
    stripped = 0
    changed = True
    while changed:
        changed = False
        for suffix in EFFECT_SUFFIXES:
            if base.endswith(suffix) and len(base) - len(suffix) >= 3:
                base = base[: -len(suffix)]
                stripped += 1
                changed = True
                break
    return base, stripped


def read_grf_names(path: Path) -> list[str]:
    """List the file names of a GRF v0x200 archive (table only, no decryption)."""
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
        offset += 17  # file entry header after the name
        try:
            names.append(raw.decode("utf-8"))
        except UnicodeDecodeError:
            try:
                names.append(raw.decode("euc-kr"))
            except UnicodeDecodeError:
                names.append(raw.decode("latin-1"))
    return names


def collect_effect_assets(game_dir: Path, archive_dir: Path | None) -> dict[str, str]:
    """Return normalized `.str` stem -> real file name for every effect in the data.

    The normalized stem is used for matching; the real file name is what the
    client loads. The dictionary key is the full normalized stem (suffixes kept)
    and every suffix-stripped base is added as an alias by the caller.
    """
    assets: dict[str, str] = {}
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if not lower.startswith(EFFECT_PREFIX) or not lower.endswith(".str"):
                continue
            file_name = name.rsplit("\\", 1)[-1]
            stem = file_name[: -len(".str")]
            assets.setdefault(normalize(stem), file_name)
    if archive_dir is not None:
        effect_dir = archive_dir / "data" / "texture" / "effect"
        if effect_dir.is_dir():
            for path in effect_dir.rglob("*.str"):
                stem = path.name[: -len(".str")]
                assets.setdefault(normalize(stem), path.name)
    return assets


def parse_skill_db(text: str) -> list[dict]:
    skills: list[dict] = []
    current: dict | None = None
    for line in text.splitlines():
        match = re.match(r"\s*- Id:\s*(\d+)", line)
        if match:
            if current is not None:
                skills.append(current)
            current = {"id": int(match.group(1))}
            continue
        if current is None:
            continue
        match = re.match(r"\s+Name:\s*(\S+)", line)
        if match and "name" not in current:
            current["name"] = match.group(1)
            continue
        match = re.match(r"\s+Description:\s*(.+?)\s*$", line)
        if match and "description" not in current:
            current["description"] = match.group(1)
    if current is not None:
        skills.append(current)
    return [skill for skill in skills if "name" in skill]


def fetch_skill_db(ssh_host: str | None, server_root: str, local: str | None) -> str:
    path = f"{server_root}/{SKILL_DB}" if local is None else f"{local}/{SKILL_DB}"
    if ssh_host is None:
        return Path(path).read_text()
    result = subprocess.run(["ssh", ssh_host, "cat", path], capture_output=True, text=True, check=True)
    return result.stdout


def match_skills(skills: list[dict], assets: dict[str, str]) -> list[tuple[int, str, str]]:
    """Return the high-confidence (id, effect file, match kind) list."""
    # Build the lookup: normalized stem -> real file name, plus the suffix-stripped
    # base as an alias (the base maps to the *shortest* real file name so
    # `stormgust` wins over `stormgust_cast` when both exist).
    lookups: dict[str, tuple[str, int]] = {}
    for normalized_stem, file_name in assets.items():
        lookups.setdefault(normalized_stem, (file_name, 0))
    for normalized_stem, file_name in assets.items():
        base, stripped = strip_effect_suffix(normalized_stem)
        previous = lookups.get(base)
        if previous is None or stripped < previous[1]:
            lookups[base] = (file_name, stripped)

    existing = set(assets.values())
    matched: list[tuple[int, str, str]] = []
    for skill in skills:
        aegis = normalize(skill["name"])
        description = normalize(skill.get("description", ""))

        candidates: list[tuple[int, int, int, str, str]] = []  # (kind, suffixes, length, file, kind_name)
        for key, kind in ((aegis, "aegis"), (description, "description")):
            if not key:
                continue
            hit = lookups.get(key)
            if hit is None:
                continue
            file_name, stripped = hit
            exact = normalize(file_name[: -len(".str")]) == key
            candidates.append((0 if exact and kind == "aegis" else 1, stripped, len(file_name), file_name, kind))

        if not candidates:
            continue

        candidates.sort()
        _, _, _, file_name, kind = candidates[0]
        matched.append((skill["id"], file_name, kind))

    # Curated extras (see CURATED_EXTRAS); only added when the file really exists.
    known_ids = {skill_id for skill_id, _, _ in matched}
    for skill_id, asset, _aegis in CURATED_EXTRAS:
        if skill_id in known_ids or asset not in existing:
            continue
        matched.append((skill_id, asset, "curated"))

    matched.sort()
    return matched


def render(rows: list[tuple[int, str]]) -> str:
    lines = [
        "//! Generated skill -> completion effect asset table.",
        "//!",
        "//! Maps a skill id to the official `.str` effect file in",
        "//! `data\\texture\\effect\\` that is played when the cast completes. Only",
        "//! high-confidence matches are generated (Aegis name or description equals",
        "//! the effect name, see `notes/korangar-skill-effect-audit.md`); every file",
        "//! was verified to exist in the CondaRO/data GRF archives.",
        "//!",
        "//! Generated by `tools/gen_skill_effect_assets.py`; do not edit by hand.",
        "//! Regenerate with: `./tools/gen_skill_effect_assets.py`",
        "//! Drift check:      `./tools/gen_skill_effect_assets.py --check`",
        "",
        "use ragnarok_packets::SkillId;",
        "",
        "/// Skill id to effect file name inside `data\\texture\\effect\\`, sorted by",
        "/// skill id so the lookup is a binary search.",
        "#[rustfmt::skip]",
        "pub(crate) static SKILL_EFFECT_ASSETS: &[(u16, &str)] = &[",
    ]
    lines.extend(f'    ({skill_id}, "{asset}"),' for skill_id, asset in rows)
    lines.extend(
        [
            "];",
            "",
            "/// Returns the official completion effect of `skill_id`, if the client",
            "/// has a high-confidence asset for it.",
            "pub(crate) fn skill_effect_asset(skill_id: SkillId) -> Option<&'static str> {",
            "    SKILL_EFFECT_ASSETS",
            "        .binary_search_by_key(&skill_id.0, |(id, _)| *id)",
            "        .ok()",
            "        .map(|position| SKILL_EFFECT_ASSETS[position].1)",
            "}",
            "",
            "#[cfg(test)]",
            "mod tests {",
            "    use ragnarok_packets::SkillId;",
            "",
            "    use super::{SKILL_EFFECT_ASSETS, skill_effect_asset};",
            "",
            "    #[test]",
            "    fn table_is_sorted_and_unique() {",
            "        assert!(SKILL_EFFECT_ASSETS.windows(2).all(|window| window[0].0 < window[1].0));",
            "        assert!(!SKILL_EFFECT_ASSETS.is_empty());",
            "    }",
            "",
            "    #[test]",
            "    fn known_skills_have_effects_and_unknown_skills_do_not() {",
            "        // Storm Gust, Magnus Exorcismus and Pierce are high-confidence matches.",
            "        assert_eq!(skill_effect_asset(SkillId(89)), Some(\"stormgust.str\"));",
            "        assert_eq!(skill_effect_asset(SkillId(79)), Some(\"magnus.str\"));",
            "        assert_eq!(skill_effect_asset(SkillId(56)), Some(\"pierce.str\"));",
            "        // Skill 1 (Basic Skill) has no completion effect asset.",
            "        assert_eq!(skill_effect_asset(SkillId(1)), None);",
            "    }",
            "}",
        ]
    )
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed table is out of date")
    parser.add_argument("--ssh", default=DEFAULT_SSH, help=f"SSH host for the rAthena tree (default: {DEFAULT_SSH})")
    parser.add_argument("--server-root", default=DEFAULT_SERVER_ROOT, help="rAthena tree on the server")
    parser.add_argument("--local", default=None, help="local rAthena tree instead of SSH")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR, help="client game directory with the GRF archives")
    parser.add_argument("--archive-dir", default=None, help="optional loose `archive/` directory (folder loader)")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")

    skill_db = fetch_skill_db(None if args.local else args.ssh, args.server_root, args.local)
    skills = parse_skill_db(skill_db)
    assets = collect_effect_assets(game_dir, Path(args.archive_dir) if args.archive_dir else None)
    if not assets:
        raise SystemExit(f"no .str effect files found in the archives under {game_dir}")

    matched = match_skills(skills, assets)

    # Fail generation if a mapped asset does not exist in the archives (can only
    # happen if the matching rules change) or if the table is empty.
    existing = set(assets.values())
    for skill_id, asset, _ in matched:
        if asset not in existing:
            raise SystemExit(f"skill {skill_id}: mapped effect does not exist in the archives: {asset}")
    if not matched:
        raise SystemExit("no high-confidence skill effect assets found")

    rows = [(skill_id, asset) for skill_id, asset, _ in matched]
    output = Path(args.output)
    content = render(rows)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_effect_assets.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} skills)")
        return 0

    output.write_text(content)
    print(f"{output}: {len(rows)} skills mapped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
