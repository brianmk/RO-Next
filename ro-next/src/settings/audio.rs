#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, print_debug};
use ro_next_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Deserializer, Serialize};

/// Maximum volume percentage that can be stored in the settings.
pub const MAX_VOLUME_PERCENT: u8 = 100;

/// The default background music volume.
///
/// The old settings used a fixed `BackgroundMusicVolume` drop-down whose
/// default was 50 %, so keeping that default means that existing players do not
/// suddenly get louder (or quieter) music.
const DEFAULT_BACKGROUND_MUSIC_VOLUME_PERCENT: u8 = 50;

/// The default volume of the master and sound effect sliders.
const DEFAULT_FULL_VOLUME_PERCENT: u8 = 100;

/// Whether the given settings file was written before the volume sliders were
/// introduced.
///
/// Old files store the music volume as `LegacyBackgroundMusicVolume` and have
/// no master/SFX fields; the current format always contains the master volume.
fn is_legacy_settings_file(data: &str) -> bool {
    !data.contains("master_volume")
}

/// Clamp a volume percentage into the valid `0..=100` range.
pub fn clamp_volume_percent(percent: u8) -> u8 {
    percent.min(MAX_VOLUME_PERCENT)
}

/// Convert a volume percentage into a linear gain in `0.0..=1.0`.
///
/// The audio engine expects linear gains; `100 %` is a gain of `1.0` and `0 %`
/// is silence. The conversion is intentionally linear (not a perceptual dB
/// curve) so the engine's `linear_to_decibel` produces the expected gain.
pub fn volume_percent_to_linear(percent: u8) -> f32 {
    clamp_volume_percent(percent) as f32 / MAX_VOLUME_PERCENT as f32
}

fn default_full_volume_percent() -> u8 {
    DEFAULT_FULL_VOLUME_PERCENT
}

fn default_background_music_volume_value() -> BackgroundMusicVolumeValue {
    BackgroundMusicVolumeValue::default()
}

fn default_true() -> bool {
    true
}

/// The legacy background music volume values of older settings files.
///
/// The audio settings used a drop-down with these variants before the volume
/// sliders were introduced. They are still deserialized so an existing
/// `audio_settings.ron` keeps its music volume after the update.
#[derive(Deserialize)]
enum LegacyBackgroundMusicVolume {
    Ten,
    Twenty,
    Thirty,
    Forty,
    Fifty,
    Sixty,
    Seventy,
    Eighty,
    Ninety,
    Hundred,
}

impl LegacyBackgroundMusicVolume {
    fn percent(&self) -> u8 {
        match self {
            Self::Ten => 10,
            Self::Twenty => 20,
            Self::Thirty => 30,
            Self::Forty => 40,
            Self::Fifty => 50,
            Self::Sixty => 60,
            Self::Seventy => 70,
            Self::Eighty => 80,
            Self::Ninety => 90,
            Self::Hundred => 100,
        }
    }
}

/// A background music volume that can be loaded from both the old enum based
/// value (`Hundred`), a percentage (`100`) and a linear volume (`1.0`).
///
/// The legacy enum is tried first, then the percentage and finally the linear
/// value. RON is self-describing, so this untagged representation accepts all
/// three forms that can appear in an `audio_settings.ron`.
#[derive(Deserialize)]
#[serde(untagged)]
enum BackgroundMusicVolumeValueRaw {
    Legacy(LegacyBackgroundMusicVolume),
    Percent(u8),
    Linear(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BackgroundMusicVolumeValue(u8);

impl Default for BackgroundMusicVolumeValue {
    fn default() -> Self {
        Self(DEFAULT_BACKGROUND_MUSIC_VOLUME_PERCENT)
    }
}

impl<'de> Deserialize<'de> for BackgroundMusicVolumeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = match BackgroundMusicVolumeValueRaw::deserialize(deserializer)? {
            BackgroundMusicVolumeValueRaw::Legacy(legacy) => legacy.percent(),
            BackgroundMusicVolumeValueRaw::Percent(percent) => clamp_volume_percent(percent),
            BackgroundMusicVolumeValueRaw::Linear(linear) => {
                let percent = if !linear.is_finite() {
                    0.0
                } else if (0.0..=1.0).contains(&linear) {
                    (linear * 100.0).round()
                } else {
                    linear.round()
                };

                clamp_volume_percent(percent.clamp(0.0, 100.0) as u8)
            }
        };

        Ok(Self(value))
    }
}

