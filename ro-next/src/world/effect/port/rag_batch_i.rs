//! Effects ported by the "i" parallel batch - the field-complete remainder of
//! the manual (T3) tier.
//!
//! Every body below is a direct transliteration of one decompiled
//! `CRagEffect::OnProcess` dispatch block that launches no primitive and opens
//! no `.str` clip: the official code only writes the caster actor's
//! `render.flags` / `render.colorChannels`, sends `MM_QUAKE`, rotates the
//! caster's `render.angle`, or plays one wave. The port routes those writes
//! through the channels batch "h" established:
//!
//! * [`EffectRuntime::add_parent_flags`] - the `if ((flags & bit) == 0) flags
//!   += bit` idiom. `begin_step` clears the word every frame, matching the
//!   official per-frame clear (`GameObject.cpp:435`), so the OR is equivalent.
//! * [`EffectRuntime::set_parent_color`] - `SetColor(-1, r, g, b)` recorded
//!   verbatim, including the `-1` channels.
//! * [`EffectRuntime::quake`] - `MM_QUAKE`.
//! * [`EffectRuntime::play_sound`] - `PlayEffectWave` and
//!   `PlayEffectWaveInline`, byte-identical helpers (`RagEffect.cpp:152-165`).
//! * [`EffectRuntime::parent_angle`] - the caster's `render.angle`; the body
//!   mutates the per-effect copy exactly like the original, wrapping with the
//!   single `> 360.0` subtraction.
//!
//! `render.position = state.parent->render.position` is a no-op in the
//! hermetic runtime (its [`EffectRuntime::position`] is already the master
//! anchor the spawn path fills from the parent), so the bodies that make that
//! copy only document it. Frame gates, literals, the `%` parity tests and the
//! RNG draw order are kept.
//!
//! This batch ports ten of the twelve ids the prior assessment listed.
//! `EF_LIGHTBLADE` and `EF_AURABLADE2` are owned by the concurrent Champion
//! batch (`port/rag/champion.rs`). The tenth, `EF_SU_STOOP`
//! (`RagEffect.cpp:22202-22207`), needed the missing `EffectId::SuStoop`
//! variant (official id 1132, `EffectTypes.h:1140`) added alongside this body;
//! it was declared with its real numeric value so no existing id shifted.

#![allow(dead_code)]

use super::runtime::{EffectRuntime, PortedEffect};

// ---------------------------------------------------------------------------
// EF_REDBODY
// ---------------------------------------------------------------------------

/// `case EF_REDBODY:` - `RagEffect.cpp:10371-10376`.
///
/// No frame gate: every frame requests caster flag 1 (additive body blend) and
/// calls `SetColor(-1, 255, 100, 100)`. No RNG, no sound.
#[derive(Debug, Default)]
pub struct RedBody;

impl RedBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for RedBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:377` - `result[EF_REDBODY] = 99999`.
        99999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.add_parent_flags(1);
        runtime.set_parent_color(-1, 255, 100, 100);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_ASSUMPTIO
// ---------------------------------------------------------------------------

/// `case EF_ASSUMPTIO:` - `RagEffect.cpp:24321-24325`.
///
/// No frame gate: every frame only requests caster flag 8 (the pulse/wave
/// outline bit). `SetColor` is never called, so the actor's colour is left
/// untouched.
#[derive(Debug, Default)]
pub struct Assumptio;

impl Assumptio {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for Assumptio {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:384` - `result[EF_ASSUMPTIO] = 99999`.
        99999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.add_parent_flags(8);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_REFLECTBODY
// ---------------------------------------------------------------------------

/// `case EF_REFLECTBODY:` - `RagEffect.cpp:24509-24513`.
///
/// No frame gate: every frame only requests caster flag `0x200`.
#[derive(Debug, Default)]
pub struct ReflectBody;

impl ReflectBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for ReflectBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:428` - `result[EF_REFLECTBODY] = 99999999`.
        99999999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.add_parent_flags(0x200);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_PINKBODY
// ---------------------------------------------------------------------------

/// `case EF_PINKBODY:` - `RagEffect.cpp:24443-24451`.
///
/// No frame gate: every frame requests caster flags 1 and 8 (in that order) and
/// calls `SetColor(-1, 255, 89, 182)`.
#[derive(Debug, Default)]
pub struct PinkBody;

impl PinkBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for PinkBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:405` - `result[EF_PINKBODY] = 99999999`.
        99999999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.add_parent_flags(1);
        runtime.add_parent_flags(8);
        runtime.set_parent_color(-1, 255, 89, 182);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_MAGICCRASHER2
