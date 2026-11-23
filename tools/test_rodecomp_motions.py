#!/usr/bin/env python3
"""Invariant tests for the extracted caster-motion facts.

`tools/rodecomp_skill_motions.json` is the audit artifact of the official
client's `CSession::GetSkillActionInfo` / `GetSkillActionInfo2` motions (the
actor states the caster is put into after a skill use).

Run:
    ./tools/test_rodecomp_motions.py
"""

from __future__ import annotations

import json
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools" / "rodecomp_skill_motions.json"
LEARNABLE = REPO / "tools" / "rathena_pre_re_learnable_skills.json"


class RodecompMotionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = json.loads(FACTS.read_text(encoding="utf-8"))
        cls.learnable = {int(key) for key in json.loads(LEARNABLE.read_text(encoding="utf-8"))["skills"]}

    def test_facts_carry_a_commit_pin(self):
        provenance = self.facts["provenance"]
        self.assertIn("rodecomp", provenance["source"])
        self.assertRegex(provenance["source"], r"@ [0-9a-f]{40}")

    def test_every_state_motion_round_trips(self):
        states = self.facts["actor_states"]
        for skill, entry in self.facts["skills"].items():
            self.assertIsInstance(entry["motion"], int, skill)
            if str(entry["motion"]) in states:
                self.assertEqual(states[str(entry["motion"])], entry["state"], skill)
            else:
                self.assertIsNone(entry["state"], skill)
            self.assertTrue(entry["name"], skill)

    def test_learnable_subset_is_covered(self):
        learnable = [int(skill) for skill in self.facts["skills"] if int(skill) in self.learnable]
        self.assertGreaterEqual(len(learnable), 30)
        # The default motion is not an override, so this must stay a small,
        # meaningful subset of the 457 learnable skills.
        self.assertLess(len(learnable), 60)


if __name__ == "__main__":
    unittest.main(verbosity=2)
