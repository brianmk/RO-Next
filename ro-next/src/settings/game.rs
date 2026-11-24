#[cfg(feature = "debug")]
use ro_next_debug::logging::{Colorize, print_debug};
use ro_next_interface::element::StateElement;
use ron::ser::PrettyConfig;
use rust_state::RustState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, RustState, StateElement)]
pub struct GameSettings {
    pub auto_attack: bool,
    /// Battle mode (`/bm`): while the chat line is closed, printable keys are
    /// routed to the hotbar slots instead of typing into the chat.
    #[serde(default)]
    pub battle_mode: bool,
    /// Draw the remaining seconds on skill cooldown overlays (hotbar and skill
    /// tree). The sweep itself is always drawn.
    #[serde(default = "default_show_cooldown_numbers")]
    pub show_cooldown_numbers: bool,
}

fn default_show_cooldown_numbers() -> bool {
    true
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            auto_attack: true,
            battle_mode: false,
            show_cooldown_numbers: true,
        }
    }
}

impl GameSettings {
    const FILE_NAME: &'static str = "client/game_settings.ron";

    pub fn new() -> Self {
        Self::load().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            print_debug!("failed to load game settings from {}", Self::FILE_NAME.magenta());
            Default::default()
        })
    }

    pub fn load() -> Option<Self> {
        crate::logging::log_debug!("settings: loading game settings from {}", Self::FILE_NAME);
        let result = {
            #[cfg(feature = "debug")]
            print_debug!("loading game settings from {}", Self::FILE_NAME.magenta());
            std::fs::read_to_string(Self::FILE_NAME)
                .ok()
                .and_then(|data| ron::from_str(&data).ok())
        };
        match &result {
            Some(_) => crate::logging::log_debug!("settings: loaded game settings from {}", Self::FILE_NAME),
            None => crate::logging::log_warn!(
                "settings: could not load game settings from {} (using defaults)",
                Self::FILE_NAME
            ),
        };
        result
    }

    pub fn save(&self) {
        #[cfg(feature = "debug")]
        print_debug!("saving game settings to {}", Self::FILE_NAME.magenta());

        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(_error) = std::fs::write(Self::FILE_NAME, data) {
            #[cfg(feature = "debug")]
            print_debug!(
                "failed to save game settings to {}: {:?}",
                Self::FILE_NAME.magenta(),
                _error.red()
            );
        }
    }
}

impl Drop for GameSettings {
    fn drop(&mut self) {
        self.save();
    }
}
