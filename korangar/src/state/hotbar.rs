//! Hotbar state for the 2019 (4 tabs × 38 entries) hotkey protocol.
//!
//! The 2019 server stores `ZC_SHORTCUT_KEY_LIST` (0x0B20) as `rotate`, a tab
//! index and 38 entries of `{type, id, level/quantity}`. The old client state
//! only kept the first 10 entries of tab 0 and treated every entry as a skill
//! id, so item hotkeys were cast as skills and every other tab/page was lost.
//!
//! This state keeps all four tabs with all 38 entries and mirrors the first ten
//! entries of the active tab into the ten visible skill slots used by the
//! hotbar window and the F1-F10 keys.

use korangar_interface::element::StateElement;
use korangar_networking::NetworkingSystem;
use ragnarok_packets::handler::PacketCallback;
use ragnarok_packets::{HotbarSlot, HotbarTab, HotkeyData, HotkeyType, ItemId};
use rust_state::RustState;

use crate::state::skills::LearnableSkill;

/// Number of hotkey tabs of the 2019 client.
pub const HOTBAR_TAB_COUNT: usize = 4;
/// Number of hotkey entries per tab in the 2019 client.
pub const HOTBAR_ENTRIES_PER_TAB: usize = 38;
/// Number of hotbar slots that are rendered and bound to F1-F10.
pub const VISIBLE_HOTBAR_SLOTS: usize = 10;

/// An item bound to a hotbar slot.
#[derive(Clone, Debug, RustState, StateElement)]
pub struct HotbarItem {
    pub item_id: ItemId,
    pub quantity: u16,
}

/// One entry of a hotbar tab.
#[derive(Clone, Default, RustState, StateElement)]
pub struct HotbarEntry {
    pub skill: Option<LearnableSkill>,
    pub item: Option<HotbarItem>,
}

#[derive(Default, RustState, StateElement)]
pub struct Hotbar {
    /// First ten entries of the active tab (rendered by the hotbar window and
    /// used by the F1-F10 keys).
    skills: [Option<LearnableSkill>; VISIBLE_HOTBAR_SLOTS],
    /// All four tabs with all 38 entries each.
    pages: Vec<Vec<HotbarEntry>>,
    /// Index of the active tab.
    active_tab: usize,
}

impl Hotbar {
    fn ensure_pages(&mut self) {
        if self.pages.len() != HOTBAR_TAB_COUNT {
            self.pages = (0..HOTBAR_TAB_COUNT)
                .map(|_| vec![HotbarEntry::default(); HOTBAR_ENTRIES_PER_TAB])
                .collect();
        }
    }

    fn page(&self, tab: usize) -> Option<&Vec<HotbarEntry>> {
        self.pages.get(tab)
    }

    fn page_mut(&mut self, tab: usize) -> Option<&mut Vec<HotbarEntry>> {
        self.pages.get_mut(tab)
    }

    /// Index of the active tab.
    pub fn active_tab(&self) -> usize {
        self.active_tab
    }

    /// Rebuilds the visible skill slots from the active tab.
    ///
    /// Item entries are not rendered in the skill slots; they stay usable from
    /// the keyboard and are kept in the page so a later client version can show
    /// them.
    pub fn refresh_visible_slots(&mut self) {
        self.ensure_pages();

        let active_tab = self.active_tab;

        for slot in 0..VISIBLE_HOTBAR_SLOTS {
            let skill = self
                .page(active_tab)
                .and_then(|page| page.get(slot))
                .and_then(|entry| entry.skill.clone());

            self.skills[slot] = skill;
        }
    }

    /// Applies a skill entry from the hotkey packet to a (tab, slot).
    pub fn apply_skill(&mut self, tab: usize, slot: usize, skill: Option<LearnableSkill>) {
        self.ensure_pages();

        if let Some(page) = self.page_mut(tab)
            && let Some(entry) = page.get_mut(slot)
        {
            entry.skill = skill;
            entry.item = None;
        }

        if tab == self.active_tab && slot < VISIBLE_HOTBAR_SLOTS {
            self.skills[slot] = self.page(tab).and_then(|page| page.get(slot)).and_then(|entry| entry.skill.clone());
        }
    }

    /// Applies an item entry from the hotkey packet to a (tab, slot).
    pub fn apply_item(&mut self, tab: usize, slot: usize, item_id: ItemId, quantity: u16) {
        self.ensure_pages();

        if let Some(page) = self.page_mut(tab)
            && let Some(entry) = page.get_mut(slot)
        {
            entry.item = Some(HotbarItem { item_id, quantity });
            entry.skill = None;
        }

        // An item replaces whatever skill was visible in that slot.
        if tab == self.active_tab && slot < VISIBLE_HOTBAR_SLOTS {
            self.skills[slot] = None;
        }
    }

    /// Clears a (tab, slot) entry.
    pub fn clear_entry(&mut self, tab: usize, slot: usize) {
        self.ensure_pages();

        if let Some(page) = self.page_mut(tab)
            && let Some(entry) = page.get_mut(slot)
        {
            *entry = HotbarEntry::default();
        }

        if tab == self.active_tab && slot < VISIBLE_HOTBAR_SLOTS {
            self.skills[slot] = None;
        }
    }

    /// Switches to another tab and returns the new active tab.
    pub fn cycle_tab(&mut self, delta: i32) -> usize {
        let tab_count = HOTBAR_TAB_COUNT as i32;
        self.active_tab = (self.active_tab as i32 + delta).rem_euclid(tab_count) as usize;
        self.refresh_visible_slots();
        self.active_tab
    }

    /// Item bound to a slot of the active tab, if any.
    pub fn item_in_slot(&self, slot: HotbarSlot) -> Option<HotbarItem> {
        self.page(self.active_tab)
            .and_then(|page| page.get(slot.0 as usize))
            .and_then(|entry| entry.item.clone())
    }

