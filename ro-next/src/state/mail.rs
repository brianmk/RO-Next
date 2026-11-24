//! Rodex mailbox state.
//!
//! At PACKETVER 20190605 rAthena only speaks the RODEX mail family (the old
//! 0x0240/0x0242 mailbox is behind `#if PACKETVER < 20150513`):
//!
//! * `ZC_ACK_MAIL_LIST3` (0x0AC2, clif.cpp clif_Mail_refreshinbox) carries the
//!   whole inbox: mail id, inbox tab, read flag, attachment flags, sender,
//!   scheduled deletion and title. This packet version no longer sends the
//!   received timestamp, so the mailbox shows the expiry instead of the date.
//! * `ZC_ACK_READ_RODEX` (0x09EB, clif.cpp clif_Mail_read) carries the body,
//!   the attached zeny and the attachment items (item id/amount/refine; the
//!   item name is not part of the packet).
//! * `ZC_ACK_ZENY_FROM_MAIL` (0x09F2) / `ZC_ACK_ITEM_FROM_MAIL` (0x09F4) report
//!   whether claiming an attachment worked.
//! * `ZC_ACK_DELETE_MAIL` (0x09F6) confirms a deletion.
//!
//! The body of a read mail is merged into the list entry by mail id because the
//! server sends a fresh list right after the read acknowledgement.

use ragnarok_packets::{MailAttachmentItem2019, MailListEntry2019};
use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// Attachment flag of the list packet: the mail contains zeny
/// (`enum mail_type` in clif.cpp: `MAIL_TYPE_ZENY = 0x2`).
pub const MAIL_TYPE_ZENY: u8 = 0x2;
/// Attachment flag of the list packet: the mail contains items
/// (`MAIL_TYPE_ITEM = 0x4`).
pub const MAIL_TYPE_ITEM: u8 = 0x4;
/// The mail was sent by an NPC/script (`MAIL_TYPE_NPC = 0x8`).
pub const MAIL_TYPE_NPC: u8 = 0x8;

/// One attachment item of an opened mail.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct MailAttachment {
    pub item_id: u32,
    pub amount: u16,
    pub identified: bool,
    pub refine: u8,
}

impl From<&MailAttachmentItem2019> for MailAttachment {
    fn from(item: &MailAttachmentItem2019) -> Self {
        Self {
            item_id: item.item_id,
            amount: item.amount,
            identified: item.identified != 0,
            refine: item.refine,
        }
    }
}

/// Body and attachments of a mail that was opened.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct MailBody {
    pub text: String,
    pub zeny: u64,
    pub items: Vec<MailAttachment>,
}

/// One mail of the inbox.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct Mail {
    pub mail_id: u64,
    /// Mailbox tab (`mail_inbox_type`): 0 = normal, 1 = account, 2 = returned.
    pub open_type: u8,
    pub read: bool,
    /// Attachment flags, see [`MAIL_TYPE_ZENY`] / [`MAIL_TYPE_ITEM`] /
    /// [`MAIL_TYPE_NPC`].
    pub mail_type: u8,
    pub sender: String,
    /// Seconds until the server automatically deletes the mail (rAthena fakes
    /// this to one year when automatic deletion is disabled).
    pub expires_in_seconds: u32,
    pub title: String,
    /// Filled by `ZC_ACK_READ_RODEX`; survives a list refresh.
    pub body: Option<MailBody>,
}

impl Mail {
    fn from_list_entry(entry: &MailListEntry2019) -> Self {
        Self {
            mail_id: entry.mail_id,
            open_type: entry.open_type,
            read: entry.read != 0,
            mail_type: entry.mail_type,
            sender: entry.sender.clone(),
            expires_in_seconds: entry.expires_in_seconds,
            title: entry.title.clone(),
            body: None,
        }
    }

    pub fn has_zeny(&self) -> bool {
        self.mail_type & MAIL_TYPE_ZENY != 0
    }

    pub fn has_items(&self) -> bool {
        self.mail_type & MAIL_TYPE_ITEM != 0
    }

    pub fn is_from_npc(&self) -> bool {
        self.mail_type & MAIL_TYPE_NPC != 0
    }

