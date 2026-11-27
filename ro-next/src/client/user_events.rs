//! The interface/user input dispatcher.
//!
//! Routes each `InputEvent` to its handler in `input_*.rs`. The debug camera
//! arms stay inline: they are the only ones that take `delta_time`, which is
//! compiled in only with the `debug` feature.

use crate::*;

impl Client {
    /// Returns whether or not the interface is focused.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn process_user_events(
        &mut self,
        input_report: &InputReport,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] delta_time: f32,
    ) -> bool {
        self.interface.process_events(&mut self.input_event_buffer);

        // Left-clicking an item box only prepares a drag; the actual drag starts once
        // the button was held for a moment or the mouse moved a few pixels while held.
        // A quick click releases the prepared state without picking the item up.
        const ITEM_DRAG_HOLD_TIME_MS: u128 = 500;
        const ITEM_DRAG_DISTANCE: f32 = 5.0;

        match self.interface.mouse_mode().prepare_item_drag() {
            Some((source, item)) => {
                let now = std::time::Instant::now();
                let pending = *self.pending_item_drag.get_or_insert((now, input_report.mouse_position));
                let held_time_ms = now.duration_since(pending.0).as_millis();
                let moved_distance = (input_report.mouse_position.left - pending.1.left)
                    .abs()
                    .max((input_report.mouse_position.top - pending.1.top).abs());

                if input_report.mouse_button_released || !input_report.left_mouse_button_down && moved_distance == 0.0 {
                    // The button was released before the thresholds were reached: this
                    // was a plain click, not a drag.
                    self.pending_item_drag = None;
                } else if held_time_ms >= ITEM_DRAG_HOLD_TIME_MS || moved_distance > ITEM_DRAG_DISTANCE {
                    self.pending_item_drag = None;

                    self.interface.set_mouse_mode(MouseMode::Custom {
                        mode: MouseInputMode::MoveItem { item, source },
                    });
                }
            }
            None => self.pending_item_drag = None,
        }

        // A press on a hotbar skill only arms a pending skill drag; the slot
        // turns the release into a use (released on the slot the press started
        // on) or a move (released on another slot). The release itself resets
        // the mode in `Interface::drop`; if it was lost (the pointer left the
        // surface while the button was held) the pending mode would stay active
        // and keep the skill glued to the cursor, so clear it as well.
        if self.interface.mouse_mode().prepare_skill_drag().is_some() && !input_report.left_mouse_button_down {
            self.interface.set_mouse_mode(MouseMode::Default);
        }

        let interface_has_focus = self.interface.has_focus();

        // Enter submits the login form and confirms the server selection, so the
        // menus can be used without reaching for the mouse. The keys are
        // data-driven (`chat_command_keys` in `data\client\keybinds.ron`).
        if self.input_system.chat_command_key_pressed() {
            if self.interface.is_window_with_class_open(WindowClass::Login) {
                let selected_service_path = SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                let service_id = *self.client_state.follow(client_state().login_window().selected_service());
                let username = self.client_state.follow(selected_service_path.username()).clone();
                let password = self.client_state.follow(selected_service_path.password()).clone();

                if !username.is_empty() && !password.is_empty() {
                    self.input_event_buffer.push(InputEvent::LogIn {
                        service_id,
                        username,
                        password,
                    });
                }
            } else if self.interface.is_window_with_class_open(WindowClass::SelectServer)
                && let Some(character_server_information) = self.client_state.follow(client_state().character_servers()).first().cloned()
            {
                self.input_event_buffer.push(InputEvent::SelectServer {
                    character_server_information,
                });
            }
        }

        if self.interface.get_mouse_mode().is_rotating_camera() {
            // TODO: Does this really need to be a InputEvent?
            let rotation = input_report.mouse_delta.width;
            self.input_event_buffer.push(InputEvent::RotateCamera { rotation });
        }

        // Battle mode and the key binding editor both change how keyboard
        // input is routed. The state is pushed into the input system before
        // the one keyboard handler runs.
        let battle_mode = *self.client_state.follow(client_state().game_settings().battle_mode());
        self.input_system.set_battle_mode(battle_mode);

        let rebind_action = self.client_state.follow(client_state().key_rebind()).capturing();
        self.input_system.set_key_capture(rebind_action.is_some());

        // Battle mode closes the chat line with Escape, like the official
        // client: the line is the only focused element here, so the key must
        // not reach the window/menu handling.
        if rebind_action.is_none() && battle_mode && interface_has_focus && self.input_system.key_pressed(winit::keyboard::KeyCode::Escape)
        {
            self.interface.unfocus();
        }

        if let Some(action) = rebind_action {
            // While a rebind waits for a key, the key is captured instead of
            // triggering its normal action. Escape cancels the capture.
            if let Some((key, modifiers)) = self.input_system.take_captured_key() {
                if key == winit::keyboard::KeyCode::Escape {
                    self.input_event_buffer.push(InputEvent::CancelRebind);
                } else {
                    self.input_event_buffer.push(InputEvent::RebindKey {
                        action,
                        key,
                        alt: modifiers.alt,
                        control: modifiers.control,
                        shift: modifiers.shift,
                    });
                }
            }
        } else if !interface_has_focus {
            self.input_system.handle_keyboard_input(
                &mut self.input_event_buffer,
                #[cfg(feature = "debug")]
                self.interface.get_mouse_mode().is_default(),
                #[cfg(feature = "debug")]
                *self.client_state.follow(client_state().render_options().use_debug_camera()),
            );
        }

        let input_events: Vec<InputEvent> = self.input_event_buffer.drain(..).collect();

        for event in input_events {
            crate::logging::log_trace!("input: dispatch {:?}", event);

            // Any user input can change UI state (window toggles, hotkeys,
            // settings, ...), so the cached element layouts are rebuilt on the
            // next layout pass.
            self.interface.invalidate_layouts();

            match event {
                InputEvent::LogIn { .. } => self.handle_input_log_in(event, input_report, client_tick),
                InputEvent::SelectServer { .. } => self.handle_input_select_server(event, input_report, client_tick),
                InputEvent::Respawn => self.handle_input_respawn(event, input_report, client_tick),
                InputEvent::LogOut => self.handle_input_log_out(event, input_report, client_tick),
                InputEvent::LogOutCharacter => self.handle_input_log_out_character(event, input_report, client_tick),
                InputEvent::Exit => self.handle_input_exit(event, input_report, client_tick),
                InputEvent::ZoomCamera { .. } => self.handle_input_zoom_camera(event, input_report, client_tick),
                InputEvent::RotateCamera { .. } => self.handle_input_rotate_camera(event, input_report, client_tick),
                InputEvent::ResetCameraRotation => self.handle_input_reset_camera_rotation(event, input_report, client_tick),
                InputEvent::ToggleMenuWindow => self.handle_input_toggle_menu_window(event, input_report, client_tick),
                InputEvent::ToggleInventoryWindow => self.handle_input_toggle_inventory_window(event, input_report, client_tick),
                InputEvent::ToggleEquipmentWindow => self.handle_input_toggle_equipment_window(event, input_report, client_tick),
                InputEvent::ToggleSkillTreeWindow => self.handle_input_toggle_skill_tree_window(event, input_report, client_tick),
                InputEvent::ToggleStatsWindow => self.handle_input_toggle_stats_window(event, input_report, client_tick),
                InputEvent::ToggleGameSettingsWindow => self.handle_input_toggle_game_settings_window(event, input_report, client_tick),
                InputEvent::ToggleInterfaceSettingsWindow => {
                    self.handle_input_toggle_interface_settings_window(event, input_report, client_tick)
                }
                InputEvent::ToggleGraphicsSettingsWindow => {
                    self.handle_input_toggle_graphics_settings_window(event, input_report, client_tick)
                }
                InputEvent::ToggleAudioSettingsWindow => self.handle_input_toggle_audio_settings_window(event, input_report, client_tick),
                InputEvent::ToggleMinimapWindow => self.handle_input_toggle_minimap_window(event, input_report, client_tick),
                InputEvent::CycleMapWindows => self.handle_input_cycle_map_windows(event, input_report, client_tick),
                InputEvent::ToggleFriendListWindow => self.handle_input_toggle_friend_list_window(event, input_report, client_tick),
                InputEvent::OpenMailboxWindow => self.handle_input_open_mailbox_window(event, input_report, client_tick),
                InputEvent::RefreshMailbox => self.handle_input_refresh_mailbox(event, input_report, client_tick),
                InputEvent::ReadMail { .. } => self.handle_input_read_mail(event, input_report, client_tick),
                InputEvent::ClaimMailZeny { .. } => self.handle_input_claim_mail_zeny(event, input_report, client_tick),
                InputEvent::ClaimMailItems { .. } => self.handle_input_claim_mail_items(event, input_report, client_tick),
                InputEvent::DeleteMail { .. } => self.handle_input_delete_mail(event, input_report, client_tick),
                InputEvent::TogglePartyWindow => self.handle_input_toggle_party_window(event, input_report, client_tick),
                InputEvent::CreateParty { .. } => self.handle_input_create_party(event, input_report, client_tick),
                InputEvent::InviteToParty { .. } => self.handle_input_invite_to_party(event, input_report, client_tick),
                InputEvent::AcceptPartyInvite { .. } => self.handle_input_accept_party_invite(event, input_report, client_tick),
                InputEvent::RejectPartyInvite { .. } => self.handle_input_reject_party_invite(event, input_report, client_tick),
                InputEvent::LeaveParty => self.handle_input_leave_party(event, input_report, client_tick),
                InputEvent::ToggleClanWindow => self.handle_input_toggle_clan_window(event, input_report, client_tick),
                InputEvent::SetPartyBookingVisible { .. } => self.handle_input_set_party_booking_visible(event, input_report, client_tick),
                InputEvent::RegisterPartyBooking => self.handle_input_register_party_booking(event, input_report, client_tick),
                InputEvent::SearchPartyBooking { .. } => self.handle_input_search_party_booking(event, input_report, client_tick),
                InputEvent::DeletePartyBooking => self.handle_input_delete_party_booking(event, input_report, client_tick),
                InputEvent::UpdatePartyBooking => self.handle_input_update_party_booking(event, input_report, client_tick),
                InputEvent::JoinPartyBooking { .. } => self.handle_input_join_party_booking(event, input_report, client_tick),
                InputEvent::ReplyPartyBookingJoin { .. } => self.handle_input_reply_party_booking_join(event, input_report, client_tick),
                InputEvent::ChangePartyOption { .. } => self.handle_input_change_party_option(event, input_report, client_tick),
                InputEvent::ChangePartyLeader { .. } => self.handle_input_change_party_leader(event, input_report, client_tick),
                InputEvent::OpenCompanionInfo { .. } => self.handle_input_open_companion_info(event, input_report, client_tick),
                InputEvent::ToggleGuildWindow => self.handle_input_toggle_guild_window(event, input_report, client_tick),
                InputEvent::InviteToGuild { .. } => self.handle_input_invite_to_guild(event, input_report, client_tick),
                InputEvent::ExpelGuildMember { .. } => self.handle_input_expel_guild_member(event, input_report, client_tick),
                InputEvent::LeaveGuild => self.handle_input_leave_guild(event, input_report, client_tick),
                InputEvent::SaveGuildNotice { .. } => self.handle_input_save_guild_notice(event, input_report, client_tick),
                InputEvent::ToggleBigMapWindow => self.handle_input_toggle_big_map_window(event, input_report, client_tick),
                InputEvent::ToggleChatWindow => self.handle_input_toggle_chat_window(event, input_report, client_tick),
                InputEvent::ToggleChatRoomWindow => self.handle_input_toggle_chat_room_window(event, input_report, client_tick),
                InputEvent::ToggleEmotePickerWindow => self.handle_input_toggle_emote_picker_window(event, input_report, client_tick),
                InputEvent::CreateChatRoom { .. } => self.handle_input_create_chat_room(event, input_report, client_tick),
                InputEvent::JoinChatRoom { .. } => self.handle_input_join_chat_room(event, input_report, client_tick),
                InputEvent::LeaveChatRoom => self.handle_input_leave_chat_room(event, input_report, client_tick),
                InputEvent::SendChatRoomMessage { .. } => self.handle_input_send_chat_room_message(event, input_report, client_tick),
                InputEvent::SubmitPincode { .. } => self.handle_input_submit_pincode(event, input_report, client_tick),
                InputEvent::ClosePincodePrompt => self.handle_input_close_pincode_prompt(event, input_report, client_tick),
                InputEvent::OpenBookPage { .. } => self.handle_input_open_book_page(event, input_report, client_tick),
                InputEvent::CloseTopWindow => self.handle_input_close_top_window(event, input_report, client_tick),
                InputEvent::OpenCartWindow => self.handle_input_open_cart_window(event, input_report, client_tick),
                InputEvent::BrowseVendingShop { .. } => self.handle_input_browse_vending_shop(event, input_report, client_tick),
                InputEvent::BuyVendingItem { .. } => self.handle_input_buy_vending_item(event, input_report, client_tick),
                InputEvent::VendingOpen => self.handle_input_vending_open(event, input_report, client_tick),
                InputEvent::VendingCancel => self.handle_input_vending_cancel(event, input_report, client_tick),
                InputEvent::ToggleQuestWindow => self.handle_input_toggle_quest_window(event, input_report, client_tick),
                InputEvent::ToggleAchievementWindow => self.handle_input_toggle_achievement_window(event, input_report, client_tick),
                InputEvent::ToggleNavigationWindow => self.handle_input_toggle_navigation_window(event, input_report, client_tick),
                InputEvent::ToggleMonsterDatabaseWindow => {
                    self.handle_input_toggle_monster_database_window(event, input_report, client_tick)
                }
                InputEvent::ClaimAchievementReward { .. } => self.handle_input_claim_achievement_reward(event, input_report, client_tick),
                InputEvent::NavigateToDestination { .. } => self.handle_input_navigate_to_destination(event, input_report, client_tick),
                InputEvent::ClearNavigationDestination => self.handle_input_clear_navigation_destination(event, input_report, client_tick),
                InputEvent::SelectMonster { .. } => self.handle_input_select_monster(event, input_report, client_tick),
                InputEvent::TradeRequest { .. } => self.handle_input_trade_request(event, input_report, client_tick),
                InputEvent::TradeAccept { .. } => self.handle_input_trade_accept(event, input_report, client_tick),
                InputEvent::TradeDecline => self.handle_input_trade_decline(event, input_report, client_tick),
                InputEvent::TradeAddItem { .. } => self.handle_input_trade_add_item(event, input_report, client_tick),
                InputEvent::TradeOfferZeny => self.handle_input_trade_offer_zeny(event, input_report, client_tick),
                InputEvent::TradeLock => self.handle_input_trade_lock(event, input_report, client_tick),
                InputEvent::TradeCommit => self.handle_input_trade_commit(event, input_report, client_tick),
                InputEvent::TradeCancel => self.handle_input_trade_cancel(event, input_report, client_tick),
                InputEvent::ViewCharacterInfo { .. } => self.handle_input_view_character_info(event, input_report, client_tick),
                InputEvent::ToggleShowInterface => self.handle_input_toggle_show_interface(event, input_report, client_tick),
                InputEvent::SelectCharacter { .. } => self.handle_input_select_character(event, input_report, client_tick),
                InputEvent::OpenCharacterCreationWindow { .. } => {
                    self.handle_input_open_character_creation_window(event, input_report, client_tick)
                }
                InputEvent::CreateCharacter { .. } => self.handle_input_create_character(event, input_report, client_tick),
                InputEvent::OpenCharacterDeleteWindow { .. } => {
                    self.handle_input_open_character_delete_window(event, input_report, client_tick)
                }
                InputEvent::DeleteCharacter { .. } => self.handle_input_delete_character(event, input_report, client_tick),
                InputEvent::SwitchCharacterSlot { .. } => self.handle_input_switch_character_slot(event, input_report, client_tick),
                InputEvent::PlayerMove { .. } => self.handle_input_player_move(event, input_report, client_tick),
                InputEvent::OpenCharacterContextMenu { .. } => {
                    self.handle_input_open_character_context_menu(event, input_report, client_tick)
                }
                InputEvent::CloseContextMenu => self.handle_input_close_context_menu(event, input_report, client_tick),
                InputEvent::OpenInventoryItemMenu { .. } => self.handle_input_open_inventory_item_menu(event, input_report, client_tick),
                InputEvent::OpenStorageItemMenu { .. } => self.handle_input_open_storage_item_menu(event, input_report, client_tick),
                InputEvent::DepositInventoryItem { .. } => self.handle_input_deposit_inventory_item(event, input_report, client_tick),
                InputEvent::WithdrawStorageItem { .. } => self.handle_input_withdraw_storage_item(event, input_report, client_tick),
                InputEvent::CloseStorage => self.handle_input_close_storage(event, input_report, client_tick),
                InputEvent::ToggleBankWindow => self.handle_input_toggle_bank_window(event, input_report, client_tick),
                InputEvent::BankDeposit => self.handle_input_bank_deposit(event, input_report, client_tick),
                InputEvent::BankWithdraw => self.handle_input_bank_withdraw(event, input_report, client_tick),
                InputEvent::CloseBank => self.handle_input_close_bank(event, input_report, client_tick),
                InputEvent::ItemDoubleClick { .. } => self.handle_input_item_double_click(event, input_report, client_tick),
                InputEvent::SubmitDialogInput { .. } => self.handle_input_submit_dialog_input(event, input_report, client_tick),
                InputEvent::DropItem { .. } => self.handle_input_drop_item(event, input_report, client_tick),
                InputEvent::OpenItemInfo { .. } => self.handle_input_open_item_info(event, input_report, client_tick),
                InputEvent::CloseItemInfo => self.handle_input_close_item_info(event, input_report, client_tick),
                InputEvent::ChatMarkupClicked { .. } => self.handle_input_chat_markup_clicked(event, input_report, client_tick),
                InputEvent::UseInventoryItem { .. } => self.handle_input_use_inventory_item(event, input_report, client_tick),
                InputEvent::RefineWeapon { .. } => self.handle_input_refine_weapon(event, input_report, client_tick),
                InputEvent::RepairItem { .. } => self.handle_input_repair_item(event, input_report, client_tick),
                InputEvent::IdentifyItem { .. } => self.handle_input_identify_item(event, input_report, client_tick),
                InputEvent::OneClickIdentify { .. } => self.handle_input_one_click_identify(event, input_report, client_tick),
                InputEvent::RequestCardComposition { .. } => self.handle_input_request_card_composition(event, input_report, client_tick),
                InputEvent::InsertCard { .. } => self.handle_input_insert_card(event, input_report, client_tick),
                InputEvent::CloseItemService => self.handle_input_close_item_service(event, input_report, client_tick),
                InputEvent::SelectRefiningItem { .. } => self.handle_input_select_refining_item(event, input_report, client_tick),
                InputEvent::RefineWithMaterial { .. } => self.handle_input_refine_with_material(event, input_report, client_tick),
                InputEvent::CloseRefiningUi => self.handle_input_close_refining_ui(event, input_report, client_tick),
                InputEvent::EquipInventoryItem { .. } => self.handle_input_equip_inventory_item(event, input_report, client_tick),
                InputEvent::WhisperChat { .. } => self.handle_input_whisper_chat(event, input_report, client_tick),
                InputEvent::OpenWhisper { .. } => self.handle_input_open_whisper(event, input_report, client_tick),
                InputEvent::SendWhisper { .. } => self.handle_input_send_whisper(event, input_report, client_tick),
                InputEvent::PlayerInteract { .. } => self.handle_input_player_interact(event, input_report, client_tick),
                InputEvent::PickUpItem { .. } => self.handle_input_pick_up_item(event, input_report, client_tick),
                #[cfg(feature = "debug")]
                InputEvent::WarpToMap { map_name, position } => {
                    let _ = self.networking_system.warp_to_map(map_name, position);
                }
                InputEvent::SendMessage { .. } => self.handle_input_send_message(event, input_report, client_tick),
                InputEvent::NextDialog { .. } => self.handle_input_next_dialog(event, input_report, client_tick),
                InputEvent::CloseDialog { .. } => self.handle_input_close_dialog(event, input_report, client_tick),
                InputEvent::ChooseDialogOption { .. } => self.handle_input_choose_dialog_option(event, input_report, client_tick),
                InputEvent::SelectWarpPoint { .. } => self.handle_input_select_warp_point(event, input_report, client_tick),
                InputEvent::MoveItem { .. } => self.handle_input_move_item(event, input_report, client_tick),
                InputEvent::SendEmotion { .. } => self.handle_input_send_emotion(event, input_report, client_tick),
                InputEvent::MoveSkill { .. } => self.handle_input_move_skill(event, input_report, client_tick),
                InputEvent::OpenSkillDescription { .. } => self.handle_input_open_skill_description(event, input_report, client_tick),
                InputEvent::CycleHotbarTab { .. } => self.handle_input_cycle_hotbar_tab(event, input_report, client_tick),
                InputEvent::ToggleHotbarWindow => self.handle_input_toggle_hotbar_window(event, input_report, client_tick),
                InputEvent::SwitchHotbarBar => self.handle_input_switch_hotbar_bar(event, input_report, client_tick),
                InputEvent::RotateHotbarRows { .. } => self.handle_input_rotate_hotbar_rows(event, input_report, client_tick),
                InputEvent::BeginRebind { .. } => self.handle_input_begin_rebind(event, input_report, client_tick),
                InputEvent::CancelRebind => self.handle_input_cancel_rebind(event, input_report, client_tick),
                InputEvent::RebindKey { .. } => self.handle_input_rebind_key(event, input_report, client_tick),
                InputEvent::ResetKeybinds => self.handle_input_reset_keybinds(event, input_report, client_tick),
                InputEvent::ToggleBattleMode { .. } => self.handle_input_toggle_battle_mode(event, input_report, client_tick),
                InputEvent::UseSkillInSlot { .. } => self.handle_input_use_skill_in_slot(event, input_report, client_tick),
                InputEvent::UseSkill { .. } => self.handle_input_use_skill(event, input_report, client_tick),
                InputEvent::CastSkill { .. } => self.handle_input_cast_skill(event, input_report, client_tick),
                InputEvent::StopSkill { .. } => self.handle_input_stop_skill(event, input_report, client_tick),
                InputEvent::AddFriend { .. } => self.handle_input_add_friend(event, input_report, client_tick),
                InputEvent::RemoveFriend { .. } => self.handle_input_remove_friend(event, input_report, client_tick),
                InputEvent::RejectFriendRequest { .. } => self.handle_input_reject_friend_request(event, input_report, client_tick),
                InputEvent::AcceptFriendRequest { .. } => self.handle_input_accept_friend_request(event, input_report, client_tick),
                InputEvent::BuyItems { .. } => self.handle_input_buy_items(event, input_report, client_tick),
                InputEvent::CloseShop => self.handle_input_close_shop(event, input_report, client_tick),
                InputEvent::BuyOrSell { .. } => self.handle_input_buy_or_sell(event, input_report, client_tick),
                InputEvent::SellItems { .. } => self.handle_input_sell_items(event, input_report, client_tick),
                InputEvent::StatUp { .. } => self.handle_input_stat_up(event, input_report, client_tick),
                InputEvent::DistributePointsForSkill { .. } => {
                    self.handle_input_distribute_points_for_skill(event, input_report, client_tick)
                }
                InputEvent::LevelUpSkills { .. } => self.handle_input_level_up_skills(event, input_report, client_tick),
                #[cfg(feature = "debug")]
                InputEvent::ReloadLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    *self.client_state.follow_mut(client_state().localization()) =
                        Localization::load_language(&self.game_file_loader, language);
                }
                #[cfg(feature = "debug")]
                InputEvent::SaveLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    self.client_state.follow(client_state().localization()).save_language(language);
                }
                #[cfg(feature = "debug")]
                InputEvent::OpenMarkerDetails { marker_identifier } => {
                    if let Some(map) = &self.map {
                        match marker_identifier {
                            MarkerIdentifier::Object(key) => {
                                let inspecting_objects = self.client_state.follow_mut(client_state().inspecting_objects());
                                let object = map.get_object(key);
                                let object_path = state::prepare_object_inspection(inspecting_objects, object);

                                self.interface.open_state_window(object_path);
                            }
                            MarkerIdentifier::LightSource(key) => {
                                let inspecting_lights = self.client_state.follow_mut(client_state().inspecting_light_sources());
                                let light_source = map.get_light_source(key);
                                let light_source_path = state::prepare_light_source_inspection(inspecting_lights, light_source);

                                self.interface.open_state_window(light_source_path);
                            }
                            MarkerIdentifier::SoundSource(index) => {
                                let inspecting_sounds = self.client_state.follow_mut(client_state().inspecting_sound_sources());
                                let sound_source = map.get_sound_source(index);
                                let sound_source_path = state::prepare_sound_source_inspection(inspecting_sounds, sound_source);

                                self.interface.open_state_window(sound_source_path);
                            }
                            MarkerIdentifier::EffectSource(index) => {
                                let inspecting_effects = self.client_state.follow_mut(client_state().inspecting_effect_sources());
                                let effect_source = map.get_effect_source(index);
                                let effect_source_path = state::prepare_effect_source_inspection(inspecting_effects, effect_source);

                                self.interface.open_state_window(effect_source_path);
                            }
                            MarkerIdentifier::Particle(..) => {
                                // TODO:
                            }
                            MarkerIdentifier::Entity(index) => {
                                let entity_id = self
                                    .client_state
                                    .try_follow(client_state().entities().index(index as usize))
                                    .expect("entity should exist")
                                    .get_entity_id();

                                // This can technically still be `None`, violating the API but we handle this
                                // case in the state window.
                                let entity_path = client_state().entities().lookup(entity_id).manually_asserted();

                                self.interface.open_state_window(entity_path);
                            }
                            MarkerIdentifier::Shadow(..) => {
                                // TODO:
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleRenderOptionsWindow => match self.interface.is_window_with_class_open(WindowClass::RenderOptions) {
                    true => self.interface.close_window_with_class(WindowClass::RenderOptions),
                    false => self
                        .interface
                        .open_window(RenderOptionsWindow::new(client_state().render_options())),
                },
                #[cfg(feature = "debug")]
                InputEvent::OpenMapDataWindow => {
                    if let Some(map) = self.map.as_ref() {
                        let inspecting_maps = self.client_state.follow_mut(client_state().inspecting_maps());
                        let map_data_path = state::prepare_map_inspection(inspecting_maps, map.get_map_data());

                        self.interface.open_state_window(map_data_path);
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleClientStateInspectorWindow => {
                    match self.interface.is_window_with_class_open(WindowClass::ClientStateInspector) {
                        true => self.interface.close_window_with_class(WindowClass::ClientStateInspector),
                        false => self.interface.open_state_window_mut(client_state()),
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleMapsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Maps) {
                            true => self.interface.close_window_with_class(WindowClass::Maps),
                            false => self.interface.open_window(MapsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleCommandsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Commands) {
                            true => self.interface.close_window_with_class(WindowClass::Commands),
                            false => self.interface.open_window(CommandsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleThemeInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::ThemeInspector) {
                    true => self.interface.close_window_with_class(WindowClass::ThemeInspector),
                    false => self.interface.open_window(ThemeInspectorWindow::new(
                        client_state().theme_inspector_window(),
                        client_state().menu_theme(),
                        client_state().in_game_theme(),
                        client_state().world_theme(),
                    )),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleProfilerWindow => match self.interface.is_window_with_class_open(WindowClass::Profiler) {
                    true => self.interface.close_window_with_class(WindowClass::Profiler),
                    false => self.interface.open_window(ProfilerWindow::new(client_state().profiler_window())),
                },
                #[cfg(feature = "debug")]
                InputEvent::TogglePacketInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::PacketInspector) {
                    true => self.interface.close_window_with_class(WindowClass::PacketInspector),
                    false => self
                        .interface
                        .open_window(PacketInspectorWindow::new(client_state().packet_history())),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleCacheStatisticsWindow => match self.interface.is_window_with_class_open(WindowClass::CacheStatistics) {
                    true => self.interface.close_window_with_class(WindowClass::CacheStatistics),
                    false => self.interface.open_state_window(client_state().cache_statistics()),
                },
                #[cfg(feature = "debug")]
                InputEvent::CameraLookAround { offset } => self.debug_camera.look_around(offset),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveRight => self.debug_camera.move_right(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveUp => self.debug_camera.move_up(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                InputEvent::CameraDecelerate => self.debug_camera.decelerate(),
                #[cfg(feature = "debug")]
                InputEvent::InspectFrame { measurement } => self.interface.open_window(FrameInspectorWindow::new(measurement)),
            }
        }

        interface_has_focus
    }
}
