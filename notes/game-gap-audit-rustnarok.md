# rustnarok-client vs korangar/RO-Next — systems, UI and protocol gap audit

**Date:** 2026-11-23
**rustnarok source (read-only):** `/tmp/rustnarok-client` — <https://github.com/kindraywind/rustnarok-client>
at `20a946ae937420b4ee204261921fac9290e2e526` (`main`, 221 Rust files under `src/`, 177,603 lines).
**Our client (read-only):** `/home/phantom/Games/ragnarok/CondaRO-linux/korangar` — workspace crates
`korangar` (`src/`, 312 files, 106,339 lines), `korangar-networking`, `ragnarok-packets`,
`korangar-interface`, `korangar-loaders`, `ragnarok-formats`.
**Server target:** 2019 rAthena, pre-renewal/transcendence, `PACKETVER` 20190605-compatible.
**Excluded (already audited 2026-11-22 in `docs/rustnarok-audit-2026-11-22.md`):**
skill effects/`src/fx`, status visuals/tints/auras, skill motion tables. Nothing below repeats those.
**Era filter used:** our own 2019 opcode table `ragnarok-packets/src/opcode_lengths_2019.rs`
(507 named opcodes, 367 server→client) is the arbiter for "exists on a 2019 server". A family whose
packets are in that table is *in scope*; a family that only appears in rustnarok's 2022/2025 tables is
marked out of scope.

---

## 1. Headline numbers

| metric | rustnarok | RO-Next | delta |
| --- | ---: | ---: | --- |
| UI modules (`src/ui/*.rs` / `interface/windows/*.rs`) | **86** (~80 windows) | **48** files (~40 in-game windows incl. skill tree, 7 debug-only) | −40 windows |
| UI lines | **53,797** | **16,469** (windows 12,218) | 3.3× smaller UI |
| average window | 673 lines | 305 lines | 2.2× |
| protocol lines | 10,920 (`net.rs` 3,489 + `net/*` 7,431) | 17,626 (`korangar-networking` 7,336 + `ragnarok-packets` 10,290) | ours is bigger/typed |
| opcodes handled | **~641** distinct `0x` constants in `net.rs` | **141** map handlers (`version_20220406.rs`), 243 typed packets | 4.5× fewer handlers |
| 2019 S→C opcodes modelled | — | 141 / 367 in the 2019 table = **38 %** | — |
| acknowledged unhandled | 1,169 in `src/net/undecoded.txt` (documented + test-enforced) | 31 in `noop_policy.rs` (24 actually registered) | our list is *under*-declared |
| game/UI settings | 6 tabs, ~30 switches (`ui/game_settings.rs` 995 lines) | `GameSettings { auto_attack }` | near-empty |

Both codebases are healthy and tested; the gap is almost entirely **breadth of screens and player-facing
systems**, not renderer or effect work.

---

## 2. UI surface

### 2.1 Windows we already have (quality delta)

Line counts are a proxy for feature depth: some windows are comparable in size, most are not, and the
per-row detail (icons, actions, tooltips, state feedback) is consistently thinner on our side.

| rustnarok window | lines | ours | lines | what rustnarok has that we don't |
| --- | ---: | --- | ---: | --- |
| `chat.rs` | 3,464 | `chat.rs` | 522 | two tabs with separate logs, per-line links (`<ITEM>/<NAVI>/<URL>`), scrollback with per-tab history, item-link hover, whisper routing, macro integration |
| `inventory.rs` | 1,477 | `inventory.rs` 209 + shared `components/item_box.rs` 366 | 575 | 8 category tabs **+ cart tab**, per-item card slot row, Ctrl+right "Magnifier" identify, drag with amount split, sort |
| `storage.rs` | 1,451 | `storage.rs` | 173 | 8 tabs, search field, sort box (Base/name/amount), per-tab category windows, kafra cart moves |
| `skills.rs` | 1,514 | `skill_tree/` | 1,206 | skill-level choice per learn, cooldown sweep + numeric seconds, party/guild skill tabs, right-click description from hotbar |
| `equipment.rs` | 1,110 | `equipment.rs` 59 + `components/paper_doll*` 851 + `equipment_stats` 387 | 1,297 | we draw 11 data-driven slots + paper doll + stats; rustnarok adds the costume tab, equip-switch sets, show/hide equip and per-slot unequip affordances |
| `party.rs` | 1,892 | `party.rs` | 482 | members' map/level/job icon, party chat tab, **exp-share / item-pickup / item-share setup UI**, member colours, LFG |
| `shop.rs` | 1,641 | `buy.rs`+`sell.rs`+`buy_cart.rs`+`sell_cart.rs`+`buy_or_sell.rs` | 1,645 | comparable size, split over five windows; rustnarok adds the deal window with running total, quantity keypad, requirement lines and sold-out states |
| `vend.rs` | 1,037 | `vending.rs` | 231 | cart-based shop editor with per-item price, shop preview as buyer |
| `hotkeys.rs` | 699 | `hotbar.rs` | 353 | **two server bars × 38 slots**, F1–F9 rows, F12 hide, Ctrl+F12 bar rotation, item slots, quick-slot window |
| `trade.rs` | 408 | `trade.rs` | 286 | zeny fields, lock/undo states, both-side item grids |
| `iteminfo.rs` | 1,096 | `item_info.rs` | 522 | **comparison with equipped item** (second pane + stat diff), card-collection view, random options in the name box |
| `basic_info.rs` | 987 | `character_overview.rs`+`stats.rs`+`menu.rs` | 1,118 | Alt+V window that also hosts the window launcher menu |
| `status.rs` | 489 | `stats.rs` | 285 | Atk/Matk/Hit/Crit/Def/Mdef/Flee/ASPD block, guild line, cost preview on the + arrows |
| `char_make.rs` | 451 | `character_creation.rs` | 67 | race, gender, hairstyle + colour with live preview, name validation |
| `char_select.rs` | 822 | `character_selection.rs` | 1,228 | ours is richer here — no gap |
| `minimap.rs` + `minimap_window.rs` | 1,215 | `minimap.rs`+`big_map.rs`+`components/minimap.rs` | 467 | see-through/opaque/hidden cycle (Ctrl+Tab), zoom ±, party/guild dots with per-member colours, boss mark, coords, marker filters |
| `quest.rs` + `quest_tracker.rs` + `quest_notice.rs` | 1,589 | `quest_log.rs` | 68 | tabbed quest window (in progress / recommended / hold / waiting), tracker with per-objective counts, quest banner |
| `npc_say.rs`+`npc_menu.rs`+`npc_input.rs` | 1,351 | `dialog.rs` | 586 | window-size/pos packets honoured, **number/text input dialog**, progress-bar dialog |
| `status_tip.rs` | 213 | `buff_bar.rs` | 357 | ours is comparable (icon + remaining time + tooltip) — no gap |
| `servers.rs`, `menu.rs`, `options.rs`, `escape.rs`, `front.rs`, `cursor.rs`, `line_edit.rs`, `window.rs` | 3,243 | window framework in `korangar-interface` + `menu.rs`/`context_menu.rs`/`cursor/` | ≈1,400 | title-bar buttons/minimise, z-order list with "top" groups, animated cursor sprites |

