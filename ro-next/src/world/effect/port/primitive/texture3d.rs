//! `PP_3DTEXTURE` (1) and `PP_3DCROSSTEXTURE` (16) -
//! `CEffectPrim::MoveEffectPrimitive`, `EffectPrimitiveMotion.cpp:122-343`.
//!
//! The world-space textured sprite behind most self-buffs (Inc Agility,
//! Inc Agi-Dex, ...). The body's `pattern` bits select anchoring, movement and
//! camera-facing modes; the matrix is rebuilt into `rotTransform` every frame
//! while `transform` keeps the body's orientation for the forward vector.
//!
//! Camera/terrain-dependent branches read [`MasterState`]; when the renderer is
//! wired it fills those fields, and they default to zero/`None` in tests.

use super::super::angles::Angles;
use super::super::math::forward;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

/// `g_heightSpeedSign` - `EffectPrimitiveRendering.cpp:46`.
const HEIGHT_SPEED_SIGN: f32 = -1.0;
/// `g_masterReachDistance` - `EffectPrimitiveRendering.cpp:55`.
const MASTER_REACH_DISTANCE: f32 = 3.0;
/// `g_projectionDepthOffset` - `RenderProjection.cpp:11`.
const PROJECTION_DEPTH_OFFSET: f32 = 10.0;
/// `g_projectionDepthNumerator` - `RenderProjection.cpp:9`.
const PROJECTION_DEPTH_NUMERATOR: f32 = 1.0;

