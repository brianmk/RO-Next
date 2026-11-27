//! `CRagEffect` shield-throw bodies:
//!
//! * `CRagEffect::TexturedShieldThrow(const char* name)` -
//!   `RagEffect.cpp:5688-5729`. Dispatch: `EF_SHIELDBOOMERANG` (249,
//!   `RagEffect.cpp:24630-24632`), `EF_SHIELDBOOMERANG2` (494, `:21684-21686`)
//!   and `EF_SHIELDBOOMERANG4` (774, `:20446-20448`). Lifetime 499
//!   (`EffectTypes.cpp:258`, `:503`, `:783`).
//! * `CRagEffect::ShieldBoomerang3(const char* name)` -
//!   `RagEffect.cpp:12078-12177`. Dispatch: `EF_SHIELDBOOMERANG3` (520,
//!   `RagEffect.cpp:21687-21689`). Lifetime 499 (`EffectTypes.cpp:529`).
//! * `CRagEffect::TANJI()` - `RagEffect.cpp:14501-14542`. Dispatch: `EF_TANJI`
//!   (265, `RagEffect.cpp:24241-24243`). Lifetime 300 (`EffectTypes.cpp:274`).
//!   TANJI is a shield throw even though its name suggests the tanji skill: it
//!   spawns `PP_SHIELDBOOMERANG` (42) with `effect\blue_ivy.bmp`, exactly like
//!   `TexturedShieldThrow` spawns it with `effect\shield_boomerang.bmp`.
//!
//! All three arm one `PP_SHIELDBOOMERANG`/`PP_SHIELDBOOMERANG3` primitive and
//! let its motion (`primitive/shield_boomerang*.rs`) do the flying. The wave is
//! `PlayEffectWaveInline`, i.e. listener-relative at height zero; the port
//! records it through [`EffectRuntime::play_sound`], the same channel the other
//! inline-wave bodies use.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedShieldThrow` - `RagEffect.cpp:5688-5729`.
#[derive(Debug)]
pub struct TexturedShieldThrow {
    name: &'static str,
    lifetime: i32,
}

impl TexturedShieldThrow {
    pub fn new(name: &'static str, lifetime: i32) -> Self {
        Self { name, lifetime }
    }
}

impl PortedEffect for TexturedShieldThrow {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::TexturedShieldThrow` - `RagEffect.cpp:5688-5729`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = parent.position;` happens before the gate,
        // but `runtime.position` is already that anchor.
        if runtime.frame != 0 {
            return true;
        }

        // `PlayEffectWaveInline(*this, "effect\\cru_shield boomerang.wav");`
        runtime.play_sound("effect\\cru_shield boomerang.wav");

        let position = EffectRuntime::official_position(runtime.position);
        let target = EffectRuntime::official_position(runtime.target_offset);
        let delta_x = target[0] - position[0];
        let delta_z = target[2] - position[2];
        let max_height = (delta_z * delta_z + delta_x * delta_x).sqrt();
        let lifetime = runtime.lifetime;

        let index = runtime.launch(PrimitiveId::ShieldBoomerang, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        // `renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.name);
        state.size = 0.0;
        // `lastPrim->state.deltaPos = this->state.deltaPos;`
        state.delta_pos = target;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.process = 0;
        slot.max_height = max_height;
        slot.distance = 7.0;
        slot.height[8] = 0.0;
        slot.alpha_b = 255;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = position[1] - 8.0;
        slot.rot_start = 0;
        slot.height[1] = 2.0;
        slot.height[2] = (slot.height[1] / slot.max_height) * (target[1] - position[1]);
        slot.full_display_angle = 0;
        slot.flag1[0] = 0;
        slot.flag1[1] = 0;
        slot.flag1[2] = 0;

        state.slots[1].life = 1;
        state.slots[1].alpha_b = 0;
        state.slots[2].life = 1;
        state.slots[2].alpha_b = 0;
        state.slots[3].life = 1;
        state.slots[3].alpha_b = 0;

        true
    }
}

/// `CRagEffect::ShieldBoomerang3` - `RagEffect.cpp:12078-12177`.
///
/// One primitive per frame in `{0, 3, 6, 9, 12}`; the wave fires on frame 0
/// only.
#[derive(Debug)]
pub struct ShieldBoomerang3 {
    name: &'static str,
    lifetime: i32,
}

impl ShieldBoomerang3 {
    pub fn new(name: &'static str, lifetime: i32) -> Self {
        Self { name, lifetime }
    }
}

