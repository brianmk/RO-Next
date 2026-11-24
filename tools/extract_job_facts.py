#!/usr/bin/env python3
"""Extract the committed job/appearance facts for the data-driven job registry.

The client's job/appearance layer (``world/entity/mod.rs`` before the P1
migration) contained one of the largest hand-maintained content tables in the
code base: a 146-arm ``job id -> Korean player sprite stem`` match, four
palette-stem overrides, two 13-entry hair-style lookup arrays and hard-coded
part path templates, plus several job-id ranges (entity type, transcendent,
baby scale). This script produces the two committed fact sources that
``tools/gen_jobs.py`` turns into ``korangar/archive/data/client/jobs.ron``:

* the **server facts**: every job id of the rAthena tree that CondaRO runs
  (``enum e_job`` in ``src/common/mmo.hpp``; this rAthena revision split the old
  ``db/pre-re/job_db.yml`` into ``job_stats/job_exp/job_aspd/job_basepoints``
  and the authoritative id assignment lives in the enum) and the job skill
  trees of ``db/pre-re/skill_tree.yml`` + ``db/re/skill_tree.yml``, and
* the **client facts**: a probe of the actual CondaRO client archives for the
  player body sprites, body palettes, head (hair) sprites, hair palettes and
  headgear sprites that exist, plus the client's ``jobidentity.lub`` /
  ``npcidentity.lub`` job tables (read from the Lua bytecode constants, no Lua
  runtime needed) for the job identity coverage and the special entity job ids
  (warp/hidden NPCs).

The ``job id -> player sprite stem`` mapping itself is **client engine data**
(the classic kRO client hard-codes it in the executable; it is not part of any
data file). The curated table below is that engine table, cross-checked against
``CondaRO.exe`` (the CP949 job-name strings) and, for every single entry,
against the archive probe: a mapping whose stem has no body sprite is only
accepted when an explicit, archive-verified fallback is defined. That keeps the
generator honest and makes the fallbacks auditable instead of invisible.

All sprite/palette probes are compared case-insensitively: the archives store
lower-case file names and the client lowercases asset lookups (the official
Windows client is case-insensitive). That is why the newer 4th/expanded class
sprites (`dragon_knight_남.spr`, …) are found and used instead of a fallback.

The scan is offline and read-only. The committed ``tools/data/job_facts.json`` lets
``tools/gen_jobs.py --check`` run in CI without the 3.2 GB client archives or
the rAthena server tree.

Usage:
    # read the server tree over ssh (the CondaRO rAthena host) and the local
    # CondaRO client archives, then write the committed facts:
    ./tools/extract_job_facts.py --ssh rathena --server-root /home/phantom/rathena

    # or with a local rAthena checkout:
    ./tools/extract_job_facts.py --local /path/to/rathena

    # explicit archives / output:
    ./tools/extract_job_facts.py --archive data.grf --archive CondaRO.grf \\
        --archive palettes.grf --output tools/data/job_facts.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import struct
import subprocess
import sys
import zlib
from pathlib import Path

DEFAULT_SSH = "rathena"
DEFAULT_SERVER_ROOT = "/home/phantom/rathena"
DEFAULT_OUTPUT = Path("tools/data/job_facts.json")

#: Files read from the rAthena tree. The job database of current rAthena is
#: split; ``enum e_job`` is the authoritative numeric job list the server sends
#: on the wire, the skill trees are the "job tree" data.
SERVER_FILES = {
    "job_enum": "src/common/mmo.hpp",
    "skill_tree_pre_re": "db/pre-re/skill_tree.yml",
    "skill_tree_re": "db/re/skill_tree.yml",
    "player_class_check": "src/map/pc.hpp",
}

DEFAULT_ARCHIVES = [
    Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/data.grf"),
    Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/CondaRO.grf"),
    Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/palettes.grf"),
]

#: Client archives of CondaRO that are probed for the appearance assets.
CLIENT_SOURCE = "CondaRO client archives (kRO 2019-06-05 data set)"
CLIENT_BINARY = Path("/home/phantom/Games/ragnarok/CondaRO-linux/client/game/CondaRO.exe")

#: `data\\luafiles514\\lua files\\datainfo\\jobidentity.lub` /
#: `...\\npcidentity.lub` inside the archives. Both are Lua 5.1 bytecode.
JOB_IDENTITY_PATH = "data\\luafiles514\\lua files\\datainfo\\jobidentity.lub"
NPC_IDENTITY_PATH = "data\\luafiles514\\lua files\\datainfo\\npcidentity.lub"

#: Headgear view id -> resource name tables, at the paths korangar's
#: `AccessoryName` table loads them from (32-bit Lua 5.1 bytecode). The probe
#: needs the same table the client uses so a missing headgear sprite is
#: reported for the right view id.
ACCESSORY_ID_PATH = "data\\luafiles514\\lua files\\accessoryid.lub"
ACCESSORY_NAME_PATH = "data\\luafiles514\\lua files\\accname.lub"

#: Job ids that are sentinels, never a real job (`JOB_MAX_BASIC`, `JOB_MAX`).
SENTINEL_JOBS = {31, 4317}

#: Transformed into `JOB_<NAME>`; entries of `enum e_job` added after the last
#: migration of this table need no code change here, the generator picks them up
#: from the facts.
PLAYER_CLASS_RANGES = [(0, 44), (4000, 5999)]
NPC_CLASS_RANGES = [(46, 999), (10000, 19999)]
MONSTER_CLASS_RANGES = [(1000, 3999), (20000, 29999)]

#: Classic client job ids that are rendered specially instead of as a sprite:
#: `JT_WARPNPC` (invisible map connection, drawn as the warp effect),
#: `JT_HIDDEN_NPC` / `JT_HIDDEN_WARP_NPC` (not drawn at all). The ids are
#: extracted from the client `npcidentity.lub`; these are the documented
#: fallbacks if a different data set does not ship that table.
SPECIAL_ENTITY_JOB_IDS = {"warp": [45], "hidden": [111, 139]}

#: rAthena `e_job` blocks of fourth classes, from the explicit enum values in
#: `src/common/mmo.hpp` (4252 basic 4th classes, 4278 alternates, 4302 expanded
#: 4th classes, 4316 the alternate expanded one).
FOURTH_JOB_RANGES = [(4252, 4264), (4278, 4281), (4302, 4308), (4316, 4316)]

#: Other `e_job` blocks of expanded (non-standard) classes: Super Novice,
#: Gunslinger/Ninja, Taekwon/Star Gladiator/Soul Linker, Kagerou/Oboro/Rebellion,
#: Summoner and the Star Emperor/Soul Reaper line including their baby variants.
EXPANDED_JOB_IDS = (
    {23, 24, 25, 4190, 4191}
    | set(range(4046, 4050))
    | {4211, 4212, 4215}
    | set(range(4218, 4245))
    | set(range(4302, 4309))
    | {4316}
)

#: Official rebirth (transcendent) second-job names: `enum e_job` names whose
#: base name is one of these are the rebirth variants. The `_HIGH`, `_T` and
#: `_T2` name suffixes are handled separately.
TRANSCENDENT_SECOND_JOBS = {
    "LORD_KNIGHT",
    "HIGH_WIZARD",
    "SNIPER",
    "HIGH_PRIEST",
    "WHITESMITH",
    "ASSASSIN_CROSS",
    "PALADIN",
    "CHAMPION",
    "PROFESSOR",
    "STALKER",
    "CREATOR",
    "CLOWN",
    "GYPSY",
}

#: Baby body sprites are rendered with this scale (classic client).
BABY_JOB_SCALE = 0.75

#: ---------------------------------------------------------------------------
#: Curated client engine tables (see the module doc for the verification rules).
#: ---------------------------------------------------------------------------
#: Job id -> player body sprite stem of the classic client engine table. The
#: stem is the `data\\sprite\\인간족\\몸통\\{sex}\\<stem>_{sex}.spr` name.
CLIENT_JOB_SPRITES = {
    0: "초보자",
    1: "검사",
    2: "마법사",
    3: "궁수",
    4: "성직자",
    5: "상인",
    6: "도둑",
    7: "기사",
    8: "성투사",
    9: "위저드",
    10: "제철공",
    11: "헌터",
    12: "어세신",
    13: "페코페코_기사",
    14: "크루세이더",
    15: "몽크",
    16: "세이지",
    17: "로그",
    18: "연금술사",
    19: "바드",
    20: "무희",
    23: "슈퍼노비스",
    24: "건너",
    25: "닌자",
    4001: "초보자",
    4002: "검사",
    4003: "마법사",
    4004: "궁수",
    4005: "성직자",
    4006: "상인",
    4007: "도둑",
    4008: "로드나이트",
    4009: "하이프리",
    4010: "하이위저드",
    4011: "화이트스미스",
    4012: "스나이퍼",
    4013: "어쌔신크로스",
    4014: "엔대운",
    4015: "팔라딘",
    4016: "챔피온",
    4017: "프로페서",
    4018: "스토커",
    4019: "크리에이터",
    4020: "클라운",
    4021: "집시",
    4023: "초보자",
    4024: "검사",
    4025: "마법사",
    4026: "궁수",
    4027: "성직자",
    4028: "상인",
    4029: "도둑",
    4030: "기사",
    4031: "성투사",
    4032: "위저드",
    4033: "제철공",
    4034: "헌터",
    4035: "어세신",
    4037: "크루세이더",
    4038: "몽크",
    4039: "세이지",
    4040: "로그",
    4041: "연금술사",
    4042: "바드",
    4043: "무희",
    4045: "슈퍼노비스",
    4046: "태권소년",
    4047: "권성",
    4049: "소울링커",
    4054: "룬나이트",
    4055: "워록",
    4056: "레인져",
    4057: "아크비숍",
    4058: "미케닉",
    4059: "길로틴크로스",
    4060: "룬나이트",
    4061: "워록",
    4062: "레인져",
    4063: "아크비숍",
    4064: "미케닉",
    4065: "길로틴크로스",
    4066: "가드",
    4067: "소서러",
    4068: "민스트럴",
    4069: "원더러",
    4070: "슈라",
    4071: "제네릭",
    4072: "쉐도우체이서",
    4073: "가드",
    4074: "소서러",
    4075: "민스트럴",
    4076: "원더러",
    4077: "슈라",
    4078: "제네릭",
    4079: "쉐도우체이서",
    4096: "룬나이트",
    4097: "워록",
    4098: "레인져",
    4099: "아크비숍",
    4100: "미케닉",
    4101: "길로틴크로스",
    4102: "가드",
    4103: "소서러",
    4104: "민스트럴",
    4105: "원더러",
    4106: "슈라",
    4107: "제네릭",
    4108: "쉐도우체이서",
    4190: "슈퍼노비스",
    4191: "슈퍼노비스",
    4211: "KAGEROU",
    4212: "OBORO",
    4215: "REBELLION",
    4222: "닌자",
    4223: "KAGEROU",
    4224: "OBORO",
    4225: "태권소년",
    4226: "권성",
    4227: "소울링커",
    4228: "건너",
    4229: "REBELLION",
    4239: "성제",
    4240: "소울리퍼",
    4241: "성제",
    4242: "소울리퍼",
    4252: "DRAGON_KNIGHT",
    4253: "MEISTER",
    4254: "SHADOW_CROSS",
    4255: "ARCH_MAGE",
    4256: "CARDINAL",
    4257: "WINDHAWK",
    4258: "IMPERIAL_GUARD",
    4259: "BIOLO",
    4260: "ABYSS_CHASER",
    4261: "ELEMETAL_MASTER",
    4262: "INQUISITOR",
    4263: "TROUBADOUR",
    4264: "TROUVERE",
    4302: "SKY_EMPEROR",
    4303: "SOUL_ASCETIC",
    4304: "SHINKIRO",
    4305: "SHIRANUI",
    4306: "NIGHT_WATCH",
    4307: "HYPER_NOVICE",
}

#: Verified body-sprite fallbacks for jobs whose engine sprite is not part of
#: this (trimmed) data set or that the client does not map at all. Every target
#: is checked against the archive probe by this script; the generator refuses a
#: facts file with a fallback that does not exist.
SPRITE_FALLBACKS = {
    21: "신페코크루세이더",
    22: "결혼",
    26: "산타",
    27: "여름",
    28: "한복",
    29: "옥토버패스트",
    30: "여름2",
    4014: "로드나이트",
    4022: "페코팔라딘",
    4036: "기사",
    4044: "크루세이더",
    4048: "권성",
    4050: "초보자",
    4051: "초보자",
    4052: "초보자",
    4080: "룬나이트",
    4081: "룬나이트",
    4082: "가드",
    4083: "가드",
    4084: "레인져",
    4085: "레인져",
    4086: "미케닉",
    4087: "미케닉",
    4109: "룬나이트",
    4110: "가드",
    4111: "레인져",
    4112: "미케닉",
    4211: "닌자",
    4212: "닌자",
    4215: "건너",
    4218: "초보자",
    4220: "초보자",
    4223: "닌자",
    4224: "닌자",
    4229: "건너",
    4238: "권성",
    4243: "성제",
    4244: "성제",
    4252: "룬나이트",
    4253: "화이트스미스",
    4254: "길로틴크로스",
    4255: "워록",
    4256: "아크비숍",
    4257: "레인져",
    4258: "가드",
    4259: "제네릭",
    4260: "쉐도우체이서",
    4261: "소서러",
    4262: "슈라",
    4263: "민스트럴",
    4264: "원더러",
    4278: "레인져",
    4279: "화이트스미스",
    4280: "룬나이트",
    4281: "가드",
    4302: "성제",
    4303: "소울리퍼",
    4304: "닌자",
    4305: "닌자",
    4306: "건너",
    4307: "슈퍼노비스",
    4308: "소울링커",
    4316: "성제",
}

#: Palette stems that differ from the sprite stem of the same job (classic
#: palette file naming). Jobs that use one of these sprites inherit the palette;
#: the reverse table is what the generator really consumes.
PALETTE_OVERRIDES_BY_JOB = {
    4009: "하이프리스트",
    4013: "어세신크로스",
    4066: "로얄가드",
    4073: "로얄가드",
    4102: "로얄가드",
    4215: "리벨리온",
    4229: "리벨리온",
}

#: Network hair style index -> client head sprite id (classic engine table).
HAIR_STYLES = {
    "male": [2, 2, 1, 7, 5, 4, 3, 6, 8, 9, 10, 12, 11],
    "female": [2, 2, 4, 7, 1, 5, 3, 6, 12, 10, 9, 11, 8],
}

#: Fallback head sprite id when the network hair style id is missing.
DEFAULT_HEAD = 1
#: Default palette color index used when a palette color is missing.
DEFAULT_BODY_PALETTE = 0
DEFAULT_HEAD_PALETTE = 0


class GrfFileTable:
    """Minimal GRF (version 0x200) file-table + plain-zlib reader.

    The CondaRO archives store the Lua data files and the sprite/palette file
    names this script needs as regular zlib streams; encrypted entries are
    rejected loudly instead of silently producing wrong facts.
    """

    def __init__(self, path: Path):
        self.path = path
        self._handle = path.open("rb")
        header = self._handle.read(46)
        if not header[:16].startswith(b"Master of Magic"):
            raise ValueError(f"{path}: not a GRF file")
        table_offset, reserved, file_count = struct.unpack_from("<III", header, 0x1E)
        self.version = struct.unpack_from("<I", header, 0x2A)[0]
        if self.version != 0x200:
            raise ValueError(f"{path}: unsupported GRF version {self.version:#x}")
        self._handle.seek(46 + table_offset)
        compressed_size, _uncompressed_size = struct.unpack("<II", self._handle.read(8))
        table = zlib.decompress(self._handle.read(compressed_size))
        # Version 0x200 stores 7 extra entries (data.ini, ...) that are not
        # part of the actual file list.
        self.entry_count = file_count - reserved - 7
        self.names: list[str] = []
        self._entries: dict[str, tuple[int, int, int, int]] = {}
        position = 0
        for _ in range(self.entry_count):
            end = table.index(b"\x00", position)
            name = table[position:end].decode("cp949", "replace").lower()
            position = end + 1
            compressed, compressed_aligned, uncompressed, flags, offset = struct.unpack_from("<II I B I", table, position)
            position += 17
            self.names.append(name)
            self._entries[name] = (compressed, compressed_aligned, uncompressed, flags, offset)
        self.size = path.stat().st_size

    def sha256(self) -> str:
        digest = hashlib.sha256()
        with self.path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        return digest.hexdigest()

    def get(self, name: str) -> bytes | None:
        """Raw decompressed content of one entry, or ``None``."""
        entry = self._entries.get(name.lower())
        if entry is None:
            return None
        _compressed, compressed_aligned, _uncompressed, flags, offset = entry
        if flags != 0x01:
            raise ValueError(
                f"{self.path.name}: entry {name!r} uses flags {flags:#x} (encryption/compression this "
                "extractor does not implement)"
            )
        self._handle.seek(46 + offset)
        data = self._handle.read(compressed_aligned)
        return zlib.decompress(data)

    def close(self) -> None:
        self._handle.close()


class ArchiveSet:
    """The client archives in load order (later archives override earlier)."""

    def __init__(self, paths: list[Path]):
        self.archives = [GrfFileTable(path) for path in paths]
        self.names: set[str] = set()
        for archive in self.archives:
            self.names |= set(archive.names)

    def get(self, name: str) -> bytes | None:
        for archive in reversed(self.archives):
            try:
                data = archive.get(name)
            except ValueError as error:
                print(f"warning: {error}; trying the next archive", file=sys.stderr)
                continue
            if data is not None:
                return data
        return None

    def close(self) -> None:
        for archive in self.archives:
            archive.close()


def lua_bytecode_constants(data: bytes) -> list[object]:
    """Constants of the top-level Lua 5.1 chunk (the RO `.lub` files).

    The client `.lub` files are 32-bit Lua 5.1 bytecode. Only the constant table
    is needed for the flat `JT_* = id` tables; nested functions are skipped,
    which is enough for `jobtbl`/`JTtbl` (their entries are plain constants).
    """
    if data[:4] != b"\x1bLua":
        raise ValueError("not a Lua bytecode file")
    position = 12

    def u32() -> int:
        nonlocal position
        value = struct.unpack_from("<I", data, position)[0]
        position += 4
        return value

    size = u32()
    position += size
    u32()
    u32()
    position += 4  # int size, size_t size, instruction size, lua_Number size
    count = u32()
    position += 4 * count
    count = u32()
    constants: list[object] = []
    for _ in range(count):
        kind = data[position]
        position += 1
        if kind == 0:
            constants.append(None)
        elif kind == 1:
            constants.append(bool(data[position]))
            position += 1
        elif kind == 3:
            constants.append(struct.unpack_from("<d", data, position)[0])
            position += 8
        elif kind == 4:
            length = u32()
            constants.append(data[position : position + length - 1].decode("utf-8", "replace"))
            position += length
        else:
            raise ValueError(f"unsupported Lua constant type {kind}")
    return constants


def constant_pairs(constants: list[object]) -> list[tuple[str, int]]:
    """Consecutive (string, integral number) constant pairs of a flat table."""
    pairs = []
    for value, number in zip(constants, constants[1:]):
        if isinstance(value, str) and isinstance(number, float) and number.is_integer():
            pairs.append((value, int(number)))
    return pairs


def lua_chunk(data: bytes) -> tuple[list[int], list[object]]:
    """Code + constant table of the top-level Lua 5.1 chunk (32-bit dump)."""
    if data[:4] != b"\x1bLua":
        raise ValueError("not a Lua bytecode file")
    position = 12

    def u32() -> int:
        nonlocal position
        value = struct.unpack_from("<I", data, position)[0]
        position += 4
        return value

    def u8() -> int:
        nonlocal position
        value = data[position]
        position += 1
        return value

    def string() -> bytes | None:
        nonlocal position
        length = u32()
        if length == 0:
            return None
        value = data[position : position + length - 1]
        position += length
        return value

    string()  # source name
    u32()  # line defined
    u32()  # last line defined
    u8()  # upvalues
    u8()  # parameters
    u8()  # vararg
    u8()  # max stack
    code_size = u32()
    code = [u32() for _ in range(code_size)]
    constant_count = u32()
    constants: list[object] = []
    for _ in range(constant_count):
        kind = u8()
        if kind == 0:
            constants.append(None)
        elif kind == 1:
            constants.append(bool(u8()))
        elif kind == 3:
            constants.append(struct.unpack_from("<d", data, position)[0])
            position += 8
        elif kind == 4:
            constants.append(string())
        else:
            raise ValueError(f"unsupported Lua constant type {kind}")
    return code, constants


def lua_table_constructor_pairs(data: bytes, key_global: bytes | None) -> list[tuple[object, object]]:
    """`(key, value)` rows of a flat Lua table constructor.

    Handles both compiler patterns the client files use: constant keys
    (`[K] = V`) and keys built from the global table `ACCESSORY_IDs`
    (`[ACCESSORY_IDs.ACCESSORY_X] = V`, used by `accname.lub`).
    """
    code, constants = lua_chunk(data)
    registers: dict[int, tuple[str, object]] = {}
    pairs: list[tuple[object, object]] = []

    for instruction in code:
        opcode = instruction & 0x3F
        a = (instruction >> 6) & 0xFF
        c = (instruction >> 14) & 0x1FF
        b = (instruction >> 23) & 0x1FF
        bx = (instruction >> 14) & 0x3FFFF

        if opcode == 1:  # LOADK
            registers[a] = ("constant", constants[bx])
        elif opcode == 5:  # GETGLOBAL
            registers[a] = ("global", constants[bx])
        elif opcode == 6:  # GETTABLE
            base = registers.get(b) if not (b & 0x100) else ("constant", constants[b & 0xFF])
            key = ("constant", constants[c & 0xFF]) if (c & 0x100) else registers.get(c)
            if (
                base is not None
                and base[0] == "global"
                and base[1] == key_global
                and key is not None
                and key[0] == "constant"
            ):
                registers[a] = ("key", key[1])
            else:
                registers[a] = ("table", None)
        elif opcode == 9:  # SETTABLE
            key = registers.get(b) if not (b & 0x100) else ("constant", constants[b & 0xFF])
            value = registers.get(c) if not (c & 0x100) else ("constant", constants[c & 0xFF])
            if key is None or value is None:
                continue
            if key[0] == "key" and value[0] == "constant":
                pairs.append((key[1], value[1]))
            elif key[0] == "constant" and value[0] == "constant":
                pairs.append((key[1], value[1]))
    return pairs


def decode_lua_name(raw: bytes) -> str:
    """Decode a client Lua string (EUC-KR or latin-1-mojibake) to text."""
    candidates: list[str] = []
    try:
        candidates.append(raw.decode("euc-kr"))
    except UnicodeDecodeError:
        pass
    try:
        mojibake = raw.decode("utf-8")
        if all(ord(char) < 256 for char in mojibake):
            candidates.append(bytes(ord(char) for char in mojibake).decode("euc-kr"))
    except (UnicodeDecodeError, UnicodeEncodeError):
        pass
    for candidate in candidates:
        if candidate.startswith("_"):
            return candidate
    return raw.decode("latin-1")


def clean_identity_name(key: str) -> str:
    """Mirror korangar's `JobIdentity` cleaning of the `JT_*` table keys."""
    if key.startswith("JT_G_"):
        return key[5:]
    if key.startswith(("JT_C1_", "JT_C2_", "JT_C3_", "JT_C4_", "JT_C5_")):
        return key[6:]
    if key.startswith("JT_"):
        return key[3:]
    return key


