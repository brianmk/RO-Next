//! De-duplication of skill completion effects.
//!
//! A skill can produce several packets that all qualify for playing its
//! completion effect: ground casts send `ZC_NOTIFY_GROUNDSKILL` (0x0117),
//! no-damage casts send `ZC_USE_SKILL` (0x09CB) and single-target damage skills
//! send `ZC_NOTIFY_SKILL` (0x01DE) — sometimes several damage packets per cast
//! for multi-hit skills. The tracker makes sure the effect of one cast is
//! spawned exactly once, no matter which packet arrives first.

use ragnarok_packets::{ClientTick, EntityId, SkillId};

/// Time window in which packets of the same cast are treated as one spawn.
///
/// The cast-start/ground-skill packets and the first damage packets of a cast
/// arrive within a few milliseconds of each other; 1.5 s also covers the normal
/// damage delay of a multi-hit skill without merging two separate casts (the
/// server after-cast delays are longer than this).
const DEDUPLICATION_WINDOW_MS: u32 = 1_500;

/// Tracks which (entity, skill) completion effects were spawned recently.
#[derive(Default)]
pub struct SkillCompletionTracker {
    recent: Vec<((EntityId, SkillId), ClientTick)>,
}

impl SkillCompletionTracker {
    /// Returns `true` when the completion effect of `skill_id` should be
    /// spawned for `entity_id` now, `false` when it was already spawned for the
    /// same cast.
    pub fn claim(&mut self, entity_id: EntityId, skill_id: SkillId, client_tick: ClientTick) -> bool {
        self.recent
            .retain(|(_, spawned_at)| client_tick.0.wrapping_sub(spawned_at.0) < DEDUPLICATION_WINDOW_MS);

        let key = (entity_id, skill_id);

        if self.recent.iter().any(|(recent_key, _)| *recent_key == key) {
            return false;
        }

        self.recent.push((key, client_tick));
        true
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, EntityId, SkillId};

    use super::SkillCompletionTracker;

    #[test]
    fn one_effect_per_cast() {
        let mut tracker = SkillCompletionTracker::default();

        assert!(tracker.claim(EntityId(1), SkillId(56), ClientTick(100)));
        // Second packet of the same cast (e.g. a multi-hit damage packet).
        assert!(!tracker.claim(EntityId(1), SkillId(56), ClientTick(350)));
        assert!(!tracker.claim(EntityId(1), SkillId(56), ClientTick(1_400)));
    }

    #[test]
    fn different_skill_or_entity_can_spawn_immediately() {
        let mut tracker = SkillCompletionTracker::default();

        assert!(tracker.claim(EntityId(1), SkillId(56), ClientTick(100)));
        assert!(tracker.claim(EntityId(1), SkillId(89), ClientTick(100)));
        assert!(tracker.claim(EntityId(2), SkillId(56), ClientTick(100)));
    }

    #[test]
    fn a_later_cast_spawns_again() {
        let mut tracker = SkillCompletionTracker::default();

        assert!(tracker.claim(EntityId(1), SkillId(56), ClientTick(100)));
        assert!(tracker.claim(EntityId(1), SkillId(56), ClientTick(1_700)));
    }
}
