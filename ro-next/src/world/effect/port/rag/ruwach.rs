//! `case EF_RUWACH:` - inline body in `CRagEffect::OnProcess`,
//! `RagEffect.cpp:23193-23232`. Lifetime 200 frames (`EffectTypes.cpp:42`).
//!
//! Ruwach is the Acolyte reveal aura. Unlike the other Acolyte support skills
//! it is **not** spawned by the skill-use packet: `GameModePackets.cpp:21615`
//! groups `AL_RUWACH` with the literal `break` skills whose cast switch entry
//! is empty. The body is driven by the caster's option bit instead:
//!
//! * `CGameActor::UpdateStateEffects` re-launches `EF_RUWACH` every
//!   `EffectLifetime(33)` = 200 frames while `IsRuwachActive(stateFlags)`
//!   (`OPTION_RUWACH`, `EffectTypes.h:4084`) is set (`GameActor.cpp:704-714`;
//!   the population-shell twin is `RenderObject.cpp:4697-4705`),
//! * the activation itself is announced with the **Sight** wave:
//!   `PlayWave("effect\\EF_Sight.wav")` in the state-flag change handler
//!   (`CGameActor::SetStatusStates`, `RenderObject.cpp:4851`, condition
//!   `:4867`, wave `:4870`).
//!
//! Every third frame the body drops two `PP_3DPARTICLE` sprites at the point of
//! a circle of radius 15 around the caster: the `particle2.spr` mote and its
//! `Shadow.spr` ground shadow. The angle advances `stateCnt / 1.3 * -5.0`
//! degrees per spawn, integer-truncated before the sine/cosine table lookup,
//! so the pair walks the circle at ~3.85 degrees per third frame.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_RUWACH` lifetime in frames - `EffectTypes.cpp:42`.
pub const LIFETIME: i32 = 200;

/// Radius of the particle ring (`RagEffect.cpp:23199`).
const RADIUS: f32 = 15.0;

/// The two sprites the body alternates (`RagEffect.cpp:23205-23206`,
/// `:23221-23222`).
const MOTE_SPRITE: &str = "이팩트\\particle2.spr";
const MOTE_ACTION: &str = "이팩트\\particle2.act";
const SHADOW_SPRITE: &str = "Shadow.spr";
const SHADOW_ACTION: &str = "Shadow.act";

#[derive(Debug, Default)]
pub struct Ruwach;

