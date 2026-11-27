//! `EF_HITLINE4` (429), `EF_HITLINE5` (430), `EF_HITLINE6` (431) - the
//! `WhiteSpark*` bodies that spawn `PP_HITLINE3` (prim 100).
//!
//! Dispatch (`RagEffect.cpp:23651-23660`):
//!
//! | effect | body | kind |
//! | --- | --- | --- |
//! | `EF_HITLINE4` | `White02Burst(kind)` - `RagEffect.cpp:16154-16199` | 3 |
//! | `EF_HITLINE5` | `WhiteSparkRing(kind)` - `RagEffect.cpp:13388-13446` | 3 |
//! | `EF_HITLINE6` | `WhiteSparkStreak(kind)` - `RagEffect.cpp:13951-14063` | 3 |
//!
//! (`EF_HITLINE3` itself calls `WhiteSparkStreak(0)`, whose `limit = 0` gate
//! makes `stateCnt >= 0` always true - the body is dead - so that id is not
//! registered. The kind-0/1 branches stay in this module because they are part
//! of the same three C++ functions and other call sites use them.)
//!
//! All three bodies load `effect\white02.bmp` into a prim 100 whose four slots
//! carry a `process` delay, an `alphaB` seed and a `height[0..vecTNow.y]`
//! sample history that the [`super::super::primitive::hitline3`] motion shifts
//! and `RenderHitLine3` sweeps. The RNG draws are made per slot in the exact
//! order of the original before the primitive is written, because the port
//! cannot borrow the runtime's RNG through the primitive state.

use super::super::primitive::{EffectPrimitiveState, PrimitiveId};
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `RSF_ALPHA | RSF_CUSTOM_BLEND` (`RenderQueues.h:11-13`).
const RENDER_FLAG: u32 = 1 | 4;

/// The common `LaunchEffectPrim(100, ...)` shape: duration, one white02
/// texture, `size = 0`, the alpha/custom-blend queue.
fn launch_white02(runtime: &mut EffectRuntime, keep_delta_pos: bool) -> usize {
    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);

    let index = runtime.launch(PrimitiveId::HitLine3, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = RENDER_FLAG;
    state.duration = lifetime;
    state.total_texture = 1;
    state.texture = Some("effect\\white02.bmp");
    state.size = 0.0;

    if keep_delta_pos {
        state.delta_pos = target;
    }

    index
}

/// The four `height[20..=1]`/`flag1[20..=1]` zeroing every body performs.
fn clear_history(state: &mut EffectPrimitiveState, index: usize) {
    for height in 1..=20 {
        state.slots[index].height[height] = 0.0;
        state.slots[index].flag1[height] = 0;
    }
}

// ---------------------------------------------------------------------------
// White02Burst
// ---------------------------------------------------------------------------

/// `CRagEffect::White02Burst(char kind)` - `RagEffect.cpp:16154-16199`.
///
/// Frames 6..=9 (the `(unsigned)(stateCnt - 6) > 3` self-gate) spawn the four
/// white02 strands that rise from the caster.
#[derive(Debug)]
pub struct White02Burst {
    kind: i8,
    lifetime: i32,
}

