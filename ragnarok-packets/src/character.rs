//! character packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x006B)]
#[variable_length]
pub struct CharacterListPacket {
    pub maximum_slot_count: u8,
    pub available_slot_count: u8,
    pub vip_slot_count: u8,
    #[new_default]
    pub unknown: [u8; 20],
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation2019>,
}

/// Sent by the character server as a response to [RequestCharacterListPacket]
/// on clients that still use the pre-2020 character list layout (rAthena 2019
/// clients, `PACKETVER < 20201007`). Each row is a `CHARACTER_INFO` of 155
/// bytes (32-bit HP/SP), unlike the 2022 layout in [CharacterInformation].
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x099D)]
#[variable_length]
pub struct CharacterInfoPerPagePacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation2019>,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09A0)]
#[packet_size(6)]
pub struct CharacterSlotPagePacket {
    pub page_quantity: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum CharacterSelectionFailedReason {
    RejectedFromServer,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// failing. Provides a reason for the character selection failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x006C)]
#[packet_size(3)]
pub struct CharacterSelectionFailedPacket {
    pub reason: CharacterSelectionFailedReason,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// succeeding. Provides a map server to connect to, along with the id of our
/// selected character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AC5)]
#[packet_size(156)]
pub struct CharacterSelectionSuccessPacket {
    pub character_id: CharacterId,
    #[length(16)]
    pub map_name: String,
    pub map_server_ip: ServerAddress,
    pub map_server_port: u16,
    // NOTE: Could be `new_default` but Rust doesn't implement `[u8; 128]: Default`.
    #[new_value([0; 128])]
    pub unknown: [u8; 128],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum CharacterCreationFailedReason {
    CharacterNameAlreadyUsed,
    NotOldEnough,
    #[numeric_value(3)]
    NotAllowedToUseSlot,
    #[numeric_value(255)]
    CharacterCerationFailed,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// failing. Provides a reason for the character creation failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x006E)]
#[packet_size(3)]
pub struct CharacterCreationFailedPacket {
    pub reason: CharacterCreationFailedReason,
}

/// Sent by the client to the character server when the player tries to create
/// a new character.
/// Attempts to create a new character in an empty slot using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A39)]
pub struct CreateCharacterPacket {
    #[length(24)]
    pub name: String,
    pub slot: u8,
    pub hair_color: u16,     // TODO: HairColor
    pub hair_style: u16,     // TODO: HairStyle
    pub start_job_id: JobId, // TODO: Job
    #[new_default]
    pub unknown: [u8; 2],
    pub sex: Sex,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct CharacterInformation {
    pub character_id: CharacterId,
    pub experience: i64,
    pub money: i32,
    pub job_experience: i64,
    pub job_level: i32,
    pub body_state: i32,
    pub health_state: i32,
    pub effect_state: i32,
    pub virtue: i32,
    pub honor: i32,
    pub stat_points: i16,
    pub health_points: i64,
    pub maximum_health_points: i64,
    pub spell_points: i64,
    pub maximum_spell_points: i64,
    pub movement_speed: i16,
    pub job_id: JobId,
    pub head: i16,
    pub body: i16,
    pub weapon: i16,
    pub base_level: i16,
    pub sp_point: i16,
    pub accessory: i16,
    pub shield: i16,
    pub accessory2: i16,
    pub accessory3: i16,
    pub head_palette: i16,
    pub body_palette: i16,
    #[length(24)]
    pub name: String,
    pub strength: u8,
    pub agility: u8,
    pub vitality: u8,
    pub intelligence: u8,
    pub dexterity: u8,
    pub luck: u8,
    pub character_number: u8,
    pub hair_color: u8,
    pub b_is_changed_char: i16,
    #[length(16)]
    pub map_name: String,
    pub deletion_reverse_date: i32,
    pub robe_palette: i32,
    pub character_slot_change_count: i32,
    pub character_name_change_count: i32,
    pub sex: Sex,
}

/// Wire layout of rAthena's `CHARACTER_INFO` for clients before 2020-10-07
/// (`PACKETVER < 20201007`), i.e. the 2019 client used by CondaRO. It differs
/// from [CharacterInformation] in the HP/SP field widths (32/32/16/16 instead
/// of four 64-bit values), making one row 155 bytes instead of 175.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct CharacterInformation2019 {
    pub character_id: CharacterId,
    pub experience: i64,
    pub money: i32,
    pub job_experience: i64,
    pub job_level: i32,
    pub body_state: i32,
    pub health_state: i32,
    pub effect_state: i32,
    pub virtue: i32,
    pub honor: i32,
    pub stat_points: i16,
    pub health_points: i32,
    pub maximum_health_points: i32,
    pub spell_points: i16,
    pub maximum_spell_points: i16,
    pub movement_speed: i16,
    pub job_id: JobId,
    pub head: i16,
    pub body: i16,
    pub weapon: i16,
    pub base_level: i16,
    pub sp_point: i16,
    pub accessory: i16,
    pub shield: i16,
    pub accessory2: i16,
    pub accessory3: i16,
    pub head_palette: i16,
    pub body_palette: i16,
    #[length(24)]
    pub name: String,
    pub strength: u8,
    pub agility: u8,
    pub vitality: u8,
    pub intelligence: u8,
    pub dexterity: u8,
    pub luck: u8,
    pub character_number: u8,
    pub hair_color: u8,
    pub b_is_changed_char: i16,
    #[length(16)]
    pub map_name: String,
    pub deletion_reverse_date: i32,
    pub robe_palette: i32,
    pub character_slot_change_count: i32,
    pub character_name_change_count: i32,
    pub sex: Sex,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// succeeding on 2019-era servers (`PACKETVER < 20201007`): the fixed-size
/// 0x006D packet carrying one legacy 155-byte CHARACTER_INFO row.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x006D)]
#[packet_size(157)]
pub struct CreateCharacterSuccessPacket2019 {
    pub character_information: CharacterInformation2019,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// succeeding. Provides all character information of the newly created
/// character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B6F)]
pub struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}

/// Sent by the client to the character server.
/// Requests a list of every character associated with the account.
#[derive(Debug, Clone, Default, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09A1)]
pub struct RequestCharacterListPacket {}

/// Sent by the character server as a response to [RequestCharacterListPacket]
/// succeeding. Provides the requested list of character information.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B72)]
#[variable_length]
pub struct RequestCharacterListSuccessPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation>,
}

/// Sent by the client to the character server when the user tries to delete a
/// character.
/// Attempts to delete a character from the user account using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01FB)]
pub struct DeleteCharacterPacket {
    pub character_id: CharacterId,
    /// This field can be used for email or date of birth, depending on the
    /// configuration of the character server.
    #[length(40)]
    pub email: String,
    /// Ignored by rAthena
    #[new_default]
    pub unknown: [u8; 10],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum CharacterDeletionFailedReason {
    NotAllowed,
    CharacterNotFound,
    NotEligible,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// failing. Provides a reason for the character deletion failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0070)]
#[packet_size(3)]
pub struct CharacterDeletionFailedPacket {
    pub reason: CharacterDeletionFailedReason,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// succeeding.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x006F)]
#[packet_size(2)]
pub struct CharacterDeletionSuccessPacket {}

/// Sent by the client to the character server when the user selects a
/// character. Attempts to select the character in the specified slot.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0066)]
pub struct SelectCharacterPacket {
    pub selected_slot: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01D7)]
#[packet_size(15)]
pub struct SpriteChangePacket {
    pub account_id: AccountId,
    pub sprite_type: SpriteChangeType,
    pub value: u32,
    pub value2: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum SpriteChangeType {
    Base,
    Hair,
    Weapon,
    HeadBottom,
    HeadTop,
    HeadMiddle,
    HairCollor,
    ClothesColor,
    Shield,
    Shoes,
    Body,
    ResetCostumes,
    Robe,
    Body2,
}

/// Every value that can be set from the server through [UpdateStatPacket],
/// [UpdateStatPacket1], [UpdateStatPacket2], and [UpdateStatPacket3].
/// All UpdateStatPackets do the same, they just have different sizes
/// correlating to the space the updated value requires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatType {
    Weight(u32),
    MaximumWeight(u32),
    MovementSpeed(u32),
    BaseLevel(u32),
    JobLevel(u32),
    Karma(u32),
    Manner(u32),
    StatPoints(u32),
    SkillPoints(u32),
    Hit(u32),
    Flee1(u32),
    Flee2(u32),
    MaximumHealthPoints(u32),
    MaximumSpellPoints(u32),
    HealthPoints(u32),
    SpellPoints(u32),
    AttackSpeed(u32),
    Attack1(u32),
    Defense1(u32),
    MagicDefense1(u32),
    Attack2(u32),
    Defense2(u32),
    MagicDefense2(u32),
    Critical(u32),
    MagicAttack1(u32),
    MagicAttack2(u32),
    Zeny(u32),
    BaseExperience(u64),
    JobExperience(u64),
    NextBaseExperience(u64),
    NextJobExperience(u64),
    StrengthStatPointCost(u8),
    AgilityStatPointCost(u8),
    VitalityStatPointCost(u8),
    IntelligenceStatPointCost(u8),
    DexterityStatPointCost(u8),
    LuckStatPointCost(u8),
    Strength(i32, i32),
    Agility(i32, i32),
    Vitality(i32, i32),
    Intelligence(i32, i32),
    Dexterity(i32, i32),
    Luck(i32, i32),
    CartInfo(u16, u32, u32),
    ActivityPoints(u32),
    TraitPoint(u32),
    MaximumActivityPoints(u32),
    Power(u32, u32),
    Stamina(u32, u32),
    Wisdom(u32, u32),
    Spell(u32, u32),
    Concentration(u32, u32),
    Creativity(u32, u32),
    PowerStatPointCost(u8),
    StaminaStatPointCost(u8),
    WisdomStatPointCost(u8),
    SpellStatPointCost(u8),
    ConcentrationStatPointCost(u8),
    CreativitySpellPointCost(u8),
    PhysicalAttack(u32),
    SpellMagicAttack(u32),
    Resistance(u32),
    MagicResistance(u32),
    HealingPlus(u32),
    CriticalDamageRate(u32),
}

impl FromBytes for StatType {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        /// For some reason, the stats are packed into the upper two bytes of an
        /// `i32`. I am unsure why that is but for now we will just work
        /// with what we got.
        fn weirdly_packed_stat(byte_reader: &mut ByteReader) -> ConversionResult<(i32, i32)> {
            let _ = i16::from_bytes(byte_reader)?;
            let base = i16::from_bytes(byte_reader)?;
            let _ = i16::from_bytes(byte_reader)?;
            let bonus = i16::from_bytes(byte_reader)?;

            Ok((base as i32, bonus as i32))
        }

        let stat = match u16::from_bytes(byte_reader).trace::<Self>()? {
            0 => u32::from_bytes(byte_reader).map(Self::MovementSpeed),
            1 => u64::from_bytes(byte_reader).map(Self::BaseExperience),
            2 => u64::from_bytes(byte_reader).map(Self::JobExperience),
            3 => u32::from_bytes(byte_reader).map(Self::Karma),
            4 => u32::from_bytes(byte_reader).map(Self::Manner),
            5 => u32::from_bytes(byte_reader).map(Self::HealthPoints),
            6 => u32::from_bytes(byte_reader).map(Self::MaximumHealthPoints),
            7 => u32::from_bytes(byte_reader).map(Self::SpellPoints),
            8 => u32::from_bytes(byte_reader).map(Self::MaximumSpellPoints),
            9 => u32::from_bytes(byte_reader).map(Self::StatPoints),
            11 => u32::from_bytes(byte_reader).map(Self::BaseLevel),
            12 => u32::from_bytes(byte_reader).map(Self::SkillPoints),
            13 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Strength(base, bonus)),
            14 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Agility(base, bonus)),
            15 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Vitality(base, bonus)),
            16 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Intelligence(base, bonus)),
            17 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Dexterity(base, bonus)),
            18 => weirdly_packed_stat(byte_reader).map(|(base, bonus)| Self::Luck(base, bonus)),
            20 => u32::from_bytes(byte_reader).map(Self::Zeny),
            22 => u64::from_bytes(byte_reader).map(Self::NextBaseExperience),
            23 => u64::from_bytes(byte_reader).map(Self::NextJobExperience),
            24 => u32::from_bytes(byte_reader).map(Self::Weight),
            25 => u32::from_bytes(byte_reader).map(Self::MaximumWeight),
            32 => u8::from_bytes(byte_reader).map(Self::StrengthStatPointCost),
            33 => u8::from_bytes(byte_reader).map(Self::AgilityStatPointCost),
            34 => u8::from_bytes(byte_reader).map(Self::VitalityStatPointCost),
            35 => u8::from_bytes(byte_reader).map(Self::IntelligenceStatPointCost),
            36 => u8::from_bytes(byte_reader).map(Self::DexterityStatPointCost),
            37 => u8::from_bytes(byte_reader).map(Self::LuckStatPointCost),
            41 => u32::from_bytes(byte_reader).map(Self::Attack1),
            42 => u32::from_bytes(byte_reader).map(Self::Attack2),
            43 => u32::from_bytes(byte_reader).map(Self::MagicAttack1),
            44 => u32::from_bytes(byte_reader).map(Self::MagicAttack2),
            45 => u32::from_bytes(byte_reader).map(Self::Defense1),
            46 => u32::from_bytes(byte_reader).map(Self::Defense2),
            47 => u32::from_bytes(byte_reader).map(Self::MagicDefense1),
            48 => u32::from_bytes(byte_reader).map(Self::MagicDefense2),
            49 => u32::from_bytes(byte_reader).map(Self::Hit),
            50 => u32::from_bytes(byte_reader).map(Self::Flee1),
            51 => u32::from_bytes(byte_reader).map(Self::Flee2),
            52 => u32::from_bytes(byte_reader).map(Self::Critical),
            53 => u32::from_bytes(byte_reader).map(Self::AttackSpeed),
            55 => u32::from_bytes(byte_reader).map(Self::JobLevel),
            99 => u16::from_bytes(byte_reader)
                .and_then(|a| Ok(Self::CartInfo(a, u32::from_bytes(byte_reader)?, u32::from_bytes(byte_reader)?))),
            219 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Power(a, u32::from_bytes(byte_reader)?))),
            220 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Stamina(a, u32::from_bytes(byte_reader)?))),
            221 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Wisdom(a, u32::from_bytes(byte_reader)?))),
            222 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Spell(a, u32::from_bytes(byte_reader)?))),
            223 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Concentration(a, u32::from_bytes(byte_reader)?))),
            224 => u32::from_bytes(byte_reader).and_then(|a| Ok(Self::Creativity(a, u32::from_bytes(byte_reader)?))),
            225 => u32::from_bytes(byte_reader).map(Self::PhysicalAttack),
            226 => u32::from_bytes(byte_reader).map(Self::SpellMagicAttack),
            227 => u32::from_bytes(byte_reader).map(Self::Resistance),
            228 => u32::from_bytes(byte_reader).map(Self::MagicResistance),
            229 => u32::from_bytes(byte_reader).map(Self::HealingPlus),
            230 => u32::from_bytes(byte_reader).map(Self::CriticalDamageRate),
            231 => u32::from_bytes(byte_reader).map(Self::TraitPoint),
            232 => u32::from_bytes(byte_reader).map(Self::ActivityPoints),
            233 => u32::from_bytes(byte_reader).map(Self::MaximumActivityPoints),
            247 => u8::from_bytes(byte_reader).map(Self::PowerStatPointCost),
            248 => u8::from_bytes(byte_reader).map(Self::StaminaStatPointCost),
            249 => u8::from_bytes(byte_reader).map(Self::WisdomStatPointCost),
            250 => u8::from_bytes(byte_reader).map(Self::SpellStatPointCost),
            251 => u8::from_bytes(byte_reader).map(Self::ConcentrationStatPointCost),
            252 => u8::from_bytes(byte_reader).map(Self::CreativitySpellPointCost),
            invalid => Err(ConversionError::from_message(format!("invalid stat id {invalid}"))),
        };

        stat.trace::<Self>()
    }
}

