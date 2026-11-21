#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::loaders::Scaling;
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
    /// Draw the chat window without a background so the game world is visible
    /// through it. Can be toggled in the chat window, persisted here.
    #[serde(default = "default_transparent_chat")]
    pub transparent_chat: bool,
    /// Draw the minimap window without its dark frame/background so only the
    /// map image is visible. Can be toggled with a button in the minimap
    /// window, persisted here.
    #[serde(default = "default_minimap_transparent_frame")]
    pub minimap_transparent_frame: bool,
    /// Show the character overview window in its compact form (only HP, SP and
    /// base EXP progress, without the name, level rows and navigation
    /// buttons). Can be toggled in the window itself, persisted here.
    #[serde(default = "default_character_overview_compact")]
    pub character_overview_compact: bool,
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

fn default_transparent_chat() -> bool {
    false
}

fn default_minimap_transparent_frame() -> bool {
    false
}

fn default_character_overview_compact() -> bool {
    false
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
            transparent_chat: false,
            minimap_transparent_frame: false,
            character_overview_compact: false,
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
