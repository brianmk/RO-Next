//! Generated job-id fallback tables.
//!
//! The client job tables of this (trimmed/custom) data set do not cover every
//! rAthena server job: some player job ids are mapped by the client to sprites
//! that are not in the GRFs (4th jobs, KAGEROU/OBORO/REBELLION, …) and some
//! have no client mapping at all (alternate/costume/unsupported jobs). A few
//! jobs also have no (or an empty) skill tree, which used to make the client
//! render a black `missing.spr` / panic when a population shell morphed into
//! such a job.
//!
//! Every alias below is verified by `generate_job_alias.py` against the actual
//! GRF file set (body sprite for at least one sex, palette where available);
//! every skill tree fallback points to a job whose tree is non-empty.

use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

use ragnarok_packets::JobId;

/// Player job id -> existing body sprite name (without the `_남`/`_여` suffix).
pub const PLAYER_JOB_SPRITE_ALIASES: &[(u16, &str)] = &[
    (21, "신페코크루세이더"),
    (22, "결혼"),
    (26, "산타"),
    (27, "여름"),
    (28, "한복"),
    (29, "옥토버패스트"),
    (30, "여름2"),
    (4014, "로드나이트"),
    (4022, "페코팔라딘"),
    (4036, "기사"),
    (4044, "크루세이더"),
    (4048, "권성"),
    (4050, "초보자"),
    (4051, "초보자"),
    (4052, "초보자"),
    (4080, "룬나이트"),
    (4081, "룬나이트"),
    (4082, "가드"),
    (4083, "가드"),
    (4084, "레인져"),
    (4085, "레인져"),
    (4086, "미케닉"),
    (4087, "미케닉"),
    (4109, "룬나이트"),
    (4110, "가드"),
    (4111, "레인져"),
    (4112, "미케닉"),
    (4211, "닌자"),
    (4212, "닌자"),
    (4215, "건너"),
    (4218, "초보자"),
    (4220, "초보자"),
    (4223, "닌자"),
    (4224, "닌자"),
    (4229, "건너"),
    (4238, "권성"),
    (4243, "성제"),
    (4244, "성제"),
    (4252, "룬나이트"),
    (4253, "화이트스미스"),
    (4254, "길로틴크로스"),
    (4255, "워록"),
    (4256, "아크비숍"),
    (4257, "레인져"),
    (4258, "가드"),
    (4259, "제네릭"),
    (4260, "쉐도우체이서"),
    (4261, "소서러"),
    (4262, "슈라"),
    (4263, "민스트럴"),
    (4264, "원더러"),
    (4278, "레인져"),
    (4279, "화이트스미스"),
    (4280, "룬나이트"),
    (4281, "가드"),
    (4302, "성제"),
    (4303, "소울리퍼"),
    (4304, "닌자"),
    (4305, "닌자"),
    (4306, "건너"),
    (4307, "슈퍼노비스"),
    (4308, "소울링커"),
    (4316, "성제"),
];

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

/// Looks up a verified body sprite alias for a player job id.
pub fn player_job_sprite_alias(job_id: JobId) -> Option<&'static str> {
    PLAYER_JOB_SPRITE_ALIASES
        .binary_search_by_key(&job_id.0, |(id, _)| *id)
        .ok()
        .map(|index| PLAYER_JOB_SPRITE_ALIASES[index].1)
}

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
    fn alias_tables_are_sorted_and_unique() {
        assert!(PLAYER_JOB_SPRITE_ALIASES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(JOB_SKILL_TREE_FALLBACKS.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(!PLAYER_JOB_SPRITE_ALIASES.is_empty());
        assert!(!JOB_SKILL_TREE_FALLBACKS.is_empty());
    }

    #[test]
    fn aliases_are_sane() {
        for (job_id, sprite) in PLAYER_JOB_SPRITE_ALIASES {
            assert!(!sprite.is_empty(), "empty sprite alias for {job_id}");
            assert!(!sprite.contains("missing"), "missing sprite alias for {job_id}: {sprite}");
        }
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
        assert_eq!(player_job_sprite_alias(JobId(4252)), Some("룬나이트"));
        assert_eq!(player_job_sprite_alias(JobId(65535)), None);
        assert_eq!(skill_tree_fallback(JobId(4080)), Some(JobId(4054)));
        assert_eq!(skill_tree_fallback(JobId(65535)), None);
        warn_once("test", JobId(65535));
    }
}
