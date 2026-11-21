//! Classic level-99 aura rendered below player entities.
//!
//! The original client shows a rotating ring plus floating sparkles below
//! characters whose base level is at least 99. The classic aura textures
//! (`aurafloat.tga`, `auraring.bmp`) are not part of the CondaRO data set, so
//! the aura is built procedurally from the effect textures that do exist:
//!
//! * `effect/ring_liner.tga` — rotating outer ring,
//! * `effect/ring_blue.tga` / `effect/ring_red.tga` — inner ring for the blue /
//!   gold variants,
//! * `effect/glow01..glow04.bmp` — soft ground glow,
//! * `effect/pikapika2.bmp` — floating sparkles.
//!
//! Every texture is optional: if one is missing the aura uses the remaining
//! ones (and if none can be loaded the aura is simply not created, so a
//! trimmed data set can never cause a panic or a black quad).
//!
//! The aura color is chosen from the configured [`AuraStyle`]:
//!
//! * `Auto` (default) — classic tiers: cyan for level 99, violet for
//!   transcendent/rebirth level 99, blue for 100+, gold for 175+,
//! * `Off` — no aura,
//! * `Cyan`, `Violet`, `Blue`, `Gold` — explicit style for all auras.
//!
//! The effect is attached to the entity (`EffectCenter::Entity`), follows it
//! while walking and removes itself when the entity despawns, dies or drops
//! below level 99. It is drawn with the regular effect renderer, so it is
//! depth tested like the other ground effects (below the sprite, occluded by
//! walls through the normal depth buffer).

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use cgmath::{Point3, Vector2, Vector3};

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::settings::AuraStyle;
use crate::world::Camera;
use crate::world::effect::{EffectBase, EffectCenter, render_world_quad};

/// Minimum base level for the classic aura.
pub const AURA_MINIMUM_LEVEL: usize = 99;
/// Base radius of the ground rings in world units.
const AURA_RADIUS: f32 = 13.0;
/// Height of the ground rings above the floor (avoids z-fighting).
const RING_HEIGHT: f32 = 0.25;
/// Number of floating sparkles.
const SPARKLE_COUNT: usize = 6;
/// Radians per second at which the outer ring rotates.
const RING_ROTATION_SPEED: f32 = 0.7;
/// Radians per second at which the inner ring rotates (opposite direction).
const INNER_RING_ROTATION_SPEED: f32 = -1.1;
/// Texture names verified to exist in the CondaRO data set.
const RING_LINER_TEXTURE: &str = "effect\\ring_liner.tga";
const RING_BLUE_TEXTURE: &str = "effect\\ring_blue.tga";
const RING_RED_TEXTURE: &str = "effect\\ring_red.tga";
const GLOW_TEXTURES: [&str; 4] = [
    "effect\\glow01.bmp",
    "effect\\glow02.bmp",
    "effect\\glow03.bmp",
    "effect\\glow04.bmp",
];
const SPARKLE_TEXTURE: &str = "effect\\pikapika2.bmp";

/// Classic tier detection for a base level (used by [`AuraStyle::Auto`]).
pub fn auto_aura_style(level: usize, transcendent: bool) -> AuraStyle {
    if level >= 175 {
        AuraStyle::Gold
    } else if level >= 100 {
        AuraStyle::Blue
    } else if transcendent {
        AuraStyle::Violet
    } else {
        AuraStyle::Cyan
    }
}

/// Overall size multiplier of the aura for a base level (higher levels get the
/// larger classic aura).
pub fn aura_tier_scale(level: usize) -> f32 {
    if level >= 175 {
        1.45
    } else if level >= 150 {
        1.3
    } else if level >= 100 {
        1.15
    } else {
        1.0
    }
}

/// Color of the aura for the given (explicit) style.
pub fn aura_color(style: AuraStyle) -> Color {
    match style {
        AuraStyle::Cyan | AuraStyle::Auto | AuraStyle::Off => Color::rgb_u8(40, 200, 255),
        AuraStyle::Violet => Color::rgb_u8(165, 70, 255),
        AuraStyle::Blue => Color::rgb_u8(45, 100, 255),
        AuraStyle::Gold => Color::rgb_u8(255, 185, 40),
    }
}

/// The level-99 aura effect of one entity.
pub struct Aura {
    center: EffectCenter,
    style_cell: Arc<AtomicU8>,
    level: usize,
    transcendent: bool,
    ring_liner: Option<Arc<Texture>>,
    ring_blue: Option<Arc<Texture>>,
    ring_red: Option<Arc<Texture>>,
    glows: Vec<Arc<Texture>>,
    sparkle: Option<Arc<Texture>>,
    time: f32,
    gets_deleted: bool,
}

