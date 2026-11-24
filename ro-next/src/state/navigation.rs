//! Client-side navigation state.
//!
//! The 2019 server has no navigation packet (`ZC_NAVIGATION_ACTIVE`/the older
//! 0x08E2 navigation opcode are not part of the 2019 table; the server's only
//! navigation helper, `clif_navigateTo`, is a script command that writes the
//! legacy 0x08E2 layout, which this client does not receive), so navigation is
//! implemented entirely on the client:
//!
//! * the destination database is loaded from the official client's navigation
//!   Lua tables (`world/library/navigation.rs`),
//! * the route is computed with the existing [`crate::world::pathing`] A*
//!   implementation and drawn on the minimap,
//! * `/navi <map> <x> <y>` and clicking a search result mark the destination.
//!
//! This module only holds the UI-facing state; the route is recomputed in
//! `lib.rs` whenever the player moves or a new destination is selected.

use ragnarok_packets::TilePosition;
use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// Kind of a navigation destination. The label is used by the search window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, RustState, StateElement)]
#[allow(dead_code)] // `Monster` is part of the shared kind enum but the current
// `/navi`/`<NAVI>` entry points only create map and NPC targets.
pub enum NavigationKind {
    #[default]
    Map,
    Npc,
    Monster,
}

impl NavigationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Map => "Map",
            Self::Npc => "NPC",
            Self::Monster => "Monster",
        }
    }
}

/// One selected navigation target.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavigationTarget {
    pub kind: NavigationKind,
    /// Display name of the target ("Prontera", "Kafra", "Poring").
    pub name: String,
    /// Map resource name without the `.gat` suffix.
    pub map_name: String,
    /// Destination cell. `None` for map-level targets (maps and monsters,
    /// whose navigation data carries a map but no usable spawn coordinate).
    pub position: Option<TilePosition>,
}

impl NavigationTarget {
    /// One-line description for the navigation window / chat.
    pub fn description(&self) -> String {
        match self.position {
            Some(position) => format!(
                "{} {} on {} ({}, {})",
                self.kind.label(),
                self.name,
                self.map_name,
                position.x,
                position.y
            ),
            None => format!("{} {} on map {}", self.kind.label(), self.name, self.map_name),
        }
    }
}

/// Navigation destination, local route and search field.
#[derive(Default, RustState, StateElement)]
pub struct Navigation {
    /// Search text of the navigation window.
    pub query: String,
    /// Destination currently marked on the minimap.
    #[hidden_element]
    destination: Option<NavigationTarget>,
    /// Walkable route from the player to the destination on the current map.
    #[hidden_element]
    route: Vec<TilePosition>,
    /// Status line shown in the window and pushed to the chat.
    #[hidden_element]
    status: String,
    /// Whether the destination is on the currently loaded map.
    #[hidden_element]
    on_current_map: bool,
}

impl Navigation {
    pub fn search(&self) -> &str {
        &self.query
    }

    pub fn destination(&self) -> Option<&NavigationTarget> {
        self.destination.as_ref()
    }

    pub fn route(&self) -> &[TilePosition] {
        &self.route
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn is_on_current_map(&self) -> bool {
        self.on_current_map
    }

    /// Mark `target` as destination and replace the local route.
    pub fn set_destination(&mut self, target: NavigationTarget, route: Vec<TilePosition>, on_current_map: bool, status: String) {
        self.destination = Some(target);
        self.route = route;
        self.on_current_map = on_current_map;
        self.status = status;
    }

    /// Replace only the route, keeping the destination (player moved).
    pub fn set_route(&mut self, route: Vec<TilePosition>, status: String) {
        self.route = route;
        self.status = status;
    }

    /// Clear destination, route and status.
    pub fn clear(&mut self) {
        self.destination = None;
        self.route.clear();
        self.on_current_map = false;
        self.status.clear();
    }

    /// Drop the route (e.g. on a map change) but keep the target so the window
    /// can still show what was selected.
    pub fn clear_route(&mut self) {
        self.route.clear();
        self.on_current_map = false;
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::TilePosition;

    use super::{Navigation, NavigationTarget};

    #[test]
    fn destination_and_route_can_be_updated_independently() {
        let mut navigation = Navigation::default();
        assert!(navigation.destination().is_none());

        navigation.set_destination(
            NavigationTarget {
                kind: super::NavigationKind::Map,
                name: "Prontera".to_owned(),
                map_name: "prontera".to_owned(),
                position: Some(TilePosition { x: 155, y: 183 }),
            },
            vec![TilePosition { x: 150, y: 180 }, TilePosition { x: 155, y: 183 }],
            true,
            "route ready".to_owned(),
        );

        assert_eq!(navigation.route().len(), 2);
        assert!(navigation.is_on_current_map());

        navigation.set_route(Vec::new(), "no route".to_owned());
        assert_eq!(navigation.destination().unwrap().name, "Prontera");
        assert!(navigation.route().is_empty());

        navigation.clear();
        assert!(navigation.destination().is_none());
        assert!(navigation.route().is_empty());
    }
}
