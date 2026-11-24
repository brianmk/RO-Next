//! Data-driven skill behaviour and procedural effect data.
//!
//! `archive/data/client/skills.ron` (shipped inside the client archive, loaded
//! through [`GameFileLoader`] so a server can override it without rebuilding
//! the client; embedded with `include_str!` as the compiled fallback) owns the
//! skill behaviour flags and procedural effect layers that used to be
//! hard-coded skill-id branches/constants:
//!
//! * **channeling** — skills cast through the channeling protocol path (Rolling
//!   Cutter 2036) instead of an ordinary cast,
//! * **warp_target** — skills that show a provisional warp portal at the target
//!   cell before the server creates the portal skill unit (Warp Portal 27),
//! * **move_while_casting** — a learned skill that grants movement while a cast
//!   is running (Free Cast 278). The job also has to carry the
//!   `move_while_casting` capability in [`crate::world::jobs`] (`jobs.ron`);
//!   both flags have to hold, exactly like the previous hard-coded
//!   `FREE_CAST_SKILL_ID` + `FREE_CAST_JOB_IDS` pair,
//! * **clouds** — procedural cloud layers the official effect tables cannot
//!   express (Storm Gust's `cloud1..7.tga` cloud),
//! * **cast_circle**, **warp_portal**, **level_aura** — the effect texture
//!   names and geometry of the procedural client effects that used to be
//!   constants in `world/effect/{cast_circle,warp,aura}.rs`.
//!
//! Skill *visuals* stay in the unified effect pipeline
//! ([`crate::world::library::skill_effects`]); this module only answers the
//! behavioural questions and supplies the procedural layer parameters.
//!
//! The table is validated on load (`deny_unknown_fields`): a malformed archive
//! override is rejected with a warning and the compiled fallback is used, so a
//! bad override can never leave the client without the behaviour data.
//! `ro-next --check-skills [PATH]` is the matching CLI check and
//! `tools/gen_skills.py --check` is the generator drift guard.

use std::sync::OnceLock;

use ragnarok_packets::SkillId;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use crate::graphics::Color;
use crate::loaders::GameFileLoader;
use crate::world::SkillPhase;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skills.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/skills.ron");

/// De-duplication phase of the procedural cloud layers.
///
/// [`SkillPhase`] indices are `0..=5`; the cloud spawn uses a value outside of
/// them so the per-cast completion tracker can claim it independently of the
/// effect phases (the previously hard-coded `STORM_CLOUD_PHASE`).
pub(crate) const CLOUD_DEDUP_PHASE: u8 = u8::MAX - 1;

