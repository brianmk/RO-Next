use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions, Price, RegularItemFlags};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoMetadata;

#[derive(Clone, Debug)]
pub enum InventoryItemDetails {
    Regular {
        amount: u16,
        equipped_position: EquipPosition,
        flags: RegularItemFlags,
    },
    Equippable {
        equip_position: EquipPosition,
        equipped_position: EquipPosition,
        bind_on_equip_type: u16,
        w_item_sprite_number: u16,
        option_count: u8,
        option_data: [ItemOptions; 5], // fix count
        refinement_level: u8,
        enchantment_level: u8,
        flags: EquippableItemFlags,
    },
}

#[derive(Clone, Debug)]
pub struct InventoryItem<Meta> {
    pub metadata: Meta,
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub details: InventoryItemDetails,
}

impl<Meta> InventoryItem<Meta> {
    /// Whether the item is currently equipped. Equipped items are shown in the
    /// equipment window and hidden from the inventory grid, like in the
    /// original client.
    pub fn is_equipped(&self) -> bool {
        let equipped_position = match &self.details {
            InventoryItemDetails::Regular { equipped_position, .. } => equipped_position,
            InventoryItemDetails::Equippable { equipped_position, .. } => equipped_position,
        };

        !equipped_position.is_empty()
    }

    pub fn is_identified(&self) -> bool {
        match &self.details {
            InventoryItemDetails::Regular { flags, .. } => flags.contains(RegularItemFlags::IDENTIFIED),
            InventoryItemDetails::Equippable { flags, .. } => flags.contains(EquippableItemFlags::IDENTIFIED),
        }
    }

    /// Whether the item was marked as a favorite by the player.
    ///
    /// The 2019 server sends the favorite flag in the item list and pickup
    /// packets; the inventory window shows a marker for favorite items.
    pub fn is_favorite(&self) -> bool {
        match &self.details {
            InventoryItemDetails::Regular { flags, .. } => flags.contains(RegularItemFlags::FAVORITE),
            InventoryItemDetails::Equippable { flags, .. } => flags.contains(EquippableItemFlags::FAVORITE),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemQuantity {
    Fixed(u32),
    Infinite,
}

impl From<u32> for ItemQuantity {
    fn from(value: u32) -> Self {
        match value == !0 {
            true => ItemQuantity::Infinite,
            false => ItemQuantity::Fixed(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopItem<Meta> {
    pub metadata: Meta,
    pub item_id: ItemId,
    pub item_type: u8,
    pub price: Price,
    pub quantity: ItemQuantity,
    pub weight: u16,
    pub location: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SellItem<Meta> {
    pub metadata: Meta,
    pub inventory_index: InventoryIndex,
    pub price: Price,
    pub overcharge_price: Price,
}

/// One row of another player's vending shop
/// (`ZC_PC_PURCHASE_ITEMLIST_FROMMC`, 0x0800 for PACKETVER 20190605).
///
/// The cart slot index is the seller's cart slot, only used to reference the
/// row in the purchase request (`CZ_PC_PURCHASE_ITEMLIST_FROMMC`, 0x0134).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VendingShopItem<Meta> {
    pub item: ShopItem<Meta>,
    pub cart_index: InventoryIndex,
}
