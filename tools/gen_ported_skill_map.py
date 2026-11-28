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

Skills whose official visual is not spawned by either skill-use switch are
curated in `CURATED_SKILL_EFFECTS` below (the option auras: `EF_RUWACH` is
re-launched from `CGameActor::UpdateStateEffects`, not from `skill_spawns`).
Extra lanes that the `SkillEffect` data cannot see (the cast-start effect of
`CSession::GetSkillActionInfo`, the hard-coded ground/unit and damage launches
of the descriptor-bearing skills with an empty `used_effect_ids`) are listed in
`CURATED_EXTRA_EFFECTS`, each with the decompiled `file:line` of its launch
site.

Run: `./tools/gen_ported_skill_map.py`
Check: `./tools/gen_ported_skill_map.py --check`
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import textwrap
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

# Support/buff skills whose official `LaunchEffect` runs on `target`, not on
# `source` (the `successEffectId` field alone does not encode the actor):
# `GameModePackets.cpp` cases for AL_INCAGI/AL_BLESSING/AL_DECAGI/AL_CURE/
# PR_IMPOSITIO/PR_SUFFRAGIUM/PR_ASPERSIO/PR_STRECOVERY/PR_LEXDIVINA/
# PR_LEXAETERNA/ALL_RESURRECTION/TF_DETOXIFY/PR_SLOWPOISON. PR_KYRIE,
# PR_MAGNIFICAT, PR_GLORIA and AL_ANGELUS stay on the caster (`source`).
TARGETED_CAST_SKILLS = {29, 30, 34, 35, 53, 54, 66, 67, 68, 71, 72, 76, 78}

# `JobInheritance.cpp:2086` (`AS_GRIMTOOTH`) and `:2100` (`AS_SPEARBOOMERANG`
# via `PlaySkillEffect`) launch these on the source actor; both bodies travel
# from `runtime.position` towards `runtime.target_offset`.
TRAVEL_FROM_CASTER = {"Grimtooth", "Spearbmr"}

# Skills whose official effect is not spawned by the skill-use switch, so the
# coverage/2019 fallback cannot know it. The client starts these from the
# actor's option/state flags instead:
#
# * `AL_RUWACH` (24) -> `EF_RUWACH` (33): the Acolyte reveal aura. The skill
#   switch is a literal `case AL_RUWACH: break;` (`GameModePackets.cpp:21615`)
#   and the 2019 table only carries `hitEffectId 1`, but
#   `CGameActor::UpdateStateEffects` re-launches `EF_RUWACH` every
#   `EffectLifetime(33)` = 200 frames while `OPTION_RUWACH` is set
#   (`GameActor.cpp:704-714`; population twin `RenderObject.cpp:4697-4705`).
#   The aura starts when the skill completes (0x09CB), hence the `Success`
#   phase.
#
# * `HT_DETECTING` (130) -> `EF_DETECTING` (119): Detect is `TargetType: Ground`
#   (`db/pre-re/skill_db.yml:130`), so the server answers with
#   `ZC_NOTIFY_GROUNDSKILL` (`clif_skill_poseffect`, `src/map/clif.cpp:6143-6164`,
#   reached from `skill_castend_pos2` because Detect is not `INF_SELF_SKILL`,
#   `src/map/skill.cpp:14381-14384`). The official ground path
#   `CGameActor::PlayGroundSkillEffect` (`RenderObject.cpp:1001`) has a
#   `case HT_DETECTING` that calls `LaunchDummyEffect(position, 0x77,
#   EffectLifetime(0x77) + 10, 0)` (`RenderObject.cpp:1096-1103`), i.e.
#   `EF_DETECTING` (119, `EffectTypes.h:127`, lifetime 250 from
#   `EffectTypes.cpp:128`) at the clicked cell. Its body is
#   `TexturedSkillAura(0)` (`RagEffect.cpp:23953-23954` -> `:15281-15359`).
#   The 2019 SkillEffect table has no descriptor field for it (the coverage
#   audit marks it `official-effect-is-hardcoded-procedural`), so the
#   coverage/2019 fallback cannot find it; the effect is curated here and
#   spawned on the ground lane that delivers Detect.
CURATED_SKILL_EFFECTS: dict[int, tuple[str, str]] = {
    24: ("Ruwach", "Success"),
    130: ("Detecting", "Ground"),
}

