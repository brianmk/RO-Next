//! prim 86 - the bare `case 86:` in `CEffectPrim::OnProcess`,
//! `EffectPrimitiveMotion.cpp:9315-9365`; dispatch `:9315`.
//!
//! The purple / sonic-wave slash of `EF_SOULBREAKER` (361) and, with
//! `flag1[1] = 1`, of the four-texture `SonicWave(1)` callers.
//! `EffectPrimId.h` has no symbol for the id, so the port names the variant
//! after its body, `CRagEffect::SonicWave` (`RagEffect.cpp:5382-5426`).
//!
//! Four independent shards start with a negative `process` (the body arms them
//! `index - 28`), so they appear one after the other. On the first live frame
//! the shard turns towards the target (`Get_Angle` of `deltaPos - position`)
//! plus a random `-5..+5` (flag `1`) or `-15..+15` (flag `0`); afterwards its
//! speed grows by 2% per frame while the alpha rises by 10, then by 4% per
//! frame while the alpha falls by 5. A shard whose alpha falls below zero dies.
//!
//! The renderer is `RenderFloorSquare` (`EffectPrimitiveRendering.cpp:2637`),
//! dispatched by the `case 86`/`case 92` arm at
//! `EffectPrimitiveRendering.cpp:545-547` - see
//! [`super::super::render_slots::render_floor_square`].

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState, get_angle};

/// One frame of the official `case 86:`
/// (`EffectPrimitiveMotion.cpp:9315-9365`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) {
    // The original does not follow the master here: `SonicWave` writes the
    // primitive's `render.position` once at spawn.
    let _ = master;

    let to_target = [state.delta_pos[0] - state.position[0], state.delta_pos[2] - state.position[2]];

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        // `const int process = slot.process++;`
        let process = slot.process;
        slot.process += 1;

        // `if (slot.process <= 0) continue;` - the gate reads the *post*
        // increment value.
        if slot.process <= 0 {
            continue;
        }

        if process == 0 {
            // `Get_Angle(deltaPos.x - position.x, deltaPos.z - position.z, &rotStart);`
            slot.rot_start = get_angle(to_target[0], to_target[1]);

            // `int roll = rand() % 11; turn = (roll < 0 ? 0 : roll) - 5;`
            // `int roll = rand() % 31; turn = (roll < 0 ? 0 : roll) - 15;`
            let turn = if slot.flag1[1] == 1 {
                (rng.clamped_random(11)).max(0) - 5
            } else {
                (rng.clamped_random(31)).max(0) - 15
            };

            let mut rot_start = slot.rot_start as i32 + turn;

            if rot_start >= 360 {
                rot_start -= 360;
            }

            if rot_start < 0 {
                rot_start += 360;
            }

            slot.rot_start = rot_start as i16;
        } else if process < 10 {
            slot.alpha_b += 10;
            slot.height[1] *= 1.02;
        } else {
            slot.alpha_b -= 5;
            slot.height[1] *= 1.04;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
                slot.life = 0;
            }
        }

        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn shards() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::SonicWave,
            duration: 300,
            size: 0.0,
            ..Default::default()
        };
        state.position = [0.0, 0.0, 0.0];
        state.delta_pos = [10.0, 0.0, 0.0];

        for slot in &mut state.slots {
            slot.life = 1;
            slot.alpha_b = 0;
            slot.process = 0;
            slot.distance = 20.0;
            slot.height[1] = 0.8;
        }

        state
    }

    #[test]
    fn sonic_wave_is_primitive_86() {
        assert_eq!(PrimitiveId::SonicWave.id(), 86);
    }

    #[test]
    fn the_first_frame_turns_towards_the_target_with_the_flag_zero_range() {
        let mut state = shards();
        state.slots[0].flag1[1] = 0;

        let mut predictor = EffectRng::new(7);
        let turn = predictor.clamped_random(31) - 15;
        let mut rng = EffectRng::new(7);

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut rng);

        let mut expected = get_angle(10.0, 0.0) as i32 + turn;
        if expected >= 360 {
            expected -= 360;
        }
        if expected < 0 {
            expected += 360;
        }

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].rot_start, expected as i16);
        assert_eq!(state.slots[0].alpha_b, 0, "the first frame does not ramp");
        // The shard already steps along the new heading.
        assert_ne!(state.slots[0].vec_b_now, [0.0; 3]);
    }

    #[test]
    fn the_sonic_texture_flag_uses_the_eleven_value_roll() {
        let mut state = shards();
        state.slots[0].flag1[1] = 1;

        let mut predictor = EffectRng::new(11);
        let turn = predictor.clamped_random(11) - 5;
        let mut rng = EffectRng::new(11);

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut rng);

        let mut expected = get_angle(10.0, 0.0) as i32 + turn;
        if expected >= 360 {
            expected -= 360;
        }
        if expected < 0 {
            expected += 360;
        }

        assert_eq!(state.slots[0].rot_start, expected as i16);
    }

    #[test]
    fn frames_one_to_nine_ramp_and_accelerate() {
        let mut state = shards();
        state.slots[0].process = 0;

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));

        // process 1 -> process 2 on the next call; that is the `process < 10`
        // branch (the pre-increment value is 1).
        state.slots[0].vec_b_now = [0.0; 3];
        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].process, 2);
        assert_eq!(state.slots[0].alpha_b, 10);
        assert!((state.slots[0].height[1] - 0.8 * 1.02).abs() < 1e-6);
    }

    #[test]
    fn from_frame_ten_the_alpha_drains_and_the_shard_dies_below_zero() {
        let mut state = shards();
        state.slots[0].process = 10;
        state.slots[0].alpha_b = 10;

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].process, 11);
        assert_eq!(state.slots[0].alpha_b, 5);
        assert!((state.slots[0].height[1] - 0.8 * 1.04).abs() < 1e-6);

        state.slots[0].alpha_b = 3;
        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn a_negative_process_only_advances_until_it_reaches_zero() {
        let mut state = shards();
        state.slots[0].process = -2;
        state.slots[0].flag1[1] = 0;

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].process, -1);
        assert_eq!(state.slots[0].rot_start, 0);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3]);

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3]);

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].process, 1);
        assert_ne!(state.slots[0].vec_b_now, [0.0; 3]);
    }
}
