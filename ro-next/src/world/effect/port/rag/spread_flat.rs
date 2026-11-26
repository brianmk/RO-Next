//! `CRagEffect::TexturedSpreadFlat(name, kind)` - `RagEffect.cpp:6986-7070`.
//!
//! Flat ground rings built from four slot columns each (`PP` prim 137,
//! `PrimHeightRamp`). Used by Redline (`ring_red.tga`, 3 calls) and Blueline
//! (`ring_blue.tga`, 6 calls).

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedSpreadFlat {
    texture: &'static str,
    kind: i32,
    count: usize,
    lifetime: i32,
}

impl TexturedSpreadFlat {
    pub fn new(texture: &'static str, kind: i32, count: usize, lifetime: i32) -> Self {
        Self {
            texture,
            kind,
            count,
            lifetime,
        }
    }
}

/// One `TexturedSpreadFlat` call: spawns a single `PP` ring primitive.
pub(crate) fn spawn_one(runtime: &mut EffectRuntime, texture: &'static str, kind: i32) {
    let lifetime = runtime.lifetime;
    let angles = Angles::shared();

    // Per-kind texture size; kind 12 picks one of three sizes with the RNG.
    let size = match kind {
        k if k <= 1 => 4.0,
        2 => 7.0,
        3 => 6.0,
        10 => (runtime.rng.clamped_random(5) + 4) as f32,
        11 => 5.0,
        12 => match runtime.rng.clamped_random(3) {
            0 => 4.0,
            1 => 40.0,
            _ => 41.0,
        },
        _ => 4.0,
    };

    // Per-slot random draws, in slot order, before the primitive is mutated.
    let mut processes = [0i32; 4];
    let mut radii = [0.0f32; 4];
    let mut angles_deg = [0i32; 4];
    let mut distances = [0.0f32; 4];

    for index in 0..4usize {
        if kind == 11 {
            processes[index] = runtime.rng.clamped_random(40) + 60;
            radii[index] = runtime.rng.clamped_random(31) as f32 * 0.1 + 6.0;
        } else if kind == 12 {
            processes[index] = runtime.rng.clamped_random(50) + 50;
            radii[index] = runtime.rng.clamped_random(17) as f32;
        } else if kind == 10 {
            processes[index] = runtime.rng.clamped_random(100);
            radii[index] = runtime.rng.clamped_random(41) as f32;
        } else {
            processes[index] = runtime.rng.clamped_random(40) + 60;
            radii[index] = 5.0;
        }

        angles_deg[index] = runtime.rng.clamped_random(360);
        distances[index] = runtime.rng.clamped_random(11) as f32 * 0.015 + 0.15;
    }

    {
        let ring = runtime.launch(PrimitiveId::HeightRamp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.size = size;
        state.texture = Some(texture);

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.full_display_angle = 360;
            slot.rot_start = (index as i16) * 90;

            match kind {
                11 => {
                    slot.alpha_b = 50;
                    slot.max_height = index as f32 * 20.0 + 60.0;
                }
                12 => {
                    slot.alpha_b = 130;
                    slot.max_height = index as f32 * 30.0 + 30.0;
                }
                _ => {
                    slot.alpha_b = 150;
                    slot.max_height = 120.0;
                }
            }

            slot.process = processes[index];
            slot.distance = distances[index];
            slot.vec_b_now[0] = radii[index] * angles.raw_cosine(angles_deg[index] as usize);
            slot.vec_b_now[2] = radii[index] * angles.raw_sine(angles_deg[index] as usize);
            slot.alpha_t = 0;
            slot.rise_angle = 90;

            slot.height = [0.0; 21];
            slot.flag1 = [0; 21];
        }
    }
}

impl PortedEffect for TexturedSpreadFlat {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        for _ in 0..self.count {
            spawn_one(runtime, self.texture, self.kind);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn redline_spawns_three_red_rings() {
        let mut effect = TexturedSpreadFlat::new("effect\\ring_red.tga", 3, 3, 75);
        let mut runtime = EffectRuntime::new(75, [0.0, 0.0], 211);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::HeightRamp);
        assert_eq!(state.duration, 75);
        assert_eq!(state.size, 6.0);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\ring_red.tga"));

        for (index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 150);
            assert_eq!(slot.max_height, 120.0);
            // Spawned at index*90, one motion step rotated it by 1.
            assert_eq!(slot.rot_start, (index as i16) * 90 + 1);
            assert_eq!(slot.rise_angle, 90);
            assert_eq!(slot.alpha_t, 0);
            assert!(slot.process >= 60 && slot.process <= 99);
            assert!((slot.distance - 0.15).abs() > 0.0);
        }
    }

    #[test]
    fn blueline_uses_kind_twelve() {
        let mut effect = TexturedSpreadFlat::new("effect\\ring_blue.tga", 12, 6, 999);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 211);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 6);

        let state = &runtime.primitives[0].state;
        assert!(matches!(state.size, 4.0 | 40.0 | 41.0));
        assert_eq!(state.slots[0].alpha_b, 130);
        assert_eq!(state.slots[0].max_height, 30.0);
        assert!(state.slots[0].process >= 50 && state.slots[0].process <= 99);
        assert!(state.slots[1].max_height == 60.0);
    }
}
