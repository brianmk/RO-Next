//! Loader for the client message table (`data/msgstringtable.txt`).
//!
//! The 2019 server sends standard system messages ("inventory full", "cannot
//! use item", trade/party refusals, ...) as `ZC_MSG` (0x0291) with a message id
//! only. The client had no message table at all, so every such message was
//! silently dropped.
//!
//! `msgstringtable.txt` is a EUC-KR text file with one message per line; the
//! line index is the message id. The classic table uses `#` as the line
//! terminator (every message ends with `#`), comment/empty lines have no
//! display text and are treated as missing so that the caller can fall back to
//! the raw id.
//!
//! Display safety: this table is the English one (`CondaRO.grf`), but the
//! loader must never hand a character to the renderer that it cannot draw.
//!
//! * the trailing `#` terminator is stripped from every message,
//! * the two arrow glyphs of the table (`▲` id 3446, `▼` id 3447) are replaced
//!   by their ASCII equivalents (`^`, `v`),
//! * a message that still contains non-ASCII text (for example when only the
//!   Korean `data.grf` is present, which has no `msgstringtable.txt` at all, or
//!   a foreign Korean table) is treated as missing, so the caller shows the
//!   clean `Message #<id>` fallback instead of squares or mojibake,
//! * the loader logs one warning at startup when it finds such entries, so a
//!   missing English resource is visible instead of silently degrading.

use encoding_rs::EUC_KR;
use korangar_loaders::FileLoader;

/// Path of the message table inside the game archives.
pub const MESSAGE_TABLE_PATH: &str = "data\\msgstringtable.txt";

/// Characters of the classic message table that have no ASCII glyph in the
/// primary font and a well-defined ASCII stand-in.
///
/// `▲`/`▼` are the only non-ASCII characters in the CondaRO English table
/// (messages 3446/3447, which the 2019 server does not actually send; they are
/// used by the official client UI). The ASCII stand-ins keep the table free of
/// glyph-dependency.
const GLYPH_REPLACEMENTS: [(char, &str); 2] = [('▲', "^"), ('▼', "v")];

/// Static statistics about the parsed table, used for the startup warning.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MessageTableStatistics {
    /// Messages that contained a known non-ASCII glyph which was mapped to
    /// ASCII.
    pub mapped_glyph_entries: usize,
    /// Messages that were treated as missing because they contain Hangul.
    pub korean_entries: usize,
    /// Messages that were treated as missing because of other non-ASCII text.
    pub unsupported_entries: usize,
}

/// The message table of the client.
pub struct MessageTable {
    entries: Vec<Option<String>>,
    statistics: MessageTableStatistics,
}

impl MessageTable {
    /// Loads the message table from the game archives. Returns [`None`] when
    /// the archive set does not contain the file.
    pub fn load(loader: &impl FileLoader) -> Option<Self> {
        let bytes = loader.get(MESSAGE_TABLE_PATH).ok()?;
        let table = Self::parse(&bytes);

        table.warn_about_non_english_entries();

        Some(table)
    }

    /// Parses a EUC-KR message table.
    pub fn parse(bytes: &[u8]) -> Self {
        let (text, ..) = EUC_KR.decode(bytes);

        let mut statistics = MessageTableStatistics::default();

        let entries = text.split('\n').map(|line| clean_entry(line, &mut statistics)).collect();

        Self { entries, statistics }
    }

    /// Number of entries of the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Message text of `message_id`, [`None`] for unknown/empty/comment ids and
    /// for messages that contain text the renderer cannot display.
    pub fn get(&self, message_id: u16) -> Option<&str> {
        self.entries.get(message_id as usize)?.as_deref()
    }

    /// Statistics of the parsed table, used by the startup warning and tests.
    #[cfg(test)]
    pub fn statistics(&self) -> MessageTableStatistics {
        self.statistics
    }

    /// Logs one warning at startup when the table is not the expected English
    /// one, so missing/untranslated resources are visible to the user.
    ///
    /// The message table is never rendered character by character: entries with
    /// Korean or other non-ASCII text are dropped in favor of the
    /// `Message #<id>` fallback, and the two arrow glyphs are mapped to ASCII.
    fn warn_about_non_english_entries(&self) {
        let statistics = self.statistics;

        if statistics.korean_entries > 0 {
            crate::logging::log_warn!(
                "msgstringtable: {} message(s) contain Korean text; the English translation table was not resolved (showing 'Message #id' \
                 for them instead of square glyphs)",
                statistics.korean_entries
            );
        } else if statistics.unsupported_entries > 0 {
            crate::logging::log_warn!(
                "msgstringtable: {} message(s) contain non-ASCII characters without an ASCII replacement; showing 'Message #id' for them \
                 instead of square glyphs",
                statistics.unsupported_entries
            );
        } else if statistics.mapped_glyph_entries > 0 {
            crate::logging::log_debug!(
                "msgstringtable: {} message(s) contain the arrow glyphs of the table, mapped to ASCII",
                statistics.mapped_glyph_entries
            );
        }
    }
}

