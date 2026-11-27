//! `EF_HATED` (543) - `RagEffect.cpp:21334-21347`.
//!
//! The body is the `EF_HATED` dispatch inline code (a caster `tintPhase` ramp
//! behind flag `0x4000`) plus `CRagEffect::ParticleTime("effect\\white02.bmp",
//! 1)`. The tint logic is reproduced here and the particle half is delegated to
//! the already-ported [`ParticleTime`] body (`rag_batch_b`), so the two C++
//! functions keep their own Rust counterparts.

use super::super::rag_batch_b::ParticleTime;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf`-independent; `0x4000` is the caster `tintPhase` flag
/// (`RenderObjectState.h`).
const HATED_TINT_FLAG: u32 = 0x4000;

#[derive(Debug)]
pub struct Hated {
    lifetime: i32,
    particle_time: ParticleTime,
}

impl Hated {
    pub fn new(lifetime: i32) -> Self {
        Self {
            lifetime,
            particle_time: ParticleTime::new("effect\\white02.bmp", 1, lifetime),
        }
    }
}

impl PortedEffect for Hated {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `if (this->state.stateCnt < 200 &&
        //      (this->state.parent->render.flags & 0x4000) == 0) {`
        //
        // Quirk kept: the actor clears `render.flags` after every rendered
        // frame (`GameObject.cpp:435`), so the flag test is true on the first
        // write of every frame - but it is still part of the condition.
        if runtime.frame < 200 && (runtime.parent_flags & HATED_TINT_FLAG) == 0 {
            runtime.add_parent_flags(HATED_TINT_FLAG);

            // `if (this->state.stateCnt <= 25) tintPhase = stateCnt;`
            if runtime.frame <= 25 {
                runtime.parent_tint_phase = Some(runtime.frame as u8);
            }

            // `if (151 <= stateCnt <= 175) tintPhase = 175 - stateCnt;`
            if (151..=175).contains(&runtime.frame) {
                runtime.parent_tint_phase = Some((175 - runtime.frame) as u8);
            }
        }

        // `this->ParticleTime("effect\\white02.bmp", 1);`
        self.particle_time.on_process(runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn ramps_the_caster_tint_phase_up_to_frame_25() {
        let mut effect = Hated::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.parent_tint_phase, Some(0));
        assert_eq!(runtime.parent_flags, HATED_TINT_FLAG);

        for _ in 0..26 {
            step(&mut effect, &mut runtime);
        }

        // The last write was frame 25.
        assert_eq!(runtime.parent_tint_phase, Some(25));
    }

    #[test]
    fn ramps_back_down_between_151_and_175() {
        let mut effect = Hated::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);
        runtime.frame = 151;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.parent_tint_phase, Some(24));

        runtime.frame = 175;
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.parent_tint_phase, Some(0));
    }

    #[test]
    fn stops_touching_the_caster_after_frame_200() {
        let mut effect = Hated::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);
        runtime.frame = 200;
        runtime.parent_tint_phase = Some(7);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.parent_tint_phase, Some(7));
        assert_eq!(runtime.parent_flags, 0);
    }

    #[test]
    fn delegates_to_particle_time_kind_one() {
        let mut effect = Hated::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        // `ParticleTime` kind 1 launches a prim 59 every fourth frame 0..=80.
        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\pok1.tga"));
        assert_eq!(runtime.primitives[0].state.texture_secondary, Some("effect\\pok3.tga"));
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 1);
    }
}
