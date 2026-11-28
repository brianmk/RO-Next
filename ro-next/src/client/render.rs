//! Per-frame world overlay rendering (names, lights, shadows, spirit spheres)
//! and the borrow bundle it needs.

use crate::*;

/// Bundles all the borrows needed by the per-frame rendering pipeline so that
/// they can be passed as a single `self` argument.
pub(crate) struct MapRenderContext<'a, 'm: 'a> {
    pub(crate) map: &'m Map,
    pub(crate) current_camera: &'a (dyn Camera + Send + Sync),
    pub(crate) point_light_set: &'a PointLightSet<'a>,
    pub(crate) client_state: &'a State<ClientState>,
    pub(crate) library: &'a Library,
    pub(crate) guild_emblems: &'a GuildEmblemCache,
    pub(crate) mouse_position: ScreenPosition,
    pub(crate) mouse_target: PickerTarget,
    pub(crate) screen_size: ScreenSize,
    pub(crate) scaling: Scaling,
    pub(crate) client_tick: ClientTick,
    pub(crate) animation_timer_ms: f32,
    pub(crate) currently_playing: bool,
    pub(crate) is_mouse_mode_default: bool,
    pub(crate) is_interface_hovered: bool,
    pub(crate) last_walking_destination: Option<TilePosition>,
    pub(crate) is_targeting_ground_skill: bool,
    pub(crate) skill_target_level: Option<SkillLevel>,
    pub(crate) buffered_action: Option<BufferedAction>,
    #[cfg(feature = "debug")]
    pub(crate) render_options: &'a RenderOptions,
    #[cfg(feature = "debug")]
    pub(crate) hovered_marker_identifier: Option<MarkerIdentifier>,
    #[cfg(feature = "debug")]
    pub(crate) pathing_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    pub(crate) tile_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    pub(crate) player_camera: &'a PlayerCamera,
    #[cfg(feature = "debug")]
    pub(crate) start_camera: &'a StartCamera,

    // Mutable rendering state
    pub(crate) model_batches: &'a mut Vec<ModelBatch>,
    pub(crate) model_instructions: &'a mut Vec<ModelInstruction>,
    pub(crate) entity_instructions: &'a mut Vec<EntityInstruction>,
    pub(crate) directional_shadow_camera: &'a mut DirectionalShadowCamera,
    pub(crate) directional_shadow_model_batches: &'a mut [Vec<ModelBatch>; PARTITION_COUNT],
    pub(crate) directional_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    pub(crate) directional_shadow_entity_instructions: &'a mut [Vec<EntityInstruction>; PARTITION_COUNT],
    pub(crate) point_shadow_camera: &'a mut PointShadowCamera,
    pub(crate) point_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    pub(crate) point_shadow_entity_instructions: &'a mut Vec<EntityInstruction>,
    pub(crate) point_light_with_shadow_instructions: &'a mut Vec<PointLightWithShadowInstruction>,
    pub(crate) point_light_instructions: &'a mut Vec<PointLightInstruction>,
    pub(crate) directional_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    pub(crate) point_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    pub(crate) deferred_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    pub(crate) indicator_instruction: &'a mut Option<IndicatorInstruction>,
    pub(crate) water_instruction: &'a mut Option<WaterInstruction<'m>>,
    pub(crate) particle_holder: &'a mut ParticleHolder,
    pub(crate) effect_holder: &'a mut EffectHolder,
    pub(crate) chat_bubbles: &'a ChatBubbles,
    pub(crate) head_marks: &'a HeadMarks,
    pub(crate) cast_bars: &'a CastBars,
    pub(crate) skill_name_texts: &'a SkillNameTexts,
    pub(crate) environment: &'a crate::world::Environment,
    pub(crate) spirit_sphere_texture: Option<&'a Arc<Texture>>,
    pub(crate) effect_renderer: &'a mut EffectRenderer,
    pub(crate) bottom_interface_renderer: &'a GameInterfaceRenderer,
    pub(crate) middle_interface_renderer: &'a mut GameInterfaceRenderer,
    #[cfg(feature = "debug")]
    pub(crate) aabb_instructions: &'a mut Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    pub(crate) circle_instructions: &'a mut Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    pub(crate) rectangle_instructions: &'a mut Vec<DebugRectangleInstruction>,
    #[cfg(feature = "debug")]
    pub(crate) bounding_box_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    pub(crate) debug_marker_renderer: &'a mut DebugMarkerRenderer,
}

