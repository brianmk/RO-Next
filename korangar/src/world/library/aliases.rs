//! Data-driven alias tables for the classic client fallbacks.
//!
//! Three content tables used to be generated Rust arrays with untracked
//! generators, which made them drift (a status icon had to be hand-added, see
//! the hardcoded-systems audit §4.10):
//!
//! * job id -> job whose client skill tree to use when the job's own tree is
//!   empty (previously `JOB_SKILL_TREE_FALLBACKS` in `job_alias.rs`),
//! * monster job id -> verified existing monster/npc sprite (previously
//!   `MONSTER_SPRITE_ALIASES` / `GENERIC_MONSTER_SPRITES` in
//!   `monster_sprite_alias.rs`),
//! * skill id -> verified existing skill icon (previously `SKILL_ICON_ALIASES`
//!   / `GENERIC_SKILL_ICONS` in `skill_icon_alias.rs`).
//!
//! All three are now one generated data file,
//! `archive/data/client/aliases.ron`, produced by `tools/gen_aliases.py` from
//! the committed facts `tools/data/alias_facts.json` (extracted from the rAthena
//! server tree and the CondaRO client archives by
//! `tools/extract_alias_facts.py`; the generators and the facts are committed,
//! `gen_aliases.py --check` is the CI drift guard).
//!
//! The file is loaded from the client archive (`data\client\aliases.ron`, so a
//! server can override the table) with the committed copy compiled in via
//! `include_str!` as the fallback. A malformed override is rejected with a
//! warning and the compiled table is used; the client never ends up without the
//! documented fallbacks (generic monster sprite / generic skill icon / empty
//! skill tree + log-once warning in [`super::job_alias::warn_once`]).
//!
//! ## Resolution order (unchanged from the old Rust tables)
//!
//! * monster sprite: exact `몬스터\<identity>` -> this table ->
//!   `npc\<identity>` -> [`generic_monster_sprites`] (in preference order),
//! * skill icon: exact `아이템\<file_name>` -> this table ->
//!   [`generic_skill_icons`] (in preference order),
//! * skill tree: job's own tree -> this table -> empty tree + log-once.

use std::collections::HashSet;
use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::{JobId, SkillId};
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\aliases.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/aliases.ron");

/// The RON schema of `data\client\aliases.ron`.
///
/// `deny_unknown_fields` makes an override with a typo fail loudly instead of
/// silently ignoring the table it tried to replace.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AliasFile {
    provenance: String,
    version: u32,
    job_skill_tree_fallbacks: Vec<(u16, u16)>,
    monster_sprite_aliases: Vec<(u16, String)>,
    generic_monster_sprites: Vec<String>,
    skill_icon_aliases: Vec<(u16, String)>,
    generic_skill_icons: Vec<String>,
}

/// The parsed, validated and leaked table (lookups return `'static` paths).
struct AliasTable {
    job_skill_tree_fallbacks: Vec<(u16, u16)>,
    monster_sprite_aliases: Vec<(u16, &'static str)>,
    generic_monster_sprites: Vec<&'static str>,
    skill_icon_aliases: Vec<(u16, &'static str)>,
    generic_skill_icons: Vec<&'static str>,
}

/// Row counts of the alias data file (returned by the `--check-aliases` CLI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AliasesSummary {
    pub job_skill_tree_fallbacks: usize,
    pub monster_sprite_aliases: usize,
    pub skill_icon_aliases: usize,
}

impl AliasesSummary {
    fn of(table: &AliasTable) -> Self {
        Self {
            job_skill_tree_fallbacks: table.job_skill_tree_fallbacks.len(),
            monster_sprite_aliases: table.monster_sprite_aliases.len(),
            skill_icon_aliases: table.skill_icon_aliases.len(),
        }
    }
}

/// The compiled fallback, parsed on first use (also used by tests and the
/// `--check-aliases` CLI).
static COMPILED: OnceLock<AliasTable> = OnceLock::new();
/// The table loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<AliasTable> = OnceLock::new();

fn compiled() -> &'static AliasTable {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(table) => table,
        Err(error) => {
            crate::logging::log_warn!("aliases: compiled fallback is invalid ({error})");
            AliasTable {
                job_skill_tree_fallbacks: Vec::new(),
                monster_sprite_aliases: Vec::new(),
                generic_monster_sprites: Vec::new(),
                skill_icon_aliases: Vec::new(),
                generic_skill_icons: Vec::new(),
            }
        }
    })
}

fn table() -> &'static AliasTable {
    ARCHIVE.get().unwrap_or_else(compiled)
}

