//! `CRagEffect::WaterOut` - `RagEffect.cpp:3537-3564`.
//!
//! `EF_WATERBALL`: a three-texture `PP_3DTEXTURE` (15) water column that rises
//! by 25 units over its lifetime, additive, `animSpeed 5`, `repeatAnim 1`.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct WaterBall {
    lifetime: i32,
}

impl WaterBall {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for WaterBall {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;

        runtime.play_sound("effect\\wizard_waterball_chulung.wav");

        let column = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(column);

        state.duration = lifetime;
        state.pattern = 0x40 | 0x100;
        state.render_flag |= 2; // RSF_ADDITIVE
        state.repeat_anim = 1;
        state.width_size = 3.5;
        state.height_size = 3.5;
        state.speed3d[1] = -(25.0 / state.duration as f32);
        state.delta_pos2[1] = -5.0;
        state.anim_speed = 5;
        // `totalTexture = 3` with the three slots the official body loads.
        state.frame_count = 3;
        state.texture_frames = vec!["effect\\water_out_a.bmp", "effect\\water_out_b.bmp", "effect\\water_out_c.bmp"];
        state.texture = Some("effect\\water_out_a.bmp");
        state.texture_secondary = Some("effect\\water_out_b.bmp");
        state.texture_tertiary = Some("effect\\water_out_c.bmp");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn water_ball_spawns_the_three_texture_column() {
        let mut effect = WaterBall::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 391);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_waterball_chulung.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Texture3d);
        assert_eq!(state.duration, 250);
        assert_eq!(state.pattern, 0x40 | 0x100);
        assert_eq!(state.render_flag & 2, 2);
        assert_eq!(state.repeat_anim, 1);
        assert_eq!(state.width_size, 3.5);
        assert_eq!(state.height_size, 3.5);
        assert!((state.speed3d[1] + 25.0 / 250.0).abs() < 1e-6);
        assert_eq!(state.anim_speed, 5);
        assert_eq!(state.texture, Some("effect\\water_out_a.bmp"));
        assert_eq!(state.texture_secondary, Some("effect\\water_out_b.bmp"));
        assert_eq!(state.texture_tertiary, Some("effect\\water_out_c.bmp"));
    }
}
