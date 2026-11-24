//! skills packet registrations.

use super::*;

pub(crate) fn register_skills<Callback>(packet_handler: &mut PacketHandler<NetworkEventList, Callback>) -> Result<(), DuplicateHandlerError>
where
    Callback: PacketCallback,
{
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

    packet_handler.register(|packet: DisplaySkillCooldownPacket| NetworkEvent::SkillCooldown {
        skill_id: packet.skill_id,
        until: packet.until,
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

    packet_handler.register(|packet: SkillCastCancelPacket| NetworkEvent::SkillCastCancelled {
        entity_id: packet.entity_id,
    })?;

    packet_handler.register(|packet: ToUseSkillSuccessPacket| NetworkEvent::SkillUseFailed {
        skill_id: packet.skill_id,
        cause: packet.cause,
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
