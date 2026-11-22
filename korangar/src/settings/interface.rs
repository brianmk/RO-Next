#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::loaders::Scaling;
use crate::state::inventory::ItemCategory;
use crate::state::localization::Language;

/// This theme name includes a zero byte so that it can not point to an actual
/// file. This is guaranteed to fail to load which will automatically fall back
/// to the default theme. The only issue is that loading the default theme will
/// cause an error to appear when running Korangar with debug features.
pub const DEFAULT_THEME_NAME: &str = "^000001default^000000\0";
/// Name of the built in classic Ragnarok Online theme.
///
/// This is not a file on disk,
/// [`InterfaceTheme::load`](crate::state::theme::InterfaceTheme::load)
/// recognizes the name and returns the built in preset.
pub const CLASSIC_THEME_NAME: &str = "classic";
pub const MENU_THEMES_PATH: &str = "client/menu_themes";
pub const IN_GAME_THEMES_PATH: &str = "client/in_game_themes";
pub const WORLD_THEMES_PATH: &str = "client/world_themes";

/// Style of the classic level-99 aura.
///
/// `Auto` uses the classic tiers (cyan for level 99, violet for transcendent
/// level 99, blue for 100+, gold for 175+). `Off` disables the aura. The
/// explicit variants force one style for all auras.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum AuraStyle {
    /// Classic tier-based aura (default).
    #[default]
    Auto,
    /// No aura at all.
    Off,
    Cyan,
    Violet,
    Blue,
    Gold,
    /// Official green level-99 aura variant (EF_LEVEL99 size type 7).
    Green,
}

impl AuraStyle {
    /// All styles, in the order shown in the interface settings dropdown.
    pub const OPTIONS: [AuraStyle; 7] = [
        AuraStyle::Auto,
        AuraStyle::Off,
        AuraStyle::Cyan,
        AuraStyle::Violet,
        AuraStyle::Blue,
        AuraStyle::Gold,
        AuraStyle::Green,
    ];

    /// Encode the style for the shared atomic cell used by running aura
    /// effects.
    pub fn encode(self) -> u8 {
        match self {
            AuraStyle::Auto => 0,
            AuraStyle::Off => 1,
            AuraStyle::Cyan => 2,
            AuraStyle::Violet => 3,
            AuraStyle::Blue => 4,
            AuraStyle::Gold => 5,
            AuraStyle::Green => 6,
        }
    }

    /// Decode a style written by [`AuraStyle::encode`]. Unknown values fall
    /// back to [`AuraStyle::Auto`].
    pub fn decode(value: u8) -> AuraStyle {
        match value {
            1 => AuraStyle::Off,
            2 => AuraStyle::Cyan,
            3 => AuraStyle::Violet,
            4 => AuraStyle::Blue,
            5 => AuraStyle::Gold,
            6 => AuraStyle::Green,
            _ => AuraStyle::Auto,
        }
    }
}

impl DropDownItem<AuraStyle> for AuraStyle {
    fn text(&self) -> &str {
        match self {
            AuraStyle::Auto => "Auto (tier default)",
            AuraStyle::Off => "Off",
            AuraStyle::Cyan => "Cyan",
            AuraStyle::Violet => "Violet",
            AuraStyle::Blue => "Blue",
            AuraStyle::Gold => "Gold",
            AuraStyle::Green => "Green",
        }
    }

    fn value(&self) -> AuraStyle {
        *self
    }
}

/// Chat marker used by the server-side aura stylist NPC to ask the client to
/// change the aura style.
///
/// The client consumes and hides any incoming chat message that starts with
/// this marker, so the NPC can change the local aura style without a custom
/// packet (which would be packet-version dependent).
pub const AURA_STYLE_MARKER: &str = "[KORANGAR_AURA]";

/// Result of parsing an aura style marker chat message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuraStyleMarker {
    /// The message is an aura marker for a known style.
    Style(AuraStyle),
    /// The message starts with the aura marker but the style is unknown.
    ///
    /// The message is still consumed (and not shown in the chat) to keep the
    /// bridge private; the client only logs the unknown style in debug builds.
    UnknownStyle,
}

/// Parses an incoming chat message as an aura style marker.
///
/// Returns `None` if the message is not an aura marker at all (normal chat is
/// never touched). The marker has to be at the start of the message (leading
/// whitespace is ignored) and is matched case-insensitively; the style name is
/// trimmed and matched by [`AuraStyle::from_style_name`].
pub fn parse_aura_style_marker(text: &str) -> Option<AuraStyleMarker> {
    let text = text.trim_start();

    // The marker itself is ASCII, but a chat message can contain multi-byte
    // UTF-8 (Korean chat, accented names, ...). `split_at` panics when the byte
    // index falls inside a character, so use `get` and treat a non-boundary
    // index as "not an aura marker".
    let split_index = text.len().min(AURA_STYLE_MARKER.len());
    let prefix = text.get(..split_index)?;
    let rest = text.get(split_index..)?;

    if !prefix.eq_ignore_ascii_case(AURA_STYLE_MARKER) {
        return None;
    }

    let style_name = rest.trim();

    match AuraStyle::from_style_name(style_name) {
        Some(style) => Some(AuraStyleMarker::Style(style)),
        None => Some(AuraStyleMarker::UnknownStyle),
    }
}

