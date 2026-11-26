//! Champion (Monk -> Champion) class effect bodies.
//!
//! The official dispatch splits the class across three handlers, chosen by the
//! effect id before the case body runs (`RagEffect.cpp:23088-23097`):
//!
//! * ids `300..=373` -> `CRagEffect::OnProcess2`,
//! * ids `>= 491` -> `CRagEffect::OnProcess3`,
//! * everything else (including `374..=490`) -> the main `OnProcess` switch.
//!
//! Eleven of the in-scope ids port 1:1 here:
//!
//! * five caster bodies, whose case body writes `state.parent->render` and
//!   nothing else - routed through the existing runtime channels
//!   ([`EffectRuntime::add_parent_flags`], [`EffectRuntime::set_parent_color`],
//!   [`EffectRuntime::set_parent_outline_alpha`],
//!   [`EffectRuntime::set_parent_effect_phase`]);
//! * six `.str` bodies, whose case body is `ProcessEZ2STRC10830()` (plus the
//!   official wave for `EF_CHAINCOMBO`). Their resources are the rows this unit
//!   added to `port/str_table.rs` and the holder opens them before frame 0.
//!
//! Frame gates, literals and the `SetColor`/`flags` argument order are kept.
//!
//! ## Unported ids and their exact blockers
//!
//! The remaining thirteen in-scope ids are **not** caster bodies; they call a
//! primitive helper whose primitive id does not exist in
//! [`PrimitiveId`](super::super::primitive::PrimitiveId). Primitive modules are
//! out of scope for this unit, so they are left unported rather than
//! approximated:
//!
//! | id | official dispatch | helper | primitive |
//! | --- | --- | --- | --- |
//! | `EF_GUMGANG` (203) | `RagEffect.cpp:24166-24168` | `SuperSlashFan(1, 0)` - `RagEffect.cpp:16964` | **35** (no `EffectPrimId.h` symbol; the enum jumps `27 -> 34`) |
//! | `EF_STEELBODY` (254) | `RagEffect.cpp:24181-24183` | `SuperSlashFan(4, 0)` - `RagEffect.cpp:16964` | **35** |
//! | `EF_GUMGANG2` (261) | `RagEffect.cpp:23506-23508` | `TexturedCrossLevel` - `RagEffect.cpp:5246` | **31** (no symbol) |
//! | `EF_AURABLADE` (367) | `OnProcess2`, `RagEffect.cpp:10368-10370` | `WhiteYellowRings` - `RagEffect.cpp:18656` | **30** (`TexturedQuadRingSized`, `RagEffect.cpp:17437`; no symbol) |
//! | `EF_BEGINASURA` (328) | `OnProcess2`, `RagEffect.cpp:10212-10214` | `AsuraStrike(0)` - `RagEffect.cpp:14187` | **68** `PP_ASURACASTING` (+ **30**) |
//! | `EF_BEGINASURA1..7` (467-473) | `RagEffect.cpp:23627-23647` | `TexturedSkillIcon` - `RagEffect.cpp:3774` | **68** `PP_ASURACASTING` |
//! | `EF_BEGINASURA11` (510) | `OnProcess3`, `RagEffect.cpp:21512-21514` | `AsuraStrike(1)` - `RagEffect.cpp:14187` | **68** `PP_ASURACASTING` (+ **30**) |
//!
//! Primitive 68 is named in `EffectPrimId.h:53`; 30, 31 and 35 are gaps in
//! that header (`27 -> 34`), so the numbers are only visible at the call sites
//! above.

#![allow(dead_code)]

use ragnarok_packets::EffectId;

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

/// Constructor-supplied clip fallback for the tests (the holder normally
/// opens the resource from `port/str_table.rs` before the first step, so this
/// is a no-op in game).
fn open_supplied_clips(clips: &mut Vec<Clip>, runtime: &mut EffectRuntime) {
    if runtime.str_player.is_none() && !clips.is_empty() {
        runtime.open_str(std::mem::take(clips));
    }
}

// ---------------------------------------------------------------------------
// EF_LIGHTBLADE
// ---------------------------------------------------------------------------

