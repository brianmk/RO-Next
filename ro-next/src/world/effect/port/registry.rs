//! Effect-id registry: `EF_*` id -> ported effect body.
//!
//! This is the table the client will consult once the ported backend is wired
//! (plan 4.2). It only contains bodies that need **no external assets**; the
//! `.str`-driven bodies (`Signum`, `CartRevolution`) are created by the asset
//! loader instead, because they need their clips at construction time.

use ragnarok_packets::EffectId;

use super::rag::{
    AgilityUp, Aqua, Bash, Blessing, BowlingBash, CartRevolution, DecAgility, Detoxication, EarthSpike, EnchantPoison, Endure, FireBall, FrostDiver, FrostDiver2, Hit2, IceWall, IncAgility, MagnumBreak, Overthrust,
    BlitzBeat, BottomCarpet, BottomLayer, BottomLayers, Cloud11Burst, GlassWall, GrimTooth, GrimToothAtk, NapalmBeat, StoneSpike, TexturedComet, PortalSpell, WaterBallBurst, EnchantPoisonCast, ForestLight6, TexturedPulseKind, FirePillar, FireSparks, TexturedHalo, TorchDustRing, FrostMysty, PortalRings, RingWave, RogueCoin, SpearBmr, WaterBall, HealBurst, MagicRing, TripleOrbit, PotionPill, RocketPunch, SprinkleSand, StormGust, VioletRingCloud, HealRingSparkle, Revive, Signum, SonicBlow, TexturedClassChange, TexturedQuadSpin, TexturedSkillAura, TexturedSpreadFlat, WhiteSpray,
};
use super::runtime::PortedEffect;

