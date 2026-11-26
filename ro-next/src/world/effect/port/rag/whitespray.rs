//! `CRagEffect::WhiteSpray(kind)` - `RagEffect.cpp:5874-5921`, used by the
//! Intimidate family (`EF_INTIMIDATE`, `EF_INTIMIDATE2/3/4`, lifetimes 999).
//!
//! Each call spawns one `PP_INTIMIDATE` primitive with four slots; the dispatch
//! blocks call it 10-20 times on frame 0.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct WhiteSpray {
    kind: i32,
    count: usize,
    lifetime: i32,
    waves: Vec<(i32, &'static str)>,
}

impl WhiteSpray {
    pub fn new(kind: i32, count: usize, lifetime: i32) -> Self {
        Self {
            kind,
            count,
            lifetime,
            waves: Vec::new(),
        }
    }

    pub fn with_waves(mut self, waves: &[(i32, &'static str)]) -> Self {
        self.waves = waves.to_vec();
        self
    }

    fn spawn_one(&self, runtime: &mut EffectRuntime) {
        let kind = self.kind;
        let lifetime = self.lifetime;

        // Per-slot RNG draws happen before the primitive is mutated.
        let mut processes = [0i32; 4];
        let mut flags = [0i8; 4];
        let mut angles = [[0i16; 4]; 4]; // rise, display, rot, orbit

        for index in 0..4usize {
            processes[index] = match kind {
                2 => runtime.rng.clamped_random(26) * -2,
                3 => -70 - runtime.rng.clamped_random(30),
                4 => -30 - runtime.rng.clamped_random(20),
                _ => -70 - runtime.rng.clamped_random(30),
            };

            flags[index] = match kind {
                2 => 2,
                3 | 4 => (runtime.rng.clamped_random(3) + 3) as i8,
                _ => 0,
            };

            angles[index] = [
                runtime.rng.clamped_random(360) as i16,
                runtime.rng.clamped_random(360) as i16,
                runtime.rng.clamped_random(360) as i16,
                runtime.rng.clamped_random(360) as i16,
            ];
        }

        let spray = runtime.launch(PrimitiveId::Intimidate, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spray);

        state.duration = lifetime;
        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.texture = Some(if kind == 4 { "effect\\white03.bmp" } else { "effect\\white01.bmp" });

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = processes[index];
            slot.distance = 12.0;
            slot.height[0] = 1.5;
            slot.height[1] = match kind {
                2 => 2.5,
                3 => 4.0,
                4 => 3.0,
                _ => 3.5,
            };
            slot.flag1[0] = flags[index];
            slot.rise_angle = angles[index][0];
            slot.full_display_angle = angles[index][1];
            slot.rot_start = angles[index][2];
            slot.height[2] = angles[index][3] as f32;
            slot.alpha_b = 0;
        }
    }
}

impl PortedEffect for WhiteSpray {
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

        for _ in 0..self.count {
            self.spawn_one(runtime);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn intimidate_spawns_twenty_primitive_batches() {
        let mut effect = WhiteSpray::new(0, 20, 999)
            .with_waves(&[(0, "effect\\EF_Bash.wav"), (35, "effect\\rog_intimidate.wav")]);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 181);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Bash.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 20);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Intimidate);
        assert_eq!(state.duration, 999);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\white01.bmp"));

        for slot in &state.slots {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.flag1[0], 0);
            assert_eq!(slot.distance, 12.0);
            assert!((slot.height[0] - 1.5).abs() < 1e-6);
            assert!((slot.height[1] - 3.5).abs() < 1e-6);
            assert!(slot.process <= -70 && slot.process >= -99);
        }
    }

    #[test]
    fn kind_four_uses_white03_and_the_short_delay() {
        let mut effect = WhiteSpray::new(4, 10, 999).with_waves(&[(25, "effect\\triangle.wav")]);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 181);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 10);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture, Some("effect\\white03.bmp"));
        assert!((state.slots[0].height[1] - 3.0).abs() < 1e-6);
        assert!(state.slots[0].flag1[0] >= 3 && state.slots[0].flag1[0] <= 5);
        assert!(state.slots[0].process <= -30 && state.slots[0].process >= -49);
    }

    #[test]
    fn kind_two_drifts_and_spins_with_flag_two() {
        let mut effect = WhiteSpray::new(2, 16, 999);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 181);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 16);
        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.flag1[0], 2);
        assert!((slot.height[1] - 2.5).abs() < 1e-6);
        assert!(slot.process <= 0 && slot.process >= -50);
    }

    #[test]
    fn the_late_wave_fires_on_its_frame() {
        let mut effect = WhiteSpray::new(0, 1, 999)
            .with_waves(&[(0, "effect\\EF_Bash.wav"), (35, "effect\\rog_intimidate.wav")]);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 181);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\EF_Bash.wav".to_owned()]);

        while runtime.frame < 35 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.sounds.len(), 1, "the late wave has not fired yet");

        step(&mut effect, &mut runtime);
        assert_eq!(
            runtime.sounds,
            vec!["effect\\EF_Bash.wav".to_owned(), "effect\\rog_intimidate.wav".to_owned()]
        );
    }
}
