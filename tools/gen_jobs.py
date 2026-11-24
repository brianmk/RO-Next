#!/usr/bin/env python3
"""Generate the data-driven job/appearance registry of the client.

Writes ``korangar/archive/data/client/jobs.ron``, the single source of truth for
player job appearance in RO-Next:

* job id -> verified player body sprite stem and body palette stem,
* the classic client job flags (entity type, baby/transcendent/4th/expanded)
  and the baby body scale,
* the sprite path templates (body, head, body palette, hair palette, headgear)
  and the sex suffixes,
* the appearance defaults and ranges (hair style -> head sprite lookup, default
  head, head sprite id range, default palette colors).

Input (committed, so this generator and its ``--check`` run offline):

* ``tools/rustnarok_actor_facts.json`` — the classic client engine's
  ``job_sprites.txt`` table (body/imf/palette/weapon folder and prefix), pinned
  by the rustnarok audit.
* ``tools/job_facts.json`` — produced by ``tools/extract_job_facts.py`` from the
  rAthena server tree (``enum e_job``, ``db/pre-re/skill_tree.yml``,
  ``db/re/skill_tree.yml``) and the CondaRO client archives (body/head/palette
  sprite probe, ``jobidentity.lub``/``npcidentity.lub``), including the verified
  sprite fallbacks for jobs whose client sprite is not part of this data set.

The generated file is embedded in the client with ``include_str!`` as a
compiled fallback and can be overridden at runtime through the client archive
(``data\\client\\jobs.ron``), exactly like the weapon and skill data files.

Usage:
    ./tools/gen_jobs.py            # regenerate the data file
    ./tools/gen_jobs.py --check    # fail if the committed file is out of date
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

DEFAULT_FACTS = Path("tools/job_facts.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/jobs.ron")
#: Rustnarok-engine `job_sprites.txt` facts (body/imf/palette/weapon names).
DEFAULT_ACTOR_FACTS = Path("tools/rustnarok_actor_facts.json")

#: Sprite/palette path templates. Placeholders: {sex}, {sprite}, {palette},
#: {head}, {hair}, {color}, {name}. These are the classic client file layouts.
PATHS = {
    "body": "인간족\\몸통\\{sex}\\{sprite}_{sex}",
    "head": "인간족\\머리통\\{sex}\\{head}_{sex}",
    "body_palette": "data\\palette\\몸\\{palette}_{sex}_{color}.pal",
    "head_palette": "data\\palette\\머리\\머리{hair}_{sex}_{color}.pal",
    "headgear": "악세사리\\{sex}\\{sex}{name}",
    "imf": "data\\imf\\{imf}_{sex}.imf",
    "weapon": "인간족\\{folder}\\{prefix}_{sex}{suffix}",
}

SEX_SUFFIX = {"male": "남", "female": "여"}

#: Entity types of job ids that are not part of the `Player` ranges. The ids are
#: the client `npcidentity.lub` job constants (`JT_WARPNPC`, `JT_HIDDEN_NPC`,
#: `JT_HIDDEN_WARP_NPC`); the entity type ranges mirror the classic client
#: classification (`pcdb_checkid_sub` in rAthena's `pc.hpp` for the player
#: range).
#: Job capabilities that are not appearance flags. The skill that grants a
#: capability is declared in `data\client\skills.ron` (the skill's
#: `move_while_casting` behaviour flag); a job only gets the capability when its
#: rAthena skill tree contains that skill. `SA_FREECAST` (278) is the
#: Sage/Scholar/Professor `Free Cast` passive and is learned by Sage (16) and
#: its rebirth class Professor (4017) only. This is the client-visible
#: behaviour of the old hard-coded `FREE_CAST_JOB_IDS = [16, 4017]`, now data.
MOVE_WHILE_CASTING_JOBS = (16, 4017)

ENTITY_TYPE_RANGES = {
    "player_ranges": [(0, 44), (4000, 5999)],
    "npc_ranges": [(46, 999), (10000, 19999)],
    "monster_ranges": [(1000, 3999), (20000, 29999)],
    # Client job ids of homunculi (JT_MER_LIF..JT_MER_VANILMIRTH_H2 and the
    # Homunculus S ids). The entity-appear object type 0x8 is the authoritative
    # marker; these ranges are the job-id fallback.
    "homunculus_ranges": [(6001, 6016), (6048, 6052)],
    # Dedicated mercenary client job ids (JT_MER_ARCHER01..JT_MER_MONSTER).
    # Summoned mercenaries usually reuse monster class ids and are identified
    # by the appear object type 0x9 instead.
    "mercenary_ranges": [(6017, 6047)],
    # Pets reuse the monster class id of their monster, so there is no separate
    # pet job id range: the appear object type 0x7 identifies them.
    "pet_ranges": [],
    "default": "Npc",
}


def load_facts(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"facts file not found: {path}")
    return json.loads(path.read_text())


def ron_str(value: str) -> str:
    """Escape a string for RON (backslashes in sprite paths are the important part)."""
    return json.dumps(value, ensure_ascii=False)


def ron_option_str(value: str | None) -> str:
    return "None" if value is None else f"Some({ron_str(value)})"


def ron_range(start: int, end: int) -> str:
    return f"(start: {start}, end: {end})"


def body_stem_set(facts: dict, sex: str) -> set[str]:
    suffix = f"_{sex}"
    return {stem[: -len(suffix)] for stem in facts["client"]["body_sprites"][sex] if stem.endswith(suffix)}


def build(facts: dict, actor_facts: dict) -> dict:
    server = facts["server"]
    client = facts["client"]
    jobs = server["jobs"]
    job_ids = {row["id"] for row in jobs}
    actor_jobs = {int(row["job"]): row for row in actor_facts.get("job_sprites", [])}
    body_sprites = {"남": body_stem_set(facts, "남"), "여": body_stem_set(facts, "여")}
    palettes = {sex: set(stems) for sex, stems in client["body_palettes"].items()}
    head_sprites = {sex: set(ids) for sex, ids in client["head_sprites"].items()}

    # The client/engine lowercases all asset lookups (see GameFileLoader), so the
    # probe and this validation are case-insensitive. The data keeps the classic
    # client spelling of the stem (`DRAGON_KNIGHT`).
    def body_exists(stem: str) -> bool:
        stem = stem.lower()
        return stem in body_sprites["남"] or stem in body_sprites["여"]

    def palette_exists(stem: str) -> bool:
        stem = stem.lower()
        return stem in palettes["남"] or stem in palettes["여"]

    problems: list[str] = []

    # The facts file is the output of the extractor and is committed, but the
    # generator still checks the invariants it depends on (so a hand-edited or
    # truncated facts file cannot silently produce wrong data).
    for job_id, stem in client["client_job_sprites"].items():
        if int(job_id) not in job_ids:
            problems.append(f"client job sprite {job_id} is not a server job")
        if not isinstance(stem, str) or not stem:
            problems.append(f"empty client job sprite for job {job_id}")
    for job_id, stem in client["sprite_fallbacks"].items():
        if int(job_id) not in job_ids:
            problems.append(f"sprite fallback {job_id} is not a server job")
        if not body_exists(stem):
            problems.append(f"sprite fallback {stem!r} of job {job_id} has no body sprite")

    rows = []
    for job in jobs:
        job_id = int(job["id"])
        intended = job["client_sprite"]
        resolved = job["sprite"]
        source = job["sprite_source"]
        palette = job["palette"]

        if source == "Mapped":
            if intended is None or resolved != intended:
                problems.append(f"job {job_id}: Mapped row does not use its client sprite")
            if not body_exists(resolved):
                problems.append(f"job {job_id}: mapped sprite {resolved!r} is missing")
        elif source == "Fallback":
            if intended is None or body_exists(intended):
                problems.append(f"job {job_id}: Fallback row must have a missing client sprite")
            if not body_exists(resolved):
                problems.append(f"job {job_id}: fallback sprite {resolved!r} is missing")
        elif source == "Unmapped":
            if intended is not None:
                problems.append(f"job {job_id}: Unmapped row has a client sprite")
            if not body_exists(resolved):
                problems.append(f"job {job_id}: fallback sprite {resolved!r} is missing")
        else:
            problems.append(f"job {job_id}: unknown sprite source {source!r}")

        if palette is not None and not palette_exists(palette):
            problems.append(f"job {job_id}: palette stem {palette!r} has no palette file")

        actor = actor_jobs.get(job_id)
        imf = actor.get("imf") if actor else None
        weapon_folder = actor.get("weapon_folder") if actor else None
        weapon_prefix = actor.get("weapon_prefix") if actor else None
        for value, name in ((imf, "imf"), (weapon_folder, "weapon folder"), (weapon_prefix, "weapon prefix")):
            if value is not None and (not isinstance(value, str) or not value):
                problems.append(f"job {job_id}: empty/invalid rustnarok {name}")

        rows.append(
            {
                "id": job_id,
                "name": job["name"],
                "entity_type": job["entity_type"],
                "sprite": resolved,
                "client_sprite": intended,
                "palette": palette,
                "imf": imf,
                "weapon_folder": weapon_folder,
                "weapon_prefix": weapon_prefix,
                "source": source,
                "baby": bool(job["baby"]),
                "transcendent": bool(job["transcendent"]),
                "fourth": bool(job["fourth"]),
                "expanded": bool(job["expanded"]),
                "scale": float(job["scale"]),
            }
        )

    rows.sort(key=lambda row: row["id"])
    for pair in zip(rows, rows[1:]):
        if pair[0]["id"] >= pair[1]["id"]:
            problems.append(f"job ids are not strictly ascending: {pair[0]['id']} then {pair[1]['id']}")

    appearance = client["appearance"]
    for sex_key, sex in (("male", "남"), ("female", "여")):
        for style, head in enumerate(appearance["hair_styles"][sex_key]):
            if head not in head_sprites[sex]:
                problems.append(f"{sex_key} hair style {style} maps to missing head sprite {head}")
    if appearance["default_head"] not in head_sprites["남"] or appearance["default_head"] not in head_sprites["여"]:
        problems.append("the default head sprite does not exist")
    for sex in ("남", "여"):
        expected = set(range(appearance["head_range"][0], appearance["head_range"][1] + 1))
        missing = expected - head_sprites[sex]
        if missing:
            problems.append(f"{sex}: head range contains missing sprite ids {sorted(missing)}")
    if appearance["hair_style_range"] != [0, len(appearance["hair_styles"]["male"]) - 1]:
        problems.append("the hair style range does not match the hair style table length")
    if len(appearance["hair_styles"]["male"]) != len(appearance["hair_styles"]["female"]):
        problems.append("the male and female hair style tables have different lengths")

    special = server["special_entity_job_ids"]
    for key in ("warp", "hidden"):
        if not special.get(key):
            problems.append(f"no special {key} job ids in the facts")

    headgear = client["headgear"]
    if headgear["view_ids"] <= 0:
        problems.append("the facts contain no headgear view ids")
    if headgear["with_sprite"] + len(headgear["missing"]) != headgear["view_ids"]:
        problems.append("the headgear sprite/missing counts do not add up")
    if headgear["missing"] != sorted(set(headgear["missing"])):
        problems.append("the headgear missing view ids are not sorted and unique")

    if problems:
        raise SystemExit("facts are inconsistent:\n  " + "\n  ".join(problems))

    # Job capabilities are curated (the client cannot derive them from the
    # appearance data); every referenced id has to be a real job row.
    capabilities = {"move_while_casting": sorted(set(MOVE_WHILE_CASTING_JOBS))}
    rows_by_id = {row["id"] for row in rows}
    for job_id in capabilities["move_while_casting"]:
        if job_id not in rows_by_id:
            raise SystemExit(f"error: move_while_casting capability references unknown job {job_id}")

    return {
        "provenance": facts["provenance"],
        "jobs": rows,
        "capabilities": capabilities,
        "appearance": appearance,
        "special": special,
        "entity_type_ranges": ENTITY_TYPE_RANGES,
        "headgear_missing": headgear["missing"],
        "headgear_view_ids": headgear["view_ids"],
        "headgear_with_sprite": headgear["with_sprite"],
        "summary": facts["client"]["summary"],
    }


def render(data: dict) -> str:
    lines: list[str] = []
    add = lines.append

    provenance = data["provenance"]
    server = provenance["server"]
    archives = ", ".join(
        f"{archive['name']} ({archive['files']} files, sha256 {archive['sha256'][:16]}…)"
        for archive in provenance["client"]["archives"]
    )
    server_files = ", ".join(f"`{path}`" for path in sorted(server["files"]))
    jobs = data["jobs"]
    appearance = data["appearance"]
    summary = data["summary"]

    add("// Data-driven job/appearance registry for RO-Next.")
    add("//")
    add("// Generated by `tools/gen_jobs.py`; do not edit by hand.")
    add("// Regenerate with: `./tools/gen_jobs.py`")
    add("// Drift check:      `./tools/gen_jobs.py --check`")
    add("//")
    add("// Sources (committed facts, no game files or server tree needed to regenerate):")
    add("//   * tools/job_facts.json — generated by `tools/extract_job_facts.py` from")
    add(f"//     the rAthena server tree (HEAD {server['head'] or 'unknown'}; {server_files})")
    add("//   * tools/rustnarok_actor_facts.json — the classic client engine table")
    add("//     (IMF and weapon folder/prefix); every runtime path is still optional")
    add("//     and falls back when the trimmed archives lack the file.")
    add("//   * job_facts archive probes:")
    add("//     and the CondaRO client archives:")
    add(f"//     {archives}.")
    add("//")
    add(f"// The job->sprite mapping is the classic client engine table ({len(jobs)} jobs: ")
    add(
        f"// {summary['mapped']} directly mapped, {summary['fallback']} with a verified fallback,"
        f" {summary['unmapped']} not mapped by the client)."
    )
    add("// Every sprite/palette in this file was probed against the archives; a mapping")
    add("// whose sprite is not part of this data set always carries the fallback that is")
    add("// actually used (`source`) and the client's intended stem (`client_sprite`).")
    add("//")
    add("//")
    add(
        f"// Headgear: {data['headgear_with_sprite']} of {data['headgear_view_ids']} accessory view ids"
        f" have a sprite; {len(data['headgear_missing'])} are documented as missing from this data set"
    )
    add("// and are skipped by the client (accessory layers are optional).")
    add("//")
    add("// The file is loaded at runtime through the normal client file loader")
    add("// (`data\\client\\jobs.ron`), so a server can override the table without")
    add("// rebuilding the client. The compiled copy below is the fallback.")
    add("(")
    provenance_text = (
        "rAthena server tree HEAD "
        + (server["head"] or "unknown")
        + " (enum e_job, db/pre-re/skill_tree.yml, db/re/skill_tree.yml) + CondaRO client archives; "
        "generated by tools/gen_jobs.py from tools/job_facts.json (extracted by tools/extract_job_facts.py)"
    )
    add(f"    provenance: {ron_str(provenance_text)},")
    add("    version: 1,")
    add("    // Classic client sprite/palette path templates.")
    add("    paths: (")
    add(f"        body: {ron_str(PATHS['body'])},")
    add(f"        head: {ron_str(PATHS['head'])},")
    add(f"        body_palette: {ron_str(PATHS['body_palette'])},")
    add(f"        head_palette: {ron_str(PATHS['head_palette'])},")
    add(f"        headgear: {ron_str(PATHS['headgear'])},")
    add(f"        imf: {ron_str(PATHS['imf'])},")
    add(f"        weapon: {ron_str(PATHS['weapon'])},")
    add("    ),")
    add("    sex_suffix: (")
    add(f"        male: {ron_str(SEX_SUFFIX['male'])},")
    add(f"        female: {ron_str(SEX_SUFFIX['female'])},")
    add("    ),")
    add("    appearance: (")
    add("        // Network hair style index -> client head sprite id.")
    add("        hair_styles: (")
    add(f"            male: {json.dumps(appearance['hair_styles']['male'])},")
    add(f"            female: {json.dumps(appearance['hair_styles']['female'])},")
    add("        ),")
    add("        // Indices accepted by the hair style lookup; bigger values are passed")
    add("        // through as sprite ids (classic client behaviour).")
    add(f"        hair_style_range: {ron_range(*appearance['hair_style_range'])},")
    add("        // Head sprite ids that exist in this data set.")
    add(f"        head_range: {ron_range(*appearance['head_range'])},")
    add("        default_head: %d," % appearance["default_head"])
    add("        default_body_palette: %d," % appearance["default_body_palette"])
    add("        default_head_palette: %d," % appearance["default_head_palette"])
    add("    ),")
    add("    // Entity classification for job ids that are not in `jobs`; the ranges are")
    add("    // the classic client job id ranges (player ids are always in `jobs`).")
    add("    entity_types: (")
    add(f"        warp: {json.dumps(data['special']['warp'])},")
    add(f"        hidden: {json.dumps(data['special']['hidden'])},")
    for key in (
        "player_ranges",
        "npc_ranges",
        "monster_ranges",
        "homunculus_ranges",
        "mercenary_ranges",
        "pet_ranges",
    ):
        if key == "homunculus_ranges":
            add("        // Homunculus client job ids (JT_MER_LIF..JT_MER_VANILMIRTH_H2 and the")
            add("        // Homunculus S ids). The entity-appear object type 0x8 is the")
            add("        // authoritative marker; these ranges are the job-id fallback.")
        if key == "mercenary_ranges":
            add("        // Dedicated mercenary client job ids (JT_MER_ARCHER01..JT_MER_MONSTER).")
            add("        // Summoned mercenaries usually reuse monster class ids and are")
            add("        // identified by the appear object type 0x9 instead.")
        if key == "pet_ranges":
            add("        // Pets reuse the monster class id of their monster and are identified")
            add("        // by the appear object type 0x7, so this list is normally empty.")
        ranges = ", ".join(ron_range(start, end) for start, end in data["entity_type_ranges"][key])
        add(f"        {key}: [{ranges}],")
    add(f"        default: {data['entity_type_ranges']['default']},")
    add("    ),")
    add("    // Job capabilities that are not appearance flags. The granting skill is")
    add("    // declared in `skills.ron` (`move_while_casting`); the job and the learned")
    add("    // skill both have to hold for the capability to apply.")
    add("    capabilities: (")
    add(f"        move_while_casting: {json.dumps(data['capabilities']['move_while_casting'])},")
    add("    ),")
    add("    // Headgear view ids whose accessory sprite is not part of this data set.")
    add("    // The client creates the headgear part only when the sprite exists, so these")
    add("    // are a documented data gap, not a rendering failure.")
    add("    headgear: (")
    add(f"        missing: {json.dumps(data['headgear_missing'])},")
    add("    ),")
    add("    // One row per rAthena `enum e_job` player job (sentinels JOB_MAX_BASIC/JOB_MAX omitted).")
    add("    //   sprite:        verified body sprite stem (used by the client)")
    add("    //   client_sprite: the stem of the classic client engine table, if it maps the job")
    add("    //   palette:       body palette stem, None when the data set has no palette file")
    add("    //                  (the sprite-embedded palette is used instead)")
    add("    //   source:        Mapped | Fallback (missing client sprite) | Unmapped (no client mapping)")
    add("    //   scale:         entity scale (0.75 for baby jobs, classic client value)")
    add("    jobs: [")
    for row in data["jobs"]:
        add("        (")
        add(f"            id: {row['id']},")
        add(f"            name: {ron_str(row['name'])},")
        add(f"            entity_type: {row['entity_type']},")
        add(f"            sprite: {ron_str(row['sprite'])},")
        add(f"            client_sprite: {ron_option_str(row['client_sprite'])},")
        add(f"            palette: {ron_option_str(row['palette'])},")
        add(f"            imf: {ron_option_str(row['imf'])},")
        add(f"            weapon_folder: {ron_option_str(row['weapon_folder'])},")
        add(f"            weapon_prefix: {ron_option_str(row['weapon_prefix'])},")
        add(f"            source: {row['source']},")
        add(f"            baby: {str(row['baby']).lower()},")
        add(f"            transcendent: {str(row['transcendent']).lower()},")
        add(f"            fourth: {str(row['fourth']).lower()},")
        add(f"            expanded: {str(row['expanded']).lower()},")
        add(f"            scale: {row['scale']},")
        add("        ),")
    add("    ],")
    add(")")
    add("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--actor-facts", default=str(DEFAULT_ACTOR_FACTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    args = parser.parse_args()

    facts = load_facts(Path(args.facts))
    actor_facts = load_facts(Path(args.actor_facts))
    data = build(facts, actor_facts)
    content = render(data)

    output = Path(args.output)
    if args.check:
        current = output.read_text() if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_jobs.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(data['jobs'])} job rows)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content)
    summary = facts["client"]["summary"]
    imf_count = sum(1 for job in data["jobs"] if job["imf"])
    weapon_count = sum(1 for job in data["jobs"] if job["weapon_folder"])
    print(
        f"{output}: {imf_count} IMF stems, {weapon_count} weapon folder/prefix rows;",
        end=" ",
    )
    print(
        f"{output}: {len(data['jobs'])} job rows "
        f"({summary['mapped']} mapped / {summary['fallback']} fallback / {summary['unmapped']} unmapped), "
        f"{summary['baby']} baby, {summary['transcendent']} transcendent, "
        f"{summary['fourth']} 4th, {summary['expanded']} expanded, "
        f"{summary['embedded_palettes']} embedded palettes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
