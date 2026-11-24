//! Procedural Storm Gust cloud.
//!
//! The snow part of Storm Gust is the official `stormgust.str` effect (texture
//! `snow_a.bmp`). The dark storm cloud above the area is *not* a `.str`/`.spr`
//! asset of the 2019 client: the official client draws it as a hard-coded 3D
//! effect from the `cloud1.tga` .. `cloud7.tga` textures. Data files therefore
//! cannot supply it, so this module reproduces the cloud with a small set of
//! camera-facing, slowly swirling billboards for the duration of the skill.
//!
//! Everything visible comes from the skill data
//! (`archive/data/client/skills.ron`, `clouds` rows): the texture list,
//! billboard count, disc radius, height/size/spin spreads, tint, alpha and the
//! fade timing. Only the deterministic puff distribution formula (golden angle
//! and integer hash) is code, because it is the shape of the procedural layer,
//! not content.

use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, render_world_quad_blended};
use crate::world::skills::CloudSpec;

/// Golden angle (radians) of the deterministic puff distribution.
const GOLDEN_ANGLE: f32 = 2.399_963;

/// Deterministic pseudo-random placement factor of one puff.
///
/// The historical procedural constants used four independent integer hashes
/// (`index * 37 % 11`, `index * 53 % 9`, `index * 29 % 7`, `index * 17 % 5`)
/// divided by a fixed divisor; they are kept verbatim so the cloud looks
/// exactly like before the data migration.
fn puff_factor(index: usize, multiplier: usize, modulus: usize, divisor: f32) -> f32 {
    ((index * multiplier % modulus) as f32) / divisor
}

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
    spec: &'static CloudSpec,
    time: f32,
    gets_deleted: bool,
}

impl StormCloud {
    /// Loads the cloud textures; returns `None` when none of them is available.
    ///
    /// The `spec` is the matching `clouds` row of the skill data (Storm Gust
    /// 87/89, `Success` phase).
    pub fn new(texture_loader: &TextureLoader, center: Point3<f32>, spec: &'static CloudSpec) -> Option<Self> {
        let textures: Vec<Arc<Texture>> = spec
            .textures
            .iter()
            .filter_map(|path| texture_loader.get_or_load(path, ImageType::Color).ok())
            .collect();

        if textures.is_empty() {
            return None;
        }

        let mut puffs = Vec::with_capacity(spec.count);

        for index in 0..spec.count {
            // Deterministic pseudo-random placement: golden-angle distribution
            // around the cell with a slight height variation.
            let angle = index as f32 * GOLDEN_ANGLE;
            let radius = spec.radius * (0.25 + 0.75 * puff_factor(index, 37, 11, 10.0));
            let height = spec.base_height + puff_factor(index, 53, 9, 8.0) * spec.height_variation;

            puffs.push(CloudPuff {
                texture: textures[index % textures.len()].clone(),
                offset: Vector3::new(angle.cos() * radius, height, angle.sin() * radius),
                size: spec.size_min + puff_factor(index, 29, 7, 6.0) * spec.size_variation,
                spin: spec.spin_min + puff_factor(index, 17, 5, 5.0) * spec.spin_variation,
                angle,
            });
        }

        Some(Self {
            puffs,
            center,
            spec,
            time: 0.0,
            gets_deleted: false,
        })
    }

    fn alpha(&self) -> f32 {
        let elapsed = self.time * 1000.0;
        let duration = self.spec.duration_ms as f32;

        let fade_in = (elapsed / self.spec.fade_in_ms as f32).clamp(0.0, 1.0);
        // A zero fade-out would divide by zero; the committed data uses 700 ms.
        let fade_out = if self.spec.fade_out_ms == 0 {
            1.0
        } else {
            ((duration - elapsed) / self.spec.fade_out_ms as f32).clamp(0.0, 1.0)
        };

        self.spec.alpha * fade_in.min(fade_out).clamp(0.0, 1.0)
    }
}

impl EffectBase for StormCloud {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center)
    }

    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.time += delta_time;
        !self.gets_deleted && self.time * 1000.0 < self.spec.duration_ms as f32
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
        let color = Color::rgba(self.spec.color.red, self.spec.color.green, self.spec.color.blue, alpha);

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