# Additional lanes of a skill that the `SkillEffect` coverage data cannot see.
# A skill can have a ported id on a lane that no `windows_client_skill_effects.json`
# field names, so those pairs are curated here instead of being derived.
#
# Every row is `(effect name, phase, citation)`. The citation is the exact
# decompiled launch site (`file:line`) and is emitted into the generated test,
# so regeneration stays deterministic and the provenance of each curated id
# survives.
#
# Cast-start lanes (`CSession::GetSkillActionInfo`, launched on the caster
# before the cast completes):
#
# * `SN_WINDWALK` (383) -> `EF_BEGINSPELL6` (58) on `Cast`: the cast-start lane
#   (`SkillActionInfo.cpp:165-178`, the `SN_WINDWALK` case at `:165` and
#   `effectId = EF_BEGINSPELL6` at `:178`) handed to `LaunchSkillActorEffect`
#   (`GameModePackets.cpp:12947-12953` -> `:17551-17559`, reached only when the
#   cast has a `delay != 0`). Body `RagEffect.cpp:23584-23591`, lifetime 40
#   (`EffectTypes.cpp:67`). Its success lane (`EF_PORTAL4` 389) stays
#   coverage-driven.
#
# Ground lanes of the descriptor-bearing skills whose `used_effect_ids` is
# empty (the 2019 table alone is not enough: for most of them it only carries
# the generic `EF_HIT1`/`EF_HIT2`, which must not be attached to `Success`).
# The launch is always a hard-coded `LaunchEffect`/`LaunchDummyEffect`:
#
# * `PR_SANCTUARY` (70) -> `EF_BOTTOM_SANC` (319): skill-unit body,
#   `CSkill::OnProcess` `case UNT_SANCTUARY:` (`MsgEffect.cpp:600-608`,
#   `LaunchEffect` at `:602`, lifetime 9999 `EffectTypes.cpp:328`). The ground
#   dispatch also launches `EF_SANCTUARY` (83, `RenderObject.cpp:1054-1056`),
#   which the port does not register.
# * `PR_MAGNUS` (79) -> `EF_BOTTOM_MAG` (318): `case UNT_MAGNUS:`
#   (`MsgEffect.cpp:816-824`, launch `:818`, lifetime 9999
#   `EffectTypes.cpp:327`). `EF_MAGNUS` (113, `RenderObject.cpp:1049-1051`) is
#   not ported.
# * `WZ_ICEWALL` (87) -> `EF_ICEWALL` (74): ground dispatch
#   (`RenderObject.cpp:1117-1121`, `LaunchDummyEffect(0x4A)` at `:1119`); the
#   same body is the unit launch (`MsgEffect.cpp:846-851`).
# * `WZ_STORMGUST` (89) -> `EF_STORMGUST` (89): ground dispatch
#   (`RenderObject.cpp:1133-1141`, the default-path `LaunchDummyEffect(0x59)`
#   at `:1139`; the min-effect path uses `EF_STORM_MIN` 708 which is not
#   ported). `EF_COLDHIT` (51) stays the per-hit effect, not ported.
# * `WZ_HEAVENDRIVE` (91) -> `EF_HEAVENSDRIVE` (142): ground dispatch
#   (`RenderObject.cpp:1059-1063`, `LaunchDummyEffect(0x8E)` at `:1061`,
#   `EffectTypes.h:150`). The per-hit `EF_EARTHHIT` (147) is not ported.
# * `SA_LANDPROTECTOR` (288) -> `EF_BOTTOM_LA` (242): `case UNT_LANDPROTECTOR:`
#   (`MsgEffect.cpp:687-695`, launch `:689`). `EF_LANDPROTECTOR` (238,
#   `RenderObject.cpp:1164-1166`) is not ported.
# * `PF_SPIDERWEB` (405) -> `EF_BOTTOM_SPIDER` (404): `case UNT_SPIDERWEB:`
#   (`MsgEffect.cpp:727-735`, launch `:729`).
# * `NJ_TATAMIGAESHI` (527) -> `EF_TATAMI` (631) and `NJ_KAENSIN` (535) ->
#   `EF_KAEN` (634): `case UNT_TATAMIGAESHI:` / `case UNT_KAEN:`
#   (`MsgEffect.cpp:1017-1025` / `:1027-1033`). Both skills already have a
#   `Ground` descriptor row (the ro-next lane the official ground/unit launch
#   replaces).
#
# Damage lane:
#
# * `MO_CHAINCOMBO` (272) -> `EF_CHAINCOMBO` (273) on `Hit`:
#   `CGameActor::ProcessSkillMessage` `case MO_CHAINCOMBO:` launches it on the
#   *target* (`RenderObject.cpp:5829-5833`, `LaunchEffect` at `:5833`), next to
#   the generic `EF_TEIHIT1` (262). The caster's `EF_GUMGANG3` (263) is not
#   ported.
#
# Skills whose 2019 ids carry no skill-specific official launch stay unmapped:
# `SM_BASH` (5), `MG_NAPALMBEAT` (11), `AC_DOUBLE` (46), `AC_SHOWER` (47) and
# `AC_CHARGEARROW` (148) only name the generic `EF_HIT1`/`EF_HIT2`/`EF_BASH`
# hit ids, `MG_SOULSTRIKE` (13) and `HT_PHANTASMIC` (1009) are launched from the
# Lua `skilleffectinfolist` table (`LaunchLuaSkillEffect`,
# `JobInheritance.cpp:1886-1927`) with the id coming from script data, and
# `CH_SOULCOLLECT` (401) is already wired through `status_effect_launches.ron`
# (`GameModePackets.cpp:12885`), so a second mapping would draw it twice.
#
# A row is an extra (skill, phase) pair: it never replaces the coverage row of
# the same skill, and a collision with an already emitted pair is a hard error.
# The official Spirit-of-the-Soul-Linker switch launches **two** effects on the
# target: `EF_SOULLINK` (the big orbit/whitespark animation) and
# `EF_ASURABODY` (a body glow). The coverage data lists both under the skill's
# `effectId` field, and the first one it sees is `EF_ASURABODY`, which left the
# visible signature animation out. These skills take `EF_SOULLINK` as their
# primary mapping; the second body is spawned by
# `client/effects.rs::secondary_ported_effect`.
SOULLINK_SKILLS = {445, 447, 448, 449, 450, 451, 452, 453, 454, 455, 456, 457, 458, 460, 461}

