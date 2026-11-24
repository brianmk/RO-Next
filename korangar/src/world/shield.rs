//! Data-driven shield appearance layer.
//!
//! The shield sprite of a player is selected from the shield item view id and
//! the shield name, using the job sprite stem of the body
//! (`rodecomp/Source/Contents/Actors/Resources/RobeResources.cpp`,
//! `EquipmentResourceMgr::GetShieldResourceName`):
//!
//! * `data\sprite\방패\<job>\<job>_<sex>_<view>_방패` for view ids that ship
//!   their own numeric sprite (custom/newer shields), and
//! * `data\sprite\방패\<job>\<job>_<sex>_<name>` for the classic shield names
//!   (`가드`, `버클러`, `쉴드`, `미러쉴드`).
//!
//! The entity-appear packets carry the shield *item id* (`sd->status.shield`),
//! while the sprite is chosen by the item view id. `archive/data/client/
//! shields.ron` (embedded fallback, archive override) carries the item id ->
//! view id mapping generated from the committed rAthena pre-renewal item facts.
//! Missing candidates are skipped cleanly by the animation loader, so the
//! entity simply has no shield layer.
//!
//! The shield is drawn as its own animation layer with the body animation
//! state/action/frame, in front of the body and behind the weapon.
//!
//! Per-motion layer direction: unlike the robe, the official client has **no**
//! `DrawOnTop`/`2DLayerDir` table for shields. `SpriteLayerRenderer` draws the
//! shield as the fixed layer 7 (`DrawLayer`/`ShouldDrawLayer`,
//! `rodecomp/Source/Contents/Sprites/SpriteLayerDrawing.cpp`) and the
//! per-motion placement comes from the cells/anchors of the shield `.act`
//! frame, which the animation loader already applies. The archives were checked
//! for a `ShieldLayerDir`/shield entry in the `2DLayerDir` Lua tables and none
//! exists, so the fixed layer remains the correct fallback.

use std::collections::HashSet;
use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::{JobId, Sex};
use serde::Deserialize;

use super::jobs;
use crate::loaders::{EntityPartFile, GameFileLoader};

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\shields.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/shields.ron");

/// Sprite path template (`방패\<stem>\<stem>_<sex>_<variant>`).
const PATH_TEMPLATE: &str = "방패\\{stem}\\{stem}_{sex}_{variant}";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ItemViewRow {
    item: u32,
    view: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ViewNameRow {
    view: u32,
    names: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShieldFile {
    #[allow(dead_code)]
    provenance: String,
    #[allow(dead_code)]
    version: u32,
    item_views: Vec<ItemViewRow>,
    view_names: Vec<ViewNameRow>,
}

struct ShieldRegistry {
    /// Sorted by item id.
    item_views: Vec<(u32, u32)>,
    /// Sorted by view id.
    view_names: Vec<(u32, Vec<String>)>,
}

static COMPILED: OnceLock<ShieldRegistry> = OnceLock::new();
static ARCHIVE: OnceLock<ShieldRegistry> = OnceLock::new();

fn compiled() -> &'static ShieldRegistry {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(registry) => registry,
        Err(error) => {
            crate::logging::log_warn!("shields: compiled fallback is invalid ({error})");
            ShieldRegistry {
                item_views: Vec::new(),
                view_names: Vec::new(),
            }
        }
    })
}

fn registry() -> &'static ShieldRegistry {
    ARCHIVE.get().unwrap_or_else(compiled)
}

fn parse(content: &str) -> Result<ShieldRegistry, String> {
    let file: ShieldFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;

    let mut item_views = file.item_views.into_iter().map(|row| (row.item, row.view)).collect::<Vec<_>>();
    item_views.sort_by_key(|(item, _)| *item);
    for pair in item_views.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!("duplicate shield item id {}", pair[0].0));
        }
    }

    let mut view_names = file.view_names.into_iter().map(|row| (row.view, row.names)).collect::<Vec<_>>();
    view_names.sort_by_key(|(view, _)| *view);
    for pair in view_names.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!("duplicate shield view id {}", pair[0].0));
        }
    }
    for (view, names) in &view_names {
        if names.is_empty() {
            return Err(format!("shield view {view} has no sprite name"));
        }
    }

    Ok(ShieldRegistry { item_views, view_names })
}

/// Loads the shield table from the client archive (or keeps the compiled
/// fallback).
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        let count = compiled().item_views.len();
        crate::logging::log_debug!("shields: loaded {count} item views from the compiled fallback");
        return count;
    };

    match parse(content) {
        Ok(registry) => {
            let count = registry.item_views.len();
            let _ = ARCHIVE.set(registry);
            crate::logging::log_debug!("shields: loaded {count} item views from {DATA_PATH}");
            count
        }
        Err(error) => {
            crate::logging::log_warn!("shields: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().item_views.len()
        }
    }
}

/// Shield view id of a shield item id, `None` when the item is not a shield of
/// the committed item table.
pub fn view_of(shield: u32) -> Option<u32> {
    let registry = registry();
    registry
        .item_views
        .binary_search_by_key(&shield, |(item, _)| *item)
        .ok()
        .map(|index| registry.item_views[index].1)
}

