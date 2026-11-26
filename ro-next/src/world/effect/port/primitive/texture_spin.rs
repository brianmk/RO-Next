//! prim 143 (`PrimTextureSpin`) - `EffectPrimitiveMotion.cpp:4759-4792`.
//!
//! Animated ground rings (`torch_red`, `dust`, `W_c` sequences): slot rotation,
//! orbit drift and a texture frame advanced every four processed frames.

use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimTextureSpin`.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;

    let frames = state.frame_count as f32;

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        slot.process += 1;

        if slot.height[2] == 0.0 {
            slot.rot_start += 6;

            if slot.rot_start >= 360 {
                slot.rot_start -= 360;
            }

            slot.rise_angle += 3;

            if slot.rise_angle >= 360 {
                slot.rise_angle -= 360;
            }
        }

        if slot.process % 4 == 0 {
            slot.height[1] += 1.0;
        }

        if slot.height[1] >= frames {
            slot.height[1] -= frames;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn ring(height2: f32, frames: u32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::TextureSpin,
            duration: 100,
            frame_count: frames,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            height: {
                let mut height = [0.0f32; 21];
                height[2] = height2;
                height
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn spinning_ring_advances_rotation_and_orbit() {
        let master = MasterState::default();
        let mut state = ring(0.0, 13);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].rot_start, 6);
        assert_eq!(state.slots[0].rise_angle, 3);
        assert_eq!(state.slots[0].process, 1);
    }

    #[test]
    fn texture_frame_advances_every_four_frames() {
        let master = MasterState::default();
        let mut state = ring(0.0, 13);

        for _ in 0..3 {
            on_process(&mut state, &master);
        }
        assert_eq!(state.slots[0].height[1], 0.0);

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].height[1], 1.0);
    }

    #[test]
    fn static_ring_keeps_its_rotation() {
        let master = MasterState::default();
        let mut state = ring(1.0, 51);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].rot_start, 0);
        assert_eq!(state.slots[0].rise_angle, 0);
    }

    #[test]
    fn texture_frame_wraps_at_the_sequence_length() {
        let master = MasterState::default();
        let mut state = ring(0.0, 9);
        state.slots[0].height[1] = 8.0;

        for _ in 0..4 {
            on_process(&mut state, &master);
        }

        assert!(state.slots[0].height[1] < 9.0);
    }
}
