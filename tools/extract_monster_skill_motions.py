#!/usr/bin/env python3
"""Extract the official monster skill -> ACT action table of the CondaRO client.

The official client does not play the player skill-action table for monsters.
`CGameMode::NotifySkillCasting` / `CGameMode::ReceiveSkillUse` look the skill up
per monster with `CSession::GetMonsterSkillInfo`
(`Source/Contents/Monster/MonsterXmlActions.cpp:43`), which reads
`data\\MonsterSkillInfo.xml`. The table is keyed by the monster's job name
(lower-cased) and the skill id; each row stores an `action` (1-based ACT action
number) and a `loopType` (0 = play once, non-zero = loop). The client then calls
`SetAction(action * 8 - 8, 1, loopType == 0)`
(`Source/Client/Mode/GameModePackets.cpp:12958-12961`, `:18371-18375`).

`MonsterSkillInfo.xml` is stored in `data.grf` with the client's GRF mixcrypt
and its tag/attribute text is obfuscated with `DecodeMessageRot`
(`Source/Contents/Tables/MessageTable.cpp:5040`: ASCII letters are reversed,
`a <-> z`, `A <-> Z`). This script reads the archive entry, de-obfuscates it and
writes the committed facts:

    tools/data/monster_skill_motions.json

The XML has a second, truncated copy of the table after the root close tag; the
official TinyXML parser stops at the root and so does this extractor.

Usage:
    ./tools/extract_monster_skill_motions.py
    ./tools/extract_monster_skill_motions.py --archive data.grf --output tools/data/monster_skill_motions.json

The client archives are read-only. The committed JSON lets
`tools/gen_monster_skill_actions.py --check` run without the 3.2 GB archive.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import xml.etree.ElementTree as ET
from pathlib import Path

import extract_job_facts

DEFAULT_ARCHIVE = Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/data.grf")
DEFAULT_OUTPUT = Path("tools/data/monster_skill_motions.json")
ENTRY = "data\\monsterskillinfo.xml"
ROOT_TAG = "Monster_Action_Table"


def decode_message_rot(text: str) -> str:
    """`DecodeMessageRot` for the Korean service (ASCII letters are reversed)."""
    decoded = []
    for char in text:
        codepoint = ord(char)
        if "a" <= char <= "z":
            char = chr(ord("a") + ord("z") - codepoint)
        elif "A" <= char <= "Z":
            char = chr(ord("A") + ord("Z") - codepoint)
        decoded.append(char)
    return "".join(decoded)


def clean_xml(raw: bytes) -> str:
    """De-obfuscate the XML and cut the truncated duplicate after the root."""
    text = decode_message_rot(raw.decode("euc-kr"))
    start = text.index(f"<{ROOT_TAG}")
    closing = f"</{ROOT_TAG}>"
    end = text.index(closing, start) + len(closing)
    return text[start:end]


def parse_entries(text: str) -> tuple[list[dict], list[dict]]:
    root = ET.fromstring(text)
    if root.tag != ROOT_TAG:
        raise SystemExit(f"unexpected root tag {root.tag!r}")

    entries: dict[tuple[str, int], dict] = {}
    duplicates: list[dict] = []

    for monster in root:
        name = monster.tag.lower()
        for skill in monster:
            if not skill.tag.startswith("_"):
                continue
            skill_id = int(skill.tag[1:])
            action_text = skill.findtext("action")
            loop_text = skill.findtext("loopType")
            if action_text is None or loop_text is None:
                continue
            action = int(action_text)
            loop_type = int(loop_text)
            row = {
                "monster": name,
                "skill": skill_id,
                "action": action,
                # Official SetAction(action * 8 - 8, ...): block 8*(action-1).
                "action_index": action - 1,
                "block": action * 8 - 8,
                "loop_type": loop_type,
                # `loopType == 0` is passed as `repeat`, which is MT_ONETIME;
                # every other value loops (MT_LOOP).
                "looping": loop_type != 0,
            }
            key = (name, skill_id)
            if key in entries:
                # `XmlNode::FindChild` returns the first matching child.
                duplicates.append(row)
                continue
            entries[key] = row

    ordered = sorted(entries.values(), key=lambda row: (row["monster"], row["skill"]))
    return ordered, duplicates


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", default=str(DEFAULT_ARCHIVE))
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    archive_path = Path(args.archive)
    if not archive_path.is_file():
        raise SystemExit(f"client archive not found: {archive_path}")

    archive = extract_job_facts.GrfFileTable(archive_path)
    raw = archive.get(ENTRY)
    archive.close()
    if raw is None:
        raise SystemExit(f"{ENTRY} is not part of {archive_path}")

    entries, duplicates = parse_entries(clean_xml(raw))

    data = {
        "provenance": {
            "source": "CondaRO client data.grf: data\\monsterskillinfo.xml",
            "archive": str(archive_path),
            "archive_size": archive_path.stat().st_size,
            "entry": ENTRY,
            "entry_size": len(raw),
            "entry_sha256": hashlib.sha256(raw).hexdigest(),
            "decoder": "DecodeMessageRot (MessageTable.cpp:5040) + GRF mixcrypt (ro-next/src/loaders/archive/native/mixcrypt.rs)",
            "official_client": (
                "CSession::GetMonsterSkillInfo (MonsterXmlActions.cpp:43); "
                "CGameMode::NotifySkillCasting (GameModePackets.cpp:12958) and "
                "CGameMode::ReceiveSkillUse (GameModePackets.cpp:18371) call "
                "SetAction(action * 8 - 8, 1, loopType == 0)"
            ),
            "note": (
                "`action` is the 1-based ACT action from the XML; `action_index` = action - 1 "
                "(SetAction argument) and `block` = action_index * 8. `loop_type == 0` plays the "
                "action once, any other value loops it. Monster names are the lower-cased job "
                "names; the client looks them up with CSession::GetJobName(...) + _strlwr. The XML "
                "contains a truncated duplicate copy after the root close tag; it is ignored."
            ),
            "duplicates": duplicates,
        },
        "entries": entries,
    }
    output = Path(args.output)
    output.write_text(json.dumps(data, indent=1) + "\n")
    print(f"wrote {output}: {len(entries)} (monster, skill) rows, {len({e['monster'] for e in entries})} monsters, {len(duplicates)} duplicates")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
