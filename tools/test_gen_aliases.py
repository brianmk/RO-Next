#!/usr/bin/env python3
"""Tests for the generated alias data pipeline.

Run directly (`python3 tools/test_gen_aliases.py`) or through CI. No game
files, server tree or Python packages are needed: only the committed facts
(`tools/data/alias_facts.json`), the legacy parity snapshot
(`tools/data/alias_legacy_parity.json`) and the committed data file.

The important test is `test_generated_tables_match_the_removed_rust_tables`:
the data file must reproduce the three hand-maintained Rust tables of the
integration HEAD before this lane entry-for-entry, so the migration is a pure
data move and cannot change rendering.
"""
from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(TOOLS_DIR))

import gen_aliases  # noqa: E402


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


class AliasDataTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.facts = gen_aliases.load_facts()
        cls.legacy = load_json(TOOLS_DIR / "data" / "alias_legacy_parity.json")
        cls.aliases = gen_aliases.build_aliases(cls.facts)

    def mutated_facts(self) -> dict:
        return copy.deepcopy(self.facts)

    def test_generated_tables_match_the_removed_rust_tables(self) -> None:
        """Entry-for-entry parity with the old Rust tables (no gameplay change)."""
        for key in (
            "job_skill_tree_fallbacks",
            "monster_sprite_aliases",
            "skill_icon_aliases",
        ):
            self.assertEqual(self.legacy[key], [list(row) for row in self.aliases[key]], key)
        for key in ("generic_monster_sprites", "generic_skill_icons"):
            self.assertEqual(self.legacy[key], list(self.aliases[key]), key)

    def test_counts(self) -> None:
        self.assertEqual(len(self.aliases["job_skill_tree_fallbacks"]), 56)
        self.assertEqual(len(self.aliases["monster_sprite_aliases"]), 232)
        self.assertEqual(len(self.aliases["generic_monster_sprites"]), 7)
        self.assertEqual(len(self.aliases["skill_icon_aliases"]), 241)
        self.assertEqual(len(self.aliases["generic_skill_icons"]), 3)

    def test_rows_are_sorted_and_unique(self) -> None:
        for key, rows in (
            ("job_skill_tree_fallbacks", self.aliases["job_skill_tree_fallbacks"]),
            ("monster_sprite_aliases", self.aliases["monster_sprite_aliases"]),
            ("skill_icon_aliases", self.aliases["skill_icon_aliases"]),
        ):
            ids = [row[0] for row in rows]
            self.assertEqual(ids, sorted(ids), f"{key} is not sorted")
            self.assertEqual(len(ids), len(set(ids)), f"{key} has duplicate ids")

    def test_paths_are_sprite_stems(self) -> None:
        for _, path in self.aliases["monster_sprite_aliases"]:
            self.assertTrue(path.startswith(("몬스터\\", "npc\\")), path)
            self.assertFalse(path.endswith((".spr", ".act")), path)
            self.assertNotIn("missing", path.lower())
        for path in self.aliases["generic_monster_sprites"]:
            self.assertTrue(path.startswith("몬스터\\"), path)
        for _, path in self.aliases["skill_icon_aliases"]:
            self.assertTrue(path.startswith("아이템\\"), path)
            self.assertFalse(path.endswith((".spr", ".act")), path)
        for path in self.aliases["generic_skill_icons"]:
            self.assertTrue(path.startswith("아이템\\"), path)

    def test_job_fallbacks_do_not_chain(self) -> None:
        keys = {row[0] for row in self.aliases["job_skill_tree_fallbacks"]}
        for job_id, fallback in self.aliases["job_skill_tree_fallbacks"]:
            self.assertNotEqual(job_id, fallback)
            self.assertNotIn(fallback, keys, f"chained fallback {job_id} -> {fallback}")

    def test_data_file_is_up_to_date(self) -> None:
        rendered = gen_aliases.render(self.facts)
        committed = gen_aliases.OUTPUT_PATH.read_text(encoding="utf-8")
        self.assertEqual(rendered, committed, "run `python3 tools/gen_aliases.py`")

    def test_rendering_is_deterministic(self) -> None:
        first = gen_aliases.render(self.facts)
        second = gen_aliases.render(gen_aliases.load_facts())
        self.assertEqual(first, second)

    def test_facts_carry_provenance(self) -> None:
        provenance = self.facts["provenance"]
        self.assertEqual(self.facts["schema_version"], 1)
        self.assertEqual(len(provenance["server"]["head"]), 40)
        self.assertEqual(len(provenance["client"]["archives"]), 3)
        for archive in provenance["client"]["archives"]:
            self.assertEqual(len(archive["sha256"]), 64)
            self.assertGreater(archive["files"], 10_000)
        for table in provenance["client"]["tables"].values():
            self.assertEqual(len(table["sha256"]), 64)
        self.assertEqual(len(provenance["curated"]["job_skill_tree_fallbacks"]) > 0, True)

    def test_resolution_methods_are_known(self) -> None:
        monster_methods = {row["method"].split(":")[0] for row in self.facts["monsters"]["sprite_aliases"]}
        self.assertLessEqual(monster_methods, {"compact-monster", "family", "fuzzy", "id-override", "variant-monster", "variant-npc"})
        skill_methods = {row["method"].split(":")[0] for row in self.facts["skills"]["icon_aliases"]}
        self.assertLessEqual(skill_methods, {"compact", "explicit", "family", "npc-token-suffix", "variant"})

    def test_generator_rejects_duplicate_ids(self) -> None:
        facts = self.mutated_facts()
        facts["monsters"]["sprite_aliases"][1]["job_id"] = facts["monsters"]["sprite_aliases"][0]["job_id"]
        with self.assertRaisesRegex(ValueError, "duplicate id"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_unsorted_ids(self) -> None:
        facts = self.mutated_facts()
        facts["skills"]["icon_aliases"] = list(reversed(facts["skills"]["icon_aliases"]))
        with self.assertRaisesRegex(ValueError, "sorted"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_a_chained_job_fallback(self) -> None:
        facts = self.mutated_facts()
        facts["jobs"]["skills_tree_fallbacks"][0]["fallback_job_id"] = facts["jobs"]["skills_tree_fallbacks"][1]["job_id"]
        with self.assertRaisesRegex(ValueError, "chained"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_a_missing_fallback_tree(self) -> None:
        facts = self.mutated_facts()
        facts["jobs"]["skills_tree_fallbacks"][0]["target_tree_skills"] = 0
        with self.assertRaisesRegex(ValueError, "no client skill tree"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_bad_monster_paths(self) -> None:
        for bad_path in ("data\\sprite\\몬스터\\poring", "몬스터\\missing", "몬스터\\poring.spr"):
            facts = self.mutated_facts()
            facts["monsters"]["sprite_aliases"][0]["sprite_path"] = bad_path
            with self.assertRaisesRegex(ValueError, "monster 1016"):
                gen_aliases.build_aliases(facts)

    def test_generator_rejects_bad_icon_paths(self) -> None:
        facts = self.mutated_facts()
        facts["skills"]["icon_aliases"][0]["icon_path"] = "몬스터\\poring"
        with self.assertRaisesRegex(ValueError, "skill 96"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_missing_generics(self) -> None:
        facts = self.mutated_facts()
        facts["monsters"]["generic_sprites"] = []
        facts["monsters"]["summary"]["generic_sprites"] = 0
        with self.assertRaisesRegex(ValueError, "no generic monster sprites"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_a_count_mismatch(self) -> None:
        facts = self.mutated_facts()
        facts["skills"]["summary"]["aliases"] = 1
        with self.assertRaisesRegex(ValueError, "skill alias count"):
            gen_aliases.build_aliases(facts)

    def test_generator_rejects_an_unknown_facts_schema(self) -> None:
        facts = self.mutated_facts()
        facts["schema_version"] = 999
        with self.assertRaisesRegex(ValueError, "schema version"):
            gen_aliases.build_aliases(facts)


if __name__ == "__main__":
    unittest.main(verbosity=2)
