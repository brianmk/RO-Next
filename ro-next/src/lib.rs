#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(random)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(unsafe_cell_access)]
#![feature(impl_trait_in_assoc_type)]
#![feature(thread_local)]

// Helper macro to time and print the startup time of RO-Next
mod msgstringtable;

macro_rules! time_phase {
    ($message:expr, { $($statements:tt)* }) => {
        #[cfg(feature = "debug")]
        let _statement_timer = ro_next_debug::logging::Timer::new($message);

        $($statements)*

        #[cfg(feature = "debug")]
        _statement_timer.stop();
    }
}

mod benchmark;
mod client;

pub use self::client_config::checks::*;

mod client_config;
mod graphics;
mod input;
mod perf;
mod state;
#[macro_use]
mod interface;
pub mod crash;
mod loaders;
pub mod logging;
mod login_server;
#[cfg(feature = "debug")]
mod networking;
mod pincode;
mod renderer;
mod settings;
mod system;
mod world;

pub(crate) use std::collections::HashSet;
pub(crate) use std::io::Cursor;
pub(crate) use std::net::SocketAddr;
pub(crate) use std::ops::Deref;
// Only the Linux display-backend selection code uses `crate::Path`; on other
// platforms every path is built from `PathBuf`/`Path` imports in the module
// that needs it, so the re-export would be unused there.
#[cfg(target_os = "linux")]
pub(crate) use std::path::Path;
pub(crate) use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
pub(crate) use std::sync::{Arc, Mutex};
pub(crate) use std::time::{Duration, Instant};

pub(crate) use cgmath::{Point3, Vector3};
pub(crate) use hashbrown::HashMap;
pub(crate) use image::{EncodableLayout, ImageFormat, ImageReader};
pub(crate) use input::{MouseInputMode, MouseModeExt};
#[cfg(feature = "debug")]
pub(crate) use networking::{PacketHistory, PacketHistoryCallback};
pub(crate) use ragnarok_packets::{
    AccountId, AmmunitionActionType, AttackRange, BuyItemResult, CharacterId, CharacterServerInformation, ClientTick, Direction,
    DisappearanceReason, EntityId, EquipPosition, EquippableItemFlags, ExperienceSource, ExperienceType, Friend, HotbarSlot, HotkeyType,
    InventoryIndex, ItemId, JobId, MarkerType, SellItemsResult, SkillId, SkillLevel, SkillType, StatType, TilePosition, UnitId,
    WorldPosition,
};
pub(crate) use renderer::InterfaceRenderer;
pub(crate) use ro_next_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
pub(crate) use ro_next_debug::logging::{Colorize, print_debug};
#[cfg(feature = "debug")]
pub(crate) use ro_next_debug::profile_block;
#[cfg(feature = "debug")]
pub(crate) use ro_next_debug::profiling::Profiler;
pub(crate) use ro_next_interface::application::Clip;
pub(crate) use ro_next_interface::layout::MouseButton;
pub(crate) use ro_next_interface::{Interface, MouseMode, take_layout_profile};
pub(crate) use ro_next_loaders::FileLoader;
pub(crate) use ro_next_messages::{MessageDomain, format_message};
pub(crate) use ro_next_networking::{
    DisconnectReason, HotkeyState, ItemQuantity, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem,
    SellItem, SupportedPacketVersion,
};
pub(crate) use rust_state::{ManuallyAssertExt, State};
#[cfg(feature = "debug")]
pub(crate) use rust_state::{VecIndexExt, VecLookupExt};
pub(crate) use settings::{
    AudioSettings, AudioSettingsPathExt, AuraStyle, GraphicsSettingsCapabilities, GraphicsSettingsPathExt, InterfaceSettings,
    InterfaceSettingsPathExt,
};
pub(crate) use state::bank::parse_zeny_amount;
pub(crate) use state::content_windows::ContentWindowsPathExt;
pub(crate) use state::hotbar::{HOTBAR_ENTRIES_PER_TAB, HOTBAR_TAB_COUNT, HotbarPathExt};
pub(crate) use state::inventory::InventoryPathExt;
pub(crate) use state::lifecycle::{
    AddEntityDecision, EntityLifecycle, RemoveEntityOutcome, ResourcePolicy, ResourceRetryQueue, SHUTDOWN_DISCONNECT_TIMEOUT,
    SessionAction, SessionEvent, SessionMachine, SessionObservation, SessionState,
};
pub(crate) use state::localization::Localization;
pub(crate) use state::skills::SkillTreePathExt;
pub(crate) use state::theme::{CursorThemePathExt, IndicatorThemePathExt, InterfaceThemePathExt, WorldThemePathExt};
pub(crate) use state::vending::VendingShopItem;
pub(crate) use state::{
    ChatChannel, ChatMessage, ClientState, ClientStatePathExt, client_state, status_effect_definition, this_entity, this_player,
};
#[cfg(feature = "debug")]
pub(crate) use wgpu::Device;
pub(crate) use wgpu::util::initialize_adapter_from_env_or_default;
pub(crate) use wgpu::wgt::{Dx12SwapchainKind, Dx12UseFrameLatencyWaitableObject};
pub(crate) use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, Dx12BackendOptions, Dx12Compiler, ExperimentalFeatures, ForceShaderModelToken,
    GlBackendOptions, GlDebugFns, GlFenceBehavior, Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, MemoryBudgetThresholds,
    MemoryHints, NoopBackendOptions, Queue, Trace,
};
pub(crate) use winit::application::ApplicationHandler;
pub(crate) use winit::dpi::{LogicalSize, PhysicalSize, Size};
pub(crate) use winit::event::WindowEvent;
pub(crate) use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
pub(crate) use winit::keyboard::PhysicalKey;
pub(crate) use winit::window::{Fullscreen, Icon, Window, WindowId};

