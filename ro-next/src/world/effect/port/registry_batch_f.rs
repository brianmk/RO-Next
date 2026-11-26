//! Registry entries owned by the "f" parallel port batch.
//!
//! Each batch touches only this file plus the body module it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag_batch_f::{HasteUp, RisingSparkles, Sightrasher, Thunderstorm2, TurnUndead};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_SIGHTRASHER: this->Sightrasher();` - `RagEffect.cpp:23855-23857`.
        EffectId::Sightrasher => Box::new(Sightrasher::default()),
        // `case EF_TURNUNDEAD: this->TurnUndead();` - `RagEffect.cpp:23901-23903`.
        EffectId::Turnundead => Box::new(TurnUndead::default()),
        // `case EF_HASTEUP: this->HasteUp();` - `RagEffect.cpp:23913-23915`.
        EffectId::Hasteup => Box::new(HasteUp::default()),
        // `case EF_DANCE2:` tint + `this->RisingSparkles(2);` -
        // `RagEffect.cpp:20108-20121`.
        EffectId::Dance2 => Box::new(RisingSparkles::new(2)),
        // `case EF_THUNDERSTORM2:` inline body - `RagEffect.cpp:21134-21147`.
        EffectId::Thunderstorm2 => Box::new(Thunderstorm2::default()),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Sightrasher,
        EffectId::Turnundead,
        EffectId::Hasteup,
        EffectId::Dance2,
        EffectId::Thunderstorm2,
    ]
}

/// `(effect, lifetime)` pairs from `EffectTypes.cpp`; asserted by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:71`
        (EffectId::Sightrasher, 250),
        // `EffectTypes.cpp:91`
        (EffectId::Turnundead, 250),
        // `EffectTypes.cpp:107`
        (EffectId::Hasteup, 250),
        // `EffectTypes.cpp:857`
        (EffectId::Dance2, 100),
        // `EffectTypes.cpp:631`
        (EffectId::Thunderstorm2, 100),
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
        assert!(spawn(EffectId::PotionBerserk).is_none());
    }
}
