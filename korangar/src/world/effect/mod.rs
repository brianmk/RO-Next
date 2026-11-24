use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use korangar_collision::{Frustum, Sphere};
use korangar_container::Cacheable;
use ragnarok_formats::map::EffectSource;
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize, Texture};
use crate::renderer::EffectRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::{Camera, PointLightId, PointLightManager};

mod arrow_effect;
mod aura;
mod cast_circle;
mod procedural;
mod skill_aura;
pub(crate) mod spirit_spheres;
mod sprite_effect;
mod status_aura;
mod status_visual;
mod storm_cloud;
mod teleport_vanish;
mod warp;
mod weather;
pub(crate) use self::arrow_effect::ArrowEffect;
pub use self::aura::*;
pub use self::cast_circle::*;
pub use self::procedural::*;
pub use self::skill_aura::*;
pub(crate) use self::spirit_spheres as spirit_sphere_renderer;
pub(crate) use self::sprite_effect::{SpriteEffect, SpriteEffectParams};
pub use self::status_aura::StatusAura;
pub use self::status_visual::{StatusGroundRing, StatusLoopingEffect};
pub use self::storm_cloud::StormCloud;
pub use self::teleport_vanish::TeleportVanish;
pub use self::warp::WarpPortal;
pub use self::weather::Weather;

pub trait EffectBase {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool;

    fn mark_for_deletion(&mut self);

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera);

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera);

    /// World position used for distance culling, `None` disables culling for
    /// this effect (it is always rendered).
    fn culling_position(&self) -> Option<Point3<f32>> {
        None
    }
}

pub trait EffectSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool);
}

impl EffectSourceExt for EffectSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool) {
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}

pub struct Effect {
    frames_per_second: usize,
    max_key: usize,
    layers: Vec<Layer>,
}

impl Effect {
    pub fn new(frames_per_second: usize, max_key: usize, layers: Vec<Layer>) -> Self {
        Self {
            frames_per_second,
            max_key,
            layers,
        }
    }
}

impl Effect {
    pub fn new_frame_timer(&self) -> FrameTimer {
        FrameTimer {
            total_timer: 0.0,
            frames_per_second: self.frames_per_second,
            max_key: self.max_key,
            current_frame: 0,
        }
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, frame_timer: &FrameTimer, position: Point3<f32>, scale: f32) {
        for layer in &self.layers {
            let Some(frame) = layer.interpolate_frame(frame_timer) else {
                continue;
            };

            if frame.texture_index > layer.textures.len() {
                continue;
            }

            renderer.render_effect(
                camera,
                position,
                layer.textures[frame.texture_index].clone(),
                [
                    Vector2::new(frame.xy[0], frame.xy[4]) * scale,
                    Vector2::new(frame.xy[1], frame.xy[5]) * scale,
                    Vector2::new(frame.xy[3], frame.xy[7]) * scale,
                    Vector2::new(frame.xy[2], frame.xy[6]) * scale,
                ],
                [
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[3] + frame.uv[1]),
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[3] + frame.uv[1]),
                ],
                frame.offset,
                frame.angle,
                frame.color,
                frame.source_blend_factor,
                frame.destination_blend_factor,
            );
        }
    }
}

impl Cacheable for Effect {
    fn size(&self) -> usize {
        // We cache effects only by count.
        0
    }
}

pub struct Layer {
    textures: Vec<Arc<Texture>>,
    indices: Vec<Option<usize>>,
    frames: Vec<Frame>,
}

impl Layer {
    pub fn new(textures: Vec<Arc<Texture>>, indices: Vec<Option<usize>>, frames: Vec<Frame>) -> Self {
        Self { textures, indices, frames }
    }
}

impl Layer {
    fn interpolate_frame(&self, frame_timer: &FrameTimer) -> Option<Frame> {
        if let Some(frame_index) = self.indices[frame_timer.current_frame] {
            if let Some(next_frame) = self.frames.get(frame_index + 2) {
                Some(Self::interpolate(
                    &self.frames[frame_index],
                    next_frame,
                    frame_timer.current_frame,
                ))
            } else {
                Some(self.frames[frame_index].clone())
            }
        } else {
            None
        }
    }

