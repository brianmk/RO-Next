//! Localized texts of the interface.
//!
//! The interface texts are grouped into three nested structs instead of being
//! flattened into [`super::Localization`]:
//!
//! * [`UiTexts`] holds the static window/button/tooltip texts of the windows
//!   that used to hardcode them,
//! * [`PaperDollTexts`] holds the equipment slot names and short labels,
//! * [`DropdownTexts`] holds the option labels of the settings drop downs. A
//!   drop down item only stores a stable key (`dropdown_keys::*`), the text is
//!   looked up through [`super::Localization::dropdown_text`] at render time.
//!
//! Grouping them keeps the top level [`super::Localization`] below the element
//! tuple limit of the interface and lets a language file add a whole group as
//! one `serde(default)` block, so language files written before the group
//! existed still load.

use korangar_interface::element::StateElement;
use korangar_interface::selector_helpers::ComputedSelector;
use ragnarok_packets::EquipPosition;
use rust_state::{RustState, Selector};
use serde::{Deserialize, Serialize};

use crate::state::ClientState;

/// Stable keys of the localizable drop down option labels.
///
/// A drop down item returns one of these keys from `DropDownItem::text_key`;
/// the text is resolved from [`DropdownTexts`] when the option is rendered.
pub mod dropdown_keys {
    pub const RESOLUTION_DESKTOP: &str = "resolution_desktop";
    pub const RESOLUTION_1280X720: &str = "resolution_1280x720";
    pub const RESOLUTION_1366X768: &str = "resolution_1366x768";
    pub const RESOLUTION_1600X900: &str = "resolution_1600x900";
    pub const RESOLUTION_1920X1080: &str = "resolution_1920x1080";
    pub const RESOLUTION_2560X1440: &str = "resolution_2560x1440";
    pub const RESOLUTION_3840X2160: &str = "resolution_3840x2160";
    pub const RESOLUTION_CUSTOM: &str = "resolution_custom";
    pub const FPS_UNLIMITED: &str = "fps_unlimited";
    pub const FPS_15: &str = "fps_15";
    pub const FPS_24: &str = "fps_24";
    pub const FPS_30: &str = "fps_30";
    pub const FPS_45: &str = "fps_45";
    pub const FPS_60: &str = "fps_60";
    pub const FPS_75: &str = "fps_75";
    pub const FPS_90: &str = "fps_90";
    pub const FPS_120: &str = "fps_120";
    pub const FPS_144: &str = "fps_144";
    pub const FPS_240: &str = "fps_240";
    pub const FPS_CUSTOM: &str = "fps_custom";
    pub const SAMPLER_NEAREST: &str = "sampler_nearest";
    pub const SAMPLER_LINEAR: &str = "sampler_linear";
    pub const SAMPLER_ANISOTROPIC_4: &str = "sampler_anisotropic_4";
    pub const SAMPLER_ANISOTROPIC_8: &str = "sampler_anisotropic_8";
    pub const SAMPLER_ANISOTROPIC_16: &str = "sampler_anisotropic_16";
    pub const SHADOW_RESOLUTION_NORMAL: &str = "shadow_resolution_normal";
    pub const SHADOW_RESOLUTION_ULTRA: &str = "shadow_resolution_ultra";
    pub const SHADOW_RESOLUTION_INSANE: &str = "shadow_resolution_insane";
    pub const SHADOW_DETAIL_LOW: &str = "shadow_detail_low";
    pub const SHADOW_DETAIL_MEDIUM: &str = "shadow_detail_medium";
    pub const SHADOW_DETAIL_HIGH: &str = "shadow_detail_high";
    pub const SHADOW_DETAIL_ULTRA: &str = "shadow_detail_ultra";
    pub const SHADOW_METHOD_HARD: &str = "shadow_method_hard";
    pub const SHADOW_METHOD_SOFT_PCF: &str = "shadow_method_soft_pcf";
    pub const SHADOW_METHOD_SOFT_PCSS: &str = "shadow_method_soft_pcss";
    pub const MSAA_OFF: &str = "msaa_off";
    pub const MSAA_X2: &str = "msaa_x2";
    pub const MSAA_X4: &str = "msaa_x4";
    pub const MSAA_X8: &str = "msaa_x8";
    pub const MSAA_X16: &str = "msaa_x16";
    pub const SSAA_OFF: &str = "ssaa_off";
    pub const SSAA_X2: &str = "ssaa_x2";
    pub const SSAA_X3: &str = "ssaa_x3";
    pub const SSAA_X4: &str = "ssaa_x4";
    pub const ANTI_ALIASING_OFF: &str = "anti_aliasing_off";
    pub const ANTI_ALIASING_FXAA: &str = "anti_aliasing_fxaa";
    pub const ANTI_ALIASING_SMAA: &str = "anti_aliasing_smaa";
    pub const ANTI_ALIASING_DLSS: &str = "anti_aliasing_dlss";
    pub const AURA_AUTO: &str = "aura_auto";
    pub const AURA_OFF: &str = "aura_off";
    pub const AURA_CYAN: &str = "aura_cyan";
    pub const AURA_VIOLET: &str = "aura_violet";
    pub const AURA_BLUE: &str = "aura_blue";
    pub const AURA_GOLD: &str = "aura_gold";
    pub const AURA_GREEN: &str = "aura_green";
    pub const SCALING_50: &str = "scaling_50";
    pub const SCALING_60: &str = "scaling_60";
    pub const SCALING_70: &str = "scaling_70";
    pub const SCALING_75: &str = "scaling_75";
    pub const SCALING_80: &str = "scaling_80";
    pub const SCALING_90: &str = "scaling_90";
    pub const SCALING_100: &str = "scaling_100";
    pub const SCALING_110: &str = "scaling_110";
    pub const SCALING_120: &str = "scaling_120";
    pub const SCALING_125: &str = "scaling_125";
    pub const SCALING_130: &str = "scaling_130";
    pub const SCALING_140: &str = "scaling_140";
    pub const SCALING_150: &str = "scaling_150";
    pub const SCALING_160: &str = "scaling_160";
    pub const SCALING_170: &str = "scaling_170";
    pub const SCALING_175: &str = "scaling_175";
    pub const SCALING_180: &str = "scaling_180";
    pub const SCALING_190: &str = "scaling_190";
    pub const SCALING_200: &str = "scaling_200";
    pub const SCALING_CUSTOM: &str = "scaling_custom";
    pub const LIGHTING_CLASSIC: &str = "lighting_classic";
    pub const LIGHTING_ENHANCED: &str = "lighting_enhanced";
}

