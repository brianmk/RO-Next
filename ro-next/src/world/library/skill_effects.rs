//! Unified skill effect descriptor pipeline.
//!
//! Every visual of a skill cast is described by **one** data file:
//!
//! `archive/data/client/skill_effects.ron` (shipped inside the client archive,
//! loaded through [`GameFileLoader`] so a server can override it without
//! rebuilding the client). The same file is embedded with `include_str!` as the
//! compiled fallback, so the archive override and the compiled table can never
//! drift apart.
//!
//! # Model
//!
//! The file is a list of [`SkillEffectDescriptor`]s, one per `(skill, phase)`
//! pair. A descriptor owns an **ordered list of layers**; every layer of the
//! phase spawns once per cast, in file order. A layer has a kind:
//!
//! * `Str` - `.str` screen sprite (`file` relative to `data\texture\effect\`),
//! * `Spr` - animated `.spr`/`.act` sprite (`files` relative to
//!   `data\sprite\`),
//! * `Texture` - texture billboard / screen effect (`file` relative to
//!   `data\texture\`, rendered by the [`crate::world::effect::HitFlash`]
//!   primitive),
//! * `Cylinder` - texture cylinder / ground aura (the shared
//!   [`crate::world::effect::SkillAura`] renderer),
//! * `Primitive` - a procedural primitive (`HitFlash`, `Projectile`,
//!   `GroundAura`; defaults to `Projectile` for `Travel`/`Rain` layers and to
//!   `HitFlash` otherwise when the `primitive` field is omitted),
//! * `BodyFlash` - a short tint of the caster sprite (the official client's
//!   hard-coded body flashes such as `EF_PORTAL5`).
//!
//! Every layer has a spawn anchor (`Caster`, `Target`, `Ground`, `Overhead`,
//! `Body`, `Travel`, `Rain`, `Auto`) and the common layer properties (asset,
//! offset, scale, duration, blend/tint, sound, action, light...). The official
//! client's per-layer flags map directly:
//!
//! * `head: true` -> `anchor: Overhead` (above the entity's head, scaled,
//!   sitting-aware; the *only* overhead heuristic, there is no hard-coded skill
//!   list any more),
//! * `attachedEntity: true` -> the layer follows its entity and is attached to
//!   the per-entity effect budget / cleanup; `false` -> fixed world cell,
//! * `wav` -> the phase's official effect sound (the client's one sound lane
//!   per entity plays the first existing candidate),
//! * `action` -> optional caster action override (the separate `skill_actions`
//!   table remains the default, so sounds/actions stay separable).
//!
//! # Hand-written visuals and official rows
//!
//! The hand-written special cases of the old `skill_visuals()` table are
//! migrated into this data file as ordinary layers
//! (`tools/data/skill_effects_custom.ron`) and the old blanket
//! `has_custom_completion()` bypass is gone: a skill can combine handcrafted
//! layers (for example the Blessing angel) with the official layers of the same
//! phase (the `축복` sprite, texture glows, ...), and **all** layers spawn.
//!
//! # Lookup cost
//!
//! The loaded table is a process-wide static with leaked, process-lifetime
//! strings. Lookups are a binary search over the sorted `(skill, phase)` keys
//! and return `&'static` slices: the spawn path performs no allocation per
//! frame and the per-phase layer list is iterated in place (bounded by
//! [`MAX_LAYERS_PER_PHASE`] at spawn time).
//!
//! Status auras (Assumptio) are a separate, status-driven subsystem
//! ([`crate::world::effect::status_aura`]); they are not tied to skill packets
//! and therefore stay separable, but are validated by the same tests/refactors
//! of this pipeline.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use cgmath::Vector3;
use ragnarok_packets::SkillId;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use crate::graphics::Color;
use crate::loaders::GameFileLoader;
use crate::world::CasterAction;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_effects.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_effects.ron");

/// Upper bound of layers spawned for a single phase of a single cast.
///
/// The data file is hand-authored/generated and normally stays well below this
/// bound; the cap is a defensive budget so a malformed/overridden data file can
/// never spawn an unbounded amount of effects from one packet.
pub(crate) const MAX_LAYERS_PER_PHASE: usize = 32;

/// When a layer of a skill is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub(crate) enum SkillPhase {
    /// Ground-target skill completion, at the target cell.
    Ground,
    /// Cast start, on the caster.
    Cast,
    /// Successful no-damage/cast completion, on the caster.
    Success,
    /// Targeted skill lands, at the target.
    Release,
    /// Skill damage hits, at the target.
    Hit,
    /// Skill damage packet arrives, on the caster.
    Caster,
}

impl SkillPhase {
    /// Stable index used as the de-duplication key and the sort key of the
    /// per-skill table.
    pub(crate) const fn index(self) -> u8 {
        match self {
            Self::Ground => 0,
            Self::Cast => 1,
            Self::Success => 2,
            Self::Release => 3,
            Self::Hit => 4,
            Self::Caster => 5,
        }
    }
}

/// What a layer is rendered by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub(crate) enum LayerKind {
    /// `.str` screen sprite.
    Str,
    /// `.spr`/`.act` animated sprite.
    Spr,
    /// Texture billboard / screen effect.
    Texture,
    /// Texture cylinder / ground aura.
    Cylinder,
    /// Procedural primitive (`primitive` field selects it).
    Primitive,
    /// Short tint of the caster sprite.
    BodyFlash,
    /// Vertical sine wave of the caster's body layers (official `render.flags`
    /// bit 0x8: the body wobbles while the effect runs).
    BodyWave,
    /// Flat-colour silhouette copies of the body behind the sprite (official
    /// `render.flags` 0x800/0x80000 outline).
    BodyOutline,
    /// Short landing squash of the body (official `render.flags` bit 0x40).
    BodySquash,
}

/// Where a layer of a skill spawns.
///
/// `Overhead` and `Body` are shorthand for an overhead/body anchored layer on
/// the caster; the `height` field can be combined with `Caster`/`Target` to
/// place an overhead/body layer on a target entity (for example an ally buff).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum LayerAnchor {
    /// On the caster entity.
    Caster,
    /// At the target entity (or target cell, falling back to the caster).
    Target,
    /// At the ground cell of the skill (falling back to the caster).
    Ground,
    /// Above the caster's head.
    Overhead,
    /// At the caster's body height.
    Body,
    /// From the caster to the target (projectile style layers).
    Travel,
    /// From above onto the target cell (rain style layers).
    Rain,
    /// At the target when the packet knows one, on the caster otherwise.
    Auto,
}

/// Height anchor of an entity-attached layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub(crate) enum LayerVertical {
    /// At the anchor's feet (the `offset` decides the rest).
    #[default]
    Feet,
    /// At the body/torso height of the entity.
    Body,
    /// Above the entity's head (scaled, sitting-aware).
    Overhead,
}

/// Blending of a texture/cylinder layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub(crate) enum LayerBlend {
    /// Straight alpha blending.
    #[default]
    Alpha,
    /// Additive glow blending.
    Additive,
}

/// Growth animation of a cylinder layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub(crate) enum LayerGrow {
    /// Radii stay constant for the whole lifetime.
    #[default]
    Constant,
    /// Only the top radius grows (`EF_BEGINSPELL` cone).
    TopRadius,
    /// Both radii grow (expanding rings).
    BothRadii,
    /// Radii stay constant while the height grows from the ground (`EF_HEAL`).
    Height,
}

/// Shared procedural primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum LayerPrimitive {
    /// A burst of camera-facing billboard quads.
    HitFlash,
    /// One or more quads travelling/raining towards the target.
    Projectile,
    /// A cylinder / ring / ground field.
    GroundAura,
}

// ---------------------------------------------------------------------------
// Data file schema.
// ---------------------------------------------------------------------------

/// Accepts both a bare value and the explicit `Some(value)`/`None` RON option
/// spelling for optional layer fields.
///
/// The data file is authored by generators and humans; writing `anchor: Target`
/// (or `light: (255, 255, 255)`) is much more readable than wrapping every
/// optional field in `Some(...)`. The untagged helper buffers the value and
/// retries it as an [`Option`], so both spellings work and typos still fail.
#[derive(Deserialize)]
#[serde(untagged)]
enum BareOrOption<T> {
    Bare(T),
    Optional(Option<T>),
}

