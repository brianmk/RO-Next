//! `CRagEffect::BeginSpellSwirl(int, const char*, char alpha)` -
//! `RagEffect.cpp:9548-9650` - and the `EF_BEGINSPELL_N6` dispatch that calls
//! it (`RagEffect.cpp:19925-19926`).
//!
//! Frame 0 plays the positional `effect\EF_BeginSpell.wav` at height `-100`,
//! raises the effect lifetime to at least 120 frames and launches prim 149
//! (`PrimWaveHeights`) with four ground arcs. `alpha` selects the asset set and
//! is copied into every slot's `slots[].alphaT`, which the renderer uses to
//! pick the glow colour and alpha multiplier:
//!
//! * `10` - the four `effect\YBfire_%02d.bmp` frames (never used by the
//!   `EF_BEGINSPELL_N*` dispatch, kept for completeness),
//! * everything else (`3`..`9` for N3..N8) - the 27 `effect\W_bubble%02d.tga`
//!   frames.
//!
//! The `EF_BEGINSPELL_N2` dispatch also runs `TexturedTwinSpin`, which is not
//! part of this body; only the swirl half is ported here.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `effect\W_bubble01.tga` .. `effect\W_bubble27.tga` - the default swirl
/// frames (`RagEffect.cpp:9630-9634`).
const W_BUBBLE_TEXTURES: [&str; 27] = [
    "effect\\W_bubble01.tga",
    "effect\\W_bubble02.tga",
    "effect\\W_bubble03.tga",
    "effect\\W_bubble04.tga",
    "effect\\W_bubble05.tga",
    "effect\\W_bubble06.tga",
    "effect\\W_bubble07.tga",
    "effect\\W_bubble08.tga",
    "effect\\W_bubble09.tga",
    "effect\\W_bubble10.tga",
    "effect\\W_bubble11.tga",
    "effect\\W_bubble12.tga",
    "effect\\W_bubble13.tga",
    "effect\\W_bubble14.tga",
    "effect\\W_bubble15.tga",
    "effect\\W_bubble16.tga",
    "effect\\W_bubble17.tga",
    "effect\\W_bubble18.tga",
    "effect\\W_bubble19.tga",
    "effect\\W_bubble20.tga",
    "effect\\W_bubble21.tga",
    "effect\\W_bubble22.tga",
    "effect\\W_bubble23.tga",
    "effect\\W_bubble24.tga",
    "effect\\W_bubble25.tga",
    "effect\\W_bubble26.tga",
    "effect\\W_bubble27.tga",
];

/// `effect\YBfire_01.bmp` .. `effect\YBfire_04.bmp` - the `alpha == 10` set
/// (`RagEffect.cpp:9619-9622`).
const YB_FIRE_TEXTURES: [&str; 4] = [
    "effect\\YBfire_01.bmp",
    "effect\\YBfire_02.bmp",
    "effect\\YBfire_03.bmp",
    "effect\\YBfire_04.bmp",
];

/// `case EF_BEGINSPELL_N6:` - `RagEffect.cpp:19925-19926`.
#[derive(Debug)]
pub struct BeginSpellSwirl {
    /// The `char alpha` argument: `6` for `EF_BEGINSPELL_N6`.
    alpha: i8,
    lifetime: i32,
}

impl BeginSpellSwirl {
    pub fn new(alpha: i8, lifetime: i32) -> Self {
        Self { alpha, lifetime }
    }
}

impl PortedEffect for BeginSpellSwirl {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        begin_spell_swirl(runtime, self.alpha);

        true
    }
}

