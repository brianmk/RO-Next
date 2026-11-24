use std::string::String;
use std::sync::Arc;
use std::time::Instant;

use arrayvec::ArrayVec;
use cgmath::{EuclideanSpace, Point3, Vector2, Vector3, VectorSpace};
use ragnarok_packets::{
    AccountId, AttackRange, CharacterId, CharacterInformation, ClientTick, Direction, DisappearanceReason, EntityId, JobId, Sex,
    SpriteChangeType, StatType, TilePosition, WorldPosition,
};
use ro_next_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use ro_next_debug::logging::Colorize;
use ro_next_interface::element::StateElement;
use ro_next_interface::window::{StateWindow, Window};
use ro_next_networking::EntityData;
use rust_state::{Path, RustState, VecItem};
#[cfg(feature = "debug")]
use smallvec::smallvec_inline;
#[cfg(feature = "debug")]
use wgpu::{BufferUsages, Device, Queue};

#[cfg(feature = "debug")]
use crate::graphics::reduce_vertices;
#[cfg(feature = "debug")]
use crate::graphics::{BindlessSupport, DebugRectangleInstruction};
use crate::graphics::{Color, EntityInstruction, ScreenPosition, ScreenSize};
use crate::loaders::{EntityPartFile, GameFileLoader, Imf};
#[cfg(feature = "debug")]
use crate::loaders::{GAT_TILE_SIZE, split_mesh_by_texture};
use crate::renderer::GameInterfaceRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
use crate::state::ClientState;
use crate::state::lifecycle::{ResourcePolicy, ResourceState};
use crate::state::theme::{InterfaceThemeType, WorldTheme};
use crate::world::{
    ActionEvent, AnimationData, AnimationLoopMode, AnimationState, Camera, FadeDirection, FadeState, JobIdentity, Library,
    MAX_WALK_PATH_SIZE, Map, PathFinder,
};
#[cfg(feature = "debug")]
use crate::world::{MarkerIdentifier, SubMesh};
#[cfg(feature = "debug")]
use crate::{Buffer, ModelVertex};

mod actor;
mod common;
mod npc;
mod player;

pub use self::actor::*;
pub use self::common::*;
pub use self::npc::*;
pub use self::player::*;

const SOUND_COOLDOWN_DURATION: u32 = 200;
const SPATIAL_SOUND_RANGE: f32 = 250.0;
const FADE_IN_DURATION_MS: u32 = 500;
/// Converts the encoded `c_level` of another player into a level that the aura
/// logic can use.
///
/// `c_level == 99` means the real level is at or above the aura level, `98`
/// means the level directly below it, lower values are the real level. The
/// aura threshold therefore matches only the `99` encoding (never `98`).
/// Jobs whose sprite is a GR2 model actor, so they cannot use the part-file
/// path (used by both the player and the NPC/NPC-like model path).
pub(super) const GR2_MODEL_ACTOR_JOB_IDS: [u16; 2] = [722, 1288];

fn aura_level_from_c_level(c_level: u16) -> usize {
    c_level as usize
}

/// Classic client `OPTION_CART` bits in the option/effect_state field.
const OPTION_CART_MASK: u32 = 0x0000_0008 | 0x0000_0080 | 0x0000_0100 | 0x0000_0200 | 0x0000_0400;

/// Classic client `OPTION_FALCON` bit in the option/effect_state field.
const OPTION_FALCON: u32 = 0x10;
/// Height above the owner's position at which the falcon is drawn.
///
/// The classic client draws the falcon a little above the hunter's head; the
/// previous value was too high (the bird floated over the head), so it was
/// lowered a bit.
const FALCON_HEIGHT: f32 = 26.0;
/// The raw falcon ACT flap loop is too slow; play it 50 % faster.
const FALCON_ANIMATION_SPEED: f32 = 1.5;

/// Grace period between the server reporting a death and the client starting
/// the death animation, so an attack animation that is already playing (the
/// killing blow) is not cut off instantly.
const DEATH_GRACE_MS: u32 = 250;

