//! Per-entity spawn/despawn/respawn/job-change lifecycle.
//!
//! The map server re-sends `AddEntity` for entities that are already visible
//! (most notably after a job change and on respawn) and sends `RemoveEntity`
//! for deaths, teleports and out-of-sight removals. The client used to deal
//! with this through ad-hoc `Vec::retain` calls and a `truncate(1)` on warp,
//! which made it easy to leave stale state behind.
//!
//! [`EntityLifecycle`] keeps one [`EntityState`] per entity id and answers the
//! questions the client actually has:
//!
//! * Is this `AddEntity` a first spawn or a replacement of an entity that is
//!   already visible (job change, respawn)?
//! * How does a `RemoveEntity` change the state (dead vs. fading out)?
//! * Which entities have to be forgotten on a warp, and which status effects
//!   belong to the old map session?

use std::collections::HashMap;

use ragnarok_packets::{DisappearanceReason, EntityId, JobId};

/// Explicit state of one tracked entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    /// The server announced the entity, the client has not created it yet.
    Spawning,
    /// The entity is visible and alive.
    Alive,
    /// The server reported the death of the entity.
    Dead,
    /// The entity was removed (teleport/logout/out of sight) and is fading out.
    Despawning,
}

/// Result of an `AddEntity` packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddEntityDecision {
    /// The entity was not known yet; create it normally.
    Create,
    /// The entity is already visible (job change/respawn/duplicate packet);
    /// replace the existing entity and inherit its fade state.
    Replace { previous: EntityState },
}

/// Result of [`EntityLifecycle::on_spawned`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOutcome {
    /// State the entity had before this spawn, if it was already known.
    pub previous_state: Option<EntityState>,
    /// Whether the job of the new spawn differs from the last known job.
    pub job_changed: bool,
}

/// Result of a `RemoveEntity` packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoveEntityOutcome {
    /// The entity was tracked; it is now dead or fading out.
    Known { state: EntityState, died: bool },
    /// The client did not know the entity (e.g. it was removed before its
    /// spawn was handled).
    Unknown,
}

/// Explicit entity lifecycle bookkeeping.
#[derive(Debug, Clone, Default)]
pub struct EntityLifecycle {
    entries: HashMap<EntityId, Entry>,
    warp_count: u64,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    state: EntityState,
    job_id: Option<JobId>,
}

