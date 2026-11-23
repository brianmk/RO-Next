#!/usr/bin/env python3
"""Extract the committed rustnarok-client facts used by the RO-Next inventory.

The rustnarok-client checkout is pinned (see PIN below); the checkout is used
read-only and is re-cloned when it is missing. The script writes four stable,
sorted JSON documents into ``tools/``:

    rustnarok_effects_facts.json   effect definitions + skill/status links
    rustnarok_motion_facts.json    ATTACK/THROW/THIRD_ATTACK/PICK_UP/KEEP/CAST/STATES
    rustnarok_status_facts.json    EFST -> icon + looping visual effect ids
    rustnarok_actor_facts.json     job sprite table, cart rules, mount/outfit bits, size statuses

The output contains no game assets and no copyrighted table text beyond the
short identifiers/one-line definitions needed to map rustnarok's effect ids to
our data. It is deterministic: keys are sorted and source lines are recorded so
the audit report can cite them.

Usage:
    ./tools/extract_rustnarok_facts.py
    ./tools/extract_rustnarok_facts.py --check
"""

from __future__ import annotations

import argparse
import ast
import json
import re
import subprocess
import sys
from pathlib import Path

PIN = "76600595363affc96d5995bf4baa790d05a9593c"
REPO = "https://github.com/kindraywind/rustnarok-client.git"
DEFAULT_CHECKOUT = Path("/tmp/rustnarok-client")
DEFAULT_OUTPUT_DIR = Path("tools")

EFFECT_TABLE_KEYS = {
    "str", "part", "ring", "spr", "func", "length", "wav", "model", "rsm", "minid",
    "hit", "caster", "cast", "struck", "used", "ground", "status",
    "unit", "unitw", "wdelay", "unitr", "area", "every", "keep", "trap",
}
SKILL_EFFECT_KEYS = {"hit", "caster", "cast", "struck", "used", "ground"}
MOTION_TABLES = ("ATTACK", "THROW", "THIRD_ATTACK", "PICK_UP", "KEEP", "CAST", "STATES")


class ExtractionError(SystemExit):
    pass


def run(*command: str, cwd: Path | None = None) -> str:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        raise ExtractionError(f"command failed: {' '.join(command)}\n{result.stderr.strip()}")
    return result.stdout.strip()


def ensure_checkout(checkout: Path) -> str:
    """Return the pinned checkout commit, cloning/re-checking-out when needed."""
    if not (checkout / ".git").is_dir():
        checkout.parent.mkdir(parents=True, exist_ok=True)
        run("git", "clone", "--filter=blob:none", REPO, str(checkout))

    head = run("git", "rev-parse", "HEAD", cwd=checkout)
    if head != PIN:
        # The pin may be reachable from origin/main; fetch it explicitly.
        run("git", "fetch", "--depth", "1", "origin", PIN, cwd=checkout)
        run("git", "checkout", "--detach", PIN, cwd=checkout)
        head = run("git", "rev-parse", "HEAD", cwd=checkout)
    if head != PIN:
        raise ExtractionError(f"rustnarok checkout is at {head}, expected pinned {PIN}")
    return head


def read_lines(path: Path) -> list[str]:
    return path.read_text(encoding="utf-8").splitlines()


