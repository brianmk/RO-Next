//! `InputEvent` handlers for the skills family.
//!
//! Each handler receives the whole event and destructures it; the
//! dispatcher in `user_events.rs` only routes.

use crate::*;

impl Client {
    /// `InputEvent::ToggleSkillTreeWindow`.
    pub(crate) fn handle_input_toggle_skill_tree_window(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::ToggleSkillTreeWindow = event else {
            unreachable!("handle_input_toggle_skill_tree_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::SkillTree) {
                true => self.interface.close_window_with_class(WindowClass::SkillTree),
                false => self.interface.open_window(SkillTreeWindow::new(
                    client_state().skill_tree_window(),
                    client_state().skill_tree().layout(),
                    client_state().skill_tree().skills(),
                    this_player().manually_asserted().skill_points(),
                )),
            }
        }
    }

    /// `InputEvent::MoveSkill`.
    pub(crate) fn handle_input_move_skill(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::MoveSkill {
            source,
            destination,
            skill,
        } = event
        else {
            unreachable!("handle_input_move_skill called with a different event variant");
        };

        match (source, destination) {
            (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                self.client_state
                    .follow_mut(client_state().hotbar())
                    .update_slot(&mut self.networking_system, slot, skill);
            }
            (SkillSource::Hotbar { slot }, SkillSource::SkillTree) => {
                self.client_state
                    .follow_mut(client_state().hotbar())
                    .clear_slot(&mut self.networking_system, slot);
            }
            (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                self.client_state
                    .follow_mut(client_state().hotbar())
                    .swap_slot(&mut self.networking_system, source_slot, destination_slot);
            }
            _ => {}
        }
    }

    /// `InputEvent::OpenSkillDescription`.
    pub(crate) fn handle_input_open_skill_description(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::OpenSkillDescription {
            name,
            learned_level,
            maximum_level,
            skill_type,
            spell_point_cost,
            attack_range,
            upgradable,
            position,
        } = event
        else {
            unreachable!("handle_input_open_skill_description called with a different event variant");
        };

        self.interface.close_window_with_class(WindowClass::SkillDescription);
        self.interface.open_window_at(
            SkillDescriptionWindow::new(
                name,
                learned_level,
                maximum_level,
                skill_type,
                spell_point_cost,
                attack_range,
                upgradable,
            ),
            position,
        );
    }

    /// `InputEvent::CycleHotbarTab`.
    pub(crate) fn handle_input_cycle_hotbar_tab(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CycleHotbarTab { delta } = event else {
            unreachable!("handle_input_cycle_hotbar_tab called with a different event variant");
        };

        self.client_state.follow_mut(client_state().hotbar()).cycle_tab(delta);

        let tab = self.client_state.follow(client_state().hotbar()).active_tab();
        self.push_localized_chat(client_state().localization().chat_hotbar_page(), &[
            ("page", &(tab + 1).to_string()),
            ("pages", &HOTBAR_TAB_COUNT.to_string()),
        ]);
    }

    /// `InputEvent::ToggleHotbarWindow`.
    pub(crate) fn handle_input_toggle_hotbar_window(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleHotbarWindow = event else {
            unreachable!("handle_input_toggle_hotbar_window called with a different event variant");
        };

        if self.client_state.try_follow(this_entity()).is_some() {
            match self.interface.is_window_with_class_open(WindowClass::Hotbar) {
                true => self.interface.close_window_with_class(WindowClass::Hotbar),
                false => self.open_hotbar_window(),
            }
        }
    }

    /// `InputEvent::SwitchHotbarBar`.
    pub(crate) fn handle_input_switch_hotbar_bar(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::SwitchHotbarBar = event else {
            unreachable!("handle_input_switch_hotbar_bar called with a different event variant");
        };

        self.client_state.follow_mut(client_state().hotbar()).switch_bar();
    }

