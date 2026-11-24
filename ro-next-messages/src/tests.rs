//! Tests for the data-driven protocol result-code message table.

use super::{
    FALLBACK, MessageDisplay, MessageDomain, format_message, format_with, parse, placeholder_names, render_template, row_count, validate,
    validate_compiled,
};

/// Every result code the client used to handle with a hardcoded `match` arm,
/// the call-site arguments it supplied and the exact string it rendered. This
/// is the behaviour-preservation snapshot: the data-driven table has to render
/// byte-for-byte the same texts (81 mappings: 4 party-member removed, 11 party
/// invitation replies, 4 party creations, 9 shop purchases, 17 skill failures,
/// 4 ammunition actions, 7 map-server bans, 3 login-server failures and 22
/// login-server refusals). If one of these changes, it has to be an explicit
/// localization decision, not a refactor side effect.
const PREVIOUS_MESSAGES: &[(MessageDomain, u16, &[(&str, &str)], &str)] = &[
    (
        MessageDomain::PartyMemberRemoved,
        0,
        &[("character_name", "Sora")],
        "Sora left the party.",
    ),
    (
        MessageDomain::PartyMemberRemoved,
        1,
        &[("character_name", "Sora")],
        "Sora was expelled from the party.",
    ),
    (
        MessageDomain::PartyMemberRemoved,
        2,
        &[("character_name", "Sora")],
        "Sora could not leave the party.",
    ),
    (
        MessageDomain::PartyMemberRemoved,
        3,
        &[("character_name", "Sora")],
        "Sora could not be expelled from the party.",
    ),
    (
        MessageDomain::PartyInviteReply,
        0,
        &[("character_name", "Sora")],
        "Sora is already in a party.",
    ),
    (
        MessageDomain::PartyInviteReply,
        1,
        &[("character_name", "Sora")],
        "Sora rejected the party invitation.",
    ),
    (
        MessageDomain::PartyInviteReply,
        2,
        &[("character_name", "Sora")],
        "Sora accepted the party invitation.",
    ),
    (MessageDomain::PartyInviteReply, 3, &[], "The party is full."),
    (
        MessageDomain::PartyInviteReply,
        4,
        &[("character_name", "Sora")],
        "A character of Sora's account is already in the party.",
    ),
    (
        MessageDomain::PartyInviteReply,
        5,
        &[("character_name", "Sora")],
        "Sora blocks party invitations.",
    ),
    (
        MessageDomain::PartyInviteReply,
        7,
        &[("character_name", "Sora")],
        "Sora is not online.",
    ),
    (
        MessageDomain::PartyInviteReply,
        8,
        &[("character_name", "Sora")],
        "Sora is on a map where parties cannot be joined.",
    ),
    (MessageDomain::PartyInviteReply, 9, &[], "You cannot invite on this map."),
    (
        MessageDomain::PartyInviteReply,
        10,
        &[],
        "Party changes are not possible in a memorial dungeon.",
    ),
    (
        MessageDomain::PartyInviteReply,
        11,
        &[("character_name", "Sora")],
        "Sora does not meet the party level restrictions.",
    ),
    (MessageDomain::PartyCreation, 0, &[], "Party created."),
    (MessageDomain::PartyCreation, 1, &[], "A party with that name already exists."),
    (MessageDomain::PartyCreation, 2, &[], "You are already in a party."),
    (MessageDomain::PartyCreation, 3, &[], "You cannot create a party on this map."),
    (MessageDomain::BuyItem, 1, &[], "You do not have enough zeny."),
    (MessageDomain::BuyItem, 2, &[], "You are over your weight limit."),
    (MessageDomain::BuyItem, 3, &[], "You have too many items."),
    (MessageDomain::BuyItem, 9, &[], "You cannot hold that many of this item."),
    (
        MessageDomain::BuyItem,
        10,
        &[],
        "Open-air shop purchases have to be traded through RODEX.",
    ),
    (MessageDomain::BuyItem, 11, &[], "The exchange failed."),
    (MessageDomain::BuyItem, 12, &[], "The exchange was well done."),
    (MessageDomain::BuyItem, 13, &[], "The item is already sold out."),
    (MessageDomain::BuyItem, 14, &[], "There are not enough goods to exchange."),
    (
        MessageDomain::SkillUseFailed,
        0,
        &[("skill_id", "5")],
        "Cannot use skill 5: skill level is too low.",
    ),
    (
        MessageDomain::SkillUseFailed,
        1,
        &[("skill_id", "5")],
        "Cannot use skill 5: not enough SP.",
    ),
    (
        MessageDomain::SkillUseFailed,
        2,
        &[("skill_id", "5")],
        "Cannot use skill 5: not enough HP.",
    ),
    (
        MessageDomain::SkillUseFailed,
        3,
        &[("skill_id", "5")],
        "Cannot use skill 5: not enough items.",
    ),
    (
        MessageDomain::SkillUseFailed,
        4,
        &[("skill_id", "5")],
        "Cannot use skill 5: skill is still in delay.",
    ),
    (
        MessageDomain::SkillUseFailed,
        5,
        &[("skill_id", "5")],
        "Cannot use skill 5: not enough zeny.",
    ),
    (
        MessageDomain::SkillUseFailed,
        6,
        &[("skill_id", "5")],
        "Cannot use skill 5: wrong weapon.",
    ),
    (
        MessageDomain::SkillUseFailed,
        7,
        &[("skill_id", "5")],
        "Cannot use skill 5: missing red gemstone.",
    ),
    (
        MessageDomain::SkillUseFailed,
        8,
        &[("skill_id", "5")],
        "Cannot use skill 5: missing blue gemstone.",
    ),
    (
        MessageDomain::SkillUseFailed,
        9,
        &[("skill_id", "5")],
        "Cannot use skill 5: too much weight.",
    ),
    (
        MessageDomain::SkillUseFailed,
        11,
        &[("skill_id", "5")],
        "Cannot use skill 5: invalid target.",
    ),
    (
        MessageDomain::SkillUseFailed,
        12,
        &[("skill_id", "5")],
        "Cannot use skill 5: too many ancilla.",
    ),
    (
        MessageDomain::SkillUseFailed,
        13,
        &[("skill_id", "5")],
        "Cannot use skill 5: missing holy water.",
    ),
    (
        MessageDomain::SkillUseFailed,
        14,
        &[("skill_id", "5")],
        "Cannot use skill 5: missing ancilla.",
    ),
    (
        MessageDomain::SkillUseFailed,
        22,
        &[("skill_id", "5")],
        "Cannot use skill 5: skill is already active.",
    ),
    (
        MessageDomain::SkillUseFailed,
        23,
        &[("skill_id", "5")],
        "Cannot use skill 5: condition not met.",
    ),
    (
        MessageDomain::SkillUseFailed,
        26,
        &[("skill_id", "5")],
        "Cannot use skill 5: invalid position.",
    ),
    (
        MessageDomain::AmmunitionAction,
        0,
        &[],
        "You have to equip proper ammunition first.",
    ),
    (
        MessageDomain::AmmunitionAction,
        1,
        &[],
        "You cannot carry that much ammunition (weight limit).",
    ),
    (
        MessageDomain::AmmunitionAction,
        2,
        &[],
        "You cannot carry that much ammunition (weight limit).",
    ),
    (MessageDomain::AmmunitionAction, 3, &[], "Ammunition equipped."),
    (MessageDomain::MapServerBan, 1, &[], "Disconnected: the server was closed."),
    (
        MessageDomain::MapServerBan,
        2,
        &[],
        "Disconnected: this account is already logged in.",
    ),
    (MessageDomain::MapServerBan, 3, &[], "Disconnected: connection timed out."),
    (MessageDomain::MapServerBan, 4, &[], "Disconnected: the server is full."),
    (MessageDomain::MapServerBan, 5, &[], "Disconnected: account restriction."),
    (
        MessageDomain::MapServerBan,
        8,
        &[],
        "Disconnected: the server still recognizes the previous connection, please try again.",
    ),
    (
        MessageDomain::MapServerBan,
        9,
        &[],
        "Disconnected: too many connections from this IP.",
    ),
    (MessageDomain::LoginFailed, 1, &[], "Server closed"),
    (MessageDomain::LoginFailed, 2, &[], "Someone has already logged in with this id"),
    (MessageDomain::LoginFailed, 8, &[], "Already online"),
    (MessageDomain::LoginFailed2, 0, &[], "Unregistered id"),
    (MessageDomain::LoginFailed2, 1, &[], "Incorrect password"),
    (MessageDomain::LoginFailed2, 2, &[], "Id has expired"),
    (MessageDomain::LoginFailed2, 3, &[], "Rejected from server"),
    (MessageDomain::LoginFailed2, 4, &[], "Blocked by gm team"),
    (MessageDomain::LoginFailed2, 5, &[], "Game outdated"),
    (
        MessageDomain::LoginFailed2,
        6,
        &[("date", "2026-01-01")],
        "You are prohibited to log in until 2026-01-01.",
    ),
    (MessageDomain::LoginFailed2, 7, &[], "Server is full"),
    (MessageDomain::LoginFailed2, 8, &[], "Company account limit reached"),
    (MessageDomain::LoginFailed2, 9, &[], "Banned by DBA team"),
    (MessageDomain::LoginFailed2, 10, &[], "Email not confirmed"),
    (MessageDomain::LoginFailed2, 11, &[], "Banned by GM team"),
    (MessageDomain::LoginFailed2, 12, &[], "Working in DB"),
    (MessageDomain::LoginFailed2, 13, &[], "Self lock"),
    (MessageDomain::LoginFailed2, 14, &[], "Not Permitted Group"),
    (MessageDomain::LoginFailed2, 99, &[], "This ID has been totally erased"),
    (
        MessageDomain::LoginFailed2,
        100,
        &[("date", "2026-01-01")],
        "Login information remains at 2026-01-01",
    ),
    (
        MessageDomain::LoginFailed2,
        101,
        &[],
        "Account has been locked for a hacking investigation. Please contact the GM Team for more information",
    ),
    (
        MessageDomain::LoginFailed2,
        102,
        &[],
        "This account has been temporarily prohibited from login due to a bug-related investigation",
    ),
    (
        MessageDomain::LoginFailed2,
        103,
        &[],
        "This character is being deleted. Login is temporarily unavailable for the time being",
    ),
    (
        MessageDomain::LoginFailed2,
        104,
        &[],
        "This character is being deleted. Login is temporarily unavailable for the time being",
    ),
    (MessageDomain::LoginFailed2, 105, &[], "Unknown error"),
];

