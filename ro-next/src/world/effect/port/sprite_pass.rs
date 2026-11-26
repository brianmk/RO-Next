//! Sprite rendering of the ported primitives -
//! `CEffectPrim::RenderSegmentSprites2`, `EffectPrimitiveRendering.cpp:
//! 7593-7662`.
//!
//! Every primitive whose body attaches a `.spr`/`.act` pair (the
//! `PP_3DPARTICLE` / `PP_3DPARTICLEGRAVITY` / `PP_3DPARTICLEORBIT` families and
//! the `TexturedClassChange` bodies) is drawn here: one screen-space quad per
//! ACT cell of every segment record.
//!
//! ## What the original does
//!
//! For each `seg` of `state.numSegment`:
//!
//! 1. `ProjectViewport(record.position)` gives the anchor `(screenX, screenY)`,
//!    `ScaleDistance(reciprocalDepth)` gives the world-to-screen factor and
//!    `ScaleByMaster` may scale it further.
//! 2. `action->Frame(previousAnimation, curMotion)` selects the ACT motion.
//! 3. For every cell with `image != -1`: the cell's `x`/`y` and the sprite
//!    image size are multiplied by `record.scale`, the cell's `scaleX`/
//!    `scaleY` and `distanceScale`, and added to the anchor. `mirrored` swaps
//!    the horizontal edges, `cell.rotation + state.roll` rotates the quad
//!    around its own centre in `DrawQuad`, and the colour is `record.color` -
//!    or `cell.colorChannels` when `pattern & 0x200`.
//!
//! ## Port decisions
//!
//! * **Projection.** The official quad is a *billboard in screen space*, not a
//!   world-space polygon, so the pass calls
//!   [`EffectRenderer::render_effect`](crate::renderer::EffectRenderer::render_effect):
//!   all four corners share the record anchor's clip-space depth.
//!   `render_effect_with_depth` (used by the `PP_3DTEXTURE` pass) needs real
//!   world-space corners and per-corner depths, which a billboard does not
//!   have. The official per-top/bottom `rhw` values are the DX9 queue's
//!   screen-space depth approximation (`g_tangent`), not world depths; the
//!   port's real depth buffer gets the anchor depth instead, the same depth an
//!   actor sprite at that position would write.
//! * **Distance scale.** `ScaleDistance` is `viewport.scaleX * reciprocalDepth
//!   / 7`; the `7` is the official world units per sprite pixel (the port's
//!   actor sprites use `SPRITE_SCALE / TILE_SIZE = 1.4 / 10` for the same
//!   conversion). The pass measures the port camera's pixels-per-world-unit at
//!   the anchor and divides by 7, so the cell scales with distance exactly like
//!   the original.
//! * **Cell position.** The raw ACT `x`/`y` is the *centre* of the cell.
//!   `CActRes::Load` rewrites it to a top-left offset with
//!   `AdjustCellPosition`/`RelativeCoordinate` (dividing out the cell scale and
//!   truncating to whole pixels), and that is the value `RenderSegmentSprites2`
//!   consumes. The port applies the same transformation in
//!   [`adjusted_cell_position`]. The quad extent itself always comes from the
//!   loaded sprite image, like `image->width * (1 + image->doubleWidth)` in the
//!   original.
//! * **Frame.** `previousAnimation` is the primitive's `action_index` (the
//!   unset `-1` is the constructor's `0`). `curMotion` is advanced by
//!   `AnimMotion` (`stateCnt % animSpeed`); the pass derives it from
//!   `state.state_cnt`/`anim_speed`/`repeat_anim`/`action_frame_count` so a
//!   primitive whose motion code does not advance `cur_motion` still renders
//!   the official frame. See [`sprite_frame_index`].
//! * **ACT durations.** `CActRes::Speed` (the ACT delay) feeds
//!   `CRenderObject::render.speed`, which effect primitives never read;
//!   `AnimMotion` only uses `state.animSpeed`. The delays are therefore not an
//!   input to this pass.

use cgmath::{InnerSpace, Point3, Rad, Vector2};
use ragnarok_formats::action::{Action, SpriteClip};
use wgpu::BlendFactor;

