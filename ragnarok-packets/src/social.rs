//! social packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

/// Sent by the client to request the guild emblem image of a guild.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0152)]
#[variable_length]
pub struct GuildEmblemImagePacket {
    pub guild_id: u32,
    pub emblem_id: u32,
    #[length_remaining]
    pub emblem_data: Vec<u8>,
}

/// Whisper a message to another character (CZ_WHISPER).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09DF)]
#[packet_size(7)]
pub struct WhisperAckPacket {
    pub result: WhisperResult,
    pub character_id: u32,
}

/// A whispered message that another character sent to the player (ZC_WHISPER).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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

/// 2019 era `ZC_NOTIFY_HP_TO_GROUPM` (0x080E): party member health update.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x080E)]
#[packet_size(14)]
pub struct PartyMemberHealthPacket2019 {
    pub account_id: AccountId,
    pub health_points: i32,
    pub maximum_health_points: i32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02C9)]
#[packet_size(3)]
pub struct UpdatePartyInvitationStatePacket {
    pub allowed: u8, // always 0 on rAthena
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x086F)]
pub struct AddFriendPacket {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0203)]
pub struct RemoveFriendPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x020A)]
#[packet_size(10)]
pub struct NotifyFriendRemovedPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0201)]
#[variable_length]
pub struct FriendListPacket2019 {
    #[repeating_remaining]
    pub friend_list: Vec<Friend2019>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0207)]
#[packet_size(34)]
pub struct FriendRequestPacket {
    pub requestee: Friend,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum FriendRequestResponse {
    Reject,
    Accept,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0208)]
pub struct FriendRequestResponsePacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub response: FriendRequestResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum FriendRequestResult {
    Accepted,
    Rejected,
    OwnFriendListFull,
    OtherFriendListFull,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0209)]
#[packet_size(36)]
pub struct FriendRequestResultPacket {
    pub result: FriendRequestResult,
    pub friend: Friend,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02C6)]
#[packet_size(30)]
pub struct PartyInvitePacket {
    pub party_id: PartyId,
    #[length(24)]
    pub party_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0988)]
#[packet_size(6)]
pub struct ClanOnlineCountPacket {
    pub online_members: u16,
    pub maximum_members: u16,
}

/// Guild chat message sent by the client (`CZ_GUILD_CHAT`, 0x017E).
///
/// rAthena validates the message with `clif_process_message`, exactly like the
/// public and party chat, so the payload has to start with
/// `<character name> : ` and the trailing `NUL` is not counted in the declared
/// packet length.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x017F)]
#[variable_length]
pub struct GuildChatReceivedPacket {
    #[length_remaining]
    pub message: String,
}

/// Result of creating a party (`ZC_ACK_MAKE_GROUP`, 0x00FA).
///
/// * `0` = party created
/// * `1` = party name already exists
/// * `2` = already in a party
/// * `3` = cannot create a party on this map
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0107)]
#[packet_size(10)]
pub struct PartyMemberPositionPacket2019 {
    pub account_id: AccountId,
    pub position: TilePosition,
}

/// `ZC_NOTIFY_CHAT_PARTY` (0x0109): party chat message.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0109)]
#[variable_length]
pub struct PartyChatMessagePacket2019 {
    pub account_id: AccountId,
    #[length_remaining]
    pub message: String,
}

/// `ZC_NOTIFY_MEMBERINFO_TO_GROUPM` (0x0ABD): party member job and base level.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0ABD)]
#[packet_size(10)]
pub struct PartyMemberJobLevelPacket2019 {
    pub account_id: AccountId,
    pub job_id: JobId,
    pub base_level: u16,
}

/// `ZC_GROUP_ISALIVE` (0x0AB2): party member death state.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00F9)]
pub struct CreatePartyPacket {
    #[length(24)]
    pub party_name: String,
}

/// `CZ_PARTY_JOIN_REQ` (0x02C4): invite a character by name.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02C4)]
pub struct PartyInviteRequestPacket2019 {
    #[length(24)]
    pub character_name: String,
}

