# Game-system gap audit: official client vs RO-Next vs rustnarok

Date: 2026-11-23. Scope: everything **except** the skill-effect / animation /
status-aura pipeline (covered by `rodecomp-skill-status-aura-audit.md` and
`rodecomp-skill-drift-resolution.md`).

Two detailed audits back this summary:

* [`game-gap-audit-official-client.md`](game-gap-audit-official-client.md) —
  rodecomp `bb34313a` vs our client, per-system matrix, window map, top 20.
* [`game-gap-audit-rustnarok.md`](game-gap-audit-rustnarok.md) — rustnarok
  `20a946ae` vs our client, UI/protocol/systems with 2019-wire verification.

## Headline numbers

| metric | RO-Next | official client | rustnarok |
| --- | ---: | ---: | ---: |
| UI windows | 40 player-facing (48 modules) | 186 `*Wnd.h` across 43 UI subsystems | ~80 |
| wire structs / packet ids | 339 structs, 158 handlers + 24 noops | 1,714 ids, 1,744 lengths | ~641 opcode constants |
| map-server opcodes known | 510 lengths (2019) → 141 handled = **38 %** of the 367 S→C 2019 opcodes | 1,744 | — |
| runtime data tables | 19 `.ron` with archive override | 1,714 packet registrations | — |

## Corrections to earlier assumptions

The detailed audit showed several systems are **registered as noops** rather
than "unhandled": mail/achievement/clan/party-booking/repute/NPC image/
equippable-switch/pincode/page/slot-page. They parse cleanly and are dropped,
which is why they never appeared in a coverage report. The real gaps are the
**missing UI + state**, not the packet stream.

## Highest-impact gaps (wire present on a 2019 server)

| # | gap | evidence | size |
| --- | --- | --- | --- |
| 1 | **Cannot drop items** — no `CZ_ITEM_DROP` path; menu entry disabled with a "not implemented" tooltip | `interface/windows/context_menu.rs:323-334` | S |
| 2 | **Guild entirely absent** (members/grades/notice/positions/storage/emblem display; emblem is downloaded and never drawn) | 22 official handlers, `world/guild_emblem.rs` unused | L |
| 3 | **No NPC input/edit dialog** — `input()`/numeric scripts block forever | `dialog.rs` has no input element | M |
| 4 | **Cart inventory missing** — merchants can render the cart but not load/unload it | cart move opcodes known, no handler | M |
| 5 | **Cannot buy from another player's vending shop** | `ZC_PC_PURCHASE_ITEMLIST` known, unhandled | M |
| 6 | **Rodex/mail absent** (`NewMailStatusPacket` noop) | 12 official handlers | L |
| 7 | **Costume/shadow-gear slots missing** | `EquippableSwitchItemListPacket` noop; `paper_doll_slots.ron` lacks them | M |
| 8 | **Shield and robe layers never rendered** | fields `#[allow(dead_code)]` in `entity/mod.rs` | M |
| 9 | **Achievements absent** (list parsed, dropped) | `AchievementListPacket` noop | M |
| 10 | **Navigation/path guide absent** | no `/navi`, no route preview | L |
| 11 | **Pets/homunculus/mercenaries are not actors** and have no info UI | `EntityType` has 5 variants only | L |
| 12 | **Refine/enchant/grade/repair/identify/merge windows absent** (only the refine number is shown) | `UI/Enchant`, `UIItemRepairWnd`, … | L |
| 13 | **No chat channels/tabs** — party/whisper traffic lands in one list; guild chat does not exist | `chat.rs` 522 lines, flat log | M |
| 14 | **Monster DB / probability table absent** | `ZC_MONSTER_INFO` known, unmapped | M |
| 15 | **Sky, lightmaps, scene fog missing** — `MapSkyData` loaded then discarded | `loaders/map/mod.rs:136-137` TODO | M/L |
| 16 | **Map cell updates/properties have no effect** — breakable walls don't change collision; PvP/indoor flags stored and never read | `map_cell_overrides` written, never read | S/M |
| 17 | **No NPC image/book/notice/tip/transport windows** | `DisplayImagePacket` noop | M |
| 18 | **Cash shop absent** (only if the server enables it) | 0 hits | L |
| 19 | **No Bink movies** — needs a pre-transcoded `.ivf` side-car | `loaders/video/mod.rs` | M/L |
| 20 | **Pincode / page-slot char list / server-forced config ignored** (deliberate noops) | `noop_policy.rs` | S each |

## Rustnarok adds (actionable, 2019-wired)

Item/Navi/URL links in chat and NPC text, item comparison pane, skill-cooldown
display, battle mode `/bm`, PvP/GvG/WoE targeting rules, hotbar 2×38 with
F1–F9/F12, key-rebinding UI, ~40-switch settings surface, whisper windows,
party chat tab + exp/pickup/share setup, head marks, notification HUD
(announce/pickup/level-up/quest banner), monster Sense window, cutins,
animation/camera shake, and chat rooms. Details with line counts and wire
evidence in the rustnarok report.

## Where we are ahead

Typed packets with size contracts and a 2019 contract suite, deterministic
unknown-packet skipping, renderer/effects/theming/profiler depth, crash handler,
resource retry/reconnect, and a broad classic-gameplay core (movement, trade,
party, storage, shops, quest tracker, minimap, skill tree, settings).

## What this audit did not verify

Which of these systems CondaRO's rAthena actually enables (client surface ≠
server config), the exact quest/dialog protocol depth, and whether any of the
"missing" work exists on unmerged remote branches. Those need a live packet
trace and a server-config read.
