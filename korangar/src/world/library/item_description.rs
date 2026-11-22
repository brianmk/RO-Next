//! Item description tables of the classic client data.
//!
//! The classic client stores the description text of every item in two text
//! tables next to the item names:
//!
//! * `data\idnum2itemdesctable.txt` - description of the *identified* item,
//! * `data\num2itemdesctable.txt` - description of the *unidentified* item
//!   (usually just "Can be identified by using a Magnifier.").
//!
//! Both files use the classic `id#` / lines / `#` format and contain RO markup
//! (`^rrggbb` colour codes and `_` separator lines). The CondaRO server archive
//! ships an English version of both tables which has priority over the Korean
//! original in the base `data.grf`.
//!
//! The client font only has Latin glyphs, so every line is filtered down to
//! displayable characters. Lines that are not displayable (e.g. the
//! untranslated Korean remainders of a few entries) are skipped instead of
//! being rendered as square placeholder glyphs.
//!
//! The table is parsed once into [`ItemDescriptions`] and cached per item id,
//! so the item metadata can keep the description lines without re-parsing or
//! re-decoding anything per frame.

use encoding_rs::EUC_KR;
use hashbrown::HashMap;
use ragnarok_packets::ItemId;

use crate::loaders::GameFileLoader;

/// Path of the identified item description table.
const IDENTIFIED_TABLE_PATH: &str = "data\\idnum2itemdesctable.txt";
/// Path of the unidentified item description table.
const UNIDENTIFIED_TABLE_PATH: &str = "data\\num2itemdesctable.txt";

/// Placeholder shown when the client data contains no description at all.
pub const NO_DESCRIPTION: &str = "No description available.";

/// Parsed item description tables, cached per item id.
#[derive(Default)]
pub struct ItemDescriptions {
    identified: HashMap<ItemId, Vec<String>>,
    unidentified: HashMap<ItemId, Vec<String>>,
}

impl ItemDescriptions {
    /// Loads both description tables from every archive, highest priority
    /// first.
    pub fn load(game_file_loader: &GameFileLoader) -> Self {
        Self {
            identified: load_table(game_file_loader, IDENTIFIED_TABLE_PATH),
            unidentified: load_table(game_file_loader, UNIDENTIFIED_TABLE_PATH),
        }
    }

    /// Description lines of an item.
    ///
    /// Prefers the table that matches the identified state. The unidentified
    /// table of the classic data is empty for many items, so it falls back to
    /// the identified description instead of showing nothing.
    pub fn get(&self, item_id: ItemId, is_identified: bool) -> Option<&[String]> {
        let primary = match is_identified {
            true => &self.identified,
            false => &self.unidentified,
        };

        if let Some(lines) = primary.get(&item_id)
            && !lines.is_empty()
        {
            return Some(lines);
        }

        if !is_identified
            && let Some(lines) = self.identified.get(&item_id)
            && !lines.is_empty()
        {
            return Some(lines);
        }

        None
    }
}

/// Parses one description table and merges the entries of all archives.
fn load_table(game_file_loader: &GameFileLoader, path: &str) -> HashMap<ItemId, Vec<String>> {
    let mut table: HashMap<ItemId, Vec<String>> = HashMap::new();

    for data in game_file_loader.get_all(path) {
        for (item_id, lines) in parse_table(&data) {
            // The first archive that contains the item wins (the English
            // CondaRO archive is loaded before the Korean base archive).
            table.entry(item_id).or_insert(lines);
        }
    }

    table
}