    fn interpolate(first: &Frame, second: &Frame, frame_index: usize) -> Frame {
        let time = 1.0 / (second.frame_index - first.frame_index) as f32 * (frame_index - first.frame_index) as f32;
        let sub_mult = 1.0;

        // TODO: angle bias
        let angle = Self::ease_interpolate(first.angle.0, second.angle.0, time, 0.0, sub_mult);
        let color = (second.color - first.color) * time + first.color * sub_mult;

        let uv = (0..8)
            .map(|index| (second.uv[index] - first.uv[index]) * time + first.uv[index] * sub_mult)
            .next_chunk()
            .unwrap();

        // TODO: scale bias
        let xy = (0..8)
            .map(|index| Self::ease_interpolate(first.xy[index], second.xy[index], time, 0.0, sub_mult))
            .next_chunk()
            .unwrap();

        // TODO: additional logic for animation type 2 and 3
        let texture_index = first.texture_index;

        // TODO: bezier curves
        let offset_x = (second.offset.x - first.offset.x) * time + first.offset.x * sub_mult;
        let offset_y = (second.offset.y - first.offset.y) * time + first.offset.y * sub_mult;

        Frame {
            frame_index,
            frame_type: first.frame_type,
            offset: Vector2::new(offset_x, offset_y),
            uv,
            xy,
            texture_index,
            animation_type: first.animation_type,
            delay: first.delay,
            angle: Rad(angle),
            color,
            source_blend_factor: first.source_blend_factor,
            destination_blend_factor: first.destination_blend_factor,
            mt_present: first.mt_present,
        }
    }

