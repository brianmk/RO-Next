use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
use korangar_interface::application::Clip;
use ragnarok_packets::{EntityId, QuestColor, QuestEffectPacket};
use rand_aes::tls::rand_f32;

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{FontSize, ImageType, Scaling, Sprite, TextureLoader};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Actions, Camera};
use crate::{Entity, Map};

/// On-screen height of a damage/heal number in pixels before magnitude and
/// distance scaling. The original client renders damage numbers much larger
/// than the interface text; this is deliberately ~10x the previous 16 px.
pub const BASE_DAMAGE_TEXT_PIXELS: f32 = 85.0;

/// World distance at which the distance scaling is neutral (roughly the
/// distance between the player camera and a monster in melee range).
const DAMAGE_TEXT_REFERENCE_DISTANCE: f32 = 260.0;
/// Clamp for the distance scaling, so that numbers stay readable but never
/// cover half the screen when the camera is very close.
const DAMAGE_TEXT_MIN_DISTANCE_SCALE: f32 = 0.9;
const DAMAGE_TEXT_MAX_DISTANCE_SCALE: f32 = 1.15;
/// A damage number may use at most this fraction of the window height.
const DAMAGE_TEXT_MAX_VIEWPORT_FRACTION: f32 = 0.32;
/// The numbers are spawned slightly above the entity so that the (large) text
/// does not cover the entity itself or its name/health bar.
const DAMAGE_TEXT_HEAD_OFFSET: f32 = 22.0;
/// Additional size for critical hits.
const CRITICAL_DAMAGE_SCALE: f32 = 1.2;

/// Size multiplier for the number of digits of a damage/heal value.
///
/// Values below 1000 use the standard size; every additional digit above 999
/// adds 20%:
///
/// * 0 - 999: 1.0
/// * 1000 - 9999: 1.2
/// * 10000 - 99999: 1.44
/// * 100000 and above: 1.728 (capped)
pub fn damage_magnitude_scale(value: u64) -> f32 {
    if value < 1000 {
        return 1.0;
    }

    let magnitude = (value as f64).log10().floor() as i32 - 2;
    let exponent = magnitude.clamp(0, 3);

    1.2f32.powi(exponent)
}

/// Final on-screen font size (in pixels) for a damage number.
///
/// Combines the base size, the magnitude scale (more digits = bigger) and a
/// mild distance scaling, and clamps the result to the window so that the
/// number is never larger than `DAMAGE_TEXT_MAX_VIEWPORT_FRACTION` of the
/// screen height.
pub fn damage_text_pixels(base_pixels: f32, magnitude_scale: f32, distance: f32, window_height: f32) -> f32 {
    let distance_scale =
        (DAMAGE_TEXT_REFERENCE_DISTANCE / distance.max(1.0)).clamp(DAMAGE_TEXT_MIN_DISTANCE_SCALE, DAMAGE_TEXT_MAX_DISTANCE_SCALE);
    let max_pixels = window_height * DAMAGE_TEXT_MAX_VIEWPORT_FRACTION;

    (base_pixels * magnitude_scale * distance_scale).min(max_pixels).max(1.0)
}

/// Computes the [`FontSize`] for a damage number at a world position.
///
/// `render_damage_text`/`render_text` multiplies the font size by the interface
/// scaling, so it is divided out here: world-space damage numbers should have a
/// fixed on-screen size independent of the UI scale.
fn damage_text_font_size(
    renderer: &GameInterfaceRenderer,
    camera: &dyn Camera,
    position: Point3<f32>,
    window_size: ScreenSize,
    base_pixels: f32,
    magnitude_scale: f32,
) -> FontSize {
    let pixels = damage_text_pixels(base_pixels, magnitude_scale, camera.distance_to(position), window_size.height);
    let scaling = renderer.scaling_factor().max(0.1);

    FontSize(pixels / scaling)
}

pub trait Particle {
    fn update(&mut self, delta_time: f32) -> bool;

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize);
}

fn random_velocity() -> f32 {
    rand_f32() * 40.0 - 20.0
}

pub struct DamageNumber {
    position: Point3<f32>,
    damage_amount: String,
    magnitude_scale: f32,
    velocity_y: f32,
    velocity_x: f32,
    velocity_z: f32,
    timer: f32,
    is_critical: bool,
}

impl DamageNumber {
    pub fn new(position: Point3<f32>, damage_amount: u64, is_critical: bool) -> Self {
        Self {
            position: position + Vector3::new(0.0, DAMAGE_TEXT_HEAD_OFFSET, 0.0),
            damage_amount: damage_amount.to_string(),
            magnitude_scale: damage_magnitude_scale(damage_amount),
            velocity_y: 50.0,
            velocity_x: random_velocity(),
            velocity_z: random_velocity(),
            timer: 0.6,
            is_critical,
        }
    }
}

