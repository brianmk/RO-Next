//! Central classification of server packets that stay registered as *noops*
//! (parsed and intentionally not turned into a `NetworkEvent`).
//!
//! Every remaining noop registration **must** go through
//! [`register_ignored_noop`], which only accepts packets listed in
//! [`IGNORED_NOOPS`]. The contract test
//! `packet_handlers::every_ignored_noop_is_registered_with_a_reason` and the
//! source check `version_module_registers_no_bare_noops` enforce that:
//!
//! * no bare `register_noop` call exists in the version module,
//! * every table entry is registered for its server kind,
//! * every entry has a non-empty reason and classification,
//! * packets owned by another live workstream name their owner.
//!
//! Dead 2022-era packets are removed instead of ignored (see the noop report).

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};

use crate::event::NetworkEventList;

/// Which server connection the packet belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoopServer {
    Character,
    Map,
}

/// Why a packet is deliberately not consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreClass {
    /// Duplicate of authoritative data the client already consumes.
    Duplicate,
    /// Server system that CondaRO does not use / no UI exists for.
    SystemUnused,
    /// Feature work out of this task's scope (tracked separately).
    FeatureOutOfScope,
    /// Nothing the client can act on (empty payload, disabled server feature,
    /// no equivalent client setting).
    NoClientAction,
    /// Owned by another live workstream; that lane replaces this noop.
    OwnedElsewhere(&'static str),
}

/// One classified, still-noop packet.
#[derive(Debug, Clone, Copy)]
pub struct IgnoredNoop {
    pub header: u16,
    #[allow(dead_code)]
    pub server: NoopServer,
    pub class: IgnoreClass,
    pub reason: &'static str,
}