/// `CZ_PARTY_JOIN_REQ_ACK` (0x02C7): accept (1) or reject (0) a party invite.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02C7)]
pub struct PartyInviteReplyRequestPacket2019 {
    pub party_id: PartyId,
    pub accept: u8,
}

/// `CZ_REQ_LEAVE_GROUP` (0x0100): leave the current party.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0100)]
pub struct LeavePartyPacket {}

/// `CZ_REQ_EXPEL_GROUP_MEMBER` (0x0103): expel a party member (leader only).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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

/// One member row of `ZC_MEMBERMGR_INFO` (0x0AA5) at PACKETVER 20190605.
///
/// rAthena layout (`src/map/packets_struct.hpp:4759`): account id, char id,
/// hair, hair color, sex, job, level, contribution EXP, online state,
/// position id and last login time. The 2016+ layout has **no name field**;
/// the client requests it separately by character id.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0166)]
#[variable_length]
pub struct GuildPositionNameInfoPacket {
    #[repeating_remaining]
    pub positions: Vec<GuildPositionNameEntry>,
}

/// One entry of `ZC_POSITION_INFO` (0x0160).
///
/// `mode` uses `enum e_guild_permission` (`src/common/mmo.hpp:885`):
/// `0x001` invite, `0x010` expel, `0x100` storage.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct GuildPositionInfoEntry {
    pub position_id: u32,
    pub mode: u32,
    pub ranking: u32,
    pub pay_rate: u32,
}

/// `ZC_POSITION_INFO` (0x0160): the permission/pay-rate row of every position
/// (`clif_guild_positioninfolist`, clif.cpp:8890).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0160)]
#[variable_length]
pub struct GuildPositionInfoPacket {
    #[repeating_remaining]
    pub positions: Vec<GuildPositionInfoEntry>,
}

/// One entry of `ZC_ACK_CHANGE_GUILD_POSITIONINFO` (0x0174), 40 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0174)]
#[variable_length]
pub struct GuildPositionChangedPacket {
    #[repeating_remaining]
    pub positions: Vec<GuildPositionChangedEntry>,
}

/// One entry of `ZC_ACK_REQ_CHANGE_MEMBERS` (0x0156), 12 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct GuildMemberPositionChangedEntry {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub position_id: i32,
}

/// `ZC_ACK_REQ_CHANGE_MEMBERS` (0x0156): a member was moved to another
/// position (`clif_guild_memberpositionchanged`, clif.cpp:8956).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0156)]
#[variable_length]
pub struct GuildMemberPositionChangedPacket {
    #[repeating_remaining]
    pub members: Vec<GuildMemberPositionChangedEntry>,
}

/// One ally/opponent row of `ZC_MYGUILD_BASIC_INFO` (0x014C), 32 bytes.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x014C)]
#[variable_length]
pub struct GuildAllianceInfoPacket {
    #[repeating_remaining]
    pub alliances: Vec<GuildAllianceInfo>,
}

/// `ZC_GUILD_NOTICE` (0x016F), 182 bytes: `subject[60]` + `notice[120]`
/// (`clif_guild_notice`, clif.cpp:9070).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x014D)]
pub struct GuildMenuInterfaceRequestPacket {}

/// `CZ_REQ_GUILD_MENU` (0x014F), 6 bytes: request one guild window tab.
///
/// `clif_parse_GuildRequestInfo` (clif.cpp:14253):
/// `0` basic info + alliance list, `1` member list + position names,
/// `2` position names + position rights, `3` skills, `4` expulsion list.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x094C)]
pub struct GuildMemberNameRequestPacket {
    pub character_id: CharacterId,
}

/// `CZ_REQ_MAKE_GUILD` (0x0165), 30 bytes: create a guild.
///
/// `clif_parse_MakeGuild` (clif.cpp) consumes one Emperium (item 714) on the
/// server (`guild_emperium_check`); the answer is
/// [`GuildCreateResultPacket`] (0x0167). The character id is written at offset
/// 2 and the 24-byte guild name at offset 6.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0165)]
#[packet_size(30)]
pub struct GuildCreateRequestPacket {
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
}

