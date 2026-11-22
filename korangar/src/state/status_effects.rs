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
//! This module keeps the active status effects of all entities in the view and
//! their timers in the [`ClientState`](crate::state::ClientState) so that the
//! buff bar can render the player's effects and count them down locally between
//! packets. Effects of other entities are tracked as well (the server sends
//! area status packets); the buff bar only displays the player's own effects.

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

/// Element ids as sent by the 2019 server for elemental weapon enchantments.
const ELEMENT_NAMES: [&str; 10] = [
    "Neutral", "Water", "Earth", "Fire", "Wind", "Poison", "Holy", "Dark", "Ghost", "Undead",
];

/// Format the server-side values of a status effect for the buff tooltip.
///
/// Most status effects do not send values (`value1`..`value3` are zero). The
/// handful that do use them for a meaningful property; those are labelled
/// here (the status indices are the `EFST_*` ids of the client data). For every
/// other status a generic value line is shown when the server sent a non-zero
/// value, so that the data is never silently dropped.
///
/// The status ids with values are the ones rAthena marks with `SendVal*` in
/// its status change database: weapon enchant (element), cash exp/item/job-exp
/// boosts, push cart and monster transform.
pub fn status_effect_value_lines(status_index: u16, values: [i32; 3]) -> Vec<String> {
    if values.iter().all(|value| *value == 0) {
        return Vec::new();
    }

    let first_value = values[0];

    match status_index {
        // EFST_WEAPONPROPERTY (`Enchantarms`): value 1 is the element.
        64 => {
            let element = usize::try_from(first_value)
                .ok()
                .and_then(|element| ELEMENT_NAMES.get(element))
                .copied()
                .unwrap_or("Unknown");

            vec![format!("Element: {element}")]
        }
        // EFST_CASH_PLUSEXP (`Expboost`): value 1 is the boost percentage.
        250 => vec![format!("Base EXP boost: {first_value}%")],
        // EFST_CASH_RECEIVEITEM (`Itemboost`): value 1 is the boost percentage.
        252 => vec![format!("Item drop boost: {first_value}%")],
        // EFST_CASH_PLUSONLYJOBEXP (`Jexpboost`): value 1 is the boost
        // percentage.
        312 => vec![format!("Job EXP boost: {first_value}%")],
        // EFST_ON_PUSH_CART (`Push_Cart`): value 1 is the cart type.
        673 => vec![format!("Push cart: {first_value}")],
        // EFST_MONSTER_TRANSFORM / EFST_ACTIVE_MONSTER_TRANSFORM: value 1 is
        // the monster job id.
        621 | 665 => vec![format!("Monster transform: #{first_value}")],
        _ => {
            let values = values
                .iter()
                .filter(|value| **value != 0)
                .map(i32::to_string)
                .collect::<Vec<_>>()
                .join(" / ");

            vec![format!("Value: {values}")]
        }
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
    /// Server-side status values (`val1`..`val3`), zero for statuses that do
    /// not send any.
    values: [i32; 3],
    started_at_client_tick: u64,
}

impl StatusEffect {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn entity_id(&self) -> u32 {
        self.entity_id
    }

    pub fn status_index(&self) -> u16 {
        self.status_index
    }

    /// The server-side status values (`val1`..`val3`).
    pub fn values(&self) -> [i32; 3] {
        self.values
    }

    /// Display lines of the status-specific values, shown in the buff tooltip.
    pub fn value_lines(&self) -> Vec<String> {
        status_effect_value_lines(self.status_index(), self.values())
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
        values: [i32; 3],
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
                effect.values = values;
                effect.started_at_client_tick = client_tick;
            }
            None => self.effects.push(StatusEffect {
                entity_id,
                status_index,
                name,
                icon,
                total_duration_in_milliseconds,
                remaining_in_milliseconds,
                values,
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

    /// Whether the given entity has no active status effects.
    ///
    /// The state tracks the effects of every entity in the view, so the buff
    /// bar has to ask for the player's own entity specifically when deciding
    /// whether it should be visible.
    pub fn is_empty_for(&self, entity_id: u32) -> bool {
        !self.effects.iter().any(|effect| effect.entity_id == entity_id)
    }
}

#[cfg(test)]
mod tests {
    use super::{StatusEffects, status_effect_definition, status_effect_value_lines};

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
        apply_with_values(effects, index, active, total, remaining, [0; 3], tick);
    }

    fn apply_with_values(effects: &mut StatusEffects, index: u16, active: bool, total: u32, remaining: u32, values: [i32; 3], tick: u64) {
        let definition = status_effect_definition(index);
        effects.apply(
            1,
            index,
            active,
            total,
            remaining,
            values,
            tick,
            definition.name.to_owned(),
            None,
        );
    }

    #[test]
    fn start_refresh_and_end_a_status_effect() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 10, true, 60_000, 60_000, 0);

        assert_eq!(effects.effects().len(), 1);
        assert_eq!(effects.effects()[0].name(), "Blessing");
        assert_eq!(effects.effects()[0].remaining_in_seconds(30_000), Some(30));

        // A refresh resets the timer and doesn't duplicate the entry.
        apply(&mut effects, 10, true, 60_000, 45_000, 30_000);
        assert_eq!(effects.effects().len(), 1);
        assert_eq!(effects.effects()[0].remaining_in_seconds(30_000), Some(45));

        // Ending the status removes it immediately.
        apply(&mut effects, 10, false, 0, 0, 31_000);
        assert!(effects.effects().is_empty());
    }

    #[test]
    fn statuses_without_timers_stay_until_the_server_ends_them() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 2, true, 0, 0, 0);

        assert_eq!(effects.effects().len(), 1);
        assert!(!effects.effects()[0].has_timer());
        assert_eq!(effects.effects()[0].remaining_in_seconds(10_000_000), None);
        effects.update(10_000_000);
        assert_eq!(effects.effects().len(), 1);

        apply(&mut effects, 2, false, 0, 0, 10_000_000);
        assert!(effects.effects().is_empty());
    }

    #[test]
    fn expired_statuses_are_removed() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 12, true, 10_000, 10_000, 0);

        effects.update(9_000);
        assert_eq!(effects.effects().len(), 1);

        effects.update(10_000);
        assert!(effects.effects().is_empty());
    }

    #[test]
    fn clear_removes_everything() {
        let mut effects = StatusEffects::default();
        apply(&mut effects, 10, true, 60_000, 60_000, 0);
        apply(&mut effects, 12, true, 60_000, 60_000, 0);

        effects.clear();
        assert!(effects.effects().is_empty());
    }

    #[test]
    fn status_values_are_stored_and_updated() {
        let mut effects = StatusEffects::default();
        apply_with_values(&mut effects, 64, true, 60_000, 60_000, [3, 0, 0], 0);

        assert_eq!(effects.effects()[0].values(), [3, 0, 0]);
        assert_eq!(effects.effects()[0].value_lines(), vec!["Element: Fire".to_owned()]);

        // A refresh with new values updates the stored values.
        apply_with_values(&mut effects, 64, true, 60_000, 50_000, [1, 0, 0], 10_000);
        assert_eq!(effects.effects().len(), 1);
        assert_eq!(effects.effects()[0].values(), [1, 0, 0]);
    }

    #[test]
    fn status_value_lines_label_known_statuses() {
        assert_eq!(status_effect_value_lines(64, [3, 0, 0]), vec!["Element: Fire".to_owned()]);
        assert_eq!(status_effect_value_lines(64, [99, 0, 0]), vec!["Element: Unknown".to_owned()]);
        assert_eq!(status_effect_value_lines(250, [50, 0, 0]), vec![
            "Base EXP boost: 50%".to_owned()
        ]);
        assert_eq!(status_effect_value_lines(252, [10, 0, 0]), vec![
            "Item drop boost: 10%".to_owned()
        ]);
        assert_eq!(status_effect_value_lines(312, [25, 0, 0]), vec![
            "Job EXP boost: 25%".to_owned()
        ]);
        assert_eq!(status_effect_value_lines(621, [1002, 0, 0]), vec![
            "Monster transform: #1002".to_owned()
        ]);
    }

    #[test]
    fn status_value_lines_fall_back_to_raw_values() {
        assert_eq!(status_effect_value_lines(1234, [5, 0, 7]), vec!["Value: 5 / 7".to_owned()]);
        assert!(status_effect_value_lines(1234, [0, 0, 0]).is_empty());
    }

    #[test]
    fn statuses_of_other_entities_are_kept_separately() {
        let mut effects = StatusEffects::default();
        let definition = status_effect_definition(10);
        effects.apply(1, 10, true, 60_000, 60_000, [0; 3], 0, definition.name.to_owned(), None);
        effects.apply(2, 10, true, 60_000, 60_000, [0; 3], 0, definition.name.to_owned(), None);

        assert_eq!(effects.effects().len(), 2);
        assert_eq!(effects.effects()[0].entity_id(), 1);
        assert_eq!(effects.effects()[1].entity_id(), 2);
    }
}
