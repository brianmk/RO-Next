//! `CRagEffect::FeatherRing` - `RagEffect.cpp:4224-4271`.
//!
//! `EF_SPHEREWIND2` (`RagEffect.cpp:23143-23147`) calls it five times on
//! frame 0, at angles 0/9/18/27/36, with `effect\fire_ivy.bmp` and `large = 1`.
//! The body launches one prim 77 per call with `size = 10` (large) or `4`, four
//! slots whose random `maxHeight`, `rotStart`, `fullDisplayAngle` and
//! `height[1]` follow the original draw order, `distance = 14`,
//! `height[0] = -14` and `alphaB = 100`.
//!
//! Lifetime is the official 99999 (`EffectTypes.cpp:403`); the port keeps the
//! primitive's `duration` equal to the effect's lifetime.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRng, EffectRuntime, PortedEffect};

/// `CRagEffect::FeatherRing(const char* name, int16_t angle, char large)` -
/// `RagEffect.cpp:4224-4271`.
fn feather_ring(runtime: &mut EffectRuntime, name: &'static str, angle: i16, large: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::SphereWind, [0.0, 0.0, 0.0]);
    let mut rng = std::mem::replace(&mut runtime.rng, EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = if large == 1 { 10.0 } else { 4.0 };

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;
        slot.max_height = rng.clamped_random(11) as f32 * 0.2 + 4.0;
        slot.distance = 14.0;
        slot.height[0] = -14.0;
        slot.rot_start = rng.clamped_random(360) as i16;
        slot.rise_angle = (slot_index as i32 * 45 + angle as i32) as i16;
        slot.alpha_b = 100;
        slot.full_display_angle = (rng.clamped_random(91) + 180) as i16;
        slot.height[1] = (rng.clamped_random(4) + 3) as f32;
    }

    runtime.rng = rng;
}

/// `case EF_SPHEREWIND2:` - `RagEffect.cpp:23143-23147`.
#[derive(Debug)]
pub struct FeatherRing {
    lifetime: i32,
}

impl FeatherRing {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for FeatherRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let mut angle = 0i16;

        while angle < 45 {
            feather_ring(runtime, "effect\\fire_ivy.bmp", angle, 1);
            angle += 9;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(99_999, [0.0, 0.0], 1);
        runtime.parent_position = Some([5.0, 10.0, 15.0]);

        runtime
    }

    #[test]
    fn five_rings_spawn_on_frame_zero_with_nine_degree_steps() {
        let mut body = FeatherRing::new(99_999);
        let mut runtime = runtime();

        body.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 5);

        for (ring, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::SphereWind);
            assert_eq!(state.count.id(), 77);
            assert_eq!(state.render_flag, 5);
            assert_eq!(state.duration, 99_999);
            assert_eq!(state.texture, Some("effect\\fire_ivy.bmp"));
            assert_eq!(state.size, 10.0);
            assert_eq!(state.position, [5.0, 10.0, 15.0]);

            for slot_index in 0..4usize {
                let slot = &state.slots[slot_index];
                assert_eq!(slot.life, 1);
                assert_eq!(slot.process, 0);
                assert_eq!(slot.distance, 14.0);
                assert_eq!(slot.height[0], -14.0);
                assert_eq!(slot.alpha_b, 100);
                assert_eq!(slot.rise_angle, (slot_index as i16) * 45 + (ring as i16) * 9);
                assert!(slot.max_height >= 4.0 && slot.max_height <= 6.0);
                assert!(slot.full_display_angle >= 180 && slot.full_display_angle <= 270);
                assert!(slot.height[1] >= 3.0 && slot.height[1] <= 6.0);
            }
        }

        let count = runtime.primitives.len();
        runtime.frame = 1;
        body.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), count);
    }

    #[test]
    fn a_small_ring_uses_size_four() {
        let mut runtime = runtime();

        feather_ring(&mut runtime, "effect\\fire_ivy.bmp", 0, 0);

        assert_eq!(runtime.primitives[0].state.size, 4.0);
    }
}
