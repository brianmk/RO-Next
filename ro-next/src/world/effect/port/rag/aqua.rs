//! `EF_AQUA` - inline body in `CRagEffect::OnProcess`,
//! `RagEffect.cpp:23363-23375`. Lifetime 100 frames.
//!
//! One `성수뜨기` sprite particle above the caster, non-repeating.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct Aqua {
    spawned: bool,
}

impl PortedEffect for Aqua {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\EF_Aqua.wav");

        let lifetime = runtime.lifetime;
        let particle = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(particle);

        state.duration = lifetime;
        state.delta_pos2[1] = -20.0;
        state.anim_speed = 2;
        state.repeat_anim = 0;
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn spawns_one_non_repeating_sprite() {
        let mut effect = Aqua::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 121);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Aqua.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Particle3d);
        assert_eq!(state.duration, 100);
        assert_eq!(state.delta_pos2[1], -20.0);
        assert_eq!(state.anim_speed, 2);
        assert_eq!(state.repeat_anim, 0);
    }
}
