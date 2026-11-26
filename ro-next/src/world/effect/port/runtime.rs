//! Per-frame driver for ported effects.
//!
//! Mirrors the official update order:
//!
//! 1. `EffectLists::Update` calls `CEffect::Update` on every live effect,
//! 2. `CEffect::Update` dispatches frame-scheduled events, calls the effect
//!    body (`OnProcess`), then `++stateCnt`,
//! 3. the primitive objects (pushed into `scene.objects` by
//!    `CRagEffect::LaunchEffectPrim`) are processed by the scene on the same
//!    frame, with `master.stateCnt` already incremented,
//! 4. a primitive retires once `++stateCnt > duration`.
//!
//! Effects retire when `stateCnt >= duration` (`Effect.cpp:214-216` in
//! `CRagEffect::OnProcess`'s tail).

use super::angles::Angles;
use super::primitive::{EffectPrimitiveState, MasterState, PrimitiveId};
pub use super::rng::EffectRng;
use super::str_player::{Clip, Ez2StrPlayer};

/// A live `CEffectPrim` owned by an effect.
#[derive(Debug, Clone)]
pub struct PrimitiveInstance {
    pub state: EffectPrimitiveState,
    pub alive: bool,
}

/// The state a ported effect body runs against.
pub struct EffectRuntime {
    /// `state.stateCnt` of the `CRagEffect` (the frame number the body sees).
    pub frame: i32,
    /// `state.duration` from the effect lifetime table.
    pub lifetime: i32,
    /// `state.tlvertX/tlvertY` - projected anchor on the 640x480 reference
    /// screen. The real client fills this from the camera every frame.
    pub screen: [f32; 2],
    /// `render.position` - the world position, used by 3-D primitives.
    pub position: [f32; 3],
    /// `state.param[0]` - effect parameter (skill level / power) some bodies
    /// use.
    pub param: f32,
    /// `state.param[1]`.
    pub param_secondary: f32,
    /// `parent->GetActRes()->Frame(0, 0)->Cell(0)->y`, used by aura kind 22 to
    /// lift the overlay to the head of the parent sprite.
    pub parent_action_height: Option<f32>,
    /// `state.deltaPos4` equivalents the effect bodies own: the offset to the
    /// target (`deltaPos`), the travelled vector (`deltaPos3`), the facing
    /// (`render.angle`) and the `flag` used by the projectile-style bodies.
    pub target_offset: [f32; 3],
    pub delta_pos3: [f32; 3],
    /// `state.deltaPos2` - the per-step vector some bodies carry across frames.
    pub delta_pos2: [f32; 3],
    pub render_angle: f32,
    /// `CRagEffect::state.longitude` / `.latitude` - the *effect-level*
    /// orientation, distinct from a primitive's own `state.longitude`.
    ///
    /// `CRagEffect::Init` sets `longitude = parent->render.angle`
    /// (`RagEffectInitialization.cpp:22`); `latitude` keeps the state
    /// constructor's zero. Bodies that aim primitives by the effect's facing
    /// (`FireArrow`, `IceArrow`) write both and their spark particles read them
    /// back on later frames.
    ///
    /// Additive: no pre-existing field changed position or meaning, and no
    /// ported body read an effect-level angle before these bodies did.
    pub longitude: f32,
    pub latitude: f32,
    /// `camera->camera.yaw` - used by action-facing effects.
    pub camera_yaw: f32,
    /// `camera->camera.pitch` - read by `FaceCamera` (primitive 155).
    pub camera_pitch: f32,
    /// Terrain height under the effect. The official code calls
    /// `scene->terrain->Height(x, z)` per primitive; the port's hermetic
    /// runtime carries a single sample (`None` = no terrain context).
    pub terrain_height: Option<f32>,
    /// `AM_CHANGE_COLOR` body tint (`None` = `AM_RETURN_COLOR`).
    pub body_tint: Option<[u8; 3]>,
    /// `state.parent->render.angle` - the caster's facing, used by bodies such
    /// as Bowling Bash.
    ///
    /// Storm Kick (`EF_STORMKICK`/`EF_STORMKICK1`) also *writes* it: the
    /// official bodies run `parent->render.angle += 30` / `+= 14` with a single
    /// `> 360.0` subtraction. The port mutates this per-effect copy exactly
    /// like the original; the client can mirror the post-step value back onto
    /// the caster actor (the original rotates the shared actor in place).
    pub parent_angle: f32,
    pub flag: i32,
    /// `state.count` - the hit count some bodies read (e.g. Blitz Beat).
    pub count: i32,
    pub primitives: Vec<PrimitiveInstance>,
    /// Sounds played on the current step (`PlayEffectWaveInline`).
    pub sounds: Vec<String>,
    /// `MM_QUAKE` requests made by the body so far.
    pub shakes: u32,
    pub rng: EffectRng,
    /// `.str` clips the body opened (`CRagEffect::Init`).
    pub str_clips: Vec<Clip>,
    /// `ProcessEZ2STR` state, created when the body opens a `.str` resource.
    pub str_player: Option<Ez2StrPlayer>,
}

