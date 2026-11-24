#!/usr/bin/env python3
"""Generate the unified skill effect descriptor data file.

The client plays skill effects from **one** data file:

    korangar/archive/data/client/skill_effects.ron

loaded at runtime through the regular `GameFileLoader` (the same loader that
reads the GRF archives), so a server can override it inside its own
`data.grf`/`CondaRO.grf`/`data\\` folder without rebuilding the client. The same
file is embedded with `include_str!` as the compiled fallback, so the archive
override and the compiled table can never drift apart.

The file is the merge of these sources:

1. `tools/data/skill_effects_official.ron` (optional, `--official`) - the complete
   official `EffectTable` layer lists (one entry per skill/phase, ordered
   layers). This is the "full official rows" generator deliverable; when it
   contains a skill/phase pair it supersedes the legacy rows for that pair.
1b. `tools/data/skill_effects_rodecomp_hardcoded.ron` (optional,
   `--rodecomp-hardcoded`) - rows extracted from the client decompilation for
   the effects the 2019 table gives an id but no data layers (their visuals are
   hard-coded C++; see `tools/gen_rodecomp_effect_layers.py`). The merge only
   adds layers whose asset is not already present for that skill/phase.
2. `tools/data/skill_effects_custom.ron` - the hand-written (older client) visuals
   migrated out of `skill_cast_effect.rs` (Blessing angel, Angelus, Inc AGI,
   Concentration, Hammer Fall, Warp/Heal cast tints, Zen/Fury auras, body
   flash). Custom layers are additive: only a layer that references the same
   asset as an official/legacy layer is folded into that layer.
3. The legacy per-kind tables that predate the pipeline, used as the data
   source until the official multi-layer generator lands:
   * `korangar/archive/data/client/skill_effect_assets.ron` (`.str` rows),
   * `korangar/archive/data/client/skill_sprite_effects.ron` (`.spr` rows),
   * `korangar/archive/data/client/skill_procedural_effects.ron`
     (HitFlash/Projectile/GroundAura primitives).

Schema (RON, one entry per skill/phase, layers spawned in order):

    (
        provenance: "...",
        effects: [
            (
                skill: 34,
                phase: Success,               // Ground|Cast|Success|Release|Hit|Caster
                cast_circle_color: (255, 230, 160),   // optional u8 RGB
                action: Idle,                 // optional CasterAction override
                layers: [
                    (kind: Str, anchor: Overhead, file: "angel.str", scale: 1.0),
                    (kind: Spr, head: true, attached_entity: true, files: ["이팩트\\축복"], duration_ms: 1500),
                    (kind: Texture, attached_entity: false, file: "effect\\pok2.tga", size: 140.0),
                ],
            ),
        ],
    )

See `tools/data/skill_effects_custom.ron` for the full field documentation. The
Rust schema lives in `korangar/src/world/library/skill_effects.rs`; the
`--check` flag is the drift guard and `korangar --check-skill-effects` validates
the schema (and the archive override) at runtime.

Usage:
    ./tools/gen_skill_effects.py                          # regenerate the data file
    ./tools/gen_skill_effects.py --check                  # fail if out of date
    ./tools/gen_skill_effects.py --official tools/data/skill_effects_official.ron
    ./tools/gen_skill_effects.py --validate-assets /path/to/client/game
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from pathlib import Path

DEFAULT_OUTPUT = Path("korangar/archive/data/client/skill_effects.ron")
DEFAULT_CUSTOM = Path("tools/data/skill_effects_custom.ron")
DEFAULT_OFFICIAL = Path("tools/data/skill_effects_official.ron")
DEFAULT_RODECOMP_HARDCODED = Path("tools/data/skill_effects_rodecomp_hardcoded.ron")
DEFAULT_RODECOMP = Path("tools/data/skill_effects_rodecomp.ron")
DEFAULT_OVERRIDES = Path("tools/data/skill_effects_overrides.ron")
LEGACY_STR = Path("korangar/archive/data/client/skill_effect_assets.ron")
LEGACY_SPR = Path("korangar/archive/data/client/skill_sprite_effects.ron")
LEGACY_PROCEDURAL = Path("korangar/archive/data/client/skill_procedural_effects.ron")

#: Phase order in the generated file (matches the client enum).
PHASES = ["Ground", "Cast", "Success", "Release", "Hit", "Caster"]
#: Legacy `.str` anchor -> unified anchor.
LEGACY_STR_ANCHORS = {"Caster": "Caster", "Target": "Target", "Travel": "Travel", "Auto": "Auto"}
#: Legacy `.spr` anchor -> unified anchor.
LEGACY_SPR_ANCHORS = {"Caster": "Caster", "Target": "Target", "Ground": "Ground"}
#: Legacy procedural primitive -> unified layer kind.
PRIMITIVE_KINDS = {"HitFlash": "Texture", "Projectile": "Primitive", "GroundAura": "Cylinder"}


# ---------------------------------------------------------------------------
# Minimal RON reader (the data files are simple tuples/maps/lists/scalars).
# ---------------------------------------------------------------------------


class RonError(ValueError):
    pass


def strip_comments(text: str) -> str:
    out: list[str] = []
    index = 0
    in_string = False
    while index < len(text):
        character = text[index]
        if in_string:
            out.append(character)
            if character == "\\" and index + 1 < len(text):
                out.append(text[index + 1])
                index += 2
                continue
            if character == '"':
                in_string = False
            index += 1
            continue

        if character == '"':
            in_string = True
            out.append(character)
            index += 1
            continue

        if character == "/" and index + 1 < len(text) and text[index + 1] == "/":
            while index < len(text) and text[index] != "\n":
                index += 1
            continue

        out.append(character)
        index += 1

    return "".join(out)


class RonReader:
    def __init__(self, text: str) -> None:
        self.text = strip_comments(text)
        self.index = 0

    def error(self, message: str) -> RonError:
        line = self.text.count("\n", 0, self.index) + 1
        return RonError(f"line {line}: {message}")

    def skip_whitespace(self) -> None:
        while self.index < len(self.text) and self.text[self.index].isspace():
            self.index += 1

    def peek(self) -> str:
        self.skip_whitespace()
        if self.index >= len(self.text):
            raise self.error("unexpected end of file")
        return self.text[self.index]

    def expect(self, character: str) -> None:
        if self.peek() != character:
            raise self.error(f"expected {character!r}, got {self.text[self.index]!r}")
        self.index += 1

    def read_identifier(self) -> str:
        self.skip_whitespace()
        start = self.index
        while self.index < len(self.text) and (self.text[self.index].isalnum() or self.text[self.index] in "_-"):
            self.index += 1
        if start == self.index:
            raise self.error("expected an identifier")
        return self.text[start : self.index]

    def read_string(self) -> str:
        self.expect('"')
        value: list[str] = []
        while True:
            if self.index >= len(self.text):
                raise self.error("unterminated string")
            character = self.text[self.index]
            self.index += 1
            if character == '"':
                return "".join(value)
            if character == "\\":
                if self.index >= len(self.text):
                    raise self.error("unterminated escape")
                escaped = self.text[self.index]
                self.index += 1
                mapping = {"n": "\n", "t": "\t", "r": "\r", "0": "\0"}
                value.append(mapping.get(escaped, escaped))
                continue
            value.append(character)

    def read_bare(self) -> object:
        self.skip_whitespace()
        start = self.index
        while self.index < len(self.text) and self.text[self.index] not in ",)] \t\r\n":
            self.index += 1
        token = self.text[start : self.index]
        if token == "true":
            return True
        if token == "false":
            return False
        if re.fullmatch(r"-?\d+", token):
            return int(token)
        if re.fullmatch(r"-?(\d+\.\d*|\.\d+|\d+)([eE][-+]?\d+)?", token):
            return float(token)
        return token

    def read_paren(self) -> object:
        """Read a map or a tuple, whichever starts after the opening paren.

        The data files use tuples for vectors/colors (`(0.0, 12.0, 0.0)`) and
        maps for records (`(skill: 1, variant: Hit, ...)`); both share the `(`
        delimiter, so a short lookahead decides which one it is.
        """
        self.expect("(")
        self.skip_whitespace()
        start = self.index
        is_map = False
        if self.index < len(self.text) and (self.text[self.index].isalpha() or self.text[self.index] == "_"):
            while self.index < len(self.text) and (self.text[self.index].isalnum() or self.text[self.index] in "_-"):
                self.index += 1
            while self.index < len(self.text) and self.text[self.index].isspace():
                self.index += 1
            is_map = self.index < len(self.text) and self.text[self.index] == ":"
        self.index = start

        if not is_map:
            values: list[object] = []
            while True:
                if self.peek() == ")":
                    self.index += 1
                    return tuple(values)
                values.append(self.read_value())
                if self.peek() == ",":
                    self.index += 1
                    continue
                if self.peek() == ")":
                    self.index += 1
                    return tuple(values)
                raise self.error("expected ',' or ')' in tuple")

        result: dict[str, object] = {}
        while True:
            if self.peek() == ")":
                self.index += 1
                return result
            key = self.read_identifier()
            self.expect(":")
            result[key] = self.read_value()
            if self.peek() == ",":
                self.index += 1
                continue
            if self.peek() == ")":
                self.index += 1
                return result
            raise self.error("expected ',' or ')' in map")

    def read_value(self) -> object:
        character = self.peek()
        if character == "(":
            return self.read_paren()
        if character == "[":
            return self.read_list()
        if character == '"':
            return self.read_string()
        return self.read_bare()

    def read_list(self) -> list[object]:
        self.expect("[")
        result: list[object] = []
        while True:
            if self.peek() == "]":
                self.index += 1
                return result
            result.append(self.read_value())
            if self.peek() == ",":
                self.index += 1
                continue
            if self.peek() == "]":
                self.index += 1
                return result
            raise self.error("expected ',' or ']' in list")


def read_ron(path: Path) -> dict[str, object]:
    return RonReader(path.read_text(encoding="utf-8")).read_paren()  # type: ignore[return-value]


# ---------------------------------------------------------------------------
# Canonical RON writer.
# ---------------------------------------------------------------------------


def escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def number(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    value = float(value)  # type: ignore[arg-type]
    if value == 0.0:
        return "0.0"
    if value == int(value) and abs(value) < 1e9:
        return f"{value:.1f}"
    return repr(value)


def tuple_of(value: object) -> str:
    assert isinstance(value, (list, tuple))
    return "(" + ", ".join(number(item) for item in value) + ")"


def string_list(value: object) -> str:
    assert isinstance(value, list)
    return "[" + ", ".join(f'"{escape(str(item))}"' for item in value) + "]"


#: Layer fields in the order they are written (only non-default values are
#: written so the generated file stays readable).
LAYER_FIELDS = [
    "kind",
    "anchor",
    "attached_entity",
    "head",
    "vertical",
    "file",
    "files",
    "frames",
    "frame_ms",
    "primitive",
    "offset",
    "scale",
    "size",
    "size_end",
    "duration_ms",
    "delay_ms",
    "cast_at",
    "count",
    "spread",
    "height",
    "bottom_radius",
    "top_radius",
    "rotation_speed",
    "grow",
    "alpha",
    "tint",
    "blend",
    "wav",
    "action",
    "light",
    "light_intensity",
    "hold_ms",
    "fade_ms",
]
LAYER_DEFAULTS: dict[str, object] = {
    "scale": 1.0,
    "offset": (0.0, 0.0, 0.0),
    "count": 1,
    "alpha": 1.0,
    "tint": (1.0, 1.0, 1.0),
    "blend": "Alpha",
    "grow": "Constant",
    "vertical": "Feet",
    "size": 0.0,
    "size_end": 0.0,
    "spread": 0.0,
    "height": 0.0,
    "bottom_radius": 0.0,
    "top_radius": 0.0,
    "rotation_speed": 0.0,
    "duration_ms": 0,
    "delay_ms": 0,
    "cast_at": None,
    "head": False,
    "light_intensity": 0.0,
    "hold_ms": 0,
    "fade_ms": 0,
    "frame_ms": 0,
}


def write_layer(layer: dict[str, object]) -> str:
    parts: list[str] = []
    for field in LAYER_FIELDS:
        if field not in layer:
            continue
        value = layer[field]
        if field in LAYER_DEFAULTS and value == LAYER_DEFAULTS[field]:
            continue
        if field in ("offset", "light", "tint"):
            parts.append(f"{field}: {tuple_of(value)}")
        elif field in ("files", "frames"):
            parts.append(f"{field}: {string_list(value)}")
        elif field == "file" or field == "wav":
            parts.append(f'{field}: "{escape(str(value))}"')
        elif field == "kind":
            parts.append(f"kind: {value}")
        elif isinstance(value, bool):
            parts.append(f"{field}: {'true' if value else 'false'}")
        elif isinstance(value, (int, float)):
            parts.append(f"{field}: {number(value)}")
        else:
            parts.append(f"{field}: {value}")
    return "(" + ", ".join(parts) + ")"


# ---------------------------------------------------------------------------
# Legacy row translation.
# ---------------------------------------------------------------------------


#: Layers the official (2026) decompilation proves are not part of the effect.
#: Keyed by (skill, phase, asset); the value is the evidence.
DROPPED_LAYERS: dict[tuple[int, str, str], str] = {
    (29, "Success", "effect\\agi_up.bmp"): (
        "EF_INCAGILITY only spawns ac_center2 primitives + EF_IncAgility.wav "
        "(RagEffect.cpp 16534); the agi_up billboard is a roBrowser-era approximation"
    ),
}


def asset_key(layer: dict[str, object]) -> str:
    """Identity used to fold duplicate layers (custom vs official vs legacy)."""
    if "file" in layer and layer["file"]:
        return str(layer["file"]).replace("/", "\\").lower()
    if "files" in layer:
        return "|".join(str(item).replace("/", "\\").lower() for item in layer["files"])  # type: ignore[union-attr]
    if layer.get("frames"):
        return "|".join(str(item).replace("/", "\\").lower() for item in layer["frames"])  # type: ignore[union-attr]
    return ""


def translate_str_row(row: dict[str, object]) -> dict[str, object]:
    layer: dict[str, object] = {"kind": "Str", "file": row["file"]}
    if row.get("overhead"):
        # The old table anchored buff completions on the caster, above the head.
        layer["anchor"] = "Caster"
        layer["head"] = True
    else:
        anchor = LEGACY_STR_ANCHORS.get(str(row.get("anchor", "Auto")), "Auto")
        layer["anchor"] = anchor
        if anchor == "Caster":
            layer["attached_entity"] = True
    return layer


def translate_spr_row(row: dict[str, object]) -> dict[str, object]:
    anchor = LEGACY_SPR_ANCHORS.get(str(row.get("anchor", "Caster")), "Caster")
    layer: dict[str, object] = {
        "kind": "Spr",
        "anchor": anchor,
        "files": list(row.get("files") or []),
    }
    if tuple(row.get("offset") or (0.0, 0.0, 0.0)) != (0.0, 0.0, 0.0):
        layer["offset"] = tuple(row["offset"])
    if float(row.get("scale", 1.0)) != 1.0:
        layer["scale"] = float(row["scale"])
    if int(row.get("duration_ms", 0)) != 0:
        layer["duration_ms"] = int(row["duration_ms"])
    return layer


def translate_procedural_row(row: dict[str, object]) -> dict[str, object]:
    primitive = str(row.get("primitive", "HitFlash"))
    kind = PRIMITIVE_KINDS.get(primitive, "Texture")
    layer: dict[str, object] = {
        "kind": kind,
        "anchor": str(row.get("anchor", "Auto")),
        "file": row.get("texture", ""),
        "duration_ms": int(row.get("duration_ms", 0)),
        "delay_ms": int(row.get("delay_ms", 0)),
        "count": int(row.get("count", 1)),
        "height": float(row.get("height", 0.0)),
        "alpha": float(row.get("alpha", 1.0)),
        "tint": tuple(row.get("color") or (1.0, 1.0, 1.0)),
        "blend": "Additive" if row.get("additive") else "Alpha",
        "rotation_speed": float(row.get("rotation_speed", 0.0)),
    }
    if primitive == "Projectile":
        layer["primitive"] = "Projectile"
    if primitive == "GroundAura":
        layer["bottom_radius"] = float(row.get("bottom_radius", 0.0))
        layer["top_radius"] = float(row.get("top_radius", 0.0))
    else:
        layer["size"] = float(row.get("size", 0.0))
        layer["size_end"] = float(row.get("size_end", 0.0))
        layer["spread"] = float(row.get("spread", 0.0))
    return layer


# ---------------------------------------------------------------------------
# Merge.
# ---------------------------------------------------------------------------


def phase_key(entry: dict[str, object]) -> tuple[int, int]:
    return int(entry["skill"]), PHASES.index(str(entry["phase"]))


def read_effects(path: Path) -> list[dict[str, object]]:
    document = read_ron(path)
    effects = document.get("effects", [])
    assert isinstance(effects, list), f"{path}: 'effects' is not a list"
    return [entry for entry in effects if isinstance(entry, dict)]


def group_rows(rows: list[dict[str, object]], translate) -> dict[tuple[int, str], list[dict[str, object]]]:
    grouped: dict[tuple[int, str], list[dict[str, object]]] = {}
    for row in rows:
        key = (int(row["skill"]), str(row["variant"]))
        grouped.setdefault(key, []).append(translate(row))
    return grouped


def build(
    custom_path: Path,
    official_path: Path | None,
    rodecomp_path: Path | None = None,
    overrides_path: Path | None = None,
    rodecomp_hardcoded_path: Path | None = None,
) -> list[dict[str, object]]:
    grouped: dict[tuple[int, str], dict[str, object]] = {}

    def entry_for(skill: int, phase: str) -> dict[str, object]:
        key = (skill, phase)
        if key not in grouped:
            grouped[key] = {"skill": skill, "phase": phase, "layers": []}
        return grouped[key]

    def add_layer(entry: dict[str, object], layer: dict[str, object]) -> None:
        assert isinstance(entry["layers"], list)
        entry["layers"].append(layer)  # type: ignore[union-attr]

    # 1. The complete official rows (when the official generator deliverable is
    #    present). They supersede the legacy approximations per skill/phase.
    official_keys: set[tuple[int, str]] = set()
    official_assets: dict[tuple[int, str], set[str]] = {}
    if official_path is not None and official_path.is_file():
        for descriptor in read_effects(official_path):
            key = (int(descriptor["skill"]), str(descriptor["phase"]))
            entry = entry_for(*key)
            for field in ("cast_circle_color", "action"):
                if field in descriptor:
                    entry[field] = descriptor[field]
            for layer in descriptor.get("layers") or []:
                assert isinstance(layer, dict)
                add_layer(entry, dict(layer))
                official_assets.setdefault(key, set()).add(asset_key(dict(layer)))
            official_keys.add(key)

    # 1a2. Hard-coded (C++) skill effects from the same decompilation, mapped
    #      through their effect id. These skills had an official effect id but
    #      no effect-table layer facts at all. Only layers whose asset is not
    #      already present for the (skill, phase) pair are added, so a skill
    #      that gains its descriptor elsewhere never loses a tuned layer.
    if rodecomp_hardcoded_path is not None and rodecomp_hardcoded_path.is_file():
        for descriptor in read_effects(rodecomp_hardcoded_path):
            key = (int(descriptor["skill"]), str(descriptor["phase"]))
            entry = entry_for(*key)
            for field in ("cast_circle_color", "action"):
                if field in descriptor and field not in entry:
                    entry[field] = descriptor[field]
            present = {asset_key(dict(layer)) for layer in entry["layers"]}
            for layer in descriptor.get("layers") or []:
                assert isinstance(layer, dict)
                asset = asset_key(dict(layer))
                if asset and asset in present:
                    continue
                add_layer(entry, dict(layer))
                official_assets.setdefault(key, set()).add(asset)
                if asset:
                    present.add(asset)
            official_keys.add(key)

    # 1b. Rows derived from the official client decompilation (rodecomp). They
    #     either add a descriptor for a skill whose effect the official client
    #     plays from hard-coded C++ (no data asset layer existed for the
    #     extractors) or append the official layers an existing descriptor was
    #     missing. The merge only ever adds layers, it never drops a tuned
    #     visual.
    if rodecomp_path is not None and rodecomp_path.is_file():
        for descriptor in read_effects(rodecomp_path):
            key = (int(descriptor["skill"]), str(descriptor["phase"]))
            entry = entry_for(*key)
            for field in ("cast_circle_color", "action"):
                if field in descriptor:
                    entry[field] = descriptor[field]
            for layer in descriptor.get("layers") or []:
                assert isinstance(layer, dict)
                add_layer(entry, dict(layer))
                official_assets.setdefault(key, set()).add(asset_key(dict(layer)))
            official_keys.add(key)

    # 2. Hand-written migrated layers. A custom layer with the same asset as an
    #    official layer **patches** that official layer (for example the official
    #    `head` flag additions that the official table does not carry, or the
    #    tuned scale/light of the migrated hand-written visual) instead of
    #    dropping or duplicating the official row. Custom layers without an
    #    official counterpart are appended.
    custom_assets: dict[tuple[int, str], set[str]] = {}
    for descriptor in read_effects(custom_path):
        key = (int(descriptor["skill"]), str(descriptor["phase"]))
        entry = entry_for(*key)
        for field in ("cast_circle_color", "action"):
            if field in descriptor:
                entry[field] = descriptor[field]
        for layer in descriptor.get("layers") or []:
            assert isinstance(layer, dict)
            layer = dict(layer)
            asset = asset_key(layer)
            if asset and asset in official_assets.get(key, set()):
                for official_layer in entry["layers"]:  # type: ignore[union-attr]
                    if asset_key(official_layer) == asset:
                        official_layer.update({field: value for field, value in layer.items() if field != "kind"})
                        break
            else:
                add_layer(entry, layer)
            if asset:
                custom_assets.setdefault(key, set()).add(asset)

    # 3. Legacy tables (the migration source until the official generator lands).
    legacy_sources = [
        (LEGACY_STR, translate_str_row),
        (LEGACY_SPR, translate_spr_row),
        (LEGACY_PROCEDURAL, translate_procedural_row),
    ]
    for path, translate in legacy_sources:
        if not path.is_file():
            continue
        grouped_rows = group_rows(read_effects(path), translate)
        for key, layers in grouped_rows.items():
            skill, phase = key
            if skill == 0 or phase not in PHASES:
                continue
            # A complete official row list for this phase replaces the legacy
            # approximation entirely.
            if key in official_keys:
                continue
            entry = entry_for(skill, phase)
            suppressed = custom_assets.get(key, set())
            for layer in layers:
                if asset_key(layer) in suppressed:
                    continue
                add_layer(entry, layer)

    # 4. Hand-written phase overrides. Unlike the additive custom layers, an
    #    override **replaces** the whole descriptor of a (skill, phase): used
    #    when the official table's row is an approximation that the decompiled
    #    client code supersedes (for example Asura Strike's `AsuraStrike`
    #    charge effect, which the 2019-era community table does not carry).
    if overrides_path is not None and overrides_path.is_file():
        for descriptor in read_effects(overrides_path):
            skill = int(descriptor["skill"])
            phase = str(descriptor["phase"])
            replacement: dict[str, object] = {"skill": skill, "phase": phase, "layers": []}
            for field in ("cast_circle_color", "action"):
                if field in descriptor:
                    replacement[field] = descriptor[field]
            for layer in descriptor.get("layers") or []:
                assert isinstance(layer, dict)
                replacement["layers"].append(dict(layer))  # type: ignore[union-attr]
            grouped[(skill, phase)] = replacement

    # Curated removals: layers the official client provably does not draw.
    for (skill, phase, asset), reason in DROPPED_LAYERS.items():
        entry = grouped.get((skill, phase))
        if entry is None:
            continue
        layers = [layer for layer in entry["layers"] if asset_key(layer) != asset]
        if len(layers) != len(entry["layers"]):
            print(f"dropped {asset!r} from skill {skill} {phase}: {reason}")
        entry["layers"] = layers

    # Stable order, empty phases with neither layers nor a tint/action dropped.
    result = []
    for entry in grouped.values():
        layers = entry["layers"]
        assert isinstance(layers, list)
        if not layers and "cast_circle_color" not in entry and "action" not in entry:
            continue
        result.append(entry)
    result.sort(key=phase_key)

    # Duplicate (skill, phase) entries are impossible (one dict per key).
    return result


def render(effects: list[dict[str, object]]) -> str:
    lines = [
        "// Unified skill effect descriptors for the client.",
        "//",
        "// Generated by `tools/gen_skill_effects.py`; do not edit by hand.",
        "// Regenerate with: `./tools/gen_skill_effects.py`",
        "// Drift check:      `./tools/gen_skill_effects.py --check`",
        "// Schema check:     `korangar --check-skill-effects`",
        "//",
        "// One entry per (skill, phase); every layer of a phase spawns once per cast,",
        "// in file order. The client de-duplicates a phase per cast, enforces a",
        "// per-phase layer budget and attaches entity layers to the effect budget /",
        "// distance culling of the effect holder.",
        "//",
        "// Phases: Ground, Cast, Success, Release, Hit, Caster",
        "// Layer kinds: Str (.str screen sprite), Spr (.spr/.act sprite),",
        "//   Texture (texture billboard/screen effect), Cylinder (texture cylinder /",
        "//   ground aura), Primitive (HitFlash/Projectile/GroundAura), BodyFlash",
        "//    (caster sprite flash).",
        "// Anchors: Caster, Target, Ground, Overhead, Body, Travel, Rain, Auto.",
        "//   `head: true` is the official client's overhead anchor (buff effects above",
        "//   the character's head); `attached_entity` makes a layer follow its entity.",
        "// The loader is `korangar/src/world/library/skill_effects.rs`.",
        "//",
        "// Sources: official EffectTable layer rows (multi-layer), the hard-coded C++",
        "// effect rows of the client decompilation (tools/data/skill_effects_rodecomp_hardcoded.ron),",
        "// the hand-written visuals (tools/data/skill_effects_custom.ron) and the legacy",
        "// per-kind tables (skill_effect_assets/sprite/procedural .ron).",
        "(",
        '    provenance: "Unified skill effect descriptors: official effect-table layers + migrated hand-written visuals",',
        "    version: 1,",
        "    effects: [",
    ]
    for entry in effects:
        skill = entry["skill"]
        phase = entry["phase"]
        lines.append("        (")
        lines.append(f"            skill: {skill},")
        lines.append(f"            phase: {phase},")
        if "cast_circle_color" in entry:
            lines.append(f"            cast_circle_color: {tuple_of(entry['cast_circle_color'])},")
        if "action" in entry:
            lines.append(f"            action: {entry['action']},")
        layers = entry["layers"]
        assert isinstance(layers, list)
        if not layers:
            lines.append("            layers: [],")
        else:
            lines.append("            layers: [")
            for layer in layers:
                assert isinstance(layer, dict)
                lines.append(f"                {write_layer(layer)},")
            lines.append("            ],")
        lines.append("        ),")
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def validate_assets(effects: list[dict[str, object]], game_dir: Path) -> list[str]:
    """Check that every effect asset referenced by the merged data exists."""
    spec = importlib.util.spec_from_file_location("gen_skill_effect_assets", Path(__file__).with_name("gen_skill_effect_assets.py"))
    assert spec is not None and spec.loader is not None
    legacy = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(legacy)

    names: list[str] = []
    for archive in ("CondaRO.grf", "data.grf", "palettes.grf"):
        path = game_dir / archive
        if path.is_file():
            names.extend(legacy.read_grf_names(path))
    if not names:
        return [f"no GRF archives found in {game_dir}"]

    available = {name.replace("/", "\\").lower() for name in names}
    errors: list[str] = []
    for entry in effects:
        skill = entry["skill"]
        for layer in entry["layers"]:  # type: ignore[union-attr]
            assert isinstance(layer, dict)
            kind = layer.get("kind")
            candidates: list[str] = []
            if kind == "Str":
                candidates = [f"data\\texture\\effect\\{layer.get('file')}"]
            elif kind == "Spr":
                candidates = [f"data\\sprite\\{stem}.spr" for stem in layer.get("files", [])]
                candidates += [f"data\\sprite\\{stem}.act" for stem in layer.get("files", [])]
            elif kind in ("Texture", "Cylinder", "Primitive"):
                candidates = [f"data\\texture\\{layer['file']}"] if layer.get("file") else []
                candidates += [f"data\\texture\\{frame}" for frame in layer.get("frames", [])]
            for candidate in candidates:
                if candidate.replace("/", "\\").lower() not in available:
                    errors.append(f"skill {skill}: missing asset {candidate}")
            wav = layer.get("wav")
            if wav and f"data\\wav\\{wav}".replace("/", "\\").lower() not in available:
                errors.append(f"skill {skill}: missing wav data\\wav\\{wav}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--custom", default=str(DEFAULT_CUSTOM), help="hand-written migrated layers")
    parser.add_argument("--official", default=str(DEFAULT_OFFICIAL), help="complete official layer rows (optional)")
    parser.add_argument("--rodecomp", default=str(DEFAULT_RODECOMP), help="rodecomp-derived rows (optional)")
    parser.add_argument(
        "--rodecomp-hardcoded",
        default=str(DEFAULT_RODECOMP_HARDCODED),
        help="hard-coded C++ effect rows for the 2019 effect-id gap (optional)",
    )
    parser.add_argument("--overrides", default=str(DEFAULT_OVERRIDES), help="hand-written phase overrides")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT), help="generated unified data file")
    parser.add_argument("--validate-assets", default=None, metavar="GAME_DIR", help="validate every referenced asset against the GRF archives")
    args = parser.parse_args()

    custom_path = Path(args.custom)
    if not custom_path.is_file():
        raise SystemExit(f"missing custom layer data: {custom_path}")

    official_path = Path(args.official)
    if not official_path.is_file():
        official_path = None

    rodecomp_path = Path(args.rodecomp)
    if not rodecomp_path.is_file():
        rodecomp_path = None

    rodecomp_hardcoded_path = Path(args.rodecomp_hardcoded)
    if not rodecomp_hardcoded_path.is_file():
        rodecomp_hardcoded_path = None

    overrides_path = Path(args.overrides)
    if not overrides_path.is_file():
        overrides_path = None

    effects = build(custom_path, official_path, rodecomp_path, overrides_path, rodecomp_hardcoded_path)
    if not effects:
        raise SystemExit("no skill effects could be merged")

    layer_count = sum(len(entry["layers"]) for entry in effects)  # type: ignore[arg-type]
    content = render(effects)
    output = Path(args.output)

    if args.validate_assets:
        errors = validate_assets(effects, Path(args.validate_assets))
        if errors:
            for error in errors:
                print(f"error: {error}", file=sys.stderr)
            return 1

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_skill_effects.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(effects)} phases, {layer_count} layers)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {len(effects)} phases, {layer_count} layers")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