### 2.2 Windows we lack that are relevant to a 2019 server

All line counts are rustnarok's. "Wire" = the packet family exists in our 2019 opcode table
(`opcode_lengths_2019.rs`), i.e. our server can actually send it.

| # | window | lines | wire in 2019? | relevance | size |
| --- | --- | ---: | --- | --- | --- |
| 1 | `guild.rs` | 2,042 | yes (29 `ZC_*GUILD*`) | **high** (WoE) | L |
| 2 | `mail.rs` (RoDEX) | 1,147 | yes (`ZC_NOTIFY_NEW_MAIL`, `ZC_ACK_READ_RODEX`, `ZC_ACK_ADD_ITEM_RODEX`) | **high** | L |
| 3 | `bank.rs` | 426 | yes (10 `BANKING`) | **high** | S |
| 4 | `cashshop.rs` | 950 | yes (12 `CASH`) | **high** | M |
| 5 | `refine.rs` | 1,152 | yes (`ZC_ACK_WEAPONREFINE` 0x0221, `ZC_REPAIRITEMLIST`) | **high** (core pre-re) | M |
| 6 | `npc_input` (text/number) | 183 | yes (`ZC_OPEN_EDITDLG`, `ZC_OPEN_EDITDLGSTR`) | **high** (custom quest NPCs) | S |
| 7 | `pick_list.rs` (card insert, magnifier, make/repair/upgrade lists, spell select) | 655 | yes (`ZC_UPDATE_CARDSLOT`, `CZ_REQ_ITEMIDENTIFY`, `ZC_SKILL_SELECT_REQUEST`) | **high** | M |
| 8 | `iteminfo` comparison pane | (in 1,096) | no packet needed | **high** QoL | M |
| 9 | `whisper.rs` | 757 | yes (6 `WHISPER`) | **high** social | M |
| 10 | `shortcut_window.rs` + `shortcuts.rs` | 1,855 | client-side | **high** (key rebinding UI absent) | M |
| 11 | `pet.rs` | 615 | yes (4 `PET`) | medium (pre-re pets) | M |
| 12 | `buying.rs` (buying store) | 541 | yes (`ZC_MYITEMLIST_BUYING_STORE`, `ZC_BUYING_STORE_ENTRY`) | medium | M |
| 13 | `chatroom.rs` | 578 | yes (4 `CHATROOM`) | medium | M |
| 14 | `monster_info.rs` (Sense) | 137 | yes (`ZC_MONSTER_INFO`) | medium | S |
| 15 | `catalog.rs` (search player stores) | 869 | yes (`ZC_OPEN_SEARCH_STORE_INFO`, `ZC_SEARCH_STORE_INFO_ACK`) | medium | M |
| 16 | `roulette.rs` | 486 | partial (`ZC_ACK_CLOSE_ROULETTE`) | medium | M |
| 17 | `cutin.rs` (NPC/skill image, `ZC_SHOW_IMAGE`) | 143 | yes (`ZC_SHOW_IMAGE`) | medium | S |
| 18 | `emotion_list.rs` | 147 | yes (`CZ_REQ_EMOTION`) | low-medium | S |
| 19 | `macros.rs` (Alt+M, Alt+1..0) | 155 | client-side | medium | S |
| 20 | `ranking.rs` | 194 | yes (12 `RANK`) | low | S |
| 21 | `pin.rs` (2nd password) | 582 | yes (`HC_PINCODE` 0x08B9) | medium (server-dependent) | S |
| 22 | `cart_choice.rs` | 238 | yes | low-medium | S |
| 23 | `merge.rs` (item merge) | 442 | yes (`ZC_MERGE_ITEM_OPEN`) | low | M |
| 24 | `worldmap.rs` (overworld map, not the current-map minimap) | 268 | client-side | low-medium | M |
| 25 | `navi_window.rs` + `navigation.rs` | 2,539 | rAthena's 2016+ `ZC_NAVIGATION_*` is **absent from our 2019 table** — verify against the server; port only if it really sends it | medium | L |
| 26 | `captcha.rs` (macro detector) | 180 | yes (5 `CAPTCHA`) | medium (server-dependent) | S |
| 27 | `booking_window.rs` + `booking.rs` | 573 | yes (`ZC_PARTY_REQ_MASTER_TO_JOIN`, `ZC_PARTY_JOIN_REQ_ACK_FROM_MASTER`) | low-medium | M |
| 28 | `battle_queue.rs` + `battleground.rs` | 407 | yes (`ZC_BATTLEFIELD_*`) | low-medium | L |
| 29 | `clan.rs` | 156 | yes (4 `CLAN`) | low | M |
| 30 | `homunculus.rs` / `mercenary.rs` | 1,118 | partial (`ZC_PROPERTY_HOMUN`, `ZC_FEED_MER`) | low | L |
| 31 | `forge.rs` / `material.rs` | 555 | partial (`ZC_ACK_ITEMREPAIR`, make lists) | low | S |
| 32 | `storage_log.rs` | 327 | guild storage log — not in our table | low | S |
| 33 | `dressroom.rs` / `stylist.rs` | 1,385 | partial (`ZC_DRESSROOM_OPEN`) | low | M |
| 34 | `grade.rs` (item grade enchant) | 875 | yes (`ZC_ACK_RANDOM_UPGRADE_ITEM`) | low | M |
| 35 | `special_popup.rs` | 38 | no | low | S |
| 36 | `taming.rs` | 123 | no (`ZC_TRYCAPTURE_MONSTER` only) | low | S |
| 37 | `attendance.rs` | 375 | not in our 2019 table (era unclear) | low | S |
| 38 | `achievement.rs` | 480 | no (`ZC_ACK_ACHIEVEMENT_LIST` exists but is post-2019 rAthena) | out of scope | — |
| 39 | `enchant.rs` (2021 enchanters) | 1,191 | no | out of scope | — |
| 40 | `reform.rs`, `laphine.rs`, `reputation.rs`, `memorial.rs`, `goldpc.rs`, `package.rs`, `level_up.rs`, `announce.rs`, `pickup.rs`, `quest_notice` (banner part), `meter.rs` | 3,452 | mostly client-side | low / QoL | S |

### 2.3 HUD/overlay pieces (not windows, but visible)

