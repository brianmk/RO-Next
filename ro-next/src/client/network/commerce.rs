//! `NetworkEvent` handlers for the commerce family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::BankOpened`.
    pub(crate) fn handle_bank_opened(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::BankOpened = event else {
            unreachable!("handle_bank_opened called with a different event variant");
        };

        if !self.interface.is_window_with_class_open(WindowClass::Bank) {
            self.interface.open_window(BankWindow);
        }

        self.client_state.follow_mut(client_state().bank()).set_status("Bank opened.");
    }

    /// `NetworkEvent::BankBalance`.
    pub(crate) fn handle_bank_balance(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::BankBalance { money, reason } = event else {
            unreachable!("handle_bank_balance called with a different event variant");
        };

        let bank = self.client_state.follow_mut(client_state().bank());
        bank.set_balance(money);
        bank.set_status(match reason {
            0 => "Bank ready.",
            _ => "The bank is not available on this map or server.",
        });

        if !self.interface.is_window_with_class_open(WindowClass::Bank) {
            self.interface.open_window(BankWindow);
        }
    }

    /// `NetworkEvent::BankDepositResult`.
    pub(crate) fn handle_bank_deposit_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::BankDepositResult { reason, money, zeny } = event else {
            unreachable!("handle_bank_deposit_result called with a different event variant");
        };

        self.client_state.follow_mut(client_state().bank()).apply_deposit(reason, money);

        // The ack carries the carried zeny after the transaction; the
        // regular 0x00B1 zeny update usually follows as well.
        if let Ok(zeny) = u64::try_from(zeny) {
            *self.client_state.follow_mut(client_state().zeny()) = Some(zeny);
        }

        if !self.interface.is_window_with_class_open(WindowClass::Bank) {
            self.interface.open_window(BankWindow);
        }
    }

    /// `NetworkEvent::BankWithdrawResult`.
    pub(crate) fn handle_bank_withdraw_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::BankWithdrawResult { reason, money, zeny } = event else {
            unreachable!("handle_bank_withdraw_result called with a different event variant");
        };

        self.client_state.follow_mut(client_state().bank()).apply_withdraw(reason, money);

        if let Ok(zeny) = u64::try_from(zeny) {
            *self.client_state.follow_mut(client_state().zeny()) = Some(zeny);
        }

        if !self.interface.is_window_with_class_open(WindowClass::Bank) {
            self.interface.open_window(BankWindow);
        }
    }

    /// `NetworkEvent::BankClosed`.
    pub(crate) fn handle_bank_closed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::BankClosed = event else {
            unreachable!("handle_bank_closed called with a different event variant");
        };

        self.client_state.follow_mut(client_state().bank()).close();
        self.interface.close_window_with_class(WindowClass::Bank);
    }

    /// `NetworkEvent::StoreEntry`.
    pub(crate) fn handle_store_entry(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::StoreEntry {
            owner_account_id,
            shop_name,
        } = event
        else {
            unreachable!("handle_store_entry called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id().0 == owner_account_id.0)
        {
            entity.set_vending_shop_name(Some(shop_name));
        }
    }

    /// `NetworkEvent::StoreEntryRemoved`.
    pub(crate) fn handle_store_entry_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::StoreEntryRemoved { owner_account_id } = event else {
            unreachable!("handle_store_entry_removed called with a different event variant");
        };

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id().0 == owner_account_id.0)
        {
            entity.set_vending_shop_name(None);
        }
    }

    /// `NetworkEvent::VendingShopList`.
    pub(crate) fn handle_vending_shop_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::VendingShopList {
            owner_account_id,
            shop_id,
            items,
        } = event
        else {
            unreachable!("handle_vending_shop_list called with a different event variant");
        };

        let shop_name = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id().0 == owner_account_id.0)
            .and_then(|entity| entity.get_vending_shop_name())
            .map(ToOwned::to_owned)
            .or_else(|| {
                let pending = self
                    .client_state
                    .follow(client_state().vending_browse())
                    .pending_shop_name()
                    .to_owned();
                (!pending.is_empty()).then_some(pending)
            })
            .unwrap_or_else(|| "Vending shop".to_owned());

        let items = items
            .into_iter()
            .map(|entry| {
                let stock = match entry.item.quantity {
                    ItemQuantity::Fixed(stock) => u16::try_from(stock).unwrap_or(u16::MAX),
                    ItemQuantity::Infinite => u16::MAX,
                };
                let item = self.async_loader.request_shop_item_metadata_load(entry.item);

                VendingShopItem::new(
                    item.item_id.0,
                    item.metadata.name.clone(),
                    item.price.0,
                    stock,
                    entry.cart_index,
                )
            })
            .collect();

        self.client_state
            .follow_mut(client_state().vending_browse())
            .open(owner_account_id.0, shop_id, shop_name, items);

        if !self.interface.is_window_with_class_open(WindowClass::VendingBrowse) {
            self.interface
                .open_window(VendingBrowseWindow::new(client_state().vending_browse()));
        }
    }

    /// `NetworkEvent::VendingPurchaseResult`.
    pub(crate) fn handle_vending_purchase_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::VendingPurchaseResult { result } = event else {
            unreachable!("handle_vending_purchase_result called with a different event variant");
        };

        // The 2019 server only sends this ack for failures, but some
        // servers also send result 0 (success); do not report that as an
        // error.
        if result != 0 {
            self.push_chat_information(format_message(MessageDomain::PurchaseFromMc, u16::from(result), &[]));
        }
    }

    /// `NetworkEvent::OpenShop`.
    pub(crate) fn handle_open_shop(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::OpenShop { items } = event else {
            unreachable!("handle_open_shop called with a different event variant");
        };

        self.shop_open = true;

        // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
        // the regular `DialogMenuPacket`. When opening the shop that dialog should be
        // closed.
        self.client_state.follow_mut(client_state().dialog_window()).end();
        self.interface.close_window_with_class(WindowClass::Dialog);

        *self.client_state.follow_mut(client_state().shop_items()) = items
            .into_iter()
            .map(|item| self.async_loader.request_shop_item_metadata_load(item))
            .collect();

        self.interface
            .open_window(BuyWindow::new(client_state().shop_items(), client_state().buy_cart()));
        self.interface.open_window(BuyCartWindow::new(client_state().buy_cart()));
    }

    /// `NetworkEvent::AskBuyOrSell`.
    pub(crate) fn handle_ask_buy_or_sell(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::AskBuyOrSell { shop_id } = event else {
            unreachable!("handle_ask_buy_or_sell called with a different event variant");
        };

        self.interface.open_window(BuyOrSellWindow::new(shop_id));
    }

    /// `NetworkEvent::BuyingCompleted`.
    pub(crate) fn handle_buying_completed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::BuyingCompleted { result } = event else {
            unreachable!("handle_buying_completed called with a different event variant");
        };

        match result {
            BuyItemResult::Successful => {
                let _ = self.networking_system.close_shop();
                self.shop_open = false;

                // Clear the cart.
                self.client_state.follow_mut(client_state().buy_cart()).clear();

                self.interface.close_window_with_class(WindowClass::Buy);
                self.interface.close_window_with_class(WindowClass::BuyCart);
            }
            result => {
                let message = format_message(MessageDomain::BuyItem, buy_item_result_code(&result), &[]);

                self.client_state
                    .follow_mut(client_state().chat_messages())
                    .push(ChatMessage::new(message, MessageColor::Error));
            }
        }
    }

    /// `NetworkEvent::SellingCompleted`.
    pub(crate) fn handle_selling_completed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SellingCompleted { result } = event else {
            unreachable!("handle_selling_completed called with a different event variant");
        };

        match result {
            SellItemsResult::Success => {
                // Clear the cart.
                self.client_state.follow_mut(client_state().buy_cart()).clear();

                self.interface.close_window_with_class(WindowClass::Sell);
                self.interface.close_window_with_class(WindowClass::SellCart);
            }
            SellItemsResult::Error => {
                self.push_localized_chat(client_state().localization().chat_sell_failed(), &[]);
            }
        }
    }

    /// `NetworkEvent::TradeAddResult`.
    pub(crate) fn handle_trade_add_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeAddResult { inventory_index, result } = event else {
            unreachable!("handle_trade_add_result called with a different event variant");
        };

        if result != 0 {
            crate::logging::log_debug!(
                "trade: adding inventory index {} failed with result {}",
                inventory_index,
                result
            );
        }
    }

    /// `NetworkEvent::TradeRequest`.
    pub(crate) fn handle_trade_request(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeRequest {
            requester_name,
            target_account_id,
            ..
        } = event
        else {
            unreachable!("handle_trade_request called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().trade())
            .set_request(requester_name.clone(), target_account_id.0);
        self.interface
            .open_window(TradeRequestWindow::new(requester_name, target_account_id.0));
    }

    /// `NetworkEvent::TradeAck`.
    pub(crate) fn handle_trade_ack(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeAck { result, .. } = event else {
            unreachable!("handle_trade_ack called with a different event variant");
        };

        if result == 3 {
            let partner_name = self
                .client_state
                .follow(client_state().trade())
                .request()
                .map(|request| request.name.clone())
                .unwrap_or_default();
            self.client_state.follow_mut(client_state().trade()).open(partner_name);
            self.interface.close_window_with_class(WindowClass::TradeRequest);
            self.interface
                .open_window(TradeWindow::new(client_state().trade(), client_state().inventory().items()));
        } else {
            self.client_state.follow_mut(client_state().trade()).clear_request();
            self.interface.close_window_with_class(WindowClass::TradeRequest);
        }
    }

    /// `NetworkEvent::TradeCancelled`.
    pub(crate) fn handle_trade_cancelled(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeCancelled = event else {
            unreachable!("handle_trade_cancelled called with a different event variant");
        };

        self.close_trade();
    }

    /// `NetworkEvent::TradeConcluded`.
    pub(crate) fn handle_trade_concluded(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeConcluded { who } = event else {
            unreachable!("handle_trade_concluded called with a different event variant");
        };

        let trade = self.client_state.follow_mut(client_state().trade());
        if who == 0 {
            trade.set_locked(true);
        } else {
            trade.set_partner_locked(true);
        }
    }

    /// `NetworkEvent::TradeExecuted`.
    pub(crate) fn handle_trade_executed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeExecuted { result } = event else {
            unreachable!("handle_trade_executed called with a different event variant");
        };

        crate::logging::log_debug!("trade executed: result {}", result);
        self.close_trade();
    }

    /// `NetworkEvent::OpenStore`.
    pub(crate) fn handle_open_store(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::OpenStore { slots } = event else {
            unreachable!("handle_open_store called with a different event variant");
        };

        // `ZC_OPENSTORE` after MC_VENDING: show the shop editor.
        // The server keeps the character in the vending state until
        // it receives CZ_REQ_OPENSTORE/2, so the editor is modal
        // (no X close) and always offers Open/Cancel.
        self.client_state.follow_mut(client_state().vending()).open(slots);
        self.interface.open_window(crate::interface::windows::VendingWindow::new(
            client_state().vending(),
            client_state().inventory().items(),
        ));
    }
}
