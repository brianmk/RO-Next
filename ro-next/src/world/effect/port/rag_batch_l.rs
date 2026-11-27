//! Effects ported by the "l" parallel batch (alternate particle spawn paths).
//!
//! ## `CThrowItemEffect` - the second `EF_THROWITEM*` implementation
//!
//! `EF_THROW_HAPPOKUNAI` (981), `EF_THROW_MULTIPLE_COIN` (982) and
//! `EF_THROW_BAKURETSU` (983) are not `CRagEffect` bodies. `EffectLists`
//! registers an allocator for each id
//! (`EffectLists.cpp:1482-1488`), and `CGameMode::LaunchTargetEffect`
//! (`WorldObjects.cpp:418-431`) spawns them through `g_effectMgr->Spawn`, not
//! through `source->LaunchEffect`. The body is `CThrowItemEffect`
//! (`ThrowEffects.cpp:20-615`), a `CEffect` subclass that creates `CParticle`
//! objects instead of `CEffectPrim` primitives.
//!
//! This module ports the three ids whose whole behaviour is
//! `CThrowItemEffect::Open`/`OnProcess` with no caster-derived data:
//!
//! | effect | id | allocator | `Open` case | lifetime |
//! | --- | --- | --- | --- | --- |
//! | `EF_THROW_HAPPOKUNAI` | 981 | `EffectLists.cpp:1485` | `:107-108`, `:213-234` | 200 (`:214`) |
//! | `EF_THROW_MULTIPLE_COIN` | 982 | `:1488` | `:110-111`, `:235-248` | 200 (`:236`) |
//! | `EF_THROW_BAKURETSU` | 983 | `:1482` | `:209-212` falling through to `:213-234` | 200 |
//!
//! `EffectTypes.cpp:990-992` carries `99999999` for these ids, but that table
//! is not used by `CEffect` subclasses: `CThrowItemEffect::Open` overwrites
//! `CEffect::effectLifetime` (the base constructor's 60, `Effect.cpp:23-32`)
//! with 200, and `IsAlive` is `!IsFinished()` (`ThrowEffects.cpp:494-496`,
//! `Effect.cpp:217-218`). The port therefore registers 200, the value the
//! original actually compares `frame` against.
//!
//! ## Particles
//!
//! `CParticle` lives in [`super::particle`]; [`super::holder`] renders it
//! through the same quad path as `PP_3DTEXTURE`. The body keeps the official
//! per-effect trail scalars (`travelFrames`, `heightDelta`, `arcHeight`,
//! `trail`, `trailRotation`, `trailAlpha`) and drives every particle in
//! `OnProcess`, exactly like the decompiled loop. No RNG is consumed: the only
//! randomness in the original path is absent from these three ids.
//!
//! ## Not ported
//!
//! * `EF_KAIHOU` (989): `Open` reads `owner->charmCount`/`charmType` through
//!   `FindRenderObject(this->actor)` (`ThrowEffects.cpp:113-118`, `:249-271`);
//!   the runtime has no charm channel and the asset is chosen at runtime.
//! * `EF_THROWITEM4_1` (980): only spawned as a side effect of
//!   `LaunchTargetEffect(EF_THROWITEM4)` (`WorldObjects.cpp:418-431`); the port
//!   has no `LaunchTargetEffect` equivalent, and registering 980 in isolation
//!   would replace the `EF_THROWITEM4` body instead of adding to it.

#![allow(dead_code)]

use super::angles::Angles;
use super::math::Matrix;
use super::particle::CastedParticle;
use super::runtime::{EffectRuntime, PortedEffect};

const EF_THROW_HAPPOKUNAI: i32 = 981;
const EF_THROW_MULTIPLE_COIN: i32 = 982;
const EF_THROW_BAKURETSU: i32 = 983;