/// `case EF_LIGHTBLADE:` - `RagEffect.cpp:24189-24193` (main `OnProcess`
/// switch).
///
/// The whole case body is `if ((flags & 4) == 0) flags += 4;`. The runtime's
/// [`EffectRuntime::begin_step`] clears `parent_flags` before the body runs -
/// the official actor clears the word at the start of the next update after it
/// drew the actor (`GameObject.cpp:435` sets `flagsResetPending`,
/// `GameActor.cpp:685-687` clears it) - so the guard is true on every rendered
/// frame and the OR is the exact `+=` equivalent.
#[derive(Debug, Default)]
pub struct LightBlade;

impl PortedEffect for LightBlade {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:391` - `result[EF_LIGHTBLADE] = 99999999`.
        99_999_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 4 == 0 {
            runtime.add_parent_flags(4);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_AURABLADE2
// ---------------------------------------------------------------------------

/// `case EF_AURABLADE2:` - `RagEffect.cpp:24184-24188`.
///
/// Same shape as [`LightBlade`] with bit `2`.
#[derive(Debug, Default)]
pub struct AuraBlade2;

impl PortedEffect for AuraBlade2 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:483` - `result[EF_AURABLADE2] = 99999999`.
        99_999_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 2 == 0 {
            runtime.add_parent_flags(2);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_ASURABODY
// ---------------------------------------------------------------------------

/// `case EF_ASURABODY:` - `RagEffect.cpp:24514-24519`.
///
/// `if ((flags & 0x800) == 0) { flags += 0x800; outlineAlpha = frame < 200 ?
/// (unsigned char)frame : 200; }`. The outline write sits *inside* the guard,
/// so it only happens on a frame whose flag word is still clear; with the
/// per-frame clear that is every rendered frame, and the alpha ramps
/// `0, 1, ..., 199, 200, 200, ...`. Both writes are kept in the official order
/// (flags first, then the alpha).
#[derive(Debug, Default)]
pub struct AsuraBody;

impl PortedEffect for AsuraBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:433` - `result[EF_ASURABODY] = 99999999`.
        99_999_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x800 == 0 {
            runtime.add_parent_flags(0x800);

            // `static_cast<unsigned char>(stateCnt)` for `stateCnt < 200`;
            // the literal `200` otherwise.
            let alpha = if runtime.frame < 200 { runtime.frame as u8 } else { 200 };
            runtime.set_parent_outline_alpha(alpha);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_ASURABODY_MONSTER
// ---------------------------------------------------------------------------

/// `case EF_ASURABODY_MONSTER:` - `RagEffect.cpp:24520-24527`.
///
/// `if ((flags & 0x800) == 0) { flags += 0x800; if (frame < 150)
/// effectPhase = (short)(frame + 50); }`. The phase write is gated on
/// `frame < 150` and, unlike the flag word, `render.effectPhase` is **not**
/// cleared per frame (`CRenderObject::Reset`, `RenderObjectState.cpp:75`), so
/// the original stops writing after frame 149 and the actor keeps the last
/// value, `199`. The runtime keeps that persistence (see
/// [`EffectRuntime::parent_effect_phase`]).
#[derive(Debug, Default)]
pub struct AsuraBodyMonster;

impl PortedEffect for AsuraBodyMonster {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:436` - `result[EF_ASURABODY_MONSTER] = 99999999`.
        99_999_999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.parent_flags & 0x800 == 0 {
            runtime.add_parent_flags(0x800);

            if runtime.frame < 150 {
                runtime.set_parent_effect_phase((runtime.frame + 50) as i16);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_TRANSBLUEBODY
// ---------------------------------------------------------------------------

/// `case EF_TRANSBLUEBODY:` - `RagEffect.cpp:24762-24766`.
///
/// Every simulated frame (`0..=200`; the lifetime is 200, so the `200`
/// boundary is unreachable) calls
/// `SetColor(-1, 205 - frame, 205 - frame, 255)`: red and green ramp down from
/// 205 to 6 while the blue channel and the `-1` alpha are fixed. `frame` is
/// the effect's `stateCnt` (`state.stateCnt`).
#[derive(Debug, Default)]
pub struct TransBlueBody;

impl PortedEffect for TransBlueBody {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:388` - `result[EF_TRANSBLUEBODY] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        // `stateCnt >= 0 && stateCnt <= 200` - two-sided in the original; the
        // lower bound is kept for completeness, the upper one is dead code
        // within the 200-frame lifetime.
        if (0..=200).contains(&frame) {
            let channel = (205 - frame) as i16;
            runtime.set_parent_color(-1, channel, channel, 255);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_COMBOATTACK1..5
// ---------------------------------------------------------------------------

/// `case EF_COMBOATTACK1: ... case EF_COMBOATTACK5:` in the shared
/// `ProcessEZ2STRC10830` group - `RagEffect.cpp:25304-25322`.
///
/// The whole group body is
/// `if (!this->ProcessEZ2STRC10830()) return false;`; the five ids differ only
/// in the `.str` resource (`yunta_1.str`..`yunta_5.str`,
/// `RagEffectInitialization.cpp:262-273`). `ProcessEZ2STRC10830` is
/// `ProcessEZ2STR(0.0f, -80.0f, 0.0f)` (`RagEffect.cpp:1823-1825`), which is
/// exactly [`EffectRuntime::process_str`]'s default offset. Lifetime 9999
/// (`EffectTypes.cpp:195-199`); `loopLimit` keeps the constructor default 0
/// (`RagEffect.cpp:109`), so the clip plays once and the effect retires with
/// it.
#[derive(Debug, Default)]
pub struct ComboAttack {
    clips: Vec<Clip>,
}

impl ComboAttack {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for ComboAttack {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:195-199` - all five ids are 9999.
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_CHAINCOMBO
// ---------------------------------------------------------------------------

/// `case EF_CHAINCOMBO:` - `RagEffect.cpp:24406-24414`.
///
/// Frame 0 copies the parent position (the port's spawn anchor already is that
/// position) and plays `effect\mon_연환.wav`; every frame then returns
/// `ProcessEZ2STRC10830()` (the `연환.str` clip,
/// `RagEffectInitialization.cpp:347-349`). Lifetime 300
/// (`EffectTypes.cpp:282`), `loopLimit` 0, so it retires with the clip.
#[derive(Debug, Default)]
pub struct ChainCombo {
    clips: Vec<Clip>,
}

impl ChainCombo {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for ChainCombo {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:282` - `result[EF_CHAINCOMBO] = 300`.
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            // `this->render.position = this->state.parent->render.position;`
            // is a no-op in the hermetic runtime: `position` is already the
            // caster anchor the spawn path fills.
            runtime.play_sound("effect\\mon_연환.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Creates the ported body for a Champion effect id, if one is portable today.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    match effect {
        EffectId::Lightblade => Some(Box::new(LightBlade)),
        EffectId::Aurablade2 => Some(Box::new(AuraBlade2)),
        EffectId::Asurabody => Some(Box::new(AsuraBody)),
        EffectId::AsurabodyMonster => Some(Box::new(AsuraBodyMonster)),
        EffectId::Transbluebody => Some(Box::new(TransBlueBody)),
        EffectId::Comboattack1 | EffectId::Comboattack2 | EffectId::Comboattack3 | EffectId::Comboattack4 | EffectId::Comboattack5 => {
            Some(Box::new(ComboAttack::new(Vec::new())))
        }
        EffectId::Chaincombo => Some(Box::new(ChainCombo::new(Vec::new()))),
        _ => None,
    }
}

/// Every Champion id this module ports.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Lightblade,
        EffectId::Aurablade2,
        EffectId::Asurabody,
        EffectId::AsurabodyMonster,
        EffectId::Transbluebody,
        EffectId::Comboattack1,
        EffectId::Comboattack2,
        EffectId::Comboattack3,
        EffectId::Comboattack4,
        EffectId::Comboattack5,
        EffectId::Chaincombo,
    ]
}

/// `EffectLifetime` for [`ids`] (`EffectTypes.cpp`).
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:391`.
        (EffectId::Lightblade, 99_999_999),
        // `EffectTypes.cpp:483`.
        (EffectId::Aurablade2, 99_999_999),
        // `EffectTypes.cpp:433`.
        (EffectId::Asurabody, 99_999_999),
        // `EffectTypes.cpp:436`.
        (EffectId::AsurabodyMonster, 99_999_999),
        // `EffectTypes.cpp:388`.
        (EffectId::Transbluebody, 200),
        // `EffectTypes.cpp:195-199`.
        (EffectId::Comboattack1, 9999),
        (EffectId::Comboattack2, 9999),
        (EffectId::Comboattack3, 9999),
        (EffectId::Comboattack4, 9999),
        (EffectId::Comboattack5, 9999),
        // `EffectTypes.cpp:282`.
        (EffectId::Chaincombo, 300),
    ]
}

/// The ids whose body only touches the caster actor: no primitive, no `.str`
/// clip.
///
/// Consumed by the shared registry invariant in `port/registry.rs`; the six
/// `.str` bodies are covered there by `needs_str_clips` instead.
pub fn is_caster_body_only(effect: EffectId) -> bool {
    matches!(
        effect,
        EffectId::Lightblade | EffectId::Aurablade2 | EffectId::Asurabody | EffectId::AsurabodyMonster | EffectId::Transbluebody
    )
}

#[cfg(test)]
mod tests {
    use super::super::super::runtime::{DEFAULT_STR_OFFSET, EffectRuntime, step};
    use super::super::super::str_player::{KeyFrame, Transform};
    use super::*;

    fn runtime(effect: &dyn PortedEffect) -> EffectRuntime {
        EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 99)
    }

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

    /// Two keys, frames 0 and 3: `ProcessEZ2STR` closes the cursor on frame 3
    /// and returns `false` on frame 4.
    fn two_key_clip() -> Vec<Clip> {
        clip(&[(0, 10.0), (3, 40.0)])
    }

    #[test]
    fn entries_match_their_official_lifetimes() {
        for &(effect, lifetime) in lifetimes() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }

        assert_eq!(ids().len(), lifetimes().len());
    }

    #[test]
    fn caster_body_ids_are_declared_as_such() {
        for &id in ids() {
            assert_eq!(
                is_caster_body_only(id),
                !matches!(
                    id,
                    EffectId::Comboattack1
                        | EffectId::Comboattack2
                        | EffectId::Comboattack3
                        | EffectId::Comboattack4
                        | EffectId::Comboattack5
                        | EffectId::Chaincombo
                ),
                "{id:?}"
            );
        }

        assert!(!is_caster_body_only(EffectId::Gumgang));
        assert!(!is_caster_body_only(EffectId::Absorbspirits));
    }

    #[test]
    fn caster_bodies_emit_no_geometry_sound_or_clip() {
        for &id in ids().iter().filter(|id| is_caster_body_only(**id)) {
            let mut body = spawn(id).unwrap();
            let mut runtime = runtime(body.as_ref());

            for _ in 0..(body.lifetime().min(400)) {
                step(body.as_mut(), &mut runtime);
            }

            assert!(runtime.primitives.is_empty(), "{id:?} launched a primitive");
            assert!(runtime.sounds.is_empty(), "{id:?} played a sound");
            assert!(runtime.positional_sounds.is_empty(), "{id:?} played a positional sound");
            assert_eq!(runtime.shakes, 0, "{id:?} shook the camera");
            assert!(runtime.str_player.is_none(), "{id:?} opened a .str clip");
        }
    }

    /// None of the eleven bodies draws from `rand()` in the original, so they
    /// must leave the runtime RNG stream untouched. Comparing the next 32 draws
    /// of a stepped and an untouched runtime seeded identically proves it.
    #[test]
    fn bodies_consume_no_rng() {
        for &id in ids() {
            let mut body = spawn(id).unwrap();

            let mut stepped = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 0x5EED);
            for _ in 0..300 {
                step(body.as_mut(), &mut stepped);
            }

            let mut untouched = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 0x5EED);
            for draw in 0..32 {
                assert_eq!(
                    stepped.rng.clamped_random(360),
                    untouched.rng.clamped_random(360),
                    "{id:?} consumed RNG before draw {draw}"
                );
            }
        }
    }

