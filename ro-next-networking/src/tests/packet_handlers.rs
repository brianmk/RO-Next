use std::collections::HashSet;

use ragnarok_bytes::{ByteReader, ByteWriter};
use ragnarok_packets::handler::{HandlerResult, NoPacketCallback};
use ragnarok_packets::{
    AccountId, AttackRange, CharacterId, ClientTick, Direction, DisplayEmotionPacket, EntityId, GuildEmblemImagePacket,
    GuildEmblemRequestPacket, GuildInviteRequestPacket, GuildLeaveRequestPacket, GuildMemberNameRequestPacket,
    GuildMenuInterfaceRequestPacket, GuildMenuRequestPacket, GuildNoticeChangeRequestPacket, InventoryIndex, ItemId, MarkerType,
    NAME_LENGTH, PacketExt, PacketHeader, RequestEmotionPacket, RequestEntityDetailsSuccessPacket, RequestPlayerDetailsSuccessPacket,
    SkillId, TilePosition,
};

use crate::event::NetworkEventList;
use crate::packet_versions::noop_policy::{IGNORED_NOOPS, IgnoreClass, NoopServer, register_ignored_noop};
use crate::{NetworkEvent, NetworkingSystem, SupportedPacketVersion, UnknownPacketAction, unknown_packet_action};

#[test]
fn every_ignored_noop_is_registered_with_a_reason() {
    let map_handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
    let character_handler =
        NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut seen = HashSet::new();

    for entry in IGNORED_NOOPS {
        assert!(
            seen.insert(entry.header),
            "noop 0x{:04X} is classified more than once",
            entry.header
        );
        assert!(!entry.reason.is_empty(), "noop 0x{:04X} has no reason", entry.header);

        if let IgnoreClass::OwnedElsewhere(owner) = entry.class {
            assert!(!owner.is_empty(), "owned noop 0x{:04X} has no owner", entry.header);
        }

        let headers = match entry.server {
            NoopServer::Map => map_handler.registered_headers(),
            NoopServer::Character => character_handler.registered_headers(),
        };

        assert!(
            headers.contains(&PacketHeader(entry.header)),
            "classified noop 0x{:04X} is not registered for {:?}",
            entry.header,
            entry.server
        );
    }
}

/// The version module plus its per-domain registration submodules, read at test
/// time so the check keeps working when a registration moves to another file.
fn version_module_sources() -> String {
    fn walk(dir: &std::path::Path, out: &mut String) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                out.push_str(&std::fs::read_to_string(&path).expect("registration source is readable"));
            }
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/packet_versions");
    let mut sources = std::fs::read_to_string(root.join("version_20220406.rs")).expect("version module is readable");
    walk(&root.join("version_20220406"), &mut sources);
    sources
}

#[test]
fn version_module_registers_no_bare_noops() {
    let source = version_module_sources();

    assert_eq!(
        source.matches("register_noop::<").count(),
        0,
        "every parsed-but-unconsumed packet must use register_ignored_noop with a classification"
    );
    // The exact number shrinks as features land (the noop table is a
    // backlog), so only a sanity floor is asserted here; the table itself
    // is validated entry by entry in
    // `every_ignored_noop_is_registered_with_a_reason`.
    assert!(
        source.matches("register_ignored_noop::<").count() >= 10,
        "expected the classified noop registrations to remain in place"
    );
}

#[test]
fn dead_2022_packets_are_not_registered() {
    let map_handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
    let headers = map_handler.registered_headers();

    for dead_header in [0x08E2u16, 0x07FB] {
        assert!(
            !headers.contains(&PacketHeader(dead_header)),
            "dead 2022 packet 0x{dead_header:04X} is still registered"
        );
    }

    let packet_source = include_str!("../../../ragnarok-packets/src/lib.rs");
    for dead_struct in ["NavigateToMonsterPacket", "UseSkillSuccessPacket"] {
        assert!(
            !packet_source.contains(&format!("pub struct {dead_struct}")),
            "dead 2022 struct {dead_struct} still exists"
        );
    }
}

#[test]
#[should_panic(expected = "is not classified")]
fn unclassified_noop_is_rejected() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let _ = register_ignored_noop::<_, ragnarok_packets::MapServerLoginSuccessPacket>(&mut handler);
}

#[test]
fn implemented_noops_emit_typed_events() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // `ZC_SPIRITS` (0x01D0): header + entity id u32 + amount i16.
    let mut bytes = vec![0xD0, 0x01];
    bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
    bytes.extend_from_slice(&3i16.to_le_bytes());
    assert_eq!(bytes.len(), 8);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("spirits packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::SpiritOrbs {
        entity_id: EntityId(0x0102_0304),
        amount: 3,
    }]));

    // `ZC_SKILL_POSTDELAY` (0x043D): header + skill id u16 + until u32.
    let mut bytes = vec![0x3D, 0x04];
    bytes.extend_from_slice(&34u16.to_le_bytes());
    bytes.extend_from_slice(&500u32.to_le_bytes());
    assert_eq!(bytes.len(), 8);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("skill cooldown packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::SkillCooldown {
        skill_id: SkillId(34),
        until: ClientTick(500),
    }]));

    // `ZC_ATTACK_RANGE` (0x013A): header + range u16.
    let mut bytes = vec![0x3A, 0x01];
    bytes.extend_from_slice(&7u16.to_le_bytes());
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("attack range packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::AttackRangeChanged {
        attack_range: AttackRange(7),
    }]));

    // `ZC_STOPMOVE` (0x0088): header + entity id u32 + x u16 + y u16.
    let mut bytes = vec![0x88, 0x00];
    bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
    bytes.extend_from_slice(&120u16.to_le_bytes());
    bytes.extend_from_slice(&45u16.to_le_bytes());
    assert_eq!(bytes.len(), 10);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("stop move packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::EntityStopped {
        entity_id: EntityId(0x0102_0304),
        position: TilePosition { x: 120, y: 45 },
    }]));

    // `ZC_COMPASS` (0x0144): header + npc id u32 + marker type u32 +
    // position (x u32, y u32) + id u8 + colour RGBA.
    let mut bytes = vec![0x44, 0x01];
    bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&120u32.to_le_bytes());
    bytes.extend_from_slice(&45u32.to_le_bytes());
    bytes.push(2);
    bytes.extend_from_slice(&[10, 20, 30, 255]);
    assert_eq!(bytes.len(), 23);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("marker packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MinimapMarker {
        npc_id: EntityId(0x0102_0304),
        marker_type: MarkerType::DisplayUntilLeave,
        ..
    }]));

    // `ZC_NOTIFY_MAPPROPERTY2` (0x099B): header + map type u16 + flags u32.
    let mut bytes = vec![0x9B, 0x09];
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.extend_from_slice(&0x0001_0000u32.to_le_bytes());
    assert_eq!(bytes.len(), 8);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("map property packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MapProperties {
        map_type: 3,
        flags: 0x0001_0000,
    }]));

    // `ZC_NOTIFY_CARTITEM_COUNTINFO` (0x0121).
    let mut bytes = vec![0x21, 0x01];
    bytes.extend_from_slice(&5i16.to_le_bytes());
    bytes.extend_from_slice(&100i16.to_le_bytes());
    bytes.extend_from_slice(&500i32.to_le_bytes());
    bytes.extend_from_slice(&8000i32.to_le_bytes());
    assert_eq!(bytes.len(), 14);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("cart count packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::CartInfo {
        current_count: 5,
        maximum_count: 100,
        current_weight: 500,
        maximum_weight: 8000,
    }]));

    // `ZC_CONFIG_NOTIFY` (0x02DA): header + flag u8.
    let bytes = vec![0xDA, 0x02, 1];
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("show equip packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::ShowEquipmentWindow {
        open: true
    }]));

    // `ZC_AUTH_FAIL` (0x006A): header + reason u8.
    let bytes = vec![0x6A, 0x00, 1];
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("map auth failure packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MapServerAuthFailed { reason: 1 }]));

    // `ZC_NOTIFY_CRITICALWEIGHT` (0x0ADE): header + percentage u32.
    let mut bytes = vec![0xDE, 0x0A];
    bytes.extend_from_slice(&50u32.to_le_bytes());
    assert_eq!(bytes.len(), 6);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("critical weight packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::CriticalWeight { percentage: 50 }]));

    // `ZC_AUTH_OK_GID` (0x0283): header + account id u32 (classified noop).
    let mut bytes = vec![0x83, 0x02];
    bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
    assert_eq!(bytes.len(), 6);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("auth ok gid packet was not handled");
    };
    assert!(events.0.is_empty());

    // `ZC_UPDATE_MAPINFO` (0x0192): header + x u16 + y u16 + cell type u16 +
    // map name[16].
    let mut bytes = vec![0x92, 0x01];
    bytes.extend_from_slice(&120u16.to_le_bytes());
    bytes.extend_from_slice(&45u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    let mut map_name = [0u8; 16];
    map_name[..8].copy_from_slice(b"prontera");
    bytes.extend_from_slice(&map_name);
    assert_eq!(bytes.len(), 24);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("map cell packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MapCellChanged {
        position: TilePosition { x: 120, y: 45 },
        cell_type: 1,
        ..
    }]));
}

#[test]
fn every_supported_version_registers_all_server_handlers() {
    // Every registered packet version must produce three working handlers.
    // This is the guarantee that connection setup never fails for a version
    // the client claims to support.
    assert!(
        !SupportedPacketVersion::ALL.is_empty(),
        "at least one packet version must be supported"
    );

    for &packet_version in SupportedPacketVersion::ALL {
        let login = NetworkingSystem::create_login_server_packet_handler(NoPacketCallback, packet_version);
        assert!(
            login.is_ok(),
            "login server handler registration failed for {packet_version:?}: {:?}",
            login.err()
        );

        let character = NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, packet_version);
        assert!(
            character.is_ok(),
            "character server handler registration failed for {packet_version:?}: {:?}",
            character.err()
        );

        let map = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, packet_version);
        assert!(
            map.is_ok(),
            "map server handler registration failed for {packet_version:?}: {:?}",
            map.err()
        );

        // A version with no registered handlers is almost certainly a
        // forgotten `match` arm; require a plausible amount of handlers.
        assert!(map.expect("map handler checked above").registered_headers().len() > 100);
    }
}

#[test]
fn duplicate_handler_registration_is_reported_as_an_internal_error_event() {
    // The networking thread cannot build a connection when its handler
    // registration fails; it must report the exact header through an
    // `InternalError` event (shown as an error window) instead of panicking.
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let error = ragnarok_packets::handler::DuplicateHandlerError::new(PacketHeader(0x0064));

    crate::report_packet_handler_registration_failure("map", &error, &sender);

    let event = receiver.try_recv().expect("a registration failure must emit an event");
    match event {
        crate::NetworkEvent::InternalError { message } => {
            assert!(message.contains("0x0064"), "message must name the header: {message}");
            assert!(message.contains("map"), "message must name the server: {message}");
        }
        other => panic!("expected NetworkEvent::InternalError, got {other:?}"),
    }
}

