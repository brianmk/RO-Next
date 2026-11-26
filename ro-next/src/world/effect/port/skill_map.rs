//! Skill -> ported effect-id map. GENERATED FILE - do not edit by hand.
//!
//! Regenerate with: `./tools/gen_ported_skill_map.py`
//! Drift check:    `./tools/gen_ported_skill_map.py --check`
//!
//! Id source: `tools/data/rodecomp_effect_coverage.json` (`used_effect_ids`).
//! The phase comes from the official field the id belongs to in
//! `tools/data/windows_client_skill_effects.json` (`successEffectId` ->
//! `Success`, `hitEffectId` -> `Hit`, `damageEffectIdOnCaster` -> `Caster`,
//! `groundEffectId` -> `Ground`, ...), intersected with the phases the client
//! already spawns (`skill_effects.ron`). Only ids that
//! `super::registry::spawn` accepts are emitted;
//! `ported_const_matches_registry` pins [`PORTED`] to `registry::ported_ids()`.

use ragnarok_packets::{EffectId, SkillId};

use crate::world::SkillPhase;

/// Every effect id the port can spawn (`registry::ported_ids()`, deduplicated
/// and sorted by numeric id).
pub const PORTED: &[EffectId] = &[
    EffectId::Hit2,
    EffectId::Hit3,
    EffectId::Entry,
    EffectId::Exit,
    EffectId::Warp,
    EffectId::Endure,
    EffectId::Glasswall,
    EffectId::Healsp,
    EffectId::Bash,
    EffectId::Magnumbreak,
    EffectId::Steal,
    EffectId::Detoxication,
    EffectId::Stonecurse,
    EffectId::Fireball,
    EffectId::Icearrow,
    EffectId::Frostdiver,
    EffectId::Frostdiver2,
    EffectId::Firearrow,
    EffectId::Napalmbeat,
    EffectId::Portal,
    EffectId::Incagility,
    EffectId::Decagility,
    EffectId::Aqua,
    EffectId::Signum,
    EffectId::Angelus,
    EffectId::Blessing,
    EffectId::Incagidex,
    EffectId::Firefly,
    EffectId::Spraypond,
    EffectId::Lockon,
    EffectId::Warpzone,
    EffectId::Cure,
    EffectId::Sphere,
    EffectId::Bowlingbash,
    EffectId::Icewall,
    EffectId::Magnificat,
    EffectId::Resurrection,
    EffectId::Recovery,
    EffectId::Earthspike,
    EffectId::Spearbmr,
    EffectId::Aspersio,
    EffectId::Lexdivina,
    EffectId::Stormgust,
    EffectId::Yufitel,
    EffectId::Yufitelhit,
    EffectId::Removetrap,
    EffectId::Bottom,
    EffectId::Blitzbeat,
    EffectId::Waterball,
    EffectId::Detecting,
    EffectId::Sonicblow,
    EffectId::Sonicblowhit,
    EffectId::Grimtooth,
    EffectId::Enchantpoison,
    EffectId::Overthrust,
    EffectId::Twohandquicken,
    EffectId::Grimtoothatk,
    EffectId::Bottom2,
    EffectId::Revive,
    EffectId::Heavensdrive,
    EffectId::Concentration,
    EffectId::Cartrevolution,
    EffectId::Hitdark,
    EffectId::PotionBerserk,
    EffectId::Intimidate,
    EffectId::BottomLa,
    EffectId::Magicrod,
    EffectId::Spearquicken,
    EffectId::RgCoin,
    EffectId::Sprinklesand,
    EffectId::Heal,
    EffectId::Heal2,
    EffectId::Exit2,
    EffectId::Readyportal2,
    EffectId::Portal2,
    EffectId::BottomMag,
    EffectId::BottomSanc,
    EffectId::Heal3,
    EffectId::Hptime,
    EffectId::Sptime,
    EffectId::Portal3,
    EffectId::Entry2,
    EffectId::Vallentine,
    EffectId::Vallentine2,
    EffectId::Portal4,
    EffectId::Portal5,
    EffectId::BottomSpider,
    EffectId::Stormkick,
    EffectId::Halfsphere,
    EffectId::Yufitel2,
    EffectId::Agiup,
    EffectId::Stormkick1,
    EffectId::Stormkick2,
    EffectId::Stormkick3,
    EffectId::Stormkick6,
    EffectId::Stormkick7,
    EffectId::Edp,
    EffectId::RgCoin2,
    EffectId::_05val,
    EffectId::Itemfast,
    EffectId::Tarotcard1,
    EffectId::Tarotcard2,
    EffectId::Tarotcard3,
    EffectId::Tarotcard4,
    EffectId::Tarotcard5,
    EffectId::Tarotcard6,
    EffectId::Tarotcard7,
    EffectId::Tarotcard8,
    EffectId::Tarotcard9,
    EffectId::Tarotcard10,
    EffectId::Tarotcard11,
    EffectId::Tarotcard12,
    EffectId::Tarotcard13,
    EffectId::Tarotcard14,
    EffectId::Ro2year,
    EffectId::SmaReady,
    EffectId::Sma3,
    EffectId::BigPortal,
    EffectId::BigPortal2,
    EffectId::Hamicastle,
    EffectId::Hamiblood,
    EffectId::Hated2,
    EffectId::ItemThunder,
    EffectId::ItemCloud,
    EffectId::ItemCurse,
    EffectId::ItemZzz,
    EffectId::ItemRain,
    EffectId::M01,
    EffectId::M02,
    EffectId::M03,
    EffectId::M04,
    EffectId::M05,
    EffectId::M06,
    EffectId::M07,
    EffectId::TempOk,
    EffectId::Hyousensou,
    EffectId::RgCoin3,
    EffectId::Bash3d5,
    EffectId::Kirikage,
    EffectId::Tatami,
    EffectId::Kasumikiri,
    EffectId::Issen,
    EffectId::Kaen,
    EffectId::Desperado,
    EffectId::LightningS,
    EffectId::BlindS,
    EffectId::PoisonS,
    EffectId::FreezingS,
    EffectId::FlareS,
    EffectId::Rapidshower,
    EffectId::Magicalbullet,
    EffectId::Spreadattack,
    EffectId::Tracking,
    EffectId::Tripleaction,
    EffectId::NpcEarthquake,
    EffectId::BottomRunner,
    EffectId::BottomTransfer,
    EffectId::NpcSlowcast,
    EffectId::Mapsphere,
    EffectId::PokLove,
    EffectId::PokWhite,
    EffectId::PokValen,
    EffectId::PokBirth,
    EffectId::PokChristmas,
    EffectId::Glow1,
    EffectId::Glow2,
    EffectId::Mapsphere2,
    EffectId::Glow11,
    EffectId::Glow12,
    EffectId::Item318,
    EffectId::Firepillaron2,
    EffectId::Windcutter,
    EffectId::Detect2,
    EffectId::Frostmysty,
    EffectId::SprMash,
    EffectId::SprSoule,
    EffectId::Earthwall,
    EffectId::MagentaTrap,
    EffectId::CobaltTrap,
    EffectId::MaizeTrap,
    EffectId::VerdureTrap,
    EffectId::NormalTrap,
    EffectId::Oratio,
    EffectId::PotionBerserk2,
    EffectId::RgCoin4,
    EffectId::Detect3,
    EffectId::Detect4,
    EffectId::Venomimpress,
    EffectId::Frostmisty,
    EffectId::Burning,
    EffectId::Stonering,
    EffectId::Intimidate2,
    EffectId::Redline,
    EffectId::Frostdiver3,
    EffectId::Fireball3,
    EffectId::Intimidate3,
    EffectId::Intimidate4,
    EffectId::RgCoin5,
    EffectId::BottomManhole,
    EffectId::Forestlight6,
    EffectId::BottomAni,
    EffectId::BottomBloodylust,
    EffectId::WaterBelow,
    EffectId::ChemicalV,
    EffectId::ChemicalV2,
    EffectId::ChemicalV3,
    EffectId::RgCoin6,
    EffectId::GradiusLaser,
    EffectId::Bash3d6,
    EffectId::Blueline,
    EffectId::PngTest,
    EffectId::RgCoin7,
    EffectId::Wallofthorn,
    EffectId::Vacuum,
    EffectId::Fireball6,
    EffectId::Coldstatus,
    EffectId::Waterball3,
    EffectId::ElGust,
    EffectId::ElBlast,
    EffectId::ElAquaplay,
    EffectId::ElUpheaval,
    EffectId::ElWildStorm,
    EffectId::ElChillyAir,
    EffectId::ElCursedSoil,
    EffectId::ElCooler,
    EffectId::ElTropic,
    EffectId::ElPyrotechnic,
    EffectId::ElPetrology,
    EffectId::ElHeater,
    EffectId::SilentBreeze,
    EffectId::MagmaFlow,
    EffectId::Icemine,
    EffectId::Icemine1,
    EffectId::RkDragonbreathWater,
    EffectId::NpcWideweb,
    EffectId::NpcBurnt,
];

