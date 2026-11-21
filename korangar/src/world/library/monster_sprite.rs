//! Monster sprite path resolution.
//!
//! The kRO data set used by this fork is partially trimmed: some monster
//! identities have no `data\sprite\몬스터\<identity>.spr/.act` (a few only
//! exist in `data\sprite\npc`). Rendering those monsters with the vanilla
//! identity path produces the black `missing.spr` placeholder, so this module
//! resolves a monster sprite with a fallback chain while the library is loaded:
//!
//! 1. exact `몬스터\<identity>` (normal korangar behavior),
//! 2. the generated, verified alias table (`monster_sprite_alias.rs`),
//! 3. the same identity in the `npc` sprite folder,
//! 4. a generic monster sprite that is verified to exist.
//!
//! Every path is checked against the game archives (`.spr` and `.act` must both
//! exist) when the library is built, so entity creation can use the resolved
//! path directly without doing file lookups on the main thread.

use hashbrown::HashMap;
use ragnarok_packets::JobId;

use super::{JobIdentity, monster_sprite_alias};
use crate::loaders::GameFileLoader;

/// Job id range of monsters, mirroring `EntityType::from_job_id`.
const MONSTER_JOB_ID_MIN: u16 = 1000;
const MONSTER_JOB_ID_MAX: u16 = 3999;
const MONSTER_2_JOB_ID_MIN: u16 = 20000;
const MONSTER_2_JOB_ID_MAX: u16 = 29999;

pub struct MonsterSprites {
    table: HashMap<JobId, String>,
    generic: String,
}

impl MonsterSprites {
    pub fn load(game_file_loader: &GameFileLoader, job_identity_table: &HashMap<JobId, JobIdentity>) -> Self {
        let generic = monster_sprite_alias::GENERIC_MONSTER_SPRITES
            .iter()
            .find(|path| sprite_exists(game_file_loader, path))
            .copied()
            .unwrap_or(monster_sprite_alias::GENERIC_MONSTER_SPRITES[0])
            .to_string();

        let mut table = HashMap::with_capacity(job_identity_table.len());

        for (job_id, identity) in job_identity_table {
            if !is_monster_job_id(*job_id) {
                continue;
            }

            let identity = identity.to_string();
            let identity = identity.trim_end_matches('\0');

            table.insert(*job_id, resolve(game_file_loader, *job_id, identity, &generic));
        }

        Self { table, generic }
    }

    /// Returns a sprite path relative to `data\sprite\` (without extension).
    pub fn path(&self, job_id: JobId) -> &str {
        self.table.get(&job_id).map(String::as_str).unwrap_or(&self.generic)
    }
}

fn resolve(game_file_loader: &GameFileLoader, job_id: JobId, identity: &str, generic: &str) -> String {
    let exact_path = format!("몬스터\\{identity}");

    if sprite_exists(game_file_loader, &exact_path) {
        return exact_path;
    }

    if let Some(alias) = monster_sprite_alias::alias(job_id)
        && sprite_exists(game_file_loader, alias)
    {
        return alias.to_string();
    }

    let npc_path = format!("npc\\{identity}");

    if sprite_exists(game_file_loader, &npc_path) {
        return npc_path;
    }

    generic.to_string()
}

fn is_monster_job_id(job_id: JobId) -> bool {
    matches!(
        job_id.0,
        MONSTER_JOB_ID_MIN..=MONSTER_JOB_ID_MAX | MONSTER_2_JOB_ID_MIN..=MONSTER_2_JOB_ID_MAX
    )
}

/// Checks whether both the `.spr` and `.act` file of a sprite path exist in the
/// archives.
///
/// `GameFileLoader::file_exists` (like the game archives themselves) expects
/// lower case paths with `\` separators.
fn sprite_exists(game_file_loader: &GameFileLoader, path: &str) -> bool {
    let sprite_path = format!("data\\sprite\\{path}.spr").to_lowercase();
    let action_path = format!("data\\sprite\\{path}.act").to_lowercase();

    game_file_loader.file_exists(&sprite_path) && game_file_loader.file_exists(&action_path)
}

#[cfg(test)]
mod tests {
    use korangar_loaders::FileLoader;
    use ragnarok_bytes::{ByteReader, FromBytes};
    use ragnarok_formats::action::ActionsData;
    use ragnarok_formats::sprite::SpriteData;
    use ragnarok_formats::version::GenericFormatMetadata;
    use ragnarok_packets::JobId;

