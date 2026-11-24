#!/usr/bin/env python3
"""Generate the client's data-driven weapon registry.

Writes ``korangar/archive/data/client/weapons.ron``, the single source of
truth for weapon handling in RO-Next:

* which **weapon kind** (sprite family, attack range, ammo/projectile kind,
  fallback chain) a weapon value maps to,
* which **sprite candidate paths** are tried for a player entity (per-job
  weapon variant next to the body sprite, plus the classic job-independent
  weapon layer as a fallback),
* which **item view ids / id ranges** belong to which kind (so item view ids
  sent by population shells or custom servers resolve to a kind),
* which **projectile assets** an auto-attack ranged weapon uses.

Inputs (both committed, so this generator and its ``--check`` run offline):

* ``tools/rathena_pre_re_weapon_items.json`` — every pre-renewal weapon item
  plus the rAthena ``enum weapon_type``/``enum e_ammo_type`` values
  (extracted by ``tools/extract_weapon_items.py`` from the CondaRO server
  tree's ``db/pre-re/item_db_equip.yml`` and ``src/map/pc.hpp``),
* ``tools/weapon_client_assets.json`` — which per-job weapon sprite families
  exist in CondaRO's client archives and which projectile item sprites are
  present (extracted by ``tools/extract_weapon_client_assets.py``).

The generated file is embedded in the client with ``include_str!`` as a
compiled fallback and can be overridden at runtime through the client archive
(``data\\client\\weapons.ron``), exactly like the skill data files.

Usage:
    ./tools/gen_weapons.py            # regenerate the data file
    ./tools/gen_weapons.py --check    # fail if the committed file is out of date
"""

from __future__ import annotations

import argparse
import collections
import json
import sys
from pathlib import Path

DEFAULT_ITEMS = Path("tools/rathena_pre_re_weapon_items.json")
DEFAULT_ASSETS = Path("tools/weapon_client_assets.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/weapons.ron")

