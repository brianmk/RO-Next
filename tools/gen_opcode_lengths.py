#!/usr/bin/env python3
"""Generate the 2019 opcode -> wire length registry for korangar.

The registry (`ragnarok-packets/src/opcode_lengths_2019.rs`) is the committed
source of truth for how many bytes every server-to-client opcode of the
CondaRO rAthena server has. It is generated from the **real** rAthena headers
at the pinned `PACKETVER`:

1. `g++ -E -P` is run on `src/map/packets.hpp`, `src/char/packets.hpp` and
   `src/common/packets.hpp` with `-DPACKETVER=20190605`, so every
   `#if PACKETVER ...` branch is resolved exactly like the running map server
   sees it.
2. All packet structs (`PACKET_*`, the lowercase `packet_*` spawn structs and
   `CHARACTER_INFO`) are compiled into a tiny `sizeof()` probe with the same
   flags and executed on the server. `__attribute__((packed))` guarantees the
   reported size is the exact wire size.
3. Flexible-array structs (`field[]`) become `Variable { base_size }` entries
   (their `sizeof` is the fixed part; the total length is in payload bytes
   0..2). Everything else becomes `Fixed(size)`.
4. A checked-in supplement (`tools/data/opcode_supplement_20190605.json`) covers the
   opcodes that the server does not define as structs: the hand-rolled
   char-server packets and the dynamic `enum packet_headers` opcodes
   (spawn/skill-unit/party/... packets). The supplement only names the server
   struct or gives the size that was verified against the server source; the
   generator fails if a referenced struct does not exist.

Usage:
    ./tools/gen_opcode_lengths.py                # regenerate the committed file
    ./tools/gen_opcode_lengths.py --check        # fail if the file is out of date
    ./tools/gen_opcode_lengths.py --ssh myhost --server-root /home/phantom/rathena
    ./tools/gen_opcode_lengths.py --local /path/to/rathena

Only read-only commands are run against the server tree.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

PACKETVER = 20190605
DEFAULT_SSH = "rathena"
DEFAULT_SERVER_ROOT = "/home/phantom/rathena"
HEADERS = {
    "map": "src/map/packets.hpp",
    "char": "src/char/packets.hpp",
    "common": "src/common/packets.hpp",
}
# Include flags taken from the rAthena map-server Makefile; needed because
# packets.hpp -> map.hpp -> script.hpp pulls in rapidyaml/mysql/yaml-cpp.
INCLUDE_FLAGS = " ".join(
    [
        "-Isrc/common",
        "-Isrc",
        "-I3rdparty/libconfig",
        "-I3rdparty/rapidyaml/src",
        "-I3rdparty/rapidyaml/ext/c4core/src",
        "-I3rdparty/yaml-cpp/include",
        "-I/usr/include/mysql",
        "-I/usr/include",
    ]
)
CXX_FLAGS = f"-std=c++17 -O0 -DPACKETVER={PACKETVER} {INCLUDE_FLAGS}"

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / "ragnarok-packets/src/opcode_lengths_2019.rs"
DEFAULT_SUPPLEMENT = ROOT / "tools/data/opcode_supplement_20190605.json"


def server_command(args: argparse.Namespace, command: str) -> str:
    """Run a shell command in the rAthena tree (local or over ssh)."""
    if args.local:
        return subprocess.run(
            ["bash", "-lc", command],
            cwd=args.local,
            check=True,
            capture_output=True,
            text=True,
        ).stdout

    remote = f"cd {args.server_root} && {command}"
    return subprocess.run(
        ["ssh", args.ssh, remote],
        check=True,
        capture_output=True,
        text=True,
        input=None,
    ).stdout


def preprocess(args: argparse.Namespace, header: str) -> str:
    command = f"g++ -E -P -DPACKETVER={PACKETVER} {INCLUDE_FLAGS} {header}"
    return server_command(args, command)


def find_structs(text: str) -> dict[str, str]:
    """Return struct name -> body for every `struct NAME { ... }`."""
    structs: dict[str, str] = {}

    for match in re.finditer(r"struct\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{", text):
        name = match.group(1)
        index = match.end()
        depth = 1
        start = index

        while index < len(text) and depth:
            character = text[index]

            if character == "{":
                depth += 1
            elif character == "}":
                depth -= 1

                if depth == 0:
                    break

            index += 1

        structs[name] = text[start:index]

    return structs


def find_headers(text: str) -> dict[str, int]:
    """Return header constant name -> opcode (WITHOUT the `HEADER_` prefix)."""
    headers: dict[str, int] = {}

    for match in re.finditer(r"const int16 HEADER_([A-Za-z0-9_]+)\s*=\s*(0x[0-9a-fA-F]+|\d+);", text):
        headers.setdefault(match.group(1), int(match.group(2), 0))

    return headers


def probe_sizes(args: argparse.Namespace, struct_sources: dict[str, dict[str, str]], header_files: dict[str, str]) -> dict[str, int]:
    """Compile and run a sizeof() probe on the server for all packet structs.

    The headers share the `PACKETS_HPP` include guard, so every header needs its
    own translation unit; otherwise only the first one would be included.
    """
    sizes: dict[str, int] = {}

    for key, header_file in header_files.items():
        names: list[str] = []

        for name in struct_sources[key]:
            if key == "map" and (name.startswith("PACKET_") or name.startswith("packet_")):
                names.append(name)
            elif key == "char" and (name.startswith("PACKET_") or name == "CHARACTER_INFO"):
                names.append(name)
            elif key == "common" and name.startswith("PACKET_"):
                names.append(name)

        if not names:
            continue

        source = [
            "#include <cstdio>",
            "#include <cstdint>",
            f'#include "{header_file}"',
            "int main(){",
        ]

        for name in sorted(names):
            source.append(f'  printf("{name} %zu\\n", sizeof({name}));')

        source.append("  return 0; }")
        source_text = "\n".join(source)

        compile_command = f"g++ {CXX_FLAGS} -o /tmp/korangar_opcode_probe -x c++ - && /tmp/korangar_opcode_probe"

        if args.local:
            result = subprocess.run(
                ["bash", "-lc", compile_command],
                cwd=args.local,
                input=source_text,
                check=False,
                capture_output=True,
                text=True,
            )
        else:
            result = subprocess.run(
                ["ssh", args.ssh, f"cd {args.server_root} && {compile_command}"],
                input=source_text,
                check=False,
                capture_output=True,
                text=True,
            )

        if result.returncode != 0:
            raise SystemExit(
                f"sizeof probe for {header_file} failed:\n{result.stderr[-4000:]}\n"
                "A struct may only exist under a different PACKETVER branch; adjust the generator."
            )

        for line in result.stdout.splitlines():
            parts = line.split()

            if len(parts) != 2:
                continue

            try:
                sizes[parts[0]] = int(parts[1])
            except ValueError:
                continue

    return sizes


def variable_structs(struct_sources: dict[str, dict[str, str]]) -> set[str]:
    return {name for structs in struct_sources.values() for name, body in structs.items() if "[]" in body}


def load_supplement(path: Path) -> dict[int, dict]:
    data = json.loads(path.read_text())
    entries: dict[int, dict] = {}

    for entry in data["entries"]:
        opcode = entry["opcode"]

        if isinstance(opcode, str):
            opcode = int(opcode, 0)

        if opcode in entries:
            raise SystemExit(f"duplicate supplement opcode 0x{opcode:04X}")

        entries[opcode] = entry

    return entries


def rust_opcode_length(entry: dict, sizes: dict[str, int], variable: set[str]) -> str:
    if "struct" in entry:
        struct = entry["struct"]

        if struct not in sizes:
            raise SystemExit(f"supplement for 0x{entry['opcode']:04X} references unknown struct {struct}")

        base_size = sizes[struct]
        variable_struct = struct in variable
    else:
        base_size = entry.get("base_size", entry.get("size"))
        variable_struct = entry["kind"] == "variable"

    kind = entry["kind"]

    if kind == "fixed":
        if "size" in entry:
            size = entry["size"]
        elif "struct" in entry:
            size = base_size
        else:
            raise SystemExit(f"fixed supplement entry 0x{entry['opcode']:04X} needs a size or struct")

        return f"OpcodeLength::Fixed({size})"

    if kind == "variable":
        if variable_struct and "struct" in entry:
            return f"OpcodeLength::Variable {{ base_size: {base_size} }}"

        if base_size is None:
            raise SystemExit(f"variable supplement entry 0x{entry['opcode']:04X} needs base_size")

        return f"OpcodeLength::Variable {{ base_size: {base_size} }}"

    raise SystemExit(f"unknown kind {kind!r} in supplement")


def generate(args: argparse.Namespace) -> str:
    struct_sources: dict[str, dict[str, str]] = {}
    header_sources: dict[str, dict[str, int]] = {}

    for key, header_file in HEADERS.items():
        text = preprocess(args, header_file)
        struct_sources[key] = find_structs(text)
        header_sources[key] = find_headers(text)

    sizes = probe_sizes(args, struct_sources, HEADERS)
    variable = variable_structs(struct_sources)
    supplement = load_supplement(args.supplement)

    entries: dict[int, tuple[str, str, str]] = {}
    unresolved: list[str] = []

    for key, header_file in HEADERS.items():
        for name, opcode in header_sources[key].items():
            if opcode in entries:
                continue

            struct = f"PACKET_{name}"

            if struct in sizes and (key == "map" and struct.startswith("PACKET_") or struct in sizes):
                if struct in variable:
                    length = f"OpcodeLength::Variable {{ base_size: {sizes[struct]} }}"
                else:
                    length = f"OpcodeLength::Fixed({sizes[struct]})"

                entries[opcode] = (name, length, f"{key} header")
                continue

            if opcode in supplement:
                length = rust_opcode_length(supplement[opcode], sizes, variable)
                entries[opcode] = (supplement[opcode]["name"], length, "supplement")
                continue

            unresolved.append(f"0x{opcode:04X} HEADER_{name} ({key}/{header_file}): no struct PACKET_{name}")

    for opcode, entry in supplement.items():
        if opcode in entries:
            continue

        length = rust_opcode_length(entry, sizes, variable)
        entries[opcode] = (entry["name"], length, "supplement")

    if unresolved:
        print("WARNING: header entries without a struct (add them to the supplement):", file=sys.stderr)

        for line in unresolved:
            print(f"  {line}", file=sys.stderr)

    lines = [
        "// @generated by tools/gen_opcode_lengths.py -- do not edit manually.",
        "//",
        f"// Source: rAthena packet headers at PACKETVER {PACKETVER}",
        "// (src/map/packets.hpp, src/char/packets.hpp, src/common/packets.hpp plus the",
        "// checked-in tools/data/opcode_supplement_20190605.json). Regenerate with:",
        "//",
        "//     ./tools/gen_opcode_lengths.py",
        "//",
        "// Every fixed entry is the exact server `sizeof(PACKET_*)`, variable entries",
        "// store the size of the fixed part; the total length is in payload bytes 0..2.",
        "",
        "use crate::PacketHeader;",
        "",
        "/// How many wire bytes a server-to-client packet has.",
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]",
        "pub enum OpcodeLength {",
        "    /// Fixed size packet, the number is the full size including the header.",
        "    Fixed(usize),",
        "    /// Variable length packet; the total size is in payload bytes 0..2.",
        "    Variable {",
        "        /// Size of the fixed part of the packet, including the header.",
        "        base_size: usize,",
        "    },",
        "}",
        "",
        "/// One entry of the 2019 opcode length registry.",
        "#[derive(Debug, Clone, Copy)]",
        "pub struct OpcodeLengthEntry {",
        "    /// Opcode.",
        "    pub header: u16,",
        "    /// rAthena name of the packet.",
        "    pub name: &'static str,",
        "    /// Length information.",
        "    pub length: OpcodeLength,",
        "    /// Where the entry came from (header constant or supplement).",
        "    pub source: &'static str,",
        "}",
        "",
        f"/// PACKETVER the registry was generated for.",
        f"pub const PACKETVER: u32 = {PACKETVER};",
        "",
        "/// The entries, sorted by opcode.",
        "pub static OPCODE_LENGTHS: &[OpcodeLengthEntry] = &[",
    ]

    for opcode in sorted(entries):
        name, length, source = entries[opcode]
        lines.append(f'    OpcodeLengthEntry {{ header: 0x{opcode:04X}, name: "{name}", length: {length}, source: "{source}" }},')

    lines += [
        "];",
        "",
        "/// Look up the 2019 wire layout of an opcode.",
        "pub fn opcode_length(header: PacketHeader) -> Option<OpcodeLength> {",
        "    opcode_length_raw(header.0)",
        "}",
        "",
        "/// Look up the 2019 wire layout of a raw opcode.",
        "pub fn opcode_length_raw(header: u16) -> Option<OpcodeLength> {",
        "    OPCODE_LENGTHS",
        "        .binary_search_by_key(&header, |entry| entry.header)",
        "        .ok()",
        "        .map(|index| OPCODE_LENGTHS[index].length)",
        "}",
        "",
        "/// Look up the registry entry of an opcode.",
        "pub fn opcode_entry(header: u16) -> Option<&'static OpcodeLengthEntry> {",
        "    OPCODE_LENGTHS",
        "        .binary_search_by_key(&header, |entry| entry.header)",
        "        .ok()",
        "        .map(|index| &OPCODE_LENGTHS[index])",
        "}",
        "",
    ]

    print(f"generated {len(entries)} opcode entries ({len(unresolved)} unresolved headers)", file=sys.stderr)

    return "\n".join(lines)


def format_rust(source: str) -> str:
    """Run rustfmt over the generated source so `--check` is stable.

    The committed file is rustfmt-formatted; formatting the generated output
    with the same tool guarantees `./tools/gen_opcode_lengths.py --check`
    passes independently of the rustfmt version in the CI image.
    """
    try:
        result = subprocess.run(
            ["rustfmt", "--edition", "2024", "--emit", "stdout"],
            input=source,
            check=True,
            capture_output=True,
            text=True,
        )
        return result.stdout
    except (subprocess.CalledProcessError, FileNotFoundError) as error:
        print(f"warning: could not run rustfmt ({error}); registry will not be formatted", file=sys.stderr)
        return source


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ssh", default=DEFAULT_SSH, help="ssh host that has the rAthena tree")
    parser.add_argument("--server-root", default=DEFAULT_SERVER_ROOT, help="rAthena tree on the ssh host")
    parser.add_argument("--local", default=None, help="use a local rAthena tree instead of ssh")
    parser.add_argument("--supplement", default=DEFAULT_SUPPLEMENT, type=Path)
    parser.add_argument("--output", default=DEFAULT_OUTPUT, type=Path)
    parser.add_argument("--check", action="store_true", help="only verify that the committed file is up to date")
    args = parser.parse_args()

    generated = format_rust(generate(args))

    if args.check:
        try:
            current = args.output.read_text()
        except FileNotFoundError:
            print(f"{args.output} does not exist; run the generator", file=sys.stderr)
            return 1

        if current != generated:
            print(f"{args.output} is out of date; run tools/gen_opcode_lengths.py", file=sys.stderr)
            return 1

        print(f"{args.output} is up to date", file=sys.stderr)
        return 0

    args.output.write_text(generated)
    print(f"wrote {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
