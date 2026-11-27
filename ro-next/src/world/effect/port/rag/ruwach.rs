//! `case EF_RUWACH:` - inline body in `CRagEffect::OnProcess`,
//! `RagEffect.cpp:23193-23233`. Lifetime 200 frames (`EffectTypes.cpp:42`).
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
    use ragnarok_packets::SkillId;

    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};
    use crate::world::{
        CasterAction, SkillPhase, cast_frame_sequence_for_skill, caster_action_for_skill, curated_skill_sound_files,
        frame_sequence_for_skill, skill_effect_layers, skill_has_effect, use_caster_action_for_skill,
    };

    fn runtime() -> EffectRuntime {
        EffectRuntime::new(LIFETIME, [0.0, 0.0], 33)
    }

    /// The official cast sound of the reveal aura is `effect\EF_Sight.wav`,
    /// played when `OPTION_RUWACH` becomes active
    /// (`CGameActor::SetStatusStates`, `RenderObject.cpp:4867-4871`).
    ///
    /// The `EF_RUWACH` body plays no wave of its own
    /// (`RagEffect.cpp:23193-23233`) and the official skill-use switch has an
    /// empty `case AL_RUWACH: break;` (`GameModePackets.cpp:21615`), so the
    /// 2019 effect-table `ef_ruwach` and the generic `ef_hit2` candidates must
    /// never win.
    #[test]
    fn the_official_sound_is_the_option_activation_sight_wave() {
        assert_eq!(
            curated_skill_sound_files(SkillId(24)).map(|files| files.to_vec()),
            Some(vec!["effect\\ef_sight.wav"]),
            "AL_RUWACH (24) must play the official EF_Sight wave"
        );
        // The generated table has no row for 24, so the curated one is the
        // only source and `ef_hit2` cannot resurface.
        assert!(
            crate::world::skill_sound_files(SkillId(24)).is_none(),
            "the 2019 generic `ef_hit2` row must not come back"
        );
    }

    /// `AL_RUWACH` never overrides the default caster motion:
    /// `GetSkillActionInfo` leaves `motion = 8` (`SkillActionInfo.cpp:51-53`,
    /// `:216`) and `GetSkillActionInfo2` leaves `motion = 7` (`:1228-1230`,
    /// `:1312`). `CPc::JobActionBase` resolves both `ST_BEGIN_SKILL` and
    /// `ST_SKILL` to the skill block 96 (`Player.cpp:1671-1746`), i.e.
    /// `CasterAction::Skill`; neither path attaches a `SetFrameSequence`.
    #[test]
    fn the_official_caster_motion_is_the_begin_skill_gesture() {
        assert_eq!(caster_action_for_skill(SkillId(24)), Some(CasterAction::Skill));
        // The two official paths agree, so `skill_actions.ron` has no
        // `use_action` row for the skill.
        assert_eq!(use_caster_action_for_skill(SkillId(24)), None);
        assert!(frame_sequence_for_skill(SkillId(24)).is_none());
        assert!(cast_frame_sequence_for_skill(SkillId(24)).is_none());
    }

    /// The 2019 table's only entry for `AL_RUWACH` is `hitEffectId 1` (the
    /// generic `lens1`/`lens2` hit flash). It is unreachable in the 2026
    /// client: the skill-use switch is a literal `case AL_RUWACH: break;`
    /// (`GameModePackets.cpp:21615`) and the real visual is the `EF_RUWACH`
    /// option aura ported here. The empty `(skill: 24, phase: Hit)` override in
    /// `tools/data/skill_effects_overrides.ron` keeps the bogus row out, so no
    /// phase may describe a Ruwach visual.
    #[test]
    fn the_skill_has_no_descriptor_row() {
        for phase in [
            SkillPhase::Ground,
            SkillPhase::Cast,
            SkillPhase::Success,
            SkillPhase::Release,
            SkillPhase::Hit,
            SkillPhase::Caster,
        ] {
            assert!(
                skill_effect_layers(SkillId(24), phase).is_empty(),
                "AL_RUWACH must not get a {phase:?} descriptor; the aura is the ported EF_RUWACH"
            );
        }
        assert!(!skill_has_effect(SkillId(24)));
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

    /// The two sprite heights, in the port's Y-up world.
    ///
    /// `RagEffect.cpp:23205-23231` spawns the mote at `deltaPos2.y = -10.0f`
    /// and its ground shadow at `0.0f`. The official client's world has Y
    /// pointing **down** (the camera frame clamp `CameraFrame.cpp:36-41`,
    /// `Upward`'s `{0, -distance, 0}` in `EffectPrimitiveMotion.cpp:84-88`, and
    /// the "fell through the floor" test `render.position.y > Height(x, z)` at
    /// `:2598`), so `-10` means *ten units above the caster's feet*. The port
    /// renders a Y-up world (the map loader negates the GAT heights,
    /// `loaders/map/vertices.rs:315-318`) and flips the axis once at render
    /// time, so a caster standing at height `H` must show the mote at
    /// `H + 10` and the shadow at `H`. Without the flip the mote ends up ten
    /// units *under* the terrain and the ground occludes it - the reported
    /// "Ruwach looks like a shadow".
    #[test]
    fn the_mote_is_ten_units_above_the_caster_and_the_shadow_on_the_ground() {
        use crate::world::effect::port::render3d::world_space_position;

        let mut effect = Ruwach;
        let mut runtime = runtime();
        effect.on_process(&mut runtime);

        let mote = &runtime.primitives[0].state;
        let shadow = &runtime.primitives[1].state;

        // The body keeps the official literals.
        assert_eq!(mote.delta_pos2[1], -10.0);
        assert_eq!(shadow.delta_pos2[1], 0.0);

        // A caster standing at port height `H` is `-H` in the official
        // convention the simulation runs in; the renderer flips the result.
        let ground = 12.0f32;
        let official_anchor = [0.0, -ground, 0.0];
        let mote_world = world_space_position([
            official_anchor[0] + mote.delta_pos2[0],
            official_anchor[1] + mote.delta_pos2[1],
            official_anchor[2] + mote.delta_pos2[2],
        ]);
        let shadow_world = world_space_position([
            official_anchor[0] + shadow.delta_pos2[0],
            official_anchor[1] + shadow.delta_pos2[1],
            official_anchor[2] + shadow.delta_pos2[2],
        ]);

        assert!((mote_world.y - (ground + 10.0)).abs() < 1e-4);
        assert!((shadow_world.y - ground).abs() < 1e-4);
        assert!(mote_world.y > shadow_world.y, "the mote floats above its shadow");
    }

    #[test]
    fn the_ring_follows_the_caster_every_frame() {
        // `this->render.position = this->state.parent->render.position;`
        // (`RagEffect.cpp:23194`) runs before the spawn gate. `Prim3DParticle`
        // resolves `org_pos = master.position + deltaPos2` on every primitive
        // step (`EffectPrimitiveMotion.cpp:2490`), so the particles already in
        // flight move with the caster instead of staying on the spawn cell.
        let mut effect = Ruwach;
        let mut runtime = runtime();
        runtime.parent_position = Some([100.0, 5.0, -50.0]);

        assert!(step(&mut effect, &mut runtime));
        assert_eq!(runtime.position, [100.0, 5.0, -50.0]);
        assert_eq!(runtime.primitives.len(), 2);
        // Primitive state is official (Y-down): the caster's port y 5 is -5
        // and the mote's `deltaPos2.y = -10` is ten units *up*.
        assert_eq!(runtime.primitives[0].state.position, [100.0, -15.0, -65.0]);

        runtime.parent_position = Some([110.0, 5.0, -50.0]);
        assert!(step(&mut effect, &mut runtime));

        // Same primitives, new anchor: the frame-0 mote (x 0, y -10, z -15)
        // is now relative to the moved caster.
        assert_eq!(runtime.position, [110.0, 5.0, -50.0]);
        assert_eq!(runtime.primitives[0].state.position, [110.0, -15.0, -65.0]);
        assert_eq!(runtime.primitives[1].state.position, [110.0, -5.0, -65.0]);
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
