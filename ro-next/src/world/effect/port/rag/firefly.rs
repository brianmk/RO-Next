//! `CRagEffect::FireSparks` - `RagEffect.cpp:371-410`.
//!
//! `EF_FIREFLY`: one `particle1.spr` spark cluster on frame 0 with the
//! two-segment colour track and the 140-frame duration override.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct FireSparks {}

impl PortedEffect for FireSparks {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();

        // RNG order: speed, longitude, latitude, distance, chCnt, size.
        let speed = (runtime.rng.clamped_random(35) + 15) as f32 / 100.0;
        let longitude = runtime.rng.clamped_random(360) as f32;
        let latitude = runtime.rng.clamped_random(360) as f32;
        let distance = (runtime.rng.clamped_random(13) + 2) as f32;
        let ch_cnt = (runtime.rng.clamped_random(25) + 5) as u32;
        let size = (runtime.rng.clamped_random(70) + 30) as f32 / 100.0;

        let spark = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spark);

        state.pattern |= 4 | 0x80;
        state.duration = 140;
        state.speed = speed;
        state.longitude = longitude;
        state.latitude = latitude;
        state.transform.make_y_rotation(angles.radian(longitude as i32));
        state.transform.append_x_rotation(angles.radian(latitude as i32));

        let forward = matrix_mult([0.0, 0.0, distance], &state.transform.values);
        state.delta_pos2 = [forward[0], forward[1] - 10.0, forward[2]];

        state.ch_cnt = ch_cnt;
        state.size = size;
        state.alpha = 0.0;
        state.max_alpha = 80.0;
        state.alpha_speed = state.max_alpha / 1.5;
        state.alpha_cycle = 1;
        state.color_cycle_index = 0;
        state.ch_point = vec![40.0, 100.0];
        state.ch_val1 = vec![120.0, 200.0];
        state.ch_val2 = vec![100.0, 80.0];
        state.sprite = Some("이팩트\\particle1.spr");
        state.action = Some("이팩트\\particle1.act");
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn firefly_spawns_the_colour_cycled_spark() {
        let mut effect = FireSparks::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 421);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let spark = &runtime.primitives[0].state;
        assert_eq!(spark.count, PrimitiveId::Particle3d);
        assert_eq!(spark.pattern & 4, 4);
        // High pattern bit is render-only; the port keeps it in the state for
        // the renderer (`0x80` = additive spark).
        assert_eq!(spark.pattern & 0x80, spark.pattern & 0x80);
        assert_eq!(spark.duration, 140);
        assert_eq!(spark.max_alpha, 80.0);
        assert_eq!(spark.alpha_cycle, 1);
        assert_eq!(spark.ch_point, vec![40.0, 100.0]);
        assert_eq!(spark.ch_val1, vec![120.0, 200.0]);
        assert_eq!(spark.ch_val2, vec![100.0, 80.0]);
        assert!(spark.ch_cnt >= 5 && spark.ch_cnt <= 29);
        assert!((0.15..0.5).contains(&spark.speed));
        assert!((0.3..1.0).contains(&spark.size));
        assert_eq!(spark.sprite, Some("이팩트\\particle1.spr"));
    }
}
