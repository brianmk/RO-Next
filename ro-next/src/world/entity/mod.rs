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

const SOUND_COOLDOWN_DURATION: u32 = 200;
const SPATIAL_SOUND_RANGE: f32 = 250.0;
const FADE_IN_DURATION_MS: u32 = 500;
/// Converts the encoded `c_level` of another player into a level that the aura
/// logic can use.
///
/// `c_level == 99` means the real level is at or above the aura level, `98`
/// means the level directly below it, lower values are the real level. The
/// aura threshold therefore matches only the `99` encoding (never `98`).
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
    steps: ArrayVec<Step, MAX_WALK_PATH_SIZE>,
    starting_timestamp: u32,
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
    arrival_position: TilePosition,
    arrival_timestamp: u32,
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
    previous_key: Option<SoundEffectKey>,
    last_played_at: Option<ClientTick>,
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

#[derive(Clone, RustState, StateElement)]
pub struct Common {
    pub entity_id: EntityId,
    /// Character id carried by the entity-appear packet (`0` for non-players).
    #[hidden_element]
    pub character_id: CharacterId,
    pub job_id: JobId,
    pub health_points: usize,
    pub maximum_health_points: usize,
    pub movement_speed: usize,
    pub direction: Direction,
    pub head_direction: usize,
    pub sex: Sex,

    #[hidden_element]
    pub entity_type: EntityType,
    /// Hair color palette index (`head_palette` in the network packets).
    #[hidden_element]
    pub head_palette: u16,
    /// Clothes color palette index (`body_palette` in the network packets).
    #[hidden_element]
    pub body_palette: u16,
    /// Lower headgear view id (`accessory` in the network packets).
    #[hidden_element]
    pub accessory: u16,
    /// Upper headgear view id (`accessory2` in the network packets).
    #[hidden_element]
    pub accessory2: u16,
    /// Middle headgear view id (`accessory3` in the network packets).
    #[hidden_element]
    pub accessory3: u16,
    /// Hair style id (`head` in the entity-appear packets and the character
    /// information row). The official client always renders the hair sprite,
    /// so it must be carried for every player entity, not only the local one.
    #[hidden_element]
    pub hair_id: Option<usize>,
    /// Option bit mask (`effectState`): falcon, cart, riding, hide/cloak,
    /// costumes, ... (server `enum e_option`, rAthena `status.hpp`).
    ///
    /// `OPTION_FALCON` (0x10) means the player currently has the hunter falcon.
    #[hidden_element]
    pub effect_state: u32,
    /// Client-visible level (`c_level`) from the entity-appear packets.
    ///
    /// The server encodes levels >= the aura level as 99 and the level below it
    /// as 98; lower levels are sent as the real level.
    #[hidden_element]
    pub c_level: u16,
    /// `bodyState` (`opt1`): freeze/stone/stun/sleep/burning/... visual state.
    #[hidden_element]
    pub body_state: u16,
    /// `healthState` (`opt2`): poison/curse/silence/confusion/blind/... state.
    #[hidden_element]
    pub health_state: u16,
    /// Equipped weapon value (`weapon` in the entity-appear packets), `0` if
    /// unarmed.
    ///
    /// The server sends the rAthena weapon type (`W_*`, e.g. `W_BOW = 11`); the
    /// value can also be an item view id (population shells / custom servers).
    /// The weapon module resolves both forms to a sprite family and projectile;
    /// see `crate::world::weapon`.
    #[hidden_element]
    pub weapon: u32,
    /// Shield value (`shield` in the entity-appear packets), `0` if none.
    ///
    /// The server sends the shield item id; [`super::shield`] resolves it to
    /// the item view id and the sprite candidates of the equipped shield.
    #[hidden_element]
    pub shield: u32,
    /// Robe view id (`robe` in the entity-appear packets), `0` if none.
    ///
    /// See [`super::robe`] for the sprite candidates and the animated opacity
    /// of transparent robes.
    #[hidden_element]
    pub robe: u16,
    /// Animation of the hunter falcon, loaded lazily when the entity actually
    /// has the falcon option (`effect_state & OPTION_FALCON`).
    #[hidden_element]
    falcon_animation: Option<Arc<AnimationData>>,
    /// Independent animation state of the falcon (perched/flapping loop).
    #[hidden_element]
    falcon_animation_state: AnimationState,
    /// Merchant cart animation, loaded lazily while the entity has a cart
    /// option (`effect_state & OPTION_CART_MASK`).
    #[hidden_element]
    cart_animation: Option<Arc<AnimationData>>,
    /// Per-job `.imf` head/body draw order, loaded lazily for players.
    #[hidden_element]
    imf: Option<Arc<Imf>>,
    /// The server's `EFST_PUSH_CART` status is active. Some 2019 servers do
    /// not put the cart option into the entity `effect_state`, so the status
    /// is an independent visibility source for the cart.
    cart_status_active: bool,
    /// Cart type from `EFST_PUSH_CART` (673) or the `OPTION_CART*` bits. 1 is
    /// the basic pushcart; 10..13 are named carts.
    cart_kind: i32,
    /// Persistent sprite tint of a status effect (Adrenaline Rush and Fury are
    /// the classic red tints). `None` means no status tint.
    status_tint: Option<Color>,
    status_weapon_blink: bool,
    #[hidden_element]
    body_wave: Option<BodyWave>,
    #[hidden_element]
    body_outline: Option<BodyOutline>,
    #[hidden_element]
    body_squash: Option<BodySquash>,
    /// Draw the body layers with an additive blend (official
    /// `EF_REDBODY`/`render.flags & 1`).
    status_body_additive: bool,
    /// Draw the equipped weapon layer with an additive blend (official
    /// `EF_AURABLADE2`/`render.flags & 2`).
    status_weapon_additive: bool,
    /// Body alpha multiplier from a status visual (`BodyFade`), `None` = 1.0.
    status_alpha: Option<f32>,
    /// Equipped weapon animation, drawn as its own layer on top of the body
    /// animation using the body animation state (same ACT action layout).
    #[hidden_element]
    weapon_animation: Option<Arc<AnimationData>>,
    /// Equipped shield animation, drawn in front of the body/head layers.
    #[hidden_element]
    shield_animation: Option<Arc<AnimationData>>,
    /// Equipped robe animation, drawn as the top layer over the body.
    #[hidden_element]
    robe_animation: Option<Arc<AnimationData>>,
    pub active_movement: Option<Movement>,
    pub animation_data: Option<Arc<AnimationData>>,
    /// Pending death: the tick at which the server reported the death. The
    /// entity keeps its current (attack) animation for a short grace period
    /// before the death animation starts, so the killing blow is visible.
    #[hidden_element]
    pending_death: Option<u32>,
    pub tile_position: TilePosition,
    pub world_position: Point3<f32>,
    pub scale: f32,
    /// Job/baby base scale, restored when `EFST_CHANGE_SIZE` ends.
    #[hidden_element]
    base_scale: f32,
    #[hidden_element]
    details: ResourceState<String>,
    /// Id of the guild the entity is in, `0` if it is not in a guild.
    ///
    /// The entity-appear packets carry the guild id, the guild *name* is only
    /// known after the server answered a name request (see
    /// [`Entity::set_details`]).
    #[hidden_element]
    guild_id: u32,
    /// The name of the guild the entity is in.
    ///
    /// `None` for entities that are not in a guild. The entity-appear packets
    /// only carry the guild id, so the name is only known after the server
    /// answered a name request (see [`Entity::set_details`]).
    #[hidden_element]
    guild_name: Option<String>,
    /// Cached `<guild name>` label rendered under the character name.
    ///
    /// Composing it once when the guild name is set avoids a `String`
    /// allocation per entity per frame in crowded towns.
    #[hidden_element]
    guild_label: Option<String>,
    /// Title of an open vending shop of this player (`ZC_STORE_ENTRY`, 0x0131,
    /// `clif.cpp:7558`). `None` when the player is not vending. Used by the
    /// character context menu to offer browsing the shop.
    #[hidden_element]
    vending_shop_name: Option<String>,
    #[hidden_element]
    animation_state: AnimationState,
    stopped_moving: bool,
    #[hidden_element]
    sound_state: SoundState,
    #[hidden_element]
    fade_state: FadeState,
    /// Temporary sprite tint of a hard-coded skill effect (`None` when the
    /// entity is not flashing).
    #[hidden_element]
    body_flash: Option<EntityFlash>,
}

#[allow(clippy::too_many_arguments)]
fn get_entity_part_files(
    library: &Library,
    entity_type: EntityType,
    job_id: JobId,
    sex: Sex,
    head: Option<usize>,
    head_palette: u16,
    body_palette: u16,
    accessory: u16,
    accessory2: u16,
    accessory3: u16,
    effect_state: u32,
) -> Vec<EntityPartFile> {
    let head_id = crate::world::jobs::resolve_head_id(sex, head);

    match entity_type {
        EntityType::Player => {
            // The part paths come from the job data (`jobs.ron` path templates)
            // and the registry resolves the verified sprite/palette stems; no
            // job-specific path or stem is hardcoded here.
            let appearance_job = crate::world::jobs::appearance_job(job_id, effect_state);
            let body = crate::world::jobs::body_part_path(appearance_job, sex);
            let head_sprite = crate::world::jobs::head_part_path(head_id, sex);

            // Hair and clothes colors are external `.pal` files for players. A
            // missing palette file is handled by the sprite loader, which then
            // falls back to the palette embedded in the sprite.
            // The palette file is paired with the rendered hair sprite, i.e. it
            // uses the same (translated) hair id as the sprite path.
            let mut entity_part_files = vec![
                EntityPartFile::with_palette(body, crate::world::jobs::body_palette_path(appearance_job, sex, body_palette)),
                EntityPartFile::with_palette(
                    head_sprite,
                    Some(crate::world::jobs::head_palette_path(head_id, sex, head_palette)),
                ),
            ];

            // Headgear accessories are layered over the head sprite. The network
            // packet field names follow the official client: `accessory` is the
            // lower, `accessory2` the upper and `accessory3` the middle headgear.
            // The official client draws them in the order middle, upper, lower,
            // skipping entries that would render the same accessory twice.
            let mut push_accessory = |view_id: u16| {
                let Some(sprite_path) = library.accessory_sprite_path(view_id, sex) else {
                    #[cfg(feature = "debug")]
                    if view_id != 0 {
                        ro_next_debug::logging::print_debug!(
                            "[{}] no accessory sprite for view id {} (sex {:?}), skipping",
                            "error".red(),
                            view_id,
                            sex
                        );
                    }
                    return;
                };

                entity_part_files.push(EntityPartFile::new(sprite_path));
            };

            if accessory3 != 0 && accessory3 != accessory {
                push_accessory(accessory3);
            }

            if accessory2 != 0 && accessory2 != accessory && accessory2 != accessory3 {
                push_accessory(accessory2);
            }

            if accessory != 0 {
                push_accessory(accessory);
            }

            entity_part_files
        }
        EntityType::Npc => vec![EntityPartFile::new(format!("npc\\{}", library.get::<JobIdentity>(job_id)))],
        // Monster sprite resolution is handled by the library: it falls back from the
        // exact `몬스터\<identity>` path to a verified alias / npc-folder / generic
        // sprite instead of the black `missing.spr` placeholder (the data set is
        // partially trimmed, see `world/library/monster_sprite.rs`).
        EntityType::Monster => vec![EntityPartFile::new(library.monster_sprite_path(job_id).to_string())],
        // Pets and mercenaries reuse monster class ids for their sprite; the
        // object type of the appear packet is what tells them apart from a wild
        // monster. Their name comes from the appear packet and is set in
        // `Common::new`.
        EntityType::Pet | EntityType::Mercenary => vec![EntityPartFile::new(library.monster_sprite_path(job_id).to_string())],
        // Homunculi have their own sprite folder (`data\sprite\homun\`), the
        // job id -> file stem mapping is data from the client job table.
        EntityType::Homunculus => vec![EntityPartFile::new(crate::world::jobs::homunculus_sprite_path(job_id))],
        // Warps and hidden NPCs have no sprite in the game files. The kRO client draws
        // warps as a dedicated hard-coded effect (see `WarpPortal`) and does not draw
        // hidden NPCs at all. Requesting a sprite path for them would end up loading
        // the black `missing.spr` placeholder, which is what used to show up as an
        // opaque black silhouette at every map connection.
        EntityType::Warp | EntityType::Hidden => Vec::new(),
    }
}

