//! `PP_LINELINK` (38) - `CEffectPrim::PrimLineLink`,
//! `EffectPrimitiveMotion.cpp:1386-1441`; dispatch `case PP_LINELINK:`
//! `EffectPrimitiveMotion.cpp:9589-9590`.
//!
//! The soul-link tube of `CRagEffect::TrackTarget`/`TrackTargetWide`
//! (`RagEffect.cpp:3839-3929`), i.e. `EF_LINELINK2` and `EF_LINELINK3`. The
//! body arms one slot with the target account id in `height[0]`; this motion
//! re-resolves that account id to an actor position **every frame** and writes
//! it into `vecTPre`, while `vecTNow` trails the master by 8 units.
//!
//! Three kind branches shape the alpha (kind is `height[4]`):
//!
//! * `2.0` (`TrackTarget`): fade in by `+6` until process 20, out by `-3` after
//!   40,
//! * `3.0` (`TrackTargetWide`): `+1` alpha until process 80 and a `sin(process
//!   % 360) * 0.005` radius swell,
//! * anything else: a `+2`/`-2` blink on `process % 10 < 5`.
//!
//! `alphaB` is a `short` in the original and the blink branch has no clamp, so
//! the port uses wrapping arithmetic to keep the same overflow behaviour on
//! `EF_LINELINK3`'s ~10^8-frame lifetime.
//!
//! The target lookup `g_session.m_aid` / `GetGameActorByAID` is a scene query
//! the hermetic primitive pass cannot make. The port's spawn bridge stores the
//! *resolved target position* in the effect's `deltaPos` for every travelling
//! body (see `client/effects.rs`), so `master.master_delta_pos` is the actor
//! position the original's lookup would return - a spawn-time snapshot, since
//! the hermetic runtime has no live scene query; `slots[0].height[0] > 0` keeps
//! the original's "target id is set" gate. The renderer is
//! [`render_tube`](super::super::render_slots::render_tube) (`RenderTube`,
//! `EffectPrimitiveRendering.cpp:3214-3310`, dispatch
//! `EffectPrimitiveRendering.cpp:690-691`).

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimLineLink` (`EffectPrimitiveMotion.cpp:1386-1441`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.position = master.position;

    let slot = &mut state.slots[0];

    slot.process += 1;

    slot.vec_t_now = state.position;
    slot.vec_t_now[1] -= 8.0;

    if slot.height[4] == 2.0 {
        if slot.process > 40 {
            slot.alpha_b -= 3;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        } else if slot.process < 20 {
            slot.alpha_b += 6;
        }
    } else if slot.height[4] == 3.0 {
        let phase = slot.process % 360;

        if slot.process <= 80 {
            slot.alpha_b = slot.alpha_b.wrapping_add(1);
        }

        slot.max_height += angles.raw_sine(phase as usize) * 0.005;
    } else if slot.process % 10 < 5 {
        slot.alpha_b = slot.alpha_b.wrapping_add(2);
    } else {
        slot.alpha_b = slot.alpha_b.wrapping_sub(2);
    }

    // `if (height[0] > 0.0f)` gates the lookup; the two original branches
    // (local player vs `GetGameActorByAID`) both resolve to a position. The
    // port's `deltaPos` already carries that resolved target position (the
    // spawn bridge fills it for the travelling bodies).
    let target = if slot.height[0] > 0.0 {
        Some(master.master_delta_pos)
    } else {
        None
    };

    if let Some(target) = target {
        slot.vec_t_pre = target;
        slot.vec_t_pre[1] -= 8.0;
    } else {
        slot.life = 0;
        slot.alpha_b = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state(kind: f32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::LineLink,
            duration: 200,
            position: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            alpha_b: 0,
            max_height: 0.2,
            height: [0.0; 21],
            ..Default::default()
        };
        state.slots[0].height[0] = 5.0;
        state.slots[0].height[4] = kind;
        state
    }

    #[test]
    fn follows_the_master_and_resolves_the_target() {
        let angles = Angles::new();
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            master_delta_pos: [50.0, 60.0, 70.0],
            ..Default::default()
        };
        let mut state = state(2.0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position, [1.0, 2.0, 3.0]);
        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].vec_t_now, [1.0, -6.0, 3.0]);
        assert_eq!(state.slots[0].vec_t_pre, [50.0, 52.0, 70.0]);
        assert_eq!(state.slots[0].alpha_b, 6);
        assert_eq!(state.slots[0].life, 1);
    }

    #[test]
    fn a_missing_target_kills_the_slot() {
        let angles = Angles::new();
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        let mut state = state(2.0);
        state.slots[0].alpha_b = 12;
        // `height[0] > 0` is the original's "target resolved" gate; a zero
        // stands in for the null lookup.
        state.slots[0].height[0] = 0.0;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].life, 0);
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn a_non_positive_target_id_skips_the_lookup() {
        let angles = Angles::new();
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            master_delta_pos: [50.0, 60.0, 70.0],
            ..Default::default()
        };
        let mut state = state(2.0);
        state.slots[0].height[0] = 0.0;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn kind_two_fades_in_then_out() {
        let angles = Angles::new();
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        let mut state = state(2.0);
        state.slots[0].alpha_b = 100;

        state.slots[0].process = 18;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, 106);

        state.slots[0].process = 40;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, 103);

        state.slots[0].process = 45;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, 100);

        // Clamped at zero.
        state.slots[0].alpha_b = 1;
        state.slots[0].process = 60;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn kind_three_grows_the_radius_with_the_sine_table() {
        let angles = Angles::new();
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        let mut state = state(3.0);
        state.slots[0].max_height = 0.7;
        state.slots[0].process = 9;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_b, 1);
        assert!((state.slots[0].max_height - (0.7 + angles.raw_sine(10) * 0.005)).abs() < 1e-6);
    }

    #[test]
    fn the_default_kind_blinks_with_wrapping_arithmetic() {
        let angles = Angles::new();
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        let mut state = state(0.0);

        // process becomes 1 -> 1 % 10 < 5 -> +2.
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, 2);

        // process becomes 6 -> 6 % 10 >= 5 -> -2.
        state.slots[0].process = 5;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, 0);

        // The original's `short` wraps here; the port keeps the quirk.
        state.slots[0].alpha_b = i16::MAX;
        state.slots[0].process = 10;
        on_process(&mut state, &angles, &master);
        assert_eq!(state.slots[0].alpha_b, i16::MAX.wrapping_add(2));
    }
}
