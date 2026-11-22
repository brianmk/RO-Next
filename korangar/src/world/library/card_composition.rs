//! Card name composition tables of the classic client.
//!
//! The official client composes the display name of a carded item from two
//! client data tables:
//!
//! * `data\cardprefixnametable.txt` - card item id → name that is placed in
//!   front of the item name (e.g. card `4001` → `Lucky`),
//! * `data\cardpostfixnametable.txt` - the card ids whose name is placed after
//!   the item name instead of in front (e.g. Roda Frog card → `of Champion`).
//!
//! Both files are simple `id#name#` tables (the postfix file usually only lists
//! the ids and the name comes from the prefix table). The CondaRO server
//! archive ships an English version of both tables which has priority over the
//! Korean version in the base `data.grf`.
//!
//! This module only provides the raw tables plus the pure name composition
//! function; [`Library`] combines it with the item name database.

use encoding_rs::EUC_KR;
use hashbrown::{HashMap, HashSet};
use ragnarok_packets::ItemId;

use crate::loaders::GameFileLoader;

/// Path of the card prefix name table.
const PREFIX_TABLE_PATH: &str = "data\\cardprefixnametable.txt";
/// Path of the card postfix name table.
const POSTFIX_TABLE_PATH: &str = "data\\cardpostfixnametable.txt";

/// The card name composition tables.
#[derive(Default)]
pub struct CardComposition {
    /// Card id → composition name (e.g. `Lucky`, `of Champion`).
    prefixes: HashMap<ItemId, String>,
    /// Card ids whose composition name is placed *after* the item name.
    postfixes: HashSet<ItemId>,
}

impl CardComposition {
    /// Loads the composition tables from every archive, highest priority first.
    ///
    /// Missing or unreadable tables are not an error: the composition then
    /// falls back to deriving the prefix from the card's own display name (see
    /// [`compose_item_name`]).
    pub fn load(game_file_loader: &GameFileLoader) -> Self {
        let mut composition = Self::default();

        for data in game_file_loader.get_all(PREFIX_TABLE_PATH) {
            for (item_id, name) in parse_table(&data) {
                composition.prefixes.entry(item_id).or_insert(name);
            }
        }

        for data in game_file_loader.get_all(POSTFIX_TABLE_PATH) {
            for (item_id, name) in parse_table(&data) {
                composition.postfixes.insert(item_id);

                // Some client data sets put the composition name into the
                // postfix table instead of the prefix one.
                if !name.is_empty() {
                    composition.prefixes.entry(item_id).or_insert(name);
                }
            }
        }

        composition
    }

    /// The composition name of a card, if the tables know it.
    pub fn prefix(&self, item_id: ItemId) -> Option<&str> {
        self.prefixes.get(&item_id).map(String::as_str)
    }

    /// Whether the composition name of the card is placed after the item name.
    pub fn is_postfix(&self, item_id: ItemId) -> bool {
        self.postfixes.contains(&item_id)
    }
}

/// Composes the display name of an item from its base name, refinement level
/// and card names.
///
/// The official client order is `[+N ][prefixes ]<base name>[ postfixes]`.
/// Cards whose name could not be determined are appended as a bracketed list
/// (`<base name> [Item #id, ...]`) so an unknown card never makes the name
/// unreadable and never silently disappears.
pub fn compose_item_name(
    base_name: &str,
    refinement_level: u8,
    prefixes: &[String],
    postfixes: &[String],
    unresolved_cards: &[String],
) -> String {
    let mut name = String::with_capacity(base_name.len() + prefixes.len() * 8 + postfixes.len() * 8);

    if refinement_level > 0 {
        name.push_str(&format!("+{refinement_level} "));
    }

    for prefix in prefixes {
        if !prefix.is_empty() {
            name.push_str(prefix);
            name.push(' ');
        }
    }

    name.push_str(base_name);

    for postfix in postfixes {
        if !postfix.is_empty() {
            name.push(' ');
            name.push_str(postfix);
        }
    }

    if !unresolved_cards.is_empty() {
        name.push_str(" [");
        name.push_str(&unresolved_cards.join(", "));
        name.push(']');
    }

    name
}