    #[test]
    fn light_blade_requests_flag_four_every_frame() {
        let mut body = LightBlade;
        let mut runtime = runtime(&body);

        assert_eq!(body.lifetime(), 99_999_999);

        for frame in 0..300 {
            assert_eq!(runtime.frame, frame);
            step(&mut body, &mut runtime);
            // `begin_step` cleared the word, the body re-added bit 4.
            assert_eq!(runtime.parent_flags, 4, "frame {frame}");
            assert_eq!(runtime.parent_color, None, "frame {frame}");
            assert_eq!(runtime.parent_outline_alpha, None, "frame {frame}");
            assert_eq!(runtime.parent_effect_phase, None, "frame {frame}");
        }
    }

    #[test]
    fn aura_blade_two_requests_flag_two_every_frame() {
        let mut body = AuraBlade2;
        let mut runtime = runtime(&body);

        assert_eq!(body.lifetime(), 99_999_999);

        for frame in 0..300 {
            assert_eq!(runtime.frame, frame);
            step(&mut body, &mut runtime);
            assert_eq!(runtime.parent_flags, 2, "frame {frame}");
            assert_eq!(runtime.parent_color, None, "frame {frame}");
            assert_eq!(runtime.parent_outline_alpha, None, "frame {frame}");
            assert_eq!(runtime.parent_effect_phase, None, "frame {frame}");
        }
    }

