//! The `CRagEffect` Storm Kick bodies -
//! `RagEffect.cpp:23663-23677` (`EF_STORMKICK`), `:23678-23688`
//! (`EF_STORMKICK1`), `:23689-23697` (`EF_STORMKICK2`), `:23698-23702`
//! (`EF_STORMKICK3`), `:23703-23711` (`EF_STORMKICK4`), `:23712-23722`
//! (`EF_STORMKICK5`), `:23723-23731` (`EF_STORMKICK6`) and `:23732-23736`
//! (`EF_STORMKICK7`).
//!
//! The first four and the last two blocks are the same three-call recipe: one
//! [`TexturedMark`] with the id-specific `kind`, then `Cloud11Burst(2)` and
//! `Cloud11Burst(3)`. Two ids add a wave (`EF_STORMKICK`/`EF_STORMKICK2` at
//! `stateCnt == 5`, `EF_STORMKICK6` at `stateCnt == 0`) and two rotate the
//! caster (`EF_STORMKICK` by 30 degrees for `stateCnt < 48`, `EF_STORMKICK1`
//! by 14 for `stateCnt <= 50`).
//!
//! `EF_STORMKICK4` and `EF_STORMKICK5` are the white-light-trail variants: each
//! calls `WhiteLightTrail("effect\\whitelight.tga", 1)` thirty times per frame
//! (the helper emits only on frame 0) and plays `effect\T_회피.wav` at frame
//! 15. Id 5 additionally sets the caster's `render.state = 37`
//! (`ST_NINJAREADY`) and `SetFrameSequence(0x60, 2, 2, 0x60, 2, 1, 25, 0)` at
//! that frame; the port records both requests on the runtime so the client can
//! apply the caster animation (`EffectRuntime::parent_state`/
//! `parent_frame_sequence`).
//!
//! ## Parent rotation
//!
//! `this->state.parent->render.angle += N` rotates the caster actor in place.
//! The port's [`EffectRuntime::parent_angle`] is the effect's copy of that
//! facing (seeded by the client from the entity's direction), and the bodies
//! mutate it exactly like the original, including the `> 360.0f` (not `>=`)
//! single subtraction. The client can mirror the post-step value back onto the
//! caster actor; see the field docs.
//!
//! ## Scope
//!
//! These are the `CRagEffect` dispatch blocks, i.e. the bodies that
//! `EffectActor::LaunchEffect` creates for these ids
//! (`PartyMemberEffects.cpp:112-166`). The separate `CStormKickEffect`
//! particle implementation (`StormKickEffect.cpp`), which `EffectLists`
//! registers only for `EF_STORMKICK` (435) and `EF_STORMKICK8` (1223), is not
//! ported here.

use ragnarok_packets::EffectId;

use super::super::runtime::{EffectRuntime, ParentFrameSequence, PortedEffect};
use super::WhiteLightTrail;
use super::cloud11::Cloud11Burst;
use super::textured_mark::TexturedMark;

/// `effect\storm2.tga` - the texture of every `TexturedMark` call.
const STORM2: &str = "effect\\storm2.tga";
/// `effect\whitelight.tga` - the texture of every `WhiteLightTrail` call.
const WHITE_LIGHT: &str = "effect\\whitelight.tga";

/// One of the six `CRagEffect` Storm Kick bodies.
#[derive(Debug)]
pub struct StormKick {
    id: EffectId,
}

impl StormKick {
    pub fn new(id: EffectId) -> Self {
        Self { id }
    }
}

/// `TexturedMark` followed by the two `Cloud11Burst` layers, in dispatch order.
fn marks_and_clouds(runtime: &mut EffectRuntime, kind: i8, lifetime: i32) {
    TexturedMark::new(STORM2, kind, lifetime).on_process(runtime);
    Cloud11Burst::new(2, lifetime).on_process(runtime);
    Cloud11Burst::new(3, lifetime).on_process(runtime);
}