/// Static texts of the interface windows, buttons and tooltips.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct UiTexts {
    pub context_menu_whisper: String,
    pub context_menu_whisper_tooltip: String,
    pub context_menu_add_friend: String,
    pub context_menu_add_friend_tooltip: String,
    pub context_menu_invite_to_party: String,
    pub context_menu_trade: String,
    pub context_menu_view_character_info: String,
    pub context_menu_item_view_info: String,
    pub context_menu_item_view_info_tooltip: String,
    pub context_menu_drop: String,
    pub dialog_choose_warp_destination: String,
    pub friend_request_text: String,
    pub character_deletion_warning: String,
    pub skill_description_tip: String,
    pub skill_not_learned_text: String,
    /// Affirmative answer ("Yes").
    pub yes_text: String,
    /// Negative answer ("No").
    pub no_text: String,
    pub skill_level_label: String,
    pub skill_type_label: String,
    pub skill_sp_cost_label: String,
    pub skill_range_label: String,
    pub skill_upgradable_label: String,
    pub skill_type_passive: String,
    pub skill_type_attack: String,
    pub skill_type_ground: String,
    pub skill_type_self_cast: String,
    pub skill_type_support: String,
    pub skill_type_trap: String,
    pub item_category_label: String,
    pub item_identified_label: String,
    pub item_weight_label: String,
    pub item_attack_label: String,
    pub item_defense_label: String,
    pub item_required_level_label: String,
    pub item_bonuses_label: String,
    pub item_card_slots_label: String,
    pub item_random_options_label: String,
    pub item_description_label: String,
    pub item_unknown_card_text: String,
    pub item_empty_card_slot_text: String,
    pub item_unavailable_card_slot_text: String,
    pub graphics_resolution_label: String,
    pub graphics_borderless_fullscreen_label: String,
    pub graphics_lighting_mode_label: String,
    pub graphics_triple_buffering_label: String,
    pub graphics_vsync_label: String,
    pub graphics_max_fps_label: String,
    pub graphics_background_fps_label: String,
    pub graphics_texture_filtering_label: String,
    pub graphics_multisampling_label: String,
    pub graphics_supersampling_label: String,
    pub graphics_anti_aliasing_label: String,
    pub graphics_shadow_method_label: String,
    pub graphics_shadow_detail_label: String,
    pub graphics_shadow_resolution_label: String,
    pub graphics_sdsm_label: String,
    pub graphics_high_quality_interface_label: String,
    pub quantity_add_one: String,
    pub quantity_add_ten: String,
    pub quantity_add_hundred: String,
    pub quantity_remove_one: String,
    pub quantity_remove_ten: String,
    pub quantity_remove_hundred: String,
    pub quantity_remove_all: String,
    pub cart_total_label: String,
    pub zeny_unknown_text: String,
    pub storage_close_tooltip: String,
    pub menu_render_options: String,
    pub menu_render_options_tooltip: String,
    pub menu_map_inspector: String,
    pub menu_map_inspector_tooltip: String,
    pub menu_client_state_inspector: String,
    pub menu_client_state_inspector_tooltip: String,
    pub menu_maps: String,
    pub menu_maps_tooltip: String,
    pub menu_commands: String,
    pub menu_commands_tooltip: String,
    pub menu_theme_inspector: String,
    pub menu_theme_inspector_tooltip: String,
    pub menu_profiler: String,
    pub menu_profiler_tooltip: String,
    pub menu_packet_inspector: String,
    pub menu_packet_inspector_tooltip: String,
    pub menu_cache_statistics: String,
    pub menu_cache_statistics_tooltip: String,
    pub equipment_header: String,
    pub character_header: String,
    pub equipment_weapon_attack_label: String,
    pub equipment_armor_defense_label: String,
    pub equipment_refine_label: String,
    pub equipment_equipped_weight_label: String,
    pub equipment_item_bonuses_label: String,
    pub equipment_attack_label: String,
    pub equipment_magic_attack_label: String,
    pub equipment_defense_label: String,
    pub equipment_magic_defense_label: String,
    pub equipment_hit_label: String,
    pub equipment_flee_label: String,
    pub equipment_critical_label: String,
    pub equipment_attack_speed_label: String,
    pub equipment_weight_label: String,
    pub context_menu_use: String,
    pub context_menu_equip: String,
    pub context_menu_use_or_equip: String,
    pub context_menu_store_one: String,
    pub context_menu_take_out_one: String,
    pub context_menu_store_all: String,
    pub context_menu_take_out_all: String,
    pub party_enter_name_tooltip: String,
    pub party_must_be_in_party_to_invite_tooltip: String,
    pub party_not_in_party_tooltip: String,
    pub party_invite_not_implemented_tooltip: String,
    pub trade_not_implemented_tooltip: String,
    pub character_info_not_implemented_tooltip: String,
    pub item_not_usable_tooltip: String,
    pub storage_kafra_tooltip: String,
    pub drop_not_implemented_tooltip: String,
    pub account_email_first_tooltip: String,
    pub setting_unsupported_tooltip: String,
    pub character_level_text: String,
    pub character_spheres_text: String,
    pub party_invitation_text: String,
    pub party_no_invitation_text: String,
}

