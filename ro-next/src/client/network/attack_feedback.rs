//! Auto-attack hit feedback: the impact wave and the defender hit effect.
//!
//! The official client feeds both from the damage message, not from the
//! attack animation (see [`crate::world::hit_feedback`] for the full
//! decompilation references):
//!
//! * `SendMsg(AM_ATTACKED, ...)` plays `GetWeaponHitWaveName(...)` /
//!   `GetHitWaveName(...)` at the defender (`GameObject.cpp:1782-1841`), with
//!   maximum distance `250` and no height offset (`:1840`);
//! * the same message spawns `effectId` on the defender: `EF_HIT1` (`0`),
//!   `EF_HITLINE7` for the Taekwon/Star job family, `EF_VIOLEN_HIT` (2420) for
//!   Violen and `EF_HIT2` (`1`) on a critical hit as well
//!   (`RenderObject.cpp:3769-3822`, `GameObject.cpp:1843-1888`);
//! * a miss (`damage == 0`) plays/spawns neither (`GameObject.cpp:1768`).
//!
//! The decision itself is data-driven and lives in
//! [`crate::world::hit_feedback::auto_attack_hit_feedback`]; this module only
//! resolves the entity facts and forwards the result to the existing audio
//! engine and the ported-effect holder (the same `AudioEngine::load` +
//! `play_spatial_sound_effect` / `PortedEffectHolder` path the descriptor and
//! ported skill effects use). One wave is played per damage message, and the
//! arrow projectile no longer plays `_hit_arrow.wav` on landing
//! (`world/effect/arrow_effect.rs`), so a bow auto attack is heard exactly
//! once.
//!
//! A dual-wielding attacker produces **two** damage messages: the packet's
//! `damage` and its `damage2` (`RenderObject.cpp:3700`, `:3856-3877`). The
//! second one is queued until `message.time`
//! (`CGameActor::ProcessActorEffects`, `:4712-4721`), i.e.
//! [`crate::world::hit_feedback::dual_wield_hit_delay_ms`] after the main-hand
//! message, and carries its own wave and hit effects. The client spawns the
//! second message's feedback through [`DelayedHitFeedback`], which holds the
//! same wave/effect objects and releases them at that deadline.

use std::sync::Arc;

use ragnarok_packets::EffectId;
use ro_next_audio::{AudioEngine, SoundEffectKey};

use crate::loaders::GameFileLoader;
use crate::renderer::EffectRenderer;
use crate::world::port::holder::PortedEffectHolder;
use crate::world::port::visual::PortedVisual;
use crate::world::{Camera, PointLightManager};
use crate::*;

/// `g_soundMgr->PlayWave` maximum distance argument of the official hit wave
/// (`GameObject.cpp:1840`).
const HIT_SOUND_RANGE: f32 = 250.0;

impl Client {
    /// Plays the official impact wave and spawns the official defender effect
    /// for one auto-attack damage message.
    ///
    /// `damage` is `false` for a miss: the official path shows the miss number
    /// and returns before the sound/effect block (`GameObject.cpp:1768-1777`).
    /// Exactly one wave is loaded/played per hit. `delay_ms` is the message's
    /// official delay: `0` for the main-hand message, the dual-wield stagger
    /// for the off-hand one, which then runs through [`DelayedHitFeedback`].
    pub(crate) fn play_auto_attack_hit_feedback(
        &mut self,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        damage: bool,
        is_critical: bool,
        client_tick: ClientTick,
        delay_ms: u32,
    ) {
        if !damage {
            return;
        }

        let attacker = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == source_entity_id)
            .map(|entity| {
                let job_id = entity.get_job_id();
                HitAttacker {
                    entity_id: source_entity_id,
                    job_id,
                    weapon: entity.get_weapon(),
                    // `IsPlayerJob(GetEffectiveJob(-1))`: only player actors
                    // carry a weapon (`CGameActor::GetWeapon()` is -1,
                    // `RenderObject.cpp:2190-2192`).
                    is_player: EntityType::from(job_id) == EntityType::Player,
                }
            });

        // The killing blow can be reported by the death/remove packet before
        // this damage message is dispatched: the monster is then only in
        // `dead_entities`. The official client plays the impact wave and
        // spawns the hit effect from the damage message at the damaged actor's
        // position (`GameObject.cpp:1782-1888`), independent of the actor's
        // lifetime, so the live-only lookup must not drop the feedback.
        // Resolve the live entity first and fall back to the last known dead
        // entity; never gate the wave/effect on the target still being alive.
        let Some(target) = resolve_target(
            destination_entity_id,
            self.client_state
                .follow(client_state().entities())
                .iter()
                .chain(self.client_state.follow(client_state().dead_entities()).iter())
                .map(TargetSnapshot::from_entity),
        ) else {
            crate::logging::log_debug!(
                "hit feedback: defender {} is not tracked, skipping the impact wave/effect",
                destination_entity_id.0
            );
            return;
        };

        let defender = target.defender();

