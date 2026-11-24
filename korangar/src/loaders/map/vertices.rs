use cgmath::{Point3, Vector2};
use ragnarok_formats::map::{GatData, GroundData, GroundTile, Surface, SurfaceType};
use smallvec::smallvec_inline;

use crate::graphics::{Color, ModelVertex, NativeModelVertex, PickerTarget, TileVertex, reduce_vertices};
use crate::loaders::map::{GAT_TILE_SIZE, GROUND_TILE_SIZE};
use crate::loaders::{TextureSetBuilder, TextureSetTexture, smooth_ground_normals};

#[derive(Copy, Clone, Debug)]
pub enum Heights {
    SouthWest,
    SouthEast,
    NorthWest,
    NorthEast,
}

/// Lightmap corner samples of an 8x8 GND lightmap (classic client
/// `GroundLightmapSet::Build`, `GroundLightmaps.cpp`): (1,1), (5,1), (1,5),
/// (5,5) of the 8x8 grid.
const LIGHTMAP_CORNER_SAMPLES: [[usize; 2]; 4] = [[1, 1], [5, 1], [1, 5], [5, 5]];

/// Decoded color of one corner of a GND lightmap, in linear RGB.
///
/// The GND 1.7+ lightmap is a sequence of `width * height / 64` 256-byte
/// lightmaps. Inside one lightmap the first quarter is the alpha channel and
/// the remaining three quarters are the RGB channels; a corner color is the
/// classic `(rgb / 128) + alpha` light contribution, normalized to `1.0`.
/// Returns `None` when the data set carries no lightmap data for the surface.
fn lightmap_corner_color(ground_data: &GroundData, light_map_index: i16, corner: usize) -> Option<Color> {
    if light_map_index < 0 {
        return None;
    }

    let data = ground_data.light_map_data.as_deref()?;
    let width = usize::try_from(ground_data.light_map_width).ok()?;
    let height = usize::try_from(ground_data.light_map_height).ok()?;
    if width == 0 || height == 0 {
        return None;
    }

    let light_map_count = usize::try_from(ground_data.light_map_count).ok()?;
    let index = usize::try_from(light_map_index).ok()?;
    if index >= light_map_count {
        return None;
    }

    let pixels = width.checked_mul(height)?;
    let per_map = pixels.checked_mul(4)?;
    let start = index.checked_mul(per_map)?;
    let map = data.get(start..start.checked_add(per_map)?)?;

    decode_lightmap_corner(map, width, height, corner)
}

/// Decodes one corner of a single 256-byte GND lightmap.
///
/// The first quarter of the lightmap is the alpha channel and the remaining
/// three quarters are the RGB channels. `GroundGeometry::InitializeLighting`
/// uses `rgb * 0.0078125 (= /128) + alpha` and normalizes the brightest channel
/// to `1.0`.
fn decode_lightmap_corner(map: &[u8], width: usize, height: usize, corner: usize) -> Option<Color> {
    if width == 0 || height == 0 {
        return None;
    }

    let pixels = width.checked_mul(height)?;
    if map.len() < pixels.checked_mul(4)? {
        return None;
    }

    // The official client samples the inset corners of the 8x8 lightmap; the
    // CPU vertex-color approximation maps the four samples to the surface
    // corners (south-west, south-east, north-west, north-east).
    let [sample_x, sample_y] = LIGHTMAP_CORNER_SAMPLES[corner.min(3)];
    let x = (sample_x * width / 8).min(width - 1);
    let y = (sample_y * height / 8).min(height - 1);
    let pixel = y * width + x;

    let alpha = f32::from(*map.get(pixel)?) / 255.0;
    let red = f32::from(*map.get(pixels + pixel * 3)?) / 128.0 + alpha;
    let green = f32::from(*map.get(pixels + pixel * 3 + 1)?) / 128.0 + alpha;
    let blue = f32::from(*map.get(pixels + pixel * 3 + 2)?) / 128.0 + alpha;

    let largest = red.max(green).max(blue);
    if largest > 1.0 {
        Some(Color::rgb(red / largest, green / largest, blue / largest))
    } else {
        Some(Color::rgb(red, green, blue))
    }
}