/// `ZC_RESULT_MAKE_GUILD` (0x0167), 3 bytes.
///
/// `clif_guild_created` (clif.cpp) reports the result of
/// [`GuildCreateRequestPacket`]: `0` = guild created, `1` = the player is
/// already in a guild, `2` = the guild name is already taken, `3` = the player
/// has no Emperium.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0167)]
#[packet_size(3)]
pub struct GuildCreateResultPacket {
    pub result: u8,
}

/// `CZ_REQ_JOIN_GUILD2` (0x0916): invite a character by name
/// (`clif_parse_GuildInvite2`, clif.cpp:14483).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x016E)]
#[packet_size(186)]
pub struct GuildNoticeChangeRequestPacket {
    pub guild_id: u32,
    #[length(60)]
    pub subject: String,
    #[length(120)]
    pub notice: String,
}

// ===========================================================================
// Party booking ("party search" / LFG) and party settings (PACKETVER 20190605)
//
// The 2019 party booking board is advertised, searched, updated and deleted
// with the 0x0802..0x080B opcode block:
//
// * 0x0802 `CZ_PARTY_BOOKING_REQ_REGISTER`  (clif.cpp:13948) — 18 bytes: level,
//   map index and 6 desired job ids.
// * 0x0803 `ZC_PARTY_BOOKING_ACK_REGISTER`   (clif.cpp:13970) — 4 bytes.
// * 0x0804 `CZ_PARTY_BOOKING_REQ_SEARCH`     (clif.cpp:13983) — 14 bytes.
// * 0x0805 `ZC_PARTY_BOOKING_ACK_SEARCH`     (clif.cpp:14000) — variable,
//   48-byte entries.
// * 0x0806 `CZ_PARTY_BOOKING_REQ_DELETE`     (clif.cpp:14024) — header only.
// * 0x0807 `ZC_PARTY_BOOKING_ACK_DELETE`     (clif.cpp:14040) — 4 bytes.
// * 0x0808 `CZ_PARTY_BOOKING_REQ_UPDATE`     (clif.cpp:14051) — 14 bytes.
// * 0x0809 `ZC_PARTY_BOOKING_NOTIFY_INSERT`  (clif.cpp:14068) — 50 bytes.
// * 0x080A `ZC_PARTY_BOOKING_NOTIFY_UPDATE`  (clif.cpp:14090) — 18 bytes.
// * 0x080B `ZC_PARTY_BOOKING_NOTIFY_DELETE`  (clif.cpp:14107) — 6 bytes.
//
// The direct "join request" flow added in 2019-12 (`CZ/ZC_PARTY_REQ_*_TO_JOIN`,
// clif.cpp:25349) is gated with `#if PACKETVER >= 20191204`, so the pinned
// CondaRO 20190605 server does not parse or send it. The packets are still
// modelled so a newer server can use the same client code.
// ===========================================================================

/// One advertisement row of `ZC_PARTY_BOOKING_ACK_SEARCH` (0x0805) and
/// `ZC_PARTY_BOOKING_NOTIFY_INSERT` (0x0809).
///
/// rAthena writes the same 48-byte row in both packets
/// (`clif_PartyBookingSearchAck`, clif.cpp:14000 and
/// `clif_PartyBookingInsertNotify`, clif.cpp:14068): index, character name,
/// expire time, level, map index and six job ids.
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct PartyBookingEntry2019 {
    pub index: u32,
    #[length(24)]
    pub character_name: String,
    /// Expire timestamp sent by the server (rAthena writes the start time
    /// here, see the `FIXME` in clif.cpp:14008).
    pub expire_time: u32,
    pub level: u16,
    pub map_id: u16,
    pub jobs: [u16; 6],
}

/// `ZC_PARTY_BOOKING_ACK_REGISTER` (0x0803): result of registering an
/// advertisement. `0` = success, `1` = failure, `2` = already registered
/// (clif.cpp:13970).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0803)]
#[packet_size(4)]
pub struct PartyBookingRegisterResultPacket2019 {
    pub result: u16,
}

