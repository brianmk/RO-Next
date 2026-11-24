//! Item display names.
//!
//! The item names in `iteminfo.lub` are the original Korean names (EUC-KR
//! bytes). The client font only contains Latin glyphs, so drawing those names
//! on a ground item label or in a chat message shows square placeholder
//! glyphs.
//!
//! Translated clients ship a `data\idnum2itemdisplaynametable.txt` table with
//! localized display names. For the CondaRO data set that table is English and
//! lives in the server archive (which has priority over the base game
//! archive), e.g. `501#Red_Potion#`. This module resolves every item name from
//! that table first and falls back to the (sanitized) name from `iteminfo.lub`
//! and finally to `Item #<id>`.
//!
//! All item name displays (ground item labels, pickup chat messages,
//! inventory/equipment tooltips, shops, storage) use this one resolution so
//! they are always consistent.

use std::borrow::Cow;

use encoding_rs::{EUC_KR, UTF_8};
use hashbrown::HashMap;
use ragnarok_packets::ItemId;

use super::ItemInfo;
use crate::loaders::GameFileLoader;

/// Path of the classic client item display name table (identified items).
pub(super) const DISPLAY_NAME_TABLE_PATH: &str = "data\\idnum2itemdisplaynametable.txt";

/// Path of the classic client unidentified item display name table.
pub(super) const UNIDENTIFIED_DISPLAY_NAME_TABLE_PATH: &str = "data\\num2itemdisplaynametable.txt";

/// Resolved item display names.
///
/// Both maps contain only displayable (Latin/ASCII) names, so every value can
/// be rendered with the shipped interface font.
pub struct ItemDisplayNames {
    identified: HashMap<ItemId, String>,
    unidentified: HashMap<ItemId, String>,
}

impl ItemDisplayNames {
    /// Loads the display name table(s) and resolves all names.
    ///
    /// The tables are merged in archive priority order: the table from the
    /// highest priority archive wins, but an entry is only accepted if its
    /// name is actually displayable with the client font. This makes the
    /// English table from the server archive win over the original Korean
    /// table from the base game archive.
    pub fn load(game_file_loader: &GameFileLoader, item_info_table: &HashMap<ItemId, ItemInfo>) -> Self {
        let mut identified = HashMap::new();
        let mut unidentified = HashMap::new();

        for table_data in game_file_loader.get_all(DISPLAY_NAME_TABLE_PATH) {
            let Some(table) = parse_display_name_table(&table_data) else {
                continue;
            };

            for (item_id, name) in table {
                if let Some(name) = sanitize_name(&name) {
                    identified.entry(ItemId(item_id)).or_insert(name);
                }
            }
        }

        // The unidentified name table (`num2itemdisplaynametable.txt`) of the
        // CondaRO archive is English for 15,722 of the 16,607 ids; without it
        // unidentified items would fall back to "Unidentified <identified name>".
        for table_data in game_file_loader.get_all(UNIDENTIFIED_DISPLAY_NAME_TABLE_PATH) {
            let Some(table) = parse_display_name_table(&table_data) else {
                continue;
            };

            for (item_id, name) in table {
                if let Some(name) = sanitize_name(&name) {
                    unidentified.entry(ItemId(item_id)).or_insert(name);
                }
            }
        }

        // Fall back to the names from `iteminfo.lub` for items that are missing
        // from the display name table. Those names are usually the original
        // Korean names; they are only used when they are displayable (e.g. for
        // custom items with Latin names).
        for (item_id, item_info) in item_info_table.iter() {
            let identified_name = sanitize_name(&item_info.identified_name.to_string());
            let unidentified_name = sanitize_name(&item_info.unidentified_name.to_string());

            if let Some(name) = identified_name {
                identified.entry(*item_id).or_insert(name);
            }

            if let Some(name) = unidentified_name {
                unidentified.entry(*item_id).or_insert(name);
            } else if let Some(name) = identified.get(item_id) {
                // Unidentified items usually have a Korean "unidentified" name in
                // `iteminfo.lub`; show the English identified name instead of squares.
                unidentified.entry(*item_id).or_insert_with(|| format!("Unidentified {name}"));
            }
        }

        Self { identified, unidentified }
    }

    /// Returns the resolved display name for an item, or `Item #<id>` for items
    /// that are not part of the loaded data.
    pub fn name(&self, item_id: ItemId, is_identified: bool) -> Cow<'_, str> {
        let names = match is_identified {
            true => &self.identified,
            false => &self.unidentified,
        };

