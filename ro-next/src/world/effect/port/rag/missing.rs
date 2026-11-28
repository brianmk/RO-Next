//! Functional ports for the effects that only had their shake wired up.
//!
//! These bodies use the already-ported primitive families for the visible
//! parts that map cleanly (`CVolcanoEffect` ground rings, `LineStreakLayered`
//! bars and `Magnum2` waves). The original particle systems for Acid
//! Demonstration, Meteor Storm and Lord of Vermilion are not particle-for-
//! particle copies yet; these are the decomp-scheduled shake frames plus the
//! closest ported geometry, so the skills no longer play without any visual.

use ragnarok_packets::EffectId;

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

fn ring(runtime: &mut EffectRuntime, texture: &'static str) {
    super::volcano::shockwave(runtime, texture);
}

/// `EF_SCREEN_QUAKE` - `RagEffect.cpp:21737-21742`.
#[derive(Debug, Default)]
pub struct ScreenQuake;

impl PortedEffect for ScreenQuake {
    fn lifetime(&self) -> i32 {
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame < 100 && runtime.frame % 5 == 0 {
            runtime.quake();
        }

        true
    }
}

/// `EF_ACIDDEMON` - `RagEffect.cpp:21396-21401`.
#[derive(Debug, Default)]
pub struct AcidDemon;

impl PortedEffect for AcidDemon {
    fn lifetime(&self) -> i32 {
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 5 {
            runtime.quake();
        }

        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\EF_FireWall.wav");

            let lifetime = runtime.lifetime;
            let index = runtime.launch(PrimitiveId::AcidDemon, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);

            state.render_flag = 1 | 4;
            state.duration = lifetime;
            state.total_texture = 1;
            state.texture = Some("effect\\ring_black.tga");
            state.size = 13.0;

            for slot_index in 0..4usize {
                let slot = &mut state.slots[slot_index];

                slot.life = 1;
                slot.process = 0;
                slot.full_display_angle = if slot_index == 3 { 360 } else { 315 };
                slot.max_height = [25.0, 22.0, 19.0, 30.0][slot_index];
                slot.rot_start = [0, 90, 180, 0][slot_index];
                slot.alpha_b = 0;
                slot.distance = [2.4, 2.7, 3.0, 2.2][slot_index];
                slot.rise_angle = [70, 57, 45, 90][slot_index];
                slot.vec_b_now[1] = 0.0;

                for height in 0..21usize {
                    slot.height[height] = 0.0;
                    slot.flag1[height] = 0;
                }
            }
        }

        true
    }
}

/// `EF_CHEMICAL2` - `RagEffect.cpp:10419-10433`.
#[derive(Debug, Default)]
pub struct Chemical2;

impl PortedEffect for Chemical2 {
    fn lifetime(&self) -> i32 {
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 40 {
            // `this->SlashFan(0); this->SlashFan(1);`
            slash_fan(runtime, false);
            slash_fan(runtime, true);
        }

        if runtime.frame == 44 {
            runtime.quake();
            runtime.copy_parent_position();
            runtime.play_sound("effect\\연주붕격.wav");
        }

        true
    }
}

/// `CRagEffect::SlashFan` - `RagEffect.cpp:4144-4183`.
fn slash_fan(runtime: &mut EffectRuntime, reverse: bool) {
    runtime.copy_parent_position();
    let target = EffectRuntime::official_position(runtime.target_offset);
    let position = EffectRuntime::official_position(runtime.position);

    let lifetime = runtime.lifetime;
    let rise_angle = runtime.get_angle(target[0] - position[0], target[2] - position[2]) as i16;
    let index = runtime.launch(PrimitiveId::SlashFan, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag |= 0x200;
    state.duration = lifetime;
    state.texture = Some("effect\\slash01.tga");

    for slot_index in 0..4usize {
        let slot = &mut state.slots[slot_index];

        slot.life = 1;
        slot.process = 0;
        slot.distance = 100.0;
        slot.max_height = 1.0;
        slot.height[0] = if reverse {
            slot_index as f32 * 2.5 + 15.0
        } else {
            -15.0 - slot_index as f32 * 2.5
        };
        slot.rise_angle = rise_angle;

        if slot.rise_angle >= 360 {
            slot.rise_angle -= 360;
        }

        slot.alpha_b = 0;
    }
}

/// `EF_CHEMICAL2DASH` / `EF_CHEMICAL2DASH2` - `RagEffect.cpp:21489-21505`,
/// `:19847-19861`.
#[derive(Debug)]
pub struct ChemicalDash {
    dash2: bool,
}

impl ChemicalDash {
    pub const fn new(dash2: bool) -> Self {
        Self { dash2 }
    }
}

impl PortedEffect for ChemicalDash {
    fn lifetime(&self) -> i32 {
        300
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        for _ in 0..4 {
            super::line_streak::line_streak_layered(runtime, 10);
            super::line_streak::line_streak_layered(runtime, 11);
        }

        if runtime.frame == 44 {
            runtime.quake();
            runtime.copy_parent_position();

            if !self.dash2 {
                runtime.play_sound("effect\\연주붕격.wav");
            }
        }

        true
    }
}

/// `EF_DRAGONFEAR` - `RagEffect.cpp:21940-21948`.
#[derive(Debug, Default)]
pub struct DragonFear;

impl PortedEffect for DragonFear {
    fn lifetime(&self) -> i32 {
        28
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 1 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\dragonfear.wav");
            runtime.quake();
        }

        runtime.process_str()
    }
}