#[test]
fn duplicate_registration_returns_the_exact_header_instead_of_panicking() {
    // The networking thread builds its handlers with these exact functions.
    // Registering the same version twice must return the duplicate header
    // (which the thread reports as a client error window), not panic.
    for &packet_version in SupportedPacketVersion::ALL {
        let mut login_handler = NetworkingSystem::create_login_server_packet_handler(NoPacketCallback, packet_version)
            .expect("login handler registration must succeed for a supported version");
        let login_duplicate = crate::packet_versions::version_20220406::register_login_server_packets(&mut login_handler)
            .expect_err("registering the same login handler set twice must be rejected");
        assert!(
            login_handler.registered_headers().contains(&login_duplicate.packet_header),
            "duplicate login registration must carry a registered header, got {login_duplicate}"
        );

        let mut character_handler = NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, packet_version)
            .expect("character handler registration must succeed for a supported version");
        let character_duplicate = crate::packet_versions::version_20220406::register_character_server_packets(&mut character_handler)
            .expect_err("registering the same character handler set twice must be rejected");
        assert!(
            character_handler.registered_headers().contains(&character_duplicate.packet_header),
            "duplicate character registration must carry a registered header, got {character_duplicate}"
        );

        let mut map_handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, packet_version)
            .expect("map handler registration must succeed for a supported version");
        let map_duplicate = crate::packet_versions::version_20220406::register_map_server_packets(&mut map_handler)
            .expect_err("registering the same map handler set twice must be rejected");
        assert!(
            map_handler.registered_headers().contains(&map_duplicate.packet_header),
            "duplicate map registration must carry a registered header, got {map_duplicate}"
        );

        // The error names the exact header, which is what the networking
        // thread logs and forwards to the error window.
        assert!(
            map_duplicate
                .to_string()
                .contains(&format!("0x{:04X}", map_duplicate.packet_header.0))
        );
    }
}

#[test]
fn emotion_request_uses_expected_header() {
    let mut writer = ByteWriter::new();
    RequestEmotionPacket { emotion: 3 }.packet_to_bytes(&mut writer).unwrap();

    assert_eq!(writer.as_slice(), &[0xBF, 0x00, 0x03]);
}

#[test]
fn guild_emblem_request_uses_expected_header() {
    let mut writer = ByteWriter::new();
    GuildEmblemRequestPacket::new(1).packet_to_bytes(&mut writer).unwrap();

    // `0x0151` + guild id (u32 little endian).
    assert_eq!(writer.as_slice(), &[0x51, 0x01, 0x01, 0x00, 0x00, 0x00]);
}

#[test]
fn guild_window_requests_use_the_2019_opcodes() {
    let mut writer = ByteWriter::new();
    GuildMenuInterfaceRequestPacket {}.packet_to_bytes(&mut writer).unwrap();
    assert_eq!(writer.as_slice(), &[0x4D, 0x01]);

    let mut writer = ByteWriter::new();
    GuildMenuRequestPacket { menu: 2 }.packet_to_bytes(&mut writer).unwrap();
    assert_eq!(writer.as_slice(), &[0x4F, 0x01, 0x02, 0x00, 0x00, 0x00]);

    // `CZ_REQNAME_BYGID` is 0x094C at 20190605 (the last
    // `clif_parse_SolveCharName` registration, clif_packetdb.hpp:1636).
    let mut writer = ByteWriter::new();
    GuildMemberNameRequestPacket {
        character_id: CharacterId(0x1234_5678),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(writer.as_slice(), &[0x4C, 0x09, 0x78, 0x56, 0x34, 0x12]);

    let mut writer = ByteWriter::new();
    GuildInviteRequestPacket { name: "Alt".to_owned() }
        .packet_to_bytes(&mut writer)
        .unwrap();
    assert_eq!(&writer.as_slice()[0..2], &[0x16, 0x09]);
    assert_eq!(writer.as_slice().len(), 26);

    let mut writer = ByteWriter::new();
    GuildLeaveRequestPacket {
        guild_id: 7,
        account_id: AccountId(150_009),
        character_id: CharacterId(150_009),
        message: String::new(),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(&writer.as_slice()[0..2], &[0x59, 0x01]);
    assert_eq!(writer.as_slice().len(), 54);

    let mut writer = ByteWriter::new();
    GuildNoticeChangeRequestPacket {
        guild_id: 7,
        subject: "subject".to_owned(),
        notice: "notice".to_owned(),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(&writer.as_slice()[0..2], &[0x6E, 0x01]);
    assert_eq!(writer.as_slice().len(), 186);
}

#[test]
fn guild_ack_packets_are_parsed_into_member_events() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0A83 ZC_ACK_LEAVE_GUILD: char id + reason[40].
    let mut bytes = vec![0x83, 0x0A];
    bytes.extend_from_slice(&150_009u32.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 40]);
    assert_eq!(bytes.len(), 46);
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("leave ack packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::GuildMemberRemoved {
        character_id: CharacterId(150_009),
        expelled: false
    }]));

    // 0x0A82 ZC_ACK_BAN_GUILD: reason[40] + char id.
    let mut bytes = vec![0x82, 0x0A];
    bytes.extend_from_slice(&[0u8; 40]);
    bytes.extend_from_slice(&150_010u32.to_le_bytes());
    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("ban ack packet was not handled");
    };
    assert!(matches!(events.0.as_slice(), [NetworkEvent::GuildMemberRemoved {
        character_id: CharacterId(150_010),
        expelled: true
    }]));
}

#[test]
fn sound_packet_is_parsed_into_play_sound_event() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // `ZC_SOUND` (0x01D3), 2019 layout: header + name[24] + act + term + aid.
    let mut bytes = vec![0xD3, 0x01];
    let mut name = [0u8; NAME_LENGTH];
    name[..18].copy_from_slice(b"effect\\ef_blessing");
    bytes.extend_from_slice(&name);
    bytes.push(1);
    bytes.extend_from_slice(&500u32.to_le_bytes());
    bytes.extend_from_slice(&0x1234_5678u32.to_le_bytes());
    assert_eq!(bytes.len(), 35);

    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("sound packet was not handled");
    };

    assert!(matches!(
        events.0.as_slice(),
        [NetworkEvent::PlaySound {
            name,
            act: 1,
            term: 500,
            account_id: AccountId(0x1234_5678),
        }] if name == "effect\\ef_blessing"
    ));
}

#[test]
fn heal_packet_emits_heal_effect_without_entity_id() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // `ZC_RECOVERY` (0x0A27), 2019 layout: header + type (SP_HP = 5) + amount u32.
    let mut bytes = vec![0x27, 0x0A];
    bytes.extend_from_slice(&5u16.to_le_bytes());
    bytes.extend_from_slice(&123u32.to_le_bytes());
    assert_eq!(bytes.len(), 8);

    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("heal packet was not handled");
    };

    assert!(matches!(events.0.as_slice(), [NetworkEvent::HealEffect {
        entity_id: None,
        heal_amount: 123,
    }]));
}

#[test]
fn state_change_packet_emits_state_change() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // `ZC_STATE_CHANGE` (0x0229), 2019 layout:
    // header + AID u32 + bodyState u16 + healthState u16 + effectState u32 +
    // isPKMode u8.
    let mut bytes = vec![0x29, 0x02];
    bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes()); // sitting? at least a state value
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&0x10u32.to_le_bytes()); // OPTION_FALCON
    bytes.push(1);
    assert_eq!(bytes.len(), 15);

    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("state change packet was not handled");
    };

    assert!(matches!(events.0.as_slice(), [NetworkEvent::StateChange {
        entity_id: EntityId(0x0102_0304),
        body_state: 2,
        health_state: 1,
        effect_state: 0x10,
        is_pk_mode_on: true,
    }]));
}

#[test]
fn change_direction_packet_emits_change_direction() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // `ZC_CHANGE_DIRECTION` (0x009C), 2019 layout: header + srcId u32 + headDir u16
    // + dir u8.
    let mut bytes = vec![0x9C, 0x00];
    bytes.extend_from_slice(&0x0102_0304u32.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes());
    bytes.push(4);
    assert_eq!(bytes.len(), 9);

    let mut reader = ByteReader::without_metadata(&bytes);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("change direction packet was not handled");
    };

    // Wire direction 4 maps to client direction `(8 - 4 + 4) & 7 = 0`.
    assert!(matches!(events.0.as_slice(), [NetworkEvent::ChangeDirection {
        entity_id: EntityId(0x0102_0304),
        head_direction: 3,
        direction: Direction::North,
    }]));
}

#[test]
fn guild_emblem_image_packet_is_parsed() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let emblem_data = vec![0x42, 0x4D, 0x36, 0x06, 0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];
    let packet = GuildEmblemImagePacket {
        guild_id: 1,
        emblem_id: 3,
        emblem_data: emblem_data.clone(),
    };

    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();

    // `0x0152` + packet length + guild id + emblem id + BMP data.
    assert_eq!(writer.as_slice().len(), 12 + emblem_data.len());
    assert_eq!(&writer.as_slice()[..2], &[0x52, 0x01]);
    assert_eq!(&writer.as_slice()[2..4], &(12u16 + emblem_data.len() as u16).to_le_bytes());

    let mut reader = ByteReader::without_metadata(writer.as_slice());

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("guild emblem packet was not handled");
    };

    assert!(matches!(
        events.0.as_slice(),
        [NetworkEvent::GuildEmblemImage {
            guild_id: 1,
            emblem_id: 3,
            emblem_data: data,
        }] if data == &emblem_data
    ));
}

#[test]
fn player_details_packet_carries_guild_name() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let packet = RequestPlayerDetailsSuccessPacket::new(
        CharacterId(0x1234_5678),
        "VerifyBot".to_owned(),
        String::new(),
        "Domination".to_owned(),
        String::new(),
        0,
    );

    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    let mut reader = ByteReader::without_metadata(writer.as_slice());

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("player-details packet was not handled");
    };

    assert!(matches!(
        events.0.as_slice(),
        [NetworkEvent::UpdateEntityDetails {
            entity_id: EntityId(0x1234_5678),
            guild_name: Some(guild_name),
            ..
        }] if guild_name == "Domination"
    ));
}

