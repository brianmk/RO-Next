# Skill parity audit against the 2019 client tables (GRF)

Date: 2026-11-24. Sources: `data.grf` + `CondaRO.grf` (CondaRO Windows client),
the decompilation `rodecomp` and the existing fact tables in `tools/`.

## Where the official tables live

The 2019 client reads Lua 5.1 tables from the `LuaFiles514` root (case-insensitive
on Windows, lower-cased in the GRFs):

```
data\luafiles514\lua files\skilleffectinfo\skilleffectinfolist.lub   SKILL_EFFECT_INFO_LIST
data\luafiles514\lua files\skilleffectinfo\effectid.lub              EFID
data\luafiles514\lua files\skilleffectinfo\actorstate.lub            ACTOR_STATE
data\luafiles514\lua files\skillinfoz\skillid.lub                    SKID
```

`data\lua files\...` (also present in `CondaRO.grf`) is the legacy pre-2012
path: identical copies for `skilleffectinfolist`/`effectid`/`actorstate` and an
older, smaller `skillid.lub`. Both GRFs' `luafiles514` copies are byte-identical
for the effect tables.

`SKILL_EFFECT_INFO_LIST` covers **66 skills** (61 with effect ids); the classic
skills' effect ids are hard-coded in `CondaRO.exe` and were already extracted
into `tools/rodecomp_effect_coverage.json`.

## What the audit found

`tools/audit_skill_parity.py` compares the GRF table (effect ids, wave files,
begin motion) with `skill_effects.ron` / `skill_sounds.ron` /
`skill_actions.ron`:

| check | before | after |
| --- | --- | --- |
| skills in the GRF table | 66 | 66 |
| no client descriptor | 62 | 43 (their 43 effect ids have no layer definitions in any extracted table) |
| missing official sound | 37 | **0** (wave files merged into `skill_sounds.ron`) |
| missing official effect asset | 19 | **0** (templates like `windhit%d` handled) |
| caster action missing/mismatch | 19 | **0** (`beginMotionType` merged into `skill_actions.ron`) |

Mismatches were fixed by feeding the GRF facts into the existing generators:

* `tools/extract_grf2019_skill_facts.py` → `tools/grf2019_skill_facts.json`
  (flat dump: `tools/client_skill_tables_2019.txt`),
* `gen_skill_sounds.py --grf2019`: 415 skills with a sound (was 397),
* `gen_skill_actions.py --grf2019`: 549 entries (`beginMotionType` fill; e.g.
  `KO_JYUMONJIKIRI` → `Action(11)`, ninja skills → `Skill`),
* `gen_skill_effect_layers.py --grf2019`: 363 official rows / 537 layers (the
  16 skills whose effect ids have complete layer data),
* `gen_skill_effects.py --overrides`: new `tools/skill_effects_overrides.ron`
  replaces whole phases where decompiled client code supersedes the table
  (Asura Strike cast → `CRagEffect::AsuraStrike`).

## Korean file names

GRF file names are stored as **EUC-KR (CP949) bytes**, not UTF-8:

* the client decodes them with `encoding_rs::EUC_KR` (`ragnarok-bytes/src/reader.rs:1,49,82`;
  `FileTableRow.file_name` is a decoded `String`), and the file table keys are
  lower-cased (`archive/native/mod.rs:71`);
* the Python GRF reader decodes with `cp949` (`GrfFileTable` in
  `extract_weapon_client_assets.py`), so the generated RON paths are UTF-8
  Hangul;
* the Lua table dump stores every non-UTF-8 string as `hex:<bytes>` and
  `extract_grf2019_skill_facts.py` decodes it as CP949 — that is how
  `effect\mon_아수라 패황권.wav`, `effect\270바람.tga` and `이팩트\침투경_`
  keep their real names.

All 614 asset references of the client data resolve against the archives
(`tools/audit_asset_case.py`), including the Korean-named textures, sprites and
sounds.

## Case sensitivity

The official client is case-insensitive; Linux is not, but both cache layers
handle it:

* GRFs: `GameFileLoader` lower-cases every lookup and the GRF name table is
  lower-case.
* Loose `archive/` folder: `FolderArchive` builds a lower-cased path → real
  path mapping, so `data/font/NotoSans.ttf` still resolves.

`tools/audit_asset_case.py` checks every `file`/`files`/`frames`/`wav` path of
the client data against both GRFs (611 assets): **0 unresolved**.

## Remaining gaps

43 effect ids (981-1011, 1066-1078, 1133-1150, 1197-1231, 4th-job / expanded
classes plus a few NPC skills) have no layer definitions in any extracted
table; their visuals live only in `CondaRO.exe`. Those skills currently get
their official **sound** and **action** but no effect rows. Extracting the remaining effect implementations
from the decompilation (`CRagEffect` switches) is the next step if those jobs
are ever enabled on the server.