    fn ease_interpolate(start_value: f32, end_value: f32, time: f32, bias: f32, sub_multiplier: f32) -> f32 {
        if bias > 0.0 {
            (end_value - start_value) * time.powf(1.0 + bias / 5.0) + start_value * sub_multiplier
        } else if bias < 0.0 {
            (end_value - start_value) * (1.0 - (1.0 - time).powf(-bias / 5.0 + 1.0)) + start_value * sub_multiplier
        } else {
            (end_value - start_value) * time + start_value * sub_multiplier
        }
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    frame_index: usize,
    frame_type: FrameType,
    offset: Vector2<f32>,
    uv: [f32; 8],
    xy: [f32; 8],
    texture_index: usize,
    animation_type: AnimationType,
    delay: f32,
    angle: Rad<f32>,
    color: Color,
    source_blend_factor: BlendFactor,
    destination_blend_factor: BlendFactor,
    mt_present: MultiTexturePresent,
}

impl Frame {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_index: usize,
        frame_type: FrameType,
        offset: Vector2<f32>,
        uv: [f32; 8],
        xy: [f32; 8],
        texture_index: usize,
        animation_type: AnimationType,
        delay: f32,
        angle: Rad<f32>,
        color: Color,
        source_blend_factor: BlendFactor,
        destination_blend_factor: BlendFactor,
        mt_present: MultiTexturePresent,
    ) -> Self {
        Self {
            frame_index,
            frame_type,
            offset,
            uv,
            xy,
            texture_index,
            animation_type,
            delay,
            angle,
            color,
            source_blend_factor,
            destination_blend_factor,
            mt_present,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationType {
    Type0,
    Type1,
    Type2,
    Type3,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameType {
    Basic,
    Morphing,
}

#[derive(Debug, Clone, Copy)]
pub enum MultiTexturePresent {
    None,
}

pub struct FrameTimer {
    total_timer: f32,
    frames_per_second: usize,
    max_key: usize,
    current_frame: usize,
}

impl FrameTimer {
    pub fn update(&mut self, delta_time: f32) -> bool {
        self.total_timer += delta_time;
        self.current_frame = (self.total_timer / (1.0 / self.frames_per_second as f32)) as usize;

        if self.current_frame >= self.max_key {
            // TODO: better wrapping
            self.total_timer = 0.0;
            self.current_frame = 0;
            return false;
        }

        true
    }
}

#[derive(Clone)]
pub enum EffectCenter {
    Entity(EntityId, Point3<f32>),
    Position(Point3<f32>),
}

impl EffectCenter {
    pub fn to_position(&self) -> Point3<f32> {
        match self {
            EffectCenter::Entity(_, position) | EffectCenter::Position(position) => *position,
        }
    }

    /// Follows the entity with the given id, if this center belongs to one.
    pub fn follow_entity(&mut self, entities: &[crate::world::Entity]) {
        if let EffectCenter::Entity(entity_id, position) = self
            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == *entity_id)
        {
            *position = entity.get_position();
        }
    }
}

pub struct EffectWithLight {
    effect: Arc<Effect>,
    frame_timer: FrameTimer,
    center: EffectCenter,
    effect_offset: Vector3<f32>,
    /// Scale of the effect quad. `.str` effects are rendered as screen space
    /// sprites sized in pixels, so this value is a plain multiplier of the
    /// size defined in the file (1.0 = original size).
    scale: f32,
    point_light_id: PointLightId,
    light_offset: Vector3<f32>,
    light_color: Color,
    light_intensity: f32,
    repeating: bool,
    current_light_intensity: f32,
    gets_deleted: bool,
}

impl EffectWithLight {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        effect: Arc<Effect>,
        frame_timer: FrameTimer,
        center: EffectCenter,
        effect_offset: Vector3<f32>,
        scale: f32,
        point_light_id: PointLightId,
        light_offset: Vector3<f32>,
        light_color: Color,
        light_intensity: f32,
        repeating: bool,
    ) -> Self {
        Self {
            effect,
            frame_timer,
            center,
            effect_offset,
            scale,
            point_light_id,
            light_offset,
            light_color,
            light_intensity,
            repeating,
            current_light_intensity: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for EffectWithLight {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position() + self.effect_offset)
    }

    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        const FADE_SPEED: f32 = 5.0;

        if let EffectCenter::Entity(entity_id, position) = &mut self.center
            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == *entity_id)
        {
            let new_position = entity.get_position();
            *position = new_position;
        }

        if !self.gets_deleted && !self.frame_timer.update(delta_time) && !self.repeating {
            self.gets_deleted = true;
        }

        let (target, clamping_function): (f32, fn(f32, f32) -> f32) = match self.gets_deleted {
            true => (0.0, f32::max),
            false => (self.light_intensity, f32::min),
        };

        self.current_light_intensity += (target - self.current_light_intensity) * FADE_SPEED * delta_time;
        self.current_light_intensity = clamping_function(self.current_light_intensity, target);

        !self.gets_deleted || self.current_light_intensity > 0.1
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let frustum = Frustum::new(camera.view_projection_matrix(), true);

        let light_position = self.center.to_position() + self.light_offset;

        if frustum.intersects_sphere(&Sphere::new(light_position, self.current_light_intensity)) {
            point_light_manager.register_fading(
                self.point_light_id,
                light_position,
                self.light_color,
                self.current_light_intensity,
                self.light_intensity,
            )
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if !self.gets_deleted {
            self.effect.render(
                renderer,
                camera,
                &self.frame_timer,
                self.center.to_position() + self.effect_offset,
                self.scale,
            );
        }
    }
}

/// The origin that the original client uses for hard-coded effects. Local
/// effect coordinates are expressed relative to the feet of the effect anchor
/// on a 640x480 screen, which is exactly the space that [`EffectRenderer`]
/// expects.
pub(crate) const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

/// Projects a world position onto the screen, returning `None` if the position
/// is behind the camera. The result is in physical window pixels with the
/// origin in the top left corner, which is the space that
/// [`EffectRenderer`] uses.
pub(crate) fn project_point(camera: &dyn Camera, position: Point3<f32>, screen_size: ScreenSize) -> Option<(Vector2<f32>, f32)> {
    let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();

    if clip_space_position.w <= 0.0 {
        return None;
    }

    let normalized_x = clip_space_position.x / clip_space_position.w;
    let normalized_y = clip_space_position.y / clip_space_position.w;
    let depth = clip_space_position.z / clip_space_position.w;

    Some((
        Vector2::new(
            (normalized_x * 0.5 + 0.5) * screen_size.width,
            (0.5 - normalized_y * 0.5) * screen_size.height,
        ),
        depth,
    ))
}

/// Renders a single textured quad. The corners are given in world space with
/// the matching texture coordinates. They are projected and re-ordered so that
/// the quad is never rendered flipped, no matter how the camera is rotated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_world_quad(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    anchor: Point3<f32>,
    screen_size: ScreenSize,
    texture: Arc<Texture>,
    color: Color,
    corners: [Point3<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
) {
    render_world_quad_blended(
        renderer,
        camera,
        anchor,
        screen_size,
        texture,
        color,
        corners,
        texture_coordinates,
        BlendFactor::One,
        BlendFactor::One,
    );
}

/// Renders a textured quad with explicit blend factors.
///
/// The factors use the straight alpha convention of the official client effect
/// data (`.str` frames and the hard-coded primitives). Effect textures and
/// colors are premultiplied, so the forward effect pass translates a
/// `SrcAlpha` source factor to `One` when it builds the pipeline; passing
/// `SrcAlpha` here is therefore correct and does not darken the quad.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_world_quad_blended(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    anchor: Point3<f32>,
    screen_size: ScreenSize,
    texture: Arc<Texture>,
    color: Color,
    corners: [Point3<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
    source_blend_factor: BlendFactor,
    destination_blend_factor: BlendFactor,
) {
    let Some((anchor_screen, _)) = project_point(camera, anchor, screen_size) else {
        return;
    };

    let mut screen_corners = [Vector2::new(0.0, 0.0); 4];
    let mut corner_depths = [0.0; 4];

    for (index, corner) in corners.iter().enumerate() {
        let Some((projected, depth)) = project_point(camera, *corner, screen_size) else {
            return;
        };

        screen_corners[index] = projected - anchor_screen;
        corner_depths[index] = depth;
    }

    let (screen_corners, texture_coordinates, corner_depths) = order_quad(screen_corners, texture_coordinates, corner_depths);

    renderer.render_effect_with_depth(
        camera,
        anchor,
        texture,
        screen_corners.map(|corner| EFFECT_ORIGIN + corner),
        corner_depths,
        [
            texture_coordinates[3],
            texture_coordinates[1],
            texture_coordinates[0],
            texture_coordinates[2],
        ],
        Vector2::new(0.0, 0.0),
        Rad(0.0),
        color,
        source_blend_factor,
        destination_blend_factor,
    );
}

/// Re-orders the corners of a screen space quad (together with their texture
/// coordinates) into top left, top right, bottom left, bottom right. This makes
/// sure that a quad that is seen from "behind" doesn't render mirrored or
/// degenerate, which would happen if the projected corners were handed to the
/// renderer in an order that is no longer a valid polygon.
pub(crate) fn order_quad(
    corners: [Vector2<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
    depths: [f32; 4],
) -> ([Vector2<f32>; 4], [Vector2<f32>; 4], [f32; 4]) {
    let mut indices = [0, 1, 2, 3];
    indices.sort_by(|&left, &right| corners[left].y.total_cmp(&corners[right].y));

    let mut top = [indices[0], indices[1]];
    top.sort_by(|&left, &right| corners[left].x.total_cmp(&corners[right].x));

    let mut bottom = [indices[2], indices[3]];
    bottom.sort_by(|&left, &right| corners[left].x.total_cmp(&corners[right].x));

    let order = [top[0], top[1], bottom[0], bottom[1]];

    (
        order.map(|index| corners[index]),
        order.map(|index| texture_coordinates[index]),
        order.map(|index| depths[index]),
    )
}

/// Default upper bound for simultaneously alive world effects.
///
/// A crowded population-engine town with several skill effects per character
/// used to grow this list without bound; every entry is a boxed object that is
/// updated and submitted every frame. The budget keeps the frame time bounded
/// while still allowing far more effects than are visible at once.
const DEFAULT_MAXIMUM_EFFECTS: usize = 200;
/// Default upper bound for effects attached to a single entity (skill units,
/// auras, cast circles). Independent from the global budget so one entity
/// cannot use up the whole effect budget.
const DEFAULT_MAXIMUM_EFFECTS_PER_ENTITY: usize = 24;
/// Default distance (in world units, one map cell is 5 units) beyond which
/// effects are no longer rendered. `0.0` disables the distance culling.
const DEFAULT_EFFECT_CULL_DISTANCE: f32 = 1800.0;

/// Environment knob for [`EffectHolder::maximum_effects`]; `0` disables the
/// cap. Documented in `scripts/perf/README.md` and
/// `notes/korangar-p0-performance-report.md`.
const MAXIMUM_EFFECTS_ENVIRONMENT_VARIABLE: &str = "KORANGAR_MAX_EFFECTS";
/// Environment knob for [`EffectHolder::maximum_effects_per_entity`]; `0`
/// disables the per-entity cap.
const MAXIMUM_EFFECTS_PER_ENTITY_ENVIRONMENT_VARIABLE: &str = "KORANGAR_MAX_EFFECTS_PER_ENTITY";
/// Environment knob for [`EffectHolder::cull_distance`]; `0` disables culling.
const EFFECT_CULL_DISTANCE_ENVIRONMENT_VARIABLE: &str = "KORANGAR_EFFECT_CULL_DISTANCE";

const DROP_LOG_INTERVAL: std::time::Duration = std::time::Duration::from_secs(5);

fn environment_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

fn environment_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0)
        .unwrap_or(default)
}

/// Whether a new effect may be spawned with the given current counts.
///
/// Pure function so the budget policy can be unit tested without any client
/// state.
fn effect_budget_allows(current_total: usize, current_for_entity: usize, maximum_total: usize, maximum_per_entity: usize) -> bool {
    if maximum_total > 0 && current_total >= maximum_total {
        return false;
    }

    if maximum_per_entity > 0 && current_for_entity >= maximum_per_entity {
        return false;
    }

    true
}

pub struct EffectHolder {
    effects: Vec<(Box<dyn EffectBase + Send + Sync>, Option<EntityId>)>,
    /// Maximum number of simultaneously alive effects (`0` = unlimited).
    maximum_effects: usize,
    /// Maximum number of effects attached to a single entity (`0` = unlimited).
    maximum_effects_per_entity: usize,
    /// Distance in world units beyond which effects are culled (`0` =
    /// disabled).
    cull_distance: f32,
    /// Effects dropped by the budget since startup (telemetry/logging).
    dropped_effects: u64,
    last_drop_log: std::cell::Cell<Option<std::time::Instant>>,
    /// Number of effects rendered / culled by distance in the last frame.
    rendered_in_last_frame: std::cell::Cell<u32>,
    culled_in_last_frame: std::cell::Cell<u32>,
}

impl Default for EffectHolder {
    fn default() -> Self {
        Self {
            effects: Vec::new(),
            maximum_effects: environment_usize(MAXIMUM_EFFECTS_ENVIRONMENT_VARIABLE, DEFAULT_MAXIMUM_EFFECTS),
            maximum_effects_per_entity: environment_usize(
                MAXIMUM_EFFECTS_PER_ENTITY_ENVIRONMENT_VARIABLE,
                DEFAULT_MAXIMUM_EFFECTS_PER_ENTITY,
            ),
            cull_distance: environment_f32(EFFECT_CULL_DISTANCE_ENVIRONMENT_VARIABLE, DEFAULT_EFFECT_CULL_DISTANCE),
            dropped_effects: 0,
            last_drop_log: std::cell::Cell::new(None),
            rendered_in_last_frame: std::cell::Cell::new(0),
            culled_in_last_frame: std::cell::Cell::new(0),
        }
    }
}

impl EffectHolder {
    /// Number of active effects (used by the performance instrumentation).
    pub fn count(&self) -> usize {
        self.effects.len()
    }

