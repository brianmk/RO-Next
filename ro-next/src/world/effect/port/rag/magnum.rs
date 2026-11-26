//! `CRagEffect::MagnumBreak` - `RagEffect.cpp:9799-9849`, effect id
//! `EF_MAGNUMBREAK`, lifetime 30 frames (`EffectTypes.cpp:26`).
//!
//! Two expanding ground rings (`PP_3DCIRCLE`, `ring_yellow.tga`) and one
//! volumetric blast (`PP_3DSPHERE`, `대폭발.tga`). The rings accelerate
//! negatively, which is what produces the "blast wave that slows down" look.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_MAGNUMBREAK` lifetime in frames.
pub const LIFETIME: i32 = 30;

#[derive(Debug, Default)]
pub struct MagnumBreak {
    spawned: bool,
}

impl PortedEffect for MagnumBreak {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\EF_MagnumBreak.wav");

        let duration = runtime.lifetime;

        // First ring: `LaunchEffectPrim(12, ...)`.
        let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(ring);
            state.duration = duration;
            state.latitude = 90.0;
            state.radius_speed = 1.75;
            state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
            state.inner_size = 12.0;
            state.alpha = 0.0;
            state.alpha_speed = state.max_alpha / 15.0;
            state.fade_out_cnt = state.duration - 15;
            state.init_segment();
        }

        // The blast: `LaunchEffectPrim(14, ...)`.
        let blast = runtime.launch(PrimitiveId::Sphere3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(blast);
            state.duration = duration;
            state.radius_speed = 1.15;
            state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
            state.long_speed = 3.0;
            state.alpha = 0.0;
            state.alpha_speed = state.max_alpha / 15.0;
            state.fade_out_cnt = state.duration - 15;
        }

        // Second ring: `LaunchEffectPrim(12, ...)`, fixed 30-frame duration.
        let outer = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(outer);
            state.duration = 30;
            state.latitude = 90.0;
            state.radius_speed = 1.75;
            state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
            state.inner_size = 12.0;
            state.alpha = 0.0;
            state.alpha_speed = state.max_alpha / 15.0;
            state.fade_out_cnt = state.duration - 15;
            state.init_segment();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{run_to_completion, step, EffectRuntime};

    const EPSILON: f32 = 1e-4;

    #[test]
    fn spawns_two_rings_and_one_blast() {
        let mut effect = MagnumBreak::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [320.0, 240.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_MagnumBreak.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 3);
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::Circle3d);
        assert_eq!(runtime.primitives[1].state.count, PrimitiveId::Sphere3d);
        assert_eq!(runtime.primitives[2].state.count, PrimitiveId::Circle3d);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.duration, 30);
        assert_eq!(ring.latitude, 90.0);
        let expected_accel = -(1.75f32 / 30.0) * 0.5;
        assert!((ring.radius_accel - expected_accel).abs() < EPSILON);
        // One motion step has already integrated the acceleration.
        assert!((ring.radius_speed - (1.75 + expected_accel)).abs() < EPSILON);
        assert_eq!(ring.inner_size, 12.0);
        assert_eq!(ring.fade_out_cnt, 15);
        assert!((ring.alpha_speed - 254.0 / 15.0).abs() < EPSILON);

        let blast = &runtime.primitives[1].state;
        let blast_accel = -(1.15f32 / 30.0) * 0.5;
        assert!((blast.radius_speed - (1.15 + blast_accel)).abs() < EPSILON);
        assert!((blast.long_speed - 3.0).abs() < EPSILON);
        assert_eq!(blast.fade_out_cnt, 15);
    }

    #[test]
    fn rings_expand_outward_over_time() {
        let mut effect = MagnumBreak::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);
        let first = runtime.primitives[0].state.radius;

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        let later = runtime.primitives[0].state.radius;
        assert!(later > first, "ring keeps growing: {} -> {}", first, later);
        assert!(runtime.primitives[0].state.radius_accel < 0.0);
    }

    #[test]
    fn blast_rotates_and_keeps_its_world_position() {
        let mut effect = MagnumBreak::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        let blast = &runtime.primitives[1].state;
        assert_eq!(blast.longitude, 3.0);
        assert_eq!(blast.position, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn effect_retires_after_thirty_frames() {
        let mut effect = MagnumBreak::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 1);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 30);
        assert_eq!(snapshots.last().unwrap().frame, 30);
    }
}