def lua_table_rows(data: bytes) -> list[tuple[str, int]]:
    """`(key, id)` rows of a flat `JT_* = id` Lua table from source or bytecode.

    The client ships these tables either as plain Lua source (`npcidentity.lub`)
    or as Lua 5.1 bytecode (`jobidentity.lub`); both forms are handled.
    """
    if data[:4] == b"\x1bLua":
        return constant_pairs(lua_bytecode_constants(data))

    text = data.decode("utf-8", "replace")
    rows: list[tuple[str, int]] = []
    for match in re.finditer(r"(?:JT_[A-Z0-9_]+)\s*=\s*(\d+)", text):
        rows.append((match.group(0).split("=")[0].strip(), int(match.group(1))))
    return rows


def parse_identity_tables(archives: ArchiveSet) -> tuple[dict[int, str], dict[int, str]]:
    """Return (identity by job id, job constant name by job id) from the client tables."""
    identity: dict[int, str] = {}
    names: dict[int, str] = {}

    npc_identity = archives.get(NPC_IDENTITY_PATH)
    if npc_identity is None:
        raise ValueError(f"{NPC_IDENTITY_PATH} is not part of the client archives")
    # The rows of `jobtbl = { JT_NAME = id, ... }` (plain source or constants).
    for name, job_id in lua_table_rows(npc_identity):
        if not name.startswith(("JT_", "jobtbl.")):
            continue
        key = name.split(".")[-1]
        names[job_id] = key
    if not names:
        raise ValueError(f"could not parse any job table row from {NPC_IDENTITY_PATH}")

    job_identity = archives.get(JOB_IDENTITY_PATH)
    if job_identity is not None:
        for name, job_id in lua_table_rows(job_identity):
            if not name.startswith(("JT_", "jobtbl.")):
                continue
            identity[job_id] = clean_identity_name(name.split(".")[-1])

    return identity, names


