//! `InputEvent` handlers for the trade family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::BrowseVendingShop`.
    pub(crate) fn handle_input_browse_vending_shop(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BrowseVendingShop {
            owner_account_id,
            shop_name,
        } = event
        else {
            unreachable!("handle_input_browse_vending_shop called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().vending_browse())
            .set_pending_shop_name(shop_name);
        let _ = self.networking_system.request_vending_list(AccountId(owner_account_id));
    }

    /// `InputEvent::BuyVendingItem`.
    pub(crate) fn handle_input_buy_vending_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BuyVendingItem { cart_index, amount, price } = event else {
            unreachable!("handle_input_buy_vending_item called with a different event variant");
        };

        let owner_account_id = self.client_state.follow(client_state().vending_browse()).owner_account_id();

        if let Some(zeny) = *self.client_state.follow(client_state().zeny())
            && zeny < u64::from(price)
        {
            self.push_chat_warning("You do not have enough zeny.".to_owned());
            return;
        }

        let _ = self
            .networking_system
            .purchase_from_vending(AccountId(owner_account_id), vec![(cart_index, amount)]);

        // rAthena resets its "which shop was opened" state after every
        // purchase request (`clif_parse_PurchaseReq`), so the next row
        // would fail with "store incorrect". Request the (updated) list
        // again to keep the window usable and its stock in sync. The
        // server does not send a success ack; the bought item arrives
        // through the regular inventory packets.
        self.client_state
            .follow_mut(client_state().vending_browse())
            .reduce_stock(cart_index, amount);
        let _ = self.networking_system.request_vending_list(AccountId(owner_account_id));
    }

    /// `InputEvent::VendingOpen`.
    pub(crate) fn handle_input_vending_open(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::VendingOpen = event else {
            unreachable!("handle_input_vending_open called with a different event variant");
        };

        let vending = self.client_state.follow(client_state().vending());
        let shop_name = vending.shop_name().to_owned();
        let items: Vec<ragnarok_packets::StoreItemEntry> = vending
            .entries()
            .iter()
            .map(|entry| ragnarok_packets::StoreItemEntry {
                inventory_index: entry.inventory_index,
                amount: entry.amount,
                price: entry.price,
            })
            .collect();
        if !shop_name.is_empty() && !items.is_empty() {
            let _ = self.networking_system.request_open_store(shop_name, items);
        } else {
            let _ = self.networking_system.cancel_open_store();
        }
        self.client_state.follow_mut(client_state().vending()).close();
        self.interface.close_window_with_class(WindowClass::Vending);
    }

    /// `InputEvent::VendingCancel`.
    pub(crate) fn handle_input_vending_cancel(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::VendingCancel = event else {
            unreachable!("handle_input_vending_cancel called with a different event variant");
        };

        let _ = self.networking_system.cancel_open_store();
        self.client_state.follow_mut(client_state().vending()).close();
        self.interface.close_window_with_class(WindowClass::Vending);
    }

    /// `InputEvent::TradeRequest`.
    pub(crate) fn handle_input_trade_request(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeRequest { account_id } = event else {
            unreachable!("handle_input_trade_request called with a different event variant");
        };

        let _ = self.networking_system.request_trade(ragnarok_packets::AccountId(account_id));
    }

    /// `InputEvent::TradeAccept`.
    pub(crate) fn handle_input_trade_accept(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeAccept { account_id } = event else {
            unreachable!("handle_input_trade_accept called with a different event variant");
        };

        let _ = account_id;
        let _ = self.networking_system.answer_trade(true);
    }

    /// `InputEvent::TradeDecline`.
    pub(crate) fn handle_input_trade_decline(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeDecline = event else {
            unreachable!("handle_input_trade_decline called with a different event variant");
        };

        let trade = self.client_state.follow_mut(client_state().trade());
        trade.clear_request();
        self.interface.close_window_with_class(WindowClass::TradeRequest);
        let _ = self.networking_system.answer_trade(false);
    }

    /// `InputEvent::TradeAddItem`.
    pub(crate) fn handle_input_trade_add_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeAddItem { inventory_index, amount } = event else {
            unreachable!("handle_input_trade_add_item called with a different event variant");
        };

        let _ = self.networking_system.add_trade_item(inventory_index, amount as u32);
    }

    /// `InputEvent::TradeOfferZeny`.
    pub(crate) fn handle_input_trade_offer_zeny(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeOfferZeny = event else {
            unreachable!("handle_input_trade_offer_zeny called with a different event variant");
        };

        let amount = self
            .client_state
            .follow(client_state().trade())
            .zeny()
            .trim()
            .parse::<u32>()
            .unwrap_or(0);
        if amount > 0 {
            let _ = self.networking_system.add_trade_item(0, amount);
        }
    }

    /// `InputEvent::TradeLock`.
    pub(crate) fn handle_input_trade_lock(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeLock = event else {
            unreachable!("handle_input_trade_lock called with a different event variant");
        };

        let _ = self.networking_system.confirm_trade();
        self.client_state.follow_mut(client_state().trade()).set_locked(true);
    }

    /// `InputEvent::TradeCommit`.
    pub(crate) fn handle_input_trade_commit(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeCommit = event else {
            unreachable!("handle_input_trade_commit called with a different event variant");
        };

        let _ = self.networking_system.commit_trade();
    }

    /// `InputEvent::TradeCancel`.
    pub(crate) fn handle_input_trade_cancel(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::TradeCancel = event else {
            unreachable!("handle_input_trade_cancel called with a different event variant");
        };

        let _ = self.networking_system.cancel_trade();
        self.close_trade();
    }
}
