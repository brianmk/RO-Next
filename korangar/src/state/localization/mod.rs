#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, Timer, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
#[cfg(feature = "debug")]
use korangar_interface::element::Element;
use korangar_interface::element::StateElement;
use korangar_loaders::FileLoader;
#[cfg(feature = "debug")]
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

mod texts;

use rust_state::Path;
pub use texts::{DropdownTexts, PaperDollTexts, UiTexts, UiTextsPathExt, dropdown_keys, template_text};

#[cfg(feature = "debug")]
use crate::input::InputEvent;
use crate::loaders::GameFileLoader;
use crate::state::ClientState;

/// Supported languages.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum Language {
    English,
    German,
}

impl Language {
    /// Convert from our supported languages to a locale code.
    fn to_locale_code(self) -> &'static str {
        match self {
            Self::English => "en-US",
            Self::German => "de-DE",
        }
    }
}

impl DropDownItem<Language> for Language {
    fn text(&self) -> &str {
        match self {
            Self::English => "English",
            Self::German => "Deutsch",
        }
    }

    fn value(&self) -> Language {
        *self
    }
}

/// Controls for reloading and saving the localization from the state
/// inspector.
///
/// It implements [`StateElement`] so it shows up in the state inspector,
/// allowing editing and saving language files from the client.
#[cfg(feature = "debug")]
#[derive(Default)]
pub struct LocalizationControls;

#[cfg(feature = "debug")]
impl StateElement<ClientState> for LocalizationControls {
    type LayoutInfo = impl std::any::Any;
    type LayoutInfoMut = impl std::any::Any;
    type Return<P>
        = impl Element<ClientState, LayoutInfo = Self::LayoutInfo>
    where
        P: rust_state::Path<ClientState, Self>;
    type ReturnMut<P>
        = impl Element<ClientState, LayoutInfo = Self::LayoutInfoMut>
    where
        P: rust_state::Path<ClientState, Self>;

    fn to_element<P>(_: P, name: String) -> Self::Return<P>
    where
        P: rust_state::Path<ClientState, Self>,
    {
        use korangar_interface::prelude::*;

        collapsible! {
            text: name,
            children: split! {
                gaps: theme().window().gaps(),
                children: (
                    button! {
                        text: "Reload",
                        tooltip: "Reload the language from disk",
                        event: InputEvent::ReloadLanguage,
                    },
                    button! {
                        text: "Save",
                        tooltip: "Save the language to disk",
                        event: InputEvent::SaveLanguage,
                    },
                ),
            },
        }
    }

    fn to_element_mut<P>(_: P, name: String) -> Self::ReturnMut<P>
    where
        P: rust_state::Path<ClientState, Self>,
    {
        use korangar_interface::prelude::*;

        collapsible! {
            text: name,
            children: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Reload",
                            tooltip: "Reload the language from disk",
                            event: InputEvent::ReloadLanguage,
                        },
                        button! {
                            text: "Save",
                            tooltip: "Save the language to disk",
                            event: InputEvent::SaveLanguage,
                        },
                    ),
                },
            ),
        }
    }
}

