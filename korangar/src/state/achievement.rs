//! Achievement state (`ZC_ALL_ACH_LIST` 0x0A23 and `ZC_ACH_UPDATE` 0x0A24).
//!
//! The 2019 rAthena achievement packets carry the achievement id, the ten
//! objective progress counters, the completion timestamp and the
//! completed/rewarded flags. They do **not** carry the achievement title or
//! description: those live in the official client's
//! `data\achievement\achievement_list.lub`, which this client data set does
//! not ship (checked against `data.grf`, 653299 files). The window therefore
//! lists the id and the raw progress counters instead of inventing names.
//!
//! The reward claim (`CZ_REQ_ACH_REWARD`, 0x0A25) is only offered for
//! completed, not-yet-rewarded achievements; the server's answer arrives as
//! `ZC_REQ_ACH_REWARD_ACK` (0x0A26) and, on success, as a `ZC_ACH_UPDATE`
//! with `got_rewarded = 1`.

use korangar_interface::element::StateElement;
use ragnarok_packets::AchievementData;
use rust_state::RustState;

/// One achievement of the player's log.
#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct AchievementEntry {
    pub achievement_id: u32,
    /// `completed` flag of the packet (the timestamp is the authoritative
    /// completion marker, the flag is what the client uses).
    pub completed: bool,
    /// Unix timestamp of completion, `0` while incomplete.
    pub completion_timestamp: u32,
    /// Server flag: the reward was already claimed.
    pub rewarded: bool,
    /// Objective progress counters. The packets do not carry the objective
    /// targets, so these are raw counters.
    pub objectives: [u32; 10],
}

impl AchievementEntry {
    fn from_packet(achievement: &AchievementData) -> Self {
        Self {
            achievement_id: achievement.acheivement_id,
            completed: achievement.is_completed != 0,
            completion_timestamp: achievement.completion_timestamp,
            rewarded: achievement.got_rewarded != 0,
            objectives: achievement.objectives,
        }
    }

    /// Whether the reward can be claimed right now.
    pub fn is_claimable(&self) -> bool {
        self.completed && !self.rewarded
    }

    /// Human readable objective counters, e.g. `120 / 0 / 5 / ...`, trimmed
    /// after the last non-zero counter. Empty when there is no progress yet.
    pub fn objective_summary(&self) -> String {
        let last = self.objectives.iter().rposition(|count| *count != 0);

        match last {
            Some(last) => {
                let counters = self.objectives[..=last].iter().map(u32::to_string).collect::<Vec<_>>().join(" / ");
                format!("{counters} (raw counters)")
            }
            None => "no progress".to_owned(),
        }
    }
}

/// Achievement log and achievement-level progress.
#[derive(Default, RustState, StateElement)]
pub struct AchievementState {
    #[hidden_element]
    entries: Vec<AchievementEntry>,
    total_score: u32,
    level: u16,
    experience: u32,
    experience_to_next_level: u32,
    /// Achievement id typed into the reward claim field.
    pub claim_input: String,
    /// Answer of the last reward claim.
    status: String,
}

impl AchievementState {
    /// Replace the whole log with a `ZC_ALL_ACH_LIST` update.
    pub fn set_list(
        &mut self,
        achievements: &[AchievementData],
        total_score: u32,
        level: u16,
        experience: u32,
        experience_to_next_level: u32,
    ) {
        self.entries = achievements.iter().map(AchievementEntry::from_packet).collect();
        self.entries.sort_by_key(|entry| entry.achievement_id);
        self.set_level(total_score, level, experience, experience_to_next_level);
        self.status.clear();
    }

    /// Apply a single `ZC_ACH_UPDATE`. The server sends this for progress,
    /// completion and reward updates (and with a zeroed achievement on map
    /// entry, which only updates the level).
    pub fn update(&mut self, achievement: &AchievementData, total_score: u32, level: u16, experience: u32, experience_to_next_level: u32) {
        self.set_level(total_score, level, experience, experience_to_next_level);

        if achievement.acheivement_id == 0 {
            return;
        }

        let entry = AchievementEntry::from_packet(achievement);

        match self
            .entries
            .binary_search_by_key(&entry.achievement_id, |existing| existing.achievement_id)
        {
            Ok(index) => self.entries[index] = entry,
            Err(index) => self.entries.insert(index, entry),
        }
    }