/// Composes the sprite part files of a character shown in a menu (the
/// character selection window) from its character-list entry.
///
/// This uses exactly the same part composition as a spawned [`Player`] entity,
/// so the preview shows the real body, hair, hair/clothes palette and the
/// middle/upper/lower headgear layers in the same order as the world sprite and
/// the equipment paper doll.
pub fn character_preview_part_files(library: &Library, character_information: &CharacterInformation) -> Vec<EntityPartFile> {
    get_entity_part_files(
        library,
        EntityType::Player,
        character_information.job_id,
        character_information.sex,
        Some(character_information.head as usize),
        character_information.head_palette as u16,
        character_information.body_palette as u16,
        character_information.accessory as u16,
        character_information.accessory2 as u16,
        character_information.accessory3 as u16,
        0,
    )
}

/// Option bits (rAthena `enum e_option`) that make the entity invisible to
/// other clients: hide, cloak, invisible and chase-walk.
const OPTION_HIDDEN_MASK: u32 = 0x0000_0002 | 0x0000_0004 | 0x0000_0040 | 0x0000_4000;

/// A temporary tint of an entity's sprites, used by hard-coded skill effects
/// (for example the yellow `EF_PORTAL5` flash of Zen).
#[derive(Debug, Clone, Copy)]
pub struct EntityFlash {
    color: Color,
    start_tick: ClientTick,
    hold_ms: u32,
    fade_ms: u32,
    additive: bool,
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
    start_tick: ClientTick,
    duration_ms: u32,
    amount: f32,
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
    color: Color,
    start_tick: ClientTick,
    duration_ms: u32,
}

impl BodyOutline {
    fn active(&self, client_tick: ClientTick) -> bool {
        client_tick.0.saturating_sub(self.start_tick.0) < self.duration_ms
    }
}

/// Vertical sine wave of the body sprites (official `render.flags` bit 0x8).
#[derive(Debug, Clone, Copy)]
pub struct BodyWave {
    start_tick: ClientTick,
    duration_ms: u32,
    amplitude: f32,
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

impl Common {
    pub fn new(
        _library: &Library,
        entity_data: &EntityData,
        tile_position: TilePosition,
        world_position: Point3<f32>,
        client_tick: ClientTick,
    ) -> Self {
        let entity_id = entity_data.entity_id;
        let job_id = entity_data.job_id;
        let head_direction = entity_data.head_direction;
        let direction = entity_data.position.direction;

        let movement_speed = entity_data.movement_speed as usize;
        let health_points = entity_data.health_points as usize;
        let maximum_health_points = entity_data.maximum_health_points as usize;
        let sex = entity_data.sex;
        let head_palette = entity_data.head_palette;
        let body_palette = entity_data.body_palette;
        let accessory = entity_data.accessory;
        let accessory2 = entity_data.accessory2;
        let accessory3 = entity_data.accessory3;
        let hair_id = Some(entity_data.head as usize);
        let weapon = entity_data.weapon;
        let shield = entity_data.shield;
        let robe = entity_data.robe;
        let effect_state = entity_data.effect_state;
        let c_level = entity_data.c_level;
        let body_state = entity_data.body_state;
        let health_state = entity_data.health_state;

        crate::logging::log_debug!(
            "entity spawn id={} job={} effect_state=0x{:X} c_level={} weapon={} shield={}",
            entity_data.entity_id.0,
            job_id.0,
            effect_state,
            c_level,
            weapon,
            shield
        );

        let active_movement = None;
        // `object_type` comes from the same entity-appear packet and is the only
        // way to distinguish pets/mercenaries from wild monsters; fall back to
        // the data-driven job id classification when it is missing/unknown.
        let entity_type = EntityType::from_object_type(entity_data.object_type).unwrap_or_else(|| job_id.into());

        let mut details = ResourceState::new(ResourcePolicy::ENTITY_DETAILS);
        // Pet/homunculus/mercenary appear packets carry the unit name, and the
        // server never answers a name request for them (they are not players or
        // NPCs), so seed the details/name label from the packet.
        if matches!(entity_type, EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary) && !entity_data.name.trim().is_empty() {
            details.mark_available(entity_data.name.trim().to_owned());
        }
        let guild_id = entity_data.guild_id;
        // Filled in later from the player details response (`0x0A30`).
        let guild_name = match entity_data.guild_name.trim() {
            "" => None,
            guild_name => Some(guild_name.to_owned()),
        };
        let guild_label = guild_name_label(entity_type, guild_name.as_deref());
        let animation_state = AnimationState::new(entity_type, client_tick);
        let scale = crate::world::jobs::scale(job_id);

        Self {
            tile_position,
            world_position,
            entity_id,
            character_id: entity_data.character_id,
            job_id,
            direction,
            head_direction,
            sex,
            head_palette,
            body_palette,
            accessory,
            accessory2,
            accessory3,
            hair_id,
            effect_state,
            c_level,
            body_state,
            health_state,
            weapon,
            shield,
            robe,
            falcon_animation: None,
            falcon_animation_state: AnimationState::new(EntityType::Npc, client_tick)
                .with_speed(FALCON_ANIMATION_SPEED)
                .with_loop_mode(AnimationLoopMode::PingPong),
            cart_animation: None,
            imf: None,
            cart_status_active: false,
            cart_kind: 0,
            status_tint: None,
            status_weapon_blink: false,
            body_wave: None,
            body_outline: None,
            body_squash: None,
            status_body_additive: false,
            status_weapon_additive: false,
            status_alpha: None,
            weapon_animation: None,
            shield_animation: None,
            robe_animation: None,
            active_movement,
            entity_type,
            movement_speed,
            health_points,
            maximum_health_points,
            animation_data: None,
            pending_death: None,
            details,
            guild_id,
            guild_name,
            guild_label,
            vending_shop_name: None,
            animation_state,
            stopped_moving: false,
            sound_state: SoundState::default(),
            // Entities that are hidden/cloaked/invisible through their option
            // bits must not be drawn at all, even while fading in.
            fade_state: match option_hides_entity(effect_state) {
                true => FadeState::from_alpha(0.0, FadeDirection::Out, client_tick, 0),
                false => FadeState::new(FADE_IN_DURATION_MS, client_tick),
            },
            body_flash: None,
            scale,
            base_scale: scale,
        }
    }

    /// Whether the entity currently has the hunter falcon option set.
    pub fn has_falcon(&self) -> bool {
        self.effect_state & OPTION_FALCON != 0
    }

    /// Update the option bit mask (`effectState`) and apply the visual
    /// consequences that the entity layer already understands: hidden,
    /// cloaked or invisible entities disappear/appear by fading.
    pub fn set_effect_state(&mut self, effect_state: u32, client_tick: ClientTick) {
        let was_hidden = option_hides_entity(self.effect_state);
        let hidden = option_hides_entity(effect_state);
        let old_appearance = crate::world::jobs::appearance_job(self.job_id, self.effect_state);
        let cart_changed = (self.effect_state ^ effect_state) & OPTION_CART_MASK != 0;
        self.effect_state = effect_state;
        let new_appearance = crate::world::jobs::appearance_job(self.job_id, effect_state);

        if cart_changed {
            // The cart type is also encoded in the option bits.
            self.cart_animation = None;
        }

        if old_appearance != new_appearance {
            // Outfits/mounts change the body sprite (and the weapon folder).
            // Drop the loaded layers so the asset retry loop reloads them with
            // the new appearance job.
            self.animation_data = None;
            self.weapon_animation = None;
            self.shield_animation = None;
            self.robe_animation = None;
            self.cart_animation = None;
        }

        if hidden != was_hidden {
            self.fade_state = match hidden {
                true => FadeState::from_alpha(0.0, FadeDirection::Out, client_tick, 0),
                false => FadeState::Opaque,
            };
        }
    }

    /// Starts a short body flash (hard-coded skill effect color flash, for
    /// example the yellow `EF_PORTAL5` flash of Zen). Calling this while a
    /// flash is running restarts it.
    pub fn start_body_flash(&mut self, color: Color, hold_ms: u32, fade_ms: u32, additive: bool, client_tick: ClientTick) {
        self.body_flash = Some(EntityFlash {
            color,
            start_tick: client_tick,
            hold_ms,
            fade_ms,
            additive,
        });
    }

    /// Vertical scale of the body sprites (official landing squash).
    fn body_squash_y(&self, client_tick: ClientTick) -> f32 {
        self.body_squash.map(|squash| squash.y_scale(client_tick)).unwrap_or(1.0)
    }

    /// Start a body squash (official landing flag 0x40).
    pub fn start_body_squash(&mut self, duration_ms: u32, amount: f32, client_tick: ClientTick) {
        self.body_squash = Some(BodySquash {
            start_tick: client_tick,
            duration_ms,
            amount,
        });
    }

    /// Whether the body sprites currently render with the additive blend
    /// (status blend or an additive body flash/overlay).
    fn body_additive(&self, client_tick: ClientTick) -> bool {
        self.status_body_additive
            || self
                .body_flash
                .is_some_and(|flash| flash.additive && flash.weight(client_tick) > 0.0)
    }

    /// Sprite tint of the entity at `client_tick`.
    ///
    /// This combines the persistent status effect colors understood by the
    /// entity layer (the red Fury glow, `OPT3_EXPLOSIONSPIRITS`) with a
    /// temporary body flash. White means "unchanged". The tint is a plain
    /// multiplication, so it works for both palette and RGBA sprites without
    /// touching the animation data.
    fn sprite_tint(&self, client_tick: ClientTick) -> Color {
        let mut tint = self.status_tint.unwrap_or(Color::WHITE);

        if let Some(flash) = self.body_flash {
            tint = blend_color(tint, flash.color, flash.weight(client_tick));
        }

        tint
    }

    /// Update the `bodyState` (`opt1`) visual state.
    pub fn set_body_state(&mut self, body_state: u16) {
        self.body_state = body_state;
    }

    /// Update the `healthState` (`opt2`) visual state.
    pub fn set_health_state(&mut self, health_state: u16) {
        self.health_state = health_state;
    }

    pub fn set_falcon_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.falcon_animation = Some(animation_data);
    }

    pub fn has_falcon_animation(&self) -> bool {
        self.falcon_animation.is_some()
    }

    /// Whether the entity currently has a merchant cart option set.
    pub fn has_cart(&self) -> bool {
        self.effect_state & OPTION_CART_MASK != 0 || self.cart_status_active
    }

    /// Updates the cart visibility from `EFST_PUSH_CART`. The option bit in
    /// `effect_state` is authoritative when present; this only adds the status
    /// as a fallback for servers that don't send the option bit.
    pub fn set_cart_status(&mut self, active: bool, kind: i32) {
        if self.cart_kind != kind {
            // A different cart type needs a different sprite; drop the loaded
            // animation so the async loader fetches the new one.
            self.cart_animation = None;
        }
        self.cart_status_active = active;
        self.cart_kind = kind.max(0);
    }

