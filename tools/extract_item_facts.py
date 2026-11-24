#!/usr/bin/env python3
"""Extract the authoritative pre-renewal item facts for the client.

The client (RO-Next) must not guess item stats from the localized item
description text: the Korean/English ``iteminfo.lub`` description is display
text, and every stat field label is grammar of one specific translation. The
**facts** are the item database rows the server actually uses.

This script reads the CondaRO pre-renewal server tree:

* ``db/pre-re/item_db.yml`` (the import manifest) and every file it imports
  (``item_db_usable.yml``, ``item_db_equip.yml``, ``item_db_etc.yml``),
* the rAthena item bonus constants from ``doc/item_bonus.txt`` (only used to
  validate the bonus mapping; the constants themselves are part of the item
  script language, see ``FLAT_BONUS_FIELDS``).

The output (``tools/data/rathena_pre_re_item_facts.json``) is the input of
``tools/gen_item_stats.py``, which writes the client data file
``korangar/archive/data/client/item_stats.ron``. Keeping the facts as a
committed JSON file means the generator (and its ``--check`` drift guard) runs
offline and reproducibly; only this extractor needs access to the server tree.

Only **unconditional, flat** item script bonuses are extracted. Conditional
bonuses (``if (...)`` blocks, ``autobonus`` triggers), percentage bonuses and
the 100+ non-stat bonus kinds stay out of the data: the client cannot evaluate
them and the server is authoritative for the derived character stats anyway.
The flat bonus parser therefore

* strips ``//`` comments and ignores everything inside string literals
  (``autobonus "{ bonus bStr,1; }",...`` must not be counted),
* drops all bonus statements inside braces (conditional blocks),
* drops a bonus statement directly attached to an ``if``/``else``/``for``/
  ``while`` without braces (rules are documented in ``flat_script_bonuses``).

Usage:
    ./tools/extract_item_facts.py \\
        --server-tree /path/to/rathena \\
        --output tools/data/rathena_pre_re_item_facts.json

    ./tools/extract_item_facts.py --check   # fail if the committed JSON is stale
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover - dependency documented in the help
    raise SystemExit("PyYAML is required: pip install pyyaml")

#: The CondaRO pre-renewal server tree shipped on this machine.
DEFAULT_SERVER_TREE = Path("/home/phantom/Games/ragnarok/CondaRO-linux/popeng-graft/scratch-condaro")
DEFAULT_OUTPUT = Path("tools/data/rathena_pre_re_item_facts.json")

#: The import manifest. Its ``Footer.Imports`` list is the authoritative list of
#: item database files; the script follows it instead of hard-coding the split.
ITEM_DB_MANIFEST = "db/pre-re/item_db.yml"

#: Items that can be used from the inventory (rAthena ``enum item_types``
#: groups). ``Cash`` items are consumables that were bought with cash points.
USABLE_TYPES = {"Healing", "Usable", "Delayconsume", "Cash"}

#: rAthena item script ``bonus b<Name>,<value>;`` constants that are
#: unconditional flat stat values the client stat panel shows. The value is the
#: list of ``ItemStats``/``StatBonuses`` fields the constant maps to. Anything
#: not listed here (percentages, race/element modifiers, autobonus triggers,
#: MaxHP, ...) is intentionally not part of the client stat panel data.
FLAT_BONUS_FIELDS = {
    "Str": ("strength",),
    "Agi": ("agility",),
    "Vit": ("vitality",),
    "Int": ("intelligence",),
    "Dex": ("dexterity",),
    "Luk": ("luck",),
    "Atk": ("attack",),
    "Matk": ("magic_attack",),
    "Def": ("defense",),
    "Mdef": ("magic_defense",),
    "Hit": ("hit",),
    "Flee": ("flee",),
    "Critical": ("critical",),
    "Aspd": ("attack_speed",),
    # Composite convenience constants that are still flat values.
    "AllStats": ("strength", "agility", "vitality", "intelligence", "dexterity", "luck"),
    "AgiVit": ("agility", "vitality"),
    "AgiDexStr": ("strength", "agility", "dexterity"),
}

#: Canonical order of the flat bonus fields in the facts JSON (stable diffs).
BONUS_FIELD_ORDER = (
    "strength",
    "agility",
    "vitality",
    "intelligence",
    "dexterity",
    "luck",
    "attack",
    "magic_attack",
    "defense",
    "magic_defense",
    "hit",
    "flee",
    "critical",
    "attack_speed",
)

_BONUS_RE = re.compile(r"bonus\s+b(\w+)\s*,\s*(-?\d+)\s*;")
_STRUCTURAL_RE = re.compile(r"\b(if|else|for|while)\b")
_STRING_RE = re.compile(r'"(?:[^"\\]|\\.)*"')


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def server_head(server_tree: Path) -> str:
    """Best-effort git commit of the server tree (empty when it is not a repo)."""
    try:
        result = subprocess.run(
            ["git", "-C", str(server_tree), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError:
        return ""
    return result.stdout.strip() if result.returncode == 0 else ""


def packet_version(server_tree: Path) -> str:
    """`PACKETVER` the server tree is built with (custom override first)."""
    for relative in ("src/custom/defines_pre.hpp", "src/config/packets.hpp"):
        path = server_tree / relative
        if not path.is_file():
            continue
        match = re.search(r"^\s*#define\s+PACKETVER\s+(\d+)", path.read_text(errors="replace"), re.M)
        if match:
            return match.group(1)
    return ""


def renewal_mode(server_tree: Path) -> str:
    """Whether the server tree enables renewal (rAthena `RENEWAL` define)."""
    path = server_tree / "src/config/renewal.hpp"
    if not path.is_file():
        return "unknown"
    text = path.read_text(errors="replace")
    if re.search(r"^\s*#define\s+RENEWAL\s*$", text, re.M):
        return "renewal (RENEWAL enabled)"
    return "pre-renewal (RENEWAL disabled)"


def mask_strings(script: str) -> str:
    """Replace string literal characters with spaces while keeping the length.

    Bonus statements inside strings (``autobonus "{ bonus bStr,1; }"``) are
    trigger payloads, not unconditional equip bonuses. Masking them lets the
    char-based scanner see braces and semicolons only from real statements.
    """
    def replace(match: re.Match[str]) -> str:
        return " " * len(match.group(0))

    return _STRING_RE.sub(replace, script)


def strip_comments(script: str) -> str:
    """Remove ``//`` comments outside string literals."""
    result = []
    index = 0
    length = len(script)
    in_string = False
    while index < length:
        character = script[index]
        if in_string:
            result.append(character)
            if character == "\\" and index + 1 < length:
                result.append(script[index + 1])
                index += 2
                continue
            if character == '"':
                in_string = False
            index += 1
            continue
        if character == '"':
            in_string = True
            result.append(character)
            index += 1
            continue
        if character == "/" and index + 1 < length and script[index + 1] == "/":
            while index < length and script[index] != "\n":
                index += 1
            continue
        result.append(character)
        index += 1
    return "".join(result)


