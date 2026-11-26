//! The shared per-entity state (`Common`): appearance, stats, movement
//! bookkeeping and the part-file lookups.

use super::*;

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
    /// The server sends the shield item id; [`crate::world::shield`] resolves
    /// it to the item view id and the sprite candidates of the equipped
    /// shield.
    #[hidden_element]
    pub shield: u32,
    /// Robe view id (`robe` in the entity-appear packets), `0` if none.
    ///
    /// See [`crate::world::robe`] for the sprite candidates and the animated
    /// opacity of transparent robes.
    #[hidden_element]
    pub robe: u16,
    /// Animation of the hunter falcon, loaded lazily when the entity actually
    /// has the falcon option (`effect_state & OPTION_FALCON`).
    #[hidden_element]
    pub(crate) falcon_animation: Option<Arc<AnimationData>>,
    /// Independent animation state of the falcon (perched/flapping loop).
    #[hidden_element]
    pub(crate) falcon_animation_state: AnimationState,
    /// Active falcon-attack flight (`AM_REQ_ATTACK`); `None` while perched.
    #[hidden_element]
    pub(crate) falcon_flight: Option<FalconFlight>,
    /// The flight has entered its return leg and the perched flap loop has been
    /// restored (the flight action is only used for the dive).
    #[hidden_element]
    pub(crate) falcon_returning: bool,
    /// Merchant cart animation, loaded lazily while the entity has a cart
    /// option (`effect_state & OPTION_CART_MASK`).
    #[hidden_element]
    pub(crate) cart_animation: Option<Arc<AnimationData>>,
    /// Per-job `.imf` head/body draw order, loaded lazily for players.
    #[hidden_element]
    pub(crate) imf: Option<Arc<Imf>>,
    /// The server's `EFST_PUSH_CART` status is active. Some 2019 servers do
    /// not put the cart option into the entity `effect_state`, so the status
    /// is an independent visibility source for the cart.
    pub(crate) cart_status_active: bool,
    /// Cart type from `EFST_PUSH_CART` (673) or the `OPTION_CART*` bits. 1 is
    /// the basic pushcart; 10..13 are named carts.
    pub(crate) cart_kind: i32,
    /// Persistent sprite tint of a status effect (Adrenaline Rush and Fury are
    /// the classic red tints). `None` means no status tint.
    pub(crate) status_tint: Option<Color>,
    pub(crate) status_weapon_blink: bool,
    #[hidden_element]
    pub(crate) body_wave: Option<BodyWave>,
    #[hidden_element]
    pub(crate) body_outline: Option<BodyOutline>,
    #[hidden_element]
    pub(crate) body_squash: Option<BodySquash>,
    /// Draw the body layers with an additive blend (official
    /// `EF_REDBODY`/`render.flags & 1`).
    pub(crate) status_body_additive: bool,
    /// Draw the equipped weapon layer with an additive blend (official
    /// `EF_AURABLADE2`/`render.flags & 2`).
    pub(crate) status_weapon_additive: bool,
    /// Body alpha multiplier from a status visual (`BodyFade`), `None` = 1.0.
    pub(crate) status_alpha: Option<f32>,
    /// Equipped weapon animation, drawn as its own layer on top of the body
    /// animation using the body animation state (same ACT action layout).
    #[hidden_element]
    pub(crate) weapon_animation: Option<Arc<AnimationData>>,
    /// Equipped shield animation, drawn in front of the body/head layers.
    #[hidden_element]
    pub(crate) shield_animation: Option<Arc<AnimationData>>,
    /// Equipped robe animation, drawn as the top layer over the body.
    #[hidden_element]
    pub(crate) robe_animation: Option<Arc<AnimationData>>,
    pub active_movement: Option<Movement>,
    pub animation_data: Option<Arc<AnimationData>>,
    /// Pending death: the tick at which the server reported the death. The
    /// entity keeps its current (attack) animation for a short grace period
    /// before the death animation starts, so the killing blow is visible.
    #[hidden_element]
    pub(crate) pending_death: Option<u32>,
    pub tile_position: TilePosition,
    pub world_position: Point3<f32>,
    pub scale: f32,
    /// Job/baby base scale, restored when `EFST_CHANGE_SIZE` ends.
    #[hidden_element]
    pub(crate) base_scale: f32,
    #[hidden_element]
    pub(crate) details: ResourceState<String>,
    /// Id of the guild the entity is in, `0` if it is not in a guild.
    ///
    /// The entity-appear packets carry the guild id, the guild *name* is only
    /// known after the server answered a name request (see
    /// [`Entity::set_details`]).
    #[hidden_element]
    pub(crate) guild_id: u32,
    /// The name of the guild the entity is in.
    ///
    /// `None` for entities that are not in a guild. The entity-appear packets
    /// only carry the guild id, so the name is only known after the server
    /// answered a name request (see [`Entity::set_details`]).
    #[hidden_element]
    pub(crate) guild_name: Option<String>,
    /// Cached `<guild name>` label rendered under the character name.
    ///
    /// Composing it once when the guild name is set avoids a `String`
    /// allocation per entity per frame in crowded towns.
    #[hidden_element]
    pub(crate) guild_label: Option<String>,
    /// Title of an open vending shop of this player (`ZC_STORE_ENTRY`, 0x0131,
    /// `clif.cpp:7558`). `None` when the player is not vending. Used by the
    /// character context menu to offer browsing the shop.
    #[hidden_element]
    pub(crate) vending_shop_name: Option<String>,
    #[hidden_element]
    pub(crate) animation_state: AnimationState,
    pub(crate) stopped_moving: bool,
    #[hidden_element]
    pub(crate) sound_state: SoundState,
    #[hidden_element]
    pub(crate) fade_state: FadeState,
    /// Temporary sprite tint of a hard-coded skill effect (`None` when the
    /// entity is not flashing).
    #[hidden_element]
    pub(crate) body_flash: Option<EntityFlash>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn get_entity_part_files(
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
pub(super) const OPTION_HIDDEN_MASK: u32 = 0x0000_0002 | 0x0000_0004 | 0x0000_0040 | 0x0000_4000;

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
            falcon_flight: None,
            falcon_returning: false,
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

        if self.effect_state & OPTION_FALCON == 0 {
            // No falcon actor: stop a flight and park the shared animation
            // state so a later re-summon starts from the perched flap loop.
            self.falcon_flight = None;
            self.falcon_returning = false;
            self.falcon_animation_state.idle(EntityType::Npc, client_tick);
        }

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

    /// World position the falcon rests at while it is perched.
    ///
    /// The classic client keeps the falcon actor next to/above its owner
    /// (`AM_SET_MASTER_POINTER`, `MsgEffect.cpp:1660-1672`); this client draws
    /// it above the head with the same owner-relative offset every frame.
    pub(crate) fn falcon_perch_position(&self) -> Point3<f32> {
        self.world_position + Vector3::new(0.0, FALCON_HEIGHT * self.scale, 0.0)
    }

    /// Starts the falcon attack flight towards `target`.
    ///
    /// Source: `CGameActor::ProcessSkillMessage` sends `AM_REQ_ATTACK` with the
    /// target position to `statusEffects[0]` (`RenderObject.cpp:5388`) and
    /// `CMsgEffect::SendMsg` starts the flight (`MsgEffect.cpp:1611-1617`).
    /// The falcon has to be summoned (`OPTION_FALCON`) and its animation
    /// loaded; otherwise there is no actor to move.
    pub fn start_falcon_flight(&mut self, target: Point3<f32>, client_tick: ClientTick) {
        if !self.has_falcon() || self.falcon_animation.is_none() {
            return;
        }

        let perch = self.falcon_perch_position();
        let Some(flight) = FalconFlight::new(perch, target, client_tick) else {
            return;
        };

        self.falcon_flight = Some(flight);
        self.falcon_returning = false;
        // Actions 8-15 of the falcon ACT are the one-frame flight directions.
        self.falcon_animation_state
            .custom(EntityType::Npc, FALCON_ACTION_FLIGHT, 0, true, client_tick);
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
        crate::world::weapon::part_candidates(appearance_job, self.sex, self.weapon)
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
        crate::world::shield::part_candidates(appearance_job, self.sex, self.shield)
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
        crate::world::robe::part_candidates(appearance_job, self.sex, self.robe, None)
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
        if crate::world::weapon::animation_is_drawable(&animation_data) {
            self.weapon_animation = Some(animation_data);
        }
    }

    /// ACT block of this entity's attack swing.
    ///
    /// The kRO player sprite layout stores the weapon swing in either the
    /// `ST_ATTACK` block (80..87) or the `ST_ATTACK2` block (88..95); which one
    /// a weapon ACT animates depends on the job/weapon pair (for example knight
    /// swords use 80..87 while knight spears and hunter bows use 88..95). Pick
    /// the block the loaded weapon ACT actually draws, so the weapon layer does
    /// not drop out during the attack. Non-player entities keep the non-player
    /// layout (`Attack1` = block 2).
    pub fn attack_action(&self) -> crate::world::AnimationActionType {
        if !matches!(self.entity_type, EntityType::Player | EntityType::Hidden) {
            return crate::world::AnimationActionType::Attack1;
        }

        match self.weapon_animation.as_deref() {
            Some(animation) => crate::world::weapon::attack_action(animation),
            None => crate::world::AnimationActionType::Attack2,
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
        if crate::world::weapon::animation_is_drawable(&animation_data) {
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
        if crate::world::weapon::animation_is_drawable(&animation_data) {
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
        match crate::world::robe::profile(self.robe) {
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

            // A falcon attack flight takes over the perch loop: the flight
            // action is a one-frame glide, and the perched flap resumes for the
            // return leg (official `FollowOwner`) and after the flight.
            let perch = self.falcon_perch_position();
            let flight_pose = self.falcon_flight.as_ref().map(|flight| flight.sample(client_tick, perch));

            match flight_pose {
                Some(Some(pose)) if !pose.flying && !self.falcon_returning => {
                    self.falcon_returning = true;
                    self.falcon_animation_state.idle(EntityType::Npc, client_tick);
                }
                Some(None) => {
                    self.falcon_flight = None;
                    self.falcon_returning = false;
                    self.falcon_animation_state.idle(EntityType::Npc, client_tick);
                }
                _ => {}
            }
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

    pub(crate) fn set_position(&mut self, map: &Map, position: TilePosition, client_tick: ClientTick) {
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

        // Falcon companion: drawn above the owner while perched, or at its own
        // flight position while a falcon attack is playing. Only rendered while
        // the option bit is set.
        if self.has_falcon()
            && let Some(falcon_animation) = self.falcon_animation.as_ref()
        {
            let perch = self.falcon_perch_position();
            let (falcon_position, falcon_direction) = self
                .falcon_flight
                .as_ref()
                .and_then(|flight| flight.sample(client_tick, perch))
                .map(|pose| (pose.position, pose.direction))
                .unwrap_or((perch, self.direction));

            falcon_animation.render(
                instructions,
                camera,
                add_to_picker,
                self.entity_id,
                falcon_position,
                &self.falcon_animation_state,
                falcon_direction,
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
