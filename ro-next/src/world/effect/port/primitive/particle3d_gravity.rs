//! `PP_3DPARTICLEGRAVITY` (primitive 6) - `CEffectPrim::Prim3DParticleGravity`,
//! `EffectPrimitiveMotion.cpp:2581-2619`; dispatch at
//! `EffectPrimitiveMotion.cpp:8173-8177`.
//!
//! A sprite particle under constant forward acceleration and gravity. The
//! forward speed is clamped at zero (the original never lets it reverse), the
//! vertical speed is not - instead, when a rising particle crosses the terrain
//! it is divided by `-1.5f`, which is the original's bounce. `EF_STEAL` uses
//! this primitive for its ten `particle7.spr` sparks.
//!
//! The original reads `scene->terrain->Height(x, z)`; the port's hermetic
//! runtime supplies one terrain sample through [`MasterState::terrain_height`]
//! (`None` = no terrain context, no bounce).

use super::super::angles::Angles;
use super::super::math::forward;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `Prim3DParticleGravity`.
pub fn on_process(state: &mut EffectPrimitiveState, _angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    state.speed += state.accel;
    state.grav_speed += state.grav_accel;
    state.speed = if state.speed < 0.0 { 0.0 } else { state.speed };

    state.speed3d = forward(state.speed, &state.transform.values);
    state.delta_pos[0] += state.speed3d[0];
    state.delta_pos[1] += state.grav_speed;
    state.delta_pos[2] += state.speed3d[2];

    state.position = [
        state.org_pos[0] + state.delta_pos[0],
        state.org_pos[1] + state.delta_pos[1],
        state.org_pos[2] + state.delta_pos[2],
    ];
    state.size += state.size_speed;

    // `g_modeMgr.GetGameMode()->scene->terrain->Height(render.position.x, ...)`.
    if state.grav_speed > 0.0 {
        if let Some(terrain_height) = master.terrain_height {
            if state.position[1] > terrain_height {
                state.grav_speed /= -1.5;

                if state.play_wave_type == 1 {
                    let coin = rng.clamped_random(3) + 1;
                    state.pending_sounds.push(format!("effect\\EF_Coin{coin}.wav"));
                }
            }
        }
    }

    state.process_alpha(master);
    state.shift_segment();

    if !anim_motion(state) {
        return false;
    }

    true
}

