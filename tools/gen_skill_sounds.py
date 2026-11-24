#!/usr/bin/env python3
"""Generate the client data file with the official per-skill cast sounds.

The official client plays the `wav` of the effect a skill spawns. The 2019 map
server rarely sends the sound itself, so the client plays a mapped sound on the
skill cast/completion path (`korangar/src/world/skill_sound.rs`). Before this
table every skill except a hand-curated handful used one generic
`ef_beginspell.wav`.

The facts come from `tools/extract_skill_sounds.py`
(`tools/data/windows_client_skill_sounds.json`). This generator walks the official
effect layers of every skill in play priority order (success, ground, cast,
release, hit, caster), resolves the effect table `wav` names against the CondaRO
GRF archives and writes the first existing candidates.

The hand-curated overrides of `tools/data/skill_sounds_custom.ron` (classic client
behaviour that the official effect table does not carry, plus the generic cast
fallback) are merged into the same file as separate `curated`/`generic`
sections, so the runtime precedence stays exactly what it was when the table was
a Rust constant: curated -> per-layer `wav` -> official generated -> generic.

Output:

    korangar/archive/data/client/skill_sounds.ron

The file is embedded with `include_str!` as the compiled fallback and can be
overridden by a server without rebuilding the client.

Usage:
    ./tools/gen_skill_sounds.py            # regenerate the data file
    ./tools/gen_skill_sounds.py --check    # fail if the file is out of date
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import re
import struct
import sys
import zlib
from pathlib import Path

DEFAULT_GAME_DIR = "/home/phantom/Games/ragnarok/CondaRO-linux/client/game"
DEFAULT_FACTS = Path("tools/data/windows_client_skill_sounds.json")
DEFAULT_CUSTOM = Path("tools/data/skill_sounds_custom.ron")
DEFAULT_RODECOMP = Path("tools/data/skill_sounds_rodecomp.ron")
DEFAULT_GRF2019 = Path("tools/data/grf2019_skill_facts.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_sounds.ron")
SCRIPT_DIR = Path(__file__).resolve().parent
ARCHIVES = ("CondaRO.grf", "data.grf", "palettes.grf")
WAV_PREFIX = "data\\wav\\"
GRF_HEADER_SIZE = 46

#: Keep a small fallback chain per skill: the first existing wav is played.
MAX_CANDIDATES = 3

#: Wavs that are played by many skills and therefore say nothing about the
#: skill. They are kept as candidates, but a skill-specific sound wins when the
#: official table has one (e.g. Fire Ball has both the generic `ef_hit2` impact
#: sound and the distinctive `ef_fireball` cast sound).
GENERIC_WAV_PATTERNS = (
    re.compile(r"^effect\\ef_hit\d*\.wav$", re.IGNORECASE),
    re.compile(r"^_enemy_hit_", re.IGNORECASE),
    re.compile(r"^effect\\ef_beginspell\d*\.wav$", re.IGNORECASE),
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


def collect_wavs(game_dir: Path) -> dict[str, str]:
    """Return lower-case wav path (relative to `data\\wav\\`) -> archive spelling."""
    wavs: dict[str, str] = {}
    for archive in ARCHIVES:
        path = game_dir / archive
        if not path.is_file():
            continue
        for name in read_grf_names(path):
            lower = name.lower()
            if not lower.startswith(WAV_PREFIX) or not lower.endswith(".wav"):
                continue
            relative = name[len(WAV_PREFIX):]
            wavs.setdefault(relative.lower(), relative)
    return wavs


def normalize_wav(name: str) -> str:
    """Normalize a table wav name to a `data\\wav`-relative path with extension."""
    candidate = name.strip().replace("/", "\\")
    if candidate.lower().startswith(WAV_PREFIX):
        candidate = candidate[len(WAV_PREFIX):]
    if not candidate.lower().endswith(".wav"):
        candidate += ".wav"
    return candidate


def load_facts(path: Path) -> dict[int, list[dict]]:
    payload = json.loads(path.read_text())
    return {int(skill_id): rows for skill_id, rows in payload["skills"].items()}


def is_generic(wav: str) -> bool:
    relative = wav.lower()
    return any(pattern.match(relative) for pattern in GENERIC_WAV_PATTERNS)


def build_rows(facts: dict[int, list[dict]], wavs: dict[str, str]) -> list[tuple[int, list[str]]]:
    rows: list[tuple[int, list[str]]] = []
    for skill_id, entries in sorted(facts.items()):
        resolved: list[tuple[int, int, str]] = []
        for index, entry in enumerate(entries):
            normalized = normalize_wav(entry["wav"])
            actual = wavs.get(normalized.lower())
            if actual is None:
                continue
            resolved.append((1 if is_generic(actual) else 0, index, actual))

        candidates: list[str] = []
        for _penalty, _index, wav in sorted(resolved):
            if wav not in candidates:
                candidates.append(wav)
            if len(candidates) >= MAX_CANDIDATES:
                break
        if candidates:
            rows.append((skill_id, candidates))
    return rows


def load_ron_helpers():
    """Reuse the minimal RON reader of the skill effect generator."""
    spec = importlib.util.spec_from_file_location("gen_skill_effects", SCRIPT_DIR / "gen_skill_effects.py")
    if spec is None or spec.loader is None:
        raise SystemExit("cannot load tools/gen_skill_effects.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_custom(path: Path) -> tuple[list[tuple[int, list[str]]], list[str], str]:
    """Read the curated sound overrides and the generic cast fallback."""
    if not path.is_file():
        raise SystemExit(f"missing custom skill sound data: {path}")
    ron = load_ron_helpers()
    document = ron.read_ron(path)
    if not isinstance(document, dict):
        raise SystemExit(f"{path}: top level value is not a map")

    provenance = document.get("provenance")
    if not isinstance(provenance, str) or not provenance:
        raise SystemExit(f"{path}: missing provenance")

    def wav_files(value: object, context: str) -> list[str]:
        if not isinstance(value, list) or not value:
            raise SystemExit(f"{path}: {context} has no wav files")
        files = []
        for entry in value:
            if not isinstance(entry, str) or not entry.lower().endswith(".wav"):
                raise SystemExit(f"{path}: {context}: not a .wav path: {entry!r}")
            files.append(entry)
        return files

    generic = wav_files(document.get("generic"), "generic")
    rows: list[tuple[int, list[str]]] = []
    seen: set[int] = set()
    for row in document.get("sounds") or []:
        if not isinstance(row, dict):
            raise SystemExit(f"{path}: sound row is not a map: {row!r}")
        skill = row.get("skill")
        if not isinstance(skill, int) or isinstance(skill, bool) or skill <= 0:
            raise SystemExit(f"{path}: invalid skill id: {skill!r}")
        if skill in seen:
            raise SystemExit(f"{path}: duplicate curated sound for skill {skill}")
        seen.add(skill)
        rows.append((skill, wav_files(row.get("files"), f"skill {skill}")))
    if not rows:
        raise SystemExit(f"{path}: no curated sounds")
    rows.sort(key=lambda row: row[0])
    return rows, generic, provenance


def load_rodecomp(path: Path) -> tuple[list[tuple[int, list[str]]], str]:
    """Read the official-client cast sounds extracted from the decompilation."""
    if not path.is_file():
        return [], ""
    ron = load_ron_helpers()
    document = ron.read_ron(path)
    if not isinstance(document, dict):
        raise SystemExit(f"{path}: top level value is not a map")
    provenance = document.get("provenance")
    if not isinstance(provenance, str) or not provenance:
        raise SystemExit(f"{path}: missing provenance")

    rows: list[tuple[int, list[str]]] = []
    seen: set[int] = set()
    for row in document.get("sounds") or []:
        if not isinstance(row, dict):
            raise SystemExit(f"{path}: sound row is not a map: {row!r}")
        skill = row.get("skill")
        if not isinstance(skill, int) or isinstance(skill, bool) or skill <= 0:
            raise SystemExit(f"{path}: invalid skill id: {skill!r}")
        if skill in seen:
            raise SystemExit(f"{path}: duplicate sound for skill {skill}")
        seen.add(skill)
        files = row.get("files")
        if not isinstance(files, list) or not files:
            raise SystemExit(f"{path}: skill {skill} has no wav files")
        for entry in files:
            if not isinstance(entry, str) or not entry.lower().endswith(".wav"):
                raise SystemExit(f"{path}: skill {skill}: not a .wav path: {entry!r}")
        rows.append((skill, list(files)))
    rows.sort(key=lambda row: row[0])
    return rows, provenance


def escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def ron_string(text: str) -> str:
    """Quote a wav path for RON (backslashes are doubled)."""
    return '"' + text.replace(chr(92), chr(92) * 2) + '"'


def render(
    rows: list[tuple[int, list[str]]],
    provenance: dict,
    curated: list[tuple[int, list[str]]],
    generic: list[str],
    custom_provenance: str,
) -> str:
    lines = [
        "// Official skill -> cast sound mapping for the client.",
        "//",
        "// Generated by `tools/gen_skill_sounds.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_sounds.py`",
        "// Drift check:      `./tools/gen_skill_sounds.py --check`",
        "//",
        "// Facts extracted from the official client's effect table",
        "// (`tools/data/windows_client_skill_sounds.json`, roBrowserLegacy RE dump of the",
        "// Windows client tables); every wav is validated against the CondaRO GRFs.",
        "// The curated section comes from `tools/data/skill_sounds_custom.ron`.",
        "//",
        "// Format: (skill id, wav files). Paths are relative to `data\\wav\\`; the",
        "// first existing file is played (see `korangar/src/world/skill_sound.rs`).",
        "// Play priority: `curated` -> the per-layer `wav` of `skill_effects.ron` ->",
        "// `sounds` -> `generic`.",
        "//",
        "// The file is loaded at runtime through the normal client file loader, so a",
        "// server can override it without rebuilding the client; the compiled Rust",
        "// copy is only the fallback for missing files.",
        "(",
        f'    provenance: "{provenance["source"]}",',
        "    // Generic cast fallback for skills without any mapping.",
        "    generic: [",
    ]
    for file in generic:
        lines.append(f"        {ron_string(file)},")
    lines.append("    ],")
    lines.append("    // Hand-curated overrides of `tools/data/skill_sounds_custom.ron`.")
    lines.append(f"    // {custom_provenance}")
    lines.append("    curated: [")
    for skill_id, files in curated:
        rendered = ", ".join(ron_string(file) for file in files)
        lines.append(f"        (skill: {skill_id}, files: [{rendered}]),")
    lines.append("    ],")
    lines.append("    // Official per-skill sounds generated from the effect table.")
    lines.append("    sounds: [")
    for skill_id, files in rows:
        rendered = ", ".join(ron_string(file) for file in files)
        lines.append(f"        (skill: {skill_id}, files: [{rendered}]),")
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--custom", default=str(DEFAULT_CUSTOM), help="hand-curated sound overrides")
    parser.add_argument("--rodecomp", default=str(DEFAULT_RODECOMP), help="official-client sounds from the decompilation")
    parser.add_argument("--grf2019", default=str(DEFAULT_GRF2019), help="per-skill wave files of the 2019 GRF tables")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    # The curated overrides do not need game data; validate them first so a bad
    # custom file fails even when the archives are unavailable.
    curated, generic, custom_provenance = load_custom(Path(args.custom))
    curated_skills = {skill for skill, _ in curated}

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")

    payload = json.loads(Path(args.facts).read_text())
    facts = load_facts(Path(args.facts))
    wavs = collect_wavs(game_dir)
    if not wavs:
        raise SystemExit(f"no wav files found in the archives under {game_dir}")

    # The 2019 GRF tables (SKILL_EFFECT_INFO_LIST) carry the wave file of every
    # skill they cover. They fill the skills the community dump does not list
    # (newer jobs); a skill that both sources cover keeps the community order.
    grf2019_path = Path(args.grf2019)
    if grf2019_path.is_file():
        grf2019 = json.loads(grf2019_path.read_text())["skills"]
        for skill_id, entry in grf2019.items():
            skill = int(skill_id)
            if skill in facts:
                continue
            wavs_of_skill = entry.get("wavs") or []
            if wavs_of_skill:
                facts[skill] = [{"wav": wav} for wav in wavs_of_skill]

    rows = build_rows(facts, wavs)
    if not rows:
        raise SystemExit("no official skill sounds resolved against the archives")

    # The decompiled client's own PlayWave calls win over the community dump:
    # they are what the official client actually plays for the skill.
    rodecomp_rows, _rodecomp_provenance = load_rodecomp(Path(args.rodecomp))
    if rodecomp_rows:
        known = {skill: files for skill, files in rows}
        for skill, files in rodecomp_rows:
            resolved = [file for file in files if file.lower() in wavs]
            if not resolved:
                raise SystemExit(f"rodecomp sound for skill {skill} does not exist in the archives: {files}")
            known[skill] = resolved
        rows = sorted(known.items())
    # The curated rows are emitted in their own section (they win over the
    # per-layer wav and the official table), so keep one row per skill.
    rows = [(skill, files) for skill, files in rows if skill not in curated_skills]

    content = render(rows, payload["provenance"], curated, generic, custom_provenance)
    output = Path(args.output)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_sounds.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(curated)} curated + {len(rows)} official skills)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    print(f"{output}: {len(curated)} curated + {len(rows)} official skills with a sound")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
