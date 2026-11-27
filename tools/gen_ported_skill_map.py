#!/usr/bin/env python3
"""Generate `ro-next/src/world/effect/port/skill_map.rs`.

The map answers: *which ported official effect id does skill X play at phase P?*

Sources:

* `tools/data/rodecomp_effect_coverage.json` - per skill, the effect ids the
  official client uses (`used_effect_ids`). This is the id source. Hard-coded
  procedural effects (no descriptor row, empty `used_effect_ids`) fall back to
  the 2019 client effect table (`client_effect_ids_2019`).
* `tools/data/windows_client_skill_effects.json` - the official field each id
  comes from (`successEffectId`, `hitEffectId`, `damageEffectIdOnCaster`,
  `groundEffectId`, ...). The coverage file is an aggregate of this file, so it
  is the only place the *phase* of an id survives.
* `ro-next/archive/data/client/skill_effects.ron` - the phases the client
  currently spawns for a skill. A phase the spawn path never reaches cannot be
  used, so the official phase is intersected with this list (see
  `PHASE_PREFERENCE` for the fallback when the official phase has no descriptor).
  Skills without any descriptor row still spawn their ported body in every
  phase, so their official phase is used directly.
* the port registry sources - `registry::ported_ids()`, emitted as the
  `PORTED` const. A Rust test pins the generated const to the runtime list.

Run: `./tools/gen_ported_skill_map.py`
Check: `./tools/gen_ported_skill_map.py --check`
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "ro-next/src/world/effect/port/skill_map.rs"

COVERAGE = ROOT / "tools/data/rodecomp_effect_coverage.json"
WINDOWS_EFFECTS = ROOT / "tools/data/windows_client_skill_effects.json"
SKILL_EFFECTS_RON = ROOT / "ro-next/archive/data/client/skill_effects.ron"
EFFECT_ID_SOURCE = ROOT / "ragnarok-packets/src/skill.rs"
PORT_DIR = ROOT / "ro-next/src/world/effect/port"

REGISTRY_SOURCES = (
    "registry_class_change.rs",
    "registry_textured_aura.rs",
    "registry_batch_a.rs",
    "registry_batch_b.rs",
    "registry_batch_c.rs",
    "registry_batch_d.rs",
    "registry_batch_e.rs",
    "registry_batch_f.rs",
    "registry_batch_g.rs",
    "registry_batch_h.rs",
    "registry_batch_i.rs",
    "registry_batch_j.rs",
    "registry_batch_k.rs",
    "registry_batch_l.rs",
)

# The client's `SkillPhase` order (`skill_effects.rs::SkillPhase::index`).
PHASES = ["Ground", "Cast", "Success", "Release", "Hit", "Caster"]
PHASE_INDEX = {phase: index for index, phase in enumerate(PHASES)}

# Official SkillEffect field -> the phase the client spawns for it.
FIELD_PHASE = {
    "effectId": "Success",
    "successEffectId": "Success",
    "successEffectIdOnCaster": "Success",
    "effectIdOnCaster": "Success",
    "hitEffectId": "Hit",
    "firstHitEffectId": "Hit",
    "damageEffectIdOnTarget": "Hit",
    "damageEffectIdOnCaster": "Caster",
    "groundEffectId": "Ground",
    "beforeHitEffectId": "Release",
    "releaseEffectId": "Release",
    "castEffectId": "Cast",
    "beginCastEffectId": "Cast",
    "beginEffectId": "Cast",
}

# When the official phase has no descriptor in the client table, use the
# nearest phase the spawn path actually reaches.
PHASE_PREFERENCE = {
    "Ground": ["Ground", "Hit", "Release", "Success", "Cast", "Caster"],
    "Cast": ["Cast", "Success", "Release", "Hit", "Ground", "Caster"],
    "Success": ["Success", "Cast", "Release", "Hit", "Ground", "Caster"],
    "Release": ["Release", "Hit", "Success", "Cast", "Ground", "Caster"],
    "Hit": ["Hit", "Release", "Success", "Cast", "Ground", "Caster"],
    "Caster": ["Caster", "Success", "Cast", "Hit", "Release", "Ground"],
}

# Impact / ground effects are launched on the target; cast/success effects and
# travelling bodies are launched on the caster (`runtime.position` is the
# master position, `runtime.target_offset` the target).
TARGET_PHASES = {"Ground", "Release", "Hit"}

# `JobInheritance.cpp:2086` (`AS_GRIMTOOTH`) and `:2100` (`AS_SPEARBOOMERANG`
# via `PlaySkillEffect`) launch these on the source actor; both bodies travel
# from `runtime.position` towards `runtime.target_offset`.
TRAVEL_FROM_CASTER = {"Grimtooth", "Spearbmr"}


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def effect_ids_by_name() -> dict[str, int]:
    """Parses the `EffectId` enum, honouring `#[numeric_value(N)]` gaps."""
    source = read(EFFECT_ID_SOURCE)
    start = source.index("pub enum EffectId {")
    end = source.index("\n}", start)
    body = source[start:end]

    names: dict[str, int] = {}
    value = 0
    pending: int | None = None

    for line in body.splitlines()[1:]:
        line = line.split("//", 1)[0].strip()
        if not line:
            continue

        match = re.fullmatch(r"#\[numeric_value\((\d+)\)\]", line)
        if match:
            pending = int(match.group(1))
            continue

        match = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*),?", line)
        if match:
            if pending is not None:
                value = pending
                pending = None
            names[match.group(1)] = value
            value += 1

    return names


def array_effect_ids(source: str, function: str) -> list[str]:
    match = re.search(
        rf"pub fn {function}\(\)[^{{]*\{{\s*&\[(.*?)\]\s*\}}",
        source,
        re.S,
    )
    if not match:
        raise SystemExit(f"could not find {function}()")
    return re.findall(r"EffectId::([A-Za-z0-9_]+)", match.group(1))


def ported_effect_names() -> list[str]:
    """Reconstructs `registry::ported_ids()` from the registry sources."""

    names: list[str] = []
    for filename in REGISTRY_SOURCES:
        names += array_effect_ids(read(PORT_DIR / filename), "ids")

    registry = read(PORT_DIR / "registry.rs")
    match = re.search(r"ids\.extend_from_slice\(&\[(.*?)\]\);", registry, re.S)
    if not match:
        raise SystemExit("could not find the hand-written list in registry.rs")
    names += re.findall(r"EffectId::([A-Za-z0-9_]+)", match.group(1))

    # `PORTED` is the deduplicated set; keep first-seen order, then sort by id.
    seen: list[str] = []
    for name in names:
        if name not in seen:
            seen.append(name)
    return seen


def descriptor_phases() -> dict[int, list[str]]:
    """Skill -> phases present in `skill_effects.ron` (the client spawn path)."""

    source = re.sub(r"//[^\n]*", "", read(SKILL_EFFECTS_RON))
    phases: dict[int, list[str]] = {}
    for skill, phase in re.findall(r"skill:\s*(\d+),\s*phase:\s*([A-Za-z]+)", source):
        bucket = phases.setdefault(int(skill), [])
        if phase in PHASE_INDEX and phase not in bucket:
            bucket.append(phase)

    for bucket in phases.values():
        bucket.sort(key=PHASE_INDEX.__getitem__)

    return phases


def official_phase(fields: list[dict]) -> str:
    """The client phase the official SkillEffect field belongs to."""
    return next((FIELD_PHASE[field["field"]] for field in fields if field["field"] in FIELD_PHASE), "Success")


def choose_phase(fields: list[dict], descriptors: list[str]) -> str:
    official = official_phase(fields)

    for phase in PHASE_PREFERENCE[official]:
        if phase in descriptors:
            return phase

    raise SystemExit(f"no client descriptor phase for official phase {official}")


# Rustfmt's `fn_call_width = 80` heuristic in `rustfmt.toml` makes it split a
# tuple element of the `MAPPED` array once the source line would exceed this
# many columns (4-space indent included). Emitting the split form up front keeps
# the generated file `rustfmt --check` clean instead of churning on every run.
MAPPED_ROW_SINGLE_LINE_MAX = 87


def mapped_row(skill: int, phase: str, name: str, anchor: str) -> list[str]:
    """One `MAPPED` element, already laid out the way rustfmt keeps it."""
    line = f"    (SkillId({skill}), SkillPhase::{phase}, EffectId::{name}, PortedAnchor::{anchor}),"

    if len(line) <= MAPPED_ROW_SINGLE_LINE_MAX:
        return [line]

    return [
        "    (",
        f"        SkillId({skill}),",
        f"        SkillPhase::{phase},",
        f"        EffectId::{name},",
        f"        PortedAnchor::{anchor},",
        "    ),",
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="fail if the committed file is stale")
    arguments = parser.parse_args()

    name_to_id = effect_ids_by_name()
    id_to_name = {effect_id: name for name, effect_id in name_to_id.items()}
    ported_names = ported_effect_names()

    missing = sorted(name for name in ported_names if name not in name_to_id)
    if missing:
        raise SystemExit(f"ported ids missing from the EffectId enum: {missing}")
    ported_set = set(ported_names)

    coverage = json.loads(read(COVERAGE))["skills"]
    windows = json.loads(read(WINDOWS_EFFECTS))["skills"]
    descriptors = descriptor_phases()

    rows: list[tuple[int, str, int, str, str]] = []
    skipped_no_descriptor: list[int] = []

    for key, entry in coverage.items():
        skill = int(key)
        raw_used = entry.get("used_effect_ids") or []
        used = [effect_id for effect_id in raw_used if id_to_name.get(effect_id) in ported_set]
        skill_descriptors = descriptors.get(skill, [])
        hardcoded_effect = False

        if not used and not raw_used and not skill_descriptors:
            # Hard-coded official effects have no descriptor rows and an empty
            # `used_effect_ids` (the coverage audit classifies them as
            # "official-effect-is-hardcoded-procedural"). The 2019 client effect
            # table (`skilleffectinfolist.lub`, extracted as
            # `client_effect_ids_2019`) still names the effect id, so the ported
            # body can be mapped for skills the descriptor pipeline never sees
            # (e.g. `HT_BLITZBEAT` 129 -> `EF_BLITZBEAT` 115). The fallback is
            # deliberately limited to skills with *no* official effect ids: it
            # must not replace the curated ids anywhere else.
            used = [
                effect_id
                for effect_id in (entry.get("client_effect_ids_2019") or [])
                if id_to_name.get(effect_id) in ported_set
            ]
            hardcoded_effect = bool(used)

        if not used:
            continue

        if not skill_descriptors and not hardcoded_effect:
            # Skills with descriptor rows keep the descriptor-phase constraint
            # below; a ported id on a descriptor-less skill is only usable when
            # it is the hard-coded official effect found above.
            skipped_no_descriptor.append(skill)
            continue

        # Every skill with ported ids in the coverage data resolves to a single
        # ported id today. If that ever changes, prefer the first id whose
        # official field phase the client can actually reach.
        candidates = [effect_id for effect_id in used if effect_id in id_to_name]
        chosen = candidates[0]
        for effect_id in candidates:
            fields = [field for field in windows.get(key, []) if effect_id in field["effects"]]
            official = official_phase(fields)
            if official in skill_descriptors:
                chosen = effect_id
                break

        effect_name = id_to_name[chosen]
        fields = [field for field in windows.get(key, []) if chosen in field["effects"]]

        if skill_descriptors:
            phase = choose_phase(fields, skill_descriptors)
            anchor = "Target" if phase in TARGET_PHASES else "Caster"
        else:
            # No descriptor rows: the ported backend is the only visual, so the
            # spawn path reaches every phase and the official phase is used
            # as-is. `CSession::PlaySkillTargetEffect` spawns these target-effect
            # entries at `target->render.position` (`JobInheritance.cpp:1586-1590`).
            phase = official_phase(fields)
            anchor = "Target"

        if effect_name in TRAVEL_FROM_CASTER:
            anchor = "Caster"

        rows.append((skill, phase, name_to_id[effect_name], effect_name, anchor))

    rows.sort(key=lambda row: (row[0], PHASE_INDEX[row[1]]))
    ported_sorted = sorted(ported_set, key=name_to_id.__getitem__)

    lines: list[str] = []
    lines.append("//! Skill -> ported effect-id map. GENERATED FILE - do not edit by hand.")
    lines.append("//!")
    lines.append("//! Regenerate with: `./tools/gen_ported_skill_map.py`")
    lines.append("//! Drift check:    `./tools/gen_ported_skill_map.py --check`")
    lines.append("//!")
    lines.append("//! Id source: `tools/data/rodecomp_effect_coverage.json` (`used_effect_ids`).")
    lines.append("//! The phase comes from the official field the id belongs to in")
    lines.append("//! `tools/data/windows_client_skill_effects.json` (`successEffectId` ->")
    lines.append("//! `Success`, `hitEffectId` -> `Hit`, `damageEffectIdOnCaster` -> `Caster`,")
    lines.append("//! `groundEffectId` -> `Ground`, ...), intersected with the phases the client")
    lines.append("//! already spawns (`skill_effects.ron`). Only ids that")
    lines.append("//! `super::registry::spawn` accepts are emitted;")
    lines.append("//! `ported_const_matches_registry` pins [`PORTED`] to `registry::ported_ids()`.")
    lines.append("")
    lines.append("use ragnarok_packets::{EffectId, SkillId};")
    lines.append("")
    lines.append("use crate::world::SkillPhase;")
    lines.append("")
    lines.append("/// Every effect id the port can spawn (`registry::ported_ids()`, deduplicated")
    lines.append("/// and sorted by numeric id).")
    lines.append("pub const PORTED: &[EffectId] = &[")
    for name in ported_sorted:
        lines.append(f"    EffectId::{name},")
    lines.append("];")
    lines.append("")
    lines.append("/// Where the official client launched the effect.")
    lines.append("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
    lines.append("pub enum PortedAnchor {")
    lines.append("    /// On the caster's position (self-targeted and travelling bodies).")
    lines.append("    Caster,")
    lines.append("    /// On the target's position (impact and ground effects).")
    lines.append("    Target,")
    lines.append("}")
    lines.append("")
    lines.append("/// One generated mapping, sorted by `(skill, phase)`.")
    lines.append("pub const MAPPED: &[(SkillId, SkillPhase, EffectId, PortedAnchor)] = &[")
    for skill, phase, effect_id, name, anchor in rows:
        lines.extend(mapped_row(skill, phase, name, anchor))
    lines.append("];")
    lines.append("")
    lines.append("/// The ported effect id `skill` plays at `phase`, if one is registered.")
    lines.append("pub fn ported_effect_for(skill: SkillId, phase: SkillPhase) -> Option<EffectId> {")
    lines.append("    mapped(skill, phase).map(|(_, _, effect_id, _)| *effect_id)")
    lines.append("}")
    lines.append("")
    lines.append("/// Where `ported_effect_for` should anchor the effect.")
    lines.append("pub fn ported_anchor_for(skill: SkillId, phase: SkillPhase) -> PortedAnchor {")
    lines.append("    mapped(skill, phase)")
    lines.append("        .map(|(_, _, _, anchor)| *anchor)")
    lines.append("        .unwrap_or(PortedAnchor::Caster)")
    lines.append("}")
    lines.append("")
    lines.append("fn mapped(skill: SkillId, phase: SkillPhase) -> Option<&'static (SkillId, SkillPhase, EffectId, PortedAnchor)> {")
    lines.append("    MAPPED")
    lines.append("        .binary_search_by_key(&(skill.0, phase.index()), |(mapped_skill, mapped_phase, ..)| {")
    lines.append("            (mapped_skill.0, mapped_phase.index())")
    lines.append("        })")
    lines.append("        .ok()")
    lines.append("        .map(|index| &MAPPED[index])")
    lines.append("}")
    lines.append("")
    lines.append("#[cfg(test)]")
    lines.append("mod tests {")
    lines.append("    use super::*;")
    lines.append("    use crate::world::effect::port::registry;")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn ported_const_matches_registry() {")
    lines.append("        let mut expected: Vec<u32> = registry::ported_ids().iter().map(|effect| *effect as u32).collect();")
    lines.append("        expected.sort_unstable();")
    lines.append("        expected.dedup();")
    lines.append("")
    lines.append("        let actual: Vec<u32> = PORTED.iter().map(|effect| *effect as u32).collect();")
    lines.append("")
    lines.append("        assert_eq!(actual, expected, \"PORTED is stale: run tools/gen_ported_skill_map.py\");")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn every_mapped_pair_spawns() {")
    lines.append("        for (skill, phase, effect, _) in MAPPED {")
    lines.append("            assert!(registry::spawn(*effect).is_some(), \"{skill:?}/{phase:?} maps to an unported id\");")
    lines.append("            assert!(")
    lines.append("                PORTED.iter().any(|ported| *ported as u32 == *effect as u32),")
    lines.append("                \"{skill:?}/{phase:?} maps to an id missing from PORTED\",")
    lines.append("            );")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn mapped_table_is_sorted_and_unique() {")
    lines.append("        let mut previous = None;")
    lines.append("        for (skill, phase, ..) in MAPPED {")
    lines.append("            let key = (skill.0, phase.index());")
    lines.append("            assert!(")
    lines.append("                previous.is_none_or(|previous| previous < key),")
    lines.append("                \"MAPPED is not sorted at {key:?}\"")
    lines.append("            );")
    lines.append("            previous = Some(key);")
    lines.append("        }")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn unmapped_skills_return_none() {")
    lines.append("        // SM_BASH only uses the generic hit ids (`0`/`1`), none of which is ported.")
    lines.append("        assert!(ported_effect_for(SkillId(5), SkillPhase::Hit).is_none());")
    lines.append("        assert!(ported_effect_for(SkillId(u16::MAX), SkillPhase::Success).is_none());")
    lines.append("        assert!(ported_effect_for(SkillId(5), SkillPhase::Success).is_none());")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn blitz_beat_maps_to_its_ported_body() {")
    lines.append("        // `HT_BLITZBEAT` (129) has no `skill_effects.ron` descriptor row,")
    lines.append("        // so this mapping can only come from the 2019 client effect table")
    lines.append("        // (`client_effect_ids_2019` -> `EF_BLITZBEAT` 115). It is what makes")
    lines.append("        // the opt-in ported body reachable for the skill.")
    lines.append("        assert_eq!(")
    lines.append("            ported_effect_for(SkillId(129), SkillPhase::Success).map(|effect| effect as u32),")
    lines.append("            Some(EffectId::Blitzbeat as u32)")
    lines.append("        );")
    lines.append("        assert_eq!(ported_anchor_for(SkillId(129), SkillPhase::Success), PortedAnchor::Target);")
    lines.append("    }")
    lines.append("")
    lines.append("    #[test]")
    lines.append("    fn every_mapped_pair_resolves_through_lookup() {")
    lines.append("        for (skill, phase, effect, anchor) in MAPPED {")
    lines.append("            assert_eq!(")
    lines.append("                ported_effect_for(*skill, *phase).map(|effect| effect as u32),")
    lines.append("                Some(*effect as u32)")
    lines.append("            );")
    lines.append("            assert_eq!(ported_anchor_for(*skill, *phase), *anchor);")
    lines.append("        }")
    lines.append("    }")
    lines.append("}")
    lines.append("")

    generated = "\n".join(lines)

    if arguments.check:
        current = read(OUT) if OUT.exists() else ""
        if current != generated:
            print(f"{OUT.relative_to(ROOT)} is stale; run ./tools/gen_ported_skill_map.py", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} is up to date ({len(rows)} mappings)")
        return 0

    OUT.write_text(generated, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}: {len(ported_sorted)} ported ids, {len(rows)} skill/phase mappings")
    if skipped_no_descriptor:
        print(f"skipped {len(skipped_no_descriptor)} skills without a client descriptor: {skipped_no_descriptor}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
