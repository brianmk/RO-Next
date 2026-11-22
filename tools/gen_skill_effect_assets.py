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

Every generated asset is checked against the real GRF archives **and** against
the effect whitelist of the Windows client
(`tools/windows_client_effect_whitelist.txt`): the 401 `.str` effect paths that
`CondaRO.exe` knows. Generation fails if a mapped `.str` does not exist in the
archives or is not part of the official client effect set. A handful of
official names that are not plain name matches (`Concentration.str`,
`Firewall%d.str`, ...) are handled by `CURATED_OVERRIDES`.

The whitelist can be regenerated (read-only) from the shipped Windows client:

    strings -n 6 client/game/CondaRO.exe | grep -E '\\.str$' | sort -u \
        > tools/windows_client_effect_whitelist.txt

Skills without a high-confidence match are simply absent from the table and
keep the current generic cast circle.

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
DEFAULT_WHITELIST = Path("tools/windows_client_effect_whitelist.txt")
DEFAULT_OUTPUT = Path("korangar/src/world/library/skill_effect_assets.rs")
SKILL_DB = "db/pre-re/skill_db.yml"
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
EFFECT_PREFIX = "data\\texture\\effect\\"

# Hand-curated mappings where the official client name is not a plain name match.
# Each value is the effect path relative to `data\\texture\\effect\\`; it must be
# covered by the official whitelist (concrete name or `%d`/`%s` template).
CURATED_OVERRIDES: dict[int, str] = {
    # The Windows client uses `Concentration.str` for both Concentration skills;
    # the data set also ships `ac_concentration.str`, but the official effect is
    # the plain one.
    45: "concentration.str",
    # The official client uses the two-part `Firewall%d.str` template; the first
    # frame set is `firewall1.str`.
    18: "firewall1.str",
    # There is no `LandMine.str` in the official effect set. The only official
    # mine effect is the Rebellion `RL_h_mine\\mine.str`, which is used for both
    # Land Mine skill ids as the closest official asset.
    116: "rl_h_mine\\mine.str",
    8210: "rl_h_mine\\mine.str",
    # `PR_MAGNUS` ("Magnus Exorcismus") does not contain the effect name as a
    # whole, but `magnus.str` is clearly the official effect.
    79: "magnus.str",
}

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


def collect_effect_assets(game_dir: Path, archive_dir: Path | None) -> dict[str, list[str]]:
    """Return normalized effect stem -> effect paths relative to `data\\texture\\effect\\`.

    The paths keep their sub folder (some official effects live there, for example
    `rl_h_mine\\mine.str` or `su_nyanggrass\\su_nyanggrass.str`). Multiple files
    can share the same normalized stem (`maximizepower.str` / `maximize power.str`)
    so the caller can prefer the official spelling.
    """
    assets: dict[str, list[str]] = {}
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if not lower.startswith(EFFECT_PREFIX) or not lower.endswith(".str"):
                continue
            relative = name[len(EFFECT_PREFIX):]
            stem = relative.rsplit("\\", 1)[-1][: -len(".str")]
            assets.setdefault(normalize(stem), []).append(relative)
    if archive_dir is not None:
        effect_dir = archive_dir / "data" / "texture" / "effect"
        if effect_dir.is_dir():
            for path in effect_dir.rglob("*.str"):
                relative = str(path.relative_to(effect_dir))
                stem = path.name[: -len(".str")]
                assets.setdefault(normalize(stem), []).append(relative)
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


def load_whitelist(path: Path) -> tuple[set[str], list[str]]:
    """Return (normalized concrete effect names, raw `%d`/`%s` templates)."""
    if not path.is_file():
        raise SystemExit(f"Windows client effect whitelist not found: {path}")
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
    if not concrete and not templates:
        raise SystemExit(f"empty Windows client effect whitelist: {path}")
    return concrete, templates


def whitelist_allows(relative_path: str, concrete: set[str], templates: list[str]) -> bool:
    basename = relative_path.rsplit("\\", 1)[-1].rsplit("/", 1)[-1]
    stem = basename[: -len(".str")] if basename.lower().endswith(".str") else basename
    if normalize(stem) in concrete:
        return True
    for template in templates:
        pattern = re.escape(template)
        pattern = pattern.replace(r"%d", r"\d+").replace(r"%s", r".+")
        if re.fullmatch(pattern, basename, flags=re.IGNORECASE):
            return True
    return False


