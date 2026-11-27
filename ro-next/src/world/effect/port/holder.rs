//! `PortedEffectHolder` - owns one ported effect body and steps it on the fixed
//! 16 ms clock of the official client.
//!
//! This is the object the client will keep in its effect list once the ported
//! backend is wired (plan 4.2). It deliberately separates the *simulation*
//! (which is exact and testable) from the *rendering* (which needs textures and
//! the effect renderer): `update` only advances the timeline, so the timing
//! model can be verified without a GPU.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cgmath::{Point3, Rad, Vector2};
use ragnarok_packets::EffectId;
use wgpu::BlendFactor;

use super::angles::Angles;
use super::primitive::{EffectPrimitiveState, PrimitiveId};
use super::runtime::{EffectRuntime, PendingEffectLaunch, PortedEffect};
use super::sound_sink::{self, SoundSink};
use super::str_player::Clip;
use super::str_table::{self, ClipSet, StrResource};
use super::{FRAME_FREQUENCY_MS, particle, registry, render_slots, render3d, sprite_pass, str_render};
use crate::graphics::{Color, ScreenSize};
use crate::loaders::{ActionLoader, GameFileLoader, ImageType, SpriteLoader, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EFFECT_ORIGIN, project_point};

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
    /// `EZeffectClip::names` per `.str` clip, already resolved to the client
    /// loader's `effect\...` paths (`str_table::ClipSet::textures`). Empty
    /// until a resource is opened; the playback player does not need it, only
    /// the draw pass does.
    str_textures: Vec<Vec<String>>,
    /// `.spr`/`.act` loaders of the client for the sprite primitives. `None`
    /// in unit tests, which have no game archives, and for effects without
    /// sprite bodies; the sprite pass then draws nothing.
    sprite_loader: Option<Arc<SpriteLoader>>,
    action_loader: Option<Arc<ActionLoader>>,
    /// `(sprite path, action path, action row) -> frame count`, filled by the
    /// asset pass in [`Self::attach_sprite_actions`]. `0` means the pair could
    /// not be loaded and the primitive stays undrawn.
    sprite_frame_counts: HashMap<(&'static str, &'static str, i32), u32>,
    /// `LaunchEffect` requests the renderer pass recorded since the last step
    /// (the runtime's [`EffectRuntime::pending_effect_launches`] outbox,
    /// drained on every 16 ms step). The visual turns them into child effects;
    /// see [`Self::take_effect_launches`].
    effect_launches: Vec<PendingEffectLaunch>,
    /// The projected anchor of the **next** step, written by
    /// [`Self::project_screen`] from the render pass.
    ///
    /// `EffectBase::render` is the only hook that receives the camera, and it
    /// runs with `&self`, so the projection is parked here (a `Mutex` because
    /// ported visuals must stay `Sync`) and copied into
    /// [`EffectRuntime::screen`] at the start of the next 16 ms step - the
    /// step whose `Move2DPrimitive` reads `master.screen`.
    projected_screen: Mutex<[f32; 2]>,
}

impl PortedEffectHolder {
    /// Creates the holder for a registered effect id.
    pub fn new(effect: EffectId, seed: u64) -> Option<Self> {
        let body = registry::spawn(effect)?;
        let mut holder = Self::with_body(body, seed);
        holder.str_resource = str_table::str_resource(effect);

        Some(holder)
    }

