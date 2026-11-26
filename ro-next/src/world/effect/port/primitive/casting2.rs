//! prim 24 (`PP_3DCASTING_2`) - `EffectPrimitiveMotion.cpp:8317-8362`
//! (`case PP_3DCASTING_2:`).
//!
//! Four stacked rings: every live slot spins (`rotStart` +1 for slots 0/2, +2
//! for slots 1/3), fades its `alphaB` in to 80 over the first 100 frames and
//! out to 0 during the last 20 frames of the primitive's `duration`, and fills
//! its 21 height samples with a sine envelope times the slot's `maxHeight`:
//!
//! ```cpp
//! slot.height[sample] = slot.maxHeight * g_sinTable[sample * 9] * 0.3f *
//!                       g_sinTable[angle % 360] + slot.height[sample];
//! ```
//!
//! Quirks kept:
//!
//! * the per-sample angle guard is `index * 45 < 90`, so only slots 0 and 1 use
//!   the `process + index * 90` phase; slots 2 and 3 use `(process + index *
//!   45) * 2`,
//! * the fade-out test compares the *slot's* `process` with the *primitive's*
//!   `duration - 20`, and the fade-in stops after 100 frames even when the
//!   duration is longer.
//!
//! The renderer is the shared [`render_3d_casting`] of
//! [`super::super::render_slots`] (`EffectPrimitiveRendering.cpp:6596-6675`,
//! dispatch `:499-515`). The only body that spawns prim 24 is
//! `CRagEffect::QuadRingStack` (`RagEffect.cpp:14107-14183`), used by
//! `EF_DEFENDER` (222) and `EF_REFLECTSHIELD` (252) - see
//! [`super::super::rag::quad_ring_stack`].
//!
//! [`render_3d_casting`]: super::super::render_slots::render_3d_casting

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PP_3DCASTING_2`. The motion always returns `true`; the
/// primitive retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) -> bool {
    // `this->render.position = this->state.master->render.position;`
    state.position = master.position;

    let duration = state.duration;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        // `++slot.process;`
        slot.process += 1;

        // `if (index == 0 || index == 2) ++slot.rotStart; else slot.rotStart += 2;`
        if index == 0 || index == 2 {
            slot.rot_start += 1;
        } else {
            slot.rot_start += 2;
        }

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        // `if (slot.process < this->state.duration - 20)` - the primitive's own
        // duration, which the body copies from the owning `CRagEffect`.
        if slot.process < duration - 20 {
            if slot.process < 100 {
                slot.alpha_b += 1;

                if slot.alpha_b > 80 {
                    slot.alpha_b = 80;
                }
            }
        } else {
            slot.alpha_b -= 5;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        for sample in 0..21usize {
            // `const int angle = index * 45 < 90 ? slot.process + index * 90 :
            //                                (slot.process + index * 45) * 2;`
            let angle = if index * 45 < 90 {
                slot.process + index as i32 * 90
            } else {
                (slot.process + index as i32 * 45) * 2
            };

            // The first assignment is the second one's `+ slot.height[sample]`
            // term; both use the same `g_sinTable[sample * 9]`.
            let envelope = slot.max_height * angles.raw_sine(sample * 9);
            slot.height[sample] =
                slot.max_height * angles.raw_sine(sample * 9) * 0.3 * angles.raw_sine((angle % 360).max(0) as usize) + envelope;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::Casting2,
            duration: 200,
            ..Default::default()
        }
    }

    fn armed(rot_start: i16, max_height: f32) -> EffectPrimitiveSlot {
        EffectPrimitiveSlot {
            life: 1,
            process: 0,
            full_display_angle: 360,
            max_height,
            rot_start,
            alpha_b: 0,
            distance: 8.0,
            rise_angle: 90,
            ..Default::default()
        }
    }

    #[test]
    fn primitive_id_is_24() {
        assert_eq!(PrimitiveId::Casting2.id(), 24);
    }

    #[test]
    fn the_ring_follows_the_master_position() {
        let angles = Angles::new();
        let master = MasterState {
            position: [4.0, -2.0, 9.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position, [4.0, -2.0, 9.0]);
    }

    #[test]
    fn slots_spin_at_one_and_two_degrees() {
        let angles = Angles::new();
        let mut state = state();

        for index in 0..4 {
            state.slots[index] = armed([180, 270, 0, 90][index], 40.0);
        }

        on_process(&mut state, &angles, &MasterState::default());

        // slots 0/2 +1, slots 1/3 +2.
        assert_eq!(state.slots[0].rot_start, 181);
        assert_eq!(state.slots[1].rot_start, 272);
        assert_eq!(state.slots[2].rot_start, 1);
        assert_eq!(state.slots[3].rot_start, 92);
    }

    #[test]
    fn rotation_wraps_at_three_hundred_sixty() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0] = armed(359, 40.0);
        state.slots[3] = armed(359, 40.0);

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].rot_start, 0);
        assert_eq!(state.slots[3].rot_start, 1);
    }

    #[test]
    fn alpha_rises_by_one_to_eighty_then_falls_by_five() {
        let angles = Angles::new();
        let mut state = state();
        state.duration = 200;
        state.slots[0] = armed(0, 40.0);

        for _ in 0..100 {
            on_process(&mut state, &angles, &MasterState::default());
        }

        assert_eq!(state.slots[0].process, 100);
        assert_eq!(state.slots[0].alpha_b, 80, "the +1 ramp stops at 80");

        // `process < duration - 20` == `process < 180` keeps the ramp; the
        // 101st frame is still below the cap.
        on_process(&mut state, &angles, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 80);

        // Run to process 179: still inside the `process < duration - 20` ramp.
        while state.slots[0].process < 179 {
            on_process(&mut state, &angles, &MasterState::default());
        }

        assert_eq!(state.slots[0].process, 179);
        assert_eq!(state.slots[0].alpha_b, 80);

        // The 180th step is the first fade frame.
        on_process(&mut state, &angles, &MasterState::default());
        assert_eq!(state.slots[0].process, 180);
        assert_eq!(state.slots[0].alpha_b, 75);

        // The fade clamps at zero (17 frames of -5 from 80).
        for _ in 0..40 {
            on_process(&mut state, &angles, &MasterState::default());
        }
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn heights_use_the_sine_envelope_and_the_official_angle_quirk() {
        let angles = Angles::new();
        let mut state = state();
        state.duration = 200;

        for index in 0..4 {
            state.slots[index] = armed(0, 10.0);
        }

        on_process(&mut state, &angles, &MasterState::default());

        for index in 0..4usize {
            // `index * 45 < 90` is true for 0/1 only.
            let angle = if index * 45 < 90 {
                1 + index as i32 * 90
            } else {
                (1 + index as i32 * 45) * 2
            };
            let expected: Vec<f32> = (0..21)
                .map(|sample| {
                    10.0 * angles.raw_sine(sample * 9) * 0.3 * angles.raw_sine((angle % 360) as usize) + 10.0 * angles.raw_sine(sample * 9)
                })
                .collect();

            assert_eq!(&state.slots[index].height[..], &expected[..], "slot {index}");
        }

        // Slot 3's doubled phase differs from the un-doubled one, which is the
        // quirk: (process + 135) * 2 = 272, not 136.
        assert_ne!(angles.raw_sine(272), angles.raw_sine(136));
        assert_eq!(
            state.slots[3].height[0],
            10.0 * angles.raw_sine(0) * 0.3 * angles.raw_sine(272) + 10.0 * angles.raw_sine(0)
        );
    }

    #[test]
    fn dead_slots_are_untouched() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].life = 0;
        state.slots[0].rot_start = 90;
        state.slots[0].process = 7;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].rot_start, 90);
        assert_eq!(state.slots[0].process, 7);
        assert_eq!(state.slots[0].height, [0.0; 21]);
    }
}