/// Localization for the client in form of a string lookup.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct Localization {
    /// Controls for reloading and saving the localization from the state
    /// inspector.
    #[cfg(feature = "debug")]
    #[serde(skip_deserializing, skip_serializing)]
    controls: LocalizationControls,
    log_in_window_title: String,
    select_service_text: String,
    account_data_text: String,
    username_text: String,
    password_text: String,
    remember_username_text: String,
    remember_password_text: String,
    log_in_button_text: String,
    log_in_button_tooltip: String,
    show_character_names_text: String,
    level_99_aura_text: String,
    show_chat_bubbles_text: String,
    chat_bubble_background_text: String,
    show_skill_names_text: String,
    minimap_window_title: String,
    minimap_button_text: String,
    minimap_loading_text: String,
    minimap_transparent_frame_text: String,
    minimap_transparent_frame_tooltip: String,
    big_map_window_title: String,
    menu_window_title: String,
    graphics_settings_button_text: String,
    game_settings_button_text: String,
    interface_settings_button_text: String,
    audio_settings_button_text: String,
    log_out_button_text: String,
    exit_button_text: String,
    character_overview_window_title: String,
    character_overview_compact_button_text: String,
    character_overview_compact_button_tooltip: String,
    character_overview_expand_button_text: String,
    character_overview_expand_button_tooltip: String,
    health_points_text: String,
    spell_points_text: String,
    experience_text: String,
    job_experience_text: String,
    name_text: String,
    base_level_text: String,
    job_level_text: String,
    inventory_button_text: String,
    equipment_button_text: String,
    stats_button_text: String,
    skill_tree_button_text: String,
    friend_list_button_text: String,
    menu_button_text: String,
    /// Label of the button that opens/closes the chat window.
    #[serde(default = "default_chat_button_text")]
    chat_button_text: String,
    chat_window_title: String,
    chat_text_box_message: String,
    chat_transparency_button_text: String,
    chat_transparency_button_tooltip: String,
    /// Tooltip of the round transparency toggle in the hotbar window.
    #[serde(default = "default_hotbar_transparency_button_tooltip")]
    hotbar_transparency_button_tooltip: String,
    audio_settings_window_title: String,
    mute_audio_on_focus_loss_button_text: String,
    mute_background_music_button_text: String,
    #[serde(default = "default_master_volume_text")]
    master_volume_text: String,
    #[serde(default = "default_sound_effect_volume_text")]
    sound_effect_volume_text: String,
    background_music_volume_text: String,
    create_character_window_title: String,
    character_name_text: String,
    create_character_button_text: String,
    create_character_button_tooltip: String,
    dialog_window_title: String,
    next_button_text: String,
    close_button_text: String,
    error_window_title: String,
    friend_list_window_title: String,
    friend_list_text_box_message: String,
    remove_button_text: String,
    hotbar_window_title: String,
    inventory_window_title: String,
    inventory_consumables_tab_text: String,
    inventory_equipments_tab_text: String,
    inventory_misc_tab_text: String,
    respawn_window_title: String,
    respawn_button_text: String,
    disconnect_button_text: String,
    server_selection_window_title: String,
    skill_tree_window_title: String,
    stats_window_title: String,
    game_settings_window_title: String,
    interface_settings_window_title: String,
    language_text: String,
    scaling_text: String,
    menu_theme_text: String,
    in_game_theme_text: String,
    world_theme_text: String,
    available_stat_points_text: String,
    strength_text: String,
    agility_text: String,
    vitality_text: String,
    intelligence_text: String,
    dexterity_text: String,
    luck_text: String,
    auto_attack_button_text: String,
    available_skill_points_text: String,
    reset_skill_points_button_text: String,
    cancel_skill_points_button_text: String,
    apply_skill_points_button_text: String,
    distribute_skill_points_button_text: String,
    #[serde(default = "default_buy_or_sell_window_title")]
    buy_or_sell_window_title: String,
    #[serde(default = "default_buy_window_title")]
    buy_window_title: String,
    #[serde(default = "default_sell_window_title")]
    sell_window_title: String,
    #[serde(default = "default_cart_window_title")]
    cart_window_title: String,
    #[serde(default = "default_storage_window_title")]
    storage_window_title: String,
    #[serde(default = "default_equipment_window_title")]
    equipment_window_title: String,
    #[serde(default = "default_select_character_window_title")]
    select_character_window_title: String,
    #[serde(default = "default_delete_character_window_title")]
    delete_character_window_title: String,
    #[serde(default = "default_party_window_title")]
    party_window_title: String,
    #[serde(default = "default_party_invitation_window_title")]
    party_invitation_window_title: String,
    #[serde(default = "default_friend_request_window_title")]
    friend_request_window_title: String,
    #[serde(default = "default_graphics_settings_window_title")]
    graphics_settings_window_title: String,
    #[serde(default = "default_item_info_window_title")]
    item_info_window_title: String,
    #[serde(default = "default_buy_button_text")]
    buy_button_text: String,
    #[serde(default = "default_sell_button_text")]
    sell_button_text: String,
    #[serde(default = "default_zeny_text")]
    zeny_text: String,
    #[serde(default = "default_cancel_button_text")]
    cancel_button_text: String,
    #[serde(default = "default_delete_button_text")]
    delete_button_text: String,
    #[serde(default = "default_switch_button_text")]
    switch_button_text: String,
    #[serde(default = "default_accept_button_text")]
    accept_button_text: String,
    #[serde(default = "default_reject_button_text")]
    reject_button_text: String,
    #[serde(default = "default_invite_button_text")]
    invite_button_text: String,
    #[serde(default = "default_create_party_button_text")]
    create_party_button_text: String,
    #[serde(default = "default_leave_party_button_text")]
    leave_party_button_text: String,
    #[serde(default = "default_switch_slots_text")]
    switch_slots_text: String,
    /// Shown on a character slot that is blocked by the anti-crash selection
    /// cooldown after a logout.
    #[serde(default = "default_please_wait_text")]
    please_wait_text: String,
    #[serde(default = "default_create_character_slot_text")]
    create_character_slot_text: String,
    #[serde(default = "default_map_text")]
    map_text: String,
    #[serde(default = "default_party_name_text")]
    party_name_text: String,
    #[serde(default = "default_invite_character_text")]
    invite_character_text: String,
    #[serde(default = "default_create_party_text")]
    create_party_text: String,
    #[serde(default = "default_account_email_text")]
    account_email_text: String,
    #[serde(default = "default_not_in_party_text")]
    not_in_party_text: String,
    /// Static texts of the windows, buttons and tooltips.
    ///
    /// Grouped into a nested struct so the language files can add a whole
    /// group with one serde default and the element tuple of this struct stays
    /// within the interface limit.
    #[serde(default = "default_ui_texts")]
    ui: UiTexts,
    /// Names and short labels of the paper doll equipment slots.
    #[serde(default = "default_paper_doll_texts")]
    paper_doll: PaperDollTexts,
    /// Labels of the settings drop down options.
    #[serde(default = "default_dropdown_texts")]
    dropdown: DropdownTexts,
    // ------------------------------------------------------------------
    // Client chat feedback (client-side validation / status messages).
    //
    // These are *not* protocol result codes: the result-code texts live in
    // `data\client\messages.ron` / `korangar-messages`. The templates below
    // use the same `{name}` placeholders as the result messages and are
    // rendered with `korangar_messages::render_template`.
    // ------------------------------------------------------------------
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_aura_style_set")]
    chat_aura_style_set: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_party_invite_ignored")]
    chat_party_invite_ignored: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_cannot_raise_stat")]
    chat_cannot_raise_stat: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_players_online")]
    chat_players_online: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_map_server_refused_login")]
    chat_map_server_refused_login: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_passive")]
    chat_skill_passive: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_aura_usage")]
    chat_aura_usage: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_aura_unknown")]
    chat_aura_unknown: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_party_usage")]
    chat_party_usage: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_hotbar_page")]
    chat_hotbar_page: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_hotkey_item_missing")]
    chat_hotkey_item_missing: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_not_learned")]
    chat_skill_not_learned: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_target_out_of_range")]
    chat_skill_target_out_of_range: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_target_gone")]
    chat_skill_target_gone: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_no_target")]
    chat_skill_no_target: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_cell_out_of_range")]
    chat_skill_cell_out_of_range: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_position_unknown")]
    chat_skill_position_unknown: String,
    // Chat templates are not interface labels: keep them out of the state
    // inspector (and out of the derived inspector element tuple).
    #[hidden_element]
    #[serde(default = "default_chat_skill_click_ground")]
    chat_skill_click_ground: String,
    #[hidden_element]
    #[serde(default = "default_chat_experience_gained")]
    chat_experience_gained: String,
    #[hidden_element]
    #[serde(default = "default_chat_experience_base")]
    chat_experience_base: String,
    #[hidden_element]
    #[serde(default = "default_chat_experience_job")]
    chat_experience_job: String,
    #[hidden_element]
    #[serde(default = "default_chat_experience_quest_base")]
    chat_experience_quest_base: String,
    #[hidden_element]
    #[serde(default = "default_chat_experience_quest_job")]
    chat_experience_quest_job: String,
    #[hidden_element]
    #[serde(default = "default_chat_item_obtained")]
    chat_item_obtained: String,
    #[hidden_element]
    #[serde(default = "default_chat_sell_failed")]
    chat_sell_failed: String,
    // ------------------------------------------------------------------
    // Error-window feedback (client-side failures that are shown in the
    // error window instead of the chat).
    // ------------------------------------------------------------------
    #[hidden_element]
    #[serde(default = "default_error_map_load_failed")]
    error_map_load_failed: String,
    #[hidden_element]
    #[serde(default = "default_error_session_expired")]
    error_session_expired: String,
    #[hidden_element]
    #[serde(default = "default_error_character_selection_missing_login_data")]
    error_character_selection_missing_login_data: String,
    #[hidden_element]
    #[serde(default = "default_error_character_selection_missing_slot")]
    error_character_selection_missing_slot: String,
    #[hidden_element]
    #[serde(default = "default_error_character_slot_switch_failed")]
    error_character_slot_switch_failed: String,
    #[hidden_element]
    #[serde(default = "default_error_map_server_auth_failed")]
    error_map_server_auth_failed: String,
    #[hidden_element]
    #[serde(default = "default_error_map_data_missing")]
    error_map_data_missing: String,
}

