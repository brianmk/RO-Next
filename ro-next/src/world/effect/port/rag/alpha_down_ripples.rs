//! `CRagEffect::AlphaDownRipples(name, kind)` - `RagEffect.cpp:9182-9236`.
//!
//! `EF_WARPZONE2` (321), the visual of a map connection warp (`JT_WARPNPC`,
//! launched by `CNpc::SetState` at `SoundMaker.cpp:1635`): a large
//! `alpha_down.tga` ground disc, a falling `ring_blue.tga` ring wave on frames
//! 1 and 2 (`PP_3DCASTING_4`, prim 65), and an orbiting `particle1.spr` mote
//! every 10 frames. The body resets its own `stateCnt` on frame 99999998, so
//! the disc restarts before the integer counter can wrap.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::{EffectPrimitiveState, PrimitiveId};
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `LaunchRippleRings` - `RagEffect.cpp:9160-9180`.
///
/// One `PP_3DCASTING_4` primitive with four falling rings. `kind` 0 copies the
/// caller's small size, `kind` 1 uses the fixed 18-unit size.
fn launch_ripple_rings(runtime: &mut EffectRuntime, name: &'static str, kind: i8, small_size: f32, turn: i32, growth: f32) {
    let lifetime = runtime.lifetime;
    let ripple = runtime.launch(PrimitiveId::Casting4, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(ripple);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);

    if kind == 0 {
        state.size = small_size;
    } else if kind == 1 {
        state.size = 18.0;
    }

    setup_ripple_slot(state, 0, (270 + turn) as i16, 2.5 + growth, (53 - turn) as i16);
    setup_ripple_slot(state, 1, turn as i16, 5.0 + growth, (60 - turn) as i16);
    setup_ripple_slot(state, 2, (90 + turn) as i16, 7.5 + growth, (55 - turn) as i16);
    setup_ripple_slot(state, 3, (180 + turn) as i16, 10.0 + growth, (50 - turn) as i16);
}

/// `SetupRippleSlot` - `RagEffect.cpp:9132-9155`.
fn setup_ripple_slot(state: &mut EffectPrimitiveState, slot_index: usize, rot_start: i16, distance: f32, rise_angle: i16) {
    let slot = &mut state.slots[slot_index];

    slot.life = 1;
    slot.process = 0;
    slot.full_display_angle = 360;
    slot.rot_start = rot_start;
    slot.alpha_b = 0;
    slot.distance = distance;
    slot.max_height = distance;
    slot.rise_angle = rise_angle;
    slot.vec_b_now[1] = 0.0;
    slot.height = [0.0; 21];
    slot.flag1 = [0; 21];
}

#[derive(Debug)]
pub struct AlphaDownRipples {
    name: &'static str,
    kind: i8,
    lifetime: i32,
}

impl AlphaDownRipples {
    pub fn new(name: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { name, kind, lifetime }
    }

    /// The `stateCnt == 0` ground disc (`PP_3DCIRCLE`, prim 12).
    fn spawn_disc(&self, runtime: &mut EffectRuntime) {
        let disc = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(disc);

        state.duration = self.lifetime;
        state.pattern |= 1;
        state.radius = 15.0;
        state.delta_pos2[1] -= 1.5;
        state.latitude = 90.0;
        state.alpha = 0.0;
        state.max_alpha = 128.0;
        state.alpha_speed = state.max_alpha / 10.0;
        state.fade_out_cnt = state.duration - 10;
        state.init_segment();
        state.texture = Some("effect\\alpha_down.tga");
    }

    /// The `stateCnt % 10 == 0` orbiting mote (`PP_3DPARTICLEORBIT`, prim 7).
    fn spawn_orbit(&self, runtime: &mut EffectRuntime) {
        let angles = Angles::shared();

        let radius = (runtime.rng.clamped_random(200) / 100) as f32 * 0.75 + 4.5;
        let longitude = runtime.rng.clamped_random(360) as f32;
        let long_speed = if runtime.rng.clamped_random(2) == 0 {
            2.5 - runtime.rng.clamped_random(11) as f32 * 0.1
        } else {
            runtime.rng.clamped_random(11) as f32 * 0.1 - 2.5
        };

        let size = (runtime.rng.clamped_random(10) / 10) as f32 + 0.7;

        let mote = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mote);

        state.radius = radius;
        state.radius_speed = 0.0005;
        state.duration = 70;
        state.longitude = longitude;
        state.long_speed = long_speed;
        state.grav_speed = -0.3;
        state.grav_accel = -(state.grav_speed / state.duration as f32) / 1.5;
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));

        let forward = [0.0f32, 0.0, state.radius];
        state.delta_pos2 = matrix_mult(forward, &state.transform.values);

        state.size = size;
        state.fade_out_cnt = state.duration - state.duration / 3;
        state.sprite = Some("이팩트\\particle1.spr");
        state.action = Some("이팩트\\particle1.act");
        state.init_segment();
    }
}

impl PortedEffect for AlphaDownRipples {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 99_999_998 {
            runtime.frame = 0;
        } else {
            if runtime.frame == 1 {
                launch_ripple_rings(runtime, self.name, self.kind, 11.0, 0, 0.0);
            }

            if runtime.frame == 2 {
                launch_ripple_rings(runtime, self.name, self.kind, 4.0, 1, 0.2);
            }
        }

