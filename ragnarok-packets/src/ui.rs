//! ui packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

/// `ZC_NOTIFY_CRITICALWEIGHT` (0x0ADE): weight percentage at which the client
/// displays the weight in red (rAthena `natural_heal_weight_rate`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0ADE)]
#[packet_size(6)]
pub struct CriticalWeightUpdatePacket {
    pub weight: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0291)]
#[packet_size(4)]
pub struct MessageTablePacket {
    pub message_id: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct AchievementData {
    pub acheivement_id: u32,
    pub is_completed: u8,
    pub objectives: [u32; 10],
    pub completion_timestamp: u32,
    pub got_rewarded: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A24)]
#[packet_size(66)]
pub struct AchievementUpdatePacket {
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    pub acheivement_data: AchievementData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A23)]
#[variable_length]
pub struct AchievementListPacket {
    #[new_derive]
    pub achievement_count: u32,
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    #[repeating(achievement_count)]
    pub acheivement_data: Vec<AchievementData>,
}

/// Sent by the map server to the client when there is a new chat message from
/// ??. Provides the message to be displayed in the chat window, as well as
/// information on how the message should be displayed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01C3)]
#[variable_length]
pub struct Broadcast2MessagePacket {
    pub font_color: ColorRGBA,
    pub font_type: u16,
    pub font_size: u16,
    pub font_alignment: u16,
    pub font_y: u16,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone uses the @broadcast
/// command. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x009A)]
#[variable_length]
pub struct BroadcastMessagePacket {
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone writes in proximity
/// chat. Provides the source player and message to be displayed in the chat
/// window and the speach bubble.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x008D)]
#[variable_length]
pub struct OverheadMessagePacket {
    pub entity_id: EntityId,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F3)]
#[variable_length]
// The trailing `NUL` of the message is not part of the declared length of the
// client-to-server layout, so the declared size can never equal the consumed
// size by construction. Known weird packet, see the P0-2 report.
#[allow_packet_size_mismatch]
pub struct GlobalMessagePacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

/// Server asks the client for a numeric value (`ZC_OPEN_EDITDLG`).
///
/// Packet layout for PACKETVER 20190605: `0142 <npc id>.L` (rAthena
/// `clif.cpp:2637`). Answered with [`InputEditDialogPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0142)]
pub struct OpenEditDialogPacket {
    pub npc_id: EntityId,
}

/// Server asks the client for a text value (`ZC_OPEN_EDITDLGSTR`).
///
/// Packet layout for PACKETVER 20190605: `01d4 <npc id>.L` (rAthena
/// `clif.cpp:2665`). Answered with [`InputEditDialogStrPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01D4)]
pub struct OpenEditDialogStrPacket {
    pub npc_id: EntityId,
}

/// Answer to `ZC_OPEN_EDITDLG` (CZ_INPUT_EDITDLG).
///
/// Packet layout for PACKETVER 20190605: `0143 <npc id>.L <value>.L`
/// (rAthena `clif.cpp:13314`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0143)]
pub struct InputEditDialogPacket {
    pub npc_id: EntityId,
    pub value: i32,
}

/// Answer to `ZC_OPEN_EDITDLGSTR` (CZ_INPUT_EDITDLGSTR).
///
/// Packet layout for PACKETVER 20190605: `01d5 <len>.W <npc id>.L <string>.?B`
/// (rAthena `clif.cpp:13333`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01D5)]
#[variable_length]
#[allow_packet_size_mismatch]
pub struct InputEditDialogStrPacket {
    pub npc_id: EntityId,
    #[length_remaining_off_by_one]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum HotkeyType {
    Item,
    Skill,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct HotkeyData {
    pub hotkey_type: HotkeyType,
    pub item_or_skill_id: u32,
    pub quantity_or_skill_level: u16,
}

impl HotkeyData {
    pub const UNBOUND: Self = Self {
        hotkey_type: HotkeyType::Item,
        item_or_skill_id: 0,
        quantity_or_skill_level: 0,
    };
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B20)]
#[packet_size(271)]
pub struct UpdateHotkeysPacket {
    pub rotate: u8,
    pub tab: HotbarTab,
    pub hotkeys: [HotkeyData; 38],
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02BA)]
pub struct SetHotkeyData1Packet {
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B21)]
pub struct SetHotkeyData2Packet {
    pub tab: HotbarTab,
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B7)]
#[variable_length]
pub struct DialogMenuPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09F9)]
pub struct QuestNotificationPacket1 {
    pub quest_id: u32,
    pub active: u8,
    pub start_time: u32,
    pub expire_time: u32,
    pub objective_count: u16,
    /// For some reason this packet always has space for three objective
    /// details, even if none are sent.
    pub objective_details: [ObjectiveDetails1; 3],
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x08FE)]
#[variable_length]
pub struct HuntingQuestNotificationPacket {
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09FA)]
#[variable_length]
pub struct HuntingQuestUpdateObjectivePacket {
    pub objective_count: u16,
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02B4)]
#[packet_size(6)]
pub struct QuestRemovedPacket {
    pub quest_id: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct Quest {
    pub quest_id: u32,
    pub active: u8,
    pub remaining_time: u32, // TODO: double check these
    pub expire_time: u32,    // TODO: double check these
    #[new_derive]
    pub objective_count: u16,
    #[repeating(objective_count)]
    pub objective_details: Vec<QuestDetails>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09F8)]
