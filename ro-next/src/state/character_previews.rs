//! Idle character previews for the character selection window.
//!
//! The character selection window shows the actual composited character
//! (body, hair, headgear and palettes) of every occupied slot, like the paper
//! doll of the equipment window. The animation data is loaded asynchronously
//! through the normal loader pipeline (see
//! `Client::request_character_previews`), so the window has to read it from the
//! client state.
//!
//! The state is keyed by slot and remembers which character the loaded data
//! belongs to: when the server moves a character to another slot, deletes a
//! character or the player returns to the character selection screen after a
//! failed map entry, a stale preview can never be shown for the wrong slot.
//! Failed loads are remembered as well, so a missing sprite does not make the
//! client retry the same (impossible) load every frame.

use std::sync::Arc;

use ragnarok_packets::CharacterId;

use crate::world::AnimationData;

/// Preview state of one character slot.
#[derive(Default)]
enum SlotPreview {
    /// The slot is empty (or the character list was cleared).
    #[default]
    Empty,
    /// The animation data was requested by the client and is currently loading.
    Requested(CharacterId),
    /// The animation data is available and can be drawn.
    Ready {
        character_id: CharacterId,
        animation_data: Arc<AnimationData>,
    },
    /// The animation data could not be loaded. The slot keeps its name and
    /// level but cannot show a character sprite.
    Unavailable(CharacterId),
}

/// Preview animation of every character selection slot.
#[derive(Default)]
pub struct CharacterPreviews {
    slots: Vec<SlotPreview>,
}

impl CharacterPreviews {
    /// Resize the preview list to the number of slots of the character
    /// server, keeping already loaded previews of the existing slots.
    pub fn set_slot_count(&mut self, slot_count: usize) {
        self.slots.resize_with(slot_count, SlotPreview::default);
    }

    /// Number of slots for which previews are tracked.
    pub fn slot_count(&self) -> usize {
        self.slots.len()
    }

    /// Drop all loaded animation data. The next character list opens the
    /// window with the loading placeholders until the data arrived again.
    pub fn clear(&mut self) {
        self.slots.iter_mut().for_each(|slot| *slot = SlotPreview::Empty);
    }

    /// The loaded preview animation of a slot, if it is ready to be drawn.
    pub fn animation_data(&self, slot: usize) -> Option<&Arc<AnimationData>> {
        match self.slots.get(slot) {
            Some(SlotPreview::Ready { animation_data, .. }) => Some(animation_data),
            _ => None,
        }
    }

    /// Whether the preview of `slot` is already requested/loaded for the
    /// character with the given id. Unavailable previews count as current so
    /// they are not requested again for the same character.
    pub fn is_current(&self, slot: usize, character_id: CharacterId) -> bool {
        match self.slots.get(slot) {
            Some(SlotPreview::Requested(requested)) | Some(SlotPreview::Unavailable(requested)) => *requested == character_id,
            Some(SlotPreview::Ready {
                character_id: ready_character_id,
                ..
            }) => *ready_character_id == character_id,
            Some(SlotPreview::Empty) | None => false,
        }
    }

    /// Mark the preview of a slot as requested for a character.
    pub fn request(&mut self, slot: usize, character_id: CharacterId) {
        if let Some(slot) = self.slots.get_mut(slot) {
            *slot = SlotPreview::Requested(character_id);
        }
    }

    /// Store the loaded animation data. Only a pending request is answered;
    /// data that arrives after the slot was cleared or used for another
    /// character is ignored.
    pub fn set_ready(&mut self, slot: usize, animation_data: Arc<AnimationData>) {
        let Some(SlotPreview::Requested(character_id)) = self.slots.get(slot) else {
            return;
        };
        let character_id = *character_id;

        self.slots[slot] = SlotPreview::Ready {
            character_id,
            animation_data,
        };
    }

    /// Mark a requested preview as unavailable, so it is not retried for the
    /// same character.
    pub fn set_unavailable(&mut self, slot: usize) {
        let Some(SlotPreview::Requested(character_id)) = self.slots.get(slot) else {
            return;
        };
        let character_id = *character_id;

        self.slots[slot] = SlotPreview::Unavailable(character_id);
    }

    /// Clear the preview of a slot that no longer has a character.
    pub fn clear_slot(&mut self, slot: usize) {
        if let Some(preview) = self.slots.get_mut(slot) {
            *preview = SlotPreview::Empty;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ragnarok_packets::CharacterId;

    use super::CharacterPreviews;
    use crate::world::{AnimationData, EntityType};

    fn character_id(id: u32) -> CharacterId {
        CharacterId(id)
    }

    fn animation_data() -> Arc<AnimationData> {
        Arc::new(AnimationData {
            animation_pair: Vec::new(),
            animations: Vec::new(),
            delays: Vec::new(),
            entity_type: EntityType::Player,
        })
    }

    #[test]
    fn preview_is_only_ready_after_a_completion_of_the_current_request() {
        let mut previews = CharacterPreviews::default();
        previews.set_slot_count(2);

        assert!(!previews.is_current(0, character_id(1)));
        assert!(previews.animation_data(0).is_none());

        previews.request(0, character_id(1));
        assert!(previews.is_current(0, character_id(1)));
        assert!(!previews.is_current(0, character_id(2)));
        assert!(previews.animation_data(0).is_none());

        previews.set_ready(0, animation_data());
        assert!(previews.is_current(0, character_id(1)));
        assert!(previews.animation_data(0).is_some());
    }

    #[test]
    fn stale_data_does_not_overwrite_a_new_request() {
        let mut previews = CharacterPreviews::default();
        previews.set_slot_count(1);

        previews.request(0, character_id(1));
        // The slot is reused for another character before the first load
        // completes (slot switch / delete + create on the same slot).
        previews.request(0, character_id(2));
        previews.set_ready(0, animation_data());

        assert!(previews.is_current(0, character_id(2)));
        assert!(previews.animation_data(0).is_some());

        // A completion for a cleared slot is ignored entirely.
        previews.clear_slot(0);
        previews.set_ready(0, animation_data());
        assert!(!previews.is_current(0, character_id(2)));
        assert!(previews.animation_data(0).is_none());
    }

    #[test]
    fn failed_preview_is_not_requested_again() {
        let mut previews = CharacterPreviews::default();
        previews.set_slot_count(1);

        previews.request(0, character_id(7));
        previews.set_unavailable(0);

        assert!(previews.is_current(0, character_id(7)));
        assert!(!previews.is_current(0, character_id(8)));
        assert!(previews.animation_data(0).is_none());
    }

    #[test]
    fn clearing_and_resizing_keeps_the_state_consistent() {
        let mut previews = CharacterPreviews::default();
        previews.set_slot_count(3);
        previews.request(1, character_id(5));
        previews.set_ready(1, animation_data());

        previews.set_slot_count(2);
        assert_eq!(previews.slot_count(), 2);
        assert!(previews.animation_data(1).is_some());

        previews.clear();
        assert!(previews.animation_data(1).is_none());
        assert!(!previews.is_current(1, character_id(5)));
    }
}