    /// `InputEvent::RotateHotbarRows`.
    pub(crate) fn handle_input_rotate_hotbar_rows(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RotateHotbarRows { delta } = event else {
            unreachable!("handle_input_rotate_hotbar_rows called with a different event variant");
        };

        self.client_state.follow_mut(client_state().hotbar()).rotate_rows(delta);
    }

    /// `InputEvent::BeginRebind`.
    pub(crate) fn handle_input_begin_rebind(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::BeginRebind { action } = event else {
            unreachable!("handle_input_begin_rebind called with a different event variant");
        };

        self.client_state.follow_mut(client_state().key_rebind()).begin(action);
    }

    /// `InputEvent::CancelRebind`.
    pub(crate) fn handle_input_cancel_rebind(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CancelRebind = event else {
            unreachable!("handle_input_cancel_rebind called with a different event variant");
        };

        self.client_state.follow_mut(client_state().key_rebind()).cancel();
    }

    /// `InputEvent::RebindKey`.
    pub(crate) fn handle_input_rebind_key(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::RebindKey {
            action,
            key,
            alt,
            control,
            shift,
        } = event
        else {
            unreachable!("handle_input_rebind_key called with a different event variant");
        };

        crate::input::keybinds::store_override(action, key, alt, control, shift);
        crate::input::keybinds::save_user_bindings();
        self.client_state.follow_mut(client_state().key_rebind()).cancel();

        let keys = crate::input::keybinds::binding_label(action);
        self.push_localized_chat(client_state().localization().qol().chat_keybind_saved(), &[
            ("action", action.label()),
            ("keys", &keys),
        ]);
    }

    /// `InputEvent::ResetKeybinds`.
    pub(crate) fn handle_input_reset_keybinds(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ResetKeybinds = event else {
            unreachable!("handle_input_reset_keybinds called with a different event variant");
        };

        crate::input::keybinds::clear_overrides();
        crate::input::keybinds::save_user_bindings();
        self.client_state.follow_mut(client_state().key_rebind()).cancel();
        self.push_localized_chat(client_state().localization().qol().chat_keybinds_reset(), &[]);
    }

    /// `InputEvent::ToggleBattleMode`.
    pub(crate) fn handle_input_toggle_battle_mode(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::ToggleBattleMode { enabled } = event else {
            unreachable!("handle_input_toggle_battle_mode called with a different event variant");
        };

        let battle_mode_path = client_state().game_settings().battle_mode();
        let battle_mode = self.client_state.follow_mut(battle_mode_path);
        let enabled = enabled.unwrap_or(!*battle_mode);
        *battle_mode = enabled;

        self.client_state.follow_mut(client_state().game_settings()).save();
        self.interface.unfocus();

        if enabled {
            self.push_localized_chat(client_state().localization().qol().chat_battle_mode_on(), &[]);
        } else {
            self.push_localized_chat(client_state().localization().qol().chat_battle_mode_off(), &[]);
        }
    }

