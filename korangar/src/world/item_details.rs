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

use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
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

/// Number of card slots that are shown for an item.
///
/// The client data (`slotCount` in `iteminfo.lub`) is authoritative. If the
/// server sends cards for a slot that the item data does not know about, the
/// slot is still shown so that no socketed card is hidden.
pub fn visible_card_slot_count(metadata: &ResourceMetadata) -> usize {
    let highest_used_slot = metadata
        .card_names
        .iter()
        .rposition(Option::is_some)
        .map(|slot| slot + 1)
        .unwrap_or(0);

    usize::from(metadata.slot_count).max(highest_used_slot).min(CARD_SLOT_COUNT)
}

/// One display line per visible card slot, e.g. `Slot 1: Poring Card`;
/// empty slots are shown as `Slot 2: —`.
pub fn card_slot_lines(metadata: &ResourceMetadata) -> Vec<String> {
    (0..visible_card_slot_count(metadata))
        .map(|slot| {
            let card_name = metadata.card_names[slot].as_deref().unwrap_or("—");
            format!("Slot {}: {card_name}", slot + 1)
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

    let mut lines = card_slot_lines(&item.metadata);
    lines.extend(random_option_lines(&item.details));
    lines
}

/// Tooltip text of an item: its composed display name followed by one line per
/// card slot, the random options and the favorite mark.
pub fn item_tooltip_text(item: &InventoryItem<ResourceMetadata>) -> String {
    let mut text = item.metadata.name.clone();

    for line in item_detail_lines(item) {
        text.push('\n');
        text.push_str(&line);
    }

    if item.is_favorite() {
        text.push_str("\nFavorite item");
    }

    text
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use korangar_networking::{InventoryItem, InventoryItemDetails};
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
    fn card_slots_show_names_and_empty_slots() {
        let card_names: CardNames = [Some("Poring Card".to_owned()), None, Some("Item #4002".to_owned()), None];
        let metadata = metadata_with(card_names, 4);

        assert_eq!(card_slot_lines(&metadata), vec![
            "Slot 1: Poring Card".to_owned(),
            "Slot 2: —".to_owned(),
            "Slot 3: Item #4002".to_owned(),
            "Slot 4: —".to_owned(),
        ]);
    }

    #[test]
    fn card_slots_use_the_item_slot_count_and_never_hide_a_card() {
        let card_names: CardNames = [None, None, Some("Poring Card".to_owned()), None];

        // The item data says two slots but the server sent a card in slot 3.
        let metadata = metadata_with(card_names, 2);
        assert_eq!(card_slot_lines(&metadata), vec![
            "Slot 1: —".to_owned(),
            "Slot 2: —".to_owned(),
            "Slot 3: Poring Card".to_owned()
        ]);

        // Items without card slots and without cards show nothing.
        let metadata = metadata_with(no_cards(), 0);
        assert!(card_slot_lines(&metadata).is_empty());
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
            "Slot 2: —".to_owned(),
            "Slot 3: Item #4002".to_owned(),
            "Random option #12: 5".to_owned(),
        ]);
    }

    #[test]
    fn tooltip_text_starts_with_the_composed_item_name() {
        let item = equippable(7, std::array::from_fn(|_| empty_option()), no_cards(), 3);
        let mut item = item;
        item.metadata.name = "+7 Lucky Test Sword of Champion".to_owned();

        assert_eq!(
            item_tooltip_text(&item),
            "+7 Lucky Test Sword of Champion\nSlot 1: —\nSlot 2: —\nSlot 3: —"
        );
    }

    #[test]
    fn unidentified_items_hide_their_details() {
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

        assert!(!item.is_identified());
        assert!(item_detail_lines(&item).is_empty());
        assert_eq!(item_tooltip_text(&item), "Test Sword");
    }
}
