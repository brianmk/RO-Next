# Game-system gap audit: official client vs RO-Next vs rustnarok

Date: 2026-11-23. Scope: everything **except** the skill-effect / animation /
status-aura pipeline (covered by `rodecomp-skill-status-aura-audit.md` and
`rodecomp-skill-drift-resolution.md`).

## Sources

| source | what it is | size |
| --- | --- | --- |
| `rodecomp` @ `bb34313a` | matching decompilation of Ragexe 2026-02-19 | 348 `CGameMode::Receive*` handlers; 42 `Source/UI` subsystems; 33 `Source/Contents` subsystems |
| `korangar` (RO-Next) | our client, integration branch | 49 `interface/windows`; ~5.1k lines `korangar-networking` (143 `NetworkEvent` variants); ~11k lines `ragnarok-packets` |
| `rustnarok-client` @ `20a946ae` | 4th-job/2025 Rust client | 221 source files; `src/fx`, `src/ui`, `src/net`, MCP/autohunt |

The server is a 2019 rAthena Pre-Renewal/Transcendence `PACKETVER` target;
2025/4th-job-only features are marked out of scope.

## 1. Packet-family coverage (indicative)

Official handlers grouped by system, compared with the number of our network
events for the same family (event names differ from handler names, so this is an
order-of-magnitude signal, not a 1:1 mapping):

| family | official handlers | our events |
| --- | ---: | ---: |
| items / inventory / equip / storage / cart | 64 | 19 |
| actors / entities / movement | 32 | 9 |
| status / options | 22 | 1 |
| **guild** | 22 | 1 (emblem only) |
| party | 20 | 13 |
| **mail / Rodex** | 12 | 0 |
| skill | 11 | 14 |
| chat / whisper | 8 | 2 |
| shop / cash | 8 | 4 |
| map / zone / enter | 5 | 8 |
| **clan** | 5 | 0 |
| **pet / homunculus** | 5 | 0 |
| quest | 4 | 5 |
| **achievement / adventure guide** | 3 | 0 |
| **UI open / window** | 3 | 0 |
| trade / exchange | 2 | 6 |
| friend / messenger | 1 | 5 |

Named official handlers that have **no** counterpart at all (grep-verified):

* Guild (22): member list/add/expel/leave, positions + names + update, notice,
  chat, relations, bans, agit list/ownership/move, guild storage
  (`ReceiveGuildStorage*`).
* Mail/Rodex (12): mailbox availability, list/refresh/append, read, delete,
  return, compose result, recipients.
* Clan (5): clan info/chat/member count/leave, actor clan name.
* Pet/Homunculus (5): pet act/state, homunculus info/property/change.
* Achievement (3), UI open/window (3), cash-shop banner.
* Refine (`ReceiveRefineStatus`), grade enchant (`ReceiveGradeEnchant*`),
  Lapine upgrade, card composition (`ReceiveCardComposit*`), extra inventory,
  buying-store purchase, captcha (`ReceiveCaptchaPreview*`), macro blacklist,
  party booking (6 handlers), vending search (`ReceiveSearchStore*`).

## 2. System matrix

| system | official evidence | our status | evidence / gap |
| --- | --- | --- | --- |
| actors, jobs, sprites, palettes | `Source/Contents/Actors`, `Sprites`, `Jobs` | **mostly yes** | `world/entity`, `world/animation`, `world/jobs`, palettes, mounts (riding), cart, falcon, imf |
| movement / pathing / collision | `Contents/Movement`, `World`, `korangar-collision` | **yes** | movement/pathing in `world/`, collision crate |
| item stats/descriptions | `Contents/Items`, `Tables` | **yes** | `archive/data/client/item_stats.ron`, `item_info` window |
| inventory / equipment / storage / cart | `Contents/Items` | **yes** | inventory, equipment, storage, cart windows |
| shops (buy/sell) | `UI/Shop`, `Dialogs` | **yes** | buy/sell/buy_cart/sell_cart |
| vending | `Contents/Store` | **yes (partial)** | vending window; no buying-store *purchase* flow, no search-store |
| refine / grade enchant / card composition | `UI/Enchant`, `ReceiveRefineStatus`, `GradeEnchant`, `CardComposit` | **no** | no refine UI/packets; only stat display |
| crafting (forging, potions, cooking) | `Contents/Items`, quest skills | **partial** | skill effects exist; no crafting UI/protocol |
| NPC dialogs | `Source/Script`, `UI/Dialogs` | **yes** | dialog window, choices, next/close |
| quests | `UI/Quest`, `Contents/Quest` | **partial** | `quest_log` window + quest effect markers; no quest accept/detail protocol verified |
| navigation / path guide | `Contents/Navigation`, `UI/Navigation` | **partial** | `maps`/`big_map` only; no walking directions |
| achievements / adventure guide | `UI/Achievement`, `UI/AdventureGuide` | **no** | none |
| monster database / drop tables | `UI/Monster`, `UI/ProbabilityTable` | **no** | none |
| party (incl. booking) | `UI/Party`, 20 handlers | **partial** | party window, invites, chat, health/position; no party booking |
| guild (members/positions/notice/relations/storage) | `UI/Guild`, 22 handlers | **no** | only emblem/name display |
| clan | `UI/Clan`, 5 handlers | **no** | none |
| friends / messenger | `Contents/Messenger` | **yes** | friend list/request |
| mail / Rodex | `UI/Rodex`, 12 handlers | **no** | none |
| trade / exchange | `UI/Exchange`, `Contents/Store` | **yes** | trade window + packets |
| chat / whisper / emotions | `UI/Chat`, `UI/Emotion`, `CashEmotion` | **partial** | chat + whisper + chat commands; no emote wheel/cash emotion |
| character select / creation / deletion | `UI/Login`, `Contents/EntryQueue` | **yes** | login, server select, character select/creation/deletion; no entry queue |
| cash shop / styling shop / package box | `UI/CashShop`, `StylingShop`, `SelectPackageItemBox` | **no** | none |
| hotkeys / settings / config | `Contents/Hotkeys`, `UI/Shortcuts`, `GameSettings` | **yes** | hotbar, game/graphics/audio/interface settings |
| map / world / water / weather | `Contents/Map`, `Ground`, `World` | **partial** | map rendering, minimap/big map, water shader bits; weather effects limited |
| sound / music / movies | `Contents/Sound`, `Movies` | **partial** | audio engine, sounds, BGM hooks; no movie player UI |
| replay | `Client/Replay`, `UI/Replay` | **no** | none |
| macro | `UI/Macro` | **no** | none |
| security (hash/encryption) | `Source/Security` | **partial** | packet crypto for login; no full client hash/anticheat |
| captcha / macro blacklist | `ReceiveCaptchaPreview*`, `MacroBlackList` | **no** | none |