    /// Update the slot and notify the map server.
    pub fn update_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot, skill: LearnableSkill)
    where
        Callback: PacketCallback + Send,
    {
        let hotkey_data = HotkeyData {
            hotkey_type: HotkeyType::Skill,
            item_or_skill_id: skill.skill_id.0 as u32,
            quantity_or_skill_level: skill.maximum_level.0,
        };

        let _ = networking_system.set_hotkey_data(HotbarTab(self.active_tab as u16), slot, hotkey_data);

        self.apply_skill(self.active_tab, slot.0 as usize, Some(skill));
    }

    /// Swap two slots in the hotbar and notify the map server.
    pub fn swap_slot<Callback>(
        &mut self,
        networking_system: &mut NetworkingSystem<Callback>,
        source_slot: HotbarSlot,
        destination_slot: HotbarSlot,
    ) where
        Callback: PacketCallback + Send,
    {
        if source_slot == destination_slot {
            return;
        }

        self.ensure_pages();

        let tab = self.active_tab;

        let (first, second) = {
            let Some(page) = self.page_mut(tab) else {
                return;
            };

            let Some(source) = page.get(source_slot.0 as usize).cloned() else {
                return;
            };
            let Some(destination) = page.get(destination_slot.0 as usize).cloned() else {
                return;
            };

            page[source_slot.0 as usize] = destination.clone();
            page[destination_slot.0 as usize] = source.clone();

            (source, destination)
        };

        let to_hotkey_data = |entry: &HotbarEntry| match (&entry.skill, &entry.item) {
            (Some(skill), _) => HotkeyData {
                hotkey_type: HotkeyType::Skill,
                item_or_skill_id: skill.skill_id.0 as u32,
                quantity_or_skill_level: skill.maximum_level.0,
            },
            (_, Some(item)) => HotkeyData {
                hotkey_type: HotkeyType::Item,
                item_or_skill_id: item.item_id.0,
                quantity_or_skill_level: item.quantity,
            },
            _ => HotkeyData::UNBOUND,
        };

        let tab = HotbarTab(self.active_tab as u16);
        let _ = networking_system.set_hotkey_data(tab, destination_slot, to_hotkey_data(&first));
        let _ = networking_system.set_hotkey_data(tab, source_slot, to_hotkey_data(&second));

        self.refresh_visible_slots();
    }

    /// Clear the slot and notify the map server.
    pub fn clear_slot<Callback>(&mut self, networking_system: &mut NetworkingSystem<Callback>, slot: HotbarSlot)
    where
        Callback: PacketCallback + Send,
    {
        let _ = networking_system.set_hotkey_data(HotbarTab(self.active_tab as u16), slot, HotkeyData::UNBOUND);

        self.clear_entry(self.active_tab, slot.0 as usize);
    }

    pub fn get_skill_in_slot(&self, slot: HotbarSlot) -> &Option<LearnableSkill> {
        &self.skills[slot.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{ClientTick, SkillId, SkillLevel};

    use super::*;
    use crate::state::skills::SkillAcquisition;
    use crate::world::SpriteAnimationState;

    fn learnable_skill(id: u16) -> LearnableSkill {
        LearnableSkill {
            skill_id: SkillId(id),
            maximum_level: SkillLevel(5),
            file_name: format!("skill_{id}"),
            skill_name: format!("Skill {id}"),
            can_select_level: false,
            acquisition: SkillAcquisition::Job,
            required_skills: Default::default(),
            required_for_skills: Default::default(),
            sprite: None,
            actions: None,
            animation_state: SpriteAnimationState::new(ClientTick(0)),
        }
    }

    #[test]
    fn packet_entries_of_all_tabs_are_stored() {
        let mut hotbar = Hotbar::default();

        hotbar.apply_skill(2, 5, Some(learnable_skill(7)));
        hotbar.apply_item(2, 0, ItemId(501), 10);

        // Tab 0 is active, so the visible slots stay empty ...
        assert!(hotbar.get_skill_in_slot(HotbarSlot(5)).is_none());

        // ... and switching to tab 2 shows the skill of entry 5.
        assert_eq!(hotbar.cycle_tab(2), 2);
        assert_eq!(
            hotbar.get_skill_in_slot(HotbarSlot(5)).as_ref().map(|skill| skill.skill_id),
            Some(SkillId(7))
        );
        assert_eq!(hotbar.active_tab(), 2);
    }

    #[test]
    fn item_entries_are_not_misread_as_skills() {
        let mut hotbar = Hotbar::default();

        hotbar.apply_item(0, 1, ItemId(501), 3);

        assert!(hotbar.get_skill_in_slot(HotbarSlot(1)).is_none());
        let item = hotbar.item_in_slot(HotbarSlot(1)).unwrap();
        assert_eq!(item.item_id, ItemId(501));
        assert_eq!(item.quantity, 3);
        assert!(hotbar.item_in_slot(HotbarSlot(2)).is_none());
    }

    #[test]
    fn clearing_a_slot_clears_the_page_entry() {
        let mut hotbar = Hotbar::default();

        hotbar.apply_skill(1, 3, Some(learnable_skill(9)));
        hotbar.cycle_tab(1);
        assert!(hotbar.get_skill_in_slot(HotbarSlot(3)).is_some());

        hotbar.clear_entry(1, 3);
        assert!(hotbar.get_skill_in_slot(HotbarSlot(3)).is_none());
    }

    #[test]
    fn tab_cycling_wraps_around() {
        let mut hotbar = Hotbar::default();

        assert_eq!(hotbar.cycle_tab(-1), HOTBAR_TAB_COUNT - 1);
        assert_eq!(hotbar.cycle_tab(1), 0);
    }
}
