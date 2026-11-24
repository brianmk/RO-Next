//! Display helpers for item data that is not part of the item description.
//!
//! The item description from `iteminfo.lub` only describes the base item. The
//! 2019 server additionally sends the *cards* slotted into a piece of equipment
//! (`EQUIPSLOTINFO`) and its *random options* (`option_data`), and it sends the
//! refinement level of equippable items. None of that is part of the official
//! item description, so the client has to render it itself. This module
//! implements the item name lookups and the display formatting for those
//! details; the actual rendering happens in the item tooltips (inventory,
//! storage, paper doll) and the item info window.

use ro_next_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::{ItemId, ItemOptions};

use super::{Library, ResourceMetadata};

/// Number of card slots of an item (`EQUIPSLOTINFO` in the 2019 packets).
pub const CARD_SLOT_COUNT: usize = 4;

/// Resolved display names of the cards slotted into an item, by card slot.
///
/// An empty slot is `None`; every card id that is present resolves to the same
/// display name as every other item (with `Item #<id>` as the fallback for ids
/// that are not part of the client data, so an unknown card never renders as an
/// empty string or a broken glyph).
pub type CardNames = [Option<String>; CARD_SLOT_COUNT];

/// Resolve the names of the cards that are slotted into an item.
///
/// Unidentified items do not reveal their cards (the official client does not
/// show them either), so every slot is `None` in that case.
pub fn resolve_card_names(library: &Library, item: &InventoryItem<NoMetadata>) -> CardNames {
    let is_identified = item.is_identified();

    std::array::from_fn(|slot| {
        let card_id = item.slot[slot];

        match is_identified {
            true if card_id != 0 => Some(library.item_name(ItemId(card_id), true).into_owned()),
            _ => None,
        }
    })
}

/// State of one of the four card slots of an item in the item description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardSlotState {
    /// A card is slotted into this slot (the card name/icon is available).
    Filled,
    /// The item has this card slot but nothing is slotted into it.
    Empty,
    /// The item does not have this card slot (slot index >= `slotCount`).
    Unavailable,
}

/// Whether the item description should show the card slot row.
///
/// Equipment with card slots always shows all four slots (filled / empty /
/// grayed out). Items without any card slots do not show a slot row.
pub fn shows_card_slots(metadata: &ResourceMetadata) -> bool {
    metadata.slot_count > 0 || metadata.card_names.iter().any(Option::is_some)
}

/// State of the card slot at the given index (0..4).
///
/// A card that the server sent is always shown as filled, even if the client
/// item data claims a lower slot count, so no socketed card can be hidden.
pub fn card_slot_state(metadata: &ResourceMetadata, slot: usize) -> CardSlotState {
    if metadata.card_names.get(slot).is_some_and(Option::is_some) {
        return CardSlotState::Filled;
    }

    match slot < usize::from(metadata.slot_count) {
        true => CardSlotState::Empty,
        false => CardSlotState::Unavailable,
    }
}

/// One display line per card slot, e.g. `Slot 1: Poring Card`,
/// `Slot 2: empty socket` or `Slot 4: unavailable`.
pub fn card_slot_lines(metadata: &ResourceMetadata) -> Vec<String> {
    (0..CARD_SLOT_COUNT)
        .map(|slot| {
            let number = slot + 1;

            match card_slot_state(metadata, slot) {
                CardSlotState::Filled => {
                    let card_name = metadata.card_names[slot].as_deref().unwrap_or("—");
                    format!("Slot {number}: {card_name}")
                }
                CardSlotState::Empty => format!("Slot {number}: empty socket"),
                CardSlotState::Unavailable => format!("Slot {number}: unavailable"),
            }
        })
        .collect()
}

/// Format one random option as a display line.
///
/// Returns `None` for an empty option slot (index `0`). The option id and its
/// value are printed as-is: without the random option name table the client
/// cannot resolve a readable name, and showing the raw id/value is still
/// strictly more useful than hiding the option. The optional `parameter` is
/// only shown when the server actually sent one.
pub fn format_random_option(option: &ItemOptions) -> Option<String> {
    if option.index == 0 {
        return None;
    }

    match option.parameter {
        0 => Some(format!("Random option #{}: {}", option.index, option.value)),
        parameter => Some(format!("Random option #{}: {} (param {parameter})", option.index, option.value)),
    }
}

