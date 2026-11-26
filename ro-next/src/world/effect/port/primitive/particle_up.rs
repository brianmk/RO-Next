//! prim 59 (`PrimParticleUp`) - `EffectPrimitiveMotion.cpp:3692-3738`.
//!
//! Upward drifting slots (heal sparks): rotate, ramp alpha in the first ten
//! processed frames, fade out, and move up by `height[2]`.

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimParticleUp`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) {
    state.position = master.position;

    let mut index = 0usize;
    let mut flash = 36i32;

    while flash > 8 {
        if state.slots[index].life == 1 {
            let slot = &mut state.slots[index];
            let process = slot.process + 1;
            slot.process = process;

            if process > 0 {
                slot.full_display_angle -= 5;

                if slot.full_display_angle < 0 {
                    slot.full_display_angle += 360;
                }

                if process == 1 {
                    slot.rot_start = rng.clamped_random(360) as i16;
                } else if process <= 10 {
                    slot.alpha_b += 15;
                } else {
                    if slot.flag1[1] == 1 {
                        if process == flash - 1 || process == flash {
                            slot.flag1[0] = 1;
                        } else {
                            slot.flag1[0] = 0;
                        }
                    }

                    slot.alpha_b -= 3;

                    if slot.alpha_b < 0 {
                        slot.alpha_b = 0;
                        slot.life = 0;
                    }
                }

                slot.vec_b_now[1] -= slot.height[2];
                slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start.max(0) as usize);
                slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start.max(0) as usize);
            }
        }

        flash -= 7;
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn up() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::ParticleUp,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            height: {
                let mut height = [0.0f32; 21];
                height[1] = 2.0;
                height[2] = 1.5;
                height
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn first_frame_sets_the_rotation_and_moves_up() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(3);
        let mut state = up();

        on_process(&mut state, &angles, &master, &mut rng);

        assert_eq!(state.slots[0].process, 1);
        assert!((state.slots[0].vec_b_now[1] + 1.5).abs() < 1e-6);
        assert_eq!(state.slots[0].full_display_angle, 355);
    }

    #[test]
    fn alpha_ramps_then_fades_and_dies() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(3);
        let mut state = up();
        state.slots[0].process = 9;
        state.slots[0].alpha_b = 150;

        // process 9 -> 10 still ramps (+15).
        on_process(&mut state, &angles, &master, &mut rng);
        assert_eq!(state.slots[0].alpha_b, 165, "frame 10 still ramps");

        // process 10 -> 11 fades (-3).
        on_process(&mut state, &angles, &master, &mut rng);
        assert_eq!(state.slots[0].alpha_b, 162, "frame 11 fades");
    }
}
