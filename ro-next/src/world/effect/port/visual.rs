//! Client-facing adapter for the official-effect port.
//!
//! [`PortedEffectHolder`] deliberately separates the simulation (the exact
//! 16 ms clock and the primitive state machines) from the client's effect list:
//! it is not an [`EffectBase`]. `PortedVisual` is the bridge. It owns one
//! holder, steps it with the wall-clock delta the client passes to every
//! effect, and forwards rendering to `holder.render`.
//!
//! Rendering support is what the holder implements: the
//! `PP_2DTEXTURE`/`PP_2DFLASH` screen quads anchored to the effect's world
//! position, the world-space quads of `PP_3DTEXTURE` (15) and
//! `PP_3DCROSSTEXTURE` (16) built by [`super::render3d`], the
//! `RenderSegmentSprites2` sprite pass of [`super::sprite_pass`] for the
//! `.spr` particle families, and the slot/record primitives of
//! [`super::render_slots`] (rings, cylinders, columns, fans, halo, wind, arc
//! walls and the `TexturedAura` glow of prim 82) as additive triangle lists,
//! and the effect's `.str` clip layers of [`super::str_render`]. The remaining
//! primitive families (`PP_3DSPHERE`, the arc/sweep families and the effect
//! ids whose `Render*` member is not ported yet) are skipped rather than
//! guessed.
//!
//! The sprite pass needs the client's [`SpriteLoader`]/[`ActionLoader`]; they
//! are handed to the holder here so it can resolve the ACT frame count while
//! the effect is stepped and draw the sprite quads. The [`GameFileLoader`] is
//! handed over for the same reason: the holder opens the effect's official
//! `.str` screen-sprite clips (`port/str_table.rs`) at spawn, before the first
//! 16 ms step.
//!
//! Sounds recorded by the ported bodies are forwarded to the client's
//! [`AudioEngine`] through [`super::sound_sink::AudioEffectSink`]: every 16 ms
//! step drains the runtime's inline and positional channels and plays each
//! entry with the same `AudioEngine::load` + `play_spatial_sound_effect`
//! calls the descriptor effects use. A visual without an audio engine (unit
//! tests) stays silent.

use std::collections::HashSet;
use std::sync::Arc;

use cgmath::Point3;
use ro_next_audio::AudioEngine;

