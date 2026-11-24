#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

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

// To make proc macros work in korangar_interface.
extern crate self as ragnarok_packets;

/// The header of a Ragnarok Online packet. It is always two bytes long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ByteConvertable, PartialOrd, Ord, Hash)]
pub struct PacketHeader(pub u16);

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

    // Implementation detail of Korangar. Can be used to convert a packet to an
    // UI element in the packet inspector.
    #[cfg(feature = "packet-to-state-element")]
    fn to_element<App: korangar_interface::application::Application>(
        self_path: impl rust_state::Path<App, Self>,
        name: String,
    ) -> Box<dyn korangar_interface::element::Element<App, LayoutInfo = ()>>;
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

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ClientTick(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AccountId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PartyId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EntityId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillId(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillLevel(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HotbarTab(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HotbarSlot(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ShopId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Price(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AttackRange(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct JobId(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ServerAddress(pub [u8; 4]);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct UserId(pub [u8; limits::USER_ID_LENGTH]);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AuthToken(pub [u8; 17]);

impl From<ServerAddress> for Ipv4Addr {
    fn from(value: ServerAddress) -> Self {
        value.0.into()
    }
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct TilePosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct LargeTilePosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ColorBGRA {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ColorRGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// Item index is always actual index + 2.
#[derive(Clone, Copy, Debug, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct InventoryIndex(pub u16);

impl FromBytes for InventoryIndex {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        // The storage list uses the same packet family as the inventory list but
        // encodes the index as `server slot + 1` (so a valid storage index can be
        // one, which would underflow the usual `- 2`). Wrap instead of panicking;
        // the storage handler shifts the value back into the real slot.
        u16::from_bytes(byte_reader).map(|raw| Self(raw.wrapping_sub(2)))
    }
}

impl ToBytes for InventoryIndex {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        u16::to_bytes(&(self.0 + 2), byte_writer)
    }
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ItemId(pub u32);

#[derive(Copy, Debug, Clone, ByteConvertable, FixedByteSize, PartialEq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum Sex {
    Female,
    Male,
    Both,
    Server,
}

/// Sent by the client to the login server.
/// The very first packet sent when logging in, it is sent after the user has
/// entered email and password.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0064)]
pub struct LoginServerLoginPacket {
    /// Unused
    #[new_default]
    pub version: [u8; 4],
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub password: String,
    /// Unused
    #[new_default]
    pub client_type: u8,
}

/// Sent by the login server as a response to [LoginServerLoginPacket]
/// succeeding. After receiving this packet, the client will connect to one of
/// the character servers provided by this packet.
#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AC4)]
#[variable_length]
pub struct LoginServerLoginSuccessPacket {
    pub login_id1: u32,
    pub account_id: AccountId,
    pub login_id2: u32,
    /// Deprecated and always 0 on rAthena
    #[new_default]
    pub ip_address: u32,
    /// Deprecated and always 0 on rAthena
    #[new_default]
    pub name: [u8; limits::NAME_LENGTH],
    /// Always 0 on rAthena
    #[new_default]
    pub unknown: u16,
    pub sex: Sex,
    pub auth_token: AuthToken,
    #[repeating_remaining]
    pub character_server_information: Vec<CharacterServerInformation>,
}

/// Sent by the character server as a response to [CharacterServerLoginPacket]
/// succeeding. Provides basic information about the number of available
/// character slots.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x082D)]
pub struct CharacterServerLoginSuccessPacket {
    /// Always 29 on rAthena
    pub unknown: u16,
    pub normal_slot_count: u8,
    pub vip_slot_count: u8,
    pub billing_slot_count: u8,
    pub producible_slot_count: u8,
    pub valid_slot: u8,
    #[new_default]
    pub unused: [u8; 20],
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099D)]
#[variable_length]
pub struct CharacterInfoPerPagePacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation2019>,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A0)]
#[packet_size(6)]
pub struct CharacterSlotPagePacket {
    pub page_quantity: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x020D)]
#[variable_length]
pub struct CharacterBanListPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterBanInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterBanInformation {
    pub character_id: CharacterId,
    #[length(20)]
    pub ban_time: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08B9)]
#[packet_size(12)]
pub struct LoginPincodePacket {
    pub pincode_seed: u32,
    pub account_id: AccountId,
    pub state: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B18)]
#[packet_size(4)]
pub struct Packet0b18 {
    /// Possibly inventory related
    #[new_default]
    pub unknown: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ConnectionRefusedReason {
    #[numeric_value(1)]
    Unknown1,
    InvalidAccountId,
    InvalidChracterId,
    #[numeric_value(6)]
    InvalidSex,
    Unkown2,
}

/// rAthena 2019 map-server auth failure (`ZC_AUTH_FAIL`, 0x006A): the server
/// rejects the map login with a single error byte (see `clif_authfail_fd`).
///
/// The 2022 layout in [`ConnectionRefusedPacket`] is 22 bytes and was also
/// registered for this opcode; parsing the 3-byte 2019 packet with it made the
/// stream lose sync during the map handshake.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006A)]
#[packet_size(3)]
pub struct MapServerAuthFailedPacket {
    pub reason: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006A)]
pub struct ConnectionRefusedPacket {
    pub reason: ConnectionRefusedReason,
    pub unknown: [u8; 19],
}

/// Sent by the map server as a response to [MapServerLoginPacket] succeeding.
/// 2019 `ZC_AUTH_OK_GID` (0x0283): sent right after the map login packet with
/// the player's block id (`sd->id`, which equals the account id). The client
/// already knows that id, so the packet only needs to be parsed for framing.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0283)]
#[packet_size(6)]
pub struct MapServerAuthOkPacket {
    pub account_id: AccountId,
}

/// `ZC_NOTIFY_CRITICALWEIGHT` (0x0ADE): weight percentage at which the client
/// displays the weight in red (rAthena `natural_heal_weight_rate`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ADE)]
#[packet_size(6)]
pub struct CriticalWeightUpdatePacket {
    pub weight: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02EB)]
#[packet_size(13)]
pub struct MapServerLoginSuccessPacket {
    pub client_tick: ClientTick,
    pub position: WorldPosition,
    /// Always [5, 5] on rAthena
    #[new_default]
    pub ignored: [u8; 2],
    pub font: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum LoginFailedReason {
    #[numeric_value(1)]
    ServerClosed,
    #[numeric_value(2)]
    AlreadyLoggedIn,
    #[numeric_value(8)]
    AlreadyOnline,
}

#[derive(Debug, Clone, Packet, ServerPacket, LoginServer, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0081)]
#[packet_size(3)]
pub struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0840)]
#[variable_length]
pub struct MapServerUnavailablePacket {
    #[length_remaining]
    pub unknown: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum LoginFailedReason2 {
    UnregisteredId,
    IncorrectPassword,
    IdExpired,
    RejectedFromServer,
    BlockedByGMTeam,
    GameOutdated,
    LoginProhibitedUntil,
    ServerFull,
    CompanyAccountLimitReached,
    BannedByDBATeam,
    UnconfirmedEmail,
    BannedByGMTeam,
    TemporaryBanForDatabaseWork,
    SelfLocked,
    NotPermittedGroup,
    #[numeric_value(99)]
    AccountIdErased,
    #[numeric_value(100)]
    LoginInformationRemains,
    #[numeric_value(101)]
    LockedForHackingInvestigation,
    #[numeric_value(102)]
    TemporaryLockedForBugInvestigation,
    #[numeric_value(103)]
    DeletingCharacter,
    #[numeric_value(104)]
    DeletingSpouseCharacter,
    UnknownError,
}

#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x083E)]
#[packet_size(26)]
pub struct LoginFailedPacket2 {
    pub reason: LoginFailedReason2,
    #[length(20)]
    pub date: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum CharacterSelectionFailedReason {
    RejectedFromServer,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// failing. Provides a reason for the character selection failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006C)]
#[packet_size(3)]
pub struct CharacterSelectionFailedPacket {
    pub reason: CharacterSelectionFailedReason,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// succeeding. Provides a map server to connect to, along with the id of our
/// selected character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006E)]
#[packet_size(3)]
pub struct CharacterCreationFailedPacket {
    pub reason: CharacterCreationFailedReason,
}

/// Sent by the client to the login server every 60 seconds to keep the
/// connection alive.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0200)]
#[ping]
pub struct LoginServerKeepalivePacket {
    #[new_value(UserId([0; 24]))]
    pub user_id: UserId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterServerInformation {
    pub server_ip: ServerAddress,
    pub server_port: u16,
    #[length(20)]
    pub server_name: String,
    pub user_count: u16,
    pub server_type: u16, // ServerType
    pub display_new: u16, // bool16 ?
    #[new_value([0; 128])]
    pub unknown: [u8; 128],
}

/// Sent by the client to the character server after after successfully logging
/// into the login server.
/// Attempts to log into the character server using the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0065)]
pub struct CharacterServerLoginPacket {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    #[new_default]
    pub unknown: u16,
    pub sex: Sex,
}

/// Sent by the client to the map server after after successfully selecting a
/// character. Attempts to log into the map server using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0436)]
pub struct MapServerLoginPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub login_id1: u32,
    pub client_tick: ClientTick,
    pub sex: Sex,
    // NOTE: 2022 clients append 4 unknown bytes here; the 2019 rAthena server
    // (PACKETVER 20190605) expects the 19-byte 0x0436 layout, so it is omitted.
}

/// An entity's skill cast was interrupted (`ZC_DISPEL`, 0x01B9).
///
/// rAthena sends this from `clif_skillcastcancel` to everyone in the area when
/// a cast is cancelled (movement, damage, another skill, ...). The packet only
/// carries the entity id: the client has to stop the looping cast animation and
/// clear its cast bar.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01B9)]
#[packet_size(6)]
pub struct SkillCastCancelPacket {
    pub entity_id: EntityId,
}
/// Sent by the client to the character server when the player tries to create
/// a new character.
/// Attempts to create a new character in an empty slot using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

#[cfg(feature = "interface")]
impl rust_state::VecItem for CharacterInformation {
    // TODO: Use CharacterId
    type Id = u32;

    fn get_id(&self) -> Self::Id {
        self.character_id.0
    }
}

/// Wire layout of rAthena's `CHARACTER_INFO` for clients before 2020-10-07
/// (`PACKETVER < 20201007`), i.e. the 2019 client used by CondaRO. It differs
/// from [CharacterInformation] in the HP/SP field widths (32/32/16/16 instead
/// of four 64-bit values), making one row 155 bytes instead of 175.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

/// Sent by the character server as a response to [CreateCharacterPacket]
/// succeeding on 2019-era servers (`PACKETVER < 20201007`): the fixed-size
/// 0x006D packet carrying one legacy 155-byte CHARACTER_INFO row.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006D)]
#[packet_size(157)]
pub struct CreateCharacterSuccessPacket2019 {
    pub character_information: CharacterInformation2019,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// succeeding. Provides all character information of the newly created
/// character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B6F)]
pub struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}

/// Sent by the client to the character server.
/// Requests a list of every character associated with the account.
#[derive(Debug, Clone, Default, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A1)]
pub struct RequestCharacterListPacket {}

/// Sent by the character server as a response to [RequestCharacterListPacket]
/// succeeding. Provides the requested list of character information.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B72)]
#[variable_length]
pub struct RequestCharacterListSuccessPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation>,
}

/// Sent by the map server to the client.
#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B1D)]
#[packet_size(2)]
#[ping]
pub struct MapServerPingPacket {}

/// Sent by the client to the map server when the player wants to move.
/// Attempts to path the player towards the provided position.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0881)]
pub struct RequestPlayerMovePacket {
    pub position: WorldPosition,
}

/// Sent by the client to the map server when the player wants to warp.
/// Attempts to warp the player to a specific position on a specific map using
/// the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0140)]
pub struct RequestWarpToMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

/// Sent by the map server to the client.
/// Informs the client that an entity is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0086)]
pub struct EntityMovePacket {
    pub entity_id: EntityId,
    pub from_to: WorldPosition2,
    pub starting_timestamp: ClientTick,
}

// TODO: Handle this to improve the combat system.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0088)]
#[packet_size(10)]
pub struct EntityStopMovePacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
}

/// Sent by the map server after the Vending skill (`MC_VENDING`) when the
/// player may open the shop creation UI (`ZC_OPENSTORE`, 0x012D).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x012D)]
#[packet_size(4)]
pub struct OpenStorePacket {
    /// Number of item slots the shop may hold (`2 + skill level`).
    pub slots: u16,
}

/// One item entry of the shop creation request.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct StoreItemEntry {
    pub inventory_index: u16,
    pub amount: u16,
    pub price: u32,
}

/// Sent by the client to create a shop (`CZ_REQ_OPENSTORE`, 0x012F).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x012F)]
#[variable_length]
pub struct RequestOpenStorePacket {
    #[length(80)]
    pub shop_name: String,
    #[repeating_remaining]
    pub items: Vec<StoreItemEntry>,
}

/// Sent by the client to cancel the shop creation UI (`CZ_REQ_OPENSTORE2`,
/// 0x01B2, result 0). rAthena clears its `prevend`/`workinprogress` state from
/// this packet, so the character cannot stay frozen after Vending.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01B2)]
#[variable_length]
pub struct CancelOpenStorePacket {
    #[length(80)]
    pub shop_name: String,
    pub result: u8,
    #[repeating_remaining]
    pub items: Vec<StoreItemEntry>,
}

/// Sent by the map server for the Taekwon `TK_HIGHJUMP` skill (and other
/// "blink" moves) before the regular stop-move correction. The client uses it
/// to place the character at the server position immediately instead of
/// waiting for the next walk/stop packet, which otherwise looks like a
/// teleport/desync after Leap (`ZC_HIGHJUMP`, 0x01FF).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01FF)]
#[packet_size(10)]
pub struct HighJumpPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
}

/// Sent by the map server when a skill relocates an entity (Snap/body
/// relocation, `ZC_SNAP`, 0x08D2): the client applies the new position
/// immediately instead of waiting for the next stop-move correction.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08D2)]
#[packet_size(10)]
pub struct SnapPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
}

/// Sent by the map server to the client.
/// Informs the client that the player is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0087)]
#[packet_size(12)]
pub struct PlayerMovePacket {
    pub starting_timestamp: ClientTick,
    pub from_to: WorldPosition2,
}

/// Sent by the client to the character server when the user tries to delete a
/// character.
/// Attempts to delete a character from the user account using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum CharacterDeletionFailedReason {
    NotAllowed,
    CharacterNotFound,
    NotEligible,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// failing. Provides a reason for the character deletion failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0070)]
#[packet_size(3)]
pub struct CharacterDeletionFailedPacket {
    pub reason: CharacterDeletionFailedReason,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// succeeding.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006F)]
#[packet_size(2)]
pub struct CharacterDeletionSuccessPacket {}

/// Sent by the client to the character server when the user selects a
/// character. Attempts to select the character in the specified slot.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0066)]
pub struct SelectCharacterPacket {
    pub selected_slot: u8,
}

/// Sent by the map server to the client when there is a new chat message from
/// the server. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x008E)]
#[variable_length]
pub struct ServerMessagePacket {
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0291)]
#[packet_size(4)]
pub struct MessageTablePacket {
    pub message_id: u16,
}

/// Sent by the client to the map server when the user hovers over an entity.
/// Attempts to fetch additional information about the entity, such as the
/// display name.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0368)]
pub struct RequestDetailsPacket {
    pub entity_id: EntityId,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the player.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A30)]
#[packet_size(106)]
pub struct RequestPlayerDetailsSuccessPacket {
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub party_name: String,
    #[length(24)]
    pub guild_name: String,
    #[length(24)]
    pub position_name: String,
    pub title_id: u32,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the entity.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ADF)]
#[packet_size(58)]
pub struct RequestEntityDetailsSuccessPacket {
    pub entity_id: EntityId,
    pub group_id: u32,
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub title: String,
}

/// Sent by the client to request the guild emblem image of a guild.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0151)]
pub struct GuildEmblemRequestPacket {
    pub guild_id: u32,
}

/// Sent by the map server as a response to [`GuildEmblemRequestPacket`].
///
/// The emblem data is a classic 24x24 guild emblem BMP file (the same bytes the
/// server stores in the `guild` table). It can be empty when the guild has no
/// emblem.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0152)]
#[variable_length]
pub struct GuildEmblemImagePacket {
    pub guild_id: u32,
    pub emblem_id: u32,
    #[length_remaining]
    pub emblem_data: Vec<u8>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09E7)]
#[packet_size(3)]
pub struct NewMailStatusPacket {
    pub new_available: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct AchievementData {
    pub acheivement_id: u32,
    pub is_completed: u8,
    pub objectives: [u32; 10],
    pub completion_timestamp: u32,
    pub got_rewarded: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A24)]
#[packet_size(66)]
pub struct AchievementUpdatePacket {
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    pub acheivement_data: AchievementData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A23)]
#[variable_length]
pub struct AchievementListPacket {
    #[new_derive]
    pub achievement_count: u32,
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    #[repeating(achievement_count)]
    pub acheivement_data: Vec<AchievementData>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D7)]
#[packet_size(15)]
pub struct SpriteChangePacket {
    pub account_id: AccountId,
    pub sprite_type: SpriteChangeType,
    pub value: u32,
    pub value2: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B08)]
#[variable_length]
pub struct InventoyStartPacket {
    pub inventory_type: u8,
    #[length_remaining]
    pub inventory_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0861)]
pub struct StorageWithdrawPacket {
    pub storage_index: u16,
    pub amount: u32,
}

/// 2019 `CZ_CLOSE_STORE` (0x0193 for PACKETVER 2019): close the Kafra storage
/// window. The 0x00F7 opcode that older clients used for this is mapped to the
/// trade item removal handler on the 2019 server.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0193)]
pub struct CloseStoragePacket {}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B0A)]
#[variable_length]
pub struct EquippableItemListPacket2019 {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation2019>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B39)]
#[variable_length]
pub struct EquippableItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EquippableSwitchItemInformation {
    pub index: InventoryIndex,
    pub position: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A9B)]
#[variable_length]
pub struct EquippableSwitchItemListPacket {
    #[repeating_remaining]
    pub item_information: Vec<EquippableSwitchItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099B)]
#[packet_size(8)]
pub struct MapTypePacket {
    pub map_type: u16,
    pub flags: u32,
}

/// Sent by the map server to the client when there is a new chat message from
/// ??. Provides the message to be displayed in the chat window, as well as
/// information on how the message should be displayed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01C3)]
#[variable_length]
pub struct Broadcast2MessagePacket {
    pub font_color: ColorRGBA,
    pub font_type: u16,
    pub font_size: u16,
    pub font_alignment: u16,
    pub font_y: u16,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone uses the @broadcast
/// command. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x009A)]
#[variable_length]
pub struct BroadcastMessagePacket {
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone writes in proximity
/// chat. Provides the source player and message to be displayed in the chat
/// window and the speach bubble.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x008D)]
#[variable_length]
pub struct OverheadMessagePacket {
    pub entity_id: EntityId,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when there is a new chat message from
/// an entity. Provides the message to be displayed in the chat window, the
/// color of the message, and the id of the entity it originated from.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C1)]
#[variable_length]
pub struct EntityMessagePacket {
    pub entity_id: EntityId,
    pub color: ColorBGRA,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C0)]
#[packet_size(7)]
pub struct DisplayEmotionPacket {
    pub entity_id: EntityId,
    pub emotion: u8,
}

/// Sent by the client to show an emotion (`CZ_REQ_EMOTION`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BF)]
pub struct RequestEmotionPacket {
    pub emotion: u8,
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

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B0)]
#[packet_size(8)]
pub struct UpdateStatPacket {
    #[length(6)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0196)]
#[packet_size(9)]
pub struct StatusChangeSequencePacket {
    pub index: u16,
    pub id: u32,
    pub state: u8,
}

/// Sent by the character server to the client when loading onto a new map.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0141)]
#[packet_size(14)]
pub struct UpdateStatPacket1 {
    #[length(12)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ACB)]
#[packet_size(12)]
pub struct UpdateStatPacket2 {
    #[length(10)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BE)]
#[packet_size(5)]
pub struct UpdateStatPacket3 {
    #[length(3)]
    pub stat_type: StatType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x013A)]
#[packet_size(4)]
pub struct UpdateAttackRangePacket {
    pub attack_range: AttackRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BB)]
pub struct RequestStatUpPacket {
    pub stat_type: StatUpType,
}

/// rAthena seems to always return [`Success`](RequestStatUpResult::Success),
/// even if the request fails.
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RequestStatUpResult {
    Failure,
    Success,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00BC)]
#[packet_size(6)]
pub struct RequestStatUpResponsePacket {
    pub staus_type: u16,
    pub success: RequestStatUpResult,
    pub value: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum Action {
    Attack,
    PickUpItem,
    SitDown,
    StandUp,
    #[numeric_value(7)]
    ContinousAttack,
    /// Unsure what this does
    #[numeric_value(12)]
    TouchSkill,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0437)]
pub struct RequestActionPacket {
    pub npc_id: EntityId,
    pub action: Action,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0362)]