#[derive(Clone, RustState, StateElement)]
pub struct Movement {
    #[hidden_element]
    pub(crate) steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>,
    pub(crate) starting_timestamp: u32,
    #[cfg(feature = "debug")]
    #[hidden_element]
    pub pathing: Option<Pathing>,
}

impl Movement {
    pub fn new(steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>, starting_timestamp: u32) -> Self {
        Self {
            steps,
            starting_timestamp,
            #[cfg(feature = "debug")]
            pathing: None,
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Clone)]
pub struct Pathing {
    pub vertex_buffer: Arc<Buffer<ModelVertex>>,
    pub index_buffer: Arc<Buffer<u32>>,
    pub submeshes: Vec<SubMesh>,
}

#[derive(Copy, Clone)]
pub struct Step {
    pub(crate) arrival_position: TilePosition,
    pub(crate) arrival_timestamp: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EntityType {
    Hidden,
    Monster,
    Npc,
    Player,
    Warp,
    /// A pet (`BL_PET`, object type `0x7`). Pets reuse monster class ids for
    /// their sprite, so the object type of the entity-appear packet is what
    /// distinguishes them from a wild monster of the same job id.
    Pet,
    /// A homunculus (`BL_HOM`, object type `0x8`), job ids 6001..=6016 and
    /// 6048..=6052, rendered from `data\sprite\homun\`.
    Homunculus,
    /// A mercenary (`BL_MER`, object type `0x9`). Mercenaries reuse monster
    /// class ids and are rendered with the monster sprite folder.
    Mercenary,
}

impl EntityType {
    /// Maps the rAthena `clif_bl_type` object type byte of the entity-appear
    /// packets to an entity type.
    ///
    /// Source: `src/map/clif.cpp:346` (`clif_bl_type`). Only the companion
    /// types are mapped here: players, monsters and NPCs already classify
    /// correctly by job id, and the existing job-id fallback keeps NPCs whose
    /// view is a monster sprite working. Pets and mercenaries *reuse monster
    /// class ids*, so the object type (`0x7`/`0x9`) is the only way to tell
    /// them apart from a wild monster of the same class. Homunculi have their
    /// own job-id range, but the object type keeps them correct even for a
    /// custom server that reuses a different id.
    pub fn from_object_type(object_type: u8) -> Option<Self> {
        match object_type {
            // NPC_PET_TYPE.
            0x7 => Some(Self::Pet),
            // NPC_HOM_TYPE.
            0x8 => Some(Self::Homunculus),
            // NPC_MERSOL_TYPE.
            0x9 => Some(Self::Mercenary),
            // Everything else is classified by job id as before.
            _ => None,
        }
    }
}

/// Equipment appearance slots the entity layer keeps from the server.
///
/// The server sends these through `ZC_SPRITE_CHANGE` (`0x01D7`) and the
/// equipment acknowledgement packets. The weapon, shield and robe are each
/// rendered as their own animation layer on top of the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentLook {
    Weapon,
    Shield,
    Robe,
}

/// Sprite layers that have to be reloaded after an appearance change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SpriteChangeOutcome {
    /// Body/head/headgear layer changed: reload the body animation data.
    pub body: bool,
    /// Equipped weapon changed: reload the weapon animation layer.
    pub weapon: bool,
    /// Equipped shield changed: reload the shield animation layer.
    pub shield: bool,
    /// Equipped robe changed: reload the robe animation layer.
    pub robe: bool,
}

impl From<JobId> for EntityType {
    fn from(job_id: JobId) -> Self {
        // Entity classification is data-driven (`jobs.ron` `entity_types`): the
        // registry knows every player job and the special warp/hidden NPC ids
        // (`JT_WARPNPC` 45, `JT_HIDDEN_NPC` 111, `JT_HIDDEN_WARP_NPC` 139), and
        // the client job-id ranges are the documented fallback for NPC/monster
        // ids and unknown values.
        crate::world::jobs::entity_type(job_id)
    }
}

#[derive(Copy, Clone, Default)]
pub struct SoundState {
    pub(crate) previous_key: Option<SoundEffectKey>,
    pub(crate) last_played_at: Option<ClientTick>,
}

