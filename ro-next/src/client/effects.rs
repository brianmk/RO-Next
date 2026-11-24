//! World-effect spawning helpers: skill effect phases and layers, status
//! auras, projectiles, cast circles and teleport vanishes.
//!
//! Extracted from `lib.rs` so the effect pipeline can be read on its own; the
//! packet and input dispatchers only call into it.

use crate::*;

impl Client {
    #[inline(always)]
    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    /// Spawns a world effect (`.str`) centered on an entity, with an attached
    /// point light.
    ///
    /// A malformed or missing effect file must not crash the client, so errors
    /// are only logged in debug builds.
    pub(crate) fn spawn_entity_effect(
        &mut self,
        effect_path: &str,
        entity_id: EntityId,
        effect_offset: Vector3<f32>,
        scale: f32,
        light_color: Color,
        light_intensity: f32,
    ) {
        let Ok(effect) = self.effect_loader.get_or_load(effect_path, &self.texture_loader) else {
            crate::logging::log_debug!("entity effect: failed to load '{}'", effect_path);
            return;
        };
        crate::logging::log_debug!("entity effect: spawned '{}' on entity {}", effect_path, entity_id.0);
        let frame_timer = effect.new_frame_timer();

        self.effect_holder.add_effect(Box::new(EffectWithLight::new(
            effect,
            frame_timer,
            EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
            effect_offset,
            scale,
            // The point light id needs to be unique. Skill effects use a
            // different id space than the visual effect packets so that both can
            // be active on the same entity at the same time.
            PointLightId::new(entity_id.0.wrapping_add(0x4000_0000)),
            Vector3::new(0.0, 12.0, 0.0),
            light_color,
            light_intensity,
            false,
        )));
    }

    /// Spawns a world effect (`.str`) centered on a world position.
    pub(crate) fn spawn_position_effect(
        &mut self,
        effect_path: &str,
        position: Point3<f32>,
        effect_offset: Vector3<f32>,
        scale: f32,
        light_color: Color,
        light_intensity: f32,
    ) {
        let Ok(effect) = self.effect_loader.get_or_load(effect_path, &self.texture_loader) else {
            crate::logging::log_debug!("position effect: failed to load '{}'", effect_path);
            return;
        };
        crate::logging::log_debug!("position effect: spawned '{}' at {:?} (scale {})", effect_path, position, scale);
        let frame_timer = effect.new_frame_timer();
        let light_id = NEXT_POSITION_EFFECT_LIGHT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        self.effect_holder.add_effect(Box::new(EffectWithLight::new(
            effect,
            frame_timer,
            EffectCenter::Position(position),
            effect_offset,
            scale,
            PointLightId::new(light_id),
            Vector3::new(0.0, 12.0, 0.0),
            light_color,
            light_intensity,
            false,
        )));
    }

    /// Loads the flying projectile sprites of `projectile` once, lazily.
    ///
    /// The candidate sprite paths (item sprites first, large skill-effect
    /// sprites as last resort) and the flight time come from the weapon data
    /// file, so no arrow/ammo path is hard-coded in the client loop.
    pub(crate) fn ensure_projectile_assets(&mut self, projectile: crate::world::weapon::ProjectileKind) {
        if self.projectile_assets.contains_key(&projectile) {
            return;
        }

        let Some(profile) = crate::world::weapon::projectile(projectile) else {
            crate::logging::log_debug!("projectile: no data profile for {:?}", projectile);
            self.projectile_assets.insert(projectile, None);
            return;
        };

        for candidate in profile.assets().iter().chain(profile.fallback_assets().iter()) {
            let sprite = self.sprite_loader.get_or_load(&format!("{candidate}.spr"), None);
            let actions = self.action_loader.get_or_load(&format!("{candidate}.act"));

            if let (Ok(sprite), Ok(actions)) = (sprite, actions) {
                crate::logging::log_debug!("projectile: loaded sprite {} ({:?})", candidate, projectile);
                self.projectile_assets
                    .insert(projectile, Some(Arc::new(ArrowAssets { sprite, actions })));
                return;
            }
        }

        crate::logging::log_debug!(
            "projectile: could not load a sprite for {:?} from the game archives",
            projectile
        );
        self.projectile_assets.insert(projectile, None);
    }

