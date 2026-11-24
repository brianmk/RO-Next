use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use korangar_messages::{MessageDomain, format_message};
use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;

use crate::event::{NetworkEventList, NoNetworkEvents};
use crate::items::ItemQuantity;
use crate::packet_versions::noop_policy::register_ignored_noop;
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
        let reason = match packet.reason {
            LoginFailedReason::ServerClosed => UnifiedLoginFailedReason::ServerClosed,
            LoginFailedReason::AlreadyLoggedIn => UnifiedLoginFailedReason::AlreadyLoggedIn,
            LoginFailedReason::AlreadyOnline => UnifiedLoginFailedReason::AlreadyOnline,
        };
        let message = format_message(MessageDomain::LoginFailed, login_failed_reason_code(&packet.reason), &[]);

        NetworkEvent::LoginServerConnectionFailed { reason, message }
    })?;
    packet_handler.register(|packet: LoginFailedPacket2| {
        let reason = match packet.reason {
            LoginFailedReason2::UnregisteredId => UnifiedLoginFailedReason::UnregisteredId,
            LoginFailedReason2::IncorrectPassword => UnifiedLoginFailedReason::IncorrectPassword,
            LoginFailedReason2::IdExpired => UnifiedLoginFailedReason::IdExpired,
            LoginFailedReason2::RejectedFromServer => UnifiedLoginFailedReason::RejectedFromServer,
            LoginFailedReason2::BlockedByGMTeam => UnifiedLoginFailedReason::BlockedByGMTeam,
            LoginFailedReason2::GameOutdated => UnifiedLoginFailedReason::GameOutdated,
            LoginFailedReason2::LoginProhibitedUntil => UnifiedLoginFailedReason::LoginProhibitedUntil,
            LoginFailedReason2::ServerFull => UnifiedLoginFailedReason::ServerFull,
            LoginFailedReason2::CompanyAccountLimitReached => UnifiedLoginFailedReason::CompanyAccountLimitReached,
            LoginFailedReason2::BannedByDBATeam => UnifiedLoginFailedReason::BannedByDBATeam,
            LoginFailedReason2::UnconfirmedEmail => UnifiedLoginFailedReason::UnconfirmedEmail,
            LoginFailedReason2::BannedByGMTeam => UnifiedLoginFailedReason::BannedByGMTeam,
            LoginFailedReason2::TemporaryBanForDatabaseWork => UnifiedLoginFailedReason::TemporaryBanForDatabaseWork,
            LoginFailedReason2::SelfLocked => UnifiedLoginFailedReason::SelfLocked,
            LoginFailedReason2::NotPermittedGroup => UnifiedLoginFailedReason::NotPermittedGroup,
            LoginFailedReason2::AccountIdErased => UnifiedLoginFailedReason::AccountIdErased,
            LoginFailedReason2::LoginInformationRemains => UnifiedLoginFailedReason::LoginInformationRemains,
            LoginFailedReason2::LockedForHackingInvestigation => UnifiedLoginFailedReason::LockedForHackingInvestigation,
            LoginFailedReason2::TemporaryLockedForBugInvestigation => UnifiedLoginFailedReason::TemporaryLockedForBugInvestigation,
            LoginFailedReason2::DeletingCharacter => UnifiedLoginFailedReason::DeletingCharacter,
            LoginFailedReason2::DeletingSpouseCharacter => UnifiedLoginFailedReason::DeletingSpouseCharacter,
            LoginFailedReason2::UnknownError => UnifiedLoginFailedReason::UnknownError,
        };
        // Only the result codes are mapped here; the user-facing texts live in
        // `data\client\messages.ron`. The date argument is only used by the two
        // "prohibited until"/"information remains" rows.
        let message = format_message(MessageDomain::LoginFailed2, login_failed_reason2_code(&packet.reason), &[(
            "date",
            &packet.date,
        )]);

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
        let message = format_message(MessageDomain::LoginFailed, login_failed_reason_code(&reason), &[]);

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
    register_ignored_noop::<_, CharacterInfoPerPagePacket>(packet_handler)?;
    register_ignored_noop::<_, CharacterSlotPagePacket>(packet_handler)?;
    register_ignored_noop::<_, CharacterBanListPacket>(packet_handler)?;
    register_ignored_noop::<_, LoginPincodePacket>(packet_handler)?;
    register_ignored_noop::<_, Packet0b18>(packet_handler)?;
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
        // rAthena `chclif_refuse_delchar` error codes: 0 = incorrect e-mail
        // address (also used for database/delete-time errors), 1 = invalid slot,
        // 2 = character is in a party or guild.
        let message = match reason {
            CharacterDeletionFailedReason::NotAllowed => {
                "The account e-mail address is incorrect (or the character cannot be deleted yet)."
            }
            CharacterDeletionFailedReason::CharacterNotFound => "The character could not be found.",
            CharacterDeletionFailedReason::NotEligible => "The character is in a party or guild and cannot be deleted.",
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

/// Converts the 2019 `ZC_NOTIFY_SKILL` (0x01DE) into the regular damage event.
///
/// The packet carries the skill id (`SKID`) and the server start tick of the
/// skill use, so the world layer can attribute skill damage (and play the
/// mapped completion effect once per cast) instead of treating it like an auto
/// attack.
fn skill_damage_event(packet: DisplaySkillEffectAndDamagePacket) -> NetworkEvent {
    NetworkEvent::DamageEffect {
        source_entity_id: packet.source_entity_id,
        destination_entity_id: packet.destination_entity_id,
        damage_amount: (packet.damage > 0).then_some(packet.damage as usize),
        attack_duration: packet.destination_delay,
        is_critical: false,
        skill_id: (packet.skill_id.0 != 0).then_some(packet.skill_id),
        skill_start_tick: Some(packet.start_time),
    }
}

pub fn register_map_server_packets<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    /// Inventory type byte of the shared list packets that belongs to the
    /// normal player inventory.
    const INVENTORY_TYPE_INVENTORY: u8 = 0;
    /// Inventory type byte of the Kafra storage list.
    const INVENTORY_TYPE_STORAGE: u8 = 2;
    /// Inventory type byte of the merchant pushcart list (`INVTYPE_CART` in
    /// rAthena `clif_cartlist`, `clif.cpp:3226`).
    const INVENTORY_TYPE_CART: u8 = 1;

    // This is a bit of a workaround for the way that the inventory is
    // sent. There is a single packet to start the inventory list,
    // followed by an arbitary number of item packets, and in the
    // end a sinle packet to mark the list as complete.
    //
    // This variable provides some transient storage shared by all the inventory
    // handlers.
    let inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));
    // The same list packets are used for the Kafra storage; they are told apart
    // by the inventory type byte (0 = inventory, 2 = storage).
    let storage_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));
    let storage_name: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    // The pushcart uses the same family with `inventory_type` 1.
    let cart_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));

    packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
    packet_handler.register(|packet: SoundPacket| NetworkEvent::PlaySound {
        name: packet.name,
        act: packet.act,
        term: packet.term,
        account_id: packet.account_id,
    })?;
    // Broadcasts are both a chat line and the top announce box of the
    // notification HUD, so the handler emits both events.
    packet_handler.register(|packet: BroadcastMessagePacket| {
        vec![
            NetworkEvent::ChatMessage {
                text: packet.message.clone(),
                color: MessageColor::Broadcast,
            },
            NetworkEvent::ServerAnnouncement { text: packet.message },
        ]
    })?;
    packet_handler.register(|packet: Broadcast2MessagePacket| {
        // Drop the alpha channel because it might be 0.
        let color = MessageColor::Rgb {
            red: packet.font_color.red,
            green: packet.font_color.green,
            blue: packet.font_color.blue,
        };
        vec![
            NetworkEvent::ChatMessage {
                text: packet.message.clone(),
                color,
            },
            NetworkEvent::ServerAnnouncement { text: packet.message },
        ]
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
    packet_handler.register(|packet: WhisperReceivedPacket| NetworkEvent::WhisperReceived {
        sender_name: packet.sender_name,
        message: packet.message.trim_end_matches('\0').to_owned(),
        is_admin: packet.is_admin != 0,
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
    // `ZC_MSG` carries only a msgstringtable id; the client resolves it with
    // `data/msgstringtable.txt` (see korangar/src/msgstringtable.rs).
    packet_handler.register(|packet: MessageTablePacket| NetworkEvent::MessageString {
        message_id: packet.message_id,
    })?;
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
    packet_handler.register(|packet: OpenStorePacket| NetworkEvent::OpenStore { slots: packet.slots })?;
    packet_handler.register(|packet: EntityStopMovePacket| NetworkEvent::EntityStopped {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;
    packet_handler.register(|packet: HighJumpPacket| NetworkEvent::EntityHighJumped {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;
    packet_handler.register(|packet: SnapPacket| NetworkEvent::EntitySnapped {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;
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
    // `ZC_CHANGE_DIRECTION` (0x009C): sent for every entity that turns and for
    // the player themselves on every map load. Parsing only is not enough, the
    // facing has to be applied to the entity.
    packet_handler.register(|packet: ChangeDirectionPacket2019| {
        let ChangeDirectionPacket2019 {
            entity_id,
            head_direction,
            direction,
        } = packet;

        // The wire direction uses the same packed encoding as the position
        // nibble in the spawn packets, so mirror `WorldPosition::from_bytes`.
        let direction = (8u16.wrapping_sub(u16::from(direction)) + 4) & 7;

        NetworkEvent::ChangeDirection {
            entity_id,
            head_direction: head_direction as usize,
            direction: Direction::from(direction),
        }
    })?;
    register_ignored_noop::<_, NotifyActorInitPacket>(packet_handler)?;
    // Remaining frequent 2019 packets that the client does not use yet. They are
    // parsed with their exact 2019 sizes; a real handler can replace the noop
    // later (shop: handled by the NPC shop branch, status effects: buff UI).
    packet_handler.register(|packet: CartItemCountInfoPacket2019| NetworkEvent::CartInfo {
        current_count: packet.current_count,
        maximum_count: packet.maximum_count,
        current_weight: packet.current_weight,
        maximum_weight: packet.maximum_weight,
    })?;
    packet_handler.register(|packet: UserCountPacket2019| NetworkEvent::UserCount {
        player_count: packet.player_count,
    })?;
    packet_handler.register(|packet: RefuseEnterPacket2019| NetworkEvent::RefuseEnter {
        error_code: packet.error_code,
    })?;
    // The map server sends `SC_NOTIFY_BAN` (0x0081) when it kicks the session
    // (for example "ID already logged in" after a stale connection). Without a
    // handler the 3-byte packet desynchronizes the stream and the disconnect
    // happens silently.
    packet_handler.register(|packet: NotifyBanPacket| NetworkEvent::MapServerBan {
        error_code: packet.error_code,
    })?;
    packet_handler.register(|packet: PartyMemberHealthPacket2019| NetworkEvent::PartyMemberHealth {
        account_id: packet.account_id,
        health_points: packet.health_points,
        maximum_health_points: packet.maximum_health_points,
    })?;
    register_ignored_noop::<_, ExchangeUndoPacket2019>(packet_handler)?;
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
    packet_handler.register(|packet: ExchangeRequestPacket2019| NetworkEvent::TradeRequest {
        requester_name: packet.requester_name,
        target_account_id: packet.target_account_id,
        target_level: packet.target_level,
    })?;
    packet_handler.register(|packet: ExchangeOfferPacket2019| NetworkEvent::TradeItemOffered {
        item_id: packet.item_id,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: ExchangeAddItemPacket2019| NetworkEvent::TradeAddResult {
        inventory_index: packet.inventory_index,
        result: packet.result,
    })?;
    packet_handler.register(|packet: ExchangeAckPacket2019| NetworkEvent::TradeAck {
        result: packet.result,
        target_account_id: packet.target_account_id,
    })?;
    packet_handler.register(|_: ExchangeCancelPacket2019| NetworkEvent::TradeCancelled)?;
    packet_handler.register(|packet: ExchangeConcludePacket2019| NetworkEvent::TradeConcluded { who: packet.who })?;
    packet_handler.register(|packet: ExchangeExecPacket2019| NetworkEvent::TradeExecuted { result: packet.result })?;
    // 2019 era `ZC_ITEM_PICKUP_ACK` (0x0A37). The server uses this packet both for
    // picking items up from the ground and for items that are added to the
    // inventory by a shop purchase, so the item is added to the inventory state
    // here (it used to be a no-op registration, which silently dropped bought
    // items).
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
    packet_handler.register(|packet: SpiritsPacket2019| NetworkEvent::SpiritOrbs {
        entity_id: packet.entity_id,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: Spirits2Packet2019| NetworkEvent::SpiritOrbs {
        entity_id: packet.entity_id,
        amount: packet.amount,
    })?;
    packet_handler.register(|packet: UpdateAttackRangePacket| NetworkEvent::AttackRangeChanged {
        attack_range: packet.attack_range,
    })?;
    register_ignored_noop::<_, AchievementUpdatePacket>(packet_handler)?;
    register_ignored_noop::<_, AchievementListPacket>(packet_handler)?;
    packet_handler.register(|packet: SpriteChangePacket| match packet.sprite_type {
        SpriteChangeType::Base => Some(NetworkEvent::ChangeJob {
            account_id: packet.account_id,
            job_id: JobId(packet.value as u16),
        }),
        SpriteChangeType::Hair => Some(NetworkEvent::ChangeHair {
            account_id: packet.account_id,
            hair_id: packet.value,
        }),
        SpriteChangeType::HeadTop | SpriteChangeType::HeadMiddle | SpriteChangeType::HeadBottom => Some(NetworkEvent::AppearanceChanged {
            account_id: packet.account_id,
            sprite_type: packet.sprite_type,
            value: packet.value,
        }),
        SpriteChangeType::HairCollor | SpriteChangeType::ClothesColor => Some(NetworkEvent::AppearanceChanged {
            account_id: packet.account_id,
            sprite_type: packet.sprite_type,
            value: packet.value,
        }),
        // Equipment changes: the server sends the new weapon type (`W_*`) /
        // shield / robe look after equipping or unequipping. The entity layer
        // routes the weapon value through the data-driven weapon registry.
        SpriteChangeType::Weapon | SpriteChangeType::Shield | SpriteChangeType::Robe => Some(NetworkEvent::AppearanceChanged {
            account_id: packet.account_id,
            sprite_type: packet.sprite_type,
            value: packet.value,
        }),
        _ => None,
    })?;
    packet_handler.register({
        let inventory_items = inventory_items.clone();
        let storage_items = storage_items.clone();
        let storage_name = storage_name.clone();
        let cart_items = cart_items.clone();

        move |packet: InventoyStartPacket| {
            match packet.inventory_type {
                INVENTORY_TYPE_INVENTORY => *inventory_items.borrow_mut() = Some(Vec::new()),
                INVENTORY_TYPE_STORAGE => {
                    *storage_items.borrow_mut() = Some(Vec::new());
                    *storage_name.borrow_mut() = packet.inventory_name.clone();
                }
                INVENTORY_TYPE_CART => *cart_items.borrow_mut() = Some(Vec::new()),
                // Guild storage lists are not displayed yet. Their packets
                // still have to be consumed so the stream stays in sync.
                _ => {}
            }

            NoNetworkEvents
        }
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
    // CondaRO runs a 2019 rAthena server (PACKETVER 20190605) which sends the
    // equippable inventory rows with the 2019 `ZC_INVENTORY_LIST` header and
    // layout (0x0B0A, 67 byte rows). The 2022 header (0x0B39) is never used by
    // that server, and an unhandled packet would abort parsing for the rest of
    // the current TCP read - which is exactly where the map-load entity spawn
    // burst arrives, so the NPC list used to get swallowed here.
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
    packet_handler.register({
        let inventory_items = inventory_items.clone();
        let storage_items = storage_items.clone();
        let storage_name = storage_name.clone();
        let cart_items = cart_items.clone();

        move |packet: InventoyEndPacket| -> NetworkEventList {
            match packet.inventory_type {
                INVENTORY_TYPE_INVENTORY => {
                    let items = inventory_items.borrow_mut().take().unwrap_or_default();
                    NetworkEvent::SetInventory { items }.into()
                }
                INVENTORY_TYPE_STORAGE => {
                    let items = storage_items.borrow_mut().take().unwrap_or_default();
                    let name = storage_name.borrow().clone();
                    NetworkEvent::SetStorage { items, name }.into()
                }
                INVENTORY_TYPE_CART => {
                    let items = cart_items.borrow_mut().take().unwrap_or_default();
                    NetworkEvent::SetCart { items }.into()
                }
                _ => NetworkEventList::default(),
            }
        }
    })?;
    register_ignored_noop::<_, EquippableSwitchItemListPacket>(packet_handler)?;
    packet_handler.register(|packet: MapTypePacket| NetworkEvent::MapProperties {
        map_type: packet.map_type,
        flags: packet.flags,
    })?;
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
    // rAthena sends the *deny* flag here: 0 = invites allowed, 1 = blocked.
    packet_handler.register(|packet: UpdatePartyInvitationStatePacket| NetworkEvent::PartyConfig {
        allow_invites: packet.allowed == 0,
    })?;
    packet_handler.register(|packet: UpdateShowEquipPacket| NetworkEvent::ShowEquipmentWindow {
        open: packet.open_equip_window != 0,
    })?;
    register_ignored_noop::<_, UpdateConfigurationPacket>(packet_handler)?;
    packet_handler.register(|packet: MarkMinimapPositionPacket| NetworkEvent::MinimapMarker {
        npc_id: packet.npc_id,
        marker_type: packet.marker_type,
        position: packet.position,
        id: packet.id,
        color: packet.color,
    })?;
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
    packet_handler.register(|packet: DisplaySpecialEffectPacket| NetworkEvent::SpecialEffect {
        entity_id: packet.entity_id,
        effect_id: packet.effect_id,
    })?;
    packet_handler.register(|packet: DisplaySkillCooldownPacket| NetworkEvent::SkillCooldown {
        skill_id: packet.skill_id,
        until: packet.until,
    })?;
    packet_handler.register(|packet: DisplaySkillEffectAndDamagePacket| skill_damage_event(packet))?;
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
    // `ZC_RECOVERY` (0x0A27): the server sends this only to the player that was
    // healed and the packet contains no entity id, so the event carries no id and
    // the world layer resolves the local player.
    packet_handler.register(|packet: DisplayPlayerHealEffect| NetworkEvent::HealEffect {
        entity_id: None,
        heal_amount: packet.heal_amount as usize,
    })?;
    packet_handler.register(|packet: StatusChangePacket| NetworkEvent::StatusEffect {
        entity_id: packet.entity_id,
        status_index: packet.index,
        active: packet.state != 0,
        total_duration_in_milliseconds: packet.duration_in_milliseconds,
        remaining_in_milliseconds: packet.remaining_in_milliseconds,
        // The server fields are unsigned in the packet layout but carry signed
        // status values (e.g. elemental enchants).
        values: packet.value.map(|value| value as i32),
    })?;
    // Sent on map load / when an entity enters the view with active statuses.
    // Missing this handler used to discard the rest of the packet burst.
    packet_handler.register(|packet: EnterStatusEffectPacket2019| NetworkEvent::StatusEffect {
        entity_id: packet.entity_id,
        status_index: packet.effect_id,
        active: true,
        total_duration_in_milliseconds: packet.duration,
        remaining_in_milliseconds: packet.remaining_duration,
        values: [packet.value1, packet.value2, packet.value3],
    })?;
    packet_handler.register(|packet: QuestListPacket| NetworkEvent::QuestList { quests: packet.quests })?;
    // The same packet drives the quest log and the quest HUD banner. The
    // objective names give the banner a readable text even though the packet
    // does not carry the quest title.
    packet_handler.register(|packet: QuestNotificationPacket1| {
        let objective_names = packet
            .objective_details
            .iter()
            .map(|objective| objective.mob_name.trim_end_matches('\0').trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();

        vec![
            NetworkEvent::QuestActive {
                quest_id: packet.quest_id,
                active: packet.active != 0,
            },
            NetworkEvent::QuestBanner {
                quest_id: packet.quest_id,
                active: packet.active != 0,
                objective_names,
            },
        ]
    })?;
    packet_handler.register(|packet: QuestRemovedPacket| NetworkEvent::QuestRemoved { quest_id: packet.quest_id })?;
    register_ignored_noop::<_, HuntingQuestNotificationPacket>(packet_handler)?;
    register_ignored_noop::<_, HuntingQuestUpdateObjectivePacket>(packet_handler)?;
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
    packet_handler.register(|packet: DisplayGainedExperiencePacket| NetworkEvent::GainedExperience {
        amount: packet.amount as i64,
        experience_type: packet.experience_type,
        experience_source: packet.experience_source,
    })?;
    register_ignored_noop::<_, DisplayImagePacket>(packet_handler)?;
    // `ZC_STATE_CHANGE` (0x0229): live body/health/option/PK state updates
    // (sitting/standing changes are separate, they come as ZC_NOTIFY_ACT).
    packet_handler.register(|packet: StateChangePacket| NetworkEvent::StateChange {
        entity_id: packet.entity_id,
        body_state: packet.body_state,
        health_state: packet.health_state,
        effect_state: packet.effect_state,
        is_pk_mode_on: packet.is_pk_mode_on != 0,
    })?;

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
            skill_id: None,
            skill_start_tick: None,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
            skill_id: None,
            skill_start_tick: None,
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
            skill_id: None,
            skill_start_tick: None,
        }),
        DamageType::CriticalHit => Some(NetworkEvent::DamageEffect {
            source_entity_id: packet.source_entity_id,
            destination_entity_id: packet.destination_entity_id,
            damage_amount: (packet.damage_amount > 0).then_some(packet.damage_amount as usize),
            attack_duration: packet.attack_duration,
            is_critical: true,
            skill_id: None,
            skill_start_tick: None,
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
    packet_handler.register(|packet: OpenEditDialogPacket| NetworkEvent::OpenEditDialog {
        npc_id: packet.npc_id,
        string_input: false,
    })?;
    packet_handler.register(|packet: OpenEditDialogStrPacket| NetworkEvent::OpenEditDialog {
        npc_id: packet.npc_id,
        string_input: true,
    })?;
    packet_handler.register(|packet: NpcDialogPacket| {
        let NpcDialogPacket { npc_id, text } = packet;

        NetworkEvent::OpenDialog { text, npc_id }
    })?;
    packet_handler.register(|packet: RequestEquipItemStatusPacket| {
        let mut events = Vec::new();

        match packet.result {
            RequestEquipItemStatus::Success => {
                events.push(NetworkEvent::UpdateEquippedPosition {
                    index: packet.inventory_index,
                    equipped_position: packet.equipped_position,
                });

                // The 2019 server does not send a sprite change packet for headgear
                // equips, the acknowledgement contains the view id of the equipped
                // item instead (rAthena `clif_equipitemack` sends `wItemSpriteNumber`
                // for EQP_VISIBLE items). Apply it so the world sprite and the paper
                // doll show the new headgear right away.
                for sprite_type in head_gear_sprite_types(packet.equipped_position) {
                    events.push(NetworkEvent::LocalAppearanceChanged {
                        sprite_type,
                        value: u32::from(packet.view_id),
                    });
                }
            }
            RequestEquipItemStatus::FailedDueToLevelRequirement => events.push(NetworkEvent::ChatMessage {
                text: "Your level is too low to equip this item.".to_owned(),
                color: MessageColor::Error,
            }),
            RequestEquipItemStatus::Failed => events.push(NetworkEvent::ChatMessage {
                text: "You cannot equip this item (your class, job level or another requirement is not met).".to_owned(),
                color: MessageColor::Error,
            }),
        }

        events
    })?;
    packet_handler.register(|packet: RequestUnequipItemStatusPacket| {
        let mut events = Vec::new();

        match packet.result {
            RequestUnequipItemStatus::Success => {
                events.push(NetworkEvent::UpdateEquippedPosition {
                    index: packet.inventory_index,
                    equipped_position: EquipPosition::NONE,
                });

                // Unequipping does not send a sprite change either; clear the
                // headgear layer that was just taken off.
                for sprite_type in head_gear_sprite_types(packet.equipped_position) {
                    events.push(NetworkEvent::LocalAppearanceChanged { sprite_type, value: 0 });
                }
            }
            RequestUnequipItemStatus::Failed => events.push(NetworkEvent::ChatMessage {
                text: "You cannot unequip this item.".to_owned(),
                color: MessageColor::Error,
            }),
        }

        events
    })?;
    register_ignored_noop::<_, Packet0b18>(packet_handler)?;
    packet_handler.register(|packet: MapServerAuthFailedPacket| NetworkEvent::MapServerAuthFailed { reason: packet.reason })?;
    packet_handler.register(|packet: CriticalWeightUpdatePacket| NetworkEvent::CriticalWeight { percentage: packet.weight })?;
    // `ZC_AUTH_OK_GID` (0x0283) only re-confirms the player's block id that the
    // client already knows from login/character selection; parsed for framing.
    register_ignored_noop::<_, MapServerAuthOkPacket>(packet_handler)?;
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
    // `ZC_AUTORUN_SKILL` (0x0147): the server tells us that an item wants to use a
    // skill (e.g. the fly wing casts AL_TELEPORT). The official client then sends
    // the normal skill request, which is what actually casts the skill and
    // consumes the item, so the client must perform that step itself.
    packet_handler.register(|packet: AutoRunSkillPacket| NetworkEvent::AutoRunSkill {
        skill_id: packet.skill_id,
        skill_level: SkillLevel(packet.skill_level),
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
    // `ZC_DISPEL` (0x01B9) is sent by rAthena `clif_skillcastcancel` when a
    // timed cast is interrupted (movement, damage, another skill, ...). The
    // client used to ignore it, so a cancelled cast kept its cast bar and the
    // looping cast animation looked stuck.
    packet_handler.register(|packet: SkillCastCancelPacket| NetworkEvent::SkillCastCancelled {
        entity_id: packet.entity_id,
    })?;
    // `ZC_ACK_TOUSESKILL` is only sent when a skill cannot be used; `flag` is 0
    // for a failure in the 2019 protocol.
    packet_handler.register(|packet: ToUseSkillSuccessPacket| NetworkEvent::SkillUseFailed {
        skill_id: packet.skill_id,
        cause: packet.cause,
    })?;
    packet_handler.register(|packet: NotifySkillUnitPacket| {
        let NotifySkillUnitPacket {
            entity_id,
            position,
            unit_id,
            skill_level,
            ..
        } = packet;

        NetworkEvent::AddSkillUnit {
            entity_id,
            unit_id,
            position,
            skill_level,
        }
    })?;
    packet_handler.register(|packet: SkillUnitDisappearPacket| {
        let SkillUnitDisappearPacket { entity_id } = packet;
        NetworkEvent::RemoveSkillUnit { entity_id }
    })?;
    // `ZC_WARPLIST` (0x0ABE): the caster of a Warp Portal has to choose the
    // destination; the server creates the portal units only after the client
    // answers with `SelectWarpPointPacket`.
    packet_handler.register(|packet: WarpPointListPacket| NetworkEvent::WarpPointList {
        skill_id: packet.skill_id,
        maps: packet
            .maps
            .into_iter()
            .map(|entry| entry.map_name)
            .filter(|map_name| !map_name.trim().is_empty())
            .collect(),
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
    // Party packets sent during the map login burst of a character that is in
    // a party. They have to be registered with their exact 2019 layout:
    // an unknown fixed-size packet (e.g. ZC_PARTY_OPTION 0x07D8, 8 bytes)
    // makes the stream resync read bytes 2..4 as a packet length, fail, and
    // discard the rest of the TCP read. That discarded the NPC spawn burst and
    // the skill list that rAthena sends after the party packets, leaving party
    // characters without NPCs and skills.
    packet_handler.register(|packet: PartyOptionPacket2019| NetworkEvent::PartyOption {
        experience_option: packet.experience_option,
        item_pickup: packet.item_pickup,
        item_share: packet.item_share,
    })?;
    packet_handler.register(|packet: PartyListPacket2019| NetworkEvent::PartyList {
        party_name: packet.party_name,
        members: packet.members,
    })?;
    packet_handler.register(|packet: AddPartyMemberPacket2019| NetworkEvent::PartyMemberAdded {
        member: crate::PartyMemberDetails {
            account_id: packet.account_id,
            character_id: packet.character_id,
            name: packet.player_name,
            party_name: packet.party_name,
            map_name: packet.map_name,
            job_id: packet.job_id,
            base_level: packet.base_level,
            position: packet.position,
            online: packet.offline == 0,
            leader: packet.role == 0,
        },
    })?;
    packet_handler.register(|packet: PartyInvitePacket| NetworkEvent::PartyInvite {
        party_id: packet.party_id,
        party_name: packet.party_name,
    })?;
    packet_handler.register(|packet: PartyInviteReplyPacket2019| NetworkEvent::PartyInviteReply {
        character_name: packet.character_name,
        result: packet.result,
    })?;
    packet_handler.register(|packet: PartyCreationResultPacket| NetworkEvent::PartyCreationResult { result: packet.result })?;
    packet_handler.register(|packet: RemovePartyMemberPacket2019| NetworkEvent::PartyMemberRemoved {
        account_id: packet.account_id,
        character_name: packet.character_name,
        result: packet.result,
    })?;
    packet_handler.register(|packet: PartyMemberPositionPacket2019| NetworkEvent::PartyMemberPosition {
        account_id: packet.account_id,
        position: packet.position,
    })?;
    packet_handler.register(|packet: PartyChatMessagePacket2019| NetworkEvent::PartyChatMessage {
        account_id: packet.account_id,
        message: packet.message,
    })?;
    packet_handler.register(|packet: PartyMemberJobLevelPacket2019| NetworkEvent::PartyMemberJobLevel {
        account_id: packet.account_id,
        job_id: packet.job_id,
        base_level: packet.base_level,
    })?;
    packet_handler.register(|packet: PartyMemberAlivePacket2019| NetworkEvent::PartyMemberAlive {
        account_id: packet.account_id,
        is_dead: packet.is_dead != 0,
    })?;
    // Party booking (the 2019 "party search") is not used by this client yet;
    // the packets are registered so a booking broadcast cannot desynchronize
    // the map server stream.
    register_ignored_noop::<_, PartyBookingJoinRequestPacket2019>(packet_handler)?;
    register_ignored_noop::<_, PartyBookingJoinResultPacket2019>(packet_handler)?;
    // `ZC_MSG_STATE_CHANGE` (0x0196) is used by the 2019 server to end statuses
    // (and to start them on non-player units); it has no duration fields.
    packet_handler.register(|packet: StatusChangeSequencePacket| NetworkEvent::StatusEffect {
        entity_id: EntityId(packet.id),
        status_index: packet.index,
        active: packet.state != 0,
        total_duration_in_milliseconds: 0,
        remaining_in_milliseconds: 0,
        // This packet variant has no value fields.
        values: [0; 3],
    })?;
    register_ignored_noop::<_, ReputationPacket>(packet_handler)?;
    register_ignored_noop::<_, ClanInfoPacket>(packet_handler)?;
    register_ignored_noop::<_, ClanOnlineCountPacket>(packet_handler)?;
    packet_handler.register(|packet: ChangeMapCellPacket| NetworkEvent::MapCellChanged {
        position: packet.position,
        cell_type: packet.cell_type,
        map_name: packet.map_name,
    })?;
    register_ignored_noop::<_, OpenMarketPacket>(packet_handler)?;
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
    packet_handler.register(|packet: RequestStatUpResponsePacket| match packet.success {
        ragnarok_packets::RequestStatUpResult::Success => Vec::new(),
        ragnarok_packets::RequestStatUpResult::Failure => vec![NetworkEvent::StatUpFailed],
    })?;
    packet_handler.register(|packet: EquipAmmunitionPacket| NetworkEvent::AmmunitionEquipped {
        inventory_index: packet.inventory_index,
    })?;
    packet_handler.register(|packet: AmmunitionActionPacket| NetworkEvent::AmmunitionAction {
        action: packet.action_type,
    })?;
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
    // `ZC_GUILD_CHAT` (0x017F): guild chat message. rAthena sends it to every
    // connected guild member, including the sender (GUILD_NOBG only skips
    // battleground members), so no local echo is added.
    packet_handler.register(|packet: GuildChatReceivedPacket| NetworkEvent::GuildChat {
        message: packet.message.trim_end_matches('\0').to_owned(),
    })?;

    // Pushcart item updates (`ZC_ADD_ITEM_TO_CART` 0x0A0B,
    // `clif_cart_additem` at `clif.cpp:7273`). The full list arrives through the
    // shared inventory family handled above; these packets keep it in sync
    // between two full refreshes.
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

    // Vending shops of other players: the server announces the shop title
    // (`ZC_STORE_ENTRY` 0x0131, `clif.cpp:7558`) and answers a browse request
    // (0x0130) with the item list (`ZC_PC_PURCHASE_ITEMLIST_FROMMC` 0x0800,
    // `clif.cpp:7592`).
    packet_handler.register(|packet: StoreEntryPacket2019| NetworkEvent::StoreEntry {
        owner_account_id: packet.owner_account_id,
        shop_name: packet.store_name.trim().to_owned(),
    })?;
    packet_handler.register(|packet: StoreEntryRemovedPacket2019| NetworkEvent::StoreEntryRemoved {
        owner_account_id: packet.owner_account_id,
    })?;
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
    packet_handler.register(|packet: VendingPurchaseResultPacket2019| NetworkEvent::VendingPurchaseResult { result: packet.result })?;
    // --- Rodex mail (2019) ------------------------------------------------
    packet_handler.register(|packet: MailboxListPacket2019| NetworkEvent::MailList {
        mails: packet.entries.mails,
    })?;
    packet_handler.register(|packet: MailReadPacket2019| NetworkEvent::MailRead {
        mail_id: packet.mail_id,
        open_type: packet.open_type,
        body: packet.body,
        zeny: packet.zeny,
        items: packet.items.items,
    })?;
    packet_handler.register(|packet: MailZenyClaimResultPacket| NetworkEvent::MailAttachmentClaimed {
        mail_id: packet.mail_id,
        zeny: true,
        result: packet.result,
    })?;
    packet_handler.register(|packet: MailItemClaimResultPacket| NetworkEvent::MailAttachmentClaimed {
        mail_id: packet.mail_id,
        zeny: false,
        result: packet.result,
    })?;
    // `ZC_ACK_DELETE_MAIL` is only sent when the server actually deleted the
    // mail; failures are only logged server side.
    packet_handler.register(|packet: MailDeleteAckPacket| NetworkEvent::MailDeleted { mail_id: packet.mail_id })?;
    packet_handler.register(|packet: MailboxWindowPacket| NetworkEvent::MailboxWindow {
        open: packet.window_type == 0,
    })?;
    // `ZC_NOTIFY_UNREADMAIL` (0x09E7) was a noop before the mailbox existed; it
    // only carries the "there is unread mail" flag.
    packet_handler.register(|packet: NewMailStatusPacket| NetworkEvent::MailUnreadStatus {
        unread: packet.new_available != 0,
    })?;
    // Guild window (PACKETVER 20190605). The layouts and clif.cpp references are
    // documented on the packet structs in `ragnarok-packets`.
    packet_handler.register(|packet: GuildMenuInterfacePacket| NetworkEvent::GuildMenuInterface {
        menu_flags: packet.menu_flags,
    })?;
    packet_handler.register(|packet: GuildInfoPacket| NetworkEvent::GuildInfo { information: packet })?;
    packet_handler.register(|packet: GuildMemberManagerInfoPacket| NetworkEvent::GuildMembers { members: packet.members })?;
    packet_handler.register(|packet: GuildPositionNameInfoPacket| NetworkEvent::GuildPositionNames {
        positions: packet.positions,
    })?;
    packet_handler.register(|packet: GuildPositionInfoPacket| NetworkEvent::GuildPositionRights {
        positions: packet.positions,
    })?;
    packet_handler.register(|packet: GuildPositionChangedPacket| NetworkEvent::GuildPositionChanged {
        positions: packet.positions,
    })?;
    packet_handler
        .register(|packet: GuildMemberPositionChangedPacket| NetworkEvent::GuildMemberPositionChanged { members: packet.members })?;
    packet_handler.register(|packet: GuildNoticePacket| NetworkEvent::GuildNotice {
        subject: packet.subject,
        notice: packet.notice,
    })?;
    packet_handler.register(|packet: GuildAllianceInfoPacket| NetworkEvent::GuildAlliances {
        alliances: packet.alliances,
    })?;
    packet_handler.register(|packet: GuildMemberNamePacket| NetworkEvent::GuildMemberName {
        character_id: packet.character_id,
        name: packet.name,
    })?;
    packet_handler.register(|packet: GuildLeaveNoticePacket| NetworkEvent::GuildMemberRemoved {
        character_id: packet.character_id,
        expelled: false,
    })?;
    packet_handler.register(|packet: GuildBanNoticePacket| NetworkEvent::GuildMemberRemoved {
        character_id: packet.character_id,
        expelled: true,
    })?;
    packet_handler.register(|packet: GuildInviteResultPacket| NetworkEvent::GuildInviteResult { result: packet.result })?;
    // Bank (PACKETVER 20190605). The server only sends these when
    // `battle_config.feature_banking` is enabled and the map does not carry the
    // `nobank` mapflag; the client always registers the handlers so a server
    // that enables the feature works without another client build.
    packet_handler.register(|packet: OpenUserInterfacePacket| NetworkEvent::OpenUserInterface {
        ui_type: packet.ui_type,
        data: packet.data,
    })?;
    packet_handler.register(|_: BankOpenAckPacket| NetworkEvent::BankOpened)?;
    packet_handler.register(|packet: BankBalancePacket| NetworkEvent::BankBalance {
        money: packet.money,
        reason: packet.reason,
    })?;
    packet_handler.register(|packet: BankDepositAckPacket| NetworkEvent::BankDepositResult {
        reason: packet.reason,
        money: packet.money,
        zeny: packet.zeny,
    })?;
    packet_handler.register(|packet: BankWithdrawAckPacket| NetworkEvent::BankWithdrawResult {
        reason: packet.reason,
        money: packet.money,
        zeny: packet.zeny,
    })?;
    packet_handler.register(|_: BankCloseAckPacket| NetworkEvent::BankClosed)?;
    // --- Item service: weapon refine, repair, identify, card composition -----
    // The opcode/layout comments live on the packet structs in
    // `ragnarok-packets`; every header below was verified against the CondaRO
    // rAthena tree at PACKETVER 20190605.
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
    // --- Refine UI (`feature.refineui`, NPC/atcommand driven) ----------------
    packet_handler.register(|_packet: OpenRefiningUiPacket| NetworkEvent::RefiningUiOpened)?;
    packet_handler.register(|packet: RefiningMaterialListPacket| NetworkEvent::RefiningMaterialList {
        item_index: packet.item_index,
        blacksmith_blessing: packet.blacksmith_blessing,
        materials: packet.materials,
    })?;

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

/// Maps an equipment position mask to the sprite change types of the headgear
/// layers it covers. Headgear has no dedicated sprite change packet on the 2019
/// server, it is sent as the view id of the equip acknowledgement.
fn head_gear_sprite_types(equip_position: EquipPosition) -> Vec<SpriteChangeType> {
    let mut sprite_types = Vec::new();

    if equip_position.contains(EquipPosition::HEAD_TOP) {
        sprite_types.push(SpriteChangeType::HeadTop);
    }
    if equip_position.contains(EquipPosition::HEAD_MIDDLE) {
        sprite_types.push(SpriteChangeType::HeadMiddle);
    }
    if equip_position.contains(EquipPosition::HEAD_LOWER) {
        sprite_types.push(SpriteChangeType::HeadBottom);
    }

    sprite_types
}

/// Wire value of [`LoginFailedReason`] (`SC_NOTIFY_BAN` result byte, the
/// `#[numeric_value]` of the packet enum).
///
/// The message text of each value lives in `data\client\messages.ron`
/// (`MessageDomain::LoginFailed`).
fn login_failed_reason_code(reason: &LoginFailedReason) -> u16 {
    match reason {
        LoginFailedReason::ServerClosed => 1,
        LoginFailedReason::AlreadyLoggedIn => 2,
        LoginFailedReason::AlreadyOnline => 8,
    }
}

/// Wire value of [`LoginFailedReason2`] (`AC_REFUSE_LOGIN2` result, the
/// `#[numeric_value]` of the packet enum; the variants without an explicit
/// numeric value use their declaration index).
///
/// The message text of each value lives in `data\client\messages.ron`
/// (`MessageDomain::LoginFailed2`).
fn login_failed_reason2_code(reason: &LoginFailedReason2) -> u16 {
    match reason {
        LoginFailedReason2::UnregisteredId => 0,
        LoginFailedReason2::IncorrectPassword => 1,
        LoginFailedReason2::IdExpired => 2,
        LoginFailedReason2::RejectedFromServer => 3,
        LoginFailedReason2::BlockedByGMTeam => 4,
        LoginFailedReason2::GameOutdated => 5,
        LoginFailedReason2::LoginProhibitedUntil => 6,
        LoginFailedReason2::ServerFull => 7,
        LoginFailedReason2::CompanyAccountLimitReached => 8,
        LoginFailedReason2::BannedByDBATeam => 9,
        LoginFailedReason2::UnconfirmedEmail => 10,
        LoginFailedReason2::BannedByGMTeam => 11,
        LoginFailedReason2::TemporaryBanForDatabaseWork => 12,
        LoginFailedReason2::SelfLocked => 13,
        LoginFailedReason2::NotPermittedGroup => 14,
        LoginFailedReason2::AccountIdErased => 99,
        LoginFailedReason2::LoginInformationRemains => 100,
        LoginFailedReason2::LockedForHackingInvestigation => 101,
        LoginFailedReason2::TemporaryLockedForBugInvestigation => 102,
        LoginFailedReason2::DeletingCharacter => 103,
        LoginFailedReason2::DeletingSpouseCharacter => 104,
        LoginFailedReason2::UnknownError => 105,
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, DisplaySkillEffectAndDamagePacket, EntityId, SkillId, SkillLevel};

    use super::skill_damage_event;
    use crate::event::NetworkEvent;

    fn skill_packet(skill_id: u16) -> DisplaySkillEffectAndDamagePacket {
        DisplaySkillEffectAndDamagePacket {
            skill_id: SkillId(skill_id),
            source_entity_id: EntityId(20000076),
            destination_entity_id: EntityId(1002),
            start_time: ClientTick(12_345),
            soruce_delay: 0,
            destination_delay: 480,
            damage: 1234,
            level: SkillLevel(10),
            div: 1,
            skill_type: 6,
        }
    }

    #[test]
    fn skill_damage_keeps_its_skill_id_and_start_tick() {
        let event = skill_damage_event(skill_packet(56));

        let NetworkEvent::DamageEffect {
            source_entity_id,
            destination_entity_id,
            damage_amount,
            attack_duration,
            is_critical,
            skill_id,
            skill_start_tick,
        } = event
        else {
            panic!("expected DamageEffect");
        };

        assert_eq!(source_entity_id, EntityId(20000076));
        assert_eq!(destination_entity_id, EntityId(1002));
        assert_eq!(damage_amount, Some(1234));
        assert_eq!(attack_duration, 480);
        assert!(!is_critical);
        assert_eq!(skill_id.map(|skill_id| skill_id.0), Some(56));
        assert_eq!(skill_start_tick.map(|tick| tick.0), Some(12_345));
    }

    #[test]
    fn skill_id_zero_means_auto_attack() {
        let event = skill_damage_event(skill_packet(0));

        let NetworkEvent::DamageEffect { skill_id, .. } = event else {
            panic!("expected DamageEffect");
        };

        assert_eq!(skill_id, None);
    }
}

#[cfg(test)]
mod message_code_tests {
    use ragnarok_bytes::{ByteWriter, ToBytes};
    use ragnarok_packets::{LoginFailedReason, LoginFailedReason2};

    use super::{login_failed_reason_code, login_failed_reason2_code};

    /// The little-endian numeric value an enum is serialized to; the message
    /// table keys are exactly this wire value.
    fn wire_code<T: ToBytes>(value: &T) -> u16 {
        let mut writer = ByteWriter::new();
        value.to_bytes(&mut writer).expect("test-value encoding cannot fail");
        let bytes = writer.into_inner();
        match bytes.len() {
            1 => u16::from(bytes[0]),
            2 => u16::from_le_bytes([bytes[0], bytes[1]]),
            4 => u16::try_from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])).expect("test code fits into u16"),
            length => panic!("unexpected enum width: {length}"),
        }
    }

    #[test]
    fn login_failed_reason_codes_match_the_wire_values() {
        for (reason, code) in [
            (LoginFailedReason::ServerClosed, 1),
            (LoginFailedReason::AlreadyLoggedIn, 2),
            (LoginFailedReason::AlreadyOnline, 8),
        ] {
            assert_eq!(login_failed_reason_code(&reason), code);
            assert_eq!(wire_code(&reason), code, "wire drift for {reason:?}");
        }
    }

    #[test]
    fn login_failed_reason2_codes_match_the_wire_values() {
        for reason in [
            LoginFailedReason2::UnregisteredId,
            LoginFailedReason2::IncorrectPassword,
            LoginFailedReason2::IdExpired,
            LoginFailedReason2::RejectedFromServer,
            LoginFailedReason2::BlockedByGMTeam,
            LoginFailedReason2::GameOutdated,
            LoginFailedReason2::LoginProhibitedUntil,
            LoginFailedReason2::ServerFull,
            LoginFailedReason2::CompanyAccountLimitReached,
            LoginFailedReason2::BannedByDBATeam,
            LoginFailedReason2::UnconfirmedEmail,
            LoginFailedReason2::BannedByGMTeam,
            LoginFailedReason2::TemporaryBanForDatabaseWork,
            LoginFailedReason2::SelfLocked,
            LoginFailedReason2::NotPermittedGroup,
            LoginFailedReason2::AccountIdErased,
            LoginFailedReason2::LoginInformationRemains,
            LoginFailedReason2::LockedForHackingInvestigation,
            LoginFailedReason2::TemporaryLockedForBugInvestigation,
            LoginFailedReason2::DeletingCharacter,
            LoginFailedReason2::DeletingSpouseCharacter,
            LoginFailedReason2::UnknownError,
        ] {
            assert_eq!(
                login_failed_reason2_code(&reason),
                wire_code(&reason),
                "wire drift for {reason:?}"
            );
        }
    }
}
