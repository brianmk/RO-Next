use std::sync::Arc;

use korangar_interface::element::StateElement;
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

    match item.item_type {
        item_type::HEALING | item_type::USABLE | item_type::DELAY_CONSUME | item_type::CASH | item_type::CASH_OLD => {
            Some(ItemUseAction::Use)
        }
        _ => None,
    }
}

/// Item type values as sent by the server (`enum item_types` in rAthena's
/// `common/mmo.hpp`).
pub mod item_type {
    /// Healing items (potions, herbs, ...).
    pub const HEALING: u8 = 0;
    /// Usable items (usable from the inventory, e.g. some boxes/scrolls).
    pub const USABLE: u8 = 2;
    /// Miscellaneous items.
    pub const ETC: u8 = 3;
    /// Armor (and shields/headgear/garments/shoes/accessories).
    pub const ARMOR: u8 = 4;
    /// Weapons.
    pub const WEAPON: u8 = 5;
    /// Cards.
    pub const CARD: u8 = 6;
    /// Pet eggs.
    pub const PET_EGG: u8 = 7;
    /// Pet armor.
    pub const PET_ARMOR: u8 = 8;
    /// Ammunition.
    pub const AMMO: u8 = 10;
    /// Items that are consumed after a delay (e.g. some buff items).
    pub const DELAY_CONSUME: u8 = 11;
    /// Cash shop consumables.
    ///
    /// Older rAthena versions use 12 for this and current versions use 18. In
    /// the fallback below both are treated as consumables: shadow gear (12 in
    /// current versions) is equippable and therefore classified before the
    /// switch.
    pub const CASH_OLD: u8 = 12;
    pub const CASH: u8 = 18;
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

    match item.item_type {
        item_type::ARMOR | item_type::WEAPON | item_type::PET_ARMOR | item_type::AMMO => ItemCategory::Equipments,
        item_type::HEALING | item_type::USABLE | item_type::DELAY_CONSUME | item_type::CASH | item_type::CASH_OLD => {
            ItemCategory::Consumables
        }
        item_type::ETC | item_type::CARD | item_type::PET_EGG => ItemCategory::Misc,
        // Unknown item types (`IT_UNKNOWN`, `IT_UNKNOWN2`, ...) are put into the
        // misc tab so that no item can disappear from the inventory.
        _ => ItemCategory::Misc,
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
