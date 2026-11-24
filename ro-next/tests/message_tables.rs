//! Regression tests for the data-driven protocol result-code message table.
//!
//! These are repo-level guards, separate from the schema/rendering tests inside
//! the `ro-next-messages` crate:
//!
//! * the hardcoded `match` arms that mapped result codes to English strings are
//!   gone from the client and networking sources (the texts live in
//!   `ro-next/archive/data/client/messages.ron` now),
//! * the result handlers go through the message table API,
//! * the `--check-messages` entry point validates the committed data and
//!   rejects malformed files.

/// The client-side result handlers (`lib.rs` of this crate).
const LIB_RS: &str = include_str!("../src/lib.rs");
/// The login packet handlers that used to hold the 26 login mappings.
const VERSION_RS: &str = include_str!("../../ro-next-networking/src/packet_versions/version_20220406.rs");
/// The generated data file the handlers render from.
const MESSAGES_RON: &str = include_str!("../archive/data/client/messages.ron");

/// English message literals that used to be hardcoded in the result-code
/// `match` arms (map-server packets in `lib.rs`, login packets in
/// `version_20220406.rs`). None of them may come back to Rust source: the
/// texts belong in the data file.
const REMOVED_HARDCODED_MESSAGES: &[&str] = &[
    "left the party.",
    "was expelled from the party.",
    "rejected the party invitation.",
    "accepted the party invitation.",
    "The party is full.",
    "blocks party invitations.",
    "where parties cannot be joined.",
    "You cannot invite on this map.",
    "Party changes are not possible in a memorial dungeon.",
    "does not meet the party level restrictions.",
    "Party created.",
    "A party with that name already exists.",
    "You cannot create a party on this map.",
    "You do not have enough zeny.",
    "You are over your weight limit.",
    "You have too many items.",
    "You cannot hold that many of this item.",
    "Open-air shop purchases have to be traded through RODEX.",
    "The exchange failed.",
    "The exchange was well done.",
    "The item is already sold out.",
    "There are not enough goods to exchange.",
    "skill level is too low",
    "not enough SP",
    "not enough HP",
    "not enough items",
    "skill is still in delay",
    "not enough zeny",
    "wrong weapon",
    "missing red gemstone",
    "missing blue gemstone",
    "too much weight",
    "invalid target",
    "too many ancilla",
    "missing holy water",
    "missing ancilla",
    "skill is already active",
    "condition not met",
    "invalid position",
    "the skill cannot be used",
    "You cannot carry that much ammunition (weight limit).",
    "Ammunition equipped.",
    "You have to equip proper ammunition first.",
    "the server was closed",
    "this account is already logged in",
    "connection timed out",
    "the server is full",
    "account restriction",
    "the server still recognizes the previous connection, please try again",
    "too many connections from this IP",
    "the server closed the connection",
    "Server closed",
    "Already online",
    "Unregistered id",
    "Incorrect password",
    "Id has expired",
    "Blocked by gm team",
    "Game outdated",
    "You are prohibited to log in until",
    "Company account limit reached",
    "Banned by DBA team",
    "Email not confirmed",
    "Banned by GM team",
    "Working in DB",
    "Self lock",
    "Not Permitted Group",
    "This ID has been totally erased",
    "Login information remains at",
    "Account has been locked for a hacking investigation",
    "temporarily prohibited from login due to a bug-related investigation",
    "This character is being deleted.",
    "Unknown error",
];

/// Client-side guards that run *before* a request is sent, so the official
/// client never receives them as a result code and the message table has no
/// row for them. They are listed here so the check below stays about result
/// codes only.
const CLIENT_SIDE_GUARDS: &[&str] = &["You do not have enough zeny."];

#[test]
fn hardcoded_result_message_arms_are_gone() {
    for (name, source) in [("ro-next/src/lib.rs", LIB_RS), ("version_20220406.rs", VERSION_RS)] {
        for message in REMOVED_HARDCODED_MESSAGES {
            if CLIENT_SIDE_GUARDS.contains(message) {
                continue;
            }
            assert!(
                !source.contains(message),
                "{name} still contains the hardcoded result message {message:?}; it belongs in messages.ron"
            );
        }
    }
}

#[test]
fn result_handlers_go_through_the_message_table() {
    assert!(
        LIB_RS.matches("format_message(").count() >= 7,
        "the map-server result handlers have to format through format_message"
    );
    for domain in [
        "MessageDomain::PartyMemberRemoved",
        "MessageDomain::PartyInviteReply",
        "MessageDomain::PartyCreation",
        "MessageDomain::BuyItem",
        "MessageDomain::SkillUseFailed",
        "MessageDomain::AmmunitionAction",
        "MessageDomain::MapServerBan",
    ] {
        assert!(LIB_RS.contains(domain), "lib.rs does not use {domain}");
    }

    assert!(
        VERSION_RS.matches("format_message(").count() >= 3,
        "the login result handlers have to format through format_message"
    );
    assert!(
        VERSION_RS.contains("login_failed_reason2_code"),
        "missing AC_REFUSE_LOGIN2 code mapping"
    );
}

#[test]
fn the_committed_data_file_is_the_message_source() {
    for domain in [
        "PartyMemberRemoved",
        "PartyInviteReply",
        "PartyCreation",
        "BuyItem",
        "SkillUseFailed",
        "AmmunitionAction",
        "MapServerBan",
        "LoginFailed",
        "LoginFailed2",
    ] {
        assert!(
            MESSAGES_RON.contains(&format!("domain: {domain},")),
            "messages.ron misses {domain}"
        );
    }
    assert!(MESSAGES_RON.contains("provenance:"), "messages.ron has no provenance line");
    assert!(
        MESSAGES_RON.contains("display: Current"),
        "messages.ron does not carry the P0 display mode"
    );
    assert!(
        MESSAGES_RON.contains("official: Some("),
        "messages.ron does not carry the official msgstringtable texts"
    );
}

#[test]
fn check_messages_validates_the_compiled_data_and_rejects_malformed_files() {
    let rows = ro_next::check_messages(None).expect("the compiled message data has to be valid");
    assert_eq!(rows, 87, "the committed table has one row per result code of the 10 domains");

    let path = std::env::temp_dir().join(format!("ro-next-messages-check-{}.ron", std::process::id()));
    std::fs::write(&path, "( this is not a message table )").expect("failed to write the malformed fixture");
    let result = ro_next::check_messages(Some(&path));
    std::fs::remove_file(&path).ok();
    assert!(result.is_err(), "check_messages accepted a malformed data file");

    assert!(
        ro_next::check_messages(Some(std::path::Path::new("/nonexistent/messages.ron"))).is_err(),
        "check_messages has to report unreadable files"
    );
}
