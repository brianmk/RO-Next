//! `CRagEffect::TexturedScatter` and `CRagEffect::TexturedScatterLow` -
//! `RagEffect.cpp:3180-3240`.
//!
//! Both bodies arm prim 44 (`PrimExpandPulse`,
//! [`super::super::primitive::expand_pulse`]) with four delayed scatter slots;
//! `TexturedScatter` is the `EF_TEIHIT1`/`1X`/`1T` family and
//! `TexturedScatterLow` the lower `EF_TEIHIT3` variant:
//!
//! | body | `process` delay | `height[0]` (radial step) | `height[1]` | `flag1[1]` |
//! | --- | --- | --- | --- | --- |
//! | `TexturedScatter` | `-10 - ClampedRandom(25)` | 3.0 | 0.7 | 24 |
//! | `TexturedScatterLow` | `-20 - ClampedRandom(10)` | 2.0 | 1.0 | 16 |
//!
//! Each call launches its own primitive, so the dispatch loops in
//! `RagEffect.cpp` (24 calls for `EF_TEIHIT1X`, 20 for `EF_TEIHIT3`) create 24
//! and 20 independent primitives per frame-0 step.
//!
//! Quirks kept:
//!
//! * the anchor is dropped by 9 units **after** the primitive is launched, so
//!   the primitive's own `render.position` keeps the pre-drop master anchor
//!   while the effect's `render.position` moves down (`RagEffect.cpp:3191`).
//!   Because the dispatch loops call the body 24/20 times, the drop is
//!   cumulative: `EF_TEIHIT1X` ends the frame 216 units below the caster and
//!   its 24 primitives sit at `y = 0, -9, ..., -207`; `EF_TEIHIT3` ends 180
//!   below with `y = 0, -9, ..., -171`. The port keeps the staircase.
//! * `TexturedScatterLow`'s `stateCnt != 0` gate is the very first statement,
//!   *before* `CopyVector(render.position, parent.position)`; `TexturedScatter`
//!   copies first and gates second. On later frames the low body is a no-op.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// One `TexturedScatter*` dispatch: `count` calls to the body on frame 0.
#[derive(Debug)]
pub struct TexturedScatter {
    texture: &'static str,
    flag: i8,
    count: usize,
    low: bool,
    lifetime: i32,
}

impl TexturedScatter {
    pub const fn new(texture: &'static str, flag: i8, count: usize, low: bool, lifetime: i32) -> Self {
        Self {
            texture,
            flag,
            count,
            low,
            lifetime,
        }
    }
}

/// `CRagEffect::TexturedScatter(const char* name, char flag)` -
/// `RagEffect.cpp:3180-3209`.
fn textured_scatter(runtime: &mut EffectRuntime, name: &'static str, flag: i8) {
    // `this->render.position = parent.position;` comes before the gate.
    runtime.copy_parent_position();

    // `if (this->state.stateCnt != 0) return;`
    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(44, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::ExpandPulse, [0.0, 0.0, 0.0]);

    // `this->render.position.y -= 9.0f;` - the effect anchor, after the
    // primitive inherited the old master position at launch.
    runtime.position[1] -= 9.0;

    // RNG order per slot: process (25), riseAngle (360), fullDisplayAngle
    // (360).
    let mut processes = [0i32; 4];
    let mut rise_angles = [0i16; 4];
    let mut display_angles = [0i16; 4];

    for slot_index in 0..4usize {
        processes[slot_index] = -10 - runtime.rng.clamped_random(25);
        rise_angles[slot_index] = runtime.rng.clamped_random(360) as i16;
        display_angles[slot_index] = runtime.rng.clamped_random(360) as i16;
    }

    let state = runtime.primitive_mut(index);

    // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = processes[slot_index];
        slot.distance = -10.0;
        slot.height[0] = 3.0;
        slot.rise_angle = rise_angles[slot_index];
        slot.full_display_angle = display_angles[slot_index];
        slot.height[1] = 0.7;
        slot.alpha_b = 0;
        slot.flag1[0] = flag;
        slot.flag1[1] = 24;
    }
}

