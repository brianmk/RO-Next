//! Deterministic stand-in for the C runtime's `rand()`.
//!
//! The original uses `rand() % range` (MSVC LCG). Matching the exact sequence is
//! not required for visual fidelity - every client differs anyway - but the
//! distribution must be the same and tests need determinism. Seed it per effect.
//!
//! Deviation documented in `PORT_STATUS.md`.

#[derive(Debug, Clone)]
pub struct EffectRng {
    state: u64,
}

impl EffectRng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    fn next_u32(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        (x.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 32) as u32
    }

    /// Consumes one value like a bare `rand()` call in the original.
    pub fn discard(&mut self) {
        let _ = self.next_u32();
    }

    /// `ClampedRandom(int range)` - `RagEffect.cpp:168-171`.
    pub fn clamped_random(&mut self, range: i32) -> i32 {
        if range <= 0 {
            return 0;
        }

        (self.next_u32() % range as u32) as i32
    }
}