/// `CRagEffect::BeginSpellSwirl(int, const char*, char alpha)` -
/// `RagEffect.cpp:9548-9650`.
fn begin_spell_swirl(runtime: &mut EffectRuntime, alpha: i8) {
    // `this->render.position = parent.position;` runs before the frame gate.
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    // `g_soundMgr->PlayWave("effect\\EF_BeginSpell.wav", render.x - player.x,
    //  -100.0f, render.z - player.z, 250, 40, 1.0f, 0);`
    runtime.play_sound_at_height("effect\\EF_BeginSpell.wav", -100.0);

    if runtime.lifetime < 120 {
        runtime.lifetime = 120;
    }

    let lifetime = runtime.lifetime;
    let textures: &'static [&'static str] = if alpha == 10 { &YB_FIRE_TEXTURES } else { &W_BUBBLE_TEXTURES };

    let swirl = runtime.launch(PrimitiveId::WaveHeights, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(swirl);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.total_texture = textures.len() as u32;
    state.texture = textures.first().copied();
    state.texture_frames = textures.to_vec();
    state.size = 0.0;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = 0;
        slot.full_display_angle = 360;
        slot.max_height = 23.0;
        slot.rot_start = [180i16, 270, 0, 90][index];
        slot.alpha_b = 0;
        slot.distance = 4.3;
        slot.rise_angle = [75i16, 63, 52, 40][index];
    }

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.alpha_t = i16::from(alpha);
        slot.vec_b_now[1] = 0.0;

        for value in 0..21usize {
            slot.height[value] = 0.0;
            slot.flag1[value] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 3);
        runtime.position = [10.0, 20.0, 30.0];
        runtime.listener_position = [1.0, 2.0, 3.0];
        runtime
    }

    #[test]
    fn n6_raises_the_lifetime_and_launches_the_swirl() {
        let mut effect = BeginSpellSwirl::new(6, 100);
        let mut runtime = runtime();

        // `on_process` alone: the same-frame primitive pass would already have
        // run one `PrimWaveHeights` step on the fresh primitive.
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.lifetime, 120, "`if (duration < 120) duration = 120;`");

        let swirl = &runtime.primitives[0].state;
        assert_eq!(swirl.count, PrimitiveId::WaveHeights);
        assert_eq!(swirl.duration, 120);
        assert_eq!(swirl.render_flag, 5);
        assert_eq!(swirl.total_texture, 27);
        assert_eq!(swirl.texture, Some("effect\\W_bubble01.tga"));
        assert_eq!(swirl.texture_frames.len(), 27);
        assert_eq!(swirl.size, 0.0);
        assert_eq!(swirl.slots[0].rot_start, 180);
        assert_eq!(swirl.slots[1].rot_start, 270);
        assert_eq!(swirl.slots[2].rot_start, 0);
        assert_eq!(swirl.slots[3].rot_start, 90);
        assert_eq!(swirl.slots[0].rise_angle, 75);
        assert_eq!(swirl.slots[3].rise_angle, 40);

        for slot in &swirl.slots {
            assert_eq!(slot.alpha_t, 6, "the `alpha` argument is copied to every slot");
            assert_eq!(slot.full_display_angle, 360);
            assert_eq!(slot.max_height, 23.0);
            assert_eq!(slot.distance, 4.3);
        }
    }

    #[test]
    fn the_begin_spell_wave_is_positional_and_matches_play_effect_wave_at() {
        let mut effect = BeginSpellSwirl::new(6, 100);
        let mut runtime = runtime();

        assert!(effect.on_process(&mut runtime));

        // `PlayWave(name, effect.x - player.x, -100, effect.z - player.z, ...)`.
        assert_eq!(runtime.positional_sounds, vec![("effect\\EF_BeginSpell.wav".to_owned(), [
            9.0, -100.0, 27.0
        ])]);
        assert!(runtime.sounds.is_empty());
    }

    #[test]
    fn the_alpha_ten_branch_uses_the_ybfire_frames() {
        let mut effect = BeginSpellSwirl::new(10, 100);
        let mut runtime = runtime();

        assert!(effect.on_process(&mut runtime));

        let swirl = &runtime.primitives[0].state;
        assert_eq!(swirl.total_texture, 4);
        assert_eq!(swirl.texture, Some("effect\\YBfire_01.bmp"));
        assert_eq!(swirl.texture_frames, YB_FIRE_TEXTURES.to_vec());
        assert_eq!(swirl.slots[0].alpha_t, 10);
    }

    #[test]
    fn the_launch_only_happens_on_frame_zero() {
        let mut effect = BeginSpellSwirl::new(6, 100);
        let mut runtime = runtime();

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.positional_sounds.len(), 1);
    }
}