def parse_accessory_names(archives: ArchiveSet) -> dict[int, str]:
    """Headgear view id -> sprite resource name from the client data tables."""
    id_data = archives.get(ACCESSORY_ID_PATH)
    name_data = archives.get(ACCESSORY_NAME_PATH)
    if id_data is None or name_data is None:
        return {}

    ids: dict[str, int] = {}
    for key, value in lua_table_constructor_pairs(id_data, None):
        if isinstance(key, bytes) and isinstance(value, float) and value.is_integer():
            ids[key.decode("latin-1")] = int(value)

    names: dict[int, str] = {}
    for key, value in lua_table_constructor_pairs(name_data, b"ACCESSORY_IDs"):
        if not isinstance(key, bytes) or not isinstance(value, bytes):
            continue
        key_name = key.decode("latin-1")
        if key_name not in ids:
            continue
        name = decode_lua_name(value)
        if name:
            names[ids[key_name]] = name
    return names


def fetch_server_file(ssh_host: str | None, server_root: str, local: str | None, relative_path: str) -> str:
    path = f"{server_root}/{relative_path}" if local is None else f"{local}/{relative_path}"
    if ssh_host is None:
        return Path(path).read_text(encoding="utf-8", errors="replace")
    result = subprocess.run(["ssh", ssh_host, "cat", path], capture_output=True, text=True, check=True)
    return result.stdout