#: Weapon kinds and the classic client weapon sprite family names observed in
#: the CondaRO archives (verified by ``--assets``).
#:
#: ``variants`` is ordered: the sprite families most likely to exist for the
#: kind come first, then fallback families (e.g. dual-wield combination sprites
#: followed by the single-weapon sprite). ``subtypes`` are the rAthena
#: ``SubType`` values that belong to the kind.
#:
#: Ranges are derived from the item database (most common ``Range`` of the
#: kind's items); kinds without items (fist, two-handed mace, dual wield) use
#: the classic value documented in the comment.
#:
#: Ammo kinds follow rAthena's weapon/ammo check (``pc.cpp`` ``pc_equipitem``):
#: bows, instruments and whips use arrows; revolvers/rifles/gatlings/shotguns
#: use bullets/shells; the grenade launcher uses grenades. Only the arrow has a
#: verified flying projectile sprite in this data set, so only arrow weapons
#: get a visual auto-attack projectile.
KINDS = [
    {"kind": "Fist", "subtypes": [], "variants": [], "range": 1, "ammo": None, "projectile": None},
    {"kind": "Dagger", "subtypes": ["Dagger"], "variants": ["단검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "OneHandedSword", "subtypes": ["1hSword"], "variants": ["검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "TwoHandedSword", "subtypes": ["2hSword"], "variants": ["양손검", "검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "OneHandedSpear", "subtypes": ["1hSpear"], "variants": ["창"], "range": 3, "ammo": None, "projectile": None},
    {"kind": "TwoHandedSpear", "subtypes": ["2hSpear"], "variants": ["양손창", "창"], "range": 3, "ammo": None, "projectile": None},
    {"kind": "OneHandedAxe", "subtypes": ["1hAxe"], "variants": ["도끼"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "TwoHandedAxe", "subtypes": ["2hAxe"], "variants": ["양손도끼", "도끼"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "Mace", "subtypes": ["Mace"], "variants": ["클럽", "해머", "스패너"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "TwoHandedMace", "subtypes": [], "variants": ["클럽"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "Staff", "subtypes": ["Staff"], "variants": ["롯드", "캔디케인롯드"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "TwoHandedStaff", "subtypes": ["2hStaff"], "variants": ["롯드", "캔디케인롯드"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "Bow", "subtypes": ["Bow"], "variants": ["활"], "range": 5, "ammo": "Arrow", "projectile": "Arrow"},
    {"kind": "Knuckle", "subtypes": ["Knuckle"], "variants": ["너클"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "MusicalInstrument", "subtypes": ["Musical"], "variants": ["악기"], "range": 1, "ammo": "Arrow", "projectile": "Arrow"},
    {"kind": "Whip", "subtypes": ["Whip"], "variants": ["채찍"], "range": 2, "ammo": "Arrow", "projectile": "Arrow"},
    {"kind": "Book", "subtypes": ["Book"], "variants": ["책"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "Katar", "subtypes": ["Katar"], "variants": ["카타르_카타르"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "Revolver", "subtypes": ["Revolver"], "variants": ["권총", "handgun2"], "range": 7, "ammo": "Bullet", "projectile": None},
    {"kind": "Rifle", "subtypes": ["Rifle"], "variants": ["라이플"], "range": 9, "ammo": "Bullet", "projectile": None},
    {"kind": "Gatling", "subtypes": ["Gatling"], "variants": ["기관총", "gun_gatling2"], "range": 9, "ammo": "Bullet", "projectile": None},
    {"kind": "Shotgun", "subtypes": ["Shotgun"], "variants": ["샷건"], "range": 9, "ammo": "Bullet", "projectile": None},
    {"kind": "Grenade", "subtypes": ["Grenade"], "variants": [], "range": 9, "ammo": "Grenade", "projectile": None},
    {"kind": "Huuma", "subtypes": ["Huuma"], "variants": ["수리검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "DoubleDagger", "subtypes": [], "variants": ["단검_단검", "단검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "DoubleSword", "subtypes": [], "variants": ["검_검", "검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "DoubleAxe", "subtypes": [], "variants": ["도끼_도끼", "도끼"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "DaggerSword", "subtypes": [], "variants": ["단검_검", "단검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "DaggerAxe", "subtypes": [], "variants": ["단검_도끼", "단검"], "range": 1, "ammo": None, "projectile": None},
    {"kind": "SwordAxe", "subtypes": [], "variants": ["검_도끼", "검"], "range": 1, "ammo": None, "projectile": None},
]

#: rAthena ``enum weapon_type`` symbol -> weapon kind. Values come from the
#: item-db facts file; the mapping is the only "translation" the generator does.
SERVER_TYPE_KINDS = {
    "W_FIST": "Fist",
    "W_DAGGER": "Dagger",
    "W_1HSWORD": "OneHandedSword",
    "W_2HSWORD": "TwoHandedSword",
    "W_1HSPEAR": "OneHandedSpear",
    "W_2HSPEAR": "TwoHandedSpear",
    "W_1HAXE": "OneHandedAxe",
    "W_2HAXE": "TwoHandedAxe",
    "W_MACE": "Mace",
    "W_2HMACE": "TwoHandedMace",
    "W_STAFF": "Staff",
    "W_2HSTAFF": "TwoHandedStaff",
    "W_BOW": "Bow",
    "W_KNUCKLE": "Knuckle",
    "W_MUSICAL": "MusicalInstrument",
    "W_WHIP": "Whip",
    "W_BOOK": "Book",
    "W_KATAR": "Katar",
    "W_REVOLVER": "Revolver",
    "W_RIFLE": "Rifle",
    "W_GATLING": "Gatling",
    "W_SHOTGUN": "Shotgun",
    "W_GRENADE": "Grenade",
    "W_HUUMA": "Huuma",
    "W_DOUBLE_DD": "DoubleDagger",
    "W_DOUBLE_SS": "DoubleSword",
    "W_DOUBLE_AA": "DoubleAxe",
    "W_DOUBLE_DS": "DaggerSword",
    "W_DOUBLE_DA": "DaggerAxe",
    "W_DOUBLE_SA": "SwordAxe",
}

#: Candidate path construction order for a weapon value that is a server
#: weapon type (`W_*`), an item view id, or an unknown numeric value.
FALLBACKS = {
    "server_type": ["PerJobVariant", "ClassicVariant"],
    "item_view": ["PerJobNumeric", "PerJobVariant", "ClassicNumeric", "ClassicVariant"],
    "unknown": ["PerJobNumeric", "ClassicNumeric"],
}

#: Sprite naming templates. Placeholders: {job_stem}, {sex}, {variant}.
PATHS = {
    "per_job": "인간족\\{folder}\\{prefix}_{sex}_{variant}",
    "classic_weapon": "인간족\\무기\\{variant}_{sex}",
}

SEX_SUFFIX = {"male": "남", "female": "여"}

#: Classic arrow item sprites, in priority order; the first one that exists in
#: the archives is used. The big `ac_chargearrow` skill-effect sprite is the
#: last resort fallback (it is much larger than a flying arrow).
ARROW_ASSET_CANDIDATES = ["화살", "강철화살", "독화살", "불화살", "사냥용화살", "그림자의화살", "날카로운화살", "녹슨화살", "돌화살", "수정화살"]
ARROW_FALLBACK_ASSETS = ["ac_chargearrow", "ma_chargearrow"]

#: Flight time of the visual arrow projectile in milliseconds (the classic
#: client's arrow flight is a short 0.2-0.35 s hop; 0.28 s was tuned in
#: `feat/arrow-projectile` and is kept as data so it can be adjusted without a
#: rebuild).
ARROW_FLIGHT_MS = 280


def load_json(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"facts file not found: {path}")
    return json.loads(path.read_text())


def compact_ranges(ids: list[int]) -> list[tuple[int, int]]:
    """Merge sorted ids into disjoint ``[start, end]`` ranges."""
    ranges: list[tuple[int, int]] = []
    for value in sorted(set(ids)):
        if ranges and value == ranges[-1][1] + 1:
            ranges[-1] = (ranges[-1][0], value)
        else:
            ranges.append((value, value))
    return ranges


def ron_str(value: str) -> str:
    """Escape a string for RON (backslashes in sprite paths are the important part)."""
    return json.dumps(value, ensure_ascii=False)


def ron_list(values: list[str]) -> str:
    return "[" + ", ".join(ron_str(value) for value in values) + "]"


def ron_enum_list(values: list[str]) -> str:
    return "[" + ", ".join(values) + "]"


def build(items: dict, assets: dict) -> dict:
    kinds = {row["kind"]: row for row in KINDS}
    subtype_kind = {}
    for row in KINDS:
        for subtype in row["subtypes"]:
            subtype_kind[subtype] = row["kind"]

    known_variants = assets["variants"]
    item_sprite_names = set(assets["ammo_item_sprites"])

    # Resolve the arrow projectile assets against the archive facts.
    arrow_assets = [f"아이템\\{name}" for name in ARROW_ASSET_CANDIDATES if name in item_sprite_names]
    arrow_fallbacks = [f"아이템\\{name}" for name in ARROW_FALLBACK_ASSETS if name in item_sprite_names]
    if not arrow_assets and not arrow_fallbacks:
        raise SystemExit("no arrow projectile sprite found in the client assets facts")

    # Validate that every configured variant actually exists in the archives.
    for row in KINDS:
        for variant in row["variants"]:
            if variant not in known_variants:
                raise SystemExit(f"kind {row['kind']}: sprite family {variant!r} does not exist in the client archives")

    # Group weapon items by kind and derive their ranges.
    items_by_kind: dict[str, list[dict]] = collections.defaultdict(list)
    views: list[dict] = []
    for item in items["items"]:
        kind = subtype_kind.get(item["subtype"])
        if kind is None:
            raise SystemExit(f"item {item['id']} ({item['aegis']}): unknown subtype {item['subtype']!r}")
        items_by_kind[kind].append(item)
        if item["view"]:
            views.append({"view": item["view"], "kind": kind, "item": item["aegis"]})

    # Most common item range of the kind is the authoritative default; keep the
    # generated value so the Rust module does not repeat the item DB.
    for row in KINDS:
        kind_items = items_by_kind.get(row["kind"], [])
        if kind_items:
            row["range"] = collections.Counter(item["range"] for item in kind_items).most_common(1)[0][0]

    # Map every weapon value to a kind (server types from the enum, item ids
    # from the ranges below).
    server_types = []
    for server_type in items["server_weapon_types"]:
        kind = SERVER_TYPE_KINDS.get(server_type["name"])
        if kind is None:
            raise SystemExit(f"no weapon kind for server type {server_type['name']}")
        server_types.append({"value": server_type["value"], "kind": kind, "name": server_type["name"]})
    server_values = [row["value"] for row in server_types]
    if len(server_values) != len(set(server_values)):
        raise SystemExit("duplicate server weapon type values")

    item_ranges = []
    for kind, kind_items in sorted(items_by_kind.items()):
        for start, end in compact_ranges([item["id"] for item in kind_items]):
            item_ranges.append({"kind": kind, "start": start, "end": end})
    item_ranges.sort(key=lambda row: row["start"])

    # The ranges must be disjoint: an id can only belong to one kind.
    for previous, current in zip(item_ranges, item_ranges[1:]):
        if current["start"] <= previous["end"]:
            raise SystemExit(f"overlapping item ranges: {previous} and {current}")

    return {
        "kinds": KINDS,
        "server_types": server_types,
        "item_ranges": item_ranges,
        "item_views": views,
        "projectiles": [
            {
                "kind": "Arrow",
                "assets": arrow_assets,
                "fallback_assets": arrow_fallbacks,
                "flight_ms": ARROW_FLIGHT_MS,
            }
        ],
        "items_by_kind": items_by_kind,
    }


def render(data: dict, items_provenance: dict, assets_provenance: dict) -> str:
    lines: list[str] = []
    add = lines.append

    item_db_sha = items_provenance.get("item_db_sha256", "")
    server_head = items_provenance.get("server_head", "")
    archives = ", ".join(f"{archive['name']} ({archive['files']} files)" for archive in assets_provenance.get("archives", []))
    item_count = sum(len(rows) for rows in data["items_by_kind"].values())

    add("// Data-driven weapon registry for RO-Next.")
    add("//")
    add("// Generated by `tools/gen_weapons.py`; do not edit by hand.")
    add("// Regenerate with: `./tools/gen_weapons.py`")
    add("// Drift check:      `./tools/gen_weapons.py --check`")
    add("//")
    add("// Sources (committed facts, no game files needed to regenerate):")
    add(f"//   * tools/rathena_pre_re_weapon_items.json — rAthena `db/pre-re/item_db_equip.yml`")
    add(f"//     of the CondaRO pre-renewal server (sha256 {item_db_sha}, server HEAD {server_head or 'unknown'});")
    add(f"//     {item_count} weapon items. `enum weapon_type` / `enum e_ammo_type` from `src/map/pc.hpp`.")
    add(f"//   * tools/weapon_client_assets.json — CondaRO client archives: {archives}.")
    add("//     Per-job weapon variant sprites and projectile item sprites that actually exist.")
    add("//")
    add("// The file is loaded at runtime through the normal client file loader")
    add("// (`data\\client\\weapons.ron`), so a server can override the weapon table")
    add("// without rebuilding the client. The compiled copy below is the fallback.")
    add("//")
    add("// Candidate paths are built from `paths` + `fallbacks`; the client never")
    add("// contains a hard-coded weapon type/view-id/suffix/arrow table.")
    add("(")
    provenance = (
        "rAthena pre-re item_db_equip.yml sha256 "
        + item_db_sha
        + " (CondaRO server) + CondaRO client archives; generated by tools/gen_weapons.py"
    )
    add(f"    provenance: {ron_str(provenance)},")
    add("    version: 1,")
    add("    paths: (")
    add(f'        per_job: {ron_str(PATHS["per_job"])},')
    add(f'        classic_weapon: {ron_str(PATHS["classic_weapon"])},')
    add("    ),")
    add("    sex_suffix: (")
    add(f'        male: {ron_str(SEX_SUFFIX["male"])},')
    add(f'        female: {ron_str(SEX_SUFFIX["female"])},')
    add("    ),")
    add("    fallbacks: (")
    for value_class in ("server_type", "item_view", "unknown"):
        add(f"        {value_class}: {ron_enum_list(FALLBACKS[value_class])},")
    add("    ),")
    add("    // rAthena `enum weapon_type` value (sent as the entity `weapon` field) -> kind.")
    add("    server_types: [")
    for row in data["server_types"]:
        add(f'        (value: {row["value"]}, kind: {row["kind"]}, name: {ron_str(row["name"])}),')
    add("    ],")
    add("    // One row per weapon kind.")
    add("    //   variants:  classic client sprite family names (ordered fallbacks).")
    add("    //   range:     most common rAthena weapon range of the kind (tiles).")
    add("    //   ammo:      rAthena ammo kind the weapon uses (Arrow/Bullet/Grenade).")
    add("    //   projectile: visual projectile spawned on auto attacks (only Arrow has a")
    add("    //               verified flying sprite in this data set).")
    add("    kinds: [")
    for row in data["kinds"]:
        ammo = f"Some({row['ammo']})" if row["ammo"] else "None"
        projectile = f"Some({row['projectile']})" if row["projectile"] else "None"
        add("        (")
        add(f"            kind: {row['kind']},")
        subtype = row["subtypes"][0] if row["subtypes"] else ""
        add(f'            subtype: {ron_str(subtype)},')
        add(f"            variants: {ron_list(row['variants'])},")
        add(f"            range: {row['range']},")
        add(f"            ammo: {ammo},")
        add(f"            projectile: {projectile},")
        add("        ),")
    add("    ],")
    add("    // Pre-renewal weapon item ids (`Type: Weapon`) as disjoint ascending ranges.")
    add("    // Used when an entity's weapon field carries an item view id instead of a")
    add("    // server weapon type (population shells / custom servers).")
    add("    item_ranges: [")
    for row in data["item_ranges"]:
        add(f'        (kind: {row["kind"]}, start: {row["start"]}, end: {row["end"]}),')
    add("    ],")
    add("    // Explicit `View:` overrides from the item database (empty for pre-re).")
    add("    item_views: [")
    for row in data["item_views"]:
        add(f'        (view: {row["view"]}, kind: {row["kind"]}),')
    add("    ],")
    add("    // Flying projectile sprites for ranged auto attacks.")
    add("    projectiles: [")
    for row in data["projectiles"]:
        add("        (")
        add(f"            kind: {row['kind']},")
        add(f"            assets: {ron_list(row['assets'])},")
        add(f"            fallback_assets: {ron_list(row['fallback_assets'])},")
        add(f"            flight_ms: {row['flight_ms']},")
        add("        ),")
    add("    ],")
    add(")")
    add("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--items", default=str(DEFAULT_ITEMS))
    parser.add_argument("--assets", default=str(DEFAULT_ASSETS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    args = parser.parse_args()

    items = load_json(Path(args.items))
    assets = load_json(Path(args.assets))
    data = build(items, assets)
    content = render(data, items["provenance"], assets["provenance"])

    output = Path(args.output)
    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_weapons.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(data['kinds'])} kinds, {len(data['item_ranges'])} id ranges)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    print(
        f"{output}: {len(data['kinds'])} kinds, {len(data['server_types'])} server types, "
        f"{len(data['item_ranges'])} id ranges, {sum(len(rows) for rows in data['items_by_kind'].values())} items, "
        f"{len(data['projectiles'])} projectiles"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
