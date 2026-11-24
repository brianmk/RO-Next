use std::sync::Arc;

use korangar_interface::element::StateElement;
pub use korangar_networking::item_type;
use korangar_networking::item_type::ItemTypeCategory;
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::{EquipPosition, InventoryIndex, ItemId};
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::Texture;
use crate::loaders::AsyncLoader;
use crate::world::ResourceMetadata;

/// The tab an item is shown in by the inventory window, like the original
/// client that splits the inventory into consumables, equipment and misc.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum ItemCategory {
    #[default]
    Consumables,
    Equipments,
    Misc,
}

/// What the "use/equip" action of an inventory item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemUseAction {
    /// Use the item (consumables, usable items, ...).
    Use,
    /// Equip the item to the given equipment position.
    Equip(EquipPosition),
}

/// Returns the action the "use/equip" entry of the item context menu performs,
/// or `None` if the item cannot be used or equipped.
pub fn item_use_action<Meta>(item: &InventoryItem<Meta>) -> Option<ItemUseAction> {
    if let InventoryItemDetails::Equippable { equip_position, .. } = &item.details
        && !equip_position.is_empty()
    {
        return Some(ItemUseAction::Equip(*equip_position));
    }

    match item_type::is_usable(item.item_type) {
        true => Some(ItemUseAction::Use),
        false => None,
    }
}

/// Returns the inventory tab an item belongs to.
///
/// Items that the server marked as equippable (they have an equipment position)
/// are always equipment, which is more reliable than the item type across
/// server versions. Everything else is classified by the item type.
pub fn item_category<Meta>(item: &InventoryItem<Meta>) -> ItemCategory {
    if let InventoryItemDetails::Equippable { equip_position, .. } = &item.details
        && !equip_position.is_empty()
    {
        return ItemCategory::Equipments;
    }

    match item_type::category(item.item_type) {
        ItemTypeCategory::Equipments => ItemCategory::Equipments,
        ItemTypeCategory::Consumables => ItemCategory::Consumables,
        ItemTypeCategory::Misc => ItemCategory::Misc,
    }
}

#[derive(Default, RustState, StateElement)]
pub struct Inventory {
    // TODO: Unhide this.
    #[hidden_element]
    items: Vec<InventoryItem<ResourceMetadata>>,
}

impl Inventory {
    pub fn fill(&mut self, async_loader: &AsyncLoader, items: Vec<InventoryItem<NoMetadata>>) {
        self.items = items
            .into_iter()
            .map(|item| async_loader.request_inventory_item_metadata_load(item))
            .collect();
    }

    pub fn add_item(&mut self, async_loader: &AsyncLoader, item: InventoryItem<NoMetadata>) {
        if let Some(found_item) = self.items.iter_mut().find(|inventory_item| inventory_item.index == item.index) {
            let InventoryItemDetails::Regular { amount, .. } = &mut found_item.details else {
                panic!();
            };

            let InventoryItemDetails::Regular { amount: added_amount, .. } = item.details else {
                panic!();
            };

            *amount += added_amount;
        } else {
            let item = async_loader.request_inventory_item_metadata_load(item);

            self.items.push(item);
        }
    }

    /// Remove all items, e.g. when the storage window is closed.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn update_item_sprite(&mut self, item_id: ItemId, texture: Arc<Texture>) {
        self.items.iter_mut().filter(|item| item.item_id == item_id).for_each(|item| {
            item.metadata.texture = Some(texture.clone());
        });
    }

    pub fn remove_item(&mut self, index: InventoryIndex, remove_amount: u16) {
        let position = self
            .items
            .iter()
            .position(|item| item.index == index)
            .expect("item not in inventory");

        if let InventoryItemDetails::Regular { amount, .. } = &mut self.items[position].details
            && *amount > remove_amount
        {
            *amount -= remove_amount;
            return;
        }

        self.items.remove(position);
    }

    /// Like [`Self::remove_item`], but silently ignores an unknown index.
    ///
    /// The cart list can arrive later than a cart delete packet (for example
    /// after a reconnect), and a malformed server packet must not take the
    /// client down.
    pub fn remove_item_if_present(&mut self, index: InventoryIndex, remove_amount: u16) {
        if !self.items.iter().any(|item| item.index == index) {
            return;
        }

        self.remove_item(index, remove_amount);
    }

    /// Updates the amount of a stackable item, e.g. after using one of the
    /// items. Removes the item if the new amount is zero.
    pub fn update_amount(&mut self, index: InventoryIndex, amount: u16) {
        if amount == 0 {
            self.items.retain(|item| item.index != index);
            return;
        }

        let Some(item) = self.items.iter_mut().find(|item| item.index == index) else {
            return;
        };

        if let InventoryItemDetails::Regular { amount: item_amount, .. } = &mut item.details {
            *item_amount = amount;
        }
    }

    pub fn update_equipped_position(&mut self, index: InventoryIndex, new_equipped_position: EquipPosition) {
        let item = self.items.iter_mut().find(|item| item.index == index).unwrap();

        let InventoryItemDetails::Equippable { equipped_position, .. } = &mut item.details else {
            // This can happen for ammunition for example.
            return;
        };

        *equipped_position = new_equipped_position;
    }
}

