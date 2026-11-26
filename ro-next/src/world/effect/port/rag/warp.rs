//! `CRagEffect::Warp` - `RagEffect.cpp:2414-2463`.
//!
//! `EF_WARP`: a yellow ring every 20 frames plus `particle1` sparks on every
//! even frame, rising through the radius band selected by `rand(200)/100`.

use super::super::angles::Angles;
use super::super::math::{Matrix, forward};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Warp {
    lifetime: i32,
}

impl Warp {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Warp {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let lifetime = runtime.lifetime;
        let angles = Angles::shared();

        if frame % 20 == 0 {
            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(ring);

            state.duration = lifetime;
            state.render_flag |= 0x200;
            state.radius = 2.0;
            state.arc_angle = 22.5;
            state.inner_size = 10.0;
            state.latitude = 90.0;
            state.transform.make_x_rotation(90.0 / 180.0 * 3.141592);
            state.radius_speed = 1.25;
            state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
            state.alpha = 200.0;
            state.fade_out_cnt = state.duration - state.duration / 2;
            state.init_segment();
            state.texture = Some("EFFECT\\RING_YELLOW.TGA");
        }

        if (frame & 1) == 0 {
            // RNG order: band, longitude, gravity.
            let band = runtime.rng.clamped_random(200) / 100;
            let radius = band as f32 * 0.7 + 1.0;
            let longitude = runtime.rng.clamped_random(360);
            let gravity = (60 - runtime.rng.clamped_random(240)) as f32 / 100.0;

            let spark = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);

            state.radius = radius;
            state.duration = 40;
            state.longitude = longitude as f32;
            state.long_speed = band as f32 * 3.0 + 3.0;
            state.grav_speed = gravity;
            state.grav_accel = -(state.grav_speed / state.duration as f32) * 0.5;

            let mut transform = Matrix::identity();
            transform.make_y_rotation(angles.radian(longitude));
            state.transform = transform;
            state.delta_pos2 = forward(radius, &state.transform.values);

            state.size = 0.2;
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
    fn warp_spawns_ring_and_even_frame_sparks() {
        let mut effect = Warp::new(80);
        let mut runtime = EffectRuntime::new(80, [0.0, 0.0], 631);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2, "ring + first spark");

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 80);
        assert_eq!(ring.render_flag & 0x200, 0x200);
        // One circle3d step expanded the radius by radiusSpeed.
        assert!(ring.radius > 2.0 && ring.radius < 3.5);
        assert_eq!(ring.arc_angle, 22.5);
        assert_eq!(ring.inner_size, 10.0);
        assert_eq!(ring.latitude, 90.0);
        assert!(ring.alpha > 0.0);
        assert_eq!(ring.fade_out_cnt, 40);
        assert_eq!(ring.texture, Some("EFFECT\\RING_YELLOW.TGA"));

        let spark = &runtime.primitives[1].state;
        assert_eq!(spark.count, PrimitiveId::Orbit);
        assert_eq!(spark.duration, 40);
        assert_eq!(spark.size, 0.2);
        assert_eq!(spark.sprite, Some("이팩트\\particle1.spr"));
        assert!(spark.radius >= 1.0 && spark.radius <= 2.3);
        assert!(spark.grav_speed >= -1.8 && spark.grav_speed <= 0.6);
    }

    #[test]
    fn spark_count_grows_on_even_frames_only() {
        let mut effect = Warp::new(80);
        let mut runtime = EffectRuntime::new(80, [0.0, 0.0], 631);

        step(&mut effect, &mut runtime);
        let after_frame_zero = runtime.primitives.len();

        step(&mut effect, &mut runtime); // frame 1: no spark
        assert_eq!(runtime.primitives.len(), after_frame_zero);

        step(&mut effect, &mut runtime); // frame 2: spark
        assert!(runtime.primitives.len() > after_frame_zero);
    }
}