        if !self
            .client_state
            .follow(client_state().entities())
            .iter()
            .any(|entity| entity.get_entity_id() == destination_entity_id)
        {
            crate::logging::log_debug!(
                "hit feedback: defender {} was already removed, using its last known position {:?} (killing blow)",
                destination_entity_id.0,
                target.position
            );
        }

        // The two damage messages of one packet re-roll the wave/effect
        // variance independently (`std::rand()` per `SendMsg`), so the
        // deterministic roll is mixed with the message index.
        let hit = u8::from(delay_ms > 0);
        let roll = hit_feedback_roll(client_tick, source_entity_id, destination_entity_id, hit);
        let feedback = auto_attack_hit_feedback(attacker, defender, is_critical, damage, roll);

        let target_position = target.position;
        let target_angle = target.parent_angle();
        let listener_position = self.client_state.try_follow(this_entity()).map(|entity| entity.get_position());

        let sound = feedback.sound.map(|wave| {
            crate::logging::log_debug!(
                "hit feedback: auto attack {} -> {} plays '{}' (delay {} ms)",
                source_entity_id.0,
                destination_entity_id.0,
                wave,
                delay_ms
            );
            (self.audio_engine.load(wave), target_position)
        });

        let mut effects: Vec<Box<dyn EffectBase + Send + Sync>> = Vec::new();
        for (slot, effect) in [feedback.hit_effect, feedback.critical_effect].into_iter().enumerate() {
            let Some(effect) = effect else {
                continue;
            };

            if let Some(effect) = self.build_auto_attack_hit_effect(
                effect,
                target_position,
                target_angle,
                destination_entity_id,
                listener_position,
                client_tick,
                slot,
                hit,
            ) {
                effects.push(effect);
            }
        }

        if delay_ms == 0 {
            if let Some((sound, position)) = sound {
                self.audio_engine.play_spatial_sound_effect(sound, position, HIT_SOUND_RANGE);
            }

            for effect in effects {
                self.effect_holder.add_unit(effect, destination_entity_id);
            }
            return;
        }

        self.effect_holder.add_unit(
            Box::new(DelayedHitFeedback::new(
                delay_ms,
                target_position,
                sound,
                Some(self.audio_engine.clone()),
                effects,
            )),
            destination_entity_id,
        );
    }

    /// Builds one official hit effect (`EF_HIT1`, `EF_HITLINE7`,
    /// `EF_VIOLEN_HIT` or `EF_HIT2`) for the defender without adding it to the
    /// holder, so a delayed damage message can spawn it later.
    #[allow(clippy::too_many_arguments)]
    fn build_auto_attack_hit_effect(
        &self,
        effect: EffectId,
        position: Point3<f32>,
        parent_angle: f32,
        owner: EntityId,
        listener_position: Option<Point3<f32>>,
        client_tick: ClientTick,
        slot: usize,
        hit: u8,
    ) -> Option<Box<dyn EffectBase + Send + Sync>> {
        let seed = u64::from(client_tick.0) ^ (u64::from(owner.0) << 32) ^ ((effect as u64) << 8) ^ slot as u64 ^ ((hit as u64) << 40);

        let Some(holder) = PortedEffectHolder::new(effect, seed) else {
            crate::logging::log_debug!("hit feedback: no registered body for {effect:?}");
            return None;
        };

        let mut visual = PortedVisual::new(
            holder,
            self.texture_loader.clone(),
            self.sprite_loader.clone(),
            self.action_loader.clone(),
            self.game_file_loader.clone(),
        )
        .with_audio_engine(self.audio_engine.clone());

        {
            let runtime = visual.runtime_mut();
            runtime.position = [position.x, position.y, position.z];
            // `Hit1` reads `state.parent->render.angle` for the ring/particle
            // facing (`RagEffect.cpp:735-743`).
            runtime.parent_angle = parent_angle;
            runtime.target_offset = runtime.position;
            if let Some(listener) = listener_position {
                runtime.listener_position = [listener.x, listener.y, listener.z];
            }
        }

        crate::logging::log_debug!("hit feedback: prepared {effect:?} for entity {}", owner.0);
        Some(Box::new(visual))
    }
}

/// One damage message the official client queues until `message.time` has
/// elapsed (`CGameActor::ProcessActorEffects`, `RenderObject.cpp:4712-4721`).
///
/// The dual-wield off-hand message is the only user: `ProcessHitMessage` copies
/// the main-hand damage message and re-sends it after
/// [`crate::world::hit_feedback::dual_wield_hit_delay_ms`]
/// (`RenderObject.cpp:3869-3877`). Its impact wave is played and its hit
/// effects start only at that deadline; before it they are neither stepped nor
/// drawn.
pub(crate) struct DelayedHitFeedback {
    remaining_seconds: f32,
    started: bool,
    /// Defender position, used for the holder's distance culling.
    position: Point3<f32>,
    sound: Option<(SoundEffectKey, Point3<f32>)>,
    audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
    effects: Vec<Box<dyn EffectBase + Send + Sync>>,
}

