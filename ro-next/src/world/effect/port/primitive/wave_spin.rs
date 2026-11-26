//! Prim 31 - `EffectPrimitiveMotion.cpp:5663-5732`
//! (`CEffectPrim::PrimWaveSpin`).
//!
//! The official enum has no symbol for this id (`EffectPrimId.h` jumps from
//! `PP_MAPPARTICLE` 27 to `PP_GI_3` 34). The port names it after its motion
//! body; the renderer is the shared `Render3DCasting`
//! (`EffectPrimitiveRendering.cpp:6596`, dispatch `:499-515`).
//!
//! Four spinning rings: every live slot advances its process, turns
//! `rotStart` by `5` degrees (`3` when the primitive's `accel` is the `99.0`
//! marker), pushes `distance` out by `0.3` (`0.1`), and raises `riseAngle`
//! towards its floor of `40` (`30` with the marker). Until process 10 the
//! alpha rises by `10` (kind 1) or `20`; afterwards it drains by the kind's
//! step and is clamped at zero.
//!
//! Each of the 21 height samples is the same sine envelope at
//! `g_sinTable[9 * point]` plus a per-slot travelling wave:
//!
//! ```cpp
//! const float base = g_sinTable[9 * point] * slot.maxHeight;
//! slot.height[point] = base;
//! slot.height[point] = g_sinTable[9 * point] * slot.maxHeight * 0.15f *
//!                      g_sinTable[(phase + slot.process) % 360] + base;
//! ```
//!
//! Spawned by `CRagEffect::TexturedCrossLevel` (`RagEffect.cpp:5246`) -
//! `EF_GUMGANG2` (261).

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimWaveSpin`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    // `FollowMaster(*this);`
    state.position = master.position;

    // The marker is an exact float comparison in the original as well.
    let fast = state.accel != 99.0;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        // `const int process = slot.process + 1; slot.process = process;`
        let process = slot.process + 1;
        slot.process = process;

        slot.rot_start += if fast { 5 } else { 3 };

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        slot.distance += if fast { 0.3 } else { 0.1 };
        slot.rise_angle -= 1;

        if fast {
            if slot.rise_angle < 40 {
                slot.rise_angle = 40;
            }
        } else if slot.rise_angle < 30 {
            slot.rise_angle = 30;
        }

        if process > 10 {
            let step = match slot.alpha_t {
                2 => 8,
                3 | 4 => 5,
                5 => 4,
                _ => 2,
            };

            slot.alpha_b -= step;

            if slot.alpha_b <= 0 {
                slot.alpha_b = 0;
            }
        } else if slot.alpha_t == 1 {
            slot.alpha_b += 10;
        } else {
            slot.alpha_b += 20;
        }

        let phase = index as i32 * 90;

        for point in 0..21usize {
            let base = angles.raw_sine(9 * point) * slot.max_height;
            slot.height[point] = base;
            slot.height[point] =
                angles.raw_sine(9 * point) * slot.max_height * 0.15 * angles.raw_sine(((phase + slot.process) % 360) as usize) + base;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn ring(accel: f32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::WaveSpin,
            duration: 300,
            accel,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].full_display_angle = 360;
        state.slots[0].max_height = 25.0;
        state.slots[0].rise_angle = 80;
        state.slots[0].rot_start = 356;
        state.slots[0].alpha_t = 0;

        state
    }

    #[test]
    fn wave_spin_is_primitive_31() {
        assert_eq!(PrimitiveId::WaveSpin.id(), 31);
    }

    #[test]
    fn first_frame_spins_and_raises_the_alpha() {
        let mut state = ring(0.0);
        on_process(&mut state, Angles::shared(), &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        assert_eq!(slot.rot_start, 1); // 356 + 5 wraps past 360
        assert!((slot.distance - 0.3).abs() < 1e-5);
        assert_eq!(slot.rise_angle, 79);
        assert_eq!(slot.alpha_b, 20);
    }

    #[test]
    fn the_accel_marker_slows_the_spin() {
        let mut state = ring(99.0);
        on_process(&mut state, Angles::shared(), &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.rot_start, 359);
        assert!((slot.distance - 0.1).abs() < 1e-5);
        assert_eq!(slot.rise_angle, 79);
    }

    #[test]
    fn rise_angle_floor_depends_on_the_marker() {
        let mut state = ring(0.0);
        state.slots[0].rise_angle = 41;
        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].rise_angle, 40);

        let mut state = ring(99.0);
        state.slots[0].rise_angle = 31;
        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].rise_angle, 30);
    }

    #[test]
    fn alpha_drain_uses_the_kind_step() {
        for (kind, step) in [(0, 2), (1, 2), (2, 8), (3, 5), (4, 5), (5, 4), (7, 2)] {
            let mut state = ring(0.0);
            state.slots[0].process = 10;
            state.slots[0].alpha_t = kind;
            state.slots[0].alpha_b = 100;

            on_process(&mut state, Angles::shared(), &MasterState::default());

            assert_eq!(state.slots[0].alpha_b, 100 - step, "kind {kind}");
        }
    }

    #[test]
    fn kind_one_raises_alpha_by_ten() {
        let mut state = ring(0.0);
        state.slots[0].alpha_t = 1;
        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 10);
    }

    #[test]
    fn height_samples_are_the_travelling_wave() {
        let mut state = ring(0.0);
        on_process(&mut state, Angles::shared(), &MasterState::default());

        let angles = Angles::shared();
        let base = angles.raw_sine(9) * 25.0;
        let expected = base * 0.15 * angles.raw_sine(1) + base;

        assert!((state.slots[0].height[1] - expected).abs() < 1e-4);
    }
}
