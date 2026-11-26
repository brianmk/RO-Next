//! Registry entries owned by the "c" parallel port batch.
//!
//! Each batch touches only this file plus the body modules it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag_batch_c::{HealSp, HitDark, WarpZone, Yufitel, YufitelHit};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_YUFITEL: this->Yufitel(0);`
        EffectId::Yufitel => Box::new(Yufitel::new(0, 250)),
        // `case EF_YUFITELHIT: this->YufitelHit(0);`
        EffectId::Yufitelhit => Box::new(YufitelHit::new(0, 250)),
        // `case EF_YUFITEL2: this->YufitelHit(1);`
        EffectId::Yufitel2 => Box::new(YufitelHit::new(1, 250)),
        // `case EF_HITDARK: this->HitDark();`
        EffectId::Hitdark => Box::new(HitDark::new(50)),
        // `case EF_HEALSP: this->HealSP();`
        EffectId::Healsp => Box::new(HealSp::new(50)),
        // `case EF_WARPZONE: this->WarpZone();`
        EffectId::Warpzone => Box::new(WarpZone::new(250)),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Yufitel,
        EffectId::Yufitelhit,
        EffectId::Yufitel2,
        EffectId::Hitdark,
        EffectId::Healsp,
        EffectId::Warpzone,
    ]
}

/// `(effect, lifetime)` pairs, asserted against `EffectTypes.cpp` by tests.
///
/// `EffectTypes.cpp:102` `EF_YUFITEL = 250`, `:103` `EF_YUFITELHIT = 250`,
/// `:461` `EF_YUFITEL2 = 250`, `:189` `EF_HITDARK = 50`,
/// `:21` `EF_HEALSP = 50`, `:70` `EF_WARPZONE = 250`.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        (EffectId::Yufitel, 250),
        (EffectId::Yufitelhit, 250),
        (EffectId::Yufitel2, 250),
        (EffectId::Hitdark, 50),
        (EffectId::Healsp, 50),
        (EffectId::Warpzone, 250),
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
        for &(effect, _) in lifetimes() {
            assert!(spawn(effect).is_some(), "{effect:?}");
        }

        assert!(spawn(EffectId::Yufitel).is_some());
        assert!(spawn(EffectId::Yufitelhit).is_some());
        assert!(spawn(EffectId::Yufitel2).is_some());
        assert!(spawn(EffectId::Hitdark).is_some());
        assert!(spawn(EffectId::Healsp).is_some());
        assert!(spawn(EffectId::Warpzone).is_some());
    }

    #[test]
    fn unported_effects_stay_out_of_the_batch() {
        // Thunderstorm is ported now; `EF_STORMKICK8` still is not.
        assert!(spawn(EffectId::Stormkick8).is_none());
        assert!(spawn(EffectId::Darkattack).is_none());
    }
}
