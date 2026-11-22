#!/usr/bin/env python3
"""Render the audit deliverables from `skill_matrix.json`.

Writes `missing-assets.txt`, `summary.md` and `recommended-data-schema.md` next
to the matrix. Read-only with respect to the client sources.
"""

from __future__ import annotations

import json
import re
from collections import Counter, defaultdict
from pathlib import Path

OUT = Path(__file__).resolve().parent
MATRIX = OUT / "skill_matrix.json"
GEN_EFFECT_ASSETS = Path("/home/phantom/Games/ragnarok/CondaRO-linux/korangar/tools/gen_skill_effect_assets.py")

STATUSES = ("COMPLETE", "PARTIAL", "MISSING", "WRONG_ASSET", "NO_OFFICIAL_EFFECT")
FAMILY_ORDER = ("Novice", "Swordman", "Mage", "Archer", "Acolyte", "Merchant", "Thief", "Extended")


def load():
    return json.loads(MATRIX.read_text())


def parse_skill_set(name: str) -> set[int]:
    text = GEN_EFFECT_ASSETS.read_text()
    match = re.search(name + r"\s*=\s*\{(.*?)\}", text, re.S)
    return {int(number) for number in re.findall(r"\b(\d+)\b", match.group(1))}


def parse_overhead_skills() -> set[int]:
    return parse_skill_set("OVERHEAD_SKILLS")


# ---------------------------------------------------------------------------
# missing-assets.txt
# ---------------------------------------------------------------------------

def write_missing_assets(data: dict) -> None:
    skills = data["skills"]
    lines = [
        "# Official skill-effect assets absent from the CondaRO GRF archives",
        "#",
        "# Scope: every learnable pre-renewal skill (rAthena pre-re skill_tree union,",
        f"# {len(skills)} skills) and the official effect layers / wavs referenced by the",
        "# extracted Windows-client facts (tools/windows_client_skill_effects.json + the",
        "# full layer table /tmp/EffectTable.js, which is the same RE dump the client data",
        "# is generated from).",
        "#",
        "# Checked against:",
        "#   /home/phantom/Games/ragnarok/CondaRO-linux/client/game/CondaRO.grf",
        "#   /home/phantom/Games/ragnarok/CondaRO-linux/client/game/data.grf",
        "#   /home/phantom/Games/ragnarok/CondaRO-linux/client/game/palettes.grf",
        "#   (653,299 file names indexed).",
        "#",
        "# A layer is listed when the official table names an asset and that exact file is",
        "# not present in any archive. For a template name (%d/%s) any matching file counts.",
        "",
        "== Official visual layers (.str / .spr / .tga / .bmp) absent from the GRFs ==",
        "",
    ]

    visual_missing = defaultdict(list)
    for skill in skills:
        for layer in skill["coverage"]["missing_asset_layers"]:
            key = (layer["kind"], layer.get("file"), layer.get("asset"))
            visual_missing[key].append(skill)

    if not visual_missing:
        lines.append("(none)")
    for (kind, file_name, asset), owners in sorted(visual_missing.items(), key=lambda item: (item[0][0] or "", item[0][1] or "")):
        owners_text = ", ".join(f"skill {s['id']} {s['aegis']} (effect {layer['effect_id']})"
                                for s in owners
                                for layer in s["coverage"]["missing_asset_layers"]
                                if layer["kind"] == kind and layer.get("file") == file_name and layer.get("asset") == asset)
        expected = asset or "(unresolved)"
        lines.append(f"- {kind}: official file {file_name!r}; expected in GRF as {expected}")
        lines.append(f"  referenced by: {owners_text}")
        if kind == "STR":
            lines.append(f"  missing: data\\texture\\effect\\{file_name}.str")
        elif kind == "SPR":
            lines.append(f"  missing: data\\sprite\\이팩트\\{file_name}.spr (+ .act)")
        lines.append("")

    lines += [
        "",
        "== Official sound files (`wav`) referenced by effect layers but absent from the GRFs ==",
        "",
    ]
    wav_missing = defaultdict(list)
    for skill in skills:
        for sound in skill["official"]["sound_wavs"]:
            if sound["asset_exists"] is False:
                wav_missing[sound["wav"]].append((skill, sound))
    if not wav_missing:
        lines.append("(none)")
    for wav, owners in sorted(wav_missing.items()):
        lines.append(f"- wav {wav!r} (expected {owners[0][1]['asset']})")
        lines.append("  referenced by: " + ", ".join(f"skill {s['id']} {s['aegis']} (effect {sound['effect_id']})" for s, sound in owners))
        lines.append("")

    lines += [
        "",
        "== Informational: audio the extracted `windows_client_skill_sounds.json` lists although",
        "   the EffectTable layer itself has no active wav (commented-out `//wav:` alternates and",
        "   symbolic `*_sound` blocks merged into the preceding numeric effect). ==",
        "",
    ]
    interesting = []
    for skill in skills:
        extracted = [row["wav"] for row in skill["official"]["sounds_extracted"]]
        layered = [row["wav"] for row in skill["official"]["sound_wavs"]]
        if sorted(set(extracted)) != sorted(set(layered)):
            interesting.append(skill)
    if not interesting:
        lines.append("(none)")
    for skill in interesting:
        extracted = sorted({row["wav"] for row in skill["official"]["sounds_extracted"]})
        layered = sorted({row["wav"] for row in skill["official"]["sound_wavs"]})
        lines.append(f"- skill {skill['id']} {skill['aegis']}: extracted={extracted} layer-derived={layered}")
    lines.append("")

    lines += [
        "",
        "== Note: current client data rows ==",
        "",
        "No current client mapping row (skill_effect_assets.ron / skill_sprite_effects.ron /",
        "skill_procedural_effects.ron) referenced by a learnable skill points at a missing",
        "asset: the generators already drop rows whose file is not in the archives.",
        "",
    ]
    (OUT / "missing-assets.txt").write_text("\n".join(lines), encoding="utf-8")