def fetch_server_head(ssh_host: str | None, server_root: str, local: str | None) -> str:
    root = local if local is not None else server_root
    command = ["git", "-C", root, "rev-parse", "HEAD"]
    if ssh_host is not None:
        command = ["ssh", ssh_host] + command
    try:
        result = subprocess.run(command, capture_output=True, text=True, check=True)
        return result.stdout.strip()
    except subprocess.CalledProcessError:
        return ""


def parse_job_enum(source: str) -> list[tuple[int, str]]:
    """Parse `enum e_job` from rAthena's `src/common/mmo.hpp`."""
    match = re.search(r"enum e_job\s*\{(.*?)\n\};", source, re.S)
    if match is None:
        raise ValueError("could not find `enum e_job` in src/common/mmo.hpp")
    jobs: list[tuple[int, str]] = []
    next_value = 0
    for raw_line in match.group(1).splitlines():
        line = raw_line.split("//")[0].strip()
        if not line:
            continue
        line = line.rstrip(",")
        entry = re.match(r"^(JOB_[A-Z0-9_]+)\s*(?:=\s*(\d+))?$", line)
        if entry is None:
            continue
        if entry.group(2) is not None:
            next_value = int(entry.group(2))
        jobs.append((next_value, entry.group(1)))
        next_value += 1
    if not jobs:
        raise ValueError("`enum e_job` did not contain any JOB_* values")
    assert jobs[0] == (0, "JOB_NOVICE")
    return jobs


