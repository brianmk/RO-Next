//! misc packet registrations.

use super::*;

pub(crate) fn register_misc<Callback>(
    packet_handler: &mut PacketHandler<NetworkEventList, Callback>,
    buffers: &ListBuffers,
) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
    let ListBuffers {
        inventory_items,
        storage_items,
        storage_name,
        cart_items,
    } = buffers;

    packet_handler.register(|packet: HighJumpPacket| NetworkEvent::EntityHighJumped {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;

    packet_handler.register(|packet: SnapPacket| NetworkEvent::EntitySnapped {
        entity_id: packet.entity_id,
        position: packet.position,
    })?;

    packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer {
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

    packet_handler.register(|packet: ExchangeRequestPacket2019| NetworkEvent::TradeRequest {
        requester_name: packet.requester_name,
        target_account_id: packet.target_account_id,
        target_level: packet.target_level,
    })?;

    packet_handler.register(|packet: ExchangeOfferPacket2019| NetworkEvent::TradeItemOffered {
        item_id: packet.item_id,
        amount: packet.amount,
    })?;

    packet_handler.register(|packet: ExchangeAckPacket2019| NetworkEvent::TradeAck {
        result: packet.result,
        target_account_id: packet.target_account_id,
    })?;

    packet_handler.register(|packet: ExchangeConcludePacket2019| NetworkEvent::TradeConcluded { who: packet.who })?;

    packet_handler.register(|packet: ExchangeExecPacket2019| NetworkEvent::TradeExecuted { result: packet.result })?;

    packet_handler.register(|packet: SpiritsPacket2019| NetworkEvent::SpiritOrbs {
        entity_id: packet.entity_id,
        amount: packet.amount,
    })?;

    packet_handler.register(|packet: Spirits2Packet2019| NetworkEvent::SpiritOrbs {
        entity_id: packet.entity_id,
        amount: packet.amount,
    })?;

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

    packet_handler.register(|packet: NextButtonPacket| {
        let NextButtonPacket { npc_id } = packet;

        NetworkEvent::AddNextButton { npc_id }
    })?;

    packet_handler.register(|packet: DisplayGainedExperiencePacket| NetworkEvent::GainedExperience {
        amount: packet.amount as i64,
        experience_type: packet.experience_type,
        experience_source: packet.experience_source,
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

    packet_handler.register(|packet: CriticalWeightUpdatePacket| NetworkEvent::CriticalWeight { percentage: packet.weight })?;
    // `ZC_AUTH_OK_GID` (0x0283) only re-confirms the player's block id that the
    // client already knows from login/character selection; parsed for framing.
    register_ignored_noop::<_, MapServerAuthOkPacket>(packet_handler)?;

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

    packet_handler.register(|packet: RequestStatUpResponsePacket| match packet.success {
        ragnarok_packets::RequestStatUpResult::Success => Vec::new(),
        ragnarok_packets::RequestStatUpResult::Failure => vec![NetworkEvent::StatUpFailed],
    })?;

    packet_handler.register(|packet: AmmunitionActionPacket| NetworkEvent::AmmunitionAction {
        action: packet.action_type,
    })?;

    packet_handler.register(|packet: RefiningMaterialListPacket| NetworkEvent::RefiningMaterialList {
        item_index: packet.item_index,
        blacksmith_blessing: packet.blacksmith_blessing,
        materials: packet.materials,
    })?;

    packet_handler.register(|packet: MonsterInfoPacket| NetworkEvent::MonsterInfo { information: packet })?;

    // Party booking / LFG (2019). rAthena registers the booking board at
    // 0x0802..0x080B; the direct join flow (0x0AE7/0x0AFA) is gated with
    // `#if PACKETVER >= 20191204` server side, so a 20190605 server never sends
    // it. Both are registered anyway so a newer server cannot desynchronize the
    // stream.
    packet_handler
        .register(|packet: PartyBookingRegisterResultPacket2019| NetworkEvent::PartyBookingRegisterResult { result: packet.result })?;

    Ok(())
}
