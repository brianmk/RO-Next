//! Registry entries owned by the "d" parallel port batch.
//!
//! Each batch touches only this file plus the body modules it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag_batch_d::{Hated2, NpcSlowcast, TempOk, Windcutter};
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `case EF_NPC_SLOWCAST: this->TexturedAura("effect\\blast_mine##clock.bmp", 4);`
        // + `PlayEffectWave("effect\\EF_DecAgility.wav")` on frame 10.
        EffectId::NpcSlowcast => Box::new(NpcSlowcast::new(300)),
        // `case EF_WINDCUTTER: PlayEffectWave("effect\\wl_siennaexecrate.wav")`
        // on frame 1 + `this->TexturedSkillAura(8);`
        EffectId::Windcutter => Box::new(Windcutter::new(100)),
        // `case EF_TEMP_OK: PlayEffectWave("_heal_effect.wav")` on frame 0
        // + `this->TexturedAura("effect\\success.bmp", 3);`
        EffectId::TempOk => Box::new(TempOk::new(100)),
        // `case EF_HATED2: PlayEffectWave("effect\\T_따듯한마법.wav")` on frame
        // 10 + `this->ParticleTime("effect\\white02.bmp", 3);`
        EffectId::Hated2 => Box::new(Hated2::new(300)),
        _ => return None,
    })
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[EffectId::NpcSlowcast, EffectId::Windcutter, EffectId::TempOk, EffectId::Hated2]
}

/// `(effect, lifetime)` pairs from `EffectTypes.cpp`; asserted by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:685`
        (EffectId::NpcSlowcast, 300),
        // `EffectTypes.cpp:733`
        (EffectId::Windcutter, 100),
        // `EffectTypes.cpp:619`
        (EffectId::TempOk, 100),
        // `EffectTypes.cpp:581`
        (EffectId::Hated2, 300),
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
    fn unowned_effects_are_not_claimed() {
        assert!(spawn(EffectId::Bash).is_none());
        assert!(spawn(EffectId::TempFail).is_none());
    }
}
