//! `EF_SOULBREAKER` (361) - `RagEffect.cpp:10320-10332`.
//!
//! The Soul Breaker cast: `CRagEffect::SonicWave(0)`
//! (`RagEffect.cpp:5382-5426`, prim 86) every frame, a `effect\\기공포.wav` hit
//! sound on frame 20, and a magenta caster tint plus the additive body flag on
//! frames 0..10.
//!
//! [`sonic_wave`] is the full C++ body: `sonic == 1` selects the four
//! `sonic_wave1..4.tga` sheets, `sonic == 0` the single `purpleslash.tga`.
//! `EF_SOULBREAKER` is the `0` caller; the `1` callers are still unported, but
//! the branch is kept because it is the same function.
//!
//! Lifetimes: `EF_SOULBREAKER` 300 (`EffectTypes.cpp:370`). Renderer
//! `RenderFloorSquare` (`EffectPrimitiveRendering.cpp:2637`); motion the bare
//! `case 86:` (`EffectPrimitiveMotion.cpp:9315-9365`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `case EF_SOULBREAKER:` dispatch - `RagEffect.cpp:10320-10332`.
#[derive(Debug)]
pub struct SoulBreaker {
    lifetime: i32,
}

impl SoulBreaker {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// The four `sonic_wave*.tga` sheets of `SonicWave(1)`.
const SONIC_WAVE_TEXTURES: [&str; 4] = [
    "effect\\sonic_wave1.tga",
    "effect\\sonic_wave2.tga",
    "effect\\sonic_wave3.tga",
    "effect\\sonic_wave4.tga",
];

/// `CRagEffect::SonicWave(char sonic)` - `RagEffect.cpp:5382-5426`.
///
/// RNG order: one `ClampedRandom(5)` vertical jitter per slot, in slot order.
fn sonic_wave(runtime: &mut EffectRuntime, sonic: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = runtime.position;
    let target = runtime.target_offset;

    // `this->state.lastPrim = LaunchEffectPrim(86, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::SonicWave, [0.0, 0.0, 0.0]);

    let mut jitter = [0.0f32; 4];

    for value in &mut jitter {
        *value = runtime.rng.clamped_random(5) as f32;
    }

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.size = 0.0;
    state.delta_pos = target;
    state.position = position;

    if sonic == 1 {
        state.total_texture = 4;
        state.texture_frames = SONIC_WAVE_TEXTURES.to_vec();
    } else {
        state.total_texture = 1;
        state.texture = Some("effect\\purpleslash.tga");
    }

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.alpha_b = 0;
        slot.process = slot_index as i32 - 28;
        slot.distance = 20.0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = jitter[slot_index] + (position[1] - 10.0);
        slot.rot_start = 0;
        slot.height[1] = 0.8;
        slot.flag1[0] = 0;
        slot.flag1[1] = sonic;
    }
}

impl PortedEffect for SoulBreaker {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 20 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\기공포.wav");
        }

        if (0..=10).contains(&runtime.frame) {
            if (runtime.parent_flags & 1) == 0 {
                runtime.add_parent_flags(1);
            }

            runtime.set_parent_color(-1, 255, 0, 255);
        }

        sonic_wave(runtime, 0);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn soul_breaker_is_registered_with_lifetime_300() {
        assert_eq!(SoulBreaker::new(300).lifetime(), 300);
    }

    #[test]
    fn frame_zero_spawns_the_four_staggered_shards() {
        let mut effect = SoulBreaker::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);
        runtime.position = [3.0, 4.0, 5.0];
        runtime.target_offset = [13.0, 0.0, 5.0];

        let mut predictor = EffectRng::new(7);
        let expected_jitter: Vec<f32> = (0..4).map(|_| predictor.clamped_random(5) as f32).collect();

        // Call the body directly so the primitive pass does not advance the
        // freshly spawned shards.
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::SonicWave);
        assert_eq!(state.texture, Some("effect\\purpleslash.tga"));
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.total_texture, 1);
        assert_eq!(state.delta_pos, [13.0, 0.0, 5.0]);
        assert_eq!(state.position, [3.0, 4.0, 5.0], "the body writes the anchor once");

        for index in 0..4usize {
            let slot = &state.slots[index];
            assert_eq!(slot.life, 1);
            assert_eq!(slot.process, index as i32 - 28);
            assert_eq!(slot.distance, 20.0);
            assert_eq!(slot.vec_b_now[0], 3.0);
            assert_eq!(slot.vec_b_now[2], 5.0);
            assert_eq!(slot.vec_b_now[1], expected_jitter[index] + 4.0 - 10.0);
            assert_eq!(slot.height[1], 0.8);
            assert_eq!(slot.flag1[1], 0);
        }
    }

    #[test]
    fn the_caster_tint_runs_through_frame_ten() {
        let mut effect = SoulBreaker::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 255, 0, 255]));
        assert_eq!(runtime.parent_flags & 1, 1);

        runtime.frame = 10;
        runtime.parent_flags = 0;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 255, 0, 255]));

        runtime.frame = 11;
        runtime.parent_color = None;
        runtime.parent_flags = 0;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);
    }

    #[test]
    fn frame_twenty_plays_the_qi_blast_sound_without_spawning() {
        let mut effect = SoulBreaker::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);
        runtime.frame = 20;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\기공포.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 0);
    }

    #[test]
    fn sonic_one_uses_the_four_sheet_animation() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        sonic_wave(&mut runtime, 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.total_texture, 4);
        assert_eq!(state.texture_frames, SONIC_WAVE_TEXTURES.to_vec());
        assert_eq!(state.texture, None);
        assert_eq!(state.slots[0].flag1[1], 1);
        assert_eq!(state.slots[3].flag1[1], 1);
    }
}
