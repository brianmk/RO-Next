use std::fmt::{Display, Formatter};
#[cfg(feature = "debug")]
use std::num::NonZeroU32;

use ro_next_interface::components::drop_down::DropDownItem;
use ro_next_interface::element::StateElement;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

use crate::graphics::{ScreenSize, dlss_disabled_reason};
use crate::state::localization::dropdown_keys;

/// A frame rate limit in frames per second.
///
/// The special value `0` means that the frame rate is not limited. A limit is
/// serialized as a plain number, so the settings file stays readable:
///
/// ```ron
/// max_fps: 60,
/// background_max_fps: 30,
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, RustState, StateElement)]
pub struct FramesPerSecond {
    frames_per_second: u32,
}

/// The resolution the game window is rendered at.
///
/// The presets are the physical pixel size of the window, independent of the
/// DPI scale factor of the monitor. "Desktop" instead uses the desktop
/// resolution by keeping the window maximized.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum WindowResolution {
    /// Use the current desktop resolution by keeping the window maximized.
    Desktop,
    /// A window with a size of 1280x720 pixels.
    R1280x720,
    /// A window with a size of 1366x768 pixels.
    R1366x768,
    /// A window with a size of 1600x900 pixels.
    R1600x900,
    /// A window with a size of 1920x1080 pixels.
    R1920x1080,
    /// A window with a size of 2560x1440 pixels.
    R2560x1440,
    /// A window with a size of 3840x2160 pixels.
    R3840x2160,
    /// A custom resolution that is not one of the presets. This is only ever
    /// set through the `RO_NEXT_RESOLUTION` environment variable and is
    /// persisted like any other resolution.
    Custom(u32, u32),
}

impl WindowResolution {
    /// All resolution options that are shown in the graphics settings window.
    pub const OPTIONS: [WindowResolution; 7] = [
        WindowResolution::Desktop,
        WindowResolution::R1280x720,
        WindowResolution::R1366x768,
        WindowResolution::R1600x900,
        WindowResolution::R1920x1080,
        WindowResolution::R2560x1440,
        WindowResolution::R3840x2160,
    ];

    /// The physical size of the window in pixels, if the resolution is not
    /// [`WindowResolution::Desktop`].
    pub fn size(self) -> Option<(u32, u32)> {
        match self {
            WindowResolution::Desktop => None,
            WindowResolution::R1280x720 => Some((1280, 720)),
            WindowResolution::R1366x768 => Some((1366, 768)),
            WindowResolution::R1600x900 => Some((1600, 900)),
            WindowResolution::R1920x1080 => Some((1920, 1080)),
            WindowResolution::R2560x1440 => Some((2560, 1440)),
            WindowResolution::R3840x2160 => Some((3840, 2160)),
            WindowResolution::Custom(width, height) => Some((width, height)),
        }
    }

    /// Parses a `WIDTHxHEIGHT` or `desktop` string. This is used by the
    /// `RO_NEXT_RESOLUTION` environment variable.
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();

        if input.eq_ignore_ascii_case("desktop") || input.eq_ignore_ascii_case("maximized") {
            return Some(WindowResolution::Desktop);
        }

        let (width, height) = input.split_once(['x', 'X'])?;
        let width: u32 = width.trim().parse().ok()?;
        let height: u32 = height.trim().parse().ok()?;

        // Refuse nonsensical sizes so a typo does not create an unusable window.
        if !(320..=16384).contains(&width) || !(240..=16384).contains(&height) {
            return None;
        }

        match (width, height) {
            (1280, 720) => Some(WindowResolution::R1280x720),
            (1366, 768) => Some(WindowResolution::R1366x768),
            (1600, 900) => Some(WindowResolution::R1600x900),
            (1920, 1080) => Some(WindowResolution::R1920x1080),
            (2560, 1440) => Some(WindowResolution::R2560x1440),
            (3840, 2160) => Some(WindowResolution::R3840x2160),
            (width, height) => Some(WindowResolution::Custom(width, height)),
        }
    }
}