    /// Cart type to use: the status value when present, else the classic option
    /// bit index.
    pub fn cart_kind(&self) -> i32 {
        if self.cart_kind > 0 {
            return self.cart_kind;
        }
        if self.effect_state & 0x0000_0008 != 0 {
            1
        } else if self.effect_state & 0x0000_0080 != 0 {
            2
        } else if self.effect_state & 0x0000_0100 != 0 {
            3
        } else if self.effect_state & 0x0000_0200 != 0 {
            4
        } else if self.effect_state & 0x0000_0400 != 0 {
            5
        } else {
            1
        }
    }

    /// Ordered candidate sprite/ACT pairs of the current cart type; the first
    /// archive-verified one is loaded, with the basic pushcart as fallback.
    pub fn get_cart_part_candidates(&self) -> Vec<EntityPartFile> {
        let (sprite, act) = crate::world::jobs::cart_paths(self.job_id, self.cart_kind());
        vec![EntityPartFile::with_action(sprite, act), EntityPartFile::new("아이템\\mc_pushcart")]
    }

    /// Sets the persistent status sprite tint (red for Adrenaline Rush/Fury).
    pub fn set_status_tint(&mut self, tint: Option<Color>) {
        self.status_tint = tint;
    }

    /// Blink the equipped weapon layer (official `EF_LIGHTBLADE`).
    pub fn set_weapon_blink(&mut self, blink: bool) {
        self.status_weapon_blink = blink;
    }

    /// Start the flat-colour body outline (official `render.flags` 0x800).
    pub fn start_body_outline(&mut self, color: Color, duration_ms: u32, client_tick: ClientTick) {
        self.body_outline = Some(BodyOutline {
            color,
            start_tick: client_tick,
            duration_ms,
        });
    }

    /// Start the vertical body wave (official `render.flags` bit 0x8).
    pub fn start_body_wave(&mut self, duration_ms: u32, amplitude: f32, client_tick: ClientTick) {
        self.body_wave = Some(BodyWave {
            start_tick: client_tick,
            duration_ms,
            amplitude,
        });
    }

    /// Set the additive sprite blend of a status effect: `body` blends the
    /// body layers (official `render.flags & 1`), `weapon` the equipped weapon
    /// layer (official `render.flags & 2`). The tint is not affected.
    pub fn set_status_additive(&mut self, body: bool, weapon: bool) {
        self.status_body_additive = body;
        self.status_weapon_additive = weapon;
    }

    pub fn set_status_alpha(&mut self, alpha: Option<f32>) {
        self.status_alpha = alpha;
    }

    pub fn needs_cart_animation(&self) -> bool {
        self.has_cart() && self.cart_animation.is_none()
    }

    #[cfg(test)]
    pub fn has_imf(&self) -> bool {
        self.imf.is_some()
    }

    pub fn needs_imf(&self) -> bool {
        self.entity_type == EntityType::Player && self.imf.is_none() && crate::world::jobs::imf_path(self.job_id, self.sex).is_some()
    }

    pub fn set_imf(&mut self, imf: Arc<Imf>) {
        self.imf = Some(imf);
    }

