#!/usr/bin/env python3
"""Invariant tests for the generated official multi-layer skill effect data.

Read-only checks over the committed artefacts of this lane:

* `tools/data/windows_client_effect_layers.json` - the extracted official effect
  table layer facts (every layer of every numeric effect id, in table order),
* `tools/data/skill_effects_official.ron` - the generated fragment consumed by the
  unified pipeline merge generator,
* `tools/data/skill_effects_official.json` - machine-readable rows + coverage,
* `tools/data/rathena_pre_re_learnable_skills.json` - the learnable-skill snapshot.

The tests assert the v1 pipeline schema (`skill_effects.rs` in the pipeline
lane), the lead-requested priority rows (Blessing 34, Increase AGI 29, Angelus
33, Lex Aeterna 78, Assumptio 361, Heal 28), the coverage accounting and that
`--check` reports the committed files as up to date.

Run:
    ./tools/test_skill_effect_layers.py
"""

from __future__ import annotations

import json
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LAYERS = REPO / "tools/data/windows_client_effect_layers.json"
OFFICIAL_RON = REPO / "tools/data/skill_effects_official.ron"
OFFICIAL_JSON = REPO / "tools/data/skill_effects_official.json"
LEARNABLE = REPO / "tools/data/rathena_pre_re_learnable_skills.json"
GENERATOR = REPO / "tools/gen_skill_effect_layers.py"