#[test]
fn player_details_packet_without_guild_has_no_guild_name() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let packet = RequestPlayerDetailsSuccessPacket::new(
        CharacterId(0x1111_2222),
        "VerifyBot".to_owned(),
        String::new(),
        // rAthena pads "no guild" with an empty fixed-length string.
        String::new(),
        String::new(),
        0,
    );

    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    let mut reader = ByteReader::without_metadata(writer.as_slice());

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("player-details packet was not handled");
    };

    assert!(matches!(events.0.as_slice(), [NetworkEvent::UpdateEntityDetails {
        entity_id: EntityId(0x1111_2222),
        guild_name: None,
        ..
    }]));
}

#[test]
fn npc_details_packet_has_no_guild_name() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let packet = RequestEntityDetailsSuccessPacket::new(EntityId(0x3333_4444), 0, "Prontera Guard".to_owned(), String::new());

    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    let mut reader = ByteReader::without_metadata(writer.as_slice());

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("entity-details packet was not handled");
    };

    assert!(matches!(events.0.as_slice(), [NetworkEvent::UpdateEntityDetails {
        entity_id: EntityId(0x3333_4444),
        guild_name: None,
        ..
    }]));
}

/// Builds a 2019 `SKILLDATA` payload (id u16, inf i32, level u16, sp u16,
/// range u16, name[24], up_flag u8) — the layout the 20190605 server sends
/// in both `ZC_SKILLINFO_LIST` (0x010F) and `ZC_ADD_SKILL` (0x0111).
fn skill_data_2019(skill_id: u16, skill_type: i32, level: u16, sp: u16, range: u16, name: &str, upgradable: u8) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&skill_id.to_le_bytes());
    data.extend_from_slice(&skill_type.to_le_bytes());
    data.extend_from_slice(&level.to_le_bytes());
    data.extend_from_slice(&sp.to_le_bytes());
    data.extend_from_slice(&range.to_le_bytes());

    let mut name_bytes = [0u8; NAME_LENGTH];
    for (index, byte) in name.as_bytes().iter().take(NAME_LENGTH - 1).enumerate() {
        name_bytes[index] = *byte;
    }
    data.extend_from_slice(&name_bytes);
    data.push(upgradable);
    data
}

#[test]
fn add_skill_packet_parses_the_2019_skill_data_layout() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut packet = Vec::new();
    packet.extend_from_slice(&0x0111u16.to_le_bytes());
    packet.extend_from_slice(&skill_data_2019(5, 4, 10, 8, 1, "Crazy Uproar", 1));

    assert_eq!(packet.len(), 39);

    let mut reader = ByteReader::without_metadata(&packet);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("add skill packet was not handled");
    };

    match events.0.as_slice() {
        [NetworkEvent::AddSkill { skill_information }] => {
            assert_eq!(skill_information.skill_id.0, 5);
            assert_eq!(skill_information.skill_level.0, 10);
            assert_eq!(skill_information.spell_point_cost, 8);
            assert_eq!(skill_information.attack_range.0, 1);
            assert_eq!(skill_information.skill_name, "Crazy Uproar");
            assert_eq!(skill_information.upgradable, 1);
        }
        events => panic!("unexpected events: {events:?}"),
    }
}

#[test]
fn skill_info_update_packet_parses_the_2019_layout() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut packet = Vec::new();
    packet.extend_from_slice(&0x07E1u16.to_le_bytes());
    packet.extend_from_slice(&5u16.to_le_bytes()); // skill id
    packet.extend_from_slice(&4i32.to_le_bytes()); // type flags (self cast)
    packet.extend_from_slice(&10u16.to_le_bytes()); // level
    packet.extend_from_slice(&8u16.to_le_bytes()); // sp cost
    packet.extend_from_slice(&1u16.to_le_bytes()); // range
    packet.push(0); // upgradable

    assert_eq!(packet.len(), 15);

    let mut reader = ByteReader::without_metadata(&packet);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("skill info update packet was not handled");
    };

    match events.0.as_slice() {
        [
            NetworkEvent::UpdateSkillInfo {
                skill_id,
                skill_level,
                spell_point_cost,
                attack_range,
                upgradable,
                ..
            },
        ] => {
            assert_eq!(skill_id.0, 5);
            assert_eq!(skill_level.0, 10);
            assert_eq!(*spell_point_cost, 8);
            assert_eq!(attack_range.0, 1);
            assert!(!*upgradable);
        }
        events => panic!("unexpected events: {events:?}"),
    }
}

#[test]
fn skill_tree_packet_parses_multiple_2019_skill_entries() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let first = skill_data_2019(5, 1, 10, 8, 1, "Bash", 1);
    let second = skill_data_2019(155, 4, 5, 30, 1, "Crazy Uproar", 0);

    let mut packet = Vec::new();
    packet.extend_from_slice(&0x010Fu16.to_le_bytes());
    packet.extend_from_slice(&((4 + first.len() + second.len()) as u16).to_le_bytes());
    packet.extend_from_slice(&first);
    packet.extend_from_slice(&second);

    let mut reader = ByteReader::without_metadata(&packet);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("skill tree packet was not handled");
    };

    match events.0.as_slice() {
        [NetworkEvent::SkillTree { skill_information }] => {
            assert_eq!(skill_information.len(), 2);
            assert_eq!(skill_information[0].skill_name, "Bash");
            assert_eq!(skill_information[1].skill_name, "Crazy Uproar");
            assert_eq!(skill_information[1].skill_level.0, 5);
        }
        events => panic!("unexpected events: {events:?}"),
    }
}

#[test]
fn skill_tree_packet_parses_a_full_login_skill_list() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // A full login list for a character with many learned skills: the server sends
    // 4 + 37 * entries bytes (37 byte SKILLDATA per skill) in one 0x010F packet.
    const ENTRIES: u16 = 37;
    let mut packet = Vec::new();
    packet.extend_from_slice(&0x010Fu16.to_le_bytes());
    packet.extend_from_slice(&(4u16 + ENTRIES * 37).to_le_bytes());

    for index in 0..ENTRIES {
        packet.extend_from_slice(&skill_data_2019(
            10 + index,
            1,
            (index % 10) as u16 + 1,
            index,
            1,
            "Test Skill",
            1,
        ));
    }

    assert_eq!(packet.len(), 4 + 37 * 37);

    let mut reader = ByteReader::without_metadata(&packet);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("full skill tree packet was not handled");
    };

    match events.0.as_slice() {
        [NetworkEvent::SkillTree { skill_information }] => {
            assert_eq!(skill_information.len(), 37);
            assert_eq!(skill_information[0].skill_id.0, 10);
            assert_eq!(skill_information[36].skill_id.0, 46);
            assert_eq!(skill_information[10].skill_level.0, 1);
        }
        events => panic!("unexpected events: {events:?}"),
    }
}

#[test]
fn skill_tree_packet_only_consumes_its_declared_entries() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // A skill list followed by another packet in the same TCP read: the variable
    // length parser has to stop exactly at the declared length so that the next
    // packet (a stat update here) is not swallowed or misaligned.
    let mut packet = Vec::new();
    packet.extend_from_slice(&0x010Fu16.to_le_bytes());
    packet.extend_from_slice(&(4u16 + 2 * 37).to_le_bytes());
    packet.extend_from_slice(&skill_data_2019(5, 1, 10, 8, 1, "Bash", 1));
    packet.extend_from_slice(&skill_data_2019(6, 2, 5, 12, 9, "Heal", 1));
    packet.extend_from_slice(&0x00B0u16.to_le_bytes());
    packet.extend_from_slice(&5u16.to_le_bytes()); // stat type: HP
    packet.extend_from_slice(&123u32.to_le_bytes()); // value

    let mut reader = ByteReader::without_metadata(&packet);

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("skill tree packet was not handled");
    };

    match events.0.as_slice() {
        [NetworkEvent::SkillTree { skill_information }] => assert_eq!(skill_information.len(), 2),
        events => panic!("unexpected events: {events:?}"),
    }

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("following stat update packet was not handled");
    };

    match events.0.as_slice() {
        [NetworkEvent::UpdateStat { stat_type }] => {
            assert_eq!(*stat_type, ragnarok_packets::StatType::HealthPoints(123));
        }
        events => panic!("unexpected events: {events:?}"),
    }
}

#[test]
fn unknown_2019_packet_is_skipped_without_losing_the_skill_list_after_it() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0B77 is not a registered 2019 packet for this client version. It carries a
    // plausible length followed by a full skill list in the same read: the skip
    // logic has to land exactly on the next (registered) packet header, otherwise
    // the login 0x010F skill list is lost (the original bug).
    let mut data = Vec::new();
    data.extend_from_slice(&0x0FFFu16.to_le_bytes());
    data.extend_from_slice(&12u16.to_le_bytes());
    data.extend_from_slice(&[0u8; 8]); // payload

    let skill_list_start = data.len();
    data.extend_from_slice(&0x010Fu16.to_le_bytes());
    data.extend_from_slice(&(4u16 + 37).to_le_bytes());
    data.extend_from_slice(&skill_data_2019(5, 1, 10, 8, 1, "Bash", 1));

    let skipped_to = match unknown_packet_action(&data, 0, &handler) {
        UnknownPacketAction::Skip(length) => length,
        action => panic!("expected the unknown packet to be skipped, got {action:?}"),
    };
    assert_eq!(skipped_to, skill_list_start);

    // The skill list itself is then parsed normally.
    let mut reader = ByteReader::without_metadata(&data[skipped_to..]);
    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("skill list after the skipped packet was not handled");
    };

    match events.0.as_slice() {
        [NetworkEvent::SkillTree { skill_information }] => assert_eq!(skill_information.len(), 1),
        events => panic!("unexpected events: {events:?}"),
    }
}

#[test]
fn longlong_stat_update_packet_carries_next_base_experience() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0ACB (ZC_LONGLONGPAR_CHANGE): varId 22 (SP_NEXTBASEEXP), 10 as i64.
    let packet: [u8; 12] = [
        0xCB, 0x0A, // header
        22, 0x00, // varId
        10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // amount
    ];
    let mut reader = ByteReader::without_metadata(&packet);

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("longlong stat update packet was not handled");
    };

    assert!(matches!(events.0.as_slice(), [NetworkEvent::UpdateStat {
        stat_type: ragnarok_packets::StatType::NextBaseExperience(10),
    }]));
}

/// The unknown packet skip only accepts a declared length when the offset
/// it points at starts another registered packet (or the end of the
/// read).
#[test]
fn unknown_variable_length_packet_can_be_skipped() {
    let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0FFF is not a registered packet, followed by a length of 12 bytes and
    // eight bytes of payload; directly after it a known stat packet starts.
    let mut data = vec![0xFF, 0x0F, 12, 0, 1, 2, 3, 4, 5, 6, 7, 8];
    data.extend_from_slice(&[0xB0, 0x00, 20, 0, 100, 0, 0, 0]);

    assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Skip(12));
}

