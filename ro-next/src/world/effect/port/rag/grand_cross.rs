//! `EF_GRANDCROSS` (226) and `EF_GRANDCROSS2` (450) - the two Grand Cross
//! bodies, `CRagEffect::GrandCrossRings(short angle, char kind)`
//! (`RagEffect.cpp:16197-16267`) and `CRagEffect::GRANDCROSS2(char flag)`
//! (`RagEffect.cpp:5163-5195`), dispatched together at
//! `RagEffect.cpp:24203-24210`.
//!
//! | effect | call | kind |
//! | --- | --- | --- |
//! | `EF_GRANDCROSS` | `GrandCrossRings(0, 0); GRANDCROSS2(0);` | 0 |
//! | `EF_GRANDCROSS2` | `GrandCrossRings(0, 1); GRANDCROSS2(1);` | 1 |
//!
//! Both bodies open with `CopyVector(this->render.position,
//! this->state.parent->render.position)` and then return on every frame but
//! frame 0, so each effect launches exactly two primitives once:
//!
//! * prim 34 (`PP_GI_3`, [`super::super::primitive::gi3`]) - four contracting
//!   rings. Kind 0 uses `effect\ring_white.tga` at `size = 1`, kind 1
//!   `effect\ring_black.tga` at `size = 12`.
//! * prim 37 (`PP_GRANDCROSS2`, [`super::super::primitive::grand_cross2`]) -
//!   two flat cross quads. Kind 0 uses `effect\ring_yellow.tga`, kind 1
//!   `effect\ring_black.tga`.
//!
//! `GrandCrossRings` also plays `effect\cru_grand cross.wav` on frame 0.
//! `renderFlag` is written as `0`, then `|= RSF_ALPHA`, then
//! `|= RSF_ALPHA | RSF_CUSTOM_BLEND` - the final `5`, but the three writes are
//! kept because that is what the decompilation does.
//!
//! Renderers:
//! [`render_3d_casting2`](super::super::render_slots::render_3d_casting2)
//! (`Render3DCasting2`, `EffectPrimitiveRendering.cpp:6519-6592`) and
//! [`render_grand_cross2`](super::super::render_slots::render_grand_cross2)
//! (`RenderGrandCross2`, `EffectPrimitiveRendering.cpp:7365-7421`).
//!
//! Lifetimes: `EF_GRANDCROSS` 999 (`EffectTypes.cpp:235`), `EF_GRANDCROSS2`
//! 999 (`:459`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The two-call `EF_GRANDCROSS` body; `kind` is the dispatch's second argument.
#[derive(Debug)]
pub struct GrandCross {
    kind: i8,
    lifetime: i32,
}

