# korangar vs Ragexe 2026-02-19 — non-skill systems gap audit

**Prepared for:** the lead (merge into repo notes). **Date:** 2026-11-23.
**Mode:** read-only. No file in either repo was modified; this report lives outside both trees.

## Sources / pins

| side | path | pin |
| --- | --- | --- |
| official decompilation | `/home/phantom/Documents/deepseek-harness/default-workspace/rodecomp` | `bb34313` (clean tree) |
| our client | `/home/phantom/Games/ragnarok/CondaRO-linux/korangar` | branch `integration/all-fixes-20261121`, HEAD `626b6107` (clean tree) |

Path shorthand used below: `K/` = korangar repo root, `O/` = rodecomp repo root
(`K/korangar/src/...` = main crate, `K/ragnarok-packets/src/lib.rs` = wire structs,
`K/korangar-networking/src/...` = handlers, `K/korangar/archive/data/client/*.ron` = runtime tables).

**Excluded (already audited elsewhere):** skill effects, skill animations/motions, status
effects/auras, skill sounds. 4th-job / 2025-only features are noted only where they are the reason
an official file exists.

## 0. Headline numbers (all counted, not estimated)

| metric | korangar | official client |
| --- | --- | --- |
| Rust source files / main-crate lines | 312 files / 106,339 lines (`K/korangar/src`) | — |
| user-facing windows | **48** files in `K/korangar/src/interface/windows/`, of which 8 are debug-only (`K/korangar/tests/ui_consistency.rs:19-28` `DEBUG_ONLY_FILES`) → **40 player-facing** | **186** `*Wnd.h` classes across `O/Source/UI/*` (43 subsystem dirs) |
| wire structs defined | **339** `pub struct/enum` in `K/ragnarok-packets/src/lib.rs` | **1,714** ids in `O/Source/Network/Packets/PacketId.h`, **1,744** length rows in `PacketRegistrations.inc` |
| packets actually handled | **158** `packet_handler.register(...)` + **24** `register_ignored_noop` = 182 registrations (`K/korangar-networking/src/packet_versions/version_20220406.rs`) | 1,744 registered lengths |
| opcodes whose length the client knows (so it can skip them) | **510** (`K/ragnarok-packets/src/opcode_lengths_2019.rs`, generated from rAthena `PACKETVER 20190605`) | 1,744 |
| runtime data tables with archive override | **19** `K/korangar/archive/data/client/*.ron` | — |
| protocol version switch | one variant, `SupportedPacketVersion::_20220406` (`K/korangar-networking/src/packet_versions/mod.rs:4-6`), 2019 layouts with `PACKETVER = 20190605` | single build |

The 24 noop registrations are the honest "parse-to-keep-the-stream-in-sync but throw away" list:
`CharacterInfoPerPage`, `CharacterSlotPage`, `CharacterBanList`, `LoginPincode`, `Packet0b18`(x2),
`NotifyActorInit`, `ExchangeUndo`, `NewMailStatus`, `AchievementUpdate`, `AchievementList`,
`EquippableSwitchItemList`, `UpdateConfiguration`, `HuntingQuestNotification`,
`HuntingQuestUpdateObjective`, `DisplayImage`, `MapServerAuthOk`, `PartyBookingJoinRequest`,
`PartyBookingJoinResult`, `Reputation`, `ClanInfo`, `ClanOnlineCount`, `OpenMarket`
(`K/korangar-networking/src/packet_versions/version_20220406.rs:110-114,468,496,681-683,915,1021,1109-1110,1134,1413,1418,1643-1644,1656-1658,1664`).

---

## 1. Matrix

Status: **yes** = usable as in the official client; **partial** = present but reduced;
**no** = absent or stub. "Evidence" cites files/symbols I actually read.

### 1. Actors & appearance

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| jobs / sprites / palettes | **yes** | `K/korangar/src/world/jobs.rs` (1,496 lines) + data table `K/korangar/archive/data/client/jobs.ron` (181 jobs: 143 mapped, 1 verified fallback, 37 documented unmapped; headgear 1,365 of 1,526 accessory ids, 161 documented missing). Layer composition `body/head/headgear/weapon` in `K/korangar/src/world/entity/mod.rs:363-402`; palettes via `body_palette_path`/`head_palette_path`; per-job `.imf` draw order (`imf` field, `Common::imf`). | none material; unmapped jobs are data-documented. |
| mounts / riding | **partial** | `MOUNT_SWAPS` (9 classic pairs) + `OUTFIT_JOBS` (wedding/Xmas/summer/hanbok/Oktoberfest) + `appearance_job()` at `K/korangar/src/world/jobs.rs:642-697`; `OPTION_RIDING 0x20`. | classic pre-renewal subset only; no 2nd-mount variants beyond the table. |
| shield & robe layers | **no** | `K/korangar/src/world/entity/mod.rs:122-135` — *"Only the weapon is rendered as its own layer so far; shield and robe are kept for the follow-up layer work"*; `shield`/`robe` fields carry `#[allow(dead_code)]` (lines 246-253) and are never read. Official: `O/Source/Contents/Sprites/RobeTopLayer.cpp`, `RobeOpacity.cpp`, `SpriteLayerDrawing.cpp`. | shield and garment never drawn; robe opacity/2nd-layer rules absent. |
| cart & falcon sprites | **yes** | `cart_paths()` `K/korangar/src/world/jobs.rs:706`; lazy `cart_animation`/`falcon_animation`, `set_cart_status` (entity/mod.rs:785-810). | — |
| pets / homunculus / mercenary / elemental | **no** | `world -w pet` = 2 hits, both in skill-icon comments; `homun`/`mercenary` = 0 hits in `K/korangar/src`. `EntityType` only `Hidden/Monster/Npc/Player/Warp` (`entity/mod.rs:114-121`). Official: `O/Source/Contents/Pet/PetStatus.cpp`, `PetTalk.cpp`; `O/Source/Contents/Actors/MercenaryAI.cpp`; `O/Source/UI/Status/UIHomunInfoWnd.h`, `UIMerInfoWnd.h`, `UIPetInfoWnd.h`, `UIPetEvolutionWnd.cpp`. | no pet/homun/merc actors or info windows; their job ids fall through `entity_type()` ranges and render as generic monsters. |
| head marks / overhead emblem | **no** | name plate = text only: `guild_name_label()` `entity/mod.rs:2090-2105` + HP fields in `Common`. Guild emblem is *downloaded and cached but never drawn*: `K/korangar/src/world/guild_emblem.rs`, creation at `K/korangar/src/lib.rs:5124-5129`; `grep -rn emblem K/korangar/src/interface K/korangar/src/renderer` = **0 hits**; `GuildEmblemCache::emblem()` has no caller. | no PNG-on-name-card emblem, no PvP/party marks, no guild-symbol above head. |
| body/health state (`opt1`/`opt2`) | **stub** | `set_body_state()`/`set_health_state()` (`entity/mod.rs:776-783, 2493-2500`) only assign fields; `grep` shows **no reader** outside the assignments (`lib.rs:5347-5359`). | freeze/stone/stun/sleep posture swap and poison/curse/blind visuals do not come from opt1/opt2 (status-effect audit covers the EFST path, not this one). |
| NPCs / monsters | **yes** | `entity_type()` (`jobs.rs:906-931`) via `jobs.ron` `entity_types` ranges + warp/hidden ids 45/111/139; monster sprite alias table `K/korangar/src/world/library/monster_sprite.rs` (232 aliases). | — |
| name/HP plates | **yes** | `Common::{name, health_points, guild_label}`, `guild_name_label()`; overhead chat `K/korangar/src/world/chat_bubble.rs` (players only — NPC/monster dialogue filtered, documented in the module header). | — |

