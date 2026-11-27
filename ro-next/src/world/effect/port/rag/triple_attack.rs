//! `EF_TRIPLEATTACK` (329), `EF_TRIPLEATTACK2` (388), `EF_TRIPLEATTACK3` (393)
//! - the three prim-69 bodies.
//!
//! | effect | body | lifetime |
//! | --- | --- | --- |
//! | `EF_TRIPLEATTACK` | `CloudDrift()` - `RagEffect.cpp:8620-8676` | 100 |
//! | `EF_TRIPLEATTACK2` | `AlphaCenterTrail()` - `RagEffect.cpp:10879-10931` | 200 |
//! | `EF_TRIPLEATTACK3` | `AlphaCenterStream()` - `RagEffect.cpp:8577-8617` | 200 |
//!
//! All three arm a `PP`-less prim 69 (`RenderPentaRing`'s ring pair): four
//! slots get a negative `process` delay, a bearing from the caster to the
//! effect's target, a scattershot `vecBNow`/`vecTNow` pair and a `height[4]`
//! kind that selects the renderer colour. `CloudDrift` uses
//! `effect\cloud11.tga`, the two `alpha_center.tga` bodies differ in delay
//! window, scale and kind.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The common `LaunchEffectPrim(69, ...)` shape.
fn launch_cloud(runtime: &mut EffectRuntime, texture: &'static str) -> usize {
    let lifetime = runtime.lifetime;
    let target = EffectRuntime::official_position(runtime.target_offset);

    let index = runtime.launch(PrimitiveId::PentaRing, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(texture);
    state.size = 0.0;
    // `this->state.lastPrim->state.deltaPos = this->state.deltaPos;`
    state.delta_pos = target;

    index
}

// ---------------------------------------------------------------------------
// CloudDrift
// ---------------------------------------------------------------------------

/// `CRagEffect::CloudDrift()` - `RagEffect.cpp:8620-8676`.
///
/// `EF_TRIPLEATTACK`: the cloud11 puffs are seeded on frames 0 and 1 and the
/// body plays the three hit sounds on frames 10/20/30.
#[derive(Debug)]
pub struct CloudDrift {
    lifetime: i32,
}

impl CloudDrift {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for CloudDrift {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, parent.position);`
        runtime.copy_parent_position();

        let angles = Angles::shared();

        if runtime.frame <= 1 {
            let position = EffectRuntime::official_position(runtime.position);
            let target = EffectRuntime::official_position(runtime.target_offset);
            let dx = target[0] - position[0];
            let dz = target[2] - position[2];

            // RNG order per slot: process (20), vecBNow.x (100),
            // vecBNow.z (100), height[1] (100), height[0] (100),
            // distance (100).
            let mut processes = [0i32; 4];
            let mut xs = [0.0f32; 4];
            let mut zs = [0.0f32; 4];
            let mut heights = [0.0f32; 4];
            let mut height0 = [0.0f32; 4];
            let mut distances = [0.0f32; 4];

            for index in 0..4usize {
                processes[index] = -runtime.rng.clamped_random(20);
                xs[index] = runtime.rng.clamped_random(100) as f32 * 0.06 - 3.0 + position[0];
                zs[index] = runtime.rng.clamped_random(100) as f32 * 0.06 - 3.0 + position[2];
                heights[index] = runtime.rng.clamped_random(100) as f32 * 0.15;
                height0[index] = runtime.rng.clamped_random(100) as f32 * 0.04 + 8.0;
                distances[index] = runtime.rng.clamped_random(100) as f32 * 0.2 + 20.0;
            }

            let index = launch_cloud(runtime, "effect\\cloud11.tga");
            let state = runtime.primitive_mut(index);

            for slot_index in 0..4usize {
                // `Get_Angle(dx, dz, &rotStart)`.
                let rot_start = super::super::primitive::get_angle(dx, dz);

                let slot = &mut state.slots[slot_index];

                slot.life = 1;
                slot.process = processes[slot_index];
                slot.rot_start = rot_start;
                slot.alpha_b = 0;
                slot.vec_b_now[0] = xs[slot_index];
                slot.vec_b_now[2] = zs[slot_index];
                slot.vec_b_now[1] = position[1];
                slot.height[1] = heights[slot_index];
                slot.vec_b_now[0] -= angles.raw_cosine(slot.rot_start.max(0) as usize) * slot.height[1];
                slot.vec_b_now[2] -= angles.raw_sine(slot.rot_start.max(0) as usize) * slot.height[1];
                slot.vec_t_now[0] = slot.vec_b_now[0];
                slot.vec_t_now[2] = slot.vec_b_now[2];
                slot.vec_t_now[1] = position[1];
                slot.max_height = 0.25;
                slot.height[0] = height0[slot_index];
                slot.distance = distances[slot_index];
                slot.height[4] = 0.0;
            }
        }

        if runtime.frame == 10 {
            runtime.play_sound("effect\\EF_hit2.wav");
        }

        if runtime.frame == 20 {
            runtime.play_sound("effect\\EF_hit4.wav");
        }

        if runtime.frame == 30 {
            runtime.play_sound("effect\\EF_hit2.wav");
        }

        true
    }
}

// ---------------------------------------------------------------------------
// AlphaCenterTrail
// ---------------------------------------------------------------------------

/// `CRagEffect::AlphaCenterTrail()` - `RagEffect.cpp:10879-10931`.
///
/// `EF_TRIPLEATTACK2`: four `alpha_center.tga` strands, `height[4] == 2`,
/// spawned on frames 0..=3; the body also plays the sharp-shooting wave on
/// frame 30.
#[derive(Debug)]
pub struct AlphaCenterTrail {
    lifetime: i32,
}

impl AlphaCenterTrail {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for AlphaCenterTrail {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = this->state.parent->render.position;`
        runtime.copy_parent_position();

        if runtime.frame == 30 {
            runtime.play_sound("effect\\샤프슈팅.wav");
        }

        // `if (this->state.stateCnt > 3) return;`
        if runtime.frame > 3 {
            return true;
        }

        let angles = Angles::shared();
        let position = EffectRuntime::official_position(runtime.position);
        let target = EffectRuntime::official_position(runtime.target_offset);
        let delta_x = target[0] - position[0];
        let delta_z = target[2] - position[2];
        let distance = (delta_z * delta_z + delta_x * delta_x).sqrt();

        // RNG order per slot: process (20), vecBNow.x (100), vecBNow.z (100),
        // height[1] (100), height[0] (100), distance (100).
        let mut processes = [0i32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut heights = [0.0f32; 4];
        let mut height0 = [0.0f32; 4];
        let mut distances = [0.0f32; 4];

        for index in 0..4usize {
            processes[index] = -45 - runtime.rng.clamped_random(20);
            xs[index] = runtime.rng.clamped_random(100) as f32 * 0.08 - 4.0 + position[0];
            zs[index] = runtime.rng.clamped_random(100) as f32 * 0.08 - 4.0 + position[2];
            heights[index] = runtime.rng.clamped_random(100) as f32 * 0.15;
            height0[index] = runtime.rng.clamped_random(100) as f32 * 0.04 + 8.0;
            distances[index] = runtime.rng.clamped_random(100) as f32 * 0.6 + distance;
        }

        let index = launch_cloud(runtime, "effect\\alpha_center.tga");
        let state = runtime.primitive_mut(index);

        for slot_index in 0..4usize {
            let rot_start = super::super::primitive::get_angle(delta_x, delta_z);

            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = processes[slot_index];
            slot.rot_start = rot_start;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = xs[slot_index];
            slot.vec_b_now[2] = zs[slot_index];
            slot.vec_b_now[1] = position[1];
            slot.height[1] = heights[slot_index];
            slot.vec_b_now[0] -= angles.raw_cosine(slot.rot_start.max(0) as usize) * slot.height[1];
            slot.vec_b_now[2] -= angles.raw_sine(slot.rot_start.max(0) as usize) * slot.height[1];
            slot.vec_t_now[0] = slot.vec_b_now[0];
            slot.vec_t_now[2] = slot.vec_b_now[2];
            slot.vec_t_now[1] = position[1];
            slot.max_height = 0.05;
            slot.height[0] = height0[slot_index];
            slot.distance = distances[slot_index];
            slot.height[4] = 2.0;
        }

        true
    }
}

// ---------------------------------------------------------------------------
// AlphaCenterStream
// ---------------------------------------------------------------------------

/// `CRagEffect::AlphaCenterStream()` - `RagEffect.cpp:8577-8617`.
///
/// `EF_TRIPLEATTACK3`: four `alpha_center.tga` strands, `height[4] == 3`,
/// spawned on frames 0..=8; the body plays the arrow-volcano wave on frame 30.
#[derive(Debug)]
pub struct AlphaCenterStream {
    lifetime: i32,
}

impl AlphaCenterStream {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for AlphaCenterStream {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        runtime.copy_parent_position();

        if runtime.frame == 30 {
            runtime.play_sound("effect\\애로우 발칸.wav");
        }

        // `if (this->state.stateCnt > 8) return;`
        if runtime.frame > 8 {
            return true;
        }

        let angles = Angles::shared();
        let position = EffectRuntime::official_position(runtime.position);
        let target = EffectRuntime::official_position(runtime.target_offset);
        let delta_x = target[0] - position[0];
        let delta_z = target[2] - position[2];
        let range = (delta_z * delta_z + delta_x * delta_x).sqrt();

        // RNG order per slot: process (110), vecBNow.x (100), vecBNow.z (100),
        // height[1] (100), maxHeight (11), height[0] (100), distance (100).
        let mut processes = [0i32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut heights = [0.0f32; 4];
        let mut max_heights = [0.0f32; 4];
        let mut height0 = [0.0f32; 4];
        let mut distances = [0.0f32; 4];

        for index in 0..4usize {
            processes[index] = -30 - runtime.rng.clamped_random(110);
            xs[index] = runtime.rng.clamped_random(100) as f32 * 0.08 - 4.0 + position[0];
            zs[index] = runtime.rng.clamped_random(100) as f32 * 0.08 - 4.0 + position[2];
            heights[index] = runtime.rng.clamped_random(100) as f32 * 0.15;
            max_heights[index] = runtime.rng.clamped_random(11) as f32 * 0.008 + 0.02;
            height0[index] = runtime.rng.clamped_random(100) as f32 * 0.06 + 4.0;
            distances[index] = runtime.rng.clamped_random(100) as f32 * 0.3 + range;
        }

        let index = launch_cloud(runtime, "effect\\alpha_center.tga");
        let state = runtime.primitive_mut(index);

        for slot_index in 0..4usize {
            let rot_start = super::super::primitive::get_angle(delta_x, delta_z);

            let slot = &mut state.slots[slot_index];

            slot.life = 1;
            slot.process = processes[slot_index];
            slot.rot_start = rot_start;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = xs[slot_index];
            slot.vec_b_now[2] = zs[slot_index];
            slot.vec_b_now[1] = position[1];
            slot.height[1] = heights[slot_index];
            slot.vec_b_now[0] -= angles.raw_cosine(slot.rot_start.max(0) as usize) * slot.height[1];
            slot.vec_b_now[2] -= angles.raw_sine(slot.rot_start.max(0) as usize) * slot.height[1];
            slot.vec_t_now[0] = slot.vec_b_now[0];
            slot.vec_t_now[2] = slot.vec_b_now[2];
            slot.vec_t_now[1] = position[1];
            slot.max_height = max_heights[slot_index];
            slot.height[0] = height0[slot_index];
            slot.distance = distances[slot_index];
            slot.height[4] = 3.0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn cloud_drift_spawns_two_cloud_primitives_and_three_hit_sounds() {
        let mut effect = CloudDrift::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 1);
        runtime.position = [10.0, 0.0, 10.0];
        runtime.target_offset = [30.0, 0.0, 10.0];

        for _ in 0..31 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.sounds, vec![
            "effect\\EF_hit2.wav".to_owned(),
            "effect\\EF_hit4.wav".to_owned(),
            "effect\\EF_hit2.wav".to_owned(),
        ]);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::PentaRing);
        assert_eq!(state.texture, Some("effect\\cloud11.tga"));
        assert_eq!(state.delta_pos, [30.0, 0.0, 10.0]);

        // Caster -> target is +x: `Get_Angle(20, 0)` is 0.
        assert_eq!(state.slots[0].rot_start, 0);
        assert_eq!(state.slots[0].max_height, 0.25);
        assert_eq!(state.slots[0].height[4], 0.0);
    }

    #[test]
    fn alpha_center_trail_uses_kind_two_and_the_long_delay() {
        let mut effect = AlphaCenterTrail::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 4);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\alpha_center.tga"));
        assert_eq!(runtime.primitives[0].state.slots[0].height[4], 2.0);
        assert!((-65..=-45).contains(&runtime.primitives[0].state.slots[0].process));
    }

    #[test]
    fn alpha_center_stream_uses_kind_three_and_nine_spawn_frames() {
        let mut effect = AlphaCenterStream::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 9);
        assert_eq!(runtime.primitives[0].state.slots[0].height[4], 3.0);
        assert!((0.02..=0.1).contains(&runtime.primitives[0].state.slots[0].max_height));
        assert!((-140..=-30).contains(&runtime.primitives[0].state.slots[0].process));
    }

    #[test]
    fn each_body_plays_its_wave_on_frame_thirty() {
        let mut trail = AlphaCenterTrail::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        for _ in 0..31 {
            step(&mut trail, &mut runtime);
        }
        assert!(runtime.sounds.contains(&"effect\\샤프슈팅.wav".to_owned()));

        let mut stream = AlphaCenterStream::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        for _ in 0..31 {
            step(&mut stream, &mut runtime);
        }
        assert!(runtime.sounds.contains(&"effect\\애로우 발칸.wav".to_owned()));
    }
}
