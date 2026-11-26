//! `CRagEffect::ThunderSparks` - `RagEffect.cpp:5430-5465`.
//!
//! Sprinkle Sand: the dispatch block calls it 20 times on frame 0, each call
//! spawning one `PP_PARTICLE_UP` (59) batch of four `thunder_center.bmp` sparks
//! that pop around the caster.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct SprinkleSand {
    count: usize,
    lifetime: i32,
}

impl SprinkleSand {
    pub fn new(count: usize, lifetime: i32) -> Self {
        Self { count, lifetime }
    }
}

impl PortedEffect for SprinkleSand {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let position = runtime.position;

        for _ in 0..self.count {
            // Per-slot random draws, in slot order.
            let mut processes = [0i32; 4];
            let mut distances = [0.0f32; 4];
            let mut xs = [0.0f32; 4];
            let mut zs = [0.0f32; 4];
            let mut rises = [0.0f32; 4];
            let mut speeds = [0.0f32; 4];

            for index in 0..4usize {
                processes[index] = runtime.rng.clamped_random(10) - 10;
                distances[index] = runtime.rng.clamped_random(16) as f32 * 0.03 + 0.1;
                xs[index] = runtime.rng.clamped_random(9) as f32 + position[0] - 4.0;
                zs[index] = runtime.rng.clamped_random(9) as f32 + position[2] - 4.0;
                rises[index] = runtime.rng.clamped_random(10) as f32 * 0.01;
                speeds[index] = runtime.rng.clamped_random(40) as f32 * 0.01 + 0.2;
            }

            let sparks = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(sparks);

            state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
            state.duration = lifetime;
            state.texture = Some("effect\\thunder_center.bmp");
            state.size = 0.0;
            state.position = position;

            for index in 0..4usize {
                let slot = &mut state.slots[index];

                slot.life = 1;
                slot.process = processes[index];
                slot.distance = distances[index];
                slot.height[8] = 0.0;
                slot.alpha_b = 0;
                slot.vec_b_now[0] = xs[index];
                slot.vec_b_now[2] = zs[index];
                slot.vec_b_now[1] = position[1];
                slot.rot_start = 0;
                slot.height[1] = rises[index];
                slot.height[2] = speeds[index];
                slot.full_display_angle = 0;
                slot.flag1[0] = 0;
                slot.flag1[1] = 0;
                slot.flag1[2] = 10;
                slot.height[7] = 0.0;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn twenty_spark_batches_spawn_on_frame_zero() {
        let mut effect = SprinkleSand::new(20, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 291);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 20);

        let sparks = &runtime.primitives[0].state;
        assert_eq!(sparks.count, PrimitiveId::ParticleUp);
        assert_eq!(sparks.duration, 300);
        assert_eq!(sparks.texture, Some("effect\\thunder_center.bmp"));
        assert!(sparks.slots.iter().all(|slot| slot.life == 1));
        assert!(sparks.slots.iter().all(|slot| slot.flag1[2] == 10));

        for slot in &sparks.slots {
            assert!(slot.process >= -10 && slot.process <= -1);
            assert!((0.1..0.6).contains(&slot.distance));
            assert!((0.2..0.62).contains(&slot.height[2]));
        }
    }

    #[test]
    fn no_further_batches_after_frame_zero() {
        let mut effect = SprinkleSand::new(20, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 291);

        step(&mut effect, &mut runtime);
        let count = runtime.primitives.len();

        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), count);
    }
}
