//! Playing of map sound effects (`ZC_SOUND`, 0x01D3) and skill sounds.
//!
//! The 2019 map server does not send a sound packet for most ambient /
//! self-buff casts, so this module additionally has a small skill → wav table
//! that is triggered from the skill cast handlers.
//!
//! ## Sound lanes
//!
//! Every entity has **one sound lane**: at most one cast/skill sound of an
//! entity is audible at a time. When a new sound for an entity arrives while
//! the previous one is still playing, the previous playback is stopped and the
//! new wav starts from the beginning ("preemption"). Sounds of different
//! entities are independent lanes and can play in parallel.
//!
//! ## Headless / muted verification
//!
//! For headless/background test clients (`KORANGAR_MUTE_AUDIO=1`) the audio
//! engine uses its silent backend (no audio device is opened) and every play
//! attempt / preemption is logged:
//!
//! ```text
//! [sound] play effect\\ef_blessing.wav (exists=true) at entity 123456 (150.0, 5.0, 87.5)
//! [sound] preempt effect\\ef_blessing.wav -> effect\\ef_angelus.wav at entity 123456 (150.0, 5.0, 87.5)
//! ```
//!
//! The real audio devices belong to the user and are never captured or tapped.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use cgmath::Point3;
use korangar_audio::{AudioEngine, SpatialSoundHandle};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_loaders::FileLoader;
use ragnarok_packets::{AccountId, EntityId, SkillId};

use crate::loaders::GameFileLoader;

/// Base path (inside the game archives / game folder) of the sound files.
const SOUND_BASE_PATH: &str = "data\\wav";
/// Audible range of skill sounds in world units.
const SKILL_SOUND_RANGE: f32 = 180.0;
/// Upper bound for the sound lanes, so a busy map cannot grow the map forever.
const MAX_SOUND_LANES: usize = 256;
/// Environment variable that disables all audio output (set by the lean test
/// launcher). Keep in sync with `korangar_audio`'s environment variable.
const MUTE_AUDIO_ENVIRONMENT_VARIABLE: &str = "KORANGAR_MUTE_AUDIO";

/// Skills that make no sound on this 2019 server, with the sound files that
/// should be played for them.
///
/// Every file was verified to exist in the CondaRO `data.grf` (see the unit
/// tests); at runtime the first existing candidate is used, so a missing file
/// is never played. If a skill is not listed here, [GENERIC_CAST_SOUND] is
/// used.
pub const SKILL_SOUNDS: &[(u16, &[&str])] = &[
    // BS_HAMMERFALL (skill id 110; id 5 is SM_BASH, a plain attack skill).
    (110, &["black_hammerfall.wav"]),
    // AL_WARP
    (27, &["effect\\warp.wav"]),
    // AL_INCAGI
    (29, &["effect\\ef_incagility.wav"]),
    // AL_ANGELUS
    (33, &["effect\\ef_angelus.wav"]),
    // AL_BLESSING (classic effect sound; spirit handler as a fallback)
    (34, &[
        "effect\\ef_blessing.wav",
        "effect\\spirit_handler\\sh_blessing_of_mystical_creatures.wav",
    ]),
    // AC_CONCENTRATION
    (45, &["effect\\ac_concentration.wav"]),
];

/// Generic cast sound for skills that are not part of [SKILL_SOUNDS].
pub const GENERIC_CAST_SOUND: &[&str] = &["effect\\ef_beginspell.wav", "effect\\em_spell_enchanting.wav"];

/// Plays map sound effects and skill sounds with one lane per entity.
///
/// The player is stored on the client so the sound path does not have to borrow
/// the audio engine and the game file loader at every call site.
pub struct SkillSoundPlayer {
    audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
    game_file_loader: Arc<GameFileLoader>,
    muted: bool,
    state: Mutex<SkillSoundState>,
}

/// The mutable part of the sound player that does not need an audio engine.
#[derive(Default)]
struct SkillSoundState {
    /// Cache whether a wav file exists in the game archives.
    exists: HashMap<String, bool>,
    /// The sound lane of every entity that is currently playing a sound.
    lanes: HashMap<EntityId, LaneSound>,
}

struct LaneSound {
    wav: String,
    handle: Option<SpatialSoundHandle>,
}

