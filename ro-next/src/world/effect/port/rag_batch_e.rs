//! Effects ported by the "e" parallel batch (`.str`-only tier).
//!
//! Owned exclusively by batch "e": bodies and tests live here.
//!
//! Every effect in this file is a `.str` screen-sprite clip and nothing else:
//! its `OnProcess`/`OnProcess3` case body either calls `ProcessEZ2STR(0, -80,
//! 0)` directly or forwards to one of the small wrappers
//! (`ProcessEZ2STRC10830` / `ProcessEZ2STRC3CC40`, both exactly that call),
//! gates zero or more `PlayEffectWaveInline` calls (or the identical
//! `PlayEffectWave` helper, `RagEffect.cpp:152-158`) on `state.stateCnt`, and
//! launches no primitive.
//! The clip itself is loaded by `CRagEffect::Init` from
//! [`super::str_table`] and opened by [`super::holder::PortedEffectHolder`]
//! before the first step, exactly like the original.
//!
//! The bodies therefore return the *clip's* liveness for the effects whose
//! dispatch checks `ProcessEZ2STR`'s result (they retire when the resource
//! finishes), and `true` for the cases that discard it (`EF_MAGICROD` plus the
//! follow-up `Meltdown`/`Assumptio2`/slow-poison-group/`Baku`/
//! `Hyousyouraku`/`Bullseye` bodies).

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

// ===========================================================================
// `.str`-only tier follow-up
// ===========================================================================
//
// The 34 ids below are the same profile as the tier above. Every section is one
// decompiled case body, one Rust `on_process`; the `.str` resource row lives in
// [`super::str_table`] and is opened by the holder before frame 0.

// ---------------------------------------------------------------------------
// EF_LIGHTBOLT
// ---------------------------------------------------------------------------

/// `case EF_FIREWALL: case EF_LIGHTBOLT:` - `RagEffect.cpp:24615-24620`.
///
/// The case calls `ProcessEZ2STR(0.0f, -80.0f, 0.0f)` directly (not through
/// `ProcessEZ2STRC10830`) and retires with the clip. `EF_LIGHTBOLT` is the only
/// label of this batch; `EF_FIREWALL` is owned elsewhere.
#[derive(Debug, Default)]
pub struct LightBolt {
    clips: Vec<Clip>,
}

impl LightBolt {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for LightBolt {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:38`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_PROVOKE
// ---------------------------------------------------------------------------

/// `case EF_PROVOKE:` - `RagEffect.cpp:24799-24809`.
///
/// `effect\swordman_provoke.wav` plays on frames 5 and 30 (two separate
/// `stateCnt` gates), then the clip owns the effect.
#[derive(Debug, Default)]
pub struct Provoke {
    clips: Vec<Clip>,
}

impl Provoke {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Provoke {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:76`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 5 {
            runtime.play_sound("effect\\swordman_provoke.wav");
        }

