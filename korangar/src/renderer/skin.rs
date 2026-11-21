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

use std::sync::{Arc, Mutex};

use image::{Rgba, RgbaImage};
use korangar_interface::skin::Slice;
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
///
/// The textures are created lazily on first use, so a client that plays with
/// the default theme never pays for loading the classic interface textures.
pub struct SkinTextures {
    texture_loader: Arc<TextureLoader>,
    inner: Mutex<Option<SkinTexturesInner>>,
}

/// The loaded skin textures.
struct SkinTexturesInner {
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
    /// Create the (lazily loaded) classic skin textures.
    pub fn new(texture_loader: Arc<TextureLoader>) -> Self {
        Self {
            texture_loader,
            inner: Mutex::new(None),
        }
    }

    /// Get the texture for a skin image, loading the textures on first use.
    pub fn get(&self, image: SkinImage) -> Option<Arc<Texture>> {
        let mut inner = self.inner.lock().unwrap();

        if inner.is_none() {
            *inner = Some(SkinTexturesInner::new(&self.texture_loader));
        }

        inner.as_ref().unwrap().get(image)
    }
}

impl SkinTexturesInner {
    fn new(texture_loader: &TextureLoader) -> Self {
        crate::logging::log_trace!("skin: building classic skin textures (lazy first use)");

        let textures = Self {
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
        };

        crate::logging::log_trace!("skin: classic skin textures built");

        textures
    }

    fn get(&self, image: SkinImage) -> Option<Arc<Texture>> {
        match image {
            SkinImage::WindowFrame => self.window_frame.clone(),
            SkinImage::WindowTitleBar => self.window_title_bar.clone(),
            SkinImage::WindowCloseButton => self.window_close_button.clone(),
            SkinImage::WindowCloseButtonHovered => self.window_close_button_hovered.clone(),
            SkinImage::ButtonNormal => self.button_normal.clone(),
            SkinImage::ButtonHovered => self.button_hovered.clone(),
            SkinImage::ButtonPressed => self.button_pressed.clone(),
            SkinImage::CheckboxUnchecked => self.checkbox_unchecked.clone(),
            SkinImage::CheckboxChecked => self.checkbox_checked.clone(),
            SkinImage::TextBox => self.text_box.clone(),
        }
    }
}