impl EffectRuntime {
    pub fn new(lifetime: i32, screen: [f32; 2], seed: u64) -> Self {
        Self {
            frame: 0,
            lifetime,
            screen,
            position: [0.0; 3],
            param: 0.0,
            param_secondary: 0.0,
            parent_action_height: None,
            target_offset: [0.0; 3],
            delta_pos3: [0.0; 3],
            delta_pos2: [0.0; 3],
            render_angle: 0.0,
            longitude: 0.0,
            latitude: 0.0,
            camera_yaw: 0.0,
            camera_pitch: 0.0,
            terrain_height: None,
            body_tint: None,
            parent_angle: 0.0,
            flag: 0,
            count: 0,
            primitives: Vec::new(),
            sounds: Vec::new(),
            shakes: 0,
            rng: EffectRng::new(seed),
            str_clips: Vec::new(),
            str_player: None,
        }
    }

    /// `CRagEffect::LaunchEffectPrim` - creates a primitive and returns its
    /// index (the port's stand-in for the `CEffectPrim*`).
    ///
    /// `CEffectPrim::Initialize` (`EffectPrimitiveState.cpp:72-82`) starts the
    /// primitive at the master's position (`state.orgPos =
    /// master.render.position; render.position = state.orgPos`). Bodies whose
    /// motion has no `FollowMaster` (prim 154's `PrimFlashFade`, prim 155's
    /// relative slot vectors) depend on that spawn anchor.
    pub fn launch(&mut self, id: PrimitiveId, delta_pos2: [f32; 3]) -> usize {
        self.primitives.push(PrimitiveInstance {
            state: EffectPrimitiveState {
                count: id,
                delta_pos2,
                position: self.position,
                ..Default::default()
            },
            alive: true,
        });

        self.primitives.len() - 1
    }

    pub fn primitive_mut(&mut self, index: usize) -> &mut EffectPrimitiveState {
        &mut self.primitives[index].state
    }

    /// `MM_QUAKE` from an effect body.
    pub fn quake(&mut self) {
        self.shakes += 1;
    }

    /// `CRenderObject::Face(x, z)` - `ActorGeometry.cpp:47-64`.
    pub fn face(&mut self, x: f32, z: f32) {
        let delta_x = x - self.position[0];
        let delta_z = z - self.position[2];

        if delta_x != 0.0 || delta_z != 0.0 {
            let mut angle = ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0;
            self.render_angle = angle;

            if angle >= 360.0 {
                angle -= 360.0;
                self.render_angle = angle;
            }

            if angle < 0.0 {
                self.render_angle = angle + 360.0;
            }
        }
    }

    /// `CRenderObject::Heading(x, z, targetX, targetZ)` -
    /// `ActorGeometry.cpp:30-45`. The effect bodies call it through a
    /// `CEffectPrim` (which inherits `CRenderObject`), so this is an associated
    /// function: the original reads no object state.
    ///
    /// Quirk kept: only one `360` subtraction and one `360` addition.
    pub fn heading(x: f32, z: f32, target_x: f32, target_z: f32) -> f32 {
        let delta_x = target_x - x;
        let delta_z = target_z - z;
        let mut angle = ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0;

        if angle >= 360.0 {
            angle -= 360.0;
        }

        if angle < 0.0 {
            angle += 360.0;
        }

        angle
    }

    /// `CRagEffect::Init`'s effect-level facing
    /// (`RagEffectInitialization.cpp:22`): `state.longitude =
    /// parent->render.angle`. `state.latitude` keeps the constructor zero.
    ///
    /// The port cannot do this in [`Self::new`]: the client fills
    /// [`Self::parent_angle`] *after* construction, and the holder steps the
    /// body directly instead of going through [`step`]. The bodies that read
    /// the effect-level angle therefore call this on their first frame, which
    /// is exactly the frame the original sees the value on.
    pub fn init_effect_angles(&mut self) {
        self.longitude = self.parent_angle;
    }

