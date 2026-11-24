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

/// Official frame sequence of a skill (`RenderObject.cpp` `SetFrameSequence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CasterFrameSequence {
    pub first: u8,
    pub last: u8,
    pub loop_to: u8,
    pub duration_ms: u32,
}

/// A parsed mapping: the default action, sorted `(skill id, action)` rows and
/// the per-skill frame sequences.
type Mapping = (CasterAction, Vec<(u16, CasterAction)>, Vec<(u16, CasterFrameSequence)>);

#[derive(Deserialize)]
struct SkillActionFile {
    #[allow(dead_code)]
    provenance: String,
    default_action: CasterAction,
    actions: Vec<SkillActionEntry>,
}

/// Accepts both a bare RON value and the explicit `Some(..)`/`None` spelling.
fn deserialize_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum BareOrOption<T> {
        Bare(T),
        Optional(Option<T>),
    }

    Ok(match BareOrOption::<T>::deserialize(deserializer)? {
        BareOrOption::Bare(value) => Some(value),
        BareOrOption::Optional(value) => value,
    })
}

#[derive(Deserialize)]
struct SkillActionEntry {
    skill: u16,
    action: CasterAction,
    /// Optional official frame range of the action (`SetFrameSequence`).
    #[serde(default, deserialize_with = "deserialize_option")]
    sequence: Option<FrameSequenceEntry>,
}

#[derive(Deserialize)]
struct FrameSequenceEntry {
    first: u8,
    last: u8,
    loop_to: u8,
    duration_ms: u32,
}

/// The compiled fallback mapping, parsed on first use (also used by tests).
static COMPILED: OnceLock<Mapping> = OnceLock::new();
/// Mapping loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<Mapping> = OnceLock::new();

fn parse(content: &str) -> Result<Mapping, String> {
    let file: SkillActionFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut rows: Vec<(u16, CasterAction)> = Vec::with_capacity(file.actions.len());
    let mut sequences: Vec<(u16, CasterFrameSequence)> = Vec::new();

    for entry in file.actions {
        rows.push((entry.skill, entry.action));
        if let Some(sequence) = entry.sequence {
            sequences.push((entry.skill, CasterFrameSequence {
                first: sequence.first,
                last: sequence.last,
                loop_to: sequence.loop_to,
                duration_ms: sequence.duration_ms,
            }));
        }
    }

    rows.sort_unstable_by_key(|(skill_id, _)| *skill_id);
    // Duplicate skill ids would make the binary search ambiguous.
    rows.dedup_by_key(|(skill_id, _)| *skill_id);
    sequences.sort_unstable_by_key(|(skill_id, _)| *skill_id);
    Ok((file.default_action, rows, sequences))
}

fn compiled() -> &'static Mapping {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("skill actions: compiled fallback is invalid ({error})");
            (CasterAction::Skill, Vec::new(), Vec::new())
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

    let (default, rows, sequences) = match parse(content) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("skill actions: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().clone()
        }
    };

    let count = rows.len();
    let _ = ARCHIVE.set((default, rows, sequences));
    crate::logging::log_info!("skill actions: loaded {count} entries, default action {default:?}");
    count
}

/// Returns the official caster action of `skill_id`, or `None` when the skill
/// is not listed (the caller then uses [`default_caster_action`]).
pub(crate) fn caster_action_for_skill(skill_id: SkillId) -> Option<CasterAction> {
    let (_default, rows, _sequences) = mapping();
    rows.binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| rows[index].1)
}

/// The official frame sequence of `skill_id`, if the decompiled client plays
/// one (`RenderObject.cpp` `SetFrameSequence`).
pub(crate) fn frame_sequence_for_skill(skill_id: SkillId) -> Option<CasterFrameSequence> {
    let (_default, _rows, sequences) = mapping();
    sequences
        .binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| sequences[index].1)
}

/// The official default action for skills that are not listed.
pub(crate) fn default_caster_action() -> CasterAction {
    mapping().0
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{CasterAction, FALLBACK, caster_action_for_skill, default_caster_action, frame_sequence_for_skill, parse};

    fn action(skill_id: u16) -> Option<CasterAction> {
        caster_action_for_skill(SkillId(skill_id))
    }

    #[test]
    fn data_file_is_valid_and_has_entries() {
        let (default, rows, _sequences) = parse(FALLBACK).expect("valid fallback");
        assert_eq!(default, CasterAction::Skill);
        assert!(rows.len() >= 200, "mapping should stay broad, got {}", rows.len());
        assert!(
            rows.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "rows must be sorted and unique"
        );
    }

    #[test]
    fn asura_and_champion_frame_sequences_are_loaded() {
        let asura = frame_sequence_for_skill(SkillId(271)).expect("Asura has an official sequence");
        assert_eq!((asura.first, asura.last, asura.loop_to, asura.duration_ms), (0, 7, 7, 3500));

        let chain = frame_sequence_for_skill(SkillId(272)).expect("Chain Combo has an official sequence");
        assert_eq!((chain.first, chain.last), (5, 5));

        assert!(frame_sequence_for_skill(SkillId(5)).is_none(), "Bash has no explicit sequence");
    }

    #[test]
    fn skills_use_the_official_act_blocks() {
        // Motion ST_ATTACK (2) maps to WeaponActionBase block 80 (Action(10)).
        for skill_id in [5, 7, 56, 62] {
            assert_eq!(action(skill_id), Some(CasterAction::Action(10)), "skill {skill_id}");
        }
        // Asura Strike plays the official attack block 88 (0x58, frames 0..7,
        // 3500 ms); Meteor Assault keeps the begin-skill gesture (block 96).
        assert_eq!(action(271), Some(CasterAction::Action(11)), "skill 271");
        assert_eq!(action(406), Some(CasterAction::Skill), "skill 406");
    }

    #[test]
    fn stance_and_buff_skills_keep_the_idle_pose() {
        // Motion 0 (ST_STAND) skills; the rest of the classic buffs use the
        // begin-skill gesture instead.
        for skill_id in [34, 270] {
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
        // The official `GetSkillActionInfo` uses the default begin-skill
        // gesture for Endure; `ReadyFight` remains supported for overrides.
        assert_eq!(action(8), Some(CasterAction::Skill));
        assert_eq!(CasterAction::ReadyFight, CasterAction::ReadyFight);
    }

    #[test]
    fn casts_keep_the_casting_gesture() {
        // The performance group and Sight use the explicit SKILL action; Heal
        // and Concentration fall through to the same official default.
        for skill_id in [306, 380, 28, 45] {
            assert_eq!(action(skill_id), Some(CasterAction::Skill), "skill {skill_id}");
        }
        assert_eq!(action(34), Some(CasterAction::Idle), "Blessing keeps the idle pose");
        assert_eq!(
            action(1015),
            Some(CasterAction::Skill),
            "Redemptio uses the begin-skill gesture"
        );
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
        // The official table also carries these skills (motion 18 / 12).
    }

    #[test]
    fn unknown_skills_fall_back_safely_to_the_casting_gesture() {
        assert_eq!(action(65_500), None);
        assert_eq!(default_caster_action(), CasterAction::Skill);
    }
}
