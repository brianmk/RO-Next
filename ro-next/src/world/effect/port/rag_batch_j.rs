//! Effects ported by the "j" parallel batch - the actor-render-field remainder
//! of the manual (T3) tier.
//!
//! Every body below is a direct transliteration of one decompiled
//! `CRagEffect::OnProcess` dispatch block that writes the *caster actor's*
//! render bytes instead of launching a primitive or opening a `.str` clip.
//! The fields are the ones `CRenderObject::Reset` initialises
//! (`RenderObjectState.cpp:75-93`) and nothing clears per frame:
//!
//! * `render.effectPhase` (`RenderObjectState.h:127`) -
//!   [`EffectRuntime::set_parent_effect_phase`].
//! * `render.tintPhase` (`:128`) - [`EffectRuntime::set_parent_tint_phase`].
//! * `render.shrinkPhase` (`:121`) -
//!   [`EffectRuntime::set_parent_shrink_phase`].
//! * `render.outlineAlpha` (`:125`) -
//!   [`EffectRuntime::set_parent_outline_alpha`] (already introduced by
//!   `EF_ASURABODY`; no body in this batch writes it).
//! * `render.runStartTick` (`:129`) -
//!   [`EffectRuntime::set_parent_run_start_tick`].
//! * `NotifyGameMode` (`RagEffect.cpp:8672-8676`) -
//!   [`EffectRuntime::notify_game_mode`].
//!
//! The `if ((render.flags & bit) == 0) { render.flags += bit; ... }` guard runs
//! every frame in the original because the actor clears `render.flags` after
//! every drawn frame (`GameObject.cpp:435`); [`EffectRuntime::begin_step`]
//! clears the recorded word the same way, so the write happens on every frame
//! that passes the body's own frame gate, exactly as in the decompilation.
//! The phase fields themselves are not cleared per frame - only
//! `CRenderObject::Reset` resets them - so the runtime keeps the last write and
//! the visual bridge re-applies it until the effect retires.
//!
//! `this->render.position = this->state.parent->render.position` is a no-op in
//! the hermetic runtime (its [`EffectRuntime::position`] already is the master
//! anchor the spawn path fills from the parent), so the bodies that make that
//! copy only document it. `PlayEffectWave` and `PlayEffectWaveInline` are the
//! same helper in this decompilation (`RagEffect.cpp:152-165`), so both are
//! recorded through [`EffectRuntime::play_sound`].
//!
//! `EF_QUAKEBODY` is included alongside `EF_QUAKEBODY2`: the decompiled
//! `case EF_QUAKEBODY:` falls through to `case EF_QUAKEBODY2:` and shares its
//! body verbatim (`RagEffect.cpp:24534-24541`), and neither id was ported
//! before. Its lifetime is 14 frames (`EffectTypes.cpp:435`).

#![allow(dead_code)]

use super::runtime::{EffectRuntime, PortedEffect};

// ---------------------------------------------------------------------------
// EF_PRESSEDBODY
// ---------------------------------------------------------------------------

/// `case EF_PRESSEDBODY:` - `RagEffect.cpp:24452-24461`.
///
/// Frame 5 copies the caster position (a no-op here) and plays
/// `effect\T_내려찍기.wav`. Frames 17-50 request caster flag `0x20` and write
/// `effectPhase = stateCnt - 17` (0..33). Lifetime 50, so the phase is
/// overwritten on every frame of that window and then persists.
#[derive(Debug, Default)]
pub struct PressedBody;

