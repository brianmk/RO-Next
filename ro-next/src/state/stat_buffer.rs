//! Buffering of stat updates that arrive before the local player entity exists.

use ragnarok_packets::StatType;

/// Maximum number of buffered stat updates.
///
/// The buffer normally holds at most the handful of values the server sends in
/// its initial stat burst, but the cap keeps it bounded if a malformed server
/// sends a flood of updates while no player entity exists.
const MAX_BUFFERED_STAT_UPDATES: usize = 128;

/// Stat updates that were received before [`crate::state::this_player`] was
/// available.
///
/// The server sends its initial stat burst (including `SP_BASEEXP`,
/// `SP_NEXTBASEEXP`, `SP_JOBEXP` and `SP_NEXTJOBEXP`) right after the map load,
/// which is handled from a network event. Depending on the order of the network
/// events and of the character creation, those updates can arrive before the
/// local player entity has been created. They would otherwise be dropped and
/// the character overview would never learn the experience needed for the next
/// level.
///
/// Updates are buffered per stat kind: receiving the full stat burst again (for
/// example after a map change or respawn) overwrites the queued value instead
/// of growing the buffer.
#[derive(Default)]
pub struct PendingStatUpdates {
    updates: Vec<StatType>,
}

impl PendingStatUpdates {
    pub fn new() -> Self {
        Self::default()
    }

    /// Buffers a stat update. A queued update of the same stat kind is
    /// overwritten, so repeated full-stat bursts do not accumulate.
    pub fn push(&mut self, stat_type: StatType) {
        if let Some(existing) = self
            .updates
            .iter_mut()
            .find(|existing| std::mem::discriminant(*existing) == std::mem::discriminant(&stat_type))
        {
            *existing = stat_type;
            return;
        }

        if self.updates.len() >= MAX_BUFFERED_STAT_UPDATES {
            self.updates.remove(0);
        }

        self.updates.push(stat_type);
    }

    /// Returns and clears all buffered stat updates, in insertion order.
    pub fn take(&mut self) -> std::vec::IntoIter<StatType> {
        std::mem::take(&mut self.updates).into_iter()
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.updates.len()
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::StatType;

    use super::PendingStatUpdates;

    #[test]
    fn buffered_updates_are_returned_in_insertion_order() {
        let mut pending = PendingStatUpdates::new();

        pending.push(StatType::BaseExperience(1));
        pending.push(StatType::NextBaseExperience(10));
        pending.push(StatType::HealthPoints(40));

        assert_eq!(pending.len(), 3);

        let updates = pending.take().collect::<Vec<_>>();

        assert_eq!(updates, vec![
            StatType::BaseExperience(1),
            StatType::NextBaseExperience(10),
            StatType::HealthPoints(40),
        ]);
        assert!(pending.is_empty());
    }

    #[test]
    fn repeated_updates_of_the_same_kind_overwrite_the_queued_value() {
        let mut pending = PendingStatUpdates::new();

        // A repeated full stat burst must not grow the buffer.
        for value in 1..=10 {
            pending.push(StatType::BaseExperience(value));
            pending.push(StatType::NextBaseExperience(value * 10));
        }

        assert_eq!(pending.len(), 2);

        let updates = pending.take().collect::<Vec<_>>();

        // The order of the stat kinds is the order in which they were first
        // buffered, but the values are the latest ones.
        assert_eq!(updates, vec![StatType::BaseExperience(10), StatType::NextBaseExperience(100)]);
    }

    #[test]
    fn taking_an_empty_buffer_returns_nothing() {
        let mut pending = PendingStatUpdates::new();

        assert!(pending.take().next().is_none());
        assert!(pending.is_empty());
    }
}