    pub fn set_cart_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.cart_animation = Some(animation_data);
    }

    pub fn has_cart_animation(&self) -> bool {
        self.cart_animation.is_some()
    }

    /// Candidate sprite part files of the equipped weapon.
    ///
    /// The candidate construction (per-job variant, classic weapon layer,
    /// numeric view id, sprite family fallback order) lives in
    /// [`crate::world::weapon::part_candidates`]; the entity layer only asks
    /// for the ordered attempts.
    pub fn get_weapon_part_candidates(&self) -> Vec<EntityPartFile> {
        let appearance_job = crate::world::jobs::appearance_job(self.job_id, self.effect_state);
        if crate::world::jobs::is_outfit_job(appearance_job) {
            // Wedding/holiday outfits show no weapon layer.
            return Vec::new();
        }
        super::weapon::part_candidates(appearance_job, self.sex, self.weapon)
    }

    /// Candidate sprite part files of the equipped shield.
    ///
    /// The candidate construction (numeric view id, classic shield names) lives
    /// in [`crate::world::shield::part_candidates`].
    pub fn get_shield_part_candidates(&self) -> Vec<EntityPartFile> {
        let appearance_job = crate::world::jobs::appearance_job(self.job_id, self.effect_state);
        if crate::world::jobs::is_outfit_job(appearance_job) {
            // Wedding/holiday outfits show no shield.
            return Vec::new();
        }
        super::shield::part_candidates(appearance_job, self.sex, self.shield)
    }

    /// Candidate sprite/action pairs of the equipped robe.
    ///
    /// The robe sprite is shared by every job/sex and the action is per job;
    /// the sprite is drawn with the body palette (classic client
    /// `RenderRobe`). See [`crate::world::robe::part_candidates`].
    pub fn get_robe_part_candidates(&self) -> Vec<EntityPartFile> {
        let appearance_job = crate::world::jobs::appearance_job(self.job_id, self.effect_state);
        if appearance_job.0 != self.job_id.0 && crate::world::jobs::is_outfit_job(appearance_job) {
            // Outfits already include the garment shape.
            return Vec::new();
        }
        // Classic `RenderRobe` draws the robe with the sprite's own embedded
        // palette (and the body's blend mode), and this data set ships no
        // external robe palettes; the per-job/sex resolution above selects the
        // right animation. A data set with external robe palettes can pass them
        // to `part_candidates`.
        super::robe::part_candidates(appearance_job, self.sex, self.robe, None)
    }

    /// Equipped weapon value: a server `W_*` weapon type or an item view id.
    pub fn get_weapon(&self) -> u32 {
        self.weapon
    }

    #[cfg(test)]
    /// Shield item id of the entity, `0` if none.
    pub fn get_shield(&self) -> u32 {
        self.shield
    }

    #[cfg(test)]
    /// Robe view id of the entity, `0` if none.
    pub fn get_robe(&self) -> u16 {
        self.robe
    }

    pub fn set_weapon_animation(&mut self, animation_data: Arc<AnimationData>) {
        // Some trimmed archives contain weapon ACTs whose frames reference no
        // sprite (sprite_number == usize::MAX placeholder). Do not keep such an
        // animation: it would never draw anything.
        if super::weapon::animation_is_drawable(&animation_data) {
            self.weapon_animation = Some(animation_data);
        }
    }

    /// Applies an equipped weapon change (server `ZC_SPRITE_CHANGE` with
    /// `Weapon` / `Shield` / `Robe`).
    pub fn set_equipment_look(&mut self, value: u32, slot: EquipmentLook) {
        match slot {
            EquipmentLook::Weapon => {
                if self.weapon != value {
                    self.weapon = value;
                    self.weapon_animation = None;
                }
            }
            EquipmentLook::Shield => {
                if self.shield != value {
                    self.shield = value;
                    self.shield_animation = None;
                }
            }
            EquipmentLook::Robe => {
                let value = value as u16;
                if self.robe != value {
                    self.robe = value;
                    self.robe_animation = None;
                }
            }
        }
    }

    pub fn has_weapon_animation(&self) -> bool {
        self.weapon_animation.is_some()
    }

    /// Keeps a loaded shield animation.
    ///
    /// Trimmed data sets can ship shield ACTs without drawable frames; such an
    /// animation is rejected like a placeholder weapon animation.
    pub fn set_shield_animation(&mut self, animation_data: Arc<AnimationData>) {
        if super::weapon::animation_is_drawable(&animation_data) {
            self.shield_animation = Some(animation_data);
        }
    }

    pub fn has_shield_animation(&self) -> bool {
        self.shield_animation.is_some()
    }

    pub fn needs_shield_animation(&self) -> bool {
        self.shield != 0 && self.shield_animation.is_none() && !self.get_shield_part_candidates().is_empty()
    }

    pub fn set_robe_animation(&mut self, animation_data: Arc<AnimationData>) {
        if super::weapon::animation_is_drawable(&animation_data) {
            self.robe_animation = Some(animation_data);
        }
    }

    pub fn has_robe_animation(&self) -> bool {
        self.robe_animation.is_some()
    }

    pub fn needs_robe_animation(&self) -> bool {
        self.robe != 0 && self.robe_animation.is_none() && !self.get_robe_part_candidates().is_empty()
    }

    /// Current opacity multiplier of the equipped robe at `client_tick`.
    ///
    /// Implements the animated `RobeOpacity` window; fully opaque robes return
    /// `1.0` without any per-frame work.
    pub fn robe_opacity(&self, client_tick: ClientTick) -> f32 {
        match super::robe::profile(self.robe) {
            Some(profile) => profile.opacity_at(client_tick.0),
            None => 1.0,
        }
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<EntityPartFile> {
        get_entity_part_files(
            library,
            self.entity_type,
            self.job_id,
            self.sex,
            self.hair_id,
            self.head_palette,
            self.body_palette,
            self.accessory,
            self.accessory2,
            self.accessory3,
            self.effect_state,
        )
    }

    /// Id of the guild the entity is in, `0` if it is not in a guild.
    pub fn get_guild_id(&self) -> u32 {
        self.guild_id
    }

    pub fn get_guild_name(&self) -> Option<&str> {
        self.guild_name.as_deref()
    }

    pub fn set_guild_name(&mut self, guild_name: Option<String>) {
        self.guild_name = guild_name;
        self.guild_label = guild_name_label(self.entity_type, self.guild_name.as_deref());
    }

    /// Cached `<guild name>` label for the name plate, `None` when the entity
    /// is not a player in a guild.
    pub fn get_guild_label(&self) -> Option<&str> {
        self.guild_label.as_deref()
    }

    pub fn is_dead(&self) -> bool {
        self.animation_state.is_dead()
    }

    /// Marks the entity dead. If an attack/pickup/skill animation is currently
    /// playing, the death is queued for a short grace period so the animation
    /// that killed the entity can finish instead of being cut off instantly.
    pub fn set_dead(&mut self, client_tick: ClientTick) {
        let busy = self.animation_state.is_attack() || self.animation_state.is_pickup() || self.animation_state.is_skill();

        if busy && self.animation_data.is_some() {
            self.pending_death.get_or_insert(client_tick.0);
        } else {
            self.pending_death = None;
            self.animation_state.dead(self.entity_type, client_tick);
        }
    }

    pub fn is_death_animation_over(&self) -> bool {
        match self.animation_data.as_ref() {
            Some(animation_data) => self.is_dead() && animation_data.is_animation_over(&self.animation_state),
            None => false,
        }
    }

    pub fn is_fading(&self) -> bool {
        self.fade_state.is_fading()
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.update_movement(map, client_tick);
        self.animation_state.update(client_tick);

        if self.fade_state.is_fading() && self.fade_state.is_done_fading_in(client_tick) {
            self.fade_state = FadeState::Opaque;
        }

        // The hunter falcon has its own animation (perched/flapping loop) and is
        // drawn above the owner. Keep its animation running independently from the
        // owner's body animation.
        if self.falcon_animation.is_some() {
            // The falcon flaps in a continuous loop. Do not reset the state when
            // the (per-direction) ACT duration is over: the frame selection already
            // wraps modulo the frame count, and resetting here causes a one-frame
            // freeze/duplicate frame at the loop point (visible as a stutter).
            self.falcon_animation_state.update(client_tick);
        }

        if let Some(animation_data) = self.animation_data.as_ref() {
            if animation_data.is_animation_over(&self.animation_state)
                && (self.animation_state.is_attack() || self.animation_state.is_pickup() || self.animation_state.is_skill())
            {
                self.animation_state.idle(self.entity_type, client_tick);
            }

            // Start a queued death once the current animation finished or the
            // grace period elapsed (cap: no stuck "dying" entities).
            if let Some(death_tick) = self.pending_death {
                let grace_over = client_tick.0.saturating_sub(death_tick) >= DEATH_GRACE_MS;
                let animation_busy =
                    self.animation_state.is_attack() || self.animation_state.is_pickup() || self.animation_state.is_skill();
                let animation_finished = !animation_busy || animation_data.is_animation_over(&self.animation_state);

                if grace_over || animation_finished {
                    self.pending_death = None;
                    self.animation_state.dead(self.entity_type, client_tick);
                }
            }

            let frame = animation_data.get_frame(&self.animation_state, camera, self.direction);

            match frame.event {
                Some(ActionEvent::Sound { key }) => {
                    self.sound_state.update(audio_engine, self.world_position, key, client_tick);
                }
                Some(ActionEvent::Attack) => {
                    // TODO: NHA What do we need to do at this event? Other
                    //       clients are playing the attackers weapon attack
                    //       sound using this event.
                }
                None | Some(ActionEvent::Unknown) => { /* Nothing to do */ }
            }
        }
    }

    fn update_movement(&mut self, map: &Map, client_tick: ClientTick) {
        self.stopped_moving = false;

        if let Some(active_movement) = self.active_movement.take() {
            let last_step = active_movement.steps.last().unwrap();

            if client_tick.0 > last_step.arrival_timestamp {
                self.set_position(map, last_step.arrival_position, client_tick);
                self.stopped_moving = true;
            } else {
                let mut last_step_index = 0;
                while active_movement.steps[last_step_index + 1].arrival_timestamp < client_tick.0 {
                    last_step_index += 1;
                }

                let last_step = active_movement.steps[last_step_index];
                let next_step = active_movement.steps[last_step_index + 1];

                self.tile_position = next_step.arrival_position;

                let last_step_position = Vector2::new(last_step.arrival_position.x as isize, last_step.arrival_position.y as isize);
                let next_step_position = Vector2::new(next_step.arrival_position.x as isize, next_step.arrival_position.y as isize);

                let array = last_step_position - next_step_position;
                let array: &[isize; 2] = array.as_ref();
                self.direction = (*array).try_into().unwrap();

                let Some(last_step_position) = map.get_world_position(last_step.arrival_position) else {
                    self.active_movement = active_movement.into();
                    return;
                };
                let Some(next_step_position) = map.get_world_position(next_step.arrival_position) else {
                    self.active_movement = active_movement.into();
                    return;
                };

                let clamped_tick = u32::max(last_step.arrival_timestamp, client_tick.0);
                let total = next_step.arrival_timestamp - last_step.arrival_timestamp;
                let offset = clamped_tick - last_step.arrival_timestamp;

                let movement_elapsed = (1.0 / total as f32) * offset as f32;
                let position = last_step_position.to_vec().lerp(next_step_position.to_vec(), movement_elapsed);

                self.world_position = Point3::from_vec(position);
                self.active_movement = active_movement.into();
            }
        }
    }

    fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
        let Some(world_position) = map.get_world_position(position) else {
            #[cfg(feature = "debug")]
            ro_next_debug::logging::print_debug!("[{}] entity position is out of map bounds", "error".red());
            return;
        };

        self.tile_position = position;
        self.world_position = world_position;
        self.active_movement = None;
        self.animation_state.idle(self.entity_type, client_tick);
    }

    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        cell_overrides: &[(TilePosition, bool)],
        start: TilePosition,
        goal: TilePosition,
        starting_timestamp: ClientTick,
    ) {
        let traversable = crate::world::TraversableMap::new(map, cell_overrides);
        if let Some(path) = path_finder.find_walkable_path(&traversable, start, goal) {
            if path.len() <= 1 {
                return;
            }

            let mut last_timestamp = starting_timestamp.0;
            let mut last_position: Option<TilePosition> = None;

            let steps: ArrayVec<Step, MAX_WALK_PATH_SIZE> = path
                .iter()
                .map(|&step| {
                    if let Some(position) = last_position {
                        const DIAGONAL_MULTIPLIER: f32 = 1.4;

                        let speed = match position.x == step.x || position.y == step.y {
                            // `true` means we are moving orthogonally
                            true => self.movement_speed as u32,
                            // `false` means we are moving diagonally
                            false => (self.movement_speed as f32 * DIAGONAL_MULTIPLIER) as u32,
                        };

                        let arrival_position = step;
                        let arrival_timestamp = last_timestamp + speed;

                        last_timestamp = arrival_timestamp;
                        last_position = Some(arrival_position);

                        Step {
                            arrival_position,
                            arrival_timestamp,
                        }
                    } else {
                        last_position = Some(start);

                        Step {
                            arrival_position: start,
                            arrival_timestamp: last_timestamp,
                        }
                    }
                })
                .collect();

            // If there is only a single step the player is already on the correct tile.
            if steps.len() > 1 {
                self.active_movement = Movement::new(steps, starting_timestamp.0).into();

                if !self.animation_state.is_walking() {
                    self.animation_state.walk(self.entity_type, self.movement_speed, starting_timestamp);
                }
            }
        }
    }

    #[cfg(feature = "debug")]
    fn pathing_texture_coordinates(steps: &[Step], step: Vector2<usize>, index: usize) -> ([Vector2<f32>; 4], i32) {
        if steps.len() - 1 == index {
            return (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                ],
                0,
            );
        }

        let arrival_position = steps[index + 1].arrival_position;
        let delta = Vector2::new(
            arrival_position.x as isize - step.x as isize,
            arrival_position.y as isize - step.y as isize,
        );

        match delta {
            Vector2 { x: 1, y: 0 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: -1, y: 0 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 0, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),
            Vector2 { x: 1, y: 1 } => (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                2,
            ),
            Vector2 { x: 1, y: -1 } => (
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                2,
            ),
            Vector2 { x: -1, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                2,
            ),
            _other => panic!("incorrent pathing"),
        }
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, bindless_support: BindlessSupport, map: &Map) {
        use crate::{Color, NativeModelVertex};

        const PATHING_MESH_OFFSET: f32 = 0.95;

        let mut pathing_native_vertices = Vec::new();

        let Some(active_movement) = self.active_movement.as_mut() else {
            return;
        };

        let mesh_color = match self.entity_type {
            EntityType::Player => Color::rgb_u8(25, 250, 225),
            EntityType::Npc => Color::rgb_u8(170, 250, 25),
            EntityType::Monster => Color::rgb_u8(250, 100, 25),
            _ => Color::WHITE,
        };

        for (index, Step { arrival_position, .. }) in active_movement.steps.iter().copied().enumerate() {
            let Some(tile) = map.get_tile(arrival_position) else {
                ro_next_debug::logging::print_debug!("[{}] movement is out of map bounds", "error".red());
                continue;
            };

            let offset = Vector2::new(
                arrival_position.x as f32 * GAT_TILE_SIZE,
                arrival_position.y as f32 * GAT_TILE_SIZE,
            );

            let first_position = Point3::new(offset.x, tile.southwest_corner_height + PATHING_MESH_OFFSET, offset.y);
            let second_position = Point3::new(
                offset.x + GAT_TILE_SIZE,
                tile.southeast_corner_height + PATHING_MESH_OFFSET,
                offset.y,
            );
            let third_position = Point3::new(
                offset.x,
                tile.northwest_corner_height + PATHING_MESH_OFFSET,
                offset.y + GAT_TILE_SIZE,
            );
            let fourth_position = Point3::new(
                offset.x + GAT_TILE_SIZE,
                tile.northeast_corner_height + PATHING_MESH_OFFSET,
                offset.y + GAT_TILE_SIZE,
            );

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(third_position, second_position, fourth_position);

            let (texture_coordinates, texture_index) = Self::pathing_texture_coordinates(
                &active_movement.steps,
                Vector2::new(arrival_position.x as usize, arrival_position.y as usize),
                index,
            );

            if let Some(first_normal) = first_normal {
                pathing_native_vertices.push(NativeModelVertex::new(
                    first_position,
                    first_normal,
                    texture_coordinates[0],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    second_position,
                    first_normal,
                    texture_coordinates[1],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    third_position,
                    first_normal,
                    texture_coordinates[2],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
            }

            if let Some(second_normal) = second_normal {
                pathing_native_vertices.push(NativeModelVertex::new(
                    third_position,
                    second_normal,
                    texture_coordinates[2],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    second_position,
                    second_normal,
                    texture_coordinates[1],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
                pathing_native_vertices.push(NativeModelVertex::new(
                    fourth_position,
                    second_normal,
                    texture_coordinates[3],
                    texture_index,
                    mesh_color,
                    0.0,
                    smallvec_inline![0; 3],
                ));
            }
        }

        let pathing_vertices = NativeModelVertex::convert_to_model_vertices(pathing_native_vertices, None);
        let (pathing_vertices, mut pathing_indices) = reduce_vertices(&pathing_vertices);

        let submeshes = match bindless_support {
            BindlessSupport::Full | BindlessSupport::Limited => {
                vec![SubMesh {
                    index_offset: 0,
                    index_count: pathing_indices.len() as u32,
                    base_vertex: 0,
                    texture_index: 0,
                    transparent: true,
                }]
            }
            BindlessSupport::None => split_mesh_by_texture(&pathing_vertices, &mut pathing_indices, None, None, None),
        };

        if let Some(pathing) = active_movement.pathing.as_mut() {
            pathing.vertex_buffer.write_exact(queue, pathing_vertices.as_slice());
            pathing.submeshes = submeshes;
        } else {
            let pathing_vertex_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing vertex buffer",
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
                &pathing_vertices,
            ));

            let pathing_index_buffer = Arc::new(Buffer::with_data(
                device,
                queue,
                "pathing index buffer",
                BufferUsages::INDEX | BufferUsages::COPY_DST,
                &pathing_indices,
            ));

            active_movement.pathing = Some(Pathing {
                vertex_buffer: pathing_vertex_buffer,
                index_buffer: pathing_index_buffer,
                submeshes,
            });
        }
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool, client_tick: ClientTick) {
        let alpha = self.fade_state.calculate_alpha(client_tick) * self.status_alpha.unwrap_or(1.0);
        let tint = self.sprite_tint(client_tick);

        let body_wave = self.body_wave.map(|wave| wave.offset(client_tick)).unwrap_or(0.0);
        let body_position = self.world_position + cgmath::Vector3::new(0.0, body_wave, 0.0);

        // Official `render.flags 0x80000`: an undead entity draws the dark green
        // body outline (option bit 0x10000, `GameModePackets.cpp:3010`).
        let undead_outline = (self.effect_state & 0x10000) != 0;
        let outline_active = self
            .body_outline
            .filter(|outline| outline.active(client_tick))
            .or(undead_outline.then_some(BodyOutline {
                color: Color::rgb_u8(0x05, 0x64, 0x05),
                start_tick: client_tick,
                duration_ms: u32::MAX,
            }));

        // Robe layer. The classic client draws the robe with the body palette
        // and the animated `RobeOpacity` alpha; robes whose per-motion
        // `DrawOnTop` entry says "behind" are drawn before the body, everything
        // else is the top layer (after the weapon/shield layers below).
        //
        // The decision comes from the official `2DLayerDir`/`_New_2DLayerDir`
        // Lua tables (`CCharacterRender::DrawOnTop` /
        // `CPc::IsDrawOnTop`), indexed by the body action and frame. When the
        // archives do not ship those tables the robe stays on top, which was
        // the previous behavior.
        let body_job = crate::world::jobs::appearance_job(self.job_id, self.effect_state);
        let (body_action, body_motion) = self
            .animation_data
            .as_ref()
            .and_then(|animation_data| animation_data.motion(&self.animation_state, camera, self.direction))
            .unwrap_or((0, 0));
        let robe_top_layer =
            crate::world::robe::draw_on_top(self.robe, self.sex, body_job, body_action, body_motion).unwrap_or_else(|| {
                crate::world::robe::profile(self.robe)
                    .map(|profile| profile.is_top_layer())
                    .unwrap_or(true)
            });
        let robe_alpha = alpha * self.robe_opacity(client_tick);

        if !robe_top_layer && let Some(robe_animation) = self.robe_animation.as_ref() {
            robe_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                body_position,
                &self.animation_state,
                self.direction,
                robe_alpha,
                tint,
                self.scale,
                self.imf.as_deref(),
                false,
                false,
                -0.03,
                1.0,
            );
        }

        if let (Some(outline), Some(animation_data)) = (outline_active, self.animation_data.as_ref()) {
            for step in 1..=3 {
                let scale = self.scale * (1.0 + step as f32 * 0.01);
                animation_data.render(
                    instructions,
                    camera,
                    add_to_picker,
                    self.entity_id,
                    body_position,
                    &self.animation_state,
                    self.direction,
                    alpha,
                    outline.color,
                    scale,
                    self.imf.as_deref(),
                    false,
                    true,
                    -0.02 - step as f32 * 0.005,
                    1.0,
                );
            }
        }

        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                body_position,
                &self.animation_state,
                self.direction,
                alpha,
                tint,
                self.scale,
                self.imf.as_deref(),
                self.body_additive(client_tick),
                false,
                0.0,
                self.body_squash_y(client_tick),
            );
        }

        // Equipped shield: drawn in front of the body and behind the weapon,
        // using the same body animation state/action/frame.
        if let Some(shield_animation) = self.shield_animation.as_ref() {
            shield_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                self.world_position,
                &self.animation_state,
                self.direction,
                alpha,
                tint,
                self.scale,
                None,
                false,
                false,
                0.01,
                1.0,
            );
        }

        // Equipped weapon: rendered as its own animation layer with the body
        // animation state/action/frame so the weapon follows idle/walk/attack.
        // `EF_LIGHTBLADE` (Magical Power) blinks it on alternate frames.
        let weapon_blink_off = self.status_weapon_blink && client_tick.0 % 2 == 1;
        if let Some(weapon_animation) = self.weapon_animation.as_ref().filter(|_| !weapon_blink_off) {
            weapon_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                self.world_position,
                &self.animation_state,
                self.direction,
                alpha,
                tint,
                self.scale,
                None,
                self.status_weapon_additive,
                false,
                0.0,
                1.0,
            );
        }

        // Robe top layer (the common case): drawn over the body and equipment
        // layers with the animated opacity of transparent robes.
        if robe_top_layer && let Some(robe_animation) = self.robe_animation.as_ref() {
            robe_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                body_position,
                &self.animation_state,
                self.direction,
                robe_alpha,
                tint,
                self.scale,
                self.imf.as_deref(),
                false,
                false,
                0.02,
                1.0,
            );
        }

        // Merchant pushcart: the official `mc_pushcart` sprite follows the body
        // animation state so it is visible on merchants with a cart option.
        if self.has_cart()
            && let Some(cart_animation) = self.cart_animation.as_ref()
        {
            cart_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                self.world_position,
                &self.animation_state,
                self.direction,
                alpha,
                tint,
                self.scale,
                None,
                false,
                false,
                0.0,
                1.0,
            );
        }

        // Falcon companion: drawn above the owner (classic client shows it perched
        // on / flying above the hunter). Only rendered while the option bit is set.
        if self.has_falcon()
            && let Some(falcon_animation) = self.falcon_animation.as_ref()
        {
            let falcon_position = self.world_position + Vector3::new(0.0, FALCON_HEIGHT * self.scale, 0.0);
            falcon_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                falcon_position,
                &self.falcon_animation_state,
                self.direction,
                alpha,
                tint,
                self.scale,
                None,
                false,
                false,
                0.0,
                1.0,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        if let Some(animation_data) = self.animation_data.as_ref() {
            animation_data.render_debug(
                instructions,
                camera,
                self.world_position,
                &self.animation_state,
                self.direction,
                Color::rgb_u8(255, 0, 0),
                Color::rgb_u8(0, 255, 0),
                self.scale,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        renderer.render_marker(camera, marker_identifier, self.world_position, hovered);
    }
}

#[derive(Clone, RustState, StateWindow)]
pub struct Player {
    common: Common,
    pub hair_id: usize,
    pub spell_points: usize,
    pub activity_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
    pub base_level: usize,
    pub job_level: usize,
    pub stat_points: u32,
    pub strength: i32,
    pub bonus_strength: i32,
    pub strength_stat_points_cost: u8,
    pub agility: i32,
    pub bonus_agility: i32,
    pub agility_stat_points_cost: u8,
    pub vitality: i32,
    pub bonus_vitality: i32,
    pub vitality_stat_points_cost: u8,
    pub intelligence: i32,
    pub bonus_intelligence: i32,
    pub intelligence_stat_points_cost: u8,
    pub dexterity: i32,
    pub bonus_dexterity: i32,
    pub dexterity_stat_points_cost: u8,
    pub luck: i32,
    pub bonus_luck: i32,
    pub luck_stat_points_cost: u8,
    /// Attack motion (server `SP_ASPD`), in milliseconds.
    pub attack_speed: u32,
    /// Current attack range (`ZC_ATTACK_RANGE`, 0x013A).
    pub attack_range: AttackRange,
    /// Weight percentage above which the weight should be shown in red
    /// (`ZC_NOTIFY_CRITICALWEIGHT`, 0x0ADE; rAthena
    /// `natural_heal_weight_rate`).
    pub critical_weight_percentage: u32,
    /// Merchant cart item count/weight (`ZC_NOTIFY_CARTITEM_COUNTINFO`, 0x0121
    /// and `SP_CARTINFO` through 0x00B0).
    pub cart_items: u16,
    pub cart_maximum_items: i16,
    pub cart_weight: u32,
    pub cart_maximum_weight: u32,
    pub skill_points: u32,
    /// Base attack including the weapon attack (server `SP_ATK1`).
    pub attack: u32,
    /// Weapon refine attack bonus / damage variance (server `SP_ATK2`).
    pub bonus_attack: u32,
    /// Minimum magic attack (server `SP_MATK2`).
    pub magic_attack_min: u32,
    /// Maximum magic attack (server `SP_MATK1`).
    pub magic_attack_max: u32,
    /// Equipment defense (server `SP_DEF1`).
    pub defense: u32,
    /// Vitality based defense (server `SP_DEF2`).
    pub bonus_defense: u32,
    /// Equipment magic defense (server `SP_MDEF1`).
    pub magic_defense: u32,
    /// Intelligence based magic defense (server `SP_MDEF2`).
    pub bonus_magic_defense: u32,
    /// Hit rate (server `SP_HIT`).
    pub hit: u32,
    /// Flee rate (server `SP_FLEE1`).
    pub flee: u32,
    /// Perfect dodge (server `SP_FLEE2`, already divided by 10).
    pub perfect_dodge: u32,
    /// Critical rate (server `SP_CRITICAL`, already divided by 10).
    pub critical: u32,
    /// Carried weight (server `SP_WEIGHT`).
    pub weight: u32,
    /// Maximum carried weight (server `SP_MAXWEIGHT`).
    pub maximum_weight: u32,
    /// Base experience of the character (server `SP_BASEEXP`).
    pub base_experience: u64,
    /// Base experience needed for the next base level (server
    /// `SP_NEXTBASEEXP`).
    pub next_base_experience: u64,
    /// Job experience of the character (server `SP_JOBEXP`).
    pub job_experience: u64,
    /// Job experience needed for the next job level (server `SP_NEXTJOBEXP`).
    pub next_job_experience: u64,
}

impl Player {
    /// Current movement speed (`SP_SPEED` / `ZC_NOTIFY_MOVESPEED`), used by the
    /// status window's Speed line.
    pub fn movement_speed(&self) -> usize {
        self.common.movement_speed
    }

    /// This function creates the player entity free-floating in the
    /// "void". When a new map is loaded on map change, the server sends
    /// the correct position we need to position the player to.
    pub fn new(library: &Library, account_id: AccountId, character_information: &CharacterInformation, client_tick: ClientTick) -> Self {
        let hair_id = character_information.head as usize;
        let spell_points = character_information.spell_points as usize;
        let activity_points = 0;
        let maximum_spell_points = character_information.maximum_spell_points as usize;
        let maximum_activity_points = 0;
        let base_level = character_information.base_level as usize;
        let job_level = character_information.job_level as usize;
        let stat_points = character_information.stat_points as u32;

        let entity_data = EntityData::from_character(account_id, character_information, WorldPosition::origin());
        let tile_position = TilePosition::new(0, 0);
        let position = Point3::origin();

        let mut common = Common::new(library, &entity_data, tile_position, position, client_tick);
        // Player's own character should not fade in.
        common.fade_state = FadeState::Opaque;

        Self {
            common,
            hair_id,
            spell_points,
            activity_points,
            maximum_spell_points,
            maximum_activity_points,
            base_level,
            job_level,
            stat_points,
            strength: character_information.strength as i32,
            bonus_strength: 0,
            strength_stat_points_cost: 0,
            agility: character_information.agility as i32,
            bonus_agility: 0,
            agility_stat_points_cost: 0,
            vitality: character_information.vitality as i32,
            bonus_vitality: 0,
            vitality_stat_points_cost: 0,
            intelligence: character_information.intelligence as i32,
            bonus_intelligence: 0,
            intelligence_stat_points_cost: 0,
            dexterity: character_information.dexterity as i32,
            bonus_dexterity: 0,
            dexterity_stat_points_cost: 0,
            luck: character_information.luck as i32,
            bonus_luck: 0,
            luck_stat_points_cost: 0,
            attack_speed: 0,
            attack_range: AttackRange(1),
            critical_weight_percentage: 0,
            cart_items: 0,
            cart_maximum_items: 0,
            cart_weight: 0,
            cart_maximum_weight: 0,
            skill_points: 0,
            attack: 0,
            bonus_attack: 0,
            magic_attack_min: 0,
            magic_attack_max: 0,
            defense: 0,
            bonus_defense: 0,
            magic_defense: 0,
            bonus_magic_defense: 0,
            hit: 0,
            flee: 0,
            perfect_dodge: 0,
            critical: 0,
            weight: 0,
            maximum_weight: 0,
            base_experience: 0,
            next_base_experience: 0,
            job_experience: 0,
            next_job_experience: 0,
        }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    /// Current health points (server `SP_HP`).
    #[inline(always)]
    pub fn health_points(&self) -> usize {
        self.common.health_points
    }

    /// Maximum health points (server `SP_MAXHP`).
    #[inline(always)]
    pub fn maximum_health_points(&self) -> usize {
        self.common.maximum_health_points
    }

    /// Base experience progress in percent (0.0 - 100.0).
    ///
    /// Returns `None` if the server did not tell us how much experience is
    /// needed for the next base level yet.
    pub fn base_experience_percent(&self) -> Option<f64> {
        match self.next_base_experience {
            0 => None,
            next => Some(self.base_experience as f64 / next as f64 * 100.0),
        }
    }

    /// Job experience progress in percent (0.0 - 100.0).
    ///
    /// Returns `None` if the server did not tell us how much experience is
    /// needed for the next job level yet.
    pub fn job_experience_percent(&self) -> Option<f64> {
        match self.next_job_experience {
            0 => None,
            next => Some(self.job_experience as f64 / next as f64 * 100.0),
        }
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn update_stat(&mut self, stat_type: StatType) {
        match stat_type {
            StatType::MaximumHealthPoints(value) => self.common.maximum_health_points = value as usize,
            StatType::MaximumSpellPoints(value) => self.maximum_spell_points = value as usize,
            StatType::HealthPoints(value) => self.common.health_points = value as usize,
            StatType::SpellPoints(value) => self.spell_points = value as usize,
            StatType::ActivityPoints(value) => self.activity_points = value as usize,
            StatType::MaximumActivityPoints(value) => self.maximum_activity_points = value as usize,
            StatType::MovementSpeed(value) => self.common.movement_speed = value as usize,
            StatType::BaseLevel(value) => self.base_level = value as usize,
            StatType::JobLevel(value) => self.job_level = value as usize,
            StatType::StatPoints(stat_points) => self.stat_points = stat_points,
            StatType::Strength(base, bonus) => {
                self.strength = base;
                self.bonus_strength = bonus;
            }
            StatType::Agility(base, bonus) => {
                self.agility = base;
                self.bonus_agility = bonus;
            }
            StatType::Vitality(base, bonus) => {
                self.vitality = base;
                self.bonus_vitality = bonus;
            }
            StatType::Intelligence(base, bonus) => {
                self.intelligence = base;
                self.bonus_intelligence = bonus;
            }
            StatType::Dexterity(base, bonus) => {
                self.dexterity = base;
                self.bonus_dexterity = bonus;
            }
            StatType::Luck(base, bonus) => {
                self.luck = base;
                self.bonus_luck = bonus;
            }
            StatType::StrengthStatPointCost(cost) => self.strength_stat_points_cost = cost,
            StatType::AgilityStatPointCost(cost) => self.agility_stat_points_cost = cost,
            StatType::VitalityStatPointCost(cost) => self.vitality_stat_points_cost = cost,
            StatType::IntelligenceStatPointCost(cost) => self.intelligence_stat_points_cost = cost,
            StatType::DexterityStatPointCost(cost) => self.dexterity_stat_points_cost = cost,
            StatType::LuckStatPointCost(cost) => self.luck_stat_points_cost = cost,
            StatType::AttackSpeed(attack_speed) => self.attack_speed = attack_speed,
            StatType::SkillPoints(skill_points) => self.skill_points = skill_points,
            StatType::Attack1(attack) => self.attack = attack,
            StatType::Attack2(attack) => self.bonus_attack = attack,
            StatType::MagicAttack1(magic_attack) => self.magic_attack_max = magic_attack,
            StatType::MagicAttack2(magic_attack) => self.magic_attack_min = magic_attack,
            StatType::Defense1(defense) => self.defense = defense,
            StatType::Defense2(defense) => self.bonus_defense = defense,
            StatType::MagicDefense1(magic_defense) => self.magic_defense = magic_defense,
            StatType::MagicDefense2(magic_defense) => self.bonus_magic_defense = magic_defense,
            StatType::Hit(hit) => self.hit = hit,
            StatType::Flee1(flee) => self.flee = flee,
            StatType::Flee2(flee) => self.perfect_dodge = flee,
            StatType::Critical(critical) => self.critical = critical,
            StatType::Weight(weight) => self.weight = weight,
            StatType::MaximumWeight(maximum_weight) => self.maximum_weight = maximum_weight,
            StatType::BaseExperience(base_experience) => self.base_experience = base_experience,
            StatType::NextBaseExperience(next_base_experience) => self.next_base_experience = next_base_experience,
            StatType::JobExperience(job_experience) => self.job_experience = job_experience,
            StatType::NextJobExperience(next_job_experience) => self.next_job_experience = next_job_experience,
            StatType::CartInfo(items, weight, maximum_weight) => {
                self.cart_items = items;
                self.cart_weight = weight;
                self.cart_maximum_weight = maximum_weight;
            }
            _ => {}
        }
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.player_bar_width;
        let gap = theme.status_bar.gap;
        let total_height =
            theme.status_bar.health_height + theme.status_bar.spell_point_height + theme.status_bar.activity_point_height + gap * 2.0;

        let mut offset = 0.0;

        let background_position = final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0);

        let background_size = ScreenSize {
            width: bar_width,
            height: total_height,
        } + theme.status_bar.border_size * 2.0;

        renderer.render_rectangle(background_position, background_size, theme.status_bar.background_color);

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.health_height,
            },
            theme.status_bar.player_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );

        offset += gap + theme.status_bar.health_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.spell_point_height,
            },
            theme.status_bar.spell_point_color,
            self.maximum_spell_points as f32,
            self.spell_points as f32,
        );

        offset += gap + theme.status_bar.spell_point_height;

        renderer.render_bar(
            final_position + ScreenPosition::only_top(offset),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.activity_point_height,
            },
            theme.status_bar.activity_point_color,
            self.maximum_activity_points as f32,
            self.activity_points as f32,
        );
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<EntityPartFile> {
        let common = self.get_common();
        get_entity_part_files(
            library,
            common.entity_type,
            common.job_id,
            common.sex,
            Some(self.hair_id),
            common.head_palette,
            common.body_palette,
            common.accessory,
            common.accessory2,
            common.accessory3,
            common.effect_state,
        )
    }
}

