//! Battle-mode keyboard routing (`/bm`).
//!
//! In battle mode the chat line stays closed, so printable keys do not type
//! into the chat. The official client routes the keys that no other window
//! wants to the shortcut layer. This client keeps the data-driven interface
//! shortcuts first (so `Alt+E` still opens the inventory) and maps the number
//! row and the first two letter rows of the keyboard to the visible hotbar
//! slots of the active bar.
//!
//! The mapping is deliberately small and predictable: `1`-`9` and `0` are the
//! ten slots of the first row, `A`-`R` are the eighteen visible slots (both
//! rows). Enter still opens the chat line; the actual chat focus is handled by
//! the client, not by this module, so this table is pure and unit-testable.

use ragnarok_packets::HotbarSlot;
use winit::keyboard::KeyCode;

use crate::state::hotbar::VISIBLE_HOTBAR_SLOTS;

/// Visible hotbar slot that a key uses in battle mode, if any.
///
/// The index is a *visible* slot (0-17), the same index the hotbar window and
/// the F1-F9 keys use.
pub fn slot_for_key(key: KeyCode) -> Option<HotbarSlot> {
    let slot = match key {
        KeyCode::Digit1 => 0,
        KeyCode::Digit2 => 1,
        KeyCode::Digit3 => 2,
        KeyCode::Digit4 => 3,
        KeyCode::Digit5 => 4,
        KeyCode::Digit6 => 5,
        KeyCode::Digit7 => 6,
        KeyCode::Digit8 => 7,
        KeyCode::Digit9 => 8,
        KeyCode::Digit0 => 9,
        KeyCode::KeyA => 10,
        KeyCode::KeyB => 11,
        KeyCode::KeyC => 12,
        KeyCode::KeyD => 13,
        KeyCode::KeyE => 14,
        KeyCode::KeyF => 15,
        KeyCode::KeyG => 16,
        KeyCode::KeyH => 17,
        _ => return None,
    };

    (slot < VISIBLE_HOTBAR_SLOTS).then_some(HotbarSlot(slot as u16))
}

#[cfg(test)]
mod tests {
    use winit::keyboard::KeyCode;

    use super::slot_for_key;

    #[test]
    fn number_row_maps_to_the_first_row() {
        assert_eq!(slot_for_key(KeyCode::Digit1).map(|slot| slot.0), Some(0));
        assert_eq!(slot_for_key(KeyCode::Digit9).map(|slot| slot.0), Some(8));
        assert_eq!(slot_for_key(KeyCode::Digit0).map(|slot| slot.0), Some(9));
    }

    #[test]
    fn letters_map_to_both_visible_rows() {
        assert_eq!(slot_for_key(KeyCode::KeyA).map(|slot| slot.0), Some(10));
        assert_eq!(slot_for_key(KeyCode::KeyH).map(|slot| slot.0), Some(17));
        // Letters without a visible slot are not routed to a hotbar slot; the
        // closed battle-mode chat line still keeps them out of the chat.
        assert_eq!(slot_for_key(KeyCode::KeyZ), None);
        assert_eq!(slot_for_key(KeyCode::Space), None);
        assert_eq!(slot_for_key(KeyCode::Enter), None);
    }
}
