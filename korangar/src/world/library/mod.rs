mod accessory_name;
mod baby_job;
mod item_info;
mod item_name;
mod item_resource;
mod item_stats;
mod job_identity;
mod map_sky_data;
mod monster_sprite;
mod monster_sprite_alias;
mod skill_icon;
mod skill_icon_alias;
mod skill_information;
mod skill_requirements;
mod skill_tree;

use std::hash::Hash;

use encoding_rs::EUC_KR;
use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::{JobId, Sex, SkillId};

pub use self::accessory_name::AccessoryName;
pub use self::baby_job::IsBabyJob;
pub use self::item_info::ItemInfo;
pub use self::item_name::{ItemName, ItemNameKey};
pub use self::item_resource::{ItemResource, ItemResourceKey};
pub use self::item_stats::{ItemStats, StatBonuses};
pub use self::job_identity::JobIdentity;
pub use self::map_sky_data::MapSkyData;
use self::monster_sprite::MonsterSprites;
use self::skill_icon::SkillIcons;
pub use self::skill_tree::SkillTreeLayout;
use crate::loaders::GameFileLoader;
pub use crate::world::library::skill_information::SkillListInformation;
pub use crate::world::library::skill_requirements::{SkillListKey, SkillListRequirements};

pub struct Library {
    job_identity_table: <JobIdentity as Table>::Storage,
    item_info_table: <ItemInfo as Table>::Storage,
    map_sky_data_table: <MapSkyData as Table>::Storage,
    skill_information_table: <SkillListInformation as Table>::Storage,
    skill_requirements_table: <SkillListRequirements as Table>::Storage,
    skill_tree_table: <SkillTreeLayout as Table>::Storage,
    baby_job_table: <IsBabyJob as Table>::Storage,
    monster_sprites: MonsterSprites,
    skill_icons: SkillIcons,
    accessory_name_table: <AccessoryName as Table>::Storage,
}

impl Library {
    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        let job_identity_table = JobIdentity::load(game_file_loader)?;
        let item_info_table = ItemInfo::load(game_file_loader)?;
        let map_sky_data_table = MapSkyData::load(game_file_loader)?;
        let skill_information_table = SkillListInformation::load(game_file_loader)?;
        let skill_requirements_table = SkillListRequirements::load(game_file_loader)?;
        let skill_tree_table = SkillTreeLayout::load(game_file_loader)?;
        let baby_job_table = IsBabyJob::load(game_file_loader)?;
        let monster_sprites = MonsterSprites::load(game_file_loader, &job_identity_table);
        let skill_icons = SkillIcons::load(game_file_loader, &skill_information_table);
        let accessory_name_table = AccessoryName::load(game_file_loader)?;

        Ok(Self {
            job_identity_table,
            item_info_table,
            map_sky_data_table,
            skill_information_table,
            skill_requirements_table,
            skill_tree_table,
            baby_job_table,
            monster_sprites,
            skill_icons,
            accessory_name_table,
        })
    }

    #[inline(always)]
    /// Whether the library has an entry for the given key. Useful to detect
    /// jobs that are not covered by this client's (trimmed) tables without
    /// falling back to a default value.
    pub fn contains<T: Table>(&self, key: T::Key<'_>) -> bool {
        T::try_get(self, key).is_some()
    }

    pub fn get<T: Table>(&self, key: T::Key<'_>) -> &T {
        T::get(self, key)
    }

    /// Sprite path (relative to `data\sprite\`, without extension) for a
    /// monster job id.
    ///
    /// The path was verified against the game archives when the library was
    /// loaded and is never the black `missing.spr` placeholder for monster
    /// ids.
    #[inline(always)]
    pub fn monster_sprite_path(&self, job_id: JobId) -> &str {
        self.monster_sprites.path(job_id)
    }

    /// Icon path (relative to `data\sprite\`, without extension) for a skill.
    ///
    /// The path was verified against the game archives when the library was
    /// loaded and is never the black `missing.spr` placeholder for skill
    /// ids.
    #[inline(always)]
    pub fn skill_icon_path(&self, skill_id: SkillId) -> &str {
        self.skill_icons.path(skill_id)
    }

    /// Sprite path (relative to `data\sprite\`, without extension) of a
    /// headgear accessory for the given view id and sex.
    ///
    /// Returns `None` if the view id is unknown (e.g. an item added by the
    /// server that is not part of the client data) or if the accessory has no
    /// sprite for the given sex. Callers then simply skip the accessory
    /// instead of trying to load a missing sprite.
    pub fn accessory_sprite_path(&self, view_id: u16, sex: Sex) -> Option<String> {
        if view_id == 0 {
            return None;
        }

        let accessory_name = self.accessory_name_table.get(&view_id)?;
        let sex_sprite_path = match sex {
            Sex::Female => "여",
            Sex::Male => "남",
            _ => return None,
        };

        Some(format!(
            "악세사리\\{sex_sprite_path}\\{sex_sprite_path}{}",
            accessory_name.name()
        ))
    }
}

/// Trait for compacting a hash map after it is completely populated.
trait HashMapExt {
    /// Compact the hash map, possibly by creating a second one.
    fn compact(self) -> Self;
}

impl<K, V> HashMapExt for HashMap<K, V>
where
    K: Eq + Hash,
{
    fn compact(self) -> Self {
        HashMap::from_iter(self)
    }
}

trait LuaExt: Sized {
    fn load_from_game_files(game_file_loader: &GameFileLoader, files: &[&str]) -> mlua::Result<Self>;
}

impl LuaExt for Lua {
    fn load_from_game_files(game_file_loader: &GameFileLoader, files: &[&str]) -> mlua::Result<Self> {
        let state = Lua::new();

        for file in files {
            let data = game_file_loader
                .get(file)
                .unwrap_or_else(|_| panic!("failed to open lua file {}", file));

            state.load(&data).exec()?;
        }

        Ok(state)
    }
}

/// Trait for data that can be stored in a table and retrieved using a key.
pub trait Table {
    type Key<'a>;
    type Storage;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage>;

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self>
    where
        Self: Sized;

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self
    where
        Self: Sized;
}

fn fix_encoding(broken: String) -> String {
    let bytes: Vec<u8> = broken.chars().map(|char| char as u8).collect();
    match EUC_KR.decode_without_bom_handling_and_without_replacement(&bytes) {
        None => broken.to_string(),
        Some(char) => char.to_string(),
    }
}