        if runtime.frame == 30 {
            runtime.play_sound("effect\\swordman_provoke.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// The shared `ProcessEZ2STRC10830` case body (EF_BRANDISHSPEAR's group)
// ---------------------------------------------------------------------------

/// The shared grouped case body `case EF_WINDHIT: case EF_ARROWSHOT: case
/// EF_BRANDISHSPEAR: ... case EF_STEALCOIN:` - `RagEffect.cpp:25278-25322`.
///
/// All labels fall into one body:
///
/// ```cpp
/// if (!this->ProcessEZ2STRC10830()) {
///   return false;
/// }
/// ```
///
/// with no wave and no state write. `ProcessEZ2STRC10830()` is
/// `ProcessEZ2STR(0.0f, -80.0f, 0.0f)` (`RagEffect.cpp:1823-1825`), so the
/// effect lives exactly as long as its clip.
///
/// This batch owns these labels of the group (the others are either ported by
/// another batch or still unported):
///
/// * `EF_BRANDISHSPEAR` (70), `EF_POISONREACT` (126), `EF_SONICBLOW2` (143),
///   `EF_PIERCESELF` (148), `EF_BOWLINGSELF` (149), `EF_SPEARSTABSELF` (150),
///   `EF_SPEARBMRSELF` (151), `EF_ENERGYCOAT` (169), `EF_DISPELL` (235),
///   `EF_STEALCOIN` (268).
///
/// The labels do not share a lifetime, so the registry passes each id's own
/// `EffectTypes.cpp` value to the constructor.
#[derive(Debug)]
pub struct BrandishSpear {
    clips: Vec<Clip>,
    lifetime: i32,
}

impl BrandishSpear {
    pub fn new(lifetime: i32, clips: Vec<Clip>) -> Self {
        Self { clips, lifetime }
    }
}

impl PortedEffect for BrandishSpear {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_IMPOSITIO
// ---------------------------------------------------------------------------

/// `case EF_IMPOSITIO:` - `RagEffect.cpp:24866-24873`.
///
/// `effect\priest_impositio.wav` plays on frame 60, then the clip owns the
/// effect.
#[derive(Debug, Default)]
pub struct Impositio {
    clips: Vec<Clip>,
}

impl Impositio {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Impositio {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:93`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 60 {
            runtime.play_sound("effect\\priest_impositio.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_LEXAETERNA
// ---------------------------------------------------------------------------

/// `case EF_LEXAETERNA:` - `RagEffect.cpp:24874-24881`.
///
/// `effect\priest_lexaeterna.wav` plays on frame 15. The resource row is the
/// non-`lexaeterna_min` branch of the `TT_MIN_EFFECT_ON_OFF` toggle
/// (`RagEffectInitialization.cpp:84-86`).
#[derive(Debug, Default)]
pub struct LexAeterna {
    clips: Vec<Clip>,
}

impl LexAeterna {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for LexAeterna {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:94`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 15 {
            runtime.play_sound("effect\\priest_lexaeterna.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_SUFFRAGIUM
// ---------------------------------------------------------------------------

/// `case EF_SUFFRAGIUM:` - `RagEffect.cpp:24892-24899`.
///
/// `effect\priest_suffragium.wav` plays on frame 0. The resource row is the
/// non-`suffragium_min` branch (`RagEffectInitialization.cpp:93-95`).
#[derive(Debug, Default)]
pub struct Suffragium {
    clips: Vec<Clip>,
}

impl Suffragium {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Suffragium {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:97`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\priest_suffragium.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_REPAIRWEAPON
// ---------------------------------------------------------------------------

/// `case EF_REPAIRWEAPON:` - `RagEffect.cpp:24985-24995`.
///
/// `effect\black_weapon_repair_a.wav` plays on frames 20 and 55.
#[derive(Debug, Default)]
pub struct RepairWeapon {
    clips: Vec<Clip>,
}

impl RepairWeapon {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for RepairWeapon {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:110`.
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 20 {
            runtime.play_sound("effect\\black_weapon_repair_a.wav");
        }

        if runtime.frame == 55 {
            runtime.play_sound("effect\\black_weapon_repair_a.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_PERFECTION
// ---------------------------------------------------------------------------

/// `case EF_PERFECTION:` - `RagEffect.cpp:25001-25008`.
///
/// `effect\black_weapon_perfection.wav` plays on frame 0. The resource row is
/// the non-`WeaponPerfection_min` branch
/// (`RagEffectInitialization.cpp:123-125`).
#[derive(Debug, Default)]
pub struct Perfection {
    clips: Vec<Clip>,
}

impl Perfection {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Perfection {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:112`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\black_weapon_perfection.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_MAXPOWER
// ---------------------------------------------------------------------------

/// `case EF_MAXPOWER:` - `RagEffect.cpp:25009-25025`.
///
/// Four gates, in source order: frame 0 the circle wave, 40 and 47 the sword
/// wave, 65 the big-sword wave. Resource row: non-`maximize_min` branch
/// (`RagEffectInitialization.cpp:126-128`).
#[derive(Debug, Default)]
pub struct MaxPower {
    clips: Vec<Clip>,
}

impl MaxPower {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for MaxPower {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:113`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\black_maximize_power_circle.wav");
        }

        if runtime.frame == 40 {
            runtime.play_sound("effect\\black_maximize_power_sword.wav");
        }

        if runtime.frame == 47 {
            runtime.play_sound("effect\\black_maximize_power_sword.wav");
        }

        if runtime.frame == 65 {
            runtime.play_sound("effect\\black_maximize_power_sword_bic.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_SPRINGTRAP
// ---------------------------------------------------------------------------

/// `case EF_SPRINGTRAP:` - `RagEffect.cpp:25050-25057`.
///
/// `effect\hunter_springtrap.wav` plays on frame 0.
#[derive(Debug, Default)]
pub struct SpringTrap {
    clips: Vec<Clip>,
}

impl SpringTrap {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for SpringTrap {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:118`.
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\hunter_springtrap.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_KYRIE
// ---------------------------------------------------------------------------

/// `case EF_KYRIE:` - `RagEffect.cpp:25058-25068`.
///
/// Two different waves: `priest_kyrie_eleison_b.wav` on frame 0 and
/// `priest_kyrie_eleison_a.wav` on frame 15. Resource row: non-`kyrie_min`
/// branch (`RagEffectInitialization.cpp:147-149`).
#[derive(Debug, Default)]
pub struct Kyrie {
    clips: Vec<Clip>,
}

impl Kyrie {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Kyrie {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:121`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\priest_kyrie_eleison_b.wav");
        }

        if runtime.frame == 15 {
            runtime.play_sound("effect\\priest_kyrie_eleison_a.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// The shared no-wave discard body (EF_SLOWPOISON's group)
// ---------------------------------------------------------------------------

/// `case EF_SLOWPOISON: case EF_CARTBOOST: case EF_SOULBURN: case
/// EF_SOULCHANGE:` - `RagEffect.cpp:23354-23359`.
///
/// ```cpp
/// this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);
/// ```
///
/// The result is **discarded**, so a finished clip does not retire the effect;
/// it runs until its lifetime (`stateCnt >= duration`). Lifetimes differ across
/// the labels (80 / 150 / 200 / 200), so the registry passes each id's own
/// `EffectTypes.cpp` value.
#[derive(Debug)]
pub struct SlowPoison {
    clips: Vec<Clip>,
    lifetime: i32,
}

impl SlowPoison {
    pub fn new(lifetime: i32, clips: Vec<Clip>) -> Self {
        Self { clips, lifetime }
    }
}

impl PortedEffect for SlowPoison {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - result discarded.
        let _clip_alive = runtime.process_str();

        true
    }
}

// ---------------------------------------------------------------------------
// EF_SPELLBREAKER
// ---------------------------------------------------------------------------

/// `case EF_SPELLBREAKER:` - `RagEffect.cpp:25255-25263`.
///
/// Frame 0 re-anchors `render.position` to the parent (the port spawns both at
/// the same position, so there is nothing to write) and plays
/// `effect\sage_spell breake.wav`; then `ProcessEZ2STRC10830` is checked.
#[derive(Debug, Default)]
pub struct SpellBreaker {
    clips: Vec<Clip>,
}

impl SpellBreaker {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for SpellBreaker {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:243`.
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            // `this->render.position = this->state.parent->render.position;`
            // is already the spawn anchor (`PortedEffectHolder`).
            runtime.play_sound("effect\\sage_spell breake.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// The shared strip case body (EF_STRIPWEAPON's group)
// ---------------------------------------------------------------------------

/// `case EF_STRIPWEAPON: case EF_STRIPSHIELD: case EF_STRIPARMOR: case
/// EF_STRIPHELM:` - `RagEffect.cpp:25323-25334`.
///
/// Frame 6 re-anchors to the parent (same spawn position in the port) and plays
/// `effect\복호격.wav`; then `ProcessEZ2STRC10830` is checked. All four labels
/// use lifetime 300 (`EffectTypes.cpp:278-281`).
#[derive(Debug, Default)]
pub struct StripWeapon {
    clips: Vec<Clip>,
}

impl StripWeapon {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for StripWeapon {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:278-281` (all four strip ids).
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 6 {
            // `this->render.position = this->state.parent->render.position;`
            // is already the spawn anchor (`PortedEffectHolder`).
            runtime.play_sound("effect\\복호격.wav");
        }

        runtime.process_str()
    }
}

// ---------------------------------------------------------------------------
// EF_MELTDOWN
// ---------------------------------------------------------------------------

/// `case EF_MELTDOWN:` - `RagEffect.cpp:23956-23962`.
///
/// Frame 0 re-anchors to the parent (already the spawn position) and plays
/// `effect\black_overthrust.wav`; `ProcessEZ2STR(0, -80, 0)` then runs with its
/// result **discarded**, so the effect keeps going after the clip ends.
#[derive(Debug, Default)]
pub struct Meltdown {
    clips: Vec<Clip>,
}

impl Meltdown {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Meltdown {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:399`.
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\black_overthrust.wav");
        }

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - result discarded.
        let _clip_alive = runtime.process_str();

        true
    }
}

// ---------------------------------------------------------------------------
// EF_ASSUMPTIO2
// ---------------------------------------------------------------------------

/// `case EF_ASSUMPTIO2:` - `RagEffect.cpp:24326-24332`.
///
/// Frame 0 re-anchors to the parent (already the spawn position) and plays
/// `effect\아숨프티오.wav`; the `ProcessEZ2STR(0, -80, 0)` result is discarded.
#[derive(Debug, Default)]
pub struct Assumptio2 {
    clips: Vec<Clip>,
}

impl Assumptio2 {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Assumptio2 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:449`.
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\아숨프티오.wav");
        }

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - result discarded.
        let _clip_alive = runtime.process_str();

        true
    }
}

// ---------------------------------------------------------------------------
// EF_BAKU / EF_HYOUSYOURAKU
// ---------------------------------------------------------------------------

/// `case EF_BAKU:` - `RagEffect.cpp:21071-21077` (in `OnProcess3`, which
/// `OnProcess` calls for ids >= 491, `RagEffect.cpp:23091-23094`).
///
/// Frame 0 re-anchors to the parent (already the spawn position) and plays
/// `effect\폭염룡.wav` via `PlayEffectWave`; that helper has the same body as
/// `PlayEffectWaveInline` (`RagEffect.cpp:152-158`), so the port's
/// `play_sound` models both. The `ProcessEZ2STR(0, -80, 0)` result is
/// discarded.
#[derive(Debug, Default)]
pub struct Baku {
    clips: Vec<Clip>,
}

impl Baku {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Baku {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:644`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\폭염룡.wav");
        }

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - result discarded.
        let _clip_alive = runtime.process_str();

        true
    }
}

/// `case EF_HYOUSYOURAKU:` - `RagEffect.cpp:21078-21084` (in `OnProcess3`).
///
/// Frame 0 plays `effect\빙정락.wav` through the same `PlayEffectWave` helper
/// as `EF_BAKU`; the clip result is discarded.
#[derive(Debug, Default)]
pub struct Hyousyouraku {
    clips: Vec<Clip>,
}

impl Hyousyouraku {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Hyousyouraku {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:645`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        if runtime.frame == 0 {
            runtime.play_sound("effect\\빙정락.wav");
        }

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - result discarded.
        let _clip_alive = runtime.process_str();

        true
    }
}

// ---------------------------------------------------------------------------
// EF_BULLSEYE
// ---------------------------------------------------------------------------

/// `case EF_MOCHI: ... case EF_BULLSEYE: ... case EF_NO100_FIRECRACKER:` -
/// `RagEffect.cpp:21946-21963` (in `OnProcess3`).
///
/// One shared body:
///
/// ```cpp
/// this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);
/// ```
///
/// with the result **discarded**. Only `EF_BULLSEYE` (649) belongs to this
/// batch; the other labels are not ported here.
#[derive(Debug, Default)]
pub struct Bullseye {
    clips: Vec<Clip>,
}

impl Bullseye {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Bullseye {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:658`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        open_supplied_clips(&mut self.clips, runtime);

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - result discarded.
        let _clip_alive = runtime.process_str();

        true
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

    // -----------------------------------------------------------------------
    // `.str`-only tier follow-up
    // -----------------------------------------------------------------------

    /// One fresh body per scheduled-wave case, with its lifetime and the
    /// `(frame, wave)` rows of the decompiled body.
    fn scheduled_bodies() -> Vec<(Box<dyn PortedEffect>, i32, Vec<(i32, &'static str)>)> {
        vec![
            (Box::new(Provoke::new(single_key_clip())), 999, vec![
                (5, "effect\\swordman_provoke.wav"),
                (30, "effect\\swordman_provoke.wav"),
            ]),
            (Box::new(Impositio::new(single_key_clip())), 999, vec![(
                60,
                "effect\\priest_impositio.wav",
            )]),
            (Box::new(LexAeterna::new(single_key_clip())), 999, vec![(
                15,
                "effect\\priest_lexaeterna.wav",
            )]),
            (Box::new(Suffragium::new(single_key_clip())), 999, vec![(
                0,
                "effect\\priest_suffragium.wav",
            )]),
            (Box::new(RepairWeapon::new(single_key_clip())), 9999, vec![
                (20, "effect\\black_weapon_repair_a.wav"),
                (55, "effect\\black_weapon_repair_a.wav"),
            ]),
            (Box::new(Perfection::new(single_key_clip())), 999, vec![(
                0,
                "effect\\black_weapon_perfection.wav",
            )]),
            (Box::new(MaxPower::new(single_key_clip())), 999, vec![
                (0, "effect\\black_maximize_power_circle.wav"),
                (40, "effect\\black_maximize_power_sword.wav"),
                (47, "effect\\black_maximize_power_sword.wav"),
                (65, "effect\\black_maximize_power_sword_bic.wav"),
            ]),
            (Box::new(SpringTrap::new(single_key_clip())), 9999, vec![(
                0,
                "effect\\hunter_springtrap.wav",
            )]),
            (Box::new(Kyrie::new(single_key_clip())), 999, vec![
                (0, "effect\\priest_kyrie_eleison_b.wav"),
                (15, "effect\\priest_kyrie_eleison_a.wav"),
            ]),
            (Box::new(SpellBreaker::new(single_key_clip())), 999, vec![(
                0,
                "effect\\sage_spell breake.wav",
            )]),
            (Box::new(StripWeapon::new(single_key_clip())), 300, vec![(
                6,
                "effect\\복호격.wav",
            )]),
            (Box::new(Meltdown::new(single_key_clip())), 250, vec![(
                0,
                "effect\\black_overthrust.wav",
            )]),
            (Box::new(Assumptio2::new(single_key_clip())), 300, vec![(
                0,
                "effect\\아숨프티오.wav",
            )]),
            (Box::new(Baku::new(single_key_clip())), 100, vec![(0, "effect\\폭염룡.wav")]),
            (Box::new(Hyousyouraku::new(single_key_clip())), 100, vec![(
                0,
                "effect\\빙정락.wav",
            )]),
        ]
    }

    #[test]
    fn the_follow_up_waves_play_on_the_official_frames() {
        for (mut body, lifetime, expected) in scheduled_bodies() {
            assert_eq!(body.lifetime(), lifetime);

            let mut runtime = runtime(lifetime);
            let last_frame = expected.iter().map(|&(frame, _)| frame).max().unwrap();
            let mut schedule = Vec::new();

            for _ in 0..=last_frame {
                runtime.sounds.clear();
                let frame = runtime.frame;
                step(&mut *body, &mut runtime);

                if !runtime.sounds.is_empty() {
                    schedule.push((frame, runtime.sounds.clone()));
                }
            }

            let expected: Vec<(i32, Vec<String>)> = expected.into_iter().map(|(frame, wave)| (frame, vec![wave.to_owned()])).collect();
            assert_eq!(schedule, expected);
        }
    }

    #[test]
    fn the_checked_follow_up_bodies_retire_with_their_clip() {
        let bodies: Vec<(Box<dyn PortedEffect>, &str)> = vec![
            (Box::new(LightBolt::new(two_key_clip())), "LightBolt"),
            (Box::new(BrandishSpear::new(999, two_key_clip())), "BrandishSpear"),
            (Box::new(Provoke::new(two_key_clip())), "Provoke"),
            (Box::new(Impositio::new(two_key_clip())), "Impositio"),
            (Box::new(LexAeterna::new(two_key_clip())), "LexAeterna"),
            (Box::new(Suffragium::new(two_key_clip())), "Suffragium"),
            (Box::new(RepairWeapon::new(two_key_clip())), "RepairWeapon"),
            (Box::new(Perfection::new(two_key_clip())), "Perfection"),
            (Box::new(MaxPower::new(two_key_clip())), "MaxPower"),
            (Box::new(SpringTrap::new(two_key_clip())), "SpringTrap"),
            (Box::new(Kyrie::new(two_key_clip())), "Kyrie"),
            (Box::new(SpellBreaker::new(two_key_clip())), "SpellBreaker"),
            (Box::new(StripWeapon::new(two_key_clip())), "StripWeapon"),
        ];

        for (mut body, label) in bodies {
            let mut runtime = runtime(body.lifetime());

            // Clip frames 0..=3, the cursor closes at 3, `ProcessEZ2STR` fails
            // on 4 and the body turns that into `return false`.
            for _ in 0..4 {
                assert!(step(&mut *body, &mut runtime), "{label}");
            }

            assert_eq!(runtime.frame, 4, "{label}");
            assert!(!step(&mut *body, &mut runtime), "{label} retires on frame 4");
        }
    }

    #[test]
    fn the_discarded_follow_up_bodies_run_past_the_clip() {
        let bodies: Vec<(Box<dyn PortedEffect>, &str)> = vec![
            (Box::new(Meltdown::new(two_key_clip())), "Meltdown"),
            (Box::new(Assumptio2::new(two_key_clip())), "Assumptio2"),
            (Box::new(SlowPoison::new(80, two_key_clip())), "SlowPoison"),
            (Box::new(Bullseye::new(two_key_clip())), "Bullseye"),
            (Box::new(Baku::new(two_key_clip())), "Baku"),
            (Box::new(Hyousyouraku::new(two_key_clip())), "Hyousyouraku"),
        ];

        for (mut body, label) in bodies {
            let mut runtime = runtime(body.lifetime());

            // The clip finishes at frame 4; the frame-4 `false` is discarded,
            // so the effect keeps stepping until its lifetime.
            for frame in 0..=6 {
                assert!(step(&mut *body, &mut runtime), "{label} frame {frame}");
            }
        }
    }

    #[test]
    fn the_follow_up_tier_launches_no_primitives() {
        let bodies: Vec<Box<dyn PortedEffect>> = vec![
            Box::new(LightBolt::new(single_key_clip())),
            Box::new(Provoke::new(single_key_clip())),
            Box::new(BrandishSpear::new(999, single_key_clip())),
            Box::new(Impositio::new(single_key_clip())),
            Box::new(LexAeterna::new(single_key_clip())),
            Box::new(Suffragium::new(single_key_clip())),
            Box::new(RepairWeapon::new(single_key_clip())),
            Box::new(Perfection::new(single_key_clip())),
            Box::new(MaxPower::new(single_key_clip())),
            Box::new(SpringTrap::new(single_key_clip())),
            Box::new(Kyrie::new(single_key_clip())),
            Box::new(SlowPoison::new(80, single_key_clip())),
            Box::new(SpellBreaker::new(single_key_clip())),
            Box::new(StripWeapon::new(single_key_clip())),
            Box::new(Meltdown::new(single_key_clip())),
            Box::new(Assumptio2::new(single_key_clip())),
            Box::new(Baku::new(single_key_clip())),
            Box::new(Hyousyouraku::new(single_key_clip())),
            Box::new(Bullseye::new(single_key_clip())),
        ];

        for mut body in bodies {
            let mut runtime = runtime(body.lifetime());

            for _ in 0..80 {
                step(&mut *body, &mut runtime);
            }

            assert!(runtime.primitives.is_empty(), "a clip-only body spawned geometry");
        }
    }
}