/// Job ids of actors that the official client renders as GR2 (Granny 3D) models
/// from `data\model\3dmob\...` instead of a 2D sprite. The game files contain
/// no `data\sprite\npc\<name>.spr`/`.act` for them, so the sprite loader used
/// to fall back to the opaque black `missing.spr` placeholder (this is what the
/// "castle flags are not rendered" and "emperium is a black box" reports saw).
///
/// - `722`  `JT_GUILD_FLAG` - `data\model\3dmob\Guildflag90_1.gr2`, drawn with
///   the owning guild's emblem texture.
/// - `1288` `JT_EMPELIUM`   - `data\model\3dmob\Empelium90_0.gr2`.
///
/// ro-next has no GR2 loader yet, so these entities are skipped instead of
/// being drawn as black silhouettes. TODO: render them once a GR2 model loader
/// and the guild emblem system exist.
const GR2_MODEL_ACTOR_JOB_IDS: [u16; 2] = [722, 1288];

#[derive(Clone, RustState, StateWindow)]
pub struct Npc {
    common: Common,
}

impl Npc {
    pub fn new(
        library: &Library,
        map: &Map,
        path_finder: &mut PathFinder,
        entity_data: EntityData,
        client_tick: ClientTick,
    ) -> Option<Self> {
        if GR2_MODEL_ACTOR_JOB_IDS.contains(&entity_data.job_id.0) {
            return None;
        }

        let Some(position) = map.get_world_position(entity_data.position.tile_position()) else {
            #[cfg(feature = "debug")]
            ro_next_debug::logging::print_debug!(
                "[{}] NPC with id {:?} is out of map bounds",
                "error".red(),
                entity_data.entity_id
            );
            return None;
        };

        let mut common = Common::new(
            library,
            &entity_data,
            entity_data.position.tile_position(),
            position,
            client_tick,
        );

        if let Some(destination) = entity_data.destination {
            common.move_from_to(
                map,
                path_finder,
                &[],
                entity_data.position.tile_position(),
                destination.tile_position(),
                client_tick,
            );
        }

        Some(Self { common })
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        if self.common.entity_type != EntityType::Monster {
            return;
        }

        let clip_space_position = camera.view_projection_matrix() * self.common.world_position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height + 5.0,
        };