impl ToBytes for StatType {
    fn to_bytes(&self, _byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        // `StatType` is only ever sent by the server; it has no client-side
        // serialized form. Return an error instead of panicking if a future
        // packet tries to serialize it.
        Err(ConversionError::from_message("StatType cannot be serialized to the wire"))
    }
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B0)]
#[packet_size(8)]
pub struct UpdateStatPacket {
    #[length(6)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0196)]
#[packet_size(9)]
pub struct StatusChangeSequencePacket {
    pub index: u16,
    pub id: u32,
    pub state: u8,
}

/// Sent by the character server to the client when loading onto a new map.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00BD)]
#[packet_size(44)]
pub struct InitialStatsPacket {
    pub stat_points: u16,
    pub strength: u8,
    pub strength_stat_points_cost: u8,
    pub agility: u8,
    pub agility_stat_points_cost: u8,
    pub vitatity: u8,
    pub vitality_stat_points_cost: u8,
    pub intelligence: u8,
    pub intelligence_stat_points_cost: u8,
    pub dexterity: u8,
    pub dexterity_stat_points_cost: u8,
    pub luck: u8,
    pub luck_stat_points_cost: u8,
    pub left_attack: u16,
    pub rigth_attack: u16,
    pub rigth_magic_attack: u16,
    pub left_magic_attack: u16,
    pub left_defense: u16,
    pub rigth_defense: u16,
    pub rigth_magic_defense: u16,
    pub left_magic_defense: u16,
    pub hit: u16, // ?
    pub flee: u16,
    pub flee2: u16,
    pub crit: u16,
    pub attack_speed: u16,
    /// Always 0 on rAthena
    #[new_default]
    pub bonus_attack_speed: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0141)]
