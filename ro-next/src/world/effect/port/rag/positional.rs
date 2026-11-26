//! Positional-sound dispatch cases: a `PlayWave` with listener-relative
//! coordinates plus the effect's `.str` clip.
//!
//! These are the seven effects that were blocked on
//! [`EffectRuntime::positional_sounds`](super::super::runtime::EffectRuntime):
//! their `OnProcess` cases call `PlayEffectWaveAt` (`RagEffect.cpp:22972`) or
//! the raw `g_soundMgr->PlayWave(name, dx, height, dz, ...)`
//! (`SoundMgr.cpp:627`), which the old `vec<String>` sound channel could not
//! express. Their only other action is `ProcessEZ2STR(0, -80, 0)`; like the
//! batch-"e" `.str` tier, the clip itself is loaded by
//! [`super::super::holder::PortedEffectHolder`] from
//! [`super::super::str_table`] before the first step (or supplied through the
//! constructor by the per-frame tests, which have no game archives).
//!
//! The official `render.position = state.parent->render.position` preamble of
//! three of the cases (elemental weapons, Loud) is already the spawn anchor of
//! the port: the client seeds `runtime.position` from the caster entity and
//! these bodies never move the effect, so the assignment has nothing left to
//! do. The one thing that cannot be reproduced per frame is the *current*
//! local-player position used by `PlayEffectWaveAt`: the hermetic runtime
//! carries the spawn-time snapshot in
//! [`listener_position`](super::super::runtime::EffectRuntime::listener_position).

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

/// Constructor-supplied clip fallback for the tests (the holder normally
/// opens the resource before the first step, so this is a no-op in game).
fn open_supplied_clips(clips: &mut Vec<Clip>, runtime: &mut EffectRuntime) {
    if runtime.str_player.is_none() && !clips.is_empty() {
        runtime.open_str(std::mem::take(clips));
    }
}

// ---------------------------------------------------------------------------
// EF_GLORIA
// ---------------------------------------------------------------------------

/// `CRagEffect` dispatch `case EF_GLORIA` - `RagEffect.cpp:24826-24833`.
///
/// Frame 0 plays `effect\priest_gloria.wav` at height `1.0`
/// (`PlayEffectWaveAt`), then every frame returns
/// `ProcessEZ2STR(0.0f, -80.0f, 0.0f)`.
#[derive(Debug, Default)]
pub struct Gloria {
    clips: Vec<Clip>,
}

impl Gloria {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Gloria {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:84` - `result[EF_GLORIA] = 999`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            // `PlayEffectWaveAt(*this, "effect\\priest_gloria.wav", 1.0f);`
            runtime.play_sound_at_height("effect\\priest_gloria.wav", 1.0);
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_FLAMELAUNCHER / EF_FROSTWEAPON / EF_LIGHTNINGLOADER / EF_SEISMICWEAPON
// ---------------------------------------------------------------------------

/// `CRagEffect` dispatch
/// `case EF_FLAMELAUNCHER: case EF_FROSTWEAPON:
///  case EF_LIGHTNINGLOADER: case EF_SEISMICWEAPON:`
/// - `RagEffect.cpp:25349-25359`.
///
/// Frame 10 re-anchors on the parent and plays `_enemy_hit_wind1.wav` at
/// height `-150.0` (`PlayEffectWaveAt`); every frame then returns
/// `ProcessEZ2STRC10830()` (= `ProcessEZ2STR(0.0f, -80.0f, 0.0f)`).
#[derive(Debug, Default)]
pub struct ElementalWeapon {
    clips: Vec<Clip>,
}

impl ElementalWeapon {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for ElementalWeapon {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:264-267` - all four ids are 999.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 10 {
            // `this->render.position = this->state.parent->render.position;`
            // is the port's spawn anchor (see the module docs).
            runtime.play_sound_at_height("_enemy_hit_wind1.wav", -150.0);
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_HOLYCROSS
// ---------------------------------------------------------------------------

/// `CRagEffect` dispatch `case EF_HOLYCROSS` - `RagEffect.cpp:25264-25270`.
///
/// Frame 0 plays `effect\cru_holy cross.wav` with the literal raw
/// `PlayWave(..., 0.0f, 0.0f, 0.0f, ...)` coordinates - the listener origin,
/// not the effect position - then every frame returns `ProcessEZ2STRC10830()`.
#[derive(Debug, Default)]
pub struct HolyCross {
    clips: Vec<Clip>,
}

impl HolyCross {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for HolyCross {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:254` - `result[EF_HOLYCROSS] = 499`.
        499
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound_at("effect\\cru_holy cross.wav", [0.0, 0.0, 0.0]);
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_LOUD
// ---------------------------------------------------------------------------

/// `CRagEffect` dispatch `case EF_LOUD` - `RagEffect.cpp:10161-10172`.
///
/// Frame 0 re-anchors on the parent and plays `effect\고성방가.wav` at height
/// `0.0` with `dx = effect.x - player.x`, `dz = effect.z - player.z` - exactly
/// the `PlayEffectWaveAt` shape with a zero height. Every frame then returns
/// `ProcessEZ2STRC10830()`.
#[derive(Debug, Default)]
pub struct Loud {
    clips: Vec<Clip>,
}

impl Loud {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Loud {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:320` - `result[EF_LOUD] = 300`.
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            // `this->render.position = this->state.parent->render.position;`
            // is the port's spawn anchor (see the module docs).
            runtime.play_sound_at_height("effect\\고성방가.wav", 0.0);
        }

        runtime.process_str()
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::runtime::{EffectRuntime, step};
    use super::super::super::str_player::{KeyFrame, Transform};
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

    /// A clip that never ends: a single key is never advanced past.
    fn single_key_clip() -> Vec<Clip> {
        clip(&[(0, 0.0)])
    }

    /// Two keys, frames 0 and 3: `ProcessEZ2STR` closes on frame 3 and
    /// returns `false` on frame 4.
    fn two_key_clip() -> Vec<Clip> {
        clip(&[(0, 10.0), (3, 40.0)])
    }

    fn runtime(lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], 17)
    }

    #[test]
    fn lifetimes_match_the_effect_types_table() {
        assert_eq!(Gloria::default().lifetime(), 999);
        assert_eq!(ElementalWeapon::default().lifetime(), 999);
        assert_eq!(HolyCross::default().lifetime(), 499);
        assert_eq!(Loud::default().lifetime(), 300);
    }

    #[test]
    fn gloria_plays_at_height_one() {
        let mut effect = Gloria::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());
        runtime.position = [13.0, 0.0, 25.0];
        runtime.listener_position = [10.0, 0.0, 20.0];

        step(&mut effect, &mut runtime);

        // `PlayEffectWaveAt(..., 1.0f)`: `[effect.x - player.x, 1, effect.z -
        // player.z]`.
        assert_eq!(runtime.positional_sounds, vec![("effect\\priest_gloria.wav".to_owned(), [
            3.0, 1.0, 5.0
        ])]);
        // The inline `PlayEffectWaveInline` channel is not used here.
        assert!(runtime.sounds.is_empty());
    }

