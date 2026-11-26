//! `CRagEffect::TorchDustRing(name, kind, angle)` - `RagEffect.cpp:3241-3310`.
//!
//! Animated dust/torch rings: a `PP_143` primitive whose `texture_frames` are
//! `torch_red01..13` (kind 1), `W_c_00..50` (kind 2) or `dust01..09` (else).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TorchDustRing {
    kind: i32,
    angle: i16,
    lifetime: i32,
}

/// The `texture_frames` tables are rebuilt per spawn, so they are kept in
/// static slices instead of being formatted at runtime.
fn frames(kind: i32) -> &'static [&'static str] {
    static TORCH: [&str; 13] = [
        "effect\\torch_red01.bmp",
        "effect\\torch_red02.bmp",
        "effect\\torch_red03.bmp",
        "effect\\torch_red04.bmp",
        "effect\\torch_red05.bmp",
        "effect\\torch_red06.bmp",
        "effect\\torch_red07.bmp",
        "effect\\torch_red08.bmp",
        "effect\\torch_red09.bmp",
        "effect\\torch_red10.bmp",
        "effect\\torch_red11.bmp",
        "effect\\torch_red12.bmp",
        "effect\\torch_red13.bmp",
    ];
    static DUST: [&str; 9] = [
        "effect\\dust01.bmp",
        "effect\\dust02.bmp",
        "effect\\dust03.bmp",
        "effect\\dust04.bmp",
        "effect\\dust05.bmp",
        "effect\\dust06.bmp",
        "effect\\dust07.bmp",
        "effect\\dust08.bmp",
        "effect\\dust09.bmp",
    ];

    if kind == 1 {
        &TORCH
    } else if kind == 2 {
        &[]
    } else {
        &DUST
    }
}

impl TorchDustRing {
    pub fn new(kind: i32, angle: i16, lifetime: i32) -> Self {
        Self { kind, angle, lifetime }
    }
}

impl PortedEffect for TorchDustRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let kind = self.kind;
        let lifetime = runtime.lifetime;
        let sequence = frames(kind);
        let rise = runtime.rng.clamped_random(360) as i16;
        let radius = runtime.param;
        let start = runtime.param_secondary;

        let ring = runtime.launch(PrimitiveId::TextureSpin, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;

        if kind == 2 {
            // `W_c_00..50` is a generated sequence: keep the count so the
            // renderer can index it, the names follow the same pattern.
            state.frame_count = 51;
            state.texture = Some("effect\\W_c_00.tga");
        } else {
            state.frame_count = sequence.len() as u32;
            state.texture_frames = sequence.to_vec();
            state.texture = sequence.first().copied();
        }

        state.slots[0].life = 1;
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;
        state.slots[0].process = 0;

        if kind == 2 {
            state.slots[0].distance = if radius < 5.0 { 20.0 } else { radius };
            state.slots[0].rot_start = if start < 0.0 || start >= 360.0 { 45 } else { start as i16 };
            state.slots[0].alpha_b = 200;
            state.slots[0].height[2] = 1.0;
        } else {
            state.slots[0].distance = 18.0;
            state.slots[0].rot_start = self.angle;
            state.slots[0].alpha_b = 200;
            state.slots[0].height[2] = 0.0;
        }

        state.slots[0].rise_angle = rise;
        state.slots[0].height[0] = -2.0;
        state.slots[0].height[1] = 0.0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn water_below_spawns_the_static_pulse_ring() {
        let mut effect = TorchDustRing::new(2, 0, 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 441);
        runtime.param = 12.0;
        runtime.param_secondary = 100.0;

        step(&mut effect, &mut runtime);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::TextureSpin);
        assert_eq!(ring.duration, 99_999_999);
        assert_eq!(ring.frame_count, 51);
        assert_eq!(ring.texture, Some("effect\\W_c_00.tga"));
        assert_eq!(ring.slots[0].distance, 12.0);
        assert_eq!(ring.slots[0].rot_start, 100);
        assert_eq!(ring.slots[0].alpha_b, 200);
        assert_eq!(ring.slots[0].height[2], 1.0);
        assert_eq!(ring.slots[0].height[0], -2.0);
    }

    #[test]
    fn dust_ring_uses_the_nine_frame_sequence() {
        let mut effect = TorchDustRing::new(0, 90, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 441);

        step(&mut effect, &mut runtime);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.frame_count, 9);
        assert_eq!(ring.texture_frames.len(), 9);
        assert_eq!(ring.texture, Some("effect\\dust01.bmp"));
        assert_eq!(ring.slots[0].distance, 18.0);
        // Spawned at 90, the spin primitive added 6.
        assert_eq!(ring.slots[0].rot_start, 96);
        assert_eq!(ring.slots[0].height[2], 0.0);
    }

    #[test]
    fn torch_ring_uses_the_thirteen_frame_sequence() {
        let mut effect = TorchDustRing::new(1, 120, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 441);

        step(&mut effect, &mut runtime);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.frame_count, 13);
        assert_eq!(ring.texture_frames[12], "effect\\torch_red13.bmp");
    }
}