    /// `CRenderObject::CalcDist(x, z)` - `ActorGeometry.cpp:66-70`.
    /// `CEffectPrim::Get_Angle` - `EffectPrimitiveState.cpp:51-70`. Returns the
    /// table index whose cosine the normalised vector still exceeds.
    pub fn get_angle(&self, x: f32, y: f32) -> i16 {
        let angles = super::angles::Angles::shared();
        let length = (x * x + y * y).sqrt();
        let normal_y = y / length;
        let normal_x = x / length;

        for index in 0..=180i16 {
            if normal_x >= angles.raw_cosine(index as usize) {
                return if normal_y >= 0.0 { index } else { 360 - index };
            }
        }

        180
    }

    pub fn calc_dist(&self, x: f32, z: f32) -> f32 {
        let delta_x = x - self.position[0];
        let delta_z = z - self.position[2];

        (delta_z * delta_z + delta_x * delta_x).sqrt()
    }

    /// `PlayEffectWaveInline(*this, name)`.
    pub fn play_sound(&mut self, name: &str) {
        self.sounds.push(name.to_owned());
    }

    /// `CRagEffect::Init` opening the `.str` resource of an effect.
    pub fn open_str(&mut self, clips: Vec<Clip>) {
        let player = Ez2StrPlayer::new(&clips);
        self.str_clips = clips;
        self.str_player = Some(player);
    }

    /// `CRagEffect::ProcessEZ2STR(0, -80, 0)`.
    ///
    /// Returns `false` when every loop of the resource has finished, which
    /// makes the body retire the effect (the original does `return false`).
    /// The player shares the effect's frame counter, and `InitEZ2STRFrame`
    /// resets it on a loop - so the runtime frame is synced back
    /// afterwards.
    pub fn process_str(&mut self) -> bool {
        let Some(player) = &mut self.str_player else {
            return true;
        };

        player.state_cnt = self.frame;

        let alive = player.process(&self.str_clips);

        self.frame = player.state_cnt;

        alive
    }

    /// Advances every live primitive by one frame.
    pub fn process_primitives(&mut self) {
        let master = MasterState {
            screen: self.screen,
            position: self.position,
            state_cnt: self.frame,
            duration: self.lifetime,
            master_delta_pos: self.target_offset,
            render_angle: self.render_angle,
            parent_angle: self.parent_angle,
            camera_yaw: self.camera_yaw,
            camera_pitch: self.camera_pitch,
            terrain_height: self.terrain_height,
            ..Default::default()
        };

        let angles = Angles::shared();

        // The primitives share the effect's RNG, which lives next to them in
        // this struct; take it out for the loop to keep the borrows separate.
        let mut rng = std::mem::replace(&mut self.rng, EffectRng::new(1));
        // Port-only outbox: motion primitives record `PlayWave` requests on
        // their own state, the runtime collects them in submission order.
        let mut primitive_sounds = Vec::new();

        for primitive in &mut self.primitives {
            if !primitive.alive {
                continue;
            }

            primitive.alive = primitive.state.on_process(angles, &master, &mut rng);
            primitive_sounds.append(&mut primitive.state.pending_sounds);
        }

        self.rng = rng;
        self.sounds.append(&mut primitive_sounds);
    }
}

/// A ported `CRagEffect` body or dedicated `CEffect` subclass.
///
/// Bodies are owned by the client's effect list
/// (`Box<dyn EffectBase + Send + Sync>`), so the simulation state has to be
/// thread-shareable even though it is only stepped on the render thread.
pub trait PortedEffect: Send + Sync {
    /// `effectLifetime` from `EffectTypes.cpp`.
    fn lifetime(&self) -> i32;

    /// One `OnProcess` step. `runtime.frame` is the `stateCnt` the original
    /// body reads; it is incremented by [`step`] after this returns.
    ///
    /// Returning `false` retires the effect, mirroring `return false` in the
    /// decompiled `OnProcess` bodies.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool;
}

impl<T: PortedEffect + ?Sized> PortedEffect for Box<T> {
    fn lifetime(&self) -> i32 {
        (**self).lifetime()
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        (**self).on_process(runtime)
    }
}

/// One `CEffect::Update` step, including the primitive pass.
///
/// Returns `false` when the effect has finished and must be retired.
pub fn step(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime) -> bool {
    if !effect.on_process(runtime) {
        return false;
    }

    runtime.frame += 1;

    if runtime.frame >= runtime.lifetime {
        return false;
    }

    runtime.process_primitives();

    true
}

/// Runs an effect to completion, collecting the state after every frame.
/// Used by the per-effect tests and by the future golden-frame tooling.
pub fn run_to_completion(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime) -> Vec<FrameSnapshot> {
    let mut snapshots = Vec::new();

    loop {
        let alive = step(effect, runtime);
        snapshots.push(FrameSnapshot::capture(runtime));

        if !alive {
            return snapshots;
        }
    }
}