impl Default for UiTexts {
    fn default() -> Self {
        Self {
            context_menu_whisper: "Whisper".to_owned(),
            context_menu_whisper_tooltip: "Open the chat with a whisper to ^000001{}^000000".to_owned(),
            context_menu_add_friend: "Add Friend".to_owned(),
            context_menu_add_friend_tooltip: "Send a friend request to ^000001{}^000000".to_owned(),
            context_menu_invite_to_party: "Invite to Party".to_owned(),
            context_menu_trade: "Trade".to_owned(),
            context_menu_view_character_info: "View Info".to_owned(),
            context_menu_item_view_info: "View Info".to_owned(),
            context_menu_item_view_info_tooltip: "Show the item description and stats".to_owned(),
            context_menu_drop: "Drop".to_owned(),
            dialog_choose_warp_destination: "Choose a warp destination".to_owned(),
            friend_request_text: "^000001{}^000000 wants to be friends with you".to_owned(),
            character_deletion_warning: "Deleting a character is permanent. To confirm, enter the e-mail address of this account."
                .to_owned(),
            skill_description_tip: "Tip: right-click any skill in the skill tree to inspect it.".to_owned(),
            skill_not_learned_text: "not learned (max {})".to_owned(),
            yes_text: "Yes".to_owned(),
            no_text: "No".to_owned(),
            skill_level_label: "Level".to_owned(),
            skill_type_label: "Type".to_owned(),
            skill_sp_cost_label: "SP cost".to_owned(),
            skill_range_label: "Range".to_owned(),
            skill_upgradable_label: "Upgradable".to_owned(),
            skill_type_passive: "Passive".to_owned(),
            skill_type_attack: "Attack".to_owned(),
            skill_type_ground: "Ground target".to_owned(),
            skill_type_self_cast: "Self cast".to_owned(),
            skill_type_support: "Support".to_owned(),
            skill_type_trap: "Trap".to_owned(),
            item_category_label: "Category".to_owned(),
            item_identified_label: "Identified".to_owned(),
            item_weight_label: "Weight".to_owned(),
            item_attack_label: "Attack".to_owned(),
            item_defense_label: "Defense".to_owned(),
            item_required_level_label: "Required level".to_owned(),
            item_bonuses_label: "Bonuses".to_owned(),
            item_card_slots_label: "Card slots".to_owned(),
            item_random_options_label: "Random options".to_owned(),
            item_description_label: "Description".to_owned(),
            item_unknown_card_text: "Unknown card".to_owned(),
            item_empty_card_slot_text: "Empty card slot".to_owned(),
            item_unavailable_card_slot_text: "Unavailable card slot".to_owned(),
            graphics_resolution_label: "Resolution".to_owned(),
            graphics_borderless_fullscreen_label: "Borderless fullscreen".to_owned(),
            graphics_lighting_mode_label: "Lighting mode".to_owned(),
            graphics_triple_buffering_label: "Triple buffering".to_owned(),
            graphics_vsync_label: "Enable VSYNC".to_owned(),
            graphics_max_fps_label: "Max FPS".to_owned(),
            graphics_background_fps_label: "Background FPS".to_owned(),
            graphics_texture_filtering_label: "Texture filtering".to_owned(),
            graphics_multisampling_label: "Multisampling".to_owned(),
            graphics_supersampling_label: "Supersampling".to_owned(),
            graphics_anti_aliasing_label: "Anti-aliasing".to_owned(),
            graphics_shadow_method_label: "Shadow method".to_owned(),
            graphics_shadow_detail_label: "Shadow detail".to_owned(),
            graphics_shadow_resolution_label: "Shadow resolution".to_owned(),
            graphics_sdsm_label: "Sample Distribution Shadow Maps".to_owned(),
            graphics_high_quality_interface_label: "High quality interface".to_owned(),
            quantity_add_one: "+1".to_owned(),
            quantity_add_ten: "+10".to_owned(),
            quantity_add_hundred: "+100".to_owned(),
            quantity_remove_one: "-1".to_owned(),
            quantity_remove_ten: "-10".to_owned(),
            quantity_remove_hundred: "-100".to_owned(),
            quantity_remove_all: "-All".to_owned(),
            cart_total_label: "Total:".to_owned(),
            zeny_unknown_text: "— z".to_owned(),
            storage_close_tooltip: "Close the storage window".to_owned(),
            menu_render_options: "Render options".to_owned(),
            menu_render_options_tooltip: "Special render options (^000001only available in debug mode^000000)".to_owned(),
            menu_map_inspector: "Map inspector".to_owned(),
            menu_map_inspector_tooltip: "Inspect the raw map data (^000001only available in debug mode^000000)".to_owned(),
            menu_client_state_inspector: "Client state inspector".to_owned(),
            menu_client_state_inspector_tooltip: "Inspect and modify the internal client state (^000001only available in debug \
                                                  mode^000000)"
                .to_owned(),
            menu_maps: "Maps".to_owned(),
            menu_maps_tooltip: "List of maps used for testing (^000001only available in debug mode^000000)".to_owned(),
            menu_commands: "Commands".to_owned(),
            menu_commands_tooltip: "List of commands used for testing (^000001only available in debug mode^000000)".to_owned(),
            menu_theme_inspector: "Theme inspector".to_owned(),
            menu_theme_inspector_tooltip: "Inspect and edit the theme (^000001only available in debug mode^000000)".to_owned(),
            menu_profiler: "Profiler".to_owned(),
            menu_profiler_tooltip: "Profile the client (^000001only available in debug mode^000000)".to_owned(),
            menu_packet_inspector: "Packet inspector".to_owned(),
            menu_packet_inspector_tooltip: "Inspect all incoming and outgoing packets (^000001only available in debug mode^000000)"
                .to_owned(),
            menu_cache_statistics: "Cache statistics".to_owned(),
            menu_cache_statistics_tooltip: "Shows statistics of the caches used by the client (^000001only available in debug mode^000000)"
                .to_owned(),
            equipment_header: "EQUIPMENT".to_owned(),
            character_header: "CHARACTER".to_owned(),
            equipment_weapon_attack_label: "Weapon attack".to_owned(),
            equipment_armor_defense_label: "Armor defense".to_owned(),
            equipment_refine_label: "Refine".to_owned(),
            equipment_equipped_weight_label: "Equipped weight".to_owned(),
            equipment_item_bonuses_label: "Item bonuses".to_owned(),
            equipment_attack_label: "Attack".to_owned(),
            equipment_magic_attack_label: "Magic attack".to_owned(),
            equipment_defense_label: "Defense".to_owned(),
            equipment_magic_defense_label: "Magic defense".to_owned(),
            equipment_hit_label: "Hit".to_owned(),
            equipment_flee_label: "Flee".to_owned(),
            equipment_critical_label: "Critical".to_owned(),
            equipment_attack_speed_label: "Attack speed".to_owned(),
            equipment_weight_label: "Weight".to_owned(),
            context_menu_use: "Use".to_owned(),
            context_menu_equip: "Equip".to_owned(),
            context_menu_use_or_equip: "Use / Equip".to_owned(),
            context_menu_store_one: "Store 1".to_owned(),
            context_menu_take_out_one: "Take out 1".to_owned(),
            context_menu_store_all: "Store all".to_owned(),
            context_menu_take_out_all: "Take out all".to_owned(),
            party_enter_name_tooltip: "Enter a party name first".to_owned(),
            party_must_be_in_party_to_invite_tooltip: "You have to be in a party to invite characters".to_owned(),
            party_not_in_party_tooltip: "You are not in a party".to_owned(),
            party_invite_not_implemented_tooltip: "Not implemented yet: the client cannot send party invitations".to_owned(),
            trade_not_implemented_tooltip: "Not implemented yet: the client cannot start trades".to_owned(),
            character_info_not_implemented_tooltip: "Not implemented yet: there is no character info window".to_owned(),
            item_not_usable_tooltip: "This item cannot be used or equipped".to_owned(),
            storage_kafra_tooltip: "Talk to a Kafra to use the storage".to_owned(),
            drop_not_implemented_tooltip: "Not implemented yet: the client cannot drop items on the ground".to_owned(),
            account_email_first_tooltip: "Enter the account e-mail address first".to_owned(),
            setting_unsupported_tooltip: "This setting is not supported on your system".to_owned(),
            character_level_text: "Lv {} / Jlv {}".to_owned(),
            character_spheres_text: "Spheres {}".to_owned(),
            party_invitation_text: "You are invited to join the party \"{}\".".to_owned(),
            party_no_invitation_text: "You have no open party invitation.".to_owned(),
        }
    }
}

