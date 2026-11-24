//! Navigation route helpers: map-name normalisation, the nearest walkable
//! tile search and the destination/route bookkeeping.
use crate::*;

impl Client {
    /// Normalize a map resource name for the navigation data: no `.gat`
    /// suffix, lower case (the navigation Lua tables use bare map names).
    pub(crate) fn normalize_navigation_map_name(map_name: &str) -> String {
        map_name.trim().trim_end_matches(".gat").to_ascii_lowercase()
    }

    /// Handle a `<NAVI>` click: mark the target and draw the local route.
    pub(crate) fn navigate_to_map_position(&mut self, map_name: &str, x: u16, y: u16) {
        self.set_navigation_destination(None, map_name, Some(TilePosition { x, y }));
    }

    /// Nearest walkable cell around `center`, used for map-level navigation
    /// targets such as the map center.
    pub(crate) fn nearest_walkable_tile(map: &Map, center: TilePosition, max_radius: u16) -> TilePosition {
        if map.is_walkable(center) {
            return center;
        }

        let max_radius = max_radius as i32;

        for radius in 1..=max_radius {
            for delta_y in -radius..=radius {
                for delta_x in -radius..=radius {
                    // Only inspect the ring at this radius.
                    if delta_x.abs() != radius && delta_y.abs() != radius {
                        continue;
                    }

                    let candidate = TilePosition {
                        x: (center.x as i32 + delta_x).clamp(0, u16::MAX as i32) as u16,
                        y: (center.y as i32 + delta_y).clamp(0, u16::MAX as i32) as u16,
                    };

                    if map.is_walkable(candidate) {
                        return candidate;
                    }
                }
            }
        }

        center
    }

    /// Marks a navigation destination and computes the walkable route with the
    /// existing [`PathFinder`] (see `world/pathing.rs`).
    ///
    /// The 2019 server has no client-side navigation packet, so the route is
    /// purely local: it is drawn on the minimap and the player still has to
    /// walk/warp there. For destinations on another map only the target is
    /// recorded (the client has no cross-map walking).
    pub(crate) fn set_navigation_destination(&mut self, display_name: Option<String>, map_name: &str, position: Option<TilePosition>) {
        let target_map = Self::normalize_navigation_map_name(map_name);
        let current_map = self.current_map_name.as_deref().map(Self::normalize_navigation_map_name);
        let is_current_map = current_map.as_deref() == Some(target_map.as_str());

        let name = display_name.unwrap_or_else(|| target_map.clone());
        let kind = match position.is_some() {
            true => crate::state::navigation::NavigationKind::Npc,
            false => crate::state::navigation::NavigationKind::Map,
        };

        let mut destination = crate::state::navigation::NavigationTarget {
            kind,
            name: name.clone(),
            map_name: target_map.clone(),
            position,
        };

        let mut route = Vec::new();
        let status;

        if is_current_map {
            let goal = match position {
                Some(position) => Some(position),
                None => self.map.as_ref().map(|map| {
                    let (width, height) = map.get_dimensions();
                    let center = TilePosition {
                        x: width / 2,
                        y: height / 2,
                    };

                    Self::nearest_walkable_tile(map, center, 12)
                }),
            };

            match goal {
                Some(goal) => {
                    destination.position = Some(goal);

                    let player_position = self.client_state.try_follow(this_entity()).map(|entity| entity.get_tile_position());

                    match (self.map.as_ref(), player_position) {
                        (Some(map), Some(start)) => match self.path_finder.find_navigation_path(&**map, start, goal) {
                            Some(path) => {
                                route = path.to_vec();
                                status = format!("Navigation: {} at {},{} - {} tiles", name, goal.x, goal.y, route.len());
                            }
                            None => {
                                status = format!("Navigation: no walkable route to {name} at {},{}", goal.x, goal.y);
                            }
                        },
                        _ => status = format!("Navigation: {name} at {},{} - map/player not ready yet", goal.x, goal.y),
                    }
                }
                None => status = format!("Navigation: map {target_map} is not loaded"),
            }
        } else {
            status = format!(
                "Navigation: {name} is on map {}. The client only draws the local route; use a warp NPC or @go to get there.",
                target_map
            );
        }

        self.client_state
            .follow_mut(client_state().navigation())
            .set_destination(destination, route, is_current_map, status.clone());

        if is_current_map && !self.interface.is_window_with_class_open(WindowClass::Minimap) {
            self.interface.open_window(MinimapWindow);
            self.map_window_cycle = MapWindowCycle::Minimap;
        }

        self.push_chat_information(status);
    }

    /// Recomputes the local route after the player moved (called on every tile
    /// change from the minimap update).
    pub(crate) fn refresh_navigation_route(&mut self) {
        let destination = match self.client_state.follow(client_state().navigation()).destination() {
            Some(destination) => destination.clone(),
            None => return,
        };

        let current_map = self.current_map_name.as_deref().map(Self::normalize_navigation_map_name);

        if current_map.as_deref() != Some(destination.map_name.as_str()) {
            return;
        }

        let (Some(goal), Some(map)) = (destination.position, self.map.as_ref()) else {
            return;
        };
        let Some(start) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_tile_position()) else {
            return;
        };

        // While the player walks along the current route there is nothing to
        // recompute; only a deviation (or a new destination) triggers a new
        // A* search. This keeps the per-tile update cheap.
        if self.client_state.follow(client_state().navigation()).route().contains(&start) {
            return;
        }

        let navigation = self.client_state.follow_mut(client_state().navigation());

        match self.path_finder.find_navigation_path(&**map, start, goal) {
            Some(path) => {
                let route = path.to_vec();
                navigation.set_route(route, format!("Route: {} tiles", path.len()));
            }
            None => navigation.set_route(Vec::new(), "No walkable route to the destination.".to_owned()),
        }
    }

    /// Clears the navigation destination and route.
    pub(crate) fn clear_navigation(&mut self) {
        self.client_state.follow_mut(client_state().navigation()).clear();
    }

    /// Re-evaluates the navigation target after a map finished loading.
    pub(crate) fn on_navigation_map_loaded(&mut self, map_name: &str) {
        let destination = self.client_state.follow(client_state().navigation()).destination().cloned();
        let Some(destination) = destination else {
            return;
        };

        if Self::normalize_navigation_map_name(map_name) == destination.map_name {
            self.set_navigation_destination(Some(destination.name), &destination.map_name, destination.position);
        } else {
            self.client_state.follow_mut(client_state().navigation()).clear_route();
        }
    }
}
