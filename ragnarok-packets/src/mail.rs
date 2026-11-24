//! mail packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09E7)]
#[packet_size(3)]
pub struct NewMailStatusPacket {
    pub new_available: u8,
}

/// One inbox row of `ZC_ACK_MAIL_LIST3` (0x0AC2); rAthena's `maillistinfo`
/// struct (src/map/packets_struct.hpp).
///
/// Layout: `{ open type }.{B} { mail id }.{Q} { read }.{B} { mail type }.{B}
/// { sender }.{24B} { scheduled deletion }.{L} { title length }.{W}
/// { title }.{?B}` for PACKETVER >= 20170419.
#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AC0)]
pub struct MailboxOpenRequestPacket {
    pub mail_id: u64,
    pub unknown: [u8; 16],
}

/// `CZ_REQ_REFRESH_MAIL_LIST2` (0x0AC1): force a mailbox refresh.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AC1)]
pub struct MailboxRefreshRequestPacket {
    pub mail_id: u64,
    pub unknown: [u8; 16],
}

/// `CZ_REQ_READ_MAIL` (0x09EA): request the body of one mail.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09EA)]
pub struct MailReadRequestPacket {
    pub open_type: u8,
    pub mail_id: u64,
}

/// `CZ_REQ_ZENY_FROM_MAIL` (0x09F1): claim the zeny attachment.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09F1)]
pub struct MailClaimZenyRequestPacket {
    pub mail_id: u64,
    pub open_type: u8,
}

/// `CZ_REQ_ITEM_FROM_MAIL` (0x09F3): claim the item attachments.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09F3)]
pub struct MailClaimItemRequestPacket {
    pub mail_id: u64,
    pub open_type: u8,
}

/// `CZ_REQ_DELETE_MAIL` (0x09F5): delete a mail without attachments.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
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
