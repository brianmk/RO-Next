//! `NetworkEvent` handlers for the inventory family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `mod.rs` only routes.

use crate::*;

impl Client {
    /// `NetworkEvent::AddGroundItem`.
    pub(crate) fn handle_add_ground_item(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::AddGroundItem {
            entity_id,
            item_id,
            is_identified,
            quantity,
            position,
            x_offset,
            y_offset,
        } = event
        else {
            unreachable!("handle_add_ground_item called with a different event variant");
        };

        if let Some(map) = self.map.as_ref()
            && let Some(mut ground_item) = GroundItem::new(
                map,
                item_id,
                entity_id,
                is_identified,
                quantity,
                position,
                x_offset,
                y_offset,
                client_tick,
            )
        {
            let ground_items = self.client_state.follow_mut(client_state().ground_items());
            let entity_part_files = ground_item.get_entity_part_files(&self.library);

            if let Some(animation_data) = self
                .async_loader
                // TODO: Technically Npc is not correct here. We could add an item
                // variant or refactor this fuction to take an optional entity
                // type.
                .request_animation_data_load(entity_id, EntityType::Npc, entity_part_files)
            {
                ground_item.set_animation_data(animation_data);
            }

            ground_items.push(ground_item);
        } else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to spawn item", "error".red());
        }
    }

    /// `NetworkEvent::RemoveGroundItem`.
    pub(crate) fn handle_remove_ground_item(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::RemoveGroundItem { entity_id } = event else {
            unreachable!("handle_remove_ground_item called with a different event variant");
        };

        if let Some(item) = self
            .client_state
            .follow_mut(client_state().ground_items())
            .iter_mut()
            .find(|item| item.get_entity_id() == entity_id)
        {
            item.fade_out(client_tick);
        }

        let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
        if buffered_action.is_some_and(|buffered_action| buffered_action.is_pick_up_item(entity_id)) {
            *buffered_action = None;
        }
    }

    /// `NetworkEvent::ItemPickedUp`.
    pub(crate) fn handle_item_picked_up(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::ItemPickedUp { item_id, amount } = event else {
            unreachable!("handle_item_picked_up called with a different event variant");
        };

        let name = self.library.composed_item_name(item_id, true, 0, &[0; 4]);
        let name = match name.trim().is_empty() {
            true => format!("Item #{}", item_id.0),
            false => name,
        };
        let text = match amount > 1 {
            true => format!("Picked up {name} x{amount}"),
            false => format!("Picked up {name}"),
        };

        self.push_notification(NotificationKind::Pickup, text, client_tick);
    }

    /// `NetworkEvent::EntityPickUpItem`.
    pub(crate) fn handle_entity_pick_up_item(&mut self, event: NetworkEvent, client_tick: ClientTick) {
        let NetworkEvent::EntityPickUpItem { entity_id, item_entity_id } = event else {
            unreachable!("handle_entity_pick_up_item called with a different event variant");
        };

        let item_position = self
            .client_state
            .follow(client_state().ground_items())
            .iter()
            .find(|item| item.get_entity_id() == item_entity_id)
            .map(|item| item.get_tile_position());

        if let Some(entity) = self
            .client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
        {
            if let Some(item_position) = item_position {
                entity.rotate_towards(item_position);
            }

            if matches!(
                entity.get_entity_type(),
                EntityType::Player | EntityType::Hidden | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary
            ) {
                entity.set_pickup(client_tick);
            }
        }
    }

    /// `NetworkEvent::SetInventory`.
    pub(crate) fn handle_set_inventory(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SetInventory { items } = event else {
            unreachable!("handle_set_inventory called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().inventory())
            .fill(&self.async_loader, items);
    }

    /// `NetworkEvent::WeaponRefineList`.
    pub(crate) fn handle_weapon_refine_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::WeaponRefineList { entries } = event else {
            unreachable!("handle_weapon_refine_list called with a different event variant");
        };

        let entries = entries
            .into_iter()
            .map(|entry| {
                let name = self.library.item_name(entry.item_id, true).into_owned();
                ItemServiceEntry::new(entry.index, entry.item_id, name, entry.refine)
            })
            .collect();

        self.client_state
            .follow_mut(client_state().item_service())
            .show(ItemServiceKind::WeaponRefine, entries);
        self.open_item_service_window(ItemServiceKind::WeaponRefine);
    }

    /// `NetworkEvent::WeaponRefineResult`.
    pub(crate) fn handle_weapon_refine_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::WeaponRefineResult { result, item_id } = event else {
            unreachable!("handle_weapon_refine_result called with a different event variant");
        };

        let name = self.library.item_name(item_id, true).into_owned();
        let message = match result {
            0 => format!("Your {name} was successfully refined."),
            1 => format!("The refine of your {name} failed and it lost its refine level."),
            2 => format!("Your {name} cannot be refined further until you raise the Weapon Refine skill level."),
            3 => format!("You lack the material required to refine your {name}."),
            _ => format!("The refine attempt for your {name} failed."),
        };

        self.client_state
            .follow_mut(client_state().item_service())
            .set_status(message.clone());

        match result {
            0 => self.push_chat_information(message),
            _ => self.push_chat_warning(message),
        }
    }

    /// `NetworkEvent::ItemRefiningResult`.
    pub(crate) fn handle_item_refining_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ItemRefiningResult { index, refine, result } = event else {
            unreachable!("handle_item_refining_result called with a different event variant");
        };

        // rAthena also re-sends the inventory list after a refine
        // (`skill.cpp:20753`); this keeps the refine level and the
        // composed item name current even before the list arrives.
        let refinement_level = u8::try_from(refine).unwrap_or(u8::MAX);
        self.client_state
            .follow_mut(client_state().inventory())
            .update_refinement(&self.async_loader, index, refinement_level);

        if *self.client_state.follow(client_state().item_service().kind()) == Some(ItemServiceKind::WeaponRefine)
            && let Some(entry) = self
                .client_state
                .follow_mut(client_state().item_service())
                .entries
                .iter_mut()
                .find(|entry| entry.index == index)
        {
            entry.refine = refinement_level;
        }

        // `ZC_ACK_ITEMREFINING` is the only result packet of the NPC
        // refine UI (`clif.cpp:22720`-`22750`); show it there.
        if *self.client_state.follow(client_state().item_service().refining().open()) {
            let message = match result {
                0 => "The refine succeeded.".to_owned(),
                1 => "The refine failed and the item broke.".to_owned(),
                2 => "The refine failed and the item lost refine levels.".to_owned(),
                _ => "The refine failed.".to_owned(),
            };

            // A break destroys the item (`pc_delitem`), so go back to
            // the item picker; every other result is followed by a
            // fresh material list (0x0AA2) that replaces the current
            // view. `start` resets the status line, hence the order.
            let candidates = match result {
                1 => Some(self.refinable_inventory_items()),
                _ => None,
            };

            {
                let refining = self.client_state.follow_mut(client_state().item_service().refining());

                if let Some(candidates) = candidates {
                    refining.start(candidates);
                }

                refining.set_status(message.clone());
            }

            match result {
                0 => self.push_chat_information(message),
                _ => self.push_chat_warning(message),
            }

            if result == 1 {
                self.open_refining_window(RefiningMode::SelectItem);
            }
        }

        crate::logging::log_debug!("item service: refine result {} for item {}", result, index.0);
    }

    /// `NetworkEvent::RepairItemList`.
    pub(crate) fn handle_repair_item_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RepairItemList { entries } = event else {
            unreachable!("handle_repair_item_list called with a different event variant");
        };

        let entries = entries
            .into_iter()
            .map(|entry| {
                let name = self.library.item_name(entry.item_id, true).into_owned();
                // The repair list sends raw 0-based server slots, not the
                // `slot + 2` client index (`clif.cpp:7153`).
                ItemServiceEntry::new(InventoryIndex(entry.index), entry.item_id, name, entry.refine)
            })
            .collect();

        self.client_state
            .follow_mut(client_state().item_service())
            .show(ItemServiceKind::ItemRepair, entries);
        self.open_item_service_window(ItemServiceKind::ItemRepair);
    }

    /// `NetworkEvent::RepairItemResult`.
    pub(crate) fn handle_repair_item_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RepairItemResult { index, success } = event else {
            unreachable!("handle_repair_item_result called with a different event variant");
        };

        self.client_state.follow_mut(client_state().inventory()).set_broken(index, !success);

        let message = match success {
            true => "The item was repaired.".to_owned(),
            false => "The item could not be repaired.".to_owned(),
        };

        self.client_state
            .follow_mut(client_state().item_service())
            .set_status(message.clone());

        match success {
            true => self.push_chat_information(message),
            false => self.push_chat_warning(message),
        }
    }

    /// `NetworkEvent::IdentifyItemList`.
    pub(crate) fn handle_identify_item_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::IdentifyItemList { indexes } = event else {
            unreachable!("handle_identify_item_list called with a different event variant");
        };

        let entries = indexes
            .into_iter()
            .filter_map(|index| self.inventory_service_entry(index))
            .collect();

        self.client_state
            .follow_mut(client_state().item_service())
            .show(ItemServiceKind::ItemIdentify, entries);
        self.open_item_service_window(ItemServiceKind::ItemIdentify);
    }

    /// `NetworkEvent::IdentifyItemResult`.
    pub(crate) fn handle_identify_item_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::IdentifyItemResult { index, success } = event else {
            unreachable!("handle_identify_item_result called with a different event variant");
        };

        // rAthena only sends the ack; the item itself is not part of
        // the packet, so the client flips the identified flag and
        // re-resolves the name/description (both depend on it).
        self.client_state
            .follow_mut(client_state().inventory())
            .set_identified(&self.async_loader, index, success);

        let message = match success {
            true => "The item was identified.".to_owned(),
            false => "The item could not be identified.".to_owned(),
        };

        self.client_state
            .follow_mut(client_state().item_service())
            .set_status(message.clone());

        match success {
            true => self.push_chat_information(message),
            false => self.push_chat_warning(message),
        }
    }

    /// `NetworkEvent::CardCompositionList`.
    pub(crate) fn handle_card_composition_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CardCompositionList { indexes } = event else {
            unreachable!("handle_card_composition_list called with a different event variant");
        };

        let entries: Vec<ItemServiceEntry> = indexes
            .into_iter()
            .filter_map(|index| self.inventory_service_entry(index))
            .collect();

        let card = self.client_state.follow(client_state().item_service().card()).clone();

        match card {
            Some(card) => self
                .client_state
                .follow_mut(client_state().item_service())
                .show_card_list(card, entries),
            None => self
                .client_state
                .follow_mut(client_state().item_service())
                .show(ItemServiceKind::CardComposition, entries),
        }

        self.open_item_service_window(ItemServiceKind::CardComposition);
    }

    /// `NetworkEvent::CardCompositionResult`.
    pub(crate) fn handle_card_composition_result(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CardCompositionResult {
            equip_index,
            card_index,
            success,
        } = event
        else {
            unreachable!("handle_card_composition_result called with a different event variant");
        };

        if success {
            // `pc_insert_card` does not re-send the equipment; it only
            // deletes the card from the inventory (`pc.cpp:6238`). The
            // card id is taken from the selected card (falling back to
            // the inventory if the delete packet already arrived).
            let card_item_id = self
                .client_state
                .follow(client_state().item_service().card())
                .as_ref()
                .filter(|card| card.index == card_index)
                .map(|card| card.item_id)
                .or_else(|| {
                    self.client_state
                        .follow(client_state().inventory().items())
                        .iter()
                        .find(|item| item.index == card_index)
                        .map(|item| item.item_id)
                });

            if let Some(card_item_id) = card_item_id {
                self.client_state
                    .follow_mut(client_state().inventory())
                    .insert_card(&self.async_loader, equip_index, card_item_id);
            }

            // The card is removed from the inventory by the
            // server's own `pc_delitem`
            // delete packet, which rAthena sends *before*
            // the ack (`pc.cpp:6238` then `6247`). Removing it here
            // as well would delete one card
            // too many when the player owns
            // a stack of the same card.
        }

        let message = match success {
            true => "The card was inserted.".to_owned(),
            false => "The card could not be inserted.".to_owned(),
        };

        self.client_state
            .follow_mut(client_state().item_service())
            .set_status(message.clone());

        match success {
            true => self.push_chat_information(message),
            false => self.push_chat_warning(message),
        }
    }

    /// `NetworkEvent::CardSlotUpdated`.
    pub(crate) fn handle_card_slot_updated(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CardSlotUpdated {
            wear_state,
            card_slot,
            item_id,
            equip_flag,
        } = event
        else {
            unreachable!("handle_card_slot_updated called with a different event variant");
        };

        // rAthena 20190605 defines ZC_UPDATE_CARDSLOT but never sends
        // it (`grep -n UPDATE_CARDSLOT src/map/clif.cpp` has no
        // match); the normal card insert path updates the slot from
        // the ack above. Apply it defensively for servers that do
        // send it: `wear_state` is the equipment position, `card_slot`
        // the slot index and `item_id` the inserted card.
        crate::logging::log_debug!(
            "item service: ZC_UPDATE_CARDSLOT wear={} slot={} item={} flag={}",
            wear_state,
            card_slot,
            item_id.0,
            equip_flag
        );

        let equipment_position = EquipPosition::from_bits_truncate(u32::from(wear_state as u16));
        if (0..4).contains(&card_slot) && !equipment_position.is_empty() {
            let index = self
                .client_state
                .follow(client_state().inventory().items())
                .iter()
                .find(|item| {
                    matches!(
                        &item.details,
                        ro_next_networking::InventoryItemDetails::Equippable { equipped_position, .. }
                            if equipped_position.intersects(equipment_position)
                    )
                })
                .map(|item| item.index);

            if let Some(index) = index {
                self.client_state.follow_mut(client_state().inventory()).set_card_slot(
                    &self.async_loader,
                    index,
                    card_slot as usize,
                    item_id,
                );
            }
        }
    }

    /// `NetworkEvent::RefiningUiOpened`.
    pub(crate) fn handle_refining_ui_opened(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RefiningUiOpened = event else {
            unreachable!("handle_refining_ui_opened called with a different event variant");
        };

        // The server opened the NPC refine UI; it does not send a
        // candidate list, so offer the player's own refinable
        // equipment and ask for the materials after the pick.
        let candidates = self.refinable_inventory_items();
        self.client_state
            .follow_mut(client_state().item_service().refining())
            .start(candidates);
        self.open_refining_window(RefiningMode::SelectItem);
    }

    /// `NetworkEvent::RefiningMaterialList`.
    pub(crate) fn handle_refining_material_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::RefiningMaterialList {
            item_index,
            blacksmith_blessing,
            materials,
        } = event
        else {
            unreachable!("handle_refining_material_list called with a different event variant");
        };

        let item = self
            .inventory_service_entry(item_index)
            // Another player's item is not in our inventory; the
            // server list still carries its item id? It does not, so
            // fall back to the index for the display and let the
            // server validate the request.
            .unwrap_or_else(|| ItemServiceEntry::new(item_index, ItemId(0), format!("Item #{}", item_index.0), 0));

        let materials = materials
            .into_iter()
            .map(|material| {
                let name = self.library.item_name(material.item_id, true).into_owned();
                RefiningMaterial {
                    item_id: material.item_id,
                    name,
                    chance: u8::try_from(material.chance).unwrap_or(0),
                    zeny: material.zeny,
                }
            })
            .collect();

        self.client_state
            .follow_mut(client_state().item_service().refining())
            .show_materials(item, u8::try_from(blacksmith_blessing).unwrap_or(0), materials);
        self.open_refining_window(RefiningMode::SelectMaterial);
    }

    /// `NetworkEvent::EquipmentSwitchItems`.
    pub(crate) fn handle_equipment_switch_items(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::EquipmentSwitchItems { items } = event else {
            unreachable!("handle_equipment_switch_items called with a different event variant");
        };

        // `ZC_EQUIPSWITCH_LIST` pre-assigns inventory items to
        // equipment positions (the equip-switch window). Recording the
        // position in the inventory state makes those items visible on
        // the paper doll and lets the normal equip/unequip flow move
        // them, instead of dropping the packet.
        let inventory = self.client_state.follow_mut(client_state().inventory());

        for item in &items {
            let position = EquipPosition::from_bits_truncate(item.position);
            inventory.update_equipped_position(item.index, position);
        }

        crate::logging::log_debug!("equip switch list: {} pre-assigned item(s)", items.len());
    }

    /// `NetworkEvent::SetStorage`.
    pub(crate) fn handle_set_storage(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SetStorage { items, name } = event else {
            unreachable!("handle_set_storage called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().storage())
            .fill(&self.async_loader, items);
        *self.client_state.follow_mut(client_state().storage_name()) = name;

        if !self.interface.is_window_with_class_open(WindowClass::Storage) {
            self.interface.open_window(StorageWindow);
        }
    }

    /// `NetworkEvent::SetStorageAmount`.
    pub(crate) fn handle_set_storage_amount(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SetStorageAmount { amount, maximum_amount } = event else {
            unreachable!("handle_set_storage_amount called with a different event variant");
        };

        *self.client_state.follow_mut(client_state().storage_amount()) = amount;
        *self.client_state.follow_mut(client_state().storage_maximum_amount()) = maximum_amount;
    }

    /// `NetworkEvent::StorageItemAdded`.
    pub(crate) fn handle_storage_item_added(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::StorageItemAdded { item } = event else {
            unreachable!("handle_storage_item_added called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().storage())
            .add_item(&self.async_loader, item);
    }

    /// `NetworkEvent::StorageItemRemoved`.
    pub(crate) fn handle_storage_item_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::StorageItemRemoved { storage_index, amount } = event else {
            unreachable!("handle_storage_item_removed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().storage())
            .remove_item(InventoryIndex(storage_index), amount as u16);
    }

    /// `NetworkEvent::StorageClosed`.
    pub(crate) fn handle_storage_closed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::StorageClosed = event else {
            unreachable!("handle_storage_closed called with a different event variant");
        };

        self.close_storage_window();
    }

    /// `NetworkEvent::SetCart`.
    pub(crate) fn handle_set_cart(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SetCart { items } = event else {
            unreachable!("handle_set_cart called with a different event variant");
        };

        // The cart list arrives on map entry and whenever the cart is
        // (re)opened. It is kept in state so the cart window can be
        // opened later without a dedicated request packet.
        self.client_state.follow_mut(client_state().cart()).fill(&self.async_loader, items);
    }

    /// `NetworkEvent::CartItemAdded`.
    pub(crate) fn handle_cart_item_added(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CartItemAdded { item } = event else {
            unreachable!("handle_cart_item_added called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().cart())
            .add_item(&self.async_loader, item);
    }

    /// `NetworkEvent::CartItemRemoved`.
    pub(crate) fn handle_cart_item_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CartItemRemoved { cart_index, amount } = event else {
            unreachable!("handle_cart_item_removed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().cart())
            .remove_item_if_present(cart_index, u16::try_from(amount).unwrap_or(u16::MAX));
    }

    /// `NetworkEvent::CartClosed`.
    pub(crate) fn handle_cart_closed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CartClosed = event else {
            unreachable!("handle_cart_closed called with a different event variant");
        };

        self.close_cart_window();
    }

    /// `NetworkEvent::CartAddItemFailed`.
    pub(crate) fn handle_cart_add_item_failed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CartAddItemFailed { result } = event else {
            unreachable!("handle_cart_add_item_failed called with a different event variant");
        };

        let message = match result {
            0 => "The cart is too heavy to hold this item.",
            _ => "The cart is full.",
        };
        self.push_chat_warning(message.to_owned());
    }

    /// `NetworkEvent::IventoryItemAdded`.
    pub(crate) fn handle_iventory_item_added(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::IventoryItemAdded { item } = event else {
            unreachable!("handle_iventory_item_added called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().inventory())
            .add_item(&self.async_loader, item);

        // TODO: Update the selling items. If you pick up an item
        // that you already have the sell window
        // should allow you to sell the new
        // amount of items.
    }

    /// `NetworkEvent::ItemObtained`.
    pub(crate) fn handle_item_obtained(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ItemObtained {
            item_id,
            quantity,
            is_identified,
        } = event
        else {
            unreachable!("handle_item_obtained called with a different event variant");
        };

        let name = self.library.item_name(item_id, is_identified);
        let message = ro_next_messages::render_template(self.client_state.follow(client_state().localization().chat_item_obtained()), &[
            ("name", name.as_ref()),
            ("quantity", &quantity.to_string()),
        ]);
        self.client_state
            .follow_mut(client_state().chat_messages())
            .push(ChatMessage::new(message, MessageColor::Information));
    }

    /// `NetworkEvent::InventoryItemRemoved`.
    pub(crate) fn handle_inventory_item_removed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::InventoryItemRemoved { index, amount, .. } = event else {
            unreachable!("handle_inventory_item_removed called with a different event variant");
        };

        self.client_state.follow_mut(client_state().inventory()).remove_item(index, amount);
    }

    /// `NetworkEvent::ItemUsed`.
    pub(crate) fn handle_item_used(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ItemUsed { item_id } = event else {
            unreachable!("handle_item_used called with a different event variant");
        };

        // The official client plays the use-item effect (EF_EXIT)
        // for consumables above the classic item ids
        // (`IsAboveItemId600` in the classic client).
        if item_id.0 > 600
            && let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id())
        {
            self.spawn_item_use_effect(entity_id);
        }
    }

    /// `NetworkEvent::InventoryItemAmountChanged`.
    pub(crate) fn handle_inventory_item_amount_changed(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::InventoryItemAmountChanged { index, amount } = event else {
            unreachable!("handle_inventory_item_amount_changed called with a different event variant");
        };

        self.client_state
            .follow_mut(client_state().inventory())
            .update_amount(index, amount);
    }

    /// `NetworkEvent::SellItemList`.
    pub(crate) fn handle_sell_item_list(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::SellItemList { items } = event else {
            unreachable!("handle_sell_item_list called with a different event variant");
        };

        self.shop_open = true;

        // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
        // the regular `DialogMenuPacket`. When opening the shop that dialog should be
        // closed.
        self.client_state.follow_mut(client_state().dialog_window()).end();
        self.interface.close_window_with_class(WindowClass::Dialog);

        let inventory_items = self.client_state.follow(client_state().inventory().items());
        let sell_items = items
            .into_iter()
            .map(|item| {
                let inventory_item = inventory_items
                    .iter()
                    .find(|inventory_item| inventory_item.index == item.inventory_index)
                    .expect("item not in inventory");

                let name = inventory_item.metadata.name.clone();
                let texture = inventory_item.metadata.texture.clone();
                let stats = inventory_item.metadata.stats;
                let description = inventory_item.metadata.description.clone();
                let card_names = inventory_item.metadata.card_names.clone();
                let slot_count = inventory_item.metadata.slot_count;
                let tooltip_text = inventory_item.metadata.tooltip_text.clone();
                let quantity = match &inventory_item.details {
                    ro_next_networking::InventoryItemDetails::Regular { amount, .. } => *amount,
                    ro_next_networking::InventoryItemDetails::Equippable { .. } => 1,
                };

                SellItem {
                    metadata: (
                        ResourceMetadata {
                            name,
                            texture,
                            stats,
                            description,
                            card_names,
                            slot_count,
                            tooltip_text,
                        },
                        quantity,
                    ),
                    inventory_index: item.inventory_index,
                    price: item.price,
                    overcharge_price: item.overcharge_price,
                }
            })
            .collect();

        *self.client_state.follow_mut(client_state().sell_items()) = sell_items;

        self.interface
            .open_window(SellWindow::new(client_state().sell_items(), client_state().sell_cart()));
        self.interface.open_window(SellCartWindow::new(client_state().sell_cart()));
    }

    /// `NetworkEvent::TradeItemOffered`.
    pub(crate) fn handle_trade_item_offered(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::TradeItemOffered { item_id, amount } = event else {
            unreachable!("handle_trade_item_offered called with a different event variant");
        };

        let trade = self.client_state.follow_mut(client_state().trade());
        trade.set_partner_offer(if item_id == 0 {
            format!("{amount} zeny")
        } else {
            format!("item #{item_id} x{amount}")
        });
    }

    /// `NetworkEvent::CartInfo`.
    pub(crate) fn handle_cart_info(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::CartInfo {
            current_count,
            maximum_count,
            current_weight,
            maximum_weight,
        } = event
        else {
            unreachable!("handle_cart_info called with a different event variant");
        };

        if let Some(player) = self.client_state.try_follow_mut(this_player()) {
            player.cart_items = current_count.max(0) as u16;
            player.cart_maximum_items = maximum_count;
            player.cart_weight = current_weight.max(0) as u32;
            player.cart_maximum_weight = maximum_weight.max(0) as u32;
        }
    }

    /// `NetworkEvent::ShowEquipmentWindow`.
    pub(crate) fn handle_show_equipment_window(&mut self, event: NetworkEvent, _client_tick: ClientTick) {
        let NetworkEvent::ShowEquipmentWindow { open } = event else {
            unreachable!("handle_show_equipment_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match (open, self.interface.is_window_with_class_open(WindowClass::Equipment)) {
                (true, false) => self.interface.open_window(EquipmentWindow::new(
                    client_state().inventory().items(),
                    this_player().manually_asserted(),
                )),
                (false, true) => self.interface.close_window_with_class(WindowClass::Equipment),
                _ => {}
            }
        }
    }
}