    /// Rendered and distance-culled effect counts of the last frame.
    pub fn render_counts(&self) -> (u32, u32) {
        (self.rendered_in_last_frame.get(), self.culled_in_last_frame.get())
    }

    /// Number of effects dropped by the budget since startup.
    pub fn dropped_effects(&self) -> u64 {
        self.dropped_effects
    }

    /// Whether another effect owned by `entity_id` may be spawned.
    fn budget_allows(&self, entity_id: Option<EntityId>) -> bool {
        let current_for_entity = entity_id
            .map(|entity_id| {
                self.effects
                    .iter()
                    .filter(|(_, effect_entity_id)| *effect_entity_id == Some(entity_id))
                    .count()
            })
            .unwrap_or(0);

        effect_budget_allows(
            self.effects.len(),
            current_for_entity,
            self.maximum_effects,
            self.maximum_effects_per_entity,
        )
    }

    fn log_dropped_effect(&self) {
        // Rate limit the log line; a crowded town can drop many effects per
        // second and logging every one would itself cost frame time.
        let now = std::time::Instant::now();
        let should_log = match self.last_drop_log.get() {
            Some(last_log) => now.duration_since(last_log) >= DROP_LOG_INTERVAL,
            None => true,
        };

        if should_log {
            self.last_drop_log.set(Some(now));
            crate::logging::log_debug!(
                "effects: budget reached ({} alive, max {} global / {} per entity), dropped {} effects so far",
                self.effects.len(),
                self.maximum_effects,
                self.maximum_effects_per_entity,
                self.dropped_effects,
            );
        }
    }

