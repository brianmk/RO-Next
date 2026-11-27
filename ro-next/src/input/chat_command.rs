//! Translation of official-client style chat slash commands into rAthena
//! atcommands.
//!
//! The official Ragnarok client lets players type commands with a leading
//! slash (`/go prontera`, `/warp prontera 150 150`, `/where`, ...) and
//! translates them into the corresponding atcommand before sending them to the
//! server. rAthena atcommands use the `@` prefix (`@go`, `@warp`, `@where`,
//! ...), so without this translation slash commands are sent as ordinary chat
//! messages and simply do nothing.
//!
//! The translation is deliberately minimal: only the leading `/` is replaced
//! by `@`, the command name, its case and all arguments are kept exactly as
//! typed. Client-side commands (`/nc`, `/w`, `/whisper`, `/bm`, `/where`) are
//! not translated because they are handled by the client itself before the
//! text is sent.

use ragnarok_packets::TilePosition;

/// Names of slash commands that the client handles itself and therefore must
/// not be forwarded to the server as atcommands.
const CLIENT_SIDE_COMMANDS: [&str; 5] = ["nc", "w", "whisper", "bm", "where"];

/// Requested battle-mode state of a `/bm` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleModeCommand {
    /// Set battle mode on (`/bm on`, `/bm 1`).
    On,
    /// Set battle mode off (`/bm off`, `/bm 0`).
    Off,
    /// Toggle the current value (`/bm`).
    Toggle,
}

/// Parses the client-side `/bm` (battle mode) command.
///
/// `None` means the text is not a battle-mode command and has to be handled
/// like normal chat. `/bm` alone toggles; `/bm on`/`off` (also `1`/`0`,
/// `true`/`false`) sets an explicit state. An unknown argument is not a valid
/// battle-mode command, so it falls through to the regular slash-command
/// translation instead of silently toggling.
pub fn parse_battle_mode_command(text: &str) -> Option<BattleModeCommand> {
    let mut parts = text.split_whitespace();
    let command = parts.next()?;

    if !command.eq_ignore_ascii_case("/bm") {
        return None;
    }

    match parts.next() {
        None => Some(BattleModeCommand::Toggle),
        Some(argument) if argument.eq_ignore_ascii_case("on") || matches!(argument, "1" | "true" | "enable") => Some(BattleModeCommand::On),
        Some(argument) if argument.eq_ignore_ascii_case("off") || matches!(argument, "0" | "false" | "disable") => {
            Some(BattleModeCommand::Off)
        }
        Some(_) => None,
    }
}

/// Handles the client-side `/where` command.
///
/// The rAthena `@where` atcommand is a GM tool that asks for a *target
/// character name* on this server, so it is useless to players. `/where` is
/// therefore answered locally with the player's own map and tile and never
/// sent to the server. The command name is case-insensitive, surrounding
/// whitespace is ignored and any arguments (`/where <name>`) are ignored.
///
/// Returns `Some(message)` with the line to push into the local chat window
/// when `text` is a `/where` command, `None` for every other text.
pub fn parse_where_command(text: &str, map_name: &str, position: Option<TilePosition>) -> Option<String> {
    let command = text.split_whitespace().next()?;

    if !command.eq_ignore_ascii_case("/where") {
        return None;
    }

    Some(format_where_location(map_name, position))
}

/// Formats the local `/where` answer.
///
/// An unknown map (empty name) or an unknown player tile means the location is
/// not available yet, e.g. while connecting or changing maps.
fn format_where_location(map_name: &str, position: Option<TilePosition>) -> String {
    match position {
        Some(position) if !map_name.is_empty() => format!("Location: {map_name} ({}, {})", position.x, position.y),
        _ => "Location unavailable.".to_owned(),
    }
}