/// Creates the ported body for an effect id, if one exists.
///
/// Returns `None` for effects that are not ported yet **and** for the ported
/// bodies that need `.str` clips (`EF_SIGNUM`, `EF_CARTREVOLUTION`); see
/// [`ported_ids`] for the asset-free set.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    if let Some(body) = super::registry_class_change::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_textured_aura::spawn(effect) {
        return Some(body);
    }

    Some(match effect {
        EffectId::Hit2 => Box::new(Hit2::new(0)),
        EffectId::Endure => Box::new(Endure::default()),
        EffectId::Enchantpoison => Box::new(EnchantPoison::default()),
        EffectId::Detoxication => Box::new(Detoxication::default()),
        EffectId::Agiup => Box::new(AgilityUp::default()),
        EffectId::Detecting => Box::new(TexturedSkillAura::new(0, 250)),
        EffectId::Detect2 => Box::new(TexturedSkillAura::new(2, 250)),
        EffectId::Detect3 => Box::new(TexturedSkillAura::new(3, 250)),
        EffectId::Detect4 => Box::new(TexturedSkillAura::new(4, 250)),
        EffectId::Redline => Box::new(TexturedSpreadFlat::new("effect\\ring_red.tga", 3, 3, 75)),
        EffectId::Blueline => Box::new(TexturedSpreadFlat::new("effect\\ring_blue.tga", 12, 6, 999)),
        EffectId::Bash3d5 => Box::new(TexturedQuadSpin::new("effect\\alpha_center.tga", 5, 6, 200)),
        EffectId::Bash3d6 => Box::new(TexturedQuadSpin::new("effect\\alpha_center.tga", 6, 8, 200)),
        EffectId::Heal => Box::new(HealBurst::new(0, 100)),
        EffectId::Heal3 => Box::new(HealBurst::new(1, 100)),
        EffectId::Heal2 => Box::new(HealRingSparkle::new("effect\\ring_white.tga", 0, 100)),
        EffectId::Portal2 => Box::new(VioletRingCloud::new(0, 9999)),
        EffectId::Portal3 => Box::new(VioletRingCloud::new(3, 9999)),
        EffectId::BigPortal => Box::new(MagicRing::new(0, 2000)),
        EffectId::BigPortal2 => Box::new(MagicRing::new(1, 99_999_999)),
        EffectId::Stormgust => Box::new(StormGust::default()),
        EffectId::Fireball3 => Box::new(RocketPunch::new(false, 100)),
        EffectId::Fireball6 => Box::new(RocketPunch::new(true, 100)),
        EffectId::Sprinklesand => Box::new(SprinkleSand::new(20, 300)),
        EffectId::PotionBerserk => Box::new(PotionPill::new(false, 199)),
        EffectId::PotionBerserk2 => Box::new(PotionPill::new(true, 199)),
        EffectId::Firepillaron2 => Box::new(FirePillar::new(90)),
        EffectId::GradiusLaser => Box::new(TripleOrbit::new("effect\\white02.bmp", 500)),
        EffectId::RgCoin => Box::new(RogueCoin::coin(300)),
        EffectId::RgCoin2 => Box::new(RogueCoin::coin2(300)),
        EffectId::RgCoin3 => Box::new(RogueCoin::coin3(300)),
        EffectId::RgCoin4 => Box::new(RogueCoin::coin4(300)),
        EffectId::RgCoin5 => Box::new(RogueCoin::coin5(300)),
        EffectId::RgCoin6 => Box::new(RogueCoin::coin6(300)),
        EffectId::RgCoin7 => Box::new(RogueCoin::coin7(300)),
        EffectId::Frostmysty => Box::new(FrostMysty::new(300)),
        EffectId::Entry2 => Box::new(PortalRings::new(100)),
        EffectId::Readyportal2 => Box::new(RingWave::blue(200)),
        EffectId::Waterball => Box::new(WaterBall::new(250)),
        EffectId::Spearbmr => Box::new(SpearBmr::new(250)),
        EffectId::Edp => Box::new(EnchantPoisonCast::default()),
        EffectId::Firefly => Box::new(FireSparks::default()),
        EffectId::BottomRunner => Box::new(TexturedHalo::new("effect\\hanmoon1.tga", 3, 29999)),
        EffectId::BottomTransfer => Box::new(TexturedHalo::new("effect\\hanmoon2.tga", 3, 29999)),
        EffectId::BottomLa => Box::new(TexturedHalo::new("effect\\aaa copy.bmp", 1, 29999)),
        EffectId::BottomSpider => Box::new(TexturedHalo::new("effect\\spiderweb.tga", 2, 29999)),
        EffectId::WaterBelow => Box::new(TorchDustRing::new(2, 0, 99_999_999)),
        EffectId::BottomManhole => Box::new(BottomLayers::new(BottomLayer::Manhole, 29999)),
        EffectId::BottomAni => Box::new(BottomLayers::new(BottomLayer::Ani, 29999)),
        EffectId::BottomBloodylust => Box::new(BottomLayers::new(BottomLayer::BloodyLust, 29999)),
        EffectId::Forestlight6 => Box::new(ForestLight6::new(29999)),
        EffectId::BottomMag => Box::new(TexturedPulseKind::new("effect\\ring_red.tga", 0, 29999)),
        EffectId::BottomSanc => Box::new(TexturedPulseKind::new("effect\\alpha_down.tga", 1, 29999)),
        EffectId::Portal => Box::new(PortalSpell::new(100)),
        EffectId::Bottom => Box::new(BottomCarpet::violet()),
        EffectId::Bottom2 => Box::new(BottomCarpet::green()),
        EffectId::Glasswall => Box::new(GlassWall::new(9999)),
        EffectId::Waterball3 => Box::new(WaterBallBurst::new(100)),
        EffectId::Grimtooth => Box::new(GrimTooth::new(250)),
        EffectId::Grimtoothatk => Box::new(GrimToothAtk::default()),
        EffectId::Heavensdrive => Box::new(StoneSpike::new(250)),
        EffectId::Napalmbeat => Box::new(NapalmBeat::default()),
        EffectId::Sma3 => Box::new(TexturedComet::new(1, 50)),
        EffectId::Blitzbeat => Box::new(BlitzBeat::new(250)),
        EffectId::Portal4 => Box::new(Cloud11Burst::new(0, 200).with_waves(&[(0, "effect\\\\윈드워크.wav")])),
        EffectId::Portal5 => Box::new(Cloud11Burst::new(1, 200)),
        EffectId::Intimidate => Box::new(
            WhiteSpray::new(0, 20, 999).with_waves(&[(0, "effect\\EF_Bash.wav"), (35, "effect\\rog_intimidate.wav")]),
        ),
        EffectId::Intimidate2 => Box::new(WhiteSpray::new(2, 16, 999)),
        EffectId::Intimidate3 => Box::new(
            WhiteSpray::new(3, 20, 999).with_waves(&[(0, "effect\\EF_Bash.wav"), (35, "effect\\rog_intimidate.wav")]),
        ),
        EffectId::Intimidate4 => Box::new(WhiteSpray::new(4, 10, 999).with_waves(&[(25, "effect\\triangle.wav")])),
        EffectId::Aqua => Box::new(Aqua::default()),
        EffectId::Revive => Box::new(Revive::default()),
        EffectId::Fireball => Box::new(FireBall::default()),
        EffectId::Decagility => Box::new(DecAgility::default()),
        EffectId::Blessing => Box::new(Blessing::default()),
        EffectId::Bash => Box::new(Bash::default()),
        EffectId::Magnumbreak => Box::new(MagnumBreak::default()),
        EffectId::Frostdiver => Box::new(FrostDiver::new(0)),
        EffectId::Frostdiver2 => Box::new(FrostDiver2::default()),
        EffectId::Frostdiver3 => Box::new(FrostDiver::new(3)),
        EffectId::Incagility => Box::new(IncAgility::new(false)),
        EffectId::Incagidex => Box::new(IncAgility::new(true)),
        EffectId::Bowlingbash => Box::new(BowlingBash::default()),
        EffectId::Icewall => Box::new(IceWall::default()),
        EffectId::Earthspike => Box::new(EarthSpike::new(0)),
        EffectId::Hyousensou => Box::new(EarthSpike::new(2)),
        EffectId::Sonicblow => Box::new(SonicBlow::default()),
        EffectId::Overthrust => Box::new(Overthrust::default()),
        _ => return None,
    })
}

