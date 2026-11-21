#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

/// The volume of the background music.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, RustState, StateElement)]
pub enum BackgroundMusicVolume {
    Ten,
    Twenty,
    Thirty,
    Forty,
    #[default]
    Fifty,
    Sixty,
    Seventy,
    Eighty,
    Ninety,
    Hundred,
}

impl BackgroundMusicVolume {
    /// The linear volume in the range from `0.0` to `1.0`.
    pub fn linear(self) -> f32 {
        match self {
            Self::Ten => 0.1,
            Self::Twenty => 0.2,
            Self::Thirty => 0.3,
            Self::Forty => 0.4,
            Self::Fifty => 0.5,
            Self::Sixty => 0.6,
            Self::Seventy => 0.7,
            Self::Eighty => 0.8,
            Self::Ninety => 0.9,
            Self::Hundred => 1.0,
        }
    }
}

impl DropDownItem<BackgroundMusicVolume> for BackgroundMusicVolume {
    fn text(&self) -> &str {
        match self {
            Self::Ten => "10%",
            Self::Twenty => "20%",
            Self::Thirty => "30%",
            Self::Forty => "40%",
            Self::Fifty => "50%",
            Self::Sixty => "60%",
            Self::Seventy => "70%",
            Self::Eighty => "80%",
            Self::Ninety => "90%",
            Self::Hundred => "100%",
        }
    }

    fn value(&self) -> BackgroundMusicVolume {
        *self
    }
}

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct AudioSettings {
    pub mute_on_focus_loss: bool,
    /// Whether the background music is muted.
    ///
    /// This is separate from [`Self::background_music_volume`] so that the
    /// previous volume is restored when the music is unmuted again.
    #[serde(default)]
    pub background_music_muted: bool,
    /// The volume of the background music.
    #[serde(default)]
    pub background_music_volume: BackgroundMusicVolume,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mute_on_focus_loss: true,
            background_music_muted: false,
            background_music_volume: BackgroundMusicVolume::default(),
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
        let result = {
            #[cfg(feature = "debug")]
            print_debug!("loading audio settings from {}", Self::FILE_NAME.magenta());
            std::fs::read_to_string(Self::FILE_NAME)
                .ok()
                .and_then(|data| ron::from_str(&data).ok())
        };
        match &result {
            Some(_) => crate::logging::log_debug!("settings: loaded audio settings from {}", Self::FILE_NAME),
            None => crate::logging::log_warn!(
                "settings: could not load audio settings from {} (using defaults)",
                Self::FILE_NAME
            ),
        };
        result
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

    /// The linear volume that the background music should be played at.
    pub fn effective_background_music_volume(&self) -> f32 {
        match self.background_music_muted {
            true => 0.0,
            false => self.background_music_volume.linear(),
        }
    }
}

impl Drop for AudioSettings {
    fn drop(&mut self) {
        self.save();
    }
}

/// The values that the audio settings window offers for the settings that can
/// only take a fixed set of values.
#[derive(RustState, StateElement)]
pub struct AudioSettingsCapabilities {
    background_music_volume_options: Vec<BackgroundMusicVolume>,
}

impl Default for AudioSettingsCapabilities {
    fn default() -> Self {
        Self {
            background_music_volume_options: vec![
                BackgroundMusicVolume::Ten,
                BackgroundMusicVolume::Twenty,
                BackgroundMusicVolume::Thirty,
                BackgroundMusicVolume::Forty,
                BackgroundMusicVolume::Fifty,
                BackgroundMusicVolume::Sixty,
                BackgroundMusicVolume::Seventy,
                BackgroundMusicVolume::Eighty,
                BackgroundMusicVolume::Ninety,
                BackgroundMusicVolume::Hundred,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_settings_without_the_new_music_fields() {
        let audio_settings: AudioSettings = ron::from_str("(mute_on_focus_loss: true)").expect("can't load old audio settings");

        assert!(audio_settings.mute_on_focus_loss);
        assert!(!audio_settings.background_music_muted);
        assert_eq!(audio_settings.background_music_volume, BackgroundMusicVolume::Fifty);
        assert_eq!(audio_settings.effective_background_music_volume(), 0.5);
    }

    #[test]
    fn mutes_background_music() {
        let audio_settings = AudioSettings {
            background_music_muted: true,
            background_music_volume: BackgroundMusicVolume::Hundred,
            ..Default::default()
        };

        assert_eq!(audio_settings.effective_background_music_volume(), 0.0);
    }
}
