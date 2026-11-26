//! `PP_3DPARTICLESPLINE` - `CEffectPrim::Prim3DParticleSpline`,
//! `EffectPrimitiveMotion.cpp:2990-3014`, dispatch `:8183-8185`.
//!
//! The multi-segment sprite trail (Water Ball 2, Blood Drain, Energy Drain
//! 2/3, Red Particle Spray, Soul Strike). The motion keeps four vectors:
//! `speed3d` (forward, reduced by the gravity accumulator), the quaternion
//! lift `deltaPos3` around `rotAxis` by `latitude`, and `deltaPos`, which is
//! pushed *backwards* by `speed3d` every frame - the sprites trail behind the
//! emitter instead of flying away from it.
//!
//! `ShiftSegment` records the current position/size/colour at record 0 and
//! shifts the older ones down; the shared `RenderSegmentSprites2` pass draws
//! one `.spr` per record, so the port reuses `sprite_pass` unchanged.
//!
//! Quirk kept: the dispatch calls the body as a statement and **ignores its
//! `bool`** (unlike `PP_3DPARTICLE`, whose dispatch returns `false` when
//! `AnimMotion` reports the end of the action). `AnimMotion` still advances
//! `curMotion`, but the primitive only retires at `stateCnt > duration`.

use super::super::angles::Angles;
use super::super::math::{forward, lift};
use super::particle3d::anim_motion;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `Prim3DParticleSpline`.
pub(super) fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    state.speed += state.accel;
    state.latitude += state.lati_speed;
    state.lati_speed += state.lati_accel;
    state.grav_speed += state.grav_accel;

    state.speed3d = forward(state.speed - state.grav_speed, &state.transform.values);
    state
        .rot_transform
        .make_transform(&state.rot_axis, angles.radian(state.roll as i32));
    state.delta_pos3 = lift(state.latitude, &state.rot_transform.values);

    state.size += state.size_speed;
    state.delta_pos[0] -= state.speed3d[0];
    state.delta_pos[1] -= state.speed3d[1];
    state.delta_pos[2] -= state.speed3d[2];
    state.position = [
        state.org_pos[0] + state.delta_pos[0] + state.delta_pos3[0],
        state.org_pos[1] + state.delta_pos[1] + state.delta_pos3[1],
        state.org_pos[2] + state.delta_pos[2] + state.delta_pos3[2],
    ];

    state.process_alpha(master);
    state.shift_segment();

    // The original returns `false` here when the ACT action runs out, but its
    // caller (`:8183-8185`) discards the value.
    let _ = anim_motion(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::math::matrix_mult;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn spline() -> EffectPrimitiveState {
        let angles = Angles::new();
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::ParticleSpline,
            duration: 60,
            delta_pos2: [0.0, -10.0, 0.0],
            longitude: 0.0,
            speed: 1.0,
            grav_speed: 2.0,
            grav_accel: -1.0 / 15.0,
            lati_speed: -2.0,
            lati_accel: 1.0 / 15.0,
            roll: 90.0,
            size: 1.7,
            num_segment: 7,
            fade_out_cnt: 60,
            ..Default::default()
        };

        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos3 = matrix_mult([0.0, 0.0, 1.0], &state.transform.values);
        state.rot_axis = state.delta_pos3;
        state.init_segment();

        state
    }

    #[test]
    fn spline_id_is_eight() {
        assert_eq!(PrimitiveId::ParticleSpline.id(), 8);
    }

    #[test]
    fn first_step_trails_backwards_and_lifts_by_latitude() {
        let angles = Angles::new();
        let master = MasterState {
            position: [5.0, 6.0, 7.0],
            ..Default::default()
        };
        let mut state = spline();

        on_process(&mut state, &angles, &master);

        assert_eq!(state.org_pos, [5.0, -4.0, 7.0]);
        // `gravSpeed` is advanced by `gravAccel` before the forward step:
        // speed - gravSpeed = 1 - (2 - 1/15), then deltaPos -= speed3d.
        let expected = 1.0 - (2.0 - 1.0 / 15.0);
        assert!((state.speed3d[2] - expected).abs() < 1e-6);
        assert!((state.delta_pos[2] + expected).abs() < 1e-6);
        // latitude starts at -2 and the lift rotates around rotAxis (90 deg).
        assert!(state.latitude < 0.0);
        assert_eq!(state.position[0], state.org_pos[0] + state.delta_pos[0] + state.delta_pos3[0]);
    }

    #[test]
    fn anim_motion_end_does_not_retire_the_primitive() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = spline();
        state.repeat_anim = 0;
        state.anim_speed = 1;
        state.action_frame_count = 1;

        // `AnimMotion` returns false on the first step (curMotion == frame
        // count without repeat), but the dispatch ignores it and the next
        // step still advances the frame.
        on_process(&mut state, &angles, &master);
        on_process(&mut state, &angles, &master);

        assert_eq!(state.cur_motion, 2);
    }
}
