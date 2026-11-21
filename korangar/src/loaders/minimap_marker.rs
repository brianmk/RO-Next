//! Generation of the minimap player marker.
//!
//! The classic Ragnarok Online client rotates a single arrow bitmap
//! (`data\texture\유저인터페이스\minimap\player_1.bmp`) to the direction the
//! player is facing. That bitmap is not shipped as an arrow in the CondaRO
//! archives (the file is a small head icon used for *other* players), and the
//! interface renderer cannot rotate textures. So the arrow is rasterized here
//! once at startup, pre-rotated into eight textures (one per facing
//! direction), which the minimap then picks from based on the player's
//! direction.

use image::{Rgba, RgbaImage};

use crate::graphics::Texture;
use crate::loaders::TextureLoader;

/// Side length of the (unscaled) arrow texture.
pub const MARKER_TEXTURE_SIZE: u32 = 19;

/// How many direction frames are generated.
pub const MARKER_DIRECTION_COUNT: usize = 8;

/// Color of the arrow fill (same green as the old square marker).
const MARKER_COLOR: [u8; 4] = [80, 255, 110, 255];
/// Dark outline of the arrow, so it stays visible on light map areas.
const MARKER_OUTLINE_COLOR: [u8; 4] = [0, 0, 0, 200];

/// Generate one texture per facing direction.
///
/// Index 0 points north (up on the minimap), the index increases clockwise:
/// north, north-east, east, south-east, south, south-west, west, north-west.
/// If texture creation fails an empty vector is returned and the minimap falls
/// back to drawing a green square.
pub fn generate_direction_markers(texture_loader: &TextureLoader) -> Vec<std::sync::Arc<Texture>> {
    (0..MARKER_DIRECTION_COUNT)
        .map(|direction| {
            let image = generate_arrow_image(direction);

            texture_loader.create_color(&format!("minimap/marker/{direction}"), image, true)
        })
        .collect()
}

/// Arrow shaped polygon pointing up (north), in texture pixel coordinates.
///
/// It is an arrow head with a notch at the bottom. The texture is centered
/// around (`MARKER_TEXTURE_SIZE` / 2, `MARKER_TEXTURE_SIZE` / 2).
fn base_arrow_polygon() -> Vec<(f32, f32)> {
    let center = MARKER_TEXTURE_SIZE as f32 / 2.0;

    vec![
        (center, 0.5),
        (MARKER_TEXTURE_SIZE as f32 - 1.5, MARKER_TEXTURE_SIZE as f32 - 2.0),
        (center, MARKER_TEXTURE_SIZE as f32 - 5.5),
        (2.0, MARKER_TEXTURE_SIZE as f32 - 2.0),
    ]
}

/// Rasterize the arrow of one direction, including a dark outline.
fn generate_arrow_image(direction: usize) -> RgbaImage {
    let mut image = RgbaImage::new(MARKER_TEXTURE_SIZE, MARKER_TEXTURE_SIZE);

    let center = MARKER_TEXTURE_SIZE as f32 / 2.0;
    // 45 degrees per direction, clockwise. The renderer has y pointing down,
    // which makes the standard rotation matrix a clockwise rotation.
    let angle = std::f32::consts::FRAC_PI_4 * direction as f32;
    let (sin, cos) = angle.sin_cos();

    let polygon = base_arrow_polygon();
    let outline_polygon: Vec<(f32, f32)> = polygon
        .iter()
        .map(|(x, y)| {
            let (x, y) = (x - center, y - center);
            let scale = 1.22;
            (center + x * scale, center + y * scale)
        })
        .collect();

    for pixel_y in 0..MARKER_TEXTURE_SIZE {
        for pixel_x in 0..MARKER_TEXTURE_SIZE {
            // Supersample 2x2 to get a smooth edge on the small texture.
            let mut outline_coverage = 0;
            let mut fill_coverage = 0;

            for sample in 0..4 {
                let offset_x = ((sample & 1) as f32 + 0.5) * 0.5;
                let offset_y = (((sample >> 1) & 1) as f32 + 0.5) * 0.5;
                let point_x = pixel_x as f32 + offset_x;
                let point_y = pixel_y as f32 + offset_y;

                // Rotate the sample back into the "pointing up" frame.
                let (rel_x, rel_y) = (point_x - center, point_y - center);
                let base_x = center + rel_x * cos + rel_y * sin;
                let base_y = center - rel_x * sin + rel_y * cos;

                if point_in_polygon(base_x, base_y, &outline_polygon) {
                    outline_coverage += 1;
                }
                if point_in_polygon(base_x, base_y, &polygon) {
                    fill_coverage += 1;
                }
            }

            let color = match (outline_coverage, fill_coverage) {
                (0, _) => Rgba([0, 0, 0, 0]),
                (_, 0) => multiply_alpha(MARKER_OUTLINE_COLOR, outline_coverage),
                _ => multiply_alpha(MARKER_COLOR, fill_coverage),
            };

            image.put_pixel(pixel_x, pixel_y, color);
        }
    }

    image
}

fn multiply_alpha(color: [u8; 4], coverage: u32) -> Rgba<u8> {
    let alpha = (color[3] as u32 * coverage / 4) as u8;
    Rgba([color[0], color[1], color[2], alpha])
}

/// Even-odd point in polygon test.
fn point_in_polygon(x: f32, y: f32, polygon: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;

    for current in 0..polygon.len() {
        let (current_x, current_y) = polygon[current];
        let (previous_x, previous_y) = polygon[previous];

        let intersects =
            (current_y > y) != (previous_y > y) && x < (previous_x - current_x) * (y - current_y) / (previous_y - current_y) + current_x;

        if intersects {
            inside = !inside;
        }

        previous = current;
    }

    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(image: &RgbaImage, x: u32, y: u32) -> u8 {
        image.get_pixel(x, y)[3]
    }

    #[test]
    fn arrow_points_up_for_north() {
        let image = generate_arrow_image(0);
        let center = MARKER_TEXTURE_SIZE / 2;

        // The tip (above the center) must be filled, the bottom area behind the
        // notch must be empty.
        assert!(alpha_at(&image, center, 1) > 0);
        assert!(alpha_at(&image, center, MARKER_TEXTURE_SIZE - 1) == 0);
    }

    #[test]
    fn arrow_rotates_with_direction() {
        let north = generate_arrow_image(0);
        let east = generate_arrow_image(2);

        let center = MARKER_TEXTURE_SIZE / 2;
        assert!(alpha_at(&north, center, 1) > 0);
        assert!(alpha_at(&east, MARKER_TEXTURE_SIZE - 2, center) > 0);
        // The pixels that are filled for north are empty for east and vice versa.
        assert!(alpha_at(&east, center, 1) == 0);
        assert!(alpha_at(&north, MARKER_TEXTURE_SIZE - 2, center) == 0);
    }

    #[test]
    fn all_directions_have_visible_pixels() {
        for direction in 0..MARKER_DIRECTION_COUNT {
            let image = generate_arrow_image(direction);
            let visible = image.pixels().filter(|pixel| pixel[3] > 0).count();

            assert!(visible > 20, "direction {direction} is empty");
        }
    }
}
