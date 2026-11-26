//! `PP_2DFLASH` - `EffectPrimitiveMotion.cpp:8121-8147` and
//! `EffectPrimitiveRendering.cpp:7487-7578`.
//!
//! The spark burst of Bash (20 of them) and countless other impacts. It
//! integrates `longSpeed`/`heightSpeed` per frame and renders one rotated quad
//! as two triangles.

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState, RenderTriangle, Vertex};

/// `case PP_2DFLASH` - one frame of motion.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.position[0] = state.delta_pos2[0] + master.screen[0];
    state.position[1] = state.delta_pos2[1] + master.screen[1];

    if (state.pattern & 1u32) != 0 && state.height_size <= 0.0 {
        state.height_speed = 0.0;
        state.height_accel = 0.0;
    }

    if state.state_cnt == state.change_point {
        state.height_speed = state.change_height_speed;
        state.long_speed = state.change_speed;
        state.long_accel = state.change_accel;
    }

    if state.height_size < 0.0 {
        state.height_speed *= -1.0;
        state.height_size = 0.0;
    }

    state.long_speed = state.long_accel + state.long_speed;
    state.height_speed = state.height_speed + state.height_accel;
    state.longitude = state.long_speed + state.longitude;
    state.height_size = state.height_speed + state.height_size;
    state.arc_angle = state.width_speed + state.arc_angle;

    // The original ignores `angles` in this branch; the parameter is kept so
    // the call site matches the other primitives.
    let _ = angles;

    state.process_alpha(master);
}

/// `CEffectPrim::Render2DPrimitive` - one rotated quad.
pub fn render(state: &EffectPrimitiveState, angles: &Angles, triangles: &mut Vec<RenderTriangle>) {
    let sin_roll = angles.sine(state.roll as i32);
    let cos_roll = angles.cosine(state.roll as i32);

    let width = state.width_size;
    let neg_width = -width;
    let height = state.height_size;
    let mut neg_height = -height;
    let mut extra = 0.0f32;

    if (state.pattern & 1) != 0 {
        neg_height += neg_height;
    } else {
        extra = height;
    }

    let x0 = neg_width * cos_roll;
    let y0 = neg_width * sin_roll;

    let p1_y = y0 + neg_height * cos_roll;
    let p1_x = x0 - neg_height * sin_roll;

    let p3_y = width * cos_roll - extra * sin_roll;
    let p3_x = width * sin_roll + extra * cos_roll;

    let px = state.position[0];
    let py = state.position[1];

    // QUIRK: the original adds `p3_y` to x and `p3_x` to y here. Keep it.
    triangles.push(RenderTriangle {
        vertices: [
            Vertex {
                x: (x0 - extra * sin_roll) + px - 0.5,
                y: (y0 + extra * cos_roll) + py - 0.5,
                z: 0.000001,
                rhw: 0.999999,
                u: 0.0,
                v: 1.0,
                diffuse: state.argb,
            },
            Vertex {
                x: (p1_x + px) - 0.5,
                y: (py + p1_y) - 0.5,
                z: 0.000001,
                rhw: 0.999999,
                u: 0.0,
                v: 0.0,
                diffuse: state.argb,
            },
            Vertex {
                x: (p3_y + px) - 0.5,
                y: (py + p3_x) - 0.5,
                z: 0.000001,
                rhw: 0.999999,
                u: 1.0,
                v: 1.0,
                diffuse: state.argb,
            },
        ],
        texture: state.cur_motion as u16,
    });

    triangles.push(RenderTriangle {
        vertices: [
            Vertex {
                x: (p3_y + px) - 0.5,
                y: (p3_x + py) - 0.5,
                z: 0.000001,
                rhw: 0.999999,
                u: 1.0,
                v: 1.0,
                diffuse: state.argb,
            },
            Vertex {
                x: (px + p1_x) - 0.5,
                y: (py + p1_y) - 0.5,
                z: 0.000001,
                rhw: 0.999999,
                u: 0.0,
                v: 0.0,
                diffuse: state.argb,
            },
            Vertex {
                // QUIRK: no -0.5 offset on this corner in the original.
                x: px + (width * cos_roll - neg_height * sin_roll),
                y: py + width * sin_roll + neg_height * cos_roll,
                z: 0.000001,
                rhw: 0.999999,
                u: 1.0,
                v: 0.0,
                diffuse: state.argb,
            },
        ],
        texture: state.cur_motion as u16,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_grows_and_position_offsets() {
        let angles = Angles::new();
        let master = MasterState {
            screen: [100.0, 200.0],
            state_cnt: 0,
            duration: 40,
            ..Default::default()
        };
        let mut state = EffectPrimitiveState {
            delta_pos2: [0.0, -40.0, 0.0],
            height_size: 20.0,
            height_speed: 2.0,
            duration: 40,
            fade_out_cnt: 40,
            max_alpha: 200.0,
            ..Default::default()
        };

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position[0], 100.0);
        assert_eq!(state.position[1], 160.0);
        assert_eq!(state.height_size, 22.0);
        // alpha starts at the constructor default (254) and is clamped to the
        // effect's maxAlpha (200) on the first step.
        assert_eq!(state.alpha, 200.0);
    }

    #[test]
    fn negative_height_flips_speed() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = EffectPrimitiveState {
            height_size: -1.0,
            height_speed: -3.0,
            ..Default::default()
        };

        on_process(&mut state, &angles, &master);

        // heightSize < 0 flips heightSpeed, then the integration adds it back:
        // 3 + 0 = 3.
        assert_eq!(state.height_size, 3.0);
        assert_eq!(state.height_speed, 3.0);
    }

    #[test]
    fn render_keeps_the_swapped_corner_quirk() {
        let angles = Angles::new();
        let state = EffectPrimitiveState {
            width_size: 10.0,
            height_size: 20.0,
            position: [100.0, 100.0, 0.0],
            argb: 0x00FF_00FF,
            ..Default::default()
        };
        let mut triangles = Vec::new();

        render(&state, &angles, &mut triangles);

        assert_eq!(triangles.len(), 2);
        // cos(0) = 1, sin(0) = 0 -> p3_y = 10, p3_x = 20. The first triangle's
        // third vertex adds p3_y to x and p3_x to y (swapped, as in the original).
        let v = triangles[0].vertices[2];
        assert!((v.x - (100.0 + 10.0 - 0.5)).abs() < 1e-3);
        assert!((v.y - (100.0 + 20.0 - 0.5)).abs() < 1e-3);
    }
}