#[packet_size(14)]
pub struct UpdateStatPacket1 {
    #[length(12)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0ACB)]
#[packet_size(12)]
pub struct UpdateStatPacket2 {
    #[length(10)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00BE)]
#[packet_size(5)]
pub struct UpdateStatPacket3 {
    #[length(3)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum StatUpType {
    Strength { amount: u8 },
    Agility { amount: u8 },
    Vitality { amount: u8 },
    Intelligence { amount: u8 },
    Dexterity { amount: u8 },
    Luck { amount: u8 },
}

impl FromBytes for StatUpType {
    fn from_bytes(_: &mut ByteReader) -> ConversionResult<Self> {
        // `StatUpType` is only serialized by the client; the server never sends
        // it. Report a conversion error instead of panicking if a packet ever
        // tries to read it back.
        Err(ConversionError::from_message("StatUpType is send-only"))
    }
}

impl ToBytes for StatUpType {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        match self {
            Self::Strength { amount } => {
                13u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Agility { amount } => {
                14u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Vitality { amount } => {
                15u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Intelligence { amount } => {
                16u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Dexterity { amount } => {
                17u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
            Self::Luck { amount } => {
                18u16.to_bytes(byte_writer)?;
                amount.to_bytes(byte_writer)
            }
        }
    }
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00BB)]
pub struct RequestStatUpPacket {
    pub stat_type: StatUpType,
}

/// rAthena seems to always return [`Success`](RequestStatUpResult::Success),
/// even if the request fails.
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum RequestStatUpResult {
    Failure,
    Success,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00BC)]
#[packet_size(6)]
pub struct RequestStatUpResponsePacket {
    pub staus_type: u16,
    pub success: RequestStatUpResult,
    pub value: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x08D4)]
pub struct SwitchCharacterSlotPacket {
    // TODO: Type this more strongly.
    pub origin_slot: u16,
    // TODO: Type this more strongly.
    pub destination_slot: u16,
    /// 1 instead of default, just in case the sever actually uses this value
    /// (rAthena does not)
    #[new_value(1)]
    pub remaining_moves: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum SwitchCharacterSlotResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B70)]
pub struct SwitchCharacterSlotResponsePacket {
    #[new_default]
    pub unknown: u16, // is always 8 ?
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

/// 2019 era `HC_ACK_CHANGE_CHARACTER_SLOT` (0x08D5).
///
/// The 2019 character server answers a character slot change with 0x08D5
/// instead of the 2022 era 0x0B70 used by
/// [SwitchCharacterSlotResponsePacket], but the 8 byte layout is identical
/// (see rAthena `chclif_changechar_slot`). Without this registration the client
/// kept waiting for the slot change to finish forever.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x08D5)]
#[packet_size(8)]
pub struct SwitchCharacterSlotResponsePacket2019 {
    #[new_default]
    pub unknown: u16,
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0983)]
#[packet_size(29)]
pub struct StatusChangePacket {
    pub index: u16,
    pub entity_id: EntityId,
    pub state: u8,
    pub duration_in_milliseconds: u32,
    pub remaining_in_milliseconds: u32,
    pub value: [u32; 3],
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0229)]
#[packet_size(15)]
pub struct StateChangePacket {
    pub entity_id: EntityId,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub is_pk_mode_on: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00B8)]