/// `CThrowItemEffect` (`ThrowEffects.cpp:20-615`) restricted to the ids in
/// this batch.
///
/// One C++ class maps to one Rust body, with the per-id data selected by
/// [`Self::id`]. The `CEffect` base fields are supplied by [`EffectRuntime`].
pub struct ThrowItemParticles {
    id: i32,
    lifetime: i32,
    /// `CThrowItemEffect::travelFrames`
    travel_frames: i32,
    /// `CThrowItemEffect::launchDelay`
    launch_delay: i32,
    /// `CThrowItemEffect::projectileCount`
    projectile_count: i32,
    /// `CThrowItemEffect::trailCount`
    trail_count: i32,
    /// `CThrowItemEffect::heightDelta`
    height_delta: f32,
    /// `CThrowItemEffect::arcHeight`
    arc_height: f32,
    /// `CThrowItemEffect::trailAlpha`
    trail_alpha: i32,
    /// `CThrowItemEffect::trailRotation`
    trail_rotation: f32,
    /// `CThrowItemEffect::trail`
    trail: [f32; 3],
    /// Port-only: the official `Open()` runs once at spawn, before the first
    /// `Update` (`EffectLists::Spawn`, `EffectLists.cpp:380-390`). The port's
    /// body has no open hook, so the first `OnProcess` performs it.
    opened: bool,
}

impl ThrowItemParticles {
    /// `CThrowItemEffect::CThrowItemEffect` - `ThrowEffects.cpp:20-23`.
    pub fn new(id: i32) -> Self {
        Self {
            id,
            lifetime: 200,
            travel_frames: 0,
            launch_delay: 0,
            projectile_count: 1,
            trail_count: 4,
            height_delta: 0.0,
            arc_height: 0.0,
            trail_alpha: 0,
            trail_rotation: 0.0,
            trail: [0.0; 3],
            opened: false,
        }
    }

    /// `CThrowItemEffect::Open` - `ThrowEffects.cpp:105-278`.
    ///
    /// `this->sourcePosition` is the target actor's position:
    /// `LaunchTargetEffect` calls `Spawn(actor, id, source.position, targetAid,
    /// ...)` and `SetSourceActor` copies `target->render.position`
    /// (`EffectLists.cpp:366-369`, `Effect.cpp:120-126`). The runtime's
    /// `target_offset` is that same target position.
    fn open(&mut self, runtime: &mut EffectRuntime) {
        runtime.particles.clear();

        match self.id {
            EF_THROW_HAPPOKUNAI => self.projectile_count = 8,
            EF_THROW_MULTIPLE_COIN => self.projectile_count = 10,
            _ => {}
        }

        let position = EffectRuntime::official_position(runtime.position);
        let source_position = EffectRuntime::official_position(runtime.target_offset);

        for row in 0..self.projectile_count {
            for column in 0..self.trail_count {
                let mut particle = CastedParticle::new(position, 5);
                particle.effect_id = self.id;
                particle.column_index = column;
                particle.row_index = row;
                particle.group_index = 1;

                match self.id {
                    EF_THROW_BAKURETSU => {
                        // `case EF_THROW_BAKURETSU:` falls through into the
                        // HAPPOKUNAI arm (`ThrowEffects.cpp:209-213`).
                        self.init_spin_particle(&mut particle, position);
                        particle.speed_parameter = 1.0;
                        self.init_happokunai_particle(&mut particle, position);
                    }
                    EF_THROW_HAPPOKUNAI => self.init_happokunai_particle(&mut particle, position),
                    EF_THROW_MULTIPLE_COIN => self.init_multiple_coin_particle(&mut particle, position),
                    _ => {}
                }

                self.init_trajectory(&mut particle, position, source_position);
                runtime.launch_particle(particle);
            }
        }
    }

    /// `case EF_THROW_HAPPOKUNAI:` - `ThrowEffects.cpp:213-234`.
    ///
    /// The `sourceActor == 0` fallback (`:227-232`) is unreachable here:
    /// `LaunchTargetEffect` requires a target and passes its AID as
    /// `sourceActor`, so `SetSourceActor` always supplied a position.
    fn init_happokunai_particle(&mut self, particle: &mut CastedParticle, position: [f32; 3]) {
        self.effect_lifetime_200();
        self.arc_height = 1.0;
        particle.set_texture("유저인터페이스\\item\\쿠나이_독.bmp");
        particle.scale = 3.0;
        particle.set_color(255, 200, 255);
        particle.angle = 330.0;
        particle.speed_parameter = 1.5;
        particle.alpha = 255;
        particle.offset_x = position[0];
        particle.offset_z = position[2];
        particle.offset_y = position[1] - 8.0;
        particle.destination_blend = 6;
    }