impl DropDownItem<WindowResolution> for WindowResolution {
    fn text(&self) -> &str {
        match self {
            WindowResolution::Desktop => "Desktop (maximized)",
            WindowResolution::R1280x720 => "1280 x 720",
            WindowResolution::R1366x768 => "1366 x 768",
            WindowResolution::R1600x900 => "1600 x 900",
            WindowResolution::R1920x1080 => "1920 x 1080",
            WindowResolution::R2560x1440 => "2560 x 1440",
            WindowResolution::R3840x2160 => "3840 x 2160",
            WindowResolution::Custom(..) => "Custom",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            WindowResolution::Desktop => dropdown_keys::RESOLUTION_DESKTOP,
            WindowResolution::R1280x720 => dropdown_keys::RESOLUTION_1280X720,
            WindowResolution::R1366x768 => dropdown_keys::RESOLUTION_1366X768,
            WindowResolution::R1600x900 => dropdown_keys::RESOLUTION_1600X900,
            WindowResolution::R1920x1080 => dropdown_keys::RESOLUTION_1920X1080,
            WindowResolution::R2560x1440 => dropdown_keys::RESOLUTION_2560X1440,
            WindowResolution::R3840x2160 => dropdown_keys::RESOLUTION_3840X2160,
            WindowResolution::Custom(..) => dropdown_keys::RESOLUTION_CUSTOM,
        };
        Some(key)
    }

    fn value(&self) -> WindowResolution {
        *self
    }
}

impl FramesPerSecond {
    /// No limit at all. The client renders as fast as the hardware allows.
    pub const UNLIMITED: FramesPerSecond = FramesPerSecond { frames_per_second: 0 };

    pub const fn new(frames_per_second: u32) -> FramesPerSecond {
        FramesPerSecond { frames_per_second }
    }

    pub const fn get(self) -> u32 {
        self.frames_per_second
    }

    pub const fn is_unlimited(self) -> bool {
        self.frames_per_second == 0
    }
}

impl Serialize for FramesPerSecond {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.frames_per_second)
    }
}

impl<'de> Deserialize<'de> for FramesPerSecond {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<FramesPerSecond, D::Error> {
        u32::deserialize(deserializer).map(FramesPerSecond::new)
    }
}

impl Default for FramesPerSecond {
    fn default() -> FramesPerSecond {
        FramesPerSecond::UNLIMITED
    }
}

impl DropDownItem<FramesPerSecond> for FramesPerSecond {
    fn text(&self) -> &str {
        match self.frames_per_second {
            0 => "Unlimited",
            15 => "15 FPS",
            24 => "24 FPS",
            30 => "30 FPS",
            45 => "45 FPS",
            60 => "60 FPS",
            75 => "75 FPS",
            90 => "90 FPS",
            120 => "120 FPS",
            144 => "144 FPS",
            240 => "240 FPS",
            // Values that are not offered by the drop down (for example coming
            // from an environment override) are still valid, but cannot be
            // displayed as a nicely named entry.
            _ => "Custom",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self.frames_per_second {
            0 => dropdown_keys::FPS_UNLIMITED,
            15 => dropdown_keys::FPS_15,
            24 => dropdown_keys::FPS_24,
            30 => dropdown_keys::FPS_30,
            45 => dropdown_keys::FPS_45,
            60 => dropdown_keys::FPS_60,
            75 => dropdown_keys::FPS_75,
            90 => dropdown_keys::FPS_90,
            120 => dropdown_keys::FPS_120,
            144 => dropdown_keys::FPS_144,
            240 => dropdown_keys::FPS_240,
            // Values that are not offered by the drop down (for example coming
            // from an environment override) are still valid, but cannot be
            // displayed as a nicely named entry.
            _ => dropdown_keys::FPS_CUSTOM,
        };
        Some(key)
    }

    fn value(&self) -> FramesPerSecond {
        *self
    }
}

/// The frame rate limits that are offered in the settings window. These are
/// only presets, the actual limit can be any value when it is set through the
/// `RO_NEXT_MAX_FPS`/`RO_NEXT_BACKGROUND_MAX_FPS` environment variables.
pub const FRAMERATE_PRESETS: &[FramesPerSecond] = &[
    FramesPerSecond::UNLIMITED,
    FramesPerSecond::new(15),
    FramesPerSecond::new(30),
    FramesPerSecond::new(45),
    FramesPerSecond::new(60),
    FramesPerSecond::new(75),
    FramesPerSecond::new(120),
    FramesPerSecond::new(144),
    FramesPerSecond::new(240),
];

/// The maximum number of frames per second a client should render per default.
pub const DEFAULT_MAX_FPS: FramesPerSecond = FramesPerSecond::new(60);

/// The maximum number of frames per second a client should render while its
/// window is in the background.
pub const DEFAULT_BACKGROUND_MAX_FPS: FramesPerSecond = FramesPerSecond::new(30);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum TextureSamplerType {
    Nearest,
    Linear,
    Anisotropic(u16),
}

impl DropDownItem<TextureSamplerType> for TextureSamplerType {
    fn text(&self) -> &str {
        match self {
            TextureSamplerType::Nearest => "Nearest",
            TextureSamplerType::Linear => "Linear",
            TextureSamplerType::Anisotropic(4) => "Anisotropic x4",
            TextureSamplerType::Anisotropic(8) => "Anisotropic x8",
            TextureSamplerType::Anisotropic(16) => "Anisotropic x16",
            TextureSamplerType::Anisotropic(_) => unimplemented!(),
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            TextureSamplerType::Nearest => dropdown_keys::SAMPLER_NEAREST,
            TextureSamplerType::Linear => dropdown_keys::SAMPLER_LINEAR,
            TextureSamplerType::Anisotropic(4) => dropdown_keys::SAMPLER_ANISOTROPIC_4,
            TextureSamplerType::Anisotropic(8) => dropdown_keys::SAMPLER_ANISOTROPIC_8,
            TextureSamplerType::Anisotropic(16) => dropdown_keys::SAMPLER_ANISOTROPIC_16,
            TextureSamplerType::Anisotropic(_) => unimplemented!(),
        };
        Some(key)
    }

