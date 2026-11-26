//! `PortedEffectHolder` - owns one ported effect body and steps it on the fixed
//! 16 ms clock of the official client.
//!
//! This is the object the client will keep in its effect list once the ported
//! backend is wired (plan 4.2). It deliberately separates the *simulation*
//! (which is exact and testable) from the *rendering* (which needs textures and
//! the effect renderer): `update` only advances the timeline, so the timing
//! model can be verified without a GPU.

use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2};
use ragnarok_packets::EffectId;
use wgpu::BlendFactor;

use super::primitive::{EffectPrimitiveState, PrimitiveId};
use super::runtime::{EffectRuntime, PortedEffect};
use super::str_player::Clip;
use super::str_table::{self, StrResource};
use super::{FRAME_FREQUENCY_MS, registry, render_slots, render3d, sprite_pass};
use crate::graphics::Color;
use crate::loaders::{ActionLoader, GameFileLoader, ImageType, SpriteLoader, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

pub struct PortedEffectHolder {
    body: Box<dyn PortedEffect>,
    runtime: EffectRuntime,
    /// Milliseconds not yet consumed by a 16 ms step.
    accumulator_ms: f64,
    finished: bool,
    /// `CRagEffect::Init`'s `.str` resource row of this effect id, if any;
    /// `None` for every effect without a screen-sprite clip. The resource
    /// itself is opened before the first step by [`Self::with_str_clips`] or
    /// [`Self::with_game_file_loader`].
    str_resource: Option<&'static StrResource>,
    /// `.spr`/`.act` loaders of the client for the sprite primitives. `None`
    /// in unit tests, which have no game archives, and for effects without
    /// sprite bodies; the sprite pass then draws nothing.
    sprite_loader: Option<Arc<SpriteLoader>>,
    action_loader: Option<Arc<ActionLoader>>,
    /// `(sprite path, action path, action row) -> frame count`, filled by the
    /// asset pass in [`Self::attach_sprite_actions`]. `0` means the pair could
    /// not be loaded and the primitive stays undrawn.
    sprite_frame_counts: HashMap<(&'static str, &'static str, i32), u32>,
}

impl PortedEffectHolder {
    /// Creates the holder for a registered effect id.
    pub fn new(effect: EffectId, seed: u64) -> Option<Self> {
        let body = registry::spawn(effect)?;
        let lifetime = body.lifetime();
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], seed);
        let _ = &mut runtime;

        Some(Self {
            body,
            runtime,
            accumulator_ms: 0.0,
            finished: false,
            str_resource: str_table::str_resource(effect),
            sprite_loader: None,
            action_loader: None,
            sprite_frame_counts: HashMap::new(),
        })
    }

    /// Official `.str` files the effect opens at spawn (`CRagEffect::Init`),
    /// if any. Names are relative to `data\texture\effect\`.
    pub fn str_files(&self) -> Option<&'static [&'static str]> {
        self.str_resource.map(|resource| resource.files)
    }

    /// Opens the effect's `.str` resource from already-parsed clips.
    ///
    /// The asset layer resolves [`Self::str_files`] through
    /// [`str_table::load_clips`] (which reads the same `EffectData` layout as
    /// the client's `EffectLoader`) and attaches the layers here, before the
    /// first 16 ms step. Effects without a `.str` resource ignore the call, so
    /// their simulation is untouched.
    pub fn with_str_clips(mut self, clips: Vec<Clip>) -> Self {
        if let Some(resource) = self.str_resource {
            self.open_str(resource, clips);
        }

        self
    }

    /// Loads and opens the effect's official `.str` resource through the
    /// client's game-file loader.
    ///
    /// A missing or malformed resource must not take the effect down: the body
    /// keeps running its non-`.str` primitives without a screen-sprite layer.
    /// With a resource attached, `ProcessEZ2STR` owns the ending exactly as in
    /// the original (`Signum::on_process` forwards its `false`).
    pub fn with_game_file_loader(mut self, game_file_loader: &GameFileLoader) -> Self {
        let Some(resource) = self.str_resource else {
            return self;
        };

        match str_table::load_clips(game_file_loader, resource) {
            Some(clips) => self.open_str(resource, clips),
            None => crate::logging::log_debug!("ported effect: missing .str resource {:?}", resource.files),
        }

        self
    }

    fn open_str(&mut self, resource: &StrResource, clips: Vec<Clip>) {
        if clips.is_empty() {
            return;
        }

        self.runtime.open_str(clips);

        if let Some(player) = self.runtime.str_player.as_mut() {
            player.loop_limit = resource.loop_limit;
        }
    }

    /// Attaches the client's sprite/action loaders, so the holder can fill the
    /// ACT frame count of sprite primitives and draw them. The effect bodies
    /// keep their own logic; only the asset-side frame count is supplied here.
    pub fn with_sprite_loaders(mut self, sprite_loader: Arc<SpriteLoader>, action_loader: Arc<ActionLoader>) -> Self {
        self.sprite_loader = Some(sprite_loader);
        self.action_loader = Some(action_loader);
        self
    }

    pub fn runtime(&self) -> &EffectRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut EffectRuntime {
        &mut self.runtime
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Texture assets the effect references; the loader prefetches these.
    ///
    /// Every slot of a primitive's texture array is listed, not only the
    /// primary one: animated sequences (`texture_frames`) and two/three
    /// texture primitives (`texture_secondary`, `texture_tertiary`) must be
    /// resident when the motion advances `curMotion`.
    pub fn textures(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.runtime.primitives.iter().flat_map(|primitive| {
            let state = &primitive.state;

            state
                .texture
                .into_iter()
                .chain(state.texture_secondary)
                .chain(state.texture_tertiary)
                .chain(state.texture_frames.iter().copied())
        })
    }

    /// Advances the effect by `delta_seconds` of wall time.
    ///
    /// The official client runs effects on a **fixed 16 ms** step, so a frame
    /// of 50 ms produces three steps and a 5 ms frame produces none.
    /// Returns `false` once the effect has finished (and keeps returning
    /// `false`).
    pub fn update(&mut self, delta_seconds: f64) -> bool {
        if self.finished {
            return false;
        }

        self.accumulator_ms += delta_seconds * 1000.0;

        while self.accumulator_ms >= FRAME_FREQUENCY_MS as f64 {
            self.accumulator_ms -= FRAME_FREQUENCY_MS as f64;

            if !self.step() {
                self.finished = true;
                return false;
            }
        }

        true
    }

    /// One fixed step, in the official order.
    ///
    /// This mirrors [`super::runtime::step`] operand for operand - body
    /// `OnProcess`, `++stateCnt`, the lifetime check, then the primitive pass -
    /// and additionally resolves the ACT frame count of newly launched sprite
    /// primitives between the body and the primitive pass. That is where the
    /// original loads the resource: `SetSpriteName`/`SetActionName` run inside
    /// the body's `LaunchEffectPrim` block, so the same frame's `AnimMotion`
    /// already sees the real `FrameCount`. Doing it after the primitive pass
    /// would be one frame too late for the `repeatAnim == 0` bodies, which
    /// retire as soon as `curMotion` reaches `FrameCount`.
    fn step(&mut self) -> bool {
        if !self.body.on_process(&mut self.runtime) {
            return false;
        }

        self.runtime.frame += 1;

        if self.runtime.frame >= self.runtime.lifetime {
            return false;
        }

        self.attach_sprite_actions();
        self.runtime.process_primitives();

        true
    }

    /// Asset-layer counterpart of `SetActionName`: resolves the `.act` row of
    /// every live sprite primitive and stores its motion count in
    /// [`EffectPrimitiveState::action_frame_count`], which the ported
    /// `AnimMotion` uses as `CActRes::FrameCount`.
    ///
    /// This is not simulation logic: the count is a file input the original
    /// reads synchronously, and the port's bodies cannot know it. It also
    /// prefetches the `.spr` so the first render frame does not hitch on disk.
    fn attach_sprite_actions(&mut self) {
        let Self {
            runtime,
            sprite_loader,
            action_loader,
            sprite_frame_counts,
            ..
        } = self;

        let (Some(sprite_loader), Some(action_loader)) = (sprite_loader.as_deref(), action_loader.as_deref()) else {
            return;
        };

        for primitive in &mut runtime.primitives {
            if !primitive.alive {
                continue;
            }

            let state = &mut primitive.state;

            let (Some(sprite_path), Some(action_path)) = (state.sprite, state.action) else {
                continue;
            };

            let row = sprite_pass::action_row(state);
            let key = (sprite_path, action_path, row as i32);

            let frame_count = match sprite_frame_counts.get(&key) {
                Some(&frame_count) => frame_count,
                None => {
                    let frame_count = if !sprite_loader.file_exists(sprite_path) || !action_loader.file_exists(action_path) {
                        0
                    } else {
                        let _ = sprite_loader.get_or_load(sprite_path, None);

                        action_loader
                            .get_or_load(action_path)
                            .ok()
                            .and_then(|actions| sprite_pass::action_frame_count(&actions.actions, row))
                            .unwrap_or(0)
                    };

                    sprite_frame_counts.insert(key, frame_count);
                    frame_count
                }
            };

            if frame_count > 0 {
                state.action_frame_count = frame_count;
            }
        }
    }

    /// Submits every primitive the port can draw today.
    ///
    /// * `PP_2DTEXTURE`/`PP_2DFLASH` - screen-space quads
    ///   (`Render2DPrimitive`).
    /// * `PP_3DTEXTURE`/`PP_3DCROSSTEXTURE` - world-space quads
    ///   (`DrawEffectPrimitive15`/`RenderCrossPlanes`), see [`render3d`].
    /// * every primitive that carries a `.spr`/`.act` pair - the sprite
    ///   particles and the `TexturedClassChange` bodies
    ///   (`RenderSegmentSprites2`), see [`sprite_pass`].
    /// * the slot/record primitives - rings, cylinders, columns, fans, halo,
    ///   wind and arc walls (`Render3DCasting`, `RenderRecordRings`,
    ///   `RenderArcBand`, `RenderFieldPanel`, `RenderPyramid`, ...), see
    ///   [`render_slots`]. They are submitted as triangles because their
    ///   vertices carry independent heights and depths; the forward effect pass
    ///   draws them right after the quad list. This includes `TexturedAura`'s
    ///   prim 82 (`RenderTeiFacingSlots`).
    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, textures: &TextureLoader) {
        let anchor = Point3::new(self.runtime.position[0], self.runtime.position[1], self.runtime.position[2]);

        for primitive in &self.runtime.primitives {
            if !primitive.alive {
                continue;
            }

            let state = &primitive.state;

            // `CEffectPrim::Render`'s `switch (state.count)` for the
            // slot/record primitives - `Render3DCasting`, `RenderRecordRings`,
            // `RenderArcBand`, `RenderFieldPanel`, `RenderPyramid`,
            // `RenderArcWalls`, `RenderFacingQuads`, `RenderRipple`,
            // `RenderSizedSquare`, `RenderRiseArc`, `RenderGroundSquare` and
            // `RenderTwinGlowSquares`.
            if render_slots::renderer(state).is_some() {
                render_slot_primitive(renderer, camera, textures, state);

                continue;
            }

            // `RenderSegmentSprites2`: a primitive that owns a sprite is drawn
            // per ACT cell, whatever its `count` is.
            if state.sprite.is_some() {
                sprite_pass::render_primitive(
                    renderer,
                    camera,
                    self.sprite_loader.as_deref(),
                    self.action_loader.as_deref(),
                    state,
                );

                continue;
            }

            match state.count {
                PrimitiveId::Texture2d | PrimitiveId::Flash2d => {
                    let Some(path) = state.texture else {
                        continue;
                    };

                    let Ok(texture) = textures.get_or_load(path, ImageType::Color) else {
                        continue;
                    };

                    let (corners, uvs) = quad_corners(state, self.runtime.screen);
                    let (source, destination) = blend_factors(state.render_flag);

                    renderer.render_effect(
                        camera,
                        anchor,
                        texture,
                        corners,
                        uvs,
                        Vector2::new(0.0, 0.0),
                        Rad(0.0),
                        primitive_color(state),
                        source,
                        destination,
                    );
                }
                PrimitiveId::Texture3d | PrimitiveId::CrossTexture3d => {
                    render3d::render_textured_primitive(renderer, camera, textures, state);
                }
                // Sprite primitives (`Particle3d`, `Particle3dGravity`,
                // `Orbit` and the `.spr` bodies) were already dispatched above
                // by their `sprite` field, and the slot/record primitives by
                // `render_slots::renderer`. The remaining ids have no ported
                // renderer.
                _ => {}
            }
        }
    }

    /// Number of 16 ms steps still pending in the accumulator (test hook).
    pub fn pending_steps(&self) -> u32 {
        (self.accumulator_ms / FRAME_FREQUENCY_MS as f64) as u32
    }
}