/// Where the official client launched the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortedAnchor {
    /// On the caster's position (self-targeted and travelling bodies).
    Caster,
    /// On the target's position (impact and ground effects).
    Target,
}

/// One generated mapping, sorted by `(skill, phase)`.
pub const MAPPED: &[(SkillId, SkillPhase, EffectId, PortedAnchor)] = &[
    (SkillId(7), SkillPhase::Success, EffectId::Magnumbreak, PortedAnchor::Caster),
    (SkillId(8), SkillPhase::Success, EffectId::Endure, PortedAnchor::Caster),
    (SkillId(14), SkillPhase::Hit, EffectId::Icearrow, PortedAnchor::Target),
    (SkillId(15), SkillPhase::Caster, EffectId::Frostdiver, PortedAnchor::Caster),
    (SkillId(16), SkillPhase::Success, EffectId::Stonecurse, PortedAnchor::Caster),
    (SkillId(17), SkillPhase::Caster, EffectId::Fireball, PortedAnchor::Caster),
    (SkillId(19), SkillPhase::Hit, EffectId::Firearrow, PortedAnchor::Target),
    (SkillId(28), SkillPhase::Hit, EffectId::Heal3, PortedAnchor::Target),
    (SkillId(29), SkillPhase::Success, EffectId::Incagility, PortedAnchor::Caster),
    (SkillId(30), SkillPhase::Success, EffectId::Decagility, PortedAnchor::Caster),
    (SkillId(31), SkillPhase::Success, EffectId::Aqua, PortedAnchor::Caster),
    (SkillId(32), SkillPhase::Success, EffectId::Signum, PortedAnchor::Caster),
    (SkillId(33), SkillPhase::Success, EffectId::Angelus, PortedAnchor::Caster),
    (SkillId(34), SkillPhase::Success, EffectId::Blessing, PortedAnchor::Caster),
    (SkillId(35), SkillPhase::Success, EffectId::Cure, PortedAnchor::Caster),
    (SkillId(45), SkillPhase::Success, EffectId::Concentration, PortedAnchor::Caster),
    (SkillId(50), SkillPhase::Success, EffectId::Steal, PortedAnchor::Caster),
    (SkillId(53), SkillPhase::Success, EffectId::Detoxication, PortedAnchor::Caster),
    (SkillId(54), SkillPhase::Success, EffectId::Resurrection, PortedAnchor::Caster),
    (SkillId(59), SkillPhase::Release, EffectId::Spearbmr, PortedAnchor::Caster),
    (SkillId(60), SkillPhase::Success, EffectId::Twohandquicken, PortedAnchor::Caster),
    (SkillId(68), SkillPhase::Success, EffectId::Aspersio, PortedAnchor::Caster),
    (SkillId(72), SkillPhase::Success, EffectId::Recovery, PortedAnchor::Caster),
    (SkillId(74), SkillPhase::Success, EffectId::Magnificat, PortedAnchor::Caster),
    (SkillId(76), SkillPhase::Success, EffectId::Lexdivina, PortedAnchor::Caster),
    (SkillId(84), SkillPhase::Success, EffectId::Yufitel, PortedAnchor::Caster),
    (SkillId(88), SkillPhase::Success, EffectId::Frostdiver2, PortedAnchor::Caster),
    (SkillId(90), SkillPhase::Success, EffectId::Earthspike, PortedAnchor::Caster),
    (SkillId(113), SkillPhase::Success, EffectId::Overthrust, PortedAnchor::Caster),
    (SkillId(124), SkillPhase::Success, EffectId::Removetrap, PortedAnchor::Caster),
    (SkillId(136), SkillPhase::Success, EffectId::Sonicblow, PortedAnchor::Caster),
    (SkillId(137), SkillPhase::Hit, EffectId::Grimtooth, PortedAnchor::Caster),
    (SkillId(138), SkillPhase::Success, EffectId::Enchantpoison, PortedAnchor::Caster),
    (SkillId(141), SkillPhase::Success, EffectId::Enchantpoison, PortedAnchor::Caster),
    (SkillId(149), SkillPhase::Success, EffectId::Sprinklesand, PortedAnchor::Caster),
    (SkillId(153), SkillPhase::Cast, EffectId::Cartrevolution, PortedAnchor::Caster),
    (SkillId(211), SkillPhase::Success, EffectId::RgCoin, PortedAnchor::Caster),
    (SkillId(219), SkillPhase::Success, EffectId::Intimidate, PortedAnchor::Caster),
    (SkillId(258), SkillPhase::Success, EffectId::Spearquicken, PortedAnchor::Caster),
    (SkillId(276), SkillPhase::Success, EffectId::Magicrod, PortedAnchor::Caster),
    (SkillId(378), SkillPhase::Success, EffectId::Edp, PortedAnchor::Caster),
    (SkillId(383), SkillPhase::Success, EffectId::Portal4, PortedAnchor::Caster),
    (SkillId(413), SkillPhase::Success, EffectId::Stormkick, PortedAnchor::Caster),
    (SkillId(425), SkillPhase::Success, EffectId::Stormkick3, PortedAnchor::Caster),
    (SkillId(446), SkillPhase::Success, EffectId::PotionBerserk, PortedAnchor::Caster),
    (SkillId(470), SkillPhase::Success, EffectId::M07, PortedAnchor::Caster),
    (SkillId(476), SkillPhase::Success, EffectId::RgCoin2, PortedAnchor::Caster),
    (SkillId(486), SkillPhase::Success, EffectId::Overthrust, PortedAnchor::Caster),
    (
        SkillId(495),
        SkillPhase::Success,
        EffectId::Twohandquicken,
        PortedAnchor::Caster,
    ),
    (SkillId(502), SkillPhase::Success, EffectId::Tripleaction, PortedAnchor::Caster),
    (SkillId(506), SkillPhase::Success, EffectId::Agiup, PortedAnchor::Caster),
    (SkillId(507), SkillPhase::Success, EffectId::Magicalbullet, PortedAnchor::Caster),
    (SkillId(512), SkillPhase::Hit, EffectId::Tracking, PortedAnchor::Target),
    (SkillId(513), SkillPhase::Success, EffectId::RgCoin3, PortedAnchor::Caster),
    (SkillId(515), SkillPhase::Success, EffectId::Rapidshower, PortedAnchor::Caster),
    (SkillId(516), SkillPhase::Success, EffectId::Desperado, PortedAnchor::Caster),
    (SkillId(518), SkillPhase::Success, EffectId::Bash3d5, PortedAnchor::Caster),
    (SkillId(519), SkillPhase::Success, EffectId::M02, PortedAnchor::Caster),
    (SkillId(520), SkillPhase::Success, EffectId::Spreadattack, PortedAnchor::Caster),
    (SkillId(528), SkillPhase::Success, EffectId::Kasumikiri, PortedAnchor::Caster),
    (SkillId(530), SkillPhase::Success, EffectId::Kirikage, PortedAnchor::Caster),
    (SkillId(537), SkillPhase::Success, EffectId::Hyousensou, PortedAnchor::Caster),
    (SkillId(544), SkillPhase::Success, EffectId::Issen, PortedAnchor::Caster),
];

