//! Effects ported by the "e" parallel batch (`.str`-only tier).
//!
//! Owned exclusively by batch "e": bodies and tests live here.
//!
//! Every effect in this file is a `.str` screen-sprite clip and nothing else:
//! its `OnProcess` case body either calls `ProcessEZ2STR(0, -80, 0)` directly
//! or forwards to one of the small wrappers (`ProcessEZ2STRC10830` /
//! `ProcessEZ2STRC3CC40`, both exactly that call), gates zero or more
//! `PlayEffectWaveInline` calls on `state.stateCnt`, and launches no primitive.
//! The clip itself is loaded by `CRagEffect::Init` from
//! [`super::str_table`] and opened by [`super::holder::PortedEffectHolder`]
//! before the first step, exactly like the original.
//!
//! The bodies therefore return the *clip's* liveness for the effects whose
//! dispatch checks `ProcessEZ2STR`'s result (they retire when the resource
//! finishes), and `true` for the one case that discards it (`EF_MAGICROD`).

#![allow(dead_code)]

use super::runtime::{EffectRuntime, PortedEffect};
use super::str_player::Clip;

/// Port plumbing for the constructor-supplied clip fallback.
///
/// `CRagEffect::Init` (`RagEffectInitialization.cpp:16-711`) opens the effect's
/// `.str` resource before the first frame, and
/// [`super::holder::PortedEffectHolder`] mirrors that at spawn. The bodies here
/// also accept clips through their constructor so the per-frame tests can drive
/// real playback without the game archives - the same fallback
/// `Signum::on_process` uses (`rag/signum.rs:32-37`). Once the player exists
/// this is a no-op, so the original body is untouched.
fn open_supplied_clips(clips: &mut Vec<Clip>, runtime: &mut EffectRuntime) {
    if runtime.str_player.is_none() && !clips.is_empty() {
        runtime.open_str(std::mem::take(clips));
    }
}

// ---------------------------------------------------------------------------
// EF_TWOHANDQUICKEN / EF_SPEARQUICKEN
// ---------------------------------------------------------------------------

/// `CRagEffect::TwoHandQuicken` - `RagEffect.cpp:9488-9495`.
///
/// Dispatch: `case EF_TWOHANDQUICKEN: case EF_SPEARQUICKEN:
/// this->TwoHandQuicken();` - `RagEffect.cpp:23963-23966`.
///
/// Frame 0 rewrites `state.duration` to **9999** and plays
/// `effect\knight_twohandquicken.wav`; every frame then returns
/// `ProcessEZ2STR(0.0f, -80.0f, 0.0f)`. The lifetime table already says 9999
/// for both ids (`EffectTypes.cpp:139`, `:259`), so the write is a no-op in
/// practice but is kept verbatim.
#[derive(Debug, Default)]
pub struct TwoHandQuicken {
    clips: Vec<Clip>,
}

impl TwoHandQuicken {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for TwoHandQuicken {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:139` (EF_TWOHANDQUICKEN) / `:259` (EF_SPEARQUICKEN).
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            // `this->state.duration = 9999;`
            runtime.lifetime = 9999;
            runtime.play_sound("effect\\knight_twohandquicken.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_LEXDIVINA
// ---------------------------------------------------------------------------

/// `CRagEffect::LexDivina` - `RagEffect.cpp:1960-1982`.
///
/// Dispatch: `case EF_LEXDIVINA: if (!this->LexDivina()) { return false; }` -
/// `RagEffect.cpp:24887-24890`.
///
/// The same wave (`effect\priest_lexdivina.wav`) plays on frames 0, 20, 35, 60
/// and 70; the body returns the clip's liveness.
#[derive(Debug, Default)]
pub struct LexDivina {
    clips: Vec<Clip>,
}

impl LexDivina {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for LexDivina {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:96`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\priest_lexdivina.wav");
        }

        if runtime.frame == 20 {
            runtime.play_sound("effect\\priest_lexdivina.wav");
        }

        if runtime.frame == 35 {
            runtime.play_sound("effect\\priest_lexdivina.wav");
        }

        if runtime.frame == 60 {
            runtime.play_sound("effect\\priest_lexdivina.wav");
        }