/// Minimal valid document with one row per domain; malformed documents are
/// derived from it so a test failure is about one specific rule.
fn minimal() -> String {
    r#"(
    provenance: "test fixture",
    version: 1,
    domains: [
        (domain: PartyMemberRemoved, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 0, key: "party.member_removed.left", msg_id: None, official: None, default: "left {character_name}", args: ["character_name"], display: Current),
        ]),
        (domain: PartyInviteReply, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 0, key: "party.invite.rejected", msg_id: Some(81), official: Some("Request for party rejected."), default: "rejected {character_name}", args: ["character_name"], display: Current),
        ]),
        (domain: PartyCreation, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 0, key: "party.creation.created", msg_id: Some(77), official: Some("Party has successfully been organized."), default: "created", args: [], display: Current),
        ]),
        (domain: BuyItem, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 1, key: "shop.buy.not_enough_zeny", msg_id: Some(55), official: Some("You do not have enough zeny."), default: "no zeny", args: [], display: Current),
        ]),
        (domain: SkillUseFailed, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 0, key: "skill.fail.level_too_low", msg_id: None, official: None, default: "level {skill_id}", args: ["skill_id"], display: Current),
        ]),
        (domain: AmmunitionAction, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 0, key: "ammunition.equip_proper_first", msg_id: Some(242), official: Some("Please equip the proper ammunition first."), default: "ammo", args: [], display: Current),
        ]),
        (domain: MapServerBan, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 1, key: "map_ban.server_closed", msg_id: Some(4), official: Some("Server Closed."), default: "closed", args: [], display: Current),
        ]),
        (domain: LoginFailed, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 1, key: "login.failed.server_closed", msg_id: Some(4), official: Some("Server Closed."), default: "closed", args: [], display: Current),
        ]),
        (domain: LoginFailed2, unknown: "unknown {code}", unknown_args: [], rows: [
            (code: 0, key: "login.failed2.unregistered_id", msg_id: Some(6), official: Some("Unregistered ID. Please make sure you have a registered account and you have correctly typed in the user ID."), default: "unregistered", args: [], display: Current),
        ]),
    ],
)"#
        .to_owned()
}

