#!/usr/bin/env python3
"""Invariant tests for the generated job/appearance registry data.

Read-only checks over the committed artefacts of the P1 appearance migration:

* ``tools/job_facts.json`` — the extracted facts (rAthena ``enum e_job``/skill
  trees, the client archive appearance probe, the verified sprite fallbacks),
* ``korangar/archive/data/client/jobs.ron`` — the generated data file consumed
  by ``korangar/src/world/jobs.rs``,
* ``tools/gen_jobs.py`` — the generator + ``--check`` drift guard.

The tests assert that the facts are complete, that every job resolves to a
sprite/palette that the archive probe saw, that the classification flags match
the rAthena job model, and that the generated file is up to date. No game files
or server tree are needed.

Run:
    ./tools/test_gen_jobs.py
"""

from __future__ import annotations

import importlib.util
import json
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools/job_facts.json"
RON = REPO / "korangar/archive/data/client/jobs.ron"
GENERATOR = REPO / "tools/gen_jobs.py"


def load_module(path: Path, name: str):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def body_stems(facts: dict, sex: str) -> set[str]:
    suffix = f"_{sex}"
    return {stem[: -len(suffix)] for stem in facts["client"]["body_sprites"][sex] if stem.endswith(suffix)}


def parse_ron_jobs(text: str) -> list[dict]:
    """Parse the job rows of the generated RON file (the file is machine generated)."""
    rows = []
    for block in re.findall(r"        \(\n(.*?)\n        \),", text, re.S):
        row: dict = {}
        for line in block.splitlines():
            line = line.strip()
            match = re.match(r'^(\w+): (.*),$', line)
            if match is None:
                continue
            key, value = match.groups()
            if value.startswith('"'):
                row[key] = json.loads(value)
            elif value.startswith("Some("):
                row[key] = json.loads(value[len("Some(") : -1])
            elif value == "None":
                row[key] = None
            elif value in ("true", "false"):
                row[key] = value == "true"
            else:
                try:
                    row[key] = int(value)
                except ValueError:
                    try:
                        row[key] = float(value)
                    except ValueError:
                        row[key] = value
        if row:
            rows.append(row)
    return rows


class JobFactsTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.facts = json.loads(FACTS.read_text())
        cls.server = cls.facts["server"]
        cls.client = cls.facts["client"]
        cls.jobs = cls.server["jobs"]
        cls.by_id = {job["id"]: job for job in cls.jobs}
        cls.gen = load_module(GENERATOR, "gen_jobs")
        cls.data = cls.gen.build(cls.facts)
        cls.ron_text = RON.read_text()
        cls.ron_jobs = parse_ron_jobs(cls.ron_text)
        cls.body = {"남": body_stems(cls.facts, "남"), "여": body_stems(cls.facts, "여")}
        cls.palettes = {sex: set(stems) for sex, stems in cls.client["body_palettes"].items()}

    # --- facts -------------------------------------------------------------

    def test_facts_provenance_is_complete(self) -> None:
        provenance = self.facts["provenance"]
        server = provenance["server"]
        self.assertRegex(server["head"], r"^[0-9a-f]{40}$", "rAthena server HEAD")
        for path in ("src/common/mmo.hpp", "db/pre-re/skill_tree.yml", "db/re/skill_tree.yml", "src/map/pc.hpp"):
            self.assertIn(path, server["files"])
            self.assertEqual(len(server["files"][path]["sha256"]), 64, path)
        archives = provenance["client"]["archives"]
        self.assertEqual([archive["name"] for archive in archives], ["data.grf", "CondaRO.grf", "palettes.grf"])
        for archive in archives:
            self.assertEqual(len(archive["sha256"]), 64, archive["name"])
            self.assertGreater(archive["files"], 10000, archive["name"])
        self.assertIn("extract_job_facts.py", provenance["extractor"])

    def test_every_server_job_is_present_and_sorted(self) -> None:
        ids = [job["id"] for job in self.jobs]
        self.assertEqual(ids, sorted(ids), "job ids must be sorted")
        self.assertEqual(len(ids), len(set(ids)), "job ids must be unique")
        self.assertNotIn(31, ids, "JOB_MAX_BASIC is a sentinel")
        self.assertNotIn(4317, ids, "JOB_MAX is a sentinel")
        self.assertEqual(len(ids), 181, "all real rAthena e_job ids")
        for job in self.jobs:
            self.assertEqual(job["entity_type"], "Player", job["name"])
            self.assertTrue(job["name"].startswith("JOB_"), job["name"])

    def test_skill_trees_are_extracted(self) -> None:
        pre_re = self.server["skill_tree_pre_re"]
        re = self.server["skill_tree_re"]
        self.assertGreater(len(pre_re), 70)
        self.assertGreater(len(re), len(pre_re), "the renewal tree file covers all jobs")
        # The 4th classes only exist in the renewal tree of this server revision.
        self.assertIn("Dragon_Knight", re)
        self.assertIn("Sky_Emperor", re)
        self.assertEqual(re["Dragon_Knight"]["own_skills"] > 0, True)
        self.assertIn("Novice", pre_re["Swordman"]["inherits"])

    def test_newer_class_sprites_are_used_where_the_data_has_them(self) -> None:
        # The CondaRO archives ship the 4th/expanded class body sprites (lower
        # case file names); the data must use them instead of a fallback.
        for job_id in (4211, 4212, 4215, 4252, 4256, 4263, 4264, 4302, 4307):
            job = self.by_id[job_id]
            self.assertEqual(job["sprite_source"], "Mapped", job["name"])
            stem = job["sprite"].lower()
            self.assertTrue(stem in self.body["남"] or stem in self.body["여"], job["name"])
        # The only mapped job whose client sprite is really missing keeps its fallback.
        self.assertEqual(self.by_id[4014]["sprite_source"], "Fallback")
        self.assertEqual(self.by_id[4014]["client_sprite"], "엔대운")
        self.assertEqual(self.by_id[4014]["sprite"], "로드나이트")

    def test_headgear_missing_list_is_documented(self) -> None:
        headgear = self.client["headgear"]
        self.assertEqual(headgear["view_ids"], 1526, "client accessory view ids")
        self.assertEqual(len(headgear["missing"]), 161, "accessory sprites missing from this data set")
        self.assertEqual(headgear["with_sprite"] + len(headgear["missing"]), headgear["view_ids"])
        missing_text = "        missing: [" + ", ".join(str(view_id) for view_id in headgear["missing"]) + "],"
        self.assertIn(missing_text, self.ron_text, "the missing headgear list has to be in the data file")

    def test_classification_matches_the_rAthena_job_model(self) -> None:
        # Transcendent rebirth classes: 4001..=4022 (not the baby novice 4023).
        for job_id in range(4001, 4023):
            if job_id == 4014 or job_id == 4022:
                continue
            self.assertTrue(self.by_id[job_id]["transcendent"], f"{job_id} should be transcendent")
        self.assertFalse(self.by_id[4023]["transcendent"], "JOB_BABY is not a transcendent job")
        self.assertTrue(self.by_id[4023]["baby"])
        self.assertFalse(self.by_id[0]["transcendent"])
        self.assertFalse(self.by_id[7]["transcendent"])
        # The _T / _T2 third-job variants are the transcendent versions.
        self.assertTrue(self.by_id[4060]["transcendent"], "JOB_RUNE_KNIGHT_T")
        self.assertTrue(self.by_id[4081]["transcendent"], "JOB_RUNE_KNIGHT_T2")
        # Baby jobs scale down; everything else is full size.
        for job in self.jobs:
            self.assertEqual(job["scale"], 0.75 if job["baby"] else 1.0, job["name"])
        # 4th classes.
        for job_id in list(range(4252, 4265)) + list(range(4278, 4282)) + list(range(4302, 4309)) + [4316]:
            self.assertTrue(self.by_id[job_id]["fourth"], f"{job_id} should be a 4th class")
        self.assertFalse(self.by_id[4054]["fourth"], "Rune Knight is a 3rd class")
        # Expanded classes.
        for job_id in (23, 24, 25, 4046, 4047, 4049, 4211, 4212, 4215, 4218, 4239, 4240):
            self.assertTrue(self.by_id[job_id]["expanded"], f"{job_id} should be an expanded class")
        self.assertFalse(self.by_id[7]["expanded"], "Knight is not expanded")
        self.assertFalse(self.by_id[4054]["expanded"], "Rune Knight is not expanded")

    def test_every_job_resolves_to_a_sprite_that_exists(self) -> None:
        for job in self.jobs:
            sprite = job["sprite"]
            self.assertTrue(sprite, job["name"])
            self.assertNotIn("missing", sprite.lower(), job["name"])
            stem = sprite.lower()
            self.assertTrue(stem in self.body["남"] or stem in self.body["여"], f"{job['name']}: {sprite}")
        for job_id, stem in self.client["sprite_fallbacks"].items():
            stem = stem.lower()
            self.assertTrue(stem in self.body["남"] or stem in self.body["여"], f"fallback of {job_id}")

    def test_fallback_rows_are_documented(self) -> None:
        mapped = [job for job in self.jobs if job["sprite_source"] == "Mapped"]
        fallback = [job for job in self.jobs if job["sprite_source"] == "Fallback"]
        unmapped = [job for job in self.jobs if job["sprite_source"] == "Unmapped"]
        self.assertEqual(len(mapped), 143)
        self.assertEqual(len(fallback), 1)
        self.assertEqual(len(unmapped), 37)
        for job in mapped:
            self.assertEqual(job["client_sprite"], job["sprite"], job["name"])
        for job in fallback:
            intended = job["client_sprite"]
            self.assertIsNotNone(intended, job["name"])
            self.assertFalse(
                intended.lower() in self.body["남"] or intended.lower() in self.body["여"],
                f"{job['name']}: fallback recorded but client sprite {intended} exists",
            )
            self.assertNotEqual(job["sprite"], intended, job["name"])
        for job in unmapped:
            self.assertIsNone(job["client_sprite"], job["name"])

    def test_every_palette_in_the_data_exists(self) -> None:
        embedded = 0
        for job in self.jobs:
            palette = job["palette"]
            if palette is None:
                embedded += 1
                continue
            stem = palette.lower()
            self.assertTrue(stem in self.palettes["남"] or stem in self.palettes["여"], job["name"])
        self.assertEqual(embedded, self.client["summary"]["embedded_palettes"])
        self.assertEqual(embedded, 16)
        # The four classic palette-stem overrides are preserved in the facts.
        overrides = self.client["palette_overrides"]
        self.assertEqual(overrides["하이프리"], "하이프리스트")
        self.assertEqual(overrides["어쌔신크로스"], "어세신크로스")
        self.assertEqual(overrides["가드"], "로얄가드")
        self.assertEqual(overrides["REBELLION"], "리벨리온")

    def test_appearance_defaults_and_hair_styles_exist(self) -> None:
        appearance = self.client["appearance"]
        self.assertEqual(len(appearance["hair_styles"]["male"]), 13)
        self.assertEqual(len(appearance["hair_styles"]["female"]), 13)
        for sex, key in (("남", "male"), ("여", "female")):
            for head in appearance["hair_styles"][key]:
                self.assertIn(head, set(self.client["head_sprites"][sex]), f"{key} head {head}")
        self.assertEqual(appearance["hair_style_range"], [0, 12])
        self.assertEqual(appearance["default_head"], 1)
        self.assertIn(1, self.client["head_sprites"]["남"])
        self.assertIn(1, self.client["head_sprites"]["여"])
        start, end = appearance["head_range"]
        for sex in ("남", "여"):
            for head in range(start, end + 1):
                self.assertIn(head, set(self.client["head_sprites"][sex]))
        # Hair palettes exist for every mapped hair style in both sexes.
        for sex in ("남", "여"):
            hair_palettes = set(self.client["head_palettes"][sex])
            for head in set(appearance["hair_styles"]["male" if sex == "남" else "female"]):
                self.assertIn(head, hair_palettes, f"{sex} hair palette {head}")

    def test_special_entity_job_ids_come_from_the_client_table(self) -> None:
        special = self.server["special_entity_job_ids"]
        self.assertEqual(special["warp"], [45], "JT_WARPNPC")
        self.assertEqual(special["hidden"], [111, 139], "JT_HIDDEN_NPC/JT_HIDDEN_WARP_NPC")
        jobs = {job["id"] for job in self.jobs}
        self.assertNotIn(45, jobs, "warp NPCs are not player jobs")
        self.assertNotIn(111, jobs)

    # --- generated file -----------------------------------------------------

    def test_generated_file_matches_the_generator(self) -> None:
        self.assertEqual(self.ron_text, self.gen.render(self.data))
        self.assertEqual(len(self.ron_jobs), 181)
        ids = [row["id"] for row in self.ron_jobs]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(ids, [job["id"] for job in self.jobs])

    def test_ron_has_all_schema_fields(self) -> None:
        expected = {
            "id",
            "name",
            "entity_type",
            "sprite",
            "client_sprite",
            "palette",
            "source",
            "baby",
            "transcendent",
            "fourth",
            "expanded",
            "scale",
        }
        for row in self.ron_jobs:
            self.assertEqual(set(row), expected, row.get("name"))
        for key in (
            "provenance: ",
            "version: 1,",
            "paths: (",
            "body: ",
            "head: ",
            "body_palette: ",
            "head_palette: ",
            "headgear: ",
            "sex_suffix: (",
            "appearance: (",
            "hair_styles: (",
            "hair_style_range: ",
            "head_range: ",
            "default_head: ",
            "default_body_palette: ",
            "default_head_palette: ",
            "entity_types: (",
            "warp: ",
            "hidden: ",
            "player_ranges: ",
            "npc_ranges: ",
            "monster_ranges: ",
            "capabilities: (",
            "move_while_casting: ",
            "jobs: [",
        ):
            self.assertIn(key, self.ron_text, key)

    def test_ron_strings_escape_backslashes(self) -> None:
        # RON strings need doubled backslashes; a single one would corrupt the
        # path (and serde would reject the file).
        for line in self.ron_text.splitlines():
            for literal in re.findall(r'"([^"]*)"', line):
                # `json.loads` rejects RON-invalid escapes such as `\m`.
                json.loads(f'"{literal}"')
                # Every backslash of the encoded literal must be part of a
                # doubled pair (a single one would be an invalid escape).
                self.assertEqual(
                    literal.count("\\"),
                    2 * literal.count("\\\\"),
                    f"unescaped backslash in {literal!r}",
                )

    def test_ron_has_no_missing_placeholder_sprite(self) -> None:
        # The documented fallbacks are real sprites; the black `missing.spr`
        # placeholder must never appear as a path or sprite value.
        self.assertNotIn("missing.spr", self.ron_text.lower())
        for row in self.ron_jobs:
            self.assertNotIn("missing", row["sprite"].lower(), row["name"])
            self.assertNotIn("missing", (row["client_sprite"] or "").lower(), row["name"])

    def test_ron_scale_matches_flags(self) -> None:
        for row in self.ron_jobs:
            self.assertEqual(row["scale"], 0.75 if row["baby"] else 1.0, row["name"])
        self.assertEqual(sum(1 for row in self.ron_jobs if row["baby"]), self.client["summary"]["baby"])

    def test_move_while_casting_capability_is_data(self) -> None:
        # The Free Cast (SA_FREECAST, 278) capability of Sage (16) and its
        # rebirth class Professor (4017) is the job half of the old hard-coded
        # `FREE_CAST_JOB_IDS`; the skill half lives in skills.ron.
        self.assertEqual(self.data["capabilities"]["move_while_casting"], [16, 4017])
        self.assertIn("move_while_casting: [16, 4017],", self.ron_text)
        ids = {job["id"] for job in self.jobs}
        for job_id in (16, 4017):
            self.assertIn(job_id, ids, "the capability must reference a real job")

    # --- generator CLI ------------------------------------------------------

    def test_generator_check_reports_the_committed_file_as_current(self) -> None:
        result = subprocess.run(
            [sys.executable, str(GENERATOR), "--check"], cwd=REPO, capture_output=True, text=True
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("up to date", result.stdout)

    def test_generator_check_detects_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "jobs.ron"
            output.write_text(self.ron_text + "\n")
            result = subprocess.run(
                [sys.executable, str(GENERATOR), "--check", "--output", str(output)],
                cwd=REPO,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 1)
            self.assertIn("out of date", result.stderr)

    def test_generator_rejects_inconsistent_facts(self) -> None:
        tampered = json.loads(FACTS.read_text())
        # A fallback that the archive probe did not see must be rejected.
        tampered["client"]["sprite_fallbacks"]["4252"] = "존재하지않는스프라이트"
        with self.assertRaises(SystemExit) as context:
            self.gen.build(tampered)
        self.assertIn("has no body sprite", str(context.exception))

    def test_generator_rejects_unsorted_jobs(self) -> None:
        tampered = json.loads(FACTS.read_text())
        tampered["server"]["jobs"] = list(reversed(tampered["server"]["jobs"]))
        jobs = tampered["server"]["jobs"]
        jobs[0], jobs[1] = jobs[1], jobs[0]
        # The reversed list itself is re-sorted by the generator; a duplicate id
        # must still be rejected.
        tampered["server"]["jobs"] = tampered["server"]["jobs"] + [tampered["server"]["jobs"][0]]
        with self.assertRaises(SystemExit) as context:
            self.gen.build(tampered)
        self.assertIn("not strictly ascending", str(context.exception))


if __name__ == "__main__":
    unittest.main(verbosity=2)
