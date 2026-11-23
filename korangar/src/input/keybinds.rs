//! Data-driven keyboard bindings.
//!
//! The keyboard shortcuts of the client used to be hardcoded in
//! [`super::InputSystem::handle_keyboard_input`]. They now live in
//! `archive/data/client/keybinds.ron`, embedded with `include_str!` as the
//! compiled fallback and loaded through [`GameFileLoader`] so a server can
//! override the bindings in the client archive without rebuilding the client.
//!
//! The table covers every *interface* key binding:
//!
//! * window / interface shortcuts ([`shortcuts`]), with modifier requirements
//!   and the "no modifier" matching used by the plain `I`/`M` keys,
//! * the emotion keys (`Alt` + key, position = emotion number),
//! * the hotbar slot keys (position = slot),
//! * the hotbar tab cycle keys,
//! * the keys that confirm a focused text input (chat command / login form).
//!
//! Debug-only camera and debug-window keys stay in the `debug` feature code:
//! they are developer shortcuts, not user-facing bindings.
//!
//! A malformed archive override is reported and the compiled table is used
//! instead, so a bad override can never leave the client without keyboard
//! controls. [`validate`] (exposed as `korangar --check-keybinds [PATH]`) is
//! the schema/drift guard.

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use serde::Deserialize;
use winit::keyboard::KeyCode;

use super::key_names::key_code_from_name;
use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\keybinds.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/keybinds.ron");

/// Maximum number of emotion keys (`Alt` + `1`..`9` in the classic client).
const MAX_EMOTION_KEYS: usize = 9;
/// Number of hotbar slots per tab (`HotbarSlot` is a `u16`, the 2019 client
/// uses 4 tabs x 10 slots).
const MAX_HOTBAR_KEYS: usize = 10;

/// A modifier that a [`Keybind`] requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
enum Modifier {
    Alt,
    Control,
    Shift,
}

/// The modifier state of the keyboard in one frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActiveModifiers {
    pub alt: bool,
    pub control: bool,
    pub shift: bool,
}

impl ActiveModifiers {
    fn has(self, modifier: Modifier) -> bool {
        match modifier {
            Modifier::Alt => self.alt,
            Modifier::Control => self.control,
            Modifier::Shift => self.shift,
        }
    }
}

/// An interface action that can be bound to a key.
///
/// The variants are stable names used by the data file; the input system maps
/// them to the corresponding [`crate::input::InputEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ShortcutAction {
    /// Close the topmost window, or open the menu when none is open.
    CloseOrMenu,
    ToggleInventoryWindow,
    ToggleSkillTreeWindow,
    ToggleStatsWindow,
    ToggleFriendListWindow,
    ToggleEquipmentWindow,
    ToggleChatWindow,
    ToggleGameSettingsWindow,
    ToggleInterfaceSettingsWindow,
    ToggleGraphicsSettingsWindow,
    ToggleAudioSettingsWindow,
    ToggleShowInterface,
    ToggleMinimapWindow,
    CycleMapWindows,
    CloseTopWindow,
}

/// A physical key as written in the data file (the winit [`KeyCode`] name).
///
/// Deserialized from a string like `"KeyE"`, `"Digit1"`, `"F1"`, `"Escape"`;
/// the full list of accepted names is the winit `KeyCode` enum (see
/// [`super::key_names`]). An unknown name is a schema error, so a typo in an
/// archive override is loud instead of silently disabling a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(try_from = "String")]
struct BindKey(KeyCode);

impl TryFrom<String> for BindKey {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        key_code_from_name(&name).map(BindKey).ok_or_else(|| {
            format!("unknown key name `{name}` (use a winit KeyCode name, for example `\"KeyE\"`, `\"Digit1\"`, `\"F1\"` or `\"Escape\"`)")
        })
    }
}

/// One key binding of the data file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShortcutFile {
    action: ShortcutAction,
    key: BindKey,
    #[serde(default)]
    modifiers: Vec<Modifier>,
    /// Whether other modifiers invalidate the binding. The plain `I`/`M` keys
    /// only fire when no modifier is held; the classic Alt/Ctrl shortcuts fire
    /// even when an additional modifier is held (they only require their own).
    #[serde(default)]
    exact_modifiers: bool,
}

