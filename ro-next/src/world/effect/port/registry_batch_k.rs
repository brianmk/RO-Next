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
//! None of these is caster-body only: each either launches primitives
//! (`PeongBlack`, `StoneSpike`, `HealRingSparkle`, the coin sprays) or a
//! particle-spline fan (`Blooddrain2`), so [`is_caster_body_only`] stays false.

use ragnarok_packets::EffectId;

use super::rag::{HealRingSparkle, RedParticleSpray, RogueCoin, StoneSpike};
use super::rag_batch_k::PeongBlack;
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
    fn batch_k_owns_six_ids() {
        assert_eq!(ids().len(), 6);
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
}