/// Names and labels of the equipment slots of the paper doll.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct PaperDollTexts {
    pub head_top_name: String,
    pub head_top_label: String,
    pub ammo_name: String,
    pub ammo_label: String,
    pub head_middle_name: String,
    pub head_middle_label: String,
    pub head_lower_name: String,
    pub head_lower_label: String,
    pub armor_name: String,
    pub armor_label: String,
    pub garment_name: String,
    pub garment_label: String,
    pub right_hand_name: String,
    pub right_hand_label: String,
    pub left_hand_name: String,
    pub left_hand_label: String,
    pub accessory_1_name: String,
    pub accessory_1_label: String,
    pub accessory_2_name: String,
    pub accessory_2_label: String,
    pub shoes_name: String,
    pub shoes_label: String,
}

impl Default for PaperDollTexts {
    fn default() -> Self {
        Self {
            head_top_name: "Head top".to_owned(),
            head_top_label: "TOP".to_owned(),
            ammo_name: "Ammo".to_owned(),
            ammo_label: "AMMO".to_owned(),
            head_middle_name: "Head middle".to_owned(),
            head_middle_label: "MID".to_owned(),
            head_lower_name: "Head lower".to_owned(),
            head_lower_label: "LOW".to_owned(),
            armor_name: "Armor".to_owned(),
            armor_label: "ARMOR".to_owned(),
            garment_name: "Garment".to_owned(),
            garment_label: "GARM".to_owned(),
            right_hand_name: "Right hand (weapon)".to_owned(),
            right_hand_label: "WPN".to_owned(),
            left_hand_name: "Left hand (shield)".to_owned(),
            left_hand_label: "SHLD".to_owned(),
            accessory_1_name: "Accessory 1".to_owned(),
            accessory_1_label: "ACC1".to_owned(),
            accessory_2_name: "Accessory 2".to_owned(),
            accessory_2_label: "ACC2".to_owned(),
            shoes_name: "Shoes".to_owned(),
            shoes_label: "SHOES".to_owned(),
        }
    }
}

impl PaperDollTexts {
    /// Localized long name of an equipment slot (shown in the empty-slot
    /// tooltip).
    pub fn slot_name(&self, position: EquipPosition) -> &str {
        match position {
            EquipPosition::HEAD_TOP => &self.head_top_name,
            EquipPosition::AMMO => &self.ammo_name,
            EquipPosition::HEAD_MIDDLE => &self.head_middle_name,
            EquipPosition::HEAD_LOWER => &self.head_lower_name,
            EquipPosition::ARMOR => &self.armor_name,
            EquipPosition::GARMENT => &self.garment_name,
            EquipPosition::RIGHT_HAND => &self.right_hand_name,
            EquipPosition::LEFT_HAND => &self.left_hand_name,
            EquipPosition::LEFT_ACCESSORY => &self.accessory_1_name,
            EquipPosition::RIGTH_ACCESSORY => &self.accessory_2_name,
            EquipPosition::SHOES => &self.shoes_name,
            _ => &self.head_top_name,
        }
    }