def json_dump(path: Path, payload: object) -> None:
    text = json.dumps(payload, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
    path.write_text(text, encoding="utf-8")


def source_header(commit: str, files: list[str]) -> dict[str, object]:
    return {
        "provenance": "rustnarok-client source facts, extracted from the pinned checkout",
        "commit": commit,
        "repository": REPO,
        "files": files,
    }


def extract_effects(checkout: Path, commit: str) -> dict[str, object]:
    path = checkout / "src" / "fx" / "effects.txt"
    effects: dict[int, dict[str, object]] = {}
    skill_links: dict[int, set[int]] = {}
    status_links: dict[int, set[int]] = {}

    def effect(effect_id: int) -> dict[str, object]:
        return effects.setdefault(effect_id, {"definitions": [], "skills": set(), "statuses": set()})

    for line_number, raw in enumerate(read_lines(path), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split()
        kind = fields[0]
        if kind not in EFFECT_TABLE_KEYS:
            continue

        effect_id: int | None = None
        if kind in {"unit", "unitw", "wdelay", "unitr", "area", "every", "keep", "trap"}:
            if len(fields) > 2 and fields[2].lstrip("-").isdigit():
                effect_id = int(fields[2])
        elif len(fields) > 1 and fields[1].lstrip("-").isdigit():
            effect_id = int(fields[1])

        if effect_id is not None:
            record = effect(effect_id)
            record["definitions"].append({"kind": kind, "line": line_number, "text": line})

        if kind in SKILL_EFFECT_KEYS and len(fields) > 1 and fields[1].lstrip("-").isdigit():
            skill_id = int(fields[1])
            for token in fields[2:]:
                if token.lstrip("-").isdigit() and int(token) >= 0:
                    skill_links.setdefault(skill_id, set()).add(int(token))
                    effect(int(token))["skills"].add(skill_id)

        if kind == "status" and len(fields) > 2 and fields[1].isdigit() and fields[2].isdigit():
            status_id = int(fields[1])
            visual_id = int(fields[2])
            status_links.setdefault(status_id, set()).add(visual_id)
            effect(visual_id)["statuses"].add(status_id)

    payload_effects = []
    for effect_id in sorted(effects):
        record = effects[effect_id]
        payload_effects.append(
            {
                "id": effect_id,
                "definitions": sorted(record["definitions"], key=lambda item: item["line"]),
                "skills": sorted(record["skills"]),
                "statuses": sorted(record["statuses"]),
            }
        )

    return {
        **source_header(commit, ["src/fx/effects.txt"]),
        "effect_count": len(payload_effects),
        "definition_count": sum(len(item["definitions"]) for item in payload_effects),
        "skill_link_count": len(skill_links),
        "status_visual_count": len(status_links),
        "effects": payload_effects,
        "skill_links": [
            {"skill": skill_id, "effect_ids": sorted(ids)}
            for skill_id, ids in sorted(skill_links.items())
        ],
        "status_links": [
            {"status": status_id, "effect_ids": sorted(ids)}
            for status_id, ids in sorted(status_links.items())
        ],
    }


def parse_numeric_array(text: str, name: str) -> list[dict[str, object]]:
    match = re.search(rf"const {name}: \[u16; \d+\] = \[(.*?)\];", text, re.S)
    if not match:
        raise ExtractionError(f"missing motion table {name}")
    start_line = text[: match.start()].count("\n") + 1
    records = []
    for offset, token in enumerate(re.findall(r"\d+", match.group(1))):
        line = start_line + match.group(1)[: match.group(1).find(token)].count("\n") if token in match.group(1) else start_line
        records.append({"skill": int(token), "table": name, "source_line": line})
    return records


def parse_cast_table(text: str) -> list[dict[str, object]]:
    match = re.search(r"const CAST: \[\(u16, i8\); \d+\] = \[(.*?)\];", text, re.S)
    if not match:
        raise ExtractionError("missing motion table CAST")
    start_line = text[: match.start()].count("\n") + 1
    records = []
    for item in re.finditer(r"\((\d+),\s*(-?\d+)\)", match.group(1)):
        line = start_line + match.group(1)[: item.start()].count("\n")
        records.append(
            {
                "skill": int(item.group(1)),
                "table": "CAST",
                "value": int(item.group(2)),
                "source_line": line,
            }
        )
    return records


def parse_states_table(text: str) -> list[dict[str, object]]:
    match = re.search(r"const STATES: \[\(u16, u8\); \d+\] = \[(.*?)\];", text, re.S)
    if not match:
        raise ExtractionError("missing motion table STATES")
    start_line = text[: match.start()].count("\n") + 1
    records = []
    for item in re.finditer(r"\((\d+),\s*(\d+)\)", match.group(1)):
        line = start_line + match.group(1)[: item.start()].count("\n")
        records.append(
            {
                "skill": int(item.group(1)),
                "table": "STATES",
                "value": int(item.group(2)),
                "source_line": line,
            }
        )
    return records


def extract_motions(checkout: Path, commit: str) -> dict[str, object]:
    path = checkout / "src" / "skill_motions.rs"
    text = path.read_text(encoding="utf-8")

    tables: dict[str, list[dict[str, object]]] = {}
    for name in ("ATTACK", "THROW", "THIRD_ATTACK", "PICK_UP", "KEEP"):
        tables[name] = parse_numeric_array(text, name)
    tables["CAST"] = parse_cast_table(text)
    tables["STATES"] = parse_states_table(text)

    return {
        **source_header(commit, ["src/skill_motions.rs"]),
        "table_entry_counts": {name: len(tables[name]) for name in MOTION_TABLES},
        "tables": tables,
    }


def extract_statuses(checkout: Path, commit: str) -> dict[str, object]:
    icons_path = checkout / "src" / "status_icons.txt"
    rows: dict[int, dict[str, object]] = {}
    for line_number, raw in enumerate(read_lines(icons_path), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split()
        if len(fields) >= 2 and fields[0].isdigit():
            status_id = int(fields[0])
            rows[status_id] = {
                "id": status_id,
                "icon": fields[1],
                "visual_effect_ids": [],
                "source_file": "src/status_icons.txt",
                "source_line": line_number,
            }

    effects_path = checkout / "src" / "fx" / "effects.txt"
    for line_number, raw in enumerate(read_lines(effects_path), start=1):
        fields = raw.strip().split()
        if len(fields) < 3 or fields[0] != "status" or not fields[1].isdigit() or not fields[2].isdigit():
            continue
        status_id = int(fields[1])
        row = rows.setdefault(
            status_id,
            {
                "id": status_id,
                "icon": None,
                "visual_effect_ids": [],
                "source_file": "src/fx/effects.txt",
                "source_line": line_number,
            },
        )
        row["visual_effect_ids"] = sorted(set(row["visual_effect_ids"]) | {int(fields[2])})

    return {
        **source_header(commit, ["src/status_icons.txt", "src/fx/effects.txt"]),
        "status_count": len(rows),
        "with_icon": sum(1 for row in rows.values() if row["icon"] is not None),
        "with_visual_effect": sum(1 for row in rows.values() if row["visual_effect_ids"]),
        "statuses": [rows[status_id] for status_id in sorted(rows)],
    }


def parse_constants(actor_text: str) -> dict[str, int]:
    """Evaluate the small integer/bitmask constants used by the actor facts."""
    raw_constants: dict[str, str] = {}
    for match in re.finditer(r"const\s+([A-Z][A-Z0-9_]*)\s*:\s*u32\s*=\s*(.*?);", actor_text, re.S):
        raw_constants[match.group(1)] = " ".join(match.group(2).split())

    arrays: dict[str, list[int]] = {}
    for match in re.finditer(r"const\s+([A-Z][A-Z0-9_]*)\s*:\s*\[u32;\s*\d+\]\s*=\s*\[(.*?)\];", actor_text, re.S):
        arrays[match.group(1)] = [int(token, 0) for token in re.findall(r"0x[0-9A-Fa-f_]+|\d+", match.group(2))]

    values: dict[str, int] = {}
    pending = dict(raw_constants)
    while pending:
        progressed = False
        for name, expression in list(pending.items()):
            names = {token for token in re.findall(r"[A-Z][A-Z0-9_]*", expression)}
            names.discard("DRAGONS")
            array_refs = re.findall(r"([A-Z][A-Z0-9_]*)\[(\d+)\]", expression)
            if any(ref not in arrays for ref, _ in array_refs):
                continue
            if any(ref not in values for ref in names):
                continue
            resolved = expression
            for ref, index in array_refs:
                resolved = resolved.replace(f"{ref}[{index}]", str(arrays[ref][int(index)]))
            for ref in names:
                resolved = re.sub(rf"\b{ref}\b", str(values[ref]), resolved)
            try:
                values[name] = int(ast.literal_eval(resolved.replace("_", "")))
            except (SyntaxError, ValueError):
                continue
            del pending[name]
            progressed = True
        if not progressed:
            break
    return {name: values[name] for name in sorted(values)}


def parse_job_sprites(path: Path) -> list[dict[str, object]]:
    rows = []
    for line_number, raw in enumerate(read_lines(path), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) < 5:
            continue
        job = int(fields[0])
        weapon = fields[4]
        if weapon == "-":
            weapon_folder = None
            weapon_prefix = None
        elif "\\" in weapon:
            weapon_folder, weapon_prefix = weapon.split("\\", 1)
        else:
            weapon_folder = weapon
            weapon_prefix = weapon
        rows.append(
            {
                "job": job,
                "body": fields[1],
                "imf": fields[2],
                "palette": fields[3],
                "weapon_folder": weapon_folder,
                "weapon_prefix": weapon_prefix,
                "source_line": line_number,
            }
        )
    return rows


def extract_actors(checkout: Path, commit: str) -> dict[str, object]:
    actor_path = checkout / "src" / "actor.rs"
    actor_text = actor_path.read_text(encoding="utf-8")
    constants = parse_constants(actor_text)

    named_carts_match = re.search(r"const NAMED_CARTS: \[&str; \d+\] = \[(.*?)\];", actor_text, re.S)
    named_carts = re.findall(r'"([^"]+)"', named_carts_match.group(1)) if named_carts_match else []
    super_novice_match = re.search(r"const SUPER_NOVICE_JOBS: \[u16; \d+\] = \[(.*?)\];", actor_text, re.S)
    super_novice_jobs = [int(value) for value in re.findall(r"\d+", super_novice_match.group(1))] if super_novice_match else []
    baby_match = re.search(r"fn baby_job\(job: u16\) -> bool \{\s*matches!\(job, (.*?)\)\s*\}", actor_text, re.S)
    baby_ranges = []
    if baby_match:
        for start, end in re.findall(r"(\d+)\.\.=(\d+)", baby_match.group(1)):
            baby_ranges.append([int(start), int(end)])
    body_scale_match = re.search(r"fn body_scale\(job: u16\) -> f32 \{(.*?)\n\}", actor_text, re.S)
    body_scale_source = " ".join(body_scale_match.group(1).split()) if body_scale_match else None

    job_sprites = parse_job_sprites(checkout / "src" / "job_sprites.txt")

    option_names = [
        "WEDDING", "UNION", "XMAS", "SUMMER", "HANBOK", "OKTOBERFEST", "RIDING",
        "WOLF_RIDING", "MADOGEAR", "ALL_RIDING_LOOK", "OPTION_FALCON", "OPTION_WUG",
        "OPTION_WUGRIDER", "OPTION_HIDE", "OPTION_CLOAK", "OPTION_INVISIBLE",
        "OPTION_CHASEWALK", "OPTION_ORCISH", "BODY_OPTIONS", "MOUNTS",
    ]
    option_bits = {name: constants[name] for name in option_names if name in constants}

    outfit_bits = {}
    outfit_match = re.search(r"\[\(XMAS, 26\), \(SUMMER, 27\), \(HANBOK, 28\), \(OKTOBERFEST, 29\)\]", actor_text)
    if outfit_match:
        outfit_bits = {
            name: int(job)
            for name, job in re.findall(r"\((XMAS|SUMMER|HANBOK|OKTOBERFEST), (\d+)\)", outfit_match.group(0))
        }
        if "WEDDING" in constants:
            outfit_bits["WEDDING"] = 22

    return {
        **source_header(commit, ["src/actor.rs", "src/job_sprites.txt"]),
        "job_sprite_count": len(job_sprites),
        "job_sprites": job_sprites,
        "cart": {
            "push_cart_status": 673,
            "named_carts": named_carts,
            "super_novice_jobs": super_novice_jobs,
            "baby_job_ranges": baby_ranges,
            "cart_path_rules": {
                "sprite_folder": "data\\\\sprite\\\\이팩트\\\\",
                "default": "손수레",
                "numbered": "손수레{n} for kind 2..9 (n = kind - 1)",
                "named": "types 10..13 use named_carts",
                "super_novice": "슈노손수레 for all super_novice_jobs",
                "baby": "numbered kind>=6 appends 베이비; named carts get _미니; default ACT is 베이비<name>",
                "act": "same stem as the sprite, except the baby default ACT",
            },
            "rope_cells": 1,
            "frames_per_world_unit": 0.92,
            "source_lines": {
                "constants": _line_of(actor_text, "pub const PUSH_CART"),
                "cart_path": _line_of(actor_text, "pub fn cart_path"),
                "rope": _line_of(actor_text, "pub const CART_ROPE"),
                "frames": _line_of(actor_text, "const CART_FRAMES"),
            },
        },
        "options": {
            "body_options_bits": option_bits,
            "outfit_jobs": outfit_bits,
            "shadow_options": {
                name: constants[name]
                for name in ("OPTION_HIDE", "OPTION_CLOAK", "OPTION_INVISIBLE", "OPTION_CHASEWALK")
                if name in constants
            },
            "source_lines": {
                "constants": _line_of(actor_text, "const WEDDING"),
                "body_options": _line_of(actor_text, "pub const BODY_OPTIONS"),
                "sprite_job": _line_of(actor_text, "pub fn sprite_job"),
            },
        },
        "size_statuses": {
            "change_size": 1420,
            "change_size_monster": 1421,
            "unit": "percent (val1 on ZC_MSG_STATE_CHANGE3)",
            "body_scale_rule": body_scale_source,
            "source_lines": {
                "change_size": _line_of(actor_text, "pub const CHANGE_SIZE"),
                "body_scale": _line_of(actor_text, "fn body_scale"),
            },
        },
    }


def _line_of(text: str, needle: str) -> int | None:
    index = text.find(needle)
    return text.count("\n", 0, index) + 1 if index >= 0 else None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--checkout", default=str(DEFAULT_CHECKOUT))
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    parser.add_argument("--check", action="store_true", help="fail if the committed facts are out of date")
    args = parser.parse_args()

    checkout = Path(args.checkout)
    commit = ensure_checkout(checkout)
    output_dir = Path(args.output_dir)

    documents = {
        "rustnarok_effects_facts.json": extract_effects(checkout, commit),
        "rustnarok_motion_facts.json": extract_motions(checkout, commit),
        "rustnarok_status_facts.json": extract_statuses(checkout, commit),
        "rustnarok_actor_facts.json": extract_actors(checkout, commit),
    }

    stale = []
    for name, payload in documents.items():
        path = output_dir / name
        expected = json.dumps(payload, indent=1, sort_keys=True, ensure_ascii=False) + "\n"
        current = path.read_text(encoding="utf-8") if path.is_file() else ""
        if args.check:
            if current != expected:
                stale.append(str(path))
            continue
        path.write_text(expected, encoding="utf-8")

    if args.check:
        if stale:
            print("error: rustnarok facts are out of date; run tools/extract_rustnarok_facts.py", file=sys.stderr)
            for path in stale:
                print(f"  {path}", file=sys.stderr)
            return 1
        print(f"rustnarok facts are up to date (pinned {PIN}; {len(documents)} files)")
        return 0

    print(f"wrote {len(documents)} rustnarok facts files for pinned {PIN}:")
    for name, payload in documents.items():
        print(f"  {output_dir / name}: {len(json.dumps(payload, sort_keys=True))} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
