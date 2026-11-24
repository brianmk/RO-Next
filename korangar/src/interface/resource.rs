use ragnarok_packets::{EquipPosition, HotbarSlot};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemSource {
    Inventory,
    /// Merchant pushcart (the merchant's second inventory).
    Cart,
    Storage,
    Equipment {
        position: EquipPosition,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSource {
    SkillTree,
    Hotbar { slot: HotbarSlot },
}