/// The audio settings of the client.
///
/// Volumes are stored as percentages (`u8` in `0..=100`) because that is the
/// unit the sliders edit and the settings file saves; the linear gains that the
/// audio engine expects are derived from them. All fields are `serde(default)`
/// so settings files written by older client versions keep loading.
#[derive(Debug, Clone, PartialEq, Serialize, RustState, StateElement)]
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
    /// Whether the background music is muted.
    ///
    /// This is separate from [`Self::background_music_volume`] so that the
    /// previous volume is restored when the music is unmuted again. The mute
    /// overrides the music volume; it does not change the stored value.
    pub background_music_muted: bool,
    /// The master volume; controls all sounds.
    ///
    /// `0 %` mutes everything, `100 %` is the full volume. Every other category
    /// volume is multiplied with this one.
    pub master_volume: u8,
    /// The volume of sound effects (including spatial/ambient sounds).
    pub sound_effect_volume: u8,
    /// The volume of the background music.
    pub background_music_volume: u8,
}

impl<'de> Deserialize<'de> for AudioSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawAudioSettings {
            #[serde(default = "default_true")]
            mute_on_focus_loss: bool,
            #[serde(default)]
            background_music_muted: bool,
            #[serde(default = "default_full_volume_percent")]
            master_volume: u8,
            #[serde(default = "default_full_volume_percent")]
            sound_effect_volume: u8,
            #[serde(default = "default_background_music_volume_value")]
            background_music_volume: BackgroundMusicVolumeValue,
        }

        let raw = RawAudioSettings::deserialize(deserializer)?;

        Ok(Self {
            mute_on_focus_loss: raw.mute_on_focus_loss,
            background_music_muted: raw.background_music_muted,
            master_volume: clamp_volume_percent(raw.master_volume),
            sound_effect_volume: clamp_volume_percent(raw.sound_effect_volume),
            background_music_volume: clamp_volume_percent(raw.background_music_volume.0),
        })
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mute_on_focus_loss: true,
            background_music_muted: false,
            master_volume: DEFAULT_FULL_VOLUME_PERCENT,
            sound_effect_volume: DEFAULT_FULL_VOLUME_PERCENT,
            background_music_volume: DEFAULT_BACKGROUND_MUSIC_VOLUME_PERCENT,
        }
    }
}

impl AudioSettings {
    const FILE_NAME: &'static str = "client/audio_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load audio settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        crate::logging::log_debug!("settings: loading audio settings from {}", Self::FILE_NAME);
        let file_content = {
            #[cfg(feature = "debug")]
            print_debug!("loading audio settings from {}", Self::FILE_NAME.magenta());
            std::fs::read_to_string(Self::FILE_NAME).ok()
        };

        let result = file_content.as_deref().and_then(|data| {
            ron::from_str::<Self>(data)
                .ok()
                .map(|settings| (settings, is_legacy_settings_file(data)))
        });