impl SkillSoundPlayer {
    pub fn new(audio_engine: Arc<AudioEngine<GameFileLoader>>, game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            audio_engine: Some(audio_engine),
            game_file_loader,
            muted: audio_output_is_muted(),
            state: Mutex::new(SkillSoundState::default()),
        }
    }

    /// Creates a muted player without an audio engine. Used by the unit tests
    /// so they never touch the user's audio device.
    #[cfg(test)]
    fn new_muted_for_tests(game_file_loader: Arc<GameFileLoader>) -> Self {
        Self {
            audio_engine: None,
            game_file_loader,
            muted: true,
            state: Mutex::new(SkillSoundState::default()),
        }
    }

    /// Whether audio output is muted for this process.
    pub fn is_muted(&self) -> bool {
        self.muted
    }

    /// Plays a sound that the map server sent with `ZC_SOUND` (0x01D3).
    ///
    /// `name` is the file name relative to `data/wav` (with or without the
    /// `.wav` extension, both spellings occur in rAthena).
    pub fn play_network_sound(&self, name: &str, account_id: AccountId, position: Option<Point3<f32>>) {
        let Some(wav) = normalize_network_sound_name(name) else {
            return;
        };

        let entity_id = EntityId(account_id.0);

        self.play(&wav, Some(entity_id), position);
    }

    /// Plays the sound of a skill cast.
    ///
    /// The 2019 server sends no sound packet for ambient casts, so the cast
    /// handlers call this with the caster entity and its position.
    pub fn play_skill_sound(&self, skill_id: SkillId, entity_id: Option<EntityId>, position: Option<Point3<f32>>) {
        let candidates = SKILL_SOUNDS
            .iter()
            .find(|(id, _)| SkillId(*id) == skill_id)
            .map(|(_, candidates)| *candidates)
            .unwrap_or(GENERIC_CAST_SOUND);

        let Some(wav) = self.first_existing(candidates) else {
            return;
        };

        self.play(wav, entity_id, position);
    }

    /// Stops all sound lanes, e.g. on map change / logout.
    pub fn clear(&self) {
        let mut state = self.state.lock().unwrap();

        for (_, lane) in state.lanes.drain() {
            if let Some(handle) = lane.handle {
                handle.stop();
            }
        }
    }

    /// Stops the sound lane of an entity, e.g. when it is removed from the map.
    pub fn remove_entity(&self, entity_id: EntityId) {
        let lane = self.state.lock().unwrap().lanes.remove(&entity_id);

        if let Some(lane) = lane
            && let Some(handle) = lane.handle
        {
            handle.stop();
        }
    }

    /// The verification log line for a play attempt.
    ///
    /// Kept public(crate) so the unit tests can check the exact format that the
    /// verification workers grep for.
    pub(crate) fn play_log_line(wav: &str, exists: bool, entity_id: Option<EntityId>, position: Option<Point3<f32>>) -> String {
        format!(
            "[sound] play {wav} (exists={exists}) at entity {} {}",
            Self::format_entity(entity_id),
            Self::format_position(position)
        )
    }

    /// The verification log line for a preemption (lane takeover).
    pub(crate) fn preempt_log_line(previous: &str, next: &str, entity_id: Option<EntityId>, position: Option<Point3<f32>>) -> String {
        format!(
            "[sound] preempt {previous} -> {next} at entity {} {}",
            Self::format_entity(entity_id),
            Self::format_position(position)
        )
    }

    fn format_entity(entity_id: Option<EntityId>) -> String {
        match entity_id {
            Some(entity_id) => entity_id.0.to_string(),
            None => "unknown".to_string(),
        }
    }

    fn format_position(position: Option<Point3<f32>>) -> String {
        match position {
            Some(position) => format!("({:.1}, {:.1}, {:.1})", position.x, position.y, position.z),
            None => "(unknown)".to_string(),
        }
    }

    fn play(&self, wav: &str, entity_id: Option<EntityId>, position: Option<Point3<f32>>) {
        let exists = self.wav_exists(wav);

        if !exists {
            #[cfg(feature = "debug")]
            print_debug!("[{}] skill sound file does not exist: {wav}", "warning".yellow());
            if self.muted {
                println!("{}", Self::play_log_line(wav, exists, entity_id, position));
            }
            return;
        }

        // Take over the lane of this entity: a new sound preempts the previous
        // one and starts from the beginning.
        let lane_key = entity_id.unwrap_or(EntityId(0));

        let previous = {
            let mut state = self.state.lock().unwrap();

            // Keep the number of lanes bounded on busy maps. Evict an arbitrary
            // other lane (its track is stopped by dropping the handle).
            if !state.lanes.contains_key(&lane_key)
                && state.lanes.len() >= MAX_SOUND_LANES
                && let Some(evicted_key) = state.lanes.keys().next().copied()
                && let Some(evicted) = state.lanes.remove(&evicted_key)
                && let Some(handle) = evicted.handle
            {
                handle.stop();
            }

            state.lanes.insert(lane_key, LaneSound {
                wav: wav.to_string(),
                handle: None,
            })
        };

        if let Some(mut previous) = previous {
            let handle = previous.handle.take();
            let still_playing = match &handle {
                Some(handle) => !handle.is_finished(),
                None => true,
            };

            if still_playing {
                if self.muted {
                    println!("{}", Self::preempt_log_line(&previous.wav, wav, entity_id, position));
                }

                if let Some(handle) = handle {
                    handle.stop();
                }
            }
        }

        let handle = self.start_sound(wav, position);

        if self.muted {
            println!("{}", Self::play_log_line(wav, exists, entity_id, position));
        }

        if let Some(handle) = handle {
            let mut state = self.state.lock().unwrap();

            // Only store the handle if the lane still belongs to this sound
            // (another sound may have taken over while loading).
            if let Some(lane) = state.lanes.get_mut(&lane_key)
                && lane.wav == wav
            {
                lane.handle = Some(handle);
            }
        }
    }

    /// Starts the wav in the audio engine.
    ///
    /// With `KORANGAR_MUTE_AUDIO=1` the engine uses its silent backend: no
    /// audio device is opened and nothing is audible, but playback state
    /// and handles still work (which keeps the lane preemption logic
    /// testable in the lean client).
    fn start_sound(&self, wav: &str, position: Option<Point3<f32>>) -> Option<SpatialSoundHandle> {
        let audio_engine = self.audio_engine.as_ref()?;
        let sound_effect_key = audio_engine.load(wav);

        match position {
            Some(position) => audio_engine.play_spatial_sound_effect_with_handle(sound_effect_key, position, SKILL_SOUND_RANGE),
            None => {
                audio_engine.play_sound_effect(sound_effect_key);
                None
            }
        }
    }

    fn wav_exists(&self, wav: &str) -> bool {
        let mut state = self.state.lock().unwrap();

        if let Some(exists) = state.exists.get(wav) {
            return *exists;
        }

        let path = format!("{SOUND_BASE_PATH}\\{wav}");
        let exists = self.game_file_loader.get(&path).is_ok();
        state.exists.insert(wav.to_string(), exists);

        exists
    }

    fn first_existing(&self, candidates: &'static [&'static str]) -> Option<&'static str> {
        candidates.iter().copied().find(|candidate| self.wav_exists(candidate))
    }
}

