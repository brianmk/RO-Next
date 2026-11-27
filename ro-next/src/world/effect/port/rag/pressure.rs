//! `EF_PRESSURE` (365) - `RagEffect.cpp:10342-10349`.
//!
//! The pressure rain: `CRagEffect::TexturedRain("effect\\cross_old.bmp", 0)`
//! (`RagEffect.cpp:5317-5378`, prim 87) plus
//! `CRagEffect::TexturedLevelRing("effect\\explosive_1_128.bmp", -28, 0)`
//! (`RagEffect.cpp:4096-4140`, prim 88). Frame 3 plays `effect\\프레셔.wav` and
//! re-anchors the effect on its parent; both helpers copy the parent anchor and
//! spawn only on frame 0.
//!
//! The two C++ functions keep their own Rust counterparts
//! ([`textured_rain`] and [`textured_level_ring`]) because other effect bodies
//! call them with different arguments; the `EF_PRESSURE` dispatch is the body
//! in [`Pressure`].
//!
//! Lifetimes: `EF_PRESSURE` 300 (`EffectTypes.cpp:374`). Renderers: the shared
//! `RenderFieldPanel` (prim 87) and `RenderSizedSquare` (prim 88).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `case EF_PRESSURE:` dispatch - `RagEffect.cpp:10342-10349`.
#[derive(Debug)]
pub struct Pressure {
    lifetime: i32,
}

impl Pressure {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// `CRagEffect::TexturedRain(const char* name, char kind)` -
/// `RagEffect.cpp:5317-5378`.
///
/// RNG order: the `kind == 4` vertical offset (`rand() % 56`) is drawn
/// **before** `fullDisplayAngle` (`ClampedRandom(360)`); every other kind draws
/// only the angle.
fn textured_rain(runtime: &mut EffectRuntime, name: &'static str, kind: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(87, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::BubbleSwirl, [0.0, 0.0, 0.0]);

    // RNG: `vecBPre.y` (only kind 4) then `fullDisplayAngle`.
    let vec_b_pre_y = if kind == 0 {
        -115.0
    } else if kind == 4 {
        -115.0 - runtime.rng.clamped_random(56) as f32
    } else {
        -80.0
    };
    let full_display_angle = runtime.rng.clamped_random(360) as i16;

    let distance = if kind == 0 {
        12.0
    } else if kind == 4 {
        14.0
    } else {
        8.0
    };

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = 0;
    slot.distance = distance;
    slot.alpha_b = 0;
    slot.vec_b_pre[0] = 0.0;
    slot.vec_b_pre[2] = 0.0;
    slot.vec_b_pre[1] = vec_b_pre_y;
    slot.rot_start = 0;
    slot.height[1] = 3.0;
    slot.full_display_angle = full_display_angle;
    slot.flag1[0] = 0;
    slot.flag1[2] = 7;

    if kind == 1 {
        slot.flag1[2] = 9;
        slot.flag1[3] = 1;
    } else if kind == 2 {
        slot.flag1[2] = 9;
        slot.flag1[3] = 3;
    } else if kind == 3 {
        slot.flag1[2] = 9;
        slot.flag1[3] = 4;
    }

    slot.flag1[4] = kind;

    state.slots[1].life = 1;
    state.slots[1].alpha_b = 0;
    state.slots[2].life = 1;
    state.slots[2].alpha_b = 0;
    state.slots[3].life = 1;
    state.slots[3].alpha_b = 0;
}

/// `CRagEffect::TexturedLevelRing(const char* name, int process, char level)` -
/// `RagEffect.cpp:4096-4140`.
///
/// Only `level == 3` draws an RNG value (`rand() % 6` for the ring lift); the
/// other levels only set `size` (1/2/3/4/10) or leave the primitive's
/// constructor `size = 1` alone.
fn textured_level_ring(runtime: &mut EffectRuntime, name: &'static str, process: i32, level: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(88, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::GrowFollow, [0.0, 0.0, 0.0]);

    // RNG: the level-3 lift only.
    let ring_lift = if level == 3 { runtime.rng.clamped_random(6) as f32 } else { 0.0 };

    let state = runtime.primitive_mut(index);

    state.render_flag = 0;
    state.render_flag |= 1;
    state.render_flag |= 1 | 4;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = Some(name);

    if level == 1 {
        state.size = 101.0;
    } else if level == 2 {
        state.size = 102.0;
    } else if level == 3 {
        state.size = 111.0;
    } else if level == 4 {
        state.size = 113.0;
    } else if level == 10 {
        state.size = 114.0;
    }

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = process;

    if level == 3 {
        slot.distance = 13.0;
        slot.height[0] = -5.0 - ring_lift;
    } else {
        slot.distance = 15.0;
        slot.height[0] = -5.0;
    }

    slot.rise_angle = 0;
    slot.alpha_b = 0;
    state.slots[1].life = 0;
    state.slots[2].life = 0;
    state.slots[3].life = 0;
}

impl PortedEffect for Pressure {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 3 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\프레셔.wav");
        }