pub fn ground_vertices(
    ground_data: &GroundData,
    texture_set_builder: &mut TextureSetBuilder,
) -> (Vec<ModelVertex>, Vec<u32>, Vec<TextureSetTexture>) {
    let ground_textures: Vec<TextureSetTexture> = ground_data
        .textures
        .iter()
        .map(|texture| texture_set_builder.register(texture))
        .collect();

    let mut ground_vertices = Vec::new();

    let width = ground_data.width as usize;
    let ground_tiles = &ground_data.ground_tiles;

    for (index, current_tile) in ground_tiles.iter().enumerate() {
        let tile_x = index % width;
        let tile_y = index / width;

        for surface_type in [SurfaceType::North, SurfaceType::East, SurfaceType::Top] {
            let surface_index = tile_surface_index(current_tile, surface_type);

            if surface_index.is_negative() {
                continue;
            }

            let surface_alignment = tile_surface_alignment(surface_type);

            let position = |alignment_index: usize, tile_for_height: &GroundTile| {
                let (surface_offset, surface_height) = surface_alignment[alignment_index];
                let height = get_tile_height_at(tile_for_height, surface_height);
                Point3::new(
                    (tile_x + surface_offset.x) as f32 * GROUND_TILE_SIZE,
                    -height,
                    (tile_y + surface_offset.y) as f32 * GROUND_TILE_SIZE,
                )
            };

            let (first_position, second_position, third_position, fourth_position) = match surface_type {
                SurfaceType::North | SurfaceType::East => {
                    let neighbor_tile_index = neighbor_tile_index(surface_type);

                    let neighbor_x = tile_x + neighbor_tile_index.x;
                    let neighbor_y = tile_y + neighbor_tile_index.y;

                    let Some(neighbor_tile) = ground_tiles.get(neighbor_x + neighbor_y * width) else {
                        continue;
                    };

                    (
                        position(0, current_tile),
                        position(1, current_tile),
                        position(2, neighbor_tile),
                        position(3, neighbor_tile),
                    )
                }
                SurfaceType::Top => (
                    position(0, current_tile),
                    position(1, current_tile),
                    position(2, current_tile),
                    position(3, current_tile),
                ),
            };

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

            let ground_surface = &ground_data.surfaces[surface_index as usize];

            let (first_texture_coordinates, second_texture_coordinates, third_texture_coordinates, fourth_texture_coordinates) =
                map_texture_coordinates(&ground_textures, ground_surface);

            let neighbor_color = |x_offset: usize, y_offset: usize| {
                let Some(neighbor_tile) = ground_tiles.get(tile_x + x_offset + (tile_y + y_offset) * width) else {
                    return ground_surface.color.into();
                };

                // FIX: It is almost certainly incorrect to use the top face in all cases.
                let neighbor_surface_index = tile_surface_index(neighbor_tile, SurfaceType::Top);
                let Some(neighbor_surface) = ground_data.surfaces.get(neighbor_surface_index as usize) else {
                    return ground_surface.color.into();
                };

                neighbor_surface.color.into()
            };

            let color_east = neighbor_color(1, 0);
            let color_north_east = neighbor_color(1, 1);
            let color_north = neighbor_color(0, 1);

            // GND lightmaps: the surface color is modulated by the decoded
            // corner light of the surface's lightmap. Surfaces without a
            // lightmap (or maps without lightmap data) keep their color, so
            // maps without data render exactly as before.
            let corner_color = |corner: usize, base: Color| match lightmap_corner_color(ground_data, ground_surface.light_map_index, corner)
            {
                Some(light) => Color {
                    red: base.red * light.red,
                    green: base.green * light.green,
                    blue: base.blue * light.blue,
                    alpha: base.alpha,
                },
                None => base,
            };

            let color_first = corner_color(0, ground_surface.color.into());
            let color_second = corner_color(1, color_east);
            let color_third = corner_color(2, color_north);
            let color_fourth = corner_color(3, color_north_east);

            if let Some(first_normal) = first_normal {
                ground_vertices.push(NativeModelVertex::new(
                    first_position,
                    first_normal,
                    first_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_first,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    second_position,
                    first_normal,
                    second_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_second,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    third_position,
                    first_normal,
                    third_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_third,
                    0.0,
                    smallvec_inline![0;3],
                ));
            }

            if let Some(second_normal) = second_normal {
                ground_vertices.push(NativeModelVertex::new(
                    third_position,
                    second_normal,
                    third_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_third,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    second_position,
                    second_normal,
                    second_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_second,
                    0.0,
                    smallvec_inline![0;3],
                ));
                ground_vertices.push(NativeModelVertex::new(
                    fourth_position,
                    second_normal,
                    fourth_texture_coordinates,
                    ground_surface.texture_index as i32,
                    color_fourth,
                    0.0,
                    smallvec_inline![0;3],
                ));
            }
        }
    }

    smooth_ground_normals(&mut ground_vertices);

    let vertices = NativeModelVertex::convert_to_model_vertices(ground_vertices, Some(&ground_textures));

    let (reduced_vertices, indices) = reduce_vertices(&vertices);

    (reduced_vertices, indices, ground_textures)
}