pub struct ChooseDialogOptionPacket {
    pub npc_id: EntityId,
    pub option: i8,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
    pub struct EquipPosition: u32 {
        const NONE = 0;
        const HEAD_LOWER = 1;
        const HEAD_MIDDLE = 512;
        const HEAD_TOP = 256;
        const RIGHT_HAND = 2;
        const LEFT_HAND = 32;
        const ARMOR = 16;
        const SHOES = 64;
        const GARMENT = 4;
        const LEFT_ACCESSORY = 8;
        const RIGTH_ACCESSORY = 128;
        const COSTUME_HEAD_TOP = 1024;
        const COSTUME_HEAD_MIDDLE = 2048;
        // rAthena `EQP_COSTUME_HEAD_LOW = 0x001000` (4096).
        const COSTUME_HEAD_LOWER = 4096;
        const COSTUME_GARMENT = 8192;
        const AMMO = 32768;
        const SHADOW_ARMOR = 65536;
        const SHADOW_WEAPON = 131072;
        const SHADOW_SHIELD = 262144;
        const SHADOW_SHOES = 524288;
        const SHADOW_RIGHT_ACCESSORY = 1048576;
        const SHADOW_LEFT_ACCESSORY = 2097152;
        const LEFT_RIGHT_ACCESSORY = 136;
        const LEFT_RIGHT_HAND = 34;
        const SHADOW_LEFT_RIGHT_ACCESSORY = 3145728;
    }
}

// TODO: check that this can be only 1 and 0, if not ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum RestartResponseStatus {
    Nothing,
    Ok,
}

// TODO: check that this can be only 1 and 0, if not Named, ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum DisconnectResponseStatus {
    Ok,
    Wait10Seconds,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum OnlineState {
    Online,
    Offline,
}
