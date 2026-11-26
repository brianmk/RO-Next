//! `PP_2DTEXTURE` (and `PP_2DQUAD`) - `CEffectPrim::Move2DPrimitive`,
//! `EffectPrimitiveMotion.cpp:7719-7813`.
//!
//! The screen-space textured sprite behind most self-buffs (Endure, Increase
//! AGI, ...): it integrates speed/roll/width/height, moves along
//! `longitude` with `ClientSine`/`ClientCosine`, cycles colours and advances its
//! motion frame.
//!
//! Deviation (documented in `PORT_STATUS.md`): the `slots[0].life == 98` branch
//! of the original drives the *skill-cooldown overlay* (`g_activeSkillEffects`,
//! `g_cooldownOwner`). It is UI bookkeeping, not a world visual, and is not
//! ported here.

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

const SENTINEL: f32 = 0.0001;
/// `g_heightSpeedSign` - `EffectPrimitiveRendering.cpp:46`.
const HEIGHT_SPEED_SIGN: f32 = -1.0;

/// One frame of `Move2DPrimitive`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
    if state.change_point == state.state_cnt {
        if state.change_width_speed != SENTINEL {
            state.width_speed = state.change_width_speed;
        }

        if state.change_height_speed != SENTINEL {
            state.height_speed = state.change_height_speed;
        }

        if state.change_speed != SENTINEL {
            state.speed = state.change_speed;
        }

        if state.change_width_accel != SENTINEL {
            state.width_accel = state.change_width_accel;
        }

        if state.change_height_accel != SENTINEL {
            state.height_accel = state.change_height_accel;
        }

        if state.change_accel != SENTINEL {
            state.accel = state.change_accel;
        }
    }

    if (state.pattern & 4u32) != 0 {
        state.position[0] = state.delta_pos2[0];
        state.position[1] = state.delta_pos2[1];
    } else {
        state.position[0] = master.screen[0] + state.delta_pos2[0];
        state.position[1] = master.screen[1] + state.delta_pos2[1];
    }

    state.speed = state.accel + state.speed;
    state.roll_speed = state.roll_accel + state.roll_speed;
    state.width_speed += state.width_accel;
    state.height_speed = state.height_accel + state.height_speed;

    if (state.pattern & 2u32) != 0 && state.state_cnt >= state.ch_cnt as i32 {
        state.width_accel = 0.0;
        state.width_speed = 0.0;
        state.height_accel = 0.0;
        state.height_speed = 0.0;
    }

    if (state.pattern & 0x80u32) != 0 {
        if state.alpha_speed < 0.0 {
            if state.alpha <= state.min_alpha {
                state.alpha_speed *= HEIGHT_SPEED_SIGN;
            }
        } else if state.alpha >= state.max_alpha {
            state.alpha_speed *= HEIGHT_SPEED_SIGN;
        }
    }

    let direction_x = state.longitude as i32;
    state.roll = state.roll_speed + state.roll;
    state.width_size = state.width_speed + state.width_size;
    state.height_size = state.height_speed + state.height_size;

    let next_x = angles.sine(direction_x) * state.speed + state.delta_pos[0];
    let direction = state.longitude as i32;
    state.delta_pos[0] = next_x;

    let step_y = angles.cosine(direction) * state.speed;
    let next_y = state.delta_pos[1] - step_y;

    state.red = (state.red as u32).wrapping_add(state.red_speed as u32) as i32;
    state.position[0] = state.delta_pos[0] + state.position[0];
    state.green = (state.green as u32).wrapping_add(state.green_speed as u32) as i32;
    state.blue = (state.blue as u32).wrapping_add(state.blue_speed as u32) as i32;
    state.delta_pos[1] = next_y;
    state.position[1] = next_y + state.position[1];

    if state.red > 255 {
        state.red = 0;
    } else if state.red < 0 {
        state.red = 255;
    }

    if state.green > 255 {
        state.green = 255;
    } else if state.green < 0 {
        state.green = 0;
    }

    if state.blue > 255 {
        state.blue = 255;
    } else if state.blue < 0 {
        state.blue = 0;
    }

    if (state.pattern & 8u32) != 0 && state.state_cnt % state.ch_cnt.max(1) as i32 == 0 {
        state.longitude = rng.clamped_random(16) as f32 + (state.longitude - 8.0);
        state.ch_cnt = (rng.clamped_random(10) + 10) as u32;
    }

    state.process_alpha(master);

    if state.state_cnt % state.anim_speed.max(1) == 0 {
        let next = state.cur_motion + 1;
        let total = state.total_texture;

        state.cur_motion = next;

        if state.repeat_anim == 0 {
            if next == total {
                return false;
            }
        } else {
            state.cur_motion = (next as i32 % total.max(1) as i32) as u32;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: super::super::PrimitiveId::Texture2d,
            duration: 100,
            fade_out_cnt: 100,
            anim_speed: 4,
            ..Default::default()
        }
    }

    #[test]
    fn moves_along_longitude() {
        let angles = Angles::new();
        let master = MasterState {
            screen: [320.0, 240.0],
            ..Default::default()
        };
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.longitude = 90.0;
        state.speed = 2.0;

        on_process(&mut state, &angles, &master, &mut rng);

        // sin(90) = 1 -> x advances by 2, cos(90) = 0 -> y unchanged.
        assert!((state.position[0] - (320.0 + 2.0)).abs() < 1e-3);
        assert!((state.position[1] - 240.0).abs() < 1e-3);
        assert_eq!(state.delta_pos[0], 2.0);
    }

    #[test]
    fn change_point_applies_sentinel_values_only_once() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.change_point = 0;
        state.change_speed = 5.0;
        state.change_width_speed = SENTINEL; // must be ignored
        state.width_speed = 1.0;

        on_process(&mut state, &angles, &master, &mut rng);

        assert_eq!(state.speed, 5.0);
        assert_eq!(state.width_speed, 1.0, "sentinel values are not applied");
    }

    #[test]
    fn animation_frame_wraps_with_repeat() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.total_texture = 2;
        state.repeat_anim = 1;
        state.anim_speed = 1;

        for _ in 0..4 {
            assert!(on_process(&mut state, &angles, &master, &mut rng));
        }

        assert!(state.cur_motion < 2);
    }

    #[test]
    fn no_repeat_stops_at_the_last_frame() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.total_texture = 2;
        state.repeat_anim = 0;
        state.anim_speed = 1;

        assert!(on_process(&mut state, &angles, &master, &mut rng));
        assert!(!on_process(&mut state, &angles, &master, &mut rng));
    }
}