impl PortedEffect for Ruwach {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = this->state.parent->render.position;` runs
        // on every frame, before the spawn gate.
        runtime.copy_parent_position();

        if runtime.frame % 3 != 0 {
            return true;
        }

        let angles = Angles::shared();
        // C++: `static_cast<int>(this->state.stateCnt / 1.3 * -5.0)`. The two
        // literals are `double`, so the division is double precision and the
        // cast truncates towards zero.
        let angle = (f64::from(runtime.frame) / 1.3 * -5.0) as i32;
        // QUIRK: the original calls `GetRadian(angle)` here and discards the
        // result (`RagEffect.cpp:23198`). It is a pure table lookup, so the
        // call has no observable effect and is not reproduced.
        let sine = angles.sine(angle) * RADIUS;
        let cosine = angles.cosine(angle) * -RADIUS;

        // `particle2.spr` mote. Both layers reuse the same scaled sine/cosine
        // because the original mutates the locals before spawning the second.
        let mote = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(mote);
            state.duration = 25;
            state.delta_pos2 = [sine, -10.0, cosine];
            state.sprite = Some(MOTE_SPRITE);
            state.action = Some(MOTE_ACTION);
            state.size = 3.0;
            state.size_speed = -0.1;
            state.alpha = 250.0;
            state.alpha_speed = -3.0;
            state.fade_out_cnt = state.duration - 6;
            state.init_segment();
        }

        // `Shadow.spr` ground sticker under the mote.
        let shadow = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(shadow);
            state.duration = 25;
            state.delta_pos2 = [sine, 0.0, cosine];
            state.sprite = Some(SHADOW_SPRITE);
            state.action = Some(SHADOW_ACTION);
            state.anim_speed = 2;
            state.size = 1.5;
            state.size_speed = -0.06;
            state.alpha = 150.0;
            state.alpha_speed = -2.5;
            state.fade_out_cnt = state.duration - 6;
            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime() -> EffectRuntime {
        EffectRuntime::new(LIFETIME, [0.0, 0.0], 33)
    }

    #[test]
    fn every_third_frame_spawns_the_mote_and_its_shadow() {
        let mut effect = Ruwach;
        let mut runtime = runtime();

        // Drive `on_process` directly: `step` also runs the primitive pass,
        // which ages `size`/`alpha` by one frame, and these assertions are the
        // spawn literals of `RagEffect.cpp:23201-23231`.
        effect.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        // Frames 1 and 2 spawn nothing.
        for frame in 1..=2 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
            assert_eq!(runtime.primitives.len(), 2, "frame {frame} spawns nothing");
        }

        // Frame 3 spawns the next pair.
        runtime.frame = 3;
        effect.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), 4);

        // Frame 0 is angle 0: sine 0, cosine -15.
        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::Particle3d);
        assert_eq!(mote.duration, 25);
        assert_eq!(mote.delta_pos2, [0.0, -10.0, -15.0]);
        assert_eq!(mote.sprite, Some(MOTE_SPRITE));
        assert_eq!(mote.action, Some(MOTE_ACTION));
        assert_eq!(mote.size, 3.0);
        assert_eq!(mote.size_speed, -0.1);
        assert_eq!(mote.alpha, 250.0);
        assert_eq!(mote.alpha_speed, -3.0);
        assert_eq!(mote.fade_out_cnt, 19);
        assert_eq!(mote.anim_speed, 4, "the mote keeps the constructor's animSpeed");

        let shadow = &runtime.primitives[1].state;
        assert_eq!(shadow.count, PrimitiveId::Particle3d);
        assert_eq!(shadow.duration, 25);
        assert_eq!(shadow.delta_pos2, [0.0, 0.0, -15.0]);
        assert_eq!(shadow.sprite, Some(SHADOW_SPRITE));
        assert_eq!(shadow.action, Some(SHADOW_ACTION));
        assert_eq!(shadow.anim_speed, 2);
        assert_eq!(shadow.size, 1.5);
        assert_eq!(shadow.size_speed, -0.06);
        assert_eq!(shadow.alpha, 150.0);
        assert_eq!(shadow.alpha_speed, -2.5);
        assert_eq!(shadow.fade_out_cnt, 19);
    }

    #[test]
    fn the_ring_walks_the_integer_truncated_angle_table() {
        let mut effect = Ruwach;
        let mut runtime = runtime();
        runtime.frame = 3;

        effect.on_process(&mut runtime);

        let angles = Angles::shared();
        let sine = angles.sine(-11) * RADIUS;
        let cosine = angles.cosine(-11) * -RADIUS;

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.delta_pos2, [sine, -10.0, cosine]);

        // The second layer reuses the same (already scaled) pair.
        let shadow = &runtime.primitives[1].state;
        assert_eq!(shadow.delta_pos2, [sine, 0.0, cosine]);
    }

    #[test]
    fn the_body_lives_its_official_two_hundred_frames() {
        let mut effect = Ruwach;
        let mut runtime = runtime();

        // `step` retires once the frame counter reaches `effectLifetime`, so
        // the first `LIFETIME - 1` frames stay alive and frame 199 ends it.
        for frame in 0..LIFETIME - 1 {
            assert!(step(&mut effect, &mut runtime), "retired before frame {frame}");
        }

        assert!(!step(&mut effect, &mut runtime), "must retire at the official lifetime");
        // Frames 0, 3, ..., 198 spawn once each: 67 pairs.
        assert_eq!(runtime.primitives.len(), 67 * 2);
    }

    #[test]
    fn the_sound_belongs_to_the_activation_not_to_the_body() {
        // `EF_RUWACH` plays no wave; `effect\EF_Sight.wav` is emitted by the
        // option-change handler (`RenderObject.cpp:4874`), which the client
        // models with the curated `skill_sounds.ron` row.
        let mut effect = Ruwach;
        let mut runtime = runtime();

        for _ in 0..LIFETIME {
            step(&mut effect, &mut runtime);
        }

        assert!(runtime.sounds.is_empty());
    }
}
