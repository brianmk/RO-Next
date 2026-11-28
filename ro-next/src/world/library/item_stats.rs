//! Data-driven stats of an item (`data\client\item_stats.ron`).
//!
//! The client does not receive structured item stats from the server. The
//! official client shows them in the item tooltip because they are plain text
//! in the localized `iteminfo.lub` description. Parsing that text is not
//! viable: the stat fields are grammar of one specific translation (the Korean
//! stat field labels), so with an English or otherwise translated
//! `iteminfo.lub` every stat silently became zero.
//!
//! The authoritative values are the **item database rows the server uses**
//! (rAthena `db/pre-re/item_db*.yml`). They are extracted by
//! `tools/extract_item_facts.py`, written into this client's data file by
//! `tools/gen_item_stats.py` and loaded here:
//!
//! * `archive/data/client/item_stats.ron` — the data file. It is loaded at
//!   runtime through [`GameFileLoader`] (`data\client\item_stats.ron`), so a
//!   server can override it without rebuilding the client.
//! * the same file is embedded with `include_str!` as the compiled fallback, so
//!   the archive override and the compiled table can never drift apart.
//!
//! The file is a strictly ascending list of `(item id, stats)`; the loader
//! rejects duplicate/out-of-order ids and unknown fields (a malformed override
//! falls back to the compiled table with a warning instead of leaving the
//! client without stats). `ro-next --check-items [PATH]` validates the schema
//! without starting a window, network connection or game archives.
//!
//! The client description text remains display-only
//! ([`crate::world::ItemDescriptions`]). Only for item ids that are **not** in
//! the facts (custom items of a server that were never added to its item
//! database, or client-only leftover ids) a clearly logged last-resort
//! description parser is used ([`super::item_stats_fallback`]); items without
//! any data or description get [`ItemStats::DEFAULT`], which is documented
//! here instead of being a silent zero.

use std::sync::OnceLock;

use ragnarok_packets::ItemId;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use super::{Library, Table, item_stats_fallback};
use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\item_stats.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/item_stats.ron");
/// Schema version the loader understands.
const SUPPORTED_VERSION: u32 = 1;

/// Stats of a single item, from the server item database.
///
/// `Default` (all zero) is the documented generic fallback for items that are
/// neither in the data file nor described by the client data; it is never
/// silently substituted for an item that is covered by the data.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ItemStats {
    /// Weapon attack (`Attack` in the item database).
    pub attack: i32,
    /// Defense (`Defense`).
    pub defense: i32,
    /// Weight in the client stat panel unit (the database weight divided by
    /// 10, like the server displays and calculates it).
    pub weight: i32,
    /// Weapon level (`WeaponLevel`) for weapons, `0` otherwise.
    pub weapon_level: u8,
    /// Armor level (`ArmorLevel`) for armor, `0` otherwise.
    pub armor_level: u8,
    /// Minimum required level (`EquipLevelMin`).
    pub required_level: u16,
    /// Card slots (`Slots`).
    pub slots: u8,
    /// Whether the item can be refined (`Refineable`).
    pub refineable: bool,
    /// Unconditional flat stat bonuses of the item script (e.g. `STR + 2`).
    pub bonuses: StatBonuses,
}

impl ItemStats {
    /// All zero stats: the generic fallback for items without any data or
    /// description. Not used for items that are covered by the data file.
    pub const DEFAULT: Self = Self {
        attack: 0,
        defense: 0,
        weight: 0,
        weapon_level: 0,
        armor_level: 0,
        required_level: 0,
        slots: 0,
        refineable: false,
        bonuses: StatBonuses::DEFAULT,
    };
}

/// Where the stats of an item came from. Used for the startup log and the
/// tests; the display code only consumes [`ItemStats`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemStatsSource {
    /// A row of the item facts data (the normal path).
    Data,
    /// Last-resort description parser (only for item ids missing from the
    /// data).
    DescriptionFallback,
    /// Neither data nor a parseable description: [`ItemStats::DEFAULT`].
    GenericFallback,
}

