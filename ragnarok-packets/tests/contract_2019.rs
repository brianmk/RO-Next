//! Struct-level 2019 packet contract tests (P0-3).
//!
//! These tests do not need the networking crate: they feed the exact rAthena
//! `PACKETVER 20190605` byte layouts into the packet structs and check that the
//! struct consumes the whole packet. The companion harness in
//! `korangar-networking/src/contract_2019.rs` runs the same check for every
//! registered server packet handler.
//!
//! Regenerate the opcode registry with `./tools/gen_opcode_lengths.py`.

use ragnarok_bytes::{ByteReader, ByteWriter};
use ragnarok_packets::opcode_lengths_2019::{OPCODE_LENGTHS, OpcodeLength, PACKETVER, opcode_length_raw};
use ragnarok_packets::{PacketExt, PacketHeader};

fn le16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

fn le32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

fn push_string(bytes: &mut Vec<u8>, value: &str, width: usize) {
    let mut field = vec![0u8; width];
    let raw = value.as_bytes();
    let length = raw.len().min(width.saturating_sub(1));
    field[..length].copy_from_slice(&raw[..length]);
    bytes.extend_from_slice(&field);
}

/// Parse a packet (header included) and assert that exactly all bytes were
/// consumed.
fn parse_exact<T: ragnarok_packets::Packet>(bytes: &[u8]) {
    let mut reader = ByteReader::without_metadata(bytes);
    let packet = T::packet_from_bytes(&mut reader).expect("packet must parse");

    assert_eq!(
        reader.get_offset(),
        bytes.len(),
        "struct consumed {} of {} bytes",
        reader.get_offset(),
        bytes.len()
    );
    // Round trip keeps the wire layout stable.
    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    assert_eq!(writer.as_slice(), bytes, "round trip changed the wire bytes");
}

#[test]
fn registry_is_sorted_and_consistent() {
    assert_eq!(PACKETVER, 20190605);
    assert!(
        OPCODE_LENGTHS.len() >= 450,
        "registry unexpectedly small: {}",
        OPCODE_LENGTHS.len()
    );

    let mut previous = 0u16;

    for entry in OPCODE_LENGTHS {
        assert!(entry.header != 0, "opcode 0x0000 must not be in the registry");
        assert!(
            entry.header > previous,
            "registry is not sorted/unique at 0x{:04X}",
            entry.header
        );
        assert!(!entry.name.is_empty(), "0x{:04X} has no name", entry.header);

        match entry.length {
            OpcodeLength::Fixed(size) => assert!(size >= 2, "0x{:04X} fixed size {size} < header size", entry.header),
            OpcodeLength::Variable { base_size } => {
                assert!(base_size >= 2, "0x{:04X} variable base {base_size} < header size", entry.header)
            }
        }

        previous = entry.header;
    }

    // Lookup has to agree with the table.
    for entry in OPCODE_LENGTHS.iter().take(64) {
        assert_eq!(opcode_length_raw(entry.header), Some(entry.length));
    }

    assert_eq!(opcode_length_raw(0xFFFF), None);
}

#[test]
fn registry_matches_declared_packet_sizes() {
    use ragnarok_packets::*;

    // Only a few representative structures declare `#[packet_size(N)]`; the
    // registry value is what the 2019 server sends.
    let cases: &[(u16, Option<usize>)] = &[
        (ItemPickupAckPacket2019::HEADER.0, ItemPickupAckPacket2019::EXPECTED_SIZE),
        (UpdateHotkeysPacket::HEADER.0, UpdateHotkeysPacket::EXPECTED_SIZE),
        (AddPartyMemberPacket2019::HEADER.0, AddPartyMemberPacket2019::EXPECTED_SIZE),
        (PartyOptionPacket2019::HEADER.0, PartyOptionPacket2019::EXPECTED_SIZE),
        (
            MapServerLoginSuccessPacket::HEADER.0,
            MapServerLoginSuccessPacket::EXPECTED_SIZE,
        ),
    ];

    for (header, expected_size) in cases {
        let registry = opcode_length_raw(*header).unwrap_or_else(|| panic!("0x{header:04X} missing from registry"));
        let OpcodeLength::Fixed(registry_size) = registry else {
            panic!("0x{header:04X} should be fixed size");
        };

        assert_eq!(
            *expected_size,
            Some(registry_size),
            "0x{header:04X}: struct declares {expected_size:?} but the 2019 server sends {registry_size} bytes"
        );
    }
}

/// 0x0A37 ZC_ITEM_PICKUP_ACK (69 bytes, `uint32 card[4]` in 2019).
#[test]
fn pickup_ack_2019() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&le16(0x0A37));
    bytes.extend_from_slice(&le16(9)); // inventory index + 2
    bytes.extend_from_slice(&le16(3)); // amount
    bytes.extend_from_slice(&le32(2206)); // item id
    bytes.push(1); // identified
    bytes.push(0); // damaged
    bytes.push(5); // refinement
    bytes.extend_from_slice(&[0u8; 16]); // card: [u32; 4]
    bytes.extend_from_slice(&le32(0x0000_0100)); // equip position
    bytes.push(4); // item type
    bytes.push(0); // result
    bytes.extend_from_slice(&le32(0)); // hire expiration
    bytes.extend_from_slice(&le16(0)); // bind on equip
    bytes.extend_from_slice(&[0u8; 5 * 5]); // 5 item options
    bytes.push(0); // favorite
    bytes.extend_from_slice(&le16(44)); // item sprite number

    assert_eq!(bytes.len(), 69);
    parse_exact::<ragnarok_packets::ItemPickupAckPacket2019>(&bytes);
}