## 3. Top gaps, ordered by player impact

Priorities combine the official handler surface (above), our grep evidence and
the rustnarok reference implementation (wire availability checked against our
2019 opcode table).

### High impact, core gameplay

| # | gap | official / reference | where it would live | size |
| --- | --- | --- | --- | --- |
| 1 | **Guild system** (members, positions, notice, relations, storage, chat) — 22 official handlers, 29 `ZC_*GUILD*` opcodes, zero UI today | `UI/Guild`, `Contents/Guild` | `interface/windows/guild*`, `state/guild.rs`, guild packets | L |
| 2 | **RoDEX mail** — 12 handlers, `ZC_NOTIFY_NEW_MAIL` currently a noop | `UI/Rodex` | `interface/windows/rodex.rs`, packets | M/L |
| 3 | **Clickable `<ITEM>`/`<NAVI>`/`<URL>` text** in chat + NPC dialogs | `UI/Chat`, quest scripts | text layout + chat/dialog + item info | M |
| 4 | **Item comparison pane** in item info (no packets needed) | `UI/Items` | `interface/windows/item_info.rs` | M |
| 5 | **Skill cooldown display** (delay is already received) | `UI/Cooldowns` | hotbar + skill tree | S |
| 6 | **Battle mode `/bm`** (chat line currently eats letters) | `UI/GameSettings` | input/chat | S |
| 7 | **NPC text/number input** (`ZC_OPEN_EDITDLG(STR)`) — custom scripts stall today | `UI/Dialogs` | `dialog.rs` + packets | S |
| 8 | **Hotbar 2×38 + F1–F9/F12/Ctrl+F12** | `UI/Shortcuts` | `hotbar.rs`, `state/hotbar.rs` | M |
| 9 | **PvP/GvG/WoE targeting rules** (map flags parsed and dropped) | `Contents/Map` | `map_flags` → combat filters | M |
| 10 | **Refine / repair UI** (`ZC_ACK_WEAPONREFINE` 0x0221 unhandled) | `UI/Enchant` | equipment window + packets | M |
| 11 | **Bank + cash shop** (10 `BANKING`, 12 `CASH` opcodes, both wired) | `UI/CashShop` | new windows + packets | S/M |
| 12 | **Whisper windows + chat tabs** | `UI/Chat` | chat UI | M |
| 13 | **Head marks** (ice/stun/sleep/curse/silence) — `opt1/opt2` tracked, never drawn | `Contents/Actors` | entity render | S |
| 14 | **Party chat tab + party settings UI** (exp/pickup/share, leader) | `UI/Party` | `party.rs` + packets | M |
| 15 | **Settings surface + key rebinding UI** (`keybinds.rs` persists but no screen) | `UI/GameSettings`, `Shortcuts` | settings windows | S/M |

### Medium

