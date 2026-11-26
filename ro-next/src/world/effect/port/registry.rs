//! Effect-id registry: `EF_*` id -> ported effect body.
//!
//! This is the table the client will consult once the ported backend is wired
//! (plan 4.2). It only contains bodies that need **no external assets**; the
//! `.str`-driven bodies (`Signum`, `CartRevolution`) are created by the asset
//! loader instead, because they need their clips at construction time.

use ragnarok_packets::EffectId;

use super::rag::{
    AgilityUp, Aqua, Bash, Blessing, BowlingBash, CartRevolution, DecAgility, Detoxication, EarthSpike, EnchantPoison, Endure, FireBall, FrostDiver, FrostDiver2, Hit2, IceWall, IncAgility, MagnumBreak, Overthrust,
    Cloud11Burst, Revive, Signum, SonicBlow, TexturedClassChange, WhiteSpray,
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

    Some(match effect {
        EffectId::Hit2 => Box::new(Hit2::new(0)),
        EffectId::Endure => Box::new(Endure::default()),
        EffectId::Enchantpoison => Box::new(EnchantPoison::default()),
        EffectId::Detoxication => Box::new(Detoxication::default()),
        EffectId::Agiup => Box::new(AgilityUp::default()),
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
            for _ in 0..40 {
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