/// Next unique point-light id for position based effects. The point light
/// manager uses the id to decide which light renders with a shadow, so it has
/// to be unique.
pub use self::client::Client;
pub(crate) use self::client::*;
pub use self::client::{SHUTDOWN_SIGNAL, init_tls_rand, log_presentation_fallback_startup_line};
pub(crate) use crate::graphics::*;
pub(crate) use crate::input::{
    BattleModeCommand, InputEvent, InputReport, InputSystem, parse_battle_mode_command, translate_slash_command,
};
pub(crate) use crate::interface::cursor::{MouseCursor, MouseCursorState};
pub(crate) use crate::interface::resource::{ItemSource, SkillSource};
pub(crate) use crate::interface::windows::*;
pub(crate) use crate::loaders::error::LoadError;
pub(crate) use crate::loaders::{
    AssetFailureSeverity, TransientAssetNoticeLimiter, asset_failure_log_key, asset_failure_severity, transient_asset_failure_message, *,
};
#[cfg(feature = "debug")]
pub(crate) use crate::renderer::DebugMarkerRenderer;
pub(crate) use crate::renderer::{AlignHorizontal, EffectRenderer, GameInterfaceRenderer, SpriteRenderer};
pub(crate) use crate::settings::{
    AuraStyleMarker, FORCE_UNFOCUSED_ALIAS_ENVIRONMENT_VARIABLE, FORCE_UNFOCUSED_ENVIRONMENT_VARIABLE, FRAMERATE_LOG_ENVIRONMENT_VARIABLE,
    FRAMERATE_LOG_INTERVAL_SECONDS, GameSettingsPathExt, GraphicsSettings, IN_GAME_THEMES_PATH, LightingMode, MENU_THEMES_PATH,
    ServiceSettingsPathExt, WORLD_THEMES_PATH, parse_aura_style_marker,
};
pub(crate) use crate::state::character_slots::{MAX_CHARACTER_SLOTS, visible_character_slots};
pub(crate) use crate::state::chat_markup::ChatMarkupLink;
pub(crate) use crate::state::inventory::{ItemCategory, ItemUseAction, item_category, item_type, item_use_action};
pub(crate) use crate::state::item_service::{ItemServiceEntry, ItemServiceKind, ItemServicePathExt, RefiningMaterial, RefiningUiPathExt};
pub(crate) use crate::state::localization::{LocalizationPathExt, QolTextsPathExt};
pub(crate) use crate::state::minimap::{MapWindowCycle, direction_index_from_tile_delta};
pub(crate) use crate::state::notifications::NotificationKind;
pub(crate) use crate::state::party::PartyPathExt;
pub(crate) use crate::state::skills::{LearnedSkill, SkillTreeLayoutPathExt, bring_skill_to_level};
pub(crate) use crate::state::stat_buffer::PendingStatUpdates;
pub(crate) use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceTheme, InterfaceThemeType, WorldTheme};
pub(crate) use crate::state::{BufferedAction, SelectedServicePath, client_theme};
pub(crate) use crate::system::{FrameTimers, GameTimer};
#[cfg(feature = "debug")]
pub(crate) use crate::world::MarkerIdentifier;
pub(crate) use crate::world::target_policy::{TargetRelations, player_attack_decision, player_combat_zone};
pub(crate) use crate::world::*;