/// `ZC_PARTY_BOOKING_ACK_SEARCH` (0x0805): search result page.
///
/// `more_results` is `1` when the server stopped at
/// `MAX_PARTY_BOOKING_RESULTS` and more rows are available (clif.cpp:14000).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0805)]
#[variable_length]
pub struct PartyBookingSearchResultPacket2019 {
    pub more_results: u8,
    #[repeating_remaining]
    pub entries: Vec<PartyBookingEntry2019>,
}

/// `ZC_PARTY_BOOKING_ACK_DELETE` (0x0807): result of deleting the own
/// advertisement. `0`/`1` = success, `2` = failure, `3` = nothing registered
/// (clif.cpp:14040).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0807)]
#[packet_size(4)]
pub struct PartyBookingDeleteResultPacket2019 {
    pub result: u16,
}

/// `ZC_PARTY_BOOKING_NOTIFY_INSERT` (0x0809): a new advertisement was added
/// to the board.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0809)]
#[packet_size(50)]
pub struct PartyBookingInsertNotifyPacket2019 {
    pub entry: PartyBookingEntry2019,
}

/// `ZC_PARTY_BOOKING_NOTIFY_UPDATE` (0x080A): the desired jobs of an
/// advertisement changed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x080A)]
#[packet_size(18)]
pub struct PartyBookingUpdateNotifyPacket2019 {
    pub index: u32,
    pub jobs: [u16; 6],
}

/// `ZC_PARTY_BOOKING_NOTIFY_DELETE` (0x080B): an advertisement was removed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x080B)]
#[packet_size(6)]
pub struct PartyBookingDeleteNotifyPacket2019 {
    pub index: u32,
}

/// `CZ_PARTY_BOOKING_REQ_REGISTER` (0x0802): advertise a party/character on
/// the booking board.
///
/// `level` is the party level shown in the search, `map_id` the map index
/// (`0` = any map) and `jobs` the six desired job ids (`u16::MAX` = any job).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0802)]
#[packet_size(18)]
pub struct PartyBookingRegisterRequestPacket2019 {
    pub level: u16,
    pub map_id: u16,
    pub jobs: [u16; 6],
}

/// `CZ_PARTY_BOOKING_REQ_SEARCH` (0x0804): search the booking board.
///
/// `job` is `-1` for any job, `last_index` pages through the results and
/// `result_count` is the maximum number of rows the client wants
/// (`MAX_PARTY_BOOKING_RESULTS` server side).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0804)]
#[packet_size(14)]
pub struct PartyBookingSearchRequestPacket2019 {
    pub level: u16,
    pub map_id: u16,
    pub job: i16,
    pub last_index: u32,
    pub result_count: u16,
}

/// `CZ_PARTY_BOOKING_REQ_DELETE` (0x0806): remove the own advertisement.
///
/// The 2019 packet db registers the request with length 2
/// (`parseable_packet(0x0806,2,...)`, clif_packetdb.hpp:1316), i.e. header
/// only.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0806)]
pub struct PartyBookingDeleteRequestPacket2019 {}

/// `CZ_PARTY_BOOKING_REQ_UPDATE` (0x0808): change the desired jobs of the own
/// advertisement.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0808)]
#[packet_size(14)]
pub struct PartyBookingUpdateRequestPacket2019 {
    pub jobs: [u16; 6],
}

/// `CZ_PARTY_REQ_MASTER_TO_JOIN` (0x0AE6): request to join the party of a
/// booking advertisement. Only parsed by servers with
/// `PACKETVER >= 20191204` (clif_packetdb.hpp:1981).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AE6)]
#[packet_size(10)]
pub struct PartyBookingJoinRequestPacket {
    pub character_id: CharacterId,
    pub account_id: AccountId,
}

/// `CZ_PARTY_REQ_ACK_MASTER_TO_JOIN` (0x0AF8): accept (`1`) or reject (`0`) a
/// booking join request, sent by the party leader.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AF8)]
#[packet_size(11)]
pub struct PartyBookingJoinReplyPacket {
    pub character_id: CharacterId,
    pub account_id: AccountId,
    pub accept: u8,
}

