//! Prim 46 - the sprite primitive of `CHIMTO`/`FVOICE`/`WINK`
//! (`EF_CHIMTO`, `EF_FVOICE`, `EF_WINK`).
//!
//! `EffectPrimId.h` has no symbol between `PP_TEIHIT2` (45) and `PP_BOTTOM`
//! (47); the official `OnProcess` switch handles it with a bare `case 46`
//! (`EffectPrimitiveMotion.cpp:9741-9760`), and `CEffectPrim::Render` draws it
//! with the shared `RenderSegmentSprites2` sprite pass
//! (`EffectPrimitiveRendering.cpp:272-281`), which the port already implements
//! in [`super::super::sprite_pass`].
//!
//! The motion body is short and unusual: it re-anchors the primitive at the
//! master's position, then *replaces* that anchor with the primitive's
//! `deltaPos` (which `CRagEffect::CHIMTO` seeds from `slots[0].vecBPre`, i.e.
//! the effect's target offset). So the `CHIMTO` sprite renders on the target,
//! not on the caster. It then runs the shared `ProcessAlpha`/`ShiftSegment`
//! tail and advances the `.act` frame with `AnimMotion`.
//!
//! The `AnimMotion` return value is **discarded** by the official case (the
//! call is a plain statement, not `if (!this->AnimMotion()) return false;`),
//! so the primitive never dies from a finished non-repeating action - it lives
//! until `stateCnt > duration` like every other primitive.

use super::particle3d::anim_motion;
use super::{EffectPrimitiveState, MasterState};

/// One frame of the official `case 46`
/// (`EffectPrimitiveMotion.cpp:9741-9760`).
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) -> bool {
    // `const vector3d& parent = this->state.master->render.position;
    //  this->render.position = parent;`
    state.position = master.position;

    // `if (this->state.slots[0].life == 1) this->state.deltaPos =
    //  this->state.slots[0].vecBPre;`
    if state.slots[0].life == 1 {
        state.delta_pos = state.slots[0].vec_b_pre;
    }

    // `this->render.position = this->state.deltaPos;`
    state.position = state.delta_pos;

    // `if ((this->state.pattern & 0x20u) != 0) this->state.roll =
    //  this->render.angle - camera->camera.yaw;`
    if (state.pattern & 0x20u32) != 0 {
        state.roll = state.render_angle - master.camera_yaw;
    }

    state.process_alpha(master);
    state.shift_segment();

    // Quirk kept: the official case ignores `AnimMotion`'s return value.
    let _ = anim_motion(state);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveState, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::ChimtoSprite,
            duration: 16,
            anim_speed: 2,
            action_frame_count: 3,
            repeat_anim: 0,
            ..Default::default()
        }
    }

    #[test]
    fn anchors_on_the_slot_target_not_the_master() {
        let master = MasterState {
            position: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].vec_b_pre = [1.0, 2.0, 3.0];

        assert!(on_process(&mut state, &master));

        assert_eq!(state.position, [1.0, 2.0, 3.0]);
        assert_eq!(state.delta_pos, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn without_a_live_slot_the_previous_delta_pos_is_kept() {
        let master = MasterState {
            position: [10.0, 20.0, 30.0],
            ..Default::default()
        };
        let mut state = state();
        state.delta_pos = [4.0, 5.0, 6.0];
        state.slots[0].vec_b_pre = [9.0, 9.0, 9.0];

        assert!(on_process(&mut state, &master));

        assert_eq!(state.position, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn a_finished_action_does_not_retire_the_primitive() {
        let master = MasterState::default();
        let mut state = state();
        state.slots[0].life = 1;
        state.anim_speed = 1;
        state.action_frame_count = 1;
        state.state_cnt = 0;

        // `AnimMotion` returns false once `curMotion == frameCount`, but the
        // official case ignores it.
        assert!(on_process(&mut state, &master));
        assert_eq!(state.cur_motion, 1);
        assert!(on_process(&mut state, &master));
        assert_eq!(state.cur_motion, 2);
    }
}
