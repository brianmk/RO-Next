#!/usr/bin/env python3
"""Render `korangar/archive/data/client/aliases.ron` from `tools/data/alias_facts.json`.

The client data file carries the three generated alias tables that used to be
hand-maintained Rust arrays:

* `job_skill_tree_fallbacks`: job id -> job whose client skill tree to use when
  the job's own tree is empty (`world/library/job_alias.rs`),
* `monster_sprite_aliases` + `generic_monster_sprites`: monster job id ->
  verified existing sprite path (`world/library/monster_sprite_alias.rs`),
* `skill_icon_aliases` + `generic_skill_icons`: skill id -> verified existing
  icon path (`world/library/skill_icon_alias.rs`).

The facts are extracted from the rAthena server tree and the CondaRO client
archives by `tools/extract_alias_facts.py`; this generator only renders them
(no game files or server tree needed), validates the schema/invariants and is
the CI drift guard:

    python3 tools/gen_aliases.py --check

The file is loaded at runtime through the normal client file loader
(`data\\client\\aliases.ron`), so a server can override the tables without
rebuilding the client; the committed copy is compiled in as the fallback.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

TOOLS_DIR = Path(__file__).resolve().parent
REPO_ROOT = TOOLS_DIR.parent
FACTS_PATH = TOOLS_DIR / "data" / "alias_facts.json"
OUTPUT_PATH = REPO_ROOT / "korangar" / "archive" / "data" / "client" / "aliases.ron"

MONSTER_FOLDERS = ("몬스터\\", "npc\\")
ICON_FOLDER = "아이템\\"


def load_facts(path: Path = FACTS_PATH) -> dict:
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def _require(condition: object, message: str) -> None:
    if not condition:
        raise ValueError(message)


def _sorted_unique_ids(rows: list[dict], key: str, label: str) -> None:
    ids = [row[key] for row in rows]
    _require(all(isinstance(value, int) and not isinstance(value, bool) for value in ids), f"{label}: ids must be integers")
    _require(ids == sorted(ids), f"{label}: ids must be sorted")
    _require(len(set(ids)) == len(ids), f"{label}: duplicate id")


def _validate_path(path: object, prefixes: tuple[str, ...], label: str) -> None:
    _require(isinstance(path, str) and path, f"{label}: empty path")
    _require(any(path.startswith(prefix) for prefix in prefixes), f"{label}: {path!r} is outside {prefixes}")
    _require(not path.endswith((".spr", ".act")), f"{label}: {path!r} must not carry a file extension")
    _require("missing" not in path.lower(), f"{label}: {path!r} is a missing placeholder")


def build_aliases(facts: dict) -> dict:
    """Validate the facts and return the four tables the data file carries."""
    _require(isinstance(facts, dict), "alias facts must be a JSON object")
    _require(facts.get("schema_version") == 1, "unsupported alias facts schema version")
    provenance = facts.get("provenance")
    _require(isinstance(provenance, dict), "alias facts have no provenance block")
    server = provenance.get("server")
    _require(isinstance(server, dict) and server.get("head"), "alias facts have no server provenance")
    client = provenance.get("client")
    _require(isinstance(client, dict), "alias facts have no client provenance")

    jobs = facts.get("jobs") or {}
    monsters = facts.get("monsters") or {}
    skills = facts.get("skills") or {}
    job_rows = jobs.get("skills_tree_fallbacks") or []
    monster_rows = monsters.get("sprite_aliases") or []
    generic_monsters = monsters.get("generic_sprites") or []
    skill_rows = skills.get("icon_aliases") or []
    generic_icons = skills.get("generic_icons") or []

    _require(job_rows, "alias facts contain no job skill-tree fallbacks")
    _require(monster_rows, "alias facts contain no monster sprite aliases")
    _require(skill_rows, "alias facts contain no skill icon aliases")

    # Schema/count parity between the facts summary and the rows.
    _require(jobs.get("summary", {}).get("fallbacks") == len(job_rows), "job fallback count does not match the summary")
    _require(monsters.get("summary", {}).get("aliases") == len(monster_rows), "monster alias count does not match the summary")
    _require(skills.get("summary", {}).get("aliases") == len(skill_rows), "skill alias count does not match the summary")
    _require(monsters.get("summary", {}).get("generic_sprites") == len(generic_monsters), "generic monster count mismatch")
    _require(skills.get("summary", {}).get("generic_icons") == len(generic_icons), "generic skill icon count mismatch")

    # Job skill-tree fallbacks: sorted, unique, no self/chained fallback.
    _sorted_unique_ids(job_rows, "job_id", "job skill-tree fallbacks")
    fallback_keys = {row["job_id"] for row in job_rows}
    for row in job_rows:
        job_id = row["job_id"]
        fallback = row["fallback_job_id"]
        _require(isinstance(fallback, int) and not isinstance(fallback, bool), f"job {job_id}: fallback must be an integer")
        _require(job_id != fallback, f"job {job_id}: fallback must not be the job itself")
        _require(fallback not in fallback_keys, f"job {job_id}: chained skill-tree fallback {job_id} -> {fallback}")
        _require(row.get("target_tree_skills", 0) > 0, f"job {job_id}: fallback target has no client skill tree")

    # Monster sprite aliases.
    _sorted_unique_ids(monster_rows, "job_id", "monster sprite aliases")
    for row in monster_rows:
        _validate_path(row.get("sprite_path"), MONSTER_FOLDERS, f"monster {row['job_id']}")
    _require(generic_monsters, "no generic monster sprites")
    for path in generic_monsters:
        _validate_path(path, ("몬스터\\",), "generic monster sprite")

    # Skill icon aliases.
    _sorted_unique_ids(skill_rows, "skill_id", "skill icon aliases")
    for row in skill_rows:
        _validate_path(row.get("icon_path"), (ICON_FOLDER,), f"skill {row['skill_id']}")
    _require(generic_icons, "no generic skill icons")
    for path in generic_icons:
        _validate_path(path, (ICON_FOLDER,), "generic skill icon")

    return {
        "job_skill_tree_fallbacks": [(row["job_id"], row["fallback_job_id"]) for row in job_rows],
        "monster_sprite_aliases": [(row["job_id"], row["sprite_path"]) for row in monster_rows],
        "generic_monster_sprites": list(generic_monsters),
        "skill_icon_aliases": [(row["skill_id"], row["icon_path"]) for row in skill_rows],
        "generic_skill_icons": list(generic_icons),
        "job_rows": job_rows,
        "monster_rows": monster_rows,
        "skill_rows": skill_rows,
    }


def escape_ron(value: str) -> str:
    return value.replace("\\", "\\\\").replace('"', '\\"')


def provenance_line(facts: dict) -> str:
    server = facts["provenance"]["server"]
    archives = facts["provenance"]["client"]["archives"]
    archive_text = ", ".join(f"{archive['name']} ({archive['files']} files, sha256 {archive['sha256'][:16]}...)" for archive in archives)
    return (
        f"rAthena server tree HEAD {server['head']} (src/common/mmo.hpp, db/pre-re/mob_db.yml, db/pre-re/skill_tree.yml) "
        f"+ CondaRO client archives ({archive_text}); generated by tools/gen_aliases.py from tools/data/alias_facts.json "
        "(extracted by tools/extract_alias_facts.py)"
    )


def render(facts: dict) -> str:
    aliases = build_aliases(facts)
    counts = (
        f"{len(aliases['job_skill_tree_fallbacks'])} job skill-tree fallbacks, "
        f"{len(aliases['monster_sprite_aliases'])} monster sprite aliases (+{len(aliases['generic_monster_sprites'])} generic), "
        f"{len(aliases['skill_icon_aliases'])} skill icon aliases (+{len(aliases['generic_skill_icons'])} generic)"
    )
    lines: list[str] = [
        "// Data-driven alias tables for RO-Next.",
        "//",
        "// Generated by `tools/gen_aliases.py`; do not edit by hand.",
        "// Regenerate with: `python3 tools/gen_aliases.py`",
        "// Drift check:      `python3 tools/gen_aliases.py --check`",
        "//",
        "// Sources: `tools/data/alias_facts.json` (generated by `tools/extract_alias_facts.py`):",
        f"//   * rAthena server tree HEAD {facts['provenance']['server']['head']}",
        "//     (`src/common/mmo.hpp`, `db/pre-re/mob_db.yml`, `db/re/mob_db.yml`,",
        "//      `db/pre-re/skill_tree.yml`, `db/re/skill_tree.yml`),",
        "//   * the CondaRO client archives:",
    ]
    for archive in facts["provenance"]["client"]["archives"]:
        lines.append(
            f"//     {archive['name']} ({archive['files']} files, sha256 {archive['sha256']}),"
        )
    lines += [
        "//   * the client Lua tables (`jobidentity`, `jobinheritlist`, `skilltreeview`,",
        "//     `skillid`, `skillinfolist`; SHA-256 recorded in the facts file).",
        "//",
        f"// Counts: {counts}. Every sprite/icon path was verified against the archives",
        "// (both `.spr` and `.act` present) and every skill-tree fallback target has a",
        "// non-empty client skill tree.",
        "//",
        "// The file is loaded at runtime through the normal client file loader",
        "// (`data\\\\client\\\\aliases.ron`), so a server can override the tables without",
        "// rebuilding the client. The compiled copy below is the fallback.",
        "(",
        f'    provenance: "{escape_ron(provenance_line(facts))}",',
        "    version: 1,",
        "    // Job id -> job whose (non-empty) client skill tree is used when the job",
        "    // has no or an empty client skill tree.",
        "    job_skill_tree_fallbacks: [",
    ]
    for (job_id, fallback), row in zip(aliases["job_skill_tree_fallbacks"], aliases["job_rows"]):
        lines.append(f"        ({job_id}, {fallback}), // {row['job_name']} -> {row['fallback_job_name']}")
    lines += [
        "    ],",
        "    // Monster job id -> existing sprite path relative to `data\\\\sprite\\\\`",
        "    // (without the `.spr`/`.act` extension); the exact identity path is always",
        "    // tried first, then this table, then `npc\\\\<identity>`, then the generics.",
        "    monster_sprite_aliases: [",
    ]
    for (job_id, path), row in zip(aliases["monster_sprite_aliases"], aliases["monster_rows"]):
        lines.append(
            f'        ({job_id}, "{escape_ron(path)}"), // {row["aegis_name"]} ({row["method"]})'
        )
    lines += [
        "    ],",
        "    // Verified generic monster sprites in preference order.",
        "    generic_monster_sprites: [",
    ]
    for path in aliases["generic_monster_sprites"]:
        lines.append(f'        "{escape_ron(path)}",')
    lines += [
        "    ],",
        "    // Skill id -> existing icon path relative to `data\\\\sprite\\\\` (without the",
        "    // `.spr`/`.act` extension); the exact `아이템\\\\<file_name>` icon is tried",
        "    // first, then this table, then the generic icons.",
        "    skill_icon_aliases: [",
    ]
    for (skill_id, path), row in zip(aliases["skill_icon_aliases"], aliases["skill_rows"]):
        lines.append(
            f'        ({skill_id}, "{escape_ron(path)}"), // {row["skill_name"]} ({row["method"]})'
        )
    lines += [
        "    ],",
        "    // Verified generic skill icons in preference order.",
        "    generic_skill_icons: [",
    ]
    for path in aliases["generic_skill_icons"]:
        lines.append(f'        "{escape_ron(path)}",')
    lines += [
        "    ],",
        ")",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is not up to date")
    parser.add_argument("--facts", default=str(FACTS_PATH), help="alias facts JSON to render (default: tools/data/alias_facts.json)")
    parser.add_argument("--output", default=str(OUTPUT_PATH), help="data file to write (default: korangar/archive/data/client/aliases.ron)")
    args = parser.parse_args()

    try:
        facts = load_facts(Path(args.facts))
        content = render(facts)
    except (OSError, ValueError) as error:
        print(f"gen_aliases: {error}", file=sys.stderr)
        return 1

    output = Path(args.output)
    if args.check:
        if not output.is_file():
            print(f"gen_aliases: {output} is missing; run `python3 tools/gen_aliases.py`", file=sys.stderr)
            return 1
        committed = output.read_text(encoding="utf-8")
        if committed == content:
            print(f"{output} is up to date")
            for line in content.splitlines():
                if line.strip().startswith("// Counts:"):
                    print(line.strip().removeprefix("// "))
                    break
            return 0
        committed_lines = committed.splitlines()
        generated_lines = content.splitlines()
        for index, (committed_line, generated_line) in enumerate(zip(committed_lines, generated_lines)):
            if committed_line != generated_line:
                print(f"{output} is out of date at line {index + 1}:", file=sys.stderr)
                print(f"  committed: {committed_line}", file=sys.stderr)
                print(f"  generated: {generated_line}", file=sys.stderr)
                break
        else:
            print(
                f"{output} is out of date ({len(committed_lines)} committed lines, {len(generated_lines)} generated lines)",
                file=sys.stderr,
            )
        print("run `python3 tools/gen_aliases.py` to regenerate it", file=sys.stderr)
        return 1

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"wrote {output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