#[variable_length]
pub struct QuestListPacket {
    #[new_derive]
    pub quest_count: u32,
    #[repeating(quest_count)]
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum ImageLocation {
    BottomLeft,
    BottomMiddle,
    BottomRight,
    MiddleFloating,
    MiddleColorless,
    #[numeric_value(255)]
    ClearAll,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01B3)]
#[packet_size(67)]
pub struct DisplayImagePacket {
    #[length(64)]
    pub image_name: String,
    pub location: ImageLocation,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum QuestColor {
    Yellow,
    Orange,
    Green,
    Purple,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B4)]
#[variable_length]
pub struct NpcDialogPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub text: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0090)]
pub struct StartDialogPacket {
    pub npc_id: EntityId,
    #[new_value(1)]
    pub dialog_type: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B9)]
pub struct NextDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0146)]
pub struct CloseDialogPacket {
    pub npc_id: EntityId,
}

/// Sent by the map server when the client should play a 3d sound effect
/// (`ZC_SOUND`, 0x01D3).
///
/// The 2019 rAthena layout is 35 bytes: `name[24]`, `act` (1 byte, start/stop
/// marker), `term` (minimum interval between two plays in milliseconds) and the
/// account id of the entity that produced the sound.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01D3)]
#[packet_size(35)]
pub struct SoundPacket {
    /// File name relative to `data/wav`, e.g. `effect\\ef_blessing.wav`.
    #[length(24)]
    pub name: String,
    /// `0`/`1` mark the start/end of a looping sound; both play it once here.
    pub act: u8,
    /// Minimum interval between two plays of the same sound, in milliseconds.
    pub term: u32,
    /// Account id of the entity that produced the sound.
    pub account_id: AccountId,
}

#[cfg(test)]
mod sound_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;

    #[test]
    fn sound_packet_has_2019_layout() {
        let packet = SoundPacket {
            name: "effect\\ef_blessing.wav".to_string(),
            act: 1,
            term: 500,
            account_id: AccountId(0x1234_5678),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();

        // Header (2) + name (24) + act (1) + term (4) + account id (4).
        assert_eq!(writer.as_slice().len(), 35);
        // Header 0x01D3 little endian.
        assert_eq!(&writer.as_slice()[..2], &[0xD3, 0x01]);

        let mut reader = ByteReader::without_metadata(writer.as_slice());
        let parsed = SoundPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(parsed.name, "effect\\ef_blessing.wav");
        assert_eq!(parsed.act, 1);
        assert_eq!(parsed.term, 500);
        assert_eq!(parsed.account_id, AccountId(0x1234_5678));
    }

    #[test]
    fn sound_packet_name_is_nul_terminated_within_24_bytes() {
        let mut writer = ByteWriter::new();
        SoundPacket {
            name: "ef_hammerfall.wav".to_string(),
            act: 0,
            term: 0,
            account_id: AccountId(1),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();

        // Header, then the name, zero padded up to 24 bytes.
        let name_bytes = &writer.as_slice()[2..26];
        assert_eq!(&name_bytes[..17], b"ef_hammerfall.wav");
        assert!(name_bytes[17..].iter().all(|byte| *byte == 0));
    }
}

// ---------------------------------------------------------------------------
// Party packets for the 2019 rAthena server (PACKETVER 20190605).
//
// Before these were registered the login burst of a character that is in a
// party contained unknown fixed-size packets. The client's stream resync reads
// bytes 2..4 of an unknown packet as a packet length (only valid for variable
// length packets), so it usually fails and discards the rest of the TCP read,
// swallowing NPC spawns and the skill list that follow the party packets.
// ---------------------------------------------------------------------------

/// Sent by the client to claim the reward of a completed achievement
/// (`CZ_REQ_ACH_REWARD`, 0x0A25 at PACKETVER 20190605).
///
/// rAthena registers the handler as a fixed 6-byte packet
/// (`clif_packetdb.hpp:1769`) and reads the achievement id at byte 2
/// (`clif_parse_AchievementCheckReward`, `clif.cpp:21846-21855`):
/// `0a25 <achievement id>.L`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A25)]
#[packet_size(6)]
pub struct AchievementRewardRequestPacket {
    pub achievement_id: u32,
}

/// Server answer to [`AchievementRewardRequestPacket`]
/// (`ZC_REQ_ACH_REWARD_ACK`, 0x0A26 at PACKETVER 20190605).
///
/// rAthena registers the packet as 7 bytes (`clif_packetdb.hpp:1770`) and
/// writes `result` at byte 2 and the achievement id at byte 3
/// (`clif_achievement_reward_ack`, `clif.cpp:21860-21872`):
/// `0a26 <result>.B <achievement id>.L`. `result = 1` means the reward was
/// granted; `0` means the claim was rejected (not completed / already
/// rewarded / inter-server failure).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A26)]
#[packet_size(7)]
pub struct AchievementRewardAckPacket {
    pub result: u8,
    pub achievement_id: u32,
}

/// `ZC_READ_BOOK` (0x0294), 10 bytes: `<book id>.L <page>.L`.
///
/// rAthena writes the packet by hand in `clif_readbook`
/// (clif.cpp:18354-18361) because it has no `PACKET_*` struct. The text of the
/// book is **not** part of the packet: the client reads it from its own
/// `data\book\<book id>.txt` file, so only the id and the page are on the wire.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0294)]
#[packet_size(10)]
pub struct ReadBookPacket {
    pub book_id: u32,
    pub page: u32,
}