# Rows whose skill is absent from the 2019 coverage data entirely. Unlike
# `CURATED_EXTRA_EFFECTS`, these are emitted without a coverage-backed test
# name; they keep the renewal/NPC shockwave effects reachable.
MANUAL_MAPPINGS: list[tuple[int, str, str, str]] = [
    (2017, "Gumgang3", "Success", "GameModePackets.cpp:19890-19901 (RK_STORMBLAST target->LaunchEffect(EF_GUMGANG3))"),
    (735, "Gumgang5", "Success", "GameModePackets.cpp:19508-19513 (NPC_MANDRAGORA source->LaunchEffect(EF_GUMGANG5))"),
    (490, "Aciddemon", "Success", "RagEffect.cpp:21396-21401 (EF_ACIDDEMON)"),
    (2476, "Chemical2dash2", "Success", "RagEffect.cpp:19847-19861 (EF_CHEMICAL2DASH2)"),
    (2477, "Chemical2dash", "Success", "RagEffect.cpp:21489-21505 (EF_CHEMICAL2DASH)"),
    (2217, "TetraGround", "Success", "RagEffect.cpp:20190-20202 (EF_TETRA_GROUND)"),
    (659, "Dragonfear", "Success", "RagEffect.cpp:21940-21948 (EF_DRAGONFEAR)"),
    (83, "Meteorstorm", "Success", "RagEffect.cpp:24950-24960 (EF_METEORSTORM)"),
    (85, "Lord", "Success", "RagEffect.cpp:24935-24945 (EF_LORD)"),
    (484, "Gravitation", "Success", "RagEffect.cpp:14550-14560 (EF_GRAVITATION)"),
    (80, "Firepillarbomb", "Success", "RagEffect.cpp:24977-25010 (EF_FIREPILLARBOMB)"),
    (2267, "SuiExplosion", "Success", "RagEffect.cpp:25178-25190 (EF_SUI_EXPLOSION)"),
]

