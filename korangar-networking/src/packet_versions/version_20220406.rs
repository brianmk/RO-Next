use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use crate::event::{NetworkEventList, NoNetworkEvents};
use crate::items::ItemQuantity;
use crate::{
    CharacterServerLoginData, HotkeyState, InventoryItem, InventoryItemDetails, LoginServerLoginData, MessageColor, NetworkEvent,
    NoMetadata, ShopItem, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

pub fn register_login_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginServerLoginSuccessPacket| NetworkEvent::LoginServerConnected {
        character_servers: packet.character_server_information,
        login_data: LoginServerLoginData {
            account_id: packet.account_id,
            login_id1: packet.login_id1,
            login_id2: packet.login_id2,
            sex: packet.sex,
        },
    })?;
    packet_handler.register(|packet: LoginFailedPacket| {
        let (reason, message) = match packet.reason {
            LoginFailedReason::ServerClosed => (UnifiedLoginFailedReason::ServerClosed, "Server closed".to_string()),
            LoginFailedReason::AlreadyLoggedIn => (
                UnifiedLoginFailedReason::AlreadyLoggedIn,
                "Someone has already logged in with this id".to_string(),
            ),
            LoginFailedReason::AlreadyOnline => (UnifiedLoginFailedReason::AlreadyOnline, "Already online".to_string()),
        };

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: LoginFailedPacket2| {
        let (reason, message) = match packet.reason {
            LoginFailedReason2::UnregisteredId => (UnifiedLoginFailedReason::UnregisteredId, "Unregistered id".to_string()),
            LoginFailedReason2::IncorrectPassword => (UnifiedLoginFailedReason::IncorrectPassword, "Incorrect password".to_string()),
            LoginFailedReason2::IdExpired => (UnifiedLoginFailedReason::IdExpired, "Id has expired".to_string()),
            LoginFailedReason2::RejectedFromServer => (UnifiedLoginFailedReason::RejectedFromServer, "Rejected from server".to_string()),
            LoginFailedReason2::BlockedByGMTeam => (UnifiedLoginFailedReason::BlockedByGMTeam, "Blocked by gm team".to_string()),
            LoginFailedReason2::GameOutdated => (UnifiedLoginFailedReason::GameOutdated, "Game outdated".to_string()),
            LoginFailedReason2::LoginProhibitedUntil => (
                UnifiedLoginFailedReason::LoginProhibitedUntil,
                format!("You are prohibited to log in until {}.", packet.date),
            ),
            LoginFailedReason2::ServerFull => (UnifiedLoginFailedReason::ServerFull, "Server is full".to_string()),
            LoginFailedReason2::CompanyAccountLimitReached => (
                UnifiedLoginFailedReason::CompanyAccountLimitReached,
                "Company account limit reached".to_string(),
            ),
            LoginFailedReason2::BannedByDBATeam => (UnifiedLoginFailedReason::BannedByDBATeam, "Banned by DBA team".to_string()),
            LoginFailedReason2::UnconfirmedEmail => (UnifiedLoginFailedReason::UnconfirmedEmail, "Email not confirmed".to_string()),
            LoginFailedReason2::BannedByGMTeam => (UnifiedLoginFailedReason::BannedByGMTeam, "Banned by GM team".to_string()),
            LoginFailedReason2::TemporaryBanForDatabaseWork => (
                UnifiedLoginFailedReason::TemporaryBanForDatabaseWork,
                "Working in DB".to_string(),
            ),
            LoginFailedReason2::SelfLocked => (UnifiedLoginFailedReason::SelfLocked, "Self lock".to_string()),
            LoginFailedReason2::NotPermittedGroup => (UnifiedLoginFailedReason::NotPermittedGroup, "Not Permitted Group".to_string()),
            LoginFailedReason2::AccountIdErased => (
                UnifiedLoginFailedReason::AccountIdErased,
                "This ID has been totally erased".to_string(),
            ),
            LoginFailedReason2::LoginInformationRemains => (
                UnifiedLoginFailedReason::LoginInformationRemains,
                format!("Login information remains at {}", packet.date),
            ),
            LoginFailedReason2::LockedForHackingInvestigation => (
                UnifiedLoginFailedReason::LockedForHackingInvestigation,
                "Account has been locked for a hacking investigation. Please contact the GM Team for more information".to_string(),
            ),
            LoginFailedReason2::TemporaryLockedForBugInvestigation => (
                UnifiedLoginFailedReason::TemporaryLockedForBugInvestigation,
                "This account has been temporarily prohibited from login due to a bug-related investigation".to_string(),
            ),
            LoginFailedReason2::DeletingCharacter => (
                UnifiedLoginFailedReason::DeletingCharacter,
                "This character is being deleted. Login is temporarily unavailable for the time being".to_string(),
            ),
            LoginFailedReason2::DeletingSpouseCharacter => (
                UnifiedLoginFailedReason::DeletingSpouseCharacter,
                "This character is being deleted. Login is temporarily unavailable for the time being".to_string(),
            ),
            LoginFailedReason2::UnknownError => (UnifiedLoginFailedReason::UnknownError, "Unknown error".to_string()),
        };

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;

    Ok(())
}

pub fn register_character_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    packet_handler.register(|packet: LoginFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            LoginFailedReason::ServerClosed => "Server closed",
            LoginFailedReason::AlreadyLoggedIn => "Someone has already logged in with this id",
            LoginFailedReason::AlreadyOnline => "Already online",
        };

        NetworkEvent::CharacterServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(
        |packet: CharacterServerLoginSuccessPacket| NetworkEvent::CharacterServerConnected {
            normal_slot_count: packet.normal_slot_count as usize,
        },
    )?;
    packet_handler.register(|packet: RequestCharacterListSuccessPacket| NetworkEvent::CharacterList {
        characters: packet.character_information,
    })?;
    // CondaRO runs a 2019 rAthena/PACKETVER 20190605 server. The authoritative
    // character list is the 0x006B packet the char server sends right after
    // CH_ENTER. 0x099D is only a duplicate response to the 0x09A1 request (and
    // rAthena sends an extra EMPTY 0x099D when the account has exactly 3
    // characters), so it must not overwrite the list: treat it as a no-op.
    packet_handler.register(|packet: CharacterListPacket| NetworkEvent::CharacterList {
        characters: packet.character_information.into_iter().map(CharacterInformation::from).collect(),
    })?;
    packet_handler.register_noop::<CharacterInfoPerPagePacket>()?;
    packet_handler.register_noop::<CharacterSlotPagePacket>()?;
    packet_handler.register_noop::<CharacterBanListPacket>()?;
    packet_handler.register_noop::<LoginPincodePacket>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register(|packet: CharacterSelectionSuccessPacket| {
        let login_data = CharacterServerLoginData {
            server_ip: IpAddr::V4(packet.map_server_ip.into()),
            server_port: packet.map_server_port,
            character_id: packet.character_id,
        };

        NetworkEvent::CharacterSelected { login_data }
    })?;
    packet_handler.register(|packet: CharacterSelectionFailedPacket| {
        let (reason, message) = match packet.reason {
            CharacterSelectionFailedReason::RejectedFromServer => (
                UnifiedCharacterSelectionFailedReason::RejectedFromServer,
                "Rejected from server",
            ),
        };

        NetworkEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|_: MapServerUnavailablePacket| {
        let reason = UnifiedCharacterSelectionFailedReason::MapServerUnavailable;
        let message = "Map server currently unavailable";

        NetworkEvent::CharacterSelectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: CreateCharacterSuccessPacket| NetworkEvent::CharacterCreated {
        character_information: packet.character_information,
    })?;
    packet_handler.register(|packet: CreateCharacterSuccessPacket2019| NetworkEvent::CharacterCreated {
        character_information: packet.character_information.into(),
    })?;
    packet_handler.register(|packet: CharacterCreationFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterCreationFailedReason::CharacterNameAlreadyUsed => "Character name is already used",
            CharacterCreationFailedReason::NotOldEnough => "You are not old enough to create a character",
            CharacterCreationFailedReason::NotAllowedToUseSlot => "You are not allowed to use this character slot",
            CharacterCreationFailedReason::CharacterCerationFailed => "Character creation failed",
        };

        NetworkEvent::CharacterCreationFailed { reason, message }
    })?;
    packet_handler.register(|_: CharacterDeletionSuccessPacket| NetworkEvent::CharacterDeleted)?;
    packet_handler.register(|packet: CharacterDeletionFailedPacket| {
        let reason = packet.reason;
        let message = match reason {
            CharacterDeletionFailedReason::NotAllowed => "You are not allowed to delete this character",
            CharacterDeletionFailedReason::CharacterNotFound => "Character was not found",
            CharacterDeletionFailedReason::NotEligible => "Character is not eligible for deletion",
        };
        NetworkEvent::CharacterDeletionFailed { reason, message }
    })?;
    packet_handler.register(|packet: SwitchCharacterSlotResponsePacket| match packet.status {
        SwitchCharacterSlotResponseStatus::Success => NetworkEvent::CharacterSlotSwitched,
        SwitchCharacterSlotResponseStatus::Error => NetworkEvent::CharacterSlotSwitchFailed,
    })?;
    // The 2019 character server answers a slot change with 0x08D5 instead of
    // 0x0B70; both are 8 byte and byte-identical, so the same event is emitted.
    packet_handler.register(|packet: SwitchCharacterSlotResponsePacket2019| match packet.status {
        SwitchCharacterSlotResponseStatus::Success => NetworkEvent::CharacterSlotSwitched,
        SwitchCharacterSlotResponseStatus::Error => NetworkEvent::CharacterSlotSwitchFailed,
    })?;

    Ok(())
}

pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    // This is a bit of a workaround for the way that the inventory is
    // sent. There is a single packet to start the inventory list,
    // followed by an arbitary number of item packets, and in the
    // end a sinle packet to mark the list as complete.
    //
    // This variable provides some transient storage shared by all the inventory
    // handlers.
    let inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));

    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
    packet_handler.register(|packet: SoundPacket| NetworkEvent::PlaySound {
        name: packet.name,
        act: packet.act,
        term: packet.term,
        account_id: packet.account_id,
    })?;
    packet_handler.register(|packet: BroadcastMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Broadcast,
    })?;
    packet_handler.register(|packet: Broadcast2MessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.font_color.red,
            green: packet.font_color.green,
            blue: packet.font_color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    // rAthena sends the boolean `ok` flag directly in `result` (1 = the item was
    // used, 0 = using it failed) together with the remaining amount.
    packet_handler.register(|packet: UseItemAckPacket| match packet.result {
        0 => Some(NetworkEvent::ChatMessage {
            text: "Failed to use item.".to_owned(),
            color: MessageColor::Error,
        }),
        _ => Some(NetworkEvent::InventoryItemAmountChanged {
            index: packet.inventory_index,
            amount: packet.amount.max(0) as u16,
        }),
    })?;
    packet_handler.register(|packet: WhisperReceivedPacket| NetworkEvent::ChatMessage {
        text: format!("[Whisper] {} : {}", packet.sender_name, packet.message),
        color: MessageColor::Information,
    })?;
    packet_handler.register(|packet: WhisperAckPacket| {
        let text = match packet.result {
            WhisperResult::Success => None,
            WhisperResult::TargetOffline => Some("The player is not online."),
            WhisperResult::Ignored => Some("The player is ignoring whispers."),
            WhisperResult::AllIgnored => Some("The player is not accepting whispers."),
        };

        text.map(|text| NetworkEvent::ChatMessage {
            text: text.to_owned(),
            color: MessageColor::Error,
        })
    })?;
    packet_handler.register(|packet: OverheadMessagePacket| {
        // 2019 `ZC_NOTIFY_CHAT` (0x008D): the message is both shown in the chat
        // window and above the speaking entity. The client decides whether an
        // overhead bubble is drawn (only player entities get one).
        let OverheadMessagePacket { entity_id, message } = packet;

        vec![
            NetworkEvent::ChatMessage {
                text: message.clone(),
                color: MessageColor::Broadcast,
            },
            NetworkEvent::OverheadChatMessage {
                entity_id,
                text: message,
                color: MessageColor::Broadcast,
            },
        ]
    })?;
    packet_handler.register(|packet: ServerMessagePacket| NetworkEvent::ChatMessage {
        text: packet.message,
        color: MessageColor::Server,
    })?;
    packet_handler.register_noop::<MessageTablePacket>()?;
    packet_handler.register(|packet: EntityMessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.color.red,
            green: packet.color.green,
            blue: packet.color.blue,
        };
        NetworkEvent::ChatMessage {
            text: packet.message,
            color,
        }
    })?;
    packet_handler.register(|packet: DisplayEmotionPacket| NetworkEvent::DisplayEmotion {
        entity_id: packet.entity_id,
        emotion: packet.emotion,
    })?;
    packet_handler.register(|packet: EntityMovePacket| {
        let EntityMovePacket {
            entity_id,
            from_to,
            starting_timestamp,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        NetworkEvent::EntityMove {
            entity_id,
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register_noop::<EntityStopMovePacket>()?;
    packet_handler.register(|packet: PlayerMovePacket| {
        let PlayerMovePacket {
            starting_timestamp,
            from_to,
        } = packet;

        let (origin, destination) = from_to.to_origin_destination();

        NetworkEvent::PlayerMove {
            origin,
            destination,
            starting_timestamp,
        }
    })?;
    packet_handler.register(|packet: ChangeMapPacket| {
        let ChangeMapPacket { map_name, position } = packet;

        let map_name = map_name.replace(".gat", "");

        NetworkEvent::ChangeMap { map_name, position }
    })?;
    packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: EntityAppearPacket| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityAppear2Packet| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: MovingEntityAppearPacket| NetworkEvent::AddEntity {
        entity_data: packet.into(),
    })?;
    packet_handler.register(|packet: EntityDisAppearPacket| NetworkEvent::RemoveEntity {
        entity_id: packet.entity_id,
        reason: packet.reason,
    })?;
    packet_handler.register(|packet: GroundItemAppearPacket| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear2Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear3Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: GroundItemAppear4Packet| NetworkEvent::AddGroundItem {
        entity_id: packet.entity_id,
        item_id: packet.item_id,
        is_identified: packet.is_identified != 0,
        quantity: packet.quantity,
        position: packet.position,
        x_offset: packet.x_offset,
        y_offset: packet.y_offset,
    })?;
    packet_handler.register(|packet: ItemDisappearPacket| NetworkEvent::RemoveGroundItem {
        entity_id: packet.entity_id,
    })?;
    packet_handler.register(|packet: UpdateStatPacket| {
        let UpdateStatPacket { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket1| {
        let UpdateStatPacket1 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket2| {
        let UpdateStatPacket2 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    packet_handler.register(|packet: UpdateStatPacket3| {
        let UpdateStatPacket3 { stat_type } = packet;
        NetworkEvent::UpdateStat { stat_type }
    })?;
    // 2019 map-load burst packets: sent on every map load / when entities turn or
    // enter the view. They are registered (as noop for now) so that they do not
    // throw away the rest of the TCP read, which used to lose spawn packets.
    packet_handler.register_noop::<ChangeDirectionPacket2019>()?;
    packet_handler.register_noop::<NotifyActorInitPacket>()?;
    packet_handler.register_noop::<EnterStatusEffectPacket2019>()?;
    // Remaining frequent 2019 packets that the client does not use yet. They are
    // parsed with their exact 2019 sizes; a real handler can replace the noop
    // later (shop: handled by the NPC shop branch, status effects: buff UI).
    packet_handler.register_noop::<CartItemCountInfoPacket2019>()?;
    packet_handler.register_noop::<UserCountPacket2019>()?;
    packet_handler.register_noop::<RefuseEnterPacket2019>()?;
    packet_handler.register_noop::<PartyMemberHealthPacket2019>()?;
    packet_handler.register_noop::<ExchangeAddItemPacket2019>()?;
    packet_handler.register_noop::<ExchangeConcludePacket2019>()?;
    packet_handler.register_noop::<ExchangeCancelPacket2019>()?;
    packet_handler.register_noop::<ExchangeExecPacket2019>()?;
    packet_handler.register_noop::<ExchangeUndoPacket2019>()?;
    packet_handler.register_noop::<StorageItemCountInfoPacket2019>()?;
    packet_handler.register_noop::<StorageDeleteItemPacket2019>()?;
    packet_handler.register_noop::<ExchangeRequestPacket2019>()?;
    packet_handler.register_noop::<ExchangeAckPacket2019>()?;
    // 2019 era `ZC_ITEM_PICKUP_ACK` (0x0A37). The server uses this packet both for
    // picking items up from the ground and for items that are added to the
    // inventory by a shop purchase, so the item is added to the inventory state
    // here (it used to be a no-op registration, which silently dropped bought
    // items).
    packet_handler.register(|packet: ItemPickupAckPacket2019| {
        if packet.result != 0 {
            return None;
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
            ..
        } = packet;

        // TODO: Not sure where to store `favorite`, it is not used by the inventory
        // yet.
        let _ = packet.favorite;

        let details = match equip_position.is_empty() {
            true => InventoryItemDetails::Regular {
                amount,
                equipped_position: EquipPosition::empty(),
                flags: {
                    let mut flags = RegularItemFlags::empty();
                    flags.set(RegularItemFlags::IDENTIFIED, identified != 0);
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
                    flags
                },
            },
        };

        let item = InventoryItem {
            metadata: NoMetadata,
            index: inventory_index,
            item_id,
            item_type,
            slot: [u32::from(card[0]), u32::from(card[1]), u32::from(card[2]), u32::from(card[3])],
            hire_expiration_date: hire_expiration_date.max(0) as u32,
            details,
        };

        Some(NetworkEvent::IventoryItemAdded { item })
    })?;
    packet_handler.register_noop::<SpiritsPacket2019>()?;
    packet_handler.register_noop::<Spirits2Packet2019>()?;
    packet_handler.register_noop::<UpdateAttackRangePacket>()?;
    packet_handler.register_noop::<NewMailStatusPacket>()?;
    packet_handler.register_noop::<AchievementUpdatePacket>()?;
    packet_handler.register_noop::<AchievementListPacket>()?;
    packet_handler.register_noop::<CriticalWeightUpdatePacket>()?;
    packet_handler.register(|packet: SpriteChangePacket| match packet.sprite_type {
        SpriteChangeType::Base => Some(NetworkEvent::ChangeJob {
            account_id: packet.account_id,
            job_id: JobId(packet.value as u16),
        }),
        SpriteChangeType::Hair => Some(NetworkEvent::ChangeHair {
            account_id: packet.account_id,
            hair_id: packet.value,
        }),
        _ => None,
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |_: InventoyStartPacket| {
            *inventory_items.borrow_mut() = Some(Vec::new());
            NoNetworkEvents
        }
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: RegularItemListPacket| {
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item_information| {
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
                        index,
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
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item| {
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
    // CondaRO runs a 2019 rAthena server (PACKETVER 20190605) which sends the
    // equippable inventory rows with the 2019 `ZC_INVENTORY_LIST` header and
    // layout (0x0B0A, 67 byte rows). The 2022 header (0x0B39) is never used by
    // that server, and an unhandled packet would abort parsing for the rest of
    // the current TCP read - which is exactly where the map-load entity spawn
    // burst arrives, so the NPC list used to get swallowed here.
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |packet: EquippableItemListPacket2019| {
            inventory_items
                .borrow_mut()
                .as_mut()
                .expect("Unexpected inventory packet")
                .extend(packet.item_information.into_iter().map(|item| {
                    let item = EquippableItemInformation::from(item);

                    InventoryItem {
                        index: item.index,
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
    packet_handler.register({
        let inventory_items = inventory_items.clone();

        move |_: InventoyEndPacket| {
            let items = inventory_items.borrow_mut().take().expect("Unexpected inventory end packet");
            NetworkEvent::SetInventory { items }
        }
    })?;
    packet_handler.register_noop::<EquippableSwitchItemListPacket>()?;
    packet_handler.register_noop::<MapTypePacket>()?;
    packet_handler.register(|packet: SkillInfoUpdatePacket| {
        let SkillInfoUpdatePacket {
            skill_id,
            skill_type,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        } = packet;
        NetworkEvent::UpdateSkillInfo {
            skill_id,
            skill_type,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable: upgradable != 0,
        }
    })?;
    packet_handler.register(|packet: AddSkillPacket| {
        let AddSkillPacket { skill_information } = packet;
        NetworkEvent::AddSkill { skill_information }
    })?;
    packet_handler.register(|packet: UpdateSkillTreePacket| {
        let UpdateSkillTreePacket { skill_information } = packet;
        NetworkEvent::SkillTree { skill_information }
    })?;
    packet_handler.register(|packet: UpdateHotkeysPacket| NetworkEvent::SetHotkeyData {
        tab: packet.tab,
        hotkeys: packet
            .hotkeys
            .into_iter()
            .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                true => HotkeyState::Unbound,
                false => HotkeyState::Bound(hotkey_data),
            })
            .collect(),
    })?;
    packet_handler.register(|packet: InitialStatsPacket| {
        let InitialStatsPacket {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
            left_attack,
            rigth_attack,
            rigth_magic_attack,
            left_magic_attack,
            left_defense,
            rigth_defense,
            rigth_magic_defense,
            left_magic_defense,
            hit,
            flee,
            flee2,
            crit,
            attack_speed,
            ..
        } = packet;

        // The packet field names don't reflect the meaning of the values, they
        // were named after the order in which they appear in the packet. The
        // real meaning (from rAthena) is:
        // - left_attack         : attack including weapon attack
        // - rigth_attack        : weapon refine attack bonus
        // - rigth_magic_attack  : maximum magic attack
        // - left_magic_attack   : minimum magic attack
        // - left_defense        : equipment defense
        // - rigth_defense       : vitality based defense
        // - rigth_magic_defense : equipment magic defense
        // - left_magic_defense  : intelligence based magic defense
        NetworkEvent::InitialStats {
            strength_stat_points_cost,
            agility_stat_points_cost,
            vitality_stat_points_cost,
            intelligence_stat_points_cost,
            dexterity_stat_points_cost,
            luck_stat_points_cost,
            attack: left_attack,
            bonus_attack: rigth_attack,
            magic_attack_max: rigth_magic_attack,
            magic_attack_min: left_magic_attack,
            defense: left_defense,
            bonus_defense: rigth_defense,
            magic_defense: rigth_magic_defense,
            bonus_magic_defense: left_magic_defense,
            hit,
            flee,
            perfect_dodge: flee2,
            critical: crit,
            attack_speed,
        }
    })?;
    packet_handler.register_noop::<UpdatePartyInvitationStatePacket>()?;
    packet_handler.register_noop::<UpdateShowEquipPacket>()?;
    packet_handler.register_noop::<UpdateConfigurationPacket>()?;
    packet_handler.register_noop::<NavigateToMonsterPacket>()?;
    packet_handler.register_noop::<MarkMinimapPositionPacket>()?;
    packet_handler.register(|packet: NextButtonPacket| {
        let NextButtonPacket { npc_id } = packet;

        NetworkEvent::AddNextButton { npc_id }
    })?;
    packet_handler.register(|packet: CloseButtonPacket| {
        let CloseButtonPacket { npc_id } = packet;

        NetworkEvent::AddCloseButton { npc_id }
    })?;
    packet_handler.register(|packet: DialogMenuPacket| {
        let DialogMenuPacket { npc_id, message } = packet;

        let choices = message.split(':').map(String::from).filter(|text| !text.is_empty()).collect();

        NetworkEvent::AddChoiceButtons { choices, npc_id }
    })?;
    packet_handler.register_noop::<DisplaySpecialEffectPacket>()?;
    packet_handler.register_noop::<DisplaySkillCooldownPacket>()?;
    packet_handler.register(|packet: DisplaySkillEffectAndDamagePacket| {
        // Reuse the regular damage event so skill damage shows damage numbers
        // and plays the hit animation just like auto attacks.
        NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage > 0).then_some(packet.damage as usize),
            attack_duration: packet.destination_delay,
            is_critical: false,
        }
    })?;
    packet_handler.register(|packet: DisplaySkillEffectNoDamagePacket| {
        // 2019 `ZC_USE_SKILL` (0x09CB): "skill used without damage". This is
        // what the server sends for instant/complete casts such as self-buffs
        // and crafting (the ambient plaza actors use it constantly). The client
        // used to only show a heal number for the destination.
        let DisplaySkillEffectNoDamagePacket {
            skill_id,
            heal_amount,
            destination_entity_id,
            source_entity_id,
            result,
        } = packet;

        NetworkEvent::SkillNoDamage {
            source_entity_id,
            destination_entity_id,
            skill_id,
            level: heal_amount,
            success: result != 0,
        }
    })?;
    packet_handler.register_noop::<DisplayPlayerHealEffect>()?;
    packet_handler.register_noop::<StatusChangePacket>()?;
    packet_handler.register_noop::<QuestNotificationPacket1>()?;
    packet_handler.register_noop::<HuntingQuestNotificationPacket>()?;
    packet_handler.register_noop::<HuntingQuestUpdateObjectivePacket>()?;
    packet_handler.register_noop::<QuestRemovedPacket>()?;
    packet_handler.register_noop::<QuestListPacket>()?;
    packet_handler.register(|packet: VisualEffectPacket| {
        let VisualEffectPacket { entity_id, effect } = packet;

        let effect_path = match effect {
            VisualEffect::BaseLevelUp => "angel.str",
            VisualEffect::JobLevelUp => "joblvup.str",
            VisualEffect::RefineFailure => "bs_refinefailed.str",
            VisualEffect::RefineSuccess => "bs_refinesuccess.str",
            VisualEffect::GameOver => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::PharmacySuccess => "p_success.str",
            VisualEffect::PharmacyFailure => "p_failed.str",
            VisualEffect::BaseLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::JobLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
            VisualEffect::BaseLevelUpTaekwon => "help_angel\\help_angel\\help_angel.str",
        };

        NetworkEvent::VisualEffect { effect_path, entity_id }
    })?;
    packet_handler.register_noop::<DisplayGainedExperiencePacket>()?;
    packet_handler.register_noop::<DisplayImagePacket>()?;
    packet_handler.register_noop::<StateChangePacket>()?;

    packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
        QuestEffect::None => NetworkEvent::RemoveQuestEffect {
            entity_id: packet.entity_id,
        },
        _ => NetworkEvent::AddQuestEffect { quest_effect: packet },
    })?;
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
    packet_handler.register(|packet: ServerTickPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RequestPlayerDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: EntityId(packet.character_id.0),
        name: packet.name,
        // The server sends an empty guild name for players that are not in a
        // guild, which has to clear a previously known guild name.
        guild_name: present_guild_name(packet.guild_name),
    })?;
    packet_handler.register(|packet: RequestEntityDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: packet.entity_id,
        name: packet.name,
        // The NPC details response does not contain a guild name.
        guild_name: None,
    })?;

    packet_handler.register(|packet: GuildEmblemImagePacket| NetworkEvent::GuildEmblemImage {
        guild_id: packet.guild_id,
        emblem_id: packet.emblem_id,
        emblem_data: packet.emblem_data,
    })?;
    packet_handler.register(|packet: UpdateEntityHealthPointsPacket| {
        let UpdateEntityHealthPointsPacket {
            entity_id,
            health_points,
            maximum_health_points,
        } = packet;

        NetworkEvent::UpdateEntityHealth {
            entity_id,
            health_points: health_points as usize,
            maximum_health_points: maximum_health_points as usize,
        }
    })?;
    packet_handler.register(|packet: RequestPlayerAttackFailedPacket| {
        let RequestPlayerAttackFailedPacket {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        } = packet;

        NetworkEvent::AttackFailed {
            target_entity_id,
            target_position,
            player_position,
            attack_range,
        }
    })?;
    packet_handler.register(|packet: DamagePacket1| match packet.damage_type {
        DamageType::Damage => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
        }),
        DamageType::PickUpItem => Some(NetworkEvent::EntityPickUpItem {
            entity_id: packet.source_entity_id,
            item_entity_id: packet.destination_entity_id,
        }),
        // `clif_sitting`/`clif_standing` only fill in the source id; the target id
        // of the packet stays 0, so the sitting entity is the source.
        DamageType::SitDown => Some(NetworkEvent::PlayerSitDown {
            entity_id: packet.source_entity_id,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.source_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: DamagePacket3| match packet.damage_type {
        DamageType::Damage => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: false,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
        }),
        DamageType::PickUpItem => Some(NetworkEvent::EntityPickUpItem {
            entity_id: packet.source_entity_id,
            item_entity_id: packet.destination_entity_id,
        }),
        // `clif_sitting`/`clif_standing` only fill in the source id; the target id
        // of the packet stays 0, so the sitting entity is the source.
        DamageType::SitDown => Some(NetworkEvent::PlayerSitDown {
            entity_id: packet.source_entity_id,
        }),
        DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
            entity_id: packet.source_entity_id,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        NetworkEvent::OpenDialog { text, npc_id }
    })?;
    packet_handler.register(|packet: RequestEquipItemStatusPacket| match packet.result {
        RequestEquipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: packet.equipped_position,
        }),
        _ => None,
    })?;
    packet_handler.register(|packet: RequestUnequipItemStatusPacket| match packet.result {
        RequestUnequipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
            index: packet.inventory_index,
            equipped_position: EquipPosition::NONE,
        }),
        _ => None,
    })?;
    packet_handler.register_noop::<Packet8302>()?;
    packet_handler.register_noop::<Packet0b18>()?;
    packet_handler.register_noop::<ConnectionRefusedPacket>()?;
    packet_handler.register(|packet: MapServerLoginSuccessPacket| NetworkEvent::UpdateClientTick {
        client_tick: packet.client_tick,
        received_at: Instant::now(),
    })?;
    packet_handler.register(|packet: RestartResponsePacket| match packet.result {
        RestartResponseStatus::Ok => NetworkEvent::LoggedOut,
        RestartResponseStatus::Nothing => NetworkEvent::ChatMessage {
            text: "Failed to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: DisconnectResponsePacket| match packet.result {
        DisconnectResponseStatus::Ok => NetworkEvent::LoggedOut,
        DisconnectResponseStatus::Wait10Seconds => NetworkEvent::ChatMessage {
            text: "Please wait 10 seconds before trying to log out.".to_string(),
            color: MessageColor::Error,
        },
    })?;
    packet_handler.register(|packet: UseSkillAckPacket2019| {
        let UseSkillAckPacket2019 {
            source_entity_id,
            destination_entity_id,
            position,
            skill_id,
            element,
            delay_time,
            disposable: _,
            attack_motion_time,
        } = packet;

        NetworkEvent::SkillCast {
            source_entity_id,
            destination_entity_id,
            position,
            skill_id,
            element,
            cast_time: delay_time,
            attack_motion_time,
        }
    })?;
    packet_handler.register_noop::<UseSkillSuccessPacket>()?;
    packet_handler.register_noop::<ToUseSkillSuccessPacket>()?;
    packet_handler.register(|packet: NotifySkillUnitPacket| {
        let NotifySkillUnitPacket {
            entity_id,
            position,
            unit_id,
            ..
        } = packet;

        NetworkEvent::AddSkillUnit {
            entity_id,
            unit_id,
            position,
        }
    })?;
    packet_handler.register(|packet: SkillUnitDisappearPacket| {
        let SkillUnitDisappearPacket { entity_id } = packet;
        NetworkEvent::RemoveSkillUnit { entity_id }
    })?;
    packet_handler.register(|packet: NotifyGroundSkillPacket| {
        let NotifyGroundSkillPacket {
            skill_id,
            entity_id,
            level,
            position,
            start_time: _,
        } = packet;

        NetworkEvent::SkillGroundCast {
            entity_id,
            skill_id,
            level: level.0,
            position,
        }
    })?;
    // CondaRO runs a 2019 rAthena server (PACKETVER 20190605). Since PACKETVER
    // 20180307 rAthena sends the friend list without the 24-byte friend name
    // (8-byte entries instead of the 32-byte ones the 2022-era
    // `FriendListPacket` parses). Using the wrong entry size makes the parser
    // read past the end of the packet, which desynchronizes the map server
    // stream and swallows the following `ZC_NPCACK_MAPMOVE` (0x0091) packet, so
    // characters with at least one friend ended up on a black screen. Friend
    // names are delivered later by the `ZC_FRIENDS_STATE` (0x0206) packet.
    packet_handler.register(|packet: FriendListPacket2019| NetworkEvent::SetFriendList {
        friend_list: packet
            .friend_list
            .into_iter()
            .map(|friend| Friend {
                account_id: friend.account_id,
                character_id: friend.character_id,
                name: String::new(),
            })
            .collect(),
    })?;
    packet_handler.register(|packet: FriendOnlineStatusPacket| NetworkEvent::FriendOnlineState {
        account_id: packet.account_id,
        character_id: packet.character_id,
        online: matches!(packet.state, OnlineState::Online),
        name: packet.name,
    })?;
    packet_handler.register(|packet: FriendRequestPacket| NetworkEvent::FriendRequest {
        requestee: packet.requestee,
    })?;
    packet_handler.register(|packet: FriendRequestResultPacket| {
        let text = match packet.result {
            FriendRequestResult::Accepted => format!("You have become friends with {}.", packet.friend.name),
            FriendRequestResult::Rejected => format!("{} does not want to be friends with you.", packet.friend.name),
            FriendRequestResult::OwnFriendListFull => "Your Friend List is full.".to_owned(),
            FriendRequestResult::OtherFriendListFull => format!("{}'s Friend List is full.", packet.friend.name),
        };

        let mut events = vec![NetworkEvent::ChatMessage {
            text,
            color: MessageColor::Information,
        }];

        if matches!(packet.result, FriendRequestResult::Accepted) {
            events.push(NetworkEvent::FriendAdded { friend: packet.friend });
        }

        events
    })?;
    packet_handler.register(|packet: NotifyFriendRemovedPacket| NetworkEvent::FriendRemoved {
        account_id: packet.account_id,
        character_id: packet.character_id,
    })?;
    packet_handler.register_noop::<PartyInvitePacket>()?;
    packet_handler.register_noop::<StatusChangeSequencePacket>()?;
    packet_handler.register_noop::<ReputationPacket>()?;
    packet_handler.register_noop::<ClanInfoPacket>()?;
    packet_handler.register_noop::<ClanOnlineCountPacket>()?;
    packet_handler.register_noop::<ChangeMapCellPacket>()?;
    packet_handler.register_noop::<OpenMarketPacket>()?;
    packet_handler.register(|packet: BuyOrSellPacket| NetworkEvent::AskBuyOrSell { shop_id: packet.shop_id })?;
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
    // 0x00B1 ZC_LONGPAR_CHANGE carries 32 bit values (zeny and the displayed
    // EXP values). Zeny has to update the player state, the other values are
    // forwarded as well so the EXP bars follow the server instead of going stale.
    packet_handler.register(|packet: ParameterChangePacket| {
        let stat_type = match packet.variable_id {
            1 => StatType::BaseExperience(packet.value as u64),
            2 => StatType::JobExperience(packet.value as u64),
            20 => StatType::Zeny(packet.value),
            22 => StatType::NextBaseExperience(packet.value as u64),
            23 => StatType::NextJobExperience(packet.value as u64),
            _ => return Vec::new(),
        };

        vec![NetworkEvent::UpdateStat { stat_type }]
    })?;

    packet_handler.register(|packet: SellListPacket| NetworkEvent::SellItemList { items: packet.items })?;
    packet_handler.register(|packet: SellItemsResultPacket| NetworkEvent::SellingCompleted { result: packet.result })?;
    packet_handler.register_noop::<RequestStatUpResponsePacket>()?;
    packet_handler.register_noop::<EquipAmmunitionPacket>()?;
    packet_handler.register_noop::<AmmunitionActionPacket>()?;
    packet_handler.register(|packet: UpdateSkillPacket| {
        let UpdateSkillPacket {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable,
        } = packet;

        NetworkEvent::UpdateSkill {
            skill_id,
            skill_level,
            spell_point_cost,
            attack_range,
            upgradable: upgradable != 0,
        }
    })?;
    packet_handler.register(|packet: RemoveSkillPacket| NetworkEvent::RemoveSkill { skill_id: packet.skill_id })?;

    Ok(())
}

/// Returns the guild name if the server actually sent one.
///
/// rAthena uses empty fixed-length strings for "not in a guild", so an empty
/// (or whitespace only) name means the player is not in a guild.
fn present_guild_name(guild_name: String) -> Option<String> {
    let guild_name = guild_name.trim();

    (!guild_name.is_empty()).then(|| guild_name.to_owned())
}
