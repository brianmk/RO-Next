//! Procedural Storm Gust cloud.
//!
//! The snow part of Storm Gust is the official `stormgust.str` effect (texture
//! `snow_a.bmp`). The dark storm cloud above the area is *not* a `.str`/`.spr`
//! asset of the 2019 client: the official client draws it as a hard-coded 3D
//! effect from the `cloud1.tga` .. `cloud7.tga` textures. Data files therefore
//! cannot supply it, so this module reproduces the cloud with a small set of
//! camera-facing, slowly swirling billboards for the duration of the skill.

use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, render_world_quad_blended};

/// Textures of the official storm cloud (data\texture\effect\cloud%d.tga).
const CLOUD_TEXTURES: [&str; 7] = [
    "effect\\cloud1.tga",
    "effect\\cloud2.tga",
    "effect\\cloud3.tga",
    "effect\\cloud4.tga",
    "effect\\cloud5.tga",
    "effect\\cloud6.tga",
    "effect\\cloud7.tga",
];

/// Lifetime of the cloud in milliseconds.
const DURATION_MS: f32 = 3_000.0;
/// Time to fade the whole cloud in / out.
const FADE_IN_MS: f32 = 250.0;
const FADE_OUT_MS: f32 = 700.0;
/// Number of cloud billboards.
const CLOUD_COUNT: usize = 18;
/// Base height of the cloud layer above the target cell in world units.
const BASE_HEIGHT: f32 = 10.0;
/// Radius of the cloud disc in world units.
const CLOUD_RADIUS: f32 = 22.0;
/// Alpha of a single cloud billboard.
const CLOUD_ALPHA: f32 = 0.55;

struct CloudPuff {
    texture: Arc<Texture>,
    /// Offset from the cloud center in world units (x, z) and height (y).
    offset: Vector3<f32>,
    /// Quad size in world units.
    size: f32,
    /// Rotation speed in radians per second.
    spin: f32,
    /// Initial rotation in radians.
    angle: f32,
}

/// A short-lived procedural storm cloud above a world position.
pub struct StormCloud {
    puffs: Vec<CloudPuff>,
    center: Point3<f32>,
    time: f32,
    gets_deleted: bool,
}

impl StormCloud {
    /// Loads the cloud textures; returns `None` when none of them is available.
    pub fn new(texture_loader: &TextureLoader, center: Point3<f32>) -> Option<Self> {
        let textures: Vec<Arc<Texture>> = CLOUD_TEXTURES
            .iter()
            .filter_map(|path| texture_loader.get_or_load(path, ImageType::Color).ok())
            .collect();

        if textures.is_empty() {
            return None;
        }

        let mut puffs = Vec::with_capacity(CLOUD_COUNT);

        for index in 0..CLOUD_COUNT {
            // Deterministic pseudo-random placement: golden-angle distribution
            // around the cell with a slight height variation.
            let angle = index as f32 * 2.399_963;
            let radius = CLOUD_RADIUS * (0.25 + 0.75 * ((index * 37 % 11) as f32 / 10.0));
            let height = BASE_HEIGHT + ((index * 53 % 9) as f32 / 8.0) * 8.0;

            puffs.push(CloudPuff {
                texture: textures[index % textures.len()].clone(),
                offset: Vector3::new(angle.cos() * radius, height, angle.sin() * radius),
                size: 16.0 + ((index * 29 % 7) as f32 / 6.0) * 16.0,
                spin: 0.15 + ((index * 17 % 5) as f32 / 5.0) * 0.35,
                angle,
            });
        }

        Some(Self {
            puffs,
            center,
            time: 0.0,
            gets_deleted: false,
        })
    }

    fn alpha(&self) -> f32 {
        let elapsed = self.time * 1000.0;
        let duration = DURATION_MS;

        let fade_in = (elapsed / FADE_IN_MS).clamp(0.0, 1.0);
        let fade_out = ((duration - elapsed) / FADE_OUT_MS).clamp(0.0, 1.0);

        CLOUD_ALPHA * fade_in.min(fade_out).clamp(0.0, 1.0)
    }
}

impl EffectBase for StormCloud {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center)
    }

    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.time += delta_time;
        !self.gets_deleted && self.time * 1000.0 < DURATION_MS
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let alpha = self.alpha();

        if alpha <= 0.001 {
            return;
        }

        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };
        let view_direction = camera.view_direction();
        let right = camera.look_up_vector().cross(view_direction).normalize();
        let up = view_direction.cross(right).normalize();
        let color = Color::rgba(0.45, 0.48, 0.58, alpha);

        for puff in &self.puffs {
            let puffs_position = self.center + puff.offset;
            let half_size = puff.size / 2.0;
            let angle = puff.angle + self.time * puff.spin;
            let rotated_right = (right * angle.cos() + up * angle.sin()) * half_size;
            let rotated_up = (up * angle.cos() - right * angle.sin()) * half_size;

            let corners = [
                puffs_position - rotated_right + rotated_up,
                puffs_position + rotated_right + rotated_up,
                puffs_position + rotated_right - rotated_up,
                puffs_position - rotated_right - rotated_up,
            ];
            let texture_coordinates = [
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 1.0),
            ];

            render_world_quad_blended(
                renderer,
                camera,
                puffs_position,
                screen_size,
                puff.texture.clone(),
                color,
                corners,
                texture_coordinates,
                BlendFactor::SrcAlpha,
                BlendFactor::OneMinusSrcAlpha,
            );
        }
    }
}
