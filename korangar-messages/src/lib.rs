//! Data-driven protocol result-code message table.
//!
//! The client used to translate protocol result codes (party, shop, skill
//! failure, ammunition, map-server ban and login failures) into English chat /
//! error strings with per-code `match` arms in `korangar/src/lib.rs` and
//! `korangar-networking/src/packet_versions/version_20220406.rs`. This crate
//! owns the shared table that replaces them:
//!
//! * the data lives in `korangar/archive/data/client/messages.ron` (generated
//!   from `tools/data/message_facts.json` by `tools/gen_messages.py`),
//! * the same file is embedded with `include_str!` as the compiled fallback,
//!   and the client loads an archive override through its normal file loader
//!   ([`install`]), so a server can override the texts without a rebuild and
//!   the two can never drift apart,
//! * validation is strict and explicit ([`validate`]): unknown fields, unknown
//!   domains, unsorted/duplicate rows, duplicate keys and undeclared format
//!   arguments are rejected. `korangar --check-messages` exposes it to CI.
//!
//! # Lookup and fallbacks
//!
//! [`format_message`] is a binary search over the rows of one domain, sorted by
//! result code. A code the table does not cover renders the domain's committed
//! `unknown` fallback (for example `Party invitation result: 12.`) instead of
//! silently dropping the message; a message template can always use `{code}`
//! (the numeric result code) in addition to the call-site arguments.
//!
//! Every lookup is allocation-light: the loaded table is a process-wide static,
//! lookups return `&'static` rows, and only the final rendered `String` is
//! allocated.
//!
//! # `display`: behaviour preservation and the localization pass
//!
//! Every row carries both texts:
//!
//! * `default` — the current client text, byte for byte (the P0 behaviour),
//! * `official` / `msg_id` — the official rAthena `MsgStringTable` entry (the
//!   skill-fail causes use the official client's own mapping from
//!   roBrowserLegacy) for the later localization pass.
//!
//! [`MessageDisplay::Current`] renders `default`; [`MessageDisplay::Official`]
//! renders `official` and falls back to `default` when the row has no official
//! text or the call site did not supply the arguments the official template
//! needs. P0 commits **every** row as `Current`, i.e. the rendered text is
//! unchanged; switching a row to `Official` is a data-only change after review.
//!
//! Arguments are supplied by the call site as `(name, value)` pairs, so the
//! text never has to encode call-site context. Call-site arguments that the
//! current template needs (member/skill names) are never dropped.
//!
//! # Client feedback templates
//!
//! The client's own chat feedback strings (client-side validation messages such
//! as "Skill {skill}: the target is out of range.") are **not** protocol result
//! codes: their templates live in the client localization files
//! (`korangar/src/state/localization`). They reuse only the named-placeholder
//! renderer through [`render_template`], so there is one template
//! implementation and no duplicate result-code table.
//!
//! # Thread safety
//!
//! The table is immutable after installation ([`OnceLock`]); rendering only
//! reads it, so lookups are safe from any thread.

use std::sync::OnceLock;

use serde::Deserialize;

/// Data file path inside the client archive (`data\client\...`).
pub const DATA_PATH: &str = "data\\client\\messages.ron";

/// Compiled fallback copy of the same data file.
const FALLBACK: &str = include_str!("../../korangar/archive/data/client/messages.ron");

/// Number of message domains (must match [`MessageDomain::ALL`]).
pub const DOMAIN_COUNT: usize = 9;

/// A family of protocol result codes with one message per code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize)]
pub enum MessageDomain {
    /// `ZC_DELETE_MEMBER_FROM_GROUP` (party member removed).
    PartyMemberRemoved,
    /// `ZC_PARTY_JOIN_REQ_ACK` (party invitation reply).
    PartyInviteReply,
    /// `ZC_ACK_MAKE_GROUP` (party creation result).
    PartyCreation,
    /// `ZC_PC_PURCHASE_RESULT` (NPC shop purchase result).
    BuyItem,
    /// `ZC_ACK_TOUSESKILL` (skill use failed, rAthena `useskill_fail_cause`).
    SkillUseFailed,
    /// `ZC_ACTION_FAILURE` (ammunition action).
    AmmunitionAction,
    /// `SC_NOTIFY_BAN` (map-server ban / forced disconnect).
    MapServerBan,
    /// `SC_NOTIFY_BAN` on the login server (`LoginFailedPacket`).
    LoginFailed,
    /// `AC_REFUSE_LOGIN2` (login server refusal).
    LoginFailed2,
}

