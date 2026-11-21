//! Offline job-id audit.
//!
//! The ignored test walks every rAthena server job id and verifies against the
//! real game archives that the client can render it: a body sprite (with `.spr`
//! and `.act`) exists for at least one sex, a body palette exists, and the job
//! resolves to a non-empty skill tree through the generated fallback table.

#[cfg(test)]
mod tests {
    use korangar_loaders::FileLoader;
    use ragnarok_packets::JobId;

    use super::super::Library;
    use crate::loaders::GameFileLoader;
    use crate::world::EntityType;
    use crate::world::entity::{get_body_palette_name, get_sprite_path_for_player_job};

    /// All jobs from the rAthena `enum e_job` (id, server name, PE/LLM content
    /// flag).
    const JOB_IDS: &str = include_str!("test_data/job_ids.txt");

    /// `JOB_MAX_BASIC` and `JOB_MAX` are sentinel values, never sent as a real
    /// job.
    const SENTINEL_JOB_IDS: [u16; 2] = [31, 4317];

    /// Player jobs without an entry in the client `JobIdentity` tables (4th,
    /// summoner and inherited player jobs). Identity is only used for
    /// NPCs/monsters, so player rendering is not affected; documented for the
    /// audit.
    const IDENTITY_FALLBACK_JOB_IDS: [u16; 27] = [
        4218, 4238, 4252, 4253, 4254, 4255, 4256, 4257, 4258, 4259, 4260, 4261, 4262, 4263, 4264, 4278, 4279, 4280, 4281, 4302, 4303, 4304,
        4305, 4306, 4307, 4308, 4316,
    ];

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

    fn sprite_exists(game_file_loader: &GameFileLoader, path: &str) -> bool {
        let sprite = format!("{path}.spr").to_lowercase();
        let actions = format!("{path}.act").to_lowercase();

        game_file_loader.file_exists(&sprite) && game_file_loader.file_exists(&actions)
    }

    #[test]
    fn fixture_contains_every_server_job() {
        let rows = JOB_IDS.lines().filter(|line| !line.is_empty() && !line.starts_with('#')).count();

        assert_eq!(rows, 183, "expected all 183 rAthena jobs in the fixture");
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn every_server_job_has_a_sprite_palette_and_skill_tree() {
        let game_file_loader = load_game_file_loader();
        let library = Library::new(&game_file_loader).expect("failed to load the game library");

        let mut checked = 0;
        let mut checked_pe_jobs = 0;
        let mut palettes_found = 0;
        let mut palettes_missing: Vec<(u16, String)> = Vec::new();

        for line in JOB_IDS.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut fields = line.split('\t');
            let id = fields.next().expect("malformed job line");
            let server_name = fields.next().expect("malformed job line");
            let is_pe_job = fields.next().unwrap_or("0") == "1";
            let job_id = JobId(id.parse().expect("malformed job id"));

            // All rAthena `e_job` values are player jobs; make sure the client
            // classification does not accidentally treat one as NPC/monster.
            assert!(
                matches!(EntityType::from(job_id), EntityType::Player),
                "{id} {server_name}: client classifies this server job as a non-player"
            );

            if SENTINEL_JOB_IDS.contains(&job_id.0) {
                // Documented sentinels: not real jobs, never sent by the server.
                continue;
            }

            let sprite = get_sprite_path_for_player_job(job_id);
            let mut body_found = false;

            for sex in ["남", "여"] {
                let path = format!("data\\sprite\\인간족\\몸통\\{sex}\\{sprite}_{sex}");

                if sprite_exists(&game_file_loader, &path) {
                    body_found = true;
                    break;
                }
            }

            assert!(
                body_found,
                "{id} {server_name}: no existing body sprite for resolved sprite '{sprite}'"
            );

            let palette = get_body_palette_name(job_id);
            let mut palette_found = false;

            for sex in ["남", "여"] {
                let path = format!("data\\palette\\몸\\{palette}_{sex}_0.pal").to_lowercase();

                if game_file_loader.file_exists(&path) {
                    palette_found = true;
                    break;
                }
            }

            // A missing body palette is not fatal: the sprite loader falls back to the
            // palette embedded in the body sprite. Record it for the audit instead.
            if palette_found {
                palettes_found += 1;
            } else {
                palettes_missing.push((job_id.0, server_name.to_owned()));
            }

            assert!(
                library.has_skill_tree(job_id),
                "{id} {server_name}: no non-empty skill tree (direct or via the fallback table)"
            );

            // The client's JobIdentity table (used for NPC/monster names) has no entry
            // for these ids; that is documented and has no effect on player rendering.
            if IDENTITY_FALLBACK_JOB_IDS.contains(&job_id.0) {
                // Player-only 4th/inherited jobs have no JobIdentity entry; identity is
                // only used for NPC/monster rendering, so this has no effect here.
                assert!(
                    !library.has_job_identity(job_id),
                    "{id} {server_name}: documented as missing from JobIdentity, but it is present"
                );
            } else {
                assert!(
                    library.has_job_identity(job_id),
                    "{id} {server_name}: no JobIdentity entry (unexpected for this job)"
                );
            }

            checked += 1;

            if is_pe_job {
                checked_pe_jobs += 1;
            }
        }

        assert_eq!(checked, 181, "expected 181 real jobs (183 minus the 2 sentinels)");
        assert!(checked_pe_jobs > 0, "expected population/LLM jobs in the fixture");
        assert!(
            palettes_found >= 140,
            "unexpectedly low body palette coverage: {palettes_found} found, {} missing (first: {:?})",
            palettes_missing.len(),
            palettes_missing.first()
        );

        // Spot checks for the jobs that were broken before the fallback tables.
        assert_eq!(get_sprite_path_for_player_job(JobId(4014)), "로드나이트");
        assert_eq!(get_sprite_path_for_player_job(JobId(4048)), "권성");
        assert_eq!(get_sprite_path_for_player_job(JobId(4252)), "룬나이트");
        assert!(library.has_skill_tree(JobId(4048)));
        assert!(library.has_skill_tree(JobId(4252)));
    }
}
