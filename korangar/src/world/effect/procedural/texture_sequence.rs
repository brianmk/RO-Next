//! Animated texture sequence primitive (flipbook billboard).
//!
//! The official client has several effect components that draw a **sequence of
//! textures** on one primitive instead of a single one: `PP_3DTEXTURE` with
//! `totalTexture > 1` (e.g. `EF_JUPITEL`'s `thunder_ball_a..f`) and
//! `PP_3DCROSSTEXTURE` with a frame list (e.g. `EF_FIREARROW`'s
//! `불화살1..6`). The effect table records these as multi-texture layers with a
//! `totalTexture`/`animSpeed` pair; the unified pipeline carries them as
//! `frames` + `frame_ms` on a `Texture` layer.
//!
//! This primitive plays them as a camera facing billboard that shows one frame
//! for `frame_ms` and fades the whole sequence in and out, so the animation is
//! visible as a flipbook instead of a static first frame.

use std::sync::Arc;

use cgmath::Point3;

use super::{FULL_QUAD_TEXTURE_COORDINATES, blend_factors, camera_facing_corners, raised};
use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::procedural::PrimitiveParams;
use crate::world::effect::{EffectBase, EffectCenter, render_world_quad_blended};

/// Fraction of the sequence lifetime used to fade in / out.
const FADE_IN: f32 = 0.10;
const FADE_OUT: f32 = 0.20;

/// A billboard that plays an ordered texture sequence once.
pub struct TextureSequence {
    frames: Vec<Arc<Texture>>,
    center: EffectCenter,
    height: f32,
    frame_time: f32,
    duration: f32,
    size: f32,
    size_end: f32,
    alpha: f32,
    color: Color,
    additive: bool,
    time: f32,
    gets_deleted: bool,
}

impl TextureSequence {
    /// Spawns the primitive for one data layer. Frames that are not part of the
    /// game data are skipped; returns `None` when no frame can be loaded.
    pub fn spawn(
        texture_loader: &TextureLoader,
        center: EffectCenter,
        frames: &[&str],
        frame_ms: u32,
        spec: &PrimitiveParams<'_>,
    ) -> Option<Self> {
        let mut loaded = Vec::with_capacity(frames.len());
        for frame in frames {
            if let Ok(texture) = texture_loader.get_or_load(frame, ImageType::Color) {
                loaded.push(texture);
            }
        }
        if loaded.is_empty() {
            return None;
        }

        let frame_time = frame_ms.clamp(16, 2000) as f32 / 1000.0;
        let duration = (frame_time * loaded.len() as f32).max(frame_time);

        Some(Self {
            frames: loaded,
            center,
            height: spec.height,
            frame_time,
            duration,
            size: spec.size.max(1.0),
            size_end: spec.size_end.max(1.0),
            alpha: spec.alpha.clamp(0.0, 1.0),
            color: Color::rgba(spec.color.red, spec.color.green, spec.color.blue, 1.0),
            additive: spec.additive,
            time: 0.0,
            gets_deleted: false,
        })
    }

    /// Index of the frame shown at `elapsed` seconds (last frame is held).
    fn frame_index(elapsed: f32, frame_time: f32, frames: usize) -> usize {
        if frames == 0 {
            return 0;
        }
        let index = (elapsed / frame_time.max(f32::EPSILON)).floor() as usize;
        index.min(frames - 1)
    }

    /// Fade weight of the whole sequence (0..1).
    fn fade(&self) -> f32 {
        let progress = (self.time / self.duration.max(f32::EPSILON)).clamp(0.0, 1.0);
        let fade_in = (progress / FADE_IN).clamp(0.0, 1.0);
        let fade_out = ((1.0 - progress) / FADE_OUT).clamp(0.0, 1.0);
        fade_in.min(fade_out)
    }
}

impl EffectBase for TextureSequence {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(raised(self.center.to_position(), self.height))
    }

    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.center.follow_entity(entities);
        self.time += delta_time;
        !self.gets_deleted && self.time < self.duration
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let fade = self.fade();
        if fade <= 0.001 {
            return;
        }

        let progress = (self.time / self.duration).clamp(0.0, 1.0);
        let size = self.size + (self.size_end - self.size) * progress;
        let alpha = self.alpha * fade;
        let color = Color::rgba(self.color.red, self.color.green, self.color.blue, alpha);
        let (source_blend_factor, destination_blend_factor) = blend_factors(self.additive);
        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };
        let position = raised(self.center.to_position(), self.height);
        let corners = camera_facing_corners(camera, position, size * 0.5, 0.0);
        let frame = Self::frame_index(self.time, self.frame_time, self.frames.len());

        render_world_quad_blended(
            renderer,
            camera,
            position,
            screen_size,
            self.frames[frame].clone(),
            color,
            corners,
            FULL_QUAD_TEXTURE_COORDINATES,
            source_blend_factor,
            destination_blend_factor,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::TextureSequence;

    #[test]
    fn frame_index_advances_with_time_and_holds_the_last_frame() {
        assert_eq!(TextureSequence::frame_index(0.0, 0.08, 3), 0);
        assert_eq!(TextureSequence::frame_index(0.07, 0.08, 3), 0);
        assert_eq!(TextureSequence::frame_index(0.08, 0.08, 3), 1);
        assert_eq!(TextureSequence::frame_index(0.16, 0.08, 3), 2);
        assert_eq!(TextureSequence::frame_index(5.0, 0.08, 3), 2);
    }

    #[test]
    fn frame_index_handles_an_empty_sequence() {
        assert_eq!(TextureSequence::frame_index(1.0, 0.08, 0), 0);
    }
}
