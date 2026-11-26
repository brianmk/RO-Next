//! Registry entries owned by the "a" parallel port batch.
//!
//! Each batch touches only this file plus the body modules it creates, so
//! parallel work cannot collide on `registry.rs`.

use ragnarok_packets::EffectId;

use super::rag_batch_a::Steal;
use super::runtime::PortedEffect;

/// Creates the batch body for an effect id.
///
/// Only dispatch blocks that are a single call of a batch body are registered
/// here. `CRagEffect::TexturedForkBolt` is not registered: every block that
/// calls it (`EF_CHEMICAL_V`, `EF_CHEMICAL_V2`, `EF_NEW_METALLICSOUND_LINE`)
/// also calls `TexturedBeam`/`TexturedSpinShards` or branches on a client
/// option, so those effects are not ported yet.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    match effect {
        EffectId::Steal => Some(Box::new(Steal::default())),
        _ => None,
    }
}

/// Every effect id handled by this batch.
pub fn ids() -> &'static [EffectId] {
    &[EffectId::Steal]
}

/// `(effect, lifetime)` pairs, asserted against `EffectTypes.cpp` by tests.
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[(EffectId::Steal, 50)]
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
}
