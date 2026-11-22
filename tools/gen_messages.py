#!/usr/bin/env python3
"""Generate the data-driven protocol result-code message table.

The client used to map protocol result codes to English sentences with `match`
arms in Rust. This generator turns the extracted facts
(`tools/message_facts.json`, produced by `tools/extract_message_facts.py`) into
the runtime data file:

    korangar/archive/data/client/messages.ron

The file is loaded at runtime through the normal client file loader, so a
server can override it without rebuilding the client; the same file is embedded
with `include_str!` as the compiled fallback, so the archive override and the
compiled table can never drift apart. The Rust schema lives in
`korangar-messages/src/lib.rs`; `korangar --check-messages` validates it.

Format: one domain per protocol result-code family, one row per result code:

    (
        provenance: "rAthena ... + CondaRO.grf msgstringtable.txt ...",
        version: 1,
        domains: [
            (
                domain: PartyInviteReply,
                unknown: "Party invitation result: {code}.",
                unknown_args: [],
                rows: [
                    (
                        code: 0,
                        key: "party.invite.already_in_party",
                        msg_id: Some(80),
                        official: Some("The Character already joined another party."),
                        default: "{character_name} is already in a party.",
                        args: ["character_name"],
                        display: Current,
                    ),
                ],
            ),
        ],
    )

Field semantics:

* `default` — the current client text, a `format!`-style template using named
  `{argument}` placeholders (`{code}` is always available and is the result
  code). This is what P0 renders, so the rendered text is byte-identical to
  the previous hardcoded `match` arms.
* `official` / `msg_id` — the official rAthena `MsgStringTable` id (or, for the
  skill-fail causes the official client only resolves on the client side, the
  official client's id) and that table entry's text. `official` is `None` when
  the table only contains an `MSI_*` macro token or no matching entry exists.
* `args` — the call-site format arguments, in the order official-client `%s` /
  `%d` placeholders would be substituted when a row is switched to `Official`.
* `display` — `Current` (render `default`) or `Official` (render `official`).
  P0 commits every row as `Current` (no user-facing behaviour change); a later
  localization pass can flip rows to `Official` after review, without touching
  Rust code.

`--check` is the drift guard: it fails when the committed data file does not
match the facts.

Usage:
    ./tools/gen_messages.py            # regenerate the data file
    ./tools/gen_messages.py --check    # fail if the file is out of date
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_FACTS = Path("tools/message_facts.json")
DEFAULT_OUTPUT = Path("korangar/archive/data/client/messages.ron")

#: Rust enum variant of every domain, in the order the client expects the
#: domains in `messages.ron` (`MessageDomain::ALL` in `korangar-messages`).
DOMAIN_VARIANTS = [
    "PartyMemberRemoved",
    "PartyInviteReply",
    "PartyCreation",
    "BuyItem",
    "SkillUseFailed",
    "AmmunitionAction",
    "MapServerBan",
    "LoginFailed",
    "LoginFailed2",
]

#: `display` values the client understands.
DISPLAY_VALUES = ("current", "official")

#: Named placeholder, e.g. `{character_name}`.
PLACEHOLDER = re.compile(r"\{([a-z_][a-z0-9_]*)\}")
#: printf-style conversion of the official-client texts, e.g. `%s`, `%d`, `%02d`.
PRINTF_CONVERSION = re.compile(r"%(?:[-+ 0#]*\d*(?:\.\d+)?)?[sdliuf]")
KEY = re.compile(r"^[a-z0-9]+(?:[._][a-z0-9]+)+$")


class ValidationError(ValueError):
    pass


def fail(message: str) -> None:
    raise ValidationError(message)


def placeholder_names(template: str) -> list[str]:
    return PLACEHOLDER.findall(template)


def printf_conversions(template: str) -> int:
    return len(PRINTF_CONVERSION.findall(template))


def validate_facts(payload: dict) -> None:
    if payload.get("version") != 1:
        fail(f"unsupported facts version {payload.get('version')!r}")
    domains = payload.get("domains")
    if not isinstance(domains, list) or not domains:
        fail("the facts file contains no domains")

    seen_keys: set[str] = set()
    seen_domains: list[str] = []
    for domain in domains:
        key = domain.get("key")
        variant = domain.get("variant")
        if variant not in DOMAIN_VARIANTS:
            fail(f"domain {key!r}: unknown Rust variant {variant!r}")
        if variant in seen_domains:
            fail(f"duplicate domain variant {variant!r}")
        seen_domains.append(variant)

        unknown = domain.get("unknown", {})
        unknown_template = unknown.get("template")
        unknown_args = unknown.get("args", [])
        if not isinstance(unknown_template, str) or not unknown_template:
            fail(f"domain {key!r}: empty unknown fallback")
        allowed = set(unknown_args) | {"code"}
        for name in placeholder_names(unknown_template):
            if name not in allowed:
                fail(f"domain {key!r}: unknown fallback uses undeclared argument {{{name}}}")

        rows = domain.get("rows")
        if not isinstance(rows, list) or not rows:
            fail(f"domain {key!r}: no rows")
        codes = [row.get("code") for row in rows]
        if codes != sorted(codes):
            fail(f"domain {key!r}: rows are not sorted by code: {codes}")
        if len(set(codes)) != len(codes):
            fail(f"domain {key!r}: duplicate result codes: {codes}")

        for row in rows:
            code = row["code"]
            context = f"{key}/{code}"
            if not isinstance(code, int) or not 0 <= code <= 0xFFFF:
                fail(f"{context}: result code is not a valid u16")
            row_key = row.get("key")
            if not isinstance(row_key, str) or not KEY.match(row_key):
                fail(f"{context}: invalid message key {row_key!r}")
            if row_key in seen_keys:
                fail(f"{context}: duplicate message key {row_key!r}")
            seen_keys.add(row_key)

            args = row.get("args", [])
            default = row.get("default")
            if not isinstance(default, str) or not default:
                fail(f"{context}: empty default text")
            allowed = set(args) | {"code"}
            used = placeholder_names(default)
            for name in used:
                if name not in allowed:
                    fail(f"{context}: default text uses undeclared argument {{{name}}}")

            display = row.get("display")
            if display not in DISPLAY_VALUES:
                fail(f"{context}: invalid display {display!r}")
            official = row.get("official")
            msg_id = row.get("msg_id")
            if official is not None:
                if not isinstance(official, str) or not official:
                    fail(f"{context}: empty official text")
                if not isinstance(msg_id, int):
                    fail(f"{context}: official text without a msg id")
                conversions = printf_conversions(official)
                if conversions > len(args):
                    fail(f"{context}: official text has {conversions} placeholders but only {len(args)} args")
                for name in placeholder_names(official):
                    if name not in allowed:
                        fail(f"{context}: official text uses undeclared argument {{{name}}}")
            if display == "official" and official is None:
                fail(f"{context}: display is 'official' but no official text is available")

    if seen_domains != DOMAIN_VARIANTS:
        fail(f"domains are missing or out of order: {seen_domains}")


def ron_string(value: str) -> str:
    escaped = value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t")
    return f'"{escaped}"'


def ron_optional_string(value: str | None) -> str:
    return "None" if value is None else f"Some({ron_string(value)})"


def ron_optional_u16(value: int | None) -> str:
    return "None" if value is None else f"Some({value})"


def ron_args(args: list[str]) -> str:
    return "[" + ", ".join(ron_string(arg) for arg in args) + "]"


def render(payload: dict) -> str:
    sources = payload["sources"]
    rathena = sources["rathena"]
    messages = sources["official_client_messages"]
    client = sources["client_defaults"]
    lines: list[str] = [
        "// Protocol result-code -> user-facing message table.",
        "//",
        "// Generated by `tools/gen_messages.py` from `tools/message_facts.json`;",
        "// do not edit by hand.",
        "// Regenerate with: `./tools/gen_messages.py`",
        "// Drift check:      `./tools/gen_messages.py --check`",
        "// Schema check:     `korangar --check-messages`",
        "//",
        "// The table replaces the per-code English `match` arms that used to live in",
        "// `korangar/src/lib.rs` (party/buy/skill-fail/ammo/ban) and",
        "// `korangar-networking/src/packet_versions/version_20220406.rs` (login).",
        "// Protocol opcodes and result enums are untouched.",
        "//",
        "// Facts extracted from the rAthena packet documentation",
        f"// ({rathena['repository']} @ {rathena['revision'][:12]}:",
        "// src/map/clif.cpp, src/map/clif.hpp, src/map/mmo.hpp, src/login/loginclif.cpp)",
        f"// and the official client message table ({messages['archive']}",
        f"// {messages['path']}, {messages['entries']} entries); the skill-fail cause -> msg id",
        "// mapping is the official client behaviour from roBrowserLegacy.",
        "// Current default texts are copied from the pre-table client",
        f"// (korangar @ {client['revision'][:12]}: korangar/src/lib.rs,",
        "// korangar-networking/src/packet_versions/version_20220406.rs) and are rendered",
        "// byte for byte (`display: Current`); `official`/`msg_id` are the official",
        "// entries for the later localization pass (`display: Official`), reviewed row",
        "// by row. `default` uses `{argument}` placeholders, `{code}` is the result code;",
        "// `args` is the substitution order for the official-client `%s`/`%d` texts.",
        "//",
        "// The file is loaded at runtime through the normal client file loader, so a",
        "// server can override it without rebuilding the client; the compiled Rust",
        "// table is only the fallback for missing files.",
        "(",
        f"    provenance: {ron_string(payload['provenance'])},",
        "    version: 1,",
        "    domains: [",
    ]

    for domain in payload["domains"]:
        unknown = domain["unknown"]
        lines.append("        (")
        lines.append(f"            domain: {domain['variant']},")
        lines.append(f"            unknown: {ron_string(unknown['template'])},")
        lines.append(f"            unknown_args: {ron_args(unknown.get('args', []))},")
        lines.append("            rows: [")
        for row in domain["rows"]:
            lines.append("                (")
            lines.append(f"                    code: {row['code']},")
            lines.append(f"                    key: {ron_string(row['key'])},")
            lines.append(f"                    msg_id: {ron_optional_u16(row.get('msg_id'))},")
            lines.append(f"                    official: {ron_optional_string(row.get('official'))},")
            lines.append(f"                    default: {ron_string(row['default'])},")
            lines.append(f"                    args: {ron_args(row.get('args', []))},")
            lines.append(f"                    display: {row['display'].capitalize()},")
            lines.append("                ),")
        lines.append("            ],")
        lines.append("        ),")
    lines.extend(["    ],", ")", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if the committed data file is out of date")
    parser.add_argument("--facts", default=str(DEFAULT_FACTS))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    payload = json.loads(Path(args.facts).read_text(encoding="utf-8"))
    try:
        validate_facts(payload)
    except ValidationError as error:
        print(f"error: {args.facts}: {error}", file=sys.stderr)
        return 1

    content = render(payload)
    output = Path(args.output)
    row_count = sum(len(domain["rows"]) for domain in payload["domains"])

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/gen_messages.py", file=sys.stderr)
            return 1
        print(f"{output} is up to date ({row_count} rows in {len(payload['domains'])} domains)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    print(f"{output}: {row_count} rows in {len(payload['domains'])} domains")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