fn malformed(needle: &str, replacement: &str) -> String {
    let valid = minimal();
    assert!(valid.contains(needle), "fixture for the malformed case is stale: {needle:?}");
    valid.replacen(needle, replacement, 1)
}

#[test]
fn compiled_fallback_is_valid_and_covers_every_previous_mapping() {
    let rows = validate_compiled().expect("the compiled message table has to be valid");
    assert_eq!(rows, 81, "the committed table has exactly the 81 previous mappings");
    assert_eq!(rows, PREVIOUS_MESSAGES.len(), "every previous mapping needs exactly one row");
    assert_eq!(row_count(), rows);

    for (domain, code, ..) in PREVIOUS_MESSAGES {
        assert!(super::row(*domain, *code).is_some(), "missing row {} code {code}", domain.key());
    }
    for domain in MessageDomain::ALL {
        assert!(!super::rows(domain).is_empty(), "domain '{}' has no rows", domain.key());
    }
    assert!(!super::provenance().is_empty());
}

#[test]
fn every_previous_hardcoded_message_renders_identically() {
    let table = parse(FALLBACK).expect("the compiled message table has to parse");
    for (domain, code, args, expected) in PREVIOUS_MESSAGES {
        let rendered = format_with(&table, *domain, *code, args);
        assert_eq!(rendered, *expected, "the rendered text of {} code {code} changed", domain.key());
    }
}

