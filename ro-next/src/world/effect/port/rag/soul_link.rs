//! `CRagEffect::TexturedOrbit` / `CRagEffect::SoulLink` -
//! `RagEffect.cpp:4954-4992` and `:13885-13965`.
//!
//! `EF_SOULLINK` (`RagEffect.cpp:21566-21586`) plays `T_캐스팅.wav` on frame 0
//! and `T_영혼.wav` on frame 145 (at height -50), and every frame calls
//! `TexturedOrbit("effect\\whitelight.tga")` followed by `SoulLink(0)`.
//!
//! * `TexturedOrbit` spawns one prim 109 on frame 0: slot 0 at `distance =
//!   2.5`, `height[10] = deltaPos.y`, four `rand(360)` draws into
//!   `height[0..3]`, `vecBPre = (-sin(90)*30, -(sin(0)*20)-40, 0)`.
//! * `SoulLink` spawns one prim 68 (`PP_ASURACASTING`, the halo-layer renderer)
//!   on frames 1 and 21, with four letter textures and slots at `process =
//!   index*20 - 60` (frame 1) / `index*20 - 120` (frame 21), `height[0] = -4 -
//!   index*6` / `22 - index*6`, `flag1[0] = index` (texture) and `flag1[4] = 2`
//!   (colour mode).
//!
//! Lifetimes: `EF_SOULLINK` 500 (`EffectTypes.cpp:512`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedOrbit(const char* name)` - `RagEffect.cpp:4954-4992`.
fn textured_orbit(runtime: &mut EffectRuntime, name: &'static str) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let target_y = runtime.target_offset[1];
    let index = runtime.launch(PrimitiveId::SoulLink, [0.0, 0.0, 0.0]);

    let mut rng = std::mem::replace(&mut runtime.rng, super::super::runtime::EffectRng::new(1));
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = 0.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    let slot = &mut state.slots[0];
    slot.life = 1;
    slot.process = 0;
    slot.distance = 2.5;
    slot.height[10] = target_y;
    slot.height[13] = 0.0;
    slot.height[14] = 0.0;
    slot.height[15] = 0.0;
    slot.height[0] = rng.clamped_random(360) as f32;
    slot.height[1] = rng.clamped_random(360) as f32;
    slot.height[2] = rng.clamped_random(360) as f32;
    slot.height[3] = rng.clamped_random(360) as f32;
    slot.vec_t_now[0] = 0.0;
    let angles = super::super::angles::Angles::shared();
    slot.vec_b_pre[0] = -angles.raw_sine(90) * 30.0;
    slot.vec_b_pre[1] = -(angles.raw_sine(0) * 20.0) - 40.0;
    slot.vec_b_pre[2] = 0.0;

    runtime.rng = rng;
}

/// `CRagEffect::SoulLink(int)` - `RagEffect.cpp:13885-13965`.
fn soul_link(runtime: &mut EffectRuntime) {
    runtime.copy_parent_position();

    let (frame, textures, base) = match runtime.frame {
        1 => (
            1,
            [
                "effect\\soul_l.tga",
                "effect\\soul_u.tga",
                "effect\\soul_o.tga",
                "effect\\soul_s.tga",
            ],
            -4.0,
        ),
        21 => (
            21,
            [
                "effect\\soul_k.tga",
                "effect\\soul_n.tga",
                "effect\\soul_i.tga",
                "effect\\soul_l.tga",
            ],
            22.0,
        ),
        _ => return,
    };

    let process_base = if frame == 1 { -60 } else { -120 };
    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::AsuraCasting, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 4;
    state.frame_count = 4;
    state.texture_frames = textures.to_vec();
    state.size = 0.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = slot_index as i32 * 20 + process_base;
        slot.distance = 5.0;
        slot.alpha_b = 0;
        slot.height[0] = base - slot_index as f32 * 6.0;
        slot.flag1[0] = slot_index as i8;
        slot.flag1[4] = 2;

        for height in 1..11usize {
            slot.height[height] = 0.0;
        }
    }
}

/// `case EF_SOULLINK:` - `RagEffect.cpp:21566-21586`.
#[derive(Debug)]
pub struct SoulLink {
    lifetime: i32,
}

impl SoulLink {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SoulLink {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.copy_parent_position();

        if runtime.frame == 0 {
            runtime.play_sound("effect\\T_캐스팅.wav");
        }

        if runtime.frame == 145 {
            runtime.play_sound_at_height("effect\\T_영혼.wav", -50.0);
        }

        textured_orbit(runtime, "effect\\whitelight.tga");
        soul_link(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn soullink_creates_the_orbit_on_frame_zero_and_sounds() {
        let mut body = SoulLink::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);
        runtime.parent_position = Some([1.0, 2.0, 3.0]);
        runtime.target_offset = [1.0, 7.0, 3.0];

        step(&mut body, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\T_캐스팅.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::SoulLink);
        assert_eq!(state.count.id(), 109);
        assert_eq!(state.texture, Some("effect\\whitelight.tga"));
        assert_eq!(state.size, 0.0);
        assert_eq!(state.slots[0].life, 1);
        assert_eq!(state.slots[0].distance, 2.5);
        assert_eq!(state.slots[0].height[10], 7.0);
    }

    #[test]
    fn soul_link_spawns_the_first_letter_set_on_frame_one() {
        let mut body = SoulLink::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);
        runtime.frame = 1;

        body.on_process(&mut runtime);

        // TexturedOrbit returns early; only the SoulLink prim exists.
        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::AsuraCasting);
        assert_eq!(state.texture_frames, vec![
            "effect\\soul_l.tga",
            "effect\\soul_u.tga",
            "effect\\soul_o.tga",
            "effect\\soul_s.tga"
        ]);
        assert_eq!(state.total_texture, 4);
        assert_eq!(state.slots[0].process, -60);
        assert_eq!(state.slots[3].process, 0);
        assert_eq!(state.slots[0].height[0], -4.0);
        assert_eq!(state.slots[3].height[0], -22.0);
        assert_eq!(state.slots[2].flag1[0], 2);
        assert_eq!(state.slots[0].flag1[4], 2);
        assert!(state.slots.iter().all(|slot| slot.life == 1));
    }

    #[test]
    fn the_second_letter_set_uses_the_k_n_i_l_textures() {
        let mut body = SoulLink::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);
        runtime.frame = 21;

        body.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.texture_frames[0], "effect\\soul_k.tga");
        assert_eq!(state.texture_frames[3], "effect\\soul_l.tga");
        assert_eq!(state.slots[0].process, -120);
        assert_eq!(state.slots[0].height[0], 22.0);
        assert_eq!(state.slots[3].height[0], 4.0);
    }

    #[test]
    fn frame_one_forty_five_plays_the_soul_wave_at_minus_fifty() {
        let mut body = SoulLink::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);
        runtime.frame = 145;

        body.on_process(&mut runtime);

        assert_eq!(runtime.positional_sounds[0].0, "effect\\T_영혼.wav");
        assert_eq!(runtime.positional_sounds[0].1, [0.0, -50.0, 0.0]);
    }
}