/// The parsed data file: strictly ascending `(item id, stats)` rows.
struct ItemStatsTable(Vec<(u32, ItemStats)>);

/// The compiled fallback table, parsed on first use.
static COMPILED: OnceLock<ItemStatsTable> = OnceLock::new();
/// Table loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<ItemStatsTable> = OnceLock::new();

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemStatsFile {
    #[allow(dead_code)]
    provenance: String,
    version: u32,
    items: Vec<(u32, ItemStats)>,
}

fn parse(content: &str) -> Result<ItemStatsTable, String> {
    let file: ItemStatsFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;

    if file.version != SUPPORTED_VERSION {
        return Err(format!(
            "unsupported schema version {} (this client understands {SUPPORTED_VERSION})",
            file.version
        ));
    }

    if file.items.is_empty() {
        return Err("the data file contains no item rows".to_owned());
    }

    let mut previous = 0;
    for (item_id, _) in &file.items {
        if *item_id == 0 {
            return Err("the data file contains item id 0".to_owned());
        }
        if *item_id <= previous {
            return Err(format!(
                "item ids must be strictly ascending and unique; {item_id} follows {previous}"
            ));
        }
        previous = *item_id;
    }

    Ok(ItemStatsTable(file.items))
}

fn compiled() -> &'static ItemStatsTable {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(table) => table,
        Err(error) => {
            crate::logging::log_warn!("item stats: compiled fallback is invalid ({error})");
            ItemStatsTable(Vec::new())
        }
    })
}

fn table() -> &'static ItemStatsTable {
    ARCHIVE.get().unwrap_or_else(compiled)
}

/// Loads the item stats data from the client archive (or keeps the compiled
/// fallback). Returns the number of loaded item rows.
///
/// A malformed data file is reported and the compiled fallback is used instead,
/// so a bad override can never leave the client without stats. The data file is
/// embedded in the binary, so the compiled table is always available.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let table = match from_archive.as_deref() {
        Some(content) => match parse(content) {
            Ok(table) => table,
            Err(error) => {
                crate::logging::log_warn!("item stats: invalid {DATA_PATH} ({error}), using the compiled fallback");
                ItemStatsTable(compiled().0.clone())
            }
        },
        None => ItemStatsTable(compiled().0.clone()),
    };

    let count = table.0.len();
    let _ = ARCHIVE.set(table);
    crate::logging::log_debug!("item stats: loaded {count} item rows from the item database data");
    count
}

/// Stats row of an item from the data file, or `None` when the item is not
/// covered by the facts (custom/client-only item).
pub(crate) fn lookup(item_id: ItemId) -> Option<&'static ItemStats> {
    let table = table();
    table
        .0
        .binary_search_by_key(&item_id.0, |(id, _)| *id)
        .ok()
        .map(|index| &table.0[index].1)
}

/// Number of card slots from the item database (`Slots`), when available.
///
/// The server item database is authoritative for custom item changes, while
/// `iteminfo.lub`'s `slotCount` is client data that may be stale.
pub(crate) fn slot_count(item_id: ItemId) -> Option<u8> {
    lookup(item_id).map(|stats| stats.slots)
}

/// Resolves the stats of an item and records where they came from.
///
/// The data file is the normal path. Only an item id that is missing there
/// falls back to the description parser (logged once per item id at debug
/// level, because client-only leftover ids are expected) and finally to
/// [`ItemStats::DEFAULT`] (logged once per item id at warn level; the generic
/// fallback is documented on [`ItemStats::DEFAULT`], it is not a silent zero).
pub(crate) fn resolve(item_id: ItemId, description: &[String]) -> (ItemStats, ItemStatsSource) {
    if let Some(stats) = lookup(item_id) {
        return (*stats, ItemStatsSource::Data);
    }

    if !description.is_empty() {
        let stats = item_stats_fallback::parse(description);
        // Client-only leftover ids are expected (the client `iteminfo.lub` is
        // larger than the server item database), so this is a debug message;
        // the per-startup summary in `ItemInfo::load` counts them.
        crate::logging::log_debug!(
            "item stats: item {} is not in the item database data; using the last-resort description parser",
            item_id.0
        );
        return (stats, ItemStatsSource::DescriptionFallback);
    }

    crate::logging::warn_once(
        module_path!(),
        &format!("item-stats-generic-fallback:{}", item_id.0),
        format_args!(
            "item stats: item {} has no item database row and no description; showing zero stats (documented generic fallback)",
            item_id.0
        ),
    );
    (ItemStats::DEFAULT, ItemStatsSource::GenericFallback)
}

