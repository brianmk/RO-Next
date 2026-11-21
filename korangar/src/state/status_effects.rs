//! Active status effects (buffs and debuffs) of the player.
//!
//! The map server informs the client about status effects with three packets:
//!
//! * `0x0983` (`ZC_MSG_STATE_CHANGE3`) - status starts/refreshes with a total
//!   and remaining duration, or ends when the state flag is zero,
//! * `0x0196` (`ZC_MSG_STATE_CHANGE`) - status ends (also used to start
//!   statuses on non-player units); it carries no duration,
//! * `0x0984` (`ZC_EFST_SET_ENTER`) - active statuses are sent again when the
//!   client enters the map or when a buffed entity enters the view.
//!
//! This module keeps the player's active status effects and their timers in the
//! [`ClientState`](crate::state::ClientState) so that the buff bar can render
//! them and count them down locally between packets.

use std::sync::Arc;

use korangar_interface::element::StateElement;
use rust_state::RustState;

use crate::graphics::Texture;

/// Static information about a known status effect.
#[derive(Clone, Copy)]
pub struct StatusEffectDefinition {
    pub name: &'static str,
    /// Client texture path of the icon, relative to `data\texture`, if the
    /// client data contains one for this status.
    pub icon: Option<&'static str>,
}

impl StatusEffectDefinition {
    /// Client texture path of the icon, relative to `data\texture` (the
    /// texture loader prepends that directory).
    pub fn icon_path(self) -> Option<String> {
        self.icon.map(str::to_owned)
    }
}

const UNKNOWN_DEFINITION: StatusEffectDefinition = StatusEffectDefinition {
    name: "Status effect",
    icon: None,
};

/// Returns the (best known) definition for a status effect index.
///
/// The indices are the values of the client's `EFST_IDs` table
/// (`data/lua files/stateicon/efstids.lub`), which is what the 2019 server
/// sends in the status packets. Statuses without an entry here are still shown
/// (with a generic name) so that no active effect is invisible.
pub fn status_effect_definition(status_index: u16) -> StatusEffectDefinition {
    match status_index {
        0 => StatusEffectDefinition {
            name: "Provoke",
            icon: Some("유저인터페이스\\item\\sm_provoke.bmp"),
        },
        1 => StatusEffectDefinition {
            name: "Endure",
            icon: Some("유저인터페이스\\item\\cr_endure.bmp"),
        },
        2 => StatusEffectDefinition {
            name: "Riding",
            icon: Some("유저인터페이스\\item\\kn_riding.bmp"),
        },
        3 => StatusEffectDefinition {
            name: "Improve Concentration",
            icon: Some("유저인터페이스\\item\\ac_concentration.bmp"),
        },
        9 => StatusEffectDefinition {
            name: "Angelus",
            icon: Some("유저인터페이스\\item\\al_angelus.bmp"),
        },
        10 => StatusEffectDefinition {
            name: "Blessing",
            icon: Some("유저인터페이스\\item\\al_blessing.bmp"),
        },
        12 => StatusEffectDefinition {
            name: "Increase AGI",
            icon: Some("유저인터페이스\\item\\al_incagi.bmp"),
        },
        15 => StatusEffectDefinition {
            name: "Impositio Manus",
            icon: Some("유저인터페이스\\item\\pr_impositio.bmp"),
        },
        241 => StatusEffectDefinition {
            name: "STR Food",
            icon: None,
        },
        242 => StatusEffectDefinition {
            name: "AGI Food",
            icon: None,
        },
        243 => StatusEffectDefinition {
            name: "VIT Food",
            icon: None,
        },
        244 => StatusEffectDefinition {
            name: "DEX Food",
            icon: None,
        },
        245 => StatusEffectDefinition {
            name: "INT Food",
            icon: None,
        },
        246 => StatusEffectDefinition {
            name: "LUK Food",
            icon: None,
        },
        // Body/health state debuffs (EFST_HEALTHSTATE_*).
        875 => StatusEffectDefinition {
            name: "Stone Curse",
            icon: None,
        },
        876 => StatusEffectDefinition {
            name: "Freezing",
            icon: None,
        },
        877 => StatusEffectDefinition { name: "Stun", icon: None },
        878 => StatusEffectDefinition { name: "Sleep", icon: None },
        881 => StatusEffectDefinition {
            name: "Burning",
            icon: None,
        },
        883 => StatusEffectDefinition {
            name: "Poison",
            icon: Some("effect\\hl_poison.tga"),
        },
        884 => StatusEffectDefinition {
            name: "Curse",
            icon: Some("effect\\hl_curse.tga"),
        },
        885 => StatusEffectDefinition {
            name: "Silence",
            icon: Some("effect\\hl_silence.tga"),
        },
        886 => StatusEffectDefinition {
            name: "Confusion",
            icon: Some("effect\\hl_confusion.tga"),
        },
        887 => StatusEffectDefinition {
            name: "Blind",
            icon: Some("effect\\hl_state_fear.tga"),
        },
        889 => StatusEffectDefinition {
            name: "Bleeding",
            icon: None,
        },
        890 => StatusEffectDefinition {
            name: "Heavy Poison",
            icon: Some("effect\\hl_heavypoison.tga"),
        },
        891 => StatusEffectDefinition {
            name: "Fear",
            icon: Some("effect\\hl_state_fear.tga"),
        },
        _ => UNKNOWN_DEFINITION,
    }
}

