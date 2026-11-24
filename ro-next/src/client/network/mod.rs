//! The map-server event dispatcher.
//!
//! This module only routes: each arm matches the `NetworkEvent` variant and
//! calls the handler in the matching family module (`session`, `character`,
//! `world`, `skills`, `inventory`, `commerce`, `social`, `ui`). Handlers get
//! the whole event and destructure it, so the dispatcher never repeats field
//! types.
pub(crate) mod character;
pub(crate) mod commerce;
pub(crate) mod inventory;
pub(crate) mod session;
pub(crate) mod skills;
pub(crate) mod social;
pub(crate) mod ui;
pub(crate) mod world;

use crate::*;

impl Client {
    pub(crate) fn handle_network_events(&mut self, client_tick: ClientTick) {
        self.networking_system.get_events(&mut self.network_event_buffer);

        // Collect the events first so handlers are free to take `&mut self`
        // (the drain iterator borrows the event buffer).
        let network_events: Vec<NetworkEvent> = self.network_event_buffer.drain().collect();

        if !network_events.is_empty() {
            // Network events change UI state (chat, inventory, stats, ...).
            self.interface.invalidate_layouts();
        }

        for event in network_events {
            match event {
                NetworkEvent::LoginServerConnected { .. } => self.handle_login_server_connected(event, client_tick),
                NetworkEvent::LoginServerConnectionFailed { .. } => self.handle_login_server_connection_failed(event, client_tick),
                NetworkEvent::InternalError { .. } => self.handle_internal_error(event, client_tick),
                NetworkEvent::LoginServerDisconnected { .. } => self.handle_login_server_disconnected(event, client_tick),
                NetworkEvent::CharacterServerConnected { .. } => self.handle_character_server_connected(event, client_tick),
                NetworkEvent::CharacterServerConnectionFailed { .. } => self.handle_character_server_connection_failed(event, client_tick),
                NetworkEvent::CharacterServerDisconnected { .. } => self.handle_character_server_disconnected(event, client_tick),
                NetworkEvent::MapServerDisconnected { .. } => self.handle_map_server_disconnected(event, client_tick),
                NetworkEvent::InitialStats { .. } => self.handle_initial_stats(event, client_tick),
                NetworkEvent::ResurrectPlayer { .. } => self.handle_resurrect_player(event, client_tick),
                NetworkEvent::PlayerStandUp { .. } => self.handle_player_stand_up(event, client_tick),
                NetworkEvent::PlayerSitDown { .. } => self.handle_player_sit_down(event, client_tick),
                NetworkEvent::SkillCast { .. } => self.handle_skill_cast(event, client_tick),
                NetworkEvent::SkillNoDamage { .. } => self.handle_skill_no_damage(event, client_tick),
                NetworkEvent::SkillCastCancelled { .. } => self.handle_skill_cast_cancelled(event, client_tick),
                NetworkEvent::SkillGroundCast { .. } => self.handle_skill_ground_cast(event, client_tick),
                NetworkEvent::PlaySound { .. } => self.handle_play_sound(event, client_tick),
                NetworkEvent::DisplayEmotion { .. } => self.handle_display_emotion(event, client_tick),
                NetworkEvent::AccountId { .. } => self.handle_account_id(event, client_tick),
                NetworkEvent::CharacterList { .. } => self.handle_character_list(event, client_tick),
                NetworkEvent::CharacterSelectionFailed { .. } => self.handle_character_selection_failed(event, client_tick),
                NetworkEvent::CharacterDeleted => self.handle_character_deleted(event, client_tick),
                NetworkEvent::CharacterDeletionFailed { .. } => self.handle_character_deletion_failed(event, client_tick),
                NetworkEvent::CharacterSelected { .. } => self.handle_character_selected(event, client_tick),
                NetworkEvent::CharacterCreated { .. } => self.handle_character_created(event, client_tick),
                NetworkEvent::CharacterCreationFailed { .. } => self.handle_character_creation_failed(event, client_tick),
                NetworkEvent::CharacterSlotSwitched => self.handle_character_slot_switched(event, client_tick),
                NetworkEvent::CharacterSlotSwitchFailed => self.handle_character_slot_switch_failed(event, client_tick),
                NetworkEvent::AddEntity { .. } => self.handle_add_entity(event, client_tick),
                NetworkEvent::RemoveEntity { .. } => self.handle_remove_entity(event, client_tick),
                NetworkEvent::AddGroundItem { .. } => self.handle_add_ground_item(event, client_tick),
                NetworkEvent::RemoveGroundItem { .. } => self.handle_remove_ground_item(event, client_tick),
                NetworkEvent::EntityMove { .. } => self.handle_entity_move(event, client_tick),
                NetworkEvent::PlayerMove { .. } => self.handle_player_move(event, client_tick),
                NetworkEvent::ChangeMap { .. } => self.handle_change_map(event, client_tick),
                NetworkEvent::UpdateClientTick { .. } => self.handle_update_client_tick(event, client_tick),
                NetworkEvent::StatusEffect { .. } => self.handle_status_effect(event, client_tick),
                NetworkEvent::ChatMessage { .. } => self.handle_chat_message(event, client_tick),
                NetworkEvent::OverheadChatMessage { .. } => self.handle_overhead_chat_message(event, client_tick),
                NetworkEvent::WhisperReceived { .. } => self.handle_whisper_received(event, client_tick),
                NetworkEvent::GuildChat { .. } => self.handle_guild_chat(event, client_tick),
                NetworkEvent::ItemPickedUp { .. } => self.handle_item_picked_up(event, client_tick),
                NetworkEvent::QuestBanner { .. } => self.handle_quest_banner(event, client_tick),
                NetworkEvent::ServerAnnouncement { .. } => self.handle_server_announcement(event, client_tick),
                NetworkEvent::DisplayImage { .. } => self.handle_display_image(event, client_tick),
                NetworkEvent::OpenBook { .. } => self.handle_open_book(event, client_tick),
                NetworkEvent::PincodeState { .. } => self.handle_pincode_state(event, client_tick),
                NetworkEvent::ChatRoomCreated { .. } => self.handle_chat_room_created(event, client_tick),
                NetworkEvent::ChatRoomAnnounced { .. } => self.handle_chat_room_announced(event, client_tick),
                NetworkEvent::ChatRoomDestroyed { .. } => self.handle_chat_room_destroyed(event, client_tick),
                NetworkEvent::ChatRoomEnterRefused { .. } => self.handle_chat_room_enter_refused(event, client_tick),
                NetworkEvent::ChatRoomEntered { .. } => self.handle_chat_room_entered(event, client_tick),
                NetworkEvent::ChatRoomMemberJoined { .. } => self.handle_chat_room_member_joined(event, client_tick),
                NetworkEvent::ChatRoomMemberLeft { .. } => self.handle_chat_room_member_left(event, client_tick),
                NetworkEvent::ChatRoomChanged { .. } => self.handle_chat_room_changed(event, client_tick),
                NetworkEvent::ChatRoomRoleChanged { .. } => self.handle_chat_room_role_changed(event, client_tick),
                NetworkEvent::UpdateEntityDetails { .. } => self.handle_update_entity_details(event, client_tick),
                NetworkEvent::GuildEmblemImage { .. } => self.handle_guild_emblem_image(event, client_tick),
                NetworkEvent::GuildMenuInterface { .. } => self.handle_guild_menu_interface(event, client_tick),
                NetworkEvent::GuildInfo { .. } => self.handle_guild_info(event, client_tick),
                NetworkEvent::GuildMembers { .. } => self.handle_guild_members(event, client_tick),
                NetworkEvent::GuildPositionNames { .. } => self.handle_guild_position_names(event, client_tick),
                NetworkEvent::GuildPositionRights { .. } => self.handle_guild_position_rights(event, client_tick),
                NetworkEvent::GuildPositionChanged { .. } => self.handle_guild_position_changed(event, client_tick),
                NetworkEvent::GuildMemberPositionChanged { .. } => self.handle_guild_member_position_changed(event, client_tick),
                NetworkEvent::GuildNotice { .. } => self.handle_guild_notice(event, client_tick),
                NetworkEvent::GuildAlliances { .. } => self.handle_guild_alliances(event, client_tick),
                NetworkEvent::GuildMemberName { .. } => self.handle_guild_member_name(event, client_tick),
                NetworkEvent::GuildMemberRemoved { .. } => self.handle_guild_member_removed(event, client_tick),
                NetworkEvent::GuildInviteResult { .. } => self.handle_guild_invite_result(event, client_tick),
                NetworkEvent::DamageEffect { .. } => self.handle_damage_effect(event, client_tick),
                NetworkEvent::EntityPickUpItem { .. } => self.handle_entity_pick_up_item(event, client_tick),
                NetworkEvent::HealEffect { .. } => self.handle_heal_effect(event, client_tick),
                NetworkEvent::ChangeDirection { .. } => self.handle_change_direction(event, client_tick),
                NetworkEvent::StateChange { .. } => self.handle_state_change(event, client_tick),
                NetworkEvent::UpdateEntityHealth { .. } => self.handle_update_entity_health(event, client_tick),
                NetworkEvent::UpdateStat { .. } => self.handle_update_stat(event, client_tick),
                NetworkEvent::OpenEditDialog { .. } => self.handle_open_edit_dialog(event, client_tick),
                NetworkEvent::OpenDialog { .. } => self.handle_open_dialog(event, client_tick),
                NetworkEvent::AddNextButton { .. } => self.handle_add_next_button(event, client_tick),
                NetworkEvent::AddCloseButton { .. } => self.handle_add_close_button(event, client_tick),
                NetworkEvent::AddChoiceButtons { .. } => self.handle_add_choice_buttons(event, client_tick),
                NetworkEvent::WarpPointList { .. } => self.handle_warp_point_list(event, client_tick),
                NetworkEvent::AddQuestEffect { .. } => self.handle_add_quest_effect(event, client_tick),
                NetworkEvent::RemoveQuestEffect { .. } => self.handle_remove_quest_effect(event, client_tick),
                NetworkEvent::SetInventory { .. } => self.handle_set_inventory(event, client_tick),
                // --- Item service: refine / repair / identify / card insert ----
                // rAthena pushes the selectable list after the matching skill
                // (or Magnifier) was used; the window answers it with the
                // matching CZ request. The opcode/layout references live on the
                // packet structs in `ragnarok-packets`.
                NetworkEvent::WeaponRefineList { .. } => self.handle_weapon_refine_list(event, client_tick),
                NetworkEvent::WeaponRefineResult { .. } => self.handle_weapon_refine_result(event, client_tick),
                NetworkEvent::ItemRefiningResult { .. } => self.handle_item_refining_result(event, client_tick),
                NetworkEvent::RepairItemList { .. } => self.handle_repair_item_list(event, client_tick),
                NetworkEvent::RepairItemResult { .. } => self.handle_repair_item_result(event, client_tick),
                NetworkEvent::IdentifyItemList { .. } => self.handle_identify_item_list(event, client_tick),
                NetworkEvent::IdentifyItemResult { .. } => self.handle_identify_item_result(event, client_tick),
                NetworkEvent::CardCompositionList { .. } => self.handle_card_composition_list(event, client_tick),
                NetworkEvent::CardCompositionResult { .. } => self.handle_card_composition_result(event, client_tick),
                NetworkEvent::CardSlotUpdated { .. } => self.handle_card_slot_updated(event, client_tick),
                NetworkEvent::RefiningUiOpened => self.handle_refining_ui_opened(event, client_tick),
                NetworkEvent::RefiningMaterialList { .. } => self.handle_refining_material_list(event, client_tick),
                NetworkEvent::EquipmentSwitchItems { .. } => self.handle_equipment_switch_items(event, client_tick),
                NetworkEvent::SetStorage { .. } => self.handle_set_storage(event, client_tick),
                NetworkEvent::SetStorageAmount { .. } => self.handle_set_storage_amount(event, client_tick),
                NetworkEvent::StorageItemAdded { .. } => self.handle_storage_item_added(event, client_tick),
                NetworkEvent::StorageItemRemoved { .. } => self.handle_storage_item_removed(event, client_tick),
                NetworkEvent::StorageClosed => self.handle_storage_closed(event, client_tick),
                NetworkEvent::OpenUserInterface { .. } => self.handle_open_user_interface(event, client_tick),
                NetworkEvent::BankOpened => self.handle_bank_opened(event, client_tick),
                NetworkEvent::BankBalance { .. } => self.handle_bank_balance(event, client_tick),
                NetworkEvent::BankDepositResult { .. } => self.handle_bank_deposit_result(event, client_tick),
                NetworkEvent::BankWithdrawResult { .. } => self.handle_bank_withdraw_result(event, client_tick),
                NetworkEvent::BankClosed => self.handle_bank_closed(event, client_tick),
                NetworkEvent::SetCart { .. } => self.handle_set_cart(event, client_tick),
                NetworkEvent::CartItemAdded { .. } => self.handle_cart_item_added(event, client_tick),
                NetworkEvent::CartItemRemoved { .. } => self.handle_cart_item_removed(event, client_tick),
                NetworkEvent::CartClosed => self.handle_cart_closed(event, client_tick),
                NetworkEvent::CartAddItemFailed { .. } => self.handle_cart_add_item_failed(event, client_tick),
                NetworkEvent::StoreEntry { .. } => self.handle_store_entry(event, client_tick),
                NetworkEvent::StoreEntryRemoved { .. } => self.handle_store_entry_removed(event, client_tick),
                NetworkEvent::VendingShopList { .. } => self.handle_vending_shop_list(event, client_tick),
                NetworkEvent::VendingPurchaseResult { .. } => self.handle_vending_purchase_result(event, client_tick),
                NetworkEvent::IventoryItemAdded { .. } => self.handle_iventory_item_added(event, client_tick),
                NetworkEvent::ItemObtained { .. } => self.handle_item_obtained(event, client_tick),
                NetworkEvent::InventoryItemRemoved { .. } => self.handle_inventory_item_removed(event, client_tick),
                NetworkEvent::SkillTree { .. } => self.handle_skill_tree(event, client_tick),
                NetworkEvent::UpdateSkillInfo { .. } => self.handle_update_skill_info(event, client_tick),
                NetworkEvent::AddSkill { .. } => self.handle_add_skill(event, client_tick),
                NetworkEvent::ItemUsed { .. } => self.handle_item_used(event, client_tick),
                NetworkEvent::InventoryItemAmountChanged { .. } => self.handle_inventory_item_amount_changed(event, client_tick),
                NetworkEvent::UpdateEquippedPosition { .. } => self.handle_update_equipped_position(event, client_tick),
                NetworkEvent::ChangeJob { .. } => self.handle_change_job(event, client_tick),
                NetworkEvent::ChangeHair { .. } => self.handle_change_hair(event, client_tick),
                NetworkEvent::AppearanceChanged { .. } => self.handle_appearance_changed(event, client_tick),
                NetworkEvent::AutoRunSkill { .. } => self.handle_auto_run_skill(event, client_tick),
                NetworkEvent::LocalAppearanceChanged { .. } => self.handle_local_appearance_changed(event, client_tick),
                NetworkEvent::LoggedOut => self.handle_logged_out(event, client_tick),
                NetworkEvent::FriendRequest { .. } => self.handle_friend_request(event, client_tick),
                // --- Rodex mail (2019) -----------------------------------------
                NetworkEvent::MailList { .. } => self.handle_mail_list(event, client_tick),
                NetworkEvent::MailRead { .. } => self.handle_mail_read(event, client_tick),
                NetworkEvent::MailAttachmentClaimed { .. } => self.handle_mail_attachment_claimed(event, client_tick),
                NetworkEvent::MailDeleted { .. } => self.handle_mail_deleted(event, client_tick),
                NetworkEvent::MailUnreadStatus { .. } => self.handle_mail_unread_status(event, client_tick),
                NetworkEvent::MailboxWindow { .. } => self.handle_mailbox_window(event, client_tick),
                // --- Party events (2019) -------------------------------------------
                NetworkEvent::PartyOption { .. } => self.handle_party_option(event, client_tick),
                NetworkEvent::PartyList { .. } => self.handle_party_list(event, client_tick),
                NetworkEvent::PartyMemberAdded { .. } => self.handle_party_member_added(event, client_tick),
                NetworkEvent::PartyMemberRemoved { .. } => self.handle_party_member_removed(event, client_tick),
                NetworkEvent::PartyMemberHealth { .. } => self.handle_party_member_health(event, client_tick),
                NetworkEvent::PartyMemberPosition { .. } => self.handle_party_member_position(event, client_tick),
                NetworkEvent::PartyMemberJobLevel { .. } => self.handle_party_member_job_level(event, client_tick),
                NetworkEvent::PartyMemberAlive { .. } => self.handle_party_member_alive(event, client_tick),
                NetworkEvent::PartyInvite { .. } => self.handle_party_invite(event, client_tick),
                NetworkEvent::PartyInviteReply { .. } => self.handle_party_invite_reply(event, client_tick),
                NetworkEvent::PartyCreationResult { .. } => self.handle_party_creation_result(event, client_tick),
                NetworkEvent::PartyChatMessage { .. } => self.handle_party_chat_message(event, client_tick),
                NetworkEvent::PartyConfig { .. } => self.handle_party_config(event, client_tick),
                // --- Party booking / LFG (2019) ------------------------------------
                NetworkEvent::PartyBookingRegisterResult { .. } => self.handle_party_booking_register_result(event, client_tick),
                NetworkEvent::PartyBookingSearchResult { .. } => self.handle_party_booking_search_result(event, client_tick),
                NetworkEvent::PartyBookingDeleteResult { .. } => self.handle_party_booking_delete_result(event, client_tick),
                NetworkEvent::PartyBookingInserted { .. } => self.handle_party_booking_inserted(event, client_tick),
                NetworkEvent::PartyBookingUpdated { .. } => self.handle_party_booking_updated(event, client_tick),
                NetworkEvent::PartyBookingDeleted { .. } => self.handle_party_booking_deleted(event, client_tick),
                NetworkEvent::PartyBookingJoinRequest { .. } => self.handle_party_booking_join_request(event, client_tick),
                NetworkEvent::PartyBookingJoinResult { .. } => self.handle_party_booking_join_result(event, client_tick),
                // --- Clan (2019) ---------------------------------------------------
                NetworkEvent::ClanInfo { .. } => self.handle_clan_info(event, client_tick),
                NetworkEvent::ClanOnlineCount { .. } => self.handle_clan_online_count(event, client_tick),
                // --- Pet / homunculus / mercenary ---------------------------------
                NetworkEvent::PetProperty { .. } => self.handle_pet_property(event, client_tick),
                NetworkEvent::PetStateChanged { .. } => self.handle_pet_state_changed(event, client_tick),
                NetworkEvent::HomunculusProperty { .. } => self.handle_homunculus_property(event, client_tick),
                NetworkEvent::HomunculusParameterChanged { .. } => self.handle_homunculus_parameter_changed(event, client_tick),
                NetworkEvent::MercenaryInfo { .. } => self.handle_mercenary_info(event, client_tick),
                NetworkEvent::MercenaryParameterChanged { .. } => self.handle_mercenary_parameter_changed(event, client_tick),
                NetworkEvent::FriendRemoved { .. } => self.handle_friend_removed(event, client_tick),
                NetworkEvent::FriendAdded { .. } => self.handle_friend_added(event, client_tick),
                NetworkEvent::FriendOnlineState { .. } => self.handle_friend_online_state(event, client_tick),
                NetworkEvent::VisualEffect { .. } => self.handle_visual_effect(event, client_tick),
                NetworkEvent::AddSkillUnit { .. } => self.handle_add_skill_unit(event, client_tick),
                NetworkEvent::RemoveSkillUnit { .. } => self.handle_remove_skill_unit(event, client_tick),
                NetworkEvent::SetFriendList { .. } => self.handle_set_friend_list(event, client_tick),
                NetworkEvent::SetHotkeyData { .. } => self.handle_set_hotkey_data(event, client_tick),
                NetworkEvent::OpenShop { .. } => self.handle_open_shop(event, client_tick),
                NetworkEvent::AskBuyOrSell { .. } => self.handle_ask_buy_or_sell(event, client_tick),
                NetworkEvent::BuyingCompleted { .. } => self.handle_buying_completed(event, client_tick),
                NetworkEvent::SellItemList { .. } => self.handle_sell_item_list(event, client_tick),
                NetworkEvent::SellingCompleted { .. } => self.handle_selling_completed(event, client_tick),
                NetworkEvent::AttackFailed { .. } => self.handle_attack_failed(event, client_tick),
                NetworkEvent::UpdateSkill { .. } => self.handle_update_skill(event, client_tick),
                NetworkEvent::RemoveSkill { .. } => self.handle_remove_skill(event, client_tick),
                // --- Feedback for small/no-op packets (exp, skill fail, ammo, ban)
                NetworkEvent::GainedExperience { .. } => self.handle_gained_experience(event, client_tick),
                NetworkEvent::SkillUseFailed { .. } => self.handle_skill_use_failed(event, client_tick),
                NetworkEvent::StatUpFailed => self.handle_stat_up_failed(event, client_tick),
                NetworkEvent::AmmunitionAction { .. } => self.handle_ammunition_action(event, client_tick),
                NetworkEvent::AmmunitionEquipped { .. } => self.handle_ammunition_equipped(event, client_tick),
                NetworkEvent::UserCount { .. } => self.handle_user_count(event, client_tick),
                NetworkEvent::MapServerBan { .. } => self.handle_map_server_ban(event, client_tick),
                NetworkEvent::RefuseEnter { .. } => self.handle_refuse_enter(event, client_tick),
                NetworkEvent::SpiritOrbs { .. } => self.handle_spirit_orbs(event, client_tick),
                NetworkEvent::SkillCooldown { .. } => self.handle_skill_cooldown(event, client_tick),
                NetworkEvent::AttackRangeChanged { .. } => self.handle_attack_range_changed(event, client_tick),
                NetworkEvent::EntityStopped { .. } => self.handle_entity_stopped(event, client_tick),
                NetworkEvent::EntityHighJumped { .. } => self.handle_entity_high_jumped(event, client_tick),
                NetworkEvent::EntitySnapped { .. } => self.handle_entity_snapped(event, client_tick),
                NetworkEvent::TradeItemOffered { .. } => self.handle_trade_item_offered(event, client_tick),
                NetworkEvent::TradeAddResult { .. } => self.handle_trade_add_result(event, client_tick),
                NetworkEvent::SpecialEffect { .. } => self.handle_special_effect(event, client_tick),
                NetworkEvent::TradeRequest { .. } => self.handle_trade_request(event, client_tick),
                NetworkEvent::TradeAck { .. } => self.handle_trade_ack(event, client_tick),
                NetworkEvent::TradeCancelled => self.handle_trade_cancelled(event, client_tick),
                NetworkEvent::TradeConcluded { .. } => self.handle_trade_concluded(event, client_tick),
                NetworkEvent::TradeExecuted { .. } => self.handle_trade_executed(event, client_tick),
                NetworkEvent::QuestList { .. } => self.handle_quest_list(event, client_tick),
                NetworkEvent::QuestActive { .. } => self.handle_quest_active(event, client_tick),
                NetworkEvent::QuestRemoved { .. } => self.handle_quest_removed(event, client_tick),
                NetworkEvent::AchievementList { .. } => self.handle_achievement_list(event, client_tick),
                NetworkEvent::AchievementUpdate { .. } => self.handle_achievement_update(event, client_tick),
                NetworkEvent::AchievementRewardResult { .. } => self.handle_achievement_reward_result(event, client_tick),
                NetworkEvent::MonsterInfo { .. } => self.handle_monster_info(event, client_tick),
                NetworkEvent::OpenStore { .. } => self.handle_open_store(event, client_tick),
                NetworkEvent::MinimapMarker { .. } => self.handle_minimap_marker(event, client_tick),
                NetworkEvent::MapProperties { .. } => self.handle_map_properties(event, client_tick),
                NetworkEvent::CartInfo { .. } => self.handle_cart_info(event, client_tick),
                NetworkEvent::MapServerAuthFailed { .. } => self.handle_map_server_auth_failed(event, client_tick),
                NetworkEvent::ShowEquipmentWindow { .. } => self.handle_show_equipment_window(event, client_tick),
                NetworkEvent::CriticalWeight { .. } => self.handle_critical_weight(event, client_tick),
                NetworkEvent::MapCellChanged { .. } => self.handle_map_cell_changed(event, client_tick),
                NetworkEvent::MessageString { .. } => self.handle_message_string(event, client_tick),
            }
        }

        // Apply stat updates that were received before the local player entity
        // existed (e.g. the initial stat burst with the experience needed for the
        // next level). This also re-applies buffered updates after a map change or
        // respawn.
        self.apply_pending_stat_updates();

        self.chat_bubbles.retain_alive(client_tick);
        self.skill_name_texts.retain_alive(client_tick);
    }
}