    /// `case EF_THROW_MULTIPLE_COIN:` - `ThrowEffects.cpp:235-248`.
    fn init_multiple_coin_particle(&mut self, particle: &mut CastedParticle, position: [f32; 3]) {
        self.effect_lifetime_200();
        self.arc_height = 10.0;
        particle.set_texture("effect\\coin_a.bmp");
        particle.scale = 2.5;
        particle.set_color(255, 200, 0);
        particle.angle = 330.0;
        particle.speed_parameter = 2.0;
        particle.alpha = 255;
        particle.destination_blend = 6;
        particle.offset_z = position[2];
        particle.offset_y = position[1] - 8.0;
        particle.offset_x = position[0];
    }

    /// `CThrowItemEffect::InitSpinParticle` - `ThrowEffects.cpp:458-471`.
    fn init_spin_particle(&mut self, particle: &mut CastedParticle, position: [f32; 3]) {
        self.effect_lifetime_200();
        self.arc_height = 1.0;
        particle.scale = 2.0;
        particle.set_color(255, 200, 255);
        particle.angle = 330.0;
        particle.speed_parameter = 3.0;
        particle.alpha = 255;
        particle.offset_x = position[0];
        particle.offset_y = position[1] - 8.0;
        particle.offset_z = position[2];
        particle.destination_blend = 6;
    }

    /// `this->effectLifetime = 200;` - the value every arm of these three ids
    /// writes (`ThrowEffects.cpp:214`, `:236`, `:459`).
    fn effect_lifetime_200(&mut self) {
        self.lifetime = 200;
    }

    /// `CThrowItemEffect::InitTrajectory` - `ThrowEffects.cpp:593-615`.
    fn init_trajectory(&mut self, particle: &mut CastedParticle, position: [f32; 3], source_position: [f32; 3]) {
        let delta_x = source_position[0] - position[0];
        let delta_z = source_position[2] - position[2];
        self.height_delta = source_position[1] - position[1];

        let heading = direction_angle(delta_x, delta_z) as i32 as u16;

        // `if (this->id == 981)` only; every other id keeps offset 0.
        let offset = if self.id == EF_THROW_HAPPOKUNAI {
            let base = (360 / self.projectile_count) as i16;
            (base as i32 * particle.row_index as i32) as u16 as i32
        } else {
            0
        };

        particle.phase_parameter = ((heading as i32 + offset) as i16 as i32) % 360;

        let distance = (delta_z * delta_z + delta_x * delta_x).sqrt();
        particle.animation_parameter = distance;

        for step in 0..100 {
            if step as f32 * particle.speed_parameter > distance {
                self.travel_frames = step;
                return;
            }
        }
    }

    /// `CThrowItemEffect::IsAlive` - `ThrowEffects.cpp:494-496`:
    /// `return !this->IsFinished();` (`Effect.cpp:217-218`).
    fn is_alive(&self, runtime: &EffectRuntime) -> bool {
        runtime.frame < self.lifetime
    }