    /// Localized short label of an equipment slot (shown in the empty slot).
    pub fn slot_label(&self, position: EquipPosition) -> &str {
        match position {
            EquipPosition::HEAD_TOP => &self.head_top_label,
            EquipPosition::AMMO => &self.ammo_label,
            EquipPosition::HEAD_MIDDLE => &self.head_middle_label,
            EquipPosition::HEAD_LOWER => &self.head_lower_label,
            EquipPosition::ARMOR => &self.armor_label,
            EquipPosition::GARMENT => &self.garment_label,
            EquipPosition::RIGHT_HAND => &self.right_hand_label,
            EquipPosition::LEFT_HAND => &self.left_hand_label,
            EquipPosition::LEFT_ACCESSORY => &self.accessory_1_label,
            EquipPosition::RIGTH_ACCESSORY => &self.accessory_2_label,
            EquipPosition::SHOES => &self.shoes_label,
            _ => &self.head_top_label,
        }
    }
}

/// Localized labels of the settings drop down options.
#[derive(Serialize, Deserialize, RustState, StateElement)]
pub struct DropdownTexts {
    pub resolution_desktop: String,
    pub resolution_1280x720: String,
    pub resolution_1366x768: String,
    pub resolution_1600x900: String,
    pub resolution_1920x1080: String,
    pub resolution_2560x1440: String,
    pub resolution_3840x2160: String,
    pub resolution_custom: String,
    pub fps_unlimited: String,
    pub fps_15: String,
    pub fps_24: String,
    pub fps_30: String,
    pub fps_45: String,
    pub fps_60: String,
    pub fps_75: String,
    pub fps_90: String,
    pub fps_120: String,
    pub fps_144: String,
    pub fps_240: String,
    pub fps_custom: String,
    pub sampler_nearest: String,
    pub sampler_linear: String,
    pub sampler_anisotropic_4: String,
    pub sampler_anisotropic_8: String,
    pub sampler_anisotropic_16: String,
    pub shadow_resolution_normal: String,
    pub shadow_resolution_ultra: String,
    pub shadow_resolution_insane: String,
    pub shadow_detail_low: String,
    pub shadow_detail_medium: String,
    pub shadow_detail_high: String,
    pub shadow_detail_ultra: String,
    pub shadow_method_hard: String,
    pub shadow_method_soft_pcf: String,
    pub shadow_method_soft_pcss: String,
    pub msaa_off: String,
    pub msaa_x2: String,
    pub msaa_x4: String,
    pub msaa_x8: String,
    pub msaa_x16: String,
    pub ssaa_off: String,
    pub ssaa_x2: String,
    pub ssaa_x3: String,
    pub ssaa_x4: String,
    pub anti_aliasing_off: String,
    pub anti_aliasing_fxaa: String,
    pub anti_aliasing_smaa: String,
    pub anti_aliasing_dlss: String,
    pub aura_auto: String,
    pub aura_off: String,
    pub aura_cyan: String,
    pub aura_violet: String,
    pub aura_blue: String,
    pub aura_gold: String,
    pub aura_green: String,
    pub scaling_50: String,
    pub scaling_60: String,
    pub scaling_70: String,
    pub scaling_75: String,
    pub scaling_80: String,
    pub scaling_90: String,
    pub scaling_100: String,
    pub scaling_110: String,
    pub scaling_120: String,
    pub scaling_125: String,
    pub scaling_130: String,
    pub scaling_140: String,
    pub scaling_150: String,
    pub scaling_160: String,
    pub scaling_170: String,
    pub scaling_175: String,
    pub scaling_180: String,
    pub scaling_190: String,
    pub scaling_200: String,
    pub scaling_custom: String,
    pub lighting_classic: String,
    pub lighting_enhanced: String,
}

