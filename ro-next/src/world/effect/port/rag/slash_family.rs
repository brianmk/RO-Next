//! The `EF_STIN` slash family - `RagEffect.cpp:3980-4032` (fan),
//! `:4687-4740` (wave), `:4902-4952` (cross), `:5032-5095` (level).
//!
//! | id | dispatch | body |
//! | --- | --- | --- |
//! | `EF_STIN` 547 | `RagEffect.cpp:21810-21814` | `TexturedSlashFan` |
//! | `EF_STIN2` 553 | `:21817-21828` | fan + two `TexturedSlashLevel` calls (kinds 0 and 1) |
//! | `EF_STIN4` 621 | `:21150-21156` | `state.count` × `TexturedSlashWave` (kind `index % 2`) |
//! | `EF_STIN5` 624 | `:21117-21123` | `TexturedSlashCross` every frame while `stateCnt < 5` |
//!
//! All four helpers only spawn on frame 0 and arm prims 119/122/125/126 with
//! `RSF_ALPHA | RSF_CUSTOM_BLEND`. Lifetimes: `EF_STIN`/`2`/`4`/`5` are 200
//! (`EffectTypes.cpp:556`, `:562`, `:630`, `:633`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRng, EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedSlashFan(const char* name)` -
/// `RagEffect.cpp:3980-4032`.
fn textured_slash_fan(runtime: &mut EffectRuntime, name: &'static str) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);
    let index = runtime.launch(PrimitiveId::SlashFan, [0.0, 0.0, 0.0]);
    let mut rng = std::mem::replace(&mut runtime.rng, EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;
    state.delta_pos = target;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = slot_index as i32 - 3;
        slot.distance = 20.0;
        slot.alpha_b = 0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = rng.clamped_random(5) as f32 + (position[1] - 10.0);
        slot.rot_start = 0;
        slot.rise_angle = slot_index as i16;
        slot.height[1] = 0.8;
        slot.flag1[0] = 0;
        slot.flag1[3] = 0;
    }

    runtime.rng = rng;
}

/// `CRagEffect::TexturedSlashLevel(const char* name, char kind)` -
/// `RagEffect.cpp:5032-5095`.
fn textured_slash_level(runtime: &mut EffectRuntime, name: &'static str, kind: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);
    let index = runtime.launch(PrimitiveId::Stin3, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;
    state.delta_pos = target;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;

        if kind == 4 || kind == 5 {
            slot.distance = 25.0;
            slot.flag1[3] = 3;
        } else if kind > 1 {
            slot.distance = 6.0;
            slot.flag1[3] = 2;
        } else {
            slot.distance = 25.0;
            slot.flag1[3] = 0;
        }

        slot.alpha_b = 0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = position[1] - 10.0;
        slot.rot_start = 0;
        slot.rise_angle = slot_index as i16;
        slot.height[1] = 2.0;
        slot.flag1[0] = 0;
        slot.flag1[1] = 0;

        if kind == 0 || kind == 2 || kind == 4 {
            slot.flag1[2] = 0;
        } else if kind == 1 || kind == 3 || kind == 5 {
            slot.flag1[2] = 1;
        }

        slot.flag1[4] = if kind == 2 || kind == 3 { 1 } else { 0 };
    }
}

/// `CRagEffect::TexturedSlashWave(const char* name, char kind)` -
/// `RagEffect.cpp:4687-4740`.
fn textured_slash_wave(runtime: &mut EffectRuntime, name: &'static str, kind: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);
    let index = runtime.launch(PrimitiveId::Stin4, [0.0, 0.0, 0.0]);
    let mut rng = std::mem::replace(&mut runtime.rng, EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;
    state.delta_pos = target;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;
        slot.distance = rng.clamped_random(11) as f32 * 0.5 + 10.0;
        slot.alpha_b = 0;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        slot.vec_b_now[1] = position[1] - 10.0;
        slot.rot_start = 0;
        slot.rise_angle = slot_index as i16;
        slot.height[1] = rng.clamped_random(21) as f32 * 0.1 + 1.0;
        slot.flag1[0] = 0;
        slot.flag1[1] = 0;

        if kind == 0 {
            slot.flag1[2] = 0;
        } else if kind == 1 {
            slot.flag1[2] = 1;
        }

        slot.flag1[3] = 1;
    }

    runtime.rng = rng;
}