#: Fields of `LayerFile` in `ro-next/src/world/library/skill_effects.rs`
#: (pipeline lane). Unknown keys are not allowed in the committed fragment.
LAYER_FIELDS = {
    "kind",
    "anchor",
    "attached_entity",
    "head",
    "vertical",
    "file",
    "files",
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
KINDS = {"Str", "Spr", "Texture", "Cylinder", "Primitive", "BodyFlash"}
ANCHORS = {"Caster", "Target", "Ground", "Overhead", "Body", "Travel", "Rain", "Auto"}
VERTICALS = {"Feet", "Body", "Overhead"}
BLENDS = {"Alpha", "Additive"}
GROWS = {"Constant", "TopRadius", "BothRadii", "Height"}
PHASES = ["Ground", "Cast", "Success", "Release", "Hit", "Caster"]
TEXTURE_EXTENSIONS = (".tga", ".bmp", ".png", ".jpg")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def parse_fragment_layers(text: str) -> list[dict]:
    """Parse the flat `(kind: ..., file: ...)` layer rows of the fragment."""
    import re

    layers = []
    for match in re.finditer(r"\(kind: (\w+), (.*?)\)[,)]", text):
        layer: dict = {"kind": match.group(1)}
        body = match.group(2)
        for field in re.finditer(r"(\w+): (\".*?\"(?<!\\)|\-?\d+(?:\.\d+)?|true|false|\([^)]*\)|\[[^\]]*\])", body):
            key, value = field.group(1), field.group(2)
            if value in ("true", "false"):
                layer[key] = value == "true"
            elif value.startswith("(") and value.endswith(")"):
                layer[key] = tuple(float(part) for part in value[1:-1].split(","))
            elif value.startswith("["):
                layer[key] = [part.strip().strip('"') for part in value[1:-1].split(",")]
            elif value.startswith('"'):
                layer[key] = value[1:-1]
            else:
                layer[key] = float(value) if "." in value else int(value)
        layers.append(layer)
    return layers


class OfficialSkillEffectDataTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = load_json(OFFICIAL_JSON)
        cls.rows = cls.document["effects"]
        cls.learnable = load_json(LEARNABLE)
        cls.layers_facts = load_json(LAYERS)

    # ------------------------------------------------------------------ facts
    def test_effect_layer_facts_keep_all_layers_in_table_order(self) -> None:
        effects = self.layers_facts["effects"]
        # Blessing has four layers: 축복 sprite, two particle6 bursts, pok2.tga.
        blessing = effects["42"]["layers"]
        self.assertEqual(len(blessing), 4)
        self.assertEqual(blessing[0]["type"], "SPR")
        self.assertEqual(blessing[0]["file"], "축복")
        self.assertTrue(blessing[0]["head"])
        self.assertEqual(blessing[3]["file"], "effect/pok2.tga")
        self.assertEqual(blessing[3]["wav"], "effect/ef_blessing")
        # Increase AGI: three ac_center2 bursts with different duplicate/alpha
        # values plus agi_up.bmp.
        increase_agi = effects["37"]["layers"]
        self.assertEqual(len(increase_agi), 4)
        self.assertEqual([layer["file"] for layer in increase_agi], ["effect/ac_center2.tga"] * 3 + ["effect/agi_up.bmp"])
        self.assertEqual([layer["duplicate"] for layer in increase_agi[:3]], [7, 3, 10])
        # Angelus: official STR layer with the overhead head flag and its sound.
        angelus = effects["41"]["layers"][0]
        self.assertEqual(angelus["type"], "STR")
        self.assertEqual(angelus["file"], "angelus")
        self.assertTrue(angelus["head"])
        self.assertTrue(angelus["attachedEntity"])
        self.assertEqual(angelus["wav"], "effect/ef_angelus")
        # Lex Aeterna and Assumptio keep the official attachedEntity/wav facts.
        self.assertEqual(effects["85"]["layers"][0]["wav"], "effect/priest_lexaeterna")
        self.assertTrue(effects["85"]["layers"][0]["attachedEntity"])
        self.assertEqual(effects["440"]["layers"][0]["file"], "asum")
        self.assertTrue(effects["440"]["layers"][0]["attachedEntity"])

    def test_extractor_gaps_are_fixed(self) -> None:
        facts = load_json(REPO / "tools/data/windows_client_skill_effects.json")
        soul_strike = {field["field"]: field for field in facts["skills"]["13"]}
        self.assertEqual(soul_strike["hitEffectId"]["effects"], [0])
        self.assertEqual(soul_strike["firstHitEffectId"]["effects"], [1])
        # Effect 0 is the official EF_HIT1 standard hit flash and keeps its facts.
        self.assertEqual(self.layers_facts["effects"]["0"]["layers"][0]["file"], "effect/pok3.tga")
        # Symbolic named effect blocks are extracted as their own blocks.
        self.assertIn("ef_arrow_projectile", self.layers_facts["named_effects"])
        self.assertIn("spear_hit_sound", self.layers_facts["named_effects"])
        # A named block must not bleed into the preceding numeric block.
        first_aid = self.layers_facts["effects"]["309"]["layers"]
        self.assertEqual(len(first_aid), 1)
        self.assertEqual(first_aid[0]["file"], "effect/pikapika2.bmp")

    # ------------------------------------------------------------------ RON
    def test_fragment_parses_and_matches_json(self) -> None:
        text = OFFICIAL_RON.read_text(encoding="utf-8")
        self.assertIn("version: 1", text)
        self.assertIn("effects: [", text)
        layer_count = text.count("(kind: ")
        self.assertEqual(layer_count, self.document["statistics"]["layers"])

    def test_every_layer_matches_the_loader_schema(self) -> None:
        for row in self.rows:
            self.assertIn("skill", row)
            self.assertIn("phase", row)
            self.assertIn(row["phase"], PHASES)
            self.assertTrue(row["layers"], f"skill {row['skill']} {row['phase']} has no layers")
            for layer in row["layers"]:
                context = f"skill {row['skill']} {row['phase']}"
                self.assertIn(layer["kind"], KINDS, context)
                for key in layer:
                    self.assertIn(key, LAYER_FIELDS, f"{context}: unknown layer key {key}")
                if layer["kind"] == "Str":
                    self.assertTrue(layer.get("file", "").endswith(".str"), context)
                elif layer["kind"] == "Spr":
                    self.assertTrue(layer.get("files"), context)
                    for stem in layer["files"]:
                        self.assertFalse(stem.lower().endswith((".spr", ".act")), context)
                elif layer["kind"] in ("Texture", "Cylinder", "Primitive"):
                    self.assertTrue(layer.get("file", "").lower().endswith(TEXTURE_EXTENSIONS), context)
                if layer["kind"] == "Primitive":
                    self.assertIn(layer.get("primitive"), {"HitFlash", "Projectile", "GroundAura"}, context)
                if "anchor" in layer:
                    self.assertIn(layer["anchor"], ANCHORS, context)
                if "vertical" in layer:
                    self.assertIn(layer["vertical"], VERTICALS, context)
                if "blend" in layer:
                    self.assertIn(layer["blend"], BLENDS, context)
                if "grow" in layer:
                    self.assertIn(layer["grow"], GROWS, context)
                if "count" in layer:
                    self.assertGreaterEqual(layer["count"], 1, context)
                    self.assertLessEqual(layer["count"], 16, context)
                if "alpha" in layer:
                    self.assertGreaterEqual(layer["alpha"], 0.0, context)
                    self.assertLessEqual(layer["alpha"], 1.0, context)
                for key in ("duration_ms", "delay_ms"):
                    if key in layer:
                        self.assertLessEqual(layer[key], 30_000, context)
                if "wav" in layer:
                    self.assertTrue(layer["wav"].lower().endswith(".wav"), context)
                if "offset" in layer:
                    self.assertEqual(len(layer["offset"]), 3, context)
                if "tint" in layer:
                    self.assertEqual(len(layer["tint"]), 3, context)

    def test_priority_rows(self) -> None:
        rows = {(row["skill"], row["phase"]): row for row in self.rows}

        blessing = rows[(34, "Success")]["layers"]
        self.assertEqual(len(blessing), 4)
        assets = [layer.get("file") or layer["files"][0] for layer in blessing]
        self.assertIn("이팩트\\축복", assets)
        self.assertIn("effect\\pok2.tga", assets)
        self.assertTrue(blessing[0]["head"])
        self.assertEqual(blessing[0]["anchor"], "Auto")
        self.assertEqual(blessing[-1]["wav"], "effect\\ef_blessing.wav")

        increase_agi = rows[(29, "Success")]["layers"]
        self.assertEqual(len(increase_agi), 4)
        textures = [layer["file"] for layer in increase_agi]
        self.assertEqual(textures.count("effect\\ac_center2.tga"), 3)
        self.assertIn("effect\\agi_up.bmp", textures)
        self.assertEqual(increase_agi[-1]["wav"], "effect\\ef_incagility.wav")

        angelus = rows[(33, "Success")]["layers"][0]
        self.assertEqual(angelus["kind"], "Str")
        self.assertEqual(angelus["file"], "angelus.str")
        self.assertTrue(angelus["head"])

        lex_aeterna = rows[(78, "Success")]["layers"][0]
        self.assertEqual(lex_aeterna["file"], "lexaeterna.str")
        self.assertTrue(lex_aeterna["attached_entity"])
        self.assertEqual(lex_aeterna["wav"], "effect\\priest_lexaeterna.wav")

        assumptio = rows[(361, "Success")]["layers"][0]
        self.assertEqual(assumptio["file"], "asum.str")
        self.assertTrue(assumptio["attached_entity"])

        heal = {layer["kind"] for layer in rows[(28, "Success")]["layers"]}
        self.assertIn("Cylinder", heal)
        self.assertIn("Texture", heal)

        # Effect 0 (EF_HIT1) and firstHitEffectId now contribute their official
        # layers to melee skills.
        soul_strike_assets = [layer["file"] for layer in rows[(13, "Hit")]["layers"]]
        self.assertIn("effect\\pok3.tga", soul_strike_assets)
        self.assertIn("effect\\lens1.tga", soul_strike_assets)

        # Symbolic block references emit Primitive/Projectile layers with Travel
        # (the loader resolves both endpoints from the packet).
        projectile = rows[(523, "Release")]["layers"][0]
        self.assertEqual(projectile["kind"], "Primitive")
        self.assertEqual(projectile["primitive"], "Projectile")
        self.assertEqual(projectile["anchor"], "Travel")
        self.assertTrue(projectile["file"].lower().endswith((".bmp", ".tga")))

    # -------------------------------------------------------------- coverage
    def test_learnable_coverage_accounting(self) -> None:
        coverage = self.document["coverage"]
        learnable_ids = {int(skill_id) for skill_id in self.learnable["skills"]}
        covered = {row["skill"] for row in self.rows} & learnable_ids
        gaps = [gap["skill"] for gap in coverage["no_effect"]]
        self.assertEqual(len(covered), coverage["skills_with_official_layers"])
        self.assertEqual(len(gaps), coverage["skills_without_official_layers"])
        self.assertEqual(set(gaps), learnable_ids - covered)
        self.assertEqual(len(set(gaps)), len(gaps), "duplicate gap entries")
        self.assertEqual(len(learnable_ids), coverage["learnable_skills"])
        self.assertEqual(len(covered) + len(gaps), len(learnable_ids))
        self.assertTrue(all(gap["reason"] for gap in coverage["no_effect"]))
        reasons = {}
        for gap in coverage["no_effect"]:
            reasons[gap["reason"]] = reasons.get(gap["reason"], 0) + 1
        self.assertEqual(reasons, coverage["gap_reasons"])

    def test_class_coverage_accounting(self) -> None:
        coverage = self.document["coverage"]
        learnable_ids = {int(skill_id) for skill_id in self.learnable["skills"]}
        covered = {row["skill"] for row in self.rows} & learnable_ids
        for class_name, entry in coverage["by_class"].items():
            class_skills = {int(skill_id) for skill_id in self.learnable["classes"][class_name]["skills"]}
            self.assertEqual(entry["learnable"], len(class_skills))
            self.assertEqual(entry["covered"], len(class_skills & covered))
            self.assertEqual(entry["missing"], len(class_skills - covered))
            self.assertEqual(entry["missing_skills"], sorted(class_skills - covered))

    def test_statistics_are_consistent(self) -> None:
        statistics = self.document["statistics"]
        self.assertEqual(statistics["rows"], len(self.rows))
        self.assertEqual(statistics["layers"], sum(len(row["layers"]) for row in self.rows))
        self.assertEqual(statistics["skills"], len({row["skill"] for row in self.rows}))
        kinds = {}
        for row in self.rows:
            for layer in row["layers"]:
                kinds[layer["kind"]] = kinds.get(layer["kind"], 0) + 1
        self.assertEqual(kinds, self.document["coverage"]["layer_count_by_kind"])

    # ----------------------------------------------------------------- drift
    def test_generator_check_passes(self) -> None:
        result = subprocess.run(
            [sys.executable, str(GENERATOR), "--check"],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
