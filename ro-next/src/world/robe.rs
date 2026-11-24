//! Data-driven robe (garment) appearance layer.
//!
//! The robe is the garment item that is drawn behind/over the player body
//! (`data\sprite\로브\...`). The classic client resolves it with
//! `EquipmentResourceMgr::GetRobeResourceName`
//! (`rodecomp/Source/Contents/Actors/Resources/RobeResources.cpp`):
//!
//! 1. the sprite is shared by every job/sex: `로브\<robe>\<robe>.spr`,
//! 2. the action is per job and sex: `로브\<robe>\<sex>\<job>_<sex>.act` (with
//!    a per-job `.spr` fallback in complete data sets).
//!
//! The robe id -> sprite folder name mapping, and the animated opacity window
//! of transparent robes (`RobeOpacity.cpp` / `querytransparentiteminfo`), live
//! in `archive/data/client/robes.ron` (embedded fallback, archive override).
//! The CondaRO archives only ship some robe variants, so every candidate is an
//! attempt: the animation loader skips missing sprite/action pairs cleanly and
//! the entity simply has no robe layer.
//!
//! The robe is drawn as its own animation layer with the body animation
//! state/action/frame and the body palette, exactly like the classic client's
//! `CCharacterRender::RenderRobe`. Whether it is the top layer or the
//! behind-the-body pass comes from the official `DrawOnTop`/`_New_DrawOnTop`
//! Lua tables (`Lua Files\SprEditInfo\2DLayerDir_F` /
//! `_New_2DLayerDir_F` and the `BigLayerDir_*`/`SmallLayerDir_*` tables); the
//! tables are loaded here and fall back to the previous "always on top"
//! behavior when the archives do not ship them.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use ro_next_loaders::FileLoader;
use mlua::{Lua, Table};
use ragnarok_packets::{JobId, Sex};
use serde::Deserialize;

use super::jobs;
use crate::loaders::{EntityPartFile, GameFileLoader};

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\robes.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/robes.ron");

/// Sprite path templates (`{name}`, `{sex}`, `{job_stem}` placeholders).
const SPRITE_PATH: &str = "로브\\{name}\\{name}";
const ACTION_PATH: &str = "로브\\{name}\\{sex}\\{job_stem}_{sex}";

