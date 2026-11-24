# Gap implementation status

Tracking the gaps from `rodecomp-game-gap-audit.md`. Order = player impact.
Batch 1 (5 agents) and batch 2 (6 agents) are merged; the table reflects `main`.

| # | gap | status | commits |
| --- | --- | --- | --- |
| 1 | Item drop (`CZ_ITEM_THROW`) | **done** | `a8acbc16` |
| 2 | NPC text/number input (`ZC_OPEN_EDITDLG(STR)`) | **done** | `91b00672` |
| 3 | Cart inventory | **done** | `5fa9dd91` |
| 4 | Buy from another player's vending shop | **done** | `5fa9dd91`, `6fec220f` |
| 5 | Guild system (info, members, positions, notice, invite/expel/leave) | **done** | `b78fc9a7` |
| 6 | Rodex mail (list, read, claim, delete) | **done** | `fc030b7f` |
| 7 | Costume / shadow-gear slots | **done** | `af61c058` |
| 8 | Shield and robe rendering | **done** (robe per-motion Lua draw tables; shield is fixed-layer by design) | `af61c058`, `776645f0` |
| 9 | Skill cooldown display | **done** | `1108f88f` |
| 10 | Battle mode `/bm` | **done** | `1108f88f` |
| 11 | Item/Navi/URL links in chat + NPC text | **done** | `507562f4`, `20b0a154` |
| 12 | Item comparison pane | **done** | `1108f88f` |
| 13 | PvP/GvG/WoE targeting rules | **done** (official HandleActorPick rules; cursor-only refusal) | `af61c058`, `776645f0` |
| 14 | Hotbar 2 bars + F1–F12 | **done** (2×9 shown; 2 quick slots unrendered) | `1108f88f` |
| 15 | Key rebinding UI + settings surface | **done** | `1108f88f` |
| 16 | Refine / repair / identify / card insert | **done** | `31f6017c` |
| 17 | Bank (cash shop dropped by user decision) | **done** | `8aae9953` |
| 18 | Whisper windows + chat tabs | **done** (block list has no 2019 packet) | `507562f4` |
| 19 | Head marks (`opt1`/`opt2`) | **done** | `507562f4` |
| 20 | Party chat tab + party settings | **done** | `507562f4`, `3d48d712` |
| 21 | Achievements | **done** (titles need client Lua data) | `20b0a154` |
| 22 | Navigation / path guide | **done** (client-side, no 2019 server packet) | `20b0a154` |
| 23 | Party booking / LFG | **done** (outgoing join request needs PACKETVER ≥ 20191204) | `3d48d712` |
| 24 | Clan | **done** (no 2019 member-list packet) | `3d48d712` |
| 25 | Pet / homunculus / mercenary | **info windows done** (AI/commands out of scope) | `3d48d712` |
| 26 | Monster DB + Sense stats | **done** — drop tables are server-side and are NOT shown by the official client either (verified: `UIMonsterInfoWnd.h`/`AdventureGuideMgr.h` have no drop fields), so no client copy is wanted | `20b0a154` |
| 27 | Notification HUD | **done** | `507562f4` |
| 28 | Sky, lightmaps, scene fog | **done** (fog needs a map with BG_Fog) | `af61c058` |
| 29 | Dynamic cell updates | **done** | `af61c058` |
| 30 | NPC image + book (notice/tip, transport: no 2019 packet) | **done** | `8f738c12` |
| 31 | Chat rooms, emotes picker | **done** | `8f738c12` |
| 32 | Bink movies | **documented gap** (needs a Bink decoder; IVF side-cars work) | `8f738c12` |
| 33 | Replay | recording exists; playback is an official-client feature but optional tooling, not prioritized (user decision) | `8f738c12` |
| 34 | Pincode prompt (page-slot list / server config unassigned) | **done** | `8f738c12` |

## Skill data parity (2019 GRF tables)

`notes/skill-parity-grf2019-audit.md`: every skill covered by the client's own
`SKILL_EFFECT_INFO_LIST` was audited against the GRF tables; sounds and caster
animations now match for all of them (415 skills with a sound, 549 action
entries), and effect rows exist for every skill whose effect ids have extracted
layer data. The remaining effect-id gap (4th-job/expanded classes 981..1231)
has no layer definitions outside `CondaRO.exe` and is documented there.

## Known remaining work
- Bink video decoding (only pre-transcoded `.ivf` side-cars play).
- In-game visual verification of batch 2 features (build + unit tests only).
