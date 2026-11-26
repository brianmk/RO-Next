//! `PP_3DPARTICLE` - `CEffectPrim::Prim3DParticle`,
//! `EffectPrimitiveMotion.cpp:2488-2577`, plus `AnimMotion` (`:576-601`).
//!
//! The general 3-D sprite particle (blessing sparks, buff motes, projectile
//! trails). `AnimMotion` advances the `.act` frame; the port takes the action's
//! frame count from [`EffectPrimitiveState::action_frame_count`], which the
//! asset layer fills when it attaches a sprite.

use super::super::angles::Angles;
use super::super::math::forward;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `Prim3DParticle`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    let frame = state.state_cnt;

    if frame == state.change_point {
        state.speed3d = state.change_speed3d;
        state.delta_pos_accel = state.change_accel3d;
    }

    state.speed += state.accel;
    state.size_speed += state.size_accel;

    if (state.pattern & 0x400u32) != 0 {
        state.speed3d[0] += state.delta_pos_accel[0];
        state.speed3d[1] += state.delta_pos_accel[1];
        state.speed3d[2] += state.delta_pos_accel[2];

        state.delta_pos[0] += state.speed3d[0];
        state.delta_pos[1] += state.speed3d[1];
        state.delta_pos[2] += state.speed3d[2];
        state.roll += state.roll_speed;
        state.size += state.size_speed;
    } else if (state.pattern & 1u32) != 0 || frame > state.fade_in_cnt {
        state.speed3d = forward(state.speed, &state.transform.values);

        state.delta_pos[0] += state.speed3d[0];
        state.delta_pos[1] += state.speed3d[1];
        state.delta_pos[2] += state.speed3d[2];
        state.roll += state.roll_speed;
        state.size += state.size_speed;
    }

    if (state.pattern & 2u32) != 0 {
        if state.state_cnt % state.ch_cnt.max(1) as i32 == 0 {
            state.ch_cnt = (rng.clamped_random(8) + 12) as u32;
            let mut x = (rng.clamped_random(5) + 5) as f32 / 100.0;
            let mut z = (rng.clamped_random(5) + 5) as f32 / 100.0;

            if rng.clamped_random(2) == 0 {
                x *= -1.0;
            }

            if rng.clamped_random(2) == 0 {
                z *= -1.0;
            }

            state.delta_pos3 = [x, state.delta_pos3[1], z];
        }

        state.delta_pos[0] += state.delta_pos3[0];
    }

    if (state.pattern & 4u32) != 0 && state.state_cnt % state.ch_cnt.max(1) as i32 == 0 {
        state.speed = (rng.clamped_random(35) + 15) as f32 / 100.0;
        state.longitude = (state.longitude - 40.0) + rng.clamped_random(80) as f32;
        state.latitude = (state.latitude - 40.0) + rng.clamped_random(80) as f32;
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.transform.append_x_rotation(angles.radian(state.latitude as i32));
        state.ch_cnt = (rng.clamped_random(10) + 5) as u32;
    }

    if (state.pattern & 8u32) != 0 && state.state_cnt % state.ch_cnt.max(1) as i32 == 0 {
        state.longitude = (state.longitude - 5.0) + rng.clamped_random(10) as f32;
        state.latitude = (state.latitude - 5.0) + rng.clamped_random(10) as f32;
        state.transform.make_x_rotation(angles.radian(state.latitude as i32));
        state.transform.append_y_rotation(angles.radian(state.longitude as i32));
        state.ch_cnt = (rng.clamped_random(15) + 5) as u32;
    }

    state.position = [
        state.org_pos[0] + state.delta_pos[0],
        state.org_pos[1] + state.delta_pos[1],
        state.org_pos[2] + state.delta_pos[2],
    ];

    if (state.pattern & 0x100u32) != 0 {
        let distance = (master.master_delta_pos[0] * master.master_delta_pos[0]
            + master.master_delta_pos[2] * master.master_delta_pos[2])
            .sqrt();

        if distance <= 3.0 {
            return false;
        }
    }

    if (state.pattern & 0x20u32) != 0 {
        state.roll = state.render_angle - master.camera_yaw;
    }

    state.process_alpha(master);
    state.shift_segment();

    anim_motion(state)
}

/// `CEffectPrim::AnimMotion` - `EffectPrimitiveMotion.cpp:576-601`.
fn anim_motion(state: &mut EffectPrimitiveState) -> bool {
    if state.state_cnt % state.anim_speed.max(1) != 0 {
        return true;
    }

    let frame_count = state.action_frame_count.max(1);

    state.cur_motion += 1;

    if state.repeat_anim == 0 {
        if (state.pattern & 0x800u32) != 0 {
            if state.cur_motion == frame_count {
                state.cur_motion -= 1;
            }
        } else if state.cur_motion == frame_count {
            return false;
        }
    } else {
        state.cur_motion %= frame_count;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn particle() -> EffectPrimitiveState {
        EffectPrimitiveState {
            duration: 80,
            pattern: 1,
            speed: 2.0,
            anim_speed: 1,
            action_frame_count: 3,
            repeat_anim: 1,
            max_alpha: 254.0,
            fade_out_cnt: 80,
            ..Default::default()
        }
    }

    #[test]
    fn moves_forward_when_pattern_one() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = particle();

        on_process(&mut state, &angles, &master, &mut rng);

        assert!(state.position[2] > 0.0);
        assert!(state.delta_pos[2] > 0.0);
    }

    #[test]
    fn animation_wraps_with_repeat() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = particle();

        let mut seen = Vec::new();
        for _ in 0..6 {
            assert!(on_process(&mut state, &angles, &master, &mut rng));
            seen.push(state.cur_motion);
        }

        assert_eq!(seen, vec![1, 2, 0, 1, 2, 0]);
    }

    #[test]
    fn animation_stops_without_repeat() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = particle();
        state.repeat_anim = 0;

        assert!(on_process(&mut state, &angles, &master, &mut rng));
        assert!(on_process(&mut state, &angles, &master, &mut rng));
        assert!(!on_process(&mut state, &angles, &master, &mut rng));
    }

    #[test]
    fn pattern_two_reaims_with_the_rng() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(9);
        let mut state = particle();
        state.pattern = 2;
        state.ch_cnt = 1;

        on_process(&mut state, &angles, &master, &mut rng);

        assert!(state.ch_cnt >= 12);
        assert!(state.delta_pos3[0] != 0.0);
        assert!(state.delta_pos3[2] != 0.0);
    }
}