impl Aura {
    /// Create the aura for an entity with the given level.
    ///
    /// Returns `None` if none of the required effect textures exist in the game
    /// data (graceful fallback for trimmed data sets).
    pub fn new(style_cell: Arc<AtomicU8>, level: usize, transcendent: bool, texture_loader: &TextureLoader) -> Option<Self> {
        let load = |name: &str| texture_loader.get_or_load(name, ImageType::Color).ok();

        let ring_liner = load(RING_LINER_TEXTURE);
        let ring_blue = load(RING_BLUE_TEXTURE);
        let ring_red = load(RING_RED_TEXTURE);
        let glows = GLOW_TEXTURES.iter().filter_map(|name| load(name)).collect::<Vec<_>>();
        let sparkle = load(SPARKLE_TEXTURE);

        if ring_liner.is_none() && ring_blue.is_none() && ring_red.is_none() && glows.is_empty() && sparkle.is_none() {
            return None;
        }

        Some(Self {
            // The actual position is filled on the first update.
            center: EffectCenter::Position(Point3::new(0.0, 0.0, 0.0)),
            style_cell,
            level,
            transcendent,
            ring_liner,
            ring_blue,
            ring_red,
            glows,
            sparkle,
            time: 0.0,
            gets_deleted: false,
        })
    }

    /// Attach the aura to an entity (the aura follows it from now on).
    pub fn attach_to_entity(&mut self, entity_id: ragnarok_packets::EntityId, position: Point3<f32>) {
        self.center = EffectCenter::Entity(entity_id, position);
    }

    fn configured_style(&self) -> AuraStyle {
        AuraStyle::decode(self.style_cell.load(Ordering::Relaxed))
    }

    /// The style actually used for rendering (Auto resolves to the tier).
    pub fn resolved_style(&self) -> AuraStyle {
        match self.configured_style() {
            AuraStyle::Auto => auto_aura_style(self.level, self.transcendent),
            style => style,
        }
    }

    /// The inner ring texture for the current style, with a fallback.
    fn inner_ring_texture(&self, style: AuraStyle) -> Option<Arc<Texture>> {
        let preferred = match style {
            AuraStyle::Blue => self.ring_blue.as_ref(),
            AuraStyle::Gold => self.ring_red.as_ref(),
            _ => None,
        };

        preferred.or(self.ring_liner.as_ref()).or(self.ring_blue.as_ref()).cloned()
    }

    fn glow_texture(&self, style: AuraStyle) -> Option<Arc<Texture>> {
        if self.glows.is_empty() {
            return None;
        }

        let index = match style {
            AuraStyle::Cyan | AuraStyle::Auto | AuraStyle::Off => 0,
            AuraStyle::Violet => 3,
            AuraStyle::Blue => 2,
            AuraStyle::Gold => 1,
        };

        self.glows.get(index).or_else(|| self.glows.first()).cloned()
    }

    /// Corners of a horizontal quad at the given height above the anchor.
    fn ground_quad(anchor: Point3<f32>, radius: f32, angle: f32, height: f32) -> [Point3<f32>; 4] {
        let directions = [
            Vector3::new(-1.0, 0.0, -1.0),
            Vector3::new(1.0, 0.0, -1.0),
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
        ];

        directions.map(|direction| {
            let x = direction.x * angle.cos() - direction.z * angle.sin();
            let z = direction.x * angle.sin() + direction.z * angle.cos();

            anchor + Vector3::new(x * radius, height, z * radius)
        })
    }

    fn quad_texture_coordinates() -> [Vector2<f32>; 4] {
        [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]
    }
}

impl EffectBase for Aura {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if self.gets_deleted {
            return false;
        }

        // Off: the aura removes itself, the client will not spawn a new one
        // while the setting stays Off.
        if self.configured_style() == AuraStyle::Off {
            return false;
        }

        let Some(entity_id) = (match &self.center {
            EffectCenter::Entity(entity_id, _) => Some(*entity_id),
            EffectCenter::Position(_) => None,
        }) else {
            return false;
        };

