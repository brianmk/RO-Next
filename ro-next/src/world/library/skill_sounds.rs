//! Skill -> cast sound mapping (`data\client\skill_sounds.ron`).
//!
//! The official client plays the `wav` of the effect a skill spawns. The 2019
//! map server rarely sends a sound packet for casts, so the client has to play
//! the skill sound itself. This module loads the mapping from the client
//! archive; a server can override the map without rebuilding the client, and
//! the compiled fallback embeds the same data file.
//!
//! The file has three sections that mirror the historical play priority
//! (curated -> per-layer `wav` of `skill_effects.ron` -> official generated ->
//! generic); the priority itself is applied in [`crate::world::skill_sound`]:
//!
//! * `curated` — hand-written, archive-verified overrides
//!   (`tools/data/skill_sounds_custom.ron`, merged by the generator),
//! * `sounds` — the official per-skill table generated from the effect table,
//! * `generic` — the fallback cast sound of skills without any mapping.
//!
//! Generation: `tools/gen_skill_sounds.py` from the facts extracted by
//! `tools/extract_skill_sounds.py` (roBrowserLegacy RE dump of the Windows
//! client effect table, wavs validated against the CondaRO GRFs). `--check` is
//! the drift guard.

use std::sync::OnceLock;

use ragnarok_packets::SkillId;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_sounds.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_sounds.ron");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillSoundFile {
    #[allow(dead_code)]
    provenance: String,
    #[serde(default)]
    generic: Vec<String>,
    #[serde(default)]
    curated: Vec<SkillSoundEntry>,
    #[serde(default)]
    sounds: Vec<SkillSoundEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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

/// The loaded sound mapping (curated overrides, official table, generic
/// fallback).
struct SoundRegistry {
    /// Sorted by skill id.
    curated: Vec<StoredSound>,
    /// Sorted by skill id.
    generated: Vec<StoredSound>,
    generic: &'static [&'static str],
}

impl SoundRegistry {
    fn empty() -> Self {
        Self {
            curated: Vec::new(),
            generated: Vec::new(),
            generic: &[],
        }
    }
}

/// The compiled fallback mapping, parsed on first use (also used by tests).
static COMPILED: OnceLock<SoundRegistry> = OnceLock::new();
/// Mapping loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<SoundRegistry> = OnceLock::new();

fn compiled() -> &'static SoundRegistry {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(registry) => registry,
        Err(error) => {
            crate::logging::log_warn!("skill sounds: compiled fallback is invalid ({error})");
            SoundRegistry::empty()
        }
    })
}

fn registry() -> &'static SoundRegistry {
    ARCHIVE.get().unwrap_or_else(compiled)
}

fn validate_file(value: &str, context: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{context}: empty wav path"));
    }
    if !value.to_ascii_lowercase().ends_with(".wav") {
        return Err(format!("{context}: '{value}' is not a .wav path"));
    }
    if value.to_ascii_lowercase().starts_with("data\\wav\\") {
        return Err(format!("{context}: '{value}' has to be relative to data\\wav"));
    }
    Ok(())
}

fn build_table(mut entries: Vec<SkillSoundEntry>, section: &str) -> Result<Vec<StoredSound>, String> {
    entries.sort_by_key(|entry| entry.skill);
    for pair in entries.windows(2) {
        if pair[0].skill == pair[1].skill {
            return Err(format!("{section}: duplicate sound row for skill {}", pair[0].skill));
        }
    }
    for entry in &entries {
        if entry.skill == 0 {
            return Err(format!("{section}: a sound row uses skill id 0"));
        }
        if entry.files.is_empty() {
            return Err(format!("{section}: skill {} has no wav candidates", entry.skill));
        }
        for file in &entry.files {
            validate_file(file, &format!("{section}: skill {}", entry.skill))?;
        }
    }
    Ok(entries.into_iter().map(StoredSound::new).collect())
}

