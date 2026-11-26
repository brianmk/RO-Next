//! `CRagEffect::VioletRingCloud(type)` - `RagEffect.cpp:15364-15577`.
//!
//! A three-stage portal: violet/red columns (prim 60) on frame 0, ripple rings
//! (prim 63) on frame 1, and the `cloud11.tga` wind crown (prim 62) on frame 2.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct VioletRingCloud {
    cloud_type: i32,
    lifetime: i32,
}

impl VioletRingCloud {
    pub fn new(cloud_type: i32, lifetime: i32) -> Self {
        Self { cloud_type, lifetime }
    }

    fn spawn_columns(&self, runtime: &mut EffectRuntime) {
        let cloud_type = self.cloud_type;
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

        if cloud_type == 3 {
            state.texture = Some("effect\\ring_red.tga");
            state.size = 10.0;
        } else {
            state.texture = Some("effect\\magic_violet.tga");
            state.size = 4.0;
        }

        for index in 0..3usize {
            let distance = match index {
                0 => 4.0,
                1 => 3.0,
                _ => 2.0,
            };

            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = 50.0;
            slot.rot_start = rots[index];
            slot.alpha_b = 0;
            slot.distance = distance;
            slot.rise_angle = 90;
            slot.alpha_t = 1400;
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [1; 21];
        }

        state.slots[3].life = 0;
    }

    fn spawn_ripples(&self, runtime: &mut EffectRuntime) {
        let cloud_type = self.cloud_type;
        let lifetime = runtime.lifetime;

        let ripples = runtime.launch(PrimitiveId::RippleRings, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ripples);

        state.render_flag = 1 | 4;
        state.duration = lifetime;

        if cloud_type == 3 {
            state.texture = Some("effect\\ring_red.tga");
            state.size = 10.0;
        } else {
            state.texture = Some("effect\\ring_blue.tga");
            state.size = 4.0;
        }

        for index in 0..3usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = match index {
                0 => 0,
                1 => {
                    if cloud_type != 3 {
                        -10
                    } else {
                        -20
                    }
                }
                _ => {
                    if cloud_type != 3 {
                        -20
                    } else {
                        -40
                    }
                }
            };
            slot.full_display_angle = 360;
            slot.max_height = 6.001;
            slot.rot_start = index as i16 * 25;
            slot.alpha_b = 0;
            slot.distance = 0.0;
            slot.rise_angle = (index as i16) + 2;
            slot.alpha_t = 3111;
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [1; 21];
        }

        state.slots[3].life = 0;
        state.slots[3].process = 0;
        state.slots[3].alpha_t = cloud_type as i16;
    }

    fn spawn_crown(&self, runtime: &mut EffectRuntime) {
        let cloud_type = self.cloud_type;
        let lifetime = runtime.lifetime;

        let crown = runtime.launch(PrimitiveId::Wind, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(crown);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some("effect\\cloud11.tga");
        state.size = if cloud_type == 3 { 10.0 } else { 4.0 };

        const MAX_HEIGHTS: [f32; 4] = [1.5, 3.5, 5.5, 7.5];
        const DISTANCES: [f32; 4] = [9.0, 7.5, 6.0, 4.5];

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
            slot.alpha_t = 0;
            slot.height = [1.5; 21];
            slot.flag1 = [0; 21];
        }
    }
}

impl PortedEffect for VioletRingCloud {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        match runtime.frame {
            0 => self.spawn_columns(runtime),
            1 => self.spawn_ripples(runtime),
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
    fn portal2_runs_its_three_stages() {
        let mut effect = VioletRingCloud::new(0, 9999);
        let mut runtime = EffectRuntime::new(9999, [0.0, 0.0], 251);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);
        let columns = &runtime.primitives[0].state;
        assert_eq!(columns.count, PrimitiveId::PulsingColumns);
        assert_eq!(columns.texture, Some("effect\\magic_violet.tga"));
        assert_eq!(columns.size, 4.0);
        assert!(columns.slots.iter().take(3).all(|slot| slot.flag1[0] == 1));
        assert_eq!(columns.slots[0].distance, 4.0);
        assert_eq!(columns.slots[2].distance, 2.0);
        assert_eq!(columns.slots[3].life, 0);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);
        let ripples = &runtime.primitives[1].state;
        assert_eq!(ripples.count, PrimitiveId::RippleRings);
        assert_eq!(ripples.texture, Some("effect\\ring_blue.tga"));
        // Spawned at -10/-20, one ripple step has advanced them.
        assert_eq!(ripples.slots[1].process, -9);
        assert_eq!(ripples.slots[2].process, -19);
        assert_eq!(ripples.slots[3].alpha_t, 0);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 3);
        let crown = &runtime.primitives[2].state;
        assert_eq!(crown.count, PrimitiveId::Wind);
        assert_eq!(crown.texture, Some("effect\\cloud11.tga"));
        assert_eq!(crown.slots[0].max_height, 1.5);
        assert_eq!(crown.slots[3].max_height, 7.5);
        // Spawned at 270, the wind primitive rotated it by 5.
        assert_eq!(crown.slots[3].rot_start, 275);
        assert_eq!(crown.slots[3].distance, 4.5);
    }

    #[test]
    fn portal3_uses_the_red_large_variant() {
        let mut effect = VioletRingCloud::new(3, 9999);
        let mut runtime = EffectRuntime::new(9999, [0.0, 0.0], 251);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\ring_red.tga"));
        assert_eq!(runtime.primitives[0].state.size, 10.0);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[1].state.texture, Some("effect\\ring_red.tga"));
        assert_eq!(runtime.primitives[1].state.slots[1].process, -19);
        assert_eq!(runtime.primitives[1].state.slots[3].alpha_t, 3);
    }
}