/// Linear mapping to inset texture coordinates by half a pixel on each edge to
/// counter texture bleeding.
fn map_texture_coordinates(
    ground_textures: &[TextureSetTexture],
    ground_surface: &Surface,
) -> (Vector2<f32>, Vector2<f32>, Vector2<f32>, Vector2<f32>) {
    let texture = ground_textures[ground_surface.texture_index as usize];

    let half_pixel_width = 0.5 / texture.width as f32;
    let half_pixel_height = 0.5 / texture.height as f32;

    let first_texture_coordinates = Vector2::new(
        half_pixel_width + ground_surface.u[0] * (1.0 - 2.0 * half_pixel_width),
        half_pixel_height + ground_surface.v[0] * (1.0 - 2.0 * half_pixel_height),
    );
    let second_texture_coordinates = Vector2::new(
        half_pixel_width + ground_surface.u[1] * (1.0 - 2.0 * half_pixel_width),
        half_pixel_height + ground_surface.v[1] * (1.0 - 2.0 * half_pixel_height),
    );
    let third_texture_coordinates = Vector2::new(
        half_pixel_width + ground_surface.u[2] * (1.0 - 2.0 * half_pixel_width),
        half_pixel_height + ground_surface.v[2] * (1.0 - 2.0 * half_pixel_height),
    );
    let fourth_texture_coordinates = Vector2::new(
        half_pixel_width + ground_surface.u[3] * (1.0 - 2.0 * half_pixel_width),
        half_pixel_height + ground_surface.v[3] * (1.0 - 2.0 * half_pixel_height),
    );

    (
        first_texture_coordinates,
        second_texture_coordinates,
        third_texture_coordinates,
        fourth_texture_coordinates,
    )
}

