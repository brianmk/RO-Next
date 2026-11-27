use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
use ragnarok_packets::{EntityId, QuestEffectPacket};
use ro_next_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use ro_next_debug::logging::Colorize;
use ro_next_interface::application::Clip;

use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{GameFileLoader, ImageType, Scaling, Sprite, TextureLoader};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Actions, Camera};
use crate::{Entity, Map};

/// Hearing range of the arrow impact sound in world units.
const ARROW_HIT_SOUND_RANGE: f32 = 250.0;

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

    /// Draws the particle. `entities` is the current entity list so a number
    /// can follow its owner (`CMsgEffect::FollowOwner`, `MsgEffect.cpp:1308`).
    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize, entities: &[Entity]);

    /// World position used for distance culling.
    fn culling_position(&self) -> Point3<f32>;

    /// Owner of a cumulative total number (official type 22).
    ///
    /// `SpawnDamageDigits` removes the previous type-22 effect of the same
    /// actor before spawning the new total (`RenderObject.cpp:4797-4806`), so
    /// [`ParticleHolder::spawn_particle`] drops the old one first.
    fn total_owner(&self) -> Option<EntityId> {
        None
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

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize, _entities: &[Entity]) {
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

    /// Enables impact sounds.
    ///
    /// The bow auto-attack impact wave (`_hit_arrow.wav`) is not played here:
    /// it is the official damage-message hit wave
    /// (`Client::play_auto_attack_hit_feedback`).
    pub fn install_audio(&mut self, audio_engine: Arc<AudioEngine<GameFileLoader>>) {
        self.audio_engine = Some(audio_engine);
    }

    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        if let Some(entity_id) = particle.total_owner() {
            self.particles.retain(|existing| existing.total_owner() != Some(entity_id));
        }

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

            particle.render(renderer, camera, window_size, entities);
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
    use super::particles_to_evict;

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
