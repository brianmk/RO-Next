//! prim 53 (`PP_THROWITEM`) - `CEffectPrim::PrimThrowItem`,
//! `EffectPrimitiveMotion.cpp:2273-2388`; dispatch at
//! `EffectPrimitiveMotion.cpp:9916-9918`.
//!
//! The thrown-item trail: one lead slot follows a parabolic arc from the
//! caster to `deltaPos`, throwing a copy into each of the other three slots on
//! every odd frame (the trail alpha drops by 25, 25 and 150). The arc is
//! parametrised by `process / flag1[1]`: at `b = process / flag1[1]` the
//! height is `vecBPre.y - sin(180 * b) * scale`, where `scale` is 10 for
//! `flag1[2] == 0`, 1 for `flag1[2] == 15`, and 30 otherwise. Once
//! `b > 1.05` the slot stops flying and fades by 20 alpha per frame.
//!
//! Unlike `PrimShieldBoomerang`, this motion has no `process > 0` gate and its
//! lead slot spins `+10` degrees per frame (`+45` in the `flag1[2] == 15`
//! mode).
//!
//! The renderer is the shared `RenderFieldPanel`
//! (`EffectPrimitiveRendering.cpp:3312-3505`); `PP_THROWITEM` is in the
//! renderer's case list at `EffectPrimitiveRendering.cpp:573-592`. The bodies
//! that spawn it are `CRagEffect::ThrowItem` (`RagEffect.cpp:12182-12272`,
//! `rag/throw_item.rs`).

use super::super::angles::Angles;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState, get_angle};

/// One frame of prim 53. `PrimThrowItem` returns `void`, so the primitive only
/// retires through the shared `stateCnt > duration` tail.
///
/// `CEffectPrim::PrimThrowItem` - `EffectPrimitiveMotion.cpp:2273-2388`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    face_camera(&mut state.slots, master);

    // `this->render.position = master->render.position;`
    state.position = master.position;

    // `for (int ec = 0; ec < 1; ++ec)`: only slot zero is the lead slot.
    if state.slots[0].life != 1 {
        return;
    }

    let mut slot = state.slots[0];
    slot.process += 1;

    // `if (process % 2 == 1) { ...trail copies... }`
    if slot.process % 2 == 1 {
        if slot.process > 3 {
            let source = state.slots[2];
            copy_trail(&mut state.slots[3], &source, 25);
        }

        if slot.process > 2 {
            let source = state.slots[1];
            copy_trail(&mut state.slots[2], &source, 25);
        }

        if slot.process > 1 {
            let source = state.slots[0];
            copy_trail(&mut state.slots[1], &source, 150);
        }
    }

    if slot.flag1[2] == 15 {
        slot.full_display_angle += 45;
    } else {
        slot.full_display_angle += 10;
    }

    // A single fixup; a value that jumps past 720 would stay out of range,
    // but no body drives `fullDisplayAngle` that far.
    if slot.full_display_angle >= 360 {
        slot.full_display_angle -= 360;
    }

    // `if (process == 1) Get_Angle(deltaPos.x - vecBNow.x, deltaPos.z -
    // vecBNow.z, &rotStart);`
    if slot.process == 1 {
        let rx = state.delta_pos[0] - slot.vec_b_now[0];
        let rz = state.delta_pos[2] - slot.vec_b_now[2];
        slot.rot_start = get_angle(rx, rz);
    }

    // `b = (float)process / (float)flag1[1];`
    let mut b = slot.process as f32 / slot.flag1[1] as f32;

    if b > 1.05 {
        slot.alpha_b -= 20;

        if slot.alpha_b < 0 {
            slot.alpha_b = 0;
        }
    } else {
        if b > 1.0 {
            b = 1.0;
        }

        // `a = 180.0f * b; sinH = (int)a;` - C++ truncates toward zero, which
        // is also Rust's `as` cast for the non-negative values reachable here.
        let sin_h = (180.0 * b) as i32;
        slot.vec_b_pre[1] += slot.height[2];

        let sine = angles.raw_sine(sin_h as usize);

        if slot.flag1[2] == 0 {
            slot.vec_b_now[1] = slot.vec_b_pre[1] - sine * 10.0;
        } else if slot.flag1[2] == 15 {
            slot.vec_b_now[1] = slot.vec_b_pre[1] - sine;
        } else {
            slot.vec_b_now[1] = slot.vec_b_pre[1] - sine * 30.0;
        }

        // Horizontal advance along `rotStart`, in every flight frame.
        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }

    state.slots[0] = slot;
}