def yaml_job_key(job_name: str) -> str:
    """`JOB_RUNE_KNIGHT_T` -> `Rune_Knight_T` (the skill-tree yml key)."""
    name = job_name.removeprefix("JOB_")
    return "_".join(part.capitalize() for part in name.split("_"))


def parse_skill_trees(yaml_text: str) -> dict[str, dict]:
    """Job skill trees of an rAthena `skill_tree.yml`."""
    import yaml

    document = yaml.safe_load(yaml_text)
    rows = document.get("Body") or []
    trees: dict[str, dict] = {}
    for row in rows:
        job = row.get("Job")
        if not job:
            continue
        tree = row.get("Tree") or []
        inherit = row.get("Inherit") or {}
        trees[job] = {
            "own_skills": len(tree),
            "inherits": sorted(name for name, enabled in inherit.items() if enabled),
        }
    return trees


def parse_client_assets(archives: ArchiveSet) -> dict:
    """Probe the client archives for every appearance asset this system uses."""
    names = archives.names
    # `body_sprites[sex]` is the set of `<stem>_<sex>` names with both a `.spr`
    # and a `.act`; `body_stems` strips the sex suffix again.
    body_sprites: dict[str, set[str]] = {}
    head_sprites: dict[str, set[str]] = {}
    for sex in ("남", "여"):
        body_prefix = f"data\\sprite\\인간족\\몸통\\{sex}\\"
        body_sprites[sex] = complete_stems(names, body_prefix)
        head_prefix = f"data\\sprite\\인간족\\머리통\\{sex}\\"
        head_sprites[sex] = complete_stems(names, head_prefix)

    # Only the *existence* of the palette stems/files is a fact the generator
    # needs; the full color list is not (the archive audit checks the real
    # files). Keeping the committed facts small avoids a multi-MB JSON.
    body_palettes: dict[str, list[str]] = {}
    for sex in ("남", "여"):
        pattern = re.compile(rf"^data\\palette\\몸\\(.+?)_{sex}_(\d+)\.pal$")
        stems = set()
        for name in names:
            match = pattern.match(name)
            if match:
                stems.add(match.group(1))
        body_palettes[sex] = sorted(stems)

    head_palettes: dict[str, list[int]] = {}
    pattern = re.compile(r"^data\\palette\\머리\\머리(\d+)_(남|여)_(\d+)\.pal$")
    for name in names:
        match = pattern.match(name)
        if match is None:
            continue
        hair = int(match.group(1))
        if hair > 30:
            # Beyond this are the dye palette files (`머리<id>_<sex>_<dye>.pal`
            # where <id> happens to be a dye id); only the small hair ids are
            # real hair styles.
            continue
        head_palettes.setdefault(match.group(2), set()).add(hair)
    head_palettes = {sex: sorted(hair_ids) for sex, hair_ids in head_palettes.items()}

    head_ids = {}
    for sex in ("남", "여"):
        suffix = f"_{sex}"
        head_ids[sex] = sorted(int(stem[: -len(suffix)]) for stem in head_sprites[sex] if stem[: -len(suffix)].isdigit())

    headgear_sprites = {sex: complete_stems(names, f"data\\sprite\\악세사리\\{sex}\\{sex}") for sex in ("남", "여")}
    accessory_names = parse_accessory_names(archives)
    headgear_missing = []
    for view_id, name in sorted(accessory_names.items()):
        stem = name.lower()
        if stem not in headgear_sprites["남"] and stem not in headgear_sprites["여"]:
            headgear_missing.append(view_id)
    headgear = {
        "view_ids": len(accessory_names),
        "with_sprite": len(accessory_names) - len(headgear_missing),
        "missing": headgear_missing,
        "sprites": {sex: len(stems) for sex, stems in headgear_sprites.items()},
    }

    return {
        "body_sprites": {sex: sorted(stems) for sex, stems in body_sprites.items()},
        "head_sprites": head_ids,
        "body_palettes": body_palettes,
        "head_palettes": head_palettes,
        "headgear": headgear,
    }


