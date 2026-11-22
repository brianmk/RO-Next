#!/usr/bin/env python3
"""Generate the client data file with the official per-skill cast sounds.

The official client plays the `wav` of the effect a skill spawns. The 2019 map
server rarely sends the sound itself, so the client plays a mapped sound on the
skill cast/completion path (`korangar/src/world/skill_sound.rs`). Before this
table every skill except a hand-curated handful used one generic
`ef_beginspell.wav`.

The facts come from `tools/extract_skill_sounds.py`
(`tools/windows_client_skill_sounds.json`). This generator walks the official
effect layers of every skill in play priority order (success, ground, cast,
release, hit, caster), resolves the effect table `wav` names against the CondaRO
GRF archives and writes the first existing candidates:

    korangar/archive/data/client/skill_sounds.ron

The file is embedded with `include_str!` as the compiled fallback and can be
overridden by a server without rebuilding the client.

Usage:
    ./tools/gen_skill_sounds.py            # regenerate the data file
    ./tools/gen_skill_sounds.py --check    # fail if the file is out of date
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
DEFAULT_FACTS = Path("tools/windows_client_skill_sounds.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_sounds.ron")
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


def render(rows: list[tuple[int, list[str]]], provenance: dict) -> str:
    lines = [
        "// Official skill -> cast sound mapping for the client.",
        "//",
        "// Generated by `tools/gen_skill_sounds.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_sounds.py`",
        "// Drift check:      `./tools/gen_skill_sounds.py --check`",
        "//",
        "// Facts extracted from the official client's effect table",
        "// (`tools/windows_client_skill_sounds.json`, roBrowserLegacy RE dump of the",
        "// Windows client tables); every wav is validated against the CondaRO GRFs.",
        "//",
        "// Format: (skill id, wav files). Paths are relative to `data\\wav\\`; the",
        "// first existing file is played (see `korangar/src/world/skill_sound.rs`).",
        "// Skills without an entry play the generic cast sound.",
        "//",
        "// The file is loaded at runtime through the normal client file loader, so a",
        "// server can override it without rebuilding the client; the compiled Rust",
        "// table is only the fallback for missing files.",
        "(",
        f'    provenance: "{provenance["source"]}",',
        "    sounds: [",
    ]
    for skill_id, files in rows:
        rendered = ", ".join(f'"{file.replace(chr(92), chr(92) * 2)}"' for file in files)
        lines.append(f"        (skill: {skill_id}, files: [{rendered}]),")
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--game-dir", default=DEFAULT_GAME_DIR)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    game_dir = Path(args.game_dir)
    if not game_dir.is_dir():
        raise SystemExit(f"game directory with GRF archives not found: {game_dir}")

    payload = json.loads(Path(args.facts).read_text())
    facts = load_facts(Path(args.facts))
    wavs = collect_wavs(game_dir)
    if not wavs:
        raise SystemExit(f"no wav files found in the archives under {game_dir}")

    rows = build_rows(facts, wavs)
    if not rows:
        raise SystemExit("no official skill sounds resolved against the archives")

    content = render(rows, payload["provenance"])
    output = Path(args.output)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_sounds.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} skills)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    print(f"{output}: {len(rows)} skills with an official sound")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
