//! Effects ported by the "h" parallel batch - the manual tier's caster-body
//! effects.
//!
//! Every body here is a direct transliteration of one decompiled
//! `CRagEffect::OnProcess` dispatch block that does **not** launch a primitive:
//! the official code only schedules waves, `MM_QUAKE`, and per-frame writes to
//! the caster actor (`render.flags`, `render.colorChannels`). The port routes
//! those writes through the existing runtime channels
//! ([`EffectRuntime::add_parent_flags`], [`EffectRuntime::set_parent_color`],
//! [`EffectRuntime::quake`], [`EffectRuntime::play_sound`]); frame gates,
//! literals, the `%` parity tests and the RNG draw order are kept.
//!
//! Two official side effects have no channel in the hermetic runtime and are
//! documented at their call site: `render.position =
//! state.parent->render.position` (the runtime's [`EffectRuntime::position`] is
//! already the master anchor filled by the spawn path) and
//! `state.parent->render.flags += N` *conditioned on the bit being clear*
//! (`begin_step` clears the word every frame, so the runtime's OR is
//! equivalent; this is the same model batch "g" uses).
//!
//! `PlayEffectWave` and `PlayEffectWaveInline` are byte-identical helpers in
//! the decompilation (`RagEffect.cpp:152-165`): both submit the same positional
//! `PlayWave(name, effect.x - player.x, 0.0, effect.z - player.z, 250, 40,
//! 1.0, 0)`. The port's [`EffectRuntime::play_sound`] is the channel for that
//! call shape, so [`Memorize`] uses it for `PlayEffectWave` exactly like the
//! other bodies use it for `PlayEffectWaveInline`.

#![allow(dead_code)]

use super::runtime::{EffectRuntime, PortedEffect};

// ---------------------------------------------------------------------------
// EF_MAGICCRASHER
// ---------------------------------------------------------------------------

/// `case EF_MAGICCRASHER:` - `RagEffect.cpp:23758-23772`.
///
/// Frame 25 plays `effect\매직 크래쉬.wav`; frame 30 sends one `MM_QUAKE`.
/// Frames 30-60 request caster flag 1 and call
/// `SetColor(-1, max(rand() % 255, 0), max(rand() % 255, 0),
/// max(rand() % 255, 0))`, i.e. three `ClampedRandom(255)` draws per frame in
/// the written red/green/blue order. Frame 30 is both the quake frame and the
/// first tinted frame.
#[derive(Debug, Default)]
pub struct MagicCrasher;

impl MagicCrasher {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for MagicCrasher {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:389` - `result[EF_MAGICCRASHER] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 25 {
            // `this->render.position = this->state.parent->render.position;`
            // is a no-op in the hermetic runtime (the master position is
            // already the parent anchor).
            runtime.play_sound("effect\\매직 크래쉬.wav");
        }

        if frame == 30 {
            runtime.quake();
        }

        if (30..=60).contains(&frame) {
            runtime.add_parent_flags(1);

            // `std::max(rand() % 255, 0)` is `ClampedRandom(255)`
            // (`RagEffect.cpp:168-171`): `rand() % range` is never negative, so
            // the clamp is the identity and the helper is the exact stand-in.
            let red = runtime.rng.clamped_random(255) as i16;
            let green = runtime.rng.clamped_random(255) as i16;
            let blue = runtime.rng.clamped_random(255) as i16;

            runtime.set_parent_color(-1, red, green, blue);
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_MEMORIZE
// ---------------------------------------------------------------------------

/// `case EF_MEMORIZE:` - `RagEffect.cpp:21521-21535`.
///
/// Frame 0 plays `effect\priest_suffragium.wav` (`PlayEffectWave`, the
/// identical twin of the inline helper). Frames 10-35 alternate on frame
/// parity: even frames request flag 1 and `SetColor(-1, 250, 250, 100)`,
/// odd frames only `SetColor(-1, 255, 255, 255)` - so the flag is requested on
/// 10, 12, ... 34 and the white frame 11, 13, ... 35 leaves it clear.
#[derive(Debug, Default)]
pub struct Memorize;

impl Memorize {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for Memorize {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:514` - `result[EF_MEMORIZE] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            runtime.play_sound("effect\\priest_suffragium.wav");
        }

        if (10..=35).contains(&frame) {
            if frame % 2 == 0 {
                runtime.add_parent_flags(1);
                runtime.set_parent_color(-1, 250, 250, 100);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_LINKLIGHT
// ---------------------------------------------------------------------------

/// `case EF_LINKLIGHT:` - `RagEffect.cpp:24435-24442`.
///
/// The only window is frames 40-70 (inclusive): request caster flag 1 and
/// `SetColor(-1, 200, 150, 50)` every frame. Nothing happens before or after.
#[derive(Debug, Default)]
pub struct LinkLight;

impl LinkLight {
    pub fn new() -> Self {
        Self
    }
}

impl PortedEffect for LinkLight {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:394` - `result[EF_LINKLIGHT] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if (40..=70).contains(&runtime.frame) {
            runtime.add_parent_flags(1);
            runtime.set_parent_color(-1, 200, 150, 50);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::runtime::{EffectRng, EffectRuntime, run_to_completion, step};
    use super::*;

    fn runtime_with(seed: u64, lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], seed)
    }

    #[test]
    fn batch_h_lifetimes_match_effect_types() {
        // `EffectTypes.cpp:389`, `:514` and `:394`.
        assert_eq!(MagicCrasher::new().lifetime(), 100);
        assert_eq!(Memorize::new().lifetime(), 200);
        assert_eq!(LinkLight::new().lifetime(), 200);
    }

    // -- EF_MAGICCRASHER ---------------------------------------------------

    #[test]
    fn magiccrasher_plays_the_wave_on_25_and_quakes_on_30() {
        let mut effect = MagicCrasher::new();
        let mut runtime = runtime_with(541, 100);

        // Frames 0..=24: no wave, no quake.
        for _ in 0..25 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 25);
        assert!(runtime.sounds.is_empty());
        assert_eq!(runtime.shakes, 0);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // frame 25
        assert_eq!(runtime.sounds, vec!["effect\\매직 크래쉬.wav".to_owned()]);
        assert_eq!(runtime.shakes, 0);

        // Frames 26..=29: the wave has played, the quake is still pending.
        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 30);
        assert_eq!(runtime.shakes, 0);

        step(&mut effect, &mut runtime); // frame 30
        assert_eq!(runtime.shakes, 1);
        assert_eq!(runtime.parent_flags, 1);

        // First tinted frame: three `ClampedRandom(255)` draws, in written
        // order. No earlier frame draws, so the seed's first three values are
        // the red, green and blue channels.
        let mut rng = EffectRng::new(541);
        let expected = [
            rng.clamped_random(255) as i16,
            rng.clamped_random(255) as i16,
            rng.clamped_random(255) as i16,
        ];
        assert_eq!(runtime.parent_color, Some([-1, expected[0], expected[1], expected[2]]));
    }

    #[test]
    fn magiccrasher_tints_exactly_frames_30_to_60_and_consumes_93_draws() {
        let mut effect = MagicCrasher::new();
        let mut runtime = runtime_with(7, 100);

        // Frame 29 is the last untinted frame.
        while runtime.frame < 29 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 29);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // frame 29 -> frame 30 sees the window
        while runtime.frame < 60 {
            step(&mut effect, &mut runtime);
        }
        // The step above processed frame 60 only after the loop's last check:
        // run it explicitly and assert the window's last frame.
        step(&mut effect, &mut runtime); // frame 60
        let last_window_color = runtime.parent_color;
        assert_eq!(runtime.parent_flags, 1);
        assert!(last_window_color.is_some());

        step(&mut effect, &mut runtime); // frame 61
        assert_eq!(runtime.parent_flags, 0);
        // `SetColor` persists on the actor; the body simply stops calling it.
        assert_eq!(runtime.parent_color, last_window_color);

        // The quake happened exactly once, on frame 30.
        assert_eq!(runtime.shakes, 1);

        // Finish the 100-frame lifetime and prove the RNG advanced exactly
        // 31 frames * 3 draws = 93 times.
        while runtime.frame < 100 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 100);
        assert_eq!(runtime.shakes, 1);

        let mut reference = EffectRng::new(7);
        for _ in 0..(31 * 3) {
            reference.clamped_random(255);
        }
        assert_eq!(runtime.rng.clamped_random(1000), reference.clamped_random(1000));
    }