/// Cleans one raw line of the table (still containing `\r`/`\0`).
///
/// Updates the statistics for the startup warning.
fn clean_entry(line: &str, statistics: &mut MessageTableStatistics) -> Option<String> {
    let line = line.trim_end_matches(['\r', '\0']);

    // The classic table terminates every message with '#'. Stripping the
    // terminator must not turn a comment line ('# ...') into a message.
    let line = line.strip_suffix('#').unwrap_or(line);

    let line = line.trim();

    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut cleaned = String::with_capacity(line.len());
    let mut mapped_glyph = false;
    let mut korean = false;
    let mut unsupported = false;

    for character in line.chars() {
        if let Some((_, replacement)) = GLYPH_REPLACEMENTS.iter().find(|(glyph, _)| *glyph == character) {
            cleaned.push_str(replacement);
            mapped_glyph = true;
        } else if character.is_ascii() {
            cleaned.push(character);
        } else if is_korean(character) {
            korean = true;
        } else {
            unsupported = true;
        }
    }

    if mapped_glyph {
        statistics.mapped_glyph_entries += 1;
    }

    if korean {
        statistics.korean_entries += 1;
        return None;
    }

    if unsupported {
        statistics.unsupported_entries += 1;
        return None;
    }

    Some(cleaned)
}

/// Whether the character is part of the Korean writing system (Hangul
/// syllables, Hangul jamo or Hangul compatibility jamo).
fn is_korean(character: char) -> bool {
    matches!(character as u32, 0xAC00..=0xD7A3 | 0x1100..=0x11FF | 0x3130..=0x318F)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The message table is a EUC-KR file; encode test strings like the real
    /// file.
    fn euc_kr(text: &str) -> Vec<u8> {
        let (bytes, ..) = EUC_KR.encode(text);
        bytes.into_owned()
    }

    #[test]
    fn message_ids_are_line_indices() {
        let table = MessageTable::parse(b"inventory is full#\r\n# comment\r\n\r\ncannot use item#\0\r\n");

        assert_eq!(table.get(0), Some("inventory is full"));
        // Comment and empty lines have no display text (fall back to the raw id).
        assert_eq!(table.get(1), None);
        assert_eq!(table.get(2), None);
        assert_eq!(table.get(3), Some("cannot use item"));
        assert_eq!(table.get(4), None);
        assert_eq!(table.get(9999), None);
    }

    #[test]
    fn the_classic_trailing_terminator_is_stripped() {
        let table = MessageTable::parse(b"Do you agree?#\r\nhttp://rathena.org#\r\n");

        assert_eq!(table.get(0), Some("Do you agree?"));
        assert_eq!(table.get(1), Some("http://rathena.org"));
        assert!(table.get(0).is_some_and(|message| !message.ends_with('#')));
    }

    #[test]
    fn the_arrow_glyphs_are_mapped_to_ascii() {
        // The two non-ASCII messages of the CondaRO table (ids 3446/3447).
        let table = MessageTable::parse(&euc_kr("▲#\r\n▼#\r\n"));

        assert_eq!(table.get(0), Some("^"));
        assert_eq!(table.get(1), Some("v"));
        assert_eq!(table.statistics().mapped_glyph_entries, 2);
        assert_eq!(table.statistics().korean_entries, 0);
        assert_eq!(table.statistics().unsupported_entries, 0);
    }

    #[test]
    fn korean_messages_fall_back_to_the_message_id() {
        // "인벤토리가 가득 찼습니다." in EUC-KR.
        let bytes = [
            0xC0, 0xCE, 0xBA, 0xA5, 0xC5, 0xE4, 0xB8, 0xAE, 0xB0, 0xA1, 0x20, 0xB0, 0xA1, 0xB5, 0xE6, 0x20, 0xC3, 0xA1, 0xBD, 0xC0, 0xB4,
            0xCF, 0xB4, 0xD9, 0x2E, 0x0D, 0x0A,
        ];
        let table = MessageTable::parse(&bytes);

        // Korean text is never handed to the renderer: the caller shows "Message #id".
        assert_eq!(table.get(0), None);
        assert_eq!(table.statistics().korean_entries, 1);
    }

    #[test]
    fn mixed_and_unsupported_messages_are_treated_as_missing() {
        let table = MessageTable::parse(&euc_kr("Good#\r\nGood 한글#\r\nGood ★#\r\n"));

        assert_eq!(table.get(0), Some("Good"));
        assert_eq!(table.get(1), None);
        assert_eq!(table.get(2), None);
        assert_eq!(table.statistics().korean_entries, 1);
        assert_eq!(table.statistics().unsupported_entries, 1);
    }

    #[test]
    fn every_displayed_message_is_ascii() {
        // The real CondaRO table is English and uses '#' as terminator on all
        // 3,577 non-empty lines. This synthetic copy contains the interesting
        // cases (terminator, arrows, a Korean line).
        let table = MessageTable::parse("Do you agree?#\r\nhttp://rathena.org#\r\n▲#\r\n▼#\r\n인벤토리가 가득 찼습니다.#\r\n".as_bytes());

        for (message_id, message) in table.entries.iter().enumerate() {
            if let Some(message) = message {
                assert!(message.is_ascii(), "message {message_id} is not ASCII: {message:?}");
                assert!(!message.ends_with('#'), "message {message_id} still has the terminator");
            }
        }
    }
}
