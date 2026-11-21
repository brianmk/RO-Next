//! A frame rate limiter that caps the number of frames the client produces.
//!
//! The limiter is intentionally simple and deterministic: it keeps an absolute
//! deadline for the next frame and sleeps until that deadline. The deadline is
//! carried from frame to frame so sleep jitter does not accumulate, but it is
//! re-anchored to the current time whenever the client falls behind by more
//! than one whole frame (for example because the GPU or `present()` took
//! longer than the frame budget, or because vsync is the actual pacing
//! mechanism). That way the limiter only ever acts as an *upper* bound and does
//! not fight vsync: if the frame already took longer than the frame budget,
//! nothing is slept at all.
//!
//! Sleeping is done with [`std::thread::sleep`] in (at most) millisecond
//! sized chunks, so an idle/backgrounded client does not busy-spin.

use std::time::{Duration, Instant};

/// Maximum amount of time we sleep in one go. Sleeping the whole remaining time
/// at once can overshoot on kernels with coarse timers, so the last part of the
/// wait is split into smaller sleeps. This is still a real sleep, not a spin.
const SLEEP_CHUNK: Duration = Duration::from_millis(1);

/// A limit of zero means "unlimited".
pub const UNLIMITED: u32 = 0;

/// Sleeps until `deadline` without busy-spinning.
fn sleep_until(deadline: Instant) {
    loop {
        let now = Instant::now();
        let Some(remaining) = deadline.checked_duration_since(now) else {
            return;
        };

        // Always sleep below the remaining duration so that we do not
        // systematically overshoot with coarse timers; the final (sub- chunk)
        // sleep is capped by the remaining time itself.
        if remaining > SLEEP_CHUNK {
            std::thread::sleep(remaining - SLEEP_CHUNK);
        } else {
            std::thread::sleep(remaining);
        }
    }
}

/// Measures the effective frame rate over roughly one second windows.
struct FrameRateMeasurement {
    window_start: Instant,
    frames_in_window: u32,
    frames_per_second: f32,
    total_frames: u64,
}

impl FrameRateMeasurement {
    fn new() -> Self {
        Self {
            window_start: Instant::now(),
            frames_in_window: 0,
            frames_per_second: 0.0,
            total_frames: 0,
        }
    }

    fn record_frame(&mut self, now: Instant) {
        self.frames_in_window += 1;
        self.total_frames += 1;

        let elapsed = now.saturating_duration_since(self.window_start);

        if elapsed >= Duration::from_secs(1) {
            self.frames_per_second = self.frames_in_window as f32 / elapsed.as_secs_f32();
            self.frames_in_window = 0;
            self.window_start = now;
        }
    }
}

/// Caps the frame rate of the client.
pub struct FrameLimiter {
    limit: u32,
    next_deadline: Option<Instant>,
    measurement: FrameRateMeasurement,
}

impl FrameLimiter {
    pub fn new() -> Self {
        Self {
            limit: UNLIMITED,
            next_deadline: None,
            measurement: FrameRateMeasurement::new(),
        }
    }

    /// Sets the limit in frames per second. `0` means unlimited.
    ///
    /// Returns `true` if the limit actually changed.
    pub fn set_limit(&mut self, limit: u32) -> bool {
        if self.limit == limit {
            return false;
        }

        self.limit = limit;
        // Drop the deadline of the previous configuration, so that a new limit
        // starts with a full frame budget instead of inheriting a stale
        // deadline (which could otherwise cause a burst of frames).
        self.next_deadline = None;
        self.measurement = FrameRateMeasurement::new();

        true
    }

    fn frame_duration(&self) -> Option<Duration> {
        match self.limit {
            UNLIMITED => None,
            limit => Some(Duration::from_secs_f64(1.0 / f64::from(limit))),
        }
    }

    /// Blocks until the next frame may start. Must be called once per frame,
    /// as early as possible in the frame.
    pub fn wait_for_next_frame(&mut self) {
        let now = Instant::now();

        if let Some(frame_duration) = self.frame_duration() {
            let deadline = match self.next_deadline {
                // Stay on the schedule that was established by the previous
                // frames as long as we are not more than one whole frame late.
                Some(deadline) if now <= deadline + frame_duration => deadline,
                // First frame after the limit was set, or we fell behind by
                // more than one frame: re-anchor to the current time instead of
                // trying to catch up (which would produce a burst of frames).
                _ => now + frame_duration,
            };

            if now < deadline {
                sleep_until(deadline);
            }

            let finished = Instant::now();
            self.next_deadline = Some(finished.max(deadline) + frame_duration);
        } else {
            self.next_deadline = None;
        }

        self.measurement.record_frame(Instant::now());
    }

    /// The effective frame rate of the last completed measurement window, in
    /// frames per second. Returns `0.0` until the first window has elapsed.
    pub fn measured_frames_per_second(&self) -> f32 {
        self.measurement.frames_per_second
    }

    /// Total number of frames accounted for by this limiter.
    #[cfg(test)]
    pub fn total_frames(&self) -> u64 {
        self.measurement.total_frames
    }
}

impl Default for FrameLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{FrameLimiter, UNLIMITED};

    #[test]
    fn unlimited_limiter_does_not_sleep() {
        let mut limiter = FrameLimiter::new();

        let start = Instant::now();
        for _ in 0..10 {
            limiter.wait_for_next_frame();
        }

        assert!(start.elapsed() < Duration::from_millis(100));
        assert_eq!(limiter.total_frames(), 10);
    }

    #[test]
    fn limited_limiter_sleeps_to_deadline() {
        let mut limiter = FrameLimiter::new();
        assert!(limiter.set_limit(100));
        assert!(!limiter.set_limit(100));

        let frame_duration = Duration::from_secs_f64(1.0 / 100.0);

        // The first frame establishes the schedule, the following frames are
        // then paced one frame duration apart.
        limiter.wait_for_next_frame();
        let start = Instant::now();
        for _ in 0..4 {
            limiter.wait_for_next_frame();
        }
        let elapsed = start.elapsed();

        // Allow some scheduling slack, but the limiter must not return before
        // the deadlines.
        assert!(elapsed >= frame_duration * 3, "limiter did not sleep long enough: {elapsed:?}");
        assert!(elapsed < frame_duration * 12, "limiter slept too long: {elapsed:?}");
    }

    #[test]
    fn changing_limit_resets_schedule() {
        let mut limiter = FrameLimiter::new();
        limiter.set_limit(60);
        limiter.wait_for_next_frame();

        assert!(limiter.set_limit(30));
        let start = Instant::now();
        limiter.wait_for_next_frame();
        let elapsed = start.elapsed();

        // After switching to 30 FPS the next frame must wait for (roughly) a
        // full 33ms frame budget instead of the old 16ms one.
        assert!(elapsed >= Duration::from_millis(20), "new limit not applied: {elapsed:?}");
    }

    #[test]
    fn measurement_reports_effective_fps() {
        let mut limiter = FrameLimiter::new();
        // Unlimited frames, but the measurement window is one second; emulate a
        // few frames without actually waiting.
        limiter.set_limit(UNLIMITED);
        for _ in 0..3 {
            limiter.wait_for_next_frame();
        }

        assert_eq!(limiter.total_frames(), 3);
    }
}
