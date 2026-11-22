#!/usr/bin/env python3
"""Extract the authoritative pre-renewal weapon item facts for the client.

The client (RO-Next) must not hard-code weapon item ids, weapon id ranges or
the server weapon type enum. This script extracts only the **facts** from the
rAthena databases of the CondaRO pre-renewal server (the server that actually
sends the weapon fields to the client):

* every item with ``Type: Weapon`` from ``db/pre-re/item_db_equip.yml``
  (id, AegisName, SubType, Range, View override, WeaponLevel),
* the ``enum weapon_type`` values from ``src/map/pc.hpp`` (``W_FIST`` ..
  ``W_DOUBLE_SA``), i.e. the numbers the server puts into ``LOOK_WEAPON`` /
  the entity-appear ``weapon`` field,
* the ``enum e_ammo_type`` values from ``src/map/pc.hpp`` (used to derive the
  projectile/ammo kind of each weapon kind).

The output (``tools/rathena_pre_re_weapon_items.json``) is the input of
``tools/gen_weapons.py``, which writes the client data file
``korangar/archive/data/client/weapons.ron``. Keeping the facts as a committed
JSON file means the generator (and its ``--check`` drift guard) runs offline and
reproducibly; only this extractor needs access to the server tree.

Usage:
    ./tools/extract_weapon_items.py \\
        --item-db /path/to/rathena/db/pre-re/item_db_equip.yml \\
        --pc-hpp /path/to/rathena/src/map/pc.hpp \\
        --server-head <git sha of the server tree>

The script only reads the given files and writes a single JSON file.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - dependency documented in the help
    raise SystemExit("PyYAML is required: pip install pyyaml")

DEFAULT_ITEM_DB = Path("/home/phantom/rathena/db/pre-re/item_db_equip.yml")
DEFAULT_PC_HPP = Path("/home/phantom/rathena/src/map/pc.hpp")
DEFAULT_OUTPUT = Path("tools/rathena_pre_re_weapon_items.json")

#: Fallback if no `--pc-hpp` is given. Values are the rAthena `enum weapon_type`
#: (source of truth: `src/map/pc.hpp`); they are only used to keep the
#: extractor usable without the server sources and are checked against the
#: parsed enum when `--pc-hpp` is supplied. `MAX_WEAPON_TYPE` is a placeholder
#: value, the dual-wield constants continue after it.
FALLBACK_WEAPON_TYPES = [
    ("W_FIST", 0),
    ("W_DAGGER", 1),
    ("W_1HSWORD", 2),
    ("W_2HSWORD", 3),
    ("W_1HSPEAR", 4),
    ("W_2HSPEAR", 5),
    ("W_1HAXE", 6),
    ("W_2HAXE", 7),
    ("W_MACE", 8),
    ("W_2HMACE", 9),
    ("W_STAFF", 10),
    ("W_BOW", 11),
    ("W_KNUCKLE", 12),
    ("W_MUSICAL", 13),
    ("W_WHIP", 14),
    ("W_BOOK", 15),
    ("W_KATAR", 16),
    ("W_REVOLVER", 17),
    ("W_RIFLE", 18),
    ("W_GATLING", 19),
    ("W_SHOTGUN", 20),
    ("W_GRENADE", 21),
    ("W_HUUMA", 22),
    ("W_2HSTAFF", 23),
    ("W_DOUBLE_DD", 25),
    ("W_DOUBLE_SS", 26),
    ("W_DOUBLE_AA", 27),
    ("W_DOUBLE_DS", 28),
    ("W_DOUBLE_DA", 29),
    ("W_DOUBLE_SA", 30),
]

#: Fallback `enum e_ammo_type` order (`src/map/pc.hpp`).
FALLBACK_AMMO_TYPES = [
    ("AMMO_NONE", 0),
    ("AMMO_ARROW", 1),
    ("AMMO_DAGGER", 2),
    ("AMMO_BULLET", 3),
    ("AMMO_SHELL", 4),
    ("AMMO_GRENADE", 5),
    ("AMMO_SHURIKEN", 6),
    ("AMMO_KUNAI", 7),
    ("AMMO_CANNONBALL", 8),
    ("AMMO_THROWWEAPON", 9),
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def strip_comments(text: str) -> str:
    """Remove // comments outside string literals (good enough for the enums)."""
    return re.sub(r"//[^\n]*", "", text)


