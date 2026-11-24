//! Monster sprite path resolution.
//!
//! The kRO data set used by this fork is partially trimmed: some monster
//! identities have no `data\sprite\몬스터\<identity>.spr/.act` (a few only
//! exist in `data\sprite\npc`). Rendering those monsters with the vanilla
//! identity path produces the black `missing.spr` placeholder, so this module
//! resolves a monster sprite with a fallback chain while the library is loaded:
//!
//! 1. exact `몬스터\<identity>` (normal ro-next behavior),
//! 2. the generated, verified alias table (`aliases.ron` via
//!    `library/aliases.rs`),
//! 3. the same identity in the `npc` sprite folder,
//! 4. a generic monster sprite that is verified to exist.
//!
//! Every path is checked against the game archives (`.spr` and `.act` must both
//! exist) when the library is built, so entity creation can use the resolved
//! path directly without doing file lookups on the main thread.

use hashbrown::HashMap;
use ragnarok_packets::JobId;

use super::{JobIdentity, aliases};
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
        let generic_candidates = aliases::generic_monster_sprites();
        let generic = generic_candidates
            .iter()
            .find(|path| sprite_exists(game_file_loader, path))
            .copied()
            .unwrap_or_else(|| generic_candidates.first().copied().unwrap_or("몬스터\\poring"))
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

    if let Some(alias) = aliases::monster_sprite_alias(job_id)
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

