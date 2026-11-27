//! `CRagEffect::TexturedCross` - `RagEffect.cpp:4993-5030`.
//!
//! `EF_PROVIDENCE` (`RagEffect.cpp:24609-24614`) calls it with
//! `effect\ring_yellow.tga` and kind 0 on **every** frame, then runs
//! `ProcessEZ2STR(0, -80, 0)` and retires when the `.str` clip ends.
//!
//! The body only spawns on frame 0: one prim 41 with `RSF_ALPHA |
//! RSF_CUSTOM_BLEND`, `size = 5`, four slots at `process = 200`,
//! `riseAngle = 83 - index`, `distance = 6 + index * 0.5`, a shared
//! `maxHeight = 120`, `alphaB = 40`, `alphaT = 1`, and all 21 height samples
//! and flags zeroed. Kind other than 0 returns after the launch, exactly like
//! the original.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedCross(const char* name, char kind)` -
/// `RagEffect.cpp:4993-5030`.
fn textured_cross(runtime: &mut EffectRuntime, name: &'static str, kind: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::TexturedCross, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 5.0;

    if kind != 0 {
        return;
    }

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 200;
        slot.full_display_angle = 360;
        slot.max_height = 120.0;
        slot.rot_start = (slot_index as i32 * 90) as i16;
        slot.alpha_b = 40;
        slot.distance = slot_index as f32 * 0.5 + 6.0;
        slot.rise_angle = (83 - slot_index as i32) as i16;
        slot.alpha_t = 1;
        slot.vec_b_now[1] = 0.0;

        for height in 0..21usize {
            slot.height[height] = 0.0;
            slot.flag1[height] = 0;
        }
    }
}

/// `case EF_PROVIDENCE:` - `RagEffect.cpp:24609-24614`.
#[derive(Debug)]
pub struct Providence {
    lifetime: i32,
}

impl Providence {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Providence {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        textured_cross(runtime, "effect\\ring_yellow.tga", 0);

        runtime.process_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providence_launches_one_prim41_on_frame_zero() {
        let mut body = Providence::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);

        body.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::TexturedCross);
        assert_eq!(state.count.id(), 41);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.duration, 200);
        assert_eq!(state.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(state.size, 5.0);
        assert_eq!(state.position, [10.0, 20.0, 30.0]);

        let first = &state.slots[0];
        assert_eq!(first.life, 1);
        assert_eq!(first.process, 200);
        assert_eq!(first.full_display_angle, 360);
        assert_eq!(first.max_height, 120.0);
        assert_eq!(first.rot_start, 0);
        assert_eq!(first.alpha_b, 40);
        assert_eq!(first.distance, 6.0);
        assert_eq!(first.rise_angle, 83);
        assert_eq!(first.alpha_t, 1);

        let fourth = &state.slots[3];
        assert_eq!(fourth.rot_start, 270);
        assert_eq!(fourth.distance, 7.5);
        assert_eq!(fourth.rise_angle, 80);
        assert_eq!(fourth.height, [0.0; 21]);
        assert_eq!(fourth.flag1, [0; 21]);

        let count = runtime.primitives.len();
        runtime.frame = 1;
        body.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), count);
    }

    #[test]
    fn non_zero_kinds_launch_without_arming_a_slot() {
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);

        textured_cross(&mut runtime, "effect\\ring_yellow.tga", 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture, Some("effect\\ring_yellow.tga"));
        assert!(state.slots.iter().all(|slot| slot.life == 0));
    }
}
