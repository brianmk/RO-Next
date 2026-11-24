#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

mod character;
pub use self::character::*;
mod commerce;
pub use self::commerce::*;
mod entity;
pub use self::entity::*;
mod inventory;
pub use self::inventory::*;
mod login;
pub use self::login::*;
mod mail;
pub use self::mail::*;
mod map;
pub use self::map::*;
mod misc;
pub use self::misc::*;
mod primitives;
pub use self::primitives::*;
mod skill;
pub use self::skill::*;
mod social;
pub use self::social::*;
mod ui;
pub use self::ui::*;

pub mod handler;

pub mod limits;

pub mod opcode_lengths_2019;

pub use self::limits::{MIN_NAME_LENGTH, NAME_LENGTH, PASSWORD_LENGTH, USER_ID_LENGTH};

mod position;

pub mod validation;

use std::net::Ipv4Addr;

use ragnarok_bytes::{
    ByteConvertable, ByteReader, ByteWriter, ConversionError, ConversionResult, ConversionResultExt, FixedByteSize, FromBytes, ToBytes,
};
#[cfg(feature = "derive")]
pub use ragnarok_macros::{CharacterServer, ClientPacket, LoginServer, MapServer, Packet, ServerPacket};
#[cfg(not(feature = "derive"))]
use ragnarok_macros::{CharacterServer, ClientPacket, LoginServer, MapServer, Packet, ServerPacket};

pub use self::position::{Direction, WorldPosition, WorldPosition2};

// To make proc macros work in ro_next_interface.
extern crate self as ragnarok_packets;

/// Base trait that all packets implement.
/// All packets in Ragnarok online consist of a header, two bytes in size,
/// followed by the packet data. If the packet does not have a fixed size,
/// the first two bytes will be the size of the packet in bytes *including* the
/// header. Packets are sent in little endian.
pub trait Packet: std::fmt::Debug + Send + Clone + 'static {
    /// Any scheduled packet that does not depend on in-game events should be
    /// marked as a ping. This is mostly for filtering when logging
    /// packet traffic.
    const IS_PING: bool;
    /// The header of the Packet.
    const HEADER: PacketHeader;
    /// Whether the packet carries a variable length.
    ///
    /// Variable length packets store their total size, including the two
    /// header bytes, in the first two payload bytes.
    const IS_VARIABLE_LENGTH: bool = false;
    /// The full wire size of a fixed size packet, including the two header
    /// bytes, when the struct declares it via `#[packet_size(..)]`.
    ///
    /// `None` means that the size is either not fixed or not declared; the
    /// 2019 opcode registry (see [`opcode_lengths_2019`]) is the authoritative
    /// source for fixed wire sizes.
    const EXPECTED_SIZE: Option<usize> = None;

    /// Read packet **without the header**. To read the packet with the header,
    /// use [`PacketExt::packet_from_bytes`].
    fn payload_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self>;

    /// Write packet **without the header**. To write the packet with the
    /// header, use [`PacketExt::packet_to_bytes`].
    fn payload_to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize>;

    // Implementation detail of RO-Next. Can be used to convert a packet to an
    // UI element in the packet inspector.
    #[cfg(feature = "packet-to-state-element")]
    fn to_element<App: ro_next_interface::application::Application>(
        self_path: impl rust_state::Path<App, Self>,
        name: String,
    ) -> Box<dyn ro_next_interface::element::Element<App, LayoutInfo = ()>>;
}

/// Extension trait for reading and writing packets with the header.
pub trait PacketExt: Packet {
    /// Read packet **with the header**. To read the packet without the header,
    /// use [`Packet::payload_from_bytes`].
    fn packet_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self>;

    /// Write packet **with the header**. To write the packet without the
    /// header, use [`Packet::payload_to_bytes`].
    fn packet_to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize>;
}

impl<T> PacketExt for T
where
    T: Packet,
{
    fn packet_from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let header = PacketHeader::from_bytes(byte_reader)?;

        if header != Self::HEADER {
            return Err(ConversionError::from_message("mismatched header"));
        }

        Self::payload_from_bytes(byte_reader)
    }

    fn packet_to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        let mut written = Self::HEADER.to_bytes(byte_writer)?;
        written += self.payload_to_bytes(byte_writer)?;
        Ok(written)
    }
}

/// Marker trait for packets sent by the client.
pub trait ClientPacket: Packet {}

/// Marker trait for packets sent by the server.
pub trait ServerPacket: Packet {}

/// Marker trait for login server packets.
pub trait LoginServerPacket: Packet {}

/// Marker trait for character server packets.
pub trait CharacterServerPacket: Packet {}

/// Marker trait for map server packets.
pub trait MapServerPacket: Packet {}

impl From<ServerAddress> for Ipv4Addr {
    fn from(value: ServerAddress) -> Self {
        value.0.into()
    }
}

#[cfg(feature = "interface")]
impl rust_state::VecItem for CharacterInformation {
    // TODO: Use CharacterId
    type Id = u32;

    fn get_id(&self) -> Self::Id {
        self.character_id.0
    }
}

impl From<CharacterInformation2019> for CharacterInformation {
    fn from(value: CharacterInformation2019) -> Self {
        Self {
            character_id: value.character_id,
            experience: value.experience,
            money: value.money,
            job_experience: value.job_experience,
            job_level: value.job_level,
            body_state: value.body_state,
            health_state: value.health_state,
            effect_state: value.effect_state,
            virtue: value.virtue,
            honor: value.honor,
            stat_points: value.stat_points,
            health_points: value.health_points as i64,
            maximum_health_points: value.maximum_health_points as i64,
            spell_points: value.spell_points as i64,
            maximum_spell_points: value.maximum_spell_points as i64,
            movement_speed: value.movement_speed,
            job_id: value.job_id,
            head: value.head,
            body: value.body,
            weapon: value.weapon,
            base_level: value.base_level,
            sp_point: value.sp_point,
            accessory: value.accessory,
            shield: value.shield,
            accessory2: value.accessory2,
            accessory3: value.accessory3,
            head_palette: value.head_palette,
            body_palette: value.body_palette,
            name: value.name,
            strength: value.strength,
            agility: value.agility,
            vitality: value.vitality,
            intelligence: value.intelligence,
            dexterity: value.dexterity,
            luck: value.luck,
            character_number: value.character_number,
            hair_color: value.hair_color,
            b_is_changed_char: value.b_is_changed_char,
            map_name: value.map_name,
            deletion_reverse_date: value.deletion_reverse_date,
            robe_palette: value.robe_palette,
            character_slot_change_count: value.character_slot_change_count,
            character_name_change_count: value.character_name_change_count,
            sex: value.sex,
        }
    }
}

