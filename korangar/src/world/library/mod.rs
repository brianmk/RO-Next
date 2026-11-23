mod accessory_name;
mod aliases;
mod card_composition;
mod item_description;
mod item_display_name;
mod item_info;
mod item_name;
mod item_resource;
mod item_stats;
mod item_stats_fallback;
pub(crate) mod job_alias;
mod job_identity;
mod map_sky_data;
mod monster_sprite;
mod skill_actions;
mod skill_delays;
mod skill_effects;
mod skill_icon;
mod skill_information;
mod skill_requirements;
mod skill_sounds;
mod skill_tree;
mod status_effects;

use std::borrow::Cow;
use std::hash::Hash;

use encoding_rs::EUC_KR;
use hashbrown::HashMap;
use korangar_loaders::FileLoader;
use mlua::Lua;
use ragnarok_packets::{ItemId, JobId, Sex, SkillId};

pub use self::accessory_name::AccessoryName;
pub use self::aliases::AliasesSummary;
pub(crate) use self::aliases::{validate as validate_alias_data, validate_compiled as validate_compiled_aliases};
use self::card_composition::{CardComposition, compose_item_name};
pub use self::item_description::NO_DESCRIPTION;
use self::item_description::{ItemDescriptions, sanitize_lines};
pub use self::item_display_name::ItemDisplayNames;
pub use self::item_info::ItemInfo;
pub use self::item_name::ItemName;
pub use self::item_resource::{ItemResource, ItemResourceKey};
pub use self::item_stats::{ItemStats, StatBonuses};
pub(crate) use self::item_stats::{
    ItemStatsSource, load as load_item_stats, resolve as resolve_item_stats, slot_count as item_stats_slot_count,
    validate as validate_item_stats_data, validate_compiled as validate_compiled_item_stats,
};
pub use self::job_identity::JobIdentity;
pub use self::map_sky_data::MapSkyData;
use self::monster_sprite::MonsterSprites;
pub(crate) use self::skill_actions::{caster_action_for_skill, default_caster_action};
#[allow(unused_imports)] // `SkillDelays` is part of the public lookup API.
pub use self::skill_delays::{SkillDelays, skill_delays};
pub(crate) use self::skill_effects::{
    LayerAnchor, LayerGrow, LayerKind, LayerPlacement, LayerPrimitive, LayerVertical, MAX_LAYERS_PER_PHASE, SkillEffectLayer, SkillPhase,
    skill_action_override, skill_cast_circle_color, skill_effect_layers, skill_effect_sound_files, skill_has_effect,
    validate as validate_skill_effect_data, validate_compiled as validate_compiled_skill_effects,
};
use self::skill_icon::SkillIcons;
pub(crate) use self::skill_sounds::{curated_skill_sound_files, generic_cast_sound_files, skill_sound_files};
pub use self::skill_tree::SkillTreeLayout;
pub(crate) use self::status_effects::{
    StatusAuraKind, StatusAuraSpec, StatusEffectCategory, StatusRefreshPolicy, refresh_policy as status_effect_refresh_policy,
    status_aura_spec, status_effect_info, validate as validate_status_effect_data, validate_compiled as validate_compiled_status_effects,
    value_lines as status_effect_value_lines,
};
use crate::loaders::GameFileLoader;
pub use crate::world::library::skill_information::SkillListInformation;
pub use crate::world::library::skill_requirements::{SkillListKey, SkillListRequirements};

pub struct Library {
    job_identity_table: <JobIdentity as Table>::Storage,
    item_info_table: <ItemInfo as Table>::Storage,
    item_display_names: ItemDisplayNames,
    map_sky_data_table: <MapSkyData as Table>::Storage,
    skill_information_table: <SkillListInformation as Table>::Storage,
    skill_requirements_table: <SkillListRequirements as Table>::Storage,
    skill_tree_table: <SkillTreeLayout as Table>::Storage,
    monster_sprites: MonsterSprites,
    skill_icons: SkillIcons,
    accessory_name_table: <AccessoryName as Table>::Storage,
    card_composition: CardComposition,
    item_descriptions: ItemDescriptions,
}

