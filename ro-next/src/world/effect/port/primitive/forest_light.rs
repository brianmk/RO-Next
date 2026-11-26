//! prim 66 (`ForestLight`) - spawned by `CRagEffect::ForestLight`
//! (`RagEffect.cpp:14597-14664`).
//!
//! The original has no motion case for this primitive: the slots are set up by
//! the effect body and the renderer draws them (facing slots). The port keeps
//! that shape - setup only, no per-frame arithmetic.
//!
//! `FollowMaster` is implicit: the body copies `render.position` from the
//! parent when the effect spawns.

use super::{EffectPrimitiveState, MasterState};

/// The original's motion dispatch has no `case 66`; only the position follow
/// applies, which the body already performs at spawn time.
pub fn on_process(_state: &mut EffectPrimitiveState) {}

/// Keeps the master position in sync for renderers that need it.
pub fn follow(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    #[test]
    fn follow_copies_the_master_position() {
        let mut state = EffectPrimitiveState::default();
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };

        follow(&mut state, &master);
        assert_eq!(state.position, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn motion_is_a_no_op() {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::ForestLight,
            duration: 100,
            ..Default::default()
        };

        on_process(&mut state);

        assert_eq!(state.state_cnt, 0);
    }
}
