//! Regression tests for the Phase 6 data-driven world/audio/client-feedback
//! configuration.
//!
//! These are repo-level guards, separate from the schema tests inside
//! `client_config/*` and `state/localization`:
//!
//! * the hard-coded constants that used to live in `lib.rs` (`DEFAULT_MAP`,
//!   `START_CAMERA_FOCUS_POINT`, `UI_BACKGROUND_MUSIC`,
//!   `MAIN_MENU_CLICK_SOUND_EFFECT`, `TELEPORT_VANISH_SOUND_EFFECT`) are gone
//!   and the data accessors are used instead,
//! * the debug map array is gone from `windows/maps.rs` (and the window stays
//!   debug-only),
//! * the quest-icon path/colour literals are gone from
//!   `world/particles/mod.rs`,
//! * the client chat/error feedback literals are gone from `lib.rs`; their
//!   templates live in the two language files and are rendered through the
//!   shared `korangar_messages::render_template`,
//! * the committed data files carry the previous values and the `--check-*` CLI
//!   entry points accept them.

use std::path::PathBuf;

/// The client source that used to hold the configuration constants and the
/// chat feedback literals.
const LIB_RS: &str = include_str!("../src/lib.rs");
/// The debug map window that used to hold the warp array.
const MAPS_RS: &str = include_str!("../src/interface/windows/maps.rs");
/// The window module registry (proves the Maps window is still debug-only).
const WINDOWS_MOD_RS: &str = include_str!("../src/interface/windows/mod.rs");
/// The particle module that used to hold the quest icon path/colours.
const PARTICLES_RS: &str = include_str!("../src/world/particles/mod.rs");
/// The committed runtime data files.
const WORLD_CONFIG_RON: &str = include_str!("../archive/data/client/world_config.ron");
const UI_AUDIO_RON: &str = include_str!("../archive/data/client/ui_audio.ron");
const DEBUG_MAPS_RON: &str = include_str!("../archive/data/client/debug_maps.ron");
/// The localization module (the `experience_labels` helper reads the four
/// experience labels from there).
const LOCALIZATION_RS: &str = include_str!("../src/state/localization/mod.rs");
/// The two language files the chat feedback templates live in.
const EN_US_RON: &str = include_str!("../archive/data/languages/en-US.ron");
const DE_DE_RON: &str = include_str!("../archive/data/languages/de-DE.ron");

/// Constants that were removed from `lib.rs`; the values live in the data
/// files now.
const REMOVED_CONFIG_CONSTANTS: &[&str] = &[
    "DEFAULT_MAP",
    "START_CAMERA_FOCUS_POINT",
    "UI_BACKGROUND_MUSIC",
    "MAIN_MENU_CLICK_SOUND_EFFECT",
    "TELEPORT_VANISH_SOUND_EFFECT",
];

