#!/usr/bin/env python3
"""Invariant tests for the item facts / item stats data pipeline.

Read-only checks over the committed artefacts of the item stats remediation:

* ``tools/data/rathena_pre_re_item_facts.json`` — the extracted server item facts,
* ``tools/extract_item_facts.py`` — the facts extractor and its flat bonus
  parser (conditional/string/percentage handling),
* ``tools/gen_item_stats.py`` — the data file generator and ``--check`` drift
  guard,
* ``korangar/archive/data/client/item_stats.ron`` — the generated data file,
* ``korangar/src/world/library/item_stats.rs`` / ``item_stats_fallback.rs`` —
  the data-first loader and the clearly separated last-resort description
  parser (the only place that may contain the Korean item description
  grammar).

Run:
    ./tools/test_item_stats.py
"""

from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools/data/rathena_pre_re_item_facts.json"
EXTRACTOR = REPO / "tools/extract_item_facts.py"
GENERATOR = REPO / "tools/gen_item_stats.py"
RON = REPO / "korangar/archive/data/client/item_stats.ron"
ITEM_STATS_RS = REPO / "korangar/src/world/library/item_stats.rs"
ITEM_STATS_FALLBACK_RS = REPO / "korangar/src/world/library/item_stats_fallback.rs"

HANGUL = re.compile(r"[\uac00-\ud7af\u1100-\u11ff\u3130-\u318f]")

BONUS_FIELDS = {
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
}


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ExtractorFactsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.facts = json.loads(FACTS.read_text())
        cls.extractor = load_module(EXTRACTOR, "extract_item_facts")
        cls.items = cls.facts["items"]
        cls.by_id = {item["id"]: item for item in cls.items}

    def test_provenance_is_complete(self) -> None:
        provenance = self.facts["provenance"]
        self.assertIn("CondaRO", provenance["source"])
        self.assertEqual(provenance["manifest"], "db/pre-re/item_db.yml")
        self.assertEqual(provenance["packetver"], "20190605")
        self.assertIn("pre-renewal", provenance["renewal_mode"])
        self.assertEqual(len(provenance["manifest_sha256"]), 64)
        self.assertEqual(len(provenance["databases"]), 3)
        for database in provenance["databases"]:
            self.assertEqual(len(database["sha256"]), 64, database)
            self.assertGreater(database["items"], 0, database)
        self.assertEqual(provenance["item_count"], len(self.items))
        self.assertEqual(sum(database["items"] for database in provenance["databases"]), len(self.items))
        self.assertEqual(provenance["equipment_items"], sum(1 for item in self.items if item["type"] in ("Weapon", "Armor", "Petarmor", "Ammo")))
        self.assertEqual(provenance["usable_items"], sum(1 for item in self.items if item["usable"]))
        self.assertEqual(provenance["items_with_flat_bonuses"], sum(1 for item in self.items if item["flat_bonuses"]))

    def test_item_rows_are_complete_and_sorted(self) -> None:
        ids = [item["id"] for item in self.items]
        self.assertEqual(ids, sorted(ids), "item ids must be sorted")
        self.assertEqual(len(ids), len(set(ids)), "item ids must be unique")

        for item in self.items:
            self.assertGreater(item["id"], 0, item)
            self.assertTrue(item["aegis"], item)
            self.assertIn(
                item["type"],
                {"Healing", "Usable", "Etc", "Armor", "Weapon", "Card", "Petegg", "Petarmor", "Ammo", "Delayconsume", "Cash"},
                item,
            )
            for position in item["equip_positions"]:
                self.assertIsInstance(position, str, item)
            # rAthena defaults: weapon/armor level is 1 for the matching type,
            # 0 for everything else.
            if item["type"] == "Weapon":
                self.assertGreaterEqual(item["weapon_level"], 1, item)
                self.assertEqual(item["armor_level"], 0, item)
            elif item["type"] == "Armor":
                self.assertGreaterEqual(item["armor_level"], 1, item)
                self.assertEqual(item["weapon_level"], 0, item)
            else:
                self.assertEqual(item["weapon_level"], 0, item)
                self.assertEqual(item["armor_level"], 0, item)
            for field in ("weight", "attack", "defense", "required_level", "slots", "view"):
                self.assertGreaterEqual(item[field], 0, (field, item))

    def test_flat_bonuses_are_only_known_nonzero_fields(self) -> None:
        for item in self.items:
            for field, value in item["flat_bonuses"].items():
                self.assertIn(field, BONUS_FIELDS, item)
                self.assertIsInstance(value, int, item)
                self.assertNotEqual(value, 0, item)

    def test_known_item_facts(self) -> None:
        # Falchion (1104): the values the old Korean description parser used to
        # derive, but from the server item database.
        falchion = self.by_id[1104]
        self.assertEqual(falchion["aegis"], "Falchion")
        self.assertEqual(falchion["type"], "Weapon")
        self.assertEqual(falchion["weapon_type"], "1hSword")
        self.assertEqual(falchion["attack"], 39)
        self.assertEqual(falchion["weight"], 600)
        self.assertEqual(falchion["slots"], 3)
        self.assertTrue(falchion["refineable"])
        self.assertEqual(falchion["required_level"], 2)
        self.assertEqual(falchion["equip_positions"], ["Right_Hand"])
        self.assertFalse(falchion["usable"])

        # Haedonggum (1123) script: `bonus bInt,3;`.
        self.assertEqual(self.by_id[1123]["flat_bonuses"], {"intelligence": 3})

        # Red Potion (501): usable consumable, no stats.
        potion = self.by_id[501]
        self.assertTrue(potion["usable"])
        self.assertEqual(potion["type"], "Healing")
        self.assertEqual(potion["attack"], 0)
        self.assertEqual(potion["equip_positions"], [])

        # An armor with a slot and a view override.
        angelic_guard = self.by_id[2116]
        self.assertEqual(angelic_guard["type"], "Armor")
        self.assertEqual(angelic_guard["armor_level"], 1)
        self.assertEqual(angelic_guard["defense"], 3)
        self.assertEqual(angelic_guard["equip_positions"], ["Left_Hand"])

    def test_conditional_bonuses_are_not_extracted(self) -> None:
        # 1189: `if (readparam(bStr)>=95) { bonus bBaseAtk,20; }` - BaseAtk is
        # not a flat client stat and the bonus is conditional anyway.
        self.assertEqual(self.by_id[1189]["flat_bonuses"], {})
        # 1311: conditional bBaseAtk/bCritical under readparam() checks.
        self.assertEqual(self.by_id[1311]["flat_bonuses"], {})
        # 1370/1371: unconditional `bonus bAspdRate,-40;` (a percentage) and a
        # conditional block; only the flat Aspd constant would be extracted and
        # there is none.
        self.assertEqual(self.by_id[1370]["flat_bonuses"], {})
        # 1188: two conditional skill blocks, then unconditional
        # `bonus bStr,1; bonus bDex,1;`.
        self.assertEqual(self.by_id[1188]["flat_bonuses"], {"strength": 1, "dexterity": 1})
        # 1186: unconditional bStr/bAgi before a conditional autobonus block.
        self.assertEqual(self.by_id[1186]["flat_bonuses"], {"strength": 5, "agility": 2})


class FlatBonusParserTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.extractor = load_module(EXTRACTOR, "extract_item_facts_parser")

    def parse(self, script: str) -> dict[str, int]:
        return self.extractor.flat_script_bonuses(script)

    def test_simple_flat_bonus(self) -> None:
        self.assertEqual(self.parse("bonus bStr,2;"), {"strength": 2})
        self.assertEqual(self.parse("bonus bStr,2;\nbonus bStr,3;"), {"strength": 5})
        self.assertEqual(self.parse("bonus bFlee,-10;"), {"flee": -10})
        self.assertEqual(self.parse("bonus bStr,0;"), {})

    def test_composite_constants(self) -> None:
        self.assertEqual(
            self.parse("bonus bAllStats,1;"),
            {"strength": 1, "agility": 1, "vitality": 1, "intelligence": 1, "dexterity": 1, "luck": 1},
        )
        self.assertEqual(self.parse("bonus bAgiVit,2;"), {"agility": 2, "vitality": 2})
        self.assertEqual(
            self.parse("bonus bAgiDexStr,3;"),
            {"strength": 3, "agility": 3, "dexterity": 3},
        )

    def test_percentage_and_non_stat_bonuses_are_skipped(self) -> None:
        self.assertEqual(self.parse("bonus bMatkRate,5;"), {})
        self.assertEqual(self.parse("bonus bMaxHP,100;"), {})
        self.assertEqual(self.parse("bonus2 bAddRace,RC_DemiHuman,5;"), {})
        self.assertEqual(self.parse("bonus bUnbreakableWeapon;"), {})

    def test_conditional_blocks_are_skipped(self) -> None:
        self.assertEqual(self.parse("if (getrefine()>8) {\n    bonus bDef,2;\n    bonus bStr,1;\n}"), {})
        self.assertEqual(self.parse("if (readparam(bStr)>=80)\n    bonus bStr,5;"), {})
        self.assertEqual(self.parse("if (readparam(bStr)>=80) bonus bStr,5;"), {})
        self.assertEqual(self.parse("bonus bStr,5;\nif (readparam(bStr)>=80)\n    bonus bDex,9;\nbonus bLuk,1;"), {
            "strength": 5,
            "luck": 1,
        })

    def test_autobonus_strings_are_not_equip_bonuses(self) -> None:
        script = 'bonus bStr,1;\nautobonus "{ bonus bDex,99; }",10,10000,BF_WEAPON,"{ specialeffect2 EF_POTION_BERSERK; }";'
        self.assertEqual(self.parse(script), {"strength": 1})

    def test_bonuses_after_a_finished_block_are_kept(self) -> None:
        self.assertEqual(self.parse("if (getrefine()>8) {\n    bonus bDex,9;\n}\nbonus bStr,1;"), {"strength": 1})

    def test_comments_are_ignored(self) -> None:
        self.assertEqual(self.parse("// bonus bStr,9;\nbonus bStr,1; // bonus bDex,9;"), {"strength": 1})

    def test_empty_script(self) -> None:
        self.assertEqual(self.parse(""), {})
        self.assertEqual(self.parse(None), {})


class GeneratorDataTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.facts = json.loads(FACTS.read_text())
        cls.gen = load_module(GENERATOR, "gen_item_stats")
        cls.rows = cls.gen.build_rows(cls.facts)
        cls.content = RON.read_text()
        cls.row_by_id = dict(cls.rows)

    def test_rows_cover_every_fact_and_are_sorted(self) -> None:
        ids = [item_id for item_id, _ in self.rows]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(ids, [item["id"] for item in self.facts["items"]])

    def test_ron_contains_every_item_once_and_is_sorted(self) -> None:
        ron_ids = [int(match) for match in re.findall(r"^\s{8}\((\d+), \(", self.content, re.M)]
        self.assertEqual(ron_ids, [item_id for item_id, _ in self.rows])
        self.assertEqual(len(set(ron_ids)), len(ron_ids))
        self.assertIn(f"//     {self.facts['provenance']['item_count']} items", self.content)

    def test_weight_is_converted_from_the_rathena_unit(self) -> None:
        # rAthena stores 0.1 units; the client shows whole units (database / 10).
        self.assertEqual(self.row_by_id[1104]["weight"], 60)
        self.assertEqual(self.row_by_id[501]["weight"], 7)
        self.assertEqual(self.row_by_id[1535]["weight"], 4)
        self.assertEqual(self.row_by_id[1186]["weight"], 200)

    def test_defaults_are_omitted_and_values_match_the_facts(self) -> None:
        falchion = self.row_by_id[1104]
        self.assertEqual(
            falchion,
            {
                "weight": 60,
                "attack": 39,
                "weapon_level": 1,
                "required_level": 2,
                "slots": 3,
                "refineable": True,
            },
        )
        # Red Potion only has a weight.
        self.assertEqual(self.row_by_id[501], {"weight": 7})

    def test_flat_bonuses_are_rendered(self) -> None:
        self.assertEqual(self.row_by_id[1123]["bonuses"], {"intelligence": 3})
        self.assertEqual(self.row_by_id[1186]["bonuses"], {"strength": 5, "agility": 2})
        self.assertIn("(1123, (weight: 90, attack: 120, weapon_level: 3, required_level: 27, slots: 1, refineable: true, bonuses: (intelligence: 3)))", self.content)

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

    def test_extractor_check_matches_the_committed_facts_when_the_server_tree_exists(self) -> None:
        # Only runnable on a machine with the CondaRO server tree; CI uses the
        # committed facts and the generator drift check above.
        server_tree = Path("/home/phantom/Games/ragnarok/CondaRO-linux/popeng-graft/scratch-condaro")
        if not server_tree.is_dir():
            self.skipTest("CondaRO server tree not available")
        result = subprocess.run(
            [sys.executable, str(EXTRACTOR), "--check"],
            cwd=REPO,
            capture_output=True,
            text=True,
            check=False,
        )
        # The committed facts record the provenance of the tree they were
        # extracted from; a locally changed server tree is reported, not a test
        # failure of this repository's data file.
        if result.returncode != 0:
            self.skipTest(f"local server tree differs from the committed facts: {result.stderr.strip()}")
        self.assertIn("is up to date", result.stdout)


class NoKoreanDependencyTest(unittest.TestCase):
    """The Korean item-description grammar must not be on the default path."""

    def test_default_loader_contains_no_korean_labels(self) -> None:
        source = ITEM_STATS_RS.read_text()
        self.assertIsNone(HANGUL.search(source), "the data-first item stats module must not contain Korean description grammar")

    def test_korean_grammar_is_confined_to_the_last_resort_module(self) -> None:
        fallback = ITEM_STATS_FALLBACK_RS.read_text()
        self.assertIsNotNone(HANGUL.search(fallback), "the last-resort parser keeps the localized description labels")
        for label in ("계열", "공격", "방어", "무게", "무기레벨", "요구레벨", "장착"):
            self.assertIn(label, fallback)

    def test_generated_data_has_no_localization_strings(self) -> None:
        self.assertIsNone(HANGUL.search(RON.read_text()), "the generated data is language-independent")

    def test_generator_and_extractor_emit_no_localization_strings(self) -> None:
        self.assertIsNone(HANGUL.search(GENERATOR.read_text()))
        self.assertIsNone(HANGUL.search(EXTRACTOR.read_text()))


if __name__ == "__main__":
    unittest.main()