### 2. Movement, pathing, collision, camera, warp

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| client-side A* pathing | **yes** | `K/korangar/src/world/pathing.rs` (424 lines): octile costs 10/14, `MAX_WALK_PATH_SIZE = 32`, `find_walkable_path`/`find_snipable_path`; comment states parity with the legacy client is required to avoid desync. | — |
| tile walkability / snipability | **yes** (static) | `impl Traversable for Map` at `K/korangar/src/world/map/mod.rs:851-863` over GAT `TileFlags::WALKABLE/SNIPABLE`. | no water/height-based rules beyond the GAT flags. |
| dynamic cell updates (`ZC_CHANGE_MAPCELL`) | **stub** | `K/korangar/src/lib.rs:6648-6665`: *"the override is recorded in the state and logged (collision follow-up)"*; `map_cell_overrides` (`state/mod.rs:186`) is written at `lib.rs:6657` and **never read**. | breakable walls/barricades/NPC-placed blocks do not change collision. |
| movement send/sync | **yes** | `RequestPlayerMovePacket`/`EntityMovePacket`/`EntityStopMovePacket`; `Entity::pathing` (`entity/mod.rs:85`), per-step send in `lib.rs:7867-7985`. | — |
| camera zoom / rotate | **yes** | `PlayerCamera::soft_zoom`/`soft_rotate` (`K/korangar/src/world/cameras/player.rs:55,60`), driven by `InputEvent::{ZoomCamera,RotateCamera}` (`lib.rs:6948-6949`), min/max distance clamps, smoothing (`cameras/smoothed.rs`). | pitch is a constant `CAMERA_PITCH`; no free pitch (same as official). |
| camera shake | **no** | `grep -i shake K/korangar/src` = 0 camera hits. Official `O/Source/Contents/Camera/CameraShake.cpp`, `CameraSmoothing.cpp`, `CameraQueries.cpp`. | no earthquake/skill shake (camera layer; skill visuals excluded, this is the camera effect). |
| teleport / warp / respawn | **yes** | `WarpPointListPacket`/`SelectWarpPoint` (`input/event.rs:299-300`), `WarpToMap` event (`input/event.rs:445-450`), respawn window `K/korangar/src/interface/windows/respawn.rs`, warp cursors (`interface/cursor/mod.rs:24,30`), vanish effect `world/effect/teleport_vanish.rs`. | — |

### 3. Items

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| inventory (3 tabs) | **yes** | `K/korangar/src/state/inventory.rs` (238 lines, `ItemCategory::{Consumables,Equipments,Misc}`), `interface/windows/inventory.rs`, item boxes `interface/components/item_box.rs`. | no item stat compare with equipped item; no favourite filter or sort (the star itself is drawn, `interface/components/item_box.rs:312`). |
| equipment + paper doll | **partial** | `interface/components/paper_doll.rs`, slot table `K/korangar/archive/data/client/paper_doll_slots.ron` (11 slots: head top/mid/low, armor, garment, both hands, both accessories, shoes, ammo), `--check-paper-doll` CLI. | **no costume or shadow-gear slots** even though the protocol supports them (`EquipPosition::COSTUME_*`/`SHADOW_*` at `K/ragnarok-packets/src/lib.rs:4647-4673`); `EquippableSwitchItemListPacket` is a noop (`version_20220406.rs:915`). |
| Kafra storage | **yes** | `interface/windows/storage.rs`, `state` plumbing, deposit/withdraw packets (`StorageDepositPacket`/`StorageWithdrawPacket`), capacity line, context-menu Store/Store-all. | no guild storage. |
| merchant cart (inventory) | **no** | cart is visual + counters only: `CartItemCountInfoPacket2019` → `NetworkEvent::CartInfo` → `player.cart_items` (`lib.rs:6610-6620`); no cart item list/window. `CZ_MOVE_ITEM_FROM_BODY_TO_CART`/`_CART_TO_BODY`/`_STORE_TO_CART` exist in the length registry (`K/ragnarok-packets/src/opcode_lengths_2019.rs:592-634`) but have **no handler**. Official `O/Source/UI/Items/UISelectCartWnd.h`. | merchants cannot view, load or unload the pushcart. |
| item stats / descriptions | **yes** | `K/korangar/archive/data/client/item_stats.ron`, `world/library/item_stats.rs`, `item_description.rs` (client `item_description` tables), `item_info.rs` (522 lines: 4 card slots, stat lines, description), card-name composition `world/library/card_composition.rs`. | — |
| ground drop / pickup | **partial** | pickup: 4 appear variants + disappear + ack handled (`version_20220406.rs:390-426,605-1152`), `world/ground_item.rs`. **Drop: explicit stub** — context menu entry is `disabled: true` with tooltip *"Not implemented yet: the client cannot drop items on the ground"* (`interface/windows/context_menu.rs:330-333`, `state/localization/texts.rs:364`). | no `CZ_ITEM_DROP` send path at all. |
| refine / upgrade | **display only** | refine level parsed (`EquippableItemInformation2019.refinement_level`), summarised in `interface/components/equipment_stats.rs:120-175` (`+N (n items)`). | no refining window/action. Official `O/Source/UI/Items/UIRefiningWnd.cpp`, `UIWeaponRefineWnd.cpp`; `O/Source/Contents/Items/ItemReformMgr.cpp`. |
| enchant / grade | **no** | `grep -i enchant` = 1 comment + alias rows (`rk_enchantblade`). Official `O/Source/Contents/Items/EnchantMgr.cpp`, `O/Source/UI/Enchant/UIEnchantUI.cpp`, `UIGradeEnchantWnd.cpp`, `UILapineUpgradeBoxWnd.cpp`. | no enchant/grading at all. |
| crafting (forging, potions, spell list) | **no** | `crafting/forge/alchem/potion` = 0 functional hits (only item-name strings). Official `O/Source/UI/Items/UIMakeTargetListWnd.cpp`, `UIMakeTargetProcessWnd.cpp`, `UIMakeTargetResultWnd.h`, `UISpellListWnd.h`, `UIMerchantItemLogWnd.cpp`. | no production UI; such NPCs stall unless scripted as plain menus. |
| vending (own shop) | **partial (yes)** | `interface/windows/vending.rs` (231 lines) + `state/vending.rs`, `ZC_OPENSTORE` → editor, `CZ_REQ_OPENSTORE` (`korangar-networking/src/lib.rs:1243-1252`). | one unit/row, no per-row amount (documented in the module header). |
| vending (browse/buy other player) | **no** | `ZC_PC_PURCHASE_ITEMLIST` is in the length registry (`opcode_lengths_2019.rs:280`) with **no handler**; `OpenStorePacket` is only used for the own-shop editor (`lib.rs:6565-6575`). | cannot buy from another merchant. |
| repair / identify / merge / change material / collection / package / barter | **no** | 0 hits for each. Official `O/Source/UI/Items/UIItemRepairWnd.cpp`, `UIItemIdentifyWnd.h`, `MergeItemWnd.cpp`, `UIChangeMaterialWnd.cpp`, `UICollectionSystemWnd.cpp`, `SelectPackageItemBox/*`, `O/Source/UI/Shop/UIBartermarketWnd.cpp`; `OpenMarketPacket` is a noop (`version_20220406.rs:1664`). | whole 2019 item-service surface missing. |

