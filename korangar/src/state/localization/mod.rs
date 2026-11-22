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

#[cfg(feature = "debug")]
use super::ClientState;
#[cfg(feature = "debug")]
use crate::input::InputEvent;
use crate::loaders::GameFileLoader;

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

impl Localization {
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
