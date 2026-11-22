#!/usr/bin/env python3
"""Read-only audit of every learnable pre-renewal skill against the official
client effect/audio facts and the current Korangar client mapping.

Writes the four deliverables under notes/skill-audit/:
  skill_matrix.json, summary.md, missing-assets.txt, recommended-data-schema.md

Nothing in the client crate or archive data is modified.
"""

from __future__ import annotations

import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import audit_core as core

OUT = core.NOTES

#: Hand-written visuals (skill_cast_effect.rs). `assets` are the asset paths the
#: hand-written code actually spawns (relative to the game data root).
CUSTOM_VISUALS = {
    27: {"summary": "Warp Portal: cast circle only; the portal is a skill unit", "assets": []},
    28: {
        "summary": "Heal: procedural ring_white target cylinders in addition to the data rows",
        "assets": [r"data\texture\effect\ring_white.tga"],
        "suppressed": False,
    },
    29: {
        "summary": "Increase AGI: hand-written EFST blessing-of-souls sparkle replaces data rows",
        "assets": [r"data\texture\effect\efst_c_blessings_of_soul\blessingsofsoul.str"],
        "overhead": True,
        "scale": 0.9,
        "suppressed": True,
    },
    33: {"summary": "Angelus: hand-written angelus.str overhead", "assets": [r"data\texture\effect\angelus.str"], "overhead": True, "scale": 1.0, "suppressed": True},
    34: {
        "summary": "Blessing: hand-written angel_fluttering.str overhead (not the official SPR/particle/pok2 layers)",
        "assets": [r"data\texture\effect\efst_angel_fluttering\angel_fluttering.str"],
        "overhead": True,
        "scale": 1.0,
        "suppressed": True,
    },
    45: {"summary": "Improve Concentration: hand-written concentration.str overhead", "assets": [r"data\texture\effect\concentration.str"], "overhead": True, "scale": 0.9, "suppressed": True},
    110: {
        "summary": "Hammer Fall: hand-written black_hammerfall.str at the feet (official layer is crashearth.str)",
        "assets": [r"data\texture\effect\black_hammerfall.str"],
        "overhead": False,
        "scale": 1.2,
        "suppressed": True,
    },
    270: {
        "summary": "Fury: procedural BEGIN_SPELL ring + four expanding ring_yellow rings",
        "assets": [r"data\texture\effect\ring_yellow.tga"],
        "suppressed": True,
    },
    401: {
        "summary": "Zen: procedural BEGIN_SPELL ring + yellow body flash",
        "assets": [r"data\texture\effect\ring_yellow.tga"],
        "suppressed": True,
    },
}

#: EFST ids the status-aura registry implements (status_aura.rs:130-135).
STATUS_AURA_EFST = {110: "Assumptio", 473: "Assumptio (party/cash)"}


def load_efst_ids() -> dict[str, int]:
    """Parse the rAthena `enum efst_type` so `EFST_*` names get numeric ids."""
    path = core.RATHENA / "src/map/status.hpp"
    text = path.read_text(encoding="utf-8", errors="replace")
    start = text.index("enum efst_type")
    body = text[text.index("{", start) + 1 : text.index("};", start)]
    body = re.sub(r"//[^\n]*", "", body)
    values: dict[str, int] = {}
    current = -1
    for token in body.split(","):
        token = token.strip()
        if not token:
            continue
        if "=" in token:
            name, value = token.split("=", 1)
            name = name.strip()
            value = value.strip()
            try:
                current = int(value, 0)
            except ValueError:
                continue
        else:
            name = token
            current += 1
        values[name] = current
    return values


def match_layer(layer: dict, current_assets: list[dict]) -> dict | None:
    """Return the current client row that renders the official layer, if any."""
    if not layer.get("asset"):
        return None
    for row in current_assets:
        if core.asset_matches(layer["asset"], row["asset"], layer.get("template", False)):
            return row
    return None


