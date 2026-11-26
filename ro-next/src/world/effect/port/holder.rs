//! `PortedEffectHolder` - owns one ported effect body and steps it on the fixed
//! 16 ms clock of the official client.
//!
//! This is the object the client will keep in its effect list once the ported
//! backend is wired (plan 4.2). It deliberately separates the *simulation*
//! (which is exact and testable) from the *rendering* (which needs textures and
//! the effect renderer): `update` only advances the timeline, so the timing
//! model can be verified without a GPU.

use ragnarok_packets::EffectId;

use super::FRAME_FREQUENCY_MS;
use super::registry;
use super::runtime::{EffectRuntime, PortedEffect};

pub struct PortedEffectHolder {
    body: Box<dyn PortedEffect>,
    runtime: EffectRuntime,
    /// Milliseconds not yet consumed by a 16 ms step.
    accumulator_ms: f64,
    finished: bool,
}

impl PortedEffectHolder {
    /// Creates the holder for a registered effect id.
    pub fn new(effect: EffectId, seed: u64) -> Option<Self> {
        let body = registry::spawn(effect)?;
        let lifetime = body.lifetime();
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], seed);
        let _ = &mut runtime;

        Some(Self {
            body,
            runtime,
            accumulator_ms: 0.0,
            finished: false,
        })
    }

    pub fn runtime(&self) -> &EffectRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut EffectRuntime {
        &mut self.runtime
    }

    pub fn is_finished(&self) -> bool {
        self.finished
    }

    /// Texture assets the effect references; the loader prefetches these.
    pub fn textures(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.runtime
            .primitives
            .iter()
            .filter_map(|primitive| primitive.state.texture)
    }

    /// Advances the effect by `delta_seconds` of wall time.
    ///
    /// The official client runs effects on a **fixed 16 ms** step, so a frame of
    /// 50 ms produces three steps and a 5 ms frame produces none. Returns
    /// `false` once the effect has finished (and keeps returning `false`).
    pub fn update(&mut self, delta_seconds: f64) -> bool {
        if self.finished {
            return false;
        }

        self.accumulator_ms += delta_seconds * 1000.0;

        while self.accumulator_ms >= FRAME_FREQUENCY_MS as f64 {
            self.accumulator_ms -= FRAME_FREQUENCY_MS as f64;

            if !super::runtime::step(self.body.as_mut(), &mut self.runtime) {
                self.finished = true;
                return false;
            }
        }

        true
    }

    /// Number of 16 ms steps still pending in the accumulator (test hook).
    pub fn pending_steps(&self) -> u32 {
        (self.accumulator_ms / FRAME_FREQUENCY_MS as f64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_runs_exactly_one_hundred_sixty_milliseconds_per_ten_frames() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        // 10 frames at 60 Hz = 166.7 ms -> 10 steps of 16 ms.
        let mut steps = 0;
        for _ in 0..10 {
            holder.update(1.0 / 60.0);
            steps = holder.runtime().frame;
        }

        assert_eq!(steps, 10);
        assert_eq!(holder.runtime().primitives.len(), 21);
    }

    #[test]
    fn catch_up_produces_three_steps_for_a_fifty_ms_frame() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        holder.update(0.050);

        assert_eq!(holder.runtime().frame, 3);
    }

    #[test]
    fn five_ms_frames_do_not_step_until_sixteen_accumulate() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        holder.update(0.005);
        assert_eq!(holder.runtime().frame, 0);

        holder.update(0.005);
        holder.update(0.005);
        assert_eq!(holder.runtime().frame, 0);

        holder.update(0.005);
        assert_eq!(holder.runtime().frame, 1);
    }

    #[test]
    fn effect_finishes_after_its_lifetime() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();

        // 40 frames * 16 ms = 640 ms.
        let mut elapsed = 0.0;
        while holder.update(0.016) {
            elapsed += 0.016;
            if elapsed > 5.0 {
                panic!("effect did not finish");
            }
        }

        assert!(holder.is_finished());
        assert_eq!(holder.runtime().frame, 40);
        assert!(!holder.update(0.016), "stays finished");
    }

    #[test]
    fn textures_lists_the_assets_of_live_primitives() {
        let mut holder = PortedEffectHolder::new(EffectId::Bash, 1).unwrap();
        holder.update(0.016);

        let textures: Vec<_> = holder.textures().collect();

        assert!(textures.contains(&"effect\\alpha_down.tga"));
        assert!(textures.contains(&"EFFECT\\alpha_center.tga"));
    }

    #[test]
    fn unported_effect_has_no_holder() {
        assert!(PortedEffectHolder::new(EffectId::Fireball, 1).is_none());
    }
}
