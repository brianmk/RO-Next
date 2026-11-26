//! `CRagEffect::FireBall` - `RagEffect.cpp:325-367`, effect id `EF_FIREBALL`.
//!
//! Four `PP_3DPARTICLE` fire sprites on frames 20/24/28/32, fired from the
//! caster toward the target with a per-frame colour ramp.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct FireBall {
    spawned: bool,
}

impl PortedEffect for FireBall {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame != 20 && frame != 24 && frame != 28 && frame != 32 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\EF_FireBall.wav");

        let angles = Angles::shared();

        // `CEffectPrim::Face(deltaPos.x, deltaPos.z)` from the caster position.
        let delta_x = runtime.target_offset[0] - runtime.position[0];
        let delta_z = runtime.target_offset[2] - runtime.position[2];
        let mut face = 0.0f32;

        if delta_x != 0.0 || delta_z != 0.0 {
            face = ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0;

            if face >= 360.0 {
                face -= 360.0;
            }

            if face < 0.0 {
                face += 360.0;
            }
        }

        let mut render_angle = 180.0 - face;

        if render_angle < 0.0 {
            render_angle += 360.0;
        }

        let distance = ((delta_x * delta_x + delta_z * delta_z) as f32).sqrt();

        let particle = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(particle);

        state.duration = 20;
        state.pattern |= 0x120;
        state.delta_pos2 = [0.0, -15.0, 0.0];
        state.longitude = -face;
        state.render_angle = render_angle;
        state.speed = -(distance / state.duration as f32);
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.size = 1.3;
        state.anim_speed = 1;

        if frame != 20 {
            state.red = 246;
            state.green = 199;
            state.blue = 76;

            state.alpha = match frame {
                24 => 180.0,
                28 => 130.0,
                _ => 80.0,
            };
        }

        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 91);
        runtime.target_offset = [10.0, 0.0, 0.0];
        runtime
    }

    #[test]
    fn spawns_on_frames_20_24_28_32() {
        let mut effect = FireBall::default();
        let mut runtime = runtime();

        while runtime.frame < 33 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 4);
        assert_eq!(runtime.sounds.len(), 4);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::Particle3d);
        assert_eq!(first.duration, 20);
        assert_eq!(first.pattern & 0x120, 0x120);
        assert_eq!(first.delta_pos2, [0.0, -15.0, 0.0]);
        assert_eq!(first.size, 1.3);
        assert_eq!(first.anim_speed, 1);
        assert_eq!(first.alpha, 254.0, "frame 20 keeps the constructor alpha");
        assert_eq!(first.red, 255, "frame 20 keeps the constructor colours");

        let second = &runtime.primitives[1].state;
        assert_eq!((second.red, second.green, second.blue), (246, 199, 76));
        assert_eq!(second.alpha, 180.0);
    }

    #[test]
    fn faces_the_target_and_wraps_the_angle() {
        let mut effect = FireBall::default();
        let mut runtime = runtime();

        while runtime.frame < 21 {
            step(&mut effect, &mut runtime);
        }

        let state = &runtime.primitives[0].state;

        // Target +x -> primitive Face = 90 -> longitude -90, render angle 90.
        assert!((state.longitude + 90.0).abs() < 1.0);
        assert!((state.render_angle - 90.0).abs() < 1.0);
        assert!(state.render_angle >= 0.0 && state.render_angle < 360.0);
        assert!(state.speed < 0.0, "the sprite flies toward the caster side");
    }
}
