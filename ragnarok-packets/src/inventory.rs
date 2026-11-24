//! inventory packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B08)]
#[variable_length]
pub struct InventoyStartPacket {
    pub inventory_type: u8,
    #[length_remaining]
    pub inventory_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B0B)]
#[packet_size(4)]
pub struct InventoyEndPacket {
    pub inventory_type: u8,
    pub flag: u8, // maybe char ?
}

/// 2019 era `ZC_ADD_ITEM_TO_STORE` (0x0A0A): an item was added to the Kafra
/// storage. The index is the storage slot encoded as `server slot + 1`.
///
/// The 2019 layout has the refinement level *before* the card slots (the 2022
/// layouts moved it and added a grade byte); 57 bytes total.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A0A)]
#[packet_size(57)]
pub struct StorageItemAddedPacket2019 {
    pub storage_index: u16,
    pub amount: i32,
    pub item_id: ItemId,
    pub item_type: u8,
    pub identified: u8,
    pub damaged: u8,
    pub refinement_level: u8,
    pub card: [u32; 4],
    pub option_data: [ItemOptions; 5],
}

/// 2019 era `ZC_CLOSE_STORE` (0x00F8): the storage (or cart) window was closed
/// by the server, e.g. because the player walked away from the Kafra.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F8)]
#[packet_size(2)]
pub struct StorageClosedPacket {}

/// 2019 `CZ_MOVE_ITEM_FROM_BODY_TO_STORE` (0x07EC for PACKETVER 2019): move an
/// item from the inventory into the Kafra storage. The inventory index uses the
/// usual `+2` wire encoding of [InventoryIndex].
///
/// Note: the 0x00F3 opcode that older clients used for this is mapped to the
/// global chat message handler on the 2019 server, so sending 0x00F3 gets the
/// sender kicked with "malformed packet (no username data)".
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x07EC)]
pub struct StorageDepositPacket {
    pub inventory_index: InventoryIndex,
    pub amount: u32,
}

/// 2019 `CZ_MOVE_ITEM_FROM_STORE_TO_BODY` (0x0861 for PACKETVER 2019): move an
/// item out of the Kafra storage. The server expects the storage slot encoded
/// as `server slot + 1`, so callers have to add one to the slot from the
/// storage list.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0861)]
pub struct StorageWithdrawPacket {
    pub storage_index: u16,
    pub amount: u32,
}

/// 2019 `CZ_CLOSE_STORE` (0x0193 for PACKETVER 2019): close the Kafra storage
/// window. The 0x00F7 opcode that older clients used for this is mapped to the
/// trade item removal handler on the 2019 server.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0193)]
pub struct CloseStoragePacket {}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct ItemOptions {
    pub index: u16,
    pub value: u16,
    pub parameter: u8,
}

bitflags::bitflags! {
    /// Flags of a regular (stackable) item row (`NORMALITEM_INFO`).
    ///
    /// For PACKETVER 2019 the server fills the second flag bit with the
    /// *favorite* mark of the item (`clif.cpp`: `Flag.PlaceETCTab =
    /// it->favorite ? 1 : 0`), which is what the official client displays as
    /// the item's star.
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
    pub struct RegularItemFlags: u8 {
        const IDENTIFIED = 0b01;
        const FAVORITE = 0b10;
    }
}

impl FixedByteSize for RegularItemFlags {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for RegularItemFlags {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader)
            // Unknown flag bits are ignored instead of panicking: the server may
            // legitimately send flags this client does not know about yet, and a
            // malformed packet must never take the connection down.
            .map(Self::from_bits_truncate)
    }
}

impl ToBytes for RegularItemFlags {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.bits().to_bytes(byte_writer)
    }
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct RegularItemInformation {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub amount: u16,
    pub equipped_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub flags: RegularItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B09)]
#[variable_length]
pub struct RegularItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<RegularItemInformation>,
}

