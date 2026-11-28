//! `CVolcanoEffect`'s ground shockwaves - `ThrowEffects.cpp:619-714`.
//!
//! The official class builds a 360-degree fan of `CParticle` quads for
//! `EF_VOLCANO`, `EF_DELUGE`, `EF_VIOLENTGALE`, `EF_LANDPROTECTOR`,
//! `EF_GANBANTEIN`, `EF_GUMGANG2..9`. The complete particle path is not part
//! of the primitive model, so the same ring is assembled from the ported
//! `PP_CHEMICALPROTECTION` wedge primitive (36 ten-degree segments). It uses
//! the effect's official `ring_*` texture and reproduces the visible ground
//! wave; the remaining field geometry of those skills is separate and still
//! follows the ordinary skill effect path.

use ragnarok_packets::EffectId;

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// One `CVolcanoEffect` shockwave ring.
pub(crate) fn shockwave(runtime: &mut EffectRuntime, texture: &'static str) {
    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    for group in 0..9usize {
        let index = runtime.launch(PrimitiveId::ChemicalProtection, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.total_texture = 1;
        state.texture = Some(texture);
        state.size = 0.0;

        for slot_index in 0..4usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = 0;
            slot.distance = 35.0;
            slot.rise_angle = ((group * 4 + slot_index) * 10) as i16;
            slot.alpha_b = 0;
        }
    }
}

/// Generic `CVolcanoEffect` body for the ground-wave family.
#[derive(Debug)]
pub struct VolcanoShockwave {
    effect: EffectId,
    texture: &'static str,
}

impl VolcanoShockwave {
    pub const fn new(effect: EffectId, texture: &'static str) -> Self {
        Self { effect, texture }
    }

    fn lifetime_for(effect: EffectId) -> i32 {
        match effect {
            EffectId::Ganbantein | EffectId::Volcano | EffectId::Gumgang3 | EffectId::Gumgang4 | EffectId::Gumgang5
            | EffectId::Gumgang6 | EffectId::Gumgang7 | EffectId::Gumgang8 | EffectId::Gumgang9 => 300,
            EffectId::Deluge | EffectId::Violentgale | EffectId::Landprotector => 999,
            _ => 300,
        }
    }
}

impl PortedEffect for VolcanoShockwave {
    fn lifetime(&self) -> i32 {
        Self::lifetime_for(self.effect)
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        shockwave(runtime, self.texture);

        true
    }
}

/// Registry entries for every `CVolcanoEffect` ground wave.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    let texture = match effect {
        EffectId::Volcano | EffectId::Gumgang7 => "effect\\ring_red.tga",
        EffectId::Deluge | EffectId::Gumgang5 => "effect\\ring_blue.tga",
        EffectId::Violentgale => "effect\\magic_green.tga",
        EffectId::Landprotector | EffectId::Ganbantein | EffectId::Gumgang6 => "effect\\ring_white.tga",
        EffectId::Gumgang3 | EffectId::Gumgang8 => "effect\\ring_yellow.tga",
        EffectId::Gumgang4 => "effect\\ring_purple.tga",
        EffectId::Gumgang9 => "effect\\ring_brown.tga",
        _ => return None,
    };

    Some(Box::new(VolcanoShockwave::new(effect, texture)))
}

pub fn ids() -> &'static [EffectId] {
    &[
        EffectId::Volcano,
        EffectId::Deluge,
        EffectId::Violentgale,
        EffectId::Landprotector,
        EffectId::Ganbantein,
        EffectId::Gumgang3,
        EffectId::Gumgang4,
        EffectId::Gumgang5,
        EffectId::Gumgang6,
        EffectId::Gumgang7,
        EffectId::Gumgang8,
        EffectId::Gumgang9,
    ]
}
