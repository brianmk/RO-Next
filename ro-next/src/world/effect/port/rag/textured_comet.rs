//! `CRagEffect::TexturedComet(name, kind)` - `RagEffect.cpp:5926-5978`.
//!
//! `EF_SMA3`: an upward comet particle cluster on the target position with a
//! two-texture **pair** selected by `kind`.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedComet {
    kind: i32,
    lifetime: i32,
}

impl TexturedComet {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }

    fn textures(&self) -> (&'static str, &'static str) {
        match self.kind {
            1 => (
                "effect\\thunder_ball_0002.bmp",
                "effect\\thunder_ball_0003.bmp",
            ),
            2 => ("effect\\meteo_comet_b.bmp", "effect\\meteo_comet_c.bmp"),
            _ => ("effect\\pok1.tga", "effect\\pok3.tga"),
        }
    }

    fn flag(&self) -> i8 {
        match self.kind {
            0 => 11,
            1 => 1,
            2 => 7,
            _ => 0,
        }
    }
}

impl PortedEffect for TexturedComet {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let target = runtime.target_offset;
        let (first, second) = self.textures();
        let flag = self.flag();

        // Per-slot RNG order: distance, x, z, speed.
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut speeds = [0.0f32; 4];

        for index in 0..4usize {
            distances[index] = runtime.rng.clamped_random(10) as f32 * 0.08 + 1.2;
            xs[index] = runtime.rng.clamped_random(9) as f32 + target[0] - 4.0;
            zs[index] = runtime.rng.clamped_random(9) as f32 + target[2] - 4.0;
            speeds[index] = runtime.rng.clamped_random(20) as f32 * 0.01 + 0.1;
        }

        let comet = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(comet);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(first);
        state.texture_secondary = Some(second);
        state.size = 0.0;
        state.position = target;

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.distance = distances[index];
            slot.height[8] = 0.0;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = xs[index];
            slot.vec_b_now[2] = zs[index];
            slot.vec_b_now[1] = target[1] - 6.0;
            slot.rot_start = 0;
            slot.height[1] = 0.0;
            slot.height[2] = speeds[index];
            slot.full_display_angle = 0;
            slot.flag1[0] = 0;
            slot.flag1[1] = 1;
            slot.flag1[2] = flag;
            slot.height[7] = 0.0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn sma3_spawns_the_thunder_comet() {
        let mut effect = TexturedComet::new(1, 50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 561);
        runtime.target_offset = [5.0, 0.0, 5.0];

        step(&mut effect, &mut runtime);

        let comet = &runtime.primitives[0].state;
        assert_eq!(comet.count, PrimitiveId::ParticleUp);
        assert_eq!(comet.duration, 50);
        assert_eq!(comet.render_flag, 5);
        assert_eq!(comet.texture, Some("effect\\thunder_ball_0002.bmp"));
        assert_eq!(
            comet.texture_secondary,
            Some("effect\\thunder_ball_0003.bmp")
        );
        assert!(comet.slots.iter().all(|slot| slot.life == 1));
        assert!(comet.slots.iter().all(|slot| slot.flag1[2] == 1));
        assert!(comet.slots.iter().all(|slot| slot.flag1[1] == 1));
        assert!(comet.slots[0].vec_b_now[0] >= 1.0 && comet.slots[0].vec_b_now[0] <= 9.0);
    }

    #[test]
    fn other_kinds_pick_their_own_textures() {
        let mut effect = TexturedComet::new(2, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 561);
        step(&mut effect, &mut runtime);
        assert_eq!(
            runtime.primitives[0].state.texture,
            Some("effect\\meteo_comet_b.bmp")
        );
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 7);

        let mut effect = TexturedComet::new(0, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 561);
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\pok1.tga"));
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 11);
    }
}