        if runtime.frame == 70 {
            runtime.play_sound("effect\\priest_lexdivina.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_CONCENTRATION
// ---------------------------------------------------------------------------

/// `CRagEffect::Concentration` - `RagEffect.cpp:9356-9364`.
///
/// Dispatch: `case EF_CONCENTRATION: if (!this->Concentration()) { return
/// false; }` - `RagEffect.cpp:25126-25130`.
///
/// Frame 0 plays `effect\ac_concentration.wav`, then the clip owns the effect.
#[derive(Debug, Default)]
pub struct Concentration {
    clips: Vec<Clip>,
}

impl Concentration {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Concentration {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:162`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\ac_concentration.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_MAGICROD
// ---------------------------------------------------------------------------

/// `case EF_MAGICROD:` - `RagEffect.cpp:25361-25364`.
///
/// The case body is `this->ProcessEZ2STRC10830(); this->SageMagicRod();`, in
/// that order:
///
/// * `ProcessEZ2STRC10830` is `ProcessEZ2STR(0.0f, -80.0f, 0.0f)`
///   (`RagEffect.cpp:1823-1825`) and its result is **discarded**, so a finished
///   clip does not retire the effect - unlike every other body in this file.
/// * `CRagEffect::SageMagicRod` (`RagEffect.cpp:9499-9505`) re-anchors
///   `render.position` to the parent on frame 0 (the port's runtime is spawned
///   at that position, see `PortedEffectHolder`) and plays `effect\sage_magic
///   rod.wav`.
///
/// The frame-0 gate reads `state.stateCnt` *after* the clip step, exactly like
/// the original; `ProcessEZ2STR` only rewrites `stateCnt` on a loop restart
/// (loop limit 0 never reaches that path here).
#[derive(Debug, Default)]
pub struct MagicRod {
    clips: Vec<Clip>,
}

impl MagicRod {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for MagicRod {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:253`.
        499
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        // `this->ProcessEZ2STRC10830();` - result discarded on purpose.
        let _clip_alive = runtime.process_str();

        // `this->SageMagicRod();`
        if runtime.frame == 0 {
            runtime.play_sound("effect\\sage_magic rod.wav");
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_ANGELUS
// ---------------------------------------------------------------------------

/// `case EF_ANGELUS:` - `RagEffect.cpp:24770-24777`.
///
/// Frame 0 plays `effect\EF_Angelus.wav`, then the body returns the clip's
/// liveness. The resource row is the non-`_min` branch of the
/// `TT_MIN_EFFECT_ON_OFF` toggle (`RagEffectInitialization.cpp:481-484`); like
/// the rest of the table, the port takes the regular name.
#[derive(Debug, Default)]
pub struct Angelus {
    clips: Vec<Clip>,
}

impl Angelus {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Angelus {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:50`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\EF_Angelus.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_CURE
// ---------------------------------------------------------------------------

/// `case EF_CURE:` - `RagEffect.cpp:24791-24798`.
///
/// Frame 0 plays `effect\Acolyte_cure.wav` (note the capital `A`), then the
/// clip owns the effect. Resource row: `RagEffectInitialization.cpp:54-57`
/// takes the non-`cure_min` branch.
#[derive(Debug, Default)]
pub struct Cure {
    clips: Vec<Clip>,
}

impl Cure {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Cure {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:75`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\Acolyte_cure.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_MAGNIFICAT
// ---------------------------------------------------------------------------

/// `case EF_MAGNIFICAT:` - `RagEffect.cpp:24834-24841`.
///
/// Frame 0 plays `effect\priest_magnificat.wav`, then the clip owns the effect.
/// Resource row: `RagEffectInitialization.cpp:69-72` (`magnificat_min` toggle).
#[derive(Debug, Default)]
pub struct Magnificat {
    clips: Vec<Clip>,
}

impl Magnificat {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Magnificat {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:85`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\priest_magnificat.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_RESURRECTION
// ---------------------------------------------------------------------------

/// `case EF_RESURRECTION:` - `RagEffect.cpp:24842-24849`.
///
/// Frame 0 plays `effect\priest_resurrection.wav`, then the clip owns the
/// effect. Resource row: `RagEffectInitialization.cpp:72-75`
/// (`resurrection_min` toggle).
#[derive(Debug, Default)]
pub struct Resurrection {
    clips: Vec<Clip>,
}

impl Resurrection {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Resurrection {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:86`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\priest_resurrection.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_RECOVERY
// ---------------------------------------------------------------------------

/// `case EF_RECOVERY:` - `RagEffect.cpp:24850-24857`.
///
/// Frame 0 plays `effect\priest_recovery.wav`, then the clip owns the effect.
#[derive(Debug, Default)]
pub struct Recovery {
    clips: Vec<Clip>,
}

impl Recovery {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Recovery {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:87`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\priest_recovery.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_STONECURSE
// ---------------------------------------------------------------------------

/// `case EF_STONECURSE:` - `RagEffect.cpp:25200-25205`.
///
/// `case EF_STONECURSE: case EF_PETRIFYATTACK: if
/// (!this->ProcessEZ2STRC3CC40()) { return false; }` - the wrapper is
/// `ProcessEZ2STR(0.0f, -80.0f, 0.0f)` (`RagEffect.cpp:1872-1874`), so the
/// effect lives exactly as long as `StoneCurse.str` and plays no wave.
#[derive(Debug, Default)]
pub struct StoneCurse {
    clips: Vec<Clip>,
}

impl StoneCurse {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for StoneCurse {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:32`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        runtime.process_str()
    }
}

#[cfg(test)]
mod tests {
    use super::super::runtime::{EffectRuntime, step};
    use super::super::str_player::{KeyFrame, Transform};
    use super::*;

    /// One clip whose keys sit on the given absolute effect frames.
    fn clip(frames: &[(i32, f32)]) -> Vec<Clip> {
        vec![Clip {
            texture_count: 1,
            frames: frames
                .iter()
                .map(|&(frame, x)| KeyFrame {
                    frame,
                    transform: Transform { x, ..Transform::default() },
                })
                .collect(),
        }]
    }

    /// A clip that never ends: a single key is never advanced past (the
    /// original only advances when the *next* key starts on this frame).
    fn single_key_clip() -> Vec<Clip> {
        clip(&[(0, 0.0)])
    }

    /// Two keys, frames 0 and 3: the cursor closes at frame 3 and
    /// `ProcessEZ2STR` returns `false` on frame 4.
    fn two_key_clip() -> Vec<Clip> {
        clip(&[(0, 10.0), (3, 40.0)])
    }

    fn runtime(lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], 11)
    }

    /// `(frame, sounds)` for every step that produced at least one sound.
    fn sound_schedule(body: &mut dyn PortedEffect, runtime: &mut EffectRuntime, frames: usize) -> Vec<(i32, Vec<String>)> {
        let mut schedule = Vec::new();

        for _ in 0..frames {
            runtime.sounds.clear();
            let frame = runtime.frame;
            step(body, runtime);

            if !runtime.sounds.is_empty() {
                schedule.push((frame, runtime.sounds.clone()));
            }
        }

        schedule
    }

    #[test]
    fn two_hand_quicken_extends_the_duration_and_plays_its_wave() {
        let mut effect = TwoHandQuicken::new(two_key_clip());
        let mut runtime = runtime(effect.lifetime());

        // The frame-0 rewrite of `state.duration`.
        assert_eq!(runtime.lifetime, 9999);
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.lifetime, 9999);
        assert_eq!(runtime.sounds, vec!["effect\\knight_twohandquicken.wav".to_owned()]);

        assert_eq!(effect.lifetime(), 9999);

        // The wave is a frame-0 event only.
        let schedule = sound_schedule(&mut effect, &mut runtime, 30);
        assert!(schedule.is_empty(), "{schedule:?}");
    }

    #[test]
    fn two_hand_quicken_ends_when_the_clip_ends() {
        let mut effect = TwoHandQuicken::new(two_key_clip());
        let mut runtime = runtime(effect.lifetime());

        // Clip frames 0..=3, the cursor closes at 3, `ProcessEZ2STR` fails on 4.
        for _ in 0..4 {
            assert!(step(&mut effect, &mut runtime));
        }

        assert_eq!(runtime.frame, 4);
        assert!(!step(&mut effect, &mut runtime), "retires on frame 4");
    }

    #[test]
    fn lex_divina_plays_five_waves_at_the_official_frames() {
        let mut effect = LexDivina::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());

        let schedule = sound_schedule(&mut effect, &mut runtime, 80);

        let expected_frames = [0, 20, 35, 60, 70];
        assert_eq!(schedule.len(), expected_frames.len());

        for (index, (frame, sounds)) in schedule.iter().enumerate() {
            assert_eq!(*frame, expected_frames[index]);
            assert_eq!(sounds, &vec!["effect\\priest_lexdivina.wav".to_owned()]);
        }
    }

    #[test]
    fn lex_divina_retires_with_its_clip() {
        let mut effect = LexDivina::new(two_key_clip());
        let mut runtime = runtime(effect.lifetime());

        for _ in 0..4 {
            assert!(step(&mut effect, &mut runtime));
        }

        assert!(!step(&mut effect, &mut runtime));
    }

    #[test]
    fn concentration_plays_ac_concentration_on_frame_zero() {
        let mut effect = Concentration::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\ac_concentration.wav".to_owned()]);
        assert_eq!(effect.lifetime(), 999);

        assert!(sound_schedule(&mut effect, &mut runtime, 5).is_empty());
    }

    #[test]
    fn magic_rod_plays_its_wave_on_frame_zero_and_discards_the_clip_result() {
        let mut effect = MagicRod::new(two_key_clip());
        let mut runtime = runtime(effect.lifetime());

        assert_eq!(effect.lifetime(), 499);

        // Frame 0: `ProcessEZ2STRC10830()` runs before `SageMagicRod()`.
        assert!(step(&mut effect, &mut runtime));
        assert_eq!(runtime.sounds, vec!["effect\\sage_magic rod.wav".to_owned()]);

        // Clip frames 0..=3, cursor closes at 3; the frame-4 `false` from
        // `ProcessEZ2STR` is discarded, so the effect keeps running.
        for _ in 0..4 {
            assert!(step(&mut effect, &mut runtime), "frame {}", runtime.frame - 1);
        }

        assert_eq!(runtime.frame, 5);
        assert!(runtime.str_player.is_some());
        assert_eq!(
            runtime.str_player.as_ref().unwrap().current_layer,
            1,
            "the clip itself has finished"
        );
        assert!(sound_schedule(&mut effect, &mut runtime, 5).is_empty());
    }

    #[test]
    fn the_buff_family_plays_exactly_one_wave_on_frame_zero() {
        let cases: Vec<(Box<dyn PortedEffect>, i32, &str)> = vec![
            (Box::new(Angelus::new(single_key_clip())), 999, "effect\\EF_Angelus.wav"),
            (Box::new(Cure::new(single_key_clip())), 999, "effect\\Acolyte_cure.wav"),
            (
                Box::new(Magnificat::new(single_key_clip())),
                999,
                "effect\\priest_magnificat.wav",
            ),
            (
                Box::new(Resurrection::new(single_key_clip())),
                999,
                "effect\\priest_resurrection.wav",
            ),
            (Box::new(Recovery::new(single_key_clip())), 999, "effect\\priest_recovery.wav"),
        ];

        for (mut body, lifetime, wave) in cases {
            let mut runtime = runtime(body.lifetime());
            assert_eq!(body.lifetime(), lifetime, "{wave}");

            assert!(step(&mut *body, &mut runtime), "{wave} frame 0");
            assert_eq!(runtime.sounds, vec![wave.to_owned()], "{wave} frame 0");

            assert!(sound_schedule(&mut *body, &mut runtime, 10).is_empty(), "{wave}");
        }
    }

    #[test]
    fn stone_curse_has_no_wave_and_ends_with_the_clip() {
        let mut effect = StoneCurse::new(two_key_clip());
        let mut runtime = runtime(effect.lifetime());

        assert_eq!(effect.lifetime(), 999);

        let snapshots = super::super::runtime::run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 5, "clip frames 0..=3, retire on 4");
        assert!(snapshots.iter().all(|snapshot| snapshot.sounds.is_empty()));
        assert!(snapshots.iter().all(|snapshot| snapshot.primitives.is_empty()));
    }

    #[test]
    fn the_clip_only_tier_launches_no_primitives() {
        let bodies: Vec<Box<dyn PortedEffect>> = vec![
            Box::new(TwoHandQuicken::new(single_key_clip())),
            Box::new(LexDivina::new(single_key_clip())),
            Box::new(Concentration::new(single_key_clip())),
            Box::new(MagicRod::new(single_key_clip())),
            Box::new(Angelus::new(single_key_clip())),
            Box::new(Cure::new(single_key_clip())),
            Box::new(Magnificat::new(single_key_clip())),
            Box::new(Resurrection::new(single_key_clip())),
            Box::new(Recovery::new(single_key_clip())),
            Box::new(StoneCurse::new(single_key_clip())),
        ];

        for mut body in bodies {
            let mut runtime = runtime(body.lifetime());

            for _ in 0..5 {
                step(&mut *body, &mut runtime);
            }

            assert!(runtime.primitives.is_empty(), "a clip-only body spawned geometry");
        }
    }

    #[test]
    fn supplied_clips_are_opened_once_and_only_when_the_runtime_has_none() {
        let mut effect = Angelus::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());

        step(&mut effect, &mut runtime);
        assert!(runtime.str_player.is_some());

        // A second clip supplied later must not replace the open one.
        effect.clips = two_key_clip();
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.str_clips.len(), 1);
        assert_eq!(runtime.str_clips[0].frames.len(), 1, "the original clip survives");
    }
}