### 4. NPC interaction

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| NPC text/next/close/menu | **yes** | `interface/windows/dialog.rs` (586 lines; `DialogElementKind::{Text,NextButton,CloseButton,Choice,CancelButton}`), packets `NpcDialogPacket`/`StartDialog`/`NextDialog`/`CloseDialog`/`DialogMenu`/`ChooseDialogOption` all handled (`version_20220406.rs`). | no rich text tags/colour pass beyond client string handling; no "NPC portrait" (below). |
| input / edit dialogs (`input()`, numeric, string) | **no** | `grep -i 'editdialog\|OpenEditDialog\|RequestInput'` = 0 in packets and client. Official `O/Source/UI/Dialogs/UINpcEditDialogWnd.h`, `UISayDialogWnd.h`, `O/Source/UI/Edit/UIEditCtrl*.cpp`. | scripts that ask for a number/name **hang** (no widget to answer). |
| NPC image (`ZC_DISPLAY_IMAGE`) | **no** | `DisplayImagePacket` is a noop (`version_20220406.rs:1134`). Official `O/Source/UI/Dialogs/UIRPImageWnd.cpp`, `UIRPImageWnd2.cpp`. | — |
| shops buy/sell | **yes** | `interface/windows/{buy,sell,buy_cart,sell_cart}.rs`, `ShopItemListPacket`/`BuyOrSellPacket`/`BuyItems`/`SellList` handled; quantity controls, cart, zeny display (`interface/components/zeny.rs`). | no price-commission/refresh display; no NPC market (`OpenMarketPacket` noop). |
| storage via NPC | **yes** | see items table. | — |
| repair NPC | **no** | no repair window/packets. Official `O/Source/UI/Items/UIItemRepairWnd.cpp`. | rAthena repairs are usually dialog-based, so impact is low on this server. |
| quest dialogs | **partial** | uses the generic dialog; **no** quest-specific dialog (accept/complete/reward) — the client never sees a quest UI except the tracker. | — |
| book / notice / tip / transport menu | **no** | 0 hits for book window, notice window, `UITipboxWnd`, `UITransMenuWnd`. Official `O/Source/UI/Dialogs/UIBookWnd.h`, `UINoticeWnd.cpp`, `UITipboxWnd.h`, `UITransMenuWnd.cpp`. | ships/airships and in-game books/notices do not render. |

### 5. Quests, navigation, achievements, adventure guide, repute

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| quest tracker | **partial** | `state/quest_log.rs` (93 lines: `quest_id` + objectives only), `interface/windows/quest_log.rs` (68 lines). `QuestListPacket`, `QuestNotificationPacket1`, `QuestRemovedPacket` handled. | **no quest title, no description, no rewards, no quest-log window** (module header admits the server packets carry no title). `HuntingQuest*` are noops (`version_20220406.rs:1109-1110`). Official `O/Source/Contents/Quest/RenewQuestMgr.cpp`, `O/Source/UI/Quest/` (11 files). |
| navigation / path guide | **no** | `grep -i navigation` in `K/korangar/src` = 2 hits, both unrelated comments (`settings/interface.rs:252`, `world/pathing.rs:42`). Official `O/Source/Contents/Navigation/` (10 files: `NavigationGraph.cpp`, `NavigationRoute.cpp`, `NavigationDestinations.cpp`) + `O/Source/UI/Navigation/` (23 files: `UIListBoxNavi.cpp`, `UIChooseWarpWnd.h`). | no `/navi`, no route preview, no NPC/mob destination search. |
| achievements | **no** | `grep -i achievement` = 0. Structs exist (`AchievementData`, `AchievementListPacket`) but parse-and-drop (`version_20220406.rs:682-683`). Official `O/Source/Contents/Achievement/AchievementMgr.cpp`, `O/Source/UI/Achievement/UIAchWnd.cpp`. | — |
| adventure guide | **no** | 0 hits. Official `O/Source/Contents/AdventureGuide/AdventureGuideMgr.cpp`, `O/Source/UI/AdventureGuide/UIAdventureGuide.cpp`. | — |
| repute / titles | **no** | `grep -i repute` = 0; `ReputationPacket` noop (`version_20220406.rs:1656`). Official `O/Source/Contents/Repute/ReputeMgr.cpp`, `O/Source/UI/Repute/UIReputeWnd.cpp`. | — |