impl AuraStyle {
    /// Canonical lower-case command name of the style.
    pub fn style_name(self) -> &'static str {
        match self {
            AuraStyle::Auto => "auto",
            AuraStyle::Off => "off",
            AuraStyle::Cyan => "cyan",
            AuraStyle::Violet => "violet",
            AuraStyle::Blue => "blue",
            AuraStyle::Gold => "gold",
            AuraStyle::Green => "green",
        }
    }

    /// Canonical command names of all styles, for usage/help messages.
    pub fn style_names() -> &'static str {
        "auto, off, cyan, violet, blue, gold, green"
    }

    /// Parses a style command name as used by the `/aura` command and the
    /// server-side aura stylist NPC bridge.
    ///
    /// All names are matched case-insensitively with surrounding whitespace
    /// ignored. Aliases are provided for the names players are likely to type.
    /// Adding a new variant only requires a new arm here (and in
    /// [`AuraStyle::style_name`], [`AuraStyle::style_names`] and
    /// [`AuraStyle::OPTIONS`]).
    pub fn from_style_name(name: &str) -> Option<AuraStyle> {
        let name = name.trim().to_ascii_lowercase();

        match name.as_str() {
            "auto" | "default" | "automatic" | "tier" => Some(AuraStyle::Auto),
            "off" | "none" | "disable" | "disabled" => Some(AuraStyle::Off),
            "cyan" | "aqua" => Some(AuraStyle::Cyan),
            "violet" | "purple" | "transcendent" | "trans" | "rebirth" => Some(AuraStyle::Violet),
            "blue" | "azure" => Some(AuraStyle::Blue),
            "gold" | "golden" | "yellow" => Some(AuraStyle::Gold),
            "green" | "emerald" => Some(AuraStyle::Green),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct InterfaceSettings {
    pub language: Language,
    pub scaling: Scaling,
    pub menu_theme: String,
    pub in_game_theme: String,
    pub world_theme: String,
    /// Show the names of player characters below them at all times, like the
    /// original client does. If this is disabled, a name is only shown while
    /// the character is hovered.
    #[serde(default = "default_show_character_names")]
    pub show_character_names: bool,
    /// Show chat messages as a floating bubble above the speaking player
    /// character (classic RO "subtitle" style), in addition to the chat window.
    #[serde(default = "default_show_chat_bubbles")]
    pub show_chat_bubbles: bool,
    /// Draw a dark background box behind overhead chat bubbles. If disabled,
    /// the text is only drawn with an outline.
    #[serde(default = "default_chat_bubble_background")]
    pub chat_bubble_background: bool,
    /// Show the name of a used skill above the caster's head, like the original
    /// client does (`Blessing !!`). Applies to the local player and to other
    /// visible characters (including population engine shells).
    #[serde(default = "default_show_skill_names_above_characters")]
    pub show_skill_names_above_characters: bool,
    /// Draw the chat window without a background so the game world is visible
    /// through it. Can be toggled in the chat window, persisted here.
    #[serde(default = "default_transparent_chat")]
    pub transparent_chat: bool,
    /// Draw the minimap window without its dark frame/background so only the
    /// map image is visible. Can be toggled with a button in the minimap
    /// window, persisted here.
    #[serde(default = "default_minimap_transparent_frame")]
    pub minimap_transparent_frame: bool,
    /// Draw the hotbar window without its background/chrome so the game world
    /// is visible through it. The slot cards, icons and overlays stay opaque.
    /// Can be toggled with a round button in the hotbar window, persisted here.
    #[serde(default = "default_transparent_hotbar")]
    pub transparent_hotbar: bool,
    /// Show the character overview window in its compact form (only HP, SP and
    /// base EXP progress, without the name, level rows and navigation
    /// buttons). Can be toggled in the window itself, persisted here.
    #[serde(default = "default_character_overview_compact")]
    pub character_overview_compact: bool,
    /// The last selected tab of the inventory window, like the original
    /// client remembers which tab was open.
    #[serde(default = "default_inventory_tab")]
    pub inventory_tab: ItemCategory,
    /// Style of the classic level-99 aura. Defaults to `Auto` (tier based) so
    /// old settings files load unchanged.
    #[serde(default)]
    pub level_99_aura_style: AuraStyle,
}

fn default_show_character_names() -> bool {
    true
}

fn default_show_chat_bubbles() -> bool {
    true
}

fn default_chat_bubble_background() -> bool {
    true
}

fn default_show_skill_names_above_characters() -> bool {
    true
}

fn default_transparent_chat() -> bool {
    false
}

fn default_minimap_transparent_frame() -> bool {
    false
}

fn default_transparent_hotbar() -> bool {
    false
}

fn default_character_overview_compact() -> bool {
    false
}

fn default_inventory_tab() -> ItemCategory {
    ItemCategory::Consumables
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            language: Language::English,
            scaling: Scaling::new(1.0),
            menu_theme: DEFAULT_THEME_NAME.to_string(),
            in_game_theme: DEFAULT_THEME_NAME.to_string(),
            world_theme: DEFAULT_THEME_NAME.to_string(),
            show_character_names: true,
            show_chat_bubbles: true,
            chat_bubble_background: true,
            show_skill_names_above_characters: true,
            transparent_chat: false,
            minimap_transparent_frame: false,
            transparent_hotbar: false,
            character_overview_compact: false,
            inventory_tab: ItemCategory::Consumables,
            level_99_aura_style: AuraStyle::Auto,
        }
    }
}

