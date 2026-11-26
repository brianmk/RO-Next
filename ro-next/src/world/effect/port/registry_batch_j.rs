//! Registry entries owned by the "j" parallel port batch.
//!
//! Each batch touches only this file plus the body module it creates, so
//! parallel work cannot collide on `registry.rs`. Every id here is a
//! manual-tier (T3) caster-body effect: it spawns no primitive and opens no
//! `.str` clip, so [`is_caster_body_only`] lets the shared registry invariant
//! accept the empty render set. The shared list is updated once by the lead
//! (the batch-h/batch-i pattern).

use ragnarok_packets::EffectId;

use super::rag_batch_j::{
    BabyBody, BlueBody, FourWayBody, KickedBody, LandBody, MadnessBlue, MadnessRed, PressedBody, QuakeBody, QuakeBody3, QuakeBody4, RedHit,
    RedLightBody, Run, SpinedBody2, TalkFrostJoke, TalkScream,
};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_PRESSEDBODY:` - `RagEffect.cpp:24452-24461`.
        EffectId::Pressedbody => Box::new(PressedBody::new()),
        // `case EF_KICKEDBODY:` - `RagEffect.cpp:24577-24588`.
        EffectId::Kickedbody => Box::new(KickedBody::new()),
        // `case EF_4WAYBODY:` - `RagEffect.cpp:24528-24533`.
        EffectId::_4waybody => Box::new(FourWayBody::new()),
        // `case EF_QUAKEBODY:` - `RagEffect.cpp:24534-24541` (falls through to
        // `EF_QUAKEBODY2`, which shares the body).
        EffectId::Quakebody => Box::new(QuakeBody::new(EffectId::Quakebody)),
        // `case EF_QUAKEBODY2:` - `RagEffect.cpp:24534-24541`.
        EffectId::Quakebody2 => Box::new(QuakeBody::new(EffectId::Quakebody2)),
        // `case EF_QUAKEBODY3:` - `RagEffect.cpp:21760-21765`.
        EffectId::Quakebody3 => Box::new(QuakeBody3::new()),
        // `case EF_QUAKEBODY4:` - `RagEffect.cpp:21766-21780`.
        EffectId::Quakebody4 => Box::new(QuakeBody4::new()),
        // `case EF_LANDBODY:` - `RagEffect.cpp:24551-24560`.
        EffectId::Landbody => Box::new(LandBody::new()),
        // `case EF_SPINEDBODY2:` - `RagEffect.cpp:24561-24570`.
        EffectId::Spinedbody2 => Box::new(SpinedBody2::new()),
        // `case EF_BLUEBODY:` - `RagEffect.cpp:21312-21317`.
        EffectId::Bluebody => Box::new(BlueBody::new()),
        // `case EF_REDLIGHTBODY:` - `RagEffect.cpp:21793-21803`.
        EffectId::Redlightbody => Box::new(RedLightBody::new()),
        // `case EF_RED_HIT:` - `RagEffect.cpp:21781-21787`.
        EffectId::RedHit => Box::new(RedHit::new()),
        // `case EF_MADNESS_BLUE:` - `RagEffect.cpp:21318-21325`.
        EffectId::MadnessBlue => Box::new(MadnessBlue::new()),
        // `case EF_MADNESS_RED:` - `RagEffect.cpp:21326-21333`.
        EffectId::MadnessRed => Box::new(MadnessRed::new()),
        // `case EF_BABYBODY:` - `RagEffect.cpp:24478-24485`.
        EffectId::Babybody => Box::new(BabyBody::new()),
        // `case EF_RUN:` - `RagEffect.cpp:23621-23627`.
        EffectId::Run => Box::new(Run::new()),
        // `case EF_TALK_FROSTJOKE:` - `RagEffect.cpp:24052-24054`.
        EffectId::TalkFrostjoke => Box::new(TalkFrostJoke::new()),
        // `case EF_TALK_SCREAM:` - `RagEffect.cpp:24055-24057`.
        EffectId::TalkScream => Box::new(TalkScream::new()),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Pressedbody,
        EffectId::Kickedbody,
        EffectId::_4waybody,
        EffectId::Quakebody,
        EffectId::Quakebody2,
        EffectId::Quakebody3,
        EffectId::Quakebody4,
        EffectId::Landbody,
        EffectId::Spinedbody2,
        EffectId::Bluebody,
        EffectId::Redlightbody,
        EffectId::RedHit,
        EffectId::MadnessBlue,
        EffectId::MadnessRed,
        EffectId::Babybody,
        EffectId::Run,
        EffectId::TalkFrostjoke,
        EffectId::TalkScream,
    ]
}

/// `(effect, lifetime)` pairs from `EffectTypes.cpp`; asserted by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:422`
        (EffectId::Pressedbody, 50),
        // `EffectTypes.cpp:424`
        (EffectId::Kickedbody, 100),
        // `EffectTypes.cpp:434`
        (EffectId::_4waybody, 100),
        // `EffectTypes.cpp:435`
        (EffectId::Quakebody, 14),
        // `EffectTypes.cpp:467`
        (EffectId::Quakebody2, 35),
        // `EffectTypes.cpp:559`
        (EffectId::Quakebody3, 60),
        // `EffectTypes.cpp:611`
        (EffectId::Quakebody4, 60),
        // `EffectTypes.cpp:455`
        (EffectId::Landbody, 35),
        // `EffectTypes.cpp:475`
        (EffectId::Spinedbody2, 50),
        // `EffectTypes.cpp:551`
        (EffectId::Bluebody, 200),
        // `EffectTypes.cpp:553`
        (EffectId::Redlightbody, 99999999),
        // `EffectTypes.cpp:557`
        (EffectId::RedHit, 60),
        // `EffectTypes.cpp:634`
        (EffectId::MadnessBlue, 60),
        // `EffectTypes.cpp:635`
        (EffectId::MadnessRed, 60),
        // `EffectTypes.cpp:429`
        (EffectId::Babybody, 99999),
        // `EffectTypes.cpp:451`
        (EffectId::Run, 9999),
        // `EffectTypes.cpp:304`
        (EffectId::TalkFrostjoke, 999),
        // `EffectTypes.cpp:305`
        (EffectId::TalkScream, 999),
    ]
}

