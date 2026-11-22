//! Offline contract tests against the rAthena `PACKETVER 20190605` wire format.
//!
//! This module is generic: it iterates **every** registered server-to-client
//! packet handler and checks that
//!
//! 1. the opcode has a committed entry in the 2019 opcode length registry
//!    (`ragnarok_packets::opcode_lengths_2019`) or an explicit, documented
//!    entry in [`IGNORED_OPCODES`],
//! 2. a buffer of exactly the registered 2019 wire size parses successfully and
//!    is consumed completely (so no layout drift can leave the stream in the
//!    middle of a packet),
//! 3. the struct's own metadata (`IS_VARIABLE_LENGTH`, `EXPECTED_SIZE`) agrees
//!    with the registry,
//! 4. the known critical 2019 layouts have byte-exact hand-written fixtures
//!    (pickup, party, skills, shop, friends, hotkeys, ...).
//!
//! The deterministic unknown-packet skip (`unknown_packet_action`) is unit
//! tested at the bottom of this module.

use ragnarok_bytes::ByteReader;
use ragnarok_packets::PacketExt;
use ragnarok_packets::handler::{HandlerResult, NoPacketCallback, PacketHandler};
use ragnarok_packets::opcode_lengths_2019::{OpcodeLength, opcode_length};
use ragnarok_packets::validation::{ValidationMode, set_mode};