/// One frame of `MoveEffectPrimitive`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, _rng: &mut EffectRng) -> bool {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    if state.change_point == state.state_cnt {
        const RESET_VALUE: f32 = 0.0001;

        if state.change_width_speed != 0.0001 {
            state.width_speed = state.change_width_speed;
        }

        if state.change_height_speed != 0.0001 {
            state.height_speed = state.change_height_speed;
        }

        if state.change_speed != 0.0001 {
            state.speed = state.change_speed;
        }

        if state.change_width_accel != 0.0001 {
            state.width_accel = state.change_width_accel;
        }

        if state.change_height_accel != 0.0001 {
            state.height_accel = state.change_height_accel;
        }

        if state.change_accel != 0.0001 {
            state.accel = state.change_accel;
        }

        if state.change_speed3d[0] != 0.0001 {
            state.speed3d[0] = state.change_speed3d[0];
        }

        if state.change_speed3d[1] != 0.0001 {
            state.speed3d[1] = state.change_speed3d[1];
        }

        if state.change_speed3d[2] != 0.0001 {
            state.speed3d[2] = state.change_speed3d[2];
        }

        state.change_width_speed = 0.0001;
        state.change_height_speed = 0.0001;
        state.change_speed = 0.0001;
        state.change_width_accel = 0.0001;
        state.change_height_accel = 0.0001;
        state.change_accel = 0.0001;
        state.change_speed3d = [RESET_VALUE; 3];
        state.change_accel3d = [RESET_VALUE; 3];
    }

    state.speed += state.accel;
    state.width_speed += state.width_accel;
    state.speed = state.speed;
    state.height_speed += state.height_accel;
    state.long_speed += state.long_accel;
    state.lati_speed += state.lati_accel;

    if (state.pattern & 0x400u32) != 0 {
        state.speed3d = [0.0; 3];
        state.speed3d = forward(state.speed, &state.transform.values);
        state.delta_pos[0] += state.speed3d[0];
        state.delta_pos[2] += state.speed3d[2];
        state.delta_pos[1] += state.speed3d[1];
    } else if (state.pattern & 0x100u32) != 0 {
        state.delta_pos[0] += state.speed3d[0];
        state.delta_pos[1] += state.speed3d[1];
        state.delta_pos[2] += state.speed3d[2];
    } else {
        state.speed3d = forward(state.speed, &state.transform.values);
        state.delta_pos[0] += state.speed3d[0];
        state.delta_pos[2] += state.speed3d[2];
        state.delta_pos[1] += state.speed3d[1];
    }

    if (state.pattern & 0x400u32) != 0 {
        let distance =
            (master.master_delta_pos[0] * master.master_delta_pos[0] + master.master_delta_pos[2] * master.master_delta_pos[2]).sqrt();

        if master.master_reach_distance >= distance {
            state.master_reached = true;
            return false;
        }

        state.longitude = master.render_angle;
    }

    state.position[0] = state.org_pos[0] + state.delta_pos[0];
    state.position[1] = state.delta_pos[1] + state.org_pos[1];
    state.position[2] = state.delta_pos[2] + state.org_pos[2];

    state.width_size += state.width_speed;
    state.height_size += state.height_speed;
    state.longitude += state.long_speed;
    state.latitude += state.lati_speed;

    if state.height_size > state.max_height_size {
        state.height_accel = 0.0;
        state.height_speed = 0.0;
    }

    if (state.pattern & 0x200u32) != 0 {
        state.rot_transform.make_z_rotation(master.camera_pitch);
        state.rot_transform.append_y_rotation(angles.radian(master.render_angle as i32));
    } else {
        if (state.pattern & 0x40u32) != 0 {
            state.longitude = master.camera_yaw + master.camera_yaw_offset;
            state.roll = master.camera_pitch;
            state.rot_transform.make_x_rotation(angles.radian(state.latitude as i32));
            state.rot_transform.append_z_rotation(angles.radian(state.roll as i32));
        } else {
            state.rot_transform.make_z_rotation(angles.radian(state.roll as i32));
            state.rot_transform.append_x_rotation(angles.radian(state.latitude as i32));
        }

        state.rot_transform.append_y_rotation(angles.radian(state.longitude as i32));
    }

    if (state.pattern & 2u32) != 0 {
        let index = state.ch_cnt;
        state.ch_cnt = if index + 1 == 10 { 0 } else { index + 1 };
        let offset = index as f32 / PROJECTION_DEPTH_OFFSET;
        state.v1 = offset;
        state.v2 = offset + PROJECTION_DEPTH_NUMERATOR;
    }

    if (state.pattern & 0x10u32) != 0 {
        master.delta_pos_out.set([
            state.delta_pos2[0] + state.delta_pos[0],
            state.delta_pos[1] + state.delta_pos2[1],
            state.delta_pos[2] + state.delta_pos2[2],
        ]);
    }

    if (state.pattern & 4u32) != 0
        && let Some(height) = master.terrain_height
        && state.position[1] > height
    {
        state.pattern &= !4u32;
        state.state_cnt = state.duration - 25;
        state.speed = 0.0;
        master.flag_out.set(true);
    }

    let frame = state.state_cnt;

    if (state.pattern & 0x20u32) != 0 && frame % state.primitive_type.max(1) == 0 {
        state.height_speed *= HEIGHT_SPEED_SIGN;
    }

    if (state.pattern & 0x800u32) != 0 && (state.red == 254 || state.blue == 254 || state.green == 254) {
        if state.rgb_cycle_delay != 0 {
            state.rgb_cycle_cnt += 1;
        }

        if state.rgb_cycle_delay == state.rgb_cycle_cnt {
            state.red += state.red_speed;
            state.green += state.green_speed;
            state.blue += state.blue_speed;
            state.rgb_cycle_cnt = 0;
        }
    } else {
        state.red += state.red_speed;
        state.green += state.green_speed;
        state.blue += state.blue_speed;
    }

    if (state.pattern & 0x800u32) != 0 {
        if state.red > 254 {
            state.red = 0;
        } else if state.red < 0 {
            state.red = 254;
        }

        if state.green > 254 {
            state.green = 0;
        } else if state.green < 0 {
            state.green = 254;
        }

        if state.blue > 254 {
            state.blue = 0;
        } else if state.blue < 0 {
            state.blue = 254;
        }
    } else {
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
    }

    state.process_alpha(master);

    if state.state_cnt % state.anim_speed.max(1) == 0 {
        let next = state.cur_motion + 1;
        state.cur_motion = next;

        if state.repeat_anim == 0 {
            if next == state.total_texture {
                return false;
            }
        } else {
            state.cur_motion = (next as i32 % state.total_texture.max(1) as i32) as u32;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: super::super::PrimitiveId::Texture3d,
            duration: 100,
            fade_out_cnt: 100,
            anim_speed: 4,
            ..Default::default()
        }
    }

    #[test]
    fn moves_up_along_the_body_matrix() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        // Inc Agility sets the matrix to X rotation 90 -> forward is -y.
        state.transform.make_x_rotation(std::f32::consts::FRAC_PI_2);
        state.speed = 1.5;

        on_process(&mut state, &angles, &master, &mut rng);

        assert!(state.position[1] < 0.0);
        assert!(state.position[0].abs() < 1e-4);
    }

    #[test]
    fn pattern_2_cycles_the_depth_uv_offset() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.pattern = 2;

        on_process(&mut state, &angles, &master, &mut rng);

        assert_eq!(state.ch_cnt, 1);
        assert!((state.v1 - 0.0).abs() < 1e-6);
        assert!((state.v2 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn master_reach_stops_the_primitive() {
        let angles = Angles::new();
        let mut master = MasterState::default();
        master.master_delta_pos = [1.0, 0.0, 1.0];
        master.master_reach_distance = MASTER_REACH_DISTANCE;
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.pattern = 0x400;

        assert!(!on_process(&mut state, &angles, &master, &mut rng));
        assert!(state.master_reached);
    }
}
