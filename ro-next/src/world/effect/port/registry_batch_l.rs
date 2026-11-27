//! Registry entries owned by the "l" parallel port batch.
//!
//! | effect | id | body | lifetime |
//! | --- | --- | --- | --- |
//! | `EF_THROW_HAPPOKUNAI` | 981 | `CThrowItemEffect` eight-projectile arm | 200 |
//! | `EF_THROW_MULTIPLE_COIN` | 982 | `CThrowItemEffect` ten-projectile arm | 200 |
//! | `EF_THROW_BAKURETSU` | 983 | `CThrowItemEffect` fall-through arm | 200 |
//!
//! These three are the alternate `CThrowItemEffect`/`CParticle` implementation,
//! not `CRagEffect::ThrowItem`: `EffectLists` registers an allocator for each
//! (`EffectLists.cpp:1482-1488`) and `CGameMode::LaunchTargetEffect`
//! (`WorldObjects.cpp:418-431`) spawns them directly. The body is
//! [`ThrowItemParticles`] in `rag_batch_l.rs`.
//!
//! `EffectTypes.cpp:990-992` lists `99999999` for these ids, but that table is
//! not applied to `CEffect` subclasses: `CThrowItemEffect::Open` overwrites
//! `effectLifetime` with 200 (`ThrowEffects.cpp:214`, `:236`) and
//! `IsAlive`/`IsFinished` retires at `frame >= effectLifetime`
//! (`:494-496`, `Effect.cpp:217-218`). The registered lifetime is therefore
//! 200, the number the original actually compares against.
//!
//! The two remaining ids of this class are skipped for concrete missing
//! channels:
//!
//! * `EF_KAIHOU` (989) needs the caster's `charmCount` and `charmType`
//!   (`ThrowEffects.cpp:113-118`, `:249-271`); the runtime has neither, and the
//!   texture is chosen at runtime (`WaterOrb`/`StoneOrb`/`FireOrb`/
//!   `LightningOrb`).
//! * `EF_THROWITEM4_1` (980) is only spawned as a side effect of
//!   `LaunchTargetEffect(EF_THROWITEM4)` (`WorldObjects.cpp:418-431`), so it
//!   needs the spawn path, not another registry row.

use ragnarok_packets::EffectId;

use super::rag_batch_l::ThrowItemParticles;
use super::runtime::PortedEffect;

pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        EffectId::ThrowHappokunai => Box::new(ThrowItemParticles::new(981)),
        EffectId::ThrowMultipleCoin => Box::new(ThrowItemParticles::new(982)),
        EffectId::ThrowBakuretsu => Box::new(ThrowItemParticles::new(983)),
        _ => return None,
    })
}

pub fn ids() -> &'static [EffectId] {
    &[EffectId::ThrowHappokunai, EffectId::ThrowMultipleCoin, EffectId::ThrowBakuretsu]
}

pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `CThrowItemEffect::Open` - `ThrowEffects.cpp:214`
        (EffectId::ThrowHappokunai, 200),
        // `CThrowItemEffect::Open` - `ThrowEffects.cpp:236`
        (EffectId::ThrowMultipleCoin, 200),
        // `CThrowItemEffect::Open` fall-through into `:214`.
        (EffectId::ThrowBakuretsu, 200),
    ]
}

pub fn is_caster_body_only(_effect: EffectId) -> bool {
    false
}

/// The registry's zero-primitive allowance for bodies that draw `CParticle`
/// objects instead of `CEffectPrim` primitives.
///
/// `registry.rs`'s `every_registered_body_assigns_its_official_texture` test
/// checks `runtime.primitives`; the alternate throw path is the first body
/// family with none. This predicate keeps that test honest instead of adding a
/// fake primitive, mirroring `is_caster_body_only`.
pub fn spawns_particles_only(effect: EffectId) -> bool {
    matches!(
        effect,
        EffectId::ThrowHappokunai | EffectId::ThrowMultipleCoin | EffectId::ThrowBakuretsu
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn batch_entries_match_their_lifetimes() {
        for &(effect, lifetime) in lifetimes() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn batch_ids_and_lifetimes_agree() {
        assert_eq!(ids().len(), lifetimes().len());
    }

    #[test]
    fn every_particle_body_creates_particles_with_an_official_texture() {
        for &effect in ids() {
            let mut body = spawn(effect).unwrap();
            let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 1);
            runtime.position = [3.0, 4.0, 5.0];
            runtime.target_offset = [13.0, 8.0, 5.0];

            step(&mut body, &mut runtime);

            assert!(!runtime.particles.is_empty(), "{effect:?} created no particles");
            assert!(runtime.primitives.is_empty(), "{effect:?} must not fake a primitive");

            for particle in &runtime.particles {
                assert!(particle.texture.is_some(), "{effect:?} left a particle without a texture");
            }

            assert!(spawns_particles_only(effect));
        }
    }

    #[test]
    fn spawns_particles_only_is_false_for_every_other_id() {
        assert!(!spawns_particles_only(EffectId::Throwitem));
        assert!(!spawns_particles_only(EffectId::Kaihou));
        assert!(!spawns_particles_only(EffectId::Throwitem4_1));
    }
}