/// The four screen-space corners and UVs of a `Render2DPrimitive` quad,
/// relative to the projected anchor (`master.screen`).
pub(crate) fn quad_corners(state: &EffectPrimitiveState, anchor_screen: [f32; 2]) -> ([Vector2<f32>; 4], [Vector2<f32>; 4]) {
    let roll = state.roll.to_radians();
    let (sin, cos) = roll.sin_cos();

    let width = state.width_size;
    let height = state.height_size;

    let center = Vector2::new(state.position[0] - anchor_screen[0], state.position[1] - anchor_screen[1]);

    // Clockwise from the top-left corner: (-w,-h), (w,-h), (w,h), (-w,h).
    let local = [
        Vector2::new(-width, -height),
        Vector2::new(width, -height),
        Vector2::new(width, height),
        Vector2::new(-width, height),
    ];

    let corners = local.map(|point| {
        Vector2::new(
            point.x * cos - point.y * sin + center.x,
            point.x * sin + point.y * cos + center.y,
        )
    });

    let uvs = [
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(0.0, 1.0),
    ];

    (corners, uvs)
}

/// `argb` as stored by `ProcessAlpha` -> renderer colour.
pub(crate) fn primitive_color(state: &EffectPrimitiveState) -> Color {
    color_from_argb(state.argb)
}

