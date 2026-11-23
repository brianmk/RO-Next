//! Regression tests for the data-driven skill behaviour and procedural effect
//! data (Phase 4 of the hardcoded-systems remediation).
//!
//! These are repo-level guards, separate from the schema tests inside
//! `world/skills.rs`:
//!
//! * the hard-coded skill-id branches that used to live in `lib.rs` (Rolling
//!   Cutter channeling, Warp Portal provisional portal, Free Cast jobs, Storm
//!   Gust cloud) are gone from the source,
//! * the procedural effect modules read their textures/geometry from the data
//!   (`world/skills.rs`) instead of hard-coded effect paths,
//! * the curated skill-sound table is gone from Rust and lives in
//!   `tools/skill_sounds_custom.ron` -> `skill_sounds.ron`,
//! * the committed data files carry the behaviour rows, and the `--check-*` CLI
//!   entry points of the client accept them.

/// The client source that used to hold the skill-id branches.
const LIB_RS: &str = include_str!("../src/lib.rs");
/// The procedural effect modules that used to hold the asset/geometry
/// constants.
const STORM_CLOUD_RS: &str = include_str!("../src/world/effect/storm_cloud.rs");
const CAST_CIRCLE_RS: &str = include_str!("../src/world/effect/cast_circle.rs");
const WARP_RS: &str = include_str!("../src/world/effect/warp.rs");
const AURA_RS: &str = include_str!("../src/world/effect/aura.rs");
/// The skill sound player that used to hold the curated Rust table.
const SKILL_SOUND_RS: &str = include_str!("../src/world/skill_sound.rs");
/// The committed runtime data files.
const SKILLS_RON: &str = include_str!("../archive/data/client/skills.ron");
const JOBS_RON: &str = include_str!("../archive/data/client/jobs.ron");
const SKILL_SOUNDS_RON: &str = include_str!("../archive/data/client/skill_sounds.ron");

/// Skill-id branches/constants that were removed from `lib.rs`. None of them
/// may come back: the behaviour belongs in `skills.ron` and is looked up by
/// the data API.
const REMOVED_SKILL_BRANCHES: &[&str] = &[
    "ROLLING_CUTTER_ID",
    "FREE_CAST_SKILL_ID",
    "FREE_CAST_JOB_IDS",
    "STORM_CLOUD_PHASE",
    "skill_id.0 == 27",
    "matches!(skill_id.0, 87 | 89)",
];

#[test]
fn hardcoded_skill_id_branches_are_gone() {
    for branch in REMOVED_SKILL_BRANCHES {
        assert!(
            !LIB_RS.contains(branch),
            "korangar/src/lib.rs still contains the hardcoded skill branch {branch:?}; it belongs in skills.ron"
        );
    }

    // The behaviour goes through the data lookups instead.
    assert!(
        LIB_RS.contains("crate::world::skills::channels("),
        "self casts have to use the data-driven channeling lookup"
    );
    assert!(
        LIB_RS.contains("crate::world::skills::has_warp_target("),
        "the provisional warp portal has to use the data-driven lookup"
    );
    assert!(
        LIB_RS.contains("crate::world::skills::cloud("),
        "the procedural cloud has to use the data-driven lookup"
    );
    assert!(
        LIB_RS.contains("crate::world::jobs::can_move_while_casting("),
        "Free Cast has to use the job capability + skill flag data"
    );
}

