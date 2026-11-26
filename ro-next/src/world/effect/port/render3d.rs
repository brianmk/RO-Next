//! World-space rendering of the ported textured primitives.
//!
//! Covers the two primitive ids whose official render path builds a textured
//! world-space quad:
//!
//! * `PP_3DTEXTURE` (15) - `CEffectPrim::DrawEffectPrimitive15`,
//!   `EffectPrimitiveRendering.cpp:116-214`,
//! * `PP_3DCROSSTEXTURE` (16) - `CEffectPrim::RenderCrossPlanes`,
//!   `EffectPrimitiveRendering.cpp:6172-6253`.
//!
//! Geometry, corner order, texture selection and blending are derived from
//! those two functions; the motion/state (transform, corner pattern bits, UV
//! window, animated frame) is produced by the already-ported primitive state
//! and is only *read* here.
//!
//! ## Screen-space convention
//!
//! [`EffectRenderer::render_effect_with_depth`](crate::renderer::EffectRenderer::render_effect_with_depth)
//! places a quad around the projected anchor: each corner is an offset in
//! window pixels from the anchor and is passed as `EFFECT_ORIGIN + offset`,
//! exactly like `render_world_quad_blended` in `world/effect/mod.rs`. This
//! module does the same, but per corner: every corner is projected on its own
//! (so a 3-D quad may span several depths) and its NDC depth is passed in the
//! matching slot. Corners are re-ordered into the renderer's
//! `(top left, top right, bottom left, bottom right)` layout, keeping every
//! position paired with its official UV, and the UV array is permuted into the
//! order the renderer's quad slots sample (`bottom right, top right, top left,
//! bottom left`).
//!
//! ## What the official renderer reads
//!
//! * `PP_3DTEXTURE` is transformed by `state.rotTransform`, which
//!   `MoveEffectPrimitive` rebuilds every frame from
//!   `roll`/`latitude`/`longitude` (or from the camera for `pattern` bits
//!   `0x40`/`0x200`).
//! * `PP_3DCROSSTEXTURE` is transformed by `state.transform` (the body's fixed
//!   orientation), *not* `rotTransform` - a quirk of `RenderCrossPlanes` that
//!   the port keeps.
//! * `state.v1`/`state.v2` are the texture's V window (`pattern & 2` scrolls
//!   it); `state.curMotion` selects the entry of the body's texture array.

use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use wgpu::BlendFactor;

use super::super::{EFFECT_ORIGIN, order_quad, project_point};
use super::holder::primitive_color;
use super::primitive::{EffectPrimitiveState, PrimitiveId};
use crate::graphics::{Color, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// Whether this module draws the primitive.
pub(crate) fn is_world_space(state: &EffectPrimitiveState) -> bool {
    matches!(state.count, PrimitiveId::Texture3d | PrimitiveId::CrossTexture3d)
}

/// Draws one `PP_3DTEXTURE`/`PP_3DCROSSTEXTURE` primitive.
///
/// The primitive's own `render.position` is the quad anchor; a primitive whose
/// anchor or any corner is behind the camera is skipped (the official code
/// drops a triangle when `Visible()` reports `rhw <= 0`).
pub(crate) fn render_textured_primitive(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    textures: &TextureLoader,
    state: &EffectPrimitiveState,
) {
    let Some(path) = texture_path(state) else {
        return;
    };

    let Ok(texture) = textures.get_or_load(path, ImageType::Color) else {
        return;
    };

    let anchor = Point3::new(state.position[0], state.position[1], state.position[2]);
    let color = primitive_color(state);

    match state.count {
        PrimitiveId::Texture3d => {
            let (source, destination) = world_blend_factors(state.render_flag);

            draw_quad(
                renderer,
                camera,
                texture,
                anchor,
                texture3d_corners(state),
                texture3d_uvs(state),
                color,
                source,
                destination,
            );
        }
        PrimitiveId::CrossTexture3d => {
            // `RenderCrossPlanes` submits every triangle with flags `1`
            // (`RSF_ALPHA`), regardless of the body's `renderFlag`.
            let (source, destination) = world_blend_factors(1);

            for plane in 0..2 {
                draw_quad(
                    renderer,
                    camera,
                    texture.clone(),
                    anchor,
                    cross_plane_corners(state, plane),
                    cross_uvs(),
                    color,
                    source,
                    destination,
                );
            }
        }
        _ => {}
    }
}

/// `SubmitTriangle(triangle, flags)` blend selection.
///
/// `RSF_ADDITIVE` (`0x2`) is the additive queue (`One`/`One`); `RSF_ALPHA`
/// (`0x1`) and `RSF_NO_DEPTH` (`0x200`) both use the straight alpha queue
/// (`SrcAlpha`/`InvSrcAlpha`) in `RendererDX9::DrawAlphaQueues`. The 3-D
/// primitives never set `RSF_CUSTOM_BLEND`.
fn world_blend_factors(render_flag: u32) -> (BlendFactor, BlendFactor) {
    if (render_flag & 0x2) != 0 {
        (BlendFactor::One, BlendFactor::One)
    } else {
        (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha)
    }
}

/// The texture the official renderer would pick: `state.textures[curMotion]`.
///
/// The port stores that array either as the animated `texture_frames` sequence
/// (the motion already advances `curMotion`, wrapping at `totalTexture`) or as
/// the fixed `texture`/`texture_secondary`/`texture_tertiary` slots.
pub(crate) fn texture_path(state: &EffectPrimitiveState) -> Option<&'static str> {
    let index = frame_index(state);

    if !state.texture_frames.is_empty() {
        return state.texture_frames.get(index).copied();
    }

    match index {
        0 => state.texture,
        1 => state.texture_secondary,
        2 => state.texture_tertiary,
        _ => None,
    }
}

