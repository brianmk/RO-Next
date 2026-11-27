//! `EF_BABY` (408) - `RagEffect.cpp:23325-23333`.
//!
//! The Baby (fire-vine) burst: frame 0 plays `effect\\EF_Blessing.wav` and then
//! calls `CRagEffect::FeatherBurst("effect\\fire_ivy.bmp", angle, 0)` five
//! times, for `angle = 0, 9, 18, 27, 36`
//! (`RagEffect.cpp:4187-4220`, prim 91 `PP_SPHEREWIND2`).
//!
//! Each call arms four slots around the caster: `riseAngle = slot * 45 +
//! angle`, a `ClampedRandom(20)`-frame stagger, `maxHeight = 2 + 0.2 *
//! ClampedRandom(11)`, `fullDisplayAngle = 180 + ClampedRandom(91)` and
//! `height[1] = 3 + ClampedRandom(4)`.
//!
//! Lifetimes: `EF_BABY` 200 (`EffectTypes.cpp:417`). Renderer
//! `RenderSphereWind` (`EffectPrimitiveRendering.cpp:5591`); motion
//! `PrimSphereWind2` (`EffectPrimitiveMotion.cpp:4096-4143`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `case EF_BABY:` dispatch - `RagEffect.cpp:23325-23333`.
#[derive(Debug)]
pub struct FeatherBurst {
    texture: &'static str,
    lifetime: i32,
}

impl FeatherBurst {
    pub const fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

/// The five ring angles of the `EF_BABY` loop.
const RING_ANGLES: [i16; 5] = [0, 9, 18, 27, 36];

/// One slot's five RNG draws of `FeatherBurst`, in the original order.
struct SlotDraws {
    process: i32,
    max_height: f32,
    rot_start: i16,
    full_display_angle: i16,
    height1: f32,
}

/// `CRagEffect::FeatherBurst(const char* name, int16_t angle, char)` -
/// `RagEffect.cpp:4187-4220`.
///
/// RNG order per slot: `ClampedRandom(20)` (stagger), `(11)` (height),
/// `(360)` (rotStart), `(91)` (display angle), `(4)` (step).
fn feather_burst(runtime: &mut EffectRuntime, name: &'static str, angle: i16) {
    // `const vector3d& origin = this->state.parent->render.position;`
    // `this->render.position = origin;`
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(91, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::SphereWind2, [0.0, 0.0, 0.0]);

    let mut draws = Vec::with_capacity(4);

    for _ in 0..4 {
        draws.push(SlotDraws {
            process: -runtime.rng.clamped_random(20),
            max_height: runtime.rng.clamped_random(11) as f32 * 0.2 + 2.0,
            rot_start: runtime.rng.clamped_random(360) as i16,
            full_display_angle: (runtime.rng.clamped_random(91) + 180) as i16,
            height1: (runtime.rng.clamped_random(4) + 3) as f32,
        });
    }

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = Some(name);
    state.size = 10.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for slot_index in 0..4usize {
        let draw = &draws[slot_index];
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = draw.process;
        slot.max_height = draw.max_height;
        slot.distance = 5.0;
        slot.height[0] = -10.0;
        slot.rot_start = draw.rot_start;
        slot.rise_angle = (slot_index as i32 * 45 + angle as i32) as i16;
        slot.alpha_b = 0;
        slot.full_display_angle = draw.full_display_angle;
        slot.height[1] = draw.height1;
    }
}

impl PortedEffect for FeatherBurst {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\EF_Blessing.wav");
        }

        for angle in RING_ANGLES {
            feather_burst(runtime, self.texture, angle);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    fn effect() -> FeatherBurst {
        FeatherBurst::new("effect\\fire_ivy.bmp", 200)
    }

    #[test]
    fn baby_is_registered_with_lifetime_200() {
        assert_eq!(effect().lifetime(), 200);
    }

    #[test]
    fn frame_zero_spawns_five_spheres_and_plays_the_blessing_sound() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Blessing.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 5);

        for (ring, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::SphereWind2);
            assert_eq!(state.texture, Some("effect\\fire_ivy.bmp"));
            assert_eq!(state.render_flag, 5);
            assert_eq!(state.size, 10.0);
            assert_eq!(state.total_texture, 1);

            for slot_index in 0..4usize {
                let slot = &state.slots[slot_index];
                assert_eq!(slot.life, 1);
                assert_eq!(slot.distance, 5.0);
                assert_eq!(slot.height[0], -10.0);
                assert_eq!(slot.rise_angle, (slot_index as i32 * 45 + (ring * 9) as i32) as i16);
                assert_eq!(slot.alpha_b, 0);
            }
        }
    }

    #[test]
    fn the_per_slot_rng_order_is_process_height_angle_display_step() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 13);

        let mut predictor = EffectRng::new(13);
        let mut expected = Vec::new();

        for _ in 0..4 {
            expected.push((
                -predictor.clamped_random(20),
                predictor.clamped_random(11) as f32 * 0.2 + 2.0,
                predictor.clamped_random(360) as i16,
                (predictor.clamped_random(91) + 180) as i16,
                (predictor.clamped_random(4) + 3) as f32,
            ));
        }

        // Call the body directly so the primitive pass does not advance the
        // freshly spawned slots.
        body.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;

        for (slot_index, (process, max_height, rot_start, display, height1)) in expected.iter().enumerate() {
            let slot = &state.slots[slot_index];
            assert_eq!(slot.process, *process);
            assert_eq!(slot.max_height, *max_height);
            assert_eq!(slot.rot_start, *rot_start);
            assert_eq!(slot.full_display_angle, *display);
            assert_eq!(slot.height[1], *height1);
        }
    }

    #[test]
    fn later_frames_neither_spawn_nor_replay_the_sound() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..3 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 5);
        assert_eq!(runtime.sounds.len(), 1);
    }

    #[test]
    fn each_ring_turns_its_slots_by_nine_degrees() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives[1].state.slots[0].rise_angle, 9);
        assert_eq!(runtime.primitives[2].state.slots[0].rise_angle, 18);
        assert_eq!(runtime.primitives[4].state.slots[3].rise_angle, 36 + 135);
    }
}