// ---------------------------------------------------------------------------
// Data file schema.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillsFile {
    #[allow(dead_code)]
    provenance: String,
    #[allow(dead_code)]
    version: u32,
    behaviours: Vec<BehaviourFile>,
    clouds: Vec<CloudFile>,
    cast_circle: CastCircleFile,
    warp_portal: WarpPortalFile,
    level_aura: LevelAuraFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BehaviourFile {
    skill: u16,
    #[serde(default)]
    channeling: bool,
    #[serde(default)]
    warp_target: bool,
    #[serde(default)]
    move_while_casting: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloudFile {
    /// Skill ids that spawn this cloud (the same skill under several packet
    /// versions / client ids).
    skills: Vec<u16>,
    phase: SkillPhase,
    textures: Vec<String>,
    count: u16,
    radius: f32,
    base_height: f32,
    height_variation: f32,
    size_min: f32,
    size_variation: f32,
    spin_min: f32,
    spin_variation: f32,
    alpha: f32,
    color: (f32, f32, f32),
    duration_ms: u32,
    fade_in_ms: u32,
    fade_out_ms: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CastCircleFile {
    ring_texture: String,
    glow_texture: String,
    radius: f32,
    height: f32,
    ring_rotation_speed: f32,
    inner_ring_rotation_speed: f32,
    inner_ring_scale: f32,
    glow_scale: f32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WarpPortalFile {
    streak_texture: String,
    gradient_texture: String,
    particle_sprite: String,
    skill_radius: f32,
    skill_height: f32,
    skill_ring_count: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LevelAuraFile {
    ring_liner_texture: String,
    ring_blue_texture: String,
    ring_red_texture: String,
    bubble_texture: String,
    glow_textures: Vec<String>,
    sparkle_texture: String,
}

// ---------------------------------------------------------------------------
// Loaded representation (leaked, process-lifetime data).
// ---------------------------------------------------------------------------

/// The behavioural flags of one skill.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SkillBehaviour {
    /// Cast through the channeling protocol path and stop early on request.
    pub channeling: bool,
    /// Show a provisional warp portal at the target cell before the server
    /// creates the real skill unit.
    pub warp_target: bool,
    /// Learning this skill grants movement while a cast is running (on a job
    /// that carries the matching capability).
    pub move_while_casting: bool,
}

/// A procedural cloud layer above the target cell (Storm Gust).
#[derive(Debug, Clone, Copy)]
pub(crate) struct CloudSpec {
    /// Phase of the skill that spawns the cloud.
    pub phase: SkillPhase,
    /// Textures of the official cloud look (cycled over the puffs).
    pub textures: &'static [&'static str],
    /// Number of cloud billboards.
    pub count: usize,
    /// Radius of the cloud disc in world units.
    pub radius: f32,
    /// Base height of the cloud layer above the target cell.
    pub base_height: f32,
    /// Height spread of the puffs above `base_height`.
    pub height_variation: f32,
    /// Minimum billboard size in world units.
    pub size_min: f32,
    /// Size spread of the puffs above `size_min`.
    pub size_variation: f32,
    /// Minimum rotation speed in radians per second.
    pub spin_min: f32,
    /// Rotation speed spread above `spin_min`.
    pub spin_variation: f32,
    /// Alpha of a single cloud billboard.
    pub alpha: f32,
    /// Tint of the cloud billboards.
    pub color: Color,
    /// Lifetime of the cloud in milliseconds.
    pub duration_ms: u32,
    /// Fade-in time in milliseconds.
    pub fade_in_ms: u32,
    /// Fade-out time in milliseconds.
    pub fade_out_ms: u32,
}

/// Textures and geometry of the ground cast circle.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CastCircleSpec {
    pub ring_texture: &'static str,
    pub glow_texture: &'static str,
    pub radius: f32,
    pub height: f32,
    pub ring_rotation_speed: f32,
    pub inner_ring_rotation_speed: f32,
    pub inner_ring_scale: f32,
    pub glow_scale: f32,
}

/// Textures and geometry of the map connection warp / skill-unit warp portal.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WarpPortalSpec {
    pub streak_texture: &'static str,
    pub gradient_texture: &'static str,
    pub particle_sprite: &'static str,
    /// Official skill-unit portal geometry from `skills.ron`: parsed and
    /// validated, but the renderer currently uses the procedural warp shape.
    #[allow(dead_code)]
    pub skill_radius: f32,
    #[allow(dead_code)]
    pub skill_height: f32,
    #[allow(dead_code)]
    pub skill_ring_count: u8,
}

/// Textures of the level-99 / level-up aura.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LevelAuraSpec {
    pub ring_liner_texture: &'static str,
    pub ring_blue_texture: &'static str,
    pub ring_red_texture: &'static str,
    pub bubble_texture: &'static str,
    pub glow_textures: &'static [&'static str],
    pub sparkle_texture: &'static str,
}

/// Number of behaviour / cloud rows of a validated data file
/// (`--check-skills`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkillsSummary {
    /// Skill behaviour rows (`behaviours`).
    pub behaviours: usize,
    /// Cloud layer rows (`clouds`; one row may cover several skill ids).
    pub clouds: usize,
}

struct SkillRegistry {
    /// Sorted by skill id.
    behaviours: Vec<(u16, SkillBehaviour)>,
    /// Sorted by skill id (a cloud row may cover several skills).
    clouds: Vec<(u16, CloudSpec)>,
    cast_circle: CastCircleSpec,
    warp_portal: WarpPortalSpec,
    level_aura: LevelAuraSpec,
}

static COMPILED: OnceLock<SkillRegistry> = OnceLock::new();
static ARCHIVE: OnceLock<SkillRegistry> = OnceLock::new();

fn compiled() -> &'static SkillRegistry {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(registry) => registry,
        Err(error) => {
            crate::logging::log_warn!("skills: compiled fallback is invalid ({error})");
            SkillRegistry::empty()
        }
    })
}

