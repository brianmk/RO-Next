//! Effects ported by the "g" parallel batch.
//!
//! This file is owned exclusively by batch "g": bodies and their tests live
//! here so parallel work never edits a shared module. Every unit is a direct
//! transliteration of one decompiled `CRagEffect::OnProcess` dispatch block;
//! the frame gates, literal constants, RNG consumption order and the official
//! quirks are kept.
//!
//! The dispatch blocks call existing ported helpers verbatim:
//! [`TexturedQuadSpin`] (`RagEffect.cpp:3617-3653`), [`HealRingSparkle`]
//! (`RagEffect.cpp:17970-18186`) and [`TexturedAura`]
//! (`RagEffect.cpp:17769-17965`). Those helpers are `PortedEffect`s already;
//! the bodies below add only the per-frame sound/colour scheduling around
//! them, so the quads and auras are still produced by the original single
//! helper implementation.
//!
//! Two official side effects have no channel in the hermetic runtime and are
//! documented at their call site: the `render.position =
//! state.parent->render.position` copy (the runtime's `position` already is
//! the master anchor filled by the spawn path) and the per-frame flag word
//! (`render.flags` is modelled by [`EffectRuntime::add_parent_flags`], whose
//! `begin_step` clear matches `GameObject.cpp:435`).

#![allow(dead_code)]

use super::rag::{HealRingSparkle, TexturedAura, TexturedQuadSpin};
use super::runtime::{EffectRuntime, PortedEffect};

/// `effect\\alpha_center.tga` - the quad-spin texture of the Bash3D family.
const QUAD_TEXTURE: &str = "effect\\alpha_center.tga";

// ---------------------------------------------------------------------------
// EF_BASH3D
// ---------------------------------------------------------------------------

/// `case EF_BASH3D:` - `RagEffect.cpp:10350-10364`.
///
/// Frame 22 plays `effect\세크리파이스.wav`. The dispatch then runs the
/// official `for (phase = 0; phase < 5; ++phase)` loop over
/// [`TexturedQuadSpin`] with kind 0; the helper itself returns as soon as
/// `stateCnt != 0` (`RagEffect.cpp:3620`), so all five phase calls do their
/// work on frame 0 and are no-ops afterwards. Frames 20-40 request the
/// caster's pulse flag (bit 8) and `SetColor(-1, 255, 200, 200)`.
///
/// The helper call order is kept: sound, phase loop, colour window.
#[derive(Debug)]
pub struct Bash3d {
    quads: TexturedQuadSpin,
}

impl Bash3d {
    pub fn new() -> Self {
        // `TexturedQuadSpin(name, phase, kind)` called for phases 0..5 with
        // kind 0; the port helper takes the phase count instead.
        Self {
            quads: TexturedQuadSpin::new(QUAD_TEXTURE, 0, 5, 200),
        }
    }
}

impl PortedEffect for Bash3d {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:375` - `result[EF_BASH3D] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 22 {
            runtime.play_sound("effect\\세크리파이스.wav");
        }

        // `for (phase = 0; phase < 5; ++phase) TexturedQuadSpin(..., phase,
        // 0);` - the helper no-ops once `stateCnt != 0`.
        self.quads.on_process(runtime);

        if (20..=40).contains(&frame) {
            runtime.add_parent_flags(8);
            runtime.set_parent_color(-1, 255, 200, 200);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_BASH3D3
// ---------------------------------------------------------------------------

/// `case EF_BASH3D3:` - `RagEffect.cpp:24256-24270`.
///
/// Frame 0 plays `effect\헤드 크러쉬.wav`. [`TexturedQuadSpin`] runs six
/// phase calls with kind 4; every phase uses the kind-4 branch
/// (`process = 0`, 7-degree `riseAngle` step). Frames 20-50 request flag 8
/// and `SetColor(-1, 255, 255, 200)`.
#[derive(Debug)]
pub struct Bash3d3 {
    quads: TexturedQuadSpin,
}

impl Bash3d3 {
    pub fn new() -> Self {
        Self {
            quads: TexturedQuadSpin::new(QUAD_TEXTURE, 4, 6, 200),
        }
    }
}

impl PortedEffect for Bash3d3 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:408` - `result[EF_BASH3D3] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            runtime.play_sound("effect\\헤드 크러쉬.wav");
        }

