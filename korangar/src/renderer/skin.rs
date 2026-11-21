//! Texture support for the classic Ragnarok Online interface skin.
//!
//! The classic client draws its interface with a fixed set of pixel art
//! textures that are stored in the game archives. Most of these textures are
//! composed out of multiple files (a window frame is made of nine 14x14 tiles,
//! a title bar of three pieces, a button of three pieces, ...), so this module
//! stitches the individual pieces into small atlases that can be drawn with the
//! generic nine slice support of [`korangar_interface`].
//!
//! All textures live in the `data/texture/유저인터페이스` folder of the client
//! archives. The [`TextureLoader`] already takes care of loading them from the
//! GRF files and of converting the classic magenta color key into
//! transparency, so nothing has to be extracted from the game files.

use std::sync::Arc;

use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::graphics::Texture;
use crate::loaders::{ImageType, TextureLoader};

/// Directory of the classic interface textures inside `data/texture`.
const CLASSIC_INTERFACE: &str = "유저인터페이스";

/// Identifier of a skin texture.
///
/// This is the image handle the interface theme stores for a skin element. The
/// theme decides *which* images are used for which interface part, the renderer
/// decides *where* the image comes from and how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SkinImage {
    /// Nine slice window frame and background.
    WindowFrame,
    /// Three slice title bar.
    WindowTitleBar,
    /// Close button (normal).
    WindowCloseButton,
    /// Close button (hovered).
    WindowCloseButtonHovered,
    /// Three slice button (normal).
    ButtonNormal,
    /// Three slice button (hovered).
    ButtonHovered,
    /// Three slice button (pressed).
    ButtonPressed,
    /// Unchecked toggle/checkbox.
    CheckboxUnchecked,
    /// Checked toggle/checkbox.
    CheckboxChecked,
    /// Nine slice text input box.
    TextBox,
}

impl SkinImage {
    /// Size of the atlas in pixels, used to lay out the stitched pieces.
    fn atlas_size(self) -> Option<(u32, u32)> {
        match self {
            Self::WindowFrame => Some((42, 42)),
            Self::WindowTitleBar => Some((36, 17)),
            Self::WindowCloseButton | Self::WindowCloseButtonHovered => None,
            Self::ButtonNormal | Self::ButtonHovered | Self::ButtonPressed => Some((18, 20)),
            Self::CheckboxUnchecked | Self::CheckboxChecked => None,
            Self::TextBox => Some((8, 8)),
        }
    }
}

/// The textures of the classic interface skin.
pub struct SkinTextures {
    window_frame: Option<Arc<Texture>>,
    window_title_bar: Option<Arc<Texture>>,
    window_close_button: Option<Arc<Texture>>,
    window_close_button_hovered: Option<Arc<Texture>>,
    button_normal: Option<Arc<Texture>>,
    button_hovered: Option<Arc<Texture>>,
    button_pressed: Option<Arc<Texture>>,
    checkbox_unchecked: Option<Arc<Texture>>,
    checkbox_checked: Option<Arc<Texture>>,
    text_box: Option<Arc<Texture>>,
}

impl SkinTextures {
    /// Create the classic skin textures.
    ///
    /// This is called once when the interface renderer is created. If the
    /// textures cannot be loaded (for example because the game archives are
    /// missing or are from a client that doesn't have the classic interface)
    /// the corresponding skin image simply won't render and the color based
    /// theme is used as a fallback.
    pub fn new(texture_loader: &TextureLoader) -> Self {
        Self {
            window_frame: build_atlas(texture_loader, SkinImage::WindowFrame, &[
                "sysbox_lu.bmp",
                "sysbox_mu.bmp",
                "sysbox_ru.bmp",
                "sysbox_lm.bmp",
                "sysbox_bg.bmp",
                "sysbox_rm.bmp",
                "sysbox_ld.bmp",
                "sysbox_md.bmp",
                "sysbox_rd.bmp",
            ]),
            window_title_bar: build_atlas(texture_loader, SkinImage::WindowTitleBar, &[
                "basic_interface\\titlebar_left.bmp",
                "basic_interface\\titlebar_mid.bmp",
                "basic_interface\\titlebar_right.bmp",
            ]),
            window_close_button: load(texture_loader, "btn_close.bmp"),
            window_close_button_hovered: load(texture_loader, "btn_close_b.bmp"),
            button_normal: build_atlas(texture_loader, SkinImage::ButtonNormal, &[
                "basic_interface\\btn_out_left.bmp",
                "basic_interface\\btn_out_mid.bmp",
                "basic_interface\\btn_out_right.bmp",
            ]),
            button_hovered: build_atlas(texture_loader, SkinImage::ButtonHovered, &[
                "basic_interface\\btn_over_left.bmp",
                "basic_interface\\btn_over_mid.bmp",
                "basic_interface\\btn_over_right.bmp",
            ]),
            button_pressed: build_atlas(texture_loader, SkinImage::ButtonPressed, &[
                "basic_interface\\btn_press_left.bmp",
                "basic_interface\\btn_press_mid.bmp",
                "basic_interface\\btn_press_right.bmp",
            ]),
            checkbox_unchecked: load(texture_loader, "checkbox_0.bmp"),
            checkbox_checked: load(texture_loader, "checkbox_1.bmp"),
            text_box: build_text_box(texture_loader),
        }
    }

