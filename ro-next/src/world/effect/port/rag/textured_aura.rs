//! `CRagEffect::TexturedAura(name, kind)` - `RagEffect.cpp:17769-17960`.
//!
//! The persistent aura helper behind ~37 effect ids (glows, status overlays,
//! map spheres, tarot cards). Spawns one `PP_AURA` primitive on frame 0; the
//! `kind` selects distance / orbit height / alpha behaviour.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedAura {
    /// `(texture, kind)` pairs; multi-aura effects (map spheres) pass four.
    auras: Vec<(&'static str, i32)>,
    lifetime: i32,
}

impl TexturedAura {
    pub fn new(auras: &[(&'static str, i32)], lifetime: i32) -> Self {
        Self {
            auras: auras.to_vec(),
            lifetime,
        }
    }
}

/// Kind parameters extracted from the original branch chain:
/// `(distance, flag1[2], height[10], fullDisplayAngle, vecBPre.y offset)`.
fn kind_params(
    kind: i32,
    param: f32,
    parent_action_height: Option<f32>,
    rng: &mut super::super::rng::EffectRng,
) -> (f32, i8, f32, i16, f32) {
    match kind {
        1 => (8.0, 0, 1.0, 120, -30.0),
        2 => (10.0, 2, 0.0, 135, -10.0),
        3 => (8.0, 3, 2.0, 155, -30.0),
        4 => (6.0, 0, 0.0, 135, -30.0),
        10 | 11 => (
            if param <= 0.0 { 70.0 } else { param * 0.7 },
            10,
            0.0,
            rng.clamped_random(360) as i16,
            0.0,
        ),
        12 => (
            if param <= 0.0 { 120.0 } else { param * 1.2 },
            12,
            0.0,
            rng.clamped_random(360) as i16,
            0.0,
        ),
        13 => (
            if param <= 0.0 { 300.0 } else { param * 3.0 },
            12,
            0.0,
            rng.clamped_random(360) as i16,
            0.0,
        ),
        14 => (
            if param <= 0.0 { 30.0 } else { param },
            4,
            0.0,
            rng.clamped_random(360) as i16,
            0.0,
        ),
        15 => (
            if param <= 0.0 { 30.0 } else { param },
            15,
            0.0,
            rng.clamped_random(360) as i16,
            0.0,
        ),
        16 | 17 => {
            let mut distance = if param <= 0.0 { 9.5 } else { param * 0.9 };

            if kind == 17 {
                distance *= 0.85;
            }

            (distance, 5, 0.0, rng.clamped_random(360) as i16, 0.0)
        }
        18 => (
            if param <= 0.0 { 9.0 } else { param * 0.9 },
            5,
            0.0,
            rng.clamped_random(360) as i16,
            0.0,
        ),
        19 => (8.0, 15, 0.0, rng.clamped_random(360) as i16, -30.0),
        20 => (2.4, 0, 0.0, 135, -27.0),
        21 => (24.0, 4, 0.0, 90, -10.0),
        22 => {
            // Lift to the parent sprite's head: `cell.y * 0.29 - 4`, or -30
            // when the cell has no height.
            let lift = match parent_action_height {
                Some(height) if height != 0.0 => height * 0.29 - 4.0,
                _ => -30.0,
            };

            (4.0, 16, 0.0, 135, lift)
        }
        _ => (0.0, 0, 0.0, 0, 0.0),
    }
}

impl PortedEffect for TexturedAura {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let param = runtime.param;
        let parent_action_height = runtime.parent_action_height;
        let position_y = runtime.position[1];

        for &(texture, kind) in &self.auras {
            let (distance, flag, height10, display_angle, vy) =
                kind_params(kind, param, parent_action_height, &mut runtime.rng);

            let aura = runtime.launch(PrimitiveId::AuraGlow, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(aura);

            state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
            state.duration = lifetime;
            state.texture = Some(texture);
            state.size = 0.0;

            for slot in &mut state.slots {
                slot.life = 0;
            }

            state.slots[0].life = 1;
            state.slots[0].process = 0;
            state.slots[0].distance = distance;
            state.slots[0].vec_b_pre[1] = position_y + vy;
            state.slots[0].flag1[2] = flag;
            state.slots[0].height[10] = height10;
            state.slots[0].full_display_angle = display_angle;

            // `RagEffect.cpp:17964-17965`: the motion kind (4 collapses to 1)
            // and the vertical lift the motion uses to place `vecBNow`.
            state.slots[0].flag1[4] = if kind == 4 { 1 } else { kind as i8 };
            state.slots[0].height[15] = position_y - state.slots[0].vec_b_pre[1];
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn oratio_spawns_the_kind_nineteen_aura() {
        let mut effect = TexturedAura::new(&[("effect\\마법진1-3.bmp", 19)], 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 191);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::AuraGlow);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\마법진1-3.bmp"));
        assert_eq!(state.slots[0].life, 1);
        assert_eq!(state.slots[0].distance, 8.0);
        assert_eq!(state.slots[0].flag1[2], 15);
        // `RagEffect.cpp:17964-17965`: motion kind and the lift the motion uses.
        assert_eq!(state.slots[0].flag1[4], 19);
        assert_eq!(state.slots[0].height[15], 30.0);
        assert_eq!(state.slots[1].life, 0);
    }

    #[test]
    fn mapsphere_spawns_four_auras() {
        let auras = [
            ("effect\\구체2.bmp", 12),
            ("effect\\구체3.bmp", 13),
            ("effect\\구체4.bmp", 11),
            ("effect\\구체5.bmp", 10),
        ];
        let mut effect = TexturedAura::new(&auras, 36_000);
        let mut runtime = EffectRuntime::new(36_000, [0.0, 0.0], 191);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 4);
        assert_eq!(runtime.primitives[0].state.slots[0].distance, 120.0);
        assert_eq!(runtime.primitives[1].state.slots[0].distance, 300.0);
        assert_eq!(runtime.primitives[3].state.slots[0].flag1[2], 10);
    }

    #[test]
    fn tarot_uses_kind_one() {
        let mut effect = TexturedAura::new(&[("effect\\Tarot01.tga", 1)], 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 191);

        step(&mut effect, &mut runtime);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.flag1[4], 1, "kind 1 stores its own motion kind");
        assert_eq!(slot.height[15], 30.0, "lift = position.y - vecBPre.y");
        assert_eq!(slot.distance, 8.0);
        assert_eq!(slot.full_display_angle, 120);
        assert_eq!(slot.height[10], 1.0);
        assert_eq!(slot.vec_b_pre[1], -30.0);
    }

    #[test]
    fn kind_twenty_two_lifts_to_the_parent_head() {
        let mut effect = TexturedAura::new(&[("effect\\베놈_상태.bmp", 22)], 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 191);
        runtime.parent_action_height = Some(100.0);
        runtime.position = [0.0, 10.0, 0.0];

        step(&mut effect, &mut runtime);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.flag1[2], 16);
        assert!((slot.vec_b_pre[1] - (10.0 + 100.0 * 0.29 - 4.0)).abs() < 1e-4);
    }

    #[test]
    fn kind_twenty_two_falls_back_to_minus_thirty() {
        let mut effect = TexturedAura::new(&[("effect\\베놈_상태.bmp", 22)], 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 191);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.slots[0].vec_b_pre[1], -30.0);
    }

    #[test]
    fn kind_seventeen_scales_the_distance() {
        let mut effect = TexturedAura::new(&[("effect\\glow02.bmp", 17)], 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 191);

        step(&mut effect, &mut runtime);

        assert!((runtime.primitives[0].state.slots[0].distance - 9.5 * 0.85).abs() < 1e-5);
    }
}
