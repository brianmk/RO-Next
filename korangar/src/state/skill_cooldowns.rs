use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, SkillId};
use rust_state::RustState;

/// One active skill cooldown (`ZC_SKILL_POSTDELAY`, 0x043D).
#[derive(Debug, Clone, Copy)]
pub struct SkillCooldown {
    pub skill_id: SkillId,
    /// Tick at which the cooldown ends.
    pub until: ClientTick,
    /// Tick at which the cooldown was started (the last update of the packet
    /// or of the scan), used for the sweep fraction.
    pub started_at: ClientTick,
}

impl SkillCooldown {
    /// Total length of the cooldown in milliseconds.
    fn duration_ms(&self) -> u32 {
        self.until.0.wrapping_sub(self.started_at.0)
    }

    /// Remaining cooldown at `now`.
    fn remaining(&self, now: ClientTick) -> Option<CooldownRemaining> {
        let remaining_ms = self.until.0.saturating_sub(now.0);

        if remaining_ms == 0 {
            return None;
        }

        let duration_ms = self.duration_ms().max(1);
        let fraction = (remaining_ms as f32 / duration_ms as f32).clamp(0.0, 1.0);

        Some(CooldownRemaining {
            remaining_ms,
            total_ms: duration_ms,
            fraction,
            seconds: remaining_ms.div_ceil(1000),
        })
    }
}

/// Display state of one active cooldown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CooldownRemaining {
    /// Remaining time in milliseconds.
    pub remaining_ms: u32,
    /// Total cooldown length in milliseconds.
    pub total_ms: u32,
    /// Remaining fraction (`1.0` right after the start, `0.0` at the end), used
    /// for the sweep overlay.
    pub fraction: f32,
    /// Remaining whole seconds, rounded up, used for the number overlay.
    pub seconds: u32,
}

impl CooldownRemaining {
    #[cfg(test)]
    fn new(remaining_ms: u32, total_ms: u32) -> Self {
        let total_ms = total_ms.max(1);
        let remaining_ms = remaining_ms.min(total_ms);
        let fraction = (remaining_ms as f32 / total_ms as f32).clamp(0.0, 1.0);

        Self {
            remaining_ms,
            total_ms,
            fraction,
            seconds: remaining_ms.div_ceil(1000),
        }
    }
}

/// Active skill cooldowns of the local player.
#[derive(RustState, StateElement)]
pub struct SkillCooldowns {
    #[hidden_element]
    entries: Vec<SkillCooldown>,
    /// Client tick of the last update; the interface uses it to compute the
    /// remaining time and the sweep fraction.
    #[hidden_element]
    now: ClientTick,
}

impl Default for SkillCooldowns {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            now: ClientTick(0),
        }
    }
}

impl SkillCooldowns {
    /// Starts (or refreshes) a cooldown. `now` is the tick at which the packet
    /// was received and defines the total length together with `until`.
    pub fn start(&mut self, skill_id: SkillId, until: ClientTick, now: ClientTick) {
        self.now = now;

        if until.0 <= now.0 {
            self.remove(skill_id);
            return;
        }

        match self.entries.iter_mut().find(|entry| entry.skill_id == skill_id) {
            Some(entry) => {
                entry.until = until;
                entry.started_at = now;
            }
            None => self.entries.push(SkillCooldown {
                skill_id,
                until,
                started_at: now,
            }),
        }
    }

    pub fn remove(&mut self, skill_id: SkillId) {
        self.entries.retain(|entry| entry.skill_id != skill_id);
    }

    /// Remove cooldowns whose end tick has been reached and advances the tick
    /// used by the display.
    pub fn prune(&mut self, client_tick: ClientTick) -> bool {
        self.now = client_tick;

        let previous = self.entries.len();
        self.entries.retain(|entry| entry.until.0 > client_tick.0);
        self.entries.len() != previous
    }

    /// Tick of the last update (used by tests and the interface).
    #[allow(dead_code)]
    pub fn now(&self) -> ClientTick {
        self.now
    }

    #[allow(dead_code)]
    pub fn until(&self, skill_id: SkillId) -> Option<ClientTick> {
        self.entries
            .iter()
            .find(|entry| entry.skill_id == skill_id)
            .map(|entry| entry.until)
    }

    /// Remaining display state of a skill cooldown, if it is still running.
    pub fn remaining(&self, skill_id: SkillId) -> Option<CooldownRemaining> {
        self.entries
            .iter()
            .find(|entry| entry.skill_id == skill_id)
            .and_then(|entry| entry.remaining(self.now))
    }

    #[allow(dead_code)]
    pub fn entries(&self) -> &[SkillCooldown] {
        &self.entries
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, SkillId};

    use super::{CooldownRemaining, SkillCooldowns};

    #[test]
    fn start_prunes_and_expires() {
        let mut cooldowns = SkillCooldowns::default();
        cooldowns.start(SkillId(1), ClientTick(100), ClientTick(0));
        cooldowns.start(SkillId(2), ClientTick(50), ClientTick(0));

        assert_eq!(cooldowns.until(SkillId(1)).map(|tick| tick.0), Some(100));
        assert!(cooldowns.prune(ClientTick(60)));
        assert!(cooldowns.until(SkillId(2)).is_none());
        assert_eq!(cooldowns.entries().len(), 1);

        cooldowns.start(SkillId(1), ClientTick(200), ClientTick(100));
        assert_eq!(cooldowns.until(SkillId(1)).map(|tick| tick.0), Some(200));
        assert_eq!(cooldowns.entries().len(), 1);
    }

    #[test]
    fn remaining_time_and_fraction_shrink_with_the_tick() {
        let mut cooldowns = SkillCooldowns::default();
        cooldowns.start(SkillId(7), ClientTick(1_000), ClientTick(0));

        cooldowns.prune(ClientTick(250));
        let remaining = cooldowns.remaining(SkillId(7)).expect("cooldown is still running");
        assert_eq!(remaining.remaining_ms, 750);
        assert_eq!(remaining.total_ms, 1_000);
        assert_eq!(remaining.seconds, 1);
        assert!((remaining.fraction - 0.75).abs() < f32::EPSILON);

        cooldowns.prune(ClientTick(900));
        let remaining = cooldowns.remaining(SkillId(7)).expect("cooldown is still running");
        assert_eq!(remaining.remaining_ms, 100);
        assert_eq!(remaining.seconds, 1, "the last fraction of a second shows as one second");

        cooldowns.prune(ClientTick(1_000));
        assert!(cooldowns.remaining(SkillId(7)).is_none());
    }

    #[test]
    fn a_new_packet_restarts_the_sweep() {
        let mut cooldowns = SkillCooldowns::default();
        cooldowns.start(SkillId(3), ClientTick(1_000), ClientTick(0));
        cooldowns.prune(ClientTick(500));
        assert!((cooldowns.remaining(SkillId(3)).unwrap().fraction - 0.5).abs() < f32::EPSILON);

        cooldowns.start(SkillId(3), ClientTick(1_500), ClientTick(500));
        cooldowns.prune(ClientTick(500));
        assert!((cooldowns.remaining(SkillId(3)).unwrap().fraction - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn an_already_expired_cooldown_is_not_stored() {
        let mut cooldowns = SkillCooldowns::default();
        cooldowns.start(SkillId(4), ClientTick(100), ClientTick(100));
        assert!(cooldowns.entries().is_empty());

        // The helper also clamps a remaining time that is longer than the total.
        let remaining = CooldownRemaining::new(2_000, 1_000);
        assert_eq!(remaining.fraction, 1.0);
        assert_eq!(remaining.seconds, 1);
    }
}