| rustnarok | ours | status |
| --- | --- | --- |
| `announce.rs` — broadcast boxes across the top (3 max, 20 s) | none; broadcasts go into chat | **no** |
| `pickup.rs` — "you picked up X" icon popup, 5 s | none | **no** |
| `level_up.rs` — level-up buttons in the bottom corners | none | **no** |
| `quest_notice.rs` — quest banner + hunt progress line | none | **no** |
| `meter.rs` — FPS + map-server ping (12 s probe) | debug `profiler.rs` (debug builds only) | **partial** |
| `status_tip.rs` — status icon strip with name/desc/time | `buff_bar.rs` (same feature, themed) | **yes** |
| `cursor.rs` — animated `cursors.spr` that changes by target | `interface/cursor/` | **yes** |
| `overhead.rs` — skill shout balloons + chat balloons + party HP bars | `world/skill_name_text.rs`, `world/chat_bubble.rs`; no party HP bar over heads | **partial** |
| `head_marks.rs` — ice block / stun / sleep / curse / silence marks over heads | none (we track `opt1`/`opt2` in `world/entity/mod.rs`, never render a mark) | **no** |

---

## 3. Gameplay systems

### 3.1 Battle mode (`/bm`) — **no**
`src/battle_mode.rs` (405 lines) + `ui/game_settings.rs` "Control" tab. The chat line stays closed and
letter keys are shortcuts; Enter/Esc semantics; the chat icon toggles it. Our `input/` has chat focus
handling but no mode: `grep -ri "battle_mode\|ChangeChatMode\|/bm"` → 0 hits.
Every RO player uses this; **high relevance, size S** (`input/mod.rs` + `input/keybinds.rs` + a settings row).

### 3.2 PvP / GvG / WoE targeting rules — **no**
`src/versus.rs` (210 lines): `Side { flags, party, guild, camp }`, `enemy()`, `attack_cursor()`,
`click()`, `siege(flags)`, `numbers_hidden(flags)` — party/guild/camp/Shift rules, emblem rendering on
siege maps, damage numbers suppressed on siege/BG. Ours parses `ZC_NOTIFY_MAPPROPERTY2` into
`state/mod.rs: map_type, map_flags` and **never reads them again** (`grep` shows only the two writes in
`lib.rs:6607`). `grep -ri "pvp\|gvg\|woe\|siege\|battleground"` in `korangar/src` → 0 hits.
WoE is core to a pre-re server; **high relevance, size M** (`world/` targeting + `input/` click policy +
`world/entity` emblem/name rules).

### 3.3 Captcha (macro detector) — **no**
`ui/captcha.rs` (180 lines): image arrives in chunks (`ZC_APPLY_MACRO_DETECTOR_CAPTCHA`), 3 preview
variants, 30 s; answer sent with `CZ_COMPLETE_APPLY_MACRO_DETECTOR_CAPTCHA`. Our 2019 table has all
five packets, no handler. Only matters if the server enables `macro_detector`; **medium, size S**.

### 3.4 Party booking — **no** (join path only)
`booking.rs` (234) + `ui/booking_window.rs` (339): level range/purpose/roles, multipart HTTP to the
rAthena web server (`/party/add`, `/search`, `/list`, `/del`), then an in-game join request. We have
`PartyBookingJoinRequestPacket2019`/`Result` and `noop_policy.rs` says *"LFG party-booking UI not
implemented"*. **low-medium, size M**.

### 3.5 Battlegrounds — **no**
`battleground.rs` (80) + `ui/battle_queue.rs` (327): camp, score, teammate positions on the minimap,
battle chat, queue apply/ready/accept. No equivalent in ours. Pre-re servers often skip BG;
**low-medium, size L**.

### 3.6 Bank / cash shop / roulette / catalog — **no**
`ui/bank.rs` 426 (deposit/withdraw with carried vs account amounts), `ui/cashshop.rs` 950 (category
tabs, pages, points, cart), `ui/roulette.rs` 486 (42 prizes, gold/silver/bronze spins),
`ui/catalog.rs` 869 (search every player store for an item or the card inside it).
All four families are in our 2019 table and unhandled. Bank and cash shop are **high** (S / M);
roulette and catalog **medium** (M).

### 3.7 Head marks — **no**
`src/head_marks.rs` (139): `Mark::{Stun, Sleep, Curse, Silence, Freeze}`, sprite names
(`status-stun`, `status-sleep`, `status-curse`, `얼음땡`), anchors (`ANCHOR = 20.0`, `CURSE_ANCHOR = 15.0`),
and `marks(body, health)` from `opt1`/`opt2`. We store both fields (`world/entity/mod.rs:229-233`) but
render nothing. **medium, size S** (`world/entity` + a particle/text overlay).

### 3.8 Emotions — **partial**
`src/emotion.rs` (150): the `ET_*` table with sprite action and *all* the chat commands per emotion
(`/!`, `/ho`, `/lv`, `/swt`, Korean shortcuts…), plus `ui/emotion_list.rs` (147) picker.
Ours: `Alt`+`1..9` binds (`input/keybinds.rs:40`, `emotion_keys`), `DisplayEmotion` event, sprite played
in `world/particles/mod.rs:264` (`Emotion` particle). Missing: emotion list window, the command aliases,
expanded emotion packs. **low-medium, size S**.

### 3.9 Cutins (`ZC_SHOW_IMAGE`) — **no**
`ui/cutin.rs` (143) with placement types; `events.rs:3488` handles `MapEvent::Cutin`. Our
`noop_policy.rs` registers 0x01B3 as `OwnedElsewhere` ("item-descriptions lane") — i.e. known and
deferred, not implemented. **medium, size S** (one texture window + event arm).

### 3.10 Map events: weather / sky / ambient / map effect objects — **partial**
rustnarok: `fx/weather.rs` (7 weather flags → snow/sakura/leaves/clouds/fog/fireworks),
`fx/sky.rs` (12 sky effect ids, clouds/clouds2/fog/star fields), `fx/map_effects.rs` (RSW type-4
emitters, fireflies, torches on 16 ms ticks), `ambient.rs` (RSW sound sources with distance fade,
cycle restart).
Ours: `world/effect/weather.rs` handles `ZC_NOTIFY_EFFECT2` for snow/sakura/leaves — good.
Sky is explicitly **TODO** in `loaders/map/mod.rs:136` (`// TODO: NHA Implement sky rendering`).
RSW ambient sound sources and RSW effect objects are parsed for picking
(`graphics/picker_target.rs: SoundSourceMarker/EffectSourceMarker`) but not played/spawned.
**low-medium, size M** (`loaders/map/mod.rs` + `world/sound`).

### 3.11 Minimap markers — **partial**
rustnarok `ui/minimap.rs` (628) + `ui/minimap_window.rs` (587): see-through/opaque/hidden,
zoom ±, party dots coloured by slot, guild dots, quest NPC marks (click → navigate), boss mark,
coordinates, marker filters (`checks_from`), memo marker.
Ours `state/minimap.rs` + `components/minimap.rs`: server NPC/quest markers (`ZC_COMPASS`), player
direction arrow, no zoom, no opacity cycle, no party/guild dots, no click-to-navigate.
**medium, size M**.