#[cfg(feature = "interface")]
impl rust_state::VecItem for CharacterInformation2019 {
    type Id = u32;

    fn get_id(&self) -> Self::Id {
        self.character_id.0
    }
}

impl From<EquippableItemInformation2019> for EquippableItemInformation {
    fn from(value: EquippableItemInformation2019) -> Self {
        Self {
            index: value.index,
            item_id: value.item_id,
            item_type: value.item_type,
            equip_position: value.equip_position,
            equipped_position: value.equipped_position,
            slot: value.slot,
            hire_expiration_date: value.hire_expiration_date,
            bind_on_equip_type: value.bind_on_equip_type,
            w_item_sprite_number: value.w_item_sprite_number,
            option_count: value.option_count,
            option_data: value.option_data,
            refinement_level: value.refinement_level,
            enchantment_level: 0,
            flags: value.flags,
        }
    }
}

impl std::fmt::Display for StatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Weight(value) => write!(f, "Weight: {}", value),
            Self::MaximumWeight(value) => write!(f, "Maximum Weight: {}", value),
            Self::MovementSpeed(value) => write!(f, "Movement Speed: {}", value),
            Self::BaseLevel(value) => write!(f, "Base Level: {}", value),
            Self::JobLevel(value) => write!(f, "Job Level: {}", value),
            Self::Karma(value) => write!(f, "Karma: {}", value),
            Self::Manner(value) => write!(f, "Manner: {}", value),
            Self::StatPoints(value) => write!(f, "Stat Points: {}", value),
            Self::SkillPoints(value) => write!(f, "Skill Point: {}", value),
            Self::Hit(value) => write!(f, "Hit: {}", value),
            Self::Flee1(value) => write!(f, "Flee1: {}", value),
            Self::Flee2(value) => write!(f, "Flee2: {}", value),
            Self::MaximumHealthPoints(value) => write!(f, "Maximum Health Points: {}", value),
            Self::MaximumSpellPoints(value) => write!(f, "Maximum Spell Points: {}", value),
            Self::HealthPoints(value) => write!(f, "Health Points: {}", value),
            Self::SpellPoints(value) => write!(f, "Spell Points: {}", value),
            Self::AttackSpeed(value) => write!(f, "Attack Speed: {}", value),
            Self::Attack1(value) => write!(f, "Attack1: {}", value),
            Self::Defense1(value) => write!(f, "Defense1: {}", value),
            Self::MagicDefense1(value) => write!(f, "Magic Defense1: {}", value),
            Self::Attack2(value) => write!(f, "Attack2: {}", value),
            Self::Defense2(value) => write!(f, "Defense2: {}", value),
            Self::MagicDefense2(value) => write!(f, "Magic Defense2: {}", value),
            Self::Critical(value) => write!(f, "Critical: {}", value),
            Self::MagicAttack1(value) => write!(f, "Magic Attack1: {}", value),
            Self::MagicAttack2(value) => write!(f, "Magic Attack2: {}", value),
            Self::Zeny(value) => write!(f, "Zeny: {}", value),
            Self::BaseExperience(value) => write!(f, "Base Experience: {}", value),
            Self::JobExperience(value) => write!(f, "Job Experience: {}", value),
            Self::NextBaseExperience(value) => write!(f, "Next Base Experience: {}", value),
            Self::NextJobExperience(value) => write!(f, "Next Job Experience: {}", value),
            Self::StrengthStatPointCost(value) => write!(f, "Strength Stat Point Cost: {}", value),
            Self::AgilityStatPointCost(value) => write!(f, "Agility Stat Point Cost: {}", value),
            Self::VitalityStatPointCost(value) => write!(f, "Vitality Stat Point Cost: {}", value),
            Self::IntelligenceStatPointCost(value) => write!(f, "Intelligence Stat Point Cost: {}", value),
            Self::DexterityStatPointCost(value) => write!(f, "Dexterity Stat Point Cost: {}", value),
            Self::LuckStatPointCost(value) => write!(f, "Luck Stat Point Cost: {}", value),
            Self::Strength(base, bonus) => write!(f, "Strength: {} (+{})", base, bonus),
            Self::Agility(base, bonus) => write!(f, "Agility: {} (+{})", base, bonus),
            Self::Vitality(base, bonus) => write!(f, "Vitality: {} (+{})", base, bonus),
            Self::Intelligence(base, bonus) => write!(f, "Intelligence: {} (+{})", base, bonus),
            Self::Dexterity(base, bonus) => write!(f, "Dexterity: {} (+{})", base, bonus),
            Self::Luck(base, bonus) => write!(f, "Luck: {} (+{})", base, bonus),
            Self::CartInfo(items, weight, max_weight) => write!(f, "Cart Info: {} items, {}/{} weight", items, weight, max_weight),
            Self::ActivityPoints(value) => write!(f, "Activity Points: {}", value),
            Self::TraitPoint(value) => write!(f, "Trait Point: {}", value),
            Self::MaximumActivityPoints(value) => write!(f, "Maximum Activity Points: {}", value),
            Self::Power(base, bonus) => write!(f, "Power: {} (+{})", base, bonus),
            Self::Stamina(base, bonus) => write!(f, "Stamina: {} (+{})", base, bonus),
            Self::Wisdom(base, bonus) => write!(f, "Wisdom: {} (+{})", base, bonus),
            Self::Spell(base, bonus) => write!(f, "Spell: {} (+{})", base, bonus),
            Self::Concentration(base, bonus) => write!(f, "Concentration: {} (+{})", base, bonus),
            Self::Creativity(base, bonus) => write!(f, "Creativity: {} (+{})", base, bonus),
            Self::PowerStatPointCost(value) => write!(f, "Power Stat Point Cost: {}", value),
            Self::StaminaStatPointCost(value) => write!(f, "Stamina Stat Point Cost: {}", value),
            Self::WisdomStatPointCost(value) => write!(f, "Wisdom Stat Point Cost: {}", value),
            Self::SpellStatPointCost(value) => write!(f, "Spell Stat Point Cost: {}", value),
            Self::ConcentrationStatPointCost(value) => write!(f, "Concentration Stat Point Cost: {}", value),
            Self::CreativitySpellPointCost(value) => write!(f, "Creativity Stat Point Cost: {}", value),
            Self::PhysicalAttack(value) => write!(f, "Physical Attack: {}", value),
            Self::SpellMagicAttack(value) => write!(f, "Spell Magic Attack: {}", value),
            Self::Resistance(value) => write!(f, "Resistance: {}", value),
            Self::MagicResistance(value) => write!(f, "Magic Resistance: {}", value),
            Self::HealingPlus(value) => write!(f, "Healing Plus: {}", value),
            Self::CriticalDamageRate(value) => write!(f, "Critical Damage Rate: {}", value),
        }
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_bytes::ByteWriter;

    use crate::{InventoryIndex, ItemId, PacketExt, WhisperPacket};

    /// The 2019 add-item packet (`ZC_ITEM_PICKUP_ACK` 0x0A37) has to match the
    /// server layout for PACKETVER 20190605 exactly: the equipment card slots
    /// are an `EQUIPSLOTINFO` with `uint32 card[4]` (16 bytes, PACKETVER >=
    /// 20181121 / RE >= 20180704), which makes the fixed packet 69 bytes long.
    /// A wrong size desynchronizes the packet stream and silently drops the
    /// inventory update (bought items only appeared after a relog).
    #[test]
    fn item_pickup_ack_packet_2019_layout() {
        use crate::{EquipPosition, ItemPickupAckPacket2019};

        let packet = ItemPickupAckPacket2019 {
            inventory_index: InventoryIndex(7),
            amount: 1,
            item_id: ItemId(2206),
            identified: 1,
            damaged: 0,
            refinement_level: 0,
            card: [0; 4],
            equip_position: EquipPosition::HEAD_TOP,
            item_type: 4,
            result: 0,
            hire_expiration_date: 0,
            bind_on_equip_type: 0,
            option_data: std::array::from_fn(|_| crate::ItemOptions {
                index: 0,
                value: 0,
                parameter: 0,
            }),
            favorite: 0,
            item_sprite_number: 44,
        };

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(bytes.len(), 69);
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0A37);
        // inventory index is sent as server index (index + 2).
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 9);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 1);
        assert_eq!(u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]), 2206);
    }

    /// 2019 `ZC_AUTORUN_SKILL` (0x0147): the server tells the client that an
    /// item wants to use a skill (fly wing -> AL_TELEPORT). Layout:
    /// `0147 <skill id>.W <type>.L <level>.W <sp>.W <range>.W <name>.24B
    /// <up>.B`.
    #[test]
    fn auto_run_skill_packet_layout() {
        use crate::AutoRunSkillPacket;

        let packet = AutoRunSkillPacket {
            skill_id: crate::SkillId(26),
            skill_type: 4,
            skill_level: 1,
            skill_sp: 10,
            skill_range: 1,
            skill_name: "Teleport".to_owned(),
            upgradable: 0,
        };

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(bytes.len(), 39);
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0147);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 26);
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 4);
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), 1);
        assert_eq!(&bytes[14..22], b"Teleport");
    }

    /// The whisper packet has to match the format the server expects:
    /// `0096 <len>.W <name>.24B <message>.?B 00`.
    #[test]
    fn whisper_packet_layout() {
        let packet = WhisperPacket::new("VerifyBot4".to_owned(), "hello".to_owned());

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0096);

        let packet_length = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
        assert_eq!(packet_length, bytes.len());
        assert_eq!(packet_length, 4 + 24 + "hello".len() + 1);

        assert_eq!(&bytes[4..14], b"VerifyBot4");
        assert_eq!(&bytes[28..34], b"hello\0");
    }

    #[test]
    fn shop_item_list_packet_layout() {
        // rAthena 2019 ZC_PC_PURCHASE_ITEMLIST: 00c6 <len>.W
        // { <price>.L <discount price>.L <item type>.B <item id>.L }*
        use crate::{Price, ShopItemInformation, ShopItemListPacket};

        let packet = ShopItemListPacket {
            items: vec![ShopItemInformation {
                price: Price(50),
                discount_price: Price(45),
                item_type: 0,
                item_id: ItemId(501),
            }],
        };

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x00C6);
        let length = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
        assert_eq!(length, bytes.len());
        assert_eq!(length, 4 + 13);
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 50);
        assert_eq!(u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]), 45);
        assert_eq!(bytes[12], 0);
        assert_eq!(u32::from_le_bytes([bytes[13], bytes[14], bytes[15], bytes[16]]), 501);
    }

    #[test]
    fn buy_items_packet_layout() {
        // rAthena 2019 CZ_PC_PURCHASE_ITEMLIST: 00c8 <len>.W
        // { <amount>.W <item id>.L }*
        use crate::{BuyItemInformation, BuyItemsPacket};

        let packet = BuyItemsPacket {
            items: vec![BuyItemInformation {
                amount: 2,
                item_id: ItemId(501),
            }],
        };

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x00C8);
        let length = u16::from_le_bytes([bytes[2], bytes[3]]) as usize;
        assert_eq!(length, bytes.len());
        assert_eq!(length, 4 + 6);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2);
        assert_eq!(u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]), 501);
    }

    #[test]
    fn buy_items_result_packet_layout() {
        // rAthena 2019 ZC_PC_PURCHASE_RESULT: 00ca <result>.B
        use crate::{BuyItemResult, BuyItemsResultPacket};

        let packet = BuyItemsResultPacket {
            result: BuyItemResult::Successful,
        };

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x00CA);
        assert_eq!(bytes.len(), 3);
        assert_eq!(bytes[2], 0);
    }

    #[test]
    fn edit_dialog_packets_layout() {
        use crate::{EntityId, InputEditDialogPacket, InputEditDialogStrPacket, OpenEditDialogPacket};

        let open = OpenEditDialogPacket::new(EntityId(42));
        let mut writer = ByteWriter::new();
        open.packet_to_bytes(&mut writer).unwrap();
        let bytes = writer.into_inner();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0142);
        assert_eq!(u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]), 42);
        assert_eq!(bytes.len(), 6);

        let numeric = InputEditDialogPacket::new(EntityId(42), -5);
        let mut writer = ByteWriter::new();
        numeric.packet_to_bytes(&mut writer).unwrap();
        let bytes = writer.into_inner();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0143);
        assert_eq!(u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]), 42);
        assert_eq!(i32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]), -5);
        assert_eq!(bytes.len(), 10);

        let text = InputEditDialogStrPacket::new(EntityId(42), "hi".to_owned());
        let mut writer = ByteWriter::new();
        text.packet_to_bytes(&mut writer).unwrap();
        let bytes = writer.into_inner();
        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x01D5);
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 42);
        assert_eq!(bytes[8], b'h');
        assert_eq!(bytes[9], b'i');
    }

    #[test]
    fn drop_item_packet_layout() {
        // rAthena CZ_ITEM_THROW at PACKETVER 20190605:
        // 00a2 <index>.W <amount>.W (clif.cpp:11994).
        use crate::DropItemPacket;

        let packet = DropItemPacket::new(InventoryIndex(7), 3);

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x00A2);
        // Inventory indices are stored as index + 2 by the client protocol.
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 9);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 3);
        assert_eq!(bytes.len(), 6);
    }

    #[test]
    fn use_item_packet_layout() {
        // rAthena CZ_USE_ITEM for modern clients: 0439 <index>.W <item id>.L
        use crate::UseItemPacket;

        let packet = UseItemPacket::new(InventoryIndex(0), ItemId(501));

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x0439);
        assert_eq!(bytes.len(), 8);
        // Inventory indices are stored as index + 2 by the client protocol.
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 2);
        assert_eq!(u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]), 501);
    }

    /// The 2019-era friend list packet (0x0201) has 8-byte entries: rAthena
    /// omits the 24-byte name since PACKETVER 20180307. Parsing it with the
    /// 2022-era 32-byte entry used to fail half way through the packet.
    #[test]
    fn friend_list_2019_layout() {
        use ragnarok_bytes::ByteReader;

        use crate::{AccountId, CharacterId, FriendListPacket, FriendListPacket2019};

        // 0201 <len>.W { <account id>.L <char id>.L }*
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0201u16.to_le_bytes());
        bytes.extend_from_slice(&12u16.to_le_bytes());
        bytes.extend_from_slice(&42u32.to_le_bytes());
        bytes.extend_from_slice(&150016u32.to_le_bytes());

        let mut byte_reader = ByteReader::without_metadata(&bytes);
        let packet = FriendListPacket2019::packet_from_bytes(&mut byte_reader).expect("2019 friend list must parse");
        assert!(byte_reader.is_empty(), "parsing must consume the whole packet");
        assert_eq!(packet.friend_list.len(), 1);
        assert_eq!(packet.friend_list[0].account_id, AccountId(42));
        assert_eq!(packet.friend_list[0].character_id, CharacterId(150016));

        // The 2022-era layout (32-byte entries with a name) must not be used for
        // this packet, it cannot be parsed from the 12 bytes the server sends.
        let mut byte_reader = ByteReader::without_metadata(&bytes);
        assert!(FriendListPacket::packet_from_bytes(&mut byte_reader).is_err());
    }

    #[test]
    fn add_friend_packet_layout() {
        // rAthena expects 0x086F for PACKETVER >= 20130320 (0x0202 is
        // CZ_CHANGE_DIRECTION there).
        use crate::AddFriendPacket;

        let packet = AddFriendPacket::new("VerifyBot4".to_owned());

        let mut byte_writer = ByteWriter::new();
        packet.packet_to_bytes(&mut byte_writer).unwrap();
        let bytes = byte_writer.into_inner();

        assert_eq!(u16::from_le_bytes([bytes[0], bytes[1]]), 0x086F);
        assert_eq!(bytes.len(), 2 + 24);
        assert_eq!(&bytes[2..12], b"VerifyBot4");
    }
}

