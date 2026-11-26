//! `PP_3DCYLINDER` - `CEffectPrim::Prim3DCylinder`,
//! `EffectPrimitiveMotion.cpp:437-488`.
//!
//! A ground cylinder (Bowling Bash's blue arc, many AoE marks). It integrates
//! `speed` upwards through `Upward`, and the inner/outer radii and height.

use super::super::angles::Angles;
use super::super::math::upward;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `Prim3DCylinder`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    state.speed += state.accel;
    state.height_speed += state.height_accel;
    state.outer_speed += state.outer_accel;
    state.inner_speed += state.inner_accel;
    state.long_speed += state.long_accel;
    state.lati_speed += state.lati_accel;
    state.longitude += state.long_speed;
    state.latitude += state.lati_speed;

    state.transform.make_x_rotation(angles.radian(state.latitude as i32));
    state.transform.append_y_rotation(angles.radian(state.longitude as i32));

    state.speed3d = upward(state.speed, &state.transform.values);
    state.delta_pos = [
        state.delta_pos[0] + state.speed3d[0],
        state.delta_pos[1] + state.speed3d[1],
        state.delta_pos[2] + state.speed3d[2],
    ];

    state.position = [
        state.org_pos[0] + state.delta_pos[0],
        state.org_pos[1] + state.delta_pos[1],
        state.org_pos[2] + state.delta_pos[2],
    ];

    state.longitude += state.long_speed;
    state.outer_size += state.outer_speed;
    state.inner_size += state.inner_speed;
    state.height_size += state.height_speed;

    if state.height_size > state.max_height_size {
        state.height_size = state.max_height_size;
    }

    if (state.pattern & 1u32) != 0 && state.state_cnt == state.fade_out_cnt {
        state.outer_speed = -(state.outer_size / (state.duration - state.state_cnt + 1) as f32);
    }

    state.process_alpha(master);

    if state.state_cnt % state.anim_speed.max(1) == 0 {
        let total = state.total_texture;
        state.cur_motion += 1;

        if state.repeat_anim == 0 {
            if state.cur_motion == total {
                state.master_reached = false;
            }
        } else {
            state.cur_motion %= total.max(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radii_and_height_integrate() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = EffectPrimitiveState {
            duration: 20,
            fade_out_cnt: 10,
            alpha: 240.0,
            max_alpha: 240.0,
            outer_size: 8.0,
            outer_speed: 0.5,
            outer_accel: -0.03,
            inner_size: 3.0,
            inner_speed: 0.5,
            inner_accel: -0.03,
            height_size: 3.5,
            ..Default::default()
        };

        on_process(&mut state, &angles, &master);

        assert!((state.outer_size - (8.0 + 0.47)).abs() < 1e-4);
        assert!((state.inner_size - (3.0 + 0.47)).abs() < 1e-4);
        assert!((state.height_size - 3.5).abs() < 1e-6);
    }

    #[test]
    fn pattern_one_shrinks_the_outer_radius_at_fade_out() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = EffectPrimitiveState {
            pattern: 1,
            duration: 20,
            state_cnt: 10,
            fade_out_cnt: 10,
            outer_size: 10.0,
            alpha: 100.0,
            max_alpha: 100.0,
            ..Default::default()
        };

        on_process(&mut state, &angles, &master);

        assert!(state.outer_speed < 0.0);
    }
}