/// Load a single interface texture from the game archives.
fn load(texture_loader: &TextureLoader, name: &str) -> Option<Arc<Texture>> {
    let path = format!("{CLASSIC_INTERFACE}\\{name}");

    crate::logging::log_trace!("skin: loading skin texture {}", path);

    match texture_loader.get_or_load(&path, ImageType::Color) {
        Ok(texture) => Some(texture),
        Err(error) => {
            crate::logging::log_warn!("skin: failed to load skin texture {}: {:?}", path, error);
            None
        }
    }
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

        crate::logging::log_trace!(
            "skin: loading atlas piece {} ({}/{}) for {:?}",
            path,
            index + 1,
            piece_count,
            image
        );

        let Ok((piece_image, _)) = texture_loader.load_texture_data(&path) else {
            crate::logging::log_warn!("skin: failed to load atlas piece {}, dropping atlas {:?}", path, image);
            return None;
        };

        let piece_image = resize_to(piece_image, piece_width, piece_height)?;

        let column = (index % columns) as u32;
        let row = (index / columns) as u32;

        copy_into(&mut atlas, &piece_image, column * piece_width, row * piece_height);
    }

    // The classic windows have a thin dark outline around the otherwise very light
    // frame. The classic texture set doesn't contain that line as a separate image,
    // so draw it into the outer ring of the frame atlas (the outer ring of the
    // nine slice ends up exactly on the window border).
    if image == SkinImage::WindowFrame {
        outline_atlas(&mut atlas);
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

/// Draw a one pixel dark outline around the outer ring of an atlas.
fn outline_atlas(image: &mut RgbaImage) {
    const OUTLINE: Rgba<u8> = Rgba([24, 24, 24, 255]);

    let width = image.width();
    let height = image.height();

    for x in 0..width {
        image.put_pixel(x, 0, OUTLINE);
        image.put_pixel(x, height - 1, OUTLINE);
    }

    for y in 0..height {
        image.put_pixel(0, y, OUTLINE);
        image.put_pixel(width - 1, y, OUTLINE);
    }
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

/// A piece of a nine slice image.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlicePiece {
    /// Position of the piece relative to the top left corner of the target
    /// area.
    pub position: (f32, f32),
    /// Size of the piece.
    pub size: (f32, f32),
    /// Normalized position of the piece inside the texture.
    pub texture_position: (f32, f32),
    /// Normalized size of the piece inside the texture.
    pub texture_size: (f32, f32),
}

/// Split a nine slice into the individual pieces that have to be drawn.
///
/// The insets of the slice are clamped to both the texture and the target size,
/// pieces that would end up empty are omitted. A slice without insets results
/// in a single piece that stretches the whole texture over the whole area.
pub fn nine_slice_pieces(area_size: (f32, f32), slices: Slice, texture_size: (f32, f32)) -> Vec<SlicePiece> {
    let (width, height) = texture_size;
    let (area_width, area_height) = area_size;

    if width <= 0.0 || height <= 0.0 || area_width <= 0.0 || area_height <= 0.0 {
        return Vec::new();
    }

    if slices.is_none() {
        return vec![SlicePiece {
            position: (0.0, 0.0),
            size: area_size,
            texture_position: (0.0, 0.0),
            texture_size: (1.0, 1.0),
        }];
    }

    // Clamp the insets so that opposite sides can never overlap.
    let left = slices.left.clamp(0.0, width);
    let right = slices.right.clamp(0.0, width - left);
    let top = slices.top.clamp(0.0, height);
    let bottom = slices.bottom.clamp(0.0, height - top);

    let source_columns = [(0.0, left), (left, width - right), (width - right, width)];
    let source_rows = [(0.0, top), (top, height - bottom), (height - bottom, height)];
    let destination_columns = [(0.0, left), (left, area_width - right), (area_width - right, area_width)];
    let destination_rows = [(0.0, top), (top, area_height - bottom), (area_height - bottom, area_height)];

    let mut pieces = Vec::with_capacity(9);

    for (row, (source_top, source_bottom)) in source_rows.iter().enumerate() {
        for (column, (source_left, source_right)) in source_columns.iter().enumerate() {
            let (destination_left, destination_right) = destination_columns[column];
            let (destination_top, destination_bottom) = destination_rows[row];

            let piece_width = destination_right - destination_left;
            let piece_height = destination_bottom - destination_top;

            // Skip empty pieces (this happens when the target area is smaller than the
            // slice insets).
            if piece_width <= 0.0 || piece_height <= 0.0 {
                continue;
            }

            pieces.push(SlicePiece {
                position: (destination_left, destination_top),
                size: (piece_width, piece_height),
                texture_position: (source_left / width, source_top / height),
                texture_size: ((source_right - source_left) / width, (source_bottom - source_top) / height),
            });
        }
    }

    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_slices_the_whole_texture_is_stretched() {
        let pieces = nine_slice_pieces((100.0, 50.0), Slice::NONE, (42.0, 42.0));

        assert_eq!(pieces.len(), 1);
        assert_eq!(pieces[0].position, (0.0, 0.0));
        assert_eq!(pieces[0].size, (100.0, 50.0));
        assert_eq!(pieces[0].texture_position, (0.0, 0.0));
        assert_eq!(pieces[0].texture_size, (1.0, 1.0));
    }

    #[test]
    fn uniform_slices_result_in_nine_pieces() {
        let pieces = nine_slice_pieces((100.0, 100.0), Slice::uniform(14.0), (42.0, 42.0));

        assert_eq!(pieces.len(), 9);

        // The top left corner is not stretched.
        assert_eq!(pieces[0].position, (0.0, 0.0));
        assert_eq!(pieces[0].size, (14.0, 14.0));
        assert_eq!(pieces[0].texture_position, (0.0, 0.0));
        assert!((pieces[0].texture_size.0 - 14.0 / 42.0).abs() < f32::EPSILON);

        // The top edge is stretched horizontally.
        assert_eq!(pieces[1].position, (14.0, 0.0));
        assert_eq!(pieces[1].size, (72.0, 14.0));
        assert!((pieces[1].texture_position.0 - 14.0 / 42.0).abs() < f32::EPSILON);

        // The bottom right corner is at the end of both the area and the texture.
        let last = pieces[8];
        assert_eq!(last.position, (86.0, 86.0));
        assert_eq!(last.size, (14.0, 14.0));
        assert!((last.texture_position.0 - 28.0 / 42.0).abs() < f32::EPSILON);
        assert!((last.texture_size.0 - 14.0 / 42.0).abs() < f32::EPSILON);
    }

    #[test]
    fn horizontal_slices_result_in_three_pieces() {
        let pieces = nine_slice_pieces((50.0, 20.0), Slice::horizontal(6.0, 6.0), (18.0, 20.0));

        assert_eq!(pieces.len(), 3);
        assert_eq!(pieces[0].size, (6.0, 20.0));
        assert_eq!(pieces[1].position, (6.0, 0.0));
        assert_eq!(pieces[1].size, (38.0, 20.0));
        assert_eq!(pieces[2].position, (44.0, 0.0));
    }

    #[test]
    fn degenerate_pieces_are_skipped_if_the_area_is_too_small() {
        // The area is smaller than the sum of the slice insets.
        let pieces = nine_slice_pieces((10.0, 10.0), Slice::uniform(14.0), (42.0, 42.0));

        for piece in &pieces {
            assert!(piece.size.0 > 0.0);
            assert!(piece.size.1 > 0.0);
        }
    }
}
