//! Official monster skill -> ACT action mapping
//! (`data\client\monster_skill_actions.ron`).
//!
//! The official client does **not** play the player skill-action table for
//! monsters. `CSession::GetMonsterSkillInfo`
//! (`Source/Contents/Monster/MonsterXmlActions.cpp:43`) reads
//! `data\MonsterSkillInfo.xml` keyed by the monster's lower-cased job name and
//! the skill id, and the call sites play the row with
//! `SetAction(action * 8 - 8, 1, loopType == 0)`
//! (`GameModePackets.cpp:12958-12961` in `NotifySkillCasting`,
//! `:18371-18375` in `ReceiveSkillUse`).
//!
//! The table is **data** in this client: it ships in the client archive and is
//! loaded at runtime through [`GameFileLoader`], so a server can override the
//! mapping without rebuilding the client. The compiled fallback embeds the same
//! data file with `include_str!` so the two can never drift apart.
//!
//! Generation: `tools/gen_monster_skill_actions.py` from the facts extracted by
//! `tools/extract_monster_skill_motions.py`. `--check` is the drift guard.

use std::sync::OnceLock;

use ragnarok_packets::SkillId;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\monster_skill_actions.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/monster_skill_actions.ron");

/// One official monster skill row: `action_index` is the argument of the
/// official `SetAction(action * 8 - 8, 1, ...)` (the XML `action - 1`), and
/// `looping` mirrors `loopType != 0` (`MT_LOOP`; `loopType == 0` is
/// `MT_ONETIME`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonsterSkillAction {
    pub action_index: u8,
    pub looping: bool,
}

#[derive(Clone, Default)]
struct Mapping {
    /// Sorted by `(monster, skill)` for the binary search.
    rows: Vec<(String, u16, MonsterSkillAction)>,
}

#[derive(Deserialize)]
struct MonsterSkillActionFile {
    #[allow(dead_code)]
    provenance: String,
    actions: Vec<MonsterSkillActionEntry>,
}

#[derive(Deserialize)]
struct MonsterSkillActionEntry {
    monster: String,
    skill: u16,
    action: ActionIndex,
    looping: bool,
}

/// RON spelling of the ACT action index (`Action(5)`).
#[derive(Deserialize)]
enum ActionIndex {
    Action(u8),
}

impl ActionIndex {
    fn index(&self) -> u8 {
        match self {
            Self::Action(index) => *index,
        }
    }
}

/// The compiled fallback mapping, parsed on first use (also used by tests).
static COMPILED: OnceLock<Mapping> = OnceLock::new();
/// Mapping loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<Mapping> = OnceLock::new();

fn parse(content: &str) -> Result<Mapping, String> {
    let file: MonsterSkillActionFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut rows = Vec::with_capacity(file.actions.len());

    for entry in file.actions {
        // The official lookup lower-cases the job name (`_strlwr`); normalize
        // the data rows too so a server override with a mixed-case name still
        // resolves.
        rows.push((
            entry.monster.trim_end_matches('\0').to_lowercase(),
            entry.skill,
            MonsterSkillAction {
                action_index: entry.action.index(),
                looping: entry.looping,
            },
        ));
    }

    rows.sort_unstable_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    // Duplicate (monster, skill) keys would make the binary search ambiguous;
    // the official `XmlNode::FindChild` returns the first child.
    rows.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    Ok(Mapping { rows })
}

fn compiled() -> &'static Mapping {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("monster skill actions: compiled fallback is invalid ({error})");
            Mapping::default()
        }
    })
}

fn mapping() -> &'static Mapping {
    ARCHIVE.get().unwrap_or_else(compiled)
}

/// Loads the mapping from the client archive (or keeps the compiled fallback).
///
/// Returns the number of loaded entries. A malformed data file is reported and
/// the compiled fallback is used instead, so a bad override can never leave the
/// client without a usable mapping.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let content = match from_archive.as_deref() {
        Some(content) => content,
        None => FALLBACK,
    };

    let mapping = match parse(content) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("monster skill actions: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().clone()
        }
    };

    let count = mapping.rows.len();
    let _ = ARCHIVE.set(mapping);
    crate::logging::log_info!("monster skill actions: loaded {count} entries");
    count
}

/// Returns the official monster motion of `skill_id` for the monster job name
/// `monster`, or `None` when the official table has no row.
///
/// The official lookup lower-cases the job name (`_strlwr` in
/// `GetMonsterSkillInfo`); `JobIdentity` strings are upper-case, so callers can
/// pass the identity directly. The monster sprite and the job-name table share
/// these stems (`MonsterResources.cpp:10` builds `몬스터\%s.spr` from the same
/// name).
pub(crate) fn action_for_name(monster: &str, skill_id: SkillId) -> Option<MonsterSkillAction> {
    let monster = monster.trim_end_matches('\0').to_lowercase();
    mapping()
        .rows
        .binary_search_by(|(name, skill, _)| name.as_str().cmp(monster.as_str()).then(skill.cmp(&skill_id.0)))
        .ok()
        .map(|index| mapping().rows[index].2)
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::action_for_name;

    #[test]
    fn official_monster_motion_is_resolved_by_job_name_and_skill() {
        // data\MonsterSkillInfo.xml: <baphomet_i><_661><action>6</action>
        // → SetAction(6 * 8 - 8 = 40, 1, loopType == 0),
        // i.e. ACT action 5, one-shot.
        let action = action_for_name("BAPHOMET_I", SkillId(661)).expect("baphomet_i has a skill 661 motion");
        assert_eq!(action.action_index, 5);
        assert_eq!(action.looping, false);
        assert_eq!(action.action_index as usize * 8, 40);

        // Action 1 → block 0 (the monster stand/idle block), one-shot.
        let action = action_for_name("alice", SkillId(28)).expect("alice has a skill 28 motion");
        assert_eq!(action.action_index, 0);
        assert_eq!(action.action_index as usize * 8, 0);
        assert!(!action.looping);

        // `loopType = 1` is MT_LOOP: the action repeats.
        let action = action_for_name("ambernite", SkillId(201)).expect("ambernite has a skill 201 motion");
        assert_eq!(action.action_index, 0);
        assert!(action.looping);
    }

    #[test]
    fn the_lookup_lowercases_the_official_job_name() {
        // `GetMonsterSkillInfo` calls `_strlwr` on the job name before looking
        // it up; JobIdentity is upper-case.
        assert_eq!(
            action_for_name("BAPHOMET_I", SkillId(661)),
            action_for_name("baphomet_i", SkillId(661))
        );
        // Trailing NULs appear in some JobIdentity strings.
        assert_eq!(
            action_for_name("baphomet_i\0", SkillId(661)),
            action_for_name("baphomet_i", SkillId(661))
        );
    }

    #[test]
    fn unknown_monsters_and_skills_have_no_motion() {
        assert_eq!(action_for_name("no_such_monster", SkillId(661)), None);
        assert_eq!(action_for_name("baphomet_i", SkillId(9999)), None);
        // The official XML has a `goblin_xmas` row, but the job name of job
        // 1245/1607 is `GOBLINE_XMAS`; the official lookup cannot reach it
        // either (the XML typo is kept as a quirk).
        assert_eq!(action_for_name("GOBLINE_XMAS", SkillId(194)), None);
        assert!(action_for_name("goblin_xmas", SkillId(194)).is_some());
    }
}