def complete_stems(names: set[str], prefix: str) -> set[str]:
    sprites = {name[len(prefix) : -4] for name in names if name.startswith(prefix) and name.endswith(".spr")}
    actions = {name[len(prefix) : -4] for name in names if name.startswith(prefix) and name.endswith(".act")}
    return sprites & actions


def body_stem_set(client: dict, sex: str) -> set[str]:
    suffix = f"_{sex}"
    return {stem[: -len(suffix)] for stem in client["body_sprites"][sex] if stem.endswith(suffix)}


def in_ranges(job_id: int, ranges: list[tuple[int, int]]) -> bool:
    return any(start <= job_id <= end for start, end in ranges)


def classify_job(job_id: int, job_name: str, trees_by_key: dict[str, dict]) -> dict:
    """Server-side classification of one `enum e_job` entry."""
    name = job_name.removeprefix("JOB_")
    baby = "BABY" in name
    base_name = name[: -1] if name.endswith("2") else name
    transcendent = not baby and (
        base_name in TRANSCENDENT_SECOND_JOBS
        or name.endswith("_HIGH")
        or name.endswith("_T")
        or name.endswith("_T2")
    )
    fourth = in_ranges(job_id, FOURTH_JOB_RANGES)
    expanded = job_id in EXPANDED_JOB_IDS
    key = yaml_job_key(job_name)
    pre_re_tree = trees_by_key.get(("pre_re", key))
    re_tree = trees_by_key.get(("re", key))
    return {
        "baby": baby,
        "transcendent": transcendent,
        "fourth": fourth,
        "expanded": expanded,
        "scale": BABY_JOB_SCALE if baby else 1.0,
        "pre_re_skill_tree": pre_re_tree,
        "re_skill_tree": re_tree,
    }


