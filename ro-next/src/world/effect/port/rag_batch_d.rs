//! Effects ported by the "d" parallel batch.
//!
//! This file is owned exclusively by batch "d": bodies and their tests live
//! here so parallel work never edits a shared module.

#![allow(dead_code)]

use super::rag::{TexturedAura, TexturedSkillAura};
use super::rag_batch_b::ParticleTime;
use super::runtime::{EffectRuntime, PortedEffect};

// ---------------------------------------------------------------------------
// EF_NPC_SLOWCAST
// ---------------------------------------------------------------------------

/// `EF_NPC_SLOWCAST` dispatch block - `RagEffect.cpp:20901-20907`.
///
/// Every frame calls
/// `CRagEffect::TexturedAura("effect\blast_mine##clock.bmp", 4)`
/// (`RagEffect.cpp:17769-17960`); on frame 10 it plays
/// `effect\EF_DecAgility.wav` (`PlayEffectWave`). The original re-anchors
/// `render.position` to the parent on that frame - the runtime already tracks
/// the spawn position, so no state is copied.
#[derive(Debug)]
pub struct NpcSlowcast {
    aura: TexturedAura,
    lifetime: i32,
}

impl NpcSlowcast {
    pub fn new(lifetime: i32) -> Self {
        Self {
            aura: TexturedAura::new(&[("effect\\blast_mine##clock.bmp", 4)], lifetime),
            lifetime,
        }
    }
}

impl PortedEffect for NpcSlowcast {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // Official order: TexturedAura is called first, then the wave gate.
        self.aura.on_process(runtime);

        if runtime.frame == 10 {
            runtime.play_sound("effect\\EF_DecAgility.wav");
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_WINDCUTTER
// ---------------------------------------------------------------------------

/// `EF_WINDCUTTER` dispatch block - `RagEffect.cpp:20693-20699`.
///
/// On frame 1 plays `effect\wl_siennaexecrate.wav` (`PlayEffectWave`), then
/// every frame calls `CRagEffect::TexturedSkillAura(8)`
/// (`RagEffect.cpp:15281-15370`), whose own frame-0 gate spawns the single
/// `PP_3DTEXTURE` aura.
#[derive(Debug)]
pub struct Windcutter {
    aura: TexturedSkillAura,
    lifetime: i32,
}

impl Windcutter {
    pub fn new(lifetime: i32) -> Self {
        Self {
            aura: TexturedSkillAura::new(8, lifetime),
            lifetime,
        }
    }
}

impl PortedEffect for Windcutter {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // Official order: the frame-1 wave precedes the helper call.
        if runtime.frame == 1 {
            runtime.play_sound("effect\\wl_siennaexecrate.wav");
        }

        self.aura.on_process(runtime);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_TEMP_OK
// ---------------------------------------------------------------------------

/// `EF_TEMP_OK` dispatch block - `RagEffect.cpp:21231-21237`.
///
/// On frame 0 plays `_heal_effect.wav` (`PlayEffectWave`, note the name has no
/// `effect\` prefix), then every frame calls
/// `CRagEffect::TexturedAura("effect\success.bmp", 3)`
/// (`RagEffect.cpp:17769-17960`).
#[derive(Debug)]
pub struct TempOk {
    aura: TexturedAura,
    lifetime: i32,
}

impl TempOk {
    pub fn new(lifetime: i32) -> Self {
        Self {
            aura: TexturedAura::new(&[("effect\\success.bmp", 3)], lifetime),
            lifetime,
        }
    }
}

impl PortedEffect for TempOk {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.play_sound("_heal_effect.wav");
        }

        self.aura.on_process(runtime);

        true
    }
}

// ---------------------------------------------------------------------------
// EF_HATED2
// ---------------------------------------------------------------------------

/// `EF_HATED2` dispatch block - `RagEffect.cpp:21346-21352`.
///
/// On frame 10 plays `effect\T_따듯한마법.wav` (`PlayEffectWave`), then every
/// frame calls `CRagEffect::ParticleTime("effect\white02.bmp", 3)`
/// (`RagEffect.cpp:12121-12177`), which bursts four-slot `PP_PARTICLEUP`
/// orbs on frames 0, 4, ..., 80.
#[derive(Debug)]
pub struct Hated2 {
    particles: ParticleTime,
    lifetime: i32,
}

impl Hated2 {
    pub fn new(lifetime: i32) -> Self {
        Self {
            particles: ParticleTime::new("effect\\white02.bmp", 3, lifetime),
            lifetime,
        }
    }
}

impl PortedEffect for Hated2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // Official order: the frame-10 wave precedes the helper call.
        if runtime.frame == 10 {
            runtime.play_sound("effect\\T_따듯한마법.wav");
        }