### 6. Social

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| chat (public/whisper) | **yes** | `interface/windows/chat.rs` (522 lines), `ChatMessage{text,color}` (`state/mod.rs:94-105`), `WhisperPacket`/`WhisperAck`/`WhisperReceived` handled, slash→atcommand translation `K/korangar/src/input/chat_command.rs` (incl. client-side `/w`,`/nc`,`/whisper`). | **one flat message list**: no public/party/guild/whisper tabs or channel routing (official `O/Source/UI/Chat/` 14 files). |
| party chat | **partial** | `PartyChatMessagePacket2019` handled, message lands in the same list. | no separate party tab, no `/p` routing UI. |
| guild chat | **no** | no guild-chat struct in `K/ragnarok-packets/src/lib.rs` (filtered list: only `GuildEmblem*`). | — |
| emotions | **partial** | `DisplayEmotionPacket`/`RequestEmotionPacket`, `Alt+1..9` via `K/korangar/archive/data/client/keybinds.ron` (`emotion_keys`), bubble/sprite animation. | no emotion picker window (official `O/Source/UI/Emotion/`, `CashEmotion/`). |
| party | **yes** | `state/party.rs` (368 lines, 12 members: name/job/base_level/map/position/HP-max/online/dead/leader), `interface/windows/party.rs` (HP bars, create/invite/leave), `PartyListPacket2019`/`AddPartyMember`/`Remove`/`Position`/`Option` handled. | no party booking/recruiting (`PartyBooking*` noops, `version_20220406.rs:1643-1644`), no mini-party, no exp-share option UI. |
| guild | **no** | guild state = entity `guild_id`/`guild_name` only (`entity/mod.rs:312-331`); `character_info.rs` shows "Guild: …". `ZC_MYGUILD_BASIC_INFO` known by length but unhandled. Official `O/Source/Contents/Guild/GuildMgr.cpp`, `O/Source/UI/Guild/UIGuildWnd.cpp`, `UIGuild_Storage_Log.cpp`, `UIEmblemWnd.cpp`, `UIAgitWnd.cpp`. | no guild window, member list, positions/grades, notice, emblem upload, guild storage, agit info. |
| clan | **no** | `ClanInfoPacket`/`ClanOnlineCountPacket` noop (`version_20220406.rs:1657-1658`). Official `O/Source/Contents/Clan/ClanInfo.cpp`, `O/Source/UI/Clan/UIClanWnd.cpp`. | — |
| friends | **yes** | `interface/windows/friend_list.rs` + `friend_request.rs`, `FriendListPacket2019`/`FriendOnlineStatus`/`FriendRequest*` handled, `state/friend_names.rs`. | no friend groups/block-list mgr (official `WhisperBlockMgr.cpp`). |
| messenger (private room) | **no** | `grep -i messenger` = 0. Official `O/Source/Contents/Messenger/ClientMessenger.cpp`, `MessengerMember.cpp`. | no `ZC_MSG` family at all. |
| mail / Rodex | **no** | `NewMailStatusPacket` noop (`version_20220406.rs:681`); no Rodex structs. Official `O/Source/Contents/Rodex/RodexMailStore.cpp`, `O/Source/UI/Rodex/UIOpenMailBoxWnd.cpp`, `UIRodexComposeWnd.cpp`, `UIRodexContentsWnd.cpp`. | no mailbox, no item attachment flow. |
| trade / exchange | **yes** | `interface/windows/trade.rs`, `state/trade.rs` (148 lines), all 2019 exchange packets handled (`ExchangeRequest/Offer/Add/Ack/Conclude/Exec/Cancel/Undo`), locked/confirmed stages. | `ExchangeUndoPacket2019` parsed but ignored (`version_20220406.rs:496` → the local state is cleared on cancel instead). |
| marriage / adoption | **no** | 0 hits (`wedding` 3 hits = the outfit body id 22). Official: `find O/Source -iname '*marri*' -o -iname '*adopt*'` → **no files**; marriage is dialog/couple-status only in this build. | no client UI to build; nothing in korangar either (couple status would surface via the same NPC dialogs). |

### 7. UI (window map, grouped)

korangar has 48 window modules, 40 player-facing. Full mapping in **Appendix A**. Grouped summary:

- **Have a counterpart:** login/server select, character select/create/delete, chat, NPC dialog, inventory, equipment/paper doll, storage, item info, buy/sell + carts, vending, trade, party, friend list, hotbar, skill tree, skill description, stats, quest tracker, minimap, big map, respawn, settings (game/graphics/audio/interface), menu, error, context menu, buff bar, spirit spheres, character overview.
- **Notable official windows with no counterpart:** CashShop (+Mileage, PayConfirm), Rodex (3), Clan, Guild (5), Achievement, AdventureGuide, Repute, Navigation (23 files), Enchant, Refining/WeaponRefine, ItemRepair/Identify/Merge/ChangeMaterial/Collection/Package, BarterMarket/Bank/SearchStore, Monster DB (`UIMonsterInfoWnd.h`), ProbabilityTable, Macro/Captcha, Party booking/seek (9+), MiniParty, Replay (6), EntryQueue, Events (Roulette/Attendance), StylingShop, CashEmotion, Emotion picker, Book, Notice/Tip, TransMenu, JobList, Homun/Merc/Pet info, SkillDelayInfo, Cooldown clock, Guild-storage log, `UIItemMoveInfo`, `UISwapEquipmentWnd`, `UIPreviewEquipWnd`.
- **korangar-only:** debug windows (maps warp list, packet inspector, profiler, frame/theme inspector, render options, cache, GM `commands.rs`) and the minimap/big-map `M` cycle.