/// The six-field trail copy of `PrimThrowItem`
/// (`EffectPrimitiveMotion.cpp:2291-2335`).
///
/// Quirk kept: `fullDisplayAngle` is assigned twice, exactly as the
/// decompilation emits it, and the alpha drop clamps after the subtraction.
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

/// `FaceCamera(EffectPrimitiveSlot*)` - `EffectPrimitiveMotion.cpp:100-114`.
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

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::ThrowItem,
            duration: 300,
            ..Default::default()
        }
    }

    fn tick(state: &mut EffectPrimitiveState) {
        on_process(state, Angles::shared(), &MasterState::default());
    }

    #[test]
    fn primitive_id_is_53() {
        assert_eq!(PrimitiveId::ThrowItem.id(), 53);
    }

    #[test]
    fn face_camera_writes_the_camera_angles_and_follows_the_master() {
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            position: [4.0, 5.0, 6.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, Angles::shared(), &master);

        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 70);
        assert_eq!(state.position, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn the_lead_slot_spins_ten_degrees_per_frame() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].full_display_angle = 350;

        tick(&mut state);

        // 350 + 10 = 360 -> one subtraction.
        assert_eq!(state.slots[0].full_display_angle, 0);
    }

    #[test]
    fn the_thrown_weapon_mode_spins_forty_five_degrees_per_frame() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].full_display_angle = 100;
        state.slots[0].flag1[2] = 15;

        tick(&mut state);

        assert_eq!(state.slots[0].full_display_angle, 145);
    }

    #[test]
    fn process_one_heads_towards_the_effect_delta_pos() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 10;
        state.delta_pos = [0.0, 0.0, 10.0];

        tick(&mut state);

        // Get_Angle(0, 10) == 91 in the client's table.
        assert_eq!(state.slots[0].rot_start, 91);
    }

    #[test]
    fn the_trail_copies_on_odd_frames_with_the_official_alpha_drops() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 100;
        state.slots[0].alpha_b = 255;
        state.slots[0].full_display_angle = 40;
        state.slots[0].vec_b_now = [1.0, 2.0, 3.0];

        // Frame 1: process == 1, no copy yet.
        tick(&mut state);
        assert_eq!(state.slots[1].alpha_b, 0);
        tick(&mut state);

        // Capture slot 0 after two frames: the copy runs before this frame's
        // spin, alpha ramp and flight step, so these are the values slot 1
        // receives.
        let expected_alpha = state.slots[0].alpha_b;
        let expected_angle = state.slots[0].full_display_angle;
        let expected_vec = state.slots[0].vec_b_now;
        let expected_flag = state.slots[0].flag1[0];

        // Frame 3: slot 2 (> 2) and slot 1 (> 1).
        tick(&mut state);
        assert_eq!(state.slots[1].alpha_b, expected_alpha - 150);
        assert_eq!(state.slots[1].full_display_angle, expected_angle);
        assert_eq!(state.slots[1].vec_b_now, expected_vec);
        assert_eq!(state.slots[1].flag1[0], expected_flag);

        // Frame 5: process > 3, so slot 3 also refreshes from slot 2, which
        // was still unarmed at frame 3.
        tick(&mut state);
        tick(&mut state);
        assert_eq!(state.slots[3].alpha_b, 0, "slot 2 was never loaded");
    }

    #[test]
    fn the_first_trail_link_is_clamped_at_zero_alpha() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 100;
        state.slots[0].alpha_b = 100;
        state.slots[0].process = 2;

        // process 3: alpha 100 -> copy 100 - 150 -> clamp 0.
        tick(&mut state);

        assert_eq!(state.slots[1].alpha_b, 0);
    }

    #[test]
    fn the_flight_arc_uses_the_process_over_reach_ratio() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 10;
        state.slots[0].flag1[2] = 0;
        state.slots[0].height[1] = 1.0;
        state.slots[0].height[2] = 2.0;
        state.slots[0].rot_start = 0;
        state.slots[0].vec_b_pre = [7.0, 10.0, 9.0];
        state.slots[0].vec_b_now = [7.0, 0.0, 9.0];

        // `process == 1` heads the slot at `deltaPos`: aim `deltaPos` 10 units
        // along +x so `Get_Angle(10, 0) == 0` and the horizontal step stays
        // along +x.
        state.delta_pos = [17.0, 0.0, 9.0];

        let angles = Angles::shared();
        tick(&mut state);

        // b = 1 / 10 = 0.1, a = 18, sinH = 18. vecBPre.y += 2 first.
        let sine = angles.raw_sine(18);
        assert_eq!(state.slots[0].vec_b_pre[1], 12.0);
        assert_eq!(state.slots[0].vec_b_now[1], 12.0 - sine * 10.0);
        assert_eq!(state.slots[0].vec_b_now[0], 7.0 + angles.raw_cosine(0));
        assert_eq!(state.slots[0].vec_b_now[2], 9.0 + angles.raw_sine(0));
    }

    #[test]
    fn the_thrown_weapon_mode_uses_a_unit_sine_scale() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 10;
        state.slots[0].flag1[2] = 15;
        state.slots[0].height[1] = 1.0;
        state.slots[0].height[2] = 0.0;
        state.slots[0].vec_b_pre[1] = 0.0;
        state.slots[0].vec_b_now = [0.0; 3];

        tick(&mut state);

        let sine = Angles::shared().raw_sine(18);
        assert_eq!(state.slots[0].vec_b_now[1], -sine);
    }

    #[test]
    fn other_modes_use_a_thirty_times_sine_scale() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 10;
        state.slots[0].flag1[2] = 8;
        state.slots[0].height[1] = 1.0;
        state.slots[0].height[2] = 0.0;
        state.slots[0].vec_b_pre[1] = 0.0;
        state.slots[0].vec_b_now = [0.0; 3];

        tick(&mut state);

        let sine = Angles::shared().raw_sine(18);
        assert_eq!(state.slots[0].vec_b_now[1], -sine * 30.0);
    }

    #[test]
    fn beyond_one_point_zero_five_reach_the_slot_fades_by_twenty() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 10;
        state.slots[0].process = 11;
        state.slots[0].alpha_b = 255;
        state.slots[0].vec_b_now = [1.0, 2.0, 3.0];

        tick(&mut state);

        // process 12 / 10 = 1.2 > 1.05.
        assert_eq!(state.slots[0].process, 12);
        assert_eq!(state.slots[0].alpha_b, 235);
        assert_eq!(state.slots[0].vec_b_now, [1.0, 2.0, 3.0], "past the end, no step");

        state.slots[0].alpha_b = 5;
        tick(&mut state);
        assert_eq!(state.slots[0].alpha_b, 0, "the fade clamps at zero");
    }

    #[test]
    fn exactly_one_point_zero_five_still_flies_with_a_clamped_angle() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 20;
        state.slots[0].process = 20;
        state.slots[0].flag1[2] = 0;
        state.slots[0].height[2] = 1.0;
        state.slots[0].vec_b_pre[1] = 0.0;
        state.slots[0].vec_b_now = [0.0; 3];

        tick(&mut state);

        // process 21 / 20 = 1.05, not > 1.05: b is clamped to 1.0, sinH = 180.
        let sine = Angles::shared().raw_sine(180);
        assert_eq!(state.slots[0].vec_b_pre[1], 1.0);
        assert_eq!(state.slots[0].vec_b_now[1], 1.0 - sine * 10.0);
    }

    #[test]
    fn a_dead_lead_slot_does_not_touch_the_trails() {
        let mut state = state();
        state.slots[1].alpha_b = 77;

        tick(&mut state);

        assert_eq!(state.slots[1].alpha_b, 77);
    }
}