/// Default for language files that were written before the `ui` text group
/// existed.
fn default_ui_texts() -> UiTexts {
    UiTexts::default()
}

/// Default for language files that were written before the paper doll text
/// group existed.
fn default_paper_doll_texts() -> PaperDollTexts {
    PaperDollTexts::default()
}

/// Default for language files that were written before the drop down text
/// group existed.
fn default_dropdown_texts() -> DropdownTexts {
    DropdownTexts::default()
}

/// Default for language files that were written before the
/// `buy_or_sell_window_title` field existed.
fn default_buy_or_sell_window_title() -> String {
    "Buy or sell".to_owned()
}

/// Default for language files that were written before the `buy_window_title`
/// field existed.
fn default_buy_window_title() -> String {
    "Buy".to_owned()
}

/// Default for language files that were written before the `sell_window_title`
/// field existed.
fn default_sell_window_title() -> String {
    "Sell".to_owned()
}

/// Default for language files that were written before the `cart_window_title`
/// field existed.
fn default_cart_window_title() -> String {
    "Cart".to_owned()
}

/// Default for language files that were written before the
/// `storage_window_title` field existed.
fn default_storage_window_title() -> String {
    "Storage".to_owned()
}

/// Default for language files that were written before the
/// `equipment_window_title` field existed.
fn default_equipment_window_title() -> String {
    "Equipment".to_owned()
}