    pub fn has_attachments(&self) -> bool {
        self.has_zeny() || self.has_items()
    }

    /// Human readable attachment part of the list row.
    fn attachment_text(&self) -> String {
        let mut parts = Vec::new();

        if self.has_zeny() {
            parts.push("zeny");
        }

        if self.has_items() {
            parts.push("items");
        }

        if self.is_from_npc() {
            parts.push("NPC");
        }

        match parts.is_empty() {
            true => "no attachments".to_owned(),
            false => parts.join(" + "),
        }
    }

    /// Human readable expiry. The 2019 list packet only carries the scheduled
    /// deletion, not the received timestamp of the mail.
    fn expiry_text(&self) -> String {
        let days = self.expires_in_seconds / 86_400;
        let hours = self.expires_in_seconds % 86_400 / 3_600;

        match (days, hours) {
            (0, 0) => "expires soon".to_owned(),
            (0, hours) => format!("expires in {hours}h"),
            (days, _) => format!("expires in {days}d"),
        }
    }

    /// Text of the collapsed list row.
    pub fn list_line(&self) -> String {
        let marker = match self.read {
            true => '\u{25CB}',
            false => '\u{25CF}',
        };

        format!(
            "{marker} {}: {} - {}, {}",
            self.sender,
            self.title,
            self.attachment_text(),
            self.expiry_text()
        )
    }

    /// Text shown inside the expanded row: body and attachments if the mail was
    /// opened already, a hint otherwise.
    pub fn details_text(&self) -> String {
        let Some(body) = &self.body else {
            return "Click \"Read\" to load the message.".to_owned();
        };

        let mut text = String::new();

        if body.zeny > 0 {
            text.push_str(&format!("Attached zeny: {}\n", body.zeny));
        }

        for item in &body.items {
            let identified = match item.identified {
                true => "",
                false => " (unidentified)",
            };
            let refine = match item.refine {
                0 => String::new(),
                refine => format!(" +{refine}"),
            };

            text.push_str(&format!("Item #{} x{}{}{}\n", item.item_id, item.amount, refine, identified));
        }

        if text.is_empty() {
            text.push_str("No attachments.\n");
        }

        text.push('\n');
        text.push_str(&body.text);
        text
    }
}

/// State of the Rodex mailbox.
#[derive(Default, RustState, StateElement)]
pub struct Mailbox {
    pub mails: Vec<Mail>,
    /// `ZC_NOTIFY_UNREADMAIL` (0x09E7): the server says there is unread mail.
    pub unread_mail_available: bool,
    /// Last status/error message shown at the top of the mailbox window.
    pub status: String,
}

impl Mailbox {
    pub fn unread_count(&self) -> usize {
        self.mails.iter().filter(|mail| !mail.read).count()
    }

    /// Replace the inbox with a fresh `ZC_ACK_MAIL_LIST3` list. Bodies of mails
    /// that are still in the list are preserved because the server sends a list
    /// refresh right after `ZC_ACK_READ_RODEX`.
    pub fn apply_list(&mut self, entries: Vec<MailListEntry2019>) {
        let mut previous = std::mem::take(&mut self.mails);

        self.mails = entries
            .iter()
            .map(|entry| {
                let mut mail = Mail::from_list_entry(entry);

                if let Some(index) = previous.iter().position(|old| old.mail_id == mail.mail_id) {
                    mail.body = previous.swap_remove(index).body;
                }

                mail
            })
            .collect();
    }

    /// Store the body of an opened mail (`ZC_ACK_READ_RODEX`).
    pub fn apply_read(&mut self, mail_id: u64, text: String, zeny: u64, items: Vec<MailAttachmentItem2019>) {
        let items = items.iter().map(MailAttachment::from).collect();
        let body = MailBody { text, zeny, items };

        let Some(mail) = self.mails.iter_mut().find(|mail| mail.mail_id == mail_id) else {
            self.status = "Received a mail body for an unknown mail.".to_owned();
            return;
        };

        mail.read = true;
        mail.body = Some(body);
    }