    /// Spawns the one-shot teleport/logout disappear effect (warp swirl +
    /// `ef_teleportation.wav`) at the position where an entity vanished.
    ///
    /// Only `Teleported` and `LoggedOut` disappearances use this; out-of-sight
    /// and death disappearances keep their plain fade / death handling.
    pub(crate) fn spawn_teleport_vanish(&mut self, position: Point3<f32>) {
        // Defensive lifetime/rate handling for bursts of population-engine
        // shells: keep at most TELEPORT_VANISH_BURST_LIMIT effects per window.
        self.recent_teleport_vanishes
            .retain(|spawn_time| spawn_time.elapsed() < TELEPORT_VANISH_WINDOW);

        if self.recent_teleport_vanishes.len() >= TELEPORT_VANISH_BURST_LIMIT {
            return;
        }

        let Some(warp_portal) = self.warp_portal.clone() else {
            return;
        };

        self.recent_teleport_vanishes.push(Instant::now());
        self.effect_holder.add_effect(Box::new(TeleportVanish::new(warp_portal, position)));
        // Spatial playback respects the muted/headless backend of the lean test
        // client (RO_NEXT_MUTE_AUDIO never opens an audio device).
        if let Some(sound_effect) = self.teleport_vanish_sound_effect {
            self.audio_engine.play_spatial_sound_effect(sound_effect, position, 250.0);
        }
        crate::logging::log_debug!(
            "entity: teleport vanish effect at ({:.1}, {:.1}, {:.1}) (sound ef_teleportation.wav)",
            position.x,
            position.y,
            position.z,
        );
    }