        let bar_width = theme.status_bar.enemy_bar_width;

        renderer.render_rectangle(
            final_position - theme.status_bar.border_size - ScreenSize::only_width(bar_width / 2.0),
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            } + (theme.status_bar.border_size * 2.0),
            theme.status_bar.background_color,
        );

        renderer.render_bar(
            final_position,
            ScreenSize {
                width: bar_width,
                height: theme.status_bar.enemy_health_height,
            },
            theme.status_bar.enemy_health_color,
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
    }
}

/// The second label line that is drawn under the name of a player, if any.
///
/// Only players in a guild get a guild label; NPCs, monsters and players
/// without a guild keep their single line name label. The name is wrapped in
/// angle brackets the way it is usually shown in Ragnarok Online clients.
pub fn guild_name_label(entity_type: EntityType, guild_name: Option<&str>) -> Option<String> {
    if entity_type != EntityType::Player {
        return None;
    }

    let guild_name = guild_name?.trim();

    if guild_name.is_empty() {
        return None;
    }

    Some(format!("<{guild_name}>"))
}

#[derive(Clone, StateElement)]
/// Either a player character or an NPC.
///
/// The player variant is intentionally stored inline instead of behind a
/// `Box`: entities are matched on every frame in the render and update paths,
/// so an extra allocation/indirection for every entity would be worse than the
/// size difference between the two variants.
#[allow(clippy::large_enum_variant)]
pub enum Entity {
    Player(Player),
    Npc(Npc),
}

impl Entity {
    /// Whether this entity currently has the hunter falcon option set.
    pub fn has_falcon(&self) -> bool {
        self.get_common().has_falcon()
    }

