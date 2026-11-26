//! `CRagEffect::RocketPunch` / `CRagEffect::GreenRocketPunch` -
//! `RagEffect.cpp:8079-8127` and `8131-8185`.
//!
//! Four `남_로켓트펀치` sprite punches on frames 0, 2, 4 and 6, flying from the
//! caster to the target. The green variant tints the sprite and overrides the
//! alpha ramp.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct RocketPunch {
    green: bool,
    lifetime: i32,
}

impl RocketPunch {
    pub fn new(green: bool, lifetime: i32) -> Self {
        Self { green, lifetime }
    }
}

impl PortedEffect for RocketPunch {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame != 0 && frame != 2 && frame != 4 && frame != 6 {
            return true;
        }

        let angles = Angles::shared();
        let target = runtime.target_offset;
        let position = runtime.position;

        // `Face(deltaPos.x, deltaPos.z)` against the caster position.
        let delta_x = target[0] - position[0];
        let delta_z = target[2] - position[2];
        let mut face_angle = if delta_x != 0.0 || delta_z != 0.0 {
            ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0
        } else {
            0.0
        };

        if face_angle >= 360.0 {
            face_angle -= 360.0;
        }

        let distance = (delta_z * delta_z + delta_x * delta_x).sqrt();
        let camera_yaw = runtime.camera_yaw;
        let mut angle = runtime.get_angle(position[0] - target[0], position[2] - target[2]) as i32;
        angle = (camera_yaw as i32 + angle) % 360;

        if angle < 0 {
            angle += 360;
        }

        let punch = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(punch);

        if self.green {
            state.render_flag = 1 | 4;
        }

        state.duration = 20;
        state.pattern |= 0x100;
        state.delta_pos2 = [0.0, -15.0, 0.0];

        // `longitude = -render.angle`, then the render angle is normalised.
        state.render_angle = face_angle;
        state.longitude = -face_angle;

        if state.render_angle < 0.0 {
            state.render_angle += 360.0;
        }

        state.speed = -(distance / state.duration as f32);
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.size = 1.0;
        state.anim_speed = 1;

        if frame == 0 {
            state.pattern |= 0x200;
        } else {
            state.alpha = (8 - frame) as f32 * 10.0;
        }

        state.sprite = Some("이팩트\\남_로켓트펀치.spr");
        state.action = Some("이팩트\\남_로켓트펀치.act");

        // Camera-relative quadrant picks the action row.
        state.action_index = if (180..270).contains(&angle) {
            4
        } else if (90..180).contains(&angle) {
            6
        } else if (0..90).contains(&angle) {
            0
        } else {
            2
        };

        if self.green {
            state.red = 105;
            state.green = 255;
            state.blue = 105;
            state.alpha = (5 - frame) as f32 * 40.0;
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
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 281);
        runtime.position = [0.0, 0.0, 0.0];
        runtime.target_offset = [0.0, 0.0, 10.0];
        runtime
    }

    #[test]
    fn punches_fire_on_even_frames_until_six() {
        let mut effect = RocketPunch::new(false, 100);
        let mut runtime = runtime();

        for _ in 0..7 {
            step(&mut effect, &mut runtime);
        }

        // Frames 0, 2, 4, 6.
        assert_eq!(runtime.primitives.len(), 4);

        let punch = &runtime.primitives[0].state;
        assert_eq!(punch.count, PrimitiveId::Particle3d);
        assert_eq!(punch.duration, 20);
        assert_eq!(punch.pattern & 0x100, 0x100);
        assert_eq!(punch.pattern & 0x200, 0x200, "first punch carries 0x200");
        assert_eq!(punch.delta_pos2, [0.0, -15.0, 0.0]);
        assert_eq!(punch.size, 1.0);
        assert_eq!(punch.anim_speed, 1);
        assert_eq!(punch.sprite, Some("이팩트\\남_로켓트펀치.spr"));
        assert_eq!(punch.action, Some("이팩트\\남_로켓트펀치.act"));
        assert!((punch.speed + 0.5).abs() < 1e-6, "distance 10 over 20 frames");
    }

    #[test]
    fn later_punches_fade_out() {
        let mut effect = RocketPunch::new(false, 100);
        let mut runtime = runtime();

        for _ in 0..3 {
            step(&mut effect, &mut runtime);
        }

        let second = &runtime.primitives[1].state;
        assert_eq!(second.pattern & 0x200, 0);
        assert_eq!(second.alpha, 60.0, "(8 - 2) * 10");
    }

    #[test]
    fn green_punch_is_tinted() {
        let mut effect = RocketPunch::new(true, 100);
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        let punch = &runtime.primitives[0].state;
        assert_eq!(punch.render_flag, 5);
        assert_eq!((punch.red, punch.green, punch.blue), (105, 255, 105));
        assert_eq!(punch.alpha, 200.0, "(5 - 0) * 40");
    }
}
