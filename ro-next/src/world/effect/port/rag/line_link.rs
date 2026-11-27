//! `CRagEffect::TrackTarget` / `TrackTargetWide` - `RagEffect.cpp:3839-3878`
//! and `:3884-3929`; dispatch `case EF_LINELINK2:` / `case EF_LINELINK3:`
//! `RagEffect.cpp:24421-24428`.
//!
//! The two soul-link bodies. `EF_LINELINK2` (`PF_SOULCHANGE`) uses
//! `TrackTarget` (kind 2, `maxHeight` 0.2); `EF_LINELINK3` (`CG_MARIONETTE`)
//! uses `TrackTargetWide` (kind 3, `maxHeight` 0.7). The dispatch plays
//! `effect\\소울 체인지.wav` and copies the parent anchor on frame 0, then the
//! body arms one **prim 38** (`PP_LINELINK`) primitive:
//!
//! * `renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND`, texture
//!   `effect\\alpha_center.tga`, `distance = 0`,
//! * `slot0.life = 1`, `process = 0`, `alphaB = 0`, `height[4]` = 2/3 (the
//!   motion's kind), `vecTNow = render.position`,
//! * `height[0] = state.deltaPos.y` - the target's account id, which the server
//!   packs into the middle `deltaPos` slot (`GameModePackets.cpp:19072-19075`,
//!   `:19131-19134`),
//! * when that value is positive the body resolves the actor; a null lookup
//!   leaves `life = 0` (the link is not drawn). The port's spawn bridge stores
//!   the *resolved target position* in `deltaPos` (like every travelling body;
//!   `client/effects.rs`), so the body records that position in `vecTPre` and
//!   the motion re-reads the same `deltaPos` every frame. The client packet's
//!   `[0, accountId, 0]` decode (`AM_SETPOS`) is therefore already resolved by
//!   the time the body runs.
//!
//! Slots 1..3 are explicitly killed. `PrimLineLink` re-resolves the target
//! every frame and draws the tube with
//! [`render_tube`](super::super::render_slots::render_tube) (`RenderTube`,
//! `EffectPrimitiveRendering.cpp:3214-3310`, dispatch `:690-691`).
//!
//! Lifetimes: `EF_LINELINK2` 200 (`EffectTypes.cpp:393`), `EF_LINELINK3`
//! 99999999 (`:404`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// One of the two soul-link dispatches; `wide` selects `TrackTargetWide`.
#[derive(Debug)]
pub struct LineLink {
    wide: bool,
    lifetime: i32,
}

impl LineLink {
    pub const fn new(wide: bool, lifetime: i32) -> Self {
        Self { wide, lifetime }
    }
}

/// `CRagEffect::TrackTarget()` - `RagEffect.cpp:3839-3878`.
fn track_target(runtime: &mut EffectRuntime) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = EffectRuntime::official_position(runtime.position);
    // `height[0] = state.deltaPos.y` is the **target AID** in the original
    // (`RagEffect.cpp:3861-3868`: `GetGameActorByAID(height[0])`), not a
    // height - it must not be flipped with the world axis.
    let target_y = runtime.target_offset[1];
    let target_position = EffectRuntime::official_position(runtime.target_offset);

    // `this->state.lastPrim = LaunchEffectPrim(38, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::LineLink, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some("effect\\alpha_center.tga");

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = 0;
    slot.alpha_b = 0;
    slot.distance = 0.0;
    slot.max_height = 0.2;
    slot.height[4] = 2.0;
    slot.vec_t_now = position;
    slot.height[0] = target_y;

    // `if (height[0] > 0.0f)`: resolve the actor. A null target kills the slot.
    let target = if slot.height[0] > 0.0 { Some(target_position) } else { None };

    if let Some(target) = target {
        slot.vec_t_pre = target;
    } else {
        slot.life = 0;
    }

    state.slots[1].life = 0;
    state.slots[2].life = 0;
    state.slots[3].life = 0;
}