impl EntityLifecycle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle an `AddEntity` announcement. Must be called before the client
    /// creates the entity; [`EntityLifecycle::on_spawned`] must follow when the
    /// entity was created successfully.
    pub fn on_add_entity(&mut self, entity_id: EntityId) -> AddEntityDecision {
        match self.entries.get(&entity_id) {
            Some(entry) => {
                crate::logging::log_debug!(
                    "entity lifecycle: duplicate AddEntity for {} (previous state {:?})",
                    entity_id.0,
                    entry.state
                );
                AddEntityDecision::Replace { previous: entry.state }
            }
            None => {
                self.entries.insert(entity_id, Entry {
                    state: EntityState::Spawning,
                    job_id: None,
                });
                AddEntityDecision::Create
            }
        }
    }

    /// The client created the entity for a previously announced `AddEntity`.
    pub fn on_spawned(&mut self, entity_id: EntityId, job_id: JobId) -> SpawnOutcome {
        let entry = self.entries.entry(entity_id).or_insert(Entry {
            state: EntityState::Spawning,
            job_id: None,
        });
        let previous_state = match entry.state {
            EntityState::Spawning => None,
            previous => Some(previous),
        };
        let job_changed = entry.job_id.is_some_and(|previous_job| previous_job != job_id);

        if job_changed {
            crate::logging::log_debug!(
                "entity lifecycle: entity {} changed job {:?} -> {:?} on spawn",
                entity_id.0,
                entry.job_id,
                job_id
            );
        }

        entry.state = EntityState::Alive;
        entry.job_id = Some(job_id);

        SpawnOutcome {
            previous_state,
            job_changed,
        }
    }

    /// Handle a `RemoveEntity` packet. The entity itself is faded out by the
    /// caller; this only updates the lifecycle bookkeeping.
    pub fn on_remove_entity(&mut self, entity_id: EntityId, reason: &DisappearanceReason) -> RemoveEntityOutcome {
        let Some(entry) = self.entries.get_mut(&entity_id) else {
            return RemoveEntityOutcome::Unknown;
        };

        let died = *reason == DisappearanceReason::Died;
        entry.state = if died { EntityState::Dead } else { EntityState::Despawning };

        RemoveEntityOutcome::Known { state: entry.state, died }
    }

    /// The server resurrected the entity (0x0A15/`ResurrectPlayer`).
    pub fn on_resurrect(&mut self, entity_id: EntityId) -> bool {
        let Some(entry) = self.entries.get_mut(&entity_id) else {
            return false;
        };

        if entry.state != EntityState::Dead {
            return false;
        }

        crate::logging::log_debug!("entity lifecycle: entity {} resurrected", entity_id.0);
        entry.state = EntityState::Alive;
        true
    }

    /// A job change (`ChangeJob`) was applied to an entity that is already
    /// visible. The follow-up duplicate `AddEntity` is expected to replace the
    /// entity with the new sprite.
    pub fn on_job_change(&mut self, entity_id: EntityId, job_id: JobId) -> bool {
        let Some(entry) = self.entries.get_mut(&entity_id) else {
            return false;
        };

        entry.job_id = Some(job_id);
        true
    }

    /// The entity finished fading out and was removed from the entity list.
    pub fn on_entity_disappeared(&mut self, entity_id: EntityId) {
        self.entries.remove(&entity_id);
    }

    /// A warp/map change happened. Returns the ids that have to be forgotten
    /// (the caller keeps only the local player and re-spawns everything else
    /// from the new map). Status effects belong to the old map session as well
    /// and have to be cleared by the caller.
    pub fn on_warp(&mut self) -> Vec<EntityId> {
        self.warp_count += 1;
        let entity_ids = self.entries.keys().copied().collect();
        self.entries.clear();
        crate::logging::log_debug!("entity lifecycle: warp {} cleared all tracked entities", self.warp_count);
        entity_ids
    }

    /// Whether the entity is tracked (test/debug helper).
    #[cfg(test)]
    pub fn state_of(&self, entity_id: EntityId) -> Option<EntityState> {
        self.entries.get(&entity_id).map(|entry| entry.state)
    }

    /// Number of tracked entities (test helper).
    #[cfg(test)]
    pub fn tracked_count(&self) -> usize {
        self.entries.len()
    }

    /// Number of handled warps (test helper).
    #[cfg(test)]
    pub fn warp_count(&self) -> u64 {
        self.warp_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entity(id: u32) -> EntityId {
        EntityId(id)
    }

    fn job(id: u16) -> JobId {
        JobId(id)
    }

    #[test]
    fn first_spawn_creates_the_entity() {
        let mut lifecycle = EntityLifecycle::new();
        assert_eq!(lifecycle.on_add_entity(entity(1)), AddEntityDecision::Create);
        let outcome = lifecycle.on_spawned(entity(1), job(0));
        assert_eq!(outcome.previous_state, None);
        assert!(outcome.previous_state.is_none());
        assert!(!outcome.job_changed);
        assert_eq!(lifecycle.state_of(entity(1)), Some(EntityState::Alive));
        assert_eq!(lifecycle.tracked_count(), 1);
    }

    #[test]
    fn duplicate_add_entity_replaces_instead_of_duplicating() {
        let mut lifecycle = EntityLifecycle::new();
        lifecycle.on_add_entity(entity(1));
        lifecycle.on_spawned(entity(1), job(0));

        assert_eq!(lifecycle.on_add_entity(entity(1)), AddEntityDecision::Replace {
            previous: EntityState::Alive
        });
        let outcome = lifecycle.on_spawned(entity(1), job(0));
        assert_eq!(outcome.previous_state, Some(EntityState::Alive));
        assert!(!outcome.job_changed);
        assert_eq!(lifecycle.tracked_count(), 1, "the entity must stay tracked exactly once");
    }

    #[test]
    fn duplicate_add_entity_after_a_job_change_reports_the_new_job() {
        let mut lifecycle = EntityLifecycle::new();
        lifecycle.on_add_entity(entity(1));
        lifecycle.on_spawned(entity(1), job(0));
        assert!(lifecycle.on_job_change(entity(1), job(7)));

        assert_eq!(lifecycle.on_add_entity(entity(1)), AddEntityDecision::Replace {
            previous: EntityState::Alive
        });
        let outcome = lifecycle.on_spawned(entity(1), job(7));
        assert!(outcome.previous_state.is_some());
        assert!(!outcome.job_changed, "the job change packet already recorded the new job");
        assert_eq!(lifecycle.state_of(entity(1)), Some(EntityState::Alive));
        assert_eq!(lifecycle.tracked_count(), 1);
    }

    #[test]
    fn spawn_that_reports_a_new_job_marks_the_job_change() {
        let mut lifecycle = EntityLifecycle::new();
        lifecycle.on_add_entity(entity(1));
        lifecycle.on_spawned(entity(1), job(0));
        lifecycle.on_add_entity(entity(1));
        let outcome = lifecycle.on_spawned(entity(1), job(7));
        assert!(outcome.job_changed);
    }

    #[test]
    fn death_and_resurrection_are_tracked() {
        let mut lifecycle = EntityLifecycle::new();
        lifecycle.on_add_entity(entity(1));
        lifecycle.on_spawned(entity(1), job(0));

        assert_eq!(
            lifecycle.on_remove_entity(entity(1), &DisappearanceReason::Died),
            RemoveEntityOutcome::Known {
                state: EntityState::Dead,
                died: true
            }
        );
        assert_eq!(lifecycle.state_of(entity(1)), Some(EntityState::Dead));

        assert!(lifecycle.on_resurrect(entity(1)));
        assert_eq!(lifecycle.state_of(entity(1)), Some(EntityState::Alive));
        assert!(!lifecycle.on_resurrect(entity(1)), "only dead entities can be resurrected");
    }

    #[test]
    fn non_death_removal_fades_out_and_can_reappear() {
        let mut lifecycle = EntityLifecycle::new();
        lifecycle.on_add_entity(entity(1));
        lifecycle.on_spawned(entity(1), job(0));

        assert_eq!(
            lifecycle.on_remove_entity(entity(1), &DisappearanceReason::Teleported),
            RemoveEntityOutcome::Known {
                state: EntityState::Despawning,
                died: false
            }
        );

        // The server re-adds the entity before the fade finished (e.g. it
        // warped back into view): replace it.
        assert_eq!(lifecycle.on_add_entity(entity(1)), AddEntityDecision::Replace {
            previous: EntityState::Despawning
        });
        lifecycle.on_spawned(entity(1), job(0));
        assert_eq!(lifecycle.state_of(entity(1)), Some(EntityState::Alive));
    }

    #[test]
    fn removal_of_an_unknown_entity_is_reported() {
        let mut lifecycle = EntityLifecycle::new();
        assert_eq!(
            lifecycle.on_remove_entity(entity(9), &DisappearanceReason::OutOfSight),
            RemoveEntityOutcome::Unknown
        );
    }

    #[test]
    fn fade_out_completion_forgets_the_entity() {
        let mut lifecycle = EntityLifecycle::new();
        lifecycle.on_add_entity(entity(1));
        lifecycle.on_spawned(entity(1), job(0));
        lifecycle.on_remove_entity(entity(1), &DisappearanceReason::OutOfSight);
        lifecycle.on_entity_disappeared(entity(1));
        assert_eq!(lifecycle.state_of(entity(1)), None);
        assert_eq!(lifecycle.tracked_count(), 0);
    }

    #[test]
    fn warp_clears_every_tracked_entity() {
        let mut lifecycle = EntityLifecycle::new();
        for id in 1..=3 {
            lifecycle.on_add_entity(entity(id));
            lifecycle.on_spawned(entity(id), job(0));
        }

        let cleared = lifecycle.on_warp();
        assert_eq!(cleared.len(), 3);
        assert_eq!(lifecycle.tracked_count(), 0);
        assert_eq!(lifecycle.warp_count(), 1);
        assert_eq!(lifecycle.state_of(entity(1)), None);
    }
}
