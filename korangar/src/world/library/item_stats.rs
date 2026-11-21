//! First pass evaluation of the stats of an item from its client description
//! (`iteminfo.lub`).
//!
//! The client does not receive structured item stats from the server. All it
//! has is the item description, which the official client shows in the item
//! tooltip. For equipment the descriptions generated for the server contain the
//! base stats of the item in a fairly regular format:
//!
//! ```text
//! 계열 : ^777777검^000000      공격 : ^77777725^000000
//! 무게 : ^77777750^000000
//! 무기레벨 : ^7777771^000000
//! 요구레벨 : ^7777772^000000
//! 장착 : ^777777노비스계열/검사계열/상인계열/도둑계열 장착가능^000000
//! ```
//!
//! Additional stat bonuses are separate lines that use the English stat
//! abbreviations even in the Korean item data:
//!
//! ```text
//! STR + 2, HIT + 10
//! ```
//!
//! This module parses those lines into [`ItemStats`]. It is a *first pass*: it
//! only understands the simple, unconditional flat values. Percentage bonuses
//! and bonuses that only apply under a condition (e.g. "LUK 90 or higher") are
//! ignored on purpose. The authoritative values for the derived character stats
//! are the ones the server sends through the status packets.

use ragnarok_packets::ItemId;

use super::{Library, Table};
use crate::loaders::GameFileLoader;

/// Stats of a single item evaluated from its client description.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemStats {
    /// Weapon attack (`공격`).
    pub attack: i32,
    /// Defense (`방어`).
    pub defense: i32,
    /// Weight (`무게`).
    pub weight: i32,
    /// Weapon level (`무기레벨`) for weapons, `0` otherwise.
    pub weapon_level: u8,
    /// Required base level (`요구레벨`).
    pub required_level: u16,
    /// Flat stat bonuses from the description (e.g. `STR + 2`).
    pub bonuses: StatBonuses,
}

impl ItemStats {
    /// All zero stats, used for items that have no description.
    pub const DEFAULT: Self = Self {
        attack: 0,
        defense: 0,
        weight: 0,
        weapon_level: 0,
        required_level: 0,
        bonuses: StatBonuses::DEFAULT,
    };

    /// Parse the stats of an item from its (already encoding-fixed) identified
    /// description lines.
    pub fn parse(description: &[String]) -> Self {
        let mut stats = Self::default();

        for line in description {
            let line = strip_color_codes(line);
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let mut is_stat_line = false;

            if let Some(class) = labeled_value(line, "계열") {
                // Only used for the classification of the item, which is not
                // needed right now but helps to skip the equip restriction
                // line below.
                let _ = class;
                is_stat_line = true;
            }

            if let Some(attack) = labeled_number(line, "공격").or_else(|| labeled_number(line, "Attack")) {
                stats.attack = attack;
                is_stat_line = true;
            }

            if let Some(defense) = labeled_number(line, "방어").or_else(|| labeled_number(line, "Defense")) {
                stats.defense = defense;
                is_stat_line = true;
            }

            if let Some(weight) = labeled_number(line, "무게").or_else(|| labeled_number(line, "Weight")) {
                stats.weight = weight;
                is_stat_line = true;
            }

            if let Some(weapon_level) = labeled_number(line, "무기레벨").or_else(|| labeled_number(line, "Weapon Level")) {
                stats.weapon_level = weapon_level.clamp(0, u8::MAX as i32) as u8;
                is_stat_line = true;
            }

            if let Some(required_level) = labeled_number(line, "요구레벨").or_else(|| labeled_number(line, "Required Level")) {
                stats.required_level = required_level.clamp(0, u16::MAX as i32) as u16;
                is_stat_line = true;
            }

            // The equip restriction line of the item description.
            if line.contains("장착") || line.contains("Equip") {
                is_stat_line = true;
            }

            if is_stat_line || is_conditional(line) {
                continue;
            }

            parse_bonus_line(line, &mut stats.bonuses);
        }

        stats
    }
}

impl Table for ItemStats {
    type Key<'a> = ItemId;
    type Storage = ();

    fn load(_game_file_loader: &GameFileLoader) -> mlua::Result<Self::Storage> {
        Ok(())
    }

    fn try_get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> Option<&'a Self> {
        super::ItemInfo::try_get(library, key).map(|item_info| &item_info.stats)
    }

    fn get<'a, 'b>(library: &'a Library, key: Self::Key<'b>) -> &'a Self {
        static DEFAULT: ItemStats = ItemStats::DEFAULT;

        Self::try_get(library, key).unwrap_or(&DEFAULT)
    }
}

/// Flat stat bonuses of an item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatBonuses {
    pub strength: i32,
    pub agility: i32,
    pub vitality: i32,
    pub intelligence: i32,
    pub dexterity: i32,
    pub luck: i32,
    pub attack: i32,
    pub magic_attack: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub hit: i32,
    pub flee: i32,
    pub critical: i32,
    pub attack_speed: i32,
}

