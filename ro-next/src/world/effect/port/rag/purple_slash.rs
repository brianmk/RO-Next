//! `EF_SOULBREAKER2` (409) - `RagEffect.cpp:24333-24347`.
//!
//! Eight `CRagEffect::PurpleSlash(angle)` calls, one per 45-degree step
//! (`RagEffect.cpp:3382-3411`, prim 92), plus the frame-0
//! `effect\\메테오 어썰트.wav` sound and the magenta caster tint on frames
//! 0..10.
//!
//! Each call spawns its own prim 92 with four shards staggered by
//! `index - 28 - ClampedRandom(6)`; the shards sit at the caster's feet
//! (`position.y - 10 + ClampedRandom(5)`) and travel along the given angle.
//!
//! Lifetimes: `EF_SOULBREAKER2` 300 (`EffectTypes.cpp:418`). Renderer
//! `RenderFloorSquare` (`EffectPrimitiveRendering.cpp:2637`); motion the bare
//! `case 92:` (`EffectPrimitiveMotion.cpp:9367-9399`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `case EF_SOULBREAKER2:` dispatch - `RagEffect.cpp:24333-24347`.
#[derive(Debug)]
pub struct SoulBreaker2 {
    lifetime: i32,
}

impl SoulBreaker2 {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// `CRagEffect::PurpleSlash(int angle)` - `RagEffect.cpp:3382-3411`.
///
/// RNG order: `offset = ClampedRandom(6)` once, then one `ClampedRandom(5)`
/// vertical jitter per slot.
fn purple_slash(runtime: &mut EffectRuntime, angle: i16) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = runtime.position;

    // `this->state.lastPrim = LaunchEffectPrim(92, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::PurpleSlash, [0.0, 0.0, 0.0]);

    let offset = runtime.rng.clamped_random(6);
    let mut jitter = [0.0f32; 4];

    for value in &mut jitter {
        *value = runtime.rng.clamped_random(5) as f32;
    }

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = Some("effect\\purpleslash.tga");
    state.size = 0.0;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = slot_index as i32 - 28 - offset;
        slot.distance = 20.0;
        slot.alpha_b = 0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = jitter[slot_index] + (position[1] - 10.0);
        slot.rot_start = angle;
        slot.height[1] = 0.8;
        slot.flag1[0] = 0;
        slot.flag1[1] = 0;
    }
}

impl PortedEffect for SoulBreaker2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\메테오 어썰트.wav");
        }

        if (0..=10).contains(&runtime.frame) {
            if (runtime.parent_flags & 1) == 0 {
                runtime.add_parent_flags(1);
            }

            runtime.set_parent_color(-1, 255, 0, 255);
        }

        for angle in (0..360).step_by(45) {
            purple_slash(runtime, angle as i16);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn soul_breaker_two_is_registered_with_lifetime_300() {
        assert_eq!(SoulBreaker2::new(300).lifetime(), 300);
    }

    #[test]
    fn frame_zero_spawns_eight_slashes_one_per_forty_five_degrees() {
        let mut effect = SoulBreaker2::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);
        runtime.position = [2.0, 6.0, 4.0];

        let mut predictor = EffectRng::new(7);
        let mut expected = Vec::new();

        // Eight calls, each drawing its own offset then four jitters.
        for _ in 0..8 {
            let offset = predictor.clamped_random(6);
            let jitter: Vec<f32> = (0..4).map(|_| predictor.clamped_random(5) as f32).collect();
            expected.push((offset, jitter));
        }

        // Call the body directly so the primitive pass does not advance the
        // freshly spawned shards.
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 8);

        for (slash, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::PurpleSlash);
            assert_eq!(state.texture, Some("effect\\purpleslash.tga"));
            assert_eq!(state.render_flag, 5);
            assert_eq!(state.total_texture, 1);
            assert_eq!(state.size, 0.0);

            let (offset, jitter) = &expected[slash];

            for index in 0..4usize {
                let slot = &state.slots[index];
                assert_eq!(slot.life, 1);
                assert_eq!(slot.process, index as i32 - 28 - offset);
                assert_eq!(slot.distance, 20.0);
                assert_eq!(slot.vec_b_now[0], 2.0);
                assert_eq!(slot.vec_b_now[2], 4.0);
                assert_eq!(slot.vec_b_now[1], jitter[index] + 6.0 - 10.0);
                assert_eq!(slot.rot_start, (slash * 45) as i16);
                assert_eq!(slot.height[1], 0.8);
                assert_eq!(slot.flag1[0], 0);
                assert_eq!(slot.flag1[1], 0);
            }
        }
    }

    #[test]
    fn the_caster_tint_runs_through_frame_ten() {
        let mut effect = SoulBreaker2::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 255, 0, 255]));
        assert_eq!(runtime.parent_flags & 1, 1);

        runtime.frame = 11;
        runtime.parent_color = None;
        runtime.parent_flags = 0;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);
    }

    #[test]
    fn frame_zero_plays_the_meteor_assault_sound() {
        let mut effect = SoulBreaker2::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\메테오 어썰트.wav".to_string()]);
    }

    #[test]
    fn later_frames_neither_spawn_nor_play_again() {
        let mut effect = SoulBreaker2::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        for _ in 0..3 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 8);
        assert_eq!(runtime.sounds.len(), 1);
    }
}
