//! Player-created vending shop state (`MC_VENDING`).
//!
//! The server sends `ZC_OPENSTORE` after the cast; the client shows this shop
//! editor. Entries are inventory-index/amount/price tuples, exactly what the
//! `CZ_REQ_OPENSTORE` packet takes. Keeping the state in `ClientState` lets the
//! window survive one frame like the other server-driven windows.

use ragnarok_packets::InventoryIndex;
use ro_next_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

/// One item put up for sale.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VendingEntry {
    pub inventory_index: u16,
    pub amount: u16,
    pub price: u32,
    /// Display name, copied when the entry is added.
    pub name: String,
}

/// The open vending-shop editor.
#[derive(Clone, Default, PartialEq, RustState, StateElement, Serialize, Deserialize)]
pub struct Vending {
    #[hidden_element]
    active: bool,
    #[hidden_element]
    slots: u16,
    #[hidden_element]
    shop_name: String,
    #[hidden_element]
    price: String,
    #[hidden_element]
    entries: Vec<VendingEntry>,
}

impl Vending {
    pub fn open(&mut self, slots: u16) {
        self.active = true;
        self.slots = slots;
        self.shop_name = "My Shop".to_string();
        self.price = "1".to_string();
        self.entries.clear();
    }

    pub fn close(&mut self) {
        self.active = false;
        self.entries.clear();
    }

    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        self.active
    }

    #[cfg(test)]
    pub fn slots(&self) -> u16 {
        self.slots
    }

    pub fn entries(&self) -> &[VendingEntry] {
        &self.entries
    }

    pub fn shop_name(&self) -> &str {
        &self.shop_name
    }

    pub fn add_entry(&mut self, entry: VendingEntry) {
        if self.entries.len() >= self.slots as usize {
            return;
        }
        if let Some(existing) = self
            .entries
            .iter_mut()
            .find(|existing| existing.inventory_index == entry.inventory_index)
        {
            existing.amount = existing.amount.saturating_add(entry.amount);
            existing.price = entry.price;
            return;
        }
        self.entries.push(entry);
    }

    #[cfg(test)]
    pub fn remove_entry(&mut self, inventory_index: u16) {
        self.entries.retain(|entry| entry.inventory_index != inventory_index);
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.active = false;
    }
}

/// One row of another player's vending shop shown in the browse window.
///
/// The server list (`ZC_PC_PURCHASE_ITEMLIST_FROMMC`, 0x0800) carries the
/// seller's cart slot; the client has to echo it back in
/// `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (0x0134) to buy that row.
#[derive(Clone, Debug, PartialEq, Eq, RustState, StateElement)]
pub struct VendingShopItem {
    pub item_id: u32,
    pub name: String,
    pub price: u32,
    pub stock: u16,
    pub cart_index: InventoryIndex,
}

impl VendingShopItem {
    pub fn new(item_id: u32, name: String, price: u32, stock: u16, cart_index: InventoryIndex) -> Self {
        Self {
            item_id,
            name,
            price,
            stock,
            cart_index,
        }
    }
}

/// The vending shop of another player that is currently open in the browse
/// window.
#[derive(Default, RustState, StateElement)]
pub struct VendingBrowse {
    #[hidden_element]
    active: bool,
    #[hidden_element]
    owner_account_id: u32,
    #[hidden_element]
    shop_id: u32,
    #[hidden_element]
    shop_name: String,
    /// Title of the shop whose list was requested. Used as a fallback when the
    /// shop owner entity is not known when the list arrives.
    #[hidden_element]
    pending_shop_name: String,
    #[hidden_element]
    items: Vec<VendingShopItem>,
}

impl VendingBrowse {
    pub fn open(&mut self, owner_account_id: u32, shop_id: u32, shop_name: String, items: Vec<VendingShopItem>) {
        self.active = true;
        self.owner_account_id = owner_account_id;
        self.shop_id = shop_id;
        self.shop_name = shop_name;
        self.pending_shop_name.clear();
        self.items = items;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.owner_account_id = 0;
        self.shop_id = 0;
        self.shop_name.clear();
        self.pending_shop_name.clear();
        self.items.clear();
    }

    /// Remember the title of the shop the player clicked before the server
    /// answers with the item list.
    pub fn set_pending_shop_name(&mut self, shop_name: String) {
        self.pending_shop_name = shop_name;
    }

    pub fn pending_shop_name(&self) -> &str {
        &self.pending_shop_name
    }

    #[cfg(test)]
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn owner_account_id(&self) -> u32 {
        self.owner_account_id
    }

    #[cfg(test)]
    pub fn shop_id(&self) -> u32 {
        self.shop_id
    }

    #[cfg(test)]
    pub fn shop_name(&self) -> &str {
        &self.shop_name
    }

    pub fn items(&self) -> &[VendingShopItem] {
        &self.items
    }

    /// Reduce the locally shown stock after a purchase. The server does not
    /// send an updated shop list on success (only the buyer's inventory and
    /// zeny change), so the browse list has to update itself.
    pub fn reduce_stock(&mut self, cart_index: InventoryIndex, amount: u16) {
        let Some(item) = self.items.iter_mut().find(|item| item.cart_index == cart_index) else {
            return;
        };

        item.stock = item.stock.saturating_sub(amount);
        self.items.retain(|item| item.stock > 0);
    }
}

#[cfg(test)]
mod tests {
    use super::{Vending, VendingBrowse, VendingEntry, VendingShopItem};

    #[test]
    fn entries_are_capped_by_slots_and_stacks_merge() {
        let mut vending = Vending::default();
        vending.open(2);
        vending.add_entry(VendingEntry {
            inventory_index: 1,
            amount: 1,
            price: 100,
            name: "Red Potion".into(),
        });
        vending.add_entry(VendingEntry {
            inventory_index: 1,
            amount: 2,
            price: 150,
            name: "Red Potion".into(),
        });
        assert_eq!(vending.entries().len(), 1);
        assert_eq!(vending.entries()[0].amount, 3);
        assert_eq!(vending.entries()[0].price, 150);
        vending.add_entry(VendingEntry {
            inventory_index: 2,
            amount: 1,
            price: 1,
            name: "Apple".into(),
        });
        vending.add_entry(VendingEntry {
            inventory_index: 3,
            amount: 1,
            price: 1,
            name: "Knife".into(),
        });
        assert_eq!(vending.entries().len(), 2);
        vending.remove_entry(1);
        assert_eq!(vending.entries()[0].inventory_index, 2);
    }

    #[test]
    fn browse_purchase_reduces_stock_and_drops_sold_out_rows() {
        use ragnarok_packets::InventoryIndex;

        let mut browse = VendingBrowse::default();
        browse.open(42, 7, "Potion Shop".into(), vec![
            VendingShopItem::new(501, "Red Potion".into(), 50, 2, InventoryIndex(0)),
            VendingShopItem::new(502, "Apple".into(), 10, 1, InventoryIndex(1)),
        ]);

        assert!(browse.is_active());
        browse.reduce_stock(InventoryIndex(0), 1);
        assert_eq!(browse.items()[0].stock, 1);
        browse.reduce_stock(InventoryIndex(0), 1);
        assert_eq!(browse.items().len(), 1);
        assert_eq!(browse.items()[0].cart_index, InventoryIndex(1));

        browse.close();
        assert!(!browse.is_active());
        assert!(browse.items().is_empty());
    }
}
