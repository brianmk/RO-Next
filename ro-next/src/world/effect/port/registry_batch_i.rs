//! Registry entries owned by the "i" parallel port batch.
//!
//! Each batch touches only this file plus the body module it creates, so
//! parallel work cannot collide on `registry.rs`. Every id here is a
//! manual-tier (T3) caster-body effect: it spawns no primitive and opens no
//! `.str` clip, so [`is_caster_body_only`] lets the shared registry invariant
//! accept the empty render set.

use ragnarok_packets::EffectId;

use super::rag_batch_i::{Assumptio, ChemicalBody, DoubleCastBody, FalconAssault, MagicCrasher2, PinkBody, RedBody, ReflectBody, Shrink};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_REDBODY:` - `RagEffect.cpp:10371-10376`.
        EffectId::Redbody => Box::new(RedBody::new()),
        // `case EF_ASSUMPTIO:` - `RagEffect.cpp:24321-24325`.
        EffectId::Assumptio => Box::new(Assumptio::new()),
        // `case EF_REFLECTBODY:` - `RagEffect.cpp:24509-24513`.
        EffectId::Reflectbody => Box::new(ReflectBody::new()),
        // `case EF_PINKBODY:` - `RagEffect.cpp:24443-24451`.
        EffectId::Pinkbody => Box::new(PinkBody::new()),
        // `case EF_MAGICCRASHER2:` - `RagEffect.cpp:23773-23781`.
        EffectId::Magiccrasher2 => Box::new(MagicCrasher2::new()),
        // `case EF_CHEMICALBODY:` - `RagEffect.cpp:21703-21717`.
        EffectId::Chemicalbody => Box::new(ChemicalBody::new()),
        // `case EF_DOUBLECASTBODY:` - `RagEffect.cpp:21462-21476`.
        EffectId::Doublecastbody => Box::new(DoubleCastBody::new()),
        // `case EF_SHRINK:` - `RagEffect.cpp:21536-21550`.
        EffectId::Shrink => Box::new(Shrink::new()),
        // `case EF_FALCONASSAULT:` - `RagEffect.cpp:23782-23797`.
        EffectId::Falconassault => Box::new(FalconAssault::new()),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Redbody,
        EffectId::Assumptio,
        EffectId::Reflectbody,
        EffectId::Pinkbody,
        EffectId::Magiccrasher2,
        EffectId::Chemicalbody,
        EffectId::Doublecastbody,
        EffectId::Shrink,
        EffectId::Falconassault,
    ]
}

/// `(effect, lifetime)` pairs from `EffectTypes.cpp`; asserted by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:377`
        (EffectId::Redbody, 99999),
        // `EffectTypes.cpp:384`
        (EffectId::Assumptio, 99999),
        // `EffectTypes.cpp:428`
        (EffectId::Reflectbody, 99999999),
        // `EffectTypes.cpp:405`
        (EffectId::Pinkbody, 99999999),
        // `EffectTypes.cpp:412`
        (EffectId::Magiccrasher2, 100),
        // `EffectTypes.cpp:509`
        (EffectId::Chemicalbody, 120),
        // `EffectTypes.cpp:530`
        (EffectId::Doublecastbody, 120),
        // `EffectTypes.cpp:608`
        (EffectId::Shrink, 200),
        // `EffectTypes.cpp:396`
        (EffectId::Falconassault, 200),
    ]
}

/// Effects that only touch the caster actor: no primitive, no `.str` clip.
pub fn is_caster_body_only(effect: EffectId) -> bool {
    matches!(
        effect,
        EffectId::Redbody
            | EffectId::Assumptio
            | EffectId::Reflectbody
            | EffectId::Pinkbody
            | EffectId::Magiccrasher2
            | EffectId::Chemicalbody
            | EffectId::Doublecastbody
            | EffectId::Shrink
            | EffectId::Falconassault
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn caster_body_only_ids_are_registered() {
        for &id in ids() {
            assert!(is_caster_body_only(id), "{id:?}");
        }
    }

    #[test]
    fn unowned_effects_are_not_claimed() {
        // `EF_ASSUMPTIO2` (24326) has a `.str` body, not a caster-body one.
        assert!(spawn(EffectId::Assumptio2).is_none());
        // `EF_SU_STOOP` (1132) is the one manual-tier id with no `EffectId`
        // variant; its neighbours are not claimed either.
        assert!(spawn(EffectId::SuGrooming).is_none());
        assert!(spawn(EffectId::SuChattering).is_none());
        // Owned elsewhere (registry.rs's hand-written table / batch "h").
        assert!(spawn(EffectId::Magiccrasher).is_none());
        assert!(spawn(EffectId::Reflectshield).is_none());
    }
}
