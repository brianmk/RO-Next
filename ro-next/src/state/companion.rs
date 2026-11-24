//! Pet / homunculus / mercenary information for the 2019 rAthena server.
//!
//! The map server sends the status sheet of the local player's own units:
//!
//! * `ZC_PROPERTY_PET` (0x01A2) — pet name, level, hunger, intimacy, accessory
//!   and monster class (clif.cpp:8286),
//! * `ZC_CHANGESTATE_PET` (0x01A4) — intimacy/hunger/accessory change
//!   (clif.cpp:8230),
//! * `ZC_PROPERTY_HOMUN` (0x09F7) — full homunculus status sheet
//!   (clif.cpp:1811),
//! * `ZC_HO_PAR_CHANGE` (0x07DB) — homunculus HP/SP/EXP change (clif.cpp:1754),
//! * `ZC_MER_INIT` (0x029B) — full mercenary status sheet (clif.cpp:18234),
//! * `ZC_MER_PAR_CHANGE` (0x02A2) — mercenary HP/SP/faith/kill change
//!   (clif.cpp:18166).
//!
//! The pet and homunculus packets carry no actor id (`ZC_MER_INIT` does start
//! with the mercenary block id), so the companion info windows are matched to
//! an entity by job id (pet), name (homunculus) or block id (mercenary).

use ragnarok_packets::{EntityId, HomunculusPropertyPacket2019, JobId, MercenaryInfoPacket2019, PetPropertyPacket2019};
use ro_next_interface::element::StateElement;
use rust_state::RustState;

/// rAthena `_sp` ids used by the companion parameter-change packets
/// (`src/map/map.hpp:514`).
pub const SP_BASEEXP: u16 = 1;
pub const SP_HP: u16 = 5;
pub const SP_MAXHP: u16 = 6;
pub const SP_SP: u16 = 7;
pub const SP_MAXSP: u16 = 8;
pub const SP_MERCKILLS: u16 = 189;
pub const SP_MERCFAITH: u16 = 190;

/// Pet base data from `ZC_PROPERTY_PET`.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct PetInfo {
    pub name: String,
    pub renamed: bool,
    pub level: u16,
    /// Fullness/hunger value (`pet.hungry`, 0..=100).
    pub hunger: u16,
    /// Intimacy value (`pet.intimate`).
    pub intimacy: u16,
    /// Accessory item view id.
    pub accessory: u16,
    /// Monster class id of the pet (used to match the pet entity).
    pub job_id: JobId,
}

impl Default for PetInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            renamed: false,
            level: 0,
            hunger: 0,
            intimacy: 0,
            accessory: 0,
            job_id: JobId(0),
        }
    }
}

/// Homunculus status sheet from `ZC_PROPERTY_HOMUN`.
#[derive(Debug, Clone, Default, RustState, StateElement)]
pub struct HomunculusInfo {
    pub name: String,
    pub renamed: bool,
    pub vaporized: bool,
    pub dead: bool,
    pub level: u16,
    pub hunger: u16,
    pub intimacy: u16,
    pub item_id: u32,
    pub attack: u16,
    pub magic_attack: u16,
    pub hit: u16,
    pub critical: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub flee: u16,
    pub attack_motion: u16,
    pub health_points: u32,
    pub maximum_health_points: u32,
    pub spell_points: u16,
    pub maximum_spell_points: u16,
    pub experience: u32,
    pub maximum_experience: u32,
    pub skill_points: u16,
    pub attack_range: u16,
}

impl HomunculusInfo {
    pub fn health_text(&self) -> String {
        match self.maximum_health_points {
            0 => format!("{}", self.health_points),
            maximum => format!("{} / {}", self.health_points, maximum),
        }
    }

    pub fn spell_text(&self) -> String {
        match self.maximum_spell_points {
            0 => format!("{}", self.spell_points),
            maximum => format!("{} / {}", self.spell_points, maximum),
        }
    }
}

/// Mercenary status sheet from `ZC_MER_INIT`.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct MercenaryInfo {
    pub entity_id: EntityId,
    pub name: String,
    pub level: u16,
    pub attack: u16,
    pub magic_attack: u16,
    pub hit: u16,
    pub critical: u16,
    pub defense: u16,
    pub magic_defense: u16,
    pub flee: u16,
    pub attack_motion: u16,
    pub health_points: u32,
    pub maximum_health_points: u32,
    pub spell_points: u32,
    pub maximum_spell_points: u32,
    pub expire_time: u32,
    pub faith: u16,
    pub calls: u32,
    pub kills: u32,
    pub attack_range: u16,
}