impl InterfaceSettings {
    const FILE_NAME: &'static str = "client/interface_settings.ron";

    pub fn new() -> Self {
        let mut settings = Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load interface settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        });

        settings.apply_environment_overrides();

        settings
    }

    /// Whether there is a saved interface settings file. This is used to decide
    /// whether a DPI based interface scaling default should be applied on the
    /// first start.
    pub fn exists() -> bool {
        std::fs::metadata(Self::FILE_NAME).is_ok()
    }

    /// Applies the `KORANGAR_UI_SCALE` environment override. It has to be a
    /// factor between 0.5 and 2.0, the value is snapped to the closest option
    /// that is offered in the interface settings window so that the drop down
    /// always has an entry for the selected value.
    fn apply_environment_overrides(&mut self) {
        if let Ok(value) = std::env::var("KORANGAR_UI_SCALE") {
            match value.trim().parse::<f32>() {
                Ok(factor) if (0.5..=2.0).contains(&factor) => {
                    self.scaling = Scaling::closest(factor);
                    #[cfg(feature = "debug")]
                    print_debug!(
                        "overriding interface scaling from environment: {} ({})",
                        value.magenta(),
                        self.scaling.get_factor().to_string().magenta()
                    );
                }
                _ => {
                    #[cfg(feature = "debug")]
                    print_debug!(
                        "[{}] ignoring invalid KORANGAR_UI_SCALE value {}, expected a factor between 0.5 and 2.0",
                        "warning".yellow(),
                        value.magenta()
                    );
                }
            }
        }
    }

    pub fn load() -> Option<Self> {
        crate::logging::log_debug!("settings: loading interface settings from {}", Self::FILE_NAME);
        let result = {
            #[cfg(feature = "debug")]
            print_debug!("loading interface settings from {}", Self::FILE_NAME.magenta());

            std::fs::read_to_string(Self::FILE_NAME)
                .ok()
                .and_then(|data| ron::from_str(&data).ok())
        };
        match &result {
            Some(_) => crate::logging::log_debug!("settings: loaded interface settings from {}", Self::FILE_NAME),
            None => crate::logging::log_warn!(
                "settings: could not load interface settings from {} (using defaults)",
                Self::FILE_NAME
            ),
        };
        result
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving interface settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save interface settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for InterfaceSettings {
    fn drop(&mut self) {
        self.save();
    }
}

#[derive(RustState, StateElement)]
pub struct InterfaceSettingsCapabilities {
    languages: Vec<Language>,
    scalings: Vec<Scaling>,
    menu_themes: Vec<String>,
    in_game_themes: Vec<String>,
    world_themes: Vec<String>,
    level_99_aura_styles: Vec<AuraStyle>,
}

impl InterfaceSettingsCapabilities {
    fn load_themes(directory: &str) -> Vec<String> {
        let has_classic_theme = directory != WORLD_THEMES_PATH;

        let mut themes = vec![DEFAULT_THEME_NAME.to_string()];
        if has_classic_theme {
            themes.push(CLASSIC_THEME_NAME.to_string());
        }

        if let Ok(entries) = std::fs::read_dir(directory) {
            themes.extend(
                entries
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| entry.file_name().to_string_lossy().strip_suffix(".ron").map(ToOwned::to_owned)),
            );

            // Sort themes excluding the built in ones since we always want those to be
            // first.
            let built_in_count = themes.len().min(2);
            themes[built_in_count..].sort_unstable();
        }

        themes
    }
}

