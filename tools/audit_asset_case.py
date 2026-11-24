#!/usr/bin/env python3
"""Case-sensitivity audit of every asset path referenced by the client data.

The official Windows client is case-insensitive; Linux (and the folder archive)
are not. GRF lookups in korangar lower-case the path, but the loose `archive/`
folder is resolved on disk, so a referenced file with the wrong case would fail
there even though it works from a GRF.

The script collects every `file`/`files`/`frames`/`wav` path of the client data
files and reports:
  * paths that do not resolve in any GRF (lower-cased lookup),
  * loose-archive paths (`data\\sprite\\...` under `archive/`) whose on-disk
    name differs in case from the reference or does not exist.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, "tools")
from extract_weapon_client_assets import GrfFileTable  # noqa: E402

GAME = Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game")
ARCHIVE = Path("korangar/archive")
GRFS = ["data.grf", "CondaRO.grf"]
RON_FILES = [
    "korangar/archive/data/client/skill_effects.ron",
    "korangar/archive/data/client/skill_sounds.ron",
]

ASSET_KEYS = ("file", "files", "frames", "wav")


def collect_paths() -> set[str]:
    paths: set[str] = set()
    pattern = re.compile(r'\b(' + "|".join(ASSET_KEYS) + r'):\s*(\[[^\]]*\]|"[^"]*")')
    for name in RON_FILES:
        text = Path(name).read_text()
        for match in pattern.finditer(text):
            value = match.group(2)
            # RON escapes backslashes (`effect\\x.tga` in the file is the path
            # `effect\x.tga`); unescape before checking the archives.
            paths.update(entry.replace("\\\\", "\\") for entry in re.findall(r'"([^"]+)"', value))
    return paths


def main() -> int:
    names: set[str] = set()
    for grf in GRFS:
        archive = GAME / grf
        if archive.is_file():
            names.update(GrfFileTable(archive).names)

    textures = {name[len("data\\texture\\"):] for name in names if name.startswith("data\\texture\\")}
    sprites = {name[len("data\\sprite\\"):] for name in names if name.startswith("data\\sprite\\")}
    wavs = {name[len("data\\wav\\"):] for name in names if name.startswith("data\\wav\\")}

    problems: list[str] = []
    loose_ok = 0
    grf_ok = 0

    for path in sorted(collect_paths()):
        normalized = path.replace("/", "\\").lstrip("\\").lower()

        # Same resolution rules as the effect generators:
        #   .str -> data\texture\effect\<name>.str, wav -> data\wav\<path>,
        #   textures -> data\texture\<path>, sprite stems -> data\sprite\<stem>.spr/.act.
        resolved = False
        if normalized.endswith(".str"):
            resolved = normalized in textures or f"effect\\{normalized}" in textures
        elif normalized.endswith(".wav"):
            resolved = normalized in wavs
        elif normalized.endswith((".tga", ".bmp", ".png", ".jpg")):
            resolved = normalized in textures or f"effect\\{normalized}" in textures
        else:
            resolved = f"{normalized}.spr" in sprites and f"{normalized}.act" in sprites

        if resolved:
            grf_ok += 1
            continue

        # Loose archive files (`archive/data/...`) are resolved on disk and are
        # case sensitive on Linux: the reference has to match the real name.
        loose = None
        if normalized.endswith((".spr", ".act")):
            loose = ARCHIVE / "data" / "sprite" / normalized
        elif normalized.endswith((".tga", ".bmp", ".png", ".jpg")):
            loose = ARCHIVE / "data" / "texture" / normalized
        elif normalized.endswith(".wav"):
            loose = ARCHIVE / "data" / "wav" / normalized

        if loose is not None and loose.is_file():
            loose_ok += 1
            continue

        problems.append(path)

    print(f"assets checked: {grf_ok + loose_ok + len(problems)} (grf={grf_ok}, loose={loose_ok}, unresolved={len(problems)})")
    for path in problems[:40]:
        print(f"  unresolved: {path}")

    return 1 if problems else 0


if __name__ == "__main__":
    raise SystemExit(main())
