//! Exact port of `Source/Client/Application/Timer.cpp`.
//!
//! The official client runs effects on a **fixed timestep** of `1000 / 60 = 16`
//! milliseconds (integer division, so 62.5 steps per second), with a catch-up
//! loop and a frame-skip toggle:
//!
//! ```cpp
//! int GetSkipFrameCount() {
//!   int skipCount = (timeGetTime() - frameTime) / frameFreq;
//!   frameTime += frameFreq * skipCount;
//!   if (g_frameskip) return skipCount;
//!   return 1;
//! }
//! ```
//!
//! `CGameMode::Update` then runs `skipCount` effect/scene steps and assigns the
//! count to `g_updateTick` for the duration of the loop.
//!
//! Consequence for the port: a 100-frame effect lasts 1.60 s, not 1.667 s, and
//! effect state is never interpolated between steps.

use super::FRAME_FREQUENCY_MS;

/// One rendered frame's worth of effect steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameBatch {
    /// Number of times the effect manager has to be updated.
    pub steps: u32,
    /// Value of `g_updateTick` while those steps run. `CGameMode` assigns the
    /// total count before the loop; effects compare it against zero to skip
    /// expensive per-frame work.
    pub update_tick: i32,
}

/// Port of the `frameTime` / `frameFreq` / `g_frameskip` globals.
#[derive(Debug, Clone)]
pub struct EffectClock {
    frame_time_ms: u64,
    frame_frequency_ms: u64,
    frameskip: bool,
}

impl EffectClock {
    /// `InitTimer(60)`.
    pub fn new(now_ms: u64) -> Self {
        Self {
            frame_time_ms: now_ms,
            frame_frequency_ms: FRAME_FREQUENCY_MS,
            frameskip: true,
        }
    }

    /// `ResetTimer()`.
    pub fn reset(&mut self, now_ms: u64) {
        self.frame_time_ms = now_ms;
    }

    /// `GetSkipFrameCount()` - `Timer.cpp:17-26`.
    pub fn begin_render(&mut self, now_ms: u64) -> FrameBatch {
        let skip_count = now_ms.saturating_sub(self.frame_time_ms) / self.frame_frequency_ms;
        self.frame_time_ms += self.frame_frequency_ms * skip_count;

        let steps = if self.frameskip { skip_count as u32 } else { 1 };

        FrameBatch {
            steps,
            update_tick: steps as i32,
        }
    }

    /// Equivalent of the chat command that flips `g_frameskip`.
    pub fn set_frameskip(&mut self, enabled: bool) {
        self.frameskip = enabled;
    }

    /// Current `frameTime`.
    pub fn frame_time_ms(&self) -> u64 {
        self.frame_time_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_is_sixteen_milliseconds() {
        assert_eq!(FRAME_FREQUENCY_MS, 16);
    }

    #[test]
    fn hundred_frames_last_1600_ms() {
        let mut clock = EffectClock::new(0);
        let mut steps = 0u32;
        let mut elapsed = 0u64;

        for _ in 0..100 {
            elapsed += 16;
            steps += clock.begin_render(elapsed).steps;
        }

        assert_eq!(steps, 100);
        assert_eq!(elapsed, 1_600);
    }

    #[test]
    fn catch_up_returns_multiple_steps() {
        let mut clock = EffectClock::new(0);

        let batch = clock.begin_render(50);
        assert_eq!(batch.steps, 3, "50 / 16 == 3");
        assert_eq!(batch.update_tick, 3);
        assert_eq!(clock.frame_time_ms(), 48);

        assert_eq!(clock.begin_render(51).steps, 0);
    }

    #[test]
    fn frameskip_off_always_steps_once() {
        let mut clock = EffectClock::new(0);
        clock.set_frameskip(false);
        assert_eq!(clock.begin_render(0).steps, 1);
        assert_eq!(clock.frame_time_ms(), 0, "frameTime is only advanced by skipCount");
    }
}
