//! Selection cooldown of the character selection window.
//!
//! When the player logs out of the game (or is sent back to the character
//! selection because a map connection failed), the character server and the
//! map server still clean up the old session for a moment. Selecting the same
//! character again immediately used to crash the client, because the server
//! rejected the fresh session while the client already started to use the old
//! state.
//!
//! This state blocks character selection for a short grace period after the
//! character selection window was shown again:
//!
//! * the character that was just played is blocked for
//!   [`SAME_CHARACTER_COOLDOWN`] (4 seconds),
//! * every other character is blocked for [`OTHER_CHARACTER_COOLDOWN`] (2
//!   seconds).
//!
//! The first login after starting the client is not delayed, because the
//! cooldown is only armed when the client returns from a map session. The
//! selection window shows the remaining time on the blocked cards and does not
//! register a selection handler until the cooldown expired, so repeated clicks
//! are a no-op instead of a crash.

use std::time::{Duration, Instant};

use ragnarok_packets::CharacterId;

/// Time the just-played character stays blocked after returning to the
/// character selection.
pub const SAME_CHARACTER_COOLDOWN: Duration = Duration::from_secs(4);
/// Time every other character stays blocked after returning to the
/// character selection.
pub const OTHER_CHARACTER_COOLDOWN: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct CharacterSelectionCooldown {
    /// Set when the client returns from a map session; consumed when the
    /// character selection window is actually shown.
    armed: bool,
    /// When the character selection window was shown after returning from a
    /// map session.
    started_at: Option<Instant>,
    /// Character that was selected last (the one the server may still consider
    /// online).
    last_character: Option<CharacterId>,
}

impl CharacterSelectionCooldown {
    /// Arm the cooldown because the client is returning from a map session.
    pub fn arm(&mut self) {
        self.armed = true;
    }

    /// Start the cooldown when the character selection window is shown.
    ///
    /// Does nothing unless the cooldown was armed, so the first login after
    /// starting the client is never delayed.
    pub fn start(&mut self, now: Instant) {
        if self.armed {
            self.armed = false;
            self.started_at = Some(now);
        }
    }

    /// Remember the character that is about to be selected, so the same
    /// character gets the longer cooldown after a logout.
    pub fn remember_selection(&mut self, character_id: CharacterId) {
        self.last_character = Some(character_id);
    }

    /// Remaining block time of a character, `None` if it can be selected.
    pub fn remaining(&self, character_id: CharacterId, now: Instant) -> Option<Duration> {
        let started_at = self.started_at?;
        let elapsed = now.saturating_duration_since(started_at);
        let cooldown = match self.last_character {
            Some(last_character) if last_character == character_id => SAME_CHARACTER_COOLDOWN,
            _ => OTHER_CHARACTER_COOLDOWN,
        };

        (elapsed < cooldown).then(|| cooldown - elapsed)
    }

    /// Whether the cooldown (or its final repaint) is currently active.
    ///
    /// The extra duration past the longest cooldown makes sure the character
    /// selection window is repainted once after the cooldown expired, so the
    /// cards become selectable again.
    pub fn is_repainting(&self, now: Instant) -> bool {
        match self.started_at {
            Some(started_at) => now.saturating_duration_since(started_at) < SAME_CHARACTER_COOLDOWN + Duration::from_millis(150),
            None => false,
        }
    }

    /// Clear the cooldown, e.g. when the client disconnects back to the login
    /// screen (the next login is a fresh session).
    pub fn clear(&mut self) {
        self.armed = false;
        self.started_at = None;
        self.last_character = None;
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use ragnarok_packets::CharacterId;

    use super::{CharacterSelectionCooldown, OTHER_CHARACTER_COOLDOWN, SAME_CHARACTER_COOLDOWN};

    fn character_id(id: u32) -> CharacterId {
        CharacterId(id)
    }

    #[test]
    fn first_login_is_not_blocked() {
        let mut cooldown = CharacterSelectionCooldown::default();
        let now = Instant::now();

        cooldown.start(now);

        assert_eq!(cooldown.remaining(character_id(1), now), None);
        assert!(!cooldown.is_repainting(now));
    }

    #[test]
    fn same_character_is_blocked_longer_than_other_characters() {
        let mut cooldown = CharacterSelectionCooldown::default();
        let now = Instant::now();
        cooldown.remember_selection(character_id(7));
        cooldown.arm();
        cooldown.start(now);

        assert_eq!(cooldown.remaining(character_id(7), now), Some(SAME_CHARACTER_COOLDOWN));
        assert_eq!(cooldown.remaining(character_id(8), now), Some(OTHER_CHARACTER_COOLDOWN));

        let after_other = now + OTHER_CHARACTER_COOLDOWN + Duration::from_millis(1);
        assert_eq!(cooldown.remaining(character_id(8), after_other), None);
        assert!(cooldown.remaining(character_id(7), after_other).is_some());

        let after_all = now + SAME_CHARACTER_COOLDOWN + Duration::from_millis(1);
        assert_eq!(cooldown.remaining(character_id(7), after_all), None);
        assert_eq!(cooldown.remaining(character_id(8), after_all), None);
    }

    #[test]
    fn cooldown_only_starts_once_per_return_from_a_map() {
        let mut cooldown = CharacterSelectionCooldown::default();
        let now = Instant::now();
        cooldown.arm();
        cooldown.start(now);

        // A second start without arming must not extend the running cooldown.
        let later = now + Duration::from_secs(1);
        cooldown.start(later);
        assert_eq!(
            cooldown.remaining(character_id(1), later),
            Some(OTHER_CHARACTER_COOLDOWN - Duration::from_secs(1))
        );
    }

    #[test]
    fn arming_again_starts_a_new_cooldown_when_the_window_is_shown() {
        let mut cooldown = CharacterSelectionCooldown::default();
        let now = Instant::now();
        cooldown.arm();
        cooldown.start(now);
        let later = now + Duration::from_secs(10);

        cooldown.arm();
        cooldown.start(later);
        assert_eq!(cooldown.remaining(character_id(1), later), Some(OTHER_CHARACTER_COOLDOWN));
    }

    #[test]
    fn repaint_stays_active_until_after_the_longest_cooldown() {
        let mut cooldown = CharacterSelectionCooldown::default();
        let now = Instant::now();
        cooldown.arm();
        cooldown.start(now);

        assert!(cooldown.is_repainting(now));
        assert!(cooldown.is_repainting(now + SAME_CHARACTER_COOLDOWN));
        assert!(!cooldown.is_repainting(now + SAME_CHARACTER_COOLDOWN + Duration::from_millis(200)));
    }

    #[test]
    fn clearing_removes_the_block() {
        let mut cooldown = CharacterSelectionCooldown::default();
        let now = Instant::now();
        cooldown.remember_selection(character_id(3));
        cooldown.arm();
        cooldown.start(now);
        cooldown.clear();

        assert_eq!(cooldown.remaining(character_id(3), now), None);
        assert!(!cooldown.is_repainting(now));
    }
}