impl Default for MercenaryInfo {
    fn default() -> Self {
        Self {
            entity_id: EntityId(0),
            name: String::new(),
            level: 0,
            attack: 0,
            magic_attack: 0,
            hit: 0,
            critical: 0,
            defense: 0,
            magic_defense: 0,
            flee: 0,
            attack_motion: 0,
            health_points: 0,
            maximum_health_points: 0,
            spell_points: 0,
            maximum_spell_points: 0,
            expire_time: 0,
            faith: 0,
            calls: 0,
            kills: 0,
            attack_range: 0,
        }
    }
}

/// Information about the local player's companion units.
#[derive(Default, RustState, StateElement)]
pub struct Companion {
    pub pet: Option<PetInfo>,
    pub homunculus: Option<HomunculusInfo>,
    pub mercenary: Option<MercenaryInfo>,
}

impl Companion {
    /// Applies `ZC_PROPERTY_PET` (0x01A2).
    pub fn apply_pet_property(&mut self, property: PetPropertyPacket2019) {
        self.pet = Some(PetInfo {
            name: property.name.trim_end_matches('\0').to_owned(),
            renamed: property.renamed != 0,
            level: property.level,
            hunger: property.hunger,
            intimacy: property.intimacy,
            accessory: property.accessory,
            job_id: property.job_id,
        });
    }

    /// Applies `ZC_CHANGESTATE_PET` (0x01A4).
    ///
    /// `0` = pre-init, `1` = intimacy, `2` = hunger, `3` = accessory,
    /// `4` = performance, `5` = hairstyle (clif.cpp:8230).
    pub fn apply_pet_state_change(&mut self, state_type: u8, value: i32) {
        let Some(pet) = &mut self.pet else {
            return;
        };

        match state_type {
            1 => pet.intimacy = value.clamp(0, u16::MAX as i32) as u16,
            2 => pet.hunger = value.clamp(0, u16::MAX as i32) as u16,
            3 => pet.accessory = value.clamp(0, u16::MAX as i32) as u16,
            _ => {}
        }
    }

    /// Applies `ZC_PROPERTY_HOMUN` (0x09F7).
    pub fn apply_homunculus_property(&mut self, property: HomunculusPropertyPacket2019) {
        self.homunculus = Some(HomunculusInfo {
            name: property.name.trim_end_matches('\0').to_owned(),
            renamed: property.flags & 0x1 != 0,
            vaporized: property.flags & 0x2 != 0,
            dead: property.flags & 0x4 != 0,
            level: property.level,
            hunger: property.hunger,
            intimacy: property.intimacy,
            item_id: property.item_id,
            attack: property.attack,
            magic_attack: property.magic_attack,
            hit: property.hit,
            critical: property.critical,
            defense: property.defense,
            magic_defense: property.magic_defense,
            flee: property.flee,
            attack_motion: property.attack_motion,
            health_points: property.health_points,
            maximum_health_points: property.maximum_health_points,
            spell_points: property.spell_points,
            maximum_spell_points: property.maximum_spell_points,
            experience: property.experience,
            maximum_experience: property.maximum_experience,
            skill_points: property.skill_points,
            attack_range: property.attack_range,
        });
    }

    /// Applies `ZC_HO_PAR_CHANGE` (0x07DB).
    pub fn apply_homunculus_parameter_change(&mut self, parameter: u16, value: i32) {
        let Some(homunculus) = &mut self.homunculus else {
            return;
        };

        let value = value.max(0) as u32;

        match parameter {
            SP_BASEEXP => homunculus.experience = value,
            SP_HP => homunculus.health_points = value,
            SP_SP => homunculus.spell_points = u16::try_from(value).unwrap_or(u16::MAX),
            _ => {}
        }
    }

    /// Applies `ZC_MER_INIT` (0x029B).
    pub fn apply_mercenary_info(&mut self, info: MercenaryInfoPacket2019) {
        self.mercenary = Some(MercenaryInfo {
            entity_id: info.entity_id,
            name: info.name.trim_end_matches('\0').to_owned(),
            level: info.level,
            attack: info.attack,
            magic_attack: info.magic_attack,
            hit: info.hit,
            critical: info.critical,
            defense: info.defense,
            magic_defense: info.magic_defense,
            flee: info.flee,
            attack_motion: info.attack_motion,
            health_points: info.health_points,
            maximum_health_points: info.maximum_health_points,
            spell_points: info.spell_points,
            maximum_spell_points: info.maximum_spell_points,
            expire_time: info.expire_time,
            faith: info.faith,
            calls: info.calls,
            kills: info.kills,
            attack_range: info.attack_range,
        });
    }

