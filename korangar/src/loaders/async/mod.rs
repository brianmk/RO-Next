use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use hashbrown::{HashMap, HashSet};
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata, ShopItem};
use ragnarok_packets::{ClientTick, EntityId, ItemId, JobId, SkillId, TilePosition};
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::graphics::Texture;
use crate::init_tls_rand;
use crate::loaders::error::LoadError;
use crate::loaders::{
    ActionLoader, AnimationLoader, EntityPartFile, ImageType, MapLoader, ModelLoader, Sprite, SpriteLoader, TextureLoader, VideoLoader,
};
use crate::perf::{self, LoadKind};
use crate::state::skills::{LearnableSkill, SkillTabLayout, SkillTreeLayout};
#[cfg(feature = "debug")]
use crate::threads;
use crate::world::{
    Actions, AnimationData, EntityType, ItemResource, ItemResourceKey, ItemStats, Library, Map, ResourceMetadata, SkillListInformation,
    SkillListKey, SkillListRequirements, SpriteAnimationState, item_tooltip_text, resolve_card_names,
};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum LoaderId {
    AnimationData(EntityId),
    /// Idle animation preview of one character selection slot.
    ///
    /// Deliberately a separate id (instead of a synthetic entity id) so a
    /// preview load can never be mistaken for the animation data of a real
    /// entity and the completion handler can route it to the character
    /// selection state.
    CharacterPreview(usize),
    /// Animation of the hunter falcon that is attached to a player entity.
    FalconAnimationData(EntityId),
    /// Animation of the merchant cart attached to a player entity.
    CartAnimationData(EntityId),
    /// Animation of the equipped weapon that is drawn as its own layer on top
    /// of the player body animation.
    WeaponAnimationData(EntityId),
    ItemSprite(ItemId),
    Map(String),
    SkillSprite(SkillId),
    SkillActions(SkillId),
}

pub enum LoadableResource {
    AnimationData(Arc<AnimationData>),
    FalconAnimation(Arc<AnimationData>),
    CartAnimation(Arc<AnimationData>),
    WeaponAnimation(Arc<AnimationData>),
    ItemSprite { texture: Arc<Texture> },
    SkillSprite { sprite: Arc<Sprite> },
    SkillActions { actions: Arc<Actions> },
    Map { map: Arc<Map>, position: Option<TilePosition> },
}

/// The outcome of an asynchronous load request.
///
/// Failed loads used to be silently dropped, which left the corresponding
/// resource missing forever (for example a missing map kept the client on a
/// black screen because `Client::map` never got set). The caller now receives
/// the error and can react to it.
pub enum LoadResult {
    Completed { id: LoaderId, resource: LoadableResource },
    Failed { id: LoaderId, error: LoadError },
}

/// Shared bookkeeping of the async loader workers.
///
/// * `requested` deduplicates load requests: an identical `LoaderId` is only
///   queued once, no matter how many frames ask for it while it is in flight.
/// * `completed` is a FIFO of finished loads pushed by the workers, so the
///   client drains results in completion order instead of scanning the whole
///   pending map every frame.
/// * `animation_in_flight` coalesces animation loads by *resource* (the
///   composed body/head/headgear part list). Population-engine crowds are made
///   of many characters that share the same appearance, so without this every
///   shell would start an identical, expensive sprite/animation load.
#[derive(Default)]
struct PendingLoads {
    requested: HashSet<LoaderId>,
    completed: VecDeque<LoadResult>,
    animation_in_flight: HashMap<Vec<EntityPartFile>, Vec<LoaderId>>,
}

pub struct AsyncLoader {
    action_loader: Arc<ActionLoader>,
    animation_loader: Arc<AnimationLoader>,
    map_loader: Arc<MapLoader>,
    model_loader: Arc<ModelLoader>,
    sprite_loader: Arc<SpriteLoader>,
    texture_loader: Arc<TextureLoader>,
    video_loader: Arc<VideoLoader>,
    library: Arc<Library>,
    pending_loads: Arc<Mutex<PendingLoads>>,
    /// Worker lane for map loads. Map loads are big and block map entry, so
    /// they must never wait behind a burst of sprite/animation loads.
    map_thread_pool: ThreadPool,
    /// Worker lane for every other load (entity animations, item/skill sprites,
    /// ...).
    visible_thread_pool: ThreadPool,
    /// Candidate paths of weapon animations that are known to be missing from
    /// the archives.
    ///
    /// CondaRO's archives do not contain every per-job weapon sprite variant
    /// (e.g. the Sniper bow). Without this cache every population-engine shell
    /// spawn would queue another archive scan on the two loader threads for a
    /// sprite that can never be found, delaying the loads the client actually
    /// needs right after map entry.
    failed_weapon_candidates: Arc<Mutex<HashSet<String>>>,
}

