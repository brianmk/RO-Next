//! `PP_3DSPLINE` (10) and `PP_3DSPIRAL` (11) - the shared helix quad ribbon,
//! `EffectPrimitiveMotion.cpp:8265-8288`, rendering dispatch
//! `EffectPrimitiveRendering.cpp:747-750` -> `RenderRecordQuads`
//! (`:5790-5842`).
//!
//! Both ids fall through to one inline body. It anchors to the master's world
//! position plus `deltaPos2`, samples the client trig table at
//! `longitude`/`longitude + longSpeed` to build a two-point arc on the XZ
//! plane, and emits the four corners of a vertical ribbon segment:
//!
//! ```text
//! segPos[0] = (x0, y0,                 z0)   left,  top
//! segPos[1] = (x1, y0 + heightSpeed,   z1)   right, top
//! segPos[2] = (x0, y0 - size,          z0)   left,  bottom
//! segPos[3] = (x1, y0 + heightSpeed - size, z1) right, bottom
//! ```
//!
//! `ProcessAlpha` then `ShiftSegment` push the strip one record down. The
//! renderer (`RenderRecordQuads`) draws each record as the two triangles
//! `(0,1,2)` and `(3,2,1)`, using `corners[]` only.
//!
//! Quirks kept:
//!
//! * `render.position` is never written, unlike most 3-D primitives. The
//!   `ShiftSegment` call still records it as `records[0].position`, so that
//!   field stays at its construction value while the corners remain correct.
//! * `x1`/`z1` sample `longitude + longSpeed` *before* the trailing `longitude
//!   += longSpeed`, and `deltaPos2.y += heightSpeed` happens after
//!   `ShiftSegment`, so this frame's strip uses the pre-update body.
//! * The body has no animation and does not return a `bool`; the dispatch arm
//!   is a statement and the primitive lives until `stateCnt > duration`.
//! * It consumes **no** RNG: it reads only `angles`, the four state arrays and
//!   the master position, so there is no stream to preserve (contrast the
//!   particle primitives, which consume `EffectRng` per axis).
//!
//! ## Registration status
//!
//! Every `LaunchEffectPrim` call site in the decompiled `RagEffect.cpp` passes
//! a literal primitive id, and no literal is 10 or 11. Both ids are wired into
//! the motion and renderer dispatches here, but no effect body is registered
//! for them: inventing one would contradict the official dispatch. The pair is
//! complete and ready for the first body that names them.

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PP_3DSPLINE`/`PP_3DSPIRAL` -
/// `EffectPrimitiveMotion.cpp:8265-8288`.
pub(super) fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.org_pos = [
        master.position[0] + state.delta_pos2[0],
        master.position[1] + state.delta_pos2[1],
        master.position[2] + state.delta_pos2[2],
    ];

    // `static_cast<int>` truncates toward zero; `as i32` does the same for the
    // degrees effects pass in.
    let x0 = angles.sine(state.longitude as i32) * state.radius + state.org_pos[0];
    let y0 = state.org_pos[1];
    let z0 = state.org_pos[2] - angles.cosine(state.longitude as i32) * state.radius;
    let x1 = angles.sine((state.longitude + state.long_speed) as i32) * state.radius + state.org_pos[0];
    let y1 = state.org_pos[1];
    let z1 = state.org_pos[2] - angles.cosine((state.longitude + state.long_speed) as i32) * state.radius;

    state.seg_pos[0] = [x0, y0, z0];
    state.seg_pos[1] = [x1, y1 + state.height_speed, z1];
    state.seg_pos[2] = [x0, y0 - state.size, z0];
    state.seg_pos[3] = [x1, (y1 + state.height_speed) - state.size, z1];
    state.process_alpha(master);
    state.shift_segment();
    state.longitude += state.long_speed;
    state.delta_pos2[1] += state.height_speed;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    const EPS: f32 = 1e-5;

    fn master(position: [f32; 3]) -> MasterState {
        // A same-length master keeps `LimitAlpha` from running the
        // "primitive outlives its master" fade branch.
        MasterState {
            position,
            duration: 100,
            state_cnt: 0,
            ..Default::default()
        }
    }

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::Spline3d,
            duration: 100,
            num_segment: 3,
            delta_pos2: [1.0, 2.0, 3.0],
            radius: 10.0,
            longitude: 30.7,
            long_speed: 10.5,
            height_speed: 3.0,
            size: 2.0,
            ..Default::default()
        }
    }

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < EPS, "expected {expected}, got {actual}");
    }

    #[test]
    fn first_step_builds_the_four_corners_from_the_trig_table() {
        let angles = Angles::new();
        let master = master([5.0, 6.0, 7.0]);
        let mut state = state();
        state.position = [9.0, 9.0, 9.0];

        on_process(&mut state, &angles, &master);

        // orgPos = master.position + deltaPos2 (the pre-update one).
        assert_eq!(state.org_pos, [6.0, 8.0, 10.0]);
        // `static_cast<int>` truncates 30.7 -> 30 and 41.2 -> 41.
        let x0 = angles.sine(30) * 10.0 + 6.0;
        let z0 = 10.0 - angles.cosine(30) * 10.0;
        let x1 = angles.sine(41) * 10.0 + 6.0;
        let z1 = 10.0 - angles.cosine(41) * 10.0;

        close(state.seg_pos[0][0], x0);
        close(state.seg_pos[0][1], 8.0);
        close(state.seg_pos[0][2], z0);
        close(state.seg_pos[1][0], x1);
        close(state.seg_pos[1][1], 8.0 + 3.0); // y0 + heightSpeed
        close(state.seg_pos[1][2], z1);
        close(state.seg_pos[2][0], x0);
        close(state.seg_pos[2][1], 8.0 - 2.0); // y0 - size
        close(state.seg_pos[2][2], z0);
        close(state.seg_pos[3][0], x1);
        close(state.seg_pos[3][1], (8.0 + 3.0) - 2.0);
        close(state.seg_pos[3][2], z1);

        // `render.position` is not written by this primitive (quirk).
        assert_eq!(state.position, [9.0, 9.0, 9.0]);
        // `ShiftSegment` records the stale render.position, not orgPos.
        assert_eq!(state.records[0].position, [9.0, 9.0, 9.0]);
        // The corners are segPos, not position.
        assert_eq!(state.records[0].corners, state.seg_pos);

        // Both updates happen after the strip is built.
        close(state.longitude, 41.2);
        close(state.delta_pos2[1], 5.0);
    }

    #[test]
    fn shift_segment_pushes_the_previous_frame_down_the_records() {
        let angles = Angles::new();
        let master = master([0.0, 0.0, 0.0]);
        let mut state = state();
        state.num_segment = 3;
        state.records = (0..3).map(|_| Default::default()).collect();

        on_process(&mut state, &angles, &master);
        let first = state.seg_pos;

        on_process(&mut state, &angles, &master);

        // The second frame's record 0 is the new strip; record 1 is the first
        // frame's record 0.
        assert_eq!(state.records[0].corners, state.seg_pos);
        assert_eq!(state.records[1].corners, first);
        // `longitude` advanced by longSpeed between frames.
        close(state.longitude, 30.7 + 10.5 + 10.5);
    }

    #[test]
    fn record_colour_uses_the_alpha_process_alpha_produced_this_frame() {
        let angles = Angles::new();
        let master = master([0.0, 0.0, 0.0]);
        let mut state = state();
        state.num_segment = 3;
        state.records = (0..3).map(|_| Default::default()).collect();
        state.alpha = 10.0;
        state.alpha_speed = 1.0;
        state.max_alpha = 254.0;
        state.fade_out_cnt = 100;
        state.red = 1;
        state.green = 2;
        state.blue = 3;

        on_process(&mut state, &angles, &master);

        // `ProcessAlpha` runs first: 10 + 1 = 11.
        close(state.alpha, 11.0);
        close(state.records[0].alpha, 11.0);
        assert_eq!(state.records[0].color, (11u32 << 24) | 0x00_01_02_03);

        // Record 1: alpha - 1 * (alpha / numSegment) = 11 - 3.666... -> byte 7.
        close(state.records[1].alpha, 11.0 - 11.0 / 3.0);
        assert_eq!(state.records[1].color >> 24, 7);
    }

    #[test]
    fn dispatch_ids_are_ten_and_eleven() {
        assert_eq!(PrimitiveId::Spline3d.id(), 10);
        assert_eq!(PrimitiveId::Spiral3d.id(), 11);
    }
}
