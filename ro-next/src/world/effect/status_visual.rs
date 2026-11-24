//! Runtime effects for the data-driven status-visual table
//! (`archive/data/client/status_visual_effects.ron`): a looping `.str` effect
//! and a world-anchored ground ring.
//!
//! Both are owned by one `(entity, status)` pair and share the same `active`
//! flag with the world reconciliation in `Client::update_status_visuals`. When
//! the status ends (or the entity leaves the map) the flag is cleared and the
//! effect removes itself on its next update, matching the status-aura
//! lifecycle.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use super::{EffectBase, EffectCenter, EffectWithLight, render_world_quad_blended};
use crate::graphics::{Color, ScreenSize, Texture};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// Lift of the ground ring above the terrain to avoid z-fighting.
const GROUND_LIFT: f32 = 1.0;

/// Wraps a repeating `.str` effect and removes it when the status ends.
pub struct StatusLoopingEffect {
    inner: EffectWithLight,
    active: Arc<AtomicBool>,
    gets_deleted: bool,
}

impl StatusLoopingEffect {
    pub fn new(inner: EffectWithLight, active: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            active,
            gets_deleted: false,
        }
    }
}

impl EffectBase for StatusLoopingEffect {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if !self.active.load(Ordering::Relaxed) {
            self.gets_deleted = true;
            self.inner.mark_for_deletion();
        }
        let alive = self.inner.update(entities, delta_time);
        !self.gets_deleted && alive
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
        self.inner.mark_for_deletion();
    }

    fn register_point_lights(&self, point_light_manager: &mut crate::world::PointLightManager, camera: &dyn Camera) {
        self.inner.register_point_lights(point_light_manager, camera);
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        self.inner.render(renderer, camera);
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        self.inner.culling_position()
    }
}

/// A flat, pulsing ring drawn below an entity while a status is active.
pub struct StatusGroundRing {
    texture: Arc<Texture>,
    center: EffectCenter,
    active: Arc<AtomicBool>,
    radius: f32,
    height: f32,
    color: Color,
    pulse: f32,
    time: f32,
    gets_deleted: bool,
}

impl StatusGroundRing {
    #[allow(clippy::too_many_arguments)] // one argument per ring geometry parameter
    pub fn new(
        texture_loader: &crate::loaders::TextureLoader,
        entity_id: EntityId,
        texture_path: &str,
        active: Arc<AtomicBool>,
        radius: f32,
        height: f32,
        color: Color,
        pulse: f32,
    ) -> Option<Self> {
        let texture = texture_loader.get_or_load(texture_path, crate::loaders::ImageType::Color).ok()?;
        Some(Self {
            texture,
            center: EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
            active,
            radius,
            height,
            color,
            pulse,
            time: 0.0,
            gets_deleted: false,
        })
    }

    fn screen_size(renderer: &EffectRenderer) -> ScreenSize {
        ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        }
    }
}

impl EffectBase for StatusGroundRing {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if !self.active.load(Ordering::Relaxed) {
            self.gets_deleted = true;
        }
        self.center.follow_entity(entities);
        self.time += delta_time;
        !self.gets_deleted
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let anchor = self.center.to_position() + Vector3::new(0.0, self.height + GROUND_LIFT, 0.0);
        let pulse = if self.pulse > 0.0 {
            1.0 + self.pulse * (self.time * 2.0).sin()
        } else {
            1.0
        };
        let radius = self.radius * pulse;
        let color = Color::rgba(self.color.red, self.color.green, self.color.blue, self.color.alpha);
        let view_direction = camera.view_direction();
        let right = camera.look_up_vector().cross(view_direction).normalize() * radius;
        let up = view_direction
            .cross(camera.look_up_vector().cross(view_direction).normalize())
            .normalize()
            * radius;
        let corners = [anchor - right - up, anchor + right - up, anchor - right + up, anchor + right + up];
        render_world_quad_blended(
            renderer,
            camera,
            anchor,
            Self::screen_size(renderer),
            self.texture.clone(),
            color,
            corners,
            [
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 1.0),
            ],
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position())
    }
}
