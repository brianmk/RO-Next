//! `CRagEffect::EarthSpike(char type)` - `RagEffect.cpp:11213-11287`.
//!
//! One large centre shard plus six spikes fanned out around the cell, all
//! `PP_3DQUADHORN`. `type 0` is the earth spike (`stone.bmp`), `type 2` the ice
//! variant (`ice.tga` + a random `EF_IceArrow1..3.wav`).

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct EarthSpike {
    variant: i32,
}

impl EarthSpike {
    pub fn new(variant: i32) -> Self {
        Self { variant }
    }
}

impl Default for EarthSpike {
    fn default() -> Self {
        Self { variant: 0 }
    }
}

impl PortedEffect for EarthSpike {
    fn lifetime(&self) -> i32 {
        240
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        match self.variant {
            0 => runtime.play_sound("effect\\wizard_earthspike.wav"),
            2 => {
                let wave = runtime.rng.clamped_random(3) + 1;
                runtime.play_sound(&format!("effect\\EF_IceArrow{wave}.wav"));
            }
            _ => {}
        }

        runtime.quake();

        let angles = Angles::shared();

        // Centre shard.
        let heading = runtime.rng.clamped_random(360);
        runtime.rng.discard();
        let _ = angles.sine(heading);
        let _ = angles.cosine(heading);

        let size = (runtime.rng.clamped_random(50) + 300) as f32 / 100.0;
        let longitude = runtime.rng.clamped_random(360) as f32;
        let latitude = (runtime.rng.clamped_random(20) + 80) as f32;

        let center = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(center);
            state.duration = 240;
            state.pattern = 1;
            state.pattern |= 2;
            state.delta_pos2 = [0.0, 10.0, 0.0];
            state.size = size;
            state.height_size = 18.0;
            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 1.0;
            state.accel = 0.01;
            state.alpha = 250.0;
            state.ch_cnt = 15;
            state.change_point = 12;
            state.change_speed = -1.2;
            state.change_accel = 0.0;
            state.fade_out_cnt = state.duration - 10;
        }

        // Six surrounding spikes.
        for step in (60..420).step_by(60) {
            let offset = runtime.rng.clamped_random(step);
            let angle = runtime.rng.clamped_random(360);
            runtime.rng.discard();

            let sine = angles.sine(angle);
            let cosine = angles.cosine(angle);
            let spike_size = (runtime.rng.clamped_random(100) + 400) as f32 / 100.0;
            let longitude = ((offset + step - 60) as f32).min(360.0);

            let spike = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spike);
            state.duration = 240;
            state.pattern |= 1;
            state.delta_pos2 = [sine * 3.5, 20.0, cosine * -3.5];
            state.size = spike_size;
            state.height_size = 20.0;
            state.longitude = longitude;
            state.latitude = 100.0;
            state.transform.make_x_rotation(angles.radian(100));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 1.5;
            state.accel = 0.0;
            state.alpha = 250.0;
            state.ch_cnt = 2;
            state.fade_out_cnt = state.duration - 10;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn spawns_center_plus_six_spikes_and_quakes() {
        let mut effect = EarthSpike::new(0);
        let mut runtime = EffectRuntime::new(240, [0.0, 0.0], 21);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_earthspike.wav".to_owned()]);
        assert_eq!(runtime.shakes, 1);
        assert_eq!(runtime.primitives.len(), 7);

        let center = &runtime.primitives[0].state;
        assert_eq!(center.pattern & 3, 3);
        assert_eq!(center.duration, 240);
        assert_eq!(center.change_point, 12);
        assert_eq!(center.change_speed, -1.2);
        assert_eq!(center.ch_cnt, 15);
        assert_eq!(center.delta_pos2, [0.0, 10.0, 0.0]);
        assert!((3.0..3.5).contains(&center.size));
        assert!((80.0..100.0).contains(&center.latitude));

        for spike in &runtime.primitives[1..] {
            let state = &spike.state;
            assert_eq!(state.pattern & 1, 1);
            assert_eq!(state.duration, 240);
            assert_eq!(state.ch_cnt, 2);
            assert_eq!(state.height_size, 20.0);
            assert!((4.0..5.0).contains(&state.size));
            assert!((20.0..23.0).contains(&state.delta_pos2[1]));
        }
    }

    #[test]
    fn ice_variant_plays_a_random_ice_arrow_sound() {
        let mut effect = EarthSpike::new(2);
        let mut runtime = EffectRuntime::new(240, [0.0, 0.0], 21);

        step(&mut effect, &mut runtime);

        let sound = &runtime.sounds[0];
        assert!(sound.starts_with("effect\\EF_IceArrow"), "got {sound}");
    }
}