impl DelayedHitFeedback {
    fn new(
        delay_ms: u32,
        position: Point3<f32>,
        sound: Option<(SoundEffectKey, Point3<f32>)>,
        audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
        effects: Vec<Box<dyn EffectBase + Send + Sync>>,
    ) -> Self {
        Self {
            remaining_seconds: delay_ms as f32 / 1000.0,
            started: false,
            position,
            sound,
            audio_engine,
            effects,
        }
    }
}

impl EffectBase for DelayedHitFeedback {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if !self.started {
            self.remaining_seconds -= delta_time;

            if self.remaining_seconds > 0.0 {
                return true;
            }

            self.started = true;

            if let (Some((sound, position)), Some(audio_engine)) = (self.sound.take(), self.audio_engine.as_ref()) {
                audio_engine.play_spatial_sound_effect(sound, position, HIT_SOUND_RANGE);
            }
        }

        self.effects.retain_mut(|effect| effect.update(entities, delta_time));

        !self.effects.is_empty()
    }

    fn mark_for_deletion(&mut self) {
        for effect in &mut self.effects {
            effect.mark_for_deletion();
        }

        self.effects.clear();
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        if !self.started {
            return;
        }

        for effect in &self.effects {
            effect.register_point_lights(point_light_manager, camera);
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if !self.started {
            return;
        }

        for effect in &self.effects {
            effect.render(renderer, camera);
        }
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.position)
    }

    fn apply_caster_state(&mut self, entity: &mut crate::world::Entity, client_tick: ClientTick) {
        if !self.started {
            return;
        }

        for effect in &mut self.effects {
            effect.apply_caster_state(entity, client_tick);
        }
    }

    fn clear_caster_state(&mut self, entity: &mut crate::world::Entity) {
        for effect in &mut self.effects {
            effect.clear_caster_state(entity);
        }
    }
}

/// Deterministic stand-in for the official `std::rand() % 4` wave variant
/// (`HitSounds.cpp:38`, `:62`). Every hit gets a stable variant, so two
/// clients replaying the same packet hear the same wave.
fn hit_feedback_roll(client_tick: ClientTick, source: EntityId, destination: EntityId, hit: u8) -> u8 {
    let mut value = client_tick.0 ^ source.0.rotate_left(13) ^ destination.0.rotate_left(27) ^ u32::from(hit).rotate_left(7);
    value = value.wrapping_mul(0x9E37_79B1);
    value ^= value >> 16;
    (value & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    #[test]
    fn roll_is_deterministic_and_varies_by_hit() {
        let first = hit_feedback_roll(ClientTick(100), EntityId(1), EntityId(2), 0);
        assert_eq!(first, hit_feedback_roll(ClientTick(100), EntityId(1), EntityId(2), 0));

        let variants = (0..16)
            .map(|tick| hit_feedback_roll(ClientTick(tick), EntityId(1), EntityId(2), 0))
            .collect::<std::collections::HashSet<_>>();
        assert!(variants.len() > 4, "the roll must reach all four variants: {variants:?}");
    }

    #[test]
    fn off_hand_roll_differs_from_the_main_hand() {
        // Two damage messages re-roll their wave/effect variance independently.
        assert_ne!(
            hit_feedback_roll(ClientTick(100), EntityId(1), EntityId(2), 1),
            hit_feedback_roll(ClientTick(100), EntityId(1), EntityId(2), 0)
        );
    }

    /// Counts how often the wrapper steps it, so the delay is observable
    /// without a renderer/audio device.
    struct CountingEffect {
        updates: Arc<AtomicUsize>,
    }

    impl EffectBase for CountingEffect {
        fn update(&mut self, _entities: &[crate::world::Entity], _delta_time: f32) -> bool {
            self.updates.fetch_add(1, Ordering::Relaxed);
            true
        }

        fn mark_for_deletion(&mut self) {}

        fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

        fn render(&self, _renderer: &mut EffectRenderer, _camera: &dyn Camera) {}
    }

    #[test]
    fn off_hand_feedback_starts_at_the_official_stagger() {
        let updates = Arc::new(AtomicUsize::new(0));
        let effect = CountingEffect { updates: updates.clone() };
        let mut feedback = DelayedHitFeedback::new(
            crate::world::dual_wield_hit_delay_ms(),
            Point3::new(1.0, 2.0, 3.0),
            None,
            None,
            vec![Box::new(effect)],
        );

        // Still 50 ms short of the 350 ms deadline: the effect is not stepped.
        assert!(feedback.update(&[], 0.3));
        assert_eq!(updates.load(Ordering::Relaxed), 0);

        // Crossing the deadline releases it, and every later frame steps it.
        assert!(feedback.update(&[], 0.06));
        assert_eq!(updates.load(Ordering::Relaxed), 1);
        assert!(feedback.update(&[], 0.016));
        assert_eq!(updates.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn delayed_feedback_is_culled_at_the_defender() {
        let feedback = DelayedHitFeedback::new(350, Point3::new(1.0, 2.0, 3.0), None, None, Vec::new());
        assert_eq!(feedback.culling_position(), Some(Point3::new(1.0, 2.0, 3.0)));
    }
}
