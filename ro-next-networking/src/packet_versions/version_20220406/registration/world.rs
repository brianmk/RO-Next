//! world packet registrations.

use super::*;

pub(crate) fn register_world<Callback>(packet_handler: &mut PacketHandler<NetworkEventList, Callback>) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
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

    packet_handler.register(|packet: EntityStopMovePacket| NetworkEvent::EntityStopped {
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

    packet_handler.register(|packet: UpdateAttackRangePacket| NetworkEvent::AttackRangeChanged {
        attack_range: packet.attack_range,
    })?;

    packet_handler.register(|packet: MapTypePacket| NetworkEvent::MapProperties {
        map_type: packet.map_type,
        flags: packet.flags,
    })?;

    packet_handler.register(|packet: DisplaySpecialEffectPacket| NetworkEvent::SpecialEffect {
        entity_id: packet.entity_id,
        effect_id: packet.effect_id,
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

    packet_handler.register(|packet: EnterStatusEffectPacket2019| NetworkEvent::StatusEffect {
        entity_id: packet.entity_id,
        status_index: packet.effect_id,
        active: true,
        total_duration_in_milliseconds: packet.duration,
        remaining_in_milliseconds: packet.remaining_duration,
        values: [packet.value1, packet.value2, packet.value3],
    })?;

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

    packet_handler.register(|packet: RequestEntityDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails {
        entity_id: packet.entity_id,
        name: packet.name,
        // The NPC details response does not contain a guild name.
        guild_name: None,
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

    packet_handler.register(|packet: WarpPointListPacket| NetworkEvent::WarpPointList {
        skill_id: packet.skill_id,
        maps: packet
            .maps
            .into_iter()
            .map(|entry| entry.map_name)
            .filter(|map_name| !map_name.trim().is_empty())
            .collect(),
    })?;

    packet_handler.register(|packet: FriendOnlineStatusPacket| NetworkEvent::FriendOnlineState {
        account_id: packet.account_id,
        character_id: packet.character_id,
        online: matches!(packet.state, OnlineState::Online),
        name: packet.name,
    })?;

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

    packet_handler.register(|packet: ChangeMapCellPacket| NetworkEvent::MapCellChanged {
        position: packet.position,
        cell_type: packet.cell_type,
        map_name: packet.map_name,
    })?;
    register_ignored_noop::<_, OpenMarketPacket>(packet_handler)?;

    packet_handler.register(|packet: NewMailStatusPacket| NetworkEvent::MailUnreadStatus {
        unread: packet.new_available != 0,
    })?;

    packet_handler.register(|packet: PetStateChangePacket2019| NetworkEvent::PetStateChanged {
        entity_id: packet.entity_id,
        state_type: packet.state_type,
        value: packet.value,
    })?;

    Ok(())
}