/// Lua files that implement the official per-motion robe draw order.
///
/// `2DLayerDir_F.lub` defines `DrawOnTop` and the `GetLayerSizeType`/
/// `GetSpriteInheriteJob` helpers; the `BigLayerDir_*`/`SmallLayerDir_*` files
/// hold the actual per-job/action/motion lists. The client loads the newer
/// `_New_2DLayerDir_F.lub` set when it exists and falls back to the classic
/// one otherwise, exactly like `CSession::LoadLuaFiles`
/// (`rodecomp/Source/Client/Session/Session.cpp`).
const SPRITE_ROBE_ID_PATH: &str = "data\\luafiles514\\lua files\\datainfo\\spriterobeid.lub";
const JOB_INHERIT_PATH: &str = "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub";
const OLD_LAYER_DIR_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\2dlayerdir_f.lub";
const OLD_BIG_MALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\biglayerdir_male.lub";
const OLD_BIG_FEMALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\biglayerdir_female.lub";
const OLD_SMALL_MALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\smalllayerdir_male.lub";
const OLD_SMALL_FEMALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\smalllayerdir_female.lub";
const NEW_LAYER_DIR_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\_new_2dlayerdir_f.lub";
const NEW_BIG_MALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\_new_biglayerdir_male.lub";
const NEW_BIG_FEMALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\_new_biglayerdir_female.lub";
const NEW_SMALL_MALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\_new_smalllayerdir_male.lub";
const NEW_SMALL_FEMALE_PATH: &str = "data\\luafiles514\\lua files\\spreditinfo\\_new_smalllayerdir_female.lub";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RobeAlphaRow {
    from: u8,
    to: u8,
    period: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RobeRow {
    id: u16,
    names: Vec<String>,
    alpha: RobeAlphaRow,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RobeFile {
    #[allow(dead_code)]
    provenance: String,
    #[allow(dead_code)]
    version: u32,
    robes: Vec<RobeRow>,
}

/// Runtime profile of one robe.
#[derive(Debug, Clone)]
pub struct RobeProfile {
    id: u16,
    names: Vec<String>,
    from: u8,
    to: u8,
    period: u32,
}

impl RobeProfile {
    /// Robe view id of the profile.
    #[allow(dead_code)]
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Sprite folder names of the robe, most preferred first.
    pub fn names(&self) -> &[String] {
        &self.names
    }

    /// Whether the robe is fully opaque at every point in time.
    #[allow(dead_code)]
    pub fn is_opaque(&self) -> bool {
        self.from == self.to && self.from == 255
    }

    /// Opacity multiplier (`0.0..=1.0`) at a client tick in milliseconds.
    ///
    /// Implements `CSession::GetRobeAlpha` (`RobeOpacity.cpp`): the alpha
    /// linearly moves from `from` to `to` and back over `period`; a zero range
    /// or period is the constant `to` value.
    pub fn opacity_at(&self, time_ms: u32) -> f32 {
        if self.period == 0 || self.from == self.to {
            return self.to as f32 / 255.0;
        }

        let half = self.period / 2;
        if half == 0 {
            return self.to as f32 / 255.0;
        }

        let elapsed = time_ms % self.period;
        let phase = if elapsed > half { self.period - elapsed } else { elapsed };
        let range = i32::from(self.to) - i32::from(self.from);
        let base = i32::from(self.to.min(self.from));
        let value = (phase as f32 / half as f32 * range as f32) as i32 + base;

        (value.clamp(0, 255) as f32) / 255.0
    }

    /// Fallback draw direction of the robe when the per-motion
    /// `DrawOnTop`/`2DLayerDir` data is not available.
    ///
    /// The official tables default to `true` (draw on top) when no entry
    /// exists for the robe/job/action, so "on top" is the correct fallback.
    /// The actual per-motion decision is [`draw_on_top`].
    pub fn is_top_layer(&self) -> bool {
        true
    }
}

/// Which of the two official layer-direction tables a robe uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerSize {
    Big,
    Small,
}

type JobLayerMap = HashMap<u32, HashMap<u32, Vec<u32>>>;

/// The four `BigLayerDir_*`/`SmallLayerDir_*` tables, indexed by
/// `[female = 0, male = 1]`.
#[derive(Debug, Default)]
struct LayerTables {
    big: [JobLayerMap; 2],
    small: [JobLayerMap; 2],
}

impl LayerTables {
    fn get(&self, size: LayerSize, sex_index: usize, job: u32) -> Option<&HashMap<u32, Vec<u32>>> {
        match size {
            LayerSize::Big => self.big[sex_index].get(&job),
            LayerSize::Small => self.small[sex_index].get(&job),
        }
    }
}

/// One generation (`DrawOnTop` or `_New_DrawOnTop`) of the official tables.
#[derive(Debug)]
struct DrawOnTopGeneration {
    layers: LayerTables,
    /// Job id -> sprite-inherit job. Applied once for the classic tables
    /// (`RIDING`/`JOB_INHERIT_LIST2`/`SPRITE`), and once for the new table.
    inherit: Vec<HashMap<u32, u32>>,
    /// `EXCEPTION_SPRITE_INHERIT_LIST`, only part of the classic generation.
    exception: HashMap<u32, u32>,
}

impl DrawOnTopGeneration {
    fn inherited_job(&self, job: u32, size: LayerSize) -> u32 {
        let mut job = job;
        for inherit in &self.inherit {
            if let Some(&mapped) = inherit.get(&job) {
                job = mapped;
            }
        }
        if size == LayerSize::Big
            && let Some(&mapped) = self.exception.get(&job)
        {
            job = mapped;
        }
        job
    }

    fn listed(&self, size: LayerSize, sex_index: usize, job: u32, action: u32, motion: u32) -> bool {
        self.layers
            .get(size, sex_index, job)
            .and_then(|actions| actions.get(&action))
            .is_some_and(|motions| motions.contains(&motion))
    }
}

/// Per-motion robe draw order loaded from the official Lua tables.
#[derive(Debug)]
struct DrawOnTopTables {
    size_by_robe: HashMap<u16, LayerSize>,
    classic: Option<DrawOnTopGeneration>,
    new: Option<DrawOnTopGeneration>,
}

static DRAW_ON_TOP: OnceLock<DrawOnTopTables> = OnceLock::new();

impl DrawOnTopTables {
    /// `true` when the robe is drawn over the body for this action/motion.
    ///
    /// This is `DrawOnTop`/`_New_DrawOnTop` from the official Lua tables: a
    /// motion listed in the layer table is the "behind" pass
    /// (`DrawOnTop == false`); everything else is the top layer, including
    /// missing robes/jobs/actions.
    fn draw_on_top(&self, robe: u16, sex: Sex, job: u32, action: u32, motion: u32) -> bool {
        let size = self.size_by_robe.get(&robe).copied().unwrap_or(LayerSize::Big);
        let sex_index = match sex {
            Sex::Female => 0,
            _ => 1,
        };

        // The official client prefers the new tables whenever
        // `_New_2DLayerDir_F.lub` loaded successfully.
        let generation = self.new.as_ref().or(self.classic.as_ref());
        let Some(generation) = generation else {
            return true;
        };

        let job = generation.inherited_job(job, size);
        !generation.listed(size, sex_index, job, action, motion)
    }
}

/// Loads the official per-motion robe tables.
///
/// The classic `2DLayerDir_F` path needs `JOBID` (from `jobinheritlist.lub`)
/// and `SPRITE_ROBE_IDs` (from `spriterobeid.lub`) because those files index
/// their tables with those globals. The newer `_New_2DLayerDir_F` path reuses
/// `GetLayerSizeType` from the classic file. Missing files are not an error:
/// the caller keeps the previous "always on top" behavior.
fn load_draw_on_top_tables(game_file_loader: &GameFileLoader) -> Option<DrawOnTopTables> {
    let lua = Lua::new();

    for path in [JOB_INHERIT_PATH, SPRITE_ROBE_ID_PATH, OLD_LAYER_DIR_PATH] {
        if let Err(error) = exec_lua_file(&lua, game_file_loader, path) {
            crate::logging::log_debug!("robes: no per-motion draw table ({error}); keeping the top-layer default");
            return None;
        }
    }

    let size_by_robe = read_layer_size_table(&lua).unwrap_or_default();

    // Both generations are optional; the official client prefers the new one
    // when `_New_2DLayerDir_F.lub` loaded and falls back to the classic one.
    let classic = if try_exec_lua_files(&lua, game_file_loader, &[
        OLD_BIG_MALE_PATH,
        OLD_BIG_FEMALE_PATH,
        OLD_SMALL_MALE_PATH,
        OLD_SMALL_FEMALE_PATH,
    ]) {
        read_classic_generation(&lua)
    } else {
        None
    };

    let new = if try_exec_lua_files(&lua, game_file_loader, &[
        NEW_LAYER_DIR_PATH,
        NEW_BIG_MALE_PATH,
        NEW_BIG_FEMALE_PATH,
        NEW_SMALL_MALE_PATH,
        NEW_SMALL_FEMALE_PATH,
    ]) {
        read_new_generation(&lua)
    } else {
        None
    };

    if classic.is_none() && new.is_none() {
        crate::logging::log_debug!("robes: layer-direction tables are empty; keeping the top-layer default");
        return None;
    }

    crate::logging::log_debug!(
        "robes: loaded per-motion draw tables (classic: {}, new: {})",
        classic.is_some(),
        new.is_some()
    );

    Some(DrawOnTopTables {
        size_by_robe,
        classic,
        new,
    })
}

fn exec_lua_file(lua: &Lua, game_file_loader: &GameFileLoader, path: &str) -> Result<(), String> {
    let bytes = game_file_loader.get(path).map_err(|error| format!("{path}: {error:?}"))?;
    lua.load(&bytes).exec().map_err(|error| format!("{path}: {error}"))
}

fn try_exec_lua_files(lua: &Lua, game_file_loader: &GameFileLoader, paths: &[&str]) -> bool {
    paths.iter().all(|path| exec_lua_file(lua, game_file_loader, path).is_ok())
}

fn read_layer_size_table(lua: &Lua) -> Option<HashMap<u16, LayerSize>> {
    let table: Table = lua.globals().get("LayerSizeTypeList").ok()?;
    let mut sizes = HashMap::new();

    for pair in table.pairs::<u16, u32>() {
        let Ok((robe, size)) = pair else {
            continue;
        };

        sizes.insert(robe, match size {
            1 => LayerSize::Small,
            _ => LayerSize::Big,
        });
    }

    Some(sizes)
}

fn read_layer_table(lua: &Lua, name: &str) -> Option<JobLayerMap> {
    let table: Table = lua.globals().get(name).ok()?;
    let mut jobs = JobLayerMap::new();

    for pair in table.pairs::<u32, Table>() {
        let Ok((job, actions)) = pair else {
            continue;
        };

        let mut actions_by_id = HashMap::new();
        for pair in actions.pairs::<u32, Vec<u32>>() {
            let Ok((action, motions)) = pair else {
                continue;
            };
            actions_by_id.insert(action, motions);
        }
        jobs.insert(job, actions_by_id);
    }

    Some(jobs)
}

fn read_inherit_table(lua: &Lua, name: &str) -> HashMap<u32, u32> {
    let Ok(table) = lua.globals().get::<Table>(name) else {
        return HashMap::new();
    };

    let mut inherit = HashMap::new();
    for pair in table.pairs::<u32, u32>() {
        let Ok((job, mapped)) = pair else {
            continue;
        };
        inherit.insert(job, mapped);
    }
    inherit
}

fn read_layer_tables(lua: &Lua, big_male: &str, big_female: &str, small_male: &str, small_female: &str) -> Option<LayerTables> {
    Some(LayerTables {
        big: [read_layer_table(lua, big_female)?, read_layer_table(lua, big_male)?],
        small: [read_layer_table(lua, small_female)?, read_layer_table(lua, small_male)?],
    })
}

fn read_classic_generation(lua: &Lua) -> Option<DrawOnTopGeneration> {
    let layers = read_layer_tables(lua, "BigLayerDir_M", "BigLayerDir_F", "SmallLayerDir_M", "SmallLayerDir_F")?;

    Some(DrawOnTopGeneration {
        layers,
        inherit: vec![
            read_inherit_table(lua, "RIDING_SPRITE_INHERIT_LIST"),
            read_inherit_table(lua, "JOB_INHERIT_LIST2"),
            read_inherit_table(lua, "SPRITE_INHERIT_LIST"),
        ],
        exception: read_inherit_table(lua, "EXCEPTION_SPRITE_INHERIT_LIST"),
    })
}

fn read_new_generation(lua: &Lua) -> Option<DrawOnTopGeneration> {
    let layers = read_layer_tables(
        lua,
        "_New_BigLayerDir_M",
        "_New_BigLayerDir_F",
        "_New_SmallLayerDir_M",
        "_New_SmallLayerDir_F",
    )?;

    Some(DrawOnTopGeneration {
        layers,
        inherit: vec![read_inherit_table(lua, "LAYER_ALL_TABLE_LIST")],
        exception: HashMap::new(),
    })
}

/// Per-motion robe draw order.
///
/// Returns `None` when the client archives do not provide the official Lua
/// tables; callers then keep the previous behavior (the robe is drawn on top).
pub fn draw_on_top(robe: u16, sex: Sex, job: JobId, action: usize, motion: usize) -> Option<bool> {
    let tables = DRAW_ON_TOP.get()?;
    Some(tables.draw_on_top(
        robe,
        sex,
        u32::from(job.0),
        u32::try_from(action).unwrap_or(u32::MAX),
        u32::try_from(motion).unwrap_or(u32::MAX),
    ))
}

struct RobeRegistry {
    /// Sorted by `id`.
    robes: Vec<RobeProfile>,
}

static COMPILED: OnceLock<RobeRegistry> = OnceLock::new();
static ARCHIVE: OnceLock<RobeRegistry> = OnceLock::new();

fn compiled() -> &'static RobeRegistry {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(registry) => registry,
        Err(error) => {
            crate::logging::log_warn!("robes: compiled fallback is invalid ({error})");
            RobeRegistry { robes: Vec::new() }
        }
    })
}

