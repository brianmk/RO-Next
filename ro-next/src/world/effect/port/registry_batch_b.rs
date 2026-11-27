//! Registry entries owned by the "b" parallel port batch.
//!
//! Each batch touches only this file plus the body modules it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag::{Hit1, WhiteSparkStreak};
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
        // `case EF_HIT1: this->Hit1();` - `RagEffect.cpp:23098-23099`,
        // lifetime 50 (`EffectTypes.cpp:9`). The standard auto-attack impact.
        EffectId::Hit1 => Box::new(Hit1),
        // `case EF_HITLINE7: this->WhiteSparkStreak(1);` -
        // `RagEffect.cpp:23660-23661`, lifetime 50
        // (`EffectTypes.cpp:443`). The Taekwon/Star hit visual.
        EffectId::Hitline7 => Box::new(WhiteSparkStreak::new(1, 50)),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::SmaReady,
        EffectId::Earthwall,
        EffectId::Hit3,
        EffectId::Hit1,
        EffectId::Hitline7,
    ]
}

/// `(effect, lifetime)` pairs, asserted against `EffectTypes.cpp` by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        (EffectId::SmaReady, 200),
        (EffectId::Earthwall, 100),
        (EffectId::Hit3, 50),
        (EffectId::Hit1, 50),
        (EffectId::Hitline7, 50),
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
        assert!(matches!(spawn(EffectId::Hit1), Some(_)));
        assert!(matches!(spawn(EffectId::Hitline7), Some(_)));
    }
}