    /// Get the texture for a skin image.
    pub fn get(&self, image: SkinImage) -> Option<&Arc<Texture>> {
        match image {
            SkinImage::WindowFrame => self.window_frame.as_ref(),
            SkinImage::WindowTitleBar => self.window_title_bar.as_ref(),
            SkinImage::WindowCloseButton => self.window_close_button.as_ref(),
            SkinImage::WindowCloseButtonHovered => self.window_close_button_hovered.as_ref(),
            SkinImage::ButtonNormal => self.button_normal.as_ref(),
            SkinImage::ButtonHovered => self.button_hovered.as_ref(),
            SkinImage::ButtonPressed => self.button_pressed.as_ref(),
            SkinImage::CheckboxUnchecked => self.checkbox_unchecked.as_ref(),
            SkinImage::CheckboxChecked => self.checkbox_checked.as_ref(),
            SkinImage::TextBox => self.text_box.as_ref(),
        }
    }
}

/// Load a single interface texture from the game archives.
fn load(texture_loader: &TextureLoader, name: &str) -> Option<Arc<Texture>> {
    let path = format!("{CLASSIC_INTERFACE}\\{name}");

    texture_loader.get_or_load(&path, ImageType::Color).ok()
}

/// Load a texture and stitch several equally sized pieces into one atlas.
///
/// The pieces are assumed to be laid out in a single row (three slice images)
/// or in a three by three grid (nine slice images). The size of the atlas is
/// derived from the first piece, which means that all pieces have to share the
/// same dimensions (which is the case for the classic interface textures).
fn build_atlas(texture_loader: &TextureLoader, image: SkinImage, pieces: &[&str]) -> Option<Arc<Texture>> {
    let (atlas_width, atlas_height) = image.atlas_size()?;
    let mut atlas = RgbaImage::new(atlas_width, atlas_height);

    let piece_count = pieces.len();
    let columns = match piece_count {
        3 => 3,
        9 => 3,
        _ => return None,
    };
    let rows = piece_count / columns;
    let piece_width = atlas_width / columns as u32;
    let piece_height = atlas_height / rows as u32;

    for (index, piece) in pieces.iter().enumerate() {
        let path = format!("{CLASSIC_INTERFACE}\\{piece}");

        let Ok((piece_image, _)) = texture_loader.load_texture_data(&path) else {
            return None;
        };

        let piece_image = resize_to(piece_image, piece_width, piece_height)?;

        let column = (index % columns) as u32;
        let row = (index / columns) as u32;

        copy_into(&mut atlas, &piece_image, column * piece_width, row * piece_height);
    }

    Some(texture_loader.create_color("classic skin atlas", atlas, true))
}

/// Generate the classic text input box texture.
///
/// The classic client doesn't ship a texture for text boxes (it draws them with
/// a one pixel border), so we generate a small nine slice that mimics its
/// sunken look.
fn build_text_box(texture_loader: &TextureLoader) -> Option<Arc<Texture>> {
    const SIZE: u32 = 8;

    let mut image = RgbaImage::from_pixel(SIZE, SIZE, Rgba([255, 255, 255, 255]));
    let border = Rgba([123, 123, 123, 255]);
    let highlight = Rgba([255, 255, 255, 255]);
    let shadow = Rgba([188, 188, 188, 255]);

    for i in 0..SIZE {
        image.put_pixel(0, i, border);
        image.put_pixel(i, 0, border);
        image.put_pixel(SIZE - 1, i, border);
        image.put_pixel(i, SIZE - 1, border);
    }

    // Inner bevel: light on the top/left, dark on the bottom/right.
    for i in 1..SIZE - 1 {
        image.put_pixel(1, i, highlight);
        image.put_pixel(i, 1, highlight);
        image.put_pixel(SIZE - 2, i, shadow);
        image.put_pixel(i, SIZE - 2, shadow);
    }

    Some(texture_loader.create_color("classic skin text box", image, true))
}

/// Resize an image to the given size, returning `None` if the image is empty.
fn resize_to(image: RgbaImage, width: u32, height: u32) -> Option<RgbaImage> {
    if image.width() == 0 || image.height() == 0 {
        return None;
    }

    if image.width() == width && image.height() == height {
        return Some(image);
    }

    Some(image::imageops::resize(
        &image,
        width,
        height,
        image::imageops::FilterType::Nearest,
    ))
}

/// Copy a piece of an atlas into the destination image.
fn copy_into(atlas: &mut RgbaImage, piece: &RgbaImage, offset_x: u32, offset_y: u32) {
    for y in 0..piece.height() {
        for x in 0..piece.width() {
            let pixel = *piece.get_pixel(x, y);
            atlas.put_pixel(offset_x + x, offset_y + y, pixel);
        }
    }
}
