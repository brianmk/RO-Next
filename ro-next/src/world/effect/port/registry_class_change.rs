//! Registry entries for every effect that is an `TexturedClassChange` call.
//!
//! Generated from the dispatch switch of `RagEffect.cpp`: effect id, the
//! `TexturedClassChange` kinds it calls, its lifetime table entry, and the
//! frame-scheduled wave/heartbeat sounds of its block. `EF_HAPGYEOK` is
//! excluded because its block also drives `ProcessEZ2STR`.

use ragnarok_packets::EffectId;

use super::rag::TexturedClassChange;
use super::runtime::PortedEffect;

/// Creates the class-change body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        EffectId::ElGust => Box::new(TexturedClassChange::new(&[70], 100)),
        EffectId::SilentBreeze => Box::new(TexturedClassChange::new(&[82], 100)),
        EffectId::MagmaFlow => Box::new(TexturedClassChange::new(&[83], 100)),
        EffectId::ElBlast => Box::new(TexturedClassChange::new(&[71], 100)),
        EffectId::ElAquaplay => Box::new(TexturedClassChange::new(&[72], 100)),
        EffectId::ElUpheaval => Box::new(TexturedClassChange::new(&[73], 100)),
        EffectId::ElWildStorm => Box::new(TexturedClassChange::new(&[74], 100)),
        EffectId::ElChillyAir => Box::new(TexturedClassChange::new(&[75], 100)),
        EffectId::ElCursedSoil => Box::new(TexturedClassChange::new(&[76], 100)),
        EffectId::ElCooler => Box::new(TexturedClassChange::new(&[77], 100)),
        EffectId::ElTropic => Box::new(TexturedClassChange::new(&[78], 100)),
        EffectId::ElPyrotechnic => Box::new(TexturedClassChange::new(&[79], 100)),
        EffectId::ElPetrology => Box::new(TexturedClassChange::new(&[80], 100)),
        EffectId::ElHeater => Box::new(TexturedClassChange::new(&[81], 100)),
        EffectId::SprMash => Box::new(TexturedClassChange::new(&[38], 100)),
        EffectId::Stonering => Box::new(TexturedClassChange::new(&[60], 300).with_waves(&[(0, "effect\\wl_siennaexecrate.wav")])),
        EffectId::MagentaTrap => Box::new(TexturedClassChange::new(&[53], 23)),
        EffectId::CobaltTrap => Box::new(TexturedClassChange::new(&[52], 23)),
        EffectId::MaizeTrap => Box::new(TexturedClassChange::new(&[51], 23)),
        EffectId::VerdureTrap => Box::new(TexturedClassChange::new(&[54], 23)),
        EffectId::NormalTrap => Box::new(TexturedClassChange::new(&[50], 23)),
        EffectId::SprSoule => Box::new(TexturedClassChange::new(&[39], 100)),
        EffectId::Item318 => Box::new(TexturedClassChange::new(&[37], 100)),
        EffectId::PokLove => Box::new(TexturedClassChange::new(&[32], 100)),
        EffectId::PokWhite => Box::new(TexturedClassChange::new(&[33], 100)),
        EffectId::PokValen => Box::new(TexturedClassChange::new(&[34], 100)),
        EffectId::PokBirth => Box::new(TexturedClassChange::new(&[35], 100)),
        EffectId::PokChristmas => Box::new(TexturedClassChange::new(&[36], 100)),
        EffectId::NpcEarthquake => Box::new(
            TexturedClassChange::new(&[31], 100)
                .with_waves(&[(0, "effect\\earth_quake.wav")])
                .with_quakes(&[0]),
        ),
        EffectId::Issen => Box::new(TexturedClassChange::new(&[18], 100).with_waves(&[(0, "effect\\일섬.wav")])),
        EffectId::Kasumikiri => Box::new(TexturedClassChange::new(&[17], 100).with_waves(&[(0, "effect\\안개베기.wav")])),
        EffectId::Kirikage => Box::new(TexturedClassChange::new(&[15], 100).with_waves(&[(0, "effect\\그림자베기.wav")])),
        EffectId::Spreadattack => Box::new(TexturedClassChange::new(&[28], 100)),
        EffectId::Rapidshower => Box::new(TexturedClassChange::new(&[26], 100).with_waves(&[(20, "effect\\래피드샤워.wav")])),
        EffectId::Magicalbullet => Box::new(TexturedClassChange::new(&[27], 100).with_waves(&[(0, "effect\\매지컬블릿.wav")])),
        EffectId::Tripleaction => Box::new(TexturedClassChange::new(&[30], 100).with_waves(&[(25, "effect\\트리플액션.wav")])),
        EffectId::Tracking => Box::new(TexturedClassChange::new(&[29], 100)),
        EffectId::LightningS => Box::new(TexturedClassChange::new(&[21], 23)),
        EffectId::Wallofthorn => Box::new(TexturedClassChange::new(&[61], 1000)),
        EffectId::Vacuum => Box::new(TexturedClassChange::new(&[64, 65], 1000)),
        EffectId::BlindS => Box::new(TexturedClassChange::new(&[22], 23)),
        EffectId::PoisonS => Box::new(TexturedClassChange::new(&[23], 23)),
        EffectId::FreezingS => Box::new(TexturedClassChange::new(&[24], 23)),
        EffectId::FlareS => Box::new(TexturedClassChange::new(&[25], 23)),
        EffectId::Tatami => Box::new(TexturedClassChange::new(&[16], 100)),
        EffectId::Kaen => Box::new(TexturedClassChange::new(&[19], 23)),
        EffectId::Desperado => Box::new(TexturedClassChange::new(&[20], 200).with_waves(&[(10, "effect\\데스페라도.wav")])),
        EffectId::Hamicastle => Box::new(TexturedClassChange::new(&[0], 100)),
        EffectId::Hamiblood => Box::new(TexturedClassChange::new(&[1], 100)),
        EffectId::ItemThunder => Box::new(TexturedClassChange::new(&[2], 100)),
        EffectId::ItemCloud => Box::new(TexturedClassChange::new(&[3], 100)),
        EffectId::ItemCurse => Box::new(TexturedClassChange::new(&[4], 100)),
        EffectId::ItemZzz => Box::new(TexturedClassChange::new(&[5], 100)),
        EffectId::ItemRain => Box::new(TexturedClassChange::new(&[6], 100)),
        EffectId::M01 => Box::new(TexturedClassChange::new(&[7], 50)),
        EffectId::M02 => Box::new(TexturedClassChange::new(&[8], 100).with_waves(&[(5, "effect\\풀버스터.wav")])),
        EffectId::M03 => Box::new(TexturedClassChange::new(&[9], 100)),
        EffectId::M04 => Box::new(TexturedClassChange::new(&[10], 99999999)),
        EffectId::M05 => Box::new(TexturedClassChange::new(&[11], 100)),
        EffectId::M06 => Box::new(TexturedClassChange::new(&[12], 100)),
        EffectId::M07 => Box::new(TexturedClassChange::new(&[13], 100)),
        EffectId::Icemine => Box::new(TexturedClassChange::new(&[84], 999)),
        EffectId::Icemine1 => Box::new(TexturedClassChange::new(&[85], 999)),
        EffectId::RkDragonbreathWater => Box::new(TexturedClassChange::new(&[86], 999)),
        EffectId::NpcWideweb => Box::new(TexturedClassChange::new(&[87], 999)),
        EffectId::NpcBurnt => Box::new(TexturedClassChange::new(&[88], 9999)),
        _ => return None,
    })
}

