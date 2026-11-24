# Gap implementation status

Tracking the gaps from `rodecomp-game-gap-audit.md`. Order = player impact.

Done in this batch (5 parallel agents + lead): 1, 2, 3, 4, 5, 6, 9, 10, 11, 12, 14, 15, 18, 19, 27.

| # | gap | status | commit |
| --- | --- | --- | --- |
| 1 | Item drop (`CZ_ITEM_THROW`) | **done** | `a8acbc16` |
| 2 | NPC text/number input (`ZC_OPEN_EDITDLG(STR)`) | **done** | `91b00672` |
| 3 | Cart inventory (load/unload pushcart) | **done** | `5fa9dd91` |
| 4 | Buy from another player's vending shop | **done** | `5fa9dd91`, `6fec220f` |
| 5 | Guild system (window, members, positions, notice, actions) | **done** | `b78fc9a7` |
| 6 | Rodex / mail (list, read, claim, delete) | **done** | `fc030b7f` |
| 7 | Costume / shadow-gear slots | todo | — |
| 8 | Shield and robe rendering | todo | — |
| 9 | Skill cooldown display | **done** | `1108f88f` |
| 10 | Battle mode `/bm` | **done** | `1108f88f` |
| 11 | Item/Navi/URL links in chat + NPC text | **done** | `507562f4` |
| 12 | Item comparison pane | **done** | `1108f88f` |
| 13 | PvP/GvG/WoE targeting rules (`map_flags`) | todo | — |
| 14 | Hotbar 2×38 + F1–F12 | **done (2×9 UI, all 4 bars stored)** | `1108f88f` |
| 15 | Key rebinding UI + settings surface | **done** | `1108f88f` |
| 16 | Refine / repair / identify / card composition | todo | — |
| 17 | Bank + cash shop | todo | — |
| 18 | Whisper windows + chat tabs | **done** | `507562f4` |
| 19 | Head marks (`opt1`/`opt2`) | **done** | `507562f4` |
| 20 | Party chat tab + party settings | **done (chat tab; settings todo)** | `507562f4` |
| 21 | Achievements | todo | — |
| 22 | Navigation / path guide | todo | — |
| 23 | Party booking | todo | — |
| 24 | Clan | todo | — |
| 25 | Pet / homunculus / mercenary | todo | — |
| 26 | Monster DB + drop tables | todo | — |
| 27 | Notification HUD (announce/pickup/level-up/quest banner) | **done** | `507562f4` |
| 28 | Sky, lightmaps, scene fog | todo | — |
| 29 | Map cell updates / map properties | todo | — |
| 30 | NPC image / book / notice / transport windows | todo | — |
| 31 | Chat rooms, emotes picker | todo | — |
| 32 | Bink movies | todo | — |
| 33 | Replay | todo | — |
| 34 | Pincode / page-slot list / server config | todo | — |
