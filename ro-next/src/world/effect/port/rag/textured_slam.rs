//! `CRagEffect::TexturedSlam` - the single spawner of prim 147.
//!
//! Prim 147 has no `EffectPrimId.h` symbol.
//! `EffectPrimitiveRendering.cpp:431-433` draws it with `RenderHaloLayers`
//! (prim 68's renderer) and `EffectPrimitiveMotion.cpp:8425-8426` moves it with
//! `PrimRisingFan` (prim 130's motion), so the port reuses both and names the
//! primitive after this body.
//!
//! | id | `RagEffect.cpp` | call |
//! | --- | --- | --- |
//! | `EF_SECRA` (859) | `:19783-19785` | `TexturedSlam("effect\\priest_spell.bmp", 1, 0)` |
//! | `EF_SECRA2` (862) | `:19786-19788` | `TexturedSlam("effect\\priest_spell.bmp", 2, 0)` |
//! | `EF_ROLLING1..10` (758-767) | `:20496-20529` | `TexturedSlam("effect\\회전카운터N.tga", 0, 0)` |
//! | `EF_HEARTASURA` (879) | `:20526-20528` | `TexturedSlam("effect\\kiss2.bmp", 2, 0)` |
//!
//! Lifetimes come from `EffectTypes.cpp`: 300 for `EF_SECRA` (`:868`) and
//! `EF_SECRA2` (`:871`), 99999999 for `EF_ROLLING1..10` (`:767-776`) and
//! `EF_HEARTASURA` (`:888`).
//!
//! Quirks kept:
//!
//! * the body is frame-zero only (`stateCount != 0` returns), so all geometry
//!   appears on the first frame;
//! * `kind` selects only the slot's `distance`/`flag1[4]` (5/6/7); the
//!   renderer's kind 6/7 branch adds the 1.1x outer pass and kind 5 does not;
//! * the body never calls `FaceCamera`, so the shared `RenderHaloLayers`
//!   rotates by the constructor's zeroed `slots[0..1].alphaT` angles
//!   (`RenderHaloLayers` is otherwise prim 68's billboarded renderer).