| # | gap | evidence | size |
| --- | --- | --- | --- |
| 16 | Achievements / adventure guide | 3 handlers, `UI/Achievement` | M |
| 17 | Party booking | 6 `PartyBooking*` handlers | S/M |
| 18 | Clan | 5 handlers, `UI/Clan` | M |
| 19 | Pet / homunculus | 5 handlers, `Contents/Pet` | M/L |
| 20 | Monster DB + drop tables | `UI/Monster`, `UI/ProbabilityTable` | S |
| 21 | Navigation / path guide | `Contents/Navigation`, `ZC_NAVIGATION_ACTIVE` not in our table | M/L |
| 22 | Card composition / magnifier identify | `ZC_UPDATE_CARDSLOT`, `CZ_REQ_ITEMIDENTIFY` | S/M |
| 23 | Buying-store purchase + search store | `ReceiveSearchStore*` | S/M |
| 24 | Chat rooms, emotes/cash emotions | `CZ_CREATE_CHATROOM`, `CZ_REQ_EMOTION` | M/S |
| 25 | Notification HUD (announce, pickup, level-up, quest banner) | `UI/Events` | S |
| 26 | Screenshots / clipboard paste / FPS-ping meter | client-only | S |

### Low / product decision

Replay, entry queue, macro + captcha + macro blacklist, slot enchant/grade,
styling shop, attendance, roulette, taming, world map (overworld), window
chrome (minimise/title buttons), multi-PACKETVER switching (our server is fixed).

## 4. Rustnarok comparison## 4. Rustnarok comparison (systems, UI, protocol)

Source: `/tmp/rustnarok-client` @ `20a946ae` (221 files, 177k lines), read-only;
era filter = our 2019 opcode table (`ragnarok-packets/src/opcode_lengths_2019.rs`,
507 named opcodes, 367 server→client). Effects/status/motions are excluded (see
`korangar-actor-parity/docs/rustnarok-audit-2026-11-22.md`).

### 4.1 Headline numbers

| metric | rustnarok | RO-Next |
| --- | ---: | ---: |
| UI windows | ~80 (86 modules) | 40 user-facing (48 files) |
| UI lines | 53,797 | 16,469 |
| map-server opcodes handled | ~641 distinct `0x` constants | 141 registered handlers = **38 % of the 367 S→C 2019 opcodes** |
| unhandled-opcode policy | 1,169 listed in `undecoded.txt` | 31 listed in `noop_policy.rs` (only 24 registered) |
| in-game settings | ~40 switches over 6 tabs | `GameSettings { auto_attack }` |

### 4.2 Windows we lack that are wired on a 2019 server

Guild (29 `ZC_*GUILD*` opcodes), RoDEX mail (`ZC_NOTIFY_NEW_MAIL` is a noop today),
Bank (10 `BANKING` opcodes), cash shop (12 `CASH`), refine (`ZC_ACK_WEAPONREFINE`
0x0221), NPC text/number input (`ZC_OPEN_EDITDLG(STR)`), card insert/magnifier
(`ZC_UPDATE_CARDSLOT`, `CZ_REQ_ITEMIDENTIFY`), whisper windows, chat rooms,
buying store, monster info (`ZC_MONSTER_INFO`), cutins (`ZC_SHOW_IMAGE`),
emotion list, macros, ranking, PIN code, cart choice, merge, captcha,
party booking, battleground, clan, homunculus/mercenary, grade enchant.

### 4.3 Systems we lack (wire present in 2019)

* **Battle mode** (`/bm`) — our chat line always eats letters.
* **PvP/GvG/WoE targeting rules** — `map_flags` parsed and dropped; no
  enemy/party/guild/camp policy.
* **Clickable `<ITEM>`/`<NAVI>`/`<URL>` runs** in chat and NPC text.
* **Skill cooldown display** — `ZC_SKILL_POSTDELAY` is received and stored, never drawn.
* **Item comparison pane** (needs no packets).
* **Head marks** (ice/stun/sleep/curse/silence over the head) — `opt1`/`opt2` are
  tracked but never rendered.
* **Hotbar capacity** — one row of 10 vs 2×38 with F1–F9/F12/Ctrl+F12.
* **Key rebinding UI** — `input/keybinds.rs` persists binds, but there is no screen.
* **Settings surface** — ~20 visual/behaviour switches (aura size, damage
  abbreviation, item options, cooldown numbers, monster HP/level, far zoom).
* **Chat tabs + whisper windows**, party chat tab, party exp/pickup/share setup.
* **Notification HUD**: announce bar, pickup popup, level-up buttons, quest banner,
  FPS/ping meter, screenshots.

### 4.4 Where we are better

Typed packets with size contracts and a 2019 contract test; deterministic
unknown-packet skipping; 2019 opcode names; renderer, effects, theming and
profiling are ahead of rustnarok. Two cheap hygiene fixes on our side:
`noop_policy.rs` lists 31 packets but registers 24 (7 stale trade rows), and the
bank/cash-shop/mail/guild/pet/refine/identify families are neither handled nor
listed, which is why they never appear in a coverage report.


## 5. What was verified vs not

* Verified by grep/file presence: the "none/partial" rows above (clan, rodex,
  auction, achievement, homunculus, mercenary, battleground, booking, cash shop,
  replay, refine/enchant/craft UIs, monster DB, navigation depth, party booking,
  search store, captcha, guild storage).
* Not verified: exact quest/dialog protocol completeness, weather feature depth,
  security/hash behaviour, and whether the party window covers every 2019
  sub-feature. These need a live packet-trace session.