/// Every effect id handled here.
pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::ElGust,
        EffectId::SilentBreeze,
        EffectId::MagmaFlow,
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
        EffectId::SprMash,
        EffectId::Stonering,
        EffectId::MagentaTrap,
        EffectId::CobaltTrap,
        EffectId::MaizeTrap,
        EffectId::VerdureTrap,
        EffectId::NormalTrap,
        EffectId::SprSoule,
        EffectId::Item318,
        EffectId::PokLove,
        EffectId::PokWhite,
        EffectId::PokValen,
        EffectId::PokBirth,
        EffectId::PokChristmas,
        EffectId::NpcEarthquake,
        EffectId::Issen,
        EffectId::Kasumikiri,
        EffectId::Kirikage,
        EffectId::Spreadattack,
        EffectId::Rapidshower,
        EffectId::Magicalbullet,
        EffectId::Tripleaction,
        EffectId::Tracking,
        EffectId::LightningS,
        EffectId::Wallofthorn,
        EffectId::Vacuum,
        EffectId::BlindS,
        EffectId::PoisonS,
        EffectId::FreezingS,
        EffectId::FlareS,
        EffectId::Tatami,
        EffectId::Kaen,
        EffectId::Desperado,
        EffectId::Hamicastle,
        EffectId::Hamiblood,
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
        EffectId::Icemine,
        EffectId::Icemine1,
        EffectId::RkDragonbreathWater,
        EffectId::NpcWideweb,
        EffectId::NpcBurnt,
    ]
}

