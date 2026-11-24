#!/usr/bin/env python3
"""Invariant tests for the generated weapon registry data.

Read-only checks over the committed artefacts of the weapon centralization:

* ``tools/data/rathena_pre_re_weapon_items.json`` — the extracted server facts
  (every pre-renewal weapon item, the rAthena ``enum weapon_type`` and
  ``enum e_ammo_type`` values),
* ``tools/data/weapon_client_assets.json`` — the client-archive facts (which
  per-job weapon sprite families and projectile sprites exist),
* ``ro-next/archive/data/client/weapons.ron`` — the generated data file
  consumed by ``ro-next/src/world/weapon.rs``,
* ``tools/gen_weapons.py`` — the generator + ``--check`` drift guard.

The tests assert that the facts are complete, that the generated file is up to
date, that every weapon item id resolves to exactly one kind and that the
projectile/ammo configuration matches rAthena's weapon rules.

Run:
    ./tools/test_gen_weapons.py
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ITEMS = REPO / "tools/data/rathena_pre_re_weapon_items.json"
ASSETS = REPO / "tools/data/weapon_client_assets.json"
RON = REPO / "ro-next/archive/data/client/weapons.ron"
GENERATOR = REPO / "tools/gen_weapons.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class WeaponFactsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.items = json.loads(ITEMS.read_text())
        cls.assets = json.loads(ASSETS.read_text())
        cls.gen = load_module(GENERATOR, "gen_weapons")
        cls.data = cls.gen.build(cls.items, cls.assets)

    def test_item_facts_are_complete(self) -> None:
        provenance = self.items["provenance"]
        self.assertEqual(provenance["item_db"], "item_db_equip.yml")
        self.assertEqual(len(provenance["item_db_sha256"]), 64, "sha256 of the item database")
        self.assertEqual(provenance["weapon_items"], 707)

        items = self.items["items"]
        self.assertEqual(len(items), 707)
        ids = [item["id"] for item in items]
        self.assertEqual(len(ids), len(set(ids)), "item ids must be unique")
        self.assertEqual(ids, sorted(ids))
        for item in items:
            self.assertTrue(item["aegis"], item)
            self.assertGreaterEqual(item["range"], 1, item)
            self.assertGreaterEqual(item["weapon_level"], 1, item)
            self.assertGreaterEqual(item["view"], 0, item)

    def test_server_weapon_types_match_rAthena(self) -> None:
        types = {row["name"]: row["value"] for row in self.items["server_weapon_types"]}
        expected = {
            "W_FIST": 0,
            "W_DAGGER": 1,
            "W_1HSWORD": 2,
            "W_2HSWORD": 3,
            "W_1HSPEAR": 4,
            "W_2HSPEAR": 5,
            "W_1HAXE": 6,
            "W_2HAXE": 7,
            "W_MACE": 8,
            "W_2HMACE": 9,
            "W_STAFF": 10,
            "W_BOW": 11,
            "W_KNUCKLE": 12,
            "W_MUSICAL": 13,
            "W_WHIP": 14,
            "W_BOOK": 15,
            "W_KATAR": 16,
            "W_REVOLVER": 17,
            "W_RIFLE": 18,
            "W_GATLING": 19,
            "W_SHOTGUN": 20,
            "W_GRENADE": 21,
            "W_HUUMA": 22,
            "W_2HSTAFF": 23,
            # The dual-wield display values continue after MAX_WEAPON_TYPE.
            "W_DOUBLE_DD": 25,
            "W_DOUBLE_SS": 26,
            "W_DOUBLE_AA": 27,
            "W_DOUBLE_DS": 28,
            "W_DOUBLE_DA": 29,
            "W_DOUBLE_SA": 30,
        }
        self.assertEqual(types, expected)

        ammo = {row["name"]: row["value"] for row in self.items["ammo_types"]}
        self.assertEqual(ammo["AMMO_ARROW"], 1)
        self.assertEqual(ammo["AMMO_BULLET"], 3)
        self.assertEqual(ammo["AMMO_GRENADE"], 5)

    def test_client_asset_facts_document_the_data_set_limitation(self) -> None:
        self.assertGreater(self.assets["per_job_variant_files"], 0)
        self.assertTrue(self.assets["variants"], "per-job weapon variant families")
        # The CondaRO data set has no classic job-independent weapon layer and no
        # official weapon-name table; this is why the sprite family names come
        # from the generated data file instead of client data.
        self.assertFalse(self.assets["classic_weapon_directory"]["present"])
        self.assertFalse(self.assets["weapon_info_table_present"])
        self.assertIn("화살", self.assets["ammo_item_sprites"], "the arrow item sprite must exist")


class WeaponGeneratorTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.items = json.loads(ITEMS.read_text())
        cls.assets = json.loads(ASSETS.read_text())
        cls.gen = load_module(GENERATOR, "gen_weapons_tested")
        cls.data = cls.gen.build(cls.items, cls.assets)
        cls.kinds = {row["kind"]: row for row in cls.data["kinds"]}

    def test_every_sprite_family_exists_in_the_client_archives(self) -> None:
        known = self.assets["variants"]
        for row in self.data["kinds"]:
            for variant in row["variants"]:
                self.assertIn(variant, known, f"kind {row['kind']} uses missing sprite family {variant!r}")
            self.assertEqual(len(row["variants"]), len(set(row["variants"])), f"duplicate variants in {row['kind']}")

    def test_every_server_weapon_type_maps_to_a_kind(self) -> None:
        server_types = self.data["server_types"]
        self.assertEqual(len(server_types), len(self.items["server_weapon_types"]))
        self.assertEqual(len({row["value"] for row in server_types}), len(server_types))
        for row in server_types:
            self.assertIn(row["kind"], self.kinds)
        # Every real rAthena weapon type is covered; only the MAX marker is not.
        values = {row["value"] for row in server_types}
        self.assertNotIn(24, values)
        self.assertEqual(values, set(range(0, 24)) | set(range(25, 31)))

    def test_item_ranges_are_disjoint_and_cover_every_weapon_id(self) -> None:
        ranges = self.data["item_ranges"]
        self.assertEqual(len(ranges), 47)
        previous_end = 0
        for row in ranges:
            self.assertLessEqual(row["start"], row["end"])
            self.assertGreater(row["start"], previous_end, "ranges must be disjoint and ascending")
            previous_end = row["end"]
            self.assertIn(row["kind"], self.kinds)

        covered = []
        for row in ranges:
            covered.extend(range(row["start"], row["end"] + 1))
        self.assertEqual(covered, sorted(item["id"] for item in self.items["items"]))

    def test_attack_range_comes_from_the_item_database(self) -> None:
        # A few well-known values from the pre-renewal item database.
        self.assertEqual(self.kinds["Dagger"]["range"], 1)
        self.assertEqual(self.kinds["OneHandedSpear"]["range"], 3)
        self.assertEqual(self.kinds["Bow"]["range"], 5)
        self.assertEqual(self.kinds["Whip"]["range"], 2)
        self.assertEqual(self.kinds["Revolver"]["range"], 7)
        self.assertEqual(self.kinds["Rifle"]["range"], 9)
        self.assertEqual(self.kinds["Grenade"]["range"], 9)

    def test_ranged_weapons_follow_rAthena_ammo_rules(self) -> None:
        # rAthena allows arrow ammo for bows, instruments and whips, bullet ammo
        # for the gunner weapons and grenade ammo for the grenade launcher.
        for kind in ("Bow", "MusicalInstrument", "Whip"):
            self.assertEqual(self.kinds[kind]["ammo"], "Arrow", kind)
            self.assertEqual(self.kinds[kind]["projectile"], "Arrow", kind)
        for kind in ("Revolver", "Rifle", "Gatling", "Shotgun"):
            self.assertEqual(self.kinds[kind]["ammo"], "Bullet", kind)
            # No verified flying bullet sprite in this data set: no visual
            # projectile (documented; the ammo kind is still authoritative).
            self.assertIsNone(self.kinds[kind]["projectile"], kind)
        self.assertEqual(self.kinds["Grenade"]["ammo"], "Grenade")

    def test_projectile_assets_exist_in_the_client_archives(self) -> None:
        self.assertEqual(len(self.data["projectiles"]), 1)
        projectile = self.data["projectiles"][0]
        self.assertEqual(projectile["kind"], "Arrow")
        self.assertTrue(projectile["assets"], "preferred arrow sprites")
        self.assertIn("아이템\\화살", projectile["assets"])
        known = self.assets["ammo_item_sprites"]
        for asset in projectile["assets"] + projectile["fallback_assets"]:
            self.assertTrue(asset.startswith("아이템\\"), asset)
            self.assertIn(asset.removeprefix("아이템\\"), known, asset)
        self.assertTrue(200 <= projectile["flight_ms"] <= 350, projectile["flight_ms"])

    def test_no_item_view_overrides_in_the_pre_re_database(self) -> None:
        # rAthena generates the client view id from the item id for all pre-re
        # weapons; explicit `View:` overrides would land here if a server adds
        # custom items, so the section is kept (and empty today).
        self.assertEqual(self.items["items"][0]["view"], 0)
        self.assertEqual(self.data["item_views"], [])

    def test_ron_strings_escape_backslashes(self) -> None:
        content = self.gen.render(self.data, self.items["provenance"], self.assets["provenance"])
        # Sprite paths are Windows-style paths; RON needs them escaped.
        self.assertIn('per_job: "인간족\\\\{folder}\\\\{prefix}_{sex}_{variant}"', content)
        self.assertIn('"아이템\\\\화살"', content)

    def test_generator_check_reports_the_committed_file_as_current(self) -> None:
        result = subprocess.run(
            [sys.executable, str(GENERATOR), "--check"],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr or result.stdout)
        self.assertIn("is up to date", result.stdout)
        self.assertTrue(RON.is_file())


if __name__ == "__main__":
    unittest.main()
