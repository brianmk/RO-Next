use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use ragnarok_formats::map::EffectSource;
use ragnarok_packets::{ClientTick, EntityId};
use ro_next_collision::{Frustum, Sphere};
use ro_next_container::Cacheable;
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
#[allow(dead_code)]
pub(crate) mod port;
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

/// Identity of a visual that lives exactly as long as one active status of one
/// entity.
///
/// The official client keeps the effect of a status in the actor's effect list
/// and deletes it by effect id when the status ends or is re-applied
/// (`CGameActor::SendMsg`, `case AM_STATUS_CHANGE:`,
/// `GameObject.cpp:2686-2700`: `while (HasId(EF_...)) { delete; }` then
/// `LaunchEffect`). The port has no per-effect ids at the holder level, so a
/// status-launched visual carries this key instead and
/// [`EffectHolder::remove_status_visual`] retires it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusVisualKey {
    pub entity_id: EntityId,
    pub status_index: u16,
}

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

    /// Applies the effect's per-frame caster-actor requests to the entity that
    /// owns it (`EffectHolder::add_unit`).
    ///
    /// The official effect bodies mutate the shared caster actor from inside
    /// `OnProcess` (`SetColor`, `render.state`, `SetFrameSequence`). The port's
    /// `update` is handed an immutable entity slice, so those requests are
    /// recorded on the effect's runtime and replayed here by the holder, which
    /// *does* have a mutable handle. The default is a no-op: only effects with
    /// such requests (currently [`port::visual::PortedVisual`]) implement it.
    fn apply_caster_state(&mut self, _entity: &mut crate::world::Entity, _client_tick: ClientTick) {}

    /// Undoes [`Self::apply_caster_state`] when the effect is retired.
    ///
    /// Called by [`EffectHolder::update`] before the effect object is dropped;
    /// implementations clear only the state they actually applied.
    fn clear_caster_state(&mut self, _entity: &mut crate::world::Entity) {}

    /// `Some` when this effect is a status-launched visual, i.e. it lives
    /// exactly as long as the status that launched it.
    ///
    /// The status handler removes the previous instance before every
    /// (re-)application and on the status end, mirroring the official
    /// delete-then-launch of `AM_STATUS_CHANGE`; the default has no key, so
    /// every other effect is untouched by that removal.
    fn status_visual_key(&self) -> Option<StatusVisualKey> {
        None
    }

    /// Child effects this effect launched since the last call.
    ///
    /// The official launchers (`CEffectPrim::RenderSMA3`,
    /// `EffectPrimitiveRendering.cpp:2394-2415`) call
    /// `EffectActor::LaunchEffect` from inside `Render`; the call appends the
    /// child to the scene object list and the actor's effect list. The port
    /// cannot mutate those lists from `EffectBase::render` (`&self`) or from
    /// `update` (immutable entity slice), so the ported visual records the
    /// requests on its runtime outbox, turns them into child visuals after the
    /// step and hands them over here. [`EffectHolder::update`] appends them,
    /// so they step from the next frame on, exactly like a child created
    /// during the previous frame's render.
    ///
    /// The default returns nothing; only [`port::visual::PortedVisual`]
    /// launches children.
    fn take_spawned_effects(&mut self) -> Vec<Box<dyn EffectBase + Send + Sync>> {
        Vec::new()
    }
    /// Screen-shake requests (`MM_QUAKE`) recorded since the last call.
    ///
    /// Ported effect bodies count their quakes on the runtime; the holder
    /// drains them here so the camera can apply the shake for the frame. The
    /// default has no shake.
    fn take_shakes(&mut self) -> u32 {
        0
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
/// cap. Documented in `scripts/perf/README.md`.
const MAXIMUM_EFFECTS_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_MAX_EFFECTS";
/// Environment knob for [`EffectHolder::maximum_effects_per_entity`]; `0`
/// disables the per-entity cap.
const MAXIMUM_EFFECTS_PER_ENTITY_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_MAX_EFFECTS_PER_ENTITY";
/// Environment knob for [`EffectHolder::cull_distance`]; `0` disables culling.
const EFFECT_CULL_DISTANCE_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_EFFECT_CULL_DISTANCE";

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
    /// `MM_QUAKE` requests drained from the effects since the last frame.
    pending_shakes: u32,
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
            pending_shakes: 0,
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
    /// Drain the `MM_QUAKE` requests recorded by the effects this frame.
    pub fn take_shakes(&mut self) -> u32 {
        std::mem::take(&mut self.pending_shakes)
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

    /// Retires the visual launched by one status of one entity.
    ///
    /// Mirrors the official `AM_STATUS_CHANGE` cleanup
    /// (`GameObject.cpp:2686-2688`): the previous instance of a status effect
    /// is deleted before the status launches it again, and on the status end.
    /// Only effects that carry the matching [`StatusVisualKey`] are marked, so
    /// a status can never remove another actor's or another status's effect.
    pub fn remove_status_visual(&mut self, entity_id: EntityId, status_index: u16) {
        let key = StatusVisualKey { entity_id, status_index };

        self.effects
            .iter_mut()
            .filter(|(effect, _)| effect.status_visual_key() == Some(key))
            .for_each(|(effect, _)| effect.mark_for_deletion());
    }

    /// Whether a live status-launched visual with this key is in the list.
    ///
    /// A visual marked for deletion is still in the list until its next step
    /// retires it, so this is only used to decide whether a guarded official
    /// launch (`if (!HasId(EF_...))`, `GameObject.cpp:2648`) has to spawn at
    /// all.
    pub fn has_status_visual(&self, entity_id: EntityId, status_index: u16) -> bool {
        let key = StatusVisualKey { entity_id, status_index };

        self.effects.iter().any(|(effect, _)| effect.status_visual_key() == Some(key))
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

    /// Steps every effect and applies the surviving effects' caster-actor
    /// requests to their owning entities.
    ///
    /// The order mirrors a frame of the official client: the effect bodies run
    /// first, an effect that returns `false` is retired (and clears the state
    /// it applied), then the surviving effects re-apply their per-frame caster
    /// requests. Retired effects are cleared before the surviving ones are
    /// applied, so a second effect on the same caster immediately restores its
    /// own tint/sequence.
    pub fn update(&mut self, entities: &mut [crate::world::Entity], delta_time: f32, client_tick: ClientTick) {
        let mut index = 0;

        while index < self.effects.len() {
            let (alive, shakes) = {
                let (effect, _) = &mut self.effects[index];
                let alive = effect.update(entities, delta_time);
                (alive, effect.take_shakes())
            };
            self.pending_shakes += shakes;

            if alive {
                index += 1;
                continue;
            }

            let (mut effect, owner) = self.effects.remove(index);

            if let Some(owner) = owner
                && let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == owner)
            {
                effect.clear_caster_state(entity);
            }
        }

        for (effect, owner) in &mut self.effects {
            let Some(owner) = *owner else {
                continue;
            };

            if let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == owner) {
                effect.apply_caster_state(entity, client_tick);
            }
        }

        // Collect the effects the renderer pass launched since the last step
        // (`RenderSMA3`'s `LaunchEffect` calls). Only the effects that were
        // already in the list are asked, so a child cannot spawn during its
        // own insertion pass; it starts stepping on the next frame like the
        // official child created during `Render`.
        //
        // The children are added unowned: the official call appends them to
        // the *local player's* effect list (and the scene list), not to the
        // caster, and the default per-entity budget (24) would truncate a
        // dense launcher stream such as `EF_STIN3`'s one-comet-per-frame
        // (about fifty alive at once). The global budget still bounds them.
        let parents = self.effects.len();

        for index in 0..parents {
            for child in self.effects[index].0.take_spawned_effects() {
                self.add_effect(child);
            }
        }
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

#[cfg(test)]
mod status_visual_tests {
    use ragnarok_packets::{ClientTick, EntityId};

    use super::{EffectBase, EffectHolder, StatusVisualKey};
    use crate::world::Camera;

    /// Minimal effect that records which key it was spawned with.
    struct KeyedVisual {
        key: Option<StatusVisualKey>,
        deleted: bool,
    }

    impl EffectBase for KeyedVisual {
        fn update(&mut self, _entities: &[crate::world::Entity], _delta_time: f32) -> bool {
            !self.deleted
        }

        fn mark_for_deletion(&mut self) {
            self.deleted = true;
        }

        fn register_point_lights(&self, _manager: &mut crate::world::PointLightManager, _camera: &dyn Camera) {}

        fn render(&self, _renderer: &mut crate::renderer::EffectRenderer, _camera: &dyn Camera) {}

        fn status_visual_key(&self) -> Option<StatusVisualKey> {
            self.key
        }
    }

    fn keyed(entity_id: EntityId, status_index: u16) -> KeyedVisual {
        KeyedVisual {
            key: Some(StatusVisualKey { entity_id, status_index }),
            deleted: false,
        }
    }

    fn effect_holder() -> EffectHolder {
        let mut holder = EffectHolder::default();
        // The ambient environment can cap the effect count; the tests only
        // exercise the key bookkeeping.
        holder.maximum_effects = 0;
        holder.maximum_effects_per_entity = 0;
        holder
    }

    #[test]
    fn removing_a_status_visual_matches_both_entity_and_status() {
        let entity = EntityId(7);
        let other_entity = EntityId(8);

        let mut holder = effect_holder();
        assert!(holder.add_unit(Box::new(keyed(entity, 87)), entity));
        assert!(holder.add_unit(Box::new(keyed(entity, 235)), entity));
        assert!(holder.add_unit(Box::new(keyed(other_entity, 87)), other_entity));
        assert!(holder.add_effect(Box::new(KeyedVisual { key: None, deleted: false })));

        holder.remove_status_visual(entity, 87);

        // The effect is still listed until its next step retires it, but the
        // other entity's identical status and the entity's other status are
        // untouched.
        assert_eq!(holder.count(), 4);
        assert!(holder.has_status_visual(entity, 87), "still listed until it steps");
        assert!(holder.has_status_visual(entity, 235));
        assert!(holder.has_status_visual(other_entity, 87));

        // An unrelated (entity, status) pair removes nothing new.
        holder.remove_status_visual(EntityId(9), 87);

        // Stepping retires the marked visual only: no entity slice is needed
        // for the key bookkeeping, so the other three survive.
        let mut entities: Vec<crate::world::Entity> = Vec::new();
        holder.update(&mut entities, 0.016, ClientTick(0));
        assert_eq!(holder.count(), 3);
        assert!(!holder.has_status_visual(entity, 87));
        assert!(holder.has_status_visual(entity, 235));
        assert!(holder.has_status_visual(other_entity, 87));
    }

    #[test]
    fn unkeyed_effects_are_never_removed_by_a_status() {
        let mut holder = effect_holder();
        assert!(holder.add_effect(Box::new(KeyedVisual { key: None, deleted: false })));

        holder.remove_status_visual(EntityId(1), 87);

        assert_eq!(holder.count(), 1);
        assert!(!holder.has_status_visual(EntityId(1), 87));
    }

    #[test]
    fn status_visual_key_is_addressable_by_value() {
        // The key is `Copy`/`Eq` so the spawn path can keep it without an
        // allocation and the removal path can rebuild it from the packet.
        let key = StatusVisualKey {
            entity_id: EntityId(42),
            status_index: 1132,
        };
        assert_eq!(key, StatusVisualKey {
            entity_id: EntityId(42),
            status_index: 1132,
        });
    }
}

#[cfg(test)]
mod spawned_effect_tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use ragnarok_packets::ClientTick;

    use super::{EffectBase, EffectHolder};
    use crate::world::Camera;

    /// Counts its `update` calls, so the test can tell when it first stepped.
    struct Counted {
        steps: Arc<AtomicU32>,
    }

    impl EffectBase for Counted {
        fn update(&mut self, _entities: &[crate::world::Entity], _delta_time: f32) -> bool {
            self.steps.fetch_add(1, Ordering::Relaxed);

            true
        }

        fn mark_for_deletion(&mut self) {}

        fn register_point_lights(&self, _manager: &mut crate::world::PointLightManager, _camera: &dyn Camera) {}

        fn render(&self, _renderer: &mut crate::renderer::EffectRenderer, _camera: &dyn Camera) {}
    }

    /// Stands in for a ported visual whose renderer launched one child.
    struct Spawner {
        child: Option<Box<dyn EffectBase + Send + Sync>>,
    }

    impl EffectBase for Spawner {
        fn update(&mut self, _entities: &[crate::world::Entity], _delta_time: f32) -> bool {
            true
        }

        fn mark_for_deletion(&mut self) {}

        fn register_point_lights(&self, _manager: &mut crate::world::PointLightManager, _camera: &dyn Camera) {}

        fn render(&self, _renderer: &mut crate::renderer::EffectRenderer, _camera: &dyn Camera) {}

        fn take_spawned_effects(&mut self) -> Vec<Box<dyn EffectBase + Send + Sync>> {
            self.child.take().into_iter().collect()
        }
    }

    #[test]
    fn renderer_launched_effects_join_the_list_and_start_on_the_next_frame() {
        let mut holder = EffectHolder::default();
        // The ambient environment can cap the effect count.
        holder.maximum_effects = 0;
        holder.maximum_effects_per_entity = 0;

        let steps = Arc::new(AtomicU32::new(0));
        holder.add_effect(Box::new(Spawner {
            child: Some(Box::new(Counted { steps: steps.clone() })),
        }));

        let mut entities: Vec<crate::world::Entity> = Vec::new();
        holder.update(&mut entities, 0.016, ClientTick(0));

        // The official `LaunchEffect` appends the child during `Render`, so it
        // is not stepped by the update that spawned it.
        assert_eq!(holder.count(), 2);
        assert_eq!(steps.load(Ordering::Relaxed), 0);

        holder.update(&mut entities, 0.016, ClientTick(0));

        assert_eq!(steps.load(Ordering::Relaxed), 1);
        assert_eq!(holder.count(), 2, "the child owns its own lifetime");
    }

    #[test]
    fn a_retiring_effect_spawns_nothing() {
        struct Retiring;

        impl EffectBase for Retiring {
            fn update(&mut self, _entities: &[crate::world::Entity], _delta_time: f32) -> bool {
                false
            }

            fn mark_for_deletion(&mut self) {}

            fn register_point_lights(&self, _manager: &mut crate::world::PointLightManager, _camera: &dyn Camera) {}

            fn render(&self, _renderer: &mut crate::renderer::EffectRenderer, _camera: &dyn Camera) {}
        }

        let mut holder = EffectHolder::default();
        holder.maximum_effects = 0;
        holder.maximum_effects_per_entity = 0;
        holder.add_effect(Box::new(Retiring));

        let mut entities: Vec<crate::world::Entity> = Vec::new();
        holder.update(&mut entities, 0.016, ClientTick(0));

        assert_eq!(holder.count(), 0);
    }
}