    /// Adds an effect that is not attached to an entity.
    ///
    /// Returns `false` if the effect budget rejected the effect.
    pub fn add_effect(&mut self, effect: Box<dyn EffectBase + Send + Sync>) -> bool {
        self.add_effect_with_owner(effect, None)
    }

    /// Adds an effect that is owned by an entity (and follows/cleans up with
    /// it).
    ///
    /// Returns `false` if the effect budget rejected the effect; callers that
    /// keep their own bookkeeping (for example the status auras) should then
    /// retry later instead of recording the effect as spawned.
    pub fn add_unit(&mut self, effect: Box<dyn EffectBase + Send + Sync>, entity_id: EntityId) -> bool {
        self.add_effect_with_owner(effect, Some(entity_id))
    }

    fn add_effect_with_owner(&mut self, effect: Box<dyn EffectBase + Send + Sync>, entity_id: Option<EntityId>) -> bool {
        if !self.budget_allows(entity_id) {
            self.dropped_effects += 1;
            self.log_dropped_effect();
            return false;
        }

        self.effects.push((effect, entity_id));
        true
    }

    pub fn remove_unit(&mut self, removed_entity_id: EntityId) {
        self.effects
            .iter_mut()
            .filter(|(_, entity_id)| entity_id.is_some_and(|entity_id| entity_id == removed_entity_id))
            .for_each(|(effect, _)| effect.mark_for_deletion());
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }

    /// Whether the effect is beyond the culling distance of the camera.
    fn is_culled(&self, effect: &(dyn EffectBase + Send + Sync), camera: &dyn Camera) -> bool {
        if self.cull_distance <= 0.0 {
            return false;
        }

        effect
            .culling_position()
            .is_some_and(|position| camera.distance_to(position) > self.cull_distance)
    }

    pub fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) {
        self.effects.retain_mut(|(effect, _)| effect.update(entities, delta_time));
    }

    pub fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        self.effects.iter().for_each(|(effect, _)| {
            // Do not register lights for effects that are too far away to be
            // rendered; this bounds the point light set in crowded towns.
            if self.is_culled(effect.as_ref(), camera) {
                return;
            }

            effect.register_point_lights(point_light_manager, camera);
        });
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let mut rendered = 0;
        let mut culled = 0;

        for (effect, _) in &self.effects {
            if self.is_culled(effect.as_ref(), camera) {
                culled += 1;
                continue;
            }

            effect.render(renderer, camera);
            rendered += 1;
        }

        self.rendered_in_last_frame.set(rendered);
        self.culled_in_last_frame.set(culled);
    }
}

#[cfg(test)]
mod budget_tests {
    use super::effect_budget_allows;

    #[test]
    fn global_budget_blocks_new_effects() {
        assert!(effect_budget_allows(0, 0, 2, 0));
        assert!(effect_budget_allows(1, 0, 2, 0));
        assert!(!effect_budget_allows(2, 0, 2, 0));
        assert!(!effect_budget_allows(3, 0, 2, 0));
    }

    #[test]
    fn zero_budget_means_unlimited() {
        assert!(effect_budget_allows(1_000_000, 1_000_000, 0, 0));
        assert!(effect_budget_allows(0, 0, 0, 0));
    }

    #[test]
    fn per_entity_budget_is_independent_of_global_budget() {
        // Global has room, the entity does not.
        assert!(!effect_budget_allows(1, 2, 10, 2));
        // Entity has room, the global budget does not.
        assert!(!effect_budget_allows(10, 0, 10, 2));
        // Both have room.
        assert!(effect_budget_allows(9, 1, 10, 2));
    }
}