fn deserialize_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(match BareOrOption::<T>::deserialize(deserializer)? {
        BareOrOption::Bare(value) => Some(value),
        BareOrOption::Optional(value) => value,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillEffectFile {
    #[allow(dead_code)]
    provenance: String,
    #[serde(default)]
    #[allow(dead_code)]
    version: u32,
    effects: Vec<DescriptorFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DescriptorFile {
    skill: u16,
    phase: SkillPhase,
    #[serde(default, deserialize_with = "deserialize_option")]
    cast_circle_color: Option<(u8, u8, u8)>,
    #[serde(default, deserialize_with = "deserialize_option")]
    action: Option<CasterAction>,
    #[serde(default)]
    layers: Vec<LayerFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LayerFile {
    kind: LayerKind,
    #[serde(default, deserialize_with = "deserialize_option")]
    anchor: Option<LayerAnchor>,
    #[serde(default, deserialize_with = "deserialize_option")]
    attached_entity: Option<bool>,
    #[serde(default)]
    head: bool,
    #[serde(default, deserialize_with = "deserialize_option")]
    vertical: Option<LayerVertical>,
    #[serde(default, deserialize_with = "deserialize_option")]
    file: Option<String>,
    #[serde(default)]
    files: Vec<String>,
    /// Ordered animation frames of a `Texture` layer (a flipbook). When present
    /// the layer plays `frames` one after another for `frame_ms` each instead
    /// of drawing `file`. This is the official client's `totalTexture`
    /// primitive (`PP_3DTEXTURE`/`PP_3DCROSSTEXTURE` with a frame sequence,
    /// e.g. `thunder_ball_a..f` or `w_bubble%02d`).
    #[serde(default)]
    frames: Vec<String>,
    /// Milliseconds each `frames` entry is shown (default 80).
    #[serde(default)]
    frame_ms: u32,
    #[serde(default, deserialize_with = "deserialize_option")]
    primitive: Option<LayerPrimitive>,
    #[serde(default)]
    offset: (f32, f32, f32),
    #[serde(default = "default_scale")]
    scale: f32,
    #[serde(default)]
    size: f32,
    #[serde(default)]
    size_end: f32,
    #[serde(default)]
    duration_ms: u32,
    #[serde(default)]
    delay_ms: u32,
    /// Fraction of the actual cast time at which this layer spawns (0..1).
    /// Used by effects whose parts are spread over the cast (Asura glyphs);
    /// instant casts (no cast time) skip such layers.
    #[serde(default, deserialize_with = "deserialize_option")]
    cast_at: Option<f32>,
    #[serde(default = "default_count")]
    count: u16,
    #[serde(default)]
    spread: f32,
    #[serde(default)]
    height: f32,
    #[serde(default)]
    bottom_radius: f32,
    #[serde(default)]
    top_radius: f32,
    #[serde(default)]
    rotation_speed: f32,
    #[serde(default)]
    grow: LayerGrow,
    #[serde(default = "default_alpha")]
    alpha: f32,
    #[serde(default, deserialize_with = "deserialize_option")]
    tint: Option<(f32, f32, f32)>,
    #[serde(default)]
    blend: LayerBlend,
    #[serde(default, deserialize_with = "deserialize_option")]
    wav: Option<String>,
    #[serde(default)]
    action: Option<CasterAction>,
    #[serde(default, deserialize_with = "deserialize_option")]
    light: Option<(u8, u8, u8)>,
    #[serde(default)]
    light_intensity: f32,
    #[serde(default)]
    hold_ms: u32,
    #[serde(default)]
    fade_ms: u32,
    /// Amplitude (world units) of a `BodyWave` layer.
    #[serde(default = "default_amplitude")]
    amplitude: f32,
}

fn default_scale() -> f32 {
    1.0
}

fn default_amplitude() -> f32 {
    0.25
}

fn default_count() -> u16 {
    1
}

fn default_alpha() -> f32 {
    1.0
}

// ---------------------------------------------------------------------------
// Loaded representation (leaked, process-lifetime data).
// ---------------------------------------------------------------------------

/// One loaded layer of a skill phase.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SkillEffectLayer {
    pub kind: LayerKind,
    /// Effective spawn anchor (`Overhead`/`Body` are normalized to `Caster`
    /// plus [`Self::vertical`]).
    pub anchor: LayerAnchor,
    /// Height anchor for entity-attached layers.
    pub vertical: LayerVertical,
    /// Follow the anchor entity instead of using a fixed world cell.
    pub attached_entity: bool,
    /// Single asset path (`.str` file, texture or primitive texture).
    pub file: Option<&'static str>,
    /// Candidate `.spr` stems for `LayerKind::Spr`.
    pub files: &'static [&'static str],
    /// Ordered animation frames of a `Texture` layer (empty = single texture).
    pub frames: &'static [&'static str],
    /// Milliseconds per frame of a sequence layer.
    pub frame_ms: u32,
    /// Primitive of a `LayerKind::Primitive` layer.
    pub primitive: Option<LayerPrimitive>,
    /// World-space offset from the anchor's feet.
    pub offset: Vector3<f32>,
    /// Sprite/effect scale (`1.0` = authored size).
    pub scale: f32,
    /// Billboard world size at the start.
    pub size: f32,
    /// Billboard world size at the end.
    pub size_end: f32,
    /// Playback duration in milliseconds (`0` = native timing).
    pub duration_ms: u32,
    /// Duplicate delay / primitive stagger in milliseconds.
    pub delay_ms: u32,
    /// Fraction of the actual cast time at which the layer spawns (0..1).
    pub cast_at: Option<f32>,
    /// Number of duplicates/burst quads.
    pub count: u16,
    /// Random world offset radius of a burst.
    pub spread: f32,
    /// World height above the anchor (cylinder height for cylinder layers).
    pub height: f32,
    /// Bottom radius of a cylinder.
    pub bottom_radius: f32,
    /// Top radius of a cylinder.
    pub top_radius: f32,
    /// Cylinder rotation in radians per second.
    pub rotation_speed: f32,
    /// Cylinder growth animation.
    pub grow: LayerGrow,
    /// Layer alpha (`0..=1`).
    pub alpha: f32,
    /// RGB tint of texture/cylinder layers.
    pub tint: Color,
    /// Additive blending of a texture/cylinder layer.
    pub additive: bool,
    /// Official effect sound relative to `data\wav`.
    pub wav: Option<&'static str>,
    /// Optional caster action override.
    pub action: Option<CasterAction>,
    /// Optional point light of a `.str` layer.
    pub light: Option<(Color, f32)>,
    /// Hold time of a body flash in milliseconds.
    pub hold_ms: u32,
    /// Fade time of a body flash in milliseconds.
    pub fade_ms: u32,
    /// Amplitude of a body wave in world units.
    pub amplitude: f32,
}

/// One loaded `(skill, phase)` descriptor: the ordered layers of the phase.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SkillEffectDescriptor {
    pub skill: u16,
    pub phase: SkillPhase,
    pub layers: &'static [SkillEffectLayer],
    /// Official caster action override of any phase of the skill.
    pub action: Option<CasterAction>,
    /// Official cast circle tint of the skill.
    pub cast_circle_color: Option<Color>,
    /// Sound candidates of the phase (the `wav` of the layers, in order).
    pub sounds: &'static [&'static str],
}

/// Where a layer is spawned (pure decision, unit-testable).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayerPlacement {
    /// Follow the caster entity.
    SourceEntity,
    /// Follow the target entity.
    TargetEntity,
    /// Fixed world cell of the target.
    TargetPosition,
    /// Fixed world cell of the caster (when no target is known).
    SourcePosition,
}

impl SkillEffectLayer {
    /// Resolves where the layer spawns.
    ///
    /// `Travel`/`Rain` layers are projectile primitives that need both
    /// endpoints; they are resolved by the projectile spawner and return
    /// `None` here.
    pub(crate) fn placement(&self, has_target_entity: bool, has_target_position: bool) -> Option<LayerPlacement> {
        match self.anchor {
            LayerAnchor::Caster => Some(LayerPlacement::SourceEntity),
            LayerAnchor::Target | LayerAnchor::Auto => match (has_target_entity, has_target_position) {
                (true, _) => Some(LayerPlacement::TargetEntity),
                (false, true) => Some(LayerPlacement::TargetPosition),
                (false, false) => Some(LayerPlacement::SourceEntity),
            },
            LayerAnchor::Ground => match has_target_position {
                true => Some(LayerPlacement::TargetPosition),
                false => Some(LayerPlacement::SourcePosition),
            },
            // Only projectile primitives are endpoint layers; a `.str`/texture/
            // sprite layer with a travel/rain anchor still plays at the target.
            LayerAnchor::Travel | LayerAnchor::Rain => match (self.kind, self.primitive) {
                (LayerKind::Primitive, Some(LayerPrimitive::Projectile)) => None,
                _ => match (has_target_entity, has_target_position) {
                    (true, _) => Some(LayerPlacement::TargetEntity),
                    (false, true) => Some(LayerPlacement::TargetPosition),
                    (false, false) => Some(LayerPlacement::SourceEntity),
                },
            },
            // Normalized to `Caster` while loading; handled for completeness.
            LayerAnchor::Overhead | LayerAnchor::Body => Some(LayerPlacement::SourceEntity),
        }
    }
}

