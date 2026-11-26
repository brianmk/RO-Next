//! Asset-loader bookkeeping: which resources are still loading, retrying
//! transient asset failures and the character-selection preview pipeline.

use crate::*;

impl Client {
    /// Drain finished async loads from the loader thread and apply their
    /// results to client state. This is the only place that promotes
    /// `self.map` from `None` to `Some` (when a map load completes), so it
    /// must run before the `self.map` check in [`Self::update_and_render`].
    ///
    /// Called as late as possible in the frame to give the loader thread the
    /// maximum window to finish in-flight work.
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn update_loaded_resources(&mut self, client_tick: ClientTick) {
        // Collect first so the borrowed loader can be released before the
        // (mutable) handling below.
        let completed_loads: Vec<LoadResult> = self.async_loader.take_completed().collect();

        for completed in completed_loads {
            let (id, resource) = match completed {
                LoadResult::Completed { id, resource } => (id, resource),
                LoadResult::Failed { id, error } => {
                    self.handle_failed_load(id, error);
                    continue;
                }
            };

            if matches!(
                id,
                LoaderId::AnimationData(_)
                    | LoaderId::FalconAnimationData(_)
                    | LoaderId::WeaponAnimationData(_)
                    | LoaderId::ShieldAnimationData(_)
                    | LoaderId::RobeAnimationData(_)
            ) {
                self.asset_retries.note_available(&id);
            }

            match (id, resource) {
                (LoaderId::AnimationData(entity_id), LoadableResource::AnimationData(animation_data)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().dead_entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.set_animation_data(animation_data);
                    }
                }
                (LoaderId::CharacterPreview(slot), LoadableResource::AnimationData(animation_data)) => {
                    self.client_state
                        .follow_mut(client_state().character_previews())
                        .set_ready(slot, animation_data);
                }
                (LoaderId::FalconAnimationData(entity_id), LoadableResource::FalconAnimation(falcon_animation)) => {
                    crate::logging::log_debug!("falcon animation: applying to entity {}", entity_id.0);
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }
                }
                (LoaderId::ImfData(entity_id, ..), LoadableResource::Imf(imf)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_imf(imf);
                    }
                }
                (LoaderId::CartAnimationData(entity_id), LoadableResource::CartAnimation(cart_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_cart_animation(cart_animation);
                    }
                }
                (LoaderId::WeaponAnimationData(entity_id), LoadableResource::WeaponAnimation(weapon_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_weapon_animation(weapon_animation);
                    }
                }
                (LoaderId::ShieldAnimationData(entity_id), LoadableResource::ShieldAnimation(shield_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_shield_animation(shield_animation);
                    }
                }
                (LoaderId::RobeAnimationData(entity_id), LoadableResource::RobeAnimation(robe_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_robe_animation(robe_animation);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture }) => {
                    self.client_state
                        .follow_mut(client_state().shop_items())
                        .iter_mut()
                        .filter(|item| item.item_id == item_id)
                        .for_each(|item| item.metadata.texture = Some(texture.clone()));

                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_item_sprite(item_id, texture.clone());

                    // Kafra storage items are loaded from a separate inventory
                    // state; without this update their icons stay blank when
                    // the item sprite finished loading while the storage window
                    // is open (the inventory may not contain the same item id).
                    self.client_state
                        .follow_mut(client_state().storage())
                        .update_item_sprite(item_id, texture);
                }
                (LoaderId::Map(map_name), LoadableResource::Map { map, position }) => {
                    match self.client_state.try_follow(this_entity()).is_none() {
                        true => {
                            // Load of main menu map
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine
                                .play_background_music_track(crate::client_config::background_music());
                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());

                            // The main menu map finished loading. Do not blindly open the
                            // character selection window here: the map load can finish after the
                            // player already logged in, which used to replace the freshly opened
                            // server selection window (or the login window) with an empty
                            // character selection screen. Show the window that matches the
                            // current login state instead.
                            let character_server_connected = self.networking_system.is_character_server_connected();
                            let login_server_connected = self.networking_system.is_login_server_connected();
                            let character_selection_open = self.interface.is_window_with_class_open(WindowClass::CharacterSelection);
                            let select_server_open = self.interface.is_window_with_class_open(WindowClass::SelectServer);
                            let login_open = self.interface.is_window_with_class_open(WindowClass::Login);

                            if character_server_connected && !character_selection_open {
                                self.open_character_selection_window();
                            } else if login_server_connected && !select_server_open && !character_selection_open {
                                self.interface
                                    .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                            } else if !character_server_connected
                                && !login_server_connected
                                && !login_open
                                && !select_server_open
                                && !character_selection_open
                            {
                                self.interface.open_window(LoginWindow::new(
                                    client_state().login_window(),
                                    client_state().login_settings(),
                                    client_state().client_info(),
                                ));
                            }

                            // The map entry may have been aborted because the map data
                            // was missing; tell the player why, now that the character
                            // selection window is back (opening the error last keeps it
                            // on top).
                            if let Some(message) = self.session.take_map_load_error() {
                                self.interface.open_window(ErrorWindow::new(message));
                            }

                            self.start_camera.set_focus_point(crate::client_config::start_camera_focus());
                        }
                        false => {
                            // Normal map switch: the map entry is complete.
                            self.session.map_assets_loaded();
                            self.current_map_name = Some(map_name.clone());
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(map.background_music_track_name());

                            if let Some(minimap) = map.get_minimap() {
                                self.client_state.follow_mut(client_state().minimap()).set_map(&map_name, minimap);
                            } else {
                                self.client_state.follow_mut(client_state().minimap()).clear();
                            }

                            if let Some(position) = position {
                                // `manually_asserted` is safe because we are in the branch where `this_player`
                                // is not `None`.
                                let player = self.client_state.follow_mut(this_entity().manually_asserted());

                                player.set_position(map, position, client_tick);
                                self.player_camera.set_focus_point(player.get_position());
                            }

                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                            self.on_navigation_map_loaded(&map_name);
                            let _ = self.networking_system.map_loaded();
                        }
                    }
                }
                (LoaderId::SkillSprite(skill_id), LoadableResource::SkillSprite { sprite }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar())
                        .update_skill_resource(skill_id, |skill| skill.sprite = Some(sprite.clone()));

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.sprite = Some(sprite);
                    }
                }
                (LoaderId::SkillActions(skill_id), LoadableResource::SkillActions { actions }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar())
                        .update_skill_resource(skill_id, |skill| skill.actions = Some(actions.clone()));

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.actions = Some(actions);
                    }
                }
                _ => {}
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    /// Reacts to an asynchronous load that failed.
    ///
    /// A failed map load is a dead end for the current map entry: the client
    /// has no map to render and would stay on a black screen forever. In
    /// that case the map connection is dropped, which sends the player back
    /// to the character selection screen (see
    /// `NetworkEvent::MapServerDisconnected`), and the error is shown
    /// there.
    /// Keep the character selection previews in sync with the character
    /// slots.
    ///
    /// Called every frame while the window is open. For every visible occupied
    /// slot whose preview is not loaded yet, the composed idle animation of the
    /// character is requested from the async loader; the result is applied by
    /// [`Self::update_loaded_resources`]. When the window is not open the
    /// loaded previews are dropped, so returning to the login screen does not
    /// keep character sprites alive.
    /// Repaint the character selection window while the anti-crash selection
    /// cooldown is running, so the countdown text updates and the cards become
    /// selectable again once the cooldown expired (the click handlers are only
    /// registered during a layout pass).
    pub(crate) fn update_character_selection_cooldown(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            return;
        }

        if self
            .client_state
            .follow(client_state().character_selection_cooldown())
            .is_repainting(Instant::now())
        {
            self.interface.invalidate_layouts();
        }
    }

    pub(crate) fn update_character_selection_previews(&mut self) {
        if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
            let previews = self.client_state.follow_mut(client_state().character_previews());
            if previews.slot_count() > 0 {
                previews.clear();
            }
            return;
        }

        let reported_slots = self.client_state.follow(client_state().character_slots()).get_slot_count();
        let slot_count = visible_character_slots(reported_slots);
        let previous_slot_count = self.client_state.follow(client_state().character_previews()).slot_count();

        self.client_state
            .follow_mut(client_state().character_previews())
            .set_slot_count(slot_count);

        // Log the clamp once per character list, not every frame the window is
        // open.
        if reported_slots > MAX_CHARACTER_SLOTS && previous_slot_count != slot_count {
            crate::logging::log_debug!(
                "character selection: server reports {reported_slots} slots, showing the first {MAX_CHARACTER_SLOTS}"
            );
        }

        for slot in 0..slot_count {
            // The character list can change while the window is open (delete,
            // slot switch); clone the row so the state borrow ends before the
            // loader is called.
            let character_information = self
                .client_state
                .follow(client_state().character_slots())
                .character_in_slot(slot)
                .cloned();

            let Some(character_information) = character_information else {
                self.client_state.follow_mut(client_state().character_previews()).clear_slot(slot);
                continue;
            };

            let character_id = character_information.character_id;
            let is_current = self
                .client_state
                .follow(client_state().character_previews())
                .is_current(slot, character_id);

            if is_current {
                continue;
            }

            let entity_part_files = character_preview_part_files(&self.library, &character_information);

            self.client_state
                .follow_mut(client_state().character_previews())
                .request(slot, character_id);

            // The appearance may already be cached (for example from a previous
            // visit of the character selection screen): in that case no
            // asynchronous load happens and the data can be applied right away.
            if let Some(animation_data) = self.async_loader.request_character_preview_load(slot, entity_part_files) {
                self.client_state
                    .follow_mut(client_state().character_previews())
                    .set_ready(slot, animation_data);
            }
        }
    }

    pub(crate) fn handle_failed_load(&mut self, id: LoaderId, error: LoadError) {
        // A missing preview sprite must not be requested again every frame: the
        // slot keeps its name/level and simply shows no character.
        if let LoaderId::CharacterPreview(slot) = &id {
            crate::logging::log_debug!("character preview unavailable for slot {slot}: {error:?}");
            self.client_state
                .follow_mut(client_state().character_previews())
                .set_unavailable(*slot);
            return;
        }

        // Missing weapon/shield/robe sprites are expected for custom/costume
        // gear (see `world::weapon`); the async loader caches the failure and
        // the entity is rendered without that layer. Keep this out of the
        // generic asset failure reporting (and out of stderr) so map entry does
        // not spam the user or the log for every population-engine shell.
        if let LoaderId::WeaponAnimationData(entity_id)
        | LoaderId::ShieldAnimationData(entity_id)
        | LoaderId::RobeAnimationData(entity_id) = &id
        {
            crate::logging::log_debug!("equipment animation unavailable for entity {}: {error:?}", entity_id.0);
            return;
        }

        match asset_failure_severity(&id) {
            AssetFailureSeverity::Fatal => {
                let LoaderId::Map(map_name) = id else {
                    // Severity and variant classification disagree; treat it as
                    // a transient failure below instead of panicking.
                    self.report_transient_asset_failure(id, error);
                    return;
                };

                // Only report the first failure of a map entry; retries after the
                // client returned to the character selection must not spam.
                if !self.session.has_map_load_error() {
                    crate::logging::log_warn!("map load failed: {map_name}: {error:?}");
                }

                let message = self.render_localized(client_state().localization().error_map_data_missing(), &[(
                    "map_name",
                    map_name.as_str(),
                )]);

                self.abort_map_entry(message);
            }
            AssetFailureSeverity::Transient => {
                if matches!(
                    id,
                    LoaderId::AnimationData(_)
                        | LoaderId::FalconAnimationData(_)
                        | LoaderId::WeaponAnimationData(_)
                        | LoaderId::ShieldAnimationData(_)
                        | LoaderId::RobeAnimationData(_)
                ) {
                    self.asset_retries.note_failed(&id, Instant::now(), &format!("{error:?}"));
                }

                self.report_transient_asset_failure(id, error);
            }
        }
    }

    /// Report a recoverable asset failure: always at warn level (visible in
    /// release) and, at most a few times per session, as a non-modal chat line.
    pub(crate) fn report_transient_asset_failure(&mut self, id: LoaderId, error: LoadError) {
        let key = asset_failure_log_key(&id);
        crate::logging::warn_once(
            module_path!(),
            &key,
            format_args!("resource load failed for {key}: {error:?} (a placeholder is used instead)"),
        );

        if self.transient_asset_notices.should_notify() {
            let message = transient_asset_failure_message(&id, &error);
            self.push_chat_warning(message);
        }
    }
}
