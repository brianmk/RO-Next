//! `CRagEffect::TexturedTumble` - `RagEffect.cpp:6022-6071`.
//!
//! `EF_CARTTER` (`RagEffect.cpp:21477-21482`) plays
//! `effect\wizard_fire_pillar_b.wav` on frame 30 and calls
//! `TexturedTumble("effect\whitelight.tga", 0)` four times on **every** frame;
//! the body itself only spawns on frame 30, so one frame produces four prim
//! 115 bursts.
//!
//! Each burst arms four slots with `maxHeight = 1`, `distance = 2.5 +
//! rand(6)*0.1`, three 3-D start points at the origin and angle triples
//! `height[0]/[4]/[8] = rand(360)`, `height[2]/[6]/[10] = rand(180)`,
//! `height[12..14] = 230`, `height[15] = rand(3) + 1`.
//!
//! Lifetime 200 (`EffectTypes.cpp:527`). Renderer
//! [`render_soul_particle`](super::super::render_slots::render_soul_particle).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRng, EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedTumble(const char* name, char)` -
/// `RagEffect.cpp:6022-6071`.
fn textured_tumble(runtime: &mut EffectRuntime, name: &'static str) {
    runtime.copy_parent_position();

    if runtime.frame != 30 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::ParticleSpread, [0.0, 0.0, 0.0]);
    let mut rng = std::mem::replace(&mut runtime.rng, EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.max_height = 1.0;
        slot.distance = rng.clamped_random(6) as f32 * 0.1 + 2.5;
        slot.process = 0;
        slot.vec_b_pre = [0.0; 3];
        slot.height[0] = rng.clamped_random(360) as f32;
        slot.height[2] = rng.clamped_random(180) as f32;
        slot.height[12] = 230.0;
        slot.vec_b_now = [0.0; 3];
        slot.height[4] = rng.clamped_random(360) as f32;
        slot.height[6] = rng.clamped_random(180) as f32;
        slot.height[13] = 230.0;
        slot.vec_t_pre = [0.0; 3];
        slot.height[8] = rng.clamped_random(360) as f32;
        slot.height[10] = rng.clamped_random(180) as f32;
        slot.height[14] = 230.0;
        slot.height[15] = (rng.clamped_random(3) + 1) as f32;
    }

    runtime.rng = rng;
}

/// `case EF_CARTTER:` - `RagEffect.cpp:21477-21482`.
#[derive(Debug)]
pub struct TexturedTumble {
    lifetime: i32,
}

impl TexturedTumble {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for TexturedTumble {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 30 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\wizard_fire_pillar_b.wav");
        }

        for _ in 0..4 {
            textured_tumble(runtime, "effect\\whitelight.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn frame_thirty_spawns_four_bursts_and_plays_the_wave() {
        let mut body = TexturedTumble::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.frame = 30;

        body.on_process(&mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_fire_pillar_b.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 4);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ParticleSpread);
        assert_eq!(state.count.id(), 115);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\whitelight.tga"));
        assert_eq!(state.duration, 200);

        for slot in &state.slots {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.max_height, 1.0);
            assert!(slot.distance >= 2.5 && slot.distance < 3.1);
            assert_eq!(slot.height[12], 230.0);
            assert_eq!(slot.height[13], 230.0);
            assert_eq!(slot.height[14], 230.0);
            assert!(slot.height[15] >= 1.0 && slot.height[15] <= 3.0);
            assert!((0.0..360.0).contains(&slot.height[0]));
            assert!((0.0..180.0).contains(&slot.height[2]));
        }
    }

    #[test]
    fn other_frames_do_not_spawn() {
        let mut body = TexturedTumble::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 0);
    }
}