impl White02Burst {
    pub fn new(kind: i8, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

impl PortedEffect for White02Burst {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = parent->render.position;`
        runtime.copy_parent_position();

        // `if (static_cast<unsigned>(stateCnt - 6) > 3) return;`
        let delta = runtime.frame - 6;

        if (delta as u32) > 3 {
            return true;
        }

        let kind = i32::from(self.kind);
        let position = EffectRuntime::official_position(runtime.position);

        // RNG order per slot: process (31), alphaB (121), riseAngle (360),
        // distance (6), vecTNow.x (7).
        let mut processes = [0i32; 4];
        let mut alphas = [0i16; 4];
        let mut rises = [0i16; 4];
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];

        for index in 0..4usize {
            processes[index] = -runtime.rng.clamped_random(31);
            alphas[index] = (runtime.rng.clamped_random(121) + 125) as i16;
            rises[index] = runtime.rng.clamped_random(360) as i16;
            distances[index] = runtime.rng.clamped_random(6) as f32 + 6.0;
            xs[index] = runtime.rng.clamped_random(7) as f32 * 0.1 + 0.8;
        }

        let index = launch_white02(runtime, false);
        let state = runtime.primitive_mut(index);

        for slot_index in 0..4usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = processes[slot_index];
            slot.max_height = if kind == 3 { 0.05 } else { 0.1 };
            slot.alpha_b = alphas[slot_index];
            slot.rise_angle = rises[slot_index];
            slot.distance = distances[slot_index];
            slot.height[0] = 0.0;
            slot.vec_t_now[0] = xs[slot_index];
            slot.vec_b_now = position;
            slot.vec_t_now[1] = 4.0;
            slot.vec_t_now[2] = 0.0;

            clear_history(state, slot_index);

            state.slots[slot_index].alpha_t = kind as i16;
        }

        true
    }
}

// ---------------------------------------------------------------------------
// WhiteSparkRing
// ---------------------------------------------------------------------------

/// `CRagEffect::WhiteSparkRing(char kind)` - `RagEffect.cpp:13388-13446`.
///
/// Frames 0..=2 spawn a ring of white02 strands whose heading is the bearing
/// from the target back to the caster plus a `[-60, 60]` jitter.
#[derive(Debug)]
pub struct WhiteSparkRing {
    kind: i8,
    lifetime: i32,
}

impl WhiteSparkRing {
    pub fn new(kind: i8, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

impl PortedEffect for WhiteSparkRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.copy_parent_position();

        // `if (stateCnt < 0 || stateCnt > 2) return;`
        if runtime.frame > 2 {
            return true;
        }

        let kind = i32::from(self.kind);
        let position = EffectRuntime::official_position(runtime.position);
        let target = EffectRuntime::official_position(runtime.target_offset);

        // `Get_Angle(render.position.x - deltaPos.x,
        //  render.position.z - deltaPos.z, &riseAngle)`.
        let base_rise = runtime.get_angle(position[0] - target[0], position[2] - target[2]) as i32;

        // RNG order per slot: process (21/26), alphaB (121), bearing jitter
        // (121), distance (7), vecTNow.x (7).
        let mut processes = [0i32; 4];
        let mut alphas = [0i16; 4];
        let mut rises = [0i16; 4];
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];

        for index in 0..4usize {
            processes[index] = if kind == 3 {
                -(25 + runtime.rng.clamped_random(21))
            } else {
                -(15 + runtime.rng.clamped_random(26))
            };
            alphas[index] = (runtime.rng.clamped_random(121) + 125) as i16;
            rises[index] = base_rise as i16 + (runtime.rng.clamped_random(121) - 60) as i16;
            distances[index] = runtime.rng.clamped_random(7) as f32 + 6.0;
            xs[index] = runtime.rng.clamped_random(7) as f32 * 0.1 + 0.8;
        }

        let index = launch_white02(runtime, true);
        let state = runtime.primitive_mut(index);

        for slot_index in 0..4usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = processes[slot_index];
            slot.max_height = if kind == 3 { 0.06 } else { 0.07 };
            slot.alpha_b = alphas[slot_index];
            slot.rise_angle = rises[slot_index];

            if slot.rise_angle >= 360 {
                slot.rise_angle -= 360;
            }

            if slot.rise_angle < 0 {
                slot.rise_angle += 360;
            }

            slot.distance = distances[slot_index];
            slot.height[0] = 0.0;
            slot.vec_t_now[0] = xs[slot_index];
            slot.vec_b_now = position;
            slot.vec_t_now[1] = 3.0;
            slot.vec_t_now[2] = 0.0;

            clear_history(state, slot_index);

            state.slots[slot_index].alpha_t = kind as i16;
        }

        true
    }
}

// ---------------------------------------------------------------------------
// WhiteSparkStreak
// ---------------------------------------------------------------------------

/// `CRagEffect::WhiteSparkStreak(char kind)` - `RagEffect.cpp:13951-14063`.
///
/// `kind` sets the spawn-frame limit: 1 -> 2 frames, 3 -> 5 frames, anything
/// else -> 0 frames (dead). Kind 1 additionally pre-fills ten history steps so
/// the streak starts already risen.
#[derive(Debug)]
pub struct WhiteSparkStreak {
    kind: i8,
    lifetime: i32,
}