bitflags::bitflags! {
    /// Flags of an equippable item row (`EQUIPITEM_INFO`).
    ///
    /// The 2019 server sends the *favorite* mark in the third flag bit
    /// (`clif.cpp`: `Flag.PlaceETCTab = it->favorite ? 1 : 0`).
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
    pub struct EquippableItemFlags: u8 {
        const IDENTIFIED = 0b001;
        const IS_BROKEN = 0b010;
        const FAVORITE = 0b100;
    }
}

impl FixedByteSize for EquippableItemFlags {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for EquippableItemFlags {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader)
            // Unknown flag bits are ignored instead of panicking: the server may
            // legitimately send flags this client does not know about yet, and a
            // malformed packet must never take the connection down.
            .map(Self::from_bits_truncate)
    }
}

impl ToBytes for EquippableItemFlags {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.bits().to_bytes(byte_writer)
    }
}

/// 2019-era layout of one row of the equippable item list
/// (`ZC_INVENTORY_LIST`, rAthena header `0x0B0A`, `PACKETVER` between
/// 2018-11-21 and 2020-09-16).
///
/// It differs from [EquippableItemInformation] (the 2022 layout used by
/// [`EquippableItemListPacket`]): the refinement level comes *before* the card
/// slots and there is no enchantment/grade byte at the end, which makes one row
/// 67 bytes instead of 68.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct EquippableItemInformation2019 {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    pub refinement_level: u8,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8,
    pub option_data: [ItemOptions; 5], // fix count
    pub flags: EquippableItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B0A)]
#[variable_length]
pub struct EquippableItemListPacket2019 {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation2019>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct EquippableItemInformation {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8,
    pub option_data: [ItemOptions; 5], // fix count
    pub refinement_level: u8,
    pub enchantment_level: u8,
    pub flags: EquippableItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B39)]
#[variable_length]
pub struct EquippableItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct EquippableSwitchItemInformation {
    pub index: InventoryIndex,
    pub position: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A9B)]
#[variable_length]
pub struct EquippableSwitchItemListPacket {
    #[repeating_remaining]
    pub item_information: Vec<EquippableSwitchItemInformation>,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0362)]
pub struct ItemPickupRequestPacket {
    pub entity_id: EntityId,
}

/// Use an item from the inventory (CZ_USE_ITEM).
///
/// Packet layout for PACKETVER >= 20080910:
/// `0439 <index>.W <item id>.L`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0439)]
pub struct UseItemPacket {
    pub inventory_index: InventoryIndex,
    pub item_id: ItemId,
}

/// Drop an item from the inventory on the ground (CZ_ITEM_THROW).
///
/// Packet layout for PACKETVER 20190605 (rAthena `clif.cpp:11994`):
/// `00a2 <index>.W <amount>.W`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00A2)]
pub struct DropItemPacket {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

/// Result of using an item (ZC_USE_ITEM_ACK).
///
/// Packet layout for PACKETVER >= 20181121:
/// `01c8 <index>.W <item id>.L <account id>.L <amount>.W <result>.B`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01C8)]
#[packet_size(15)]
pub struct UseItemAckPacket {
    pub inventory_index: InventoryIndex,
    pub item_id: ItemId,
    pub account_id: AccountId,
    pub amount: i16,
    pub result: u8,
}

/// 2019 era `ZC_NOTIFY_CARTITEM_COUNTINFO` (0x0121): cart item/weight counts.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0121)]
#[packet_size(14)]
pub struct CartItemCountInfoPacket2019 {
    pub current_count: i16,
    pub maximum_count: i16,
    pub current_weight: i32,
    pub maximum_weight: i32,
}

/// 2019 era `ZC_NOTIFY_STOREITEM_COUNTINFO` (0x00F2): storage item count.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F2)]
#[packet_size(6)]
pub struct StorageItemCountInfoPacket2019 {
    pub amount: u16,
    pub maximum_amount: u16,
}

/// 2019 era `ZC_DELETE_ITEM_FROM_STORE` (0x00F6): an item was removed from
/// storage.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F6)]
#[packet_size(8)]
pub struct StorageDeleteItemPacket2019 {
    pub storage_index: u16,
    pub amount: u32,
}

/// 2019 era `ZC_ITEM_PICKUP_ACK` (0x0A37): acknowledgement of an item pickup.
///
/// The client does not show the pickup message yet, but every pickup sends this
/// 69 byte packet and an unknown packet used to drop the rest of the TCP read.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A37)]
#[packet_size(69)]
pub struct ItemPickupAckPacket2019 {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
    pub item_id: ItemId,
    pub identified: u8,
    pub damaged: u8,
    pub refinement_level: u8,
    /// `EQUIPSLOTINFO` of the server: `{ uint32 card[4]; }` (16 bytes) for
    /// PACKETVER >= 20181121, so the whole packet is 69 bytes.
    pub card: [u32; 4],
    pub equip_position: EquipPosition,
    pub item_type: u8,
    pub result: u8,
    pub hire_expiration_date: i32,
    pub bind_on_equip_type: u16,
    pub option_data: [ItemOptions; 5],
    pub favorite: u8,
    pub item_sprite_number: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02DA)]