/// A frame's worth of observable state.
#[derive(Debug, Clone)]
pub struct FrameSnapshot {
    pub frame: i32,
    pub primitives: Vec<EffectPrimitiveState>,
    pub sounds: Vec<String>,
}

impl FrameSnapshot {
    fn capture(runtime: &EffectRuntime) -> Self {
        Self {
            frame: runtime.frame,
            primitives: runtime.primitives.iter().map(|p| p.state.clone()).collect(),
            sounds: runtime.sounds.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Spawner {
        lifetime: i32,
    }

    impl PortedEffect for Spawner {
        fn lifetime(&self) -> i32 {
            self.lifetime
        }

        fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
            if runtime.frame != 0 {
                return true;
            }

            let index = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);
            state.duration = 10;
            state.radius = 5.0;
            state.pattern = 1;

            true
        }
    }

    #[test]
    fn effect_runs_exactly_lifetime_frames() {
        let mut effect = Spawner { lifetime: 10 };
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        // Frames 0..=9 are simulated, the 10th step retires the effect.
        assert_eq!(snapshots.len(), 10);
        assert_eq!(snapshots.last().unwrap().frame, 10);
    }

    #[test]
    fn primitive_launched_at_frame_zero_is_processed_on_frame_zero() {
        let mut effect = Spawner { lifetime: 10 };
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.state_cnt, 1);
    }

    #[test]
    fn rng_is_deterministic_and_matches_clamped_random() {
        let mut left = EffectRng::new(42);
        let mut right = EffectRng::new(42);

        for _ in 0..100 {
            assert_eq!(left.clamped_random(360), right.clamped_random(360));
        }

        let mut rng = EffectRng::new(7);
        for _ in 0..1000 {
            let value = rng.clamped_random(25);
            assert!((0..25).contains(&value));
        }

        assert_eq!(rng.clamped_random(0), 0);
    }

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-3
    }

    #[test]
    fn heading_matches_the_cardinal_angles() {
        // `Heading(x, z, targetX, targetZ)` = atan2(dx, -dz) in degrees. The
        // values are not exact quarters because the original divides by
        // `3.141592f` (`90.00002` for +x), so compare with a 1e-3 tolerance.
        assert!(close(EffectRuntime::heading(0.0, 0.0, 0.0, -1.0), 0.0));
        assert!(close(EffectRuntime::heading(0.0, 0.0, 1.0, 0.0), 90.0));
        assert!(close(EffectRuntime::heading(0.0, 0.0, 0.0, 1.0), 180.0));
        assert!(close(EffectRuntime::heading(0.0, 0.0, -1.0, 0.0), 270.0));
    }

    #[test]
    fn heading_is_relative_to_the_source_point() {
        // A zero target vector keeps atan2(0, -0) at 180 (the `-0.0` sign),
        // i.e. the original's behaviour for an unchanged position.
        assert!(close(EffectRuntime::heading(5.0, 7.0, 5.0, 7.0), 180.0));
        // Same direction whether the pair is translated or not.
        assert_eq!(
            EffectRuntime::heading(5.0, 7.0, 6.0, 7.0),
            EffectRuntime::heading(0.0, 0.0, 1.0, 0.0)
        );
    }

    #[test]
    fn launched_primitives_start_at_the_master_position() {
        let mut runtime = EffectRuntime::new(10, [0.0, 0.0], 1);
        runtime.position = [10.0, 20.0, 30.0];
        let index = runtime.launch(PrimitiveId::Circle2d, [1.0, 2.0, 3.0]);

        let state = &runtime.primitives[index].state;
        assert_eq!(state.position, [10.0, 20.0, 30.0]);
        assert_eq!(state.delta_pos2, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn init_effect_angles_copies_the_parent_facing_into_longitude() {
        let mut runtime = EffectRuntime::new(160, [0.0, 0.0], 1);
        runtime.parent_angle = 135.0;
        runtime.latitude = 12.0;

        runtime.init_effect_angles();

        assert_eq!(runtime.longitude, 135.0);
        // `Init` never assigns latitude; the constructor zero is kept.
        assert_eq!(runtime.latitude, 12.0);
    }

    #[test]
    fn effect_level_angles_start_at_zero() {
        let runtime = EffectRuntime::new(160, [0.0, 0.0], 1);

        assert_eq!(runtime.longitude, 0.0);
        assert_eq!(runtime.latitude, 0.0);
    }
}