impl Default for InterfaceSettingsCapabilities {
    fn default() -> Self {
        Self {
            // TODO: Don't hardcode this, load it from the disk instead.
            languages: vec![Language::English, Language::German],
            scalings: Scaling::OPTIONS.to_vec(),
            menu_themes: Self::load_themes(MENU_THEMES_PATH),
            in_game_themes: Self::load_themes(IN_GAME_THEMES_PATH),
            world_themes: Self::load_themes(WORLD_THEMES_PATH),
            level_99_aura_styles: AuraStyle::OPTIONS.to_vec(),
        }
    }
}

impl InterfaceSettingsCapabilities {
    /// Makes sure the currently selected scaling is part of the options that
    /// are shown in the interface settings window. This is needed when the
    /// scaling was set through the environment to a value that is not part of
    /// the default options.
    pub fn ensure_scaling(&mut self, scaling: Scaling) {
        if !self.scalings.contains(&scaling) {
            self.scalings.push(scaling);
            self.scalings
                .sort_by(|left, right| left.get_factor().total_cmp(&right.get_factor()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuraStyle, AuraStyleMarker, parse_aura_style_marker};

    #[test]
    fn style_names_roundtrip_through_the_parser() {
        for style in AuraStyle::OPTIONS {
            assert_eq!(AuraStyle::from_style_name(style.style_name()), Some(style));
        }
    }

    #[test]
    fn style_name_aliases_are_accepted() {
        assert_eq!(AuraStyle::from_style_name("  DEFAULT "), Some(AuraStyle::Auto));
        assert_eq!(AuraStyle::from_style_name("none"), Some(AuraStyle::Off));
        assert_eq!(AuraStyle::from_style_name("Purple"), Some(AuraStyle::Violet));
        assert_eq!(AuraStyle::from_style_name("transcendent"), Some(AuraStyle::Violet));
        assert_eq!(AuraStyle::from_style_name("AZURE"), Some(AuraStyle::Blue));
        assert_eq!(AuraStyle::from_style_name("golden"), Some(AuraStyle::Gold));
    }

    #[test]
    fn unknown_style_name_is_rejected() {
        assert_eq!(AuraStyle::from_style_name("pink"), None);
        assert_eq!(AuraStyle::from_style_name(""), None);
        assert_eq!(AuraStyle::from_style_name("cyan gold"), None);
    }

    #[test]
    fn aura_marker_parses_the_style() {
        assert_eq!(
            parse_aura_style_marker("[KORANGAR_AURA] cyan"),
            Some(AuraStyleMarker::Style(AuraStyle::Cyan))
        );
        assert_eq!(
            parse_aura_style_marker("  [KORANGAR_AURA]   GOLD  "),
            Some(AuraStyleMarker::Style(AuraStyle::Gold))
        );
        assert_eq!(
            parse_aura_style_marker("[korangar_aura] violet"),
            Some(AuraStyleMarker::Style(AuraStyle::Violet))
        );
    }

    #[test]
    fn aura_marker_with_unknown_style_is_consumed_as_unknown() {
        assert_eq!(
            parse_aura_style_marker("[KORANGAR_AURA] pink"),
            Some(AuraStyleMarker::UnknownStyle)
        );
        assert_eq!(parse_aura_style_marker("[KORANGAR_AURA]"), Some(AuraStyleMarker::UnknownStyle));
    }

    #[test]
    fn normal_chat_is_not_an_aura_marker() {
        assert_eq!(parse_aura_style_marker("hello [KORANGAR_AURA] cyan"), None);
        assert_eq!(parse_aura_style_marker("@go prontera"), None);
        assert_eq!(parse_aura_style_marker(""), None);
        assert_eq!(parse_aura_style_marker("[KORANGAR_AURA2] cyan"), None);
    }

    #[test]
    fn korean_chat_does_not_panic_on_the_byte_prefix() {
        // The prefix is cut at the byte length of the ASCII marker (15); that
        // byte index is in the middle of a 3-byte Korean character for many
        // chat lines (e.g. right after `"VerifyBot2 : "`).
        assert_eq!(parse_aura_style_marker("안녕하세요 한글채팅"), None);
        assert_eq!(parse_aura_style_marker("VerifyBot2 : 요요요요요"), None);
        assert_eq!(parse_aura_style_marker("도시… 아니 나라 수준의 규모"), None);
        assert_eq!(parse_aura_style_marker("Señorita"), None);
        // The marker still works when it directly follows multi-byte text.
        assert_eq!(parse_aura_style_marker("한글 [KORANGAR_AURA] cyan"), None);
    }
}
