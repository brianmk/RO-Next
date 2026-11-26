//! `CRagEffect::WhiteRingRipple(size)` - `RagEffect.cpp:6839-6908`.
//!
//! The 100-frame white ring ripple, plus the three bottom-layer dispatch blocks
//! that combine it with halos and dust rings.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::textured_halo::TexturedHalo;
use super::torch_dust_ring::TorchDustRing;

/// One `WhiteRingRipple(size)` call. Returns `false` when the frame gate
/// rejects.
pub(crate) fn white_ring_ripple(runtime: &mut EffectRuntime, size: i32) -> bool {
    if runtime.frame % 100 != 0 {
        return false;
    }

    let lifetime = runtime.lifetime;

    let ripple = runtime.launch(PrimitiveId::RippleRings, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(ripple);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.texture = Some("effect\\ring_white.tga");
    state.size = match size {
        1 => 8.0,
        2 => 6.0,
        _ => 4.0,
    };

    const PROCESSES: [i32; 3] = [0, -10, -20];
    const RISES: [i16; 3] = [2, 3, 4];

    for index in 0..3usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = PROCESSES[index];
        slot.full_display_angle = 360;
        slot.max_height = 6.001;
        slot.rot_start = index as i16 * 25;
        slot.alpha_b = 0;
        slot.distance = 0.0;
        slot.rise_angle = RISES[index];
        slot.alpha_t = 100;
        slot.vec_b_now[1] = 0.0;
        slot.height = [0.0; 21];
        slot.flag1 = [1; 21];
    }

    state.slots[3].life = 0;
    state.slots[3].process = 0;
    state.slots[3].alpha_t = 4;

    true
}

/// The bottom-layer composites that draw a halo/dust ring plus the ripple.
#[derive(Debug)]
pub enum BottomLayer {
    /// `EF_BOTTOM_MANHOLE`: twirl halo (kind 4) + ripple 0.
    Manhole,
    /// `EF_BOTTOM_ANI`: dust ring (kind 0, angle 0) + ripple 1.
    Ani,
    /// `EF_BOTTOM_BLOODYLUST`: torch rings at 0/120/240 + ripple 2.
    BloodyLust,
}

pub struct BottomLayers {
    layer: BottomLayer,
    lifetime: i32,
}

impl BottomLayers {
    pub fn new(layer: BottomLayer, lifetime: i32) -> Self {
        Self { layer, lifetime }
    }
}

impl PortedEffect for BottomLayers {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            match self.layer {
                BottomLayer::Manhole => {
                    TexturedHalo::new("effect\\twirl.bmp", 4, self.lifetime).on_process(runtime);
                }
                BottomLayer::Ani => {
                    TorchDustRing::new(0, 0, self.lifetime).on_process(runtime);
                }
                BottomLayer::BloodyLust => {
                    for angle in [0, 120, 240] {
                        TorchDustRing::new(1, angle, self.lifetime).on_process(runtime);
                    }
                }
            }
        }

        match self.layer {
            BottomLayer::Manhole => {
                white_ring_ripple(runtime, 0);
            }
            BottomLayer::Ani => {
                white_ring_ripple(runtime, 1);
            }
            BottomLayer::BloodyLust => {
                white_ring_ripple(runtime, 2);
            }
        }

        true
    }
}

#[derive(Debug)]
pub struct RingRippleOnly {
    size: i32,
    lifetime: i32,
}

impl RingRippleOnly {
    pub fn new(size: i32, lifetime: i32) -> Self {
        Self { size, lifetime }
    }
}

impl PortedEffect for RingRippleOnly {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        white_ring_ripple(runtime, self.size);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn ripple_fires_every_hundred_frames() {
        let mut effect = RingRippleOnly::new(1, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 451);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        let ripple = &runtime.primitives[0].state;
        assert_eq!(ripple.count, PrimitiveId::RippleRings);
        assert_eq!(ripple.size, 8.0);
        assert_eq!(ripple.texture, Some("effect\\ring_white.tga"));
        assert_eq!(ripple.slots[0].alpha_t, 100);
        assert_eq!(ripple.slots[3].alpha_t, 4);

        while runtime.frame < 101 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 2, "frame 100 fires again");
    }

    #[test]
    fn sizes_follow_the_code() {
        for (code, expected) in [(0, 4.0), (1, 8.0), (2, 6.0)] {
            let mut effect = RingRippleOnly::new(code, 200);
            let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 451);
            step(&mut effect, &mut runtime);
            assert_eq!(runtime.primitives[0].state.size, expected);
        }
    }
}
