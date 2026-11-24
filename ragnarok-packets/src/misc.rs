//! misc packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02D9)]
#[packet_size(10)]
pub struct UpdateConfigurationPacket {
    pub config_type: u32,
    pub value: u32, // only enabled and disabled ?
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B5)]
#[packet_size(6)]
pub struct NextButtonPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B6)]
#[packet_size(6)]
pub struct CloseButtonPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum HealType {
    #[numeric_value(5)]
    Health,
    #[numeric_value(7)]
    SpellPoints,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct HuntingObjective {
    pub quest_id: u32,
    pub mob_id: u32,
    pub total_count: u16,
    pub current_count: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ExperienceType {
    #[numeric_value(1)]
    BaseExperience,
    JobExperience,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ExperienceSource {
    Regular,
    Quest,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0ACC)]
#[packet_size(18)]
pub struct DisplayGainedExperiencePacket {
    pub account_id: AccountId,
    pub amount: u64,
    pub experience_type: ExperienceType,
    pub experience_source: ExperienceSource,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum RestartType {
    Respawn,
    Disconnect,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B1)]
#[packet_size(8)]
pub struct ParameterChangePacket {
    pub variable_id: u16,
    pub value: u32,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B2)]
pub struct RestartPacket {
    pub restart_type: RestartType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B3)]
#[packet_size(3)]
pub struct RestartResponsePacket {
    pub result: RestartResponseStatus,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x018B)]
#[packet_size(4)]
pub struct DisconnectResponsePacket {
    pub result: DisconnectResponseStatus,
}

/// Sent to the map server to request a clean disconnect (`CZ_REQ_DISCONNECT`).
///
/// Unlike the restart packet (0x00B2), which returns to the character selection
/// and keeps the account online, this makes the map server save the character
/// and tell the character server (and through it the login server) that the
/// account is offline, so it can log in again immediately.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x018A)]
pub struct RequestDisconnectPacket {
    /// Disconnect type. The server ignores the value, official clients send 0.
    pub disconnect_type: u16,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct ReputationEntry {
    pub reputation_type: u64,
    pub points: i64,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B8D)]
#[variable_length]
pub struct ReputationPacket {
    pub success: u8,
    #[repeating_remaining]
    pub entries: Vec<ReputationEntry>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct Aliance {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct Antagonist {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B7A)]
#[variable_length]
pub struct OpenMarketPacket {
    #[repeating_remaining]
    pub items: Vec<MarketItemInformation>,
}

/// `ZC_UI_OPEN` (0x0AE2): the server asks the client to open one of its
/// built-in UI windows.
///
/// At PACKETVER 20190605 the packet is 7 bytes: `type.W type.B data.L`
/// (`PACKET_ZC_UI_OPEN`, `src/map/packets_struct.hpp:2241`, `clif_ui_open` at
/// `src/map/clif.cpp:21916`). `ui_type` is `enum out_ui_type`
/// (`src/map/clif.hpp:1415`); `0` is `OUT_UI_BANK`, which the `openbank` NPC
/// script command sends (`src/map/script.cpp:27070`). Opening the bank still
/// needs the `CZ_REQ_OPEN_BANKING` request below.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AE2)]
#[packet_size(7)]
pub struct OpenUserInterfacePacket {
    /// `enum out_ui_type`: 0 bank, 1 stylist, 5 tip, 6 quest, ...
    pub ui_type: u8,
    pub data: u32,
}

/// Result of a successful Sense (`WZ_ESTIMATION`) cast
/// (`ZC_MONSTER_INFO`, 0x018C at PACKETVER 20190605).
///
/// rAthena `clif_skill_estimation` (`clif.cpp:6272-6298`) writes:
/// `018c <class>.W <level>.W <size>.W <hp>.L <def>.W <race>.W <mdef>.W
/// <element>.W <water>.B <earth>.B <fire>.B <wind>.B <poison>.B <holy>.B
/// <shadow>.B <ghost>.B <undead>.B` (29 bytes). The class is the monster
/// view/job id, the elemental values are percentage resistances.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x018C)]
#[packet_size(29)]
pub struct MonsterInfoPacket {
    pub job_id: JobId,
    pub level: u16,
    pub size: u16,
    pub health_points: u32,
    pub defense: i16,
    pub race: u16,
    pub magic_defense: i16,
    pub element: u16,
    pub water: u8,
    pub earth: u8,
    pub fire: u8,
    pub wind: u8,
    pub poison: u8,
    pub holy: u8,
    pub shadow: u8,
    pub ghost: u8,
    pub undead: u8,
}

/// Opens the refine UI (`ZC_OPEN_REFINING_UI`, 0x0AA0).
///
/// Layout (`clif_refineui_open`, `clif.cpp:22475`): `{ header }.W` only.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AA0)]
#[packet_size(2)]
pub struct OpenRefiningUiPacket {}

/// One selectable material of the refine UI (`ZC_REFINING_MATERIAL_LIST` row).
///
/// rAthena's `PACKET_ZC_REFINING_MATERIAL_LIST_SUB`
/// (`packets_struct.hpp:4165`): `{ item id }.L { chance }.b { zeny }.L` = 9
/// bytes at PACKETVER 20190605.
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct RefiningMaterialEntry {
    pub item_id: ItemId,
    /// Success chance in percent (`cost->chance / 100`, 0-100).
    pub chance: i8,
    /// Zeny cost of the refine attempt.
    pub zeny: i32,
}

/// Materials and success chances for one selected item
/// (`ZC_REFINING_MATERIAL_LIST`, 0x0AA2).
///
/// Layout (`clif_refineui_info`, `clif.cpp:22504`; struct
/// `packets_struct.hpp:4175`): `{ header }.W { length }.W { item index }.W
/// { blacksmith blessing }.b` followed by one [`RefiningMaterialEntry`] per
/// material. The item index is the `slot + 2` client index
/// (`p->itemIndex = client_index( index )`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AA2)]
#[variable_length]
pub struct RefiningMaterialListPacket {
    pub item_index: InventoryIndex,
    /// Number of Blacksmith Blessings required for the selected refine level,
    /// `0` when the item cannot use them.
    pub blacksmith_blessing: i8,
    #[repeating_remaining]
    pub materials: Vec<RefiningMaterialEntry>,
}

/// Attempt a refine in the refine UI (`CZ_REQ_REFINING`, 0x0AA3).
///
/// Layout (`clif_parse_refineui_refine`, `clif.cpp:22599`; struct
/// `packets_struct.hpp:4186`): `{ header }.W { client index }.W
/// { material item id }.L { blacksmith blessing }.b` = 9 bytes. The material
/// id is a plain item id (`p->itemId` is compared with `cost->nameid`), the
/// blessing byte is `0`/`1`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AA3)]
#[packet_size(9)]
pub struct RefiningRequestPacket {
    pub index: InventoryIndex,
    pub material_item_id: ItemId,
    pub blacksmith_blessing: i8,
}

/// Close the refine UI (`CZ_CLOSE_REFINING_UI`, 0x0AA4).
///
/// Layout (`clif_parse_refineui_close`, `clif.cpp:22493`): `{ header }.W`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AA4)]
#[packet_size(2)]
pub struct CloseRefiningUiRequestPacket {}
