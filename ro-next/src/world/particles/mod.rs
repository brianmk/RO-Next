use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
use ragnarok_packets::{Direction, EntityId, QuestEffectPacket};
use ro_next_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use ro_next_debug::logging::Colorize;
use ro_next_interface::application::Clip;

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{FontSize, GameFileLoader, ImageType, Scaling, Sprite, TextureLoader};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Actions, Camera};
use crate::{Entity, Map};

/// Hearing range of the arrow impact sound in world units.
const ARROW_HIT_SOUND_RANGE: f32 = 250.0;

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

/// Renders a damage/heal number, using the installed sprite damage font and
/// falling back to the interface text renderer.
#[allow(clippy::too_many_arguments)] // one argument per render parameter (position, size, colour, font, ...)
fn render_number(
    renderer: &GameInterfaceRenderer,
    camera: &dyn Camera,
    window_size: ScreenSize,
    text: &str,
    position: Point3<f32>,
    base_pixels: f32,
    magnitude_scale: f32,
    color: Color,
) {
    let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();
    let screen_position = camera.clip_to_screen_space(clip_space_position);
    let final_position = ScreenPosition {
        left: screen_position.x * window_size.width,
        top: screen_position.y * window_size.height,
    };

    let height = damage_text_pixels(base_pixels, magnitude_scale, camera.distance_to(position), window_size.height);

    if let Some(font) = crate::world::damage_font::damage_font() {
        let screen_clip = ScreenClip {
            left: 0.0,
            top: 0.0,
            right: window_size.width,
            bottom: window_size.height,
        };

        font.render(renderer, text, final_position, height, color, screen_clip);
        return;
    }

    let font_size = FontSize(height / renderer.scaling_factor().max(0.1));
    renderer.render_damage_text(text, final_position, color, font_size);
}

pub trait Particle {
    fn update(&mut self, delta_time: f32) -> bool;

    /// Sound played at the particle's impact position when the particle ends.
    ///
    /// The default is silence; flying projectiles override it so the arrow hit
    /// sound is played exactly when the arrow lands instead of when it is
    /// spawned.
    fn impact_sound(&self) -> Option<(SoundEffectKey, Point3<f32>)> {
        None
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize);

    /// World position used for distance culling.
    fn culling_position(&self) -> Point3<f32>;
}

/// Upward speed of a damage number at spawn (world units per second).
///
/// Together with [`DAMAGE_NUMBER_GRAVITY`] this forms the parabolic arc of the
/// official "damage fountain": the number rises, peaks and falls back while it
/// drifts away from the hit entity.
const DAMAGE_NUMBER_UP_SPEED: f32 = 45.0;
/// Downward acceleration of the damage-number arc (world units / s^2).
const DAMAGE_NUMBER_GRAVITY: f32 = 150.0;
/// Horizontal drift speed of the fountain (world units per second).
const DAMAGE_NUMBER_DRIFT_SPEED: f32 = 8.0;
/// Lifetime of a damage number in seconds.
const DAMAGE_NUMBER_LIFETIME: f32 = 0.9;

/// Horizontal drift direction of the official damage fountain.
///
/// The classic client drifts the numbers diagonally away from the hit entity,
/// based on the octant the entity faces (`CMsgEffect::MsgEffectDriftDirection`:
/// angle 0..90 -> (-0.8, +0.8), 90..180 -> (-0.8, -0.8), 180..270 ->
/// (+0.8, -0.8), 270..360 -> (+0.8, +0.8)).
pub fn damage_number_drift(direction: Direction) -> Vector3<f32> {
    let (x, z) = match u16::from(direction) {
        // North / North-East.
        0 | 1 => (-0.8, 0.8),
        // East / South-East.
        2 | 3 => (-0.8, -0.8),
        // South / South-West.
        4 | 5 => (0.8, -0.8),
        // West / North-West.
        _ => (0.8, 0.8),
    };

    Vector3::new(x, 0.0, z) * DAMAGE_NUMBER_DRIFT_SPEED
}