fn registry() -> &'static SkillRegistry {
    ARCHIVE.get().unwrap_or_else(compiled)
}

impl SkillRegistry {
    /// Minimal registry used when even the compiled data file is unreadable.
    ///
    /// It knows no behaviour and no cloud (every lookup falls back to its
    /// documented default) and its procedural specs are empty, so the
    /// procedural effects simply do not render. It never panics.
    fn empty() -> Self {
        Self {
            behaviours: Vec::new(),
            clouds: Vec::new(),
            cast_circle: CastCircleSpec {
                ring_texture: "",
                glow_texture: "",
                radius: 0.0,
                height: 0.0,
                ring_rotation_speed: 0.0,
                inner_ring_rotation_speed: 0.0,
                inner_ring_scale: 0.0,
                glow_scale: 0.0,
            },
            warp_portal: WarpPortalSpec {
                streak_texture: "",
                gradient_texture: "",
                particle_sprite: "",
                skill_radius: 0.0,
                skill_height: 0.0,
                skill_ring_count: 0,
            },
            level_aura: LevelAuraSpec {
                ring_liner_texture: "",
                ring_blue_texture: "",
                ring_red_texture: "",
                bubble_texture: "",
                glow_textures: &[],
                sparkle_texture: "",
            },
        }
    }
}

/// Loads the skill behaviour data from the client archive (or the compiled
/// fallback).
///
/// Returns the number of skill behaviour rows. A malformed data file is
/// reported and the compiled fallback is used instead.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_debug!(
            "skills: loaded {} behaviour rows from the compiled fallback",
            compiled().behaviours.len()
        );
        return compiled().behaviours.len();
    };

    let registry = match parse(content) {
        Ok(registry) => {
            crate::logging::log_debug!(
                "skills: loaded {} behaviour rows and {} cloud rows from the archive",
                registry.behaviours.len(),
                registry.clouds.len()
            );
            registry
        }
        Err(error) => {
            crate::logging::log_warn!("skills: invalid {DATA_PATH} ({error}), using the compiled fallback");
            parse(FALLBACK).unwrap_or_else(|fallback_error| {
                crate::logging::log_warn!("skills: compiled fallback is invalid too ({fallback_error})");
                SkillRegistry::empty()
            })
        }
    };

    let count = registry.behaviours.len();
    let _ = ARCHIVE.set(registry);
    count
}

/// The behaviour flags of `skill_id`, if the data file has a row for it.
pub(crate) fn behaviour(skill_id: SkillId) -> Option<&'static SkillBehaviour> {
    let behaviours = &registry().behaviours;
    behaviours
        .binary_search_by_key(&skill_id.0, |(skill, _)| *skill)
        .ok()
        .map(|index| &behaviours[index].1)
}

/// Whether `skill_id` is a channeling skill (Rolling Cutter 2036).
pub(crate) fn channels(skill_id: SkillId) -> bool {
    behaviour(skill_id).is_some_and(|behaviour| behaviour.channeling)
}

/// Whether `skill_id` shows a provisional warp portal at its target cell
/// (Warp Portal 27).
pub(crate) fn has_warp_target(skill_id: SkillId) -> bool {
    behaviour(skill_id).is_some_and(|behaviour| behaviour.warp_target)
}

/// Whether learning `skill_id` grants movement while casting (the job also has
/// to carry the `move_while_casting` capability in `jobs.ron`).
pub(crate) fn grants_move_while_casting(skill_id: SkillId) -> bool {
    behaviour(skill_id).is_some_and(|behaviour| behaviour.move_while_casting)
}

/// The procedural cloud layer of `skill_id`, if the data file has one.
pub(crate) fn cloud(skill_id: SkillId) -> Option<&'static CloudSpec> {
    let clouds = &registry().clouds;
    clouds
        .binary_search_by_key(&skill_id.0, |(skill, _)| *skill)
        .ok()
        .map(|index| &clouds[index].1)
}

/// Textures and geometry of the ground cast circle.
pub(crate) fn cast_circle() -> &'static CastCircleSpec {
    &registry().cast_circle
}

/// Textures and geometry of the warp portal effects.
pub(crate) fn warp_portal() -> &'static WarpPortalSpec {
    &registry().warp_portal
}

