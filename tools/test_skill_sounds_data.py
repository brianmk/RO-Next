#!/usr/bin/env python3
"""Invariant tests for the merged skill-sound data file.

The curated skill sounds used to be a Rust constant (`SKILL_SOUNDS` in
`korangar/src/world/skill_sound.rs`). They now live in
``tools/skill_sounds_custom.ron`` and are merged by
``tools/gen_skill_sounds.py`` into the ``curated``/``generic`` sections of
``korangar/archive/data/client/skill_sounds.ron`` (together with the official
per-skill ``sounds`` table).

These tests are the offline CI guard for that merge: the full generator
``--check`` needs the CondaRO GRF archives (it resolves official wav names
against them), but the curated merge can be verified from the committed files
alone.

Run:
    ./tools/test_skill_sounds_data.py
"""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CUSTOM = REPO / "tools/skill_sounds_custom.ron"
RON = REPO / "korangar/archive/data/client/skill_sounds.ron"
GENERATOR = REPO / "tools/gen_skill_sounds.py"
EFFECT_GENERATOR = REPO / "tools/gen_skill_effects.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SkillSoundDataTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.gen = load_module(GENERATOR, "gen_skill_sounds")
        cls.ron = load_module(EFFECT_GENERATOR, "gen_skill_effects")
        cls.custom_curated, cls.custom_generic, cls.custom_provenance = cls.gen.load_custom(CUSTOM)
        cls.committed = cls.ron.read_ron(RON)

    def test_committed_file_has_the_three_sections(self) -> None:
        for section in ("provenance", "generic", "curated", "sounds"):
            self.assertIn(section, self.committed, section)
        self.assertIsInstance(self.committed["generic"], list)
        self.assertIsInstance(self.committed["curated"], list)
        self.assertIsInstance(self.committed["sounds"], list)

    def test_curated_section_matches_the_custom_input(self) -> None:
        curated = [(int(row["skill"]), list(row["files"])) for row in self.committed["curated"]]
        self.assertEqual(curated, self.custom_curated, "the curated rows are merged verbatim and sorted")

    def test_generic_sound_matches_the_custom_input(self) -> None:
        self.assertEqual(list(self.committed["generic"]), self.custom_generic)

    def test_curated_skills_are_not_duplicated_in_the_generated_table(self) -> None:
        curated_skills = {skill for skill, _ in self.custom_curated}
        generated_skills = {int(row["skill"]) for row in self.committed["sounds"]}
        self.assertEqual(curated_skills & generated_skills, set())

    def test_generated_table_is_broad_and_sorted(self) -> None:
        skills = [int(row["skill"]) for row in self.committed["sounds"]]
        self.assertGreaterEqual(len(skills), 300)
        self.assertEqual(skills, sorted(skills))
        self.assertEqual(len(skills), len(set(skills)))

    def test_every_path_is_a_relative_wav(self) -> None:
        tables = [list(self.committed["generic"])]
        tables.extend(list(row["files"]) for row in self.committed["curated"])
        tables.extend(list(row["files"]) for row in self.committed["sounds"])
        for files in tables:
            self.assertTrue(files)
            for file in files:
                self.assertTrue(file.lower().endswith(".wav"), file)
                self.assertFalse(file.lower().startswith("data\\wav\\"), file)

    def test_curated_rows_are_exactly_the_previous_rust_table(self) -> None:
        # The six skills and their candidate lists of the deleted SKILL_SOUNDS
        # constant, in play-priority order.
        self.assertEqual(
            self.custom_curated,
            [
                (27, ["effect\\ef_readyportal.wav", "effect\\ef_portal.wav"]),
                (29, ["effect\\ef_incagility.wav"]),
                (33, ["effect\\ef_angelus.wav"]),
                (34, ["effect\\ef_blessing.wav", "effect\\spirit_handler\\sh_blessing_of_mystical_creatures.wav"]),
                (45, ["effect\\ac_concentration.wav"]),
                (110, ["effect\\black_hammerfall.wav"]),
            ],
        )
        # The deleted GENERIC_CAST_SOUND constant.
        self.assertEqual(self.custom_generic, ["effect\\ef_beginspell.wav", "effect\\em_spell_enchanting.wav"])

    def test_custom_loader_rejects_bad_rows(self) -> None:
        tampered = CUSTOM.read_text(encoding="utf-8").replace(
            "        (skill: 110, files: [\"effect\\\\black_hammerfall.wav\"]),",
            "        (skill: 110, files: [\"effect\\\\black_hammerfall.wav\"]),\n"
            "        (skill: 110, files: [\"effect\\\\black_hammerfall.wav\"]),",
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "skill_sounds_custom.ron"
            path.write_text(tampered, encoding="utf-8")
            with self.assertRaises(SystemExit) as context:
                self.gen.load_custom(path)
        self.assertIn("duplicate curated sound", str(context.exception))

    def test_custom_loader_rejects_non_wav_files(self) -> None:
        tampered = CUSTOM.read_text(encoding="utf-8").replace(
            "effect\\\\ef_incagility.wav",
            "effect\\\\ef_incagility.tga",
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "skill_sounds_custom.ron"
            path.write_text(tampered, encoding="utf-8")
            with self.assertRaises(SystemExit) as context:
                self.gen.load_custom(path)
        self.assertIn("not a .wav path", str(context.exception))


if __name__ == "__main__":
    unittest.main(verbosity=2)
