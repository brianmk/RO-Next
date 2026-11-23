//! Player-to-player trade state (`CZ_REQ_EXCHANGE_ITEM` and the 2019 exchange
//! packets). The server owns the authoritative item lists; this state mirrors
//! the local offer for the trade window and the pending request prompt.

use korangar_interface::element::StateElement;
use rust_state::RustState;

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct TradeItem {
    pub inventory_index: u16,
    pub name: String,
    pub amount: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct TradeRequestInfo {
    pub name: String,
    pub account_id: u32,
}

#[derive(Default, RustState, StateElement)]
pub struct Trade {
    #[hidden_element]
    request: Option<TradeRequestInfo>,
    #[hidden_element]
    partner_name: String,
    #[hidden_element]
    active: bool,
    #[hidden_element]
    items: Vec<TradeItem>,
    zeny: String,
    #[hidden_element]
    locked: bool,
    #[hidden_element]
    partner_locked: bool,
}

impl Trade {
    pub fn set_request(&mut self, name: String, account_id: u32) {
        self.request = Some(TradeRequestInfo { name, account_id });
        self.active = false;
        self.items.clear();
        self.zeny = "0".to_string();
        self.locked = false;
        self.partner_locked = false;
    }

    pub fn request(&self) -> Option<&TradeRequestInfo> {
        self.request.as_ref()
    }

    pub fn clear_request(&mut self) {
        self.request = None;
    }

    pub fn open(&mut self, partner_name: String) {
        self.request = None;
        self.partner_name = partner_name;
        self.active = true;
        self.items.clear();
        self.zeny = "0".to_string();
        self.locked = false;
        self.partner_locked = false;
    }

    pub fn close(&mut self) {
        self.active = false;
        self.request = None;
        self.items.clear();
        self.zeny = "0".to_string();
        self.locked = false;
        self.partner_locked = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn partner_name(&self) -> &str {
        &self.partner_name
    }

    pub fn items(&self) -> &[TradeItem] {
        &self.items
    }

    pub fn add_item(&mut self, inventory_index: u16, name: String, amount: u16) {
        if let Some(item) = self.items.iter_mut().find(|item| item.inventory_index == inventory_index) {
            item.amount = item.amount.saturating_add(amount);
        } else {
            self.items.push(TradeItem {
                inventory_index,
                name,
                amount,
            });
        }
    }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    pub fn set_partner_locked(&mut self, locked: bool) {
        self.partner_locked = locked;
    }

    pub fn partner_locked(&self) -> bool {
        self.partner_locked
    }
}

#[cfg(test)]
mod tests {
    use super::Trade;

    #[test]
    fn offers_merge_and_close_resets() {
        let mut trade = Trade::default();
        trade.open("Partner".into());
        trade.add_item(1, "Red Potion".into(), 1);
        trade.add_item(1, "Red Potion".into(), 2);
        assert_eq!(trade.items().len(), 1);
        assert_eq!(trade.items()[0].amount, 3);
        trade.close();
        assert!(!trade.is_active());
        assert!(trade.items().is_empty());
    }
}