fn registry() -> &'static RobeRegistry {
    ARCHIVE.get().unwrap_or_else(compiled)
}

/// Parse and validate the robe data file.
fn parse(content: &str) -> Result<RobeRegistry, String> {
    let file: RobeFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;

    if file.robes.is_empty() {
        return Err("the data file contains no robes".to_owned());
    }

    let mut robes = Vec::with_capacity(file.robes.len());
    for row in file.robes {
        if row.names.is_empty() {
            return Err(format!("robe {} has no sprite name", row.id));
        }
        if row.alpha.period == 0 && row.alpha.from != row.alpha.to {
            return Err(format!("robe {} has an animated alpha with a zero period", row.id));
        }
        if robes.iter().any(|robe: &RobeProfile| robe.id == row.id) {
            return Err(format!("duplicate robe id {}", row.id));
        }
        robes.push(RobeProfile {
            id: row.id,
            names: row.names,
            from: row.alpha.from,
            to: row.alpha.to,
            period: row.alpha.period,
        });
    }
    robes.sort_by_key(|robe| robe.id);

    Ok(RobeRegistry { robes })
}

/// Returns the profile of a robe id, or `None` when the id is not in the table.
pub fn profile(robe: u16) -> Option<&'static RobeProfile> {
    registry()
        .robes
        .binary_search_by_key(&robe, |robe| robe.id)
        .ok()
        .map(|index| &registry().robes[index])
}