/// Sprite name suffixes of a shield view id.
pub fn names_of(view: u32) -> &'static [String] {
    let registry = registry();
    match registry.view_names.binary_search_by_key(&view, |(view, _)| *view) {
        Ok(index) => &registry.view_names[index].1,
        Err(_) => &[],
    }
}

/// Builds the ordered shield sprite candidates of a player entity.
///
/// The candidates are the raw packet value first (custom servers often send the
/// view id directly), then the resolved item view id, each as the numeric
/// `_<view>_방패` sprite and the numeric sprite without the suffix, followed by
/// the classic shield names of the view. Missing candidates are skipped by the
/// animation loader.
pub(crate) fn part_candidates(job_id: JobId, sex: Sex, shield: u32) -> Vec<EntityPartFile> {
    if shield == 0 {
        return Vec::new();
    }

    let job_stem = jobs::player_sprite_stem(job_id);
    let sex_suffix = jobs::sex_suffix(sex).unwrap_or("남");
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |variant: &str| {
        let path = PATH_TEMPLATE
            .replace("{stem}", job_stem)
            .replace("{sex}", sex_suffix)
            .replace("{variant}", variant);
        if seen.insert(path.clone()) {
            candidates.push(EntityPartFile::new(path));
        }
    };

    let mut numeric = |value: u32| {
        push(&format!("{value}_방패"));
        push(&value.to_string());
    };

    numeric(shield);

    let view = view_of(shield);
    if let Some(view) = view.filter(|view| *view != shield) {
        numeric(view);
    }

    for value in std::iter::once(shield).chain(view) {
        for name in names_of(value) {
            push(name);
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use korangar_loaders::FileLoader;
    use ragnarok_packets::{JobId, Sex};

    use super::{load, names_of, parse, part_candidates, view_of};

    #[test]
    fn compiled_data_is_valid() {
        let registry = parse(super::FALLBACK).expect("the committed shield data has to be valid");
        assert_eq!(registry.item_views.len(), 40);
        assert_eq!(view_of(2101), Some(1));
        assert_eq!(view_of(2107), Some(4));
        assert_eq!(view_of(9999), None);
        assert_eq!(names_of(1), &["가드".to_owned()]);
        assert_eq!(names_of(2), &["버클러".to_owned()]);
        assert_eq!(names_of(3), &["쉴드".to_owned()]);
        assert_eq!(names_of(4), &["미러쉴드".to_owned()]);
        assert!(names_of(5).is_empty());
    }

    #[test]
    fn item_ids_resolve_to_the_classic_sprite_names() {
        let paths: Vec<String> = part_candidates(JobId(1), Sex::Male, 2101)
            .into_iter()
            .map(|part| part.path)
            .collect();

        assert_eq!(paths, vec![
            "방패\\검사\\검사_남_2101_방패".to_owned(),
            "방패\\검사\\검사_남_2101".to_owned(),
            "방패\\검사\\검사_남_1_방패".to_owned(),
            "방패\\검사\\검사_남_1".to_owned(),
            "방패\\검사\\검사_남_가드".to_owned(),
        ]);
    }

    #[test]
    fn numeric_view_ids_are_tried_with_and_without_the_suffix() {
        let paths: Vec<String> = part_candidates(JobId(1), Sex::Female, 2189)
            .into_iter()
            .map(|part| part.path)
            .collect();
        assert!(paths.contains(&"방패\\검사\\검사_여_2189_방패".to_owned()));
        assert!(paths.contains(&"방패\\검사\\검사_여_2189".to_owned()));
        assert_eq!(paths.len(), 2);
    }

    #[test]
    fn no_shield_has_no_candidates() {
        assert!(part_candidates(JobId(1), Sex::Male, 0).is_empty());
    }

    #[test]
    fn malformed_data_is_rejected() {
        let data = super::FALLBACK;
        assert!(parse("(item_views: [)").is_err());
        assert!(parse(&data.replace("(item: 2101, view: 1)", "(item: 2101, view: 1, typo: 1)")).is_err());
        assert!(parse(&data.replace("(item: 2101, view: 1),", "(item: 2101, view: 1), (item: 2101, view: 2),")).is_err());
    }

    /// The generated candidates have to hit real archive files. Ignored in the
    /// normal test run because the CondaRO archives are not in the repository.
    #[test]
    #[ignore = "requires the CondaRO game archives in the client directory"]
    fn shield_candidates_exist_in_the_archives() {
        let loader = crate::loaders::GameFileLoader::default();
        loader.load_archives_from_settings();

        let parts = part_candidates(JobId(1), Sex::Male, 2101);
        let found = parts.iter().any(|part| {
            loader.file_exists(&format!("data\\sprite\\{}.spr", part.path))
                && loader.file_exists(&format!("data\\sprite\\{}.act", part.path))
        });
        assert!(found, "no shield sprite/action pair of Guard exists: {parts:?}");
    }

    #[test]
    fn load_falls_back_without_archives() {
        let loader = crate::loaders::GameFileLoader::default();
        assert_eq!(load(&loader), 40);
    }
}