#[test]
fn unknown_packet_ending_the_read_can_be_skipped() {
    let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let data = vec![0xFF, 0x0F, 8, 0, 1, 2, 3, 4];

    assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Skip(8));
}

/// Fixed-size packets have no length field, so bytes 2..4 are payload. The
/// skip must not be used unless the resulting offset starts a known packet.
#[test]
fn unknown_fixed_size_packet_is_not_skipped() {
    let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // Bytes 2..4 form a plausible length (20), but the offset it points at is
    // not another packet header and the read is longer, so it must not skip.
    let mut data = vec![0xFF, 0x0F, 20, 0];
    data.extend_from_slice(&[0; 24]);

    assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Discard);

    // A length below the minimum (4) is never accepted.
    let data = vec![0xFF, 0x0F, 3, 0, 1, 2, 3, 4, 5, 6, 7, 8];

    assert_eq!(unknown_packet_action(&data, 0, &handler), UnknownPacketAction::Discard);
}

/// rAthena sends these party packets during the map login burst of a
/// character that is in a party (in this order before the NPC/skill burst).
///
/// Every one of them has to be registered with its exact 2019 layout: an
/// unknown fixed-size packet (0x07D8 has no length field of its own) makes
/// the stream resync read its payload as a length, fail, and discard the
/// rest of the TCP read - which swallowed the NPC spawn burst and the whole
/// skill list for party characters (empty skill tree / no NPCs).
#[test]
fn party_login_burst_is_parsed_without_desync() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x07D8 ZC_PARTY_OPTION: exp option (u32), item pickup (u8), item share (u8).
    let mut data = vec![0xD8, 0x07];
    data.extend_from_slice(&1u32.to_le_bytes());
    data.push(1);
    data.push(0);

    // 0x0AE4 ZC_ADD_MEMBER_TO_GROUP (89 bytes).
    data.extend_from_slice(&0x0AE4u16.to_le_bytes());
    data.extend_from_slice(&150_009u32.to_le_bytes()); // account id
    data.extend_from_slice(&150_009u32.to_le_bytes()); // char id
    data.extend_from_slice(&0u32.to_le_bytes()); // role: 0 = leader
    data.extend_from_slice(&7u16.to_le_bytes()); // job
    data.extend_from_slice(&99u16.to_le_bytes()); // base level
    data.extend_from_slice(&155u16.to_le_bytes()); // x
    data.extend_from_slice(&180u16.to_le_bytes()); // y
    data.push(0); // online
    data.extend_from_slice(&fixed_name("feeders", NAME_LENGTH));
    data.extend_from_slice(&fixed_name("MrChamp", NAME_LENGTH));
    data.extend_from_slice(&fixed_name("prontera", 16));
    data.push(1); // share pickup
    data.push(1); // share loot

    // 0x0AE5 ZC_GROUP_LIST with one member (54 bytes).
    data.extend_from_slice(&0x0AE5u16.to_le_bytes());
    let party_list_length = 4u16 + NAME_LENGTH as u16 + 54;
    data.extend_from_slice(&party_list_length.to_le_bytes());
    data.extend_from_slice(&fixed_name("feeders", NAME_LENGTH));
    data.extend_from_slice(&150_009u32.to_le_bytes()); // account id
    data.extend_from_slice(&150_009u32.to_le_bytes()); // char id
    data.extend_from_slice(&fixed_name("MrChamp", NAME_LENGTH));
    data.extend_from_slice(&fixed_name("prontera", 16));
    data.push(1); // role: 1 = normal member
    data.push(0); // online
    data.extend_from_slice(&7u16.to_le_bytes()); // job
    data.extend_from_slice(&99u16.to_le_bytes()); // base level

    // 0x080E ZC_NOTIFY_HP_TO_GROUPM (14 bytes).
    data.extend_from_slice(&0x080Eu16.to_le_bytes());
    data.extend_from_slice(&150_009u32.to_le_bytes());
    data.extend_from_slice(&7500i32.to_le_bytes());
    data.extend_from_slice(&7500i32.to_le_bytes());

    // The skill list that used to be swallowed.
    data.extend_from_slice(&0x010Fu16.to_le_bytes());
    data.extend_from_slice(&(4u16 + 37).to_le_bytes());
    data.extend_from_slice(&skill_data_2019(5, 1, 10, 8, 1, "Bash", 1));

    let mut reader = ByteReader::without_metadata(&data);
    let mut events = Vec::new();

    while !reader.is_empty() {
        match handler.process_one(&mut reader) {
            HandlerResult::Ok(output) => events.extend(output.0),
            HandlerResult::UnhandledPacket => {
                let offset = reader.get_offset();
                let header = u16::from_le_bytes([data[offset], data[offset + 1]]);
                panic!("party login burst desynchronized at 0x{header:04X}");
            }
            HandlerResult::PacketCutOff => panic!("party login burst was cut off"),
            HandlerResult::InternalError(error) => panic!("party login burst handler failed: {error:?}"),
        }
    }

    assert!(matches!(events[0], NetworkEvent::PartyOption { experience_option: 1, .. }));
    assert!(matches!(&events[1], NetworkEvent::PartyMemberAdded { member } if member.name == "MrChamp" && member.leader));
    assert!(matches!(&events[2], NetworkEvent::PartyList { party_name, members } if party_name == "feeders" && members.len() == 1));
    assert!(matches!(events[3], NetworkEvent::PartyMemberHealth {
        health_points: 7500,
        maximum_health_points: 7500,
        ..
    }));
    assert!(matches!(&events[4], NetworkEvent::SkillTree { skill_information } if skill_information.len() == 1));
}

/// ZC_MSG (0x0291) used to be a noop, so msgstringtable system messages
/// were dropped completely.
#[test]
fn message_table_packet_reaches_the_client() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let events = feed(&mut handler, &[0x91, 0x02, 77, 0]);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MessageString { message_id: 77 }]));
}

/// The 0x0ACC experience packet has to reach the client (it was a noop, so
/// no exp feedback ever appeared). Layout: account id u32, amount i64,
/// var id u16 (1 = base, 2 = job), exp type u16 (0 = normal, 1 = quest).
#[test]
fn experience_gain_packet_reaches_the_client() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut data = vec![0xCC, 0x0A];
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1234i64.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());

    let events = feed(&mut handler, &data);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::GainedExperience {
        amount: 1234,
        experience_type: ragnarok_packets::ExperienceType::BaseExperience,
        experience_source: ragnarok_packets::ExperienceSource::Regular,
    }]));
}

/// ZC_ACK_TOUSESKILL (0x0110) used to be a noop, so a failed cast was
/// completely silent.
#[test]
fn skill_use_failure_packet_reaches_the_client() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut data = vec![0x10, 0x01];
    data.extend_from_slice(&5u16.to_le_bytes()); // skill id
    data.extend_from_slice(&0u32.to_le_bytes()); // btype
    data.extend_from_slice(&0u32.to_le_bytes()); // item id
    data.push(0); // flag
    data.push(1); // cause: not enough SP

    let events = feed(&mut handler, &data);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::SkillUseFailed {
        skill_id: ragnarok_packets::SkillId(5),
        cause: 1,
    }]));
}

/// The targeted skill packet (`CZ_USE_SKILL_TOID`, 0x089B) must send
/// `<level>.W <skill id>.W <target id>.L`; swapping level and id made the
/// server silently ignore every targeted cast (all buffs, Zen, ...).
#[test]
fn targeted_skill_packet_sends_level_before_skill_id() {
    use ragnarok_packets::{EntityId, SkillId, SkillLevel};

    let mut writer = ByteWriter::new();
    ragnarok_packets::UseSkillAtTargetPacket2019::new(SkillLevel(3), SkillId(401), EntityId(20_000_080))
        .packet_to_bytes(&mut writer)
        .unwrap();
    let bytes = writer.into_inner();

    assert_eq!(bytes.len(), 10);
    assert_eq!(&bytes[0..2], &[0x9B, 0x08]);
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 3, "level must be at offset 2");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 401, "skill id must be at offset 4");
    assert_eq!(
        u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]),
        20_000_080,
        "target id must be at offset 6"
    );
}

/// SC_NOTIFY_BAN (0x0081) on the map server: a 3-byte packet that used to
/// desynchronize the stream and silently kick the client.
#[test]
fn map_server_ban_packet_reaches_the_client() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let events = feed(&mut handler, &[0x81, 0x00, 2]);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MapServerBan { error_code: 2 }]));
}

/// The remaining cheap feedback packets that used to be noops.
#[test]
fn small_feedback_packets_reach_the_client() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // ZC_USER_COUNT (0x00C2): header + players count.
    let mut data = vec![0xC2, 0x00];
    data.extend_from_slice(&17i32.to_le_bytes());
    let events = feed(&mut handler, &data);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::UserCount { player_count: 17 }]));

    // ZC_STATUS_CHANGE_ACK (0x00BC): sp u16, ok u8, value u8 (ok = 0 = failure).
    let data = [0xBC, 0x00, 1, 0, 0, 0];
    let events = feed(&mut handler, &data);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::StatUpFailed]));

    // ZC_ACTION_FAILURE (0x013B): uint16 type.
    let data = [0x3B, 0x01, 2, 0];
    let events = feed(&mut handler, &data);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::AmmunitionAction {
        action: ragnarok_packets::AmmunitionActionType::WeightLimitExceeded2,
    }]));

    // ZC_EQUIP_ARROW (0x013C): uint16 inventory index (the wire value is the
    // client index + 2, like every other inventory index field).
    let data = [0x3C, 0x01, 5, 0];
    let events = feed(&mut handler, &data);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::AmmunitionEquipped {
        inventory_index: ragnarok_packets::InventoryIndex(3)
    }]));
}

/// Character deletion has to send the real account e-mail address: rAthena
/// runs with `char_del_option: 1` and compares the 40-byte field with the
/// account e-mail, so the old hardcoded "a@a.com" made every request fail.
#[test]
fn delete_character_request_carries_the_account_email() {
    use ragnarok_packets::*;

    // The packet itself must use the real address, zero padded to 40 bytes
    // and followed by the 10 ignored bytes (56 bytes total for 0x01FB).
    let mut writer = ByteWriter::new();
    DeleteCharacterPacket::new(CharacterId(150_044), "v1@test.local".to_owned())
        .packet_to_bytes(&mut writer)
        .unwrap();
    let bytes = writer.into_inner();

    assert_eq!(&bytes[..2], &[0xFB, 0x01]);
    assert_eq!(&bytes[2..6], &150_044u32.to_le_bytes());
    assert_eq!(&bytes[6..19], b"v1@test.local");
    assert!(bytes[19..46].iter().all(|byte| *byte == 0));
    assert_eq!(bytes.len(), 2 + 4 + 40 + 10);
}