use super::holder::PortedEffectHolder;
use super::runtime::EffectRuntime;
use super::sound_sink::AudioEffectSink;
use crate::loaders::{ActionLoader, GameFileLoader, ImageType, SpriteLoader, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::effect::EffectBase;
use crate::world::{Camera, PointLightManager};

/// A live ported effect in the client's effect list.
pub struct PortedVisual {
    holder: PortedEffectHolder,
    /// `None` only in unit tests, which have no wgpu device. Rendering is a
    /// no-op without a loader; simulation and lifetime stay testable.
    texture_loader: Option<Arc<TextureLoader>>,
    /// The client's audio engine. `None` in unit tests (no audio device); the
    /// recorded sounds are then dropped by [`PortedEffectHolder::update`].
    audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
    /// Assets already requested from the loader. Ported bodies launch their
    /// primitives over time, so the texture list grows after construction.
    /// Owned strings because a `.str` clip's texture names come from its
    /// resource file and are not `&'static`.
    prefetched: HashSet<String>,
    gets_deleted: bool,
    /// The caster tint this visual last applied, i.e. the running
    /// `render.colorChannels` of the official actor. `SetColor` leaves a
    /// channel untouched when its argument is `-1`, so the previous value is
    /// needed to resolve such a call.
    applied_tint: Option<crate::graphics::Color>,
    /// Effect frame of the last `SetFrameSequence` applied to the caster.
    /// `parent_frame_sequence` stays set on the runtime forever; this stamp
    /// makes the visual apply each sequence once instead of restarting its
    /// 25 ms window on every 16 ms step.
    applied_frame_sequence: Option<i32>,
}

impl PortedVisual {
    pub fn new(
        holder: PortedEffectHolder,
        texture_loader: Arc<TextureLoader>,
        sprite_loader: Arc<SpriteLoader>,
        action_loader: Arc<ActionLoader>,
        game_file_loader: Arc<GameFileLoader>,
    ) -> Self {
        let mut visual = Self {
            // The game-file loader opens the effect's official `.str` clips
            // (`port/str_table.rs`) before the first step, like
            // `CRagEffect::Init` does on the original.
            holder: holder
                .with_sprite_loaders(sprite_loader, action_loader)
                .with_game_file_loader(&game_file_loader),
            texture_loader: Some(texture_loader),
            audio_engine: None,
            prefetched: HashSet::new(),
            gets_deleted: false,
            applied_tint: None,
            applied_frame_sequence: None,
        };

        visual.prefetch_textures();

        visual
    }

    /// Attaches the client's audio engine, so the sounds the ported bodies
    /// record are played through the existing sound-effect mixer.
    ///
    /// Kept out of [`Self::new`] so the unit tests (and any headless caller)
    /// can drive the visual without an audio device; without it the visual
    /// stays silent.
    pub fn with_audio_engine(mut self, audio_engine: Arc<AudioEngine<GameFileLoader>>) -> Self {
        self.audio_engine = Some(audio_engine);

        self
    }

    pub fn runtime(&self) -> &EffectRuntime {
        self.holder.runtime()
    }

    pub fn runtime_mut(&mut self) -> &mut EffectRuntime {
        self.holder.runtime_mut()
    }

    pub fn is_finished(&self) -> bool {
        self.gets_deleted || self.holder.is_finished()
    }

    /// Requests every texture the effect currently references.
    ///
    /// The holder renders through the loader's cache, so this is the same
    /// `get_or_load` path the other effects use (`HitFlash::spawn`,
    /// `CastCircle::new`); doing it during the update pass keeps the first
    /// visible frame from hitching on a disk read. A missing texture is not
    /// fatal: the primitive simply draws nothing, as with the descriptor
    /// effects.
    fn prefetch_textures(&mut self) {
        let Some(texture_loader) = &self.texture_loader else {
            return;
        };

        for path in self.holder.textures() {
            if !self.prefetched.insert(path.to_owned()) {
                continue;
            }

            if texture_loader.get_or_load(path, ImageType::Color).is_err() {
                crate::logging::log_debug!("ported effect: missing texture '{}'", path);
            }
        }
    }
}

impl EffectBase for PortedVisual {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if self.gets_deleted {
            return false;
        }

        let alive = match &self.audio_engine {
            Some(audio_engine) => {
                let mut sink = AudioEffectSink::new(audio_engine);
                self.holder.update_with_sounds(delta_time as f64, &mut sink)
            }
            // No audio device (tests): the simulation still steps and drops
            // the sounds.
            None => self.holder.update(delta_time as f64),
        };
        self.prefetch_textures();

        alive
    }

    /// Applies the caster requests the ported body recorded on its runtime.
    ///
    /// This is the mutable-actor bridge the field docs of
    /// `EffectRuntime::parent_color`/`parent_frame_sequence` pointed at:
    /// [`EffectHolder::update`](crate::world::effect::EffectHolder::update)
    /// steps every effect against the immutable slice and then calls this
    /// method on each effect's owning entity.
    ///
    /// * `SetColor` is replayed as [`Entity::set_status_tint`] with `-1`
    ///   channels keeping their previously applied value, exactly like
    ///   `RenderObject.cpp:534-547`.
    /// * `SetFrameSequence` is replayed with the client's existing
    ///   `set_custom_skill_action` + `set_skill_frame_sequence` pair. The
    ///   official `animation` byte is an ACT block offset (`0x60` = block 12,
    ///   i.e. `0x60 + direction`), so the block index is `animation / 8`; the
    ///   frame range and the 25 ms duration are copied unchanged. Only the
    ///   first application of a sequence is sent (the runtime keeps the last
    ///   request for its whole lifetime).
    /// * `render.state` (e.g. `ST_NINJAREADY = 37`) is recorded on the runtime
    ///   but has no client-side state machine to drive; the visible part of the
    ///   official write - animation `0x60`, frame `2`, 25 ms - is carried by
    ///   the frame sequence above.
    /// * The actor render bytes the manual-tier bodies write directly on the
    ///   parent (`render.effectPhase`, `tintPhase`, `shrinkPhase`,
    ///   `outlineAlpha`, `runStartTick`; `RenderObjectState.h:121-129`) are
    ///   replayed onto the caster's `Entity` registers by
    ///   [`apply_render_fields`]. Those registers persist for the whole effect
    ///   because the original only clears them in `CRenderObject::Reset`; the
    ///   visual re-applies the runtime's last write every frame and clears it
    ///   when the effect retires. The client has no render pass for these
    ///   phases yet, so they are applied-but-record-only (see
    ///   `world::entity::Common`'s field docs for the official consumers that
    ///   would read them).
    fn apply_caster_state(&mut self, entity: &mut crate::world::Entity, client_tick: ragnarok_packets::ClientTick) {
        let runtime = self.holder.runtime();

        if let Some(channels) = runtime.parent_color {
            let tint = tint_from_channels(channels, self.applied_tint.unwrap_or(crate::graphics::Color::WHITE));
            self.applied_tint = Some(tint);
            entity.set_status_tint(Some(tint));
        }

        if let Some(sequence) = runtime.parent_frame_sequence
            && self.applied_frame_sequence != Some(runtime.parent_frame_sequence_frame)
        {
            let duration = sequence.duration.max(0) as u32;

            entity.set_custom_skill_action(action_block(sequence.animation), duration, sequence.loop_ != 0, client_tick);
            entity.set_skill_frame_sequence(
                sequence.first_frame,
                sequence.last_frame,
                sequence.finish_frame,
                duration,
                client_tick,
            );

            self.applied_frame_sequence = Some(runtime.parent_frame_sequence_frame);
        }

        apply_render_fields(entity, CasterRenderFields::read(runtime));
    }

    /// Clears the tint this visual applied when the effect retires.
    ///
    /// Only a tint that was actually applied is cleared, so a visual that never
    /// called `SetColor` cannot erase a status tint owned by another system.
    /// The clear sets `None`; if a status effect also owns a tint on this
    /// entity, `Client::refresh_entity_status_visual` re-applies it on its next
    /// status change (the official actor has no separate tint stack either -
    /// the last `SetColor` simply wins until something writes again).
    fn clear_caster_state(&mut self, entity: &mut crate::world::Entity) {
        if self.applied_tint.take().is_some() {
            entity.set_status_tint(None);
        }

        self.applied_frame_sequence = None;

        clear_render_fields(entity, CasterRenderFields::read(self.holder.runtime()));
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let Some(texture_loader) = &self.texture_loader else {
            return;
        };

        // 2-D screen quads, the world-space `PP_3DTEXTURE`/
        // `PP_3DCROSSTEXTURE` quads and the `.spr` particle quads; the holder
        // owns the primitive dispatch.
        self.holder.render(renderer, camera, texture_loader);
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        let [x, y, z] = self.holder.runtime().position;

        Some(Point3::new(x, y, z))
    }
}

