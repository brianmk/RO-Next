//! `PP_3DCIRCLE` - `CEffectPrim::Prim3DCircle`,
//! `EffectPrimitiveMotion.cpp:791-822`.
//!
//! The world-space ground ring. It is the workhorse of ground spells and buffs
//! (Magnum Break's two rings, Blessing's halo, Sanctuary, ...). Unlike the 2-D
//! circle it lives in world space: the matrix is rebuilt every frame from
//! `latitude` (X rotation) and `longitude` (appended Y rotation), and a forward
//! vector is integrated into `deltaPos`.

use super::super::angles::Angles;
use super::super::math::forward;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `Prim3DCircle`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    if state.change_point == state.state_cnt {
        state.radius_accel = state.change_accel;
        state.radius_speed = state.change_speed;
    }

    state.speed += state.accel;
    state.long_speed += state.long_accel;
    state.radius_speed += state.radius_accel;
    state.inner_speed += state.inner_accel;
    state.longitude += state.long_speed;
    state.radius += state.radius_speed;
    state.inner_size += state.inner_speed;

    state.transform.make_x_rotation(angles.radian(state.latitude as i32));
    state.transform.append_y_rotation(angles.radian(state.longitude as i32));

    state.speed3d = forward(state.speed, &state.transform.values);
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

    if (state.pattern & 1u32) != 0 || state.radius < state.inner_size {
        state.size = state.radius;
    } else {
        state.size = state.inner_size;
    }

    state.process_alpha(master);
    state.shift_segment();
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn magnum_ring() -> EffectPrimitiveState {
        EffectPrimitiveState {
            latitude: 90.0,
            radius_speed: 1.75,
            inner_size: 12.0,
            duration: 30,
            fade_out_cnt: 15,
            max_alpha: 254.0,
            alpha_speed: 254.0 / 15.0,
            ..Default::default()
        }
    }

    #[test]
    fn radius_rises_then_decelerates() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = magnum_ring();
        state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;

        let first_speed = state.radius_speed + state.radius_accel;
        on_process(&mut state, &angles, &master);

        assert!((state.radius_speed - first_speed).abs() < EPSILON);
        assert!((state.radius - first_speed).abs() < EPSILON);
        assert!(state.radius_accel < 0.0, "ring decelerates as it expands");
    }

    #[test]
    fn size_switches_from_radius_to_inner_size() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = magnum_ring();
        state.radius_accel = 0.0;

        on_process(&mut state, &angles, &master);
        assert_eq!(state.size, state.radius, "radius < innerSize");

        state.radius = 20.0;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.size, 12.0, "innerSize once radius passes it");

        state.pattern = 1;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.size, state.radius, "pattern bit 0 forces radius");
    }

    #[test]
    fn longitude_and_matrix_advance_together() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = magnum_ring();
        state.long_speed = 3.0;

        on_process(&mut state, &angles, &master);

        assert!((state.longitude - 3.0).abs() < EPSILON);
        assert!((state.transform.values[9] - 0.0).abs() < EPSILON);
    }

    #[test]
    fn segment_record_tracks_the_ring() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = magnum_ring();
        state.num_segment = 2;
        state.init_segment();

        on_process(&mut state, &angles, &master);

        // records[0] is the ring written this frame; records[1] is the copy of
        // the previous frame's record[0] (radius 0 before the first step).
        assert!((state.records[0].radius - state.radius).abs() < EPSILON);
        assert_eq!(state.records[1].radius, 0.0);
        assert_eq!(state.records[0].alpha, state.alpha);
    }
}