use crate::event::NetworkEventList;
use crate::{NetworkingSystem, SupportedPacketVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Server {
    Login,
    Character,
    Map,
}

fn handler(server: Server) -> PacketHandler<NetworkEventList, NoPacketCallback> {
    match server {
        Server::Login => NetworkingSystem::create_login_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap(),
        Server::Character => {
            NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap()
        }
        Server::Map => NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap(),
    }
}

const SERVERS: [(Server, &str); 3] = [(Server::Login, "login"), (Server::Character, "character"), (Server::Map, "map")];

/// Registered opcodes that cannot be checked by the generic 2019 harness.
///
/// Every entry has to carry a reason, and every entry has to still be
/// registered (stale entries fail
/// [`ignored_opcodes_are_registered_and_reasoned`]).
///
/// Categories:
/// * `dead 2022 registration` - a 2022-only opcode that the 2019 server never
///   sends. It is still registered (the client advertises 20220406) but there
///   is no 2019 wire layout to check against.
/// * `not derivable` - a 2019 opcode whose layout is written by hand in the
///   server with no header struct. These are documented in the P0 report.
const IGNORED_OPCODES: &[(u16, &str)] = &[
    (
        0x0086,
        "dead registration: 2019 sends ZC_NOTIFY_MOVE as 0x09FD (enum unit_walkingType); 0x0086 is used by pre-2015 PACKETVERs",
    ),
    (
        0x0283,
        "not derivable: ZC_AUTH_OK_GID is written by hand in clif_parse_AuthOk (packet 0x0283, 6 bytes); the client only parses it for \
         framing",
    ),
    (
        0x0ADE,
        "not derivable: ZC_NOTIFY_CRITICALWEIGHT is written by hand in clif_weight_limit (packet 0x0ADE, 6 bytes, u32 percentage)",
    ),
    (
        0x008A,
        "dead registration: 2019 sends ZC_NOTIFY_ACT as 0x08C8 (enum damageType); 0x008A is the pre-2012 layout",
    ),
    (
        0x009E,
        "dead registration: 2019 sends ZC_ITEM_FALL_ENTRY5 as 0x0ADD (enum dropflooritemType); 0x009E is the pre-2017 layout",
    ),
    (
        0x02C1,
        "not sent by the 2019 server: colored entity messages use 0x09CD/0x017F; 0x02C1 is registered from the 2022 packet set",
    ),
    (
        0x084B,
        "dead registration: 2019 sends ZC_ITEM_FALL_ENTRY5 as 0x0ADD; 0x084B is the 2021+ layout",
    ),
    (
        0x09F9,
        "not derivable: ZC_ADD_QUEST has no length field and appends variable objective rows; the client struct expects a fixed 3-row \
         tail (known drift, documented in the P0 report)",
    ),
    (
        0x0B39,
        "dead 2022 registration: 2019 sends the equipment item list as 0x0B0A (enum inventorylistequipType); 0x0B39 is the 2022 \
         68-byte-entry layout",
    ),
    (
        0x0B41,
        "dead 2022 registration: 2019 sends ZC_ITEM_PICKUP_ACK as 0x0A37; 0x0B41 is the 2022 layout",
    ),
    (
        0x0B6F,
        "dead 2022 registration: 2019 sends HC_ACCEPT_MAKECHAR as 0x006D; 0x0B6F is the PACKETVER >= 20201007 layout",
    ),
    (
        0x0B70,
        "dead 2022 registration: 2019 sends HC_ACK_CHANGE_CHARACTER_SLOT as 0x08D5; 0x0B70 is the PACKETVER >= 20201007 layout",
    ),
    (
        0x0B72,
        "dead 2022 registration: 2019 sends HC_ACK_CHARINFO_PER_PAGE as 0x099D; 0x0B72 is the PACKETVER >= 20201007 layout",
    ),
    (
        0x0B7A,
        "dead 2022 registration: 2019 has no NPC market (0x09D5/0x09D7 are the closest 2019 packets); 0x0B7A is the 2021+ layout",
    ),
];

/// Full packet payloads for opcodes whose all-zero / all-one synthetic payload
/// hits an invalid enum discriminant. These are the exact 2019 wire bytes; the
/// generic harness parses them and still asserts full consumption.
const PAYLOAD_FIXTURES: &[(u16, &[u8])] = &[
    // 0x00BE ZC_STATUS_CHANGE: stat-point-cost id (32) + u8 amount.
    (0x00BE, &[0xBE, 0x00, 0x20, 0x00, 0x01]),
    // 0x09CA ZC_SKILL_ENTRY5: length 23, ids, position, unit id 0x7E (safety wall), range/visible/level.
    (0x09CA, &[
        0xCA, 0x09, 0x17, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x01, 0x01,
        0x01,
    ]),
    // 0x0A27 ZC_RECOVERY: heal type 5 (HP) + u32 amount.
    (0x0A27, &[0x27, 0x0A, 0x05, 0x00, 0x64, 0x00, 0x00, 0x00]),
    // 0x0ACC ZC_NOTIFY_EXP2: account id, amount, experience type 1 (base), source 0 (regular).
    (0x0ACC, &[
        0xCC, 0x0A, 0x00, 0x00, 0x00, 0x00, 0xD2, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    ]),
];

/// Builds a packet buffer of exactly `wire_size` bytes with `header` and a
/// zeroed payload, or a payload filled with `filler`.
fn buffer_with_size(header: u16, wire_size: usize, filler: u8) -> Vec<u8> {
    let mut bytes = vec![filler; wire_size];
    bytes[0..2].copy_from_slice(&header.to_le_bytes());
    bytes
}

fn set_declared_length(bytes: &mut [u8], length: usize) {
    bytes[2..4].copy_from_slice(&(length as u16).to_le_bytes());
}

fn metadata_is_variable(handler: &PacketHandler<NetworkEventList, NoPacketCallback>, opcode: u16) -> bool {
    handler
        .metadata(ragnarok_packets::PacketHeader(opcode))
        .map(|metadata| metadata.is_variable_length)
        .unwrap_or(false)
}

fn feed(handler: &mut PacketHandler<NetworkEventList, NoPacketCallback>, bytes: &[u8]) -> Result<usize, String> {
    let mut reader = ByteReader::without_metadata(bytes);

    match handler.process_one(&mut reader) {
        HandlerResult::Ok(_) => Ok(reader.get_offset()),
        HandlerResult::UnhandledPacket => Err("unhandled packet".to_owned()),
        HandlerResult::PacketCutOff => Err("reported as cut off".to_owned()),
        HandlerResult::InternalError(error) => Err(format!("handler failed: {error:?}")),
    }
}

#[test]
fn ignored_opcodes_are_registered_and_reasoned() {
    let mut registered = std::collections::HashSet::new();

    for (server, _) in SERVERS {
        for header in handler(server).registered_headers() {
            registered.insert(header.0);
        }
    }

    let mut seen = std::collections::HashSet::new();

    for (opcode, reason) in IGNORED_OPCODES {
        assert!(*opcode != 0x0000, "sentinel ignore entry must be removed");
        assert!(!reason.is_empty(), "ignored opcode 0x{opcode:04X} needs a reason");
        assert!(seen.insert(*opcode), "opcode 0x{opcode:04X} is ignored twice");
        assert!(
            registered.contains(opcode),
            "0x{opcode:04X} is in the ignore list but is not registered any more (stale entry: {reason})"
        );
    }
}

/// Human-readable coverage report (run with `-- --nocapture`).
#[test]
fn report_registered_coverage() {
    for (server, name) in SERVERS {
        let handler = handler(server);
        let headers = handler.registered_headers();
        let missing: Vec<String> = headers
            .iter()
            .filter(|header| opcode_length(**header).is_none())
            .map(|header| format!("0x{:04X}", header.0))
            .collect();

        println!(
            "{}: {} registered, {} without registry entry: {:?}",
            name,
            headers.len(),
            missing.len(),
            missing
        );
    }
}

#[test]
fn all_registered_opcodes_have_a_registry_entry_or_an_explicit_ignore() {
    let ignored: std::collections::HashSet<u16> = IGNORED_OPCODES.iter().map(|(opcode, _)| *opcode).collect();
    let mut problems = Vec::new();

    for (server, name) in SERVERS {
        for header in handler(server).registered_headers() {
            if opcode_length(header).is_none() && !ignored.contains(&header.0) {
                problems.push(format!(
                    "registered {} packet 0x{:04X} has no 2019 registry entry and is not in IGNORED_OPCODES",
                    name, header.0
                ));
            }
        }
    }

    assert!(
        problems.is_empty(),
        "{} opcode(s) missing from the 2019 registry:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

#[test]
fn registered_struct_metadata_agrees_with_the_registry() {
    set_mode(ValidationMode::Enforce);

    let mut problems = Vec::new();

    for (server, name) in SERVERS {
        for header in handler(server).registered_headers() {
            let Some(length) = opcode_length(header) else {
                continue;
            };

            let Some(metadata) = handler(server).metadata(header) else {
                problems.push(format!("{}: 0x{:04X} has no metadata", name, header.0));
                continue;
            };

            match length {
                OpcodeLength::Fixed(size) => {
                    // A few 2019 packets are fixed size on the wire but still
                    // carry their own length field (because the client struct is
                    // shared with a variable 2022 layout). Their structs are
                    // `#[variable_length]`; the harness feeds the fixed size as
                    // the declared length for them.
                    if let Some(expected_size) = metadata.expected_size
                        && expected_size != size
                    {
                        problems.push(format!(
                            "{}: 0x{:04X} declares #[packet_size({expected_size})] but the 2019 server sends {size} bytes",
                            name, header.0
                        ));
                    }
                }
                OpcodeLength::Variable { .. } => {
                    if !metadata.is_variable_length {
                        problems.push(format!(
                            "{}: 0x{:04X} is variable per the 2019 registry but the struct is not #[variable_length]",
                            name, header.0
                        ));
                    }
                }
            }
        }
    }

    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Feeds every registered handler a synthetic buffer of the registered 2019
/// size and asserts that the struct consumes the packet exactly.
#[test]
fn every_registered_server_packet_consumes_its_2019_wire_size() {
    set_mode(ValidationMode::Enforce);

    let ignored: std::collections::HashSet<u16> = IGNORED_OPCODES.iter().map(|(opcode, _)| *opcode).collect();
    let mut problems: Vec<String> = Vec::new();
    let mut checked = 0;

    for (server, name) in SERVERS {
        let mut packet_handler = handler(server);

        for header in packet_handler.registered_headers() {
            if ignored.contains(&header.0) {
                continue;
            }

            let Some(length) = opcode_length(header) else {
                continue;
            };

            let result = match length {
                _ if PAYLOAD_FIXTURES.iter().any(|(fixture_header, _)| *fixture_header == header.0) => {
                    let fixture = PAYLOAD_FIXTURES
                        .iter()
                        .find(|(fixture_header, _)| *fixture_header == header.0)
                        .unwrap()
                        .1;

                    match feed(&mut packet_handler, fixture) {
                        Ok(consumed) if consumed == fixture.len() => Ok(()),
                        Ok(consumed) => Err(format!(
                            "0x{:04X} fixture consumed {consumed} of {} bytes",
                            header.0,
                            fixture.len()
                        )),
                        Err(error) => Err(format!("0x{:04X} fixture: {error}", header.0)),
                    }
                }
                OpcodeLength::Fixed(size) => {
                    // Some structs reject an all-zero payload because 0 is not
                    // a valid enum discriminant; try a second, valid-looking
                    // filler before declaring the layout broken.
                    let mut result = None;

                    for filler in [0u8, 1u8] {
                        let mut bytes = buffer_with_size(header.0, size, filler);

                        // Some 2019 packets are fixed size on the wire but still
                        // carry their own length field (the client struct reads it
                        // because it is shared with a variable layout). The server
                        // always writes the fixed size there.
                        if metadata_is_variable(&packet_handler, header.0) {
                            set_declared_length(&mut bytes, size);
                        }

                        match feed(&mut packet_handler, &bytes) {
                            Ok(consumed) if consumed == size => {
                                result = Some(Ok(()));
                                break;
                            }
                            Ok(consumed) => {
                                result = Some(Err(format!("0x{:04X} consumed {consumed} of {size} bytes", header.0)));
                            }
                            Err(error) => {
                                result = Some(Err(format!("0x{:04X} ({size} bytes, filler {filler}): {error}", header.0)));
                            }
                        }
                    }

                    result.unwrap_or_else(|| Err(format!("0x{:04X}: no candidate buffer", header.0)))
                }
                OpcodeLength::Variable { base_size } => {
                    // Every variable packet self-describes its length. Try all
                    // plausible total lengths; the first one that parses and is
                    // consumed completely proves that declared == consumed.
                    let mut result = None;

                    'candidate: for filler in [0u8, 1u8] {
                        for declared in (base_size.max(4)..=600).step_by(1) {
                            let mut bytes = buffer_with_size(header.0, declared, filler);
                            set_declared_length(&mut bytes, declared);

                            match feed(&mut packet_handler, &bytes) {
                                Ok(consumed) if consumed == declared => {
                                    result = Some(Ok(()));
                                    break 'candidate;
                                }
                                Ok(_) | Err(_) => {}
                            }
                        }
                    }

                    result.unwrap_or_else(|| {
                        Err(format!(
                            "0x{:04X}: no declared length in {}..=600 parsed successfully (variable packet)",
                            header.0,
                            base_size.max(4)
                        ))
                    })
                }
            };

            checked += 1;

            if let Err(problem) = result {
                problems.push(format!("{}: {problem}", name));
            }
        }
    }

    assert!(
        checked >= 140,
        "expected to check at least 140 registered packets, only found {checked}"
    );
    assert!(
        problems.is_empty(),
        "{} registered packets failed the 2019 contract:\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// Convenience: parse a hand-written fixture and assert it consumed everything.
fn fixture(server: Server, bytes: &[u8]) -> Result<(), String> {
    let mut packet_handler = handler(server);

    match feed(&mut packet_handler, bytes) {
        Ok(consumed) if consumed == bytes.len() => Ok(()),
        Ok(consumed) => Err(format!("consumed {consumed} of {} bytes", bytes.len())),
        Err(error) => Err(error),
    }
}

/// Feed a sequence of packets into one handler (for packets whose handler keeps
/// state between the start/list/end packets).
fn fixture_sequence(server: Server, packets: &[Vec<u8>]) -> Result<(), String> {
    let mut packet_handler = handler(server);

    for bytes in packets {
        match feed(&mut packet_handler, bytes) {
            Ok(consumed) if consumed == bytes.len() => {}
            Ok(consumed) => return Err(format!("consumed {consumed} of {} bytes", bytes.len())),
            Err(error) => return Err(error),
        }
    }

    Ok(())
}

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

/// Byte-exact 2019 fixtures for the layouts that caused real bugs before.
///
/// These are intentionally *not* generated: they document the exact wire
/// layout that the 2019 server sends and will fail if a struct drifts.
#[test]
fn critical_2019_fixtures() {
    set_mode(ValidationMode::Enforce);

    // 0x0A37 ZC_ITEM_PICKUP_ACK, 69 bytes (2019: EQUIPSLOTINFO is uint32 card[4]).
    let mut pickup = Vec::new();
    pickup.extend_from_slice(&le16(0x0A37));
    pickup.extend_from_slice(&le16(9)); // inventory index + 2
    pickup.extend_from_slice(&le16(3)); // amount
    pickup.extend_from_slice(&le32(2206)); // item id
    pickup.push(1); // identified
    pickup.push(0); // damaged
    pickup.push(5); // refinement
    pickup.extend_from_slice(&[0u8; 16]); // card: [u32; 4]
    pickup.extend_from_slice(&le32(0x0000_0100)); // equip position (head top)
    pickup.push(4); // item type
    pickup.push(0); // result
    pickup.extend_from_slice(&le32(0)); // hire expiration
    pickup.extend_from_slice(&le16(0)); // bind on equip
    pickup.extend_from_slice(&[0u8; 5 * 5]); // 5 item options (index u16 / value u16 / parameter u8)
    pickup.push(0); // favorite
    pickup.extend_from_slice(&le16(44)); // item sprite number
    assert_eq!(pickup.len(), 69, "0x0A37 fixture must be 69 bytes");
    assert_eq!(fixture(Server::Map, &pickup), Ok(()), "0x0A37 pickup fixture");

    // 0x07D8 ZC_REQ_GROUPINFO_CHANGE_V2, 8 bytes (party exp/item rules).
    let mut party_option = Vec::new();
    party_option.extend_from_slice(&le16(0x07D8));
    party_option.extend_from_slice(&le32(1));
    party_option.extend_from_slice(&[1, 1]);
    assert_eq!(party_option.len(), 8);
    assert_eq!(fixture(Server::Map, &party_option), Ok(()), "0x07D8 party option fixture");

    // 0x0AE4 ZC_ADD_MEMBER_TO_GROUP, 89 bytes for PACKETVER >= 20171207.
    let mut add_member = Vec::new();
    add_member.extend_from_slice(&le16(0x0AE4));
    add_member.extend_from_slice(&le32(2000001)); // account id
    add_member.extend_from_slice(&le32(150001)); // char id
    add_member.extend_from_slice(&le32(1)); // role
    add_member.extend_from_slice(&le16(7)); // class
    add_member.extend_from_slice(&le16(99)); // base level
    add_member.extend_from_slice(&le16(150)); // x
    add_member.extend_from_slice(&le16(150)); // y
    add_member.push(0); // state
    push_string(&mut add_member, "Party", 24);
    push_string(&mut add_member, "Player", 24);
    push_string(&mut add_member, "prontera.gat", 16);
    add_member.push(1); // pickup rule
    add_member.push(1); // share rule
    assert_eq!(add_member.len(), 89, "0x0AE4 fixture must be 89 bytes");
    assert_eq!(fixture(Server::Map, &add_member), Ok(()), "0x0AE4 party member fixture");

    // 0x0AE5 ZC_GROUP_LIST, 28 + 54 per member.
    let mut party_list = Vec::new();
    party_list.extend_from_slice(&le16(0x0AE5));
    party_list.extend_from_slice(&le16(0)); // length, patched below
    push_string(&mut party_list, "Party", 24);
    let mut member = Vec::new();
    member.extend_from_slice(&le32(2000001)); // account id
    member.extend_from_slice(&le32(150001)); // char id
    push_string(&mut member, "Player", 24);
    push_string(&mut member, "prontera.gat", 16);
    member.push(0); // leader
    member.push(0); // state
    member.extend_from_slice(&le16(7)); // class
    member.extend_from_slice(&le16(99)); // base level
    assert_eq!(member.len(), 54);
    party_list.extend_from_slice(&member);
    let total = party_list.len() as u16;
    party_list[2..4].copy_from_slice(&total.to_le_bytes());
    assert_eq!(party_list.len(), 28 + 54);
    assert_eq!(fixture(Server::Map, &party_list), Ok(()), "0x0AE5 party list fixture");

    // 0x080E ZC_NOTIFY_HP_TO_GROUPM, 14 bytes for PACKETVER >= 20100119.
    let mut hp = Vec::new();
    hp.extend_from_slice(&le16(0x080E));
    hp.extend_from_slice(&le32(150001));
    hp.extend_from_slice(&le32(1234));
    hp.extend_from_slice(&le32(5678));
    assert_eq!(hp.len(), 14);
    assert_eq!(fixture(Server::Map, &hp), Ok(()), "0x080E party hp fixture");

    // 0x010F ZC_SKILLINFO_LIST, 4 + 37 per skill.
    let mut skills = Vec::new();
    skills.extend_from_slice(&le16(0x010F));
    skills.extend_from_slice(&le16(0)); // length, patched below
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
        skills.extend_from_slice(&entry);
    }
    let total = skills.len() as u16;
    skills[2..4].copy_from_slice(&total.to_le_bytes());
    assert_eq!(skills.len(), 4 + 37 * 2);
    assert_eq!(fixture(Server::Map, &skills), Ok(()), "0x010F skill list fixture");

    // 0x00C6 ZC_PC_PURCHASE_ITEMLIST, 4 + 13 per shop entry.
    let mut shop = Vec::new();
    shop.extend_from_slice(&le16(0x00C6));
    shop.extend_from_slice(&le16(0)); // length, patched below
    shop.extend_from_slice(&le32(50)); // price
    shop.extend_from_slice(&le32(45)); // discount price
    shop.push(0); // item type
    shop.extend_from_slice(&le32(501)); // item id
    let total = shop.len() as u16;
    shop[2..4].copy_from_slice(&total.to_le_bytes());
    assert_eq!(shop.len(), 4 + 13);
    assert_eq!(fixture(Server::Map, &shop), Ok(()), "0x00C6 shop list fixture");

    // 0x00CA ZC_PC_PURCHASE_RESULT, 3 bytes.
    let mut shop_result = Vec::new();
    shop_result.extend_from_slice(&le16(0x00CA));
    shop_result.push(0);
    assert_eq!(shop_result.len(), 3);
    assert_eq!(fixture(Server::Map, &shop_result), Ok(()), "0x00CA shop result fixture");

    // 0x00C8 CZ_PC_PURCHASE_ITEMLIST (client -> server): round trip.
    let mut buy = Vec::new();
    buy.extend_from_slice(&le16(0x00C8));
    buy.extend_from_slice(&le16(4 + 6));
    buy.extend_from_slice(&le16(2)); // amount
    buy.extend_from_slice(&le32(501)); // item id
    assert_eq!(buy.len(), 10);
    let packet = ragnarok_packets::BuyItemsPacket::packet_from_bytes(&mut ByteReader::without_metadata(&buy)).expect("0x00C8 parse");
    let mut writer = ragnarok_bytes::ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    assert_eq!(writer.into_inner(), buy, "0x00C8 shop request round trip");

    // 0x0201 ZC_FRIENDS_LIST, 4 + 8 per entry (2019 drops the 24 byte name).
    let mut friends = Vec::new();
    friends.extend_from_slice(&le16(0x0201));
    friends.extend_from_slice(&le16(4 + 8 * 2));
    for id in [42u32, 43] {
        friends.extend_from_slice(&le32(id)); // account id
        friends.extend_from_slice(&le32(150000 + id)); // char id
    }
    assert_eq!(friends.len(), 4 + 8 * 2);
    assert_eq!(fixture(Server::Map, &friends), Ok(()), "0x0201 friend list fixture");

    // 0x0B20 ZC_SHORTCUT_KEY_LIST, fixed 271 bytes.
    let hotkeys = buffer_with_size(0x0B20, 271, 0);
    assert_eq!(fixture(Server::Map, &hotkeys), Ok(()), "0x0B20 hotkey fixture");

    // 0x0B09 ZC_ITEMLIST_NORMAL, 4 + 34 per entry.
    let mut inventory = Vec::new();
    inventory.extend_from_slice(&le16(0x0B09));
    inventory.extend_from_slice(&le16(5 + 34));
    inventory.push(0); // inventory type
    let mut item = Vec::new();
    item.extend_from_slice(&le16(2)); // index (server slot + 2)
    item.extend_from_slice(&le32(501)); // item id
    item.push(0); // item type
    item.extend_from_slice(&le16(5)); // amount
    item.extend_from_slice(&le32(0)); // equip position
    item.extend_from_slice(&[0u8; 16]); // cards
    item.extend_from_slice(&le32(0)); // hire expiration
    item.push(0); // flags
    assert_eq!(item.len(), 34);
    inventory.extend_from_slice(&item);
    assert_eq!(inventory.len(), 5 + 34);

    // The inventory handlers keep state between the 0x0B08 start, the 0x0B09
    // list and the 0x0B0B end packet, so feed the whole sequence.
    let mut inventory_start = Vec::new();
    inventory_start.extend_from_slice(&le16(0x0B08));
    inventory_start.extend_from_slice(&le16(6)); // 2 + 2 + type + NUL
    inventory_start.push(0); // inventory type
    inventory_start.push(0); // empty inventory name
    let inventory_end = vec![0x0B, 0x0B, 0, 0];

    assert_eq!(
        fixture_sequence(Server::Map, &[inventory_start, inventory, inventory_end]),
        Ok(()),
        "0x0B08/0x0B09/0x0B0B inventory list fixture"
    );
}

/// Unit tests for the deterministic unknown-packet skip.
mod unknown_packet {
    use ragnarok_packets::handler::{NoPacketCallback, PacketHandler};
    use ragnarok_packets::*;

    use crate::event::NetworkEventList;
    use crate::{UnknownPacketAction, unknown_packet_action};

    fn handler() -> PacketHandler<NetworkEventList, NoPacketCallback> {
        let mut handler = PacketHandler::<NetworkEventList, NoPacketCallback>::default();
        handler.register_noop::<MapServerLoginSuccessPacket>().unwrap();
        handler.register_noop::<DamagePacket3>().unwrap();

        handler
    }

    #[test]
    fn known_fixed_packet_is_skipped_exactly() {
        // 0x083E (login server refuse login, 26 bytes) is not registered in the
        // map handler, but the registry knows its exact size.
        let mut data = vec![0u8; 26];
        data[0..2].copy_from_slice(&0x083Eu16.to_le_bytes());
        data.extend_from_slice(&0x02EBu16.to_le_bytes());

        match unknown_packet_action(&data, 0, &handler()) {
            UnknownPacketAction::Skip(26) => {}
            action => panic!("expected exact skip of 26 bytes, got {action:?}"),
        }
    }

    #[test]
    fn known_fixed_packet_is_kept_until_complete() {
        let mut data = vec![0u8; 20];
        data[0..2].copy_from_slice(&0x083Eu16.to_le_bytes());

        assert_eq!(unknown_packet_action(&data, 0, &handler()), UnknownPacketAction::NeedMoreData);
    }

    #[test]
    fn known_variable_packet_uses_declared_length() {
        // 0x006B char list: 27 byte fixed part, 2 rows -> 27 + 155 * 2 bytes.
        let mut data = vec![0u8; 27 + 155 * 2];
        data[0..2].copy_from_slice(&0x006Bu16.to_le_bytes());
        data[2..4].copy_from_slice(&((27 + 155 * 2) as u16).to_le_bytes());

        match unknown_packet_action(&data, 0, &handler()) {
            UnknownPacketAction::Skip(length) => assert_eq!(length, 27 + 155 * 2),
            action => panic!("expected declared-length skip, got {action:?}"),
        }
    }

    #[test]
    fn known_variable_packet_waits_for_the_rest() {
        let mut data = vec![0u8; 40];
        data[0..2].copy_from_slice(&0x006Bu16.to_le_bytes());
        data[2..4].copy_from_slice(&500u16.to_le_bytes());

        assert_eq!(unknown_packet_action(&data, 0, &handler()), UnknownPacketAction::NeedMoreData);
    }

    #[test]
    fn unknown_packet_heuristic_falls_back_when_it_lands_on_a_registered_header() {
        // 0x0FFF is not in the registry: length field says 8, followed by a
        // registered header, so the heuristic may skip it.
        let mut data = vec![0u8; 8];
        data[0..2].copy_from_slice(&0x0FFFu16.to_le_bytes());
        data[2..4].copy_from_slice(&8u16.to_le_bytes());
        data.extend_from_slice(&0x02EBu16.to_le_bytes());
        data.resize(16, 0);

        assert_eq!(unknown_packet_action(&data, 0, &handler()), UnknownPacketAction::Skip(8));
    }

    #[test]
    fn unknown_packet_without_a_registered_successor_is_discarded() {
        let mut data = vec![0u8; 8];
        data[0..2].copy_from_slice(&0x0FFFu16.to_le_bytes());
        data[2..4].copy_from_slice(&8u16.to_le_bytes());
        data.extend_from_slice(&0x0FFEu16.to_le_bytes());
        data.resize(16, 0);

        assert_eq!(unknown_packet_action(&data, 0, &handler()), UnknownPacketAction::Discard);
    }
}