/// The ported effect id `skill` plays at `phase`, if one is registered.
pub fn ported_effect_for(skill: SkillId, phase: SkillPhase) -> Option<EffectId> {
    mapped(skill, phase).map(|(_, _, effect_id, _)| *effect_id)
}

/// Where `ported_effect_for` should anchor the effect.
pub fn ported_anchor_for(skill: SkillId, phase: SkillPhase) -> PortedAnchor {
    mapped(skill, phase)
        .map(|(_, _, _, anchor)| *anchor)
        .unwrap_or(PortedAnchor::Caster)
}

fn mapped(skill: SkillId, phase: SkillPhase) -> Option<&'static (SkillId, SkillPhase, EffectId, PortedAnchor)> {
    MAPPED
        .binary_search_by_key(&(skill.0, phase.index()), |(mapped_skill, mapped_phase, ..)| {
            (mapped_skill.0, mapped_phase.index())
        })
        .ok()
        .map(|index| &MAPPED[index])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::registry;

    #[test]
    fn ported_const_matches_registry() {
        let mut expected: Vec<u32> = registry::ported_ids().iter().map(|effect| *effect as u32).collect();
        expected.sort_unstable();
        expected.dedup();

        let actual: Vec<u32> = PORTED.iter().map(|effect| *effect as u32).collect();

        assert_eq!(actual, expected, "PORTED is stale: run tools/gen_ported_skill_map.py");
    }

    #[test]
    fn every_mapped_pair_spawns() {
        for (skill, phase, effect, _) in MAPPED {
            assert!(registry::spawn(*effect).is_some(), "{skill:?}/{phase:?} maps to an unported id");
            assert!(
                PORTED.iter().any(|ported| *ported as u32 == *effect as u32),
                "{skill:?}/{phase:?} maps to an id missing from PORTED",
            );
        }
    }

    #[test]
    fn mapped_table_is_sorted_and_unique() {
        let mut previous = None;
        for (skill, phase, ..) in MAPPED {
            let key = (skill.0, phase.index());
            assert!(
                previous.is_none_or(|previous| previous < key),
                "MAPPED is not sorted at {key:?}"
            );
            previous = Some(key);
        }
    }

    #[test]
    fn unmapped_skills_return_none() {
        // SM_BASH only uses the generic hit ids (`0`/`1`), none of which is ported.
        assert!(ported_effect_for(SkillId(5), SkillPhase::Hit).is_none());
        assert!(ported_effect_for(SkillId(u16::MAX), SkillPhase::Success).is_none());
        assert!(ported_effect_for(SkillId(5), SkillPhase::Success).is_none());
    }

    #[test]
    fn every_mapped_pair_resolves_through_lookup() {
        for (skill, phase, effect, anchor) in MAPPED {
            assert_eq!(
                ported_effect_for(*skill, *phase).map(|effect| effect as u32),
                Some(*effect as u32)
            );
            assert_eq!(ported_anchor_for(*skill, *phase), *anchor);
        }
    }
}
