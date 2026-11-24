//! `InputEvent` handlers for the inventory family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ToggleInventoryWindow`.
    pub(crate) fn handle_input_toggle_inventory_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleInventoryWindow = event else {
            unreachable!("handle_input_toggle_inventory_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Inventory) {
                true => self.interface.close_window_with_class(WindowClass::Inventory),
                false => self.interface.open_window(InventoryWindow::new(
                    client_state().inventory().items(),
                    client_state().interface_settings(),
                )),
            }
        }
    }

    /// `InputEvent::ToggleEquipmentWindow`.
    pub(crate) fn handle_input_toggle_equipment_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleEquipmentWindow = event else {
            unreachable!("handle_input_toggle_equipment_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Equipment) {
                true => self.interface.close_window_with_class(WindowClass::Equipment),
                false => self.interface.open_window(EquipmentWindow::new(
                    client_state().inventory().items(),
                    this_player().manually_asserted(),
                )),
            }
        }
    }

    /// `InputEvent::OpenCartWindow`.
    pub(crate) fn handle_input_open_cart_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenCartWindow = event else {
            unreachable!("handle_input_open_cart_window called with a different event variant");
        };

        if !self.interface.is_window_with_class_open(WindowClass::Cart) {
            self.interface.open_window(CartWindow);
        }
    }

    /// `InputEvent::OpenInventoryItemMenu`.
    pub(crate) fn handle_input_open_inventory_item_menu(
        &mut self,
        event: InputEvent,
        input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::OpenInventoryItemMenu { inventory_index } = event else {
            unreachable!("handle_input_open_inventory_item_menu called with a different event variant");
        };

        open_inventory_item_menu(
            &self.client_state,
            &mut self.interface,
            inventory_index,
            input_report.mouse_position,
        );
    }

    /// `InputEvent::OpenStorageItemMenu`.
    pub(crate) fn handle_input_open_storage_item_menu(&mut self, event: InputEvent, input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenStorageItemMenu { storage_index } = event else {
            unreachable!("handle_input_open_storage_item_menu called with a different event variant");
        };

        open_storage_item_menu(
            &self.client_state,
            &mut self.interface,
            storage_index,
            input_report.mouse_position,
        );
    }

    /// `InputEvent::DepositInventoryItem`.
    pub(crate) fn handle_input_deposit_inventory_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::DepositInventoryItem { inventory_index, amount } = event else {
            unreachable!("handle_input_deposit_inventory_item called with a different event variant");
        };

        let _ = self.networking_system.storage_deposit(inventory_index, amount);
    }

    /// `InputEvent::WithdrawStorageItem`.
    pub(crate) fn handle_input_withdraw_storage_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::WithdrawStorageItem { storage_index, amount } = event else {
            unreachable!("handle_input_withdraw_storage_item called with a different event variant");
        };

        // The server expects the storage slot encoded as `slot + 1`.
        let _ = self.networking_system.storage_withdraw(storage_index.0, amount);
    }

    /// `InputEvent::CloseStorage`.
    pub(crate) fn handle_input_close_storage(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseStorage = event else {
            unreachable!("handle_input_close_storage called with a different event variant");
        };

        let _ = self.networking_system.close_storage();
        self.close_storage_window();
    }

    /// `InputEvent::BankDeposit`.
    pub(crate) fn handle_input_bank_deposit(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BankDeposit = event else {
            unreachable!("handle_input_bank_deposit called with a different event variant");
        };

        let amount = parse_zeny_amount(self.client_state.follow(client_state().bank()).deposit_input());

        match amount {
            Some(amount) => {
                if let Some(account_id) = self.own_account_id() {
                    let _ = self.networking_system.bank_deposit(account_id, amount);
                    self.client_state.follow_mut(client_state().bank()).set_status("Depositing...");
                }
            }
            None => self
                .client_state
                .follow_mut(client_state().bank())
                .set_status("Enter a valid zeny amount first."),
        }
    }

    /// `InputEvent::BankWithdraw`.
    pub(crate) fn handle_input_bank_withdraw(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BankWithdraw = event else {
            unreachable!("handle_input_bank_withdraw called with a different event variant");
        };

        let amount = parse_zeny_amount(self.client_state.follow(client_state().bank()).withdraw_input());

        match amount {
            Some(amount) => {
                if let Some(account_id) = self.own_account_id() {
                    let _ = self.networking_system.bank_withdraw(account_id, amount);
                    self.client_state.follow_mut(client_state().bank()).set_status("Withdrawing...");
                }
            }
            None => self
                .client_state
                .follow_mut(client_state().bank())
                .set_status("Enter a valid zeny amount first."),
        }
    }

    /// `InputEvent::ItemDoubleClick`.
    pub(crate) fn handle_input_item_double_click(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ItemDoubleClick { source, item } = event else {
            unreachable!("handle_input_item_double_click called with a different event variant");
        };

        // Double clicking is the quick way to move a whole stack while
        // the Kafra storage window is open.
        if self.interface.is_window_with_class_open(WindowClass::Storage) {
            match source {
                ItemSource::Inventory => {
                    let _ = self.networking_system.storage_deposit(item.index, item_amount(&item).max(1));
                }
                ItemSource::Storage => {
                    let _ = self.networking_system.storage_withdraw(item.index.0, item_amount(&item).max(1));
                }
                ItemSource::Cart => {
                    let _ = self.networking_system.move_item_from_cart(item.index, item_amount(&item).max(1));
                }
                ItemSource::Equipment { .. } => {}
            }
            return;
        }

        // While the cart window is open, double clicking moves a stack
        // between the inventory and the pushcart.
        if self.interface.is_window_with_class_open(WindowClass::Cart) {
            match source {
                ItemSource::Inventory => {
                    let _ = self.networking_system.move_item_to_cart(item.index, item_amount(&item).max(1));
                }
                ItemSource::Cart => {
                    let _ = self.networking_system.move_item_from_cart(item.index, item_amount(&item).max(1));
                }
                _ => {}
            }
            return;
        }

        // Otherwise double clicking an inventory item uses consumables and
        // tries to equip equipment; the server validates requirements.
        if let ItemSource::Inventory = source {
            match item_use_action(&item) {
                Some(ItemUseAction::Use) if item.is_identified() => {
                    let _ = self.networking_system.use_item(item.index, item.item_id);
                }
                Some(ItemUseAction::Equip(equip_position)) => {
                    let _ = self.networking_system.request_item_equip(item.index, equip_position);
                }
                _ => {}
            }
        }
    }

    /// `InputEvent::DropItem`.
    pub(crate) fn handle_input_drop_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::DropItem { inventory_index, amount } = event else {
            unreachable!("handle_input_drop_item called with a different event variant");
        };

        if let Err(error) = self.networking_system.drop_item(inventory_index, amount) {
            crate::logging::log_warn!("failed to send item drop: {error:?}");
        }
    }

    /// `InputEvent::OpenItemInfo`.
    pub(crate) fn handle_input_open_item_info(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenItemInfo { inventory_index, source } = event else {
            unreachable!("handle_input_open_item_info called with a different event variant");
        };

        open_item_info_window(
            &self.client_state,
            &mut self.interface,
            inventory_index,
            source,
            &self.library,
            &self.texture_loader,
        );
    }

    /// `InputEvent::CloseItemInfo`.
    pub(crate) fn handle_input_close_item_info(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseItemInfo = event else {
            unreachable!("handle_input_close_item_info called with a different event variant");
        };

        self.interface.close_window_with_class(WindowClass::ItemInfo);
    }

    /// `InputEvent::UseInventoryItem`.
    pub(crate) fn handle_input_use_inventory_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::UseInventoryItem { inventory_index, item_id } = event else {
            unreachable!("handle_input_use_inventory_item called with a different event variant");
        };

        let _ = self.networking_system.use_item(inventory_index, item_id);
    }

    /// `InputEvent::RefineWeapon`.
    pub(crate) fn handle_input_refine_weapon(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RefineWeapon { inventory_index } = event else {
            unreachable!("handle_input_refine_weapon called with a different event variant");
        };

        // rAthena drops the request unless the `WS_WEAPONREFINE`
        // menu skill is pending (`clif_parse_WeaponRefine`,
        // `clif.cpp:13255`), i.e. unless the server sent the refine
        // list. Explain that instead of sending a request that is
        // silently ignored.
        if *self.client_state.follow(client_state().item_service().kind()) != Some(ItemServiceKind::WeaponRefine) {
            self.push_chat_warning("Use the Blacksmith Weapon Refine skill to start refining a weapon.".to_owned());
        } else if let Err(error) = self.networking_system.refine_weapon(inventory_index) {
            crate::logging::log_warn!("item service: could not send refine request: {error:?}");
        }
    }

    /// `InputEvent::RepairItem`.
    pub(crate) fn handle_input_repair_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RepairItem { inventory_index } = event else {
            unreachable!("handle_input_repair_item called with a different event variant");
        };

        // The repair request is only valid while the
        // `BS_REPAIRWEAPON` menu skill is pending (`clif.cpp:13231`).
        if *self.client_state.follow(client_state().item_service().kind()) != Some(ItemServiceKind::ItemRepair) {
            self.push_chat_warning("Use the Repair Weapon skill to repair damaged items.".to_owned());
            return;
        }

        // The item id/refine come from the repair list: when the
        // player repairs another player's equipment the slot refers
        // to *that* player's inventory, so a lookup in the local
        // inventory would fail (or find an unrelated item). rAthena
        // only reads the index from the request (`clif.cpp:13242`),
        // so the other fields are informational and the card slots
        // can stay zero.
        let (item_id, refine) = {
            let service = self.client_state.follow(client_state().item_service());
            let from_list = service
                .entries
                .iter()
                .find(|entry| entry.index == inventory_index)
                .map(|entry| (entry.item_id, entry.refine));

            from_list.unwrap_or_else(|| {
                let inventory = self.client_state.follow(client_state().inventory().items());
                let from_inventory = inventory.iter().find(|item| item.index == inventory_index).map(|item| {
                    let refine = match &item.details {
                        ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
                        ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
                    };
                    (item.item_id, refine)
                });

                from_inventory.unwrap_or((ItemId(0), 0))
            })
        };

        if let Err(error) = self.networking_system.repair_item(inventory_index, item_id, refine, [0; 4]) {
            crate::logging::log_warn!("item service: could not send repair request: {error:?}");
        }
    }

    /// `InputEvent::IdentifyItem`.
    pub(crate) fn handle_input_identify_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::IdentifyItem { inventory_index } = event else {
            unreachable!("handle_input_identify_item called with a different event variant");
        };

        // Answer to the identify list that a Magnifier produced.
        if let Err(error) = self.networking_system.identify_item(inventory_index) {
            crate::logging::log_warn!("item service: could not send identify request: {error:?}");
        }
    }

    /// `InputEvent::OneClickIdentify`.
    pub(crate) fn handle_input_one_click_identify(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OneClickIdentify { inventory_index } = event else {
            unreachable!("handle_input_one_click_identify called with a different event variant");
        };

        // The one-click identify consumes a Magnifier (or Novice
        // Magnifier); rAthena silently ignores the request when the
        // player has none (`clif.cpp:21397`), so check first and tell
        // the player instead of doing nothing.
        const MAGNIFIER_ITEM_ID: ItemId = ItemId(611);
        const NOVICE_MAGNIFIER_ITEM_ID: ItemId = ItemId(12_325);

        let has_magnifier = self
            .client_state
            .follow(client_state().inventory().items())
            .iter()
            .any(|item| item.item_id == MAGNIFIER_ITEM_ID || item.item_id == NOVICE_MAGNIFIER_ITEM_ID);

        match has_magnifier {
            true => {
                if let Err(error) = self.networking_system.one_click_identify(inventory_index) {
                    crate::logging::log_warn!("item service: could not send one-click identify: {error:?}");
                }
            }
            false => self.push_chat_warning("You need a Magnifier to identify items.".to_owned()),
        }
    }

    /// `InputEvent::RequestCardComposition`.
    pub(crate) fn handle_input_request_card_composition(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::RequestCardComposition { card_index } = event else {
            unreachable!("handle_input_request_card_composition called with a different event variant");
        };

        // Remember the card: the answer list only contains the
        // equipment indexes, so the insert request later needs the
        // card data from here.
        if let Some(card) = self.inventory_service_entry(card_index) {
            *self.client_state.follow_mut(client_state().item_service().card()) = Some(card);
        }

        if let Err(error) = self.networking_system.request_card_composition_list(card_index) {
            crate::logging::log_warn!("item service: could not request card composition list: {error:?}");
        }
    }

    /// `InputEvent::InsertCard`.
    pub(crate) fn handle_input_insert_card(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::InsertCard { card_index, equip_index } = event else {
            unreachable!("handle_input_insert_card called with a different event variant");
        };

        if let Err(error) = self.networking_system.insert_card(card_index, equip_index) {
            crate::logging::log_warn!("item service: could not send card insert: {error:?}");
        }
    }

    /// `InputEvent::CloseItemService`.
    pub(crate) fn handle_input_close_item_service(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseItemService = event else {
            unreachable!("handle_input_close_item_service called with a different event variant");
        };

        self.close_item_service_window()
    }

    /// `InputEvent::SelectRefiningItem`.
    pub(crate) fn handle_input_select_refining_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SelectRefiningItem { inventory_index } = event else {
            unreachable!("handle_input_select_refining_item called with a different event variant");
        };

        // The server answers with `ZC_REFINING_MATERIAL_LIST`, which
        // opens the material step of the window.
        if let Err(error) = self.networking_system.select_refining_item(inventory_index) {
            crate::logging::log_warn!("item service: could not select refining item: {error:?}");
        }
    }

    /// `InputEvent::RefineWithMaterial`.
    pub(crate) fn handle_input_refine_with_material(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RefineWithMaterial {
            material_item_id,
            blacksmith_blessing,
        } = event
        else {
            unreachable!("handle_input_refine_with_material called with a different event variant");
        };

        let item_index = self
            .client_state
            .follow(client_state().item_service().refining().item())
            .as_ref()
            .map(|item| item.index);

        if let Some(item_index) = item_index
            && let Err(error) = self
                .networking_system
                .request_refining(item_index, material_item_id, blacksmith_blessing)
        {
            crate::logging::log_warn!("item service: could not send refine attempt: {error:?}");
        }
    }

    /// `InputEvent::EquipInventoryItem`.
    pub(crate) fn handle_input_equip_inventory_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::EquipInventoryItem {
            inventory_index,
            equip_position,
        } = event
        else {
            unreachable!("handle_input_equip_inventory_item called with a different event variant");
        };

        let _ = self.networking_system.request_item_equip(inventory_index, equip_position);
    }

    /// `InputEvent::PickUpItem`.
    pub(crate) fn handle_input_pick_up_item(&mut self, event: InputEvent, _input_report: &InputReport, client_tick: ClientTick) {
        let InputEvent::PickUpItem { entity_id } = event else {
            unreachable!("handle_input_pick_up_item called with a different event variant");
        };

        self.mouse_cursor.set_state(MouseCursorState::PickUpItem, client_tick);

        if let Some(map) = &self.map {
            let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
            let item_position = self
                .client_state
                .follow(client_state().ground_items())
                .iter()
                .find(|item| item.get_entity_id() == entity_id)
                .map(|item| item.get_tile_position());

            if let (Some(player_position), Some(item_position)) = (player_position, item_position) {
                if player_position
                    .x
                    .abs_diff(item_position.x)
                    .max(player_position.y.abs_diff(item_position.y))
                    <= ITEM_PICKUP_RANGE.0
                {
                    let _ = self.networking_system.pick_up_item(entity_id);

                    *self.client_state.follow_mut(client_state().buffered_action()) = None;
                } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                    &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                    player_position,
                    item_position,
                    ITEM_PICKUP_RANGE,
                ) && let Some(nearest_tile) = path.last()
                {
                    let _ = self.networking_system.player_move(WorldPosition {
                        x: nearest_tile.x,
                        y: nearest_tile.y,
                        direction: Direction::North,
                    });

                    *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::PickUpItem { entity_id });
                } else {
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;
                }
            }
        }
    }

    /// `InputEvent::MoveItem`.
    pub(crate) fn handle_input_move_item(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::MoveItem { source, destination, item } = event else {
            unreachable!("handle_input_move_item called with a different event variant");
        };

        match (source, destination) {
            (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                let _ = self.networking_system.request_item_equip(item.index, position);
            }
            (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                let _ = self.networking_system.request_item_unequip(item.index);
            }
            (ItemSource::Inventory, ItemSource::Storage) => {
                let amount = item_amount(&item).max(1);
                let _ = self.networking_system.storage_deposit(item.index, amount);
            }
            (ItemSource::Storage, ItemSource::Inventory) => {
                let amount = item_amount(&item).max(1);
                let _ = self.networking_system.storage_withdraw(item.index.0, amount);
            }
            (ItemSource::Inventory, ItemSource::Cart) => {
                let amount = item_amount(&item).max(1);
                let _ = self.networking_system.move_item_to_cart(item.index, amount);
            }
            (ItemSource::Cart, ItemSource::Inventory) => {
                let amount = item_amount(&item).max(1);
                let _ = self.networking_system.move_item_from_cart(item.index, amount);
            }
            (ItemSource::Storage, ItemSource::Cart) => {
                let amount = item_amount(&item).max(1);
                let _ = self.networking_system.move_storage_item_to_cart(item.index.0, amount);
            }
            (ItemSource::Cart, ItemSource::Storage) => {
                let amount = item_amount(&item).max(1);
                let _ = self.networking_system.move_cart_item_to_storage(item.index, amount);
            }
            _ => {}
        }
    }

    /// `InputEvent::BuyItems`.
    pub(crate) fn handle_input_buy_items(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BuyItems { items } = event else {
            unreachable!("handle_input_buy_items called with a different event variant");
        };

        let _ = self.networking_system.purchase_items(items);
    }

    /// `InputEvent::CloseShop`.
    pub(crate) fn handle_input_close_shop(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseShop = event else {
            unreachable!("handle_input_close_shop called with a different event variant");
        };

        let _ = self.networking_system.close_shop();
        self.shop_open = false;

        // Clear the carts.
        self.client_state.follow_mut(client_state().buy_cart()).clear();
        self.client_state.follow_mut(client_state().sell_cart()).clear();

        self.interface.close_window_with_class(WindowClass::Buy);
        self.interface.close_window_with_class(WindowClass::BuyCart);
        self.interface.close_window_with_class(WindowClass::Sell);
        self.interface.close_window_with_class(WindowClass::SellCart);
    }

    /// `InputEvent::BuyOrSell`.
    pub(crate) fn handle_input_buy_or_sell(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BuyOrSell { shop_id, buy_or_sell } = event else {
            unreachable!("handle_input_buy_or_sell called with a different event variant");
        };

        let _ = self.networking_system.select_buy_or_sell(shop_id, buy_or_sell);
        self.interface.close_window_with_class(WindowClass::BuyOrSell);
    }

    /// `InputEvent::SellItems`.
    pub(crate) fn handle_input_sell_items(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SellItems { items } = event else {
            unreachable!("handle_input_sell_items called with a different event variant");
        };

        let _ = self.networking_system.sell_items(items);
    }
}