/// Effects that only touch the caster actor: no primitive, no `.str` clip.
pub fn is_caster_body_only(effect: EffectId) -> bool {
    matches!(
        effect,
        EffectId::Pressedbody
            | EffectId::Kickedbody
            | EffectId::_4waybody
            | EffectId::Quakebody
            | EffectId::Quakebody2
            | EffectId::Quakebody3
            | EffectId::Quakebody4
            | EffectId::Landbody
            | EffectId::Spinedbody2
            | EffectId::Bluebody
            | EffectId::Redlightbody
            | EffectId::RedHit
            | EffectId::MadnessBlue
            | EffectId::MadnessRed
            | EffectId::Babybody
            | EffectId::Run
            | EffectId::TalkFrostjoke
            | EffectId::TalkScream
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
    fn every_registered_id_is_caster_body_only() {
        for &id in ids() {
            assert!(is_caster_body_only(id), "{id:?}");
        }
    }

    #[test]
    fn batch_j_owns_eighteen_ids() {
        assert_eq!(ids().len(), 18);
    }

    #[test]
    fn unowned_effects_are_not_claimed() {
        // The other actor-render-field ids handled elsewhere stay unclaimed:
        // `EF_ASURABODY`/`EF_ASURABODY_MONSTER` are Champion's, and
        // `EF_BABYBODY_BACK`/`EF_BILLBOARD` still need their own fields.
        assert!(spawn(EffectId::Asurabody).is_none());
        assert!(spawn(EffectId::AsurabodyMonster).is_none());
        assert!(spawn(EffectId::BabybodyBack).is_none());
        // A generic non-caster id must not be claimed either.
        assert!(spawn(EffectId::Bash).is_none());
    }
}