/// The chat feedback templates (`state/localization` keys) with the exact
/// English text the previous hard-coded literals rendered.
///
/// The list is the behaviour-preservation snapshot: if one of these texts
/// changes it has to be an explicit localization decision, not a refactor side
/// effect.
const CHAT_FEEDBACK: &[(&str, &str)] = &[
    ("chat_aura_style_set", "Aura style set to \\\"{style}\\\"."),
    (
        "chat_party_invite_ignored",
        "Ignoring party invitation from \\\"{name}\\\": you are already in a party.",
    ),
    ("chat_cannot_raise_stat", "You cannot raise that stat right now."),
    ("chat_players_online", "{count} players are online."),
    (
        "chat_map_server_refused_login",
        "The map server refused the login (error {error}).",
    ),
    ("chat_skill_passive", "Skill {skill} is a passive skill and cannot be cast."),
    ("chat_aura_usage", "Usage: /aura <style>. Available styles: {styles}."),
    (
        "chat_aura_unknown",
        "Unknown aura style \\\"{style}\\\". Available styles: {styles}.",
    ),
    ("chat_party_usage", "Usage: /p <message>"),
    ("chat_hotbar_page", "Hotbar page {page}/{pages}."),
    ("chat_hotkey_item_missing", "The item of that hotkey is not in your inventory."),
    ("chat_skill_not_learned", "You have not learned skill {skill}."),
    ("chat_skill_target_out_of_range", "Skill {skill}: the target is out of range."),
    ("chat_skill_target_gone", "Skill {skill}: the target is no longer in view."),
    ("chat_skill_no_target", "Skill {skill}: select a target first."),
    ("chat_skill_cell_out_of_range", "Skill {skill}: that cell is out of range."),
    (
        "chat_skill_position_unknown",
        "Skill {skill}: the player position is not known yet.",
    ),
    (
        "chat_skill_click_ground",
        "Skill {skill}: click a cell on the ground to cast it.",
    ),
    ("chat_experience_gained", "Gained {gains}."),
    ("chat_experience_base", "base EXP"),
    ("chat_experience_job", "job EXP"),
    ("chat_experience_quest_base", "base EXP (quest)"),
    ("chat_experience_quest_job", "job EXP (quest)"),
    ("chat_item_obtained", "You got {name} ({quantity})."),
    ("chat_sell_failed", "Failed to sell items"),
    (
        "error_map_load_failed",
        "Failed to load the map '{map_name}'. Please try again.",
    ),
    ("error_session_expired", "Your session expired. Please log in again."),
    (
        "error_character_selection_missing_login_data",
        "Character selection failed: missing login data.",
    ),
    (
        "error_character_selection_missing_slot",
        "Character selection failed: the server did not send that character slot.",
    ),
    ("error_character_slot_switch_failed", "Failed to switch character slots"),
    (
        "error_map_server_auth_failed",
        "The map server rejected the connection (reason {reason}). Please try again.",
    ),
    (
        "error_map_data_missing",
        "Map data missing: `{map_name}`\\nThis map is not in the game archives. Please log out and choose another character.",
    ),
];

/// Literal fragments of the previous chat feedback strings; none of them may
/// come back into `lib.rs`.
const REMOVED_CHAT_LITERALS: &[&str] = &[
    "players are online.",
    "You cannot raise that stat right now.",
    "has not learned skill",
    "select a target first.",
    "the target is out of range.",
    "the target is no longer in view.",
    "that cell is out of range.",
    "player position is not known yet.",
    "click a cell on the ground to cast it.",
    "Hotbar page ",
    "not in your inventory.",
    "Usage: /p <message>",
    "Usage: /aura <style>",
    "Unknown aura style",
    "Aura style set to",
    "Ignoring party invitation from",
    "The map server refused the login (error",
    "is a passive skill and cannot be cast.",
    "You are not in a party.",
    "You got {name}",
    "Gained {",
    "base EXP",
    "job EXP",
    "Failed to sell items",
    "Failed to load the map",
    "Your session expired",
    "Character selection failed",
    "Failed to switch character slots",
    "The map server rejected the connection",
    "Map data missing",
];

/// Writes `content` to a unique temporary file and returns its path.
fn temporary_file(name: &str, content: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("korangar-p3-{}-{name}", std::process::id()));
    std::fs::write(&path, content).expect("temporary data file has to be writable");
    path
}

#[test]
fn config_constants_are_gone_and_data_accessors_are_used() {
    for constant in REMOVED_CONFIG_CONSTANTS {
        assert!(
            !LIB_RS.contains(constant),
            "korangar/src/lib.rs still contains the hard-coded constant {constant}; it belongs in a client_config data file"
        );
    }

    for usage in [
        "crate::client_config::default_map()",
        "crate::client_config::start_camera_focus()",
        "crate::client_config::background_music()",
        "crate::client_config::main_menu_click_sound()",
        "crate::client_config::teleport_vanish_sound()",
    ] {
        assert!(LIB_RS.contains(usage), "lib.rs has to use {usage}");
    }
}

#[test]
fn debug_map_window_is_data_driven_and_stays_debug_only() {
    // The previous array and its 23 coordinate pairs are gone from the window.
    assert!(
        !MAPS_RS.contains("MAP_WARPS"),
        "the debug warp array has to live in debug_maps.ron"
    );
    assert!(
        !MAPS_RS.contains("const MAP_COUNT"),
        "the debug warp count has to come from the data file"
    );
    for coordinate in ["TilePosition { x: 119, y: 59 }", "(\"geffen\"", "(\"mid_camp\""] {
        assert!(
            !MAPS_RS.contains(coordinate),
            "windows/maps.rs still hardcodes the debug warp {coordinate:?}"
        );
    }
    assert!(
        MAPS_RS.contains("crate::client_config::warps()"),
        "the Maps window has to read the warp list from the data file"
    );

    // The window (and therefore the runtime warp registry) stays debug-only.
    assert!(
        WINDOWS_MOD_RS.contains("#[cfg(feature = \"debug\")]\nmod maps;"),
        "the Maps window has to stay behind the debug feature"
    );
    assert!(
        !DEBUG_MAPS_RON.is_empty() && DEBUG_MAPS_RON.contains("debug-only"),
        "debug_maps.ron has to document that it is a debug-only convenience list"
    );
}