/// The party requests have to use the 2019 opcodes and layouts, otherwise
/// rAthena ignores or misparses them (party play was impossible before).
#[test]
fn party_requests_use_the_2019_opcodes_and_layouts() {
    use ragnarok_packets::*;

    let mut writer = ByteWriter::new();
    CreatePartyPacket {
        party_name: "feeders".to_owned(),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(&writer.as_slice()[..2], &[0xF9, 0x00]);
    assert_eq!(writer.as_slice().len(), 2 + 24);
    assert_eq!(&writer.as_slice()[2..9], b"feeders");

    let mut writer = ByteWriter::new();
    PartyInviteRequestPacket2019 {
        character_name: "MrChamp".to_owned(),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(&writer.as_slice()[..2], &[0xC4, 0x02]);
    assert_eq!(writer.as_slice().len(), 2 + 24);

    let mut writer = ByteWriter::new();
    PartyInviteReplyRequestPacket2019 {
        party_id: PartyId(13),
        accept: 1,
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(writer.as_slice(), &[0xC7, 0x02, 13, 0, 0, 0, 1]);

    let mut writer = ByteWriter::new();
    LeavePartyPacket {}.packet_to_bytes(&mut writer).unwrap();
    assert_eq!(writer.as_slice(), &[0x00, 0x01]);

    let mut writer = ByteWriter::new();
    ExpelPartyMemberPacket {
        account_id: AccountId(150_009),
        character_name: "MrChamp".to_owned(),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    assert_eq!(&writer.as_slice()[..2], &[0x03, 0x01]);
    assert_eq!(writer.as_slice().len(), 2 + 4 + 24);

    let mut writer = ByteWriter::new();
    PartyChatRequestPacket {
        message: "MrChamp : hello".to_owned(),
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    let bytes = writer.into_inner();
    assert_eq!(&bytes[..2], &[0x08, 0x01]);
    let declared_length = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
    assert_eq!(declared_length, bytes.len());
    assert_eq!(&bytes[4..], b"MrChamp : hello\0");
}

/// Builds a NUL padded fixed length string field.
fn fixed_name(name: &str, length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    let name_bytes = name.as_bytes();
    let copied = name_bytes.len().min(length - 1);
    bytes[..copied].copy_from_slice(&name_bytes[..copied]);
    bytes
}

#[test]
fn party_login_packets_are_registered() {
    let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    for header in [
        0x07D8u16, 0x0AE4, 0x0AE5, 0x080E, 0x02C5, 0x00FA, 0x0105, 0x0107, 0x0109, 0x0ABD, 0x0AB2,
    ] {
        assert!(
            handler.is_registered(ragnarok_packets::PacketHeader(header)),
            "0x{header:04X} must be registered"
        );
    }
}

#[test]
fn booking_clan_and_companion_packets_are_registered_and_parsed() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    for header in [
        0x0803u16, 0x0805, 0x0807, 0x0809, 0x080A, 0x080B, 0x0AE7, 0x0AFA, 0x098A, 0x0988, 0x01A2, 0x01A4, 0x09F7, 0x07DB, 0x029B, 0x02A2,
    ] {
        assert!(
            handler.is_registered(ragnarok_packets::PacketHeader(header)),
            "0x{header:04X} must be registered"
        );
    }

    let register_result = feed(&mut handler, &[0x03, 0x08, 0x00, 0x00]);
    assert!(matches!(register_result.0.as_slice(), [
        NetworkEvent::PartyBookingRegisterResult { result: 0 }
    ]));

    let online_count = feed(&mut handler, &[0x88, 0x09, 0x03, 0x00, 0x14, 0x00]);
    assert!(matches!(online_count.0.as_slice(), [NetworkEvent::ClanOnlineCount {
        online_members: 3,
        maximum_members: 20
    }]));

    let mut pet = vec![0xA2, 0x01];
    pet.extend_from_slice(b"Poring\0");
    pet.resize(2 + 24, 0);
    pet.push(0); // renamed
    pet.extend_from_slice(&10u16.to_le_bytes()); // level
    pet.extend_from_slice(&80u16.to_le_bytes()); // hunger
    pet.extend_from_slice(&500u16.to_le_bytes()); // intimacy
    pet.extend_from_slice(&0u16.to_le_bytes()); // accessory
    pet.extend_from_slice(&1002u16.to_le_bytes()); // job
    assert_eq!(pet.len(), 37);

    let pet = feed(&mut handler, &pet);
    assert!(matches!(
        pet.0.as_slice(),
        [NetworkEvent::PetProperty { property }] if property.name == "Poring" && property.level == 10 && property.job_id.0 == 1002
    ));
}

#[test]
fn known_headers_are_registered() {
    let handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x009C)));
    assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x0B1B)));
    assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x0984)));
    assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x01D0)));
    assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x01E1)));
    assert!(handler.is_registered(ragnarok_packets::PacketHeader(0x0A37)));
    assert!(!handler.is_registered(ragnarok_packets::PacketHeader(0x0FFF)));
}

fn feed(handler: &mut ragnarok_packets::handler::PacketHandler<NetworkEventList, NoPacketCallback>, bytes: &[u8]) -> NetworkEventList {
    let mut reader = ByteReader::without_metadata(bytes);
    let result = handler.process_one(&mut reader);

    match result {
        HandlerResult::Ok(events) => events,
        HandlerResult::UnhandledPacket => panic!("packet 0x{:04X} is not registered", u16::from_le_bytes([bytes[0], bytes[1]])),
        HandlerResult::PacketCutOff => panic!("packet was reported as cut off: {bytes:?}"),
        HandlerResult::InternalError(error) => panic!("packet handler failed: {error:?}"),
    }
}

fn regular_item_entry(raw_index: u16) -> Vec<u8> {
    let mut entry = Vec::new();
    entry.extend_from_slice(&raw_index.to_le_bytes()); // index
    entry.extend_from_slice(&501u32.to_le_bytes()); // item id
    entry.push(0); // item type
    entry.extend_from_slice(&5u16.to_le_bytes()); // amount
    entry.extend_from_slice(&0u32.to_le_bytes()); // equipped position
    entry.extend_from_slice(&[0u8; 16]); // cards
    entry.extend_from_slice(&0u32.to_le_bytes()); // hire expiration
    entry.push(0); // flags
    entry
}

fn list_packet(header: u16, inventory_type: u8, payload: &[u8]) -> Vec<u8> {
    let length = (4 + 1 + payload.len()) as u16;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&header.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.push(inventory_type);
    bytes.extend_from_slice(payload);
    bytes
}

fn list_start(inventory_type: u8, name: &str) -> Vec<u8> {
    let length = (4 + 1 + name.len() + 1) as u16;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0B08u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.push(inventory_type);
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    bytes
}

fn list_end(inventory_type: u8) -> Vec<u8> {
    vec![0x0B, 0x0B, inventory_type, 0]
}

#[test]
fn inventory_and_storage_lists_are_routed_by_inventory_type() {
    // Inventory list (type 0): the raw wire index is already `server slot + 2`,
    // so InventoryIndex reports slot 0.
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    feed(&mut handler, &list_start(0, ""));
    feed(&mut handler, &list_packet(0x0B09, 0, &regular_item_entry(2)));
    let events = feed(&mut handler, &list_end(0));

    assert!(
        matches!(events.0.as_slice(), [NetworkEvent::SetInventory { items }] if items.len() == 1 && items[0].index == ragnarok_packets::InventoryIndex(0))
    );

    // Storage list (type 2): the wire index is `server slot + 1`; a storage
    // slot zero is encoded as raw index 1 and must come out as slot 0.
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    feed(&mut handler, &list_start(2, "Storage"));
    feed(&mut handler, &list_packet(0x0B09, 2, &regular_item_entry(1)));
    let events = feed(&mut handler, &list_end(2));

    assert!(
        matches!(events.0.as_slice(), [NetworkEvent::SetStorage { items, name }] if items.len() == 1 && items[0].index == ragnarok_packets::InventoryIndex(0) && name == "Storage")
    );
}

#[test]
fn cart_lists_are_routed_and_guild_storage_is_ignored() {
    // Cart list (type 1): uses the normal `slot + 2` index encoding.
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    feed(&mut handler, &list_start(1, ""));
    feed(&mut handler, &list_packet(0x0B09, 1, &regular_item_entry(2)));
    let events = feed(&mut handler, &list_end(1));

    assert!(
        matches!(events.0.as_slice(), [NetworkEvent::SetCart { items }] if items.len() == 1 && items[0].index == ragnarok_packets::InventoryIndex(0))
    );

    // Guild storage (type 3) is still not displayed; packets have to be
    // consumed so the stream stays in sync.
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    feed(&mut handler, &list_start(3, "Guild"));
    feed(&mut handler, &list_packet(0x0B09, 3, &regular_item_entry(1)));
    let events = feed(&mut handler, &list_end(3));

    assert!(events.0.is_empty());
}

#[test]
fn storage_add_remove_amount_and_close_packets() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0A0A ZC_ADD_ITEM_TO_STORE, 2019 layout (57 bytes).
    let mut added = Vec::new();
    added.extend_from_slice(&0x0A0Au16.to_le_bytes());
    added.extend_from_slice(&1u16.to_le_bytes()); // storage slot 0 (+1)
    added.extend_from_slice(&5i32.to_le_bytes()); // amount
    added.extend_from_slice(&501u32.to_le_bytes()); // item id
    added.extend_from_slice(&[0, 1, 0, 0]); // type, identified, damaged, refine
    added.extend_from_slice(&[0u8; 16]); // cards
    added.extend_from_slice(&[0u8; 25]); // options
    assert_eq!(added.len(), 57);

    let events = feed(&mut handler, &added);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::StorageItemAdded { item }] if item.index == ragnarok_packets::InventoryIndex(0)));

    // 0x00F2 amount, 0x00F6 remove, 0x00F8 close.
    let mut amount = Vec::new();
    amount.extend_from_slice(&0x00F2u16.to_le_bytes());
    amount.extend_from_slice(&3u16.to_le_bytes());
    amount.extend_from_slice(&300u16.to_le_bytes());
    assert!(matches!(feed(&mut handler, &amount).0.as_slice(), [
        NetworkEvent::SetStorageAmount {
            amount: 3,
            maximum_amount: 300
        }
    ]));

    let mut removed = Vec::new();
    removed.extend_from_slice(&0x00F6u16.to_le_bytes());
    removed.extend_from_slice(&1u16.to_le_bytes());
    removed.extend_from_slice(&2u32.to_le_bytes());
    assert!(matches!(feed(&mut handler, &removed).0.as_slice(), [
        NetworkEvent::StorageItemRemoved {
            storage_index: 0,
            amount: 2
        }
    ]));

    assert!(matches!(feed(&mut handler, &[0xF8, 0x00]).0.as_slice(), [
        NetworkEvent::StorageClosed
    ]));
}