pub fn generate_tile_vertices(gat_data: &mut GatData) -> (Vec<ModelVertex>, Vec<u32>, Vec<TileVertex>, Vec<u32>) {
    #[allow(unused_mut)]
    let mut tile_vertices = Vec::new();
    let mut tile_picker_vertices = Vec::new();

    for tile in gat_data.tiles.iter_mut() {
        tile.southwest_corner_height = -tile.southwest_corner_height;
        tile.southeast_corner_height = -tile.southeast_corner_height;
        tile.northwest_corner_height = -tile.northwest_corner_height;
        tile.northeast_corner_height = -tile.northeast_corner_height;
    }

    let tile_picker_indices = gat_data
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| !tile.flags.is_empty())
        .flat_map(|(index, tile)| {
            let x = index % gat_data.map_width as usize;
            let y = index / gat_data.map_width as usize;

            let offset = Vector2::new(x as f32 * GAT_TILE_SIZE, y as f32 * GAT_TILE_SIZE);

            #[cfg(feature = "debug")]
            {
                const TILE_MESH_OFFSET: f32 = 0.9;

                let first_position = Point3::new(offset.x, tile.southwest_corner_height + TILE_MESH_OFFSET, offset.y);
                let second_position = Point3::new(
                    offset.x + GAT_TILE_SIZE,
                    tile.southeast_corner_height + TILE_MESH_OFFSET,
                    offset.y,
                );
                let third_position = Point3::new(
                    offset.x,
                    tile.northwest_corner_height + TILE_MESH_OFFSET,
                    offset.y + GAT_TILE_SIZE,
                );
                let fourth_position = Point3::new(
                    offset.x + GAT_TILE_SIZE,
                    tile.northeast_corner_height + TILE_MESH_OFFSET,
                    offset.y + GAT_TILE_SIZE,
                );

                let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
                let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

                let tile_type_index = TryInto::<u8>::try_into(tile.flags).unwrap() as usize;

                let first_texture_coordinates = Vector2::new(0.0, 1.0);
                let second_texture_coordinates = Vector2::new(1.0, 1.0);
                let third_texture_coordinates = Vector2::new(0.0, 0.0);
                let fourth_texture_coordinates = Vector2::new(1.0, 0.0);

                if let Some(first_normal) = first_normal {
                    tile_vertices.push(ModelVertex::new(
                        first_position,
                        first_normal,
                        first_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        second_position,
                        first_normal,
                        second_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        third_position,
                        first_normal,
                        third_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                }

                if let Some(second_normal) = second_normal {
                    tile_vertices.push(ModelVertex::new(
                        third_position,
                        second_normal,
                        third_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        second_position,
                        second_normal,
                        second_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                    tile_vertices.push(ModelVertex::new(
                        fourth_position,
                        second_normal,
                        fourth_texture_coordinates,
                        Color::WHITE,
                        tile_type_index as i32,
                        0.0,
                    ));
                }
            }

            let first_position = Point3::new(offset.x, tile.southwest_corner_height, offset.y);
            let second_position = Point3::new(offset.x + GAT_TILE_SIZE, tile.southeast_corner_height, offset.y);
            let third_position = Point3::new(offset.x, tile.northwest_corner_height, offset.y + GAT_TILE_SIZE);
            let fourth_position = Point3::new(offset.x + GAT_TILE_SIZE, tile.northeast_corner_height, offset.y + GAT_TILE_SIZE);

            let (_, color) = PickerTarget::Tile { x: x as u16, y: y as u16 }.into();

            let offset = tile_picker_vertices.len() as u32;

            tile_picker_vertices.push(TileVertex::new(first_position, color));
            tile_picker_vertices.push(TileVertex::new(second_position, color));
            tile_picker_vertices.push(TileVertex::new(third_position, color));
            tile_picker_vertices.push(TileVertex::new(fourth_position, color));

            // Since the tile position is encoded in the vertex color, vertices of tiles
            // never share vertices, so we know the correct, minimal indices.
            [offset, offset + 1, offset + 2, offset + 2, offset + 1, offset + 3]
        })
        .collect();

    let (reduced_tile_vertices, tile_indices) = reduce_vertices(&tile_vertices);

    (reduced_tile_vertices, tile_indices, tile_picker_vertices, tile_picker_indices)
}

pub fn tile_surface_index(tile: &GroundTile, surface_type: SurfaceType) -> i32 {
    match surface_type {
        SurfaceType::North => tile.north_surface_index,
        SurfaceType::East => tile.east_surface_index,
        SurfaceType::Top => tile.top_surface_index,
    }
}

pub fn get_tile_height_at(tile: &GroundTile, point: Heights) -> f32 {
    match point {
        Heights::SouthWest => tile.southwest_corner_height,
        Heights::SouthEast => tile.southeast_corner_height,
        Heights::NorthWest => tile.northwest_corner_height,
        Heights::NorthEast => tile.northeast_corner_height,
    }
}

pub fn tile_surface_alignment(surface_type: SurfaceType) -> [(Vector2<usize>, Heights); 4] {
    match surface_type {
        SurfaceType::North => [
            (Vector2::new(0, 1), Heights::NorthWest),
            (Vector2::new(1, 1), Heights::NorthEast),
            (Vector2::new(0, 1), Heights::SouthWest),
            (Vector2::new(1, 1), Heights::SouthEast),
        ],
        SurfaceType::East => [
            (Vector2::new(1, 1), Heights::NorthEast),
            (Vector2::new(1, 0), Heights::SouthEast),
            (Vector2::new(1, 1), Heights::NorthWest),
            (Vector2::new(1, 0), Heights::SouthWest),
        ],
        SurfaceType::Top => [
            (Vector2::new(0, 0), Heights::SouthWest),
            (Vector2::new(1, 0), Heights::SouthEast),
            (Vector2::new(0, 1), Heights::NorthWest),
            (Vector2::new(1, 1), Heights::NorthEast),
        ],
    }
}

pub fn neighbor_tile_index(surface_type: SurfaceType) -> Vector2<usize> {
    match surface_type {
        SurfaceType::North => Vector2::new(0, 1),
        SurfaceType::East => Vector2::new(1, 0),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_lightmap_corner;

    fn lightmap(pixel_red: u8, alpha: u8) -> Vec<u8> {
        let mut map = vec![0u8; 64 * 4];
        // The alpha channel is the first 64 bytes.
        map[9] = alpha;
        // The RGB channels follow: 3 bytes per pixel.
        map[64 + 9 * 3] = pixel_red;
        map
    }

    #[test]
    fn decodes_the_official_corner_samples() {
        // 8x8 lightmap, corner 0 samples pixel (1,1) = 9, rgb 128 -> red 1.0.
        let color = decode_lightmap_corner(&lightmap(128, 0), 8, 8, 0).expect("8x8 lightmap must decode");
        assert!((color.red - 1.0).abs() < 0.001);
        assert!(color.green.abs() < 0.001);
        assert!(color.blue.abs() < 0.001);

        // The other corners sample (5,1), (1,5) and (5,5), which are black in
        // this fixture.
        for corner in 1..4 {
            let color = decode_lightmap_corner(&lightmap(128, 0), 8, 8, corner).unwrap();
            assert!(color.red.abs() < 0.001, "corner {corner} must be black");
        }
    }

    #[test]
    fn alpha_contributes_to_the_light() {
        // alpha = 255 alone already gives a white light contribution.
        let color = decode_lightmap_corner(&lightmap(0, 255), 8, 8, 0).unwrap();
        assert!((color.red - 1.0).abs() < 0.001);
        assert!((color.green - 1.0).abs() < 0.001);
        assert!((color.blue - 1.0).abs() < 0.001);
    }

    #[test]
    fn malformed_lightmaps_are_ignored() {
        assert!(decode_lightmap_corner(&[], 8, 8, 0).is_none());
        assert!(decode_lightmap_corner(&[0; 255], 8, 8, 0).is_none());
        assert!(decode_lightmap_corner(&[0; 256], 0, 8, 0).is_none());
    }
}