/// Maximum number of guild positions in the 2019 client (`MAX_GUILDPOSITION`).
pub const MAX_GUILD_POSITIONS: usize = 20;

#[cfg(test)]
mod bank_packet_tests {
    use ragnarok_bytes::{ByteWriter, ToBytes};

    use super::*;

    trait PacketToBytesVec {
        fn packet_to_bytes_vec(&self) -> Vec<u8>;
    }

    impl<T: PacketExt> PacketToBytesVec for T {
        fn packet_to_bytes_vec(&self) -> Vec<u8> {
            let mut writer = ByteWriter::new();
            self.packet_to_bytes(&mut writer).unwrap();
            writer.into_inner()
        }
    }

    #[test]
    fn bank_requests_match_the_rathena_layout() {
        let open = BankOpenRequestPacket {
            account_id: AccountId(0x0102_0304),
        }
        .packet_to_bytes_vec();
        assert_eq!(open.len(), 6);
        assert_eq!(&open[..2], &[0xB6, 0x09]);
        assert_eq!(&open[2..6], &0x0102_0304u32.to_le_bytes());

        let deposit = BankDepositRequestPacket {
            account_id: AccountId(7),
            zeny: 1000,
        }
        .packet_to_bytes_vec();
        assert_eq!(deposit.len(), 10);
        assert_eq!(&deposit[..2], &[0xA7, 0x09]);
        assert_eq!(&deposit[2..6], &7u32.to_le_bytes());
        assert_eq!(&deposit[6..10], &1000u32.to_le_bytes());

        let withdraw = BankWithdrawRequestPacket {
            account_id: AccountId(7),
            zeny: 250,
        }
        .packet_to_bytes_vec();
        assert_eq!(withdraw.len(), 10);
        assert_eq!(&withdraw[..2], &[0xA9, 0x09]);
    }