#[packet_size(3)]
pub struct UpdateShowEquipPacket {
    pub open_equip_window: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x009D)]
#[packet_size(19)]
pub struct GroundItemAppearPacket {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub position: TilePosition,
    pub quantity: u16,
    pub x_offset: u8,
    pub y_offset: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x009E)]
pub struct GroundItemAppear2Packet {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub position: TilePosition,
    pub x_offset: u8,
    pub y_offset: u8,
    pub quantity: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x084B)]
pub struct GroundItemAppear3Packet {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub item_type: u16,
    pub is_identified: u8,
    pub position: TilePosition,
    pub x_offset: u8,
    pub y_offset: u8,
    pub quantity: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0ADD)]
#[packet_size(24)]
pub struct GroundItemAppear4Packet {
    pub entity_id: EntityId,
    pub item_id: ItemId,
    pub item_type: u16,
    pub is_identified: u8,
    pub position: TilePosition,
    pub x_offset: u8,
    pub y_offset: u8,
    pub quantity: u16,
    pub show_drop_effect: u8,
    pub drop_effect_mode: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00A1)]
#[packet_size(6)]
pub struct ItemDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum ItemPickupResult {
    Success,
    Invalid,
    Overweight,
    Unknown0,
    NoSpace,
    MaximumOfItem,
    Unknown1,
    StackLimitation,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B41)]
pub struct ItemPickupPacket {
    pub index: InventoryIndex,
    pub quantity: u16,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub is_broken: u8,
    pub cards: [u32; 4],
    pub equip_position: EquipPosition,
    pub item_type: u8,
    pub result: ItemPickupResult,
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub option_data: [ItemOptions; 5], // fix count
    pub favorite: u8,
    pub look: u16,
    pub refinement_level: u8,
    pub enchantment_level: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum RemoveItemReason {
    Normal,
    ItemUsedForSkill,
    RefineFailed,
    MaterialChanged,
    MovedToStorage,
    MovedToCart,
    ItemSold,
    ConsumedByFourSpiritAnalysis,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x07FA)]
#[packet_size(8)]
pub struct RemoveItemFromInventoryPacket {
    pub remove_reason: RemoveItemReason,
    pub index: InventoryIndex,
    pub amount: u16,
}

impl FixedByteSize for EquipPosition {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for EquipPosition {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_reader)
            // Unknown flag bits are ignored instead of panicking: the server may
            // legitimately send flags this client does not know about yet, and a
            // malformed packet must never take the connection down.
            .map(Self::from_bits_truncate)
    }
}

impl ToBytes for EquipPosition {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.bits().to_bytes(byte_writer)
    }
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0998)]
pub struct RequestEquipItemPacket {
    pub inventory_index: InventoryIndex,
    pub equip_position: EquipPosition,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum RequestEquipItemStatus {
    #[numeric_value(0)]
    Success,
    /// The server rejected the equip because the required level is not reached.
    #[numeric_value(1)]
    FailedDueToLevelRequirement,
    /// The server rejected the equip for any other reason (class, job, slot,
    /// requirements, ...).
    #[numeric_value(2)]
    Failed,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0999)]