// ---------------------------------------------------------------------------
// Static tables.
// ---------------------------------------------------------------------------

/// The compiled fallback, parsed on first use (also used by tests).
static COMPILED: OnceLock<Vec<SkillEffectDescriptor>> = OnceLock::new();
/// The table loaded from the client archive, if the file exists there.
static ARCHIVE: OnceLock<Vec<SkillEffectDescriptor>> = OnceLock::new();

fn compiled() -> &'static [SkillEffectDescriptor] {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(descriptors) => descriptors,
        Err(error) => {
            crate::logging::log_warn!("skill effects: compiled fallback is invalid ({error})");
            Vec::new()
        }
    })
}

fn table() -> &'static [SkillEffectDescriptor] {
    ARCHIVE.get().map(Vec::as_slice).unwrap_or_else(compiled)
}

/// Loads the descriptor table from the client archive (or the compiled
/// fallback).
///
/// Returns the number of loaded `(skill, phase)` descriptors. A malformed data
/// file is reported and the compiled fallback is used instead, so a bad
/// override can never leave the client without skill visuals.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_info!("skill effects: loaded {} phases from the compiled fallback", compiled().len());
        return compiled().len();
    };

    let descriptors = match parse(content) {
        Ok(descriptors) => {
            crate::logging::log_info!("skill effects: loaded {} phases from the archive", descriptors.len());
            descriptors
        }
        Err(error) => {
            crate::logging::log_warn!("skill effects: invalid {DATA_PATH} ({error}), using the compiled fallback");
            compiled().to_vec()
        }
    };

    let count = descriptors.len();
    let _ = ARCHIVE.set(descriptors);
    count
}

/// The ordered layers of one skill phase (empty when the skill has no visual
/// for that phase).
pub(crate) fn skill_effect_layers(skill_id: SkillId, phase: SkillPhase) -> &'static [SkillEffectLayer] {
    descriptor(skill_id, phase).map(|descriptor| descriptor.layers).unwrap_or(&[])
}

/// The full descriptor of one skill phase.
pub(crate) fn descriptor(skill_id: SkillId, phase: SkillPhase) -> Option<&'static SkillEffectDescriptor> {
    let table = table();
    let key = (skill_id.0, phase.index());
    table
        .binary_search_by_key(&key, |descriptor| (descriptor.skill, descriptor.phase.index()))
        .ok()
        .map(|index| &table[index])
}

/// All descriptors of `skill_id` (any phase), sorted by phase order.
fn descriptors_of(skill_id: SkillId) -> &'static [SkillEffectDescriptor] {
    let table = table();
    let start = table.partition_point(|descriptor| descriptor.skill < skill_id.0);
    let end = table.partition_point(|descriptor| descriptor.skill <= skill_id.0);
    &table[start..end]
}

/// Whether the pipeline has any visual for `skill_id`.
pub(crate) fn skill_has_effect(skill_id: SkillId) -> bool {
    !descriptors_of(skill_id).is_empty()
}

/// Official cast circle tint of `skill_id`, if the data file overrides the
/// element color derived from the cast packet.
pub(crate) fn skill_cast_circle_color(skill_id: SkillId) -> Option<Color> {
    let descriptors = descriptors_of(skill_id);
    descriptors.iter().filter_map(|descriptor| descriptor.cast_circle_color).next()
}

/// Official caster action override of `skill_id` (the `skill_actions` table
/// remains the default).
pub(crate) fn skill_action_override(skill_id: SkillId) -> Option<CasterAction> {
    let descriptors = descriptors_of(skill_id);
    descriptors.iter().find_map(|descriptor| {
        descriptor
            .action
            .or_else(|| descriptor.layers.iter().find_map(|layer| layer.action))
    })
}

/// Sound candidates of `skill_id` for one phase (the `wav` fields of its
/// layers, in order; empty when the data file has no sound for the phase).
pub(crate) fn skill_effect_sound_files(skill_id: SkillId, phase: SkillPhase) -> &'static [&'static str] {
    descriptor(skill_id, phase).map(|descriptor| descriptor.sounds).unwrap_or(&[])
}

/// Validates a document against the schema and returns the number of
/// `(skill, phase)` descriptors it contains.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `ro-next --check-skill-effects [PATH]` so CI can validate the committed
/// data file (or an archive override) without starting a window or a network
/// connection.
pub(crate) fn validate(content: &str) -> Result<usize, String> {
    parse(content).map(|descriptors| descriptors.len())
}

/// Validates the compiled fallback (used by `--check-skill-effects` without a
/// path and by the tests).
pub(crate) fn validate_compiled() -> Result<usize, String> {
    validate(FALLBACK)
}

// ---------------------------------------------------------------------------
// Parsing and validation.
// ---------------------------------------------------------------------------

fn parse(content: &str) -> Result<Vec<SkillEffectDescriptor>, String> {
    let file: SkillEffectFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    validate_file(&file)?;
    Ok(build(file))
}

fn validate_file(file: &SkillEffectFile) -> Result<(), String> {
    if file.effects.is_empty() {
        return Err("the data file contains no skill effects".to_owned());
    }

    for descriptor in &file.effects {
        if descriptor.skill == 0 {
            return Err("a descriptor uses skill id 0".to_owned());
        }

        if descriptor.layers.is_empty() && descriptor.cast_circle_color.is_none() && descriptor.action.is_none() {
            return Err(format!(
                "skill {} phase {:?}: descriptor has no layers, cast circle color or action",
                descriptor.skill, descriptor.phase
            ));
        }

        for layer in &descriptor.layers {
            validate_layer(descriptor.skill, descriptor.phase, layer)?;
        }
    }

    Ok(())
}

