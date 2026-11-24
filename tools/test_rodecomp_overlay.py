#!/usr/bin/env python3
"""Invariant tests for the rodecomp-derived skill effect overlay.

* `tools/rodecomp_skill_facts.json` - the extracted official-client facts,
* `tools/skill_effects_rodecomp.ron` - the generated overlay (input of the
  unified pipeline),
* `tools/skill_effects_rodecomp.ron` rows must be valid v1 pipeline layers,
* the overlay must only add skills that had no descriptor in the committed
  unified data (it is for effects the data pipeline could not resolve),
* the overlay must be up to date (`--check`).

Run:
    ./tools/test_rodecomp_overlay.py
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools/rodecomp_skill_facts.json"
OVERLAY = REPO / "tools/skill_effects_rodecomp.ron"
MERGED = REPO / "korangar/archive/data/client/skill_effects.ron"
GENERATOR = REPO / "tools/gen_rodecomp_overlay.py"

#: Fields of `LayerFile` in `korangar/src/world/library/skill_effects.rs`.
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
    "action",
    "light",
    "light_intensity",
    "hold_ms",
    "fade_ms",
}


def load_generator():
    spec = importlib.util.spec_from_file_location("gen_skill_effects", REPO / "tools/gen_skill_effects.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class RodecompOverlayTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.gen = load_generator()
        cls.facts = json.loads(FACTS.read_text(encoding="utf-8"))
        cls.overlay = cls.gen.read_effects(OVERLAY)
        cls.merged = {int(entry["skill"]) for entry in cls.gen.read_effects(MERGED)}
        cls.learnable = {int(key) for key in json.loads((REPO / "tools/rathena_pre_re_learnable_skills.json").read_text())["skills"]}

    def test_facts_carry_provenance_and_a_commit_pin(self):
        provenance = self.facts["provenance"]
        self.assertIn("rodecomp", provenance["source"])
        self.assertRegex(provenance["source"], r"@ [0-9a-f]{40}")
        self.assertTrue(self.facts["skills"])

    def test_every_skill_is_learnable_pre_renewal(self):
        for entry in self.facts["skills"]:
            self.assertIn(int(entry["skill"]), self.learnable, f"skill {entry['skill']} is not a learnable pre-re skill")

    def test_layers_use_only_known_fields(self):
        for entry in self.overlay:
            for layer in entry["layers"]:
                unknown = set(layer) - LAYER_FIELDS
                self.assertFalse(unknown, f"skill {entry['skill']}: unknown layer fields {unknown}")
                self.assertIn(
                layer["kind"],
                {"Str", "Spr", "Texture", "Cylinder", "Primitive", "BodyFlash", "BodyWave", "BodyOutline", "BodySquash"},
            )

    def test_layer_values_are_in_schema_range(self):
        for entry in self.overlay:
            for layer in entry["layers"]:
                self.assertLessEqual(layer.get("duration_ms", 0), 30_000)
                self.assertLessEqual(layer.get("delay_ms", 0), 30_000)
                self.assertGreater(layer.get("scale", 1.0), 0.0)
                self.assertTrue(0.0 <= layer.get("alpha", 1.0) <= 1.0)
                if "file" in layer:
                    self.assertTrue(layer["file"].lower().endswith((".tga", ".bmp", ".str")), layer["file"])
                for frame in layer.get("frames", []):
                    self.assertTrue(frame.lower().endswith((".tga", ".bmp")), frame)
                if layer.get("frames"):
                    self.assertTrue(16 <= layer.get("frame_ms", 0) <= 2000, layer.get("frame_ms"))
                    self.assertNotIn("file", layer)

    def test_overlay_only_adds_or_extends_rows(self):
        # The overlay either adds a descriptor for a skill with no rows, or
        # appends the official layers a descriptor was missing. It must never
        # remove a layer, so every overlay asset has to be in the merged data.
        facts = {int(entry["skill"]): entry for entry in self.facts["skills"]}
        self.assertEqual(set(facts), {int(entry["skill"]) for entry in self.overlay})
        merged: dict[int, set[str]] = {}
        for row in self.gen.read_effects(MERGED):
            assets = merged.setdefault(int(row["skill"]), set())
            for layer in row.get("layers") or []:
                paths = ([layer["file"]] if layer.get("file") else []) + list(layer.get("files") or []) + list(layer.get("frames") or [])
                for path in paths:
                    assets.add(path.replace("/", "\\").lower().rsplit(".", 1)[0])
        for skill, entry in facts.items():
            self.assertIn(skill, merged, f"skill {skill} missing from the merged data")
            for layer in entry["layers"]:
                paths = ([layer["file"]] if layer.get("file") else []) + list(layer.get("files") or []) + list(layer.get("frames") or [])
                for path in paths:
                    key = path.replace("/", "\\").lower().rsplit(".", 1)[0]
                    self.assertIn(key, merged[skill], f"skill {skill}: overlay asset {path} missing from the merged data")

    def test_overlay_is_up_to_date(self):
        result = subprocess.run([sys.executable, str(GENERATOR), "--check"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr or result.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
