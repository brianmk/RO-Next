//! Client-side navigation destination database.
//!
//! The official client ships the navigation data as Lua tables in the game
//! archives (`data\luafiles514\lua
//! files\navigation\navi_{map,npc,mob}_krpri.lub`). The client already unifies
//! the compiled `.lub` bytecode to Lua 5.1 at startup (see
//! `GameFileLoader::patch_lua_files`), so the tables can be read with the same
//! `mlua` state that the item/job data uses.
//!
//! Layout of the tables (the matching `navi_f_krpri.lua` query functions copy
//! the fields one to one, see the official client decompilation
//! `Source/Contents/Time/ClientTiming.cpp:150-225`):
//!
//! * `Navi_Map = { { map, display name, icon id, width, height }, ... }`
//! * `Navi_Npc = { { map, npc id, object type, sprite id, name, title, x, y },
//!   ... }`
//! * `Navi_Mob = { { map, mob id, object type, unknown, name, aegis name,
//!   level, trait code }, ... }` - the mob rows carry a map but no usable spawn
//!   cell (the official client only navigates to the map), so monster
//!   destinations are map-level.
//!
//! Everything is optional data: a server without the navigation Lua files
//! still gets an empty database and the rest of the client works unchanged.

use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::{JobId, TilePosition};

use crate::loaders::GameFileLoader;

const NAVI_MAP_FILE: &str = "data\\luafiles514\\lua files\\navigation\\navi_map_krpri.lub";
const NAVI_NPC_FILE: &str = "data\\luafiles514\\lua files\\navigation\\navi_npc_krpri.lub";
const NAVI_MOB_FILE: &str = "data\\luafiles514\\lua files\\navigation\\navi_mob_krpri.lub";

/// Maximum number of search results returned to the window.
pub const MAX_SEARCH_RESULTS: usize = 80;

/// Kind of a navigation destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationKind {
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

/// One searchable destination. This is a flattened view over the three Lua
/// tables so the window does not have to know their layout.
#[derive(Clone, Debug)]
pub struct NavigationDestination {
    pub kind: NavigationKind,
    pub name: String,
    /// Map resource name without the `.gat` suffix.
    pub map_name: String,
    /// Exact destination cell, if the data has one (NPCs).
    pub position: Option<TilePosition>,
    /// Monster level, if known.
    pub level: Option<u16>,
    /// MVP/Boss flag (mob object type `301`).
    pub mvp: bool,
    /// Extra text for the result label (aegis name, NPC title, map size).
    pub detail: String,
    /// Mob/job id for monster rows, `None` for maps and NPCs.
    pub job_id: Option<JobId>,
    lower_name: String,
    lower_map: String,
    lower_detail: String,
}

impl NavigationDestination {
    /// Label shown in the search result list.
    pub fn label(&self) -> String {
        let location = match self.position {
            Some(position) => format!("{} ({}, {})", self.map_name, position.x, position.y),
            None => self.map_name.clone(),
        };

        let mut label = format!("[{}] {} - {location}", self.kind.label(), self.name);

        if let Some(level) = self.level {
            label.push_str(&format!("  Lv {level}"));
        }

        if self.mvp {
            label.push_str("  MVP");
        }

        if !self.detail.is_empty() {
            label.push_str(&format!("  ({})", self.detail));
        }

        label
    }
}

/// Navigation data loaded from the client archives.
#[derive(Default)]
pub struct NavigationData {
    destinations: Vec<NavigationDestination>,
    /// Number of loaded map rows (for the startup log).
    map_count: usize,
    npc_count: usize,
    mob_count: usize,
}