/// Packets that stay registered as noops, with the reason and the owning lane
/// (if any).
pub const IGNORED_NOOPS: &[IgnoredNoop] = &[
    // ------------------------------------------------------- character server
    IgnoredNoop {
        header: 0x020D,
        server: NoopServer::Character,
        class: IgnoreClass::NoClientAction,
        reason: "HC_BLOCK_CHARACTER: ban list; bans are reported through LoginFailedPacket/LoginFailedPacket2",
    },
    IgnoredNoop {
        header: 0x08B9,
        server: NoopServer::Character,
        class: IgnoreClass::NoClientAction,
        reason: "HC_PINCODE: server has pincode_enabled=no and the client has no pincode UI",
    },
    IgnoredNoop {
        header: 0x099D,
        server: NoopServer::Character,
        class: IgnoreClass::Duplicate,
        reason: "HC_ACK_CHARINFO_PER_PAGE: duplicate of the authoritative 0x006B character list",
    },
    IgnoredNoop {
        header: 0x09A0,
        server: NoopServer::Character,
        class: IgnoreClass::NoClientAction,
        reason: "HC_ACK_CHARINFO_PER_PAGE slot page count: character selection has no paging UI",
    },
    IgnoredNoop {
        header: 0x0B18,
        server: NoopServer::Character,
        class: IgnoreClass::OwnedElsewhere("item-descriptions"),
        reason: "ZC_EXTEND_BODYITEM_SIZE: inventory expand flag, owned by the item-descriptions lane",
    },
    // ------------------------------------------------------------ map server
    IgnoredNoop {
        header: 0x0283,
        server: NoopServer::Map,
        class: IgnoreClass::NoClientAction,
        reason: "ZC_AUTH_OK_GID: re-confirms the player block id that the client already knows from login; parsed for framing",
    },
    IgnoredNoop {
        header: 0x0B1B,
        server: NoopServer::Map,
        class: IgnoreClass::NoClientAction,
        reason: "ZC_NOTIFY_ACTORINIT: empty map-load confirmation, nothing to store",
    },
    IgnoredNoop {
        header: 0x02D9,
        server: NoopServer::Map,
        class: IgnoreClass::NoClientAction,
        reason: "ZC_CONFIG_NOTIFY: server config notification without a client equivalent setting",
    },
    IgnoredNoop {
        header: 0x0A9B,
        server: NoopServer::Map,
        class: IgnoreClass::OwnedElsewhere("item-descriptions"),
        reason: "ZC_EQUIPSWITCH_LIST: equip-switch window, owned by the item-descriptions lane",
    },
    IgnoredNoop {
        header: 0x01B3,
        server: NoopServer::Map,
        class: IgnoreClass::OwnedElsewhere("item-descriptions"),
        reason: "ZC_SHOW_IMAGE: server image window, owned by the item-descriptions lane",
    },
    // Trade is a separate feature (two-sided window + state + c2s packets).
    IgnoredNoop {
        header: 0x00EA,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_ACK_ADD_EXCHANGE_ITEM: trade window is a separate feature (no trade UI/state yet)",
    },
    IgnoredNoop {
        header: 0x00EC,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_CONCLUDE_EXCHANGE_ITEM: trade window is a separate feature",
    },
    IgnoredNoop {
        header: 0x00EE,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_CANCEL_EXCHANGE_ITEM: trade window is a separate feature",
    },
    IgnoredNoop {
        header: 0x00F0,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_EXEC_EXCHANGE_ITEM: trade window is a separate feature",
    },
    IgnoredNoop {
        header: 0x00F1,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_EXCHANGEITEM_UNDO: trade window is a separate feature",
    },
    IgnoredNoop {
        header: 0x01F4,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_REQ_EXCHANGE_ITEM: trade window is a separate feature",
    },
    IgnoredNoop {
        header: 0x01F5,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_ACK_EXCHANGE_ITEM: trade window is a separate feature",
    },
    // Party booking / LFG is not used by CondaRO's party flow.
    IgnoredNoop {
        header: 0x0AE7,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_PARTY_REQ_MASTER_TO_JOIN: LFG party-booking UI not implemented (normal party flow is)",
    },
    IgnoredNoop {
        header: 0x0AFA,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_PARTY_JOIN_REQ_ACK_FROM_MASTER: LFG party-booking UI not implemented",
    },
    // Systems CondaRO does not run / has no UI for.
    IgnoredNoop {
        header: 0x0A23,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_ACK_ACHIEVEMENT_LIST: achievement system/UI not used on CondaRO",
    },
    IgnoredNoop {
        header: 0x0A24,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_ACHIEVEMENT_UPDATE: achievement system/UI not used on CondaRO",
    },
    IgnoredNoop {
        header: 0x098A,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_CLANINFO: clan system/UI not used on CondaRO",
    },
    IgnoredNoop {
        header: 0x0988,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_NOTIFY_CLAN_CONNECTINFO: clan system/UI not used on CondaRO",
    },
    IgnoredNoop {
        header: 0x0B8D,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_REPUTE_INFO: reputation/ranking system not used on CondaRO",
    },
    IgnoredNoop {
        header: 0x09E7,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_NOTIFY_NEW_MAIL: mail system/mailbox UI not used on CondaRO",
    },
    IgnoredNoop {
        header: 0x0B7A,
        server: NoopServer::Map,
        class: IgnoreClass::SystemUnused,
        reason: "ZC_NPC_MARKET_OPEN: NPC market UI not used on CondaRO (classic shop path is implemented)",
    },
    // Quest log is a separate feature; CondaRO quests are dialog-driven.
    IgnoredNoop {
        header: 0x09F8,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_ALL_QUEST_LIST: quest log window not implemented (dialog-only quests)",
    },
    IgnoredNoop {
        header: 0x09F9,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_QUEST_NOTIFY: quest log window not implemented (dialog-only quests)",
    },
    IgnoredNoop {
        header: 0x09FA,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_HUNTING_QUEST_UPDATE_OBJECTIVE: quest log window not implemented",
    },
    IgnoredNoop {
        header: 0x08FE,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_HUNTING_QUEST_NOTIFY: quest log window not implemented",
    },
    IgnoredNoop {
        header: 0x02B4,
        server: NoopServer::Map,
        class: IgnoreClass::FeatureOutOfScope,
        reason: "ZC_QUEST_REMOVED: quest log window not implemented",
    },
];

/// Register a packet that is deliberately only parsed.
///
/// Panics when the packet is not in [`IGNORED_NOOPS`], so a noop can never be
/// added without a documented classification. This is a start-up invariant:
/// the version module is a compile-time constant and the contract tests
/// (`unclassified_noop_is_rejected`) run it before any release. The panic is
/// therefore an explicit exception to the "no panics in the networking crate"
/// rule instead of being silently tolerated.
#[expect(
    clippy::panic,
    reason = "start-up invariant: a noop registration without a documented classification is a build bug caught by the contract tests"
)]
pub fn register_ignored_noop<Callback, Packet>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
    Packet: ragnarok_packets::Packet,
{
    let entry = IGNORED_NOOPS
        .iter()
        .find(|entry| entry.header == Packet::HEADER.0)
        .unwrap_or_else(|| panic!("noop packet 0x{:04X} is not classified in IGNORED_NOOPS", Packet::HEADER.0));
    debug_assert!(
        !entry.reason.is_empty(),
        "noop packet 0x{:04X} has an empty reason",
        entry.header
    );
    debug_assert!(
        !matches!(entry.class, IgnoreClass::OwnedElsewhere(owner) if owner.is_empty()),
        "owned noop packet 0x{:04X} has an empty owner",
        entry.header
    );

    packet_handler.register_noop::<Packet>()
}
