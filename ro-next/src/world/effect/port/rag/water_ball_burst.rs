//! `CRagEffect::WaterBallBurst` - `RagEffect.cpp:3484-3533`.
//!
//! `EF_WATERBALL3`: two expanding `ring_blue` rings plus an `ice.tga` droplet
//! sphere, all with `g_depthAverageHalf` deceleration.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

#[derive(Debug)]
pub struct WaterBallBurst {
    lifetime: i32,
}

impl WaterBallBurst {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for WaterBallBurst {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        runtime.play_sound("effect\\wizard_waterball_chulung.wav");

        // First ring: 30 frames, innerSize 12.
        let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = 30;
        state.render_flag |= 0x200;
        state.latitude = 90.0;
        state.radius_speed = 1.75;
        state.radius_accel = -(state.radius_speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.inner_size = 12.0;
        state.alpha = 0.0;
        state.alpha_speed = state.max_alpha / 15.0;
        state.fade_out_cnt = 10;
        state.init_segment();
        state.texture = Some("effect\\ring_blue.tga");

        // Droplets sphere.
        let droplets = runtime.launch(PrimitiveId::Sphere3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(droplets);

        state.duration = 30;
        state.radius_speed = 1.15;
        state.radius_accel = -(state.radius_speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.long_speed = 3.0;
        state.alpha = 0.0;
        state.alpha_speed = state.max_alpha / 15.0;
        state.fade_out_cnt = 10;
        state.texture = Some("effect\\ice.tga");

        // Second ring: 20 frames.
        let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = 20;
        state.render_flag |= 0x200;
        state.latitude = 90.0;
        state.radius_speed = 1.75;
        state.radius_accel = -(state.radius_speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.inner_size = 12.0;
        state.alpha = 0.0;
        state.alpha_speed = state.max_alpha / 15.0;
        state.fade_out_cnt = 10;
        state.init_segment();
        state.texture = Some("effect\\ring_blue.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn water_ball_burst_spawns_two_rings_and_droplets() {
        let mut effect = WaterBallBurst::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 511);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_waterball_chulung.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 3);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::Circle3d);
        assert_eq!(first.duration, 30);
        assert_eq!(first.latitude, 90.0);
        assert_eq!(first.inner_size, 12.0);
        assert_eq!(first.fade_out_cnt, 10);
        assert_eq!(first.texture, Some("effect\\ring_blue.tga"));
        assert!((first.radius_accel + 1.75 / 30.0 * DEPTH_AVERAGE_HALF).abs() < 1e-5);

        let droplets = &runtime.primitives[1].state;
        assert_eq!(droplets.count, PrimitiveId::Sphere3d);
        assert_eq!(droplets.duration, 30);
        assert_eq!(droplets.long_speed, 3.0);
        assert_eq!(droplets.texture, Some("effect\\ice.tga"));

        let second = &runtime.primitives[2].state;
        assert_eq!(second.count, PrimitiveId::Circle3d);
        assert_eq!(second.duration, 20);
    }
}