def match_skills(
    skills: list[dict],
    assets: dict[str, list[str]],
    whitelist: tuple[set[str], list[str]],
) -> list[tuple[int, str, str]]:
    """Return the high-confidence (id, effect path, match kind) list."""
    concrete, templates = whitelist

    # Build the lookup: normalized stem -> (file, stripped suffix count). Alias
    # entries for suffix-stripped bases keep the *shortest* real file name.
    lookups: dict[str, list[tuple[str, int]]] = {}
    for normalized_stem, files in assets.items():
        for file in files:
            lookups.setdefault(normalized_stem, []).append((file, 0))
    for normalized_stem, files in assets.items():
        base, stripped = strip_effect_suffix(normalized_stem)
        if base == normalized_stem:
            continue
        for file in files:
            previous = lookups.get(base)
            if previous is None or stripped < previous[0][1]:
                lookups[base] = [(file, stripped)]

    def file_rank(candidate: tuple[str, int]) -> tuple:
        file, stripped = candidate
        return (
            0 if whitelist_allows(file, concrete, templates) else 1,
            stripped,
            sum(1 for char in file.rsplit("\\", 1)[-1] if not char.isalnum()),
            len(file),
            -ord(file[0]) if file else 0,
            file,
        )

    matched: list[tuple[int, str, str]] = []
    for skill in skills:
        aegis_key = normalize(skill["name"])
        description_key = normalize(skill.get("description", ""))

        candidates: list[tuple[int, tuple]] = []
        for key, kind in ((aegis_key, "aegis"), (description_key, "description")):
            if not key:
                continue
            for candidate in lookups.get(key, ()):
                # Exact Aegis-name matches rank before description matches.
                candidates.append((0 if kind == "aegis" else 1, file_rank(candidate) + (kind,)))

        if not candidates:
            continue

        candidates.sort()
        file_rank_tuple = candidates[0][1]
        file = file_rank_tuple[-2]
        kind = file_rank_tuple[-1]
        matched.append((skill["id"], file, kind))

    # Curated overrides for official names that are not plain name matches.
    overrides = {skill_id: path for skill_id, path in CURATED_OVERRIDES.items()}
    matched = [(skill_id, overrides.pop(skill_id, path), kind) for skill_id, path, kind in matched]
    existing_paths = {path.lower(): path for files in assets.values() for path in files}
    for skill_id, path in overrides.items():
        if path.lower() not in existing_paths:
            raise SystemExit(f"curated override for skill {skill_id} does not exist in the archives: {path}")
        matched.append((skill_id, existing_paths[path.lower()], "curated"))

    # Authoritative validation: every mapped effect must be part of the official
    # Windows client effect whitelist.
    invalid = [
        (skill_id, path)
        for skill_id, path, _ in matched
        if not whitelist_allows(path, concrete, templates)
    ]
    if invalid:
        details = "\n".join(f"  skill {skill_id}: {path}" for skill_id, path in invalid)
        raise SystemExit(f"{len(invalid)} mapped effect(s) are not in the Windows client whitelist:\n{details}")

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
        "//! is verified to exist in the CondaRO/data GRF archives and to be part of",
        "//! the Windows client effect whitelist",
        "//! (`tools/windows_client_effect_whitelist.txt`).",
        "//!",
        "//! Generated by `tools/gen_skill_effect_assets.py`; do not edit by hand.",
        "//! Regenerate with: `./tools/gen_skill_effect_assets.py`",
        "//! Drift check:      `./tools/gen_skill_effect_assets.py --check`",
        "",
        "use ragnarok_packets::SkillId;",
        "",
        "/// Skill id to effect file path inside `data\\texture\\effect\\`, sorted by",
        "/// skill id so the lookup is a binary search.",
        "#[rustfmt::skip]",
        "pub(crate) static SKILL_EFFECT_ASSETS: &[(u16, &str)] = &[",
    ]
    for skill_id, asset in rows:
        escaped = asset.replace("\\", "\\\\")
        lines.append(f'    ({skill_id}, "{escaped}"),')
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
            "    fn known_skills_have_official_effects_and_unknown_skills_do_not() {",
            "        // Storm Gust, Magnus Exorcismus and Pierce are high-confidence matches.",
            "        assert_eq!(skill_effect_asset(SkillId(89)), Some(\"stormgust.str\"));",
            "        assert_eq!(skill_effect_asset(SkillId(79)), Some(\"magnus.str\"));",
            "        assert_eq!(skill_effect_asset(SkillId(56)), Some(\"pierce.str\"));",
            "        // Official spellings from the Windows client whitelist.",
            "        assert_eq!(skill_effect_asset(SkillId(45)), Some(\"concentration.str\"));",
            "        assert_eq!(skill_effect_asset(SkillId(122)), Some(\"blastmine.str\"));",
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
    parser.add_argument("--whitelist", default=str(DEFAULT_WHITELIST), help="Windows client effect whitelist")
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
    whitelist = load_whitelist(Path(args.whitelist))

    matched = match_skills(skills, assets, whitelist)

    # Fail generation if a mapped asset does not exist in the archives (can only
    # happen for a curated override, validated above) or if the table is empty.
    existing = {path.lower() for files in assets.values() for path in files}
    for skill_id, asset, _ in matched:
        if asset.lower() not in existing:
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
