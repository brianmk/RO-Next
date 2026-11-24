//! Item type values as sent by the server (`enum item_types` in rAthena's
//! `common/mmo.hpp`).
//!
//! This is the **single** source of the item type numbers and of the item type
//! classification used by the networking packet handlers and the client
//! inventory. It used to be duplicated: raw item type constants in the 2019
//! version handler (storage packet equipment detection) and a second copy in
//! [`crate::state::inventory`] (inventory tab / use action classification).
//!
//! The numbers are protocol values of the supported server generation; they are
//! not client settings, so changing them means supporting a different server.

/// Healing items (potions, herbs, ...).
pub const HEALING: u8 = 0;
/// Reserved "unknown" type.
pub const UNKNOWN: u8 = 1;
/// Usable items (usable from the inventory, e.g. some boxes/scrolls).
pub const USABLE: u8 = 2;
/// Miscellaneous items.
pub const ETC: u8 = 3;
/// Armor (and shields/headgear/garments/shoes/accessories).
pub const ARMOR: u8 = 4;
/// Weapons.
pub const WEAPON: u8 = 5;
/// Cards.
pub const CARD: u8 = 6;
/// Pet eggs. The server sends pet eggs as [`ARMOR`] to the client.
pub const PET_EGG: u8 = 7;
/// Pet armor.
pub const PET_ARMOR: u8 = 8;
/// Reserved "unknown" type.
pub const UNKNOWN2: u8 = 9;
/// Ammunition.
pub const AMMO: u8 = 10;
/// Items that are consumed after a delay (e.g. some buff items).
pub const DELAY_CONSUME: u8 = 11;
/// Legacy cash shop consumables.
///
/// Older rAthena versions used 12 for cash consumables; current versions use
/// 12 for shadow gear and 18 for cash items. The server maps shadow gear to
/// [`WEAPON`] or [`ARMOR`] before it sends it to the client, so a wire value of
/// 12 only reaches the client from an old server that meant cash items. It is
/// therefore classified as a consumable, like it always was.
pub const CASH_OLD: u8 = 12;
/// Cash shop consumables.
pub const CASH: u8 = 18;

/// Coarse category of an item type, used by the inventory window tabs.
///
/// This is the networking-side classification; the client interface maps it to
/// its own UI enum. Keeping it here means the inventory state and the packet
/// handlers cannot disagree about what an item type means.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTypeCategory {
    /// Items shown in the equipments tab (armor, weapons, ammunition, ...).
    Equipments,
    /// Items shown in the consumables tab (potions, usable items, ...).
    Consumables,
    /// Everything else, including unknown item types.
    Misc,
}

/// Category of an item type value.
///
/// Unknown values (including the reserved `UNKNOWN`/`UNKNOWN2` types) are put
/// into [`ItemTypeCategory::Misc`] so that no item can disappear from the
/// inventory.
pub fn category(item_type: u8) -> ItemTypeCategory {
    match item_type {
        ARMOR | WEAPON | PET_ARMOR | AMMO => ItemTypeCategory::Equipments,
        HEALING | USABLE | DELAY_CONSUME | CASH | CASH_OLD => ItemTypeCategory::Consumables,
        _ => ItemTypeCategory::Misc,
    }
}

/// Whether the item type describes equipment (used by packet handlers that have
/// to decide whether an item without an explicit equipment position is still
/// equipment, e.g. the 2019 storage item packet).
pub fn is_equipment(item_type: u8) -> bool {
    matches!(item_type, ARMOR | WEAPON | PET_ARMOR | AMMO)
}

/// Whether an item of this type can be used from the inventory ("use" instead
/// of "equip" in the item context menu). The server still validates all use
/// conditions; this only decides whether the client offers the action.
pub fn is_usable(item_type: u8) -> bool {
    matches!(item_type, HEALING | USABLE | DELAY_CONSUME | CASH | CASH_OLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_rathena_item_types_enum() {
        // `enum item_types` in rAthena `src/common/mmo.hpp`.
        assert_eq!(
            [
                HEALING,
                UNKNOWN,
                USABLE,
                ETC,
                ARMOR,
                WEAPON,
                CARD,
                PET_EGG,
                PET_ARMOR,
                UNKNOWN2,
                AMMO,
                DELAY_CONSUME,
                CASH_OLD,
                CASH,
            ],
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 18],
        );
    }

    #[test]
    fn equipment_types_are_classified_as_equipment() {
        for item_type in [ARMOR, WEAPON, PET_ARMOR, AMMO] {
            assert!(is_equipment(item_type), "{item_type}");
            assert_eq!(category(item_type), ItemTypeCategory::Equipments, "{item_type}");
        }
    }

    #[test]
    fn consumable_types_are_classified_as_consumables() {
        for item_type in [HEALING, USABLE, DELAY_CONSUME, CASH, CASH_OLD] {
            assert!(is_usable(item_type), "{item_type}");
            assert_eq!(category(item_type), ItemTypeCategory::Consumables, "{item_type}");
        }
    }

    #[test]
    fn misc_and_unknown_types_never_disappear() {
        for item_type in [ETC, CARD, PET_EGG, UNKNOWN, UNKNOWN2, 13, 17, 19, 255] {
            assert!(!is_equipment(item_type), "{item_type}");
            assert!(!is_usable(item_type), "{item_type}");
            assert_eq!(category(item_type), ItemTypeCategory::Misc, "{item_type}");
        }
    }

    #[test]
    fn the_two_packet_handler_helpers_agree_with_the_category() {
        // The packet handlers and the inventory tab must not disagree.
        for item_type in 0..=255u8 {
            let category = category(item_type);
            assert_eq!(is_equipment(item_type), category == ItemTypeCategory::Equipments, "{item_type}");
            if !is_equipment(item_type) {
                assert_eq!(is_usable(item_type), category == ItemTypeCategory::Consumables, "{item_type}");
            }
        }
    }
}
