//! Registry entries owned by the "e" parallel port batch.
//!
//! Every id here is one of the `.str`-only dispatch cases: the effect body is
//! the clip plus (at most) a scheduled wave, so the identity body is the only
//! state - the holder opens the resource from `super::str_table` at spawn.

use ragnarok_packets::EffectId;

use super::rag_batch_e::{
    Angelus, Assumptio2, Baku, BrandishSpear, Bullseye, Concentration, Cure, Hyousyouraku, Impositio, Kyrie, LexAeterna, LexDivina,
    LightBolt, MagicRod, Magnificat, MaxPower, Meltdown, Perfection, Provoke, Recovery, RepairWeapon, Resurrection, SlowPoison,
    SpellBreaker, SpringTrap, StoneCurse, StripWeapon, Suffragium, TwoHandQuicken,
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

        // `.str`-only tier follow-up.
        //
        // `case EF_FIREWALL: case EF_LIGHTBOLT:` - `:24615-24620`.
        EffectId::Lightbolt => Box::new(LightBolt::new(Vec::new())),
        // `case EF_PROVOKE:` - `:24799-24809`.
        EffectId::Provoke => Box::new(Provoke::new(Vec::new())),
        // Shared `ProcessEZ2STRC10830` group - `:25278-25322`. Lifetimes differ
        // per label (`EffectTypes.cpp:79/135/152/157-160/178/244/277`).
        EffectId::Brandishspear => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Poisonreact => Box::new(BrandishSpear::new(9999, Vec::new())),
        EffectId::Sonicblow2 => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Pierceself => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Bowlingself => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Spearstabself => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Spearbmrself => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Energycoat => Box::new(BrandishSpear::new(9999, Vec::new())),
        EffectId::Dispell => Box::new(BrandishSpear::new(999, Vec::new())),
        EffectId::Stealcoin => Box::new(BrandishSpear::new(300, Vec::new())),
        // `case EF_IMPOSITIO:` - `:24866-24873`.
        EffectId::Impositio => Box::new(Impositio::new(Vec::new())),
        // `case EF_LEXAETERNA:` - `:24874-24881`.
        EffectId::Lexaeterna => Box::new(LexAeterna::new(Vec::new())),
        // `case EF_SUFFRAGIUM:` - `:24892-24899`.
        EffectId::Suffragium => Box::new(Suffragium::new(Vec::new())),
        // `case EF_REPAIRWEAPON:` - `:24985-24995`.
        EffectId::Repairweapon => Box::new(RepairWeapon::new(Vec::new())),
        // `case EF_PERFECTION:` - `:25001-25008`.
        EffectId::Perfection => Box::new(Perfection::new(Vec::new())),
        // `case EF_MAXPOWER:` - `:25009-25025`.
        EffectId::Maxpower => Box::new(MaxPower::new(Vec::new())),
        // `case EF_SPRINGTRAP:` - `:25050-25057`.
        EffectId::Springtrap => Box::new(SpringTrap::new(Vec::new())),
        // `case EF_KYRIE:` - `:25058-25068`.
        EffectId::Kyrie => Box::new(Kyrie::new(Vec::new())),
        // `case EF_SLOWPOISON: case EF_CARTBOOST: case EF_SOULBURN: case
        //  EF_SOULCHANGE:` - `:23354-23359`; per-id lifetimes
        // (`EffectTypes.cpp:145/400/415/416`).
        EffectId::Slowpoison => Box::new(SlowPoison::new(80, Vec::new())),
        EffectId::Cartboost => Box::new(SlowPoison::new(150, Vec::new())),
        EffectId::Soulburn => Box::new(SlowPoison::new(200, Vec::new())),
        EffectId::Soulchange => Box::new(SlowPoison::new(200, Vec::new())),
        // `case EF_SPELLBREAKER:` - `:25255-25263`.
        EffectId::Spellbreaker => Box::new(SpellBreaker::new(Vec::new())),
        // `case EF_STRIPWEAPON: case EF_STRIPSHIELD: case EF_STRIPARMOR:
        //  case EF_STRIPHELM:` - `:25323-25334`.
        EffectId::Stripweapon | EffectId::Stripshield | EffectId::Striparmor | EffectId::Striphelm => {
            Box::new(StripWeapon::new(Vec::new()))
        }
        // `case EF_MELTDOWN:` - `:23956-23962`.
        EffectId::Meltdown => Box::new(Meltdown::new(Vec::new())),
        // `case EF_ASSUMPTIO2:` - `:24326-24332`.
        EffectId::Assumptio2 => Box::new(Assumptio2::new(Vec::new())),
        // `case EF_BAKU:` / `case EF_HYOUSYOURAKU:` - `OnProcess3`,
        // `:21071-21084`.
        EffectId::Baku => Box::new(Baku::new(Vec::new())),
        EffectId::Hyousyouraku => Box::new(Hyousyouraku::new(Vec::new())),
        // `case EF_BULLSEYE:` group - `OnProcess3`, `:21946-21963`.
        EffectId::Bullseye => Box::new(Bullseye::new(Vec::new())),
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
        // `.str`-only tier follow-up.
        EffectId::Lightbolt,
        EffectId::Provoke,
        EffectId::Brandishspear,
        EffectId::Impositio,
        EffectId::Lexaeterna,
        EffectId::Suffragium,
        EffectId::Repairweapon,
        EffectId::Perfection,
        EffectId::Maxpower,
        EffectId::Springtrap,
        EffectId::Kyrie,
        EffectId::Poisonreact,
        EffectId::Slowpoison,
        EffectId::Sonicblow2,
        EffectId::Pierceself,
        EffectId::Bowlingself,
        EffectId::Spearstabself,
        EffectId::Spearbmrself,
        EffectId::Energycoat,
        EffectId::Spellbreaker,
        EffectId::Dispell,
        EffectId::Stealcoin,
        EffectId::Stripweapon,
        EffectId::Stripshield,
        EffectId::Striparmor,
        EffectId::Striphelm,
        EffectId::Meltdown,
        EffectId::Cartboost,
        EffectId::Soulburn,
        EffectId::Soulchange,
        EffectId::Assumptio2,
        EffectId::Baku,
        EffectId::Hyousyouraku,
        EffectId::Bullseye,
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
        // `.str`-only tier follow-up.
        //
        // `EffectTypes.cpp:38`.
        (EffectId::Lightbolt, 200),
        // `EffectTypes.cpp:76`.
        (EffectId::Provoke, 999),
        // `EffectTypes.cpp:79`.
        (EffectId::Brandishspear, 999),
        // `EffectTypes.cpp:93`.
        (EffectId::Impositio, 999),
        // `EffectTypes.cpp:94`.
        (EffectId::Lexaeterna, 999),
        // `EffectTypes.cpp:97`.
        (EffectId::Suffragium, 999),
        // `EffectTypes.cpp:110`.
        (EffectId::Repairweapon, 9999),
        // `EffectTypes.cpp:112`.
        (EffectId::Perfection, 999),
        // `EffectTypes.cpp:113`.
        (EffectId::Maxpower, 999),
        // `EffectTypes.cpp:118`.
        (EffectId::Springtrap, 9999),
        // `EffectTypes.cpp:121`.
        (EffectId::Kyrie, 999),
        // `EffectTypes.cpp:135`.
        (EffectId::Poisonreact, 9999),
        // `EffectTypes.cpp:145`.
        (EffectId::Slowpoison, 80),
        // `EffectTypes.cpp:152`.
        (EffectId::Sonicblow2, 999),
        // `EffectTypes.cpp:157`.
        (EffectId::Pierceself, 999),
        // `EffectTypes.cpp:158`.
        (EffectId::Bowlingself, 999),
        // `EffectTypes.cpp:159`.
        (EffectId::Spearstabself, 999),
        // `EffectTypes.cpp:160`.
        (EffectId::Spearbmrself, 999),
        // `EffectTypes.cpp:178`.
        (EffectId::Energycoat, 9999),
        // `EffectTypes.cpp:243`.
        (EffectId::Spellbreaker, 999),
        // `EffectTypes.cpp:244`.
        (EffectId::Dispell, 999),
        // `EffectTypes.cpp:277`.
        (EffectId::Stealcoin, 300),
        // `EffectTypes.cpp:278`.
        (EffectId::Stripweapon, 300),
        // `EffectTypes.cpp:279`.
        (EffectId::Stripshield, 300),
        // `EffectTypes.cpp:280`.
        (EffectId::Striparmor, 300),
        // `EffectTypes.cpp:281`.
        (EffectId::Striphelm, 300),
        // `EffectTypes.cpp:399`.
        (EffectId::Meltdown, 250),
        // `EffectTypes.cpp:400`.
        (EffectId::Cartboost, 150),
        // `EffectTypes.cpp:415`.
        (EffectId::Soulburn, 200),
        // `EffectTypes.cpp:416`.
        (EffectId::Soulchange, 200),
        // `EffectTypes.cpp:449`.
        (EffectId::Assumptio2, 300),
        // `EffectTypes.cpp:644`.
        (EffectId::Baku, 100),
        // `EffectTypes.cpp:645`.
        (EffectId::Hyousyouraku, 100),
        // `EffectTypes.cpp:658`.
        (EffectId::Bullseye, 100),
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
            // `.str`-only tier follow-up (`RagEffectInitialization.cpp` rows).
            (EffectId::Lightbolt, &["Lightning.str"][..]),
            (EffectId::Provoke, &["Provoke.str"][..]),
            (EffectId::Brandishspear, &["Brandish.str"][..]),
            (EffectId::Impositio, &["Impositio.str"][..]),
            (EffectId::Lexaeterna, &["LexAeterna.str"][..]),
            (EffectId::Suffragium, &["Suffragium.str"][..]),
            (EffectId::Repairweapon, &["RepairWeapon.str"][..]),
            (EffectId::Perfection, &["WeaponPerfection.str"][..]),
            (EffectId::Maxpower, &["MaximizePower.str"][..]),
            (EffectId::Springtrap, &["Spring.str"][..]),
            (EffectId::Kyrie, &["Kyrie.str"][..]),
            (EffectId::Poisonreact, &["PoisonReact_1st.str"][..]),
            (EffectId::Slowpoison, &["slowp.str"][..]),
            (EffectId::Sonicblow2, &["SonicBlow.str"][..]),
            (EffectId::Pierceself, &["Pierce.str"][..]),
            (EffectId::Bowlingself, &["Bowling.str"][..]),
            (EffectId::Spearstabself, &["SpearStab.str"][..]),
            (EffectId::Spearbmrself, &["SpearBoomerang.str"][..]),
            (EffectId::Energycoat, &["EnergyCoat.str"][..]),
            (EffectId::Spellbreaker, &["spell.str"][..]),
            (EffectId::Dispell, &["디스펠.str"][..]),
            (EffectId::Stealcoin, &["steal_coin.str"][..]),
            (EffectId::Stripweapon, &["strip_weapon.str"][..]),
            (EffectId::Stripshield, &["strip_shield.str"][..]),
            (EffectId::Striparmor, &["strip_armor.str"][..]),
            (EffectId::Striphelm, &["strip_helm.str"][..]),
            (EffectId::Meltdown, &["melt.str"][..]),
            (EffectId::Cartboost, &["cart.str"][..]),
            (EffectId::Soulburn, &["소울번.str"][..]),
            (EffectId::Soulchange, &["사람효과.str"][..]),
            (EffectId::Assumptio2, &["asum.str"][..]),
            (EffectId::Baku, &["fire dragon.str"][..]),
            (EffectId::Hyousyouraku, &["icy.str"][..]),
            (EffectId::Bullseye, &["불스아이.str"][..]),
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
        // case in `rag/lkconcentration.rs` (it also calls `SetColor`), so it
        // is not part of this batch.
        assert!(spawn(EffectId::Stormkick8).is_none());
    }
}
