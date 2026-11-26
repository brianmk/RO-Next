//! prim 7 (`PP_3DPARTICLEORBIT`) - `CEffectPrim::Prim3DParticleOrbit`,
//! `EffectPrimitiveMotion.cpp:2936-2985`.
//!
//! Orbiting particles: the position is rebuilt every frame from the master
//! origin plus the orbiting forward vector; radius/longitude/gravity each
//! integrate their speed and acceleration.

use super::super::angles::Angles;
use super::super::math::forward;
use super::{EffectPrimitiveState, MasterState};

/// One frame of the orbit primitive.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.org_pos = master.position;

    state.radius_speed += state.radius_accel;
    state.grav_speed += state.grav_accel;
    state.long_speed += state.long_accel;

    state.radius += state.radius_speed;
    state.longitude += state.long_speed;
    state.delta_pos2[1] += state.grav_speed;
    state.org_pos[0] += state.delta_pos2[0];
    state.org_pos[1] += state.delta_pos2[1];
    state.org_pos[2] += state.delta_pos2[2];
    state.roll += state.roll_speed;

    if state.state_cnt == state.change_point {
        state.grav_speed = state.change_speed;
        state.grav_accel = state.change_accel;
    }

    if state.ch_radi_speed != 0 {
        let mut index = state.color_cycle_index;
        let count = state.ch_point.len() as u32;

        if index < 0 || index as u32 >= count {
            state.color_cycle_index = 0;
            index = 0;
        }

        if state.state_cnt as f32 == state.ch_point[index as usize] {
            state.radius_speed = state.ch_val1[index as usize];
            state.radius_accel = state.ch_val2[index as usize];
            state.color_cycle_index = index + 1;
        }
    }

    state.transform.make_y_rotation(angles.radian(state.longitude as i32));

    state.delta_pos = forward(state.radius, &state.transform.values);

    state.position = [
        state.org_pos[0] + state.delta_pos[0],
        state.org_pos[1] + state.delta_pos[1],
        state.org_pos[2] + state.delta_pos[2],
    ];
    state.size += state.size_speed;

    state.latitude += state.lati_speed;
    state.transform.append_z_rotation(state.latitude);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn orbit() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::Orbit,
            duration: 100,
            radius: 3.0,
            radius_speed: 1.0,
            grav_speed: -1.2,
            grav_accel: -0.1,
            ..Default::default()
        }
    }

    #[test]
    fn orbit_integrates_radius_and_gravity() {
        let angles = Angles::new();
        let master = MasterState {
            position: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        let mut state = orbit();

        on_process(&mut state, &angles, &master);

        assert!((state.radius - 4.0).abs() < 1e-6);
        assert!((state.grav_speed + 1.3).abs() < 1e-6);
        assert!((state.delta_pos2[1] + 1.3).abs() < 1e-6);
        assert_eq!(state.org_pos[0], 10.0);
        assert!((state.org_pos[1] - (20.0 - 1.3)).abs() < 1e-6);
        assert!(state.size >= 0.0);
    }

    #[test]
    fn change_point_swaps_the_gravity_terms() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = orbit();
        state.change_point = 3;
        state.state_cnt = 3;
        state.change_speed = 5.0;
        state.change_accel = 0.5;

        on_process(&mut state, &angles, &master);

        // The values are assigned after this frame's integration.
        assert_eq!(state.grav_speed, 5.0);
        assert_eq!(state.grav_accel, 0.5);
    }
}
