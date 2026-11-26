//! `CRagEffect::Cloud11Burst(kind)` - `RagEffect.cpp:16089-16157`.
//!
//! Four slot-driven rising wind columns using `cloud11.tga` (`PP_WIND`). Used
//! by the portal effects and the Storm Kick family.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Cloud11Burst {
    kind: i32,
    lifetime: i32,
    /// `(frame, wave)` sounds the dispatch block schedules.
    waves: Vec<(i32, &'static str)>,
}

impl Cloud11Burst {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self {
            kind,
            lifetime,
            waves: Vec::new(),
        }
    }

    pub fn with_waves(mut self, waves: &[(i32, &'static str)]) -> Self {
        self.waves = waves.to_vec();
        self
    }
}

impl PortedEffect for Cloud11Burst {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        for wave in self.waves.iter().filter(|(at, _)| *at == frame) {
            runtime.play_sound(wave.1);
        }

        if frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let kind = self.kind;

        // The distance/rise random draws happen per slot, in slot order, before
        // the primitive is mutated (same RNG order as the original).
        let mut distances = [0.0f32; 4];
        let mut rises = [0i16; 4];

        for index in 0..4usize {
            if kind >= 2 {
                distances[index] = if kind == 2 {
                    14.0 - index as f32 * 2.0
                } else if kind == 3 {
                    6.0 - index as f32
                } else {
                    runtime.rng.clamped_random(101) as f32 * 0.03 + 4.5
                };
            } else {
                distances[index] = if kind == 1 {
                    runtime.rng.clamped_random(101) as f32 * 0.01 + 2.5
                } else {
                    runtime.rng.clamped_random(101) as f32 * 0.03 + 4.5
                };
            }

            rises[index] = (runtime.rng.clamped_random(21) + 80) as i16;
        }

        let cloud = runtime.launch(PrimitiveId::Wind, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(cloud);

        state.duration = lifetime;
        state.size = if kind == 1 { 8.0 } else { 5.0 };
        state.texture = Some("effect\\cloud11.tga");

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 30;

            slot.max_height = if kind == 1 {
                index as f32 * 2.0 + 3.0
            } else if kind == 2 {
                48.0 - index as f32 * 5.0
            } else if kind == 3 {
                28.0 - index as f32 * 5.0
            } else {
                index as f32 * 2.0 + 5.0
            };

            slot.rot_start = (index as i16) * 90;
            slot.alpha_b = 0;

            slot.alpha_t = if kind >= 2 { 2 } else { 1 };
            slot.distance = distances[index];
            slot.rise_angle = rises[index];

            slot.height = [1.0; 21];
            slot.flag1 = [0; 21];
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn portal4_spawns_four_wind_slots() {
        let mut effect = Cloud11Burst::new(0, 200).with_waves(&[(0, "effect\\윈드워크.wav")]);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\윈드워크.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Wind);
        assert_eq!(state.duration, 200);
        assert_eq!(state.size, 5.0);
        assert_eq!(state.texture, Some("effect\\cloud11.tga"));

        for (index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            // Spawned at 30, one motion step added 3.
            assert_eq!(slot.full_display_angle, 33);
            assert_eq!(slot.rot_start, ((index as i16) * 90 + 5) % 360);
            assert_eq!(slot.alpha_t, 1);
            assert_eq!(slot.max_height, index as f32 * 2.0 + 5.0);
            // kind 0: (rand(101) * 0.03) + 4.5
            assert!((4.5..7.6).contains(&slot.distance));
            assert_eq!(slot.alpha_b, 10);
            assert!((80..=100).contains(&slot.rise_angle));
        }
    }

    #[test]
    fn portal5_uses_the_larger_size_and_kind_two_layout() {
        let mut effect = Cloud11Burst::new(1, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.size, 8.0);
        assert_eq!(state.slots[0].max_height, 3.0);
        assert_eq!(state.slots[0].alpha_t, 1);
        assert!((2.5..3.6).contains(&state.slots[0].distance));
    }

    #[test]
    fn storm_kick_uses_alpha_t_two_and_descending_heights() {
        let mut effect = Cloud11Burst::new(2, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].alpha_t, 2);
        assert_eq!(state.slots[0].max_height, 48.0);
        assert_eq!(state.slots[0].distance, 14.0);
    }

    #[test]
    fn slots_advance_once_per_frame() {
        let mut effect = Cloud11Burst::new(0, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 2);
    }
}
