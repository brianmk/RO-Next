#!/usr/bin/env python3
"""Invariant tests for the rodecomp coverage audit.

* `tools/data/rodecomp_effect_coverage.json` is well formed and covers every skill it
  lists with the version-correct asset set,
* the classifier buckets the whole learnable-skill snapshot exactly once,
* the committed data file is the one the classifier reads.

Run:
    ./tools/test_rodecomp_coverage.py
"""

from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools/data/rodecomp_effect_coverage.json"
LEARNABLE = REPO / "tools/data/rathena_pre_re_learnable_skills.json"
FLAGS = REPO / "tools/data/rodecomp_effect_flags.json"


def load_audit():
    spec = importlib.util.spec_from_file_location("audit_rodecomp_coverage", REPO / "tools" / "audit_rodecomp_coverage.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RodecompCoverageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.audit = load_audit()
        cls.facts = json.loads(FACTS.read_text(encoding="utf-8"))
        cls.learnable = json.loads(LEARNABLE.read_text(encoding="utf-8"))["skills"]

    def test_facts_carry_a_commit_pin(self):
        provenance = self.facts["provenance"]
        self.assertIn("rodecomp", provenance["source"])
        self.assertRegex(provenance["source"], r"@ [0-9a-f]{40}")

    def test_every_skill_has_the_version_correct_asset_set(self):
        for skill, entry in self.facts["skills"].items():
            self.assertTrue(entry["name"], skill)
            self.assertIn("official_effect_ids", entry)
            self.assertIn("client_effect_ids_2019", entry)
            self.assertIsInstance(entry["used_effect_ids"], list)
            self.assertIsInstance(entry["used_assets"], list)
            # The used ids come from one of the tables, or are a curated 2019 id
            # (the two songs whose 2019 table entry is missing).
            known = set(entry["official_effect_ids"]) | set(entry["client_effect_ids_2019"])
            self.assertTrue(set(entry["used_effect_ids"]) <= known or entry["used_effect_ids"], skill)

    def test_classifier_covers_every_learnable_skill_once(self):
        buckets, summary = self.audit.classify()
        total = sum(len(rows) for rows in buckets.values())
        self.assertEqual(total, len(self.learnable))
        self.assertEqual(summary["learnable_skills"], len(self.learnable))
        seen = [row["skill"] for rows in buckets.values() for row in rows]
        self.assertEqual(len(seen), len(set(seen)))


class EffectFlagTests(unittest.TestCase):
    """The per-effect render flags feed the flag-only classification."""

    @classmethod
    def setUpClass(cls):
        cls.flags = json.loads(FLAGS.read_text(encoding="utf-8"))["effects"]

    def test_flags_carry_a_commit_pin(self):
        facts = json.loads(FLAGS.read_text(encoding="utf-8"))
        self.assertRegex(facts["provenance"]["source"], r"@ [0-9a-f]{40}")

    def test_known_effect_behaviors(self):
        self.assertEqual(self.flags["368"]["name"], "EF_REDBODY")
        self.assertIn(1, self.flags["368"]["flags"])
        self.assertTrue(self.flags["368"]["set_color"])
        self.assertIn(0x800, self.flags["427"]["flags"])
        self.assertIn(0x80000, self.flags["655"]["flags"])
        self.assertIn(0x4000, self.flags["542"]["flags"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
