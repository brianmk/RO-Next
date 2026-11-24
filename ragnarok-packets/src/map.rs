//! map packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

/// Sent by the client to the map server when the player wants to warp.
/// Attempts to warp the player to a specific position on a specific map using
/// the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0140)]
pub struct RequestWarpToMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x099B)]
#[packet_size(8)]
pub struct MapTypePacket {
    pub map_type: u16,
    pub flags: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0091)]
#[packet_size(22)]
pub struct ChangeMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum MarkerType {
    DisplayFor15Seconds,
    DisplayUntilLeave,
    RemoveMark,
}

/// One entry of the Warp Portal destination list (`ZC_WARPLIST`, 0x0ABE).
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct WarpPointEntry {
    /// Map name, padded with NUL bytes to `MAP_NAME_LENGTH_EXT` (16) bytes.
    #[length(16)]
    pub map_name: String,
}

/// Sent by the map server when the player has to choose the destination of a
/// Warp Portal (`AL_WARP`).
///
/// The client answers with [`SelectWarpPointPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0ABE)]
#[variable_length]
pub struct WarpPointListPacket {
    pub skill_id: SkillId,
    #[repeating_remaining]
    pub maps: Vec<WarpPointEntry>,
}

/// Sent by the client to the map server to answer a Warp Portal destination
/// list. `map_name` is the destination map from [`WarpPointListPacket`].
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x011B)]
#[packet_size(20)]
pub struct SelectWarpPointPacket {
    pub skill_id: SkillId,
    #[length(16)]
    pub map_name: String,
}

#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x007D)]
pub struct MapLoadedPacket {}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0192)]
#[packet_size(24)]
pub struct ChangeMapCellPacket {
    pub position: TilePosition,
    pub cell_type: u16,
    #[length(16)]
    pub map_name: String,
}

#[cfg(test)]
mod warp_point_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;

    /// `ZC_WARPLIST` (0x0ABE) carries the skill id and NUL-padded 16 byte map
    /// names; the client answers with `CZ_SELECT_WARPPOINT` (0x011B).
    #[test]
    fn warp_point_list_packet_parses_2019_layout() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0ABEu16.to_le_bytes());
        bytes.extend_from_slice(&22u16.to_le_bytes()); // 6 bytes header + one 16 byte map
        bytes.extend_from_slice(&27u16.to_le_bytes()); // AL_WARP
        bytes.extend_from_slice(b"prontera.gat\0\0\0\0");

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = WarpPointListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(packet.skill_id, SkillId(27));
        assert_eq!(packet.maps.len(), 1);
        assert_eq!(packet.maps[0].map_name.trim_end_matches('\0'), "prontera.gat");
    }

    #[test]
    fn select_warp_point_packet_has_2019_layout() {
        let packet = SelectWarpPointPacket {
            skill_id: SkillId(27),
            map_name: "prontera.gat".to_string(),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let bytes = writer.into_inner();

        // Header (2) + skill id (2) + map name (16).
        assert_eq!(bytes.len(), 20);
        assert_eq!(&bytes[..2], &[0x1B, 0x01]);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 27);
        assert_eq!(&bytes[4..16], b"prontera.gat");
        assert_eq!(&bytes[16..20], &[0, 0, 0, 0]);
    }
}