/// Loads the robe table from the client archive (or keeps the compiled
/// fallback).
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    // The per-motion layer direction is a separate data set (the official
    // client's `2DLayerDir`/`_New_2DLayerDir` Lua tables) and is loaded even
    // when the robe sprite table itself comes from the compiled fallback.
    if let Some(tables) = load_draw_on_top_tables(game_file_loader) {
        let _ = DRAW_ON_TOP.set(tables);
    }

    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        let count = compiled().robes.len();
        crate::logging::log_debug!("robes: loaded {count} robes from the compiled fallback");
        return count;
    };

    match parse(content) {
        Ok(registry) => {
            let count = registry.robes.len();
            let _ = ARCHIVE.set(registry);
            crate::logging::log_debug!("robes: loaded {count} robes from {DATA_PATH}");
            count
        }
        Err(error) => {
            crate::logging::log_warn!("robes: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().robes.len()
        }
    }
}

/// Builds the ordered sprite/action candidate pairs of a player entity's robe.
///
/// The candidates are the shared robe sprite with the per-job action first,
/// then the per-job sprite/action pair (complete data sets ship it), then the
/// shared sprite/action pair. `palette_path` is the optional external palette
/// that replaces the palette embedded in the robe sprite; the CondaRO archives
/// ship no robe palette files, so the entity layer passes `None`. Missing pairs
/// are skipped by the animation loader, so the list is an attempt order.
pub(crate) fn part_candidates(job_id: JobId, sex: Sex, robe: u16, palette_path: Option<String>) -> Vec<EntityPartFile> {
    if robe == 0 {
        return Vec::new();
    }

    let Some(profile) = profile(robe) else {
        // A custom server can ship a robe sprite named after the view id.
        return vec![EntityPartFile::new(format!("로브\\{robe}\\{robe}"))];
    };

    let job_stem = jobs::player_sprite_stem(job_id);
    let sex_suffix = jobs::sex_suffix(sex).unwrap_or("남");
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |path: String, action: Option<String>| {
        let key = format!("{path}\u{1}{action:?}");
        if seen.insert(key) {
            candidates.push(EntityPartFile::with_palette_and_action(path, palette_path.clone(), action));
        }
    };

    for name in profile.names() {
        let sprite = SPRITE_PATH.replace("{name}", name);
        let action = ACTION_PATH
            .replace("{name}", name)
            .replace("{sex}", sex_suffix)
            .replace("{job_stem}", job_stem);

        // Shared sprite + per-job action (the layout of this data set).
        push(sprite.clone(), Some(action.clone()));
        // Per-job sprite + action (complete/classic data sets).
        push(action, None);
        // Shared sprite + shared action (some custom data sets).
        push(sprite, None);
    }

    candidates
}