/// `CRagEffect::TexturedScatterLow(const char* name, char flag)` -
/// `RagEffect.cpp:3211-3240`.
fn textured_scatter_low(runtime: &mut EffectRuntime, name: &'static str, flag: i8) {
    // `if (this->state.stateCnt != 0) return;` is the first statement here,
    // before the parent copy.
    if runtime.frame != 0 {
        return;
    }

    runtime.copy_parent_position();

    let lifetime = runtime.lifetime;

    let index = runtime.launch(PrimitiveId::ExpandPulse, [0.0, 0.0, 0.0]);

    runtime.position[1] -= 9.0;

    // RNG order per slot: process (10), riseAngle (360), fullDisplayAngle
    // (360).
    let mut processes = [0i32; 4];
    let mut rise_angles = [0i16; 4];
    let mut display_angles = [0i16; 4];

    for slot_index in 0..4usize {
        processes[slot_index] = -20 - runtime.rng.clamped_random(10);
        rise_angles[slot_index] = runtime.rng.clamped_random(360) as i16;
        display_angles[slot_index] = runtime.rng.clamped_random(360) as i16;
    }

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = processes[slot_index];
        slot.distance = -10.0;
        slot.height[0] = 2.0;
        slot.rise_angle = rise_angles[slot_index];
        slot.full_display_angle = display_angles[slot_index];
        slot.height[1] = 1.0;
        slot.alpha_b = 0;
        slot.flag1[0] = flag;
        slot.flag1[1] = 16;
    }
}

impl PortedEffect for TexturedScatter {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `case EF_TEIHIT1X:` / `case EF_TEIHIT3:` - `RagEffect.cpp:24228-24231`
    /// and `:24348-24351`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        for _ in 0..self.count {
            if self.low {
                textured_scatter_low(runtime, self.texture, self.flag);
            } else {
                textured_scatter(runtime, self.texture, self.flag);
            }
        }

        true
    }
}

/// `case EF_TEIHIT1T:` - `RagEffect.cpp:19571-19583`.
///
/// `EF_TEIHIT1T` (889, lifetime 300 at `EffectTypes.cpp:898`) is the only
/// `TexturedScatter` dispatch with an extra frame-5 step:
///
/// ```cpp
/// if (this->state.stateCnt == 5) {
///   this->render.position = this->state.parent->render.position;
///   PlayEffectWave(*this, "effect\\mon_맹룡과강.wav");
/// }
/// {
///   const int count = g_serviceOptions.Get(TT_MIN_EFFECT_ON_OFF) ? 6 : 20;
///   for (int index = 0; index < count; ++index) {
///     this->TexturedScatter("effect\\alpha_center.tga", 2);
///   }
/// }
/// ```
///
/// The scatter loop runs on **every** frame: `TexturedScatter` copies the
/// parent first and then returns on any non-zero frame, so the only primitives
/// come from frame 0, while the frame-5 wave/position step is the real extra
/// behaviour. The port keeps both (the wave is a `PlayEffectWave` on the
/// effect's own position, i.e. [`EffectRuntime::play_sound`]).
///
/// Quirk kept: the wave call is `PlayEffectWave`, not the inline helper, so it
/// has no listener-relative offset here (the runtime records the name only).
#[derive(Debug)]
pub struct TeiHit1T {
    count: usize,
    lifetime: i32,
}

impl TeiHit1T {
    /// `TT_MIN_EFFECT_ON_OFF` defaults to off, so the official count is 20
    /// (`ServiceOptions.cpp:499-501`).
    pub const fn new(count: usize, lifetime: i32) -> Self {
        Self { count, lifetime }
    }
}