    #[test]
    fn elemental_weapons_play_on_frame_ten_at_minus_150() {
        let mut effect = ElementalWeapon::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());
        runtime.position = [13.0, 0.0, 25.0];
        runtime.listener_position = [10.0, 0.0, 20.0];

        // Frames 0..=9: no sound yet (the case gates on `stateCnt == 10`).
        for _ in 0..10 {
            step(&mut effect, &mut runtime);
            assert!(runtime.positional_sounds.is_empty());
        }

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.positional_sounds, vec![("_enemy_hit_wind1.wav".to_owned(), [
            3.0, -150.0, 5.0
        ])]);
    }

    #[test]
    fn elemental_weapon_sound_is_a_single_frame_ten_event() {
        let mut effect = ElementalWeapon::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());

        for _ in 0..14 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.positional_sounds.len(), 1);
    }

    #[test]
    fn holy_cross_plays_at_the_listener_origin() {
        let mut effect = HolyCross::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());
        runtime.position = [13.0, 0.0, 25.0];
        runtime.listener_position = [10.0, 0.0, 20.0];

        step(&mut effect, &mut runtime);

        // The raw call passes literal zeroes, not the effect offsets.
        assert_eq!(runtime.positional_sounds, vec![("effect\\cru_holy cross.wav".to_owned(), [
            0.0, 0.0, 0.0
        ])]);
        assert!(runtime.sounds.is_empty());
    }

    #[test]
    fn loud_plays_at_the_effect_position() {
        let mut effect = Loud::new(single_key_clip());
        let mut runtime = runtime(effect.lifetime());
        runtime.position = [13.0, 0.0, 25.0];
        runtime.listener_position = [10.0, 0.0, 20.0];

        step(&mut effect, &mut runtime);

        // Raw `PlayWave(name, dx, 0.0f, dz, ...)` == `PlayEffectWaveAt` height 0.
        assert_eq!(runtime.positional_sounds, vec![("effect\\고성방가.wav".to_owned(), [
            3.0, 0.0, 5.0
        ])]);
        assert!(runtime.sounds.is_empty());
    }

    #[test]
    fn positional_bodies_retire_when_the_str_clip_ends() {
        for mut effect in [
            Box::new(Gloria::new(two_key_clip())) as Box<dyn PortedEffect>,
            Box::new(ElementalWeapon::new(two_key_clip())) as Box<dyn PortedEffect>,
            Box::new(HolyCross::new(two_key_clip())) as Box<dyn PortedEffect>,
            Box::new(Loud::new(two_key_clip())) as Box<dyn PortedEffect>,
        ] {
            let mut runtime = runtime(effect.lifetime());

            // Frames 0..=3 are alive; `ProcessEZ2STR` fails on frame 4.
            for frame in 0..4 {
                assert!(step(&mut *effect, &mut runtime), "frame {frame}");
            }

            assert!(!step(&mut *effect, &mut runtime));
        }
    }

    #[test]
    fn without_a_clip_the_effect_lasts_its_full_lifetime() {
        let mut effect = Gloria::new(Vec::new());
        let mut runtime = runtime(effect.lifetime());

        for _ in 0..998 {
            assert!(step(&mut effect, &mut runtime));
        }

        assert!(!step(&mut effect, &mut runtime));
        assert_eq!(runtime.frame, 999);
    }
}
