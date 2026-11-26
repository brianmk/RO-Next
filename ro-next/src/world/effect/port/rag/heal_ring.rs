//! `CRagEffect::HealRingSparkle(name, type)` - `RagEffect.cpp:17970-18186`.
//!
//! `EF_HEAL2` (type 0): a white ring column plus spark bursts, and for type 1 a
//! second, taller `alpha_down` column on frame 1.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct HealRingSparkle {
    ring_texture: &'static str,
    sparkle_type: i32,
    lifetime: i32,
}

impl HealRingSparkle {
    pub fn new(ring_texture: &'static str, sparkle_type: i32, lifetime: i32) -> Self {
        Self {
            ring_texture,
            sparkle_type,
            lifetime,
        }
    }

    fn spawn_ring(&self, runtime: &mut EffectRuntime) {
        let kind = self.sparkle_type;
        let lifetime = runtime.lifetime;

        // slots 2 and 3 draw their rotation at spawn.
        let rot2 = runtime.rng.clamped_random(360);
        let rot3 = runtime.rng.clamped_random(360);

        let ring = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.ring_texture);
        state.size = if kind == 2 { 4.0 } else { 9.0 };

        let alpha_t = if kind == 0 { 60 } else { 40 };
        let small_height = if kind == 0 { 12.0 } else { 18.0 };

        for slot in &mut state.slots {
            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.alpha_b = 0;
            slot.vec_b_now[1] = 0.0;
            slot.alpha_t = alpha_t;
            slot.height = [0.0; 21];
            slot.flag1 = [0; 21];
        }

        state.slots[0].max_height = 60.0;
        state.slots[0].rot_start = 0;
        state.slots[0].distance = 4.6;
        state.slots[0].rise_angle = 90;

        state.slots[1].max_height = 60.0;
        state.slots[1].rot_start = 180;
        state.slots[1].distance = 5.0;
        state.slots[1].rise_angle = 90;

        state.slots[2].max_height = small_height;
        state.slots[2].rot_start = rot2 as i16;
        state.slots[2].distance = 5.0;
        state.slots[2].rise_angle = 50;

        state.slots[3].max_height = small_height;
        state.slots[3].rot_start = rot3 as i16;
        state.slots[3].distance = 5.2;
        state.slots[3].rise_angle = 48;

        if kind < 2 {
            runtime.play_sound("_heal_effect.wav");
        }
    }

    fn spawn_sparks(&self, runtime: &mut EffectRuntime) {
        let kind = self.sparkle_type;
        let lifetime = runtime.lifetime;
        let position = runtime.position;

        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut speeds = [0.0f32; 4];

        for index in 0..4usize {
            distances[index] = runtime.rng.clamped_random(10) as f32 * 0.1 + 2.0;
            xs[index] = runtime.rng.clamped_random(13) as f32 + position[0] - 6.0;
            zs[index] = runtime.rng.clamped_random(13) as f32 + position[2] - 6.0;
            speeds[index] = runtime.rng.clamped_random(40) as f32 * 0.01 + if kind == 0 { 0.4 } else { 0.6 };
        }

        let sparks = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(sparks);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some("effect\\pok1.tga");
        state.texture_secondary = Some("effect\\pok3.tga");
        state.size = 0.0;
        state.position = position;

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.distance = distances[index];
            slot.height[8] = 0.0;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = xs[index];
            slot.vec_b_now[2] = zs[index];
            slot.vec_b_now[1] = position[1];
            slot.rot_start = 0;
            slot.height[1] = 0.0;
            slot.height[2] = speeds[index];
            slot.full_display_angle = 0;
            slot.flag1[0] = 0;
            slot.flag1[1] = 1;
            slot.flag1[2] = 11;
            slot.height[7] = 0.0;
        }
    }

    fn spawn_tall_ring(&self, runtime: &mut EffectRuntime) {
        let lifetime = runtime.lifetime;
        let rot2 = runtime.rng.clamped_random(360);
        let rot3 = runtime.rng.clamped_random(360);

        let ring = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some("effect\\alpha_down.tga");
        state.size = 9.0;

        for slot in &mut state.slots {
            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.alpha_b = 0;
            slot.vec_b_now[1] = 0.0;
            slot.height = [0.0; 21];
            slot.flag1 = [0; 21];
        }

        state.slots[0].max_height = 90.0;
        state.slots[0].rot_start = 0;
        state.slots[0].distance = 4.2;
        state.slots[0].rise_angle = 90;
        state.slots[0].alpha_t = 40;

        state.slots[1].max_height = 90.0;
        state.slots[1].rot_start = 90;
        state.slots[1].distance = 4.3;
        state.slots[1].rise_angle = 90;
        state.slots[1].alpha_t = 40;

        state.slots[2].max_height = 10.0;
        state.slots[2].rot_start = rot2 as i16;
        state.slots[2].distance = 5.3;
        state.slots[2].rise_angle = 46;
        state.slots[2].alpha_t = 30;

        state.slots[3].max_height = 10.0;
        state.slots[3].rot_start = rot3 as i16;
        state.slots[3].distance = 5.4;
        state.slots[3].rise_angle = 44;
        state.slots[3].alpha_t = 30;
    }
}

impl PortedEffect for HealRingSparkle {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let kind = self.sparkle_type;

        if frame == 0 {
            self.spawn_ring(runtime);
        } else if frame <= 50 && frame % 6 == 0 && kind < 2 {
            self.spawn_sparks(runtime);
        }

        if kind == 1 && frame == 1 {
            self.spawn_tall_ring(runtime);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn heal2_raises_four_ring_slots() {
        let mut effect = HealRingSparkle::new("effect\\ring_white.tga", 0, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 241);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1, "no sparks on frame 0");
        assert_eq!(runtime.sounds, vec!["_heal_effect.wav".to_owned()]);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::PulsingColumns);
        assert_eq!(ring.size, 9.0);
        assert_eq!(ring.texture, Some("effect\\ring_white.tga"));
        assert!(ring.slots.iter().all(|slot| slot.life == 1));
        assert!(ring.slots.iter().all(|slot| slot.alpha_t == 60));
        assert_eq!(ring.slots[0].max_height, 60.0);
        assert_eq!(ring.slots[1].rot_start, 180);
        assert_eq!(ring.slots[2].max_height, 12.0);
        assert_eq!(ring.slots[2].rise_angle, 50);
        assert_eq!(ring.slots[3].rise_angle, 48);
    }

    #[test]
    fn sparks_start_on_frame_six() {
        let mut effect = HealRingSparkle::new("effect\\ring_white.tga", 0, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 241);

        while runtime.frame < 6 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        let sparks = &runtime.primitives[1].state;
        assert_eq!(sparks.count, PrimitiveId::ParticleUp);
        assert_eq!(sparks.texture, Some("effect\\pok1.tga"));
        assert_eq!(sparks.texture_secondary, Some("effect\\pok3.tga"));
        assert!(sparks.slots.iter().all(|slot| slot.flag1[1] == 1));
    }
}