impl MessageDomain {
    /// Every domain, in the order the data file has to list them.
    pub const ALL: [Self; DOMAIN_COUNT] = [
        Self::PartyMemberRemoved,
        Self::PartyInviteReply,
        Self::PartyCreation,
        Self::BuyItem,
        Self::SkillUseFailed,
        Self::AmmunitionAction,
        Self::MapServerBan,
        Self::LoginFailed,
        Self::LoginFailed2,
    ];

    /// Stable index used by the in-memory table.
    pub const fn index(self) -> usize {
        match self {
            Self::PartyMemberRemoved => 0,
            Self::PartyInviteReply => 1,
            Self::PartyCreation => 2,
            Self::BuyItem => 3,
            Self::SkillUseFailed => 4,
            Self::AmmunitionAction => 5,
            Self::MapServerBan => 6,
            Self::LoginFailed => 7,
            Self::LoginFailed2 => 8,
        }
    }

    /// Stable snake-case key of the domain (diagnostics and tooling).
    pub const fn key(self) -> &'static str {
        match self {
            Self::PartyMemberRemoved => "party_member_removed",
            Self::PartyInviteReply => "party_invite_reply",
            Self::PartyCreation => "party_creation",
            Self::BuyItem => "buy_item",
            Self::SkillUseFailed => "skill_use_failed",
            Self::AmmunitionAction => "ammunition_action",
            Self::MapServerBan => "map_server_ban",
            Self::LoginFailed => "login_failed",
            Self::LoginFailed2 => "login_failed2",
        }
    }
}

/// Which text of a row is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum MessageDisplay {
    /// The current client text (`default`); P0 behaviour.
    #[default]
    Current,
    /// The official `MsgStringTable` text (`official`), for the later
    /// localization pass.
    Official,
}

/// One validated message row of a domain.
#[derive(Debug)]
pub struct MessageRow {
    code: u16,
    key: String,
    msg_id: Option<u16>,
    official: Option<String>,
    default: String,
    args: Vec<String>,
    display: MessageDisplay,
}

impl MessageRow {
    /// The protocol result code this row is keyed by.
    pub fn code(&self) -> u16 {
        self.code
    }

    /// Stable message key (unique across the table, used by localization).
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Official `MsgStringTable` id of the message, if the official server /
    /// client defines one.
    pub fn msg_id(&self) -> Option<u16> {
        self.msg_id
    }

    /// Official text of [`Self::msg_id`], if the table entry has usable text
    /// (an `MSI_*` macro placeholder is not usable).
    pub fn official(&self) -> Option<&str> {
        self.official.as_deref()
    }

    /// Current client text (the P0 default), a named-placeholder template.
    pub fn default_text(&self) -> &str {
        &self.default
    }

    /// Call-site format argument names, in the order official-client `%s`/`%d`
    /// placeholders are substituted.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Which text is rendered.
    pub fn display(&self) -> MessageDisplay {
        self.display
    }
}

struct DomainTable {
    unknown: String,
    rows: Vec<MessageRow>,
}

impl DomainTable {
    fn emergency(domain: MessageDomain) -> Self {
        Self {
            unknown: format!("Unknown {} result (code {{code}}).", domain.key()),
            rows: Vec::new(),
        }
    }
}

struct Table {
    provenance: String,
    domains: Vec<DomainTable>,
    row_count: usize,
}