### 3.12 Quest tracking — **partial**
rustnarok: `ui/quest.rs` 1,186 (four tabs, rewards, recommended list), `ui/quest_tracker.rs` 212
(per-objective counters under the minimap), `ui/quest_notice.rs` 191 (banner + hunt progress).
Ours: `state/quest_log.rs` (list/objectives/updates) + `interface/windows/quest_log.rs` (68 lines:
tracker list only), toggled by `InputEvent::ToggleQuestWindow` (`lib.rs:7200`). No quest detail view,
no banner, no rewards. **medium-high, size M**.
Note: `noop_policy.rs` still gives the quest family the reason "quest log window not implemented"
(0x09F8/0x09F9/0x09FA/0x08FE/0x02B4) although `QuestListPacket` (0x09F8) and
`QuestNotificationPacket1` (0x09F9) are registered as real handlers at `version_20220406.rs:1103-1104`
— the triage table and the handlers need reconciling (see §5).

### 3.13 Item comparison — **no**
`ui/iteminfo.rs`: a second "compare" window beside the description (`pair()`, `compare_background`,
`comparison_empty_card_slot`, `where it goes (0: nothing to compare with)`), plus card-collection
comparison. Ours `interface/windows/item_info.rs` has no compare pane and no equipped-item diff.
**high QoL, size M**.

### 3.14 Skill cooldown display — **no (state only)**
rustnarok: cooldown sweep + numeric seconds on hotbar/skill slots, switchable ("Cooldown numbers" in
`game_settings.rs`). Ours: `state/skill_cooldowns.rs` parses `ZC_SKILL_POSTDELAY` (0x043D) and its own
doc says *"Cooldown UI is owned by the skill-souls/cast-bar lane; this state is the typed packet-layer
hand-off"* — `grep skill_cooldown interface/ world/` → 0 hits. **high, size S** (hotbar + skill tree
sweep + one settings flag).

### 3.15 Camera behaviour — **partial**
rustnarok: `Camera { target, distance, pitch, yaw, ground }` with goals (`camera_goal`), `/camera`
zoom-out toggle, `far zoom` setting, indoor handling, screen-shake effect (`/quake`).
Ours: `world/cameras/` (1,148 lines incl. debug/point-shadow/smoothed, start) with yaw/pitch/distance,
MouseInputMode::RotateCamera. No zoom-out/far-zoom option, no screen shake. **low, size S**.

### 3.16 Chat rooms — **no**
`ui/chatroom.rs` (578) + 9 `MapEvent` variants (`ChatRoom`, `ChatRoomMade`, `ChatRoomJoined`,
`ChatRoomOwner`…). Wire present in 2019 (`CZ_CREATE_CHATROOM`, `ZC_CHANGE_CHATROOM`). No equivalent.
**medium, size M**.

### 3.17 Chat macros — **no**
`ui/macros.rs` (155): 10 lines set in the macro window (Alt+M), sent with Alt+1..0, persisted in the
settings folder. **medium-low, size S**.

### 3.18 Party / guild tooling — **partial / no**
Party: ours has roster + HP bars (`interface/windows/party.rs`, `state/party.rs`: health, position,
job level, alive), but no party chat tab, no member map/level display, no **exp-share / pickup / item-share
setup** (rustnarok `setup_lines(exp, pickup, share)`), no leader change UI, no LFG.
Guild: **nothing** — no guild window at all (`grep -ri guild korangar/src` hits only
`world/guild_emblem.rs` and entity name rendering). 29 guild opcodes are in our 2019 table.
Party **medium M**, guild **high L**.

### 3.19 NPC text/number input — **no**
`ui/npc_input.rs` (183) binds `ZC_OPEN_EDITDLG` / `ZC_OPEN_EDITDLGSTR` and sends
`CZ_INPUT_EDITDLG(STR)`. Our `dialog.rs` supports text lines, Next, Close, menu options and warp lists
(`DialogElementKind::{Text, NextButton, CloseButton, Menu}`) but no input box, so a script using
`input()` or a numeric prompt stalls. **high (custom NPCs), size S**.
Also missing: `ZC_SAY_DIALOG` window size/position packets (`NpcWindowSize`/`NpcWindowPos` handled as
events in rustnarok) and the progress-bar dialog.

### 3.20 Item/Navi/URL links in text — **no**
`src/ui/links.rs` (399): parses the three item forms (`<ITEM>name<INFO>id</INFO></ITEM>`,
`<ITEMLINK>`, `<ITEML>`), `<NAVI>map,x,y</NAVI>`, `<URL>`, `<QUEST>`, `<TIPBOX>` into marked runs that
keep links through wrapping; click opens the item description, starts navigation, opens the browser.
Ours: chat and dialog draw plain text (`grep ITEMLINK\|href` → 0). This is how players link gear in
trade chat; **high, size M** (text layout + chat + dialog + item_info).

### 3.21 Echoes of the official HUD we lack — **no** (all small)
Announce bar (`ui/announce.rs` 145), pickup popup (`ui/pickup.rs` 242), level-up corner buttons
(`ui/level_up.rs` 133), quest banner (`ui/quest_notice.rs` 191), FPS/ping meter (`ui/meter.rs` 85),
screenshot on Print Screen (`screenshot.rs` 30, `input.rs:3819`), clipboard paste into text boxes
(`input.rs:4122`). Our equivalents: none (screenshots/clipboard are trivial wins).

### 3.22 MVP / kill feedback / extended damage — **no**
`fx/exp_shown.rs` (base+job exp float on kill), MVP display (`ZC_MVP`, `ZC_MVP_GETTING_ITEM`,
`ZC_THROW_MVPITEM` in our 2019 table, unhandled), combo window (`MapEvent::ComboWindow`),
"extended damage" K/M abbreviation setting. Ours has damage/heal/miss numbers with magnitude scaling
(`world/particles/mod.rs:103`) but no combo, no kill exp, no MVP.
MVP **medium S**; the rest **low S**.

### 3.23 Autohunt / MCP / control socket / addon — **out of scope (design)**
`autohunt.rs` 347, `mcp.rs` 171, `control.rs`, `play.rs` 77, `addon.rs` 132: a JSON control socket for
an external agent, a plugin that plays the character, an MCP server binary, and a server-side "addon"
sidecar with its own JSON protocol. Rustnarok explicitly keeps hunt logic *out* of the client. This is
2025 tooling with no server-side counterpart; not recommended for a live 2019 server (it is, in
effect, a first-class botting surface). Its useful idea for us is **debug automation**, which we
already have via `feature = "debug"` windows.

### 3.24 Identify / Magnifier, card slots, item merge — **no**
`ui/inventory.rs` Ctrl+right magnifier → `CZ_REQ_ITEMIDENTIFY`/`ZC_ACK_ITEMIDENTIFY` (both in our 2019
table, unhandled); `ZC_UPDATE_CARDSLOT` for card composition, `ZC_MERGE_ITEM_OPEN` for merge.
Identify **medium, S**; merge **low, S**.

---

## 4. Networking / protocol

### 4.1 Structure