impl Particle for DamageNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 200.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;
        self.position.x += self.velocity_x * delta_time;
        self.position.z += self.velocity_z * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        let color = match self.is_critical {
            true => Color::rgb_u8(255, 180, 0),
            false => Color::WHITE,
        };

        let base_pixels = match self.is_critical {
            true => BASE_DAMAGE_TEXT_PIXELS * CRITICAL_DAMAGE_SCALE,
            false => BASE_DAMAGE_TEXT_PIXELS,
        };
        let font_size = damage_text_font_size(renderer, camera, self.position, window_size, base_pixels, self.magnitude_scale);

        renderer.render_damage_text(&self.damage_amount, final_position, color, font_size);
    }
}

pub struct Miss {
    position: Point3<f32>,
    timer: f32,
}

impl Miss {
    pub fn new(position: Point3<f32>) -> Self {
        Self {
            position: position + Vector3::new(0.0, DAMAGE_TEXT_HEAD_OFFSET, 0.0),
            timer: 0.6,
        }
    }
}

impl Particle for Miss {
    fn update(&mut self, delta_time: f32) -> bool {
        self.position.y += (self.timer - 0.1).max(0.0) * 70.0 * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };
        let alpha = (self.timer * 10.0).min(1.0);

        let font_size = damage_text_font_size(renderer, camera, self.position, window_size, BASE_DAMAGE_TEXT_PIXELS, 1.0);

        renderer.render_damage_text("miss", final_position, Color::rgba(1.0, 0.0, 0.0, alpha), font_size);
    }
}

pub struct HealNumber {
    position: Point3<f32>,
    heal_amount: String,
    magnitude_scale: f32,
    velocity_y: f32,
    timer: f32,
}

impl HealNumber {
    pub fn new(position: Point3<f32>, heal_amount: u64) -> Self {
        Self {
            position: position + Vector3::new(0.0, DAMAGE_TEXT_HEAD_OFFSET, 0.0),
            heal_amount: heal_amount.to_string(),
            magnitude_scale: damage_magnitude_scale(heal_amount),
            velocity_y: 50.0,
            timer: 1.0,
        }
    }
}

impl Particle for HealNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 50.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        let font_size = damage_text_font_size(
            renderer,
            camera,
            self.position,
            window_size,
            BASE_DAMAGE_TEXT_PIXELS,
            self.magnitude_scale,
        );

        renderer.render_damage_text(&self.heal_amount, final_position, Color::rgb_u8(30, 255, 30), font_size);
    }
}

/// An emotion played above an entity, rendered from the animation frames of
/// `emotion.act`. Despawns after playing through the animation once.
pub struct Emote {
    position: Point3<f32>,
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
    emotion_id: usize,
    timer: f32,
}

impl Emote {
    /// Offset above the entity position so the emote hovers over the head.
    const POSITION_OFFSET: Vector3<f32> = Vector3::new(0.0, 30.0, 0.0);

    pub fn new(entity_position: Point3<f32>, sprite: Arc<Sprite>, actions: Arc<Actions>, emotion_id: usize) -> Self {
        Self {
            position: entity_position + Self::POSITION_OFFSET,
            sprite,
            actions,
            emotion_id,
            timer: 0.0,
        }
    }

    /// Milliseconds each animation frame is displayed for, scaled by the
    /// per-action delay of the loaded actions.
    fn frame_time(&self) -> f32 {
        let delay = self.actions.delays[self.emotion_id % self.actions.delays.len()];
        delay * 50.0
    }

    fn frame_count(&self) -> usize {
        let action = &self.actions.actions[self.emotion_id % self.actions.actions.len()];
        action.motions.len()
    }
}

impl Particle for Emote {
    fn update(&mut self, delta_time: f32) -> bool {
        self.timer += delta_time * 1000.0;
        self.timer < self.frame_count() as f32 * self.frame_time()
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        let frame = (self.timer / self.frame_time()) as usize;

        self.actions.render_sprite_frame(
            renderer,
            &self.sprite,
            self.emotion_id,
            frame,
            final_position,
            ScreenClip::unbound(),
            Color::WHITE,
            1.0,
        );
    }
}

pub struct QuestIcon {
    position: Point3<f32>,
    texture: Arc<Texture>,
    color: Color,
}

impl QuestIcon {
    pub fn new(texture_loader: &TextureLoader, map: &Map, quest_effect: QuestEffectPacket) -> Option<Self> {
        // TODO: Use the height of the entity as offset.
        let icon_offset = Vector3::new(0.0, 25.0, 0.0);
        let Some(entity_position) = map.get_world_position(quest_effect.position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] quest icon is out of map bounds", "error".red());
            return None;
        };

        let position = entity_position + icon_offset;
        let effect_id = quest_effect.effect as usize;
        let texture = texture_loader
            .get_or_load(
                &format!("유저인터페이스\\minimap\\quest_{}_{}.bmp", effect_id, 1), /* 1 - 3 */
                ImageType::Color,
            )
            .unwrap();
        let color = match quest_effect.color {
            QuestColor::Yellow => Color::rgb_u8(200, 200, 30),
            QuestColor::Orange => Color::rgb_u8(200, 100, 30),
            QuestColor::Green => Color::rgb_u8(30, 200, 30),
            QuestColor::Purple => Color::rgb_u8(200, 30, 200),
        };