/// All display lines of the random options of an item.
///
/// Stackable items (and equippable items without options) have no random
/// options; the option slots of the packet are zeroed in that case.
pub fn random_option_lines(details: &InventoryItemDetails) -> Vec<String> {
    let InventoryItemDetails::Equippable { option_data, .. } = details else {
        return Vec::new();
    };

    option_data.iter().filter_map(format_random_option).collect()
}

/// All display lines of the details of an item that are shown in addition to
/// its (composed) name: the card slots and the random options.
pub fn item_detail_lines(item: &InventoryItem<ResourceMetadata>) -> Vec<String> {
    // Unidentified items hide their cards and options, just like the official
    // client.
    if !item.is_identified() {
        return Vec::new();
    }

    let mut lines = Vec::new();

    if shows_card_slots(&item.metadata) {
        lines.extend(card_slot_lines(&item.metadata));
    }

    lines.extend(random_option_lines(&item.details));
    lines
}

/// Appends the item description to a tooltip text.
fn push_description(text: &mut String, metadata: &ResourceMetadata) {
    if metadata.description.is_empty() {
        return;
    }

    text.push_str("\n\n");

    for (index, line) in metadata.description.iter().enumerate() {
        if index != 0 {
            text.push('\n');
        }

        text.push_str(line);
    }
}

