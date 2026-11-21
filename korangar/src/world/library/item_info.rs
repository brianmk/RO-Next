use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::ItemId;

use super::{HashMapExt, ItemName, ItemResource, ItemStats, Library, LuaExt, Table, fix_encoding};
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub(super) identified_name: ItemName,
    pub(super) unidentified_name: ItemName,
    pub(super) identified_resource: ItemResource,
    pub(super) unidentified_resource: ItemResource,
    /// Stats evaluated from the identified item description.
    pub(super) stats: ItemStats,
}

impl Table for ItemInfo {
    type Key<'a> = ItemId;
    type Storage = HashMap<ItemId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &["data\\luafiles514\\lua files\\datainfo\\iteminfo.lub"])?;

        let globals = state.globals();
        let mut result = HashMap::new();

        if let Ok(table) = globals.get::<mlua::Table>("tbl") {
            for (item_id, item_table) in table.pairs::<u32, mlua::Table>().flatten() {
                let identified_description = item_table
                    .get::<mlua::Table>("identifiedDescriptionName")
                    .ok()
                    .map(|description| {
                        description
                            .sequence_values::<String>()
                            .flatten()
                            .map(fix_encoding)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                let info = ItemInfo {
                    identified_name: ItemName::from_option(item_table.get("identifiedDisplayName").ok().map(fix_encoding)),
                    unidentified_name: ItemName::from_option(item_table.get("unidentifiedDisplayName").ok().map(fix_encoding)),
                    identified_resource: ItemResource::from_option(item_table.get("identifiedResourceName").ok().map(fix_encoding)),
                    unidentified_resource: ItemResource::from_option(item_table.get("unidentifiedResourceName").ok().map(fix_encoding)),
                    stats: ItemStats::parse(&identified_description),
                };

                result.insert(ItemId(item_id), info);
            }
        }

        Ok(result.compact())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        library.item_info_table.get(&key)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: ItemInfo = ItemInfo {
            identified_name: ItemName::not_found_value(),
            unidentified_name: ItemName::not_found_value(),
            identified_resource: ItemResource::not_found_value(),
            unidentified_resource: ItemResource::not_found_value(),
            stats: ItemStats::DEFAULT,
        };
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
