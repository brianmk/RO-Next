//! `CRagEffect::Vallentine(kind)` - `RagEffect.cpp:8741-8779`.
//!
//! The Valentine/NY-item sprite effects: one animated sprite on frame 0 with a
//! kind-selected asset, `animSpeed 2` (4 for the fast variants), `repeatAnim 0`
//! and the `0x200` pattern flag.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Vallentine {
    kind: i32,
    lifetime: i32,
}

impl Vallentine {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }

    fn assets(&self) -> (&'static str, &'static str, i32) {
        match self.kind {
            2 => ("이팩트\\05vallentine.spr", "이팩트\\05vallentine.act", 2),
            3 => ("이팩트\\fast.spr", "이팩트\\fast.act", 4),
            4 => ("이팩트\\ro2effect.spr", "이팩트\\ro2effect.act", 4),
            _ => ("이팩트\\vallentine.spr", "이팩트\\vallentine.act", 2),
        }
    }
}

impl PortedEffect for Vallentine {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        runtime.play_sound("effect\\vallentine.wav");

        let lifetime = runtime.lifetime;
        let (sprite, action, anim_speed) = self.assets();

        let effect = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(effect);

        state.pattern |= 0x200;
        state.duration = lifetime;
        state.delta_pos2[1] = 0.0;
        state.anim_speed = anim_speed;
        state.repeat_anim = 0;
        state.sprite = Some(sprite);
        state.action = Some(action);
        state.action_index = if self.kind == 2 || self.kind == 3 || self.kind == 4 {
            0
        } else {
            self.kind
        };

        if self.kind == 4 {
            state.render_flag |= 0x200;
        }

        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn valentine_uses_the_classic_sprite() {
        let mut effect = Vallentine::new(0, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 601);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\vallentine.wav".to_owned()]);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Particle3d);
        assert_eq!(state.pattern & 0x200, 0x200);
        assert_eq!(state.duration, 100);
        assert_eq!(state.anim_speed, 2);
        assert_eq!(state.repeat_anim, 0);
        assert_eq!(state.sprite, Some("이팩트\\vallentine.spr"));
        assert_eq!(state.action, Some("이팩트\\vallentine.act"));
        assert_eq!(state.action_index, 0);
    }

    #[test]
    fn fast_variant_is_faster_and_different() {
        let mut effect = Vallentine::new(3, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 601);
        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.anim_speed, 4);
        assert_eq!(state.sprite, Some("이팩트\\fast.spr"));
        assert_eq!(state.action_index, 0);
    }

    #[test]
    fn ro2_year_sets_the_render_flag() {
        let mut effect = Vallentine::new(4, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 601);
        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.render_flag & 0x200, 0x200);
        assert_eq!(state.sprite, Some("이팩트\\ro2effect.spr"));
        assert_eq!(state.anim_speed, 4);
    }
}
