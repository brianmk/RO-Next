//! `CRagEffect::WhiteRingWave` (`RagEffect.cpp:13637-13700`) and
//! `CRagEffect::BlueRingWave` (`RagEffect.cpp:14036-14102`).
//!
//! Both build an `PP_RIPPLERING` (63) wave with three ring slots
//! (`process` 0/-10/-20, `rotStart` 0/25/50, `riseAngle` 2/3/4) and a fourth
//! driver slot with `alphaT = 1`. White runs every 100 frames, blue once plus
//! an `EF_ReadyPortal.wav` heartbeat every 14 frames.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct RingWave {
    blue: bool,
    lifetime: i32,
}

impl RingWave {
    pub fn white(lifetime: i32) -> Self {
        Self {
            blue: false,
            lifetime,
        }
    }

    pub fn blue(lifetime: i32) -> Self {
        Self {
            blue: true,
            lifetime,
        }
    }

    fn spawn(&self, runtime: &mut EffectRuntime) {
        let lifetime = runtime.lifetime;
        let texture = if self.blue {
            "effect\\ring_blue.tga"
        } else {
            "effect\\ring_white.tga"
        };

        let wave = runtime.launch(PrimitiveId::RippleRings, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(wave);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(texture);
        state.size = if self.blue { 5.0 } else { 16.0 };

        const PROCESSES: [i32; 3] = [0, -10, -20];
        const RISES: [i16; 3] = [2, 3, 4];

        for index in 0..3usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = PROCESSES[index];
            slot.full_display_angle = 360;
            slot.max_height = 6.001;
            slot.rot_start = index as i16 * 25;
            slot.alpha_b = 0;
            slot.distance = 0.0;
            slot.rise_angle = RISES[index];
            slot.alpha_t = 100;
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [1; 21];
        }

        state.slots[3].life = 0;
        state.slots[3].process = 0;
        state.slots[3].alpha_t = 1;
    }
}

impl PortedEffect for RingWave {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if self.blue {
            if frame % 14 == 0 {
                runtime.play_sound("effect\\EF_ReadyPortal.wav");
            }

            if frame == 0 {
                self.spawn(runtime);
            }
        } else if frame % 100 == 0 {
            self.spawn(runtime);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn blue_wave_beats_every_fourteen_frames() {
        let mut effect = RingWave::blue(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 381);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.sounds, vec!["effect\\EF_ReadyPortal.wav".to_owned()]);

        let wave = &runtime.primitives[0].state;
        assert_eq!(wave.count, PrimitiveId::RippleRings);
        assert_eq!(wave.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(wave.size, 5.0);
        assert_eq!(wave.slots[0].process, 1, "advanced by the ripple step");
        assert_eq!(wave.slots[1].process, -9);
        assert_eq!(wave.slots[2].process, -19);
        assert_eq!(wave.slots[0].rise_angle, 2);
        assert_eq!(wave.slots[2].rise_angle, 4);
        assert!(wave.slots.iter().take(3).all(|slot| slot.flag1[0] == 1));
        assert_eq!(wave.slots[3].life, 0);
        assert_eq!(wave.slots[3].alpha_t, 1);

        for _ in 0..14 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.sounds.len(), 2, "next beat on frame 14");
        assert_eq!(runtime.primitives.len(), 1, "no second wave spawn");
    }

    #[test]
    fn white_wave_runs_every_hundred_frames() {
        let mut effect = RingWave::white(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 381);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(
            runtime.primitives[0].state.texture,
            Some("effect\\ring_white.tga")
        );
        assert_eq!(runtime.primitives[0].state.size, 16.0);

        // The wave re-spawns on every multiple of 100 while the effect lives.
        while runtime.frame <= 100 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 2, "frames 0 and 100");
    }
}