#[test]
fn p0_renders_the_current_text_for_every_row() {
    // P0 acceptance: no user-facing behaviour change. Flipping a row to
    // `Official` is the reviewed localization pass, not part of this change.
    for domain in MessageDomain::ALL {
        for row in super::rows(domain) {
            assert_eq!(
                row.display,
                MessageDisplay::Current,
                "row {} ({} code {}) must render the current text in P0",
                row.key,
                domain.key(),
                row.code
            );
        }
    }
}

#[test]
fn missing_code_uses_the_documented_fallback() {
    let table = parse(FALLBACK).expect("the compiled message table has to parse");

    let cases: &[(MessageDomain, u16, &[(&str, &str)], &str)] = &[
        (
            MessageDomain::PartyMemberRemoved,
            4,
            &[("character_name", "Sora")],
            "Party member Sora left the party.",
        ),
        (
            MessageDomain::PartyInviteReply,
            12,
            &[("character_name", "Sora")],
            "Party invitation result: 12.",
        ),
        (MessageDomain::PartyCreation, 250, &[], "Party creation failed (result 250)."),
        (MessageDomain::BuyItem, 250, &[], "Shop purchase failed (code 250)."),
        (
            MessageDomain::SkillUseFailed,
            200,
            &[("skill_id", "5")],
            "Cannot use skill 5: the skill cannot be used.",
        ),
        (
            MessageDomain::AmmunitionAction,
            250,
            &[],
            "Unknown ammunition action (code 250).",
        ),
        (
            MessageDomain::MapServerBan,
            11,
            &[],
            "Disconnected: the server closed the connection.",
        ),
        (MessageDomain::LoginFailed, 250, &[], "Login failed (code 250)."),
        (MessageDomain::LoginFailed2, 250, &[], "Login failed (code 250)."),
    ];

    for (domain, code, args, expected) in cases {
        assert_eq!(
            format_with(&table, *domain, *code, args),
            *expected,
            "{} fallback",
            domain.key()
        );
        // The public entry point uses the installed/compiled table.
        assert_eq!(format_message(*domain, *code, args), *expected);
    }
}