        match &result {
            Some((settings, is_legacy)) => {
                crate::logging::log_debug!("settings: loaded audio settings from {}", Self::FILE_NAME);

                // Old files (drop-down era) are missing the master/SFX fields and
                // store the music volume as an enum ident. Rewrite them in the
                // current format right away so the migration is persisted even
                // if the client is killed before the settings are next saved.
                if *is_legacy {
                    crate::logging::log_debug!("settings: migrating legacy audio settings to the slider format");
                    settings.save();
                }

                Some(settings.clone())
            }
            None => {
                crate::logging::log_warn!(
                    "settings: could not load audio settings from {} (using defaults)",
                    Self::FILE_NAME
                );
                None
            }
        }
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving audio settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save audio settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }

    /// The linear master gain that should be applied to all sounds.
    pub fn master_volume_linear(&self) -> f32 {
        volume_percent_to_linear(self.master_volume)
    }

    /// The linear sound effect gain (without the master gain).
    pub fn sound_effect_volume_linear(&self) -> f32 {
        volume_percent_to_linear(self.sound_effect_volume)
    }

    /// The linear background music gain (without the master gain).
    pub fn background_music_volume_linear(&self) -> f32 {
        volume_percent_to_linear(self.background_music_volume)
    }

    /// The gain of the background music track: the music volume, or silence
    /// while the music is muted.
    ///
    /// The master gain is not included; it is applied on the master track by
    /// the audio engine, which multiplies it with every sub-track.
    pub fn background_music_track_volume(&self) -> f32 {
        match self.background_music_muted {
            true => 0.0,
            false => self.background_music_volume_linear(),
        }
    }

    /// The gain that all sounds effectively play at (master × category).
    pub fn effective_master_volume(&self) -> f32 {
        self.master_volume_linear()
    }

    /// The gain that sound effects effectively play at (master × sound
    /// effects).
    pub fn effective_sound_effect_volume(&self) -> f32 {
        self.master_volume_linear() * self.sound_effect_volume_linear()
    }

    /// The gain that spatial (ambient) sound effects effectively play at.
    ///
    /// Spatial sounds are a kind of sound effect, so they use the same volume.
    pub fn effective_spatial_sound_effect_volume(&self) -> f32 {
        self.master_volume_linear() * self.sound_effect_volume_linear()
    }

    /// The gain that the background music effectively plays at.
    ///
    /// The music mute overrides the music volume (it returns `0.0`) and the
    /// master volume is multiplied in.
    pub fn effective_background_music_volume(&self) -> f32 {
        match self.background_music_muted {
            true => 0.0,
            false => self.master_volume_linear() * self.background_music_volume_linear(),
        }
    }
}