        if runtime.frame == 0 {
            self.spawn_disc(runtime);
        }

        if runtime.frame % 10 != 0 {
            return true;
        }

        self.spawn_orbit(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::EffectRuntime;

    /// One body step with the primitive pass left out, so the assertions see
    /// the values the body wrote (the motion would integrate them).
    fn run_body(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime) -> bool {
        let alive = effect.on_process(runtime);
        runtime.frame += 1;
        alive
    }

    #[test]
    fn warpzone2_lays_the_disc_then_the_two_ring_waves() {
        let mut effect = AlphaDownRipples::new("effect\\ring_blue.tga", 0, 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 1);

        run_body(&mut effect, &mut runtime);

        // Frame 0: the ground disc plus the first orbit mote.
        assert_eq!(runtime.primitives.len(), 2);
        let disc = &runtime.primitives[0].state;
        assert_eq!(disc.count, PrimitiveId::Circle3d);
        assert_eq!(disc.duration, 99_999_999);
        assert_eq!(disc.radius, 15.0);
        assert_eq!(disc.pattern & 1, 1);
        assert_eq!(disc.alpha, 0.0);
        assert_eq!(disc.max_alpha, 128.0);
        assert_eq!(disc.texture, Some("effect\\alpha_down.tga"));

        let mote = &runtime.primitives[1].state;
        assert_eq!(mote.count, PrimitiveId::Orbit);
        assert_eq!(mote.duration, 70);
        assert_eq!(mote.grav_speed, -0.3);
        assert_eq!(mote.sprite, Some("이팩트\\particle1.spr"));

        // Frame 1: the 11-unit ring wave.
        run_body(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 3);
        let first = &runtime.primitives[2].state;
        assert_eq!(first.count, PrimitiveId::Casting4);
        assert_eq!(first.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(first.size, 11.0);
        assert_eq!(first.render_flag, 1 | 4);
        assert_eq!(first.slots[0].rot_start, 270);
        assert_eq!(first.slots[1].rot_start, 0);
        assert_eq!(first.slots[3].rot_start, 180);
        assert_eq!(first.slots[0].distance, 2.5);
        assert_eq!(first.slots[3].distance, 10.0);
        assert_eq!(first.slots[0].rise_angle, 53);
        assert_eq!(first.slots[3].rise_angle, 50);

        // Frame 2: the second ring wave (kind 0 keeps the caller's 4-unit
        // size; only kind 1 uses the fixed 18).
        run_body(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 4);
        let second = &runtime.primitives[3].state;
        assert_eq!(second.count, PrimitiveId::Casting4);
        assert_eq!(second.size, 4.0);
        assert_eq!(second.slots[0].rot_start, 271);
        assert_eq!(second.slots[0].distance, 2.7);
        assert_eq!(second.slots[0].rise_angle, 52);
        assert_eq!(second.slots[1].distance, 5.2);
        assert_eq!(second.slots[3].rise_angle, 49);

        // Frame 10: another orbit mote, no new ring wave.
        while runtime.frame <= 10 {
            run_body(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 5);
        assert_eq!(runtime.primitives[4].state.count, PrimitiveId::Orbit);
    }

    /// `deltaPos2.y -= 1.5f` (`RagEffect.cpp:9219`) is a **lift**, not a drop.
    ///
    /// The official client's world is Y-down (see `render3d`'s module docs), so
    /// subtracting 1.5 raises the `alpha_down.tga` disc 1.5 units off the floor
    /// to keep it from z-fighting with the terrain. The port's world is Y-up
    /// and the renderer flips the axis, so the disc lands above the ground.
    /// With the flip missing it sat 1.5 units *inside* the terrain; the floor
    /// then occluded the whole portal ("rendered under the floor").
    #[test]
    fn the_ground_disc_is_lifted_above_the_floor() {
        use crate::world::effect::port::render3d::world_space_position;

        let mut effect = AlphaDownRipples::new("effect\\ring_blue.tga", 0, 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 1);

        run_body(&mut effect, &mut runtime);

        let disc = &runtime.primitives[0].state;
        assert_eq!(disc.delta_pos2[1], -1.5);
        assert_eq!(disc.latitude, 90.0, "the disc lies flat on the ground");

        // Caster on flat ground: official anchor `-H`, disc `-H - 1.5`, port
        // render height `H + 1.5`.
        let ground = 7.0f32;
        let disc_world = world_space_position([0.0, -ground + disc.delta_pos2[1], 0.0]);
        let floor_world = world_space_position([0.0, -ground, 0.0]);

        assert!((disc_world.y - (ground + 1.5)).abs() < 1e-4);
        assert!(disc_world.y > floor_world.y, "the disc must float above the floor");
    }

    #[test]
    fn counter_reset_replays_the_disc() {
        let mut effect = AlphaDownRipples::new("effect\\ring_blue.tga", 0, 99_999_999);
        let mut runtime = EffectRuntime::new(99_999_999, [0.0, 0.0], 1);
        runtime.frame = 99_999_998;

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.frame, 0, "reset frame falls through to the disc");
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::Circle3d);
        assert_eq!(runtime.primitives[0].state.duration, 99_999_999);
    }
}