/// `CRagEffect::TexturedSlashCross(const char* name)` -
/// `RagEffect.cpp:4902-4952`.
fn textured_slash_cross(runtime: &mut EffectRuntime, name: &'static str) {
    runtime.copy_parent_position();

    if runtime.frame >= 5 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);
    let frame = runtime.frame;
    let index = runtime.launch(PrimitiveId::SlashCross, [0.0, 0.0, 0.0]);
    let mut rng = std::mem::replace(&mut runtime.rng, EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;
    state.delta_pos = target;

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = slot_index as i32 - 3;
        slot.distance = 20.0;
        slot.alpha_b = 0;

        let (x, z) = match frame {
            0 => (position[0], position[2]),
            1 => (position[0] + 5.0, position[2]),
            2 => (position[0] - 5.0, position[2]),
            3 => (position[0], position[2] + 5.0),
            _ => (position[0], position[2] - 5.0),
        };

        slot.vec_b_now[0] = x;
        slot.vec_b_now[2] = z;
        slot.vec_b_now[1] = rng.clamped_random(5) as f32 + (position[1] - 10.0);
        slot.rot_start = rng.clamped_random(360) as i16;
        slot.rise_angle = slot_index as i16;
        slot.height[1] = 1.5;
        slot.flag1[0] = 0;
        slot.flag1[3] = 1;
    }

    runtime.rng = rng;
}

/// `EF_STIN` (547) - `RagEffect.cpp:21810-21814`.
#[derive(Debug)]
pub struct SlashFan {
    lifetime: i32,
}

impl SlashFan {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SlashFan {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 10 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\T_바람마법.wav");
        }

        textured_slash_fan(runtime, "effect\\270바람.tga");

        true
    }
}

/// `EF_STIN2` (553) - `RagEffect.cpp:21817-21828`.
#[derive(Debug)]
pub struct SlashLevel {
    lifetime: i32,
}

impl SlashLevel {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SlashLevel {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if [5, 20, 35, 50, 65].contains(&runtime.frame) {
            runtime.copy_parent_position();
            let height = if runtime.frame <= 50 { (5 - runtime.frame) as f32 } else { -50.0 };
            runtime.play_sound_at("effect\\T_날라차기.wav", [0.0, height, 0.0]);
        }

        textured_slash_fan(runtime, "effect\\270바람.tga");
        textured_slash_level(runtime, "effect\\270바람.tga", 0);
        textured_slash_level(runtime, "effect\\270바람.tga", 1);

        true
    }
}

/// `EF_STIN4` (621) - `RagEffect.cpp:21150-21156`.
#[derive(Debug)]
pub struct SlashWave {
    lifetime: i32,
}

impl SlashWave {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SlashWave {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\풍인.wav");
        }

        for index in 0..runtime.count.max(0) {
            textured_slash_wave(runtime, "effect\\270바람.tga", (index % 2) as i8);
        }

        true
    }
}

/// `EF_STIN5` (624) - `RagEffect.cpp:21117-21123`.
#[derive(Debug)]
pub struct SlashCross {
    lifetime: i32,
}

impl SlashCross {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SlashCross {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 1 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\t_바람방출.wav");
        }

        textured_slash_cross(runtime, "effect\\270바람.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);
        runtime.target_offset = [10.0, 20.0, 40.0];

        runtime
    }

    #[test]
    fn stin_launches_one_fan_on_frame_zero() {
        let mut body = SlashFan::new(200);
        let mut runtime = runtime();

        body.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::SlashFan);
        assert_eq!(state.count.id(), 119);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\270바람.tga"));
        assert_eq!(state.size, 0.0);
        // The target is converted to official (Y-down) simulation space.
        assert_eq!(state.delta_pos, [10.0, -20.0, 40.0]);