/// Data file schema; the field names are the public data-file contract.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeybindFile {
    #[allow(dead_code)]
    provenance: String,
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    shortcuts: Vec<ShortcutFile>,
    emotion_keys: Vec<BindKey>,
    hotbar_keys: Vec<BindKey>,
    hotbar_tab_previous: BindKey,
    hotbar_tab_next: BindKey,
    chat_command_keys: Vec<BindKey>,
}

/// A validated key binding.
#[derive(Debug, Clone)]
pub struct Keybind {
    pub action: ShortcutAction,
    pub key: KeyCode,
    modifiers: Vec<Modifier>,
    exact_modifiers: bool,
}

impl Keybind {
    /// Whether the required modifiers are held (and, for exact bindings, no
    /// other modifier is held).
    pub fn matches(&self, active: ActiveModifiers) -> bool {
        if self.exact_modifiers {
            return active.alt == self.modifiers.contains(&Modifier::Alt)
                && active.control == self.modifiers.contains(&Modifier::Control)
                && active.shift == self.modifiers.contains(&Modifier::Shift);
        }

        self.modifiers.iter().all(|modifier| active.has(*modifier))
    }
}

/// The validated runtime key binding table.
#[derive(Debug)]
pub struct Keybinds {
    shortcuts: Vec<Keybind>,
    emotion_keys: Vec<KeyCode>,
    hotbar_keys: Vec<KeyCode>,
    hotbar_tab_previous: KeyCode,
    hotbar_tab_next: KeyCode,
    chat_command_keys: Vec<KeyCode>,
}

impl Keybinds {
    fn from_file(file: KeybindFile) -> Self {
        Self {
            shortcuts: file
                .shortcuts
                .into_iter()
                .map(|shortcut| Keybind {
                    action: shortcut.action,
                    key: shortcut.key.0,
                    modifiers: shortcut.modifiers,
                    exact_modifiers: shortcut.exact_modifiers,
                })
                .collect(),
            emotion_keys: file.emotion_keys.into_iter().map(|key| key.0).collect(),
            hotbar_keys: file.hotbar_keys.into_iter().map(|key| key.0).collect(),
            hotbar_tab_previous: file.hotbar_tab_previous.0,
            hotbar_tab_next: file.hotbar_tab_next.0,
            chat_command_keys: file.chat_command_keys.into_iter().map(|key| key.0).collect(),
        }
    }

    /// Number of key bindings in the table (used by the `--check-keybinds`
    /// output).
    fn count(&self) -> usize {
        self.shortcuts.len() + self.emotion_keys.len() + self.hotbar_keys.len() + 2 + self.chat_command_keys.len()
    }

    /// Rejects data that parses but cannot work: duplicate combinations,
    /// out-of-range hotbar/emotion lists and a missing close key.
    fn validate(&self) -> Result<(), String> {
        if self.shortcuts.is_empty() {
            return Err("`shortcuts` must not be empty".to_owned());
        }

        if !self.shortcuts.iter().any(|shortcut| shortcut.action == ShortcutAction::CloseOrMenu) {
            return Err("`shortcuts` has no CloseOrMenu binding (Escape), the client cannot close windows".to_owned());
        }

        let mut seen = Vec::with_capacity(self.shortcuts.len());
        for shortcut in &self.shortcuts {
            let mut modifiers = shortcut.modifiers.clone();
            modifiers.sort_by_key(|modifier| match modifier {
                Modifier::Alt => 0,
                Modifier::Control => 1,
                Modifier::Shift => 2,
            });
            modifiers.dedup();

            if modifiers.len() != shortcut.modifiers.len() {
                return Err(format!("shortcut `{:?}` lists a modifier twice", shortcut.action));
            }

            let combination = (shortcut.key, modifiers, shortcut.exact_modifiers);
            if seen.contains(&combination) {
                return Err(format!(
                    "duplicate shortcut binding: key {:?} with {:?} modifiers is bound twice",
                    shortcut.key, shortcut.modifiers
                ));
            }
            seen.push(combination);
        }

        if self.emotion_keys.is_empty() {
            return Err("`emotion_keys` must not be empty".to_owned());
        }
        if self.emotion_keys.len() > MAX_EMOTION_KEYS {
            return Err(format!("`emotion_keys` has more than {MAX_EMOTION_KEYS} entries"));
        }
        check_unique("emotion_keys", &self.emotion_keys)?;

        if self.hotbar_keys.is_empty() {
            return Err("`hotbar_keys` must not be empty".to_owned());
        }
        if self.hotbar_keys.len() > MAX_HOTBAR_KEYS {
            return Err(format!("`hotbar_keys` has more than {MAX_HOTBAR_KEYS} entries"));
        }
        check_unique("hotbar_keys", &self.hotbar_keys)?;

        if self.hotbar_tab_previous == self.hotbar_tab_next {
            return Err("`hotbar_tab_previous` and `hotbar_tab_next` must be different keys".to_owned());
        }

        if self.chat_command_keys.is_empty() {
            return Err("`chat_command_keys` must not be empty".to_owned());
        }
        check_unique("chat_command_keys", &self.chat_command_keys)?;

        Ok(())
    }
}

