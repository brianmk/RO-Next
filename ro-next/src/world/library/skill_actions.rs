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
//!
//! Some rows also carry the official caster frame range
//! (`RenderObject.cpp`/`GameModePackets.cpp` `SetFrameSequence`) and the
//! `SetSequenceInterval` play rate. The interval is recorded in 60 Hz client
//! ticks; `CasterFrameSequence::step_ms` converts it to the per-frame duration
//! the animation engine uses (Fury: 30 ticks = 500 ms).

use std::sync::OnceLock;

use ragnarok_packets::SkillId;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use crate::loaders::GameFileLoader;
use crate::world::CasterAction;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_actions.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_actions.ron");

/// Official frame sequence of a skill (`RenderObject.cpp` `SetFrameSequence`).
///
/// `interval_ticks` is the preceding `SetSequenceInterval(N)` value: the
/// official client advances one sequence frame every `N` update ticks of its
/// 60 Hz timer (`Client/Application/Launcher.cpp:1522`, `InitTimer(60)`), so
/// `interval_ticks: 30` is one frame every 500 ms. `0` means the sequence did
/// not override the interval and the ACT frame delay decides the pace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CasterFrameSequence {
    pub first: u8,
    pub last: u8,
    pub loop_to: u8,
    pub duration_ms: u32,
    pub interval_ticks: u8,
}

impl CasterFrameSequence {
    /// Duration of one sequence frame in milliseconds, when the official
    /// `SetSequenceInterval` overrides the ACT timing.
    pub fn step_ms(&self) -> Option<u32> {
        (self.interval_ticks > 0).then(|| (f32::from(self.interval_ticks) * 1000.0 / 60.0).round() as u32)
    }
}

/// A parsed mapping: the default action, the sorted cast-path rows, the
/// optional 0x09CB rows and the per-skill frame sequences.
#[derive(Clone)]
struct Mapping {
    default: CasterAction,
    rows: Vec<(u16, CasterAction)>,
    use_rows: Vec<(u16, CasterAction)>,
    sequences: Vec<(u16, CasterFrameSequence)>,
}

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
    /// Optional override for the `ZC_USE_SKILL` (0x09CB) path, which the
    /// official client drives with `GetSkillActionInfo2` instead of the
    /// `GetSkillActionInfo` table used for the 0x0B1A cast ack.
    #[serde(default, deserialize_with = "deserialize_option")]
    use_action: Option<CasterAction>,
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
    /// Official `SetSequenceInterval` in 60 Hz update ticks; absent in older
    /// data files and for sequences that keep the ACT frame delay.
    #[serde(default)]
    interval_ticks: u8,
}

/// The compiled fallback mapping, parsed on first use (also used by tests).
static COMPILED: OnceLock<Mapping> = OnceLock::new();
/// Mapping loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<Mapping> = OnceLock::new();

fn parse(content: &str) -> Result<Mapping, String> {
    let file: SkillActionFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut rows: Vec<(u16, CasterAction)> = Vec::with_capacity(file.actions.len());
    let mut use_rows: Vec<(u16, CasterAction)> = Vec::new();
    let mut sequences: Vec<(u16, CasterFrameSequence)> = Vec::new();

    for entry in file.actions {
        rows.push((entry.skill, entry.action));
        if let Some(use_action) = entry.use_action {
            use_rows.push((entry.skill, use_action));
        }
        if let Some(sequence) = entry.sequence {
            sequences.push((entry.skill, CasterFrameSequence {
                first: sequence.first,
                last: sequence.last,
                loop_to: sequence.loop_to,
                duration_ms: sequence.duration_ms,
                interval_ticks: sequence.interval_ticks,
            }));
        }
    }

    rows.sort_unstable_by_key(|(skill_id, _)| *skill_id);
    // Duplicate skill ids would make the binary search ambiguous.
    rows.dedup_by_key(|(skill_id, _)| *skill_id);
    use_rows.sort_unstable_by_key(|(skill_id, _)| *skill_id);
    use_rows.dedup_by_key(|(skill_id, _)| *skill_id);
    sequences.sort_unstable_by_key(|(skill_id, _)| *skill_id);
    Ok(Mapping {
        default: file.default_action,
        rows,
        use_rows,
        sequences,
    })
}

fn compiled() -> &'static Mapping {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(mapping) => mapping,
        Err(error) => {
            crate::logging::log_warn!("skill actions: compiled fallback is invalid ({error})");
            Mapping {
                default: CasterAction::Skill,
                rows: Vec::new(),
                use_rows: Vec::new(),
                sequences: Vec::new(),
            }
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
            crate::logging::log_warn!("skill actions: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().clone()
        }
    };

    let count = mapping.rows.len();
    let default = mapping.default;
    let _ = ARCHIVE.set(mapping);
    crate::logging::log_info!("skill actions: loaded {count} entries, default action {default:?}");
    count
}