impl Table {
    fn emergency() -> Self {
        Self {
            provenance: "emergency fallback (the compiled message table failed to parse)".to_owned(),
            domains: MessageDomain::ALL.iter().copied().map(DomainTable::emergency).collect(),
            row_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Schema (deserialization with `deny_unknown_fields`).
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MessageFile {
    provenance: String,
    version: u32,
    domains: Vec<DomainFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DomainFile {
    domain: MessageDomain,
    unknown: String,
    #[serde(default)]
    unknown_args: Vec<String>,
    rows: Vec<RowFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RowFile {
    code: u16,
    key: String,
    #[serde(default)]
    msg_id: Option<u16>,
    #[serde(default)]
    official: Option<String>,
    default: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    display: MessageDisplay,
}

// ---------------------------------------------------------------------------
// Public API.
// ---------------------------------------------------------------------------

static TABLE: OnceLock<Table> = OnceLock::new();

/// Loads an archive override of the message table.
///
/// The client calls this once at start-up with the content of
/// `data\client\messages.ron` when the archive provides one. A malformed
/// override is rejected (`Err`) and the compiled fallback stays in use, so a
/// bad override can never leave the client without result messages. Returns the
/// number of validated rows of the installed table.
pub fn install(content: &str) -> Result<usize, String> {
    let table = parse(content)?;
    let count = table.row_count;
    // The first installed table wins; a second call is a no-op (the client
    // installs exactly once). Ignoring the result keeps `install` idempotent.
    let _ = TABLE.set(table);
    Ok(count)
}

/// Renders the message of `(domain, code)` with the call-site arguments.
///
/// `args` are `(name, value)` pairs; `{code}` is always available in a
/// template. A code that is not in the table renders the domain's committed
/// `unknown` fallback, so no result is ever silently dropped. A missing named
/// argument is rendered literally (`{name}`) instead of truncating the message.
pub fn format_message(domain: MessageDomain, code: u16, args: &[(&str, &str)]) -> String {
    format_with(table(), domain, code, args)
}

/// The validated row of `(domain, code)`, if the table has one.
pub fn row(domain: MessageDomain, code: u16) -> Option<&'static MessageRow> {
    let domain_table = table().domains.get(domain.index())?;
    domain_table
        .rows
        .binary_search_by_key(&code, |row| row.code)
        .ok()
        .map(|index| &domain_table.rows[index])
}

/// All rows of `domain`, sorted by result code.
pub fn rows(domain: MessageDomain) -> &'static [MessageRow] {
    table()
        .domains
        .get(domain.index())
        .map(|domain_table| domain_table.rows.as_slice())
        .unwrap_or(&[])
}

/// Number of committed message rows.
pub fn row_count() -> usize {
    table().row_count
}

/// Provenance line of the installed (or compiled) table.
pub fn provenance() -> &'static str {
    &table().provenance
}

/// Validates a message table document and returns its number of rows.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `korangar --check-messages [PATH]` so CI can validate the committed data
/// file (or an archive override) without starting a window or a network
/// connection.
pub fn validate(content: &str) -> Result<usize, String> {
    parse(content).map(|table| table.row_count)
}

/// Validates the compiled fallback (used by `--check-messages` without a path
/// and by the tests).
pub fn validate_compiled() -> Result<usize, String> {
    validate(FALLBACK)
}

fn table() -> &'static Table {
    TABLE.get_or_init(|| match parse(FALLBACK) {
        Ok(table) => table,
        Err(_) => Table::emergency(),
    })
}

// ---------------------------------------------------------------------------
// Parsing, validation and rendering.
// ---------------------------------------------------------------------------

fn parse(content: &str) -> Result<Table, String> {
    let file: MessageFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    validate_file(&file)?;
    Ok(build(file))
}

fn build(file: MessageFile) -> Table {
    let mut domains: Vec<Option<DomainTable>> = (0..DOMAIN_COUNT).map(|_| None).collect();
    let mut row_count = 0;

    for domain_file in file.domains {
        let index = domain_file.domain.index();
        row_count += domain_file.rows.len();
        let rows = domain_file
            .rows
            .into_iter()
            .map(|row| MessageRow {
                code: row.code,
                key: row.key,
                msg_id: row.msg_id,
                official: row.official,
                default: row.default,
                args: row.args,
                display: row.display,
            })
            .collect();
        domains[index] = Some(DomainTable {
            unknown: domain_file.unknown,
            rows,
        });
    }

    let domains = domains
        .into_iter()
        .enumerate()
        .map(|(index, domain)| domain.unwrap_or_else(|| DomainTable::emergency(MessageDomain::ALL[index])))
        .collect();

    Table {
        provenance: file.provenance,
        domains,
        row_count,
    }
}

fn validate_file(file: &MessageFile) -> Result<(), String> {
    if file.version != 1 {
        return Err(format!("unsupported message table version {}", file.version));
    }
    if file.provenance.trim().is_empty() {
        return Err("the message table has no provenance line".to_owned());
    }
    if file.domains.len() != DOMAIN_COUNT {
        return Err(format!("expected {DOMAIN_COUNT} message domains, found {}", file.domains.len()));
    }

    let mut seen_keys: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut seen_domains: Vec<usize> = Vec::with_capacity(DOMAIN_COUNT);

    for domain in &file.domains {
        let index = domain.domain.index();
        if seen_domains.last().is_some_and(|last| *last >= index) {
            return Err(format!(
                "message domains are missing, duplicated or out of order at '{}'",
                domain.domain.key()
            ));
        }
        seen_domains.push(index);

        if domain.unknown.trim().is_empty() {
            return Err(format!("domain '{}' has an empty unknown fallback", domain.domain.key()));
        }
        validate_template_arguments(
            &domain.unknown,
            &domain.unknown_args,
            &format!("domain '{}' unknown fallback", domain.domain.key()),
        )?;

        if domain.rows.is_empty() {
            return Err(format!("domain '{}' has no message rows", domain.domain.key()));
        }

        let mut previous_code: Option<u16> = None;
        for row in &domain.rows {
            let context = format!("domain '{}' code {}", domain.domain.key(), row.code);
            if let Some(previous_code) = previous_code
                && row.code <= previous_code
            {
                return Err(format!(
                    "{context}: rows are not sorted and unique (previous code {previous_code})"
                ));
            }
            previous_code = Some(row.code);

            if row.key.trim().is_empty() {
                return Err(format!("{context}: empty message key"));
            }
            if !row
                .key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
            {
                return Err(format!("{context}: invalid message key '{}'", row.key));
            }
            if !seen_keys.insert(&row.key) {
                return Err(format!("{context}: duplicate message key '{}'", row.key));
            }
            if row.default.trim().is_empty() {
                return Err(format!("{context}: empty default text"));
            }
            for argument in &row.args {
                if !is_argument_name(argument) {
                    return Err(format!("{context}: invalid argument name '{argument}'"));
                }
            }
            validate_template_arguments(&row.default, &row.args, &context)?;

            if let Some(official) = &row.official {
                if official.trim().is_empty() {
                    return Err(format!("{context}: empty official text"));
                }
                if row.msg_id.is_none() {
                    return Err(format!("{context}: official text without a msg id"));
                }
                let conversions = printf_conversions(official);
                if conversions > row.args.len() {
                    return Err(format!(
                        "{context}: official text has {conversions} placeholders but only {} args",
                        row.args.len()
                    ));
                }
                validate_template_arguments(official, &row.args, &context)?;
            }
            if row.display == MessageDisplay::Official && row.official.is_none() {
                return Err(format!("{context}: display is 'Official' but no official text is available"));
            }
        }
    }

    if seen_domains != (0..DOMAIN_COUNT).collect::<Vec<_>>() {
        return Err("message domains are missing, duplicated or out of order".to_owned());
    }

    Ok(())
}

fn is_argument_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(|character: char| character.is_ascii_digit())
        && name
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_')
}

