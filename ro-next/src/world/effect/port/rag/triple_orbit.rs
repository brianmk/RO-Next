//! `CRagEffect::TexturedTripleOrbit(name)` - `RagEffect.cpp:7123-7183`.
//!
//! Gradius Laser: three orbiting texture columns on `PP_PULSING_COLUMNS` (60)
//! with `maxHeight` 74/77/80 and distance 1.3/1.0/0.7.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TripleOrbit {
    texture: &'static str,
    lifetime: i32,
}

impl TripleOrbit {
    pub fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

impl PortedEffect for TripleOrbit {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;

        let rots = [
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
        ];

        let orbit = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(orbit);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.size = 16.0;

        const MAX_HEIGHTS: [f32; 3] = [74.0, 77.0, 80.0];
        const DISTANCES: [f32; 3] = [1.3, 1.0, 0.7];

        for index in 0..3usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = MAX_HEIGHTS[index];
            slot.rot_start = rots[index];
            slot.alpha_b = 0;
            slot.distance = DISTANCES[index];
            slot.rise_angle = 90;
            slot.alpha_t = 3080;
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [0; 21];
        }

        state.slots[3].life = 0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn gradivs_laser_spawns_three_orbits() {
        let mut effect = TripleOrbit::new("effect\\white02.bmp", 500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 321);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let orbit = &runtime.primitives[0].state;
        assert_eq!(orbit.count, PrimitiveId::PulsingColumns);
        assert_eq!(orbit.duration, 500);
        assert_eq!(orbit.size, 16.0);
        assert_eq!(orbit.render_flag, 5);
        assert_eq!(orbit.texture, Some("effect\\white02.bmp"));
        assert_eq!(orbit.slots[0].max_height, 74.0);
        assert_eq!(orbit.slots[1].max_height, 77.0);
        assert_eq!(orbit.slots[2].max_height, 80.0);
        assert_eq!(orbit.slots[0].distance, 1.3);
        assert_eq!(orbit.slots[2].distance, 0.7);
        assert!(orbit.slots.iter().take(3).all(|slot| slot.alpha_t == 3080));
        assert_eq!(orbit.slots[3].life, 0);
    }
}
