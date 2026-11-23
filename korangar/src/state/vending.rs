//! Player-created vending shop state (`MC_VENDING`).
//!
//! The server sends `ZC_OPENSTORE` after the cast; the client shows this shop
//! editor. Entries are inventory-index/amount/price tuples, exactly what the
//! `CZ_REQ_OPENSTORE` packet takes. Keeping the state in `ClientState` lets the
//! window survive one frame like the other server-driven windows.

use korangar_interface::element::StateElement;
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

    pub fn is_active(&self) -> bool {
        self.active
    }

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

    pub fn remove_entry(&mut self, inventory_index: u16) {
        self.entries.retain(|entry| entry.inventory_index != inventory_index);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.active = false;
    }
}

#[cfg(test)]
mod tests {
    use super::{Vending, VendingEntry};

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
}