        self.particles.on_process(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::runtime::EffectRuntime;

    /// Steps `effect` for frames `0..=last` and returns the frames on which a
    /// wave was queued (each frame's appends are measured separately).
    fn wave_frames(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime, last: i32) -> Vec<i32> {
        let mut frames = Vec::new();
        let mut sounds = runtime.sounds.len();

        for frame in 0..=last {
            runtime.frame = frame;
            assert!(effect.on_process(runtime));

            if runtime.sounds.len() > sounds {
                frames.push(frame);
                sounds = runtime.sounds.len();
            }
        }

        frames
    }

    // -- EF_NPC_SLOWCAST ---------------------------------------------------

    #[test]
    fn npc_slowcast_spawns_the_kind_four_aura() {
        let mut effect = NpcSlowcast::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);

        assert_eq!(effect.lifetime(), 300);
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;

        assert_eq!(state.count, PrimitiveId::AuraGlow);
        assert_eq!(state.count.id(), 82);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, 300);
        assert_eq!(state.texture, Some("effect\\blast_mine##clock.bmp"));
        assert_eq!(state.size, 0.0);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 0);
        // Kind 4: distance 6.0, display 135, no extra height, lift -30.
        assert_eq!(slot.distance, 6.0);
        assert_eq!(slot.vec_b_pre[1], -30.0);
        assert_eq!(slot.flag1[2], 0);
        assert_eq!(slot.height[10], 0.0);
        assert_eq!(slot.full_display_angle, 135);
        // `kind == 4` stores motion kind 1 (`RagEffect.cpp:17964`).
        assert_eq!(slot.flag1[4], 1);
        assert_eq!(slot.height[15], 30.0);
        assert_eq!(state.slots[1].life, 0);
    }

    #[test]
    fn npc_slowcast_plays_its_wave_only_on_frame_ten() {
        let mut effect = NpcSlowcast::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 11);

        assert_eq!(wave_frames(&mut effect, &mut runtime, 12), vec![10]);
        assert_eq!(runtime.sounds, vec!["effect\\EF_DecAgility.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1, "only the frame-0 aura spawns");
    }

    // -- EF_WINDCUTTER -----------------------------------------------------

    #[test]
    fn windcutter_spawns_the_type_eight_skill_aura() {
        let mut effect = Windcutter::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 12);

        assert_eq!(effect.lifetime(), 100);
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 1);
        assert!(runtime.sounds.is_empty(), "the wave is on frame 1");

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Texture3d);
        assert_eq!(state.count.id(), 15);
        assert_eq!(state.duration, 57);
        assert_eq!(state.texture, Some("effect\\fashasha.tga"));
        assert_eq!(state.width_speed, 1.5);
        assert_eq!(state.height_speed, 1.5);
        assert!((state.width_accel - 1.5 / 14.0).abs() < 1e-6);
        assert!((state.height_accel - 1.5 / 14.0).abs() < 1e-6);
        // The second assignment of deltaPos2.y wins.
        assert_eq!(state.delta_pos2[1], -5.0);
        assert_eq!(state.speed3d[1], -0.5);
        assert_eq!(state.roll, 90.0);
        // Type 8 is not 3 or 6: the default burst window.
        assert_eq!(state.change_point, 17);
        assert_eq!(state.fade_out_cnt, 40);
        assert_eq!((state.red, state.green, state.blue), (0x8C, 0xFF, 0x8C));
        assert_eq!(state.change_width_speed, 0.1);
        assert_eq!(state.change_height_speed, 0.1);
    }

    #[test]
    fn windcutter_plays_its_wave_only_on_frame_one() {
        let mut effect = Windcutter::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 12);

        assert_eq!(wave_frames(&mut effect, &mut runtime, 100), vec![1]);
        assert_eq!(runtime.sounds, vec!["effect\\wl_siennaexecrate.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);
    }

    // -- EF_TEMP_OK --------------------------------------------------------

    #[test]
    fn temp_ok_plays_the_heal_wave_on_frame_zero_and_spawns_the_kind_three_aura() {
        let mut effect = TempOk::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 13);

        assert_eq!(effect.lifetime(), 100);
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, vec!["_heal_effect.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::AuraGlow);
        assert_eq!(state.duration, 100);
        assert_eq!(state.texture, Some("effect\\success.bmp"));

        let slot = &state.slots[0];
        // Kind 3: distance 8.0, display 155, height[10] 2.0, lift -30.
        assert_eq!(slot.distance, 8.0);
        assert_eq!(slot.vec_b_pre[1], -30.0);
        assert_eq!(slot.flag1[2], 3);
        assert_eq!(slot.height[10], 2.0);
        assert_eq!(slot.full_display_angle, 155);
        assert_eq!(slot.flag1[4], 3);
        assert_eq!(slot.height[15], 30.0);
    }

    #[test]
    fn temp_ok_plays_no_further_waves() {
        let mut effect = TempOk::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 13);

        assert_eq!(wave_frames(&mut effect, &mut runtime, 100), vec![0]);
    }

    // -- EF_HATED2 ---------------------------------------------------------

    #[test]
    fn hated2_spawns_twenty_one_particle_bursts() {
        let mut effect = Hated2::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 14);

        assert_eq!(effect.lifetime(), 300);

        for frame in 0..=300 {
            runtime.frame = frame;
            assert!(effect.on_process(&mut runtime));
        }

        // Kind 3 bursts on 0, 4, ..., 80: (80 - 0) / 4 + 1 = 21.
        assert_eq!(runtime.primitives.len(), 21);

        for primitive in &runtime.primitives {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleUp);
            assert_eq!(state.count.id(), 59);
            assert_eq!(state.render_flag, 1 | 4);
            assert_eq!(state.duration, 300);
            assert_eq!(state.total_texture, 2);
            assert_eq!(state.texture, Some("effect\\pok1.tga"));
            assert_eq!(state.texture_secondary, Some("effect\\pok3.tga"));
            assert_eq!(state.size, 0.0);

            for slot in &state.slots {
                assert_eq!(slot.life, 1);
                // Kind 3 stores flag 13 in the third flag byte.
                assert_eq!(slot.flag1[2], 13);
            }
        }
    }

    #[test]
    fn hated2_plays_its_wave_only_on_frame_ten() {
        let mut effect = Hated2::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 14);

        assert_eq!(wave_frames(&mut effect, &mut runtime, 300), vec![10]);
        assert_eq!(runtime.sounds, vec!["effect\\T_따듯한마법.wav".to_owned()]);
    }
}