| | rustnarok | RO-Next |
| --- | --- | --- |
| shape | one `net.rs` (3,489) with hand-written `match` on `u16` opcode, `decode.rs` (1,231) using raw `u16_at/i32_at` offsets, `map_session.rs` (1,272) for senders, `map_event.rs` (1,574) for the event enum | derive-macro typed packets (`#[derive(Packet, ServerPacket, MapServer)]`) in `ragnarok-packets` (243 packets, 10,290 lines) + a registry of handlers in `korangar-networking/src/packet_versions/version_20220406.rs` (158 `register` calls: 3 login / 14 char / 141 map) emitting 143 typed `NetworkEvent`s |
| safety | bounds by convention; length tables as text (`packet_lengths.txt` 1,304 / `_extra` 269 / `_20250402` 1,313) | `ByteReader`, `IS_VARIABLE_LENGTH`, `EXPECTED_SIZE`, per-packet size checks, plus an **offline contract suite** `contract_2019.rs` (743) that parses every registered handler's 2019 wire size and asserts exact consumption |
| versions | runtime: `packetver()` from `clientinfo.xml` `<packetver>` or `RUSTNAROK_CLIENT_PACKETVER`, default 20220406, newer table from 20250402; per-packet gates (`PACKETVER_TAKEOFF_LOCATION`, `PACKETVER_EMOTION_EXPANSION`) | compile-time: `enum SupportedPacketVersion { _20220406 }`, `ALL` = one entry; the 2019 opcode table is the contract reference |
| event surface | 361 `MapEvent` variants + 13 `CharEvent` | 143 `NetworkEvent` variants |
| unhandled | 1,169 opcodes listed in `undecoded.txt` with a reason, enforced by `every_known_packet_is_decoded_or_listed` | 31 classified noops in `noop_policy.rs` (several reasons stale) and whole families (bank/cashshop/mail/guild/pet/refine/identify/cards) are **not even listed** |

**Verdict:** our protocol layer is structurally better (typed, version-checked, contract-tested); rustnarok
is 4.5× broader in coverage and honest about what it skips.

### 4.2 Server→client families rustnarok handles that we do not (2019-relevant)

Each row cites rustnarok's `MapEvent` variants; "wire" = opcodes present in `opcode_lengths_2019.rs`.

| family | rustnarok events | wire 2019 | our status |
| --- | --- | --- | --- |
| Bank | `BankBalance`, `BankMoved` | 10 opcodes | no handler, not in noop list |
| Cash shop | `CashShopOpen`, `CashShopList`, `CashShopBuyResult`, `PointShop*` | 12 opcodes | no |
| RoDEX mail | `MailUnread/List/Read/Claimed/Deleted/Write/Receiver/ItemAdded/ItemRemoved/Sent` | `ZC_NOTIFY_NEW_MAIL`, `ZC_ACK_READ_RODEX`, `ZC_ACK_ADD_ITEM_RODEX` | noop (`ZC_NOTIFY_NEW_MAIL` only) |
| Guild | `Guild`, `GuildInfo`, `GuildCastles`, `GuildMenu`, `RaiseAnswer`, `GuildMembers`, `GuildPositions`, `GuildPositionInfo`, `GuildMemberPosition`, `PositionTitle`, `GuildSkills`, `GuildExpelled`, `GuildEmblem`, `GuildNotice`, `GuildChat`, `GuildRelations`, `Ally*`, `GuildDisbanded`, `GuildLeft` | 29 opcodes | only `GuildEmblemImagePacket` |
| Refine / repair / upgrade lists | `RefineOpen`, `RefineMaterials`, `Refined`, `RefineAnnounced`, `RepairList`, `Repaired`, `UpgradeList`, `Upgraded` | `ZC_ACK_WEAPONREFINE` 0x0221, `ZC_REPAIRITEMLIST` | no |
| Card / identify / make lists | `CardList`, `CardInserted`, `IdentifyList`, `Identified`, `MakeList`, `CookList`, `ArrowList`, `SelectCart`, `SpellList`, `SkillSelect` | `ZC_UPDATE_CARDSLOT`, `ZC_ACK_ITEMIDENTIFY`, `ZC_SKILL_SELECT_REQUEST` | no |
| Pet | `PetEggs`, `PetState`, `PetInfo`, `PetFed`, `PetAct`, `PetAutoFeed` | 4 `PET` opcodes | no |
| Monster info / boss | `MonsterInfo`, `BossInfo` | `ZC_MONSTER_INFO` | no |
| Chat rooms | 9 variants | 4 opcodes | no |
| Buying stores | `OpenBuyingStore`, `MyBuyingStore`, `BuyingStoreSign*`, `BuyingStoreItems`, `BuyingStoreSold`… | `ZC_MYITEMLIST_BUYING_STORE` etc. | no |
| Merge | `MergeOpen`, `Merged` | 3 opcodes | no |
| Ranking | `Ranking`, `CharName`, `RankingPoints` | 12 opcodes | no |
| PvP / BG | `BattleHp`, `BattleCamp`, `BattleQueue`, `Queue*`, `BattleScore`, `BattleChat`, `PvpRank`, `BattlePosition` | `ZC_BATTLEFIELD_NOTIFY_HP` | no |
| MVP | `Mvp`, `MvpItem`, `MvpExp`, `MvpOverweight` | 4 opcodes | no |
| Cutin | `Cutin` | `ZC_SHOW_IMAGE` | noop (owned elsewhere) |
| NPC input / window / progress | `NpcInput`, `NpcWindowSize`, `NpcWindowPos`, `Progress`, `ProgressActor`, `ProgressCancel` | 4 `EDITDLG` opcodes | no |
| Navigate / warp list / memo | `Navigate`, `WarpList`, `MemoResult`, `MapForbids` | `ZC_NAVIGATION_*` not in our table (verify server side) | partial (`WarpPointList` only) |
| NPC music / talkie box | `NpcMusic`, `TalkieBox` | `ZC_PLAY_NPC_BGM` | no |
| Dressroom / style | `DressRoom`, `StyleChanged` | `ZC_DRESSROOM_OPEN`, `ZC_STYLE_CHANGE_RES` | no |
| Grade enchant | `GradeMaterials`, `GradeResult`, `GradeAnnounced` | `ZC_ACK_RANDOM_UPGRADE_ITEM` | no |
| Captcha | `CaptchaStart`, `CaptchaChunk`, `CaptchaAsk`, `CaptchaClosed` | 5 opcodes | no |
| Clan | `ClanInfo`, `ClanOnlineCount`, `ClanLeft`, `ClanChat` | 4 opcodes | noop (2 of 4) |
| Market/barter, item preview, reform, laphine, enchant, achievement, star/soul/shadow | — | absent from our 2019 table | out of scope (product decision) |

Out of scope by era (2025/4th job): `StarSkill`, `Spirits`, `Shields`, `Charms`, `Souls`,
`ShadowScars`, `TitleChanged`, `Elemental*` (absent from our table), achievement, reputation, memorial,
package, laphine/enchant/reform, gold PC.

