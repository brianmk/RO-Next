//! Registry entries owned by the "b" parallel port batch.
//!
//! Each batch touches only this file plus the body modules it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag_batch_b::{Hit3, ParticleTime, StoneWall};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_SMA_READY: this->ParticleTime("effect\\white02.bmp", 2);`
        EffectId::SmaReady => Box::new(ParticleTime::new("effect\\white02.bmp", 2, 200)),
        // `case EF_EARTHWALL: this->StoneWall();`
        EffectId::Earthwall => Box::new(StoneWall::new(100)),
        // `case EF_HIT3: this->Hit3();`
        EffectId::Hit3 => Box::new(Hit3::new(50)),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[EffectId::SmaReady, EffectId::Earthwall, EffectId::Hit3]
}

/// `(effect, lifetime)` pairs, asserted against `EffectTypes.cpp` by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        (EffectId::SmaReady, 200),
        (EffectId::Earthwall, 100),
        (EffectId::Hit3, 50),
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
    fn batch_bodies_are_the_official_helpers() {
        assert!(matches!(spawn(EffectId::SmaReady), Some(_)));
        assert!(matches!(spawn(EffectId::Earthwall), Some(_)));
        assert!(matches!(spawn(EffectId::Hit3), Some(_)));
    }
}