    /// `CThrowItemEffect::OnProcess` - `ThrowEffects.cpp:500-579`.
    fn process_particles(&mut self, runtime: &mut EffectRuntime) {
        let angles = Angles::shared();
        let position = EffectRuntime::official_position(runtime.position);
        let frame = runtime.frame;
        let camera_pitch = runtime.camera_pitch;
        let mut parent_angle = runtime.parent_angle;

        // These members are uninitialised in the C++ constructor
        // (`ThrowEffects.cpp:20-23` lists only `trail`); the first lead
        // particle writes them before any trail column reads them. Starting
        // from zero keeps the port deterministic without changing the order.
        let mut trail = self.trail;
        let mut trail_rotation = self.trail_rotation;
        let mut trail_alpha = self.trail_alpha;

        for particle in &mut runtime.particles {
            particle.position = position;

            if self.launch_delay <= frame {
                let mut rotation = particle.angle;

                if particle.column_index == 0 {
                    rotation = ((rotation + 45.0) as i32 % 360) as f32;
                } else {
                    trail_alpha -= if particle.column_index != 1 { 25 } else { 150 };
                    particle.alpha = trail_alpha;
                    particle.offset_x = trail[0];
                    particle.offset_y = trail[1];
                    particle.offset_z = trail[2];
                    rotation = trail_rotation;
                }

                let progress = frame as f32 / self.travel_frames as f32;
                let mut height = 0.0;

                if progress > 1.05 {
                    particle.alpha = (particle.alpha - 20).max(0);
                } else {
                    let index = particle.phase_parameter;
                    let radius = particle.speed_parameter / particle.animation_parameter;
                    let moved_x = particle.speed_parameter * angles.raw_cosine(index as usize) + particle.offset_x;
                    let moved_y = radius * self.height_delta + particle.offset_y;
                    let moved_z = particle.speed_parameter * angles.raw_sine(index as usize) + particle.offset_z;
                    particle.offset_x = moved_x;
                    particle.offset_y = moved_y;
                    particle.offset_z = moved_z;
                    // `g_sinTable[-static_cast<int>(progress * -180.0f)]`.
                    let sin_index = -((progress * -180.0) as i32);
                    height = angles.raw_sine(sin_index as usize) * self.arc_height;
                }

                particle.angle = rotation;

                trail = [particle.offset_x, particle.offset_y, particle.offset_z];
                trail_rotation = rotation;
                trail_alpha = particle.alpha;

                if particle.alpha > 0 {
                    let mut view = Matrix::identity();
                    view.make_x_rotation(camera_pitch / 180.0 * 3.141592);
                    view.append_y_rotation(rotation / 180.0 * 3.141592);

                    let mut scale = Matrix::identity();
                    scale.set_scale(particle.scale, particle.scale, 1.0);

                    let mut translate = Matrix::identity();
                    translate.make_translate(particle.offset_x, particle.offset_y - height, particle.offset_z);

                    let mut rotation_matrix = Matrix::identity();
                    rotation_matrix.compose(&scale, &view);

                    let mut transform = Matrix::identity();
                    transform.compose(&rotation_matrix, &translate);
                    particle.transform = transform;

                    // `if (this->id == 0x3D5 && target->rowIndex == 0 &&
                    // target->columnIndex == 0)` - the official body spins the
                    // caster actor (`ThrowEffects.cpp:566-573`). The port has no
                    // actor object; `runtime.parent_angle` is the caster angle
                    // the client spawn path supplies, and the write is kept for
                    // the visual bridge.
                    if self.id == EF_THROW_HAPPOKUNAI && particle.row_index == 0 && particle.column_index == 0 {
                        parent_angle = ((parent_angle + 45.0) as i32 % 360) as f32;
                    }
                }
            }

            particle.elapsed_frames += 1;
        }

        self.trail = trail;
        self.trail_rotation = trail_rotation;
        self.trail_alpha = trail_alpha;
        runtime.parent_angle = parent_angle;
    }
}