impl Table for ItemStats {
    type Key<'a> = ItemId;
    type Storage = ();

    fn load(game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        // The stats are stored per item in `ItemInfo`; the table is a
        // process-wide static so the data file is loaded exactly once.
        load(game_file_loader);
        Ok(())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        super::ItemInfo::try_get(library, key).map(|item_info| &item_info.stats)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: ItemStats = ItemStats::DEFAULT;

        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

/// Flat stat bonuses of an item (e.g. `STR + 2`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct StatBonuses {
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub intelligence: i32,
    pub dexterity: i32,
    pub luck: i32,
    pub attack: i32,
    pub magic_attack: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub hit: i32,
    pub flee: i32,
    pub critical: i32,
    pub attack_speed: i32,
}

impl StatBonuses {
    /// All zero bonuses.
    pub const DEFAULT: Self = Self {
        strength: 0,
        agility: 0,
        vitality: 0,
        intelligence: 0,
        dexterity: 0,
        luck: 0,
        attack: 0,
        magic_attack: 0,
        defense: 0,
        magic_defense: 0,
        hit: 0,
        flee: 0,
        critical: 0,
        attack_speed: 0,
    };

    /// Add the bonuses of another item to these bonuses.
    pub fn merge(&mut self, other: &Self) {
        self.strength += other.strength;
        self.agility += other.agility;
        self.vitality += other.vitality;
        self.intelligence += other.intelligence;
        self.dexterity += other.dexterity;
        self.luck += other.luck;
        self.attack += other.attack;
        self.magic_attack += other.magic_attack;
        self.defense += other.defense;
        self.magic_defense += other.magic_defense;
        self.hit += other.hit;
        self.flee += other.flee;
        self.critical += other.critical;
        self.attack_speed += other.attack_speed;
    }

