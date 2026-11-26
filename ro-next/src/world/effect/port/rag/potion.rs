//! `CRagEffect::Potion_Berserk` (`RagEffect.cpp:2477-2520`) and
//! `CRagEffect::Potion_Awakening` (`RagEffect.cpp:15045-15087`) with their
//! shared `CRagEffect::PotionPillar` helper (`:2459-2473`).
//!
//! Both pills raise an `alpha_down.tga` cylinder, tint the drinker and spray
//! `ac_center2.tga` spots every four frames.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::PotionPillar(heightSpeed, duration)`.
fn potion_pillar(runtime: &mut EffectRuntime, height_speed: f32, duration: i32) {
    let pillar = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(pillar);

    state.duration = duration;
    state.outer_size = 4.5;
    state.inner_size = 4.5;
    state.height_size = 10.0;
    state.height_speed = height_speed;
    state.height_accel = 0.01;
    state.alpha = 0.0;
    state.max_alpha = 90.0;
    state.alpha_speed = state.max_alpha / 20.0;
    state.fade_out_cnt = state.duration - 10;
    state.texture = Some("effect\\alpha_down.tga");
}

#[derive(Debug)]
pub struct PotionPill {
    /// `false` = Berserk (red), `true` = Awakening (yellow).
    awakening: bool,
    lifetime: i32,
}

impl PotionPill {
    pub fn new(awakening: bool, lifetime: i32) -> Self {
        Self { awakening, lifetime }
    }

    fn spot_color(&self) -> (i32, i32, i32) {
        if self.awakening { (240, 240, 120) } else { (240, 0, 0) }
    }

    fn body_color(&self) -> [u8; 3] {
        if self.awakening { [250, 250, 200] } else { [250, 120, 120] }
    }
}

impl PortedEffect for PotionPill {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 0 {
            runtime.body_tint = Some(self.body_color());
            runtime.quake();
            potion_pillar(runtime, 1.0, 50);
        }

        if frame == 18 {
            runtime.play_sound("effect\\ac_concentration.wav");
        }

        if (frame - 13) as u32 <= 96 && frame % 4 == 0 {
            // RNG order: rotation, forward, speed, width.
            let rotation = runtime.rng.clamped_random(360);
            let forward = (runtime.rng.clamped_random(7) + 2) as f32;
            let speed = (runtime.rng.clamped_random(50) + 20) as f32 / 60.0;
            let width = (runtime.rng.clamped_random(60) + 30) as f32 / 10.0;

            let spot = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spot);

            state.duration = 40;
            state.transform.make_y_rotation(angles.radian(rotation));

            let offset = matrix_mult([0.0, 0.0, forward], &state.transform.values);
            state.delta_pos2 = offset;

            state.transform.make_x_rotation(angles.radian(90));
            state.speed = speed;
            state.width_size = width;
            state.height_size = 0.2;

            let (red, green, blue) = self.spot_color();
            state.red = red;
            state.green = green;
            state.blue = blue;

            state.alpha = 0.0;
            state.max_alpha = 180.0;
            state.alpha_speed = state.max_alpha / 20.0;
            state.fade_out_cnt = state.duration - 20;
            state.texture = Some("effect\\ac_center2.tga");
            state.slots[0].life = 0;
        }

        runtime.process_str();

        if frame > 150 {
            runtime.body_tint = None;
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn berserk_raises_the_pillar_and_tints_red() {
        let mut effect = PotionPill::new(false, 199);
        let mut runtime = EffectRuntime::new(199, [0.0, 0.0], 301);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.body_tint, Some([250, 120, 120]));
        assert_eq!(runtime.primitives.len(), 1);

        let pillar = &runtime.primitives[0].state;
        assert_eq!(pillar.count, PrimitiveId::Cylinder3d);
        assert_eq!(pillar.duration, 50);
        assert_eq!(pillar.outer_size, 4.5);
        // 10.0 + heightSpeed 1.0 + heightAccel 0.01 after the first cylinder step.
        assert!((pillar.height_size - 11.01).abs() < 1e-4);
        assert_eq!(pillar.max_alpha, 90.0);
        assert_eq!(pillar.alpha_speed, 4.5);
        assert_eq!(pillar.fade_out_cnt, 40);
        assert_eq!(pillar.texture, Some("effect\\alpha_down.tga"));
    }

    #[test]
    fn spots_spawn_every_four_frames_from_thirteen() {
        let mut effect = PotionPill::new(false, 199);
        let mut runtime = EffectRuntime::new(199, [0.0, 0.0], 301);

        while runtime.frame < 13 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 1, "no spot before frame 13");

        step(&mut effect, &mut runtime);
        assert!(runtime.sounds.is_empty(), "sound lands on frame 18");

        while runtime.frame < 19 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.sounds, vec!["effect\\ac_concentration.wav".to_owned()]);

        let spot = &runtime.primitives[1].state;
        assert_eq!(spot.count, PrimitiveId::CrossTexture3d);
        assert_eq!(spot.duration, 40);
        assert_eq!((spot.red, spot.green, spot.blue), (240, 0, 0));
        assert_eq!(spot.max_alpha, 180.0);
        assert_eq!(spot.fade_out_cnt, 20);
        assert_eq!(spot.texture, Some("effect\\ac_center2.tga"));
        assert_eq!(spot.slots[0].life, 0);
    }

    #[test]
    fn awakening_uses_the_yellow_variant_and_stops_after_150() {
        let mut effect = PotionPill::new(true, 199);
        let mut runtime = EffectRuntime::new(199, [0.0, 0.0], 301);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.body_tint, Some([250, 250, 200]));

        while runtime.frame < 152 {
            if !step(&mut effect, &mut runtime) {
                break;
            }
        }

        assert_eq!(runtime.body_tint, None, "colour returns when the body retires");
    }
}