use super::super::{EFFECT_ORIGIN, project_point};
use super::angles::Angles;
use super::primitive::{EffectPrimitiveState, PrimitiveRecord};
use crate::graphics::{Color, ScreenSize};
use crate::loaders::{ActionLoader, SpriteLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// `CRenderer::ScaleDistance` divides by this: the official world units per
/// sprite pixel (the port's actor sprites use `SPRITE_SCALE / TILE_SIZE =
/// 0.14`, i.e. the same `1 / 7`).
const WORLD_UNITS_PER_SPRITE_PIXEL: f32 = 7.0;

/// `RSF_CUSTOM_BLEND` (0x4), `RSF_TOPMOST` (0x8), `RSF_ADDITIVE` (0x2).
const RSF_ADDITIVE: u32 = 0x2;
const RSF_CUSTOM_BLEND: u32 = 0x4;
const RSF_TOPMOST: u32 = 0x8;
/// `pattern & 0x200` switches a sprite primitive to the per-cell ACT colour.
const PATTERN_CELL_COLOR: u32 = 0x200;

/// The four corners/UVs of one ACT cell, already in the renderer's layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CellQuad {
    /// Flat index into `Sprite::textures` (`palette_size` already added for
    /// `imageType == 1`).
    pub texture_index: usize,
    /// Corner offsets from the projected record anchor, in window pixels, in
    /// the order top left, top right, bottom left, bottom right.
    pub corners: [Vector2<f32>; 4],
    /// UVs in the order the effect renderer samples:
    /// bottom right, top right, top left, bottom left.
    pub texture_coordinates: [Vector2<f32>; 4],
    /// `record.color`, or the cell colour when `pattern & 0x200`.
    pub color: Color,
}

/// The action row the original renders: `render.previousAnimation`.
///
/// The port stores `-1` for "never called `SetAction`"; the official
/// `CEffectPrim` constructor leaves `previousAnimation = 0`
/// (`EffectPrim.cpp:123`), so an unset row is row 0.
pub(crate) fn action_row(state: &EffectPrimitiveState) -> usize {
    if state.action_index < 0 { 0 } else { state.action_index as usize }
}

/// `CActRes::AdjustCellPosition` / `RelativeCoordinate`
/// (`ActionResource.cpp:16-23`, `:61-73`): the raw ACT position is the centre
/// of the cell, and the official loader rewrites it to the top-left corner
/// relative to the anchor, dividing out the cell scale and truncating to whole
/// pixels.
///
/// `RenderSegmentSprites2` then multiplies the stored value by the cell scale
/// again, so the port has to apply this transformation before it can use the
/// raw [`SpriteClip::position`].
pub(crate) fn adjusted_cell_position(position: i32, extent: f32, scale: f32) -> f32 {
    // `scale == 0` is UB in the original (`coordinate / 0` cast to int); the
    // render multiplies the result by 0, so the net offset is zero.
    if scale == 0.0 {
        return 0.0;
    }

    let extent = extent as i32;
    let center = extent / 2 + extent % 2;
    let offset = center as f32 * scale;

    ((position as f32 - offset) / scale) as i32 as f32
}

/// `CActRes::FrameCount(animation)` - the number of motions in one ACT action
/// row (never less than one, like the original).
///
/// `AnimMotion` uses this count both as the wrap length and as the retirement
/// test for `repeatAnim == 0`, so the holder has to fill
/// [`EffectPrimitiveState::action_frame_count`] from the loaded file before the
/// primitive's first step.
pub(crate) fn action_frame_count(actions: &[Action], row: usize) -> Option<u32> {
    actions.get(row).map(|action| action.motions.len().max(1) as u32)
}

/// The `curMotion` `AnimMotion` would have after `state.state_cnt` processed
/// frames: an increment every `anim_speed` frames, wrapping (or stopping on
/// the last frame when `repeat_anim == 0`).
///
/// `AnimMotion` runs before `OnProcess` increments `stateCnt`, so after `n`
/// frames the increment count is `ceil(n / animSpeed)`; the extra frame is
/// what makes this match the official `render.curMotion` exactly (see the
/// `frame_selection_matches_the_simulated_cur_motion` test).
pub(crate) fn sprite_frame_index(state: &EffectPrimitiveState, frame_count: usize) -> usize {
    let frame_count = frame_count.max(1) as u64;
    let anim_speed = state.anim_speed.max(1) as u64;
    let ticks = (state.state_cnt.max(0) as u64).div_ceil(anim_speed);

    if state.repeat_anim == 0 {
        // `AnimMotion` clamps to the last frame for `pattern & 0x800`; without
        // that bit it retires the primitive when it reaches `FrameCount`, so
        // an alive primitive is always below the count and the clamp is only
        // a guard.
        ticks.min(frame_count - 1) as usize
    } else {
        (ticks % frame_count) as usize
    }
}

/// `DrawQuad`'s queue selection for a sprite quad (`RenderQueues.cpp:16-45`,
/// `QueuedDrawingDX9.cpp:155-165`).
///
/// `None` means the queue's blend state cannot be derived faithfully from the
/// decompilation; the primitive is left undrawn instead of guessed.
pub(crate) fn sprite_blend_factors(render_flag: u32) -> Option<(BlendFactor, BlendFactor)> {
    // `queue_0210` is drawn with whatever blend state the previous queue left
    // behind; the decompilation does not define it. No ported sprite body sets
    // `RSF_TOPMOST`, so leave it undrawn rather than guess.
    if (render_flag & RSF_TOPMOST) != 0 {
        return None;
    }

    // `RSF_CUSTOM_BLEND` routes to `queue_01EC`, which `QueuedDrawingDX9`
    // explicitly draws with `D3DBLEND_ONE`/`D3DBLEND_ONE` (additive).
    if (render_flag & RSF_CUSTOM_BLEND) != 0 {
        return Some((BlendFactor::One, BlendFactor::One));
    }

    // `RSF_ADDITIVE` -> the additive queue.
    if (render_flag & RSF_ADDITIVE) != 0 {
        return Some((BlendFactor::One, BlendFactor::One));
    }

    // `RSF_ALPHA` (and `RSF_NO_DEPTH`, which only changes the queue, not the
    // blend) -> straight alpha, translated to premultiplied by the effect
    // pipeline exactly like the 3-D pass' `SrcAlpha`/`InvSrcAlpha` pair.
    Some((BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha))
}

