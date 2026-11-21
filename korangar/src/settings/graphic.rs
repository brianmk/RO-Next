#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{
    LimitFramerate, Msaa, PresentModeInfo, ScreenSpaceAntiAliasing, ShadowDetail, ShadowMethod, ShadowResolution, Ssaa, TextureSamplerType,
    WindowResolution,
};

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GraphicsSettings {
    pub lighting_mode: LightingMode,
    pub vsync: bool,
    pub limit_framerate: LimitFramerate,
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
            limit_framerate: LimitFramerate::Unlimited,
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

        settings
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
    fn apply_environment_overrides(&mut self) {
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

    pub fn load() -> Option<Self> {
        #[cfg(feature = "debug")]
        print_debug!("loading graphics settings from {}", Self::FILE_NAME.magenta());

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
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

    fn value(&self) -> LightingMode {
        *self
    }
}

#[derive(RustState, StateElement)]
pub struct GraphicsSettingsCapabilities {
    lighting_modes: Vec<LightingMode>,
    texture_filtering_options: Vec<TextureSamplerType>,
    limit_framerate_options: Vec<LimitFramerate>,
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
            limit_framerate_options: vec![
                LimitFramerate::Unlimited,
                LimitFramerate::Limit(30),
                LimitFramerate::Limit(60),
                LimitFramerate::Limit(120),
                LimitFramerate::Limit(144),
                LimitFramerate::Limit(240),
            ],
            supported_msaa: Vec::new(),
            ssaa_options: vec![Ssaa::Off, Ssaa::X2, Ssaa::X3, Ssaa::X4],
            screen_space_anti_aliasing_options: vec![ScreenSpaceAntiAliasing::Off, ScreenSpaceAntiAliasing::Fxaa],
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
