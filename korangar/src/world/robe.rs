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
//! `CCharacterRender::RenderRobe`. By default it is a top layer
//! (`RobeTopLayer.cpp` returns `true` when no per-motion layer direction entry
//! exists, which is the common case for this data set); the per-motion
//! `DrawOnTop` tables are not part of the appearance data this client loads.

use std::collections::HashSet;
use std::sync::OnceLock;

use korangar_loaders::FileLoader;
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

    /// Whether the robe is drawn as a top layer.
    ///
    /// The classic `IsTopLayerRobe`/`IsRobeDrawOnTop` tables default to
    /// `true` (draw on top) when no entry exists for the robe/job/action; this
    /// data set ships no entry for the supported robes, so the top layer is
    /// the correct default.
    pub fn is_top_layer(&self) -> bool {
        true
    }
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
    use korangar_loaders::FileLoader;
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
}