#[cfg(test)]
mod tests {
    use ragnarok_bytes::{ByteWriter, ToBytes};
    use ragnarok_packets::{AmmunitionActionType, AttackRange, BuyItemResult, JobId, SkillId, SkillLevel, SkillType};

    use super::{ammunition_action_code, buy_item_result_code, can_move_while_casting, cutin_texture_candidates, is_same_map_teleport};
    use crate::interface::windows::book_pages;
    use crate::state::lifecycle::SessionState;
    use crate::state::skills::LearnedSkill;

    fn learned(skill_id: u16) -> LearnedSkill {
        LearnedSkill {
            skill_id: SkillId(skill_id),
            skill_level: SkillLevel(1),
            skill_type: SkillType::Passive,
            spell_point_cost: 0,
            attack_range: AttackRange(1),
            skill_name: String::new(),
            upgradable: false,
        }
    }

    #[test]
    fn free_cast_requires_the_data_driven_skill_flag_and_job_capability() {
        // Sage with the passive learned (skills.ron `move_while_casting` flag
        // + jobs.ron `move_while_casting` capability).
        assert!(can_move_while_casting(JobId(16), &[learned(278)]));
        // Professor (rebirth Sage) with the passive learned.
        assert!(can_move_while_casting(JobId(4017), &[learned(278)]));

        // Non-Sage jobs are never allowed, even with the skill in the list.
        assert!(!can_move_while_casting(JobId(8), &[learned(278)]));
        assert!(!can_move_while_casting(JobId(0), &[learned(278)]));

        // Sage without the passive is rooted.
        assert!(!can_move_while_casting(JobId(16), &[]));
        assert!(!can_move_while_casting(JobId(16), &[learned(28), learned(34)]));
    }

    #[test]
    fn unknown_jobs_default_to_rooted() {
        assert!(!can_move_while_casting(JobId(9999), &[learned(278)]));
        // The behaviour is a data lookup, not a skill-id branch: an unknown
        // skill never grants it.
        assert!(!can_move_while_casting(JobId(16), &[learned(65_500)]));
    }

    #[test]
    fn job_registry_data_is_valid() {
        let rows = super::check_jobs(None).expect("the committed job data has to be valid");
        assert_eq!(rows, 181, "one row per real rAthena e_job id");
        assert!(crate::world::jobs::profile(JobId(4252)).is_some());
        assert!(crate::world::jobs::profile(JobId(65535)).is_none());
    }

