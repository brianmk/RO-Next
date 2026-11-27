//! `CRagEffect::ThrowItem(const char* name, char kind)` -
//! `RagEffect.cpp:12182-12272`.
//!
//! One `PP_THROWITEM` (53) primitive, armed on a frame that depends on
//! `kind`:
//!
//! | kind | target frame | texture (dispatch id) |
//! | --- | --- | --- |
//! | 0 | 0 | `유저인터페이스\item\빈병.bmp` (`EF_THROWITEM`, 298) |
//! | 1 | 0 | `유저인터페이스\item\빈포션병.bmp` (`EF_THROWITEM2`, 299), `...\버서크포션.bmp` (`EF_THROWITEM5`, 541), `...\폭탄버섯포자.bmp` (`EF_THROWITEM11`, 914) |
//! | 2 | 0 | `유저인터페이스\item\돌.bmp` (`EF_THROWITEM3`, 308) |
//! | 3 | 0 | knives/shuriken/coins (`EF_THROWITEM6`-`EF_THROWITEM10`, 600/613-616) |
//! | 10 | 5 | `유저인터페이스\item\화염병.bmp` (`EF_THROWITEM4`, 539, first call) |
//! | 11 | 10 | `유저인터페이스\item\염산병.bmp` (`EF_THROWITEM4`, 539, second call) |
//!
//! All dispatch blocks are the body alone; `EF_THROWITEM4` calls the body
//! twice, hence [`DoubleThrowItem`]. Lifetimes are 300 for `EF_THROWITEM`,
//! `EF_THROWITEM2`, `EF_THROWITEM4`, `EF_THROWITEM5` and `EF_THROWITEM11`, and
//! 200 for `EF_THROWITEM3` and `EF_THROWITEM6..10`
//! (`EffectTypes.cpp:307-308`, `:317`, `:548-550`, `:609`, `:622-625`, `:923`).
//!
//! The primitive's motion is `primitive/throw_item.rs`; the renderer is the
//! shared `RenderFieldPanel`.
//!
//! Quirk kept: `vecBNow.y` starts at `0.0f` while `vecBPre.y` starts at
//! `position.y - 8.0f`, so the first motion frame snaps the item up to the
//! arc. The dispatch sets `flag1[3] = this->state.count` only for `kind == 1`.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// One `CRagEffect::ThrowItem(name, kind)` call.
#[derive(Debug)]
pub struct ThrowItem {
    name: &'static str,
    kind: i8,
    lifetime: i32,
}

impl ThrowItem {
    pub fn new(name: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { name, kind, lifetime }
    }
}

impl PortedEffect for ThrowItem {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::ThrowItem` - `RagEffect.cpp:12182-12272`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let kind = self.kind;

        // `int targetFrame; if (kind == 10) targetFrame = 5; else if (kind ==
        // 11) targetFrame = 10; else targetFrame = 0;`
        let target_frame = if kind == 10 {
            5
        } else if kind == 11 {
            10
        } else {
            0
        };

        if runtime.frame != target_frame {
            return true;
        }

        let position = EffectRuntime::official_position(runtime.position);
        let target = EffectRuntime::official_position(runtime.target_offset);
        let dx = target[0] - position[0];
        let dz = target[2] - position[2];
        let max_height = (dx * dx + dz * dz).sqrt();
        let lifetime = runtime.lifetime;
        let count = runtime.count;

        let index = runtime.launch(PrimitiveId::ThrowItem, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.name);
        state.size = 0.0;
        state.delta_pos = target;

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.process = 0;
        slot.max_height = max_height;

        slot.distance = if kind == 2 {
            2.0
        } else if kind == 3 {
            4.0
        } else {
            3.0
        };

        slot.height[8] = 0.0;
        slot.alpha_b = 255;
        slot.vec_b_now[0] = position[0];
        slot.vec_b_now[2] = position[2];
        // Quirk: the horizontal start is the parent position but the vertical
        // start is the world origin; `vecBPre.y` carries the real height.
        slot.vec_b_now[1] = 0.0;
        slot.vec_b_pre[1] = position[1] - 8.0;
        slot.rot_start = 0;

        slot.height[1] = if kind == 2 {
            2.0
        } else if kind == 3 {
            3.0
        } else {
            slot.max_height / 25.0
        };

        slot.height[2] = (slot.height[1] / slot.max_height) * (target[1] - position[1]);

        if kind == 3 {
            slot.full_display_angle = 330;
            slot.flag1[0] = 0;
            slot.height[7] = 0.0;
            slot.flag1[2] = 15;
        } else {
            slot.full_display_angle = 0;
            slot.flag1[0] = 0;
            slot.height[7] = 0.0;

            if kind == 0 {
                slot.flag1[2] = 8;
            } else if kind == 1 {
                slot.flag1[2] = 9;
                // `flag1[3] = static_cast<char>(this->state.count);`
                slot.flag1[3] = count as i8;
            } else if kind >= 10 {
                slot.flag1[2] = 9;
                slot.flag1[3] = 4;
            } else if kind == 2 {
                slot.flag1[2] = 0;
            }
        }

        // `for (stepIdx = 0; stepIdx < 100; ++stepIdx) if (maxHeight <
        // stepIdx * height[1]) { flag1[1] = stepIdx; break; }`
        for step_index in 0..100i32 {
            if slot.max_height < step_index as f32 * slot.height[1] {
                slot.flag1[1] = step_index as i8;
                break;
            }
        }

        state.slots[1].life = 1;
        state.slots[1].alpha_b = 0;
        state.slots[2].life = 1;
        state.slots[2].alpha_b = 0;
        state.slots[3].life = 1;
        state.slots[3].alpha_b = 0;

        true
    }
}

