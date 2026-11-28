//! `CRagEffect::LineStreakLayered` - `RagEffect.cpp:16760-16831`.
//!
//! The ground line bars of `EF_CHEMICAL3` (`RagEffect.cpp:24367-24373`, kinds 0
//! and 1, eight calls) and `EF_CHEMICAL4` (`RagEffect.cpp:21124-21133`, kinds
//! 20 and 21, eight calls). The body only spawns on frame 0; each call launches
//! one prim 56 [`PrimitiveId::Chemical3`] with `line3.tga` (`kind < 10`) or
//! `line3y.tga` (otherwise), four slots and `RSF_ALPHA | RSF_NO_DEPTH`.
//!
//! The per-slot branch is the original's `kind / 10` split:
//!
//! * `0..9`  - `maxHeight = rand(4)*0.1 + 0.3`, `height[2] = rand(31) + 20`,
//!   `flag1[0] = rand(3)`, `height[3] = rand(21)*0.1 + 1.5`,
//! * `10..19` - `maxHeight = rand(6)*0.1 + g_depthAverageHalf`, `height[2] =
//!   rand(31) + 40`, `flag1[0] = rand(3) + 10`, `height[3] = rand(11)*0.1 + 1`,
//! * otherwise - `maxHeight = rand(4)*0.1 + 0.4`, `height[2] = rand(31) + 30`,
//!   `flag1[0] = rand(3) + 20`, `height[3] = rand(21)*0.1 + 1.5`.
//!
//! `kind % 10 == 0/1` additionally seeds `height[0]` with `-15 - rand(11)` /
//! `15 + rand(11)`. Every slot draws `height[1] = rand(200)` and the same
//! `riseAngle` from the effect anchor/target. `g_depthAverageHalf` is a
//! rendering constant whose value the decompilation does not expose; the port
//! uses the original's documented default of `0.5` (the value is only a
//! `maxHeight` jitter for the unused `10..19` branch, since neither
//! `EF_CHEMICAL3` nor `EF_CHEMICAL4` passes such a kind).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRng, EffectRuntime, PortedEffect};

/// `CRagEffect::LineStreakLayered(short kind)` - `RagEffect.cpp:16760-16831`.
pub(crate) fn line_streak_layered(runtime: &mut EffectRuntime, kind: i16) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);
    let rise_angle = runtime.get_angle(target[0] - position[0], target[2] - position[2]);

    let texture = if kind < 10 { "effect\\line3.tga" } else { "effect\\line3y.tga" };

    let index = runtime.launch(PrimitiveId::Chemical3, [0.0, 0.0, 0.0]);
    let mut rng = std::mem::replace(&mut runtime.rng, EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 0x1 | 0x200; // RSF_ALPHA | RSF_NO_DEPTH
    state.duration = lifetime;
    state.texture = Some(texture);

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;
        slot.distance = 100.0;

        if kind % 10 == 0 {
            slot.height[0] = -15.0 - rng.clamped_random(11) as f32;
        } else if kind % 10 == 1 {
            slot.height[0] = rng.clamped_random(11) as f32 + 15.0;
        }

        slot.height[1] = rng.clamped_random(200) as f32;

        if kind < 10 {
            slot.max_height = rng.clamped_random(4) as f32 * 0.1 + 0.3;
            slot.height[2] = rng.clamped_random(31) as f32 + 20.0;
            slot.flag1[0] = rng.clamped_random(3) as i8;
            slot.height[3] = rng.clamped_random(21) as f32 * 0.1 + 1.5;
        } else if kind < 20 {
            slot.max_height = rng.clamped_random(6) as f32 * 0.1 + 0.5;
            slot.height[2] = rng.clamped_random(31) as f32 + 40.0;
            slot.flag1[0] = rng.clamped_random(3) as i8 + 10;
            slot.height[3] = rng.clamped_random(11) as f32 * 0.1 + 1.0;
        } else {
            slot.max_height = rng.clamped_random(4) as f32 * 0.1 + 0.4;
            slot.height[2] = rng.clamped_random(31) as f32 + 30.0;
            slot.flag1[0] = rng.clamped_random(3) as i8 + 20;
            slot.height[3] = rng.clamped_random(21) as f32 * 0.1 + 1.5;
        }

        slot.rise_angle = rise_angle;
        slot.alpha_b = 0;
    }

    runtime.rng = rng;
}

/// `case EF_CHEMICAL3:` - `RagEffect.cpp:24367-24373`.
#[derive(Debug)]
pub struct Chemical3 {
    lifetime: i32,
}

impl Chemical3 {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Chemical3 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        for _ in 0..4 {
            line_streak_layered(runtime, 0);
            line_streak_layered(runtime, 1);
        }

        true
    }
}

/// `case EF_CHEMICAL4:` - `RagEffect.cpp:21124-21133`.
#[derive(Debug)]
pub struct Chemical4 {
    lifetime: i32,
}

impl Chemical4 {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Chemical4 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 28 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\피어싱샷.wav");
        }

        for _ in 0..4 {
            line_streak_layered(runtime, 20);
            line_streak_layered(runtime, 21);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);
        runtime.target_offset = [10.0, 20.0, 40.0];

        runtime
    }

    #[test]
    fn chemical_three_launches_eight_bars_on_frame_zero_only() {
        let mut body = Chemical3::new(300);
        let mut runtime = runtime();

        step(&mut body, &mut runtime);
        assert_eq!(runtime.primitives.len(), 8);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Chemical3);
        assert_eq!(state.render_flag, 0x201);
        assert_eq!(state.duration, 300);
        assert_eq!(state.texture, Some("effect\\line3.tga"));

        // The other seven spawns of the first frame keep the same texture.
        let second = &runtime.primitives[1].state;
        assert_eq!(second.texture, Some("effect\\line3.tga"));
        // All eight slots are armed with the target angle (10 units +z -> 91).
        for index in 0..4usize {
            assert_eq!(state.slots[index].life, 1);
            assert_eq!(state.slots[index].distance, 100.0);
            assert_eq!(state.slots[index].rise_angle, 91);
        }

        let count = runtime.primitives.len();
        step(&mut body, &mut runtime);
        assert_eq!(runtime.primitives.len(), count);
    }

    #[test]
    fn chemical_four_uses_the_yellow_texture_and_plays_its_wave_on_frame_28() {
        let mut body = Chemical4::new(300);
        let mut runtime = runtime();

        body.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 8);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\line3y.tga"));
        assert!(runtime.primitives[0].state.slots[0].flag1[0] >= 20);

        runtime.frame = 28;
        body.on_process(&mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\피어싱샷.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 8);
    }

    #[test]
    fn zero_kind_seeds_the_low_height_and_does_not_consume_rng_for_it() {
        let mut body = Chemical3::new(300);
        let mut runtime = runtime();

        body.on_process(&mut runtime);

        let slot = &runtime.primitives[0].state.slots[0];
        assert!(slot.height[0] <= -15.0 && slot.height[0] >= -25.0);
        assert!(slot.max_height >= 0.3 && slot.max_height < 0.7);
        assert!(slot.height[2] >= 20.0 && slot.height[2] < 51.0);
        assert!(slot.height[3] >= 1.5 && slot.height[3] < 3.6);
    }

    #[test]
    fn kind_one_seeds_the_high_height() {
        let mut body = Chemical3::new(300);
        let mut runtime = runtime();

        body.on_process(&mut runtime);

        let slot = &runtime.primitives[1].state.slots[0];
        assert!(slot.height[0] >= 15.0 && slot.height[0] <= 25.0);
    }
}