pub struct ItemPickupRequestPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F3)]
#[variable_length]
// The trailing `NUL` of the message is not part of the declared length of the
// client-to-server layout, so the declared size can never equal the consumed
// size by construction. Known weird packet, see the P0-2 report.
#[allow_packet_size_mismatch]
pub struct GlobalMessagePacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

/// Use an item from the inventory (CZ_USE_ITEM).
///
/// Packet layout for PACKETVER >= 20080910:
/// `0439 <index>.W <item id>.L`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0439)]
pub struct UseItemPacket {
    pub inventory_index: InventoryIndex,
    pub item_id: ItemId,
}

/// Server asks the client for a numeric value (`ZC_OPEN_EDITDLG`).
///
/// Packet layout for PACKETVER 20190605: `0142 <npc id>.L` (rAthena
/// `clif.cpp:2637`). Answered with [`InputEditDialogPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0142)]
pub struct OpenEditDialogPacket {
    pub npc_id: EntityId,
}

/// Server asks the client for a text value (`ZC_OPEN_EDITDLGSTR`).
///
/// Packet layout for PACKETVER 20190605: `01d4 <npc id>.L` (rAthena
/// `clif.cpp:2665`). Answered with [`InputEditDialogStrPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D4)]
pub struct OpenEditDialogStrPacket {
    pub npc_id: EntityId,
}

/// Answer to `ZC_OPEN_EDITDLG` (CZ_INPUT_EDITDLG).
///
/// Packet layout for PACKETVER 20190605: `0143 <npc id>.L <value>.L`
/// (rAthena `clif.cpp:13314`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0143)]
pub struct InputEditDialogPacket {
    pub npc_id: EntityId,
    pub value: i32,
}

/// Answer to `ZC_OPEN_EDITDLGSTR` (CZ_INPUT_EDITDLGSTR).
///
/// Packet layout for PACKETVER 20190605: `01d5 <len>.W <npc id>.L <string>.?B`
/// (rAthena `clif.cpp:13333`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D5)]
#[variable_length]
#[allow_packet_size_mismatch]
pub struct InputEditDialogStrPacket {
    pub npc_id: EntityId,
    #[length_remaining_off_by_one]
    pub value: String,
}

/// Drop an item from the inventory on the ground (CZ_ITEM_THROW).
///
/// Packet layout for PACKETVER 20190605 (rAthena `clif.cpp:11994`):
/// `00a2 <index>.W <amount>.W`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01C8)]
#[packet_size(15)]
pub struct UseItemAckPacket {
    pub inventory_index: InventoryIndex,
    pub item_id: ItemId,
    pub account_id: AccountId,
    pub amount: i16,
    pub result: u8,
}

/// Whisper a message to another character (CZ_WHISPER).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0096)]
#[variable_length]
pub struct WhisperPacket {
    /// Name of the character the message is sent to.
    #[length(24)]
    pub target_name: String,
    /// Message that is sent to the target character.
    #[length_remaining]
    pub message: String,
}

/// Result of a whisper that the client sent (ZC_ACK_WHISPER).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09DF)]
#[packet_size(7)]
pub struct WhisperAckPacket {
    pub result: WhisperResult,
    pub character_id: u32,
}

/// A whispered message that another character sent to the player (ZC_WHISPER).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09DE)]
#[variable_length]
pub struct WhisperReceivedPacket {
    /// Account id of the sender.
    pub sender_id: u32,
    /// Name of the sender.
    #[length(24)]
    pub sender_name: String,
    /// Whether the sender is a game master.
    pub is_admin: u8,
    /// The whispered message.
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u8)]
pub enum WhisperResult {
    #[numeric_value(0)]
    Success,
    #[numeric_value(1)]
    TargetOffline,
    #[numeric_value(2)]
    Ignored,
    #[numeric_value(3)]
    AllIgnored,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0139)]
#[packet_size(16)]
pub struct RequestPlayerAttackFailedPacket {
    pub target_entity_id: EntityId,
    pub target_position: TilePosition,
    pub player_position: TilePosition,
    pub attack_range: AttackRange,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0977)]
#[packet_size(14)]
pub struct UpdateEntityHealthPointsPacket {
    pub entity_id: EntityId,
    pub health_points: u32,
    pub maximum_health_points: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum DamageType {
    Damage,
    PickUpItem,
    SitDown,
    StandUp,
    DamageEndure, // Not confirmed
    Splash,       // Not confirmed
    Skill,        // Not confirmed
    RepeatDamage, // Not confirmed
    MultiHitDamage,
    MultiHitDamageEndure, // Not confirmed
    CriticalHit,
    LuckyDodge,
    TouchSkill, // Not confirmed
    CriticalMultiHit,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x008A)]
pub struct DamagePacket1 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub attack_duration: u32,
    pub damage_delay: u32,
    pub damage_amount: i16,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: i16,
}

// FIX: This one is the attack animation one
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08C8)]
#[packet_size(34)]
pub struct DamagePacket3 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub attack_duration: u32,
    pub damage_delay: u32,
    pub damage_amount: u32,
    pub is_special_damage: u8,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x007F)]
#[packet_size(6)]
#[ping]
pub struct ServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0360)]
#[ping]
pub struct RequestServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum SwitchCharacterSlotResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08D5)]
#[packet_size(8)]
pub struct SwitchCharacterSlotResponsePacket2019 {
    #[new_default]
    pub unknown: u16,
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

/// 2019 era `ZC_CHANGE_DIRECTION` (0x009C): an entity turned to a new
/// direction.
///
/// rAthena sends this to the client itself on every map load (see
/// `clif_parse_LoadEndAck` -> `clif_changed_dir`) and for entities that turn
/// while in view. The client does not use the facing yet, but the packet has to
/// be parsed so that it does not desynchronize the packet stream (it used to
/// drop every packet after it in the same TCP read).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x009C)]
#[packet_size(9)]
pub struct ChangeDirectionPacket2019 {
    pub entity_id: EntityId,
    pub head_direction: u16,
    pub direction: u8,
}

/// 2019 era `ZC_NOTIFY_ACTORINIT` (0x0B1B): sent by the map server once the
/// client has finished loading the map. It only consists of the packet header.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B1B)]
#[packet_size(2)]
pub struct NotifyActorInitPacket {}

/// 2019 era `ZC_EFST_SET_ENTER` (0x0984): a status effect (buff/debuff) that is
/// already active when the entity enters the view, also sent for every active
/// status effect on map load.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0984)]
#[packet_size(28)]
pub struct EnterStatusEffectPacket2019 {
    pub entity_id: EntityId,
    pub effect_id: u16,
    pub duration: u32,
    pub remaining_duration: u32,
    pub value1: i32,
    pub value2: i32,
    pub value3: i32,
}

/// 2019 era `ZC_NOTIFY_CARTITEM_COUNTINFO` (0x0121): cart item/weight counts.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0121)]
#[packet_size(14)]
pub struct CartItemCountInfoPacket2019 {
    pub current_count: i16,
    pub maximum_count: i16,
    pub current_weight: i32,
    pub maximum_weight: i32,
}

/// 2019 era `ZC_USER_COUNT` (0x00C2): answer to `/who`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C2)]
#[packet_size(6)]
pub struct UserCountPacket2019 {
    pub player_count: i32,
}

/// 2019 era `ZC_REFUSE_ENTER` (0x0074): the map server refused the map login.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0074)]
#[packet_size(3)]
pub struct RefuseEnterPacket2019 {
    pub error_code: u8,
}

/// 2019 era `ZC_NOTIFY_HP_TO_GROUPM` (0x080E): party member health update.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x080E)]
#[packet_size(14)]
pub struct PartyMemberHealthPacket2019 {
    pub account_id: AccountId,
    pub health_points: i32,
    pub maximum_health_points: i32,
}

/// Sent by the client to request a trade with another player
/// (`CZ_REQ_EXCHANGE_ITEM`, 0x00E4).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E4)]
#[packet_size(6)]
pub struct RequestExchangeItemPacket {
    pub target_account_id: AccountId,
}

/// Sent by the client to accept/reject a trade request
/// (`CZ_ACK_EXCHANGE_ITEM`, 0x00E6). 3 = accept, 4 = reject.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E6)]
#[packet_size(3)]
pub struct AnswerExchangeItemPacket {
    pub result: u8,
}

/// Sent by the client to offer an inventory item or zeny in a trade
/// (`CZ_ADD_EXCHANGE_ITEM`, 0x00E8). `inventory_index == 0` means zeny.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E8)]
#[packet_size(8)]
pub struct AddExchangeItemPacket {
    pub inventory_index: u16,
    pub amount: u32,
}

/// Sent by the client to lock its side of the trade
/// (`CZ_CONCLUDE_EXCHANGE_ITEM`, 0x00EB).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EB)]
#[packet_size(2)]
pub struct ConfirmExchangeItemPacket {}

/// Sent by the client to cancel the trade (`CZ_CANCEL_EXCHANGE_ITEM`, 0x00ED).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00ED)]
#[packet_size(2)]
pub struct CancelExchangeItemPacket {}

/// Sent by the client to commit the trade (`CZ_EXEC_EXCHANGE_ITEM`, 0x00EF).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EF)]
#[packet_size(2)]
pub struct CommitExchangeItemPacket {}

/// `ZC_ADD_EXCHANGE_ITEM` (0x0A96 for PACKETVER 2019): the partner offered an
/// item or zeny in the current trade. `item_id == 0` means the amount is zeny.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A96)]
#[packet_size(61)]
pub struct ExchangeOfferPacket2019 {
    pub item_id: u32,
    pub item_type: u8,
    pub amount: u32,
    #[new_default]
    pub identified: u8,
    #[new_default]
    pub damaged: u8,
    #[new_default]
    pub refine: u8,
    #[new_default]
    pub cards: [u32; 4],
    #[new_default]
    pub options: [u8; 25],
    #[new_default]
    pub location: u32,
    #[new_default]
    pub look: u16,
}

/// 2019 era `ZC_ACK_ADD_EXCHANGE_ITEM` (0x00EA): trade window item ack.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EA)]
#[packet_size(5)]
pub struct ExchangeAddItemPacket2019 {
    pub inventory_index: u16,
    pub result: u8,
}

/// 2019 era `ZC_CONCLUDE_EXCHANGE_ITEM` (0x00EC): trade was accepted by a side.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EC)]
#[packet_size(3)]
pub struct ExchangeConcludePacket2019 {
    pub who: u8,
}

/// 2019 era `ZC_CANCEL_EXCHANGE_ITEM` (0x00EE): trade window was closed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00EE)]
#[packet_size(2)]
pub struct ExchangeCancelPacket2019 {}

/// 2019 era `ZC_EXEC_EXCHANGE_ITEM` (0x00F0): trade was executed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F0)]
#[packet_size(3)]
pub struct ExchangeExecPacket2019 {
    pub result: u8,
}

/// 2019 era `ZC_EXCHANGEITEM_UNDO` (0x00F1): the last trade item was undone.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F1)]
#[packet_size(2)]
pub struct ExchangeUndoPacket2019 {}

/// 2019 era `ZC_NOTIFY_STOREITEM_COUNTINFO` (0x00F2): storage item count.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F2)]
#[packet_size(6)]
pub struct StorageItemCountInfoPacket2019 {
    pub amount: u16,
    pub maximum_amount: u16,
}

/// 2019 era `ZC_DELETE_ITEM_FROM_STORE` (0x00F6): an item was removed from
/// storage.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F6)]
#[packet_size(8)]
pub struct StorageDeleteItemPacket2019 {
    pub storage_index: u16,
    pub amount: u32,
}

/// 2019 era `ZC_REQ_EXCHANGE_ITEM` (0x01F4): another player requests a trade.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01F4)]
#[packet_size(32)]
pub struct ExchangeRequestPacket2019 {
    #[length(24)]
    pub requester_name: String,
    pub target_account_id: AccountId,
    pub target_level: u16,
}

/// 2019 era `ZC_ACK_EXCHANGE_ITEM` (0x01F5): answer to a trade request.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01F5)]
#[packet_size(9)]
pub struct ExchangeAckPacket2019 {
    pub result: u8,
    pub target_account_id: AccountId,
    pub target_level: u16,
}

/// 2019 era `ZC_ITEM_PICKUP_ACK` (0x0A37): acknowledgement of an item pickup.
///
/// The client does not show the pickup message yet, but every pickup sends this
/// 69 byte packet and an unknown packet used to drop the rest of the TCP read.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

/// 2019 era `ZC_AUTORUN_SKILL` (0x0147): notification of an auto-casted
/// skill. The server sends this for skill items (e.g. fly wing ->
/// `itemskill "AL_TELEPORT",1`); the client is expected to perform the skill
/// use itself (send a normal skill request), the item is not consumed before
/// that.
///
/// Layout: `0147 <skill id>.W <type>.L <level>.W <sp cost>.W <range>.W
/// <skill name>.24B <upgradable>.B`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0147)]
pub struct AutoRunSkillPacket {
    pub skill_id: SkillId,
    pub skill_type: u32,
    pub skill_level: u16,
    pub skill_sp: u16,
    pub skill_range: u16,
    #[length(24)]
    pub skill_name: String,
    pub upgradable: u8,
}

/// 2019 era `ZC_SPIRITS` (0x01D0): monk spirit sphere count.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D0)]
#[packet_size(8)]
pub struct SpiritsPacket2019 {
    pub entity_id: EntityId,
    pub amount: i16,
}

/// 2019 era `ZC_SPIRITS2` (0x01E1): monk spirit sphere count (variant).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01E1)]
#[packet_size(8)]
pub struct Spirits2Packet2019 {
    pub entity_id: EntityId,
    pub amount: i16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0091)]
#[packet_size(22)]
pub struct ChangeMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum DisappearanceReason {
    OutOfSight,
    Died,
    LoggedOut,
    Teleported,
    TrickDead,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0080)]
#[packet_size(7)]
pub struct EntityDisAppearPacket {
    pub entity_id: EntityId,
    pub reason: DisappearanceReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FD)]
#[variable_length]
pub struct MovingEntityAppearPacket {
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub move_start_time: u32,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition2,
    pub x_size: u8,
    pub y_size: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0148)]
#[packet_size(8)]
pub struct ResurrectionPacket {
    pub entity_id: EntityId,
    /// Always 0 in rAthena.
    #[new_default]
    pub packet_type: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FE)]
#[variable_length]
pub struct EntityAppearPacket {
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition,
    pub x_size: u8,
    pub y_size: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FF)]
#[variable_length]
pub struct EntityAppear2Packet {
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job_id: JobId,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition,
    pub x_size: u8,
    pub y_size: u8,
    pub state: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum SkillType {
    #[numeric_value(0)]
    Passive,
    #[numeric_value(1)]
    Attack,
    #[numeric_value(2)]
    Ground,
    #[numeric_value(4)]
    SelfCast,
    #[numeric_value(16)]
    Support,
    #[numeric_value(32)]
    Trap,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SkillInformation {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    #[length(24)]
    pub skill_name: String,
    // TODO: bool
    pub upgradable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x010F)]
#[variable_length]
pub struct UpdateSkillTreePacket {
    #[repeating_remaining]
    pub skill_information: Vec<SkillInformation>,
}

/// Sent by the server when the level/SP cost/range of a known skill changes
/// (`ZC_SKILLINFO_UPDATE2`, 0x07E1 on this packet version).
#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x07E1)]
#[packet_size(15)]
pub struct SkillInfoUpdatePacket {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    // TODO: bool
    pub upgradable: u8,
}

/// Sent by the server when a single skill is learned or upgraded
/// (`ZC_ADD_SKILL`).
///
/// For PACKETVER 20190605 the server uses the `SKILLDATA` layout (skill id,
/// `i32` type flags, level, SP cost, range, name[24], upgradable flag), which
/// matches [`SkillInformation`].
#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0111)]
#[packet_size(39)]
pub struct AddSkillPacket {
    pub skill_information: SkillInformation,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum HotkeyType {
    Item,
    Skill,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HotkeyData {
    pub hotkey_type: HotkeyType,
    pub item_or_skill_id: u32,
    pub quantity_or_skill_level: u16,
}

impl HotkeyData {
    pub const UNBOUND: Self = Self {
        hotkey_type: HotkeyType::Item,
        item_or_skill_id: 0,
        quantity_or_skill_level: 0,
    };
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B20)]
#[packet_size(271)]
pub struct UpdateHotkeysPacket {
    pub rotate: u8,
    pub tab: HotbarTab,
    pub hotkeys: [HotkeyData; 38],
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02BA)]
pub struct SetHotkeyData1Packet {
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B21)]
pub struct SetHotkeyData2Packet {
    pub tab: HotbarTab,
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C9)]
#[packet_size(3)]
pub struct UpdatePartyInvitationStatePacket {
    pub allowed: u8, // always 0 on rAthena
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02DA)]
#[packet_size(3)]
pub struct UpdateShowEquipPacket {
    pub open_equip_window: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02D9)]
#[packet_size(10)]
pub struct UpdateConfigurationPacket {
    pub config_type: u32,
    pub value: u32, // only enabled and disabled ?
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum MarkerType {
    DisplayFor15Seconds,
    DisplayUntilLeave,
    RemoveMark,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0144)]
#[packet_size(23)]
pub struct MarkMinimapPositionPacket {
    pub npc_id: EntityId,
    pub marker_type: MarkerType,
    pub position: LargeTilePosition,
    pub id: u8,
    pub color: ColorRGBA,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B5)]
