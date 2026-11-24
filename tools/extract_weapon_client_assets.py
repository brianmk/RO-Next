#!/usr/bin/env python3
"""Extract the client-archive weapon asset facts for the weapon data file.

The weapon data file (``korangar/archive/data/client/weapons.ron``) is generated
from two fact sources:

1. the server item database (``tools/extract_weapon_items.py``) and
2. this script, which scans the **actual client archives** of the CondaRO data
   set and records which weapon sprite families exist, how the per-job weapon
   variants are named (numeric item view id vs classic Korean weapon name) and
   which projectile/ammo sprite pairs are present.

The scan is offline and only reads the GRF file tables; the result
(``tools/data/weapon_client_assets.json``) is committed so that
``tools/gen_weapons.py`` and its ``--check`` drift guard work without the
3.2 GB game archives.

What is recorded
----------------
* ``variants``: for every ``data\\sprite\\인간족\\<job>\\<job>_<sex>_<variant>.spr``
  file, the variant name (numeric view id or classic weapon family name) and the
  number of job stems that ship it. This is what lets the generator include only
  sprite families that actually exist in this data set.
* ``classic_weapon_directory``: whether the classic client's job-independent
  ``data\\sprite\\인간족\\무기\\`` layer exists. It does **not** exist in the
  CondaRO data set, so the client falls back to the per-job variants only.
* ``ammo_item_sprites``: which of the classic projectile/ammo item sprites
  (arrow, bullet, grenade, shuriken) exist as a ``.spr``/``.act`` pair.
* ``weapon_info_table``: whether the official weapon-name table
  (``weapongrp.lub``) exists (it does not), i.e. the classic name mapping has to
  come from the generated data file instead of the client data.

Usage:
    ./tools/extract_weapon_client_assets.py \\
        --archive /path/to/client/game/data.grf \\
        --archive /path/to/client/game/CondaRO.grf \\
        --archive /path/to/client/game/palettes.grf \\
        --output tools/data/weapon_client_assets.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import sys
import zlib
from pathlib import Path

DEFAULT_ARCHIVES = [
    Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/data.grf"),
    Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/CondaRO.grf"),
    Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/palettes.grf"),
]
DEFAULT_OUTPUT = Path("tools/data/weapon_client_assets.json")

#: Per-job weapon variant file name:
#: ``data\sprite\인간족\<stem>\<stem>_<sex>_<variant>.spr``.
PER_JOB_VARIANT = re.compile(r"^data\\sprite\\인간족\\([^\\]+)\\\1_(남|여)_(.+)\.spr$")

#: Classic job-independent weapon layer directory.
CLASSIC_WEAPON_DIR = "data\\sprite\\인간족\\무기\\"

#: Official weapon-name table (absent in this data set).
WEAPON_INFO_TABLE = "data\\luafiles514\\lua files\\datainfo\\weapongrp.lub"

#: Projectile/ammo item sprites a flying projectile may use (classic item
#: sprites under ``data\sprite\아이템\``). Only the ones that exist in the
#: archives make it into the generated data file.
AMMO_ITEM_SPRITES = [
    # Normal and special arrows.
    "화살",
    "강철화살",
    "독화살",
    "불화살",
    "사냥용화살",
    "그림자의화살",
    "날카로운화살",
    "녹슨화살",
    "돌화살",
    "무형의화살",
    "바람의화살",
    "뼈화살",
    "수정화살",
    # Large skill-effect sprites: last-resort visual fallbacks.
    "ac_chargearrow",
    "ma_chargearrow",
    "brokenarrow",
    "ra_arrowstorm",
    "dc_throwarrow",
    # Bullets / shells.
    "마석탄",
    "빙결탄",
    "슬러그탄_1",
    "슬러그탄_2",
    "슬러그탄_3",
    # Grenades.
    "멜론폭탄",
    "바나나폭탄",
    "사과폭탄",
]


class GrfFileTable:
    """Minimal GRF (version 0x200) file-table reader.

    Only the file *names* are needed; the data offset/decryption logic is not
    implemented. The layout is documented in
    ``korangar/src/loaders/archive/native/mod.rs``.
    """

    def __init__(self, path: Path):
        self.path = path
        self._handle = path.open("rb")
        header = self._handle.read(46)
        if not header[:16].startswith(b"Master of Magic"):
            raise ValueError(f"{path}: not a GRF file")
        table_offset, reserved, file_count = struct.unpack_from("<III", header, 0x1E)
        self.version = struct.unpack_from("<I", header, 0x2A)[0]
        self._handle.seek(46 + table_offset)
        compressed_size, _uncompressed_size = struct.unpack("<II", self._handle.read(8))
        table = zlib.decompress(self._handle.read(compressed_size))
        # Version 0x200 stores 7 extra entries (data.ini, ...) that are not part
        # of the actual file list.
        count = file_count - reserved - 7
        self.names: list[str] = []
        position = 0
        for _ in range(count):
            end = table.index(b"\x00", position)
            self.names.append(table[position:end].decode("cp949", "replace").lower())
            position = end + 1 + 17
        self._handle.close()

    def sha256(self) -> str:
        digest = hashlib.sha256()
        with self.path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        return digest.hexdigest()


def collect(archives: list[Path]) -> dict:
    variant_stems: dict[str, set[str]] = {}
    ammo_sprites: dict[str, dict[str, bool]] = {name: {"spr": False, "act": False} for name in AMMO_ITEM_SPRITES}
    classic_weapon_files = 0
    weapon_info_table = False
    per_job_files = 0
    archive_info = []

    for archive_path in archives:
        if not archive_path.is_file():
            raise SystemExit(f"archive not found: {archive_path}")
        table = GrfFileTable(archive_path)
        archive_info.append({"name": archive_path.name, "size": archive_path.stat().st_size, "sha256": table.sha256(), "files": len(table.names)})

        for name in table.names:
            variant_match = PER_JOB_VARIANT.match(name)
            if variant_match is not None:
                variant_stems.setdefault(variant_match.group(3), set()).add(variant_match.group(1))
                per_job_files += 1
            elif name.startswith(CLASSIC_WEAPON_DIR):
                classic_weapon_files += 1
            if name == WEAPON_INFO_TABLE:
                weapon_info_table = True
            for sprite, flags in ammo_sprites.items():
                stem = f"data\\sprite\\아이템\\{sprite}"
                if name == f"{stem}.spr":
                    flags["spr"] = True
                elif name == f"{stem}.act":
                    flags["act"] = True

    return {
        "provenance": {
            "source": "CondaRO client archives (kRO 2019-06-05 data set)",
            "archives": archive_info,
            "scanner": "tools/extract_weapon_client_assets.py",
        },
        "per_job_variant_files": per_job_files,
        "variants": {name: {"stems": len(stems)} for name, stems in sorted(variant_stems.items())},
        "classic_weapon_directory": {"present": classic_weapon_files > 0, "files": classic_weapon_files},
        "weapon_info_table_present": weapon_info_table,
        "ammo_item_sprites": {name: flags for name, flags in ammo_sprites.items() if flags["spr"] and flags["act"]},
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", action="append", default=None, help="GRF to scan (repeatable); defaults to the CondaRO game archives")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    archives = [Path(path) for path in (args.archive or DEFAULT_ARCHIVES)]
    output = collect(archives)

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n")

    print(
        f"{out_path}: {len(output['variants'])} weapon variants from {output['per_job_variant_files']} per-job files, "
        f"{len(output['ammo_item_sprites'])} ammo sprites, classic weapon dir present: {output['classic_weapon_directory']['present']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