/// `state.curMotion`, clamped into the primitive's texture array.
///
/// The motion keeps `curMotion < totalTexture` (it wraps, or retires the
/// primitive when the animation does not repeat). The clamp only guards
/// against a body that under-reports `totalTexture`/`frame_count`; it never
/// advances the frame by itself.
pub(crate) fn frame_index(state: &EffectPrimitiveState) -> usize {
    let index = state.cur_motion as usize;

    match state.texture_frames.is_empty() {
        true => index.min(2),
        false => index.min(state.texture_frames.len() - 1),
    }
}

/// `DrawEffectPrimitive15`: the four world-space corners in the official
/// order, `(0,-h,w)`, `(0,-h,-w)`, `(0,h,w)`, `(0,h,-w)`, after the
/// `pattern` corner bits, the `rotTransform` rotation and `render.position`.
pub(crate) fn texture3d_corners(state: &EffectPrimitiveState) -> [Vector3<f32>; 4] {
    let width = state.width_size;
    let height = state.height_size;

    let mut corners = [
        Vector3::new(0.0, -height, width),
        Vector3::new(0.0, -height, -width),
        Vector3::new(0.0, height, width),
        Vector3::new(0.0, height, -width),
    ];

    if (state.pattern & 0x80) != 0 {
        corners[0].y += corners[0].y;
        corners[1].y += corners[1].y;
        corners[2].y = 0.0;
        corners[3].y = 0.0;
    }

    if (state.pattern & 8) != 0 {
        corners[2].y = 0.0;
        corners[3].y = 0.0;
    }

    corners.map(|corner| transform_point(corner, &state.rot_transform.values, state.position))
}

/// `DrawEffectPrimitive15` UVs: `v1` at the `-height` edge and `v2` at the
/// `+height` edge, `u` running `0..1` along `width`.
pub(crate) fn texture3d_uvs(state: &EffectPrimitiveState) -> [Vector2<f32>; 4] {
    [
        Vector2::new(0.0, state.v1),
        Vector2::new(1.0, state.v1),
        Vector2::new(0.0, state.v2),
        Vector2::new(1.0, state.v2),
    ]
}

/// `RenderCrossPlanes`: one of the two quads of the cross, in the official
/// corner order, after the body's `transform` and `render.position`.
///
/// Plane 0 spans the X-Z plane (`x = +-height`), plane 1 the Y-Z plane
/// (`y = +-height`); both span `z = +-width`.
pub(crate) fn cross_plane_corners(state: &EffectPrimitiveState, plane: usize) -> [Vector3<f32>; 4] {
    debug_assert!(plane < 2);

    let width = state.width_size;
    let height = state.height_size;

    let corners = match plane {
        0 => [
            Vector3::new(-height, 0.0, width),
            Vector3::new(-height, 0.0, -width),
            Vector3::new(height, 0.0, width),
            Vector3::new(height, 0.0, -width),
        ],
        _ => [
            Vector3::new(0.0, -height, width),
            Vector3::new(0.0, -height, -width),
            Vector3::new(0.0, height, width),
            Vector3::new(0.0, height, -width),
        ],
    };

    corners.map(|corner| transform_point(corner, &state.transform.values, state.position))
}