    #[test]
    fn trans_blue_body_ramps_red_and_green_from_205_to_6() {
        let mut body = TransBlueBody;
        let mut runtime = runtime(&body);

        for frame in 0..200 {
            assert_eq!(runtime.frame, frame);

            let expected = [-1, 205 - frame as i16, 205 - frame as i16, 255];
            let alive = step(&mut body, &mut runtime);

            assert_eq!(runtime.parent_color, Some(expected), "frame {frame}");
            assert_eq!(runtime.parent_flags, 0, "frame {frame}");
            // The 200th step (`frame == 199`) is the one that retires the
            // effect: `step` increments and then tests `frame >= lifetime`.
            assert_eq!(alive, frame < 199, "retirement on frame {frame}");
        }

        assert_eq!(runtime.frame, 200);
        assert_eq!(runtime.parent_color, Some([-1, 6, 6, 255]));
    }

    #[test]
    fn trans_blue_body_keeps_the_last_colour_when_it_retires() {
        // The original's `SetColor` persists on the actor, so the port leaves
        // the last request in place instead of clearing it.
        let mut body = TransBlueBody;
        let mut runtime = runtime(&body);

        for _ in 0..200 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.frame, 200);
        assert_eq!(runtime.parent_color, Some([-1, 6, 6, 255]));
    }

    #[test]
    fn asura_body_ramps_outline_alpha_and_freezes_at_200() {
        let mut body = AsuraBody;
        let mut runtime = runtime(&body);

        assert_eq!(body.lifetime(), 99_999_999);

        for frame in 0..=250 {
            assert_eq!(runtime.frame, frame);
            step(&mut body, &mut runtime);

            let expected = if frame < 200 { frame as u8 } else { 200 };
            assert_eq!(runtime.parent_flags, 0x800, "frame {frame}");
            assert_eq!(runtime.parent_outline_alpha, Some(expected), "frame {frame}");
            assert_eq!(runtime.parent_effect_phase, None, "frame {frame}");
        }

        // Far beyond the ramp the value stays pinned at the literal 200.
        for _ in 0..5000 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.parent_outline_alpha, Some(200));
        assert_eq!(runtime.parent_flags, 0x800);
    }

    #[test]
    fn asura_body_writes_nothing_when_the_flag_is_already_set() {
        // `if ((flags & 0x800) == 0)` - the outline write is inside the guard.
        // `begin_step` normally clears the word, so this only happens if the
        // host presents stale flags; the guard is kept verbatim. The body is
        // called directly here because `step` would clear the word first.
        let mut body = AsuraBody;
        let mut runtime = runtime(&body);

        runtime.parent_flags = 0x800;
        assert!(body.on_process(&mut runtime));

        assert_eq!(runtime.parent_flags, 0x800);
        assert_eq!(runtime.parent_outline_alpha, None);
    }

    #[test]
    fn asura_body_monster_phase_runs_50_to_199_then_freezes() {
        let mut body = AsuraBodyMonster;
        let mut runtime = runtime(&body);

        assert_eq!(body.lifetime(), 99_999_999);

        for frame in 0..=200 {
            assert_eq!(runtime.frame, frame);
            step(&mut body, &mut runtime);

            let expected = if frame < 150 { Some((frame + 50) as i16) } else { Some(199) };
            assert_eq!(runtime.parent_flags, 0x800, "frame {frame}");
            assert_eq!(runtime.parent_effect_phase, expected, "frame {frame}");
            assert_eq!(runtime.parent_outline_alpha, None, "frame {frame}");
        }

        // The write window closes at 150, and `effectPhase` is not cleared per
        // frame in the original, so 199 stays.
        for _ in 0..5000 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.parent_effect_phase, Some(199));
        assert_eq!(runtime.parent_flags, 0x800);
    }

    #[test]
    fn asura_body_monster_skips_the_phase_when_the_flag_is_already_set() {
        let mut body = AsuraBodyMonster;
        let mut runtime = runtime(&body);

        // Called directly: `step` clears the flag word in `begin_step` first.
        runtime.parent_flags = 0x800;
        assert!(body.on_process(&mut runtime));

        assert_eq!(runtime.parent_flags, 0x800);
        assert_eq!(runtime.parent_effect_phase, None);
    }

    // -----------------------------------------------------------------------
    // `.str` bodies
    // -----------------------------------------------------------------------

    #[test]
    fn combo_attack_runs_the_clip_without_a_wave() {
        for &id in ids()
            .iter()
            .filter(|id| !is_caster_body_only(**id) && !matches!(**id, EffectId::Chaincombo))
        {
            let mut body = ComboAttack::new(two_key_clip());
            let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 7);

            for frame in 0..=3 {
                assert_eq!(runtime.frame, frame);
                assert!(step(&mut body, &mut runtime), "{id:?} retired on frame {frame}");
                assert!(runtime.sounds.is_empty(), "{id:?} played a sound on frame {frame}");
                assert!(runtime.positional_sounds.is_empty(), "{id:?} played a positional sound");
                assert_eq!(runtime.shakes, 0, "{id:?} shook the camera");
                assert_eq!(runtime.str_offset, DEFAULT_STR_OFFSET, "{id:?}");
            }

            // Clip keys at frames 0 and 3; the cursor closes on 3, so the body
            // returns `false` on frame 4 - exactly `ProcessEZ2STRC10830()`.
            assert_eq!(runtime.frame, 4);
            assert!(!step(&mut body, &mut runtime), "{id:?}");
            assert_eq!(runtime.frame, 4, "{id:?}");
        }
    }

    #[test]
    fn chain_combo_plays_its_wave_on_frame_zero_only() {
        let mut body = ChainCombo::new(two_key_clip());
        let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 7);

        assert_eq!(body.lifetime(), 300);

        assert!(step(&mut body, &mut runtime));
        assert_eq!(runtime.sounds, vec!["effect\\mon_연환.wav".to_owned()]);
        assert_eq!(runtime.str_offset, DEFAULT_STR_OFFSET);
        runtime.sounds.clear();

        for frame in 1..=3 {
            assert!(step(&mut body, &mut runtime), "retired on frame {frame}");
            assert!(runtime.sounds.is_empty(), "frame {frame}");
        }

        assert_eq!(runtime.frame, 4);
        assert!(!step(&mut body, &mut runtime));
        assert_eq!(runtime.frame, 4);
    }

    #[test]
    fn chain_combo_wave_is_not_repeated_after_the_first_loop() {
        // A looping resource is not configured for `EF_CHAINCOMBO`
        // (`loopLimit = 0`), but the frame-0 gate is per effect lifetime, so
        // re-stepping the same runtime cannot replay the wave either.
        let mut body = ChainCombo::new(two_key_clip());
        let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 7);

        step(&mut body, &mut runtime);
        runtime.sounds.clear();

        step(&mut body, &mut runtime);
        assert!(runtime.sounds.is_empty());
    }

    #[test]
    fn str_bodies_open_no_player_without_clips() {
        // In game the holder opens the resource; without it the body must not
        // panic and must not invent a clip.
        for id in [EffectId::Comboattack1, EffectId::Chaincombo] {
            let mut body = spawn(id).unwrap();
            let mut runtime = runtime(body.as_ref());

            for _ in 0..10 {
                assert!(step(body.as_mut(), &mut runtime), "{id:?}");
            }

            assert!(runtime.str_player.is_none(), "{id:?}");
        }
    }
}