    pub fn set_falcon_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().set_falcon_animation(animation_data);
    }

    pub fn has_falcon_animation(&self) -> bool {
        self.get_common().has_falcon_animation()
    }

    /// Whether this entity currently has a merchant cart option set.
    pub fn has_cart(&self) -> bool {
        self.get_common().has_cart()
    }

    pub fn set_cart_status(&mut self, active: bool, kind: i32) {
        self.get_common_mut().set_cart_status(active, kind);
    }

    pub fn get_cart_part_candidates(&self) -> Vec<EntityPartFile> {
        self.get_common().get_cart_part_candidates()
    }

    pub fn set_status_tint(&mut self, tint: Option<Color>) {
        self.get_common_mut().set_status_tint(tint);
    }

    pub fn set_weapon_blink(&mut self, blink: bool) {
        self.get_common_mut().set_weapon_blink(blink);
    }

    pub fn start_body_wave(&mut self, duration_ms: u32, amplitude: f32, client_tick: ClientTick) {
        self.get_common_mut().start_body_wave(duration_ms, amplitude, client_tick);
    }

    pub fn start_body_outline(&mut self, color: Color, duration_ms: u32, client_tick: ClientTick) {
        self.get_common_mut().start_body_outline(color, duration_ms, client_tick);
    }

    pub fn start_body_squash(&mut self, duration_ms: u32, amount: f32, client_tick: ClientTick) {
        self.get_common_mut().start_body_squash(duration_ms, amount, client_tick);
    }

    /// Sets the additive body/weapon sprite blend of the active statuses.
    pub fn set_status_additive(&mut self, body: bool, weapon: bool) {
        self.get_common_mut().set_status_additive(body, weapon);
    }

    pub fn set_status_alpha(&mut self, alpha: Option<f32>) {
        self.get_common_mut().set_status_alpha(alpha);
    }

    pub fn needs_cart_animation(&self) -> bool {
        self.get_common().needs_cart_animation()
    }

    pub fn set_cart_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().set_cart_animation(animation_data);
    }

    pub fn has_cart_animation(&self) -> bool {
        self.get_common().has_cart_animation()
    }

    pub fn get_weapon_part_candidates(&self) -> Vec<EntityPartFile> {
        self.get_common().get_weapon_part_candidates()
    }

    pub fn get_shield_part_candidates(&self) -> Vec<EntityPartFile> {
        self.get_common().get_shield_part_candidates()
    }

    pub fn get_robe_part_candidates(&self) -> Vec<EntityPartFile> {
        self.get_common().get_robe_part_candidates()
    }

    pub fn get_weapon(&self) -> u32 {
        self.get_common().get_weapon()
    }

    #[cfg(test)]
    pub fn get_shield(&self) -> u32 {
        self.get_common().get_shield()
    }

    #[cfg(test)]
    pub fn get_robe(&self) -> u16 {
        self.get_common().get_robe()
    }

    pub fn set_weapon_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().set_weapon_animation(animation_data);
    }

    pub fn set_shield_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().set_shield_animation(animation_data);
    }

    pub fn set_robe_animation(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().set_robe_animation(animation_data);
    }

    pub fn has_weapon_animation(&self) -> bool {
        self.get_common().has_weapon_animation()
    }

    pub fn has_shield_animation(&self) -> bool {
        self.get_common().has_shield_animation()
    }

    pub fn has_robe_animation(&self) -> bool {
        self.get_common().has_robe_animation()
    }

    /// Whether the body animation data of this entity is still missing
    /// (requested but not loaded yet, or the load was lost/failed).
    pub fn needs_animation_data(&self) -> bool {
        self.get_common().animation_data.is_none()
    }

    /// Whether the falcon animation is required but still missing.
    pub fn needs_falcon_animation(&self) -> bool {
        let common = self.get_common();
        common.has_falcon() && !common.has_falcon_animation()
    }

    /// Whether the equipped weapon animation is required but still missing.
    pub fn needs_weapon_animation(&self) -> bool {
        let common = self.get_common();
        common.animation_data.is_some() && !common.has_weapon_animation() && !common.get_weapon_part_candidates().is_empty()
    }

    /// Whether the equipped shield animation is required but still missing.
    pub fn needs_shield_animation(&self) -> bool {
        let common = self.get_common();
        common.animation_data.is_some() && common.needs_shield_animation()
    }

    /// Whether the equipped robe animation is required but still missing.
    pub fn needs_robe_animation(&self) -> bool {
        let common = self.get_common();
        common.animation_data.is_some() && common.needs_robe_animation()
    }

    fn get_common(&self) -> &Common {
        match self {
            Self::Player(player) => player.get_common(),
            Self::Npc(npc) => npc.get_common(),
        }
    }

    fn get_common_mut(&mut self) -> &mut Common {
        match self {
            Self::Player(player) => player.get_common_mut(),
            Self::Npc(npc) => npc.get_common_mut(),
        }
    }

    pub fn get_entity_id(&self) -> EntityId {
        self.get_common().entity_id
    }

    /// Character id from the entity-appear packet (`GID`), `0` for non-player
    /// actors.
    pub fn get_character_id(&self) -> CharacterId {
        self.get_common().character_id
    }

    pub fn get_sex(&self) -> Sex {
        self.get_common().sex
    }

    /// rAthena `opt1` (`bodyState`): stone/freeze/stun/sleep/burning/...
    ///
    /// The head mark overlay renders a status icon above the entity for this
    /// value; the server sends updates through `ZC_STATE_CHANGE` (0x0229).
    pub fn get_body_state(&self) -> u16 {
        self.get_common().body_state
    }

    /// rAthena `opt2` (`healthState`): poison/curse/silence/confusion/blind/...
    pub fn get_health_state(&self) -> u16 {
        self.get_common().health_state
    }

    pub fn needs_imf(&self) -> bool {
        self.get_common().needs_imf()
    }

    pub fn set_imf(&mut self, imf: Arc<Imf>) {
        self.get_common_mut().set_imf(imf);
    }

    pub fn get_job_id(&self) -> JobId {
        self.get_common().job_id
    }

    /// Base level of player entities, `None` for entities whose level is not
    /// known (NPCs, and player shells that only appear via a spawn packet).
    pub fn get_base_level(&self) -> Option<usize> {
        match self {
            Self::Player(player) => Some(player.base_level),
            // Other players (including population-engine shells) only expose the
            // encoded `c_level`: 99 means "level >= aura level" (draw the aura),
            // 98 is the level directly below the aura level, lower values are the
            // real level. Do not treat 98 as an aura level.
            Self::Npc(npc) => match npc.get_common().entity_type {
                EntityType::Player => Some(aura_level_from_c_level(npc.get_common().c_level)),
                _ => None,
            },
        }
    }

    /// Whether this entity uses a transcendent (rebirth) job.
    ///
    /// The classic level-99 aura uses a violet tint for transcendent
    /// characters. The flag comes from the job data (`jobs.ron`), so it follows
    /// the server job database instead of a magic id range.
    pub fn is_transcendent_job(&self) -> bool {
        crate::world::jobs::is_transcendent(self.get_job_id())
    }

    pub fn get_entity_type(&self) -> EntityType {
        self.get_common().entity_type
    }

    pub fn is_dead(&self) -> bool {
        self.get_common().is_dead()
    }

    pub fn is_death_animation_over(&self) -> bool {
        self.get_common().is_death_animation_over()
    }

    pub fn is_fading(&self) -> bool {
        self.get_common().is_fading()
    }

    pub fn fade_out(&mut self, reason: DisappearanceReason, client_tick: ClientTick) {
        const DIED_FADE_DURATION_MS: u32 = 2000;

        let fade_state = &mut self.get_common_mut().fade_state;

        let fade_duration = match reason {
            DisappearanceReason::OutOfSight => FADE_IN_DURATION_MS,
            DisappearanceReason::Died => DIED_FADE_DURATION_MS,
            _ => FADE_IN_DURATION_MS,
        };

        let current_alpha = fade_state.calculate_alpha(client_tick);
        *fade_state = FadeState::from_alpha(current_alpha, FadeDirection::Out, client_tick, fade_duration);
    }

    pub fn inherit_fade_state(&mut self, previous_entity: &Entity, client_tick: ClientTick) {
        let fade_state = &mut self.get_common_mut().fade_state;
        let previous_fade_state = previous_entity.get_common().fade_state;

        *fade_state = match previous_fade_state {
            FadeState::Opaque => FadeState::Opaque,
            FadeState::Fading { .. } => {
                let alpha = previous_fade_state.calculate_alpha(client_tick);
                FadeState::from_alpha(alpha, FadeDirection::In, client_tick, FADE_IN_DURATION_MS)
            }
        };
    }

    pub fn should_be_removed(&self, client_tick: ClientTick) -> bool {
        self.get_common().fade_state.is_done_fading_out(client_tick)
    }

    /// Whether the entity details/name have to be (re)requested now.
    ///
    /// A request that was lost is retried after the request timeout (bounded by
    /// the attempt budget), so a lost `0x0A30`/`0x0ADF` response can no longer
    /// leave an entity nameless for its whole lifetime.
    pub fn details_need_request(&mut self, now: Instant) -> bool {
        self.get_common_mut().details.needs_request(now)
    }

    pub fn set_job(&mut self, _library: &Library, job_id: JobId) {
        let scale = crate::world::jobs::scale(job_id);

        let common = self.get_common_mut();
        common.job_id = job_id;
        common.scale = scale;
    }

    /// Applies an appearance (sprite) change for headgear, palettes or
    /// equipment.
    ///
    /// Returns the sprite layers that have to be reloaded, or `None` for sprite
    /// change types the entity layer does not handle.
    pub fn apply_sprite_change(&mut self, sprite_type: SpriteChangeType, value: u32) -> Option<SpriteChangeOutcome> {
        // The packet field names follow the official client: `accessory` is the
        // lower, `accessory2` the upper and `accessory3` the middle headgear.
        let common = self.get_common_mut();
        let mut outcome = SpriteChangeOutcome::default();

        match sprite_type {
            SpriteChangeType::HeadTop => {
                common.accessory2 = value as u16;
                outcome.body = true;
            }
            SpriteChangeType::HeadMiddle => {
                common.accessory3 = value as u16;
                outcome.body = true;
            }
            SpriteChangeType::HeadBottom => {
                common.accessory = value as u16;
                outcome.body = true;
            }
            SpriteChangeType::HairCollor => {
                common.head_palette = value as u16;
                outcome.body = true;
            }
            SpriteChangeType::ClothesColor => {
                common.body_palette = value as u16;
                outcome.body = true;
            }
            // Equipping/unequipping a weapon does not change the body sprite,
            // but it changes the weapon animation layer. The server sends the
            // weapon type in `ZC_SPRITE_CHANGE` when the equipment changes.
            SpriteChangeType::Weapon => {
                common.set_equipment_look(value, EquipmentLook::Weapon);
                outcome.weapon = true;
            }
            SpriteChangeType::Shield => {
                common.set_equipment_look(value, EquipmentLook::Shield);
                outcome.shield = true;
            }
            SpriteChangeType::Robe => {
                common.set_equipment_look(value, EquipmentLook::Robe);
                outcome.robe = true;
            }
            _ => return None,
        }

        Some(outcome)
    }

    pub fn set_hair(&mut self, hair_id: usize) {
        if let Self::Player(player) = self {
            player.hair_id = hair_id
        }
    }

    pub fn set_animation_data(&mut self, animation_data: Arc<AnimationData>) {
        self.get_common_mut().animation_data = Some(animation_data)
    }

    pub fn get_entity_part_files(&self, library: &Library) -> Vec<EntityPartFile> {
        match self {
            Self::Player(player) => player.get_entity_part_files(library),
            Self::Npc(npc) => npc.get_common().get_entity_part_files(library),
        }
    }

    pub fn set_details_requested(&mut self, now: Instant) {
        self.get_common_mut().details.mark_requested(now);
    }

    pub fn set_details(&mut self, details: String) {
        self.get_common_mut().details.mark_available(details);
    }

    pub fn get_details(&self) -> Option<&String> {
        self.get_common().details.as_option()
    }

    /// Id of the guild the entity is in, `0` if it is not in a guild.
    pub fn get_guild_id(&self) -> u32 {
        self.get_common().get_guild_id()
    }

    /// The guild name of the entity, if it is known and the entity is in a
    /// guild.
    pub fn get_guild_name(&self) -> Option<&str> {
        self.get_common().get_guild_name()
    }

    /// Cached `<guild name>` label for the name plate.
    pub fn get_guild_label(&self) -> Option<&str> {
        self.get_common().get_guild_label()
    }

    /// Updates the guild name of the entity.
    ///
    /// The server sends the guild name together with the player details, so
    /// this is called from the same place as [`Entity::set_details`]. An empty
    /// name means the player is not in a guild (anymore) and removes the label.
    pub fn set_guild_name(&mut self, guild_name: Option<String>) {
        self.get_common_mut().set_guild_name(guild_name);
    }

    /// Title of the player's open vending shop, `None` when the player is not
    /// vending (or the shop entry was removed).
    pub fn get_vending_shop_name(&self) -> Option<&str> {
        self.get_common().vending_shop_name.as_deref()
    }

    /// Updates the vending shop title from `ZC_STORE_ENTRY` /
    /// `ZC_DISAPPEAR_ENTRY`.
    pub fn set_vending_shop_name(&mut self, shop_name: Option<String>) {
        self.get_common_mut().vending_shop_name = shop_name;
    }

    pub fn get_tile_position(&self) -> TilePosition {
        self.get_common().tile_position
    }

    pub fn get_position(&self) -> Point3<f32> {
        self.get_common().world_position
    }

    pub fn get_scale(&self) -> f32 {
        self.get_common().scale
    }

    /// Applies `EFST_CHANGE_SIZE` / `EFST_CHANGE_SIZE_MONSTER` (1420/1421).
    /// `percent` is the server `val1` (100 = normal); `false` restores the job
    /// base scale.
    pub fn set_status_scale(&mut self, active: bool, percent: i32) {
        let common = self.get_common_mut();
        common.scale = if active && percent > 0 {
            common.base_scale * percent as f32 / 100.0
        } else {
            common.base_scale
        };
    }

    pub fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
        self.get_common_mut().set_position(map, position, client_tick);
    }

    pub fn set_dead(&mut self, client_tick: ClientTick) {
        self.get_common_mut().set_dead(client_tick);
    }

    /// Update the body direction (`0x009C` `ZC_CHANGE_DIRECTION`). The network
    /// direction is converted to the client's `Direction` enum by the packet
    /// handler; the head direction is a separate value.
    pub fn set_direction(&mut self, direction: Direction) {
        self.get_common_mut().direction = direction;
    }

    /// Current body direction (used by the damage-number fountain).
    pub fn get_direction(&self) -> Direction {
        self.get_common().direction
    }

    pub fn set_head_direction(&mut self, head_direction: usize) {
        self.get_common_mut().head_direction = head_direction;
    }

    /// Option bit mask (`effectState`) update (`0x0229` `ZC_STATE_CHANGE`).
    pub fn set_effect_state(&mut self, effect_state: u32, client_tick: ClientTick) {
        self.get_common_mut().set_effect_state(effect_state, client_tick);
    }

    /// `bodyState` (`opt1`) update (`0x0229`).
    pub fn set_body_state(&mut self, body_state: u16) {
        self.get_common_mut().set_body_state(body_state);
    }

    /// `healthState` (`opt2`) update (`0x0229`).
    pub fn set_health_state(&mut self, health_state: u16) {
        self.get_common_mut().set_health_state(health_state);
    }

    pub fn set_idle(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.idle(entity_type, client_tick);
    }

    pub fn set_pickup(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.pickup(entity_type, client_tick);
    }

    /// Combat-ready stance (official `READYFIGHT` skill action, `SM_ENDURE`).
    pub fn set_ready_fight(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.ready_fight(entity_type, client_tick);
    }

    /// Sit down (`ZC_NOTIFY_ACT` with `DamageType::SitDown`).
    pub fn set_sitting(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.sit(entity_type, client_tick);
    }

    /// Stand up again (`ZC_NOTIFY_ACT` with `DamageType::StandUp`).
    pub fn set_standing(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.stand_up(entity_type, client_tick);
    }

    /// Play the skill-cast animation on this entity for `cast_time`
    /// milliseconds. Casts with a cast time loop the animation, instant casts
    /// play it once.
    pub fn set_skill_cast(&mut self, cast_time: u32, looping: bool, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut()
            .animation_state
            .skill(entity_type, cast_time, looping, client_tick);
    }

    /// Plays an explicit ACT action index (`CasterAction::Action`).
    pub fn set_custom_skill_action(&mut self, action_index: usize, cast_time: u32, looping: bool, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut()
            .animation_state
            .custom(entity_type, action_index, cast_time, looping, client_tick);
    }

    /// Restricts the current caster animation to the official per-skill frame
    /// sequence (`RenderObject.cpp` `SetFrameSequence`): frames
    /// `first..=last`, then `loop_to..=last` for `duration_ms`.
    pub fn set_skill_frame_sequence(&mut self, first: u8, last: u8, loop_to: u8, duration_ms: u32, client_tick: ClientTick) {
        self.get_common_mut()
            .animation_state
            .frame_sequence(first, last, loop_to, duration_ms, client_tick);
    }

    /// Starts a short body flash (hard-coded skill effect color flash, for
    /// example the yellow `EF_PORTAL5` flash of Zen).
    pub fn start_body_flash(&mut self, color: Color, hold_ms: u32, fade_ms: u32, additive: bool, client_tick: ClientTick) {
        self.get_common_mut()
            .start_body_flash(color, hold_ms, fade_ms, additive, client_tick);
    }

    /// Stop the looping skill-cast animation of this entity.
    ///
    /// Called when the server reports a cancelled cast (`ZC_DISPEL`, 0x01B9):
    /// the caster returns to its idle animation instead of looping the cast
    /// gesture until some other action replaces it.
    pub fn cancel_skill_cast(&mut self, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut().animation_state.idle(entity_type, client_tick);
    }

    /// Duration of the skill-cast animation of this entity in milliseconds.
    ///
    /// Effects that are spawned together with a cast have to stay alive at
    /// least this long so that the visual does not end before the animation
    /// does.
    pub fn skill_animation_duration_ms(&self) -> u32 {
        self.get_common()
            .animation_data
            .as_ref()
            .map(|animation_data| animation_data.skill_animation_duration_ms())
            .unwrap_or(0)
    }

    pub fn is_sitting(&self) -> bool {
        self.get_common().animation_state.is_sitting()
    }

    pub fn rotate_towards(&mut self, target_position: TilePosition) {
        let common = self.get_common_mut();

        // FIX: This check is a little bit broken. This will prefer rotation diagonally
        // over rotating straight.
        if let Ok(direction) = Direction::try_from([
            (common.tile_position.x as isize - target_position.x as isize).clamp(-1, 1),
            (common.tile_position.y as isize - target_position.y as isize).clamp(-1, 1),
        ]) {
            common.direction = direction;
        }
    }

    pub fn set_attack(&mut self, attack_duration: u32, critical: bool, client_tick: ClientTick) {
        let entity_type = self.get_entity_type();
        self.get_common_mut()
            .animation_state
            .attack(entity_type, attack_duration, critical, client_tick);
    }

    pub fn stopped_moving(&self) -> bool {
        self.get_common().stopped_moving
    }

    pub fn stop_movement(&mut self) {
        self.get_common_mut().active_movement = None;
    }

    pub fn update_health(&mut self, health_points: usize, maximum_health_points: usize) {
        let common = self.get_common_mut();
        common.health_points = health_points;
        common.maximum_health_points = maximum_health_points;
    }

    pub fn update(&mut self, audio_engine: &AudioEngine<GameFileLoader>, map: &Map, camera: &dyn Camera, client_tick: ClientTick) {
        self.get_common_mut().update(audio_engine, map, camera, client_tick);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn move_from_to(
        &mut self,
        map: &Map,
        path_finder: &mut PathFinder,
        cell_overrides: &[(TilePosition, bool)],
        from: TilePosition,
        to: TilePosition,
        starting_timestamp: ClientTick,
    ) {
        self.get_common_mut()
            .move_from_to(map, path_finder, cell_overrides, from, to, starting_timestamp);
    }

    #[cfg(feature = "debug")]
    pub fn generate_pathing_mesh(&mut self, device: &Device, queue: &Queue, bindless_support: BindlessSupport, map: &Map) {
        self.get_common_mut().generate_pathing_mesh(device, queue, bindless_support, map);
    }

    pub fn render(&self, instructions: &mut Vec<EntityInstruction>, camera: &dyn Camera, add_to_picker: bool, client_tick: ClientTick) {
        self.get_common().render(instructions, camera, add_to_picker, client_tick);
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, camera: &dyn Camera) {
        self.get_common().render_debug(instructions, camera);
    }

    #[cfg(feature = "debug")]
    pub fn get_pathing(&self) -> Option<&Pathing> {
        self.get_common()
            .active_movement
            .as_ref()
            .and_then(|movement| movement.pathing.as_ref())
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        self.get_common().render_marker(renderer, camera, marker_identifier, hovered);
    }

    pub fn render_status(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, theme: &WorldTheme, window_size: ScreenSize) {
        match self {
            Self::Player(player) => player.render_status(renderer, camera, theme, window_size),
            Self::Npc(npc) => npc.render_status(renderer, camera, theme, window_size),
        }
    }
}