    /// Report the result of claiming an attachment (`ZC_ACK_ZENY_FROM_MAIL` /
    /// `ZC_ACK_ITEM_FROM_MAIL`).
    pub fn apply_claimed(&mut self, mail_id: u64, zeny: bool, result: u8) {
        let attachment = match zeny {
            true => "zeny",
            false => "items",
        };

        self.status = match (zeny, result) {
            (_, 0) => format!("Claimed the {attachment} attachment of mail {mail_id}."),
            (true, 1) => "Could not claim the zeny: your zeny would exceed the limit.".to_owned(),
            (false, 2) => "Could not claim the items: inventory full or too heavy.".to_owned(),
            _ => format!("Could not claim the {attachment} attachment of mail {mail_id}."),
        };
    }

    /// Remove a deleted mail (`ZC_ACK_DELETE_MAIL` is only sent on success).
    pub fn apply_deleted(&mut self, mail_id: u64) {
        self.mails.retain(|mail| mail.mail_id != mail_id);
        self.status = format!("Deleted mail {mail_id}.");
    }

    pub fn set_unread_mail_available(&mut self, unread: bool) {
        self.unread_mail_available = unread;
    }

    /// Clear everything when leaving the map server.
    pub fn clear(&mut self) {
        self.mails.clear();
        self.unread_mail_available = false;
        self.status.clear();
    }

    /// One-line summary shown above the mail list.
    pub fn status_line(&self) -> String {
        let mut line = match self.mails.is_empty() {
            true => "Mailbox is empty.".to_owned(),
            false => format!("{} mail(s), {} unread.", self.mails.len(), self.unread_count()),
        };

        if self.unread_mail_available && self.unread_count() == 0 {
            line.push_str(" The server reports unread mail; press Refresh.");
        }

        if !self.status.is_empty() {
            line.push('\n');
            line.push_str(&self.status);
        }

        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(mail_id: u64, read: u8, mail_type: u8, title: &str) -> MailListEntry2019 {
        MailListEntry2019 {
            open_type: 0,
            mail_id,
            read,
            mail_type,
            sender: "Alice".to_owned(),
            expires_in_seconds: 86_400,
            title_length: title.len() as u16 + 1,
            title: title.to_owned(),
        }
    }

    #[test]
    fn list_refresh_keeps_the_body_of_read_mails() {
        let mut mailbox = Mailbox::default();
        mailbox.apply_list(vec![entry(7, 0, 0x2 | 0x4, "Hello")]);

        mailbox.apply_read(7, "Body text".to_owned(), 1_500, Vec::new());
        assert_eq!(mailbox.mails[0].body.as_ref().unwrap().text, "Body text");
        assert!(mailbox.mails[0].read);

        // The server sends a fresh list right after the read acknowledgement;
        // the body has to survive it and the mail is now marked read.
        mailbox.apply_list(vec![entry(7, 1, 0x2 | 0x4, "Hello")]);
        assert_eq!(mailbox.mails[0].body.as_ref().unwrap().text, "Body text");
        assert!(mailbox.mails[0].read);
    }

    #[test]
    fn claim_result_updates_the_status_line() {
        let mut mailbox = Mailbox::default();
        mailbox.apply_list(vec![entry(7, 0, 0x2, "Hello")]);

        mailbox.apply_claimed(7, true, 0);
        assert!(mailbox.status.contains("Claimed"));

        mailbox.apply_claimed(7, false, 2);
        assert!(mailbox.status.contains("inventory full"));

        mailbox.apply_deleted(7);
        assert!(mailbox.mails.is_empty());
    }

    #[test]
    fn attachment_flags_follow_the_server_mail_type() {
        let mailbox_entry = entry(7, 0, 0x2 | 0x4 | 0x8, "Hello");
        let mail = Mail::from_list_entry(&mailbox_entry);

        assert!(mail.has_zeny());
        assert!(mail.has_items());
        assert!(mail.is_from_npc());
        assert!(mail.has_attachments());
        assert!(!mail.read);

        let line = mail.list_line();
        assert!(line.contains("Alice"));
        assert!(line.contains("Hello"));
        assert!(line.contains("zeny + items"));
        assert!(line.contains('\u{25CF}'));
    }
}
