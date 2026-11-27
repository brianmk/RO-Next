//! `EF_MAGNUM2` (339) - `RagEffect.cpp:10249-10250`.
//!
//! The yellow Magnum Break shockwave: the dispatch calls
//! `CRagEffect::Magnum2("effect\\ring_yellow.tga")`
//! (`RagEffect.cpp:5197-5242`). The body resets `state.flag` on frame 0, plays
//! `permeter_attack.wav` on frame 20 and `effect\\EF_MagnumBreak.wav` on frame
//! 30, and from frame 25 to 55 (every third frame) launches one **prim 73**
//! `PP_MAGNUM2` wave while `state.flag < 11`.
//!
//! The flag is incremented *before* the gate, so the eleventh step (frame 55)
//! consumes the counter without spawning: the effect emits ten waves, each with
//! two live slots whose radius, height and rise angle follow `time = frame -
//! 25`.
//!
//! Lifetimes: `EF_MAGNUM2` 100 (`EffectTypes.cpp:348`). Renderer
//! `RenderMagnum2` (`EffectPrimitiveRendering.cpp:2121`); motion the bare
//! `case PP_MAGNUM2:` (`EffectPrimitiveMotion.cpp:8377-8420`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `case EF_MAGNUM2:` dispatch - `RagEffect.cpp:10249-10250`.
#[derive(Debug)]
pub struct Magnum2 {
    texture: &'static str,
    lifetime: i32,
}

impl Magnum2 {
    pub const fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

/// `CRagEffect::Magnum2(const char* name)` - `RagEffect.cpp:5197-5242`.
fn magnum2(runtime: &mut EffectRuntime, name: &'static str) {
    runtime.copy_parent_position();

    if runtime.frame == 0 {
        runtime.flag = 0;
    }

    if runtime.frame == 20 {
        runtime.play_sound("permeter_attack.wav");
    } else if runtime.frame == 30 {
        runtime.play_sound("effect\\EF_MagnumBreak.wav");
    }

    if !(25..=55).contains(&runtime.frame) || runtime.frame % 3 != 0 {
        return;
    }

    runtime.flag += 1;

    if runtime.flag >= 11 {
        return;
    }

    let time = runtime.frame - 25;
    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(73, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::Magnum2, [0.0, 0.0, 0.0]);

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = Some(name);
    state.size = 5.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for slot_index in 0..2usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;
        slot.full_display_angle = 360;
        slot.max_height = 20.0 - time as f32 * 0.7;
        slot.rot_start = (slot_index as i32 * 9 + time * 6) as i16;
        slot.alpha_b = 0;
        slot.distance = time as f32 * 0.2 + 5.0;
        slot.rise_angle = (70 - slot_index as i32 * 8 - time * 2) as i16;
        slot.alpha_t = slot_index as i16;
    }
}

impl PortedEffect for Magnum2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        magnum2(runtime, self.texture);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    fn effect() -> Magnum2 {
        Magnum2::new("effect\\ring_yellow.tga", 100)
    }

    #[test]
    fn magnum_two_is_registered_with_lifetime_100() {
        assert_eq!(effect().lifetime(), 100);
    }

    #[test]
    fn frame_zero_resets_the_wave_counter() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.flag = 7;

        step(&mut body, &mut runtime);

        assert_eq!(runtime.flag, 0);
        assert_eq!(runtime.primitives.len(), 0);
    }

    #[test]
    fn frame_twenty_seven_launches_the_first_wave() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.frame = 27; // first frame in 25..=55 divisible by three

        body.on_process(&mut runtime);

        assert_eq!(runtime.flag, 1);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Magnum2);
        assert_eq!(state.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(state.size, 5.0);
        assert_eq!(state.render_flag, 5);

        // time = 2.
        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.full_display_angle, 360);
        assert!((slot.max_height - 18.6).abs() < 1e-6);
        assert_eq!(slot.rot_start, 12);
        assert!((slot.distance - 5.4).abs() < 1e-6);
        assert_eq!(slot.rise_angle, 66);
        assert_eq!(slot.alpha_t, 0);

        let second = &state.slots[1];
        assert_eq!(second.life, 1);
        assert_eq!(second.rot_start, 21);
        assert_eq!(second.rise_angle, 58);
        assert_eq!(second.alpha_t, 1);
        assert_eq!(state.slots[2].life, 0);
    }

    #[test]
    fn frame_twenty_five_is_not_a_third_frame() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.frame = 25;

        body.on_process(&mut runtime);

        assert_eq!(runtime.flag, 0);
        assert_eq!(runtime.primitives.len(), 0);
    }

    #[test]
    fn the_wave_fields_follow_the_elapsed_time() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.frame = 36; // time = 11

        body.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert!((state.slots[0].max_height - (20.0 - 11.0 * 0.7)).abs() < 1e-6);
        assert_eq!(state.slots[0].rot_start, 66);
        assert!((state.slots[0].distance - (11.0 * 0.2 + 5.0)).abs() < 1e-6);
        assert_eq!(state.slots[0].rise_angle, 48);
        assert_eq!(state.slots[1].rot_start, 75);
        assert_eq!(state.slots[1].rise_angle, 40);
    }

    #[test]
    fn the_sounds_play_on_frames_twenty_and_thirty() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.frame = 20;

        step(&mut body, &mut runtime);
        assert_eq!(runtime.sounds, vec!["permeter_attack.wav".to_string()]);

        runtime.frame = 30;
        step(&mut body, &mut runtime);
        assert_eq!(runtime.sounds[1], "effect\\EF_MagnumBreak.wav");
    }

    #[test]
    fn only_every_third_frame_launches_and_the_eleventh_is_gated() {
        let mut body = effect();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);

        for frame in 0..100 {
            runtime.frame = frame;
            step(&mut body, &mut runtime);
        }

        // Frames 27, 30, ..., 54 -> ten increments, all below the `< 11` gate.
        assert_eq!(runtime.flag, 10);
        assert_eq!(runtime.primitives.len(), 10);
    }
}
