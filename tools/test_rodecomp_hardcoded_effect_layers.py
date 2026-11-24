#!/usr/bin/env python3
"""Invariant tests for the hard-coded (decompilation) skill effect fragment.

Covers the artefacts of the 2019 effect-id gap lane:

* `tools/data/rodecomp_effect_layers.json` - the extracted facts (effect id -> layers,
  provenance line numbers, scope, documented skips),
* `tools/data/skill_effects_rodecomp_hardcoded.ron` - the generated fragment merged by
  `tools/gen_skill_effects.py --rodecomp-hardcoded`.

Run:
    ./tools/test_rodecomp_hardcoded_effect_layers.py
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools/data/rodecomp_effect_layers.json"
FRAGMENT = REPO / "tools/data/skill_effects_rodecomp_hardcoded.ron"
GENERATOR = REPO / "tools/gen_rodecomp_effect_layers.py"
GRF2019 = REPO / "tools/data/grf2019_skill_facts.json"

LAYER_FIELDS = {
    "kind",
    "anchor",
    "attached_entity",
    "head",
    "vertical",
    "file",
    "files",
    "frames",
    "frame_ms",
    "primitive",
    "offset",
    "scale",
    "size",
    "size_end",
    "duration_ms",
    "delay_ms",
    "hold_ms",
    "fade_ms",
    "count",
    "spread",
    "height",
    "bottom_radius",
    "top_radius",
    "rotation_speed",
    "grow",
    "alpha",
    "tint",
    "blend",
    "wav",
    "light",
    "light_intensity",
}
KINDS = {"Str", "Spr", "Texture", "Cylinder", "Primitive", "BodyFlash"}
PHASES = {"Ground", "Cast", "Success", "Release", "Hit", "Caster"}


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def load_generator() -> object:
    spec = importlib.util.spec_from_file_location("gen_skill_effects", REPO / "tools/gen_skill_effects.py")
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class RodecompHardcodedEffectTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.facts = load_json(FACTS)
        cls.grf2019 = load_json(GRF2019)["skills"]
        cls.rows = load_generator().read_effects(FRAGMENT)
        cls.covered = {row["skill"] for row in cls.rows}

    def test_fragment_uses_only_the_loader_schema(self) -> None:
        self.assertTrue(self.rows, "the fragment must not be empty")
        for row in self.rows:
            self.assertIn(row["phase"], PHASES, row)
            for layer in row.get("layers", []):
                self.assertIn(layer["kind"], KINDS, layer)
                unknown = set(layer) - LAYER_FIELDS
                self.assertFalse(unknown, f"skill {row['skill']}: unknown layer fields {unknown}")
                if layer["kind"] in ("Texture", "Cylinder", "Primitive"):
                    self.assertTrue(layer.get("file"), layer)
                if layer["kind"] == "Str":
                    self.assertTrue(str(layer.get("file", "")).lower().endswith(".str"), layer)
                if layer["kind"] == "Spr":
                    self.assertTrue(layer.get("files"), layer)
                if layer["kind"] == "Primitive":
                    self.assertEqual(layer.get("primitive"), "Projectile", layer)
                    self.assertIn(layer.get("anchor"), {"Travel", "Rain"}, layer)
                self.assertLessEqual(layer.get("duration_ms", 0), 30_000)
                self.assertLessEqual(layer.get("count", 1), 16)

    def test_scope_skills_are_either_covered_or_documented(self) -> None:
        scope = [int(skill) for skill in self.facts["scope"]["skills"]]
        skipped = {int(skill) for skill in self.facts["skipped_skills"]}
        self.assertEqual(len(scope), 43, "the scope is the 43-skills parity gap")
        self.assertEqual(len(set(scope)), 43)
        self.assertEqual(scope, sorted(scope))
        missing = [skill for skill in scope if skill not in self.covered and skill not in skipped]
        self.assertFalse(missing, f"scoped skills with neither rows nor a skip reason: {missing}")
        self.assertTrue(skipped <= set(scope))
        for skill_id, entry in self.facts["skipped_skills"].items():
            self.assertTrue(entry.get("reason"), f"skill {skill_id} needs a reason")

    def test_every_fact_effect_id_is_reachable_or_skipped(self) -> None:
        skipped = {int(key) for key in self.facts["skipped_effect_ids"]}
        for effect_id, entry in self.facts["effects"].items():
            self.assertTrue(entry.get("source"), f"effect {effect_id} needs source line provenance")
            self.assertTrue(entry.get("layers"), f"effect {effect_id} has no layers")
            self.assertNotIn(int(effect_id), skipped)
        for effect_id, entry in self.facts["skipped_effect_ids"].items():
            self.assertTrue(entry.get("name") and entry.get("reason"), f"skipped effect {effect_id} needs a name and reason")

        # Every non-skipped effect id referenced by a scoped skill is mapped.
        scope = {int(skill) for skill in self.facts["scope"]["skills"]}
        known = {int(key) for key in self.facts["effects"]} | skipped
        for skill in scope:
            for kind_ids in self.grf2019.get(str(skill), {}).get("effects", {}).values():
                for effect_id in kind_ids:
                    self.assertIn(int(effect_id), known, f"skill {skill} references unmapped effect {effect_id}")

    def test_detail_generator_check_passes(self) -> None:
        result = subprocess.run(
            [sys.executable, str(GENERATOR), "--check"],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