    fn set_level(&mut self, total_score: u32, level: u16, experience: u32, experience_to_next_level: u32) {
        self.total_score = total_score;
        self.level = level;
        self.experience = experience;
        self.experience_to_next_level = experience_to_next_level;
    }

    /// Store the result of a reward claim. On success the rewarded flag is set
    /// immediately; the server's following `ZC_ACH_UPDATE` confirms it.
    pub fn set_reward_result(&mut self, achievement_id: u32, success: bool) {
        if success {
            if let Ok(index) = self
                .entries
                .binary_search_by_key(&achievement_id, |existing| existing.achievement_id)
            {
                self.entries[index].rewarded = true;
            }

            self.status = format!("Achievement {achievement_id}: reward granted.");
        } else {
            self.status = format!("Achievement {achievement_id}: reward not available.");
        }
    }

    /// Look up one achievement (used by the state tests).
    #[cfg(test)]
    pub fn entry(&self, achievement_id: u32) -> Option<&AchievementEntry> {
        self.entries
            .binary_search_by_key(&achievement_id, |existing| existing.achievement_id)
            .ok()
            .map(|index| &self.entries[index])
    }

    /// All achievements, sorted by id.
    pub fn entries(&self) -> &[AchievementEntry] {
        &self.entries
    }

    pub fn total_score(&self) -> u32 {
        self.total_score
    }

    pub fn level(&self) -> u16 {
        self.level
    }

    pub fn experience(&self) -> u32 {
        self.experience
    }

    pub fn experience_to_next_level(&self) -> u32 {
        self.experience_to_next_level
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn completed_count(&self) -> usize {
        self.entries.iter().filter(|entry| entry.completed).count()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.status.clear();
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::AchievementData;

    use super::AchievementState;

    fn achievement(id: u32, completed: u8, rewarded: u8) -> AchievementData {
        let mut objectives = [0u32; 10];
        objectives[0] = id * 2;
        objectives[2] = 7;

        AchievementData {
            acheivement_id: id,
            is_completed: completed,
            objectives,
            completion_timestamp: if completed != 0 { 1_600_000_000 } else { 0 },
            got_rewarded: rewarded,
        }
    }

    #[test]
    fn list_and_update_keep_the_log_sorted_and_claimable() {
        let mut state = AchievementState::default();
        state.set_list(&[achievement(20, 0, 0), achievement(10, 1, 0)], 120, 3, 50, 200);

        assert_eq!(state.level(), 3);
        assert_eq!(state.entries()[0].achievement_id, 10);
        assert!(state.entry(10).unwrap().is_claimable());
        assert!(!state.entry(20).unwrap().is_claimable());

        state.update(&achievement(20, 1, 0), 130, 3, 70, 200);
        assert!(state.entry(20).unwrap().is_claimable());
        assert_eq!(state.completed_count(), 2);

        state.set_reward_result(20, true);
        assert!(state.entry(20).unwrap().rewarded);
        assert!(!state.entry(20).unwrap().is_claimable());

        state.set_reward_result(10, false);
        assert!(!state.entry(10).unwrap().rewarded);
    }

    #[test]
    fn zeroed_update_only_changes_the_level() {
        let mut state = AchievementState::default();
        state.set_list(&[achievement(10, 1, 0)], 10, 1, 0, 100);
        state.update(
            &AchievementData {
                acheivement_id: 0,
                is_completed: 0,
                objectives: [0; 10],
                completion_timestamp: 0,
                got_rewarded: 0,
            },
            20,
            2,
            0,
            200,
        );

        assert_eq!(state.entries().len(), 1);
        assert_eq!(state.level(), 2);
        assert_eq!(state.total_score(), 20);
    }
}
