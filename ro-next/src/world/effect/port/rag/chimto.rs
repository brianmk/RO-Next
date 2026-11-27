//! `EF_CHIMTO` (267), `EF_FVOICE` (606), `EF_WINK` (607) -
//! `CRagEffect::CHIMTO(char flag)`, `RagEffect.cpp:14419-14511`.
//!
//! On frame 0 the body spawns one prim 46 sprite and seeds its single live slot
//! with the effect's target offset (`state.deltaPos`); the prim-46 motion then
//! re-anchors the sprite there. `flag` selects the sprite/action pair, the
//! animation speed and the size, and `SetAction` picks the ACT row from the
//! bearing between the caster and the target plus the camera yaw.
//!
//! Flags 12 and 13 are part of the same C++ function (`EF_CHIMTO2` passes 12;
//! 13 is unused by the dispatch), so they are kept here even though only flags
//! 0/1/2 are registered.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Chimto {
    flag: i8,
    lifetime: i32,
}

impl Chimto {
    pub fn new(flag: i8, lifetime: i32) -> Self {
        Self { flag, lifetime }
    }
}

impl PortedEffect for Chimto {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, this->state.parent->render.position);`
        runtime.copy_parent_position();

        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        // `if (flag == 0) PlayEffectWaveInline(*this, "effect\\mon_침투경.wav");`
        if self.flag == 0 {
            runtime.play_sound("effect\\mon_침투경.wav");
        }

        // `this->state.lastPrim = this->LaunchEffectPrim(46, {0, 0, 0});`
        let index = runtime.launch(PrimitiveId::ChimtoSprite, [0.0, 0.0, 0.0]);

        let target = EffectRuntime::official_position(runtime.target_offset);
        let lifetime = runtime.lifetime;

        // `Get_Angle(render.position.x - deltaPos.x, render.position.z -
        //  deltaPos.z, &angle)` then `(camera->camera.yaw + angle) % 360`.
        // Computed before the primitive borrow (both reads are side-effect
        // free and the original computes them after the sprite writes).
        let mut angle = runtime.get_angle(runtime.position[0] - target[0], runtime.position[2] - target[2]) as i32;
        angle = (runtime.camera_yaw as i32 + angle) % 360;

        if angle < 0 {
            angle += 360;
        }

        let state = runtime.primitive_mut(index);
        state.duration = lifetime;
        state.slots[0].life = 1;
        state.slots[0].max_height = 0.0;
        state.slots[0].vec_b_pre = [target[0], target[1] - 0.0, target[2]];

        match self.flag {
            0 => {
                state.alpha = 200.0;
                state.sprite = Some("이팩트\\침투경_.spr");
                state.action = Some("이팩트\\침투경_.act");
                state.anim_speed = 2;
                state.size = 2.5;
            }
            1 => {
                state.sprite = Some("이팩트\\wink.spr");
                state.action = Some("이팩트\\wink.act");
                state.anim_speed = 6;
                state.size = 1.0;
                state.repeat_anim = 0;
                state.pattern |= 0x200;
            }
            2 => {
                state.sprite = Some("이팩트\\fvoice.spr");
                state.action = Some("이팩트\\fvoice.act");
                state.anim_speed = 6;
                state.size = 1.0;
                state.repeat_anim = 0;
                state.pattern |= 0x200;
            }
            12 => {
                state.sprite = Some("이팩트\\헨드레드스피어.spr");
                state.action = Some("이팩트\\헨드레드스피어.act");
                state.anim_speed = 6;
                state.size = 1.0;
                state.repeat_anim = 0;
                state.pattern |= 0x200;
            }
            13 => {
                state.sprite = Some("이팩트\\윈드커터.spr");
                state.action = Some("이팩트\\윈드커터.act");
                state.anim_speed = 6;
                state.size = 1.0;
                state.repeat_anim = 0;
                state.pattern |= 0x200;
            }
            _ => {}
        }

        // `Get_Angle(render.position.x - deltaPos.x, render.position.z -
        //  deltaPos.z, &angle)` then `(camera->camera.yaw + angle) % 360`.
        // (Already computed above, before the primitive borrow.)

        // `state.renderFlag |= 0x200;` - `RSF_NO_DEPTH`.
        state.render_flag |= 0x200;

        // `SetAction(0/1/2/3, 16, 1)` - `CRenderObject::SetAction` keeps the
        // ACTION row, ignores `motion`/`repeat` and resets `curMotion`.
        state.action_index = if (180..270).contains(&angle) {
            0
        } else if (90..180).contains(&angle) {
            1
        } else if (0..90).contains(&angle) {
            2
        } else {
            3
        };
        state.cur_motion = 0;

        state.init_segment();
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn seeded(flag: i8, lifetime: i32) -> (Chimto, EffectRuntime) {
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], 1);
        runtime.position = [0.0, 0.0, 0.0];
        runtime.target_offset = [10.0, 0.0, 0.0];

        (Chimto::new(flag, lifetime), runtime)
    }

    #[test]
    fn chimto_spawns_the_penetration_sprite_at_the_target() {
        let (mut effect, mut runtime) = seeded(0, 16);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\mon_침투경.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ChimtoSprite);
        assert_eq!(state.sprite, Some("이팩트\\침투경_.spr"));
        assert_eq!(state.action, Some("이팩트\\침투경_.act"));
        assert_eq!(state.anim_speed, 2);
        assert_eq!(state.size, 2.5);
        assert_eq!(state.duration, 16);
        assert_eq!(state.alpha, 200.0);
        // The motion copies `vecBPre` into the anchor and the position.
        assert_eq!(state.position, [10.0, 0.0, 0.0]);
        assert_eq!(state.slots[0].vec_b_pre, [10.0, 0.0, 0.0]);
    }

    #[test]
    fn wink_and_fvoice_set_the_cell_colour_pattern() {
        for (flag, sprite) in [(1i8, "이팩트\\wink.spr"), (2, "이팩트\\fvoice.spr")] {
            let (mut effect, mut runtime) = seeded(flag, 100);

            step(&mut effect, &mut runtime);

            let state = &runtime.primitives[0].state;
            assert_eq!(state.sprite, Some(sprite));
            assert_eq!(state.pattern & 0x200, 0x200);
            assert_eq!(state.repeat_anim, 0);
            assert_eq!(state.render_flag & 0x200, 0x200);
        }
    }

    #[test]
    fn set_action_picks_the_row_from_the_bearing_and_camera_yaw() {
        // Caster at the origin, target at +x: `Get_Angle(0 - 10, 0 - 0)` is
        // 180 (the target is to the caster's -x), so with yaw 0 the row is 0.
        let (mut effect, mut runtime) = seeded(0, 16);
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.action_index, 0);

        // Camera yaw 180 turns that into 0..90, i.e. row 2.
        let (mut effect, mut runtime) = seeded(0, 16);
        runtime.camera_yaw = 180.0;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.action_index, 2);
    }

    #[test]
    fn only_the_first_frame_spawns() {
        let (mut effect, mut runtime) = seeded(0, 16);

        step(&mut effect, &mut runtime);
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
    }
}