def _brace_depths(masked: str) -> list[int]:
    """Brace depth at every character position of the (string-masked) script."""
    depths = []
    depth = 0
    for character in masked:
        depths.append(depth)
        if character == "{":
            depth += 1
        elif character == "}":
            depth = max(depth - 1, 0)
    return depths


def _statement_context(masked: str, start: int, depths: list[int]) -> bool:
    """Whether the statement at ``start`` is unconditional (top level).

    A statement is unconditional when it is at brace depth 0 and the text
    between the previous statement boundary (``;``, ``}`` or the start of the
    script) and the statement does not contain an ``if``/``else``/``for``/
    ``while`` keyword. That rejects the brace-less single-statement forms
    (``if (readparam(bStr)>=80) bonus bStr,1;``) as well as the imports of
    conditional blocks, while keeping the flat bonuses after a finished block
    (``}\\nbonus bStr,1;``).
    """
    if depths[start] != 0:
        return False

    boundary = 0
    for index in range(start - 1, -1, -1):
        if masked[index] in ";}":
            boundary = index + 1
            break

    prefix = masked[boundary:start]
    if _STRUCTURAL_RE.search(prefix):
        return False

    return prefix.strip() == ""


def flat_script_bonuses(script: str | None) -> dict[str, int]:
    """Unconditional flat stat bonuses of an item script.

    Returns the canonical client field names (see ``BONUS_FIELD_ORDER``) with
    their summed value, only for non-zero values. The parser is deliberately
    conservative: anything it cannot prove to be unconditional is skipped.
    """
    bonuses: dict[str, int] = {}
    if not script:
        return bonuses

    cleaned = strip_comments(script)
    masked = mask_strings(cleaned)
    depths = _brace_depths(masked)

    for match in _BONUS_RE.finditer(masked):
        constant = match.group(1)
        fields = FLAT_BONUS_FIELDS.get(constant)
        if fields is None:
            continue
        if not _statement_context(masked, match.start(), depths):
            continue
        value = int(match.group(2))
        if value == 0:
            continue
        for field in fields:
            bonuses[field] = bonuses.get(field, 0) + value

    return {field: bonuses[field] for field in BONUS_FIELD_ORDER if bonuses.get(field, 0) != 0}


def load_manifest(server_tree: Path) -> tuple[Path, list[Path]]:
    """Returns the manifest path and the imported item database files."""
    manifest = server_tree / ITEM_DB_MANIFEST
    if not manifest.is_file():
        raise SystemExit(f"item database manifest not found: {manifest}")

    payload = yaml.safe_load(manifest.read_text())
    imports = ((payload.get("Footer") or {}).get("Imports")) or []
    paths = []
    for entry in imports:
        relative = entry.get("Path") if isinstance(entry, dict) else entry
        if not relative:
            raise SystemExit(f"{manifest}: import without a Path: {entry!r}")
        path = server_tree / relative
        if not path.is_file():
            raise SystemExit(f"{manifest}: imported item database not found: {path}")
        paths.append(path)

    if not paths:
        raise SystemExit(f"{manifest}: no imports found")

    return manifest, paths