/// One active status effect of the player.
#[derive(Clone, Default, RustState, StateElement)]
pub struct StatusEffect {
    /// Entity (account id) the status effect belongs to.
    entity_id: u32,
    status_index: u16,
    name: String,
    #[hidden_element]
    icon: Option<Arc<Texture>>,
    /// Total duration in milliseconds, `0` if the server did not send one.
    total_duration_in_milliseconds: u32,
    /// Remaining duration at [`Self::started_at_client_tick`].
    remaining_in_milliseconds: u32,
    started_at_client_tick: u64,
}

impl StatusEffect {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn icon(&self) -> Option<&Arc<Texture>> {
        self.icon.as_ref()
    }

    /// Whether the server sent a (positive) duration for this status effect.
    pub fn has_timer(&self) -> bool {
        self.total_duration_in_milliseconds > 0 && self.remaining_in_milliseconds > 0
    }

    /// Remaining milliseconds at the given client tick; `None` if the status
    /// effect has no timer (it then stays until the server sends the end).
    pub fn remaining_in_milliseconds(&self, client_tick: u64) -> Option<u32> {
        if !self.has_timer() {
            return None;
        }

        let elapsed = client_tick.saturating_sub(self.started_at_client_tick);
        Some(self.remaining_in_milliseconds.saturating_sub(elapsed.min(u32::MAX as u64) as u32))
    }

    /// Remaining seconds (rounded up) at the given client tick; `None` if the
    /// status effect has no timer.
    pub fn remaining_in_seconds(&self, client_tick: u64) -> Option<u32> {
        self.remaining_in_milliseconds(client_tick)
            .map(|milliseconds| milliseconds.div_ceil(1000))
    }

    /// Whether the status effect expired locally (only for statuses with a
    /// timer; statuses without one are ended by the server).
    pub fn is_expired(&self, client_tick: u64) -> bool {
        self.remaining_in_milliseconds(client_tick) == Some(0)
    }
}

/// All active status effects of the player.
#[derive(Default, RustState, StateElement)]
pub struct StatusEffects {
    #[hidden_element]
    effects: Vec<StatusEffect>,
    /// Latest client tick, used by the buff bar to count down the timers.
    current_client_tick: u64,
}

