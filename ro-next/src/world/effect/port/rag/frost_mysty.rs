//! `EF_FROSTMYSTY` dispatch block - `RagEffect.cpp:20682-20689`.
//!
//! `SaintWing("effect\\freeze_a.bmp")` on frame 0 plus `SnowFall("", 1)` on
//! every frame (its own eight-frame gate), and `t_에너지방출.wav` on frame 10.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::snow_fall;

#[derive(Debug)]
pub struct FrostMysty {
    lifetime: i32,
}

impl FrostMysty {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }

    /// `CRagEffect::SaintWing(name)` - `RagEffect.cpp:5092-5119`.
    fn saint_wing(&self, runtime: &mut EffectRuntime) {
        let lifetime = runtime.lifetime;

        let wing = runtime.launch(PrimitiveId::RisingFan2, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(wing);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some("effect\\freeze_a.bmp");
        state.size = 0.0;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].distance = 24.0;
        state.slots[0].alpha_b = 0;
        state.slots[0].height[0] = 0.0;
        state.slots[0].flag1[0] = 0;
        state.slots[0].flag1[4] = 1;

        for index in 1..21 {
            state.slots[0].height[index] = 0.0;
        }
    }
}

impl PortedEffect for FrostMysty {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 10 {
            runtime.play_sound("effect\\t_에너지방출.wav");
        }

        if frame == 0 {
            self.saint_wing(runtime);
        }

        snow_fall::spawn(runtime, 1);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn frost_mysty_grows_the_wing_and_drops_snow() {
        let mut effect = FrostMysty::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 361);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2, "wing + first snow batch");

        let wing = &runtime.primitives[0].state;
        assert_eq!(wing.count, PrimitiveId::RisingFan2);
        assert_eq!(wing.duration, 300);
        assert_eq!(wing.render_flag, 5);
        assert_eq!(wing.texture, Some("effect\\freeze_a.bmp"));
        // 24.0 grown once by the fan primitive (x 1.01).
        assert!((wing.slots[0].distance - 24.24).abs() < 1e-4);
        assert_eq!(wing.slots[0].flag1[4], 1);
        assert_eq!(wing.slots[1].life, 0);

        let snow = &runtime.primitives[1].state;
        assert_eq!(snow.count, PrimitiveId::ParticleUp);
        assert_eq!(snow.texture, Some("effect\\E9_snow03.tga"));
    }

    #[test]
    fn wave_lands_on_frame_ten_and_snow_falls_every_eight() {
        let mut effect = FrostMysty::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 361);

        while runtime.frame < 10 {
            step(&mut effect, &mut runtime);
        }
        assert!(runtime.sounds.is_empty());

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\t_에너지방출.wav".to_owned()]);

        // Snow batches land on frames 0, 8 and 16.
        let before = runtime.primitives.len();
        while runtime.frame <= 16 {
            step(&mut effect, &mut runtime);
        }
        assert!(runtime.primitives.len() > before, "frame 16 adds a snow batch");
    }
}
