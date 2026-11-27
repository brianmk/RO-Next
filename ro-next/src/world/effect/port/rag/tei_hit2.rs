//! `EF_TEIHIT2` (264) and `EF_BACKSTAP` (275) - the two dispatch cases of
//! `CRagEffect::TEIHIT2()` (`RagEffect.cpp:12040-12085`).
//!
//! ```cpp
//! case EF_TEIHIT2:
//!   for (int index = 0; index < 5; ++index) this->TEIHIT2();
//!   break;
//! case EF_BACKSTAP:
//!   if (this->state.stateCnt == 20) {
//!     this->render.position = this->state.parent->render.position;
//!     PlayEffectWaveInline(*this, "effect\\rog_back stap.wav");
//!   }
//!   for (int index = 0; index < 20; ++index) this->TEIHIT2();
//!   break;
//! ```
//!
//! `TEIHIT2` itself copies the parent position, gates on `stateCnt == 0` and
//! then launches one prim 45 (`PP_TEIHIT2`,
//! [`super::super::primitive::tei_hit2`]) with `effect\thunder_center.bmp`,
//! `size = 0`, the effect's target offset in `deltaPos` and four `0.05`-scaled
//! flurry slots:
//!
//! ```cpp
//! process   = ClampedRandom(10) - 40;
//! distance  = ClampedRandom(16) * 0.05f + 0.1f;
//! height[1] = ClampedRandom(50) * 0.01f + 0.75f;   // horizontal step
//! height[2] = ClampedRandom(200) * 0.01f - 1.0f;   // vertical step
//! ```
//!
//! Because every `TEIHIT2()` call copies the parent position *before* the
//! `stateCnt != 0` gate, `EF_BACKSTAP` still performs twenty parent copies on
//! every frame; only frame 0 launches primitives. The frame-20 wave is the
//! only other event. This is the original's behaviour, kept verbatim.
//!
//! Renderer: the shared `RenderFieldPanel`
//! (`EffectPrimitiveRendering.cpp:3312-3505`; `PP_TEIHIT2` at `:574`).
//!
//! Lifetimes: both 300 (`EffectTypes.cpp:273`, `:284`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// One dispatch case of `CRagEffect::TEIHIT2`; `count` is the loop bound.
#[derive(Debug)]
pub struct TeiHit2 {
    count: usize,
    backstap: bool,
    lifetime: i32,
}

impl TeiHit2 {
    pub const fn new(count: usize, backstap: bool, lifetime: i32) -> Self {
        Self { count, backstap, lifetime }
    }
}

/// `CRagEffect::TEIHIT2()` - `RagEffect.cpp:12040-12085`.
fn tei_hit2(runtime: &mut EffectRuntime) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);

    let index = runtime.launch(PrimitiveId::TeiHit2, [0.0, 0.0, 0.0]);

    // RNG order per slot: process (10), distance (16), height[1] (50),
    // height[2] (200).
    let mut processes = [0i32; 4];
    let mut distances = [0.0f32; 4];
    let mut sides = [0.0f32; 4];
    let mut rises = [0.0f32; 4];

    for slot_index in 0..4usize {
        processes[slot_index] = runtime.rng.clamped_random(10) - 40;
        distances[slot_index] = runtime.rng.clamped_random(16) as f32 * 0.05 + 0.1;
        sides[slot_index] = runtime.rng.clamped_random(50) as f32 * 0.01 + 0.75;
        rises[slot_index] = runtime.rng.clamped_random(200) as f32 * 0.01 - 1.0;
    }

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some("effect\\thunder_center.bmp");
    // `state.size = 0.0f;`
    state.size = 0.0;
    // `state.deltaPos = this->state.deltaPos;`
    state.delta_pos = target;
    // `CopyVector(this->state.lastPrim->render.position, this->render.position);`
    state.position = position;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = processes[slot_index];
        slot.distance = distances[slot_index];
        slot.height[8] = 0.0;
        slot.alpha_b = 0;
        slot.vec_b_now = [target[0], target[1] - 8.0, target[2]];
        slot.rot_start = 0;
        slot.height[1] = sides[slot_index];
        slot.height[2] = rises[slot_index];
        slot.full_display_angle = 0;
        slot.flag1[0] = 0;
        slot.flag1[1] = 0;
        slot.flag1[2] = 2;
        slot.height[7] = 0.0;
    }
}

