//! Registry entries owned by the "e" parallel port batch.
//!
//! Every id here is one of the `.str`-only dispatch cases: the effect body is
//! the clip plus (at most) a scheduled wave, so the identity body is the only
//! state - the holder opens the resource from `super::str_table` at spawn.

use ragnarok_packets::EffectId;

use super::rag_batch_e::{
    Angelus, Concentration, Cure, LexDivina, MagicRod, Magnificat, Recovery, Resurrection, StoneCurse, TwoHandQuicken,
};
use super::runtime::PortedEffect;

pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_TWOHANDQUICKEN: case EF_SPEARQUICKEN:
        //  this->TwoHandQuicken();` - `RagEffect.cpp:23963-23966`.
        EffectId::Twohandquicken => Box::new(TwoHandQuicken::new(Vec::new())),
        EffectId::Spearquicken => Box::new(TwoHandQuicken::new(Vec::new())),
        // `case EF_LEXDIVINA: if (!this->LexDivina())` - `RagEffect.cpp:24887`.
        EffectId::Lexdivina => Box::new(LexDivina::new(Vec::new())),
        // `case EF_CONCENTRATION: if (!this->Concentration())` - `:25126`.
        EffectId::Concentration => Box::new(Concentration::new(Vec::new())),
        // `case EF_MAGICROD: this->ProcessEZ2STRC10830(); this->SageMagicRod();`
        // - `:25361-25364`.
        EffectId::Magicrod => Box::new(MagicRod::new(Vec::new())),
        // `case EF_ANGELUS:` wave on frame 0 - `:24770-24777`.
        EffectId::Angelus => Box::new(Angelus::new(Vec::new())),
        // `case EF_CURE:` wave on frame 0 - `:24791-24798`.
        EffectId::Cure => Box::new(Cure::new(Vec::new())),
        // `case EF_MAGNIFICAT:` wave on frame 0 - `:24834-24841`.
        EffectId::Magnificat => Box::new(Magnificat::new(Vec::new())),
        // `case EF_RESURRECTION:` wave on frame 0 - `:24842-24849`.
        EffectId::Resurrection => Box::new(Resurrection::new(Vec::new())),
        // `case EF_RECOVERY:` wave on frame 0 - `:24850-24857`.
        EffectId::Recovery => Box::new(Recovery::new(Vec::new())),
        // `case EF_STONECURSE: case EF_PETRIFYATTACK:` clip only - `:25200-25205`.
        EffectId::Stonecurse => Box::new(StoneCurse::new(Vec::new())),
        _ => return None,
    })
}

pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Twohandquicken,
        EffectId::Spearquicken,
        EffectId::Lexdivina,
        EffectId::Concentration,
        EffectId::Magicrod,
        EffectId::Angelus,
        EffectId::Cure,
        EffectId::Magnificat,
        EffectId::Resurrection,
        EffectId::Recovery,
        EffectId::Stonecurse,
    ]
}

pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:139`.
        (EffectId::Twohandquicken, 9999),
        // `EffectTypes.cpp:259`.
        (EffectId::Spearquicken, 9999),
        // `EffectTypes.cpp:96`.
        (EffectId::Lexdivina, 999),
        // `EffectTypes.cpp:162`.
        (EffectId::Concentration, 999),
        // `EffectTypes.cpp:253`.
        (EffectId::Magicrod, 499),
        // `EffectTypes.cpp:50`.
        (EffectId::Angelus, 999),
        // `EffectTypes.cpp:75`.
        (EffectId::Cure, 999),
        // `EffectTypes.cpp:85`.
        (EffectId::Magnificat, 999),
        // `EffectTypes.cpp:86`.
        (EffectId::Resurrection, 999),
        // `EffectTypes.cpp:87`.
        (EffectId::Recovery, 999),
        // `EffectTypes.cpp:32`.
        (EffectId::Stonecurse, 999),
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
    fn every_batch_id_owns_its_official_str_file() {
        use super::super::str_table::str_clips_for;

        let expect = [
            (EffectId::Twohandquicken, &["TwoHand.str"][..]),
            (EffectId::Spearquicken, &["TwoHand.str"][..]),
            (EffectId::Lexdivina, &["LexDivina.str"][..]),
            (EffectId::Concentration, &["Concentration.str"][..]),
            (EffectId::Magicrod, &["매직로드.str"][..]),
            (EffectId::Angelus, &["Angelus.str"][..]),
            (EffectId::Cure, &["Cure.str"][..]),
            (EffectId::Magnificat, &["Magnificat.str"][..]),
            (EffectId::Resurrection, &["Resurrection.str"][..]),
            (EffectId::Recovery, &["Recovery.str"][..]),
            (EffectId::Stonecurse, &["StoneCurse.str"][..]),
        ];

        assert_eq!(expect.len(), ids().len());

        for (effect, files) in expect {
            assert_eq!(str_clips_for(effect), Some(files), "{effect:?}");
        }
    }

    #[test]
    fn unowned_effects_are_not_claimed() {
        assert!(spawn(EffectId::Bash).is_none());
        // `EF_LKCONCENTRATION` shares `TwoHand.str` but has its own dispatch
        // case (it also calls `SetColor`, which the port does not model yet),
        // so it is not part of this batch.
        assert!(spawn(EffectId::Lkconcentration).is_none());
    }
}