    /// Draws the ground cast circle below an entity for `duration_ms`.
    pub(crate) fn spawn_cast_circle(&mut self, entity_id: EntityId, color: Color, duration_ms: u32) {
        let center = EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0));
        // Textures, radius and ring animation come from `skills.ron`
        // (`cast_circle` section).
        let cast_circle = crate::world::skills::cast_circle();

        let Some(circle) = CastCircle::new(
            &self.texture_loader,
            center,
            cast_circle.radius,
            color,
            (duration_ms as f32 / 1000.0).max(0.2),
        ) else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to load the cast circle textures", "error".red());
            return;
        };

        self.effect_holder.add_effect(Box::new(circle));
    }

    #[allow(clippy::too_many_arguments)] // mirrors the effect-layer schema (phase, anchor, timing, tint)
    /// Spawns every layer of one skill phase.
    ///
    /// All visuals of a skill cast are driven by the single unified descriptor
    /// table ([`crate::world::skill_effect_layers`]): the `.str` screen
    /// sprites, the `.spr`/`.act` sprites, the texture billboards, the
    /// procedural primitives and the entity/body/overhead anchored layers
    /// of the phase spawn here, in data order.
    ///
    /// Every phase spawns at most once per cast even if several packets (cast
    /// start, ground cast, damage, ...) describe the same cast; the layer list
    /// is bounded by [`crate::world::MAX_LAYERS_PER_PHASE`] and every
    /// spawned effect goes through the effect holder's global/per-entity
    /// budget and distance culling. The spawn path iterates the static
    /// layer slice in place: no allocation per layer or per frame.
    pub(crate) fn spawn_skill_effect_phase(
        &mut self,
        skill_id: SkillId,
        phase: SkillPhase,
        source_entity_id: EntityId,
        target_entity_id: Option<EntityId>,
        target_position: Option<Point3<f32>>,
        client_tick: ClientTick,
        cast_time_ms: f32,
    ) {
        let layers = crate::world::skill_effect_layers(skill_id, phase);

        if layers.is_empty() {
            return;
        }

        if !self
            .skill_completion_tracker
            .claim(source_entity_id, skill_id, phase.index(), client_tick)
        {
            return;
        }

        let spawn_count = layers.len().min(crate::world::MAX_LAYERS_PER_PHASE);
        if layers.len() > spawn_count {
            crate::logging::log_warn!(
                "skill effect: skill {} phase {:?} has {} layers, spawning the first {}",
                skill_id.0,
                phase,
                layers.len(),
                spawn_count
            );
        }

        let source_position = self.entity_position(source_entity_id);

        for layer in &layers[..spawn_count] {
            self.spawn_skill_effect_layer(
                skill_id,
                phase,
                layer,
                source_entity_id,
                target_entity_id,
                target_position,
                source_position,
                client_tick,
                cast_time_ms,
            );
        }

        // Procedural cloud layers the official effect table cannot express
        // (Storm Gust's dark cloud above the cell, built from cloud1..7.tga):
        // the skill/phase/texture/timing rows live in `skills.ron` and are
        // claimed once per cast.
        if let Some(cloud_spec) = crate::world::skills::cloud(skill_id)
            && phase == cloud_spec.phase
            && let Some(world_position) = target_position
            && self
                .skill_completion_tracker
                .claim(source_entity_id, skill_id, crate::world::CLOUD_DEDUP_PHASE, client_tick)
            && let Some(cloud) = StormCloud::new(&self.texture_loader, world_position, cloud_spec)
        {
            crate::logging::log_debug!("skill effect: spawned procedural cloud for skill {}", skill_id.0);
            self.effect_holder.add_unit(Box::new(cloud), source_entity_id);
        }
    }

    /// Spawns one layer of a skill phase (see
    /// [`Self::spawn_skill_effect_phase`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn spawn_skill_effect_layer(
        &mut self,
        skill_id: SkillId,
        phase: SkillPhase,
        layer: &SkillEffectLayer,
        source_entity_id: EntityId,
        target_entity_id: Option<EntityId>,
        target_position: Option<Point3<f32>>,
        source_position: Option<Point3<f32>>,
        client_tick: ClientTick,
        cast_time_ms: f32,
    ) {
        // Data-file layers can be tied to the *actual* cast time (`cast_at`),
        // so the parts stay spread over the cast even when DEX shortens it;
        // instant casts skip them (official behaviour).
        let delay_ms = match layer.cast_at {
            Some(fraction) if cast_time_ms >= 300.0 => layer.delay_ms + (cast_time_ms * fraction).round() as u32,
            Some(_) => return,
            None => layer.delay_ms,
        };

        let Some((center, vertical, owner)) = self.skill_layer_center(layer, source_entity_id, target_entity_id, target_position) else {
            return;
        };

        match layer.kind {
            LayerKind::BodyFlash => {
                if let Some(entity) = self
                    .client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .find(|entity| entity.get_entity_id() == source_entity_id)
                {
                    entity.start_body_flash(layer.tint, layer.hold_ms, layer.fade_ms, layer.additive, client_tick);
                }
            }
            LayerKind::BodySquash => {
                if let Some(entity) = self
                    .client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .find(|entity| entity.get_entity_id() == source_entity_id)
                {
                    entity.start_body_squash(layer.duration_ms, layer.amplitude, client_tick);
                }
            }
            LayerKind::BodyOutline => {
                if let Some(entity) = self
                    .client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .find(|entity| entity.get_entity_id() == source_entity_id)
                {
                    entity.start_body_outline(layer.tint, layer.duration_ms, client_tick);
                }
            }
            LayerKind::BodyWave => {
                if let Some(entity) = self
                    .client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .find(|entity| entity.get_entity_id() == source_entity_id)
                {
                    entity.start_body_wave(layer.duration_ms, layer.amplitude, client_tick);
                }
            }
            LayerKind::Str => {
                let Some(effect_path) = layer.file else {
                    return;
                };
                let (light_color, light_intensity) = layer.light.unwrap_or((Color::WHITE, 0.0));
                let effect_offset = layer.offset + Vector3::new(0.0, vertical, 0.0);

                crate::logging::log_debug!(
                    "skill effect: spawned '{}' for skill {} ({:?}, anchor {:?}, vertical {:?})",
                    effect_path,
                    skill_id.0,
                    phase,
                    layer.anchor,
                    layer.vertical
                );

                match center {
                    EffectCenter::Entity(entity_id, _) => {
                        self.spawn_entity_effect(effect_path, entity_id, effect_offset, layer.scale, light_color, light_intensity)
                    }
                    EffectCenter::Position(position) => {
                        self.spawn_position_effect(effect_path, position, effect_offset, layer.scale, light_color, light_intensity)
                    }
                }
            }
            LayerKind::Texture => {
                let Some(params) = PrimitiveParams::from_layer(layer) else {
                    return;
                };
                let (center, offset) = self.texture_layer_center_with_offset(center, layer, vertical);

                // A `frames` layer is the official client's multi-texture
                // (totalTexture) component: play it as a flipbook.
                if !layer.frames.is_empty() {
                    let Some(sequence) = TextureSequence::spawn(&self.texture_loader, center, layer.frames, layer.frame_ms, &params) else {
                        return;
                    };

                    crate::logging::log_debug!(
                        "skill effect: spawned texture sequence ({} frames, {} ms/frame) for skill {} ({:?}, anchor {:?}, vertical {:?})",
                        layer.frames.len(),
                        layer.frame_ms,
                        skill_id.0,
                        phase,
                        layer.anchor,
                        layer.vertical
                    );

                    self.add_skill_effect(Box::new(sequence), owner);
                    return;
                }

                let Some(flash) = HitFlash::spawn(&self.texture_loader, center, offset, delay_ms, &params) else {
                    return;
                };

                crate::logging::log_debug!(
                    "skill effect: spawned texture layer '{}' for skill {} ({:?}, anchor {:?}, vertical {:?})",
                    params.texture,
                    skill_id.0,
                    phase,
                    layer.anchor,
                    layer.vertical
                );

                self.add_skill_effect(Box::new(flash), owner);
            }
            LayerKind::Cylinder => {
                let Some(params) = PrimitiveParams::from_layer(layer) else {
                    return;
                };
                let center = self.texture_layer_center(center, layer, vertical);
                let Some(aura) = spawn_ground_aura(&self.texture_loader, center, &params) else {
                    return;
                };

                crate::logging::log_debug!(
                    "skill effect: spawned cylinder layer '{}' for skill {} ({:?}, anchor {:?}, vertical {:?})",
                    params.texture,
                    skill_id.0,
                    phase,
                    layer.anchor,
                    layer.vertical
                );

                self.add_skill_effect(Box::new(aura), owner);
            }
            LayerKind::Spr => {
                // Official sprite layers use `duplicate` to spawn several copies
                // around the entity (for example the six Blessing sparkles). The
                // copies are spread on a ring of the layer spread (a small
                // fallback radius keeps copies visible when the data has no
                // spread); the first loadable candidate pair is used for all of
                // them.
                let count = layer.count.clamp(1, 16) as usize;
                let spread = match layer.spread {
                    spread if spread > 0.0 => spread,
                    _ => 1.5,
                };

                for index in 0..count {
                    let mut params = SpriteEffectParams::from_layer(layer);
                    let angle = index as f32 * std::f32::consts::TAU / count as f32;
                    params.offset += Vector3::new(angle.cos() * spread, vertical, angle.sin() * spread);

                    let Some(sprite_effect) = SpriteEffect::spawn(&self.sprite_loader, &self.action_loader, center.clone(), &params) else {
                        if index == 0 {
                            crate::logging::log_debug!(
                                "skill effect: no sprite effect asset could be loaded for skill {} ({:?})",
                                skill_id.0,
                                phase
                            );
                        }
                        return;
                    };

                    crate::logging::log_debug!(
                        "skill effect: spawned sprite layer '{}' for skill {} ({:?}, anchor {:?}, vertical {:?}, copy {}/{})",
                        params.files.first().copied().unwrap_or("<none>"),
                        skill_id.0,
                        phase,
                        layer.anchor,
                        layer.vertical,
                        index + 1,
                        count
                    );

                    self.effect_holder.add_effect(Box::new(sprite_effect));
                }
            }
            LayerKind::Primitive => {
                let Some(params) = PrimitiveParams::from_layer(layer) else {
                    return;
                };

                match layer.primitive {
                    Some(LayerPrimitive::Projectile) => {
                        let Some(target) = target_position else {
                            return;
                        };
                        let (mode, start) = match layer.anchor {
                            LayerAnchor::Rain => (ProjectileMode::Rain, target),
                            _ => {
                                let Some(caster) = source_position else {
                                    return;
                                };
                                (ProjectileMode::Travel, caster)
                            }
                        };

                        if let Some(projectile) = Projectile::spawn(&self.texture_loader, &params, mode, start, target) {
                            crate::logging::log_debug!(
                                "skill effect: spawned projectile layer '{}' for skill {} ({:?})",
                                params.texture,
                                skill_id.0,
                                phase
                            );
                            self.effect_holder.add_effect(Box::new(projectile));
                        }
                    }
                    Some(LayerPrimitive::GroundAura) => {
                        let center = self.texture_layer_center(center, layer, vertical);
                        let Some(aura) = spawn_ground_aura(&self.texture_loader, center, &params) else {
                            return;
                        };

                        crate::logging::log_debug!(
                            "skill effect: spawned primitive ground aura '{}' for skill {} ({:?})",
                            params.texture,
                            skill_id.0,
                            phase
                        );

                        self.add_skill_effect(Box::new(aura), owner);
                    }
                    Some(LayerPrimitive::HitFlash) | None => {
                        let (center, offset) = self.texture_layer_center_with_offset(center, layer, vertical);
                        let Some(flash) = HitFlash::spawn(&self.texture_loader, center, offset, delay_ms, &params) else {
                            return;
                        };

                        crate::logging::log_debug!(
                            "skill effect: spawned primitive hit flash '{}' for skill {} ({:?})",
                            params.texture,
                            skill_id.0,
                            phase
                        );

                        self.add_skill_effect(Box::new(flash), owner);
                    }
                }
            }
        }
    }

    /// Adds a spawned skill effect, attached to its entity when the layer asked
    /// for it (which makes the per-entity budget, cleanup on entity removal and
    /// distance culling apply).
    pub(crate) fn add_skill_effect(&mut self, effect: Box<dyn EffectBase + Send + Sync>, owner: Option<EntityId>) {
        match owner {
            Some(entity_id) => {
                self.effect_holder.add_unit(effect, entity_id);
            }
            None => {
                self.effect_holder.add_effect(effect);
            }
        }
    }

    /// The position of an entity of the current map, if it is known.
    pub(crate) fn entity_position(&self, entity_id: EntityId) -> Option<Point3<f32>> {
        self.client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| entity.get_position())
    }

    /// Overhead and body height anchors of an entity (in world units above its
    /// feet), used by the `vertical` anchor of skill effect layers.
    pub(crate) fn entity_effect_heights(&self, entity_id: EntityId) -> Option<(f32, f32)> {
        self.client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| {
                (
                    crate::world::overhead_effect_height(entity),
                    crate::world::body_effect_height(entity),
                )
            })
    }

    /// Resolves the spawn center, the entity-only vertical offset and the owner
    /// entity of a layer.
    ///
    /// Entity anchors include the overhead/body height of the live entity (so
    /// buff layers follow the entity scale and sitting state); fixed world
    /// cells keep the plain data offset.
    pub(crate) fn skill_layer_center(
        &self,
        layer: &SkillEffectLayer,
        source_entity_id: EntityId,
        target_entity_id: Option<EntityId>,
        target_position: Option<Point3<f32>>,
    ) -> Option<(EffectCenter, f32, Option<EntityId>)> {
        let placement = layer.placement(target_entity_id.is_some(), target_position.is_some())?;

        let entity_center = |entity_id: EntityId, this: &Self| {
            let (overhead_height, body_height) = this.entity_effect_heights(entity_id)?;
            let vertical = match layer.vertical {
                LayerVertical::Feet => 0.0,
                LayerVertical::Body => body_height,
                LayerVertical::Overhead => overhead_height,
            };

            Some((
                EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
                vertical,
                layer.attached_entity.then_some(entity_id),
            ))
        };

        match placement {
            LayerPlacement::SourceEntity => entity_center(source_entity_id, self),
            LayerPlacement::TargetEntity => entity_center(target_entity_id?, self),
            LayerPlacement::TargetPosition => Some((EffectCenter::Position(target_position?), 0.0, None)),
            LayerPlacement::SourcePosition => {
                let position = self.entity_position(source_entity_id)?;
                Some((EffectCenter::Position(position), 0.0, None))
            }
        }
    }

    /// Combines a resolved layer center with the layer offset and vertical
    /// anchor for the texture/cylinder primitives (they take a center point
    /// directly).
    /// Like [`Self::texture_layer_center`], but keeps the offset separate for
    /// procedural primitives: `EffectCenter::follow_entity` refreshes the
    /// entity position every frame and would otherwise drop the data-file
    /// offset.
    pub(crate) fn texture_layer_center_with_offset(
        &self,
        center: EffectCenter,
        layer: &SkillEffectLayer,
        vertical: f32,
    ) -> (EffectCenter, Vector3<f32>) {
        let offset = Vector3::new(layer.offset.x, vertical + layer.offset.y, layer.offset.z);

        match center {
            EffectCenter::Entity(entity_id, position) => (EffectCenter::Entity(entity_id, position), offset),
            EffectCenter::Position(position) => (EffectCenter::Position(position + offset), Vector3::new(0.0, 0.0, 0.0)),
        }
    }

    pub(crate) fn texture_layer_center(&self, center: EffectCenter, layer: &SkillEffectLayer, vertical: f32) -> EffectCenter {
        match center {
            EffectCenter::Entity(entity_id, _) => EffectCenter::Entity(
                entity_id,
                Point3::new(layer.offset.x, vertical + layer.offset.y, layer.offset.z),
            ),
            EffectCenter::Position(position) => EffectCenter::Position(position + layer.offset),
        }
    }

    /// Applies an aura style selected through `/aura` or by the server-side
    /// aura stylist NPC (via the private chat marker) and persists it
    /// immediately.
    ///
    /// The running aura effects read the style from a shared atomic cell that
    /// is refreshed from the interface settings every frame, so the change
    /// is visible on the next frame without recreating the effects.
    pub(crate) fn set_aura_style(&mut self, style: AuraStyle) {
        let style_path = client_state().interface_settings().level_99_aura_style();
        *self.client_state.follow_mut(style_path) = style;

        self.client_state.follow(client_state().interface_settings()).save();

        self.push_localized_chat(client_state().localization().chat_aura_style_set(), &[(
            "style",
            style.style_name(),
        )]);
    }
}