/// Parses the classic `id#` / description lines / `#` format.
///
/// Returns the description lines that are non-empty after the cleanup, so
/// entries without a usable description are skipped entirely.
pub fn parse_table(data: &[u8]) -> Vec<(ItemId, Vec<String>)> {
    let mut entries = Vec::new();
    let mut current_id: Option<ItemId> = None;
    let mut raw_lines: Vec<String> = Vec::new();

    for raw_line in data.split(|byte| *byte == b'\n') {
        let line = trim_ascii(raw_line);

        match current_id {
            // Outside an entry: a line with `digits#` starts a new entry.
            None => {
                let Some(item_id) = parse_entry_start(line) else {
                    continue;
                };

                current_id = Some(item_id);
                raw_lines.clear();
            }
            // Inside an entry: a line that is just `#` ends it.
            Some(item_id) => {
                if line == b"#" {
                    let lines = clean_description_lines(std::mem::take(&mut raw_lines));

                    if !lines.is_empty() {
                        entries.push((item_id, lines));
                    }

                    current_id = None;
                } else if line.is_empty() {
                    // Whitespace-only lines are paragraph separators and are
                    // kept (the block is cleaned up afterwards).
                    raw_lines.push(String::new());
                } else if let Some(line) = decode_line(line)
                    && let Some(line) = sanitize_line(&line)
                {
                    raw_lines.push(line);
                }
            }
        }
    }

    // Tolerate a missing final terminator (malformed custom tables).
    if let Some(item_id) = current_id {
        let lines = clean_description_lines(std::mem::take(&mut raw_lines));

        if !lines.is_empty() {
            entries.push((item_id, lines));
        }
    }

    entries
}

/// Returns the item id of an `id#` entry start line.
fn parse_entry_start(line: &[u8]) -> Option<ItemId> {
    let number = line.strip_suffix(b"#")?;

    if number.is_empty() || !number.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let id = std::str::from_utf8(number).ok()?.parse::<u32>().ok()?;

    Some(ItemId(id))
}

/// Decodes one raw description line and strips the RO text markup.
///
/// The CondaRO table is mostly UTF-8 English while the Korean base table is
/// EUC-KR, so both encodings have to be tried before falling back to a lossy
/// decode.
fn decode_line(bytes: &[u8]) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }

    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => match EUC_KR.decode_without_bom_handling_and_without_replacement(bytes) {
            Some(text) => text.into_owned(),
            None => String::from_utf8_lossy(bytes).into_owned(),
        },
    };

    Some(text)
}

/// Returns whether the client font can display this character.
///
/// The shipped interface font (`NotoSans.ttf`) has Latin glyphs only (no
/// Hangul, no CJK, no arrows and almost no math/geometric symbols), so the text
/// has to be restricted to printable ASCII. The RO colour codes (`^rrggbb`) are
/// part of the text (the renderer interprets them) and are therefore allowed as
/// well.
fn is_displayable_character(character: char) -> bool {
    character.is_ascii_graphic() || character == ' '
}

/// Strips stray `#` placeholders and drops lines that contain characters the
/// client font cannot render.
///
/// The RO colour markup (`^rrggbb`) is preserved so the renderer can draw the
/// official blue stat labels; the text layouter strips it from the layout.
pub fn sanitize_line(line: &str) -> Option<String> {
    let line: String = line.chars().filter(|character| *character != '#').collect();
    let line = line.trim_end();

    if line.chars().any(|character| !is_displayable_character(character)) {
        return None;
    }

    Some(line.to_owned())
}

/// Sanitizes description lines from another source (e.g. the iteminfo
/// fallback), dropping non-displayable lines.
pub fn sanitize_lines<S: AsRef<str>>(lines: &[S]) -> Vec<String> {
    clean_description_lines(lines.iter().filter_map(|line| sanitize_line(line.as_ref())).collect())
}

