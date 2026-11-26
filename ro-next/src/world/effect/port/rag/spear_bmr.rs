//! `CRagEffect::SpearBmr` - `RagEffect.cpp:1091-1131`.
//!
//! `EF_SPEARBMR`: four `창` spear sprites on frames 0/4/8/12 flying from the
//! caster to the target, fading `180 → 130 → 80`, with the 8-direction action
//! row from `CGameObject::Get8Dir2(angle, 1)` (`GameObject.cpp:188-213`).

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CGameObject::Get8Dir2(rot, flag)` for `flag != 0`.
fn get_8_dir2(camera_yaw: f32, rot: f32) -> i32 {
    let mut relative = -camera_yaw - rot;

    while relative < 0.0 {
        relative += 360.0;
    }

    while relative >= 360.0 {
        relative -= 360.0;
    }

    if relative < 22.5 || relative > 337.5 {
        return 0;
    }

    ((relative as f64 + 22.5) / 45.0) as i32
}

#[derive(Debug)]
pub struct SpearBmr {
    lifetime: i32,
}

impl SpearBmr {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SpearBmr {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame != 0 && frame != 4 && frame != 8 && frame != 12 {
            return true;
        }

        let angles = Angles::shared();
        let target = runtime.target_offset;
        let position = runtime.position;
        let camera_yaw = runtime.camera_yaw;
        let parent_angle = runtime.parent_angle;

        runtime.play_sound("effect\\EF_FireBall.wav");

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
        let direction = get_8_dir2(camera_yaw, parent_angle);

        let spear = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spear);

        state.duration = 15;
        state.pattern |= 0x120;
        state.delta_pos2 = [0.0, -15.0, 0.0];
        state.longitude = -face_angle;

        // The render angle is mirrored for the spear sprite, then normalised.
        let mut render_angle = 180.0 - face_angle;

        if render_angle < 0.0 {
            render_angle += 360.0;
        }

        state.render_angle = render_angle;
        state.speed = -(distance / state.duration as f32);
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.size = 1.0;
        state.anim_speed = 1;

        match frame {
            4 => state.alpha = 180.0,
            8 => state.alpha = 130.0,
            12 => state.alpha = 80.0,
            _ => {}
        }

        state.action_index = direction;
        state.sprite = Some("이팩트\\창.spr");
        state.action = Some("이팩트\\창.act");
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn four_spears_fly_and_fade() {
        let mut effect = SpearBmr::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 401);
        runtime.target_offset = [0.0, 0.0, 15.0];

        for _ in 0..13 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 4);
        assert_eq!(runtime.sounds.len(), 4);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::Particle3d);
        assert_eq!(first.duration, 15);
        assert_eq!(first.pattern & 0x120, 0x120);
        assert_eq!(first.delta_pos2, [0.0, -15.0, 0.0]);
        assert_eq!(first.size, 1.0);
        assert_eq!(first.anim_speed, 1);
        assert_eq!(first.sprite, Some("이팩트\\창.spr"));
        assert_eq!(first.action, Some("이팩트\\창.act"));
        assert!((first.speed + 1.0).abs() < 1e-6, "distance 15 over 15 frames");

        assert_eq!(runtime.primitives[1].state.alpha, 180.0);
        assert_eq!(runtime.primitives[2].state.alpha, 130.0);
        assert_eq!(runtime.primitives[3].state.alpha, 80.0);
    }

    #[test]
    fn eight_direction_mapping_wraps() {
        assert_eq!(get_8_dir2(0.0, 0.0), 0);
        // rot 270 -> relative 90 -> (90 + 22.5) / 45 = 2
        assert_eq!(get_8_dir2(0.0, 270.0), 2);
        // rot 350 -> relative 10 -> inside the front sector
        assert_eq!(get_8_dir2(0.0, 350.0), 0);
        // rot 320 -> relative 40 -> 1
        assert_eq!(get_8_dir2(0.0, 320.0), 1);
    }
}