// ---------------------------------------------------------------------------

/// `case EF_MAGICCRASHER2:` - `RagEffect.cpp:23773-23781`.
///
/// Frame 25 plays `effect\swordman_provoke.wav` (after the official
/// `render.position = parent->render.position` copy, a no-op here). Frames
/// 0-60 call `SetColor(-1, max(rand() % 255, 0), ...)` - three
/// `ClampedRandom(255)` draws per frame in the written red/green/blue order.
/// Unlike `EF_MAGICCRASHER`, the block never requests a caster flag.
#[derive(Debug, Default)]
pub struct MagicCrasher2;

impl MagicCrasher2 {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for MagicCrasher2 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:412` - `result[EF_MAGICCRASHER2] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 25 {
            runtime.play_sound("effect\\swordman_provoke.wav");
        }

        if (0..=60).contains(&frame) {
            // `std::max(rand() % 255, 0)` is `ClampedRandom(255)`
            // (`RagEffect.cpp:168-171`): the modulo result is never negative,
            // so the clamp is the identity and the helper is exact.
            let red = runtime.rng.clamped_random(255) as i16;
            let green = runtime.rng.clamped_random(255) as i16;
            let blue = runtime.rng.clamped_random(255) as i16;

            runtime.set_parent_color(-1, red, green, blue);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_CHEMICALBODY
// ---------------------------------------------------------------------------

/// `case EF_CHEMICALBODY:` - `RagEffect.cpp:21703-21717`.
///
/// While `stateCnt < 31`: every frame requests flag 8; even frames also request
/// flag 1 and tint the caster `(0, 0, 255)`; odd frames only tint white. From
/// frame 31 on the block does nothing (the last tint persists).
#[derive(Debug, Default)]
pub struct ChemicalBody;

impl ChemicalBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for ChemicalBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:509` - `result[EF_CHEMICALBODY] = 120`.
        120
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame < 31 {
            runtime.add_parent_flags(8);

            if frame % 2 == 0 {
                runtime.add_parent_flags(1);
                runtime.set_parent_color(-1, 0, 0, 255);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_DOUBLECASTBODY
// ---------------------------------------------------------------------------

/// `case EF_DOUBLECASTBODY:` - `RagEffect.cpp:21462-21476`.
///
/// Frames 10-50: request flag 8; even frames also request flag 1 and tint the
/// caster `(255, 0, 0)`; odd frames only tint white. Nothing outside the
/// window, so frames 0-9 and 51+ leave the last tint in place.
#[derive(Debug, Default)]
pub struct DoubleCastBody;

impl DoubleCastBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for DoubleCastBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:530` - `result[EF_DOUBLECASTBODY] = 120`.
        120
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if (10..=50).contains(&frame) {
            runtime.add_parent_flags(8);

            if frame % 2 == 0 {
                runtime.add_parent_flags(1);
                runtime.set_parent_color(-1, 255, 0, 0);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_SHRINK
// ---------------------------------------------------------------------------

/// `case EF_SHRINK:` - `RagEffect.cpp:21536-21550`.
///
/// Frame 5 plays `effect\EF_BeginSpell.wav` (`PlayEffectWave`). Frames 5-25
/// alternate on parity: even frames request flag 1 and tint `(250, 250, 100)`;
/// odd frames only tint white. Frame 5 is odd, so the sound frame only tints
/// white - the first flagged frame is 6.
#[derive(Debug, Default)]
pub struct Shrink;

impl Shrink {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for Shrink {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:608` - `result[EF_SHRINK] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 5 {
            runtime.play_sound("effect\\EF_BeginSpell.wav");
        }

        if (5..=25).contains(&frame) {
            if frame % 2 == 0 {
                runtime.add_parent_flags(1);
                runtime.set_parent_color(-1, 250, 250, 100);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_FALCONASSAULT
// ---------------------------------------------------------------------------

/// `case EF_FALCONASSAULT:` - `RagEffect.cpp:23782-23797`.
///
/// Frame 30 sends one `MM_QUAKE`. Frames 30-54 add 30 degrees to the caster's
/// facing with the original's single `> 360.0` subtraction (exactly `360.0` is
/// kept, `390.0` becomes `30.0`). Frames 30-50 additionally request flag 1
/// only while `stateCnt % 10 < 5`, i.e. 30-34, 40-44 and 50 - never 35-39 or
/// 45-49.
#[derive(Debug, Default)]
pub struct FalconAssault;

impl FalconAssault {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for FalconAssault {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:396` - `result[EF_FALCONASSAULT] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 30 {
            runtime.quake();
        }

        if (30..=54).contains(&frame) {
            runtime.parent_angle += 30.0;

            if runtime.parent_angle > 360.0 {
                runtime.parent_angle -= 360.0;
            }
        }

        if (30..=50).contains(&frame) && frame % 10 < 5 {
            runtime.add_parent_flags(1);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_SU_STOOP
// ---------------------------------------------------------------------------

/// `case EF_SU_STOOP:` - `RagEffect.cpp:22202-22207`.
///
/// Frame 0 copies the caster position (the port's spawn anchor already is that
/// position, so the copy is a no-op here) and plays
/// `effect\mon_금강불괴.wav` through `PlayEffectWave`. Every later frame is a
/// no-op: no flags, no colour, no primitive, no RNG. Lifetime 99999999
/// (`EffectTypes.cpp:1141`), so the effect only ends when the server deletes
/// it; the body always returns `true`.
#[derive(Debug, Default)]
pub struct SuStoop;

impl SuStoop {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for SuStoop {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:1141` - `result[EF_SU_STOOP] = 99999999`.
        99999999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            // `this->render.position = this->state.parent->render.position;`
            // is a no-op in the hermetic runtime: `position` already is the
            // caster anchor filled by the spawn path.
            runtime.play_sound("effect\\mon_금강불괴.wav");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};
    use ragnarok_packets::EffectId;

    use super::super::runtime::{EffectRng, EffectRuntime, run_to_completion, step};
    use super::*;

    fn runtime_with(seed: u64, lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], seed)
    }

    #[test]
    fn batch_i_lifetimes_match_effect_types() {
        // `EffectTypes.cpp:377`, `:384`, `:428`, `:405`, `:412`, `:509`,
        // `:530`, `:608`, `:396` and `:1141`.
        assert_eq!(RedBody::new().lifetime(), 99999);
        assert_eq!(Assumptio::new().lifetime(), 99999);
        assert_eq!(ReflectBody::new().lifetime(), 99999999);
        assert_eq!(PinkBody::new().lifetime(), 99999999);
        assert_eq!(MagicCrasher2::new().lifetime(), 100);
        assert_eq!(ChemicalBody::new().lifetime(), 120);
        assert_eq!(DoubleCastBody::new().lifetime(), 120);
        assert_eq!(Shrink::new().lifetime(), 200);
        assert_eq!(FalconAssault::new().lifetime(), 200);
        assert_eq!(SuStoop::new().lifetime(), 99999999);
    }

    #[test]
    fn sustoop_id_keeps_its_official_numeric_value() {
        // `EffectTypes.h:1139-1141` - `EF_SU_STOOP = 1132`, between
        // `EF_SU_CHATTERING` (1124) and `EF_FIREDANCE` (1133). `22202` is the
        // decompiled case's line number, never an id. Pinned through the
        // `ByteConvertable` wire encoding (`#[numeric_value]` does not change
        // the Rust discriminant) so no later insertion can shift it.
        fn encoded(effect: EffectId) -> [u8; 4] {
            let mut writer = ByteWriter::new();
            effect.to_bytes(&mut writer).expect("EffectId encodes");

            writer.as_slice().try_into().expect("EffectId is a u32")
        }

        assert_eq!(encoded(EffectId::SuStoop), 1132u32.to_le_bytes());
        assert_eq!(encoded(EffectId::SuChattering), 1124u32.to_le_bytes());
        assert_eq!(encoded(EffectId::Firedance), 1133u32.to_le_bytes());

        let bytes = 1132u32.to_le_bytes();
        let decoded = EffectId::from_bytes(&mut ByteReader::without_metadata(&bytes)).expect("1132 is a known id");
        assert!(matches!(decoded, EffectId::SuStoop), "1132 must decode to SuStoop");
    }

    // -- EF_REDBODY --------------------------------------------------------

    #[test]
    fn redbody_tints_and_flags_every_frame() {
        let mut effect = RedBody::new();
        let mut runtime = runtime_with(1, 99999);

        for frame in 0..3 {
            step(&mut effect, &mut runtime);

            assert_eq!(runtime.frame, frame + 1);
            assert_eq!(runtime.parent_flags, 1, "frame {frame}");
            assert_eq!(runtime.parent_color, Some([-1, 255, 100, 100]), "frame {frame}");
        }

        // Caster-body only: no geometry, no sound, no shake, no RNG.
        assert!(runtime.primitives.is_empty());
        assert!(runtime.sounds.is_empty());
        assert_eq!(runtime.shakes, 0);
        let mut reference = EffectRng::new(1);
        assert_eq!(runtime.rng.clamped_random(1000), reference.clamped_random(1000));
    }

    // -- EF_ASSUMPTIO ------------------------------------------------------

    #[test]
    fn assumptio_requests_the_pulse_flag_every_frame() {
        let mut effect = Assumptio::new();
        let mut runtime = runtime_with(2, 99999);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_flags, 8);
        assert_eq!(runtime.parent_color, None, "SetColor is never called");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_flags, 8);
        assert_eq!(runtime.parent_color, None);

        assert!(runtime.primitives.is_empty());
        assert!(runtime.sounds.is_empty());
    }

    // -- EF_REFLECTBODY ----------------------------------------------------

    #[test]
    fn reflectbody_requests_flag_0x200_every_frame() {
        let mut effect = ReflectBody::new();
        let mut runtime = runtime_with(4, 99999999);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_flags, 0x200);
        assert_eq!(runtime.parent_color, None);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_flags, 0x200);
        assert_eq!(runtime.parent_color, None);
    }

    // -- EF_PINKBODY -------------------------------------------------------

    #[test]
    fn pinkbody_requests_both_flags_and_pink_tint() {
        let mut effect = PinkBody::new();
        let mut runtime = runtime_with(6, 99999999);

        step(&mut effect, &mut runtime);
        // Bit 1 `+= 1` then bit 8 `+= 8`: the recorded word is 9.
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 255, 89, 182]));

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 255, 89, 182]));

        assert!(runtime.sounds.is_empty());
    }

    // -- EF_MAGICCRASHER2 --------------------------------------------------

    #[test]
    fn magiccrasher2_plays_the_wave_on_25_and_tints_from_frame_zero() {
        let mut effect = MagicCrasher2::new();
        let mut runtime = runtime_with(8123, 100);
        let mut reference = EffectRng::new(8123);

        // Frame 0 is already tinted: the window is `>= 0 && <= 60`.
        let expected = [
            reference.clamped_random(255) as i16,
            reference.clamped_random(255) as i16,
            reference.clamped_random(255) as i16,
        ];
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, expected[0], expected[1], expected[2]]));
        assert!(runtime.sounds.is_empty());
        assert_eq!(runtime.parent_flags, 0, "the block never requests a flag");

        // Frames 1..=24 tint without a sound (24 frames * 3 draws).
        for _ in 0..24 {
            reference.clamped_random(255);
            reference.clamped_random(255);
            reference.clamped_random(255);
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 25);
        assert!(runtime.sounds.is_empty());

        let expected = [
            reference.clamped_random(255) as i16,
            reference.clamped_random(255) as i16,
            reference.clamped_random(255) as i16,
        ];
        step(&mut effect, &mut runtime); // frame 25
        assert_eq!(runtime.sounds, vec!["effect\\swordman_provoke.wav".to_owned()]);
        assert_eq!(runtime.parent_color, Some([-1, expected[0], expected[1], expected[2]]));
    }

    #[test]
    fn magiccrasher2_consumes_three_draws_for_frames_0_to_60_only() {
        let mut effect = MagicCrasher2::new();
        let mut runtime = runtime_with(77, 100);

        while runtime.frame < 100 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 100);
        assert_eq!(runtime.shakes, 0);

        // 61 tinted frames * 3 draws = 183 draws; frame 61 and later draw none.
        let mut reference = EffectRng::new(77);
        for _ in 0..(61 * 3) {
            reference.clamped_random(255);
        }
        assert_eq!(runtime.rng.clamped_random(1000), reference.clamped_random(1000));
    }

    #[test]
    fn magiccrasher2_retires_after_100_frames() {
        let mut effect = MagicCrasher2::new();
        let mut runtime = runtime_with(9, 100);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 100);
        assert_eq!(snapshots.last().unwrap().frame, 100);
    }

    // -- EF_CHEMICALBODY ---------------------------------------------------

    #[test]
    fn chemicalbody_alternates_blue_and_white_until_frame_30() {
        let mut effect = ChemicalBody::new();
        let mut runtime = runtime_with(11, 120);

        step(&mut effect, &mut runtime); // frame 0, even
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 0, 0, 255]));

        step(&mut effect, &mut runtime); // frame 1, odd
        assert_eq!(runtime.parent_flags, 0b1000);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        step(&mut effect, &mut runtime); // frame 2, even again
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 0, 0, 255]));

        // Frames 3..=29, ending right before the last processed frame (30).
        while runtime.frame < 30 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 30);

        step(&mut effect, &mut runtime); // frame 30, even - last active frame
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 0, 0, 255]));

        step(&mut effect, &mut runtime); // frame 31, outside `stateCnt < 31`
        assert_eq!(runtime.parent_flags, 0);
        // `SetColor` persists; only the flag request is per-frame.
        assert_eq!(runtime.parent_color, Some([-1, 0, 0, 255]));

        // Finish the 120-frame lifetime: nothing changes after frame 31.
        while runtime.frame < 120 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 120);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 0, 0, 255]));
        assert!(runtime.sounds.is_empty());
    }

    // -- EF_DOUBLECASTBODY -------------------------------------------------

    #[test]
    fn doublecastbody_tints_from_10_to_50() {
        let mut effect = DoubleCastBody::new();
        let mut runtime = runtime_with(12, 120);

        // Frames 0..=9: nothing.
        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 10);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, None);

        step(&mut effect, &mut runtime); // frame 10, even
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 255, 0, 0]));

        step(&mut effect, &mut runtime); // frame 11, odd
        assert_eq!(runtime.parent_flags, 0b1000);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        // Process frames 12..=49 (frame 49 is odd -> white, flag 8).
        while runtime.frame < 50 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 50);

        step(&mut effect, &mut runtime); // frame 50, even - last active frame
        assert_eq!(runtime.parent_flags, 0b1001);
        assert_eq!(runtime.parent_color, Some([-1, 255, 0, 0]));

        step(&mut effect, &mut runtime); // frame 51, outside the window
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 0, 0]));
    }

    // -- EF_SHRINK ---------------------------------------------------------

    #[test]
    fn shrink_plays_the_wave_on_5_and_alternates_to_25() {
        let mut effect = Shrink::new();
        let mut runtime = runtime_with(13, 200);

        // Frames 0..=4: nothing.
        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 5);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, None);
        assert!(runtime.sounds.is_empty());

        step(&mut effect, &mut runtime); // frame 5, odd - wave + white only
        assert_eq!(runtime.sounds, vec!["effect\\EF_BeginSpell.wav".to_owned()]);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        step(&mut effect, &mut runtime); // frame 6, even - first flagged frame
        assert_eq!(runtime.parent_flags, 1);
        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 100]));

        // Process frames 7..=24 (frame 24 is even -> tinted + flag).
        while runtime.frame < 25 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 25);
        assert_eq!(runtime.parent_flags, 1, "frame 24 was the last even frame");
        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 100]));

        step(&mut effect, &mut runtime); // frame 25, odd - last window frame
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        step(&mut effect, &mut runtime); // frame 26, outside the window
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
        assert_eq!(runtime.sounds.len(), 1);
    }

    // -- EF_FALCONASSAULT --------------------------------------------------

    #[test]
    fn falconassault_quakes_on_30_and_rotates_the_caster_to_54() {
        let mut effect = FalconAssault::new();
        let mut runtime = runtime_with(14, 200);

        // Frames 0..=29: no quake, no rotation, no flag.
        for _ in 0..30 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 30);
        assert_eq!(runtime.shakes, 0);
        assert_eq!(runtime.parent_angle, 0.0);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // frame 30
        assert_eq!(runtime.shakes, 1);
        assert_eq!(runtime.parent_angle, 30.0);
        assert_eq!(runtime.parent_flags, 1);

        // Frames 31..=34: rotation and flag 1 (all `% 10 < 5`).
        step(&mut effect, &mut runtime); // 31
        assert_eq!(runtime.parent_angle, 60.0);
        assert_eq!(runtime.parent_flags, 1);
        step(&mut effect, &mut runtime); // 32
        assert_eq!(runtime.parent_angle, 90.0);
        assert_eq!(runtime.parent_flags, 1);
        step(&mut effect, &mut runtime); // 33
        assert_eq!(runtime.parent_angle, 120.0);
        assert_eq!(runtime.parent_flags, 1);
        step(&mut effect, &mut runtime); // 34
        assert_eq!(runtime.parent_angle, 150.0);
        assert_eq!(runtime.parent_flags, 1);

        // Frame 35 keeps rotating but stops requesting the flag (35 % 10 == 5).
        step(&mut effect, &mut runtime); // 35
        assert_eq!(runtime.parent_angle, 180.0);
        assert_eq!(runtime.parent_flags, 0);

        // Frames 36..=39: rotation only.
        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 40);
        assert_eq!(runtime.parent_angle, 300.0);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // 40, `% 10 == 0`
        assert_eq!(runtime.parent_angle, 330.0);
        assert_eq!(runtime.parent_flags, 1);

        // The original's `>` comparison: exactly 360 is kept, 390 wraps to 30.
        step(&mut effect, &mut runtime); // 41
        assert_eq!(runtime.parent_angle, 360.0);
        assert_eq!(runtime.parent_flags, 1);
        step(&mut effect, &mut runtime); // 42
        assert_eq!(runtime.parent_angle, 30.0);
        assert_eq!(runtime.parent_flags, 1);

        // Frames 43..=44 still flag; frame 45 stops (45 % 10 == 5).
        step(&mut effect, &mut runtime); // 43
        assert_eq!(runtime.parent_angle, 60.0);
        assert_eq!(runtime.parent_flags, 1);
        step(&mut effect, &mut runtime); // 44
        assert_eq!(runtime.parent_angle, 90.0);
        assert_eq!(runtime.parent_flags, 1);
        step(&mut effect, &mut runtime); // 45
        assert_eq!(runtime.parent_angle, 120.0);
        assert_eq!(runtime.parent_flags, 0);

        // Frames 46..=49 rotate without the flag.
        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 50);
        assert_eq!(runtime.parent_angle, 240.0);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // 50, `% 10 == 0`
        assert_eq!(runtime.parent_angle, 270.0);
        assert_eq!(runtime.parent_flags, 1);

        // Frames 51..=53 rotate only; frame 54 is the last rotation frame.
        for _ in 0..3 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 54);
        assert_eq!(runtime.parent_angle, 360.0);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // 54
        assert_eq!(runtime.parent_angle, 30.0);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // 55, outside the rotation window
        assert_eq!(runtime.parent_angle, 30.0);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.shakes, 1);
        assert!(runtime.sounds.is_empty());
    }

    #[test]
    fn falconassault_retires_after_200_frames_without_further_quakes() {
        let mut effect = FalconAssault::new();
        let mut runtime = runtime_with(15, 200);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 200);
        assert_eq!(runtime.shakes, 1);
        assert_eq!(runtime.parent_angle, 30.0);
    }

    // -- EF_SU_STOOP -------------------------------------------------------

    #[test]
    fn sustoop_plays_the_wave_on_frame_zero_only() {
        let mut effect = SuStoop::new();
        let mut runtime = runtime_with(16, 99999999);

        step(&mut effect, &mut runtime); // frame 0
        assert_eq!(runtime.sounds, vec!["effect\\mon_금강불괴.wav".to_owned()]);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, None, "SetColor is never called");
        assert!(runtime.primitives.is_empty());

        // The buffer is cumulative here (the holder drains it per step): no
        // later frame adds a second sound.
        for _ in 0..120 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 121);
        assert_eq!(runtime.sounds.len(), 1);
        assert_eq!(runtime.sounds[0], "effect\\mon_금강불괴.wav");
        assert_eq!(runtime.shakes, 0);
        assert!(runtime.primitives.is_empty());
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, None);

        // No RNG draws: a fresh generator is still in the same state.
        let mut reference = EffectRng::new(16);
        assert_eq!(runtime.rng.clamped_random(1000), reference.clamped_random(1000));
    }

    #[test]
    fn sustoop_stays_alive_until_its_huge_lifetime() {
        let mut effect = SuStoop::new();
        let mut runtime = runtime_with(17, 99999999);

        // 500 frames is far short of the 99999999-frame lifetime, so every
        // step reports alive and the frame counter reaches 500.
        for _ in 0..500 {
            assert!(step(&mut effect, &mut runtime));
        }
        assert_eq!(runtime.frame, 500);
        assert_eq!(runtime.sounds.len(), 1);
    }
}
