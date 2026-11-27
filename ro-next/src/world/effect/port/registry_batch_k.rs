//! Registry entries owned by the "k" parallel port batch.
//!
//! Every id here was previously unregistrable: the official effect body was
//! already ported (or, for [`PeongBlack`], is the kind-1 arm of an already
//! ported shared body), but the id was missing from `EffectId`. The variants
//! were added with their real `#[numeric_value]` so no existing id shifted.
//!
//! | effect | id | `EffectTypes.h` | dispatch | body |
//! | --- | --- | --- | --- | --- |
//! | `EF_BLOODDRAIN2` | 1052 | `:1060` | `RagEffect.cpp:22115-22117` | `RedParticleSpray(1)` (ported for 216) |
//! | `EF_PEONG_BLACK` | 1207 | `:1215` | `:22254-22261` | `SonicSlashFan(1)` + tint (`rag_batch_k.rs`) |
//! | `EF_NEW_HEAVENS_DRIVE` | 1399 | `:1407` | `:22325-22327` | `StoneSpike()` (ported for 142) |
//! | `EF_HEAL_BLUE_2` | 2427 | `:2435` | `:22886-22891` | `HealRingSparkle("ring_blue", 2)` (kind 0 is `EF_HEAL2`) |
//! | `EF_RG_COIN_BLUE` | 2436 | `:2444` | `:22916-22940` | `RogueCoin::coin_blue` |
//! | `EF_RG_COIN_BLUE_2` | 2437 | `:2445` | `:22941-22965` | `RogueCoin::coin_blue_2` |
//!
//! The 21 ring aliases registered after those are pure
//! `TexturedQuadRingSized` dispatch arms; see [`SizedRing`] and the table in
//! `rag_batch_k.rs`.
//!
//! None of these is caster-body only: each either launches primitives
//! (`PeongBlack`, `StoneSpike`, `HealRingSparkle`, the coin sprays, the ring
//! aliases) or a particle-spline fan (`Blooddrain2`), so
//! [`is_caster_body_only`] stays false.

use ragnarok_packets::EffectId;

use super::rag::{HealRingSparkle, RedParticleSpray, RogueCoin, StoneSpike};
use super::rag_batch_k::{PeongBlack, RingSound, SizedRing};
use super::runtime::PortedEffect;

pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_BLOODDRAIN2:` - `RagEffect.cpp:22115-22117`.
        EffectId::Blooddrain2 => Box::new(RedParticleSpray::new(1, 200)),
        // `case EF_PEONG_BLACK:` - `RagEffect.cpp:22254-22261`.
        EffectId::PeongBlack => Box::new(PeongBlack::new(200)),
        // `case EF_NEW_HEAVENS_DRIVE:` - `RagEffect.cpp:22325-22327`.
        EffectId::NewHeavensDrive => Box::new(StoneSpike::new(75)),
        // `case EF_HEAL_BLUE_2:` - `RagEffect.cpp:22886-22891`.
        EffectId::HealBlue2 => Box::new(HealRingSparkle::new("effect\\ring_blue.tga", 2, 100)),
        // `case EF_RG_COIN_BLUE:` - `RagEffect.cpp:22916-22940`.
        EffectId::RgCoinBlue => Box::new(RogueCoin::coin_blue(3000)),
        // `case EF_RG_COIN_BLUE_2:` - `RagEffect.cpp:22941-22965`.
        EffectId::RgCoinBlue2 => Box::new(RogueCoin::coin_blue_2(3000)),
        // `case EF_NEW_MOON_SLASH:` - `RagEffect.cpp:22334-22341`.
        // Lifetime 1418 (`EffectTypes.cpp:1427`).
        EffectId::NewMoonSlash => Box::new(SizedRing::new(1418, RingSound::Wave, &[(
            45,
            "effect\\new_comet\\new_comet_cast_ring\\ring_rainbow.tga",
            20,
        )])),
        // `case EF_NEW_HEAL_BLUE:` - `RagEffect.cpp:22419-22431`.
        // Lifetime 60 (`EffectTypes.cpp:1565`).
        EffectId::NewHealBlue => Box::new(SizedRing::new(60, RingSound::MinusOneHundred, &[
            (45, "effect\\ring_blue.tga", 20),
            (25, "effect\\ring_blue.tga", 21),
        ])),
        // `case EF_NEW_HEAL_WHITE:` - `RagEffect.cpp:22457-22464`.
        // Lifetime 40 (`EffectTypes.cpp:1569`); the helper raises the duration
        // to 56 and zeroes the ring alpha.
        EffectId::NewHealWhite => Box::new(SizedRing::new(40, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 22),
            (25, "effect\\ring_white.tga", 22),
        ])),
        // `case EF_NEW_HEAL_YELLOW:` - `RagEffect.cpp:22465-22472`.
        // Lifetime 40 (`EffectTypes.cpp:1574`).
        EffectId::NewHealYellow => Box::new(SizedRing::new(40, RingSound::Wave, &[
            (45, "effect\\ring_yellow.tga", 23),
            (25, "effect\\ring_yellow.tga", 23),
        ])),
        // `case EF_NEW_SAFETYWALL:` - `RagEffect.cpp:22610-22622`.
        // Lifetime 60 (`EffectTypes.cpp:1686`).
        EffectId::NewSafetywall => Box::new(SizedRing::new(60, RingSound::MinusOneHundred, &[
            (45, "effect\\ring_white.tga", 31),
            (25, "effect\\ring_white.tga", 31),
        ])),
        // `case EF_NEW_PNEUMA:` - `RagEffect.cpp:22638-22645`.
        // Lifetime 1872 (`EffectTypes.cpp:1881`).
        EffectId::NewPneuma => Box::new(SizedRing::new(1872, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 42),
            (25, "effect\\ring_white.tga", 42),
        ])),
        // `case EF_NEW_SW_CAST:` - `RagEffect.cpp:22646-22653`.
        // Lifetime 1876 (`EffectTypes.cpp:1885`).
        EffectId::NewSwCast => Box::new(SizedRing::new(1876, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 43),
            (25, "effect\\ring_white.tga", 43),
        ])),
        // `case EF_ABR_MOTHER:` - `RagEffect.cpp:22688-22695`.
        // Lifetime 1902 (`EffectTypes.cpp:1911`).
        EffectId::AbrMother => Box::new(SizedRing::new(1902, RingSound::Wave, &[
            (45, "effect\\new_abr_mother\\abr_mother.tga", 0),
            (25, "effect\\new_abr_mother\\abr_mother.tga", 0),
        ])),
        // `case EF_ABR_INFINITY_RING:` - `RagEffect.cpp:22654-22663`.
        // Lifetime 1910 (`EffectTypes.cpp:1919`). The official `parent !=
        // nullptr` guard is always true in the hermetic runtime.
        EffectId::AbrInfinityRing => Box::new(SizedRing::new(1910, RingSound::Wave, &[
            (45, "effect\\new_abr_infinity\\ring_infinty.tga", 0),
            (25, "effect\\new_abr_infinity\\ring_infinty.tga", 0),
        ])),
        // `case EF_NEW_D_CANNON_ENTITY:` - `RagEffect.cpp:22672-22679`.
        // Lifetime 1918 (`EffectTypes.cpp:1927`).
        EffectId::NewDCannonEntity => Box::new(SizedRing::new(1918, RingSound::Wave, &[
            (45, "effect\\new_d_cannon\\ABR_DUAL_CANNON.tga", 0),
            (25, "effect\\new_d_cannon\\ABR_DUAL_CANNON.tga", 0),
        ])),
        // `case EF_FLAME_TRAP_RING:` - `RagEffect.cpp:22432-22444`.
        // Lifetime 1923 (`EffectTypes.cpp:1932`).
        EffectId::FlameTrapRing => Box::new(SizedRing::new(1923, RingSound::MinusOneHundred, &[
            (45, "effect\\ring_red.tga", 44),
            (25, "effect\\ring_red.tga", 44),
        ])),
        // `case EF_NEW_PRAMEN_RING:` - `RagEffect.cpp:22445-22456`.
        // Lifetime 1932 (`EffectTypes.cpp:1941`).
        EffectId::NewPramenRing => Box::new(SizedRing::new(1932, RingSound::MinusOneHundred, &[(
            90,
            "effect\\new_pramen\\ring_blue.tga",
            5,
        )])),
        // `case EF_NEW_BATTLE_W_ENTITY:` - `RagEffect.cpp:22664-22671`.
        // Lifetime 1938 (`EffectTypes.cpp:1947`).
        EffectId::NewBattleWEntity => Box::new(SizedRing::new(1938, RingSound::Wave, &[
            (45, "effect\\new_battle_w\\ABR_BATTLE_WARIOR.tga", 0),
            (25, "effect\\new_battle_w\\ABR_BATTLE_WARIOR.tga", 0),
        ])),
        // `case EF_PNEUMATICUS_PROCELLA_RING:` - `RagEffect.cpp:22680-22687`.
        // Lifetime 1959 (`EffectTypes.cpp:1968`).
        EffectId::PneumaticusProcellaRing => Box::new(SizedRing::new(1959, RingSound::Wave, &[
            (45, "effect\\pneumaticusprocella\\pneumaticusprocella_cast\\ring_yellow.tga", 0),
            (25, "effect\\pneumaticusprocella\\pneumaticusprocella_cast\\ring_yellow.tga", 0),
        ])),
        // `case EF_SINCERE_FAITH_RING:` - `RagEffect.cpp:22696-22703`.
        // Lifetime 1963 (`EffectTypes.cpp:1972`).
        EffectId::SincereFaithRing => Box::new(SizedRing::new(1963, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 45),
            (25, "effect\\ring_white.tga", 45),
        ])),
        // `case EF_POWERFUL_FAITH_RING:` - `RagEffect.cpp:22704-22711`.
        // Lifetime 1986 (`EffectTypes.cpp:1995`).
        EffectId::PowerfulFaithRing => Box::new(SizedRing::new(1986, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 46),
            (25, "effect\\ring_white.tga", 46),
        ])),
        // `case EF_FIRM_FAITH_RING:` - `RagEffect.cpp:22712-22719`.
        // Lifetime 1998 (`EffectTypes.cpp:2007`).
        EffectId::FirmFaithRing => Box::new(SizedRing::new(1998, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 47),
            (25, "effect\\ring_white.tga", 47),
        ])),
        // `case EF_RESEARCH_REPORT_RING:` - `RagEffect.cpp:22720-22727`.
        // Lifetime 2240 (`EffectTypes.cpp:2249`).
        EffectId::ResearchReportRing => Box::new(SizedRing::new(2240, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 48),
            (25, "effect\\ring_white.tga", 48),
        ])),
        // `case EF_WOODEN_WARRIOR_RING:` - `RagEffect.cpp:22728-22735`.
        // Lifetime 2249 (`EffectTypes.cpp:2258`).
        EffectId::WoodenWarriorRing => Box::new(SizedRing::new(2249, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 49),
            (25, "effect\\ring_white.tga", 49),
        ])),
        // `case EF_WOODEN_FAIRY_RING:` - `RagEffect.cpp:22736-22743`.
        // Lifetime 2255 (`EffectTypes.cpp:2264`).
        EffectId::WoodenFairyRing => Box::new(SizedRing::new(2255, RingSound::Wave, &[
            (45, "effect\\ring_white.tga", 50),
            (25, "effect\\ring_white.tga", 50),
        ])),
        // `case EF_BEGINSPELL_RING_YELLOW:` - `RagEffect.cpp:22818-22826`.
        // Lifetime 40 (`EffectTypes.cpp:2376`).
        EffectId::BeginspellRingYellow => Box::new(SizedRing::new(40, RingSound::Wave, &[
            (45, "effect\\ring_yellow.tga", 0),
            (25, "effect\\ring_yellow.tga", 0),
        ])),
        _ => return None,
    })
}

pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Blooddrain2,
        EffectId::PeongBlack,
        EffectId::NewHeavensDrive,
        EffectId::HealBlue2,
        EffectId::RgCoinBlue,
        EffectId::RgCoinBlue2,
        EffectId::NewMoonSlash,
        EffectId::NewHealBlue,
        EffectId::NewHealWhite,
        EffectId::NewHealYellow,
        EffectId::NewSafetywall,
        EffectId::NewPneuma,
        EffectId::NewSwCast,
        EffectId::AbrMother,
        EffectId::AbrInfinityRing,
        EffectId::NewDCannonEntity,
        EffectId::FlameTrapRing,
        EffectId::NewPramenRing,
        EffectId::NewBattleWEntity,
        EffectId::PneumaticusProcellaRing,
        EffectId::SincereFaithRing,
        EffectId::PowerfulFaithRing,
        EffectId::FirmFaithRing,
        EffectId::ResearchReportRing,
        EffectId::WoodenWarriorRing,
        EffectId::WoodenFairyRing,
        EffectId::BeginspellRingYellow,
    ]
}

pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:1061`
        (EffectId::Blooddrain2, 200),
        // `EffectTypes.cpp:1216`
        (EffectId::PeongBlack, 200),
        // `EffectTypes.cpp:1408` - 75, not the 250 of `EF_HEAVENSDRIVE`.
        (EffectId::NewHeavensDrive, 75),
        // `EffectTypes.cpp:2423`
        (EffectId::HealBlue2, 100),
        // `EffectTypes.cpp:2428`
        (EffectId::RgCoinBlue, 3000),
        // `EffectTypes.cpp:2429`
        (EffectId::RgCoinBlue2, 3000),
        // `EffectTypes.cpp:1427` - the decompiled table literally repeats the
        // effect id as the lifetime for this entry.
        (EffectId::NewMoonSlash, 1418),
        // `EffectTypes.cpp:1565`
        (EffectId::NewHealBlue, 60),
        // `EffectTypes.cpp:1569`
        (EffectId::NewHealWhite, 40),
        // `EffectTypes.cpp:1574`
        (EffectId::NewHealYellow, 40),
        // `EffectTypes.cpp:1686`
        (EffectId::NewSafetywall, 60),
        // `EffectTypes.cpp:1881` - lifetime equals the effect id in the table.
        (EffectId::NewPneuma, 1872),
        // `EffectTypes.cpp:1885` - lifetime equals the effect id in the table.
        (EffectId::NewSwCast, 1876),
        // `EffectTypes.cpp:1911` - lifetime equals the effect id in the table.
        (EffectId::AbrMother, 1902),
        // `EffectTypes.cpp:1919` - lifetime equals the effect id in the table.
        (EffectId::AbrInfinityRing, 1910),
        // `EffectTypes.cpp:1927` - lifetime equals the effect id in the table.
        (EffectId::NewDCannonEntity, 1918),
        // `EffectTypes.cpp:1932` - lifetime equals the effect id in the table.
        (EffectId::FlameTrapRing, 1923),
        // `EffectTypes.cpp:1941` - lifetime equals the effect id in the table.
        (EffectId::NewPramenRing, 1932),
        // `EffectTypes.cpp:1947` - lifetime equals the effect id in the table.
        (EffectId::NewBattleWEntity, 1938),
        // `EffectTypes.cpp:1968` - lifetime equals the effect id in the table.
        (EffectId::PneumaticusProcellaRing, 1959),
        // `EffectTypes.cpp:1972` - lifetime equals the effect id in the table.
        (EffectId::SincereFaithRing, 1963),
        // `EffectTypes.cpp:1995` - lifetime equals the effect id in the table.
        (EffectId::PowerfulFaithRing, 1986),
        // `EffectTypes.cpp:2007` - lifetime equals the effect id in the table.
        (EffectId::FirmFaithRing, 1998),
        // `EffectTypes.cpp:2249` - lifetime equals the effect id in the table.
        (EffectId::ResearchReportRing, 2240),
        // `EffectTypes.cpp:2258` - lifetime equals the effect id in the table.
        (EffectId::WoodenWarriorRing, 2249),
        // `EffectTypes.cpp:2264` - lifetime equals the effect id in the table.
        (EffectId::WoodenFairyRing, 2255),
        // `EffectTypes.cpp:2376`
        (EffectId::BeginspellRingYellow, 40),
    ]
}

pub fn is_caster_body_only(_effect: EffectId) -> bool {
    false
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

    #[test]
    fn batch_k_owns_twenty_seven_ids() {
        assert_eq!(ids().len(), 27);
    }

    #[test]
    fn unowned_effects_are_not_claimed() {
        assert!(spawn(EffectId::Bash).is_none());
        // `EF_SMA_BLACK2` (1205) / `EF_STIN_BLACK` (1206) share the 1207 hole
        // but need bodies that are not ported (`BlueRingStack`,
        // `TexturedSlashLevel`), so they stay unclaimed. `SmaBlack` (1204) and
        // `EnergydrainBlack` (1208) are neighbours owned by other batches.
        assert!(spawn(EffectId::SmaBlack).is_none());
        assert!(spawn(EffectId::EnergydrainBlack).is_none());
    }

    /// `#[numeric_value]` drives the wire encoding, not the Rust
    /// discriminant, so pin every id added by this batch through
    /// `ToBytes`/`FromBytes` (the `EF_SU_STOOP` lesson: `22202` was a line
    /// number, and `EF_BLOODDRAIN2`'s `1262` was this enum's *line* too; the
    /// official id is 1052).
    #[test]
    fn batch_k_ids_keep_their_official_numeric_values() {
        fn encoded(effect: EffectId) -> [u8; 4] {
            let mut writer = ByteWriter::new();
            effect.to_bytes(&mut writer).expect("EffectId encodes");

            writer.as_slice().try_into().expect("EffectId is a u32")
        }

        fn decoded(value: u32) -> EffectId {
            let bytes = value.to_le_bytes();
            EffectId::from_bytes(&mut ByteReader::without_metadata(&bytes)).expect("id is known")
        }

        // `EffectTypes.h:1060` - between `EF_LG_KINGS_GRACE` (1051) and
        // `EF_NPC_WIDEWEB` (1053).
        assert_eq!(encoded(EffectId::Blooddrain2), 1052u32.to_le_bytes());
        assert!(matches!(decoded(1052), EffectId::Blooddrain2));

        // `EffectTypes.h:1215` - between `EF_STIN_BLACK` (1206) and
        // `EF_ENERGYDRAIN_BLACK` (1208).
        assert_eq!(encoded(EffectId::PeongBlack), 1207u32.to_le_bytes());
        assert!(matches!(decoded(1207), EffectId::PeongBlack));

        // `EffectTypes.h:1407` - after the old `Max` (1243), before
        // `EF_NEW_METALLICSOUND_LINE` (1584).
        assert_eq!(encoded(EffectId::NewHeavensDrive), 1399u32.to_le_bytes());
        assert!(matches!(decoded(1399), EffectId::NewHeavensDrive));

        // `EffectTypes.h:2435` / `:2444` / `:2445` - above the previous max
        // (1584), in official order.
        assert_eq!(encoded(EffectId::HealBlue2), 2427u32.to_le_bytes());
        assert!(matches!(decoded(2427), EffectId::HealBlue2));
        assert_eq!(encoded(EffectId::RgCoinBlue), 2436u32.to_le_bytes());
        assert!(matches!(decoded(2436), EffectId::RgCoinBlue));
        assert_eq!(encoded(EffectId::RgCoinBlue2), 2437u32.to_le_bytes());
        assert!(matches!(decoded(2437), EffectId::RgCoinBlue2));

        // Neighbours must not have shifted.
        assert_eq!(encoded(EffectId::LgKingsGrace), 1051u32.to_le_bytes());
        assert_eq!(encoded(EffectId::NpcWideweb), 1053u32.to_le_bytes());
        assert_eq!(encoded(EffectId::SmaBlack), 1204u32.to_le_bytes());
        assert_eq!(encoded(EffectId::EnergydrainBlack), 1208u32.to_le_bytes());
        assert_eq!(encoded(EffectId::SoulExplosion), 1242u32.to_le_bytes());
        assert_eq!(encoded(EffectId::Max), 1243u32.to_le_bytes());
        assert_eq!(encoded(EffectId::NewMetallicSoundLine), 1584u32.to_le_bytes());
    }

    /// The 21 ring aliases are all above the enum's old `Max` (1243), so each
    /// carries an explicit `#[numeric_value]` from `EffectTypes.h`. Pin them so
    /// a source line can never masquerade as an id.
    #[test]
    fn ring_alias_ids_keep_their_official_numeric_values() {
        fn encoded(effect: EffectId) -> [u8; 4] {
            let mut writer = ByteWriter::new();
            effect.to_bytes(&mut writer).expect("EffectId encodes");

            writer.as_slice().try_into().expect("EffectId is a u32")
        }

        fn decoded(value: u32) -> EffectId {
            let bytes = value.to_le_bytes();
            EffectId::from_bytes(&mut ByteReader::without_metadata(&bytes)).expect("id is known")
        }

        let official: [(EffectId, u32); 21] = [
            (EffectId::NewMoonSlash, 1418),
            (EffectId::NewHealBlue, 1556),
            (EffectId::NewHealWhite, 1560),
            (EffectId::NewHealYellow, 1565),
            (EffectId::NewSafetywall, 1677),
            (EffectId::NewPneuma, 1872),
            (EffectId::NewSwCast, 1876),
            (EffectId::AbrMother, 1902),
            (EffectId::AbrInfinityRing, 1910),
            (EffectId::NewDCannonEntity, 1918),
            (EffectId::FlameTrapRing, 1923),
            (EffectId::NewPramenRing, 1932),
            (EffectId::NewBattleWEntity, 1938),
            (EffectId::PneumaticusProcellaRing, 1959),
            (EffectId::SincereFaithRing, 1963),
            (EffectId::PowerfulFaithRing, 1986),
            (EffectId::FirmFaithRing, 1998),
            (EffectId::ResearchReportRing, 2240),
            (EffectId::WoodenWarriorRing, 2249),
            (EffectId::WoodenFairyRing, 2255),
            (EffectId::BeginspellRingYellow, 2367),
        ];

        for (effect, id) in official {
            assert_eq!(encoded(effect), id.to_le_bytes(), "{effect:?}");
            assert_eq!(format!("{:?}", decoded(id)), format!("{effect:?}"), "id {id}");
        }
    }
}