impl AsyncLoader {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action_loader: Arc<ActionLoader>,
        animation_loader: Arc<AnimationLoader>,
        map_loader: Arc<MapLoader>,
        model_loader: Arc<ModelLoader>,
        sprite_loader: Arc<SpriteLoader>,
        texture_loader: Arc<TextureLoader>,
        video_loader: Arc<VideoLoader>,
        library: Arc<Library>,
    ) -> Self {
        // Two lanes: a dedicated map loader thread (map entry latency must not
        // depend on the sprite queue) and a small pool for everything else.
        let map_thread_pool = ThreadPoolBuilder::new()
            .thread_name(|number| format!("korangar map loader {number}"))
            .num_threads(1)
            .start_handler(|_| init_tls_rand())
            .build()
            .unwrap();

        let visible_thread_pool = ThreadPoolBuilder::new()
            .thread_name(|number| format!("korangar loader {number}"))
            .num_threads(2)
            .start_handler(|_| init_tls_rand())
            .build()
            .unwrap();

        Self {
            failed_weapon_candidates: Arc::new(Mutex::new(HashSet::new())),
            action_loader,
            animation_loader,
            map_loader,
            model_loader,
            sprite_loader,
            texture_loader,
            video_loader,
            library,
            pending_loads: Arc::new(Mutex::new(PendingLoads::default())),
            map_thread_pool,
            visible_thread_pool,
        }
    }

    /// Requests the composed idle animation of a character selection slot.
    ///
    /// This uses the same loader and cache as the entity animations, so the
    /// preview of a character that is already known as an entity (for example
    /// after returning to the selection screen) is served from the cache.
    #[must_use]
    pub fn request_character_preview_load(&self, slot: usize, entity_part_files: Vec<EntityPartFile>) -> Option<Arc<AnimationData>> {
        self.request_animation_load(LoaderId::CharacterPreview(slot), EntityType::Player, entity_part_files)
    }

    #[must_use]
    pub fn request_animation_data_load(
        &self,
        entity_id: EntityId,
        entity_type: EntityType,
        entity_part_files: Vec<EntityPartFile>,
    ) -> Option<Arc<AnimationData>> {
        self.request_animation_load(LoaderId::AnimationData(entity_id), entity_type, entity_part_files)
    }

    /// Shared implementation of the animation data load requests.
    fn request_animation_load(
        &self,
        id: LoaderId,
        entity_type: EntityType,
        entity_part_files: Vec<EntityPartFile>,
    ) -> Option<Arc<AnimationData>> {
        // Entities without sprite files (warps and hidden NPCs) are not drawn from an
        // animation. Loading an empty list would make the animation loader fall back to
        // the black "missing" sprite, so we skip the load entirely.
        if entity_part_files.is_empty() {
            return None;
        }

        match self.animation_loader.get(&entity_part_files) {
            Some(animation_data) => Some(animation_data),
            None => {
                let resource_key = entity_part_files.clone();

                {
                    let mut pending = self.pending_loads.lock().unwrap();

                    // The same appearance is already being loaded for another
                    // entity: wait for that load instead of starting a duplicate
                    // one. The worker delivers the result to every waiter.
                    if let Some(waiters) = pending.animation_in_flight.get_mut(&resource_key) {
                        waiters.push(id.clone());
                        pending.requested.insert(id);
                        perf::load_coalesced(LoadKind::AnimationData);
                        return None;
                    }

                    if pending.requested.contains(&id) {
                        return None;
                    }

                    pending.requested.insert(id.clone());
                    pending.animation_in_flight.insert(resource_key.clone(), vec![id]);
                }

                let sprite_loader = self.sprite_loader.clone();
                let action_loader = self.action_loader.clone();
                let animation_loader = self.animation_loader.clone();
                let pending_loads = Arc::clone(&self.pending_loads);
                let requested_at = Instant::now();

                perf::load_submitted(LoadKind::AnimationData);

                self.visible_thread_pool.spawn(move || {
                    #[cfg(feature = "debug")]
                    let _measurement = threads::Loader::start_frame();

                    let started = Instant::now();
                    perf::load_queue_latency(
                        LoadKind::AnimationData,
                        started.saturating_duration_since(requested_at).as_nanos() as u64,
                    );

                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("animation data load");

                    let result = match animation_loader.get(&entity_part_files) {
                        Some(animation_data) => Ok(animation_data),
                        None => animation_loader.load(&sprite_loader, &action_loader, entity_type, &entity_part_files),
                    };

                    let service_nanos = started.elapsed().as_nanos() as u64;
                    perf::load_service_latency(LoadKind::AnimationData, service_nanos, result.is_err());

                    let mut pending = pending_loads.lock().unwrap();
                    let waiters = pending.animation_in_flight.remove(&resource_key).unwrap_or_default();

                    match result {
                        Ok(animation_data) => {
                            for waiter in waiters {
                                pending.requested.remove(&waiter);
                                pending.completed.push_back(LoadResult::Completed {
                                    id: waiter,
                                    resource: LoadableResource::AnimationData(animation_data.clone()),
                                });
                            }
                        }
                        Err(error) => {
                            // `LoadError` is not `Clone`, so the waiters all get a
                            // diagnostic message with the underlying error.
                            let message = format!("{error:?}");

                            for waiter in waiters {
                                pending.requested.remove(&waiter);
                                pending.completed.push_back(LoadResult::Failed {
                                    id: waiter,
                                    error: LoadError::UnsupportedFormat(message.clone()),
                                });
                            }
                        }
                    }
                });

                None
            }
        }
    }

    /// Requests the hunter falcon animation for `entity_id`.
    ///
    /// The trimmed archive stores the falcon sprite and action under different
    /// stems (`ht_falcon.spr` + `h_falcon.act`), so the first candidate uses an
    /// explicit action-path override. Further candidates cover archives that
    /// have the regular same-stem pair or the soul-falcon effect sprite.
    #[must_use]
    pub fn request_falcon_animation_data_load(&self, entity_id: EntityId) -> Option<Arc<AnimationData>> {
        // The CondaRO archives store the official hunter falcon as
        // `아이템\ht_falcon.spr` + `아이템\h_falcon.act` (the sprite and action
        // stems differ in this trimmed data). `soul_falcon`, `sp_soulfalcon`
        // and `sn_falconassault` are *effect* sprites (Soul Linker / Falcon
        // Assault), not the companion falcon, so they are only fallbacks. The
        // candidates are tried in this fixed order: picking the sprite with the
        // most drawable frames (the old heuristic) selected the Soul Linker
        // effect instead of the hunter falcon.
        // The official hunter falcon companion is the effect sprite
        // `data\sprite\이팩트\매` (Korean "매" = falcon; `매2` is the
        // alternate variant). The `이팩트\soul_falcon` and `아이템\*falcon*`
        // sprites are Soul Linker / Falcon Assault effects.
        let candidates: Vec<Vec<EntityPartFile>> = vec![
            vec![EntityPartFile::new("이팩트\\매")],
            vec![EntityPartFile::new("이팩트\\매2")],
            vec![EntityPartFile::with_action("아이템\\ht_falcon", "아이템\\h_falcon")],
            vec![EntityPartFile::new("아이템\\ht_falcon")],
            vec![EntityPartFile::new("이팩트\\soul_falcon\\soul_falcon")],
            vec![EntityPartFile::new("아이템\\sp_soulfalcon")],
            vec![EntityPartFile::new("아이템\\sn_falconassault")],
        ];

        fn animated_part_score(animation_data: &AnimationData) -> usize {
            animation_data
                .animations
                .iter()
                .take(8)
                .flat_map(|animation| animation.frames.iter())
                .map(|frame| frame.frame_parts.iter().filter(|part| part.sprite_number != usize::MAX).count())
                .sum()
        }

        for parts in &candidates {
            if let Some(animation_data) = self.animation_loader.get(parts) {
                crate::logging::log_debug!(
                    "falcon animation: cache hit for entity {} with {:?} (animated score {})",
                    entity_id.0,
                    parts.first().map(|part| part.path.as_str()),
                    animated_part_score(&animation_data)
                );
                return Some(animation_data);
            }
        }

        crate::logging::log_debug!("falcon animation: requesting load for entity {}", entity_id.0);

        let sprite_loader = self.sprite_loader.clone();
        let action_loader = self.action_loader.clone();
        let animation_loader = self.animation_loader.clone();

        self.request_visible_load(LoaderId::FalconAnimationData(entity_id), LoadKind::AnimationData, move || {
            let mut last_error = LoadError::UnsupportedFormat("no falcon sprite found in the game archives".to_owned());

            // First candidate in priority order = the official hunter falcon.
            for parts in &candidates {
                match animation_loader.load(&sprite_loader, &action_loader, EntityType::Npc, parts) {
                    Ok(animation_data) => {
                        crate::logging::log_debug!(
                            "falcon candidate {:?} loaded: {} pair(s), {} animation(s), animated score {} (selected)",
                            parts.first().map(|part| part.path.as_str()),
                            animation_data.animation_pair.len(),
                            animation_data.animations.len(),
                            animated_part_score(&animation_data)
                        );
                        return Ok(LoadableResource::FalconAnimation(animation_data));
                    }
                    Err(error) => {
                        crate::logging::log_debug!(
                            "falcon candidate {:?} unavailable: {error:?}",
                            parts.first().map(|part| part.path.as_str())
                        );
                        last_error = error;
                    }
                }
            }

            Err(last_error)
        });

        None
    }

    /// Requests the merchant cart animation for `entity_id`.
    ///
    /// The classic pushcart sprite is `data\sprite\아이템\mc_pushcart.spr`;
    /// the cart is drawn with the body animation state so it follows idle,
    /// walk and attack frames. Cart tiers (`OPTION_CART2..5`) use the
    /// remodeled sprites when present.
    #[must_use]
    pub fn request_cart_animation_data_load(&self, entity_id: EntityId) -> Option<Arc<AnimationData>> {
        let candidates: Vec<Vec<EntityPartFile>> = vec![
            vec![EntityPartFile::new("아이템\\mc_pushcart")],
            vec![EntityPartFile::new("아이템\\mc_changecart")],
            vec![EntityPartFile::new("아이템\\mc_cartdecorate")],
        ];

        for parts in &candidates {
            if let Some(animation_data) = self.animation_loader.get(parts) {
                return Some(animation_data);
            }
        }

        let sprite_loader = self.sprite_loader.clone();
        let action_loader = self.action_loader.clone();
        let animation_loader = self.animation_loader.clone();

        self.request_visible_load(LoaderId::CartAnimationData(entity_id), LoadKind::AnimationData, move || {
            let mut last_error = LoadError::UnsupportedFormat("no cart sprite found in the game archives".to_owned());
            for parts in &candidates {
                match animation_loader.load(&sprite_loader, &action_loader, EntityType::Npc, parts) {
                    Ok(animation_data) => return Ok(LoadableResource::CartAnimation(animation_data)),
                    Err(error) => last_error = error,
                }
            }
            Err(last_error)
        });

        None
    }

    /// Requests the weapon animation for `entity_id`.
    ///
    /// The candidate paths are built by the data-driven weapon registry
    /// ([`crate::world::weapon::part_candidates`]): per-job weapon variants
    /// next to the body sprite
    /// (`data\sprite\인간족\<job>\<job>_<sex>_<variant>`) first, then the
    /// classic job-independent weapon layer. The weapon is drawn as its own
    /// animation layer using the body animation state, so its
    /// frames/offsets are not affected by the body/head composition.
    #[must_use]
    pub fn request_weapon_animation_data_load(
        &self,
        entity_id: EntityId,
        weapon_part_candidates: Vec<EntityPartFile>,
    ) -> Option<Arc<AnimationData>> {
        if weapon_part_candidates.is_empty() {
            return None;
        }

        // Remember the first candidate path as the identity of the
        // (job, sex, weapon) combination: all candidates of one entity share
        // the same prefix and the archives are static, so a failed lookup can
        // be cached forever.
        let cache_key = weapon_part_candidates[0].path.clone();
        if self.failed_weapon_candidates.lock().unwrap().contains(&cache_key) {
            return None;
        }

        if let Some(animation_data) = weapon_part_candidates.iter().find_map(|parts| {
            self.animation_loader
                .get(std::slice::from_ref(parts))
                .filter(|animation_data| crate::world::weapon::animation_is_drawable(animation_data))
        }) {
            return Some(animation_data);
        }

        let sprite_loader = self.sprite_loader.clone();
        let action_loader = self.action_loader.clone();
        let animation_loader = self.animation_loader.clone();
        let failed_weapon_candidates = Arc::clone(&self.failed_weapon_candidates);

        self.request_visible_load(LoaderId::WeaponAnimationData(entity_id), LoadKind::AnimationData, move || {
            let mut last_error = LoadError::UnsupportedFormat("no weapon sprite found in the game archives".to_owned());

            for part in &weapon_part_candidates {
                let parts = std::slice::from_ref(part);
                if !(sprite_loader.file_exists(&format!("{}.spr", part.path)) && action_loader.file_exists(&format!("{}.act", part.path))) {
                    continue;
                }

                match animation_loader.load(&sprite_loader, &action_loader, EntityType::Player, parts) {
                    Ok(animation_data) => {
                        // A trimmed data set can contain numeric per-job weapon
                        // ACTs whose clips are all hidden placeholders (notably
                        // bows: `궁수_남_1701` exists but draws nothing). Keep
                        // trying the later classic-variant candidates instead of
                        // dropping the whole weapon layer.
                        if crate::world::weapon::animation_is_drawable(&animation_data) {
                            return Ok(LoadableResource::WeaponAnimation(animation_data));
                        }
                        crate::logging::log_debug!(
                            "weapon candidate {:?} has no drawable frames, trying the next candidate",
                            part.path
                        );
                    }
                    Err(error) => last_error = error,
                }
            }

            // No candidate could be loaded: remember the combination so the
            // next entity with the same job/sex/weapon does not scan the
            // archives again.
            failed_weapon_candidates.lock().unwrap().insert(cache_key.clone());
            crate::logging::log_debug!("weapon animation not in the archives, disabling the weapon layer for `{cache_key}`");

            Err(last_error)
        });

        None
    }

    #[must_use]
    pub fn request_skill_sprite_load(&self, skill_id: SkillId, path: &str) -> Option<Arc<Sprite>> {
        let sprite_path = format!("{path}.spr");

        match self.sprite_loader.get(&sprite_path, None) {
            Some(sprite) => Some(sprite),
            None => {
                let sprite_loader = self.sprite_loader.clone();

                self.request_visible_load(LoaderId::SkillSprite(skill_id), LoadKind::SkillSprite, move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("skill sprite load");

                    let sprite = match sprite_loader.get(&sprite_path, None) {
                        Some(sprite) => sprite,
                        None => sprite_loader.load(&sprite_path, None)?,
                    };
                    Ok(LoadableResource::SkillSprite { sprite })
                });

                None
            }
        }
    }

    #[must_use]
    pub fn request_skill_actions_load(&self, skill_id: SkillId, path: &str) -> Option<Arc<Actions>> {
        let actions_path = format!("{path}.act");

        match self.action_loader.get(&actions_path) {
            Some(actions) => Some(actions),
            None => {
                let action_loader = self.action_loader.clone();

                self.request_visible_load(LoaderId::SkillActions(skill_id), LoadKind::SkillActions, move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("skill actions load");

                    let actions = match action_loader.get(&actions_path) {
                        Some(actions) => actions,
                        None => action_loader.load(&actions_path)?,
                    };
                    Ok(LoadableResource::SkillActions { actions })
                });

                None
            }
        }
    }

    #[must_use]
    pub fn request_item_sprite_load(&self, item_id: ItemId, path: &str, image_type: ImageType) -> Option<Arc<Texture>> {
        match self.texture_loader.get(path, image_type) {
            Some(texture) => Some(texture),
            None => {
                let texture_loader = self.texture_loader.clone();
                let path = path.to_string();

                self.request_visible_load(LoaderId::ItemSprite(item_id), LoadKind::ItemSprite, move || {
                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("item sprite load");

                    let texture = match texture_loader.get(&path, image_type) {
                        None => texture_loader.load(&path, image_type)?,
                        Some(texture) => texture,
                    };
                    Ok(LoadableResource::ItemSprite { texture })
                });

                None
            }
        }
    }

    pub fn request_inventory_item_metadata_load(&self, item: InventoryItem<NoMetadata>) -> InventoryItem<ResourceMetadata> {
        let is_identified = item.is_identified();

        let resource_name = self.library.get::<ItemResource>(ItemResourceKey {
            item_id: item.item_id,
            is_identified,
        });
        let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
        let texture = self.request_item_sprite_load(item.item_id, &full_path, ImageType::Color);
        let stats = *self.library.get::<ItemStats>(item.item_id);
        let description = self.library.item_description(item.item_id, is_identified);
        let slot_count = self.library.item_slot_count(item.item_id);
        let refinement_level = match &item.details {
            InventoryItemDetails::Equippable { refinement_level, .. } => *refinement_level,
            InventoryItemDetails::Regular { .. } => 0,
        };
        // The displayed name composes the refinement level and the slotted
        // cards, like the official client. Cards and random options are not
        // part of the item description; the card names have to be resolved
        // here, where the item data is available.
        let name = self
            .library
            .composed_item_name(item.item_id, is_identified, refinement_level, &item.slot);
        let card_names = resolve_card_names(&self.library, &item);

        let metadata = ResourceMetadata {
            texture,
            name,
            stats,
            description,
            card_names,
            slot_count,
            tooltip_text: Default::default(),
        };

        // Compose the tooltip once; the interface only clones the cache.
        let mut item = InventoryItem { metadata, ..item };
        item.metadata.tooltip_text = item_tooltip_text(&item).into();
        item
    }

    pub fn request_shop_item_metadata_load(&self, item: ShopItem<NoMetadata>) -> ShopItem<ResourceMetadata> {
        let resource_name = self.library.get::<ItemResource>(ItemResourceKey {
            item_id: item.item_id,
            is_identified: true,
        });
        let full_path = format!("유저인터페이스\\item\\{resource_name}.bmp");
        let texture = self.request_item_sprite_load(item.item_id, &full_path, ImageType::Color);
        let stats = *self.library.get::<ItemStats>(item.item_id);
        let description = self.library.item_description(item.item_id, true);
        let slot_count = self.library.item_slot_count(item.item_id);
        // Shop items are plain item records without cards or refinement.
        let name = self.library.composed_item_name(item.item_id, true, 0, &[0; 4]);

        let metadata = ResourceMetadata {
            texture,
            name: name.clone(),
            stats,
            description,
            card_names: std::array::from_fn(|_| None),
            slot_count,
            tooltip_text: name.into(),
        };

        ShopItem { metadata, ..item }
    }

    pub fn request_learnable_skill_load(&self, job_id: JobId, skill_id: SkillId, client_tick: ClientTick) -> LearnableSkill {
        let skill_information = self.library.get::<SkillListInformation>(skill_id);
        let skill_requirements = self.library.get::<SkillListRequirements>(SkillListKey::with_job(job_id, skill_id));

        // The icon set of the data set is trimmed; the library resolves a verified
        // exact/alias/generic icon instead of letting the sprite loader fall back to
        // the black `missing.spr` placeholder (see `world/library/skill_icon.rs`).
        let path = self.library.skill_icon_path(skill_id).to_string();
        let sprite = self.request_skill_sprite_load(skill_id, &path);
        let actions = self.request_skill_actions_load(skill_id, &path);

        LearnableSkill {
            skill_id,
            maximum_level: skill_information.maximum_level,
            file_name: skill_information.file_name.clone(),
            skill_name: skill_information.name.clone(),
            can_select_level: skill_information.can_select_level,
            acquisition: skill_information.acquisition,
            required_skills: skill_requirements.required_skills.clone(),
            required_for_skills: skill_requirements.required_for_skills.clone(),
            sprite,
            actions,
            animation_state: SpriteAnimationState::new(client_tick),
        }
    }

    pub fn request_map_load(&self, map_name: String, position: Option<TilePosition>) {
        let id = LoaderId::Map(map_name.clone());

        match self.map_loader.get(&map_name) {
            Some(map) => {
                let mut pending = self.pending_loads.lock().unwrap();

                if pending.requested.contains(&id) {
                    perf::load_coalesced(LoadKind::Map);
                    return;
                }

                pending.requested.insert(id.clone());
                pending.completed.push_back(LoadResult::Completed {
                    id,
                    resource: LoadableResource::Map { map, position },
                });
            }
            None => {
                {
                    let mut pending = self.pending_loads.lock().unwrap();

                    if pending.requested.contains(&id) {
                        perf::load_coalesced(LoadKind::Map);
                        return;
                    }

                    pending.requested.insert(id.clone());
                }

                let map_loader = self.map_loader.clone();
                let model_loader = self.model_loader.clone();
                let texture_loader = self.texture_loader.clone();
                let video_loader = self.video_loader.clone();
                let library = self.library.clone();
                let pending_loads = Arc::clone(&self.pending_loads);
                let requested_at = Instant::now();

                perf::load_submitted(LoadKind::Map);

                // Map loads run on their own lane so they never wait behind a
                // sprite/animation flood.
                self.map_thread_pool.spawn(move || {
                    #[cfg(feature = "debug")]
                    let _measurement = threads::Loader::start_frame();

                    let started = Instant::now();
                    perf::load_queue_latency(LoadKind::Map, started.saturating_duration_since(requested_at).as_nanos() as u64);

                    #[cfg(feature = "debug")]
                    let _load_measurement = Profiler::start_measurement("map load");

                    let result = map_loader.load(map_name, &model_loader, texture_loader, video_loader.clone(), &library);
                    perf::load_service_latency(LoadKind::Map, started.elapsed().as_nanos() as u64, result.is_err());

                    let mut pending = pending_loads.lock().unwrap();
                    pending.requested.remove(&id);

                    match result {
                        Ok(map) => pending.completed.push_back(LoadResult::Completed {
                            id,
                            resource: LoadableResource::Map { map, position },
                        }),
                        Err(error) => pending.completed.push_back(LoadResult::Failed { id, error }),
                    }
                });
            }
        }
    }

    pub fn request_skill_tree_layout_load(&self, job_id: JobId, client_tick: ClientTick) -> SkillTreeLayout {
        // Not every job id the server sends has an entry in the client's skill tree
        // data: the population engine's ambient shells change to arbitrary jobs (for
        // example 4th jobs) that this pre-renewal client does not know. Those must
        // not crash the client. `resolve` also applies the generated fallback table
        // (high/expanded jobs map to the tree of their base job) instead of letting
        // those characters end up with a completely empty skill tree.
        let Some(layout) = crate::world::SkillTreeLayout::resolve(&self.library, job_id) else {
            return SkillTreeLayout { tabs: Vec::new() };
        };

        let tabs = layout
            .tabs
            .iter()
            .map(|tab| {
                let name = tab.name.clone();
                let skills = HashMap::from_iter(
                    tab.skills
                        .iter()
                        .map(|(slot, skill_id)| (*slot, self.request_learnable_skill_load(job_id, *skill_id, client_tick))),
                );

                SkillTabLayout { name, skills }
            })
            .collect();

        SkillTreeLayout { tabs }
    }

    /// Queue a non-map load on the visible-resource lane.
    ///
    /// Requests for an identical [`LoaderId`] that is already in flight are
    /// dropped: loading the same resource twice would only duplicate work and
    /// delay the rest of the queue.
    fn request_visible_load<F>(&self, id: LoaderId, kind: LoadKind, load_function: F)
    where
        F: FnOnce() -> Result<LoadableResource, LoadError> + Send + 'static,
    {
        {
            let mut pending = self.pending_loads.lock().unwrap();

            if pending.requested.contains(&id) {
                perf::load_coalesced(kind);
                return;
            }

            pending.requested.insert(id.clone());
        }

        let pending_loads = Arc::clone(&self.pending_loads);
        let requested_at = Instant::now();

        perf::load_submitted(kind);

        self.visible_thread_pool.spawn(move || {
            #[cfg(feature = "debug")]
            let _measurement = threads::Loader::start_frame();

            let started = Instant::now();
            perf::load_queue_latency(kind, started.saturating_duration_since(requested_at).as_nanos() as u64);

            let result = load_function();
            perf::load_service_latency(kind, started.elapsed().as_nanos() as u64, result.is_err());

            let mut pending = pending_loads.lock().unwrap();
            pending.requested.remove(&id);

            match result {
                Ok(resource) => pending.completed.push_back(LoadResult::Completed { id, resource }),
                Err(error) => pending.completed.push_back(LoadResult::Failed { id, error }),
            }
        });
    }

    /// Take all finished loads, in completion order.
    ///
    /// Workers push into a queue instead of the caller scanning the pending map
    /// on every frame; the client drains this iterator once per frame.
    pub fn take_completed(&self) -> impl Iterator<Item = LoadResult> + '_ {
        std::iter::from_fn({
            let pending_loads = Arc::clone(&self.pending_loads);

            move || {
                let mut pending = pending_loads.lock().unwrap();

                let result = pending.completed.pop_front()?;

                match &result {
                    LoadResult::Completed { id, .. } | LoadResult::Failed { id, .. } => {
                        pending.requested.remove(id);
                    }
                }

                Some(result)
            }
        })
    }
}
