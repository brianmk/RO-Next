//! `CRagEffect::GlassWall` - `RagEffect.cpp:10970-11020`.
//!
//! `EF_GLASSWALL`: four `ring_blue.tga` strips around the caster (two per side,
//! mirrored on X and Z), `pattern 40`, `type 50`, rising 0.25 per frame, and
//! the effect retires on its own duration.

use super::super::angles::Angles;
use super::super::math::{Matrix, matrix_mult};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct GlassWall {
    lifetime: i32,
}

impl GlassWall {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for GlassWall {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let lifetime = runtime.lifetime;
        let angles = Angles::shared();

        if frame == 0 {
            runtime.play_sound("effect\\EF_GlassWall.wav");

            let mut angle = (180.0 - runtime.parent_angle) as i32;

            if angle < 0 {
                angle += 360;
            }

            let mut rotation = Matrix::identity();
            rotation.make_y_rotation(angles.radian(angle));

            for side in [1.0f32, -1.0f32] {
                let offsets = [(2.6 * side, 0.0, 3.0, angle + 90), (3.0 * side, 0.0, 2.6, angle)];

                for (index, (z_offset, x_offset, width, longitude)) in offsets.iter().enumerate() {
                    let strip = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
                    let state = runtime.primitive_mut(strip);

                    state.duration = lifetime;
                    state.pattern = 40;
                    state.type_field = 50;

                    let offset = if index == 0 { [0.0, 0.0, *z_offset] } else { [*x_offset, 0.0, 0.0] };
                    state.delta_pos2 = matrix_mult(offset, &rotation.values);

                    state.longitude = *longitude as f32;
                    state.width_size = *width;
                    state.height_size = 40.0;
                    state.height_speed = 0.25;
                    state.alpha = 0.0;
                    state.max_alpha = 180.0;
                    state.alpha_speed = state.max_alpha / 6.0;
                    state.fade_out_cnt = state.duration - 6;
                    state.texture = Some("effect\\ring_blue.tga");
                }
            }
        }

        runtime.process_str();

        // The original returns false when the effect reaches its duration.
        if frame == lifetime {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn glass_wall_spawns_four_strips() {
        let mut effect = GlassWall::new(9999);
        let mut runtime = EffectRuntime::new(9999, [0.0, 0.0], 501);
        runtime.parent_angle = 0.0;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_GlassWall.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 4);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::Texture3d);
        assert_eq!(first.duration, 9999);
        assert_eq!(first.pattern, 40);
        assert_eq!(first.type_field, 50);
        assert_eq!(first.width_size, 3.0);
        // One texture-3d step raised it by heightSpeed.
        assert!((first.height_size - 40.25).abs() < 1e-4);
        assert!((first.height_speed.abs() - 0.25).abs() < 1e-6);
        assert_eq!(first.max_alpha, 180.0);
        assert_eq!(first.fade_out_cnt, 9993);
        assert_eq!(first.texture, Some("effect\\ring_blue.tga"));
        // angle 180 -> first strip longitude 270
        assert_eq!(first.longitude, 270.0);

        let second = &runtime.primitives[1].state;
        assert_eq!(second.width_size, 2.6);
        assert_eq!(second.longitude, 180.0);
    }

    #[test]
    fn glass_wall_retires_on_its_duration() {
        let mut effect = GlassWall::new(5);
        let mut runtime = EffectRuntime::new(5, [0.0, 0.0], 501);

        // Frames 1..=4 stay alive; frame 5 equals the duration and retires.
        for _ in 0..4 {
            assert!(step(&mut effect, &mut runtime));
        }

        assert!(!step(&mut effect, &mut runtime), "frame 5 retires the effect");
    }
}
