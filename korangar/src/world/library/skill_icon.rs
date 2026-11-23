//! Skill icon path resolution.
//!
//! Skill icons live in `data\sprite\아이템\<file_name>` where `file_name` comes
//! from the skill info tables. The icon set of this (trimmed/custom) data set
//! only covers part of the skill list, so some skills currently render the
//! black `npc\missing.spr` placeholder.
//!
//! This module resolves a verified icon path while the library is loaded:
//!
//! 1. exact `아이템\<file_name>` sprite (normal korangar behavior),
//! 2. the generated, verified alias table (`aliases.ron` via
//!    `library/aliases.rs`),
//! 3. a generic skill icon that is verified to exist.
//!
//! The resolved paths are cached so the async skill loader does not have to do
//! file lookups for every skill tree / hotbar entry.

use hashbrown::HashMap;
use ragnarok_packets::SkillId;

use super::{SkillListInformation, aliases};
use crate::loaders::GameFileLoader;

pub struct SkillIcons {
    table: HashMap<SkillId, String>,
    generic: String,
}

impl SkillIcons {
    pub fn load(game_file_loader: &GameFileLoader, skill_information_table: &HashMap<SkillId, SkillListInformation>) -> Self {
        let generic_candidates = aliases::generic_skill_icons();
        let generic = generic_candidates
            .iter()
            .find(|path| sprite_exists(game_file_loader, path))
            .copied()
            .unwrap_or_else(|| generic_candidates.first().copied().unwrap_or("아이템\\nv_basic"))
            .to_string();

        let mut table = HashMap::with_capacity(skill_information_table.len());

        for (skill_id, skill_information) in skill_information_table {
            table.insert(
                *skill_id,
                resolve(game_file_loader, *skill_id, &skill_information.file_name, &generic),
            );
        }

        Self { table, generic }
    }

    /// Returns an icon path relative to `data\sprite\` (without extension).
    pub fn path(&self, skill_id: SkillId) -> &str {
        self.table.get(&skill_id).map(String::as_str).unwrap_or(&self.generic)
    }
}

fn resolve(game_file_loader: &GameFileLoader, skill_id: SkillId, file_name: &str, generic: &str) -> String {
    let exact_path = format!("아이템\\{file_name}");

    // The icon itself only needs the `.spr` file; the action file is used for the
    // animation preview, so a missing `.act` is not a reason to replace the icon.
    if sprite_exists(game_file_loader, &exact_path) {
        return exact_path;
    }

    if let Some(alias) = aliases::skill_icon_alias(skill_id)
        && sprite_exists(game_file_loader, alias)
    {
        return alias.to_string();
    }

    generic.to_string()
}

/// Checks whether the `.spr` file of an icon path exists in the archives.
///
/// `GameFileLoader::file_exists` (like the game archives themselves) expects
/// lower case paths with `\` separators.
fn sprite_exists(game_file_loader: &GameFileLoader, path: &str) -> bool {
    let sprite_path = format!("data\\sprite\\{path}.spr").to_lowercase();

    game_file_loader.file_exists(&sprite_path)
}

#[cfg(test)]
mod tests {
    use korangar_loaders::FileLoader;
    use ragnarok_bytes::{ByteReader, FromBytes};
    use ragnarok_formats::action::ActionsData;
    use ragnarok_formats::sprite::SpriteData;
    use ragnarok_formats::version::GenericFormatMetadata;
    use ragnarok_packets::SkillId;

    use super::super::Library;
    use super::{aliases, sprite_exists};
    use crate::loaders::GameFileLoader;

    /// All skills from the skill info tables (id, skill name, icon file name
    /// per line).
    const SKILL_IDS: &str = include_str!("test_data/skill_ids.txt");

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
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn all_skill_ids_resolve_to_existing_icons() {
        let game_file_loader = load_game_file_loader();
        let library = Library::new(&game_file_loader).expect("failed to load the game library");

        let mut count = 0;

        for line in SKILL_IDS.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut fields = line.split('\t');
            let id = fields.next().expect("malformed skill line");
            let skill_name = fields.next().expect("malformed skill line");
            let file_name = fields.next().expect("malformed skill line");
            let skill_id = SkillId(id.parse().expect("malformed skill id"));
            let path = library.skill_icon_path(skill_id);

            assert!(
                !path.to_lowercase().contains("missing"),
                "{id} {skill_name} (icon {file_name}) resolved to the missing sprite placeholder: {path}"
            );
            assert!(
                sprite_exists(&game_file_loader, path),
                "{id} {skill_name} (icon {file_name}) resolved to an icon that does not exist: {path}"
            );

            // The icon sprite must parse; the action file is only used for skill previews.
            let sprite_path = format!("data\\sprite\\{path}.spr").to_lowercase();
            let sprite_bytes = game_file_loader.get(&sprite_path).expect("failed to read icon sprite");
            SpriteData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&sprite_bytes))
                .unwrap_or_else(|error| panic!("{id} {skill_name}: invalid icon {sprite_path}: {error:?}"));

            let action_path = format!("data\\sprite\\{path}.act").to_lowercase();
            if let Ok(action_bytes) = game_file_loader.get(&action_path) {
                ActionsData::from_bytes(&mut ByteReader::with_default_metadata::<GenericFormatMetadata>(&action_bytes))
                    .unwrap_or_else(|error| panic!("{id} {skill_name}: invalid actions {action_path}: {error:?}"));
            }

            count += 1;
        }

        assert_eq!(count, 1229, "expected all 1229 skills in the fixture");
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn reported_missing_skill_icons_resolve_to_the_expected_aliases() {
        let game_file_loader = load_game_file_loader();
        let library = Library::new(&game_file_loader).expect("failed to load the game library");

        // Representative entries from the audit: NPC_ skill copies are mapped to their
        // player-skill icons, element spirits / Doram / pet skills use documented
        // fallbacks.
        for (id, expected) in [
            (720u16, "아이템\\wl_jackfrost"),  // NPC_JACKFROST -> Wizard frost skill icon
            (740, "아이템\\rk_ignitionbreak"), // NPC_IGNITIONBREAK
            (8414, "아이템\\mg_firebolt"),     // EL_HEATER (element fallback)
            (545, "아이템\\nv_basic"),         // MB_FIGHTING (no pet skill icons in data set)
            (2055, "아이템\\ab_duplelight"),   // AB_DUPLELIGHT_MELEE
        ] {
            assert_eq!(library.skill_icon_path(SkillId(id)), expected, "unexpected icon for {id}");
        }
    }

    #[test]
    #[ignore = "requires the game archives (run from the korangar crate root, client/game_archives.ron)"]
    fn alias_table_entries_exist_in_archives() {
        let game_file_loader = load_game_file_loader();

        for (id, path) in aliases::skill_icon_aliases() {
            assert!(
                sprite_exists(&game_file_loader, path),
                "alias {id} points to a missing icon: {path}"
            );

            let action_path = format!("data\\sprite\\{path}.act").to_lowercase();
            assert!(
                game_file_loader.get(&action_path).is_ok(),
                "alias {id} points to an icon without action file: {path}"
            );
        }

        for path in aliases::generic_skill_icons() {
            assert!(sprite_exists(&game_file_loader, path), "generic icon does not exist: {path}");
        }
    }
}
