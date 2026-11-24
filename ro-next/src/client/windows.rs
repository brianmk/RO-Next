//! Window and UI-service helpers: opening/closing the trade, storage, bank,
//! cart, item-service, refining and vending windows, plus the character
//! selection re-entry helpers.

use crate::*;

impl Client {
    /// Close the Kafra storage window and clear the transient storage state.
    ///
    /// Called when the server closes the storage (`ZC_CLOSE_STORE`), when the
    /// player closes it in the UI and defensively on map changes so a stale
    /// window can never show items from a previous storage session.
    /// Closes the trade windows and drops the transient trade state.
    pub(crate) fn close_trade(&mut self) {
        self.interface.close_window_with_class(WindowClass::Trade);
        self.interface.close_window_with_class(WindowClass::TradeRequest);
        self.client_state.follow_mut(client_state().trade()).close();
    }

    pub(crate) fn close_storage_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Storage);
        self.client_state.follow_mut(client_state().storage()).clear();
        *self.client_state.follow_mut(client_state().storage_amount()) = 0;
        *self.client_state.follow_mut(client_state().storage_maximum_amount()) = 0;
        *self.client_state.follow_mut(client_state().storage_name()) = String::new();
    }

    /// Account id of the logged-in player. The banking requests are checked
    /// against it by the server (`clif_parse_BankOpen`, clif.cpp:7342).
    pub(crate) fn own_account_id(&self) -> Option<AccountId> {
        self.saved_login_data.map(|login_data| login_data.account_id)
    }

    /// Open the bank window, start the banking session and request the vault.
    pub(crate) fn open_bank_window(&mut self) {
        let Some(account_id) = self.own_account_id() else {
            return;
        };

        if !self.interface.is_window_with_class_open(WindowClass::Bank) {
            self.interface.open_window(BankWindow);
        }

        self.client_state.follow_mut(client_state().bank()).open();

        let _ = self.networking_system.open_bank(account_id);
        let _ = self.networking_system.check_bank(account_id);
    }

    /// Close the bank window and end the banking session.
    pub(crate) fn close_bank_window(&mut self) {
        if let Some(account_id) = self.own_account_id() {
            let _ = self.networking_system.close_bank(account_id);
        }

        self.client_state.follow_mut(client_state().bank()).close();
        self.interface.close_window_with_class(WindowClass::Bank);
    }

    /// Close the cart window and drop the transient cart list.
    ///
    /// rAthena sends `ZC_CARTOFF` when the cart is removed, and a map change
    /// invalidates the old list as well.
    pub(crate) fn close_cart_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Cart);
        self.client_state.follow_mut(client_state().cart()).clear();
    }

    /// Open the shared item service window for the given service.
    ///
    /// The window is opened whenever the server sends a service list; opening
    /// it again for a different service replaces the previous window because
    /// all four services share one window class.
    pub(crate) fn open_item_service_window(&mut self, kind: ItemServiceKind) {
        self.interface.close_window_with_class(WindowClass::ItemService);
        self.interface.open_window(ItemServiceWindow::new(kind));
    }

    /// Close the item service window and drop its list.
    pub(crate) fn close_item_service_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::ItemService);
        self.client_state.follow_mut(client_state().item_service()).clear();
    }

    /// Open the NPC refine UI window in the given step.
    pub(crate) fn open_refining_window(&mut self, mode: RefiningMode) {
        self.interface.close_window_with_class(WindowClass::Refining);
        self.interface.open_window(RefiningWindow::new(mode));
    }

    /// Close the NPC refine UI and drop its state.
    pub(crate) fn close_refining_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Refining);
        self.client_state.follow_mut(client_state().item_service().refining()).clear();
    }

    /// Equipment that the NPC refine UI offers for selection.
    ///
    /// rAthena's `clif_refineui_info` requires the item to be identified and
    /// undamaged (`clif.cpp:22520`); the refine database decides whether the
    /// item is refinable at all, so the client only filters by the data it has
    /// (weapons/armor, undamaged, identified, unequipped and below the
    /// pre-renewal refine maximum of 10, `status.hpp:36`).
    pub(crate) fn refinable_inventory_items(&self) -> Vec<ItemServiceEntry> {
        self.client_state
            .follow(client_state().inventory().items())
            .iter()
            .filter(|item| {
                if !item.is_identified() || item.is_equipped() {
                    return false;
                }

                if item.item_type != item_type::WEAPON && item.item_type != item_type::ARMOR {
                    return false;
                }

                match &item.details {
                    ro_next_networking::InventoryItemDetails::Equippable {
                        refinement_level, flags, ..
                    } => *refinement_level < 10 && !flags.contains(EquippableItemFlags::IS_BROKEN),
                    ro_next_networking::InventoryItemDetails::Regular { .. } => false,
                }
            })
            .map(|item| {
                let refine = match &item.details {
                    ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
                    ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
                };
                ItemServiceEntry::new(item.index, item.item_id, item.metadata.name.clone(), refine)
            })
            .collect()
    }

    /// Resolve a service row for one of the player's own inventory items.
    pub(crate) fn inventory_service_entry(&self, index: InventoryIndex) -> Option<ItemServiceEntry> {
        let inventory = self.client_state.follow(client_state().inventory().items());
        let item = inventory.iter().find(|item| item.index == index)?;
        let refine = match &item.details {
            ro_next_networking::InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
            ro_next_networking::InventoryItemDetails::Regular { .. } => 0,
        };

        Some(ItemServiceEntry::new(index, item.item_id, item.metadata.name.clone(), refine))
    }

    /// Close the vending browse window and drop the shop list of the player's
    /// shop.
    pub(crate) fn close_vending_browse(&mut self) {
        self.interface.close_window_with_class(WindowClass::VendingBrowse);
        self.client_state.follow_mut(client_state().vending_browse()).close();
    }

    /// Aborts the current map entry after its map data could not be loaded and
    /// returns to the character selection screen, where the reason is shown.
    ///
    /// Without this the client would keep `self.map == None` and render a black
    /// screen forever, because a failed map load used to be dropped silently.
    pub(crate) fn abort_map_entry(&mut self, message: String) {
        // The session machine records the error, sets the abort deadline and
        // moves to `Recovering`; the client only has to drop the map connection.
        self.session.map_entry_failed(message, Instant::now());

        if self.client_state.try_follow(this_player()).is_none() {
            // The menu background map failed to load; there is no map entry to
            // abort and therefore nothing to go back to.
            if let Some(message) = self.session.take_map_load_error() {
                self.interface.open_window(ErrorWindow::new(message));
            }
            return;
        }

        if self.networking_system.is_map_server_connected() {
            // Dropping the map connection triggers `NetworkEvent::MapServerDisconnected`,
            // which is responsible for going back to the character selection.
            // If that event does not arrive the session machine recovers on its
            // own once the abort deadline expires.
            self.networking_system.disconnect_from_map_server();
        } else {
            // The connection is already gone (e.g. the server kicked us while the
            // map was loading); recover directly.
            self.return_to_character_selection();
        }
    }

    /// Leaves the current map and returns to the character selection screen,
    /// reloading the menu background map.
    /// Open the character selection window, centred in the available screen
    /// area, and start the anti-crash selection cooldown if the client just
    /// returned from a map session.
    pub(crate) fn open_character_selection_if_closed(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            #[cfg(not(feature = "debug"))]
            self.interface.close_all_windows();

            #[cfg(feature = "debug")]
            self.interface.close_all_windows_except(DEBUG_WINDOWS);

            self.open_character_selection_window();
        }
    }

    pub(crate) fn open_character_selection_window(&mut self) {
        self.client_state
            .follow_mut(client_state().character_selection_cooldown())
            .start(Instant::now());

        let window_position = {
            let state = self.client_state.follow(client_state());
            character_selection_window_position(state)
        };

        self.interface.open_window_at_auto_size(
            CharacterSelectionWindow::new(
                client_state().character_slots(),
                client_state().switch_request(),
                client_state().character_previews(),
                client_state().character_selection_cooldown(),
            ),
            window_position,
        );
    }

    pub(crate) fn return_to_character_selection(&mut self) {
        // The server may still be cleaning up the old map session. Block rapid
        // re-selection until the character selection window is shown and the
        // grace period started.
        self.client_state.follow_mut(client_state().character_selection_cooldown()).arm();
        // Chat rooms and the pincode prompt belong to the previous map/account
        // session; never carry them into the character selection screen.
        self.client_state
            .follow_mut(client_state().content_windows().chat_room())
            .leave_locally();
        self.client_state.follow_mut(client_state().content_windows().pincode()).dismiss();

        self.map = None;
        // The same-map teleport fast path must never apply to the next character
        // map entry: the client is back at the character selection and the map
        // is unloaded. Keeping the old name used to skip the map load for a
        // character that lives on the same map as the menu background.
        self.current_map_name = None;
        self.session.recovery_finished();
        // Every entity of the map session is gone; status effects were already
        // cleared by the map-server disconnect handler.
        self.entity_lifecycle.on_warp();
        self.asset_retries.clear();

        if self.networking_system.is_character_server_connected() {
            // The map connection failed (e.g. while the map server is still
            // restarting) but we are still connected to the character server:
            // go back to the character selection and refresh the list so the
            // player can simply try again.
            let _ = self.networking_system.request_character_list();
        } else {
            self.session.schedule_character_reconnect(Instant::now());
        }

        self.particle_holder.clear();
        self.effect_holder.clear();
        self.point_light_manager.clear();
        self.audio_engine.clear_ambient_sound();

        self.client_state.follow_mut(client_state().entities()).clear();
        self.client_state.follow_mut(client_state().dead_entities()).clear();
        self.chat_bubbles.clear();
        self.pending_item_drag = None;
        self.skill_name_texts.clear();
        self.cast_bars.clear();
        self.aftercast_act_until = None;
        self.client_state.follow_mut(client_state().ground_items()).clear();
        self.pending_context_menu = None;
        *self.client_state.follow_mut(client_state().buffered_action()) = None;

        // We are back at the character selection screen; the map
        // music must not continue there, but the menu music has to
        // play instead of silence.
        self.audio_engine
            .play_background_music_track(crate::client_config::background_music());

        #[cfg(not(feature = "debug"))]
        self.interface.close_all_windows();

        #[cfg(feature = "debug")]
        self.interface.close_all_windows_except(DEBUG_WINDOWS);

        self.async_loader
            .request_map_load(crate::client_config::default_map().to_string(), Some(TilePosition::new(0, 0)));
    }
}
