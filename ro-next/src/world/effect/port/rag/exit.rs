//! `CRagEffect::Exit` - `RagEffect.cpp:2368-2418`.
//!
//! `EF_EXIT`: a rising `alpha_down` cylinder plus `particle1` sparks on the
//! orbit primitive every nine frames.

use super::super::angles::Angles;
use super::super::math::{Matrix, forward};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Exit {
    lifetime: i32,
}

impl Exit {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Exit {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 0 {
            let pillar = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(pillar);

            state.duration = 100;
            state.outer_size = 4.5;
            state.inner_size = 4.5;
            state.height_size = 35.0;
            state.alpha = 0.0;
            state.max_alpha = 90.0;
            state.alpha_speed = state.max_alpha / 30.0;
            state.fade_out_cnt = state.duration - 20;
            state.texture = Some("effect\\alpha_down.tga");

            runtime.play_sound("_heal_effect.wav");
        }

        if frame % 9 == 0 {
            // The original draws one discarded `rand()` before the longitude.
            let _discard = runtime.rng.clamped_random(360);
            let longitude = runtime.rng.clamped_random(360);

            let spark = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);

            state.radius = 3.0;
            state.duration = 50;
            state.longitude = longitude as f32;
            state.grav_speed = -1.2;
            state.grav_accel = -(state.grav_speed / state.duration as f32) / 1.5;

            let mut transform = Matrix::identity();
            transform.make_y_rotation(angles.radian(longitude));
            state.transform = transform;

            state.delta_pos2 = forward(3.0, &state.transform.values);
            state.size = 0.55;
            state.fade_out_cnt = state.duration - state.duration / 3;
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
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn exit_raises_the_pillar_and_orbiting_sparks() {
        let mut effect = Exit::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 621);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["_heal_effect.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2, "pillar + first spark");

        let pillar = &runtime.primitives[0].state;
        assert_eq!(pillar.count, PrimitiveId::Cylinder3d);
        assert_eq!(pillar.duration, 100);
        assert_eq!(pillar.outer_size, 4.5);
        assert_eq!(pillar.inner_size, 4.5);
        assert_eq!(pillar.height_size, 35.0);
        assert_eq!(pillar.max_alpha, 90.0);
        assert_eq!(pillar.fade_out_cnt, 80);
        assert_eq!(pillar.texture, Some("effect\\alpha_down.tga"));

        let spark = &runtime.primitives[1].state;
        assert_eq!(spark.count, PrimitiveId::Orbit);
        assert_eq!(spark.duration, 50);
        assert_eq!(spark.size, 0.55);
        assert_eq!(spark.sprite, Some("이팩트\\particle1.spr"));
        // One orbit step applied gravAccel.
        assert!(spark.grav_speed < -1.17 && spark.grav_speed > -1.21);
        assert_eq!(spark.fade_out_cnt, 34);
        assert!(spark.radius >= 3.0 && spark.radius < 3.2);
    }

    #[test]
    fn sparks_repeat_every_nine_frames() {
        let mut effect = Exit::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 621);

        step(&mut effect, &mut runtime);
        let base = runtime.primitives.len();

        while runtime.frame < 9 {
            step(&mut effect, &mut runtime);
        }
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), base + 1);
    }
}