def build_facts(server: dict, client: dict, client_binary_strings: bytes | None) -> dict:
    jobs = server["jobs"]
    job_ids = {job["id"] for job in jobs}
    real_jobs = [job for job in jobs if job["id"] not in SENTINEL_JOBS]
    body_male = body_stem_set(client, "남")
    body_female = body_stem_set(client, "여")

    problems: list[str] = []

    # The engine lowercases sprite lookups (the archives store lower-case names
    # and the official Windows client is case-insensitive), so the probe has to
    # compare case-insensitively as well. The classic client table spells the
    # newer (4th/expanded) class sprites in upper case (`DRAGON_KNIGHT`), which
    # the archives serve as `dragon_knight`.
    def body_exists(stem: str) -> bool:
        stem = stem.lower()
        return stem in body_male or stem in body_female

    # --- curated engine table sanity -------------------------------------
    for job_id in CLIENT_JOB_SPRITES:
        if job_id not in job_ids:
            problems.append(f"curated client job sprite has unknown job id {job_id}")
    for job_id, stem in SPRITE_FALLBACKS.items():
        if job_id not in job_ids:
            problems.append(f"sprite fallback has unknown job id {job_id}")
        if not body_exists(stem):
            problems.append(f"sprite fallback target {stem!r} of job {job_id} has no body sprite")
    for job_id in real_jobs:
        intended = CLIENT_JOB_SPRITES.get(job_id["id"])
        if intended is not None and body_exists(intended):
            continue
        if job_id["id"] not in SPRITE_FALLBACKS:
            problems.append(f"job {job_id['id']} {job_id['name']} has no existing sprite and no fallback")

    # --- palette overrides resolve to palette stems ----------------------
    client_palettes_male = set(client["body_palettes"]["남"])
    client_palettes_female = set(client["body_palettes"]["여"])

    def palette_exists(stem: str) -> bool:
        stem = stem.lower()
        return stem in client_palettes_male or stem in client_palettes_female

    # Build sprite-stem -> palette-stem from the override table. The override is
    # keyed by job id; the sprite stem of that job is the source of the mapping.
    palette_overrides: dict[str, str] = {}
    for job_id, palette_stem in PALETTE_OVERRIDES_BY_JOB.items():
        sprite_stem = CLIENT_JOB_SPRITES.get(job_id)
        if sprite_stem is None:
            problems.append(f"palette override for job {job_id} has no client sprite mapping")
            continue
        if not palette_exists(palette_stem):
            problems.append(f"palette override target {palette_stem!r} of job {job_id} has no palette file")
        if palette_stem != sprite_stem:
            palette_overrides[sprite_stem] = palette_stem

    # --- hair styles ------------------------------------------------------
    for sex, key in (("남", "male"), ("여", "female")):
        available = set(client["head_sprites"][sex])
        for style, head_id in enumerate(HAIR_STYLES[key]):
            if head_id not in available:
                problems.append(f"{key} hair style {style} maps to missing head sprite {head_id}")
        if not client["head_palettes"].get(sex):
            problems.append(f"no {key} hair palettes in the archives")

    # --- headgear ----------------------------------------------------------
    headgear = client["headgear"]
    if headgear["view_ids"] and headgear["with_sprite"] + len(headgear["missing"]) != headgear["view_ids"]:
        problems.append("headgear view id count does not match the sprite/missing split")
    if headgear["missing"] != sorted(set(headgear["missing"])):
        problems.append("headgear missing view ids are not sorted and unique")

    # --- head range and special entity ids ---------------------------------
    both_sexes = set(client["head_sprites"]["남"]) & set(client["head_sprites"]["여"])
    if not both_sexes:
        problems.append("no head sprites exist for both sexes")
        head_range = [0, 0]
    else:
        head_range = [min(both_sexes), max(both_sexes)]

    identity = client["job_identity"]
    special_ids = {
        "warp": [job_id for job_id, name in client["job_names"].items() if name == "JT_WARPNPC"],
        "hidden": [
            job_id
            for job_id, name in client["job_names"].items()
            if name in ("JT_HIDDEN_NPC", "JT_HIDDEN_WARP_NPC")
        ],
    }
    for key, ids in special_ids.items():
        if ids != SPECIAL_ENTITY_JOB_IDS[key]:
            problems.append(
                f"client job table special ids for {key} are {ids}, expected {SPECIAL_ENTITY_JOB_IDS[key]}"
            )
        special_ids[key] = sorted(ids)

    if problems:
        raise SystemExit("facts are inconsistent:\n  " + "\n  ".join(problems))

    # --- per job facts ------------------------------------------------------
    rows = []
    for job in jobs:
        job_id = job["id"]
        if job_id in SENTINEL_JOBS:
            continue
        intended = CLIENT_JOB_SPRITES.get(job_id)
        if intended is not None and body_exists(intended):
            resolved = intended
            source = "Mapped"
        elif job_id in SPRITE_FALLBACKS:
            resolved = SPRITE_FALLBACKS[job_id]
            source = "Fallback" if intended is not None else "Unmapped"
        else:
            problems.append(f"job {job_id} {job['name']} has no resolved sprite")
            continue

        palette_stem = palette_overrides.get(resolved, resolved)
        palette = palette_stem if palette_exists(palette_stem) else None
        palette_embedded = palette is None

        rows.append(
            {
                "id": job_id,
                "name": job["name"],
                "job_key": yaml_job_key(job["name"]),
                "client_sprite": intended,
                "sprite": resolved,
                "sprite_source": source,
                "palette": palette,
                "palette_embedded": palette_embedded,
                "entity_type": "Player",
                "baby": job["baby"],
                "transcendent": job["transcendent"],
                "fourth": job["fourth"],
                "expanded": job["expanded"],
                "scale": job["scale"],
                "identity": identity.get(job_id),
            }
        )

    if problems:
        raise SystemExit("facts are inconsistent:\n  " + "\n  ".join(problems))

    rows.sort(key=lambda row: row["id"])
    if len({row["id"] for row in rows}) != len(rows):
        raise SystemExit("duplicate job ids in the job facts")

    # --- hair style / appearance defaults ---------------------------------
    hair_style_range = [0, len(HAIR_STYLES["male"]) - 1]
    if len(HAIR_STYLES["male"]) != len(HAIR_STYLES["female"]):
        raise SystemExit("male and female hair style tables have different lengths")

    summary = {
        "jobs": len(rows),
        "mapped": sum(1 for row in rows if row["sprite_source"] == "Mapped"),
        "fallback": sum(1 for row in rows if row["sprite_source"] == "Fallback"),
        "unmapped": sum(1 for row in rows if row["sprite_source"] == "Unmapped"),
        "embedded_palettes": sum(1 for row in rows if row["palette_embedded"]),
        "baby": sum(1 for row in rows if row["baby"]),
        "transcendent": sum(1 for row in rows if row["transcendent"]),
        "fourth": sum(1 for row in rows if row["fourth"]),
        "expanded": sum(1 for row in rows if row["expanded"]),
        "gender_specific": sum(
            1
            for row in rows
            if not (row["sprite"] in body_male and row["sprite"] in body_female)
        ),
        "head_sprites": {"male": len(client["head_sprites"]["남"]), "female": len(client["head_sprites"]["여"])},
        "headgear": {
            "view_ids": headgear["view_ids"],
            "with_sprite": headgear["with_sprite"],
            "missing": len(headgear["missing"]),
        },
    }

    binary_provenance = None
    if client_binary_strings is not None:
        binary_provenance = {
            "sha256": hashlib.sha256(client_binary_strings).hexdigest(),
            "curated_stems_present": sum(
                1 for stem in CLIENT_JOB_SPRITES.values() if stem.encode("cp949", "replace") in client_binary_strings
            ),
            "curated_stems": len(CLIENT_JOB_SPRITES),
            "note": (
                "the CP949 job-name strings of the classic client table; stems of classes newer than this "
                "executable are absent from it (verified against the archive probe instead)"
            ),
        }

    return {
        "provenance": {
            "server": server["provenance"],
            "client": client["provenance"],
            "client_binary": binary_provenance,
            "extractor": "tools/extract_job_facts.py",
            "client_job_sprite_table": (
                "classic kRO client engine job->player-sprite table (hard-coded in the client executable); "
                "curated, every stem verified against the archive probe, missing stems covered by archive-verified fallbacks"
            ),
            "notes": (
                "rAthena split its job database; the authoritative job id assignment is `enum e_job` in "
                "src/common/mmo.hpp and the job trees are db/pre-re/skill_tree.yml + db/re/skill_tree.yml"
            ),
        },
        "server": {
            "jobs": rows,
            "skill_tree_pre_re": server["skill_tree_pre_re"],
            "skill_tree_re": server["skill_tree_re"],
            "player_class_ranges": PLAYER_CLASS_RANGES,
            "npc_class_ranges": NPC_CLASS_RANGES,
            "monster_class_ranges": MONSTER_CLASS_RANGES,
            "special_entity_job_ids": special_ids,
        },
        "client": {
            "job_identity": {str(job_id): name for job_id, name in sorted(identity.items()) if job_id in job_ids},
            "client_job_sprites": {str(job_id): stem for job_id, stem in sorted(CLIENT_JOB_SPRITES.items())},
            "sprite_fallbacks": {str(job_id): stem for job_id, stem in sorted(SPRITE_FALLBACKS.items())},
            "palette_overrides": dict(sorted(palette_overrides.items())),
            "body_sprites": client["body_sprites"],
            "body_palettes": client["body_palettes"],
            "head_sprites": client["head_sprites"],
            "head_palettes": client["head_palettes"],
            "headgear": client["headgear"],
            "appearance": {
                "hair_styles": HAIR_STYLES,
                "hair_style_range": hair_style_range,
                "head_range": head_range,
                "default_head": DEFAULT_HEAD,
                "default_body_palette": DEFAULT_BODY_PALETTE,
                "default_head_palette": DEFAULT_HEAD_PALETTE,
            },
            "summary": summary,
        },
    }