fn validate_layer(skill: u16, phase: SkillPhase, layer: &LayerFile) -> Result<(), String> {
    let context = || format!("skill {skill} phase {phase:?} ({:?})", layer.kind);

    match layer.kind {
        LayerKind::Str => match layer.file.as_deref() {
            Some(file) if file.to_ascii_lowercase().ends_with(".str") => {}
            Some(file) => return Err(format!("{}: '.str' layer file does not end in .str: {file}", context())),
            None => return Err(format!("{}: '.str' layer has no file", context())),
        },
        LayerKind::Spr => {
            if layer.files.is_empty() {
                return Err(format!("{}: sprite layer has no candidate files", context()));
            }
            if layer.files.iter().any(|file| file.is_empty()) {
                return Err(format!("{}: sprite layer has an empty candidate file", context()));
            }
        }
        LayerKind::Texture | LayerKind::Cylinder => {
            if !layer.frames.is_empty() {
                if layer.kind != LayerKind::Texture {
                    return Err(format!("{}: only texture layers can have animation frames", context()));
                }
                if layer.frames.len() > 64 {
                    return Err(format!("{}: too many animation frames ({})", context(), layer.frames.len()));
                }
                if !(16..=2000).contains(&layer.frame_ms) {
                    return Err(format!("{}: frame_ms {} is outside 16..=2000", context(), layer.frame_ms));
                }
                if layer
                    .frames
                    .iter()
                    .any(|frame| !(frame.to_ascii_lowercase().ends_with(".tga") || frame.to_ascii_lowercase().ends_with(".bmp")))
                {
                    return Err(format!("{}: animation frames must be .tga/.bmp textures", context()));
                }
            } else {
                match layer.file.as_deref() {
                    Some(file) if !file.is_empty() => {}
                    _ => return Err(format!("{}: layer has no texture file", context())),
                }
            }
        }
        LayerKind::Primitive => match layer.file.as_deref() {
            Some(file) if !file.is_empty() => {}
            _ => return Err(format!("{}: primitive layer has no texture file", context())),
        },
        LayerKind::BodyFlash => {
            if layer.hold_ms > 10_000 || layer.fade_ms > 10_000 {
                return Err(format!("{}: body flash timings are out of range", context()));
            }
        }
        LayerKind::BodyWave => {
            if !(0.0..=2.0).contains(&layer.amplitude) || layer.duration_ms > 10_000 {
                return Err(format!("{}: body wave is out of range", context()));
            }
        }
        LayerKind::BodyOutline => {
            if layer.duration_ms == 0 || layer.duration_ms > 10_000 {
                return Err(format!("{}: body outline duration is out of range", context()));
            }
        }
        LayerKind::BodySquash => {
            if layer.duration_ms == 0 || layer.duration_ms > 10_000 || !(0.0..=0.9).contains(&layer.amplitude) {
                return Err(format!("{}: body squash is out of range", context()));
            }
        }
    }

    if let Some(wav) = layer.wav.as_deref()
        && !wav.to_ascii_lowercase().ends_with(".wav")
    {
        return Err(format!("{}: sound '{wav}' is not a .wav path", context()));
    }

    if !layer.scale.is_finite() || layer.scale <= 0.0 || layer.scale > 100.0 {
        return Err(format!("{}: invalid scale {}", context(), layer.scale));
    }
    if !layer.alpha.is_finite() || !(0.0..=1.0).contains(&layer.alpha) {
        return Err(format!("{}: invalid alpha {}", context(), layer.alpha));
    }
    if layer.duration_ms > 30_000 || layer.delay_ms > 30_000 {
        return Err(format!("{}: invalid duration/delay", context()));
    }
    if layer.count == 0 || layer.count > 16 {
        return Err(format!("{}: invalid duplicate count {}", context(), layer.count));
    }
    if layer.hold_ms > 10_000 || layer.fade_ms > 10_000 || layer.light_intensity > 1_000.0 {
        return Err(format!("{}: invalid timing/light values", context()));
    }
    for (name, value) in [
        ("size", layer.size),
        ("size_end", layer.size_end),
        ("spread", layer.spread),
        ("height", layer.height),
        ("bottom_radius", layer.bottom_radius),
        ("top_radius", layer.top_radius),
        ("rotation_speed", layer.rotation_speed),
        ("light_intensity", layer.light_intensity),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(format!("{}: invalid {name} {value}", context()));
        }
    }
    for value in [layer.offset.0, layer.offset.1, layer.offset.2] {
        if !value.is_finite() {
            return Err(format!("{}: invalid offset", context()));
        }
    }
    if let Some((red, green, blue)) = layer.tint
        && !(red.is_finite() && green.is_finite() && blue.is_finite())
    {
        return Err(format!("{}: invalid tint", context()));
    }

    Ok(())
}

/// Builds the sorted, merged runtime table.
fn build(file: SkillEffectFile) -> Vec<SkillEffectDescriptor> {
    // One entry per (skill, phase); several descriptors with the same key are
    // merged in file order (their layers are concatenated).
    let mut grouped: BTreeMap<(u16, u8), DescriptorBuilder> = BTreeMap::new();

    for descriptor in file.effects {
        let key = (descriptor.skill, descriptor.phase.index());
        let entry = grouped.entry(key).or_insert_with(|| DescriptorBuilder {
            skill: descriptor.skill,
            phase: descriptor.phase,
            layers: Vec::new(),
            action: None,
            cast_circle_color: None,
        });

        if entry.action.is_none() {
            entry.action = descriptor.action;
        }
        if entry.cast_circle_color.is_none() {
            entry.cast_circle_color = descriptor
                .cast_circle_color
                .map(|(red, green, blue)| Color::rgb_u8(red, green, blue));
        }

        for layer in descriptor.layers {
            entry.layers.push(layer.into_runtime());
        }
    }

    grouped
        .into_values()
        .map(|builder| {
            let layers: &'static [SkillEffectLayer] = Box::leak(builder.layers.into_boxed_slice());
            let mut sounds: Vec<&'static str> = Vec::new();
            for layer in layers {
                if let Some(wav) = layer.wav
                    && !sounds.contains(&wav)
                {
                    sounds.push(wav);
                }
            }
            let sounds: &'static [&'static str] = Box::leak(sounds.into_boxed_slice());

            SkillEffectDescriptor {
                skill: builder.skill,
                phase: builder.phase,
                layers,
                action: builder.action,
                cast_circle_color: builder.cast_circle_color,
                sounds,
            }
        })
        .collect()
}

struct DescriptorBuilder {
    skill: u16,
    phase: SkillPhase,
    layers: Vec<SkillEffectLayer>,
    action: Option<CasterAction>,
    cast_circle_color: Option<Color>,
}