impl Default for DropdownTexts {
    fn default() -> Self {
        Self {
            resolution_desktop: "Desktop (maximized)".to_owned(),
            resolution_1280x720: "1280 x 720".to_owned(),
            resolution_1366x768: "1366 x 768".to_owned(),
            resolution_1600x900: "1600 x 900".to_owned(),
            resolution_1920x1080: "1920 x 1080".to_owned(),
            resolution_2560x1440: "2560 x 1440".to_owned(),
            resolution_3840x2160: "3840 x 2160".to_owned(),
            resolution_custom: "Custom".to_owned(),
            fps_unlimited: "Unlimited".to_owned(),
            fps_15: "15 FPS".to_owned(),
            fps_24: "24 FPS".to_owned(),
            fps_30: "30 FPS".to_owned(),
            fps_45: "45 FPS".to_owned(),
            fps_60: "60 FPS".to_owned(),
            fps_75: "75 FPS".to_owned(),
            fps_90: "90 FPS".to_owned(),
            fps_120: "120 FPS".to_owned(),
            fps_144: "144 FPS".to_owned(),
            fps_240: "240 FPS".to_owned(),
            fps_custom: "Custom".to_owned(),
            sampler_nearest: "Nearest".to_owned(),
            sampler_linear: "Linear".to_owned(),
            sampler_anisotropic_4: "Anisotropic x4".to_owned(),
            sampler_anisotropic_8: "Anisotropic x8".to_owned(),
            sampler_anisotropic_16: "Anisotropic x16".to_owned(),
            shadow_resolution_normal: "Normal".to_owned(),
            shadow_resolution_ultra: "Ultra".to_owned(),
            shadow_resolution_insane: "Insane".to_owned(),
            shadow_detail_low: "Low".to_owned(),
            shadow_detail_medium: "Medium".to_owned(),
            shadow_detail_high: "High".to_owned(),
            shadow_detail_ultra: "Ultra".to_owned(),
            shadow_method_hard: "Hard".to_owned(),
            shadow_method_soft_pcf: "Soft (PCF)".to_owned(),
            shadow_method_soft_pcss: "Soft (PCSS)".to_owned(),
            msaa_off: "Off".to_owned(),
            msaa_x2: "x2".to_owned(),
            msaa_x4: "x4".to_owned(),
            msaa_x8: "x8".to_owned(),
            msaa_x16: "x16".to_owned(),
            ssaa_off: "Off".to_owned(),
            ssaa_x2: "x2".to_owned(),
            ssaa_x3: "x3".to_owned(),
            ssaa_x4: "x4".to_owned(),
            anti_aliasing_off: "Off".to_owned(),
            anti_aliasing_fxaa: "FXAA".to_owned(),
            anti_aliasing_smaa: "SMAA".to_owned(),
            anti_aliasing_dlss: "DLSS".to_owned(),
            aura_auto: "Auto (tier default)".to_owned(),
            aura_off: "Off".to_owned(),
            aura_cyan: "Cyan".to_owned(),
            aura_violet: "Violet".to_owned(),
            aura_blue: "Blue".to_owned(),
            aura_gold: "Gold".to_owned(),
            aura_green: "Green".to_owned(),
            scaling_50: "50%".to_owned(),
            scaling_60: "60%".to_owned(),
            scaling_70: "70%".to_owned(),
            scaling_75: "75%".to_owned(),
            scaling_80: "80%".to_owned(),
            scaling_90: "90%".to_owned(),
            scaling_100: "100%".to_owned(),
            scaling_110: "110%".to_owned(),
            scaling_120: "120%".to_owned(),
            scaling_125: "125%".to_owned(),
            scaling_130: "130%".to_owned(),
            scaling_140: "140%".to_owned(),
            scaling_150: "150%".to_owned(),
            scaling_160: "160%".to_owned(),
            scaling_170: "170%".to_owned(),
            scaling_175: "175%".to_owned(),
            scaling_180: "180%".to_owned(),
            scaling_190: "190%".to_owned(),
            scaling_200: "200%".to_owned(),
            scaling_custom: "Custom".to_owned(),
            lighting_classic: "Classic".to_owned(),
            lighting_enhanced: "Enhanced".to_owned(),
        }
    }
}

