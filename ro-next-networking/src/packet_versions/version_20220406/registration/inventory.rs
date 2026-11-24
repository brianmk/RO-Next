//! inventory packet registrations.

use super::*;

pub(crate) fn register_inventory<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
    buffers: &ListBuffers,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    let ListBuffers {
        inventory_items,
        storage_items,
        storage_name: _,
        cart_items,
    } = buffers;

    packet_handler.register(|packet: UseItemAckPacket| match packet.result {
        0 => vec![NetworkEvent::ChatMessage {
            text: "Failed to use item.".to_owned(),
            color: MessageColor::Error,
        }],
        _ => vec![
            NetworkEvent::InventoryItemAmountChanged {
                index: packet.inventory_index,
                amount: packet.amount.max(0) as u16,
            },
            NetworkEvent::ItemUsed { item_id: packet.item_id },
        ],
    })?;

    packet_handler.register(|packet: ItemDisappearPacket| NetworkEvent::RemoveGroundItem {
        entity_id: packet.entity_id,
    })?;

    packet_handler.register(|packet: CartItemCountInfoPacket2019| NetworkEvent::CartInfo {
        current_count: packet.current_count,
        maximum_count: packet.maximum_count,
        current_weight: packet.current_weight,
        maximum_weight: packet.maximum_weight,
    })?;

    packet_handler.register(|packet: StorageItemCountInfoPacket2019| NetworkEvent::SetStorageAmount {
        amount: packet.amount,
        maximum_amount: packet.maximum_amount,
    })?;

    packet_handler.register(|packet: StorageDeleteItemPacket2019| NetworkEvent::StorageItemRemoved {
        storage_index: packet.storage_index.saturating_sub(1),
        amount: packet.amount,
    })?;

    packet_handler.register(|packet: StorageItemAddedPacket2019| {
        let StorageItemAddedPacket2019 {
            storage_index,
            amount,
            item_id,
            item_type,
            identified,
            damaged,
            refinement_level,
            card,
            option_data,
        } = packet;

        // The 0x0A0A packet does not carry the equipment position, but it
        // carries everything else about the item: refinement, broken state,
        // cards and random options. Items with those details are equipment, so
        // build the equippable variant to keep them (starting with the 2019
        // server, `ZC_ADD_ITEM_TO_STORE` is also sent when the player stores an
        // item, and without this the storage tooltip lost refine/cards/options
        // until the storage was reopened).
        let is_equipment = crate::item_type::is_equipment(item_type)
            || damaged != 0
            || refinement_level != 0
            || card.iter().any(|card| *card != 0)
            || option_data.iter().any(|option| option.index != 0);

        let details = match is_equipment {
            true => InventoryItemDetails::Equippable {
                // The packet has no equipment position. Weapons and ammunition
                // have an unambiguous slot; other equipment keeps an empty
                // position (the item is displayed in the storage and the server
                // sends the real position again when it is withdrawn).
                equip_position: match item_type {
                    crate::item_type::WEAPON => EquipPosition::RIGHT_HAND,
                    crate::item_type::AMMO => EquipPosition::AMMO,
                    _ => EquipPosition::empty(),
                },
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: option_data.len() as u8,
                option_data,
                refinement_level,
                enchantment_level: 0,
                flags: {
                    let mut flags = EquippableItemFlags::empty();
                    flags.set(EquippableItemFlags::IDENTIFIED, identified != 0);
                    flags.set(EquippableItemFlags::IS_BROKEN, damaged != 0);
                    flags
                },
            },
            false => InventoryItemDetails::Regular {
                amount: u16::try_from(amount.max(1)).unwrap_or(u16::MAX),
                equipped_position: EquipPosition::empty(),
                flags: match identified != 0 {
                    true => RegularItemFlags::IDENTIFIED,
                    false => RegularItemFlags::empty(),
                },
            },
        };

        NetworkEvent::StorageItemAdded {
            item: InventoryItem {
                // The 0x0A0A wire index is `server slot + 1`.
                index: InventoryIndex(storage_index.saturating_sub(1)),
                metadata: NoMetadata,
                item_id,
                item_type,
                slot: card,
                hire_expiration_date: 0,
                details,
            },
        }
    })?;

    packet_handler.register(|_: StorageClosedPacket| NetworkEvent::StorageClosed)?;

    packet_handler.register(|packet: ExchangeAddItemPacket2019| NetworkEvent::TradeAddResult {
        inventory_index: packet.inventory_index,
        result: packet.result,
    })?;

    packet_handler.register(|packet: ItemPickupAckPacket2019| {
        if packet.result != 0 {
            return Vec::new();
        }

        let ItemPickupAckPacket2019 {
            inventory_index,
            amount,
            item_id,
            identified,
            damaged,
            refinement_level,
            card,
            equip_position,
            item_type,
            hire_expiration_date,
            bind_on_equip_type,
            option_data,
            item_sprite_number,
            favorite,
            ..
        } = packet;

        let details = match equip_position.is_empty() {
            true => InventoryItemDetails::Regular {
                amount,
                equipped_position: EquipPosition::empty(),
                flags: {
                    let mut flags = RegularItemFlags::empty();
                    flags.set(RegularItemFlags::IDENTIFIED, identified != 0);
                    flags.set(RegularItemFlags::FAVORITE, favorite != 0);
                    flags
                },
            },
            false => InventoryItemDetails::Equippable {
                equip_position,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type,
                w_item_sprite_number: item_sprite_number,
                option_count: option_data.len() as u8,
                option_data,
                refinement_level,
                enchantment_level: 0,
                flags: {
                    let mut flags = EquippableItemFlags::empty();
                    flags.set(EquippableItemFlags::IDENTIFIED, identified != 0);
                    flags.set(EquippableItemFlags::IS_BROKEN, damaged != 0);
                    flags.set(EquippableItemFlags::FAVORITE, favorite != 0);
                    flags
                },
            },
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index: inventory_index,
            item_id,
            item_type,
            slot: card,
            hire_expiration_date: hire_expiration_date.max(0) as u32,
            details,
        };

        let picked_up = NetworkEvent::ItemPickedUp { item_id, amount };

        vec![NetworkEvent::IventoryItemAdded { item }, picked_up]
    })?;

    packet_handler.register({
        let inventory_items = inventory_items.clone();
        let storage_items = storage_items.clone();
        let cart_items = cart_items.clone();

        move |packet: RegularItemListPacket| {
            let is_storage = packet.inventory_type == INVENTORY_TYPE_STORAGE;

            let target = match packet.inventory_type {
                INVENTORY_TYPE_INVENTORY => inventory_items.clone(),
                INVENTORY_TYPE_STORAGE => storage_items.clone(),
                INVENTORY_TYPE_CART => cart_items.clone(),
                _ => return NoNetworkEvents,
            };

            let mut slot = target.borrow_mut();
            // A list without a preceding start packet used to panic inside the
            // networking thread. Ignore the packet instead: the stream is still
            // in sync because the packet itself was parsed completely.
            let Some(items) = slot.as_mut() else {
                eprintln!("[network] regular item list without a preceding start packet; ignoring");
                return NoNetworkEvents;
            };
            items.extend(packet.item_information.into_iter().map(|item_information| {
                let RegularItemInformation {
                    index,
                    item_id,
                    item_type,
                    amount,
                    equipped_position,
                    slot,
                    hire_expiration_date,
                    flags,
                } = item_information;

                InventoryItem {
                    // Storage rows use the `server slot + 1` wire encoding,
                    // while InventoryIndex subtracts two. Shift them back so
                    // the index is the plain 0-based storage slot.
                    index: match is_storage {
                        true => InventoryIndex(index.0.wrapping_add(1)),
                        false => index,
                    },
                    metadata: NoMetadata,
                    item_id,
                    item_type,
                    slot,
                    hire_expiration_date,
                    details: InventoryItemDetails::Regular {
                        amount,
                        equipped_position,
                        flags,
                    },
                }
            }));
            NoNetworkEvents
        }
    })?;

    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: EquippableItemListPacket| {
            let mut slot = inventory_items.borrow_mut();
            // See above: never panic on an out-of-order inventory packet.
            let Some(items) = slot.as_mut() else {
                eprintln!("[network] equippable inventory without a preceding start packet; ignoring");
                return NoNetworkEvents;
            };
            items.extend(packet.item_information.into_iter().map(|item| {
                let EquippableItemInformation {
                    index,
                    item_id,
                    item_type,
                    equip_position,
                    equipped_position,
                    slot,
                    hire_expiration_date,
                    bind_on_equip_type,
                    w_item_sprite_number,
                    option_count,
                    option_data,
                    refinement_level,
                    enchantment_level,
                    flags,
                } = item;

                InventoryItem {
                    index,
                    metadata: NoMetadata,
                    item_id,
                    item_type,
                    slot,
                    hire_expiration_date,
                    details: InventoryItemDetails::Equippable {
                        equip_position,
                        equipped_position,
                        bind_on_equip_type,
                        w_item_sprite_number,
                        option_count,
                        option_data,
                        refinement_level,
                        enchantment_level,
                        flags,
                    },
                }
            }));
            NoNetworkEvents
        }
    })?;

    packet_handler.register({
        let inventory_items = inventory_items.clone();
        let storage_items = storage_items.clone();
        let cart_items = cart_items.clone();

        move |packet: EquippableItemListPacket2019| {
            let is_storage = packet.inventory_type == INVENTORY_TYPE_STORAGE;

            let target = match packet.inventory_type {
                INVENTORY_TYPE_INVENTORY => inventory_items.clone(),
                INVENTORY_TYPE_STORAGE => storage_items.clone(),
                INVENTORY_TYPE_CART => cart_items.clone(),
                _ => return NoNetworkEvents,
            };

            let mut slot = target.borrow_mut();
            // See above: a 2019 equippable list can arrive without a start packet
            // (e.g. after a reconnect); ignore it instead of panicking.
            let Some(items) = slot.as_mut() else {
                eprintln!("[network] 2019 equippable item list without a preceding start packet; ignoring");
                return NoNetworkEvents;
            };
            items.extend(packet.item_information.into_iter().map(|item| {
                let item = EquippableItemInformation::from(item);

                InventoryItem {
                    index: match is_storage {
                        true => InventoryIndex(item.index.0.wrapping_add(1)),
                        false => item.index,
                    },
                    metadata: NoMetadata,
                    item_id: item.item_id,
                    item_type: item.item_type,
                    slot: item.slot,
                    hire_expiration_date: item.hire_expiration_date,
                    details: InventoryItemDetails::Equippable {
                        equip_position: item.equip_position,
                        equipped_position: item.equipped_position,
                        bind_on_equip_type: item.bind_on_equip_type,
                        w_item_sprite_number: item.w_item_sprite_number,
                        option_count: item.option_count,
                        option_data: item.option_data,
                        refinement_level: item.refinement_level,
                        enchantment_level: item.enchantment_level,
                        flags: item.flags,
                    },
                }
            }));
            NoNetworkEvents
        }
    })?;

    packet_handler.register(|packet: EquippableSwitchItemListPacket| NetworkEvent::EquipmentSwitchItems {
        items: packet.item_information,
    })?;

    packet_handler.register(|packet: UpdateShowEquipPacket| NetworkEvent::ShowEquipmentWindow {
        open: packet.open_equip_window != 0,
    })?;
    register_ignored_noop::<_, UpdateConfigurationPacket>(packet_handler)?;

    packet_handler.register(|packet: ItemPickupPacket| {
        let ItemPickupPacket {
            index,
            quantity,
            item_id,
            is_identified,
            is_broken,
            cards,
            equip_position,
            item_type,
            result,
            hire_expiration_date,
            bind_on_equip_type,
            option_data,
            favorite,
            look,
            refinement_level,
            enchantment_level,
        } = packet;

        if result != ItemPickupResult::Success {
            return vec![NetworkEvent::ChatMessage {
                text: "Failed to pick up item.".to_string(),
                color: MessageColor::Error,
            }];
        }

        // TODO: Not sure where to store these, since the *InventoryItem packets are not
        // sending these either. We will certainly use them at some point though.
        let _ = (favorite, look);

        let details = match equip_position.is_empty() {
            true => InventoryItemDetails::Regular {
                amount: quantity,
                equipped_position: equip_position,
                flags: {
                    let mut flags = RegularItemFlags::empty();
                    flags.set(RegularItemFlags::IDENTIFIED, is_identified != 0);
                    flags
                },
            },
            false => InventoryItemDetails::Equippable {
                equip_position,
                equipped_position: EquipPosition::empty(),
                bind_on_equip_type,
                w_item_sprite_number: 0,
                option_count: option_data.len() as u8,
                option_data,
                refinement_level,
                enchantment_level,
                flags: {
                    let mut flags = EquippableItemFlags::empty();
                    flags.set(EquippableItemFlags::IDENTIFIED, is_identified != 0);
                    flags.set(EquippableItemFlags::IS_BROKEN, is_broken != 0);
                    flags
                },
            },
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index,
            item_id,
            item_type,
            slot: cards,
            hire_expiration_date,
            details,
        };

        let is_identified = is_identified != 0;

        vec![NetworkEvent::IventoryItemAdded { item }, NetworkEvent::ItemObtained {
            item_id,
            quantity,
            is_identified,
        }]
    })?;

    packet_handler.register(|packet: RemoveItemFromInventoryPacket| NetworkEvent::InventoryItemRemoved {
        reason: packet.remove_reason,
        index: packet.index,
        amount: packet.amount,
    })?;

    packet_handler.register(|packet: ShopItemListPacket| {
        let items = packet
            .items
            .into_iter()
            .map(|item| ShopItem {
                metadata: NoMetadata,
                item_id: item.item_id,
                item_type: item.item_type,
                // The 2019 list sends the regular and the discounted price. The server
                // charges the discounted price, so display that one.
                price: match item.discount_price {
                    Price(0) => item.price,
                    discount_price => discount_price,
                },
                quantity: ItemQuantity::Infinite,
                // The 2019 list does not send the weight or the equipment location.
                weight: 0,
                location: 0,
            })
            .collect();

        NetworkEvent::OpenShop { items }
    })?;

    packet_handler.register(|packet: BuyItemsResultPacket| NetworkEvent::BuyingCompleted { result: packet.result })?;

    packet_handler.register(|packet: SellItemsResultPacket| NetworkEvent::SellingCompleted { result: packet.result })?;

    packet_handler.register(|packet: EquipAmmunitionPacket| NetworkEvent::AmmunitionEquipped {
        inventory_index: packet.inventory_index,
    })?;

    packet_handler.register(|packet: CartItemAddedPacket2019| {
        let amount = u16::try_from(packet.amount).unwrap_or(u16::MAX);
        let flags = match packet.identified != 0 {
            true => RegularItemFlags::IDENTIFIED,
            false => RegularItemFlags::empty(),
        };

        NetworkEvent::CartItemAdded {
            item: InventoryItem {
                metadata: NoMetadata,
                index: packet.cart_index,
                item_id: packet.item_id,
                item_type: packet.item_type,
                slot: packet.card,
                hire_expiration_date: 0,
                details: InventoryItemDetails::Regular {
                    amount,
                    equipped_position: EquipPosition::empty(),
                    flags,
                },
            },
        }
    })?;

    packet_handler.register(|packet: CartDeleteItemPacket2019| NetworkEvent::CartItemRemoved {
        cart_index: packet.cart_index,
        amount: packet.amount,
    })?;

    packet_handler.register(|packet: CartAddItemAckPacket2019| NetworkEvent::CartAddItemFailed { result: packet.result })?;

    packet_handler.register(|_: CartOffPacket| NetworkEvent::CartClosed)?;

    packet_handler.register(|packet: VendingItemListPacket2019| {
        let items = packet
            .items
            .into_iter()
            .map(|entry| crate::items::VendingShopItem {
                item: ShopItem {
                    metadata: NoMetadata,
                    item_id: entry.item_id,
                    item_type: entry.item_type,
                    price: Price(entry.price),
                    quantity: ItemQuantity::Fixed(entry.amount.into()),
                    weight: 0,
                    location: entry.location,
                },
                cart_index: entry.cart_index,
            })
            .collect();

        NetworkEvent::VendingShopList {
            owner_account_id: packet.owner_account_id,
            shop_id: packet.shop_id,
            items,
        }
    })?;

    packet_handler.register(|packet: MailItemClaimResultPacket| NetworkEvent::MailAttachmentClaimed {
        mail_id: packet.mail_id,
        zeny: false,
        result: packet.result,
    })?;

    packet_handler.register(|packet: WeaponRefineListPacket| NetworkEvent::WeaponRefineList { entries: packet.entries })?;

    packet_handler.register(|packet: WeaponRefineResultPacket| NetworkEvent::WeaponRefineResult {
        result: packet.result,
        item_id: packet.item_id,
    })?;

    packet_handler.register(|packet: ItemRefiningResultPacket| NetworkEvent::ItemRefiningResult {
        index: packet.index,
        refine: packet.refine,
        result: packet.result,
    })?;

    packet_handler.register(|packet: RepairItemListPacket| NetworkEvent::RepairItemList { entries: packet.entries })?;

    packet_handler.register(|packet: RepairItemResultPacket| NetworkEvent::RepairItemResult {
        index: packet.index,
        success: packet.result == 0,
    })?;

    packet_handler.register(|packet: IdentifyItemListPacket| NetworkEvent::IdentifyItemList { indexes: packet.indexes })?;

    packet_handler.register(|packet: IdentifyItemResultPacket| NetworkEvent::IdentifyItemResult {
        index: packet.index,
        success: packet.result == 0,
    })?;

    packet_handler.register(|packet: CardCompositionListPacket| NetworkEvent::CardCompositionList { indexes: packet.indexes })?;

    packet_handler.register(|packet: CardCompositionResultPacket| NetworkEvent::CardCompositionResult {
        equip_index: packet.equip_index,
        card_index: packet.card_index,
        success: packet.result == 0,
    })?;

    packet_handler.register(|packet: UpdateCardSlotPacket| NetworkEvent::CardSlotUpdated {
        wear_state: packet.wear_state,
        card_slot: packet.card_slot,
        item_id: packet.item_id,
        equip_flag: packet.equip_flag,
    })?;

    Ok(())
}