/// Default for language files that were written before the
/// `select_character_window_title` field existed.
fn default_select_character_window_title() -> String {
    "Select Character".to_owned()
}

/// Default for language files that were written before the
/// `delete_character_window_title` field existed.
fn default_delete_character_window_title() -> String {
    "Delete character".to_owned()
}

/// Default for language files that were written before the `party_window_title`
/// field existed.
fn default_party_window_title() -> String {
    "Party".to_owned()
}

/// Default for language files that were written before the
/// `party_invitation_window_title` field existed.
fn default_party_invitation_window_title() -> String {
    "Party invitation".to_owned()
}

/// Default for language files that were written before the
/// `friend_request_window_title` field existed.
fn default_friend_request_window_title() -> String {
    "Friend request".to_owned()
}

/// Default for language files that were written before the
/// `graphics_settings_window_title` field existed.
fn default_graphics_settings_window_title() -> String {
    "Graphics Settings".to_owned()
}

/// Default for language files that were written before the
/// `item_info_window_title` field existed.
fn default_item_info_window_title() -> String {
    "Item".to_owned()
}

/// Default for language files that were written before the `buy_button_text`
/// field existed.
fn default_buy_button_text() -> String {
    "Buy".to_owned()
}

/// Default for language files that were written before the `sell_button_text`
/// field existed.
fn default_sell_button_text() -> String {
    "Sell".to_owned()
}

/// Default for language files that were written before the `zeny_text` field
/// existed.
fn default_zeny_text() -> String {
    "Zeny".to_owned()
}

/// Default for language files that were written before the `cancel_button_text`
/// field existed.
fn default_cancel_button_text() -> String {
    "Cancel".to_owned()
}

