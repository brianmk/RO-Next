//! Official skill -> cast sound mapping (`data\client\skill_sounds.ron`).
//!
//! The official client plays the `wav` of the effect a skill spawns. The 2019
//! map server rarely sends a sound packet for casts, so the client has to play
//! the skill sound itself. This module loads the generated per-skill sound
//! table from the client archive; a server can override the map without
//! rebuilding the client, and the compiled fallback embeds the same data file.
//!
//! Generation: `tools/gen_skill_sounds.py` from the facts extracted by
//! `tools/extract_skill_sounds.py` (roBrowserLegacy RE dump of the Windows
//! client effect table, wavs validated against the CondaRO GRFs). `--check` is
//! the drift guard.

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::SkillId;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_sounds.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_sounds.ron");

#[derive(Deserialize)]
struct SkillSoundFile {
    #[allow(dead_code)]
    provenance: String,
    sounds: Vec<SkillSoundEntry>,
}

#[derive(Deserialize)]
struct SkillSoundEntry {
    skill: u16,
    files: Vec<String>,
}

/// One skill with its wav candidates (relative to `data\wav`, in play priority
/// order).
struct StoredSound {
    skill: u16,
    files: &'static [&'static str],
}

impl StoredSound {
    fn new(entry: SkillSoundEntry) -> Self {
        let files: Vec<&'static str> = entry
            .files
            .into_iter()
            .map(|file| Box::leak(file.into_boxed_str()) as &'static str)
            .collect();
        Self {
            skill: entry.skill,
            files: Box::leak(files.into_boxed_slice()),
        }
    }
}

/// The compiled fallback mapping, parsed on first use (also used by tests).
static COMPILED: OnceLock<Vec<StoredSound>> = OnceLock::new();
/// Mapping loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<Vec<StoredSound>> = OnceLock::new();

fn compiled() -> &'static [StoredSound] {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(sounds) => sounds,
        Err(error) => {
            crate::logging::log_warn!("skill sounds: compiled fallback is invalid ({error})");
            Vec::new()
        }
    })
}

fn mapping() -> &'static [StoredSound] {
    ARCHIVE.get().map(|sounds| sounds.as_slice()).unwrap_or_else(|| compiled())
}

fn parse(content: &str) -> Result<Vec<StoredSound>, String> {
    let file: SkillSoundFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut sounds: Vec<StoredSound> = file.sounds.into_iter().map(StoredSound::new).collect();
    sounds.sort_by_key(|sound| sound.skill);
    sounds.dedup_by_key(|sound| sound.skill);
    Ok(sounds)
}

/// Loads the sound mapping from the client archive (or the compiled fallback).
///
/// Returns the number of loaded skills. A malformed data file is reported and
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

    let sounds = match parse(content) {
        Ok(sounds) => sounds,
        Err(error) => {
            crate::logging::log_warn!("skill sounds: invalid {DATA_PATH} ({error}), using the compiled fallback");
            match parse(FALLBACK) {
                Ok(sounds) => sounds,
                Err(fallback_error) => {
                    crate::logging::log_warn!("skill sounds: compiled fallback is invalid too ({fallback_error})");
                    Vec::new()
                }
            }
        }
    };

    let count = sounds.len();
    let _ = ARCHIVE.set(sounds);
    crate::logging::log_debug!("skill sounds: loaded {count} entries");
    count
}

/// Returns the official wav candidates of `skill_id` in play priority order, or
/// `None` when the skill has no generated sound mapping.
pub(crate) fn skill_sound_files(skill_id: SkillId) -> Option<&'static [&'static str]> {
    let sounds = mapping();
    sounds
        .binary_search_by_key(&skill_id.0, |sound| sound.skill)
        .ok()
        .map(|index| sounds[index].files)
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{FALLBACK, parse};

    fn files_for(skill_id: u16) -> Option<Vec<&'static str>> {
        super::skill_sound_files(SkillId(skill_id)).map(|files| files.to_vec())
    }

    #[test]
    fn data_file_is_valid_and_broad() {
        let sounds = parse(FALLBACK).expect("valid fallback");
        assert!(sounds.len() >= 300, "sound mapping should stay broad, got {}", sounds.len());
        assert!(
            sounds.windows(2).all(|pair| pair[0].skill < pair[1].skill),
            "rows must be sorted and unique"
        );
    }

    #[test]
    fn known_skills_have_their_official_sounds() {
        // Heal: the official `_heal_effect.wav` (at the root of data\wav).
        assert_eq!(files_for(28).as_deref().map(|files| files[0]), Some("_heal_effect.wav"));
        // Storm Gust and Magnum Break have skill-specific sounds.
        assert!(files_for(89).is_some_and(|files| files[0].contains("wizard_stormgust")));
        assert!(files_for(7).is_some_and(|files| files[0].contains("ef_magnumbreak")));
    }

    #[test]
    fn gloria_uses_the_official_gloria_effect_sound() {
        // Official tables: `SkillEffect[SK.PR_GLORIA] = { effectId: 75 }`,
        // `EffectTable[75]` is the STR effect `gloria.str` with
        // `wav: 'effect/priest_gloria'`. Gloria is an instant skill, so this
        // sound has to play once at completion and never as a pre-cast sound
        // (timing is covered by `SkillSoundGate` tests in `world::skill_sound`).
        assert_eq!(
            files_for(75).as_deref().map(|files| files[0]),
            Some("effect\\priest_gloria.wav"),
            "Gloria (PR_GLORIA, id 75) must use the official priest_gloria.wav"
        );
        // AL_CURE (id 35) is a different skill and keeps its own sound; the
        // Gloria facts must never leak onto it.
        assert_eq!(files_for(35).as_deref().map(|files| files[0]), Some("effect\\acolyte_cure.wav"));
    }

    #[test]
    fn instant_buff_sounds_are_mapped() {
        // Blessing, Increase AGI, Angelus and Concentration are the instant
        // buffs used for the regression checks; each must have an official wav.
        for skill_id in [34, 29, 33, 45] {
            assert!(
                files_for(skill_id).is_some_and(|files| !files.is_empty()),
                "skill {skill_id} has no sound"
            );
        }
    }

    #[test]
    fn distinctive_sounds_win_over_generic_hit_sounds() {
        // Fire Ball's effect layers include the generic `ef_hit2` impact sound;
        // the distinctive fire ball sounds must be preferred.
        let files = files_for(17).expect("fire ball sound");
        assert!(!files[0].contains("ef_hit2"), "generic hit sound was preferred: {files:?}");
        assert!(files.iter().any(|file| file.contains("ef_firehit") || file.contains("ef_fireball")));
    }

    #[test]
    fn every_candidate_is_a_wav_path() {
        let sounds = parse(FALLBACK).expect("valid fallback");
        for sound in &sounds {
            assert!(!sound.files.is_empty(), "skill {} has no candidates", sound.skill);
            for file in sound.files {
                assert!(
                    file.to_ascii_lowercase().ends_with(".wav"),
                    "skill {}: not a wav: {file}",
                    sound.skill
                );
                assert!(
                    !file.starts_with("data\\wav\\"),
                    "skill {}: path must be relative: {file}",
                    sound.skill
                );
            }
        }
    }

    #[test]
    fn unknown_skills_have_no_generated_sound() {
        assert!(super::skill_sound_files(SkillId(65_500)).is_none());
    }
}