CURATED_EXTRA_EFFECTS: dict[int, tuple[str, str, str]] = {
    383: ("Beginspell6", "Cast", "SkillActionInfo.cpp:165-178 -> GameModePackets.cpp:12947-12953/:17551-17559"),
    70: ("BottomSanc", "Ground", "MsgEffect.cpp:600-608 (CSkill::OnProcess UNT_SANCTUARY, LaunchEffect at :602)"),
    79: ("BottomMag", "Ground", "MsgEffect.cpp:816-824 (CSkill::OnProcess UNT_MAGNUS, LaunchEffect at :818)"),
    87: ("Icewall", "Ground", "RenderObject.cpp:1117-1121 (PlayGroundSkillEffect WZ_ICEWALL, LaunchDummyEffect 0x4A=74 at :1119)"),
    89: ("Stormgust", "Ground", "RenderObject.cpp:1133-1141 (PlayGroundSkillEffect WZ_STORMGUST, LaunchDummyEffect 0x59=89 at :1139)"),
    91: ("Heavensdrive", "Ground", "RenderObject.cpp:1059-1063 (PlayGroundSkillEffect WZ_HEAVENDRIVE, LaunchDummyEffect 0x8E=142 at :1061)"),
    272: ("Chaincombo", "Hit", "RenderObject.cpp:5829-5833 (ProcessSkillMessage MO_CHAINCOMBO, LaunchEffect EF_CHAINCOMBO at :5833)"),
    288: ("BottomLa", "Ground", "MsgEffect.cpp:687-695 (CSkill::OnProcess UNT_LANDPROTECTOR, LaunchEffect at :689)"),
    405: ("BottomSpider", "Ground", "MsgEffect.cpp:727-735 (CSkill::OnProcess UNT_SPIDERWEB, LaunchEffect at :729)"),
    527: ("Tatami", "Ground", "MsgEffect.cpp:1017-1025 (CSkill::OnProcess UNT_TATAMIGAESHI, LaunchEffect at :1019)"),
    535: ("Kaen", "Ground", "MsgEffect.cpp:1027-1033 (CSkill::OnProcess UNT_KAEN, LaunchEffect at :1030)"),
}


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
        if skill in CURATED_SKILL_EFFECTS:
            # Curated above; the coverage/2019 fallbacks would otherwise pick a
            # wrong id (AL_RUWACH's only 2019 id is the generic hit effect).
            continue
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

        if skill in TARGETED_CAST_SKILLS:
            anchor = "Target"

        if skill in SOULLINK_SKILLS:
            # `GameModePackets.cpp:19425-19439` (`SL_SOULLINKER` and the other
            # `SL_*` cases): `EF_SOULLINK` on the target, then
            # `EF_ASURABODY` on the same target. The signature animation is
            # `EF_SOULLINK`; the coverage data's first id is the body glow.
            effect_name = "Soullink"
            phase = "Success"
            anchor = "Target"

        rows.append((skill, phase, name_to_id[effect_name], effect_name, anchor))

    for skill, (name, phase) in sorted(CURATED_SKILL_EFFECTS.items()):
        if name not in ported_set:
            raise SystemExit(f"curated skill {skill} maps to the unported effect {name}")
        anchor = "Target" if phase in TARGET_PHASES else "Caster"
        rows.append((skill, phase, name_to_id[name], name, anchor))

    for skill, (name, phase, citation) in sorted(CURATED_EXTRA_EFFECTS.items()):
        if name not in ported_set:
            raise SystemExit(f"curated extra skill {skill} maps to the unported effect {name}")
        if any(row[0] == skill and row[1] == phase for row in rows):
            raise SystemExit(f"curated extra {skill}/{phase} duplicates a coverage row")
        anchor = "Target" if phase in TARGET_PHASES else "Caster"
        rows.append((skill, phase, name_to_id[name], name, anchor))

    for skill, name, phase, _citation in MANUAL_MAPPINGS:
        if name not in ported_set:
            raise SystemExit(f"manual mapping {skill} maps to the unported effect {name}")
        if any(row[0] == skill and row[1] == phase for row in rows):
            raise SystemExit(f"manual mapping {skill}/{phase} duplicates an existing row")
        anchor = "Target" if phase in TARGET_PHASES else "Caster"
        rows.append((skill, phase, name_to_id[name], name, anchor))

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
    lines.append("        // `SM_BASH` (5) keeps its `Hit` descriptor row; the 2019 table only")
    lines.append("        // names the generic `EF_HIT1`/`EF_HIT2`, which are not the skill's")
    lines.append("        // own launch, so no phase of it is mapped.")
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
    lines.append("    fn detect_maps_to_its_ported_ground_aura() {")
    lines.append("        // `HT_DETECTING` (130) is `TargetType: Ground`, so the arriving lane")
    lines.append("        // is `ZC_NOTIFY_GROUNDSKILL` and the official")
    lines.append("        // `CGameActor::PlayGroundSkillEffect` launches `EF_DETECTING` (119)")
    lines.append("        // at the clicked cell (`RenderObject.cpp:1096-1103`). Its body is")
    lines.append("        // `TexturedSkillAura(0)` (`RagEffect.cpp:23953-23954` -> `:15281-15359`),")
    lines.append("        // registered as `EffectId::Detecting`. The 2019 SkillEffect table has")
    lines.append("        // no descriptor field for it, so the mapping is curated in")
    lines.append("        // `gen_ported_skill_map.py` instead of coming from the coverage data.")
    lines.append("        assert_eq!(")
    lines.append("            ported_effect_for(SkillId(130), SkillPhase::Ground).map(|effect| effect as u32),")
    lines.append("            Some(EffectId::Detecting as u32)")
    lines.append("        );")
    lines.append("        assert_eq!(ported_anchor_for(SkillId(130), SkillPhase::Ground), PortedAnchor::Target);")
    lines.append("    }")
    lines.append("")
    # One emitted test per curated extra lane: the citation is part of the row,
    # so the generated file records where each hand-picked id came from.
    for skill, (effect_name, phase, citation) in sorted(CURATED_EXTRA_EFFECTS.items()):
        skill_name = coverage[str(skill)]["name"]
        anchor = "Target" if phase in TARGET_PHASES else "Caster"
        test_name = f"curated_extra_{skill_name.lower()}_plays_{effect_name.lower()}_on_{phase.lower()}"
        lines.append("    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`")
        lines.append("    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.")
        lines.append("    ///")
        # rustfmt (`wrap_comments`, `comment_width = 80`) would re-wrap a long
        # citation line, so emit it already wrapped and stay `rustfmt --check`
        # clean without a second formatting pass.
        for citation_line in textwrap.wrap(f"Official launch: {citation}", width=68):
            lines.append(f"    /// {citation_line}")
        lines.append("    #[test]")
        lines.append(f"    fn {test_name}() {{")
        lines.append("        assert_eq!(")
        lines.append(f"            ported_effect_for(SkillId({skill}), SkillPhase::{phase}).map(|effect| effect as u32),")
        lines.append(f"            Some(EffectId::{effect_name} as u32)")
        lines.append("        );")
        lines.append(f"        assert_eq!(ported_anchor_for(SkillId({skill}), SkillPhase::{phase}), PortedAnchor::{anchor});")
        lines.append("    }")
        lines.append("")

    lines.append("    #[test]")
    lines.append("    fn wind_walk_success_lane_stays_coverage_driven() {")
    lines.append("        // The second Wind Walk lane is not curated: `EF_PORTAL4` comes from")
    lines.append("        // the coverage data (`successEffectId` -> `Success`,")
    lines.append("        // `GameModePackets.cpp:19098`), so it is pinned by the coverage row.")
    lines.append("        assert_eq!(")
    lines.append("            ported_effect_for(SkillId(383), SkillPhase::Success).map(|effect| effect as u32),")
    lines.append("            Some(EffectId::Portal4 as u32)")
    lines.append("        );")
    lines.append("        assert_eq!(ported_anchor_for(SkillId(383), SkillPhase::Success), PortedAnchor::Caster);")
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