/// Applies one `SetColor(alpha, red, green, blue)` to the running tint.
///
/// The official call leaves a channel untouched when its argument is `-1`
/// (`RenderObject.cpp:534-547`), so `previous` supplies that value; the effect
/// starts from the actor's white (255,255,255,255) defaults.
fn tint_from_channels(channels: [i16; 4], previous: crate::graphics::Color) -> crate::graphics::Color {
    let channel = |value: i16, previous: f32| {
        if value == -1 { previous } else { f32::from(value as u8) / 255.0 }
    };

    crate::graphics::Color::rgba(
        channel(channels[1], previous.red),
        channel(channels[2], previous.green),
        channel(channels[3], previous.blue),
        channel(channels[0], previous.alpha),
    )
}

/// `SetFrameSequence`'s `animation` argument is the ACT block offset
/// (`0x60` = block 12); `Entity`'s custom action takes the block index.
fn action_block(animation: u8) -> usize {
    usize::from(animation / 8)
}

/// The five actor render bytes the manual-tier effect bodies write directly on
/// the shared parent (`CRenderObject::render`, `RenderObjectState.h:121-129`).
///
/// The official bodies mutate the shared actor from inside `OnProcess`; the
/// port cannot, so the runtime keeps the write and this snapshot carries it to
/// the caster's [`crate::world::Entity`]. `None` means the effect never wrote
/// that register, and therefore must not clear another effect's value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct CasterRenderFields {
    /// `short render.effectPhase` (`RenderObjectState.h:127`).
    effect_phase: Option<i16>,
    /// `unsigned char render.tintPhase` (`:128`).
    tint_phase: Option<u8>,
    /// `unsigned char render.shrinkPhase` (`:121`, low byte of the
    /// `pulsePhaseState` union).
    shrink_phase: Option<u8>,
    /// `unsigned char render.outlineAlpha` (`:125`).
    outline_alpha: Option<u8>,
    /// `unsigned char render.runStartTick` (`:129`).
    run_start_tick: Option<u8>,
}