pub(crate) fn is_monster_job_id(job_id: JobId) -> bool {
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
    use ro_next_loaders::FileLoader;
    use ragnarok_bytes::{ByteReader, FromBytes};
    use ragnarok_formats::action::ActionsData;
    use ragnarok_formats::sprite::SpriteData;
    use ragnarok_formats::version::GenericFormatMetadata;
    use ragnarok_packets::JobId;

    use super::super::Library;
    use super::{aliases, sprite_exists};
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
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
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
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
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

    /// The monster ids that spawn in the Geffenia dungeon (`gefenia01`-`04`,
    /// from rAthena `npc/pre-re/mobs/dungeons/gefenia.txt`).
    const GEFFENIA_MONSTER_IDS: &[(u16, &str)] = &[
        // npc/pre-re/mobs/dungeons/gefenia.txt
        (1371, "FALSE_ANGEL"),
        (1370, "SUCCUBUS"),
        (1374, "INCUBUS"),
        (1390, "VIOLY"),
        (1292, "MINI_DEMON"),
        (1219, "ABYSMAL_KNIGHT"),
        (1268, "BLOODY_KNIGHT"),
        (1203, "MYSTELTAINN"),
        (1205, "EXECUTIONER"),
        (1204, "OGRETOOTH"),
        // Event/quest spawns that can appear on gefenia01-04 (RWC, christmas,
        // halloween, the sign quest and the tournament scripts).
        (1244, "CHIMERA"),
        (1259, "GRYPHON"),
        (1283, "DARK_LORD"),
        (1289, "DARK_ILLUSION"),
        (1295, "ELDER"),
        (1302, "CAT_O_NINE_TAIL"),
        (1307, "MA_ERO"),
        (1365, "APOCALYPSE"),
        (1377, "ELDER"),
        (1388, "ARCHANGELING"),
        (1502, "MAYA"),
        (1812, "LUDE"),
        (2248, "GOLDEN_PORING"),
        (2379, "SOCK_STEALING_RACCOON"),
        (2380, "GIFT_STEALING_RACCOON"),
        (2621, "ELUSIVE_VIOLY"),
        (2622, "ELUSIVE_SUCCUBUS"),
        (2623, "ELUSIVE_INCUBUS"),
        (2834, "SWEET_DREAMING_ANGEL"),
        (3202, "ORGANIC_PUMPKIN"),
    ];

    /// The animation loader composes one `Animation` per action of the body
    /// pair. Every `Animation` must contain at least one frame, otherwise
    /// `AnimationData::get_frame` would compute `frame % 0` (or index into an
    /// empty frame list) and panic while the entity is rendered. It also
    /// divides by the length of the delays list, which must not be empty.
    ///
    /// This test runs the same checks on the parsed action data of every
    /// rAthena monster (and prints the Geffenia monsters explicitly, since
    /// those are the maps that were reported as crashing).
    #[test]
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
    fn mob_animations_have_frames() {
        let game_file_loader = load_game_file_loader();
        let library = Library::new(&game_file_loader).expect("failed to load the game library");

        let mut failures: Vec<String> = Vec::new();

        let check_mob = |id: u16, aegis_name: &str, failures: &mut Vec<String>| {
            let job_id = JobId(id);
            let path = library.monster_sprite_path(job_id);
            let sprite_path = format!("data\\sprite\\{path}.spr").to_lowercase();
            let action_path = format!("data\\sprite\\{path}.act").to_lowercase();
            let sprite_bytes = game_file_loader.get(&sprite_path).expect("failed to read sprite");
            let action_bytes = game_file_loader.get(&action_path).expect("failed to read actions");

            let sprite_data = SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&sprite_bytes))
                .unwrap_or_else(|error| panic!("{id} {aegis_name}: invalid sprite {sprite_path}: {error:?}"));
            let texture_count = sprite_data.palette_image_data.len() + sprite_data.rgba_image_data.len();

            let actions_data = ActionsData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&action_bytes))
                .unwrap_or_else(|error| panic!("{id} {aegis_name}: invalid actions {action_path}: {error:?}"));

            // Same defaulting as `ActionLoader`: old act files have no delays.
            let delay_count = actions_data
                .delays
                .as_ref()
                .map(|delays| delays.len())
                .unwrap_or(actions_data.actions.len());

            // Same frame count as the animation loader: motions without sprite
            // clips are skipped and do not produce a frame.
            let frame_counts: Vec<usize> = actions_data
                .actions
                .iter()
                .map(|action| action.motions.iter().filter(|motion| motion.sprite_clip_count > 0).count())
                .collect();

            if actions_data.actions.is_empty() {
                failures.push(format!("{id} {aegis_name}: action file has no actions ({action_path})"));
            }

            // A motion whose clips all reference missing textures (or -1) is
            // merged into a placeholder frame part with `usize::MAX` indices,
            // which used to panic when rendered (out of bounds index).
            for (action_index, action) in actions_data.actions.iter().enumerate() {
                for (motion_index, motion) in action.motions.iter().enumerate() {
                    if motion.sprite_clip_count == 0 {
                        continue;
                    }

                    let valid_clips = motion
                        .sprite_clips
                        .iter()
                        .filter(|clip| {
                            if clip.sprite_number < 0 {
                                return false;
                            }

                            let mut sprite_number = clip.sprite_number as usize;
                            if clip.sprite_type.unwrap_or(0) == 1 {
                                sprite_number += sprite_data.palette_image_data.len();
                            }

                            sprite_number < texture_count
                        })
                        .count();

                    if valid_clips == 0 {
                        failures.push(format!(
                            "{id} {aegis_name}: action {action_index} motion {motion_index} has no valid sprite clip ({action_path})"
                        ));
                    }
                }
            }

            if delay_count == 0 {
                failures.push(format!("{id} {aegis_name}: action file has no delays ({action_path})"));
            }

            for (action_index, frame_count) in frame_counts.iter().enumerate() {
                if *frame_count == 0 {
                    failures.push(format!("{id} {aegis_name}: action {action_index} has no frame ({action_path})"));
                }
            }

            (path.to_string(), frame_counts)
        };

        let mut geffenia_failures: Vec<String> = Vec::new();

        for (id, aegis_name) in GEFFENIA_MONSTER_IDS {
            let before = failures.len();
            let (path, frame_counts) = check_mob(*id, aegis_name, &mut failures);
            println!("geffenia {id} {aegis_name} -> {path}: actions={frame_counts:?}");

            geffenia_failures.extend(failures[before..].iter().cloned());
        }

        for line in MOB_IDS.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let (id, aegis_name) = line.split_once('\t').expect("malformed mob line");
            check_mob(id.parse().expect("malformed mob id"), aegis_name, &mut failures);
        }

        if !failures.is_empty() {
            println!("all-mob animation issues ({}):", failures.len());
            for failure in &failures {
                println!("  {failure}");
            }
        }

        assert!(
            geffenia_failures.is_empty(),
            "geffenia mobs with unusable animations:\n{}",
            geffenia_failures.join("\n")
        );
    }

    #[test]
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
    fn alias_table_entries_exist_in_archives() {
        let game_file_loader = load_game_file_loader();

        for (id, path) in aliases::monster_sprite_aliases() {
            assert!(
                sprite_exists(&game_file_loader, path),
                "alias {id} points to a missing sprite: {path}"
            );
        }

        for path in aliases::generic_monster_sprites() {
            assert!(sprite_exists(&game_file_loader, path), "generic sprite does not exist: {path}");
        }
    }
}
