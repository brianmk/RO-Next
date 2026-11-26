//! `CRagEffect::LockOnMarker` - `RagEffect.cpp:9740-9795`.
//!
//! `EF_LOCKON`: two crossed `LockOn128.tga` quads that rotate at 4.5°/frame and
//! shrink by 1 unit/frame, with the yellow→red RGB cycle every 20 frames.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct LockOn {
    lifetime: i32,
}

impl LockOn {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for LockOn {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;

        for longitude in [0.0f32, 45.0] {
            let marker = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(marker);

            state.duration = lifetime;
            state.pattern |= 0x800;
            state.longitude = longitude;
            state.long_speed = 4.5;
            state.roll = 90.0;
            state.change_width_accel = 0.0;
            state.change_height_accel = 0.0;
            state.width_size = 30.0;
            state.height_size = 30.0;
            state.width_speed = -1.0;
            state.height_speed = -1.0;
            state.change_point = 20;
            state.change_width_speed = 0.0;
            state.change_height_speed = 0.0;
            state.rgb_cycle_delay = 20;
            state.red = 250;
            state.blue = 150;
            state.blue_speed = -15;
            state.green = 150;
            state.green_speed = -15;
            state.texture = Some("effect\\LockOn128.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn lock_on_spawns_two_crossed_markers() {
        let mut effect = LockOn::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 651);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::CrossTexture3d);
        assert_eq!(first.duration, 200);
        assert_eq!(first.pattern & 0x800, 0x800);
        // Spawned at 0/45, the first texture3d step applied longSpeed.
        assert!((first.longitude - 4.5).abs() < 1e-4);
        assert_eq!(first.long_speed, 4.5);
        assert_eq!(first.roll, 90.0);
        // width/heightSpeed -1 shrinks both by one on the first step.
        assert!((first.width_size - 29.0).abs() < 1e-4);
        assert!((first.height_size - 29.0).abs() < 1e-4);
        assert_eq!(first.change_point, 20);
        assert_eq!(first.rgb_cycle_delay, 20);
        // green/blueSpeed -15 applied once.
        assert_eq!((first.red, first.green, first.blue), (250, 135, 135));
        assert_eq!((first.green_speed, first.blue_speed), (-15, -15));
        assert_eq!(first.texture, Some("effect\\LockOn128.tga"));

        assert!((runtime.primitives[1].state.longitude - 49.5).abs() < 1e-4);
    }
}
