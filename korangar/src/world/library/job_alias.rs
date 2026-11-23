//! Generated job-id skill-tree fallback table.
//!
//! The client job tables of this (trimmed/custom) data set do not cover every
//! rAthena server job: a few jobs have no (or an empty) client skill tree,
//! which used to make the client panic when a population shell morphed into
//! such a job. Every fallback below points to a job whose tree is non-empty.
//!
//! The player appearance data (job -> sprite/palette stem, hair styles, entity
//! type, baby/transcendent/fourth/expanded flags, scale) is **not** here
//! anymore: it lives in `archive/data/client/jobs.ron` and is loaded by
//! [`crate::world::jobs`]. The sprite alias table that used to live here was
//! folded into that data file as the documented `source: Fallback` rows.
//!
//! [`warn_once`] remains the shared log-once helper for ids that the client
//! data does not know (job appearance, job identity, skill tree, entity type).

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use ragnarok_packets::JobId;

/// Job id -> job whose skill tree should be used when the job has an empty or
/// missing skill tree (alternate/4th/expanded/unused jobs).
pub const JOB_SKILL_TREE_FALLBACKS: &[(u16, u16)] = &[
    (13, 7),
    (21, 14),
    (22, 0),
    (26, 0),
    (27, 0),
    (28, 0),
    (29, 0),
    (30, 0),
    (4014, 4008),
    (4022, 4015),
    (4036, 4030),
    (4044, 4037),
    (4048, 4047),
    (4050, 0),
    (4051, 0),
    (4052, 0),
    (4080, 4054),
    (4081, 4054),
    (4082, 4066),
    (4083, 4066),
    (4084, 4056),
    (4085, 4056),
    (4086, 4058),
    (4087, 4058),
    (4109, 4096),
    (4110, 4097),
    (4111, 4098),
    (4112, 4099),
    (4238, 4226),
    (4243, 4239),
    (4244, 4226),
    (4252, 4054),
    (4253, 4011),
    (4254, 4059),
    (4255, 4055),
    (4256, 4057),
    (4257, 4056),
    (4258, 4066),
    (4259, 4071),
    (4260, 4072),
    (4261, 4067),
    (4262, 4070),
    (4263, 4068),
    (4264, 4069),
    (4278, 4056),
    (4279, 4011),
    (4280, 4054),
    (4281, 4066),
    (4302, 4239),
    (4303, 4240),
    (4304, 25),
    (4305, 25),
    (4306, 24),
    (4307, 23),
    (4308, 4049),
    (4316, 4239),
];

/// Looks up a skill tree fallback for a job id.
pub fn skill_tree_fallback(job_id: JobId) -> Option<JobId> {
    JOB_SKILL_TREE_FALLBACKS
        .binary_search_by_key(&job_id.0, |(id, _)| *id)
        .ok()
        .map(|index| JobId(JOB_SKILL_TREE_FALLBACKS[index].1))
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
    use std::collections::HashSet;

    use ragnarok_packets::JobId;

    use super::*;

    #[test]
    fn skill_tree_fallbacks_are_sorted_and_unique() {
        assert!(JOB_SKILL_TREE_FALLBACKS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(!JOB_SKILL_TREE_FALLBACKS.is_empty());
    }

    #[test]
    fn skill_tree_fallbacks_have_no_cycles() {
        let keys: HashSet<u16> = JOB_SKILL_TREE_FALLBACKS.iter().map(|(job_id, _)| *job_id).collect();

        for (job_id, fallback) in JOB_SKILL_TREE_FALLBACKS {
            assert_ne!(job_id, fallback);
            assert!(!keys.contains(fallback), "chained skill tree fallback {job_id} -> {fallback}");
        }
    }

    #[test]
    fn lookups() {
        assert_eq!(skill_tree_fallback(JobId(4080)), Some(JobId(4054)));
        assert_eq!(skill_tree_fallback(JobId(65535)), None);
        warn_once("test", JobId(65535));
    }
}