### 8. World / map

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| GND/RSW/RSM rendering, models, animation | **yes** | `K/korangar/src/loaders/map/mod.rs` (439 lines), `world/map/mod.rs` (863 lines), `ragnarok-formats` parsers, model/animation loaders, water plane `loaders/map/water_plane.rs`, wave pass `graphics/passes/forward/wave.rs`. | GND >= 2.6 water planes not read (`loaders/map/mod.rs:161` TODO). |
| water | **yes** | `water_plane.rs`, `WaterVertex`/`WaterInstruction`, `water_opacity` uniform. | no reflection/refraction. |
| weather | **yes** | `world/effect/weather.rs` — 6 EF ids (snow 162, sakura 163, leaves 333, clouds 233/516, fog 515) driven by `ZC_NOTIFY_EFFECT2`, wired at `lib.rs:6507`. | fog is a particle sheet, not a scene fog (below). |
| sky | **no** | `loaders/map/mod.rs:136` `// TODO: NHA Implement sky rendering`; `MapSkyData` is loaded from `mapskydata.lub` but assigned to `_map_sky_data` and dropped (line 137). Official `O/Source/Contents/World/SkyBox.cpp`, `O/Source/Contents/Map/MapSkyMgr.cpp`. | no skybox/sky colour per map. |
| lighting | **partial** | `world/map/lighting.rs` (37 lines) from RSW `LightSettings` (ambient+diffuse+latitude/longitude) + RSW point light sources with kd-tree (`loaders/map/mod.rs:299-312`) + SDSM/PSSM shadow cameras. | **no lightmaps** (`grep lightmap` = 0; official `O/Source/Contents/Ground/GroundLightmaps.cpp`, `GroundLighting.cpp`); no scene fog. |
| minimap / big map / markers | **yes** | minimap generated from GAT at load (`loaders/map/minimap.rs`: walkable darkening, water tint), `state/minimap.rs`, `interface/windows/{minimap,big_map}.rs`, `ZC_COMPASS` markers (`MarkMinimapPositionPacket`), player arrow + map-window cycle. | big map is a scaled minimap image (no labels/links/NPC list). |
| ground items | **yes** | `world/ground_item.rs` (128 lines), 4 appear variants, pickup ack, fade-out. | no drop (see items). |
| map events / properties | **stub** | `MapTypePacket` → `client_state.map_type()/map_flags()` written at `lib.rs:6606-6609` and **never read**; `MapCellChanged` stored and unused (see §2). | indoor/PvP/no-memo/no-weather flags have no effect. |
| map-placed effect sources (RSW) | **no** | `effect_sources` are only debug picker markers (`renderer/game_interface.rs:438-441`); no runtime playback. | torches/fountain ambience effects missing. |
| map-placed sound sources (RSW) | **yes** | `Map::set_ambient_sound_sources` wired after every map load (`lib.rs:1541,8443,8494`), `AudioEngine::add_ambient_sound`. | — |

### 9. Sound / music / movies

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| BGM per map | **yes** | `AudioEngine::get_track_for_map` (`K/korangar-audio/src/lib.rs:345`), loose `BGM/`, `bgm/`, `data/mp3` folders mounted (`loaders/gamefile/mod.rs:31-37,149`). | no jukebox/`/bgm` command. |
| ambient / spatial / UI SFX | **yes** | spatial listener + range (`lib.rs:433-460`), `set_ambient_sound`, UI sound table `K/korangar/archive/data/client/ui_audio.ron`, skill sounds (audited elsewhere). | — |
| movies / cutscenes | **partial** | `loaders/video/mod.rs:13` accepts `.bik` paths but loads a **side-car** `data\video\<name>.bik.ivf` (AV1, blake3-suffixed) via `korangar-video` (rav1d); missing file → warn_once + placeholder. Official `O/Source/Contents/Movies/BinkMovie.cpp`, `BinkMovies.cpp`, `O/Source/UI/Dialogs/UIVideoWnd.cpp`. | **no Bink decoder**; un-converted `.bik` (login movie, cutscenes) cannot play. |

### 10. Networking / protocol

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| packet surface | **partial (~11% of ids)** | 339 structs, 158 real handlers, 24 noops, 510 known opcode lengths (see §0). | see family list below. |
| missing families (server-sent) | — | none of these have a struct: guild (`ZC_MYGUILD_BASIC_INFO`, `ZC_GUILD_SKILLINFO`, `CZ_REQ_JOIN_GUILD`, … all present only as *lengths*), clan, mail/Rodex, achievement (struct exists, noop), repute, adventure guide (`ZC_ADVENTURE_...` n/a), navigation, cash shop (`ZC_PC_CASH_POINT_ITEMLIST`, `ZC_ACK_SE_CASH_ITEM_LIST2`), entry queue, party booking, NPC market/barter, monster info (`ZC_MONSTER_INFO`), probability/macro, rumi/roulette/attendance, replay, item repair/refine/enchant, cart moves, `ZC_PC_PURCHASE_ITEMLIST`, NPC images, pincode. | — |
| security / hash | **no** | no executable digest: `grep -i 'digest\|blake3\|md5\|sha1' K/korangar-networking` = 0 (blake3 is only used for the IVF trailer, `loaders/video/mod.rs:51`). Official `O/Source/Network/Packets/ExecutableDigestPacket.cpp` (MD5 of exe → char server), `O/Source/Security/FileSignatureVerifier.cpp`, `NProtectGameGuardMgr.cpp`, `ThemidaMgr.cpp`. | rAthena does not verify the digest, so this is a protocol difference, not a blocker. No packet encryption in either side (official source has none — `grep -i encrypt O/Source/Network` = 0 functional hits). |
| replay | **no** | `grep -i replay` = 0. Official `O/Source/Client/Replay/`, `O/Source/UI/Replay/` (6 files). | no record/playback. |
| login / char select | **yes** | `interface/windows/{login,server_selection,character_selection,character_creation,character_deletion}.rs`, `state/character_slots.rs`, char-server login success/slot/ban packets (`CharacterBanList` noop), reconnect/backoff `state/lifecycle/session.rs` (2 s × 30). | **no pincode UI** — explicitly documented: `noop_policy.rs:68` *"server has pincode_enabled=no and the client has no pincode UI"*; page/slot-page packets noop (`version_20220406.rs:110-111`). |
| hotkeys / config | **yes / partial** | hotkeys read+write: `ZC_SHORTCUT_KEY_LIST` → `state/hotbar.rs` (4 tabs × 38, first 10 mirrored), writes via `SetHotkeyData2Packet` (`state/hotbar.rs:168,223`). Local settings persist as `client/*.ron` (`settings/*.rs` FILE_NAME consts). | `UpdateConfigurationPacket` (server-forced config) is a noop (`version_20220406.rs:1021`); no server-linked user data (official `O/Source/Contents/UserData/*` load/save); no in-game keybind remap UI (keybinds are a data file, `input/keybinds.rs`). |
| cash shop | **no** | 0 hits; `O/Source/UI/CashShop/UICashShopWnd.cpp`. | — |
| entry queue | **no** | 0 hits. Official `O/Source/Contents/EntryQueue/`, `O/Source/UI/EntryQueue/UIEntryQueueWnd.cpp`. | irrelevant for a direct-connect rAthena server. |
| protocol versioning | **single** | `SupportedPacketVersion::_20220406` only (`packet_versions/mod.rs:4-6`); layouts are 2019 (`PACKETVER 20190605`). | no runtime version negotiation; a different server PACKETVER needs code changes. |