impl PressedBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for PressedBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:422` - `result[EF_PRESSEDBODY] = 50`.
        50
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 5 {
            // `this->render.position = this->state.parent->render.position;`
            // is a no-op: `position` already is the caster anchor.
            runtime.play_sound("effect\\T_내려찍기.wav");
        }

        if runtime.parent_flags & 0x20 == 0 && (17..=50).contains(&frame) {
            runtime.add_parent_flags(0x20);
            runtime.set_parent_effect_phase((frame - 17) as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_KICKEDBODY
// ---------------------------------------------------------------------------

/// `case EF_KICKEDBODY:` - `RagEffect.cpp:24577-24588`.
///
/// Frame 17 plays `effect\EF_hit2.wav`; frame 25 plays `effect\EF_hit4.wav`
/// (each preceded by the no-op position copy). From frame 30 on, every frame
/// requests caster flag `0x80` and writes `effectPhase = stateCnt - 30`.
/// Lifetime 100, so the last write is `70`.
#[derive(Debug, Default)]
pub struct KickedBody;

impl KickedBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for KickedBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:424` - `result[EF_KICKEDBODY] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 17 {
            runtime.play_sound("effect\\EF_hit2.wav");
        }

        if frame == 25 {
            runtime.play_sound("effect\\EF_hit4.wav");
        }

        if runtime.parent_flags & 0x80 == 0 && frame >= 30 {
            runtime.add_parent_flags(0x80);
            runtime.set_parent_effect_phase((frame - 30) as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_4WAYBODY
// ---------------------------------------------------------------------------

/// `case EF_4WAYBODY:` - `RagEffect.cpp:24528-24533`.
///
/// From frame 30 on, every frame ORs `0x1000` into the caster flags (the body
/// uses `|=`, not the usual guarded `+=`) and writes
/// `effectPhase = stateCnt - 30`. Lifetime 100.
#[derive(Debug, Default)]
pub struct FourWayBody;

impl FourWayBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for FourWayBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:434` - `result[EF_4WAYBODY] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame >= 30 {
            runtime.add_parent_flags(0x1000);
            runtime.set_parent_effect_phase((runtime.frame - 30) as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_QUAKEBODY / EF_QUAKEBODY2
// ---------------------------------------------------------------------------

/// `case EF_QUAKEBODY: case EF_QUAKEBODY2:` - `RagEffect.cpp:24534-24541`.
///
/// No frame gate: every frame requests caster flag `0x2000` and writes
/// `effectPhase = stateCnt`. Shared by two ids with different lifetimes.
#[derive(Debug, Default)]
pub struct QuakeBody {
    effect_id: Option<ragnarok_packets::EffectId>,
}

impl QuakeBody {
    pub fn new(effect_id: ragnarok_packets::EffectId) -> Self {
        Self {
            effect_id: Some(effect_id),
        }
    }
}

impl PortedEffect for QuakeBody {
    fn lifetime(&self) -> i32 {
        match self.effect_id {
            // `EffectTypes.cpp:435` - `result[EF_QUAKEBODY] = 14`.
            Some(ragnarok_packets::EffectId::Quakebody) => 14,
            // `EffectTypes.cpp:467` - `result[EF_QUAKEBODY2] = 35`.
            _ => 35,
        }
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x2000 == 0 {
            runtime.add_parent_flags(0x2000);
            runtime.set_parent_effect_phase(runtime.frame as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_QUAKEBODY3
// ---------------------------------------------------------------------------

/// `case EF_QUAKEBODY3:` - `RagEffect.cpp:21760-21765`.
///
/// Only frames 31-50 touch the caster: each ORs flag `0x2000` and writes
/// `effectPhase = stateCnt`. Frames outside the window leave the previous
/// value alone. Lifetime 60.
#[derive(Debug, Default)]
pub struct QuakeBody3;

impl QuakeBody3 {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for QuakeBody3 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:559` - `result[EF_QUAKEBODY3] = 60`.
        60
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if (31..=50).contains(&runtime.frame) {
            runtime.add_parent_flags(0x2000);
            runtime.set_parent_effect_phase(runtime.frame as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_QUAKEBODY4
// ---------------------------------------------------------------------------

/// `case EF_QUAKEBODY4:` - `RagEffect.cpp:21766-21780`.
///
/// Frame 20 copies the caster position (no-op) and plays
/// `effect\ef_hit6.wav`. From frame 21 on, every frame ORs flag `0x2000`,
/// writes `effectPhase = stateCnt` and alternates `SetColor(-1, 250, 50, 50)`
/// on even frames with `SetColor(-1, 255, 255, 255)` on odd frames. Frame 21
/// is the first coloured frame and is odd, so it tints white. Lifetime 60.
#[derive(Debug, Default)]
pub struct QuakeBody4;

impl QuakeBody4 {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for QuakeBody4 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:611` - `result[EF_QUAKEBODY4] = 60`.
        60
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 20 {
            runtime.play_sound("effect\\ef_hit6.wav");
        }

        if frame > 20 {
            runtime.add_parent_flags(0x2000);
            runtime.set_parent_effect_phase(frame as i16);

            if frame % 2 == 0 {
                runtime.set_parent_color(-1, 250, 50, 50);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_LANDBODY
// ---------------------------------------------------------------------------

/// `case EF_LANDBODY:` - `RagEffect.cpp:24551-24560`.
///
/// The first guard requests caster flag `0x20000` and writes
/// `tintPhase = stateCnt` on every frame. The second requests flag `0x40` and
/// writes `effectPhase = stateCnt + 11` only while `stateCnt < 25` (11..35).
/// Lifetime 35, so the tint writes 0..34 and the phase freezes at 35.
#[derive(Debug, Default)]
pub struct LandBody;

impl LandBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for LandBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:455` - `result[EF_LANDBODY] = 35`.
        35
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if runtime.parent_flags & 0x20000 == 0 {
            runtime.add_parent_flags(0x20000);
            runtime.set_parent_tint_phase(frame as u8);
        }

        if runtime.parent_flags & 0x40 == 0 && frame < 25 {
            runtime.add_parent_flags(0x40);
            runtime.set_parent_effect_phase((frame + 11) as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_SPINEDBODY2
// ---------------------------------------------------------------------------

/// `case EF_SPINEDBODY2:` - `RagEffect.cpp:24561-24570`.
///
/// Frame 5 copies the caster position (no-op) and plays
/// `effect\T_돌려차기.wav`. Frames 14-49 request caster flag `0x40` and write
/// `effectPhase = stateCnt - 14` (0..35). Lifetime 50.
#[derive(Debug, Default)]
pub struct SpinedBody2;

impl SpinedBody2 {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for SpinedBody2 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:475` - `result[EF_SPINEDBODY2] = 50`.
        50
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 5 {
            runtime.play_sound("effect\\T_돌려차기.wav");
        }

        if runtime.parent_flags & 0x40 == 0 && (14..=49).contains(&frame) {
            runtime.add_parent_flags(0x40);
            runtime.set_parent_effect_phase((frame - 14) as i16);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_BLUEBODY
// ---------------------------------------------------------------------------

/// `case EF_BLUEBODY:` - `RagEffect.cpp:21312-21317`.
///
/// While `stateCnt <= 150`, every frame requests caster flag `0x4000` and
/// writes `tintPhase = stateCnt / 3` (integer division, 0..50). After frame
/// 150 the guard fails and the last value stays. Lifetime 200.
#[derive(Debug, Default)]
pub struct BlueBody;

impl BlueBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for BlueBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:551` - `result[EF_BLUEBODY] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x4000 == 0 && runtime.frame <= 150 {
            runtime.add_parent_flags(0x4000);
            runtime.set_parent_tint_phase((runtime.frame / 3) as u8);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_MADNESS_BLUE
// ---------------------------------------------------------------------------

/// `case EF_MADNESS_BLUE:` - `RagEffect.cpp:21318-21325`.
///
/// Every fourth frame requests caster flag `0x4000` and pins `tintPhase = 20`;
/// the other frames leave the field untouched. Lifetime 60.
#[derive(Debug, Default)]
pub struct MadnessBlue;

impl MadnessBlue {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for MadnessBlue {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:634` - `result[EF_MADNESS_BLUE] = 60`.
        60
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame % 4 == 0 && runtime.parent_flags & 0x4000 == 0 {
            runtime.add_parent_flags(0x4000);
            runtime.set_parent_tint_phase(20);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_MADNESS_RED
// ---------------------------------------------------------------------------

/// `case EF_MADNESS_RED:` - `RagEffect.cpp:21326-21333`.
///
/// Every fourth frame requests caster flag `0x40000` and pins
/// `tintPhase = 20`. Lifetime 60.
#[derive(Debug, Default)]
pub struct MadnessRed;

impl MadnessRed {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for MadnessRed {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:635` - `result[EF_MADNESS_RED] = 60`.
        60
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame % 4 == 0 && runtime.parent_flags & 0x40000 == 0 {
            runtime.add_parent_flags(0x40000);
            runtime.set_parent_tint_phase(20);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_RED_HIT
// ---------------------------------------------------------------------------

/// `case EF_RED_HIT:` - `RagEffect.cpp:21781-21787`.
///
/// While `stateCnt <= 16`, every frame ORs caster flag `0x40000` and writes
/// `tintPhase = (unsigned char)(stateCnt * 3)` (0..48). Lifetime 60, so the
/// value then persists.
#[derive(Debug, Default)]
pub struct RedHit;

impl RedHit {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for RedHit {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:557` - `result[EF_RED_HIT] = 60`.
        60
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame <= 16 {
            runtime.add_parent_flags(0x40000);
            runtime.set_parent_tint_phase((runtime.frame * 3) as u8);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_REDLIGHTBODY
// ---------------------------------------------------------------------------

/// `case EF_REDLIGHTBODY: case EF_UNK_1398:` - `RagEffect.cpp:21793-21803`.
///
/// Every frame requests caster flag `0x40000` and writes
/// `tintPhase = (unsigned char)((float)stateCnt * 0.125f)` while
/// `stateCnt <= 200`, then pins `25`. The float multiply is kept verbatim:
/// frame 8 gives exactly `1.0`, frame 199 truncates `24.875` to `24`, frame 200
/// gives `25`, and frame 201 takes the `else` branch (also `25`). Lifetime
/// 99999999.
#[derive(Debug, Default)]
pub struct RedLightBody;

impl RedLightBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for RedLightBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:553` - `result[EF_REDLIGHTBODY] = 99999999`.
        99999999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x40000 == 0 {
            runtime.add_parent_flags(0x40000);

            if runtime.frame <= 200 {
                runtime.set_parent_tint_phase((runtime.frame as f32 * 0.125f32) as u8);
            } else {
                runtime.set_parent_tint_phase(25);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_BABYBODY
// ---------------------------------------------------------------------------

/// `case EF_BABYBODY:` - `RagEffect.cpp:24478-24485`.
///
/// Every frame requests caster flag `0x10` and, while `stateCnt <= 45`,
/// writes `shrinkPhase = stateCnt` (0..45). Lifetime 99999, so the value then
/// persists.
#[derive(Debug, Default)]
pub struct BabyBody;

impl BabyBody {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for BabyBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:429` - `result[EF_BABYBODY] = 99999`.
        99999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x10 == 0 {
            runtime.add_parent_flags(0x10);

            if runtime.frame <= 45 {
                runtime.set_parent_shrink_phase(runtime.frame as u8);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_RUN
// ---------------------------------------------------------------------------

/// `case EF_RUN:` - `RagEffect.cpp:23621-23627`.
///
/// Every frame requests caster flag `0x8000` and writes
/// `runStartTick = (unsigned char)stateCnt`. The byte wraps after 255, so
/// frame 256 writes `0` again. Lifetime 9999.
#[derive(Debug, Default)]
pub struct Run;

impl Run {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for Run {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:451` - `result[EF_RUN] = 9999`.
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x8000 == 0 {
            runtime.add_parent_flags(0x8000);
            runtime.set_parent_run_start_tick(runtime.frame as u8);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_TALK_FROSTJOKE
// ---------------------------------------------------------------------------

/// `case EF_TALK_FROSTJOKE:` - `RagEffect.cpp:24052-24054`.
///
/// The body calls `NotifyGameMode(1)` (`RagEffect.cpp:8672-8676`), which sends
/// `MM_SONG_TALK` only on `stateCnt == 0`; the mode then reads
/// `BA_frostjoke.txt`, picks a line and sends it as a "talk" chat packet.
/// Lifetime 999; the port records the one-shot request.
#[derive(Debug, Default)]
pub struct TalkFrostJoke;

impl TalkFrostJoke {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for TalkFrostJoke {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:304` - `result[EF_TALK_FROSTJOKE] = 999`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.notify_game_mode(1);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_TALK_SCREAM
// ---------------------------------------------------------------------------

/// `case EF_TALK_SCREAM:` - `RagEffect.cpp:24055-24057`.
///
/// `NotifyGameMode(0)` on frame 0: `MM_SONG_TALK` selects `DC_scream.txt`.
/// Lifetime 999; the port records the one-shot request.
#[derive(Debug, Default)]
pub struct TalkScream;

impl TalkScream {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for TalkScream {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:305` - `result[EF_TALK_SCREAM] = 999`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.notify_game_mode(0);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::EffectId;

    use super::super::runtime::{EffectRng, EffectRuntime, PortedEffect, step};
    use super::*;

    fn runtime_with(lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], 1)
    }

    /// Runs the body exactly at `frame` with the per-frame flag word cleared,
    /// like one `step` without the retirement check.
    fn process_at(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime, frame: i32) {
        runtime.frame = frame;
        runtime.begin_step();
        assert!(effect.on_process(runtime), "frame {frame}");
    }

    #[test]
    fn batch_j_lifetimes_match_effect_types() {
        // `EffectTypes.cpp:422`, `:424`, `:434`, `:435`, `:467`, `:559`,
        // `:611`, `:455`, `:475`, `:551`, `:553`, `:557`, `:634`, `:635`,
        // `:429`, `:451`, `:304` and `:305`.
        assert_eq!(PressedBody::new().lifetime(), 50);
        assert_eq!(KickedBody::new().lifetime(), 100);
        assert_eq!(FourWayBody::new().lifetime(), 100);
        assert_eq!(QuakeBody::new(EffectId::Quakebody).lifetime(), 14);
        assert_eq!(QuakeBody::new(EffectId::Quakebody2).lifetime(), 35);
        assert_eq!(QuakeBody3::new().lifetime(), 60);
        assert_eq!(QuakeBody4::new().lifetime(), 60);
        assert_eq!(LandBody::new().lifetime(), 35);
        assert_eq!(SpinedBody2::new().lifetime(), 50);
        assert_eq!(BlueBody::new().lifetime(), 200);
        assert_eq!(RedLightBody::new().lifetime(), 99999999);
        assert_eq!(RedHit::new().lifetime(), 60);
        assert_eq!(MadnessBlue::new().lifetime(), 60);
        assert_eq!(MadnessRed::new().lifetime(), 60);
        assert_eq!(BabyBody::new().lifetime(), 99999);
        assert_eq!(Run::new().lifetime(), 9999);
        assert_eq!(TalkFrostJoke::new().lifetime(), 999);
        assert_eq!(TalkScream::new().lifetime(), 999);
    }

    // -- EF_PRESSEDBODY ----------------------------------------------------

    #[test]
    fn pressedbody_sounds_on_5_and_phases_17_through_50() {
        let mut effect = PressedBody::new();
        let mut runtime = runtime_with(50);

        process_at(&mut effect, &mut runtime, 4);
        assert!(runtime.sounds.is_empty());
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 5);
        assert_eq!(runtime.sounds, vec!["effect\\T_내려찍기.wav".to_owned()]);
        assert_eq!(runtime.parent_flags, 0, "the sound frame requests no flag");
        assert_eq!(runtime.parent_effect_phase, None, "the phase window starts at 17");

        process_at(&mut effect, &mut runtime, 16);
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 17);
        assert_eq!(runtime.parent_flags, 0x20);
        assert_eq!(runtime.parent_effect_phase, Some(0));

        process_at(&mut effect, &mut runtime, 50);
        assert_eq!(runtime.parent_flags, 0x20);
        assert_eq!(runtime.parent_effect_phase, Some(33));

        // The window closes; the last phase persists (`CRenderObject::Reset`
        // is the only reset).
        process_at(&mut effect, &mut runtime, 51);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_effect_phase, Some(33));
        assert_eq!(runtime.sounds.len(), 1);
    }

    // -- EF_KICKEDBODY -----------------------------------------------------

    #[test]
    fn kickedbody_plays_two_hits_then_phases_from_30() {
        let mut effect = KickedBody::new();
        let mut runtime = runtime_with(100);

        process_at(&mut effect, &mut runtime, 17);
        assert_eq!(runtime.sounds, vec!["effect\\EF_hit2.wav".to_owned()]);
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 25);
        assert_eq!(runtime.sounds, vec![
            "effect\\EF_hit2.wav".to_owned(),
            "effect\\EF_hit4.wav".to_owned()
        ]);

        process_at(&mut effect, &mut runtime, 29);
        assert_eq!(runtime.parent_effect_phase, None, "the phase window starts at 30");
        assert_eq!(runtime.parent_flags, 0);

        process_at(&mut effect, &mut runtime, 30);
        assert_eq!(runtime.parent_flags, 0x80);
        assert_eq!(runtime.parent_effect_phase, Some(0));

        process_at(&mut effect, &mut runtime, 100);
        assert_eq!(runtime.parent_effect_phase, Some(70));
    }

    // -- EF_4WAYBODY -------------------------------------------------------

    #[test]
    fn fourwaybody_or_phases_from_30() {
        let mut effect = FourWayBody::new();
        let mut runtime = runtime_with(100);

        process_at(&mut effect, &mut runtime, 29);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 30);
        assert_eq!(runtime.parent_flags, 0x1000);
        assert_eq!(runtime.parent_effect_phase, Some(0));

        // `|=` on a word that `begin_step` cleared is still `0x1000`.
        process_at(&mut effect, &mut runtime, 31);
        assert_eq!(runtime.parent_flags, 0x1000);
        assert_eq!(runtime.parent_effect_phase, Some(1));

        process_at(&mut effect, &mut runtime, 99);
        assert_eq!(runtime.parent_effect_phase, Some(69));
    }

    // -- EF_QUAKEBODY / EF_QUAKEBODY2 -------------------------------------

    #[test]
    fn quakebody_and_quakebody2_write_every_frame_with_their_own_lifetime() {
        for (id, lifetime) in [(EffectId::Quakebody, 14), (EffectId::Quakebody2, 35)] {
            let mut effect = QuakeBody::new(id);
            let mut runtime = runtime_with(lifetime);

            for frame in 0..5 {
                process_at(&mut effect, &mut runtime, frame);

                assert_eq!(runtime.parent_flags, 0x2000, "{id:?} frame {frame}");
                assert_eq!(runtime.parent_effect_phase, Some(frame as i16), "{id:?} frame {frame}");
            }

            assert!(runtime.primitives.is_empty(), "{id:?}");
            assert!(runtime.sounds.is_empty(), "{id:?}");
        }
    }

    // -- EF_QUAKEBODY3 -----------------------------------------------------

    #[test]
    fn quakebody3_phases_only_31_through_50() {
        let mut effect = QuakeBody3::new();
        let mut runtime = runtime_with(60);

        process_at(&mut effect, &mut runtime, 30);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 31);
        assert_eq!(runtime.parent_flags, 0x2000);
        assert_eq!(runtime.parent_effect_phase, Some(31));

        process_at(&mut effect, &mut runtime, 50);
        assert_eq!(runtime.parent_effect_phase, Some(50));

        process_at(&mut effect, &mut runtime, 51);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_effect_phase, Some(50), "the last write persists");
    }

    // -- EF_QUAKEBODY4 -----------------------------------------------------

    #[test]
    fn quakebody4_sounds_on_20_then_phases_and_alternates_colour() {
        let mut effect = QuakeBody4::new();
        let mut runtime = runtime_with(60);

        process_at(&mut effect, &mut runtime, 20);
        assert_eq!(runtime.sounds, vec!["effect\\ef_hit6.wav".to_owned()]);
        assert_eq!(runtime.parent_effect_phase, None, "the phase starts at 21");
        assert_eq!(runtime.parent_color, None);

        // 21 is odd: white.
        process_at(&mut effect, &mut runtime, 21);
        assert_eq!(runtime.parent_flags, 0x2000);
        assert_eq!(runtime.parent_effect_phase, Some(21));
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        // 22 is even: red.
        process_at(&mut effect, &mut runtime, 22);
        assert_eq!(runtime.parent_effect_phase, Some(22));
        assert_eq!(runtime.parent_color, Some([-1, 250, 50, 50]));

        process_at(&mut effect, &mut runtime, 23);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
    }

    // -- EF_LANDBODY -------------------------------------------------------

    #[test]
    fn landbody_writes_tint_every_frame_and_phase_below_25() {
        let mut effect = LandBody::new();
        let mut runtime = runtime_with(35);

        process_at(&mut effect, &mut runtime, 0);
        // First guard `0x20000`, second guard `0x40`, in that order.
        assert_eq!(runtime.parent_flags, 0x20040);
        assert_eq!(runtime.parent_tint_phase, Some(0));
        assert_eq!(runtime.parent_effect_phase, Some(11));

        process_at(&mut effect, &mut runtime, 24);
        assert_eq!(runtime.parent_tint_phase, Some(24));
        assert_eq!(runtime.parent_effect_phase, Some(35));

        // Frame 25: the tint still writes, the phase guard is closed.
        process_at(&mut effect, &mut runtime, 25);
        assert_eq!(runtime.parent_flags, 0x20000);
        assert_eq!(runtime.parent_tint_phase, Some(25));
        assert_eq!(runtime.parent_effect_phase, Some(35));

        process_at(&mut effect, &mut runtime, 34);
        assert_eq!(runtime.parent_tint_phase, Some(34));
    }

    // -- EF_SPINEDBODY2 ----------------------------------------------------

    #[test]
    fn spinedbody2_sounds_on_5_and_phases_14_through_49() {
        let mut effect = SpinedBody2::new();
        let mut runtime = runtime_with(50);

        process_at(&mut effect, &mut runtime, 5);
        assert_eq!(runtime.sounds, vec!["effect\\T_돌려차기.wav".to_owned()]);
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 13);
        assert_eq!(runtime.parent_effect_phase, None);

        process_at(&mut effect, &mut runtime, 14);
        assert_eq!(runtime.parent_flags, 0x40);
        assert_eq!(runtime.parent_effect_phase, Some(0));

        process_at(&mut effect, &mut runtime, 49);
        assert_eq!(runtime.parent_effect_phase, Some(35));

        process_at(&mut effect, &mut runtime, 50);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_effect_phase, Some(35));
    }

    // -- EF_BLUEBODY -------------------------------------------------------

    #[test]
    fn bluebody_tint_is_statecnt_over_three_up_to_150() {
        let mut effect = BlueBody::new();
        let mut runtime = runtime_with(200);

        process_at(&mut effect, &mut runtime, 0);
        assert_eq!(runtime.parent_flags, 0x4000);
        assert_eq!(runtime.parent_tint_phase, Some(0));

        process_at(&mut effect, &mut runtime, 2);
        assert_eq!(runtime.parent_tint_phase, Some(0), "integer division");

        process_at(&mut effect, &mut runtime, 3);
        assert_eq!(runtime.parent_tint_phase, Some(1));

        process_at(&mut effect, &mut runtime, 150);
        assert_eq!(runtime.parent_tint_phase, Some(50));

        process_at(&mut effect, &mut runtime, 151);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_tint_phase, Some(50), "the last write persists");
    }

    // -- EF_MADNESS_BLUE / EF_MADNESS_RED ---------------------------------

    #[test]
    fn madness_bodies_pin_tint_on_every_fourth_frame() {
        let cases: [(Box<dyn PortedEffect>, u32, &str); 2] = [
            (Box::new(MadnessBlue::new()), 0x4000, "EF_MADNESS_BLUE"),
            (Box::new(MadnessRed::new()), 0x40000, "EF_MADNESS_RED"),
        ];

        for (mut effect, expected_flag, name) in cases {
            let mut runtime = runtime_with(60);

            process_at(&mut *effect, &mut runtime, 0);
            assert_eq!(runtime.parent_flags, expected_flag, "{name}");
            assert_eq!(runtime.parent_tint_phase, Some(20), "{name}");

            // Non-multiple frames write nothing; the pin persists.
            process_at(&mut *effect, &mut runtime, 1);
            assert_eq!(runtime.parent_flags, 0, "{name}");
            assert_eq!(runtime.parent_tint_phase, Some(20), "{name}");

            process_at(&mut *effect, &mut runtime, 4);
            assert_eq!(runtime.parent_flags, expected_flag, "{name}");
            assert_eq!(runtime.parent_tint_phase, Some(20), "{name}");
        }
    }

    // -- EF_RED_HIT --------------------------------------------------------

    #[test]
    fn red_hit_triples_tint_up_to_16() {
        let mut effect = RedHit::new();
        let mut runtime = runtime_with(60);

        process_at(&mut effect, &mut runtime, 0);
        assert_eq!(runtime.parent_flags, 0x40000);
        assert_eq!(runtime.parent_tint_phase, Some(0));

        process_at(&mut effect, &mut runtime, 16);
        assert_eq!(runtime.parent_tint_phase, Some(48));

        process_at(&mut effect, &mut runtime, 17);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_tint_phase, Some(48), "the last write persists");
    }

    // -- EF_REDLIGHTBODY ---------------------------------------------------

    #[test]
    fn redlightbody_multiplies_by_0_125_and_pins_25_after_200() {
        let mut effect = RedLightBody::new();
        let mut runtime = runtime_with(99999999);

        process_at(&mut effect, &mut runtime, 0);
        assert_eq!(runtime.parent_flags, 0x40000);
        assert_eq!(runtime.parent_tint_phase, Some(0));

        process_at(&mut effect, &mut runtime, 8);
        assert_eq!(runtime.parent_tint_phase, Some(1), "8 * 0.125 == 1.0");

        process_at(&mut effect, &mut runtime, 199);
        assert_eq!(runtime.parent_tint_phase, Some(24), "24.875 truncates");

        process_at(&mut effect, &mut runtime, 200);
        assert_eq!(runtime.parent_tint_phase, Some(25), "25.0 truncates to 25");

        process_at(&mut effect, &mut runtime, 201);
        assert_eq!(runtime.parent_flags, 0x40000, "the flag is re-requested every frame");
        assert_eq!(runtime.parent_tint_phase, Some(25), "the `else` branch pins 25");
    }

    // -- EF_BABYBODY -------------------------------------------------------

    #[test]
    fn babybody_shrinks_up_to_45() {
        let mut effect = BabyBody::new();
        let mut runtime = runtime_with(99999);

        process_at(&mut effect, &mut runtime, 0);
        assert_eq!(runtime.parent_flags, 0x10);
        assert_eq!(runtime.parent_shrink_phase, Some(0));

        process_at(&mut effect, &mut runtime, 45);
        assert_eq!(runtime.parent_shrink_phase, Some(45));

        process_at(&mut effect, &mut runtime, 46);
        assert_eq!(runtime.parent_flags, 0x10, "the flag keeps being requested");
        assert_eq!(runtime.parent_shrink_phase, Some(45), "the last write persists");
    }

    // -- EF_RUN ------------------------------------------------------------

    #[test]
    fn run_writes_the_start_tick_with_byte_wrap() {
        let mut effect = Run::new();
        let mut runtime = runtime_with(9999);

        process_at(&mut effect, &mut runtime, 0);
        assert_eq!(runtime.parent_flags, 0x8000);
        assert_eq!(runtime.parent_run_start_tick, Some(0));

        process_at(&mut effect, &mut runtime, 7);
        assert_eq!(runtime.parent_run_start_tick, Some(7));

        process_at(&mut effect, &mut runtime, 255);
        assert_eq!(runtime.parent_run_start_tick, Some(255));

        // `(unsigned char)256` wraps to 0, exactly like the cast.
        process_at(&mut effect, &mut runtime, 256);
        assert_eq!(runtime.parent_run_start_tick, Some(0));
    }

    // -- EF_TALK_FROSTJOKE / EF_TALK_SCREAM -------------------------------

    #[test]
    fn talk_bodies_notify_the_mode_once_on_frame_zero() {
        let mut frostjoke = TalkFrostJoke::new();
        let mut scream = TalkScream::new();
        let mut runtime = runtime_with(999);

        process_at(&mut frostjoke, &mut runtime, 0);
        assert_eq!(runtime.game_mode_notify, Some(1));
        assert!(runtime.primitives.is_empty());
        assert!(runtime.sounds.is_empty());

        process_at(&mut scream, &mut runtime, 0);
        assert_eq!(runtime.game_mode_notify, Some(0));

        // Only `stateCnt == 0` reaches `NotifyGameMode`; a later frame leaves
        // the recorded request alone.
        runtime.game_mode_notify = None;
        process_at(&mut scream, &mut runtime, 1);
        assert_eq!(runtime.game_mode_notify, None);
    }

    // -- shared ------------------------------------------------------------

    #[test]
    fn batch_j_bodies_spawn_no_geometry_and_draw_no_rng() {
        let runtimes: Vec<(&str, Box<dyn PortedEffect>, EffectRuntime)> = vec![
            ("pressed", Box::new(PressedBody::new()), runtime_with(50)),
            ("kicked", Box::new(KickedBody::new()), runtime_with(100)),
            ("4way", Box::new(FourWayBody::new()), runtime_with(100)),
            ("quake", Box::new(QuakeBody::new(EffectId::Quakebody)), runtime_with(14)),
            ("quake2", Box::new(QuakeBody::new(EffectId::Quakebody2)), runtime_with(35)),
            ("quake3", Box::new(QuakeBody3::new()), runtime_with(60)),
            ("quake4", Box::new(QuakeBody4::new()), runtime_with(60)),
            ("land", Box::new(LandBody::new()), runtime_with(35)),
            ("spined2", Box::new(SpinedBody2::new()), runtime_with(50)),
            ("blue", Box::new(BlueBody::new()), runtime_with(200)),
            ("redlight", Box::new(RedLightBody::new()), runtime_with(99999999)),
            ("redhit", Box::new(RedHit::new()), runtime_with(60)),
            ("madness_blue", Box::new(MadnessBlue::new()), runtime_with(60)),
            ("madness_red", Box::new(MadnessRed::new()), runtime_with(60)),
            ("baby", Box::new(BabyBody::new()), runtime_with(99999)),
            ("run", Box::new(Run::new()), runtime_with(9999)),
            ("frostjoke", Box::new(TalkFrostJoke::new()), runtime_with(999)),
            ("scream", Box::new(TalkScream::new()), runtime_with(999)),
        ];

        for (name, mut effect, mut runtime) in runtimes {
            for _ in 0..32 {
                if !step(&mut *effect, &mut runtime) {
                    break;
                }
            }

            assert!(runtime.primitives.is_empty(), "{name} launched geometry");
            assert_eq!(runtime.shakes, 0, "{name} quaked");

            // The bodies draw no random numbers; the shared RNG stream must be
            // untouched, so the next value equals a fresh stream's.
            let mut reference = EffectRng::new(1);
            assert_eq!(runtime.rng.clamped_random(1000), reference.clamped_random(1000), "{name}");
        }
    }
}