/// `CRagEffect::TrackTargetWide()` - `RagEffect.cpp:3884-3929`.
fn track_target_wide(runtime: &mut EffectRuntime) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = EffectRuntime::official_position(runtime.position);
    // `height[0] = state.deltaPos.y` is the **target AID** in the original
    // (`RagEffect.cpp:3861-3868`: `GetGameActorByAID(height[0])`), not a
    // height - it must not be flipped with the world axis.
    let target_y = runtime.target_offset[1];
    let target_position = EffectRuntime::official_position(runtime.target_offset);

    let index = runtime.launch(PrimitiveId::LineLink, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some("effect\\alpha_center.tga");

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = 0;
    slot.alpha_b = 0;
    slot.distance = 0.0;
    slot.max_height = 0.7;
    slot.height[4] = 3.0;
    slot.vec_t_now = position;
    slot.height[0] = target_y;

    let target = if slot.height[0] > 0.0 { Some(target_position) } else { None };

    if let Some(target) = target {
        slot.vec_t_pre = target;
    } else {
        slot.life = 0;
    }

    state.slots[1].life = 0;
    state.slots[2].life = 0;
    state.slots[3].life = 0;
}

impl PortedEffect for LineLink {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `case EF_LINELINK2:` / `case EF_LINELINK3:` - `RagEffect.cpp:24421-24433`.
        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("effect\\소울 체인지.wav");
        }

        if self.wide {
            track_target_wide(runtime);
        } else {
            track_target(runtime);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    const TARGET: [f32; 3] = [50.0, 60.0, 70.0];

    fn runtime(lifetime: i32) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], 1);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);
        runtime.target_offset = TARGET;
        runtime
    }

    #[test]
    fn linelink_two_arms_the_narrow_tube() {
        let mut effect = LineLink::new(false, 200);
        let mut runtime = runtime(200);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\소울 체인지.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::LineLink);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.duration, 200);
        assert_eq!(state.texture, Some("effect\\alpha_center.tga"));

        // PrimLineLink runs in the same step: process 1, vecTNow trails the
        // master by 8, vecTPre trails the target by 8, kind-2 alpha +6.
        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 1);
        assert_eq!(slot.height[0], TARGET[1]);
        assert_eq!(slot.height[4], 2.0);
        assert_eq!(slot.max_height, 0.2);
        // The slot vectors are official (Y-down). `PrimLineLink` copies the
        // master/target and then does `y -= 8`, which is eight units *up* in
        // the official world.
        assert_eq!(slot.vec_t_now, [10.0, -28.0, 30.0]);
        assert_eq!(slot.vec_t_pre, [50.0, -68.0, 70.0]);
        assert_eq!(slot.alpha_b, 6);

        assert_eq!(state.slots[1].life, 0);
        assert_eq!(state.slots[2].life, 0);
        assert_eq!(state.slots[3].life, 0);
    }

    #[test]
    fn linelink_three_arms_the_wide_tube() {
        let mut effect = LineLink::new(true, 99_999_999);
        let mut runtime = runtime(99_999_999);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.duration, 99_999_999);
        assert_eq!(state.slots[0].height[4], 3.0);
        // PrimLineLink's kind-3 branch swells the radius in the same step:
        // 0.7 + sin(process % 360) * 0.005.
        let expected = 0.7 + crate::world::effect::port::angles::Angles::shared().raw_sine(1) * 0.005;
        assert!((state.slots[0].max_height - expected).abs() < 1e-6);
        assert_eq!(state.slots[0].alpha_b, 1);
    }

    #[test]
    fn a_zero_target_offset_kills_the_slot_at_spawn() {
        let mut effect = LineLink::new(false, 200);
        let mut runtime = runtime(200);
        // No target: the runtime's `deltaPos` is still the constructor zero.
        runtime.target_offset = [0.0, 0.0, 0.0];

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.slots[0].height[0], 0.0);
        assert_eq!(runtime.primitives[0].state.slots[0].life, 0);
        // `vecTPre` keeps the constructor's zero.
        assert_eq!(runtime.primitives[0].state.slots[0].vec_t_pre, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_zero_target_height_skips_the_lookup() {
        let mut effect = LineLink::new(false, 200);
        let mut runtime = runtime(200);
        // A ground-level target position leaves `height[0] == 0`; the original's
        // `height[0] > 0` gate then drops the link.
        runtime.target_offset = [50.0, 0.0, 70.0];

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.slots[0].height[0], 0.0);
        assert_eq!(runtime.primitives[0].state.slots[0].life, 0);
    }

    #[test]
    fn only_frame_zero_arms_and_plays_the_wave() {
        let mut effect = LineLink::new(false, 200);
        let mut runtime = runtime(200);

        step(&mut effect, &mut runtime);
        runtime.sounds.clear();

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        assert!(runtime.sounds.is_empty());
        // The motion keeps moving the slot every frame.
        assert_eq!(runtime.primitives[0].state.slots[0].process, 6);
    }
}