    #[test]
    fn bank_acknowledgements_parse_the_server_layout() {
        // PACKET_ZC_ACK_BANKING_DEPOSIT: reason.W money.Q zeny.L
        let mut bytes = vec![0xA8, 0x09];
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&123_456u64.to_le_bytes());
        bytes.extend_from_slice(&7_890i32.to_le_bytes());

        let packet = BankDepositAckPacket::packet_from_bytes(&mut ByteReader::without_metadata(&bytes)).unwrap();
        assert_eq!(packet.reason, 1);
        assert_eq!(packet.money, 123_456);
        assert_eq!(packet.zeny, 7_890);

        // PACKET_ZC_BANKING_CHECK: money.Q reason.W
        let mut bytes = vec![0xA6, 0x09];
        bytes.extend_from_slice(&55u64.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());

        let packet = BankBalancePacket::packet_from_bytes(&mut ByteReader::without_metadata(&bytes)).unwrap();
        assert_eq!(packet.money, 55);
        assert_eq!(packet.reason, 0);
    }

    #[test]
    fn open_user_interface_packet_is_seven_bytes() {
        assert_eq!(OpenUserInterfacePacket { ui_type: 0, data: 0 }.packet_to_bytes_vec().len(), 7);
    }
}

#[cfg(test)]
mod content_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter, ToBytes};

    use super::*;

    #[test]
    fn achievement_wire_layouts_match_the_2019_server() {
        // CZ_REQ_ACH_REWARD: clif_packetdb.hpp:1769 registers 6 bytes and
        // clif_parse_AchievementCheckReward reads the id at offset 2.
        let mut writer = ByteWriter::new();
        AchievementRewardRequestPacket {
            achievement_id: 0x0102_0304,
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        let bytes = writer.into_inner();
        assert_eq!(bytes.len(), 6);
        assert_eq!(&bytes[..2], &[0x25, 0x0A]);
        assert_eq!(&bytes[2..6], &0x0102_0304u32.to_le_bytes());

        // ZC_REQ_ACH_REWARD_ACK: 7 bytes, result at 2, id at 3.
        let mut bytes = vec![0u8; 7];
        bytes[0..2].copy_from_slice(&0x0A26u16.to_le_bytes());
        bytes[2] = 1;
        bytes[3..7].copy_from_slice(&0x0102_0304u32.to_le_bytes());

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = AchievementRewardAckPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 7);
        assert_eq!(packet.result, 1);
        assert_eq!(packet.achievement_id, 0x0102_0304);
    }

    #[test]
    fn monster_info_parses_the_2019_wire_layout() {
        // 018c <class>.W <level>.W <size>.W <hp>.L <def>.W <race>.W
        //      <mdef>.W <element>.W { <resistance>.B } * 9
        let mut bytes = vec![0u8; 29];
        bytes[0..2].copy_from_slice(&0x018Cu16.to_le_bytes());
        bytes[2..4].copy_from_slice(&1002u16.to_le_bytes());
        bytes[4..6].copy_from_slice(&61u16.to_le_bytes());
        bytes[6..8].copy_from_slice(&1u16.to_le_bytes());
        bytes[8..12].copy_from_slice(&3211521u32.to_le_bytes());
        bytes[12..14].copy_from_slice(&12i16.to_le_bytes());
        bytes[14..16].copy_from_slice(&1u16.to_le_bytes());
        bytes[16..18].copy_from_slice(&7i16.to_le_bytes());
        bytes[18..20].copy_from_slice(&9u16.to_le_bytes());
        bytes[20..29].copy_from_slice(&[100, 90, 80, 70, 60, 50, 40, 30, 20]);

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = MonsterInfoPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 29);
        assert_eq!(packet.job_id, JobId(1002));
        assert_eq!(packet.level, 61);
        assert_eq!(packet.size, 1);
        assert_eq!(packet.health_points, 3211521);
        assert_eq!(packet.defense, 12);
        assert_eq!(packet.race, 1);
        assert_eq!(packet.magic_defense, 7);
        assert_eq!(packet.element, 9);
        assert_eq!(packet.water, 100);
        assert_eq!(packet.undead, 20);
    }
}