/// `CZ_GROUPINFO_CHANGE_V2` (0x07D7): change the party EXP share and item
/// rules. `clif_parse_PartyChangeOption` (clif.cpp:13893) reads the EXP flag
/// as `L`, the item pickup and item share flags as bytes.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x07D7)]
#[packet_size(8)]
pub struct PartyOptionRequestPacket2019 {
    /// `0` = each take, `1` = even share.
    pub experience_option: u32,
    /// `1` = "even share" item pickup rule.
    pub item_pickup: u8,
    /// `1` = item sharing enabled.
    pub item_share: u8,
}

/// `CZ_CHANGE_GROUP_MASTER` (0x07DA): transfer party leadership.
///
/// `clif_parse_PartyChangeLeader` (clif.cpp:13938) reads the account id at
/// offset 2.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x07DA)]
#[packet_size(6)]
pub struct ChangePartyLeaderPacket {
    pub account_id: AccountId,
}

// ===========================================================================
// Pet / homunculus / mercenary (PACKETVER 20190605)
//
// * 0x01A2 `ZC_PROPERTY_PET` (clif.cpp:8286) — 37 bytes at 20190605 (name,
//   renamed flag, level, hunger, intimacy, accessory, job).
// * 0x01A4 `ZC_CHANGESTATE_PET` (clif.cpp:8230) — 11 bytes, intimacy/hunger
//   update of one pet.
// * 0x07DB `ZC_HO_PAR_CHANGE` (clif.cpp:1754) — 8 bytes, homunculus HP/SP/EXP
//   change (type + i32).
// * 0x09F7 `ZC_PROPERTY_HOMUN` (clif.cpp:1811) — 77 bytes at 20190605; the full
//   homunculus status sheet.
// * 0x029B `ZC_MER_INIT` (clif.cpp:18234) — 80 bytes at 20190605
//   (`packet(0x029b,80)`, clif_packetdb.hpp:944).
// * 0x02A2 `ZC_MER_PAR_CHANGE` (clif.cpp:18166) — 8 bytes, mercenary
//   HP/SP/faith/kills update.
//
// The pet/homunculus packets have no actor id (they always describe the local
// player's unit), which is why the state matches them by job/name.
// `ZC_MER_INIT` starts with the mercenary block id.
// ===========================================================================

/// `ZC_PROPERTY_PET` (0x01A2): pet base data of the local player's pet.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01A2)]
#[packet_size(37)]
pub struct PetPropertyPacket2019 {
    #[length(24)]
    pub name: String,
    /// `1` when the pet was renamed (`pet.rename_flag`).
    pub renamed: u8,
    pub level: u16,
    /// Fullness/hunger value (`pet.hungry`).
    pub hunger: u16,
    /// Intimacy value (`pet.intimate`).
    pub intimacy: u16,
    /// Accessory item view id.
    pub accessory: u16,
    /// Monster class id of the pet.
    pub job_id: JobId,
}

/// `ZC_CHANGESTATE_PET` (0x01A4): one pet state value changed.
///
/// `state_type`: `0` = pre-init, `1` = intimacy, `2` = hunger, `3` =
/// accessory, `4` = performance, `5` = hairstyle (clif.cpp:8230).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01A4)]
#[packet_size(11)]
pub struct PetStateChangePacket2019 {
    pub state_type: u8,
    pub entity_id: EntityId,
    pub value: i32,
}

/// `ZC_PROPERTY_HOMUN` (0x09F7): full homunculus status sheet.
///
/// The layout is `PACKET_ZC_PROPERTY_HOMUN2` at PACKETVER 20190605
/// (`src/map/packets_struct.hpp:2823`, `packet(0x09F7,77)`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09F7)]
#[packet_size(77)]
pub struct HomunculusPropertyPacket2019 {
    #[length(24)]
    pub name: String,
    /// Bit 0 = renamed, bit 1 = vaporized, bit 2 = dead (clif.cpp:1820).
    pub flags: u8,
    pub level: u16,
    pub hunger: u16,
    pub intimacy: u16,
    /// Equipped item id (`0` unless a homunculus item is equipped).
    pub item_id: u32,
    pub attack: u16,
    pub magic_attack: u16,
    pub hit: u16,
    pub critical: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub flee: u16,
    pub attack_motion: u16,
    pub health_points: u32,
    pub maximum_health_points: u32,
    pub spell_points: u16,
    pub maximum_spell_points: u16,
    pub experience: u32,
    pub maximum_experience: u32,
    pub skill_points: u16,
    pub attack_range: u16,
}