impl VecItem for Entity {
    type Id = EntityId;

    fn get_id(&self) -> Self::Id {
        self.get_entity_id()
    }
}

// TODO: Derive this
impl StateWindow<ClientState> for Entity {
    fn to_window<'a>(_self_path: impl Path<ClientState, Self>) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: "Entity",
            theme: InterfaceThemeType::InGame,
            closable: true,
            // TODO: This is gonna be a bit hacky but we want to have this save path possibly be
            // None and dispaly a message if the entity disappeared.
            elements: (),
        }
    }

    fn to_window_mut<'a>(_self_path: impl Path<ClientState, Self>) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: "Entity",
            theme: InterfaceThemeType::InGame,
            closable: true,
            // TODO: This is gonna be a bit hacky but we want to have this save path possibly be
            // None and dispaly a message if the entity disappeared.
            elements: (),
        }
    }
}

#[cfg(test)]
mod object_type_tests {
    use super::EntityType;

    #[test]
    fn only_companion_object_types_override_the_job_id_classification() {
        assert_eq!(EntityType::from_object_type(0x7), Some(EntityType::Pet));
        assert_eq!(EntityType::from_object_type(0x8), Some(EntityType::Homunculus));
        assert_eq!(EntityType::from_object_type(0x9), Some(EntityType::Mercenary));
        // Players/monsters/NPCs keep the existing job-id classification.
        assert_eq!(EntityType::from_object_type(0x0), None);
        assert_eq!(EntityType::from_object_type(0x5), None);
        assert_eq!(EntityType::from_object_type(0x6), None);
        assert_eq!(EntityType::from_object_type(0xC), None);
        assert_eq!(EntityType::from_object_type(0x7F), None);
    }
}

#[cfg(test)]
mod guild_name_label_tests {
    use super::{EntityType, guild_name_label};

    #[test]
    fn player_in_guild_gets_angle_bracketed_label() {
        assert_eq!(
            guild_name_label(EntityType::Player, Some("Domination")),
            Some("<Domination>".to_owned())
        );
    }

    #[test]
    fn player_without_guild_gets_no_label() {
        assert_eq!(guild_name_label(EntityType::Player, None), None);
        assert_eq!(guild_name_label(EntityType::Player, Some("")), None);
        assert_eq!(guild_name_label(EntityType::Player, Some("   ")), None);
    }

    #[test]
    fn guild_label_is_trimmed() {
        assert_eq!(
            guild_name_label(EntityType::Player, Some("  Domination  ")),
            Some("<Domination>".to_owned())
        );
    }

    #[test]
    fn npcs_and_monsters_never_get_a_guild_label() {
        assert_eq!(guild_name_label(EntityType::Npc, Some("Domination")), None);
        assert_eq!(guild_name_label(EntityType::Monster, Some("Domination")), None);
        assert_eq!(guild_name_label(EntityType::Warp, Some("Domination")), None);
        assert_eq!(guild_name_label(EntityType::Hidden, Some("Domination")), None);
    }
}

#[cfg(test)]
mod c_level_tests {
    use super::aura_level_from_c_level;
    use crate::world::AURA_MINIMUM_LEVEL;

    #[test]
    fn encoded_c_level_maps_to_aura_eligibility() {
        // 99 means "level >= aura level".
        assert_eq!(aura_level_from_c_level(99), 99);
        assert!(aura_level_from_c_level(99) >= AURA_MINIMUM_LEVEL);

        // 98 is the level directly below the aura level and must never be
        // treated as an aura level.
        assert_eq!(aura_level_from_c_level(98), 98);
        assert!(aura_level_from_c_level(98) < AURA_MINIMUM_LEVEL);

        // Lower levels are transmitted as the real level.
        assert_eq!(aura_level_from_c_level(50), 50);
        assert!(aura_level_from_c_level(50) < AURA_MINIMUM_LEVEL);
        assert_eq!(aura_level_from_c_level(1), 1);
    }

    #[test]
    fn only_encoded_99_reaches_the_aura_threshold() {
        for level in 0..=200u16 {
            assert_eq!(
                aura_level_from_c_level(level) >= AURA_MINIMUM_LEVEL,
                level >= AURA_MINIMUM_LEVEL as u16,
                "unexpected aura eligibility for c_level {level}"
            );
        }
    }
}