    /// Wraps an already-spawned body. Split from [`Self::new`] so the
    /// simulation tests can step a synthetic body that records sounds on both
    /// channels; production effects always come from the registry.
    fn with_body(body: Box<dyn PortedEffect>, seed: u64) -> Self {
        let lifetime = body.lifetime();

        Self {
            body,
            runtime: EffectRuntime::new(lifetime, [0.0, 0.0], seed),
            accumulator_ms: 0.0,
            finished: false,
            str_resource: None,
            str_textures: Vec::new(),
            sprite_loader: None,
            action_loader: None,
            sprite_frame_counts: HashMap::new(),
            effect_launches: Vec::new(),
            projected_screen: Mutex::new([0.0, 0.0]),
        }
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
    pub fn with_str_clips(self, clips: Vec<Clip>) -> Self {
        self.with_str_clip_set(ClipSet {
            clips,
            textures: Vec::new(),
        })
    }

    /// Opens the effect's `.str` resource from already-parsed clips and their
    /// per-clip texture lists.
    ///
    /// The asset layer resolves [`Self::str_files`] through
    /// [`str_table::load_clips`] and gets one of these; `with_str_clips` is the
    /// texture-less shorthand for tests.
    pub fn with_str_clip_set(mut self, clip_set: ClipSet) -> Self {
        if let Some(resource) = self.str_resource {
            self.open_str(resource, clip_set);
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
            Some(clip_set) => self.open_str(resource, clip_set),
            None => crate::logging::log_debug!("ported effect: missing .str resource {:?}", resource.files),
        }

        self
    }

    fn open_str(&mut self, resource: &StrResource, clip_set: ClipSet) {
        if clip_set.clips.is_empty() {
            return;
        }

        self.runtime.open_str(clip_set.clips);
        self.str_textures = clip_set.textures;

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

    /// `g_renderer->Project(&this->state.parent->render.position.x, view,
    /// vertex)` -> `state.tlvertX/tlvertY` (`RagEffect.cpp:681-684` and the
    /// 26 other bodies that project their anchor).
    ///
    /// The result is the effect anchor in the render target's pixel space with
    /// the official origin (top-left, y growing downward), matching the camera
    /// and window size [`EffectRenderer`] draws with. It is recomputed on every
    /// render pass and handed to the next 16 ms step, where
    /// `Move2DPrimitive` adds it to `deltaPos2` (`EffectPrimitiveMotion.cpp:
    /// 7777-7784`) and [`quad_corners`] subtracts it again to get the
    /// anchor-relative quad offset. Using one value for both ends is what
    /// keeps the quad exactly on the projected anchor: the official recomputes
    /// `tlvert` once per process frame with the same camera it later renders
    /// with.
    ///
    /// The official projects `state.parent->render.position`; the port's bodies
    /// copy that into `render.position` (`EffectRuntime::copy_parent_position`)
    /// or anchor at it directly, so `runtime.position` is the value to project.
    ///
    /// A position behind the camera has no projection (`project_point` returns
    /// `None`); the official `Project` would divide by a non-positive `rhw`
    /// and produce garbage that the renderer clips away, so the port parks
    /// `[0.0, 0.0]` and lets the quad keep its anchor-relative offset.
    pub fn project_screen(&self, camera: &dyn Camera, window_size: ScreenSize) {
        let [x, y, z] = self.runtime.position;

        let screen = project_point(camera, Point3::new(x, y, z), window_size)
            .map_or([0.0, 0.0], |(pixel_position, _depth)| [pixel_position.x, pixel_position.y]);

        *self.projected_screen.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = screen;
    }

    /// Texture assets the effect references; the loader prefetches these.
    ///
    /// Every slot of a primitive's texture array is listed, not only the
    /// primary one: animated sequences (`texture_frames`) and two/three
    /// texture primitives (`texture_secondary`, `texture_tertiary`) must be
    /// resident when the motion advances `curMotion`.
    pub fn textures(&self) -> impl Iterator<Item = &str> + '_ {
        let primitives = self.runtime.primitives.iter().flat_map(|primitive| {
            let state = &primitive.state;

            state
                .texture
                .into_iter()
                .chain(state.texture_secondary)
                .chain(state.texture_tertiary)
                .chain(state.texture_frames.iter().copied())
        });

        // `CParticle` textures (`CThrowItemEffect`'s item bitmaps); they are
        // not part of a primitive, so the primitive chain above misses them.
        let particles = self.runtime.particles.iter().filter_map(|particle| particle.texture);

        // The `.str` clip's own texture list. Its paths are owned by the
        // holder (the loader builds them from the file's 128-byte names), so
        // they cannot be `&'static str` like the primitive paths.
        let str_textures = self.str_textures.iter().flatten().map(String::as_str);

        primitives.chain(particles).chain(str_textures)
    }

    /// Advances the effect by `delta_seconds` of wall time.
    ///
    /// The official client runs effects on a **fixed 16 ms** step, so a frame
    /// of 50 ms produces three steps and a 5 ms frame produces none.
    /// Returns `false` once the effect has finished (and keeps returning
    /// `false`).
    ///
    /// Sounds the body records are dropped; use [`Self::update_with_sounds`]
    /// to forward them to an audio sink.
    pub fn update(&mut self, delta_seconds: f64) -> bool {
        self.update_with_sounds(delta_seconds, &mut DiscardSounds)
    }

    /// Advances the effect and forwards every recorded sound to `sink`.
    ///
    /// The sounds are drained after **each** 16 ms step, in the order the body
    /// and the primitive pass recorded them, so a catch-up frame of 50 ms
    /// emits the sounds of its first step before those of its third and every
    /// sound is played exactly once. Positions are resolved by
    /// [`sound_sink::drain_into`].
    pub fn update_with_sounds(&mut self, delta_seconds: f64, sink: &mut dyn SoundSink) -> bool {
        if self.finished {
            return false;
        }

        self.accumulator_ms += delta_seconds * 1000.0;

        while self.accumulator_ms >= FRAME_FREQUENCY_MS as f64 {
            self.accumulator_ms -= FRAME_FREQUENCY_MS as f64;

            // The previous frame's `render` pass recorded the `LaunchEffect`
            // requests of the launcher primitives (`RenderSMA3`) on the
            // runtime outbox. Collect them before the step so the visual can
            // spawn the child effects into the client's effect list even when
            // this step retires the parent.
            self.effect_launches.extend(self.runtime.take_effect_launches());

            let alive = self.step();
            // The body may have played its last sound before returning
            // `false`, so drain even when the step retires the effect.
            sound_sink::drain_into(&mut self.runtime, sink);

            if !alive {
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
        self.runtime.begin_step();

        // The projected anchor the previous render pass stored. The official
        // body projects it at the top of `OnProcess`, before it launches the
        // primitives this step moves; the port has no camera here, so the
        // render pass hands the value over.
        self.runtime.screen = *self.projected_screen.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

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
    /// * the effect's own `.str` clips - `CRagEffect::Render`/ `RenderAniClip`,
    ///   see [`str_render`]. The pass runs after the primitive list and draws
    ///   only the clips `ProcessEZ2STR` marked as visible this frame (layer 0
    ///   is the resource's control layer and is never drawn).
    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, textures: &TextureLoader) {
        // `state.tlvertX/tlvertY` for the next 16 ms step: every official body
        // that draws a `PP_2DTEXTURE` projects its anchor before it moves the
        // primitives (`RagEffect.cpp:681-684` for `IncAgility`). `render` is
        // the only hook with the camera, so the value is parked for `step`.
        self.project_screen(camera, renderer.window_size());

        let anchor = Point3::new(self.runtime.position[0], self.runtime.position[1], self.runtime.position[2]);

        for primitive in &self.runtime.primitives {
            if !primitive.alive {
                continue;
            }

            let state = &primitive.state;

            // `CEffectPrim::Render`'s `case PP_SMA:` - `RenderSMA3`
            // (`EffectPrimitiveRendering.cpp:2385-2420`). The launcher submits
            // no geometry at all: it launches `EF_SMA2`/`EF_SMA3`/
            // `EF_SMA_BLACK2` at the slot's position, and the port records
            // those `LaunchEffect` requests on the runtime outbox for the
            // visual to spawn.
            if matches!(state.count, PrimitiveId::Sma) {
                render_slots::render_sma3(state, &self.runtime);

                continue;
            }

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
                // `PP_2DTEXTURE`/`PP_2DTEXTUREORBIT` -> `Render2DPrimitive`.
                //
                // Deviation: `PP_2DFLASH` (3) shares this branch but the
                // official draws it as a three-vertex screen-space fan
                // (`Render`'s `case PP_2DFLASH`, `EffectPrimitiveRendering.cpp:
                // 232-270`), not as this quad. Its geometry needs a
                // `longitude`/`arcAngle` fan, so it stays on the quad path
                // until that renderer is ported (see `PORT_STATUS.md`).
                PrimitiveId::Texture2d | PrimitiveId::Flash2d => {
                    let Some(path) = state.texture else {
                        continue;
                    };

                    let Ok(texture) = textures.get_or_load(path, ImageType::Color) else {
                        continue;
                    };

                    // `runtime.screen` is the `tlvert` the step that moved this
                    // primitive used; `quad_corners` subtracts it to obtain the
                    // anchor-relative offset the official draws.
                    let (corners, uvs) = quad_corners(state, self.runtime.screen);
                    let (source, destination) = blend_factors(state.render_flag);

                    // `Render2DPrimitive` writes `z = 0.000001`, `rhw =
                    // 0.999999` (`EffectPrimitiveRendering.cpp:7515-7516`),
                    // i.e. a screen-space quad at the near plane that is never
                    // occluded by the scene. The effect pass compares with
                    // `GreaterEqual` and does not write depth, so NDC depth
                    // `1.0` is the reverse-z equivalent of that near plane -
                    // without it the glyph is hidden behind the caster's own
                    // sprite, which is drawn closer than the feet anchor.
                    renderer.render_effect_with_depth(
                        camera,
                        anchor,
                        texture,
                        corners,
                        [SCREEN_QUAD_DEPTH; 4],
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

        // `CThrowItemEffect::Render` (`ThrowEffects.cpp:583-589`): the
        // effect's own `CParticle` list, drawn after the primitive pass. There
        // are no primitives on this path; the particles are the whole visual.
        particle::render_particles(renderer, camera, textures, &self.runtime.particles);

        // `CRagEffect::Render` - the effect's own `.str` pass, drawn once per
        // visible clip after the primitive list. It is driven by
        // `ProcessEZ2STR`'s `layerDrawn` flags, not by a `PrimitiveId`.
        str_render::render(
            renderer,
            camera,
            textures,
            anchor,
            self.runtime.str_player.as_ref(),
            &self.runtime.str_clips,
            &self.str_textures,
            str_render::screen_offset(self.runtime.str_offset),
        );
    }

    /// Child effects the renderer pass launched since the last drain, in
    /// submission order (the official `RenderSMA3` calls `LaunchEffect` from
    /// `Render`, `EffectPrimitiveRendering.cpp:2394-2415`).
    ///
    /// The ported visual takes these after each `update` and spawns them into
    /// the client's effect holder, which is where the official call appends the
    /// child to the scene's object list. The queue is only filled while the
    /// effect is stepped by a [`PortedEffectHolder`]; the hermetic
    /// `runtime::step` path leaves it alone.
    pub fn take_effect_launches(&mut self) -> Vec<PendingEffectLaunch> {
        std::mem::take(&mut self.effect_launches)
    }
}

/// Sink of [`PortedEffectHolder::update`]: drops the recorded sounds, for
/// callers that own no audio engine (the hermetic tests).
struct DiscardSounds;

impl SoundSink for DiscardSounds {
    fn play(&mut self, _sound: &sound_sink::PortedSound, _position: Point3<f32>) {}
}

/// `Render2DPrimitive`'s near-plane depth (`z = 0.000001`, `rhw = 0.999999`),
/// expressed in the client's reverse-z NDC where `1.0` is the near plane.
pub(crate) const SCREEN_QUAD_DEPTH: f32 = 1.0;

/// The four screen-space corners and UVs of a `Render2DPrimitive` quad.
///
/// Port of `CEffectPrim::Render2DPrimitive`
/// (`EffectPrimitiveRendering.cpp:7487-7589`), the renderer behind
/// `PP_2DTEXTURE` (and `PP_2DTEXTUREORBIT`):
///
/// * the quad is centred on `render.position`, the prim's absolute screen
///   position, which `Move2DPrimitive` builds as `tlvert + deltaPos2 +
///   deltaPos` (`EffectPrimitiveMotion.cpp:7777-7784`). The port stores that
///   value in `state.position` and subtracts the effect's own `screen` again,
///   so the corners are anchor-relative pixel offsets (plus [`EFFECT_ORIGIN`],
///   which [`EffectRenderer::render_effect`] removes before it re-adds the
///   projected anchor);
/// * `widthSize`/`heightSize` are half extents, so the official Increase AGI
///   glyph is 80x40 pixels for `40`/`20` (the `agi_up.bmp` texture is 64x32,
///   same 2:1 aspect);
/// * `roll` is truncated to integer degrees and read from the client's sine
///   table (`ClientSine(static_cast<int>(state.roll))`), not `sinf`;
/// * `pattern & 1` anchors the quad's bottom edge on `render.position` and
///   doubles the height upward instead of centring it (`extra = 0`, `negHeight
///   = -2 * height`);
/// * every emitted vertex gets `- 0.5f`, the official half-pixel shift.
///
/// The official emits the quad as two triangles with the vertex order
/// `v0 = (-w, +extra)`, `v1 = (-w, negHeight)`, `v2 = (+w, +extra)`,
/// `v3 = (+w, negHeight)` and the UVs `v0 (0,1)`, `v1 (0,0)`, `v2 (1,1)`,
/// `v3 (1,0)` - i.e. bottom-left, top-left, bottom-right, top-right in the
/// official y-down screen space. [`EffectRenderer::render_effect`] takes the
/// corners in its own layout and pairs `corners[i]` with `uvs[perm[i]]`,
/// `perm = [2, 1, 3, 0]`, so the UV array is permuted here to keep every
/// corner with its own official UV. Getting that pairing wrong folds the
/// texture (the Increase AGI banner was reported as drawn upside down).
pub(crate) fn quad_corners(state: &EffectPrimitiveState, anchor_screen: [f32; 2]) -> ([Vector2<f32>; 4], [Vector2<f32>; 4]) {
    let angles = Angles::shared();
    let sin = angles.sine(state.roll as i32);
    let cos = angles.cosine(state.roll as i32);

    let width = state.width_size;
    let height = state.height_size;

    // `Render2DPrimitive:7494-7500`: `negHeight = -height` (doubled with
    // `pattern & 1`) and `extra = height` unless the height is anchored.
    let (negative_height, extra) = match (state.pattern & 1) != 0 {
        true => (-height - height, 0.0),
        false => (-height, height),
    };

    // The official adds `- 0.5f` to every vertex after the rotation, so it is
    // equivalent (and cheaper) to fold it into the centre.
    let center = Vector2::new(
        state.position[0] - anchor_screen[0] - 0.5,
        state.position[1] - anchor_screen[1] - 0.5,
    );

    let rotate = |x: f32, y: f32| Vector2::new(x * cos - y * sin + center.x, x * sin + y * cos + center.y);

    let corners = [
        EFFECT_ORIGIN + rotate(-width, extra),
        EFFECT_ORIGIN + rotate(-width, negative_height),
        EFFECT_ORIGIN + rotate(width, extra),
        EFFECT_ORIGIN + rotate(width, negative_height),
    ];

    // Official UVs per vertex: v0 (0,1), v1 (0,0), v2 (1,1), v3 (1,0),
    // permuted into the renderer's `[BR, TR, TL, BL]` slot order.
    let uvs = [
        Vector2::new(1.0, 0.0),
        Vector2::new(0.0, 0.0),
        Vector2::new(0.0, 1.0),
        Vector2::new(1.0, 1.0),
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
    use super::super::sound_sink::RecordingSink;
    use super::super::str_player::{KeyFrame, Transform};
    use super::*;

    /// A camera at `(0, 0, -10)` looking down `+z`, 45 degree vertical FOV on a
    /// 640x480 window; the same conventions as the game cameras (left handed,
    /// y up, reverse-z) because the matrices come from the shared helpers.
    struct TestCamera {
        view: cgmath::Matrix4<f32>,
        projection: cgmath::Matrix4<f32>,
        view_projection: cgmath::Matrix4<f32>,
    }

    impl TestCamera {
        fn new() -> Self {
            let view = cgmath::Matrix4::look_to_lh(
                Point3::new(0.0, 0.0, -10.0),
                cgmath::Vector3::unit_z(),
                cgmath::Vector3::unit_y(),
            );
            let projection = crate::graphics::perspective_reverse_lh(cgmath::Deg(45.0), 4.0 / 3.0);

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

        fn look_up_vector(&self) -> cgmath::Vector3<f32> {
            cgmath::Vector3::unit_y()
        }

        fn view_projection_matrices(&self) -> (cgmath::Matrix4<f32>, cgmath::Matrix4<f32>) {
            (self.view, self.projection)
        }

        fn view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
            self.view_projection
        }

        fn view_direction(&self) -> cgmath::Vector3<f32> {
            cgmath::Vector3::unit_z()
        }
    }

    const WINDOW: ScreenSize = ScreenSize {
        width: 640.0,
        height: 480.0,
    };

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

    /// The UV `EffectRenderer` pairs with `corners[i]`: it stores
    /// `texture_coordinates[2]` for the top-left, `[3]` for the bottom-left,
    /// `[1]` for the top-right and `[0]` for the bottom-right
    /// (`renderer/effect.rs:272-283`).
    fn paired_uvs(uvs: [Vector2<f32>; 4]) -> [Vector2<f32>; 4] {
        [uvs[2], uvs[1], uvs[3], uvs[0]]
    }

    #[test]
    fn two_d_quad_corners_match_the_official_vertices_and_uvs() {
        let state = EffectPrimitiveState {
            position: [10.0, 20.0, 0.0],
            width_size: 2.0,
            height_size: 3.0,
            roll: 0.0,
            ..Default::default()
        };

        let (corners, uvs) = quad_corners(&state, [4.0, 6.0]);

        // Official `Render2DPrimitive` vertices, anchor-relative, with the
        // `- 0.5f` half-pixel shift and `EFFECT_ORIGIN` added back for the
        // renderer (which subtracts it again).
        let origin = EFFECT_ORIGIN;
        let center = Vector2::new(10.0 - 4.0 - 0.5, 20.0 - 6.0 - 0.5);
        assert_eq!(corners, [
            origin + center + Vector2::new(-2.0, 3.0),
            origin + center + Vector2::new(-2.0, -3.0),
            origin + center + Vector2::new(2.0, 3.0),
            origin + center + Vector2::new(2.0, -3.0),
        ]);
        assert_eq!(uvs, [
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]);

        // With the renderer's pairing every corner keeps its own official UV:
        // bottom-left (0,1), top-left (0,0), bottom-right (1,1), top-right
        // (1,0). If this pairing flips, the banner is drawn mirrored/folded.
        assert_eq!(paired_uvs(uvs), [
            Vector2::new(0.0, 1.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 0.0),
        ]);
    }

    #[test]
    fn quad_corners_translate_with_the_anchor_and_keep_the_same_uvs() {
        let state = EffectPrimitiveState {
            position: [320.0, 240.0, 0.0],
            width_size: 10.0,
            height_size: 20.0,
            roll: 0.0,
            ..Default::default()
        };

        let (a, uvs_a) = quad_corners(&state, [300.0, 200.0]);
        let (b, _) = quad_corners(&state, [320.0, 220.0]);

        // The glyph is drawn relative to the projected anchor: moving the
        // anchor by (20, 20) moves every corner by exactly (-20, -20).
        for index in 0..4 {
            assert_eq!(a[index] - b[index], Vector2::new(20.0, 20.0));
        }
        assert_eq!(uvs_a, [
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]);
    }

    #[test]
    fn pattern_one_doubles_the_quad_height_upward() {
        let state = EffectPrimitiveState {
            position: [0.0, 0.0, 0.0],
            width_size: 10.0,
            height_size: 20.0,
            roll: 0.0,
            pattern: 1,
            ..Default::default()
        };

        let (corners, _) = quad_corners(&state, [0.0, 0.0]);

        // `negHeight = -2 * height`, `extra = 0`: the quad starts at the
        // anchor and extends twice the height upward instead of being
        // centred on it (y grows downward, so upward is negative).
        assert_eq!(corners[0], EFFECT_ORIGIN + Vector2::new(-10.5, -0.5));
        assert_eq!(corners[1], EFFECT_ORIGIN + Vector2::new(-10.5, -40.5));
        assert_eq!(corners[2], EFFECT_ORIGIN + Vector2::new(9.5, -0.5));
        assert_eq!(corners[3], EFFECT_ORIGIN + Vector2::new(9.5, -40.5));
    }

    #[test]
    fn quad_corners_rotate_with_the_integer_degree_table() {
        let angles = Angles::shared();
        let state = EffectPrimitiveState {
            position: [0.0, 0.0, 0.0],
            width_size: 10.0,
            height_size: 20.0,
            roll: 90.0,
            ..Default::default()
        };

        let (corners, _) = quad_corners(&state, [0.0, 0.0]);

        // `R(-w, +h)` with the client's 90 degree sine/cosine.
        let (sin, cos) = (angles.sine(90), angles.cosine(90));
        let expected = Vector2::new(-10.0 * cos - 20.0 * sin - 0.5, -10.0 * sin + 20.0 * cos - 0.5);
        assert!((corners[0].x - (EFFECT_ORIGIN.x + expected.x)).abs() < 1e-4);
        assert!((corners[0].y - (EFFECT_ORIGIN.y + expected.y)).abs() < 1e-4);

        // The roll is truncated to whole degrees, exactly like
        // `ClientSine(static_cast<int>(this->state.roll))`: 90.9 uses 90.
        let mut truncated = state;
        truncated.roll = 90.9;
        let (same, _) = quad_corners(&truncated, [0.0, 0.0]);
        assert_eq!(same, corners);
    }

    /// The client's texture loader resolves a body's `effect\agi_up.bmp` as
    /// `data\texture\effect\agi_up.bmp` (`loaders/texture/mod.rs`,
    /// `load_texture_data` prefixes `data\texture\`) and keys a BMP's magenta
    /// background to alpha 0. This checks the real assets through the real
    /// archive loader, not just the paths the bodies carry.
    ///
    /// Skipped when the client configuration is not present (the same guard the
    /// other archive-backed tests use).
    #[test]
    fn incagility_textures_exist_in_the_game_archives() {
        use ro_next_loaders::FileLoader;

        if !std::path::Path::new("client/game_archives.ron").exists() {
            return;
        }

        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        for path in [
            "data\\texture\\effect\\agi_up.bmp",
            "data\\texture\\effect\\dex_agi_up.bmp",
            "data\\texture\\effect\\ac_center2.tga",
        ] {
            assert!(game_file_loader.file_exists(path), "missing texture {path}");
        }

        // 64x32 8-bit indexed BMP; the official glyph quad is 2 * 40 by
        // 2 * 20 = 80x40, the same 2:1 aspect.
        let bmp = game_file_loader
            .get("data\\texture\\effect\\agi_up.bmp")
            .expect("agi_up.bmp is readable");
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(i32::from_le_bytes(bmp[18..22].try_into().unwrap()), 64);
        assert_eq!(i32::from_le_bytes(bmp[22..26].try_into().unwrap()), 32);
        assert_eq!(u16::from_le_bytes(bmp[28..30].try_into().unwrap()), 8);
    }

    #[test]
    fn project_screen_sets_the_runtime_screen_before_the_first_step() {
        let mut holder = PortedEffectHolder::new(EffectId::Incagility, 7).unwrap();
        assert_eq!(holder.runtime().screen, [0.0, 0.0], "no projection yet");

        holder.runtime_mut().position = [0.0, 0.0, 0.0];
        holder.project_screen(&TestCamera::new(), WINDOW);

        // `project_screen` only parks the value (render has `&self`); the next
        // 16 ms step hands it to the runtime, which is the step whose
        // `Move2DPrimitive` consumes it - exactly the official order.
        holder.update(0.016);

        assert_eq!(holder.runtime().screen, [320.0, 240.0], "the anchor is the window centre");
        assert_eq!(holder.runtime().primitives[0].state.count, PrimitiveId::Texture2d);
    }

    #[test]
    fn project_screen_follows_the_effect_anchor() {
        let mut holder = PortedEffectHolder::new(EffectId::Incagility, 7).unwrap();
        holder.runtime_mut().position = [10.0, 0.0, 0.0];

        holder.project_screen(&TestCamera::new(), WINDOW);
        holder.update(0.016);

        let [x, y] = holder.runtime().screen;
        assert!(x > 320.0, "a caster right of the screen axis projects right of the centre");
        assert!((y - 240.0).abs() < 1e-4);

        // The projection scales with the window: 1280x960 puts the centre at
        // (640, 480) and doubles the offset from it.
        let wide = ScreenSize {
            width: 1280.0,
            height: 960.0,
        };
        holder.project_screen(&TestCamera::new(), wide);
        holder.update(0.016);
        assert_eq!(holder.runtime().screen, [640.0 + (x - 320.0) * 2.0, 480.0]);
    }

    #[test]
    fn project_screen_is_zero_behind_the_camera() {
        let mut holder = PortedEffectHolder::new(EffectId::Incagility, 7).unwrap();
        // The camera looks down `+z` from `-10`; this anchor is behind it.
        holder.runtime_mut().position = [0.0, 0.0, -50.0];

        holder.project_screen(&TestCamera::new(), WINDOW);
        holder.update(0.016);

        assert_eq!(holder.runtime().screen, [0.0, 0.0]);
    }

    #[test]
    fn incagility_glyph_quad_is_anchored_on_the_projected_caster() {
        let mut holder = PortedEffectHolder::new(EffectId::Incagility, 7).unwrap();
        holder.runtime_mut().position = [100.0, 0.0, 150.0];

        holder.project_screen(&TestCamera::new(), WINDOW);
        holder.update(0.016);

        let runtime = holder.runtime();
        let screen = runtime.screen;
        let glyph = &runtime.primitives[0].state;

        // The step moved the glyph once: it rises along `-y`.
        assert!(glyph.position[1] < screen[1], "the glyph rises above the anchor");
        assert!((glyph.position[0] - screen[0]).abs() < 1e-4, "longitude 0 keeps x");

        let (corners, uvs) = quad_corners(glyph, screen);
        let center = Vector2::new(glyph.position[0] - screen[0] - 0.5, glyph.position[1] - screen[1] - 0.5);

        // 40x20 half extents -> an 80x40 quad (the official `widthSize`/
        // `heightSize` are half sizes; `agi_up.bmp` is 64x32, same 2:1).
        assert_eq!(corners, [
            EFFECT_ORIGIN + center + Vector2::new(-40.0, 20.0),
            EFFECT_ORIGIN + center + Vector2::new(-40.0, -20.0),
            EFFECT_ORIGIN + center + Vector2::new(40.0, 20.0),
            EFFECT_ORIGIN + center + Vector2::new(40.0, -20.0),
        ]);
        assert_eq!(uvs, [
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]);
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
        // Thunderstorm is ported now; `EF_STORMKICK8` (1223) still needs the
        // unported `WhiteLightTrail`/`SetFrameSequence` helpers.
        assert!(PortedEffectHolder::new(EffectId::Stormkick8, 1).is_none());
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
    fn str_texture_names_are_carried_for_the_draw_pass_and_prefetch() {
        let mut holder = PortedEffectHolder::new(EffectId::Signum, 1).unwrap().with_str_clip_set(ClipSet {
            clips: clip(&[(0, 10.0)]),
            textures: vec![vec!["effect\\cross01.bmp".to_owned(), "effect\\cross02.bmp".to_owned()]],
        });

        assert_eq!(holder.runtime().str_clips.len(), 1);
        assert_eq!(holder.runtime().str_player.as_ref().unwrap().transforms[0].x, 0.0);

        assert!(holder.update(0.016));

        let textures: Vec<_> = holder.textures().collect();
        assert!(textures.contains(&"effect\\cross01.bmp"));
        assert!(textures.contains(&"effect\\cross02.bmp"));
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

    /// One step that records two inline `PlayEffectWaveInline` sounds and one
    /// positional `PlayWave` offset. No single ported body uses both channels
    /// today, so this is the synthetic mixed case for the drain order.
    struct MixedChannels;

    impl PortedEffect for MixedChannels {
        fn lifetime(&self) -> i32 {
            20
        }

        fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
            if runtime.frame == 0 {
                runtime.play_sound("effect\\EF_Signum.wav");
                runtime.play_sound("effect\\EF_Bash.wav");
                runtime.play_sound_at("effect\\cru_holy cross.wav", [3.0, 1.0, 5.0]);
            }

            true
        }
    }

    #[test]
    fn all_recorded_channels_play_once_in_recorded_order_with_their_offsets() {
        let mut holder = PortedEffectHolder::with_body(Box::new(MixedChannels), 1);
        holder.runtime_mut().position = [13.0, 7.0, 25.0];
        holder.runtime_mut().listener_position = [10.0, 7.0, 20.0];

        let mut sink = RecordingSink::default();
        assert!(holder.update_with_sounds(0.016, &mut sink));

        assert_eq!(sink.plays, vec![
            ("effect\\EF_Signum.wav".to_owned(), None, Point3::new(13.0, 7.0, 25.0)),
            ("effect\\EF_Bash.wav".to_owned(), None, Point3::new(13.0, 7.0, 25.0)),
            (
                "effect\\cru_holy cross.wav".to_owned(),
                Some([3.0, 1.0, 5.0]),
                Point3::new(13.0, 8.0, 25.0)
            ),
        ]);

        // Played once: a later step has nothing left to drain.
        assert!(holder.update_with_sounds(0.016, &mut sink));
        assert_eq!(sink.plays.len(), 3);
    }

    struct OneSoundPerStep;

    impl PortedEffect for OneSoundPerStep {
        fn lifetime(&self) -> i32 {
            20
        }

        fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
            runtime.play_sound(&format!("effect\\step{}.wav", runtime.frame));

            true
        }
    }

    #[test]
    fn a_catch_up_frame_drains_every_step_in_order() {
        let mut holder = PortedEffectHolder::with_body(Box::new(OneSoundPerStep), 1);
        let mut sink = RecordingSink::default();

        assert!(holder.update_with_sounds(0.050, &mut sink));

        // 50 ms = three 16 ms steps (frames 0, 1 and 2), each drained on its
        // own: the sounds of the first step are not deferred behind the third.
        assert_eq!(holder.runtime().frame, 3);

        let names: Vec<_> = sink.plays.iter().map(|(name, ..)| name.as_str()).collect();
        assert_eq!(names, ["effect\\step0.wav", "effect\\step1.wav", "effect\\step2.wav"]);
    }

    struct LastSound;

    impl PortedEffect for LastSound {
        fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
            runtime.play_sound("effect\\last.wav");

            false
        }

        fn lifetime(&self) -> i32 {
            1
        }
    }

    #[test]
    fn a_sound_recorded_by_the_retiring_step_is_still_played() {
        let mut holder = PortedEffectHolder::with_body(Box::new(LastSound), 1);
        let mut sink = RecordingSink::default();

        assert!(!holder.update_with_sounds(0.016, &mut sink));
        assert_eq!(sink.plays.len(), 1);
        assert_eq!(sink.plays[0].0, "effect\\last.wav");
    }

    #[test]
    fn the_holder_drains_the_runtime_effect_outbox_on_the_next_step() {
        // `RenderSMA3` records the launches during `render`; the holder drains
        // them on the following 16 ms step and exposes them to the visual.
        let mut holder = PortedEffectHolder::new(EffectId::Stin3, 1).unwrap();

        holder.runtime().launch_effect(EffectId::Sma3, [1.0, 2.0, 3.0], None);
        holder
            .runtime()
            .launch_effect(EffectId::Sma2, [4.0, 5.0, 6.0], Some([8.0, 1.0, 0.0]));

        assert!(holder.update(0.016));

        assert_eq!(holder.take_effect_launches(), vec![
            PendingEffectLaunch {
                effect: EffectId::Sma3,
                position: [1.0, 2.0, 3.0],
                delta_pos2: None,
            },
            PendingEffectLaunch {
                effect: EffectId::Sma2,
                position: [4.0, 5.0, 6.0],
                delta_pos2: Some([8.0, 1.0, 0.0]),
            },
        ]);

        // Drained: the next step hands over nothing.
        assert!(holder.update(0.016));
        assert!(holder.take_effect_launches().is_empty());
    }

    #[test]
    fn a_real_body_forwards_its_official_sounds_through_the_holder() {
        // Signum records `EF_Signum.wav` then `EF_Bash.wav` on frame 30
        // (`RagEffect.cpp:2107-2154`).
        let mut holder = PortedEffectHolder::new(EffectId::Signum, 1).unwrap();
        let mut sink = RecordingSink::default();

        for _ in 0..31 {
            assert!(holder.update_with_sounds(0.016, &mut sink));
        }

        let names: Vec<_> = sink.plays.iter().map(|(name, ..)| name.as_str()).collect();
        assert_eq!(names, ["effect\\EF_Signum.wav", "effect\\EF_Bash.wav"]);
        assert_eq!(sink.plays[0].2, Point3::new(0.0, 0.0, 0.0));
    }
}