#[test]
fn procedural_effects_read_their_assets_from_data() {
    for (name, source) in [
        ("storm_cloud.rs", STORM_CLOUD_RS),
        ("cast_circle.rs", CAST_CIRCLE_RS),
        ("warp.rs", WARP_RS),
        ("aura.rs", AURA_RS),
    ] {
        assert!(
            source.contains("crate::world::skills::"),
            "{name} has to read its procedural spec from the skill data"
        );
    }

    // No effect asset path/geometry constant may stay in the effect modules.
    assert!(!STORM_CLOUD_RS.contains("CLOUD_TEXTURES"), "storm cloud textures must be data");
    assert!(
        !STORM_CLOUD_RS.contains(r#""effect\\cloud"#),
        "storm_cloud.rs must not hardcode the cloud texture paths"
    );
    assert!(
        !CAST_CIRCLE_RS.contains(r#""effect\\ring_liner.tga""#) && !CAST_CIRCLE_RS.contains(r#""effect\\alpha_down.tga""#),
        "cast_circle.rs must not hardcode its texture paths"
    );
    assert!(
        !WARP_RS.contains(r#""effect\\ring_blue.tga""#) && !WARP_RS.contains(r#""이팩트\\particle1.spr""#),
        "warp.rs must not hardcode its texture paths"
    );
    assert!(
        !AURA_RS.contains(r#""effect\\glow01.bmp""#) && !AURA_RS.contains(r#""effect\\whitelight.tga""#),
        "aura.rs must not hardcode its texture paths"
    );
}

#[test]
fn curated_skill_sounds_are_gone_from_rust() {
    assert!(
        !SKILL_SOUND_RS.contains("SKILL_SOUNDS"),
        "the curated Rust sound table has to live in tools/skill_sounds_custom.ron"
    );
    assert!(
        !SKILL_SOUND_RS.contains("GENERIC_CAST_SOUND"),
        "the generic cast sound has to live in the sound data file"
    );
    assert!(
        SKILL_SOUND_RS.contains("crate::world::curated_skill_sound_files("),
        "the player has to use the curated data lookup"
    );
}

#[test]
fn skill_data_file_carries_the_behaviour_rows() {
    for row in [
        "(skill: 27, warp_target: true)",
        "(skill: 278, move_while_casting: true)",
        "(skill: 2036, channeling: true)",
    ] {
        assert!(SKILLS_RON.contains(row), "skills.ron is missing the row {row}");
    }

    // Storm Gust's procedural cloud (skills 87/89), with the official textures
    // and the previous constants as data.
    assert!(SKILLS_RON.contains("skills: [87, 89]"));
    assert!(SKILLS_RON.contains("phase: Success"));
    assert!(SKILLS_RON.contains("count: 18"));
    assert!(SKILLS_RON.contains("radius: 22.0"));
    assert!(SKILLS_RON.contains("base_height: 10.0"));
    assert!(SKILLS_RON.contains("duration_ms: 3000"));
    assert!(SKILLS_RON.contains(r#""effect\\cloud1.tga""#));
    assert!(SKILLS_RON.contains(r#""effect\\cloud7.tga""#));

    // The procedural effect sections (cast circle / warp portal / level aura).
    assert!(SKILLS_RON.contains("cast_circle: ("));
    assert!(SKILLS_RON.contains(r#"ring_texture: "effect\\ring_liner.tga""#));
    assert!(SKILLS_RON.contains("warp_portal: ("));
    assert!(SKILLS_RON.contains(r#"particle_sprite: "이팩트\\particle1.spr""#));
    assert!(SKILLS_RON.contains("level_aura: ("));
    assert!(SKILLS_RON.contains(r#"sparkle_texture: "effect\\pikapika2.bmp""#));
}

#[test]
fn jobs_data_carries_the_move_while_casting_capability() {
    assert!(JOBS_RON.contains("capabilities: ("));
    assert!(JOBS_RON.contains("move_while_casting: [16, 4017],"));
}

#[test]
fn curated_sounds_are_in_the_sound_data_file() {
    assert!(SKILL_SOUNDS_RON.contains("curated: ["));
    assert!(
        SKILL_SOUNDS_RON.contains(r#"(skill: 110, files: ["effect\\black_hammerfall.wav"])"#),
        "Hammer Fall's curated sound has to be in the data file"
    );
    assert!(SKILL_SOUNDS_RON.contains(r#"(skill: 27, files: ["effect\\ef_readyportal.wav", "effect\\ef_portal.wav"])"#));
    assert!(SKILL_SOUNDS_RON.contains("generic: ["));
    assert!(SKILL_SOUNDS_RON.contains(r#"effect\\ef_beginspell.wav"#));
}

#[test]
fn data_cli_checks_accept_the_committed_files() {
    let skills = korangar::check_skills(None).expect("the committed skill behaviour data has to be valid");
    assert_eq!(skills.behaviours, 3);
    assert_eq!(skills.clouds, 2);

    let effects = korangar::check_skill_effects(None).expect("the committed skill effect data has to be valid");
    assert!(effects > 250, "unexpected skill effect descriptor count {effects}");

    let jobs = korangar::check_jobs(None).expect("the committed job data has to be valid");
    assert_eq!(jobs, 181);
}
