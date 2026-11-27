//! Client-side monster database.
//!
//! The database is built at startup from data that is actually in the client
//! archives:
//!
//! * the navigation monster table (`navi_mob_krpri.lub`) provides the English
//!   display name, the Aegis name, the level, the MVP flag and every spawn map,
//! * the job identity table (`jobidentity.lub`/`npcidentity.lub`) provides the
//!   sprite identity for every monster job id, so monsters that are missing
//!   from the navigation table still appear by their sprite name,
//! * the resolved monster sprite path (`monster_sprite.rs`) is shown in the
//!   detail line.
//!
//! **Drop tables are not available.** The client archives contain no monster
//! database (`data.grf` was checked: no `mob_db`, `mobinfo` or
//! `mob_db.yml`/`.txt` file), so drop items and drop rates are server data and
//! are intentionally not shown. The window labels the missing columns instead
//! of inventing values.

use hashbrown::HashMap;
use ragnarok_packets::JobId;

use super::JobIdentity;
use super::navigation::{NavigationData, NavigationKind};
use crate::world::library::monster_sprite::is_monster_job_id;

/// Maximum number of search results returned to the window.
pub const MAX_SEARCH_RESULTS: usize = 80;

/// One monster of the database.
#[derive(Clone, Debug)]
pub struct MonsterEntry {
    pub job_id: JobId,
    pub name: String,
    pub aegis_name: String,
    /// Level from the navigation data, `None` when the entry only comes from
    /// the job identity table.
    pub level: Option<u16>,
    pub mvp: bool,
    /// Maps the monster spawns on, in navigation data order.
    pub maps: Vec<String>,
    /// Resolved sprite path relative to `data\sprite\`.
    pub sprite: String,
    lower_name: String,
    lower_aegis: String,
    lower_maps: String,
}

impl MonsterEntry {
    /// Short location text: first maps plus a remainder counter.
    pub fn location_summary(&self) -> String {
        const SHOWN_MAPS: usize = 3;

        if self.maps.is_empty() {
            return "unknown (not in the navigation data)".to_owned();
        }

        let mut summary = self.maps.iter().take(SHOWN_MAPS).cloned().collect::<Vec<_>>().join(", ");

        if self.maps.len() > SHOWN_MAPS {
            summary.push_str(&format!(" (+{} more)", self.maps.len() - SHOWN_MAPS));
        }

        summary
    }

    /// Label of one search result row.
    pub fn label(&self) -> String {
        let level = match self.level {
            Some(level) => format!("Lv {level}"),
            None => "Lv ?".to_owned(),
        };

        let mut label = format!("#{} {}  {level}", self.job_id.0, self.name);

        if self.mvp {
            label.push_str("  MVP");
        }

        if !self.aegis_name.is_empty() {
            label.push_str(&format!("  ({})", self.aegis_name));
        }

        label.push_str(&format!("  - {}", self.location_summary()));

        label
    }

    /// Full detail text for the selected monster.
    pub fn detail_text(&self) -> String {
        let level = match self.level {
            Some(level) => level.to_string(),
            None => "unknown".to_owned(),
        };
        let maps = match self.maps.is_empty() {
            true => "unknown".to_owned(),
            false => self.maps.join(", "),
        };

        format!(
            "#{}  {}\nAegis name: {}\nLevel: {level}\nHP: unknown (needs the server mob DB)\nMVP/Boss: {}\nSprite: {}\nMaps: \
             {maps}\nDrops: not available without the server mob DB",
            self.job_id.0,
            self.name,
            match self.aegis_name.is_empty() {
                true => "unknown",
                false => &self.aegis_name,
            },
            match self.mvp {
                true => "yes",
                false => "no",
            },
            self.sprite,
        )
    }
}

/// Monster database, indexed by mob/job id.
#[derive(Default)]
pub struct MonsterDatabase {
    entries: Vec<MonsterEntry>,
    by_id: HashMap<JobId, usize>,
}

