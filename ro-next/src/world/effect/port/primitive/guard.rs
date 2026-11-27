//! prim 61 `PP_GUARD` - `CEffectPrim::PrimGuard`,
//! `EffectPrimitiveMotion.cpp:1344-1383`; dispatch at
//! `EffectPrimitiveMotion.cpp:8547-8548`.
//!
//! The black-maximize sphere cage of `EF_GUARD2` (496), spawned by
//! `CRagEffect::BlackMaximize` (`RagEffect.cpp:3453-3480`).
//!
//! Each live slot turns to face its parent (`parent->render.angle + 270`),
//! spins `+10` degrees per frame when `flag1[0] == 1` (every `BlackMaximize`
//! spawn), adds `20` alpha per frame for the first nine frames while the alpha
//! is below `100` (the original does **not** clamp after the add), and from
//! frame 27 on lifts `height[0]` by `0.1` while draining six alpha per frame.
//!
//! Quirks kept: the first frame (`process <= 1`) always re-arms the angle
//! without the `flag1[0]` spin test, and the `>= 360` subtraction runs once per
//! frame even after the `+10` spin.
//!
//! The renderer is `RenderSphereCage` (`EffectPrimitiveRendering.cpp:4328`),
//! dispatched by the `case PP_GUARD` arm at
//! `EffectPrimitiveRendering.cpp:391-392` - see
//! [`super::super::render_slots::render_sphere_cage`].

use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimGuard` (`EffectPrimitiveMotion.cpp:1344-1383`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    // `this->render.position = this->state.master->render.position;`
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 1 {
            // `slot.rotStart = (int16_t)(parent->render.angle + 270.0f);`
            slot.rot_start = (master.parent_angle + 270.0) as i16;
        } else if slot.flag1[0] == 1 {
            slot.rot_start += 10;
        } else {
            slot.rot_start = (master.parent_angle + 270.0) as i16;
        }

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.process < 10 {
            if slot.alpha_b < 100 {
                slot.alpha_b += 20;
            }
        } else if slot.process > 26 {
            slot.height[0] += 0.1;

            if slot.alpha_b > 0 {
                slot.alpha_b -= 6;
            }

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn cage() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Guard,
            duration: 200,
            size: 11.0,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].max_height = 10.0;
        state.slots[0].distance = 1.0;
        state.slots[0].alpha_b = 0;
        state.slots[0].flag1[0] = 1;

        state
    }

    #[test]
    fn guard_is_primitive_61() {
        assert_eq!(PrimitiveId::Guard.id(), 61);
    }

    #[test]
    fn the_first_frame_arms_the_parent_angle_and_ramps_alpha() {
        let mut state = cage();
        let master = MasterState {
            parent_angle: 45.0,
            ..Default::default()
        };

        on_process(&mut state, &master);

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        assert_eq!(slot.rot_start, 315);
        assert_eq!(slot.alpha_b, 20);
        assert_eq!(slot.height[0], 0.0);
    }

    #[test]
    fn the_guard_flag_spins_the_cage_ten_degrees() {
        let mut state = cage();
        state.slots[0].process = 1;
        state.slots[0].rot_start = 350;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 2);
        assert_eq!(state.slots[0].rot_start, 0, "350 + 10 wraps through 360");
    }

    #[test]
    fn a_non_spinning_slot_re_arms_the_parent_angle() {
        let mut state = cage();
        state.slots[0].process = 1;
        state.slots[0].rot_start = 10;
        state.slots[0].flag1[0] = 0;

        on_process(&mut state, &MasterState {
            parent_angle: 90.0,
            ..Default::default()
        });

        assert_eq!(state.slots[0].rot_start, 0, "90 + 270 wraps through 360");
    }

    #[test]
    fn the_alpha_ramp_only_runs_while_below_one_hundred() {
        let mut state = cage();
        state.slots[0].process = 4;
        state.slots[0].alpha_b = 99;

        on_process(&mut state, &MasterState::default());

        // The original adds 20 behind a `< 100` test and does not clamp.
        assert_eq!(state.slots[0].alpha_b, 119);

        state.slots[0].process = 6;
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 119);
    }

    #[test]
    fn frames_after_twenty_six_lift_and_drain() {
        let mut state = cage();
        state.slots[0].process = 26;
        state.slots[0].alpha_b = 40;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 27);
        assert!((state.slots[0].height[0] - 0.1).abs() < 1e-6);
        assert_eq!(state.slots[0].alpha_b, 34);
    }

    #[test]
    fn the_drain_clamps_at_zero() {
        let mut state = cage();
        state.slots[0].process = 26;
        state.slots[0].alpha_b = 3;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 0);
    }
}