#[packet_size(6)]
pub struct NextButtonPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B6)]
#[packet_size(6)]
pub struct CloseButtonPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B7)]
#[variable_length]
pub struct DialogMenuPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum EffectId {
    Hit1,
    Hit2,
    Hit3,
    Hit4,
    Hit5,
    Hit6,
    Entry,
    Exit,
    Warp,
    Enhance,
    Coin,
    Endure,
    Beginspell,
    Glasswall,
    Healsp,
    Soulstrike,
    Bash,
    Magnumbreak,
    Steal,
    Hiding,
    Pattack,
    Detoxication,
    Sight,
    Stonecurse,
    Fireball,
    Firewall,
    Icearrow,
    Frostdiver,
    Frostdiver2,
    Lightbolt,
    Thunderstorm,
    Firearrow,
    Napalmbeat,
    Ruwach,
    Teleportation,
    Readyportal,
    Portal,
    Incagility,
    Decagility,
    Aqua,
    Signum,
    Angelus,
    Blessing,
    Incagidex,
    Smoke,
    Firefly,
    Sandwind,
    Torch,
    Spraypond,
    Firehit,
    Firesplashhit,
    Coldhit,
    Windhit,
    Poisonhit,
    Beginspell2,
    Beginspell3,
    Beginspell4,
    Beginspell5,
    Beginspell6,
    Beginspell7,
    Lockon,
    Warpzone,
    Sightrasher,
    Barrier,
    Arrowshot,
    Invenom,
    Cure,
    Provoke,
    Mvp,
    Skidtrap,
    Brandishspear,
    Cone,
    Sphere,
    Bowlingbash,
    Icewall,
    Gloria,
    Magnificat,
    Resurrection,
    Recovery,
    Earthspike,
    Spearbmr,
    Pierce,
    Turnundead,
    Sanctuary,
    Impositio,
    Lexaeterna,
    Aspersio,
    Lexdivina,
    Suffragium,
    Stormgust,
    Lord,
    Benedictio,
    Meteorstorm,
    Yufitel,
    Yufitelhit,
    Quagmire,
    Firepillar,
    Firepillarbomb,
    Hasteup,
    Flasher,
    Removetrap,
    Repairweapon,
    Crashearth,
    Perfection,
    Maxpower,
    Blastmine,
    Blastminebomb,
    Claymore,
    Freezing,
    Bubble,
    Gaspush,
    Springtrap,
    Kyrie,
    Magnus,
    Bottom,
    Blitzbeat,
    Waterball,
    Waterball2,
    Fireivy,
    Detecting,
    Cloaking,
    Sonicblow,
    Sonicblowhit,
    Grimtooth,
    Venomdust,
    Enchantpoison,
    Poisonreact,
    Poisonreact2,
    Overthrust,
    Splasher,
    Twohandquicken,
    Autocounter,
    Grimtoothatk,
    Freeze,
    Freezed,
    Icecrash,
    Slowpoison,
    Bottom2,
    Firepillaron,
    Sandman,
    Revive,
    Pneuma,
    Heavensdrive,
    Sonicblow2,
    Brandish2,
    Shockwave,
    Shockwavehit,
    Earthhit,
    Pierceself,
    Bowlingself,
    Spearstabself,
    Spearbmrself,
    Holyhit,
    Concentration,
    Refineok,
    Refinefail,
    Jobchange,
    Lvup,
    Joblvup,
    Toprank,
    Party,
    Rain,
    Snow,
    Sakura,
    StatusState,
    Banjjakii,
    Makeblur,
    Tamingsuccess,
    Tamingfailed,
    Energycoat,
    Cartrevolution,
    Venomdust2,
    Changedark,
    Changefire,
    Changecold,
    Changewind,
    Changeflame,
    Changeearth,
    Chaingeholy,
    Changepoison,
    Hitdark,
    Mentalbreak,
    Magicalatthit,
    SuiExplosion,
    Darkattack,
    Suicide,
    Comboattack1,
    Comboattack2,
    Comboattack3,
    Comboattack4,
    Comboattack5,
    Guidedattack,
    Poisonattack,
    Silenceattack,
    Stunattack,
    Petrifyattack,
    Curseattack,
    Sleepattack,
    Telekhit,
    Pong,
    Level99,
    Level99_2,
    Level99_3,
    Gumgang,
    Potion1,
    Potion2,
    Potion3,
    Potion4,
    Potion5,
    Potion6,
    Potion7,
    Potion8,
    Darkbreath,
    Deffender,
    Keeping,
    Summonslave,
    Blooddrain,
    Energydrain,
    PotionCon,
    Potion_,
    PotionBerserk,
    Potionpillar,
    Defender,
    Ganbantein,
    Wind,
    Volcano,
    Grandcross,
    Intimidate,
    Chookgi,
    Cloud,
    Cloud2,
    Mappillar,
    Linelink,
    Cloud3,
    Spellbreaker,
    Dispell,
    Deluge,
    Violentgale,
    Landprotector,
    BottomVo,
    BottomDe,
    BottomVi,
    BottomLa,
    Fastmove,
    Magicrod,
    Holycross,
    Shieldcharge,
    Mappillar2,
    Providence,
    Shieldboomerang,
    Spearquicken,
    Devotion,
    Reflectshield,
    Absorbspirits,
    Steelbody,
    Flamelauncher,
    Frostweapon,
    Lightningloader,
    Seismicweapon,
    Mappillar3,
    Mappillar4,
    Gumgang2,
    Teihit1,
    Gumgang3,
    Teihit2,
    Tanji,
    Teihit1x,
    Chimto,
    Stealcoin,
    Stripweapon,
    Stripshield,
    Striparmor,
    Striphelm,
    Chaincombo,
    RgCoin,
    Backstap,
    Teihit3,
    BottomDissonance,
    BottomLullaby,
    BottomRichmankim,
    BottomEternalchaos,
    BottomDrumbattlefield,
    BottomRingnibelungen,
    BottomRokisweil,
    BottomIntoabyss,
    BottomSiegfried,
    BottomWhistle,
    BottomAssassincross,
    BottomPoembragi,
    BottomAppleidun,
    BottomUglydance,
    BottomHumming,
    BottomDontforgetme,
    BottomFortunekiss,
    BottomServiceforyou,
    TalkFrostjoke,
    TalkScream,
    Pokjuk,
    Throwitem,
    Throwitem2,
    Chemicalprotection,
    PokjukSound,
    Demonstration,
    Chemical2,
    Teleportation2,
    PharmacyOk,
    PharmacyFail,
    Forestlight,
    Throwitem3,
    Firstaid,
    Sprinklesand,
    Loud,
    Heal,
    Heal2,
    Exit2,
    Glasswall2,
    Readyportal2,
    Portal2,
    BottomMag,
    BottomSanc,
    Heal3,
    Warpzone2,
    Forestlight2,
    Forestlight3,
    Forestlight4,
    Heal4,
    Foot,
    Foot2,
    Beginasura,
    Tripleattack,
    Hitline,
    Hptime,
    Sptime,
    Maple,
    Blind,
    Poison,
    Guard,
    Joblvup50,
    Angel2,
    Magnum2,
    Callzone,
    Portal3,
    Couplecasting,
    Heartcasting,
    Entry2,
    Saintwing,
    Spherewind,
    Colorpaper,
    Lightsphere,
    Waterfall,
    Waterfall90,
    WaterfallSmall,
    WaterfallSmall90,
    WaterfallT2,
    WaterfallT2_90,
    WaterfallSmallT2,
    WaterfallSmallT2_90,
    MiniTetris,
    Ghost,
    Bat,
    Bat2,
    Soulbreaker,
    Level99_4,
    Vallentine,
    Vallentine2,
    Pressure,
    Bash3d,
    Aurablade,
    Redbody,
    Lkconcentration,
    BottomGospel,
    Angel,
    Devil,
    Dragonsmoke,
    BottomBasilica,
    Assumptio,
    Hitline2,
    Bash3d2,
    Energydrain2,
    Transbluebody,
    Magiccrasher,
    Lightsphere2,
    Lightblade,
    Energydrain3,
    Linelink2,
    Linklight,
    Truesight,
    Falconassault,
    Tripleattack2,
    Portal4,
    Meltdown,
    Cartboost,
    Rejectsword,
    Tripleattack3,
    Spherewind2,
    Linelink3,
    Pinkbody,
    Level99_5,
    Level99_6,
    Bash3d3,
    Bash3d4,
    Napalmvalcan,
    Portal5,
    Magiccrasher2,
    BottomSpider,
    BottomFogwall,
    Soulburn,
    Soulchange,
    Baby,
    Soulbreaker2,
    Rainbow,
    Peong,
    Tanji2,
    Pressedbody,
    Spinedbody,
    Kickedbody,
    Airtexture,
    Hitbody,
    Doublegumgang,
    Reflectbody,
    Babybody,
    Babybody2,
    Giantbody,
    Giantbody2,
    Asurabody,
    _4waybody,
    Quakebody,
    AsurabodyMonster,
    Hitline3,
    Hitline4,
    Hitline5,
    Hitline6,
    Electric,
    Electric2,
    Hitline7,
    Stormkick,
    Halfsphere,
    Attackenergy,
    Attackenergy2,
    Chemical3,
    Assumptio2,
    Bluecasting,
    Run,
    Stoprun,
    Stopeffect,
    Jumpbody,
    Landbody,
    Foot3,
    Foot4,
    TaeReady,
    Grandcross2,
    Soulstrike2,
    Yufitel2,
    NpcStop,
    Darkcasting,
    Gumgangnpc,
    Agiup,
    Jumpkick,
    Quakebody2,
    Stormkick1,
    Stormkick2,
    Stormkick3,
    Stormkick4,
    Stormkick5,
    Stormkick6,
    Stormkick7,
    Spinedbody2,
    Beginasura1,
    Beginasura2,
    Beginasura3,
    Beginasura4,
    Beginasura5,
    Beginasura6,
    Beginasura7,
    Aurablade2,
    Devil1,
    Devil2,
    Devil3,
    Devil4,
    Devil5,
    Devil6,
    Devil7,
    Devil8,
    Devil9,
    Devil10,
    Doublegumgang2,
    Doublegumgang3,
    Blackdevil,
    Flowercast,
    Flowercast2,
    Flowercast3,
    Mochi,
    Lamadan,
    Edp,
    Shieldboomerang2,
    RgCoin2,
    Guard2,
    Slim,
    Slim2,
    Slim3,
    Chemicalbody,
    Castspin,
    Piercebody,
    Soullink,
    Chookgi2,
    Memorize,
    Soullight,
    Mapae,
    Itempokjuk,
    _05val,
    Beginasura11,
    Night,
    Chemical2dash,
    Groundsample,
    GiExplosion,
    Cloud4,
    Cloud5,
    BottomHermode,
    Cartter,
    Itemfast,
    Shieldboomerang3,
    Doublecastbody,
    Gravitation,
    Tarotcard1,
    Tarotcard2,
    Tarotcard3,
    Tarotcard4,
    Tarotcard5,
    Tarotcard6,
    Tarotcard7,
    Tarotcard8,
    Tarotcard9,
    Tarotcard10,
    Tarotcard11,
    Tarotcard12,
    Tarotcard13,
    Tarotcard14,
    Aciddemon,
    Greenbody,
    Throwitem4,
    BabybodyBack,
    Throwitem5,
    Bluebody,
    Hated,
    Redlightbody,
    Ro2year,
    SmaReady,
    Stin,
    RedHit,
    BlueHit,
    Quakebody3,
    Sma,
    Sma2,
    Stin2,
    Hittexture,
    Stin3,
    Sma3,
    Bluefall,
    Bluefall90,
    Fastbluefall,
    Fastbluefall90,
    BigPortal,
    BigPortal2,
    ScreenQuake,
    Homuncasting,
    Hflimoon1,
    Hflimoon2,
    Hflimoon3,
    HoUp,
    Hamidefence,
    Hamicastle,
    Hamiblood,
    Hated2,
    Twilight1,
    Twilight2,
    Twilight3,
    ItemThunder,
    ItemCloud,
    ItemCurse,
    ItemZzz,
    ItemRain,
    ItemLight,
    Angel3,
    M01,
    M02,
    M03,
    M04,
    M05,
    M06,
    M07,
    Kaizel,
    Kaahi,
    Cloud6,
    Food01,
    Food02,
    Food03,
    Food04,
    Food05,
    Food06,
    Shrink,
    Throwitem6,
    Sight2,
    Quakebody4,
    Firehit2,
    NpcStop2,
    NpcStop2Del,
    Fvoice,
    Wink,
    CookingOk,
    CookingFail,
    TempOk,
    TempFail,
    Hapgyeok,
    Throwitem7,
    Throwitem8,
    Throwitem9,
    Throwitem10,
    Bunsinjyutsu,
    Kouenka,
    Hyousensou,
    BottomSuiton,
    Stin4,
    Thunderstorm2,
    Chemical4,
    Stin5,
    MadnessBlue,
    MadnessRed,
    RgCoin3,
    Bash3d5,
    Chookgi3,
    Kirikage,
    Tatami,
    Kasumikiri,
    Issen,
    Kaen,
    Baku,
    Hyousyouraku,
    Desperado,
    LightningS,
    BlindS,
    PoisonS,
    FreezingS,
    FlareS,
    Rapidshower,
    Magicalbullet,
    Spreadattack,
    Trackcasting,
    Tracking,
    Tripleaction,
    Bullseye,
    MapMagiczone,
    MapMagiczone2,
    Damage1,
    Damage1_2,
    Damage1_3,
    Undeadbody,
    UndeadbodyDel,
    GreenNumber,
    BlueNumber,
    RedNumber,
    PurpleNumber,
    BlackNumber,
    WhiteNumber,
    YellowNumber,
    PinkNumber,
    BubbleDrop,
    NpcEarthquake,
    DaSpace,
    Dragonfear,
    Bleeding,
    Wideconfuse,
    BottomRunner,
    BottomTransfer,
    CrystalBlue,
    BottomEvilland,
    Guard3,
    NpcSlowcast,
    Criticalwound,
    Green99_3,
    Green99_5,
    Green99_6,
    Mapsphere,
    PokLove,
    PokWhite,
    PokValen,
    PokBirth,
    PokChristmas,
    MapMagiczone3,
    MapMagiczone4,
    Dust,
    TorchRed,
    TorchGreen,
    MapGhost,
    Glow1,
    Glow2,
    Glow4,
    TorchPurple,
    Cloud7,
    Cloud8,
    Flowerleaf,
    Mapsphere2,
    Glow11,
    Glow12,
    Circlelight,
    Item315,
    Item316,
    Item317,
    Item318,
    StormMin,
    PokJap,
    MapGreenlight,
    MapMagicwall,
    MapGreenlight2,
    Yellowfly1,
    Yellowfly2,
    BottomBlue,
    BottomBlue2,
    Wewish,
    Firepillaron2,
    Forestlight5,
    Soulbreaker3,
    AdoStr,
    IgnStr,
    Chimto2,
    Windcutter,
    Detect2,
    Frostmysty,
    CrimsonStr,
    HellStr,
    SprMash,
    SprSoule,
    DhowlStr,
    Earthwall,
    Soulbreaker4,
    ChainlStr,
    ChookgiFire,
    ChookgiWind,
    ChookgiWater,
    ChookgiGround,
    MagentaTrap,
    CobaltTrap,
    MaizeTrap,
    VerdureTrap,
    NormalTrap,
    Cloaking2,
    AimedStr,
    ArrowstormStr,
    LaulamusStr,
    LauagnusStr,
    MilshieldStr,
    Concentration2,
    Fireball2,
    Bunsinjyutsu2,
    Cleartime,
    Glasswall3,
    Oratio,
    PotionBerserk2,
    Circlepower,
    Rolling1,
    Rolling2,
    Rolling3,
    Rolling4,
    Rolling5,
    Rolling6,
    Rolling7,
    Rolling8,
    Rolling9,
    Rolling10,
    Purplebody,
    Stin6,
    RgCoin4,
    Poisonwav,
    Poisonsmoke,
    Gumgang4,
    Shieldboomerang4,
    Castspin2,
    Vulcanwav,
    Agiup2,
    Detect3,
    Agiup3,
    Detect4,
    Electric3,
    Guard4,
    BottomBarrier,
    BottomStealth,
    Repairtime,
    NcAnal,
    Firethrow,
    Venomimpress,
    Frostmisty,
    Burning,
    Coldthrow,
    Makehallu,
    Hallutime,
    Infraredscan,
    Crashaxe,
    Gthunder,
    Stonering,
    Intimidate2,
    Stasis,
    Redline,
    Frostdiver3,
    BottomBasilica2,
    Recognized,
    Tetra,
    Tetracasting,
    Fireball3,
    Intimidate3,
    Recognized2,
    Cloaking3,
    Intimidate4,
    Stretch,
    Blackbody,
    Enervation,
    Enervation2,
    Enervation3,
    Enervation4,
    Enervation5,
    Enervation6,
    Linelink4,
    RgCoin5,
    WaterfallAni,
    BottomManhole,
    Manhole,
    Makefeint,
    Forestlight6,
    Darkcasting2,
    BottomAni,
    BottomMaelstrom,
    BottomBloodylust,
    BeginspellN1,
    BeginspellN2,
    HealN,
    ChookgiN,
    Joblvup50_2,
    Chemical2dash2,
    Chemical2dash3,
    Rollingcast,
    WaterBelow,
    WaterFade,
    BeginspellN3,
    BeginspellN4,
    BeginspellN5,
    BeginspellN6,
    BeginspellN7,
    BeginspellN8,
    WaterSmoke,
    Dance1,
    Dance2,
    Linkparticle,
    Soullight2,
    SprParticle,
    SprParticle2,
    SprPlant,
    ChemicalV,
    Shootparticle,
    BotReverb,
    RainParticle,
    ChemicalV2,
    Secra,
    BotReverb2,
    Circlepower2,
    Secra2,
    ChemicalV3,
    Enervation7,
    Circlepower3,
    SprPlant2,
    Circlepower4,
    SprPlant3,
    RgCoin6,
    SprPlant4,
    Circlepower5,
    SprPlant5,
    Circlepower6,
    SprPlant6,
    Circlepower7,
    SprPlant7,
    Circlepower8,
    SprPlant8,
    Heartasura,
    Beginspell150,
    Level99_150,
    Primecharge,
    Glasswall4,
    GradiusLaser,
    Bash3d6,
    Gumgang5,
    Hitline8,
    Electric4,
    Teihit1t,
    Spinmove,
    Fireball4,
    Tripleattack4,
    Chemical3s,
    Groundshake,
    Dq9Charge,
    Dq9Charge2,
    Dq9Charge3,
    Dq9Charge4,
    Blueline,
    Selfscroll,
    SprLightprint,
    PngTest,
    BeginspellYb,
    Chemical2dash4,
    Groundshake2,
    Pressure2,
    RgCoin7,
    Primecharge2,
    Primecharge3,
    Primecharge4,
    Greencasting,
    Wallofthorn,
    Fireball5,
    Throwitem11,
    SprPlant9,
    Demonicfire,
    Demonicfire2,
    Demonicfire3,
    Hellsplant,
    Firewall2,
    Vacuum,
    SprPlant10,
    SprLightprint2,
    Poisonsmoke2,
    Makehallu2,
    Shockwave2,
    SprPlant11,
    Coldthrow2,
    Demonicfire4,
    Pressure3,
    Linkparticle2,
    Soullight3,
    Chareffect,
    Gumgang6,
    Fireball6,
    Gumgang7,
    Gumgang8,
    Gumgang9,
    BottomDe2,
    Coldstatus,
    SprLightprint3,
    Waterball3,
    HealN2,
    RainParticle2,
    Cloud9,
    Yellowfly3,
    ElGust,
    ElBlast,
    ElAquaplay,
    ElUpheaval,
    ElWildStorm,
    ElChillyAir,
    ElCursedSoil,
    ElCooler,
    ElTropic,
    ElPyrotechnic,
    ElPetrology,
    ElHeater,
    PoisonMist,
    EraserCutter,
    SilentBreeze,
    MagmaFlow,
    Graybody,
    LavaSlide,
    SonicClaw,
    TinderBreaker,
    MidnightFrenzy,
    Macro,
    ChemicalAllrange,
    TetraFire,
    TetraWater,
    TetraWind,
    TetraGround,
    Emitter,
    VolcanicAsh,
    Level99Orb1,
    Level99Orb2,
    Level150,
    Level150Sub,
    Throwitem4_1,
    ThrowHappokunai,
    ThrowMultipleCoin,
    ThrowBakuretsu,
    RotateHuumaranka,
    RotateBg,
    RotateLineGray,
    _2011rwc,
    _2011rwc2,
    Kaihou,
    GroundExplosion,
    KgKagehumi,
    KoZenkaiWater,
    KoZenkaiLand,
    KoZenkaiFire,
    KoZenkaiWind,
    KoJyumonjikiri,
    KoSetsudan,
    RedCross,
    KoIzayoi,
    RotateLineBlue,
    KgKyomu,
    KoHuumaranka,
    Bluelightbody,
    Kagemusya,
    ObGensou,
    No100Firecracker,
    KoMakibishi,
    Kaihou1,
    Akaitsuki,
    Zangetsu,
    Gensou,
    HatEffect,
    Cherryblossom,
    EventCloud,
    RunMakeOk,
    RunMakeFailure,
    MiresultMakeOk,
    MiresultMakeFail,
    AllRayOfProtection,
    Venomfog,
    Duststorm,
    Level160,
    Level160Sub,
    Mapchain,
    MagicFloor,
    Icemine,
    Flamecorss,
    Icemine1,
    DanceBladeAtk,
    Darkpiercing,
    Invincibleoff2,
    Maxpain,
    Deathsummon,
    Moonstar,
    Strangelights,
    SuperStar,
    Yellobody,
    Colorpaper2,
    EvilsPaw,
    GcDarkcrow,
    RkDragonbreathWater,
    AllFullThrottle,
    SrFlashcombo,
    RkLuxanima,
    Cloud10,
    SoElementalShield,
    AbOffertorium,
    WlTelekinesisIntense,
    GnIllusiondoping,
    NcMagmaEruption,
    LgKingsGrace,
    Blooddrain2,
    NpcWideweb,
    NpcBurnt,
    NpcChill,
    RaUnlimit,
    AbOffertoriumRing,
    ScEscape,
    WmFriggSong,
    Flicker,
    CMaker,
    HammerOfGod,
    MassSpiral,
    FireRain,
    Whitebody,
    BanishingBuster,
    Slugshot,
    DTail,
    BindTrap1,
    BindTrap2,
    BindTrap3,
    Jumpbody1,
    AnimatedEmitter,
    RlExplosion,
    CMaker1,
    QdShot,
    PAlter,
    SStorm,
    MusicHat,
    CloudKill,
    Escape,
    XenoSlasher,
    Flowersmoke,
    Fstone,
    Qscaraba,
    Ljosalfar,
    Happinessstar,
    PowerOfGaia,
    MapleFalls,
    MarkingUseChangemonster,
    MagicalFeather,
    MermaidLonging,
    GiftOfSnow,
    AchComplete,
    TimeAccessory,
    Spritemable,
    Tunaparty,
    Freshshrimp,
    #[numeric_value(1123)]
    SuGrooming,
    SuChattering,
    #[numeric_value(1133)]
    Firedance,
    RichsCoinA,
    #[numeric_value(1137)]
    EChain,
    HeatBarrel,
    HMine,
    FallenAngel,
    #[numeric_value(1149)]
    ImmuneProperty,
    MoveCoordinate,
    #[numeric_value(1197)]
    LightsphereSun,
    LightsphereMoon,
    LightsphereStar,
    #[numeric_value(1202)]
    Novaexplosing,
    StarEmperor,
    SmaBlack,
    #[numeric_value(1208)]
    EnergydrainBlack,
    BlinkBody,
    #[numeric_value(1218)]
    Solarburst,
    SjDocument,
    FallingStar,
    #[numeric_value(1223)]
    Stormkick8,
    #[numeric_value(1229)]
    NewmoonKick,
    FullmoonKick,
    BookOfDimension,
    #[numeric_value(1233)]
    CurseExplosion,
    SoulReaper,
    #[numeric_value(1242)]
    SoulExplosion,
    Max,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01F3)]
