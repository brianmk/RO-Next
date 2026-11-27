//! `CRagEffect::TexturedRisingOrbs(name, kind)` - `RagEffect.cpp:7187-7265`.
//!
//! `EF_HPTIME` (kind 0) and `EF_SPTIME` (kind 1): rising orb bursts every four
//! frames until frame 20, using the pok1/pok3 pair (or the passed texture for
//! kind 2).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct RisingOrbs {
    texture: &'static str,
    kind: i32,
    lifetime: i32,
}

impl RisingOrbs {
    pub fn new(texture: &'static str, kind: i32, lifetime: i32) -> Self {
        Self { texture, kind, lifetime }
    }
}

impl PortedEffect for RisingOrbs {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let kind = self.kind;

        if frame == 0 {
            match kind {
                0 => runtime.play_sound("_heal_effect.wav"),
                1 => runtime.play_sound("effect\\흡기.wav"),
                _ => {}
            }
        }

        if frame > 20 || frame % 4 != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let position = EffectRuntime::official_position(runtime.position);

        let (first, second) = if kind == 2 {
            (self.texture, self.texture)
        } else {
            ("effect\\pok1.tga", "effect\\pok3.tga")
        };

        // Per-slot RNG: distance (kind 2 only for live slots), x, z, speed.
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut speeds = [0.0f32; 4];

        for index in 0..4usize {
            if kind == 2 {
                if index < 2 {
                    distances[index] = runtime.rng.clamped_random(11) as f32 * 0.1 + 2.0;
                }
                xs[index] = runtime.rng.clamped_random(19) as f32 + position[0] - 9.0;
                zs[index] = runtime.rng.clamped_random(19) as f32 + position[2] - 9.0;
            } else {
                distances[index] = runtime.rng.clamped_random(10) as f32 * 0.1 + 1.5;
                xs[index] = runtime.rng.clamped_random(7) as f32 + position[0] - 3.0;
                zs[index] = runtime.rng.clamped_random(7) as f32 + position[2] - 3.0;
            }
            speeds[index] = runtime.rng.clamped_random(20) as f32 * 0.01 + 0.2;
        }

        let orbs = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(orbs);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(first);
        state.texture_secondary = Some(second);
        state.size = 0.0;
        state.position = position;

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;

            if kind == 2 && index >= 2 {
                slot.life = 0;
            } else {
                slot.distance = distances[index];
            }

            slot.height[8] = 0.0;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = xs[index];
            slot.vec_b_now[2] = zs[index];
            slot.vec_b_now[1] = position[1] - 6.0;
            slot.rot_start = 0;
            slot.height[1] = 0.0;
            slot.height[2] = speeds[index];
            slot.full_display_angle = 0;
            slot.flag1[0] = 0;
            slot.flag1[1] = 1;
            slot.flag1[2] = match kind {
                0 | 2 => 11,
                3 => 18,
                _ => 1,
            };
            slot.height[7] = 0.0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn hp_time_plays_the_heal_wave_and_orbits() {
        let mut effect = RisingOrbs::new("effect\\white02.bmp", 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 641);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["_heal_effect.wav".to_owned()]);
        let orbs = &runtime.primitives[0].state;
        assert_eq!(orbs.count, PrimitiveId::ParticleUp);
        assert_eq!(orbs.texture, Some("effect\\pok1.tga"));
        assert_eq!(orbs.texture_secondary, Some("effect\\pok3.tga"));
        assert!(orbs.slots.iter().all(|slot| slot.life == 1));
        assert!(orbs.slots.iter().all(|slot| slot.flag1[2] == 11));
        assert!(orbs.slots[0].distance >= 1.5 && orbs.slots[0].distance <= 2.5);
    }

    #[test]
    fn sp_time_uses_the_inhale_wave_and_kind_one_flag() {
        let mut effect = RisingOrbs::new("effect\\white02.bmp", 1, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 641);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\흡기.wav".to_owned()]);
        assert!(runtime.primitives[0].state.slots.iter().all(|slot| slot.flag1[2] == 1));
    }

    #[test]
    fn bursts_stop_after_frame_twenty() {
        let mut effect = RisingOrbs::new("effect\\white02.bmp", 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 641);

        while runtime.frame <= 20 {
            step(&mut effect, &mut runtime);
        }
        let at_twenty = runtime.primitives.len();
        assert_eq!(at_twenty, 6, "frames 0, 4, 8, 12, 16, 20");

        while runtime.frame <= 60 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), at_twenty);
    }
}