impl CasterRenderFields {
    /// Reads the runtime's last write of every register. The values persist on
    /// the runtime because `CRenderObject::Reset` (`RenderObjectState.cpp:75`,
    /// `:90`) is their only reset in the original.
    fn read(runtime: &EffectRuntime) -> Self {
        Self {
            effect_phase: runtime.parent_effect_phase,
            tint_phase: runtime.parent_tint_phase,
            shrink_phase: runtime.parent_shrink_phase,
            outline_alpha: runtime.parent_outline_alpha,
            run_start_tick: runtime.parent_run_start_tick,
        }
    }
}

/// Replays each register the effect actually wrote onto the caster entity.
///
/// Unwritten (`None`) registers are left alone: a second effect on the same
/// actor must keep its own value.
fn apply_render_fields(entity: &mut crate::world::Entity, fields: CasterRenderFields) {
    if let Some(phase) = fields.effect_phase {
        entity.set_render_effect_phase(Some(phase));
    }

    if let Some(phase) = fields.tint_phase {
        entity.set_render_tint_phase(Some(phase));
    }

    if let Some(phase) = fields.shrink_phase {
        entity.set_render_shrink_phase(Some(phase));
    }

    if let Some(alpha) = fields.outline_alpha {
        entity.set_render_outline_alpha(Some(alpha));
    }

    if let Some(tick) = fields.run_start_tick {
        entity.set_render_run_start_tick(Some(tick));
    }
}