pub struct DamageNumber {
    position: Point3<f32>,
    /// Spawn position of the arc (already offset above the head).
    origin: Point3<f32>,
    /// Horizontal drift velocity of the fountain.
    drift: Vector3<f32>,
    damage_amount: String,
    magnitude_scale: f32,
    elapsed: f32,
    is_critical: bool,
}

impl DamageNumber {
    /// Creates a damage number above `position` that follows the official
    /// damage fountain: a parabolic arc that drifts diagonally away from the
    /// hit entity based on the direction it faces. `direction` is optional so
    /// effects without an entity (e.g. ground damage) still show a number.
    pub fn new(position: Point3<f32>, damage_amount: u64, is_critical: bool, direction: Direction) -> Self {
        let origin = position + Vector3::new(0.0, DAMAGE_TEXT_HEAD_OFFSET, 0.0);

        Self {
            position: origin,
            origin,
            drift: damage_number_drift(direction),
            damage_amount: damage_amount.to_string(),
            magnitude_scale: damage_magnitude_scale(damage_amount),
            elapsed: 0.0,
            is_critical,
        }
    }
}

impl Particle for DamageNumber {
    fn culling_position(&self) -> Point3<f32> {
        self.position
    }

    fn update(&mut self, delta_time: f32) -> bool {
        self.elapsed += delta_time;
        let time = self.elapsed;

        // Parabolic arc: rises with the spawn speed and falls under gravity
        // while the horizontal drift carries it away from the entity.
        let height = DAMAGE_NUMBER_UP_SPEED * time - 0.5 * DAMAGE_NUMBER_GRAVITY * time * time;

        self.position = self.origin + self.drift * time + Vector3::new(0.0, height, 0.0);

        self.elapsed < DAMAGE_NUMBER_LIFETIME
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
        // Official sprite digits when the HD damage font is installed, text
        // rendering as the fallback for data sets without it.
        if let Some(font) = crate::world::damage_font::damage_font() {
            let height = damage_text_pixels(
                base_pixels,
                self.magnitude_scale,
                camera.distance_to(self.position),
                window_size.height,
            );
            let screen_clip = ScreenClip {
                left: 0.0,
                top: 0.0,
                right: window_size.width,
                bottom: window_size.height,
            };

            font.render(renderer, &self.damage_amount, final_position, height, color, screen_clip);
            return;
        }

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
    fn culling_position(&self) -> Point3<f32> {
        self.position
    }

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
    origin: Point3<f32>,
    heal_amount: String,
    magnitude_scale: f32,
    elapsed: f32,
}

impl HealNumber {
    pub fn new(position: Point3<f32>, heal_amount: u64) -> Self {
        let origin = position + Vector3::new(0.0, DAMAGE_TEXT_HEAD_OFFSET, 0.0);

        Self {
            position: origin,
            origin,
            heal_amount: heal_amount.to_string(),
            magnitude_scale: damage_magnitude_scale(heal_amount),
            elapsed: 0.0,
        }
    }
}

impl Particle for HealNumber {
    fn culling_position(&self) -> Point3<f32> {
        self.position
    }

    fn update(&mut self, delta_time: f32) -> bool {
        self.elapsed += delta_time;
        let time = self.elapsed;
        let height = DAMAGE_NUMBER_UP_SPEED * time - 0.5 * DAMAGE_NUMBER_GRAVITY * time * time;
        self.position = self.origin + Vector3::new(0.0, height, 0.0);
        self.elapsed < DAMAGE_NUMBER_LIFETIME
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        render_number(
            renderer,
            camera,
            window_size,
            &self.heal_amount,
            self.position,
            BASE_DAMAGE_TEXT_PIXELS,
            self.magnitude_scale,
            Color::rgb_u8(30, 255, 30),
        );
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
    fn culling_position(&self) -> Point3<f32> {
        self.position
    }

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
            ro_next_debug::logging::print_debug!("[{}] quest icon is out of map bounds", "error".red());
            return None;
        };