def main() -> int:
    skills, status_icon = core.load_rathena()
    skill_effects, sounds, sprite_effects, effect_table = core.load_official()
    procedural = json.loads((core.ROOT / "tools/windows_client_procedural_effects.json").read_text())["effects"]
    actions = json.loads((core.ROOT / "tools/windows_client_skill_actions.json").read_text())
    client = core.load_client()
    index = core.AssetIndex()
    efst_ids = load_efst_ids()
    raw_effects = core.parse_raw_skill_effect(core.RAW_SKILL_EFFECT_JS)
    aegis_to_raw = {}
    for skill_id, skill in skills.items():
        if skill["aegis"] in raw_effects:
            aegis_to_raw[skill_id] = raw_effects[skill["aegis"]]

    current_sounds = {row["skill"]: row.get("files", []) for row in client["sounds"]}
    current_actions = {row["skill"]: row.get("action") for row in client["actions"]}

    entries = []
    for skill_id in sorted(skills):
        skill = skills[skill_id]
        layers = core.official_layers_for_skill(skill_id, skill_effects, effect_table, index, procedural)
        official_sounds = core.official_sounds_for_skill(skill_id, sounds)

        # --- official summary -------------------------------------------------
        effect_ids: dict[str, list[int]] = {}
        for field in skill_effects["skills"].get(str(skill_id), []):
            effect_ids[field["field"]] = field["effects"]
        visual_layers = [layer for layer in layers if layer["kind"] in ("STR", "SPR", "TEXTURE")]
        hardcoded_layers = [layer for layer in layers if layer["kind"] in ("FUNC", "PRIM", "RSM", "HARDCODED")]
        coverable = [layer for layer in visual_layers if layer.get("asset") and layer.get("asset_exists")]
        missing_asset_layers = [layer for layer in visual_layers if not (layer.get("asset") and layer.get("asset_exists"))]

        # Facts the extracted table drops (`firstHitEffectId`, effect id 0, ...).
        raw_fields = aegis_to_raw.get(skill_id, {})
        extractor_gaps = []
        for field_name, ids in raw_fields.items():
            extracted = effect_ids.get(field_name)
            if extracted is None or sorted(extracted) != sorted(ids):
                extractor_gaps.append({"field": field_name, "effect_ids": ids, "extracted": extracted})

        # Official sound facts are derived from the effect layers themselves:
        # `windows_client_skill_sounds.json` is the same RE dump but its
        # block-splitting regex merges the symbolic `ef_*` blocks that follow a
        # numeric effect (see summary.md, `extract_*` data-quality note).
        official_sound_rows = []
        seen_sound_stems = set()
        for layer in layers:
            wav = layer.get("wav")
            if not wav:
                continue
            stem = core.decode_game_name(wav).replace("/", "\\").rsplit("\\", 1)[-1].lower()
            if stem in seen_sound_stems:
                continue
            seen_sound_stems.add(stem)
            official_sound_rows.append({
                "field": layer.get("field"),
                "effect_id": layer["effect_id"],
                "wav": core.decode_game_name(wav),
                "wav_raw": wav,
                "asset": layer.get("wav_asset"),
                "asset_exists": layer.get("wav_asset_exists"),
                "template": layer.get("wav_template", False),
            })

        # --- current client handling -----------------------------------------
        data_assets = core.current_assets_for_skill(skill_id, client, index)
        custom = CUSTOM_VISUALS.get(skill_id)
        suppressed = bool(custom and custom.get("suppressed"))
        custom_assets = []
        if custom:
            for path in custom.get("assets", []):
                custom_assets.append({
                    "asset": path,
                    "exists": index.exists(path),
                    "variant": None,
                    "source": "hand-written skill_visuals",
                })

        if suppressed:
            effective_assets = custom_assets
        else:
            effective_assets = data_assets["str"] + data_assets["spr"] + data_assets["proc"] + custom_assets

        # De-duplicate by asset path for coverage (a duplicated official layer is
        # still covered by one current row).
        covered, missing, phase_mismatch = [], [], []
        for layer in coverable:
            row = match_layer(layer, effective_assets)
            if row is None:
                # Try the suppressed data rows to explain "row exists but is skipped".
                suppressed_row = match_layer(layer, data_assets["str"] + data_assets["spr"] + data_assets["proc"]) if suppressed else None
                missing.append({
                    "kind": layer["kind"],
                    "effect_id": layer["effect_id"],
                    "field": layer.get("field"),
                    "variant": layer.get("variant"),
                    "anchor": layer.get("anchor"),
                    "file": layer.get("file_decoded", layer.get("file")),
                    "file_raw": layer.get("file"),
                    "asset": layer.get("asset"),
                    "head": layer.get("head"),
                    "attached_entity": layer.get("attached_entity"),
                    "duration_ms": layer.get("duration_ms"),
                    "delay_ms": layer.get("delay_ms"),
                    "data_row_exists_but_suppressed": bool(suppressed_row),
                })
                continue
            covered.append({
                "kind": layer["kind"],
                "effect_id": layer["effect_id"],
                "field": layer.get("field"),
                "variant": layer.get("variant"),
                "anchor": layer.get("anchor"),
                "file": layer.get("file"),
                "asset": layer.get("asset"),
                "current_variant": row.get("variant"),
                "current_asset": row.get("asset"),
                "current_source": row.get("source", "data row"),
                "phase_matches": row.get("source") == "hand-written skill_visuals"
                or row.get("variant") == layer.get("variant"),
            })
            if row.get("source") != "hand-written skill_visuals" and row.get("variant") != layer.get("variant"):
                phase_mismatch.append({
                    "asset": layer.get("asset"),
                    "official_phase": layer.get("variant"),
                    "current_phase": row.get("variant"),
                })

        # Wrong-placeholder detection: a hand-written/custom asset that does not
        # equal any official asset for the skill.
        official_assets = {layer.get("asset") for layer in visual_layers if layer.get("asset")}
        wrong_assets = []
        for asset in custom_assets:
            if asset["asset"].lower() not in {a.lower() for a in official_assets if a}:
                wrong_assets.append(asset["asset"])

        coverage_basis = ""
        if not layers and not raw_fields:
            status = "NO_OFFICIAL_EFFECT"
            coverage_basis = "no entry in the official skill effect table"
        elif not visual_layers:
            # Only hard-coded / sound effects: there is no official asset layer
            # to audit. The hard-coded hints are kept in official.hardcoded_layers.
            status = "NO_OFFICIAL_EFFECT"
            coverage_basis = "official effect has no asset layer (hard-coded code effect and/or sound only)"
        elif len(covered) == len(coverable) and not missing_asset_layers:
            status = "COMPLETE"
            coverage_basis = "every official asset layer is rendered"
        elif covered:
            status = "PARTIAL"
            coverage_basis = "some official asset layers are missing or use a different asset"
        elif missing_asset_layers and not coverable:
            status = "PARTIAL"
            coverage_basis = "every official asset is absent from the GRFs (see missing-assets.txt)"
        elif wrong_assets:
            status = "WRONG_ASSET"
            coverage_basis = "the hand-written/placeholder asset is not the official one"
        else:
            status = "MISSING"
            coverage_basis = "no client visual matches the official asset layers"



        # --- sound coverage ---------------------------------------------------
        official_stems = []
        for row in official_sound_rows:
            stem = row["asset"].rsplit("\\", 1)[-1].rsplit(".", 1)[0].lower()
            if stem not in official_stems:
                official_stems.append(stem)
        current_stems = [f.replace("/", "\\").rsplit("\\", 1)[-1].rsplit(".", 1)[0].lower() for f in current_sounds.get(skill_id, [])]

        def sound_covered(stem: str) -> bool:
            if "%" in stem:
                regex = re.compile("^" + re.escape(stem).replace(r"%d", r"\d+").replace(r"%s", r".+") + "$")
                return any(regex.match(current) for current in current_stems)
            return any(stem == current or current.startswith(stem) for current in current_stems)

        if not official_stems:
            sound_status = "NO_OFFICIAL"
        elif all(sound_covered(stem) for stem in official_stems):
            sound_status = "COVERED"
        elif any(sound_covered(stem) for stem in official_stems):
            sound_status = "PARTIAL"
        else:
            sound_status = "MISSING"

        # --- caster action ----------------------------------------------------
        official_action = actions["actions"].get(str(skill_id), {}).get("action", actions["defaults"]["DEFAULT"])
        current_action = current_actions.get(skill_id, "Skill")
        action_status = "MATCH" if official_action == current_action else "MISMATCH"

        # --- status aura ------------------------------------------------------
        aura = None
        if skill.get("status_icon"):
            efst_id = efst_ids.get(skill["status_icon"])
            aura = {
                "efst": skill["status_icon"],
                "efst_id": efst_id,
                "implemented": efst_id in STATUS_AURA_EFST if efst_id is not None else False,
            }

        entries.append({
            "id": skill_id,
            "aegis": skill["aegis"],
            "name": skill["name"],
            "max_level": skill["max_level"],
            "type": skill["type"],
            "target_type": skill["target_type"],
            "element": skill["element"],
            "cast_time_ms": skill["cast_time_ms"],
            "after_cast_act_delay_ms": skill["after_cast_act_delay_ms"],
            "duration_ms": skill["duration_ms"],
            "cooldown_ms": skill["cooldown_ms"],
            "status_effect": skill.get("status"),
            "job_families": skill["families"],
            "jobs": skill["jobs"],
            "official": {
                "effect_ids": effect_ids,
                "layers": [
                    {
                        "kind": layer["kind"],
                        "effect_id": layer["effect_id"],
                        "field": layer.get("field"),
                        "phase": layer.get("variant"),
                        "anchor": layer.get("anchor"),
                        "file": layer.get("file_decoded", layer.get("file")),
                        "file_raw": layer.get("file"),
                        "asset": layer.get("asset"),
                        "asset_exists": layer.get("asset_exists"),
                        "type_raw": layer.get("type_raw"),
                        "head": layer.get("head"),
                        "attached_entity": layer.get("attached_entity"),
                        "duration_ms": layer.get("duration_ms"),
                        "delay_ms": layer.get("delay_ms"),
                        "y_offset": layer.get("y_offset"),
                        "duplicate": layer.get("duplicate"),
                        "wav": core.decode_game_name(layer.get("wav")) if layer.get("wav") else None,
                        "wav_raw": layer.get("wav"),
                        "wav_asset": layer.get("wav_asset"),
                        "func": layer.get("func"),
                        "source": layer.get("source"),
                    }
                    for layer in layers
                ],
                "sound_wavs": official_sound_rows,
                "caster_action": official_action,
                "sounds_extracted": [
                    {"field": row.get("field"), "effect_id": row.get("effect_id"), "wav": row.get("wav")}
                    for row in official_sounds
                ],
                "sounds_extracted_disagrees": sorted({row.get("wav") for row in official_sounds})
                != sorted({row.get("wav") for row in official_sound_rows}),
                "raw_skill_effect": raw_fields,
                "extractor_gaps": extractor_gaps,
                "hardcoded_layers": len(hardcoded_layers),
            },
            "current_client": {
                "custom_visual": custom,
                "data_rows": {
                    "str": data_assets["str"],
                    "spr": data_assets["spr"],
                    "procedural": data_assets["proc"],
                },
                "suppressed_by_custom_completion": suppressed,
                "effective_assets": effective_assets,
                "sounds": current_sounds.get(skill_id, []),
                "sound_status": sound_status,
                "action": current_action,
                "action_status": action_status,
                "status_aura": aura,
                "status_aura_implemented": bool(aura and aura["implemented"]),
            },
            "coverage": {
                "status": status,
                "basis": coverage_basis,
                "official_visual_layers": len(visual_layers),
                "coverable_layers": len(coverable),
                "covered_layers": len(covered),
                "covered_layer_details": covered,
                "missing_layers": missing,
                "missing_asset_layers": missing_asset_layers,
                "phase_mismatches": phase_mismatch,
                "wrong_assets": wrong_assets,
            },
        })

    # Derived fields: unique missing assets and a fix priority.
    status_weight = {"WRONG_ASSET": 100, "MISSING": 90, "PARTIAL": 60, "COMPLETE": 0, "NO_OFFICIAL_EFFECT": 0}
    core_families = {"Acolyte", "Mage", "Swordman", "Archer", "Merchant", "Thief"}
    for entry in entries:
        coverage = entry["coverage"]
        unique = {}
        for layer in coverage["missing_layers"] + coverage["missing_asset_layers"]:
            unique[(layer["kind"], layer.get("asset") or layer.get("file"))] = layer
        coverage["missing_assets_unique"] = [
            {"kind": key[0], "asset": key[1], "effect_ids": sorted({l["effect_id"] for l in unique.values() if (l["kind"], l.get("asset") or l.get("file")) == key})}
            for key in unique
        ]
        weight = status_weight[coverage["status"]]
        if weight:
            weight += 15 if set(entry["job_families"]) & core_families else 5
            weight += min(20, 2 * len(coverage["missing_layers"]))
        entry["priority"] = weight

    payload = {
        "provenance": {
            "skill_db": str(core.RATHENA / "db/pre-re/skill_db.yml"),
            "skill_tree": str(core.RATHENA / "db/pre-re/skill_tree.yml"),
            "status_db": str(core.RATHENA / "db/pre-re/status.yml"),
            "official_effects": str(core.ROOT / "tools/windows_client_skill_effects.json"),
            "official_effect_layers": str(core.EFFECT_TABLE_JS),
            "official_effect_layers_fallback": str(core.EFFECT_TABLE_JS_FALLBACK),
            "official_skill_effect_js": str(core.RAW_SKILL_EFFECT_JS),
            "official_sounds": str(core.ROOT / "tools/windows_client_skill_sounds.json"),
            "official_actions": str(core.ROOT / "tools/windows_client_skill_actions.json"),
            "official_sprite_effects": str(core.ROOT / "tools/windows_client_sprite_effects.json"),
            "client_mapping": [
                "korangar/archive/data/client/skill_effect_assets.ron",
                "korangar/archive/data/client/skill_sprite_effects.ron",
                "korangar/archive/data/client/skill_procedural_effects.ron",
                "korangar/archive/data/client/skill_sounds.ron",
                "korangar/archive/data/client/skill_actions.ron",
                "korangar/src/world/skill_cast_effect.rs",
                "korangar/src/world/effect/status_aura.rs",
            ],
            "grf_archives": [str(core.GAME_DIR / archive) for archive in core.ARCHIVES],
            "rathena_path_note": "The task named /home/phantom/rathena; that path does not exist on this machine. The two rAthena 2019 checkouts (popeng-llm-cpp/condaro-llm and popeng-hotspots/condaro) are byte-identical (md5 f577daa67fe495bd0a3bf5d55dc86d6f for db/pre-re/skill_db.yml); the first is used.",
            "client_head": "e87268af",
        },
        "skills": entries,
    }
    (OUT / "skill_matrix.json").write_text(json.dumps(payload, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"skill_matrix.json: {len(entries)} learnable skills")
    print(Counter(entry["coverage"]["status"] for entry in entries))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