#[cfg(test)]
mod item_service_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use crate::{
        CardCompositionListPacket, CardCompositionListRequestPacket, CardCompositionRequestPacket, CardCompositionResultPacket,
        CloseRefiningUiRequestPacket, IdentifyItemListPacket, IdentifyItemRequestPacket, IdentifyItemResultPacket, InventoryIndex, ItemId,
        ItemRefiningResultPacket, OneClickIdentifyRequestPacket, OpenRefiningUiPacket, PacketExt, RefiningMaterialListPacket,
        RefiningRequestPacket, RefiningSelectItemRequestPacket, RepairItemListPacket, RepairItemRequestPacket, RepairItemResultPacket,
        UpdateCardSlotPacket, WeaponRefineListPacket, WeaponRefineRequestPacket, WeaponRefineResultPacket,
    };

    fn bytes<T: PacketExt>(packet: &T) -> Vec<u8> {
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        writer.into_inner()
    }

    /// `ZC_NOTIFY_WEAPONITEMLIST` rows are 23 bytes at PACKETVER 20190605:
    /// `{ index }.W { item id }.L { refine }.B { card[4] }.L`
    /// (`clif_item_refine_list`, `clif.cpp:7209`).
    #[test]
    fn weapon_refine_list_2019_layout() {
        let mut payload = Vec::new();
        payload.push(0x21);
        payload.push(0x02);
        payload.extend_from_slice(&(4u16 + 23).to_le_bytes());
        payload.extend_from_slice(&(7u16 + 2).to_le_bytes());
        payload.extend_from_slice(&1500u32.to_le_bytes());
        payload.push(4);
        payload.extend_from_slice(&[0u8; 16]);

        let mut reader = ByteReader::without_metadata(&payload);
        let packet = WeaponRefineListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), payload.len());
        assert_eq!(packet.entries.len(), 1);
        assert_eq!(packet.entries[0].index, InventoryIndex(7));
        assert_eq!(packet.entries[0].item_id, ItemId(1500));
        assert_eq!(packet.entries[0].refine, 4);
        assert_eq!(packet.entries[0].card, [0; 4]);
    }

    /// `ZC_ACK_WEAPONREFINE`: `{ result }.L { item id }.L`, 10 bytes
    /// (`clif_upgrademessage`, `clif.cpp:6937`).
    #[test]
    fn weapon_refine_result_2019_layout() {
        let packet = WeaponRefineResultPacket {
            result: 0,
            item_id: ItemId(1500),
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 10);
        assert_eq!(&bytes[..2], &[0x23, 0x02]);
        assert_eq!(&bytes[2..6], &0i32.to_le_bytes());
        assert_eq!(&bytes[6..10], &1500u32.to_le_bytes());
    }

    /// `ZC_ACK_ITEMREFINING`: `{ result }.W { index }.W { refine }.W`, 8 bytes.
    #[test]
    fn item_refining_result_2019_layout() {
        let packet = ItemRefiningResultPacket {
            result: 0,
            index: InventoryIndex(3),
            refine: 5,
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 8);
        assert_eq!(&bytes[..2], &[0x88, 0x01]);
        assert_eq!(&bytes[2..4], &0u16.to_le_bytes());
        assert_eq!(&bytes[4..6], &(3u16 + 2).to_le_bytes());
        assert_eq!(&bytes[6..8], &5u16.to_le_bytes());
    }

    /// `ZC_REPAIRITEMLIST`: the row index is the raw server slot (no +2),
    /// `clif_item_repair_list` writes `i` directly (`clif.cpp:7153`).
    #[test]
    fn repair_item_list_2019_layout() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0x01FCu16.to_le_bytes());
        payload.extend_from_slice(&(4u16 + 23).to_le_bytes());
        payload.extend_from_slice(&9u16.to_le_bytes());
        payload.extend_from_slice(&1500u32.to_le_bytes());
        payload.push(6);
        payload.extend_from_slice(&[0u8; 16]);

        let mut reader = ByteReader::without_metadata(&payload);
        let packet = RepairItemListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), payload.len());
        assert_eq!(packet.entries[0].index, 9, "raw server slot, no +2 encoding");
        assert_eq!(packet.entries[0].item_id, ItemId(1500));
        assert_eq!(packet.entries[0].refine, 6);
    }

    /// `CZ_REQ_ITEMREPAIR1` is 25 bytes and the index is the raw server slot
    /// (`PACKET_CZ_REQ_ITEMREPAIR1`, `packets_struct.hpp:2944`).
    #[test]
    fn repair_item_request_2019_layout() {
        let packet = RepairItemRequestPacket {
            index: 9,
            item_id: ItemId(1500),
            refine: 6,
            card: [0; 4],
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 25);
        assert_eq!(&bytes[..2], &[0xFD, 0x01]);
        assert_eq!(&bytes[2..4], &9u16.to_le_bytes(), "raw server slot");
        assert_eq!(&bytes[4..8], &1500u32.to_le_bytes());
        assert_eq!(bytes[8], 6);
    }

    /// `ZC_ACK_ITEMREPAIR`: `{ index }.W { result }.B` with the +2 client
    /// index (`clif.cpp:7186`).
    #[test]
    fn repair_item_result_2019_layout() {
        let packet = RepairItemResultPacket {
            index: InventoryIndex(9),
            result: 0,
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 5);
        assert_eq!(&bytes[..2], &[0xFE, 0x01]);
        assert_eq!(&bytes[2..4], &(9u16 + 2).to_le_bytes());
        assert_eq!(bytes[4], 0);
    }

    /// `ZC_ITEMIDENTIFY_LIST` is `{ index }.W` rows of +2 client indexes
    /// (`clif.cpp:7111`).
    #[test]
    fn identify_item_list_2019_layout() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0x0177u16.to_le_bytes());
        payload.extend_from_slice(&(4u16 + 4).to_le_bytes());
        payload.extend_from_slice(&(2u16 + 2).to_le_bytes());
        payload.extend_from_slice(&(5u16 + 2).to_le_bytes());

        let mut reader = ByteReader::without_metadata(&payload);
        let packet = IdentifyItemListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), payload.len());
        assert_eq!(packet.indexes, vec![InventoryIndex(2), InventoryIndex(5)]);
    }

    /// `CZ_REQ_ITEMIDENTIFY` (0x0178) and `CZ_REQ_ONECLICK_ITEMIDENTIFY`
    /// (0x0A35) are both 4 bytes with a +2 client index.
    #[test]
    fn identify_request_2019_layout() {
        let identify = bytes(&IdentifyItemRequestPacket { index: InventoryIndex(5) });
        assert_eq!(identify.len(), 4);
        assert_eq!(&identify[..2], &[0x78, 0x01]);
        assert_eq!(&identify[2..4], &(5u16 + 2).to_le_bytes());

        let one_click = bytes(&OneClickIdentifyRequestPacket { index: InventoryIndex(5) });
        assert_eq!(one_click.len(), 4);
        assert_eq!(&one_click[..2], &[0x35, 0x0A]);
        assert_eq!(&one_click[2..4], &(5u16 + 2).to_le_bytes());
    }

    /// `ZC_ACK_ITEMIDENTIFY`: `{ index }.W { result }.B`, 5 bytes.
    #[test]
    fn identify_item_result_2019_layout() {
        let packet = IdentifyItemResultPacket {
            index: InventoryIndex(5),
            result: 0,
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 5);
        assert_eq!(&bytes[..2], &[0x79, 0x01]);
        assert_eq!(&bytes[2..4], &(5u16 + 2).to_le_bytes());
        assert_eq!(bytes[4], 0);
    }

    /// `ZC_ITEMCOMPOSITION_LIST` rows are +2 client indexes (`clif.cpp:7068`).
    #[test]
    fn card_composition_list_2019_layout() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0x017Bu16.to_le_bytes());
        payload.extend_from_slice(&(4u16 + 2).to_le_bytes());
        payload.extend_from_slice(&(11u16 + 2).to_le_bytes());

        let mut reader = ByteReader::without_metadata(&payload);
        let packet = CardCompositionListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), payload.len());
        assert_eq!(packet.indexes, vec![InventoryIndex(11)]);
    }

    /// The card composition requests use the +2 client index encoding
    /// (`clif.cpp:13468`/`13490`).
    #[test]
    fn card_composition_request_2019_layout() {
        let list = bytes(&CardCompositionListRequestPacket { index: InventoryIndex(11) });
        assert_eq!(list.len(), 4);
        assert_eq!(&list[..2], &[0x7A, 0x01]);
        assert_eq!(&list[2..4], &(11u16 + 2).to_le_bytes());

        let insert = bytes(&CardCompositionRequestPacket {
            card_index: InventoryIndex(4),
            equip_index: InventoryIndex(11),
        });
        assert_eq!(insert.len(), 6);
        assert_eq!(&insert[..2], &[0x7C, 0x01]);
        assert_eq!(&insert[2..4], &(4u16 + 2).to_le_bytes());
        assert_eq!(&insert[4..6], &(11u16 + 2).to_le_bytes());
    }

    /// `ZC_ACK_ITEMCOMPOSITION`: `{ equip }.W { card }.W { result }.B`, 7
    /// bytes.
    #[test]
    fn card_composition_result_2019_layout() {
        let packet = CardCompositionResultPacket {
            equip_index: InventoryIndex(11),
            card_index: InventoryIndex(4),
            result: 0,
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 7);
        assert_eq!(&bytes[..2], &[0x7D, 0x01]);
        assert_eq!(&bytes[2..4], &(11u16 + 2).to_le_bytes());
        assert_eq!(&bytes[4..6], &(4u16 + 2).to_le_bytes());
        assert_eq!(bytes[6], 0);
    }

    /// `CZ_REQ_WEAPONREFINE` is 6 bytes: a 32 bit +2 client index follows the
    /// header (`clif_packetdb.hpp:472`, `clif.cpp:13262`).
    #[test]
    fn weapon_refine_request_2019_layout() {
        let packet = WeaponRefineRequestPacket::for_inventory_index(InventoryIndex(7));
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 6);
        assert_eq!(&bytes[..2], &[0x22, 0x02]);
        assert_eq!(&bytes[2..6], &(9u32).to_le_bytes());
    }

    /// `ZC_REFINING_MATERIAL_LIST` (0x0AA2): `{ item index }.W
    /// { blessing }.b` followed by 9 byte material rows (`clif.cpp:22504`).
    #[test]
    fn refining_material_list_2019_layout() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&0x0AA2u16.to_le_bytes());
        payload.extend_from_slice(&(7u16 + 9).to_le_bytes());
        payload.extend_from_slice(&(4u16 + 2).to_le_bytes()); // client index
        payload.push(2); // blacksmith blessings
        payload.extend_from_slice(&984u32.to_le_bytes()); // material item id
        payload.push(75); // chance
        payload.extend_from_slice(&5000i32.to_le_bytes()); // zeny

        let mut reader = ByteReader::without_metadata(&payload);
        let packet = RefiningMaterialListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), payload.len());
        assert_eq!(packet.item_index, InventoryIndex(4));
        assert_eq!(packet.blacksmith_blessing, 2);
        assert_eq!(packet.materials.len(), 1);
        assert_eq!(packet.materials[0].item_id, ItemId(984));
        assert_eq!(packet.materials[0].chance, 75);
        assert_eq!(packet.materials[0].zeny, 5000);
    }

    /// The refine UI requests: 0x0AA0/0x0AA4 are 2 bytes, 0x0AA1 4 bytes and
    /// 0x0AA3 9 bytes (`clif.cpp:22475`-`22599`).
    #[test]
    fn refining_ui_requests_2019_layout() {
        assert_eq!(bytes(&OpenRefiningUiPacket {}), vec![0xA0, 0x0A]);
        assert_eq!(bytes(&CloseRefiningUiRequestPacket {}), vec![0xA4, 0x0A]);

        let select = bytes(&RefiningSelectItemRequestPacket { index: InventoryIndex(4) });
        assert_eq!(select.len(), 4);
        assert_eq!(&select[..2], &[0xA1, 0x0A]);
        assert_eq!(&select[2..4], &(4u16 + 2).to_le_bytes());

        let refine = bytes(&RefiningRequestPacket {
            index: InventoryIndex(4),
            material_item_id: ItemId(984),
            blacksmith_blessing: 0,
        });
        assert_eq!(refine.len(), 9);
        assert_eq!(&refine[..2], &[0xA3, 0x0A]);
        assert_eq!(&refine[2..4], &(4u16 + 2).to_le_bytes());
        assert_eq!(&refine[4..8], &984u32.to_le_bytes());
        assert_eq!(refine[8], 0);
    }

    /// `ZC_UPDATE_CARDSLOT` is 11 bytes at PACKETVER 20190605.
    #[test]
    fn update_card_slot_2019_layout() {
        let packet = UpdateCardSlotPacket {
            wear_state: 1,
            card_slot: 2,
            item_id: ItemId(4001),
            equip_flag: 0,
        };
        let bytes = bytes(&packet);
        assert_eq!(bytes.len(), 11);
        assert_eq!(&bytes[..2], &[0x3F, 0x0A]);
        assert_eq!(&bytes[2..4], &1i16.to_le_bytes());
        assert_eq!(&bytes[4..6], &2i16.to_le_bytes());
        assert_eq!(&bytes[6..10], &4001u32.to_le_bytes());
    }
}