impl<'a, 'm: 'a> MapRenderContext<'a, 'm> {
    #[inline(always)]
    #[cfg(feature = "debug")]
    #[ro_next_debug::profile]
    pub(crate) fn render_markers(&mut self) {
        let entities = self.client_state.follow(client_state().entities());

        self.map.render_markers(
            self.debug_marker_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );

        self.map.render_markers(
            self.middle_interface_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn render_directional_shadows(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        for partition_index in 0..PARTITION_COUNT {
            let partition_camera = self.directional_shadow_camera.get_partition_camera(partition_index);

            let object_set = self.map.cull_objects_with_frustum(
                &partition_camera,
                self.directional_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                self.render_options.frustum_culling,
            );

            let offset = self.directional_shadow_model_instructions.len();
            let model_batches = &mut self.directional_shadow_model_batches[partition_index];
            let entity_instructions = &mut self.directional_shadow_entity_instructions[partition_index];

            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_objects))]
            self.map.render_objects(
                self.directional_shadow_model_instructions,
                &object_set,
                self.animation_timer_ms,
                &partition_camera,
            );

            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_map))]
            self.map.render_ground(self.directional_shadow_model_instructions);

            let count = self.directional_shadow_model_instructions.len() - offset;

            model_batches.push(ModelBatch {
                offset,
                count,
                texture_set: self.map.get_texture_set().clone(),
                vertex_buffer: self.map.get_model_vertex_buffer().clone(),
                index_buffer: self.map.get_model_index_buffer().clone(),
            });

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_map_tiles))]
            self.map
                .render_overlay_tiles(self.directional_shadow_model_instructions, model_batches, self.tile_texture_set);

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_pathing))]
            self.map.render_entity_pathing(
                self.directional_shadow_model_instructions,
                model_batches,
                entities,
                self.pathing_texture_set,
            );

            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_ground_items))]
            self.map
                .render_ground_items(entity_instructions, ground_items, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_entities(entity_instructions, entities, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_dead_entities(entity_instructions, dead_entities, &partition_camera, self.client_tick);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn render_point_lights(&mut self) {
        let point_shadow_entities = PointShadowEntities {
            entities: self.client_state.follow(client_state().entities()),
            dead_entities: self.client_state.follow(client_state().dead_entities()),
            ground_items: self.client_state.follow(client_state().ground_items()),
            client_tick: self.client_tick,
        };

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights(self.point_light_instructions);

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights_with_shadows(
            self.map,
            self.point_shadow_camera,
            self.point_shadow_object_set_buffer,
            self.point_shadow_model_instructions,
            self.point_shadow_entity_instructions,
            self.point_light_with_shadow_instructions,
            &point_shadow_entities,
            self.animation_timer_ms,
            #[cfg(feature = "debug")]
            self.render_options,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn render_geometry(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        let offset = self.model_instructions.len();
        let object_set = self.map.cull_objects_with_frustum(
            self.current_camera,
            self.deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            self.render_options.frustum_culling,
        );

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_objects))]
        self.map.render_objects(
            self.model_instructions,
            &object_set,
            self.animation_timer_ms,
            self.current_camera,
        );

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_map))]
        self.map.render_ground(self.model_instructions);

        self.model_batches.push(ModelBatch {
            offset,
            count: self.model_instructions.len() - offset,
            texture_set: self.map.get_texture_set().clone(),
            vertex_buffer: self.map.get_model_vertex_buffer().clone(),
            index_buffer: self.map.get_model_index_buffer().clone(),
        });

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_map_tiles))]
        self.map
            .render_overlay_tiles(self.model_instructions, self.model_batches, self.tile_texture_set);

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_pathing))]
        self.map
            .render_entity_pathing(self.model_instructions, self.model_batches, entities, self.pathing_texture_set);

        let entity_camera: &dyn Camera = match true {
            #[cfg(feature = "debug")]
            _ if self.render_options.show_entities_paper => self.player_camera,
            _ => self.current_camera,
        };

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_ground_items))]
        self.map
            .render_ground_items(self.entity_instructions, ground_items, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_entities(self.entity_instructions, entities, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_dead_entities(self.entity_instructions, dead_entities, entity_camera, self.client_tick);

        #[cfg(feature = "debug")]
        if self.render_options.show_entities_debug {
            self.map.render_entities_debug(self.rectangle_instructions, entities, entity_camera);
            self.map
                .render_entities_debug(self.rectangle_instructions, dead_entities, entity_camera);
        }

        #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_water))]
        self.map.render_water(self.water_instruction, self.animation_timer_ms);
    }

    #[inline(always)]
    #[cfg(feature = "debug")]
    #[ro_next_debug::profile]
    pub(crate) fn render_bounding_boxes(&mut self) {
        if self.render_options.show_bounding_boxes {
            let culling_camera: &dyn Camera = match self.currently_playing {
                true => self.player_camera,
                false => self.start_camera,
            };

            let object_set = self.map.cull_objects_with_frustum(
                culling_camera,
                self.bounding_box_object_set_buffer,
                self.render_options.frustum_culling,
            );

            self.map
                .render_bounding(self.aabb_instructions, self.render_options.frustum_culling, &object_set);
        }
    }

    /// Renders the name of an entity centered below its feet, like the original
    /// client does.
    ///
    /// Players that are in a guild additionally get a second, smaller line with
    /// their guild name (`<GuildName>`) below the name.
    pub(crate) fn render_entity_name(&self, entity: &Entity) {
        let Some(name) = entity.get_details() else {
            return;
        };

        let name = name.split('#').next().unwrap();
        let clip_space_position = self.current_camera.view_projection_matrix() * entity.get_position().to_homogeneous();
        let screen_position = self.current_camera.clip_to_screen_space(clip_space_position);

        // Skip names of entities that are off screen.
        if !(0.0..=1.0).contains(&screen_position.x) || !(0.0..=1.0).contains(&screen_position.y) {
            return;
        }

        // Entity positions are at the feet. The status/health bars are drawn a few
        // pixels below that, so the name has to start below the bar instead of at the
        // same height, otherwise both overlap.
        let name_below_feet = 20.0 * self.scaling.get_factor();

        let position = ScreenPosition {
            left: screen_position.x * self.screen_size.width,
            top: screen_position.y * self.screen_size.height + name_below_feet,
        };

        let color = entity_name_color(entity.get_entity_type());

        let name_font_size = FontSize(NAME_FONT_SIZE);
        self.middle_interface_renderer
            .render_outlined_text(name, position, color, Color::BLACK, name_font_size, AlignHorizontal::Center);

        // The label is cached on the entity (`<guild name>`), so this path does
        // not allocate a String per entity per frame.
        let Some(guild_label) = entity.get_guild_label() else {
            return;
        };

        // The guild line is stacked directly below the name line. Measuring the name
        // text keeps the offset correct for every UI scaling factor and font.
        let name_size = self.middle_interface_renderer.measure_text(name, name_font_size);
        let guild_name_position = ScreenPosition {
            left: position.left,
            top: position.top + name_size.height + GUILD_NAME_GAP * self.scaling.get_factor(),
        };

        self.middle_interface_renderer.render_outlined_text(
            guild_label,
            guild_name_position,
            GUILD_NAME_COLOR,
            Color::BLACK,
            FontSize(GUILD_NAME_FONT_SIZE),
            AlignHorizontal::Center,
        );

        // The classic client draws the guild emblem to the left of the guild name.
        // The emblem is requested when the guild name becomes known; until the map
        // server answered, a neutral placeholder is drawn instead of nothing/black.
        let guild_id = entity.get_guild_id();

        if guild_id != 0 {
            let guild_label_size = self
                .middle_interface_renderer
                .measure_text(guild_label, FontSize(GUILD_NAME_FONT_SIZE));
            let emblem_extent = GUILD_EMBLEM_ICON_SIZE * self.scaling.get_factor();
            let emblem_gap = 2.0 * self.scaling.get_factor();
            let emblem_position = ScreenPosition {
                left: guild_name_position.left - guild_label_size.width / 2.0 - emblem_gap - emblem_extent,
                top: guild_name_position.top + (guild_label_size.height - emblem_extent) / 2.0,
            };
            let emblem_size = ScreenSize {
                width: emblem_extent,
                height: emblem_extent,
            };

            match self.guild_emblems.emblem(guild_id) {
                Some(emblem) => self.middle_interface_renderer.render_sprite(
                    emblem.texture.clone(),
                    emblem_position,
                    emblem_size,
                    ScreenClip::unbound(),
                    Color::WHITE,
                    false,
                ),
                None => self
                    .middle_interface_renderer
                    .render_rectangle(emblem_position, emblem_size, GUILD_EMBLEM_PLACEHOLDER_COLOR),
            }
        }
    }

    /// Draws the classic blue-white spirit spheres around entities with an
    /// active sphere count (Monk/Champion). The data comes from
    /// `ZC_SPIRITS` (see `state::spirit_orbs`); the same texture and layered
    /// additive quads as roBrowserLegacy's `SpiritSphere` renderer are used.
    pub(crate) fn render_spirit_spheres(&mut self) {
        let Some(texture) = self.spirit_sphere_texture.cloned() else {
            return;
        };

        let entities = self.client_state.follow(client_state().entities());
        let orbs = self.client_state.follow(client_state().spirit_orbs());
        let time = self.animation_timer_ms / 1000.0;

        for entry in orbs.entries() {
            if entry.amount <= 0 {
                continue;
            }
            let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entry.entity_id) else {
                continue;
            };
            let position = entity.get_position() + Vector3::new(0.0, 0.0, 0.0);
            crate::world::spirit_sphere_renderer::render(
                self.effect_renderer,
                self.current_camera,
                texture.clone(),
                position,
                self.screen_size,
                entry.amount,
                time,
            );
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    pub(crate) fn render_world_overlays(&mut self) {
        #[cfg(feature = "debug")]
        if let Some(marker_identifier) = self.hovered_marker_identifier {
            self.map.render_marker_overlay(
                self.aabb_instructions,
                self.circle_instructions,
                self.current_camera,
                marker_identifier,
                self.point_light_set,
                self.animation_timer_ms,
            );
        }

        self.particle_holder.render(
            self.bottom_interface_renderer,
            self.current_camera,
            self.screen_size,
            self.scaling,
            self.client_state.follow(client_state().entities()),
        );

        self.effect_holder.render(self.effect_renderer, self.current_camera);
        self.environment
            .render_weather(self.effect_renderer, self.current_camera, self.screen_size);
        self.render_spirit_spheres();

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            #[cfg(feature = "debug")]
            profile_block!("render player status");

            player.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().world_theme()),
                self.screen_size,
            );
        }

        if let Some(BufferedAction::AttackEntity { entity_id }) = self.buffered_action
            && let Some(entity) = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().world_theme()),
                self.screen_size,
            );
        }

        // Player names are shown below the characters at all times, like in the
        // original client. The interface setting can restrict this to the hovered
        // entity only.
        let show_character_names = *self.client_state.follow(client_state().interface_settings().show_character_names());

        if show_character_names && self.currently_playing {
            for entity in self.client_state.follow(client_state().entities()) {
                // Players and NPCs have permanent name labels in the original client.
                // Monsters and other entities only show their name while hovered.
                if matches!(
                    entity.get_entity_type(),
                    EntityType::Player | EntityType::Npc | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary
                ) {
                    self.render_entity_name(entity);
                }
            }
        }

        // Status icons above entity heads (`opt1`/`opt2`). Drawn after the name
        // labels and below the chat bubbles, so a speaking frozen character
        // still shows the freeze icon.
        if self.currently_playing {
            self.head_marks.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
            );
        }

        // Overhead chat bubbles ("subtitles") above speaking player characters.
        // They are drawn after the name labels so they don't get overlapped by
        // them, and they have their own per-message lifetime.
        let show_chat_bubbles = *self.client_state.follow(client_state().interface_settings().show_chat_bubbles());

        if show_chat_bubbles && chat_bubbles_enabled_by_environment() && self.currently_playing {
            let show_background = *self
                .client_state
                .follow(client_state().interface_settings().chat_bubble_background())
                && chat_bubble_background_enabled_by_environment();

            self.chat_bubbles.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
                show_background,
                self.client_tick,
            );
        }

        // Short-lived skill name labels above the caster's head ("Blessing !!").
        // They are drawn on top of the chat bubbles so a cast during a chat line
        // stays readable.
        let show_skill_names = *self
            .client_state
            .follow(client_state().interface_settings().show_skill_names_above_characters());

        if show_skill_names && skill_names_enabled_by_environment() && self.currently_playing {
            crate::logging::log_trace!(
                "skill name texts: rendering {} label(s) (setting={})",
                self.skill_name_texts.len(),
                show_skill_names
            );

            self.skill_name_texts.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
                self.client_tick,
            );
        }

        // Cast progress bar above the local player's head while a cast with a cast
        // time is running (driven by server-confirmed cast starts).
        if self.currently_playing {
            // The progress fill uses the theme's green success token, so the
            // cast bar is green in the default and in the classic theme.
            let cast_bar_fill_color = *self.client_state.follow(client_theme().tokens().color().state_success());

            self.cast_bars.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
                self.client_tick,
                cast_bar_fill_color,
            );
        }

        match self.mouse_target {
            PickerTarget::Tile { x, y } => {
                // Only show if the mouse mode is default, walking, or
                // selecting the target of a ground skill.
                if self.currently_playing
                    && !self.is_interface_hovered
                    && (self.is_mouse_mode_default || self.last_walking_destination.is_some() || self.is_targeting_ground_skill)
                {
                    let walk_indicator_color = *self.client_state.follow(client_state().world_theme().indicator().walking());

                    #[cfg_attr(feature = "debug", ro_next_debug::debug_condition(self.render_options.show_indicators))]
                    self.map
                        .render_walk_indicator(self.indicator_instruction, walk_indicator_color, TilePosition { x, y });
                }
            }
            PickerTarget::Entity(entity_id) => {
                if !self.is_interface_hovered && self.is_mouse_mode_default {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        // Since the buffered attack entity will render its status anyway,
                        // we make sure not to render it here again if it's the same.
                        if !self
                            .buffered_action
                            .is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id))
                        {
                            entity.render_status(
                                self.middle_interface_renderer,
                                self.current_camera,
                                self.client_state.follow(client_state().world_theme()),
                                self.screen_size,
                            );
                        }

                        // Render the name below the character instead of at the
                        // mouse cursor. If names are always shown, player names are
                        // already rendered above, so avoid drawing them twice.
                        let has_permanent_label = matches!(
                            entity.get_entity_type(),
                            EntityType::Player | EntityType::Npc | EntityType::Pet | EntityType::Homunculus | EntityType::Mercenary
                        );

                        if !has_permanent_label || !show_character_names {
                            self.render_entity_name(entity);
                        }
                    } else if let Some(item) = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        let name = self.library.item_name(item.item_id, item.is_identified);

                        // TODO: Don't allocate every frame
                        let text = format!("{name}: {}ea", item.quantity);
                        self.middle_interface_renderer
                            .render_hover_text(&text, self.scaling, self.mouse_position);
                    }
                }
            }
            _ => {}
        }

        // While selecting a skill target, show the level the skill will be
        // cast at next to the cursor, like in the original client.
        if let Some(level) = self.skill_target_level {
            // TODO: Don't allocate every frame
            let text = level.0.to_string();
            self.middle_interface_renderer
                .render_hover_text(&text, self.scaling, self.mouse_position);
        }
    }
}