#[packet_size(10)]
pub struct DisplaySpecialEffectPacket {
    pub entity_id: EntityId,
    /// Raw effect id; kept as `u32` so weather effect ids that have no
    /// `EffectId` enum variant are not rejected by the decoder.
    pub effect_id: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x043D)]
#[packet_size(8)]
pub struct DisplaySkillCooldownPacket {
    pub skill_id: SkillId,
    pub until: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01DE)]
#[packet_size(33)]
pub struct DisplaySkillEffectAndDamagePacket {
    pub skill_id: SkillId,
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub start_time: ClientTick,
    pub soruce_delay: u32,
    pub destination_delay: u32,
    pub damage: u32,
    pub level: SkillLevel,
    pub div: u16,
    pub skill_type: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum HealType {
    #[numeric_value(5)]
    Health,
    #[numeric_value(7)]
    SpellPoints,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A27)]
#[packet_size(8)]
pub struct DisplayPlayerHealEffect {
    pub heal_type: HealType,
    pub heal_amount: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09CB)]
#[packet_size(17)]
pub struct DisplaySkillEffectNoDamagePacket {
    pub skill_id: SkillId,
    pub heal_amount: u32,
    pub destination_entity_id: EntityId,
    pub source_entity_id: EntityId,
    pub result: u8,
}

/// One entry of the Warp Portal destination list (`ZC_WARPLIST`, 0x0ABE).
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct WarpPointEntry {
    /// Map name, padded with NUL bytes to `MAP_NAME_LENGTH_EXT` (16) bytes.
    #[length(16)]
    pub map_name: String,
}

/// Sent by the map server when the player has to choose the destination of a
/// Warp Portal (`AL_WARP`).
///
/// The client answers with [`SelectWarpPointPacket`].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ABE)]
#[variable_length]
pub struct WarpPointListPacket {
    pub skill_id: SkillId,
    #[repeating_remaining]
    pub maps: Vec<WarpPointEntry>,
}

/// Sent by the client to the map server to answer a Warp Portal destination
/// list. `map_name` is the destination map from [`WarpPointListPacket`].
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x011B)]
#[packet_size(20)]
pub struct SelectWarpPointPacket {
    pub skill_id: SkillId,
    #[length(16)]
    pub map_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ObjectiveDetails1 {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub mob_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F9)]
pub struct QuestNotificationPacket1 {
    pub quest_id: u32,
    pub active: u8,
    pub start_time: u32,
    pub expire_time: u32,
    pub objective_count: u16,
    /// For some reason this packet always has space for three objective
    /// details, even if none are sent.
    pub objective_details: [ObjectiveDetails1; 3],
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct HuntingObjective {
    pub quest_id: u32,
    pub mob_id: u32,
    pub total_count: u16,
    pub current_count: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08FE)]
#[variable_length]
pub struct HuntingQuestNotificationPacket {
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09FA)]
#[variable_length]
pub struct HuntingQuestUpdateObjectivePacket {
    pub objective_count: u16,
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02B4)]
#[packet_size(6)]
pub struct QuestRemovedPacket {
    pub quest_id: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct QuestDetails {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub kill_count: u16,
    pub total_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Quest {
    pub quest_id: u32,
    pub active: u8,
    pub remaining_time: u32, // TODO: double check these
    pub expire_time: u32,    // TODO: double check these
    #[new_derive]
    pub objective_count: u16,
    #[repeating(objective_count)]
    pub objective_details: Vec<QuestDetails>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F8)]
#[variable_length]
pub struct QuestListPacket {
    #[new_derive]
    pub quest_count: u32,
    #[repeating(quest_count)]
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum VisualEffect {
    BaseLevelUp,
    JobLevelUp,
    RefineFailure,
    RefineSuccess,
    GameOver,
    PharmacySuccess,
    PharmacyFailure,
    BaseLevelUpSuperNovice,
    JobLevelUpSuperNovice,
    BaseLevelUpTaekwon,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x019B)]
#[packet_size(10)]
pub struct VisualEffectPacket {
    pub entity_id: EntityId,
    pub effect: VisualEffect,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ExperienceType {
    #[numeric_value(1)]
    BaseExperience,
    JobExperience,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum ExperienceSource {
    Regular,
    Quest,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ACC)]
#[packet_size(18)]
pub struct DisplayGainedExperiencePacket {
    pub account_id: AccountId,
    pub amount: u64,
    pub experience_type: ExperienceType,
    pub experience_source: ExperienceSource,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum ImageLocation {
    BottomLeft,
    BottomMiddle,
    BottomRight,
    MiddleFloating,
    MiddleColorless,
    #[numeric_value(255)]
    ClearAll,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01B3)]
#[packet_size(67)]
pub struct DisplayImagePacket {
    #[length(64)]
    pub image_name: String,
    pub location: ImageLocation,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0229)]
#[packet_size(15)]
pub struct StateChangePacket {
    pub entity_id: EntityId,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub is_pk_mode_on: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00A1)]
#[packet_size(6)]
pub struct ItemDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x07FA)]
#[packet_size(8)]
pub struct RemoveItemFromInventoryPacket {
    pub remove_reason: RemoveItemReason,
    pub index: InventoryIndex,
    pub amount: u16,
}

// TODO: improve names
#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum QuestEffect {
    Quest,
    Quest2,
    Job,
    Job2,
    Event,
    Event2,
    ClickMe,
    DailyQuest,
    Event3,
    JobQuest,
    JumpingPoring,
    #[numeric_value(9999)]
    None,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum QuestColor {
    Yellow,
    Orange,
    Green,
    Purple,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0446)]
#[packet_size(14)]
pub struct QuestEffectPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub effect: QuestEffect,
    pub color: QuestColor,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B4)]
#[variable_length]
pub struct NpcDialogPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub text: String,
}

#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x007D)]
pub struct MapLoadedPacket {}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0187)]
#[ping]
pub struct CharacterServerKeepalivePacket {
    /// rAthena never reads this value, so just set it to 0.
    #[new_value(AccountId(0))]
    pub account_id: AccountId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0090)]
pub struct StartDialogPacket {
    pub npc_id: EntityId,
    #[new_value(1)]
    pub dialog_type: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B9)]
pub struct NextDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0146)]
pub struct CloseDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B8)]
pub struct ChooseDialogOptionPacket {
    pub npc_id: EntityId,
    pub option: i8,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
        const COSTUME_HEAD_LOWER = 4196;
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0998)]
pub struct RequestEquipItemPacket {
    pub inventory_index: InventoryIndex,
    pub equip_position: EquipPosition,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0999)]
#[packet_size(11)]
pub struct RequestEquipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub view_id: u16,
    pub result: RequestEquipItemStatus,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x013C)]
#[packet_size(4)]
pub struct EquipAmmunitionPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
// rAthena sends this as an uint16 (`PACKET_ZC_ACTION_FAILURE`: int16 packetType + uint16 type),
// so the enum must be serialized as u16 as well. With the default u8 the packet was 3 bytes
// instead of 4 and parsing it left a stray byte in the packet stream.
#[numeric_type(u16)]
pub enum AmmunitionActionType {
    EquipProperAmmunitionFirst,
    WeightLimitExceeded1,
    WeightLimitExceeded2,
    Equipped,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x013B)]
#[packet_size(4)]
pub struct AmmunitionActionPacket {
    pub action_type: AmmunitionActionType,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00AB)]
pub struct RequestUnequipItemPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RequestUnequipItemStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099A)]
#[packet_size(9)]
pub struct RequestUnequipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub result: RequestUnequipItemStatus,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RestartType {
    Respawn,
    Disconnect,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B1)]
#[packet_size(8)]
pub struct ParameterChangePacket {
    pub variable_id: u16,
    pub value: u32,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B2)]
pub struct RestartPacket {
    pub restart_type: RestartType,
}

// TODO: check that this can be only 1 and 0, if not ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum RestartResponseStatus {
    Nothing,
    Ok,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00B3)]
#[packet_size(3)]
pub struct RestartResponsePacket {
    pub result: RestartResponseStatus,
}

// TODO: check that this can be only 1 and 0, if not Named, ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum DisconnectResponseStatus {
    Ok,
    Wait10Seconds,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x018B)]
#[packet_size(4)]
pub struct DisconnectResponsePacket {
    pub result: DisconnectResponseStatus,
}

/// Sent to the map server to request a clean disconnect (`CZ_REQ_DISCONNECT`).
///
/// Unlike the restart packet (0x00B2), which returns to the character selection
/// and keeps the account online, this makes the map server save the character
/// and tell the character server (and through it the login server) that the
/// account is offline, so it can log in again immediately.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x018A)]
pub struct RequestDisconnectPacket {
    /// Disconnect type. The server ignores the value, official clients send 0.
    pub disconnect_type: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0438)]
pub struct UseSkillAtIdPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_id: EntityId,
}

/// 2019-era target skill packet (`CZ_USE_SKILL_TOID`).
///
/// rAthena parses targeted skill use from 0x089B for `PACKETVER >= 20130320`
/// (our 20190605 server): 10 bytes. rAthena's own layout comment is
/// `<skill lv>.W <skill id>.W <target id>.L`, so offsets are 2 = **level**
/// (u16), 4 = **skill id** (u16), 6 = target id (u32). We previously sent the
/// two in the opposite order, which the server read as skill=level/level=id and
/// silently ignored.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x089B)]
pub struct UseSkillAtTargetPacket2019 {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AF4)]
pub struct UseSkillOnGroundPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_position: TilePosition,
    #[new_default]
    pub unused: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B10)]
pub struct StartUseSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B11)]
pub struct EndUseSkillPacket {
    pub skill_id: SkillId,
}

/// 2019-era (`PACKETVER` >= 20181212) layout of `ZC_USESKILL_ACK`.
///
/// The 2019 server sends this header instead of the [UseSkillSuccessPacket]
/// header (0x07FB) that newer clients use; compared to that packet it has an
/// additional trailing `attackMT` field (29 bytes in total). It is parsed and
/// turned into the typed `NetworkEvent::SkillCast` by the client.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B1A)]
#[packet_size(29)]
pub struct UseSkillAckPacket2019 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub position: TilePosition,
    pub skill_id: SkillId,
    pub element: u32,
    pub delay_time: u32,
    pub disposable: u8,
    pub attack_motion_time: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0110)]
#[packet_size(14)]
pub struct ToUseSkillSuccessPacket {
    pub skill_id: SkillId,
    pub btype: i32,
    pub item_id: ItemId,
    pub flag: u8,
    pub cause: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum UnitId {
    #[numeric_value(0x7E)]
    Safetywall,
    Firewall,
    WarpWaiting,
    WarpActive,
    Benedictio,
    Sanctuary,
    Magnus,
    Pneuma,
    Dummyskill,
    FirepillarWaiting,
    FirepillarActive,
    HiddenTrap,
    Trap,
    HiddenWarpNpc,
    UsedTraps,
    Icewall,
    Quagmire,
    Blastmine,
    Skidtrap,
    Anklesnare,
    Venomdust,
    Landmine,
    Shockwave,
    Sandman,
    Flasher,
    Freezingtrap,
    Claymoretrap,
    Talkiebox,
    Volcano,
    Deluge,
    Violentgale,
    Landprotector,
    Lullaby,
    Richmankim,
    Eternalchaos,
    Drumbattlefield,
    Ringnibelungen,
    Rokisweil,
    Intoabyss,
    Siegfried,
    Dissonance,
    Whistle,
    Assassincross,
    Poembragi,
    Appleidun,
    Uglydance,
    Humming,
    Dontforgetme,
    Fortunekiss,
    Serviceforyou,
    Graffiti,
    Demonstration,
    Callfamily,
    Gospel,
    Basilica,
    Moonlit,
    Fogwall,
    Spiderweb,
    Gravitation,
    Hermode,
    Kaensin,
    Suiton,
    Tatamigaeshi,
    Kaen,
    GrounddriftWind,
    GrounddriftDark,
    GrounddriftPoison,
    GrounddriftWater,
    GrounddriftFire,
    Deathwave,
    Waterattack,
    Windattack,
    Earthquake,
    Evilland,
    DarkRunner,
    DarkTransfer,
    Epiclesis,
    Earthstrain,
    Manhole,
    Dimensiondoor,
    Chaospanic,
    Maelstrom,
    Bloodylust,
    Feintbomb,
    Magentatrap,
    Cobalttrap,
    Maizetrap,
    Verduretrap,
    Firingtrap,
    Iceboundtrap,
    Electricshocker,
    Clusterbomb,
    Reverberation,
    SevereRainstorm,
    Firewalk,
    Electricwalk,
    Netherworld,
    PsychicWave,
    CloudKill,
    Poisonsmoke,
    Neutralbarrier,
    Stealthfield,
    Warmer,
    ThornsTrap,
    Wallofthorn,
    DemonicFire,
    FireExpansionSmokePowder,
    FireExpansionTearGas,
    HellsPlant,
    VacuumExtreme,
    Banding,
    FireMantle,
    WaterBarrier,
    Zephyr,
    PowerOfGaia,
    FireInsignia,
    WaterInsignia,
    WindInsignia,
    EarthInsignia,
    PoisonMist,
    LavaSlide,
    VolcanicAsh,
    ZenkaiWater,
    ZenkaiLand,
    ZenkaiFire,
    ZenkaiWind,
    Makibishi,
    Venomfog,
    Icemine,
    Flamecross,
    Hellburning,
    MagmaEruption,
    KingsGrace,
    GlitteringGreed,
    BTrap,
    FireRain,
    Catnippowder,
    Nyanggrass,
    Creatingstar,
    Dummy0,
    RainOfCrystal,
    MysteryIllusion,
    #[numeric_value(269)]
    StrantumTremor,
    ViolentQuake,
    AllBloom,
    TornadoStorm,
    FloralFlareRoad,
    AstralStrike,
    CrossRain,
    PneumaticusProcella,
    AbyssSquare,
    AcidifiedZoneWater,
    AcidifiedZoneGround,
    AcidifiedZoneWind,
    AcidifiedZoneFire,
    LightningLand,
    VenomSwamp,
    Conflagration,
    CaneOfEvilEye,
    TwinklingGalaxy,
    StarCannon,
    GrenadesDropping,
    #[numeric_value(290)]
    Fuumashouaku,
    MissionBombard,
    TotemOfTutelary,
    HyunRoksBreeze,
    Shinkirou, // mirage
    JackFrostNova,
    GroundGravitation,
    #[numeric_value(298)]
    Kunaiwaikyoku,
    #[numeric_value(20852)]
    Deepblindtrap,
    Solidtrap,
    Swifttrap,
    Flametrap,
    #[numeric_value(0xC1)]
    GdLeadership,
    #[numeric_value(0xC2)]
    GdGlorywounds,
    #[numeric_value(0xC3)]
    GdSoulcold,
    #[numeric_value(0xC4)]
    GdHawkeyes,
    #[numeric_value(0x190)]
    Max,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09CA)]
#[packet_size(23)]
pub struct NotifySkillUnitPacket {
    pub lenght: u16,
    pub entity_id: EntityId,
    pub creator_id: EntityId,
    pub position: TilePosition,
    pub unit_id: UnitId,
    pub range: u8,
    pub visible: u8,
    pub skill_level: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0117)]
#[packet_size(18)]
pub struct NotifyGroundSkillPacket {
    pub skill_id: SkillId,
    pub entity_id: EntityId,
    pub level: SkillLevel,
    pub position: TilePosition,
    pub start_time: ClientTick,
}

/// Sent by the map server when the client should play a 3d sound effect
/// (`ZC_SOUND`, 0x01D3).
///
/// The 2019 rAthena layout is 35 bytes: `name[24]`, `act` (1 byte, start/stop
/// marker), `term` (minimum interval between two plays in milliseconds) and the
/// account id of the entity that produced the sound.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x01D3)]
#[packet_size(35)]
pub struct SoundPacket {
    /// File name relative to `data/wav`, e.g. `effect\\ef_blessing.wav`.
    #[length(24)]
    pub name: String,
    /// `0`/`1` mark the start/end of a looping sound; both play it once here.
    pub act: u8,
    /// Minimum interval between two plays of the same sound, in milliseconds.
    pub term: u32,
    /// Account id of the entity that produced the sound.
    pub account_id: AccountId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0120)]
#[packet_size(6)]
pub struct SkillUnitDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Friend {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
}

// NOTE: `CZ_ADD_FRIENDS` is 0x0202 for clients older than 2012 only. Since
// PACKETVER 20130320 rAthena expects 0x086F (0x0202 became
// `CZ_CHANGE_DIRECTION`), so sending 0x0202 to a modern server corrupts the
// packet stream.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x086F)]
pub struct AddFriendPacket {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0203)]
pub struct RemoveFriendPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x020A)]
#[packet_size(10)]
pub struct NotifyFriendRemovedPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0201)]
#[variable_length]
pub struct FriendListPacket {
    #[repeating_remaining]
    pub friend_list: Vec<Friend>,
}

/// A single entry of the 2019-era `ZC_FRIENDS_LIST` (0x0201) packet.
///
/// Since PACKETVER 20180307 rAthena no longer sends the 24-byte friend name in
/// the friend list (see `clif_friendslist_send`), so on the 2019 CondaRO
/// server each entry consists of the account and character id only. Parsing
/// such a packet with the 2022-era [Friend] entry would read past the end of
/// the packet and desynchronize the whole map server stream.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Friend2019 {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

/// 2019-era `ZC_FRIENDS_LIST` (0x0201) packet: 8-byte entries without a name.
///
/// Registered for the map server of the 2019 CondaRO server instead of
/// [FriendListPacket], whose entries are 32 bytes and include the friend name.
/// Reading the wrong entry size made the parser skip past the packet, which
/// silently swallowed the following `ZC_NPCACK_MAPMOVE` (0x0091) packet and
/// left the client on a black screen. Friend names are delivered later by the
/// `ZC_FRIENDS_STATE` (0x0206) packet.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0201)]
#[variable_length]
pub struct FriendListPacket2019 {
    #[repeating_remaining]
    pub friend_list: Vec<Friend2019>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum OnlineState {
    Online,
    Offline,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0206)]