/// Trims and normalizes the raw description lines.
///
/// Empty lines are kept as paragraph separators (collapsed to one), leading and
/// trailing empty lines are removed.
fn clean_description_lines(raw_lines: Vec<String>) -> Vec<String> {
    let mut lines = Vec::with_capacity(raw_lines.len());

    for line in raw_lines {
        let line = line.trim_end().to_owned();
        let is_empty = line.trim().is_empty();

        if is_empty {
            if lines.last().is_some_and(|last: &String| last.trim().is_empty()) {
                continue;
            }

            lines.push(String::new());
        } else {
            lines.push(line);
        }
    }

    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    while lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }

    lines
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) && bytes.first() != Some(&b'\r') {
        bytes = &bytes[1..];
    }

    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }

    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_classic_description_entries() {
        let data = b"501#\r\nA bottle of potion.\r\n \r\n^0000CCTYPE:^000000 Restorative\r\n#\r\n502#\r\nAnother potion.\r\n#\r\n";

        let entries = parse_table(data);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, ItemId(501));
        assert_eq!(entries[0].1, vec![
            "A bottle of potion.".to_owned(),
            String::new(),
            "^0000CCTYPE:^000000 Restorative".to_owned()
        ]);
        assert_eq!(entries[1].0, ItemId(502));
        assert_eq!(entries[1].1, vec!["Another potion.".to_owned()]);
    }

    #[test]
    fn skips_empty_entries_and_garbage() {
        let data = b"garbage\r\n501#\r\n\r\n#\r\n502#\r\nReal text.\r\n#\r\n";

        let entries = parse_table(data);

        assert_eq!(entries, vec![(ItemId(502), vec!["Real text.".to_owned()])]);
    }

    #[test]
    fn keeps_colour_codes_and_strips_placeholders() {
        // The interface text renderer interprets `^rrggbb`, so the colour codes
        // are kept in the description text.
        assert_eq!(
            sanitize_line("^0000CCType:^000000 One-Handed Sword"),
            Some("^0000CCType:^000000 One-Handed Sword".to_owned())
        );
        assert_eq!(
            sanitize_line("^990099Magnifier^000000"),
            Some("^990099Magnifier^000000".to_owned())
        );
        assert_eq!(sanitize_line("a # place holder"), Some("a  place holder".to_owned()));
        assert_eq!(sanitize_line("a ^ b"), Some("a ^ b".to_owned()));
        assert_eq!(sanitize_line("a ^123 b"), Some("a ^123 b".to_owned()));
    }

    #[test]
    fn lines_with_non_ascii_characters_are_dropped() {
        assert_eq!(sanitize_line("Caf\u{e9}"), None);
        assert_eq!(sanitize_line("\u{d55c}\u{ad6d}\u{c5b4}"), None);
        assert_eq!(sanitize_line("Plain ASCII line"), Some("Plain ASCII line".to_owned()));
    }

    #[test]
    fn decodes_euc_kr_lines() {
        // "무게" (weight) encoded as EUC-KR.
        let line = decode_line(&[0xB9, 0xAB, 0xB0, 0xD4]).expect("line should decode");

        assert!(line.chars().all(|character| !character.is_ascii() && !character.is_control()));
        assert_eq!(sanitize_line(&line), None);
    }

    #[test]
    fn collapsed_blank_lines_and_trims_edges() {
        let lines = clean_description_lines(vec![
            String::new(),
            "Line 1".to_owned(),
            "   ".to_owned(),
            String::new(),
            "Line 2".to_owned(),
            String::new(),
        ]);

        assert_eq!(lines, vec!["Line 1".to_owned(), String::new(), "Line 2".to_owned()]);
    }

    #[test]
    fn unidentified_falls_back_to_identified_description() {
        let mut descriptions = ItemDescriptions::default();
        descriptions.identified.insert(ItemId(501), vec!["Identified text.".to_owned()]);

        assert_eq!(
            descriptions.get(ItemId(501), false),
            Some(["Identified text.".to_owned()].as_slice())
        );
        assert_eq!(
            descriptions.get(ItemId(501), true),
            Some(["Identified text.".to_owned()].as_slice())
        );
    }

    #[test]
    fn unidentified_description_prefers_the_unidentified_table() {
        let mut descriptions = ItemDescriptions::default();
        descriptions.identified.insert(ItemId(501), vec!["Identified text.".to_owned()]);
        descriptions
            .unidentified
            .insert(ItemId(501), vec!["Can be identified by using a Magnifier.".to_owned()]);

        assert_eq!(
            descriptions.get(ItemId(501), false),
            Some(["Can be identified by using a Magnifier.".to_owned()].as_slice())
        );
    }

    #[test]
    fn missing_descriptions_return_none() {
        let descriptions = ItemDescriptions::default();

        assert_eq!(descriptions.get(ItemId(999_999), true), None);
        assert_eq!(descriptions.get(ItemId(999_999), false), None);
    }

    #[test]
    fn korean_lines_are_dropped_instead_of_rendered_as_squares() {
        // "무게 : 60" (weight) in EUC-KR mixed with an English line.
        let data = b"501#\r\nA potion.\r\n\xb9\xab\xb0\xd4 : 60\r\n#\r\n";

        let entries = parse_table(data);

        assert_eq!(entries, vec![(ItemId(501), vec!["A potion.".to_owned()])]);
    }

    #[test]
    fn an_entry_that_is_only_korean_is_skipped_entirely() {
        let data = b"501#\r\n\xb9\xab\xb0\xd4 : 60\r\n#\r\n";

        assert!(parse_table(data).is_empty());
    }

    #[test]
    fn sanitize_lines_drops_korean_and_empty_lines() {
        let lines = sanitize_lines(&["", "Damage +10", "\u{d55c}\u{ad6d}\u{c5b4}", "", "Weight: 1"]);

        assert_eq!(lines, vec!["Damage +10".to_owned(), String::new(), "Weight: 1".to_owned()]);
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn condaro_english_description_tables_are_loaded() {
        let game_file_loader = crate::loaders::GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
        game_file_loader.load_patched_lua_files();

        if !lua_archive_existed {
            game_file_loader.remove_patched_lua_files();
        }

        let descriptions = ItemDescriptions::load(&game_file_loader);

        // Falchion (1104): English one-handed sword description from
        // CondaRO.grf, with the colour codes already stripped.
        let falchion = descriptions.get(ItemId(1104), true).expect("Falchion description missing");
        assert!(falchion.iter().any(|line| line.contains("One-Handed Sword")));
        // The official blue labels use the RO colour markup, which the client
        // text renderer understands; it has to be preserved.
        assert!(falchion.iter().any(|line| line.contains("^0000CC")));

        // Every line of every entry has to be displayable with the client font.
        for item_id in [501u32, 1104, 1201, 2301, 1326] {
            if let Some(lines) = descriptions.get(ItemId(item_id), true) {
                assert!(
                    lines.iter().all(|line| line.chars().all(is_displayable_character)),
                    "non-displayable description line for item {item_id}: {lines:?}"
                );
            }
        }

        // 1326 is a mixed entry of the English table (Korean lines with English
        // fragments): only the ASCII lines survive, never Korean squares.
        let mixed = descriptions.get(ItemId(1326), true);
        assert!(mixed.is_none_or(|lines| lines.iter().all(|line| { line.chars().all(is_displayable_character) })));

        // English table coverage: the text-encoding audit measured that 9,619 of
        // the 9,647 iteminfo ids get an English description (the remaining 28
        // fall back to the placeholder). Count the ids that have a usable
        // description either from the English table or from a displayable
        // iteminfo fallback.
        use crate::world::library::{ItemInfo, Table};

        let item_info_table = ItemInfo::load(&game_file_loader).expect("failed to load iteminfo");
        let mut described = 0;

        for item_id in item_info_table.keys() {
            if descriptions.get(*item_id, true).is_some() {
                described += 1;
            }
        }

        assert!(
            described >= 9_500,
            "only {described} of {} iteminfo ids have an English description",
            item_info_table.len()
        );

        // Unidentified Falchion: "Can be identified by using a Magnifier."
        let unidentified = descriptions
            .get(ItemId(1104), false)
            .expect("unidentified Falchion description missing");
        assert!(unidentified.iter().any(|line| line.contains("Magnifier")));

        // Red Potion (501) has a real English description.
        let potion = descriptions.get(ItemId(501), true).expect("Red Potion description missing");
        assert!(potion.iter().any(|line| line.contains("Red Herb")));
    }
}
