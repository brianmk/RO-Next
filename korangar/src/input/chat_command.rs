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
//! typed. Client-side commands (`/nc`, `/w`, `/whisper`) are not translated
//! because they are handled by the client itself before the text is sent.

/// Names of slash commands that the client handles itself and therefore must
/// not be forwarded to the server as atcommands.
const CLIENT_SIDE_COMMANDS: [&str; 3] = ["nc", "w", "whisper"];

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
    use super::translate_slash_command;

    #[test]
    fn translates_slash_commands_to_atcommands() {
        assert_eq!(translate_slash_command("/go prontera").as_deref(), Some("@go prontera"));
        assert_eq!(translate_slash_command("/where").as_deref(), Some("@where"));
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
        assert_eq!(translate_slash_command("/WHERE").as_deref(), Some("@WHERE"));
    }

    #[test]
    fn keeps_client_side_commands() {
        assert_eq!(translate_slash_command("/nc"), None);
        assert_eq!(translate_slash_command("/w Bob hello"), None);
        assert_eq!(translate_slash_command("/whisper Bob hello"), None);
    }
}
