//! Registry entries owned by the "g" parallel port batch.
//!
//! Each batch touches only this file plus the body module it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag_batch_g::{AbsorbSpirits, Bash3d, Bash3d3, Bash3d4, HitTexture, TrueSight};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_ABSORBSPIRITS:` - `RagEffect.cpp:23122-23128`.
        EffectId::Absorbspirits => Box::new(AbsorbSpirits::new()),
        // `case EF_BASH3D:` - `RagEffect.cpp:10350-10364`.
        EffectId::Bash3d => Box::new(Bash3d::new()),
        // `case EF_BASH3D3:` - `RagEffect.cpp:24256-24270`.
        EffectId::Bash3d3 => Box::new(Bash3d3::new()),
        // `case EF_BASH3D4:` - `RagEffect.cpp:24271-24285`.
        EffectId::Bash3d4 => Box::new(Bash3d4::new()),
        // `case EF_TRUESIGHT:` - `RagEffect.cpp:24286-24294`.
        EffectId::Truesight => Box::new(TrueSight::new()),
        // `case EF_HITTEXTURE:` - `RagEffect.cpp:21741-21743`.
        EffectId::Hittexture => Box::new(HitTexture::new()),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Absorbspirits,
        EffectId::Bash3d,
        EffectId::Bash3d3,
        EffectId::Bash3d4,
        EffectId::Truesight,
        EffectId::Hittexture,
    ]
}

/// `(effect, lifetime)` pairs from `EffectTypes.cpp`; asserted by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:262`
        (EffectId::Absorbspirits, 100),
        // `EffectTypes.cpp:375`
        (EffectId::Bash3d, 200),
        // `EffectTypes.cpp:408`
        (EffectId::Bash3d3, 200),
        // `EffectTypes.cpp:409`
        (EffectId::Bash3d4, 200),
        // `EffectTypes.cpp:395`
        (EffectId::Truesight, 250),
        // `EffectTypes.cpp:563`
        (EffectId::Hittexture, 200),
    ]
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
    fn unowned_effects_are_not_claimed() {
        assert!(spawn(EffectId::Bash).is_none());
        assert!(spawn(EffectId::Napalmvalcan).is_none());
        assert!(spawn(EffectId::PotionBerserk).is_none());
    }
}