#[packet_size(35)]
pub struct FriendOnlineStatusPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub state: OnlineState,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0207)]
#[packet_size(34)]
pub struct FriendRequestPacket {
    pub requestee: Friend,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum FriendRequestResponse {
    Reject,
    Accept,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0208)]
pub struct FriendRequestResponsePacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub response: FriendRequestResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum FriendRequestResult {
    Accepted,
    Rejected,
    OwnFriendListFull,
    OtherFriendListFull,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0209)]
#[packet_size(36)]
pub struct FriendRequestResultPacket {
    pub result: FriendRequestResult,
    pub friend: Friend,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C6)]
#[packet_size(30)]
pub struct PartyInvitePacket {
    pub party_id: PartyId,
    #[length(24)]
    pub party_name: String,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ReputationEntry {
    pub reputation_type: u64,
    pub points: i64,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B8D)]
#[variable_length]
pub struct ReputationPacket {
    pub success: u8,
    #[repeating_remaining]
    pub entries: Vec<ReputationEntry>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Aliance {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct Antagonist {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x098A)]
#[variable_length]
pub struct ClanInfoPacket {
    pub clan_id: u32,
    #[length(24)]
    pub clan_name: String,
    #[length(24)]
    pub clan_master: String,
    #[length(16)]
    pub clan_map: String,
    #[new_derive]
    pub aliance_count: u8,
    #[new_derive]
    pub antagonist_count: u8,
    #[repeating(aliance_count)]
    pub aliances: Vec<Aliance>,
    #[repeating(antagonist_count)]
    pub antagonists: Vec<Antagonist>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0988)]
#[packet_size(6)]
pub struct ClanOnlineCountPacket {
    pub online_members: u16,
    pub maximum_members: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0192)]
#[packet_size(24)]
pub struct ChangeMapCellPacket {
    pub position: TilePosition,
    pub cell_type: u16,
    #[length(16)]
    pub map_name: String,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MarketItemInformation {
    pub name_id: u32,
    pub item_type: u8,
    pub price: Price,
    pub quantity: u32,
    pub weight: u16,
    pub location: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B7A)]
#[variable_length]
pub struct OpenMarketPacket {
    #[repeating_remaining]
    pub items: Vec<MarketItemInformation>,
}

/// One entry of the item list of an NPC shop.
///
/// Layout for PACKETVER 2019 (MAIN < 20210203), i.e. the layout used by the
/// CondaRO server: `{ price u32, discount price u32, item type u8, item id u32
/// }`.
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ShopItemInformation {
    pub price: Price,
    pub discount_price: Price,
    pub item_type: u8,
    pub item_id: ItemId,
}

/// Item list of an NPC shop (ZC_PC_PURCHASE_ITEMLIST, 2019 layout).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C6)]
#[variable_length]
pub struct ShopItemListPacket {
    #[repeating_remaining]
    pub items: Vec<ShopItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C4)]
#[packet_size(6)]
pub struct BuyOrSellPacket {
    pub shop_id: ShopId,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum BuyOrSellOption {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C5)]
pub struct SelectBuyOrSellPacket {
    pub shop_id: ShopId,
    pub option: BuyOrSellOption,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum BuyItemResult {
    #[numeric_value(0)]
    Successful,
    #[numeric_value(1)]
    NotEoughZeny,
    #[numeric_value(2)]
    WeightLimitExceeded,
    #[numeric_value(3)]
    TooManyItems,
    #[numeric_value(9)]
    TooManyOfThisItem,
    #[numeric_value(10)]
    PropsOpenAir,
    #[numeric_value(11)]
    ExchangeFailed,
    #[numeric_value(12)]
    ExchangeWellDone,
    #[numeric_value(13)]
    ItemSoldOut,
    #[numeric_value(14)]
    NotEnoughGoods,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct BuyItemInformation {
    pub amount: u16,
    pub item_id: ItemId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C8)]
#[variable_length]
pub struct BuyItemsPacket {
    #[repeating_remaining]
    pub items: Vec<BuyItemInformation>,
}

/// Result of a purchase at an NPC shop (ZC_PC_PURCHASE_RESULT, 2019 layout).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00CA)]
#[packet_size(3)]
pub struct BuyItemsResultPacket {
    pub result: BuyItemResult,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09D4)]
pub struct CloseShopPacket {}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SellItemInformation {
    pub inventory_index: InventoryIndex,
    pub price: Price,
    pub overcharge_price: Price,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C7)]
#[variable_length]
pub struct SellListPacket {
    #[repeating_remaining]
    pub items: Vec<SellItemInformation>,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct SoldItemInformation {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00C9)]
#[variable_length]
pub struct SellItemsPacket {
    #[repeating_remaining]
    pub items: Vec<SoldItemInformation>,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub enum SellItemsResult {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00CB)]
#[packet_size(3)]
pub struct SellItemsResultPacket {
    pub result: SellItemsResult,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0112)]
pub struct LevelUpSkillPacket {
    pub skill_id: SkillId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x010E)]
#[packet_size(11)]
pub struct UpdateSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    // TODO: bool,
    pub upgradable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0441)]
#[packet_size(4)]
pub struct RemoveSkillPacket {
    pub skill_id: SkillId,
}

/// `CZ_MOVE_ITEM_FROM_BODY_TO_CART` (0x0126 for PACKETVER 20190605): move an
/// item from the inventory into the merchant pushcart. The inventory index uses
/// the usual `+2` wire encoding of [InventoryIndex].
///
/// Layout: rAthena `packets_struct.hpp` `PACKET_CZ_MOVE_ITEM_FROM_BODY_TO_CART`
/// (header, index, amount), parsed at `clif.cpp:12544`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0125)]
#[packet_size(8)]
pub struct CartDeleteItemPacket2019 {
    pub cart_index: InventoryIndex,
    pub amount: u32,
}

/// 2019 `ZC_ACK_ADDITEM_TO_CART` (0x012C): moving an item into the cart failed
/// (0 = weight, 1 = item count), see `clif.cpp:7313`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x012C)]
#[packet_size(3)]
pub struct CartAddItemAckPacket2019 {
    pub result: u8,
}

/// 2019 `ZC_CARTOFF` (0x012B): the cart was removed; the client has to drop the
/// cart list and close the cart window (`clif.cpp:3284`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x012B)]
#[packet_size(2)]
pub struct CartOffPacket {}

/// `CZ_REQ_BUY_FROMMC` (0x0130): ask the map server for another player's
/// vending shop list. `clif_parse_VendingListReq` (`clif.cpp:14128`) reads the
/// account id and answers with `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0130)]
#[packet_size(6)]
pub struct RequestVendingListPacket {
    pub owner_account_id: AccountId,
}

/// 2019 `ZC_STORE_ENTRY` (0x0131): a vending shop entered the player's view.
/// Sent by `clif_showvendingboard` (`clif.cpp:7558`) with the owner account id
/// and the 80 byte shop title.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0131)]
#[packet_size(86)]
pub struct StoreEntryPacket2019 {
    pub owner_account_id: AccountId,
    #[length(80)]
    pub store_name: String,
}

/// 2019 `ZC_DISAPPEAR_ENTRY` (0x0132): the vending shop of the player left the
/// view or was closed (`clif_closevendingboard`, `clif.cpp:7576`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0132)]
#[packet_size(6)]
pub struct StoreEntryRemovedPacket2019 {
    pub owner_account_id: AccountId,
}

/// One row of `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800) for PACKETVER
/// 20190605.
///
/// Layout from rAthena `packets_struct.hpp`
/// `PACKET_ZC_PC_PURCHASE_ITEMLIST_FROMMC_sub` (the `PACKETVER >= 20100105`
/// variant) as written by `clif_vendinglist` (`clif.cpp:7592`): 63 bytes per
/// row. `cart_index` is the seller's cart slot in the usual `slot + 2`
/// encoding, which is exactly what the purchase request has to echo back.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct VendingItemInformation2019 {
    pub price: u32,
    pub amount: u16,
    pub cart_index: InventoryIndex,
    pub item_type: u8,
    pub item_id: ItemId,
    pub identified: u8,
    pub damaged: u8,
    pub refinement_level: u8,
    pub card: [u32; 4],
    pub option_data: [ItemOptions; 5],
    pub location: u32,
    pub view_sprite: u16,
}

/// 2019 `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800): the item list of another
/// player's vending shop. rAthena sends `AID`, the shop's `vender_id` and one
/// [VendingItemInformation2019] per row (`clif.cpp:7592`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0800)]
#[variable_length]
pub struct VendingItemListPacket2019 {
    pub owner_account_id: AccountId,
    pub shop_id: u32,
    #[repeating_remaining]
    pub items: Vec<VendingItemInformation2019>,
}

/// One `{ <amount>.W <index>.W }` entry of a
/// `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (0x0134) request.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PurchaseFromShopEntry {
    pub amount: u16,
    pub cart_index: InventoryIndex,
}

/// `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (0x0134): buy items from another player's
/// vending shop. The server reads `{ amount, index }` rows and matches the cart
/// slot index against the shop it last sent (`clif.cpp:14138`,
/// `vending.cpp:129`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0134)]
#[variable_length]
pub struct PurchaseFromShopPacket {
    pub owner_account_id: AccountId,
    #[repeating_remaining]
    pub items: Vec<PurchaseFromShopEntry>,
}

/// 2019 `ZC_PC_PURCHASE_RESULT_FROMMC` (0x0135): the vending purchase failed
/// (the server does not send a success ack; the item arrives through the
/// regular inventory packets). Result codes: 1 not enough zeny, 2 overweight,
/// 4 out of stock, 5 trading, 6 stale shop, 7 no sales info
/// (`clif.cpp:7642`, `clif.hpp:818`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0135)]
#[packet_size(7)]
pub struct VendingPurchaseResultPacket2019 {
    pub cart_index: InventoryIndex,
    pub amount: u16,
    pub result: u8,
}

/// Guild chat message sent by the client (`CZ_GUILD_CHAT`, 0x017E).
///
/// rAthena validates the message with `clif_process_message`, exactly like the
/// public and party chat, so the payload has to start with
/// `<character name> : ` and the trailing `NUL` is not counted in the declared
/// packet length.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x017E)]
#[variable_length]
#[allow_packet_size_mismatch]
pub struct GuildChatRequestPacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

/// Guild chat message sent by the server (`ZC_GUILD_CHAT`, 0x017F).
///
/// The payload is the full `<name> : <message>` line like the party chat
/// packet; the packet length includes the trailing `NUL`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x017F)]
#[variable_length]
pub struct GuildChatReceivedPacket {
    #[length_remaining]
    pub message: String,
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

#[cfg(test)]
mod warp_point_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;

    /// `ZC_WARPLIST` (0x0ABE) carries the skill id and NUL-padded 16 byte map
    /// names; the client answers with `CZ_SELECT_WARPPOINT` (0x011B).
    #[test]
    fn warp_point_list_packet_parses_2019_layout() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x0ABEu16.to_le_bytes());
        bytes.extend_from_slice(&22u16.to_le_bytes()); // 6 bytes header + one 16 byte map
        bytes.extend_from_slice(&27u16.to_le_bytes()); // AL_WARP
        bytes.extend_from_slice(b"prontera.gat\0\0\0\0");

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = WarpPointListPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(packet.skill_id, SkillId(27));
        assert_eq!(packet.maps.len(), 1);
        assert_eq!(packet.maps[0].map_name.trim_end_matches('\0'), "prontera.gat");
    }

    #[test]
    fn select_warp_point_packet_has_2019_layout() {
        let packet = SelectWarpPointPacket {
            skill_id: SkillId(27),
            map_name: "prontera.gat".to_string(),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let bytes = writer.into_inner();

        // Header (2) + skill id (2) + map name (16).
        assert_eq!(bytes.len(), 20);
        assert_eq!(&bytes[..2], &[0x1B, 0x01]);
        assert_eq!(u16::from_le_bytes([bytes[2], bytes[3]]), 27);
        assert_eq!(&bytes[4..16], b"prontera.gat");
        assert_eq!(&bytes[16..20], &[0, 0, 0, 0]);
    }
}

#[cfg(test)]
mod sound_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;

    #[test]
    fn sound_packet_has_2019_layout() {
        let packet = SoundPacket {
            name: "effect\\ef_blessing.wav".to_string(),
            act: 1,
            term: 500,
            account_id: AccountId(0x1234_5678),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();

        // Header (2) + name (24) + act (1) + term (4) + account id (4).
        assert_eq!(writer.as_slice().len(), 35);
        // Header 0x01D3 little endian.
        assert_eq!(&writer.as_slice()[..2], &[0xD3, 0x01]);

        let mut reader = ByteReader::without_metadata(writer.as_slice());
        let parsed = SoundPacket::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(parsed.name, "effect\\ef_blessing.wav");
        assert_eq!(parsed.act, 1);
        assert_eq!(parsed.term, 500);
        assert_eq!(parsed.account_id, AccountId(0x1234_5678));
    }

    #[test]
    fn sound_packet_name_is_nul_terminated_within_24_bytes() {
        let mut writer = ByteWriter::new();
        SoundPacket {
            name: "ef_hammerfall.wav".to_string(),
            act: 0,
            term: 0,
            account_id: AccountId(1),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();

        // Header, then the name, zero padded up to 24 bytes.
        let name_bytes = &writer.as_slice()[2..26];
        assert_eq!(&name_bytes[..17], b"ef_hammerfall.wav");
        assert!(name_bytes[17..].iter().all(|byte| *byte == 0));
    }
}

// ---------------------------------------------------------------------------
// Party packets for the 2019 rAthena server (PACKETVER 20190605).
//
// Before these were registered the login burst of a character that is in a
// party contained unknown fixed-size packets. The client's stream resync reads
// bytes 2..4 of an unknown packet as a packet length (only valid for variable
// length packets), so it usually fails and discards the rest of the TCP read,
// swallowing NPC spawns and the skill list that follow the party packets.
// ---------------------------------------------------------------------------

/// Result of creating a party (`ZC_ACK_MAKE_GROUP`, 0x00FA).
///
/// * `0` = party created
/// * `1` = party name already exists
/// * `2` = already in a party
/// * `3` = cannot create a party on this map
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00FA)]
#[packet_size(3)]
pub struct PartyCreationResultPacket {
    pub result: u8,
}

/// Result of a party invitation (`ZC_PARTY_JOIN_REQ_ACK`, 0x02C5).
///
/// `character_name` is the invitee and `result` is one of the rAthena
/// `e_party_invite_reply` values (0 = already in a party, 1 = rejected,
/// 2 = accepted, 3 = party full, ...).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C5)]
#[packet_size(30)]
pub struct PartyInviteReplyPacket2019 {
    #[length(24)]
    pub character_name: String,
    pub result: i32,
}

/// `ZC_ADD_MEMBER_TO_GROUP` (0x0AE4, 2019 layout): a party member joined or
/// changed map. 89 bytes.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE4)]
#[packet_size(89)]
pub struct AddPartyMemberPacket2019 {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    /// `0` = party leader, `1` = normal member (rAthena sends the inverted
    /// "is normal member" role value here).
    pub role: u32,
    pub job_id: JobId,
    pub base_level: u16,
    pub position: TilePosition,
    /// `0` = connected, `1` = disconnected.
    pub offline: u8,
    #[length(24)]
    pub party_name: String,
    #[length(24)]
    pub player_name: String,
    #[length(16)]
    pub map_name: String,
    pub share_pickup: i8,
    pub share_loot: i8,
}

/// One entry of a `ZC_GROUP_LIST` (0x0AE5) party roster: 54 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct PartyListMember {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
    #[length(16)]
    pub map_name: String,
    /// `0` = party leader, `1` = normal member.
    pub role: u8,
    /// `0` = connected, `1` = disconnected.
    pub offline: u8,
    pub job_id: JobId,
    pub base_level: u16,
}

/// `ZC_GROUP_LIST` (0x0AE5, 2019 layout): the complete party roster.
///
/// Layout: `{ header }.{W} { length }.{W} { party name }.{24B}
/// { account id }.{L} { char id }.{L} { name }.{24B} { map name }.{16B}
/// { role }.{B} { offline }.{B} { job }.{W} { base level }.{W}` repeated.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE5)]
#[variable_length]
pub struct PartyListPacket2019 {
    #[length(24)]
    pub party_name: String,
    #[repeating_remaining]
    pub members: Vec<PartyListMember>,
}

/// `ZC_DELETE_MEMBER_FROM_GROUP` (0x0105): a member left or was expelled.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0105)]
#[packet_size(31)]
pub struct RemovePartyMemberPacket2019 {
    pub account_id: AccountId,
    #[length(24)]
    pub character_name: String,
    /// rAthena `e_party_member_withdraw`.
    pub result: u8,
}

/// `ZC_NOTIFY_POSITION_TO_GROUPM` (0x0107): party member position on the map.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0107)]
#[packet_size(10)]
pub struct PartyMemberPositionPacket2019 {
    pub account_id: AccountId,
    pub position: TilePosition,
}

/// `ZC_NOTIFY_CHAT_PARTY` (0x0109): party chat message.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0109)]
#[variable_length]
pub struct PartyChatMessagePacket2019 {
    pub account_id: AccountId,
    #[length_remaining]
    pub message: String,
}

/// `ZC_NOTIFY_MEMBERINFO_TO_GROUPM` (0x0ABD): party member job and base level.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0ABD)]
#[packet_size(10)]
pub struct PartyMemberJobLevelPacket2019 {
    pub account_id: AccountId,
    pub job_id: JobId,
    pub base_level: u16,
}

/// `ZC_GROUP_ISALIVE` (0x0AB2): party member death state.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AB2)]
#[packet_size(7)]
pub struct PartyMemberAlivePacket2019 {
    pub account_id: AccountId,
    pub is_dead: u8,
}

/// `ZC_PARTY_REQ_MASTER_TO_JOIN` (0x0AE7): party booking join request.
///
/// Party booking (2019 "party search") is not used by the CondaRO client yet,
/// but the packet is registered so that a party booking broadcast cannot
/// desynchronize the packet stream.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE7)]
#[packet_size(38)]
pub struct PartyBookingJoinRequestPacket2019 {
    pub character_id: CharacterId,
    pub account_id: AccountId,
    #[length(24)]
    pub name: String,
    pub position: TilePosition,
}

/// `ZC_PARTY_JOIN_REQ_ACK_FROM_MASTER` (0x0AFA): party booking join result.
///
/// Registered for framing safety only, see [PartyBookingJoinRequestPacket2019].
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AFA)]
#[packet_size(58)]
pub struct PartyBookingJoinResultPacket2019 {
    #[length(24)]
    pub player_name: String,
    #[length(24)]
    pub party_name: String,
    pub account_id: AccountId,
    pub refused: u32,
}

/// `CZ_MAKE_GROUP` (0x00F9): create a party with the given name.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F9)]
pub struct CreatePartyPacket {
    #[length(24)]
    pub party_name: String,
}

/// `CZ_PARTY_JOIN_REQ` (0x02C4): invite a character by name.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C4)]
pub struct PartyInviteRequestPacket2019 {
    #[length(24)]
    pub character_name: String,
}

/// `CZ_PARTY_JOIN_REQ_ACK` (0x02C7): accept (1) or reject (0) a party invite.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x02C7)]
pub struct PartyInviteReplyRequestPacket2019 {
    pub party_id: PartyId,
    pub accept: u8,
}

/// `CZ_REQ_LEAVE_GROUP` (0x0100): leave the current party.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0100)]
pub struct LeavePartyPacket {}

/// `CZ_REQ_EXPEL_GROUP_MEMBER` (0x0103): expel a party member (leader only).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0103)]
pub struct ExpelPartyMemberPacket {
    pub account_id: AccountId,
    #[length(24)]
    pub character_name: String,
}

