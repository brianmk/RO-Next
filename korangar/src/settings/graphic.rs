#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{
    DEFAULT_BACKGROUND_MAX_FPS, DEFAULT_MAX_FPS, FRAMERATE_PRESETS, FramesPerSecond, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing,
    ShadowDetail, ShadowMethod, ShadowResolution, Ssaa, TextureSamplerType, WindowResolution,
};
use crate::state::localization::dropdown_keys;

/// Environment variable that overrides [`GraphicsSettings::max_fps`].
pub const MAX_FPS_ENVIRONMENT_VARIABLE: &str = "KORANGAR_MAX_FPS";
/// Environment variable that overrides
/// [`GraphicsSettings::background_max_fps`].
pub const BACKGROUND_MAX_FPS_ENVIRONMENT_VARIABLE: &str = "KORANGAR_BACKGROUND_MAX_FPS";
/// If set to a truthy value (`1`, `true`, `yes`, `on`), the window is always
/// treated as being in the background. This is useful for headless or
/// background verification clients that never receive keyboard focus.
pub const FORCE_UNFOCUSED_ENVIRONMENT_VARIABLE: &str = "KORANGAR_FORCE_UNFOCUSED";
/// Alias of [`FORCE_UNFOCUSED_ENVIRONMENT_VARIABLE`].
pub const FORCE_UNFOCUSED_ALIAS_ENVIRONMENT_VARIABLE: &str = "KORANGAR_UNFOCUSED_ALWAYS";
/// If set to a truthy value, the effective frame rate and the active limit are
/// logged every [`FRAMERATE_LOG_INTERVAL_SECONDS`] seconds.
pub const FRAMERATE_LOG_ENVIRONMENT_VARIABLE: &str = "KORANGAR_FPS_LOG";
/// Interval between two frame rate log lines.
pub const FRAMERATE_LOG_INTERVAL_SECONDS: u64 = 5;

/// Parses a frame rate limit from a string.
///
/// Accepts plain numbers (`60`) as well as the special values `unlimited`,
/// `off` and `none` (all meaning `0`).
pub fn parse_frames_per_second(value: &str) -> Option<FramesPerSecond> {
    let value = value.trim();

    if value.is_empty() {
        return None;
    }

    match value.to_ascii_lowercase().as_str() {
        "unlimited" | "off" | "none" | "null" => Some(FramesPerSecond::UNLIMITED),
        _ => value
            .parse::<u32>()
            .ok()
            .filter(|frames_per_second| *frames_per_second <= 1000)
            .map(FramesPerSecond::new),
    }
}

fn default_max_fps() -> FramesPerSecond {
    DEFAULT_MAX_FPS
}

fn default_background_max_fps() -> FramesPerSecond {
    DEFAULT_BACKGROUND_MAX_FPS
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GraphicsSettings {
    pub lighting_mode: LightingMode,
    pub vsync: bool,
    /// Maximum number of frames per second while the client window is focused.
    ///
    /// `0` means unlimited. Defaults to 60 FPS. When the settings file from an
    /// older client version does not contain this field, the default is used.
    #[serde(default = "default_max_fps")]
    pub max_fps: FramesPerSecond,
    /// Maximum number of frames per second while the client window is in the
    /// background (unfocused, occluded or minimized).
    ///
    /// `0` means unlimited. Defaults to 30 FPS.
    #[serde(default = "default_background_max_fps")]
    pub background_max_fps: FramesPerSecond,
    pub triple_buffering: bool,
    pub texture_filtering: TextureSamplerType,
    pub msaa: Msaa,
    pub ssaa: Ssaa,
    pub screen_space_anti_aliasing: ScreenSpaceAntiAliasing,
    pub shadow_method: ShadowMethod,
    pub shadow_resolution: ShadowResolution,
    pub shadow_detail: ShadowDetail,
    pub sdsm: bool,
    pub high_quality_interface: bool,
    /// The resolution of the game window. This is applied while the client is
    /// running, there is no need to restart it.
    #[serde(default = "default_window_resolution")]
    pub window_resolution: WindowResolution,
    /// Whether the window is shown in borderless fullscreen mode. If this is
    /// enabled, [`Self::window_resolution`] is ignored.
    #[serde(default)]
    pub fullscreen: bool,
}

fn default_window_resolution() -> WindowResolution {
    WindowResolution::R1280x720
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            lighting_mode: LightingMode::Enhanced,
            vsync: true,
            max_fps: DEFAULT_MAX_FPS,
            background_max_fps: DEFAULT_BACKGROUND_MAX_FPS,
            triple_buffering: true,
            texture_filtering: TextureSamplerType::Anisotropic(4),
            msaa: Msaa::X4,
            ssaa: Ssaa::Off,
            screen_space_anti_aliasing: ScreenSpaceAntiAliasing::Off,
            shadow_method: ShadowMethod::SoftPCSS,
            shadow_resolution: ShadowResolution::Normal,
            shadow_detail: ShadowDetail::Medium,
            sdsm: true,
            high_quality_interface: true,
            window_resolution: default_window_resolution(),
            fullscreen: false,
        }
    }
}