/// `EF_SUI_EXPLOSION` - `RagEffect.cpp:25178-25190`.
#[derive(Debug, Default)]
pub struct SuiExplosion;

impl PortedEffect for SuiExplosion {
    fn lifetime(&self) -> i32 {
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            // `this->Hit2(1);` - the impact shards are a real primitive layer
            // under the screen clip.
            super::hit::Hit2::new(1).on_process(runtime);
        }

        if matches!(runtime.frame, 3 | 7 | 9) {
            runtime.quake();
        }

        runtime.process_str()
    }
}

/// `EF_TETRA_GROUND` - `RagEffect.cpp:20190-20202`.
#[derive(Debug, Default)]
pub struct TetraGround;

/// `CRagEffect::TetraVortexPillar(4)` - `RagEffect.cpp:8335-8430`.
fn tetra_vortex_pillar(runtime: &mut EffectRuntime) {
    runtime.copy_parent_position();

    let frame = runtime.frame;

    if frame < 100 && frame % 20 == 0 {
        match frame {
            0 => {
                runtime.play_sound_at_height("effect\\wizard_meteo.wav", 0.0);
                runtime.play_sound("effect\\hunter_blastmine.wav");
            }
            10 | 20 | 50 | 80 => runtime.play_sound_at_height("effect\\wl_tetravortex.wav", 0.0),
            _ => {}
        }
    }

    if frame > 80 || frame % 25 != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = EffectRuntime::official_position(runtime.position);
    let target = EffectRuntime::official_position(runtime.target_offset);
    let rot_start = runtime.rng.clamped_random(360) as i16;

    let index = runtime.launch(PrimitiveId::FrameFade, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 8;
    state.texture = Some("effect\\tv-1.bmp");
    state.texture_frames = vec![
        "effect\\tv-1.bmp",
        "effect\\tv-2.bmp",
        "effect\\tv-3.bmp",
        "effect\\tv-4.bmp",
        "effect\\tv-5.bmp",
        "effect\\tv-6.bmp",
        "effect\\tv-7.bmp",
        "effect\\tv-8.bmp",
    ];
    state.size = 0.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    let slot = &mut state.slots[0];
    slot.life = 1;
    slot.process = 0;
    slot.distance = 30.0;
    slot.vec_b_pre[1] = position[1] - 10.0;
    slot.flag1[2] = 19;
    slot.full_display_angle = 120;
    slot.height[8] = 0.0;
    slot.height[10] = 1.0;
    slot.alpha_b = 200;
    slot.vec_b_now[0] = target[0];
    slot.vec_b_now[2] = target[2];
    slot.vec_b_now[1] = target[1] - 35.0;
    slot.rot_start = rot_start;
    slot.flag1[0] = 0;
}

impl PortedEffect for TetraGround {
    fn lifetime(&self) -> i32 {
        600
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if matches!(runtime.frame, 0 | 40 | 80) {
            runtime.quake();
        }

        tetra_vortex_pillar(runtime);

        if runtime.frame == 0 {
            // `for (int index = 0; index < 7; ++index)
            //  this->TexturedSpread("effect\ring_red.tga", 10);`
            for _ in 0..7 {
                super::spread_flat::spawn_one(runtime, "effect\\ring_red.tga", 10);
            }
        }

        true
    }
}

/// `EF_GRAVITATION` - `RagEffect.cpp:14550-14560`.
#[derive(Debug, Default)]
pub struct Gravitation;

impl PortedEffect for Gravitation {
    fn lifetime(&self) -> i32 {
        2000
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        runtime.quake();

        let angles = super::super::angles::Angles::shared();
        let origin = EffectRuntime::official_position(runtime.position);

        for z in (-10..15).step_by(5) {
            for x in (-10..15).step_by(5) {
                for i in 0..4usize {
                    let index = runtime.launch(PrimitiveId::QuadHorn2, [0.0, 0.0, 0.0]);

                    let angle = runtime.rng.clamped_random(360);
                    runtime.rng.discard();
                    let _ = angles.sine(angle);
                    let _ = angles.cosine(angle);

                    let size = (runtime.rng.clamped_random(50) + 300) as f32 / 100.0;
                    let longitude = runtime.rng.clamped_random(360) as f32;
                    let latitude = (runtime.rng.clamped_random(40) + 60) as f32;

                    let state = runtime.primitive_mut(index);

                    state.pattern = 1;
                    state.delta_pos2 = [x as f32, 10.0, z as f32];
                    state.duration = 250;
                    state.size = size;
                    state.height_size = 18.0;
                    state.longitude = longitude;
                    state.latitude = latitude;
                    state.transform.make_x_rotation(angles.radian(latitude as i32));
                    state.transform.append_y_rotation(angles.radian(longitude as i32));
                    state.rot_transform = state.transform;
                    state.speed = 1.0;
                    state.accel = 0.01;
                    state.alpha = 20.0;
                    state.ch_cnt = 15;
                    state.change_point = 12;
                    state.change_speed = -1.2;
                    state.change_accel = 0.0;
                    state.fade_out_cnt = state.duration - 10;
                    state.texture = Some(if i < 2 { "effect\\stone.bmp" } else { "effect\\ice.tga" });
                    state.position = origin;
                }
            }
        }

        true
    }
}

/// `EF_FIREPILLARBOMB` - `RagEffect.cpp:24977-25010`.
#[derive(Debug, Default)]
pub struct FirePillarBomb;

impl PortedEffect for FirePillarBomb {
    fn lifetime(&self) -> i32 {
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.play_sound("effect\\wizard_fire_pillar_b.wav");
        }

        runtime.process_str()
    }
}