impl PortedEffect for ThrowItemParticles {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CEffect::Update`'s `OnProcess` call (`Effect.cpp:129-169`).
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if !self.is_alive(runtime) {
            return false;
        }

        if !self.opened {
            self.open(runtime);
            self.opened = true;
        }

        self.process_particles(runtime);

        true
    }
}

/// `DirectionAngle(float x, float z)` - `EffectLists.cpp:135-144`.
fn direction_angle(x: f32, z: f32) -> f32 {
    let length = (x * x + z * z).sqrt();
    let angle = ((1.0f32 / length * x) as f64).acos() as f32 / 3.141592f32 * 180.0f32;

    if z < 0.0 {
        return 360.0 - angle;
    }

    angle
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    const POSITION: [f32; 3] = [3.0, 4.0, 5.0];
    const TARGET: [f32; 3] = [13.0, 8.0, 5.0];

    fn new_runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);
        runtime.position = POSITION;
        runtime.target_offset = TARGET;
        runtime
    }

    fn spawn(id: i32) -> (ThrowItemParticles, EffectRuntime) {
        let mut effect = ThrowItemParticles::new(id);
        let mut runtime = new_runtime();
        effect.on_process(&mut runtime);

        (effect, runtime)
    }

    fn run(effect: &mut ThrowItemParticles, runtime: &mut EffectRuntime, frames: usize) {
        for _ in 0..frames {
            assert!(step(effect, runtime));
        }
    }

    fn particle_snapshot(runtime: &EffectRuntime) -> Vec<(i32, i32, i32, i32, f32, i32, f32, f32, [f32; 3], Matrix)> {
        runtime
            .particles
            .iter()
            .map(|particle| {
                (
                    particle.effect_id,
                    particle.row_index,
                    particle.column_index,
                    particle.phase_parameter,
                    particle.animation_parameter,
                    particle.alpha,
                    particle.speed_parameter,
                    particle.angle,
                    [particle.offset_x, particle.offset_y, particle.offset_z],
                    particle.transform,
                )
            })
            .collect()
    }

    #[test]
    fn the_batch_ids_are_the_three_alternate_throw_effects() {
        use crate::world::effect::port::registry_batch_l;

        let names: Vec<String> = registry_batch_l::ids().iter().map(|id| format!("{id:?}")).collect();
        assert_eq!(names, vec![
            "ThrowHappokunai".to_owned(),
            "ThrowMultipleCoin".to_owned(),
            "ThrowBakuretsu".to_owned()
        ]);
    }

    #[test]
    fn happokunai_spawns_eight_projectiles_with_four_trails_each() {
        let (effect, runtime) = spawn(EF_THROW_HAPPOKUNAI);

        assert_eq!(effect.lifetime(), 200);
        assert_eq!(runtime.particles.len(), 32);

        for (index, particle) in runtime.particles.iter().enumerate() {
            assert_eq!(particle.effect_id, 981);
            assert_eq!(particle.group_index, 1);
            assert_eq!(particle.row_index, index as i32 / 4);
            assert_eq!(particle.column_index, index as i32 % 4);
        }
    }

    #[test]
    fn happokunai_particles_carry_the_official_spin_values() {
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();
        effect.open(&mut runtime);
        let particle = &runtime.particles[0];

        assert_eq!(particle.texture, Some("유저인터페이스\\item\\쿠나이_독.bmp"));
        assert_eq!(particle.scale, 3.0);
        assert_eq!((particle.red, particle.green, particle.blue), (255, 200, 255));
        assert_eq!(particle.angle, 330.0);
        assert_eq!(particle.speed_parameter, 1.5);
        assert_eq!(particle.alpha, 255);
        assert_eq!(particle.destination_blend, 6);
        assert_eq!(particle.animation_parameter, 10.0);
        assert_eq!(particle.phase_parameter, 0);
        // `CParticle.position` is official (Y-down) simulation space.
        assert_eq!(particle.position, [3.0, -4.0, 5.0]);
    }

    #[test]
    fn happokunai_adds_the_row_phase_offset() {
        let (effect, runtime) = spawn(EF_THROW_HAPPOKUNAI);

        assert_eq!(effect.projectile_count, 8);
        assert_eq!(effect.trail_count, 4);

        for row in 0..8 {
            for column in 0..4 {
                assert_eq!(runtime.particles[row * 4 + column].phase_parameter, row as i32 * 45);
            }
        }
    }

    #[test]
    fn multiple_coin_spawns_ten_by_four_with_the_coin_values() {
        let mut effect = ThrowItemParticles::new(EF_THROW_MULTIPLE_COIN);
        let mut runtime = new_runtime();
        effect.open(&mut runtime);

        assert_eq!(runtime.particles.len(), 40);
        assert_eq!(effect.projectile_count, 10);
        assert_eq!(effect.trail_count, 4);
        assert_eq!(effect.arc_height, 10.0);
        assert_eq!(effect.travel_frames, 6);

        let particle = &runtime.particles[0];
        assert_eq!(particle.texture, Some("effect\\coin_a.bmp"));
        assert_eq!(particle.scale, 2.5);
        assert_eq!((particle.red, particle.green, particle.blue), (255, 200, 0));
        assert_eq!(particle.angle, 330.0);
        assert_eq!(particle.speed_parameter, 2.0);
        assert_eq!(particle.alpha, 255);
        assert_eq!(particle.destination_blend, 6);
        assert_eq!(particle.phase_parameter, 0);
        assert_eq!(particle.row_index, 0);
    }

    #[test]
    fn bakuretsu_falls_through_into_the_poison_kunai_arm() {
        // The official `case EF_THROW_BAKURETSU` has no `break` and falls
        // through into `case EF_THROW_HAPPOKUNAI` (`ThrowEffects.cpp:209-213`),
        // which overwrites the fire texture with the poison one and the
        // `InitSpinParticle` speed 3.0 with 1.5.
        let mut effect = ThrowItemParticles::new(EF_THROW_BAKURETSU);
        let mut runtime = new_runtime();
        effect.open(&mut runtime);

        assert_eq!(runtime.particles.len(), 4);
        assert_eq!(effect.projectile_count, 1);
        assert_eq!(effect.arc_height, 1.0);
        assert_eq!(effect.travel_frames, 7);

        let particle = &runtime.particles[0];
        assert_eq!(particle.texture, Some("유저인터페이스\\item\\쿠나이_독.bmp"));
        assert_eq!(particle.scale, 3.0);
        assert_eq!(particle.speed_parameter, 1.5);
        assert_eq!(particle.phase_parameter, 0);
        assert_eq!(particle.angle, 330.0);
    }

    #[test]
    fn the_body_retires_at_frame_two_hundred() {
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();

        let snapshots = crate::world::effect::port::runtime::run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 200, "frames 0..=199 were simulated");
        assert_eq!(runtime.frame, 200);
        assert_eq!(effect.lifetime(), 200);
    }

    #[test]
    fn the_lead_particle_spins_forty_five_degrees_per_frame() {
        let (_, runtime) = spawn(EF_THROW_HAPPOKUNAI);

        assert_eq!(runtime.particles[0].angle, 15.0);
        assert_eq!(runtime.particles[1].angle, 15.0, "trail copies the lead rotation");
        assert_eq!(runtime.particles[2].angle, 15.0);
        assert_eq!(runtime.particles[3].angle, 15.0);

        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.particles[0].angle, 15.0);
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.particles[0].angle, 60.0);
    }

    #[test]
    fn trail_alpha_drops_by_one_fifty_then_twenty_five() {
        let (_, runtime) = spawn(EF_THROW_HAPPOKUNAI);

        assert_eq!(runtime.particles[0].alpha, 255);
        assert_eq!(runtime.particles[1].alpha, 105);
        assert_eq!(runtime.particles[2].alpha, 80);
        assert_eq!(runtime.particles[3].alpha, 55);
    }

    #[test]
    fn the_lead_particle_flies_along_the_heading_and_height_delta() {
        let (_, runtime) = spawn(EF_THROW_HAPPOKUNAI);
        let angles = Angles::shared();

        // heading 0, heightDelta 4, distance 10, speed 1.5:
        // moved = (1.5*cos(0) + 3, 0.15*4 - 4, 1.5*sin(0) + 5).
        let lead = &runtime.particles[0];
        assert_eq!(direction_angle(10.0, 0.0), 0.0);
        assert!((lead.offset_x - (1.5 * angles.raw_cosine(0) + 3.0)).abs() < 1e-5);
        // Official Y-down: the initial offset is -4 - 8 = -12 and
        // `heightDelta` is -8 - (-4) = -4.
        assert!((lead.offset_y - (0.15 * -4.0 - 12.0)).abs() < 1e-5);
        assert!((lead.offset_z - (1.5 * angles.raw_sine(0) + 5.0)).abs() < 1e-5);
    }

    #[test]
    fn the_flight_arc_uses_the_official_sine_table() {
        let (_, runtime) = spawn(EF_THROW_MULTIPLE_COIN);

        // `travelFrames == 6`; at frame 0 progress is 0, height is 0.
        let particle = &runtime.particles[0];
        assert_eq!(particle.transform.values[10], particle.offset_y);
        // Same official-space start: -12, plus `heightDelta` -4 at speed 2.
        assert!((particle.offset_y - (0.2 * -4.0 - 12.0)).abs() < 1e-5);
        assert_eq!(particle.angle, 15.0);
    }

    #[test]
    fn the_lead_alpha_fades_after_the_reach_is_exceeded() {
        // One unit to the target: `travelFrames` is 1, so frame 2 has
        // progress 2.0 > 1.05 and fades by 20.
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();
        runtime.target_offset = [4.0, 4.0, 5.0];

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.particles[0].alpha, 255, "frame 0");
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.particles[0].alpha, 255, "progress == 1.0 still flies");
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.particles[0].alpha, 235, "frame 2: progress 2.0 > 1.05");
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.particles[0].alpha, 215);
    }

    #[test]
    fn the_flight_arc_lifts_the_particle_above_its_offset() {
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();

        // Frames 0..=3: progress is 3/7 at frame 3, so the sine index is 77.
        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }

        let angles = Angles::shared();
        let particle = &runtime.particles[0];
        let height = angles.raw_sine(77) * effect.arc_height;

        assert!(height > 0.0);
        assert_eq!(particle.transform.values[10], particle.offset_y - height);
    }

    #[test]
    fn the_transform_carries_the_scale_and_the_arc_corrected_offset() {
        let (_, runtime) = spawn(EF_THROW_HAPPOKUNAI);
        let particle = &runtime.particles[0];

        // The body composes `scale`, then `XRotation(cameraPitch)` followed
        // by `YRotation(rotation)`, then the translated offset. Rebuild it
        // independently and compare the 12 cells.
        let mut view = Matrix::identity();
        view.make_x_rotation(0.0 / 180.0 * 3.141592);
        view.append_y_rotation(15.0 / 180.0 * 3.141592);
        let mut scale = Matrix::identity();
        scale.set_scale(3.0, 3.0, 1.0);
        let mut rotation = Matrix::identity();
        rotation.compose(&scale, &view);
        let mut translate = Matrix::identity();
        translate.make_translate(particle.offset_x, particle.offset_y, particle.offset_z);
        let mut expected = Matrix::identity();
        expected.compose(&rotation, &translate);

        assert_eq!(particle.transform, expected);
        assert_eq!(particle.transform.values[9], particle.offset_x);
        assert_eq!(particle.transform.values[10], particle.offset_y);
        assert_eq!(particle.transform.values[11], particle.offset_z);
    }

    #[test]
    fn happokunai_spins_the_caster_but_the_coin_does_not() {
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();
        runtime.parent_angle = 100.0;

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_angle, 145.0);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_angle, 190.0);

        let mut coin = ThrowItemParticles::new(EF_THROW_MULTIPLE_COIN);
        let mut runtime = new_runtime();
        runtime.parent_angle = 100.0;
        coin.on_process(&mut runtime);
        assert_eq!(runtime.parent_angle, 100.0);
        assert_eq!(coin.id, EF_THROW_MULTIPLE_COIN);
    }

    #[test]
    fn the_throw_bodies_consume_no_rng() {
        let mut left_effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut left_runtime = new_runtime();
        left_effect.on_process(&mut left_runtime);
        run(&mut left_effect, &mut left_runtime, 5);

        let mut right_effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut right_runtime = EffectRuntime::new(200, [0.0, 0.0], 999);
        right_runtime.position = POSITION;
        right_runtime.target_offset = TARGET;
        right_effect.on_process(&mut right_runtime);
        run(&mut right_effect, &mut right_runtime, 5);

        assert_eq!(particle_snapshot(&left_runtime), particle_snapshot(&right_runtime));
    }

    #[test]
    fn direction_angle_matches_the_official_formula() {
        assert_eq!(direction_angle(10.0, 0.0), 0.0);
        assert!((direction_angle(0.0, 10.0) - 90.0).abs() < 1e-4);
        assert!((direction_angle(-10.0, 0.0) - 180.0).abs() < 1e-4);
        assert!((direction_angle(0.0, -10.0) - 270.0).abs() < 1e-4);
    }

    #[test]
    fn travel_frames_is_the_first_step_past_the_distance() {
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut particle = CastedParticle::new(POSITION, 5);
        particle.speed_parameter = 1.5;
        effect.init_trajectory(&mut particle, POSITION, TARGET);
        assert_eq!(effect.travel_frames, 7, "7 * 1.5 = 10.5 > 10");

        particle.speed_parameter = 2.0;
        effect.init_trajectory(&mut particle, POSITION, TARGET);
        assert_eq!(effect.travel_frames, 6, "6 * 2 = 12 > 10");
    }

    #[test]
    fn the_particle_position_follows_the_effect_every_frame() {
        let mut effect = ThrowItemParticles::new(EF_THROW_HAPPOKUNAI);
        let mut runtime = new_runtime();
        step(&mut effect, &mut runtime);
        runtime.position = [30.0, 40.0, 50.0];
        step(&mut effect, &mut runtime);

        for particle in &runtime.particles {
            // `CParticle.position` follows the official (Y-down) anchor.
            assert_eq!(particle.position, [30.0, -40.0, 50.0]);
            assert_eq!(particle.elapsed_frames, 2);
        }
    }
}