    use super::super::Library;
    use super::{monster_sprite_alias, sprite_exists};
    use crate::loaders::GameFileLoader;

    /// All mob ids from the rAthena mob db (id, aegis name per line).
    const MOB_IDS: &str = include_str!("test_data/mob_ids.txt");

    /// Builds a game file loader exactly like the client startup does: the
    /// `lub` bytecode of the game archives has to be upcast to Lua 5.1
    /// first (see `patch_lua_files`).
    ///
    /// The generated `lua_files.7z` is only a local, transient build artifact;
    /// it is removed again if the test created it.
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
    fn monster_job_id_ranges_match_entity_type() {
        use super::{MONSTER_2_JOB_ID_MAX, MONSTER_2_JOB_ID_MIN, MONSTER_JOB_ID_MAX, MONSTER_JOB_ID_MIN, is_monster_job_id};

        assert!(is_monster_job_id(JobId(MONSTER_JOB_ID_MIN)));
        assert!(is_monster_job_id(JobId(MONSTER_JOB_ID_MAX)));
        assert!(is_monster_job_id(JobId(MONSTER_2_JOB_ID_MIN)));
        assert!(is_monster_job_id(JobId(MONSTER_2_JOB_ID_MAX)));
        assert!(!is_monster_job_id(JobId(MONSTER_JOB_ID_MIN - 1)));
        assert!(!is_monster_job_id(JobId(MONSTER_JOB_ID_MAX + 1)));
        assert!(!is_monster_job_id(JobId(MONSTER_2_JOB_ID_MAX + 1)));
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn all_mob_ids_resolve_to_existing_sprites() {
        let game_file_loader = load_game_file_loader();
        let library = Library::new(&game_file_loader).expect("failed to load the game library");

        let mut count = 0;

        for line in MOB_IDS.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (id, aegis_name) = line.split_once('\t').expect("malformed mob line");
            let job_id = JobId(id.parse().expect("malformed mob id"));
            let path = library.monster_sprite_path(job_id);

            assert!(
                !path.to_lowercase().contains("missing"),
                "{id} {aegis_name} resolved to the missing sprite placeholder: {path}"
            );
            assert!(
                sprite_exists(&game_file_loader, path),
                "{id} {aegis_name} resolved to a sprite that does not exist: {path}"
            );

            // The sprite and action files must not only exist, they must also parse:
            // this is the same data the client loads at runtime.
            let sprite_path = format!("data\\sprite\\{path}.spr").to_lowercase();
            let action_path = format!("data\\sprite\\{path}.act").to_lowercase();
            let sprite_bytes = game_file_loader.get(&sprite_path).expect("failed to read sprite");
            let action_bytes = game_file_loader.get(&action_path).expect("failed to read actions");

            SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&sprite_bytes))
                .unwrap_or_else(|error| panic!("{id} {aegis_name}: invalid sprite {sprite_path}: {error:?}"));
            ActionsData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&action_bytes))
                .unwrap_or_else(|error| panic!("{id} {aegis_name}: invalid actions {action_path}: {error:?}"));

            count += 1;
        }

        assert_eq!(count, 1008, "expected all 1008 mob ids in the fixture");
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn reported_missing_monsters_resolve_to_the_expected_aliases() {
        let game_file_loader = load_game_file_loader();
        let library = Library::new(&game_file_loader).expect("failed to load the game library");

        // The user report that started this work: Archer Skeleton was rendered as the
        // black missing sprite. The generated table maps it to the closest existing
        // skeleton sprite; SOLDIER_SKELETON and VERIT are the other examples from the
        // audit.
        for (id, expected) in [
            (1016u16, "몬스터\\skel_archer"),
            (1028, "몬스터\\skel_soldier"),
            (1032, "몬스터\\mutant_verit"),
            (1047, "npc\\pecopeco_egg"),
            (1182, "몬스터\\red_mushroom"),
        ] {
            assert_eq!(library.monster_sprite_path(JobId(id)), expected, "unexpected alias for {id}");
        }
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn alias_table_entries_exist_in_archives() {
        let game_file_loader = load_game_file_loader();

        for (id, path) in monster_sprite_alias::MONSTER_SPRITE_ALIASES {
            assert!(
                sprite_exists(&game_file_loader, path),
                "alias {id} points to a missing sprite: {path}"
            );
        }

        for path in monster_sprite_alias::GENERIC_MONSTER_SPRITES {
            assert!(sprite_exists(&game_file_loader, path), "generic sprite does not exist: {path}");
        }
    }
}