impl LayerFile {
    fn into_runtime(self) -> SkillEffectLayer {
        // `head: true` (the official client's overhead flag) is the only overhead
        // heuristic; `Overhead`/`Body` are explicit height-anchored spellings. The
        // loaded layer keeps them (so the log and the data show the effective
        // overhead anchor instead of a bare `Caster`), while placement treats them
        // as entity-anchored on the caster.
        let (anchor, shorthand_vertical) = match self.anchor {
            Some(LayerAnchor::Overhead) => (LayerAnchor::Overhead, Some(LayerVertical::Overhead)),
            Some(LayerAnchor::Body) => (LayerAnchor::Body, Some(LayerVertical::Body)),
            Some(anchor) => (anchor, None),
            None if self.head => (LayerAnchor::Overhead, Some(LayerVertical::Overhead)),
            None => (LayerAnchor::Auto, None),
        };

        let vertical = self.vertical.or(shorthand_vertical).unwrap_or(match self.head {
            true => LayerVertical::Overhead,
            false => LayerVertical::Feet,
        });

        let attached_entity = self.attached_entity.unwrap_or(
            matches!(anchor, LayerAnchor::Caster | LayerAnchor::Overhead | LayerAnchor::Body) && self.kind != LayerKind::BodyFlash,
        );

        let file = self.file.map(|file| Box::leak(file.into_boxed_str()) as &'static str);
        let files: &'static [&'static str] = Box::leak(
            self.files
                .into_iter()
                .map(|file| Box::leak(file.into_boxed_str()) as &'static str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let frames: &'static [&'static str] = Box::leak(
            self.frames
                .into_iter()
                .map(|frame| Box::leak(frame.into_boxed_str()) as &'static str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let wav = self.wav.map(|wav| Box::leak(wav.into_boxed_str()) as &'static str);
        let tint = self.tint.map_or(Color::WHITE, |(red, green, blue)| Color::rgb(red, green, blue));
        let light = self
            .light
            .map(|(red, green, blue)| Color::rgb_u8(red, green, blue))
            .map(|color| (color, self.light_intensity));
        // The official effect table does not always spell out the primitive of a
        // hard-coded component; travelling components are projectiles, the rest
        // are billboard bursts.
        let primitive = self.primitive.or(match anchor {
            LayerAnchor::Travel | LayerAnchor::Rain => Some(LayerPrimitive::Projectile),
            _ => Some(LayerPrimitive::HitFlash),
        });

        SkillEffectLayer {
            kind: self.kind,
            anchor,
            vertical,
            attached_entity,
            file,
            files,
            frames,
            frame_ms: self.frame_ms,
            primitive,
            offset: Vector3::new(self.offset.0, self.offset.1, self.offset.2),
            scale: self.scale,
            size: self.size,
            size_end: self.size_end,
            duration_ms: self.duration_ms,
            delay_ms: self.delay_ms,
            cast_at: self.cast_at,
            count: self.count,
            spread: self.spread,
            height: self.height,
            bottom_radius: self.bottom_radius,
            top_radius: self.top_radius,
            rotation_speed: self.rotation_speed,
            grow: self.grow,
            alpha: self.alpha,
            tint,
            additive: self.blend == LayerBlend::Additive,
            wav,
            action: self.action,
            light: match light {
                Some((color, intensity)) if intensity > 0.0 => Some((color, intensity)),
                _ => None,
            },
            hold_ms: self.hold_ms,
            fade_ms: self.fade_ms,
            amplitude: self.amplitude,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use ragnarok_packets::SkillId;

    use super::{
        FALLBACK, LayerAnchor, LayerGrow, LayerKind, LayerPlacement, LayerPrimitive, LayerVertical, SkillEffectDescriptor, SkillPhase,
        descriptor, parse, skill_action_override, skill_cast_circle_color, skill_effect_layers, skill_effect_sound_files, skill_has_effect,
        validate, validate_compiled,
    };

    /// A small document that exercises every layer kind and anchor path. The
    /// data-specific assertions use this fixture so they stay valid when the
    /// generated/merged archive file changes (for example when the complete
    /// official layer rows replace the legacy approximations).
    const FIXTURE: &str = r#"(
        provenance: "test fixture",
        version: 1,
        effects: [
            (skill: 100, phase: Cast, cast_circle_color: (1, 2, 3), action: Idle, layers: [
                (kind: BodyFlash, tint: (1.0, 1.0, 0.0), hold_ms: 200, fade_ms: 600),
                (kind: Str, anchor: Caster, head: true, file: "a.str", light: (255, 255, 255), light_intensity: 10.0),
                (kind: Spr, anchor: Auto, attached_entity: true, head: true, files: ["이팩트\\a"], duration_ms: 500),
            ]),
            (skill: 100, phase: Success, layers: [
                (kind: Cylinder, anchor: Target, attached_entity: true, file: "effect\\ring.tga", duration_ms: 1500, height: 20.0, bottom_radius: 5.0, top_radius: 7.0, count: 4, delay_ms: 100, alpha: 0.5, tint: (0.2, 1.0, 0.2), blend: Additive, grow: BothRadii),
                (kind: Primitive, anchor: Rain, primitive: Projectile, file: "effect\\rain.tga", duration_ms: 600, count: 6, size: 7.0, height: 40.0),
                (kind: Str, anchor: Travel, file: "b.str"),
                (kind: Str, anchor: Caster, file: "c.str", wav: "effect\\c.wav"),
            ]),
        ],
    )"#;

    fn parsed_fixture() -> Vec<SkillEffectDescriptor> {
        super::parse(FIXTURE).expect("the test fixture has to be valid")
    }

    fn fixture_layers(phase: SkillPhase) -> &'static [super::SkillEffectLayer] {
        let descriptor = parsed_fixture()
            .into_iter()
            .find(|descriptor| descriptor.skill == 100 && descriptor.phase == phase)
            .expect("fixture phase");
        descriptor.layers
    }

    fn skil_effect_layers(skill_id: u16, phase: SkillPhase) -> &'static [super::SkillEffectLayer] {
        skill_effect_layers(SkillId(skill_id), phase)
    }

    /// Asura Strike uses the official `AsuraStrike` visuals: two white ring
    /// waves and the `asura1..3` figure around the caster. The old data had a
    /// generic yellow cast cone, which is why the skill looked like it had no
    /// animation.
    #[test]
    fn asura_strike_carries_the_official_charge_effect() {
        let cast = skil_effect_layers(271, SkillPhase::Cast);
        let textures: HashSet<&str> = cast.iter().filter_map(|layer| layer.file).collect();

        for expected in [
            "effect\\ring_white.tga",
            "effect\\asura1.tga",
            "effect\\asura2.tga",
            "effect\\asura3.tga",
        ] {
            assert!(textures.contains(expected), "missing {expected} in {textures:?}");
        }
    }

    /// Wind Walk (`SN_WINDWALK`, 383) plays two official lanes:
    ///
    /// * on use, `EF_BEGINSPELL6` (`SkillActionInfo.cpp:1750` ->
    ///   `GameModePackets.cpp:12947-12953`/`:17551-17559`,
    ///   `RagEffect.cpp:23584-23591`): two white `TexturedQuadRingSized` ring
    ///   sets (base alphas 45 and 25) plus the inline
    ///   `effect\EF_BeginSpell.wav`;
    /// * on success, `EF_PORTAL4` (`GameModePackets.cpp:19098`,
    ///   `RagEffect.cpp:23300-23311`): the `Cloud11Burst(0)` wind columns, the
    ///   inline `effect\윈드워크.wav` and the caster's `SetColor(-1, 220, 250,
    ///   220)` + `render.flags += 1` window on frames 5-25, which at the 16 ms
    ///   client step is an 80 ms delay and a 336 ms hold.
    ///
    /// There is no `.str` clip and no status-linked persistent visual:
    /// `EFST_WINDWALK` sits in the no-op groups of
    /// `CGameMode::ApplyStatusChange` (`GameModePackets.cpp:6372`, `:7061`) and
    /// is absent from `CPlayer::ReapplySkillEffects`
    /// (`Player.cpp:3180-3837`), so only the status icon row exists
    /// (`status_effects.ron` id 116).
    #[test]
    fn wind_walk_carries_its_two_official_lanes() {
        let cast = skil_effect_layers(383, SkillPhase::Cast);
        assert_eq!(cast.len(), 2, "EF_BEGINSPELL6 arms two ring sets");

        for layer in cast {
            assert_eq!(layer.kind, LayerKind::Cylinder);
            assert_eq!(layer.anchor, LayerAnchor::Caster);
            assert!(layer.attached_entity);
            assert_eq!(layer.file, Some("effect\\ring_white.tga"));
            assert_eq!(layer.duration_ms, 896);
            assert_eq!(layer.height, 2.0);
            assert_eq!(layer.bottom_radius, 4.1);
            assert_eq!(layer.top_radius, 4.1);
            assert!(layer.additive);
            assert_eq!(layer.tint, crate::graphics::Color::WHITE);
        }
        assert_eq!(cast[0].alpha, 0.35);
        assert_eq!(cast[1].alpha, 0.2);
        assert_eq!(cast[0].wav, Some("effect\\EF_BeginSpell.wav"));
        assert_eq!(cast[1].wav, None, "the wave plays once per cast");

        let success = skil_effect_layers(383, SkillPhase::Success);
        assert_eq!(success.len(), 6, "five cloud11 columns + the caster tint");

        for layer in &success[..5] {
            assert_eq!(layer.kind, LayerKind::Texture);
            assert_eq!(layer.anchor, LayerAnchor::Caster);
            assert!(layer.attached_entity);
            assert_eq!(layer.file, Some("effect\\cloud11.tga"));
        }
        assert_eq!(success[1].wav, Some("effect\\윈드워크.wav"));
        assert!(success[0].wav.is_none(), "the wave belongs to the first offset column");

        let flash = success[5];
        assert_eq!(flash.kind, LayerKind::BodyFlash);
        assert_eq!(flash.delay_ms, 80, "stateCnt 5 opens the tint window");
        assert_eq!(flash.hold_ms, 336, "stateCnt 5..=25 is 21 frames of 16 ms");
        assert!(flash.additive);
        for (channel, expected) in [
            (flash.tint.red, 220.0 / 255.0),
            (flash.tint.green, 250.0 / 255.0),
            (flash.tint.blue, 220.0 / 255.0),
        ] {
            assert!((channel - expected).abs() < 0.001, "{channel} != {expected}");
        }
    }

    /// Dumps the globals of the remaining `luafiles514` tables (data info,
    /// sprite data, external settings) to find any animation/effect data.
    #[test]
    #[ignore]
    fn probe_luafiles514_tables() {
        use mlua::Lua;
        use ro_next_loaders::FileLoader;

        let loader = crate::loaders::GameFileLoader::default();
        loader.load_archives_from_settings();
        loader.load_patched_lua_files();

        let prerequisites = [
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\datainfo\\jobname.lub",
            "data\\luafiles514\\lua files\\datainfo\\accessoryid.lub",
            "data\\luafiles514\\lua files\\datainfo\\accname.lub",
        ];
        let candidates = [
            "data\\luafiles514\\lua files\\datainfo\\weapontable.lub",
            "data\\luafiles514\\lua files\\datainfo\\pcjobnamegender.lub",
            "data\\luafiles514\\lua files\\datainfo\\shadowtable.lub",
            "data\\luafiles514\\lua files\\datainfo\\spriterobeid.lub",
            "data\\luafiles514\\lua files\\spreditinfo\\spreditinfo_f.lub",
            "data\\luafiles514\\lua files\\skilleffectinfo\\skilleffectinfo_f.lub",
            "data\\luafiles514\\lua files\\newskillinfo\\skillinfo_f.lub",
            "data\\luafiles514\\lua files\\emotion\\emotionlist.lub",
            "data\\luafiles514\\lua files\\service_korea\\externalsettings_kr.lub",
        ];

        let mut out = String::new();
        for candidate in candidates {
            let lua = Lua::new();
            for path in prerequisites {
                if let Ok(bytes) = loader.get(path) {
                    let _ = lua.load(&bytes).exec();
                }
            }

            let before: Vec<String> = lua
                .globals()
                .pairs::<mlua::Value, mlua::Value>()
                .filter_map(|pair| match pair {
                    Ok((mlua::Value::String(name), _)) => Some(name.to_string_lossy().to_string()),
                    _ => None,
                })
                .collect();

            let bytes = match loader.get(candidate) {
                Ok(bytes) => bytes,
                Err(_) => {
                    out.push_str(&format!("{candidate}: missing\n"));
                    continue;
                }
            };
            match lua.load(&bytes).exec() {
                Ok(()) => out.push_str(&format!("{candidate}: loaded\n")),
                Err(error) => out.push_str(&format!("{candidate}: failed: {error}\n")),
            }

            for pair in lua.globals().pairs::<mlua::Value, mlua::Value>() {
                let Ok((mlua::Value::String(name), mlua::Value::Table(table))) = pair else {
                    continue;
                };
                let name = name.to_string_lossy().to_string();
                if before.contains(&name) {
                    continue;
                }
                let mut sample = String::new();
                for pair in table.clone().pairs::<mlua::Value, mlua::Value>().take(6) {
                    let Ok((key, value)) = pair else { continue };
                    sample.push_str(&format!("{key:?}={value:?}; "));
                }
                out.push_str(&format!("  new table {name} len={:?} sample: {sample}\n", table.len().ok()));
            }
        }

        std::fs::write("/tmp/luafiles514_probe.txt", &out).unwrap();
        println!("wrote {} bytes", out.len());
    }

    /// Dumps the official 2019 client skill tables from the GRF archives
    /// (`skillid`/`effectid`/`skilleffectinfolist`/`actorstate`). Used by the
    /// parity audit; writes a flat, line oriented table to
    /// `/tmp/official_skill_tables.txt`.
    #[test]
    #[ignore]
    fn dump_official_skill_tables() {
        use mlua::Lua;
        use ro_next_loaders::FileLoader;

        let loader = crate::loaders::GameFileLoader::default();
        loader.load_archives_from_settings();
        loader.load_patched_lua_files();

        let lua = Lua::new();
        for path in [
            "data\\luafiles514\\lua files\\skillinfoz\\jobinheritlist.lub",
            "data\\luafiles514\\lua files\\skillinfoz\\skillid.lub",
            "data\\luafiles514\\lua files\\skilleffectinfo\\effectid.lub",
            "data\\luafiles514\\lua files\\skilleffectinfo\\actorstate.lub",
            "data\\luafiles514\\lua files\\skilleffectinfo\\skilleffectinfolist.lub",
            "data\\luafiles514\\lua files\\skilleffectinfo\\skilleffectinfo_f.lub",
            "data\\luafiles514\\lua files\\newskillinfo\\skillid.lub",
        ] {
            match loader.get(path) {
                Ok(bytes) => match lua.load(&bytes).exec() {
                    Ok(()) => println!("loaded {path}"),
                    Err(error) => println!("execute failed {path}: {error}"),
                },
                Err(error) => println!("missing {path}: {error:?}"),
            }
        }

        let mut out = String::new();

        for name in [
            "SKID",
            "EFID",
            "ACTOR_STATE",
            "SKILL_EFFECT_INFO_LIST",
            "SKILL_EFFECT_INFO",
            "SKILL_INFO_LIST",
        ] {
            if let Ok(table) = lua.globals().get::<mlua::Table>(name) {
                out.push_str(&format!("table|{name}|{}\n", table.len().unwrap_or(0)));
            }
        }

        // SKID: skill Aegis name -> id.
        if let Ok(skid) = lua.globals().get::<mlua::Table>("SKID") {
            for pair in skid.pairs::<String, i64>() {
                let Ok((name, id)) = pair else { continue };
                out.push_str(&format!("skillid|{id}|{name}\n"));
            }
        }

        // EFID: effect Aegis name -> id.
        if let Ok(efid) = lua.globals().get::<mlua::Table>("EFID") {
            for pair in efid.pairs::<String, i64>() {
                let Ok((name, id)) = pair else { continue };
                out.push_str(&format!("effectid|{id}|{name}\n"));
            }
        }

        // SKILL_EFFECT_INFO_LIST: skill id -> effect fields.
        let Ok(list) = lua.globals().get::<mlua::Table>("SKILL_EFFECT_INFO_LIST") else {
            panic!("SKILL_EFFECT_INFO_LIST is missing from the client tables");
        };

        for pair in list.pairs::<i64, mlua::Table>() {
            let Ok((skill, entry)) = pair else { continue };

            for field in entry.clone().pairs::<String, mlua::Value>() {
                let Ok((key, value)) = field else { continue };

                match value {
                    mlua::Value::Integer(number) => out.push_str(&format!("skill|{skill}|{key}|{number}\n")),
                    mlua::Value::Number(number) => out.push_str(&format!("skill|{skill}|{key}|{number}\n")),
                    mlua::Value::String(text) => {
                        let bytes = text.as_bytes();
                        match std::str::from_utf8(&bytes) {
                            Ok(text) => out.push_str(&format!("skill|{skill}|{key}|{text}\n")),
                            Err(_) => {
                                let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                                out.push_str(&format!("skill|{skill}|{key}|hex:{hex}\n"));
                            }
                        }
                    }
                    mlua::Value::Table(table) => {
                        for (index, value) in table.sequence_values::<mlua::Value>().enumerate() {
                            match value {
                                Ok(mlua::Value::Integer(number)) => out.push_str(&format!("skill|{skill}|{key}[{index}]|{number}\n")),
                                Ok(mlua::Value::String(text)) => {
                                    let bytes = text.as_bytes();
                                    match std::str::from_utf8(&bytes) {
                                        Ok(text) => out.push_str(&format!("skill|{skill}|{key}[{index}]|{text}\n")),
                                        Err(_) => {
                                            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                                            out.push_str(&format!("skill|{skill}|{key}[{index}]|hex:{hex}\n"))
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // ACTOR_STATE: state name -> id (used for the caster animation).
        if let Ok(states) = lua.globals().get::<mlua::Table>("ACTOR_STATE") {
            for pair in states.pairs::<String, i64>() {
                let Ok((name, id)) = pair else { continue };
                out.push_str(&format!("actorstate|{id}|{name}\n"));
            }
        }

        std::fs::write("/tmp/official_skill_tables.txt", &out).unwrap();
        println!("wrote {} bytes", out.len());
    }

    #[test]
    fn compiled_fallback_is_valid_and_broad() {
        let count = validate_compiled().expect("the compiled skill effect fallback has to be valid");
        assert!(count > 250, "the unified data file should stay broad, got {count} phases");
        assert!(skill_has_effect(SkillId(5)), "Bash");
        assert!(skill_has_effect(SkillId(34)), "Blessing");
        assert!(!skill_has_effect(SkillId(65_500)));
    }

    #[test]
    fn a_phase_can_have_several_layers_of_different_kinds() {
        let cast = fixture_layers(SkillPhase::Cast);
        assert_eq!(cast.len(), 3);
        let kinds: HashSet<LayerKind> = cast.iter().map(|layer| layer.kind).collect();
        assert_eq!(kinds, HashSet::from([LayerKind::BodyFlash, LayerKind::Str, LayerKind::Spr]));

        // The committed data file combines the hand-written Blessing angel with
        // the official layers of the same phase (this is what the old
        // `has_custom_completion` bypass used to prevent).
        let blessing = skil_effect_layers(34, SkillPhase::Success);
        let committed_kinds: HashSet<LayerKind> = blessing.iter().map(|layer| layer.kind).collect();
        assert!(
            committed_kinds.contains(&LayerKind::Str),
            "missing the hand-written angel layer: {blessing:?}"
        );
        assert!(
            committed_kinds.contains(&LayerKind::Spr),
            "missing the official sprite layer: {blessing:?}"
        );
        assert!(
            committed_kinds.contains(&LayerKind::Texture),
            "missing the official texture layer: {blessing:?}"
        );

        // Increment AGI is the official effect only (three `ac_center2` bursts
        // plus the `agi_up.bmp` glow); the old hand-written `blessingsofsoul`
        // overlay made it look like Fury and must not come back.
        let inc_agi = skil_effect_layers(29, SkillPhase::Success);
        assert!(inc_agi.iter().all(|layer| layer.kind != LayerKind::Str));
        assert!(inc_agi.iter().any(|layer| layer.kind == LayerKind::Texture));
    }

    #[test]
    fn head_flag_drives_the_overhead_anchor() {
        let cast = fixture_layers(SkillPhase::Cast);

        let overhead = cast.iter().find(|layer| layer.kind == LayerKind::Str).expect("overhead str layer");
        assert_eq!(overhead.anchor, LayerAnchor::Caster);
        assert_eq!(overhead.vertical, LayerVertical::Overhead, "head: true => overhead");
        assert!(overhead.light.is_some());

        // Spr layer: explicit Auto anchor + head => target-or-caster placement
        // with the overhead height anchor.
        let sprite = cast.iter().find(|layer| layer.kind == LayerKind::Spr).expect("sprite layer");
        assert_eq!(sprite.anchor, LayerAnchor::Auto);
        assert_eq!(sprite.vertical, LayerVertical::Overhead);
        assert!(sprite.attached_entity);

        // The committed Angelus row is the official `head: true` STR effect.
        let angelus = skil_effect_layers(33, SkillPhase::Success);
        assert!(
            angelus
                .iter()
                .any(|layer| { layer.file == Some("angelus.str") && layer.vertical == LayerVertical::Overhead })
        );
    }

    #[test]
    fn official_head_rows_and_body_aura_exceptions_are_data_driven() {
        // Blessing's official effect-42 layers must all be present and unchanged.
        let blessing = skil_effect_layers(34, SkillPhase::Success);
        assert!(
            blessing
                .iter()
                .any(|layer| layer.kind == LayerKind::Spr && layer.files.iter().any(|file| file.contains("축복")))
        );
        assert!(
            blessing
                .iter()
                .filter(|layer| layer.files.iter().any(|file| file.contains("particle6")))
                .count()
                >= 2,
            "the two official particle6 layers have to stay in the data"
        );
        assert!(
            blessing
                .iter()
                .any(|layer| layer.file == Some("effect\\pok2.tga") && layer.vertical == LayerVertical::Feet)
        );

        // Hammer Fall uses the official effect-102 `crashearth.str`, not the old
        // black_hammerfall placeholder.
        let hammerfall = skil_effect_layers(110, SkillPhase::Success);
        assert!(hammerfall.iter().any(|layer| layer.file == Some("crashearth.str")));
        assert!(!hammerfall.iter().any(|layer| layer.file == Some("black_hammerfall.str")));

        // Overhead support `.str` layers the official table does not flag: the
        // legacy hand-verified knowledge now lives in the data (`head: true`).
        for skill_id in [33_u16, 66, 75, 59, 62, 357] {
            assert!(
                skil_effect_layers(skill_id, SkillPhase::Success).iter().any(|layer| {
                    layer.kind == LayerKind::Str
                        && layer.vertical == LayerVertical::Overhead
                        && matches!(layer.anchor, LayerAnchor::Overhead | LayerAnchor::Auto)
                }),
                "skill {skill_id} must keep its overhead anchor (anchor field must not say Caster for head layers)"
            );
        }

        // Weapon/body auras deliberately stay at torso height instead.
        for skill_id in [60_u16, 258, 495] {
            assert!(
                skil_effect_layers(skill_id, SkillPhase::Success)
                    .iter()
                    .any(|layer| layer.kind == LayerKind::Str && layer.vertical == LayerVertical::Body),
                "skill {skill_id} is a body/weapon aura and must not float overhead"
            );
        }
    }

    #[test]
    fn legacy_and_official_rows_keep_their_anchor() {
        // `effectIdOnCaster`-style rows stay attached to the caster.
        let pierce = skil_effect_layers(56, SkillPhase::Success);
        assert!(
            pierce
                .iter()
                .any(|layer| layer.anchor == LayerAnchor::Caster && layer.attached_entity)
        );

        // `hitEffectId`-style rows are aimed at the target (the official row may
        // be entity-attached or a fixed cell depending on `attachedEntity`).
        let hit = skil_effect_layers(56, SkillPhase::Hit);
        assert!(hit.iter().any(|layer| layer.anchor == LayerAnchor::Target));

        // Travel/Rain `.str`/texture layers still play at the target, only
        // projectile primitives use the two endpoints.
        let success = fixture_layers(SkillPhase::Success);
        let travel_str = success.iter().find(|layer| layer.file == Some("b.str")).expect("travel str layer");
        assert_eq!(travel_str.anchor, LayerAnchor::Travel);
        assert_eq!(travel_str.placement(true, true), Some(LayerPlacement::TargetEntity));
        assert_eq!(travel_str.placement(false, true), Some(LayerPlacement::TargetPosition));
    }

    #[test]
    fn placement_follows_the_anchor_and_the_packet_data() {
        let success = fixture_layers(SkillPhase::Success);

        // Target anchor: follow the target entity, fall back to the target cell,
        // then to the caster.
        let target = success.iter().find(|layer| layer.kind == LayerKind::Cylinder).expect("cylinder");
        assert_eq!(target.placement(true, true), Some(LayerPlacement::TargetEntity));
        assert_eq!(target.placement(false, true), Some(LayerPlacement::TargetPosition));
        assert_eq!(target.placement(false, false), Some(LayerPlacement::SourceEntity));

        // Auto anchor behaves the same for the target-or-caster layers.
        let sprite = fixture_layers(SkillPhase::Cast)
            .iter()
            .find(|layer| layer.kind == LayerKind::Spr)
            .expect("sprite");
        assert_eq!(sprite.placement(true, true), Some(LayerPlacement::TargetEntity));
        assert_eq!(sprite.placement(false, true), Some(LayerPlacement::TargetPosition));
        assert_eq!(sprite.placement(false, false), Some(LayerPlacement::SourceEntity));

        // Caster anchor always follows the caster.
        let caster = success.iter().find(|layer| layer.file == Some("c.str")).expect("caster str");
        assert_eq!(caster.placement(true, true), Some(LayerPlacement::SourceEntity));

        // Projectile layers are endpoints, not a placement.
        let projectile = success
            .iter()
            .find(|layer| layer.primitive == Some(LayerPrimitive::Projectile))
            .expect("projectile");
        assert_eq!(projectile.placement(true, true), None);
    }

    #[test]
    fn cylinders_carry_growth_blending_and_duplicates() {
        let cylinder = fixture_layers(SkillPhase::Success)
            .iter()
            .find(|layer| layer.kind == LayerKind::Cylinder)
            .expect("cylinder");
        assert_eq!(cylinder.grow, LayerGrow::BothRadii);
        assert_eq!(cylinder.count, 4);
        assert_eq!(cylinder.delay_ms, 100);
        assert!(cylinder.additive);
        assert_eq!(cylinder.bottom_radius, 5.0);
        assert_eq!(cylinder.top_radius, 7.0);
        assert_eq!(cylinder.height, 20.0);
        assert!(cylinder.attached_entity);

        // The committed Zen cast phase still has its growing cone and body flash.
        let zen = skil_effect_layers(401, SkillPhase::Cast);
        assert!(
            zen.iter()
                .any(|layer| layer.kind == LayerKind::BodyFlash && layer.hold_ms == 200 && layer.fade_ms == 600)
        );
        assert!(
            zen.iter()
                .any(|layer| layer.kind == LayerKind::Cylinder && layer.grow == LayerGrow::TopRadius)
        );
    }

    #[test]
    fn heal_columns_are_anchored_on_the_healed_target() {
        let heal = skil_effect_layers(28, SkillPhase::Success);
        let columns: Vec<_> = heal
            .iter()
            .filter(|layer| {
                layer.kind == LayerKind::Cylinder
                    && matches!(layer.anchor, LayerAnchor::Target | LayerAnchor::Auto)
                    && layer.grow == LayerGrow::Height
                    && layer.attached_entity
                    && layer.tint.green > layer.tint.red
                    && layer.tint.green > layer.tint.blue
            })
            .collect();
        assert!(
            !columns.is_empty(),
            "the green EF_HEAL columns must stay on the healed target: {heal:?}"
        );
    }

    #[test]
    fn texture_sequences_are_part_of_the_schema() {
        let content = r#"(
            provenance: "sequence fixture",
            effects: [(
                skill: 100,
                phase: Success,
                layers: [(kind: Texture, frames: ["a.tga", "b.tga"], frame_ms: 80, size: 4.0, duration_ms: 160)],
            )],
        )"#;
        let descriptors = parse(content).expect("a frame sequence is valid");
        let layer = descriptors[0].layers[0];
        assert_eq!(layer.frames, &["a.tga", "b.tga"]);
        assert_eq!(layer.frame_ms, 80);
        assert!(layer.file.is_none());

        let too_short = content.replace("frame_ms: 80", "frame_ms: 1");
        assert!(parse(&too_short).is_err(), "frame_ms below 16 must be rejected");
        let bad_frame = content.replace("b.tga", "b.tga?");
        assert!(parse(&bad_frame).is_err(), "non-texture frames must be rejected");
    }

    #[test]
    fn cast_circle_tints_are_data_driven() {
        assert_eq!(
            skill_cast_circle_color(SkillId(27)),
            Some(crate::graphics::Color::rgb_u8(160, 130, 255))
        );
        assert_eq!(
            skill_cast_circle_color(SkillId(34)),
            Some(crate::graphics::Color::rgb_u8(255, 230, 160))
        );
        // Skills without an override follow the element color of the packet.
        assert_eq!(skill_cast_circle_color(SkillId(89)), None);
    }

    #[test]
    fn sound_candidates_come_from_the_phase_layers() {
        assert!(skill_effect_sound_files(SkillId(33), SkillPhase::Success).contains(&"effect\\ef_angelus.wav"));
        assert!(skill_effect_sound_files(SkillId(45), SkillPhase::Success).contains(&"effect\\ac_concentration.wav"));

        // The fixture wav is exposed as a sound candidate of the phase.
        let sounds = fixture_layers(SkillPhase::Success)
            .iter()
            .filter_map(|layer| layer.wav)
            .collect::<Vec<_>>();
        assert_eq!(sounds, vec!["effect\\c.wav"]);
    }

    #[test]
    fn action_override_is_part_of_the_descriptor() {
        let descriptor = parsed_fixture()
            .into_iter()
            .find(|descriptor| descriptor.skill == 100 && descriptor.phase == SkillPhase::Cast)
            .expect("fixture cast descriptor");
        let expected = Some(crate::world::CasterAction::Idle);
        assert_eq!(descriptor.action, expected);
        assert_eq!(descriptor.cast_circle_color, Some(crate::graphics::Color::rgb_u8(1, 2, 3)));

        // The official fragment now carries the caster action of the skill
        // (Blessing uses the explicit `SKILL` action); the descriptor override is
        // consulted before the separate `skill_actions` table.
        assert_eq!(skill_action_override(SkillId(34)), Some(crate::world::CasterAction::Skill));
    }

    #[test]
    fn loaded_layer_fields_are_sane() {
        let parsed = super::parse(FALLBACK).expect("valid fallback");
        assert!(!parsed.is_empty());
        for descriptor in parsed {
            assert_ne!(descriptor.skill, 0);
            for layer in descriptor.layers {
                assert!(layer.scale > 0.0);
                assert!((0.0..=1.0).contains(&layer.alpha));
                match layer.kind {
                    LayerKind::Str => assert!(layer.file.is_some_and(|file| file.ends_with(".str"))),
                    LayerKind::Spr => assert!(!layer.files.is_empty()),
                    LayerKind::Texture => assert!(
                        layer.file.is_some() || !layer.frames.is_empty(),
                        "a texture layer has a file or an animation frame sequence"
                    ),
                    LayerKind::Cylinder => assert!(layer.file.is_some()),
                    LayerKind::Primitive => {
                        assert!(layer.primitive.is_some());
                        assert!(layer.file.is_some());
                    }
                    LayerKind::BodyFlash | LayerKind::BodyWave | LayerKind::BodyOutline | LayerKind::BodySquash => {}
                }
            }
        }
    }

    #[test]
    fn every_skill_phase_is_sorted_and_unique() {
        let parsed = super::parse(FALLBACK).expect("valid fallback");
        assert!(
            parsed
                .windows(2)
                .all(|pair| { (pair[0].skill, pair[0].phase.index()) < (pair[1].skill, pair[1].phase.index()) })
        );
    }

    #[test]
    fn malformed_data_is_rejected_without_panicking() {
        assert!(validate("not ron").is_err());
        assert!(validate("(provenance: \"x\", effects: [])").is_err());

        // Unknown phase / kind.
        assert!(validate("(provenance: \"x\", effects: [(skill: 1, phase: Nope, layers: [(kind: Str, file: \"a.str\")])])").is_err());
        assert!(validate("(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Nope, file: \"a.str\")])])").is_err());

        // Missing/incorrect kind-specific assets.
        assert!(validate("(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Str, file: \"a.tga\")])])").is_err());
        assert!(validate("(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Spr, files: [])])])").is_err());
        // A primitive layer needs a texture; the primitive itself defaults by
        // anchor (Travel/Rain -> Projectile, otherwise HitFlash).
        assert!(validate("(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Primitive)])])").is_err());
        assert!(
            validate(
                "(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Texture, file: \"effect\\\\a.tga\", alpha: 2.0)])])"
            )
            .is_err()
        );
        assert!(
            validate(
                "(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Texture, file: \"effect\\\\a.tga\", scale: 0.0)])])"
            )
            .is_err()
        );

        // Unknown fields are rejected by the schema (typos cannot silently do
        // nothing in an archive override).
        assert!(
            validate(
                "(provenance: \"x\", effects: [(skill: 1, phase: Hit, layers: [(kind: Texture, file: \"effect\\\\a.tga\", filesz: 1.0)])])"
            )
            .is_err()
        );
    }

    #[test]
    fn a_valid_minimal_document_is_accepted() {
        let count = validate(
            "(provenance: \"x\", version: 1, effects: [(skill: 1, phase: Cast, cast_circle_color: (1, 2, 3), layers: []),(skill: 2, \
             phase: Success, layers: [(kind: Str, file: \"a.str\"), (kind: Spr, files: [\"이팩트\\\\a\"], duration_ms: 500)]),])",
        )
        .expect("minimal document should validate");
        assert_eq!(count, 2);
    }

    #[test]
    fn descriptor_lookup_uses_the_sorted_table() {
        assert_eq!(
            descriptor(SkillId(34), SkillPhase::Success).map(|descriptor| descriptor.skill),
            Some(34)
        );
        assert!(descriptor(SkillId(34), SkillPhase::Hit).is_none());
        assert!(descriptor(SkillId(65_500), SkillPhase::Success).is_none());
        assert!(skill_has_effect(SkillId(89)), "Storm Gust");
    }

    #[test]
    fn status_aura_subsystem_stays_available() {
        // The Assumptio persistent glow is status-driven and not tied to skill
        // packets; the pipeline keeps it separable but reachable.
        assert!(crate::world::status_aura_spec(110).is_some());
        assert!(crate::world::status_aura_spec(473).is_some());
        assert!(crate::world::status_aura_spec(65_500).is_none());
    }

    /// `MO_FINGEROFFENSIVE` / Throw Spirit Sphere (267) launches a single
    /// official effect: `EF_TANJI` (265), whose body draws
    /// `effect\blue_ivy.bmp` and plays `effect\mon_탄지신통.wav`
    /// (`JobInheritance.cpp:2224-2230`, `RagEffect.cpp:14501-14542`).
    ///
    /// The 2019 roBrowser table additionally carried `hitEffectId 1`
    /// (`EF_HIT1`, the generic `lens1`/`lens2` + `ef_hit2.wav` flash); the
    /// official effect list has no such id for 267
    /// (`comparison.json`: `extra_ids_in_robrowser: [1]`). The empty `Hit`
    /// override drops it, so the impact falls back to the skill's own wave.
    #[test]
    fn finger_offensive_uses_only_the_official_tanji_effect() {
        let success = skil_effect_layers(267, SkillPhase::Success);
        assert_eq!(success.len(), 1, "one EF_TANJI layer");
        assert_eq!(success[0].file, Some("effect\\blue_ivy.bmp"));
        assert!(descriptor(SkillId(267), SkillPhase::Hit).is_none(), "no EF_HIT1 row");
        assert!(skil_effect_layers(267, SkillPhase::Hit).is_empty());
        // The distinctive TANJI wave stays on the success layer.
        assert_eq!(skill_effect_sound_files(SkillId(267), SkillPhase::Success), &[
            "effect\\mon_탄지신통.wav"
        ]);
    }
}