### 11. Robustness

| system | status | evidence | notable gaps |
| --- | --- | --- | --- |
| crash handling | **yes** | `K/korangar/src/crash.rs` (131 lines): panic hook with full backtrace, SIGSEGV/ABRT/BUS/ILL/FPE handlers with glibc backtrace then re-raise (core dump preserved), re-entrancy guard. | no minidump/upload; no in-game crash report window. |
| error windows | **yes** | `interface/windows/error.rs`, opened for map-server auth failure (`lib.rs:6623-6632`), duplicate-handler errors (`ragnarok-packets/src/handler.rs` `DuplicateHandlerError`), `ErrorWindow::new` usages across login. | errors are English-only strings from the protocol code. |
| resource retry / reconnect | **yes** | `state/lifecycle/resource.rs` (`ResourcePolicy`, `ResourceRetryQueue`, deadline + bounded retries + give-up), `state/lifecycle/session.rs` (map-entry deadlines, map-load retries, char-server reconnect 2 s × 30, clean-shutdown timer). | no retry for video/BGM side-cars (warn once + placeholder). |

---

## 2. Top 20 gaps by player impact

Sizes: **S** ≤ 1 day, **M** 2-5 days, **L** > 1 week (single dev, incl. tests).

| # | gap | official source | closest korangar place | size |
| --- | --- | --- | --- | --- |
| 1 | **Cannot drop items.** No `CZ_ITEM_DROP` send path; menu entry disabled with a "not implemented" tooltip. Every player needs this on day one. | `O/Source/Contents/Actors/Item.cpp`, `O/Source/Network/Packets/PacketRegistrations.inc` (`CZ_ITEM_DROP`), `O/Source/UI/Items/UIItemMoveInfo.cpp` (drop confirm) | enable the button + event in `K/korangar/src/interface/windows/context_menu.rs:323-334`; add `ItemDropPacket` to `K/ragnarok-packets/src/lib.rs`; route in `K/korangar-networking/src/packet_versions/version_20220406.rs`; handle `NetworkEvent` in `K/korangar/src/lib.rs` | S |
| 2 | **Guild system entirely absent** (window, members, grades, notice, positions, guild storage, emblem display). Emblem is even downloaded and then unused. | `O/Source/Contents/Guild/GuildMgr.cpp`, `GuildInfo.cpp`, `O/Source/UI/Guild/UIGuildWnd.cpp`, `UIGuild_Storage_Log.cpp`, `UIEmblemWnd.cpp`, `UIAgitWnd.cpp` | new `K/korangar/src/state/guild.rs` + `interface/windows/guild.rs`; structs for `ZC_MYGUILD_BASIC_INFO`/member list in `ragnarok-packets`; draw `GuildEmblemCache` texture (`K/korangar/src/world/guild_emblem.rs`) in the name card | L |
| 3 | **No NPC input/edit dialog.** Scripts using `input()`/numeric/string entry block the dialog forever. | `O/Source/UI/Dialogs/UINpcEditDialogWnd.h`, `UISayDialogWnd.h`, `O/Source/UI/Edit/UIEditCtrl*.cpp`, `O/Source/Network/Packets/PacketRegistrations.inc` (`CZ_..._EDITDLG`) | extend `interface/windows/dialog.rs` with an input element (reuse `interface/windows/character_deletion.rs` textbox handling); add the request packet to `ragnarok-packets`; register the `ZC_OPEN_EDITDLG`/`ZC_EDITDLG` handlers | M |
| 4 | **Cart inventory missing** — merchants can't see/load/unload the pushcart although the sprite renders and the counts arrive. | `O/Source/UI/Items/UISelectCartWnd.h`, `O/Source/Contents/Items/ClientItemLists.cpp` (cart inventory type) | add a cart tab/window next to `interface/windows/storage.rs`; handle `CZ_MOVE_ITEM_FROM_BODY_TO_CART`/`_CART_TO_BODY`/`_STORE_TO_CART` (lengths already in `opcode_lengths_2019.rs:592-634`); extend `state/inventory.rs` | M |
| 5 | **Cannot buy from another player's vending shop.** `ZC_PC_PURCHASE_ITEMLIST` known but unhandled. | `O/Source/UI/Shop/UIMerchantItemPurchaseWnd.h`, `UIMerchantShopTitle.cpp`, `O/Source/Network/Packets/PacketRegistrations.inc` (`CZ_PC_PURCHASE_ITEMLIST_FROMMC`) | new vendor-browse window next to `interface/windows/buy.rs`; distinguish `OpenStorePacket` (own shop) from the purchase list; reuse `ShopItem` plumbing | M |
| 6 | **Rodex / mail absent** (`NewMailStatusPacket` noop, no mailbox/compose/attachment flow). | `O/Source/Contents/Rodex/RodexMailStore.cpp`, `O/Source/UI/Rodex/UIOpenMailBoxWnd.cpp`, `UIRodexComposeWnd.cpp`, `UIRodexContentsWnd.cpp` | new `state/mail.rs` + `interface/windows/rodex*.rs`; ~12 packet structs; reuse the inventory/attachment UX from `interface/windows/trade.rs` | L |
| 7 | **Costume / shadow-gear equipment slots missing.** `EquippableSwitchItemListPacket` is parsed and dropped, so costume items can be equipped by the server but are invisible in the UI. | `O/Source/UI/Items/UISecondCostumeWnd.cpp`, `UISwapEquipmentWnd.cpp`, `O/Source/Contents/Items/ClientItemLists.cpp` | extend `K/korangar/archive/data/client/paper_doll_slots.ron` + `interface/components/paper_doll_slots.rs` (EquipPosition already has the bits) and stop noop-ing `EquippableSwitchItemListPacket` (`version_20220406.rs:915`) | M |
| 8 | **Shield and robe not rendered.** Fields exist with `#[allow(dead_code)]`. | `O/Source/Contents/Sprites/RobeTopLayer.cpp`, `RobeOpacity.cpp`, `SpriteLayerDrawing.cpp` | add `EntityPartFile`s for shield/robe in `K/korangar/src/world/entity/mod.rs:363-402`, robe palette path in `world/jobs.rs`, sprite-layer order in `renderer` | M |
| 9 | **Achievement system absent** (list parsed but discarded). | `O/Source/Contents/Achievement/AchievementMgr.cpp`, `O/Source/UI/Achievement/UIAchWnd.cpp` | `state/achievement.rs` reusing `AchievementData` (`ragnarok-packets`), window next to `interface/windows/quest_log.rs`; drop the noop at `version_20220406.rs:682-683` | M |
| 10 | **Navigation / path guide absent** (no `/navi`, no destination search, no warp chooser). | `O/Source/Contents/Navigation/NavigationGraph.cpp`, `NavigationRoute.cpp`, `NavigationDestinations.cpp`, `O/Source/UI/Navigation/UIListBoxNavi.cpp`, `UIChooseWarpWnd.h` | new `state/navigation.rs`; route drawing on the minimap component (`interface/components/minimap.rs`); reuse `world/pathing.rs` for the local part | L |
| 11 | **Pets, homunculus and mercenaries are not actors and have no info UI.** | `O/Source/Contents/Pet/PetStatus.cpp`, `PetTalk.cpp`, `O/Source/Contents/Actors/MercenaryAI.cpp`, `O/Source/UI/Status/UIHomunInfoWnd.h`, `UIMerInfoWnd.h`, `UIPetInfoWnd.h` | extend `EntityType` (`entity/mod.rs:114-121`) + `entity_type()` in `world/jobs.rs`; add job ranges to `jobs.ron`; info windows beside `interface/windows/stats.rs` | L |
| 12 | **No refine / enchant / grade / repair / identify windows.** Only the refine *number* is displayed. | `O/Source/UI/Items/UIRefiningWnd.cpp`, `UIWeaponRefineWnd.cpp`, `UIItemRepairWnd.cpp`, `UIItemIdentifyWnd.h`, `O/Source/UI/Enchant/UIEnchantUI.cpp`, `UIGradeEnchantWnd.cpp`, `O/Source/Contents/Items/EnchantMgr.cpp` | one generic "item service" window pattern next to `interface/windows/item_info.rs`; per-service packet structs | L |
| 13 | **No chat channels/tabs** — party and whisper traffic is dumped into one list; guild chat does not exist. | `O/Source/UI/Chat/` (14 files, `UIChatWnd`, channel tabs), `O/Source/Contents/Messenger/WhisperBlockMgr.cpp` | `interface/windows/chat.rs` + `ChatMessage` gains a channel enum (`state/mod.rs:94-105`); guild-chat struct + handler | M |
| 14 | **No monster DB / probability table** (`ZC_MONSTER_INFO` known, unmapped; no drop-rate view). | `O/Source/UI/Monster/UIMonsterInfoWnd.h`, `O/Source/UI/ProbabilityTable/UIProbabilityTable.cpp` | windows beside `interface/windows/item_info.rs`; structs for `ZC_MONSTER_INFO` (length already known) | M |
| 15 | **Sky, lightmaps and scene fog missing** — indoor/outdoor maps look flat; `MapSkyData` is loaded then thrown away. | `O/Source/Contents/World/SkyBox.cpp`, `O/Source/Contents/Map/MapSkyMgr.cpp`, `O/Source/Contents/Ground/GroundLightmaps.cpp`, `GroundLighting.cpp` | `loaders/map/mod.rs:136-137` (sky), `world/map/lighting.rs` (lightmap texture), new fog in `graphics/passes/forward` | M/L |
| 16 | **Map cell updates and map properties have no effect** — breakable walls/barricades do not change collision; indoor/PvP flags are stored and never read. | `O/Source/Contents/Ground/GroundPicking.cpp`, `O/Source/Contents/Map/MapPropertyMgr.cpp`, `MapInfoMgr.cpp` | make `Map` mutable or keep an override grid that `Traversable for Map` consults (`world/map/mod.rs:851`); consume `map_type`/`map_flags` in rendering/weather | S/M |
| 17 | **No NPC image, book, notice, tip or transport menu** — several scripted events render nothing. | `O/Source/UI/Dialogs/UIRPImageWnd.cpp`, `UIBookWnd.h`, `UINoticeWnd.cpp`, `UITipboxWnd.h`, `UITransMenuWnd.cpp` | extend `interface/windows/dialog.rs` with image/book elements; stop noop-ing `DisplayImagePacket` (`version_20220406.rs:1134`) | M |
| 18 | **Cash shop / cash points absent** (only relevant if CondaRO enables it). | `O/Source/UI/CashShop/UICashShopWnd.cpp`, `UIMileageWnd.cpp`, `UIPayConfirmWnd.h` | new `state/cash_shop.rs` + window; char-server cash packets | L |
| 19 | **No movies (Bink).** `.bik` requires a pre-transcoded `.ivf` side-car; login movie/cutscenes are skipped with a placeholder. | `O/Source/Contents/Movies/BinkMovie.cpp`, `BinkMovies.cpp`, `O/Source/UI/Dialogs/UIVideoWnd.cpp` | `K/korangar/src/loaders/video/mod.rs` (decoder) or ship a transcode step in `tools/` | M/L |
| 20 | **Pincode, page/slot-page character list, server-forced config are ignored** (deliberate noops; each becomes a blocker on a differently configured server). | `O/Source/UI/Login/`, `O/Source/Network/Packets/PacketRegistrations.inc` (`CZ_..._PINCODE`), `O/Source/Contents/Configuration/ConfigMgr.cpp` | `interface/windows/character_selection.rs` (pincode prompt), `state/lifecycle/session.rs` (config apply); remove `noop_policy.rs:68` rationale when done | S each |

