//! `CRagEffect::TexturedHalo(name, kind)` - `RagEffect.cpp:4839-4896`.
//!
//! The persistent ground halo under the "bottom" effects and Forest Light.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedHalo {
    texture: &'static str,
    kind: i32,
    lifetime: i32,
}

impl TexturedHalo {
    pub fn new(texture: &'static str, kind: i32, lifetime: i32) -> Self {
        Self {
            texture,
            kind,
            lifetime,
        }
    }
}

impl PortedEffect for TexturedHalo {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let kind = self.kind;
        let lifetime = runtime.lifetime;
        let rise = runtime.rng.clamped_random(360) as i16;

        let halo = runtime.launch(PrimitiveId::Halo, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(halo);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(self.texture);

        state.slots[0].life = 1;
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;

        state.slots[0].distance = match kind {
            2 => 12.0,
            3 => 10.0,
            4 => 12.0,
            10 => 30.0,
            11 => 28.0,
            12 => 20.0,
            _ => 7.0,
        };

        if kind == 3 {
            state.slots[0].rot_start = 225;
            state.slots[0].alpha_b = 250;
            state.size = 111.0;
        } else if kind == 4 {
            state.slots[0].rot_start = 0;
            state.slots[0].alpha_b = 200;
            state.size = 4.0;
        } else if kind == 10 || kind == 11 || kind == 12 {
            state.slots[0].rot_start = 0;
            state.slots[0].alpha_b = 200;
            state.size = 10.0;
        } else {
            state.slots[0].rot_start = 0;
            state.slots[0].alpha_b = 100;
            state.size = 5.0;
        }

        state.slots[0].rise_angle = rise;
        state.slots[0].height[0] = -2.0;
        state.slots[0].height[4] = kind as f32;

        if kind == 10 || kind == 11 || kind == 12 {
            state.slots[0].rise_angle = 360;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn runner_spawns_the_hanmoon_halo() {
        let mut effect = TexturedHalo::new("effect\\hanmoon1.tga", 3, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 431);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let halo = &runtime.primitives[0].state;
        assert_eq!(halo.count, PrimitiveId::Halo);
        assert_eq!(halo.duration, 29999);
        assert_eq!(halo.render_flag, 5);
        assert_eq!(halo.texture, Some("effect\\hanmoon1.tga"));
        assert_eq!(halo.size, 111.0);
        assert_eq!(halo.slots[0].distance, 10.0);
        assert_eq!(halo.slots[0].alpha_b, 250);
        assert_eq!(halo.slots[0].height[4], 3.0);
        assert_eq!(halo.slots[1].life, 0);
    }

    #[test]
    fn spider_uses_the_slow_orbit() {
        let mut effect = TexturedHalo::new("effect\\spiderweb.tga", 2, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 431);

        step(&mut effect, &mut runtime);

        let halo = &runtime.primitives[0].state;
        assert_eq!(halo.slots[0].distance, 12.0);
        // Random spawn angle advanced by one (kind 2).
        assert!((1..=359).contains(&halo.slots[0].rise_angle));
        assert_eq!(halo.size, 5.0);
    }

    #[test]
    fn kind_ten_uses_the_wide_pulse() {
        let mut effect = TexturedHalo::new("effect\\aaa copy.bmp", 10, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 431);

        step(&mut effect, &mut runtime);

        let halo = &runtime.primitives[0].state;
        assert_eq!(halo.size, 10.0);
        // Spawned at 360, normalised to 0 by the motion step.
        assert_eq!(halo.slots[0].rise_angle, 0);
        assert!(halo.slots[0].distance < 30.0, "kind 10 pulls in below 181");
    }
}