/// Loads the alias table from the client archive (or the compiled fallback).
///
/// A malformed data file is reported and the compiled fallback is used instead,
/// so a bad override can never leave the client without the documented
/// fallbacks. Returns the loaded row counts.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> AliasesSummary {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        let summary = AliasesSummary::of(compiled());
        crate::logging::log_debug!("aliases: loaded the compiled fallback table");
        return summary;
    };

    let loaded = match parse(content) {
        Ok(table) => {
            crate::logging::log_debug!("aliases: loaded the table from {DATA_PATH}");
            table
        }
        Err(error) => {
            crate::logging::log_warn!("aliases: invalid {DATA_PATH} ({error}), using the compiled fallback");
            return AliasesSummary::of(compiled());
        }
    };

    let summary = AliasesSummary::of(&loaded);
    let _ = ARCHIVE.set(loaded);
    summary
}

/// Job id -> job whose (non-empty) client skill tree should be used when the
/// job's own tree is empty or missing.
pub(crate) fn job_skill_tree_fallback(job_id: JobId) -> Option<JobId> {
    let table = table();
    table
        .job_skill_tree_fallbacks
        .binary_search_by_key(&job_id.0, |(id, _)| *id)
        .ok()
        .map(|index| JobId(table.job_skill_tree_fallbacks[index].1))
}

/// Verified fallback sprite path for a monster job id, if one exists.
pub(crate) fn monster_sprite_alias(job_id: JobId) -> Option<&'static str> {
    let table = table();
    table
        .monster_sprite_aliases
        .binary_search_by_key(&job_id.0, |(id, _)| *id)
        .ok()
        .map(|index| table.monster_sprite_aliases[index].1)
}

/// Generic monster sprites in preference order (all verified to exist).
pub(crate) fn generic_monster_sprites() -> &'static [&'static str] {
    &table().generic_monster_sprites
}

/// Verified fallback icon path for a skill id, if one exists.
pub(crate) fn skill_icon_alias(skill_id: SkillId) -> Option<&'static str> {
    let table = table();
    table
        .skill_icon_aliases
        .binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| table.skill_icon_aliases[index].1)
}

/// Generic skill icons in preference order (all verified to exist).
pub(crate) fn generic_skill_icons() -> &'static [&'static str] {
    &table().generic_skill_icons
}

// ---------------------------------------------------------------------------
// Parsing and validation.
// ---------------------------------------------------------------------------

/// Validates an alias data document and returns the row counts.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `korangar --check-aliases [PATH]`, so CI can validate the committed data
/// file (or an archive override) without the game archives, a window or a
/// network connection.
pub(crate) fn validate(content: &str) -> Result<AliasesSummary, String> {
    parse(content).map(|table| AliasesSummary::of(&table))
}

/// Validates the compiled fallback (used by `--check-aliases` without a path
/// and by the tests).
pub(crate) fn validate_compiled() -> Result<AliasesSummary, String> {
    validate(FALLBACK)
}

fn parse(content: &str) -> Result<AliasTable, String> {
    let file: AliasFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    build(file)
}

fn build(file: AliasFile) -> Result<AliasTable, String> {
    if file.version != 1 {
        return Err(format!("unsupported data file version {}", file.version));
    }
    if file.provenance.trim().is_empty() {
        return Err("the data file has no provenance".to_owned());
    }
    if file.job_skill_tree_fallbacks.is_empty() {
        return Err("the data file contains no job skill-tree fallbacks".to_owned());
    }
    if file.monster_sprite_aliases.is_empty() {
        return Err("the data file contains no monster sprite aliases".to_owned());
    }
    if file.skill_icon_aliases.is_empty() {
        return Err("the data file contains no skill icon aliases".to_owned());
    }
    if file.generic_monster_sprites.is_empty() {
        return Err("the data file contains no generic monster sprites".to_owned());
    }
    if file.generic_skill_icons.is_empty() {
        return Err("the data file contains no generic skill icons".to_owned());
    }

    validate_sorted_unique(&file.job_skill_tree_fallbacks, "job skill-tree fallback")?;
    let fallback_keys: HashSet<u16> = file.job_skill_tree_fallbacks.iter().map(|(job_id, _)| *job_id).collect();
    for (job_id, fallback_job_id) in &file.job_skill_tree_fallbacks {
        if job_id == fallback_job_id {
            return Err(format!("job {job_id}: fallback must not be the job itself"));
        }
        if fallback_keys.contains(fallback_job_id) {
            return Err(format!("job {job_id}: chained skill-tree fallback to {fallback_job_id}"));
        }
    }

    validate_sorted_unique(&file.monster_sprite_aliases, "monster sprite alias")?;
    for (job_id, path) in &file.monster_sprite_aliases {
        validate_sprite_path(path, &["몬스터\\", "npc\\"], &format!("monster {job_id}"))?;
    }
    for path in &file.generic_monster_sprites {
        validate_sprite_path(path, &["몬스터\\"], "generic monster sprite")?;
    }

    validate_sorted_unique(&file.skill_icon_aliases, "skill icon alias")?;
    for (skill_id, path) in &file.skill_icon_aliases {
        validate_sprite_path(path, &["아이템\\"], &format!("skill {skill_id}"))?;
    }
    for path in &file.generic_skill_icons {
        validate_sprite_path(path, &["아이템\\"], "generic skill icon")?;
    }

    let table = AliasTable {
        job_skill_tree_fallbacks: file.job_skill_tree_fallbacks,
        monster_sprite_aliases: file.monster_sprite_aliases.into_iter().map(|(id, path)| (id, leak(path))).collect(),
        generic_monster_sprites: file.generic_monster_sprites.into_iter().map(leak).collect(),
        skill_icon_aliases: file.skill_icon_aliases.into_iter().map(|(id, path)| (id, leak(path))).collect(),
        generic_skill_icons: file.generic_skill_icons.into_iter().map(leak).collect(),
    };

    Ok(table)
}