impl NavigationData {
    pub fn load(game_file_loader: &GameFileLoader) -> Self {
        let Some(lua) = load_tables(game_file_loader) else {
            crate::logging::log_debug!("navigation: no navigation Lua tables in the game archives");
            return Self::default();
        };

        let mut data = Self::default();

        if let Ok(table) = lua.globals().get::<mlua::Table>("Navi_Map") {
            for row in table.sequence_values::<mlua::Table>().flatten() {
                let Some(map_name) = read_string(&row, 1) else {
                    continue;
                };
                let display_name = read_string(&row, 2).unwrap_or_else(|| map_name.clone());
                let width = read_u32(&row, 4).unwrap_or(0);
                let height = read_u32(&row, 5).unwrap_or(0);

                let detail = match width > 0 && height > 0 {
                    true => format!("{width}x{height}"),
                    false => String::new(),
                };

                data.destinations.push(NavigationDestination {
                    kind: NavigationKind::Map,
                    name: display_name,
                    map_name,
                    position: None,
                    level: None,
                    mvp: false,
                    detail,
                    job_id: None,
                    lower_name: String::new(),
                    lower_map: String::new(),
                    lower_detail: String::new(),
                });
                data.map_count += 1;
            }
        }

        if let Ok(table) = lua.globals().get::<mlua::Table>("Navi_Npc") {
            for row in table.sequence_values::<mlua::Table>().flatten() {
                let Some(map_name) = read_string(&row, 1) else {
                    continue;
                };
                let Some(name) = read_string(&row, 5) else {
                    continue;
                };
                let title = read_string(&row, 6).unwrap_or_default();
                let x = read_u32(&row, 7).unwrap_or(0);
                let y = read_u32(&row, 8).unwrap_or(0);

                if x == 0 || y == 0 {
                    continue;
                }

                let detail = match title.trim().is_empty() {
                    true => map_name.clone(),
                    false => format!("{title}, {map_name}"),
                };

                data.destinations.push(NavigationDestination {
                    kind: NavigationKind::Npc,
                    name,
                    map_name,
                    position: Some(TilePosition {
                        x: x.min(u16::MAX as u32) as u16,
                        y: y.min(u16::MAX as u32) as u16,
                    }),
                    level: None,
                    mvp: false,
                    detail,
                    job_id: None,
                    lower_name: String::new(),
                    lower_map: String::new(),
                    lower_detail: String::new(),
                });
                data.npc_count += 1;
            }
        }

        if let Ok(table) = lua.globals().get::<mlua::Table>("Navi_Mob") {
            for row in table.sequence_values::<mlua::Table>().flatten() {
                let Some(map_name) = read_string(&row, 1) else {
                    continue;
                };
                let Some(name) = read_string(&row, 5) else {
                    continue;
                };
                let aegis_name = read_string(&row, 6).unwrap_or_default();
                let level = read_u32(&row, 7).unwrap_or(0);
                // rAthena/clif mob object type: 300 = normal monster, 301 =
                // MVP/Boss. The 8th column is the packed element/size/race
                // code, which this database does not interpret.
                let object_type = read_u32(&row, 3).unwrap_or(0);
                let mob_id = read_u32(&row, 2).unwrap_or(0);

                data.destinations.push(NavigationDestination {
                    kind: NavigationKind::Monster,
                    name,
                    map_name,
                    position: None,
                    level: (level > 0).then_some(level.min(u16::MAX as u32) as u16),
                    mvp: object_type == 301,
                    detail: aegis_name,
                    job_id: (mob_id > 0 && mob_id <= u16::MAX as u32).then_some(JobId(mob_id as u16)),
                    lower_name: String::new(),
                    lower_map: String::new(),
                    lower_detail: String::new(),
                });
                data.mob_count += 1;
            }
        }

        for destination in &mut data.destinations {
            destination.lower_name = destination.name.to_lowercase();
            destination.lower_map = destination.map_name.to_lowercase();
            destination.lower_detail = destination.detail.to_lowercase();
        }

        crate::logging::log_info!(
            "navigation: loaded {} map, {} NPC and {} monster destinations",
            data.map_count,
            data.npc_count,
            data.mob_count
        );

        data
    }

    pub fn is_empty(&self) -> bool {
        self.destinations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.destinations.len()
    }

    pub fn all(&self) -> &[NavigationDestination] {
        &self.destinations
    }

    /// Short description shown above the search field.
    pub fn hint(&self) -> String {
        format!(
            "{} destinations loaded from the client navigation data. Select one to mark it on the minimap and draw the local route.",
            self.len()
        )
    }

    /// Case-insensitive search over name, map, aegis/title detail and level.
    ///
    /// Results are ordered by match quality (name prefix, name contains, other
    /// field contains) and category (maps, NPCs, monsters) so the common
    /// "type a town name" search puts the map first.
    pub fn search(&self, query: &str) -> Vec<&NavigationDestination> {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return Vec::new();
        }

        let level_query = query.strip_prefix("lv ").or_else(|| query.strip_prefix("level "));