/// Default for language files that were written before the `delete_button_text`
/// field existed.
fn default_delete_button_text() -> String {
    "Delete".to_owned()
}

/// Default for language files that were written before the `switch_button_text`
/// field existed.
fn default_switch_button_text() -> String {
    "Switch".to_owned()
}

/// Default for language files that were written before the `accept_button_text`
/// field existed.
fn default_accept_button_text() -> String {
    "Accept".to_owned()
}

/// Default for language files that were written before the `reject_button_text`
/// field existed.
fn default_reject_button_text() -> String {
    "Reject".to_owned()
}

/// Default for language files that were written before the `invite_button_text`
/// field existed.
fn default_invite_button_text() -> String {
    "Invite".to_owned()
}

/// Default for language files that were written before the
/// `create_party_button_text` field existed.
fn default_create_party_button_text() -> String {
    "Create party".to_owned()
}

/// Default for language files that were written before the
/// `leave_party_button_text` field existed.
fn default_leave_party_button_text() -> String {
    "Leave party".to_owned()
}

/// Default for language files that were written before the `switch_slots_text`
/// field existed.
fn default_switch_slots_text() -> String {
    "Switch slots".to_owned()
}

/// Default for language files that were written before the
/// `create_character_slot_text` field existed.
fn default_create_character_slot_text() -> String {
    "Create Character".to_owned()
}

/// Default for language files that were written before the
/// `please_wait_text` field existed.
fn default_please_wait_text() -> String {
    "Please wait…".to_owned()
}

/// Default for language files that were written before the `map_text` field
/// existed.
fn default_map_text() -> String {
    "Map".to_owned()
}

/// Default for language files that were written before the `party_name_text`
/// field existed.
fn default_party_name_text() -> String {
    "Party name".to_owned()
}

/// Default for language files that were written before the
/// `invite_character_text` field existed.
fn default_invite_character_text() -> String {
    "Invite a character".to_owned()
}

/// Default for language files that were written before the `create_party_text`
/// field existed.
fn default_create_party_text() -> String {
    "Create a party".to_owned()
}

/// Default for language files that were written before the `not_in_party_text`
/// field existed.
fn default_not_in_party_text() -> String {
    "You are not in a party.".to_owned()
}

/// Default for language files that were written before the `account_email_text`
/// field existed.
fn default_account_email_text() -> String {
    "Account e-mail".to_owned()
}

/// Default label of the chat window toggle button.
///
/// New fields are optional in the persisted language files, so a language file
/// that was written before the field existed still loads.
fn default_chat_button_text() -> String {
    "Chat".to_owned()
}

fn default_hotbar_transparency_button_tooltip() -> String {
    "Make the hotbar window background transparent".to_owned()
}

/// Default for language files that were written before the master volume
/// slider existed.
fn default_master_volume_text() -> String {
    "Master".to_owned()
}

/// Default for language files that were written before the sound effect volume
/// slider existed.
fn default_sound_effect_volume_text() -> String {
    "Sound effects".to_owned()
}

/// Default for language files that were written before the
/// `chat_aura_style_set` field existed.
fn default_chat_aura_style_set() -> String {
    "Aura style set to \"{style}\".".to_owned()
}

/// Default for language files that were written before the
/// `chat_party_invite_ignored` field existed.
fn default_chat_party_invite_ignored() -> String {
    "Ignoring party invitation from \"{name}\": you are already in a party.".to_owned()
}

/// Default for language files that were written before the
/// `chat_cannot_raise_stat` field existed.
fn default_chat_cannot_raise_stat() -> String {
    "You cannot raise that stat right now.".to_owned()
}

/// Default for language files that were written before the
/// `chat_players_online` field existed.
fn default_chat_players_online() -> String {
    "{count} players are online.".to_owned()
}