/// `CZ_REQUEST_CHAT_PARTY` (0x0108): party chat message.
///
/// The server expects the same `<name> : <message>` format (NUL terminated) as
/// the regular global chat.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0108)]
#[variable_length]
// See [`GlobalMessagePacket`]: client-to-server packet whose trailing `NUL`
// is not counted in the declared length.
#[allow_packet_size_mismatch]
pub struct PartyChatRequestPacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

/// `ZC_PARTY_OPTION` / `ZC_REQ_GROUPINFO_CHANGE_V2` (0x07D8): the party EXP
/// sharing and item rules.
///
/// rAthena sends this during the map login burst of a character that is in a
/// party (see `clif_party_option`). An unregistered fixed-size packet makes the
/// client's stream resync mis-read bytes 2..4 as a length and discard the rest
/// of the TCP read, which swallowed the NPC spawn burst and the skill list for
/// party characters. Layout: `{ header }.{W} { exp option }.{L}
/// { item pickup }.{B} { item share }.{B}` (8 bytes).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x07D8)]
#[packet_size(8)]
pub struct PartyOptionPacket2019 {
    /// 0 = EXP sharing disabled, 1 = enabled, 2 = cannot be changed.
    pub experience_option: u32,
    /// 1 = "even share" item pickup rule.
    pub item_pickup: u8,
    /// 1 = item sharing enabled.
    pub item_share: u8,
}

/// `SC_NOTIFY_BAN` (0x0081) as sent by the **map** server when it kicks an
/// already connected session (e.g. "ID already logged in" after a stale
/// connection). The login/character server variant is [LoginFailedPacket].
///
/// Unregistered this is a 3-byte unknown packet: the stream resync then reads
/// the error code as a length and discards the rest of the TCP read.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0081)]
#[packet_size(3)]
pub struct NotifyBanPacket {
    pub error_code: u8,
}

// ---------------------------------------------------------------------------
// Rodex mail packets for the 2019 rAthena server (PACKETVER 20190605).
//
// rAthena replaced the old mailbox with RODEX in 20150513; at 20190605 the
// server only ever sends the RODEX variants (see the `#if PACKETVER < 20150513`
// branches in src/map/clif.cpp):
//
// * `ZC_ACK_MAIL_LIST3` (0x0AC2) - mailbox list, clif.cpp
//   clif_Mail_refreshinbox (PACKETVER >= 20170419 selects cmd 0xac2).
// * `ZC_ACK_READ_RODEX` (0x09EB) - opened mail, clif.cpp clif_Mail_read.
// * `ZC_ACK_ZENY_FROM_MAIL` (0x09F2) / `ZC_ACK_ITEM_FROM_MAIL` (0x09F4) -
//   attachment claim result, clif.cpp clif_Mail_getattachment.
// * `ZC_ACK_DELETE_MAIL` (0x09F6) - delete result (only sent on success),
//   clif.cpp clif_mail_delete.
// * `ZC_MAIL_WINDOWS` (0x0260) - server-side request to open the mailbox
//   (atcommand `@mail`, script command `openmail`), clif.cpp clif_Mail_window.
// * `ZC_NOTIFY_UNREADMAIL` (0x09E7) - the existing [NewMailStatusPacket].
// ---------------------------------------------------------------------------

/// One inbox row of `ZC_ACK_MAIL_LIST3` (0x0AC2); rAthena's `maillistinfo`
/// struct (src/map/packets_struct.hpp).
///
/// Layout: `{ open type }.{B} { mail id }.{Q} { read }.{B} { mail type }.{B}
/// { sender }.{24B} { scheduled deletion }.{L} { title length }.{W}
/// { title }.{?B}` for PACKETVER >= 20170419.
#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MailListEntry2019 {
    /// Mailbox tab (`mail_inbox_type`): 0 = normal, 1 = account, 2 = returned.
    pub open_type: u8,
    pub mail_id: u64,
    /// `1` = read, `0` = unread (`msg->status != MAIL_UNREAD`).
    pub read: u8,
    /// Attachment flags (`mail_type`): 0x2 = zeny, 0x4 = items, 0x8 = from NPC.
    pub mail_type: u8,
    #[length(24)]
    pub sender: String,
    /// Seconds until the server automatically deletes the mail. The server
    /// fakes this to one year when auto deletion is disabled.
    pub expires_in_seconds: u32,
    /// Length of `title` in bytes including the terminating NUL.
    pub title_length: u16,
    #[length(title_length)]
    pub title: String,
}

/// The repeated part of [`MailboxListPacket2019`].
///
/// The 2019 list packet has no entry count (rAthena writes a literal `1` where
/// older packet versions had the page type) and every row carries a variable
/// length title, so the rows are parsed until the declared packet end instead
/// of using `#[repeating_remaining]` (which requires fixed size entries).
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MailListEntries2019 {
    pub mails: Vec<MailListEntry2019>,
}

impl FromBytes for MailListEntries2019 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let mut mails = Vec::new();

        // `from_n_bytes` (used by `#[length_remaining]`) installs a limit for
        // exactly the remaining packet bytes, so this ends at the packet end.
        while !byte_reader.is_empty() {
            mails.push(MailListEntry2019::from_bytes(byte_reader).trace::<Self>()?);
        }

        Ok(Self { mails })
    }
}

impl ToBytes for MailListEntries2019 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.mails.to_bytes(byte_writer)
    }
}

/// `ZC_ACK_MAIL_LIST3` (0x0AC2), the 2019 mailbox list.
///
/// Layout (src/map/clif.cpp clif_Mail_refreshinbox): `{ header }.{W}
/// { length }.{W} { unknown = 1 }.{B}` followed by one [MailListEntry2019] per
/// mail. rAthena sends the whole inbox, not a page, at this packet version.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AC2)]
#[variable_length]
pub struct MailboxListPacket2019 {
    /// Written as a literal `1` by rAthena (`WFIFOB(fd, 4) = 1; // Unknown`).
    pub unknown: u8,
    #[length_remaining]
    pub entries: MailListEntries2019,
}

/// One item attachment of `ZC_ACK_READ_RODEX` (0x09EB); rAthena's
/// `PACKET_ZC_ACK_READ_RODEX_SUB`.
///
/// At PACKETVER 20190605 (MAIN_NUM >= 20181121) `EQUIPSLOTINFO` is
/// `uint32 card[4]`, so one entry is 59 bytes:
/// `{ amount }.{W} { item id }.{L} { identified }.{B} { damaged }.{B}
/// { refine }.{B} { cards }.{16B} { location }.{L} { item type }.{B}
/// { view sprite }.{W} { bind on equip }.{W} { options }.{5*5B}`.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MailAttachmentItem2019 {
    pub amount: u16,
    pub item_id: u32,
    pub identified: u8,
    pub damaged: u8,
    pub refine: u8,
    pub card: [u32; 4],
    pub location: u32,
    pub item_type: u8,
    pub view_sprite: u16,
    pub bind_on_equip: u16,
    pub options: [ItemOptions; 5],
}

/// The repeated attachment block of [`MailReadPacket2019`], parsed until the
/// declared packet end (the server appends exactly `item count` 59 byte rows).
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MailAttachmentItems2019 {
    pub items: Vec<MailAttachmentItem2019>,
}

impl FromBytes for MailAttachmentItems2019 {
    fn from_bytes(byte_reader: &mut ByteReader) -> ConversionResult<Self> {
        let mut items = Vec::new();

        while !byte_reader.is_empty() {
            items.push(MailAttachmentItem2019::from_bytes(byte_reader).trace::<Self>()?);
        }

        Ok(Self { items })
    }
}

impl ToBytes for MailAttachmentItems2019 {
    fn to_bytes(&self, byte_writer: &mut ByteWriter) -> ConversionResult<usize> {
        self.items.to_bytes(byte_writer)
    }
}

/// `ZC_ACK_READ_RODEX` (0x09EB): the opened mail body and its attachments.
///
/// Layout (src/map/clif.cpp clif_Mail_read): `{ header }.{W} { length }.{W}
/// { open type }.{B} { mail id }.{Q} { text length }.{W} { zeny }.{Q}
/// { item count }.{B} { body }.{?B} { items }*item count`. `text length`
/// includes the terminating NUL.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09EB)]
#[variable_length]
pub struct MailReadPacket2019 {
    /// Mailbox tab the mail is in (`mail_inbox_type`).
    pub open_type: u8,
    pub mail_id: u64,
    /// Length of `body` including the terminating NUL.
    pub text_length: u16,
    pub zeny: u64,
    /// Number of [MailAttachmentItem2019] rows that follow the body.
    pub item_count: u8,
    #[length(text_length)]
    pub body: String,
    #[length_remaining]
    pub items: MailAttachmentItems2019,
}

/// `ZC_ACK_ZENY_FROM_MAIL` (0x09F2): result of claiming the zeny attachment.
///
/// `result`: 0 = success, 1 = too much zeny, 2 = too many items (rAthena
/// `clif_mail_getattachment`). The server separately sends the normal zeny
/// status-update packet, so the client only has to report failures.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F2)]
#[packet_size(12)]
pub struct MailZenyClaimResultPacket {
    pub mail_id: u64,
    pub open_type: u8,
    pub result: u8,
}

/// `ZC_ACK_ITEM_FROM_MAIL` (0x09F4): result of claiming the item attachments.
///
/// `result`: 0 = success, 1 = failure, 2 = too many items / too heavy. The
/// claimed items themselves arrive as normal `ZC_ITEM_PICKUP_ACK` (0x0A37)
/// packets, which the client already handles.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F4)]
#[packet_size(12)]
pub struct MailItemClaimResultPacket {
    pub mail_id: u64,
    pub open_type: u8,
    pub result: u8,
}

/// `ZC_ACK_DELETE_MAIL` (0x09F6): a mail was deleted.
///
/// rAthena only sends this packet on success (`clif_mail_delete`); failures are
/// only reported in the server log.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F6)]
#[packet_size(11)]
pub struct MailDeleteAckPacket {
    pub open_type: u8,
    pub mail_id: u64,
}

/// `ZC_MAIL_WINDOWS` (0x0260): the server asks the client to open or close the
/// mailbox (atcommand `@mail`, script command `openmail`; clif.cpp
/// clif_Mail_window).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0260)]
#[packet_size(6)]
pub struct MailboxWindowPacket {
    /// `0` = open, `1` = close.
    pub window_type: i32,
}

/// `CZ_REQ_OPEN_MAILBOX2` (0x0AC0): open the mailbox and request the list.
///
/// The 2019 client sends the three upper mail ids of the normal/returned/
/// account tabs; all zero requests the first page of the normal inbox.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AC0)]
pub struct MailboxOpenRequestPacket {
    pub mail_id: u64,
    pub unknown: [u8; 16],
}

/// `CZ_REQ_REFRESH_MAIL_LIST2` (0x0AC1): force a mailbox refresh.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AC1)]
pub struct MailboxRefreshRequestPacket {
    pub mail_id: u64,
    pub unknown: [u8; 16],
}

/// `CZ_REQ_READ_MAIL` (0x09EA): request the body of one mail.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09EA)]
pub struct MailReadRequestPacket {
    pub open_type: u8,
    pub mail_id: u64,
}

/// `CZ_REQ_ZENY_FROM_MAIL` (0x09F1): claim the zeny attachment.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F1)]
pub struct MailClaimZenyRequestPacket {
    pub mail_id: u64,
    pub open_type: u8,
}

/// `CZ_REQ_ITEM_FROM_MAIL` (0x09F3): claim the item attachments.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F3)]
pub struct MailClaimItemRequestPacket {
    pub mail_id: u64,
    pub open_type: u8,
}

/// `CZ_REQ_DELETE_MAIL` (0x09F5): delete a mail without attachments.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09F5)]
pub struct MailDeleteRequestPacket {
    pub open_type: u8,
    pub mail_id: u64,
}

// ===========================================================================
// Guild system (PACKETVER 20190605)
//
// rAthena only enables the guild packets for 2019 at these opcodes:
//
// * 0x014E `ZC_ACK_GUILD_MENUINTERFACE` (clif.cpp:8715) — window tab/rights
//   mask
// * 0x0A84 `ZC_GUILD_INFO` (clif.cpp:8734) — basic information
// * 0x014C `ZC_MYGUILD_BASIC_INFO` (clif.cpp:8772) — alliance/opposition list
// * 0x0AA5 `ZC_MEMBERMGR_INFO` (clif.cpp:8813) — member manager list
// * 0x0166 `ZC_POSITION_ID_NAME_INFO` (clif.cpp:8863) — position names
// * 0x0160 `ZC_POSITION_INFO` (clif.cpp:8890) — position rights
// * 0x0174 `ZC_ACK_CHANGE_GUILD_POSITIONINFO` (clif.cpp:8925)
// * 0x0156 `ZC_ACK_REQ_CHANGE_MEMBERS` (clif.cpp:8956)
// * 0x016F `ZC_GUILD_NOTICE` (clif.cpp:9070)
// * 0x0AF7 `ZC_ACK_REQNAME_BYGID` (clif.cpp:7007) — offline member names
//
// The member list of this PACKETVER does NOT contain the character name (the
// name was only added in 2016/2020 layouts, see the `#if` chain in
// `src/map/packets_struct.hpp:4759`). The classic client resolves the names of
// offline members with `CZ_REQNAME_BYGID` and gets one `ZC_ACK_REQNAME_BYGID`
// back per member; `0x094C` is the 2013+ opcode of that request (the last
// `clif_parse_SolveCharName` registration in `clif_packetdb.hpp:1636`).
// ===========================================================================

/// One member row of `ZC_MEMBERMGR_INFO` (0x0AA5) at PACKETVER 20190605.
///
/// rAthena layout (`src/map/packets_struct.hpp:4759`): account id, char id,
/// hair, hair color, sex, job, level, contribution EXP, online state,
/// position id and last login time. The 2016+ layout has **no name field**;
/// the client requests it separately by character id.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct GuildMemberInfo {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub hair: u16,
    pub hair_color: u16,
    pub sex: u16,
    pub job_id: JobId,
    pub level: u16,
    pub contribution_exp: u32,
    /// `0` = offline, non-zero = online (`member.online`).
    pub online: i32,
    pub position_id: i32,
    pub last_login: u32,
}

/// `ZC_MEMBERMGR_INFO` (0x0AA5): the complete guild member manager list.
///
/// `clif_guild_memberlist` (clif.cpp:8813) writes one
/// [`GuildMemberInfo`] per member into this variable-length packet.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AA5)]
#[variable_length]
pub struct GuildMemberManagerInfoPacket {
    #[repeating_remaining]
    pub members: Vec<GuildMemberInfo>,
}

/// `ZC_ACK_GUILD_MENUINTERFACE` (0x014E), 6 bytes.
///
/// `clif_guild_masterormember` (clif.cpp:8715) sends `0xD7` for the guild
/// master and `0x57` for a normal member. The `0x80` bit is therefore the
/// "guild master / notice edit" flag (the other bits are tab permissions:
/// `0x01` member manager, `0x02` positions, `0x04` skills, `0x10` expulsion
/// list, `0x40` all-guild list).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x014E)]
#[packet_size(6)]
pub struct GuildMenuInterfacePacket {
    pub menu_flags: u32,
}

/// `ZC_GUILD_INFO` (0x0A84), the 2016+ layout, 94 bytes.
///
/// `clif_guild_basicinfo` (clif.cpp:8734) writes the guild id, level, connected
/// and maximum member count, average level, EXP/max EXP, tax points, honor,
/// virtue, emblem version, name, managed land, zeny and the guild master's
/// character id. The master **name** is not part of this layout; it is
/// resolved from the member list row whose character id equals
/// [`Self::master_character_id`] (position 0).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A84)]
#[packet_size(94)]
pub struct GuildInfoPacket {
    pub guild_id: u32,
    pub level: u32,
    /// Number of currently connected members (`guild.connect_member`).
    pub member_count: u32,
    pub max_member_count: u32,
    pub average_level: u32,
    pub experience: u32,
    pub max_experience: u32,
    /// Tax points (rAthena currently always sends 0).
    pub points: u32,
    pub honor: u32,
    pub virtue: u32,
    /// Emblem version, used to refresh the cached emblem.
    pub emblem_id: u32,
    #[length(24)]
    pub guild_name: String,
    /// Managed castle/land name, 16 bytes (`MAP_NAME_LENGTH_EXT`).
    #[length(16)]
    pub manage_land: String,
    pub zeny: u32,
    /// Character id of the guild master (`guild.member[0].char_id`).
    pub master_character_id: CharacterId,
}

/// One position name of `ZC_POSITION_ID_NAME_INFO` (0x0166).
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct GuildPositionNameEntry {
    pub position_id: u32,
    #[length(24)]
    pub name: String,
}

/// `ZC_POSITION_ID_NAME_INFO` (0x0166).
///
/// The server always writes all [`MAX_GUILD_POSITIONS`] entries (20 * 28 bytes
/// plus the length field = 564 bytes, `clif_guild_positionnamelist` at
/// clif.cpp:8863). Modeled as variable length because the packet carries its
/// own length field and the 2022 client reads a different struct there.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0166)]
#[variable_length]
pub struct GuildPositionNameInfoPacket {
    #[repeating_remaining]
    pub positions: Vec<GuildPositionNameEntry>,
}

/// Maximum number of guild positions in the 2019 client (`MAX_GUILDPOSITION`).
pub const MAX_GUILD_POSITIONS: usize = 20;

/// One entry of `ZC_POSITION_INFO` (0x0160).
///
/// `mode` uses `enum e_guild_permission` (`src/common/mmo.hpp:885`):
/// `0x001` invite, `0x010` expel, `0x100` storage.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct GuildPositionInfoEntry {
    pub position_id: u32,
    pub mode: u32,
    pub ranking: u32,
    pub pay_rate: u32,
}

/// `ZC_POSITION_INFO` (0x0160): the permission/pay-rate row of every position
/// (`clif_guild_positioninfolist`, clif.cpp:8890).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0160)]
#[variable_length]
pub struct GuildPositionInfoPacket {
    #[repeating_remaining]
    pub positions: Vec<GuildPositionInfoEntry>,
}

/// One entry of `ZC_ACK_CHANGE_GUILD_POSITIONINFO` (0x0174), 40 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct GuildPositionChangedEntry {
    pub position_id: u32,
    pub mode: u32,
    pub ranking: u32,
    pub pay_rate: u32,
    #[length(24)]
    pub name: String,
}

/// `ZC_ACK_CHANGE_GUILD_POSITIONINFO` (0x0174): a position was renamed or its
/// rights changed (`clif_guild_positionchanged`, clif.cpp:8925).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0174)]
#[variable_length]
pub struct GuildPositionChangedPacket {
    #[repeating_remaining]
    pub positions: Vec<GuildPositionChangedEntry>,
}

/// One entry of `ZC_ACK_REQ_CHANGE_MEMBERS` (0x0156), 12 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct GuildMemberPositionChangedEntry {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub position_id: i32,
}

/// `ZC_ACK_REQ_CHANGE_MEMBERS` (0x0156): a member was moved to another
/// position (`clif_guild_memberpositionchanged`, clif.cpp:8956).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0156)]
#[variable_length]
pub struct GuildMemberPositionChangedPacket {
    #[repeating_remaining]
    pub members: Vec<GuildMemberPositionChangedEntry>,
}

/// One ally/opponent row of `ZC_MYGUILD_BASIC_INFO` (0x014C), 32 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct GuildAllianceInfo {
    /// `0` = ally, `1` = antagonist (`guild_alliance.opposition`,
    /// `src/common/mmo.hpp:787`).
    pub relation: u32,
    pub guild_id: u32,
    #[length(24)]
    pub name: String,
}

/// `ZC_MYGUILD_BASIC_INFO` (0x014C): the alliance and opposition list
/// (`clif_guild_allianceinfo`, clif.cpp:8772).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x014C)]
#[variable_length]
pub struct GuildAllianceInfoPacket {
    #[repeating_remaining]
    pub alliances: Vec<GuildAllianceInfo>,
}

/// `ZC_GUILD_NOTICE` (0x016F), 182 bytes: `subject[60]` + `notice[120]`
/// (`clif_guild_notice`, clif.cpp:9070).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x016F)]
#[packet_size(182)]
pub struct GuildNoticePacket {
    #[length(60)]
    pub subject: String,
    #[length(120)]
    pub notice: String,
}