#[packet_size(11)]
pub struct RequestEquipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub view_id: u16,
    pub result: RequestEquipItemStatus,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x013C)]
#[packet_size(4)]
pub struct EquipAmmunitionPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00AB)]
pub struct RequestUnequipItemPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum RequestUnequipItemStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x099A)]
#[packet_size(9)]
pub struct RequestUnequipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub result: RequestUnequipItemStatus,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct MarketItemInformation {
    pub name_id: u32,
    pub item_type: u8,
    pub price: Price,
    pub quantity: u32,
    pub weight: u16,
    pub location: u32,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct SoldItemInformation {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

/// `CZ_MOVE_ITEM_FROM_BODY_TO_CART` (0x0126 for PACKETVER 20190605): move an
/// item from the inventory into the merchant pushcart. The inventory index uses
/// the usual `+2` wire encoding of [InventoryIndex].
///
/// Layout: rAthena `packets_struct.hpp` `PACKET_CZ_MOVE_ITEM_FROM_BODY_TO_CART`
/// (header, index, amount), parsed at `clif.cpp:12544`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0126)]
pub struct MoveItemToCartPacket {
    pub inventory_index: InventoryIndex,
    pub amount: u32,
}

/// `CZ_MOVE_ITEM_FROM_CART_TO_BODY` (0x0127 for PACKETVER 20190605): move an
/// item out of the cart back into the inventory. The cart slot is encoded like
/// an inventory index (`slot + 2`).
///
/// Layout: rAthena `packets_struct.hpp` `PACKET_CZ_MOVE_ITEM_FROM_CART_TO_BODY`
/// (header, index, amount), parsed at `clif.cpp:12558`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0127)]
pub struct MoveItemFromCartPacket {
    pub cart_index: InventoryIndex,
    pub amount: u32,
}

/// `CZ_MOVE_ITEM_FROM_STORE_TO_CART` (0x0128 for PACKETVER 20190605): move an
/// item out of the open Kafra storage into the cart. The server decodes the
/// index as `client_index - 1` (`clif.cpp:13654`), i.e. the storage slot has to
/// be sent as `slot + 1`.
///
/// Layout: rAthena `packets_struct.hpp`
/// `PACKET_CZ_MOVE_ITEM_FROM_STORE_TO_CART` (header, index, amount).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0128)]
pub struct MoveItemFromStorageToCartPacket {
    /// Storage slot encoded as `slot + 1`.
    pub storage_index: u16,
    pub amount: u32,
}

/// `CZ_MOVE_ITEM_FROM_CART_TO_STORE` (0x0129 for PACKETVER 20190605): move an
/// item from the cart into the open Kafra storage. The cart slot is encoded
/// like an inventory index (`slot + 2`), see `clif.cpp:13627`.
///
/// Layout: rAthena `packets_struct.hpp`
/// `PACKET_CZ_MOVE_ITEM_FROM_CART_TO_STORE` (header, index, amount).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0129)]
pub struct MoveItemFromCartToStoragePacket {
    pub cart_index: InventoryIndex,
    pub amount: u32,
}

/// 2019 `ZC_ADD_ITEM_TO_CART` (0x0A0B): an item was added to the cart, e.g.
/// after a deposit or when the cart list is refreshed. The cart slot uses the
/// usual inventory index encoding (`slot + 2`).
///
/// The 2019 layout has the refinement byte *before* the card slots and then the
/// five random options (`clif.cpp:7273`, 57 bytes total).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A0B)]
#[packet_size(57)]
pub struct CartItemAddedPacket2019 {
    pub cart_index: InventoryIndex,
    pub amount: i32,
    pub item_id: ItemId,
    pub item_type: u8,
    pub identified: u8,
    pub damaged: u8,
    pub refinement_level: u8,
    pub card: [u32; 4],
    pub option_data: [ItemOptions; 5],
}

/// 2019 `ZC_DELETE_ITEM_FROM_CART` (0x0125): an item (or part of a stack) was
/// removed from the cart. The cart slot uses the `slot + 2` encoding
/// (`clif.cpp:7530`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0125)]
#[packet_size(8)]
pub struct CartDeleteItemPacket2019 {
    pub cart_index: InventoryIndex,
    pub amount: u32,
}

/// 2019 `ZC_ACK_ADDITEM_TO_CART` (0x012C): moving an item into the cart failed
/// (0 = weight, 1 = item count), see `clif.cpp:7313`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x012C)]
#[packet_size(3)]
pub struct CartAddItemAckPacket2019 {
    pub result: u8,
}

/// 2019 `ZC_CARTOFF` (0x012B): the cart was removed; the client has to drop the
/// cart list and close the cart window (`clif.cpp:3284`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x012B)]
#[packet_size(2)]
pub struct CartOffPacket {}

/// One row of `ZC_NOTIFY_WEAPONITEMLIST` (0x0221).
///
/// rAthena's `PACKET_ZC_NOTIFY_WEAPONITEMLIST_sub` (`packets_struct.hpp:3332`,
/// only the `int16 index, uint32 itemId, uint8 refine, EQUIPSLOTINFO slot`
/// fields are used). `clif_item_refine_list` writes the *client* index
/// (`client_index(i)` = server slot + 2, `clif.cpp:7233`), so the index is an
/// [`InventoryIndex`].
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct WeaponRefineListEntry {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    /// Current refine level of the weapon (`item.refine`), `0` = not refined.
    pub refine: u8,
    /// Card slots of the weapon; unused by the refine window, kept so the
    /// packet size (23 bytes per row) is right.
    pub card: [u32; 4],
}

/// List of weapons that the merchant can refine (`ZC_NOTIFY_WEAPONITEMLIST`,
/// 0x0221).
///
/// Layout (`clif_item_refine_list`, `clif.cpp:7209`): `{ header }.W { length
/// }.W` followed by one [`WeaponRefineListEntry`] per weapon. The server only
/// sends this list after the player used the Blacksmith skill
/// `WS_WEAPONREFINE`, and it only accepts [`WeaponRefineRequestPacket`] while
/// that menu skill is pending (`clif_parse_WeaponRefine`, `clif.cpp:13255`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0221)]
#[variable_length]
pub struct WeaponRefineListPacket {
    #[repeating_remaining]
    pub entries: Vec<WeaponRefineListEntry>,
}

/// Result of a weapon refine attempt (`ZC_ACK_WEAPONREFINE`, 0x0223).
///
/// Layout (`clif_upgrademessage`, `clif.cpp:6937`; struct
/// `packets_struct.hpp:2728`): `{ header }.W { result }.L { item id }.L`.
/// Result codes (rAthena `e_ack_itemrefining` usage,
/// `skill.cpp:20703`/`20710`/`20751`/`20780`): `0` = success, `1` = failure
/// (the weapon lost its refine level), `2` = the refine level is already at the
/// skill limit, `3` = the required material is missing.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0223)]
#[packet_size(10)]
pub struct WeaponRefineResultPacket {
    pub result: i32,
    pub item_id: ItemId,
}

/// Refine level update of a successful/failed refine (`ZC_ACK_ITEMREFINING`,
/// 0x0188).
///
/// Layout (`clif_refine`, `clif.cpp:6919`; struct `packets.hpp:964`):
/// `{ header }.W { result }.W { index }.W { refine }.W`. The skill refine
/// additionally re-sends the inventory list (`skill.cpp:20753`); the NPC refine
/// UI does not, so this packet is the authoritative refine level for it
/// (`clif.cpp:22727`-`22750`). `result` is `e_ack_itemrefining`
/// (`clif.hpp:767`): `0` success, `1` failure (the item broke and was
/// destroyed), `2` downgrade (the item lost refine levels) and `3` failure
/// (protected by Blacksmith Blessings, nothing changed).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0188)]
#[packet_size(8)]
pub struct ItemRefiningResultPacket {
    pub result: u16,
    pub index: InventoryIndex,
    pub refine: u16,
}

/// One row of `ZC_REPAIRITEMLIST` (0x01FC).
///
/// rAthena's `REPAIRITEM_INFO1` (`packets_struct.hpp:2925`):
/// `{ index }.W { item id }.L { refine }.B { card[4] }.L`.
///
/// **The index is the raw server slot, not the `slot + 2` client index**:
/// `clif_item_repair_list` writes the loop variable `i` unchanged
/// (`clif.cpp:7153`) and `clif_parse_RepairItem` passes the value back to
/// `skill_repairweapon` unchanged (`clif.cpp:13242`). Use [`u16`] here and
/// convert with `InventoryIndex(raw)`; do not use [`InventoryIndex`] which
/// shifts by two on the wire.
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct RepairItemListEntry {
    /// Raw 0-based inventory slot of the damaged item.
    pub index: u16,
    pub item_id: ItemId,
    pub refine: u8,
    pub card: [u32; 4],
}

/// List of damaged items that can be repaired (`ZC_REPAIRITEMLIST`, 0x01FC).
///
/// Layout (`clif_item_repair_list`, `clif.cpp:7143`): `{ header }.W { length
/// }.W` followed by one [`RepairItemListEntry`] (23 bytes) per item. The list
/// is only sent after the player used `BS_REPAIRWEAPON`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01FC)]
#[variable_length]
pub struct RepairItemListPacket {
    #[repeating_remaining]
    pub entries: Vec<RepairItemListEntry>,
}

/// Result of an item repair request (`ZC_ACK_ITEMREPAIR`, 0x01FE).
///
/// Layout (`clif_item_repaireffect`, `clif.cpp:7182`; struct
/// `packets.hpp:1134`): `{ header }.W { index }.W { result }.B`. Unlike the
/// repair list index, this index *is* the `slot + 2` client index
/// (`client_index(idx)`, `clif.cpp:7186`), so it is an [`InventoryIndex`].
/// `result` is `0` = success, `1` = failure (the item was not repaired).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01FE)]
#[packet_size(5)]
pub struct RepairItemResultPacket {
    pub index: InventoryIndex,
    pub result: u8,
}

/// List of unidentified items (`ZC_ITEMIDENTIFY_LIST`, 0x0177).
///
/// Layout (`clif_item_identify_list`, `clif.cpp:7098`): `{ header }.W { length
/// }.W` followed by one `{ client index }.W` per unidentified item
/// (`client_index(i)`, `clif.cpp:7111`). The list is sent by the merchant Item
/// Appraisal skill `MC_IDENTIFY` (`skills/merchant/itemappraisal.cpp:17`),
/// which consumes a Magnifier as its reagent; the answer is
/// [`IdentifyItemRequestPacket`], which rAthena only accepts while that menu
/// skill is pending (`clif.cpp:13385`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0177)]
#[variable_length]
pub struct IdentifyItemListPacket {
    #[repeating_remaining]
    pub indexes: Vec<InventoryIndex>,
}

/// Result of an identify request (`ZC_ACK_ITEMIDENTIFY`, 0x0179).
///
/// Layout (`clif_item_identified`, `clif.cpp:7130`; struct `packets.hpp:1127`):
/// `{ header }.W { index }.W { result }.B`. `index` is the `slot + 2` client
/// index (`clif.cpp:7134`); `result` is `0` = success, `1` = failure.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0179)]
#[packet_size(5)]
pub struct IdentifyItemResultPacket {
    pub index: InventoryIndex,
    pub result: u8,
}

/// List of equipment a card can be inserted into (`ZC_ITEMCOMPOSITION_LIST`,
/// 0x017B).
///
/// Layout (`clif_use_card`, `clif.cpp:7023`): `{ header }.W { length }.W`
/// followed by one `{ client index }.W` per eligible item (`i + 2`,
/// `clif.cpp:7068`). Despite the rAthena comment saying "name id", the value
/// written is the inventory index of the equipment. The list is sent as the
/// answer to [`CardCompositionListRequestPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x017B)]
#[variable_length]
pub struct CardCompositionListPacket {
    #[repeating_remaining]
    pub indexes: Vec<InventoryIndex>,
}

/// Result of a card insertion (`ZC_ACK_ITEMCOMPOSITION`, 0x017D).
///
/// Layout (`clif_insert_card`, `clif.cpp:7084`; struct `packets.hpp:1119`):
/// `{ header }.W { equip index }.W { card index }.W { result }.B`. Both indexes
/// are `slot + 2` client indexes (`clif.cpp:7088`); `result` is `0` = success,
/// `1` = failure.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x017D)]
#[packet_size(7)]
pub struct CardCompositionResultPacket {
    pub equip_index: InventoryIndex,
    pub card_index: InventoryIndex,
    pub result: u8,
}

/// Card slot update (`ZC_UPDATE_CARDSLOT`, 0x0A3F).
///
/// Layout (`PACKET_ZC_UPDATE_CARDSLOT`, `packets_struct.hpp:3775`):
/// `{ header }.W { wear state }.w { card slot }.w { item id }.L { equip flag
/// }.b` (11 bytes at PACKETVER 20190605). rAthena 20190605 defines this struct
/// in `packets_struct.hpp` but **never sends it** (`grep -n UPDATE_CARDSLOT
/// src/map/clif.cpp` has no match); card changes arrive through the
/// `ZC_ACK_ITEMCOMPOSITION` ack plus the normal item list refresh. It is parsed
/// so a future server that uses it cannot desynchronize the packet stream.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A3F)]
#[packet_size(11)]
pub struct UpdateCardSlotPacket {
    pub wear_state: i16,
    pub card_slot: i16,
    pub item_id: ItemId,
    pub equip_flag: i8,
}

/// Request to refine the selected weapon (`CZ_REQ_WEAPONREFINE`, 0x0222).
///
/// Layout (`clif_parse_WeaponRefine`, `clif.cpp:13247`; registered as a
/// 6 byte parseable packet in `clif_packetdb.hpp:472`):
/// `{ header }.W { client index }.L`. rAthena reads a 32 bit value at offset 2
/// and calls `server_index()` on it (`clif.cpp:13262`), so the field is the
/// *client* index (server slot + 2) as a 32 bit number. It is kept as a raw
/// `u32` because [`InventoryIndex`] serializes as a 16 bit value.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0222)]
#[packet_size(6)]
pub struct WeaponRefineRequestPacket {
    /// Wire value: inventory slot + 2 (see [`WeaponRefineRequestPacket::new`]).
    pub client_index: u32,
}

impl WeaponRefineRequestPacket {
    /// Create the request for the given inventory index. The +2 encoding is
    /// what rAthena's `server_index()` expects (`clif.cpp:13262`).
    pub fn for_inventory_index(inventory_index: InventoryIndex) -> Self {
        Self {
            client_index: u32::from(inventory_index.0) + 2,
        }
    }
}

/// Request to repair a damaged item (`CZ_REQ_ITEMREPAIR1`, 0x01FD).
///
/// Layout (`PACKET_CZ_REQ_ITEMREPAIR1`, `packets_struct.hpp:2944`;
/// `clif_parse_RepairItem`, `clif.cpp:13222`): `{ header }.W { index }.W
/// { item id }.L { refine }.B { card[4] }.L` = 25 bytes at PACKETVER 20190605
/// (the opcode registry also lists 25). The index is the raw 0-based server
/// slot, matching [`RepairItemListEntry::index`]. rAthena only reads
/// `item.index` (`clif.cpp:13242`); the remaining fields are informational and
/// may be zero.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01FD)]
#[packet_size(25)]
pub struct RepairItemRequestPacket {
    /// Raw 0-based inventory slot of the damaged item.
    pub index: u16,
    pub item_id: ItemId,
    pub refine: u8,
    pub card: [u32; 4],
}

/// Request the list of equipment a card can be inserted into
/// (`CZ_REQ_ITEMCOMPOSITION_LIST`, 0x017A).
///
/// Layout (`clif_parse_UseCard`, `clif.cpp:13460`): `{ header }.W
/// { client index }.W`; the server converts with `server_index()`
/// (`clif.cpp:13468`), so [`InventoryIndex`] serializes the right value.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x017A)]
#[packet_size(4)]
pub struct CardCompositionListRequestPacket {
    pub index: InventoryIndex,
}

/// Insert a card into a piece of equipment (`CZ_REQ_ITEMCOMPOSITION`, 0x017C).
///
/// Layout (`clif_parse_InsertCard`, `clif.cpp:13478`): `{ header }.W
/// { card index }.W { equip index }.W`; both are client indexes converted with
/// `server_index()` (`clif.cpp:13490`), so [`InventoryIndex`] is correct.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x017C)]
#[packet_size(6)]
pub struct CardCompositionRequestPacket {
    pub card_index: InventoryIndex,
    pub equip_index: InventoryIndex,
}

/// Identify an item with one click, consuming a Magnifier
/// (`CZ_REQ_ONECLICK_ITEMIDENTIFY`, 0x0A35).
///
/// Layout (`clif_parse_Oneclick_Itemidentify`, `clif.cpp:21385`; registered as
/// a 4 byte parseable packet in `clif_packetdb.hpp:1765`): `{ header }.W
/// { client index }.W`. rAthena subtracts two from the wire value
/// (`clif.cpp:21389`), so [`InventoryIndex`] serializes the right value. This
/// opcode is client-to-server only and therefore not part of the
/// server-to-client opcode registry. Unlike [`IdentifyItemRequestPacket`] it
/// does not require a pending menu skill: the server consumes a Magnifier
/// (item id 611, `itemdb.hpp:57`) or Novice Magnifier (12325, `itemdb.hpp:106`)
/// from the inventory and answers with [`IdentifyItemResultPacket`].
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A35)]
#[packet_size(4)]
pub struct OneClickIdentifyRequestPacket {
    pub index: InventoryIndex,
}