### 4.3 Things our protocol layer does better
- Typed packets with per-struct `EXPECTED_SIZE` + a registry that fails the build/test if a layout drifts.
- `contract_2019.rs` offline validation (no live server needed) + byte-exact fixtures for pickup,
  party, skills, shop, friends, hotkeys.
- Deterministic unknown-packet skip (`unknown_packet_action`) instead of a silent desync.
- 2019 opcode table with 507 *names* (rustnarok's tables are numbers + a separate undecoded list).

---

## 5. Quality-of-life, input, settings, performance

| area | rustnarok | ours | relevance | size |
| --- | --- | --- | --- | --- |
| in-game settings | `ui/game_settings.rs` 995: Basic / Effect / Control / Graphics / Other / Extras tabs; ~30 switches incl. Show aura, Mini aura, Screen shake, Extended damage, Quick Slots, Monster HP, Monster level, Far zoom, Item options in name box, **Cooldown numbers**, Exp to 0.01 %, Kill exp, Interface grows | `settings/game.rs` = `GameSettings { auto_attack }`; visual toggles scattered in `settings/interface.rs` (character names, chat bubbles, transparent chat/minimap/hotbar, skill names) | **high** QoL | M |
| key rebinding | `ui/shortcut_window.rs` 490 + `shortcuts.rs` 1,365 (defaults from `hotkey.lub`, `UserKeys.lua` overrides, 4 tabs) | `input/keybinds.rs` supports rebinds and persists them, but **no UI** (`grep BindKey interface/` → 0) | high | S |
| hotbar capacity | 2 bars × 38 server slots, F1–F9, F12, Ctrl+F12, quick-slot window | 1 row of 10 | high | M |
| item links / paste | `links.rs` 399, clipboard paste in `line_edit`/`input.rs:4122` | none | high | M |
| item comparison | `iteminfo` compare pane | none | high | M |
| status marks | `head_marks.rs` | none | medium | S |
| minimap zoom/opacity | `ui/minimap.rs` | none | medium | S |
| screenshots | `screenshot.rs` (JPEG, numbered, Print Screen) | none | low | S |
| FPS/ping meter | `ui/meter.rs` (12 s `CZ_REQUEST_TIME`) | debug profiler only | low | S |
| window chrome | `ui/window.rs` 972: title bar, close/minimise, drag, tooltips | themed frames in `korangar-interface`; no minimise/title buttons | low | M |
| multi-PACKETVER | runtime switch, per-packet gates | single compiled version | low (our server is fixed) | L |
| renderer/effects/perf | — | ours is far ahead (wgpu, shadow passes, theme tokens, profiler) | — | — |

Two hygiene findings on our side, both cheap:
- `noop_policy.rs` declares 31 classified packets but has only 24 `register_ignored_noop` call sites,
  and several reasons are stale: the trade entries say "no trade UI/state yet" while
  `interface/windows/trade.rs` + `state/trade.rs` exist, and the quest entries say the quest log is not
  implemented while `QuestListPacket`/`QuestNotificationPacket1` are live handlers. The table is meant
  to be the authoritative "what we deliberately skip" list, so stale rows hide real gaps.
- Bank / cash shop / RoDEX / guild / pet / refine / identify / card-slot / monster-info families are
  **not in the noop table at all** — they are unregistered *and* unclassified, which is why they never
  show up in a coverage report.

---

## 6. Master gap table

`ours`: yes / partial / no. `rel`: relevance for a 2019 pre-re server. `size`: S ≤ ~200 lines/1 day,
M 200–700/2–4 days, L > 700/1 week+.

| # | feature | rustnarok evidence | ours | rel | size | where we'd add it |
| ---: | --- | --- | --- | --- | --- | --- |
| 1 | Guild window | `ui/guild.rs` 2,042; 29 `ZC_*GUILD*` | no | high | L | `interface/windows/guild.rs`, `state/guild.rs`, `NetworkEvent::Guild*` |
| 2 | RoDEX mailbox | `ui/mail.rs` 1,147; `MailUnread..MailSent` | no | high | L | `windows/mail.rs`, `state/mail.rs`, register `0x09E7` family |
| 3 | Bank | `ui/bank.rs` 426; 10 `BANKING` opcodes | no | high | S | `windows/bank.rs`, `state/bank.rs`, 10 handlers |
| 4 | Cash shop | `ui/cashshop.rs` 950 | no | high | M | `windows/cash_shop.rs`, `state/cash_shop.rs` |
| 5 | Refine / repair UI | `ui/refine.rs` 1,152; `ZC_ACK_WEAPONREFINE` 0x0221 | no | high | M | `windows/refine.rs`, `state/refine.rs` |
| 6 | NPC text/number input | `ui/npc_input.rs` 183; 4 `EDITDLG` opcodes | no | high | S | `windows/dialog.rs` (`DialogElementKind::Input`), 2 handlers |
| 7 | Item comparison | `ui/iteminfo.rs` compare pane | no | high | M | `windows/item_info.rs` second pane + `state/inventory` equipped lookup |
| 8 | Item/Navi links in text | `ui/links.rs` 399 | no | high | M | `interface/components/text` + `chat.rs` + `dialog.rs` |
| 9 | Battle mode `/bm` | `battle_mode.rs` 405 | no | high | S | `input/mod.rs`, `settings/game.rs` |
| 10 | PvP/GvG/WoE targeting + siege rules | `versus.rs` 210 | no (flags parsed, unused) | high | M | `world/` target policy, `input/` click, `lib.rs` |
| 11 | Skill cooldown display | cooldown sweep + numbers on hotbar | partial (state only) | high | S | `windows/hotbar.rs`, `windows/skill_tree/` |
| 12 | Hotbar 2 bars × 38 + F1–F9/F12 | `ui/hotkeys.rs` 699 | partial (10 slots) | high | M | `windows/hotbar.rs`, `state/hotbar.rs` |
| 13 | Key rebinding UI | `ui/shortcut_window.rs` 490 | no (engine exists) | high | S | `windows/shortcut_settings.rs` |
| 14 | In-game settings surface | `ui/game_settings.rs` 995 | no (`GameSettings{auto_attack}`) | high | M | `settings/game.rs` + `windows/game_settings.rs` |
| 15 | Whisper windows | `ui/whisper.rs` 757; 6 `WHISPER` | no (chat prefill) | high | M | `windows/whisper.rs`, `state/whisper.rs` |
| 16 | Quest log (tabbed) + banner | `ui/quest.rs` 1,186, `quest_tracker` 212, `quest_notice` 191 | partial (68-line tracker) | med-high | M | `windows/quest_log.rs`, state already exists |
| 17 | Card insert / Magnifier identify | `pick_list.rs` 655, `inventory.rs` magnifier | no | med-high | M | `windows/pick_list.rs`, 2 handlers |
| 18 | Inventory depth (card slots, cart tab, sort) | `ui/inventory.rs` 1,477 | partial | med-high | M | `windows/inventory.rs` |
| 19 | Equipment: costume tab, equip-switch, show/hide | `ui/equipment.rs` 1,110 | partial (11 slots + doll + stats exist) | med-high | M | `windows/equipment.rs`, `components/paper_doll_slots.rs` |
| 20 | Storage tabs/search/sort | `ui/storage.rs` 1,451 | partial | med-high | M | `windows/storage.rs` |
| 21 | Character creation (race/gender/hair preview) | `ui/char_make.rs` 451 | partial (name only) | high | M | `windows/character_creation.rs` |
| 22 | Party options (exp/pickup/share) + chat | `ui/party.rs` 1,892 `setup_lines` | partial | med | M | `windows/party.rs`, `state/party.rs` |
| 23 | Minimap zoom/opacity/party+guild dots | `ui/minimap.rs` 628 | partial | med | M | `components/minimap.rs`, `state/minimap.rs` |
| 24 | Head marks (freeze/stun/sleep/curse) | `head_marks.rs` 139 | no | med | S | `world/entity/mod.rs` |
| 25 | Cutin (`ZC_SHOW_IMAGE`) | `ui/cutin.rs` 143 | no (noop) | med | S | `windows/cutin.rs`, handler |
| 26 | Monster info (Sense) | `ui/monster_info.rs` 137 | no | med | S | `windows/monster_info.rs` |
| 27 | MVP display | `Mvp/MvpItem/MvpExp/MvpOverweight` | no | med | S | `lib.rs` effect arm + text |
| 28 | Pet window | `ui/pet.rs` 615 | no | med | M | `windows/pet.rs`, `state/pet.rs` |
| 29 | Buying stores | `ui/buying.rs` 541 | no | med | M | `windows/buying.rs` |
| 30 | Chat rooms | `ui/chatroom.rs` 578 | no | med | M | `windows/chat_room.rs` |
| 31 | Macros (Alt+M, Alt+1..0) | `ui/macros.rs` 155 | no | med | S | `state/macro.rs` + input |
| 32 | Search-store catalog | `ui/catalog.rs` 869 | no | med | M | `windows/catalog.rs` |
| 33 | Roulette | `ui/roulette.rs` 486 | no | med | M | `windows/roulette.rs` |
| 34 | Captcha | `ui/captcha.rs` 180 | no | med | S | `windows/captcha.rs` |
| 35 | Refine/repair item lists | `pick_list` text mode | no | med | S | shared with #5/#17 |
| 36 | PIN pad | `ui/pin.rs` 582; `HC_PINCODE` ignored | no | med | S | `windows/pin.rs`, replace noop |
| 37 | Emotion list + command aliases | `emotion.rs` 150, `emotion_list.rs` 147 | partial | low-med | S | `windows/emotion_list.rs` |
| 38 | Cart choice | `ui/cart_choice.rs` 238 | no | low-med | S | `windows/cart_choice.rs` |
| 39 | Party booking board | `booking.rs` 234 + window 339 | no (join only) | low-med | M | `windows/booking.rs` |
| 40 | Battleground + queue | `battleground.rs` 80 + `battle_queue.rs` 327 | no | low-med | L | `windows/battle_queue.rs`, `state/battleground.rs` |
| 41 | World map (overworld) | `ui/worldmap.rs` 268 | partial (current-map minimap) | low-med | M | `windows/big_map.rs` |
| 42 | Navigation window | `navigation.rs` 1,050 + window 1,489 | no | med | L | `windows/navigation.rs` + table loader |
| 43 | Sky / RSW ambient / map effect objects | `fx/sky.rs`, `ambient.rs`, `fx/map_effects.rs` | partial (weather yes, sky TODO) | low-med | M | `loaders/map/mod.rs:136`, `world/ambient.rs` |
| 44 | Announce bar | `ui/announce.rs` 145 | no | low | S | `interface/` overlay |
| 45 | Pickup popup | `ui/pickup.rs` 242 | no | low | S | overlay |
| 46 | Level-up buttons | `ui/level_up.rs` 133 | no | low | S | overlay |
| 47 | Quest banner | `ui/quest_notice.rs` 191 | no | low | S | overlay |
| 48 | Screenshots | `screenshot.rs` 30 | no | low | S | `input/` + `graphics` readback |
| 49 | Clipboard paste | `input.rs:4122` | no | low | S | `interface` text box |
| 50 | FPS/ping meter | `ui/meter.rs` 85 | partial (debug) | low | S | `pub use` profiler |
| 51 | Kill exp / combo / extended damage | `fx/exp_shown.rs`, `ComboWindow` | no | low | S | `world/particles` |
| 52 | Clan window | `ui/clan.rs` 156 | no | low | M | `windows/clan.rs` |
| 53 | Homunculus / mercenary windows | 790 + 328 | no | low | L | `windows/companion.rs` |
| 54 | Item grade enchant | `ui/grade.rs` 875 | no | low | M | `windows/grade.rs` |
| 55 | Merge / forge / material | 442 + 374 + 181 | no | low | S-M | `windows/merge.rs` |
| 56 | Dressroom / stylist | 400 + 985 | no | low | M | `windows/stylist.rs` |
| 57 | Ranking board | `ui/ranking.rs` 194 | no | low | S | `windows/ranking.rs` |
| 58 | Storage usage log | `ui/storage_log.rs` 327 | no | low | S | `windows/storage_log.rs` |
| 59 | Taming minigame | `ui/taming.rs` 123 | no | low | S | `windows/taming.rs` |
| 60 | Special popup | `ui/special_popup.rs` 38 | no | low | S | `windows/popup.rs` |
| 61 | Attendance calendar | `ui/attendance.rs` 375 | no | out of era | — | — |
| 62 | Achievement window | `ui/achievement.rs` 480 | no | out of era | — | — |
| 63 | Enchant / reform / laphine | 1,191 + 614 + 881 | no | out of era | — | — |
| 64 | Reputation / memorial / gold PC / package | 326 + 323 + 161 + 351 | no | out of era | — | — |
| 65 | Autohunt/MCP/control/addon | `autohunt.rs` 347, `mcp.rs` 171, `control.rs`, `addon.rs` 132 | no | out of scope | — | — |

---

## 7. Top 15 actionable gaps (ordered by value/cost for a 2019 pre-re server)

1. **Guild window** — the single largest missing subsystem: 29 `ZC_*GUILD*` opcodes in our 2019 table,
   zero UI, zero handlers beyond emblem. WoE-guild management (members, positions, notice, skills,
   relations) is core. Size L, relevance high. Start with read-only info/members, then positions/notice.
2. **RoDEX mail** — `ZC_NOTIFY_NEW_MAIL` is currently a noop; mail is how servers hand out items.
   1,147-line reference; mailbox + read + claim is the 80 % slice. Size L→M if scoped to read/claim.
3. **Item/Navi links in chat and NPC text** (`links.rs`) — trade chat and quest NPCs are unreadable
   without clickable `<ITEM>`/`<NAVI>` runs. One text-layout feature unlocks chat, dialog and item info.
   Size M, high.
4. **Item comparison pane** in `item_info` — the highest-value QoL for equipment decisions, needs no
   new packets. Size M, high.
5. **Skill cooldown display** — we already receive `ZC_SKILL_POSTDELAY` and store it; nothing renders it.
   Sweep + seconds on hotbar and skill tree. Size S, high.
6. **Battle mode `/bm`** — table stakes for keyboard play; our chat line always eats letters. Size S, high.
7. **PvP/GvG/WoE targeting rules** — `map_flags` is parsed and dropped; no enemy/party/guild/camp
   policy, no damage-number suppression on siege. Size M, high.
8. **NPC text/number input** — custom rAthena scripts using `input()`/numeric prompts cannot proceed.
   Size S, high; add `DialogElementKind::Input` + 2 handlers.
9. **Hotbar 2 bars × 38 slots + F1–F9/F12/Ctrl+F12** — 10 slots is the most-complained-about limit on a
   pre-re skill list. Size M, high.
10. **Bank + cash shop** — both fully wired in 2019, neither exists. Bank is S; cash shop M. These two
    are the most-used "server economy" windows.
11. **Quest log (tabbed) + quest banner** — we have a 68-line tracker and the state, no detail view,
    no recommended list, no banner. Most rAthena 2019 quests already send the list. Size M, med-high.
12. **Character creation** — name-only today vs race/gender/hair with preview; first-run experience.
    Size M, high.
13. **Key rebinding + settings surface** — `input/keybinds.rs` already persists binds and `GameSettings`
    has one field; a settings window with the ~20 switches rustnarok ships (aura size, damage
    abbreviation, item options in name, cooldown numbers, monster HP/level, far zoom) is cheap polish.
    Size S–M, high.
14. **Refine/repair UI** — `ZC_ACK_WEAPONREFINE` (0x0221) is unhandled, so a 2019 server's refine window
    is a no-op. Size M, high (needs item-list + chance display).
15. **Whisper windows + chat tabs** — rustnarok gives each partner a window with a log, block list and
    away message; ours only pre-fills `/w`. Size M, high social value.

Equipment is close, not absent: we draw 11 data-driven slots + paper doll + stats (1,297 lines); the
missing pieces are the costume tab, equip-switch sets, show/hide equip and per-slot unequip.

Honourable mentions (cheap, visible): head marks (S), pickup popup (S), announce bar (S), screenshots
(S), minimap zoom/party dots (S–M), monster-info/Sense (S), MVP banner (S), cutin window (S).

---

## 8. Out of scope (2025 / 4th job / design)

Not recommended for a 2019 pre-renewal/transcendence server; recorded so nobody re-audits them:

- **4th-job status/effect families** in `MapEvent`: `StarSkill`, `Spirits`, `Shields`, `Charms`,
  `Souls`, `ShadowScars`, `TitleChanged`; `fx/world_fx/status4.rs`, `prims/transcend.rs`,
  `prims/souls.rs` — 4th-job only (already covered as out of scope in the 2026-11-22 effects report).
- **Post-2019 systems**: `ui/achievement.rs` (480), `ui/guild_companion.rs` (422, 2020 guild UI),
  `ui/memorial.rs` (323, memorial dungeon), `ui/package.rs` (351, `0x2731` package pick),
  `ui/reputation.rs` (326), `ui/laphine.rs` (881), `ui/enchant.rs` (1,191, 2021 enchanter),
  `ui/reform.rs` (614), `ui/goldpc.rs` (161), `ui/attendance.rs` (375) — their opcode families are
  absent from our 2019 table, except `ZC_REPUTE_INFO` and `ZC_GOLDPCCAFE_POINT` which exist but are
  unused on CondaRO (`noop_policy.rs` already classes reputation as `SystemUnused`).
- **2025 tooling**: `mcp.rs`, `control.rs`, `autohunt.rs`, `play.rs`, `addon.rs` — agent/control-socket
  surface with no server counterpart; borrowing it would mean building botting infrastructure.
- **rustnarok's renderer-bound primitives** — already rejected in the 2026-11-22 report.
- Ambiguous-era, low value: `ui/grade.rs` (2018 item grade), `ui/dressroom.rs`/`ui/stylist.rs` (2018),
  `ui/clan.rs` (2017), `ui/storage_log.rs` — include only if CondaRO enables those server systems.
  Note `grade.rs` and `dressroom.rs` **do** have 2019 opcodes (`ZC_ACK_RANDOM_UPGRADE_ITEM`,
  `ZC_DRESSROOM_OPEN`), so "out of era" is a product decision there, not a protocol one.

---

## 9. Method / limitations

- Source-only audit; no Wine, no live client, no server changes; **neither repository was modified**
  (this file lives in `/tmp/gap-audit/`).
- Relevance was decided by (a) presence of the packet family in our own 2019 opcode registry
  (`opcode_lengths_2019.rs`, 507 named opcodes) and (b) known rAthena feature era; where the registry
  and rAthena history disagree (navigation, attendance, homunculus/mercenary) the row says so.
- Line counts are `wc -l` on the cited files; they indicate feature depth, not quality.
- Counts used: 86 UI modules / 53,797 UI lines / ~641 opcodes / 361 `MapEvent` variants vs 48 window
  files (40 windows) / 16,469 interface lines / 141 map handlers / 143 `NetworkEvent` variants /
  243 typed packets / 507 2019 opcodes with 158 total registered handlers.
- Not re-audited here (per instructions): `src/fx/*` effects, status visuals/tints/auras, skill motion
  tables — see `korangar-actor-parity/docs/rustnarok-audit-2026-11-22.md`.

### Verification commands used (read-only)

```bash
# rustnarok
ls /tmp/rustnarok-client/src/ui/*.rs | wc -l                        # 86
wc -l /tmp/rustnarok-client/src/ui/*.rs | tail -1                   # 53,797
grep -oE "0x[0-9A-Fa-f]{4}" /tmp/rustnarok-client/src/net.rs | sort -u | wc -l   # 641
awk '/pub enum MapEvent/,/^}/' /tmp/rustnarok-client/src/net/map_event.rs | grep -cE "^\s{4}[A-Z]"

# ours
ls korangar/src/interface/windows/*.rs | wc -l                      # 48
grep -c "packet_handler.register(" korangar-networking/src/packet_versions/version_20220406.rs  # 158
grep -c "name: \"" ragnarok-packets/src/opcode_lengths_2019.rs      # 507
grep -rni "pvp|gvg|woe|siege|battleground" korangar/src             # 0
grep -rn "map_flags" korangar/src                                   # write-only
grep -rni "battle_mode|ChangeChatMode|/bm" korangar/src             # 0
```