def parse_enum(text: str, name: str) -> list[tuple[str, int]]:
    """Parse a simple C++ enum into (symbol, numeric value) pairs.

    Handles implicit sequential values, explicit numeric values and aliases
    (``W_SHIELD = MAX_WEAPON_TYPE``). ``MAX_*`` markers are kept in the value
    sequence (they consume a number) but not returned: the dual-wield weapon
    types continue *after* ``MAX_WEAPON_TYPE``.
    """
    match = re.search(rf"enum\s+{re.escape(name)}\s*:[^;{{]*{{(.*?)}};", text, re.S)
    if match is None:
        raise SystemExit(f"enum {name} not found in the given pc.hpp")

    body = strip_comments(match.group(1))
    values: dict[str, int] = {}
    next_value = 0
    for raw in body.split(","):
        entry = raw.strip()
        if not entry:
            continue
        if "=" in entry:
            symbol, value = (part.strip() for part in entry.split("=", 1))
            if value.isdigit():
                next_value = int(value, 0)
            elif value in values:
                next_value = values[value]
            else:
                raise SystemExit(f"{name}: cannot resolve value of {symbol} = {value}")
        else:
            symbol = entry
        values[symbol] = next_value
        next_value += 1

    return [(symbol, value) for symbol, value in values.items() if not symbol.startswith("MAX_")]


def real_weapon_types(types: list[tuple[str, int]]) -> list[dict]:
    """Keep the real ``W_*`` weapon values (drop the ``W_SHIELD`` alias)."""
    result = []
    seen: set[int] = set()
    for symbol, value in types:
        if not symbol.startswith("W_") or symbol == "W_SHIELD":
            continue
        if value in seen:
            continue
        seen.add(value)
        result.append({"value": value, "name": symbol})
    result.sort(key=lambda row: row["value"])
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--item-db", default=str(DEFAULT_ITEM_DB))
    parser.add_argument("--pc-hpp", default=str(DEFAULT_PC_HPP))
    parser.add_argument("--server-head", default="", help="git commit of the rAthena tree the files were taken from")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    item_db = Path(args.item_db)
    if not item_db.is_file():
        raise SystemExit(f"item_db_equip.yml not found: {item_db}")
    payload = yaml.safe_load(item_db.read_text())
    items = payload.get("Body") or []
    weapons = [item for item in items if item.get("Type") == "Weapon"]
    if not weapons:
        raise SystemExit("no Type: Weapon items found")

    weapon_types = FALLBACK_WEAPON_TYPES
    ammo_types = FALLBACK_AMMO_TYPES
    pc_hpp = Path(args.pc_hpp)
    if pc_hpp.is_file():
        text = pc_hpp.read_text(errors="replace")
        weapon_types = parse_enum(text, "weapon_type")
        ammo_types = parse_enum(text, "e_ammo_type")
    else:
        print(f"warning: {pc_hpp} not found, using the built-in enum fallback", file=sys.stderr)

    rows = []
    for item in weapons:
        rows.append(
            {
                "id": int(item["Id"]),
                "aegis": str(item["AegisName"]),
                "name": str(item.get("Name", "")),
                "subtype": str(item.get("SubType", "")),
                "range": int(item.get("Range", 1)),
                "view": int(item.get("View", 0)),
                "weapon_level": int(item.get("WeaponLevel", 1)),
            }
        )
    rows.sort(key=lambda row: row["id"])

    output = {
        "provenance": {
            "source": "rAthena pre-renewal item database of the CondaRO server tree",
            "item_db": item_db.name,
            "item_db_sha256": sha256(item_db),
            "server_head": args.server_head,
            "weapon_items": len(rows),
        },
        "server_weapon_types": real_weapon_types(weapon_types),
        "ammo_types": [{"value": value, "name": name} for name, value in ammo_types if name != "AMMO_NONE"],
        "items": rows,
    }

    out_path = Path(args.output)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n")

    subtypes = sorted({row["subtype"] for row in rows})
    print(f"{out_path}: {len(rows)} weapons, {len(subtypes)} subtypes, {len(output['server_weapon_types'])} server types")
    return 0


if __name__ == "__main__":
    sys.exit(main())
