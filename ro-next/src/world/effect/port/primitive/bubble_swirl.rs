//! prim 87 - `CEffectPrim::PrimBubbleSwirl`,
//! `EffectPrimitiveMotion.cpp:6962-7068`; dispatch at
//! `EffectPrimitiveMotion.cpp:9811-9812`.
//!
//! The `EF_PRESSURE` (365) rain-of-crosses primitive spawned by
//! `CRagEffect::TexturedRain` (`RagEffect.cpp:5317-5378`). `EffectPrimId.h`
//! has no symbol between `PP_SUPERANGEL` (85) and `PP_RECT_UP2` (90), so the
//! port names the variant after its motion body.
//!
//! Slot 0 is the lead and slots 1..3 its three trailing copies, which lag by a
//! growing alpha drop (`-100`, then `-25`, then `-25`) and copy the lead's
//! angle, distance, `height[8]`, vector and texture index one step behind. The
//! lead sweeps its `fullDisplayAngle` by `-15` degrees per frame, rises by
//! `height[1]` (`3.0` for every `TexturedRain` kind) and, on the frame its
//! angle crosses `250..280` (or on frame 33, depending on `flag1[4]`), shakes
//! the screen. `kind == 4` uses the first branch and zeroes `flag1[4]` when it
//! fires, so its quake happens once; every other kind quakes on process 33.
//!
//! Quirks kept:
//!
//! * the trailing copies are written **before** the lead's alpha ramp, so the
//!   lag chain sees the previous frame's alphas,
//! * `fullDisplayAngle` is assigned twice per copy (the decompiler emits the
//!   same write before and after `vecBNow`),
//! * slot 0's alpha is capped only by the original's `short`; the body does not
//!   clamp it on the way up.
//!
//! `FaceCamera(this->state.slots)` runs here exactly like the original (the
//! hermetic runtime has no camera, so the live render path recomputes the two
//! angles at draw time; see
//! [`super::super::render_slots::render_field_panel_with_facing`]). The
//! `MM_QUAKE` sends are collected in
//! [`EffectPrimitiveState::pending_quakes`](super::EffectPrimitiveState::pending_quakes)
//! and drained by the runtime.
//!
//! The renderer is the shared `RenderFieldPanel`
//! (`EffectPrimitiveRendering.cpp:3312-3505`); `case 87` is in its case list at
//! `EffectPrimitiveRendering.cpp:580`.

use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// One frame of `PrimBubbleSwirl` (`EffectPrimitiveMotion.cpp:6962-7068`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(&mut state.slots, master);

    // `const CRagEffect* master = this->state.master;`
    // `this->render.position = master->render.position;`
    state.position = master.position;

    // `for (int ec = 0; ec < 1; ++ec)` - only slot 0 is the lead slot.
    for ec in 0..1usize {
        if state.slots[ec].life != 1 {
            continue;
        }

        let process = state.slots[ec].process + 1;
        state.slots[ec].process = process;

        if process <= 0 {
            continue;
        }

        // `if (process > 3)` - slot 3 copies slot 2.
        if process > 3 {
            let source = state.slots[ec + 2];

            copy_trail(&mut state.slots[ec + 3], &source, 25);
        }

        // `if (process > 2)` - slot 2 copies slot 1.
        if process > 2 {
            let source = state.slots[ec + 1];

            copy_trail(&mut state.slots[ec + 2], &source, 25);
        }

        // `if (process > 1)` - slot 1 copies slot 0.
        if process > 1 {
            let source = state.slots[ec];

            copy_trail(&mut state.slots[ec + 1], &source, 100);
        }

        // `if (process <= 10) alphaB += 20;`
        if process <= 10 {
            state.slots[ec].alpha_b += 20;
        }

        if state.slots[ec].flag1[4] == 4 {
            // `fullDisplayAngle -= 15`, then `+= 360` when it went negative.
            state.slots[ec].full_display_angle -= 15;

            if state.slots[ec].full_display_angle < 0 {
                state.slots[ec].full_display_angle += 360;
            }

            state.slots[ec].vec_b_pre[1] += state.slots[ec].height[1];
            state.slots[ec].vec_b_now = add(state.position, state.slots[ec].vec_b_pre);

            if state.slots[ec].vec_b_pre[1] >= -60.0
                && state.slots[ec].full_display_angle >= 250
                && state.slots[ec].full_display_angle <= 280
            {
                state.slots[ec].flag1[4] = 0;
                // `mode->SendMsg(MM_QUAKE, 0, 0, 0, 0);`
                state.pending_quakes += 1;
            }
        } else if process <= 32 {
            if state.slots[ec].flag1[4] == 0 || process <= 24 {
                state.slots[ec].full_display_angle -= 15;

                if state.slots[ec].full_display_angle < 0 {
                    state.slots[ec].full_display_angle += 360;
                }

                state.slots[ec].vec_b_pre[1] += state.slots[ec].height[1];
            }

            state.slots[ec].vec_b_now = add(state.position, state.slots[ec].vec_b_pre);
        } else {
            if state.slots[ec].flag1[4] == 0 && process == 33 {
                // `mode->SendMsg(MM_QUAKE, 0, 0, 0, 0);`
                state.pending_quakes += 1;
            }

            state.slots[ec].alpha_b -= 5;

            if state.slots[ec].alpha_b < 0 {
                state.slots[ec].alpha_b = 0;
            }
        }
    }
}