    #[test]
    fn job_registry_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-jobs-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(jobs: [)").expect("failed to write the malformed fixture");

        let result = super::check_jobs(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn job_registry_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/jobs.ron");
        let error = super::check_jobs(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn alias_data_is_valid() {
        let summary = super::check_aliases(None).expect("the committed alias data has to be valid");
        assert_eq!(
            summary.job_skill_tree_fallbacks, 56,
            "one fallback per server job with an empty client tree"
        );
        assert_eq!(
            summary.monster_sprite_aliases, 232,
            "one alias per monster without an exact sprite"
        );
        assert_eq!(summary.skill_icon_aliases, 241, "one alias per skill without an exact icon");
    }

    #[test]
    fn alias_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-aliases-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(version: 1,").expect("failed to write the malformed fixture");

        let result = super::check_aliases(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn keybind_data_is_valid() {
        let bindings = super::check_keybinds(None).expect("the committed keybind data has to be valid");
        // 19 shortcut entries + 9 emotion keys + 9 hotbar slots + 2 chat
        // command keys.
        assert_eq!(bindings, 39);
        assert_eq!(crate::input::keybinds::shortcuts().len(), 19);
        assert_eq!(crate::input::keybinds::hotbar_keys().len(), 9);
    }

    #[test]
    fn keybind_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-keybinds-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(shortcuts: [)").expect("failed to write the malformed fixture");

        let result = super::check_keybinds(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn alias_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/aliases.ron");
        let error = super::check_aliases(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn keybind_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/keybinds.ron");
        let error = super::check_keybinds(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn paper_doll_data_is_valid() {
        let slots = super::check_paper_doll(None).expect("the committed paper-doll data has to be valid");
        assert_eq!(slots, 21, "one slot per equipment position (11 classic + 10 costume/shadow)");
    }

    #[test]
    fn paper_doll_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-paper-doll-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(slots: [)").expect("failed to write the malformed fixture");

        let result = super::check_paper_doll(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn paper_doll_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/paper_doll_slots.ron");
        let error = super::check_paper_doll(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    #[test]
    fn skill_behaviour_data_is_valid() {
        let summary = super::check_skills(None).expect("the committed skill data has to be valid");
        assert_eq!(summary.behaviours, 2, "Rolling Cutter and Free Cast");
        assert_eq!(summary.clouds, 2, "the cloud row covers both Storm Gust ids");
        assert!(crate::world::skills::channels(SkillId(2036)));
        assert!(crate::world::skills::grants_move_while_casting(SkillId(278)));
    }

    #[test]
    fn skill_behaviour_check_rejects_malformed_data() {
        let directory = std::env::temp_dir();
        let path = directory.join(format!("ro-next-skills-check-{}.ron", std::process::id()));
        std::fs::write(&path, "(behaviours: [)").expect("failed to write the malformed fixture");

        let result = super::check_skills(Some(&path));
        let _ = std::fs::remove_file(&path);

        let error = result.expect_err("malformed data must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn skill_behaviour_check_reports_a_missing_file() {
        let path = std::path::Path::new("/nonexistent/ro-next/skills.ron");
        let error = super::check_skills(Some(path)).expect_err("a missing file must be rejected");
        assert!(error.contains("failed to read"), "{error}");
    }

    /// Regression test for the logout -> re-select black screen: after logging
    /// out the client keeps the name of the last map but has no loaded map and
    /// is in `MapLoading`. A `ChangeMap` with the same name is a *fresh map
    /// entry* in that situation and must take the full map load path instead of
    /// the same-map teleport shortcut (which skipped the load and left the
    /// client on a black screen).
    #[test]
    fn fresh_character_map_entry_is_not_a_same_map_teleport() {
        assert!(!is_same_map_teleport(
            Some("prontera"),
            "prontera",
            false,
            SessionState::MapLoading
        ));
        // Even if a menu background map finished loading, the session is still
        // in the map entry and the character map has to be loaded.
        assert!(!is_same_map_teleport(
            Some("prontera"),
            "prontera",
            true,
            SessionState::MapLoading
        ));
        assert!(!is_same_map_teleport(None, "prontera", false, SessionState::MapLoading));
    }

    #[test]
    fn in_game_same_map_move_is_a_teleport() {
        assert!(is_same_map_teleport(Some("prontera"), "prontera", true, SessionState::InGame));
        // Different map, missing map data or not in the game: never a
        // same-map teleport.
        assert!(!is_same_map_teleport(Some("prontera"), "geffen", true, SessionState::InGame));
        assert!(!is_same_map_teleport(Some("prontera"), "prontera", false, SessionState::InGame));
        assert!(!is_same_map_teleport(
            Some("prontera"),
            "prontera",
            true,
            SessionState::CharSelect
        ));
    }

    /// The little-endian numeric value an enum is serialized to; the message
    /// table keys are exactly this wire value.
    fn wire_code<T: ToBytes>(value: &T) -> u16 {
        let mut writer = ByteWriter::new();
        value.to_bytes(&mut writer).expect("test-value encoding cannot fail");
        let bytes = writer.into_inner();
        match bytes.len() {
            1 => u16::from(bytes[0]),
            2 => u16::from_le_bytes([bytes[0], bytes[1]]),
            4 => u16::try_from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])).expect("test code fits into u16"),
            length => panic!("unexpected enum width: {length}"),
        }
    }

    #[test]
    fn buy_item_result_codes_match_the_wire_values() {
        for (result, code) in [
            (BuyItemResult::NotEoughZeny, 1),
            (BuyItemResult::WeightLimitExceeded, 2),
            (BuyItemResult::TooManyItems, 3),
            (BuyItemResult::TooManyOfThisItem, 9),
            (BuyItemResult::PropsOpenAir, 10),
            (BuyItemResult::ExchangeFailed, 11),
            (BuyItemResult::ExchangeWellDone, 12),
            (BuyItemResult::ItemSoldOut, 13),
            (BuyItemResult::NotEnoughGoods, 14),
        ] {
            assert_eq!(buy_item_result_code(&result), code);
            assert_eq!(wire_code(&result), code, "wire drift for {result:?}");
        }
    }

    #[test]
    fn ammunition_action_codes_match_the_wire_values() {
        for (action, code) in [
            (AmmunitionActionType::EquipProperAmmunitionFirst, 0),
            (AmmunitionActionType::WeightLimitExceeded1, 1),
            (AmmunitionActionType::WeightLimitExceeded2, 2),
            (AmmunitionActionType::Equipped, 3),
        ] {
            assert_eq!(ammunition_action_code(&action), code);
            assert_eq!(wire_code(&action), code, "wire drift for {action:?}");
        }
    }
    #[test]
    fn cutin_names_resolve_to_the_classic_illust_directory_first() {
        let candidates = cutin_texture_candidates("kafra_01");

        assert_eq!(candidates[0], "유저인터페이스\\illust\\kafra_01.bmp");
        assert!(!candidates.is_empty());
        // Monster/NPC portrait fallbacks are present for servers that send a
        // sprite name instead of a cut-in name.
        assert!(candidates.iter().any(|candidate| candidate == "monster\\kafra_01.bmp"));
        assert!(candidates.iter().any(|candidate| candidate == "npc\\kafra_01.bmp"));
    }

    #[test]
    fn cutin_paths_with_extension_are_kept_verbatim() {
        let candidates = cutin_texture_candidates("monster\\1002.bmp");
        assert_eq!(candidates[0], "monster\\1002.bmp");
        assert!(cutin_texture_candidates("   ").is_empty());
    }

    #[test]
    fn book_pages_drop_the_background_command_and_split_by_line_count() {
        let mut text = String::from("%faf0e6\r\n");
        for index in 0..32 {
            text.push_str(&format!("line {index}\r\n"));
        }

        let pages = book_pages(&text);
        assert_eq!(pages.len(), 3);
        assert_eq!(pages[0].len(), crate::interface::windows::book::BOOK_LINES_PER_PAGE);
        assert_eq!(pages[0][0], "line 0");
        assert_eq!(pages[2].len(), 2);
        assert_eq!(pages[2][1], "line 31");
        assert!(book_pages("").is_empty());
    }
}