def item_fact(entry: dict) -> dict:
    """Converts one rAthena item database entry into the client fact row."""
    item_type = str(entry.get("Type") or "Etc")
    subtype = entry.get("SubType", "")
    locations = entry.get("Locations") or {}
    equip_positions = sorted(name for name, enabled in locations.items() if enabled)

    return {
        "id": int(entry["Id"]),
        "aegis": str(entry["AegisName"]),
        "type": item_type,
        "subtype": "" if subtype in (None, 0, "0") else str(subtype),
        "weapon_type": str(subtype) if item_type == "Weapon" and subtype not in (None, 0, "0") else "",
        "slots": int(entry.get("Slots", 0)),
        "refineable": bool(entry.get("Refineable", False)),
        "view": int(entry.get("View", 0)),
        "equip_positions": equip_positions,
        "usable": item_type in USABLE_TYPES,
        "weight": int(entry.get("Weight", 0)),
        "attack": int(entry.get("Attack", 0)),
        "defense": int(entry.get("Defense", 0)),
        "weapon_level": int(entry.get("WeaponLevel", 1)) if item_type == "Weapon" else 0,
        "armor_level": int(entry.get("ArmorLevel", 1)) if item_type == "Armor" else 0,
        "required_level": int(entry.get("EquipLevelMin", 0)),
        "flat_bonuses": flat_script_bonuses(entry.get("Script")),
    }


def extract(server_tree: Path, server_head_value: str = "") -> dict:
    manifest, db_paths = load_manifest(server_tree)

    items: list[dict] = []
    seen: dict[int, str] = {}
    database_counts: dict[str, int] = {}
    for path in db_paths:
        payload = yaml.safe_load(path.read_text())
        body = payload.get("Body") or []
        if not body:
            raise SystemExit(f"{path}: no item rows")
        database_counts[path.name] = len(body)
        for entry in body:
            item_id = int(entry["Id"])
            if item_id in seen:
                raise SystemExit(f"duplicate item id {item_id} in {path} (already defined by {seen[item_id]})")
            seen[item_id] = path.name
            items.append(item_fact(entry))

    items.sort(key=lambda row: row["id"])

    # Validation: every id unique and every flat bonus value an integer in a
    # known client field. A corrupted script that maps a constant differently
    # must not silently reach the client.
    bonus_fields = [field for row in items for field in row["flat_bonuses"]]
    unknown = sorted({field for field in bonus_fields if field not in BONUS_FIELD_ORDER})
    if unknown:
        raise SystemExit(f"unknown flat bonus fields: {unknown}")

    return {
        "provenance": {
            "source": "rAthena pre-renewal item database of the CondaRO server tree",
            "server_head": server_head_value or server_head(server_tree),
            "packetver": packet_version(server_tree) or "unknown",
            "renewal_mode": renewal_mode(server_tree),
            "manifest": ITEM_DB_MANIFEST,
            "manifest_sha256": sha256(manifest),
            "databases": [
                {
                    "path": str(path.relative_to(server_tree)),
                    "sha256": sha256(path),
                    "items": database_counts[path.name],
                }
                for path in db_paths
            ],
            "item_count": len(items),
            "usable_items": sum(1 for row in items if row["usable"]),
            "equipment_items": sum(1 for row in items if row["type"] in ("Weapon", "Armor", "Petarmor", "Ammo")),
            "items_with_flat_bonuses": sum(1 for row in items if row["flat_bonuses"]),
        },
        "items": items,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--server-tree", default=str(DEFAULT_SERVER_TREE))
    parser.add_argument("--server-head", default="", help="git commit of the server tree the files were taken from")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed JSON differs from a fresh extraction")
    args = parser.parse_args()

    server_tree = Path(args.server_tree)
    if not server_tree.is_dir():
        raise SystemExit(f"server tree not found: {server_tree}")

    payload = extract(server_tree, args.server_head)
    content = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    output = Path(args.output)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/extract_item_facts.py", file=sys.stderr)
            # The server tree path and hashes change when the tree moves; report
            # the first differing line instead of dumping megabytes of JSON.
            current_lines = current.splitlines()
            new_lines = content.splitlines()
            for line_number, (old, new) in enumerate(zip(current_lines, new_lines), 1):
                if old != new:
                    print(f"  first difference at line {line_number}: {old!r} != {new!r}", file=sys.stderr)
                    break
            else:
                print(f"  line count differs: {len(current_lines)} != {len(new_lines)}", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({payload['provenance']['item_count']} items)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    provenance = payload["provenance"]
    print(
        f"{output}: {provenance['item_count']} items "
        f"({provenance['equipment_items']} equipment, {provenance['usable_items']} usable, "
        f"{provenance['items_with_flat_bonuses']} with flat bonuses)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