#[test]
fn storage_deposit_and_withdraw_packets_use_the_expected_index_encoding() {
    use ragnarok_packets::{CloseStoragePacket, PacketExt, StorageDepositPacket, StorageWithdrawPacket};

    let mut writer = ByteWriter::new();
    StorageDepositPacket {
        inventory_index: ragnarok_packets::InventoryIndex(0),
        amount: 5,
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    let bytes = writer.into_inner();
    assert_eq!(bytes.len(), 8);
    // 2019 uses 0x07EC for the deposit request.
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x07EC);
    // Inventory index 0 is encoded as 2.
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 2);

    let mut writer = ByteWriter::new();
    StorageWithdrawPacket {
        storage_index: 1,
        amount: 5,
    }
    .packet_to_bytes(&mut writer)
    .unwrap();
    let bytes = writer.into_inner();
    assert_eq!(bytes.len(), 8);
    // 2019 uses 0x0861 for the withdraw request.
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0861);
    // The caller passes the storage slot + 1.
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 1);

    let mut writer = ByteWriter::new();
    CloseStoragePacket {}.packet_to_bytes(&mut writer).unwrap();
    assert_eq!(writer.into_inner(), vec![0x93, 0x01]);
}

#[test]
fn item_add_packet_updates_the_inventory() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0A37 ZC_ITEM_PICKUP_ACK, 69 byte 2019 layout (EQUIPSLOTINFO is u32[4]);
    // index is client_index (slot 2 -> raw 4), a successful add of 7 Red
    // Potions to slot 2.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0A37u16.to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes()); // index
    bytes.extend_from_slice(&7u16.to_le_bytes()); // amount
    bytes.extend_from_slice(&501u32.to_le_bytes()); // item id
    bytes.extend_from_slice(&[1, 0, 0]); // identified, damaged, refine
    bytes.extend_from_slice(&[0u8; 16]); // cards (u32[4])
    bytes.extend_from_slice(&0u32.to_le_bytes()); // equip position
    bytes.extend_from_slice(&[0, 0]); // item type, result
    bytes.extend_from_slice(&0i32.to_le_bytes()); // hire expiration
    bytes.extend_from_slice(&0u16.to_le_bytes()); // bind on equip type
    bytes.extend_from_slice(&[0u8; 25]); // options
    bytes.extend_from_slice(&[0u8, 0, 0]); // favorite + item sprite number
    assert_eq!(bytes.len(), 69);

    let events = feed(&mut handler, &bytes);
    // The 2019 pickup ack adds the item to the inventory and reports the
    // pickup for the notification HUD; the chat line is produced by the
    // client from the same events elsewhere.
    assert!(matches!(events.0.as_slice(), [
        NetworkEvent::IventoryItemAdded { item },
        NetworkEvent::ItemPickedUp { item_id, amount },
    ] if item.index == ragnarok_packets::InventoryIndex(2)
        && *item_id == ragnarok_packets::ItemId(501)
        && *amount == 7));
}

#[test]
fn guild_chat_packet_produces_a_guild_chat_event() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x017F ZC_GUILD_CHAT: header, length, `<name> : <message>` and the
    // trailing NUL (which the declared length includes).
    let message = "Alice : hello guild";
    let length = 4 + message.len() + 1;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x017Fu16.to_le_bytes());
    bytes.extend_from_slice(&(length as u16).to_le_bytes());
    bytes.extend_from_slice(message.as_bytes());
    bytes.push(0);

    let events = feed(&mut handler, &bytes);
    assert!(matches!(events.0.as_slice(), [
        NetworkEvent::GuildChat { message },
    ] if message == "Alice : hello guild"));
}

#[test]
fn display_emotion_packet_preserves_entity_and_emotion() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();
    let packet = DisplayEmotionPacket {
        entity_id: EntityId(0x1234_5678),
        emotion: 3,
    };
    let mut writer = ByteWriter::new();
    packet.packet_to_bytes(&mut writer).unwrap();
    let mut reader = ByteReader::without_metadata(writer.as_slice());

    let HandlerResult::Ok(events) = handler.process_one(&mut reader) else {
        panic!("display-emotion packet was not handled");
    };

    assert!(matches!(events.0.as_slice(), [NetworkEvent::DisplayEmotion {
        entity_id: EntityId(0x1234_5678),
        emotion: 3,
    }]));
}

#[test]
fn cart_add_and_delete_packets_update_the_cart_list() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0A0B ZC_ADD_ITEM_TO_CART (2019 layout, 57 bytes). Raw index 4 is
    // cart slot 2 (`slot + 2` encoding).
    let mut added = Vec::new();
    added.extend_from_slice(&0x0A0Bu16.to_le_bytes());
    added.extend_from_slice(&4u16.to_le_bytes()); // cart slot 2 (+2)
    added.extend_from_slice(&5i32.to_le_bytes()); // amount
    added.extend_from_slice(&501u32.to_le_bytes()); // item id
    added.extend_from_slice(&[0, 1, 0, 0]); // type, identified, damaged, refine
    added.extend_from_slice(&[0u8; 16]); // cards
    added.extend_from_slice(&[0u8; 25]); // options
    assert_eq!(added.len(), 57);

    let events = feed(&mut handler, &added);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::CartItemAdded { item }]
        if item.index == ragnarok_packets::InventoryIndex(2) && item.item_id == ragnarok_packets::ItemId(501)));

    // 0x0125 ZC_DELETE_ITEM_FROM_CART.
    let mut deleted = Vec::new();
    deleted.extend_from_slice(&0x0125u16.to_le_bytes());
    deleted.extend_from_slice(&4u16.to_le_bytes()); // cart slot 2 (+2)
    deleted.extend_from_slice(&2u32.to_le_bytes()); // amount
    assert_eq!(deleted.len(), 8);

    let events = feed(&mut handler, &deleted);
    assert!(
        matches!(events.0.as_slice(), [NetworkEvent::CartItemRemoved { cart_index, amount }]
        if *cart_index == ragnarok_packets::InventoryIndex(2) && *amount == 2)
    );
}

#[test]
fn vending_shop_list_packet_carries_stock_price_and_cart_slot() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0800 ZC_PC_PURCHASE_ITEMLIST_FROMMC: AID, vender id and one 63
    // byte row (clif.cpp:7592).
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0800u16.to_le_bytes());
    bytes.extend_from_slice(&75u16.to_le_bytes()); // length, patched below
    bytes.extend_from_slice(&42u32.to_le_bytes()); // owner account id
    bytes.extend_from_slice(&7u32.to_le_bytes()); // vender id
    bytes.extend_from_slice(&100u32.to_le_bytes()); // price
    bytes.extend_from_slice(&3u16.to_le_bytes()); // stock
    bytes.extend_from_slice(&4u16.to_le_bytes()); // cart slot 2 (+2)
    bytes.push(0); // item type
    bytes.extend_from_slice(&501u32.to_le_bytes()); // item id
    bytes.extend_from_slice(&[1, 0, 0]); // identified, damaged, refine
    bytes.extend_from_slice(&[0u8; 16]); // cards
    bytes.extend_from_slice(&[0u8; 25]); // options
    bytes.extend_from_slice(&0u32.to_le_bytes()); // location
    bytes.extend_from_slice(&0u16.to_le_bytes()); // view sprite
    assert_eq!(bytes.len(), 75);
    bytes[2..4].copy_from_slice(&75u16.to_le_bytes());

    let events = feed(&mut handler, &bytes);
    let [
        NetworkEvent::VendingShopList {
            owner_account_id,
            shop_id,
            items,
        },
    ] = events.0.as_slice()
    else {
        panic!("expected VendingShopList, got {:?}", events.0);
    };
    assert_eq!(owner_account_id.0, 42);
    assert_eq!(*shop_id, 7);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].cart_index, ragnarok_packets::InventoryIndex(2));
    assert_eq!(items[0].item.price.0, 100);
    assert_eq!(items[0].item.quantity, crate::items::ItemQuantity::Fixed(3));

    // 0x0135 ZC_PC_PURCHASE_RESULT_FROMMC.
    let mut result = Vec::new();
    result.extend_from_slice(&0x0135u16.to_le_bytes());
    result.extend_from_slice(&4u16.to_le_bytes());
    result.extend_from_slice(&1u16.to_le_bytes());
    result.push(1); // not enough zeny
    let events = feed(&mut handler, &result);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::VendingPurchaseResult {
        result: 1
    }]));
}

#[test]
fn cart_and_vending_request_packets_use_the_2019_layout() {
    use ragnarok_packets::{
        MoveItemFromCartPacket, MoveItemFromStorageToCartPacket, MoveItemToCartPacket, PacketExt, PurchaseFromShopEntry,
        PurchaseFromShopPacket, RequestVendingListPacket,
    };

    fn encode(packet: impl PacketExt) -> Vec<u8> {
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        writer.into_inner()
    }

    let bytes = encode(MoveItemToCartPacket {
        inventory_index: ragnarok_packets::InventoryIndex(0),
        amount: 5,
    });
    assert_eq!(bytes.len(), 8);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0126);
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 2);

    let bytes = encode(MoveItemFromCartPacket {
        cart_index: ragnarok_packets::InventoryIndex(1),
        amount: 5,
    });
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0127);
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 3);

    let bytes = encode(MoveItemFromStorageToCartPacket {
        storage_index: 4,
        amount: 5,
    });
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0128);
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 4);

    let bytes = encode(RequestVendingListPacket {
        owner_account_id: ragnarok_packets::AccountId(42),
    });
    assert_eq!(bytes, vec![0x30, 0x01, 42, 0, 0, 0]);

    let bytes = encode(PurchaseFromShopPacket {
        owner_account_id: ragnarok_packets::AccountId(42),
        items: vec![PurchaseFromShopEntry {
            amount: 1,
            cart_index: ragnarok_packets::InventoryIndex(2),
        }],
    });
    assert_eq!(bytes.len(), 12);
    assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0134);
    assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 12);
    assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 42);
    assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 1);
    assert_eq!(u16::from_le_bytes([bytes[10], bytes[11]]), 4);
}

