//! prim 65 (`PP_3DCASTING_4`) - `EffectPrimitiveMotion.cpp:348-390`.
//!
//! `CEffectPrim::Prim3DCasting4`: the four falling rings of
//! `CRagEffect::AlphaDownRipples` (`EF_WARPZONE2`). Each live slot sinks by
//! `0.05` per frame and snaps back to a `10.0` radius at the ground; the 21
//! height samples follow `maxHeight` while the slot's `flag1` stays clear, and
//! the slot fades out over the last 40 frames of the primitive.

use super::{EffectPrimitiveState, MasterState};

/// One frame of `Prim3DCasting4`.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let duration = state.duration;
        let slot = &mut state.slots[index];

        slot.process += 1;
        slot.distance -= 0.05;

        if slot.distance <= 0.0 {
            slot.distance = 10.0;
            slot.alpha_b = 0;
        }

        slot.rise_angle = (90.0 - slot.distance * 9.0) as i16;
        slot.max_height = slot.distance;

        if slot.process >= duration - 40 {
            slot.alpha_b -= 3;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        } else if slot.alpha_b < 70 {
            slot.alpha_b += 1;
        }

        for height in 0..21usize {
            if slot.flag1[height] == 0 {
                let mut value = slot.max_height;

                if value < 0.0 {
                    value = 0.0;
                }

                slot.height[height] = value;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn rings() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Casting4,
            duration: 300,
            ..Default::default()
        };

        for index in 0..4 {
            state.slots[index] = EffectPrimitiveSlot {
                life: 1,
                distance: 2.5 + index as f32,
                max_height: 2.5 + index as f32,
                full_display_angle: 360,
                ..Default::default()
            };
        }

        state
    }

    #[test]
    fn rings_sink_and_snap_back() {
        let mut state = rings();
        let master = MasterState::default();

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].process, 1);
        assert!((state.slots[0].distance - 2.45).abs() < 1e-6);
        assert!((state.slots[0].max_height - 2.45).abs() < 1e-6);
        assert_eq!(state.slots[0].rise_angle, (90.0f32 - 2.45 * 9.0) as i16);
        assert_eq!(state.slots[0].alpha_b, 1);
        assert!((state.slots[0].height[0] - 2.45).abs() < 1e-6);

        // A slot at the ground resets to the 10-unit drop height.
        state.slots[1].distance = 0.02;
        on_process(&mut state, &master);
        assert_eq!(state.slots[1].distance, 10.0);
        assert_eq!(state.slots[1].alpha_b, 1);
    }

    #[test]
    fn ring_fades_out_during_the_last_forty_frames() {
        let mut state = rings();
        let master = MasterState::default();

        state.duration = 100;
        state.slots[0].process = 60;
        state.slots[0].alpha_b = 70;
        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 67);
    }

    #[test]
    fn inactive_slots_are_left_alone() {
        let mut state = rings();
        let master = MasterState::default();

        state.slots[2].life = 0;
        state.slots[2].distance = 4.0;
        on_process(&mut state, &master);

        assert_eq!(state.slots[2].distance, 4.0);
        assert_eq!(state.slots[2].process, 0);
    }
}