def collect_server(ssh_host: str | None, server_root: str, local: str | None) -> dict:
    sources = {key: fetch_server_file(ssh_host, server_root, local, path) for key, path in SERVER_FILES.items()}
    provenance_files = {}
    for key, path in SERVER_FILES.items():
        provenance_files[path] = {
            "sha256": hashlib.sha256(sources[key].encode("utf-8")).hexdigest(),
            "bytes": len(sources[key].encode("utf-8")),
        }

    server_head = fetch_server_head(ssh_host, server_root, local)
    jobs = parse_job_enum(sources["job_enum"])
    pre_re_trees = parse_skill_trees(sources["skill_tree_pre_re"])
    re_trees = parse_skill_trees(sources["skill_tree_re"])
    trees_by_key = {}
    for key, value in pre_re_trees.items():
        trees_by_key[("pre_re", key)] = value
    for key, value in re_trees.items():
        trees_by_key[("re", key)] = value

    rows = []
    for job_id, job_name in jobs:
        info = classify_job(job_id, job_name, trees_by_key)
        if job_id in SENTINEL_JOBS:
            continue
        if not in_ranges(job_id, PLAYER_CLASS_RANGES):
            raise SystemExit(
                f"job {job_id} {job_name} is outside the documented player job ranges "
                f"{PLAYER_CLASS_RANGES}; update the classification ranges"
            )
        rows.append({"id": job_id, "name": job_name, **info})

    return {
        "provenance": {
            "source": "rAthena CondaRO pre-renewal server tree",
            "head": server_head,
            "files": provenance_files,
        },
        "jobs": rows,
        "skill_tree_pre_re": pre_re_trees,
        "skill_tree_re": re_trees,
    }


def collect_client(archives: ArchiveSet) -> dict:
    identity, job_names = parse_identity_tables(archives)
    assets = parse_client_assets(archives)
    archive_info = [
        {"name": archive.path.name, "size": archive.size, "sha256": archive.sha256(), "files": len(archive.names)}
        for archive in archives.archives
    ]
    client = {
        "provenance": {
            "source": CLIENT_SOURCE,
            "archives": archive_info,
            "job_identity_table": JOB_IDENTITY_PATH,
            "npc_identity_table": NPC_IDENTITY_PATH,
        },
        "job_identity": identity,
        "job_names": job_names,
        **assets,
    }
    return client


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--ssh", default=DEFAULT_SSH, help=f"ssh host with the rAthena tree (default: {DEFAULT_SSH})")
    parser.add_argument("--server-root", default=DEFAULT_SERVER_ROOT, help=f"rAthena tree on the ssh host (default: {DEFAULT_SERVER_ROOT})")
    parser.add_argument("--local", default=None, help="use a local rAthena tree instead of ssh")
    parser.add_argument("--archive", action="append", default=None, help="client GRF to probe (repeatable); defaults to the CondaRO archives")
    parser.add_argument("--client-binary", default=str(CLIENT_BINARY), help="client executable used as a cross-check for the curated sprite names")
    parser.add_argument("--output", default=str(DEFAULT_OUTPUT))
    args = parser.parse_args()

    server = collect_server(args.ssh, args.server_root, args.local)
    archives = ArchiveSet([Path(path) for path in (args.archive or DEFAULT_ARCHIVES)])
    try:
        client = collect_client(archives)
    finally:
        archives.close()

    client_binary_strings = None
    client_binary = Path(args.client_binary)
    if client_binary.is_file():
        client_binary_strings = client_binary.read_bytes()

    facts = build_facts(server, client, client_binary_strings)

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(facts, indent=2, sort_keys=True) + "\n")

    summary = facts["client"]["summary"]
    print(
        f"{output}: {summary['jobs']} jobs "
        f"({summary['mapped']} mapped / {summary['fallback']} fallback / {summary['unmapped']} unmapped), "
        f"{summary['embedded_palettes']} embedded palettes, "
        f"headgear {summary['headgear']['with_sprite']}/{summary['headgear']['view_ids']} with sprites "
        f"({summary['headgear']['missing']} documented missing)"
    )
    print(f"server head {facts['provenance']['server']['head'] or 'unknown'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