#[cfg(test)]
mod tests {
    use ro_next_loaders::FileLoader;
    use ragnarok_packets::{JobId, Sex};

    use super::{load, parse, part_candidates, profile};

    fn paths(parts: &[crate::loaders::EntityPartFile]) -> Vec<(String, Option<String>)> {
        parts.iter().map(|part| (part.path.clone(), part.action_path.clone())).collect()
    }

    #[test]
    fn compiled_data_is_valid() {
        let registry = parse(super::FALLBACK).expect("the committed robe data has to be valid");
        assert_eq!(registry.robes.len(), 50, "all robe sprite ids should be modelled");
        assert_eq!(profile(1).unwrap().names(), &["천사날개".to_owned(), "ANGEL_WINGS".to_owned()]);
        assert!(profile(0).is_none());
        assert!(profile(51).is_none());
    }

    #[test]
    fn animated_opacity_matches_the_official_curve() {
        // Robe 36 fades between 100 and 200 over 25500 ms.
        let robe = profile(36).expect("robe 36 is in the data file");
        assert!((robe.opacity_at(0) - 100.0 / 255.0).abs() < 0.001);
        assert!((robe.opacity_at(12750) - 200.0 / 255.0).abs() < 0.001);
        assert!((robe.opacity_at(25500) - 100.0 / 255.0).abs() < 0.001);
        assert!((robe.opacity_at(6375) - 150.0 / 255.0).abs() < 0.01);

        // Opaque robes stay opaque.
        let opaque = profile(1).expect("robe 1 is in the data file");
        assert!(opaque.is_opaque());
        assert!((opaque.opacity_at(1234) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn candidates_use_the_shared_sprite_and_per_job_action() {
        let swordman = JobId(1);
        let parts = part_candidates(swordman, Sex::Male, 1, Some("data\\palette\\몸\\남\\검사_남_0.pal".to_owned()));
        assert_eq!(paths(&parts), vec![
            (
                "로브\\천사날개\\천사날개".to_owned(),
                Some("로브\\천사날개\\남\\검사_남".to_owned())
            ),
            ("로브\\천사날개\\남\\검사_남".to_owned(), None),
            ("로브\\천사날개\\천사날개".to_owned(), None),
            (
                "로브\\ANGEL_WINGS\\ANGEL_WINGS".to_owned(),
                Some("로브\\ANGEL_WINGS\\남\\검사_남".to_owned())
            ),
            ("로브\\ANGEL_WINGS\\남\\검사_남".to_owned(), None),
            ("로브\\ANGEL_WINGS\\ANGEL_WINGS".to_owned(), None),
        ]);
        assert!(parts.iter().all(|part| part.palette_path.is_some()));
    }

    #[test]
    fn unrobed_entities_have_no_candidates() {
        assert!(part_candidates(JobId(1), Sex::Male, 0, None).is_empty());
    }

    #[test]
    fn unknown_robe_ids_try_the_numeric_folder() {
        let parts = part_candidates(JobId(1), Sex::Female, 500, None);
        assert_eq!(paths(&parts), vec![("로브\\500\\500".to_owned(), None)]);
    }

    #[test]
    fn malformed_data_is_rejected() {
        let data = super::FALLBACK;
        assert!(parse("(robes: [)").is_err());
        assert!(parse(&data.replace("(id: 1, names: [", "(id: 1, typo: 1, names: [")).is_err());
        assert!(parse(&data.replace("names: [\"천사날개\", \"ANGEL_WINGS\"]", "names: []")).is_err());
    }

    /// The generated candidates have to hit real archive files. Ignored in the
    /// normal test run because the CondaRO archives are not in the repository.
    #[test]
    #[ignore = "requires the CondaRO game archives in the client directory"]
    fn robe_candidates_exist_in_the_archives() {
        let loader = crate::loaders::GameFileLoader::default();
        loader.load_archives_from_settings();

        let parts = part_candidates(JobId(1), Sex::Male, 1, None);
        let found = parts.iter().any(|part| {
            let action = part.action_path.as_deref().unwrap_or(&part.path);
            loader.file_exists(&format!("data\\sprite\\{}.spr", part.path)) && loader.file_exists(&format!("data\\sprite\\{}.act", action))
        });
        assert!(found, "no robe sprite/action pair of robe 1 exists: {parts:?}");
    }

    /// The archive override path goes through the normal file loader; without
    /// archives the compiled fallback is used.
    #[test]
    fn load_falls_back_without_archives() {
        let loader = crate::loaders::GameFileLoader::default();
        assert_eq!(load(&loader), 50);
        let _ = loader.file_exists("data\\client\\robes.ron");
    }

    #[test]
    fn reads_the_official_layer_direction_lua_schema() {
        use std::collections::HashMap;

        use mlua::Lua;

        use super::{LayerSize, read_inherit_table, read_layer_size_table, read_layer_table};

        let lua = Lua::new();
        lua.load(
            r#"
            JOBID = { JT_NOVICE = 0, JT_SWORDMAN = 1, JT_KNIGHT = 7, JT_KNIGHT_H = 4008 }
            SPRITE_ROBE_IDs = { ROBE_KIRIN_WING = 6 }
            LayerSizeTypeList = { [SPRITE_ROBE_IDs.ROBE_KIRIN_WING] = 1 }
            BigLayerDir_M = {
                [JOBID.JT_SWORDMAN] = {
                    [0] = { 0, 1, 2 },
                    [5] = { 0 },
                },
            }
            LAYER_ALL_TABLE_LIST = { [JOBID.JT_KNIGHT_H] = JOBID.JT_KNIGHT }
            "#,
        )
        .exec()
        .unwrap();

        assert_eq!(read_layer_size_table(&lua).unwrap().get(&6), Some(&LayerSize::Small));

        let jobs = read_layer_table(&lua, "BigLayerDir_M").unwrap();
        assert_eq!(jobs.get(&1).unwrap().get(&0), Some(&vec![0, 1, 2]));
        assert_eq!(jobs.get(&1).unwrap().get(&5), Some(&vec![0]));

        assert_eq!(read_inherit_table(&lua, "LAYER_ALL_TABLE_LIST").get(&4008), Some(&7));
        assert!(read_inherit_table(&lua, "JOB_INHERIT_LIST2").is_empty());
    }

    #[test]
    fn per_motion_draw_order_follows_the_official_lists() {
        use std::collections::HashMap;

        use super::{DrawOnTopGeneration, DrawOnTopTables, LayerSize, LayerTables};

        let mut layers = LayerTables::default();
        let mut actions = HashMap::new();
        actions.insert(0, vec![0, 1, 2]);
        actions.insert(5, vec![0]);
        layers.big[1].insert(1, actions);
        let classic = DrawOnTopGeneration {
            layers,
            inherit: vec![HashMap::from([(13, 1)])],
            exception: HashMap::new(),
        };
        let tables = DrawOnTopTables {
            size_by_robe: HashMap::from([(1, LayerSize::Big), (6, LayerSize::Small)]),
            classic: Some(classic),
            new: None,
        };

        // Swordman idle: frames 0..=2 are the behind pass.
        assert!(!tables.draw_on_top(1, Sex::Male, 1, 0, 0));
        assert!(!tables.draw_on_top(1, Sex::Male, 1, 0, 2));
        assert!(tables.draw_on_top(1, Sex::Male, 1, 0, 3));
        // Attack action 5 frame 0 is behind.
        assert!(!tables.draw_on_top(1, Sex::Male, 1, 5, 0));
        assert!(tables.draw_on_top(1, Sex::Male, 1, 5, 1));
        // Inherited riding job (13 -> 1).
        assert!(!tables.draw_on_top(1, Sex::Male, 13, 0, 0));
        // Unknown job/action falls back to the top layer.
        assert!(tables.draw_on_top(1, Sex::Male, 999, 0, 0));
        assert!(tables.draw_on_top(1, Sex::Male, 1, 99, 0));
        // Robe 6 uses the (empty) small-robe table, so it stays on top.
        assert!(tables.draw_on_top(6, Sex::Male, 1, 0, 0));
        // An unknown robe id defaults to the big table, like
        // `GetLayerSizeType`.
        assert!(!tables.draw_on_top(99, Sex::Male, 1, 0, 0));
        // Female table is separate (empty here) and defaults to the top layer.
        assert!(tables.draw_on_top(1, Sex::Female, 1, 0, 0));
    }

    #[test]
    fn new_generation_wins_and_uses_its_own_inherit_list() {
        use std::collections::HashMap;

        use super::{DrawOnTopGeneration, DrawOnTopTables, LayerSize, LayerTables};

        let mut classic_layers = LayerTables::default();
        let mut classic_actions = HashMap::new();
        classic_actions.insert(0, vec![0]);
        classic_layers.big[1].insert(1, classic_actions);
        let classic = DrawOnTopGeneration {
            layers: classic_layers,
            inherit: vec![],
            exception: HashMap::new(),
        };

        let mut new_layers = LayerTables::default();
        let mut new_actions = HashMap::new();
        new_actions.insert(0, vec![0]);
        new_layers.big[1].insert(7, new_actions);
        let new = DrawOnTopGeneration {
            layers: new_layers,
            inherit: vec![HashMap::from([(4008, 7)])],
            exception: HashMap::new(),
        };

        let tables = DrawOnTopTables {
            size_by_robe: HashMap::from([(1, LayerSize::Big)]),
            classic: Some(classic),
            new: Some(new),
        };

        // The new generation takes precedence and inherits 4008 -> 7.
        assert!(!tables.draw_on_top(1, Sex::Male, 4008, 0, 0));
        // Job 1 is not in the new table, so it is drawn on top even though the
        // classic table lists it.
        assert!(tables.draw_on_top(1, Sex::Male, 1, 0, 0));
    }

    /// Loads the official Lua tables from the CondaRO archives. Ignored in the
    /// normal test run because the game archives are not part of the
    /// repository.
    #[test]
    #[ignore = "requires the CondaRO game archives in the client directory"]
    fn draw_tables_are_loaded_from_the_archives() {
        let loader = crate::loaders::GameFileLoader::default();
        loader.load_archives_from_settings();
        loader.load_patched_lua_files();

        let tables = super::load_draw_on_top_tables(&loader).expect("the archives ship the official DrawOnTop tables");

        // Robe 1 is a big robe; Swordman idle frames 0..=2 are behind the
        // body in both generations (verified against the Lua files).
        assert!(!tables.draw_on_top(1, Sex::Male, 1, 0, 0));
        assert!(tables.draw_on_top(1, Sex::Male, 1, 0, 3));
    }
}