/// Tooltip text of an item: its composed display name followed by one line per
/// card slot, the random options, the favorite mark and the full item
/// description.
pub fn item_tooltip_text(item: &InventoryItem<ResourceMetadata>) -> String {
    let mut text = item.metadata.name.clone();

    for line in item_detail_lines(item) {
        text.push('\n');
        text.push_str(&line);
    }

    if item.is_favorite() {
        text.push_str("\nFavorite item");
    }

    push_description(&mut text, &item.metadata);

    text
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ro_next_networking::{InventoryItem, InventoryItemDetails};
    use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions};

    use super::*;
    use crate::world::ItemStats;

    fn empty_option() -> ItemOptions {
        ItemOptions {
            index: 0,
            value: 0,
            parameter: 0,
        }
    }

    fn option(index: u16, value: u16, parameter: u8) -> ItemOptions {
        ItemOptions { index, value, parameter }
    }

    fn metadata_with(card_names: CardNames, slot_count: u8) -> ResourceMetadata {
        ResourceMetadata {
            texture: None,
            name: "Test Sword".to_owned(),
            stats: ItemStats::DEFAULT,
            description: Vec::new(),
            card_names,
            slot_count,
            tooltip_text: Arc::from(""),
        }
    }

    fn equippable(
        refinement_level: u8,
        option_data: [ItemOptions; 5],
        card_names: CardNames,
        slot_count: u8,
    ) -> InventoryItem<ResourceMetadata> {
        InventoryItem {
            metadata: metadata_with(card_names, slot_count),
            index: InventoryIndex(0),
            item_id: ItemId(1104),
            item_type: 5,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                equip_position: EquipPosition::RIGHT_HAND,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 5,
                option_data,
                refinement_level,
                enchantment_level: 0,
                flags: EquippableItemFlags::IDENTIFIED,
            },
        }
    }

    fn no_cards() -> CardNames {
        std::array::from_fn(|_| None)
    }

    #[test]
    fn empty_options_are_skipped() {
        let details = InventoryItemDetails::Equippable {
            equip_position: EquipPosition::RIGHT_HAND,
            equipped_position: EquipPosition::empty(),
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 5,
            option_data: std::array::from_fn(|_| empty_option()),
            refinement_level: 0,
            enchantment_level: 0,
            flags: EquippableItemFlags::empty(),
        };

        assert!(random_option_lines(&details).is_empty());
    }

    #[test]
    fn random_options_are_formatted_without_name_table() {
        let details = InventoryItemDetails::Equippable {
            equip_position: EquipPosition::RIGHT_HAND,
            equipped_position: EquipPosition::empty(),
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 5,
            option_data: [option(12, 5, 0), empty_option(), option(3, 30, 1), empty_option(), empty_option()],
            refinement_level: 0,
            enchantment_level: 0,
            flags: EquippableItemFlags::empty(),
        };

        assert_eq!(random_option_lines(&details), vec![
            "Random option #12: 5".to_owned(),
            "Random option #3: 30 (param 1)".to_owned(),
        ]);
    }

    #[test]
    fn regular_items_have_no_random_options() {
        let details = InventoryItemDetails::Regular {
            amount: 10,
            equipped_position: EquipPosition::empty(),
            flags: ragnarok_packets::RegularItemFlags::empty(),
        };

        assert!(random_option_lines(&details).is_empty());
    }

    #[test]
    fn card_slot_states_cover_filled_empty_and_unavailable() {
        // Two-slot item with a card in slot 1: slot 2 is a normal empty socket,
        // slots 3 and 4 are unavailable/grayed out.
        let card_names: CardNames = [Some("Poring Card".to_owned()), None, None, None];
        let metadata = metadata_with(card_names, 2);

        assert!(shows_card_slots(&metadata));
        assert_eq!(card_slot_state(&metadata, 0), CardSlotState::Filled);
        assert_eq!(card_slot_state(&metadata, 1), CardSlotState::Empty);
        assert_eq!(card_slot_state(&metadata, 2), CardSlotState::Unavailable);
        assert_eq!(card_slot_state(&metadata, 3), CardSlotState::Unavailable);
        assert_eq!(card_slot_lines(&metadata), vec![
            "Slot 1: Poring Card".to_owned(),
            "Slot 2: empty socket".to_owned(),
            "Slot 3: unavailable".to_owned(),
            "Slot 4: unavailable".to_owned(),
        ]);
    }

    #[test]
    fn card_slots_always_show_four_entries_and_never_hide_a_card() {
        let card_names: CardNames = [None, None, Some("Poring Card".to_owned()), None];

        // The item data says two slots but the server sent a card in slot 3:
        // the card is still shown as filled (and slot 4 stays unavailable).
        let metadata = metadata_with(card_names, 2);
        let lines = card_slot_lines(&metadata);

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[2], "Slot 3: Poring Card");
        assert_eq!(lines[3], "Slot 4: unavailable");

        // Items without card slots and without cards do not show a slot row.
        let metadata = metadata_with(no_cards(), 0);
        assert!(!shows_card_slots(&metadata));
        assert!(card_slot_lines(&metadata).iter().all(|line| line.ends_with("unavailable")));
    }

    #[test]
    fn unslotted_items_have_no_card_slot_row_in_the_tooltip() {
        let mut item = equippable(0, std::array::from_fn(|_| empty_option()), no_cards(), 0);
        item.metadata.description = vec!["A potion.".to_owned()];

        assert_eq!(item_tooltip_text(&item), "Test Sword\n\nA potion.");
    }

    #[test]
    fn detail_lines_include_card_slots_and_options() {
        let card_names: CardNames = [Some("Poring Card".to_owned()), None, Some("Item #4002".to_owned()), None];
        let item = equippable(
            7,
            [option(12, 5, 0), empty_option(), empty_option(), empty_option(), empty_option()],
            card_names,
            3,
        );

        assert_eq!(item_detail_lines(&item), vec![
            "Slot 1: Poring Card".to_owned(),
            "Slot 2: empty socket".to_owned(),
            "Slot 3: Item #4002".to_owned(),
            "Slot 4: unavailable".to_owned(),
            "Random option #12: 5".to_owned(),
        ]);
    }

    #[test]
    fn tooltip_text_contains_name_slots_and_description() {
        let item = equippable(7, std::array::from_fn(|_| empty_option()), no_cards(), 3);
        let mut item = item;
        item.metadata.name = "+7 Lucky Test Sword of Champion".to_owned();
        item.metadata.description = vec!["A test sword.".to_owned(), String::new(), "Type: Sword".to_owned()];

        assert_eq!(
            item_tooltip_text(&item),
            "+7 Lucky Test Sword of Champion\nSlot 1: empty socket\nSlot 2: empty socket\nSlot 3: empty socket\nSlot 4: unavailable\n\nA \
             test sword.\n\nType: Sword"
        );
    }

    #[test]
    fn unidentified_items_hide_cards_but_keep_the_description() {
        let card_names: CardNames = [Some("Poring Card".to_owned()), None, None, None];
        let mut item = equippable(
            7,
            [option(12, 5, 0), empty_option(), empty_option(), empty_option(), empty_option()],
            card_names,
            3,
        );

        if let InventoryItemDetails::Equippable { flags, .. } = &mut item.details {
            *flags = EquippableItemFlags::empty();
        }

        item.metadata.description = vec!["Can be identified by using a Magnifier.".to_owned()];

        assert!(!item.is_identified());
        assert!(item_detail_lines(&item).is_empty());
        assert_eq!(
            item_tooltip_text(&item),
            "Test Sword\n\nCan be identified by using a Magnifier."
        );
    }
}