impl PortedEffect for TeiHit2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if self.backstap && runtime.frame == 20 {
            // `this->render.position = this->state.parent->render.position;`
            runtime.copy_parent_position();
            runtime.play_sound("effect\\rog_back stap.wav");
        }

        for _ in 0..self.count {
            tei_hit2(runtime);
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
    fn teihit2_launches_five_flurry_primitives_on_frame_zero() {
        let mut effect = TeiHit2::new(5, false, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);

        // Predict the first primitive's four slots.
        let mut expected = EffectRng::new(11);
        let mut processes = [0i32; 4];
        let mut distances = [0.0f32; 4];
        let mut sides = [0.0f32; 4];
        let mut rises = [0.0f32; 4];

        for slot_index in 0..4usize {
            processes[slot_index] = expected.clamped_random(10) - 40;
            distances[slot_index] = expected.clamped_random(16) as f32 * 0.05 + 0.1;
            sides[slot_index] = expected.clamped_random(50) as f32 * 0.01 + 0.75;
            rises[slot_index] = expected.clamped_random(200) as f32 * 0.01 - 1.0;
        }

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 5);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::TeiHit2);
        assert_eq!(state.texture, Some("effect\\thunder_center.bmp"));
        assert_eq!(state.size, 0.0);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.duration, 300);
        assert_eq!(state.delta_pos, [0.0; 3]);
        assert_eq!(state.position, [0.0, 0.0, 0.0]);

        for (slot_index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            // The primitive pass runs in the same step: process +1, and every
            // seeded process is still negative, so nothing else moves.
            assert_eq!(slot.process, processes[slot_index] + 1);
            assert_eq!(slot.distance, distances[slot_index]);
            assert_eq!(slot.height[1], sides[slot_index]);
            assert_eq!(slot.height[2], rises[slot_index]);
            assert_eq!(slot.height[7], 0.0);
            assert_eq!(slot.height[8], 0.0);
            assert_eq!(slot.alpha_b, 0);
            assert_eq!(slot.vec_b_now, [0.0, -8.0, 0.0]);
            assert_eq!(slot.rot_start, 0);
            assert_eq!(slot.full_display_angle, 0);
            assert_eq!(slot.flag1[0], 0);
            assert_eq!(slot.flag1[1], 0);
            assert_eq!(slot.flag1[2], 2);
            assert!(slot.process <= -30 && slot.process >= -39);
        }
    }

    #[test]
    fn later_frames_launch_nothing() {
        let mut effect = TeiHit2::new(5, false, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);

        for _ in 0..6 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 5);
    }

    #[test]
    fn backstap_launches_twenty_and_plays_the_wave_on_frame_twenty() {
        let mut effect = TeiHit2::new(20, true, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 5);
        runtime.parent_position = Some([1.0, 2.0, 3.0]);

        // Frame 0: twenty launches.
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 20);
        assert!(runtime.sounds.is_empty());

        // Advance to frame 20. The body never launches again.
        for _ in 0..19 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 20);
        assert_eq!(runtime.primitives.len(), 20);

        runtime.parent_position = Some([7.0, 8.0, 9.0]);
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.frame, 21);
        assert_eq!(runtime.sounds, vec!["effect\\rog_back stap.wav".to_owned()]);
        assert_eq!(runtime.position, [7.0, 8.0, 9.0]);
        assert_eq!(runtime.primitives.len(), 20);
    }
}
