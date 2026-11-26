//! `EF_CHEMICAL_V` / `EF_CHEMICAL_V2` dispatch - `RagEffect.cpp:19764-19772`.
//!
//! Both blocks call one [`TexturedBeam`] and two `TexturedForkBolt`s in that
//! order, with the beam flag carrying the variant:
//!
//! * `EF_CHEMICAL_V`  - `TexturedBeam(0, "effect\\line3.tga")`, then
//!   `TexturedForkBolt("effect\\melody_a.bmp", 0)` and
//!   `TexturedForkBolt("effect\\melody_b.bmp", 0)`,
//! * `EF_CHEMICAL_V2` - the same with flag `1`.
//!
//! `EF_CHEMICAL_V3` is a single `TexturedBeam(2, "effect\\line3.tga")`, so it
//! registers [`TexturedBeam`] directly.
//!
//! `EF_NEW_METALLICSOUND_LINE` (official id 1584) is the third fork-bolt
//! caller: `RagEffect.cpp:22532-22544` calls one `TexturedBeam(2, ...)` and
//! three `TexturedForkBolt(..., 2)`s. That case branches on the
//! `TT_MIN_EFFECT_ON_OFF` service option; the option defaults to "off"
//! (`ServiceOptions::Get` returns `false` for an unset id,
//! `ServiceOptions.cpp:499-501`), so [`MetallicSoundLine`] ports the `else`
//! branch (`effect\new_metalicsound\...`) and the reduced `effect\mineffect\
//! ...` asset set is not modelled.

use super::super::rag_batch_a::TexturedForkBolt;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::textured_beam::TexturedBeam;

/// The two `TexturedForkBolt` calls of one chemical line, in dispatch order.
#[derive(Debug)]
pub struct ChemicalLine {
    beam: TexturedBeam,
    forks: [TexturedForkBolt; 2],
    lifetime: i32,
}

impl ChemicalLine {
    pub fn new(flag: i8, lifetime: i32) -> Self {
        Self {
            beam: TexturedBeam::new(flag, "effect\\line3.tga", lifetime),
            forks: [
                TexturedForkBolt::new("effect\\melody_a.bmp", flag, lifetime),
                TexturedForkBolt::new("effect\\melody_b.bmp", flag, lifetime),
            ],
            lifetime,
        }
    }
}

/// `EF_NEW_METALLICSOUND_LINE` - `RagEffect.cpp:22532-22544`, default
/// (`TT_MIN_EFFECT_ON_OFF == 0`) branch.
///
/// One [`TexturedBeam`] plus three [`TexturedForkBolt`]s, all with flag `2`,
/// lifetime 300 (`EffectTypes.cpp:1593`). The `mineffect` alternate paths are a
/// server-side service option and are documented above.
#[derive(Debug)]
pub struct MetallicSoundLine {
    beam: TexturedBeam,
    forks: [TexturedForkBolt; 3],
    lifetime: i32,
}

impl MetallicSoundLine {
    pub fn new(lifetime: i32) -> Self {
        Self {
            beam: TexturedBeam::new(
                2,
                "effect\\new_metalicsound\\new_metalicsound_line\\met_damege_line_1.tga",
                lifetime,
            ),
            forks: [
                TexturedForkBolt::new(
                    "effect\\new_metalicsound\\new_metalicsound_icon\\met_particle_1.tga",
                    2,
                    lifetime,
                ),
                TexturedForkBolt::new(
                    "effect\\new_metalicsound\\new_metalicsound_icon\\met_particle_2.tga",
                    2,
                    lifetime,
                ),
                TexturedForkBolt::new(
                    "effect\\new_metalicsound\\new_metalicsound_icon\\met_particle_3.tga",
                    2,
                    lifetime,
                ),
            ],
            lifetime,
        }
    }
}

impl PortedEffect for MetallicSoundLine {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // The official case runs the four calls in this exact order every
        // frame; each body gates its own emission.
        self.beam.on_process(runtime);

        for fork in &mut self.forks {
            fork.on_process(runtime);
        }

        true
    }
}

