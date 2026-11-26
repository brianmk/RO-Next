//! `CRagEffect::RemoveTrap` - `RagEffect.cpp:1986-2013`.
//!
//! `EF_REMOVETRAP`: twelve `particle1` motes on frames 0 and 7, each with its
//! own radius/longitude/gravity.

use super::super::angles::Angles;
use super::super::math::{forward, Matrix};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct RemoveTrap {
    lifetime: i32,
}

impl RemoveTrap {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for RemoveTrap {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 0 {
            runtime.play_sound("effect\\hunter_removetrap.wav");
        }

        if frame != 0 && frame != 7 {
            return true;
        }

        for _ in 0..12 {
            // RNG order: gravity accel, longitude (the leading rand() is
            // discarded by the original).
            let _discard = runtime.rng.clamped_random(360);
            let grav_accel = runtime.rng.clamped_random(10) as f32 / 1000.0 + 0.08;
            let longitude = (runtime.rng.clamped_random(360)) as i32;

            let mote = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(mote);

            state.duration = 50;
            state.radius_speed = 0.2;
            state.grav_speed = -2.0;
            state.grav_accel = grav_accel;
            state.longitude = longitude as f32;
            state.size = 0.8;

            let mut transform = Matrix::identity();
            transform.make_y_rotation(angles.radian(longitude));
            state.transform = transform;

            state.delta_pos2 = forward(state.radius, &state.transform.values);
            state.fade_out_cnt = 35;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn remove_trap_bursts_twice() {
        let mut effect = RemoveTrap::new(70);
        let mut runtime = EffectRuntime::new(70, [0.0, 0.0], 691);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 12);
        assert_eq!(runtime.sounds, vec!["effect\\hunter_removetrap.wav".to_owned()]);

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::Orbit);
        assert_eq!(mote.duration, 50);
        assert_eq!(mote.radius_speed, 0.2);
        // Spawned at -2.0, one orbit step applied gravAccel.
        assert!(mote.grav_speed < -1.9 && mote.grav_speed >= -2.0);
        assert!(mote.grav_accel >= 0.08 && mote.grav_accel <= 0.089);
        assert_eq!(mote.size, 0.8);
        assert_eq!(mote.fade_out_cnt, 35);
        assert_eq!(mote.sprite, Some("이팩트\\particle1.spr"));

        while runtime.frame < 7 {
            step(&mut effect, &mut runtime);
        }
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 24);
    }
}