/// The six-field trailing copy of `PrimBubbleSwirl`
/// (`EffectPrimitiveMotion.cpp:6986-6999`, `:7002-7015`, `:7018-7031`).
///
/// Quirk kept: `fullDisplayAngle` is assigned twice (before and after
/// `vecBNow`), and the alpha drop is applied before the zero clamp.
fn copy_trail(destination: &mut EffectPrimitiveSlot, source: &EffectPrimitiveSlot, alpha_drop: i16) {
    destination.full_display_angle = source.full_display_angle;
    destination.distance = source.distance;
    destination.height[8] = source.height[8];
    destination.alpha_b = source.alpha_b - alpha_drop;

    if destination.alpha_b < 0 {
        destination.alpha_b = 0;
    }

    destination.vec_b_now = source.vec_b_now;
    destination.full_display_angle = source.full_display_angle;
    destination.flag1[0] = source.flag1[0];
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// `FaceCamera(EffectPrimitiveSlot*)` - `EffectPrimitiveMotion.cpp:100-114`.
///
/// Kept local, exactly like the other primitive ports.
fn face_camera(slots: &mut [EffectPrimitiveSlot; 4], master: &MasterState) {
    let yaw = master.camera_yaw as i16 as i32;
    slots[0].alpha_t = (360 - yaw) as i16;

    let pitch = master.camera_pitch as i16 as i32;
    let mut alpha_t = 360 - pitch + 90;

    if alpha_t >= 360 {
        alpha_t -= 360;
    }

    slots[1].alpha_t = alpha_t as i16;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn pressure_rain() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::BubbleSwirl,
            duration: 300,
            size: 0.0,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].distance = 12.0;
        state.slots[0].alpha_b = 0;
        state.slots[0].vec_b_pre = [0.0, -115.0, 0.0];
        state.slots[0].height[1] = 3.0;
        state.slots[0].full_display_angle = 0;
        state.slots[0].flag1[0] = 0;
        state.slots[0].flag1[2] = 7;
        state.slots[0].flag1[4] = 0;

        for slot in &mut state.slots[1..] {
            slot.life = 1;
            slot.alpha_b = 0;
        }

        state
    }

    #[test]
    fn bubble_swirl_is_primitive_87() {
        assert_eq!(PrimitiveId::BubbleSwirl.id(), 87);
    }

    #[test]
    fn the_lead_rises_and_sweeps_its_angle() {
        let mut state = pressure_rain();

        on_process(&mut state, &MasterState::default());

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        assert_eq!(slot.alpha_b, 20);
        assert_eq!(slot.vec_b_pre[1], -112.0);
        assert_eq!(slot.full_display_angle, 345);
        assert_eq!(slot.vec_b_now, [0.0, -112.0, 0.0]);
    }

    #[test]
    fn the_trailing_slots_copy_one_step_behind() {
        let mut state = pressure_rain();

        // Frame 1: only the lead updates.
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[1].full_display_angle, 0);
        assert_eq!(state.slots[2].full_display_angle, 0);

        // Frame 2: slot 1 copies the lead's frame-1 values (alpha - 100).
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[1].full_display_angle, 345);
        assert_eq!(state.slots[1].alpha_b, 0, "20 - 100 clamps to zero");
        assert_eq!(state.slots[1].vec_b_now, [0.0, -112.0, 0.0]);

        // Frame 3: slot 2 copies slot 1.
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[2].full_display_angle, 345);

        // Frame 4: slot 3 copies slot 2.
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[3].full_display_angle, 345);
    }

    #[test]
    fn kind_zero_shakes_the_screen_once_on_process_33() {
        let mut state = pressure_rain();
        state.slots[0].process = 31;

        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].process, 32);
        assert_eq!(state.pending_quakes, 0);

        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].process, 33);
        assert_eq!(state.pending_quakes, 1);

        on_process(&mut state, &MasterState::default());
        assert_eq!(state.pending_quakes, 1, "only process 33 shakes");
    }

    #[test]
    fn kind_four_quakes_on_the_angle_window_and_clears_the_kind() {
        let mut state = pressure_rain();
        state.slots[0].flag1[4] = 4;
        state.slots[0].full_display_angle = 265;
        state.slots[0].vec_b_pre[1] = -60.0;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.pending_quakes, 1);
        assert_eq!(state.slots[0].flag1[4], 0);
    }

    #[test]
    fn face_camera_writes_the_two_camera_angles() {
        let mut state = pressure_rain();
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            ..Default::default()
        };

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 360 - 20 + 90 - 360);
    }
}
