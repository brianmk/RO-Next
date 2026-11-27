#!/usr/bin/env python3
"""Invariant tests for the generated skill behaviour / procedural effect data.

Read-only checks over the committed artefacts of the Phase 4 migration:

* ``tools/data/skills_custom.ron`` — the single hand-written input,
* ``ro-next/archive/data/client/skills.ron`` — the generated runtime file
  consumed by ``ro-next/src/world/skills.rs``,
* ``tools/gen_skills.py`` — the generator + ``--check`` drift guard.

No game files, no server tree and no network are needed.

Run:
    ./tools/test_skills_data.py
"""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CUSTOM = REPO / "tools/data/skills_custom.ron"
RON = REPO / "ro-next/archive/data/client/skills.ron"
GENERATOR = REPO / "tools/gen_skills.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SkillBehaviourDataTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gen = load_module(GENERATOR, "gen_skills")
        cls.data = cls.gen.parse_custom(CUSTOM)
        cls.ron_text = RON.read_text(encoding="utf-8")

    # --- committed file -----------------------------------------------------

    def test_generated_file_matches_the_generator(self) -> None:
        self.assertEqual(self.ron_text, self.gen.render(self.data))

    def test_behaviour_flags_are_exactly_the_previous_hardcoded_behaviour(self) -> None:
        behaviours = {row["skill"]: row for row in self.data["behaviours"]}
        self.assertEqual(sorted(behaviours), [278, 2036])
        self.assertEqual(behaviours[2036].get("channeling"), True, "Rolling Cutter channeling")
        self.assertEqual(behaviours[278].get("move_while_casting"), True, "Free Cast move-while-casting")
        # Warp Portal (27) is no longer a behaviour row: its portal is the
        # skill unit's official EF_PORTAL2/EF_READYPORTAL2.
        self.assertNotIn(27, behaviours)
        # No other flag is enabled on those rows (behaviour must not change).
        for skill, row in behaviours.items():
            for flag in ("channeling", "move_while_casting"):
                expected = flag in {
                    2036: {"channeling"},
                    278: {"move_while_casting"},
                }[skill]
                self.assertEqual(bool(row.get(flag, False)), expected, f"skill {skill} {flag}")

    def test_storm_gust_cloud_is_data(self) -> None:
        self.assertEqual(len(self.data["clouds"]), 1)
        cloud = self.data["clouds"][0]
        self.assertEqual(cloud["skills"], [87, 89], "Storm Gust under both client/packet ids")
        self.assertEqual(cloud["phase"], "Success")
        self.assertEqual(
            cloud["textures"],
            [f"effect\\cloud{index}.tga" for index in range(1, 8)],
            "official cloud1..7.tga textures",
        )
        # The previous hard-coded constants of storm_cloud.rs.
        self.assertEqual(cloud["count"], 18)
        self.assertEqual(cloud["radius"], 22.0)
        self.assertEqual(cloud["base_height"], 10.0)
        self.assertEqual(cloud["height_variation"], 8.0)
        self.assertEqual(cloud["size_min"], 16.0)
        self.assertEqual(cloud["size_variation"], 16.0)
        self.assertEqual(cloud["spin_min"], 0.15)
        self.assertEqual(cloud["spin_variation"], 0.35)
        self.assertEqual(cloud["alpha"], 0.55)
        self.assertEqual(cloud["color"], (0.45, 0.48, 0.58))
        self.assertEqual(cloud["duration_ms"], 3000)
        self.assertEqual(cloud["fade_in_ms"], 250)
        self.assertEqual(cloud["fade_out_ms"], 700)

    def test_procedural_effect_assets_are_the_previous_constants(self) -> None:
        circle = self.data["cast_circle"]
        self.assertEqual(circle["ring_texture"], "effect\\ring_liner.tga")
        self.assertEqual(circle["glow_texture"], "effect\\alpha_down.tga")
        self.assertEqual(circle["radius"], 7.0)
        self.assertEqual(circle["height"], 0.2)
        self.assertEqual(circle["ring_rotation_speed"], 1.6)
        self.assertEqual(circle["inner_ring_rotation_speed"], -2.3)
        self.assertEqual(circle["inner_ring_scale"], 0.62)
        self.assertEqual(circle["glow_scale"], 1.15)

        portal = self.data["warp_portal"]
        self.assertEqual(portal["streak_texture"], "effect\\ring_blue.tga")
        self.assertEqual(portal["gradient_texture"], "effect\\alpha_down.tga")
        self.assertEqual(portal["particle_sprite"], "이팩트\\particle1.spr")

        aura = self.data["level_aura"]
        self.assertEqual(aura["ring_liner_texture"], "effect\\ring_liner.tga")
        self.assertEqual(aura["ring_blue_texture"], "effect\\ring_blue.tga")
        self.assertEqual(aura["ring_red_texture"], "effect\\ring_red.tga")
        self.assertEqual(aura["bubble_texture"], "effect\\whitelight.tga")
        self.assertEqual(aura["sparkle_texture"], "effect\\pikapika2.bmp")
        self.assertEqual(
            aura["glow_textures"],
            [f"effect\\glow0{index}.bmp" for index in range(1, 5)],
        )

    def test_ron_has_all_schema_fields(self) -> None:
        for key in (
            "provenance: ",
            "version: 1,",
            "behaviours: [",
            "clouds: [",
            "cast_circle: (",
            "warp_portal: (",
            "level_aura: (",
        ):
            self.assertIn(key, self.ron_text, key)

    # --- validation ---------------------------------------------------------

    def test_generator_rejects_unknown_fields(self) -> None:
        tampered = CUSTOM.read_text(encoding="utf-8").replace(
            "        radius: 7.0,",
            "        radius: 7.0,\n        bogus: 1.0,",
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "skills_custom.ron"
            path.write_text(tampered, encoding="utf-8")
            with self.assertRaises(SystemExit) as context:
                self.gen.parse_custom(path)
        self.assertIn("cast_circle fields", str(context.exception))

    def test_generator_rejects_disabled_behaviour_rows(self) -> None:
        tampered = CUSTOM.read_text(encoding="utf-8").replace(
            "        (skill: 2036, channeling: true),",
            "        (skill: 2036, channeling: false),",
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "skills_custom.ron"
            path.write_text(tampered, encoding="utf-8")
            with self.assertRaises(SystemExit) as context:
                self.gen.parse_custom(path)
        self.assertIn("no enabled flag", str(context.exception))

    # --- generator CLI ------------------------------------------------------

    def test_generator_check_reports_the_committed_file_as_current(self) -> None:
        result = subprocess.run([sys.executable, str(GENERATOR), "--check"], cwd=REPO, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("up to date", result.stdout)

    def test_generator_check_detects_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "skills.ron"
            output.write_text(self.ron_text + "\n", encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(GENERATOR), "--check", "--output", str(output)],
                cwd=REPO,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.returncode, 1)
        self.assertIn("out of date", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
