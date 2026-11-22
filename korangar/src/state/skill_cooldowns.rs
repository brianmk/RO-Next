use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, SkillId};
use rust_state::RustState;

/// One active skill cooldown (`ZC_SKILL_POSTDELAY`, 0x043D).
#[derive(Debug, Clone, Copy)]
pub struct SkillCooldown {
    pub skill_id: SkillId,
    pub until: ClientTick,
}

/// Active skill cooldowns of the local player.
///
/// Cooldown UI is owned by the skill-souls/cast-bar lane; this state is the
/// typed packet-layer hand-off.
#[derive(Default, RustState, StateElement)]
pub struct SkillCooldowns {
    #[hidden_element]
    entries: Vec<SkillCooldown>,
}

impl SkillCooldowns {
    pub fn start(&mut self, skill_id: SkillId, until: ClientTick) {
        match self.entries.iter_mut().find(|entry| entry.skill_id == skill_id) {
            Some(entry) => entry.until = until,
            None => self.entries.push(SkillCooldown { skill_id, until }),
        }
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, skill_id: SkillId) {
        self.entries.retain(|entry| entry.skill_id != skill_id);
    }

    /// Remove cooldowns whose end tick has been reached.
    pub fn prune(&mut self, client_tick: ClientTick) -> bool {
        let previous = self.entries.len();
        self.entries.retain(|entry| entry.until.0 > client_tick.0);
        self.entries.len() != previous
    }

    #[allow(dead_code)]
    pub fn until(&self, skill_id: SkillId) -> Option<ClientTick> {
        self.entries
            .iter()
            .find(|entry| entry.skill_id == skill_id)
            .map(|entry| entry.until)
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

    use super::SkillCooldowns;

    #[test]
    fn start_prunes_and_expires() {
        let mut cooldowns = SkillCooldowns::default();
        cooldowns.start(SkillId(1), ClientTick(100));
        cooldowns.start(SkillId(2), ClientTick(50));

        assert_eq!(cooldowns.until(SkillId(1)).map(|tick| tick.0), Some(100));
        assert!(cooldowns.prune(ClientTick(60)));
        assert!(cooldowns.until(SkillId(2)).is_none());
        assert_eq!(cooldowns.entries().len(), 1);

        cooldowns.start(SkillId(1), ClientTick(200));
        assert_eq!(cooldowns.until(SkillId(1)).map(|tick| tick.0), Some(200));
        assert_eq!(cooldowns.entries().len(), 1);
    }
}