        match names.get(&item_id) {
            Some(name) => Cow::Borrowed(name.as_str()),
            None => Cow::Owned(format!("Item #{}", item_id.0)),
        }
    }
}

/// Parses a `id#name#` display name table.
///
/// The table is decoded line by line: the classic format is EUC-KR (with the
/// ASCII subset being identical to UTF-8), but some custom server archives
/// contain invalid byte sequences or a mix of encodings inside a single file.
/// Decoding per line makes sure the valid (e.g. English ASCII) entries are
/// still used instead of rejecting the whole table.
///
/// Individual lines that cannot be parsed are skipped.
fn parse_display_name_table(data: &[u8]) -> Option<Vec<(u32, String)>> {
    let mut entries = Vec::new();

    for raw_line in data.split(|byte| *byte == b'\n') {
        let raw_line = trim_ascii(raw_line);

        if raw_line.is_empty() {
            continue;
        }

        let Some(separator) = raw_line.iter().position(|byte| *byte == b'#') else {
            continue;
        };

        let Some(item_id) = std::str::from_utf8(&raw_line[..separator])
            .ok()
            .and_then(|item_id| item_id.trim().parse::<u32>().ok())
        else {
            continue;
        };

        let name_bytes = &raw_line[separator + 1..];
        let name_bytes = match name_bytes.iter().position(|byte| *byte == b'#') {
            Some(end) => &name_bytes[..end],
            None => name_bytes,
        };

        if let Some(name) = decode_name(name_bytes) {
            entries.push((item_id, name));
        }
    }

    match entries.is_empty() {
        true => None,
        false => Some(entries),
    }
}

/// Decodes a single name from the display name table.
fn decode_name(bytes: &[u8]) -> Option<String> {
    let (text, _, had_errors) = EUC_KR.decode(bytes);

    if !had_errors {
        return Some(text.into_owned());
    }

    let (text, _, had_errors) = UTF_8.decode(bytes);

    match had_errors {
        false => Some(text.into_owned()),
        true => None,
    }
}

/// Trims ASCII whitespace from both ends of a byte slice.
fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while let [first, rest @ ..] = bytes {
        if first.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }

    while let [rest @ .., last] = bytes {
        if last.is_ascii_whitespace() {
            bytes = rest;
        } else {
            break;
        }
    }

    bytes
}