/// Answer to the identify item selection dialog
/// (`CZ_REQ_ITEMIDENTIFY`, 0x0178).
///
/// Layout (`clif_parse_ItemIdentify`, `clif.cpp:13381`): `{ header }.W
/// { client index }.W`; the server converts with `server_index()`
/// (`clif.cpp:13390`). The request is only accepted while the server has
/// `MC_IDENTIFY` pending, i.e. after it sent [`IdentifyItemListPacket`].
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0178)]
#[packet_size(4)]
pub struct IdentifyItemRequestPacket {
    pub index: InventoryIndex,
}

// ---------------------------------------------------------------------------
// Refine UI packets (`UIRefiningWnd` in the official client).
//
// Since 2016 rAthena has a second, NPC/atcommand driven refine flow that is
// enabled with `feature.refineui` (conf/battle/feature.conf:85, "on" on
// CondaRO). The refine NPCs call the `refineui` script command
// (npc/merchants/refine.txt:532), which opens this UI. The opcodes below are
// client-to-server/server-to-client pairs of that flow, all guarded by
// `PACKETVER >= 20161012` in rAthena and therefore present at 20190605.
// ---------------------------------------------------------------------------

/// Select the item to refine in the refine UI (`CZ_REFINING_SELECT_ITEM`,
/// 0x0AA1).
///
/// Layout (`clif_parse_refineui_add`, `clif.cpp:22574`): `{ header }.W
/// { client index }.W`; the server converts with `server_index()`. The server
/// answers with [`RefiningMaterialListPacket`].
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AA1)]
#[packet_size(4)]
pub struct RefiningSelectItemRequestPacket {
    pub index: InventoryIndex,
}
