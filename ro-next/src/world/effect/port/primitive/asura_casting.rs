//! `PP_ASURACASTING` (68) - `EffectPrimitiveMotion.cpp:3224-3405`
//! (`CEffectPrim::PrimAsuraCasting`).
//!
//! The three kanji columns of `EF_BEGINASURA`/`EF_BEGINASURA11` (the six
//! `effect\asuraN.tga` glyphs) and the icon flash of `EF_BEGINASURA1..7`.
//! Every live slot ramps its ten height
//! samples in thirteen-unit steps during the first twenty frames, then drains
//! them and starts eating the sample at the top (`ShrinkLast`). The exact ramp
//! depends on `flag1[4]`:
//!
//! * `2` - the short column: limits `33, 56, 79, ... 240` (23 apart) and a
//!   two-unit drain over samples `1..=8`, with the two last samples shrinking
//!   after frame 100,
//! * `0`/`1`/`3` - the tall column: limits `70, 90, 110, ... 250` (20 apart)
//!   and a five-unit drain over samples `1..=9`, with only the top sample
//!   shrinking (after frame 140, or after frame 25 when `flag1[4] == 4`).
//!
//! Quirks kept:
//!
//! * the tall-column countdown is written as `process > (flag1[4] != 4 ? 140 :
//!   25)`, so kind `4` shrinks its top sample much earlier,
//! * `distance -= 0.1` only during the first fifty frames and only for kinds
//!   other than `4`,
//! * `FollowMaster` is conditional: the primitive only snaps to the master
//!   position when `slots[0].flag1[4] == 1` (the icon bodies), never for the
//!   kanji columns (`0`/`3`), whose heights already carry their offset. The
//!   master position is the effect's `render.position`, which `AsuraStrike` and
//!   `TexturedSkillIcon` refresh from the caster every frame
//!   (`EffectRuntime::copy_parent_position`),
//! * `FaceCamera(state.slots)` writes `slots[0/1].alphaT`, which the renderer
//!   turns into a rotation. The port's hermetic simulation has no camera, so
//!   the renderer computes the same two angles from the live camera at draw
//!   time ([`super::super::render_slots::face_camera_angles`]) exactly like
//!   `RenderTeiFacingSlots` does for prim 82.
//!
//! Spawned by `CRagEffect::AsuraStrike` (`RagEffect.cpp:14187`) and
//! `CRagEffect::TexturedSkillIcon` (`RagEffect.cpp:3774`) - see
//! [`super::super::rag::asura`]. The renderer is `RenderHaloLayers`
//! (`EffectPrimitiveRendering.cpp:4140`, dispatch `:430-433`).

use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// `ShrinkLast` - `EffectPrimitiveMotion.cpp:3084-3094`.
fn shrink_last(slot: &mut EffectPrimitiveSlot, value: usize, growth: f32) {
    let height = slot.height[value] - 5.0;
    slot.height[value] = height;

    if height > 0.0 {
        slot.distance *= growth;
    }

    if height < 0.0 {
        slot.height[value] = 0.0;
    }
}