impl PortedEffect for ShieldBoomerang3 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::ShieldBoomerang3` - `RagEffect.cpp:12078-12177`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame != 0 && frame != 3 && frame != 6 && frame != 9 && frame != 12 {
            return true;
        }

        if frame == 0 {
            runtime.play_sound("effect\\cru_shield boomerang.wav");
        }

        let position = EffectRuntime::official_position(runtime.position);
        let lifetime = runtime.lifetime;

        // `lastPrim = LaunchEffectPrim(116, {0, 0, 0});`
        let index = runtime.launch(PrimitiveId::ShieldBoomerang3, [0.0, 0.0, 0.0]);
        let rot_start = runtime.rng.clamped_random(360) as i16;
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.name);
        state.size = 0.0;

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.process = 0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = position[1] - 8.0;
        slot.distance = 5.0;
        slot.height[8] = 0.0;
        slot.alpha_b = 25;
        slot.rot_start = rot_start;
        slot.height[1] = 2.5;
        // `vecBNow.x -= height[1] * 15.0f * g_cosTable[rotStart]` and the z
        // twin. The backwards offset is the launch start point.
        let angles = super::super::angles::Angles::shared();
        slot.vec_b_now[0] -= slot.height[1] * 15.0 * angles.raw_cosine(rot_start as usize);
        slot.vec_b_now[2] -= slot.height[1] * 15.0 * angles.raw_sine(rot_start as usize);
        slot.full_display_angle = 0;
        slot.flag1[0] = 0;
        slot.flag1[2] = 0;

        state.slots[1].life = 1;
        state.slots[1].alpha_b = 0;
        state.slots[2].life = 1;
        state.slots[2].alpha_b = 0;
        state.slots[3].life = 1;
        state.slots[3].alpha_b = 0;

        true
    }
}

/// `CRagEffect::TANJI` - `RagEffect.cpp:14501-14542`.
///
/// Spawns one `PP_SHIELDBOOMERANG` at frame 0 with `flag1[2] = 1`, the mode
/// that copies the trail every frame and flips to the return leg at its reach.
#[derive(Debug)]
pub struct Tanji {
    lifetime: i32,
}

impl Tanji {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Tanji {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::TANJI` - `RagEffect.cpp:14501-14542`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, parent->render.position);` runs
        // before the gate; `runtime.position` already holds that anchor.
        if runtime.frame != 0 {
            return true;
        }

        // `PlayEffectWaveInline(*this, "effect\\mon_탄지신통.wav");`
        runtime.play_sound("effect\\mon_탄지신통.wav");

        let position = EffectRuntime::official_position(runtime.position);
        let target = EffectRuntime::official_position(runtime.target_offset);
        let dx = target[0] - position[0];
        let dz = target[2] - position[2];
        let max_height = (dx * dx + dz * dz).sqrt();
        let lifetime = runtime.lifetime;
        let count = runtime.count;

        let index = runtime.launch(PrimitiveId::ShieldBoomerang, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some("effect\\blue_ivy.bmp");
        state.size = 0.0;
        state.delta_pos = target;

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.process = -15;
        slot.max_height = max_height;
        // `distance = (float)(int)this->state.count * 0.4f + 2.0f;`
        slot.distance = count as f32 * 0.4 + 2.0;
        slot.height[8] = 0.0;
        slot.alpha_b = 5;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = position[1] - 8.0;
        slot.rot_start = 0;
        slot.height[1] = 2.0;
        slot.height[2] = (slot.height[1] / slot.max_height) * (target[1] - position[1]);
        slot.full_display_angle = 0;
        slot.flag1[0] = 0;
        slot.flag1[1] = 0;
        slot.flag1[2] = 1;
        slot.height[7] = 0.0;

        state.slots[1].life = 1;
        state.slots[1].alpha_b = 0;
        state.slots[2].life = 1;
        state.slots[2].alpha_b = 0;
        state.slots[3].life = 1;
        state.slots[3].alpha_b = 0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::EffectRuntime;

    fn runtime(lifetime: i32, seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], seed);
        runtime.position = [3.0, 4.0, 5.0];
        runtime.target_offset = [3.0, 9.0, 15.0];
        runtime
    }

    /// Runs one body step at `frame` without the runtime's primitive pass, so
    /// the assertions see the exact values the body writes (a `step` would
    /// immediately advance the spawned primitive by one motion frame).
    fn at_frame(runtime: &mut EffectRuntime, frame: i32) {
        runtime.frame = frame;
    }

    #[test]
    fn textured_shield_throw_spawns_one_shield_primitive_on_frame_zero() {
        let mut effect = TexturedShieldThrow::new("effect\\shield_boomerang.bmp", 499);
        let mut runtime = runtime(499, 1);

        effect.on_process(&mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\cru_shield boomerang.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ShieldBoomerang);
        assert_eq!(state.count.id(), 42);
        assert_eq!(state.duration, 499);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.texture, Some("effect\\shield_boomerang.bmp"));
        assert_eq!(state.size, 0.0);
        // The target is converted to official (Y-down) space.
        assert_eq!(state.delta_pos, [3.0, -9.0, 15.0]);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 0);
        assert_eq!(slot.max_height, 10.0, "sqrt(10^2 + 0^2)");
        assert_eq!(slot.distance, 7.0);
        assert_eq!(slot.height[8], 0.0);
        assert_eq!(slot.alpha_b, 255);
        // Official Y-down: the port anchor's y 4 is -4, then the -8 lift.
        assert_eq!(slot.vec_b_now, [3.0, -12.0, 5.0], "y = official position.y - 8");
        assert_eq!(slot.rot_start, 0);
        assert_eq!(slot.height[1], 2.0);
        // `height[2] = height[1]/maxHeight * (target.y - position.y)`, both in
        // official (Y-down) space: -9 - (-4) = -5.
        assert_eq!(slot.height[2], 2.0 / 10.0 * -5.0);
        assert_eq!(slot.full_display_angle, 0);
        assert_eq!(slot.flag1[2], 0);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 0);
        }
    }

    #[test]
    fn textured_shield_throw_is_gated_to_frame_zero() {
        let effect = TexturedShieldThrow::new("effect\\shield_boomerang.bmp", 499);

        for frame in 0..3 {
            let mut runtime = runtime(499, 1);
            let mut effect = TexturedShieldThrow::new(effect.name, effect.lifetime);
            at_frame(&mut runtime, frame);

            effect.on_process(&mut runtime);

            assert_eq!(runtime.primitives.len(), usize::from(frame == 0), "frame {frame}");
            assert_eq!(runtime.sounds.len(), usize::from(frame == 0), "frame {frame}");
        }
    }

    #[test]
    fn shield_boomerang3_spawns_on_frames_0_3_6_9_and_12_only() {
        let effect = ShieldBoomerang3::new("effect\\shield_boomerang.bmp", 499);

        for frame in 0..15 {
            let mut runtime = runtime(499, 9);
            let mut effect = ShieldBoomerang3::new(effect.name, effect.lifetime);
            at_frame(&mut runtime, frame);

            effect.on_process(&mut runtime);

            let expected = matches!(frame, 0 | 3 | 6 | 9 | 12);
            assert_eq!(runtime.primitives.len(), usize::from(expected), "frame {frame}");
            assert_eq!(runtime.sounds.len(), usize::from(frame == 0), "frame {frame}");
        }
    }

    #[test]
    fn shield_boomerang3_arms_the_five_fragment_throw() {
        let mut effect = ShieldBoomerang3::new("effect\\shield_boomerang.bmp", 499);
        let mut runtime = runtime(499, 9);

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.sounds, vec!["effect\\cru_shield boomerang.wav".to_owned()]);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ShieldBoomerang3);
        assert_eq!(state.count.id(), 116);
        assert_eq!(state.duration, 499);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.texture, Some("effect\\shield_boomerang.bmp"));
        assert_eq!(state.size, 0.0);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 0);
        assert_eq!(slot.distance, 5.0);
        assert_eq!(slot.alpha_b, 25);
        assert_eq!(slot.height[8], 0.0);
        assert_eq!(slot.height[1], 2.5);
        assert!((0..360).contains(&(slot.rot_start as i32)));
        // The backwards launch offset is exactly 2.5 * 15 units.
        let offset_x = slot.vec_b_now[0] - 3.0;
        let offset_z = slot.vec_b_now[2] - 5.0;
        assert!((offset_x * offset_x + offset_z * offset_z - 37.5 * 37.5).abs() < 1e-3);
        // Official Y-down: the port anchor's y 4 is -4, then the -8 lift.
        assert_eq!(slot.vec_b_now[1], -12.0);
        assert_eq!(slot.full_display_angle, 0);
        assert_eq!(slot.flag1[0], 0);
        assert_eq!(slot.flag1[2], 0);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 0);
        }
    }

    #[test]
    fn tanji_spawns_the_shield_throw_with_its_mode_one_flags() {
        let mut effect = Tanji::new(300);
        let mut runtime = runtime(300, 1);
        runtime.count = 7;

        effect.on_process(&mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\mon_탄지신통.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ShieldBoomerang);
        assert_eq!(state.duration, 300);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.texture, Some("effect\\blue_ivy.bmp"));
        assert_eq!(state.size, 0.0);
        // The target is converted to official (Y-down) simulation space.
        assert_eq!(state.delta_pos, [3.0, -9.0, 15.0]);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, -15);
        assert_eq!(slot.max_height, 10.0);
        // 7 * 0.4 + 2.0.
        assert_eq!(slot.distance, 4.8);
        assert_eq!(slot.alpha_b, 5);
        // Official Y-down: the port anchor's y 4 is -4, then the -8 lift.
        assert_eq!(slot.vec_b_now, [3.0, -12.0, 5.0]);
        assert_eq!(slot.height[1], 2.0);
        // `height[2] = height[1]/maxHeight * (target.y - position.y)`, both in
        // official (Y-down) space: -9 - (-4) = -5.
        assert_eq!(slot.height[2], 2.0 / 10.0 * -5.0);
        assert_eq!(slot.full_display_angle, 0);
        assert_eq!(slot.flag1[2], 1);
        assert_eq!(slot.height[7], 0.0);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 0);
        }
    }

    #[test]
    fn tanji_is_gated_to_frame_zero() {
        for frame in 0..2 {
            let mut effect = Tanji::new(300);
            let mut runtime = runtime(300, 1);
            at_frame(&mut runtime, frame);

            effect.on_process(&mut runtime);

            assert_eq!(runtime.primitives.len(), usize::from(frame == 0), "frame {frame}");
            assert_eq!(runtime.sounds.len(), usize::from(frame == 0), "frame {frame}");
        }
    }
}