fn check_unique(label: &str, keys: &[KeyCode]) -> Result<(), String> {
    for (index, key) in keys.iter().enumerate() {
        if keys[..index].contains(key) {
            return Err(format!("`{label}` contains the key {key:?} twice"));
        }
    }
    Ok(())
}

fn parse(content: &str) -> Result<Keybinds, String> {
    let file: KeybindFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    let keybinds = Keybinds::from_file(file);
    keybinds.validate()?;
    Ok(keybinds)
}

static COMPILED: OnceLock<Keybinds> = OnceLock::new();
static ARCHIVE: OnceLock<Keybinds> = OnceLock::new();

fn compiled() -> &'static Keybinds {
    COMPILED.get_or_init(|| parse(FALLBACK).expect("the compiled keybind data has to be valid"))
}

fn table() -> &'static Keybinds {
    ARCHIVE.get().unwrap_or_else(|| compiled())
}

/// The interface shortcuts of the active table, in evaluation order.
pub fn shortcuts() -> &'static [Keybind] {
    &table().shortcuts
}

/// The `Alt` + key emotion bindings; the index is the emotion number.
pub fn emotion_keys() -> &'static [KeyCode] {
    &table().emotion_keys
}

/// The hotbar slot keys; the index is the slot.
pub fn hotbar_keys() -> &'static [KeyCode] {
    &table().hotbar_keys
}

/// The previous/next hotbar tab keys (`F11`/`F12` by default).
pub fn hotbar_tab_keys() -> (KeyCode, KeyCode) {
    (table().hotbar_tab_previous, table().hotbar_tab_next)
}

/// The keys that confirm a focused text input (chat command / login form).
pub fn chat_command_keys() -> &'static [KeyCode] {
    &table().chat_command_keys
}

/// Validates key binding data and returns the number of bindings.
pub(crate) fn validate(content: &str) -> Result<usize, String> {
    parse(content).map(|keybinds| keybinds.count())
}

/// Validates the compiled fallback table.
pub(crate) fn validate_compiled() -> Result<usize, String> {
    parse(FALLBACK).map(|keybinds| keybinds.count())
}

/// Loads the key binding table from the client archive (or keeps the compiled
/// fallback).
///
/// Returns the number of loaded bindings. A malformed data file is reported
/// and the compiled fallback is used instead.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_debug!("keybinds: loaded {} bindings from the compiled fallback", compiled().count());
        return compiled().count();
    };

    match parse(content) {
        Ok(keybinds) => {
            let count = keybinds.count();
            let _ = ARCHIVE.set(keybinds);
            crate::logging::log_debug!("keybinds: loaded {count} bindings from {DATA_PATH}");
            count
        }
        Err(error) => {
            crate::logging::log_warn!("keybinds: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().count()
        }
    }
}

#[cfg(test)]
mod tests {
    use winit::keyboard::KeyCode;

    use super::{
        ActiveModifiers, Modifier, ShortcutAction, chat_command_keys, compiled, emotion_keys, hotbar_keys, hotbar_tab_keys, parse,
        shortcuts, validate, validate_compiled,
    };

