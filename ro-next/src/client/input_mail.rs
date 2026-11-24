//! `InputEvent` handlers for the mail family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::OpenMailboxWindow`.
    pub(crate) fn handle_input_open_mailbox_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenMailboxWindow = event else {
            unreachable!("handle_input_open_mailbox_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Mail) {
                true => self.interface.close_window_with_class(WindowClass::Mail),
                false => {
                    self.interface.open_window(MailboxWindow::new(client_state().mail()));
                    // The mailbox list is only sent on request.
                    let _ = self.networking_system.open_mailbox();
                }
            }
        }
    }

    /// `InputEvent::RefreshMailbox`.
    pub(crate) fn handle_input_refresh_mailbox(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RefreshMailbox = event else {
            unreachable!("handle_input_refresh_mailbox called with a different event variant");
        };

        let _ = self.networking_system.refresh_mailbox();
    }

    /// `InputEvent::ReadMail`.
    pub(crate) fn handle_input_read_mail(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ReadMail { mail_id, open_type } = event else {
            unreachable!("handle_input_read_mail called with a different event variant");
        };

        let _ = self.networking_system.read_mail(mail_id, open_type);
    }

    /// `InputEvent::ClaimMailZeny`.
    pub(crate) fn handle_input_claim_mail_zeny(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ClaimMailZeny { mail_id, open_type } = event else {
            unreachable!("handle_input_claim_mail_zeny called with a different event variant");
        };

        let _ = self.networking_system.claim_mail_zeny(mail_id, open_type);
    }

    /// `InputEvent::ClaimMailItems`.
    pub(crate) fn handle_input_claim_mail_items(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ClaimMailItems { mail_id, open_type } = event else {
            unreachable!("handle_input_claim_mail_items called with a different event variant");
        };

        let _ = self.networking_system.claim_mail_items(mail_id, open_type);
    }

    /// `InputEvent::DeleteMail`.
    pub(crate) fn handle_input_delete_mail(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::DeleteMail { mail_id, open_type } = event else {
            unreachable!("handle_input_delete_mail called with a different event variant");
        };

        let _ = self.networking_system.delete_mail(mail_id, open_type);
    }
}