/// Flat `Sprite::textures` index of a cell, `None` for transparent cells and
/// for `imageType` values the original does not have (`images[2]`).
pub(crate) fn cell_texture_index(cell: &SpriteClip, palette_size: usize) -> Option<usize> {
    if cell.sprite_number < 0 {
        return None;
    }

    match cell.sprite_type.unwrap_or(0) {
        0 => Some(cell.sprite_number as usize),
        1 => Some(palette_size + cell.sprite_number as usize),
        _ => None,
    }
}

fn plain_uvs() -> [Vector2<f32>; 4] {
    [
        Vector2::new(1.0, 1.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(0.0, 0.0),
        Vector2::new(0.0, 1.0),
    ]
}

fn mirrored_uvs() -> [Vector2<f32>; 4] {
    [
        Vector2::new(0.0, 1.0),
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(1.0, 1.0),
    ]
}

/// `quad.color = record.color` (A,R,G,B) -> [`Color`].
fn record_color(color: u32) -> Color {
    Color::rgba_u8(
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
        ((color >> 24) & 0xFF) as u8,
    )
}

/// The ACT cell colour. The file stores the four channel bytes as
/// red, green, blue, alpha (little endian `u32` = R | G << 8 | B << 16 |
/// A << 24), while `DrawQuad` re-packs it into the A,R,G,B layout the renderer
/// uses; this decodes the file layout.
fn cell_color(color: u32) -> Color {
    Color::rgba_u8(
        (color & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        ((color >> 16) & 0xFF) as u8,
        ((color >> 24) & 0xFF) as u8,
    )
}

/// Converts the camera's pixels-per-world-unit at `anchor` into the official
/// `ScaleDistance` value (see the module docs).
///
/// `project_point` of the anchor and of the anchor moved one world unit along
/// the camera's right axis gives the local scale; using the real window size
/// makes the result window-pixel based, which is the unit the effect renderer
/// treats its corner offsets as.
pub(crate) fn distance_scale(camera: &dyn Camera, anchor: Point3<f32>, window: ScreenSize) -> Option<f32> {
    let (origin, _) = project_point(camera, anchor, window)?;

    let right = camera.look_up_vector().cross(camera.view_direction());

    if right.magnitude2() <= f32::EPSILON {
        return None;
    }

    let right = right.normalize();
    let (offset, _) = project_point(camera, anchor + right, window)?;
    let pixels_per_world_unit = (offset - origin).magnitude();

    if !pixels_per_world_unit.is_finite() || pixels_per_world_unit <= 0.0 {
        return None;
    }

    Some(pixels_per_world_unit / WORLD_UNITS_PER_SPRITE_PIXEL)
}

/// Builds the quads of one segment record, in window-pixel offsets from the
/// projected anchor.
///
/// `texture_size` resolves the image size for cells whose ACT has no explicit
/// `size` (versions before 2.5); the caller passes the loaded texture size.
#[allow(clippy::too_many_arguments)]
pub(crate) fn cell_quads(
    state: &EffectPrimitiveState,
    record: &PrimitiveRecord,
    cells: &[SpriteClip],
    viewport: ScreenSize,
    anchor_screen: Vector2<f32>,
    distance_scale: f32,
    palette_size: usize,
    texture_size: impl Fn(usize) -> Option<Vector2<f32>>,
) -> Vec<CellQuad> {
    let mut quads = Vec::with_capacity(cells.len());
    let angles = Angles::shared();

    for cell in cells {
        let Some(texture_index) = cell_texture_index(cell, palette_size) else {
            continue;
        };

        // `AdjustCellPosition` uses the ACT's stored image size for ACT >= 2.5
        // and the sprite image size for older files; `RenderSegmentSprites2`
        // always uses the sprite image size for the quad extent.
        let Some(texture_extent) = texture_size(texture_index) else {
            continue;
        };

        let adjust_extent = cell
            .size
            .map(|size| Vector2::new(size.x as f32, size.y as f32))
            .unwrap_or(texture_extent);

        // `zoom` (ACT < 2.4) and `zoom2` (ACT >= 2.4) are the same
        // scaleX/scaleY pair; the loaders keep whichever the version has.
        let zoom = cell.zoom.unwrap_or(1.0);
        let scale_x = zoom * cell.zoom2.map(|zoom| zoom.x).unwrap_or(1.0);
        let scale_y = zoom * cell.zoom2.map(|zoom| zoom.y).unwrap_or(1.0);
        let scale = record.scale * distance_scale;

        // The official loader stores the cell as a top-left offset; the raw
        // ACT position is its centre.
        let offset_x = adjusted_cell_position(cell.position.x, adjust_extent.x, scale_x);
        let offset_y = adjusted_cell_position(cell.position.y, adjust_extent.y, scale_y);

        let top = anchor_screen.y + offset_y * scale * scale_y;
        let bottom = top + texture_extent.y * scale * scale_y;
        let left = anchor_screen.x + offset_x * scale * scale_x;
        let right = left + texture_extent.x * scale * scale_x;

        // `RenderSegmentSprites2` drops a cell that is fully outside the
        // viewport before it rotates it.
        if top >= viewport.height || bottom <= 0.0 {
            continue;
        }

        if left >= viewport.width || right <= 0.0 {
            continue;
        }

        // `quad.angle = static_cast<float>(cell.rotation) + state.roll`, then
        // `DrawQuad` reads it through `ClientCosine/ClientSine(int)`: the sum
        // is truncated to whole degrees and rotated around the quad centre.
        let angle = (cell.angle.unwrap_or(0) as f32 + state.roll) as i32;
        let (sin, cos) = (angles.sine(angle), angles.cosine(angle));

        let half_width = (right - left) * 0.5;
        let half_height = (bottom - top) * 0.5;
        let center = Vector2::new(left + half_width, top + half_height);

        let local = [
            Vector2::new(-half_width, -half_height),
            Vector2::new(half_width, -half_height),
            Vector2::new(-half_width, half_height),
            Vector2::new(half_width, half_height),
        ];

        // `DrawQuad` adds 0.5 to every vertex (texel-centre alignment).
        let corners = local.map(|point| {
            Vector2::new(
                point.x * cos - point.y * sin + center.x + 0.5,
                point.y * cos + point.x * sin + center.y + 0.5,
            ) - anchor_screen
        });

        let texture_coordinates = if cell.mirror_on != 0 { mirrored_uvs() } else { plain_uvs() };

        let color = if (state.pattern & PATTERN_CELL_COLOR) != 0 {
            // `EffectPrim.cpp` initialises ACT < 2.0 cells to `0xFFFFFFFF`.
            cell_color(cell.color.unwrap_or(0xFFFF_FFFF))
        } else {
            record_color(record.color)
        };

        quads.push(CellQuad {
            texture_index,
            corners,
            texture_coordinates,
            color,
        });
    }

    quads
}

/// Draws one primitive that carries a sprite.
///
/// A primitive without `sprite`/`action`, without a loaded asset pair, without
/// live segment records or with a blend queue that cannot be derived draws
/// nothing.
pub(crate) fn render_primitive(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    sprite_loader: Option<&SpriteLoader>,
    action_loader: Option<&ActionLoader>,
    state: &EffectPrimitiveState,
) {
    let (Some(sprite_loader), Some(action_loader)) = (sprite_loader, action_loader) else {
        return;
    };

    let (Some(sprite_path), Some(action_path)) = (state.sprite, state.action) else {
        return;
    };

    // The loaders fall back to a placeholder for missing files; a placeholder
    // quad is worse than nothing.
    if !sprite_loader.file_exists(sprite_path) || !action_loader.file_exists(action_path) {
        return;
    }

    let Ok(sprite) = sprite_loader.get_or_load(sprite_path, None) else {
        return;
    };
    let Ok(actions) = action_loader.get_or_load(action_path) else {
        return;
    };

    if sprite.textures.is_empty() {
        return;
    }

    let Some(action) = actions.actions.get(action_row(state)) else {
        return;
    };

    let frame_count = state.action_frame_count.max(1) as usize;
    let frame = sprite_frame_index(state, frame_count);
    let Some(motion) = action.motions.get(frame).or_else(|| action.motions.first()) else {
        return;
    };

    let Some((source, destination)) = sprite_blend_factors(state.render_flag) else {
        return;
    };

    let window = renderer.window_size();
    let segments = state.num_segment.max(0) as usize;

    for segment in 0..segments {
        let Some(record) = state.records.get(segment) else {
            continue;
        };

        let anchor = Point3::new(record.position[0], record.position[1], record.position[2]);

        let Some((anchor_screen, _)) = project_point(camera, anchor, window) else {
            continue;
        };

        let Some(distance_scale) = distance_scale(camera, anchor, window) else {
            continue;
        };

        let quads = cell_quads(
            state,
            record,
            &motion.sprite_clips,
            window,
            anchor_screen,
            distance_scale,
            sprite.palette_size,
            |index| {
                sprite.textures.get(index).map(|texture| {
                    let size = texture.get_size();
                    Vector2::new(size.width as f32, size.height as f32)
                })
            },
        );

        for quad in quads {
            let Some(texture) = sprite.textures.get(quad.texture_index) else {
                continue;
            };

            renderer.render_effect(
                camera,
                anchor,
                texture.clone(),
                quad.corners.map(|corner| EFFECT_ORIGIN + corner),
                quad.texture_coordinates,
                Vector2::new(0.0, 0.0),
                // `DrawQuad` does its own rotation around the quad centre.
                Rad(0.0),
                quad.color,
                source,
                destination,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Deg, Matrix4, Vector3};
    use ragnarok_formats::action::{Action, Motion, SpriteClip};

    use super::*;
    use crate::world::effect::port::primitive::{MasterState, PrimitiveId};
    use crate::world::effect::port::runtime::EffectRng;

    const WINDOW: ScreenSize = ScreenSize {
        width: 640.0,
        height: 480.0,
    };

    /// A camera at `(0, 0, -10)` looking down `+z`, 45 degree vertical FOV on a
    /// 4:3 window. The matrices are built by the same helpers the game
    /// cameras use, so the projection conventions (left handed, y up,
    /// reverse-z) are identical.
    struct TestCamera {
        view: Matrix4<f32>,
        projection: Matrix4<f32>,
        view_projection: Matrix4<f32>,
    }

    impl TestCamera {
        fn new() -> Self {
            let view = Matrix4::look_to_lh(Point3::new(0.0, 0.0, -10.0), Vector3::unit_z(), Vector3::unit_y());
            let projection = crate::graphics::perspective_reverse_lh(Deg(45.0), 4.0 / 3.0);

            Self {
                view,
                projection,
                view_projection: projection * view,
            }
        }
    }

    impl Camera for TestCamera {
        fn camera_position(&self) -> Point3<f32> {
            Point3::new(0.0, 0.0, -10.0)
        }

        fn focus_point(&self) -> Point3<f32> {
            Point3::new(0.0, 0.0, 0.0)
        }

        fn generate_view_projection(&mut self, _window_size: ScreenSize) {}

        fn look_up_vector(&self) -> Vector3<f32> {
            Vector3::unit_y()
        }

        fn view_projection_matrices(&self) -> (Matrix4<f32>, Matrix4<f32>) {
            (self.view, self.projection)
        }

        fn view_projection_matrix(&self) -> Matrix4<f32> {
            self.view_projection
        }

        fn view_direction(&self) -> Vector3<f32> {
            Vector3::unit_z()
        }
    }

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState::default()
    }

    fn cell() -> SpriteClip {
        SpriteClip {
            position: Vector2::new(10, -20),
            sprite_number: 0,
            mirror_on: 0,
            color: Some(0xFFFF_FFFF),
            zoom: None,
            zoom2: Some(Vector2::new(2.0, 3.0)),
            angle: Some(0),
            sprite_type: Some(0),
            size: Some(Vector2::new(30, 40)),
        }
    }

    fn record(scale: f32) -> PrimitiveRecord {
        PrimitiveRecord {
            scale,
            color: 0x80_FF_80_40,
            ..Default::default()
        }
    }

    fn motion() -> Motion {
        Motion {
            range1: [0; 4],
            range2: [0; 4],
            sprite_clip_count: 0,
            sprite_clips: Vec::new(),
            event_id: None,
            attach_point_count: None,
            attach_points: Vec::new(),
        }
    }

    #[test]
    fn action_frame_count_reads_the_number_of_motions() {
        let actions = [
            Action {
                motion_count: 3,
                motions: vec![motion(), motion(), motion()],
            },
            Action {
                motion_count: 1,
                motions: vec![motion()],
            },
            Action {
                motion_count: 0,
                motions: Vec::new(),
            },
        ];

        assert_eq!(action_frame_count(&actions, 0), Some(3));
        assert_eq!(action_frame_count(&actions, 1), Some(1));
        // `CActRes::FrameCount` never returns less than one.
        assert_eq!(action_frame_count(&actions, 2), Some(1));
        assert_eq!(action_frame_count(&actions, 3), None);
    }

    #[test]
    fn unset_action_rows_render_row_zero() {
        let mut state = state();
        state.action_index = -1;
        assert_eq!(action_row(&state), 0);

        state.action_index = 0;
        assert_eq!(action_row(&state), 0);

        state.action_index = 3;
        assert_eq!(action_row(&state), 3);
    }

    #[test]
    fn frame_selection_wraps_at_the_action_frame_count() {
        let mut state = state();

        // A primitive that has not been stepped yet shows the first frame.
        assert_eq!(sprite_frame_index(&state, 3), 0);

        // animSpeed 1, three frames: 0,1,2,0,1,...
        state.anim_speed = 1;
        state.action_frame_count = 3;
        state.repeat_anim = 1;

        let frames: Vec<usize> = (1..=6)
            .map(|state_cnt| {
                state.state_cnt = state_cnt;
                sprite_frame_index(&state, 3)
            })
            .collect();
        assert_eq!(frames, vec![1, 2, 0, 1, 2, 0]);

        // animSpeed 2, four frames: every frame index is shown for two ticks.
        state.anim_speed = 2;
        state.action_frame_count = 4;
        let frames: Vec<usize> = (1..=9)
            .map(|state_cnt| {
                state.state_cnt = state_cnt;
                sprite_frame_index(&state, 4)
            })
            .collect();
        assert_eq!(frames, vec![1, 1, 2, 2, 3, 3, 0, 0, 1]);
    }

    #[test]
    fn frame_selection_stops_on_the_last_frame_without_repeat() {
        let mut state = state();
        state.anim_speed = 2;
        state.action_frame_count = 4;
        state.repeat_anim = 0;

        let frames: Vec<usize> = (1..=9)
            .map(|state_cnt| {
                state.state_cnt = state_cnt;
                sprite_frame_index(&state, 4)
            })
            .collect();

        assert_eq!(frames, vec![1, 1, 2, 2, 3, 3, 3, 3, 3]);
    }

    #[test]
    fn frame_selection_guards_a_zero_anim_speed() {
        let mut state = state();
        state.state_cnt = 3;
        state.anim_speed = 0;
        state.action_frame_count = 3;
        state.repeat_anim = 1;

        // The port's `AnimMotion` treats a non-positive speed as 1.
        assert_eq!(sprite_frame_index(&state, 3), 0);
    }

    #[test]
    fn frame_selection_matches_the_simulated_cur_motion() {
        use crate::world::effect::port::primitive::particle3d;

        let angles = Angles::new();
        let master = MasterState::default();

        for anim_speed in [1, 2, 3, 4, 6] {
            let mut rng = EffectRng::new(1);
            let mut state = EffectPrimitiveState {
                count: PrimitiveId::Particle3d,
                duration: 120,
                pattern: 1,
                anim_speed,
                action_frame_count: 5,
                repeat_anim: 1,
                fade_out_cnt: 120,
                ..Default::default()
            };

            for _ in 0..30 {
                assert!(particle3d::on_process(&mut state, &angles, &master, &mut rng));
                // `EffectPrimitiveState::on_process` increments the counter
                // after the primitive function, which is the state the renderer
                // sees.
                state.state_cnt += 1;
                assert_eq!(
                    sprite_frame_index(&state, 5),
                    state.cur_motion as usize,
                    "animSpeed {anim_speed} at stateCnt {}",
                    state.state_cnt
                );
            }
        }
    }

    #[test]
    fn cell_quads_apply_the_official_cell_scaling_and_offsets() {
        let mut state = state();
        state.pattern = 0;
        state.roll = 0.0;

        let quads = cell_quads(
            &state,
            &record(0.5),
            &[cell()],
            WINDOW,
            Vector2::new(100.0, 200.0),
            2.0,
            0,
            |_| Some(Vector2::new(30.0, 40.0)),
        );

        assert_eq!(quads.len(), 1);
        // `AdjustCellPosition` first turns the raw centres (10, -20) into the
        // top-left offsets: x = (10 - 15*2) / 2 = -10, y = trunc((-20 - 20*3)
        // / 3) = -26. Then scaleX/scaleY/record.scale/distanceScale are
        // applied: the quad is 30*2*0.5*2 = 60 wide and 40*3*0.5*2 = 120 high,
        // centred on (10, -18) because of the integer truncation in y.
        // The 0.5 texel-centre offset is `DrawQuad`'s.
        assert_eq!(quads[0].corners, [
            Vector2::new(-19.5, -77.5),
            Vector2::new(40.5, -77.5),
            Vector2::new(-19.5, 42.5),
            Vector2::new(40.5, 42.5),
        ]);
    }

    #[test]
    fn adjust_cell_position_turns_the_raw_centre_into_the_top_left() {
        // Even extents: the raw position is the quad centre.
        assert_eq!(adjusted_cell_position(0, 20.0, 1.0), -10.0);
        assert_eq!(adjusted_cell_position(10, 20.0, 1.0), 0.0);
        assert_eq!(adjusted_cell_position(-10, 20.0, 1.0), -20.0);

        // Odd extents use `extent / 2 + extent % 2` (the image is one pixel
        // wider to the left of its centre).
        assert_eq!(adjusted_cell_position(0, 29.0, 1.0), -15.0);
        assert_eq!(adjusted_cell_position(0, 1.0, 1.0), -1.0);

        // A scaled cell: `RelativeCoordinate` truncates toward zero.
        // (39 - 29/2*2 - 2.5) / 2.5 = (39 - 37.5) / 2.5 = 0.6 -> 0.
        assert_eq!(adjusted_cell_position(39, 29.0, 2.5), 0.0);
        assert_eq!(adjusted_cell_position(0, 29.0, 2.5), -15.0);

        // `scale == 0` is a division by zero in the original; the render
        // multiplies the stored value by zero, so the net offset is zero.
        assert_eq!(adjusted_cell_position(5, 20.0, 0.0), 0.0);
    }

    #[test]
    fn cell_quads_use_the_act_size_and_fall_back_to_the_texture_size() {
        let mut cell = cell();
        cell.zoom2 = Some(Vector2::new(1.0, 1.0));
        // The raw ACT centre; with an unscaled cell the adjusted top-left is
        // (0, 0).
        cell.position = Vector2::new(15, 20);

        let anchor = Vector2::new(100.0, 100.0);

        // ACT >= 2.5 carries the image size, which `AdjustCellPosition` uses.
        // The drawn quad always uses the sprite image size, passed by the
        // loader (here deliberately different from the ACT size).
        let quads = cell_quads(&state(), &record(1.0), &[cell.clone()], WINDOW, anchor, 1.0, 0, |_| {
            Some(Vector2::new(40.0, 50.0))
        });
        assert_eq!(quads.len(), 1);
        // The corners are anchor-relative, so the anchor cancels out. The
        // adjusted top-left is (0, 0) because the ACT extent (30, 40) matches
        // the raw centre; the quad is (40, 50).
        assert_eq!(quads[0].corners[3], Vector2::new(40.0 + 0.5, 50.0 + 0.5));

        // Older ACTs do not; the loaded texture size is the fallback.
        cell.size = None;
        let mut state = state();
        state.pattern = 0;
        let quads = cell_quads(&state, &record(1.0), &[cell], WINDOW, anchor, 1.0, 0, |index| {
            assert_eq!(index, 0);
            Some(Vector2::new(12.0, 18.0))
        });
        assert_eq!(quads.len(), 1);
        // The fallback extent (12, 18) changes the adjusted top-left to
        // (15 - 6, 20 - 9) = (9, 11).
        assert_eq!(quads[0].corners[3], Vector2::new(9.0 + 12.0 + 0.5, 11.0 + 18.0 + 0.5));
    }

    #[test]
    fn cell_quads_rotate_around_the_cell_centre() {
        let mut state = state();
        state.roll = 90.0;

        let mut cell = cell();
        // Remove the offset/scales so only the rotation remains.
        cell.position = Vector2::new(0, 0);
        cell.zoom2 = Some(Vector2::new(1.0, 1.0));
        cell.size = Some(Vector2::new(20, 10));

        let quads = cell_quads(&state, &record(1.0), &[cell], WINDOW, Vector2::new(0.0, 0.0), 1.0, 0, |_| {
            Some(Vector2::new(20.0, 10.0))
        });
        assert_eq!(quads.len(), 1);

        // Same table the renderer uses, so the expectation is exact. The raw
        // centre (0, 0) becomes the top-left (-10, -5), so the cell occupies
        // (-10,-5)-(10,5) and rotates around (0, 0).
        let angles = Angles::shared();
        let (sin, cos) = (angles.sine(90), angles.cosine(90));
        let expected = [
            Vector2::new(-10.0 * cos - -5.0 * sin + 0.5, -5.0 * cos + -10.0 * sin + 0.5),
            Vector2::new(10.0 * cos - -5.0 * sin + 0.5, -5.0 * cos + 10.0 * sin + 0.5),
            Vector2::new(-10.0 * cos - 5.0 * sin + 0.5, 5.0 * cos + -10.0 * sin + 0.5),
            Vector2::new(10.0 * cos - 5.0 * sin + 0.5, 5.0 * cos + 10.0 * sin + 0.5),
        ];
        assert_eq!(quads[0].corners, expected);
    }

    #[test]
    fn mirrored_cells_flip_the_texture_coordinates() {
        let mut cell = cell();
        cell.mirror_on = 1;

        let quads = cell_quads(
            &state(),
            &record(1.0),
            &[cell],
            WINDOW,
            Vector2::new(100.0, 100.0),
            1.0,
            0,
            |_| Some(Vector2::new(30.0, 40.0)),
        );

        assert_eq!(quads[0].texture_coordinates, [
            Vector2::new(0.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
        ]);
    }

    #[test]
    fn cells_entirely_outside_the_viewport_are_culled() {
        let state = state();
        let record = record(1.0);

        // Far above the screen.
        let quads = cell_quads(&state, &record, &[cell()], WINDOW, Vector2::new(100.0, -1000.0), 1.0, 0, |_| {
            Some(Vector2::new(30.0, 40.0))
        });
        assert!(quads.is_empty());

        // Far below the screen.
        let quads = cell_quads(&state, &record, &[cell()], WINDOW, Vector2::new(100.0, 2000.0), 1.0, 0, |_| {
            Some(Vector2::new(30.0, 40.0))
        });
        assert!(quads.is_empty());

        // Far left of the screen.
        let quads = cell_quads(&state, &record, &[cell()], WINDOW, Vector2::new(-1000.0, 100.0), 1.0, 0, |_| {
            Some(Vector2::new(30.0, 40.0))
        });
        assert!(quads.is_empty());
    }

    #[test]
    fn transparent_cells_are_skipped() {
        let mut cell = cell();
        cell.sprite_number = -1;

        let quads = cell_quads(
            &state(),
            &record(1.0),
            &[cell],
            WINDOW,
            Vector2::new(100.0, 100.0),
            1.0,
            0,
            |_| None,
        );

        assert!(quads.is_empty());
    }

    #[test]
    fn palette_and_rgba_images_use_the_flat_texture_index() {
        let mut cell = cell();

        cell.sprite_type = Some(0);
        cell.sprite_number = 5;
        assert_eq!(cell_texture_index(&cell, 100), Some(5));

        cell.sprite_type = Some(1);
        cell.sprite_number = 5;
        assert_eq!(cell_texture_index(&cell, 100), Some(105));

        // `CSprRes::images[2]` only; the original would index out of bounds.
        cell.sprite_type = Some(2);
        assert_eq!(cell_texture_index(&cell, 100), None);

        cell.sprite_type = Some(0);
        cell.sprite_number = -1;
        assert_eq!(cell_texture_index(&cell, 100), None);
    }

    #[test]
    fn cell_colours_are_used_only_with_pattern_200() {
        let mut state = state();
        let mut cell = cell();
        cell.color = Some(0x80_FF_80_40);

        // `record.color` (A,R,G,B) without the pattern bit.
        let quads = cell_quads(
            &state,
            &record(1.0),
            &[cell.clone()],
            WINDOW,
            Vector2::new(100.0, 100.0),
            1.0,
            0,
            |_| Some(Vector2::new(30.0, 40.0)),
        );
        assert_eq!(quads[0].color, Color::rgba_u8(0xFF, 0x80, 0x40, 0x80));

        // The ACT stores the cell colour as R,G,B,A bytes.
        state.pattern = PATTERN_CELL_COLOR;
        let quads = cell_quads(
            &state,
            &record(1.0),
            &[cell],
            WINDOW,
            Vector2::new(100.0, 100.0),
            1.0,
            0,
            |_| Some(Vector2::new(30.0, 40.0)),
        );
        assert_eq!(quads[0].color, Color::rgba_u8(0x40, 0x80, 0xFF, 0x80));
    }

    #[test]
    fn sprite_blend_flags_follow_the_render_queues() {
        assert_eq!(
            sprite_blend_factors(1),
            Some((BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha))
        );
        assert_eq!(sprite_blend_factors(3), Some((BlendFactor::One, BlendFactor::One)));
        assert_eq!(sprite_blend_factors(2), Some((BlendFactor::One, BlendFactor::One)));
        // `RSF_CUSTOM_BLEND` is the additive queue_01EC.
        assert_eq!(sprite_blend_factors(5), Some((BlendFactor::One, BlendFactor::One)));
        // `RSF_NO_DEPTH` only changes the queue.
        assert_eq!(
            sprite_blend_factors(0x201),
            Some((BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha))
        );
        // `RSF_TOPMOST` has no derivable blend state; it is not drawn.
        assert_eq!(sprite_blend_factors(0x9), None);
    }

    #[test]
    fn record_projection_is_monotonic_and_depth_ordered() {
        let camera = TestCamera::new();

        let (near_screen, near_depth) = project_point(&camera, Point3::new(0.0, 0.0, 0.0), WINDOW).unwrap();
        let (far_screen, far_depth) = project_point(&camera, Point3::new(0.0, 0.0, 10.0), WINDOW).unwrap();
        let (right_screen, _) = project_point(&camera, Point3::new(10.0, 0.0, 0.0), WINDOW).unwrap();

        // Dead centre on both axes; +x moves right on screen.
        assert_eq!(near_screen, Vector2::new(320.0, 240.0));
        assert_eq!(far_screen, Vector2::new(320.0, 240.0));
        assert!(right_screen.x > near_screen.x);

        // Reverse-z: the closer point has the larger depth.
        assert!(near_depth > far_depth);

        // The world-to-screen scale falls with depth.
        let near_scale = distance_scale(&camera, Point3::new(0.0, 0.0, 0.0), WINDOW).unwrap();
        let far_scale = distance_scale(&camera, Point3::new(0.0, 0.0, 10.0), WINDOW).unwrap();
        assert!(near_scale > far_scale);

        // Half the depth -> twice the scale (exact for this camera).
        assert!((near_scale / far_scale - 2.0).abs() < 1e-5);
    }

    #[test]
    fn a_body_without_a_sprite_draws_nothing() {
        let camera = TestCamera::new();
        let mut renderer = EffectRenderer::new(WINDOW);
        let state = state();

        assert!(state.sprite.is_none());
        render_primitive(&mut renderer, &camera, None, None, &state);

        assert!(renderer.get_instructions().is_empty());
    }

    #[test]
    fn a_sprite_without_assets_draws_nothing() {
        let camera = TestCamera::new();
        let mut renderer = EffectRenderer::new(WINDOW);
        let mut state = state();
        state.sprite = Some("이팩트\\particle1.spr");
        state.action = Some("이팩트\\particle1.act");
        state.num_segment = 1;
        state.init_segment();

        // No sprite/action loaders (the unit-test configuration).
        render_primitive(&mut renderer, &camera, None, None, &state);

        assert!(renderer.get_instructions().is_empty());
    }
}