/// Validates that every `{name}` of a template is declared by `args` or is the
/// implicit `{code}`.
fn validate_template_arguments(template: &str, args: &[String], context: &str) -> Result<(), String> {
    for name in placeholder_names(template)? {
        if name != "code" && !args.iter().any(|argument| argument == name) {
            return Err(format!("{context}: template uses undeclared argument {{{name}}}"));
        }
    }
    Ok(())
}

/// Returns the `{name}` placeholders of a template, rejecting malformed braces.
fn placeholder_names(template: &str) -> Result<Vec<&str>, String> {
    let mut names = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let close = after
            .find('}')
            .ok_or_else(|| format!("template has an unclosed placeholder: {template:?}"))?;
        let name = &after[..close];
        if !is_argument_name(name) {
            return Err(format!("template has an invalid placeholder {{{name}}}: {template:?}"));
        }
        names.push(name);
        rest = &after[close + 1..];
    }
    if rest.contains('}') {
        return Err(format!("template has a stray '}}': {template:?}"));
    }
    Ok(names)
}

/// Number of printf conversions (`%s`, `%d`, ...) in an official-client text.
fn printf_conversions(template: &str) -> usize {
    let mut count = 0;
    let mut characters = template.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '%' {
            continue;
        }
        match characters.peek() {
            Some('%') => {
                characters.next();
            }
            Some('s' | 'd' | 'i' | 'u' | 'f') => {
                count += 1;
                characters.next();
            }
            _ => {}
        }
    }
    count
}