/// `ZC_HO_PAR_CHANGE` (0x07DB): homunculus parameter change.
///
/// `parameter` is the rAthena `_sp` id: `1` = base EXP, `5` = HP, `7` = SP
/// (`src/map/map.hpp:514`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x07DB)]
#[packet_size(8)]
pub struct HomunculusParameterChangePacket2019 {
    pub parameter: u16,
    pub value: i32,
}

/// `ZC_MER_INIT` (0x029B): full mercenary status sheet at PACKETVER 20190605.
///
/// rAthena writes this by hand in `clif_mercenary_info` (clif.cpp:18234) and
/// registers it as 80 bytes (`packet(0x029b,80)`, clif_packetdb.hpp:944).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x029B)]
#[packet_size(80)]
pub struct MercenaryInfoPacket2019 {
    pub entity_id: EntityId,
    pub attack: u16,
    pub magic_attack: u16,
    pub hit: u16,
    pub critical: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub flee: u16,
    pub attack_motion: u16,
    #[length(24)]
    pub name: String,
    pub level: u16,
    pub health_points: u32,
    pub maximum_health_points: u32,
    pub spell_points: u32,
    pub maximum_spell_points: u32,
    /// Unix timestamp (client tick) when the mercenary disappears.
    pub expire_time: u32,
    pub faith: u16,
    pub calls: u32,
    pub kills: u32,
    pub attack_range: u16,
}

/// `ZC_MER_PAR_CHANGE` (0x02A2): mercenary parameter change. The parameter
/// ids are the rAthena `_sp` values (`SP_HP` 5, `SP_MAXHP` 6, `SP_SP` 7,
/// `SP_MERCKILLS` 189, `SP_MERCFAITH` 190).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x02A2)]
#[packet_size(8)]
pub struct MercenaryParameterChangePacket2019 {
    pub parameter: u16,
    pub value: i32,
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

    #[test]
    fn guild_create_request_layout_matches_20190605() {
        let packet = GuildCreateRequestPacket {
            character_id: CharacterId(0x1234_5678),
            name: "Domination".to_owned(),
        };

        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        let bytes = writer.as_slice();
        assert_eq!(bytes.len(), 30, "CZ_REQ_MAKE_GUILD is 30 bytes");
        assert_eq!(&bytes[0..2], &[0x65, 0x01], "header at offset 0");
        assert_eq!(&bytes[2..6], &0x1234_5678u32.to_le_bytes(), "char id at offset 2");
        assert_eq!(&bytes[6..16], b"Domination", "name at offset 6");
        assert_eq!(&bytes[16..30], &[0u8; 14], "name is zero padded to 24 bytes");
    }

    #[test]
    fn guild_create_request_rejects_a_name_over_24_bytes() {
        let packet = GuildCreateRequestPacket {
            character_id: CharacterId(1),
            name: "abcdefghijklmnopqrstuvwxy".to_owned(),
        };

        let mut writer = ByteWriter::new();
        assert!(
            packet.packet_to_bytes(&mut writer).is_err(),
            "25 bytes do not fit the 24-byte field"
        );
    }