/// Clears exactly the registers [`apply_render_fields`] installed.
fn clear_render_fields(entity: &mut crate::world::Entity, fields: CasterRenderFields) {
    if fields.effect_phase.is_some() {
        entity.set_render_effect_phase(None);
    }

    if fields.tint_phase.is_some() {
        entity.set_render_tint_phase(None);
    }

    if fields.shrink_phase.is_some() {
        entity.set_render_shrink_phase(None);
    }

    if fields.outline_alpha.is_some() {
        entity.set_render_outline_alpha(None);
    }

    if fields.run_start_tick.is_some() {
        entity.set_render_run_start_tick(None);
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::EffectId;

    use super::*;

    fn visual(effect: EffectId) -> PortedVisual {
        PortedVisual {
            holder: PortedEffectHolder::new(effect, 1).expect("registered effect"),
            texture_loader: None,
            audio_engine: None,
            prefetched: HashSet::new(),
            gets_deleted: false,
            applied_tint: None,
            applied_frame_sequence: None,
        }
    }

    #[test]
    fn registered_effect_yields_a_live_visual() {
        let mut visual = visual(EffectId::Bash);

        assert!(!visual.is_finished());
        assert_eq!(visual.runtime().frame, 0);

        assert!(visual.update(&[], 0.016));
        assert_eq!(visual.runtime().frame, 1, "one 16 ms step");

        assert!(!visual.is_finished());
    }

    #[test]
    fn finishes_after_the_effect_lifetime() {
        let mut visual = visual(EffectId::Bash);

        // Bash is 40 steps long; `update(0.016)` consumes exactly one step.
        for _ in 0..40 {
            visual.update(&[], 0.016);
        }

        assert!(visual.is_finished());
        assert_eq!(visual.runtime().frame, 40);
        assert!(!visual.update(&[], 0.016), "stays finished");
    }

    #[test]
    fn update_accumulates_short_frames() {
        let mut visual = visual(EffectId::Bash);

        for _ in 0..5 {
            visual.update(&[], 0.01);
        }

        assert_eq!(visual.runtime().frame, 3, "50 ms = three 16 ms steps");
    }

    #[test]
    fn deletion_stops_updates() {
        let mut visual = visual(EffectId::Bash);

        visual.mark_for_deletion();

        assert!(visual.is_finished());
        assert!(!visual.update(&[], 0.016));
    }

    #[test]
    fn unported_effect_has_no_holder() {
        assert!(PortedEffectHolder::new(EffectId::Stormkick8, 1).is_none());
    }

    #[test]
    fn str_effect_keeps_its_opened_clip_across_update() {
        use super::super::str_player::{Clip, KeyFrame, Transform};

        let clips = vec![Clip {
            texture_count: 1,
            frames: vec![
                KeyFrame {
                    frame: 0,
                    transform: Transform {
                        x: 1.0,
                        ..Default::default()
                    },
                },
                KeyFrame {
                    frame: 3,
                    transform: Transform {
                        x: 2.0,
                        ..Default::default()
                    },
                },
            ],
        }];
        let mut visual = PortedVisual {
            holder: PortedEffectHolder::new(EffectId::Signum, 1)
                .expect("registered effect")
                .with_str_clips(clips),
            texture_loader: None,
            audio_engine: None,
            prefetched: HashSet::new(),
            gets_deleted: false,
            applied_tint: None,
            applied_frame_sequence: None,
        };

        assert!(visual.runtime().str_player.is_some());
        assert_eq!(visual.runtime().str_clips.len(), 1);

        for _ in 0..4 {
            assert!(visual.update(&[], 0.016));
        }

        let player = visual.runtime().str_player.as_ref().expect("clip still open");
        assert_eq!(player.transforms[0].x, 2.0, "the `.str` timeline advanced with the 16 ms clock");

        // With loopLimit 0 the finished resource retires the effect.
        assert!(!visual.update(&[], 0.016));
        assert!(visual.is_finished());
    }

    #[test]
    fn tint_channels_keep_previous_values_for_minus_one() {
        use crate::graphics::Color;

        // `SetColor(-1, 255, 255, 160)` - the alpha channel is untouched.
        let tint = tint_from_channels([-1, 255, 255, 160], Color::WHITE);
        assert!((tint.red - 1.0).abs() < 1e-6);
        assert!((tint.green - 1.0).abs() < 1e-6);
        assert!((tint.blue - 160.0 / 255.0).abs() < 1e-6);
        assert!((tint.alpha - 1.0).abs() < 1e-6);

        // The second `SetColor(-1, 100, 100, 100)` of the Reject Sword blink
        // replaces all three colour channels and keeps the alpha.
        let tint = tint_from_channels([-1, 100, 100, 100], tint);
        assert!((tint.red - 100.0 / 255.0).abs() < 1e-6);
        assert!((tint.alpha - 1.0).abs() < 1e-6);
    }

    #[test]
    fn frame_sequence_animation_converts_to_the_act_block_index() {
        // `SetFrameSequence(0x60, ...)` selects block 96 = 12 * 8, the block
        // `CasterAction::Action(12)` names in the skill-action table.
        assert_eq!(action_block(0x60), 12);
        assert_eq!(action_block(0x58), 11);
        assert_eq!(action_block(0), 0);
    }

    #[test]
    fn caster_render_fields_read_only_the_registers_the_body_wrote() {
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 1);

        // Nothing written: the bridge must not clear another effect's value.
        assert_eq!(CasterRenderFields::read(&runtime), CasterRenderFields::default());

        runtime.set_parent_effect_phase(14);
        runtime.set_parent_tint_phase(63);
        runtime.set_parent_shrink_phase(22);
        runtime.set_parent_outline_alpha(200);
        runtime.set_parent_run_start_tick(7);

        assert_eq!(CasterRenderFields::read(&runtime), CasterRenderFields {
            effect_phase: Some(14),
            tint_phase: Some(63),
            shrink_phase: Some(22),
            outline_alpha: Some(200),
            run_start_tick: Some(7),
        });
    }

    #[test]
    fn caster_render_fields_keep_the_last_write_across_steps() {
        // `CRenderObject::Reset` is the only reset; `begin_step` must not drop
        // the phase, so the bridge keeps re-applying it until retirement.
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 1);
        runtime.set_parent_effect_phase(33);
        runtime.set_parent_tint_phase(34);

        let mut reference = runtime;
        reference.begin_step();

        let fields = CasterRenderFields::read(&reference);
        assert_eq!(fields.effect_phase, Some(33));
        assert_eq!(fields.tint_phase, Some(34));
        assert_eq!(fields.shrink_phase, None, "never written");
    }
}