        for (index, slot) in state.slots.iter().enumerate() {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.process, index as i32 - 3);
            assert_eq!(slot.distance, 20.0);
            assert_eq!(slot.rot_start, 0);
            assert_eq!(slot.rise_angle, index as i16);
            assert_eq!(slot.height[1], 0.8);
            // `official position.y (-20) - 10 + ClampedRandom(5)` stays in
            // [-30, -25).
            assert!(slot.vec_b_now[1] >= -30.0 && slot.vec_b_now[1] < -25.0);
        }
    }

    #[test]
    fn stin2_launches_a_fan_and_two_levels() {
        let mut body = SlashLevel::new(200);
        let mut runtime = runtime();

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);
        assert_eq!(runtime.primitives[1].state.count, PrimitiveId::Stin3);
        assert_eq!(runtime.primitives[1].state.slots[0].flag1[2], 0);
        assert_eq!(runtime.primitives[2].state.slots[0].flag1[2], 1);
        assert_eq!(runtime.primitives[1].state.slots[0].flag1[3], 0);
        assert_eq!(runtime.primitives[1].state.slots[0].distance, 25.0);
    }

    #[test]
    fn stin2_plays_the_five_kick_waves_at_their_heights() {
        let mut body = SlashLevel::new(200);
        let mut runtime = runtime();

        for frame in [5, 20, 35, 50, 65] {
            runtime.frame = frame;
            runtime.sounds.clear();
            runtime.positional_sounds.clear();
            body.on_process(&mut runtime);

            let expected = if frame <= 50 { (5 - frame) as f32 } else { -50.0 };
            assert_eq!(runtime.positional_sounds[0].0, "effect\\T_날라차기.wav");
            assert_eq!(runtime.positional_sounds[0].1, [0.0, expected, 0.0]);
        }
    }

    #[test]
    fn stin4_launches_one_wave_per_count() {
        let mut body = SlashWave::new(200);
        let mut runtime = runtime();
        runtime.count = 3;

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);
        assert_eq!(runtime.sounds, vec!["effect\\풍인.wav".to_string()]);

        for index in 0..3usize {
            let slot = &runtime.primitives[index].state.slots[0];
            assert_eq!(runtime.primitives[index].state.count, PrimitiveId::Stin4);
            assert_eq!(slot.flag1[2], (index % 2) as i8);
            assert_eq!(slot.flag1[3], 1);
            assert!(slot.height[1] >= 1.0 && slot.height[1] <= 3.0);
        }
    }

    #[test]
    fn stin5_launches_a_cross_for_each_of_five_frames() {
        let mut body = SlashCross::new(200);
        let mut runtime = runtime();

        for frame in 0..6 {
            runtime.frame = frame;
            let before = runtime.primitives.len();
            body.on_process(&mut runtime);

            if frame == 1 {
                assert_eq!(runtime.sounds, vec!["effect\\t_바람방출.wav".to_string()]);
            }

            if frame >= 5 {
                assert_eq!(runtime.primitives.len(), before);
                continue;
            }

            assert_eq!(runtime.primitives.len(), before + 1);
            let primitive = runtime.primitives.last().unwrap();
            let slot = &primitive.state.slots[0];

            assert_eq!(primitive.state.count, PrimitiveId::SlashCross);
            assert_eq!(primitive.state.count.id(), 126);
            let expected_x = match frame {
                0 => 10.0,
                1 => 15.0,
                2 => 5.0,
                _ => 10.0,
            };
            let expected_z = match frame {
                3 => 35.0,
                4 => 25.0,
                _ => 30.0,
            };
            assert_eq!(slot.vec_b_now[0], expected_x);
            assert_eq!(slot.vec_b_now[2], expected_z);
            assert_eq!(slot.height[1], 1.5);
            assert_eq!(slot.flag1[3], 1);
        }
    }
}