/// One frame of `PrimAsuraCasting`. Always returns `true`; the primitive
/// retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    // `FaceCamera(this->state.slots);` is applied by the renderer (no camera in
    // the simulation).

    // `if (this->state.slots[0].flag1[4] == 1) FollowMaster(*this);`
    if state.slots[0].flag1[4] == 1 {
        state.position = master.position;
    }

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        // `if (++slot.process <= 0) continue;`
        let slot = &mut state.slots[index];
        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        if slot.flag1[4] == 2 {
            if slot.process < 20 {
                for value in 1..=10usize {
                    slot.height[value] += 13.0;

                    let limit = 33.0 + (value as f32 - 1.0) * 23.0;

                    if slot.height[value] >= limit {
                        slot.height[value] = limit;
                    }
                }
            } else {
                for value in 1..9usize {
                    slot.height[value] -= 2.0;

                    if slot.height[value] < 0.0 {
                        slot.height[value] = 0.0;
                    }
                }

                if slot.process > 100 {
                    shrink_last(slot, 9, 1.005);
                    shrink_last(slot, 10, 1.005);
                }
            }

            continue;
        }

        if slot.process < 20 {
            for value in 1..=10usize {
                slot.height[value] += 13.0;

                let limit = 70.0 + (value as f32 - 1.0) * 20.0;

                if slot.height[value] >= limit {
                    slot.height[value] = limit;
                }
            }
        } else {
            for value in 1..10usize {
                slot.height[value] -= 5.0;

                if slot.height[value] < 0.0 {
                    slot.height[value] = 0.0;
                }
            }

            let limit = if slot.flag1[4] != 4 { 140 } else { 25 };

            if slot.process > limit {
                shrink_last(slot, 10, 1.01);
            }
        }

        if slot.process < 50 && slot.flag1[4] != 4 {
            slot.distance -= 0.1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn blade(kind: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::AsuraCasting,
            duration: 1000,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = -1;
        state.slots[0].flag1[4] = kind;
        state.slots[0].distance = 18.0;

        state
    }

    #[test]
    fn asura_casting_is_primitive_68() {
        assert_eq!(PrimitiveId::AsuraCasting.id(), 68);
    }

    #[test]
    fn a_process_that_lands_on_zero_does_not_ramp() {
        let mut state = blade(0);

        on_process(&mut state, &MasterState::default());

        // `-1 + 1 == 0`, and `if (++slot.process <= 0) continue;`.
        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].height[1], 0.0);
        assert_eq!(state.slots[0].height[10], 0.0);
    }

    #[test]
    fn tall_column_ramps_by_thirteen_to_the_official_limits() {
        let mut state = blade(0);
        state.slots[0].process = 0;

        on_process(&mut state, &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        for value in 1..=10usize {
            assert_eq!(slot.height[value], 13.0);
        }

        // The per-sample limits are 70, 90, ... 250; the clamp is applied
        // after the thirteen-unit step.
        state.slots[0].height[1] = 69.0;
        state.slots[0].height[10] = 240.0;
        on_process(&mut state, &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.height[1], 70.0);
        assert_eq!(slot.height[10], 250.0);
    }

    #[test]
    fn short_column_uses_the_23_step_limits() {
        let mut state = blade(2);
        state.slots[0].process = 0;
        state.slots[0].height[1] = 32.0;
        state.slots[0].height[10] = 230.0;

        on_process(&mut state, &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.height[1], 33.0);
        assert_eq!(slot.height[10], 240.0);
    }

    #[test]
    fn kind_four_shrinks_the_top_sample_after_twenty_five_frames() {
        let mut state = blade(4);
        state.slots[0].process = 25;
        state.slots[0].height[10] = 20.0;

        on_process(&mut state, &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.process, 26);
        // The drain loop only walks samples 1..=9, then
        // `ShrinkLast(10, 1.01)` removes five from the top sample and grows
        // the ring by 1%.
        assert_eq!(slot.height[10], 15.0);
        assert!((slot.distance - 18.0 * 1.01).abs() < 1e-4);
    }

    #[test]
    fn kind_zero_only_shrinks_after_frame_one_forty() {
        let mut state = blade(0);
        state.slots[0].process = 25;
        state.slots[0].height[10] = 20.0;

        on_process(&mut state, &MasterState::default());

        // Still draining samples 1..=9, so the top sample is untouched yet.
        // The first fifty frames also pull the ring in by 0.1.
        assert_eq!(state.slots[0].height[10], 20.0);
        assert!((state.slots[0].distance - 17.9).abs() < 1e-5);

        state.slots[0].process = 140;
        state.slots[0].height[10] = 20.0;
        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].height[10], 15.0);
        assert!((state.slots[0].distance - 17.9 * 1.01).abs() < 1e-4);
    }

    #[test]
    fn distance_drain_stops_at_frame_fifty_and_for_kind_four() {
        let mut state = blade(0);
        state.slots[0].process = 48;

        on_process(&mut state, &MasterState::default());
        assert!((state.slots[0].distance - 17.9).abs() < 1e-5);

        on_process(&mut state, &MasterState::default());
        assert!((state.slots[0].distance - 17.9).abs() < 1e-5);

        let mut state = blade(4);
        state.slots[0].process = 10;
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].distance, 18.0);
    }

    #[test]
    fn only_kind_one_follows_the_master() {
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };

        let mut state = blade(1);
        on_process(&mut state, &master);
        assert_eq!(state.position, [1.0, 2.0, 3.0]);

        let mut state = blade(0);
        on_process(&mut state, &master);
        assert_eq!(state.position, [0.0, 0.0, 0.0]);
    }
}