#[cfg(test)]
mod tests {
    use korangar_networking::NoMetadata;
    use ragnarok_packets::{InventoryIndex, ItemId, RegularItemFlags};

    use super::*;

    fn item(item_type: u8) -> InventoryItem<NoMetadata> {
        InventoryItem {
            metadata: NoMetadata,
            index: InventoryIndex(0),
            item_id: ItemId(501),
            item_type,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Regular {
                amount: 1,
                equipped_position: EquipPosition::empty(),
                flags: RegularItemFlags::IDENTIFIED,
            },
        }
    }

    #[test]
    fn tabs_and_use_action_use_the_single_item_type_source() {
        // The item type classification lives in `korangar_networking::item_type`;
        // the inventory must not carry a second copy of the constants.
        for (item_type, category) in [
            (item_type::HEALING, ItemCategory::Consumables),
            (item_type::USABLE, ItemCategory::Consumables),
            (item_type::DELAY_CONSUME, ItemCategory::Consumables),
            (item_type::CASH, ItemCategory::Consumables),
            (item_type::CASH_OLD, ItemCategory::Consumables),
            (item_type::ARMOR, ItemCategory::Equipments),
            (item_type::WEAPON, ItemCategory::Equipments),
            (item_type::PET_ARMOR, ItemCategory::Equipments),
            (item_type::AMMO, ItemCategory::Equipments),
            (item_type::ETC, ItemCategory::Misc),
            (item_type::CARD, ItemCategory::Misc),
            (item_type::PET_EGG, ItemCategory::Misc),
            (item_type::UNKNOWN, ItemCategory::Misc),
            (item_type::UNKNOWN2, ItemCategory::Misc),
        ] {
            assert_eq!(item_category(&item(item_type)), category, "item type {item_type}");
        }

        assert_eq!(item_use_action(&item(item_type::HEALING)), Some(ItemUseAction::Use));
        assert_eq!(item_use_action(&item(item_type::USABLE)), Some(ItemUseAction::Use));
        assert_eq!(item_use_action(&item(item_type::CASH)), Some(ItemUseAction::Use));
        assert_eq!(item_use_action(&item(item_type::WEAPON)), None);
        assert_eq!(item_use_action(&item(item_type::ETC)), None);
    }

    #[test]
    fn equippable_details_win_over_the_item_type() {
        // The server marks equipment with an equipment position; that is more
        // reliable than the item type across server versions.
        let mut equippable = item(item_type::ETC);
        equippable.details = InventoryItemDetails::Equippable {
            equip_position: EquipPosition::RIGHT_HAND,
            equipped_position: EquipPosition::empty(),
            bind_on_equip_type: 0,
            w_item_sprite_number: 0,
            option_count: 0,
            option_data: std::array::from_fn(|_| ragnarok_packets::ItemOptions {
                index: 0,
                value: 0,
                parameter: 0,
            }),
            refinement_level: 0,
            enchantment_level: 0,
            flags: ragnarok_packets::EquippableItemFlags::IDENTIFIED,
        };

        assert_eq!(item_category(&equippable), ItemCategory::Equipments);
        assert_eq!(
            item_use_action(&equippable),
            Some(ItemUseAction::Equip(EquipPosition::RIGHT_HAND))
        );
    }
}