/// `EF_THROWITEM4`'s dispatch block (`RagEffect.cpp:21359-21362`): two
/// `ThrowItem` calls per frame, one at target frame 5 and one at target frame
/// 10. Each call keeps its own gate, so it is exactly two independent bodies.
#[derive(Debug)]
pub struct DoubleThrowItem {
    first: ThrowItem,
    second: ThrowItem,
    lifetime: i32,
}

impl DoubleThrowItem {
    pub fn new(first: ThrowItem, second: ThrowItem, lifetime: i32) -> Self {
        Self { first, second, lifetime }
    }
}

impl PortedEffect for DoubleThrowItem {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `EF_THROWITEM4`'s two-call dispatch - `RagEffect.cpp:21359-21362`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let first = self.first.on_process(runtime);
        let second = self.second.on_process(runtime);
        first && second
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::EffectRuntime;

    fn runtime(lifetime: i32, count: i32) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], 1);
        runtime.position = [3.0, 4.0, 5.0];
        runtime.target_offset = [3.0, 9.0, 15.0];
        runtime.count = count;
        runtime
    }

    /// Runs the body once with `runtime.frame` set to `frame`, without the
    /// runtime's primitive pass. This is the exact body state at its spawn
    /// frame; `step` would immediately advance the spawned primitive's motion.
    fn spawn_at(effect: &mut ThrowItem, frame: i32) -> EffectRuntime {
        let mut runtime = runtime(effect.lifetime(), 0);
        runtime.frame = frame;
        effect.on_process(&mut runtime);
        runtime
    }

    #[test]
    fn kind_zero_arms_the_thrown_item_on_frame_zero() {
        let mut effect = ThrowItem::new("유저인터페이스\\item\\빈병.bmp", 0, 300);

        let runtime = spawn_at(&mut effect, 0);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ThrowItem);
        assert_eq!(state.count.id(), 53);
        assert_eq!(state.duration, 300);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.texture, Some("유저인터페이스\\item\\빈병.bmp"));
        assert_eq!(state.size, 0.0);
        // The target is converted to official (Y-down) simulation space.
        assert_eq!(state.delta_pos, [3.0, -9.0, 15.0]);

        let slot = &state.slots[0];
        // The target/offset vector is converted to official (Y-down) space.
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 0);
        assert_eq!(slot.max_height, 10.0);
        assert_eq!(slot.distance, 3.0);
        assert_eq!(slot.height[8], 0.0);
        assert_eq!(slot.alpha_b, 255);
        assert_eq!(slot.vec_b_now, [3.0, 0.0, 5.0], "y starts at the origin");
        // Official Y-down: the port anchor's y 4 is -4, then the -8 lift.
        assert_eq!(slot.vec_b_pre[1], -12.0, "y = official position.y - 8");
        assert_eq!(slot.rot_start, 0);
        assert_eq!(slot.height[1], 10.0 / 25.0);
        // `height[2] = height[1]/maxHeight * (target.y - position.y)`, both in
        // official (Y-down) space: -9 - (-4) = -5.
        assert_eq!(slot.height[2], (10.0 / 25.0) / 10.0 * -5.0);
        assert_eq!(slot.full_display_angle, 0);
        assert_eq!(slot.flag1[2], 8);
        // 10 < stepIdx * 0.4 first holds at stepIdx = 26 (10 < 10.4).
        assert_eq!(slot.flag1[1], 26);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 0);
        }
    }

    #[test]
    fn kind_one_writes_the_effect_count_into_flag1_slot_three() {
        let mut effect = ThrowItem::new("유저인터페이스\\item\\빈포션병.bmp", 1, 300);

        let runtime = {
            let mut runtime = runtime(300, 42);
            effect.on_process(&mut runtime);
            runtime
        };

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.distance, 3.0);
        assert_eq!(slot.flag1[2], 9);
        assert_eq!(slot.flag1[3], 42);
    }

    #[test]
    fn kind_two_uses_the_stone_distance_and_twice_the_reach_step() {
        let mut effect = ThrowItem::new("유저인터페이스\\item\\돌.bmp", 2, 200);

        let runtime = spawn_at(&mut effect, 0);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.distance, 2.0, "the body's kind-2 distance");
        assert_eq!(slot.height[1], 2.0);
        assert_eq!(slot.flag1[2], 0);
        // 10 < 2 * stepIdx first holds at stepIdx = 6 (10 < 12).
        assert_eq!(slot.flag1[1], 6);
    }

    #[test]
    fn kind_three_uses_the_shuriken_spin_and_distance() {
        let mut effect = ThrowItem::new("유저인터페이스\\item\\수리검.bmp", 3, 200);

        let runtime = spawn_at(&mut effect, 0);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.distance, 4.0);
        assert_eq!(slot.height[1], 3.0);
        assert_eq!(slot.full_display_angle, 330);
        assert_eq!(slot.flag1[2], 15);
        // 10 < 3 * stepIdx first holds at stepIdx = 4 (10 < 12).
        assert_eq!(slot.flag1[1], 4);
    }

    #[test]
    fn kind_ten_waits_for_frame_five() {
        let mut effect = ThrowItem::new("유저인터페이스\\item\\화염병.bmp", 10, 300);

        assert!(spawn_at(&mut effect, 4).primitives.is_empty(), "no spawn before frame 5");

        let runtime = spawn_at(&mut effect, 5);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.slots[0].distance, 3.0);
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[2], 9);
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[3], 4);
    }

    #[test]
    fn kind_eleven_waits_for_frame_ten() {
        let mut effect = ThrowItem::new("유저인터페이스\\item\\염산병.bmp", 11, 300);

        assert!(spawn_at(&mut effect, 9).primitives.is_empty(), "no spawn before frame 10");

        let runtime = spawn_at(&mut effect, 10);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.slots[0].flag1[3], 4);
    }

    #[test]
    fn double_throw_item_runs_both_calls_on_their_own_frames() {
        let mut effect = DoubleThrowItem::new(
            ThrowItem::new("유저인터페이스\\item\\화염병.bmp", 10, 300),
            ThrowItem::new("유저인터페이스\\item\\염산병.bmp", 11, 300),
            300,
        );
        let mut runtime = runtime(300, 0);

        runtime.frame = 5;
        effect.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        runtime.frame = 10;
        effect.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.primitives[0].state.texture, Some("유저인터페이스\\item\\화염병.bmp"));
        assert_eq!(runtime.primitives[1].state.texture, Some("유저인터페이스\\item\\염산병.bmp"));
    }

    #[test]
    fn a_throw_item_does_not_spawn_twice() {
        let mut effect = ThrowItem::new("effect\\coin_a.bmp", 0, 200);

        let mut runtime = runtime(200, 0);

        effect.on_process(&mut runtime);
        runtime.frame = 1;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
    }
}
