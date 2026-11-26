//! `CRagEffect::Sphere` - `RagEffect.cpp:2017-2046`.
//!
//! `EF_SPHERE`: orbiting `particle1` motes every ten frames, with the radius
//! cycle switching at frame 95 (`chRadiSpeed`).

use super::super::angles::Angles;
use super::super::math::{forward, Matrix};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Sphere {
    lifetime: i32,
}

impl Sphere {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Sphere {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        // The original shortens the effect duration to `500 - stateCnt`.
        let remaining = 500 - frame;

        if frame % 10 == 0 {
            let mote = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(mote);

            state.duration = 300;
            state.radius = 0.1;
            state.radius_speed = 0.15;
            state.grav_speed = 0.15;
            state.longitude = 2.0;
            state.long_speed = 8.0;
            state.long_accel = -0.01;
            state.size = 1.2;

            let mut transform = Matrix::identity();
            transform.make_y_rotation(angles.radian(2));
            state.transform = transform;

            let rotated = forward(0.1, &state.transform.values);
            state.delta_pos2 = [rotated[0], rotated[1] - 30.0, rotated[2]];

            state.ch_radi_speed = 1;
            state.color_cycle_index = 0;
            state.ch_point = vec![95.0];
            state.ch_val1 = vec![-0.15];
            state.ch_val2 = vec![state.radius_accel];

            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.init_segment();
        }

        if frame >= remaining {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn sphere_emits_orbit_motes_every_ten_frames() {
        let mut effect = Sphere::new(500);
        let mut runtime = EffectRuntime::new(500, [0.0, 0.0], 681);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        while runtime.frame <= 10 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 2);

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::Orbit);
        assert_eq!(mote.duration, 300);
        assert_eq!(mote.radius_speed, 0.15);
        assert_eq!(mote.grav_speed, 0.15);
        // Spawned at 8.0, the orbit step applied longAccel.
        assert!(mote.long_speed <= 8.0 && mote.long_speed >= 7.0);
        assert_eq!(mote.long_accel, -0.01);
        assert_eq!(mote.size, 1.2);
        assert_eq!(mote.ch_radi_speed, 1);
        assert_eq!(mote.ch_point, vec![95.0]);
        assert_eq!(mote.ch_val1, vec![-0.15]);
        assert_eq!(mote.sprite, Some("이팩트\\particle1.spr"));
    }
}