Honourable mentions (impact real but smaller): camera shake, RSW effect sources, emotion picker,
party booking/mini-party, monster/`UINoticeMessageBox` box variants, `UIMakeTarget*` crafting,
`UIBartermarketWnd`/`UIBank_NewWnd`, `UIItemReformWnd`/`UILapineUpgradeBoxWnd`, jukebox/`/bgm`,
`UICaptcha*` macro windows, `UIJobListWnd`, `UISkillDelayInfo`, `UIStash`-style NPC market.

---

## 3. Not verified

- **Which of the above CondaRO's rAthena actually sends.** I did not read the server tree or its
  `conf/`; features like cash shop, achievement, clan, rodex, party booking and navigation may be
  disabled server-side, which would lower their real impact. The matrix describes the *client*
  surface, not the live server config.
- **The 1,714 vs 1,744 official counts** are enum members / registration rows in the decompilation,
  and include ids this 2026 build needs for content a 2019 rAthena never sends. The honest
  comparison is "families", not raw ids; the family list in §10 is what I checked by name.
- **`OpenStorePacket` semantics for third-party shops** — I confirmed the client only uses it for
  the own-shop editor and that no purchase-list handler exists, but I did not trace whether
  rAthena's 2019 `clif` uses a different opcode for browsing vendor shops (the purchase-list
  opcode is in the registry at `opcode_lengths_2019.rs:280`, which is why I report the gap).