#[test]
fn malformed_documents_are_rejected() {
    let valid = minimal();
    assert!(validate(&valid).is_ok());

    let cases: &[(&str, String)] = &[
        (
            "unknown field",
            malformed(r#"default: "created", args: []"#, r#"default: "created", args: [], extra: 1"#),
        ),
        ("unknown version", malformed("version: 1,", "version: 99,")),
        ("unknown domain", malformed("domain: PartyCreation,", "domain: NotADomain,")),
        ("duplicate domain", malformed("domain: PartyCreation,", "domain: BuyItem,")),
        (
            "unsorted rows",
            malformed(
                r#"(code: 0, key: "party.creation.created", msg_id: Some(77), official: Some("Party has successfully been organized."), default: "created", args: [], display: Current),"#,
                r#"(code: 1, key: "party.creation.created", msg_id: None, official: None, default: "created", args: [], display: Current),
            (code: 0, key: "party.creation.created_second", msg_id: None, official: None, default: "created", args: [], display: Current),"#,
            ),
        ),
        (
            "duplicate code",
            malformed(
                r#"(code: 0, key: "party.creation.created", msg_id: Some(77), official: Some("Party has successfully been organized."), default: "created", args: [], display: Current),"#,
                r#"(code: 0, key: "party.creation.created", msg_id: None, official: None, default: "created", args: [], display: Current),
            (code: 0, key: "party.creation.created_second", msg_id: None, official: None, default: "created", args: [], display: Current),"#,
            ),
        ),
        (
            "duplicate key",
            malformed(r#"key: "party.invite.rejected""#, r#"key: "party.creation.created""#),
        ),
        ("empty default", malformed(r#"default: "created""#, r#"default: """#)),
        (
            "undeclared argument",
            malformed(
                r#"default: "left {character_name}", args: ["character_name"]"#,
                r#"default: "left {nobody}", args: ["character_name"]"#,
            ),
        ),
        (
            "official without msg id",
            malformed(
                r#"msg_id: Some(81), official: Some("Request for party rejected.")"#,
                r#"msg_id: None, official: Some("Request for party rejected.")"#,
            ),
        ),
        (
            "too many official placeholders",
            malformed(
                r#"official: Some("Request for party rejected.")"#,
                r#"official: Some("%s %s rejected.")"#,
            ),
        ),
        (
            "official display without official text",
            malformed(
                r#"msg_id: Some(77), official: Some("Party has successfully been organized.")"#,
                r#"msg_id: None, official: None"#,
            )
            .replacen(
                r#"default: "created", args: [], display: Current"#,
                r#"default: "created", args: [], display: Official"#,
                1,
            ),
        ),
        (
            "stray closing brace",
            malformed(r#"default: "created""#, r#"default: "created }""#),
        ),
        (
            "empty provenance",
            malformed(r#"provenance: "test fixture""#, r#"provenance: """#),
        ),
    ];

    for (name, document) in cases {
        assert!(validate(document).is_err(), "the malformed document '{name}' was accepted");
    }
}

#[test]
fn malformed_documents_never_drop_the_message() {
    let table = parse(&minimal()).expect("the minimal fixture has to be valid");
    // Unknown code, missing arguments and an empty argument list all render a
    // complete message instead of panicking or dropping the text.
    assert_eq!(format_with(&table, MessageDomain::PartyMemberRemoved, 9, &[]), "unknown 9");
    assert_eq!(
        format_with(&table, MessageDomain::PartyMemberRemoved, 0, &[]),
        "left {character_name}"
    );
}

#[test]
fn official_display_renders_the_official_text_and_falls_back() {
    let official = malformed(
        r#"official: Some("Request for party rejected.")"#,
        r#"official: Some("Request for party %s rejected.")"#,
    )
    .replacen(
        r#"default: "rejected {character_name}", args: ["character_name"], display: Current"#,
        r#"default: "rejected {character_name}", args: ["character_name"], display: Official"#,
        1,
    );
    let table = parse(&official).expect("the official-display fixture has to be valid");
    assert_eq!(
        format_with(&table, MessageDomain::PartyInviteReply, 0, &[("character_name", "Sora")]),
        "Request for party Sora rejected."
    );
    // Missing call-site arguments fall back to the current text instead of
    // rendering a half-substituted official message.
    assert_eq!(
        format_with(&table, MessageDomain::PartyInviteReply, 0, &[]),
        "rejected {character_name}"
    );
}

#[test]
fn named_placeholders_and_code_are_substituted() {
    assert_eq!(placeholder_names("a {one} b {two}").unwrap(), vec!["one", "two"]);
    assert!(placeholder_names("no placeholders").unwrap().is_empty());
    assert!(placeholder_names("{broken").is_err());
    assert!(placeholder_names("{}").is_err());
    assert!(placeholder_names("stray }").is_err());
}

/// The client feedback renderer shares the placeholder syntax with the result
/// message table but has no result code: the chat feedback templates (which
/// live in the client localization files, not in this table) use it.
#[test]
fn client_feedback_templates_render_named_arguments() {
    assert_eq!(
        render_template("Skill {skill}: the target is out of range.", &[("skill", "12")]),
        "Skill 12: the target is out of range."
    );
    assert_eq!(
        render_template("Hotbar page {page}/{pages}.", &[("page", "1"), ("pages", "3")]),
        "Hotbar page 1/3."
    );

    // A missing argument keeps the placeholder visible instead of dropping
    // text (same documented behaviour as the result messages).
    assert_eq!(render_template("Skill {skill}.", &[]), "Skill {skill}.");

    // `{code}` is only substituted by the result-code table; the client
    // feedback renderer has no result code and keeps it visible.
    assert_eq!(
        render_template("Page {page}/{pages} ({code}).", &[("page", "1"), ("pages", "3")]),
        "Page 1/3 ({code})."
    );
}
