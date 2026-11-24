//! Monster database window state.
//!
//! The database itself (names, levels, spawn maps, sprite path) is loaded from
//! client data by [`crate::world::library::monster_database`] and passed to the
//! window as a shared table. This module only keeps the search field and the
//! last Sense result (`ZC_MONSTER_INFO`, 0x018C).
//!
//! Drop tables are **not** available: the client archives contain no monster
//! database (`data.grf` has no `mob_db`/`mobinfo` file), so drop rates and item
//! drops require the server DB and are intentionally not shown.

use korangar_interface::element::StateElement;
use ragnarok_packets::JobId;
use rust_state::RustState;

/// Result of a Sense (`WZ_ESTIMATION`) cast.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SenseInfo {
    pub job_id: JobId,
    pub level: u16,
    /// rAthena `status.size` (0 = small, 1 = medium, 2 = large).
    pub size: u16,
    pub health_points: u32,
    pub defense: i16,
    pub race: u16,
    pub magic_defense: i16,
    /// rAthena `status.def_ele` (element id, see `element_name`).
    pub element: u16,
    /// Elemental damage percentages in the packet order: water, earth, fire,
    /// wind, poison, holy, shadow, ghost, undead.
    pub resistances: [u8; 9],
}

impl SenseInfo {
    pub fn size_name(&self) -> &'static str {
        match self.size {
            0 => "Small",
            1 => "Medium",
            2 => "Large",
            _ => "Unknown",
        }
    }

    pub fn element_name(&self) -> &'static str {
        element_name(self.element)
    }

    pub fn race_name(&self) -> &'static str {
        race_name(self.race)
    }

    /// One line with the elemental resistances in packet order.
    pub fn resistance_summary(&self) -> String {
        const ELEMENTS: [&str; 9] = ["Water", "Earth", "Fire", "Wind", "Poison", "Holy", "Shadow", "Ghost", "Undead"];

        ELEMENTS
            .iter()
            .zip(self.resistances.iter())
            .map(|(name, value)| format!("{name} {value}%"))
            .collect::<Vec<_>>()
            .join("  ")
    }
}

/// rAthena element ids (`enum e_element`).
pub fn element_name(element: u16) -> &'static str {
    match element {
        0 => "Neutral",
        1 => "Water",
        2 => "Earth",
        3 => "Fire",
        4 => "Wind",
        5 => "Poison",
        6 => "Holy",
        7 => "Shadow",
        8 => "Ghost",
        9 => "Undead",
        _ => "Unknown",
    }
}

/// rAthena race ids (`enum e_race`).
pub fn race_name(race: u16) -> &'static str {
    match race {
        0 => "Formless",
        1 => "Undead",
        2 => "Brute",
        3 => "Plant",
        4 => "Insect",
        5 => "Fish",
        6 => "Demon",
        7 => "Demi-Human",
        8 => "Angel",
        9 => "Dragon",
        _ => "Unknown",
    }
}

/// Search field and Sense result of the monster database window.
#[derive(Default, RustState, StateElement)]
pub struct MonsterDatabaseState {
    /// Search text of the monster database window.
    pub query: String,
    /// Last Sense result, shown at the top of the window.
    #[hidden_element]
    sense: Option<SenseInfo>,
    /// Status line (e.g. "Sense received for ...").
    #[hidden_element]
    status: String,
    /// Monster id selected in the search result list, used to show the detail
    /// view for the entry that was clicked.
    #[hidden_element]
    selected: Option<JobId>,
}

impl MonsterDatabaseState {
    pub fn search(&self) -> &str {
        &self.query
    }

    pub fn sense(&self) -> Option<&SenseInfo> {
        self.sense.as_ref()
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn selected(&self) -> Option<JobId> {
        self.selected
    }

    pub fn set_sense(&mut self, info: SenseInfo) {
        self.status = format!("Sense: monster #{} (level {})", info.job_id.0, info.level);
        self.selected = Some(info.job_id);
        self.sense = Some(info);
    }

    pub fn set_selected(&mut self, job_id: JobId) {
        self.selected = Some(job_id);
    }

    pub fn clear(&mut self) {
        self.sense = None;
        self.selected = None;
        self.status.clear();
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::JobId;

    use super::{SenseInfo, element_name, race_name};

    #[test]
    fn sense_names_cover_the_server_enums() {
        assert_eq!(element_name(9), "Undead");
        assert_eq!(element_name(42), "Unknown");
        assert_eq!(race_name(7), "Demi-Human");

        let sense = SenseInfo {
            job_id: JobId(1002),
            level: 1,
            size: 1,
            health_points: 55,
            defense: 0,
            race: 3,
            magic_defense: 0,
            element: 2,
            resistances: [100, 50, 100, 100, 100, 100, 100, 100, 100],
        };

        assert_eq!(sense.size_name(), "Medium");
        assert_eq!(sense.race_name(), "Plant");
        assert_eq!(sense.element_name(), "Earth");
        assert!(sense.resistance_summary().starts_with("Water 100%  Earth 50%"));
    }
}