        let position = entity_position + icon_offset;
        let effect_id = quest_effect.effect as usize;
        // Texture template, frame and tints are data-driven
        // (`data\client\world_config.ron`, see `client_config::world_config`).
        // A texture the archives do not contain skips the icon instead of
        // crashing the client, so a broken data override degrades visibly.
        let texture_path = crate::client_config::quest_icon_texture(effect_id);
        let Ok(texture) = texture_loader.get_or_load(&texture_path, ImageType::Color) else {
            crate::logging::log_warn!("quest icon: texture {texture_path:?} could not be loaded, skipping the icon");
            return None;
        };
        let color = crate::client_config::quest_icon_color(quest_effect.color);

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

/// Default maximum number of simultaneously alive particles (damage/heal
/// numbers, miss texts, emotes). In a crowded town with a population engine
/// the damage number spam is the most allocation-heavy per-frame path, so the
/// oldest particles are recycled when the budget is reached.
const DEFAULT_MAXIMUM_PARTICLES: usize = 300;
/// Default distance (in world units, one map cell is 5 units) beyond which
/// particles are no longer rendered. `0.0` disables the distance culling.
const DEFAULT_PARTICLE_CULL_DISTANCE: f32 = 2200.0;

/// Environment knob for [`ParticleHolder::maximum_particles`]; `0` disables
/// the cap.
const MAXIMUM_PARTICLES_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_MAX_PARTICLES";
/// Environment knob for [`ParticleHolder::cull_distance`]; `0` disables
/// culling.
const PARTICLE_CULL_DISTANCE_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_PARTICLE_CULL_DISTANCE";

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

/// Number of existing particles that have to be evicted for a new one.
///
/// Pure function so the budget policy can be unit tested without client state.
fn particles_to_evict(current: usize, maximum: usize) -> usize {
    if maximum > 0 && current >= maximum {
        // Evict the oldest particle to make room, so a burst can never grow the
        // list past the budget.
        1
    } else {
        0
    }
}

pub struct ParticleHolder {
    particles: Vec<Box<dyn Particle + Send + Sync>>,
    quest_icons: HashMap<EntityId, QuestIcon>,
    /// Maximum number of simultaneously alive particles (`0` = unlimited).
    maximum_particles: usize,
    /// Distance in world units beyond which particles are culled (`0` = off).
    cull_distance: f32,
    /// Particles evicted by the budget since startup.
    dropped_particles: u64,
    /// Particles rendered / culled by distance in the last frame.
    rendered_in_last_frame: std::cell::Cell<u32>,
    culled_in_last_frame: std::cell::Cell<u32>,
    /// Audio engine used for particle impact sounds (arrow hits).
    audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
    /// Pre-loaded `_hit_arrow.wav` key, played when an arrow projectile lands.
    arrow_hit_sound: Option<SoundEffectKey>,
}

impl Default for ParticleHolder {
    fn default() -> Self {
        Self {
            particles: Vec::new(),
            quest_icons: HashMap::new(),
            maximum_particles: environment_usize(MAXIMUM_PARTICLES_ENVIRONMENT_VARIABLE, DEFAULT_MAXIMUM_PARTICLES),
            cull_distance: environment_f32(PARTICLE_CULL_DISTANCE_ENVIRONMENT_VARIABLE, DEFAULT_PARTICLE_CULL_DISTANCE),
            dropped_particles: 0,
            rendered_in_last_frame: std::cell::Cell::new(0),
            culled_in_last_frame: std::cell::Cell::new(0),
            audio_engine: None,
            arrow_hit_sound: None,
        }
    }
}

impl ParticleHolder {
    /// Number of active particles (used by the performance instrumentation).
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Rendered and distance-culled particle counts of the last frame.
    pub fn render_counts(&self) -> (u32, u32) {
        (self.rendered_in_last_frame.get(), self.culled_in_last_frame.get())
    }

    /// Number of particles evicted by the budget since startup.
    pub fn dropped_particles(&self) -> u64 {
        self.dropped_particles
    }

    /// Enables impact sounds and pre-loads the arrow hit sound.
    pub fn install_audio(&mut self, audio_engine: Arc<AudioEngine<GameFileLoader>>) {
        self.arrow_hit_sound = Some(audio_engine.load("data\\wav\\_hit_arrow.wav"));
        self.audio_engine = Some(audio_engine);
    }