/// `RenderCrossPlanes` UVs: `v0`/`v1` share `v = 1`, `v2`/`v3` share `v = 0`;
/// `u` is `0` at `+width` and `1` at `-width`.
pub(crate) fn cross_uvs() -> [Vector2<f32>; 4] {
    [
        Vector2::new(0.0, 1.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
    ]
}

/// `TransformPrimitiveVertex` (a `matrix_mult`) plus `render.position`.
fn transform_point(point: Vector3<f32>, matrix: &[f32; 12], position: [f32; 3]) -> Vector3<f32> {
    let transformed = super::math::matrix_mult([point.x, point.y, point.z], matrix);

    Vector3::new(
        transformed[0] + position[0],
        transformed[1] + position[1],
        transformed[2] + position[2],
    )
}

/// A quad ready for [`EffectRenderer::render_effect_with_depth`]: anchor
/// relative corner offsets, slot-ordered UVs and per-corner NDC depths.
type RendererQuad = ([Vector2<f32>; 4], [Vector2<f32>; 4], [f32; 4]);

/// Projects the four corners and builds the arrays the effect renderer takes.
///
/// Returns `None` when the anchor or any corner is behind the camera (the
/// official `RenderTriangle::Visible` drops such triangles).
fn renderer_quad(
    camera: &dyn Camera,
    renderer: &EffectRenderer,
    anchor: Point3<f32>,
    corners: [Vector3<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
) -> Option<RendererQuad> {
    let screen_size = renderer.window_size();
    let (anchor_screen, _) = project_point(camera, anchor, screen_size)?;

    let mut screen_corners = [Vector2::new(0.0, 0.0); 4];
    let mut corner_depths = [0.0; 4];

    for (index, corner) in corners.iter().enumerate() {
        let (projected, depth) = project_point(camera, Point3::new(corner.x, corner.y, corner.z), screen_size)?;
        screen_corners[index] = projected - anchor_screen;
        corner_depths[index] = depth;
    }

    Some(renderer_quad_arrays(screen_corners, texture_coordinates, corner_depths))
}

/// Re-orders projected corners into the renderer layout and converts the
/// official UV order into the order of the renderer's quad slots.
pub(crate) fn renderer_quad_arrays(
    screen_corners: [Vector2<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
    corner_depths: [f32; 4],
) -> RendererQuad {
    let (corners, texture_coordinates, corner_depths) = order_quad(screen_corners, texture_coordinates, corner_depths);

    (
        corners.map(|corner| EFFECT_ORIGIN + corner),
        [
            texture_coordinates[3],
            texture_coordinates[1],
            texture_coordinates[0],
            texture_coordinates[2],
        ],
        corner_depths,
    )
}

#[allow(clippy::too_many_arguments)]
fn draw_quad(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    texture: Arc<Texture>,
    anchor: Point3<f32>,
    corners: [Vector3<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
    color: Color,
    source_blend_factor: BlendFactor,
    destination_blend_factor: BlendFactor,
) {
    let Some((screen_corners, texture_coordinates, corner_depths)) = renderer_quad(camera, renderer, anchor, corners, texture_coordinates)
    else {
        return;
    };

    renderer.render_effect_with_depth(
        camera,
        anchor,
        texture,
        screen_corners,
        corner_depths,
        texture_coordinates,
        Vector2::new(0.0, 0.0),
        Rad(0.0),
        color,
        source_blend_factor,
        destination_blend_factor,
    );
}

#[cfg(test)]
mod tests {
    use super::super::math::Matrix;
    use super::*;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState::default()
    }

    #[test]
    fn world_space_ids_are_the_two_textured_primitive_ids() {
        let mut state = state();

        state.count = PrimitiveId::Texture3d;
        assert!(is_world_space(&state));

        state.count = PrimitiveId::CrossTexture3d;
        assert!(is_world_space(&state));

        state.count = PrimitiveId::Texture2d;
        assert!(!is_world_space(&state));
    }

    #[test]
    fn default_texture3d_corners_follow_the_official_order() {
        let mut state = state();
        state.position = [10.0, 20.0, 30.0];
        state.width_size = 4.0;
        state.height_size = 8.0;

        let corners = texture3d_corners(&state);

        assert_eq!(corners, [
            Vector3::new(10.0, 12.0, 34.0),
            Vector3::new(10.0, 12.0, 26.0),
            Vector3::new(10.0, 28.0, 34.0),
            Vector3::new(10.0, 28.0, 26.0),
        ]);
    }

    #[test]
    fn texture3d_pattern_80_doubles_the_width_edge_and_flattens_the_other() {
        let mut state = state();
        state.width_size = 1.0;
        state.height_size = 2.0;
        state.pattern = 0x80;

        let corners = texture3d_corners(&state);

        assert_eq!(corners, [
            Vector3::new(0.0, -4.0, 1.0),
            Vector3::new(0.0, -4.0, -1.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0),
        ]);
    }

    #[test]
    fn texture3d_pattern_8_flattens_the_positive_height_edge() {
        let mut state = state();
        state.width_size = 1.0;
        state.height_size = 2.0;
        state.pattern = 8;

        let corners = texture3d_corners(&state);

        assert_eq!(corners[2], Vector3::new(0.0, 0.0, 1.0));
        assert_eq!(corners[3], Vector3::new(0.0, 0.0, -1.0));
        assert_eq!(corners[0], Vector3::new(0.0, -2.0, 1.0));
    }

    #[test]
    fn texture3d_corners_are_rotated_by_the_rotation_transform() {
        let mut state = state();
        state.width_size = 4.0;
        state.height_size = 8.0;
        // Exact 90 degree Z rotation: x' = -y, y' = x.
        state.rot_transform = Matrix {
            values: [0.0, 1.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        };

        let corners = texture3d_corners(&state);

        assert_eq!(corners, [
            Vector3::new(8.0, 0.0, 4.0),
            Vector3::new(8.0, 0.0, -4.0),
            Vector3::new(-8.0, 0.0, 4.0),
            Vector3::new(-8.0, 0.0, -4.0),
        ]);
    }

    #[test]
    fn texture3d_uvs_use_the_v1_v2_texel_window() {
        let mut state = state();
        state.v1 = 0.25;
        state.v2 = 0.75;

        assert_eq!(texture3d_uvs(&state), [
            Vector2::new(0.0, 0.25),
            Vector2::new(1.0, 0.25),
            Vector2::new(0.0, 0.75),
            Vector2::new(1.0, 0.75),
        ]);
    }

    #[test]
    fn the_state_default_covers_the_whole_texture() {
        // `EffectPrim.cpp:126-130` starts every primitive with `v1 = 0` and
        // `v2 = 1`; a 3-D texture quad would otherwise sample a single texel
        // row.
        let state = EffectPrimitiveState::default();

        assert_eq!(state.v1, 0.0);
        assert_eq!(state.v2, 1.0);
        assert_eq!(texture3d_uvs(&state), [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]);
    }

    #[test]
    fn first_cross_plane_spans_the_xz_plane() {
        let mut state = state();
        state.position = [1.0, 2.0, 3.0];
        state.width_size = 5.0;
        state.height_size = 7.0;

        assert_eq!(cross_plane_corners(&state, 0), [
            Vector3::new(-6.0, 2.0, 8.0),
            Vector3::new(-6.0, 2.0, -2.0),
            Vector3::new(8.0, 2.0, 8.0),
            Vector3::new(8.0, 2.0, -2.0),
        ]);
    }

    #[test]
    fn second_cross_plane_spans_the_yz_plane() {
        let mut state = state();
        state.position = [1.0, 2.0, 3.0];
        state.width_size = 5.0;
        state.height_size = 7.0;

        assert_eq!(cross_plane_corners(&state, 1), [
            Vector3::new(1.0, -5.0, 8.0),
            Vector3::new(1.0, -5.0, -2.0),
            Vector3::new(1.0, 9.0, 8.0),
            Vector3::new(1.0, 9.0, -2.0),
        ]);
    }

    #[test]
    fn cross_planes_use_the_body_transform_and_not_the_rotation_transform() {
        let mut state = state();
        state.width_size = 5.0;
        state.height_size = 7.0;
        // Exact 90 degree Y rotation: x' = z, z' = -x.
        state.transform = Matrix {
            values: [0.0, 0.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        // A translation the renderer must ignore: `RenderCrossPlanes` only
        // reads `state.transform`.
        state.rot_transform = Matrix {
            values: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 100.0, 100.0, 100.0],
        };

        assert_eq!(cross_plane_corners(&state, 1), [
            Vector3::new(5.0, -7.0, 0.0),
            Vector3::new(-5.0, -7.0, 0.0),
            Vector3::new(5.0, 7.0, 0.0),
            Vector3::new(-5.0, 7.0, 0.0),
        ]);
    }

    #[test]
    fn cross_uvs_match_the_official_assignment() {
        assert_eq!(cross_uvs(), [
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
        ]);
    }

    #[test]
    fn animated_frames_are_selected_by_cur_motion() {
        let mut state = state();
        state.texture_frames = vec!["effect\\a.bmp", "effect\\b.bmp", "effect\\c.bmp"];

        state.cur_motion = 0;
        assert_eq!(texture_path(&state), Some("effect\\a.bmp"));
        assert_eq!(frame_index(&state), 0);

        state.cur_motion = 1;
        assert_eq!(texture_path(&state), Some("effect\\b.bmp"));
        assert_eq!(frame_index(&state), 1);

        state.cur_motion = 2;
        assert_eq!(texture_path(&state), Some("effect\\c.bmp"));
        assert_eq!(frame_index(&state), 2);
    }

    #[test]
    fn fixed_textures_are_selected_by_cur_motion() {
        let mut state = state();
        state.texture = Some("effect\\primary.bmp");
        state.texture_secondary = Some("effect\\secondary.bmp");
        state.texture_tertiary = Some("effect\\tertiary.bmp");

        assert_eq!(texture_path(&state), Some("effect\\primary.bmp"));

        state.cur_motion = 1;
        assert_eq!(texture_path(&state), Some("effect\\secondary.bmp"));

        state.cur_motion = 2;
        assert_eq!(texture_path(&state), Some("effect\\tertiary.bmp"));

        state.cur_motion = 1;
        state.texture_secondary = None;
        assert_eq!(texture_path(&state), None);
    }

    #[test]
    fn frame_index_is_clamped_to_the_array_the_motion_indexes() {
        let mut state = state();
        state.texture_frames = vec!["effect\\a.bmp", "effect\\b.bmp"];

        // The motion wraps at `totalTexture`; the clamp only protects against
        // a body that under-reports it.
        state.cur_motion = 7;
        assert_eq!(frame_index(&state), 1);
        assert_eq!(texture_path(&state), Some("effect\\b.bmp"));

        state.texture_frames.clear();
        state.cur_motion = 9;
        assert_eq!(frame_index(&state), 2);
        assert_eq!(texture_path(&state), None);
    }

    #[test]
    fn additive_is_rsf_additive_and_not_rsf_no_depth() {
        assert_eq!(world_blend_factors(1), (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha));
        assert_eq!(world_blend_factors(2), (BlendFactor::One, BlendFactor::One));
        assert_eq!(world_blend_factors(3), (BlendFactor::One, BlendFactor::One));
        // `0x200` is `RSF_NO_DEPTH`, not an additive shorthand.
        assert_eq!(
            world_blend_factors(0x200),
            (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha)
        );
    }

    #[test]
    fn renderer_arrays_use_the_effect_origin_and_permuted_uvs() {
        let screen_corners = [
            Vector2::new(10.0, 10.0),
            Vector2::new(20.0, 10.0),
            Vector2::new(10.0, 30.0),
            Vector2::new(20.0, 30.0),
        ];
        let uvs = [
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
        ];
        let depths = [0.9, 0.8, 0.7, 0.6];

        let (corners, texture_coordinates, corner_depths) = renderer_quad_arrays(screen_corners, uvs, depths);

        // `EFFECT_ORIGIN` (319, 291) plus the anchor-relative offset.
        assert_eq!(corners, [
            Vector2::new(329.0, 301.0),
            Vector2::new(339.0, 301.0),
            Vector2::new(329.0, 321.0),
            Vector2::new(339.0, 321.0),
        ]);
        // Renderer slots: bottom right, top right, top left, bottom left.
        assert_eq!(texture_coordinates, [
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(0.0, 0.0),
        ]);
        assert_eq!(corner_depths, [0.9, 0.8, 0.7, 0.6]);
    }

    #[test]
    fn renderer_arrays_reorder_bottom_up_corners() {
        // A quad handed over in any winding still reaches the renderer as
        // top left, top right, bottom left, bottom right, with each UV kept
        // next to its corner.
        let screen_corners = [
            Vector2::new(30.0, 40.0),
            Vector2::new(10.0, 40.0),
            Vector2::new(30.0, 20.0),
            Vector2::new(10.0, 20.0),
        ];
        let uvs = [
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
        ];

        let (corners, texture_coordinates, _) = renderer_quad_arrays(screen_corners, uvs, [0.0; 4]);

        assert_eq!(corners, [
            Vector2::new(319.0 + 10.0, 291.0 + 20.0),
            Vector2::new(319.0 + 30.0, 291.0 + 20.0),
            Vector2::new(319.0 + 10.0, 291.0 + 40.0),
            Vector2::new(319.0 + 30.0, 291.0 + 40.0),
        ]);
        // Top left corner was index 3 (uv (1,0)); top right index 2 (uv
        // (0,0)); bottom left index 1 (uv (1,1)); bottom right index 0
        // (uv (0,1)).
        assert_eq!(texture_coordinates, [
            Vector2::new(0.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
        ]);
    }
}
