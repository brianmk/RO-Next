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
//!   `EF_HITLINE7` for the Taekwon/Star job family, and `EF_HIT2` (`1`) on a
//!   critical hit as well (`RenderObject.cpp:3769-3822`,
//!   `GameObject.cpp:1843-1888`);
//! * a miss (`damage == 0`) plays/spawns neither (`GameObject.cpp:1768`).
//!
//! The decision itself is data-driven and lives in
//! [`crate::world::hit_feedback::auto_attack_hit_feedback`]; this module only
//! resolves the entity facts and forwards the result to the existing audio
//! engine and the ported-effect holder (the same `AudioEngine::load` +
//! `play_spatial_sound_effect` / `PortedEffectHolder` path the descriptor and
//! ported skill effects use). One wave is played per damage event, and the
//! arrow projectile no longer plays `_hit_arrow.wav` on landing
//! (`world/effect/arrow_effect.rs`), so a bow auto attack is heard exactly
//! once.

use ragnarok_packets::EffectId;

use crate::world::port::holder::PortedEffectHolder;
use crate::world::port::visual::PortedVisual;
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
    /// Exactly one wave is loaded/played per hit.
    pub(crate) fn play_auto_attack_hit_feedback(
        &mut self,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        damage: bool,
        is_critical: bool,
        client_tick: ClientTick,
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

        let Some(defender) = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == destination_entity_id)
            .map(|entity| {
                let job_id = entity.get_job_id();
                HitDefender {
                    entity_id: destination_entity_id,
                    job_id,
                    is_player: EntityType::from(job_id) == EntityType::Player,
                }
            })
        else {
            crate::logging::log_debug!(
                "hit feedback: defender {} is not tracked, skipping the impact wave/effect",
                destination_entity_id.0
            );
            return;
        };

        let roll = hit_feedback_roll(client_tick, source_entity_id, destination_entity_id);
        let feedback = auto_attack_hit_feedback(attacker, defender, is_critical, damage, roll);

        let Some(target_position) = self.entity_position(destination_entity_id) else {
            return;
        };
        let target_angle = self.entity_parent_angle(destination_entity_id);
        let listener_position = self.client_state.try_follow(this_entity()).map(|entity| entity.get_position());

        if let Some(wave) = feedback.sound {
            crate::logging::log_debug!(
                "hit feedback: auto attack {} -> {} plays '{}'",
                source_entity_id.0,
                destination_entity_id.0,
                wave
            );
            let sound_effect = self.audio_engine.load(wave);
            self.audio_engine
                .play_spatial_sound_effect(sound_effect, target_position, HIT_SOUND_RANGE);
        }

        for (index, effect) in [feedback.hit_effect, feedback.critical_effect].into_iter().enumerate() {
            let Some(effect) = effect else {
                continue;
            };

            self.spawn_auto_attack_hit_effect(
                effect,
                target_position,
                target_angle,
                destination_entity_id,
                listener_position,
                client_tick,
                index,
            );
        }
    }

    /// Spawns one official hit effect (`EF_HIT1`, `EF_HITLINE7` or `EF_HIT2`)
    /// on the defender through the ported-effect holder.
    #[allow(clippy::too_many_arguments)]
    fn spawn_auto_attack_hit_effect(
        &mut self,
        effect: EffectId,
        position: Point3<f32>,
        parent_angle: f32,
        owner: EntityId,
        listener_position: Option<Point3<f32>>,
        client_tick: ClientTick,
        slot: usize,
    ) {
        let seed = u64::from(client_tick.0) ^ (u64::from(owner.0) << 32) ^ ((effect as u64) << 8) ^ slot as u64;

        let Some(holder) = PortedEffectHolder::new(effect, seed) else {
            crate::logging::log_debug!("hit feedback: no registered body for {effect:?}");
            return;
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

        self.effect_holder.add_unit(Box::new(visual), owner);
        crate::logging::log_debug!("hit feedback: spawned {effect:?} on entity {}", owner.0);
    }
}

/// Deterministic stand-in for the official `std::rand() % 4` wave variant
/// (`HitSounds.cpp:38`, `:62`). Every hit gets a stable variant, so two
/// clients replaying the same packet hear the same wave.
fn hit_feedback_roll(client_tick: ClientTick, source: EntityId, destination: EntityId) -> u8 {
    let mut value = client_tick.0 ^ source.0.rotate_left(13) ^ destination.0.rotate_left(27);
    value = value.wrapping_mul(0x9E37_79B1);
    value ^= value >> 16;
    (value & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_is_deterministic_and_varies_by_hit() {
        let first = hit_feedback_roll(ClientTick(100), EntityId(1), EntityId(2));
        assert_eq!(first, hit_feedback_roll(ClientTick(100), EntityId(1), EntityId(2)));

        let variants = (0..16)
            .map(|tick| hit_feedback_roll(ClientTick(tick), EntityId(1), EntityId(2)))
            .collect::<std::collections::HashSet<_>>();
        assert!(variants.len() > 4, "the roll must reach all four variants: {variants:?}");
    }
}