        let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id) else {
            // The entity despawned (map change, distance, death cleanup).
            return false;
        };

        if entity.is_dead() {
            return false;
        }

        let Some(level) = entity.get_base_level() else {
            // Level unknown for this entity (e.g. population shell without a
            // level packet) — fall back gracefully and do not render an aura.
            return false;
        };

        if level < AURA_MINIMUM_LEVEL {
            // The level dropped below the aura threshold (or the entity was
            // re-created with a lower level).
            return false;
        }

        self.level = level;
        self.transcendent = entity.is_transcendent_job();
        self.center.follow_entity(entities);
        self.time += delta_time;

        true
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let style = self.resolved_style();

        if style == AuraStyle::Off {
            return;
        }

        let anchor = self.center.to_position();
        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };

        let color = aura_color(style);
        let scale = aura_tier_scale(self.level);
        let pulse = 0.8 + 0.2 * (self.time * 2.3).sin();
        let radius = AURA_RADIUS * scale * (1.0 + 0.035 * (self.time * 1.8).sin());
        let texture_coordinates = Self::quad_texture_coordinates();

        // Outer rotating ring.
        if let Some(texture) = self.ring_liner.clone() {
            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture,
                Color::rgba(
                    color.red * 0.9 * pulse,
                    color.green * 0.9 * pulse,
                    color.blue * 0.9 * pulse,
                    1.0,
                ),
                Self::ground_quad(anchor, radius, self.time * RING_ROTATION_SPEED, RING_HEIGHT),
                texture_coordinates,
            );
        }

        // Inner counter-rotating ring, slightly smaller and brighter.
        if let Some(texture) = self.inner_ring_texture(style) {
            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture,
                Color::rgba(color.red * 0.8 * pulse, color.green * 0.8 * pulse, color.blue * pulse, 1.0),
                Self::ground_quad(anchor, radius * 0.62, self.time * INNER_RING_ROTATION_SPEED, RING_HEIGHT + 0.05),
                texture_coordinates,
            );
        }

        // Soft ground glow that pulses with the aura.
        if let Some(texture) = self.glow_texture(style) {
            let glow = 0.35 + 0.25 * (self.time * 1.6).sin().max(0.0);
            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture,
                Color::rgba(color.red * glow, color.green * glow, color.blue * glow, 1.0),
                Self::ground_quad(anchor, radius * 1.15, self.time * 0.2, RING_HEIGHT - 0.05),
                texture_coordinates,
            );
        }

        // Floating sparkles that rise, sink and slowly orbit around the feet.
        if let Some(texture) = self.sparkle.clone() {
            for index in 0..SPARKLE_COUNT {
                let phase = index as f32 * std::f32::consts::TAU / SPARKLE_COUNT as f32;
                let angle = phase + self.time * 0.35;
                let sparkle_radius = radius * (0.5 + 0.18 * (self.time * 1.7 + phase).sin().abs());
                let height = 2.0 + 5.0 * (0.5 + 0.5 * (self.time * 1.3 + phase * 1.7).sin());

                let sparkle_center = anchor + Vector3::new(angle.cos() * sparkle_radius, height, angle.sin() * sparkle_radius);
                let sparkle_size = (0.9 + 0.5 * (self.time * 2.1 + phase).sin().abs()) * scale;
                let sparkle_alpha = 0.55 + 0.45 * (self.time * 3.0 + phase * 2.3).sin().abs();

                render_world_quad(
                    renderer,
                    camera,
                    anchor,
                    screen_size,
                    texture.clone(),
                    Color::rgba(
                        (color.red * 0.7 + 0.3) * sparkle_alpha,
                        (color.green * 0.7 + 0.3) * sparkle_alpha,
                        (color.blue * 0.7 + 0.3) * sparkle_alpha,
                        1.0,
                    ),
                    Self::ground_quad(sparkle_center, sparkle_size, angle, 0.0),
                    texture_coordinates,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_tiers_follow_the_classic_rules() {
        assert_eq!(auto_aura_style(99, false), AuraStyle::Cyan);
        assert_eq!(auto_aura_style(99, true), AuraStyle::Violet);
        assert_eq!(auto_aura_style(100, false), AuraStyle::Blue);
        assert_eq!(auto_aura_style(174, false), AuraStyle::Blue);
        assert_eq!(auto_aura_style(175, false), AuraStyle::Gold);
        assert_eq!(auto_aura_style(250, false), AuraStyle::Gold);
    }

    #[test]
    fn aura_styles_have_distinct_colors() {
        let colors = [AuraStyle::Cyan, AuraStyle::Violet, AuraStyle::Blue, AuraStyle::Gold].map(aura_color);

        for (index, color) in colors.iter().enumerate() {
            for other in &colors[index + 1..] {
                assert_ne!(color, other);
            }
        }
    }

    #[test]
    fn higher_levels_get_larger_auras() {
        assert!(aura_tier_scale(100) > aura_tier_scale(99));
        assert!(aura_tier_scale(150) > aura_tier_scale(100));
        assert!(aura_tier_scale(175) > aura_tier_scale(150));
    }

    #[test]
    fn aura_style_round_trips_through_the_atomic_cell() {
        for style in [
            AuraStyle::Auto,
            AuraStyle::Off,
            AuraStyle::Cyan,
            AuraStyle::Violet,
            AuraStyle::Blue,
            AuraStyle::Gold,
        ] {
            assert_eq!(AuraStyle::decode(style.encode()), style);
        }

        // Invalid values fall back to Auto.
        assert_eq!(AuraStyle::decode(255), AuraStyle::Auto);
    }
}
