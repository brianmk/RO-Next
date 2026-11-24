//! login packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

/// Sent by the client to the login server.
/// The very first packet sent when logging in, it is sent after the user has
/// entered email and password.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x020D)]
#[variable_length]
pub struct CharacterBanListPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterBanInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct CharacterBanInformation {
    pub character_id: CharacterId,
    #[length(20)]
    pub ban_time: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x08B9)]
#[packet_size(12)]
pub struct LoginPincodePacket {
    pub pincode_seed: u32,
    pub account_id: AccountId,
    pub state: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x006A)]
#[packet_size(3)]
pub struct MapServerAuthFailedPacket {
    pub reason: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0283)]
#[packet_size(6)]
pub struct MapServerAuthOkPacket {
    pub account_id: AccountId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum LoginFailedReason {
    #[numeric_value(1)]
    ServerClosed,
    #[numeric_value(2)]
    AlreadyLoggedIn,
    #[numeric_value(8)]
    AlreadyOnline,
}

#[derive(Debug, Clone, Packet, ServerPacket, LoginServer, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0081)]
#[packet_size(3)]
pub struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0840)]
#[variable_length]
pub struct MapServerUnavailablePacket {
    #[length_remaining]
    pub unknown: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x083E)]
#[packet_size(26)]
pub struct LoginFailedPacket2 {
    pub reason: LoginFailedReason2,
    #[length(20)]
    pub date: String,
}

/// Sent by the client to the login server every 60 seconds to keep the
/// connection alive.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0200)]
#[ping]
pub struct LoginServerKeepalivePacket {
    #[new_value(UserId([0; 24]))]
    pub user_id: UserId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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

/// Sent by the map server to the client.
#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B1D)]
#[packet_size(2)]
#[ping]
pub struct MapServerPingPacket {}

/// Sent by the map server to the client when there is a new chat message from
/// the server. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x008E)]
#[variable_length]
pub struct ServerMessagePacket {
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x007F)]
#[packet_size(6)]
#[ping]
pub struct ServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0360)]
#[ping]
pub struct RequestServerTickPacket {
    pub client_tick: ClientTick,
}

/// 2019 era `ZC_USER_COUNT` (0x00C2): answer to `/who`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00C2)]
#[packet_size(6)]
pub struct UserCountPacket2019 {
    pub player_count: i32,
}

/// 2019 era `ZC_REFUSE_ENTER` (0x0074): the map server refused the map login.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0074)]
#[packet_size(3)]
pub struct RefuseEnterPacket2019 {
    pub error_code: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0187)]
#[ping]
pub struct CharacterServerKeepalivePacket {
    /// rAthena never reads this value, so just set it to 0.
    #[new_value(AccountId(0))]
    pub account_id: AccountId,
}

/// `SC_NOTIFY_BAN` (0x0081) as sent by the **map** server when it kicks an
/// already connected session (e.g. "ID already logged in" after a stale
/// connection). The login/character server variant is [LoginFailedPacket].
///
/// Unregistered this is a 3-byte unknown packet: the stream resync then reads
/// the error code as a length and discards the rest of the TCP read.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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

/// `ZC_ACK_BAN_GUILD` (0x0A82 at PACKETVER >= 20161019), 46 bytes.
///
/// `clif_guild_expulsion` (clif.cpp:9143); unlike the leave packet the reason
/// comes first (`PACKET_ZC_ACK_BAN_GUILD3`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A82)]
#[packet_size(46)]
pub struct GuildBanNoticePacket {
    #[length(40)]
    pub reason: String,
    pub character_id: CharacterId,
}

/// `ZC_ACK_OPEN_BANKING` (0x09B7), 4 bytes: the server accepted the banking
/// session (`clif_bank_open`, `src/map/clif.cpp:7327`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09B7)]
#[packet_size(4)]
pub struct BankOpenAckPacket {
    /// Always 0 in rAthena.
    pub unknown: u16,
}

/// `ZC_ACK_CLOSE_BANKING` (0x09B9), 4 bytes (`clif_bank_close`,
/// `src/map/clif.cpp:7369`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09B9)]
#[packet_size(4)]
pub struct BankCloseAckPacket {
    /// Always 0 in rAthena.
    pub unknown: u16,
}

/// `ZC_BANKING_CHECK` (0x09A6), 12 bytes: the current bank vault and a reason
/// code (`clif_Bank_Check`, `src/map/clif.cpp:7405`).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09B6)]
#[packet_size(6)]
pub struct BankOpenRequestPacket {
    pub account_id: AccountId,
}

/// `CZ_REQ_CLOSE_BANKING` (0x09B8), 6 bytes
/// (`clif_parse_BankClose`, `src/map/clif.cpp:7383`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09B8)]
#[packet_size(6)]
pub struct BankCloseRequestPacket {
    pub account_id: AccountId,
}

/// `CZ_REQ_BANKING_CHECK` (0x09AB), 6 bytes: request `ZC_BANKING_CHECK`
/// (`clif_parse_BankCheck`, `src/map/clif.cpp:7419`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09AB)]
#[packet_size(6)]
pub struct BankCheckRequestPacket {
    pub account_id: AccountId,
}

/// `CZ_REQ_BANKING_DEPOSIT` (0x09A7), 10 bytes (`clif_parse_BankDeposit`,
/// `src/map/clif.cpp:7462`).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09A9)]
#[packet_size(10)]
pub struct BankWithdrawRequestPacket {
    pub account_id: AccountId,
    /// Amount to move out of the bank vault.
    pub zeny: u32,
}

/// `ZC_REFUSE_ENTER_ROOM` (0x00DA), 3 bytes.
///
/// `clif_joinchatfail` (clif.cpp:4579-4590). `result`:
/// `0` = full, `1` = wrong password, `2` = kicked, `3` = success,
/// `4` = not enough zeny, `5` = level too low, `6` = level too high,
/// `7` = unsuitable job class.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x00DA)]
#[packet_size(3)]
pub struct RefuseEnterChatRoomPacket {
    pub result: u8,
}

/// `CZ_PINCODE_CHECK` (0x08B8), 10 bytes: answer to `HC_ACK_PINCODE` when the
/// account already has a PIN.
///
/// `chclif_parse_pincode_check` (char_clif.cpp:189-213) reads the account id at
/// offset 2 and four PIN bytes at offset 6. The bytes are obfuscated with the
/// seed of the `HC_ACK_PINCODE` packet, see
/// `ro_next::pincode::encrypt_pincode`.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x08BA)]
#[packet_size(10)]
pub struct PincodeSetNewPacket {
    pub account_id: AccountId,
    pub pin: [u8; 4],
}