    #[test]
    fn magiccrasher_retires_after_100_frames() {
        let mut effect = MagicCrasher::new();
        let mut runtime = runtime_with(1, 100);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 100);
        assert_eq!(snapshots.last().unwrap().frame, 100);
    }

    // -- EF_MEMORIZE -------------------------------------------------------

    #[test]
    fn memorize_plays_the_wave_on_zero_and_alternates_from_10_to_35() {
        let mut effect = Memorize::new();
        let mut runtime = runtime_with(403, 200);

        step(&mut effect, &mut runtime); // frame 0
        assert_eq!(runtime.sounds, vec!["effect\\priest_suffragium.wav".to_owned()]);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        // Frames 1..=9: nothing.
        for _ in 0..9 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 10);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // frame 10, even
        assert_eq!(runtime.parent_flags, 1);
        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 100]));

        step(&mut effect, &mut runtime); // frame 11, odd
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        step(&mut effect, &mut runtime); // frame 12, even again
        assert_eq!(runtime.parent_flags, 1);
        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 100]));

        // Frames 13..=35, ending with the odd white frame 35.
        while runtime.frame < 36 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 36);
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));

        step(&mut effect, &mut runtime); // frame 36 is already outside
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
    }

    #[test]
    fn memorize_retires_after_200_frames_without_consuming_rng() {
        let mut effect = Memorize::new();
        let mut runtime = runtime_with(9, 200);
        let reference = runtime.rng.clone();

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 200);

        // The body draws no random value; the stream is untouched.
        let mut untouched = reference;
        assert_eq!(runtime.rng.clamped_random(1000), untouched.clamped_random(1000));
    }

    // -- EF_LINKLIGHT ------------------------------------------------------

    #[test]
    fn linklight_tints_only_frames_40_to_70() {
        let mut effect = LinkLight::new();
        let mut runtime = runtime_with(5, 200);

        // Frames 0..=39: nothing.
        for _ in 0..40 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 40);
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        step(&mut effect, &mut runtime); // frame 40, first window frame
        assert_eq!(runtime.parent_flags, 1);
        assert_eq!(runtime.parent_color, Some([-1, 200, 150, 50]));

        step(&mut effect, &mut runtime); // frame 41
        assert_eq!(runtime.parent_flags, 1);
        assert_eq!(runtime.parent_color, Some([-1, 200, 150, 50]));

        // Process through frame 70 (runtime.frame becomes 71).
        while runtime.frame < 71 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.frame, 71);
        assert_eq!(runtime.parent_flags, 1, "frame 70 was the last window frame");

        step(&mut effect, &mut runtime); // frame 71, first frame outside
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.parent_color, Some([-1, 200, 150, 50]));
    }

    #[test]
    fn linklight_retires_after_200_frames() {
        let mut effect = LinkLight::new();
        let mut runtime = runtime_with(1, 200);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 200);
        assert_eq!(runtime.shakes, 0);
        assert!(runtime.sounds.is_empty());
    }
}
