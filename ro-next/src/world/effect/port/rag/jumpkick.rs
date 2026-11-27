//! `EF_JUMPKICK` - `RagEffect.cpp:24591-24604`.
//!
//! No primitive: the effect only drives the caster. On frame 5 it plays
//! `effect\T_날라차기.wav`, sets `parent->render.state = 27` and calls
//! `SetFrameSequence(0x60, 5, 5, 0x60, 5, 1, 25, 0)`; on frame 35 it restores
//! `parent->render.state = 2` when the state is still 27.
//!
//! Lifetime 100 (`EffectTypes.cpp:466`).

use super::super::runtime::{EffectRuntime, ParentFrameSequence, PortedEffect};

#[derive(Debug)]
pub struct JumpKick {
    lifetime: i32,
}

impl JumpKick {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for JumpKick {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 5 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\T_날라차기.wav");
            runtime.set_parent_state(27);
            runtime.set_parent_frame_sequence(ParentFrameSequence {
                animation: 0x60,
                first_frame: 5,
                last_frame: 5,
                finish_animation: 0x60,
                finish_frame: 5,
                timed: 1,
                duration: 25,
                loop_: 0,
            });
        }

        if runtime.frame == 35 && runtime.parent_state == Some(27) {
            runtime.set_parent_state(2);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn frame_five_plays_the_kick_and_arms_the_sequence() {
        let mut body = JumpKick::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);

        for _ in 0..6 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.sounds, vec!["effect\\T_날라차기.wav".to_string()]);
        assert_eq!(runtime.parent_state, Some(27));
        assert_eq!(
            runtime.parent_frame_sequence,
            Some(ParentFrameSequence {
                animation: 0x60,
                first_frame: 5,
                last_frame: 5,
                finish_animation: 0x60,
                finish_frame: 5,
                timed: 1,
                duration: 25,
                loop_: 0,
            })
        );
    }

    #[test]
    fn frame_thirty_five_restores_the_idle_state_only_from_twenty_seven() {
        let mut body = JumpKick::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.frame = 35;
        runtime.parent_state = Some(27);

        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_state, Some(2));

        runtime.frame = 35;
        runtime.parent_state = Some(9);
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_state, Some(9));
    }
}
