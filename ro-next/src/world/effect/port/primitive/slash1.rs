//! prim 75 (`PP_SLASH1`) - `EffectPrimitiveMotion.cpp:9213-9258`
//! (`case PP_SLASH1:`).
//!
//! The four slots are slash blades rooted at the master's position. Each live
//! slot rises by one unit per frame (`height[0]` is the blade radius) and, once
//! it passes 10 units, fades out by 2 alpha until it dies:
//!
//! ```cpp
//! ++slot.process;
//! slot.height[0] += 1.0f;
//! if (slot.height[0] > 10.0f) {
//!   slot.alphaB -= 2;
//!   if (slot.alphaB < 0) { slot.alphaB = 0; slot.life = 0; }
//! }
//! ```
//!
//! `flag1[0]` picks the two growth variants: `1` reaches out by 1 unit per
//! frame with a `maxHeight` cap of 2 and a 5-frame alpha ramp, anything else by
//! 2 units per frame with a cap of 4 and a 7-frame ramp. The fade test does not
//! short-circuit the growth, so a slot that just died still advances one last
//! time - the original's `continue` only covers dead *slots*.
//!
//! The renderer is
//! [`render_slash_1`](super::super::render_slots::render_slash_1)
//! (`EffectPrimitiveRendering.cpp:5402-5468`, dispatch `:527-528`).
//!
//! Bodies that spawn it: `CRagEffect::JobLvUp50_2` (`EF_KAIZEL`, 590,
//! `RagEffect.cpp:2568-2589`) and `CRagEffect::RunStop` (`EF_STOPEFFECT`, 444,
//! `:2593-2617`) - see [`super::super::rag::job_lv_up`] and
//! [`super::super::rag::run_stop`]. The other two spawners are still unported:
//! `CRagEffect::JobLvUp50` (`:2648-2678`) and the two `SuperAngel` ring bursts
//! (`:12664`, `:12684`).

use super::{EffectPrimitiveState, MasterState};

/// One frame of `PP_SLASH1`. The motion always returns `true`; the primitive
/// retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) -> bool {
    // `const vector3d& parent = this->state.master->render.position;`
    // `this->render.position = parent;`
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;
        slot.height[0] += 1.0;

        if slot.height[0] > 10.0 {
            slot.alpha_b -= 2;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
                slot.life = 0;
            }
        }

        if slot.flag1[0] == 1 {
            slot.distance += 1.0;

            if slot.max_height < 2.0 {
                slot.max_height += 0.2;
            }

            if slot.process <= 5 {
                slot.alpha_b += 5;
            }
        } else {
            slot.distance += 2.0;

            if slot.max_height < 4.0 {
                slot.max_height += 0.4;
            }

            if slot.process <= 7 {
                slot.alpha_b += 5;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    /// The original accumulates `maxHeight += 0.2f`/`+= 0.4f` frame by frame,
    /// so the assertion has to accumulate the same way to match bit for bit.
    fn accumulate(step: f32, times: usize) -> f32 {
        let mut value = 0.0f32;

        for _ in 0..times {
            value += step;
        }

        value
    }

    fn state(flag: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Slash1,
            duration: 200,
            ..Default::default()
        };

        for slot in &mut state.slots {
            slot.life = 1;
            slot.process = 0;
            slot.distance = 2.0;
            slot.height[0] = 1.0;
            slot.max_height = 0.0;
            slot.alpha_b = 0;
            slot.flag1[0] = flag;
        }

        state
    }

    #[test]
    fn primitive_id_is_75() {
        assert_eq!(PrimitiveId::Slash1.id(), 75);
    }

    #[test]
    fn the_blades_follow_the_master_and_rise_by_one_unit() {
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        let mut state = state(0);

        on_process(&mut state, &master);

        assert_eq!(state.position, [1.0, 2.0, 3.0]);

        for slot in &state.slots {
            assert_eq!(slot.process, 1);
            assert_eq!(slot.height[0], 2.0);
        }
    }

    #[test]
    fn flag_zero_grows_by_two_and_caps_height_at_four() {
        let mut state = state(0);
        let master = MasterState::default();

        for frame in 1..=7 {
            on_process(&mut state, &master);

            assert_eq!(state.slots[0].distance, 2.0 + 2.0 * frame as f32);
            assert_eq!(state.slots[0].alpha_b, (5 * frame) as i16);
            assert_eq!(state.slots[0].max_height, accumulate(0.4, frame));
        }

        // Frames 8 and 9 keep the 35 alpha; the 10th frame crosses the
        // `height[0] > 10` threshold (1 + 10) and also reaches the height cap.
        for _ in 8..=9 {
            on_process(&mut state, &master);
        }

        assert_eq!(state.slots[0].alpha_b, 35);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].height[0], 11.0);
        assert_eq!(state.slots[0].max_height, accumulate(0.4, 10));
        assert_eq!(state.slots[0].alpha_b, 33, "the fade starts on the crossing frame");

        // 33 alpha at -2 per frame dies on the 17th fade frame (process 27).
        for _ in 0..17 {
            on_process(&mut state, &master);
        }

        assert_eq!(state.slots[0].process, 27);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn flag_one_grows_by_one_and_caps_height_at_two() {
        let mut state = state(1);
        let master = MasterState::default();

        for frame in 1..=5 {
            on_process(&mut state, &master);

            assert_eq!(state.slots[0].distance, 2.0 + frame as f32);
            assert_eq!(state.slots[0].alpha_b, (5 * frame) as i16);
            assert_eq!(state.slots[0].max_height, accumulate(0.2, frame));
        }

        for _ in 6..=10 {
            on_process(&mut state, &master);
        }

        assert_eq!(state.slots[0].max_height, accumulate(0.2, 10));
        // The 10th frame is the first fade frame (height[0] = 11): 25 - 2.
        assert_eq!(state.slots[0].alpha_b, 23);
    }

    #[test]
    fn the_blade_fades_out_past_ten_units_and_dies_at_zero_alpha() {
        // Start the blade at 10 units so the next frame crosses the threshold.
        let mut state = state(0);
        state.slots[1].life = 1;
        state.slots[1].height[0] = 10.0;
        state.slots[1].alpha_b = 0;
        state.slots[1].flag1[0] = 0;

        let master = MasterState::default();
        on_process(&mut state, &master);

        // height[0] = 11 -> alpha 0 - 2 = -2 -> clamp 0 and the slot dies, but
        // the growth branch still runs on this frame: distance +2, maxHeight
        // +0.4 and the `process <= 7` ramp adds 5 alpha back to the dead slot.
        assert_eq!(state.slots[1].height[0], 11.0);
        assert_eq!(state.slots[1].alpha_b, 5);
        assert_eq!(state.slots[1].life, 0);
        assert_eq!(state.slots[1].distance, 4.0, "growth is not skipped by the death");
        assert_eq!(state.slots[1].max_height, accumulate(0.4, 1));
        assert_eq!(state.slots[1].process, 1);

        // The next frame skips the dead slot entirely.
        on_process(&mut state, &master);
        assert_eq!(state.slots[1].height[0], 11.0);
        assert_eq!(state.slots[1].distance, 4.0);
        assert_eq!(state.slots[1].process, 1);
    }

    #[test]
    fn dead_slots_are_untouched() {
        let mut state = state(0);

        for slot in &mut state.slots {
            slot.life = 0;
        }

        on_process(&mut state, &MasterState::default());

        for slot in &state.slots {
            assert_eq!(slot.process, 0);
            assert_eq!(slot.height[0], 1.0);
            assert_eq!(slot.distance, 2.0);
        }
    }
}