/// `ZC_ACK_REQNAME_BYGID` (0x0AF7), 32 bytes at PACKETVER >= 20180221.
///
/// rAthena answers `CZ_REQNAME_BYGID` with `clif_solved_charname`
/// (clif.cpp:7007). `flag` is 3 when the character server returned a name and
/// 2 when the char id is unknown.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AF7)]
#[packet_size(32)]
pub struct GuildMemberNamePacket {
    pub flag: u16,
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
}

/// `CZ_REQ_GUILD_MENUINTERFACE` (0x014D), 2 bytes: request the guild window tab
/// and master mask (`clif_parse_GuildCheckMaster`, clif.cpp:14235).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x014D)]
pub struct GuildMenuInterfaceRequestPacket {}

/// `CZ_REQ_GUILD_MENU` (0x014F), 6 bytes: request one guild window tab.
///
/// `clif_parse_GuildRequestInfo` (clif.cpp:14253):
/// `0` basic info + alliance list, `1` member list + position names,
/// `2` position names + position rights, `3` skills, `4` expulsion list.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x014F)]
pub struct GuildMenuRequestPacket {
    pub menu: u32,
}

/// `CZ_REQNAME_BYGID` (0x094C at PACKETVER 20190605), 6 bytes.
///
/// The last `clif_parse_SolveCharName` registration for the 2019 client is
/// `clif_packetdb.hpp:1636`; 0x0369 from the old table is
/// `CZ_ACTION_REQUEST` at this PACKETVER and must not be used.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x094C)]
pub struct GuildMemberNameRequestPacket {
    pub character_id: CharacterId,
}

/// `CZ_REQ_JOIN_GUILD2` (0x0916): invite a character by name
/// (`clif_parse_GuildInvite2`, clif.cpp:14483).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0916)]
pub struct GuildInviteRequestPacket {
    #[length(24)]
    pub name: String,
}

/// `CZ_REQ_LEAVE_GUILD` (0x0159), 54 bytes: leave a guild.
///
/// `clif_parse_GuildLeave` (clif.cpp:14516) requires the guild id, the own
/// account/character id and a 40-byte message.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0159)]
#[packet_size(54)]
pub struct GuildLeaveRequestPacket {
    pub guild_id: u32,
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(40)]
    pub message: String,
}

/// `CZ_REQ_BAN_GUILD` (0x015B), 54 bytes: expel a guild member.
///
/// `clif_parse_GuildExpulsion` (clif.cpp:14529). The message is the reason the
/// server stores in the expulsion list.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x015B)]
#[packet_size(54)]
pub struct GuildExpelRequestPacket {
    pub guild_id: u32,
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(40)]
    pub message: String,
}

/// `CZ_REQ_GUILD_NOTICE` (0x016E), 186 bytes: change subject and notice.
///
/// `clif_parse_GuildChangeNotice` (clif.cpp:14452); the packet db entry is
/// `parseable_packet(0x016e,186,clif_parse_GuildChangeNotice,2,6,66)`, i.e.
/// guild id at offset 2, subject at offset 6 and notice at offset 66.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x016E)]
#[packet_size(186)]
pub struct GuildNoticeChangeRequestPacket {
    pub guild_id: u32,
    #[length(60)]
    pub subject: String,
    #[length(120)]
    pub notice: String,
}

#[cfg(test)]
mod guild_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;

    fn guild_member(account_id: u32, character_id: u32, online: i32) -> GuildMemberInfo {
        GuildMemberInfo {
            account_id: AccountId(account_id),
            character_id: CharacterId(character_id),
            hair: 7,
            hair_color: 3,
            sex: 1,
            job_id: JobId(14),
            level: 99,
            contribution_exp: 12_345,
            online,
            position_id: 2,
            last_login: 1_700_000_000,
        }
    }

    #[test]
    fn guild_member_info_is_34_bytes() {
        // rAthena `GUILD_MEMBER_INFO` (packets_struct.hpp:4759) at 20190605:
        // 4 + 4 + 5 * 2 + 4 + 4 + 4 + 4 = 34 bytes.
        assert_eq!(<GuildMemberInfo as ragnarok_bytes::FixedByteSize>::size_in_bytes(), 34);

        let member = guild_member(150_009, 150_009, 1);
        let mut writer = ByteWriter::new();
        member.to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 34);
    }

    #[test]
    fn guild_info_packet_is_94_bytes_and_round_trips() {
        let packet = GuildInfoPacket {
            guild_id: 7,
            level: 5,
            member_count: 3,
            max_member_count: 20,
            average_level: 90,
            experience: 1_000,
            max_experience: 2_000,
            points: 0,
            honor: 0,
            virtue: 0,
            emblem_id: 42,
            guild_name: "Domination".to_owned(),
            manage_land: "Prontera".to_owned(),
            zeny: 0,
            master_character_id: CharacterId(150_009),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 94, "ZC_GUILD_INFO is 94 bytes at 20190605");

        let mut reader = ByteReader::without_metadata(writer.as_slice());
        let parsed = GuildInfoPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(parsed.guild_name, "Domination");
        assert_eq!(parsed.master_character_id, CharacterId(150_009));
        assert_eq!(reader.get_offset(), 94);
    }

    #[test]
    fn member_manager_packet_is_variable_and_parses_the_2019_rows() {
        let packet = GuildMemberManagerInfoPacket {
            members: vec![guild_member(150_009, 150_009, 1), guild_member(150_010, 150_010, 0)],
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        // 2 header + 2 length + 2 * 34.
        assert_eq!(writer.as_slice().len(), 72);

        let mut reader = ByteReader::without_metadata(writer.as_slice());
        let parsed = GuildMemberManagerInfoPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(parsed.members.len(), 2);
        assert_eq!(parsed.members[0].account_id, AccountId(150_009));
        assert_eq!(parsed.members[1].online, 0);
        assert_eq!(reader.get_offset(), 72);
    }

    #[test]
    fn position_name_packet_parses_all_20_entries() {
        let positions = (0..MAX_GUILD_POSITIONS)
            .map(|index| GuildPositionNameEntry {
                position_id: index as u32,
                name: format!("Position {index}"),
            })
            .collect();

        let packet = GuildPositionNameInfoPacket { positions };
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 564, "20 * 28 + 4 length/header");

        let mut reader = ByteReader::without_metadata(writer.as_slice());
        let parsed = GuildPositionNameInfoPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(parsed.positions.len(), 20);
        assert_eq!(parsed.positions[19].name, "Position 19");
        assert_eq!(reader.get_offset(), 564);
    }

    #[test]
    fn notice_packet_is_182_bytes() {
        let packet = GuildNoticePacket {
            subject: "Welcome".to_owned(),
            notice: "Be nice to each other.".to_owned(),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 182);

        let mut reader = ByteReader::without_metadata(writer.as_slice());
        let parsed = GuildNoticePacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(parsed.subject, "Welcome");
        assert_eq!(parsed.notice, "Be nice to each other.");
    }

    #[test]
    fn guild_request_headers_and_sizes_match_20190605() {
        let mut writer = ByteWriter::new();
        GuildMenuInterfaceRequestPacket {}.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), &[0x4D, 0x01]);

        let mut writer = ByteWriter::new();
        GuildMenuRequestPacket { menu: 1 }.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), &[0x4F, 0x01, 0x01, 0x00, 0x00, 0x00]);

        let mut writer = ByteWriter::new();
        GuildMemberNameRequestPacket {
            character_id: CharacterId(0x1234_5678),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        assert_eq!(writer.as_slice(), &[0x4C, 0x09, 0x78, 0x56, 0x34, 0x12]);

        let mut writer = ByteWriter::new();
        GuildNoticeChangeRequestPacket {
            guild_id: 7,
            subject: "s".to_owned(),
            notice: "n".to_owned(),
        }
        .packet_to_bytes(&mut writer)
        .unwrap();
        assert_eq!(writer.as_slice().len(), 186);
    }

    #[test]
    fn guild_info_parses_the_2019_wire_layout() {
        // Hand-built `ZC_GUILD_INFO` (0x0A84) fixture from the rAthena layout
        // comment in clif.cpp:8730 and PACKET_ZC_GUILD_INFO (packets_struct.hpp:4830).
        let mut bytes = vec![0u8; 94];
        bytes[0..2].copy_from_slice(&0x0A84u16.to_le_bytes());
        bytes[2..6].copy_from_slice(&7u32.to_le_bytes());
        bytes[6..10].copy_from_slice(&5u32.to_le_bytes());
        bytes[10..14].copy_from_slice(&3u32.to_le_bytes());
        bytes[14..18].copy_from_slice(&20u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&90u32.to_le_bytes());
        bytes[22..26].copy_from_slice(&1_000u32.to_le_bytes());
        bytes[26..30].copy_from_slice(&2_000u32.to_le_bytes());
        bytes[30..34].copy_from_slice(&10u32.to_le_bytes());
        bytes[34..38].copy_from_slice(&1u32.to_le_bytes());
        bytes[38..42].copy_from_slice(&2u32.to_le_bytes());
        bytes[42..46].copy_from_slice(&42u32.to_le_bytes());
        bytes[46..56].copy_from_slice(b"Domination");
        bytes[70..78].copy_from_slice(b"Prontera");
        bytes[86..90].copy_from_slice(&100u32.to_le_bytes());
        bytes[90..94].copy_from_slice(&150_009u32.to_le_bytes());

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = GuildInfoPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 94);
        assert_eq!(packet.guild_id, 7);
        assert_eq!(packet.member_count, 3);
        assert_eq!(packet.max_member_count, 20);
        assert_eq!(packet.points, 10);
        assert_eq!(packet.emblem_id, 42);
        assert_eq!(packet.guild_name, "Domination");
        assert_eq!(packet.manage_land, "Prontera");
        assert_eq!(packet.zeny, 100);
        assert_eq!(packet.master_character_id, CharacterId(150_009));
    }

    #[test]
    fn member_manager_parses_the_2019_wire_layout() {
        // Header + length + one 34-byte GUILD_MEMBER_INFO row.
        let mut bytes = vec![0u8; 38];
        bytes[0..2].copy_from_slice(&0x0AA5u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&38u16.to_le_bytes());
        bytes[4..8].copy_from_slice(&150_009u32.to_le_bytes());
        bytes[8..12].copy_from_slice(&150_010u32.to_le_bytes());
        bytes[12..14].copy_from_slice(&7u16.to_le_bytes());
        bytes[14..16].copy_from_slice(&3u16.to_le_bytes());
        bytes[16..18].copy_from_slice(&1u16.to_le_bytes());
        bytes[18..20].copy_from_slice(&14u16.to_le_bytes());
        bytes[20..22].copy_from_slice(&99u16.to_le_bytes());
        bytes[22..26].copy_from_slice(&12_345u32.to_le_bytes());
        bytes[26..30].copy_from_slice(&1i32.to_le_bytes());
        bytes[30..34].copy_from_slice(&2i32.to_le_bytes());
        bytes[34..38].copy_from_slice(&1_700_000_000u32.to_le_bytes());

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = GuildMemberManagerInfoPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 38);
        assert_eq!(packet.members.len(), 1);
        let member = &packet.members[0];
        assert_eq!(member.account_id, AccountId(150_009));
        assert_eq!(member.character_id, CharacterId(150_010));
        assert_eq!(member.hair, 7);
        assert_eq!(member.hair_color, 3);
        assert_eq!(member.sex, 1);
        assert_eq!(member.job_id, JobId(14));
        assert_eq!(member.level, 99);
        assert_eq!(member.contribution_exp, 12_345);
        assert_eq!(member.online, 1);
        assert_eq!(member.position_id, 2);
        assert_eq!(member.last_login, 1_700_000_000);
    }

    #[test]
    fn member_name_packet_parses_the_2019_wire_layout() {
        // Header + flag + char id + name[24].
        let mut bytes = vec![0u8; 32];
        bytes[0..2].copy_from_slice(&0x0AF7u16.to_le_bytes());
        bytes[2..4].copy_from_slice(&3u16.to_le_bytes());
        bytes[4..8].copy_from_slice(&150_009u32.to_le_bytes());
        bytes[8..11].copy_from_slice(b"Alt");

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = GuildMemberNamePacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 32);
        assert_eq!(packet.flag, 3);
        assert_eq!(packet.character_id, CharacterId(150_009));
        assert_eq!(packet.name, "Alt");
    }

    #[test]
    fn guild_notice_parses_the_2019_wire_layout() {
        // Header + subject[60] + notice[120] = 182 bytes.
        let mut bytes = vec![0u8; 182];
        bytes[0..2].copy_from_slice(&0x016Fu16.to_le_bytes());
        bytes[2..9].copy_from_slice(b"Welcome");
        bytes[62..76].copy_from_slice(b"Be nice to all");

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = GuildNoticePacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 182);
        assert_eq!(packet.subject, "Welcome");
        assert_eq!(packet.notice, "Be nice to all");
    }

    #[test]
    fn leave_and_expel_packets_are_54_bytes() {
        let leave = GuildLeaveRequestPacket {
            guild_id: 7,
            account_id: AccountId(150_009),
            character_id: CharacterId(150_009),
            message: String::new(),
        };
        let expel = GuildExpelRequestPacket {
            guild_id: 7,
            account_id: AccountId(150_010),
            character_id: CharacterId(150_010),
            message: "bye".to_owned(),
        };

        let mut writer = ByteWriter::new();
        leave.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 54);
        assert_eq!(&writer.as_slice()[0..2], &[0x59, 0x01]);

        let mut writer = ByteWriter::new();
        expel.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 54);
        assert_eq!(&writer.as_slice()[0..2], &[0x5B, 0x01]);
    }
}

/// `ZC_ACK_LEAVE_GUILD` (0x0A83 at PACKETVER >= 20161019), 46 bytes.
///
/// `clif_guild_leave` (clif.cpp:9124) notifies every guild member that a
/// character left the guild. The 2016+ layout replaces the 24-byte name with
/// the character id (`PACKET_ZC_ACK_LEAVE_GUILD2`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A83)]
#[packet_size(46)]
pub struct GuildLeaveNoticePacket {
    pub character_id: CharacterId,
    #[length(40)]
    pub reason: String,
}

/// `ZC_ACK_BAN_GUILD` (0x0A82 at PACKETVER >= 20161019), 46 bytes.
///
/// `clif_guild_expulsion` (clif.cpp:9143); unlike the leave packet the reason
/// comes first (`PACKET_ZC_ACK_BAN_GUILD3`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A82)]
#[packet_size(46)]
pub struct GuildBanNoticePacket {
    #[length(40)]
    pub reason: String,
    pub character_id: CharacterId,
}

/// `ZC_ACK_REQ_JOIN_GUILD` (0x0169), 3 bytes.
///
/// `clif_guild_inviteack` (clif.cpp:9111) only sends this on failure:
/// `0` = target cannot be invited (already in a guild / trading),
/// `1` = target offline, `3` = guild full. There is no success ack; the
/// invitation itself is `ZC_REQ_JOIN_GUILD` (0x016A) to the target.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0169)]
#[packet_size(3)]
pub struct GuildInviteResultPacket {
    pub result: u8,
}

// --- Bank (rAthena `clif_parse_Bank*`, PACKETVER >= 20130717) --------------
//
// All layouts come from `src/map/packets.hpp` (structs `PACKET_ZC_*BANKING*`)
// and the parsers in `src/map/clif.cpp:7327-7535`.

/// `ZC_UI_OPEN` (0x0AE2): the server asks the client to open one of its
/// built-in UI windows.
///
/// At PACKETVER 20190605 the packet is 7 bytes: `type.W type.B data.L`
/// (`PACKET_ZC_UI_OPEN`, `src/map/packets_struct.hpp:2241`, `clif_ui_open` at
/// `src/map/clif.cpp:21916`). `ui_type` is `enum out_ui_type`
/// (`src/map/clif.hpp:1415`); `0` is `OUT_UI_BANK`, which the `openbank` NPC
/// script command sends (`src/map/script.cpp:27070`). Opening the bank still
/// needs the `CZ_REQ_OPEN_BANKING` request below.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AE2)]
#[packet_size(7)]
pub struct OpenUserInterfacePacket {
    /// `enum out_ui_type`: 0 bank, 1 stylist, 5 tip, 6 quest, ...
    pub ui_type: u8,
    pub data: u32,
}

/// `ZC_ACK_OPEN_BANKING` (0x09B7), 4 bytes: the server accepted the banking
/// session (`clif_bank_open`, `src/map/clif.cpp:7327`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09B7)]
#[packet_size(4)]
pub struct BankOpenAckPacket {
    /// Always 0 in rAthena.
    pub unknown: u16,
}

/// `ZC_ACK_CLOSE_BANKING` (0x09B9), 4 bytes (`clif_bank_close`,
/// `src/map/clif.cpp:7369`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09B9)]
#[packet_size(4)]
pub struct BankCloseAckPacket {
    /// Always 0 in rAthena.
    pub unknown: u16,
}

/// `ZC_BANKING_CHECK` (0x09A6), 12 bytes: the current bank vault and a reason
/// code (`clif_Bank_Check`, `src/map/clif.cpp:7405`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A6)]
#[packet_size(12)]
pub struct BankBalancePacket {
    /// Amount in the account-wide bank vault (`sd.bank_vault`).
    pub money: u64,
    /// Reason code; rAthena always sends 0 here.
    pub reason: u16,
}

/// `ZC_ACK_BANKING_DEPOSIT` (0x09A8), 16 bytes (`clif_bank_deposit`,
/// `src/map/clif.cpp:7446`). `reason` uses `enum e_BANKING_DEPOSIT_ACK`
/// (`src/map/clif.hpp:93`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A8)]
#[packet_size(16)]
pub struct BankDepositAckPacket {
    pub reason: u16,
    /// Bank vault after the transaction.
    pub money: u64,
    /// Carried zeny after the transaction.
    pub zeny: i32,
}

/// `ZC_ACK_BANKING_WITHDRAW` (0x09AA), 16 bytes (`clif_bank_withdraw`,
/// `src/map/clif.cpp:7490`). `reason` uses `enum e_BANKING_WITHDRAW_ACK`
/// (`src/map/clif.hpp:100`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09AA)]
#[packet_size(16)]
pub struct BankWithdrawAckPacket {
    pub reason: u16,
    /// Bank vault after the transaction.
    pub money: u64,
    /// Carried zeny after the transaction.
    pub zeny: i32,
}

/// `CZ_REQ_OPEN_BANKING` (0x09B6), 6 bytes: open the banking session. rAthena
/// only answers when `AID` equals the sender's account id
/// (`clif_parse_BankOpen`, `src/map/clif.cpp:7342`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09B6)]
#[packet_size(6)]
pub struct BankOpenRequestPacket {
    pub account_id: AccountId,
}

/// `CZ_REQ_CLOSE_BANKING` (0x09B8), 6 bytes
/// (`clif_parse_BankClose`, `src/map/clif.cpp:7383`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09B8)]
#[packet_size(6)]
pub struct BankCloseRequestPacket {
    pub account_id: AccountId,
}

/// `CZ_REQ_BANKING_CHECK` (0x09AB), 6 bytes: request `ZC_BANKING_CHECK`
/// (`clif_parse_BankCheck`, `src/map/clif.cpp:7419`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09AB)]
#[packet_size(6)]
pub struct BankCheckRequestPacket {
    pub account_id: AccountId,
}

/// `CZ_REQ_BANKING_DEPOSIT` (0x09A7), 10 bytes (`clif_parse_BankDeposit`,
/// `src/map/clif.cpp:7462`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A7)]
#[packet_size(10)]
pub struct BankDepositRequestPacket {
    pub account_id: AccountId,
    /// Amount to move into the bank vault.
    pub zeny: u32,
}