// ===========================================================================
// NPC image / book, chat room and pincode packets (PACKETVER 20190605)
//
// The 2019 layouts below are taken from rAthena:
//
// * `ZC_SHOW_IMAGE2` 0x01B3 — `clif_cutin` (clif.cpp:2719-2730),
// * `ZC_READ_BOOK` 0x0294 — `clif_readbook` (clif.cpp:18354-18361),
// * chat room family 0x00D5..0x00E3 — `clif.cpp:4469-4680` and
//   `clif_packetdb.hpp:85-92`,
// * `HC_ACK_PINCODE` 0x08B9 (already declared as [`LoginPincodePacket`]) and
//   the client answers 0x08B8/0x08BA — `char_clif.cpp:146-213`.
// ===========================================================================

#[cfg(test)]
mod content_window_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;

    trait PacketToBytesVec {
        fn packet_to_bytes_vec(&self) -> Vec<u8>;
    }

    impl<T: PacketExt> PacketToBytesVec for T {
        fn packet_to_bytes_vec(&self) -> Vec<u8> {
            let mut writer = ByteWriter::new();
            self.packet_to_bytes(&mut writer).unwrap();
            writer.into_inner()
        }
    }

    #[test]
    fn read_book_is_the_hand_written_ten_byte_layout() {
        let bytes = ReadBookPacket { book_id: 11017, page: 2 }.packet_to_bytes_vec();

        assert_eq!(bytes.len(), 10);
        assert_eq!(&bytes[..2], &[0x94, 0x02]);
        assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 11017);
        assert_eq!(u32::from_le_bytes(bytes[6..10].try_into().unwrap()), 2);
    }

    #[test]
    fn create_chat_room_matches_the_2019_wire_layout() {
        let bytes = CreateChatRoomPacket {
            limit: 20,
            room_type: 1,
            password: String::new(),
            title: "Trade".to_owned(),
        }
        .packet_to_bytes_vec();

        // 2 header + 2 size + 2 limit + 1 type + 8 password + 5 title + the
        // trailing NUL of the variable title. rAthena only requires
        // `packetSize >= sizeof(PACKET_CZ_CREATE_CHATROOM)` (15) and copies
        // `packetSize - 15 + 1` bytes, so the NUL is accepted (`clif.cpp:12335`).
        assert_eq!(bytes.len(), 21);
        assert_eq!(&bytes[..2], &[0xD5, 0x00]);
        assert_eq!(u16::from_le_bytes(bytes[2..4].try_into().unwrap()) as usize, bytes.len());
        assert_eq!(u16::from_le_bytes(bytes[4..6].try_into().unwrap()), 20);
        assert_eq!(bytes[6], 1);
        assert_eq!(&bytes[7..15], &[0; 8]);
        assert_eq!(&bytes[15..20], b"Trade");
        assert_eq!(bytes[20], 0);
    }

    #[test]
    fn enter_and_exit_chat_room_use_the_2019_sizes() {
        let enter = EnterChatRoomRequestPacket {
            chat_id: 0x0102_0304,
            password: String::new(),
        }
        .packet_to_bytes_vec();
        assert_eq!(enter.len(), 14);
        assert_eq!(&enter[..2], &[0xD9, 0x00]);
        assert_eq!(u32::from_le_bytes(enter[2..6].try_into().unwrap()), 0x0102_0304);
        assert_eq!(&enter[6..14], &[0; 8]);

        let exit = ExitChatRoomPacket {}.packet_to_bytes_vec();
        assert_eq!(exit, [0xE3, 0x00]);
    }

    #[test]
    fn chat_room_new_entry_parses_owner_limit_and_title() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x00D7u16.to_le_bytes());
        // Size placeholder, patched below: 17 fixed bytes + title.
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&42u32.to_le_bytes()); // owner
        bytes.extend_from_slice(&7u32.to_le_bytes()); // chat id
        bytes.extend_from_slice(&20u16.to_le_bytes()); // limit
        bytes.extend_from_slice(&3u16.to_le_bytes()); // users
        bytes.push(1); // public
        bytes.extend_from_slice(b"Party");
        let size = bytes.len() as u16;
        bytes[2..4].copy_from_slice(&size.to_le_bytes());

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = ChatRoomNewEntryPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(packet.owner_id, AccountId(42));
        assert_eq!(packet.chat_id, 7);
        assert_eq!(packet.limit, 20);
        assert_eq!(packet.users, 3);
        assert_eq!(packet.room_type, 1);
        assert_eq!(packet.title, "Party");
    }

    #[test]
    fn enter_chat_room_parses_the_member_rows() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x00DBu16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&9u32.to_le_bytes()); // chat id

        for (role, name) in [(0u32, "Owner"), (1u32, "Guest")] {
            bytes.extend_from_slice(&role.to_le_bytes());
            let mut name_bytes = [0u8; 24];
            name_bytes[..name.len()].copy_from_slice(name.as_bytes());
            bytes.extend_from_slice(&name_bytes);
        }

        let size = bytes.len() as u16;
        bytes[2..4].copy_from_slice(&size.to_le_bytes());

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = EnterChatRoomPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(packet.chat_id, 9);
        assert_eq!(packet.members.len(), 2);
        assert_eq!(packet.members[0].role, 0);
        assert_eq!(packet.members[0].name, "Owner");
        assert_eq!(packet.members[1].role, 1);
        assert_eq!(packet.members[1].name, "Guest");
    }

    #[test]
    fn pincode_requests_match_char_clif_layout() {
        let check = PincodeCheckPacket {
            account_id: AccountId(0x1122_3344),
            pin: [b'1', b'2', b'3', b'4'],
        }
        .packet_to_bytes_vec();

        assert_eq!(check.len(), 10);
        assert_eq!(&check[..2], &[0xB8, 0x08]);
        assert_eq!(u32::from_le_bytes(check[2..6].try_into().unwrap()), 0x1122_3344);
        assert_eq!(&check[6..10], b"1234");

        let set_new = PincodeSetNewPacket {
            account_id: AccountId(1),
            pin: [b'0'; 4],
        }
        .packet_to_bytes_vec();
        assert_eq!(set_new.len(), 10);
        assert_eq!(&set_new[..2], &[0xBA, 0x08]);
    }
}