impl GrandCross {
    pub const fn new(kind: i8, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

/// `CRagEffect::GrandCrossRings(short angle, char kind)` -
/// `RagEffect.cpp:16197-16267`.
fn grand_cross_rings(runtime: &mut EffectRuntime, angle: i16, kind: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    runtime.play_sound("effect\\cru_grand cross.wav");

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(34, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::Gi3, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    // `state.renderFlag = 0; state.renderFlag |= RSF_ALPHA;
    //  state.renderFlag |= RSF_ALPHA | RSF_CUSTOM_BLEND;`
    state.render_flag = 0;
    state.render_flag |= 1;
    state.render_flag |= 1 | 4;
    state.duration = lifetime;

    if kind == 0 {
        state.texture = Some("effect\\ring_white.tga");
        state.size = 1.0;
    } else {
        state.texture = Some("effect\\ring_black.tga");
        state.size = 12.0;
    }

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = 0;
        slot.full_display_angle = 90;
        slot.max_height = 120.0;
        slot.alpha_b = angle;
        slot.distance = 19.9;
        slot.rise_angle = 90;
        slot.flag1[4] = kind;
    }

    const ROT_STARTS: [i16; 4] = [180, 90, 0, 270];
    const FLAG_X: [i8; 4] = [23, 23, -23, -23];
    const FLAG_Z: [i8; 4] = [23, -23, -23, 23];

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.rot_start = ROT_STARTS[index];
        slot.flag1[0] = FLAG_X[index];
        slot.flag1[1] = FLAG_Z[index];
    }
}

/// `CRagEffect::GRANDCROSS2(char flag)` - `RagEffect.cpp:5163-5195`.
fn grand_cross2(runtime: &mut EffectRuntime, flag: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(37, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::GrandCross2, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 0;
    state.render_flag |= 1;
    state.render_flag |= 1 | 4;
    state.duration = lifetime;

    state.texture = Some(if flag == 0 {
        "effect\\ring_yellow.tga"
    } else {
        "effect\\ring_black.tga"
    });
    state.size = 1.0;

    for index in 0..2usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = 0;
        slot.max_height = 60.0;
        slot.rot_start = (index * 90) as i16;
        slot.alpha_b = 0;
        slot.height[0] = 24.0;
        slot.height[1] = 3.2;
        slot.height[2] = 0.0;
    }

    state.slots[2].life = 0;
    state.slots[3].life = 0;
}

impl PortedEffect for GrandCross {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `GrandCrossRings(0, kind)` then `GRANDCROSS2(kind)`.
        grand_cross_rings(runtime, 0, self.kind);
        grand_cross2(runtime, self.kind);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::angles::Angles;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn grand_cross_launches_the_white_rings_and_the_yellow_cross() {
        let mut effect = GrandCross::new(0, 999);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 3);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\cru_grand cross.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        let rings = &runtime.primitives[0].state;
        assert_eq!(rings.count, PrimitiveId::Gi3);
        assert_eq!(rings.texture, Some("effect\\ring_white.tga"));
        assert_eq!(rings.size, 1.0);
        assert_eq!(rings.render_flag, 5);
        assert_eq!(rings.duration, 999);
        assert_eq!(rings.slots[0].full_display_angle, 90);
        assert_eq!(rings.slots[0].max_height, 120.0);
        assert_eq!(rings.slots[0].rise_angle, 90);
        assert_eq!(rings.slots[0].flag1[4], 0);
        // The motion runs in the same step: process 1, alpha +10, distance
        // -0.01.
        assert_eq!(rings.slots[0].process, 1);
        assert_eq!(rings.slots[0].alpha_b, 10);
        assert!((rings.slots[0].distance - 19.89).abs() < 1e-5);

        let rot_starts: Vec<i16> = rings.slots.iter().map(|slot| slot.rot_start).collect();
        assert_eq!(rot_starts, vec![180, 90, 0, 270]);
        let flag_x: Vec<i8> = rings.slots.iter().map(|slot| slot.flag1[0]).collect();
        assert_eq!(flag_x, vec![23, 23, -23, -23]);
        let flag_z: Vec<i8> = rings.slots.iter().map(|slot| slot.flag1[1]).collect();
        assert_eq!(flag_z, vec![23, -23, -23, 23]);

        let cross = &runtime.primitives[1].state;
        assert_eq!(cross.count, PrimitiveId::GrandCross2);
        assert_eq!(cross.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(cross.size, 1.0);
        assert_eq!(cross.render_flag, 5);
        assert_eq!(cross.slots[0].life, 1);
        assert_eq!(cross.slots[1].life, 1);
        assert_eq!(cross.slots[2].life, 0);
        assert_eq!(cross.slots[3].life, 0);
        assert_eq!(cross.slots[1].rot_start, 90);
        assert_eq!(cross.slots[0].max_height, 60.0);
        // Motion: process 1 -> alpha 9, heights +0.005 and a 60*sin(1) z.
        let mut expected_height = 24.0f32;
        expected_height += 0.005;
        let mut expected_side = 3.2f32;
        expected_side += 0.005;
        assert_eq!(cross.slots[0].process, 1);
        assert_eq!(cross.slots[0].alpha_b, 9);
        assert_eq!(cross.slots[0].height[0], expected_height);
        assert_eq!(cross.slots[0].height[1], expected_side);
        assert_eq!(cross.slots[0].height[2], Angles::shared().raw_sine(1) * 60.0);
    }

    #[test]
    fn grand_cross2_uses_black_rings_at_size_twelve() {
        let mut effect = GrandCross::new(1, 999);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 3);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\cru_grand cross.wav".to_owned()]);

        let rings = &runtime.primitives[0].state;
        assert_eq!(rings.texture, Some("effect\\ring_black.tga"));
        assert_eq!(rings.size, 12.0);
        assert_eq!(rings.slots[0].flag1[4], 1);

        let cross = &runtime.primitives[1].state;
        assert_eq!(cross.texture, Some("effect\\ring_black.tga"));
    }

    #[test]
    fn later_frames_launch_nothing() {
        let mut effect = GrandCross::new(0, 999);
        let mut runtime = EffectRuntime::new(999, [0.0, 0.0], 3);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);
    }
}
