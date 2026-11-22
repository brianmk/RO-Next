//! Runtime generation of per-map minimap textures.
//!
//! The CondaRO client archives do not contain the official per-map minimap
//! bitmaps (`data\minimap\<map>.bmp`), only the minimap UI icons. To still show
//! a useful minimap we generate a small top-down image from the map data that
//! is loaded anyway:
//!
//! - the average color of every ground surface texture is combined with the
//!   surface light color from the GND file,
//! - blocked (non-walkable) cells are darkened so buildings/walls show up as a
//!   darker footprint, and water cells are tinted blue using the GAT flags.
//!
//! Generation runs in the map loader thread and the resulting texture is stored
//! in the loaded [`Map`](crate::world::Map), so it is generated once per map
//! and benefits from the existing bounded map cache.

use image::RgbaImage;
use ragnarok_formats::map::{GatData, GroundData, TileFlags};

use crate::loaders::TextureLoader;
use crate::world::MinimapData;

/// Maximum size of the generated minimap texture in pixels. The window is much
/// smaller, this just keeps the image sharp enough when the window is resized
/// and keeps generation cheap.
const MAX_TEXTURE_DIMENSION: u32 = 384;

/// Number of texture samples per axis when averaging a ground texture.
const TEXTURE_SAMPLE_GRID: u32 = 24;

/// How much non-walkable cells are darkened (1.0 = no change).
const BLOCKED_BRIGHTNESS: f32 = 0.42;

/// How much water cells are tinted towards the water color.
const WATER_BLEND: f32 = 0.65;
const WATER_COLOR: [f32; 3] = [0.24, 0.34, 0.62];

const FALLBACK_SURFACE_COLOR: [f32; 3] = [0.38, 0.36, 0.33];

/// Generate a minimap for a map from its ground and walkability data.
///
/// Returns `None` if the map data is inconsistent or the texture can not be
/// created; the caller then simply shows an empty minimap window instead.
pub fn generate_minimap(
    resource_file: &str,
    ground_data: &GroundData,
    gat_data: &GatData,
    texture_loader: &TextureLoader,
) -> Option<MinimapData> {
    let cells_width = u32::try_from(gat_data.map_width).ok()?.max(1);
    let cells_height = u32::try_from(gat_data.map_height).ok()?.max(1);

    if gat_data.tiles.len() < (cells_width as usize) * (cells_height as usize) {
        return None;
    }

    let ground_width = ground_data.width.max(1) as u32;
    let ground_height = ground_data.height.max(1) as u32;

    let step = cells_width.max(cells_height).div_ceil(MAX_TEXTURE_DIMENSION).max(1);
    let map_width = cells_width.div_ceil(step);
    let map_height = cells_height.div_ceil(step);

    let surface_colors = ground_surface_colors(ground_data, texture_loader);

    let mut image_data = vec![0u8; (map_width as usize) * (map_height as usize) * 4];

    for pixel_y in 0..map_height {
        for pixel_x in 0..map_width {
            let mut color = [0.0f32; 3];
            let mut walkable_cells = 0;
            let mut water_cells = 0;
            let mut sampled_cells = 0;

            let first_cell_x = pixel_x * step;
            let first_cell_y = pixel_y * step;
            let last_cell_x = ((pixel_x + 1) * step).min(cells_width);
            let last_cell_y = ((pixel_y + 1) * step).min(cells_height);

            let mut tile_color = FALLBACK_SURFACE_COLOR;

            for cell_y in first_cell_y..last_cell_y {
                for cell_x in first_cell_x..last_cell_x {
                    let tile_x = (cell_x / 2) as usize;
                    let tile_y = (cell_y / 2) as usize;

                    if tile_x < ground_width as usize && tile_y < ground_height as usize {
                        let tile_index = tile_y * ground_width as usize + tile_x;
                        if let Some(ground_tile) = ground_data.ground_tiles.get(tile_index)
                            && let Some(surface_color) = surface_colors.get(ground_tile.top_surface_index.max(0) as usize)
                        {
                            tile_color = *surface_color;
                        }
                    }

                    let cell_index = (cell_y as usize) * (cells_width as usize) + (cell_x as usize);
                    if let Some(tile) = gat_data.tiles.get(cell_index) {
                        if tile.flags.contains(TileFlags::WALKABLE) {
                            walkable_cells += 1;
                        }
                        if tile.flags.contains(TileFlags::WATER) {
                            water_cells += 1;
                        }
                    }

                    color[0] += tile_color[0];
                    color[1] += tile_color[1];
                    color[2] += tile_color[2];
                    sampled_cells += 1;
                }
            }

            if sampled_cells == 0 {
                sampled_cells = 1;
            }

            let mut red = color[0] / sampled_cells as f32;
            let mut green = color[1] / sampled_cells as f32;
            let mut blue = color[2] / sampled_cells as f32;

            let walkable_fraction = walkable_cells as f32 / sampled_cells as f32;
            let water_fraction = water_cells as f32 / sampled_cells as f32;

            if walkable_fraction < 0.5 {
                let brightness = BLOCKED_BRIGHTNESS + (1.0 - BLOCKED_BRIGHTNESS) * walkable_fraction * 2.0;
                red *= brightness;
                green *= brightness;
                blue *= brightness;
            }

            if water_fraction > 0.25 {
                let blend = WATER_BLEND * (water_fraction - 0.25) / 0.75;
                red = red * (1.0 - blend) + WATER_COLOR[0] * blend;
                green = green * (1.0 - blend) + WATER_COLOR[1] * blend;
                blue = blue * (1.0 - blend) + WATER_COLOR[2] * blend;
            }

            let index = ((pixel_y as usize) * (map_width as usize) + (pixel_x as usize)) * 4;
            image_data[index] = (red.clamp(0.0, 1.0) * 255.0) as u8;
            image_data[index + 1] = (green.clamp(0.0, 1.0) * 255.0) as u8;
            image_data[index + 2] = (blue.clamp(0.0, 1.0) * 255.0) as u8;
            image_data[index + 3] = 255;
        }
    }

    // The ground/GAT rows are stored bottom-up (RO map y=0 is the south edge),
    // while the minimap window draws the texture top-down. Flip the rows so the
    // minimap matches the world orientation (previously it appeared upside down).
    let row_len = map_width as usize * 4;
    for y in 0..(map_height as usize / 2) {
        let top = y * row_len;
        let bottom = (map_height as usize - 1 - y) * row_len;
        for byte in 0..row_len {
            image_data.swap(top + byte, bottom + byte);
        }
    }

    let image = RgbaImage::from_raw(map_width, map_height, image_data)?;
    let texture = texture_loader.create_color(&format!("minimap/{resource_file}"), image, false);

    Some(MinimapData {
        texture,
        width: map_width as u16,
        height: map_height as u16,
        cells_width: cells_width.min(u16::MAX as u32) as u16,
        cells_height: cells_height.min(u16::MAX as u32) as u16,
    })
}