    /// Applies `ZC_MER_PAR_CHANGE` (0x02A2).
    pub fn apply_mercenary_parameter_change(&mut self, parameter: u16, value: i32) {
        let Some(mercenary) = &mut self.mercenary else {
            return;
        };

        let unsigned = value.max(0) as u32;

        match parameter {
            SP_HP => mercenary.health_points = unsigned,
            SP_MAXHP => mercenary.maximum_health_points = unsigned,
            SP_SP => mercenary.spell_points = unsigned,
            SP_MAXSP => mercenary.maximum_spell_points = unsigned,
            SP_MERCKILLS => mercenary.kills = unsigned,
            SP_MERCFAITH => mercenary.faith = u16::try_from(unsigned).unwrap_or(u16::MAX),
            _ => {}
        }
    }

    /// Clears all companion information (map change/logout).
    pub fn clear(&mut self) {
        self.pet = None;
        self.homunculus = None;
        self.mercenary = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pet() -> PetPropertyPacket2019 {
        PetPropertyPacket2019 {
            name: "Poring".to_owned(),
            renamed: 0,
            level: 10,
            hunger: 80,
            intimacy: 500,
            accessory: 0,
            job_id: JobId(1002),
        }
    }

    fn homunculus() -> HomunculusPropertyPacket2019 {
        HomunculusPropertyPacket2019 {
            name: "Lif".to_owned(),
            flags: 0,
            level: 40,
            hunger: 50,
            intimacy: 750,
            item_id: 0,
            attack: 100,
            magic_attack: 50,
            hit: 80,
            critical: 3,
            defense: 20,
            magic_defense: 10,
            flee: 60,
            attack_motion: 500,
            health_points: 1000,
            maximum_health_points: 2000,
            spell_points: 100,
            maximum_spell_points: 200,
            experience: 10,
            maximum_experience: 100,
            skill_points: 3,
            attack_range: 1,
        }
    }

    fn mercenary() -> MercenaryInfoPacket2019 {
        MercenaryInfoPacket2019 {
            entity_id: EntityId(150_000),
            attack: 100,
            magic_attack: 0,
            hit: 90,
            critical: 5,
            defense: 30,
            magic_defense: 10,
            flee: 70,
            attack_motion: 400,
            name: "Bow Mercenary".to_owned(),
            level: 30,
            health_points: 3000,
            maximum_health_points: 3000,
            spell_points: 100,
            maximum_spell_points: 100,
            expire_time: 0,
            faith: 50,
            calls: 1,
            kills: 4,
            attack_range: 9,
        }
    }

    #[test]
    fn pet_property_and_state_changes_are_applied() {
        let mut companion = Companion::default();
        companion.apply_pet_property(pet());

        let pet = companion.pet.as_ref().unwrap();
        assert_eq!(pet.name, "Poring");
        assert_eq!(pet.level, 10);
        assert_eq!(pet.job_id, JobId(1002));

        companion.apply_pet_state_change(1, 900);
        companion.apply_pet_state_change(2, 40);
        let pet = companion.pet.as_ref().unwrap();
        assert_eq!(pet.intimacy, 900);
        assert_eq!(pet.hunger, 40);
    }

    #[test]
    fn homunculus_property_and_parameter_change_are_applied() {
        let mut companion = Companion::default();
        companion.apply_homunculus_property(homunculus());
        assert_eq!(companion.homunculus.as_ref().unwrap().health_text(), "1000 / 2000");

        companion.apply_homunculus_parameter_change(SP_HP, 1500);
        companion.apply_homunculus_parameter_change(SP_SP, 150);
        let homunculus = companion.homunculus.as_ref().unwrap();
        assert_eq!(homunculus.health_points, 1500);
        assert_eq!(homunculus.spell_points, 150);
    }

    #[test]
    fn mercenary_info_and_parameter_change_are_applied() {
        let mut companion = Companion::default();
        companion.apply_mercenary_info(mercenary());
        assert_eq!(companion.mercenary.as_ref().unwrap().name, "Bow Mercenary");

        companion.apply_mercenary_parameter_change(SP_HP, 1200);
        companion.apply_mercenary_parameter_change(SP_MERCKILLS, 8);
        companion.apply_mercenary_parameter_change(SP_MERCFAITH, 75);
        let mercenary = companion.mercenary.as_ref().unwrap();
        assert_eq!(mercenary.health_points, 1200);
        assert_eq!(mercenary.kills, 8);
        assert_eq!(mercenary.faith, 75);
    }
}