    fn no_modifiers() -> ActiveModifiers {
        ActiveModifiers::default()
    }

    fn alt() -> ActiveModifiers {
        ActiveModifiers {
            alt: true,
            ..ActiveModifiers::default()
        }
    }

    fn control() -> ActiveModifiers {
        ActiveModifiers {
            control: true,
            ..ActiveModifiers::default()
        }
    }

    fn shortcut(action: ShortcutAction) -> usize {
        shortcuts()
            .iter()
            .position(|shortcut| shortcut.action == action)
            .unwrap_or_else(|| panic!("missing shortcut {action:?}"))
    }

    #[test]
    fn compiled_table_is_valid() {
        let count = validate_compiled().expect("the committed keybind data has to be valid");
        assert_eq!(
            count,
            shortcuts().len() + emotion_keys().len() + hotbar_keys().len() + 2 + chat_command_keys().len()
        );
        // Defaults: 16 shortcuts, 9 emotions, 10 hotbar slots, 2 tab keys and
        // the 2 Enter keys that confirm a focused text input.
        assert_eq!(count, 16 + 9 + 10 + 2 + 2);
    }

    #[test]
    fn defaults_match_the_former_hardcoded_bindings() {
        // The defaults are the exact bindings that used to be hardcoded in
        // `handle_keyboard_input`, so existing players see no change.
        let assert_shortcut = |action, key, modifiers: &[Modifier], exact| {
            let shortcut = &shortcuts()[shortcut(action)];
            assert_eq!(shortcut.key, key, "wrong key for {action:?}");
            assert_eq!(shortcut.modifiers, modifiers, "wrong modifiers for {action:?}");
            assert_eq!(shortcut.exact_modifiers, exact, "wrong modifier matching for {action:?}");
        };

        assert_shortcut(ShortcutAction::CloseOrMenu, KeyCode::Escape, &[], false);
        assert_shortcut(ShortcutAction::ToggleInventoryWindow, KeyCode::KeyE, &[Modifier::Alt], false);
        assert_shortcut(ShortcutAction::ToggleSkillTreeWindow, KeyCode::KeyS, &[Modifier::Alt], false);
        assert_shortcut(ShortcutAction::ToggleStatsWindow, KeyCode::KeyA, &[Modifier::Alt], false);
        assert_shortcut(ShortcutAction::ToggleFriendListWindow, KeyCode::KeyZ, &[Modifier::Alt], false);
        assert_shortcut(ShortcutAction::ToggleEquipmentWindow, KeyCode::KeyQ, &[Modifier::Alt], false);
        assert_shortcut(ShortcutAction::ToggleChatWindow, KeyCode::KeyC, &[Modifier::Alt], false);
        assert_shortcut(
            ShortcutAction::ToggleGameSettingsWindow,
            KeyCode::KeyS,
            &[Modifier::Control],
            false,
        );
        assert_shortcut(
            ShortcutAction::ToggleInterfaceSettingsWindow,
            KeyCode::KeyI,
            &[Modifier::Control],
            false,
        );
        assert_shortcut(
            ShortcutAction::ToggleGraphicsSettingsWindow,
            KeyCode::KeyG,
            &[Modifier::Control],
            false,
        );
        assert_shortcut(
            ShortcutAction::ToggleAudioSettingsWindow,
            KeyCode::KeyA,
            &[Modifier::Control],
            false,
        );
        assert_shortcut(ShortcutAction::ToggleShowInterface, KeyCode::KeyH, &[Modifier::Control], false);
        assert_shortcut(ShortcutAction::ToggleMinimapWindow, KeyCode::KeyM, &[Modifier::Control], false);
        assert_shortcut(ShortcutAction::CloseTopWindow, KeyCode::KeyQ, &[Modifier::Control], false);

        // The plain keys are exact: they must not fire while a modifier is held.
        let plain_maps = shortcuts()
            .iter()
            .find(|shortcut| shortcut.action == ShortcutAction::CycleMapWindows)
            .expect("plain M binding");
        assert_eq!(plain_maps.key, KeyCode::KeyM);
        assert!(plain_maps.exact_modifiers);
        let plain_inventory = shortcuts()
            .iter()
            .find(|shortcut| shortcut.action == ShortcutAction::ToggleInventoryWindow && shortcut.exact_modifiers)
            .expect("plain I binding");
        assert_eq!(plain_inventory.key, KeyCode::KeyI);

        assert_eq!(emotion_keys(), &[
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
        ]);
        assert_eq!(hotbar_keys(), &[
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
            KeyCode::F9,
            KeyCode::F10,
        ]);
        assert_eq!(hotbar_tab_keys(), (KeyCode::F11, KeyCode::F12));
        assert_eq!(chat_command_keys(), &[KeyCode::Enter, KeyCode::NumpadEnter]);
    }