- **Whether shield/robe rendering is in flight on a branch**: all local branches are merged into
  `integration/all-fixes-20261121` (no commits ahead), but I did not search remote branches.
- **Marriage/adoption**: no official client source files exist in this decompilation, so I cannot
  state what the official UI looks like; I only state korangar has nothing.
- **Refine/repair real-server relevance**: on rAthena these are usually NPC-script dialogs; the
  official *windows* may only be exercised by servers that implement the newer packet flow. I did
  not determine which one CondaRO uses.
- **Line numbers** are valid for `626b6107`; HEAD moves (the repo merges several branches per day).

---

## Appendix A — window map (korangar ↔ official UI dirs)

`K/korangar/src/interface/windows/` (48 files, 40 player-facing; 8 debug-only per
`K/korangar/tests/ui_consistency.rs:19-28`).

| official `O/Source/UI/*` | korangar window(s) | note |
| --- | --- | --- |
| `Login` | `login.rs`, `server_selection.rs`, `character_selection.rs`, `character_creation.rs`, `character_deletion.rs` | pincode/page/slot-page missing |
| `Chat` | `chat.rs` | single channel, no tabs |
| `Dialogs` | `dialog.rs`, `respawn.rs`, `error.rs` | no input/say/book/image/notice/tip/transmenu |
| `Items` | `inventory.rs`, `equipment.rs`, `storage.rs`, `item_info.rs`, `sell*.rs`, `buy*.rs`, `vending.rs` | no cart/refine/repair/identify/reform/collection/package |
| `Shop` | `buy.rs`, `sell.rs`, `buy_cart.rs`, `sell_cart.rs`, `vending.rs` | no barter/bank/search-store/merchant log |
| `Exchange` | `trade.rs` | complete enough |
| `Party` | `party.rs` (window + invite prompt) | no booking/seek/mini-party |
| `Guild` | — | none |
| `Clan` | — | none |
| `Achievement` | — | none |
| `AdventureGuide` | — | none |
| `Repute` | — | none |
| `Quest` | `quest_log.rs` | tracker only |
| `Navigation` | — | none |
| `Rodex` | — | none |
| `CashShop` | — | none |
| `CashEmotion` | — | none |
| `Emotion` | — (hotkeys only, `input/keybinds.rs`) | no picker |
| `Enchant` | — | none |
| `Macro` | — | none |
| `Monster` | — | none |
| `ProbabilityTable` | — | none |
| `Replay` | — | none |
| `EntryQueue` | — | none |
| `Events` | — | no roulette/attendance |
| `StylingShop` | — | none |
| `Cooldowns` | `buff_bar.rs`, `state/skill_cooldowns.rs` | no cooldown-clock widget |
| `SkillDelayInfo` | `skill_description.rs` (partial) | no dedicated ACD info |
| `Skill` | `skill_tree/`, `skill_description.rs`, `hotbar.rs` | good |
| `Status` | `stats.rs`, `character_overview.rs`, `character_info.rs` | no job list / homun / merc / pet |
| `GameSettings` | `game_settings.rs`, `graphics_settings.rs`, `audio_settings.rs`, `interface_settings.rs` | keybind editor missing |
| `Menu` | `menu.rs` | — |
| `Shortcuts` | `hotbar.rs` | binds only via `keybinds.ron` |
| `SelectPackageItemBox` | — | none |
| `Acceptance`, `Buttons`, `Edit`, `Gauges`, `Layout`, `Lists`, `Text`, `Window` | framework / n/a | `Gauges` partly via `interface/components/progress_bar.rs` |
| — | `minimap.rs`, `big_map.rs`, `maps.rs`(debug), `commands.rs`(debug), `packet_inspector.rs`(debug), `profiler.rs`(debug), `frame_inspector.rs`(debug), `theme_inspector.rs`(debug), `render_options.rs`(debug), `cache.rs`(debug), `context_menu.rs`, `friend_list.rs`, `friend_request.rs`, `spirit_spheres.rs`, `skill_tree` | korangar-only |

## Appendix B — exact packet numbers (reproducible)

```
# korangar
grep -c 'packet_handler.register(' korangar-networking/src/packet_versions/version_20220406.rs   # 158
grep -c 'register_ignored_noop'       .../version_20220406.rs                                    # 24
grep -cE '^\s+header: 0x[0-9A-Fa-f]{4},' ragnarok-packets/src/opcode_lengths_2019.rs             # 510
grep -oP '^pub (struct|enum) \K\w+' ragnarok-packets/src/lib.rs | wc -l                           # 339
ls korangar/src/interface/windows/*.rs | wc -l                                                    # 48

# official
grep -c '^  ID_' Source/Network/Packets/PacketId.h                                                # 1714
wc -l Source/Network/Packets/PacketRegistrations.inc                                              # 1744
ls Source/UI/*/*Wnd.h Source/UI/*/*Wnd2.h | wc -l                                                 # 186
ls Source/UI | wc -l                                                                              # 43 dirs
```