    /// `InputEvent::UseSkillInSlot`.
    pub(crate) fn handle_input_use_skill_in_slot(&mut self, event: InputEvent, _input_report: &InputReport, client_tick: ClientTick) {
        let InputEvent::UseSkillInSlot { slot } = event else {
            unreachable!("handle_input_use_skill_in_slot called with a different event variant");
        };

        // Item hotkeys use the first matching inventory item, like the
        // original client uses the first stack of that item.
        if let Some(item) = self.client_state.follow(client_state().hotbar()).item_in_slot(slot) {
            let inventory_index = self
                .client_state
                .follow(client_state().inventory().items())
                .iter()
                .find(|inventory_item| inventory_item.item_id == item.item_id)
                .map(|inventory_item| inventory_item.index);

            match inventory_index {
                Some(inventory_index) => {
                    let _ = self.networking_system.use_item(inventory_index, item.item_id);
                }
                None => self.push_localized_chat(client_state().localization().chat_hotkey_item_missing(), &[]),
            }

            return;
        }

        // Skills that require a target switch to a target selection
        // mouse mode, while self cast skills are used immediately,
        // like in the original client.
        let skill = self
            .client_state
            .follow(client_state().hotbar())
            .get_skill_in_slot(slot)
            .as_ref()
            .and_then(|learnable_skill| {
                self.client_state
                    .follow(client_state().skill_tree().skills())
                    .iter()
                    .find(|learned_skill| learned_skill.skill_id == learnable_skill.skill_id)
                    .map(|learned_skill| {
                        // The character may have the skill at a lower level than
                        // the hotbar slot was created for (the skill was
                        // re-learned or copied onto a character that cannot max
                        // it); in that case cast the level the character really
                        // has instead of silently doing nothing.
                        let level = SkillLevel(learned_skill.skill_level.0.min(learnable_skill.maximum_level.0));
                        (learned_skill.skill_id, learned_skill.skill_type, level)
                    })
            });

        match skill {
            Some((skill_id, skill_type, level)) => {
                self.use_skill(Some(slot), skill_id, skill_type, level, client_tick);
            }
            None => {
                if let Some(learnable_skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref() {
                    self.push_localized_chat(client_state().localization().chat_skill_not_learned(), &[(
                        "skill",
                        &learnable_skill.skill_id.0.to_string(),
                    )]);
                }
            }
        }
    }

    /// `InputEvent::UseSkill`.
    pub(crate) fn handle_input_use_skill(&mut self, event: InputEvent, _input_report: &InputReport, client_tick: ClientTick) {
        let InputEvent::UseSkill { skill_id, level } = event else {
            unreachable!("handle_input_use_skill called with a different event variant");
        };

        // Double clicking a skill in the skill tree uses it, like in
        // the original client. Unlearned and passive skills are
        // ignored.
        let skill = self
            .client_state
            .follow(client_state().skill_tree().skills())
            .iter()
            .find(|learned_skill| learned_skill.skill_id == skill_id)
            .map(|learned_skill| (learned_skill.skill_type, SkillLevel(level.0.min(learned_skill.skill_level.0))));

        match skill {
            Some((skill_type, level)) => self.use_skill(None, skill_id, skill_type, level, client_tick),
            None => self.push_localized_chat(client_state().localization().chat_skill_not_learned(), &[(
                "skill",
                &skill_id.0.to_string(),
            )]),
        }
    }

    /// `InputEvent::CastSkill`.
    pub(crate) fn handle_input_cast_skill(&mut self, event: InputEvent, input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::CastSkill {
            skill_id,
            skill_type,
            level,
        } = event
        else {
            unreachable!("handle_input_cast_skill called with a different event variant");
        };

        // The skill type comes from the skill the player selected (hotbar or
        // skill tree); the learned list is only used for the range, with a
        // fallback so a cast is never silently dropped just because the
        // learned entry is missing (e.g. high-job/trimmed skill trees).
        let attack_range = self
            .client_state
            .follow(client_state().skill_tree().skills())
            .iter()
            .find(|learned_skill| learned_skill.skill_id == skill_id)
            .map(|learned_skill| learned_skill.attack_range)
            .unwrap_or(AttackRange(1));

        if crate::logging::enabled(crate::logging::LEVEL_DEBUG) {
            crate::logging::log_debug!(
                "cast skill {:?} level {} type {:?} range {} target {:?}",
                skill_id,
                level.0,
                skill_type,
                attack_range.0,
                input_report.mouse_target
            );
        }

        {
            match skill_type {
                SkillType::Passive => {
                    self.push_localized_chat(client_state().localization().chat_skill_passive(), &[(
                        "skill",
                        &skill_id.0.to_string(),
                    )]);
                }
                SkillType::Attack => {
                    if let PickerTarget::Entity(entity_id) = input_report.mouse_target
                        && let Some(map) = &self.map
                    {
                        let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                        let target_position = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| entity.get_tile_position());

                        if let (Some(player_position), Some(target_position)) = (player_position, target_position) {
                            if player_position
                                .x
                                .abs_diff(target_position.x)
                                .max(player_position.y.abs_diff(target_position.y))
                                <= attack_range.0
                            {
                                let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                            } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                player_position,
                                target_position,
                                attack_range,
                            ) && let Some(nearest_tile) = path.last()
                            {
                                // The target is out of range, so walk towards it and
                                // send the cast right away. The server remembers skill
                                // requests made while walking and casts as soon as the
                                // target is in range, following the target's position
                                // at every step.
                                let _ = self.networking_system.player_move(WorldPosition {
                                    x: nearest_tile.x,
                                    y: nearest_tile.y,
                                    direction: Direction::North,
                                });
                                let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                            } else {
                                self.push_localized_chat(client_state().localization().chat_skill_target_out_of_range(), &[(
                                    "skill",
                                    &skill_id.0.to_string(),
                                )]);
                            }
                        } else {
                            self.push_localized_chat(client_state().localization().chat_skill_target_gone(), &[(
                                "skill",
                                &skill_id.0.to_string(),
                            )]);
                        }
                    } else {
                        crate::logging::log_debug!("cast skill {:?}: no entity target or map, dropping the attack cast", skill_id);
                        self.push_localized_chat(client_state().localization().chat_skill_no_target(), &[(
                            "skill",
                            &skill_id.0.to_string(),
                        )]);
                    }
                }
                SkillType::Ground | SkillType::Trap => {
                    if let PickerTarget::Tile { x, y } = input_report.mouse_target {
                        let target_position = TilePosition { x, y };

                        if let Some(map) = &self.map {
                            let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());

                            if let Some(player_position) = player_position {
                                if player_position
                                    .x
                                    .abs_diff(target_position.x)
                                    .max(player_position.y.abs_diff(target_position.y))
                                    <= attack_range.0
                                {
                                    let _ = self.networking_system.cast_ground_skill(skill_id, level, target_position);
                                } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                    &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                    player_position,
                                    target_position,
                                    attack_range,
                                ) && let Some(nearest_tile) = path.last()
                                {
                                    // The target is out of range, so walk towards it and
                                    // send the cast right away. The server remembers
                                    // skill requests made while walking and casts as
                                    // soon as the position is in range.
                                    let _ = self.networking_system.player_move(WorldPosition {
                                        x: nearest_tile.x,
                                        y: nearest_tile.y,
                                        direction: Direction::North,
                                    });
                                    let _ = self.networking_system.cast_ground_skill(skill_id, level, target_position);
                                } else {
                                    self.push_localized_chat(client_state().localization().chat_skill_cell_out_of_range(), &[(
                                        "skill",
                                        &skill_id.0.to_string(),
                                    )]);
                                }
                            } else {
                                self.push_localized_chat(client_state().localization().chat_skill_position_unknown(), &[(
                                    "skill",
                                    &skill_id.0.to_string(),
                                )]);
                            }
                        }
                    } else {
                        crate::logging::log_debug!(
                            "cast skill {:?}: ground skill without a tile target, dropping the cast",
                            skill_id
                        );
                        self.push_localized_chat(client_state().localization().chat_skill_click_ground(), &[(
                            "skill",
                            &skill_id.0.to_string(),
                        )]);
                    }
                }
                SkillType::SelfCast => self.cast_self_skill(skill_id, level),
                SkillType::Support => {
                    if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                        if let Some(map) = &self.map {
                            let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                            let target_position = self
                                .client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id)
                                .map(|entity| entity.get_tile_position());

                            if let (Some(player_position), Some(target_position)) = (player_position, target_position) {
                                if player_position
                                    .x
                                    .abs_diff(target_position.x)
                                    .max(player_position.y.abs_diff(target_position.y))
                                    <= attack_range.0
                                {
                                    let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                    &TraversableMap::new(map, self.client_state.follow(client_state().map_cell_overrides())),
                                    player_position,
                                    target_position,
                                    attack_range,
                                ) && let Some(nearest_tile) = path.last()
                                {
                                    // The target is out of range, so walk towards it and
                                    // send the cast right away. The server remembers skill
                                    // requests made while walking and casts as soon as the
                                    // target is in range, following the target's position
                                    // at every step.
                                    let _ = self.networking_system.player_move(WorldPosition {
                                        x: nearest_tile.x,
                                        y: nearest_tile.y,
                                        direction: Direction::North,
                                    });
                                    let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                } else {
                                    self.push_localized_chat(client_state().localization().chat_skill_target_out_of_range(), &[(
                                        "skill",
                                        &skill_id.0.to_string(),
                                    )]);
                                }
                            } else {
                                self.push_localized_chat(client_state().localization().chat_skill_target_gone(), &[(
                                    "skill",
                                    &skill_id.0.to_string(),
                                )]);
                            }
                        }
                    } else {
                        crate::logging::log_debug!(
                            "cast skill {:?}: support skill without an entity target, casting on self",
                            skill_id
                        );

                        let _ = self.networking_system.cast_skill(
                            skill_id,
                            level,
                            self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                        );
                    }
                }
            }
        }
    }

    /// `InputEvent::StopSkill`.
    pub(crate) fn handle_input_stop_skill(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::StopSkill { slot } = event else {
            unreachable!("handle_input_stop_skill called with a different event variant");
        };

        if let Some(skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
            && crate::world::skills::channels(skill.skill_id)
        {
            let _ = self.networking_system.stop_channeling_skill(skill.skill_id);
        }
    }

    /// `InputEvent::StatUp`.
    pub(crate) fn handle_input_stat_up(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::StatUp { stat_type } = event else {
            unreachable!("handle_input_stat_up called with a different event variant");
        };

        let _ = self.networking_system.request_stat_up(stat_type);
    }

    /// `InputEvent::DistributePointsForSkill`.
    pub(crate) fn handle_input_distribute_points_for_skill(
        &mut self,
        event: InputEvent,
        _input_report: &InputReport,
        _client_tick: ClientTick,
    ) {
        let InputEvent::DistributePointsForSkill { skill_id } = event else {
            unreachable!("handle_input_distribute_points_for_skill called with a different event variant");
        };

        if let Some(available_skill_points) = self.client_state.try_follow(this_player().skill_points()).copied() {
            let job_id = self.client_state.follow(this_entity().manually_asserted()).get_job_id();
            let pending_skill_points = self
                .client_state
                .follow(client_state().skill_tree_window().pending_skill_points())
                .len();
            let available_skill_points = (available_skill_points as usize).saturating_sub(pending_skill_points);

            let skill_information = self.library.get::<SkillListInformation>(skill_id);
            let learned_skills = self.client_state.follow(client_state().skill_tree().skills());

            let mut new_skill_points = self
                .client_state
                .follow(client_state().skill_tree_window().pending_skill_points())
                .clone();

            let current_skill_level = learned_skills
                .iter()
                .find(|skill| skill.skill_id == skill_id)
                .map(|skill| skill.skill_level.0)
                .unwrap_or_default()
                + new_skill_points
                    .iter()
                    .filter(|pending_skill_level| **pending_skill_level == skill_id)
                    .count() as u16;

            // If the skill is already at max level we don't do anything
            if current_skill_level < skill_information.maximum_level.0 {
                let target_skill_level = SkillLevel(current_skill_level + 1);

                bring_skill_to_level(
                    &mut new_skill_points,
                    &self.library,
                    learned_skills,
                    job_id,
                    skill_id,
                    target_skill_level,
                    available_skill_points,
                );

                *self
                    .client_state
                    .follow_mut(client_state().skill_tree_window().pending_skill_points()) = new_skill_points;
            }
        }
    }

    /// `InputEvent::LevelUpSkills`.
    pub(crate) fn handle_input_level_up_skills(&mut self, event: InputEvent, _input_report: &InputReport, _client_tick: ClientTick) {
        let InputEvent::LevelUpSkills { skill_ids } = event else {
            unreachable!("handle_input_level_up_skills called with a different event variant");
        };

        for skill_id in skill_ids {
            if self.networking_system.level_up_skill(skill_id).is_err() {
                break;
            }
        }
    }
}