    /// Key of the classic `_hit_arrow.wav` arrow impact sound, if audio is
    /// installed.
    pub fn arrow_hit_sound(&self) -> Option<SoundEffectKey> {
        self.arrow_hit_sound
    }

    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        let evict = particles_to_evict(self.particles.len(), self.maximum_particles);

        if evict > 0 {
            // Remove the oldest particle (swap_remove keeps this O(1); the render
            // order of independent particles is irrelevant).
            self.particles.swap_remove(0);
            self.dropped_particles += 1;
        }

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

    #[cfg_attr(feature = "debug", ro_next_debug::profile("update particles"))]
    pub fn update(&mut self, delta_time: f32) {
        // Particles that end this frame report their impact sound, which is
        // played after the update so no engine call happens inside the loop.
        let mut impact_sounds: Vec<(SoundEffectKey, Point3<f32>)> = Vec::new();

        self.particles.retain_mut(|particle| {
            let alive = particle.update(delta_time);

            if !alive && let Some(sound) = particle.impact_sound() {
                impact_sounds.push(sound);
            }

            alive
        });

        if let Some(audio_engine) = self.audio_engine.as_ref() {
            for (sound, position) in impact_sounds {
                audio_engine.play_spatial_sound_effect(sound, position, ARROW_HIT_SOUND_RANGE);
            }
        }
    }

    #[cfg_attr(feature = "debug", ro_next_debug::profile("render particles"))]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
        scaling: Scaling,
        entities: &[Entity],
    ) {
        let mut rendered = 0;
        let mut culled = 0;

        for particle in &self.particles {
            if self.cull_distance > 0.0 && camera.distance_to(particle.culling_position()) > self.cull_distance {
                culled += 1;
                continue;
            }

            particle.render(renderer, camera, window_size);
            rendered += 1;
        }

        self.rendered_in_last_frame.set(rendered);
        self.culled_in_last_frame.set(culled);

        entities
            .iter()
            .filter_map(|entity| self.quest_icons.get(&entity.get_entity_id()))
            .for_each(|quest_icon| quest_icon.render(renderer, camera, window_size, scaling.get_factor()));
    }
}

#[cfg(test)]
mod tests {
    use super::{BASE_DAMAGE_TEXT_PIXELS, damage_magnitude_scale, damage_text_pixels, particles_to_evict};

    #[test]
    fn particle_budget_evicts_only_when_full() {
        assert_eq!(particles_to_evict(0, 300), 0);
        assert_eq!(particles_to_evict(299, 300), 0);
        assert_eq!(particles_to_evict(300, 300), 1);
        assert_eq!(particles_to_evict(10_000, 300), 1);
    }

    #[test]
    fn zero_particle_budget_means_unlimited() {
        assert_eq!(particles_to_evict(usize::MAX / 2, 0), 0);
        assert_eq!(particles_to_evict(0, 0), 0);
    }

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

/// Loaded sprite/action pair of the classic arrow projectile.
pub struct ArrowAssets {
    pub sprite: Arc<Sprite>,
    /// Action file of the sprite pair. A candidate only counts when both files
    /// load, so it is kept for projectile animation; the renderer draws the
    /// sprite's first frame today.
    #[allow(dead_code)]
    pub actions: Arc<Actions>,
}

#[cfg(test)]
mod damage_fountain_tests {
    use cgmath::{InnerSpace, Vector3};
    use ragnarok_packets::Direction;

    use super::{DAMAGE_NUMBER_DRIFT_SPEED, damage_number_drift};

    /// The drift direction follows the entity facing octant like the classic
    /// client (`MsgEffectDriftDirection`).
    #[test]
    fn drift_follows_the_entity_facing() {
        let cases = [
            (Direction::North, Vector3::new(-0.8, 0.0, 0.8)),
            (Direction::East, Vector3::new(-0.8, 0.0, -0.8)),
            (Direction::South, Vector3::new(0.8, 0.0, -0.8)),
            (Direction::West, Vector3::new(0.8, 0.0, 0.8)),
        ];

        for (direction, expected) in cases {
            let drift = damage_number_drift(direction);
            let expected = expected * DAMAGE_NUMBER_DRIFT_SPEED;
            assert!((drift - expected).magnitude() < 1e-5, "{direction:?}: {drift:?}");
        }
    }
}
