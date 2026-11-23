//! Official skill -> caster action mapping (`data\client\skill_actions.ron`).
//!
//! The official Windows client keeps a hard-coded table that assigns every
//! skill an entity action (`ATTACK`, `IDLE`, `PICKUP`, `SKILL`, `READYFIGHT`,
//! or "prevent the default skill action"). The table is **data** in this
//! client: it ships in the client archive and is loaded at runtime through
//! [`GameFileLoader`], so a server can override the mapping without rebuilding
//! the client. The compiled fallback embeds the same data file with
//! `include_str!` so the two can never drift apart.
//!
//! Generation: `tools/gen_skill_actions.py` from the facts extracted by
//! `tools/extract_skill_actions.py` (roBrowserLegacy RE dump of the Windows
//! client tables). `--check` is the drift guard.

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::SkillId;
use serde::Deserialize;

use crate::loaders::GameFileLoader;
use crate::world::CasterAction;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_actions.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_actions.ron");

/// A parsed mapping: the default action plus sorted `(skill id, action)` rows.
type Mapping = (CasterAction, Vec<(u16, CasterAction)>);

#[derive(Deserialize)]
struct SkillActionFile {
    #[allow(dead_code)]
    provenance: String,
    default_action: CasterAction,
    actions: Vec<SkillActionEntry>,
}

#[derive(Deserialize)]
struct SkillActionEntry {
    skill: u16,
    action: CasterAction,
}

/// The compiled fallback mapping, parsed on first use (also used by tests).
static COMPILED: OnceLock<Mapping> = OnceLock::new();
/// Mapping loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<Mapping> = OnceLock::new();

fn parse(content: &str) -> Result<Mapping, String> {
    let file: SkillActionFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut rows: Vec<(u16, CasterAction)> = file.actions.into_iter().map(|entry| (entry.skill, entry.action)).collect();
    rows.sort_unstable_by_key(|(skill_id, _)| *skill_id);
    // Duplicate skill ids would make the binary search ambiguous.
    rows.dedup_by_key(|(skill_id, _)| *skill_id);
    Ok((file.default_action, rows))
}

fn compiled() -> &'static Mapping {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("skill actions: compiled fallback is invalid ({error})");
            (CasterAction::Skill, Vec::new())
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

    let (default, rows) = match parse(content) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("skill actions: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().clone()
        }
    };

    let count = rows.len();
    let _ = ARCHIVE.set((default, rows));
    crate::logging::log_debug!("skill actions: loaded {count} entries, default action {default:?}");
    count
}

/// Returns the official caster action of `skill_id`, or `None` when the skill
/// is not listed (the caller then uses [`default_caster_action`]).
pub(crate) fn caster_action_for_skill(skill_id: SkillId) -> Option<CasterAction> {
    let (_default, rows) = mapping();
    rows.binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| rows[index].1)
}

/// The official default action for skills that are not listed.
pub(crate) fn default_caster_action() -> CasterAction {
    mapping().0
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{CasterAction, FALLBACK, caster_action_for_skill, default_caster_action, parse};

    fn action(skill_id: u16) -> Option<CasterAction> {
        caster_action_for_skill(SkillId(skill_id))
    }

    #[test]
    fn data_file_is_valid_and_has_entries() {
        let (default, rows) = parse(FALLBACK).expect("valid fallback");
        assert_eq!(default, CasterAction::Skill);
        assert!(rows.len() >= 200, "mapping should stay broad, got {}", rows.len());
        assert!(
            rows.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "rows must be sorted and unique"
        );
    }

    #[test]
    fn attack_skills_play_the_attack_pose() {
        // Bash, Magnum Break, Pierce, Bowling Bash, Meteor Assault, Asura.
        for skill_id in [5, 7, 56, 62, 406, 271] {
            assert_eq!(action(skill_id), Some(CasterAction::Attack), "skill {skill_id}");
        }
    }

    #[test]
    fn stance_and_buff_skills_keep_the_idle_pose() {
        for skill_id in [1, 29, 261, 262, 264, 268, 270, 401, 1015] {
            assert_eq!(action(skill_id), Some(CasterAction::Idle), "skill {skill_id}");
        }
    }

    #[test]
    fn traps_use_the_pickup_pose() {
        for skill_id in [116, 121, 123, 125, 151] {
            assert_eq!(action(skill_id), Some(CasterAction::Pickup), "skill {skill_id}");
        }
    }

    #[test]
    fn endures_ready_fight_stance_is_preserved() {
        // `SkillAction[SK.SM_ENDURE]` uses `makeGenericSkillAction('READYFIGHT')`;
        // the old hand-maintained table downgraded it to the idle pose.
        assert_eq!(action(8), Some(CasterAction::ReadyFight));
    }

    #[test]
    fn casts_keep_the_casting_gesture() {
        // Blessing, the bard/dancer performance group and Sight use the
        // explicit `SKILL` action; other casts fall through to the same default.
        for skill_id in [34, 306, 380] {
            assert_eq!(action(skill_id), Some(CasterAction::Skill), "skill {skill_id}");
        }
        assert_eq!(action(28), None, "Heal is not listed and uses the default");
    }

    #[test]
    fn ready_shot_and_throw_use_the_official_act_action() {
        // The official client's `GetSkillActionInfo` puts the caster into an
        // actor state (ST_READYSHOT = 18 / ST_THROW = 12); `CGameActor::SetState`
        // maps that state to the flat ACT animation 88 / 40 through
        // `WeaponActionBase`, i.e. ACT action 11 / 5 (the classic 8-direction
        // ACT layout). The roBrowser action dump only knows "Attack"/"Idle" for
        // these skills, so the data file carries the curated override.
        assert_eq!(action(47), Some(CasterAction::Action(11)), "AC_SHOWER");
        assert_eq!(action(394), Some(CasterAction::Action(11)), "CG_ARROWVULCAN");
        assert_eq!(action(446), Some(CasterAction::Action(5)), "AM_BERSERKPITCHER");
    }

    #[test]
    fn unknown_skills_fall_back_safely_to_the_casting_gesture() {
        assert_eq!(action(65_500), None);
        assert_eq!(default_caster_action(), CasterAction::Skill);
    }
}