impl PortedEffect for TeiHit1T {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 5 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\mon_맹룡과강.wav");
        }

        for _ in 0..self.count {
            textured_scatter(runtime, "effect\\alpha_center.tga", 2);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn teihit1x_spawns_twenty_four_scatter_primitives_on_frame_zero() {
        let mut effect = TexturedScatter::new("effect\\lens1.tga", 1, 24, false, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 99);

        // Predict the first two slots with the same RNG the body consumes.
        let mut expected = EffectRng::new(99);
        let first_process = -10 - expected.clamped_random(25);
        let first_rise = expected.clamped_random(360) as i16;
        let first_display = expected.clamped_random(360) as i16;
        let second_process = -10 - expected.clamped_random(25);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 24);
        // Each call drops the effect anchor by 9 after launching, so 24 calls
        // end 216 units below and the primitives form a staircase.
        assert_eq!(runtime.position, [0.0, -216.0, 0.0]);

        for (primitive_index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::ExpandPulse);
            assert_eq!(state.texture, Some("effect\\lens1.tga"));
            assert_eq!(state.render_flag, 5);
            assert_eq!(state.duration, 300);
            assert_eq!(state.position, [0.0, -9.0 * primitive_index as f32, 0.0]);

            for slot in &state.slots {
                assert_eq!(slot.life, 1);
                assert_eq!(slot.distance, -10.0);
                assert_eq!(slot.height[0], 3.0);
                assert_eq!(slot.height[1], 0.7);
                assert_eq!(slot.alpha_b, 0);
                assert_eq!(slot.flag1[0], 1);
                assert_eq!(slot.flag1[1], 24);
                assert!(slot.process >= -35 && slot.process <= -9, "process {}", slot.process);
                assert!((0..360).contains(&(slot.rise_angle as i32)));
                assert!((0..360).contains(&(slot.full_display_angle as i32)));
            }
        }

        // The primitive pass runs in the same step, so the +1 is on top of the
        // seeded delay.
        let first = &runtime.primitives[0].state.slots[0];
        assert_eq!(first.process, first_process + 1);
        assert_eq!(first.rise_angle, first_rise);
        assert_eq!(first.full_display_angle, first_display);
        assert_eq!(runtime.primitives[0].state.slots[1].process, second_process + 1);
    }

    #[test]
    fn later_frames_do_not_spawn_more_scatter_primitives() {
        let mut effect = TexturedScatter::new("effect\\lens1.tga", 1, 24, false, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 99);

        step(&mut effect, &mut runtime);
        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 24);
    }

    #[test]
    fn teihit3_uses_the_low_variant() {
        let mut effect = TexturedScatter::new("effect\\lens2.tga", 2, 20, true, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);

        let mut expected = EffectRng::new(7);
        let first_process = -20 - expected.clamped_random(10);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 20);

        assert_eq!(runtime.position, [0.0, -180.0, 0.0]);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture, Some("effect\\lens2.tga"));
        assert_eq!(state.slots[0].process, first_process + 1);
        assert_eq!(state.slots[0].height[0], 2.0);
        assert_eq!(state.slots[0].height[1], 1.0);
        assert_eq!(state.slots[0].flag1[0], 2);
        assert_eq!(state.slots[0].flag1[1], 16);
        assert!(state.slots[0].process >= -30 && state.slots[0].process <= -20);
    }

    #[test]
    fn the_low_variant_does_not_copy_the_parent_after_frame_zero() {
        let mut effect = TexturedScatter::new("effect\\lens2.tga", 2, 1, true, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);
        runtime.position = [1.0, 2.0, 3.0];

        step(&mut effect, &mut runtime);

        runtime.parent_position = Some([50.0, 60.0, 70.0]);
        runtime.frame = 5;

        // `TexturedScatterLow` returns before the copy, so the anchor stays.
        effect.on_process(&mut runtime);

        assert_eq!(runtime.position, [1.0, -7.0, 3.0]);

        // `TexturedScatter` copies first, so it does follow.
        let mut effect = TexturedScatter::new("effect\\lens1.tga", 1, 1, false, 300);
        effect.on_process(&mut runtime);

        assert_eq!(runtime.position, [50.0, 60.0, 70.0]);
    }

    #[test]
    fn teihit1t_plays_the_frame_five_wave_and_scatters_on_frame_zero() {
        let mut effect = TeiHit1T::new(20, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 5);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);

        // Frame 0: 20 alpha_center scatters, flag 2. Each body call copies the
        // parent first, so with a live parent the otherwise-cumulative 9-unit
        // drop collapses to a single drop.
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 20);
        assert_eq!(runtime.position, [10.0, 11.0, 30.0]);
        assert!(
            runtime
                .primitives
                .iter()
                .all(|primitive| primitive.state.position == [10.0, 20.0, 30.0])
        );
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\alpha_center.tga"));
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[0], 2);

        // Frames 1..4: the loop still runs but every scatter gates on frame 0;
        // the parent copy still follows the parent every frame.
        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 20);
        assert!(runtime.sounds.is_empty());

        // Frame 5: wave + parent copy, no new primitives.
        runtime.parent_position = Some([50.0, 60.0, 70.0]);
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 20);
        assert_eq!(runtime.sounds, vec!["effect\\mon_맹룡과강.wav".to_owned()]);
        assert_eq!(runtime.position, [50.0, 60.0, 70.0]);

        // Later frames keep following the parent through the scatter copy.
        runtime.parent_position = Some([1.0, 2.0, 3.0]);
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.position, [1.0, 2.0, 3.0]);
        assert_eq!(runtime.primitives.len(), 20);
    }
}
