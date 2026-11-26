//! prim 104 (`PP_HALFSPHERE`) - `EffectPrimitiveMotion.cpp:8550-8575`.
//!
//! The half-sphere shell of `CRagEffect::TexturedPulse` (`EF_HALFSPHERE`). One
//! live slot: its fan start angle is pinned to the caster's facing plus 180
//! every frame, its top vector is parked 90 degrees off that angle at a radius
//! of 5, and the alpha ramps by 5 up to 120 over the first 24 frames.
//!
//! `FollowMaster` (`EffectPrimitiveMotion.cpp:115-118`) sets
//! `render.position = master->render.position`; the port keeps that as the
//! first line, exactly like the other primitives.

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PP_HALFSPHERE`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    // `FollowMaster(*this);`
    state.position = master.position;

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];

        slot.process += 1;
        // `static_cast<std::int16_t>(master->state.parent->render.angle + 180.0f)`
        // truncates toward zero; the original only subtracts one full turn.
        slot.rot_start = (master.parent_angle + 180.0) as i16;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        // `rotStart + (rotStart < 270 ? 90 : -270)`, which stays in `[0, 360)`.
        let angle = slot.rot_start as i32 + if slot.rot_start < 270 { 90 } else { -270 };
        slot.vec_t_now[0] = angles.raw_cosine(angle.max(0) as usize) * 5.0;
        slot.vec_t_now[2] = angles.raw_sine(angle.max(0) as usize) * 5.0;

        if slot.process <= 50 && slot.alpha_b < 120 {
            slot.alpha_b += 5;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn pulse(master_angle: f32) -> (EffectPrimitiveState, MasterState) {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::HalfSphere,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 8.0,
            max_height: 8.0,
            full_display_angle: 180,
            ..Default::default()
        };

        let master = MasterState {
            position: [10.0, 20.0, 30.0],
            parent_angle: master_angle,
            ..Default::default()
        };

        (state, master)
    }

    #[test]
    fn position_follows_the_master() {
        let angles = Angles::new();
        let (mut state, master) = pulse(0.0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position, [10.0, 20.0, 30.0]);
    }

    #[test]
    fn rot_start_is_the_parent_facing_plus_180() {
        let angles = Angles::new();

        for (parent_angle, expected) in [(0.0, 180i16), (100.0, 280), (200.0, 20), (359.0, 179)] {
            let (mut state, master) = pulse(parent_angle);

            on_process(&mut state, &angles, &master);

            assert_eq!(state.slots[0].rot_start, expected, "parent angle {parent_angle}");
        }
    }

    #[test]
    fn top_vector_uses_the_shifted_angle_at_radius_five() {
        let angles = Angles::new();
        // rotStart 180 (< 270) selects +90 -> table angle 270.
        let (mut state, master) = pulse(0.0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].vec_t_now[0], angles.raw_cosine(270) * 5.0);
        assert_eq!(state.slots[0].vec_t_now[2], angles.raw_sine(270) * 5.0);
        // The y component is never written by this primitive.
        assert_eq!(state.slots[0].vec_t_now[1], 0.0);
    }

    #[test]
    fn the_270_boundary_takes_the_minus_270_branch() {
        let angles = Angles::new();
        // parent 90 -> rotStart 270, so the original uses -270 and lands on 0.
        let (mut state, master) = pulse(90.0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].rot_start, 270);
        assert_eq!(state.slots[0].vec_t_now[0], angles.raw_cosine(0) * 5.0);
        assert_eq!(state.slots[0].vec_t_now[2], angles.raw_sine(0) * 5.0);
    }

    #[test]
    fn alpha_ramps_by_five_until_120() {
        let angles = Angles::new();
        let (mut state, master) = pulse(0.0);

        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].alpha_b, 5);

        // The 24th frame reaches 120 (23 * 5 = 115 before it).
        for _ in 0..23 {
            on_process(&mut state, &angles, &master);
        }

        assert_eq!(state.slots[0].process, 24);
        assert_eq!(state.slots[0].alpha_b, 120);

        // `alphaB < 120` stops the ramp; `process` keeps counting.
        for _ in 0..30 {
            on_process(&mut state, &angles, &master);
        }

        assert_eq!(state.slots[0].process, 54);
        assert_eq!(state.slots[0].alpha_b, 120);
    }

    #[test]
    fn dead_slots_are_not_touched() {
        let angles = Angles::new();
        let (mut state, master) = pulse(0.0);

        on_process(&mut state, &angles, &master);

        for index in 1..4usize {
            assert_eq!(state.slots[index].process, 0);
            assert_eq!(state.slots[index].rot_start, 0);
            assert_eq!(state.slots[index].alpha_b, 0);
            assert_eq!(state.slots[index].vec_t_now, [0.0; 3]);
        }
    }
}
