//! Functional ports for the effects that only had their shake wired up.
//!
//! These bodies use the already-ported primitive families for the visible
//! parts that map cleanly (`CVolcanoEffect` ground rings, `LineStreakLayered`
//! bars and `Magnum2` waves). The original particle systems for Acid
//! Demonstration, Meteor Storm and Lord of Vermilion are not particle-for-
//! particle copies yet; these are the decomp-scheduled shake frames plus the
//! closest ported geometry, so the skills no longer play without any visual.

use ragnarok_packets::EffectId;

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
            ring(runtime, "effect\\ring_black.tga");
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
        if runtime.frame == 44 {
            runtime.quake();
            runtime.copy_parent_position();
            runtime.play_sound("effect\\연주붕격.wav");
        }

        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_black.tga");
        }

        true
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
        if runtime.frame == 44 {
            runtime.quake();
            runtime.copy_parent_position();

            if !self.dash2 {
                runtime.play_sound("effect\\연주붕격.wav");
            }
        }

        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_black.tga");
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

        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_red.tga");
        }

        true
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
        if matches!(runtime.frame, 3 | 7 | 9) {
            runtime.quake();
        }

        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_red.tga");
        }

        true
    }
}

/// `EF_TETRA_GROUND` - `RagEffect.cpp:20190-20202`.
#[derive(Debug, Default)]
pub struct TetraGround;

impl PortedEffect for TetraGround {
    fn lifetime(&self) -> i32 {
        600
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            ring(runtime, "effect\\magic_green.tga");
            runtime.quake();
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
        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_black.tga");
            runtime.quake();
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
        if runtime.frame == 35 {
            runtime.quake();
        }

        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_red.tga");
        }

        true
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
        if (25..=55).contains(&runtime.frame) && runtime.frame % 3 == 0 {
            runtime.quake();
        }

        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_red.tga");
        }

        true
    }
}

/// `EF_LORD` - `RagEffect.cpp:24935-24945`.
#[derive(Debug, Default)]
pub struct Lord;

impl PortedEffect for Lord {
    fn lifetime(&self) -> i32 {
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            ring(runtime, "effect\\ring_blue.tga");
            runtime.quake();
        }

        true
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
