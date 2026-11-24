//! `InputEvent` handlers for the ui family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::OpenCompanionInfo`.
    pub(crate) fn handle_input_open_companion_info(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenCompanionInfo { entity_id } = event else {
            unreachable!("handle_input_open_companion_info called with a different event variant");
        };

        let kind = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .and_then(|entity| match entity.get_entity_type() {
                EntityType::Pet => Some(CompanionKind::Pet),
                EntityType::Homunculus => Some(CompanionKind::Homunculus),
                EntityType::Mercenary => Some(CompanionKind::Mercenary),
                _ => None,
            });

        if let Some(kind) = kind {
            self.interface.close_window_with_class(WindowClass::CompanionInfo);
            self.interface.open_window(CompanionInfoWindow::new(entity_id, kind));
        }
    }

    /// `InputEvent::OpenBookPage`.
    pub(crate) fn handle_input_open_book_page(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenBookPage { book_id, page } = event else {
            unreachable!("handle_input_open_book_page called with a different event variant");
        };

        let pages = load_book_pages(&self.game_file_loader, book_id);
        self.interface.close_window_with_class(WindowClass::Book);
        self.interface.open_window(BookWindow::new(book_id, page, pages));
    }

    /// `InputEvent::CloseTopWindow`.
    pub(crate) fn handle_input_close_top_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseTopWindow = event else {
            unreachable!("handle_input_close_top_window called with a different event variant");
        };

        let pending_warp_point = self.client_state.follow(client_state().dialog_window()).warp_point_skill();
        let dialog_was_open = self.interface.is_window_with_class_open(WindowClass::Dialog);
        self.interface.close_top_window(&self.client_state);

        // Closing the Warp Portal destination list without a choice
        // answers the server with an invalid destination so it can drop
        // the pending cast state (`skill_castend_map` clears the menu).
        if !self.interface.is_window_with_class_open(WindowClass::Dialog)
            && dialog_was_open
            && let Some(skill_id) = pending_warp_point
        {
            let dialog = self.client_state.follow_mut(client_state().dialog_window());
            dialog.clear_warp_point();
            dialog.clear_choices();
            let _ = self.networking_system.select_warp_point(skill_id, String::new());
        }
    }

    /// `InputEvent::NavigateToDestination`.
    pub(crate) fn handle_input_navigate_to_destination(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::NavigateToDestination { name, map_name, position } = event else {
            unreachable!("handle_input_navigate_to_destination called with a different event variant");
        };

        self.set_navigation_destination(Some(name), &map_name, position);
    }

    /// `InputEvent::OpenCharacterContextMenu`.
    pub(crate) fn handle_input_open_character_context_menu(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::OpenCharacterContextMenu { entity_id, position } = event else {
            unreachable!("handle_input_open_character_context_menu called with a different event variant");
        };

        if open_character_context_menu(&self.client_state, &mut self.interface, entity_id, position) {
            self.pending_context_menu = Some((entity_id, position));
            let _ = self.networking_system.entity_details(entity_id);
        }
    }

    /// `InputEvent::CloseContextMenu`.
    pub(crate) fn handle_input_close_context_menu(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseContextMenu = event else {
            unreachable!("handle_input_close_context_menu called with a different event variant");
        };

        self.interface.close_window_with_class(WindowClass::ContextMenu);
    }

    /// `InputEvent::CloseBank`.
    pub(crate) fn handle_input_close_bank(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseBank = event else {
            unreachable!("handle_input_close_bank called with a different event variant");
        };

        self.close_bank_window()
    }

    /// `InputEvent::SubmitDialogInput`.
    pub(crate) fn handle_input_submit_dialog_input(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SubmitDialogInput { npc_id, string_input } = event else {
            unreachable!("handle_input_submit_dialog_input called with a different event variant");
        };

        let text = {
            let dialog = self.client_state.follow_mut(client_state().dialog_window());
            std::mem::take(&mut dialog.input_text)
        };
        let result = if string_input {
            self.networking_system.input_edit_dialog_str(npc_id, text)
        } else {
            let value = text.trim().parse::<i32>().unwrap_or(0);
            self.networking_system.input_edit_dialog(npc_id, value)
        };

        if let Err(error) = result {
            crate::logging::log_warn!("failed to send dialog input: {error:?}");
        }
    }

    /// `InputEvent::CloseRefiningUi`.
    pub(crate) fn handle_input_close_refining_ui(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseRefiningUi = event else {
            unreachable!("handle_input_close_refining_ui called with a different event variant");
        };

        let _ = self.networking_system.close_refining_ui();
        self.close_refining_window();
    }

    /// `InputEvent::NextDialog`.
    pub(crate) fn handle_input_next_dialog(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::NextDialog { npc_id } = event else {
            unreachable!("handle_input_next_dialog called with a different event variant");
        };

        let _ = self.networking_system.next_dialog(npc_id);
    }

    /// `InputEvent::CloseDialog`.
    pub(crate) fn handle_input_close_dialog(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CloseDialog { npc_id } = event else {
            unreachable!("handle_input_close_dialog called with a different event variant");
        };

        let _ = self.networking_system.close_dialog(npc_id);
        self.client_state.follow_mut(client_state().dialog_window()).end();
        self.interface.close_window_with_class(WindowClass::Dialog);
    }

    /// `InputEvent::ChooseDialogOption`.
    pub(crate) fn handle_input_choose_dialog_option(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ChooseDialogOption { npc_id, option } = event else {
            unreachable!("handle_input_choose_dialog_option called with a different event variant");
        };

        let _ = self.networking_system.choose_dialog_option(npc_id, option);

        // The official client closes its menu window as soon as an
        // option is picked; the dialog window itself stays open. In
        // our combined dialog window this means that the choice
        // buttons have to disappear right away. Otherwise the next
        // menu (or the answer of the NPC) would be appended below
        // the options of the previous menu.
        self.client_state.follow_mut(client_state().dialog_window()).clear_choices();

        if option == -1 {
            self.interface.close_window_with_class(WindowClass::Dialog);
        }
    }
}
