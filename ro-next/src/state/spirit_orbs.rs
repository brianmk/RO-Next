use ragnarok_packets::EntityId;
use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// Spirit-sphere count of one entity (`ZC_SPIRITS` 0x01D0 /
/// `ZC_SPIRITS2` 0x01E1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiritOrbsEntry {
    pub entity_id: EntityId,
    pub amount: i16,
}

/// Spirit-sphere counts of the entities on the current map.
///
/// Sphere rendering is owned by the skill-souls/cast-bar lane; this state is
/// the typed packet-layer hand-off.
#[derive(Default, RustState, StateElement)]
pub struct SpiritOrbs {
    #[hidden_element]
    entries: Vec<SpiritOrbsEntry>,
}

impl SpiritOrbs {
    pub fn set(&mut self, entity_id: EntityId, amount: i16) {
        match amount <= 0 {
            true => self.remove(entity_id),
            false => match self.entries.iter_mut().find(|entry| entry.entity_id == entity_id) {
                Some(entry) => entry.amount = amount,
                None => self.entries.push(SpiritOrbsEntry { entity_id, amount }),
            },
        }
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, entity_id: EntityId) {
        self.entries.retain(|entry| entry.entity_id != entity_id);
    }

    #[allow(dead_code)]
    pub fn get(&self, entity_id: EntityId) -> i16 {
        self.entries
            .iter()
            .find(|entry| entry.entity_id == entity_id)
            .map(|entry| entry.amount)
            .unwrap_or(0)
    }

    #[allow(dead_code)]
    pub fn entries(&self) -> &[SpiritOrbsEntry] {
        &self.entries
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::EntityId;

    use super::SpiritOrbs;

    #[test]
    fn set_updates_and_removes() {
        let mut orbs = SpiritOrbs::default();
        orbs.set(EntityId(1), 3);
        orbs.set(EntityId(2), 1);

        assert_eq!(orbs.get(EntityId(1)), 3);
        assert_eq!(orbs.get(EntityId(2)), 1);

        orbs.set(EntityId(1), 5);
        assert_eq!(orbs.get(EntityId(1)), 5);
        assert_eq!(orbs.entries().len(), 2);

        orbs.set(EntityId(1), 0);
        assert_eq!(orbs.get(EntityId(1)), 0);
        assert_eq!(orbs.entries().len(), 1);
    }
}