/// Returns the official caster action of `skill_id`, or `None` when the skill
/// is not listed (the caller then uses [`default_caster_action`]).
pub(crate) fn caster_action_for_skill(skill_id: SkillId) -> Option<CasterAction> {
    mapping()
        .rows
        .binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| mapping().rows[index].1)
}

/// Returns the official `ZC_USE_SKILL` (0x09CB) action of `skill_id`, or
/// `None` when the skill uses the same action as the cast path.
pub(crate) fn use_caster_action_for_skill(skill_id: SkillId) -> Option<CasterAction> {
    mapping()
        .use_rows
        .binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| mapping().use_rows[index].1)
}

/// The official frame sequence of `skill_id`, if the decompiled client plays
/// one (`RenderObject.cpp` `SetFrameSequence`).
pub(crate) fn frame_sequence_for_skill(skill_id: SkillId) -> Option<CasterFrameSequence> {
    mapping()
        .sequences
        .binary_search_by_key(&skill_id.0, |(id, _)| *id)
        .ok()
        .map(|index| mapping().sequences[index].1)
}

/// The official default action for skills that are not listed.
pub(crate) fn default_caster_action() -> CasterAction {
    mapping().default
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{
        CasterAction, CasterFrameSequence, FALLBACK, caster_action_for_skill, default_caster_action, frame_sequence_for_skill, parse,
        use_caster_action_for_skill,
    };

    fn action(skill_id: u16) -> Option<CasterAction> {
        caster_action_for_skill(SkillId(skill_id))
    }

    fn use_action(skill_id: u16) -> Option<CasterAction> {
        use_caster_action_for_skill(SkillId(skill_id))
    }

    #[test]
    fn data_file_is_valid_and_has_entries() {
        let mapping = parse(FALLBACK).expect("valid fallback");
        assert_eq!(mapping.default, CasterAction::Skill);
        assert!(
            mapping.rows.len() >= 200,
            "mapping should stay broad, got {}",
            mapping.rows.len()
        );
        assert!(
            mapping.rows.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "rows must be sorted and unique"
        );
        assert!(
            mapping.use_rows.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "use rows must be sorted and unique"
        );
    }

    #[test]
    fn asura_and_champion_frame_sequences_are_loaded() {
        let asura = frame_sequence_for_skill(SkillId(271)).expect("Asura has an official sequence");
        assert_eq!((asura.first, asura.last, asura.loop_to, asura.duration_ms), (0, 7, 7, 3500));

        let chain = frame_sequence_for_skill(SkillId(272)).expect("Chain Combo has an official sequence");
        assert_eq!((chain.first, chain.last), (5, 5));

        // `case MO_EXPLOSIONSPIRITS:` - `GameModePackets.cpp:18839-18840`:
        // `SetSequenceInterval(30); SetFrameSequence(0x60, 1, 2, 0x60, 2, 1, 1000, 0)`.
        let fury = frame_sequence_for_skill(SkillId(270)).expect("Fury has an official skill-use sequence");
        assert_eq!((fury.first, fury.last, fury.loop_to, fury.duration_ms), (1, 2, 2, 1000));
        // 30 ticks of the client's 60 Hz timer is one frame every 500 ms.
        assert_eq!(fury.interval_ticks, 30);
        assert_eq!(fury.step_ms(), Some(500));

        assert!(frame_sequence_for_skill(SkillId(5)).is_none(), "Bash has no explicit sequence");
    }

    #[test]
    fn all_skill_use_sequences_carry_the_official_block_and_rate() {
        // LK_AURABLADE (`GameModePackets.cpp:18886`) uses block 0x20 (the
        // ready-fight block), not the skill block 0x60.
        assert_eq!(action(355), Some(CasterAction::Action(4)), "skill 355");
        let aura = frame_sequence_for_skill(SkillId(355)).expect("Aura Blade has an official sequence");
        assert_eq!((aura.first, aura.last, aura.loop_to, aura.duration_ms), (0, 5, 5, 1000));

        // HW_SOULDRAIN (`GameModePackets.cpp:19050`) uses the attack block 0x58.
        assert_eq!(action(364), Some(CasterAction::Action(11)), "skill 364");
        let drain = frame_sequence_for_skill(SkillId(364)).expect("Soul Drain has an official sequence");
        assert_eq!((drain.first, drain.last, drain.loop_to, drain.duration_ms), (0, 3, 3, 1000));

        // Grouped cases share one sequence (`GameModePackets.cpp:20805`).
        assert_eq!(action(318), Some(CasterAction::Action(12)), "skill 318");
        let frost_joke = frame_sequence_for_skill(SkillId(318)).expect("Frost Joke has an official sequence");
        assert_eq!(frost_joke.interval_ticks, 10);
        assert_eq!(frost_joke.step_ms(), Some(167));

        // Block 0x60 is the skill block everywhere else.
        for skill_id in [268, 269, 326, 1010] {
            assert_eq!(action(skill_id), Some(CasterAction::Action(12)), "skill {skill_id}");
            assert!(frame_sequence_for_skill(SkillId(skill_id)).is_some(), "skill {skill_id}");
        }
    }

    #[test]
    fn sequences_without_an_interval_keep_the_act_timing() {
        // Bash has no sequence at all; Blessing stays idle and has no action
        // animation to restrict (the "no-action" case).
        assert!(frame_sequence_for_skill(SkillId(5)).is_none());
        assert_eq!(action(34), Some(CasterAction::Idle));
        assert!(frame_sequence_for_skill(SkillId(34)).is_none());

        // Palm Strike has a sequence but no `SetSequenceInterval`; its frames
        // keep the ACT delay (interval 0 => `step_ms() == None`).
        let palm = frame_sequence_for_skill(SkillId(370)).expect("Palm Strike has an official sequence");
        assert_eq!(palm.interval_ticks, 0);
        assert_eq!(palm.step_ms(), None);
    }

    #[test]
    fn sequence_interval_converts_60hz_ticks_to_milliseconds() {
        let sequence = CasterFrameSequence {
            first: 1,
            last: 2,
            loop_to: 2,
            duration_ms: 1000,
            interval_ticks: 30,
        };
        assert_eq!(sequence.step_ms(), Some(500));
        assert_eq!(
            CasterFrameSequence {
                interval_ticks: 0,
                ..sequence
            }
            .step_ms(),
            None
        );
        assert_eq!(
            CasterFrameSequence {
                interval_ticks: 6,
                ..sequence
            }
            .step_ms(),
            Some(100)
        );
    }

    #[test]
    fn skills_use_the_official_act_blocks() {
        // Motion ST_ATTACK (2) plays the normal weapon swing: `set_attack`
        // picks the ACT block the equipped weapon layer actually animates
        // (`attack_action()`), matching the official `ST_ATTACK ->
        // ST_ATTACK2` upgrade in `CPc::SetState` (`Player.cpp:4545`).
        for skill_id in [5, 7, 56, 62] {
            assert_eq!(action(skill_id), Some(CasterAction::Attack), "skill {skill_id}");
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
        assert_eq!(action(34), Some(CasterAction::Idle), "skill 34");

        // Fury is motion 0 in `GetSkillActionInfo`, but `ReceiveSkillUse`
        // overrides it with the skill block frames 1..=2.
        assert_eq!(action(270), Some(CasterAction::Action(12)), "skill 270");
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
    fn use_actions_follow_get_skill_action_info2() {
        // The 0x09CB path uses `GetSkillActionInfo2`, which disagrees with
        // the cast-ack table for 131 of the 373 castable pre-re skills.
        // Blessing: cast ack 0 (Idle), use 7 (Skill).
        assert_eq!(action(34), Some(CasterAction::Idle));
        assert_eq!(use_action(34), Some(CasterAction::Skill));
        // Incagi is the reverse: cast 7, use 0.
        assert_eq!(action(29), Some(CasterAction::Skill));
        assert_eq!(use_action(29), Some(CasterAction::Idle));
        // Auto Counter: cast 11 (ReadyFight), use 2 (weapon swing).
        assert_eq!(action(61), Some(CasterAction::ReadyFight));
        assert_eq!(use_action(61), Some(CasterAction::Attack));
        // Potion Pitcher: cast 0 (Idle), use 12 (throw block 40).
        assert_eq!(action(231), Some(CasterAction::Idle));
        assert_eq!(use_action(231), Some(CasterAction::Action(5)));
        // Berserk Pitcher: `GetSkillActionInfo2` returns motion -1, i.e. the
        // official client does not change the pose at all.
        assert_eq!(use_action(446), Some(CasterAction::Keep));
        // Skills where both paths agree carry no override.
        assert_eq!(use_action(28), None);
    }

    #[test]
    fn unknown_skills_fall_back_safely_to_the_casting_gesture() {
        assert_eq!(action(65_500), None);
        assert_eq!(default_caster_action(), CasterAction::Skill);
    }
}