impl Library {
    pub fn new(game_file_loader: &GameFileLoader) -> mlua::Result<Self> {
        // The data-driven alias tables (job skill-tree fallbacks, monster
        // sprite aliases, skill icon aliases) are needed while the library
        // tables below are built, so they are loaded first.
        aliases::load(game_file_loader);
        let job_identity_table = JobIdentity::load(game_file_loader)?;
        // The item info table resolves the stats of every item from the item
        // database data, so the data file has to be loaded first.
        load_item_stats(game_file_loader);
        let item_info_table = ItemInfo::load(game_file_loader)?;
        let item_display_names = ItemDisplayNames::load(game_file_loader, &item_info_table);
        let map_sky_data_table = MapSkyData::load(game_file_loader)?;
        let skill_information_table = SkillListInformation::load(game_file_loader)?;
        let skill_requirements_table = SkillListRequirements::load(game_file_loader)?;
        let skill_tree_table = SkillTreeLayout::load(game_file_loader)?;
        let monster_sprites = MonsterSprites::load(game_file_loader, &job_identity_table);
        let skill_icons = SkillIcons::load(game_file_loader, &skill_information_table);
        let accessory_name_table = AccessoryName::load(game_file_loader)?;
        let card_composition = CardComposition::load(game_file_loader);
        let item_descriptions = ItemDescriptions::load(game_file_loader);
        skill_actions::load(game_file_loader);
        skill_sounds::load(game_file_loader);
        skill_effects::load(game_file_loader);
        // The data-driven weapon registry (weapon types, sprite candidates,
        // projectiles) is loaded here so every subsystem sees the same table.
        crate::world::weapon::load(game_file_loader);
        // The data-driven status-effect presentation table (names, icons,
        // categories, value labels, refresh policy and persistent auras).
        status_effects::load(game_file_loader);
        // The data-driven job/appearance registry (job sprites, palettes, hair
        // styles, job flags) is loaded here so every subsystem sees one table.
        crate::world::jobs::load(game_file_loader);
        // The data-driven skill behaviour and procedural effect data (clouds,
        // channeling, warp target, cast circle, warp portal, level aura).
        crate::world::skills::load(game_file_loader);

        Ok(Self {
            job_identity_table,
            item_info_table,
            item_display_names,
            map_sky_data_table,
            skill_information_table,
            skill_requirements_table,
            skill_tree_table,
            monster_sprites,
            skill_icons,
            accessory_name_table,
            card_composition,
            item_descriptions,
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

    /// Resolved display name of an item.
    ///
    /// This is the single source of item names for every display (ground item
    /// labels, pickup chat messages, inventory/equipment tooltips, shops,
    /// storage). It uses the English display name table from the client data,
    /// falls back to a displayable name from `iteminfo.lub` and finally to
    /// `Item #<id>`.
    #[inline(always)]
    pub fn item_name(&self, item_id: ItemId, is_identified: bool) -> Cow<'_, str> {
        self.item_display_names.name(item_id, is_identified)
    }

    /// Number of card slots of an item (`slotCount` from `iteminfo.lub`).
    #[inline(always)]
    pub fn item_slot_count(&self, item_id: ItemId) -> u8 {
        ItemInfo::try_get(self, item_id)
            .map(|item_info| item_info.slot_count())
            .unwrap_or(0)
    }

    /// Description lines of an item from the classic description tables
    /// (`idnum2itemdesctable.txt` / `num2itemdesctable.txt`).
    ///
    /// Falls back to the description from `iteminfo.lub` when the item is not
    /// covered by the classic tables. Both sources are filtered to displayable
    /// (Latin) text so untranslated Korean data can never reintroduce square
    /// placeholder glyphs. If neither source has a usable description the
    /// [`NO_DESCRIPTION`] placeholder is returned.
    pub fn item_description(&self, item_id: ItemId, is_identified: bool) -> Vec<String> {
        if let Some(lines) = self.item_descriptions.get(item_id, is_identified) {
            return lines.to_vec();
        }

        let Some(item_info) = ItemInfo::try_get(self, item_id) else {
            return vec![NO_DESCRIPTION.to_owned()];
        };

        let fallback = sanitize_lines(item_info.description());

        match fallback.is_empty() {
            true => vec![NO_DESCRIPTION.to_owned()],
            false => fallback,
        }
    }

    /// Composed display name of an item, following the official client
    /// convention: `[+N ][card prefixes ]<base name>[ card postfixes]`.
    ///
    /// Unidentified items only show their unidentified name (neither the
    /// refinement level nor the cards are revealed). Cards that are missing
    /// from the composition tables fall back to their display name without the
    /// `Card` suffix; card ids that cannot be resolved at all are appended in
    /// brackets so they neither crash the name nor disappear silently.
    pub fn composed_item_name(&self, item_id: ItemId, is_identified: bool, refinement_level: u8, card_slots: &[u32; 4]) -> String {
        let base_name = self.item_name(item_id, is_identified).into_owned();

        if !is_identified {
            return base_name;
        }

        let mut prefixes = Vec::new();
        let mut postfixes = Vec::new();
        let mut unresolved_cards = Vec::new();

        for card_slot in card_slots.iter().copied().filter(|card_slot| *card_slot != 0) {
            let card_id = ItemId(card_slot);

            if let Some(prefix) = self.card_composition.prefix(card_id) {
                let prefix = prefix.to_owned();

                match self.card_composition.is_postfix(card_id) {
                    true => postfixes.push(prefix),
                    false => prefixes.push(prefix),
                }

                continue;
            }

            // The card is missing from the composition tables. Derive a prefix
            // from its display name; unresolvable ids are shown in brackets.
            let display_name = self.item_name(card_id, true).into_owned();

            match display_name.starts_with("Item #") {
                true => unresolved_cards.push(display_name),
                false => prefixes.push(display_name.strip_suffix(" Card").unwrap_or(&display_name).to_owned()),
            }
        }

        compose_item_name(&base_name, refinement_level, &prefixes, &postfixes, &unresolved_cards)
    }

    /// Returns whether the job has a job identity (name/sprite) in this data
    /// set.
    pub(crate) fn has_job_identity(&self, job_id: JobId) -> bool {
        job_identity::JobIdentity::try_get(self, job_id).is_some()
    }

    /// Returns whether the job resolves to a non-empty skill tree (directly or
    /// via the generated fallback table).
    pub(crate) fn has_skill_tree(&self, job_id: JobId) -> bool {
        skill_tree::SkillTreeLayout::resolve(self, job_id).is_some()
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

        // The accessory path template is data (`jobs.ron` `paths.headgear`);
        // the name itself is still the data-driven `accname.lub` table.
        crate::world::jobs::headgear_part_path(accessory_name.name(), sex)
    }

    /// View ids of every headgear the client name table knows.
    ///
    /// Used by the `--check-appearance` audit to validate that every headgear
    /// sprite exists in the archives.
    pub(crate) fn accessory_view_ids(&self) -> impl Iterator<Item = u16> + '_ {
        self.accessory_name_table.keys().copied()
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
