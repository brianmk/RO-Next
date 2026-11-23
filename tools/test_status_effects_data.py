#!/usr/bin/env python3
"""Invariant tests for the data-driven status-effect data.

Read-only checks over the committed artefacts of this lane:

* `tools/status_facts.json` - the extracted/merged facts document
  (rAthena server sources, official client Lua tables, vendored
  roBrowserLegacy classic table, curated overlay; provenance hashes included),
* `tools/status_effect_custom.ron` - the curated overlay,
* `tools/status_effect_legacy_tables.ron` - the frozen snapshot of the
  hand-generated tables this lane replaced (transition parity oracle),
* `korangar/archive/data/client/status_effects.ron` - the generated client
  data file (drift-checked by `tools/gen_status_effects.py --check`).

Run from the repository root:

    ./tools/test_status_effects_data.py

The extractor (`tools/extract_status_facts.py --check`) needs the game data and
the rAthena tree, so it is exercised manually (see the lane report) instead of
in CI; the generator `--check` and this test only need the committed files.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FACTS = REPO / "tools/status_facts.json"
CUSTOM = REPO / "tools/status_effect_custom.ron"
LEGACY = REPO / "tools/status_effect_legacy_tables.ron"
ROBROWSER = REPO / "tools/status_info_robrowser.json"
GENERATOR = REPO / "tools/gen_status_effects.py"
DATA = REPO / "korangar/archive/data/client/status_effects.ron"

#: Old name ids whose label came from the old generator's off-by-one server
#: enum parse (`XXX_EFST_WORLDSTORE_ACTIVE` skipped). The client `efstids.lub`
#: and the server enum agree on the real ids, so the new table shifted them.
NAME_SHIFT = range(1064, 1073)

CATEGORIES = {"Buff", "Debuff", "Toggle", "System"}
VALUE_FORMATS = {"Element", "Percent", "Number", "Id"}


def load_facts() -> dict:
    return json.loads(FACTS.read_text(encoding="utf-8"))


def pairs(path: Path) -> list[tuple[int, str]]:
    text = path.read_text(encoding="utf-8")
    return [(int(match.group(1)), match.group(2)) for match in re.finditer(r'\((\d+), "((?:[^"\\]|\\.)*)"\)', text)]


class StatusFactsTests(unittest.TestCase):
    def test_facts_have_provenance(self) -> None:
        facts = load_facts()
        self.assertEqual(facts["schema"], 1)
        provenance = facts["provenance"]
        self.assertEqual(provenance["generated_by"], "tools/extract_status_facts.py")
        server = provenance["server"]
        self.assertIn("src/map/status.hpp", server["files"])
        self.assertIn("db/pre-re/status.yml", server["files"])
        self.assertTrue(server["files"]["db/pre-re/status.yml"]["sha256"])
        client = provenance["client"]["lub_files"]
        for key in ("stateiconinfo", "stateiconimginfo", "efstids"):
            self.assertIn(key, client)
            self.assertTrue(client[key]["sha256"], f"{key} has no provenance hash")
        self.assertTrue(provenance["robrowser"]["facts_sha256"])
        self.assertEqual(len(provenance["robrowser"]["commit"]), 40)
        self.assertTrue(provenance["custom"]["sha256"])

    def test_statuses_are_sorted_and_unique(self) -> None:
        facts = load_facts()
        statuses = facts["statuses"]
        self.assertGreater(len(statuses), 500)
        ids = [entry["id"] for entry in statuses]
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(len(ids), len(set(ids)))

    def test_status_rows_are_presentable(self) -> None:
        facts = load_facts()
        for entry in facts["statuses"]:
            status_id = entry["id"]
            name = entry.get("name")
            if name is not None:
                self.assertTrue(name.isascii(), f"status {status_id}: non-ASCII name {name!r}")
                self.assertTrue(any(character.isalpha() for character in name), f"status {status_id}: name without letters")
                self.assertNotIn("#", name)
                self.assertLessEqual(len(name), 32)
            icon = entry.get("icon")
            if icon is not None:
                self.assertTrue(icon.lower().endswith(".tga"), f"status {status_id}: icon {icon!r} is not .tga")
            self.assertIn(entry["category"], CATEGORIES)
            self.assertIn(entry["refresh"], {"Replace", "Keep"})
            self.assertEqual(entry["stack"], "Single")

            labels = entry.get("value_labels") or []
            slots = [label["slot"] for label in labels]
            self.assertEqual(slots, sorted(set(slots)), f"status {status_id}: duplicate/unsorted value slots")
            for label in labels:
                self.assertIn(label["slot"], (1, 2, 3))
                self.assertIn(label["format"], VALUE_FORMATS)
                self.assertTrue(label["label"].isascii() and label["label"])

    def test_value_statuses_have_value_labels(self) -> None:
        facts = load_facts()
        by_id = {entry["id"]: entry for entry in facts["statuses"]}
        # rAthena's `SendVal1` statuses: weapon enchant, the three cash boosts,
        # push cart and the two monster transformations.
        for status_id in (64, 250, 252, 312, 621, 665, 673):
            entry = by_id.get(status_id)
            self.assertIsNotNone(entry, f"status {status_id} is missing")
            assert entry is not None
            self.assertTrue(entry.get("value_labels"), f"status {status_id} lost its value labels")
            self.assertIn(1, entry["send_values"], f"status {status_id} is not a SendVal1 status")
        # The curated category override covers the health-state debuff the
        # official title table has no data for.
        self.assertEqual(by_id[889]["category"], "Debuff")
        self.assertEqual(by_id[889]["category_source"], "custom")
        self.assertEqual(by_id[64]["value_labels"][0]["format"], "Element")
        self.assertEqual(by_id[250]["value_labels"][0]["format"], "Percent")
        self.assertEqual(by_id[621]["value_labels"][0]["format"], "Id")

    def test_element_table_is_complete(self) -> None:
        facts = load_facts()
        elements = [(entry["id"], entry["name"]) for entry in facts["elements"]]
        self.assertEqual([element_id for element_id, _ in elements], list(range(10)))
        self.assertEqual(elements[0], (0, "Neutral"))
        self.assertEqual(elements[3], (3, "Fire"))

    def test_aura_rows(self) -> None:
        facts = load_facts()
        by_id = {entry["id"]: entry for entry in facts["statuses"]}
        # Assumptio (the first data row, moved out of Rust) + party Assumptio.
        for status_id in (110, 473):
            aura = by_id[status_id].get("aura")
            self.assertIsNotNone(aura, f"status {status_id} lost its Assumptio aura")
            assert aura is not None
            self.assertEqual(aura["kind"], "PersistentGlow")
            self.assertIn("procedural", aura["source"])
        # Only Assumptio keeps a persistent glow; other buffs must not.
        for status_id in (3, 9, 10, 12):
            self.assertNotIn("aura", by_id[status_id], f"status {status_id} must not have a persistent aura")
        for status_id in (0, 875, 30_000):
            self.assertNotIn("aura", by_id.get(status_id, {}))
        # Status 110 and 473 share identical aura data.
        self.assertEqual(by_id[110]["aura"], by_id[473]["aura"])

    def test_aura_and_icon_sources_are_documented(self) -> None:
        facts = load_facts()
        for entry in facts["statuses"]:
            if "aura" in entry:
                self.assertTrue(entry["aura"]["source"])
                for frame in entry["aura"]["textures"]["body_frames"]:
                    self.assertTrue(frame.lower().endswith((".tga", ".bmp")), frame)
            if entry.get("icon") is not None:
                self.assertIn(entry["icon_source"], {"custom", "stateiconimginfo.lub", "robrowser-statusinfo"})

    def test_legacy_name_and_icon_parity(self) -> None:
        facts = load_facts()
        by_id = {entry["id"]: entry for entry in facts["statuses"]}
        legacy = LEGACY.read_text(encoding="utf-8")

        def legacy_list(name: str) -> list[tuple[int, str]]:
            block = re.search(rf"    {name}: \[(.*?)\n    \],", legacy, re.S)
            self.assertIsNotNone(block, f"legacy snapshot has no {name}")
            assert block is not None
            return [
                (int(match.group(1)), match.group(2))
                for match in re.finditer(r'\((\d+), "((?:[^"\\]|\\.)*)"\)', block.group(1))
            ]

        names = legacy_list("names")
        icons = legacy_list("icons")
        extra_names = legacy_list("extra_names")
        self.assertEqual(len(names), 509)
        self.assertEqual(len(icons), 355)
        self.assertEqual(len(extra_names), 10)

        for status_id, name in names:
            expected_id = status_id + 1 if status_id in NAME_SHIFT else status_id
            entry = by_id.get(expected_id)
            self.assertIsNotNone(entry, f"status {expected_id} (legacy {status_id}) is missing")
            assert entry is not None
            self.assertEqual(entry.get("name"), name, f"status {expected_id} (legacy {status_id}) name changed")
        for status_id, icon in icons:
            entry = by_id.get(status_id)
            self.assertIsNotNone(entry, f"status {status_id} is missing")
            assert entry is not None
            # The texture loader resolves GRF names case-insensitively; the old
            # table and the client tables disagree on ASCII case for some icons.
            self.assertEqual(
                (entry.get("icon") or "").lower(),
                icon.lower(),
                f"status {status_id} icon changed",
            )
        for status_id, name in extra_names:
            self.assertEqual(by_id[status_id].get("name"), name, f"status {status_id} extra name changed")

        # The old names in the drifted region are the *next* status' real title.
        self.assertEqual(by_id[1065].get("name"), "Infinity Drink")
        # EFST_SP_SHA (1064) has no English client title and kept no
        # presentation row; the buff bar falls back to the generic name.
        self.assertIsNone(by_id.get(1064))

    def test_robrowser_facts_are_vendored_with_provenance(self) -> None:
        document = json.loads(ROBROWSER.read_text(encoding="utf-8"))
        provenance = document["provenance"]
        self.assertIn("MrAntares/roBrowserLegacy", provenance["url"])
        self.assertEqual(len(provenance["commit"]), 40)
        self.assertEqual(provenance["files"]["StatusInfo.js"]["sha256"], "68788aee20c804eed90d19afb3caaee10f2833f33d3bbc2674cbe3049d10e47f")
        entries = document["entries"]
        self.assertEqual(len(entries), 336)
        self.assertGreater(sum(1 for entry in entries if entry["icon"]), 300)


class GeneratedDataTests(unittest.TestCase):
    def test_generator_check_passes(self) -> None:
        result = subprocess.run(
            [sys.executable, str(GENERATOR), "--check"],
            cwd=REPO,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, f"{GENERATOR} --check failed:\n{result.stdout}\n{result.stderr}")
        self.assertIn("is up to date", result.stdout)

    def test_generated_file_is_sorted_and_unique(self) -> None:
        text = DATA.read_text(encoding="utf-8")
        ids = [int(match.group(1)) for match in re.finditer(r"^            id: (\d+),$", text, re.M)]
        self.assertGreater(len(ids), 500)
        self.assertEqual(ids, sorted(ids))
        self.assertEqual(len(ids), len(set(ids)))

    def test_generated_file_matches_facts(self) -> None:
        facts = load_facts()
        by_id = {entry["id"]: entry for entry in facts["statuses"]}
        text = DATA.read_text(encoding="utf-8")
        blocks = re.split(r"^        \($", text, flags=re.M)[1:]
        seen = 0
        for block in blocks:
            id_match = re.search(r"^            id: (\d+),$", block, re.M)
            if id_match is None:
                continue
            status_id = int(id_match.group(1))
            entry = by_id[status_id]
            self.assertIsNotNone(entry)
            name_match = re.search(r'^            name: "((?:[^"\\]|\\.)*)",$', block, re.M)
            self.assertEqual(name_match.group(1) if name_match else None, entry.get("name"))
            icon_match = re.search(r'^            icon: "((?:[^"\\]|\\.)*)",$', block, re.M)
            self.assertEqual(icon_match.group(1) if icon_match else None, entry.get("icon"))
            category = re.search(r"^            category: (\w+),$", block, re.M)
            self.assertIsNotNone(category)
            assert category is not None
            self.assertEqual(category.group(1), entry["category"])
            seen += 1
        self.assertEqual(seen, len(by_id))


if __name__ == "__main__":
    unittest.main(verbosity=2)