impl GraphicsSettings {
    const FILE_NAME: &'static str = "client/graphics_settings.ron";

    pub fn new() -> Self {
        let mut settings = Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load graphics settings from {}", Self::FILE_NAME.magenta());

            Default::default()
        });

        settings.apply_environment_overrides();
        settings.sanitize();

        settings
    }

    /// Replaces settings that cannot be used by the current build. A hand
    /// edited settings file could select an anti-aliasing mode that is not
    /// implemented (DLSS), in which case the client falls back to no
    /// anti-aliasing instead of failing to start.
    fn sanitize(&mut self) {
        if self.screen_space_anti_aliasing == ScreenSpaceAntiAliasing::Dlss {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] DLSS is not available in this build, falling back to no anti-aliasing",
                "warning".yellow()
            );

            self.screen_space_anti_aliasing = ScreenSpaceAntiAliasing::Off;
        }
    }

    /// Applies the environment overrides for the settings that are useful for
    /// automated or temporary test setups. They take precedence over the
    /// values stored in the settings file (but the file is still written on
    /// exit, so the overrides become the new saved settings).
    ///
    /// - `KORANGAR_RESOLUTION`: `WIDTHxHEIGHT` (for example `1920x1080`) or
    ///   `desktop` to keep the window maximized.
    /// - `KORANGAR_FULLSCREEN`: `1`/`true` for borderless fullscreen, anything
    ///   else disables it.
    fn apply_display_environment_overrides(&mut self) {
        if let Ok(value) = std::env::var("KORANGAR_RESOLUTION") {
            if let Some(window_resolution) = WindowResolution::parse(&value) {
                self.window_resolution = window_resolution;
                #[cfg(feature = "debug")]
                print_debug!("overriding window resolution from environment: {}", value.magenta());
            } else {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] ignoring invalid KORANGAR_RESOLUTION value {}",
                    "warning".yellow(),
                    value.magenta()
                );
            }
        }

        if let Ok(value) = std::env::var("KORANGAR_FULLSCREEN") {
            let value = value.trim();
            self.fullscreen = matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on");
            #[cfg(feature = "debug")]
            print_debug!("overriding fullscreen mode from environment: {}", value.magenta());
        }
    }

    /// The frame rate limit that applies right now, depending on whether the
    /// client window is currently in the background.
    pub fn effective_max_fps(&self, background: bool) -> u32 {
        match background {
            true => self.background_max_fps.get(),
            false => self.max_fps.get(),
        }
    }

    /// Applies the frame rate related environment overrides.
    ///
    /// The overrides take precedence over the persisted settings. They are only
    /// applied once at startup, so changing the values in the settings window
    /// afterwards still works as expected. Returns `true` if at least one value
    /// was overridden.
    pub fn apply_environment_overrides(&mut self) -> bool {
        // Display/UI overrides first (resolution, borderless fullscreen).
        self.apply_display_environment_overrides();

        let mut overridden = false;

        if let Some(frames_per_second) = Self::environment_frames_per_second(MAX_FPS_ENVIRONMENT_VARIABLE) {
            self.max_fps = frames_per_second;
            overridden = true;
        }

        if let Some(frames_per_second) = Self::environment_frames_per_second(BACKGROUND_MAX_FPS_ENVIRONMENT_VARIABLE) {
            self.background_max_fps = frames_per_second;
            overridden = true;
        }

        overridden
    }

    fn environment_frames_per_second(variable: &str) -> Option<FramesPerSecond> {
        let value = std::env::var(variable).ok()?;

        match parse_frames_per_second(&value) {
            Some(frames_per_second) => Some(frames_per_second),
            None => {
                eprintln!("[framerate] ignoring invalid {variable} value {value:?}");
                None
            }
        }
    }

    pub fn load() -> Option<Self> {
        crate::logging::log_debug!("settings: loading graphics settings from {}", Self::FILE_NAME);
        let result = {
            #[cfg(feature = "debug")]
            print_debug!("loading graphics settings from {}", Self::FILE_NAME.magenta());

            std::fs::read_to_string(Self::FILE_NAME)
                .ok()
                .and_then(|data| ron::from_str(&data).ok())
        };
        match &result {
            Some(_) => crate::logging::log_debug!("settings: loaded graphics settings from {}", Self::FILE_NAME),
            None => crate::logging::log_warn!(
                "settings: could not load graphics settings from {} (using defaults)",
                Self::FILE_NAME
            ),
        };
        result
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving graphics settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save graphics settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for GraphicsSettings {
    fn drop(&mut self) {
        self.save();
    }
}

/// The lighting mode used when rendering the game.
#[derive(Copy, Clone, PartialEq, Eq, Serialize, Deserialize, StateElement)]
pub enum LightingMode {
    /// Mode that mimics the way the original client rendered the game.
    Classic,
    /// Mode that enabled all enhanced graphics features.
    Enhanced,
}

impl DropDownItem<LightingMode> for LightingMode {
    fn text(&self) -> &str {
        match self {
            LightingMode::Classic => "Classic",
            LightingMode::Enhanced => "Enhanced",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            LightingMode::Classic => dropdown_keys::LIGHTING_CLASSIC,
            LightingMode::Enhanced => dropdown_keys::LIGHTING_ENHANCED,
        };
        Some(key)
    }

    fn value(&self) -> LightingMode {
        *self
    }
}

#[derive(RustState, StateElement)]
pub struct GraphicsSettingsCapabilities {
    lighting_modes: Vec<LightingMode>,
    texture_filtering_options: Vec<TextureSamplerType>,
    max_fps_options: Vec<FramesPerSecond>,
    background_max_fps_options: Vec<FramesPerSecond>,
    supported_msaa: Vec<Msaa>,
    ssaa_options: Vec<Ssaa>,
    screen_space_anti_aliasing_options: Vec<ScreenSpaceAntiAliasing>,
    shadow_method_options: Vec<ShadowMethod>,
    shadow_resolution_options: Vec<ShadowResolution>,
    shadow_detail_options: Vec<ShadowDetail>,
    window_resolution_options: Vec<WindowResolution>,
    vsync_setting_disabled: bool,
}

impl Default for GraphicsSettingsCapabilities {
    fn default() -> Self {
        Self {
            lighting_modes: vec![LightingMode::Classic, LightingMode::Enhanced],
            texture_filtering_options: vec![
                TextureSamplerType::Nearest,
                TextureSamplerType::Linear,
                TextureSamplerType::Anisotropic(4),
                TextureSamplerType::Anisotropic(8),
                TextureSamplerType::Anisotropic(16),
            ],
            max_fps_options: FRAMERATE_PRESETS.to_vec(),
            background_max_fps_options: FRAMERATE_PRESETS.to_vec(),
            supported_msaa: Vec::new(),
            ssaa_options: vec![Ssaa::Off, Ssaa::X2, Ssaa::X3, Ssaa::X4],
            screen_space_anti_aliasing_options: vec![
                ScreenSpaceAntiAliasing::Off,
                ScreenSpaceAntiAliasing::Fxaa,
                ScreenSpaceAntiAliasing::Smaa,
                // DLSS needs the NVIDIA NGX SDK/runtime and is not implemented in this
                // build. It is shown in the drop down, but disabled with a tooltip.
                ScreenSpaceAntiAliasing::Dlss,
            ],
            shadow_method_options: vec![ShadowMethod::Hard, ShadowMethod::SoftPCF, ShadowMethod::SoftPCSS],
            shadow_resolution_options: vec![ShadowResolution::Normal, ShadowResolution::Ultra, ShadowResolution::Insane],
            shadow_detail_options: vec![ShadowDetail::Low, ShadowDetail::Medium, ShadowDetail::High, ShadowDetail::Ultra],
            window_resolution_options: WindowResolution::OPTIONS.to_vec(),
            vsync_setting_disabled: true,
        }
    }
}

impl GraphicsSettingsCapabilities {
    pub fn update(&mut self, supported_msaa: Vec<Msaa>, present_mode_info: PresentModeInfo) {
        self.supported_msaa = supported_msaa;
        self.vsync_setting_disabled = !present_mode_info.supports_mailbox && !present_mode_info.supports_immediate;
    }

    /// Makes sure the currently selected resolution is part of the options that
    /// are shown in the graphics settings window. This is needed when the
    /// resolution was set to a custom value through the environment, otherwise
    /// the drop down would not have an entry for the selected value.
    pub fn ensure_window_resolution(&mut self, window_resolution: WindowResolution) {
        if !self.window_resolution_options.contains(&window_resolution) {
            self.window_resolution_options.insert(1, window_resolution);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{DEFAULT_BACKGROUND_MAX_FPS, DEFAULT_MAX_FPS, FramesPerSecond};

    #[test]
    fn default_limits_are_60_focused_and_30_in_background() {
        let settings = GraphicsSettings::default();

        assert_eq!(settings.max_fps, FramesPerSecond::new(60));
        assert_eq!(settings.background_max_fps, FramesPerSecond::new(30));
        assert_eq!(settings.effective_max_fps(false), 60);
        assert_eq!(settings.effective_max_fps(true), 30);

        assert_eq!(DEFAULT_MAX_FPS, FramesPerSecond::new(60));
        assert_eq!(DEFAULT_BACKGROUND_MAX_FPS, FramesPerSecond::new(30));
    }

    #[test]
    fn zero_frames_per_second_means_unlimited() {
        let mut settings = GraphicsSettings::default();
        settings.max_fps = FramesPerSecond::UNLIMITED;
        settings.background_max_fps = FramesPerSecond::UNLIMITED;

        assert_eq!(settings.effective_max_fps(false), 0);
        assert_eq!(settings.effective_max_fps(true), 0);
        assert!(settings.max_fps.is_unlimited());
        assert!(settings.background_max_fps.is_unlimited());
    }

    #[test]
    fn frame_rate_limits_are_serialized_as_plain_numbers() {
        let serialized = ron::ser::to_string(&FramesPerSecond::new(60)).unwrap();
        assert_eq!(serialized, "60");

        let deserialized: FramesPerSecond = ron::from_str("30").unwrap();
        assert_eq!(deserialized, FramesPerSecond::new(30));
    }

    #[test]
    fn old_settings_files_still_load() {
        // Settings file written by a client version that still had the old
        // `limit_framerate` drop down and no frame rate limits.
        let old_settings = r#"(
            lighting_mode: Enhanced,
            vsync: true,
            limit_framerate: Limit(60),
            triple_buffering: true,
            texture_filtering: Anisotropic(4),
            msaa: X4,
            ssaa: Off,
            screen_space_anti_aliasing: Off,
            shadow_method: SoftPCSS,
            shadow_resolution: Normal,
            shadow_detail: Medium,
            sdsm: true,
            high_quality_interface: true,
        )"#;

        let settings: GraphicsSettings = ron::from_str(old_settings).expect("old settings must still load");

        assert_eq!(settings.max_fps, FramesPerSecond::new(60));
        assert_eq!(settings.background_max_fps, FramesPerSecond::new(30));
        assert!(settings.vsync);
    }

    #[test]
    fn frame_rate_limits_round_trip_through_ron() {
        let mut settings = GraphicsSettings::default();
        settings.max_fps = FramesPerSecond::new(144);
        settings.background_max_fps = FramesPerSecond::UNLIMITED;

        let serialized = ron::ser::to_string_pretty(&settings, PrettyConfig::new()).unwrap();
        assert!(serialized.contains("max_fps: 144"));
        assert!(serialized.contains("background_max_fps: 0"));

        let deserialized: GraphicsSettings = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.max_fps, settings.max_fps);
        assert_eq!(deserialized.background_max_fps, settings.background_max_fps);
    }

    #[test]
    fn parse_frame_rate_limits() {
        assert_eq!(parse_frames_per_second("60"), Some(FramesPerSecond::new(60)));
        assert_eq!(parse_frames_per_second("  30 "), Some(FramesPerSecond::new(30)));
        assert_eq!(parse_frames_per_second("0"), Some(FramesPerSecond::UNLIMITED));
        assert_eq!(parse_frames_per_second("unlimited"), Some(FramesPerSecond::UNLIMITED));
        assert_eq!(parse_frames_per_second("OFF"), Some(FramesPerSecond::UNLIMITED));
        assert_eq!(parse_frames_per_second(""), None);
        assert_eq!(parse_frames_per_second("-1"), None);
        assert_eq!(parse_frames_per_second("sixty"), None);
        assert_eq!(parse_frames_per_second("100000"), None);
    }
}
