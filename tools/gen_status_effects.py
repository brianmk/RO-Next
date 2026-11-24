#!/usr/bin/env python3
"""Generate the client's data-driven status-effect table.

The client resolves status names, icons, categories, value-label formats,
refresh/stack policy and persistent status auras from **one** data file:

    ro-next/archive/data/client/status_effects.ron

loaded at runtime through the regular `GameFileLoader` (the same loader that
reads the GRF archives), so a server can override it inside its own
`data.grf`/`CondaRO.grf`/`data\\` folder without rebuilding the client. The same
file is embedded with `include_str!` as the compiled fallback, so the archive
override and the compiled table can never drift apart.

Input is the committed facts document

    tools/data/status_facts.json

produced by `tools/extract_status_facts.py` from the rAthena server sources,
the official 2019 client Lua tables (`stateiconinfo.lub`,
`stateiconimginfo.lub`, `efstids.lub`), the vendored roBrowserLegacy classic
status facts and the curated overlay `tools/data/status_effect_custom.ron`. Running
this generator (and its `--check` mode) needs no game data and no server tree,
so it is safe to run in CI.

Schema (RON, one entry per status id, sorted ascending):

    (
        provenance: "...",
        version: 1,
        value_enums: (
            element: ["Neutral", "Water", ...],   // value format `Element`
        ),
        statuses: [
            (
                id: 10,
                name: "Blessing",                 // omitted when no safe English name exists
                icon: "블레싱.tga",                 // file name inside data\\texture\\effect\\
                category: Buff,                   // Buff | Debuff | Toggle | System
                refresh: Replace,                 // Replace | Keep
                stack: Single,
                value_labels: [                   // only statuses the server sends values for
                    (slot: 1, label: "Element", format: Element),
                ],
                aura: (                           // optional persistent glow
                    kind: PersistentGlow,
                    textures: (ground: "...", ring: "...", body_frames: ["..."], body_glow: "..."),
                    tint: (ground: (r, g, b), body: (...), glow: (...)),
                    radii: (ground: 11.0, ring: 7.5, body: 26.0, body_glow: 15.0),
                    timing: (
                        body_frame_rate: 8.0,
                        ring_rotation_speed: 0.8,
                        body_rotation_speed: 0.35,
                        body_camera_offset: 2.5,
                        pulse_speed: 2.2,
                        pulse_amount: 0.22,
                    ),
                    alpha: (ground: 0.42, ring: 0.65, body: 0.6, body_glow: 0.38),
                    height: 13.0,
                ),
            ),
        ],
    )

The Rust schema lives in `ro-next/src/world/library/status_effects.rs`; the
`--check` flag is the drift guard and `ro-next --check-status-effects`
validates the schema (and an archive override) at runtime.

Usage (from the repository root):

    ./tools/gen_status_effects.py            # regenerate the data file
    ./tools/gen_status_effects.py --check    # fail if out of date
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_FACTS = REPO / "tools/data/status_facts.json"
DEFAULT_OUTPUT = REPO / "ro-next/archive/data/client/status_effects.ron"

#: Must match the Rust enums of `world/library/status_effects.rs`.
CATEGORIES = ("Buff", "Debuff", "Toggle", "System")
REFRESH_POLICIES = ("Replace", "Keep")
STACK_POLICIES = ("Single",)
VALUE_FORMATS = ("Element", "Percent", "Number", "Id")
AURA_KINDS = ("PersistentGlow",)


class GeneratorError(ValueError):
    pass


def ron_string(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def ron_float(value: float) -> str:
    if value == 0.0:
        return "0.0"
    if float(value).is_integer():
        return f"{value:.1f}"
    return repr(float(value))


def ron_colour(colour: list[float]) -> str:
    return "(" + ", ".join(ron_float(component) for component in colour) + ")"


def validate_facts(facts: dict[str, object]) -> None:
    """Reject inconsistent facts before they can become client data."""
    schema = facts.get("schema")
    if schema != 1:
        raise GeneratorError(f"unsupported facts schema {schema!r} (expected 1)")

    statuses = facts.get("statuses")
    if not isinstance(statuses, list) or not statuses:
        raise GeneratorError("the facts document contains no statuses")

    previous_id = -1
    for row in statuses:
        assert isinstance(row, dict)
        status_id = int(row["id"])
        if status_id <= previous_id:
            raise GeneratorError(f"status ids are not strictly ascending ({status_id} after {previous_id})")
        previous_id = status_id

        name = row.get("name")
        if name is not None:
            assert isinstance(name, str)
            if not name or not name.isascii() or "#" in name or not any(character.isalpha() for character in name):
                raise GeneratorError(f"status {status_id}: unusable name {name!r}")

        icon = row.get("icon")
        if icon is not None:
            assert isinstance(icon, str)
            if not icon.lower().endswith(".tga"):
                raise GeneratorError(f"status {status_id}: status icons must be .tga files, got {icon!r}")

        category = row.get("category")
        if category not in CATEGORIES:
            raise GeneratorError(f"status {status_id}: unknown category {category!r}")
        if row.get("refresh") not in REFRESH_POLICIES:
            raise GeneratorError(f"status {status_id}: unknown refresh policy {row.get('refresh')!r}")
        if row.get("stack") not in STACK_POLICIES:
            raise GeneratorError(f"status {status_id}: unknown stack policy {row.get('stack')!r}")

        labels = row.get("value_labels")
        if labels is not None:
            assert isinstance(labels, list)
            slots: set[int] = set()
            for label in labels:
                assert isinstance(label, dict)
                slot = int(label["slot"])
                if slot not in (1, 2, 3) or slot in slots:
                    raise GeneratorError(f"status {status_id}: invalid/duplicate value slot {slot}")
                slots.add(slot)
                if label.get("format") not in VALUE_FORMATS:
                    raise GeneratorError(f"status {status_id}: unknown value format {label.get('format')!r}")
                text = label.get("label")
                if not isinstance(text, str) or not text.isascii() or not text:
                    raise GeneratorError(f"status {status_id}: unusable value label {text!r}")

        aura = row.get("aura")
        if aura is not None:
            assert isinstance(aura, dict)
            validate_aura(status_id, aura)


def validate_aura(status_id: int, aura: dict[str, object]) -> None:
    if aura.get("kind") not in AURA_KINDS:
        raise GeneratorError(f"status {status_id}: unknown aura kind {aura.get('kind')!r}")
    if not aura.get("source"):
        raise GeneratorError(f"status {status_id}: aura has no documented source")

    textures = aura["textures"]
    assert isinstance(textures, dict)
    for key in ("ground", "ring", "body_glow"):
        texture = textures.get(key)
        if not isinstance(texture, str) or not texture.lower().endswith((".tga", ".bmp")):
            raise GeneratorError(f"status {status_id}: aura {key} texture is invalid: {texture!r}")
    frames = textures.get("body_frames")
    if not isinstance(frames, list) or not frames:
        raise GeneratorError(f"status {status_id}: aura has no body frames")
    for frame in frames:
        if not isinstance(frame, str) or not frame.lower().endswith((".tga", ".bmp")):
            raise GeneratorError(f"status {status_id}: aura body frame is invalid: {frame!r}")

    tint = aura["tint"]
    assert isinstance(tint, dict)
    for key, colour in tint.items():
        if not isinstance(colour, list) or len(colour) != 3:
            raise GeneratorError(f"status {status_id}: aura tint {key} is invalid")
        for component in colour:
            if not (0.0 <= float(component) <= 1.0):
                raise GeneratorError(f"status {status_id}: aura tint {key} is outside 0..1")

    radii = aura["radii"]
    assert isinstance(radii, dict)
    for key, radius in radii.items():
        if float(radius) <= 0.0 or float(radius) > 100.0:
            raise GeneratorError(f"status {status_id}: aura radius {key} is out of range ({radius})")

    alpha = aura["alpha"]
    assert isinstance(alpha, dict)
    for key, value in alpha.items():
        if not (0.0 <= float(value) <= 1.0):
            raise GeneratorError(f"status {status_id}: aura alpha {key} is outside 0..1")

    timing = aura["timing"]
    assert isinstance(timing, dict)
    for key in ("body_frame_rate", "ring_rotation_speed", "body_rotation_speed", "body_camera_offset", "pulse_speed"):
        if float(timing[key]) < 0.0:
            raise GeneratorError(f"status {status_id}: aura timing {key} is negative")
    if not (0.0 <= float(timing["pulse_amount"]) <= 1.0):
        raise GeneratorError(f"status {status_id}: aura pulse amount is outside 0..1")
    if not (0.0 < float(aura["height"]) <= 100.0):
        raise GeneratorError(f"status {status_id}: aura height is out of range ({aura['height']})")


def render(facts: dict[str, object]) -> str:
    statuses = facts["statuses"]
    elements = facts.get("elements", [])
    assert isinstance(statuses, list)
    assert isinstance(elements, list)
    element_names = [entry["name"] for entry in sorted(elements, key=lambda entry: entry["id"])]
    assert all(isinstance(name, str) for name in element_names)

    counts = facts.get("provenance", {}).get("counts", {})
    assert isinstance(counts, dict)

    lines = [
        "// Status-effect presentation data for the client.",
        "//",
        "// Generated by `tools/gen_status_effects.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_status_effects.py`",
        "// Drift check:      `./tools/gen_status_effects.py --check`",
        "// Schema check:     `ro-next --check-status-effects`",
        "//",
        "// One entry per EFST status id, sorted by id. The client resolves the buff",
        "// bar name/tooltip, the official status icon (transparent `effect\\*.tga`,",
        "// not the opaque skill icon), the buff/debuff category, the value-label",
        "// lines of the statuses whose server packets carry values, the refresh/",
        "// stack policy and the persistent world aura from this table.",
        "//",
        "// Sources: rAthena `src/map/status.hpp` (`efst_type`, `sc_type`) and",
        "// `db/pre-re/status.yml`; the official client `stateiconinfo.lub`,",
        "// `stateiconimginfo.lub` and `efstids.lub`; the reverse-engineered",
        "// roBrowserLegacy classic status table (`tools/data/status_info_robrowser.json`);",
        "// the curated overlay `tools/data/status_effect_custom.ron` (extra names, icon",
        "// choices, SendVal value semantics and persistent aura specs). Extracted",
        "// facts: `tools/data/status_facts.json`.",
        "//",
        "// Categories: Buff, Debuff, Toggle, System. Refresh policies: Replace,",
        "// Keep. Stack policies: Single. Aura kinds: PersistentGlow.",
        "// Icons are file names inside `data\\texture\\effect\\`; aura textures are",
        "// paths relative to `data\\texture\\`.",
        "//",
        f"// {counts.get('client_state_info', 0)} client titles, {counts.get('client_icons', 0)} explicit client",
        f"// icons and {counts.get('robrowser_icons', 0)} classic icons were extracted; {len(statuses)} statuses",
        f"// have presentation data and {counts.get('auras', 0)} of them have an aura.",
        "(",
        '    provenance: "Status-effect presentation data: official client/server names, icons, categories, value labels and persistent aura specs",',
        "    version: 1,",
        "    value_enums: (",
        "        element: [" + ", ".join(ron_string(name) for name in element_names) + "],",
        "    ),",
        "    statuses: [",
    ]

    for row in statuses:
        assert isinstance(row, dict)
        lines.append("        (")
        lines.append(f"            id: {int(row['id'])},")
        if row.get("name") is not None:
            lines.append(f"            name: {ron_string(str(row['name']))},")
        if row.get("icon") is not None:
            lines.append(f"            icon: {ron_string(str(row['icon']))},")
        lines.append(f"            category: {row['category']},")
        lines.append(f"            refresh: {row['refresh']},")
        lines.append(f"            stack: {row['stack']},")

        labels = row.get("value_labels")
        if labels is not None:
            assert isinstance(labels, list)
            lines.append("            value_labels: [")
            for label in sorted(labels, key=lambda label: int(label["slot"])):
                lines.append(
                    f"                (slot: {int(label['slot'])}, label: {ron_string(str(label['label']))}, format: {label['format']}),"
                )
            lines.append("            ],")

        aura = row.get("aura")
        if aura is not None:
            assert isinstance(aura, dict)
            textures = aura["textures"]
            tint = aura["tint"]
            radii = aura["radii"]
            timing = aura["timing"]
            alpha = aura["alpha"]
            assert isinstance(textures, dict) and isinstance(tint, dict) and isinstance(radii, dict)
            assert isinstance(timing, dict) and isinstance(alpha, dict)
            lines.append("            aura: (")
            lines.append(f"                kind: {aura['kind']},")
            lines.append("                textures: (")
            lines.append(f"                    ground: {ron_string(str(textures['ground']))},")
            lines.append(f"                    ring: {ron_string(str(textures['ring']))},")
            lines.append("                    body_frames: [" + ", ".join(ron_string(str(frame)) for frame in textures["body_frames"]) + "],")
            lines.append(f"                    body_glow: {ron_string(str(textures['body_glow']))},")
            lines.append("                ),")
            lines.append("                tint: (")
            for key in ("ground", "body", "glow"):
                lines.append(f"                    {key}: {ron_colour(tint[key])},")
            lines.append("                ),")
            lines.append(
                "                radii: ("
                + ", ".join(f"{key}: {ron_float(float(radii[key]))}" for key in ("ground", "ring", "body", "body_glow"))
                + "),"
            )
            lines.append("                timing: (")
            for key in ("body_frame_rate", "ring_rotation_speed", "body_rotation_speed", "body_camera_offset", "pulse_speed", "pulse_amount"):
                lines.append(f"                    {key}: {ron_float(float(timing[key]))},")
            lines.append("                ),")
            lines.append(
                "                alpha: ("
                + ", ".join(f"{key}: {ron_float(float(alpha[key]))}" for key in ("ground", "ring", "body", "body_glow"))
                + "),"
            )
            lines.append(f"                height: {ron_float(float(aura['height']))},")
            lines.append("            ),")

        lines.append("        ),")

    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--facts", default=str(DEFAULT_FACTS), help="extracted facts JSON")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="generated RON data file")
    args = parser.parse_args()

    facts_path = Path(args.facts)
    if not facts_path.is_file():
        raise SystemExit(f"missing facts file: {facts_path} (run ./tools/extract_status_facts.py first)")
    facts = json.loads(facts_path.read_text(encoding="utf-8"))
    validate_facts(facts)

    content = render(facts)
    output = Path(args.output)
    statuses = facts["statuses"]
    assert isinstance(statuses, list)
    values = sum(1 for row in statuses if row.get("value_labels"))
    auras = sum(1 for row in statuses if row.get("aura"))
    named = sum(1 for row in statuses if row.get("name"))
    iconed = sum(1 for row in statuses if row.get("icon"))

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_status_effects.py", file=sys.stderr)
            return 1
        print(
            f"{output} is up to date ({len(statuses)} statuses: {named} names, {iconed} icons, "
            f"{values} value labels, {auras} auras)"
        )
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {len(statuses)} statuses ({named} names, {iconed} icons, {values} value labels, {auras} auras)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