impl StatusEffects {
    /// Start, refresh or end a status effect.
    ///
    /// The name and icon are only used when the status effect is (re)started;
    /// the caller resolves them from the client data.
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        entity_id: u32,
        status_index: u16,
        active: bool,
        total_duration_in_milliseconds: u32,
        remaining_in_milliseconds: u32,
        client_tick: u64,
        name: String,
        icon: Option<Arc<Texture>>,
    ) {
        let existing = self
            .effects
            .iter_mut()
            .position(|effect| effect.entity_id == entity_id && effect.status_index == status_index);

        if !active {
            if let Some(index) = existing {
                self.effects.remove(index);
            }
            return;
        }

        match existing {
            Some(index) => {
                let effect = &mut self.effects[index];
                effect.name = name;
                effect.icon = icon;
                effect.total_duration_in_milliseconds = total_duration_in_milliseconds;
                effect.remaining_in_milliseconds = remaining_in_milliseconds;
                effect.started_at_client_tick = client_tick;
            }
            None => self.effects.push(StatusEffect {
                entity_id,
                status_index,
                name,
                icon,
                total_duration_in_milliseconds,
                remaining_in_milliseconds,
                started_at_client_tick: client_tick,
            }),
        }
    }

    /// Removes expired status effects. Called every frame; local expiry is only
    /// a safety net for statuses whose end packet was lost.
    pub fn update(&mut self, client_tick: u64) {
        self.effects.retain(|effect| !effect.is_expired(client_tick));
    }

    /// Updates the client tick used to render the countdown of the buff bar.
    pub fn set_client_tick(&mut self, client_tick: u64) {
        self.current_client_tick = client_tick;
    }

    pub fn current_client_tick(&self) -> u64 {
        self.current_client_tick
    }

    /// Removes all status effects (map change, leaving the game).
    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn effects(&self) -> &[StatusEffect] {
        &self.effects
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusEffects, status_effect_definition};

    #[test]
    fn known_buffs_have_names_and_icons() {
        assert_eq!(status_effect_definition(10).name, "Blessing");
        assert_eq!(status_effect_definition(12).name, "Increase AGI");
        assert_eq!(status_effect_definition(9).name, "Angelus");
        assert_eq!(status_effect_definition(3).name, "Improve Concentration");
        assert!(status_effect_definition(10).icon_path().is_some());
        assert!(status_effect_definition(12).icon_path().is_some());
    }

    #[test]
    fn unknown_statuses_still_get_a_name() {
        assert_eq!(status_effect_definition(30_000).name, "Status effect");
        assert!(status_effect_definition(30_000).icon_path().is_none());
    }

    fn apply(effects: &mut StatusEffects, index: u16, active: bool, total: u32, remaining: u32, tick: u64) {
        let definition = status_effect_definition(index);
        effects.apply(1, index, active, total, remaining, tick, definition.name.to_owned(), None);
    }

    #[test]
    fn start_refresh_and_end_a_status_effect() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 10, true, 60_000, 60_000, 0);

        assert_eq!(effects.len(), 1);
        assert_eq!(effects.effects()[0].name(), "Blessing");
        assert_eq!(effects.effects()[0].remaining_in_seconds(30_000), Some(30));

        // A refresh resets the timer and doesn't duplicate the entry.
        apply(&mut effects, 10, true, 60_000, 45_000, 30_000);
        assert_eq!(effects.len(), 1);
        assert_eq!(effects.effects()[0].remaining_in_seconds(30_000), Some(45));

        // Ending the status removes it immediately.
        apply(&mut effects, 10, false, 0, 0, 31_000);
        assert!(effects.is_empty());
    }

    #[test]
    fn statuses_without_timers_stay_until_the_server_ends_them() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 2, true, 0, 0, 0);

        assert_eq!(effects.len(), 1);
        assert!(!effects.effects()[0].has_timer());
        assert_eq!(effects.effects()[0].remaining_in_seconds(10_000_000), None);
        effects.update(10_000_000);
        assert_eq!(effects.len(), 1);

        apply(&mut effects, 2, false, 0, 0, 10_000_000);
        assert!(effects.is_empty());
    }

    #[test]
    fn expired_statuses_are_removed() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 12, true, 10_000, 10_000, 0);

        effects.update(9_000);
        assert_eq!(effects.len(), 1);

        effects.update(10_000);
        assert!(effects.is_empty());
    }

    #[test]
    fn clear_removes_everything() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 10, true, 60_000, 60_000, 0);
        apply(&mut effects, 12, true, 60_000, 60_000, 0);

        effects.clear();
        assert!(effects.is_empty());
    }
}