#[cfg(test)]
mod companion_and_booking_packet_tests {
    use ragnarok_bytes::ByteWriter;

    use super::*;

    fn packet_len(packet: &impl PacketExt) -> usize {
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        writer.as_slice().len()
    }

    #[test]
    fn party_booking_client_packets_match_the_2019_wire_sizes() {
        assert_eq!(
            packet_len(&PartyBookingRegisterRequestPacket2019 {
                level: 99,
                map_id: 0,
                jobs: [u16::MAX; 6],
            }),
            18
        );
        assert_eq!(
            packet_len(&PartyBookingSearchRequestPacket2019 {
                level: 0,
                map_id: 0,
                job: -1,
                last_index: 0,
                result_count: 10,
            }),
            14
        );
        assert_eq!(packet_len(&PartyBookingUpdateRequestPacket2019 { jobs: [0; 6] }), 14);
        assert_eq!(packet_len(&PartyBookingDeleteRequestPacket2019 {}), 2);
        assert_eq!(
            packet_len(&PartyBookingJoinRequestPacket {
                character_id: CharacterId(1),
                account_id: AccountId(2),
            }),
            10
        );
        assert_eq!(
            packet_len(&PartyBookingJoinReplyPacket {
                character_id: CharacterId(1),
                account_id: AccountId(2),
                accept: 1,
            }),
            11
        );
        assert_eq!(
            packet_len(&PartyOptionRequestPacket2019 {
                experience_option: 1,
                item_pickup: 1,
                item_share: 0,
            }),
            8
        );
        assert_eq!(packet_len(&ChangePartyLeaderPacket { account_id: AccountId(2) }), 6);
    }

