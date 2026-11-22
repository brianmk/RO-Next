//! Loader for the client message table (`data/msgstringtable.txt`).
//!
//! The 2019 server sends standard system messages ("inventory full", "cannot
//! use item", trade/party refusals, ...) as `ZC_MSG` (0x0291) with a message id
//! only. The client had no message table at all, so every such message was
//! silently dropped.
//!
//! `msgstringtable.txt` is a EUC-KR text file with one message per line; the
//! line index is the message id. Comment lines (`# ...`) and empty lines have
//! no display text and are treated as missing so that the caller can fall back
//! to the raw id.

use encoding_rs::EUC_KR;
use korangar_loaders::FileLoader;

/// Path of the message table inside the game archives.
pub const MESSAGE_TABLE_PATH: &str = "data\\msgstringtable.txt";

/// The message table of the client.
pub struct MessageTable {
    entries: Vec<String>,
}

impl MessageTable {
    /// Loads the message table from the game archives. Returns [`None`] when
    /// the archive set does not contain the file.
    pub fn load(loader: &impl FileLoader) -> Option<Self> {
        let bytes = loader.get(MESSAGE_TABLE_PATH).ok()?;
        Some(Self::parse(&bytes))
    }

    /// Parses a EUC-KR message table.
    pub fn parse(bytes: &[u8]) -> Self {
        let (text, ..) = EUC_KR.decode(bytes);

        let entries = text
            .split('\n')
            .map(|line| line.trim_end_matches(['\r', '\0']).to_owned())
            .collect();

        Self { entries }
    }

    /// Number of entries of the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Message text of `message_id`, [`None`] for unknown/empty/comment ids.
    pub fn get(&self, message_id: u16) -> Option<&str> {
        let entry = self.entries.get(message_id as usize)?;
        let entry = entry.trim();

        match entry.is_empty() || entry.starts_with('#') {
            true => None,
            false => Some(entry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_ids_are_line_indices() {
        let table = MessageTable::parse(b"inventory is full\r\n# comment\r\n\r\ncannot use item\0\r\n");

        assert_eq!(table.get(0), Some("inventory is full"));
        // Comment and empty lines have no display text (fall back to the raw id).
        assert_eq!(table.get(1), None);
        assert_eq!(table.get(2), None);
        assert_eq!(table.get(3), Some("cannot use item"));
        assert_eq!(table.get(4), None);
        assert_eq!(table.get(9999), None);
    }

    #[test]
    fn korean_messages_are_decoded() {
        // "인벤토리가 가득 찼습니다." in EUC-KR.
        let bytes = [
            0xC0, 0xCE, 0xBA, 0xA5, 0xC5, 0xE4, 0xB8, 0xAE, 0xB0, 0xA1, 0x20, 0xB0, 0xA1, 0xB5, 0xE6, 0x20, 0xC3, 0xA1, 0xBD, 0xC0, 0xB4,
            0xCF, 0xB4, 0xD9, 0x2E, 0x0D, 0x0A,
        ];
        let table = MessageTable::parse(&bytes);

        assert_eq!(table.get(0), Some("인벤토리가 가득 찼습니다."));
    }
}
