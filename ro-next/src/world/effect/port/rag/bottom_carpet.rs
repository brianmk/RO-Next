//! `CRagEffect::Bottom` (`RagEffect.cpp:1768-1820`) and
//! `CRagEffect::Bottom2` (`:1718-1764`).
//!
//! `EF_BOTTOM` (violet) and `EF_BOTTOM2` (green) draw four `PP_3DTEXTURE`
//! strips in a cross under the caster, aligned by the parent's angle.

use super::super::angles::Angles;
use super::super::math::{Matrix, matrix_mult};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct BottomCarpet {
    green: bool,
    lifetime: i32,
}

impl BottomCarpet {
    pub fn violet() -> Self {
        Self {
            green: false,
            lifetime: 350,
        }
    }

    pub fn green() -> Self {
        Self {
            green: true,
            lifetime: 350,
        }
    }

    fn spawn(&self, runtime: &mut EffectRuntime) {
        let angles = Angles::shared();
        let parent_angle = runtime.parent_angle;

        let mut angle = (180.0 - parent_angle) as i32;

        if angle < 0 {
            angle += 360;
        }

        let mut yaw = Matrix::identity();
        yaw.make_y_rotation(angles.radian(angle));

        let width = if self.green { 2.5f32 } else { 5.0f32 };
        let texture = if self.green {
            "effect\\magic_green.tga"
        } else {
            "effect\\magic_violet.tga"
        };

        for side in [1.0f32, -1.0f32] {
            // First strip: offset along Z (green) or X? The originals differ in
            // which axis the side offset is applied to; both spawn two strips.
            let offsets = if self.green {
                [[0.0, 0.0, width * side], [width * side, 0.0, 0.0]]
            } else {
                [[0.0, 0.0, side * 5.0], [side * 5.0, 0.0, 0.0]]
            };

            for (index, offset) in offsets.iter().enumerate() {
                let strip = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(strip);

                state.duration = 9999;
                state.common_pattern = 1;
                state.pattern = 40;
                state.type_field = 50;

                let moved = matrix_mult(*offset, &yaw.values);
                state.delta_pos2 = moved;

                state.longitude = (angle + if index == 0 { 90 } else { 0 }) as f32;
                state.width_size = width;
                state.height_size = 15.0;
                state.height_speed = 0.25;
                state.texture = Some(texture);

                if self.green {
                    state.alpha = 180.0;
                } else {
                    state.alpha = 0.0;
                    state.max_alpha = 180.0;
                    state.alpha_speed = state.max_alpha / 6.0;
                }

                state.fade_out_cnt = 9999 - 35;
            }
        }
    }
}

impl PortedEffect for BottomCarpet {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            self.spawn(runtime);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn bottom_violet_spawns_four_strips() {
        let mut effect = BottomCarpet::violet();
        let mut runtime = EffectRuntime::new(350, [0.0, 0.0], 491);
        runtime.parent_angle = 0.0;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 4);

        let strip = &runtime.primitives[0].state;
        assert_eq!(strip.count, PrimitiveId::Texture3d);
        assert_eq!(strip.duration, 9999);
        assert_eq!(strip.common_pattern, 1);
        assert_eq!(strip.pattern, 40);
        assert_eq!(strip.type_field, 50);
        assert_eq!(strip.width_size, 5.0);
        // One texture-3d step raised the height by heightSpeed.
        assert!((strip.height_size - 15.25).abs() < 1e-4);
        assert!((strip.height_speed.abs() - 0.25).abs() < 1e-6);
        assert_eq!(strip.max_alpha, 180.0);
        assert_eq!(strip.fade_out_cnt, 9964);
        assert_eq!(strip.texture, Some("effect\\magic_violet.tga"));
        // Angle 180: the first strip's longitude is 270.
        assert_eq!(strip.longitude, 270.0);
    }

    #[test]
    fn bottom2_is_green_and_starts_visible() {
        let mut effect = BottomCarpet::green();
        let mut runtime = EffectRuntime::new(350, [0.0, 0.0], 491);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 4);
        let strip = &runtime.primitives[0].state;
        assert!((strip.alpha - 180.0).abs() < 1.0, "green carpet starts visible");
        assert_eq!(strip.width_size, 2.5);
        assert_eq!(strip.texture, Some("effect\\magic_green.tga"));
    }
}