impl StatBonuses {
    /// All zero bonuses.
    pub const DEFAULT: Self = Self {
        strength: 0,
        agility: 0,
        vitality: 0,
        intelligence: 0,
        dexterity: 0,
        luck: 0,
        attack: 0,
        magic_attack: 0,
        defense: 0,
        magic_defense: 0,
        hit: 0,
        flee: 0,
        critical: 0,
        attack_speed: 0,
    };

    fn add(&mut self, stat: BonusStat, amount: i32) {
        let value = match stat {
            BonusStat::Strength => &mut self.strength,
            BonusStat::Agility => &mut self.agility,
            BonusStat::Vitality => &mut self.vitality,
            BonusStat::Intelligence => &mut self.intelligence,
            BonusStat::Dexterity => &mut self.dexterity,
            BonusStat::Luck => &mut self.luck,
            BonusStat::Attack => &mut self.attack,
            BonusStat::MagicAttack => &mut self.magic_attack,
            BonusStat::Defense => &mut self.defense,
            BonusStat::MagicDefense => &mut self.magic_defense,
            BonusStat::Hit => &mut self.hit,
            BonusStat::Flee => &mut self.flee,
            BonusStat::Critical => &mut self.critical,
            BonusStat::AttackSpeed => &mut self.attack_speed,
        };

        *value += amount;
    }

    /// Add the bonuses of another item to these bonuses.
    pub fn merge(&mut self, other: &Self) {
        self.strength += other.strength;
        self.agility += other.agility;
        self.vitality += other.vitality;
        self.intelligence += other.intelligence;
        self.dexterity += other.dexterity;
        self.luck += other.luck;
        self.attack += other.attack;
        self.magic_attack += other.magic_attack;
        self.defense += other.defense;
        self.magic_defense += other.magic_defense;
        self.hit += other.hit;
        self.flee += other.flee;
        self.critical += other.critical;
        self.attack_speed += other.attack_speed;
    }

    /// Returns `true` if no bonus was evaluated.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BonusStat {
    Strength,
    Agility,
    Vitality,
    Intelligence,
    Dexterity,
    Luck,
    Attack,
    MagicAttack,
    Defense,
    MagicDefense,
    Hit,
    Flee,
    Critical,
    AttackSpeed,
}

impl BonusStat {
    fn from_token(token: &str) -> Option<Self> {
        Some(match token {
            "STR" => Self::Strength,
            "AGI" => Self::Agility,
            "VIT" => Self::Vitality,
            "INT" => Self::Intelligence,
            "DEX" => Self::Dexterity,
            "LUK" => Self::Luck,
            "ATK" => Self::Attack,
            "MATK" => Self::MagicAttack,
            "DEF" => Self::Defense,
            "MDEF" => Self::MagicDefense,
            "HIT" => Self::Hit,
            "FLEE" => Self::Flee,
            "CRI" | "CRIT" => Self::Critical,
            "ASPD" => Self::AttackSpeed,
            _ => return None,
        })
    }
}

/// Remove the color codes (`^rrggbb`) the client uses to colorize the item
/// description.
fn strip_color_codes(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();

    while let Some(character) = characters.next() {
        if character != '^' {
            result.push(character);
            continue;
        }

        let mut code_length = 0;

        while code_length < 6 {
            match characters.peek() {
                Some(next) if next.is_ascii_hexdigit() => {
                    characters.next();
                    code_length += 1;
                }
                _ => break,
            }
        }

        // A caret that is not followed by a color code is part of the text.
        if code_length < 6 {
            result.push('^');
        }
    }

    result
}

/// Returns the raw value of a labeled field in a description line, e.g. `공격`
/// in `계열 : 검    공격 : 25`.
fn labeled_value<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    let index = line.find(label)?;
    let rest = line[index + label.len()..].trim_start();
    let rest = rest.strip_prefix(':')?;

    Some(rest.trim())
}

/// Returns the number of a labeled field in a description line.
fn labeled_number(line: &str, label: &str) -> Option<i32> {
    let value = labeled_value(line, label)?;
    parse_leading_number(value)
}

fn parse_leading_number(value: &str) -> Option<i32> {
    let start = value.find(|character: char| character.is_ascii_digit() || character == '-')?;
    let number: String = value[start..]
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '-')
        .collect();

    number.parse().ok()
}

/// Lines that describe a conditional bonus. We don't want to count those as
/// always active stats.
fn is_conditional(line: &str) -> bool {
    const CONDITIONAL_MARKERS: &[&str] = &[
        "이상", "이하", "때", "동안", "경우", "제련", "정련", "착용", "확률", "마다", "초 ", "매 ", "레벨",
    ];

    CONDITIONAL_MARKERS.iter().any(|marker| line.contains(marker))
}

