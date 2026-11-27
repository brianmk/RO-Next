#!/usr/bin/env python3
"""Generate the client's official auto-attack hit feedback tables.

Writes ``ro-next/archive/data/client/hit_feedback.ron``, the data behind the
normal (auto) attack hit sound and target hit effect:

* **weapon -> impact wave** - the official client's
  ``WeaponHitWaveNameTable`` (``GetWeaponHitWaveName``,
  ``Source/Contents/Sound/HitSounds.cpp``). The table is keyed by the client's
  base weapon class; this generator maps it onto the client's ``WeaponKind``
  enum (the same enum the data-driven ``weapons.ron`` registry uses).
* **unarmed fist roll** / **enemy (no attacker weapon) roll** /
  **Taekwon job ids** - the three branches of ``GetWeaponHitWaveName`` that
  are hard-coded in the official function.
* **hit effect job ids** - the ``effectId`` switch of
  ``CGameActor::ProcessHitMessage`` (``RenderObject.cpp:3800-3822``):
  ``EF_HITLINE7`` for the Taekwon/Star job family and ``EF_VIOLEN_HIT`` for
  Violen, ``EF_HIT1`` otherwise; a critical hit adds ``EF_HIT2``.
* **defender job -> clothing wave** - ``InitJobHitWaveNames``. Only the rows
  that differ from ``player_clothes.wav`` are emitted; the client applies the
  official ``job > 3950 -> job - 3950`` index shift.

Inputs (committed, so this generator and its ``--check`` run offline):

* ``tools/data/hit_feedback_facts.json`` - extracted from the CondaRO client
  archive (``CondaRO.grf``, ``data\\luafiles514\\lua files\\datainfo\\weapontable.lub``)
  and from the matching client decompilation
  (``Source/Contents/Sound/HitSounds.cpp``). The JSON records the exact source
  and the method line ranges.
* ``tools/gen_weapons.py`` - the ``KINDS`` list is imported so the generator
  fails when a new weapon kind has no official impact wave.

The generated file is embedded in the client with ``include_str!`` as a
compiled fallback (``world/entity/hit_feedback.rs``).

Usage:
    ./tools/gen_hit_feedback.py            # regenerate the data file
    ./tools/gen_hit_feedback.py --check    # fail if the committed file is out of date
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from gen_weapons import KINDS  # noqa: E402

DEFAULT_FACTS = Path("tools/data/hit_feedback_facts.json")
DEFAULT_OUTPUT = Path("ro-next/archive/data/client/hit_feedback.ron")

#: ``weapontable.lub`` ``Weapon_IDs`` symbol -> the Rust ``WeaponKind`` it
#: resolves to. ``GetRealWeaponId`` (``JobInheritance.cpp:815-822``) folds the
#: 31..102 item classes onto these base classes; ``weapons.ron`` already does
#: that folding through its ``server_types`` / ``item_views`` resolution.
CLASS_KINDS = {
    "WEAPONTYPE_NONE": "Fist",
    "WEAPONTYPE_SHORTSWORD": "Dagger",
    "WEAPONTYPE_SWORD": "OneHandedSword",
    "WEAPONTYPE_TWOHANDSWORD": "TwoHandedSword",
    "WEAPONTYPE_SPEAR": "OneHandedSpear",
    "WEAPONTYPE_TWOHANDSPEAR": "TwoHandedSpear",
    "WEAPONTYPE_AXE": "OneHandedAxe",
    "WEAPONTYPE_TWOHANDAXE": "TwoHandedAxe",
    "WEAPONTYPE_MACE": "Mace",
    "WEAPONTYPE_TWOHANDMACE": "TwoHandedMace",
    "WEAPONTYPE_ROD": "Staff",
    "WEAPONTYPE_BOW": "Bow",
    "WEAPONTYPE_KNUKLE": "Knuckle",
    "WEAPONTYPE_INSTRUMENT": "MusicalInstrument",
    "WEAPONTYPE_WHIP": "Whip",
    "WEAPONTYPE_BOOK": "Book",
    "WEAPONTYPE_CATARRH": "Katar",
    "WPCLASS_GUN_HANDGUN": "Revolver",
    "WPCLASS_GUN_RIFLE": "Rifle",
    "WPCLASS_GUN_GATLING": "Gatling",
    "WPCLASS_GUN_SHOTGUN": "Shotgun",
    "WPCLASS_GUN_GRANADE": "Grenade",
    "WPCLASS_SYURIKEN": "Huuma",
    "WPCLASS_TWOHANDROD": "TwoHandedStaff",
    "WEAPONTYPE_SHORTSWORD_SHORTSWORD": "DoubleDagger",
    "WEAPONTYPE_SWORD_SWORD": "DoubleSword",
    "WEAPONTYPE_AXE_AXE": "DoubleAxe",
    "WEAPONTYPE_SHORTSWORD_SWORD": "DaggerSword",
    "WEAPONTYPE_SHORTSWORD_AXE": "DaggerAxe",
    "WEAPONTYPE_SWORD_AXE": "SwordAxe",
}


def ron_str(value: str) -> str:
    """Quote a string as a RON string (JSON's escaping is valid RON)."""
    return json.dumps(value, ensure_ascii=False)


def load_json(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def build(facts: dict) -> dict:
    kind_order = [row["kind"] for row in KINDS]
    by_kind: dict[str, str] = {}
    for row in facts["weapon_hit_waves"]:
        kind = CLASS_KINDS.get(row["class_name"])
        if kind is None:
            raise SystemExit(f"unknown weapon class {row['class_name']} in the facts file")
        if kind in by_kind:
            raise SystemExit(f"duplicate weapon kind {kind} in the facts file")
        by_kind[kind] = row["wave"]

    missing = [kind for kind in kind_order if kind not in by_kind]
    extra = [kind for kind in by_kind if kind not in kind_order]
    if missing:
        raise SystemExit(f"weapon kinds without an official hit wave: {', '.join(missing)}")
    if extra:
        raise SystemExit(f"facts contain weapon kinds not in gen_weapons.KINDS: {', '.join(extra)}")

    # Rows that repeat the array default are not data; the official
    # initializer writes a few of them (`jobWaves[0] = "player_clothes.wav"`).
    by_index: dict[int, str] = {}
    for row in facts["job_hit_waves"]:
        if row["wave"] == facts["job_default_wave"]:
            continue
        previous = by_index.setdefault(row["index"], row["wave"])
        if previous != row["wave"]:
            raise SystemExit(f"conflicting job hit waves for index {row['index']}: {previous} / {row['wave']}")
    job_rows = [{"index": index, "wave": wave} for index, wave in sorted(by_index.items())]

    for name in ("fist_waves", "enemy_waves"):
        if len(facts[name]) != 4:
            raise SystemExit(f"{name} must contain the four official variants")

    # `ProcessHitMessage` (`RenderObject.cpp:3801-3821`): fourteen Taekwon/Star
    # family jobs use EF_HITLINE7 and the two Violen jobs use EF_VIOLEN_HIT.
    if len(facts["hitline7_jobs"]) != 14:
        raise SystemExit(f"hitline7_jobs must contain the fourteen official jobs, got {len(facts['hitline7_jobs'])}")
    if len(facts["violen_jobs"]) != 2:
        raise SystemExit(f"violen_jobs must contain the two official jobs, got {len(facts['violen_jobs'])}")

    return {
        "weapon_waves": [(kind, by_kind[kind]) for kind in kind_order],
        "fist_waves": facts["fist_waves"],
        "enemy_waves": facts["enemy_waves"],
        "taekwon_jobs": facts["taekwon_jobs"],
        "hitline7_jobs": facts["hitline7_jobs"],
        "violen_jobs": facts["violen_jobs"],
        "job_default_wave": facts["job_default_wave"],
        "job_waves": job_rows,
    }


def render(facts: dict, data: dict) -> str:
    lines: list[str] = []
    add = lines.append

    add("// Official auto-attack hit feedback tables for RO-Next.")
    add("//")
    add("// Generated by `tools/gen_hit_feedback.py`; do not edit by hand.")
    add("// Regenerate with: `./tools/gen_hit_feedback.py`")
    add("// Drift check:      `./tools/gen_hit_feedback.py --check`")
    add("//")
    add("// Sources (committed facts, no game files needed to regenerate):")
    weapon = facts["provenance"]["weapon_hit_waves"]
    add(f"//   * {weapon['extracted_from']}")
    add(f"//     table `{weapon['table']}`; logic {weapon['logic']}.")
    add(f"//     {weapon['note']}")
    job = facts["provenance"]["job_hit_waves"]
    add(f"//   * {job['extracted_from']} (`{job['method_line']}`).")
    add(f"//     {job['note']}")
    hit = facts["provenance"]["hit_effects"]
    add(f"//   * {hit['extracted_from']}: {hit['logic']}")
    add(f"//     effect ids: {hit['effect_ids']}.")
    add(f"//     job ids: {hit['job_ids']}.")
    add("//")
    add("// `weapon_waves` is indexed by the client `WeaponKind`; the official")
    add("// `GetWeaponHitWaveName(-1)` (no attacker weapon) and `realWeapon == 0`")
    add("// (bare hand, non-Taekwon) branches are the two random `*_waves` lists.")
    add("// `job_waves` is the defender's clothing impact wave; jobs above 3950 use")
    add("// `index = job - 3950`, every unlisted job uses `job_default_wave`.")
    add("(")
    add(f"    provenance: {ron_str('CondaRO client weapontable.lub + client decompilation; generated by tools/gen_hit_feedback.py')},")
    add("    version: 2,")
    add("    // Official `WeaponHitWaveNameTable`, one row per `WeaponKind`.")
    add("    weapon_waves: [")
    for kind, wave in data["weapon_waves"]:
        add(f"        (kind: {kind}, wave: {ron_str(wave)}),")
    add("    ],")
    add("    // `realWeapon == 0`, not Taekwon: random `_hit_fist1..4.wav`.")
    add("    fist_waves: [")
    for wave in data["fist_waves"]:
        add(f"        {ron_str(wave)},")
    add("    ],")
    add("    // `realWeapon == -1` (monster/unarmed attacker): random")
    add("    // `_enemy_hit_normal1..4.wav`.")
    add("    enemy_waves: [")
    for wave in data["enemy_waves"]:
        add(f"        {ron_str(wave)},")
    add("    ],")
    add("    // `GetDisplayJob() == JT_TAEKWON || JT_TAEKWON_B` skips the fist roll")
    add("    // and returns the Fist kind row (`_hit_mace.wav`).")
    add(f"    taekwon_jobs: {data['taekwon_jobs']},")
    add("    // `ProcessHitMessage` (`RenderObject.cpp:3801-3817`): the Taekwon/Star")
    add("    // job family spawns EF_HITLINE7 (434) instead of EF_HIT1 (0).")
    add(f"    hitline7_jobs: {data['hitline7_jobs']},")
    add("    // EF_VIOLEN_HIT (2420) jobs (`RenderObject.cpp:3818-3821`). The packet")
    add("    // crate has no EffectId for 2420, so the client suppresses the effect")
    add("    // instead of inventing EF_HIT1.")
    add(f"    violen_jobs: {data['violen_jobs']},")
    add("    // `InitJobHitWaveNames`: the array default and the explicit rows that")
    add("    // differ from it (`jobWaves[job]`, or `job - 3950` for jobs > 3950).")
    add(f"    job_default_wave: {ron_str(data['job_default_wave'])},")
    add("    job_waves: [")
    for row in data["job_waves"]:
        add(f"        (index: {row['index']}, wave: {ron_str(row['wave'])}),")
    add("    ],")
    add(")")
    add("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    args = parser.parse_args()

    facts = load_json(Path(args.facts))
    data = build(facts)
    content = render(facts, data)

    output = Path(args.output)
    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_hit_feedback.py", file=sys.stderr)
            return 1
        print(
            f"{output} is up to date ({len(data['weapon_waves'])} weapon kinds, "
            f"{len(data['job_waves'])} job waves)"
        )
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(
        f"{output}: {len(data['weapon_waves'])} weapon kinds, "
        f"{len(data['job_waves'])} job waves, "
        f"{len(data['fist_waves'])} fist waves, {len(data['enemy_waves'])} enemy waves"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
