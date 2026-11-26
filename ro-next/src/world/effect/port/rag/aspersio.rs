//! `CRagEffect::Aspersio` - `RagEffect.cpp:1568-1597`.
//!
//! `EF_ASPERSIO`: holy water drops on the orbit primitive every five frames
//! between frames 61 and 139, with the priest wave on frame 70 and a duration
//! override to 400.

use super::super::angles::Angles;
use super::super::math::{Matrix, forward};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Aspersio {
    lifetime: i32,
}

impl Aspersio {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Aspersio {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 70 {
            runtime.play_sound("effect\\priest_aspersio.wav");
        }

        if frame % 5 == 0 && frame > 60 && frame < 140 {
            let longitude = (runtime.rng.clamped_random(360)) as i32;

            let drop = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(drop);

            state.duration = 40;
            state.radius = 1.5;
            state.grav_speed = 0.19;
            state.grav_accel = 0.01;
            state.longitude = longitude as f32;
            state.size = 1.2;

            let mut transform = Matrix::identity();
            transform.make_y_rotation(angles.radian(longitude));
            state.transform = transform;

            // The offset is `(0, -30, radius)` rotated by the longitude.
            let rotated = forward(1.5, &state.transform.values);
            state.delta_pos2 = [rotated[0], rotated[1] - 30.0, rotated[2]];

            state.fade_out_cnt = 30;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.init_segment();
        }

        runtime.process_str();

        // The original overrides the effect duration to 400 and retires there.
        if frame >= 400 {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn aspersio_drips_holy_water_during_its_window() {
        let mut effect = Aspersio::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 671);

        while runtime.frame <= 60 {
            step(&mut effect, &mut runtime);
        }
        assert!(runtime.primitives.is_empty(), "no drops before frame 61");

        while runtime.frame <= 65 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 1, "frame 65 is the first drop");

        let drop = &runtime.primitives[0].state;
        assert_eq!(drop.count, PrimitiveId::Orbit);
        assert_eq!(drop.duration, 40);
        assert_eq!(drop.radius, 1.5);
        // Spawned at 0.19, one orbit step applied gravAccel 0.01.
        assert!((drop.grav_speed - 0.2).abs() < 1e-5);
        assert_eq!(drop.grav_accel, 0.01);
        assert_eq!(drop.size, 1.2);
        assert_eq!(drop.fade_out_cnt, 30);
        assert_eq!(drop.sprite, Some("이팩트\\particle1.spr"));
        assert!(drop.delta_pos2[1] < -29.0);
    }

    #[test]
    fn wave_lands_on_frame_seventy() {
        let mut effect = Aspersio::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 671);

        while runtime.frame < 70 {
            step(&mut effect, &mut runtime);
        }
        assert!(runtime.sounds.is_empty());

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\priest_aspersio.wav".to_owned()]);
    }
}
