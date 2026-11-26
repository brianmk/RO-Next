//! Registry entries owned by the "h" parallel port batch.

use ragnarok_packets::EffectId;

use super::rag_batch_h::{LinkLight, MagicCrasher, Memorize};
use super::runtime::PortedEffect;

pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    match effect {
        EffectId::Magiccrasher => Some(Box::new(MagicCrasher)),
        EffectId::Memorize => Some(Box::new(Memorize)),
        EffectId::Linklight => Some(Box::new(LinkLight)),
        _ => None,
    }
}

pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Magiccrasher,
        EffectId::Memorize,
        EffectId::Linklight,
    ]
}

pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        (EffectId::Magiccrasher, 100),
        (EffectId::Memorize, 200),
        (EffectId::Linklight, 200),
    ]
}

/// Effects that only touch the caster actor: no primitive, no `.str` clip.
///
/// The manual (T3) tier is entirely this shape, so the shared registry
/// invariant consults this list instead of every batch editing `registry.rs`.
pub fn is_caster_body_only(effect: EffectId) -> bool {
    matches!(effect, EffectId::Magiccrasher | EffectId::Memorize | EffectId::Linklight)
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
    fn caster_body_only_ids_are_registered() {
        for &id in ids() {
            assert!(is_caster_body_only(id), "{id:?}");
        }
    }
}