fn format_with(table: &Table, domain: MessageDomain, code: u16, args: &[(&str, &str)]) -> String {
    let Some(domain_table) = table.domains.get(domain.index()) else {
        return format!("Unknown {} result (code {code}).", domain.key());
    };

    match domain_table.rows.binary_search_by_key(&code, |row| row.code) {
        Ok(index) => render_row(&domain_table.rows[index], code, args),
        Err(_) => render_named(&domain_table.unknown, code, args),
    }
}

fn render_row(row: &MessageRow, code: u16, args: &[(&str, &str)]) -> String {
    if row.display == MessageDisplay::Official
        && let Some(official) = row.official.as_deref()
    {
        let named = placeholder_names(official).unwrap_or_default();
        if !named.is_empty()
            && let Ok(rendered) = render_official_named(official, code, args)
        {
            return rendered;
        }
        if named.is_empty()
            && let Some(rendered) = render_official_printf(official, &row.args, args)
        {
            return rendered;
        }
        // The official text cannot be rendered with the arguments this call
        // site supplied: fall back to the current text (documented above).
    }

    render_named(&row.default, code, args)
}

/// Renders a `` `{name}` `` placeholder template **without** a protocol result
/// code.
///
/// This is the shared renderer for the client feedback strings (chat
/// notifications like "Skill {skill}: the target is out of range."). Their
/// templates live in the client localization files, not in this protocol
/// result-code table; only the placeholder syntax and the missing-argument
/// behaviour are shared with [`format_message`], so the client has exactly one
/// renderer and no duplicate message table.
///
/// A `{code}` placeholder is kept verbatim (those call sites have no result
/// code); every other `{name}` is substituted from `args`, and a missing
/// argument keeps the placeholder visible instead of dropping text.
pub fn render_template(template: &str, args: &[(&str, &str)]) -> String {
    render_with_code(template, None, args)
}

fn render_named(template: &str, code: u16, args: &[(&str, &str)]) -> String {
    render_with_code(template, Some(code), args)
}

fn render_with_code(template: &str, code: Option<u16>, args: &[(&str, &str)]) -> String {
    let mut rendered = String::with_capacity(template.len() + 16);
    let mut rest = template;
    loop {
        let Some(open) = rest.find('{') else {
            rendered.push_str(rest);
            return rendered;
        };
        rendered.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            // Malformed template: keep the remainder verbatim.
            rendered.push_str(&rest[open..]);
            return rendered;
        };
        let name = &after[..close];
        if name == "code" {
            match code {
                Some(code) => rendered.push_str(&code.to_string()),
                // Client feedback templates have no result code: keep the
                // placeholder visible instead of silently dropping it.
                None => {
                    rendered.push('{');
                    rendered.push_str(name);
                    rendered.push('}');
                }
            }
        } else if let Some((_, value)) = args.iter().find(|(key, _)| *key == name) {
            rendered.push_str(value);
        } else {
            // Missing call-site argument: keep the placeholder visible instead
            // of dropping text (the validation guarantees the name exists).
            rendered.push('{');
            rendered.push_str(name);
            rendered.push('}');
        }
        rest = &after[close + 1..];
    }
}

fn render_official_named(template: &str, code: u16, args: &[(&str, &str)]) -> Result<String, ()> {
    Ok(render_named(template, code, args))
}

/// Substitutes the official-client `%s`/`%d` placeholders in argument order.
///
/// Returns `None` when the template uses a conversion the formatter does not
/// know or when the call site did not supply enough arguments, so the caller
/// can fall back to the current text.
fn render_official_printf(template: &str, argument_names: &[String], args: &[(&str, &str)]) -> Option<String> {
    let mut next_argument = 0;
    let mut rendered = String::with_capacity(template.len() + 16);
    let mut characters = template.chars().peekable();
    while let Some(character) = characters.next() {
        if character != '%' {
            rendered.push(character);
            continue;
        }
        match characters.peek() {
            Some('%') => {
                characters.next();
                rendered.push('%');
            }
            Some('s' | 'd' | 'i' | 'u' | 'f') => {
                characters.next();
                let name = argument_names.get(next_argument)?;
                let value = args.iter().find(|(key, _)| key == name).map(|(_, value)| *value)?;
                rendered.push_str(value);
                next_argument += 1;
            }
            _ => return None,
        }
    }
    Some(rendered)
}

#[cfg(test)]
mod tests;
