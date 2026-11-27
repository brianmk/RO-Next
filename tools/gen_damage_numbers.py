#!/usr/bin/env python3
"""Generate the official damage-number tables.

Writes ``ro-next/archive/data/client/damage_numbers.ron``: the digit atlas,
the per-digit layout, the style -> animation/scale table and the official
number colours. The client loads it through the normal ``GameFileLoader`` and
embeds the same file with ``include_str!`` as compiled fallback.

Source of truth is the client decompilation:

* ``Source/Contents/Actors/RenderObject.cpp:4775-4843``
  (``CGameActor::SpawnDamageDigits``) - digit extraction, per-digit horizontal
  offsets, the extended-damage K/M markers, and the type-22 0.5 scale reset.
* ``Source/Contents/Effects/Core/MsgEffect.cpp:206-553``
  (``CMsgEffect::OnProcess``) - the per-style timeline (scale, fade, rise,
  lifetime). Exact per-style ranges: :246-259 critical, :284-298 normal/half,
  :321-331 miss, :333-351 total, :535-551 type 114.
* ``Source/Contents/Effects/Core/MsgEffect.cpp:1308-1329``
  (``CMsgEffect::FollowOwner``) / :122-202 (drift helpers) - the numbers follow
  the target and drift diagonally by ``dir * (3 + elapsed / 3)``.
* ``Source/Contents/Effects/Core/MsgEffect.cpp:1608-1860``
  (``CMsgEffect::SendMsg`` / ``AM_SETPOS2`` :1718-1775 / ``AM_SETPOS``
  :1777-1860) - spawn offset (12 below the head coordinate, 27 for the total
  number), scale 5, depth -2.
* ``Source/Contents/Actors/GameObject.cpp:1900-1945`` and
  ``Source/Client/Mode/GameModePackets.cpp:1653-1665, 4683-4698`` - the
  colours the packet handler assigns to the numbers.
* ``Source/Contents/Actors/GameObject.cpp:1917-1924`` - the critical backing
  sprite: the only ``AM_MAKE_CRITICAL_EFFECT`` sender is the ``id == 148``
  (``DMG_MULTI_HIT_CRITICAL``) branch. ``CMsgEffect`` spawns the extra
  ``MET_CRITICAL_EFFECT`` (type 15) from ``MAKETYPE_CRITICAL_EFFECT``
  (``WorldObjects.cpp:1176-1182``) with the damage skin's message sprite and
  ACT action 2; ``MsgEffect.cpp:377-395`` / ``:1825-1835`` give its pose.
* ``data\\sprite\\이팩트\\숫자.spr/.act`` (committed loose archive copy) - the
  ten digit frames and their 0.3 cell zoom. ``data\\sprite\\이팩트\\miss.spr``
  and ``data\\sprite\\이팩트\\msg.spr/.act`` (the ``MET_CRITICAL_EFFECT``
  backing, image 3 at action 2) live in ``data.grf`` and are not committed;
  their cells are recorded from the GRF.

The numeric facts live in ``tools/data/damage_number_facts.json`` with a
``source`` per row. This script only checks them against the shipped sprite and
formats the RON file.

Usage:
    ./tools/gen_damage_numbers.py            # regenerate the data file
    ./tools/gen_damage_numbers.py --check    # fail if the committed file is out of date
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

DEFAULT_FACTS = Path("tools/data/damage_number_facts.json")
DEFAULT_OUTPUT = Path("ro-next/archive/data/client/damage_numbers.ron")
DEFAULT_SPRITE = Path("ro-next/archive/data/sprite/이팩트/숫자.spr")
DEFAULT_ACTION = Path("ro-next/archive/data/sprite/이팩트/숫자.act")
#: The critical backing sprite lives in ``data.grf`` and is not committed, so
#: these are optional: when absent the recorded cells are used as-is.
DEFAULT_BACKING_SPRITE = Path("ro-next/archive/data/sprite/이팩트/msg.spr")
DEFAULT_BACKING_ACTION = Path("ro-next/archive/data/sprite/이팩트/msg.act")

EXPECTED_STYLES = ["normal", "critical", "half", "total", "simple", "miss"]
EXPECTED_COLORS = {
    "white",
    "player_red",
    "critical_yellow",
    "blue_damage",
    "heal_green",
    "sp_blue",
    "status_green",
    "status_blue",
    "status_red",
    "status_purple",
    "status_black",
    "status_white",
    "status_yellow",
    "status_pink",
}


def ron_str(value: str) -> str:
    """Quote a string as a RON string (JSON escaping is valid RON)."""
    return json.dumps(value, ensure_ascii=False)


def ron_float(value: float) -> str:
    """Format a float so RON parses it as a float (``3.0``, not ``3``)."""
    return repr(float(value))


def read_sprite_dimensions(path: Path) -> list[tuple[int, int]]:
    """Image dimensions of an RO ``.spr`` (only the header/RLE walk is needed)."""
    data = path.read_bytes()
    if data[:2] != b"SP":
        raise SystemExit(f"{path}: not a sprite file")
    version = struct.unpack_from("<H", data, 2)[0]
    count = struct.unpack_from("<H", data, 4)[0]
    offset = 6
    if version >= 0x102:
        offset += 2  # RGBA image count
    sizes: list[tuple[int, int]] = []
    for _ in range(count):
        width, height = struct.unpack_from("<HH", data, offset)
        offset += 4
        size = width * height
        if version < 0x201:
            offset += size
        else:
            encoded = struct.unpack_from("<H", data, offset)[0]
            offset += 2
            end = offset + encoded
            decoded = 0
            while decoded < size and offset < end:
                byte = data[offset]
                offset += 1
                if byte == 0:
                    length = max(data[offset], 1)
                    offset += 1
                    decoded += length
                else:
                    decoded += 1
        sizes.append((width, height))
    return sizes


def read_action_motions(path: Path) -> list[list[tuple[int, int, int, int]]]:
    """Motion count and cell (x, y, image, zoom) list of an RO ``.act``."""
    data = path.read_bytes()
    if data[:2] != b"AC":
        raise SystemExit(f"{path}: not an action file")
    version = struct.unpack_from("<H", data, 2)[0]
    offset = 4
    action_count = struct.unpack_from("<H", data, offset)[0]
    offset += 2 + 10
    actions: list[list[tuple[int, int, int, int]]] = []
    for _ in range(action_count):
        motion_count = struct.unpack_from("<I", data, offset)[0]
        offset += 4
        motions = []
        for _ in range(motion_count):
            offset += 32  # range1 + range2
            cell_count = struct.unpack_from("<I", data, offset)[0]
            offset += 4
            cells = []
            for _ in range(cell_count):
                x, y = struct.unpack_from("<ii", data, offset)
                offset += 8
                image = struct.unpack_from("<i", data, offset)[0]
                offset += 4
                offset += 4  # mirror
                if version >= 0x200:
                    offset += 4  # colour
                if version >= 0x204:
                    zoom = struct.unpack_from("<2f", data, offset)
                    offset += 8
                else:
                    zoom = (struct.unpack_from("<f", data, offset)[0], 1.0)
                    offset += 4
                if version >= 0x200:
                    offset += 8  # angle + sprite type
                if version >= 0x205:
                    offset += 8  # size
                cells.append((x, y, image, zoom[0]))
            if version >= 0x200:
                offset += 4  # event id
            if version >= 0x203:
                attach_count = struct.unpack_from("<I", data, offset)[0]
                offset += 4 + attach_count * 16
            motions.append(cells)
        actions.append(motions)
    return actions


def build(
    facts: dict,
    sprite: Path,
    action: Path,
    backing_sprite: Path | None = None,
    backing_action: Path | None = None,
) -> dict:
    digit_rows = facts["digits"]
    sizes = read_sprite_dimensions(sprite)
    actions = read_action_motions(action)
    if len(actions) != 1:
        raise SystemExit(f"{action}: expected one action, got {len(actions)}")
    motions = actions[0]
    if len(motions) != digit_rows["motion_count"]:
        raise SystemExit(f"{action}: expected {digit_rows['motion_count']} digit motions, got {len(motions)}")
    if len(sizes) != len(motions):
        raise SystemExit(f"{sprite}: {len(sizes)} images but {len(motions)} digit motions")

    digits = []
    for index, cells in enumerate(motions):
        if len(cells) != 1:
            raise SystemExit(f"{action}: motion {index} has {len(cells)} cells, expected 1")
        cell_x, cell_y, image, zoom = cells[0]
        width, height = sizes[index]
        if index != image:
            raise SystemExit(f"{action}: motion {index} uses image {image}, expected digit == motion")
        if (cell_x, cell_y) != (digit_rows["cell_x"], digit_rows["cell_y"]):
            raise SystemExit(f"{action}: motion {index} cell position {(cell_x, cell_y)}")
        if abs(zoom - digit_rows["cell_scale_x"]) > 1e-6:
            raise SystemExit(f"{action}: motion {index} zoom {zoom} != {digit_rows['cell_scale_x']}")
        digits.append(
            {
                "digit": index,
                "image": image,
                "width": width,
                "height": height,
            }
        )

    styles = facts["styles"]
    names = [style["name"] for style in styles]
    if names != EXPECTED_STYLES:
        raise SystemExit(f"style rows must be {EXPECTED_STYLES}, got {names}")
    for style in styles:
        if style["vertical_curve"] not in ("quadratic", "linear"):
            raise SystemExit(f"style {style['name']}: unknown curve {style['vertical_curve']}")
        if style["lifetime_frames"] <= 0 or style["spawn_offset"] < 0:
            raise SystemExit(f"style {style['name']}: invalid lifetime/spawn offset")
        if not (style["min_scale"] <= style["max_scale"]):
            raise SystemExit(f"style {style['name']}: min_scale > max_scale")

    layout = facts["layout"]
    if layout["unit"] <= 0 or layout["extended_step"] <= 0:
        raise SystemExit("layout unit/extended_step must be positive")
    timeline = facts["timeline"]
    if timeline["frame_ms"] <= 0 or timeline["step_ms"] <= 0 or timeline["multi_hit_interval_ms"] <= 0:
        raise SystemExit("timeline values must be positive")

    color_names = {color["name"] for color in facts["colors"]}
    if color_names != EXPECTED_COLORS:
        raise SystemExit(f"colour rows must be {sorted(EXPECTED_COLORS)}, got {sorted(color_names)}")
    for style in styles:
        if style["color"] not in color_names:
            raise SystemExit(f"style {style['name']}: unknown colour {style['color']}")

    backing = facts["critical_backing"]
    if backing["number_style"] != "critical":
        raise SystemExit("critical_backing must belong to the critical number style")
    if backing["action_index"] < 0 or backing["image"] < 0:
        raise SystemExit("critical_backing: negative action/image index")
    if backing["motion_count"] <= 0 or len(backing["motion_offsets_x"]) != backing["motion_count"]:
        raise SystemExit("critical_backing: motion_count must match motion_offsets_x")
    if backing["motion_rate"] <= 0.0:
        raise SystemExit("critical_backing: motion_rate must be positive")
    if backing["width"] <= 0 or backing["height"] <= 0:
        raise SystemExit("critical_backing: empty image size")
    if not (backing["min_scale"] <= backing["base_scale"]):
        raise SystemExit("critical_backing: min_scale > base_scale")
    if backing["scale_step"] < 0.0 or backing["alpha_step"] < 0.0 or backing["spawn_offset"] < 0.0:
        raise SystemExit("critical_backing: negative timeline constant")
    if backing["lifetime_frames"] <= 0.0:
        raise SystemExit("critical_backing: invalid lifetime")
    if backing["depth_layer"] >= backing["digit_depth_layer"]:
        raise SystemExit("critical_backing: must be drawn behind the digits (smaller depthLayer)")

    if backing_sprite is not None and backing_action is not None and backing_sprite.is_file() and backing_action.is_file():
        # The backing sprite is not committed (it lives in `data.grf`); this
        # validates the recorded cells when a data set does ship the pair.
        sizes = read_sprite_dimensions(backing_sprite)
        actions = read_action_motions(backing_action)
        if len(actions) <= backing["action_index"]:
            raise SystemExit(f"{backing_action}: no action {backing['action_index']}")
        motions = actions[backing["action_index"]]
        if len(motions) != backing["motion_count"]:
            raise SystemExit(f"{backing_action}: action {backing['action_index']} has {len(motions)} motions")
        for index, cells in enumerate(motions):
            if len(cells) != 1:
                raise SystemExit(f"{backing_action}: action {backing['action_index']} motion {index} has {len(cells)} cells")
            cell_x, cell_y, image, zoom = cells[0]
            if image != backing["image"]:
                raise SystemExit(f"{backing_action}: motion {index} uses image {image}")
            if (cell_y, round(zoom, 3)) != (backing["cell_y"], round(backing["cell_scale_x"], 3)):
                raise SystemExit(f"{backing_action}: motion {index} cell {(cell_y, zoom)}")
            if cell_x != backing["motion_offsets_x"][index]:
                raise SystemExit(f"{backing_action}: motion {index} x {cell_x}")
        if sizes[backing["image"]] != (backing["width"], backing["height"]):
            raise SystemExit(f"{backing_sprite}: image {backing['image']} is {sizes[backing['image']]}")

    return {"digits": digits, "styles": styles}


def render(facts: dict, data: dict) -> str:
    lines: list[str] = []

    def add(line: str = "") -> None:
        lines.append(line)

    add("// Official damage-number visuals.")
    add("//")
    add("// Generated by `tools/gen_damage_numbers.py`; do not edit by hand.")
    add("// Regenerate with: `./tools/gen_damage_numbers.py`")
    add("// Drift check:      `./tools/gen_damage_numbers.py --check`")
    add("//")
    add("// Source of truth is the client decompilation:")
    add("// * `CGameActor::SpawnDamageDigits` (RenderObject.cpp:4775-4843)")
    add("// * `CMsgEffect::OnProcess` / `FollowOwner` (MsgEffect.cpp:122-202,")
    add("//   :206-553, :1308-1329)")
    add("// * `CMsgEffect::SendMsg` (`AM_SETPOS2`, MsgEffect.cpp:1608-1775)")
    add("// * damage-number colours (GameObject.cpp:1900-1945,")
    add("//   GameModePackets.cpp:1653-1665, 4683-4698)")
    add("//")
    add("// The digit atlas is `data\\sprite\\이팩트\\숫자.spr`: ten 0.3-zoomed images")
    add("// indexed by the ACT motion (digit 0 -> motion 0, ... digit 9 -> motion 9).")
    add("// The K/M markers of `TT_EXTEND_DAMAGE_ON_OFF` (motions 10/11,")
    add("// RenderObject.cpp:4781-4785) are not shipped by this data set; the")
    add("// client falls back to the full number when a marker is unavailable.")
    add("//")
    add("// Timeline units: `elapsed` is in the official 24 ms clock")
    add("// (`GetElapsedFrames`, GameActor.cpp:1107-1109); the client advances it in")
    add("// fixed 16 ms steps.")
    add("(")
    add(f"    provenance: {ron_str(facts['provenance']['source'])},")
    add("    version: 2,")
    timeline = facts["timeline"]
    add("    // Official timeline: `GetElapsedFrames` advances in 24 ms units and")
    add("    // the client runs effect state on a fixed 16 ms step. Multi-hit")
    add("    // packets emit one number per `g_multiHitInterval`.")
    add("    timeline: (")
    add(f"        frame_ms: {timeline['frame_ms']},")
    add(f"        step_ms: {timeline['step_ms']},")
    add(f"        multi_hit_interval_ms: {timeline['multi_hit_interval_ms']},")
    add("    ),")
    add("    // Digit -> sprite image; every digit is one 28x42-ish image.")
    add("    digits: [")
    for row in data["digits"]:
        add(
            f"        (digit: {row['digit']}, image: {row['image']}, "
            f"width: {row['width']}, height: {row['height']}),"
        )
    add("    ],")
    digits = facts["digits"]
    add("    atlas: (")
    add(f"        sprite: {ron_str(digits['sprite'])},")
    add(f"        action: {ron_str(digits['action'])},")
    add(f"        cell_x: {digits['cell_x']},")
    add(f"        cell_y: {digits['cell_y']},")
    add(f"        cell_scale_x: {digits['cell_scale_x']},")
    add(f"        cell_scale_y: {digits['cell_scale_y']},")
    add("    ),")
    layout = facts["layout"]
    add("    // Per-digit screen offsets in pre-`render.scale` sprite pixels,")
    add("    // right-aligned against the centre:")
    add("    // `(digits - index * 2) * unit + bias` (RenderObject.cpp:4823).")
    add("    // The extended (million-unit) variant is :4818 / :4820.")
    add("    layout: (")
    add(f"        unit: {layout['unit']},")
    add(f"        bias: {layout['bias']},")
    add(f"        extended_base: {layout['extended_base']},")
    add(f"        extended_first_add: {layout['extended_first_add']},")
    add(f"        extended_step: {layout['extended_step']},")
    add("    ),")
    extended = facts["extended_damage"]
    add("    // `TT_EXTEND_DAMAGE_ON_OFF`: million -> /1000 + marker 10,")
    add("    // hundred-million -> /1000000 + marker 11.")
    add("    extended: (")
    add(f"        enabled_value: {extended['enabled_value']},")
    add(f"        million_divisor: {extended['million_divisor']},")
    add(f"        million_marker: {extended['million_marker']},")
    add(f"        hundred_million_value: {extended['hundred_million_value']},")
    add(f"        hundred_million_divisor: {extended['hundred_million_divisor']},")
    add(f"        hundred_million_marker: {extended['hundred_million_marker']},")
    add("    ),")
    miss = facts["miss"]
    add("    // `SpawnDamageMsgEffect` uses `g_session.damageSpriteName` for misses")
    add("    // (WorldObjects.cpp:963-983); `data\\sprite\\이팩트\\miss.spr` is in data.grf.")
    add("    miss: (")
    add(f"        sprite: {ron_str(miss['sprite'])},")
    add(f"        action: {ron_str(miss['action'])},")
    add(f"        image: {miss['image']},")
    add(f"        cell_x: {miss['cell_x']},")
    add(f"        cell_y: {miss['cell_y']},")
    add(f"        cell_scale_x: {miss['cell_scale_x']},")
    add(f"        cell_scale_y: {miss['cell_scale_y']},")
    add(f"        width: {miss['width']},")
    add(f"        height: {miss['height']},")
    add(f"        cell_alpha: {miss['cell_alpha']},")
    add("    ),")
    backing = facts["critical_backing"]
    add("    // `MET_CRITICAL_EFFECT` (type 15): the sprite drawn *behind* a critical")
    add("    // number. `AM_MAKE_CRITICAL_EFFECT` (GameObject.cpp:1632-1636) is only")
    add("    // sent from the `id == 148` DMG_MULTI_HIT_CRITICAL branch")
    add("    // (GameObject.cpp:1917-1924, CombatTypes.h:16); `MAKETYPE_CRITICAL_EFFECT`")
    add("    // (WorldObjects.cpp:1176-1182) loads the damage skin's message sprite")
    add("    // (`이팩트\\msg`, ClientTiming.cpp:784-806) and plays ACT action 2.")
    add("    // The pose is the `AM_SETPOS` critical case (MsgEffect.cpp:1825-1835):")
    add("    // same origin as the digits (y -= 12) but scale 3.0 and depthLayer -5")
    add("    // (digits: -2, MsgEffect.cpp:1772), i.e. behind them; the timeline is")
    add("    // MsgEffect.cpp:377-395. The sprite is in data.grf, not committed.")
    add("    critical_backing: (")
    add(f"        number_style: {ron_str(backing['number_style'])},")
    add(f"        sprite: {ron_str(backing['sprite'])},")
    add(f"        action: {ron_str(backing['action'])},")
    add(f"        action_index: {backing['action_index']},")
    add(f"        image: {backing['image']},")
    add(f"        width: {backing['width']},")
    add(f"        height: {backing['height']},")
    add(f"        cell_y: {backing['cell_y']},")
    add(f"        cell_scale_x: {ron_float(backing['cell_scale_x'])},")
    add(f"        cell_scale_y: {ron_float(backing['cell_scale_y'])},")
    add(f"        motion_count: {backing['motion_count']},")
    add("        motion_offsets_x: [" + ", ".join(str(value) for value in backing["motion_offsets_x"]) + "],")
    add(f"        motion_start: {backing['motion_start']},")
    add(f"        motion_rate: {ron_float(backing['motion_rate'])},")
    add(f"        base_scale: {ron_float(backing['base_scale'])},")
    add(f"        min_scale: {ron_float(backing['min_scale'])},")
    add(f"        scale_step: {ron_float(backing['scale_step'])},")
    add(f"        spawn_offset: {ron_float(backing['spawn_offset'])},")
    add(f"        depth_layer: {backing['depth_layer']},")
    add(f"        digit_depth_layer: {backing['digit_depth_layer']},")
    add(f"        argb: 0x{backing['argb']:08X},")
    add(f"        alpha_start: {ron_float(backing['alpha_start'])},")
    add(f"        alpha_step: {ron_float(backing['alpha_step'])},")
    add(f"        lifetime_frames: {ron_float(backing['lifetime_frames'])},")
    add(f"        trigger_message: {backing['trigger_message']},")
    add(f"        trigger_damage_type: {backing['trigger_damage_type']},")
    add("    ),")
    add("    // One row per `CMsgEffect` number type. `scale` is the render scale")
    add("    // (`AM_SETPOS2` starts at 5.0; the total number overrides it to 0.5);")
    add("    // `scale_step` is the per-frame shrink and `scale_growth` the")
    add("    // incremental grow of the total number.")
    add("    styles: [")
    for style in data["styles"]:
        add("        (")
        add(f"            name: {ron_str(style['name'])},")
        add(f"            type_id: {style['type_id']},")
        add(f"            lifetime_frames: {style['lifetime_frames']},")
        add(f"            spawn_offset: {style['spawn_offset']},")
        add(
            "            vertical_curve: "
            + ("Quadratic" if style["vertical_curve"] == "quadratic" else "Linear")
            + ","
        )
        add(f"            vertical_speed: {style['vertical_speed']},")
        add(f"            horizontal_drift: {'true' if style['horizontal_drift'] else 'false'},")
        add(f"            base_scale: {style['base_scale']},")
        add(f"            min_scale: {style['min_scale']},")
        add(f"            max_scale: {style['max_scale']},")
        add(f"            scale_step: {style['scale_step']},")
        add(f"            scale_growth: {style['scale_growth']},")
        add(f"            alpha_start: {style['alpha_start']},")
        add(f"            alpha_step: {style['alpha_step']},")
        add(f"            fade_start: {style['fade_start']},")
        add(f"            fade_step: {style['fade_step']},")
        add(f"            color: {ron_str(style['color'])},")
        add("        ),")
    add("    ],")
    add("    // Official ARGB colours (the alpha byte is the style alpha).")
    add("    colors: [")
    for color in facts["colors"]:
        add(f"        (name: {ron_str(color['name'])}, argb: 0x{color['argb']:08X}),")
    add("    ],")
    add(")")
    add("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    parser.add_argument("--sprite", default=str(DEFAULT_SPRITE))
    parser.add_argument("--action", default=str(DEFAULT_ACTION))
    parser.add_argument("--backing-sprite", default=str(DEFAULT_BACKING_SPRITE))
    parser.add_argument("--backing-action", default=str(DEFAULT_BACKING_ACTION))
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    args = parser.parse_args()

    facts = json.loads(Path(args.facts).read_text(encoding="utf-8"))
    data = build(facts, Path(args.sprite), Path(args.action), Path(args.backing_sprite), Path(args.backing_action))
    content = render(facts, data)

    output = Path(args.output)
    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_damage_numbers.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({len(data['digits'])} digits, {len(data['styles'])} styles)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {len(data['digits'])} digits, {len(data['styles'])} styles")
    return 0


if __name__ == "__main__":
    sys.exit(main())
