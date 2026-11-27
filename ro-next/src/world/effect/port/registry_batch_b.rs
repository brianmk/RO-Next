//! Registry entries owned by the "b" parallel port batch.
//!
//! Each batch touches only this file plus the body modules it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag::{Hit1, ViolenHit, WhiteSparkStreak};
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
        // `EF_VIOLEN_HIT` is a `CSpriteEffect`
        // (`EffectLists.cpp:5129`): its body is the sprite setup
        // (`SpriteEffect.cpp:65-75`, `:289-295`), lifetime 100
        // (`SpriteEffect.cpp:73`). The Violen job hit visual.
        EffectId::ViolenHit => Box::new(ViolenHit),
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
        EffectId::ViolenHit,
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
        (EffectId::ViolenHit, 100),
    ]
}

#[cfg(test)]
mod tests {
    use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};
    use ragnarok_packets::EffectId;

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

    /// `#[numeric_value]` drives the wire encoding, not the Rust
    /// discriminant: `EF_VIOLEN_HIT` sits between `EF_BEGINSPELL_RING_YELLOW`
    /// (2367) and `EF_HEAL_BLUE_2` (2427), so the neighbours must not shift.
    #[test]
    fn violen_hit_keeps_its_official_wire_id() {
        fn encoded(effect: EffectId) -> [u8; 4] {
            let mut writer = ByteWriter::new();
            effect.to_bytes(&mut writer).expect("EffectId encodes");

            writer.as_slice().try_into().expect("EffectId is a u32")
        }

        fn decoded(value: u32) -> EffectId {
            let bytes = value.to_le_bytes();
            EffectId::from_bytes(&mut ByteReader::without_metadata(&bytes)).expect("id is known")
        }

        // `EffectTypes.h:2428`.
        assert_eq!(encoded(EffectId::ViolenHit), 2420u32.to_le_bytes());
        assert!(matches!(decoded(2420), EffectId::ViolenHit));

        // Neighbours must not have shifted.
        assert_eq!(encoded(EffectId::BeginspellRingYellow), 2367u32.to_le_bytes());
        assert_eq!(encoded(EffectId::HealBlue2), 2427u32.to_le_bytes());
        assert_eq!(encoded(EffectId::RgCoinBlue2), 2437u32.to_le_bytes());
    }

    #[test]
    fn batch_bodies_are_the_official_helpers() {
        assert!(matches!(spawn(EffectId::SmaReady), Some(_)));
        assert!(matches!(spawn(EffectId::Earthwall), Some(_)));
        assert!(matches!(spawn(EffectId::Hit3), Some(_)));
        assert!(matches!(spawn(EffectId::Hit1), Some(_)));
        assert!(matches!(spawn(EffectId::Hitline7), Some(_)));
        assert!(matches!(spawn(EffectId::ViolenHit), Some(_)));
    }
}