    #[test]
    fn modifier_matching_preserves_the_conflict_handling() {
        let inventory = &shortcuts()[shortcut(ShortcutAction::ToggleInventoryWindow)];
        // Alt+E fires with Alt, and (like before) also with Ctrl+Alt+E.
        assert!(inventory.matches(alt()));
        assert!(inventory.matches(ActiveModifiers { control: true, ..alt() }));
        assert!(!inventory.matches(no_modifiers()));
        assert!(!inventory.matches(control()));

        // The plain I binding is exact: Ctrl+I is the interface settings window.
        let plain_inventory = shortcuts()
            .iter()
            .find(|shortcut| shortcut.action == ShortcutAction::ToggleInventoryWindow && shortcut.exact_modifiers)
            .expect("plain I binding");
        assert!(plain_inventory.matches(no_modifiers()));
        assert!(!plain_inventory.matches(control()));
        assert!(!plain_inventory.matches(alt()));

        let minimap = &shortcuts()[shortcut(ShortcutAction::ToggleMinimapWindow)];
        assert!(minimap.matches(control()));
        assert!(!minimap.matches(no_modifiers()));
    }

    #[test]
    fn malformed_data_is_rejected() {
        assert!(validate("(shortcuts: [)").is_err(), "broken RON must be rejected");
        assert!(
            validate(&compiled_data().replace(
                "(action: CloseOrMenu, key: \"Escape\")",
                "(action: CloseOrMenu, key: \"Escape\", typo: true)"
            ))
            .is_err(),
            "unknown fields must be rejected"
        );
        assert!(
            validate(&compiled_data().replace("key: \"Escape\"", "key: \"NotAKey\"")).is_err(),
            "unknown key names must be rejected"
        );
        assert!(
            validate(&compiled_data().replace("action: CloseOrMenu", "action: NotAnAction")).is_err(),
            "unknown actions must be rejected"
        );
        assert!(
            validate(&compiled_data().replace("hotbar_tab_next: \"F12\"", "hotbar_tab_next: \"F11\"")).is_err(),
            "duplicate tab keys must be rejected"
        );
        assert!(
            validate(&compiled_data().replace("hotbar_keys: [\"F1\",", "hotbar_keys: [\"F1\", \"F1\",")).is_err(),
            "duplicate hotbar keys must be rejected"
        );
        assert!(
            validate(&compiled_data().replace(
                "(action: CloseOrMenu, key: \"Escape\"),",
                "(action: CloseOrMenu, key: \"Escape\"), (action: CloseOrMenu, key: \"Escape\"),",
            ))
            .is_err(),
            "duplicate shortcut combinations must be rejected"
        );
        assert!(
            validate(&compiled_data().replace("emotion_keys: [\"Digit1\"", "emotion_keys: [\"Digit1\", \"Digit1\"")).is_err(),
            "duplicate emotion keys must be rejected"
        );
    }

    /// The raw compiled data, for tests that mutate single fields.
    fn compiled_data() -> &'static str {
        super::FALLBACK
    }

    #[test]
    fn the_compiled_table_round_trips_through_the_parser() {
        let keybinds = parse(compiled_data()).expect("compiled data must parse");
        assert_eq!(keybinds.count(), compiled().count());
        assert_eq!(
            super::table().count(),
            compiled().count(),
            "without an archive override the compiled table is used"
        );
    }
}