fn validate_sorted_unique<T>(rows: &[(u16, T)], what: &str) -> Result<(), String> {
    for pair in rows.windows(2) {
        if pair[0].0 >= pair[1].0 {
            return Err(format!(
                "{what} ids are not strictly sorted/unique: {} -> {}",
                pair[0].0, pair[1].0
            ));
        }
    }
    Ok(())
}

fn validate_sprite_path(path: &str, prefixes: &[&str], what: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err(format!("{what}: empty sprite path"));
    }
    if !prefixes.iter().any(|prefix| path.starts_with(prefix)) {
        return Err(format!("{what}: {path:?} is outside {prefixes:?}"));
    }
    if path.ends_with(".spr") || path.ends_with(".act") {
        return Err(format!("{what}: {path:?} must not carry a file extension"));
    }
    if path.to_lowercase().contains("missing") {
        return Err(format!("{what}: {path:?} is a missing placeholder"));
    }
    Ok(())
}

/// All monster sprite alias rows (test/archive-audit helper).
#[cfg(test)]
pub(crate) fn monster_sprite_aliases() -> &'static [(u16, &'static str)] {
    &table().monster_sprite_aliases
}

/// All skill icon alias rows (test/archive-audit helper).
#[cfg(test)]
pub(crate) fn skill_icon_aliases() -> &'static [(u16, &'static str)] {
    &table().skill_icon_aliases
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_fallback_is_valid() {
        let summary = validate_compiled().expect("the compiled alias table has to be valid");
        assert_eq!(summary.job_skill_tree_fallbacks, 56);
        assert_eq!(summary.monster_sprite_aliases, 232);
        assert_eq!(summary.skill_icon_aliases, 241);
        assert_eq!(generic_monster_sprites().len(), 7);
        assert_eq!(generic_skill_icons().len(), 3);
    }

    #[test]
    fn job_skill_tree_fallback_lookups() {
        assert_eq!(job_skill_tree_fallback(JobId(4080)), Some(JobId(4054)));
        assert_eq!(job_skill_tree_fallback(JobId(4110)), Some(JobId(4097)));
        assert_eq!(job_skill_tree_fallback(JobId(4316)), Some(JobId(4239)));
        assert_eq!(job_skill_tree_fallback(JobId(0)), None);
        assert_eq!(job_skill_tree_fallback(JobId(65535)), None);
    }

    #[test]
    fn monster_sprite_alias_lookups() {
        assert_eq!(monster_sprite_alias(JobId(1016)), Some("몬스터\\skel_archer"));
        assert_eq!(monster_sprite_alias(JobId(1047)), Some("npc\\pecopeco_egg"));
        assert_eq!(monster_sprite_alias(JobId(2308)), Some("몬스터\\md_ninja_ghost"));
        assert_eq!(monster_sprite_alias(JobId(65535)), None);
        assert!(generic_monster_sprites().iter().all(|path| path.starts_with("몬스터\\")));
    }

    #[test]
    fn skill_icon_alias_lookups() {
        assert_eq!(skill_icon_alias(SkillId(96)), Some("아이템\\rk_enchantblade"));
        assert_eq!(skill_icon_alias(SkillId(720)), Some("아이템\\wl_jackfrost"));
        assert_eq!(skill_icon_alias(SkillId(10016)), Some("아이템\\nv_basic"));
        assert_eq!(skill_icon_alias(SkillId(65535)), None);
        assert!(generic_skill_icons().iter().all(|path| path.starts_with("아이템\\")));
    }

    #[test]
    fn all_lookups_use_the_data_tables() {
        // Every row of the compiled table must be found by the binary search.
        let table = compiled();
        for (job_id, path) in &table.monster_sprite_aliases {
            assert_eq!(monster_sprite_alias(JobId(*job_id)), Some(*path));
        }
        for (skill_id, path) in &table.skill_icon_aliases {
            assert_eq!(skill_icon_alias(SkillId(*skill_id)), Some(*path));
        }
        for (job_id, fallback) in &table.job_skill_tree_fallbacks {
            assert_eq!(job_skill_tree_fallback(JobId(*job_id)), Some(JobId(*fallback)));
        }
    }

    #[test]
    fn malformed_data_is_rejected() {
        let error = validate("(version: 1,").expect_err("malformed RON must be rejected");
        assert!(error.contains("schema error"), "{error}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let tampered = FALLBACK.replace("    version: 1,", "    version: 1,\n    unknown_field: true,");
        let error = validate(&tampered).expect_err("unknown fields must be rejected");
        assert!(error.contains("Unexpected field named `unknown_field`"), "{error}");
    }

    #[test]
    fn unsupported_versions_are_rejected() {
        let tampered = FALLBACK.replace("    version: 1,", "    version: 2,");
        let error = validate(&tampered).expect_err("unsupported versions must be rejected");
        assert!(error.contains("unsupported data file version 2"), "{error}");
    }

    #[test]
    fn empty_tables_are_rejected() {
        let content = r#"
(
    provenance: "test",
    version: 1,
    job_skill_tree_fallbacks: [],
    monster_sprite_aliases: [],
    generic_monster_sprites: ["몬스터\\poring"],
    skill_icon_aliases: [],
    generic_skill_icons: ["아이템\\nv_basic"],
)
"#;
        let error = validate(content).expect_err("empty tables must be rejected");
        assert!(error.contains("no job skill-tree fallbacks"), "{error}");
    }

    #[test]
    fn out_of_order_ids_are_rejected() {
        let content = r#"
(
    provenance: "test",
    version: 1,
    job_skill_tree_fallbacks: [(13, 7), (8, 0)],
    monster_sprite_aliases: [(1016, "몬스터\\skel_archer")],
    generic_monster_sprites: ["몬스터\\poring"],
    skill_icon_aliases: [(96, "아이템\\rk_enchantblade")],
    generic_skill_icons: ["아이템\\nv_basic"],
)
"#;
        let error = validate(content).expect_err("unsorted ids must be rejected");
        assert!(error.contains("not strictly sorted"), "{error}");
    }

    #[test]
    fn chained_job_fallbacks_are_rejected() {
        let content = r#"
(
    provenance: "test",
    version: 1,
    job_skill_tree_fallbacks: [(7, 0), (13, 7)],
    monster_sprite_aliases: [(1016, "몬스터\\skel_archer")],
    generic_monster_sprites: ["몬스터\\poring"],
    skill_icon_aliases: [(96, "아이템\\rk_enchantblade")],
    generic_skill_icons: ["아이템\\nv_basic"],
)
"#;
        let error = validate(content).expect_err("chained fallbacks must be rejected");
        assert!(error.contains("chained skill-tree fallback"), "{error}");
    }

    #[test]
    fn bad_sprite_paths_are_rejected() {
        for bad_path in ["data\\sprite\\몬스터\\poring", "몬스터\\missing", "몬스터\\poring.spr"] {
            let escaped_path = bad_path.replace('\\', "\\\\");
            let content = format!(
                r#"
(
    provenance: "test",
    version: 1,
    job_skill_tree_fallbacks: [(13, 7)],
    monster_sprite_aliases: [(1016, "{escaped_path}")],
    generic_monster_sprites: ["몬스터\\poring"],
    skill_icon_aliases: [(96, "아이템\\rk_enchantblade")],
    generic_skill_icons: ["아이템\\nv_basic"],
)
"#
            );
            let error = validate(&content).expect_err("a bad sprite path must be rejected");
            assert!(error.contains("monster 1016"), "{bad_path}: {error}");
        }
    }

    #[test]
    fn bad_icon_paths_are_rejected() {
        let content = r#"
(
    provenance: "test",
    version: 1,
    job_skill_tree_fallbacks: [(13, 7)],
    monster_sprite_aliases: [(1016, "몬스터\\skel_archer")],
    generic_monster_sprites: ["몬스터\\poring"],
    skill_icon_aliases: [(96, "몬스터\\poring")],
    generic_skill_icons: ["아이템\\nv_basic"],
)
"#;
        let error = validate(content).expect_err("a bad icon path must be rejected");
        assert!(error.contains("skill 96"), "{error}");
    }
}
