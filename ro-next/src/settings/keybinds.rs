//! Persisted player key binding overrides.
//!
//! The default bindings live in `data\client\keybinds.ron` (see
//! [`crate::input::keybinds`]). Remapping a shortcut in the interface settings
//! window writes only the overrides to this settings file, so a client archive
//! update can change the defaults without destroying the player's choices.

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

/// One overridden shortcut.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeybindOverride {
    /// Stable action name (`ShortcutAction::name`).
    pub action: String,
    /// Physical key name (`KeyCode` debug name, e.g. `KeyE`, `F12`).
    pub key: String,
    #[serde(default)]
    pub alt: bool,
    #[serde(default)]
    pub control: bool,
    #[serde(default)]
    pub shift: bool,
}

/// Saved key binding overrides.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KeybindSettings {
    #[serde(default)]
    pub overrides: Vec<KeybindOverride>,
}

impl KeybindSettings {
    const FILE_NAME: &'static str = "client/keybind_settings.ron";

    /// Loads the saved overrides. A missing or malformed file means "use the
    /// data defaults"; it is never an error that could take the keyboard away.
    pub fn new() -> Self {
        crate::logging::log_debug!("settings: loading key bindings from {}", Self::FILE_NAME);

        std::fs::read_to_string(Self::FILE_NAME)
            .ok()
            .and_then(|data| ron::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let data = ron::ser::to_string_pretty(self, PrettyConfig::new()).unwrap();

        if let Err(error) = std::fs::write(Self::FILE_NAME, data) {
            crate::logging::log_warn!("settings: could not save key bindings to {}: {error:?}", Self::FILE_NAME);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{KeybindOverride, KeybindSettings};

    #[test]
    fn overrides_round_trip_through_ron() {
        let settings = KeybindSettings {
            overrides: vec![KeybindOverride {
                action: "ToggleInventoryWindow".to_owned(),
                key: "KeyB".to_owned(),
                alt: false,
                control: true,
                shift: false,
            }],
        };

        let data = ron::ser::to_string(&settings).unwrap();
        let parsed: KeybindSettings = ron::from_str(&data).unwrap();

        assert_eq!(parsed.overrides, settings.overrides);
    }

    #[test]
    fn missing_modifier_fields_default_to_false() {
        let parsed: KeybindSettings = ron::from_str("(overrides: [(action: \"ToggleChatWindow\", key: \"KeyK\")])").unwrap();

        assert_eq!(parsed.overrides.len(), 1);
        assert!(!parsed.overrides[0].alt);
        assert!(!parsed.overrides[0].control);
        assert!(!parsed.overrides[0].shift);
    }
}