/// Parses an `id#name#` composition table.
///
/// Lines that cannot be parsed are skipped. The names are decoded as EUC-KR
/// (the classic client data encoding) with a UTF-8 fallback for translated
/// tables that are not EUC-KR.
fn parse_table(data: &[u8]) -> Vec<(ItemId, String)> {
    let mut entries = Vec::new();

    for line in data.split(|byte| *byte == b'\n') {
        let line = trim_ascii(line);

        if line.is_empty() {
            continue;
        }

        let Some(separator) = line.iter().position(|byte| *byte == b'#') else {
            continue;
        };

        let id = std::str::from_utf8(&line[..separator])
            .ok()
            .and_then(|id| id.trim().parse::<u32>().ok());

        let Some(id) = id else {
            continue;
        };

        let rest = &line[separator + 1..];
        let name_end = rest.iter().position(|byte| *byte == b'#').unwrap_or(rest.len());
        let name = decode_text(trim_ascii(&rest[..name_end]));

        entries.push((ItemId(id), name));
    }

    entries
}

fn trim_ascii(mut bytes: &[u8]) -> &[u8] {
    while bytes.first().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[1..];
    }

    while bytes.last().is_some_and(u8::is_ascii_whitespace) {
        bytes = &bytes[..bytes.len() - 1];
    }

    bytes
}

/// Decodes classic client text: EUC-KR first, UTF-8 as fallback.
fn decode_text(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    match EUC_KR.decode_without_bom_handling_and_without_replacement(bytes) {
        Some(text) => text.into_owned(),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owned(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn parses_composition_table_lines() {
        let entries = parse_table(b"4001#Lucky#\r\n4014#of Champion#\r\nbroken\r\n4022#Vital#");

        assert_eq!(entries, vec![
            (ItemId(4001), "Lucky".to_owned()),
            (ItemId(4014), "of Champion".to_owned()),
            (ItemId(4022), "Vital".to_owned()),
        ]);
    }

    #[test]
    fn empty_postfix_entries_are_parsed() {
        let entries = parse_table(b"4014#\r\n4022#\r\n");

        assert_eq!(entries, vec![(ItemId(4014), String::new()), (ItemId(4022), String::new())]);
    }

    #[test]
    fn composes_plain_item_name() {
        assert_eq!(compose_item_name("Falchion", 0, &[], &[], &[]), "Falchion");
    }

    #[test]
    fn composes_refinement_prefix() {
        assert_eq!(compose_item_name("Falchion", 7, &[], &[], &[]), "+7 Falchion");
    }

    #[test]
    fn composes_card_prefix_and_postfix() {
        assert_eq!(
            compose_item_name("Falchion", 0, &owned(&["Lucky", "Vital"]), &owned(&["of Champion"]), &[]),
            "Lucky Vital Falchion of Champion"
        );
    }

    #[test]
    fn composes_refinement_cards_and_postfix() {
        assert_eq!(
            compose_item_name("Falchion", 7, &owned(&["Lucky"]), &owned(&["of Champion"]), &[]),
            "+7 Lucky Falchion of Champion"
        );
    }

    #[test]
    fn unresolved_cards_are_shown_in_brackets() {
        assert_eq!(
            compose_item_name("Falchion", 2, &owned(&["Lucky"]), &[], &owned(&["Item #9999", "Item #9998"])),
            "+2 Lucky Falchion [Item #9999, Item #9998]"
        );
    }

    /// Loads the game archives exactly like the client startup does.
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
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn condaro_english_composition_tables_compose_item_names() {
        let game_file_loader = load_game_file_loader();
        let library = crate::world::Library::new(&game_file_loader).expect("failed to load the game library");

        // Falchion (1104) has three card slots.
        assert_eq!(library.item_slot_count(ItemId(1104)), 3);

        // Poring Card = "Lucky" (prefix), Lunatic Card = "Vital" (prefix).
        assert_eq!(
            library.composed_item_name(ItemId(1104), true, 7, &[4001, 4002, 0, 0]),
            "+7 Lucky Vital Falchion"
        );

        // Roda Frog Card = "of Champion" (postfix).
        assert_eq!(
            library.composed_item_name(ItemId(1104), true, 0, &[4014, 0, 0, 0]),
            "Falchion of Champion"
        );

        // Unidentified items reveal neither refinement nor cards.
        let unidentified = library.composed_item_name(ItemId(1104), false, 7, &[4001, 0, 0, 0]);
        assert!(!unidentified.contains("+7"), "unidentified name: {unidentified}");
        assert!(!unidentified.contains("Lucky"), "unidentified name: {unidentified}");

        // Card ids that are not part of the client data degrade to a bracket
        // list instead of disappearing or panicking.
        let unknown = library.composed_item_name(ItemId(1104), true, 0, &[999_999, 0, 0, 0]);
        assert_eq!(unknown, "Falchion [Item #999999]");
    }
}