/// `CZ_REQ_BANKING_WITHDRAW` (0x09A9), 10 bytes (`clif_parse_BankWithdraw`,
/// `src/map/clif.cpp:7505`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x09A9)]
#[packet_size(10)]
pub struct BankWithdrawRequestPacket {
    pub account_id: AccountId,
    /// Amount to move out of the bank vault.
    pub zeny: u32,
}

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
mod mail_packet_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter, ToBytes};

    use super::*;
    use crate::{ItemOptions, PacketExt};

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// One `maillistinfo` row exactly as clif_Mail_refreshinbox writes it at
    /// PACKETVER 20190605.
    fn push_mail_list_row(bytes: &mut Vec<u8>, open_type: u8, mail_id: u64, read: u8, mail_type: u8, sender: &str, title: &str) {
        bytes.push(open_type);
        push_u64(bytes, mail_id);
        bytes.push(read);
        bytes.push(mail_type);

        let mut sender_bytes = [0u8; 24];
        sender_bytes[..sender.len()].copy_from_slice(sender.as_bytes());
        bytes.extend_from_slice(&sender_bytes);

        push_u32(bytes, 3_600);
        push_u16(bytes, (title.len() + 1) as u16);
        bytes.extend_from_slice(title.as_bytes());
        bytes.push(0);
    }

    #[test]
    fn mailbox_list_packet_2019_parses_every_row() {
        let mut payload = Vec::new();
        payload.push(1);
        push_mail_list_row(&mut payload, 0, 7, 0, 0x2 | 0x4, "Alice", "Hello");
        push_mail_list_row(&mut payload, 2, 9, 1, 0x0, "Bob", "Returned mail");

        let mut bytes = Vec::new();
        push_u16(&mut bytes, 0x0AC2);
        push_u16(&mut bytes, (4 + payload.len()) as u16);
        bytes.extend_from_slice(&payload);

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = MailboxListPacket2019::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), bytes.len());
        assert_eq!(packet.unknown, 1);
        assert_eq!(packet.entries.mails.len(), 2);

        let first = &packet.entries.mails[0];
        assert_eq!(first.open_type, 0);
        assert_eq!(first.mail_id, 7);
        assert_eq!(first.read, 0);
        assert_eq!(first.mail_type, 0x2 | 0x4);
        assert_eq!(first.sender, "Alice");
        assert_eq!(first.expires_in_seconds, 3_600);
        assert_eq!(first.title, "Hello");

        let second = &packet.entries.mails[1];
        assert_eq!(second.open_type, 2);
        assert_eq!(second.mail_id, 9);
        assert_eq!(second.read, 1);
        assert_eq!(second.mail_type, 0);
        assert_eq!(second.sender, "Bob");
        assert_eq!(second.title, "Returned mail");
    }

    #[test]
    fn mailbox_list_packet_2019_handles_an_empty_inbox() {
        let mut bytes = Vec::new();
        push_u16(&mut bytes, 0x0AC2);
        push_u16(&mut bytes, 5);
        bytes.push(1);

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = MailboxListPacket2019::packet_from_bytes(&mut reader).unwrap();

        assert!(packet.entries.mails.is_empty());
        assert_eq!(reader.get_offset(), 5);
    }

    #[test]
    fn mail_read_packet_2019_matches_the_server_layout() {
        let body = "Come to Prontera!\0";
        let mut payload = Vec::new();
        payload.push(0);
        push_u64(&mut payload, 0x1234_5678_9ABC_DEF0);
        push_u16(&mut payload, body.len() as u16);
        push_u64(&mut payload, 1_500);
        payload.push(1);
        payload.extend_from_slice(body.as_bytes());

        // One 59 byte attachment: 3 Poring cards, +7 refine, one option.
        push_u16(&mut payload, 30);
        push_u32(&mut payload, 909);
        payload.push(1);
        payload.push(0);
        payload.push(7);
        for card in [4001u32, 4002, 4003, 0] {
            push_u32(&mut payload, card);
        }
        push_u32(&mut payload, 0x0100);
        payload.push(4);
        push_u16(&mut payload, 44);
        push_u16(&mut payload, 0);
        for (index, value) in [(6u16, 10u16), (7, 20), (0, 0), (0, 0), (0, 0)] {
            push_u16(&mut payload, index);
            push_u16(&mut payload, value);
            payload.push(0);
        }

        let mut bytes = Vec::new();
        push_u16(&mut bytes, 0x09EB);
        push_u16(&mut bytes, (4 + payload.len()) as u16);
        bytes.extend_from_slice(&payload);

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = MailReadPacket2019::packet_from_bytes(&mut reader).unwrap();

        assert_eq!(reader.get_offset(), bytes.len());
        assert_eq!(packet.open_type, 0);
        assert_eq!(packet.mail_id, 0x1234_5678_9ABC_DEF0);
        assert_eq!(packet.text_length, body.len() as u16);
        assert_eq!(packet.zeny, 1_500);
        assert_eq!(packet.item_count, 1);
        assert_eq!(packet.body, "Come to Prontera!");
        assert_eq!(packet.item_count, 1);
        assert_eq!(packet.items.items.len(), 1);

        let item = &packet.items.items[0];
        assert_eq!(item.amount, 30);
        assert_eq!(item.item_id, 909);
        assert_eq!(item.identified, 1);
        assert_eq!(item.damaged, 0);
        assert_eq!(item.refine, 7);
        assert_eq!(item.card, [4001, 4002, 4003, 0]);
        assert_eq!(item.location, 0x0100);
        assert_eq!(item.item_type, 4);
        assert_eq!(item.view_sprite, 44);
        assert_eq!(item.bind_on_equip, 0);
        assert_eq!(item.options[0].index, 6);
        assert_eq!(item.options[0].value, 10);
        assert_eq!(item.options[0].parameter, 0);
    }

    #[test]
    fn mail_attachment_item_is_59_bytes_at_20190605() {
        let item = MailAttachmentItem2019 {
            amount: 0,
            item_id: 0,
            identified: 0,
            damaged: 0,
            refine: 0,
            card: [0; 4],
            location: 0,
            item_type: 0,
            view_sprite: 0,
            bind_on_equip: 0,
            options: std::array::from_fn(|_| ItemOptions {
                index: 0,
                value: 0,
                parameter: 0,
            }),
        };

        let mut writer = ByteWriter::new();
        item.to_bytes(&mut writer).unwrap();

        assert_eq!(writer.as_slice().len(), 59);
    }

    #[test]
    fn mail_client_requests_match_the_2019_wire_layout() {
        // Every header/field offset below is what the rAthena parsers read:
        // clif_parse_Mail_refreshinbox reads `<mail id>.Q` at byte 2 of 0x0AC0
        // (clif.cpp:16221), clif_parse_Mail_read `<tab>.B` at 2 and
        // `<mail id>.Q` at 3 (clif.cpp:16403), clif_parse_Mail_getattach
        // `<mail id>.Q` at 2 and `<tab>.B` at 10 (clif.cpp:16501) and
        // clif_parse_Mail_delete `<tab>.B` at 2 and `<mail id>.Q` at 3
        // (clif.cpp:16607).

        let open = MailboxOpenRequestPacket {
            mail_id: 0,
            unknown: [0; 16],
        }
        .packet_to_bytes_vec();
        assert_eq!(open.len(), 26);
        assert_eq!(&open[..2], &[0xC0, 0x0A]);
        assert_eq!(&open[2..10], &0u64.to_le_bytes());

        let refresh = MailboxRefreshRequestPacket {
            mail_id: 0,
            unknown: [0; 16],
        }
        .packet_to_bytes_vec();
        assert_eq!(refresh.len(), 26);
        assert_eq!(&refresh[..2], &[0xC1, 0x0A]);

        let read = MailReadRequestPacket {
            open_type: 2,
            mail_id: 0x0102_0304_0506_0708,
        }
        .packet_to_bytes_vec();
        assert_eq!(read.len(), 11);
        assert_eq!(&read[..2], &[0xEA, 0x09]);
        assert_eq!(read[2], 2);
        assert_eq!(&read[3..11], &0x0102_0304_0506_0708u64.to_le_bytes());

        let claim_zeny = MailClaimZenyRequestPacket {
            mail_id: 0x0102_0304_0506_0708,
            open_type: 1,
        }
        .packet_to_bytes_vec();
        assert_eq!(claim_zeny.len(), 11);
        assert_eq!(&claim_zeny[..2], &[0xF1, 0x09]);
        assert_eq!(&claim_zeny[2..10], &0x0102_0304_0506_0708u64.to_le_bytes());
        assert_eq!(claim_zeny[10], 1);

        let claim_items = MailClaimItemRequestPacket {
            mail_id: 0x0102_0304_0506_0708,
            open_type: 1,
        }
        .packet_to_bytes_vec();
        assert_eq!(claim_items.len(), 11);
        assert_eq!(&claim_items[..2], &[0xF3, 0x09]);
        assert_eq!(&claim_items[2..10], &0x0102_0304_0506_0708u64.to_le_bytes());
        assert_eq!(claim_items[10], 1);

        let delete = MailDeleteRequestPacket {
            open_type: 2,
            mail_id: 0x0102_0304_0506_0708,
        }
        .packet_to_bytes_vec();
        assert_eq!(delete.len(), 11);
        assert_eq!(&delete[..2], &[0xF5, 0x09]);
        assert_eq!(delete[2], 2);
        assert_eq!(&delete[3..11], &0x0102_0304_0506_0708u64.to_le_bytes());
    }

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
}

// ---------------------------------------------------------------------------
// Item service packets: weapon refine, item repair, identify and card
// composition (PACKETVER 20190605).
//
// Every opcode and layout below was verified against the CondaRO rAthena tree
// (`/home/phantom/popeng-llm-cpp/condaro-llm`, PACKETVER 20190605); the cited
// `clif.cpp` / `packets_struct.hpp` / `clif_packetdb.hpp` lines are relative to
// that tree. At PACKETVER >= 20181121 every item id and every `EQUIPSLOTINFO`
// card is 32 bit, which is why the entries below carry `ItemId` (u32) and
// `[u32; 4]` instead of 16 bit values.
// ---------------------------------------------------------------------------

/// One row of `ZC_NOTIFY_WEAPONITEMLIST` (0x0221).
///
/// rAthena's `PACKET_ZC_NOTIFY_WEAPONITEMLIST_sub` (`packets_struct.hpp:3332`,
/// only the `int16 index, uint32 itemId, uint8 refine, EQUIPSLOTINFO slot`
/// fields are used). `clif_item_refine_list` writes the *client* index
/// (`client_index(i)` = server slot + 2, `clif.cpp:7233`), so the index is an
/// [`InventoryIndex`].
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
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

/// Opens the refine UI (`ZC_OPEN_REFINING_UI`, 0x0AA0).
///
/// Layout (`clif_refineui_open`, `clif.cpp:22475`): `{ header }.W` only.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AA0)]
#[packet_size(2)]
pub struct OpenRefiningUiPacket {}

/// One selectable material of the refine UI (`ZC_REFINING_MATERIAL_LIST` row).
///
/// rAthena's `PACKET_ZC_REFINING_MATERIAL_LIST_SUB`
/// (`packets_struct.hpp:4165`): `{ item id }.L { chance }.b { zeny }.L` = 9
/// bytes at PACKETVER 20190605.
#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct RefiningMaterialEntry {
    pub item_id: ItemId,
    /// Success chance in percent (`cost->chance / 100`, 0-100).
    pub chance: i8,
    /// Zeny cost of the refine attempt.
    pub zeny: i32,
}

/// Materials and success chances for one selected item
/// (`ZC_REFINING_MATERIAL_LIST`, 0x0AA2).
///
/// Layout (`clif_refineui_info`, `clif.cpp:22504`; struct
/// `packets_struct.hpp:4175`): `{ header }.W { length }.W { item index }.W
/// { blacksmith blessing }.b` followed by one [`RefiningMaterialEntry`] per
/// material. The item index is the `slot + 2` client index
/// (`p->itemIndex = client_index( index )`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AA2)]
#[variable_length]
pub struct RefiningMaterialListPacket {
    pub item_index: InventoryIndex,
    /// Number of Blacksmith Blessings required for the selected refine level,
    /// `0` when the item cannot use them.
    pub blacksmith_blessing: i8,
    #[repeating_remaining]
    pub materials: Vec<RefiningMaterialEntry>,
}

/// Select the item to refine in the refine UI (`CZ_REFINING_SELECT_ITEM`,
/// 0x0AA1).
///
/// Layout (`clif_parse_refineui_add`, `clif.cpp:22574`): `{ header }.W
/// { client index }.W`; the server converts with `server_index()`. The server
/// answers with [`RefiningMaterialListPacket`].
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AA1)]
#[packet_size(4)]
pub struct RefiningSelectItemRequestPacket {
    pub index: InventoryIndex,
}

/// Attempt a refine in the refine UI (`CZ_REQ_REFINING`, 0x0AA3).
///
/// Layout (`clif_parse_refineui_refine`, `clif.cpp:22599`; struct
/// `packets_struct.hpp:4186`): `{ header }.W { client index }.W
/// { material item id }.L { blacksmith blessing }.b` = 9 bytes. The material
/// id is a plain item id (`p->itemId` is compared with `cost->nameid`), the
/// blessing byte is `0`/`1`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AA3)]
#[packet_size(9)]
pub struct RefiningRequestPacket {
    pub index: InventoryIndex,
    pub material_item_id: ItemId,
    pub blacksmith_blessing: i8,
}

/// Close the refine UI (`CZ_CLOSE_REFINING_UI`, 0x0AA4).
///
/// Layout (`clif_parse_refineui_close`, `clif.cpp:22493`): `{ header }.W`.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0AA4)]
#[packet_size(2)]
pub struct CloseRefiningUiRequestPacket {}

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

/// `ZC_READ_BOOK` (0x0294), 10 bytes: `<book id>.L <page>.L`.
///
/// rAthena writes the packet by hand in `clif_readbook`
/// (clif.cpp:18354-18361) because it has no `PACKET_*` struct. The text of the
/// book is **not** part of the packet: the client reads it from its own
/// `data\book\<book id>.txt` file, so only the id and the page are on the wire.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0294)]
#[packet_size(10)]
pub struct ReadBookPacket {
    pub book_id: u32,
    pub page: u32,
}

/// `ZC_ACK_CREATE_CHATROOM` (0x00D6), 3 bytes.
///
/// `clif_createchat` (clif.cpp:4475-4485). `flag`:
/// `0` = created, `1` = limit exceeded, `2` = same room already exists.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00D6)]
#[packet_size(3)]
pub struct CreateChatRoomResultPacket {
    pub flag: u8,
}

/// `ZC_ROOM_NEWENTRY` (0x00D7), variable length.
///
/// `clif_dispchat` (clif.cpp:4513-4532) announces a chat room to the
/// surrounding players, including its owner, id, limit, current user count,
/// status and title.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00D7)]
#[variable_length]
pub struct ChatRoomNewEntryPacket {
    pub owner_id: AccountId,
    pub chat_id: u32,
    pub limit: u16,
    pub users: u16,
    pub room_type: u8,
    #[length_remaining]
    pub title: String,
}

/// `ZC_DESTROY_ROOM` (0x00D8), 6 bytes: the chat room closed.
///
/// `clif_clearchat` (clif.cpp:4560-4570).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00D8)]
#[packet_size(6)]
pub struct DestroyChatRoomPacket {
    pub chat_id: u32,
}

/// `ZC_REFUSE_ENTER_ROOM` (0x00DA), 3 bytes.
///
/// `clif_joinchatfail` (clif.cpp:4579-4590). `result`:
/// `0` = full, `1` = wrong password, `2` = kicked, `3` = success,
/// `4` = not enough zeny, `5` = level too low, `6` = level too high,
/// `7` = unsuitable job class.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00DA)]
#[packet_size(3)]
pub struct RefuseEnterChatRoomPacket {
    pub result: u8,
}

/// One member row of [`EnterChatRoomPacket`] (24 byte name + 4 byte role).
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct ChatRoomMember {
    /// `0` = owner (gets the room menu), `1` = normal member.
    pub role: u32,
    #[length(24)]
    pub name: String,
}

/// `ZC_ENTER_ROOM` (0x00DB), variable length.
///
/// `clif_joinchatok` (clif.cpp:4593-4631). The title is not part of this
/// packet; the client keeps the title from the `ZC_ROOM_NEWENTRY` it joined.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00DB)]
#[variable_length]
pub struct EnterChatRoomPacket {
    pub chat_id: u32,
    #[repeating_remaining]
    pub members: Vec<ChatRoomMember>,
}

/// `ZC_MEMBER_NEWENTRY` (0x00DC), 28 bytes: a member joined.
///
/// `clif_addchat` (clif.cpp:4634-4645).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00DC)]
#[packet_size(28)]
pub struct ChatRoomMemberNewEntryPacket {
    pub count: u16,
    #[length(24)]
    pub name: String,
}

/// `ZC_MEMBER_EXIT` (0x00DD), 29 bytes: a member left or was kicked.
///
/// `clif_chat_leave` (clif.cpp:4663-4674).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00DD)]
#[packet_size(29)]
pub struct ChatRoomMemberExitPacket {
    pub count: u16,
    #[length(24)]
    pub name: String,
    pub kicked: u8,
}

/// `ZC_CHANGE_CHATROOM` (0x00DF), variable length: room title/limit/status
/// changed. `clif_changechatstatus` (clif.cpp:4534-4557).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00DF)]
#[variable_length]
pub struct ChangeChatRoomPacket {
    pub owner_id: AccountId,
    pub chat_id: u32,
    pub limit: u16,
    pub users: u16,
    pub flag: u8,
    #[length_remaining]
    pub title: String,
}

/// `ZC_ROLE_CHANGE` (0x00E1), 30 bytes: the room owner changed.
///
/// `clif_chat_role` (clif.cpp:4648-4660). `flag` is `0` for the new owner and
/// `1` for a normal member.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E1)]
#[packet_size(30)]
pub struct ChatRoomRoleChangePacket {
    pub flag: u32,
    #[length(24)]
    pub name: String,
}

/// `CZ_CREATE_CHATROOM` (0x00D5), variable length.
///
/// `clif_parse_CreateChatRoom` (clif.cpp:12320-12363). The password is a fixed
/// 8 byte C string, the title fills the rest of the packet.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00D5)]
#[variable_length]
pub struct CreateChatRoomPacket {
    pub limit: u16,
    pub room_type: u8,
    #[length(8)]
    pub password: String,
    #[length_remaining]
    pub title: String,
}

/// `CZ_REQ_ENTER_ROOM` (0x00D9), 14 bytes: join a chat room.
///
/// `clif_parse_ChatAddMember` (clif.cpp:12367-12376).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00D9)]
#[packet_size(14)]
pub struct EnterChatRoomRequestPacket {
    pub chat_id: u32,
    #[length(8)]
    pub password: String,
}

/// `CZ_CHANGE_CHATROOM` (0x00DE), variable length: change title/limit/status.
///
/// `clif_parse_ChatRoomStatusChange` (clif.cpp:12380-12405).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00DE)]
#[variable_length]
pub struct ChangeChatRoomRequestPacket {
    pub limit: u16,
    pub room_type: u8,
    #[length(8)]
    pub password: String,
    #[length_remaining]
    pub title: String,
}

/// `CZ_EXIT_ROOM` (0x00E3), 2 bytes: leave the current chat room.
///
/// `clif_parse_ChatLeave` (clif.cpp:12426-12431) has no payload.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00E3)]
#[packet_size(2)]
pub struct ExitChatRoomPacket {}

/// `CZ_PINCODE_CHECK` (0x08B8), 10 bytes: answer to `HC_ACK_PINCODE` when the
/// account already has a PIN.
///
/// `chclif_parse_pincode_check` (char_clif.cpp:189-213) reads the account id at
/// offset 2 and four PIN bytes at offset 6. The bytes are obfuscated with the
/// seed of the `HC_ACK_PINCODE` packet, see
/// `korangar::pincode::encrypt_pincode`.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08B8)]
#[packet_size(10)]
pub struct PincodeCheckPacket {
    pub account_id: AccountId,
    pub pin: [u8; 4],
}

/// `CZ_PINCODE_SET_NEW` (0x08BA), 10 bytes: set the first PIN of the account.
///
/// `chclif_parse_pincode_setnew` (char_clif.cpp:330-363).
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08BA)]
#[packet_size(10)]
pub struct PincodeSetNewPacket {
    pub account_id: AccountId,
    pub pin: [u8; 4],
}

#[cfg(test)]
mod content_packet_tests {
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