/// 0x010F ZC_SKILLINFO_LIST (4 + 37 bytes per skill entry).
#[test]
fn skill_list_2019() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&le16(0x010F));
    bytes.extend_from_slice(&le16(0)); // patched below

    for skill_id in [1u16, 5] {
        let mut entry = Vec::new();
        entry.extend_from_slice(&le16(skill_id));
        entry.extend_from_slice(&le16(0)); // inf
        entry.extend_from_slice(&le16(0)); // unknown
        entry.extend_from_slice(&le16(10)); // level
        entry.extend_from_slice(&le16(10)); // sp
        entry.extend_from_slice(&le16(9)); // range
        push_string(&mut entry, "Bash", 24);
        entry.push(1); // upgradable
        assert_eq!(entry.len(), 37);
        bytes.extend_from_slice(&entry);
    }

    let total = bytes.len() as u16;
    bytes[2..4].copy_from_slice(&total.to_le_bytes());
    parse_exact::<ragnarok_packets::UpdateSkillTreePacket>(&bytes);
}

/// 0x0201 ZC_FRIENDS_LIST: 4 + 8 bytes per row in 2019 (no 24 byte name).
#[test]
fn friend_list_2019() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&le16(0x0201));
    bytes.extend_from_slice(&le16(4 + 8 * 2));
    bytes.extend_from_slice(&le32(42));
    bytes.extend_from_slice(&le32(150042));
    bytes.extend_from_slice(&le32(43));
    bytes.extend_from_slice(&le32(150043));

    parse_exact::<ragnarok_packets::FriendListPacket2019>(&bytes);
}

/// 0x00C6 ZC_PC_PURCHASE_ITEMLIST: 4 + 13 bytes per shop entry in 2019.
#[test]
fn shop_list_2019() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&le16(0x00C6));
    bytes.extend_from_slice(&le16(0)); // patched below
    bytes.extend_from_slice(&le32(50)); // price
    bytes.extend_from_slice(&le32(45)); // discount price
    bytes.push(0); // item type
    bytes.extend_from_slice(&le32(501)); // item id

    let total = bytes.len() as u16;
    bytes[2..4].copy_from_slice(&total.to_le_bytes());
    assert_eq!(bytes.len(), 4 + 13);
    parse_exact::<ragnarok_packets::ShopItemListPacket>(&bytes);
}

/// 0x00C8 CZ_PC_PURCHASE_ITEMLIST: 4 + 6 bytes per entry (`amount` u16, `item
/// id` u32).
#[test]
fn shop_buy_request_2019() {
    let packet = ragnarok_packets::BuyItemsPacket {
        items: vec![ragnarok_packets::BuyItemInformation {
            amount: 2,
            item_id: ragnarok_packets::ItemId(501),
        }],
    };

    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    let bytes = writer.into_inner();

    assert_eq!(bytes.len(), 4 + 6);
    assert_eq!(&bytes[0..2], &le16(0x00C8));
    assert_eq!(&bytes[2..4], &le16(10));

    let mut reader = ByteReader::without_metadata(&bytes);
    let parsed = ragnarok_packets::BuyItemsPacket::packet_from_bytes(&mut reader).unwrap();
    assert_eq!(reader.get_offset(), bytes.len());
    assert_eq!(parsed.items.len(), 1);
    assert_eq!(parsed.items[0].item_id, ragnarok_packets::ItemId(501));
}

/// 0x0B20 ZC_SHORTCUT_KEY_LIST: fixed 271 bytes.
#[test]
fn hotkey_list_2019() {
    let mut bytes = vec![0u8; 271];
    bytes[0..2].copy_from_slice(&le16(0x0B20));
    parse_exact::<ragnarok_packets::UpdateHotkeysPacket>(&bytes);
}

/// 0x0AE4 ZC_ADD_MEMBER_TO_GROUP: fixed 89 bytes for PACKETVER 20190605.
#[test]
fn party_member_2019() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&le16(0x0AE4));
    bytes.extend_from_slice(&le32(2000001));
    bytes.extend_from_slice(&le32(150001));
    bytes.extend_from_slice(&le32(1)); // role
    bytes.extend_from_slice(&le16(7)); // class
    bytes.extend_from_slice(&le16(99)); // base level
    bytes.extend_from_slice(&le16(150)); // x
    bytes.extend_from_slice(&le16(150)); // y
    bytes.push(0); // state
    push_string(&mut bytes, "Party", 24);
    push_string(&mut bytes, "Player", 24);
    push_string(&mut bytes, "prontera.gat", 16);
    bytes.push(1); // pickup rule
    bytes.push(1); // share rule

    assert_eq!(bytes.len(), 89);
    parse_exact::<ragnarok_packets::AddPartyMemberPacket2019>(&bytes);
}

/// The registry must provide a length for every registered server opcode; this
/// is checked against the handler registration in korangar-networking.
#[test]
fn registry_lookup_is_cheap_and_deterministic() {
    let header = PacketHeader(0x0A37);
    assert_eq!(opcode_length_raw(header.0), Some(OpcodeLength::Fixed(69)));
    assert_eq!(opcode_length_raw(header.0), opcode_length_raw(header.0));
}
