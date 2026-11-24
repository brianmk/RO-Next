//! Active quests and hunting objectives (`ZC_QUEST_LIST` 0x09F8 and the
//! quest notification/update packets). The server sends the list on map entry
//! and small updates afterwards; this state feeds the quest tracker window.

use ragnarok_packets::Quest;
use ro_next_interface::element::StateElement;
use rust_state::RustState;

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct QuestObjective {
    pub name: String,
    pub current: u16,
    pub total: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, RustState, StateElement)]
pub struct QuestEntry {
    pub quest_id: u32,
    pub active: bool,
    pub objectives: Vec<QuestObjective>,
}

#[derive(Default, RustState, StateElement)]
pub struct QuestLog {
    #[hidden_element]
    entries: Vec<QuestEntry>,
}

impl QuestLog {
    pub fn set_list(&mut self, quests: &[Quest]) {
        self.entries = quests
            .iter()
            .map(|quest| QuestEntry {
                quest_id: quest.quest_id,
                active: quest.active != 0,
                objectives: quest
                    .objective_details
                    .iter()
                    .map(|objective| QuestObjective {
                        name: objective.mob_name.clone(),
                        current: objective.kill_count,
                        total: objective.total_count,
                    })
                    .collect(),
            })
            .collect();
    }

    pub fn set_active(&mut self, quest_id: u32, active: bool) {
        match self.entries.iter_mut().find(|entry| entry.quest_id == quest_id) {
            Some(entry) => entry.active = active,
            None if active => self.entries.push(QuestEntry {
                quest_id,
                active: true,
                objectives: Vec::new(),
            }),
            None => {}
        }
    }

    pub fn remove(&mut self, quest_id: u32) {
        self.entries.retain(|entry| entry.quest_id != quest_id);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn active(&self) -> impl Iterator<Item = &QuestEntry> {
        self.entries.iter().filter(|entry| entry.active)
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::QuestLog;

    #[test]
    fn updates_and_removals_follow_the_server() {
        let mut log = QuestLog::default();
        log.set_active(10, true);
        log.set_active(20, false);
        assert_eq!(log.active().count(), 1);
        log.set_active(10, false);
        assert!(log.active().next().is_none());
        log.remove(10);
        assert!(log.is_empty());
    }
}