/// Effect ids whose body needs `.str` clips; constructed by the asset loader,
/// not by [`spawn`].
pub fn needs_str_clips(effect: EffectId) -> bool {
    matches!(effect, EffectId::Signum | EffectId::Cartrevolution)
}

/// Every ported, asset-free effect id.
pub fn ported_ids() -> Vec<EffectId> {
    let mut ids = super::registry_class_change::ids().to_vec();
    ids.extend_from_slice(super::registry_textured_aura::ids());

    ids.extend_from_slice(&[
        EffectId::Hit2,
        EffectId::Endure,
        EffectId::Blessing,
        EffectId::Fireball,
        EffectId::Decagility,
        EffectId::Aqua,
        EffectId::Revive,
        EffectId::Enchantpoison,
        EffectId::Detoxication,
        EffectId::Agiup,
        EffectId::Detecting,
        EffectId::Detect2,
        EffectId::Detect3,
        EffectId::Detect4,
        EffectId::Redline,
        EffectId::Blueline,
        EffectId::Bash3d5,
        EffectId::Bash3d6,
        EffectId::Heal,
        EffectId::Heal3,
        EffectId::Heal2,
        EffectId::Portal2,
        EffectId::Portal3,
        EffectId::BigPortal,
        EffectId::BigPortal2,
        EffectId::Stormgust,
        EffectId::Fireball3,
        EffectId::Fireball6,
        EffectId::Sprinklesand,
        EffectId::PotionBerserk,
        EffectId::PotionBerserk2,
        EffectId::Firepillaron2,
        EffectId::GradiusLaser,
        EffectId::RgCoin,
        EffectId::RgCoin2,
        EffectId::RgCoin3,
        EffectId::RgCoin4,
        EffectId::RgCoin5,
        EffectId::RgCoin6,
        EffectId::RgCoin7,
        EffectId::Frostmysty,
        EffectId::Entry2,
        EffectId::Readyportal2,
        EffectId::Waterball,
        EffectId::Spearbmr,
        EffectId::Edp,
        EffectId::Firefly,
        EffectId::BottomRunner,
        EffectId::BottomTransfer,
        EffectId::BottomLa,
        EffectId::BottomSpider,
        EffectId::WaterBelow,
        EffectId::BottomManhole,
        EffectId::BottomAni,
        EffectId::BottomBloodylust,
        EffectId::Forestlight6,
        EffectId::BottomMag,
        EffectId::BottomSanc,
        EffectId::Portal,
        EffectId::Bottom,
        EffectId::Bottom2,
        EffectId::Glasswall,
        EffectId::Waterball3,
        EffectId::Grimtooth,
        EffectId::Grimtoothatk,
        EffectId::Heavensdrive,
        EffectId::Napalmbeat,
        EffectId::Sma3,
        EffectId::Blitzbeat,
        EffectId::Portal4,
        EffectId::Portal5,
        EffectId::Intimidate,
        EffectId::Intimidate2,
        EffectId::Intimidate3,
        EffectId::Intimidate4,
        EffectId::Rapidshower,
        EffectId::Magicalbullet,
        EffectId::Spreadattack,
        EffectId::Tracking,
        EffectId::Tripleaction,
        EffectId::Kirikage,
        EffectId::Kasumikiri,
        EffectId::Issen,
        EffectId::M02,
        EffectId::M07,
        EffectId::NpcEarthquake,
        EffectId::PokLove,
        EffectId::SilentBreeze,
        EffectId::Stonering,
        EffectId::Vacuum,
        EffectId::Bash,
        EffectId::Magnumbreak,
        EffectId::Frostdiver,
        EffectId::Frostdiver2,
        EffectId::Frostdiver3,
        EffectId::Incagility,
        EffectId::Incagidex,
        EffectId::Bowlingbash,
        EffectId::Icewall,
        EffectId::Earthspike,
        EffectId::Hyousensou,
        EffectId::Sonicblow,
        EffectId::Overthrust,
    ]);

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ported_id_spawns_with_its_official_lifetime() {
        for effect in ported_ids() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert!(body.lifetime() > 0, "{effect:?} has no lifetime");
        }
    }

    #[test]
    fn lifetimes_match_the_lifetime_table() {
        let expect = [
            (EffectId::Hit2, 50),
            (EffectId::Endure, 80),
            (EffectId::Blessing, 150),
            (EffectId::Fireball, 100),
            (EffectId::Decagility, 100),
            (EffectId::Aqua, 100),
            (EffectId::Revive, 250),
            (EffectId::Enchantpoison, 250),
            (EffectId::Detoxication, 100),
            (EffectId::Agiup, 100),
            (EffectId::Detecting, 250),
            (EffectId::Detect2, 250),
            (EffectId::Detect3, 250),
            (EffectId::Detect4, 250),
            (EffectId::Redline, 75),
            (EffectId::Blueline, 999),
            (EffectId::Bash3d5, 200),
            (EffectId::Bash3d6, 200),
            (EffectId::Heal, 100),
            (EffectId::Heal3, 100),
            (EffectId::Heal2, 100),
            (EffectId::Portal2, 9999),
            (EffectId::Portal3, 9999),
            (EffectId::BigPortal, 2000),
            (EffectId::BigPortal2, 99_999_999),
            (EffectId::Stormgust, 9999),
            (EffectId::Fireball3, 100),
            (EffectId::Fireball6, 100),
            (EffectId::Sprinklesand, 300),
            (EffectId::PotionBerserk, 199),
            (EffectId::PotionBerserk2, 199),
            (EffectId::Firepillaron2, 90),
            (EffectId::GradiusLaser, 500),
            (EffectId::RgCoin, 300),
            (EffectId::RgCoin2, 300),
            (EffectId::RgCoin3, 300),
            (EffectId::RgCoin4, 300),
            (EffectId::RgCoin5, 300),
            (EffectId::RgCoin6, 300),
            (EffectId::RgCoin7, 300),
            (EffectId::Frostmysty, 300),
            (EffectId::Entry2, 100),
            (EffectId::Readyportal2, 200),
            (EffectId::Waterball, 250),
            (EffectId::Spearbmr, 250),
            (EffectId::Edp, 120),
            (EffectId::Firefly, 100),
            (EffectId::BottomRunner, 29999),
            (EffectId::BottomTransfer, 29999),
            (EffectId::BottomLa, 29999),
            (EffectId::BottomSpider, 29999),
            (EffectId::WaterBelow, 99_999_999),
            (EffectId::BottomManhole, 29999),
            (EffectId::BottomAni, 29999),
            (EffectId::BottomBloodylust, 29999),
            (EffectId::Forestlight6, 29999),
            (EffectId::BottomMag, 29999),
            (EffectId::BottomSanc, 29999),
            (EffectId::Portal, 100),
            (EffectId::Bottom, 350),
            (EffectId::Bottom2, 350),
            (EffectId::Glasswall, 9999),
            (EffectId::Waterball3, 100),
            (EffectId::Grimtooth, 250),
            (EffectId::Grimtoothatk, 250),
            (EffectId::Heavensdrive, 250),
            (EffectId::Napalmbeat, 100),
            (EffectId::Sma3, 50),
            (EffectId::Blitzbeat, 250),
            (EffectId::Portal4, 200),
            (EffectId::Portal5, 200),
            (EffectId::Intimidate, 999),
            (EffectId::Intimidate2, 999),
            (EffectId::Intimidate3, 999),
            (EffectId::Intimidate4, 999),
            (EffectId::Rapidshower, 100),
            (EffectId::Magicalbullet, 100),
            (EffectId::Spreadattack, 100),
            (EffectId::Tracking, 100),
            (EffectId::Tripleaction, 100),
            (EffectId::Kirikage, 100),
            (EffectId::Kasumikiri, 100),
            (EffectId::Issen, 100),
            (EffectId::M02, 100),
            (EffectId::M07, 100),
            (EffectId::NpcEarthquake, 100),
            (EffectId::PokLove, 100),
            (EffectId::SilentBreeze, 100),
            (EffectId::Stonering, 300),
            (EffectId::Vacuum, 1000),
            (EffectId::Bash, 40),
            (EffectId::Magnumbreak, 30),
            (EffectId::Frostdiver, 150),
            (EffectId::Frostdiver2, 100),
            (EffectId::Frostdiver3, 150),
            (EffectId::Incagility, 100),
            (EffectId::Incagidex, 100),
            (EffectId::Bowlingbash, 250),
            (EffectId::Icewall, 250),
            (EffectId::Earthspike, 250),
            (EffectId::Hyousensou, 250),
            (EffectId::Sonicblow, 40),
            (EffectId::Overthrust, 250),
        ];

        for (effect, lifetime) in expect {
            let body = spawn(effect).unwrap();
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn str_driven_bodies_are_not_spawned_without_assets() {
        assert!(needs_str_clips(EffectId::Signum));
        assert!(needs_str_clips(EffectId::Cartrevolution));
        assert!(spawn(EffectId::Signum).is_none());
        assert!(spawn(EffectId::Cartrevolution).is_none());
    }

    #[test]
    fn every_registered_body_assigns_its_official_texture() {
        use crate::world::effect::port::primitive::PrimitiveId;
        use crate::world::effect::port::runtime::{step, EffectRuntime};

        for effect in ported_ids() {
            let mut body = spawn(effect).unwrap();
            let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 99);

            // Some bodies only start emitting after a delay (Fire Ball at frame
            // 20), so run a few frames before requiring primitives.
            // 160 frames covers the bodies whose first primitive is late
            // (Storm Gust's first fragment lands on frame 58).
            for _ in 0..160 {
                step(&mut body, &mut runtime);

                if !runtime.primitives.is_empty() {
                    break;
                }
            }

            assert!(!runtime.primitives.is_empty(), "{effect:?} spawned nothing");

            for primitive in &runtime.primitives {
                if primitive.state.count == PrimitiveId::Particle3d {
                    // Sprite particles carry `.spr/.act`, not a texture.
                    continue;
                }

                assert!(
                    primitive.state.texture.is_some(),
                    "{effect:?} left a {:?} primitive without a texture",
                    primitive.state.count
                );
            }
        }
    }

    #[test]
    fn key_textures_match_the_decompilation() {
        use crate::world::effect::port::runtime::{step, EffectRuntime};

        let mut bash = spawn(EffectId::Bash).unwrap();
        let mut runtime = EffectRuntime::new(bash.lifetime(), [0.0, 0.0], 1);
        step(&mut bash, &mut runtime);

        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\alpha_down.tga"));
        assert_eq!(runtime.primitives[1].state.texture, Some("EFFECT\\alpha_center.tga"));

        let mut bowling = spawn(EffectId::Bowlingbash).unwrap();
        let mut runtime = EffectRuntime::new(bowling.lifetime(), [0.0, 0.0], 1);
        step(&mut bowling, &mut runtime);

        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(runtime.primitives[1].state.texture, Some("effect\\ring_yellow.tga"));
    }

    #[test]
    fn unported_effects_return_none() {
        assert!(spawn(EffectId::Thunderstorm).is_none());
        assert!(!ported_ids().iter().any(|effect| !needs_str_clips(*effect) && spawn(*effect).is_none()));
    }
}
