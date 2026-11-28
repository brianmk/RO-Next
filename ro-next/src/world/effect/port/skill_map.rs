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
    EffectId::Hit1,
    EffectId::Hit2,
    EffectId::Hit3,
    EffectId::Entry,
    EffectId::Exit,
    EffectId::Warp,
    EffectId::Endure,
    EffectId::Glasswall,
    EffectId::Healsp,
    EffectId::Soulstrike,
    EffectId::Bash,
    EffectId::Magnumbreak,
    EffectId::Steal,
    EffectId::Detoxication,
    EffectId::Stonecurse,
    EffectId::Fireball,
    EffectId::Icearrow,
    EffectId::Frostdiver,
    EffectId::Frostdiver2,
    EffectId::Lightbolt,
    EffectId::Thunderstorm,
    EffectId::Firearrow,
    EffectId::Napalmbeat,
    EffectId::Ruwach,
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
    EffectId::Beginspell6,
    EffectId::Lockon,
    EffectId::Warpzone,
    EffectId::Sightrasher,
    EffectId::Cure,
    EffectId::Provoke,
    EffectId::Brandishspear,
    EffectId::Sphere,
    EffectId::Bowlingbash,
    EffectId::Icewall,
    EffectId::Gloria,
    EffectId::Magnificat,
    EffectId::Resurrection,
    EffectId::Recovery,
    EffectId::Earthspike,
    EffectId::Spearbmr,
    EffectId::Turnundead,
    EffectId::Impositio,
    EffectId::Lexaeterna,
    EffectId::Aspersio,
    EffectId::Lexdivina,
    EffectId::Suffragium,
    EffectId::Stormgust,
    EffectId::Lord,
    EffectId::Meteorstorm,
    EffectId::Yufitel,
    EffectId::Yufitelhit,
    EffectId::Firepillarbomb,
    EffectId::Hasteup,
    EffectId::Removetrap,
    EffectId::Repairweapon,
    EffectId::Perfection,
    EffectId::Maxpower,
    EffectId::Springtrap,
    EffectId::Kyrie,
    EffectId::Bottom,
    EffectId::Blitzbeat,
    EffectId::Waterball,
    EffectId::Waterball2,
    EffectId::Detecting,
    EffectId::Sonicblow,
    EffectId::Sonicblowhit,
    EffectId::Grimtooth,
    EffectId::Enchantpoison,
    EffectId::Poisonreact,
    EffectId::Overthrust,
    EffectId::Twohandquicken,
    EffectId::Grimtoothatk,
    EffectId::Slowpoison,
    EffectId::Bottom2,
    EffectId::Revive,
    EffectId::Heavensdrive,
    EffectId::Sonicblow2,
    EffectId::Pierceself,
    EffectId::Bowlingself,
    EffectId::Spearstabself,
    EffectId::Spearbmrself,
    EffectId::Concentration,
    EffectId::Energycoat,
    EffectId::Cartrevolution,
    EffectId::Hitdark,
    EffectId::SuiExplosion,
    EffectId::Comboattack1,
    EffectId::Comboattack2,
    EffectId::Comboattack3,
    EffectId::Comboattack4,
    EffectId::Comboattack5,
    EffectId::Gumgang,
    EffectId::Blooddrain,
    EffectId::Energydrain,
    EffectId::PotionBerserk,
    EffectId::Defender,
    EffectId::Ganbantein,
    EffectId::Volcano,
    EffectId::Grandcross,
    EffectId::Intimidate,
    EffectId::Spellbreaker,
    EffectId::Dispell,
    EffectId::Deluge,
    EffectId::Violentgale,
    EffectId::Landprotector,
    EffectId::BottomLa,
    EffectId::Magicrod,
    EffectId::Holycross,
    EffectId::Shieldcharge,
    EffectId::Providence,
    EffectId::Shieldboomerang,
    EffectId::Spearquicken,
    EffectId::Devotion,
    EffectId::Reflectshield,
    EffectId::Absorbspirits,
    EffectId::Steelbody,
    EffectId::Flamelauncher,
    EffectId::Frostweapon,
    EffectId::Lightningloader,
    EffectId::Seismicweapon,
    EffectId::Gumgang2,
    EffectId::Teihit1,
    EffectId::Gumgang3,
    EffectId::Teihit2,
    EffectId::Tanji,
    EffectId::Teihit1x,
    EffectId::Chimto,
    EffectId::Stealcoin,
    EffectId::Stripweapon,
    EffectId::Stripshield,
    EffectId::Striparmor,
    EffectId::Striphelm,
    EffectId::Chaincombo,
    EffectId::RgCoin,
    EffectId::Backstap,
    EffectId::Teihit3,
    EffectId::BottomDissonance,
    EffectId::BottomLullaby,
    EffectId::BottomRichmankim,
    EffectId::BottomEternalchaos,
    EffectId::BottomDrumbattlefield,
    EffectId::BottomRingnibelungen,
    EffectId::BottomRokisweil,
    EffectId::BottomIntoabyss,
    EffectId::BottomSiegfried,
    EffectId::BottomWhistle,
    EffectId::BottomAssassincross,
    EffectId::BottomPoembragi,
    EffectId::BottomAppleidun,
    EffectId::BottomUglydance,
    EffectId::BottomHumming,
    EffectId::BottomDontforgetme,
    EffectId::BottomFortunekiss,
    EffectId::BottomServiceforyou,
    EffectId::TalkFrostjoke,
    EffectId::TalkScream,
    EffectId::Throwitem,
    EffectId::Throwitem2,
    EffectId::Chemicalprotection,
    EffectId::Chemical2,
    EffectId::Throwitem3,
    EffectId::Firstaid,
    EffectId::Sprinklesand,
    EffectId::Loud,
    EffectId::Heal,
    EffectId::Heal2,
    EffectId::Exit2,
    EffectId::Readyportal2,
    EffectId::Portal2,
    EffectId::BottomMag,
    EffectId::BottomSanc,
    EffectId::Heal3,
    EffectId::Warpzone2,
    EffectId::Beginasura,
    EffectId::Tripleattack,
    EffectId::Hptime,
    EffectId::Sptime,
    EffectId::Magnum2,
    EffectId::Portal3,
    EffectId::Entry2,
    EffectId::Soulbreaker,
    EffectId::Vallentine,
    EffectId::Vallentine2,
    EffectId::Pressure,
    EffectId::Bash3d,
    EffectId::Aurablade,
    EffectId::Redbody,
    EffectId::Lkconcentration,
    EffectId::Assumptio,
    EffectId::Energydrain2,
    EffectId::Transbluebody,
    EffectId::Magiccrasher,
    EffectId::Lightblade,
    EffectId::Energydrain3,
    EffectId::Linelink2,
    EffectId::Linklight,
    EffectId::Truesight,
    EffectId::Falconassault,
    EffectId::Tripleattack2,
    EffectId::Portal4,
    EffectId::Meltdown,
    EffectId::Cartboost,
    EffectId::Rejectsword,
    EffectId::Tripleattack3,
    EffectId::Spherewind2,
    EffectId::Linelink3,
    EffectId::Pinkbody,
    EffectId::Bash3d3,
    EffectId::Bash3d4,
    EffectId::Napalmvalcan,
    EffectId::Portal5,
    EffectId::Magiccrasher2,
    EffectId::BottomSpider,
    EffectId::Soulburn,
    EffectId::Soulchange,
    EffectId::Baby,
    EffectId::Soulbreaker2,
    EffectId::Pressedbody,
    EffectId::Kickedbody,
    EffectId::Doublegumgang,
    EffectId::Reflectbody,
    EffectId::Babybody,
    EffectId::Asurabody,
    EffectId::_4waybody,
    EffectId::Quakebody,
    EffectId::AsurabodyMonster,
    EffectId::Hitline4,
    EffectId::Hitline5,
    EffectId::Hitline6,
    EffectId::Hitline7,
    EffectId::Stormkick,
    EffectId::Halfsphere,
    EffectId::Chemical3,
    EffectId::Assumptio2,
    EffectId::Bluecasting,
    EffectId::Run,
    EffectId::Stopeffect,
    EffectId::Landbody,
    EffectId::Grandcross2,
    EffectId::Soulstrike2,
    EffectId::Yufitel2,
    EffectId::Gumgangnpc,
    EffectId::Agiup,
    EffectId::Jumpkick,
    EffectId::Quakebody2,
    EffectId::Stormkick1,
    EffectId::Stormkick2,
    EffectId::Stormkick3,
    EffectId::Stormkick4,
    EffectId::Stormkick5,
    EffectId::Stormkick6,
    EffectId::Stormkick7,
    EffectId::Spinedbody2,
    EffectId::Beginasura1,
    EffectId::Beginasura2,
    EffectId::Beginasura3,
    EffectId::Beginasura4,
    EffectId::Beginasura5,
    EffectId::Beginasura6,
    EffectId::Beginasura7,
    EffectId::Aurablade2,
    EffectId::Doublegumgang2,
    EffectId::Doublegumgang3,
    EffectId::Edp,
    EffectId::Shieldboomerang2,
    EffectId::RgCoin2,
    EffectId::Guard2,
    EffectId::Chemicalbody,
    EffectId::Soullink,
    EffectId::Memorize,
    EffectId::_05val,
    EffectId::Beginasura11,
    EffectId::Chemical2dash,
    EffectId::Cartter,
    EffectId::Itemfast,
    EffectId::Shieldboomerang3,
    EffectId::Doublecastbody,
    EffectId::Gravitation,
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
    EffectId::Aciddemon,
    EffectId::Throwitem4,
    EffectId::Throwitem5,
    EffectId::Bluebody,
    EffectId::Hated,
    EffectId::Redlightbody,
    EffectId::Ro2year,
    EffectId::SmaReady,
    EffectId::Stin,
    EffectId::RedHit,
    EffectId::Quakebody3,
    EffectId::Sma2,
    EffectId::Stin2,
    EffectId::Hittexture,
    EffectId::Stin3,
    EffectId::Sma3,
    EffectId::BigPortal,
    EffectId::BigPortal2,
    EffectId::ScreenQuake,
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
    EffectId::Kaizel,
    EffectId::Shrink,
    EffectId::Throwitem6,
    EffectId::Quakebody4,
    EffectId::Fvoice,
    EffectId::Wink,
    EffectId::TempOk,
    EffectId::Throwitem7,
    EffectId::Throwitem8,
    EffectId::Throwitem9,
    EffectId::Throwitem10,
    EffectId::Kouenka,
    EffectId::Hyousensou,
    EffectId::Stin4,
    EffectId::Thunderstorm2,
    EffectId::Chemical4,
    EffectId::Stin5,
    EffectId::MadnessBlue,
    EffectId::MadnessRed,
    EffectId::RgCoin3,
    EffectId::Bash3d5,
    EffectId::Kirikage,
    EffectId::Tatami,
    EffectId::Kasumikiri,
    EffectId::Issen,
    EffectId::Kaen,
    EffectId::Baku,
    EffectId::Hyousyouraku,
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
    EffectId::Bullseye,
    EffectId::NpcEarthquake,
    EffectId::Dragonfear,
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
    EffectId::Rolling1,
    EffectId::Rolling2,
    EffectId::Rolling3,
    EffectId::Rolling4,
    EffectId::Rolling5,
    EffectId::Rolling6,
    EffectId::Rolling7,
    EffectId::Rolling8,
    EffectId::Rolling9,
    EffectId::Rolling10,
    EffectId::RgCoin4,
    EffectId::Gumgang4,
    EffectId::Shieldboomerang4,
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
    EffectId::Chemical2dash2,
    EffectId::WaterBelow,
    EffectId::BeginspellN6,
    EffectId::Dance2,
    EffectId::ChemicalV,
    EffectId::ChemicalV2,
    EffectId::Secra,
    EffectId::Secra2,
    EffectId::ChemicalV3,
    EffectId::RgCoin6,
    EffectId::Heartasura,
    EffectId::GradiusLaser,
    EffectId::Bash3d6,
    EffectId::Gumgang5,
    EffectId::Teihit1t,
    EffectId::Blueline,
    EffectId::PngTest,
    EffectId::RgCoin7,
    EffectId::Wallofthorn,
    EffectId::Throwitem11,
    EffectId::Vacuum,
    EffectId::Gumgang6,
    EffectId::Fireball6,
    EffectId::Gumgang7,
    EffectId::Gumgang8,
    EffectId::Gumgang9,
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
    EffectId::TetraGround,
    EffectId::ThrowHappokunai,
    EffectId::ThrowMultipleCoin,
    EffectId::ThrowBakuretsu,
    EffectId::Icemine,
    EffectId::Icemine1,
    EffectId::RkDragonbreathWater,
    EffectId::Blooddrain2,
    EffectId::NpcWideweb,
    EffectId::NpcBurnt,
    EffectId::SuStoop,
    EffectId::SmaBlack2,
    EffectId::PeongBlack,
    EffectId::EnergydrainBlack,
    EffectId::NewHeavensDrive,
    EffectId::NewMoonSlash,
    EffectId::NewHealBlue,
    EffectId::NewHealWhite,
    EffectId::NewHealYellow,
    EffectId::NewMetallicSoundLine,
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
    EffectId::ViolenHit,
    EffectId::HealBlue2,
    EffectId::RgCoinBlue,
    EffectId::RgCoinBlue2,
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
    (SkillId(6), SkillPhase::Success, EffectId::Provoke, PortedAnchor::Caster),
    (SkillId(7), SkillPhase::Success, EffectId::Magnumbreak, PortedAnchor::Caster),
    (SkillId(8), SkillPhase::Success, EffectId::Endure, PortedAnchor::Caster),
    (SkillId(14), SkillPhase::Hit, EffectId::Icearrow, PortedAnchor::Target),
    (SkillId(15), SkillPhase::Caster, EffectId::Frostdiver, PortedAnchor::Caster),
    (SkillId(16), SkillPhase::Success, EffectId::Stonecurse, PortedAnchor::Caster),
    (SkillId(17), SkillPhase::Caster, EffectId::Fireball, PortedAnchor::Caster),
    (SkillId(19), SkillPhase::Hit, EffectId::Firearrow, PortedAnchor::Target),
    (SkillId(20), SkillPhase::Hit, EffectId::Lightbolt, PortedAnchor::Target),
    (SkillId(21), SkillPhase::Hit, EffectId::Thunderstorm, PortedAnchor::Target),
    (SkillId(24), SkillPhase::Success, EffectId::Ruwach, PortedAnchor::Caster),
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
    (SkillId(56), SkillPhase::Success, EffectId::Pierceself, PortedAnchor::Caster),
    (SkillId(57), SkillPhase::Success, EffectId::Brandishspear, PortedAnchor::Caster),
    (SkillId(58), SkillPhase::Success, EffectId::Spearstabself, PortedAnchor::Caster),
    (SkillId(59), SkillPhase::Success, EffectId::Spearbmrself, PortedAnchor::Caster),
    (SkillId(60), SkillPhase::Success, EffectId::Twohandquicken, PortedAnchor::Caster),
    (SkillId(62), SkillPhase::Success, EffectId::Bowlingself, PortedAnchor::Caster),
    (SkillId(66), SkillPhase::Success, EffectId::Impositio, PortedAnchor::Caster),
    (SkillId(67), SkillPhase::Success, EffectId::Suffragium, PortedAnchor::Caster),
    (SkillId(68), SkillPhase::Success, EffectId::Aspersio, PortedAnchor::Caster),
    (SkillId(70), SkillPhase::Ground, EffectId::BottomSanc, PortedAnchor::Target),
    (SkillId(71), SkillPhase::Success, EffectId::Slowpoison, PortedAnchor::Caster),
    (SkillId(72), SkillPhase::Success, EffectId::Recovery, PortedAnchor::Caster),
    (SkillId(73), SkillPhase::Success, EffectId::Kyrie, PortedAnchor::Caster),
    (SkillId(74), SkillPhase::Success, EffectId::Magnificat, PortedAnchor::Caster),
    (SkillId(75), SkillPhase::Success, EffectId::Gloria, PortedAnchor::Caster),
    (SkillId(76), SkillPhase::Success, EffectId::Lexdivina, PortedAnchor::Caster),
    (SkillId(77), SkillPhase::Success, EffectId::Turnundead, PortedAnchor::Caster),
    (SkillId(78), SkillPhase::Success, EffectId::Lexaeterna, PortedAnchor::Caster),
    (SkillId(79), SkillPhase::Ground, EffectId::BottomMag, PortedAnchor::Target),
    (SkillId(80), SkillPhase::Success, EffectId::Firepillarbomb, PortedAnchor::Caster),
    (SkillId(81), SkillPhase::Success, EffectId::Sightrasher, PortedAnchor::Caster),
    (SkillId(83), SkillPhase::Success, EffectId::Meteorstorm, PortedAnchor::Caster),
    (SkillId(84), SkillPhase::Success, EffectId::Yufitel, PortedAnchor::Caster),
    (SkillId(85), SkillPhase::Success, EffectId::Lord, PortedAnchor::Caster),
    (SkillId(86), SkillPhase::Success, EffectId::Waterball2, PortedAnchor::Caster),
    (SkillId(87), SkillPhase::Ground, EffectId::Icewall, PortedAnchor::Target),
    (SkillId(88), SkillPhase::Success, EffectId::Frostdiver2, PortedAnchor::Caster),
    (SkillId(89), SkillPhase::Ground, EffectId::Stormgust, PortedAnchor::Target),
    (SkillId(90), SkillPhase::Success, EffectId::Earthspike, PortedAnchor::Caster),
    (SkillId(91), SkillPhase::Ground, EffectId::Heavensdrive, PortedAnchor::Target),
    (SkillId(108), SkillPhase::Success, EffectId::Repairweapon, PortedAnchor::Caster),
    (SkillId(111), SkillPhase::Success, EffectId::Hasteup, PortedAnchor::Caster),
    (SkillId(112), SkillPhase::Success, EffectId::Perfection, PortedAnchor::Caster),
    (SkillId(113), SkillPhase::Success, EffectId::Overthrust, PortedAnchor::Caster),
    (SkillId(114), SkillPhase::Success, EffectId::Maxpower, PortedAnchor::Caster),
    (SkillId(124), SkillPhase::Success, EffectId::Removetrap, PortedAnchor::Caster),
    (SkillId(129), SkillPhase::Success, EffectId::Blitzbeat, PortedAnchor::Target),
    (SkillId(130), SkillPhase::Ground, EffectId::Detecting, PortedAnchor::Target),
    (SkillId(131), SkillPhase::Success, EffectId::Springtrap, PortedAnchor::Caster),
    (SkillId(136), SkillPhase::Success, EffectId::Sonicblow, PortedAnchor::Caster),
    (SkillId(137), SkillPhase::Hit, EffectId::Grimtooth, PortedAnchor::Caster),
    (SkillId(138), SkillPhase::Success, EffectId::Enchantpoison, PortedAnchor::Caster),
    (SkillId(139), SkillPhase::Success, EffectId::Poisonreact, PortedAnchor::Caster),
    (SkillId(141), SkillPhase::Success, EffectId::Enchantpoison, PortedAnchor::Caster),
    (SkillId(142), SkillPhase::Success, EffectId::Firstaid, PortedAnchor::Caster),
    (SkillId(149), SkillPhase::Success, EffectId::Sprinklesand, PortedAnchor::Caster),
    (SkillId(152), SkillPhase::Release, EffectId::Throwitem3, PortedAnchor::Target),
    (SkillId(153), SkillPhase::Cast, EffectId::Cartrevolution, PortedAnchor::Caster),
    (SkillId(155), SkillPhase::Success, EffectId::Loud, PortedAnchor::Caster),
    (SkillId(157), SkillPhase::Success, EffectId::Energycoat, PortedAnchor::Caster),
    (SkillId(211), SkillPhase::Success, EffectId::Stealcoin, PortedAnchor::Caster),
    (SkillId(212), SkillPhase::Success, EffectId::Backstap, PortedAnchor::Caster),
    (SkillId(214), SkillPhase::Success, EffectId::Teihit3, PortedAnchor::Caster),
    (SkillId(215), SkillPhase::Success, EffectId::Stripweapon, PortedAnchor::Caster),
    (SkillId(216), SkillPhase::Success, EffectId::Stripshield, PortedAnchor::Caster),
    (SkillId(217), SkillPhase::Success, EffectId::Striparmor, PortedAnchor::Caster),
    (SkillId(218), SkillPhase::Success, EffectId::Striphelm, PortedAnchor::Caster),
    (SkillId(219), SkillPhase::Success, EffectId::Intimidate, PortedAnchor::Caster),
    (SkillId(230), SkillPhase::Release, EffectId::Throwitem, PortedAnchor::Target),
    (SkillId(231), SkillPhase::Success, EffectId::Throwitem2, PortedAnchor::Caster),
    (
        SkillId(234),
        SkillPhase::Success,
        EffectId::Chemicalprotection,
        PortedAnchor::Caster,
    ),
    (
        SkillId(235),
        SkillPhase::Success,
        EffectId::Chemicalprotection,
        PortedAnchor::Caster,
    ),
    (
        SkillId(236),
        SkillPhase::Success,
        EffectId::Chemicalprotection,
        PortedAnchor::Caster,
    ),
    (
        SkillId(237),
        SkillPhase::Success,
        EffectId::Chemicalprotection,
        PortedAnchor::Caster,
    ),
    (SkillId(250), SkillPhase::Success, EffectId::Shieldcharge, PortedAnchor::Caster),
    (
        SkillId(251),
        SkillPhase::Release,
        EffectId::Shieldboomerang,
        PortedAnchor::Target,
    ),
    (SkillId(252), SkillPhase::Success, EffectId::Reflectshield, PortedAnchor::Caster),
    (SkillId(253), SkillPhase::Success, EffectId::Holycross, PortedAnchor::Caster),
    (SkillId(254), SkillPhase::Success, EffectId::Grandcross, PortedAnchor::Caster),
    (SkillId(255), SkillPhase::Success, EffectId::Devotion, PortedAnchor::Caster),
    (SkillId(256), SkillPhase::Success, EffectId::Providence, PortedAnchor::Caster),
    (SkillId(257), SkillPhase::Success, EffectId::Defender, PortedAnchor::Caster),
    (SkillId(258), SkillPhase::Success, EffectId::Spearquicken, PortedAnchor::Caster),
    (SkillId(262), SkillPhase::Success, EffectId::Absorbspirits, PortedAnchor::Caster),
    (SkillId(263), SkillPhase::Success, EffectId::Tripleattack, PortedAnchor::Caster),
    (SkillId(266), SkillPhase::Success, EffectId::Teihit2, PortedAnchor::Caster),
    (SkillId(267), SkillPhase::Success, EffectId::Tanji, PortedAnchor::Caster),
    (SkillId(268), SkillPhase::Success, EffectId::Steelbody, PortedAnchor::Caster),
    (SkillId(270), SkillPhase::Success, EffectId::Gumgang, PortedAnchor::Caster),
    (SkillId(271), SkillPhase::Hit, EffectId::Teihit1x, PortedAnchor::Target),
    (SkillId(272), SkillPhase::Hit, EffectId::Chaincombo, PortedAnchor::Target),
    (SkillId(276), SkillPhase::Success, EffectId::Magicrod, PortedAnchor::Caster),
    (SkillId(277), SkillPhase::Success, EffectId::Spellbreaker, PortedAnchor::Caster),
    (SkillId(280), SkillPhase::Success, EffectId::Flamelauncher, PortedAnchor::Caster),
    (SkillId(281), SkillPhase::Success, EffectId::Frostweapon, PortedAnchor::Caster),
    (
        SkillId(282),
        SkillPhase::Success,
        EffectId::Lightningloader,
        PortedAnchor::Caster,
    ),
    (SkillId(283), SkillPhase::Success, EffectId::Seismicweapon, PortedAnchor::Caster),
    (SkillId(288), SkillPhase::Ground, EffectId::BottomLa, PortedAnchor::Target),
    (SkillId(289), SkillPhase::Success, EffectId::Dispell, PortedAnchor::Caster),
    (SkillId(306), SkillPhase::Success, EffectId::BottomLullaby, PortedAnchor::Caster),
    (
        SkillId(307),
        SkillPhase::Success,
        EffectId::BottomRichmankim,
        PortedAnchor::Caster,
    ),
    (
        SkillId(308),
        SkillPhase::Success,
        EffectId::BottomEternalchaos,
        PortedAnchor::Caster,
    ),
    (
        SkillId(309),
        SkillPhase::Success,
        EffectId::BottomDrumbattlefield,
        PortedAnchor::Caster,
    ),
    (
        SkillId(310),
        SkillPhase::Success,
        EffectId::BottomRingnibelungen,
        PortedAnchor::Caster,
    ),
    (
        SkillId(311),
        SkillPhase::Success,
        EffectId::BottomRokisweil,
        PortedAnchor::Caster,
    ),
    (
        SkillId(312),
        SkillPhase::Success,
        EffectId::BottomIntoabyss,
        PortedAnchor::Caster,
    ),
    (
        SkillId(313),
        SkillPhase::Success,
        EffectId::BottomSiegfried,
        PortedAnchor::Caster,
    ),
    (
        SkillId(317),
        SkillPhase::Success,
        EffectId::BottomDissonance,
        PortedAnchor::Caster,
    ),
    (SkillId(318), SkillPhase::Cast, EffectId::TalkFrostjoke, PortedAnchor::Caster),
    (SkillId(319), SkillPhase::Success, EffectId::BottomWhistle, PortedAnchor::Caster),
    (
        SkillId(320),
        SkillPhase::Success,
        EffectId::BottomAssassincross,
        PortedAnchor::Caster,
    ),
    (
        SkillId(321),
        SkillPhase::Success,
        EffectId::BottomPoembragi,
        PortedAnchor::Caster,
    ),
    (
        SkillId(322),
        SkillPhase::Success,
        EffectId::BottomAppleidun,
        PortedAnchor::Caster,
    ),
    (
        SkillId(325),
        SkillPhase::Success,
        EffectId::BottomUglydance,
        PortedAnchor::Caster,
    ),
    (SkillId(326), SkillPhase::Cast, EffectId::TalkScream, PortedAnchor::Caster),
    (SkillId(327), SkillPhase::Success, EffectId::BottomHumming, PortedAnchor::Caster),
    (
        SkillId(328),
        SkillPhase::Success,
        EffectId::BottomDontforgetme,
        PortedAnchor::Caster,
    ),
    (
        SkillId(329),
        SkillPhase::Success,
        EffectId::BottomFortunekiss,
        PortedAnchor::Caster,
    ),
    (
        SkillId(330),
        SkillPhase::Success,
        EffectId::BottomServiceforyou,
        PortedAnchor::Caster,
    ),
    (SkillId(355), SkillPhase::Success, EffectId::Aurablade, PortedAnchor::Caster),
    (
        SkillId(357),
        SkillPhase::Success,
        EffectId::Lkconcentration,
        PortedAnchor::Caster,
    ),
    (SkillId(361), SkillPhase::Success, EffectId::Assumptio, PortedAnchor::Caster),
    (SkillId(364), SkillPhase::Success, EffectId::Energydrain, PortedAnchor::Caster),
    (SkillId(365), SkillPhase::Success, EffectId::Magiccrasher, PortedAnchor::Caster),
    (SkillId(366), SkillPhase::Cast, EffectId::Lightblade, PortedAnchor::Caster),
    (SkillId(367), SkillPhase::Release, EffectId::Pressure, PortedAnchor::Target),
    (SkillId(368), SkillPhase::Success, EffectId::Bash3d, PortedAnchor::Caster),
    (SkillId(373), SkillPhase::Success, EffectId::Energydrain3, PortedAnchor::Caster),
    (SkillId(374), SkillPhase::Success, EffectId::Linelink2, PortedAnchor::Caster),
    (SkillId(375), SkillPhase::Success, EffectId::Magiccrasher, PortedAnchor::Caster),
    (SkillId(378), SkillPhase::Success, EffectId::Edp, PortedAnchor::Caster),
    (SkillId(379), SkillPhase::Release, EffectId::Soulbreaker, PortedAnchor::Target),
    (SkillId(380), SkillPhase::Success, EffectId::Truesight, PortedAnchor::Caster),
    (SkillId(381), SkillPhase::Success, EffectId::Falconassault, PortedAnchor::Caster),
    (SkillId(382), SkillPhase::Success, EffectId::Tripleattack2, PortedAnchor::Caster),
    (SkillId(383), SkillPhase::Cast, EffectId::Beginspell6, PortedAnchor::Caster),
    (SkillId(383), SkillPhase::Success, EffectId::Portal4, PortedAnchor::Caster),
    (SkillId(384), SkillPhase::Success, EffectId::Meltdown, PortedAnchor::Caster),
    (SkillId(387), SkillPhase::Success, EffectId::Cartboost, PortedAnchor::Caster),
    (SkillId(390), SkillPhase::Success, EffectId::Rejectsword, PortedAnchor::Caster),
    (SkillId(394), SkillPhase::Success, EffectId::Tripleattack3, PortedAnchor::Caster),
    (SkillId(395), SkillPhase::Success, EffectId::Spherewind2, PortedAnchor::Caster),
    (SkillId(396), SkillPhase::Success, EffectId::Linelink3, PortedAnchor::Caster),
    (SkillId(397), SkillPhase::Success, EffectId::Magnum2, PortedAnchor::Caster),
    (SkillId(398), SkillPhase::Cast, EffectId::Bash3d3, PortedAnchor::Caster),
    (SkillId(399), SkillPhase::Cast, EffectId::Bash3d4, PortedAnchor::Caster),
    (SkillId(400), SkillPhase::Success, EffectId::Napalmvalcan, PortedAnchor::Caster),
    (SkillId(402), SkillPhase::Success, EffectId::Magiccrasher2, PortedAnchor::Caster),
    (SkillId(405), SkillPhase::Ground, EffectId::BottomSpider, PortedAnchor::Target),
    (SkillId(406), SkillPhase::Success, EffectId::Soulbreaker2, PortedAnchor::Caster),
    (SkillId(408), SkillPhase::Success, EffectId::Baby, PortedAnchor::Caster),
    (SkillId(411), SkillPhase::Success, EffectId::Stopeffect, PortedAnchor::Caster),
    (SkillId(413), SkillPhase::Success, EffectId::Stormkick, PortedAnchor::Caster),
    (SkillId(415), SkillPhase::Success, EffectId::Pressedbody, PortedAnchor::Caster),
    (SkillId(417), SkillPhase::Success, EffectId::Hitline4, PortedAnchor::Caster),
    (SkillId(419), SkillPhase::Success, EffectId::Kickedbody, PortedAnchor::Caster),
    (SkillId(421), SkillPhase::Success, EffectId::Chemical3, PortedAnchor::Caster),
    (SkillId(425), SkillPhase::Success, EffectId::Stormkick3, PortedAnchor::Caster),
    (SkillId(426), SkillPhase::Success, EffectId::Landbody, PortedAnchor::Caster),
    (SkillId(431), SkillPhase::Success, EffectId::Bluecasting, PortedAnchor::Caster),
    (SkillId(432), SkillPhase::Success, EffectId::Bluecasting, PortedAnchor::Caster),
    (SkillId(433), SkillPhase::Success, EffectId::Bluecasting, PortedAnchor::Caster),
    (SkillId(434), SkillPhase::Success, EffectId::Hated, PortedAnchor::Caster),
    (SkillId(445), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(446), SkillPhase::Success, EffectId::PotionBerserk, PortedAnchor::Caster),
    (SkillId(447), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(448), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(449), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(450), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(451), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(452), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(453), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(454), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(455), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(456), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(457), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(458), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(459), SkillPhase::Success, EffectId::Hasteup, PortedAnchor::Caster),
    (SkillId(460), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(461), SkillPhase::Success, EffectId::Soullink, PortedAnchor::Target),
    (SkillId(462), SkillPhase::Success, EffectId::Hated, PortedAnchor::Caster),
    (SkillId(463), SkillPhase::Success, EffectId::Hated, PortedAnchor::Caster),
    (SkillId(464), SkillPhase::Success, EffectId::Bluebody, PortedAnchor::Caster),
    (SkillId(465), SkillPhase::Success, EffectId::Reflectbody, PortedAnchor::Caster),
    (SkillId(467), SkillPhase::Success, EffectId::Stin, PortedAnchor::Caster),
    (SkillId(468), SkillPhase::Success, EffectId::Hitline4, PortedAnchor::Caster),
    (SkillId(469), SkillPhase::Success, EffectId::_4waybody, PortedAnchor::Caster),
    (SkillId(470), SkillPhase::Success, EffectId::Babybody, PortedAnchor::Caster),
    (
        SkillId(471),
        SkillPhase::Success,
        EffectId::AsurabodyMonster,
        PortedAnchor::Caster,
    ),
    (SkillId(472), SkillPhase::Success, EffectId::Steelbody, PortedAnchor::Caster),
    (SkillId(475), SkillPhase::Success, EffectId::Guard2, PortedAnchor::Caster),
    (SkillId(476), SkillPhase::Success, EffectId::RgCoin2, PortedAnchor::Caster),
    (
        SkillId(479),
        SkillPhase::Success,
        EffectId::Chemicalprotection,
        PortedAnchor::Caster,
    ),
    (
        SkillId(480),
        SkillPhase::Release,
        EffectId::Shieldboomerang3,
        PortedAnchor::Target,
    ),
    (
        SkillId(482),
        SkillPhase::Success,
        EffectId::Doublecastbody,
        PortedAnchor::Caster,
    ),
    (SkillId(483), SkillPhase::Success, EffectId::Ganbantein, PortedAnchor::Target),
    (SkillId(484), SkillPhase::Success, EffectId::Gravitation, PortedAnchor::Caster),
    (SkillId(485), SkillPhase::Success, EffectId::Cartter, PortedAnchor::Caster),
    (SkillId(486), SkillPhase::Success, EffectId::Overthrust, PortedAnchor::Caster),
    (SkillId(489), SkillPhase::Success, EffectId::Chemicalbody, PortedAnchor::Caster),
    (SkillId(490), SkillPhase::Success, EffectId::Aciddemon, PortedAnchor::Caster),
    (SkillId(494), SkillPhase::Success, EffectId::Asurabody, PortedAnchor::Caster),
    (
        SkillId(495),
        SkillPhase::Success,
        EffectId::Twohandquicken,
        PortedAnchor::Caster,
    ),
    (SkillId(502), SkillPhase::Success, EffectId::Tripleaction, PortedAnchor::Caster),
    (SkillId(503), SkillPhase::Success, EffectId::Bullseye, PortedAnchor::Caster),
    (SkillId(504), SkillPhase::Success, EffectId::MadnessBlue, PortedAnchor::Caster),
    (SkillId(505), SkillPhase::Success, EffectId::MadnessRed, PortedAnchor::Caster),
    (SkillId(506), SkillPhase::Success, EffectId::Agiup, PortedAnchor::Caster),
    (SkillId(507), SkillPhase::Success, EffectId::Magicalbullet, PortedAnchor::Caster),
    (SkillId(512), SkillPhase::Hit, EffectId::Tracking, PortedAnchor::Target),
    (SkillId(513), SkillPhase::Success, EffectId::RgCoin3, PortedAnchor::Caster),
    (SkillId(514), SkillPhase::Success, EffectId::Chemical4, PortedAnchor::Caster),
    (SkillId(515), SkillPhase::Success, EffectId::Rapidshower, PortedAnchor::Caster),
    (SkillId(516), SkillPhase::Success, EffectId::Desperado, PortedAnchor::Caster),
    (SkillId(517), SkillPhase::Cast, EffectId::MadnessRed, PortedAnchor::Caster),
    (SkillId(518), SkillPhase::Success, EffectId::Bash3d5, PortedAnchor::Caster),
    (SkillId(519), SkillPhase::Success, EffectId::M02, PortedAnchor::Caster),
    (SkillId(520), SkillPhase::Success, EffectId::Spreadattack, PortedAnchor::Caster),
    (SkillId(523), SkillPhase::Release, EffectId::Throwitem7, PortedAnchor::Target),
    (SkillId(524), SkillPhase::Release, EffectId::Throwitem8, PortedAnchor::Target),
    (SkillId(525), SkillPhase::Release, EffectId::Throwitem9, PortedAnchor::Target),
    (SkillId(526), SkillPhase::Release, EffectId::Throwitem10, PortedAnchor::Target),
    (SkillId(527), SkillPhase::Ground, EffectId::Tatami, PortedAnchor::Target),
    (SkillId(528), SkillPhase::Success, EffectId::Kasumikiri, PortedAnchor::Caster),
    (SkillId(530), SkillPhase::Success, EffectId::Kirikage, PortedAnchor::Caster),
    (SkillId(534), SkillPhase::Success, EffectId::Kouenka, PortedAnchor::Caster),
    (SkillId(535), SkillPhase::Ground, EffectId::Kaen, PortedAnchor::Target),
    (SkillId(536), SkillPhase::Success, EffectId::Baku, PortedAnchor::Caster),
    (SkillId(537), SkillPhase::Success, EffectId::Hyousensou, PortedAnchor::Caster),
    (SkillId(539), SkillPhase::Success, EffectId::Hyousyouraku, PortedAnchor::Caster),
    (SkillId(540), SkillPhase::Success, EffectId::Stin4, PortedAnchor::Caster),
    (SkillId(541), SkillPhase::Success, EffectId::Thunderstorm2, PortedAnchor::Caster),
    (SkillId(542), SkillPhase::Success, EffectId::Stin5, PortedAnchor::Caster),
    (SkillId(544), SkillPhase::Success, EffectId::Issen, PortedAnchor::Caster),
    (SkillId(659), SkillPhase::Success, EffectId::Dragonfear, PortedAnchor::Caster),
    (SkillId(735), SkillPhase::Success, EffectId::Gumgang5, PortedAnchor::Caster),
    (SkillId(1004), SkillPhase::Release, EffectId::Throwitem6, PortedAnchor::Target),
    (SkillId(1005), SkillPhase::Ground, EffectId::Quakebody4, PortedAnchor::Target),
    (SkillId(1008), SkillPhase::Success, EffectId::Frostweapon, PortedAnchor::Caster),
    (SkillId(1010), SkillPhase::Success, EffectId::Fvoice, PortedAnchor::Caster),
    (SkillId(1011), SkillPhase::Success, EffectId::Wink, PortedAnchor::Caster),
    (
        SkillId(1017),
        SkillPhase::Success,
        EffectId::Seismicweapon,
        PortedAnchor::Caster,
    ),
    (
        SkillId(1018),
        SkillPhase::Success,
        EffectId::Flamelauncher,
        PortedAnchor::Caster,
    ),
    (
        SkillId(1019),
        SkillPhase::Success,
        EffectId::Lightningloader,
        PortedAnchor::Caster,
    ),
    (SkillId(2017), SkillPhase::Success, EffectId::Gumgang3, PortedAnchor::Caster),
    (SkillId(2217), SkillPhase::Success, EffectId::TetraGround, PortedAnchor::Caster),
    (SkillId(2267), SkillPhase::Success, EffectId::SuiExplosion, PortedAnchor::Caster),
    (
        SkillId(2476),
        SkillPhase::Success,
        EffectId::Chemical2dash2,
        PortedAnchor::Caster,
    ),
    (
        SkillId(2477),
        SkillPhase::Success,
        EffectId::Chemical2dash,
        PortedAnchor::Caster,
    ),
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
        // `SM_BASH` (5) keeps its `Hit` descriptor row; the 2019 table only
        // names the generic `EF_HIT1`/`EF_HIT2`, which are not the skill's
        // own launch, so no phase of it is mapped.
        assert!(ported_effect_for(SkillId(5), SkillPhase::Hit).is_none());
        assert!(ported_effect_for(SkillId(u16::MAX), SkillPhase::Success).is_none());
        assert!(ported_effect_for(SkillId(5), SkillPhase::Success).is_none());
    }

    #[test]
    fn blitz_beat_maps_to_its_ported_body() {
        // `HT_BLITZBEAT` (129) has no `skill_effects.ron` descriptor row,
        // so this mapping can only come from the 2019 client effect table
        // (`client_effect_ids_2019` -> `EF_BLITZBEAT` 115). It is what makes
        // the opt-in ported body reachable for the skill.
        assert_eq!(
            ported_effect_for(SkillId(129), SkillPhase::Success).map(|effect| effect as u32),
            Some(EffectId::Blitzbeat as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(129), SkillPhase::Success), PortedAnchor::Target);
    }

    #[test]
    fn detect_maps_to_its_ported_ground_aura() {
        // `HT_DETECTING` (130) is `TargetType: Ground`, so the arriving lane
        // is `ZC_NOTIFY_GROUNDSKILL` and the official
        // `CGameActor::PlayGroundSkillEffect` launches `EF_DETECTING` (119)
        // at the clicked cell (`RenderObject.cpp:1096-1103`). Its body is
        // `TexturedSkillAura(0)` (`RagEffect.cpp:23953-23954` -> `:15281-15359`),
        // registered as `EffectId::Detecting`. The 2019 SkillEffect table has
        // no descriptor field for it, so the mapping is curated in
        // `gen_ported_skill_map.py` instead of coming from the coverage data.
        assert_eq!(
            ported_effect_for(SkillId(130), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::Detecting as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(130), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: MsgEffect.cpp:600-608 (CSkill::OnProcess
    /// UNT_SANCTUARY, LaunchEffect at :602)
    #[test]
    fn curated_extra_pr_sanctuary_plays_bottomsanc_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(70), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::BottomSanc as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(70), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: MsgEffect.cpp:816-824 (CSkill::OnProcess
    /// UNT_MAGNUS, LaunchEffect at :818)
    #[test]
    fn curated_extra_pr_magnus_plays_bottommag_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(79), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::BottomMag as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(79), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: RenderObject.cpp:1117-1121 (PlayGroundSkillEffect
    /// WZ_ICEWALL, LaunchDummyEffect 0x4A=74 at :1119)
    #[test]
    fn curated_extra_wz_icewall_plays_icewall_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(87), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::Icewall as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(87), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: RenderObject.cpp:1133-1141 (PlayGroundSkillEffect
    /// WZ_STORMGUST, LaunchDummyEffect 0x59=89 at :1139)
    #[test]
    fn curated_extra_wz_stormgust_plays_stormgust_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(89), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::Stormgust as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(89), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: RenderObject.cpp:1059-1063 (PlayGroundSkillEffect
    /// WZ_HEAVENDRIVE, LaunchDummyEffect 0x8E=142 at :1061)
    #[test]
    fn curated_extra_wz_heavendrive_plays_heavensdrive_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(91), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::Heavensdrive as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(91), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: RenderObject.cpp:5829-5833 (ProcessSkillMessage
    /// MO_CHAINCOMBO, LaunchEffect EF_CHAINCOMBO at :5833)
    #[test]
    fn curated_extra_mo_chaincombo_plays_chaincombo_on_hit() {
        assert_eq!(
            ported_effect_for(SkillId(272), SkillPhase::Hit).map(|effect| effect as u32),
            Some(EffectId::Chaincombo as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(272), SkillPhase::Hit), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: MsgEffect.cpp:687-695 (CSkill::OnProcess
    /// UNT_LANDPROTECTOR, LaunchEffect at :689)
    #[test]
    fn curated_extra_sa_landprotector_plays_bottomla_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(288), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::BottomLa as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(288), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: SkillActionInfo.cpp:165-178 ->
    /// GameModePackets.cpp:12947-12953/:17551-17559
    #[test]
    fn curated_extra_sn_windwalk_plays_beginspell6_on_cast() {
        assert_eq!(
            ported_effect_for(SkillId(383), SkillPhase::Cast).map(|effect| effect as u32),
            Some(EffectId::Beginspell6 as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(383), SkillPhase::Cast), PortedAnchor::Caster);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: MsgEffect.cpp:727-735 (CSkill::OnProcess
    /// UNT_SPIDERWEB, LaunchEffect at :729)
    #[test]
    fn curated_extra_pf_spiderweb_plays_bottomspider_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(405), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::BottomSpider as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(405), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: MsgEffect.cpp:1017-1025 (CSkill::OnProcess
    /// UNT_TATAMIGAESHI, LaunchEffect at :1019)
    #[test]
    fn curated_extra_nj_tatamigaeshi_plays_tatami_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(527), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::Tatami as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(527), SkillPhase::Ground), PortedAnchor::Target);
    }

    /// Curated extra lane: a `(skill, phase)` pair the `SkillEffect`
    /// coverage data cannot see, so `gen_ported_skill_map.py` curates it.
    ///
    /// Official launch: MsgEffect.cpp:1027-1033 (CSkill::OnProcess
    /// UNT_KAEN, LaunchEffect at :1030)
    #[test]
    fn curated_extra_nj_kaensin_plays_kaen_on_ground() {
        assert_eq!(
            ported_effect_for(SkillId(535), SkillPhase::Ground).map(|effect| effect as u32),
            Some(EffectId::Kaen as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(535), SkillPhase::Ground), PortedAnchor::Target);
    }

    #[test]
    fn wind_walk_success_lane_stays_coverage_driven() {
        // The second Wind Walk lane is not curated: `EF_PORTAL4` comes from
        // the coverage data (`successEffectId` -> `Success`,
        // `GameModePackets.cpp:19098`), so it is pinned by the coverage row.
        assert_eq!(
            ported_effect_for(SkillId(383), SkillPhase::Success).map(|effect| effect as u32),
            Some(EffectId::Portal4 as u32)
        );
        assert_eq!(ported_anchor_for(SkillId(383), SkillPhase::Success), PortedAnchor::Caster);
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