impl DropdownTexts {
    /// All drop down option keys, used by the completeness test.
    #[cfg(test)]
    pub const KEYS: &'static [&'static str] = &[
        dropdown_keys::RESOLUTION_DESKTOP,
        dropdown_keys::RESOLUTION_1280X720,
        dropdown_keys::RESOLUTION_1366X768,
        dropdown_keys::RESOLUTION_1600X900,
        dropdown_keys::RESOLUTION_1920X1080,
        dropdown_keys::RESOLUTION_2560X1440,
        dropdown_keys::RESOLUTION_3840X2160,
        dropdown_keys::RESOLUTION_CUSTOM,
        dropdown_keys::FPS_UNLIMITED,
        dropdown_keys::FPS_15,
        dropdown_keys::FPS_24,
        dropdown_keys::FPS_30,
        dropdown_keys::FPS_45,
        dropdown_keys::FPS_60,
        dropdown_keys::FPS_75,
        dropdown_keys::FPS_90,
        dropdown_keys::FPS_120,
        dropdown_keys::FPS_144,
        dropdown_keys::FPS_240,
        dropdown_keys::FPS_CUSTOM,
        dropdown_keys::SAMPLER_NEAREST,
        dropdown_keys::SAMPLER_LINEAR,
        dropdown_keys::SAMPLER_ANISOTROPIC_4,
        dropdown_keys::SAMPLER_ANISOTROPIC_8,
        dropdown_keys::SAMPLER_ANISOTROPIC_16,
        dropdown_keys::SHADOW_RESOLUTION_NORMAL,
        dropdown_keys::SHADOW_RESOLUTION_ULTRA,
        dropdown_keys::SHADOW_RESOLUTION_INSANE,
        dropdown_keys::SHADOW_DETAIL_LOW,
        dropdown_keys::SHADOW_DETAIL_MEDIUM,
        dropdown_keys::SHADOW_DETAIL_HIGH,
        dropdown_keys::SHADOW_DETAIL_ULTRA,
        dropdown_keys::SHADOW_METHOD_HARD,
        dropdown_keys::SHADOW_METHOD_SOFT_PCF,
        dropdown_keys::SHADOW_METHOD_SOFT_PCSS,
        dropdown_keys::MSAA_OFF,
        dropdown_keys::MSAA_X2,
        dropdown_keys::MSAA_X4,
        dropdown_keys::MSAA_X8,
        dropdown_keys::MSAA_X16,
        dropdown_keys::SSAA_OFF,
        dropdown_keys::SSAA_X2,
        dropdown_keys::SSAA_X3,
        dropdown_keys::SSAA_X4,
        dropdown_keys::ANTI_ALIASING_OFF,
        dropdown_keys::ANTI_ALIASING_FXAA,
        dropdown_keys::ANTI_ALIASING_SMAA,
        dropdown_keys::ANTI_ALIASING_DLSS,
        dropdown_keys::AURA_AUTO,
        dropdown_keys::AURA_OFF,
        dropdown_keys::AURA_CYAN,
        dropdown_keys::AURA_VIOLET,
        dropdown_keys::AURA_BLUE,
        dropdown_keys::AURA_GOLD,
        dropdown_keys::AURA_GREEN,
        dropdown_keys::SCALING_50,
        dropdown_keys::SCALING_60,
        dropdown_keys::SCALING_70,
        dropdown_keys::SCALING_75,
        dropdown_keys::SCALING_80,
        dropdown_keys::SCALING_90,
        dropdown_keys::SCALING_100,
        dropdown_keys::SCALING_110,
        dropdown_keys::SCALING_120,
        dropdown_keys::SCALING_125,
        dropdown_keys::SCALING_130,
        dropdown_keys::SCALING_140,
        dropdown_keys::SCALING_150,
        dropdown_keys::SCALING_160,
        dropdown_keys::SCALING_170,
        dropdown_keys::SCALING_175,
        dropdown_keys::SCALING_180,
        dropdown_keys::SCALING_190,
        dropdown_keys::SCALING_200,
        dropdown_keys::SCALING_CUSTOM,
        dropdown_keys::LIGHTING_CLASSIC,
        dropdown_keys::LIGHTING_ENHANCED,
    ];

    /// Resolve the localized text of a drop down option label.
    pub fn text(&self, key: &str) -> Option<&str> {
        match key {
            dropdown_keys::RESOLUTION_DESKTOP => Some(&self.resolution_desktop),
            dropdown_keys::RESOLUTION_1280X720 => Some(&self.resolution_1280x720),
            dropdown_keys::RESOLUTION_1366X768 => Some(&self.resolution_1366x768),
            dropdown_keys::RESOLUTION_1600X900 => Some(&self.resolution_1600x900),
            dropdown_keys::RESOLUTION_1920X1080 => Some(&self.resolution_1920x1080),
            dropdown_keys::RESOLUTION_2560X1440 => Some(&self.resolution_2560x1440),
            dropdown_keys::RESOLUTION_3840X2160 => Some(&self.resolution_3840x2160),
            dropdown_keys::RESOLUTION_CUSTOM => Some(&self.resolution_custom),
            dropdown_keys::FPS_UNLIMITED => Some(&self.fps_unlimited),
            dropdown_keys::FPS_15 => Some(&self.fps_15),
            dropdown_keys::FPS_24 => Some(&self.fps_24),
            dropdown_keys::FPS_30 => Some(&self.fps_30),
            dropdown_keys::FPS_45 => Some(&self.fps_45),
            dropdown_keys::FPS_60 => Some(&self.fps_60),
            dropdown_keys::FPS_75 => Some(&self.fps_75),
            dropdown_keys::FPS_90 => Some(&self.fps_90),
            dropdown_keys::FPS_120 => Some(&self.fps_120),
            dropdown_keys::FPS_144 => Some(&self.fps_144),
            dropdown_keys::FPS_240 => Some(&self.fps_240),
            dropdown_keys::FPS_CUSTOM => Some(&self.fps_custom),
            dropdown_keys::SAMPLER_NEAREST => Some(&self.sampler_nearest),
            dropdown_keys::SAMPLER_LINEAR => Some(&self.sampler_linear),
            dropdown_keys::SAMPLER_ANISOTROPIC_4 => Some(&self.sampler_anisotropic_4),
            dropdown_keys::SAMPLER_ANISOTROPIC_8 => Some(&self.sampler_anisotropic_8),
            dropdown_keys::SAMPLER_ANISOTROPIC_16 => Some(&self.sampler_anisotropic_16),
            dropdown_keys::SHADOW_RESOLUTION_NORMAL => Some(&self.shadow_resolution_normal),
            dropdown_keys::SHADOW_RESOLUTION_ULTRA => Some(&self.shadow_resolution_ultra),
            dropdown_keys::SHADOW_RESOLUTION_INSANE => Some(&self.shadow_resolution_insane),
            dropdown_keys::SHADOW_DETAIL_LOW => Some(&self.shadow_detail_low),
            dropdown_keys::SHADOW_DETAIL_MEDIUM => Some(&self.shadow_detail_medium),
            dropdown_keys::SHADOW_DETAIL_HIGH => Some(&self.shadow_detail_high),
            dropdown_keys::SHADOW_DETAIL_ULTRA => Some(&self.shadow_detail_ultra),
            dropdown_keys::SHADOW_METHOD_HARD => Some(&self.shadow_method_hard),
            dropdown_keys::SHADOW_METHOD_SOFT_PCF => Some(&self.shadow_method_soft_pcf),
            dropdown_keys::SHADOW_METHOD_SOFT_PCSS => Some(&self.shadow_method_soft_pcss),
            dropdown_keys::MSAA_OFF => Some(&self.msaa_off),
            dropdown_keys::MSAA_X2 => Some(&self.msaa_x2),
            dropdown_keys::MSAA_X4 => Some(&self.msaa_x4),
            dropdown_keys::MSAA_X8 => Some(&self.msaa_x8),
            dropdown_keys::MSAA_X16 => Some(&self.msaa_x16),
            dropdown_keys::SSAA_OFF => Some(&self.ssaa_off),
            dropdown_keys::SSAA_X2 => Some(&self.ssaa_x2),
            dropdown_keys::SSAA_X3 => Some(&self.ssaa_x3),
            dropdown_keys::SSAA_X4 => Some(&self.ssaa_x4),
            dropdown_keys::ANTI_ALIASING_OFF => Some(&self.anti_aliasing_off),
            dropdown_keys::ANTI_ALIASING_FXAA => Some(&self.anti_aliasing_fxaa),
            dropdown_keys::ANTI_ALIASING_SMAA => Some(&self.anti_aliasing_smaa),
            dropdown_keys::ANTI_ALIASING_DLSS => Some(&self.anti_aliasing_dlss),
            dropdown_keys::AURA_AUTO => Some(&self.aura_auto),
            dropdown_keys::AURA_OFF => Some(&self.aura_off),
            dropdown_keys::AURA_CYAN => Some(&self.aura_cyan),
            dropdown_keys::AURA_VIOLET => Some(&self.aura_violet),
            dropdown_keys::AURA_BLUE => Some(&self.aura_blue),
            dropdown_keys::AURA_GOLD => Some(&self.aura_gold),
            dropdown_keys::AURA_GREEN => Some(&self.aura_green),
            dropdown_keys::SCALING_50 => Some(&self.scaling_50),
            dropdown_keys::SCALING_60 => Some(&self.scaling_60),
            dropdown_keys::SCALING_70 => Some(&self.scaling_70),
            dropdown_keys::SCALING_75 => Some(&self.scaling_75),
            dropdown_keys::SCALING_80 => Some(&self.scaling_80),
            dropdown_keys::SCALING_90 => Some(&self.scaling_90),
            dropdown_keys::SCALING_100 => Some(&self.scaling_100),
            dropdown_keys::SCALING_110 => Some(&self.scaling_110),
            dropdown_keys::SCALING_120 => Some(&self.scaling_120),
            dropdown_keys::SCALING_125 => Some(&self.scaling_125),
            dropdown_keys::SCALING_130 => Some(&self.scaling_130),
            dropdown_keys::SCALING_140 => Some(&self.scaling_140),
            dropdown_keys::SCALING_150 => Some(&self.scaling_150),
            dropdown_keys::SCALING_160 => Some(&self.scaling_160),
            dropdown_keys::SCALING_170 => Some(&self.scaling_170),
            dropdown_keys::SCALING_175 => Some(&self.scaling_175),
            dropdown_keys::SCALING_180 => Some(&self.scaling_180),
            dropdown_keys::SCALING_190 => Some(&self.scaling_190),
            dropdown_keys::SCALING_200 => Some(&self.scaling_200),
            dropdown_keys::SCALING_CUSTOM => Some(&self.scaling_custom),
            dropdown_keys::LIGHTING_CLASSIC => Some(&self.lighting_classic),
            dropdown_keys::LIGHTING_ENHANCED => Some(&self.lighting_enhanced),
            _ => None,
        }
    }
}

