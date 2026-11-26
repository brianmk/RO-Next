//! `CRagEffect::HealBurst(name, flag)` - `RagEffect.cpp:18215-18362`.
//!
//! The Heal pillar: on frame 0 a `PP_PULSING_COLUMNS` (60) column pair rises
//! with `alpha_down.tga`; from frame 0 to the limit, every six frames a
//! `PP_PARTICLE_UP` (59) sprays `pok1.tga` sparks around the caster.
//!
//! `flag == 0` is `EF_HEAL`, `flag == 1` is `EF_HEAL3` (the higher-level heal,
//! with two textures and a taller column).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct HealBurst {
    flag: i32,
    lifetime: i32,
}

impl HealBurst {
    pub fn new(flag: i32, lifetime: i32) -> Self {
        Self { flag, lifetime }
    }

    fn spawn_column(&self, runtime: &mut EffectRuntime) {
        let flag = self.flag;
        let lifetime = runtime.lifetime;

        // slot 0 draws rotStart, then (for flag 1) maxHeight and distance.
        let rot0 = runtime.rng.clamped_random(360);
        let (max_height0, distance0) = if flag == 1 {
            let height = runtime.rng.clamped_random(20) as f32 + 40.0;
            let mut distance = 4.5;
            distance += runtime.rng.clamped_random(100) as f32 * 0.005;
            (height, distance)
        } else {
            (40.0, 4.6)
        };

        let rot1 = runtime.rng.clamped_random(360);

        let column = runtime.launch(PrimitiveId::PulsingColumns, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(column);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some("effect\\alpha_down.tga");
        state.size = if flag == 0 { 9.0 } else { 5.0 };

        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].full_display_angle = 360;
        state.slots[0].rot_start = rot0 as i16;
        state.slots[0].alpha_b = 0;
        state.slots[0].rise_angle = 90;
        state.slots[0].vec_b_now[1] = 0.0;
        state.slots[0].alpha_t = if flag == 0 { 60 } else { 30 };
        state.slots[0].max_height = max_height0;
        state.slots[0].distance = distance0;
        state.slots[0].height = [0.0; 21];
        state.slots[0].flag1 = [0; 21];

        state.slots[1].life = 1;
        state.slots[1].process = 0;
        state.slots[1].full_display_angle = 360;
        state.slots[1].rot_start = rot1 as i16;
        state.slots[1].alpha_b = 0;
        state.slots[1].rise_angle = 90;
        state.slots[1].vec_b_now[1] = 0.0;
        state.slots[1].alpha_t = if flag == 0 { 60 } else { 30 };
        state.slots[1].max_height = if flag == 0 { 40.0 } else { max_height0 };
        state.slots[1].distance = if flag == 0 { 4.8 } else { distance0 + 0.2 };
        state.slots[1].height = [0.0; 21];
        state.slots[1].flag1 = [0; 21];

        state.slots[2].life = 0;
        state.slots[3].life = 0;

        runtime.play_sound("_heal_effect.wav");
    }

    fn spawn_sparks(&self, runtime: &mut EffectRuntime) {
        let flag = self.flag;
        let lifetime = runtime.lifetime;
        let position = runtime.position;

        // Per-slot random draws in slot order.
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut speeds = [0.0f32; 4];

        for index in 0..4usize {
            distances[index] = runtime.rng.clamped_random(10) as f32 * 0.1 + 2.0;
            xs[index] = runtime.rng.clamped_random(13) as f32 + position[0] - 6.0;
            zs[index] = runtime.rng.clamped_random(13) as f32 + position[2] - 6.0;
            speeds[index] = runtime.rng.clamped_random(40) as f32 * 0.01 + if flag == 0 { 0.3 } else { 0.4 };
        }

        let sparks = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(sparks);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some("effect\\pok1.tga");
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
            slot.flag1[1] = if flag == 1 { 1 } else { 0 };
            slot.flag1[2] = 11;
            slot.height[7] = 0.0;
        }
    }
}

impl PortedEffect for HealBurst {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            self.spawn_column(runtime);
        }

        let limit = if self.flag != 0 { 25 } else { 50 };

        if frame > limit {
            return true;
        }

        if frame % 6 != 0 {
            return true;
        }

        self.spawn_sparks(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn frame_zero_raises_the_heal_column() {
        let mut effect = HealBurst::new(0, 100);
        let mut runtime = EffectRuntime::new(100, [3.0, 4.0], 231);

        step(&mut effect, &mut runtime);

        // Slot 0 column + slot 1 pillar + first spark burst.
        assert_eq!(runtime.primitives.len(), 2, "column and sparks");
        assert_eq!(runtime.sounds, vec!["_heal_effect.wav".to_owned()]);

        let column = &runtime.primitives[0].state;
        assert_eq!(column.count, PrimitiveId::PulsingColumns);
        assert_eq!(column.duration, 100);
        assert_eq!(column.size, 9.0);
        assert_eq!(column.render_flag, 5);
        assert_eq!(column.texture, Some("effect\\alpha_down.tga"));
        assert_eq!(column.slots[0].alpha_t, 60);
        assert_eq!(column.slots[0].max_height, 40.0);
        assert_eq!(column.slots[0].distance, 4.6);
        assert_eq!(column.slots[1].alpha_t, 60);
        assert_eq!(column.slots[1].distance, 4.8);
        assert_eq!(column.slots[2].life, 0);
        assert_eq!(column.slots[3].life, 0);
    }

    #[test]
    fn sparks_spawn_every_six_frames() {
        let mut effect = HealBurst::new(0, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 231);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2, "frame 5 has no burst");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 3, "frame 6 spawns one");

        let sparks = &runtime.primitives[2].state;
        assert_eq!(sparks.count, PrimitiveId::ParticleUp);
        assert_eq!(sparks.texture, Some("effect\\pok1.tga"));
        assert!(sparks.slots.iter().all(|slot| slot.life == 1));
        assert!(sparks.slots.iter().all(|slot| slot.flag1[2] == 11));
    }

    #[test]
    fn heal3_uses_the_short_limit_and_two_textures() {
        let mut effect = HealBurst::new(1, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 231);

        step(&mut effect, &mut runtime);

        let column = &runtime.primitives[0].state;
        assert_eq!(column.size, 5.0);
        assert_eq!(column.slots[0].alpha_t, 30);
        assert!((column.slots[0].max_height - 40.0).abs() <= 20.0);
        assert!((column.slots[0].distance - 4.5).abs() <= 0.5);

        let sparks = &runtime.primitives[1].state;
        assert!(sparks.slots.iter().all(|slot| slot.flag1[1] == 1));

        // The flag-1 variant stops at frame 25: past it no further bursts.
        while runtime.frame < 26 {
            step(&mut effect, &mut runtime);
        }
        let before = runtime.primitives.len();
        while runtime.frame < 40 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), before, "no bursts past frame 25");
    }
}