impl PortedEffect for ChemicalLine {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // The dispatch block runs the three calls in this exact order every
        // frame; each body decides on which frames it emits.
        self.beam.on_process(runtime);
        self.forks[0].on_process(runtime);
        self.forks[1].on_process(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);
        runtime.target_offset = [0.0, 0.0, 10.0];
        runtime
    }

    #[test]
    fn frame_zero_spawns_the_beam_then_both_fork_bolts() {
        let mut effect = ChemicalLine::new(1, 300);
        let mut runtime = runtime();

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 3);
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::FlashFade);
        assert_eq!(runtime.primitives[1].state.count, PrimitiveId::WanderingFlames);
        assert_eq!(runtime.primitives[2].state.count, PrimitiveId::WanderingFlames);

        // Beam flag -> slot flag1[0].
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[0], 1);
        // Fork flag -> slot flag1[0] (and the two melody textures).
        assert_eq!(runtime.primitives[1].state.slots[0].flag1[0], 1);
        assert_eq!(runtime.primitives[2].state.slots[0].flag1[0], 1);
        assert_eq!(runtime.primitives[1].state.texture, Some("effect\\melody_a.bmp"));
        assert_eq!(runtime.primitives[2].state.texture, Some("effect\\melody_b.bmp"));
    }

    #[test]
    fn fork_bolts_repeat_on_frames_ten_and_twenty() {
        let mut effect = ChemicalLine::new(0, 300);
        let mut runtime = runtime();

        for frame in 0..=20 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        // One beam + three emissions per fork body.
        assert_eq!(runtime.primitives.len(), 7);

        let beams = runtime
            .primitives
            .iter()
            .filter(|primitive| primitive.state.count == PrimitiveId::FlashFade)
            .count();
        assert_eq!(beams, 1);
    }

    #[test]
    fn v_one_and_v_two_only_differ_in_the_flag() {
        let mut first = ChemicalLine::new(0, 300);
        let mut second = ChemicalLine::new(1, 300);
        let mut left = runtime();
        let mut right = runtime();

        first.on_process(&mut left);
        second.on_process(&mut right);

        for index in 0..3 {
            assert_eq!(left.primitives[index].state.count, right.primitives[index].state.count);
        }

        assert_eq!(left.primitives[0].state.slots[0].flag1[0], 0);
        assert_eq!(right.primitives[0].state.slots[0].flag1[0], 1);
    }

    #[test]
    fn chemical_lifetime_is_300() {
        assert_eq!(ChemicalLine::new(0, 300).lifetime(), 300);
        assert_eq!(TexturedBeam::new(2, "effect\\line3.tga", 300).lifetime(), 300);
    }

    #[test]
    fn metallic_sound_line_spawns_the_beam_then_three_fork_bolts() {
        let mut effect = MetallicSoundLine::new(300);
        let mut runtime = runtime();

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 4);
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::FlashFade);
        assert_eq!(runtime.primitives[1].state.count, PrimitiveId::WanderingFlames);
        assert_eq!(runtime.primitives[2].state.count, PrimitiveId::WanderingFlames);
        assert_eq!(runtime.primitives[3].state.count, PrimitiveId::WanderingFlames);

        assert_eq!(
            runtime.primitives[0].state.texture,
            Some("effect\\new_metalicsound\\new_metalicsound_line\\met_damege_line_1.tga")
        );

        // Every call uses flag 2 (`TexturedBeam`/`TexturedForkBolt`).
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[0], 2);

        for (index, texture) in [
            "effect\\new_metalicsound\\new_metalicsound_icon\\met_particle_1.tga",
            "effect\\new_metalicsound\\new_metalicsound_icon\\met_particle_2.tga",
            "effect\\new_metalicsound\\new_metalicsound_icon\\met_particle_3.tga",
        ]
        .into_iter()
        .enumerate()
        {
            let primitive = &runtime.primitives[1 + index].state;
            assert_eq!(primitive.texture, Some(texture));
            assert_eq!(primitive.slots[0].flag1[0], 2);
        }
    }

    #[test]
    fn metallic_sound_line_lifetime_is_300_and_the_forks_repeat() {
        assert_eq!(MetallicSoundLine::new(300).lifetime(), 300);

        let mut effect = MetallicSoundLine::new(300);
        let mut runtime = runtime();

        for frame in 0..=20 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        // One beam + three emissions per fork body.
        assert_eq!(runtime.primitives.len(), 10);
    }
}