        // `for (phase = 0; phase < 6; ++phase) TexturedQuadSpin(..., phase,
        // 4);`
        self.quads.on_process(runtime);

        if (20..=50).contains(&frame) {
            runtime.add_parent_flags(8);
            runtime.set_parent_color(-1, 255, 255, 200);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_BASH3D4
// ---------------------------------------------------------------------------

/// `case EF_BASH3D4:` - `RagEffect.cpp:24271-24285`.
///
/// Frame 22 plays `effect\비트 조인트.wav`. [`TexturedQuadSpin`] runs six
/// phase calls with kind 5 (also the `process = 0` branch). Frames 20-50
/// request flag 8 and `SetColor(-1, 255, 255, 255)`.
///
/// The quad geometry is identical to `EF_BASH3D5` (registered with kind 5,
/// six phases); only the extra sound/colour schedule differs.
#[derive(Debug)]
pub struct Bash3d4 {
    quads: TexturedQuadSpin,
}

impl Bash3d4 {
    pub fn new() -> Self {
        Self {
            quads: TexturedQuadSpin::new(QUAD_TEXTURE, 5, 6, 200),
        }
    }
}

impl PortedEffect for Bash3d4 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:409` - `result[EF_BASH3D4] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 22 {
            runtime.play_sound("effect\\비트 조인트.wav");
        }

        // `for (phase = 0; phase < 6; ++phase) TexturedQuadSpin(..., phase,
        // 5);`
        self.quads.on_process(runtime);

        if (20..=50).contains(&frame) {
            runtime.add_parent_flags(8);
            runtime.set_parent_color(-1, 255, 255, 255);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_TRUESIGHT
// ---------------------------------------------------------------------------

/// `case EF_TRUESIGHT:` - `RagEffect.cpp:24286-24294`.
///
/// Frame 0 plays `effect\hunter_detecting.wav`. [`TexturedQuadSpin`] runs
/// twelve phase calls with kind 3 (the 7-degree kind-3 `riseAngle` step and
/// `process = 0`). Unlike the Bash3D ids this block has no colour/flag window.
#[derive(Debug)]
pub struct TrueSight {
    quads: TexturedQuadSpin,
}

impl TrueSight {
    pub fn new() -> Self {
        Self {
            quads: TexturedQuadSpin::new(QUAD_TEXTURE, 3, 12, 250),
        }
    }
}

impl PortedEffect for TrueSight {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:395` - `result[EF_TRUESIGHT] = 250`.
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.play_sound("effect\\hunter_detecting.wav");
        }

        // `for (phase = 0; phase < 12; ++phase) TexturedQuadSpin(..., phase,
        // 3);`
        self.quads.on_process(runtime);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_ABSORBSPIRITS
// ---------------------------------------------------------------------------

/// `case EF_ABSORBSPIRITS:` - `RagEffect.cpp:23122-23128`.
///
/// Frame 0 plays the positional `effect\흡기.wav` at height -80
/// (`PlayEffectWaveAt`), then every frame calls
/// `HealRingSparkle("effect\ring_blue.tga", 2)`. The helper spawns its
/// four-column ring only on frame 0; type 2 both picks the small `size = 4`
/// ring and suppresses the `_heal_effect.wav` and the pok-particle bursts
/// (`RagEffect.cpp:18015`, `:18016`, `:18119`), so this id is a single
/// four-column `PP_RECT_UP`/prim-60 ring for its whole 100-frame lifetime.
#[derive(Debug)]
pub struct AbsorbSpirits {
    ring: HealRingSparkle,
}

impl AbsorbSpirits {
    pub fn new() -> Self {
        Self {
            ring: HealRingSparkle::new("effect\\ring_blue.tga", 2, 100),
        }
    }
}

impl Default for AbsorbSpirits {
    fn default() -> Self {
        Self::new()
    }
}

impl PortedEffect for AbsorbSpirits {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:262` - `result[EF_ABSORBSPIRITS] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            // `PlayEffectWaveAt(*this, "effect\흡기.wav", -80.0f)`.
            runtime.play_sound_at_height("effect\\흡기.wav", -80.0);
        }

        self.ring.on_process(runtime);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_HITTEXTURE
// ---------------------------------------------------------------------------

/// `case EF_HITTEXTURE:` - `RagEffect.cpp:21741-21743`.
///
/// One-line dispatch: `TexturedAura("effect\freeze_a.bmp", 2)`
/// (`RagEffect.cpp:17769-17965`). Kind 2 is the
/// `(distance 10, flag1[2] 2, height[10] 0, display 135, vecBPre.y -10)`
/// branch; the helper's common tail draws `rotStart` and writes the anchor
/// vectors.
#[derive(Debug)]
pub struct HitTexture {
    aura: TexturedAura,
}

impl HitTexture {
    pub fn new() -> Self {
        Self {
            aura: TexturedAura::new(&[("effect\\freeze_a.bmp", 2)], 200),
        }
    }
}

impl Default for HitTexture {
    fn default() -> Self {
        Self::new()
    }
}

impl PortedEffect for HitTexture {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:563` - `result[EF_HITTEXTURE] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        self.aura.on_process(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::primitive::PrimitiveId;
    use super::super::runtime::{EffectRng, EffectRuntime, run_to_completion, step};
    use super::*;

    fn runtime_with(seed: u64, lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], seed)
    }

    // -- EF_BASH3D ---------------------------------------------------------

    #[test]
    fn bash3d_frame_zero_spawns_five_kind_zero_quad_spins() {
        let mut effect = Bash3d::new();
        let mut runtime = runtime_with(366, 200);

        assert!(effect.on_process(&mut runtime));

        assert!(runtime.sounds.is_empty(), "the wave is on frame 22, not 0");
        assert_eq!(runtime.primitives.len(), 5);

        // The helper draws four rotStart values per phase, in phase order.
        let mut rng = EffectRng::new(366);

        for (phase, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::SpinQuad);
            assert_eq!(state.duration, 200);
            assert_eq!(state.render_flag, 1 | 4);
            assert_eq!(state.texture, Some(QUAD_TEXTURE));
            assert_eq!(state.roll, 0.0);

            for (index, slot) in state.slots.iter().enumerate() {
                assert_eq!(slot.life, 1);
                assert_eq!(slot.height[4], 0.0, "kind 0");
                assert_eq!(slot.process, -24);
                assert_eq!(slot.distance, 2.0);
                assert_eq!(slot.rise_angle, (phase as i32 * 22 + index as i32 * 90) as i16);
                assert_eq!(slot.rot_start, rng.clamped_random(360) as i16);
                assert_eq!(slot.alpha_b, 0);
                assert_eq!(slot.height[0], -12.0);
            }
        }
    }

    #[test]
    fn bash3d_plays_the_wave_on_22_and_tints_through_40() {
        let mut effect = Bash3d::new();
        let mut runtime = runtime_with(1, 200);

        // Frames 0..18: before the window.
        for _ in 0..19 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 19);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // frame 19
        assert_eq!(runtime.parent_color, None);

        step(&mut effect, &mut runtime); // frame 20
        assert_eq!(runtime.parent_color, Some([-1, 255, 200, 200]));
        assert_eq!(runtime.parent_flags, 8);

        step(&mut effect, &mut runtime); // frame 21
        step(&mut effect, &mut runtime); // frame 22
        assert_eq!(runtime.sounds, vec!["effect\\세크리파이스.wav".to_owned()]);

        while runtime.frame < 41 {
            step(&mut effect, &mut runtime);
        }
        // Frame 40 was the last window frame and still requested the flag.
        assert_eq!(runtime.parent_flags, 8);
        assert_eq!(runtime.parent_color, Some([-1, 255, 200, 200]));

        step(&mut effect, &mut runtime); // frame 41, outside the window
        assert_eq!(runtime.parent_flags, 0);
        // `SetColor` persists on the actor; only the flag word is per-frame.
        assert_eq!(runtime.parent_color, Some([-1, 255, 200, 200]));
    }

    // -- EF_BASH3D3 --------------------------------------------------------

    #[test]
    fn bash3d3_frame_zero_spawns_six_kind_four_quads_and_plays_the_wave() {
        let mut effect = Bash3d3::new();
        let mut runtime = runtime_with(399, 200);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, vec!["effect\\헤드 크러쉬.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 6);

        let mut rng = EffectRng::new(399);

        for (phase, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::SpinQuad);
            assert_eq!(state.duration, 200);

            for (index, slot) in state.slots.iter().enumerate() {
                assert_eq!(slot.height[4], 4.0, "kind 4");
                // Only kinds 2/3/6 use `process = 0`; kind 4 takes the
                // `process = -24` else branch (`RagEffect.cpp:3636`).
                assert_eq!(slot.process, -24);
                assert_eq!(slot.distance, 2.0);
                assert_eq!(slot.rise_angle, (phase as i32 * 22 + index as i32 * 90) as i16);
                assert_eq!(slot.rot_start, rng.clamped_random(360) as i16);
            }
        }
    }

    #[test]
    fn bash3d3_tints_between_20_and_50() {
        let mut effect = Bash3d3::new();
        let mut runtime = runtime_with(3, 200);

        for _ in 0..20 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 20);
        assert_eq!(runtime.parent_color, None);

        step(&mut effect, &mut runtime); // frame 20
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 200]));
        assert_eq!(runtime.parent_flags, 8);

        while runtime.frame < 51 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.parent_flags, 8, "frame 50 was still inside");

        step(&mut effect, &mut runtime); // frame 51
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 200]));
    }

    // -- EF_BASH3D4 --------------------------------------------------------

    #[test]
    fn bash3d4_frame_zero_spawns_six_kind_five_quads() {
        let mut effect = Bash3d4::new();
        let mut runtime = runtime_with(400, 200);

        assert!(effect.on_process(&mut runtime));

        assert!(runtime.sounds.is_empty(), "the wave is on frame 22, not 0");
        assert_eq!(runtime.primitives.len(), 6);

        let mut rng = EffectRng::new(400);

        for (phase, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::SpinQuad);
            assert_eq!(state.texture, Some(QUAD_TEXTURE));

            for (index, slot) in state.slots.iter().enumerate() {
                assert_eq!(slot.height[4], 5.0, "kind 5");
                // Only kinds 2/3/6 use `process = 0`.
                assert_eq!(slot.process, -24);
                assert_eq!(slot.rise_angle, (phase as i32 * 22 + index as i32 * 90) as i16);
                assert_eq!(slot.rot_start, rng.clamped_random(360) as i16);
            }
        }
    }

    #[test]
    fn bash3d4_plays_the_wave_on_22_and_tints_through_50() {
        let mut effect = Bash3d4::new();
        let mut runtime = runtime_with(4, 200);

        for _ in 0..23 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 23);
        assert_eq!(runtime.sounds, vec!["effect\\비트 조인트.wav".to_owned()]);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
        assert_eq!(runtime.parent_flags, 8);

        while runtime.frame < 51 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.parent_flags, 8, "frame 50 was still inside");

        step(&mut effect, &mut runtime); // frame 51
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
    }

    // -- EF_TRUESIGHT ------------------------------------------------------

    #[test]
    fn truesight_frame_zero_spawns_twelve_kind_three_quads_and_plays_the_wave() {
        let mut effect = TrueSight::new();
        let mut runtime = runtime_with(386, 250);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, vec!["effect\\hunter_detecting.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 12);

        let mut rng = EffectRng::new(386);

        for (phase, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::SpinQuad);
            assert_eq!(state.duration, 250);
            assert_eq!(state.texture, Some(QUAD_TEXTURE));

            for (index, slot) in state.slots.iter().enumerate() {
                assert_eq!(slot.height[4], 3.0, "kind 3");
                assert_eq!(slot.process, 0);
                // QUIRK kept: only kind 3 steps riseAngle by 7 per phase.
                assert_eq!(slot.rise_angle, (phase as i32 * 7 + index as i32 * 90) as i16);
                assert_eq!(slot.rot_start, rng.clamped_random(360) as i16);
            }
        }
    }

    #[test]
    fn truesight_has_no_tint_and_keeps_its_twelve_quads_for_250_frames() {
        let mut effect = TrueSight::new();
        let mut runtime = runtime_with(5, 250);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 250);
        assert_eq!(runtime.primitives.len(), 12);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);
    }

    // -- EF_ABSORBSPIRITS --------------------------------------------------

    #[test]
    fn absorbspirits_frame_zero_raises_the_blue_ring_and_the_intake_wave() {
        let mut effect = AbsorbSpirits::new();
        let mut runtime = runtime_with(253, 100);
        runtime.position = [12.0, 5.0, 7.0];

        assert!(effect.on_process(&mut runtime));

        // `PlayEffectWaveAt(..., -80.0f)` is positional; `_heal_effect.wav`
        // is suppressed for type 2.
        assert!(runtime.sounds.is_empty());
        assert_eq!(runtime.positional_sounds, vec![("effect\\흡기.wav".to_owned(), [
            12.0, -80.0, 7.0
        ])]);

        assert_eq!(runtime.primitives.len(), 1);
        let ring = &runtime.primitives[0].state;

        assert_eq!(ring.count, PrimitiveId::PulsingColumns);
        assert_eq!(ring.duration, 100);
        assert_eq!(ring.render_flag, 1 | 4);
        assert_eq!(ring.texture, Some("effect\\ring_blue.tga"));
        // Type 2 is not type 0: `size` and the small maxHeight follow the
        // else branch (`RagEffect.cpp:17976`, `:18035`).
        assert_eq!(ring.size, 4.0);
        assert!(ring.slots.iter().all(|slot| slot.life == 1));
        assert!(ring.slots.iter().all(|slot| slot.alpha_t == 40));
        assert_eq!(ring.slots[0].max_height, 60.0);
        assert_eq!(ring.slots[1].max_height, 60.0);
        assert_eq!(ring.slots[2].max_height, 18.0);
        assert_eq!(ring.slots[3].max_height, 18.0);
        assert_eq!(ring.slots[0].rise_angle, 90);
        assert_eq!(ring.slots[2].rise_angle, 50);
        assert_eq!(ring.slots[3].rise_angle, 48);

        // Slots 2 and 3 are the only RNG draws of `HealRingSparkle`.
        let mut rng = EffectRng::new(253);
        assert_eq!(ring.slots[2].rot_start, rng.clamped_random(360) as i16);
        assert_eq!(ring.slots[3].rot_start, rng.clamped_random(360) as i16);
    }

    #[test]
    fn absorbspirits_stays_a_single_ring_for_the_whole_lifetime() {
        let mut effect = AbsorbSpirits::new();
        let mut runtime = runtime_with(253, 100);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 100);
        // Type 2 has neither the 6-frame pok sparks nor the type-1 ring.
        assert_eq!(runtime.primitives.len(), 1);
    }

    // -- EF_HITTEXTURE -----------------------------------------------------

    #[test]
    fn hittexture_frame_zero_spawns_the_kind_two_freeze_aura() {
        let mut effect = HitTexture::new();
        let mut runtime = runtime_with(554, 200);
        runtime.position = [3.0, 7.0, -2.0];

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 1);
        let aura = &runtime.primitives[0].state;

        assert_eq!(aura.count, PrimitiveId::AuraGlow);
        assert_eq!(aura.duration, 200);
        assert_eq!(aura.render_flag, 1 | 4);
        assert_eq!(aura.texture, Some("effect\\freeze_a.bmp"));
        assert_eq!(aura.size, 0.0);
        assert!(aura.slots.iter().skip(1).all(|slot| slot.life == 0));

        let slot = &aura.slots[0];
        // Kind 2: `(10.0, 2, 0.0, 135, -10.0)` from `kind_params`.
        assert_eq!(slot.life, 1);
        assert_eq!(slot.distance, 10.0);
        assert_eq!(slot.flag1[2], 2);
        assert_eq!(slot.height[10], 0.0);
        assert_eq!(slot.full_display_angle, 135);
        assert_eq!(slot.flag1[4], 2);
        // Official Y-down: port y 7 is -7, then the kind's -10 lift.
        assert_eq!(slot.vec_b_pre[1], -17.0, "official position.y -7 + lift -10");
        assert_eq!(slot.height[15], 10.0);
        assert_eq!(slot.vec_b_now[0], 3.0);
        assert_eq!(slot.vec_b_now[2], -2.0);

        let mut rng = EffectRng::new(554);
        assert_eq!(slot.rot_start, rng.clamped_random(360) as i16);
    }
}