/// Decodes the packed `argb` of a primitive/slot triangle.
///
/// The official emitters pack the alpha into the high byte and the three
/// colour bytes below it (`(((alphaB << 8) + r) << 8) + g) << 8) + b`), which
/// is exactly `EffectPrimitiveState::argb`'s layout after `ProcessAlpha`.
pub(crate) fn color_from_argb(argb: u32) -> Color {
    let alpha = ((argb >> 24) & 0xFF) as f32 / 255.0;
    let red = ((argb >> 16) & 0xFF) as f32 / 255.0;
    let green = ((argb >> 8) & 0xFF) as f32 / 255.0;
    let blue = (argb & 0xFF) as f32 / 255.0;

    Color::rgba(red, green, blue, alpha)
}

/// Submits one slot/record primitive as its official triangle list.
///
/// The slot renderers build the geometry in world space and the renderer
/// projects every corner on its own (`RenderTeiRect`/`RenderRecordRings` also
/// project per vertex, then reject a triangle whose `rhw` is not positive).
/// A triangle without a resolvable texture draws nothing, like the descriptor
/// effects; a missing asset must not take the whole effect down.
///
/// The context carries the three values `RenderTeiFacingSlots` (prim 82) reads
/// outside the primitive state: the camera (`extra == 3`'s `TransformVert` and
/// `FaceCamera`), `g_renderer->viewport`, and `this->render.distanceScale`.
/// The other renderers ignore it.
fn render_slot_primitive(renderer: &mut EffectRenderer, camera: &dyn Camera, textures: &TextureLoader, state: &EffectPrimitiveState) {
    let context = render_slots::SlotRenderContext {
        camera,
        viewport: renderer.window_size(),
        // `CRenderObject::Reset`'s 1.0; see `CEFFECT_PRIM_DISTANCE_SCALE`.
        distance_scale: render_slots::CEFFECT_PRIM_DISTANCE_SCALE,
        // `FaceCamera`, computed at render time because the simulation has no
        // camera. The rest of the module reads the same two angles.
        facing: render_slots::face_camera_angles(camera),
    };

    // One primitive usually draws a few hundred triangles that all sample the
    // same one of its (at most three) texture slots, so the loader is only
    // asked again when the path changes.
    let mut cached: Option<(&'static str, Option<Arc<crate::graphics::Texture>>)> = None;

    for triangle in render_slots::triangles_with_context(state, Some(&context)) {
        let Some(path) = triangle.texture else {
            continue;
        };

        let texture = match &cached {
            Some((cached_path, texture)) if *cached_path == path => texture.clone(),
            _ => {
                let texture = textures.get_or_load(path, ImageType::Color).ok();
                cached = Some((path, texture.clone()));

                texture
            }
        };

        let Some(texture) = texture else {
            continue;
        };

        let (source_blend_factor, destination_blend_factor) = triangle.blend.factors();
        let corners = render_slots::triangle_points(&triangle);
        let texture_coordinates = render_slots::triangle_uvs(&triangle);
        let color = color_from_argb(triangle.argb);

        if let Some(projected) = &triangle.projected {
            // `RenderTeiRect`'s `extra == 3`: the corners are already NDC.
            renderer.render_triangle_projected(
                projected.corners,
                projected.depth,
                texture_coordinates,
                color,
                source_blend_factor,
                destination_blend_factor,
                texture,
            );

            continue;
        }

        match triangle.forced_depth {
            Some(depth) => renderer.render_triangle_with_depth(
                camera,
                corners,
                texture_coordinates,
                [depth; 3],
                color,
                source_blend_factor,
                destination_blend_factor,
                texture,
            ),
            None => renderer.render_triangle(
                camera,
                corners,
                texture_coordinates,
                color,
                source_blend_factor,
                destination_blend_factor,
                texture,
            ),
        }
    }
}

/// `RSF_ADDITIVE` (0x2) or the `0x200` shorthand the effect bodies use.
fn blend_factors(render_flag: u32) -> (BlendFactor, BlendFactor) {
    if (render_flag & 0x200) != 0 {
        (BlendFactor::One, BlendFactor::One)
    } else {
        (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha)
    }
}

#[cfg(test)]
mod tests {
    use super::super::str_player::{KeyFrame, Transform};
    use super::*;

    fn clip(keys: &[(i32, f32)]) -> Vec<Clip> {
        vec![Clip {
            texture_count: 1,
            frames: keys
                .iter()
                .map(|&(frame, x)| KeyFrame {
                    frame,
                    transform: Transform {
                        x,
                        kind: 0,
                        ..Default::default()
                    },
                })
                .collect(),
        }]
    }

    #[test]
    fn bash_runs_exactly_one_hundred_sixty_milliseconds_per_ten_frames() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        // 10 frames at 60 Hz = 166.7 ms -> 10 steps of 16 ms.
        let mut steps = 0;
        for _ in 0..10 {
            holder.update(1.0 / 60.0);
            steps = holder.runtime().frame;
        }

        assert_eq!(steps, 10);
        assert_eq!(holder.runtime().primitives.len(), 21);
    }

    #[test]
    fn catch_up_produces_three_steps_for_a_fifty_ms_frame() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        holder.update(0.050);

        assert_eq!(holder.runtime().frame, 3);
    }

    #[test]
    fn five_ms_frames_do_not_step_until_sixteen_accumulate() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        holder.update(0.005);
        assert_eq!(holder.runtime().frame, 0);

        holder.update(0.005);
        holder.update(0.005);
        assert_eq!(holder.runtime().frame, 0);

        holder.update(0.005);
        assert_eq!(holder.runtime().frame, 1);
    }

    #[test]
    fn effect_finishes_after_its_lifetime() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        // 40 frames * 16 ms = 640 ms.
        let mut elapsed = 0.0;
        while holder.update(0.016) {
            elapsed += 0.016;
            if elapsed > 5.0 {
                panic!("effect did not finish");
            }
        }

        assert!(holder.is_finished());
        assert_eq!(holder.runtime().frame, 40);
        assert!(!holder.update(0.016), "stays finished");
    }

    #[test]
    fn textures_lists_the_assets_of_live_primitives() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();
        holder.update(0.016);

        let textures: Vec<_> = holder.textures().collect();

        assert!(textures.contains(&"effect\\alpha_down.tga"));
        assert!(textures.contains(&"EFFECT\\alpha_center.tga"));
    }

    #[test]
    fn textures_lists_secondary_and_animated_slots() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();
        holder.update(0.016);

        {
            let state = holder.runtime_mut().primitive_mut(0);
            state.texture = Some("effect\\primary.bmp");
            state.texture_secondary = Some("effect\\secondary.bmp");
            state.texture_tertiary = Some("effect\\tertiary.bmp");
            state.texture_frames = vec!["effect\\frame0.bmp", "effect\\frame1.bmp"];
        }

        let textures: Vec<_> = holder.textures().collect();

        assert!(textures.contains(&"effect\\primary.bmp"));
        assert!(textures.contains(&"effect\\secondary.bmp"));
        assert!(textures.contains(&"effect\\tertiary.bmp"));
        assert!(textures.contains(&"effect\\frame0.bmp"));
        assert!(textures.contains(&"effect\\frame1.bmp"));
    }

    #[test]
    fn two_d_quad_corners_keep_their_existing_geometry_and_uvs() {
        let state = EffectPrimitiveState {
            position: [10.0, 20.0, 0.0],
            width_size: 2.0,
            height_size: 3.0,
            roll: 0.0,
            ..Default::default()
        };

        let (corners, uvs) = quad_corners(&state, [4.0, 6.0]);

        assert_eq!(corners, [
            Vector2::new(4.0, 11.0),
            Vector2::new(8.0, 11.0),
            Vector2::new(8.0, 17.0),
            Vector2::new(4.0, 17.0),
        ]);
        assert_eq!(uvs, [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
        ]);
    }

    #[test]
    fn quad_corners_are_relative_to_the_anchor_and_rotated() {
        let state = EffectPrimitiveState {
            position: [320.0, 240.0, 0.0],
            width_size: 10.0,
            height_size: 20.0,
            roll: 0.0,
            ..Default::default()
        };

        let (corners, uvs) = quad_corners(&state, [300.0, 200.0]);

        assert_eq!(corners[0], Vector2::new(20.0 - 10.0, 40.0 - 20.0));
        assert_eq!(corners[2], Vector2::new(20.0 + 10.0, 40.0 + 20.0));
        assert_eq!(uvs[2], Vector2::new(1.0, 1.0));

        // A 90 degree roll swaps the axes.
        let mut rotated = state;
        rotated.roll = 90.0;
        let (rotated_corners, _) = quad_corners(&rotated, [300.0, 200.0]);
        // (-10,-20) rotated by 90 deg -> (+20,-10), plus the centre (20,40).
        assert!((rotated_corners[0].x - 40.0).abs() < 1e-4);
        assert!((rotated_corners[0].y - 30.0).abs() < 1e-4);
    }

    #[test]
    fn color_decodes_the_packed_argb() {
        let state = EffectPrimitiveState {
            argb: 0x80_FF_00_40,
            ..Default::default()
        };

        let color = primitive_color(&state);

        assert!((color.red - 1.0).abs() < 1e-6);
        assert!((color.green - 0.0).abs() < 1e-6);
        assert!((color.blue - 64.0 / 255.0).abs() < 1e-6);
        assert!((color.alpha - 128.0 / 255.0).abs() < 1e-6);
    }

    #[test]
    fn additive_flag_selects_one_one_blending() {
        assert_eq!(blend_factors(0x200), (BlendFactor::One, BlendFactor::One));
        assert_eq!(blend_factors(1), (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha));
    }

    #[test]
    fn unported_effect_has_no_holder() {
        assert!(PortedEffectHolder::new(EffectId::Thunderstorm, 1).is_none());
    }

    #[test]
    fn str_effects_name_their_official_resource_before_any_asset_is_loaded() {
        let signum = PortedEffectHolder::new(EffectId::Signum, 1).unwrap();
        assert_eq!(signum.str_files(), Some(&["Cross.str"][..]));
        assert!(signum.runtime().str_player.is_none(), "no clip before the asset layer attaches");
        assert!(signum.runtime().str_clips.is_empty());

        let cart = PortedEffectHolder::new(EffectId::Cartrevolution, 1).unwrap();
        assert_eq!(cart.str_files(), Some(&["CartRevolution.str"][..]));
        assert!(cart.runtime().str_player.is_none());

        let bash = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();
        assert_eq!(bash.str_files(), None);
    }

    #[test]
    fn supplied_clips_open_the_str_player_at_spawn() {
        let mut holder = PortedEffectHolder::new(EffectId::Signum, 1)
            .unwrap()
            .with_str_clips(clip(&[(0, 10.0), (3, 40.0)]));

        assert_eq!(holder.runtime().str_clips.len(), 1);
        assert_eq!(holder.runtime().str_clips[0].texture_count, 1);

        let player = holder.runtime().str_player.as_ref().expect("the clip was opened");
        assert_eq!(player.state_cnt, 0);
        assert_eq!(player.loop_limit, 0, "RagEffectState's default loopLimit");
        assert_eq!(player.current_key, vec![0]);

        // `InitEZ2STRFrame` only resets the cursor; the first key is applied by
        // the first `ProcessEZ2STR` call (frame 0 of the effect).
        assert!(holder.update(0.016));
        let player = holder.runtime().str_player.as_ref().unwrap();
        assert_eq!(player.transforms[0].x, 10.0);

        let cart = PortedEffectHolder::new(EffectId::Cartrevolution, 1)
            .unwrap()
            .with_str_clips(clip(&[(0, 0.0)]));
        assert!(cart.runtime().str_player.is_some());
    }

    #[test]
    fn the_opened_clip_advances_on_the_sixteen_millisecond_clock() {
        let mut holder = PortedEffectHolder::new(EffectId::Signum, 1)
            .unwrap()
            .with_str_clips(clip(&[(0, 10.0), (3, 40.0)]));

        // Key frames land on the same counter `ProcessEZ2STR` reads: the key
        // at frame 3 is drawn on the fourth 16 ms step.
        for _ in 0..4 {
            assert!(holder.update(0.016));
        }

        let player = holder.runtime().str_player.as_ref().expect("clip still open");
        assert_eq!(player.transforms[0].x, 40.0);
        assert_eq!(player.current_key[0], -1, "the single layer reached its last key");

        // The next step makes `ProcessEZ2STR` return false (loopLimit 0), and
        // `Signum::on_process` forwards that as the effect's retirement.
        assert!(!holder.update(0.016));
        assert!(holder.is_finished());
    }

    #[test]
    fn effects_without_a_str_resource_ignore_supplied_clips() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1)
            .unwrap()
            .with_str_clips(clip(&[(0, 1.0)]));

        assert!(holder.runtime().str_player.is_none());
        assert!(holder.runtime().str_clips.is_empty());

        assert!(holder.update(0.016));
        assert_eq!(holder.runtime().primitives.len(), 21, "Bash is unchanged");
    }

    #[test]
    fn a_missing_str_asset_leaves_the_effect_alive_without_its_clip() {
        let mut holder = PortedEffectHolder::new(EffectId::Signum, 1)
            .unwrap()
            .with_game_file_loader(&GameFileLoader::default());

        assert!(holder.runtime().str_player.is_none());
        assert!(holder.update(0.016));
        assert!(holder.runtime().primitives.is_empty(), "the ring lands on frame 30");
    }
}