/// `this->state.parent->render.angle += delta;`
/// `if (parent.angle > 360.0f) parent.angle -= 360.0f;`
///
/// QUIRK: the comparison is `>`, so an angle that lands exactly on 360 is kept
/// until the next increment.
fn spin_parent(runtime: &mut EffectRuntime, delta: f32) {
    runtime.parent_angle += delta;

    if runtime.parent_angle > 360.0 {
        runtime.parent_angle -= 360.0;
    }
}

impl PortedEffect for StormKick {
    /// `EffectTypes.cpp:444,468-474`: 100 for ids 0/1/2/3/6, 200 for id 4, 300
    /// for id 5 and 1000 for id 7.
    fn lifetime(&self) -> i32 {
        match self.id {
            EffectId::Stormkick4 => 200,
            EffectId::Stormkick5 => 300,
            EffectId::Stormkick7 => 1000,
            _ => 100,
        }
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let lifetime = self.lifetime();

        match self.id {
            // `RagEffect.cpp:23663-23677`
            EffectId::Stormkick => {
                if runtime.frame == 5 {
                    // `this->render.position = this->state.parent->render.position;`
                    // (the runtime anchor already is the parent position)
                    runtime.play_sound("effect\\T_회오리차기.wav");
                }

                marks_and_clouds(runtime, 0, lifetime);

                // `if (stateCnt < 48)`
                if runtime.frame < 48 {
                    spin_parent(runtime, 30.0);
                }
            }
            // `RagEffect.cpp:23678-23688`
            EffectId::Stormkick1 => {
                marks_and_clouds(runtime, 1, lifetime);

                // `if (stateCnt <= 50)`
                if runtime.frame <= 50 {
                    spin_parent(runtime, 14.0);
                }
            }
            // `RagEffect.cpp:23689-23697`
            EffectId::Stormkick2 => {
                if runtime.frame == 5 {
                    runtime.play_sound("ice_titan_attack.wav");
                }

                marks_and_clouds(runtime, 6, lifetime);
            }
            // `RagEffect.cpp:23698-23702`
            EffectId::Stormkick3 => {
                marks_and_clouds(runtime, 3, lifetime);
            }
            // `RagEffect.cpp:23703-23711`
            EffectId::Stormkick4 => {
                if runtime.frame == 15 {
                    // `this->render.position = this->state.parent->render.position;`
                    // is the spawn anchor already; only the wave is observable.
                    runtime.play_sound("effect\\T_회피.wav");
                }

                // `for (int index = 0; index < 30; ++index)`
                for _ in 0..30 {
                    WhiteLightTrail::new(WHITE_LIGHT, 1, lifetime).on_process(runtime);
                }
            }
            // `RagEffect.cpp:23712-23722`
            EffectId::Stormkick5 => {
                if runtime.frame == 15 {
                    runtime.play_sound("effect\\T_회피.wav");
                    // `this->state.parent->render.state = 37;` (`ST_NINJAREADY`)
                    runtime.set_parent_state(37);
                    // `SetFrameSequence(0x60, 2, 2, 0x60, 2, 1, 25, 0)`
                    runtime.set_parent_frame_sequence(ParentFrameSequence {
                        animation: 0x60,
                        first_frame: 2,
                        last_frame: 2,
                        finish_animation: 0x60,
                        finish_frame: 2,
                        timed: 1,
                        duration: 25,
                        loop_: 0,
                    });
                }

                for _ in 0..30 {
                    WhiteLightTrail::new(WHITE_LIGHT, 1, lifetime).on_process(runtime);
                }
            }
            // `RagEffect.cpp:23723-23731`
            EffectId::Stormkick6 => {
                if runtime.frame == 0 {
                    runtime.play_sound("effect\\REVERBERATION.wav");
                }

                marks_and_clouds(runtime, 11, lifetime);
            }
            // `RagEffect.cpp:23732-23736`
            EffectId::Stormkick7 => {
                marks_and_clouds(runtime, 2, lifetime);
            }
            _ => {}
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    const IDS: [EffectId; 6] = [
        EffectId::Stormkick,
        EffectId::Stormkick1,
        EffectId::Stormkick2,
        EffectId::Stormkick3,
        EffectId::Stormkick6,
        EffectId::Stormkick7,
    ];

    /// The two white-light-trail ids (`RagEffect.cpp:23703-23722`).
    const TRAIL_IDS: [EffectId; 2] = [EffectId::Stormkick4, EffectId::Stormkick5];

    fn run(id: EffectId, frames: usize) -> EffectRuntime {
        let mut effect = StormKick::new(id);
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 17);

        for _ in 0..frames {
            step(&mut effect, &mut runtime);
        }

        runtime
    }

    #[test]
    fn lifetimes_match_the_effect_types_table() {
        let expect = [
            (EffectId::Stormkick, 100),
            (EffectId::Stormkick1, 100),
            (EffectId::Stormkick2, 100),
            (EffectId::Stormkick3, 100),
            (EffectId::Stormkick4, 200),
            (EffectId::Stormkick5, 300),
            (EffectId::Stormkick6, 100),
            (EffectId::Stormkick7, 1000),
        ];

        for (id, lifetime) in expect {
            assert_eq!(StormKick::new(id).lifetime(), lifetime, "{id:?}");
        }
    }

    #[test]
    fn every_body_spawns_the_mark_then_the_two_cloud_layers() {
        for id in IDS {
            let runtime = run(id, 1);

            assert_eq!(runtime.primitives.len(), 3, "{id:?}");

            assert_eq!(runtime.primitives[0].state.count, PrimitiveId::StackedRings, "{id:?}");
            assert_eq!(runtime.primitives[0].state.count.id(), 103, "{id:?}");
            assert_eq!(runtime.primitives[1].state.count, PrimitiveId::Wind, "{id:?}");
            assert_eq!(runtime.primitives[2].state.count, PrimitiveId::Wind, "{id:?}");

            assert_eq!(runtime.primitives[0].state.texture, Some("effect\\storm2.tga"), "{id:?}");
            assert_eq!(runtime.primitives[1].state.texture, Some("effect\\cloud11.tga"), "{id:?}");
            assert_eq!(runtime.primitives[2].state.texture, Some("effect\\cloud11.tga"), "{id:?}");

            // Cloud 2 then cloud 3: the kind-2 slot heights start at 48, the
            // kind-3 ones at 28, and both use alphaT 2.
            assert_eq!(runtime.primitives[1].state.slots[0].max_height, 48.0, "{id:?}");
            assert_eq!(runtime.primitives[2].state.slots[0].max_height, 28.0, "{id:?}");
            assert_eq!(runtime.primitives[1].state.slots[0].alpha_t, 2, "{id:?}");
            assert_eq!(runtime.primitives[2].state.slots[0].alpha_t, 2, "{id:?}");
        }
    }

    #[test]
    fn each_body_uses_its_official_textured_mark_kind() {
        let expect = [
            (EffectId::Stormkick, 0i8),
            (EffectId::Stormkick1, 1),
            (EffectId::Stormkick2, 6),
            (EffectId::Stormkick3, 3),
            (EffectId::Stormkick6, 11),
            (EffectId::Stormkick7, 2),
        ];

        for (id, kind) in expect {
            let runtime = run(id, 1);
            let mark = &runtime.primitives[0].state;

            assert_eq!(mark.slots[0].flag1[4], kind, "{id:?}");
            assert_eq!(mark.render_flag, 1 | 4, "{id:?}");
            assert_eq!(
                mark.duration,
                if matches!(id, EffectId::Stormkick7) { 1000 } else { 100 },
                "{id:?}"
            );

            // Kind 2 sets flag1[5] = 10 (never fades); the rest keep 0.
            assert_eq!(mark.slots[0].flag1[5], if kind == 2 { 10 } else { 0 }, "{id:?}");
        }
    }

    #[test]
    fn the_layers_spawn_only_on_frame_zero() {
        for id in IDS {
            let runtime = run(id, 10);

            assert_eq!(runtime.primitives.len(), 3, "{id:?}");
        }
    }

    #[test]
    fn frame_zero_consumes_the_rng_in_dispatch_order() {
        // Frame 0, in order: TexturedMark's `ClampedRandom(360)`, the four
        // `ClampedRandom(21) + 80` rise angles of Cloud11Burst(2), the four of
        // Cloud11Burst(3), then PrimSpinFade's `rand() % 3` when the
        // primitives are processed.
        const SEED: u64 = 4242;
        let mut model = crate::world::effect::port::rng::EffectRng::new(SEED);
        let rot_start = model.clamped_random(360) as i16;
        let mut rises = [[0i16; 4]; 2];

        for layer in rises.iter_mut() {
            for rise in layer.iter_mut() {
                *rise = (model.clamped_random(21) + 80) as i16;
            }
        }

        let turn = model.clamped_random(3) as i16;

        let mut effect = StormKick::new(EffectId::Stormkick7);
        let mut runtime = EffectRuntime::new(1000, [0.0, 0.0], SEED);
        step(&mut effect, &mut runtime);

        let mark = &runtime.primitives[0].state;
        assert_eq!(mark.slots[0].rot_start, (rot_start + turn + 3) % 360);

        for (layer, expected) in rises.iter().enumerate() {
            let cloud = &runtime.primitives[1 + layer].state;

            for index in 0..4usize {
                assert_eq!(cloud.slots[index].rise_angle, expected[index], "layer {layer} slot {index}");
            }
        }
    }

    #[test]
    fn stormkick_plays_its_wave_on_frame_five() {
        let runtime = run(EffectId::Stormkick, 5);
        assert!(runtime.sounds.is_empty());

        let runtime = run(EffectId::Stormkick, 6);
        assert_eq!(runtime.sounds, vec!["effect\\T_회오리차기.wav".to_owned()]);
    }

    #[test]
    fn stormkick2_plays_its_wave_on_frame_five() {
        let runtime = run(EffectId::Stormkick2, 5);
        assert!(runtime.sounds.is_empty());

        let runtime = run(EffectId::Stormkick2, 6);
        assert_eq!(runtime.sounds, vec!["ice_titan_attack.wav".to_owned()]);
    }

    #[test]
    fn stormkick6_plays_its_wave_on_frame_zero() {
        let runtime = run(EffectId::Stormkick6, 1);
        assert_eq!(runtime.sounds, vec!["effect\\REVERBERATION.wav".to_owned()]);
    }

    #[test]
    fn the_other_bodies_play_no_wave() {
        for id in [EffectId::Stormkick1, EffectId::Stormkick3, EffectId::Stormkick7] {
            let runtime = run(id, 60);
            assert!(runtime.sounds.is_empty(), "{id:?}");
        }
    }

    #[test]
    fn stormkick_spins_the_parent_thirty_degrees_until_frame_47() {
        let mut effect = StormKick::new(EffectId::Stormkick);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);
        runtime.parent_angle = 300.0;

        // Frames 0..=47 spin; frame 48 does not.
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_angle, 330.0);
        step(&mut effect, &mut runtime);
        // 330 + 30 == 360, and the original compares `> 360.0f`.
        assert_eq!(runtime.parent_angle, 360.0);
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_angle, 30.0);

        // Replay to frame 48 and prove the gate.
        let mut effect = StormKick::new(EffectId::Stormkick);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);
        for _ in 0..48 {
            step(&mut effect, &mut runtime);
        }

        let before = runtime.parent_angle;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_angle, before);
    }

    #[test]
    fn stormkick1_spins_the_parent_fourteen_degrees_through_frame_50() {
        let mut effect = StormKick::new(EffectId::Stormkick1);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);
        runtime.parent_angle = 350.0;

        step(&mut effect, &mut runtime);
        // 350 + 14 == 364, so one subtraction applies.
        assert_eq!(runtime.parent_angle, 4.0);

        // 51 frames (0..=50) spin, frame 51 does not.
        let mut effect = StormKick::new(EffectId::Stormkick1);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);
        for _ in 0..51 {
            step(&mut effect, &mut runtime);
        }

        let before = runtime.parent_angle;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_angle, before);
    }

    #[test]
    fn the_other_bodies_do_not_spin_the_parent() {
        for id in [
            EffectId::Stormkick2,
            EffectId::Stormkick3,
            EffectId::Stormkick6,
            EffectId::Stormkick7,
        ] {
            let mut effect = StormKick::new(id);
            let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 17);
            runtime.parent_angle = 90.0;

            for _ in 0..60 {
                step(&mut effect, &mut runtime);
            }

            assert_eq!(runtime.parent_angle, 90.0, "{id:?}");
        }
    }

    #[test]
    fn the_trail_bodies_spawn_thirty_white_light_primitives_on_frame_zero() {
        for id in TRAIL_IDS {
            let runtime = run(id, 1);

            // Each `WhiteLightTrail` call launches its own prim 96.
            assert_eq!(runtime.primitives.len(), 30, "{id:?}");

            for primitive in &runtime.primitives {
                let state = &primitive.state;
                assert_eq!(state.count, PrimitiveId::PeongUp, "{id:?}");
                assert_eq!(state.count.id(), 96, "{id:?}");
                assert_eq!(state.render_flag, 1 | 4, "{id:?}");
                assert_eq!(state.texture, Some("effect\\whitelight.tga"), "{id:?}");
                assert_eq!(
                    state.duration,
                    if matches!(id, EffectId::Stormkick4) { 200 } else { 300 },
                    "{id:?}"
                );

                for slot in &state.slots {
                    assert_eq!(slot.life, 1, "{id:?}");
                    // The kind-1 branch marks the blue colour.
                    assert_eq!(slot.flag1[0], 2, "{id:?}");
                }
            }
        }
    }

    #[test]
    fn the_trail_bodies_stop_emitting_after_frame_zero() {
        for id in TRAIL_IDS {
            let runtime = run(id, 10);
            assert_eq!(runtime.primitives.len(), 30, "{id:?}");
        }
    }

    #[test]
    fn both_trail_bodies_play_the_evasion_wave_on_frame_fifteen() {
        for id in TRAIL_IDS {
            let runtime = run(id, 15);
            assert!(runtime.sounds.is_empty(), "{id:?}");

            let runtime = run(id, 16);
            assert_eq!(runtime.sounds, vec!["effect\\T_회피.wav".to_owned()], "{id:?}");
        }
    }

    #[test]
    fn stormkick5_sets_the_ninja_ready_state_and_frame_sequence_on_frame_fifteen() {
        let at_fourteen = run(EffectId::Stormkick5, 15);
        assert_eq!(at_fourteen.parent_state, None);
        assert_eq!(at_fourteen.parent_frame_sequence, None);

        let runtime = run(EffectId::Stormkick5, 16);

        // `parent->render.state = 37` (`ST_NINJAREADY`, `ActorEnums.h:55`).
        assert_eq!(runtime.parent_state, Some(37));
        let sequence = runtime.parent_frame_sequence.expect("SetFrameSequence ran");
        assert_eq!(sequence, ParentFrameSequence {
            animation: 0x60,
            first_frame: 2,
            last_frame: 2,
            finish_animation: 0x60,
            finish_frame: 2,
            timed: 1,
            duration: 25,
            loop_: 0,
        });
        assert_eq!(runtime.parent_frame_sequence_frame, 15);
    }

    #[test]
    fn stormkick4_does_not_touch_the_parent_state_or_sequence() {
        let runtime = run(EffectId::Stormkick4, 20);
        assert_eq!(runtime.parent_state, None);
        assert_eq!(runtime.parent_frame_sequence, None);
    }
}
