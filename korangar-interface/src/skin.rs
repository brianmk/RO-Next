//! Support for image based interface skins (9-slice / 3-slice textures).
//!
//! Korangar's default themes describe the interface exclusively using colors
//! and geometry. Classic Ragnarok Online on the other hand was drawn using a
//! fixed set of pixel art interface textures: window frames that are composed
//! out of nine pieces, title bars made of three pieces, buttons, text boxes and
//! so on.
//!
//! This module adds a small layer that allows a theme to reference application
//! specific images for these elements. The application decides how an image is
//! resolved (korangar loads the textures from the game archives) and how a
//! nine slice is drawn. Every field is optional, so a theme that doesn't fill
//! in the skin keeps the plain color based look.

use rust_state::RustState;

use crate::application::Application;
use crate::layout::WindowLayout;
use crate::layout::area::Area;

/// Insets (in interface units) of a nine slice image.
///
/// The four values describe how many pixels from each side of the source image
/// are *not* stretched. A [`Slice`] of zero for every side means that the whole
/// image is simply stretched over the target area.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Slice {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Slice {
    /// A slice that doesn't slice anything, i.e. the image is stretched.
    pub const NONE: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// Create a new slice from the four insets.
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self { left, top, right, bottom }
    }

    /// Create a slice with the same inset on every side.
    pub fn uniform(inset: f32) -> Self {
        Self::new(inset, inset, inset, inset)
    }

    /// Create a slice that only slices horizontally.
    pub fn horizontal(left: f32, right: f32) -> Self {
        Self::new(left, 0.0, right, 0.0)
    }

    /// Scale the insets.
    pub fn scaled(self, scaling: f32) -> Self {
        Self::new(
            self.left * scaling,
            self.top * scaling,
            self.right * scaling,
            self.bottom * scaling,
        )
    }

    /// Returns `true` if the slice is empty and therefore no nine slice
    /// rendering is required.
    pub fn is_none(self) -> bool {
        self == Self::NONE
    }
}

/// An image with a color tint and nine slice information.
#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ImageSlice<App>
where
    App: Application,
{
    /// The application specific image handle.
    pub image: App::Image,
    /// Color that is multiplied with the image.
    pub color: App::Color,
    /// Insets that are not stretched when the image is drawn.
    pub slices: Slice,
    /// Use linear filtering instead of nearest neighbor filtering.
    pub smooth: bool,
}

impl<App: Application> ImageSlice<App> {
    /// Scale the slice insets of the image.
    pub fn scaled_slices(&self, scaling: f32) -> Slice {
        self.slices.scaled(scaling)
    }

    /// Add this image to the layout, drawing it into the given area.
    pub fn add_to_layout<'a>(&self, layout: &mut WindowLayout<'a, App>, area: Area) {
        layout.add_image(area, self.image, self.color, self.slices, self.smooth);
    }
}

/// Skin of a window.
#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "ImageSlice<App>: serde::Serialize",
        deserialize = "ImageSlice<App>: serde::Deserialize<'de>"
    ))
)]
pub struct WindowSkin<App>
where
    App: Application,
{
    /// Nine slice that replaces the window background.
    ///
    /// The slices are usually the border of the window frame, which means that
    /// the themed `border` of the window has to be large enough to not draw
    /// the contents over the frame.
    pub frame: Option<ImageSlice<App>>,
    /// Image that is drawn over the title area of the window.
    ///
    /// This is usually a three slice (left cap, stretched middle, right cap).
    pub title_bar: Option<ImageSlice<App>>,
    /// Image for the close button.
    pub close_button: Option<ImageSlice<App>>,
    /// Image that is drawn when the close button is hovered.
    ///
    /// Falls back to `close_button` if this is not set.
    pub hovered_close_button: Option<ImageSlice<App>>,
}

impl<App: Application> Default for WindowSkin<App> {
    fn default() -> Self {
        Self {
            frame: None,
            title_bar: None,
            close_button: None,
            hovered_close_button: None,
        }
    }
}

/// Skin of a button.
#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "ImageSlice<App>: serde::Serialize",
        deserialize = "ImageSlice<App>: serde::Deserialize<'de>"
    ))
)]
pub struct ButtonSkin<App>
where
    App: Application,
{
    pub normal: Option<ImageSlice<App>>,
    pub hovered: Option<ImageSlice<App>>,
    pub pressed: Option<ImageSlice<App>>,
    pub disabled: Option<ImageSlice<App>>,
}

impl<App: Application> Default for ButtonSkin<App> {
    fn default() -> Self {
        Self {
            normal: None,
            hovered: None,
            pressed: None,
            disabled: None,
        }
    }
}

/// Skin of a toggleable button (checkbox).
#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "ImageSlice<App>: serde::Serialize",
        deserialize = "ImageSlice<App>: serde::Deserialize<'de>"
    ))
)]
pub struct StateButtonSkin<App>
where
    App: Application,
{
    /// Background of the button, usually the same as [`ButtonSkin`].
    pub background: ButtonSkin<App>,
    /// Image for an unchecked toggle.
    pub unchecked: Option<ImageSlice<App>>,
    /// Image for a checked toggle.
    pub checked: Option<ImageSlice<App>>,
}

impl<App: Application> Default for StateButtonSkin<App> {
    fn default() -> Self {
        Self {
            background: ButtonSkin::default(),
            unchecked: None,
            checked: None,
        }
    }
}

/// Skin of a drop down button (the value display and the menu title).
#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "ImageSlice<App>: serde::Serialize",
        deserialize = "ImageSlice<App>: serde::Deserialize<'de>"
    ))
)]
pub struct DropDownSkin<App>
where
    App: Application,
{
    pub button: ButtonSkin<App>,
}

impl<App: Application> Default for DropDownSkin<App> {
    fn default() -> Self {
        Self {
            button: ButtonSkin::default(),
        }
    }
}

/// The skin part of an interface theme.
///
/// An empty (all `None`) skin means that the interface is drawn the plain
/// color based way. The application provides a preset that fills in the fields
/// with the classic Ragnarok Online interface textures.
#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "serde",
    serde(bound(
        serialize = "ImageSlice<App>: serde::Serialize",
        deserialize = "ImageSlice<App>: serde::Deserialize<'de>"
    ))
)]
pub struct SkinTheme<App>
where
    App: Application,
{
    pub window: WindowSkin<App>,
    pub button: ButtonSkin<App>,
    pub state_button: StateButtonSkin<App>,
    pub drop_down: DropDownSkin<App>,
    pub text_box: Option<ImageSlice<App>>,
    pub field: Option<ImageSlice<App>>,
    pub tooltip: Option<ImageSlice<App>>,
}

impl<App: Application> Default for SkinTheme<App> {
    fn default() -> Self {
        Self {
            window: WindowSkin::default(),
            button: ButtonSkin::default(),
            state_button: StateButtonSkin::default(),
            drop_down: DropDownSkin::default(),
            text_box: None,
            field: None,
            tooltip: None,
        }
    }
}