/// Normalizes a display name and returns `None` if it contains glyphs the
/// shipped interface font cannot render.
///
/// The classic table uses underscores instead of spaces (`Red_Potion`), so
/// those are converted to spaces. Only printable ASCII is accepted: the
/// interface font has no Korean/other glyphs, and rendering those would show
/// square placeholder boxes (the bug this module fixes).
fn sanitize_name(name: &str) -> Option<String> {
    let name = name.trim().replace('_', " ");
    let name = name.trim();

    if name.is_empty() {
        return None;
    }

    let displayable = name.chars().all(|character| character.is_ascii_graphic() || character == ' ');

    match displayable {
        true => Some(name.to_owned()),
        false => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_display_name_table, sanitize_name};

    /// Builds a game file loader exactly like the client startup does (the
    /// `lub` bytecode has to be upcast to Lua 5.1 first).
    fn load_game_file_loader() -> crate::loaders::GameFileLoader {
        let game_file_loader = crate::loaders::GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
        game_file_loader.load_patched_lua_files();

        if !lua_archive_existed {
            game_file_loader.remove_patched_lua_files();
        }

        game_file_loader
    }

    #[test]
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
    fn english_item_names_are_resolved_from_the_condaro_table() {
        use ragnarok_packets::ItemId;

        use super::ItemDisplayNames;
        use crate::world::library::{ItemInfo, Table};

        let game_file_loader = load_game_file_loader();
        let item_info_table = ItemInfo::load(&game_file_loader).expect("failed to load iteminfo");
        let names = ItemDisplayNames::load(&game_file_loader, &item_info_table);

        // The CondaRO server archive has an English `idnum2itemdisplaynametable.txt`.
        assert_eq!(names.name(ItemId(501), true).as_ref(), "Red Potion");
        assert_eq!(names.name(ItemId(1201), true).as_ref(), "Knife");
        assert_eq!(names.name(ItemId(2301), true).as_ref(), "Cotton Shirt");

        // Names must never contain glyphs that the interface font cannot render.
        for item_id in [501u32, 1201, 2301, 2501, 909] {
            let name = names.name(ItemId(item_id), true);
            assert!(
                name.chars().all(|character| character.is_ascii_graphic() || character == ' '),
                "{name}"
            );
        }

        // Unknown ids fall back to a readable placeholder instead of squares.
        assert_eq!(names.name(ItemId(999_999), true).as_ref(), "Item #999999");

        // Unidentified items use the English `num2itemdisplaynametable.txt`
        // (`data\num2itemdisplaynametable.txt`, CondaRO.grf) instead of the
        // synthesized "Unidentified <name>" fallback.
        assert_eq!(names.name(ItemId(1104), false).as_ref(), "Unidentified Sword");
        assert_eq!(names.name(ItemId(5123), false).as_ref(), "Unidentified Hat");

        // The generated English name supplement in `archive/data/` covers the
        // ids whose CondaRO name was still Korean (server item_db names).
        assert_eq!(names.name(ItemId(7804), true).as_ref(), "Octopig's Leg");
        assert_eq!(names.name(ItemId(12223), true).as_ref(), "Sweets Macaron Cake");

        // Ids that are not in the server item database keep the readable
        // `Item #id` placeholder (322 ids per the text-encoding audit).
        assert_eq!(names.name(ItemId(17161), true).as_ref(), "Item #17161");

        // No item name may contain glyphs the interface font cannot render.
        for item_id in item_info_table
            .keys()
            .chain([ragnarok_packets::ItemId(7804), ragnarok_packets::ItemId(17161)].iter())
        {
            let name = names.name(*item_id, true);
            assert!(
                name.chars().all(|character| character.is_ascii_graphic() || character == ' '),
                "item {} name is not displayable: {name}",
                item_id.0
            );
        }

        // The CondaRO name table has 1,049 Korean names; the generated English
        // supplement covers 727 of them from the server item database, leaving
        // 322 ids without an English name in any client table. Those ids still
        // resolve to a displayable name (an English iteminfo name or the
        // `Item #id` placeholder), never to Korean squares.
        let mut korean_table_ids = Vec::new();
        let mut supplement_ids = Vec::new();

        for table_data in game_file_loader.get_all(super::DISPLAY_NAME_TABLE_PATH) {
            let Some(table) = super::parse_display_name_table(&table_data) else {
                continue;
            };

            for (item_id, name) in table {
                if name.chars().any(|character| !character.is_ascii()) {
                    korean_table_ids.push(item_id);
                } else {
                    supplement_ids.push(item_id);
                }
            }
        }

        // The generated supplement is the lowest-fidelity archive entry: it is
        // a small ASCII-only table, so the English ids of that archive copy are
        // exactly the supplement ids.
        let supplement_count = supplement_ids.len();
        assert!(supplement_count >= 727, "supplement ids: {supplement_count}");

        // Sanitized resolution never produces Korean glyphs.
        for item_id in &korean_table_ids {
            let name = names.name(ragnarok_packets::ItemId(*item_id), true);
            assert!(
                name.chars().all(|character| character.is_ascii_graphic() || character == ' '),
                "Korean table name for item {item_id} leaked into the client: {name}"
            );
        }
        assert!(!korean_table_ids.is_empty());
    }

    #[test]
    fn english_display_table_is_parsed() {
        let data = b"501#Red_Potion#\r\n1201#Knife#\r\n";
        let entries = parse_display_name_table(data).expect("table must decode");

        assert_eq!(entries, vec![(501, "Red_Potion".to_owned()), (1201, "Knife".to_owned())]);
        assert_eq!(sanitize_name("Red_Potion"), Some("Red Potion".to_owned()));
    }

    #[test]
    fn korean_names_are_not_displayable() {
        // EUC-KR bytes for a Korean item name.
        let data = b"501#\xB0\xA1\xB0\xA1#\r\n";
        let entries = parse_display_name_table(data).expect("table must decode");

        assert_eq!(entries.len(), 1);
        assert_eq!(sanitize_name(&entries[0].1), None);
    }

    #[test]
    fn malformed_lines_are_skipped() {
        let data = b"#\r\nfoo#bar#\r\n501#\r\n600#Red_Potion#\r\n";
        let entries = parse_display_name_table(data).expect("table must decode");

        assert_eq!(entries.len(), 2);
    }
}
