//! The map-server event dispatcher: one arm per `NetworkEvent`.
//!
//! Kept apart from `lib.rs` because it is the largest single function of the
//! client; the per-family handlers are being moved out of the `match`
//! incrementally.
use crate::*;

impl Client {
    pub(crate) fn handle_network_events(&mut self, client_tick: ClientTick) {
        self.networking_system.get_events(&mut self.network_event_buffer);

        // Collect the events first so handlers are free to take `&mut self`
        // (the drain iterator borrows the event buffer).
        let network_events: Vec<NetworkEvent> = self.network_event_buffer.drain().collect();

        if !network_events.is_empty() {
            // Network events change UI state (chat, inventory, stats, ...).
            self.interface.invalidate_layouts();
        }

        for event in network_events {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    self.play_main_menu_click_sound();

                    // Remove `_m`/`_f` suffix from the username. The suffix is only for *creating*
                    // an account and thus can (and needs to) be removed after the first successful
                    // login.
                    {
                        let selected_service_path =
                            SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                        let username_path = selected_service_path.username();

                        let username = self.client_state.follow_mut(username_path);

                        if let Some(stripped) = username.strip_suffix("_m") {
                            *username = stripped.to_owned();
                        } else if let Some(stripped) = username.strip_suffix("_f") {
                            *username = stripped.to_owned();
                        }
                    }

                    self.saved_login_data = Some(login_data);
                    self.session.transition(SessionEvent::LoginSucceeded);

                    // A previously established character/map connection (e.g. from a session
                    // that was kicked or logged out) must not block the new server selection.
                    // Only disconnect connections that actually exist: disconnecting an idle
                    // connection emits a synthetic "closed by client" event, which would make
                    // the disconnect handler close the server selection window we are about to
                    // open and bounce the player back to the login screen.
                    if self.networking_system.is_character_server_connected() {
                        self.networking_system.disconnect_from_character_server();
                    }

                    if self.networking_system.is_map_server_connected() {
                        self.networking_system.disconnect_from_map_server();
                    }

                    // A fresh login must not auto-reconnect to a previously selected character
                    // server (map-restart recovery state): reset the retry state and the
                    // remembered server so the Select Server window is shown and only the
                    // player's click connects.
                    self.session.character_server_disconnected();
                    self.session.clear_character_reconnect();
                    self.saved_character_server = None;

                    *self.client_state.follow_mut(client_state().character_servers()) = character_servers;

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.interface
                        .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_login_server();

                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::InternalError { message } => {
                    // A client build/setup bug (e.g. duplicate packet handler).
                    // Log it as an error and show the message, never panic.
                    crate::logging::log_error!("internal client error: {message}");
                    self.interface.open_window(ErrorWindow::new(message));
                }
                NetworkEvent::LoginServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let Some(socket_address) = self.saved_login_server_address else {
                            // No address means the user never completed the login
                            // flow; nothing to reconnect to.
                            continue;
                        };
                        self.networking_system.connect_to_login_server(
                            self.saved_packet_version,
                            socket_address,
                            &self.saved_username,
                            &self.saved_password,
                        );
                    }
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    crate::logging::log_debug!(
                        "character server: connected ({} slots), requesting the character list",
                        normal_slot_count
                    );
                    self.session.character_server_connected();
                    if self.session.state() == SessionState::Recovering {
                        self.session.transition(SessionEvent::CharacterServerConnected);
                    }

                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_slot_count(normal_slot_count);

                    let _ = self.networking_system.request_character_list();
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    self.session.character_server_disconnected();
                    self.networking_system.disconnect_from_character_server();

                    if self.map.is_none() && self.saved_login_data.is_some() {
                        // The character server can be briefly unavailable (e.g. while the map
                        // server restarts and the old map session is still registered).
                        // Retry instead of ending up on a dead screen.
                        self.session.schedule_character_reconnect(Instant::now());
                    } else {
                        self.interface.open_window(ErrorWindow::new(message.to_owned()));
                    }
                }
                NetworkEvent::CharacterServerDisconnected { reason } => {
                    self.session.character_server_disconnected();

                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        self.session.schedule_character_reconnect(Instant::now());
                    } else if !self.networking_system.is_map_server_connected()
                        && !self.networking_system.is_login_server_connected()
                        && !self.interface.is_window_with_class_open(WindowClass::SelectServer)
                    {
                        // Only fall back to the login screen when the character server closed
                        // while we are not in the middle of the login/server-selection flow.
                        // During login the character server connection is intentionally dropped
                        // (or never existed); a "closed by client" event must not wipe the
                        // server selection window that was just opened.
                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(LoginWindow::new(
                            client_state().login_window(),
                            client_state().login_settings(),
                            client_state().client_info(),
                        ));

                        self.audio_engine
                            .play_background_music_track(crate::client_config::background_music());
                    }
                }
                NetworkEvent::MapServerDisconnected { reason } => {
                    // Guild emblems are only valid for the map session; the server may
                    // have changed them and the rate limit should be reset as well.
                    self.guild_emblems.clear();
                    // Status effects belong to the map session as well.
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.client_state.follow_mut(client_state().spirit_orbs()).clear();
                    self.client_state.follow_mut(client_state().mail()).clear();
                    // The guild window data is only valid for this map session.
                    self.client_state.follow_mut(client_state().guild()).reset();
                    // Party-booking results, clan info and companion sheets are
                    // map-session data too.
                    self.client_state.follow_mut(client_state().party_booking()).clear();
                    self.client_state.follow_mut(client_state().companion()).clear();

                    if self.session.state() == SessionState::Shutdown {
                        // We are quitting; the clean logout is done, don't go back to
                        // the character selection screen.
                        continue;
                    }

                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the map server with error");
                    }

                    self.session.map_server_disconnected();
                    self.return_to_character_selection();
                }
                NetworkEvent::InitialStats {
                    strength_stat_points_cost,
                    agility_stat_points_cost,
                    vitality_stat_points_cost,
                    intelligence_stat_points_cost,
                    dexterity_stat_points_cost,
                    luck_stat_points_cost,
                    attack,
                    bonus_attack,
                    magic_attack_max,
                    magic_attack_min,
                    defense,
                    bonus_defense,
                    magic_defense,
                    bonus_magic_defense,
                    hit,
                    flee,
                    perfect_dodge,
                    critical,
                    attack_speed,
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.strength_stat_points_cost = strength_stat_points_cost;
                        player.agility_stat_points_cost = agility_stat_points_cost;
                        player.vitality_stat_points_cost = vitality_stat_points_cost;
                        player.intelligence_stat_points_cost = intelligence_stat_points_cost;
                        player.dexterity_stat_points_cost = dexterity_stat_points_cost;
                        player.luck_stat_points_cost = luck_stat_points_cost;
                        player.attack = attack as u32;
                        player.bonus_attack = bonus_attack as u32;
                        player.magic_attack_max = magic_attack_max as u32;
                        player.magic_attack_min = magic_attack_min as u32;
                        player.defense = defense as u32;
                        player.bonus_defense = bonus_defense as u32;
                        player.magic_defense = magic_defense as u32;
                        player.bonus_magic_defense = bonus_magic_defense as u32;
                        player.hit = hit as u32;
                        player.flee = flee as u32;
                        player.perfect_dodge = perfect_dodge as u32;
                        player.critical = critical as u32;
                        player.attack_speed = attack_speed as u32;
                    }
                }
                NetworkEvent::ResurrectPlayer { entity_id } => {
                    self.entity_lifecycle.on_resurrect(entity_id);

                    // The server only revives the character, it does not send a separate
                    // stand-up packet, so restore the idle animation ourselves. Without this
                    // the player stayed in the dead pose after respawning.
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_idle(client_tick);
                    }

                    // If the resurrected player is us, close the resurrect window.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == entity_id)
                    {
                        self.interface.close_window_with_class(WindowClass::Respawn);
                    }
                }
                NetworkEvent::PlayerStandUp { entity_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_standing(client_tick);
                    }
                }
                NetworkEvent::PlayerSitDown { entity_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_sitting(client_tick);
                    }
                }
                NetworkEvent::SkillCast {
                    source_entity_id,
                    destination_entity_id,
                    position,
                    skill_id,
                    element,
                    cast_time,
                    attack_motion_time,
                } => {
                    // Other clients show the caster's skill-cast action for the cast
                    // duration and a ground cast circle below the caster. The 2019
                    // server does not send the animation separately, so this is the main
                    // cue for ambient self-buffs, crafting, warp portal casts and the
                    // skills of other players.
                    let has_cast_time = cast_time > 0;
                    // The server packet's cast time is authoritative. The
                    // per-skill table only provides a lower bound for the
                    // animation/pose duration when the packet has no usable
                    // timing (the old code used one global 300 ms constant).
                    let table_delays = skill_delays(skill_id, 1);
                    let animation_minimum = table_delays
                        .map(|delays| delays.total_cast_ms().max(delays.aftercast_act_ms))
                        .unwrap_or(0);
                    let cast_animation_time = cast_time.max(attack_motion_time).max(animation_minimum);
                    // The cast circle tint is data-driven (a per-skill override in the
                    // unified skill effect descriptors) and falls back to the element
                    // color of the `ZC_USESKILL_ACK` packet.
                    let cast_circle_color =
                        crate::world::skill_cast_circle_color(skill_id).unwrap_or_else(|| crate::world::element_color(element));

                    if let Some(delays) = table_delays {
                        crate::logging::log_debug!(
                            "skill delay table: skill={} cast={}ms fixed={}ms aftercast_act={}ms aftercast_walk={}ms cooldown={}ms",
                            skill_id.0,
                            delays.cast_ms,
                            delays.fixed_cast_ms,
                            delays.aftercast_act_ms,
                            delays.aftercast_walk_ms,
                            delays.cooldown_ms
                        );
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        match caster_action(skill_id) {
                            // Stance/buff skills (Zen, Fury, Endure, ...) keep the idle
                            // pose in the official client; the hard-coded skill effect
                            // is the visual feedback instead of a casting gesture.
                            CasterAction::Idle => entity.set_idle(client_tick),
                            // Casts with a cast time loop the casting gesture until the
                            // cast finishes; instant casts play the action once.
                            CasterAction::Skill => entity.set_skill_cast(cast_animation_time, has_cast_time, client_tick),
                            // Attack skills play the attack animation (like the
                            // official client's `ATTACK` skill action).
                            // Official `WeaponActionBase(ST_ATTACK)` = block 80
                            // (RenderObject.cpp:3353), not the normal attack block.
                            CasterAction::Attack => entity.set_custom_skill_action(10, cast_animation_time, false, client_tick),
                            // Pickup-style skills play the pickup animation.
                            CasterAction::Pickup => entity.set_pickup(client_tick),
                            CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                            CasterAction::Action(action_index) => {
                                entity.set_custom_skill_action(action_index as usize, cast_animation_time, has_cast_time, client_tick)
                            }
                        }

                        // The official per-skill frame range (`SetFrameSequence`
                        // in RenderObject.cpp) describes the *damage* animation;
                        // for casts with a cast time it starts when the skill
                        // resolves (see the damage arms below).
                        if !has_cast_time && let Some(sequence) = crate::world::frame_sequence_for_skill(skill_id) {
                            entity.set_skill_frame_sequence(
                                sequence.first,
                                sequence.last,
                                sequence.loop_to,
                                sequence.duration_ms,
                                client_tick,
                            );
                        }

                        // Targeted casts face their target, self-casts keep the current
                        // direction.
                        if destination_entity_id != source_entity_id {
                            entity.rotate_towards(position);
                        }
                    }

                    // Warp Portal fallback: the server asks the caster for the
                    // destination (ZC_WARPLIST) before it creates the portal skill
                    // units. When the skill use packet already carries the target
                    // cell, show the blue portal there immediately; the real
                    // `AddSkillUnit` packet replaces it (same cell) when the warp
                    // opens. This keeps the portal visible even when the warp
                    // destination menu cannot be answered.
                    if crate::world::skills::has_warp_target(skill_id)
                        && let Some(world_position) = self.map.as_ref().and_then(|map| map.get_world_position(position))
                    {
                        let now = Instant::now();
                        let lifetime = Duration::from_secs(20);
                        let provisional_id = EntityId(0xFF00_0000 | ((position.x as u32) << 8) | position.y as u32);
                        self.warp_skill_units
                            .retain(|(unit_entity_id, ..)| *unit_entity_id != provisional_id);
                        self.warp_skill_units.push((provisional_id, world_position, now, now + lifetime));
                    }

                    // Show the skill name above the caster's head, like the original
                    // client (own character, other players and population shells).
                    self.show_skill_name_text(source_entity_id, skill_id, client_tick);

                    // Official cast-start effects of the unified descriptor table: the
                    // cast-time body flash, the `EF_BEGINSPELL` cone and every other
                    // layer of the `Cast` phase (per-cast de-duplicated).
                    self.spawn_skill_effect_phase(
                        skill_id,
                        SkillPhase::Cast,
                        source_entity_id,
                        None,
                        None,
                        client_tick,
                        cast_time as f32,
                    );

                    // Asura Strike shakes the screen for the local caster (the
                    // original client draws no full-screen lettering: the kanji
                    // are world layers of the skill effect data).
                    if skill_id == SkillId(271) {
                        let is_own_cast = self
                            .client_state
                            .try_follow(this_entity())
                            .is_some_and(|entity| entity.get_entity_id() == source_entity_id);

                        if is_own_cast {
                            self.player_camera.shake(0.8, 1.1);
                        }
                    }

                    // Local player cast bars are driven by this server-confirmed
                    // cast-start packet (cast_time > 0), never by local requests.
                    self.start_own_cast_bar(source_entity_id, skill_id, cast_time, client_tick);

                    if has_cast_time {
                        // The circle has to stay for the full cast: the cast time can be
                        // shorter than one ACT cycle (the animation then plays a full
                        // cycle) and it can be longer than the animation (the animation
                        // loops), so use the longer of both.
                        let animation_duration = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| entity.skill_animation_duration_ms())
                            .unwrap_or(0);
                        let circle_duration = cast_animation_time.max(animation_duration);

                        self.spawn_cast_circle(source_entity_id, cast_circle_color, circle_duration);
                    }

                    // The 2019 server sends no sound packet for most casts, so the
                    // client plays the mapped skill sound itself — but only when the
                    // skill actually completes, never when the cast is merely
                    // acknowledged. A cast acknowledgment is sent *before* the skill
                    // takes effect, so playing here would be heard as a casting
                    // pre-sound instead of the skill's effect sound (e.g. Gloria).
                    // Cast-time skills play at their completion packet below; for an
                    // instant skill the completion/damage packet claims the sound in
                    // `SkillNoDamage`/`DamageEffect`.
                    if !has_cast_time {
                        self.skill_sound_gate.remember_instant_cast(source_entity_id, skill_id, client_tick);
                    } else if crate::world::skill_sound_is_cast_start(skill_id)
                        && self.skill_sound_gate.claim_cast_start(source_entity_id, skill_id, client_tick)
                    {
                        // Dangerous Soul Collect/Zen and similar long casts use
                        // the generic begin-spell sound: it belongs at the
                        // start of the cast, not after the cast finished.
                        let sound_position = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| entity.get_position());
                        self.skill_sounds.play_skill_sound_for_phase(
                            skill_id,
                            crate::world::SkillPhase::Success,
                            Some(source_entity_id),
                            sound_position,
                        );
                    }
                }
                NetworkEvent::SkillNoDamage {
                    source_entity_id,
                    // The destination is used to place target-anchored success
                    // effects (Heal, Resurrection, ...) at the ally that received
                    // the skill instead of on the caster.
                    destination_entity_id,
                    skill_id,
                    level,
                    success,
                } => {
                    // 2019 `ZC_USE_SKILL` (0x09CB): the caster performed an
                    // instant/complete skill. Play the full cast animation once and the
                    // completion effect, so self-buffs and the blacksmith crafting loop
                    // become visible.
                    if success {
                        if self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(source_entity_id) {
                            self.cast_bars.clear();
                            // Pace the next action with this skill's server-side
                            // after-cast act delay instead of a global constant.
                            if let Some(delays) = skill_delays(skill_id, level as u16) {
                                crate::logging::log_debug!(
                                    "skill delay table: skill={} level={} cast={}ms fixed={}ms aftercast_act={}ms aftercast_walk={}ms \
                                     cooldown={}ms",
                                    skill_id.0,
                                    level,
                                    delays.cast_ms,
                                    delays.fixed_cast_ms,
                                    delays.aftercast_act_ms,
                                    delays.aftercast_walk_ms,
                                    delays.cooldown_ms
                                );
                            }
                            self.apply_aftercast_act_delay(skill_id, level as u16, client_tick);
                        }

                        let source_position = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| entity.get_position());

                        // World position of the skill target, when the packet names
                        // one and it is not the caster. Target/`Auto` anchored
                        // success effects (for example the Heal sparkles) play
                        // there; without it they fall back to the caster.
                        let destination_position = if destination_entity_id == source_entity_id {
                            None
                        } else {
                            self.client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == destination_entity_id)
                                .map(|entity| entity.get_tile_position())
                                .and_then(|tile_position| self.map.as_ref().and_then(|map| map.get_world_position(tile_position)))
                        };

                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                        {
                            match caster_action(skill_id) {
                                // Stance/buff skills keep the idle pose (see the
                                // `SkillCast` arm above).
                                CasterAction::Idle => entity.set_idle(client_tick),
                                // This packet has no cast time: play the whole ACT skill
                                // animation once at its natural frame speed.
                                CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                                // Official `WeaponActionBase(ST_ATTACK)` = block 80.
                                CasterAction::Attack => entity.set_custom_skill_action(10, INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                                CasterAction::Pickup => entity.set_pickup(client_tick),
                                CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                                CasterAction::Action(action_index) => {
                                    entity.set_custom_skill_action(action_index as usize, 0, false, client_tick)
                                }
                            }

                            // Official frame range of the skill's damage animation.
                            if let Some(sequence) = crate::world::frame_sequence_for_skill(skill_id) {
                                entity.set_skill_frame_sequence(
                                    sequence.first,
                                    sequence.last,
                                    sequence.loop_to,
                                    sequence.duration_ms,
                                    client_tick,
                                );
                            }
                        }

                        self.show_skill_name_text(source_entity_id, skill_id, client_tick);

                        // Every layer of the `Success` phase of the unified descriptor
                        // table: hand-written layers (Blessing angel, ...) and the
                        // official layers of the same phase spawn side by side. The
                        // target entity is passed so `Target`/`Auto` anchored layers
                        // (heal columns, target buffs) follow the affected entity.
                        self.spawn_skill_effect_phase(
                            skill_id,
                            SkillPhase::Success,
                            source_entity_id,
                            (destination_entity_id != EntityId(0)).then_some(destination_entity_id),
                            destination_position,
                            client_tick,
                            0.0,
                        );

                        // Instant/self-buff casts (0x09CB) have no sound packet on
                        // the 2019 server either: play the mapped official sound once
                        // the server confirms that the skill completed. Instant
                        // skills claimed their cast here (the step above only
                        // recorded the acknowledgment), cast-time skills play their
                        // single sound now.
                        self.play_completion_sound(source_entity_id, skill_id, SkillPhase::Success, source_position, client_tick);
                    }

                    // `ZC_USE_SKILL` (0x09CB) is sent for every no-damage skill
                    // and its `level` field is the skill
                    // level, not a healed amount. It
                    // must not be rendered as a heal number (Blessing/AGI and
                    // other self-buffs showed a green "10"
                    // above the caster). Real heal
                    // numbers come from `ZC_RECOVERY`/0x0A27
                    // (`NetworkEvent::HealEffect`).
                }
                NetworkEvent::SkillCastCancelled { entity_id } => {
                    // `ZC_DISPEL` (0x01B9, rAthena `clif_skillcastcancel`): the
                    // cast of this entity was interrupted (movement, damage,
                    // another skill, ...). Stop the looping cast gesture and
                    // clear the local player's cast bar and skill label so a
                    // cancelled cast never leaves a stuck bar or animation.
                    crate::logging::log_debug!("skill cast cancelled (ZC_DISPEL / 0x01B9): entity={}", entity_id.0);

                    if self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(entity_id) {
                        self.cast_bars.clear();
                        self.skill_name_texts.remove_entity(entity_id);
                    }

                    // A cancelled cast must never play its sound later: drop the
                    // pending acknowledgement (instant cast) and the played marker.
                    self.skill_sound_gate.clear_entity(entity_id);

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.cancel_skill_cast(client_tick);
                    }
                }
                NetworkEvent::SkillGroundCast {
                    entity_id,
                    skill_id,
                    level: _,
                    position,
                } => {
                    // Ground-skill pose packet: the ambient crafting loop uses it as a
                    // fall back so at least the hammer swing is visible and audible.
                    // Play the action once and the completion effect at the target cell.
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        match caster_action(skill_id) {
                            CasterAction::Idle => entity.set_idle(client_tick),
                            CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                            CasterAction::Attack => entity.set_attack(INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                            CasterAction::Pickup => entity.set_pickup(client_tick),
                            CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                            CasterAction::Action(action_index) => {
                                entity.set_custom_skill_action(action_index as usize, 0, false, client_tick)
                            }
                        }

                        // Official frame range of the skill animation.
                        if let Some(sequence) = crate::world::frame_sequence_for_skill(skill_id) {
                            entity.set_skill_frame_sequence(
                                sequence.first,
                                sequence.last,
                                sequence.loop_to,
                                sequence.duration_ms,
                                client_tick,
                            );
                        }
                    }

                    self.show_skill_name_text(entity_id, skill_id, client_tick);

                    let world_position = self.map.as_ref().and_then(|map| map.get_world_position(position));

                    // Every layer of the `Ground` and `Success` phases of the unified
                    // descriptor table. `Success` follows the ground cell when one is
                    // known (the official client plays e.g. Storm Gust's effect there);
                    // hand-written completions are ordinary `Success` layers now.
                    for phase in [SkillPhase::Ground, SkillPhase::Success] {
                        self.spawn_skill_effect_phase(skill_id, phase, entity_id, None, world_position, client_tick, 0.0);
                    }

                    // Ground skills complete on this packet: play the mapped sound
                    // once (instant ground casts claimed their acknowledgment above).
                    self.play_completion_sound(entity_id, skill_id, SkillPhase::Ground, world_position, client_tick);
                }
                NetworkEvent::PlaySound { name, account_id, .. } => {
                    let entity_id = EntityId(account_id.0);

                    let position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| entity.get_position());

                    self.skill_sounds.play_network_sound(&name, account_id, position);
                }
                NetworkEvent::DisplayEmotion { entity_id, emotion } => {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        && let Ok(sprite) = self.sprite_loader.get_or_load("이팩트\\emotion.spr", None)
                        && let Ok(actions) = self.action_loader.get_or_load("이팩트\\emotion.act")
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(Emote::new(entity.get_position(), sprite, actions, emotion as usize)));
                    }
                }
                NetworkEvent::AccountId { .. } => {}
                NetworkEvent::CharacterList { characters } => {
                    self.session.clear_character_reconnect();

                    self.play_main_menu_click_sound();

                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_characters(characters);

                    // While the pincode prompt is up the official client blocks
                    // character selection; a selection attempt would make the char
                    // server disconnect the session (`chclif_parse` only allows
                    // 0x08B8/0x08BE/0x08C5/0x09A1 while the PIN is not verified).
                    let pincode_visible = self.client_state.follow(client_state().content_windows().pincode()).visible;

                    if !pincode_visible && !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
                        // TODO: this will do one unnecessary restore_focus. check
                        // if that will be problematic

                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.open_character_selection_window();
                    }

                    // A previous map entry may have been aborted because the map
                    // data was missing; show the reason now that we are back at
                    // the character selection screen.
                    if let Some(message) = self.session.take_map_load_error() {
                        self.interface.open_window(ErrorWindow::new(message));
                    }
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => {
                    crate::logging::log_debug!("character selection: server rejected the selection: {message}");
                    // The map server may still be restarting. Stay on the character
                    // selection screen and refresh the list so selecting the character can
                    // simply be retried once the map server is back.
                    let _ = self.networking_system.request_character_list();

                    // Aborting a map entry (e.g. because the map data is missing) can
                    // make the server reject a follow-up request. The map error is the
                    // actionable message, so don't cover it with "Rejected from server".
                    if !self.session.map_entry_aborted() {
                        self.interface.open_window(ErrorWindow::new(message.to_owned()));
                    }
                }
                NetworkEvent::CharacterDeleted => {
                    if let Some(character_id) = self.client_state.follow_mut(client_state().currently_deleting()).take() {
                        self.client_state
                            .follow_mut(client_state().character_slots())
                            .remove_with_id(character_id);
                    }
                    self.client_state.follow_mut(client_state().delete_character_email()).clear();
                    self.interface.close_window_with_class(WindowClass::CharacterDeletion);
                }
                NetworkEvent::CharacterDeletionFailed { message, .. } => {
                    *self.client_state.follow_mut(client_state().currently_deleting()) = None;
                    self.interface.close_window_with_class(WindowClass::CharacterDeletion);
                    self.interface.open_window(ErrorWindow::new(message.to_owned()))
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    crate::logging::log_debug!("character selection: server accepted the character, connecting to the map server");
                    self.play_main_menu_click_sound();

                    let Some(saved_login_data) = self.saved_login_data.as_ref() else {
                        // The character selection cannot be processed without the
                        // login data. Show the error instead of panicking.
                        let message = self
                            .render_localized(client_state().localization().error_character_selection_missing_login_data(), &[
                            ]);
                        self.interface.open_window(ErrorWindow::new(message));
                        continue;
                    };
                    let saved_login_data = *saved_login_data;
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system
                        .connect_to_map_server(self.saved_packet_version, &saved_login_data, login_data);
                    // NOTE: Nothing else must be sent to the map server until it responds:
                    // rAthena requires the first read of a session to contain exactly the
                    // login packet and disconnects otherwise. The client tick is part of
                    // the map server login success packet, so there is no need to request
                    // it here.

                    let Some(character_information) = self
                        .client_state
                        .follow(client_state().character_slots())
                        .with_id(login_data.character_id)
                        .cloned()
                    else {
                        let message = self.render_localized(client_state().localization().error_character_selection_missing_slot(), &[]);
                        self.interface.open_window(ErrorWindow::new(message));
                        continue;
                    };

                    let mut player = Entity::Player(Player::new(
                        &self.library,
                        saved_login_data.account_id,
                        &character_information,
                        client_tick,
                    ));

                    *self.client_state.follow_mut(client_state().player_name()) = character_information.name;

                    // Remember the local ids for the guild actions (leave/expel)
                    // and start the guild state of this character from scratch.
                    self.client_state
                        .follow_mut(client_state().guild())
                        .set_local_player(saved_login_data.account_id, character_information.character_id);

                    let entity_id = player.get_entity_id();
                    let entity_type = player.get_entity_type();
                    let entity_part_files = player.get_entity_part_files(&self.library);

                    if let Some(animation_data) = self
                        .async_loader
                        .request_animation_data_load(entity_id, entity_type, entity_part_files)
                    {
                        player.set_animation_data(animation_data);
                    }

                    if player.has_falcon()
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
                    {
                        player.set_falcon_animation(falcon_animation);
                    }

                    request_equipment_layers(&self.async_loader, &mut player);

                    let layout = self.async_loader.request_skill_tree_layout_load(player.get_job_id(), client_tick);
                    *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
                    *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .clear();

                    self.client_state.follow_mut(client_state().entities()).push(player);
                    self.retrack_local_player();

                    self.interface.close_window_with_class(WindowClass::CharacterSelection);
                    self.interface.open_window(MinimapWindow);
                    self.map_window_cycle = MapWindowCycle::Minimap;

                    self.interface.open_window(CharacterOverviewWindow::new(
                        client_state().player_name(),
                        // TODO: Check that manually asserting is fine. Technically this window should only
                        // be open while the player is selected.
                        this_player().manually_asserted(),
                        client_state().interface_settings(),
                    ));
                    self.interface.open_window(ChatWindow::new(
                        client_state().chat_window(),
                        client_state().chat_messages(),
                        client_state().interface_settings(),
                    ));
                    self.open_hotbar_window();
                    self.open_notification_hud_window();

                    // Put the dialog system in a well-defined state.
                    self.client_state.follow_mut(client_state().dialog_window()).end();

                    self.map = None;
                    // A fresh character map entry must always take the full map
                    // load path; the previous map name only describes the map
                    // session we just left.
                    self.current_map_name = None;
                    // A storage window from a previous session must never show up
                    // in the new character's game state.
                    self.close_storage_window();
                    self.close_cart_window();
                    self.close_vending_browse();
                    self.close_trade();
                    // The map server sends the map change packet (and with it the
                    // map data we have to load) right after the login handshake.
                    // If the packet stream is desynchronized that packet would be
                    // lost and the client would stay on a black screen forever; the
                    // session machine arms the map-entry watchdog for this case.
                    self.session.begin_map_entry(Instant::now());
                    self.skill_sounds.clear();
                    self.skill_sound_gate.clear();
                    self.client_state.follow_mut(client_state().quest_log()).clear();
                    self.client_state.follow_mut(client_state().navigation()).clear();
                    self.client_state.follow_mut(client_state().monster_database()).clear();
                    self.client_state.follow_mut(client_state().achievement()).clear();

                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.level_99_aura_entities.clear();
                    self.status_aura_flags.clear();
                    if let Some(weather) = self.weather.as_mut() {
                        weather.clear();
                    }
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .add_character(character_information);

                    self.interface.close_window_with_class(WindowClass::CharacterCreation);
                }
                NetworkEvent::CharacterCreationFailed { message, .. } => {
                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterSlotSwitched => {
                    *self.client_state.follow_mut(client_state().switch_request()) = None;
                }
                NetworkEvent::CharacterSlotSwitchFailed => {
                    let message = self.render_localized(client_state().localization().error_character_slot_switch_failed(), &[]);
                    self.interface.open_window(ErrorWindow::new(message));
                }
                NetworkEvent::AddEntity { entity_data } => {
                    let dead_sit = entity_data.dead_sit;

                    if let Some(map) = &self.map
                        && let Some(npc) = Npc::new(&self.library, map, &mut self.path_finder, entity_data, client_tick)
                    {
                        let mut npc = Entity::Npc(npc);

                        // The idle-unit appear packet carries the spawn-time dead/sit
                        // pose (`state`: 0 standing, 1 dead, 2 sitting). Only players
                        // use it in practice.
                        if dead_sit != 0 && npc.get_entity_type() == EntityType::Player {
                            match dead_sit {
                                1 => npc.set_dead(client_tick),
                                2 => npc.set_sitting(client_tick),
                                _ => {}
                            }
                        }

                        let entity_id = npc.get_entity_id();
                        let entity_type = npc.get_entity_type();
                        let entity_part_files = npc.get_entity_part_files(&self.library);

                        // Explicit lifecycle bookkeeping: the server re-sends
                        // `AddEntity` for entities that are already visible (job
                        // change, respawn). The machine records the spawn and the
                        // client replaces the old entity instead of duplicating it.
                        let add_decision = self.entity_lifecycle.on_add_entity(entity_id);
                        let spawn_outcome = self.entity_lifecycle.on_spawned(entity_id, npc.get_job_id());

                        if spawn_outcome.previous_state.is_some() || spawn_outcome.job_changed {
                            crate::logging::log_debug!(
                                "entity: replacing already visible entity {} (previous state {:?}, job changed {})",
                                entity_id.0,
                                spawn_outcome.previous_state,
                                spawn_outcome.job_changed
                            );
                        }

                        let entities = self.client_state.follow_mut(client_state().entities());

                        // If the entity was already visible, we use its old alpha value.
                        if matches!(add_decision, AddEntityDecision::Replace { .. })
                            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id)
                        {
                            npc.inherit_fade_state(entity, client_tick);
                        };

                        // Remove the old copy of a replaced entity so it never
                        // exists twice.
                        entities.retain(|entity| entity.get_entity_id() != entity_id);

                        if let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(entity_id, entity_type, entity_part_files)
                        {
                            npc.set_animation_data(animation_data);
                        }

                        if npc.has_falcon()
                            && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
                        {
                            npc.set_falcon_animation(falcon_animation);
                        }

                        request_equipment_layers(&self.async_loader, &mut npc);

                        #[cfg(feature = "debug")]
                        npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

                        entities.push(npc);
                    }
                }
                NetworkEvent::RemoveEntity { entity_id, reason } => {
                    self.chat_bubbles.remove_entity(entity_id);
                    self.skill_name_texts.remove_entity(entity_id);
                    self.cast_bars.remove_entity(entity_id);
                    self.skill_sounds.remove_entity(entity_id);
                    self.skill_sound_gate.clear_entity(entity_id);
                    self.client_state.follow_mut(client_state().spirit_orbs()).remove(entity_id);

                    let remove_outcome = self.entity_lifecycle.on_remove_entity(entity_id, &reason);
                    match remove_outcome {
                        RemoveEntityOutcome::Known { state, died } => {
                            crate::logging::log_debug!(
                                "entity lifecycle: remove entity {} -> {:?} (died: {})",
                                entity_id.0,
                                state,
                                died
                            );
                        }
                        RemoveEntityOutcome::Unknown => {
                            crate::logging::log_debug!("entity lifecycle: remove for untracked entity {}", entity_id.0);
                        }
                    }

                    // The entity is gone; pending visual loads for it are moot.
                    self.asset_retries.forget(&LoaderId::AnimationData(entity_id));
                    self.asset_retries.forget(&LoaderId::FalconAnimationData(entity_id));
                    self.asset_retries.forget(&LoaderId::WeaponAnimationData(entity_id));
                    self.asset_retries.forget(&LoaderId::ShieldAnimationData(entity_id));
                    self.asset_retries.forget(&LoaderId::RobeAnimationData(entity_id));

                    // Classic teleport/logout behaviour: entities that leave via
                    // teleport/logout play a warp effect and sound at their last
                    // position. Out-of-sight and death removals stay plain.
                    let disappearance_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| entity.get_position());

                    if matches!(reason, DisappearanceReason::Teleported | DisappearanceReason::LoggedOut)
                        && let Some(position) = disappearance_position
                    {
                        self.spawn_teleport_vanish(position);
                    }

                    // If the motive is dead, you need to set the player to dead.
                    if reason == DisappearanceReason::Died {
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            let entity_type = entity.get_entity_type();

                            if entity_type == EntityType::Monster {
                                let mut entity = entity.clone();
                                entity.set_dead(client_tick);
                                entity.stop_movement();

                                // Remove the entity from the list of alive entities.
                                self.client_state
                                    .follow_mut(client_state().entities())
                                    .retain(|entity| entity.get_entity_id() != entity_id);

                                // Add the entity to the list of dead entities.
                                self.client_state.follow_mut(client_state().dead_entities()).push(entity);
                            } else if entity_type == EntityType::Player {
                                entity.set_dead(client_tick);

                                // If the player is us, we need to open the respawn window.
                                if entity_id == self.client_state.follow(client_state().entities())[0].get_entity_id() {
                                    self.interface.open_window(RespawnWindow);
                                }
                            }
                        }
                    } else {
                        // For non-death disappearances, start fading out the entity.
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            entity.fade_out(reason, client_tick);
                        }
                    }

                    // If the entity that was removed had an attack buffered we remove the entity
                    // from the buffer.
                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id)) {
                        *buffered_action = None;
                    }
                }
                NetworkEvent::AddGroundItem {
                    entity_id,
                    item_id,
                    is_identified,
                    quantity,
                    position,
                    x_offset,
                    y_offset,
                } => {
                    if let Some(map) = self.map.as_ref()
                        && let Some(mut ground_item) = GroundItem::new(
                            map,
                            item_id,
                            entity_id,
                            is_identified,
                            quantity,
                            position,
                            x_offset,
                            y_offset,
                            client_tick,
                        )
                    {
                        let ground_items = self.client_state.follow_mut(client_state().ground_items());
                        let entity_part_files = ground_item.get_entity_part_files(&self.library);

                        if let Some(animation_data) = self
                            .async_loader
                            // TODO: Technically Npc is not correct here. We could add an item
                            // variant or refactor this fuction to take an optional entity
                            // type.
                            .request_animation_data_load(entity_id, EntityType::Npc, entity_part_files)
                        {
                            ground_item.set_animation_data(animation_data);
                        }

                        ground_items.push(ground_item);
                    } else {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] failed to spawn item", "error".red());
                    }
                }
                NetworkEvent::RemoveGroundItem { entity_id } => {
                    if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.fade_out(client_tick);
                    }

                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_pick_up_item(entity_id)) {
                        *buffered_action = None;
                    }
                }
                NetworkEvent::EntityMove {
                    entity_id,
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    // Clone the dynamic cell overrides before borrowing the
                    // entity list mutably (breakable walls must affect the walk
                    // prediction too).
                    let cell_overrides = self.client_state.follow(client_state().map_cell_overrides()).clone();
                    let entities = self.client_state.follow_mut(client_state().entities());
                    let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity
                        && let Some(map) = &self.map
                    {
                        entity.move_from_to(
                            map,
                            &mut self.path_finder,
                            &cell_overrides,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        entity.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::PlayerMove {
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    let cell_overrides = self.client_state.follow(client_state().map_cell_overrides()).clone();

                    if let Some(map) = &self.map
                        && let Some(player) = self.client_state.try_follow_mut(this_entity())
                    {
                        player.move_from_to(
                            map,
                            &mut self.path_finder,
                            &cell_overrides,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        player.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::ChangeMap { map_name, position } => {
                    // A same-map "map move" is a teleport (fly wing, AL_TELEPORT,
                    // random warp, ...). The server sends the normal map move packet
                    // with the current map name; reloading the map for it would take
                    // long and hide the teleport, so reposition the player directly,
                    // play the warp effect/sound and let the server spawn the
                    // surrounding entities again.
                    if is_same_map_teleport(
                        self.current_map_name.as_deref(),
                        map_name.as_str(),
                        self.map.is_some(),
                        self.session.state(),
                    ) {
                        let old_position = self.client_state.try_follow(this_entity()).map(|entity| entity.get_position());

                        // Drop the transient state of the old position.
                        self.client_state.follow_mut(client_state().entities()).truncate(1);
                        self.client_state.follow_mut(client_state().dead_entities()).clear();
                        self.client_state.follow_mut(client_state().ground_items()).clear();
                        *self.client_state.follow_mut(client_state().buffered_action()) = None;
                        // A teleport is a warp as well: everything but the player is
                        // re-spawned by the server, so drop the entity bookkeeping and
                        // pending visual loads of the old position. Status effects stay
                        // (a teleport must not remove buffs).
                        self.entity_lifecycle.on_warp();
                        self.asset_retries.clear();
                        self.retrack_local_player();
                        self.particle_holder.clear();
                        self.effect_holder.clear();
                        // The aura tracker must be reset together with the effects:
                        // the player entity survives the fast-path teleport, but its
                        // aura effect was cleared above. Keeping the id in the tracker
                        // would make `update_level_99_auras` skip re-creating it.
                        self.level_99_aura_entities.clear();
                        self.chat_bubbles.clear();
                        self.cast_bars.clear();
                        self.skill_name_texts.clear();

                        if let Some(position) = old_position {
                            self.spawn_teleport_vanish(position);
                        }

                        let Some(map) = self.map.as_mut() else {
                            continue;
                        };
                        let player = self.client_state.follow_mut(this_entity().manually_asserted());
                        player.set_position(map, position, client_tick);
                        self.player_camera.set_focus_point(player.get_position());

                        // Ask the server for the entities around the new position
                        // (same load-end acknowledgement as after a real map change).
                        let _ = self.networking_system.map_loaded();
                        continue;
                    }

                    self.map = None;
                    self.skill_sounds.clear();
                    self.skill_sound_gate.clear();
                    self.client_state.follow_mut(client_state().quest_log()).clear();
                    self.client_state.follow_mut(client_state().navigation()).clear();
                    self.client_state.follow_mut(client_state().monster_database()).clear();
                    self.client_state.follow_mut(client_state().achievement()).clear();
                    // The server closes the storage on a map change; close the UI
                    // and drop the transient list immediately so it can't go stale.
                    self.close_storage_window();
                    self.close_cart_window();
                    self.close_vending_browse();
                    self.close_trade();
                    self.minimap_last_tile = None;
                    self.client_state.follow_mut(client_state().minimap()).clear();
                    self.client_state.follow_mut(client_state().spirit_orbs()).clear();
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.level_99_aura_entities.clear();
                    self.status_aura_flags.clear();
                    if let Some(weather) = self.weather.as_mut() {
                        weather.clear();
                    }
                    self.chat_bubbles.clear();
                    self.pending_item_drag = None;
                    self.pending_item_drag = None;
                    self.skill_name_texts.clear();
                    self.cast_bars.clear();
                    self.aftercast_act_until = None;
                    self.warp_skill_units.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    // Dynamic cell overrides and the map properties belong to the
                    // old map; the server re-sends them after the map entry.
                    self.client_state.follow_mut(client_state().map_cell_overrides()).clear();
                    *self.client_state.follow_mut(client_state().map_type()) = 0;
                    *self.client_state.follow_mut(client_state().map_flags()) = 0;

                    // Only the player must stay alive between map changes.
                    self.client_state.follow_mut(client_state().entities()).truncate(1);
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;

                    // Everything else belongs to the old map session: forget the
                    // entity bookkeeping and pending visual loads. The local
                    // player's status effects are kept: the server only re-sends
                    // statuses with the `DisplayPc`/`DisplayNpc` flag when the
                    // player enters the new map, so clearing all of them would make
                    // timed buffs (Assumptio, Blessing, ...) vanish from the buff
                    // bar and lose their world aura until they are recast. Timed
                    // statuses still expire locally and an end packet removes them;
                    // statuses of other entities are dropped with the old session.
                    self.entity_lifecycle.on_warp();
                    self.asset_retries.clear();
                    let own_entity_id = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id().0);
                    if let Some(own_entity_id) = own_entity_id {
                        self.client_state
                            .follow_mut(client_state().status_effects())
                            .retain_for_entity(own_entity_id);
                    } else {
                        self.client_state.follow_mut(client_state().status_effects()).clear();
                    }
                    self.retrack_local_player();

                    // Close any remaining dialogs.
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    self.session.map_change_received(map_name.clone(), Some(position), Instant::now());
                    self.async_loader.request_map_load(map_name, Some(position));
                }
                NetworkEvent::UpdateClientTick { client_tick, received_at } => {
                    self.game_timer.set_client_tick(client_tick, received_at);
                }
                NetworkEvent::StatusEffect {
                    entity_id,
                    status_index,
                    active,
                    total_duration_in_milliseconds,
                    remaining_in_milliseconds,
                    values,
                } => {
                    // Status effects of all entities are tracked (the server sends
                    // area status packets); the buff bar only displays the player's
                    // own effects.
                    let definition = status_effect_definition(status_index);
                    let icon = match (active, definition.icon_path()) {
                        (true, Some(path)) => match self.texture_loader.get_or_load(&path, ImageType::Color) {
                            Ok(icon) => Some(icon),
                            Err(error) => {
                                crate::logging::log_warn!("buffbar: failed to load status icon '{}': {:?}", path, error);
                                None
                            }
                        },
                        _ => None,
                    };

                    if active && definition.icon.is_none() {
                        crate::logging::log_debug!(
                            "status: status effect {} ({}) has no icon mapping",
                            status_index,
                            definition.name
                        );
                    }

                    let client_tick = client_tick.0 as u64;
                    self.client_state.follow_mut(client_state().status_effects()).apply(
                        entity_id.0,
                        status_index,
                        active,
                        total_duration_in_milliseconds,
                        remaining_in_milliseconds,
                        values,
                        client_tick,
                        definition.name.to_owned(),
                        icon,
                    );

                    // Data-driven body visuals (tints/alpha); the table lives in
                    // `archive/data/client/status_visual_effects.ron`.
                    self.refresh_entity_status_visual(entity_id);

                    // `EFST_CHANGE_SIZE` / `EFST_CHANGE_SIZE_MONSTER`
                    // (1420/1421): the server `val1` is the size in percent.
                    if matches!(status_index, 1420 | 1421)
                        && let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_status_scale(active, values[0]);
                    }

                    // `EFST_PUSH_CART` (673) keeps the merchant cart visible even
                    // when the server's entity option bit is missing.
                    if status_index == 673
                        && let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_cart_status(active, values[0]);
                    }
                }
                NetworkEvent::ChatMessage { text, color } => {
                    // The server-side aura stylist NPC asks the client to change its
                    // local aura style with a private chat marker. Apply the style and
                    // hide the marker message from the chat window; normal chat is
                    // passed through untouched.
                    if let Some(marker) = parse_aura_style_marker(&text) {
                        match marker {
                            AuraStyleMarker::Style(style) => self.set_aura_style(style),
                            AuraStyleMarker::UnknownStyle => {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] aura stylist NPC requested an unknown aura style", "error".red());
                            }
                        }

                        continue;
                    }

                    // Server announcements are a separate `ServerAnnouncement`
                    // event (broadcast packets); a `Broadcast` color alone is
                    // also used for normal player chat, so it must not open the
                    // announce box.
                    // While a chat room is open the same line is also shown in the
                    // room window (own lines arrive as `ZC_NOTIFY_PLAYERCHAT`,
                    // other members as `ZC_NOTIFY_CHAT`).
                    if self.client_state.follow(client_state().content_windows().chat_room()).is_in_room() {
                        self.client_state
                            .follow_mut(client_state().content_windows().chat_room())
                            .push_message(text.clone(), false);
                        self.interface.invalidate_layouts();
                    }

                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, color));
                }
                NetworkEvent::OverheadChatMessage { entity_id, text, color } => {
                    if self.client_state.follow(client_state().content_windows().chat_room()).is_in_room() {
                        self.client_state
                            .follow_mut(client_state().content_windows().chat_room())
                            .push_message(text.clone(), false);
                        self.interface.invalidate_layouts();
                    }

                    self.chat_bubbles.push(entity_id, text, color, client_tick);
                }
                NetworkEvent::WhisperReceived {
                    sender_name,
                    message,
                    is_admin,
                } => {
                    // A whisper belongs to its own conversation and to the
                    // whisper chat tab, never to the public tab.
                    self.client_state
                        .follow_mut(client_state().whisper())
                        .push_incoming(&sender_name, message.clone());

                    let prefix = match is_admin {
                        true => "[Whisper][GM]",
                        false => "[Whisper]",
                    };

                    self.push_chat_message(ChatMessage::whisper(
                        format!("{prefix} {sender_name} : {message}"),
                        MessageColor::Information,
                        sender_name,
                    ));
                }
                NetworkEvent::GuildChat { message } => {
                    // `ZC_GUILD_CHAT` carries the full `<name> : <message>`
                    // line (the same format party chat uses).
                    let message = message.trim_end_matches('\0').trim();

                    if !message.is_empty() {
                        self.push_chat_message(ChatMessage::new_channel(
                            format!("[Guild] {message}"),
                            MessageColor::Rgb {
                                red: 150,
                                green: 210,
                                blue: 255,
                            },
                            ChatChannel::Guild,
                        ));
                    }
                }
                NetworkEvent::ItemPickedUp { item_id, amount } => {
                    let name = self.library.composed_item_name(item_id, true, 0, &[0; 4]);
                    let name = match name.trim().is_empty() {
                        true => format!("Item #{}", item_id.0),
                        false => name,
                    };
                    let text = match amount > 1 {
                        true => format!("Picked up {name} x{amount}"),
                        false => format!("Picked up {name}"),
                    };

                    self.push_notification(NotificationKind::Pickup, text, client_tick);
                }
                NetworkEvent::QuestBanner {
                    quest_id,
                    active,
                    objective_names,
                } => {
                    if active {
                        let text = match objective_names.first() {
                            Some(objective) => format!("Quest started: {objective}"),
                            None => format!("Quest started (Quest #{quest_id})"),
                        };

                        self.push_notification(NotificationKind::Quest, text, client_tick);
                    }
                }
                NetworkEvent::ServerAnnouncement { text } => {
                    let text = text.replace(['\r', '\n'], " ");
                    self.push_notification(NotificationKind::Announce, text, client_tick);
                }
                NetworkEvent::DisplayImage { image_name, location } => {
                    if matches!(location, ragnarok_packets::ImageLocation::ClearAll) {
                        self.client_state.follow_mut(client_state().dialog_window()).clear_image();
                    } else if let Some(texture) = load_cutin_texture(&self.game_file_loader, &self.texture_loader, &image_name) {
                        self.client_state.follow_mut(client_state().dialog_window()).set_image(texture);
                    }

                    self.interface.invalidate_layouts();
                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::OpenBook { book_id, page } => {
                    let pages = load_book_pages(&self.game_file_loader, book_id);
                    self.interface.close_window_with_class(WindowClass::Book);
                    self.interface.open_window(BookWindow::new(book_id, page, pages));
                }
                NetworkEvent::PincodeState {
                    pincode_seed,
                    account_id,
                    state,
                } => {
                    let outcome = self
                        .client_state
                        .follow_mut(client_state().content_windows().pincode())
                        .handle_state(pincode_seed, account_id, state);

                    match outcome {
                        crate::state::content_windows::PincodePromptOutcome::Open => {
                            self.interface.close_window_with_class(WindowClass::Pincode);
                            self.interface.close_window_with_class(WindowClass::CharacterSelection);
                            self.interface
                                .open_window(PincodeWindow::new(client_state().content_windows().pincode()));
                        }
                        crate::state::content_windows::PincodePromptOutcome::Passed => {
                            self.interface.close_window_with_class(WindowClass::Pincode);
                            self.open_character_selection_if_closed();
                        }
                        crate::state::content_windows::PincodePromptOutcome::Unsupported(message) => {
                            self.interface.close_window_with_class(WindowClass::Pincode);
                            self.open_character_selection_if_closed();
                            self.push_chat_message(ChatMessage::new(message.to_owned(), MessageColor::Error));
                        }
                    }
                }
                NetworkEvent::ChatRoomCreated { flag } => match flag {
                    0 => {
                        let player_name = self.client_state.follow(client_state().player_name()).clone();
                        self.client_state
                            .follow_mut(client_state().content_windows().chat_room())
                            .created(&player_name);
                        self.interface.close_window_with_class(WindowClass::ChatRoom);
                        self.interface
                            .open_window(ChatRoomWindow::new(client_state().content_windows().chat_room()));
                    }
                    1 => self
                        .client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .set_error("The chat room limit was exceeded.".to_owned()),
                    _ => self
                        .client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .set_error("A chat room with the same name already exists.".to_owned()),
                },
                NetworkEvent::ChatRoomAnnounced {
                    owner_id,
                    chat_id,
                    limit,
                    users,
                    room_type,
                    title,
                } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .announced(crate::state::content_windows::ChatRoomSummary {
                            owner_id,
                            chat_id,
                            title,
                            users,
                            limit,
                            room_type,
                        });
                    self.interface.invalidate_layouts();
                }
                NetworkEvent::ChatRoomDestroyed { chat_id } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .destroyed(chat_id);
                    self.interface.invalidate_layouts();
                }
                NetworkEvent::ChatRoomEnterRefused { result } => {
                    let message = match result {
                        0 => "The chat room is full.",
                        1 => "The chat room password is wrong.",
                        2 => "You were kicked from that chat room.",
                        4 => "You do not have enough zeny for that chat room.",
                        5 => "Your level is too low for that chat room.",
                        6 => "Your level is too high for that chat room.",
                        7 => "Your job class cannot enter that chat room.",
                        _ => "The server refused to enter the chat room.",
                    };
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .set_error(message.to_owned());
                    self.interface.invalidate_layouts();
                }
                NetworkEvent::ChatRoomEntered { chat_id, members } => {
                    let members: Vec<(String, bool)> = members.iter().map(|member| (member.name.clone(), member.role == 0)).collect();
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .entered(chat_id, &members);
                    self.interface.close_window_with_class(WindowClass::ChatRoom);
                    self.interface
                        .open_window(ChatRoomWindow::new(client_state().content_windows().chat_room()));
                }
                NetworkEvent::ChatRoomMemberJoined { name, .. } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .member_joined(&name);
                    self.interface.invalidate_layouts();
                }
                NetworkEvent::ChatRoomMemberLeft { name, kicked, .. } => {
                    let local_player_name = self.client_state.follow(client_state().player_name()).clone();
                    let local_left = self
                        .client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .member_left(&name, kicked, &local_player_name);

                    if local_left {
                        self.interface.close_window_with_class(WindowClass::ChatRoom);
                    } else {
                        self.interface.invalidate_layouts();
                    }
                }
                NetworkEvent::ChatRoomChanged {
                    owner_id,
                    chat_id,
                    limit,
                    users,
                    room_type,
                    title,
                } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .changed(chat_id, title, limit, room_type, owner_id, users);
                    self.interface.invalidate_layouts();
                }
                NetworkEvent::ChatRoomRoleChanged { flag, name } => {
                    self.client_state
                        .follow_mut(client_state().content_windows().chat_room())
                        .role_changed(flag, &name);
                    self.interface.invalidate_layouts();
                }
                NetworkEvent::UpdateEntityDetails {
                    entity_id,
                    name,
                    guild_name,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    let mut guild_id_to_request = None;

                    if let Some(entity) = entity {
                        entity.set_details(name);
                        // The player details response also contains the guild name;
                        // an empty name removes the guild label again (for example
                        // after a player left their guild).
                        entity.set_guild_name(guild_name);

                        // The guild label is only known now, request the emblem image
                        // for it (one request per guild per map session).
                        if entity.get_guild_name().is_some() {
                            guild_id_to_request = Some(entity.get_guild_id()).filter(|guild_id| *guild_id != 0);
                        }
                    }

                    if let Some(guild_id) = guild_id_to_request
                        && self.guild_emblems.request_if_needed(guild_id)
                    {
                        let _ = self.networking_system.request_guild_emblem(guild_id);
                    }

                    // The name was requested because the player right-clicked the
                    // character before its name was loaded. Open the menu now.
                    if let Some((pending_entity_id, position)) = self.pending_context_menu
                        && pending_entity_id == entity_id
                    {
                        self.pending_context_menu = None;

                        if open_character_context_menu(&self.client_state, &mut self.interface, entity_id, position) {
                            let _ = self.networking_system.entity_details(entity_id);
                        }
                    }
                }
                NetworkEvent::GuildEmblemImage {
                    guild_id,
                    emblem_id,
                    emblem_data,
                } => match decode_emblem(&emblem_data) {
                    Ok(image) => {
                        let name = format!("guild_emblem_{guild_id}_{emblem_id}");
                        let texture = self.texture_loader.create_color(&name, image, true);
                        self.guild_emblems.insert(guild_id, GuildEmblem { texture });
                    }
                    Err(_error) => {
                        #[cfg(feature = "debug")]
                        print_debug!("Failed to decode guild emblem of guild {guild_id} (emblem id {emblem_id}): {_error}");
                    }
                },
                NetworkEvent::GuildMenuInterface { menu_flags } => {
                    self.client_state.follow_mut(client_state().guild()).menu_flags = menu_flags;
                }
                NetworkEvent::GuildInfo { information } => {
                    let (guild_id, guild_name) = {
                        let guild = self.client_state.follow_mut(client_state().guild());
                        guild.apply_info(&information);
                        (guild.guild_id, guild.name.clone())
                    };

                    if guild_id != 0 {
                        if !guild_name.trim().is_empty() {
                            // The overhead guild label is otherwise only filled from
                            // `ZC_ACK_REQNAMEALL` (hover/context menu). The guild window
                            // just learned the local guild name, so apply it to every
                            // nearby member of the same guild.
                            for entity in self.client_state.follow_mut(client_state().entities()) {
                                if entity.get_guild_id() == guild_id {
                                    entity.set_guild_name(Some(guild_name.clone()));
                                }
                            }
                        }

                        // The emblem is downloaded on demand; the guild window is a
                        // second place that can learn the guild id first.
                        if self.guild_emblems.request_if_needed(guild_id) {
                            let _ = self.networking_system.request_guild_emblem(guild_id);
                        }
                    }
                }
                NetworkEvent::GuildMembers { members } => {
                    let mut name_requests = Vec::new();

                    {
                        let guild = self.client_state.follow_mut(client_state().guild());
                        guild.apply_members(&members);

                        // The 2019 member list has no name field. Ask for every
                        // unknown name once per map session.
                        let unknown: Vec<CharacterId> = guild
                            .members
                            .iter()
                            .filter(|member| member.name.trim().is_empty() && member.character_id.0 != 0)
                            .map(|member| member.character_id)
                            .filter(|character_id| !guild.requested_names.contains(character_id))
                            .collect();

                        for character_id in unknown {
                            guild.requested_names.push(character_id);
                            name_requests.push(character_id);
                        }
                    }

                    for character_id in name_requests {
                        let _ = self.networking_system.request_guild_member_name(character_id);
                    }
                }
                NetworkEvent::GuildPositionNames { positions } => {
                    self.client_state
                        .follow_mut(client_state().guild())
                        .apply_position_names(&positions);
                }
                NetworkEvent::GuildPositionRights { positions } => {
                    self.client_state.follow_mut(client_state().guild()).apply_position_info(&positions);
                }
                NetworkEvent::GuildPositionChanged { positions } => {
                    self.client_state
                        .follow_mut(client_state().guild())
                        .apply_position_changes(&positions);
                }
                NetworkEvent::GuildMemberPositionChanged { members } => {
                    self.client_state
                        .follow_mut(client_state().guild())
                        .apply_member_position_changes(&members);
                }
                NetworkEvent::GuildNotice { subject, notice } => {
                    self.client_state.follow_mut(client_state().guild()).apply_notice(subject, notice);
                }
                NetworkEvent::GuildAlliances { alliances } => {
                    self.client_state.follow_mut(client_state().guild()).apply_alliances(&alliances);
                }
                NetworkEvent::GuildMemberName { character_id, name } => {
                    self.client_state
                        .follow_mut(client_state().guild())
                        .apply_member_name(character_id, name);
                }
                NetworkEvent::GuildMemberRemoved { character_id, expelled } => {
                    let is_local = self
                        .client_state
                        .follow(client_state().guild())
                        .local_character_id
                        .is_some_and(|local_character_id| local_character_id == character_id);

                    if is_local {
                        self.client_state.follow_mut(client_state().guild()).leave_local_guild();
                        self.interface.close_window_with_class(WindowClass::Guild);
                    } else {
                        let guild = self.client_state.follow_mut(client_state().guild());
                        guild.remove_member(character_id);
                        guild.status_message = match expelled {
                            true => "A member was expelled from the guild.".to_owned(),
                            false => "A member left the guild.".to_owned(),
                        };
                    }
                }
                NetworkEvent::GuildInviteResult { result } => {
                    let guild = self.client_state.follow_mut(client_state().guild());
                    guild.status_message = match result {
                        0 => "The invitation failed: the player is already in a guild or busy.".to_owned(),
                        1 => "The invitation failed: the player is offline.".to_owned(),
                        3 => "The invitation failed: the guild is full.".to_owned(),
                        result => format!("The invitation failed (result {result})."),
                    };
                }
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    damage_amount,
                    attack_duration,
                    is_critical,
                    skill_id,
                    skill_start_tick: _,
                } => {
                    let target_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .map(|entity| entity.get_tile_position());

                    // Instant damage skills (e.g. Bash, Magnum Break) send no
                    // completion packet: their acknowledged cast waits for this hit
                    // packet to play the official skill sound (so it is heard at the
                    // impact, not as a pre-cast sound). Cast-time damage skills are
                    // unchanged.
                    if let Some(skill_id) = skill_id {
                        // Instant skills have a pending cast and claim the hit
                        // sound here. Cast-time damage skills (e.g. Asura
                        // Strike) have no completion/no-damage packet, so their
                        // mapped hit/effect sound has to be claimed from the
                        // damage packet as well.
                        let instant_claimed = self.skill_sound_gate.claim_damage_hit(source_entity_id, skill_id, client_tick);
                        if instant_claimed || self.skill_sound_gate.claim_completion(source_entity_id, skill_id, client_tick) {
                            let source_position = self
                                .client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == source_entity_id)
                                .map(|entity| entity.get_position());
                            self.skill_sounds.play_skill_sound_for_phase(
                                skill_id,
                                SkillPhase::Hit,
                                Some(source_entity_id),
                                source_position,
                            );
                        }
                    }

                    // Skill damage attribution (`ZC_NOTIFY_SKILL`, 0x01DE): play
                    // the official release/hit effect at the damaged target and the
                    // caster effect on the caster. Attack packets carry no skill id,
                    // so auto attacks can never trigger skill effects.
                    if let Some(skill_id) = skill_id
                        && self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(source_entity_id)
                    {
                        let target_world_position = target_position
                            .and_then(|tile_position| self.map.as_ref().and_then(|map| map.get_world_position(tile_position)));
                        if !crate::world::skill_has_effect(skill_id) {
                            crate::logging::log_debug!("damage skill: skill {} has no effect mapping", skill_id.0);
                        }

                        // Hand-written and official layers are no longer mutually
                        // exclusive: every phase of the unified descriptor table
                        // spawns, at the damaged target and on the caster.
                        for phase in [SkillPhase::Success, SkillPhase::Release, SkillPhase::Hit, SkillPhase::Caster] {
                            self.spawn_skill_effect_phase(
                                skill_id,
                                phase,
                                source_entity_id,
                                Some(destination_entity_id),
                                target_world_position,
                                client_tick,
                                0.0,
                            );
                        }
                    }

                    // Auto attack logic.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == source_entity_id)
                    {
                        let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                        let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                        if let Some(BufferedAction::AttackEntity { entity_id }) = *buffered_action {
                            let _ = self.networking_system.player_attack(entity_id);

                            if !auto_attack {
                                *buffered_action = None;
                            }
                        }
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    // TODO: Maybe also or_else this_entity?
                    {
                        if let Some(target_position) = target_position {
                            entity.rotate_towards(target_position);
                        }

                        entity.set_attack(attack_duration, is_critical, client_tick);
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        let particle: Box<dyn Particle + Send + Sync> = match damage_amount {
                            Some(amount) => Box::new(DamageNumber::new(
                                entity.get_position(),
                                amount as u64,
                                is_critical,
                                entity.get_direction(),
                            )),
                            None => Box::new(Miss::new(entity.get_position())),
                        };

                        self.particle_holder.spawn_particle(particle);
                    }

                    // Classic auto-attack projectile (arrows): the weapon data
                    // file decides which weapons fire one, so the client loop
                    // has no hard-coded bow type / item view id range. Only auto
                    // attacks (attack packets carry no skill id) spawn it.
                    if skill_id.is_none()
                        && let Some(target_tile) = target_position
                    {
                        let attacker = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| (entity.get_weapon(), entity.get_position()));

                        if let Some((weapon, attacker_position)) = attacker
                            && let Some(projectile) = crate::world::weapon::projectile_for_weapon(weapon)
                            && let Some(target_world_position) = self.map.as_ref().and_then(|map| map.get_world_position(target_tile))
                        {
                            self.ensure_projectile_assets(projectile.kind());

                            if let Some(Some(projectile_assets)) = self.projectile_assets.get(&projectile.kind()).cloned() {
                                crate::logging::log_debug!(
                                    "projectile: spawning {:?} from {:?} to {:?}",
                                    projectile.kind(),
                                    attacker_position,
                                    target_world_position
                                );
                                // The arrow is a world-space quad rotated along
                                // its flight vector (the item sprite has no
                                // per-direction frames); the effect also plays
                                // the impact sound when it lands.
                                if let Some(texture) = projectile_assets.sprite.textures.first().cloned() {
                                    self.effect_holder.add_effect(Box::new(ArrowEffect::new(
                                        texture,
                                        attacker_position,
                                        target_world_position,
                                        projectile.flight_seconds(),
                                        self.particle_holder.arrow_hit_sound(),
                                        Some(self.audio_engine.clone()),
                                    )));
                                }
                            }
                        }
                    }
                }
                NetworkEvent::EntityPickUpItem { entity_id, item_entity_id } => {
                    let item_position = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == item_entity_id)
                        .map(|item| item.get_tile_position());

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        if let Some(item_position) = item_position {
                            entity.rotate_towards(item_position);
                        }

                        if matches!(
                            entity.get_entity_type(),
                            EntityType::Player | EntityType::Hidden | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary
                        ) {
                            entity.set_pickup(client_tick);
                        }
                    }
                }
                NetworkEvent::HealEffect { entity_id, heal_amount } => {
                    // `ZC_RECOVERY` has no entity id (the server only sends it to the
                    // player themselves), so `None` resolves to the local player.
                    let position = entity_id
                        .and_then(|entity_id| {
                            self.client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id)
                                .map(|entity| entity.get_position())
                        })
                        .or_else(|| self.client_state.try_follow(this_entity()).map(|entity| entity.get_position()));

                    if let Some(position) = position {
                        self.particle_holder
                            .spawn_particle(Box::new(HealNumber::new(position, heal_amount as u64)));
                    }
                }
                NetworkEvent::ChangeDirection {
                    entity_id,
                    head_direction,
                    direction,
                } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_direction(direction);
                        entity.set_head_direction(head_direction);
                    }
                }
                NetworkEvent::StateChange {
                    entity_id,
                    body_state,
                    health_state,
                    effect_state,
                    ..
                } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_body_state(body_state);
                        entity.set_health_state(health_state);
                        // Applies hide/cloak/invisible by hiding the entity; the
                        // remaining option bits (falcon/cart/riding/costumes) are
                        // stored for the appearance workstream.
                        entity.set_effect_state(effect_state, client_tick);
                    }

                    // Load the hunter falcon animation lazily the first time the
                    // option is enabled for this entity.
                    let falcon_entity_index = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .position(|entity| entity.get_entity_id() == entity_id && entity.has_falcon() && !entity.has_falcon_animation())
                        .filter(|_| effect_state & 0x10 != 0);

                    if let Some(index) = falcon_entity_index
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
                        && let Some(entity) = self.client_state.follow_mut(client_state().entities()).get_mut(index)
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }
                }
                NetworkEvent::UpdateEntityHealth {
                    entity_id,
                    health_points,
                    maximum_health_points,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_health(health_points, maximum_health_points);
                    }

                    // Keep the own HP bar in the party window in sync. The server
                    // does not send `ZC_NOTIFY_HP_TO_GROUPM` for the local player,
                    // so the party window takes the HP from the local entity.
                    let own_account_id = self.saved_login_data.as_ref().map(|login_data| login_data.account_id);
                    let is_own_entity = self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|entity| entity.get_entity_id() == entity_id);

                    if let (true, Some(own_account_id)) = (is_own_entity, own_account_id) {
                        self.client_state.follow_mut(client_state().party()).apply_health(
                            own_account_id,
                            health_points as i32,
                            maximum_health_points as i32,
                        );
                    }
                }
                NetworkEvent::UpdateStat { stat_type } => {
                    // The zeny amount is displayed by the inventory window, so keep it in
                    // the client state as well (the player stats do not contain it).
                    if let StatType::Zeny(value) = stat_type {
                        crate::logging::log_debug!("zeny update: {}", value);
                        *self.client_state.follow_mut(client_state().zeny()) = Some(u64::from(value));
                    }

                    // rAthena 2019 has no dedicated level-up packet; the client
                    // derives the level-up indicator from the base/job level
                    // stat update. Only an increase is a level up (a respawn or
                    // map change re-sends the same level).
                    let level_up = match &stat_type {
                        StatType::BaseLevel(level) => self
                            .client_state
                            .try_follow(this_player())
                            .and_then(|player| (*level as usize > player.base_level).then_some((true, *level as u16))),
                        StatType::JobLevel(level) => self
                            .client_state
                            .try_follow(this_player())
                            .and_then(|player| (*level as usize > player.job_level).then_some((false, *level as u16))),
                        _ => None,
                    };

                    match self.client_state.try_follow_mut(this_player()) {
                        Some(player) => player.update_stat(stat_type),
                        // The initial stat burst can arrive before the local player entity
                        // was created; buffer it instead of dropping it.
                        None => self.pending_stat_updates.push(stat_type),
                    }

                    if let Some((base, level)) = level_up {
                        self.push_notification(
                            NotificationKind::LevelUp,
                            {
                                match base {
                                    true => format!("Base level up! Lv {level}"),
                                    false => format!("Job level up! Lv {level}"),
                                }
                            },
                            client_tick,
                        );
                    }
                }
                NetworkEvent::OpenEditDialog { npc_id, string_input } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .start_input(npc_id, string_input);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::OpenDialog { text, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .initialize(npc_id)
                        .add_text(text);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddNextButton { npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // An NPCs could start the dialog with this packet so we want to make sure it's initialized.
                        .initialize(npc_id)
                        .add_next_button();

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddCloseButton { npc_id } => {
                    // Some NPCs send the `CloseButtonPacket` after the dialog
                    // has been closed. We want to filter these out because otherwise we get a
                    // close button at the start of the next dialog.
                    if self.interface.is_window_with_class_open(WindowClass::Dialog) {
                        self.client_state
                            .follow_mut(client_state().dialog_window())
                            // Technically this call is redundant since the window is already open
                            // but we keep it for consistency.
                            .initialize(npc_id)
                            .add_close_button();
                    }
                }
                NetworkEvent::AddChoiceButtons { choices, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // Some NPCs start the dialog with this packet so we need to make sure it's initialized.
                        .initialize(npc_id)
                        .add_choice_buttons(choices);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::WarpPointList { skill_id, maps } => {
                    // Warp Portal (`AL_WARP`) finished casting: the server asks for
                    // the destination map and creates the portal units only after
                    // the client answered. Show the destination list in the dialog
                    // window and send the choice back in `SelectWarpPoint`.
                    if maps.is_empty() {
                        continue;
                    }

                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .start_warp_point_list(skill_id, maps);
                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddQuestEffect { quest_effect } => {
                    if let Some(map) = &self.map {
                        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
                    }
                }
                NetworkEvent::RemoveQuestEffect { entity_id } => self.particle_holder.remove_quest_icon(entity_id),
                NetworkEvent::SetInventory { items } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .fill(&self.async_loader, items);
                }
                // --- Item service: refine / repair / identify / card insert ----
                // rAthena pushes the selectable list after the matching skill
                // (or Magnifier) was used; the window answers it with the
                // matching CZ request. The opcode/layout references live on the
                // packet structs in `ragnarok-packets`.
                NetworkEvent::WeaponRefineList { entries } => {
                    let entries = entries
                        .into_iter()
                        .map(|entry| {
                            let name = self.library.item_name(entry.item_id, true).into_owned();
                            ItemServiceEntry::new(entry.index, entry.item_id, name, entry.refine)
                        })
                        .collect();

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .show(ItemServiceKind::WeaponRefine, entries);
                    self.open_item_service_window(ItemServiceKind::WeaponRefine);
                }
                NetworkEvent::WeaponRefineResult { result, item_id } => {
                    let name = self.library.item_name(item_id, true).into_owned();
                    let message = match result {
                        0 => format!("Your {name} was successfully refined."),
                        1 => format!("The refine of your {name} failed and it lost its refine level."),
                        2 => format!("Your {name} cannot be refined further until you raise the Weapon Refine skill level."),
                        3 => format!("You lack the material required to refine your {name}."),
                        _ => format!("The refine attempt for your {name} failed."),
                    };

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .set_status(message.clone());

                    match result {
                        0 => self.push_chat_information(message),
                        _ => self.push_chat_warning(message),
                    }
                }
                NetworkEvent::ItemRefiningResult { index, refine, result } => {
                    // rAthena also re-sends the inventory list after a refine
                    // (`skill.cpp:20753`); this keeps the refine level and the
                    // composed item name current even before the list arrives.
                    let refinement_level = u8::try_from(refine).unwrap_or(u8::MAX);
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_refinement(&self.async_loader, index, refinement_level);

                    if *self.client_state.follow(client_state().item_service().kind()) == Some(ItemServiceKind::WeaponRefine)
                        && let Some(entry) = self
                            .client_state
                            .follow_mut(client_state().item_service())
                            .entries
                            .iter_mut()
                            .find(|entry| entry.index == index)
                    {
                        entry.refine = refinement_level;
                    }

                    // `ZC_ACK_ITEMREFINING` is the only result packet of the NPC
                    // refine UI (`clif.cpp:22720`-`22750`); show it there.
                    if *self.client_state.follow(client_state().item_service().refining().open()) {
                        let message = match result {
                            0 => "The refine succeeded.".to_owned(),
                            1 => "The refine failed and the item broke.".to_owned(),
                            2 => "The refine failed and the item lost refine levels.".to_owned(),
                            _ => "The refine failed.".to_owned(),
                        };

                        // A break destroys the item (`pc_delitem`), so go back to
                        // the item picker; every other result is followed by a
                        // fresh material list (0x0AA2) that replaces the current
                        // view. `start` resets the status line, hence the order.
                        let candidates = match result {
                            1 => Some(self.refinable_inventory_items()),
                            _ => None,
                        };

                        {
                            let refining = self.client_state.follow_mut(client_state().item_service().refining());

                            if let Some(candidates) = candidates {
                                refining.start(candidates);
                            }

                            refining.set_status(message.clone());
                        }

                        match result {
                            0 => self.push_chat_information(message),
                            _ => self.push_chat_warning(message),
                        }

                        if result == 1 {
                            self.open_refining_window(RefiningMode::SelectItem);
                        }
                    }

                    crate::logging::log_debug!("item service: refine result {} for item {}", result, index.0);
                }
                NetworkEvent::RepairItemList { entries } => {
                    let entries = entries
                        .into_iter()
                        .map(|entry| {
                            let name = self.library.item_name(entry.item_id, true).into_owned();
                            // The repair list sends raw 0-based server slots, not the
                            // `slot + 2` client index (`clif.cpp:7153`).
                            ItemServiceEntry::new(InventoryIndex(entry.index), entry.item_id, name, entry.refine)
                        })
                        .collect();

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .show(ItemServiceKind::ItemRepair, entries);
                    self.open_item_service_window(ItemServiceKind::ItemRepair);
                }
                NetworkEvent::RepairItemResult { index, success } => {
                    self.client_state.follow_mut(client_state().inventory()).set_broken(index, !success);

                    let message = match success {
                        true => "The item was repaired.".to_owned(),
                        false => "The item could not be repaired.".to_owned(),
                    };

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .set_status(message.clone());

                    match success {
                        true => self.push_chat_information(message),
                        false => self.push_chat_warning(message),
                    }
                }
                NetworkEvent::IdentifyItemList { indexes } => {
                    let entries = indexes
                        .into_iter()
                        .filter_map(|index| self.inventory_service_entry(index))
                        .collect();

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .show(ItemServiceKind::ItemIdentify, entries);
                    self.open_item_service_window(ItemServiceKind::ItemIdentify);
                }
                NetworkEvent::IdentifyItemResult { index, success } => {
                    // rAthena only sends the ack; the item itself is not part of
                    // the packet, so the client flips the identified flag and
                    // re-resolves the name/description (both depend on it).
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .set_identified(&self.async_loader, index, success);

                    let message = match success {
                        true => "The item was identified.".to_owned(),
                        false => "The item could not be identified.".to_owned(),
                    };

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .set_status(message.clone());

                    match success {
                        true => self.push_chat_information(message),
                        false => self.push_chat_warning(message),
                    }
                }
                NetworkEvent::CardCompositionList { indexes } => {
                    let entries: Vec<ItemServiceEntry> = indexes
                        .into_iter()
                        .filter_map(|index| self.inventory_service_entry(index))
                        .collect();

                    let card = self.client_state.follow(client_state().item_service().card()).clone();

                    match card {
                        Some(card) => self
                            .client_state
                            .follow_mut(client_state().item_service())
                            .show_card_list(card, entries),
                        None => self
                            .client_state
                            .follow_mut(client_state().item_service())
                            .show(ItemServiceKind::CardComposition, entries),
                    }

                    self.open_item_service_window(ItemServiceKind::CardComposition);
                }
                NetworkEvent::CardCompositionResult {
                    equip_index,
                    card_index,
                    success,
                } => {
                    if success {
                        // `pc_insert_card` does not re-send the equipment; it only
                        // deletes the card from the inventory (`pc.cpp:6238`). The
                        // card id is taken from the selected card (falling back to
                        // the inventory if the delete packet already arrived).
                        let card_item_id = self
                            .client_state
                            .follow(client_state().item_service().card())
                            .as_ref()
                            .filter(|card| card.index == card_index)
                            .map(|card| card.item_id)
                            .or_else(|| {
                                self.client_state
                                    .follow(client_state().inventory().items())
                                    .iter()
                                    .find(|item| item.index == card_index)
                                    .map(|item| item.item_id)
                            });

                        if let Some(card_item_id) = card_item_id {
                            self.client_state.follow_mut(client_state().inventory()).insert_card(
                                &self.async_loader,
                                equip_index,
                                card_item_id,
                            );
                        }

                        // The card is removed from the inventory by the
                        // server's own `pc_delitem`
                        // delete packet, which rAthena sends *before*
                        // the ack (`pc.cpp:6238` then `6247`). Removing it here
                        // as well would delete one card
                        // too many when the player owns
                        // a stack of the same card.
                    }

                    let message = match success {
                        true => "The card was inserted.".to_owned(),
                        false => "The card could not be inserted.".to_owned(),
                    };

                    self.client_state
                        .follow_mut(client_state().item_service())
                        .set_status(message.clone());

                    match success {
                        true => self.push_chat_information(message),
                        false => self.push_chat_warning(message),
                    }
                }
                NetworkEvent::CardSlotUpdated {
                    wear_state,
                    card_slot,
                    item_id,
                    equip_flag,
                } => {
                    // rAthena 20190605 defines ZC_UPDATE_CARDSLOT but never sends
                    // it (`grep -n UPDATE_CARDSLOT src/map/clif.cpp` has no
                    // match); the normal card insert path updates the slot from
                    // the ack above. Apply it defensively for servers that do
                    // send it: `wear_state` is the equipment position, `card_slot`
                    // the slot index and `item_id` the inserted card.
                    crate::logging::log_debug!(
                        "item service: ZC_UPDATE_CARDSLOT wear={} slot={} item={} flag={}",
                        wear_state,
                        card_slot,
                        item_id.0,
                        equip_flag
                    );

                    let equipment_position = EquipPosition::from_bits_truncate(u32::from(wear_state as u16));
                    if (0..4).contains(&card_slot) && !equipment_position.is_empty() {
                        let index = self
                            .client_state
                            .follow(client_state().inventory().items())
                            .iter()
                            .find(|item| {
                                matches!(
                                    &item.details,
                                    ro_next_networking::InventoryItemDetails::Equippable { equipped_position, .. }
                                        if equipped_position.intersects(equipment_position)
                                )
                            })
                            .map(|item| item.index);

                        if let Some(index) = index {
                            self.client_state.follow_mut(client_state().inventory()).set_card_slot(
                                &self.async_loader,
                                index,
                                card_slot as usize,
                                item_id,
                            );
                        }
                    }
                }
                NetworkEvent::RefiningUiOpened => {
                    // The server opened the NPC refine UI; it does not send a
                    // candidate list, so offer the player's own refinable
                    // equipment and ask for the materials after the pick.
                    let candidates = self.refinable_inventory_items();
                    self.client_state
                        .follow_mut(client_state().item_service().refining())
                        .start(candidates);
                    self.open_refining_window(RefiningMode::SelectItem);
                }
                NetworkEvent::RefiningMaterialList {
                    item_index,
                    blacksmith_blessing,
                    materials,
                } => {
                    let item = self
                        .inventory_service_entry(item_index)
                        // Another player's item is not in our inventory; the
                        // server list still carries its item id? It does not, so
                        // fall back to the index for the display and let the
                        // server validate the request.
                        .unwrap_or_else(|| ItemServiceEntry::new(item_index, ItemId(0), format!("Item #{}", item_index.0), 0));

                    let materials = materials
                        .into_iter()
                        .map(|material| {
                            let name = self.library.item_name(material.item_id, true).into_owned();
                            RefiningMaterial {
                                item_id: material.item_id,
                                name,
                                chance: u8::try_from(material.chance).unwrap_or(0),
                                zeny: material.zeny,
                            }
                        })
                        .collect();

                    self.client_state
                        .follow_mut(client_state().item_service().refining())
                        .show_materials(item, u8::try_from(blacksmith_blessing).unwrap_or(0), materials);
                    self.open_refining_window(RefiningMode::SelectMaterial);
                }
                NetworkEvent::EquipmentSwitchItems { items } => {
                    // `ZC_EQUIPSWITCH_LIST` pre-assigns inventory items to
                    // equipment positions (the equip-switch window). Recording the
                    // position in the inventory state makes those items visible on
                    // the paper doll and lets the normal equip/unequip flow move
                    // them, instead of dropping the packet.
                    let inventory = self.client_state.follow_mut(client_state().inventory());

                    for item in &items {
                        let position = EquipPosition::from_bits_truncate(item.position);
                        inventory.update_equipped_position(item.index, position);
                    }

                    crate::logging::log_debug!("equip switch list: {} pre-assigned item(s)", items.len());
                }
                NetworkEvent::SetStorage { items, name } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .fill(&self.async_loader, items);
                    *self.client_state.follow_mut(client_state().storage_name()) = name;

                    if !self.interface.is_window_with_class_open(WindowClass::Storage) {
                        self.interface.open_window(StorageWindow);
                    }
                }
                NetworkEvent::SetStorageAmount { amount, maximum_amount } => {
                    *self.client_state.follow_mut(client_state().storage_amount()) = amount;
                    *self.client_state.follow_mut(client_state().storage_maximum_amount()) = maximum_amount;
                }
                NetworkEvent::StorageItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .add_item(&self.async_loader, item);
                }
                NetworkEvent::StorageItemRemoved { storage_index, amount } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .remove_item(InventoryIndex(storage_index), amount as u16);
                }
                NetworkEvent::StorageClosed => {
                    self.close_storage_window();
                }
                NetworkEvent::OpenUserInterface { ui_type, data: _ } => {
                    // `OUT_UI_BANK` (0, `clif.hpp:1416`) is the NPC `openbank`
                    // trigger. The window itself is opened by the banking request.
                    if ui_type == 0 {
                        self.open_bank_window();
                    }
                }
                NetworkEvent::BankOpened => {
                    if !self.interface.is_window_with_class_open(WindowClass::Bank) {
                        self.interface.open_window(BankWindow);
                    }

                    self.client_state.follow_mut(client_state().bank()).set_status("Bank opened.");
                }
                NetworkEvent::BankBalance { money, reason } => {
                    let bank = self.client_state.follow_mut(client_state().bank());
                    bank.set_balance(money);
                    bank.set_status(match reason {
                        0 => "Bank ready.",
                        _ => "The bank is not available on this map or server.",
                    });

                    if !self.interface.is_window_with_class_open(WindowClass::Bank) {
                        self.interface.open_window(BankWindow);
                    }
                }
                NetworkEvent::BankDepositResult { reason, money, zeny } => {
                    self.client_state.follow_mut(client_state().bank()).apply_deposit(reason, money);

                    // The ack carries the carried zeny after the transaction; the
                    // regular 0x00B1 zeny update usually follows as well.
                    if let Ok(zeny) = u64::try_from(zeny) {
                        *self.client_state.follow_mut(client_state().zeny()) = Some(zeny);
                    }

                    if !self.interface.is_window_with_class_open(WindowClass::Bank) {
                        self.interface.open_window(BankWindow);
                    }
                }
                NetworkEvent::BankWithdrawResult { reason, money, zeny } => {
                    self.client_state.follow_mut(client_state().bank()).apply_withdraw(reason, money);

                    if let Ok(zeny) = u64::try_from(zeny) {
                        *self.client_state.follow_mut(client_state().zeny()) = Some(zeny);
                    }

                    if !self.interface.is_window_with_class_open(WindowClass::Bank) {
                        self.interface.open_window(BankWindow);
                    }
                }
                NetworkEvent::BankClosed => {
                    self.client_state.follow_mut(client_state().bank()).close();
                    self.interface.close_window_with_class(WindowClass::Bank);
                }
                NetworkEvent::SetCart { items } => {
                    // The cart list arrives on map entry and whenever the cart is
                    // (re)opened. It is kept in state so the cart window can be
                    // opened later without a dedicated request packet.
                    self.client_state.follow_mut(client_state().cart()).fill(&self.async_loader, items);
                }
                NetworkEvent::CartItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().cart())
                        .add_item(&self.async_loader, item);
                }
                NetworkEvent::CartItemRemoved { cart_index, amount } => {
                    self.client_state
                        .follow_mut(client_state().cart())
                        .remove_item_if_present(cart_index, u16::try_from(amount).unwrap_or(u16::MAX));
                }
                NetworkEvent::CartClosed => {
                    self.close_cart_window();
                }
                NetworkEvent::CartAddItemFailed { result } => {
                    let message = match result {
                        0 => "The cart is too heavy to hold this item.",
                        _ => "The cart is full.",
                    };
                    self.push_chat_warning(message.to_owned());
                }
                NetworkEvent::StoreEntry {
                    owner_account_id,
                    shop_name,
                } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == owner_account_id.0)
                    {
                        entity.set_vending_shop_name(Some(shop_name));
                    }
                }
                NetworkEvent::StoreEntryRemoved { owner_account_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == owner_account_id.0)
                    {
                        entity.set_vending_shop_name(None);
                    }
                }
                NetworkEvent::VendingShopList {
                    owner_account_id,
                    shop_id,
                    items,
                } => {
                    let shop_name = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id().0 == owner_account_id.0)
                        .and_then(|entity| entity.get_vending_shop_name())
                        .map(ToOwned::to_owned)
                        .or_else(|| {
                            let pending = self
                                .client_state
                                .follow(client_state().vending_browse())
                                .pending_shop_name()
                                .to_owned();
                            (!pending.is_empty()).then_some(pending)
                        })
                        .unwrap_or_else(|| "Vending shop".to_owned());

                    let items = items
                        .into_iter()
                        .map(|entry| {
                            let stock = match entry.item.quantity {
                                ItemQuantity::Fixed(stock) => u16::try_from(stock).unwrap_or(u16::MAX),
                                ItemQuantity::Infinite => u16::MAX,
                            };
                            let item = self.async_loader.request_shop_item_metadata_load(entry.item);

                            VendingShopItem::new(
                                item.item_id.0,
                                item.metadata.name.clone(),
                                item.price.0,
                                stock,
                                entry.cart_index,
                            )
                        })
                        .collect();

                    self.client_state
                        .follow_mut(client_state().vending_browse())
                        .open(owner_account_id.0, shop_id, shop_name, items);

                    if !self.interface.is_window_with_class_open(WindowClass::VendingBrowse) {
                        self.interface
                            .open_window(VendingBrowseWindow::new(client_state().vending_browse()));
                    }
                }
                NetworkEvent::VendingPurchaseResult { result } => {
                    // The 2019 server only sends this ack for failures, but some
                    // servers also send result 0 (success); do not report that as an
                    // error.
                    if result != 0 {
                        self.push_chat_information(format_message(MessageDomain::PurchaseFromMc, u16::from(result), &[]));
                    }
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .add_item(&self.async_loader, item);

                    // TODO: Update the selling items. If you pick up an item
                    // that you already have the sell window
                    // should allow you to sell the new
                    // amount of items.
                }
                NetworkEvent::ItemObtained {
                    item_id,
                    quantity,
                    is_identified,
                } => {
                    let name = self.library.item_name(item_id, is_identified);
                    let message =
                        ro_next_messages::render_template(self.client_state.follow(client_state().localization().chat_item_obtained()), &[
                            ("name", name.as_ref()),
                            ("quantity", &quantity.to_string()),
                        ]);
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, MessageColor::Information));
                }
                NetworkEvent::InventoryItemRemoved { index, amount, .. } => {
                    self.client_state.follow_mut(client_state().inventory()).remove_item(index, amount);
                }
                NetworkEvent::SkillTree { skill_information } => {
                    crate::logging::log_debug!("skills: initial skill list with {} entries", skill_information.len());

                    if skill_information.is_empty() {
                        // An empty list is valid for a character without skills, but if the
                        // server DB has learned skills it means the list was lost (stream
                        // desync) - make that visible instead of silently showing an empty
                        // skill tree.
                        crate::logging::log_warn!("skills: server sent an empty skill list (learned skills may be missing)");
                    }
                    *self.client_state.follow_mut(client_state().skill_tree().skills()) =
                        skill_information.into_iter().map(LearnedSkill::new).collect();

                    self.merge_learned_skills_into_skill_tree(client_tick);
                }
                NetworkEvent::UpdateSkillInfo {
                    skill_id,
                    skill_type,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                } => {
                    // Partial skill update (level up, SP cost/range change). The
                    // name is not part of this packet, so only known skills are
                    // updated; the tree/hotbar read the new level from this entry.
                    crate::logging::log_debug!("skills: update skill {} level {}", skill_id.0, skill_level.0);
                    let skills = self.client_state.follow_mut(client_state().skill_tree().skills());

                    if let Some(learned_skill) = skills.iter_mut().find(|learned_skill| learned_skill.skill_id == skill_id) {
                        learned_skill.skill_type = skill_type;
                        learned_skill.skill_level = skill_level;
                        learned_skill.spell_point_cost = spell_point_cost;
                        learned_skill.attack_range = attack_range;
                        learned_skill.upgradable = upgradable;
                    }
                }
                NetworkEvent::AddSkill { skill_information } => {
                    // A skill that was learned/upgraded after the initial skill list.
                    // Replace the old entry so the skill tree and the hotbar pick up
                    // the new level immediately, without a relog.
                    let learned_skill = LearnedSkill::new(skill_information);
                    crate::logging::log_debug!(
                        "skills: add skill {} level {}",
                        learned_skill.skill_id.0,
                        learned_skill.skill_level.0
                    );
                    let skills = self.client_state.follow_mut(client_state().skill_tree().skills());

                    match skills.iter_mut().find(|existing| existing.skill_id == learned_skill.skill_id) {
                        Some(existing) => *existing = learned_skill,
                        None => skills.push(learned_skill),
                    }

                    self.merge_learned_skills_into_skill_tree(client_tick);
                }
                NetworkEvent::ItemUsed { item_id } => {
                    // The official client plays the use-item effect (EF_EXIT)
                    // for consumables above the classic item ids
                    // (`IsAboveItemId600` in the classic client).
                    if item_id.0 > 600
                        && let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id())
                    {
                        self.spawn_item_use_effect(entity_id);
                    }
                }
                NetworkEvent::InventoryItemAmountChanged { index, amount } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_amount(index, amount);
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_equipped_position(index, equipped_position);
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    else {
                        // Not every job change belongs to an entity the client knows
                        // about: the population engine also changes the jobs of its
                        // in-memory shell players, whose ids are not the ids the client
                        // received in the spawn packets. Ignore those updates instead of
                        // panicking on them.
                        #[cfg(feature = "debug")]
                        print_debug!(
                            "[{}] ignoring job change of unknown entity {:?}",
                            "warning".yellow(),
                            account_id
                        );
                        // The entity is not in the world yet (e.g. the server sent the job
                        // change while the map was still loading). There is nothing to update.
                        continue;
                    };

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    if !self.library.contains::<crate::world::SkillTreeLayout>(job_id) {
                        // Population/LLM shells change their job to jobs that this
                        // client's trimmed skill-tree tables do not contain. Log the
                        // offending job id (harmless; the local skill tree is only
                        // updated for the own character) to make future investigations
                        // of missing job tables easy.
                        eprintln!(
                            "[skill_tree] job {:?} (entity {:?}) is not in the client skill-tree tables",
                            job_id, account_id
                        );
                    }

                    entity.set_job(&self.library, job_id);
                    self.entity_lifecycle.on_job_change(entity.get_entity_id(), job_id);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }

                    if entity.has_falcon()
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity.get_entity_id())
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }

                    request_equipment_layers(&self.async_loader, entity);

                    // Only the own character's job change affects the local skill tree
                    // window. Population shells send job changes for jobs that do not
                    // exist in this client's skill tree tables, which used to panic when
                    // the skill tree layout was looked up for them.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id().0 == account_id.0)
                    {
                        let layout = self.async_loader.request_skill_tree_layout_load(job_id, client_tick);
                        *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) =
                            layout.tabs.len().saturating_sub(1);
                        *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                        self.client_state
                            .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                            .clear();
                    }
                }
                NetworkEvent::ChangeHair { account_id, hair_id } => {
                    let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    else {
                        #[cfg(feature = "debug")]
                        print_debug!(
                            "[{}] ignoring hair change of unknown entity {:?}",
                            "warning".yellow(),
                            account_id
                        );
                        continue;
                    };

                    entity.set_hair(hair_id as usize);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }

                    if entity.has_falcon()
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity.get_entity_id())
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }

                    request_equipment_layers(&self.async_loader, entity);
                }
                NetworkEvent::AppearanceChanged {
                    account_id,
                    sprite_type,
                    value,
                } => {
                    let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    else {
                        continue;
                    };

                    let Some(outcome) = entity.apply_sprite_change(sprite_type, value) else {
                        continue;
                    };

                    if outcome.body {
                        // Sprite layers changed: reload the animation data so both the
                        // world sprite and the equipment paper doll show the new appearance.
                        if let Some(animation_data) = self.async_loader.request_animation_data_load(
                            entity.get_entity_id(),
                            entity.get_entity_type(),
                            entity.get_entity_part_files(&self.library),
                        ) {
                            entity.set_animation_data(animation_data);
                        }
                    }

                    if outcome.weapon || outcome.shield || outcome.robe {
                        // Equipping/unequipping a weapon, shield or robe does not
                        // change the body sprite; only the equipment layers change.
                        request_equipment_layers(&self.async_loader, entity);
                    }
                }
                NetworkEvent::AutoRunSkill { skill_id, skill_level } => {
                    // Skill items (fly wing, casting scrolls, ...): the server only
                    // tells us which skill the item wants; the client has to send the
                    // skill request itself (that is what the original client does).
                    // Item skills are self-targeted on this server setup (fly wing's
                    // AL_TELEPORT level 1), so cast it on the local player.
                    let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) else {
                        continue;
                    };

                    let _ = self.networking_system.cast_skill(skill_id, skill_level, entity_id);
                }
                NetworkEvent::LocalAppearanceChanged { sprite_type, value } => {
                    let Some(entity) = self.client_state.try_follow_mut(this_entity()) else {
                        continue;
                    };

                    let Some(outcome) = entity.apply_sprite_change(sprite_type, value) else {
                        continue;
                    };

                    if outcome.body {
                        // Sprite layers changed: reload the animation data so that both the
                        // world sprite and the equipment paper doll show the new appearance.
                        if let Some(animation_data) = self.async_loader.request_animation_data_load(
                            entity.get_entity_id(),
                            entity.get_entity_type(),
                            entity.get_entity_part_files(&self.library),
                        ) {
                            entity.set_animation_data(animation_data);
                        }
                    }

                    if outcome.weapon || outcome.shield || outcome.robe {
                        request_equipment_layers(&self.async_loader, entity);
                    }
                }
                NetworkEvent::LoggedOut => {
                    // The server confirmed the logout; switch away from the map
                    // music right away so there is no silent gap while the map
                    // server disconnects and the menu map is loaded again.
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.client_state.follow_mut(client_state().spirit_orbs()).clear();
                    self.client_state.follow_mut(client_state().mail()).clear();
                    self.audio_engine
                        .play_background_music_track(crate::client_config::background_music());
                    self.networking_system.disconnect_from_map_server();
                }
                NetworkEvent::FriendRequest { requestee } => {
                    crate::state::friend_names::remember(requestee.character_id.0, &requestee.name);
                    self.interface.open_window(FriendRequestWindow::new(requestee));
                }
                // --- Rodex mail (2019) -----------------------------------------
                NetworkEvent::MailList { mails } => {
                    self.client_state.follow_mut(client_state().mail()).apply_list(mails);
                }
                NetworkEvent::MailRead {
                    mail_id,
                    open_type: _,
                    body,
                    zeny,
                    items,
                } => {
                    self.client_state
                        .follow_mut(client_state().mail())
                        .apply_read(mail_id, body, zeny, items);
                }
                NetworkEvent::MailAttachmentClaimed { mail_id, zeny, result } => {
                    let mailbox = self.client_state.follow_mut(client_state().mail());
                    mailbox.apply_claimed(mail_id, zeny, result);
                    let status = mailbox.status.clone();
                    self.push_chat_information(status);
                }
                NetworkEvent::MailDeleted { mail_id } => {
                    let mailbox = self.client_state.follow_mut(client_state().mail());
                    mailbox.apply_deleted(mail_id);
                    let status = mailbox.status.clone();
                    self.push_chat_information(status);
                }
                NetworkEvent::MailUnreadStatus { unread } => {
                    self.client_state
                        .follow_mut(client_state().mail())
                        .set_unread_mail_available(unread);
                }
                NetworkEvent::MailboxWindow { open } => match open {
                    true => {
                        if !self.interface.is_window_with_class_open(WindowClass::Mail) {
                            self.interface.open_window(MailboxWindow::new(client_state().mail()));
                        }
                        // `ZC_MAIL_WINDOWS` only opens the window; the inbox
                        // has to be requested explicitly.
                        let _ = self.networking_system.open_mailbox();
                    }
                    false => self.interface.close_window_with_class(WindowClass::Mail),
                },
                // --- Party events (2019) -------------------------------------------
                NetworkEvent::PartyOption {
                    experience_option,
                    item_pickup,
                    item_share,
                } => {
                    let party = self.client_state.follow_mut(client_state().party());
                    party.experience_option = experience_option;
                    party.item_pickup = item_pickup != 0;
                    party.item_share = item_share != 0;
                }
                NetworkEvent::PartyList { party_name, members } => {
                    let was_in_party = self.client_state.follow(client_state().party()).is_in_party();

                    let party = self.client_state.follow_mut(client_state().party());

                    if members.is_empty() && !was_in_party {
                        continue;
                    }

                    party.apply_roster(party_name, &members);

                    if members.is_empty() {
                        party.clear();
                        self.interface.close_window_with_class(WindowClass::Party);
                    } else if !was_in_party {
                        // The player just joined a party (login burst or accepted
                        // invite): show the roster like the original client.
                        self.open_party_window();
                    }
                }
                NetworkEvent::PartyMemberAdded { member } => {
                    let was_in_party = self.client_state.follow(client_state().party()).is_in_party();

                    self.client_state.follow_mut(client_state().party()).apply_member_details(&member);

                    if !was_in_party {
                        self.open_party_window();
                    }
                }
                NetworkEvent::PartyMemberRemoved {
                    account_id,
                    character_name,
                    result,
                } => {
                    let own_account_id = self.saved_login_data.as_ref().map(|login_data| login_data.account_id);
                    let is_self = own_account_id == Some(account_id);

                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_member_removed(account_id);

                    let message = format_message(MessageDomain::PartyMemberRemoved, u16::from(result), &[(
                        "character_name",
                        &character_name,
                    )]);

                    self.push_chat_information(message);

                    if is_self {
                        self.client_state.follow_mut(client_state().party()).clear();
                        self.interface.close_window_with_class(WindowClass::Party);
                    }
                }
                NetworkEvent::PartyMemberHealth {
                    account_id,
                    health_points,
                    maximum_health_points,
                } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_health(account_id, health_points, maximum_health_points);
                }
                NetworkEvent::PartyMemberPosition { account_id, position } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_position(account_id, position);
                }
                NetworkEvent::PartyMemberJobLevel {
                    account_id,
                    job_id,
                    base_level,
                } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_job_level(account_id, job_id, base_level);
                }
                NetworkEvent::PartyMemberAlive { account_id, is_dead } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_alive(account_id, is_dead);
                }
                NetworkEvent::PartyInvite { party_id, party_name } => {
                    if self.client_state.follow(client_state().party()).is_in_party() {
                        // Already in a party; the server will reject the invitation
                        // anyway, but tell the user why nothing happens.
                        self.push_localized_chat(client_state().localization().chat_party_invite_ignored(), &[(
                            "name",
                            party_name.as_str(),
                        )]);
                        continue;
                    }

                    *self.client_state.follow_mut(client_state().party().invite()) =
                        Some(crate::state::party::PartyInvitePrompt { party_id, party_name });

                    self.interface.open_window(PartyInviteWindow::new(client_state().party().invite()));
                }
                NetworkEvent::PartyInviteReply { character_name, result } => {
                    self.interface.close_window_with_class(WindowClass::PartyInvite);
                    *self.client_state.follow_mut(client_state().party().invite()) = None;

                    let message = format_message(MessageDomain::PartyInviteReply, result.clamp(0, i32::from(u16::MAX)) as u16, &[
                        ("character_name", &character_name),
                    ]);

                    self.push_chat_information(message);
                }
                NetworkEvent::PartyCreationResult { result } => {
                    let message = format_message(MessageDomain::PartyCreation, u16::from(result), &[]);

                    self.push_chat_information(message);
                }
                NetworkEvent::PartyChatMessage { account_id, message } => {
                    let name = self
                        .client_state
                        .follow(client_state().party())
                        .member_name(account_id)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("Account #{}", account_id.0));

                    // The server echoes the full "<name> : <message>" string in
                    // `ZC_NOTIFY_CHAT_PARTY`, but the client already knows and
                    // renders the name, so strip the redundant prefix.
                    let message = message
                        .strip_prefix(&format!("{name} : "))
                        .unwrap_or(&message)
                        .trim_end_matches('\0');

                    self.push_chat_message(ChatMessage::new_channel(
                        format!("[Party] {name}: {message}"),
                        MessageColor::Rgb {
                            red: 120,
                            green: 230,
                            blue: 140,
                        },
                        ChatChannel::Party,
                    ));
                }
                NetworkEvent::PartyConfig { allow_invites } => {
                    self.client_state.follow_mut(client_state().party()).allow_invites = allow_invites;
                }
                // --- Party booking / LFG (2019) ------------------------------------
                NetworkEvent::PartyBookingRegisterResult { result } => {
                    let status = {
                        let booking = self.client_state.follow_mut(client_state().party_booking());
                        booking.apply_register_result(result);
                        booking.status_message.clone()
                    };
                    self.push_chat_information(status);
                }
                NetworkEvent::PartyBookingSearchResult { more_results, entries } => {
                    self.client_state
                        .follow_mut(client_state().party_booking())
                        .apply_search(more_results, entries);
                }
                NetworkEvent::PartyBookingDeleteResult { result } => {
                    let status = {
                        let booking = self.client_state.follow_mut(client_state().party_booking());
                        booking.apply_delete_result(result);
                        booking.status_message.clone()
                    };
                    self.push_chat_information(status);
                }
                NetworkEvent::PartyBookingInserted { entry } => {
                    self.client_state.follow_mut(client_state().party_booking()).apply_inserted(entry);
                }
                NetworkEvent::PartyBookingUpdated { index, jobs } => {
                    self.client_state
                        .follow_mut(client_state().party_booking())
                        .apply_updated(index, jobs);
                }
                NetworkEvent::PartyBookingDeleted { index } => {
                    self.client_state.follow_mut(client_state().party_booking()).apply_deleted(index);
                }
                NetworkEvent::PartyBookingJoinRequest {
                    character_id,
                    account_id,
                    name,
                    position,
                } => {
                    let message = {
                        let booking = self.client_state.follow_mut(client_state().party_booking());
                        booking.apply_join_request(character_id, account_id, name, position);
                        booking.join_requests.last().map(|request| request.name.clone())
                    };

                    if let Some(name) = message {
                        self.push_chat_information(format!("{name} requested to join your party through the party search."));
                    }

                    self.open_party_window();
                }
                NetworkEvent::PartyBookingJoinResult {
                    player_name,
                    party_name,
                    account_id,
                    refused,
                } => {
                    let message = {
                        let booking = self.client_state.follow_mut(client_state().party_booking());
                        booking.apply_join_result(player_name, party_name, account_id, refused);
                        booking.status_message.clone()
                    };
                    self.push_chat_information(message);
                }
                // --- Clan (2019) ---------------------------------------------------
                NetworkEvent::ClanInfo { info } => {
                    self.client_state.follow_mut(client_state().clan()).apply_info(info);
                }
                NetworkEvent::ClanOnlineCount {
                    online_members,
                    maximum_members,
                } => {
                    self.client_state
                        .follow_mut(client_state().clan())
                        .apply_online_count(online_members, maximum_members);
                }
                // --- Pet / homunculus / mercenary ---------------------------------
                NetworkEvent::PetProperty { property } => {
                    self.client_state
                        .follow_mut(client_state().companion())
                        .apply_pet_property(property);
                }
                NetworkEvent::PetStateChanged {
                    entity_id: _,
                    state_type,
                    value,
                } => {
                    self.client_state
                        .follow_mut(client_state().companion())
                        .apply_pet_state_change(state_type, value);
                }
                NetworkEvent::HomunculusProperty { property } => {
                    self.client_state
                        .follow_mut(client_state().companion())
                        .apply_homunculus_property(property);
                }
                NetworkEvent::HomunculusParameterChanged { parameter, value } => {
                    self.client_state
                        .follow_mut(client_state().companion())
                        .apply_homunculus_parameter_change(parameter, value);
                }
                NetworkEvent::MercenaryInfo { info } => {
                    self.client_state.follow_mut(client_state().companion()).apply_mercenary_info(info);
                }
                NetworkEvent::MercenaryParameterChanged { parameter, value } => {
                    self.client_state
                        .follow_mut(client_state().companion())
                        .apply_mercenary_parameter_change(parameter, value);
                }
                NetworkEvent::FriendRemoved { account_id, character_id } => {
                    let friend_list = self.client_state.follow_mut(client_state().friend_list());
                    if let Some(index) = friend_list
                        .iter()
                        .position(|friend| friend.account_id == account_id && friend.character_id == character_id)
                    {
                        friend_list.remove(index);
                        let friend_online = self.client_state.follow_mut(client_state().friend_online());
                        if index < friend_online.len() {
                            friend_online.remove(index);
                        }
                    }
                }
                NetworkEvent::FriendAdded { friend } => {
                    crate::state::friend_names::remember(friend.character_id.0, &friend.name);
                    self.client_state.follow_mut(client_state().friend_list()).push(friend);
                    self.client_state.follow_mut(client_state().friend_online()).push(false);
                }
                NetworkEvent::FriendOnlineState {
                    account_id,
                    character_id,
                    online,
                    name,
                } => {
                    crate::logging::log_debug!("friend 0x0206: cid={} online={} name={:?}", character_id.0, online, name);
                    crate::state::friend_names::remember(character_id.0, &name);

                    let index = self
                        .client_state
                        .follow(client_state().friend_list())
                        .iter()
                        .position(|friend| friend.character_id == character_id || friend.account_id == account_id);

                    if let Some(index) = index {
                        {
                            let friend_list = self.client_state.follow_mut(client_state().friend_list());
                            if !name.trim().is_empty() {
                                friend_list[index].name = name.clone();
                            } else if friend_list[index].name.trim().is_empty() {
                                friend_list[index].name = crate::state::friend_names::cached_name(character_id.0).unwrap_or_default();
                            }
                        }

                        let friend_count = self.client_state.follow(client_state().friend_list()).len();
                        let friend_online = self.client_state.follow_mut(client_state().friend_online());
                        friend_online.resize(friend_count, false);
                        friend_online[index] = online;
                    } else {
                        // The 0x0206 update can arrive before/without a 0x0201 entry
                        // (for example right after accepting a friend request): add the
                        // friend so the UI shows it immediately.
                        let display_name = if name.trim().is_empty() {
                            crate::state::friend_names::cached_name(character_id.0).unwrap_or_default()
                        } else {
                            name.clone()
                        };

                        let friend_count = {
                            let friend_list = self.client_state.follow_mut(client_state().friend_list());
                            friend_list.push(Friend {
                                account_id,
                                character_id,
                                name: display_name,
                            });
                            friend_list.len()
                        };

                        let friend_online = self.client_state.follow_mut(client_state().friend_online());
                        friend_online.resize(friend_count, false);
                        friend_online[friend_count - 1] = online;
                    }
                }
                NetworkEvent::VisualEffect { effect_path, entity_id } => {
                    self.spawn_entity_effect(effect_path, entity_id, Vector3::new(0.0, 9.0, 0.0), 1.0, Color::WHITE, 50.0);
                }
                NetworkEvent::AddSkillUnit {
                    entity_id,
                    unit_id,
                    position,
                    skill_level,
                } => {
                    // Warp portal skill units have no sprite in the game files; the
                    // client draws the classic portal effect for them (like for warp
                    // NPCs), see `render_warp_portals`. Ambient NPCs cast these in the
                    // Prontera plaza.
                    if matches!(unit_id, UnitId::WarpWaiting | UnitId::WarpActive) {
                        let world_position = self.map.as_ref().and_then(|map| map.get_world_position(position));

                        if let Some(world_position) = world_position {
                            // `WarpWaiting` and `WarpActive` are separate packets for the
                            // same unit, so replace the previous position (this also
                            // prevents duplicate visuals for the same portal). Drop a
                            // provisional portal of the same cell as well.
                            self.warp_skill_units.retain(|(unit_entity_id, unit_position, ..)| {
                                *unit_entity_id != entity_id
                                    && (*unit_entity_id >= EntityId(0xFF00_0000) || (*unit_position - world_position).magnitude() > 8.0)
                            });

                            // Pre-renewal Warp Portal lasts 5 s per skill level; the
                            // expiry is only a safety net (the server sends a removal
                            // packet when the unit ends).
                            let lifetime = Duration::from_millis(5_000 * u64::from(skill_level.max(1)) + 1_000);
                            self.warp_skill_units
                                .push((entity_id, world_position, Instant::now(), Instant::now() + lifetime));
                        }

                        continue;
                    }

                    let Some(map) = &self.map else {
                        continue;
                    };

                    match unit_id {
                        UnitId::Firewall => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let Ok(effect) = self.effect_loader.get_or_load("firewall.str", &self.texture_loader) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] failed to load firewall effect", "error".red());
                                continue;
                            };
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    1.0,
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(255, 30, 0),
                                    60.0,
                                    true,
                                )),
                                entity_id,
                            );
                        }
                        UnitId::Pneuma => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let Ok(effect) = self.effect_loader.get_or_load("pneuma1.str", &self.texture_loader) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] failed to load pneuma effect", "error".red());
                                continue;
                            };
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    1.0,
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(83, 220, 108),
                                    40.0,
                                    false,
                                )),
                                entity_id,
                            );
                        }
                        _ => {}
                    }
                }
                NetworkEvent::RemoveSkillUnit { entity_id } => {
                    self.effect_holder.remove_unit(entity_id);
                    self.warp_skill_units.retain(|(unit_entity_id, ..)| *unit_entity_id != entity_id);
                }
                NetworkEvent::SetFriendList { friend_list } => {
                    let friend_list = friend_list
                        .into_iter()
                        .map(|mut friend| {
                            if friend.name.trim().is_empty() {
                                if let Some(name) = crate::state::friend_names::cached_name(friend.character_id.0) {
                                    friend.name = name;
                                }
                            } else {
                                crate::state::friend_names::remember(friend.character_id.0, &friend.name);
                            }
                            friend
                        })
                        .collect::<Vec<_>>();
                    let friend_count = friend_list.len();

                    crate::logging::log_debug!(
                        "friend list: {} entries (names filled from the persistent cache where known)",
                        friend_count
                    );

                    *self.client_state.follow_mut(client_state().friend_list()) = friend_list;
                    *self.client_state.follow_mut(client_state().friend_online()) = vec![false; friend_count];
                }
                NetworkEvent::SetHotkeyData { tab, hotkeys } => {
                    // The 2019 server sends 4 tabs with 38 entries each. Keep all of
                    // them and distinguish skill hotkeys from item hotkeys (the old
                    // code treated every entry as a skill id, so item hotkeys were
                    // "cast" as skills and every tab but 0 was dropped).
                    let tab_index = tab.0 as usize;

                    if tab_index >= HOTBAR_TAB_COUNT {
                        continue;
                    }

                    let job_id = self.client_state.try_follow(this_entity()).map(Entity::get_job_id);

                    for (index, hotkey) in hotkeys.into_iter().take(HOTBAR_ENTRIES_PER_TAB).enumerate() {
                        match hotkey {
                            HotkeyState::Bound(hotkey) => match hotkey.hotkey_type {
                                HotkeyType::Skill => {
                                    let Some(job_id) = job_id else {
                                        continue;
                                    };

                                    let skill_id = SkillId(hotkey.item_or_skill_id as u16);

                                    let mut skill = self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick);
                                    skill.maximum_level.0 = hotkey.quantity_or_skill_level;

                                    self.client_state
                                        .follow_mut(client_state().hotbar())
                                        .apply_skill(tab_index, index, Some(skill));
                                }
                                HotkeyType::Item => {
                                    self.client_state.follow_mut(client_state().hotbar()).apply_item(
                                        tab_index,
                                        index,
                                        ItemId(hotkey.item_or_skill_id),
                                        hotkey.quantity_or_skill_level,
                                    );
                                }
                            },
                            HotkeyState::Unbound => self.client_state.follow_mut(client_state().hotbar()).clear_entry(tab_index, index),
                        }
                    }

                    // Refresh the visible slots when the active tab was updated
                    // (also for tabs other than 0).
                    self.client_state.follow_mut(client_state().hotbar()).refresh_visible_slots();
                }
                NetworkEvent::OpenShop { items } => {
                    self.shop_open = true;

                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    *self.client_state.follow_mut(client_state().shop_items()) = items
                        .into_iter()
                        .map(|item| self.async_loader.request_shop_item_metadata_load(item))
                        .collect();

                    self.interface
                        .open_window(BuyWindow::new(client_state().shop_items(), client_state().buy_cart()));
                    self.interface.open_window(BuyCartWindow::new(client_state().buy_cart()));
                }
                NetworkEvent::AskBuyOrSell { shop_id } => {
                    self.interface.open_window(BuyOrSellWindow::new(shop_id));
                }
                NetworkEvent::BuyingCompleted { result } => match result {
                    BuyItemResult::Successful => {
                        let _ = self.networking_system.close_shop();
                        self.shop_open = false;

                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Buy);
                        self.interface.close_window_with_class(WindowClass::BuyCart);
                    }
                    result => {
                        let message = format_message(MessageDomain::BuyItem, buy_item_result_code(&result), &[]);

                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new(message, MessageColor::Error));
                    }
                },
                NetworkEvent::SellItemList { items } => {
                    self.shop_open = true;

                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    let inventory_items = self.client_state.follow(client_state().inventory().items());
                    let sell_items = items
                        .into_iter()
                        .map(|item| {
                            let inventory_item = inventory_items
                                .iter()
                                .find(|inventory_item| inventory_item.index == item.inventory_index)
                                .expect("item not in inventory");

                            let name = inventory_item.metadata.name.clone();
                            let texture = inventory_item.metadata.texture.clone();
                            let stats = inventory_item.metadata.stats;
                            let description = inventory_item.metadata.description.clone();
                            let card_names = inventory_item.metadata.card_names.clone();
                            let slot_count = inventory_item.metadata.slot_count;
                            let tooltip_text = inventory_item.metadata.tooltip_text.clone();
                            let quantity = match &inventory_item.details {
                                ro_next_networking::InventoryItemDetails::Regular { amount, .. } => *amount,
                                ro_next_networking::InventoryItemDetails::Equippable { .. } => 1,
                            };

                            SellItem {
                                metadata: (
                                    ResourceMetadata {
                                        name,
                                        texture,
                                        stats,
                                        description,
                                        card_names,
                                        slot_count,
                                        tooltip_text,
                                    },
                                    quantity,
                                ),
                                inventory_index: item.inventory_index,
                                price: item.price,
                                overcharge_price: item.overcharge_price,
                            }
                        })
                        .collect();

                    *self.client_state.follow_mut(client_state().sell_items()) = sell_items;

                    self.interface
                        .open_window(SellWindow::new(client_state().sell_items(), client_state().sell_cart()));
                    self.interface.open_window(SellCartWindow::new(client_state().sell_cart()));
                }
                NetworkEvent::SellingCompleted { result } => match result {
                    SellItemsResult::Success => {
                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Sell);
                        self.interface.close_window_with_class(WindowClass::SellCart);
                    }
                    SellItemsResult::Error => {
                        self.push_localized_chat(client_state().localization().chat_sell_failed(), &[]);
                    }
                },
                NetworkEvent::AttackFailed {
                    target_entity_id,
                    target_position,
                    player_position,
                    attack_range,
                } => {
                    if let Some(map) = &self.map
                        && self.client_state.try_follow_mut(this_entity()).is_some()
                        // Make sure that the entity is on screen.
                        && self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .any(|entity| entity.get_entity_id() == target_entity_id)
                        // Re-apply the PvP/GvG/PK targeting rules: the server
                        // can report an out-of-range attack for a player that
                        // this map does not make attackable.
                        && self.can_attack_entity(target_entity_id)
                        && let Some(path) =
                            self.path_finder
                                .find_walkable_path_in_range(&TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())), player_position, target_position, attack_range)
                    {
                        let nearest_tile = path.last().unwrap();

                        let _ = self.networking_system.player_move(WorldPosition {
                            x: nearest_tile.x,
                            y: nearest_tile.y,
                            direction: Direction::North,
                        });

                        *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity {
                            entity_id: target_entity_id,
                        });
                    }
                }
                NetworkEvent::UpdateSkill {
                    skill_id,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.client_state.follow_mut(client_state().skill_tree()).update_skill(
                        skill_id,
                        skill_level,
                        spell_point_cost,
                        attack_range,
                        upgradable,
                    );
                }
                NetworkEvent::RemoveSkill { skill_id } => {
                    self.client_state.follow_mut(client_state().skill_tree()).remove_skill(skill_id);
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .remove(&skill_id);
                }
                // --- Feedback for small/no-op packets (exp, skill fail, ammo, ban)
                NetworkEvent::GainedExperience {
                    amount,
                    experience_type,
                    experience_source,
                } => {
                    self.pending_experience.add(amount, experience_type, experience_source);
                }
                NetworkEvent::SkillUseFailed { skill_id, cause } => {
                    let skill_id_text = skill_id.0.to_string();
                    let message = format_message(MessageDomain::SkillUseFailed, u16::from(cause), &[("skill_id", &skill_id_text)]);

                    self.push_chat_information(message);
                }
                NetworkEvent::StatUpFailed => {
                    self.push_localized_chat(client_state().localization().chat_cannot_raise_stat(), &[]);
                }
                NetworkEvent::AmmunitionAction { action } => {
                    self.push_chat_information(format_message(
                        MessageDomain::AmmunitionAction,
                        ammunition_action_code(&action),
                        &[],
                    ));
                }
                NetworkEvent::AmmunitionEquipped { inventory_index } => {
                    crate::logging::log_debug!("ammunition equipped from inventory index {:?}", inventory_index);
                }
                NetworkEvent::UserCount { player_count } => {
                    self.push_localized_chat(client_state().localization().chat_players_online(), &[(
                        "count",
                        &player_count.to_string(),
                    )]);
                }
                NetworkEvent::MapServerBan { error_code } => {
                    self.push_chat_information(format_message(MessageDomain::MapServerBan, u16::from(error_code), &[]));
                    self.session.map_server_disconnected();
                    self.return_to_character_selection();
                }
                NetworkEvent::RefuseEnter { error_code } => {
                    self.push_localized_chat(client_state().localization().chat_map_server_refused_login(), &[(
                        "error",
                        &error_code.to_string(),
                    )]);
                }
                NetworkEvent::SpiritOrbs { entity_id, amount } => {
                    self.client_state.follow_mut(client_state().spirit_orbs()).set(entity_id, amount);
                }
                NetworkEvent::SkillCooldown { skill_id, until } => {
                    self.client_state
                        .follow_mut(client_state().skill_cooldowns())
                        .start(skill_id, until, client_tick);
                }
                NetworkEvent::AttackRangeChanged { attack_range } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.attack_range = attack_range;
                    }
                }
                NetworkEvent::EntityStopped { entity_id, position } => {
                    let Some(map) = &self.map else {
                        continue;
                    };

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.stop_movement();
                        entity.set_position(map, position, client_tick);
                    }
                }
                NetworkEvent::EntityHighJumped { entity_id, position } => {
                    // Taekwon Leap (`TK_HIGHJUMP`, ZC_HIGHJUMP 0x01FF): the
                    // server has already moved the character; apply its
                    // position immediately so the client does not keep the old
                    // cell and then snap on the following stop-move packet.
                    let Some(map) = &self.map else {
                        continue;
                    };

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.stop_movement();
                        entity.set_position(map, position, client_tick);

                        if entity_id == EntityId(self.saved_login_data.map_or(0, |data| data.account_id.0)) {
                            self.player_camera.set_focus_point(entity.get_position());
                        }
                    }
                }
                NetworkEvent::EntitySnapped { entity_id, position } => {
                    // Body Relocation / Snap (ZC_SNAP 0x08D2): apply the
                    // server's new position immediately so the client does not
                    // keep rendering the character at the old cell.
                    let Some(map) = &self.map else {
                        continue;
                    };

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.stop_movement();
                        entity.set_position(map, position, client_tick);

                        if entity_id == EntityId(self.saved_login_data.map_or(0, |data| data.account_id.0)) {
                            self.player_camera.set_focus_point(entity.get_position());
                        }
                    }
                }
                NetworkEvent::TradeItemOffered { item_id, amount } => {
                    let trade = self.client_state.follow_mut(client_state().trade());
                    trade.set_partner_offer(if item_id == 0 {
                        format!("{amount} zeny")
                    } else {
                        format!("item #{item_id} x{amount}")
                    });
                }
                NetworkEvent::TradeAddResult { inventory_index, result } => {
                    if result != 0 {
                        crate::logging::log_debug!(
                            "trade: adding inventory index {} failed with result {}",
                            inventory_index,
                            result
                        );
                    }
                }
                NetworkEvent::SpecialEffect { effect_id, .. } => {
                    // `ZC_NOTIFY_EFFECT2` (0x01F3) is the map weather packet
                    // (snow/fog/cloud/sakura/...). Indoor maps use the indoor
                    // map type from `ZC_NOTIFY_MAPPROPERTY2`; outdoor weather
                    // effects are not drawn there.
                    let map_type = *self.client_state.follow(client_state().map_type());
                    if crate::state::is_indoor_map_type(map_type) {
                        crate::logging::log_debug!("ignoring outdoor weather effect {effect_id} on an indoor map");
                        if let Some(weather) = self.weather.as_mut() {
                            weather.clear();
                        }
                    } else if let Some(weather) = self.weather.as_mut() {
                        weather.set_effect(effect_id as i32, 0.0);
                    }
                }
                NetworkEvent::TradeRequest {
                    requester_name,
                    target_account_id,
                    ..
                } => {
                    self.client_state
                        .follow_mut(client_state().trade())
                        .set_request(requester_name.clone(), target_account_id.0);
                    self.interface
                        .open_window(TradeRequestWindow::new(requester_name, target_account_id.0));
                }
                NetworkEvent::TradeAck { result, .. } => {
                    if result == 3 {
                        let partner_name = self
                            .client_state
                            .follow(client_state().trade())
                            .request()
                            .map(|request| request.name.clone())
                            .unwrap_or_default();
                        self.client_state.follow_mut(client_state().trade()).open(partner_name);
                        self.interface.close_window_with_class(WindowClass::TradeRequest);
                        self.interface
                            .open_window(TradeWindow::new(client_state().trade(), client_state().inventory().items()));
                    } else {
                        self.client_state.follow_mut(client_state().trade()).clear_request();
                        self.interface.close_window_with_class(WindowClass::TradeRequest);
                    }
                }
                NetworkEvent::TradeCancelled => {
                    self.close_trade();
                }
                NetworkEvent::TradeConcluded { who } => {
                    let trade = self.client_state.follow_mut(client_state().trade());
                    if who == 0 {
                        trade.set_locked(true);
                    } else {
                        trade.set_partner_locked(true);
                    }
                }
                NetworkEvent::TradeExecuted { result } => {
                    crate::logging::log_debug!("trade executed: result {}", result);
                    self.close_trade();
                }
                NetworkEvent::QuestList { quests } => {
                    self.client_state.follow_mut(client_state().quest_log()).set_list(&quests);
                }
                NetworkEvent::QuestActive { quest_id, active } => {
                    self.client_state
                        .follow_mut(client_state().quest_log())
                        .set_active(quest_id, active);
                }
                NetworkEvent::QuestRemoved { quest_id } => {
                    self.client_state.follow_mut(client_state().quest_log()).remove(quest_id);
                }
                NetworkEvent::AchievementList {
                    achievements,
                    total_score,
                    level,
                    experience,
                    experience_to_next_level,
                } => {
                    self.client_state.follow_mut(client_state().achievement()).set_list(
                        &achievements,
                        total_score,
                        level,
                        experience,
                        experience_to_next_level,
                    );
                }
                NetworkEvent::AchievementUpdate {
                    achievement,
                    total_score,
                    level,
                    experience,
                    experience_to_next_level,
                } => {
                    self.client_state.follow_mut(client_state().achievement()).update(
                        &achievement,
                        total_score,
                        level,
                        experience,
                        experience_to_next_level,
                    );
                }
                NetworkEvent::AchievementRewardResult { achievement_id, success } => {
                    self.client_state
                        .follow_mut(client_state().achievement())
                        .set_reward_result(achievement_id, success);
                    let message = match success {
                        true => format!("Achievement {achievement_id}: reward granted."),
                        false => format!("Achievement {achievement_id}: reward not available."),
                    };
                    self.push_chat_information(message);
                }
                NetworkEvent::MonsterInfo { information } => {
                    let sense = crate::state::monster_database::SenseInfo {
                        job_id: information.job_id,
                        level: information.level,
                        size: information.size,
                        health_points: information.health_points,
                        defense: information.defense,
                        race: information.race,
                        magic_defense: information.magic_defense,
                        element: information.element,
                        resistances: [
                            information.water,
                            information.earth,
                            information.fire,
                            information.wind,
                            information.poison,
                            information.holy,
                            information.shadow,
                            information.ghost,
                            information.undead,
                        ],
                    };

                    self.client_state.follow_mut(client_state().monster_database()).set_sense(sense);
                    self.push_chat_information(format!(
                        "Sense: monster #{} level {} HP {}",
                        information.job_id.0, information.level, information.health_points
                    ));

                    if !self.interface.is_window_with_class_open(WindowClass::MonsterDatabase) {
                        self.interface
                            .open_window(MonsterDatabaseWindow::new(self.library.monster_database()));
                    }
                }
                NetworkEvent::OpenStore { slots } => {
                    // `ZC_OPENSTORE` after MC_VENDING: show the shop editor.
                    // The server keeps the character in the vending state until
                    // it receives CZ_REQ_OPENSTORE/2, so the editor is modal
                    // (no X close) and always offers Open/Cancel.
                    self.client_state.follow_mut(client_state().vending()).open(slots);
                    self.interface.open_window(crate::interface::windows::VendingWindow::new(
                        client_state().vending(),
                        client_state().inventory().items(),
                    ));
                }
                NetworkEvent::MinimapMarker {
                    npc_id,
                    marker_type,
                    position,
                    id: _,
                    color,
                } => {
                    let minimap = self.client_state.follow_mut(client_state().minimap());

                    match marker_type {
                        MarkerType::RemoveMark => minimap.remove_marker(npc_id),
                        MarkerType::DisplayFor15Seconds | MarkerType::DisplayUntilLeave => {
                            let expires_at = match marker_type {
                                MarkerType::DisplayFor15Seconds => Some(client_tick.0.wrapping_add(15_000)),
                                MarkerType::DisplayUntilLeave => None,
                                MarkerType::RemoveMark => unreachable!(),
                            };

                            minimap.add_marker(crate::state::minimap::MinimapMarker {
                                entity_id: npc_id,
                                position: TilePosition {
                                    x: position.x.min(u16::MAX as u32) as u16,
                                    y: position.y.min(u16::MAX as u32) as u16,
                                },
                                color: [color.red, color.green, color.blue, color.alpha],
                                expires_at,
                            });
                        }
                    }
                }
                NetworkEvent::MapProperties { map_type, flags } => {
                    *self.client_state.follow_mut(client_state().map_type()) = map_type;
                    *self.client_state.follow_mut(client_state().map_flags()) = flags;
                }
                NetworkEvent::CartInfo {
                    current_count,
                    maximum_count,
                    current_weight,
                    maximum_weight,
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.cart_items = current_count.max(0) as u16;
                        player.cart_maximum_items = maximum_count;
                        player.cart_weight = current_weight.max(0) as u32;
                        player.cart_maximum_weight = maximum_weight.max(0) as u32;
                    }
                }
                NetworkEvent::MapServerAuthFailed { reason } => {
                    crate::logging::log_warn!("map server rejected the login (ZC_AUTH_FAIL reason {reason})");
                    let message = self.render_localized(client_state().localization().error_map_server_auth_failed(), &[(
                        "reason",
                        &reason.to_string(),
                    )]);
                    self.interface.open_window(ErrorWindow::new(message));
                }
                NetworkEvent::ShowEquipmentWindow { open } => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match (open, self.interface.is_window_with_class_open(WindowClass::Equipment)) {
                            (true, false) => self.interface.open_window(EquipmentWindow::new(
                                client_state().inventory().items(),
                                this_player().manually_asserted(),
                            )),
                            (false, true) => self.interface.close_window_with_class(WindowClass::Equipment),
                            _ => {}
                        }
                    }
                }
                NetworkEvent::CriticalWeight { percentage } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.critical_weight_percentage = percentage;
                    }
                }
                NetworkEvent::MapCellChanged {
                    position,
                    cell_type,
                    map_name,
                } => {
                    // A delayed update for the map the player just left must not
                    // patch collision on the new map.
                    if let Some(current_map_name) = self.current_map_name.as_deref()
                        && !current_map_name.eq_ignore_ascii_case(&map_name)
                    {
                        crate::logging::log_debug!("ignoring map cell update for `{map_name}` while on `{current_map_name}`");
                        continue;
                    }

                    // rAthena cell types: 0 walkable, 1 blocked, 2 water,
                    // 3 water+walkable, 4 water+snipeable, 5 cliff+snipeable,
                    // 6 cliff. Only 0 and 3 are walkable. The loaded map is an
                    // immutable Arc<Map>, so the override is recorded in the
                    // state and applied by `TraversableMap` in the path finder.
                    let walkable = matches!(cell_type, 0 | 3);
                    let overrides = self.client_state.follow_mut(client_state().map_cell_overrides());

                    match overrides.iter_mut().find(|(tile, _)| *tile == position) {
                        Some((_, current)) => *current = walkable,
                        None => overrides.push((position, walkable)),
                    }

                    crate::logging::log_debug!(
                        "map cell changed: {} ({}, {}) -> type {}",
                        map_name,
                        position.x,
                        position.y,
                        cell_type
                    );
                }
                NetworkEvent::MessageString { message_id } => {
                    let text = match self.message_table.as_ref().and_then(|table| table.get(message_id)) {
                        Some(text) => text.to_owned(),
                        None => format!("Message #{message_id}"),
                    };

                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, MessageColor::Rgb {
                            red: 240,
                            green: 220,
                            blue: 120,
                        }));
                }
            }
        }

        // Apply stat updates that were received before the local player entity
        // existed (e.g. the initial stat burst with the experience needed for the
        // next level). This also re-applies buffered updates after a map change or
        // respawn.
        self.apply_pending_stat_updates();

        self.chat_bubbles.retain_alive(client_tick);
        self.skill_name_texts.retain_alive(client_tick);
    }
}
