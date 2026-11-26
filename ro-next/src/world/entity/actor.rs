//! The `Entity` actor enum (player or NPC) with its movement, rendering and
//! UI trait impls.

use super::*;

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

    /// Starts the official falcon attack flight towards `target`.
    ///
    /// Triggered from the skill damage packet for
    /// `IsFalconAttackSkill()` ids (Blitz Beat, Falcon Assault, hawk skills).
    pub fn start_falcon_flight(&mut self, target: Point3<f32>, client_tick: ClientTick) {
        self.get_common_mut().start_falcon_flight(target, client_tick);
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

    /// Records the `render.effectPhase` write of a ported effect body
    /// (`port/visual.rs`). Feeds [`Common::render_effect_phase`]; no client
    /// render pass reads it yet.
    pub fn set_render_effect_phase(&mut self, phase: Option<i16>) {
        self.get_common_mut().set_render_effect_phase(phase);
    }

    /// Records the `render.tintPhase` write of a ported effect body. Feeds
    /// [`Common::render_tint_phase`]; no client render pass reads it yet.
    pub fn set_render_tint_phase(&mut self, phase: Option<u8>) {
        self.get_common_mut().set_render_tint_phase(phase);
    }

    /// Records the `render.shrinkPhase` write of a ported effect body. Feeds
    /// [`Common::render_shrink_phase`]; no client render pass reads it yet.
    pub fn set_render_shrink_phase(&mut self, phase: Option<u8>) {
        self.get_common_mut().set_render_shrink_phase(phase);
    }

    /// Records the `render.outlineAlpha` write of a ported effect body. Feeds
    /// [`Common::render_outline_alpha`]; no client render pass reads it yet.
    pub fn set_render_outline_alpha(&mut self, alpha: Option<u8>) {
        self.get_common_mut().set_render_outline_alpha(alpha);
    }

    /// Records the `render.runStartTick` write of a ported effect body. Feeds
    /// [`Common::render_run_start_tick`]; no client render pass reads it yet.
    pub fn set_render_run_start_tick(&mut self, tick: Option<u8>) {
        self.get_common_mut().set_render_run_start_tick(tick);
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

    pub fn set_attack(&mut self, attack_duration: u32, _critical: bool, client_tick: ClientTick) {
        // Critical hits share the normal weapon swing in the official client;
        // only the damage number differs. The action block is chosen from the
        // loaded weapon ACT because the player attack blocks (80..87 and
        // 88..95) are job/weapon specific.
        let entity_type = self.get_entity_type();
        let common = self.get_common_mut();
        let action = common.attack_action();
        common.animation_state.attack(entity_type, attack_duration, action, client_tick);
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