/// Textures of the level-99 / level-up aura.
pub(crate) fn level_aura() -> &'static LevelAuraSpec {
    &registry().level_aura
}

/// Validates a document against the schema and returns its row counts.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `ro-next --check-skills [PATH]` so CI can validate the committed data file
/// (or an archive override) without starting a window or a network connection.
pub(crate) fn validate(content: &str) -> Result<SkillsSummary, String> {
    parse(content).map(|registry| SkillsSummary {
        behaviours: registry.behaviours.len(),
        clouds: registry.clouds.len(),
    })
}

/// Validates the compiled fallback (used by `--check-skills` without a path and
/// by the tests).
pub(crate) fn validate_compiled() -> Result<SkillsSummary, String> {
    validate(FALLBACK)
}

// ---------------------------------------------------------------------------
// Parsing and validation.
// ---------------------------------------------------------------------------

fn parse(content: &str) -> Result<SkillRegistry, String> {
    let file: SkillsFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    build(file)
}

fn validate_texture(value: &str, name: &str, suffixes: &[&str]) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{name} is empty"));
    }
    let lower = value.to_ascii_lowercase();
    if !suffixes.iter().any(|suffix| lower.ends_with(suffix)) {
        return Err(format!("{name} '{value}' does not end in {}", suffixes.join("/")));
    }
    Ok(())
}

