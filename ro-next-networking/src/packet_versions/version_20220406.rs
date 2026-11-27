use std::cell::RefCell;
use std::net::IpAddr;
use std::rc::Rc;
use std::time::Instant;

use ragnarok_packets::handler::{DuplicateHandlerError, PacketCallback, PacketHandler};
use ragnarok_packets::*;
use ro_next_messages::{MessageDomain, format_message};

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
    packet_handler.register(|packet: LoginPincodePacket| NetworkEvent::PincodeState {
        pincode_seed: packet.pincode_seed,
        account_id: packet.account_id,
        state: packet.state,
    })?;
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
        // `div` is the hit count (`clif_skill_damage`): the official client
        // splits the damage into this many numbers at 200 ms intervals
        // (`RenderObject.cpp:5367-5368`, `:5931-5947`). Aegis-style Throw
        // Spirit Sphere sends one packet with `div = skill level`
        // (`conf/battle/skill.conf: finger_offensive_type: 0`).
        hit_count: packet.div,
        attack_duration: packet.destination_delay,
        is_critical: false,
        skill_id: (packet.skill_id.0 != 0).then_some(packet.skill_id),
        skill_start_tick: Some(packet.start_time),
    }
}

mod registration;
pub use registration::register_map_server_packets;

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
            hit_count,
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
        assert_eq!(hit_count, 1);
        assert_eq!(attack_duration, 480);
        assert!(!is_critical);
        assert_eq!(skill_id.map(|skill_id| skill_id.0), Some(56));
        assert_eq!(skill_start_tick.map(|tick| tick.0), Some(12_345));
    }

    #[test]
    fn multi_hit_skill_damage_keeps_the_hit_count() {
        let mut packet = skill_packet(267);
        packet.damage = 500;
        packet.div = 5;

        let NetworkEvent::DamageEffect {
            damage_amount, hit_count, ..
        } = skill_damage_event(packet)
        else {
            panic!("expected DamageEffect");
        };

        // Throw Spirit Sphere (Aegis style) sends one packet with
        // `div = skill level`; the client splits the 500 damage into five
        // numbers of 100 each.
        assert_eq!(damage_amount, Some(500));
        assert_eq!(hit_count, 5);
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