fn parse(content: &str) -> Result<SoundRegistry, String> {
    let file: SkillSoundFile = ron::from_str(content).map_err(|error| error.to_string())?;

    if file.generic.is_empty() {
        return Err("the data file has no generic cast sound".to_owned());
    }
    for file in &file.generic {
        validate_file(file, "generic")?;
    }
    let generic: &'static [&'static str] = Box::leak(
        file.generic
            .into_iter()
            .map(|file| Box::leak(file.into_boxed_str()) as &'static str)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );

    if file.curated.is_empty() {
        return Err("the data file has no curated sounds".to_owned());
    }
    let curated = build_table(file.curated, "curated")?;
    let generated = build_table(file.sounds, "sounds")?;
    if let Some(overlap) = curated
        .iter()
        .find(|curated| generated.binary_search_by_key(&curated.skill, |sound| sound.skill).is_ok())
    {
        return Err(format!(
            "skill {} appears in both the curated and the generated sound table",
            overlap.skill
        ));
    }

    Ok(SoundRegistry {
        curated,
        generated,
        generic,
    })
}

/// Loads the sound mapping from the client archive (or the compiled fallback).
///
/// Returns the number of loaded skills (curated + generated). A malformed data
/// file is reported and the compiled fallback is used instead, so a bad
/// override can never leave the client without a usable mapping.
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
                    SoundRegistry::empty()
                }
            }
        }
    };

    let count = sounds.curated.len() + sounds.generated.len();
    let _ = ARCHIVE.set(sounds);
    crate::logging::log_debug!("skill sounds: loaded {count} entries");
    count
}

/// Returns the hand-curated wav candidates of `skill_id` in play priority
/// order, or `None` when the data file has no curated override for the skill.
pub(crate) fn curated_skill_sound_files(skill_id: SkillId) -> Option<&'static [&'static str]> {
    let sounds = &registry().curated;
    sounds
        .binary_search_by_key(&skill_id.0, |sound| sound.skill)
        .ok()
        .map(|index| sounds[index].files)
}

/// Returns the official generated wav candidates of `skill_id` in play priority
/// order, or `None` when the skill has no generated sound mapping.
pub(crate) fn skill_sound_files(skill_id: SkillId) -> Option<&'static [&'static str]> {
    let sounds = &registry().generated;
    sounds
        .binary_search_by_key(&skill_id.0, |sound| sound.skill)
        .ok()
        .map(|index| sounds[index].files)
}

/// The generic cast fallback of skills without any curated/generated mapping.
pub(crate) fn generic_cast_sound_files() -> &'static [&'static str] {
    registry().generic
}