/// Default for language files that were written before the
/// `chat_map_server_refused_login` field existed.
fn default_chat_map_server_refused_login() -> String {
    "The map server refused the login (error {error}).".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_passive` field existed.
fn default_chat_skill_passive() -> String {
    "Skill {skill} is a passive skill and cannot be cast.".to_owned()
}

/// Default for language files that were written before the
/// `chat_aura_usage` field existed.
fn default_chat_aura_usage() -> String {
    "Usage: /aura <style>. Available styles: {styles}.".to_owned()
}

/// Default for language files that were written before the
/// `chat_aura_unknown` field existed.
fn default_chat_aura_unknown() -> String {
    "Unknown aura style \"{style}\". Available styles: {styles}.".to_owned()
}

/// Default for language files that were written before the
/// `chat_party_usage` field existed.
fn default_chat_party_usage() -> String {
    "Usage: /p <message>".to_owned()
}

/// Default for language files that were written before the
/// `chat_hotbar_page` field existed.
fn default_chat_hotbar_page() -> String {
    "Hotbar page {page}/{pages}.".to_owned()
}

/// Default for language files that were written before the
/// `chat_hotkey_item_missing` field existed.
fn default_chat_hotkey_item_missing() -> String {
    "The item of that hotkey is not in your inventory.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_not_learned` field existed.
fn default_chat_skill_not_learned() -> String {
    "You have not learned skill {skill}.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_target_out_of_range` field existed.
fn default_chat_skill_target_out_of_range() -> String {
    "Skill {skill}: the target is out of range.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_target_gone` field existed.
fn default_chat_skill_target_gone() -> String {
    "Skill {skill}: the target is no longer in view.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_no_target` field existed.
fn default_chat_skill_no_target() -> String {
    "Skill {skill}: select a target first.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_cell_out_of_range` field existed.
fn default_chat_skill_cell_out_of_range() -> String {
    "Skill {skill}: that cell is out of range.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_position_unknown` field existed.
fn default_chat_skill_position_unknown() -> String {
    "Skill {skill}: the player position is not known yet.".to_owned()
}

/// Default for language files that were written before the
/// `chat_skill_click_ground` field existed.
fn default_chat_skill_click_ground() -> String {
    "Skill {skill}: click a cell on the ground to cast it.".to_owned()
}

/// Default for language files that were written before the
/// `chat_experience_gained` field existed.
fn default_chat_experience_gained() -> String {
    "Gained {gains}.".to_owned()
}

/// Default for language files that were written before the
/// `chat_experience_base` field existed.
fn default_chat_experience_base() -> String {
    "base EXP".to_owned()
}

/// Default for language files that were written before the
/// `chat_experience_job` field existed.
fn default_chat_experience_job() -> String {
    "job EXP".to_owned()
}

/// Default for language files that were written before the
/// `chat_experience_quest_base` field existed.
fn default_chat_experience_quest_base() -> String {
    "base EXP (quest)".to_owned()
}

/// Default for language files that were written before the
/// `chat_experience_quest_job` field existed.
fn default_chat_experience_quest_job() -> String {
    "job EXP (quest)".to_owned()
}

/// Default for language files that were written before the
/// `chat_item_obtained` field existed.
fn default_chat_item_obtained() -> String {
    "You got {name} ({quantity}).".to_owned()
}

/// Default for language files that were written before the
/// `chat_sell_failed` field existed.
fn default_chat_sell_failed() -> String {
    "Failed to sell items".to_owned()
}

/// Default for language files that were written before the
/// `error_map_load_failed` field existed.
fn default_error_map_load_failed() -> String {
    "Failed to load the map '{map_name}'. Please try again.".to_owned()
}

/// Default for language files that were written before the
/// `error_session_expired` field existed.
fn default_error_session_expired() -> String {
    "Your session expired. Please log in again.".to_owned()
}

/// Default for language files that were written before the
/// `error_character_selection_missing_login_data` field existed.
fn default_error_character_selection_missing_login_data() -> String {
    "Character selection failed: missing login data.".to_owned()
}

/// Default for language files that were written before the
/// `error_character_selection_missing_slot` field existed.
fn default_error_character_selection_missing_slot() -> String {
    "Character selection failed: the server did not send that character slot.".to_owned()
}

/// Default for language files that were written before the
/// `error_character_slot_switch_failed` field existed.
fn default_error_character_slot_switch_failed() -> String {
    "Failed to switch character slots".to_owned()
}

/// Default for language files that were written before the
/// `error_map_server_auth_failed` field existed.
fn default_error_map_server_auth_failed() -> String {
    "The map server rejected the connection (reason {reason}). Please try again.".to_owned()
}

/// Default for language files that were written before the
/// `error_map_data_missing` field existed.
fn default_error_map_data_missing() -> String {
    "Map data missing: `{map_name}`\nThis map is not in the game archives. Please log out and choose another character.".to_owned()
}

impl Localization {
    /// Localized labels of the aggregated experience-gain chat line, in the
    /// order base / job / quest base / quest job.
    pub(crate) fn experience_labels(&self) -> [&str; 4] {
        [
            self.chat_experience_base.as_str(),
            self.chat_experience_job.as_str(),
            self.chat_experience_quest_base.as_str(),
            self.chat_experience_quest_job.as_str(),
        ]
    }
}

/// Path to the whole [`Localization`] of the client state.
///
/// This is a `'static` path value on purpose: [`rust_state::State::get`] ties
/// the lifetime of the returned reference to the lifetime of the path value,
/// so borrowing localized strings out of the state needs a path that outlives
/// the borrow.
#[derive(Clone, Copy)]
pub struct LocalizationPath;

impl rust_state::Selector<ClientState, Localization> for LocalizationPath {
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a Localization> {
        self.follow(state)
    }
}

impl Path<ClientState, Localization> for LocalizationPath {
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a Localization> {
        Some(&state.localization)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut Localization> {
        Some(&mut state.localization)
    }
}

/// Static instance of [`LocalizationPath`].
pub const LOCALIZATION_PATH: LocalizationPath = LocalizationPath;

impl Localization {
    /// Resolve the localized text of a drop down option key.
    ///
    /// Used by [`korangar_interface::application::Application::localized_text`]
    /// when the interface renders a drop down whose items carry a
    /// [`dropdown_keys`] value.
    pub fn dropdown_text(&self, key: &str) -> Option<&str> {
        self.dropdown.text(key)
    }

    /// The static window/button/tooltip texts.
    #[cfg(test)]
    pub fn ui_texts(&self) -> &UiTexts {
        &self.ui
    }

    /// The paper doll equipment slot texts.
    pub fn paper_doll_texts(&self) -> &PaperDollTexts {
        &self.paper_doll
    }

    /// Save the localization to a file based on the provided language.
    // TODO: Currently this will just save to the file system but we might want to
    // save using the `GameFileLoader` instead.
    #[cfg(feature = "debug")]
    pub fn save_language(&self, language: Language) {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Save language");

        let locale_code = language.to_locale_code();
        let file_name = format!("archive/data/languages/{locale_code}.ron");

        #[cfg(feature = "debug")]
        print_debug!("saving to file {}", file_name.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(&file_name, data) {
            #[cfg(feature = "debug")]
            print_debug!("failed to save language to {}: {:?}", file_name.magenta(), _error.red());
        }

        #[cfg(feature = "debug")]
        timer.stop();
    }

    /// Load the localization from a file based on the provided language.
    pub fn load_language(game_file_loader: &GameFileLoader, language: Language) -> Self {
        #[cfg(feature = "debug")]
        let timer = Timer::new("Load language");

        let locale_code = language.to_locale_code();
        let file_name = format!("data\\languages\\{locale_code}.ron");

        #[cfg(feature = "debug")]
        print_debug!("loading from file {}", file_name.magenta());

        let bytes = game_file_loader.get(&file_name).expect("language files should be present");
        let localization = ron::de::from_bytes(&bytes).expect("language files should be valid");

        #[cfg(feature = "debug")]
        timer.stop();

        localization
    }
}

#[cfg(test)]
mod languages {
    use std::collections::BTreeSet;

    use crate::state::localization::{Language, Localization};

    /// Collect the field names of a RON struct file.
    ///
    /// The language files are plain RON structs with one `field: "value",`
    /// entry per line, so a small line scanner is enough and keeps the parity
    /// test independent of the (private) struct layout.
    fn field_names(file_content: &str) -> BTreeSet<String> {
        file_content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let (name, _) = line.split_once(':')?;
                let name = name.trim();
                let is_field_name = !name.is_empty()
                    && name
                        .chars()
                        .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_');
                is_field_name.then(|| name.to_owned())
            })
            .collect()
    }

    fn language_file(language: Language) -> String {
        let file_name = format!("archive/data/languages/{}.ron", language.to_locale_code());
        std::fs::read_to_string(file_name).expect("language file should exist")
    }

    /// Every key of the English file must also exist in the German file,
    /// otherwise German players see the English fallback of the serde default.
    #[test]
    fn language_files_have_the_same_keys() {
        let english = field_names(&language_file(Language::English));
        let german = field_names(&language_file(Language::German));

        let missing_in_german: Vec<_> = english.difference(&german).cloned().collect();
        let missing_in_english: Vec<_> = german.difference(&english).cloned().collect();

        assert!(missing_in_german.is_empty(), "de-DE.ron is missing keys: {missing_in_german:?}");
        assert!(
            missing_in_english.is_empty(),
            "en-US.ron is missing keys: {missing_in_english:?}"
        );
    }

    /// Language files written before a field was added must still load: the new
    /// field is `#[serde(default = "...")]` and falls back to English.
    #[test]
    fn old_language_files_without_new_fields_still_load() {
        let english = language_file(Language::English);
        let old_file: String = english
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.starts_with("buy_window_title:") && !line.starts_with("zeny_text:") && !line.starts_with("account_email_text:")
            })
            .collect::<Vec<_>>()
            .join("\n");

        let localization: Localization = ron::de::from_str(&old_file).expect("old language file must still load");
        // The fields fall back to the English defaults instead of failing to load.
        let serialized = ron::ser::to_string(&localization).expect("localization must serialize");
        assert!(
            serialized.contains("buy_window_title:\"Buy\""),
            "default buy window title is missing"
        );
        assert!(serialized.contains("zeny_text:\"Zeny\""), "default zeny text is missing");
    }

    /// Remove one nested text group from a language file, for the back-compat
    /// test of files that were written before the group existed.
    fn without_group(file_content: &str, group: &str) -> String {
        let start_marker = format!("{group}: (");
        let mut lines = file_content.lines();
        let mut kept = Vec::new();
        let mut skipping = false;
        for line in lines.by_ref() {
            if skipping {
                if line.trim() == ")," {
                    skipping = false;
                }
                continue;
            }
            if line.trim_start().starts_with(&start_marker) {
                // A group starts with `<group>: (` and ends with `),`.
                skipping = true;
                continue;
            }
            kept.push(line);
        }
        assert!(!skipping, "group {group} is not closed");
        kept.join("\n")
    }

    /// Language files written before the grouped UI texts existed must load and
    /// get the English defaults for the whole group.
    #[test]
    fn language_files_without_new_text_groups_still_load() {
        let english = language_file(Language::English);
        let old_file = without_group(&without_group(&without_group(&english, "ui"), "paper_doll"), "dropdown");
        assert!(!old_file.contains("context_menu_whisper:"), "the ui group was not removed");
        assert!(!old_file.contains("head_top_name:"), "the paper doll group was not removed");
        assert!(!old_file.contains("resolution_desktop:"), "the dropdown group was not removed");

        let localization: Localization = ron::de::from_str(&old_file).expect("old language file without the text groups must loads");
        assert_eq!(
            localization.ui_texts().context_menu_whisper,
            "Whisper",
            "the ui text group falls back to the English defaults"
        );
        assert_eq!(
            localization
                .paper_doll_texts()
                .slot_label(ragnarok_packets::EquipPosition::HEAD_TOP),
            "TOP",
            "the paper doll group falls back to the English defaults"
        );
        assert_eq!(
            localization.dropdown_text("resolution_desktop"),
            Some("Desktop (maximized)"),
            "the drop down group falls back to the English defaults"
        );
    }

    #[test]
    fn language_files_are_valid() {
        // Please extend this when adding a language.
        let languages = [Language::English, Language::German];

        for language in languages {
            // Used match here so the test fails if somebody forgets to adjust the test
            // after adding a language.
            match language {
                Language::English | Language::German => {
                    let locale_code = language.to_locale_code();
                    let file_name = format!("archive/data/languages/{locale_code}.ron");
                    let file_content = std::fs::read_to_string(file_name).expect("language file should exist");
                    let _: Localization = ron::de::from_str(&file_content).expect("language file should be valid");
                }
            }
        }
    }
}