        let mut matches: Vec<(u8, &NavigationDestination)> = self
            .destinations
            .iter()
            .filter_map(|destination| {
                let rank = match (
                    destination.lower_name.starts_with(&query),
                    destination.lower_name.contains(&query),
                    destination.lower_map.contains(&query) || destination.lower_detail.contains(&query),
                ) {
                    (true, ..) => 0,
                    (_, true, _) => 1,
                    (_, _, true) => 2,
                    _ => {
                        if let (Some(level), Some(level_query)) = (destination.level, level_query)
                            && level_query.parse::<u16>().ok() == Some(level)
                        {
                            3
                        } else {
                            return None;
                        }
                    }
                };

                Some((rank, destination))
            })
            .collect();

        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank.cmp(right_rank).then_with(|| {
                let left_kind = kind_order(left.kind);
                let right_kind = kind_order(right.kind);
                left_kind
                    .cmp(&right_kind)
                    .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
                    .then_with(|| left.map_name.cmp(&right.map_name))
            })
        });

        matches
            .into_iter()
            .take(MAX_SEARCH_RESULTS)
            .map(|(_, destination)| destination)
            .collect()
    }
}

fn kind_order(kind: NavigationKind) -> u8 {
    match kind {
        NavigationKind::Map => 0,
        NavigationKind::Npc => 1,
        NavigationKind::Monster => 2,
    }
}

/// Loads the three navigation Lua files into one Lua state. Missing files are
/// not an error (a server may ship a trimmed archive).
fn load_tables(game_file_loader: &GameFileLoader) -> Option<Lua> {
    let lua = Lua::new();
    let mut loaded = 0;

    for file in [NAVI_MAP_FILE, NAVI_NPC_FILE, NAVI_MOB_FILE] {
        if !game_file_loader.file_exists(file) {
            continue;
        }

        let Ok(bytes) = game_file_loader.get(file) else {
            continue;
        };

        match lua.load(&bytes).exec() {
            Ok(()) => loaded += 1,
            Err(error) => crate::logging::log_debug!("navigation: failed to load {file}: {error}"),
        }
    }

    (loaded > 0).then_some(lua)
}

fn read_string(table: &mlua::Table, index: i64) -> Option<String> {
    let value = table.get::<Option<String>>(index).ok().flatten()?;
    let value = value.trim().to_owned();

    (!value.is_empty()).then_some(value)
}

fn read_u32(table: &mlua::Table, index: i64) -> Option<u32> {
    let value = table.get::<Option<i64>>(index).ok().flatten()?;
    u32::try_from(value).ok()
}

#[cfg(test)]
mod tests {
    use korangar_loaders::FileLoader;

    use super::NavigationData;
    use crate::loaders::GameFileLoader;

    /// Builds a game file loader exactly like the client startup does.
    fn load_game_file_loader() -> GameFileLoader {
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
        game_file_loader.load_patched_lua_files();

        if !lua_archive_existed {
            game_file_loader.remove_patched_lua_files();
        }

        game_file_loader
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn navigation_tables_load_from_the_archives() {
        let game_file_loader = load_game_file_loader();
        let data = NavigationData::load(&game_file_loader);

        println!("navigation destinations: {}", data.len());

        assert!(data.len() > 1000, "navigation tables look empty: {} rows", data.len());

        let prontera = data
            .all()
            .iter()
            .find(|destination| destination.kind == super::NavigationKind::Map && destination.map_name == "prontera");
        assert!(prontera.is_some(), "prontera is missing from the map table");

        let poiring = data.search("poring");
        println!("search 'poring': {} hits", poiring.len());
        assert!(!poiring.is_empty());

        let kafra = data.search("kafra");
        println!("search 'kafra': {} hits", kafra.len());

        if let Some(npc) = kafra.iter().find(|destination| destination.kind == super::NavigationKind::Npc) {
            println!("first Kafra: {} on {} at {:?}", npc.name, npc.map_name, npc.position);
        }

        let mut maps = 0;
        let mut npcs = 0;
        let mut mobs = 0;

        for destination in data.all() {
            match destination.kind {
                super::NavigationKind::Map => maps += 1,
                super::NavigationKind::Npc => npcs += 1,
                super::NavigationKind::Monster => mobs += 1,
            }
        }

        println!("maps: {maps}, npcs: {npcs}, monsters: {mobs}");
    }
}
