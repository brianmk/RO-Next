//! `CRagEffect::Blessing` - `RagEffect.cpp:1612-1720`, effect id `EF_BLESSING`,
//! lifetime 150 frames.
//!
//! Frame 0 spawns the cyan halo (`PP_3DCIRCLE`); frames 0, 3, 6, 9 add a
//! `축복.spr` particle; every fourth frame adds a `particle6.spr` mote whose
//! velocity is built through a Y rotation. Both particles are
//! `PP_3DPARTICLE`; their `.act` frame count comes from the asset layer.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_BLESSING` lifetime in frames.
pub const LIFETIME: i32 = 150;

#[derive(Debug, Default)]
pub struct Blessing {
    spawned: bool,
}

impl PortedEffect for Blessing {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let angles = Angles::shared();

        if runtime.frame == 0 {
            self.spawned = true;
            runtime.play_sound("effect\\EF_Blessing.wav");

            let duration = runtime.lifetime;
            let halo = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(halo);
                state.duration = duration;
                state.pattern |= 1;
                state.radius = 10.0;
                state.latitude = 90.0;
                state.red = 32;
                state.green = 176;
                state.blue = 232;
                state.alpha = 0.0;
                state.max_alpha = 100.0;
                state.alpha_speed = state.max_alpha / 30.0;
                state.fade_out_cnt = state.duration - 30;
                state.init_segment();
            }
        }

        if runtime.frame % 3 == 0 && runtime.frame < 10 {
            let frame = runtime.frame;
            let lifetime = runtime.lifetime;

            let particle = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(particle);
            state.duration = lifetime;
            state.anim_speed = 2;
            state.alpha = (frame as f32 * -6.0) + 200.0;
            state.delta_pos2 = [0.0, -25.0, 0.0];
            state.init_segment();
        }

        if runtime.frame % 4 == 0 {
            let rotation = runtime.rng.clamped_random(360);
            let distance = (runtime.rng.clamped_random(7) + 2) as f32;
            let speed = (runtime.rng.clamped_random(20) + 20) as f32 / 100.0;

            let mote = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(mote);

            state.duration = 80;
            state.pattern |= 0x80;
            state.transform.make_y_rotation(angles.radian(rotation));
            state.delta_pos2 = matrix_mult([0.0, -25.0, distance], &state.transform.values);
            state.transform.make_x_rotation(angles.radian(-90));
            state.speed = speed;
            state.accel = -(state.speed / state.duration as f32) * 0.5;
            state.alpha = 0.0;
            state.size = 0.5;
            state.fade_out_cnt = state.duration - state.duration / 5;
            state.alpha_speed = state.max_alpha / 10.0;
            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    fn runtime() -> EffectRuntime {
        EffectRuntime::new(LIFETIME, [0.0, 0.0], 81)
    }

    #[test]
    fn frame_zero_spawns_halo_particle_and_mote() {
        let mut effect = Blessing::default();
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Blessing.wav".to_owned()]);
        // halo + frame-0 blessing particle + frame-0 mote.
        assert_eq!(runtime.primitives.len(), 3);

        let halo = &runtime.primitives[0].state;
        assert_eq!(halo.count, PrimitiveId::Circle3d);
        assert_eq!(halo.radius, 10.0);
        assert_eq!(halo.latitude, 90.0);
        assert_eq!((halo.red, halo.green, halo.blue), (32, 176, 232));
        assert_eq!(halo.max_alpha, 100.0);
        assert!((halo.alpha_speed - 100.0 / 30.0).abs() < 1e-4);
        assert_eq!(halo.fade_out_cnt, 120);

        let particle = &runtime.primitives[1].state;
        assert_eq!(particle.count, PrimitiveId::Particle3d);
        assert_eq!(particle.duration, LIFETIME);
        assert_eq!(particle.anim_speed, 2);
        assert_eq!(particle.alpha, 200.0);
        assert_eq!(particle.delta_pos2, [0.0, -25.0, 0.0]);

        let mote = &runtime.primitives[2].state;
        assert_eq!(mote.count, PrimitiveId::Particle3d);
        assert_eq!(mote.duration, 80);
        assert_eq!(mote.pattern & 0x80, 0x80);
        assert_eq!(mote.size, 0.5);
        assert_eq!(mote.fade_out_cnt, 64);
        assert!((0.2..0.4).contains(&mote.speed));
    }

    #[test]
    fn blesses_then_keeps_moting_until_lifetime() {
        let mut effect = Blessing::default();
        let mut runtime = runtime();

        // Frames 0..9 add the blessing particle on 0/3/6/9, motes every 4.
        while runtime.frame < 10 {
            step(&mut effect, &mut runtime);
        }

        let particles = runtime
            .primitives
            .iter()
            .filter(|p| p.state.count == PrimitiveId::Particle3d)
            .count();
        assert!(particles >= 4 + 3, "got {particles} particles");

        while step(&mut effect, &mut runtime) {
            if runtime.frame > 400 {
                panic!("effect did not retire");
            }
        }

        assert_eq!(runtime.frame, LIFETIME);
    }
}