    /// Returns `true` if no bonus was evaluated.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Validates a data file against the schema and returns the number of item
/// rows it contains.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `ro-next --check-items [PATH]`, so CI can validate the committed data file
/// (or an archive override) without starting a window or a network connection.
pub(crate) fn validate(content: &str) -> Result<usize, String> {
    parse(content).map(|table| table.0.len())
}

/// Validates the compiled fallback (used by `--check-items` without a path and
/// by the tests).
pub(crate) fn validate_compiled() -> Result<usize, String> {
    validate(FALLBACK)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_data_file_is_valid_and_sorted() {
        let count = validate_compiled().expect("the compiled item stats data must be valid");
        assert_eq!(count, 28532, "one row per item of the CondaRO pre-re + renewal item databases");

        let table = compiled();
        assert!(
            table.0.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "rows must be sorted and unique"
        );
    }

    #[test]
    fn data_rows_contain_the_authoritative_values() {
        // Falchion (1104): one-handed sword, attack 39, weight 600/10 = 60,
        // weapon level 1, required level 2, three slots, refineable.
        let falchion = lookup(ItemId(1104)).expect("Falchion must be covered by the data");
        assert_eq!(falchion.attack, 39);
        assert_eq!(falchion.defense, 0);
        assert_eq!(falchion.weight, 60);
        assert_eq!(falchion.weapon_level, 1);
        assert_eq!(falchion.required_level, 2);
        assert_eq!(falchion.slots, 3);
        assert!(falchion.refineable);

        // Haedonggum (1123) has the unconditional `bonus bInt,3;` script.
        let haedonggum = lookup(ItemId(1123)).expect("Haedonggum must be covered by the data");
        assert_eq!(haedonggum.bonuses.intelligence, 3);
        assert_eq!(haedonggum.bonuses.strength, 0);

        // Red Potion (501): a consumable without stats but with a weight.
        let potion = lookup(ItemId(501)).expect("Red Potion must be covered by the data");
        assert_eq!(potion.weight, 7);
        assert_eq!(potion.attack, 0);
        assert!(potion.bonuses.is_empty());
    }

    #[test]
    fn data_path_does_not_depend_on_the_description_language() {
        // An English description without any of the old Korean/English stat
        // labels used to produce an all-zero stat panel. The data file must
        // win before the description is ever consulted.
        let english = vec!["A basic one-handed sword.".to_owned()];

        let (stats, source) = resolve(ItemId(1104), &english);

        assert_eq!(source, ItemStatsSource::Data);
        assert_eq!(stats.attack, 39);
        assert_eq!(stats.weight, 60);
        assert_eq!(stats.bonuses, StatBonuses::DEFAULT);
    }

    #[test]
    fn unknown_item_without_description_uses_the_documented_generic_fallback() {
        let (stats, source) = resolve(ItemId(999_999), &[]);

        assert_eq!(source, ItemStatsSource::GenericFallback);
        assert_eq!(stats, ItemStats::DEFAULT);
    }

    #[test]
    fn unknown_item_with_description_uses_the_logged_description_fallback() {
        // The last-resort parser still understands the English labels of the
        // classic description tables, but it is only reached for unknown ids.
        let description = vec!["Attack:^CC0000 42^000000".to_owned(), "Weight:^009900 15^000000".to_owned()];

        let (stats, source) = resolve(ItemId(999_998), &description);

        assert_eq!(source, ItemStatsSource::DescriptionFallback);
        assert_eq!(stats.attack, 42);
        assert_eq!(stats.weight, 15);
    }

    #[test]
    fn slot_count_comes_from_the_item_database() {
        assert_eq!(slot_count(ItemId(1104)), Some(3));
        assert_eq!(slot_count(ItemId(501)), Some(0));
        assert_eq!(slot_count(ItemId(999_999)), None);
    }

    #[test]
    #[ignore = "requires the game archives (run from the ro-next crate root, client/game_archives.ron)"]
    fn condaro_archives_are_covered_by_the_item_database_data() {
        // End-to-end check on the real CondaRO client data: the iteminfo table
        // that the client loads must resolve its stats from the item database
        // data, not from the localized description text.
        let game_file_loader = GameFileLoader::default();
        game_file_loader.load_archives_from_settings();

        let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
        game_file_loader.load_patched_lua_files();

        if !lua_archive_existed {
            game_file_loader.remove_patched_lua_files();
        }

        assert_eq!(
            load(&game_file_loader),
            6169,
            "the archive data must contain the committed item rows"
        );

        let item_info = crate::world::ItemInfo::load(&game_file_loader).expect("iteminfo.lub must load from the archives");
        let mut from_data = 0;
        let mut compared = 0;
        let mut description_differences = 0;
        let mut field_differences = [0usize; 6];
        let mut samples: Vec<(ItemId, ItemStats, ItemStats, Vec<String>)> = Vec::new();

        for (item_id, info) in item_info.iter() {
            if lookup(*item_id).is_none() {
                continue;
            }

            from_data += 1;

            // Diagnostic only: how often the last-resort description parser
            // would have produced different values. The data file is the
            // authoritative source either way.
            let parsed = item_stats_fallback::parse(&info.description);

            // Compare only the fields the old description parser produced
            // (the data file additionally carries armor level, slots and
            // refineability, which the description text never had).
            let differs = parsed.attack != info.stats.attack
                || parsed.defense != info.stats.defense
                || parsed.weight != info.stats.weight
                || parsed.weapon_level != info.stats.weapon_level
                || parsed.required_level != info.stats.required_level
                || parsed.bonuses != info.stats.bonuses;

            if differs {
                description_differences += 1;
                field_differences[0] += usize::from(parsed.attack != info.stats.attack);
                field_differences[1] += usize::from(parsed.defense != info.stats.defense);
                field_differences[2] += usize::from(parsed.weight != info.stats.weight);
                field_differences[3] += usize::from(parsed.weapon_level != info.stats.weapon_level);
                field_differences[4] += usize::from(parsed.required_level != info.stats.required_level);
                field_differences[5] += usize::from(parsed.bonuses != info.stats.bonuses);

                if samples.len() < 8 {
                    samples.push((*item_id, info.stats, parsed, info.description.iter().take(6).cloned().collect()));
                }
            }

            compared += 1;
        }

        eprintln!(
            "item stats archive check: {from_data}/{} iteminfo items covered by the item database data; the description parser would \
             disagree on {description_differences}/{compared} (attack {}, defense {}, weight {}, weapon level {}, required level {}, \
             bonuses {})",
            item_info.len(),
            field_differences[0],
            field_differences[1],
            field_differences[2],
            field_differences[3],
            field_differences[4],
            field_differences[5],
        );

        for (item_id, data, parsed, description) in &samples {
            eprintln!(
                "  disagreement sample {}: data attack={} defense={} weight={} weapon_level={} required_level={} bonuses={:?} | \
                 description-parser attack={} defense={} weight={} weapon_level={} required_level={} bonuses={:?} | {description:?}",
                item_id.0,
                data.attack,
                data.defense,
                data.weight,
                data.weapon_level,
                data.required_level,
                data.bonuses,
                parsed.attack,
                parsed.defense,
                parsed.weight,
                parsed.weapon_level,
                parsed.required_level,
                parsed.bonuses,
            );
        }

        assert!(
            from_data >= 5_000,
            "only {from_data} of {} iteminfo items are covered",
            item_info.len()
        );

        // Well-known items have the authoritative values.
        assert_eq!(lookup(ItemId(1104)).map(|stats| stats.attack), Some(39));
        assert_eq!(lookup(ItemId(1123)).map(|stats| stats.bonuses.intelligence), Some(3));
        assert_eq!(lookup(ItemId(501)).map(|stats| stats.weight), Some(7));
    }

    #[test]
    fn load_without_archives_keeps_the_compiled_fallback() {
        // The compiled table is embedded in the binary, so a missing/malformed
        // archive override can never leave the client without item stats.
        let loader = GameFileLoader::default();
        assert_eq!(load(&loader), 28532);
        assert_eq!(lookup(ItemId(1104)).map(|stats| stats.attack), Some(39));
    }

    #[test]
    fn malformed_data_is_rejected() {
        // Missing version field.
        assert!(validate("(provenance: \"x\", items: [(1, (weight: 1))])").is_err());
        // Unknown field (deny_unknown_fields).
        assert!(validate("(provenance: \"x\", version: 1, items: [(1, (attck: 1))])").is_err());
        // Unsupported schema version.
        assert!(validate("(provenance: \"x\", version: 2, items: [(1, (weight: 1))])").is_err());
        // Empty table.
        assert!(validate("(provenance: \"x\", version: 1, items: [])").is_err());
        // Duplicate item ids.
        assert!(validate("(provenance: \"x\", version: 1, items: [(2, (weight: 1)), (2, (weight: 2))])").is_err());
        // Out-of-order item ids.
        assert!(validate("(provenance: \"x\", version: 1, items: [(2, (weight: 1)), (1, (weight: 2))])").is_err());
        // Item id 0.
        assert!(validate("(provenance: \"x\", version: 1, items: [(0, (weight: 1))])").is_err());
        // Syntactically invalid RON.
        assert!(validate("not ron at all").is_err());
    }
}