impl MonsterDatabase {
    /// Builds the database from the navigation monster table plus the job
    /// identity table. The navigation table wins for names/levels; job
    /// identities fill in monsters that the navigation data does not list.
    ///
    /// `sprite_path` resolves the verified sprite path of a job id; it is a
    /// closure so this module does not depend on the library loader order.
    pub fn build(navigation: &NavigationData, job_identities: &HashMap<JobId, JobIdentity>, sprite_path: impl Fn(JobId) -> String) -> Self {
        let mut entries: HashMap<JobId, MonsterEntry> = HashMap::new();

        for destination in navigation.all() {
            if destination.kind != NavigationKind::Monster {
                continue;
            }

            let Some(job_id) = destination.job_id else {
                continue;
            };

            let entry = entries.entry(job_id).or_insert_with(|| MonsterEntry {
                job_id,
                name: destination.name.clone(),
                aegis_name: destination.detail.clone(),
                level: destination.level,
                mvp: destination.mvp,
                maps: Vec::new(),
                sprite: sprite_path(job_id),
                lower_name: String::new(),
                lower_aegis: String::new(),
                lower_maps: String::new(),
            });

            if entry.name.is_empty() {
                entry.name = destination.name.clone();
            }
            if entry.aegis_name.is_empty() {
                entry.aegis_name = destination.detail.clone();
            }
            if entry.level.is_none() {
                entry.level = destination.level;
            }
            entry.mvp |= destination.mvp;

            if !destination.map_name.is_empty() && !entry.maps.contains(&destination.map_name) {
                entry.maps.push(destination.map_name.clone());
            }
        }

        for (job_id, identity) in job_identities {
            if !is_monster_job_id(*job_id) || entries.contains_key(job_id) {
                continue;
            }

            entries.insert(*job_id, MonsterEntry {
                job_id: *job_id,
                name: identity.to_string(),
                aegis_name: String::new(),
                level: None,
                mvp: false,
                maps: Vec::new(),
                sprite: sprite_path(*job_id),
                lower_name: String::new(),
                lower_aegis: String::new(),
                lower_maps: String::new(),
            });
        }

        let mut entries: Vec<MonsterEntry> = entries.into_values().collect();

        for entry in &mut entries {
            entry.lower_name = entry.name.to_lowercase();
            entry.lower_aegis = entry.aegis_name.to_lowercase();
            entry.lower_maps = entry.maps.join(",").to_lowercase();
        }

        entries.sort_by(|left, right| {
            left.lower_name
                .cmp(&right.lower_name)
                .then_with(|| left.job_id.0.cmp(&right.job_id.0))
        });

        let by_id = entries.iter().enumerate().map(|(index, entry)| (entry.job_id, index)).collect();

        crate::logging::log_info!("monster database: {} monsters", entries.len());

        Self { entries, by_id }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn entry(&self, job_id: JobId) -> Option<&MonsterEntry> {
        self.by_id.get(&job_id).map(|index| &self.entries[*index])
    }

    /// Case-insensitive search over name, Aegis name and spawn maps.
    pub fn search(&self, query: &str) -> Vec<&MonsterEntry> {
        let query = query.trim().to_lowercase();

        if query.is_empty() {
            return Vec::new();
        }

        let level_query = query.strip_prefix("lv ").or_else(|| query.strip_prefix("level "));

        let mut matches: Vec<(u8, &MonsterEntry)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let rank = match (
                    entry.lower_name.starts_with(&query),
                    entry.lower_name.contains(&query),
                    entry.lower_aegis.contains(&query) || entry.lower_maps.contains(&query),
                ) {
                    (true, ..) => 0,
                    (_, true, _) => 1,
                    (_, _, true) => 2,
                    _ => {
                        if let (Some(level), Some(level_query)) = (entry.level, level_query)
                            && level_query.parse::<u16>().ok() == Some(level)
                        {
                            3
                        } else {
                            return None;
                        }
                    }
                };

                Some((rank, entry))
            })
            .collect();

        matches.sort_by(|(left_rank, left), (right_rank, right)| {
            left_rank
                .cmp(right_rank)
                .then_with(|| left.lower_name.cmp(&right.lower_name))
                .then_with(|| left.job_id.0.cmp(&right.job_id.0))
        });

        matches.into_iter().take(MAX_SEARCH_RESULTS).map(|(_, entry)| entry).collect()
    }
}

#[cfg(test)]
mod tests {
    use hashbrown::HashMap;
    use ragnarok_packets::JobId;

    use super::MonsterDatabase;
    use crate::world::library::navigation::NavigationData;

    #[test]
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
    fn monster_database_loads_from_the_archives() {

        use crate::loaders::GameFileLoader;
        use crate::world::library::Library;

        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
        game_file_loader.load_patched_lua_files();

        // Keep the patched archive on disk until the library finished reading
        // it: the 7z archive opens its file lazily on every lookup.
        let library = Library::new(&game_file_loader).expect("failed to load the game library");
        let database = library.monster_database();

        println!("monsters: {}", database.len());
        assert!(database.len() > 1000, "monster database looks empty: {} rows", database.len());

        let poring = database.search("poring");
        println!("search 'poring': {} hits", poring.len());
        for entry in poring.iter().take(5) {
            println!("{}", entry.label());
        }
        assert!(!poring.is_empty());

        if let Some(entry) = database.entry(JobId(1002)) {
            println!("1002 detail:\n{}", entry.detail_text());
        }

        // 19428 is the classic client view id used by the navigation table for
        // Poring; it must carry the level and spawn maps from `navi_mob`.
        if let Some(entry) = database.entry(JobId(19428)) {
            println!("19428: {}", entry.label());
            assert_eq!(entry.level, Some(1));
            assert!(!entry.maps.is_empty());
        } else {
            panic!("navigation Poring view id 19428 is missing from the monster database");
        }

        if !lua_archive_existed {
            game_file_loader.remove_patched_lua_files();
        }
    }

    #[test]
    fn empty_inputs_produce_an_empty_database() {
        let database = MonsterDatabase::build(&NavigationData::default(), &HashMap::new(), |_| String::new());

        assert_eq!(database.len(), 0);
        assert!(database.search("poring").is_empty());
        assert!(database.entry(JobId(1002)).is_none());
    }
}
