#!/usr/bin/env python3
"""Generate the client's data-driven item stats table.

Writes ``korangar/archive/data/client/item_stats.ron``, the single source of
truth for the structured stats of every item in RO-Next:

* base stats (attack, defense, weight, weapon/armor level),
* requirements (required level, card slots, refineable),
* unconditional flat stat bonuses (``STR + 2`` style values).

The client used to parse these values out of the localized ``iteminfo.lub``
description text. That made the whole stat panel depend on one specific
translation's grammar (the Korean stat field labels); with an English iteminfo
every stat was silently zero. The facts now come from the item database the
server actually uses.

Input (committed, so this generator and its ``--check`` drift guard run
offline): ``tools/rathena_pre_re_item_facts.json`` — extracted from the CondaRO
pre-renewal server tree by ``tools/extract_item_facts.py``.

The generated file is embedded in the client with ``include_str!`` as a
compiled fallback and can be overridden at runtime through the client archive
(``data\\client\\item_stats.ron``), exactly like the other client data files.

Usage:
    ./tools/gen_item_stats.py            # regenerate the data file
    ./tools/gen_item_stats.py --check    # fail if the committed file is out of date
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

DEFAULT_FACTS = Path("tools/rathena_pre_re_item_facts.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/item_stats.ron")

#: RON field name -> facts field. The RON fields are the snake_case field names
#: of ``korangar/src/world/library/item_stats.rs`` (``ItemStats``).
STAT_FIELDS = (
    ("attack", "attack"),
    ("defense", "defense"),
    ("weight", "weight"),
    ("weapon_level", "weapon_level"),
    ("armor_level", "armor_level"),
    ("required_level", "required_level"),
    ("slots", "slots"),
    ("refineable", "refineable"),
)

#: Canonical order of the flat ``StatBonuses`` fields (must match the Rust
#: struct field order and the facts file order).
BONUS_FIELDS = (
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

#: rAthena stores item weight in units of 0.1 (``db/pre-re/item_db.yml``:
#: "Weight: Item weight. Each 10 is 1 weight"). The client stat panel shows the
#: same integer weight the server uses for its own display and calculations,
#: i.e. the database value divided by 10. Documented here so the data file does
#: not contain a second, unexplained unit.
WEIGHT_DIVISOR = 10

KNOWN_TYPES = {
    "Healing",
    "Usable",
    "Etc",
    "Armor",
    "Weapon",
    "Card",
    "Petegg",
    "Petarmor",
    "Ammo",
    "Delayconsume",
    "Cash",
}


def load_facts(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"facts file not found: {path}; run ./tools/extract_item_facts.py")
    return json.loads(path.read_text())


def ron_str(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def build_rows(facts: dict) -> list[tuple[int, dict]]:
    """Converts the facts rows into ``(item id, stat fields)`` rows.

    Only non-default fields are emitted; the RON struct defaults the rest (the
    generator is the only writer, so this keeps the data file compact without
    making the schema ambiguous). ``weight`` is always emitted as the first
    field so that a row is never empty and the unit conversion is visible.
    """
    provenance = facts.get("provenance") or {}
    if provenance.get("item_count") != len(facts.get("items") or []):
        raise SystemExit("facts provenance item_count does not match the items list")

    rows: list[tuple[int, dict]] = []
    seen: set[int] = set()
    for item in facts["items"]:
        item_id = int(item["id"])
        if item_id in seen:
            raise SystemExit(f"duplicate item id {item_id} in the facts file")
        seen.add(item_id)

        item_type = item["type"]
        if item_type not in KNOWN_TYPES:
            raise SystemExit(f"item {item_id}: unknown rAthena item type {item_type!r}")

        bonuses = item.get("flat_bonuses") or {}
        unknown = sorted(set(bonuses) - set(BONUS_FIELDS))
        if unknown:
            raise SystemExit(f"item {item_id}: unknown flat bonus fields {unknown}")
        if item.get("weight", 0) % WEIGHT_DIVISOR > 0:
            # The rAthena weight is a 0.1 unit; the client stat panel only shows
            # whole units (like the server does), so the remainder is truncated
            # and has to be documented in the audit report.
            pass

        row: dict[str, object] = {"weight": int(item.get("weight", 0)) // WEIGHT_DIVISOR}
        for ron_field, facts_field in STAT_FIELDS:
            if ron_field == "weight":
                continue
            value = item.get(facts_field)
            if isinstance(value, bool):
                if value:
                    row[ron_field] = True
            elif value:
                row[ron_field] = int(value)
        if bonuses:
            row["bonuses"] = {field: int(bonuses[field]) for field in BONUS_FIELDS if bonuses.get(field, 0) != 0}
        rows.append((item_id, row))

    rows.sort(key=lambda entry: entry[0])
    if [item_id for item_id, _ in rows] != sorted(seen):
        raise SystemExit("facts item ids are not unique after sorting")

    return rows


def render(facts: dict, rows: list[tuple[int, dict]]) -> str:
    provenance = facts["provenance"]
    databases = provenance.get("databases") or []
    database_lines = "".join(
        f"//     {database['path']}: sha256 {database['sha256']} ({database['items']} items)\n" for database in databases
    )
    source = provenance.get("source", "unknown source")
    server_head = provenance.get("server_head") or "not available (server tree is not a git checkout)"
    packetver = provenance.get("packetver") or "unknown"
    renewal_mode = provenance.get("renewal_mode") or "unknown"
    item_count = provenance.get("item_count", len(rows))
    equipment = provenance.get("equipment_items", 0)
    with_bonuses = provenance.get("items_with_flat_bonuses", 0)

    provenance_string = (
        f"{source}; manifest {provenance.get('manifest', '')} sha256 {provenance.get('manifest_sha256', '')} "
        f"({item_count} items, PACKETVER {packetver}, {renewal_mode}); generated by tools/gen_item_stats.py"
    )

    lines: list[str] = []
    add = lines.append
    add("// Data-driven item stats for RO-Next.")
    add("//")
    add("// Generated by `tools/gen_item_stats.py`; do not edit by hand.")
    add("// Regenerate with: `./tools/gen_item_stats.py`")
    add("// Drift check:      `./tools/gen_item_stats.py --check`")
    add("// Schema check:     `korangar --check-items`")
    add("//")
    add("// Sources (committed facts, no game files needed to regenerate):")
    add("//   * tools/rathena_pre_re_item_facts.json — extracted by `tools/extract_item_facts.py`")
    add("//     from the CondaRO pre-renewal server tree's item database import manifest")
    add("//     `db/pre-re/item_db.yml`:")
    add(database_lines.rstrip("\n"))
    add(f"//     rAthena server HEAD: {server_head}")
    add(f"//     server PACKETVER: {packetver}, {renewal_mode}")
    add(f"//     {item_count} items ({equipment} equipment, {with_bonuses} with unconditional flat bonuses).")
    add("//")
    add("// Weight is rAthena's 0.1 unit divided by 10, i.e. the whole client weight")
    add("// unit the server uses for its own display and calculations.")
    add("//")
    add("// The file is loaded at runtime through the normal client file loader")
    add("// (`data\\client\\item_stats.ron`), so a server can override the table without")
    add("// rebuilding the client. The compiled copy below is the fallback.")
    add("//")
    add("// `items` is a strictly ascending list of `(item id, stats)`. Fields equal to")
    add("// their default are omitted; `weight` is always present. Only unconditional")
    add("// flat bonuses are listed (conditional/percentage bonuses are evaluated by the")
    add("// server and are not part of the client stat panel data).")
    add("(")
    add(f"    version: 1,")
    add(f"    provenance: {ron_str(provenance_string)},")
    add("    items: [")
    for item_id, row in rows:
        fields = []
        for ron_field in ("weight", "attack", "defense", "weapon_level", "armor_level", "required_level", "slots", "refineable"):
            if ron_field not in row:
                continue
            value = row[ron_field]
            fields.append(f"{ron_field}: {str(value).lower() if isinstance(value, bool) else value}")
        if "bonuses" in row:
            bonus_fields = ", ".join(f"{field}: {value}" for field, value in row["bonuses"].items())
            fields.append(f"bonuses: ({bonus_fields})")
        add(f"        ({item_id}, ({', '.join(fields)})),")
    add("    ],")
    add(")")
    add("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    args = parser.parse_args()

    facts = load_facts(Path(args.facts))
    rows = build_rows(facts)
    content = render(facts, rows)
    output = Path(args.output)

    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_item_stats.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(rows)} items)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    print(f"{output}: {len(rows)} items, {sum(1 for _, row in rows if 'bonuses' in row)} with flat bonuses, {len(content.encode())} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