        textured_rain(runtime, "effect\\cross_old.bmp", 0);
        textured_level_ring(runtime, "effect\\explosive_1_128.bmp", -28, 0);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn pressure_is_registered_with_lifetime_300() {
        assert_eq!(Pressure::new(300).lifetime(), 300);
    }

    #[test]
    fn frame_zero_spawns_the_rain_and_the_ring() {
        let mut effect = Pressure::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);

        // The only RNG draw of the EF_PRESSURE arguments is
        // `TexturedRain`'s `fullDisplayAngle`.
        let mut predictor = EffectRng::new(7);
        let expected_angle = predictor.clamped_random(360) as i16;

        // Call the body directly so the primitive pass does not advance the
        // freshly spawned slots.
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 2);

        let rain = &runtime.primitives[0].state;
        assert_eq!(rain.count, PrimitiveId::BubbleSwirl);
        assert_eq!(rain.texture, Some("effect\\cross_old.bmp"));
        assert_eq!(rain.render_flag, 5);
        assert_eq!(rain.size, 0.0);
        assert_eq!(rain.slots[0].life, 1);
        assert_eq!(rain.slots[0].distance, 12.0);
        assert_eq!(rain.slots[0].vec_b_pre[1], -115.0);
        assert_eq!(rain.slots[0].height[1], 3.0);
        assert_eq!(rain.slots[0].full_display_angle, expected_angle);
        assert_eq!(rain.slots[0].flag1[2], 7);
        assert_eq!(rain.slots[0].flag1[4], 0);
        assert_eq!(rain.slots[1].life, 1);
        assert_eq!(rain.slots[3].life, 1);

        let ring = &runtime.primitives[1].state;
        assert_eq!(ring.count, PrimitiveId::GrowFollow);
        assert_eq!(ring.texture, Some("effect\\explosive_1_128.bmp"));
        assert_eq!(ring.render_flag, 5);
        assert_eq!(ring.size, 1.0, "level 0 keeps the constructor size");
        assert_eq!(ring.slots[0].process, -28);
        assert_eq!(ring.slots[0].distance, 15.0);
        assert_eq!(ring.slots[0].height[0], -5.0);
        assert_eq!(ring.slots[0].rise_angle, 0);
        assert_eq!(ring.slots[0].alpha_b, 0);
        assert_eq!(ring.slots[1].life, 0);
    }

    #[test]
    fn later_frames_do_not_spawn_again() {
        let mut effect = Pressure::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 3);

        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 4);
    }

    #[test]
    fn frame_three_plays_the_pressure_sound() {
        let mut effect = Pressure::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 3);
        runtime.frame = 3;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\프레셔.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 0, "only frame zero spawns");
    }

    #[test]
    fn a_level_three_ring_draws_the_six_value_lift() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 5);
        let mut predictor = EffectRng::new(5);
        let expected = -5.0 - predictor.clamped_random(6) as f32;

        textured_level_ring(&mut runtime, "effect\\explosive_1_128.bmp", -10, 3);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.size, 111.0);
        assert_eq!(state.slots[0].distance, 13.0);
        assert_eq!(state.slots[0].height[0], expected);
    }

    #[test]
    fn the_rain_kind_four_draws_its_offset_before_the_angle() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 9);
        let mut predictor = EffectRng::new(9);
        let expected_y = -115.0 - predictor.clamped_random(56) as f32;
        let expected_angle = predictor.clamped_random(360) as i16;

        textured_rain(&mut runtime, "유저인터페이스\\item\\스피어.bmp", 4);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].distance, 14.0);
        assert_eq!(state.slots[0].vec_b_pre[1], expected_y);
        assert_eq!(state.slots[0].full_display_angle, expected_angle);
    }

    #[test]
    fn kind_three_selects_the_nine_renderer_with_the_fourth_texture_row() {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);

        textured_rain(&mut runtime, "effect\\cross_old.bmp", 3);

        let slot = runtime.primitives[0].state.slots[0];
        assert_eq!(slot.flag1[2], 9);
        assert_eq!(slot.flag1[3], 4);
        assert_eq!(slot.flag1[4], 3);
    }
}
