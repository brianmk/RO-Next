//! `CRagEffect::PortalRings` - `RagEffect.cpp:17145-17250`.
//!
//! `EF_ENTRY2`: four `ring_blue.tga` columns on `PP_PULSING_COLUMNS` with the
//! portal wave and the `flag1 = 2` slot kind.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct PortalRings {
    lifetime: i32,
}

impl PortalRings {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for PortalRings {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;

        runtime.play_sound("effect\\EF_Portal.wav");

        let rots = [
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
        ];

        let rings = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(rings);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some("effect\\ring_blue.tga");
        state.size = 4.0;

        const MAX_HEIGHTS: [f32; 4] = [30.0, 30.0, 4.0, 4.0];
        const DISTANCES: [f32; 4] = [3.7, 3.4, 3.6, 3.7];
        const RISE_ANGLES: [i16; 4] = [90, 90, 10, 5];

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = MAX_HEIGHTS[index];
            slot.rot_start = rots[index];
            slot.alpha_b = 0;
            slot.distance = DISTANCES[index];
            slot.rise_angle = RISE_ANGLES[index];
            slot.alpha_t = 1400;
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [2; 21];
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn entry2_spawns_four_blue_portal_rings() {
        let mut effect = PortalRings::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 371);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Portal.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let rings = &runtime.primitives[0].state;
        assert_eq!(rings.count, PrimitiveId::PulsingColumns);
        assert_eq!(rings.duration, 100);
        assert_eq!(rings.size, 4.0);
        assert_eq!(rings.texture, Some("effect\\ring_blue.tga"));

        assert_eq!(rings.slots[0].max_height, 30.0);
        assert_eq!(rings.slots[1].max_height, 30.0);
        assert_eq!(rings.slots[2].max_height, 4.0);
        assert_eq!(rings.slots[3].max_height, 4.0);
        assert_eq!(rings.slots[0].distance, 3.7);
        assert_eq!(rings.slots[1].distance, 3.4);
        assert_eq!(rings.slots[2].rise_angle, 10);
        assert_eq!(rings.slots[3].rise_angle, 5);
        assert!(rings.slots.iter().all(|slot| slot.flag1[0] == 2));
        assert!(rings.slots.iter().all(|slot| slot.alpha_t == 1400));
    }
}