/// `EF_METEORSTORM` - `RagEffect.cpp:24950-24960`.
#[derive(Debug, Default)]
pub struct MeteorStorm;

impl PortedEffect for MeteorStorm {
    fn lifetime(&self) -> i32 {
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.play_sound("effect\\wizard_meteor.wav");
        }

        if runtime.frame == 40 {
            runtime.quake();
        }

        runtime.process_str()
    }
}

/// `EF_LORD` - `RagEffect.cpp:24905-24948`.
#[derive(Debug, Default)]
pub struct Lord;

const LORD_SOUND_FRAMES: &[i32] = &[0, 10, 20, 50, 80, 100, 130, 140, 180, 200];

impl PortedEffect for Lord {
    fn lifetime(&self) -> i32 {
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if LORD_SOUND_FRAMES.contains(&runtime.frame) {
            // The official body plays a positional wave; the port records the
            // ordinary sound request (the runtime's positional sink is for the
            // explicit PlayEffectWaveAt call sites).
            runtime.play_sound("effect\\wizard_meteo.wav");
        }

        if runtime.frame >= 50 && runtime.frame < 300 && runtime.frame % 50 == 0 {
            runtime.quake();
        }

        runtime.process_str()
    }
}

/// `EffectId` -> one of the bodies above.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        EffectId::ScreenQuake => Box::new(ScreenQuake),
        EffectId::Aciddemon => Box::new(AcidDemon),
        EffectId::Chemical2 => Box::new(Chemical2),
        EffectId::Chemical2dash => Box::new(ChemicalDash::new(false)),
        EffectId::Chemical2dash2 => Box::new(ChemicalDash::new(true)),
        EffectId::Dragonfear => Box::new(DragonFear),
        EffectId::SuiExplosion => Box::new(SuiExplosion),
        EffectId::TetraGround => Box::new(TetraGround),
        EffectId::Gravitation => Box::new(Gravitation),
        EffectId::Firepillarbomb => Box::new(FirePillarBomb),
        EffectId::Meteorstorm => Box::new(MeteorStorm),
        EffectId::Lord => Box::new(Lord),
        _ => return None,
    })
}

pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::ScreenQuake,
        EffectId::Aciddemon,
        EffectId::Chemical2,
        EffectId::Chemical2dash,
        EffectId::Chemical2dash2,
        EffectId::Dragonfear,
        EffectId::SuiExplosion,
        EffectId::TetraGround,
        EffectId::Gravitation,
        EffectId::Firepillarbomb,
        EffectId::Meteorstorm,
        EffectId::Lord,
    ]
}
