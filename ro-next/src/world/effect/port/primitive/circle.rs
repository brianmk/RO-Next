//! `PP_2DCIRCLE` - `EffectPrimitiveMotion.cpp:8149` and
//! `EffectPrimitiveRendering.cpp:5649`.
//!
//! The ring behind Bash, Frost Diver, Sonic Blow and many older effects. It
//! integrates `radius` and `radiusSpeed` once per frame and builds two
//! triangles per arc segment with a rotating UV offset (`uInc`).

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState, RenderTriangle, Vertex};

/// `case PP_2DCIRCLE` - one frame of motion.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position[0] = state.delta_pos2[0] + master.screen[0];
    state.position[1] = state.delta_pos2[1] + master.screen[1];
    state.radius_speed = state.radius_accel + state.radius_speed;
    state.radius = state.radius_speed + state.radius;

    if (state.pattern & 1u32) != 0 || state.radius < state.inner_size {
        state.size = state.radius;
    } else {
        state.size = state.inner_size;
    }

    state.process_alpha(master);
}

/// `CEffectPrim::Render2DCircle` - appends the ring's triangles.
pub fn render(state: &EffectPrimitiveState, angles: &Angles, triangles: &mut Vec<RenderTriangle>) {
    let mut u_inc = 0.0f32;
    let inner_size = state.radius - state.size;
    let temp = 360.0f32 - state.arc_angle;

    let mut i = 0.0f32;
    while i <= temp {
        let cs1 = angles.raw_cosine(i as usize);
        let sn1 = angles.raw_sine(i as usize);
        let cs2 = angles.raw_cosine((i + state.arc_angle) as usize);
        let sn2 = angles.raw_sine((i + state.arc_angle) as usize);

        triangles.push(RenderTriangle {
            vertices: [
                Vertex {
                    x: state.position[0] + inner_size * cs1,
                    y: state.position[1] - inner_size * sn1,
                    z: 0.000001,
                    rhw: 0.999999,
                    u: u_inc,
                    v: 1.0,
                    diffuse: state.argb,
                },
                Vertex {
                    x: state.position[0] + state.radius * cs2,
                    y: state.position[1] - state.radius * sn2,
                    z: 0.000001,
                    rhw: 0.999999,
                    u: u_inc + 0.25,
                    v: 0.0,
                    diffuse: state.argb,
                },
                Vertex {
                    x: state.position[0] + state.radius * cs1,
                    y: state.position[1] - state.radius * sn1,
                    z: 0.000001,
                    rhw: 0.999999,
                    u: u_inc,
                    v: 0.0,
                    diffuse: state.argb,
                },
            ],
            texture: 0,
        });

        triangles.push(RenderTriangle {
            vertices: [
                Vertex {
                    x: state.position[0] + inner_size * cs1,
                    y: state.position[1] - inner_size * sn1,
                    z: 0.000001,
                    rhw: 0.999999,
                    u: u_inc,
                    v: 1.0,
                    diffuse: state.argb,
                },
                Vertex {
                    x: state.position[0] + inner_size * cs2,
                    y: state.position[1] - inner_size * sn2,
                    z: 0.000001,
                    rhw: 0.999999,
                    u: u_inc + 0.25,
                    v: 1.0,
                    diffuse: state.argb,
                },
                Vertex {
                    x: state.position[0] + state.radius * cs2,
                    y: state.position[1] - state.radius * sn2,
                    z: 0.000001,
                    rhw: 0.999999,
                    u: u_inc + 0.25,
                    v: 0.0,
                    diffuse: state.argb,
                },
            ],
            texture: 0,
        });

        u_inc += 0.25;
        if u_inc >= 1.0 {
            u_inc = 0.0;
        }

        i += state.arc_angle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            radius: 100.0,
            size: 100.0,
            inner_size: 0.0,
            arc_angle: 90.0,
            position: [320.0, 240.0, 0.0],
            alpha: 255.0,
            max_alpha: 255.0,
            duration: 40,
            fade_out_cnt: 40,
            argb: 0x00FF_00FF,
            ..Default::default()
        }
    }

    #[test]
    fn integrates_speed_then_radius() {
        let master = MasterState::default();
        let mut state = state();
        state.inner_size = 50.0;
        state.radius = 0.0;
        state.radius_speed = 3.0;
        state.radius_accel = 0.1;

        on_process(&mut state, &master);

        assert_eq!(state.radius_speed, 3.1);
        assert!((state.radius - 3.1).abs() < 1e-6);
        assert_eq!(state.size, 3.1, "radius < innerSize branch");
    }

    #[test]
    fn pattern_bit_one_forces_size_to_radius() {
        let master = MasterState::default();
        let mut state = state();
        state.radius = 60.0;
        state.inner_size = 50.0;
        state.pattern = 1;

        on_process(&mut state, &master);

        assert_eq!(state.size, 60.0);

        state.pattern = 0;
        on_process(&mut state, &master);

        assert_eq!(state.size, 50.0);
    }

    #[test]
    fn ring_builds_two_triangles_per_segment() {
        let angles = Angles::new();
        let mut state = state();
        state.size = 50.0;
        let mut triangles = Vec::new();

        render(&state, &angles, &mut triangles);

        // 0, 90, 180, 270 -> 4 segments -> 8 triangles.
        assert_eq!(triangles.len(), 8);

        let v = triangles[0].vertices[0];
        assert!((v.x - (320.0 + 50.0)).abs() < 1e-3);
        assert!((v.y - 240.0).abs() < 1e-3);
        assert_eq!(v.v, 1.0);
        assert_eq!(v.u, 0.0);
        assert_eq!(v.z, 0.000001);
        assert_eq!(v.rhw, 0.999999);
    }

    #[test]
    fn uv_wraps_after_four_segments() {
        let angles = Angles::new();
        let mut state = state();
        state.arc_angle = 45.0;
        let mut triangles = Vec::new();

        render(&state, &angles, &mut triangles);

        assert_eq!(triangles.len(), 16);
        assert_eq!(triangles[0].vertices[0].u, 0.0);
        assert_eq!(triangles[2].vertices[0].u, 0.25);
        assert_eq!(triangles[4].vertices[0].u, 0.5);
        assert_eq!(triangles[6].vertices[0].u, 0.75);
        assert_eq!(triangles[8].vertices[0].u, 0.0);
    }
}