/// `CEffectPrim::AnimMotion` - `EffectPrimitiveMotion.cpp:576-601`.
///
/// Mirrors the copy in [`super::particle3d`]; this file may only contain code
/// it owns, so the helper is repeated rather than borrowed.
fn anim_motion(state: &mut EffectPrimitiveState) -> bool {
    if state.state_cnt % state.anim_speed.max(1) != 0 {
        return true;
    }

    let frame_count = state.action_frame_count.max(1);

    state.cur_motion += 1;

    if state.repeat_anim == 0 {
        if (state.pattern & 0x800u32) != 0 {
            if state.cur_motion == frame_count {
                state.cur_motion -= 1;
            }
        } else if state.cur_motion == frame_count {
            return false;
        }
    } else {
        state.cur_motion %= frame_count;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn particle() -> EffectPrimitiveState {
        EffectPrimitiveState {
            duration: 50,
            speed: 1.0,
            accel: -0.01,
            grav_speed: -1.5,
            grav_accel: -0.03,
            size: 2.0,
            size_speed: -0.04,
            // The original's frame-0 fade-out test needs a real frame, not
            // the `-1` constructor default.
            fade_out_cnt: 50 - 50 / 3,
            ..Default::default()
        }
    }

    fn master() -> MasterState {
        MasterState {
            position: [10.0, 20.0, 30.0],
            state_cnt: 0,
            duration: 50,
            ..Default::default()
        }
    }

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-5
    }

    #[test]
    fn first_step_uses_the_official_operation_order() {
        let angles = Angles::new();
        let master = master();
        let mut rng = EffectRng::new(1);
        let mut state = particle();

        assert!(on_process(&mut state, &angles, &master, &mut rng));

        // orgPos = master.position + deltaPos2.
        assert_eq!(state.org_pos, [10.0, 20.0, 30.0]);
        // speed += accel; gravSpeed += gravAccel.
        assert!(close(state.speed, 0.99));
        assert!(close(state.grav_speed, -1.53));
        // Forward(speed3d, speed, transform) - identity transform is +Z.
        assert!(close(state.speed3d[0], 0.0));
        assert!(close(state.speed3d[2], 0.99));
        // deltaPos.x += speed3d.x; deltaPos.y += gravSpeed; deltaPos.z += speed3d.z.
        assert!(close(state.delta_pos[0], 0.0));
        assert!(close(state.delta_pos[1], -1.53));
        assert!(close(state.delta_pos[2], 0.99));
        assert!(close(state.position[1], 18.47));
        assert!(close(state.position[2], 30.99));
        // size += sizeSpeed.
        assert!(close(state.size, 1.96));
        // No bounce while the particle is still falling.
        assert!(close(state.grav_speed, -1.53));
        assert!(state.pending_sounds.is_empty());
    }

    #[test]
    fn forward_speed_is_clamped_to_zero() {
        let angles = Angles::new();
        let master = master();
        let mut rng = EffectRng::new(1);
        let mut state = particle();
        state.speed = 0.01;
        state.accel = -1.0;

        assert!(on_process(&mut state, &angles, &master, &mut rng));

        assert_eq!(state.speed, 0.0);
        assert_eq!(state.speed3d, [0.0, 0.0, 0.0]);
        assert_eq!(state.delta_pos[2], 0.0);
    }

    #[test]
    fn a_rising_particle_bounces_once_it_is_below_the_terrain_height() {
        let angles = Angles::new();
        let mut master = master();
        master.terrain_height = Some(0.0);
        let mut rng = EffectRng::new(1);
        let mut state = particle();
        state.grav_speed = 0.1;
        state.grav_accel = 0.0;

        assert!(on_process(&mut state, &angles, &master, &mut rng));

        // gravSpeed /= -1.5f.
        assert!(close(state.grav_speed, 0.1 / -1.5));
        // playWaveType is 0, so the bounce is silent.
        assert!(state.pending_sounds.is_empty());
    }

    #[test]
    fn a_coin_wave_is_requested_on_the_bounce_when_play_wave_type_is_one() {
        let angles = Angles::new();
        let mut master = master();
        master.terrain_height = Some(0.0);
        let mut rng = EffectRng::new(7);
        let mut state = particle();
        state.grav_speed = 0.1;
        state.grav_accel = 0.0;
        state.play_wave_type = 1;

        // The wave name consumes one `ClampedRandom(3)`.
        let mut expected_rng = EffectRng::new(7);
        let coin = expected_rng.clamped_random(3) + 1;

        assert!(on_process(&mut state, &angles, &master, &mut rng));

        assert_eq!(state.pending_sounds, vec![format!("effect\\EF_Coin{coin}.wav")]);
    }

    #[test]
    fn no_bounce_happens_below_the_terrain() {
        let angles = Angles::new();
        let mut master = master();
        master.terrain_height = Some(100.0);
        let mut rng = EffectRng::new(1);
        let mut state = particle();
        state.grav_speed = 0.1;
        state.grav_accel = 0.0;

        assert!(on_process(&mut state, &angles, &master, &mut rng));

        assert!(close(state.grav_speed, 0.1));
    }

    #[test]
    fn animation_expires_without_repeat() {
        let angles = Angles::new();
        let master = master();
        let mut rng = EffectRng::new(1);
        let mut state = particle();
        state.repeat_anim = 0;
        state.action_frame_count = 1;

        assert!(!on_process(&mut state, &angles, &master, &mut rng));
    }

    #[test]
    fn primitive_id_is_six() {
        assert_eq!(super::super::PrimitiveId::Particle3dGravity.id(), 6);
    }
}