/// The Rodex mailbox packets (`ZC_ACK_MAIL_LIST3` 0x0AC2,
/// `ZC_ACK_READ_RODEX` 0x09EB, the claim/delete acks and
/// `ZC_MAIL_WINDOWS` 0x0260) have to reach the client. Before this lane
/// only `ZC_NOTIFY_UNREADMAIL` (0x09E7) was registered, as a noop.
#[test]
fn rodex_mail_packets_reach_the_client() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0AC2 ZC_ACK_MAIL_LIST3: one unread mail with zeny and items.
    let mut row = Vec::new();
    row.push(0); // open type: normal inbox
    row.extend_from_slice(&7u64.to_le_bytes()); // mail id
    row.push(0); // unread
    row.push(0x2 | 0x4); // mail type: zeny + items
    row.extend_from_slice(&fixed_name("Alice", 24)); // sender
    row.extend_from_slice(&3_600u32.to_le_bytes()); // scheduled deletion
    row.extend_from_slice(&6u16.to_le_bytes()); // title length incl. NUL
    row.extend_from_slice(b"Hello\0");

    let mut list = Vec::new();
    list.extend_from_slice(&0x0AC2u16.to_le_bytes());
    list.extend_from_slice(&((5 + row.len()) as u16).to_le_bytes());
    list.push(1); // unknown, always 1
    list.extend_from_slice(&row);

    let events = feed(&mut handler, &list);
    match events.0.as_slice() {
        [NetworkEvent::MailList { mails }] => {
            assert_eq!(mails.len(), 1);
            assert_eq!(mails[0].mail_id, 7);
            assert_eq!(mails[0].read, 0);
            assert_eq!(mails[0].mail_type, 0x2 | 0x4);
            assert_eq!(mails[0].sender, "Alice");
            assert_eq!(mails[0].title, "Hello");
        }
        other => panic!("unexpected events for 0x0AC2: {other:?}"),
    }

    // 0x09EB ZC_ACK_READ_RODEX: body + zeny, no item rows.
    let body = b"(no message)\0";
    let mut read = Vec::new();
    read.extend_from_slice(&0x09EBu16.to_le_bytes());
    read.extend_from_slice(&((24 + body.len()) as u16).to_le_bytes());
    read.push(0); // open type
    read.extend_from_slice(&7u64.to_le_bytes()); // mail id
    read.extend_from_slice(&(body.len() as u16).to_le_bytes()); // text length
    read.extend_from_slice(&1_500u64.to_le_bytes()); // zeny
    read.push(0); // item count
    read.extend_from_slice(body);

    let events = feed(&mut handler, &read);
    match events.0.as_slice() {
        [
            NetworkEvent::MailRead {
                mail_id,
                open_type,
                body,
                zeny,
                items,
            },
        ] => {
            assert_eq!(*mail_id, 7);
            assert_eq!(*open_type, 0);
            assert_eq!(body, "(no message)");
            assert_eq!(*zeny, 1_500);
            assert!(items.is_empty());
        }
        other => panic!("unexpected events for 0x09EB: {other:?}"),
    }

    // 0x09F2 / 0x09F4 claim results and 0x09F6 delete ack.
    let mut claim_zeny = Vec::new();
    claim_zeny.extend_from_slice(&0x09F2u16.to_le_bytes());
    claim_zeny.extend_from_slice(&7u64.to_le_bytes());
    claim_zeny.extend_from_slice(&[0, 0]); // open type, result
    assert_eq!(claim_zeny.len(), 12);
    let events = feed(&mut handler, &claim_zeny);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MailAttachmentClaimed {
        mail_id: 7,
        zeny: true,
        result: 0,
    }]));

    let mut claim_item = Vec::new();
    claim_item.extend_from_slice(&0x09F4u16.to_le_bytes());
    claim_item.extend_from_slice(&7u64.to_le_bytes());
    claim_item.extend_from_slice(&[0, 2]); // open type, result: too heavy
    assert_eq!(claim_item.len(), 12);
    let events = feed(&mut handler, &claim_item);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MailAttachmentClaimed {
        mail_id: 7,
        zeny: false,
        result: 2,
    }]));

    let mut delete = Vec::new();
    delete.extend_from_slice(&0x09F6u16.to_le_bytes());
    delete.push(0); // open type
    delete.extend_from_slice(&7u64.to_le_bytes());
    assert_eq!(delete.len(), 11);
    let events = feed(&mut handler, &delete);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MailDeleted { mail_id: 7 }]));

    // 0x0260 ZC_MAIL_WINDOWS: the server opens the mailbox.
    let mut window = Vec::new();
    window.extend_from_slice(&0x0260u16.to_le_bytes());
    window.extend_from_slice(&0i32.to_le_bytes());
    assert_eq!(window.len(), 6);
    let events = feed(&mut handler, &window);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MailboxWindow { open: true }]));

    // 0x09E7 ZC_NOTIFY_UNREADMAIL is a real event now.
    let events = feed(&mut handler, &[0xE7, 0x09, 1]);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::MailUnreadStatus { unread: true }]));
}

#[test]
fn bank_packets_produce_typed_events() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0AE2 ZC_UI_OPEN with `OUT_UI_BANK` (clif_ui_open, clif.cpp:21916).
    let mut ui_open = Vec::new();
    ui_open.extend_from_slice(&0x0AE2u16.to_le_bytes());
    ui_open.push(0); // OUT_UI_BANK
    ui_open.extend_from_slice(&0u32.to_le_bytes());
    assert_eq!(ui_open.len(), 7);
    let events = feed(&mut handler, &ui_open);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::OpenUserInterface {
        ui_type: 0,
        data: 0
    }]));

    // 0x09B7 ZC_ACK_OPEN_BANKING.
    let events = feed(&mut handler, &[0xB7, 0x09, 0, 0]);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::BankOpened]));

    // 0x09A6 ZC_BANKING_CHECK: money.Q reason.W.
    let mut balance = Vec::new();
    balance.extend_from_slice(&0x09A6u16.to_le_bytes());
    balance.extend_from_slice(&123_456u64.to_le_bytes());
    balance.extend_from_slice(&0u16.to_le_bytes());
    assert_eq!(balance.len(), 12);
    let events = feed(&mut handler, &balance);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::BankBalance {
        money: 123_456,
        reason: 0,
    }]));

    // 0x09A8 ZC_ACK_BANKING_DEPOSIT: reason.W money.Q zeny.L.
    let mut deposit = Vec::new();
    deposit.extend_from_slice(&0x09A8u16.to_le_bytes());
    deposit.extend_from_slice(&0u16.to_le_bytes());
    deposit.extend_from_slice(&500u64.to_le_bytes());
    deposit.extend_from_slice(&1_000i32.to_le_bytes());
    assert_eq!(deposit.len(), 16);
    let events = feed(&mut handler, &deposit);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::BankDepositResult {
        reason: 0,
        money: 500,
        zeny: 1_000,
    }]));
}

/// A successful use-item ack (`ZC_USE_ITEM_ACK` 0x01C8) must update the
/// amount *and* emit `ItemUsed`, which drives the official use-item
/// animation. A failed use only reports the error.
/// `ZC_LONGPAR_CHANGE` (0x00B1) variable 20 is zeny; it must reach the
/// client as `StatType::Zeny` so the inventory row updates.
#[test]
fn zeny_longpar_change_emits_the_zeny_stat() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut packet = Vec::new();
    packet.extend_from_slice(&0x00B1u16.to_le_bytes());
    packet.extend_from_slice(&20u16.to_le_bytes());
    packet.extend_from_slice(&12_345u32.to_le_bytes());
    assert_eq!(packet.len(), 8);

    let events = feed(&mut handler, &packet);
    assert!(
        events.0.iter().any(|event| matches!(event, NetworkEvent::UpdateStat {
            stat_type: ragnarok_packets::StatType::Zeny(12_345)
        })),
        "{:?}",
        events.0
    );
}

#[test]
fn use_item_ack_emits_the_use_item_animation_event() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut used = Vec::new();
    used.extend_from_slice(&0x01C8u16.to_le_bytes());
    used.extend_from_slice(&7u16.to_le_bytes()); // inventory index 5 (+2)
    used.extend_from_slice(&501u32.to_le_bytes()); // item id
    used.extend_from_slice(&1u32.to_le_bytes()); // account id
    used.extend_from_slice(&4i16.to_le_bytes()); // remaining amount
    used.push(1); // result: success
    assert_eq!(used.len(), 15);

    let events = feed(&mut handler, &used);
    assert!(
        events
            .0
            .iter()
            .any(|event| matches!(event, NetworkEvent::ItemUsed { item_id } if item_id.0 == 501))
    );
    assert!(
        events
            .0
            .iter()
            .any(|event| matches!(event, NetworkEvent::InventoryItemAmountChanged { amount: 4, .. }))
    );

    let mut failed = used.clone();
    *failed.last_mut().unwrap() = 0;
    let events = feed(&mut handler, &failed);
    assert!(!events.0.iter().any(|event| matches!(event, NetworkEvent::ItemUsed { .. })));
}

