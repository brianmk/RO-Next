//! Prim 30 - `EffectPrimitiveMotion.cpp:8863-8916` (`case 30:`).
//!
//! The official enum has no symbol for this id (`EffectPrimId.h` jumps from
//! `PP_MAPPARTICLE` 27 to `PP_GI_3` 34), so the port names it after its
//! renderer, `Render3DCasting` (`EffectPrimitiveRendering.cpp:6596`, dispatch
//! `:499-515`), which it shares with prims 24, 31 and the other casting rings.
//!
//! Four expanding, fading rings: every live slot advances its process, pushes
//! `distance` out by `0.07`, lowers `riseAngle` by one (clamped at 10, which
//! also zeroes `alphaB`), and fades in by `5`/`10` per frame (the `10` for
//! every kind except `alphaT == 22`) until `distance` reaches `4.0`. From then
//! on `alphaB` drops by `3` per frame; when it falls below one the body resets
//! `distance` to `3.37` and re-arms `riseAngle` to `64` (`alphaT == 22`) or
//! `74` - but only while `process < duration - 30`, so the last thirty frames
//! are a clean fade-out.
//!
//! Each of the 21 height samples gets the same envelope, sampled at
//! `g_sinTable[sample * 9]`, and its own angular phase:
//!
//! ```cpp
//! const int angle = index * 45 < 90 ? slot.process + index * 90
//!                                   : (slot.process + index * 45) * 2;
//! slot.height[sample] = slot.maxHeight;
//! slot.height[sample] = g_sinTable[sample * 9] * slot.maxHeight * 0.3f *
//!                       g_sinTable[angle % 360] + slot.height[sample];
//! ```
//!
//! The first assignment is the second one's `+ slot.height[sample]` term, so
//! the port evaluates the same expression once.
//!
//! Quirks kept:
//!
//! * the per-sample angle guard is `index * 45 < 90`, so slots 0/1 use `process
//!   + index * 90` and slots 2/3 use `(process + index * 45) * 2`,
//! * `alphaT` carries the ring kind (`22` for `WhiteYellowRings`), it is not a
//!   camera angle for this primitive,
//! * `alphaB` is not clamped above 255; the renderer packs it into a byte.
//!
//! Spawned by `CRagEffect::TexturedQuadRingSized` (`RagEffect.cpp:17437`) -
//! `EF_AURABLADE` (367) and the rings of `EF_BEGINASURA`/`11`
//! (`AsuraStrike`, `RagEffect.cpp:14187`).

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `case 30:`. The case body has no return value; the primitive
/// retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    // `this->render.position = this->state.master->render.position;`
    state.position = master.position;

    let duration = state.duration;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        slot.distance += 0.07;
        slot.rise_angle -= 1;

        if slot.rise_angle < 10 {
            slot.rise_angle = 10;
            slot.alpha_b = 0;
        }

        if slot.distance < 4.0 {
            if slot.alpha_t == 22 {
                slot.alpha_b += 5;
            } else {
                slot.alpha_b += 10;
            }
        } else {
            slot.alpha_b -= 3;

            if slot.alpha_b < 1 {
                slot.alpha_b = 0;

                if slot.process < duration - 30 {
                    slot.distance = 3.37;

                    if slot.alpha_t == 22 {
                        slot.rise_angle = 64;
                    } else {
                        slot.rise_angle = 74;
                    }
                }
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

            let envelope = slot.max_height * angles.raw_sine(sample * 9);
            slot.height[sample] = envelope * 0.3 * angles.raw_sine((angle % 360).max(0) as usize) + envelope;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn ring(kind: i16) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Casting3d,
            duration: 60,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].full_display_angle = 360;
        state.slots[0].max_height = 25.0;
        state.slots[0].alpha_t = kind;
        state.slots[0].distance = 4.1;
        state.slots[0].rise_angle = 80;

        state
    }

    #[test]
    fn casting_3d_is_primitive_30() {
        assert_eq!(PrimitiveId::Casting3d.id(), 30);
    }

    #[test]
    fn a_ring_that_starts_just_outside_the_radius_re_arms() {
        let mut state = ring(0);
        on_process(&mut state, Angles::shared(), &MasterState::default());

        // `4.1 + 0.07 >= 4.0`, so the first frame drains the alpha (already
        // zero), re-arms the radius and the rise angle, and keeps expanding.
        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        assert_eq!(slot.distance, 3.37);
        assert_eq!(slot.rise_angle, 74);
        assert_eq!(slot.alpha_b, 0);
        assert!(slot.height[1] != 0.0);
    }

    #[test]
    fn kind_22_fades_in_at_half_speed() {
        let mut state = ring(22);
        state.slots[0].distance = 3.0;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 5);
        assert_eq!(state.slots[0].rise_angle, 79);
    }

    #[test]
    fn a_ring_inside_the_radius_fades_in_and_out() {
        let mut state = ring(0);
        state.slots[0].distance = 3.0;

        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 10);

        state.slots[0].distance = 4.1;
        state.slots[0].alpha_b = 2;
        on_process(&mut state, Angles::shared(), &MasterState::default());

        // Below one, the ring re-arms its radius and angle.
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].distance, 3.37);
        assert_eq!(state.slots[0].rise_angle, 74);
    }

    #[test]
    fn the_last_thirty_frames_do_not_re_arm() {
        let mut state = ring(0);
        state.slots[0].process = 29; // this frame is 30 == duration - 30
        state.slots[0].distance = 4.1;
        state.slots[0].alpha_b = 2;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 0);
        assert!((state.slots[0].distance - 4.17).abs() < 1e-5);
        assert_eq!(state.slots[0].rise_angle, 79);
    }

    #[test]
    fn the_rise_angle_floor_is_ten_and_zeroes_the_alpha() {
        // Eleven becomes ten without hitting the guard.
        let mut state = ring(0);
        state.slots[0].distance = 4.0;
        state.slots[0].rise_angle = 11;
        state.slots[0].alpha_b = 50;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].rise_angle, 10);
        assert_eq!(state.slots[0].alpha_b, 47);

        // Ten would drop below the floor, so the guard fires: rise snaps to
        // ten and the alpha is zeroed before the out-of-radius branch re-arms
        // the ring.
        let mut state = ring(0);
        state.slots[0].distance = 4.0;
        state.slots[0].rise_angle = 10;
        state.slots[0].alpha_b = 50;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].rise_angle, 74);
        assert_eq!(state.slots[0].distance, 3.37);
    }

    #[test]
    fn the_phase_guard_separates_the_two_slot_pairs() {
        let mut state = ring(0);
        for slot in &mut state.slots {
            slot.life = 1;
            slot.process = 0;
            slot.max_height = 20.0;
            slot.distance = 4.1;
        }

        on_process(&mut state, Angles::shared(), &MasterState::default());

        let angles = Angles::shared();
        let envelope = 20.0 * angles.raw_sine(9);
        let expected = |degrees: i32| envelope * 0.3 * angles.raw_sine(degrees as usize) + envelope;

        // Slots 0/1 use `1 + index * 90`, slots 2/3 `(1 + index * 45) * 2`.
        assert!((state.slots[0].height[1] - expected(1)).abs() < 1e-4);
        assert!((state.slots[1].height[1] - expected(91)).abs() < 1e-4);
        assert!((state.slots[2].height[1] - expected(182)).abs() < 1e-4);
        assert!((state.slots[3].height[1] - expected(272)).abs() < 1e-4);
    }
}
