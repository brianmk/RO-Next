//! Registry entries owned by the "i" parallel port batch.

use ragnarok_packets::EffectId;

use super::runtime::PortedEffect;

pub fn spawn(_effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    None
}

pub fn ids() -> &'static [EffectId] {
    &[]
}

pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[]
}

/// Effects that only touch the caster actor: no primitive, no `.str` clip.
pub fn is_caster_body_only(_effect: EffectId) -> bool {
    false
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