#[test]
fn quest_icon_is_data_driven() {
    for literal in [
        "유저인터페이스\\\\minimap",
        "QuestColor::Yellow",
        "QuestColor::Orange",
        "QuestColor::Green",
        "QuestColor::Purple",
        "Color::rgb_u8(200, 200, 30)",
        "Color::rgb_u8(200, 100, 30)",
        "Color::rgb_u8(30, 200, 30)",
        "Color::rgb_u8(200, 30, 200)",
    ] {
        assert!(
            !PARTICLES_RS.contains(literal),
            "world/particles/mod.rs still hardcodes the quest icon value {literal:?}; it belongs in world_config.ron"
        );
    }

    assert!(PARTICLES_RS.contains("crate::client_config::quest_icon_texture("));
    assert!(PARTICLES_RS.contains("crate::client_config::quest_icon_color("));

    // The committed data file carries the previous path template and tints.
    assert!(WORLD_CONFIG_RON.contains(r#"texture: "유저인터페이스\\minimap\\quest_{effect}_{frame}.bmp""#));
    assert!(WORLD_CONFIG_RON.contains("frame: 1,"));
    for colour in [
        "(color: Yellow, rgb: (200, 200, 30))",
        "(color: Orange, rgb: (200, 100, 30))",
        "(color: Green, rgb: (30, 200, 30))",
        "(color: Purple, rgb: (200, 30, 200))",
    ] {
        assert!(WORLD_CONFIG_RON.contains(colour), "world_config.ron is missing {colour}");
    }
}

#[test]
fn world_and_audio_data_carry_the_previous_values() {
    let world = korangar::check_world_config(None).expect("the committed world config has to be valid");
    assert_eq!(world.default_map, "geffen");
    assert_eq!(world.quest_colors, 4);
    assert!(WORLD_CONFIG_RON.contains("start_camera_focus: (600.0, 0.0, 240.0)"));

    let audio = korangar::check_ui_audio(None).expect("the committed UI audio data has to be valid");
    assert_eq!(audio.background_music.as_deref(), Some("bgm\\01.mp3"));
    assert_eq!(audio.sound_effects, 2);
    assert!(UI_AUDIO_RON.contains(r#"main_menu_click_sound: "버튼소리.wav""#));
    assert!(UI_AUDIO_RON.contains(r#"teleport_vanish_sound: "effect\\ef_teleportation.wav""#));

    assert_eq!(
        korangar::check_debug_maps(None).expect("the committed debug map data has to be valid"),
        23
    );
    assert!(DEBUG_MAPS_RON.contains(r#"(map_name: "geffen", x: 119, y: 59)"#));
}

#[test]
fn data_files_are_loaded_through_the_archive_with_a_compiled_fallback() {
    for (module, path, fallback) in [
        ("world_config", "data\\\\client\\\\world_config.ron", "world_config.ron"),
        ("ui_audio", "data\\\\client\\\\ui_audio.ron", "ui_audio.ron"),
        ("debug_maps", "data\\\\client\\\\debug_maps.ron", "debug_maps.ron"),
    ] {
        assert!(
            LIB_RS.contains("crate::client_config::load(game_file_loader.as_ref())"),
            "the client start-up has to load the client_config data files"
        );
        let source = match module {
            "world_config" => include_str!("../src/client_config/world_config.rs"),
            "ui_audio" => include_str!("../src/client_config/ui_audio.rs"),
            _ => include_str!("../src/client_config/debug_maps.rs"),
        };
        assert!(source.contains(path), "{module} has to load {path} through the archive");
        assert!(
            source.contains(&format!("include_str!(\"../../archive/data/client/{fallback}\")")),
            "{module} has to embed {fallback} as the compiled fallback"
        );
        assert!(source.contains("deny_unknown_fields"), "{module} has to reject unknown fields");
    }
}

#[test]
fn data_cli_checks_accept_and_reject_files() {
    // Valid committed data via the PATH variant of the CLI entry points.
    let world_path = temporary_file(
        "world-config.ron",
        &WORLD_CONFIG_RON.replace("default_map: \"geffen\"", "default_map: \"prontera\""),
    );
    let world = korangar::check_world_config(Some(&world_path)).expect("a valid override has to pass");
    assert_eq!(world.default_map, "prontera");
    std::fs::remove_file(&world_path).ok();

    // Unknown fields are rejected loudly (deny_unknown_fields).
    let bad_world_path = temporary_file(
        "world-config-bad.ron",
        &WORLD_CONFIG_RON.replace("    version: 1,", "    version: 1,\n    unknown_field: true,"),
    );
    let error = korangar::check_world_config(Some(&bad_world_path)).expect_err("an unknown field has to be rejected");
    assert!(error.contains("unknown") || error.contains("Unexpected field"), "{error}");
    std::fs::remove_file(&bad_world_path).ok();

    let bad_audio_path = temporary_file(
        "ui-audio-bad.ron",
        &UI_AUDIO_RON.replace(
            r#"main_menu_click_sound: "버튼소리.wav""#,
            r#"main_menu_click_sound: "버튼소리.ogg""#,
        ),
    );
    let error = korangar::check_ui_audio(Some(&bad_audio_path)).expect_err("an undecodable extension has to be rejected");
    assert!(error.contains("main_menu_click_sound"), "{error}");
    std::fs::remove_file(&bad_audio_path).ok();

    let duplicate_maps_path = temporary_file(
        "debug-maps-bad.ron",
        &DEBUG_MAPS_RON.replace(
            r#"(map_name: "mid_camp", x: 180, y: 240),"#,
            r#"(map_name: "geffen", x: 180, y: 240),"#,
        ),
    );
    let error = korangar::check_debug_maps(Some(&duplicate_maps_path)).expect_err("a duplicate map has to be rejected");
    assert!(error.contains("duplicate debug warp"), "{error}");
    std::fs::remove_file(&duplicate_maps_path).ok();
}

#[test]
fn chat_feedback_literals_are_localized() {
    // No previous feedback literal may come back into lib.rs.
    for literal in REMOVED_CHAT_LITERALS {
        assert!(
            !LIB_RS.contains(literal),
            "korangar/src/lib.rs still contains the hardcoded chat feedback {literal:?}; it belongs in the localization files"
        );
    }

    // Every template is a localization key and is used at a call site.
    for (key, english) in CHAT_FEEDBACK {
        assert!(
            EN_US_RON.contains(&format!("    {key}: \"{english}\",")),
            "en-US.ron is missing or changed the {key} template (expected: {english})"
        );
        assert!(
            DE_DE_RON.contains(&format!("    {key}: ")),
            "de-DE.ron is missing the localized {key} key"
        );
        // The four experience labels are read in bulk by
        // `Localization::experience_labels`, everything else through its path.
        let read_directly = key.starts_with("chat_experience_") && *key != "chat_experience_gained";
        if read_directly {
            assert!(
                LOCALIZATION_RS.contains(&format!("self.{key}.as_str()")),
                "experience_labels() has to read the localization key {key}"
            );
        } else {
            assert!(
                LIB_RS.contains(&format!(".{key}()")),
                "lib.rs has to render the client feedback through the localization key {key}"
            );
        }
    }

    // The reused "not in a party" text is an existing localization key.
    assert!(LIB_RS.contains(".not_in_party_text()"));

    // The chat feedback uses the shared placeholder renderer of the message
    // crate instead of a second message table.
    assert!(
        LIB_RS.contains("korangar_messages::render_template("),
        "the chat feedback templates have to use korangar_messages::render_template"
    );
    assert!(
        LIB_RS.contains("fn push_localized_chat"),
        "the chat feedback has to go through one localized push helper"
    );
    assert!(
        LIB_RS.contains("fn render_localized"),
        "the error-window feedback has to go through one localized render helper"
    );
    assert!(
        LIB_RS.contains("experience_labels()"),
        "the experience labels have to use the localization helper"
    );
}