use ragnarok_packets::EffectId;

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedSlam` - `RagEffect.cpp:4264-4305`.
///
/// Arms one prim 147: render flag `RSF_ALPHA | RSF_CUSTOM_BLEND`, one texture,
/// `size = 0`, one live slot at `process = 0`, `height[0] = 0`,
/// `alphaB = 0`, `flag1[0] = 0` and `height[1..21] = 0`.
fn textured_slam(runtime: &mut EffectRuntime, texture: &'static str, kind: i16) {
    // `const vector3d& origin = parent.position; this->render.position = origin;`
    // - `runtime.position` already is that anchor.

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let index = runtime.launch(PrimitiveId::TexturedSlam, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = Some(texture);
    state.size = 0.0;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    state.slots[0].life = 1;
    state.slots[0].process = 0;

    if kind == 1 {
        state.slots[0].distance = 21.0;
        state.slots[0].flag1[4] = 6;
    } else if kind == 2 {
        state.slots[0].distance = 21.0;
        state.slots[0].flag1[4] = 7;
    } else {
        state.slots[0].distance = 13.0;
        state.slots[0].flag1[4] = 5;
    }

    state.slots[0].height[0] = 0.0;
    state.slots[0].alpha_b = 0;
    state.slots[0].flag1[0] = 0;

    for step in 1..21usize {
        state.slots[0].height[step] = 0.0;
    }
}

/// One `EF_*` id that dispatches to [`textured_slam`], with its official
/// lifetime (`EffectTypes.cpp`).
#[derive(Debug)]
pub struct TexturedSlam {
    texture: &'static str,
    kind: i16,
    lifetime: i32,
}

impl TexturedSlam {
    const fn new(texture: &'static str, kind: i16, lifetime: i32) -> Self {
        Self { texture, kind, lifetime }
    }
}

impl PortedEffect for TexturedSlam {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        textured_slam(runtime, self.texture, self.kind);

        true
    }
}

/// The ten `EF_ROLLING1..10` textures, in id order
/// (`RagEffect.cpp:20496-20529`).
pub const ROLLING_TEXTURES: [&str; 10] = [
    "effect\\회전카운터1.tga",
    "effect\\회전카운터2.tga",
    "effect\\회전카운터3.tga",
    "effect\\회전카운터4.tga",
    "effect\\회전카운터5.tga",
    "effect\\회전카운터6.tga",
    "effect\\회전카운터7.tga",
    "effect\\회전카운터8.tga",
    "effect\\회전카운터9.tga",
    "effect\\회전카운터10.tga",
];

/// Creates the ported body for a `TexturedSlam` effect id.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    Some(match effect {
        // `EffectTypes.cpp:868`, `:871`.
        EffectId::Secra => Box::new(TexturedSlam::new("effect\\priest_spell.bmp", 1, 300)),
        EffectId::Secra2 => Box::new(TexturedSlam::new("effect\\priest_spell.bmp", 2, 300)),
        // `EffectTypes.cpp:767-776`.
        EffectId::Rolling1 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[0], 0, 99_999_999)),
        EffectId::Rolling2 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[1], 0, 99_999_999)),
        EffectId::Rolling3 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[2], 0, 99_999_999)),
        EffectId::Rolling4 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[3], 0, 99_999_999)),
        EffectId::Rolling5 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[4], 0, 99_999_999)),
        EffectId::Rolling6 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[5], 0, 99_999_999)),
        EffectId::Rolling7 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[6], 0, 99_999_999)),
        EffectId::Rolling8 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[7], 0, 99_999_999)),
        EffectId::Rolling9 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[8], 0, 99_999_999)),
        EffectId::Rolling10 => Box::new(TexturedSlam::new(ROLLING_TEXTURES[9], 0, 99_999_999)),
        // `EffectTypes.cpp:888`.
        EffectId::Heartasura => Box::new(TexturedSlam::new("effect\\kiss2.bmp", 2, 99_999_999)),
        _ => return None,
    })
}

/// `EffectLifetime` for every id [`spawn`] accepts (`EffectTypes.cpp`).
pub fn lifetimes() -> &'static [(EffectId, i32)] {
    &[
        // `EffectTypes.cpp:767-776`.
        (EffectId::Rolling1, 99_999_999),
        (EffectId::Rolling2, 99_999_999),
        (EffectId::Rolling3, 99_999_999),
        (EffectId::Rolling4, 99_999_999),
        (EffectId::Rolling5, 99_999_999),
        (EffectId::Rolling6, 99_999_999),
        (EffectId::Rolling7, 99_999_999),
        (EffectId::Rolling8, 99_999_999),
        (EffectId::Rolling9, 99_999_999),
        (EffectId::Rolling10, 99_999_999),
        // `EffectTypes.cpp:868`, `:871`.
        (EffectId::Secra, 300),
        (EffectId::Secra2, 300),
        // `EffectTypes.cpp:888`.
        (EffectId::Heartasura, 99_999_999),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    fn harness(effect: &dyn PortedEffect) -> EffectRuntime {
        EffectRuntime::new(effect.lifetime(), [1.0, 2.0], 5)
    }

    #[test]
    fn entries_match_their_official_lifetimes() {
        for &(effect, lifetime) in lifetimes() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn secra_arms_one_halo_layer_fan() {
        let mut body = spawn(EffectId::Secra).unwrap();
        let mut runtime = harness(body.as_ref());

        step(body.as_mut(), &mut runtime);

        let primitive = &runtime.primitives[0].state;

        assert_eq!(primitive.count, PrimitiveId::TexturedSlam);
        assert_eq!(primitive.count.id(), 147);
        assert_eq!(primitive.render_flag, 5);
        assert_eq!(primitive.texture, Some("effect\\priest_spell.bmp"));
        assert_eq!(primitive.total_texture, 1);
        assert_eq!(primitive.size, 0.0);
        assert_eq!(primitive.slots[0].life, 1);
        assert_eq!(primitive.slots[0].process, 1);
        // `PrimRisingFan` closes the radius by 0.2 while `process < 50`.
        assert_eq!(primitive.slots[0].distance, 20.8);
        assert_eq!(primitive.slots[0].flag1[4], 6);
        assert_eq!(primitive.slots[0].alpha_b, 0);
        assert_eq!(primitive.slots[0].flag1[0], 0);
        assert_eq!(primitive.slots[0].height[0], 0.0);
        assert_eq!(primitive.slots[0].height[1], 10.0);
        assert_eq!(primitive.slots[0].height[10], 10.0);
        assert_eq!(primitive.slots[1].life, 0);
    }

    #[test]
    fn secra2_and_heart_asura_use_kind_two() {
        for (effect, texture) in [
            (EffectId::Secra2, "effect\\priest_spell.bmp"),
            (EffectId::Heartasura, "effect\\kiss2.bmp"),
        ] {
            let mut body = spawn(effect).unwrap();
            let mut runtime = harness(body.as_ref());

            body.on_process(&mut runtime);

            let slot = &runtime.primitives[0].state.slots[0];
            assert_eq!(slot.distance, 21.0, "{effect:?}");
            assert_eq!(slot.flag1[4], 7, "{effect:?}");
            assert_eq!(runtime.primitives[0].state.texture, Some(texture), "{effect:?}");
        }
    }

    #[test]
    fn rolling_uses_kind_zero_at_distance_thirteen() {
        let mut body = spawn(EffectId::Rolling1).unwrap();
        let mut runtime = harness(body.as_ref());

        body.on_process(&mut runtime);

        let primitive = &runtime.primitives[0].state;
        assert_eq!(primitive.texture, Some(ROLLING_TEXTURES[0]));
        assert_eq!(primitive.slots[0].distance, 13.0);
        assert_eq!(primitive.slots[0].flag1[4], 5);

        let mut last = spawn(EffectId::Rolling10).unwrap();
        let mut runtime = harness(last.as_ref());
        last.on_process(&mut runtime);
        assert_eq!(runtime.primitives[0].state.texture, Some(ROLLING_TEXTURES[9]));
    }

    #[test]
    fn the_body_is_frame_zero_only() {
        let mut body = spawn(EffectId::Secra).unwrap();
        let mut runtime = harness(body.as_ref());

        step(body.as_mut(), &mut runtime);
        step(body.as_mut(), &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
    }
}
