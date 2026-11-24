//! Runtime state of the key binding editor in the interface settings window.
//!
//! The editor stores the index of the action whose binding is waiting for the
//! next key press instead of the action itself, so the state stays a simple
//! serializable value and does not couple the input layer to the state tree.

use ro_next_interface::element::StateElement;
use rust_state::RustState;

use crate::input::{ALL_SHORTCUT_ACTIONS, ShortcutAction};

/// Pending key binding capture of the interface settings window.
#[derive(Default, RustState, StateElement)]
pub struct KeyRebindState {
    /// Index into [`ALL_SHORTCUT_ACTIONS`] whose key is being captured.
    #[hidden_element]
    capturing: Option<usize>,
}

impl KeyRebindState {
    /// Starts capturing a key for the given action.
    pub fn begin(&mut self, action: ShortcutAction) {
        self.capturing = ALL_SHORTCUT_ACTIONS.iter().position(|candidate| *candidate == action);
    }

    /// The action whose binding is being captured.
    pub fn capturing(&self) -> Option<ShortcutAction> {
        self.capturing.and_then(|index| ALL_SHORTCUT_ACTIONS.get(index)).copied()
    }

    /// Stops capturing a key.
    pub fn cancel(&mut self) {
        self.capturing = None;
    }

    /// Whether the editor is waiting for a key.
    #[allow(dead_code)]
    pub fn is_capturing(&self) -> bool {
        self.capturing.is_some()
    }
}

#[cfg(test)]
mod tests {
    use crate::input::ShortcutAction;
    use crate::state::key_rebind::KeyRebindState;

    #[test]
    fn begin_cancel_roundtrip() {
        let mut state = KeyRebindState::default();
        assert!(!state.is_capturing());

        state.begin(ShortcutAction::RotateHotbarRows);
        assert_eq!(state.capturing(), Some(ShortcutAction::RotateHotbarRows));
        assert!(state.is_capturing());

        state.cancel();
        assert_eq!(state.capturing(), None);
    }
}