fn build(file: SkillsFile) -> Result<SkillRegistry, String> {
    if file.behaviours.is_empty() {
        return Err("the data file contains no skill behaviours".to_owned());
    }
    if file.clouds.is_empty() {
        return Err("the data file contains no cloud layers".to_owned());
    }

    let mut behaviours: Vec<(u16, SkillBehaviour)> = Vec::with_capacity(file.behaviours.len());
    for row in file.behaviours {
        if row.skill == 0 {
            return Err("a behaviour row uses skill id 0".to_owned());
        }
        if !row.channeling && !row.warp_target && !row.move_while_casting {
            return Err(format!("skill {} behaviour row has no enabled flag", row.skill));
        }
        behaviours.push((row.skill, SkillBehaviour {
            channeling: row.channeling,
            warp_target: row.warp_target,
            move_while_casting: row.move_while_casting,
        }));
    }
    behaviours.sort_by_key(|(skill, _)| *skill);
    for pair in behaviours.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!("duplicate behaviour row for skill {}", pair[0].0));
        }
    }

    let mut clouds: Vec<(u16, CloudSpec)> = Vec::new();
    for row in file.clouds {
        if row.skills.is_empty() {
            return Err("a cloud row has no skill ids".to_owned());
        }
        for texture in &row.textures {
            validate_texture(texture, "cloud texture", &[".tga", ".bmp"])?;
        }
        if row.textures.is_empty() {
            return Err("a cloud row has no textures".to_owned());
        }
        if row.count == 0 || row.count > 64 {
            return Err(format!("a cloud row has an invalid billboard count {}", row.count));
        }
        if !row.radius.is_finite() || row.radius <= 0.0 {
            return Err(format!("a cloud row has an invalid radius {}", row.radius));
        }
        for (name, value) in [
            ("base_height", row.base_height),
            ("height_variation", row.height_variation),
            ("size_variation", row.size_variation),
            ("spin_variation", row.spin_variation),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("a cloud row has an invalid {name} {value}"));
            }
        }
        if !row.size_min.is_finite() || row.size_min <= 0.0 {
            return Err(format!("a cloud row has an invalid size_min {}", row.size_min));
        }
        if !row.spin_min.is_finite() {
            return Err(format!("a cloud row has an invalid spin_min {}", row.spin_min));
        }
        if !row.alpha.is_finite() || !(0.0..=1.0).contains(&row.alpha) {
            return Err(format!("a cloud row has an invalid alpha {}", row.alpha));
        }
        for component in [row.color.0, row.color.1, row.color.2] {
            if !component.is_finite() || !(0.0..=1.0).contains(&component) {
                return Err(format!("a cloud row has an invalid color component {component}"));
            }
        }
        if row.duration_ms == 0 {
            return Err("a cloud row has a zero duration".to_owned());
        }
        if row.fade_in_ms + row.fade_out_ms > row.duration_ms {
            return Err("the cloud fade times do not fit the duration".to_owned());
        }

        let textures: &'static [&'static str] = Box::leak(
            row.textures
                .into_iter()
                .map(|texture| Box::leak(texture.into_boxed_str()) as &'static str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let spec = CloudSpec {
            phase: row.phase,
            textures,
            count: row.count as usize,
            radius: row.radius,
            base_height: row.base_height,
            height_variation: row.height_variation,
            size_min: row.size_min,
            size_variation: row.size_variation,
            spin_min: row.spin_min,
            spin_variation: row.spin_variation,
            alpha: row.alpha,
            color: Color::rgb(row.color.0, row.color.1, row.color.2),
            duration_ms: row.duration_ms,
            fade_in_ms: row.fade_in_ms,
            fade_out_ms: row.fade_out_ms,
        };

        for skill in row.skills {
            if skill == 0 {
                return Err("a cloud row uses skill id 0".to_owned());
            }
            if clouds.iter().any(|(existing, _)| *existing == skill) {
                return Err(format!("duplicate cloud row for skill {skill}"));
            }
            clouds.push((skill, spec));
        }
    }
    clouds.sort_by_key(|(skill, _)| *skill);

    let circle = file.cast_circle;
    validate_texture(&circle.ring_texture, "cast circle ring texture", &[".tga", ".bmp"])?;
    validate_texture(&circle.glow_texture, "cast circle glow texture", &[".tga", ".bmp"])?;
    if !circle.radius.is_finite() || circle.radius <= 0.0 {
        return Err(format!("invalid cast circle radius {}", circle.radius));
    }
    if !circle.height.is_finite() || circle.height < 0.0 {
        return Err(format!("invalid cast circle height {}", circle.height));
    }
    if !circle.inner_ring_scale.is_finite() || circle.inner_ring_scale <= 0.0 {
        return Err(format!("invalid cast circle inner ring scale {}", circle.inner_ring_scale));
    }
    if !circle.glow_scale.is_finite() || circle.glow_scale <= 0.0 {
        return Err(format!("invalid cast circle glow scale {}", circle.glow_scale));
    }
    for (name, value) in [
        ("ring_rotation_speed", circle.ring_rotation_speed),
        ("inner_ring_rotation_speed", circle.inner_ring_rotation_speed),
    ] {
        if !value.is_finite() {
            return Err(format!("invalid cast circle {name} {value}"));
        }
    }
    let cast_circle = CastCircleSpec {
        ring_texture: Box::leak(circle.ring_texture.into_boxed_str()),
        glow_texture: Box::leak(circle.glow_texture.into_boxed_str()),
        radius: circle.radius,
        height: circle.height,
        ring_rotation_speed: circle.ring_rotation_speed,
        inner_ring_rotation_speed: circle.inner_ring_rotation_speed,
        inner_ring_scale: circle.inner_ring_scale,
        glow_scale: circle.glow_scale,
    };

    let portal = file.warp_portal;
    validate_texture(&portal.streak_texture, "warp portal streak texture", &[".tga", ".bmp"])?;
    validate_texture(&portal.gradient_texture, "warp portal gradient texture", &[".tga", ".bmp"])?;
    validate_texture(&portal.particle_sprite, "warp portal particle sprite", &[".spr"])?;
    if !portal.skill_radius.is_finite() || portal.skill_radius <= 0.0 {
        return Err(format!("invalid warp portal skill radius {}", portal.skill_radius));
    }
    if !portal.skill_height.is_finite() || portal.skill_height < 0.0 {
        return Err(format!("invalid warp portal skill height {}", portal.skill_height));
    }
    if portal.skill_ring_count == 0 || portal.skill_ring_count > 8 {
        return Err(format!("invalid warp portal ring count {}", portal.skill_ring_count));
    }
    let warp_portal = WarpPortalSpec {
        streak_texture: Box::leak(portal.streak_texture.into_boxed_str()),
        gradient_texture: Box::leak(portal.gradient_texture.into_boxed_str()),
        particle_sprite: Box::leak(portal.particle_sprite.into_boxed_str()),
        skill_radius: portal.skill_radius,
        skill_height: portal.skill_height,
        skill_ring_count: portal.skill_ring_count,
    };

    let aura = file.level_aura;
    validate_texture(&aura.ring_liner_texture, "level aura ring liner texture", &[".tga", ".bmp"])?;
    validate_texture(&aura.ring_blue_texture, "level aura ring blue texture", &[".tga", ".bmp"])?;
    validate_texture(&aura.ring_red_texture, "level aura ring red texture", &[".tga", ".bmp"])?;
    validate_texture(&aura.bubble_texture, "level aura bubble texture", &[".tga", ".bmp"])?;
    validate_texture(&aura.sparkle_texture, "level aura sparkle texture", &[".tga", ".bmp"])?;
    if aura.glow_textures.is_empty() {
        return Err("the level aura has no glow textures".to_owned());
    }
    for texture in &aura.glow_textures {
        validate_texture(texture, "level aura glow texture", &[".tga", ".bmp"])?;
    }
    let level_aura = LevelAuraSpec {
        ring_liner_texture: Box::leak(aura.ring_liner_texture.into_boxed_str()),
        ring_blue_texture: Box::leak(aura.ring_blue_texture.into_boxed_str()),
        ring_red_texture: Box::leak(aura.ring_red_texture.into_boxed_str()),
        bubble_texture: Box::leak(aura.bubble_texture.into_boxed_str()),
        glow_textures: Box::leak(
            aura.glow_textures
                .into_iter()
                .map(|texture| Box::leak(texture.into_boxed_str()) as &'static str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        ),
        sparkle_texture: Box::leak(aura.sparkle_texture.into_boxed_str()),
    };

    Ok(SkillRegistry {
        behaviours,
        clouds,
        cast_circle,
        warp_portal,
        level_aura,
    })
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{
        FALLBACK, SkillPhase, behaviour, cast_circle, channels, cloud, grants_move_while_casting, has_warp_target, level_aura, validate,
        validate_compiled, warp_portal,
    };

    #[test]
    fn compiled_fallback_is_valid() {
        let summary = validate_compiled().expect("the committed skill data has to be valid");
        assert_eq!(summary.behaviours, 3);
        assert_eq!(summary.clouds, 2, "the cloud row covers the two Storm Gust ids");
        assert!(validate("not ron").is_err());
        assert!(validate("(provenance: \"x\")").is_err());
    }

    #[test]
    fn behaviour_flags_are_data_driven() {
        // Rolling Cutter (2036) is the only channeling skill.
        assert!(channels(SkillId(2036)));
        for skill_id in [27_u16, 278, 34, 0, 65_500] {
            assert!(!channels(SkillId(skill_id)), "skill {skill_id} must not channel");
        }

        // Warp Portal (27) is the only provisional warp-target skill.
        assert!(has_warp_target(SkillId(27)));
        for skill_id in [2036_u16, 278, 28, 0, 65_500] {
            assert!(
                !has_warp_target(SkillId(skill_id)),
                "skill {skill_id} must not show a warp portal"
            );
        }

        // Free Cast (278) is the only move-while-casting skill flag.
        assert!(grants_move_while_casting(SkillId(278)));
        for skill_id in [27_u16, 2036, 34, 0, 65_500] {
            assert!(
                !grants_move_while_casting(SkillId(skill_id)),
                "skill {skill_id} must not grant Free Cast"
            );
        }

        assert_eq!(
            behaviour(SkillId(2036)),
            Some(&super::SkillBehaviour {
                channeling: true,
                warp_target: false,
                move_while_casting: false,
            })
        );
        assert_eq!(behaviour(SkillId(65_500)), None);
    }

    #[test]
    fn storm_gust_cloud_is_data_driven() {
        for skill_id in [87_u16, 89] {
            let spec = cloud(SkillId(skill_id)).expect("Storm Gust cloud row");
            assert_eq!(spec.phase, SkillPhase::Success);
            assert_eq!(spec.count, 18);
            assert_eq!(spec.textures.len(), 7);
            assert_eq!(spec.textures[0], "effect\\cloud1.tga");
            assert_eq!(spec.textures[6], "effect\\cloud7.tga");
            assert_eq!(spec.radius, 22.0);
            assert_eq!(spec.base_height, 10.0);
            assert_eq!(spec.height_variation, 8.0);
            assert_eq!(spec.size_min, 16.0);
            assert_eq!(spec.size_variation, 16.0);
            assert_eq!(spec.spin_min, 0.15);
            assert_eq!(spec.spin_variation, 0.35);
            assert_eq!(spec.alpha, 0.55);
            assert_eq!(spec.duration_ms, 3_000);
            assert_eq!(spec.fade_in_ms, 250);
            assert_eq!(spec.fade_out_ms, 700);
        }

        // No other skill spawns the procedural cloud.
        for skill_id in [88_u16, 90, 2036, 65_500] {
            assert!(cloud(SkillId(skill_id)).is_none(), "skill {skill_id} must not have a cloud");
        }
    }

    #[test]
    fn procedural_assets_are_data_driven() {
        let circle = cast_circle();
        assert_eq!(circle.ring_texture, "effect\\ring_liner.tga");
        assert_eq!(circle.glow_texture, "effect\\alpha_down.tga");
        assert_eq!(circle.radius, 7.0);
        assert_eq!(circle.height, 0.2);
        assert_eq!(circle.ring_rotation_speed, 1.6);
        assert_eq!(circle.inner_ring_rotation_speed, -2.3);
        assert_eq!(circle.inner_ring_scale, 0.62);
        assert_eq!(circle.glow_scale, 1.15);

        let portal = warp_portal();
        assert_eq!(portal.streak_texture, "effect\\ring_blue.tga");
        assert_eq!(portal.gradient_texture, "effect\\alpha_down.tga");
        assert_eq!(portal.particle_sprite, "이팩트\\particle1.spr");
        assert_eq!(portal.skill_radius, 8.0);
        assert_eq!(portal.skill_height, 0.2);
        assert_eq!(portal.skill_ring_count, 2);

        let aura = level_aura();
        assert_eq!(aura.ring_liner_texture, "effect\\ring_liner.tga");
        assert_eq!(aura.ring_blue_texture, "effect\\ring_blue.tga");
        assert_eq!(aura.ring_red_texture, "effect\\ring_red.tga");
        assert_eq!(aura.bubble_texture, "effect\\whitelight.tga");
        assert_eq!(aura.glow_textures, &[
            "effect\\glow01.bmp",
            "effect\\glow02.bmp",
            "effect\\glow03.bmp",
            "effect\\glow04.bmp"
        ]);
        assert_eq!(aura.sparkle_texture, "effect\\pikapika2.bmp");
    }

    #[test]
    fn rejects_unknown_fields() {
        let tampered = FALLBACK.replace("    version: 1,", "    version: 1,\n    unknown_field: true,");
        let error = validate(&tampered).unwrap_err();
        assert!(error.contains("Unexpected field") || error.contains("unknown field"), "{error}");
    }

    #[test]
    fn rejects_duplicate_and_disabled_rows() {
        let duplicate = FALLBACK.replace(
            "        (skill: 278, move_while_casting: true),",
            "        (skill: 278, move_while_casting: true),\n        (skill: 278, move_while_casting: true),",
        );
        assert!(validate(&duplicate).unwrap_err().contains("duplicate behaviour row"));

        let disabled = FALLBACK.replace(
            "        (skill: 2036, channeling: true),",
            "        (skill: 2036, channeling: false),",
        );
        assert!(validate(&disabled).unwrap_err().contains("no enabled flag"));
    }

    #[test]
    fn rejects_bad_cloud_and_procedural_data() {
        let bad_texture = FALLBACK.replace("\"effect\\\\cloud1.tga\"", "\"effect\\\\cloud1.spr\"");
        assert!(validate(&bad_texture).unwrap_err().contains("cloud texture"));
        let bad_alpha = FALLBACK.replace("            alpha: 0.55,", "            alpha: 2.0,");
        assert!(validate(&bad_alpha).unwrap_err().contains("alpha"));
        let bad_count = FALLBACK.replace("            count: 18,", "            count: 0,");
        assert!(validate(&bad_count).unwrap_err().contains("billboard count"));
        let bad_circle = FALLBACK.replace("        radius: 7.0,", "        radius: 0.0,");
        assert!(validate(&bad_circle).unwrap_err().contains("cast circle radius"));
        let bad_sprite = FALLBACK.replace(
            "particle_sprite: \"이팩트\\\\particle1.spr\"",
            "particle_sprite: \"particle1.tga\"",
        );
        assert!(validate(&bad_sprite).unwrap_err().contains("particle sprite"));
    }
}
