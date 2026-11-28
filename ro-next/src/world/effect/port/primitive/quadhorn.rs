//! `PP_3DQUADHORN` - `CEffectPrim::AdvanceEffectPrimitive`,
//! `EffectPrimitiveMotion.cpp:543-583`. Used by GrimTooth, Ice Wall and the
//! stone-shard effects.
//!
//! The primitive integrates a forward speed through the matrix, applies the
//! `changePoint` overrides, and stops moving when `pattern & 1` is set and the
//! frame reaches `chCnt`.

use super::super::angles::Angles;
use super::super::math::forward;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `AdvanceEffectPrimitive`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    let _ = angles;

    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    if state.state_cnt == state.change_point {
        state.speed = state.change_speed;
        state.accel = state.change_accel;
    }

    state.speed = state.accel + state.speed;

    state.speed3d = forward(state.speed, &state.transform.values);
    state.delta_pos = [
        state.delta_pos[0] + state.speed3d[0],
        state.delta_pos[1] + state.speed3d[1],
        state.delta_pos[2] + state.speed3d[2],
    ];
    state.position = [
        state.delta_pos[0] + state.org_pos[0],
        state.delta_pos[1] + state.org_pos[1],
        state.delta_pos[2] + state.org_pos[2],
    ];

    if (state.pattern & 1u32) != 0 && state.state_cnt == state.ch_cnt as i32 {
        state.accel = 0.0;
        state.speed = 0.0;
    }

    if (state.pattern & 2u32) != 0 {
        if state.state_cnt == state.duration - 30 {
            state.speed = state.change_speed3d[1];
            state.accel = state.change_accel3d[1];
        }
    } else {
        state.process_alpha(master);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::math::Matrix;

    const EPSILON: f32 = 1e-4;

    fn shard() -> EffectPrimitiveState {
        EffectPrimitiveState {
            duration: 40,
            pattern: 1,
            ch_cnt: 6,
            speed: 3.0,
            accel: -(3.0f32 / 40.0) * 2.0,
            height_size: 10.0,
            size: 0.6,
            fade_out_cnt: 30,
            ..Default::default()
        }
    }

    #[test]
    fn moves_forward_and_stops_at_change_count() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = shard();
        state.delta_pos2 = [0.0, 20.0, 0.0];

        on_process(&mut state, &angles, &master);

        // Forward at identity matrix is +z, so only z changes.
        assert!(state.position[2] > 0.0);
        assert!((state.position[1] - 20.0).abs() < EPSILON, "anchored at deltaPos2.y");
        assert!((state.speed3d[2] - state.speed).abs() < EPSILON);

        // At frame 6 the pattern-1 stop zeroes the speed.
        state.state_cnt = 6;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.speed, 0.0);
        assert_eq!(state.accel, 0.0);
    }

    #[test]
    fn rotated_matrix_changes_the_direction() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = shard();
        state.pattern = 0;
        state.transform = Matrix::identity();
        state.transform.make_x_rotation(std::f32::consts::FRAC_PI_2);

        on_process(&mut state, &angles, &master);

        // X rotation by 90 deg sends +z to -y.
        assert!(state.position[1] < 0.0);
        assert!(state.position[2].abs() < EPSILON);
    }

    #[test]
    fn change_point_applies_speed_and_accel() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = shard();
        state.change_point = 0;
        state.change_speed = 7.0;
        state.change_accel = 0.5;

        on_process(&mut state, &angles, &master);

        assert!((state.speed - 7.5).abs() < EPSILON);
    }
}

/// One frame of `PP_3DQUADHORN2` - `EffectPrimitiveMotion.cpp:8189-8226`.
///
/// The Gravitation shards. Every six frames the speed alternates between the
/// configured drift and the upward `1.18` burst; at frame 550 the motion
/// settles back to the configured drift. The shape is the same pyramid as
/// prim 9, so the renderer is shared.
pub fn on_process2(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    let frame = state.state_cnt;

    if frame < 550 {
        if frame % 6 == 2 {
            state.speed = state.change_speed;
            state.accel = state.change_accel;
        } else if frame % 6 == 5 {
            state.speed = 1.18;
            state.accel = 0.01;
        }
    } else if frame == 550 {
        state.speed = state.change_speed;
        state.accel = state.change_accel;
    }

    state.speed += state.accel;
    state.speed3d = super::super::math::forward(state.speed, &state.transform.values);
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

    state.process_alpha(master);
}