        Some(Self { position, texture, color })
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize, scaling_factor: f32) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_sprite(
            self.texture.clone(),
            final_position - ScreenSize::uniform(15.0 * scaling_factor),
            ScreenSize::uniform(30.0 * scaling_factor),
            ScreenClip::unbound(),
            self.color,
            true,
        );
    }
}

#[derive(Default)]
pub struct ParticleHolder {
    particles: Vec<Box<dyn Particle + Send + Sync>>,
    quest_icons: HashMap<EntityId, QuestIcon>,
}

impl ParticleHolder {
    /// Number of active particles (used by the performance instrumentation).
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        self.particles.push(particle);
    }

    pub fn add_quest_icon(&mut self, texture_loader: &TextureLoader, map: &Map, quest_effect: QuestEffectPacket) {
        let entity_id = quest_effect.entity_id;

        if let Some(quest_icon) = QuestIcon::new(texture_loader, map, quest_effect) {
            self.quest_icons.insert(entity_id, quest_icon);
        }
    }

    pub fn remove_quest_icon(&mut self, entity_id: EntityId) {
        self.quest_icons.remove(&entity_id);
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.quest_icons.clear();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("update particles"))]
    pub fn update(&mut self, delta_time: f32) {
        self.particles.retain_mut(|particle| particle.update(delta_time));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render particles"))]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
        scaling: Scaling,
        entities: &[Entity],
    ) {
        self.particles
            .iter()
            .for_each(|particle| particle.render(renderer, camera, window_size));

        entities
            .iter()
            .filter_map(|entity| self.quest_icons.get(&entity.get_entity_id()))
            .for_each(|quest_icon| quest_icon.render(renderer, camera, window_size, scaling.get_factor()));
    }
}

#[cfg(test)]
mod tests {
    use super::{BASE_DAMAGE_TEXT_PIXELS, damage_magnitude_scale, damage_text_pixels};

    #[test]
    fn values_below_1000_use_the_standard_size() {
        for value in [0, 1, 9, 10, 99, 999] {
            assert_eq!(damage_magnitude_scale(value), 1.0, "value {value}");
        }
    }

    #[test]
    fn each_extra_digit_adds_20_percent() {
        assert!((damage_magnitude_scale(1_000) - 1.2).abs() < 1e-5);
        assert!((damage_magnitude_scale(9_999) - 1.2).abs() < 1e-5);
        assert!((damage_magnitude_scale(10_000) - 1.44).abs() < 1e-5);
        assert!((damage_magnitude_scale(99_999) - 1.44).abs() < 1e-5);
        assert!((damage_magnitude_scale(100_000) - 1.728).abs() < 1e-4);
        assert!((damage_magnitude_scale(1_000_000) - 1.728).abs() < 1e-4);
    }

    #[test]
    fn base_size_is_much_larger_than_the_old_chat_sized_font() {
        // The old damage number font size was 16 px; the current base is 85 px
        // (the user asked to halve the first, larger iteration).
        assert!(BASE_DAMAGE_TEXT_PIXELS >= 80.0);
    }

    #[test]
    fn distance_scaling_is_clamped() {
        // Far away numbers shrink, but only to 90% of the base size.
        let far = damage_text_pixels(BASE_DAMAGE_TEXT_PIXELS, 1.0, 10_000.0, 720.0);
        let reference = damage_text_pixels(BASE_DAMAGE_TEXT_PIXELS, 1.0, 260.0, 720.0);

        assert!(far < reference);
        assert!(far >= BASE_DAMAGE_TEXT_PIXELS * 0.9 - 1e-3);

        // Very close numbers grow a little, but are capped to 115% of base.
        let close = damage_text_pixels(BASE_DAMAGE_TEXT_PIXELS, 1.0, 1.0, 720.0);
        assert!(close <= BASE_DAMAGE_TEXT_PIXELS * 1.15 + 1e-3);
    }

    #[test]
    fn magnitude_and_viewport_clamps_are_applied() {
        // 85 * 1.728 = 146.88, below the 32% of 720 = 230.4 viewport cap.
        let pixels = damage_text_pixels(BASE_DAMAGE_TEXT_PIXELS, damage_magnitude_scale(1_000_000), 260.0, 720.0);
        assert!((pixels - 146.88).abs() < 1e-2);

        // Small window: the base size (85) is below the 32% cap of 192, so it is
        // not reduced.
        let pixels = damage_text_pixels(BASE_DAMAGE_TEXT_PIXELS, 1.0, 260.0, 600.0);
        assert!((pixels - BASE_DAMAGE_TEXT_PIXELS).abs() < 1e-3);

        // But a larger magnitude is capped at 32% of the window height.
        let pixels = damage_text_pixels(BASE_DAMAGE_TEXT_PIXELS, damage_magnitude_scale(1_000_000), 260.0, 400.0);
        assert!((pixels - 128.0).abs() < 1e-3);
    }
}