    #[test]
    fn guild_create_result_is_3_bytes() {
        let mut writer = ByteWriter::new();
        GuildCreateResultPacket { result: 3 }.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice().len(), 3, "ZC_RESULT_MAKE_GUILD is 3 bytes");
        assert_eq!(writer.as_slice(), &[0x67, 0x01, 0x03]);
    }

    #[test]
    fn guild_invite_from_server_layout_matches_20190605() {
        // Header + guild id + name[24], with a full 24-byte name (no NUL).
        let mut bytes = vec![0u8; 30];
        bytes[0..2].copy_from_slice(&0x016Au16.to_le_bytes());
        bytes[2..6].copy_from_slice(&42u32.to_le_bytes());
        bytes[6..30].copy_from_slice(b"ABCDEFGHIJKLMNOPQRSTUVWX");

        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = GuildInviteRequestFromServerPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(reader.get_offset(), 30);
        assert_eq!(packet.guild_id, 42);
        assert_eq!(packet.guild_name.len(), 24, "the 24-byte field is not truncated");
        assert_eq!(packet.guild_name, "ABCDEFGHIJKLMNOPQRSTUVWX");
    }

    #[test]
    fn guild_invite_reply_is_10_bytes() {
        let mut writer = ByteWriter::new();
        GuildInviteReplyPacket { guild_id: 42, answer: 1 }
            .packet_to_bytes(&mut writer)
            .unwrap();
        assert_eq!(writer.as_slice(), &[0x6B, 0x01, 42, 0, 0, 0, 1, 0, 0, 0]);

        let mut writer = ByteWriter::new();
        GuildInviteReplyPacket { guild_id: 42, answer: 0 }
            .packet_to_bytes(&mut writer)
            .unwrap();
        assert_eq!(writer.as_slice(), &[0x6B, 0x01, 42, 0, 0, 0, 0, 0, 0, 0]);
    }
}

/// `ZC_ACK_LEAVE_GUILD` (0x0A83 at PACKETVER >= 20161019), 46 bytes.
///
/// `clif_guild_leave` (clif.cpp:9124) notifies every guild member that a
/// character left the guild. The 2016+ layout replaces the 24-byte name with
/// the character id (`PACKET_ZC_ACK_LEAVE_GUILD2`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A83)]
#[packet_size(46)]
pub struct GuildLeaveNoticePacket {
    pub character_id: CharacterId,
    #[length(40)]
    pub reason: String,
}

/// `ZC_ACK_REQ_JOIN_GUILD` (0x0169), 3 bytes.
///
/// `clif_guild_inviteack` (clif.cpp:9111) only sends this on failure:
/// `0` = target cannot be invited (already in a guild / trading),
/// `1` = target offline, `3` = guild full. There is no success ack; the
/// invitation itself is `ZC_REQ_JOIN_GUILD` (0x016A) to the target.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0169)]
#[packet_size(3)]
pub struct GuildInviteResultPacket {
    pub result: u8,
}

/// `ZC_REQ_JOIN_GUILD` (0x016A), 30 bytes: the server invites the local player
/// to a guild.
///
/// `clif_guild_invite` (clif.cpp) sends the guild id at offset 2 and the
/// 24-byte guild name at offset 6. The local player answers with
/// [`GuildInviteReplyPacket`] (0x016B).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x016A)]
#[packet_size(30)]
pub struct GuildInviteRequestFromServerPacket {
    pub guild_id: u32,
    #[length(24)]
    pub guild_name: String,
}

/// `CZ_JOIN_GUILD` (0x016B), 10 bytes: accept or refuse a guild invitation.
///
/// `clif_parse_GuildInviteAnswer` (clif.cpp): the guild id at offset 2 and the
/// answer at offset 6 (`0` = refuse, `1` = accept).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x016B)]
#[packet_size(10)]
pub struct GuildInviteReplyPacket {
    pub guild_id: u32,
    pub answer: u32,
}

// --- Bank (rAthena `clif_parse_Bank*`, PACKETVER >= 20130717) --------------
//
// All layouts come from `src/map/packets.hpp` (structs `PACKET_ZC_*BANKING*`)
// and the parsers in `src/map/clif.cpp:7327-7535`.

/// `ZC_ACK_CREATE_CHATROOM` (0x00D6), 3 bytes.
///
/// `clif_createchat` (clif.cpp:4475-4485). `flag`:
/// `0` = created, `1` = limit exceeded, `2` = same room already exists.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00D8)]
#[packet_size(6)]
pub struct DestroyChatRoomPacket {
    pub chat_id: u32,
}

/// One member row of [`EnterChatRoomPacket`] (24 byte name + 4 byte role).
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00E3)]
#[packet_size(2)]
pub struct ExitChatRoomPacket {}