impl WhiteSparkStreak {
    pub fn new(kind: i8, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

impl PortedEffect for WhiteSparkStreak {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.copy_parent_position();

        let kind = i32::from(self.kind);
        let limit = if kind == 1 {
            2
        } else if kind == 3 {
            5
        } else {
            0
        };

        // `if (stateCnt < 0 || stateCnt >= limit) return;`
        if runtime.frame < 0 || runtime.frame >= limit {
            return true;
        }

        let position = EffectRuntime::official_position(runtime.position);

        // RNG order per slot: process (11/41/26), alphaB (51/121), riseAngle
        // (360), distance (5/6), vecTNow.x (7).
        let mut processes = [0i32; 4];
        let mut alphas = [0i16; 4];
        let mut rises = [0i16; 4];
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];

        for index in 0..4usize {
            if kind == 1 {
                processes[index] = -runtime.rng.clamped_random(11);
                alphas[index] = (runtime.rng.clamped_random(51) + 125) as i16;
            } else if kind == 3 {
                processes[index] = -30 - runtime.rng.clamped_random(41);
                alphas[index] = (runtime.rng.clamped_random(121) + 125) as i16;
            } else {
                processes[index] = -runtime.rng.clamped_random(26);

                if kind >= 3 {
                    alphas[index] = (runtime.rng.clamped_random(121) + 125) as i16;
                } else {
                    alphas[index] = (runtime.rng.clamped_random(51) + 125) as i16;
                }
            }

            rises[index] = runtime.rng.clamped_random(360) as i16;

            if kind == 1 {
                distances[index] = runtime.rng.clamped_random(5) as f32 + 5.0;
            } else {
                distances[index] = runtime.rng.clamped_random(6) as f32 + 6.0;
            }

            xs[index] = runtime.rng.clamped_random(7) as f32 * 0.1 + 0.6;
        }

        let index = launch_white02(runtime, false);
        let state = runtime.primitive_mut(index);

        for slot_index in 0..4usize {
            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = processes[slot_index];
            slot.max_height = if kind == 3 { 0.07 } else { 0.1 };
            slot.alpha_b = alphas[slot_index];
            slot.rise_angle = rises[slot_index];

            if kind == 1 {
                slot.vec_t_now[1] = 3.0;
            } else if kind == 3 {
                slot.vec_t_now[1] = 4.0;
            } else {
                slot.vec_t_now[1] = 5.0;
            }

            slot.distance = distances[slot_index];
            slot.height[0] = 0.0;
            slot.vec_t_now[0] = xs[slot_index];
            slot.vec_b_now = position;
            slot.vec_t_now[2] = 0.0;

            clear_history(state, slot_index);

            state.slots[slot_index].alpha_t = kind as i16;

            if kind == 1 {
                for _step in 0..10 {
                    state.slots[slot_index].height[0] += 6.0;

                    if state.slots[slot_index].height[0] > 180.0 {
                        state.slots[slot_index].height[0] = 180.0;
                    }

                    let mut layer = state.slots[slot_index].vec_t_now[1] as i32;

                    while layer >= 1 {
                        let target = layer as usize;
                        state.slots[slot_index].height[target] = state.slots[slot_index].height[target - 1];
                        layer -= 1;
                    }
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn white02_burst_spawns_on_frames_six_to_nine() {
        let mut effect = White02Burst::new(3, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 4);

        for primitive in &runtime.primitives {
            assert_eq!(primitive.state.count, PrimitiveId::HitLine3);
            assert_eq!(primitive.state.texture, Some("effect\\white02.bmp"));
            assert_eq!(primitive.state.render_flag, RENDER_FLAG);
            assert_eq!(primitive.state.duration, 200);
            assert_eq!(primitive.state.slots[0].max_height, 0.05);
            assert_eq!(primitive.state.slots[0].vec_t_now[1], 4.0);
            assert_eq!(primitive.state.slots[0].alpha_t, 3);
        }
    }

    #[test]
    fn white_spark_ring_spawns_three_rings_with_a_jittered_bearing() {
        let mut effect = WhiteSparkRing::new(3, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.position = [10.0, 0.0, 0.0];
        runtime.target_offset = [30.0, 0.0, 0.0];

        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 3);
        assert_eq!(runtime.primitives[0].state.delta_pos, [30.0, 0.0, 0.0]);

        // Caster - target = -20 on x, i.e. the base bearing is 180; the
        // jitter is [-60, 60].
        for slot in &runtime.primitives[0].state.slots {
            assert!((120..=240).contains(&slot.rise_angle), "rise {}", slot.rise_angle);
        }
    }

    #[test]
    fn white_spark_streak_kind_zero_is_dead() {
        let mut effect = WhiteSparkStreak::new(0, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..6 {
            step(&mut effect, &mut runtime);
        }

        assert!(runtime.primitives.is_empty());
    }

    #[test]
    fn white_spark_streak_kind_three_spawns_five_strands() {
        let mut effect = WhiteSparkStreak::new(3, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..6 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 5);
        assert_eq!(runtime.primitives[0].state.slots[0].vec_t_now[1], 4.0);
        assert_eq!(runtime.primitives[0].state.slots[0].max_height, 0.07);
    }
}
