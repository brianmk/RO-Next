//! `InputEvent` handlers for the session family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::LogIn`.
    pub(crate) fn handle_input_log_in(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LogIn {
            service_id,
            username,
            password,
        } = event
        else {
            unreachable!("handle_input_log_in called with a different event variant");
        };

        // The selected service can be stale (persisted `recent_service_id`
        // from an older client info) and its address can fail DNS
        // resolution. Both must show an error window, never panic.
        let services = self.client_state.follow(client_state().client_info().services()).clone();
        let target = match crate::login_server::resolve_login_server(&services, service_id) {
            Ok(target) => target,
            Err(error) => {
                crate::logging::log_warn!("cannot connect to the selected login server: {error:?}");
                self.interface.open_window(ErrorWindow::new(error.user_message()));
                return;
            }
        };
        let socket_address = target.socket_address;
        let packet_version = target.packet_version;

        self.saved_login_server_address = Some(socket_address);
        self.saved_username = username.clone();
        self.saved_password = password.clone();
        self.saved_packet_version = packet_version;

        self.session.transition(SessionEvent::LoginStarted);
        self.networking_system
            .connect_to_login_server(packet_version, socket_address, username, password);
    }

    /// `InputEvent::SelectServer`.
    pub(crate) fn handle_input_select_server(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SelectServer {
            character_server_information,
        } = event
        else {
            unreachable!("handle_input_select_server called with a different event variant");
        };

        let Some(login_data) = self.saved_login_data else {
            // The window was still open although we are not logged in (e.g.
            // after a stale state). Go back to the login screen instead of
            // panicking.
            self.interface.close_window_with_class(WindowClass::SelectServer);
            self.interface.open_window(LoginWindow::new(
                client_state().login_window(),
                client_state().login_settings(),
                client_state().client_info(),
            ));
            return;
        };

        self.saved_character_server = Some(character_server_information.clone());

        self.networking_system.disconnect_from_login_server();

        // `connect_to_character_server` replaces a stale connection itself, so
        // clicking the server always starts a fresh character server session.
        self.networking_system
            .connect_to_character_server(self.saved_packet_version, &login_data, character_server_information);
    }

    /// `InputEvent::Respawn`.
    pub(crate) fn handle_input_respawn(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::Respawn = event else {
            unreachable!("handle_input_respawn called with a different event variant");
        };

        let _ = self.networking_system.respawn();
        self.interface.close_window_with_class(WindowClass::Respawn);
    }

    /// `InputEvent::LogOut`.
    pub(crate) fn handle_input_log_out(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LogOut = event else {
            unreachable!("handle_input_log_out called with a different event variant");
        };

        let _ = self.networking_system.log_out();
    }

    /// `InputEvent::LogOutCharacter`.
    pub(crate) fn handle_input_log_out_character(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LogOutCharacter = event else {
            unreachable!("handle_input_log_out_character called with a different event variant");
        };

        // The next login is a fresh session, so the character
        // selection cooldown must not leak into it.
        self.client_state.follow_mut(client_state().character_selection_cooldown()).clear();
        self.networking_system.disconnect_from_character_server();
    }

    /// `InputEvent::Exit`.
    pub(crate) fn handle_input_exit(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::Exit = event else {
            unreachable!("handle_input_exit called with a different event variant");
        };

        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst)
    }

    /// `InputEvent::SubmitPincode`.
    pub(crate) fn handle_input_submit_pincode(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SubmitPincode { code } = event else {
            unreachable!("handle_input_submit_pincode called with a different event variant");
        };

        let (seed, account_id, is_new) = {
            let pincode = self.client_state.follow(client_state().content_windows().pincode());
            (pincode.seed, pincode.account_id, pincode.is_new)
        };

        match crate::pincode::encrypt_pincode(seed, &code) {
            Some(pin) => {
                if self.networking_system.send_pincode(account_id, pin, is_new).is_ok() {
                    self.client_state
                        .follow_mut(client_state().content_windows().pincode())
                        .code
                        .clear();
                } else {
                    self.client_state.follow_mut(client_state().content_windows().pincode()).error =
                        "Not connected to the character server.".to_owned();
                }
            }
            None => {
                self.client_state.follow_mut(client_state().content_windows().pincode()).error = "Enter exactly 4 digits.".to_owned();
            }
        }
    }

    /// `InputEvent::ClosePincodePrompt`.
    pub(crate) fn handle_input_close_pincode_prompt(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ClosePincodePrompt = event else {
            unreachable!("handle_input_close_pincode_prompt called with a different event variant");
        };

        self.client_state.follow_mut(client_state().content_windows().pincode()).dismiss();
        self.interface.close_window_with_class(WindowClass::Pincode);
    }
}
