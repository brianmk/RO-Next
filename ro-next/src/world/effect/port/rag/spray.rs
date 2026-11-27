//! `CRagEffect::TexturedSprayKind(name, spin, kind)` -
//! `RagEffect.cpp:4642-4689` and `CRagEffect::TexturedSpray(name)` -
//! `RagEffect.cpp:4064-4097`.
//!
//! The `PP_INTIMIDATE` (43) spray used by the Rogue coin effects and the water
//! ball family. Both helpers bail out unless the effect is on frame 0, so any
//! dispatch block that calls them on later frames renders nothing - a quirk
//! the port keeps.

use super::super::primitive::PrimitiveId;
use super::super::runtime::EffectRuntime;

/// One `TexturedSprayKind` call. Returns `false` when the frame gate rejects
/// it.
pub(crate) fn spray_kind(runtime: &mut EffectRuntime, name: &'static str, spin: f32, kind: i32) -> bool {
    if runtime.frame != 0 {
        return false;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);

    let mut processes = [0i32; 4];
    let mut angles = [[0i16; 4]; 4];

    for index in 0..4usize {
        processes[index] = -18 - runtime.rng.clamped_random(30);
        angles[index] = [
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
        ];
    }

    let spray = runtime.launch(PrimitiveId::Intimidate, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(spray);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.texture = Some(name);
    state.position[1] -= 12.0;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.vec_b_now = target;
        slot.process = processes[index];
        slot.distance = 12.0;
        slot.height[0] = 1.0;
        slot.rise_angle = angles[index][0];
        slot.full_display_angle = angles[index][1];
        slot.rot_start = angles[index][2];
        slot.height[2] = angles[index][3] as f32;
        slot.height[1] = spin;
        slot.alpha_b = 0;
        slot.flag1[0] = match kind {
            1 => 6,
            7 => 7,
            8 => 8,
            9 => 9,
            _ => 2,
        };
    }

    true
}

/// `CRagEffect::TexturedSpray(name)` - the `height[0] = 1.5` variant with
/// `flag1[0] = 1`.
pub(crate) fn spray(runtime: &mut EffectRuntime, name: &'static str) -> bool {
    if runtime.frame != 0 {
        return false;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);

    let mut processes = [0i32; 4];
    let mut angles = [[0i16; 4]; 4];

    for index in 0..4usize {
        processes[index] = -18 - runtime.rng.clamped_random(30);
        angles[index] = [
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
            runtime.rng.clamped_random(360) as i16,
        ];
    }

    let spray = runtime.launch(PrimitiveId::Intimidate, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(spray);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.position[1] -= 12.0;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.vec_b_now = target;
        slot.process = processes[index];
        slot.distance = 12.0;
        slot.height[0] = 1.5;
        slot.rise_angle = angles[index][0];
        slot.full_display_angle = angles[index][1];
        slot.rot_start = angles[index][2];
        slot.height[2] = angles[index][3] as f32;
        slot.height[1] = 3.5;
        slot.alpha_b = 0;
        slot.flag1[0] = 1;
    }

    true
}

#[derive(Debug)]
pub struct SprayKind {
    name: &'static str,
    spin: f32,
    kind: i32,
    lifetime: i32,
}

impl SprayKind {
    pub fn new(name: &'static str, spin: f32, kind: i32, lifetime: i32) -> Self {
        Self {
            name,
            spin,
            kind,
            lifetime,
        }
    }
}

impl super::super::runtime::PortedEffect for SprayKind {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        spray_kind(runtime, self.name, self.spin, self.kind);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn spray_kind_spawns_four_slots_on_frame_zero() {
        let mut effect = SprayKind::new("effect\\shield.bmp", 5.0, 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 331);
        runtime.target_offset = [1.0, 0.0, 2.0];

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        let spray = &runtime.primitives[0].state;
        assert_eq!(spray.count, PrimitiveId::Intimidate);
        assert_eq!(spray.duration, 300);
        assert_eq!(spray.texture, Some("effect\\shield.bmp"));
        assert_eq!(spray.render_flag, 5);
        assert!(spray.slots.iter().all(|slot| slot.life == 1));
        assert!(spray.slots.iter().all(|slot| slot.flag1[0] == 2));
        assert!(spray.slots.iter().all(|slot| (slot.height[1] - 5.0).abs() < 1e-6));
    }

    #[test]
    fn later_frames_spawn_nothing() {
        let mut effect = SprayKind::new("effect\\shield.bmp", 5.0, 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 331);

        step(&mut effect, &mut runtime);
        let before = runtime.primitives.len();

        for _ in 0..12 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), before);
    }
}
