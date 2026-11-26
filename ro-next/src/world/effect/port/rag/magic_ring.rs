//! `CRagEffect::TexturedMagicRing(type)` (the `char` overload) -
//! `RagEffect.cpp:15565-15703`.
//!
//! Big Portal: a three-column magic ring (prim 60, `magic_violet.tga` /
//! `ring_red.tga`) plus the `cloud11.tga` crown (prim 62) on frame 2.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct MagicRing {
    ring_type: i32,
    lifetime: i32,
}

impl MagicRing {
    pub fn new(ring_type: i32, lifetime: i32) -> Self {
        Self { ring_type, lifetime }
    }

    fn spawn_columns(&self, runtime: &mut EffectRuntime) {
        let ring_type = self.ring_type;
        let lifetime = runtime.lifetime;

        let rots = [
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
        ];

        let columns = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(columns);

        state.render_flag = 1 | 4;
        state.duration = lifetime;

        if ring_type == 3 {
            state.texture = Some("effect\\ring_red.tga");
            state.size = 10.0;
        } else {
            state.texture = Some("effect\\magic_violet.tga");
            state.size = 4.0;
        }

        const DISTANCES: [f32; 3] = [7.0, 5.5, 4.0];

        for index in 0..3usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = 80.0;
            slot.rot_start = rots[index];
            slot.alpha_b = 0;
            slot.distance = DISTANCES[index];
            slot.rise_angle = 90;
            slot.alpha_t = if ring_type == 1 { -1 } else { 1400 };
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [3; 21];
        }

        state.slots[3].life = 0;
    }

    fn spawn_crown(&self, runtime: &mut EffectRuntime) {
        let ring_type = self.ring_type;
        let lifetime = runtime.lifetime;

        let crown = runtime.launch(PrimitiveId::Wind, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(crown);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some("effect\\cloud11.tga");
        state.size = if ring_type == 3 { 10.0 } else { 4.0 };

        const MAX_HEIGHTS: [f32; 4] = [2.5, 5.0, 7.5, 10.0];
        const DISTANCES: [f32; 4] = [13.0, 11.0, 9.0, 8.0];

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 30;
            slot.max_height = MAX_HEIGHTS[index];
            slot.rot_start = index as i16 * 90;
            slot.alpha_b = 0;
            slot.distance = DISTANCES[index];
            slot.rise_angle = 90;
            slot.alpha_t = 3;
            slot.height = [1.5; 21];
            slot.flag1 = [0; 21];
        }
    }
}

impl PortedEffect for MagicRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        match runtime.frame {
            0 => self.spawn_columns(runtime),
            2 => self.spawn_crown(runtime),
            _ => {}
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn big_portal_opens_with_columns_and_crown() {
        let mut effect = MagicRing::new(0, 2000);
        let mut runtime = EffectRuntime::new(2000, [0.0, 0.0], 261);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        let columns = &runtime.primitives[0].state;
        assert_eq!(columns.count, PrimitiveId::PulsingColumns);
        assert_eq!(columns.texture, Some("effect\\magic_violet.tga"));
        assert_eq!(columns.size, 4.0);
        assert_eq!(columns.slots[0].max_height, 80.0);
        assert_eq!(columns.slots[0].distance, 7.0);
        assert_eq!(columns.slots[2].distance, 4.0);
        assert_eq!(columns.slots[0].alpha_t, 1400);
        assert!(columns.slots.iter().take(3).all(|slot| slot.flag1[0] == 3));

        // Frame 1 does nothing; frame 2 adds the crown.
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        let crown = &runtime.primitives[1].state;
        assert_eq!(crown.count, PrimitiveId::Wind);
        assert_eq!(crown.texture, Some("effect\\cloud11.tga"));
        assert_eq!(crown.slots[0].max_height, 2.5);
        assert_eq!(crown.slots[0].distance, 13.0);
        assert_eq!(crown.slots[3].max_height, 10.0);
        assert_eq!(crown.slots[3].distance, 8.0);
        assert!(crown.slots.iter().all(|slot| slot.alpha_t == 3));
    }

    #[test]
    fn big_portal2_uses_negative_alpha_target() {
        let mut effect = MagicRing::new(1, 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 261);

        step(&mut effect, &mut runtime);

        assert!(runtime.primitives[0].state.slots.iter().take(3).all(|slot| slot.alpha_t == -1));
    }
}