/// Save the settings when the client state is dropped (clean logout/exit).
///
/// This is **not** compiled for unit tests: `AudioSettings` values are created
/// and dropped by several tests, and saving them would overwrite the real
/// `client/audio_settings.ron` of a developer running the tests from the crate
/// directory. The client binary and integration builds always include it.
#[cfg(not(test))]
impl Drop for AudioSettings {
    fn drop(&mut self) {
        self.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volumes(settings: &AudioSettings) -> (u8, u8, u8) {
        (
            settings.master_volume,
            settings.sound_effect_volume,
            settings.background_music_volume,
        )
    }

    #[test]
    fn loads_settings_without_the_new_volume_fields() {
        let audio_settings: AudioSettings = ron::from_str("(mute_on_focus_loss: true)").expect("can't load old audio settings");

        assert!(audio_settings.mute_on_focus_loss);
        assert!(!audio_settings.background_music_muted);
        assert_eq!(volumes(&audio_settings), (100, 100, 50));
        assert_eq!(audio_settings.effective_background_music_volume(), 0.5);
        assert_eq!(audio_settings.effective_master_volume(), 1.0);
        assert_eq!(audio_settings.effective_sound_effect_volume(), 1.0);
    }

    #[test]
    fn detects_legacy_settings_files() {
        assert!(is_legacy_settings_file(
            "(mute_on_focus_loss: true, background_music_muted: false, background_music_volume: Sixty)"
        ));
        assert!(!is_legacy_settings_file(
            "(mute_on_focus_loss: true, background_music_muted: false, master_volume: 100, sound_effect_volume: 100, \
             background_music_volume: 60)"
        ));
    }

    #[test]
    fn migrates_legacy_background_music_volume_enum() {
        for (legacy, expected) in [
            ("Ten", 10),
            ("Twenty", 20),
            ("Thirty", 30),
            ("Forty", 40),
            ("Fifty", 50),
            ("Sixty", 60),
            ("Seventy", 70),
            ("Eighty", 80),
            ("Ninety", 90),
            ("Hundred", 100),
        ] {
            let settings: AudioSettings = ron::from_str(&format!(
                "(mute_on_focus_loss: true, background_music_muted: false, background_music_volume: {legacy})"
            ))
            .unwrap_or_else(|error| panic!("can't load legacy volume {legacy}: {error}"));

            assert_eq!(settings.background_music_volume, expected, "legacy value {legacy}");
            assert_eq!(settings.master_volume, 100);
            assert_eq!(settings.sound_effect_volume, 100);
        }
    }

    #[test]
    fn loads_the_new_numeric_volume_format() {
        let audio_settings: AudioSettings = ron::from_str(
            "(mute_on_focus_loss: false, background_music_muted: true, master_volume: 80, sound_effect_volume: 60, \
             background_music_volume: 40)",
        )
        .expect("can't load new audio settings");

        assert!(!audio_settings.mute_on_focus_loss);
        assert!(audio_settings.background_music_muted);
        assert_eq!(volumes(&audio_settings), (80, 60, 40));
    }

    #[test]
    fn clamps_out_of_range_volumes_on_load() {
        let audio_settings: AudioSettings = ron::from_str("(master_volume: 255, sound_effect_volume: 150, background_music_volume: 101)")
            .expect("can't load out of range audio settings");

        assert_eq!(volumes(&audio_settings), (100, 100, 100));
    }

    #[test]
    fn accepts_a_linear_background_music_volume() {
        let audio_settings: AudioSettings =
            ron::from_str("(background_music_volume: 0.25)").expect("can't load linear background music volume");

        assert_eq!(audio_settings.background_music_volume, 25);
    }

    #[test]
    fn rejects_unknown_background_music_volume_values() {
        assert!(ron::from_str::<AudioSettings>("(background_music_volume: OneHundredAndTen)").is_err());
    }

    #[test]
    fn clamps_volume_percentages() {
        assert_eq!(clamp_volume_percent(0), 0);
        assert_eq!(clamp_volume_percent(100), 100);
        assert_eq!(clamp_volume_percent(101), 100);
        assert_eq!(clamp_volume_percent(u8::MAX), 100);
    }

    #[test]
    fn converts_percentages_to_linear_gains() {
        assert_eq!(volume_percent_to_linear(0), 0.0);
        assert_eq!(volume_percent_to_linear(25), 0.25);
        assert_eq!(volume_percent_to_linear(50), 0.5);
        assert_eq!(volume_percent_to_linear(100), 1.0);
    }

    #[test]
    fn combines_master_and_category_volumes_multiplicatively() {
        let audio_settings = AudioSettings {
            master_volume: 50,
            sound_effect_volume: 40,
            background_music_volume: 80,
            ..Default::default()
        };

        assert_eq!(audio_settings.effective_master_volume(), 0.5);
        assert_eq!(audio_settings.effective_sound_effect_volume(), 0.2);
        assert_eq!(audio_settings.effective_spatial_sound_effect_volume(), 0.2);
        assert_eq!(audio_settings.effective_background_music_volume(), 0.4);
    }

    #[test]
    fn music_mute_overrides_the_music_volume_but_not_sound_effects() {
        let audio_settings = AudioSettings {
            background_music_muted: true,
            master_volume: 100,
            background_music_volume: 100,
            sound_effect_volume: 100,
            ..Default::default()
        };

        assert_eq!(audio_settings.background_music_track_volume(), 0.0);
        assert_eq!(audio_settings.effective_background_music_volume(), 0.0);
        assert_eq!(audio_settings.effective_sound_effect_volume(), 1.0);
    }

    #[test]
    fn master_volume_zero_silences_everything() {
        let audio_settings = AudioSettings {
            master_volume: 0,
            sound_effect_volume: 100,
            background_music_volume: 100,
            ..Default::default()
        };

        assert_eq!(audio_settings.effective_master_volume(), 0.0);
        assert_eq!(audio_settings.effective_sound_effect_volume(), 0.0);
        assert_eq!(audio_settings.effective_background_music_volume(), 0.0);
    }

    #[test]
    fn settings_roundtrip_through_ron() {
        let audio_settings = AudioSettings {
            mute_on_focus_loss: false,
            background_music_muted: true,
            master_volume: 70,
            sound_effect_volume: 30,
            background_music_volume: 90,
        };

        let serialized = ron::ser::to_string_pretty(&audio_settings, PrettyConfig::new()).unwrap();
        let deserialized: AudioSettings = ron::from_str(&serialized).expect("can't load saved audio settings");

        assert_eq!(deserialized, audio_settings);
        // The new format is numeric/percentage based, not the old enum idents.
        assert!(serialized.contains("master_volume: 70"));
        assert!(serialized.contains("sound_effect_volume: 30"));
        assert!(serialized.contains("background_music_volume: 90"));
        assert!(!serialized.contains("Hundred"));
    }
}