impl SoundState {
    pub fn update(
        &mut self,
        audio_engine: &AudioEngine<GameFileLoader>,
        position: Point3<f32>,
        sound_effect_key: SoundEffectKey,
        client_tick: ClientTick,
    ) {
        let should_play = if Some(sound_effect_key) == self.previous_key
            && let Some(last_tick) = self.last_played_at
        {
            (client_tick.0.wrapping_sub(last_tick.0)) >= SOUND_COOLDOWN_DURATION
        } else {
            true
        };

        if should_play {
            audio_engine.play_spatial_sound_effect(sound_effect_key, position, SPATIAL_SOUND_RANGE);
            self.last_played_at = Some(client_tick);
            self.previous_key = Some(sound_effect_key);
        }
    }
}

/// A temporary tint of an entity's sprites, used by hard-coded skill effects
/// (for example the yellow `EF_PORTAL5` flash of Zen).
#[derive(Debug, Clone, Copy)]
pub struct EntityFlash {
    pub(crate) color: Color,
    pub(crate) start_tick: ClientTick,
    pub(crate) hold_ms: u32,
    pub(crate) fade_ms: u32,
    pub(crate) additive: bool,
}

impl EntityFlash {
    /// Weight of the flash color at `client_tick`, `0.0` when it is over.
    fn weight(&self, client_tick: ClientTick) -> f32 {
        let elapsed = client_tick.0.saturating_sub(self.start_tick.0);

        if elapsed >= self.hold_ms + self.fade_ms {
            return 0.0;
        }

        if elapsed < self.hold_ms {
            1.0
        } else {
            let fade_elapsed = elapsed - self.hold_ms;
            1.0 - fade_elapsed as f32 / self.fade_ms as f32
        }
    }
}

/// Horizontal squash of the body (official landing flag 0x40).
#[derive(Debug, Clone, Copy)]
pub struct BodySquash {
    pub(crate) start_tick: ClientTick,
    pub(crate) duration_ms: u32,
    pub(crate) amount: f32,
}

impl BodySquash {
    /// Vertical scale factor at `client_tick` (1.0 when inactive).
    fn y_scale(&self, client_tick: ClientTick) -> f32 {
        let elapsed = client_tick.0.saturating_sub(self.start_tick.0);

        if elapsed >= self.duration_ms {
            return 1.0;
        }

        let weight = 1.0 - elapsed as f32 / self.duration_ms as f32;
        1.0 - self.amount * weight
    }
}

/// Flat silhouette copies of the body (official sprite outline).
#[derive(Debug, Clone, Copy)]
pub struct BodyOutline {
    pub(crate) color: Color,
    pub(crate) start_tick: ClientTick,
    pub(crate) duration_ms: u32,
}

impl BodyOutline {
    fn active(&self, client_tick: ClientTick) -> bool {
        client_tick.0.saturating_sub(self.start_tick.0) < self.duration_ms
    }
}

/// Vertical sine wave of the body sprites (official `render.flags` bit 0x8).
#[derive(Debug, Clone, Copy)]
pub struct BodyWave {
    pub(crate) start_tick: ClientTick,
    pub(crate) duration_ms: u32,
    pub(crate) amplitude: f32,
}

impl BodyWave {
    /// Vertical offset of the body at `client_tick` (`0.0` when the wave is
    /// over). One full sine period per 500 ms, like the official wave phase.
    fn offset(&self, client_tick: ClientTick) -> f32 {
        let elapsed = client_tick.0.saturating_sub(self.start_tick.0);

        if elapsed >= self.duration_ms {
            return 0.0;
        }

        let phase = elapsed as f32 * std::f32::consts::TAU / 500.0;
        phase.sin() * self.amplitude
    }
}

/// Blends `base` towards `target` by `weight`.
fn blend_color(base: Color, target: Color, weight: f32) -> Color {
    let weight = weight.clamp(0.0, 1.0);

    Color::rgba(
        base.red * (1.0 - weight) + target.red * weight,
        base.green * (1.0 - weight) + target.green * weight,
        base.blue * (1.0 - weight) + target.blue * weight,
        base.alpha * (1.0 - weight) + target.alpha * weight,
    )
}

fn option_hides_entity(effect_state: u32) -> bool {
    effect_state & OPTION_HIDDEN_MASK != 0
}