/// Skills that have a hand-curated override (used by the tests and the muted
/// verification run).
#[cfg(test)]
pub(crate) fn curated_skill_ids() -> Vec<u16> {
    registry().curated.iter().map(|sound| sound.skill).collect()
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
        assert!(
            sounds.generated.len() >= 300,
            "sound mapping should stay broad, got {}",
            sounds.generated.len()
        );
        assert_eq!(sounds.curated.len(), 10, "the hand-curated overrides");
        assert!(!sounds.generic.is_empty());
        assert!(
            sounds.generated.windows(2).all(|pair| pair[0].skill < pair[1].skill),
            "rows must be sorted and unique"
        );
        assert!(
            sounds.curated.windows(2).all(|pair| pair[0].skill < pair[1].skill),
            "curated rows must be sorted and unique"
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
    fn blitz_beat_keeps_its_distinctive_first_wave() {
        // `CRagEffect::BlitzBeat` plays `effect\hunter_blitzbeat_1st.wav` on
        // frame 5 and `effect\hunter_blitzbeat.wav` on frame 30
        // (`RagEffect.cpp:11631-11638`). Falcon Assault's
        // `EF_FALCONASSAULT` body plays no wave (`RagEffect.cpp:23782-23797`);
        // its only sound is the effect-table wave `hunter_blitzbeat`
        // (`tools/data/windows_client_skill_sounds.json`, skill 381). Blitz
        // Beat must therefore keep the first wave as its skill sound, or both
        // falcon skills play the same wave.
        assert_eq!(
            super::curated_skill_sound_files(SkillId(129)).map(|files| files.to_vec()),
            Some(vec!["effect\\hunter_blitzbeat_1st.wav"])
        );
        assert_eq!(
            files_for(381).as_deref().map(|files| files[0]),
            Some("effect\\hunter_blitzbeat.wav")
        );
        assert_ne!(
            files_for(129).as_deref().map(|files| files[0]),
            files_for(381).as_deref().map(|files| files[0]),
            "Blitz Beat must not play Falcon Assault's wave"
        );
    }

    #[test]
    fn steel_body_plays_the_official_skill_switch_sound() {
        // `CGameMode::ReceiveSkillUse`'s `case MO_STEELBODY:` plays
        // `effect\mon_맹룡과강.wav` (`GameModePackets.cpp:18845`) next to
        // `EF_GUMGANG2` (261) and the 20 ms block-96 frame hold. The
        // effect-table entry of `EF_STEELBODY` (254) names `mon_금강불괴`, but
        // the decompiled effect body of 254 (`RagEffect.cpp:24181-24183`) never
        // plays a wave, so the skill switch's wav is the official one and the
        // cur/generated table must keep it (no curated override).
        assert_eq!(files_for(268).as_deref(), Some(&["effect\\mon_맹룡과강.wav"][..]));
        assert!(super::curated_skill_sound_files(SkillId(268)).is_none());
    }

    #[test]
    fn instant_buff_sounds_are_mapped() {
        // Blessing, Increase AGI, Angelus and Concentration are the instant
        // buffs used for the regression checks; each must have a curated wav.
        for skill_id in [34, 29, 33, 45] {
            assert!(
                super::curated_skill_sound_files(SkillId(skill_id)).is_some_and(|files| !files.is_empty()),
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
        let mut tables = Vec::new();
        tables.extend(sounds.curated.iter().map(|sound| (sound.skill, sound.files)));
        tables.extend(sounds.generated.iter().map(|sound| (sound.skill, sound.files)));
        for (skill, files) in tables {
            assert!(!files.is_empty(), "skill {skill} has no candidates");
            for file in files {
                assert!(file.to_ascii_lowercase().ends_with(".wav"), "skill {skill}: not a wav: {file}");
                assert!(!file.starts_with("data\\wav\\"), "skill {skill}: path must be relative: {file}");
            }
        }
        for file in sounds.generic {
            assert!(file.to_ascii_lowercase().ends_with(".wav"), "generic: not a wav: {file}");
        }
    }

    #[test]
    fn unknown_skills_have_no_generated_sound() {
        assert!(super::skill_sound_files(SkillId(65_500)).is_none());
        assert!(super::curated_skill_sound_files(SkillId(65_500)).is_none());
    }

    #[test]
    fn curated_rows_are_not_duplicated_in_the_generated_table() {
        let sounds = parse(FALLBACK).expect("valid fallback");
        for curated in &sounds.curated {
            assert!(
                sounds.generated.binary_search_by_key(&curated.skill, |sound| sound.skill).is_err(),
                "skill {} is in both tables",
                curated.skill
            );
        }
    }

    /// `MO_INVESTIGATE` / Occult Impaction (266) plays the short monk impact
    /// wave. The 2019 CondaRO client used `effect\mon_탄지신통.wav` for the
    /// `EF_CHIMTO` body; the 2026 decompilation changed that body to the long
    /// `effect\mon_침투경.wav` (`RagEffect.cpp:14427`), which is the Solar
    /// Explosion boom (122 756 uncompressed bytes against 11 430 for the
    /// impact wave). The curated row keeps the 2019 client's sound.
    #[test]
    fn investigate_plays_the_short_2019_monk_impact_wave() {
        assert_eq!(
            super::curated_skill_sound_files(SkillId(266)).map(|files| files.to_vec()),
            Some(vec!["effect\\mon_탄지신통.wav"]),
            "MO_INVESTIGATE (266) must play the 2019 monk impact wave"
        );
        assert!(
            super::skill_sound_files(SkillId(266)).is_none(),
            "the generated 2026 `mon_침투경` row must not resurface"
        );
        // Throw Spirit Sphere uses the same short wave in both clients.
        assert_eq!(
            super::skill_sound_files(SkillId(267)),
            Some(&["effect\\mon_탄지신통.wav", "effect\\ef_hit2.wav"][..])
        );
    }
}