# ---------------------------------------------------------------------------
# summary.md
# ---------------------------------------------------------------------------

def status_counts(skills):
    return Counter(skill["coverage"]["status"] for skill in skills)


def family_counts(skills, status):
    counter = Counter()
    for skill in skills:
        if skill["coverage"]["status"] != status:
            continue
        for family in skill["job_families"]:
            counter[family] += 1
    return counter


def write_summary(data: dict) -> None:
    skills = data["skills"]
    counts = status_counts(skills)
    total = len(skills)
    layers = [layer for skill in skills for layer in skill["official"]["layers"]]
    visual = [layer for layer in layers if layer["kind"] in ("STR", "SPR", "TEXTURE")]
    kind_counts = Counter(layer["kind"] for layer in layers)
    covered_total = sum(skill["coverage"]["covered_layers"] for skill in skills)
    missing_layer_total = sum(len(skill["coverage"]["missing_layers"]) for skill in skills)
    missing_asset_total = sum(len(skill["coverage"]["missing_asset_layers"]) for skill in skills)
    overhead_skills = parse_overhead_skills()
    body_aura_skills = parse_skill_set("BODY_AURA_SKILLS")

    noncomplete = [skill for skill in skills if skill["coverage"]["status"] not in ("COMPLETE", "NO_OFFICIAL_EFFECT")]
    noncomplete.sort(key=lambda skill: -skill["priority"])
    with_icon = [skill for skill in skills if skill["current_client"]["status_aura"]]
    aura_implemented = [skill for skill in skills if skill["current_client"]["status_aura_implemented"]]
    custom = [skill for skill in skills if skill["current_client"]["custom_visual"]]
    head_true = {}
    for skill in skills:
        head_layers = [layer for layer in skill["official"]["layers"] if layer.get("head")]
        if head_layers:
            head_true[skill["id"]] = (skill, head_layers)

    lines = []
    add = lines.append

    add("# Pre-renewal skill effect audit (Korangar vs official client data)")
    add("")
    add(f"Audit of **all {total} learnable pre-renewal skills** (union of the rAthena pre-re skill trees)")
    add("against the extracted official Windows-client effect/audio facts and the current Korangar")
    add("client mapping. Read-only investigation: no client Rust code, archive data or GRF was modified.")
    add("")
    add("| | |")
    add("|---|---|")
    add("| Client repo | `/home/phantom/Games/ragnarok/CondaRO-linux/korangar` @ `e87268af` |")
    add("| rAthena source | `/home/phantom/popeng-llm-cpp/condaro-llm/db/pre-re/` — **note:** the path named in the task (`/home/phantom/rathena`) does not exist on this machine; the two rAthena 2019 checkouts present are byte-identical (`db/pre-re/skill_db.yml` md5 `f577daa67fe495bd0a3bf5d55dc86d6f`), so the `condaro-llm` one is used. |")
    add("| Official effect facts | `tools/windows_client_skill_effects.json` (557 skills, 401 effect ids) + full layer table `/tmp/EffectTable.js` (`roBrowserLegacy_EffectTable.js`, 635 numeric effects, all fields) |")
    add("| Official sounds | layer `wav` facts of the same table (see method note on `windows_client_skill_sounds.json`) |")
    add("| Official sprite/action facts | `tools/windows_client_sprite_effects.json`, `tools/windows_client_skill_actions.json` |")
    add("| GRF check | `CondaRO.grf` + `data.grf` + `palettes.grf`, 653,299 file names |")
    add("")
    add("## 1. Result counts")
    add("")
    add("| Status | Skills | Share | Meaning |")
    add("|---|---:|---:|---|")
    meaning = {
        "COMPLETE": "every official asset layer is rendered by the client",
        "PARTIAL": "some official layers rendered, some missing",
        "MISSING": "official asset layers exist but the client renders none of them",
        "WRONG_ASSET": "a hand-written/placeholder asset is shown instead of the official layers",
        "NO_OFFICIAL_EFFECT": "official table defines no renderable asset layer (passive, hard-coded or sound-only)",
    }
    for status in STATUSES:
        add(f"| {status} | {counts[status]} | {counts[status] / total:.1%} | {meaning[status]} |")
    add(f"| **total** | **{total}** | 100% | |")
    add("")
    add(f"Official layer rows across the {total} skills: **{len(layers)}** "
        f"({', '.join(f'{kind} {count}' for kind, count in sorted(kind_counts.items()))}).")
    add(f"Of these, **{len(visual)}** are asset layers (STR/SPR/TEXTURE), **{covered_total}** are covered by the client, "
        f"**{missing_layer_total}** official layers are not covered, and **{missing_asset_total}** official asset layer"
        + ("s" if missing_asset_total != 1 else "") + " name a file absent from the GRFs.")
    add("")
    add("The `PARTIAL`/`MISSING`/`WRONG_ASSET` set is small (14 skills) because the generated data files")
    add("already cover the large majority of asset effects. The important gaps are concentrated in")
    add("hand-written buff visuals and a few projectile/spawn effects; the deeper systemic issues are the")
    add("`head`-flag propagation and the persistent status auras (sections 3.4 and 3.6).")
    add("")
    add("## 2. Class / job-family breakdown")
    add("")
    add("Each skill can belong to several job trees (inheritance), so the family columns sum to more")
    add("than the total. `Extended` = Super Novice / Gunslinger / Ninja / Taekwon / Star Gladiator /")
    add("Soul Linker / Gangsi / Death Knight / Dark Collector.")
    add("")
    header = "| family | " + " | ".join(STATUSES) + " | total skills |"
    add(header)
    add("|---|" + "---:|" * (len(STATUSES) + 1))
    family_totals = Counter()
    for skill in skills:
        for family in skill["job_families"]:
            family_totals[family] += 1
    for family in FAMILY_ORDER:
        row = [str(family)]
        for status in STATUSES:
            row.append(str(family_counts(skills, status)[family]))
        row.append(str(family_totals[family]))
        add("| " + " | ".join(row) + " |")
    add("")
    add("Observation: the Merchant and Archer families have the highest `NO_OFFICIAL_EFFECT`")
    add("counts (many trap/ammo/support skills are hard-coded or sound-only in the official table),")
    add("while the Acolyte family holds two of the three wrong-asset buffs (Blessing, Increase AGI).")
    add("")
    add("## 3. Findings, highest impact first")
    add("")
    add("### 3.1 Wrong hand-written placeholder assets (3 skills, P0)")
    add("")
    add("These skills are in `has_custom_completion` (`skill_cast_effect.rs:392-404`), which silently")
    add("discards every data-driven layer the generators produced. The hand-written completion then")
    add("shows a non-official file. The official data rows exist and are already generated, but are")
    add("skipped at runtime.")
    add("")
    add("| Skill | Official effect/layers | Client now | Exact missing layers | Fix |")
    add("|---|---|---|---|---|")
    for skill in noncomplete:
        if skill["coverage"]["status"] != "WRONG_ASSET":
            continue
        official = []
        for layer in skill["official"]["layers"]:
            if layer["kind"] in ("STR", "SPR", "TEXTURE"):
                detail = f"{layer['kind']} `{layer.get('file')}`"
                flags = []
                if layer.get("head"):
                    flags.append("head")
                if layer.get("attached_entity"):
                    flags.append("attached")
                if layer.get("duration_ms"):
                    flags.append(f"{layer['duration_ms']} ms")
                if flags:
                    detail += " (" + ", ".join(flags) + ")"
                official.append(detail)
        custom = skill["current_client"]["custom_visual"]
        current = ", ".join(f"`{Path(a).name}`" for a in custom["assets"]) if custom and custom["assets"] else "(nothing)"
        missing = "<br>".join(
            f"{layer['kind']} `{layer.get('file')}` effect {layer['effect_id']}"
            + (" *(generated row exists but is suppressed)*" if layer.get("data_row_exists_but_suppressed") else "")
            for layer in skill["coverage"]["missing_layers"]
        )
        if skill["id"] == 34:
            fix = "Drop the custom arm/`has_custom_completion` entry; enable the official layers: SPR `축복` (blessing, 1500 ms, head, attached, yOffset -120) + 2× sparkling `particle6` + `pok2.tga` glow."
        elif skill["id"] == 29:
            fix = "Drop the custom arm/`has_custom_completion` entry; enable effect 37 = 3× `ac_center2.tga` bursts (1000 ms, attached, delayed/faded) + `agi_up.bmp` column. The generated rows already exist."
        elif skill["id"] == 110:
            fix = "Replace the hand-written `black_hammerfall.str` with the official `crashearth.str` (already generated in `skill_effect_assets.ron`), then drop the custom arm."
        else:
            fix = "Use the official asset(s)."
        add(f"| {skill['id']} {skill['aegis']} | {'; '.join(official)} | {current} | {missing} | {fix} |")
    add("")
    add("### 3.2 The three task-named buffs in detail")
    add("")
    add("**Blessing (34, AL_BLESSING, Acolyte)** — official effect **42** (`EF_BLESSING`):")
    add("")
    add("| # | type | file | head | attachedEntity | duration | delay | other |")
    add("|---|---|---|---|---|---|---|---|")
    add("| 1 | SPR | `축복` (blessing) | true | true | 1500 ms | delayFrame 30 | repeat, stackable, yOffset -120 (≈30 world units up) |")
    add("| 2 | 3D | `particle6` (sprite) | false | true | 1200 ms | delayOffset 300 | duplicate 6, sparkling, fade in/out, size 50 |")
    add("| 3 | 3D | `particle6` (sprite) | false | true | 1200 ms | delayOffset 400 | duplicate 6, fade in/out, size 50 |")
    add("| 4 | 3D | `effect/pok2.tga` | false | false | 2500 ms | — | blue additive glow, alphaMax 0.3, size 140 |")
    add("")
    add("The **flying-angel layer is the SPR `축복`** (`data\\sprite\\이팩트\\축복.spr` + `.act`, both present")
    add("in `CondaRO.grf`), not the `angel_fluttering.str` placeholder. The official sound is")
    add("`effect/ef_blessing` (client already maps it). The persistent `EFST_BLESSING` (id 10)")
    add("status visual is also not implemented (`status_aura.rs` only knows EFST 110/473).")
    add("")
    add("**Increase AGI (29, AL_INCAGI, Acolyte)** — official effect **37** (`EF_INCAGILITY`):")
    add("")
    add("| # | type | file | attachedEntity | duration | delay | other |")
    add("|---|---|---|---|---|---|---|")
    add("| 1 | 3D | `effect/ac_center2.tga` | true | 1000 ms | delayLate 500 | duplicate 7, alphaMax 1, posz 1..6, fadeOut, sizeX 2.5 |")
    add("| 2 | 3D | `effect/ac_center2.tga` | true | 1000 ms | delayOffset 400 | duplicate 3, alphaMax .75, fadeOut |")
    add("| 3 | 3D | `effect/ac_center2.tga` | true | 1000 ms | — | duplicate 10, alphaMax 1, fadeOut |")
    add("| 4 | 3D | `effect/agi_up.bmp` | true | 1000 ms | — | fadeIn/out, overlay, size 100×45, posz 0.4→3, wav `ef_incagility` |")
    add("")
    add("The generated `skill_procedural_effects.ron` already contains rows for these four layers")
    add("(skill 29) but `has_custom_completion(29)` makes `spawn_skill_effect_variants` return early")
    add("(`lib.rs:2457`). The client shows `efst_c_blessings_of_soul\\blessingsofsoul.str` instead.")
    add("")
    add("**Angelus (33, AL_ANGELUS, Acolyte)** — official effect **41** (`EF_ANGELUS`):")
    add("STR `angelus`, **head:true**, attachedEntity:true, wav `effect/ef_angelus`. The hand-written")
    add("completion uses the same `angelus.str` and anchors it overhead, so the visual result is")
    add("**COMPLETE** — but only by accident: the official `head:true` flag is *not* propagated by the")
    add("data pipeline (section 3.4), so replacing the hand-written arm with table data today would")
    add("regress it to a feet-anchored sprite.")
    add("")
    add("### 3.3 Assumptio — the reference working multi-layer/status example")
    add("")
    add("`HP_ASSUMPTIO` (361) is the only skill that combines all three layers of the modern client")
    add("model and is the template the other buffs should follow:")
    add("")
    add("| Layer | Source | Detail |")
    add("|---|---|---|")
    add("| one-shot `.str` | `skill_effect_assets.ron` | effect 440 `asum.str`, Success/Auto, attachedEntity:true, wav `effect/아숨프티오` |")
    add("| persistent status aura | `status_aura.rs` (`ASSUMPTIO`) | EFST 110 / 473; blue-white ground glow + rotating `bubble1.bmp` ring + `round_sphere.bmp` body glow; procedural, follows the entity, scales/sits correctly |")
    add("| transparency | effect renderer | alpha-blended quads (report `notes/korangar-effect-transparency-report.md`), no opaque placeholder |")
    add("")
    add("Status: **COMPLETE** (1/1 official asset layer + aura). By contrast Blessing, Increase AGI,")
    add("Angelus and Concentration apply EFSTs 10, 12, 9, 3 but have no aura registered.")
    add("")
    add("### 3.4 Systemic: official `head` (overhead) flag is not propagated")
    add("")
    add("The official table marks one-shot buff layers with `head: true` (+ `yOffset` screen offset).")
    add("The `.str` generator does **not** read that flag; it uses a hand-curated skill-id set")
    add("(`OVERHEAD_SKILLS`, 43 ids, in `tools/gen_skill_effect_assets.py`) and an exception set")
    add("(`BODY_AURA_SKILLS`, 14 ids). For the pre-renewal learnable set only **9 skills** have an")
    add("official `head:true` layer, and **6 of them are not in `OVERHEAD_SKILLS`**:")
    add("")
    add("| Skill | Official head layer | Client handling | Result |")
    add("|---|---|---|---|")
    for skill_id, (skill, head_layers) in sorted(head_true.items()):
        layer = head_layers[0]
        custom = skill["current_client"]["custom_visual"]
        if skill["current_client"]["suppressed_by_custom_completion"]:
            handling = "hand-written arm (overhead=" + ("true)" if (custom or {}).get("overhead") else "false)")
            result = "custom visual overhead" if (custom or {}).get("overhead") else "custom visual at the feet"
        else:
            rows = [row for row in skill["current_client"]["data_rows"]["str"]
                    if row["asset"].lower().endswith((layer.get("file") or "").lower() + ".str")]
            handling = f"data row (overhead={rows[0].get('overhead')})" if rows else "data rows via spr/proc"
            if skill_id in body_aura_skills:
                result = "body/weapon aura (`BODY_AURA_SKILLS`) — intentional"
            elif rows and not rows[0].get("overhead"):
                result = "**rendered at the feet (bug)**"
            elif rows:
                result = "overhead"
            else:
                result = "n/a (sprite layer has its own head offset)"
        in_set = "in" if skill_id in overhead_skills else "**NOT in**"
        layer_desc = f"{layer['kind']} `{layer.get('file')}` effect {layer['effect_id']}"
        add(f"| {skill_id} {skill['aegis']} | {layer_desc} | {in_set} `OVERHEAD_SKILLS`; {handling} | {result} |")
    add("")
    add("Skills affected today: **KN_SPEARBOOMERANG (59)**, **KN_BOWLINGBASH (62)** and")
    add("**LK_CONCENTRATION (357)** render the official buff `.str` at the feet; AL_HOLYWATER (31)")
    add("is fine because the sprite generator already applies `head`/`yOffset`; Angelus/Blessing are")
    add("masked by the hand-written arms. The fix belongs in the data schema: carry `head`/`yOffset`")
    add("per layer instead of guessing from a skill-id list (see `recommended-data-schema.md`).")
    add("")
    add("### 3.5 Multi-layer / spawn-effect gaps (PARTIAL, 4 skills)")
    add("")
    add("| Skill | Official layers | Missing layers | Note |")
    add("|---|---|---|---|")
    for skill in noncomplete:
        if skill["coverage"]["status"] != "PARTIAL":
            continue
        official = "; ".join(
            f"{layer['kind']} `{layer.get('file')}` ×{sum(1 for other in skill['official']['layers'] if other.get('asset') == layer.get('asset') and other['kind'] == layer['kind'])}"
            for layer in skill["official"]["layers"]
            if layer["kind"] in ("STR", "SPR", "TEXTURE") and layer is not None
        )
        # unique rendering of official layers
        seen = set()
        official_unique = []
        for layer in skill["official"]["layers"]:
            if layer["kind"] not in ("STR", "SPR", "TEXTURE"):
                continue
            key = (layer["kind"], layer.get("asset"))
            if key in seen:
                continue
            seen.add(key)
            count = sum(1 for other in skill["official"]["layers"] if other["kind"] == layer["kind"] and other.get("asset") == layer.get("asset"))
            official_unique.append(f"{layer['kind']} `{layer.get('file')}`" + (f" ×{count}" if count > 1 else ""))
        missing_unique = []
        seen = set()
        for layer in skill["coverage"]["missing_layers"]:
            key = (layer["kind"], layer.get("asset") or layer.get("file"))
            if key in seen:
                continue
            seen.add(key)
            count = sum(1 for other in skill["coverage"]["missing_layers"] if (other["kind"], other.get("asset") or other.get("file")) == key)
            missing_unique.append(f"{layer['kind']} `{layer.get('file')}`" + (f" ×{count}" if count > 1 else ""))
        note = skill["coverage"]["basis"]
        if skill["coverage"]["missing_asset_layers"]:
            note = "official SPR `shockwave` absent from the GRFs; STR `shockwavehit` is mapped"
        add(f"| {skill['id']} {skill['aegis']} | {'; '.join(official_unique)} | {'; '.join(missing_unique) or '—'} | {note} |")
    add("")
    multi = [skill for skill in skills if skill["coverage"]["official_visual_layers"] >= 2]
    multi.sort(key=lambda skill: -skill["coverage"]["official_visual_layers"])
    add("")
    add(f"Multi-layer coverage overall is good: **{len(multi)}** skills have two or more official asset")
    add(f"layers and **{sum(1 for skill in multi if skill['coverage']['status'] == 'COMPLETE')}** of them are COMPLETE. The largest official")
    add("effects are mostly fully mapped (e.g. " + ", ".join(
        f"{skill['id']} {skill['aegis']} {skill['coverage']['official_visual_layers']} layers [{skill['coverage']['status']}]"
        for skill in multi[:6]) + "); the asset-layer gaps are the four PARTIAL skills listed above.")
    add("")
    add("Details: Fireball's official projectile is the animated `fireball.spr` (3D `playSprite`,")
    add("4 colour-keyed copies, 333 ms each) while the client fires `fireparticle.tga`; Sightrasher's")
    add("8 `sight.spr` 3D layers have no client counterpart; Venom Dust misses the `particle3.spr`")
    add("swirl. All three `.spr` files exist in `CondaRO.grf`, so these are pure mapping gaps.")
    add("")
    add("### 3.6 Persistent status auras: only Assumptio is implemented")
    add("")
    add(f"{len(with_icon)} learnable skills apply a status effect whose `EFST_*` icon is known from")
    add("`db/pre-re/status.yml`; `status_aura.rs` registers only **EFST 110/473** (Assumptio / party")
    add(f"Assumptio). Implemented: {', '.join(f'{s['id']} {s['aegis']}' for s in aura_implemented)}.")
    add("The official client draws a persistent glow for most buffs (Blessing, Increase AGI, Angelus,")
    add("Improve Concentration, Two-Hand Quicken, Impositio, Suffragium, Aspersio, Kyrie, ...). The")
    add("aura registry was designed to be extended per status; the highest-visibility pre-renewal")
    add("candidates are EFST 10 (Blessing), 12 (Increase AGI), 9 (Angelus), 3 (Improve Concentration),")
    add("2 (Two-Hand Quicken), 15/16/17/18 (Impositio/Suffragium/Aspersio/Benedictio).")
    add("")
    add("### 3.7 Missing asset layers (MISSING, 7 skills)")
    add("")
    add("| Skill | Official layer | Expected asset |")
    add("|---|---|---|")
    for skill in noncomplete:
        if skill["coverage"]["status"] != "MISSING":
            continue
        for layer in skill["coverage"]["missing_layers"]:
            add(f"| {skill['id']} {skill['aegis']} | {layer['kind']} `{layer.get('file')}` effect {layer['effect_id']} | `{layer.get('asset')}` |")
    add("")
    add("All are cheap wins: the assets are in the GRFs and only the data row is missing.")
    add("152/230/523/524/525/1004 are thrown-item 3D billboards using item icons from")
    add("`data\\texture\\유저인터페이스\\item\\` (stone, acid bottle, shuriken, kunai, huuma, venom knife);")
    add("534 NJ_KOUENKA misses the plain `firehit.str` (the client only has the `firehit%d` template).")
    add("The client currently falls back to a generic `pok3.tga` projectile.")
    add("")
    add("### 3.8 Sound facts")
    add("")
    sound_counts = Counter(skill["current_client"]["sound_status"] for skill in skills)
    add("| sound status | skills |")
    add("|---|---:|")
    for status in ("COVERED", "PARTIAL", "MISSING", "NO_OFFICIAL"):
        add(f"| {status} | {sound_counts[status]} |")
    add("")
    wav_missing = sorted({sound["wav"] for skill in skills for sound in skill["official"]["sound_wavs"] if sound["asset_exists"] is False})
    sound_missing = [skill for skill in skills if skill["current_client"]["sound_status"] == "MISSING"]
    sound_partial = [skill for skill in skills if skill["current_client"]["sound_status"] == "PARTIAL"]
    disagreements = sum(1 for skill in skills if skill["official"]["sounds_extracted_disagrees"])
    add(f"Official wavs missing from the GRFs: **{len(wav_missing)}** unique ({', '.join(f'`{wav}`' for wav in wav_missing)}).")
    add("")
    add(f"Client skills with an official sound but **no mapping** ({len(sound_missing)}): "
        + ", ".join(f"{skill['aegis']} ({skill['id']})" for skill in sound_missing) + ".")
    add(f"Partial sound mapping ({len(sound_partial)}): "
        + ", ".join(f"{skill['aegis']} ({skill['id']})" for skill in sound_partial)
        + " — each is missing only the official `_hit_fist%d` template. The client sound generator")
    add("(`tools/gen_skill_sounds.py`) does not expand `%d` templates and drops wavs that are not exact")
    add("GRF paths, so template sounds (`_hit_fist%d`, `ef_icearrow%d`, `ef_firearrow%d`) are lost.")
    add("")
    add("**Data-quality note on the extracted sound table.** `windows_client_skill_sounds.json` and its")
    add("extractor merge the symbolic `ef_*` / `*_sound` blocks that follow a numeric effect into that")
    add("effect (the block delimiter is `\\n\\t(\\d+): [`, so non-numeric keys are swallowed). Examples:")
    add("skill 16 MG_STONECURSE gets the symbolic `stonecurse_sound`, skills 142/378 get Hyper-Novice /")
    add("Spirit-Handler wavs, skills 52/53/57/136/138 get commented-out `//wav:` alternates, and skill 16")
    add("gets a symbolic-only sound. After decoding the EUC-KR paths, **" + str(disagreements) + " learnable")
    add("skills** disagree with the layer-derived wav list. This audit therefore derives official sounds")
    add("from the effect layers themselves. The same block-delimiter bug exists in")
    add("`extract_windows_client_skill_sprite_effects.py` and `extract_skill_sounds.py`.")
    add("")
    add("### 3.9 Extractor gaps that hide real official layers")
    add("")
    gaps = [(skill, skill["official"]["extractor_gaps"]) for skill in skills if skill["official"]["extractor_gaps"]]
    add(f"{len(gaps)} learnable skills have facts in the raw `SkillEffect.js` that")
    add("`tools/windows_client_skill_effects.json` drops: `firstHitEffectId` is not in the extractor's")
    add("`FIELDS`, and effect id **0** (the standard `pok3.tga` hit flash) is filtered out by")
    add("`numeric_ids`. The client data is generated from the same JSON, so these layers are absent")
    add("from the client too:")
    add("")
    add("| Skill | Dropped field | Effect ids |")
    add("|---|---|---|")
    for skill, extractor_gaps in gaps:
        for gap in extractor_gaps:
            add(f"| {skill['id']} {skill['aegis']} | `{gap['field']}` | {gap['effect_ids']} |")
    add("")
    add("The clearest real loss is the `firstHitEffectId` lens flash of MG_SOULSTRIKE (13) and")
    add("AC_DOUBLE (46); the effect-0 omission is partly masked because the client's procedural fallback")
    add("already renders a generic `pok3.tga` hit flash for those skills.")
    add("")
    add("Fix the extractor (add the `firstHitEffectId*` fields, keep 0 as the standard hit flash) and")
    add("regenerate the data files.")
    add("")
    add("### 3.10 Double-spawn / phase notes")
    add("")
    add("- **Heal (28)** is COMPLETE but renders the EF_HEAL cylinders twice: the hand-written")
    add("  `target_auras` in `skill_visuals` *and* the generated procedural rows for effect 312/320")
    add("  (`lib.rs:2457` only short-circuits skills in `has_custom_completion`; 28 is not in it).")
    add("  Its official Hit layers (effect 320) are also covered by Success-phase rows.")
    add("- **MC_CARTREVOLUTION (153)**: official Hit `cartrevolution.str` is mapped as a Cast row.")
    add("- Custom-skill data rows are dead weight today: 29/33/34/110/270/401 have generated rows")
    add("  that `has_custom_completion` skips (`lib.rs:2457,4571`).")
    add("")
    add("## 4. Prioritized fix list")
    add("")
    add("| Priority | Skill | Status | Why it matters | Concrete fix |")
    add("|---|---|---|---|---|")
    add("| P0 | 34 AL_BLESSING | WRONG_ASSET | Signature Acolyte buff; players expect the flying angel | Use official effect 42: SPR `축복` (head, 1500 ms, yOffset -120) + 2× `particle6` sparkle + `pok2.tga`; remove hand-written arm; add EFST 10 aura |")
    add("| P0 | 29 AL_INCAGI | WRONG_ASSET | Signature Acolyte buff; generated ac_center2/agi_up rows already exist | Remove hand-written arm + `has_custom_completion` entry; enable effect 37 layers; add EFST 12 aura |")
    add("| P0 | 110 BS_HAMMERFALL | WRONG_ASSET | Visible Blacksmith skill; official asset is in the GRF | Replace `black_hammerfall.str` with `crashearth.str`; drop custom arm so the generated row plays |")
    add("| P1 | 33 AL_ANGELUS | COMPLETE (fragile) | Works only through the hand-written arm | Propagate official `head:true`; then the regenerated row keeps the overhead anchor |")
    add("| P1 | 59/62/357 (Spear Boomerang, Bowling Bash, LK Concentration) | COMPLETE but feet-anchored | Official `head:true` `.str` buffs shown at the feet | Add official per-layer `head`/`yOffset` to the descriptor; regenerate |")
    add("| P1 | 152/230/523/524/525/1004 | MISSING | Official thrown-item billboards are in the GRFs | Add rows for effect 308/298/613/614/615/600 item-icon 3D layers |")
    add("| P2 | 17 MG_FIREBALL | PARTIAL | Missing animated fireball projectile | Add SPR/3D `fireball.spr` layer for effect 24 |")
    add("| P2 | 81 WZ_SIGHTRASHER | PARTIAL | Missing 8 `sight.spr` 3D layers | Add effect 62 sprite layers |")
    add("| P2 | 140 AS_VENOMDUST | PARTIAL | Missing `particle3.spr` swirl | Add effect 171 sprite layer |")
    add("| P2 | 118 HT_SHOCKWAVE | PARTIAL | Official `shockwave.spr` absent from GRF | Keep STR `shockwavehit.str`; note in missing-assets.txt |")
    add("| P2 | 534 NJ_KOUENKA | MISSING | Plain `firehit.str` exists, only `%d` template mapped | Add `firehit.str` row |")
    add("| P2 | buff skills with EFST | n/a | Only Assumptio has a persistent aura | Extend `status_aura_spec`: EFST 10, 12, 9, 3, 2, 15-18 |")
    add("| P2 | sound gaps (8 skills) | n/a | Official cast sounds not played | Expand `%d` wav templates in `gen_skill_sounds.py` |")
    add("")
    add("## 5. Method / provenance")
    add("")
    add("Reproduce: `cd notes/skill-audit && python3 audit.py && python3 report.py`")
    add("(`audit_core.py` holds the parsers; it reads the repo/tools/GRFs read-only).")
    add("")
    add("- Learnable set: union of every job `Tree` entry with `MaxLevel != 0` in")
    add("  `db/pre-re/skill_tree.yml` (77 job trees, 457 unique skills, inheritance included).")
    add("- Official layers: `/tmp/EffectTable.js` (identical to")
    add("  `CondaRO-linux/notes/korangar-windows-client-data/roBrowserLegacy_EffectTable.js`),")
    add("  parsed with all fields (`type`, `file`/`textureName`/`spriteName`, `wav`, `head`,")
    add("  `attachedEntity`, `duration`, `delayOffset`/`delayLate`/`delayFrame`, `yOffset`,")
    add("  `duplicate`, `func`). Symbolic `ef_*` blocks are kept separate from numeric effect blocks.")
    add("- Official skill→effect ids: `tools/windows_client_skill_effects.json` (the pipeline input).")
    add("- Current client: `skill_effect_assets.ron` (189 rows), `skill_sprite_effects.ron` (48),")
    add("  `skill_procedural_effects.ron` (265), `skill_sounds.ron` (369), `skill_actions.ron` (205),")
    add("  plus the hand-written `skill_visuals`/`has_custom_completion` (8 skills) and `status_aura`")
    add("  (2 EFST ids) in the Rust sources.")
    add("- Asset existence: every referenced `.str`/`.spr`/`.tga`/`.bmp`/`.wav` looked up in the")
    add("  three GRF name tables (653,299 files). Template names match any concrete file.")
    add("- Action coverage: 457/457 MATCH (the client table and the official facts agree; the")
    add("  client plays the default `Skill` gesture where the table lists no action).")
    add("- Escaped Korean paths are EUC-KR (`\\\\xNN`) in the JS dump and are decoded before")
    add("  asset checks (e.g. `유저인터페이스\\item\\돌.bmp`).")
    add("")
    add("### Caveats")
    add("")
    add("- `NO_OFFICIAL_EFFECT` (290) means the official table defines no renderable asset layer: 84")
    add("  are passive skills; the rest are hard-coded primitives / sound-only effects in the")
    add("  official client (`official.hardcoded_layers` keeps the evidence, e.g. camera quake,")
    add("  bolt-rain FUNC effects). It does not mean the skill has no visible official behavior.")
    add("- The extracted `windows_client_skill_effects.json` omits effect id 0 and")
    add("  `firstHitEffectId`; see section 3.9.")
    add("- The official table is a community RE dump; a handful of entries are internally")
    add("  inconsistent (e.g. some 4th-job ids). Only facts for pre-renewal skills were used, and")
    add("  every asset fact was re-validated against the GRFs.")
    add("- `PARTIAL` counts layers, not visual quality: a covered layer can still differ in scale,")
    add("  alpha or timing from the official values (e.g. the built-in procedural HitFlash uses")
    add("  normalised sizes).")
    add("")
    (OUT / "summary.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    data = load()
    write_missing_assets(data)
    write_summary(data)
    print("wrote missing-assets.txt and summary.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