    fn value(&self) -> TextureSamplerType {
        *self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowResolution {
    Normal,
    Ultra,
    Insane,
}

impl ShadowResolution {
    pub fn directional_shadow_resolution(self) -> u32 {
        match self {
            ShadowResolution::Normal => 2048,
            ShadowResolution::Ultra => 3072,
            ShadowResolution::Insane => 4096,
        }
    }

    pub fn point_shadow_resolution(self) -> u32 {
        match self {
            ShadowResolution::Normal => 128,
            ShadowResolution::Ultra => 256,
            ShadowResolution::Insane => 512,
        }
    }
}

impl DropDownItem<ShadowResolution> for ShadowResolution {
    fn text(&self) -> &str {
        match self {
            Self::Normal => "Normal",
            Self::Ultra => "Ultra",
            Self::Insane => "Insane",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            Self::Normal => dropdown_keys::SHADOW_RESOLUTION_NORMAL,
            Self::Ultra => dropdown_keys::SHADOW_RESOLUTION_ULTRA,
            Self::Insane => dropdown_keys::SHADOW_RESOLUTION_INSANE,
        };
        Some(key)
    }

    fn value(&self) -> ShadowResolution {
        *self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowDetail {
    Low,
    Medium,
    High,
    Ultra,
}

impl DropDownItem<ShadowDetail> for ShadowDetail {
    fn text(&self) -> &str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            Self::Low => dropdown_keys::SHADOW_DETAIL_LOW,
            Self::Medium => dropdown_keys::SHADOW_DETAIL_MEDIUM,
            Self::High => dropdown_keys::SHADOW_DETAIL_HIGH,
            Self::Ultra => dropdown_keys::SHADOW_DETAIL_ULTRA,
        };
        Some(key)
    }

    fn value(&self) -> ShadowDetail {
        *self
    }
}

impl From<ShadowDetail> for u32 {
    fn from(value: ShadowDetail) -> Self {
        match value {
            ShadowDetail::Low => 1,
            ShadowDetail::Medium => 2,
            ShadowDetail::High => 3,
            ShadowDetail::Ultra => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowMethod {
    Hard,
    SoftPCF,
    SoftPCSS,
}

impl DropDownItem<ShadowMethod> for ShadowMethod {
    fn text(&self) -> &str {
        match self {
            Self::Hard => "Hard",
            Self::SoftPCF => "Soft (PCF)",
            Self::SoftPCSS => "Soft (PCSS)",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            Self::Hard => dropdown_keys::SHADOW_METHOD_HARD,
            Self::SoftPCF => dropdown_keys::SHADOW_METHOD_SOFT_PCF,
            Self::SoftPCSS => dropdown_keys::SHADOW_METHOD_SOFT_PCSS,
        };
        Some(key)
    }

    fn value(&self) -> ShadowMethod {
        *self
    }
}

impl From<ShadowMethod> for u32 {
    fn from(value: ShadowMethod) -> Self {
        match value {
            ShadowMethod::Hard => 0,
            ShadowMethod::SoftPCF => 1,
            ShadowMethod::SoftPCSS => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Msaa {
    Off,
    X2,
    X4,
    X8,
    X16,
}

impl DropDownItem<Msaa> for Msaa {
    fn text(&self) -> &str {
        match self {
            Msaa::Off => "Off",
            Msaa::X2 => "x2",
            Msaa::X4 => "x4",
            Msaa::X8 => "x8",
            Msaa::X16 => "x16",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            Msaa::Off => dropdown_keys::MSAA_OFF,
            Msaa::X2 => dropdown_keys::MSAA_X2,
            Msaa::X4 => dropdown_keys::MSAA_X4,
            Msaa::X8 => dropdown_keys::MSAA_X8,
            Msaa::X16 => dropdown_keys::MSAA_X16,
        };
        Some(key)
    }

    fn value(&self) -> Msaa {
        *self
    }
}

impl Display for Msaa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Msaa::Off => "Off".fmt(f),
            Msaa::X2 => "x2".fmt(f),
            Msaa::X4 => "x4".fmt(f),
            Msaa::X8 => "x8".fmt(f),
            Msaa::X16 => "x16".fmt(f),
        }
    }
}

impl From<u32> for Msaa {
    fn from(value: u32) -> Self {
        match value {
            1 => Msaa::Off,
            2 => Msaa::X2,
            4 => Msaa::X4,
            8 => Msaa::X8,
            16 => Msaa::X16,
            _ => panic!("Unknown sample count"),
        }
    }
}

impl Msaa {
    pub fn sample_count(self) -> u32 {
        match self {
            Msaa::Off => 1,
            Msaa::X2 => 2,
            Msaa::X4 => 4,
            Msaa::X8 => 8,
            Msaa::X16 => 16,
        }
    }

    pub fn multisampling_activated(self) -> bool {
        self != Msaa::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Ssaa {
    Off,
    X2,
    X3,
    X4,
}

impl DropDownItem<Ssaa> for Ssaa {
    fn text(&self) -> &str {
        match self {
            Ssaa::Off => "Off",
            Ssaa::X2 => "x2",
            Ssaa::X3 => "x3",
            Ssaa::X4 => "x4",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            Ssaa::Off => dropdown_keys::SSAA_OFF,
            Ssaa::X2 => dropdown_keys::SSAA_X2,
            Ssaa::X3 => dropdown_keys::SSAA_X3,
            Ssaa::X4 => dropdown_keys::SSAA_X4,
        };
        Some(key)
    }

    fn value(&self) -> Ssaa {
        *self
    }
}

impl Display for Ssaa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ssaa::Off => "Off".fmt(f),
            Ssaa::X2 => "x2".fmt(f),
            Ssaa::X3 => "x3".fmt(f),
            Ssaa::X4 => "x4".fmt(f),
        }
    }
}

impl Ssaa {
    pub fn calculate_size(self, base_size: ScreenSize) -> ScreenSize {
        match self {
            Ssaa::Off => base_size,
            Ssaa::X2 => base_size * f32::sqrt(2.0),
            Ssaa::X3 => base_size * f32::sqrt(3.0),
            Ssaa::X4 => base_size * 2.0,
        }
    }

    pub fn supersampling_activated(self) -> bool {
        self != Ssaa::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ScreenSpaceAntiAliasing {
    Off,
    Fxaa,
    /// SMAA 1x: morphological anti-aliasing based on the color buffer.
    Smaa,
    /// DLSS (or DLAA) based anti-aliasing. This is not implemented in this
    /// build, the entry is shown in the settings window but disabled.
    Dlss,
}

impl DropDownItem<ScreenSpaceAntiAliasing> for ScreenSpaceAntiAliasing {
    fn text(&self) -> &str {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off",
            ScreenSpaceAntiAliasing::Fxaa => "FXAA",
            ScreenSpaceAntiAliasing::Smaa => "SMAA",
            ScreenSpaceAntiAliasing::Dlss => "DLSS",
        }
    }

    fn text_key(&self) -> Option<&'static str> {
        let key = match self {
            ScreenSpaceAntiAliasing::Off => dropdown_keys::ANTI_ALIASING_OFF,
            ScreenSpaceAntiAliasing::Fxaa => dropdown_keys::ANTI_ALIASING_FXAA,
            ScreenSpaceAntiAliasing::Smaa => dropdown_keys::ANTI_ALIASING_SMAA,
            ScreenSpaceAntiAliasing::Dlss => dropdown_keys::ANTI_ALIASING_DLSS,
        };
        Some(key)
    }

    fn value(&self) -> ScreenSpaceAntiAliasing {
        *self
    }

    fn disabled(&self) -> Option<&'static str> {
        match self {
            ScreenSpaceAntiAliasing::Dlss => Some(dlss_disabled_reason()),
            _ => None,
        }
    }
}

impl Display for ScreenSpaceAntiAliasing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off".fmt(f),
            ScreenSpaceAntiAliasing::Fxaa => "FXAA".fmt(f),
            ScreenSpaceAntiAliasing::Smaa => "SMAA".fmt(f),
            ScreenSpaceAntiAliasing::Dlss => "DLSS".fmt(f),
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Default, rust_state::RustState, StateElement)]
pub struct RenderOptions {
    pub show_frames_per_second: bool,
    pub frustum_culling: bool,
    pub show_bounding_boxes: bool,
    pub show_map: bool,
    pub show_objects: bool,
    pub show_entities: bool,
    pub show_entities_paper: bool,
    pub show_entities_debug: bool,
    pub show_ground_items: bool,
    pub show_water: bool,
    pub show_indicators: bool,
    pub enable_ambient_lighting: bool,
    pub enable_directional_lighting: bool,
    pub enable_point_lights: bool,
    pub enable_particle_lighting: bool,
    pub use_debug_camera: bool,
    pub show_wireframe: bool,
    pub show_object_markers: bool,
    pub show_light_markers: bool,
    pub show_sound_markers: bool,
    pub show_effect_markers: bool,
    pub show_particle_markers: bool,
    pub show_entity_markers: bool,
    pub show_shadow_markers: bool,
    pub show_map_tiles: bool,
    pub show_pathing: bool,
    pub show_picker_buffer: bool,
    pub show_directional_shadow_map: Option<NonZeroU32>,
    pub show_point_shadow_map: Option<NonZeroU32>,
    pub show_light_culling_count_buffer: bool,
    pub show_sdsm_partitions: bool,
    pub show_rectangle_instructions: bool,
    pub show_glyph_instructions: bool,
    pub show_sprite_instructions: bool,
    pub show_sdf_instructions: bool,
}

#[cfg(feature = "debug")]
impl RenderOptions {
    pub fn new() -> Self {
        Self {
            show_frames_per_second: false,
            frustum_culling: true,
            show_bounding_boxes: false,
            show_map: true,
            show_objects: true,
            show_entities: true,
            show_entities_paper: false,
            show_entities_debug: false,
            show_ground_items: true,
            show_water: true,
            show_indicators: true,
            enable_ambient_lighting: true,
            enable_directional_lighting: true,
            enable_point_lights: true,
            enable_particle_lighting: true,
            use_debug_camera: false,
            show_wireframe: false,
            show_object_markers: false,
            show_light_markers: false,
            show_sound_markers: false,
            show_effect_markers: false,
            show_particle_markers: false,
            show_entity_markers: false,
            show_shadow_markers: false,
            show_map_tiles: false,
            show_pathing: false,
            show_picker_buffer: false,
            show_directional_shadow_map: None,
            show_point_shadow_map: None,
            show_light_culling_count_buffer: false,
            show_sdsm_partitions: false,
            show_rectangle_instructions: false,
            show_glyph_instructions: false,
            show_sprite_instructions: false,
            show_sdf_instructions: false,
        }
    }
}

#[cfg(feature = "debug")]
impl RenderOptions {
    pub fn show_buffers(&self) -> bool {
        self.show_picker_buffer
            || self.show_directional_shadow_map.is_some()
            || self.show_point_shadow_map.is_some()
            || self.show_light_culling_count_buffer
            || self.show_sdsm_partitions
    }
}
