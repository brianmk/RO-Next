//! `CRagEffect::TeleportSpiral(name)` - `RagEffect.cpp:17683-17765`.
//!
//! `EF_EXIT2`: a four-helix teleport spiral on `PP_PULSING_COLUMNS` (60) with
//! rising heights 20/45/70/180 and the teleportation wave.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TeleportSpiral {
    texture: &'static str,
    lifetime: i32,
}

impl TeleportSpiral {
    pub fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

impl PortedEffect for TeleportSpiral {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;

        runtime.play_sound("effect\\EF_Teleportation.wav");

        let spiral = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spiral);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.size = 4.0;

        const MAX_HEIGHTS: [f32; 4] = [180.0, 70.0, 45.0, 20.0];
        const DISTANCES: [f32; 4] = [1.8, 2.0, 2.2, 2.4];
        const RISE_ANGLES: [i16; 4] = [90, 88, 86, 84];
        const ROTS: [i16; 4] = [270, 180, 90, 0];
        const ALPHA_T: [i16; 4] = [58, 60, 60, 60];

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = MAX_HEIGHTS[index];
            slot.rot_start = ROTS[index];
            slot.alpha_b = 0;
            slot.distance = DISTANCES[index];
            slot.rise_angle = RISE_ANGLES[index];
            slot.alpha_t = ALPHA_T[index];
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [0; 21];
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn exit2_builds_the_four_helix_spiral() {
        let mut effect = TeleportSpiral::new("effect\\ring_purple.tga", 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 661);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Teleportation.wav".to_owned()]);
        let spiral = &runtime.primitives[0].state;
        assert_eq!(spiral.count, PrimitiveId::PulsingColumns);
        assert_eq!(spiral.duration, 100);
        assert_eq!(spiral.size, 4.0);
        assert_eq!(spiral.texture, Some("effect\\ring_purple.tga"));

        assert_eq!(spiral.slots[0].max_height, 180.0);
        assert_eq!(spiral.slots[1].max_height, 70.0);
        assert_eq!(spiral.slots[2].max_height, 45.0);
        assert_eq!(spiral.slots[3].max_height, 20.0);
        assert_eq!(spiral.slots[0].rot_start, 270);
        assert_eq!(spiral.slots[3].rot_start, 0);
        assert_eq!(spiral.slots[0].alpha_t, 58);
        assert_eq!(spiral.slots[3].alpha_t, 60);
        assert_eq!(spiral.slots[0].distance, 1.8);
        assert_eq!(spiral.slots[3].distance, 2.4);
    }
}