/// Translates a leading slash command into its rAthena atcommand form.
///
/// Returns `Some("@<command> <args>")` if `text` is a slash command and
/// `None` if it has to be sent as a regular chat message (normal chat, a raw
/// atcommand, empty text, a bare `/`, or a command that the client handles
/// itself).
///
/// Leading whitespace is skipped when looking for the command because rAthena
/// only recognizes an atcommand when the `@` is the very first byte, so the
/// returned command does not contain that whitespace.
pub fn translate_slash_command(text: &str) -> Option<String> {
    let trimmed = text.trim_start();
    let rest = trimmed.strip_prefix('/')?;

    // A valid command starts with at least one ASCII letter (`/ go` and `//go`
    // are regular chat, not commands).
    let command = rest
        .chars()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect::<String>();

    if command.is_empty()
        || CLIENT_SIDE_COMMANDS
            .iter()
            .any(|client_command| command.eq_ignore_ascii_case(client_command))
    {
        return None;
    }

    Some(format!("@{rest}"))
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::TilePosition;

    use super::{BattleModeCommand, parse_battle_mode_command, parse_where_command, translate_slash_command};

    #[test]
    fn battle_mode_command_toggles_and_sets_the_state() {
        assert_eq!(parse_battle_mode_command("/bm"), Some(BattleModeCommand::Toggle));
        assert_eq!(parse_battle_mode_command("  /BM  "), Some(BattleModeCommand::Toggle));
        assert_eq!(parse_battle_mode_command("/bm on"), Some(BattleModeCommand::On));
        assert_eq!(parse_battle_mode_command("/bm 1"), Some(BattleModeCommand::On));
        assert_eq!(parse_battle_mode_command("/bm off"), Some(BattleModeCommand::Off));
        assert_eq!(parse_battle_mode_command("/bm 0"), Some(BattleModeCommand::Off));
        assert_eq!(parse_battle_mode_command("/bm disable"), Some(BattleModeCommand::Off));
    }

    #[test]
    fn non_battle_mode_text_is_ignored() {
        assert_eq!(parse_battle_mode_command("hello"), None);
        assert_eq!(parse_battle_mode_command("/bmm"), None);
        assert_eq!(parse_battle_mode_command("/b m"), None);
        assert_eq!(parse_battle_mode_command(""), None);
        // Unknown arguments stay normal chat commands.
        assert_eq!(parse_battle_mode_command("/bm maybe"), None);
    }

    #[test]
    fn battle_mode_is_not_translated_to_an_atcommand() {
        assert_eq!(translate_slash_command("/bm"), None);
    }

    #[test]
    fn where_command_shows_the_local_location() {
        let position = Some(TilePosition { x: 156, y: 187 });

        assert_eq!(
            parse_where_command("/where", "prontera", position).as_deref(),
            Some("Location: prontera (156, 187)")
        );
        assert_eq!(
            parse_where_command("  /WHERE  ", "prontera", position).as_deref(),
            Some("Location: prontera (156, 187)")
        );
        // Arguments are ignored, so `/where <name>` never reaches the server.
        assert_eq!(
            parse_where_command("/where Bob", "prontera", position).as_deref(),
            Some("Location: prontera (156, 187)")
        );
    }

    #[test]
    fn where_command_reports_an_unavailable_location() {
        let position = Some(TilePosition { x: 156, y: 187 });

        assert_eq!(
            parse_where_command("/where", "prontera", None).as_deref(),
            Some("Location unavailable.")
        );
        assert_eq!(
            parse_where_command("/where", "", position).as_deref(),
            Some("Location unavailable.")
        );
        assert_eq!(
            parse_where_command("/where", "", None).as_deref(),
            Some("Location unavailable.")
        );
    }

    #[test]
    fn non_where_text_is_ignored() {
        let position = Some(TilePosition { x: 156, y: 187 });

        assert_eq!(parse_where_command("hello", "prontera", position), None);
        assert_eq!(parse_where_command("/whereabouts", "prontera", position), None);
        assert_eq!(parse_where_command("/ where", "prontera", position), None);
        assert_eq!(parse_where_command("", "prontera", position), None);
    }

    #[test]
    fn where_is_not_translated_to_an_atcommand() {
        assert_eq!(translate_slash_command("/where"), None);
        assert_eq!(translate_slash_command("/where Bob"), None);
        assert_eq!(translate_slash_command("/WHERE"), None);
    }

    #[test]
    fn translates_slash_commands_to_atcommands() {
        assert_eq!(translate_slash_command("/go prontera").as_deref(), Some("@go prontera"));
        assert_eq!(
            translate_slash_command("/warp prontera 150 150").as_deref(),
            Some("@warp prontera 150 150")
        );
        assert_eq!(translate_slash_command("/memo").as_deref(), Some("@memo"));
        assert_eq!(translate_slash_command("/save").as_deref(), Some("@save"));
    }

    #[test]
    fn keeps_raw_atcommands_unchanged() {
        assert_eq!(translate_slash_command("@go prontera"), None);
        assert_eq!(translate_slash_command("@where"), None);
    }

    #[test]
    fn keeps_normal_chat_unchanged() {
        assert_eq!(translate_slash_command("hello /go"), None);
        assert_eq!(translate_slash_command(""), None);
        assert_eq!(translate_slash_command("   "), None);
        // Party/guild/channel prefixes are not commands.
        assert_eq!(translate_slash_command("#channel hello"), None);
        assert_eq!(translate_slash_command("$party hello"), None);
        assert_eq!(translate_slash_command("%guild hello"), None);
    }

    #[test]
    fn rejects_incomplete_commands() {
        assert_eq!(translate_slash_command("/"), None);
        assert_eq!(translate_slash_command(" // "), None);
        assert_eq!(translate_slash_command("/ go"), None);
        assert_eq!(translate_slash_command("//go"), None);
        assert_eq!(translate_slash_command("@"), None);
    }

    #[test]
    fn skips_leading_whitespace() {
        assert_eq!(translate_slash_command("  /go 0").as_deref(), Some("@go 0"));
        assert_eq!(translate_slash_command("\t/go\t0").as_deref(), Some("@go\t0"));
    }

    #[test]
    fn preserves_command_case() {
        assert_eq!(translate_slash_command("/Go 0").as_deref(), Some("@Go 0"));
        assert_eq!(translate_slash_command("/WARP prontera").as_deref(), Some("@WARP prontera"));
    }

    #[test]
    fn keeps_client_side_commands() {
        assert_eq!(translate_slash_command("/nc"), None);
        assert_eq!(translate_slash_command("/w Bob hello"), None);
        assert_eq!(translate_slash_command("/whisper Bob hello"), None);
        assert_eq!(translate_slash_command("/where"), None);
        assert_eq!(translate_slash_command("/where Bob"), None);
    }
}
