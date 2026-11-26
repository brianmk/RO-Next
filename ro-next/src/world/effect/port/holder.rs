//! `PortedEffectHolder` - owns one ported effect body and steps it on the fixed
//! 16 ms clock of the official client.
//!
//! This is the object the client will keep in its effect list once the ported
//! backend is wired (plan 4.2). It deliberately separates the *simulation*
//! (which is exact and testable) from the *rendering* (which needs textures and
//! the effect renderer): `update` only advances the timeline, so the timing
//! model can be verified without a GPU.

use cgmath::{Point3, Rad, Vector2};
use ragnarok_packets::EffectId;
use wgpu::BlendFactor;

use crate::graphics::Color;
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

use super::FRAME_FREQUENCY_MS;
use super::primitive::{EffectPrimitiveState, PrimitiveId};
use super::registry;
use super::render3d;
use super::runtime::{EffectRuntime, PortedEffect};

pub struct PortedEffectHolder {
    body: Box<dyn PortedEffect>,
    runtime: EffectRuntime,
    /// Milliseconds not yet consumed by a 16 ms step.
    accumulator_ms: f64,
    finished: bool,
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
        })
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
    /// The official client runs effects on a **fixed 16 ms** step, so a frame of
    /// 50 ms produces three steps and a 5 ms frame produces none. Returns
    /// `false` once the effect has finished (and keeps returning `false`).
    pub fn update(&mut self, delta_seconds: f64) -> bool {
        if self.finished {
            return false;
        }

        self.accumulator_ms += delta_seconds * 1000.0;

        while self.accumulator_ms >= FRAME_FREQUENCY_MS as f64 {
            self.accumulator_ms -= FRAME_FREQUENCY_MS as f64;

            if !super::runtime::step(self.body.as_mut(), &mut self.runtime) {
                self.finished = true;
                return false;
            }
        }

        true
    }

    /// Submits every primitive the port can draw today.
    ///
    /// * `PP_2DTEXTURE`/`PP_2DFLASH` - screen-space quads
    ///   (`Render2DPrimitive`).
    /// * `PP_3DTEXTURE`/`PP_3DCROSSTEXTURE` - world-space quads
    ///   (`DrawEffectPrimitive15`/`RenderCrossPlanes`), see [`render3d`].
    ///
    /// Rings, cylinders, arcs and the sprite primitives need geometry/render
    /// paths that are not ported yet; they are skipped rather than guessed.
    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, textures: &TextureLoader) {
        let anchor = Point3::new(
            self.runtime.position[0],
            self.runtime.position[1],
            self.runtime.position[2],
        );

        for primitive in &self.runtime.primitives {
            if !primitive.alive {
                continue;
            }

            let state = &primitive.state;

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
                // TODO(sprite pass): `Particle3d`/`Particle3dGravity`/`Orbit`
                // (and the `.spr` bodies of `rag::class_change`) only carry
                // `sprite`/`action`; `ParticleUp`, `PulsingColumns`,
                // `RippleRings`, `RisingFan`, `Halo`, `SpinQuad`,
                // `TextureSpin`, `WanderingFlames` are slot/record primitives.
                // `SpriteEffect` is not reusable as-is: it needs
                // `Arc<Sprite>`/`Arc<Actions>` from `SpriteLoader`/
                // `ActionLoader`, neither of which reaches
                // `PortedEffectHolder::render`, and the official
                // `RenderSegmentSprites2` projects every `records[seg]` and
                // draws one sprite cell per ACT cell with
                // `sprite->images[cell.imageType][cell.image]` (the port's
                // `texture_secondary` slot), scaling by the record's
                // `scale`/`cell.scaleY` and the record's depth. Missing: the
                // two loaders in the holder context, a per-record cell
                // renderer, and the ACT frame selection from
                // `action_index`/`action_frame_count` plus `cur_motion`.
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
    let alpha = ((state.argb >> 24) & 0xFF) as f32 / 255.0;
    let red = ((state.argb >> 16) & 0xFF) as f32 / 255.0;
    let green = ((state.argb >> 8) & 0xFF) as f32 / 255.0;
    let blue = (state.argb & 0xFF) as f32 / 255.0;

    Color::rgba(red, green, blue, alpha)
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
    use super::*;

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
}
