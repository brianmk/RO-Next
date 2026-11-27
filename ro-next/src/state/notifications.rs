//! Notification HUD state.
//!
//! Small transient overlays that the official client shows on top of the game
//! world: server broadcasts, item pickup toasts, level-up banners and quest
//! banners. The state only keeps the bounded list of currently visible
//! notifications and their lifetimes; the interface window renders them.
//!
//! Notifications are timestamped with the server client tick (the same tick the
//! chat bubbles use), so they freeze with the rest of the client when the
//! connection is paused instead of expiring while the player cannot see them.

use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// Maximum number of notifications visible at once.
pub const MAX_NOTIFICATIONS: usize = 8;

/// The visual class of a notification; the overlay uses it to pick the
/// background and text color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationKind {
    /// Server-wide broadcast (`ZC_BROADCAST` / `ZC_BROADCAST2`).
    Announce,
    /// An item was picked up (`ZC_ITEM_PICKUP_ACK`).
    Pickup,
    /// The local player gained a base or job level.
    LevelUp,
    /// A quest was started/updated (`ZC_QUEST_NOTIFY`).
    Quest,
}

impl NotificationKind {
    /// How long a notification stays on screen, in milliseconds.
    pub fn lifetime_ms(self) -> u32 {
        match self {
            NotificationKind::Announce => 8_000,
            NotificationKind::Pickup => 3_500,
            NotificationKind::LevelUp => 5_000,
            NotificationKind::Quest => 8_000,
        }
    }
}

/// One notification shown in the HUD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HudNotification {
    pub kind: NotificationKind,
    pub text: String,
    /// Client tick at which the notification was created.
    pub created_at: u32,
    pub lifetime_ms: u32,
}

impl HudNotification {
    fn age(&self, now: u32) -> u32 {
        now.wrapping_sub(self.created_at)
    }

    /// Whether the notification is still visible at `now`.
    pub fn is_alive(&self, now: u32) -> bool {
        self.age(now) < self.lifetime_ms
    }
}

/// Bounded list of the currently visible notifications.
#[derive(Default, RustState, StateElement)]
pub struct NotificationHud {
    #[hidden_element]
    entries: Vec<HudNotification>,
}

impl NotificationHud {
    /// Add a notification (the oldest one is dropped when the list is full).
    pub fn push(&mut self, kind: NotificationKind, text: impl Into<String>, now: u32) {
        let text = text.into();

        if text.is_empty() {
            return;
        }

        self.retain_alive(now);

        // Coalesce repeated identical text of the same kind (for example the
        // same item picked up repeatedly) instead of stacking a wall of equal
        // toasts; refresh the timestamp so it stays visible.
        if let Some(existing) = self.entries.iter_mut().find(|entry| entry.kind == kind && entry.text == text) {
            existing.created_at = now;
            return;
        }

        if self.entries.len() >= MAX_NOTIFICATIONS {
            self.entries.remove(0);
        }

        self.entries.push(HudNotification {
            kind,
            text,
            created_at: now,
            lifetime_ms: kind.lifetime_ms(),
        });
    }

    /// Remove expired notifications.
    pub fn retain_alive(&mut self, now: u32) {
        self.entries.retain(|entry| entry.is_alive(now));
    }

    pub fn entries(&self) -> &[HudNotification] {
        &self.entries
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_NOTIFICATIONS, NotificationHud, NotificationKind};

    #[test]
    fn notifications_expire_after_their_lifetime() {
        let mut hud = NotificationHud::default();
        hud.push(NotificationKind::Pickup, "Red Potion", 100);

        hud.retain_alive(100 + NotificationKind::Pickup.lifetime_ms() - 1);
        assert_eq!(hud.entries().len(), 1);

        hud.retain_alive(100 + NotificationKind::Pickup.lifetime_ms());
        assert!(hud.is_empty());
    }

    #[test]
    fn the_oldest_notification_is_dropped_when_the_list_is_full() {
        let mut hud = NotificationHud::default();

        for index in 0..MAX_NOTIFICATIONS + 5 {
            hud.push(NotificationKind::Pickup, format!("item {index}"), 0);
        }

        assert_eq!(hud.entries().len(), MAX_NOTIFICATIONS);
        assert_eq!(hud.entries().first().unwrap().text, "item 5");
    }

    #[test]
    fn identical_notifications_are_coalesced_and_refreshed() {
        let mut hud = NotificationHud::default();
        hud.push(NotificationKind::Announce, "Server restart", 0);
        hud.push(NotificationKind::Announce, "Server restart", 1_000);

        assert_eq!(hud.entries().len(), 1);
        assert_eq!(hud.entries()[0].created_at, 1_000);
    }

    #[test]
    fn different_kinds_with_the_same_text_are_kept_apart() {
        let mut hud = NotificationHud::default();
        hud.push(NotificationKind::Announce, "Test", 0);
        hud.push(NotificationKind::Quest, "Test", 0);

        assert_eq!(hud.entries().len(), 2);
    }

    #[test]
    fn empty_text_is_ignored() {
        let mut hud = NotificationHud::default();
        hud.push(NotificationKind::LevelUp, "", 0);

        assert!(hud.is_empty());
    }

    #[test]
    fn the_wrapping_client_tick_is_handled() {
        let mut hud = NotificationHud::default();
        hud.push(NotificationKind::Pickup, "Potion", u32::MAX - 10);

        // Ten ticks later (wrapped) the notification is still alive.
        hud.retain_alive(5);
        assert_eq!(hud.entries().len(), 1);
    }
}