/// The item service packets (refine, repair, identify, card composition)
/// are registered and turn into the typed events the client consumes.
#[test]
fn item_service_packets_emit_typed_events() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x0221 ZC_NOTIFY_WEAPONITEMLIST: length + one 23 byte row.
    let mut refine_list = Vec::new();
    refine_list.extend_from_slice(&0x0221u16.to_le_bytes());
    refine_list.extend_from_slice(&(4u16 + 23).to_le_bytes());
    refine_list.extend_from_slice(&(7u16 + 2).to_le_bytes()); // client index
    refine_list.extend_from_slice(&1500u32.to_le_bytes()); // item id
    refine_list.push(4); // refine
    refine_list.extend_from_slice(&[0u8; 16]); // cards
    assert_eq!(refine_list.len(), 27);
    let events = feed(&mut handler, &refine_list);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::WeaponRefineList { entries }]
        if entries.len() == 1
            && entries[0].index == InventoryIndex(7)
            && entries[0].item_id == ItemId(1500)
            && entries[0].refine == 4));

    // 0x0223 ZC_ACK_WEAPONREFINE: result + item id.
    let mut refine_result = Vec::new();
    refine_result.extend_from_slice(&0x0223u16.to_le_bytes());
    refine_result.extend_from_slice(&0i32.to_le_bytes());
    refine_result.extend_from_slice(&1500u32.to_le_bytes());
    let events = feed(&mut handler, &refine_result);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::WeaponRefineResult {
        result: 0,
        item_id: ItemId(1500),
    }]));

    // 0x01FC ZC_REPAIRITEMLIST: the row index is a raw server slot.
    let mut repair_list = Vec::new();
    repair_list.extend_from_slice(&0x01FCu16.to_le_bytes());
    repair_list.extend_from_slice(&(4u16 + 23).to_le_bytes());
    repair_list.extend_from_slice(&9u16.to_le_bytes()); // raw server slot
    repair_list.extend_from_slice(&1500u32.to_le_bytes());
    repair_list.push(6);
    repair_list.extend_from_slice(&[0u8; 16]);
    let events = feed(&mut handler, &repair_list);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::RepairItemList { entries }]
        if entries.len() == 1 && entries[0].index == 9 && entries[0].refine == 6));

    // 0x01FE ZC_ACK_ITEMREPAIR: client index + result.
    let mut repair_result = Vec::new();
    repair_result.extend_from_slice(&0x01FEu16.to_le_bytes());
    repair_result.extend_from_slice(&(9u16 + 2).to_le_bytes());
    repair_result.push(0);
    let events = feed(&mut handler, &repair_result);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::RepairItemResult {
        index: InventoryIndex(9),
        success: true,
    }]));

    // 0x0177 ZC_ITEMIDENTIFY_LIST: length + client indexes.
    let mut identify_list = Vec::new();
    identify_list.extend_from_slice(&0x0177u16.to_le_bytes());
    identify_list.extend_from_slice(&(4u16 + 4).to_le_bytes());
    identify_list.extend_from_slice(&(2u16 + 2).to_le_bytes());
    identify_list.extend_from_slice(&(5u16 + 2).to_le_bytes());
    let events = feed(&mut handler, &identify_list);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::IdentifyItemList { indexes }]
        if *indexes == vec![InventoryIndex(2), InventoryIndex(5)]));

    // 0x0179 ZC_ACK_ITEMIDENTIFY: client index + result.
    let mut identify_result = Vec::new();
    identify_result.extend_from_slice(&0x0179u16.to_le_bytes());
    identify_result.extend_from_slice(&(5u16 + 2).to_le_bytes());
    identify_result.push(0);
    let events = feed(&mut handler, &identify_result);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::IdentifyItemResult {
        index: InventoryIndex(5),
        success: true,
    }]));

    // 0x017B ZC_ITEMCOMPOSITION_LIST: length + client indexes.
    let mut composition_list = Vec::new();
    composition_list.extend_from_slice(&0x017Bu16.to_le_bytes());
    composition_list.extend_from_slice(&(4u16 + 2).to_le_bytes());
    composition_list.extend_from_slice(&(11u16 + 2).to_le_bytes());
    let events = feed(&mut handler, &composition_list);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::CardCompositionList { indexes }]
        if *indexes == vec![InventoryIndex(11)]));

    // 0x017D ZC_ACK_ITEMCOMPOSITION: equip index + card index + result.
    let mut composition_result = Vec::new();
    composition_result.extend_from_slice(&0x017Du16.to_le_bytes());
    composition_result.extend_from_slice(&(11u16 + 2).to_le_bytes());
    composition_result.extend_from_slice(&(4u16 + 2).to_le_bytes());
    composition_result.push(0);
    let events = feed(&mut handler, &composition_result);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::CardCompositionResult {
        equip_index: InventoryIndex(11),
        card_index: InventoryIndex(4),
        success: true,
    }]));

    // 0x0AA0 ZC_OPEN_REFINING_UI opens the NPC refine UI.
    let events = feed(&mut handler, &[0xA0, 0x0A]);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::RefiningUiOpened]));

    // 0x0AA2 ZC_REFINING_MATERIAL_LIST: client index + blessing + rows.
    let mut refining_list = Vec::new();
    refining_list.extend_from_slice(&0x0AA2u16.to_le_bytes());
    refining_list.extend_from_slice(&(7u16 + 9).to_le_bytes());
    refining_list.extend_from_slice(&(4u16 + 2).to_le_bytes());
    refining_list.push(2);
    refining_list.extend_from_slice(&984u32.to_le_bytes());
    refining_list.push(75);
    refining_list.extend_from_slice(&5000i32.to_le_bytes());
    assert_eq!(refining_list.len(), 16);
    let events = feed(&mut handler, &refining_list);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::RefiningMaterialList {
        item_index: InventoryIndex(4),
        blacksmith_blessing: 2,
        materials,
    }] if materials.len() == 1 && materials[0].item_id == ItemId(984) && materials[0].chance == 75));

    // 0x0A3F ZC_UPDATE_CARDSLOT is parsed (never sent by rAthena 20190605,
    // but it exists in the 2019 registry and must not desync the stream).
    let mut card_slot = Vec::new();
    card_slot.extend_from_slice(&0x0A3Fu16.to_le_bytes());
    card_slot.extend_from_slice(&1i16.to_le_bytes());
    card_slot.extend_from_slice(&2i16.to_le_bytes());
    card_slot.extend_from_slice(&4001u32.to_le_bytes());
    card_slot.push(0);
    assert_eq!(card_slot.len(), 11);
    let events = feed(&mut handler, &card_slot);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::CardSlotUpdated {
        wear_state: 1,
        card_slot: 2,
        item_id: ItemId(4001),
        equip_flag: 0,
    }]));
}

#[test]
fn display_image_packet_emits_display_image() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut bytes = vec![0xB3, 0x01];
    let mut image = [0u8; 64];
    image[..9].copy_from_slice(b"kafra_01\0");
    bytes.extend_from_slice(&image);
    bytes.push(2); // bottom right corner
    assert_eq!(bytes.len(), 67);

    let events = feed(&mut handler, &bytes);
    match events.0.as_slice() {
        [NetworkEvent::DisplayImage { image_name, location }] => {
            assert_eq!(image_name, "kafra_01");
            assert!(matches!(location, ragnarok_packets::ImageLocation::BottomRight));
        }
        other => panic!("unexpected events for 0x01B3: {other:?}"),
    }
}

#[test]
fn read_book_packet_emits_open_book() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut bytes = vec![0x94, 0x02];
    bytes.extend_from_slice(&11017u32.to_le_bytes());
    bytes.extend_from_slice(&3u32.to_le_bytes());
    assert_eq!(bytes.len(), 10);

    let events = feed(&mut handler, &bytes);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::OpenBook {
        book_id: 11017,
        page: 3,
    }]));
}

#[test]
fn pincode_packet_emits_pincode_state() {
    let mut handler =
        NetworkingSystem::create_character_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    let mut bytes = vec![0xB9, 0x08];
    bytes.extend_from_slice(&0x1234u32.to_le_bytes()); // seed
    bytes.extend_from_slice(&77u32.to_le_bytes()); // account id
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PINCODE_ASK
    assert_eq!(bytes.len(), 12);

    let events = feed(&mut handler, &bytes);
    match events.0.as_slice() {
        [
            NetworkEvent::PincodeState {
                pincode_seed,
                account_id,
                state,
            },
        ] => {
            assert_eq!(*pincode_seed, 0x1234);
            assert_eq!(*account_id, AccountId(77));
            assert_eq!(*state, 1);
        }
        other => panic!("unexpected events for 0x08B9: {other:?}"),
    }
}

#[test]
fn chat_room_packets_emit_room_events() {
    let mut handler = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback, SupportedPacketVersion::_20220406).unwrap();

    // 0x00D6 create result.
    let events = feed(&mut handler, &[0xD6, 0x00, 0]);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::ChatRoomCreated { flag: 0 }]));

    // 0x00D7 room announcement.
    let mut announcement = Vec::new();
    announcement.extend_from_slice(&0x00D7u16.to_le_bytes());
    announcement.extend_from_slice(&0u16.to_le_bytes());
    announcement.extend_from_slice(&42u32.to_le_bytes());
    announcement.extend_from_slice(&9u32.to_le_bytes());
    announcement.extend_from_slice(&20u16.to_le_bytes());
    announcement.extend_from_slice(&3u16.to_le_bytes());
    announcement.push(1);
    announcement.extend_from_slice(b"Party");
    let size = announcement.len() as u16;
    announcement[2..4].copy_from_slice(&size.to_le_bytes());

    let events = feed(&mut handler, &announcement);
    match events.0.as_slice() {
        [
            NetworkEvent::ChatRoomAnnounced {
                owner_id,
                chat_id,
                limit,
                users,
                room_type,
                title,
            },
        ] => {
            assert_eq!(*owner_id, AccountId(42));
            assert_eq!(*chat_id, 9);
            assert_eq!(*limit, 20);
            assert_eq!(*users, 3);
            assert_eq!(*room_type, 1);
            assert_eq!(title, "Party");
        }
        other => panic!("unexpected events for 0x00D7: {other:?}"),
    }

    // 0x00DB joined room with two members.
    let mut joined = Vec::new();
    joined.extend_from_slice(&0x00DBu16.to_le_bytes());
    joined.extend_from_slice(&0u16.to_le_bytes());
    joined.extend_from_slice(&9u32.to_le_bytes());
    for (role, name) in [(0u32, "Owner"), (1u32, "Guest")] {
        joined.extend_from_slice(&role.to_le_bytes());
        let mut name_bytes = [0u8; 24];
        name_bytes[..name.len()].copy_from_slice(name.as_bytes());
        joined.extend_from_slice(&name_bytes);
    }
    let size = joined.len() as u16;
    joined[2..4].copy_from_slice(&size.to_le_bytes());

    let events = feed(&mut handler, &joined);
    match events.0.as_slice() {
        [NetworkEvent::ChatRoomEntered { chat_id, members }] => {
            assert_eq!(*chat_id, 9);
            assert_eq!(members.len(), 2);
            assert_eq!(members[0].name, "Owner");
            assert_eq!(members[1].name, "Guest");
        }
        other => panic!("unexpected events for 0x00DB: {other:?}"),
    }

    // 0x00DC member joined.
    let mut new_member = vec![0xDC, 0x00];
    new_member.extend_from_slice(&3u16.to_le_bytes());
    let mut name_bytes = [0u8; 24];
    name_bytes[..5].copy_from_slice(b"Third");
    new_member.extend_from_slice(&name_bytes);
    assert_eq!(new_member.len(), 28);
    let events = feed(&mut handler, &new_member);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::ChatRoomMemberJoined {
        count: 3,
        name,
    }] if name == "Third"));

    // 0x00DD member exit (kicked flag).
    let mut exit = vec![0xDD, 0x00];
    exit.extend_from_slice(&2u16.to_le_bytes());
    let mut name_bytes = [0u8; 24];
    name_bytes[..5].copy_from_slice(b"Third");
    exit.extend_from_slice(&name_bytes);
    exit.push(1);
    assert_eq!(exit.len(), 29);
    let events = feed(&mut handler, &exit);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::ChatRoomMemberLeft {
        count: 2,
        name,
        kicked: true,
    }] if name == "Third"));

    // 0x00E1 owner change.
    let mut role = vec![0xE1, 0x00];
    role.extend_from_slice(&1u32.to_le_bytes());
    let mut name_bytes = [0u8; 24];
    name_bytes[..5].copy_from_slice(b"Guest");
    role.extend_from_slice(&name_bytes);
    assert_eq!(role.len(), 30);
    let events = feed(&mut handler, &role);
    assert!(matches!(events.0.as_slice(), [NetworkEvent::ChatRoomRoleChanged {
        flag: 1,
        name,
    }] if name == "Guest"));
}
