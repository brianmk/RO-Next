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
use super::str_player::{Clip, Ez2StrPlayer};

pub use super::rng::EffectRng;

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
    pub primitives: Vec<PrimitiveInstance>,
    /// Sounds played on the current step (`PlayEffectWaveInline`).
    pub sounds: Vec<String>,
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
            primitives: Vec::new(),
            sounds: Vec::new(),
            rng: EffectRng::new(seed),
            str_clips: Vec::new(),
            str_player: None,
        }
    }

    /// `CRagEffect::LaunchEffectPrim` - creates a primitive and returns its
    /// index (the port's stand-in for the `CEffectPrim*`).
    pub fn launch(&mut self, id: PrimitiveId, delta_pos2: [f32; 3]) -> usize {
        self.primitives.push(PrimitiveInstance {
            state: EffectPrimitiveState {
                count: id,
                delta_pos2,
                ..Default::default()
            },
            alive: true,
        });

        self.primitives.len() - 1
    }

    pub fn primitive_mut(&mut self, index: usize) -> &mut EffectPrimitiveState {
        &mut self.primitives[index].state
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
    /// Returns `false` when every loop of the resource has finished, which makes
    /// the body retire the effect (the original does `return false`). The player
    /// shares the effect's frame counter, and `InitEZ2STRFrame` resets it on a
    /// loop - so the runtime frame is synced back afterwards.
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
        };

        let angles = Angles::shared();

        // The primitives share the effect's RNG, which lives next to them in
        // this struct; take it out for the loop to keep the borrows separate.
        let mut rng = std::mem::replace(&mut self.rng, EffectRng::new(1));

        for primitive in &mut self.primitives {
            if !primitive.alive {
                continue;
            }

            primitive.alive = primitive.state.on_process(angles, &master, &mut rng);
        }

        self.rng = rng;
    }
}

/// A ported `CRagEffect` body or dedicated `CEffect` subclass.
pub trait PortedEffect {
    /// `effectLifetime` from `EffectTypes.cpp`.
    fn lifetime(&self) -> i32;

    /// One `OnProcess` step. `runtime.frame` is the `stateCnt` the original
    /// body reads; it is incremented by [`step`] after this returns.
    ///
    /// Returning `false` retires the effect, mirroring `return false` in the
    /// decompiled `OnProcess` bodies.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool;
}

/// One `CEffect::Update` step, including the primitive pass.
///
/// Returns `false` when the effect has finished and must be retired.
pub fn step(effect: &mut impl PortedEffect, runtime: &mut EffectRuntime) -> bool {
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
pub fn run_to_completion(effect: &mut impl PortedEffect, runtime: &mut EffectRuntime) -> Vec<FrameSnapshot> {
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
}
