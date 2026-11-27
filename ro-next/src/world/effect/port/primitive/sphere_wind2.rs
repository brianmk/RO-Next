//! prim 91 `PP_SPHEREWIND2` - `CEffectPrim::PrimSphereWind2`,
//! `EffectPrimitiveMotion.cpp:4096-4143`; dispatch at
//! `EffectPrimitiveMotion.cpp:8676-8677`.
//!
//! The fire-vine sphere of `EF_BABY` (408), spawned by
//! `CRagEffect::FeatherBurst` (`RagEffect.cpp:4187-4220`) five times (one ring
//! per 9 degrees). It is the `SphereWind` renderer's second motion:
//! the slot radius grows 1% per frame, `riseAngle` turns by one or two degrees
//! (a `ClampedRandom(2)` draw adds the second), `rotStart` advances by
//! `height[1]` degrees, and the alpha ramps `+2` up to frame 25 and `-2` after
//! frame 70.
//!
//! The renderer is `RenderSphereWind` (`EffectPrimitiveRendering.cpp:5591`),
//! dispatched by the `case PP_SPHEREWIND2` arm at
//! `EffectPrimitiveRendering.cpp:404-405` - see
//! [`super::super::render_slots::render_sphere_wind`]. (`PP_SPHEREWIND` 77
//! shares it; that id is not ported.)

use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimSphereWind2` (`EffectPrimitiveMotion.cpp:4096-4143`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState, rng: &mut EffectRng) {
    // `FollowMaster(*this);`
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        let process = slot.process + 1;
        slot.process = process;

        if process <= 0 {
            continue;
        }

        if process <= 25 {
            slot.alpha_b += 2;
        } else if process > 70 {
            slot.alpha_b -= 2;
        }

        if slot.alpha_b < 0 {
            slot.alpha_b = 0;
        }

        slot.distance *= 1.01;
        slot.rise_angle += 1;

        if rng.clamped_random(2) == 0 {
            slot.rise_angle += 1;
        }

        if slot.rise_angle >= 360 {
            slot.rise_angle -= 360;
        }

        // `static_cast<std::int16_t>(static_cast<int>(static_cast<float>(rotStart) +
        // height[1]))`
        slot.rot_start = (slot.rot_start as f32 + slot.height[1]) as i32 as i16;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn sphere() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::SphereWind2,
            duration: 200,
            size: 10.0,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].max_height = 2.0;
        state.slots[0].distance = 5.0;
        state.slots[0].alpha_b = 0;
        state.slots[0].rot_start = 0;
        state.slots[0].rise_angle = 0;
        state.slots[0].height[0] = -10.0;
        state.slots[0].height[1] = 3.0;

        state
    }

    #[test]
    fn sphere_wind_two_is_primitive_91() {
        assert_eq!(PrimitiveId::SphereWind2.id(), 91);
    }

    #[test]
    fn the_sphere_grows_and_ramps_two_alpha_per_frame() {
        let mut state = sphere();

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(1));

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        assert_eq!(slot.alpha_b, 2);
        assert!((slot.distance - 5.05).abs() < 1e-5);
        assert!(slot.rise_angle == 1 || slot.rise_angle == 2);
        assert_eq!(slot.rot_start, 3);
    }

    #[test]
    fn the_rotation_advances_by_height_one_and_wraps() {
        let mut state = sphere();
        state.slots[0].rot_start = 357;
        state.slots[0].height[1] = 6.0;

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].rot_start, 3);
    }

    #[test]
    fn the_rise_angle_takes_the_two_degree_step_on_a_zero_roll() {
        let mut state = sphere();

        // Find a seed whose first `ClampedRandom(2)` is zero.
        let mut seed = 1u64;
        while EffectRng::new(seed).clamped_random(2) != 0 {
            seed += 1;
        }

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(seed));

        assert_eq!(state.slots[0].rise_angle, 2);
    }

    #[test]
    fn the_rise_angle_wraps_at_three_sixty() {
        let mut state = sphere();
        state.slots[0].rise_angle = 359;

        let mut seed = 1u64;
        while EffectRng::new(seed).clamped_random(2) == 0 {
            seed += 1;
        }

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(seed));

        assert_eq!(state.slots[0].rise_angle, 0);
    }

    #[test]
    fn the_drain_starts_after_frame_seventy() {
        let mut state = sphere();
        state.slots[0].process = 70;
        state.slots[0].alpha_b = 10;

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].alpha_b, 8);

        // Frame 70 itself neither ramps nor drains.
        let mut state = sphere();
        state.slots[0].process = 69;
        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn a_negative_process_holds_the_sphere_still() {
        let mut state = sphere();
        state.slots[0].process = -3;
        state.slots[0].rot_start = 30;

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].process, -2);
        assert_eq!(state.slots[0].distance, 5.0);
        assert_eq!(state.slots[0].rot_start, 30);
    }

    #[test]
    fn the_motion_follows_the_master() {
        let mut state = sphere();

        on_process(
            &mut state,
            &MasterState {
                position: [7.0, 8.0, 9.0],
                ..Default::default()
            },
            &mut EffectRng::new(1),
        );

        assert_eq!(state.position, [7.0, 8.0, 9.0]);
    }
}