    #[test]
    fn party_booking_server_rows_are_48_bytes() {
        // The insert notification is header (2) + one row (48).
        assert_eq!(
            packet_len(&PartyBookingInsertNotifyPacket2019 {
                entry: PartyBookingEntry2019 {
                    index: 0,
                    character_name: String::new(),
                    expire_time: 0,
                    level: 0,
                    map_id: 0,
                    jobs: [0; 6],
                },
            }),
            50
        );
    }

    #[test]
    fn companion_packets_match_the_2019_wire_sizes() {
        assert_eq!(
            packet_len(&PetPropertyPacket2019 {
                name: String::new(),
                renamed: 0,
                level: 0,
                hunger: 0,
                intimacy: 0,
                accessory: 0,
                job_id: JobId(1002),
            }),
            37
        );
        assert_eq!(
            packet_len(&PetStateChangePacket2019 {
                state_type: 1,
                entity_id: EntityId(1),
                value: 0,
            }),
            11
        );
        assert_eq!(
            packet_len(&HomunculusPropertyPacket2019 {
                name: String::new(),
                flags: 0,
                level: 0,
                hunger: 0,
                intimacy: 0,
                item_id: 0,
                attack: 0,
                magic_attack: 0,
                hit: 0,
                critical: 0,
                defense: 0,
                magic_defense: 0,
                flee: 0,
                attack_motion: 0,
                health_points: 0,
                maximum_health_points: 0,
                spell_points: 0,
                maximum_spell_points: 0,
                experience: 0,
                maximum_experience: 0,
                skill_points: 0,
                attack_range: 0,
            }),
            77
        );
        assert_eq!(packet_len(&HomunculusParameterChangePacket2019 { parameter: 5, value: 0 }), 8);
        assert_eq!(
            packet_len(&MercenaryInfoPacket2019 {
                entity_id: EntityId(1),
                attack: 0,
                magic_attack: 0,
                hit: 0,
                critical: 0,
                defense: 0,
                magic_defense: 0,
                flee: 0,
                attack_motion: 0,
                name: String::new(),
                level: 0,
                health_points: 0,
                maximum_health_points: 0,
                spell_points: 0,
                maximum_spell_points: 0,
                expire_time: 0,
                faith: 0,
                calls: 0,
                kills: 0,
                attack_range: 0,
            }),
            80
        );
        assert_eq!(packet_len(&MercenaryParameterChangePacket2019 { parameter: 5, value: 0 }), 8);
    }
}
