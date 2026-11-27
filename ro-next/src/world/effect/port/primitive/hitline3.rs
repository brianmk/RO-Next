//! Prim 100 `PP_HITLINE3` - the white line/spark primitive of
//! `EF_HITLINE4/5/6` and the `WhiteSpark*` bodies.
//!
//! Official motion: `EffectPrimitiveMotion.cpp:8458-8483`; renderer:
//! `RenderHitLine3` (`EffectPrimitiveRendering.cpp:1872-1962`, dispatch
//! `:440-442`).
//!
//! Unlike `PP_HITLINE` (prim 70), which shifts a 21-sample height history and
//! wraps at 180, `PP_HITLINE3` clamps `height[0]` at 180, shifts the history by
//! the *slot's* `vecTNow.y` (a float sample count, 3/4/5 depending on the
//! body's `kind`) and re-anchors `vecBNow` on the master every frame. The
//! `vecTNow.y` loop bound is exactly what `RenderHitLine3` iterates, so it is
//! both a sample count and a top-ring `y` for the `RenderPentaRing` family;
//! prim 100 only ever uses it as the count.

use super::{EffectPrimitiveState, MasterState};

/// One frame of the official `case PP_HITLINE3`
/// (`EffectPrimitiveMotion.cpp:8458-8483`).
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) -> bool {
    for index in 0..4usize {
        let slot = &mut state.slots[index];

        // `if (slots[index].life == 1) { ... }`
        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process > 0 {
            if slot.process > 35 {
                slot.alpha_b -= 5;
            }

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }

            slot.height[0] += 6.0;

            if slot.height[0] > 180.0 {
                slot.height[0] = 180.0;
            }

            // `for (int value = (int)slots[index].vecTNow.y; value >= 1;
            //  value--) { height[value] = height[value - 1]; ... }`
            let mut value = slot.vec_t_now[1] as i32;

            while value >= 1 {
                let target = value as usize;
                slot.height[target] = slot.height[target - 1];
                slot.flag1[target] = slot.flag1[target - 1];
                value -= 1;
            }

            slot.vec_b_now = master.position;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveState, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::HitLine3,
            duration: 200,
            ..Default::default()
        };

        state.slots[0].life = 1;
        state.slots[0].vec_t_now[1] = 4.0;
        state.slots[0].height[0] = 0.0;
        state.slots[0].alpha_b = 250;
        state
    }

    #[test]
    fn shifts_the_height_history_by_the_slot_sample_count() {
        let mut state = state();
        state.slots[0].height[0] = 10.0;
        state.slots[0].height[1] = 20.0;
        state.slots[0].height[2] = 30.0;
        state.slots[0].height[3] = 40.0;

        assert!(on_process(&mut state, &MasterState::default()));

        // `height[0] += 6` happens *before* the shift, so the new value is
        // copied into `height[1]`.
        assert_eq!(&state.slots[0].height[0..5], &[16.0, 16.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn clamps_the_leading_height_at_180() {
        let mut state = state();
        state.slots[0].height[0] = 179.0;

        for _ in 0..2 {
            assert!(on_process(&mut state, &MasterState::default()));
        }

        assert_eq!(state.slots[0].height[0], 180.0);
        assert_eq!(state.slots[0].height[1], 180.0);
    }

    #[test]
    fn fades_after_frame_thirty_five_and_reanchors_on_the_master() {
        let master = MasterState {
            position: [7.0, 8.0, 9.0],
            ..Default::default()
        };
        let mut state = state();
        state.slots[0].process = 35;

        assert!(on_process(&mut state, &master));

        assert_eq!(state.slots[0].process, 36);
        assert_eq!(state.slots[0].alpha_b, 245);
        assert_eq!(state.slots[0].vec_b_now, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn a_dead_slot_is_untouched() {
        let mut state = state();
        state.slots[0].life = 0;

        assert!(on_process(&mut state, &MasterState::default()));

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].height[0], 0.0);
    }
}
