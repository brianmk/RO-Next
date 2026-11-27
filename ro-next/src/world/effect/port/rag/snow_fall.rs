//! `CRagEffect::SnowFall(name, kind)` - `RagEffect.cpp:6615-6663`.
//!
//! Falling snow: a `PP_PARTICLE_UP` (59) batch of four `E9_snow03/E9_snow02`
//! flakes every eight frames while the effect runs.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct SnowFall {
    kind: i32,
    lifetime: i32,
}

impl SnowFall {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

/// One `SnowFall` call (frame-gated to `frame % 8 == 0` up to frame 100).
pub(crate) fn spawn(runtime: &mut EffectRuntime, kind: i32) -> bool {
    let frame = runtime.frame;

    if (frame as u32) > 100 || frame % 8 != 0 {
        return false;
    }

    let lifetime = runtime.lifetime;
    let position = EffectRuntime::official_position(runtime.position);
    let angles = Angles::shared();

    let mut distances = [0.0f32; 4];
    let mut radii = [0.0f32; 4];
    let mut angles_deg = [0i32; 4];
    let mut drops = [0.0f32; 4];
    let mut speeds = [0.0f32; 4];

    for index in 0..4usize {
        distances[index] = runtime.rng.clamped_random(6) as f32 * 0.1 + 1.5;
        radii[index] = runtime.rng.clamped_random(41) as f32;
        angles_deg[index] = runtime.rng.clamped_random(360);
        drops[index] = runtime.rng.clamped_random(31) as f32;
        speeds[index] = runtime.rng.clamped_random(20) as f32 * 0.01 + 0.1;
    }

    let snow = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(snow);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.texture = Some("effect\\E9_snow03.tga");
    state.texture_secondary = Some("effect\\E9_snow02.tga");
    state.size = 0.0;
    state.position = position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = 0;
        slot.distance = distances[index];
        slot.height[8] = 0.0;
        slot.alpha_b = 0;
        slot.vec_b_now[0] = position[0] + radii[index] * angles.raw_cosine(angles_deg[index] as usize);
        slot.vec_b_now[2] = position[2] + radii[index] * angles.raw_sine(angles_deg[index] as usize);
        slot.vec_b_now[1] = position[1] - 6.0 - drops[index];
        slot.rot_start = 0;
        slot.height[1] = 0.0;
        slot.height[2] = speeds[index];
        slot.full_display_angle = 0;
        slot.flag1[0] = 0;
        slot.flag1[1] = 1;
        slot.flag1[2] = match kind {
            0 => 11,
            1 | 2 => 1,
            3 => 13,
            _ => 0,
        };
        slot.height[7] = 0.0;
    }

    true
}

impl PortedEffect for SnowFall {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        spawn(runtime, self.kind);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn snow_spawns_every_eight_frames() {
        let mut effect = SnowFall::new(1, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 351);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        for _ in 0..7 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 1, "frame 7 is not a burst");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2, "frame 8 spawns");

        let snow = &runtime.primitives[0].state;
        assert_eq!(snow.count, PrimitiveId::ParticleUp);
        assert_eq!(snow.texture, Some("effect\\E9_snow03.tga"));
        assert_eq!(snow.texture_secondary, Some("effect\\E9_snow02.tga"));
        assert!(snow.slots.iter().all(|slot| slot.life == 1));
        assert!(snow.slots.iter().all(|slot| slot.flag1[1] == 1));
        assert!(snow.slots.iter().all(|slot| slot.flag1[2] == 1));
    }

    #[test]
    fn snow_stops_after_frame_one_hundred() {
        let mut effect = SnowFall::new(3, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 351);

        while runtime.frame < 101 {
            step(&mut effect, &mut runtime);
        }
        let at_101 = runtime.primitives.len();

        while runtime.frame < 140 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), at_101);
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 13);
    }
}