/// Selector for a localized text template with a single `{}` placeholder that
/// is replaced by a runtime value (e.g. a character name).
///
/// The selector is returned as `impl Selector` on purpose: that keeps the
/// concrete selector type opaque, so it can be passed to the generic text
/// properties of the interface components without ambiguity (the item type
/// also implements [`Selector`] for itself through the rust-state blanket
/// impl).
pub fn template_text(template: fn(&ClientState) -> &str, value: String) -> impl Selector<ClientState, String> {
    ComputedSelector::new_default(move |state: &ClientState| template(state).replace("{}", &value))
}

#[cfg(test)]
mod tests {
    use korangar_interface::element::StateElement;
    use ragnarok_packets::EquipPosition;

    use super::{DropdownTexts, PaperDollTexts, UiTexts};

    /// Every drop down option key must resolve to a non-empty label, otherwise
    /// an option would silently fall back to its untranslated `text()` value.
    #[test]
    fn every_dropdown_key_resolves_to_a_label() {
        let texts = DropdownTexts::default();
        assert_eq!(DropdownTexts::KEYS.len(), 77, "the drop down key table is out of date");
        for key in DropdownTexts::KEYS {
            let text = texts
                .text(key)
                .unwrap_or_else(|| panic!("drop down key {key:?} is not in the table"));
            assert!(!text.trim().is_empty(), "drop down key {key:?} resolves to an empty label");
        }
        // Unknown keys fall back to the untranslated label of the option.
        assert!(texts.text("not-a-drop-down-key").is_none());
    }

    /// The paper doll slot helpers must cover every equipment position that is
    /// shown on the paper doll.
    #[test]
    fn every_paper_doll_slot_has_a_name_and_a_label() {
        let texts = PaperDollTexts::default();
        let positions = [
            EquipPosition::HEAD_TOP,
            EquipPosition::AMMO,
            EquipPosition::HEAD_MIDDLE,
            EquipPosition::HEAD_LOWER,
            EquipPosition::ARMOR,
            EquipPosition::GARMENT,
            EquipPosition::RIGHT_HAND,
            EquipPosition::LEFT_HAND,
            EquipPosition::LEFT_ACCESSORY,
            EquipPosition::RIGTH_ACCESSORY,
            EquipPosition::SHOES,
        ];
        for position in positions {
            assert!(
                !texts.slot_name(position).trim().is_empty(),
                "missing slot name for {position:?}"
            );
            assert!(
                !texts.slot_label(position).trim().is_empty(),
                "missing slot label for {position:?}"
            );
        }
    }

    /// The defaults of the text groups are complete English fallbacks: a
    /// language file that predates the groups must not produce empty labels.
    #[test]
    fn text_group_defaults_are_complete() {
        let serialized = ron::ser::to_string(&UiTexts::default()).expect("UI texts must serialize");
        assert!(!serialized.contains(":\"\""), "a UI text default is empty: {serialized}");

        let serialized = ron::ser::to_string(&PaperDollTexts::default()).expect("paper doll texts must serialize");
        assert!(
            !serialized.contains(":\"\""),
            "a paper doll text default is empty: {serialized}"
        );

        let serialized = ron::ser::to_string(&DropdownTexts::default()).expect("drop down texts must serialize");
        assert!(!serialized.contains(":\"\""), "a drop down text default is empty: {serialized}");

        // The generated element implementations must stay available so the
        // debug state inspector can show and edit the groups.
        fn assert_state_element<T: StateElement<crate::state::ClientState>>() {}
        assert_state_element::<UiTexts>();
        assert_state_element::<PaperDollTexts>();
        assert_state_element::<DropdownTexts>();
    }
}