/// Scan a description line for flat stat bonuses like `STR + 2` or `HIT+10`.
fn parse_bonus_line(line: &str, bonuses: &mut StatBonuses) {
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if !bytes[index].is_ascii_alphabetic() {
            index += 1;
            continue;
        }

        let token_start = index;

        while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
            index += 1;
        }

        let token = &line[token_start..index];
        let stat = BonusStat::from_token(&token.to_ascii_uppercase());

        let mut cursor = index;

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        let sign = match bytes.get(cursor) {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => continue,
        };

        cursor += 1;

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        let number_start = cursor;

        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }

        if number_start == cursor {
            continue;
        }

        let amount = match line[number_start..cursor].parse::<i32>() {
            Ok(amount) => amount,
            Err(_) => continue,
        };

        let mut percent_cursor = cursor;

        while percent_cursor < bytes.len() && bytes[percent_cursor].is_ascii_whitespace() {
            percent_cursor += 1;
        }

        // Percentage bonuses don't stack linearly, so we skip them for now.
        if bytes.get(percent_cursor) == Some(&b'%') {
            index = percent_cursor + 1;
            continue;
        }

        if let Some(stat) = stat {
            bonuses.add(stat, amount * sign);
        }

        index = cursor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|line| line.to_string()).collect()
    }

    #[test]
    fn parses_weapon_description() {
        let stats = ItemStats::parse(&lines(&[
            "누구나 사용할 수 있을 것만 같은 검. 성직자만은 사용하지 못한다.",
            "계열 : ^777777검^000000      공격 : ^77777725^000000",
            "무게 : ^77777750^000000",
            "무기레벨 : ^7777771^000000",
            "요구레벨 : ^7777772^000000",
            "장착 : ^777777노비스계열/검사계열/상인계열/도둑계열 장착가능^000000",
        ]));

        assert_eq!(stats.attack, 25);
        assert_eq!(stats.defense, 0);
        assert_eq!(stats.weight, 50);
        assert_eq!(stats.weapon_level, 1);
        assert_eq!(stats.required_level, 2);
        assert!(stats.bonuses.is_empty());
    }

    #[test]
    fn parses_armor_description() {
        let stats = ItemStats::parse(&lines(&[
            "순면으로 되어 있는 옷으로 착용감이 상쾌하고 기분을 좋게 해준다",
            "계열 : ^777777갑옷^000000    방어 : ^77777710^000000",
            "무게 : ^77777710^000000",
            "장착 : ^777777전직업 장착가능^000000",
        ]));

        assert_eq!(stats.attack, 0);
        assert_eq!(stats.defense, 10);
        assert_eq!(stats.weight, 10);
        assert_eq!(stats.weapon_level, 0);
    }

    #[test]
    fn parses_required_level_without_value() {
        let stats = ItemStats::parse(&lines(&[
            "계열 : ^777777지팡이^000000  공격 : ^77777715^000000",
            "무게 : ^77777740^000000",
            "무기레벨 : ^7777771^000000",
            "요구레벨 : ^777777없음^000000",
        ]));

        assert_eq!(stats.required_level, 0);
    }

    #[test]
    fn parses_flat_bonus_lines() {
        let stats = ItemStats::parse(&lines(&[
            "STR +2",
            "DEX -1",
            "MATK + 30",
            "HIT+30",
            "STR + 5, AGI + 2",
            "ATK + 10, MATK + 10",
        ]));

        assert_eq!(stats.bonuses.strength, 7);
        assert_eq!(stats.bonuses.agility, 2);
        assert_eq!(stats.bonuses.dexterity, -1);
        assert_eq!(stats.bonuses.magic_attack, 40);
        assert_eq!(stats.bonuses.hit, 30);
        assert_eq!(stats.bonuses.attack, 10);
    }

    #[test]
    fn parses_mixed_case_bonus_lines() {
        let stats = ItemStats::parse(&lines(&["STR + 1, Dex + 1, Luk + 1"]));

        assert_eq!(stats.bonuses.strength, 1);
        assert_eq!(stats.bonuses.dexterity, 1);
        assert_eq!(stats.bonuses.luck, 1);
    }

    #[test]
    fn ignores_percentage_bonuses() {
        let stats = ItemStats::parse(&lines(&["MATK + 5%", "ASPD + 3 %"]));

        assert_eq!(stats.bonuses.magic_attack, 0);
        assert_eq!(stats.bonuses.attack_speed, 0);
    }

    #[test]
    fn ignores_conditional_bonus_lines() {
        let stats = ItemStats::parse(&lines(&["LUK 90이상일 때 ATK +20. DEX 90이상일 때 CRI +5.", "STR + 2"]));

        assert_eq!(stats.bonuses.attack, 0);
        assert_eq!(stats.bonuses.critical, 0);
        assert_eq!(stats.bonuses.strength, 2);
    }

    #[test]
    fn does_not_confuse_partial_stat_names() {
        let stats = ItemStats::parse(&lines(&["MATK + 10"]));

        assert_eq!(stats.bonuses.magic_attack, 10);
        assert_eq!(stats.bonuses.attack, 0);
    }

    #[test]
    fn strips_color_codes() {
        assert_eq!(strip_color_codes("^777777무게^000000 : ^77777710^000000"), "무게 : 10");
        assert_eq!(strip_color_codes("a ^ b"), "a ^ b");
    }
}
