use hashbrown::HashMap;
use mlua::Lua;
use ragnarok_packets::ItemId;

use super::{
    HashMapExt, ItemName, ItemResource, ItemStats, ItemStatsSource, Library, LuaExt, Table, fix_encoding, item_stats_slot_count,
    resolve_item_stats,
};
use crate::loaders::GameFileLoader;

#[derive(Debug, Clone)]
pub struct ItemInfo {
    pub(super) identified_name: ItemName,
    pub(super) unidentified_name: ItemName,
    pub(super) identified_resource: ItemResource,
    pub(super) unidentified_resource: ItemResource,
    /// Stats of the item from the item database data (`item_stats.ron`).
    pub(super) stats: ItemStats,
    /// The raw identified description lines from `iteminfo.lub`, used by the
    /// item info window.
    pub(super) description: Vec<String>,
    /// Number of card slots of the item (`Slots` in the item database, with
    /// `slotCount` in `iteminfo.lub` as the fallback for items that are not in
    /// the database).
    pub(super) slot_count: u8,
}

impl ItemInfo {
    /// The raw identified item description lines.
    pub fn description(&self) -> &[String] {
        &self.description
    }

    /// Number of card slots of the item.
    pub fn slot_count(&self) -> u8 {
        self.slot_count
    }
}

impl Table for ItemInfo {
    type Key<'a> = ItemId;
    type Storage = HashMap<ItemId, Self>;

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        let state = Lua::load_from_game_files(game_file_loader, &["data\\luafiles514\\lua files\\datainfo\\iteminfo.lub"])?;

        let globals = state.globals();
        let mut result = HashMap::new();
        let mut data_rows = 0;
        let mut description_fallbacks = 0;
        let mut generic_fallbacks = 0;

        if let Ok(table) = globals.get::<mlua::Table>("tbl") {
            for (item_id, item_table) in table.pairs::<u32, mlua::Table>().flatten() {
                let item_id = ItemId(item_id);
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

                let (stats, source) = resolve_item_stats(item_id, &identified_description);
                match source {
                    ItemStatsSource::Data => data_rows += 1,
                    ItemStatsSource::DescriptionFallback => description_fallbacks += 1,
                    ItemStatsSource::GenericFallback => generic_fallbacks += 1,
                }

                // The server item database is authoritative for the card slot
                // count; `iteminfo.lub` may be stale (or missing the item).
                let slot_count = item_stats_slot_count(item_id)
                    .unwrap_or_else(|| item_table.get::<u32>("slotCount").ok().unwrap_or(0).clamp(0, u8::MAX as u32) as u8);

                let info = ItemInfo {
                    identified_name: ItemName::from_option(item_table.get("identifiedDisplayName").ok().map(fix_encoding)),
                    unidentified_name: ItemName::from_option(item_table.get("unidentifiedDisplayName").ok().map(fix_encoding)),
                    identified_resource: ItemResource::from_option(item_table.get("identifiedResourceName").ok().map(fix_encoding)),
                    unidentified_resource: ItemResource::from_option(item_table.get("unidentifiedResourceName").ok().map(fix_encoding)),
                    stats,
                    description: identified_description,
                    slot_count,
                };

                result.insert(item_id, info);
            }
        }

        crate::logging::log_debug!(
            "item stats: {data_rows} items from the item database data, {description_fallbacks} description fallbacks, \
             {generic_fallbacks} generic fallbacks"
        );

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
            description: Vec::new(),
            slot_count: 0,
        };
        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}