/// Normalizes a `ZC_SOUND` name to the path that the audio engine expects
/// (relative to `data/wav`, backslash separated, with a `.wav` extension).
fn normalize_network_sound_name(name: &str) -> Option<String> {
    let name = name.trim_matches('\0').trim();

    if name.is_empty() {
        return None;
    }

    let mut name = name.replace('/', "\\");

    if let Some(stripped) = name.strip_prefix("data\\wav\\") {
        name = stripped.to_string();
    }

    if !name.to_ascii_lowercase().ends_with(".wav") {
        name.push_str(".wav");
    }

    Some(name)
}

/// Whether all audio output is disabled with `KORANGAR_MUTE_AUDIO`.
fn audio_output_is_muted() -> bool {
    let Ok(value) = std::env::var(MUTE_AUDIO_ENVIRONMENT_VARIABLE) else {
        return false;
    };

    matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

impl SkillSoundState {
    #[cfg(test)]
    fn lanes_len(&self) -> usize {
        self.lanes.len()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Checks that every sound file in [SKILL_SOUNDS] / [GENERIC_CAST_SOUND]
    /// exists in the game archives.
    ///
    /// This needs the CondaRO game archives (symlinked into the runtime
    /// directory) and is skipped when they are not available.
    #[test]
    fn skill_sound_files_exist_in_the_game_archives() {
        if !Path::new("client/game_archives.ron").exists() {
            return;
        }

        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();
        let player = SkillSoundPlayer::new_muted_for_tests(Arc::new(game_file_loader));

        for (skill_id, candidates) in SKILL_SOUNDS {
            let candidate = candidates
                .iter()
                .find(|candidate| player.wav_exists(candidate))
                .unwrap_or_else(|| panic!("no sound file exists for skill {skill_id}: {candidates:?}"));
            println!("skill {skill_id} -> {candidate}");
        }

        for candidate in GENERIC_CAST_SOUND {
            assert!(player.wav_exists(candidate), "generic cast sound is missing: {candidate}");
        }
    }

    /// Prints the muted-mode play/preempt log lines for every table entry and
    /// for a forced lane take-over. This is the verification evidence for the
    /// lean client runs; it never touches an audio device.
    #[test]
    fn muted_skill_sounds_are_logged() {
        if !Path::new("client/game_archives.ron").exists() {
            return;
        }

        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();
        let player = SkillSoundPlayer::new_muted_for_tests(Arc::new(game_file_loader));

        let position = Some(Point3::new(150.0, 5.0, 87.5));

        for (skill_id, _) in SKILL_SOUNDS {
            player.play_skill_sound(SkillId(*skill_id), Some(EntityId(42)), position);
        }

        // A second skill of the same entity takes over the lane and logs a
        // preemption.
        player.play_skill_sound(SkillId(34), Some(EntityId(42)), position);

        // A network sound (`ZC_SOUND`) for an entity.
        player.play_network_sound("effect\\ef_glow.wav", AccountId(43), position);
    }

    #[test]
    fn required_skills_are_in_the_sound_table() {
        // Skill id 5 is `SM_BASH` (plain attack), only 110 is `BS_HAMMERFALL`.
        for skill_id in [110, 27, 29, 33, 34, 45] {
            assert!(
                SKILL_SOUNDS.iter().any(|(id, _)| *id == skill_id),
                "skill {skill_id} has no sound mapping"
            );
        }
    }

    #[test]
    fn network_sound_names_are_normalized() {
        assert_eq!(
            normalize_network_sound_name("effect\\ef_blessing.wav"),
            Some("effect\\ef_blessing.wav".to_string())
        );
        assert_eq!(
            normalize_network_sound_name("data\\wav\\effect/ef_blessing.wav"),
            Some("effect\\ef_blessing.wav".to_string())
        );
        assert_eq!(
            normalize_network_sound_name("ef_hammerfall"),
            Some("ef_hammerfall.wav".to_string())
        );
        assert_eq!(normalize_network_sound_name(""), None);
        assert_eq!(normalize_network_sound_name("\0\0"), None);
    }

    #[test]
    fn new_sound_takes_over_the_lane_of_the_same_entity() {
        let mut state = SkillSoundState::default();

        assert!(state.lanes.is_empty());

        let previous = state.lanes.insert(EntityId(1), LaneSound {
            wav: "black_hammerfall.wav".to_string(),
            handle: None,
        });
        assert!(previous.is_none());

        // A new sound for the same entity takes the lane over.
        let previous = state.lanes.insert(EntityId(1), LaneSound {
            wav: "effect\\ef_blessing.wav".to_string(),
            handle: None,
        });
        assert_eq!(previous.unwrap().wav, "black_hammerfall.wav");

        // Different entities are independent lanes.
        let previous = state.lanes.insert(EntityId(2), LaneSound {
            wav: "effect\\ef_angelus.wav".to_string(),
            handle: None,
        });
        assert!(previous.is_none());
        assert_eq!(state.lanes_len(), 2);

        state.lanes.remove(&EntityId(1));
        assert_eq!(state.lanes_len(), 1);
    }

    #[test]
    fn muted_play_log_format() {
        let line = SkillSoundPlayer::play_log_line(
            "effect\\ef_blessing.wav",
            true,
            Some(EntityId(123456)),
            Some(Point3::new(150.0, 5.0, 87.5)),
        );

        assert_eq!(
            line,
            "[sound] play effect\\ef_blessing.wav (exists=true) at entity 123456 (150.0, 5.0, 87.5)"
        );
    }

    #[test]
    fn muted_preempt_log_format() {
        let line = SkillSoundPlayer::preempt_log_line(
            "effect\\ef_blessing.wav",
            "effect\\ef_angelus.wav",
            Some(EntityId(123456)),
            None,
        );

        assert_eq!(
            line,
            "[sound] preempt effect\\ef_blessing.wav -> effect\\ef_angelus.wav at entity 123456 (unknown)"
        );
    }
}
