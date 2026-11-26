//! `EF_FIREPILLARON2` dispatch block - `RagEffect.cpp:20741-20746`, built from
//! `CRagEffect::RingStack` (`:2861-2882`) and three `TexturedSpreadFlat` calls.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::spread_flat;

#[derive(Debug)]
pub struct FirePillar {
    lifetime: i32,
}

impl FirePillar {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// `CRagEffect::RingStack(white)`.
fn ring_stack(runtime: &mut EffectRuntime, white: bool) {
    let lifetime = runtime.lifetime;
    let frame = runtime.frame;
    let texture = if white { "effect\\ring_white.tga" } else { "effect\\magic_red.tga" };

    for index in 1..=3i32 {
        let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = lifetime;
        state.long_speed = -3.0;
        state.outer_size = (index + 3) as f32;
        state.inner_size = (index as f32) + 1.5;
        state.height_size = ((8 - 2 * index) * (5 - index) + 1) as f32 + 2.5;
        state.roll_speed = -0.5;
        state.alpha = (frame * -7 + 240) as f32;
        state.texture = Some(texture);
    }
}

impl PortedEffect for FirePillar {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        ring_stack(runtime, true);

        for _ in 0..3 {
            spread_flat::spawn_one(runtime, "effect\\ring_white.tga", 11);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn fire_pillar_stacks_three_rings_and_three_flat_rings() {
        let mut effect = FirePillar::new(90);
        let mut runtime = EffectRuntime::new(90, [0.0, 0.0], 311);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 6);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::Cylinder3d);
        assert_eq!(first.duration, 90);
        assert_eq!(first.long_speed, -3.0);
        assert_eq!(first.outer_size, 4.0);
        assert_eq!(first.inner_size, 2.5);
        assert_eq!(first.roll_speed, -0.5);
        assert_eq!(first.alpha, 240.0);
        assert_eq!(first.texture, Some("effect\\ring_white.tga"));

        assert_eq!(runtime.primitives[2].state.outer_size, 6.0);
        // (8 - 6) * 2 + 1 + 2.5 = 7.5
        assert_eq!(runtime.primitives[2].state.height_size, 7.5);

        let flat = &runtime.primitives[3].state;
        assert_eq!(flat.count, PrimitiveId::HeightRamp);
        assert_eq!(flat.texture, Some("effect\\ring_white.tga"));
        assert_eq!(flat.size, 5.0);
    }
}
