#!/usr/bin/env python3
"""Extract the facts for the data-driven protocol result-code message table.

The client used to map protocol result codes to English sentences with
`match` arms in Rust (`korangar/src/lib.rs` for the map-server packets and
`korangar-networking/src/packet_versions/version_20220406.rs` for the login
packets). This extractor produces the committed fact file that replaces them:

    tools/message_facts.json

Two sources are combined, per the audit (`notes/client-hardcoded-systems-audit.md`
§4.4 / §8 Phase 0 item 3):

* **rAthena** (`https://github.com/rathena/rathena`, pinned revision) — the
  protocol result codes and the `MsgStringTable` ids/texts the official client
  uses for them. Parsed from the packet-documentation comments in
  `src/map/clif.cpp` and `src/login/loginclif.cpp`, and from the enums in
  `src/map/clif.hpp` / `src/map/mmo.hpp`. Only plain facts (numeric ids, enum
  names, message ids, one-line message texts) are extracted; no rAthena code or
  comments are copied.
* **client `data\\msgstringtable.txt`** (CondaRO.grf) — the actual English
  display text of every `MsgStringTable` id, so a msg id can be validated and
  turned into the official text. Edition note: the extracted CondaRO table has
  one stray leading line that does not end with the classic `#` terminator
  (a Node.js warning), which shifts every id by one; the loader drops such
  leading lines before indexing, exactly like `korangar/src/msgstringtable.rs`.

The **current client default texts are not in the server sources**, so this
script embeds them, together with their source location at the integration
revision (`korangar/src/lib.rs:<lines>`, `.../version_20220406.rs:<lines>`).
They are the behaviour-preservation baseline: P0 renders these strings byte for
byte, the official texts are committed next to them for the later localization
pass.

One client-side fact is not in rAthena at all: the official client maps
`ZC_ACK_TOUSESKILL` `cause` values to msg ids itself
(`USESKILL_FAIL_*` -> `MsgStringTable`). The mapping below is the documented
reverse-engineered official-client behaviour from roBrowserLegacy
(`src/Engine/MapEngine/Skill.js`, `onSkillResult`), which is the same source the
repository already uses for the official skill tables.

Usage:
    ./tools/extract_message_facts.py --rathena-dir /path/to/rathena \\
        --msgstringtable /path/to/client/game/data/msgstringtable.txt
    ./tools/extract_message_facts.py --check   # drift guard
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

DEFAULT_RATHENA_DIR = Path("/home/phantom/rathena")
DEFAULT_MSGSTRINGTABLE = Path("/home/phantom/Games/ragnarok/CondaRo-client/game/data/msgstringtable.txt")
DEFAULT_OUTPUT = Path("tools/message_facts.json")

#: rAthena revision the committed facts were extracted from.
RATHENA_COMMIT = "226bd83d39584bc91b6bf133f40bce95aa1f178c"
RATHENA_REPOSITORY = "https://github.com/rathena/rathena"

#: roBrowserLegacy revision/source of the official-client skill-fail mapping.
ROBROWSER_REPOSITORY = "https://github.com/MrAntares/roBrowserLegacy"
ROBROWSER_FILE = "src/Engine/MapEngine/Skill.js"

#: Client checkout the default texts were copied from.
CLIENT_COMMIT = "f454127e992af85e555ee759b9afb2a9ca649103"

#: `useskill_fail_cause` -> `MsgStringTable` id, official client (roBrowserLegacy
#: `onSkillResult`). Causes the official client leaves without a message (0, 11,
#: 12, 14, 22, 23, 26) are not listed and get no msg id.
OFFICIAL_CLIENT_SKILL_FAIL_IDS = {
    1: 202,  # not enough SP
    2: 203,  # not enough HP
    3: 808,  # not enough items
    4: 219,  # skill still in delay
    5: 233,  # not enough zeny
    6: 239,  # wrong weapon
    7: 246,  # missing red gemstone
    8: 247,  # missing blue gemstone
    9: 580,  # too much weight
    13: 1398,  # missing holy water
}

#: rAthena packet documentation gives a `MsgStringTable` id for these codes, but
#: the id is off by one in the 2019 CondaRO English table. The text search finds
#: the actual entry; the original rAthena id is kept in `rathena_msg_id`.
BAN_CODE_8_NEEDLE = "Server still recognizes your last log-in"

#: `ZC_PC_PURCHASE_RESULT` (NPC shop) results: rAthena documents only the English
#: texts, not the msg ids. These ids are the `MsgStringTable` entries whose text
#: matches the rAthena text (verified once against the CondaRO table; the
#: `source` of the row records that they are text matches, not rAthena ids).
BUY_RESULT_MSG_IDS = {
    1: 55,  # "You do not have enough zeny."
    2: 56,  # "You are over your Weight Limit."
    3: 221,  # "Out of the maximum capacity"
    9: 1940,  # "Amounts are exceeded the possession of the item..."
    10: 2917,  # "Street vending sales will be traded on RODEX"
    11: 3552,  # "Exchange failed."
    12: 3553,  # "Exchange successfuly completed."
    13: 3555,  # "The item has been sold and out of stock."
    14: 3554,  # "Not enough items to exchange."
}

#: `AC_REFUSE_LOGIN2` codes with a client message that is not a plain
#: `MsgStringTable` text: the 2019 client displays MSI_* entries as its own
#: built-in strings. The msg id is still recorded (provenance), the official
#: text is `None` because the table only contains the macro token.
MSI_PREFIX = "MSI_"

#: `AC_REFUSE_LOGIN2` codes whose client text is a hardcoded built-in string
#: (verified against the CondaRO English table): id, or `None`.
LOGIN2_BUILTIN_IDS = {
    0: 6,  # "Unregistered ID. Please make sure..."
    1: 7,  # "Incorrect User ID or Password..."
    2: 8,  # "This ID is expired."
    3: 9,  # "Rejected from Server."
    4: 705,  # MSI_REFUSE_BAN_BY_GM: the client substitutes its own string
    6: 449,  # "You are prohibited to log in until %s."
    7: 264,  # "Server is jammed due to over population..."
    100: 809,  # "Login information remains at %s."
    101: 810,  # hacking investigation
    102: 811,  # bug-related investigation
    103: 859,  # character being deleted
    104: 860,  # spouse character being deleted
}


# ---------------------------------------------------------------------------
# rAthena parsing.
# ---------------------------------------------------------------------------


class ExtractionError(RuntimeError):
    pass


def read_source(rathena_dir: Path, relative: str) -> str:
    path = rathena_dir / relative
    if not path.is_file():
        raise ExtractionError(f"missing rAthena source file: {path}")
    return path.read_text(encoding="utf-8", errors="replace")


def doc_block_before(source: str, marker: str) -> list[str]:
    """Return the documentation comment lines directly before `marker`."""
    try:
        index = source.index(marker)
    except ValueError as error:
        raise ExtractionError(f"marker not found in rAthena sources: {marker}") from error

    lines = source[:index].splitlines()
    index = len(lines) - 1
    while index >= 0 and not lines[index].strip():
        index -= 1

    collected: list[str] = []
    if index >= 0 and lines[index].rstrip().endswith("*/"):
        while index >= 0 and not lines[index].lstrip().startswith("/*"):
            collected.append(lines[index])
            index -= 1
        if index >= 0:
            collected.append(lines[index])
        collected.reverse()
        return collected

    while index >= 0 and lines[index].lstrip().startswith("///"):
        collected.append(lines[index])
        index -= 1
    collected.reverse()
    return collected


def doc_text(line: str) -> str:
    """Strip the comment decoration from a documentation line."""
    line = line.strip()
    for prefix in ("///", "/*", "*/", "*"):
        if line.startswith(prefix):
            line = line[len(prefix) :]
            break
    return line.strip()


CODE_ROW = re.compile(r"^(\d+)\s*=\s*(.*)$")


def parse_code_rows(doc: list[str]) -> dict[int, str]:
    """Parse `N = text` / `result=N : text` documentation rows."""
    rows: dict[int, str] = {}
    current: int | None = None
    for line in doc:
        text = doc_text(line)
        match = CODE_ROW.match(text)
        if not match:
            result_row = re.search(r"result=(\d+)\s*:\s*(.*)", text)
            if result_row:
                rows[int(result_row.group(1))] = result_row.group(2).strip()
            continue
        current = int(match.group(1))
        rows[current] = match.group(2).strip()
    return rows


MSG_ID = re.compile(r"MsgStringTable\[(\d+)\]")
MSG_ID_TEXT = re.compile(r'MsgStringTable\[(\d+)\]\s*=\s*"([^"]*)"')


def parse_party_invite_reply(clif: str) -> dict[int, int | None]:
    doc = doc_block_before(clif, "void clif_party_invite_reply")
    ids: dict[int, int | None] = {}
    for line in doc:
        text = doc_text(line)
        match = re.search(r"result=(\d+)\s*:\s*(.*)", text)
        if not match:
            continue
        code = int(match.group(1))
        rest = match.group(2)
        msg = MSG_ID.search(rest)
        ids[code] = int(msg.group(1)) if msg else None
    return ids


def parse_party_created(clif: str) -> dict[int, int | None]:
    doc = doc_block_before(clif, "void clif_party_created")
    ids: dict[int, int | None] = {}
    for code, text in parse_code_rows(doc).items():
        if code > 3:
            continue
        msg = MSG_ID.search(text)
        ids[code] = int(msg.group(1)) if msg else None
    return ids


def parse_ammunition_action(clif: str) -> dict[int, list[int]]:
    """Parse `ZC_ACTION_FAILURE` -> msg ids (code 3 lists three job variants)."""
    doc = doc_block_before(clif, "void clif_arrow_fail")
    ids: dict[int, list[int]] = {}
    current: int | None = None
    for line in doc:
        text = doc_text(line)
        match = CODE_ROW.match(text)
        if match:
            current = int(match.group(1))
            ids.setdefault(current, [])
        if current is not None:
            for msg in MSG_ID.finditer(text):
                value = int(msg.group(1))
                if value not in ids[current]:
                    ids[current].append(value)
    return ids


def parse_map_server_ban(clif: str) -> dict[int, int]:
    doc = doc_block_before(clif, "void clif_authfail_fd")
    ids: dict[int, int] = {}
    for line in doc:
        text = doc_text(line)
        match = CODE_ROW.match(text)
        if not match:
            continue
        msg = MSG_ID.search(match.group(2))
        if msg:
            ids[int(match.group(1))] = int(msg.group(1))
    return ids


def parse_buy_result_texts(clif: str) -> dict[int, str]:
    doc = doc_block_before(clif, "void clif_npc_buy_result")
    return {code: text.strip('"') for code, text in parse_code_rows(doc).items()}


def parse_login_failed_login_server(loginclif: str) -> dict[int, int | None]:
    """`LoginFailedPacket` (SC_NOTIFY_BAN, 0x0081) codes 1/2/8 on the login server."""
    doc = doc_block_before(loginclif, "static void logclif_sent_auth_result")
    rows: dict[int, int | None] = {}
    for line in doc:
        text = doc_text(line)
        match = CODE_ROW.match(text)
        if not match:
            continue
        msg = MSG_ID.search(match.group(2))
        rows[int(match.group(1))] = int(msg.group(1)) if msg else None
    return rows


def parse_login_failed_login_server2(loginclif: str) -> dict[int, str]:
    """`AC_REFUSE_LOGIN2` code -> documented server-side English text."""
    doc = doc_block_before(loginclif, "static void logclif_auth_failed(struct login_session_data* sd, int32 result)")
    rows: dict[int, str] = {}
    for line in doc:
        text = doc_text(line)
        match = CODE_ROW.match(text)
        if match:
            rows[int(match.group(1))] = match.group(2).strip()
        elif text.lower().startswith("default ="):
            rows[-1] = text.split("=", 1)[1].strip()
    return rows


def parse_skill_fail_enum(clif_hpp: str) -> dict[int, str]:
    """`useskill_fail_cause` numeric value -> enum name."""
    values: dict[int, str] = {}
    for match in re.finditer(r"(USESKILL_FAIL_[A-Z0-9_]+)\s*=\s*(\d+)", clif_hpp):
        values[int(match.group(2))] = match.group(1)
    return values


def parse_party_member_withdraw_enum(mmo_hpp: str) -> dict[int, str]:
    values: dict[int, str] = {}
    block = re.search(r"enum e_party_member_withdraw\s*\{(.*?)\};", mmo_hpp, re.S)
    if block:
        for index, line in enumerate(block.group(1).splitlines()):
            match = re.match(r"\s*(PARTY_MEMBER_WITHDRAW_[A-Z_]+)", line)
            if match:
                values[index] = match.group(1)
    return values


# ---------------------------------------------------------------------------
# msgstringtable parsing.
# ---------------------------------------------------------------------------


def load_msgstringtable(path: Path) -> tuple[list[str | None], int]:
    """Return (entries indexed by msg id, number of dropped leading lines)."""
    if not path.is_file():
        raise ExtractionError(f"msgstringtable not found: {path}")
    text = path.read_bytes().decode("euc-kr", errors="replace")
    lines = text.splitlines()

    dropped = 0
    # The classic table terminates every line with '#'; a leading line without
    # the terminator is not a message (the CondaRO extraction has one such
    # Node.js warning line in front of the table).
    while lines and not lines[0].rstrip().endswith("#"):
        lines.pop(0)
        dropped += 1

    entries: list[str | None] = []
    for line in lines:
        line = line.strip()
        if not line.endswith("#"):
            entries.append(None)
            continue
        message = line[:-1].strip()
        if not message or message.startswith("#"):
            entries.append(None)
            continue
        entries.append(message)
    return entries, dropped


def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text).strip().lower()


def find_msg_id(entries: list[str | None], needle: str) -> int | None:
    """Find the table entry whose text starts with (or equals) `needle`."""
    wanted = normalize(needle)
    for index, text in enumerate(entries):
        if text is None:
            continue
        if normalize(text).startswith(wanted):
            return index
    return None


# ---------------------------------------------------------------------------
# Domain specification: the client's current default texts.
# ---------------------------------------------------------------------------

PARTY_MEMBER_REMOVED = [
    # (result code, key, args, default, client source lines)
    (0, "party.member_removed.left", ["character_name"], "{character_name} left the party.", "lib.rs:5276"),
    (1, "party.member_removed.expelled", ["character_name"], "{character_name} was expelled from the party.", "lib.rs:5277"),
    (2, "party.member_removed.cannot_leave", ["character_name"], "{character_name} could not leave the party.", "lib.rs:5278"),
    (3, "party.member_removed.cannot_expel", ["character_name"], "{character_name} could not be expelled from the party.", "lib.rs:5279"),
]

PARTY_INVITE_REPLY = [
    (0, "party.invite.already_in_party", ["character_name"], "{character_name} is already in a party.", "lib.rs:5345"),
    (1, "party.invite.rejected", ["character_name"], "{character_name} rejected the party invitation.", "lib.rs:5346"),
    (2, "party.invite.accepted", ["character_name"], "{character_name} accepted the party invitation.", "lib.rs:5347"),
    (3, "party.invite.party_full", [], "The party is full.", "lib.rs:5348"),
    (
        4,
        "party.invite.account_already_in_party",
        ["character_name"],
        "A character of {character_name}'s account is already in the party.",
        "lib.rs:5349",
    ),
    (5, "party.invite.blocked", ["character_name"], "{character_name} blocks party invitations.", "lib.rs:5350"),
    (7, "party.invite.offline", ["character_name"], "{character_name} is not online.", "lib.rs:5351"),
    (
        8,
        "party.invite.restricted_map",
        ["character_name"],
        "{character_name} is on a map where parties cannot be joined.",
        "lib.rs:5352",
    ),
    (9, "party.invite.cannot_invite_on_map", [], "You cannot invite on this map.", "lib.rs:5353"),
    (
        10,
        "party.invite.memorial_dungeon",
        [],
        "Party changes are not possible in a memorial dungeon.",
        "lib.rs:5354",
    ),
    (
        11,
        "party.invite.level_restriction",
        ["character_name"],
        "{character_name} does not meet the party level restrictions.",
        "lib.rs:5355",
    ),
]

PARTY_CREATION = [
    (0, "party.creation.created", [], "Party created.", "lib.rs:5363"),
    (1, "party.creation.name_exists", [], "A party with that name already exists.", "lib.rs:5364"),
    (2, "party.creation.already_in_party", [], "You are already in a party.", "lib.rs:5365"),
    (3, "party.creation.cannot_create_on_map", [], "You cannot create a party on this map.", "lib.rs:5366"),
]

BUY_ITEM = [
    (1, "shop.buy.not_enough_zeny", [], "You do not have enough zeny.", "lib.rs:5681"),
    (2, "shop.buy.weight_limit", [], "You are over your weight limit.", "lib.rs:5682"),
    (3, "shop.buy.too_many_items", [], "You have too many items.", "lib.rs:5683"),
    (9, "shop.buy.too_many_of_this_item", [], "You cannot hold that many of this item.", "lib.rs:5684"),
    (
        10,
        "shop.buy.open_air_shop",
        [],
        "Open-air shop purchases have to be traded through RODEX.",
        "lib.rs:5685",
    ),
    (11, "shop.buy.exchange_failed", [], "The exchange failed.", "lib.rs:5686"),
    (12, "shop.buy.exchange_well_done", [], "The exchange was well done.", "lib.rs:5687"),
    (13, "shop.buy.item_sold_out", [], "The item is already sold out.", "lib.rs:5688"),
    (14, "shop.buy.not_enough_goods", [], "There are not enough goods to exchange.", "lib.rs:5689"),
]

SKILL_USE_FAILED = [
    (0, "skill.fail.level_too_low", ["skill_id"], "Cannot use skill {skill_id}: skill level is too low.", "lib.rs:5812"),
    (1, "skill.fail.not_enough_sp", ["skill_id"], "Cannot use skill {skill_id}: not enough SP.", "lib.rs:5813"),
    (2, "skill.fail.not_enough_hp", ["skill_id"], "Cannot use skill {skill_id}: not enough HP.", "lib.rs:5814"),
    (3, "skill.fail.not_enough_items", ["skill_id"], "Cannot use skill {skill_id}: not enough items.", "lib.rs:5815"),
    (4, "skill.fail.still_in_delay", ["skill_id"], "Cannot use skill {skill_id}: skill is still in delay.", "lib.rs:5816"),
    (5, "skill.fail.not_enough_zeny", ["skill_id"], "Cannot use skill {skill_id}: not enough zeny.", "lib.rs:5817"),
    (6, "skill.fail.wrong_weapon", ["skill_id"], "Cannot use skill {skill_id}: wrong weapon.", "lib.rs:5818"),
    (
        7,
        "skill.fail.missing_red_gemstone",
        ["skill_id"],
        "Cannot use skill {skill_id}: missing red gemstone.",
        "lib.rs:5819",
    ),
    (
        8,
        "skill.fail.missing_blue_gemstone",
        ["skill_id"],
        "Cannot use skill {skill_id}: missing blue gemstone.",
        "lib.rs:5820",
    ),
    (9, "skill.fail.too_much_weight", ["skill_id"], "Cannot use skill {skill_id}: too much weight.", "lib.rs:5821"),
    (11, "skill.fail.invalid_target", ["skill_id"], "Cannot use skill {skill_id}: invalid target.", "lib.rs:5823"),
    (12, "skill.fail.too_many_ancilla", ["skill_id"], "Cannot use skill {skill_id}: too many ancilla.", "lib.rs:5824"),
    (
        13,
        "skill.fail.missing_holy_water",
        ["skill_id"],
        "Cannot use skill {skill_id}: missing holy water.",
        "lib.rs:5825",
    ),
    (14, "skill.fail.missing_ancilla", ["skill_id"], "Cannot use skill {skill_id}: missing ancilla.", "lib.rs:5826"),
    (22, "skill.fail.already_active", ["skill_id"], "Cannot use skill {skill_id}: skill is already active.", "lib.rs:5827"),
    (23, "skill.fail.condition_not_met", ["skill_id"], "Cannot use skill {skill_id}: condition not met.", "lib.rs:5828"),
    (26, "skill.fail.invalid_position", ["skill_id"], "Cannot use skill {skill_id}: invalid position.", "lib.rs:5829"),
]

AMMUNITION_ACTION = [
    (
        0,
        "ammunition.equip_proper_first",
        [],
        "You have to equip proper ammunition first.",
        "lib.rs:5847",
    ),
    (
        1,
        "ammunition.weight_limit_1",
        [],
        "You cannot carry that much ammunition (weight limit).",
        "lib.rs:5848",
    ),
    (
        2,
        "ammunition.weight_limit_2",
        [],
        "You cannot carry that much ammunition (weight limit).",
        "lib.rs:5849",
    ),
    (3, "ammunition.equipped", [], "Ammunition equipped.", "lib.rs:5850"),
]

MAP_SERVER_BAN = [
    (1, "map_ban.server_closed", [], "Disconnected: the server was closed.", "lib.rs:5864"),
    (2, "map_ban.already_logged_in", [], "Disconnected: this account is already logged in.", "lib.rs:5865"),
    (3, "map_ban.connection_timed_out", [], "Disconnected: connection timed out.", "lib.rs:5866"),
    (4, "map_ban.server_full", [], "Disconnected: the server is full.", "lib.rs:5867"),
    (5, "map_ban.account_restriction", [], "Disconnected: account restriction.", "lib.rs:5868"),
    (
        8,
        "map_ban.previous_connection",
        [],
        "Disconnected: the server still recognizes the previous connection, please try again.",
        "lib.rs:5869",
    ),
    (
        9,
        "map_ban.too_many_connections",
        [],
        "Disconnected: too many connections from this IP.",
        "lib.rs:5870",
    ),
]

LOGIN_FAILED = [
    (1, "login.failed.server_closed", [], "Server closed", "version_20220406.rs:42"),
    (2, "login.failed.already_logged_in", [], "Someone has already logged in with this id", "version_20220406.rs:45"),
    (8, "login.failed.already_online", [], "Already online", "version_20220406.rs:48"),
]

LOGIN_FAILED2 = [
    (0, "login.failed2.unregistered_id", [], "Unregistered id", "version_20220406.rs:55"),
    (1, "login.failed2.incorrect_password", [], "Incorrect password", "version_20220406.rs:56"),
    (2, "login.failed2.id_expired", [], "Id has expired", "version_20220406.rs:57"),
    (3, "login.failed2.rejected_from_server", [], "Rejected from server", "version_20220406.rs:58"),
    (4, "login.failed2.blocked_by_gm", [], "Blocked by gm team", "version_20220406.rs:59"),
    (5, "login.failed2.game_outdated", [], "Game outdated", "version_20220406.rs:60"),
    (
        6,
        "login.failed2.login_prohibited_until",
        ["date"],
        "You are prohibited to log in until {date}.",
        "version_20220406.rs:61",
    ),
    (7, "login.failed2.server_full", [], "Server is full", "version_20220406.rs:64"),
    (
        8,
        "login.failed2.company_account_limit",
        [],
        "Company account limit reached",
        "version_20220406.rs:66",
    ),
    (9, "login.failed2.banned_by_dba", [], "Banned by DBA team", "version_20220406.rs:69"),
    (10, "login.failed2.unconfirmed_email", [], "Email not confirmed", "version_20220406.rs:70"),
    (11, "login.failed2.banned_by_gm", [], "Banned by GM team", "version_20220406.rs:72"),
    (
        12,
        "login.failed2.temporary_ban_database_work",
        [],
        "Working in DB",
        "version_20220406.rs:74",
    ),
    (13, "login.failed2.self_locked", [], "Self lock", "version_20220406.rs:77"),
    (14, "login.failed2.not_permitted_group", [], "Not Permitted Group", "version_20220406.rs:78"),
    (
        99,
        "login.failed2.account_id_erased",
        [],
        "This ID has been totally erased",
        "version_20220406.rs:80",
    ),
    (
        100,
        "login.failed2.login_information_remains",
        ["date"],
        "Login information remains at {date}",
        "version_20220406.rs:81",
    ),
    (
        101,
        "login.failed2.locked_for_hacking_investigation",
        [],
        "Account has been locked for a hacking investigation. Please contact the GM Team for more information",
        "version_20220406.rs:85",
    ),
    (
        102,
        "login.failed2.temporary_locked_for_bug_investigation",
        [],
        "This account has been temporarily prohibited from login due to a bug-related investigation",
        "version_20220406.rs:90",
    ),
    (
        103,
        "login.failed2.deleting_character",
        [],
        "This character is being deleted. Login is temporarily unavailable for the time being",
        "version_20220406.rs:93",
    ),
    (
        104,
        "login.failed2.deleting_spouse_character",
        [],
        "This character is being deleted. Login is temporarily unavailable for the time being",
        "version_20220406.rs:98",
    ),
    (105, "login.failed2.unknown_error", [], "Unknown error", "version_20220406.rs:101"),
]

DOMAINS = [
    {
        "key": "party_member_removed",
        "variant": "PartyMemberRemoved",
        "server_source": "src/map/mmo.hpp enum e_party_member_withdraw + ZC_DELETE_MEMBER_FROM_GROUP (0x0105) docs",
        "unknown": ("Party member {character_name} left the party.", ["character_name"]),
        "rows": PARTY_MEMBER_REMOVED,
        "msg_ids": "none",
    },
    {
        "key": "party_invite_reply",
        "variant": "PartyInviteReply",
        "server_source": "src/map/clif.cpp clif_party_invite_reply (ZC_PARTY_JOIN_REQ_ACK 0x02C5) docs",
        "unknown": ("Party invitation result: {code}.", []),
        "rows": PARTY_INVITE_REPLY,
        "msg_ids": "rathena",
    },
    {
        "key": "party_creation",
        "variant": "PartyCreation",
        "server_source": "src/map/clif.cpp clif_party_created (ZC_ACK_MAKE_GROUP 0x00FA) docs",
        "unknown": ("Party creation failed (result {code}).", []),
        "rows": PARTY_CREATION,
        "msg_ids": "rathena",
    },
    {
        "key": "buy_item",
        "variant": "BuyItem",
        "server_source": "src/map/clif.cpp clif_npc_buy_result (ZC_PC_PURCHASE_RESULT 0x00CA) docs",
        "unknown": ("Shop purchase failed (code {code}).", []),
        "rows": BUY_ITEM,
        "msg_ids": "text",
    },
    {
        "key": "skill_use_failed",
        "variant": "SkillUseFailed",
        "server_source": "src/map/clif.hpp enum useskill_fail_cause + ZC_ACK_TOUSESKILL (0x0110) docs; client msg ids from roBrowserLegacy onSkillResult",
        "unknown": ("Cannot use skill {skill_id}: the skill cannot be used.", ["skill_id"]),
        "rows": SKILL_USE_FAILED,
        "msg_ids": "official_client",
    },
    {
        "key": "ammunition_action",
        "variant": "AmmunitionAction",
        "server_source": "src/map/clif.cpp clif_arrow_fail (ZC_ACTION_FAILURE 0x013B) docs",
        "unknown": ("Unknown ammunition action (code {code}).", []),
        "rows": AMMUNITION_ACTION,
        "msg_ids": "rathena",
    },
    {
        "key": "map_server_ban",
        "variant": "MapServerBan",
        "server_source": "src/map/clif.cpp clif_authfail_fd (SC_NOTIFY_BAN 0x0081) docs",
        "unknown": ("Disconnected: the server closed the connection.", []),
        "rows": MAP_SERVER_BAN,
        "msg_ids": "rathena",
    },
    {
        "key": "login_failed",
        "variant": "LoginFailed",
        "server_source": "src/login/loginclif.cpp logclif_sent_auth_result (SC_NOTIFY_BAN 0x0081) docs",
        "unknown": ("Login failed (code {code}).", []),
        "rows": LOGIN_FAILED,
        "msg_ids": "rathena",
    },
    {
        "key": "login_failed2",
        "variant": "LoginFailed2",
        "server_source": "src/login/loginclif.cpp logclif_auth_failed (AC_REFUSE_LOGIN2 0x0840) docs",
        "unknown": ("Login failed (code {code}).", []),
        "rows": LOGIN_FAILED2,
        "msg_ids": "login2",
    },
]


def official_text(entries: list[str | None], msg_id: int | None) -> str | None:
    if msg_id is None or msg_id >= len(entries):
        return None
    text = entries[msg_id]
    if text is None or text.startswith(MSI_PREFIX):
        return None
    return text


def build_payload(rathena_dir: Path, msgstringtable: Path) -> dict:
    clif = read_source(rathena_dir, "src/map/clif.cpp")
    clif_hpp = read_source(rathena_dir, "src/map/clif.hpp")
    mmo_hpp = read_source(rathena_dir, "src/map/mmo.hpp")
    loginclif = read_source(rathena_dir, "src/login/loginclif.cpp")

    entries, dropped = load_msgstringtable(msgstringtable)

    party_invite_ids = parse_party_invite_reply(clif)
    party_created_ids = parse_party_created(clif)
    ammunition_ids = parse_ammunition_action(clif)
    ban_ids = parse_map_server_ban(clif)
    buy_texts = parse_buy_result_texts(clif)
    login_ids = parse_login_failed_login_server(loginclif)
    login2_texts = parse_login_failed_login_server2(loginclif)
    skill_fail_enum = parse_skill_fail_enum(clif_hpp)
    withdraw_enum = parse_party_member_withdraw_enum(mmo_hpp)

    def rathena_id_for(domain: dict, code: int) -> int | None:
        if domain["msg_ids"] == "rathena":
            if domain["key"] == "party_invite_reply":
                return party_invite_ids.get(code)
            if domain["key"] == "party_creation":
                return party_created_ids.get(code)
            if domain["key"] == "map_server_ban":
                return ban_ids.get(code)
            if domain["key"] == "login_failed":
                return login_ids.get(code)
        return None

    domains = []
    for domain in DOMAINS:
        rows = []
        for code, key, args, default, client_source in domain["rows"]:
            msg_id: int | None
            rathena_msg_id: int | None = None
            msg_id_source: str

            if domain["msg_ids"] == "none":
                msg_id = None
                msg_id_source = "none: the server only sends a result code, the client composes the text from the member name"
            elif domain["msg_ids"] == "official_client":
                if code not in skill_fail_enum:
                    raise ExtractionError(f"skill fail cause {code} is not in the rAthena enum")
                msg_id = OFFICIAL_CLIENT_SKILL_FAIL_IDS.get(code)
                msg_id_source = (
                    "official client (roBrowserLegacy onSkillResult)"
                    if msg_id is not None
                    else "none: the official client shows no message for this cause"
                )
            elif domain["msg_ids"] == "text":
                if code not in buy_texts:
                    raise ExtractionError(f"buy result code {code} is not documented by rAthena")
                msg_id = BUY_RESULT_MSG_IDS.get(code)
                expected = buy_texts[code]
                if msg_id is not None:
                    resolved = official_text(entries, msg_id)
                    if resolved is None:
                        raise ExtractionError(f"buy result {code}: msg id {msg_id} has no usable text in msgstringtable")
                    msg_id_source = f'msgstringtable text match for rAthena text "{expected}"'
                else:
                    msg_id_source = f'none: no msgstringtable entry matches rAthena text "{expected}"'
            elif domain["msg_ids"] == "login2":
                if code not in login2_texts:
                    # rAthena documents the final AC_REFUSE_LOGIN2 case as
                    # "default = Unknown Error."; the client enum commits that
                    # fallthrough as the numeric value 105.
                    if code != 105:
                        raise ExtractionError(f"login result code {code} is not documented by rAthena")
                    msg_id = None
                    msg_id_source = 'none: rAthena documents this as the "default = Unknown Error." fallthrough'
                    official = None
                    row = {
                        "code": code,
                        "key": key,
                        "args": args,
                        "default": default,
                        "display": "current",
                        "msg_id": msg_id,
                        "official": official,
                        "msg_id_source": msg_id_source,
                        "client_source": client_source,
                    }
                    rows.append(row)
                    continue
                text = login2_texts[code]
                if text.startswith(MSI_PREFIX):
                    msg_id = find_msg_id(entries, text)
                    if msg_id is None:
                        raise ExtractionError(f"login result {code}: macro {text} not found in msgstringtable")
                    msg_id_source = f"msgstringtable macro token {text} (the client substitutes its built-in string)"
                elif code in LOGIN2_BUILTIN_IDS:
                    msg_id = LOGIN2_BUILTIN_IDS[code]
                    if official_text(entries, msg_id) is None:
                        msg_id_source = f"msgstringtable macro token {entries[msg_id]} (the client substitutes its built-in string)"
                    else:
                        msg_id_source = "official client built-in message, matches msgstringtable text"
                else:
                    msg_id = find_msg_id(entries, text)
                    msg_id_source = "msgstringtable text match" if msg_id is not None else "none: no matching client message"
            elif domain["key"] == "login_failed":
                rathena_msg_id = ban_ids.get(code)
                if code == 8:
                    msg_id = find_msg_id(entries, BAN_CODE_8_NEEDLE)
                    msg_id_source = (
                        f"msgstringtable text match (rAthena id {rathena_msg_id} is off by one in the 2019 CondaRO English table)"
                    )
                else:
                    msg_id = rathena_msg_id
                    msg_id_source = "rAthena documentation (SC_NOTIFY_BAN MsgStringTable id)"
            else:
                rathena_msg_id = rathena_id_for(domain, code)
                if domain["key"] == "ammunition_action":
                    candidates = ammunition_ids.get(code, [])
                    if not candidates:
                        raise ExtractionError(f"ammunition action {code} is not documented by rAthena")
                    if len(candidates) == 1:
                        msg_id = candidates[0]
                        msg_id_source = "rAthena documentation (MsgStringTable id)"
                    else:
                        # Code 3 is job dependent (1040/1175/245). The generic
                        # client message is the one that does not name a weapon.
                        msg_id = next((candidate for candidate in candidates if "ammunition has been equipped" in normalize(entries[candidate] or "")), None)
                        msg_id_source = (
                            f"rAthena documentation; job-dependent alternatives {candidates}, "
                            'generic "Ammunition has been equipped." selected'
                        )
                elif domain["key"] == "map_server_ban" and code == 8:
                    rathena_msg_id = ban_ids.get(code)
                    msg_id = find_msg_id(entries, BAN_CODE_8_NEEDLE)
                    msg_id_source = (
                        f"msgstringtable text match (rAthena id {rathena_msg_id} is off by one in the 2019 CondaRO English table)"
                    )
                else:
                    msg_id = rathena_msg_id
                    msg_id_source = "rAthena documentation (MsgStringTable id)" if msg_id is not None else "none: rAthena documents no MsgStringTable id"

            official = official_text(entries, msg_id)
            row = {
                "code": code,
                "key": key,
                "args": args,
                "default": default,
                "display": "current",
                "msg_id": msg_id,
                "official": official,
                "msg_id_source": msg_id_source,
                "client_source": client_source,
            }
            if rathena_msg_id is not None and rathena_msg_id != msg_id:
                row["rathena_msg_id"] = rathena_msg_id
            rows.append(row)

        unknown_template, unknown_args = domain["unknown"]
        entry = {
            "key": domain["key"],
            "variant": domain["variant"],
            "server_source": domain["server_source"],
            "unknown": {"template": unknown_template, "args": unknown_args},
            "rows": rows,
        }
        if domain["key"] == "party_member_removed":
            entry["enum"] = withdraw_enum
        domains.append(entry)

    payload = {
        "version": 1,
        "generated_by": "tools/extract_message_facts.py",
        "provenance": (
            "rAthena packet documentation + msgstringtable.txt of the CondaRO client; "
            "current client default texts from korangar@f454127e"
        ),
        "sources": {
            "rathena": {
                "repository": RATHENA_REPOSITORY,
                "revision": RATHENA_COMMIT,
                "files": [
                    "src/map/clif.cpp",
                    "src/map/clif.hpp",
                    "src/map/mmo.hpp",
                    "src/login/loginclif.cpp",
                ],
            },
            "official_client_messages": {
                "archive": "CondaRO.grf",
                "path": "data\\msgstringtable.txt",
                "encoding": "EUC-KR",
                "entries": len(entries),
                "dropped_leading_lines": dropped,
                "note": "classic table, every message terminated by '#'; ids are line indices",
            },
            "official_client_skill_fail_mapping": {
                "repository": ROBROWSER_REPOSITORY,
                "file": ROBROWSER_FILE,
                "note": "reverse-engineered official client onSkillResult cause -> MsgStringTable mapping",
            },
            "client_defaults": {
                "revision": CLIENT_COMMIT,
                "files": [
                    "korangar/src/lib.rs",
                    "korangar-networking/src/packet_versions/version_20220406.rs",
                ],
            },
        },
        "domains": domains,
    }
    return payload


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true", help="fail if the committed facts file is out of date")
    parser.add_argument("--rathena-dir", default=str(DEFAULT_RATHENA_DIR), help="rAthena checkout at the pinned revision")
    parser.add_argument("--msgstringtable", default=str(DEFAULT_MSGSTRINGTABLE), help="client data/msgstringtable.txt")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    payload = build_payload(Path(args.rathena_dir), Path(args.msgstringtable))
    content = json.dumps(payload, indent=2, ensure_ascii=False) + "\n"
    output = Path(args.output)

    if args.check:
        current = output.read_text(encoding="utf-8") if output.is_file() else ""
        if current != content:
            print(f"error: {output} is out of date; run ./tools/extract_message_facts.py", file=sys.stderr)
            return 1
        row_count = sum(len(domain["rows"]) for domain in payload["domains"])
        print(f"{output} is up to date ({row_count} rows)")
        return 0

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(content, encoding="utf-8")
    row_count = sum(len(domain["rows"]) for domain in payload["domains"])
    print(f"{output}: {row_count} rows in {len(payload['domains'])} domains")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
