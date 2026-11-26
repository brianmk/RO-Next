//! Effect-id registry: `EF_*` id -> ported effect body.
//!
//! This is the table the client will consult once the ported backend is wired
//! (plan 4.2). It only contains bodies that need **no external assets**; the
//! `.str`-driven bodies (`Signum`, `CartRevolution`) are created by the asset
//! loader instead, because they need their clips at construction time.

use ragnarok_packets::EffectId;

use super::rag::{
    Bash, Blessing, BowlingBash, CartRevolution, EarthSpike, Endure, FrostDiver, FrostDiver2, Hit2, IceWall, IncAgility, MagnumBreak, Overthrust,
    Signum, SonicBlow,
};
use super::runtime::PortedEffect;

/// Creates the ported body for an effect id, if one exists.
///
/// Returns `None` for effects that are not ported yet **and** for the ported
/// bodies that need `.str` clips (`EF_SIGNUM`, `EF_CARTREVOLUTION`); see
/// [`ported_ids`] for the asset-free set.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        EffectId::Hit2 => Box::new(Hit2::new(0)),
        EffectId::Endure => Box::new(Endure::default()),
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
pub fn ported_ids() -> &'static [EffectId] {
    &[
        EffectId::Hit2,
        EffectId::Endure,
        EffectId::Blessing,
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
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ported_id_spawns_with_its_official_lifetime() {
        for &effect in ported_ids() {
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
    fn unported_effects_return_none() {
        assert!(spawn(EffectId::Fireball).is_none());
        assert!(!ported_ids().iter().any(|effect| !needs_str_clips(*effect) && spawn(*effect).is_none()));
    }
}