/// `(effect, lifetime)` pairs, asserted against `EffectTypes.cpp` by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        (EffectId::ElGust, 100),
        (EffectId::SilentBreeze, 100),
        (EffectId::MagmaFlow, 100),
        (EffectId::ElBlast, 100),
        (EffectId::ElAquaplay, 100),
        (EffectId::ElUpheaval, 100),
        (EffectId::ElWildStorm, 100),
        (EffectId::ElChillyAir, 100),
        (EffectId::ElCursedSoil, 100),
        (EffectId::ElCooler, 100),
        (EffectId::ElTropic, 100),
        (EffectId::ElPyrotechnic, 100),
        (EffectId::ElPetrology, 100),
        (EffectId::ElHeater, 100),
        (EffectId::SprMash, 100),
        (EffectId::Stonering, 300),
        (EffectId::MagentaTrap, 23),
        (EffectId::CobaltTrap, 23),
        (EffectId::MaizeTrap, 23),
        (EffectId::VerdureTrap, 23),
        (EffectId::NormalTrap, 23),
        (EffectId::SprSoule, 100),
        (EffectId::Item318, 100),
        (EffectId::PokLove, 100),
        (EffectId::PokWhite, 100),
        (EffectId::PokValen, 100),
        (EffectId::PokBirth, 100),
        (EffectId::PokChristmas, 100),
        (EffectId::NpcEarthquake, 100),
        (EffectId::Issen, 100),
        (EffectId::Kasumikiri, 100),
        (EffectId::Kirikage, 100),
        (EffectId::Spreadattack, 100),
        (EffectId::Rapidshower, 100),
        (EffectId::Magicalbullet, 100),
        (EffectId::Tripleaction, 100),
        (EffectId::Tracking, 100),
        (EffectId::LightningS, 23),
        (EffectId::Wallofthorn, 1000),
        (EffectId::Vacuum, 1000),
        (EffectId::BlindS, 23),
        (EffectId::PoisonS, 23),
        (EffectId::FreezingS, 23),
        (EffectId::FlareS, 23),
        (EffectId::Tatami, 100),
        (EffectId::Kaen, 23),
        (EffectId::Desperado, 200),
        (EffectId::Hamicastle, 100),
        (EffectId::Hamiblood, 100),
        (EffectId::ItemThunder, 100),
        (EffectId::ItemCloud, 100),
        (EffectId::ItemCurse, 100),
        (EffectId::ItemZzz, 100),
        (EffectId::ItemRain, 100),
        (EffectId::M01, 50),
        (EffectId::M02, 100),
        (EffectId::M03, 100),
        (EffectId::M04, 99999999),
        (EffectId::M05, 100),
        (EffectId::M06, 100),
        (EffectId::M07, 100),
        (EffectId::Icemine, 999),
        (EffectId::Icemine1, 999),
        (EffectId::RkDragonbreathWater, 999),
        (EffectId::NpcWideweb, 999),
        (EffectId::NpcBurnt, 9999),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn every_entry_spawns_with_its_lifetime() {
        for &(effect, lifetime) in lifetimes() {
            let mut body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn scheduled_waves_use_single_backslash_paths_and_their_frames() {
        let mut body = spawn(EffectId::Issen).unwrap();
        let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 3);

        step(body.as_mut(), &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\일섬.wav".to_owned()]);

        // Triple Action plays its wave on frame 25, not at spawn.
        let mut body = spawn(EffectId::Tripleaction).unwrap();
        let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 3);
        for _ in 0..25 {
            step(body.as_mut(), &mut runtime);
        }
        assert!(runtime.sounds.is_empty());
        step(body.as_mut(), &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\트리플액션.wav".to_owned()]);
    }

    #[test]
    fn every_entry_produces_its_sprite() {
        for &effect in ids() {
            let mut body = spawn(effect).unwrap();
            let lifetime = body.lifetime();
            let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], 7);

            for _ in 0..40 {
                step(body.as_mut(), &mut runtime);
                if !runtime.primitives.is_empty() {
                    break;
                }
            }

            assert!(!runtime.primitives.is_empty(), "{effect:?} spawned nothing");
            assert!(
                runtime.primitives.iter().all(|p| p.state.sprite.is_some()),
                "{effect:?} spawned a primitive without a sprite"
            );
        }
    }
}
