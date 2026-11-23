//! Job compatibility helpers that are not part of the data registries.
//!
//! The data-driven tables that used to live here are now in
//! `archive/data/client/*.ron`:
//!
//! * the player job appearance data (sprite/palette stems, hair styles, job
//!   flags, and the 63-row player sprite alias table that used to be
//!   `PLAYER_JOB_SPRITE_ALIASES`) lives in `jobs.ron` and is loaded by
//!   [`crate::world::jobs`],
//! * the job id -> skill tree fallback table (`JOB_SKILL_TREE_FALLBACKS`) lives
//!   in `aliases.ron` and is loaded by [`super::aliases`];
//!   [`skill_tree_fallback`] is the single lookup the skill tree resolver uses.
//!
//! What remains here is [`warn_once`], the shared log-once helper for job ids
//! that the client data does not know (job appearance, job identity, skill
//! tree, entity type). Logging once keeps a population shell full of custom
//! server job ids from spamming the log, while still making the substitution
//! visible instead of silently rendering a wrong sprite.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use ragnarok_packets::JobId;

/// Looks up the client skill tree fallback of a job id.
///
/// The table is content data extracted from the client skill-tree tables; see
/// [`super::aliases`] and the module docs of [`crate::world::library`].
pub fn skill_tree_fallback(job_id: JobId) -> Option<JobId> {
    super::aliases::job_skill_tree_fallback(job_id)
}

/// Logs a warning once per job id. Used instead of silently substituting a
/// wrong sprite/identity for job ids that the client tables do not know.
pub fn warn_once(category: &str, job_id: JobId) {
    static WARNED: LazyLock<Mutex<HashSet<(String, u16)>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

    let mut warned = WARNED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

    if warned.insert((category.to_string(), job_id.0)) {
        eprintln!("[job] {category} for unknown job id {}", job_id.0);
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::JobId;

    use super::*;

    #[test]
    fn skill_tree_fallback_uses_the_alias_data() {
        assert_eq!(skill_tree_fallback(JobId(4080)), Some(JobId(4054)));
        assert_eq!(skill_tree_fallback(JobId(4110)), Some(JobId(4097)));
        assert_eq!(skill_tree_fallback(JobId(0)), None);
        assert_eq!(skill_tree_fallback(JobId(65535)), None);
    }

    #[test]
    fn warn_once_is_callable_for_unknown_ids() {
        warn_once("test", JobId(65535));
    }
}