fn ground_surface_colors(ground_data: &GroundData, texture_loader: &TextureLoader) -> Vec<[f32; 3]> {
    let mut texture_colors: Vec<Option<[f32; 3]>> = vec![None; ground_data.textures.len()];

    ground_data
        .surfaces
        .iter()
        .map(|surface| {
            let texture_index = surface.texture_index.max(0) as usize;

            let base_color = match texture_colors.get(texture_index).copied().flatten() {
                Some(color) => color,
                None => {
                    let color = ground_data
                        .textures
                        .get(texture_index)
                        .and_then(|name| average_texture_color(name, texture_loader))
                        .unwrap_or(FALLBACK_SURFACE_COLOR);

                    if let Some(slot) = texture_colors.get_mut(texture_index) {
                        *slot = Some(color);
                    }

                    color
                }
            };

            let light_color = crate::graphics::Color::from(surface.color);
            let light = [light_color.red.max(0.05), light_color.green.max(0.05), light_color.blue.max(0.05)];

            [base_color[0] * light[0], base_color[1] * light[1], base_color[2] * light[2]]
        })
        .collect()
}

fn average_texture_color(texture_name: &str, texture_loader: &TextureLoader) -> Option<[f32; 3]> {
    let (image, _) = texture_loader.load_texture_data(texture_name).ok()?;

    let width = image.width().max(1);
    let height = image.height().max(1);
    let step_x = (width / TEXTURE_SAMPLE_GRID).max(1);
    let step_y = (height / TEXTURE_SAMPLE_GRID).max(1);

    let mut color = [0.0f64; 3];
    let mut samples = 0.0f64;

    let mut y = 0;
    while y < height {
        let mut x = 0;
        while x < width {
            let pixel = image.get_pixel(x, y);
            let alpha = pixel[3] as f64 / 255.0;

            if alpha > 0.05 {
                color[0] += pixel[0] as f64 / 255.0;
                color[1] += pixel[1] as f64 / 255.0;
                color[2] += pixel[2] as f64 / 255.0;
                samples += 1.0;
            }

            x += step_x;
        }

        y += step_y;
    }

    if samples < 1.0 {
        return None;
    }

    Some([
        (color[0] / samples) as f32,
        (color[1] / samples) as f32,
        (color[2] / samples) as f32,
    ])
}
