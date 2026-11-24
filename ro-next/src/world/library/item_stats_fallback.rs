//! Last-resort item stat parser for item ids that are **not** in the item
//! database data (`data\client\item_stats.ron`, see
//! [`super::item_stats`]).
//!
//! This module is deliberately *not* part of the default stat path. The
//! authoritative stats come from the server item database; this parser only
//! keeps custom/client-only items (an id the server item database does not
//! know, for example a leftover id of a newer client data set) from showing
//! nothing at all. Every use is logged once per item id
//! ([`super::item_stats::resolve`]), so a missing data row is visible in the
//! log instead of being silent.
//!
//! It parses the client `iteminfo.lub` description text. That text is only
//! display data, so the parser has to understand the localization it was
//! generated for: the Korean field labels (`계열`/`공격`/`방어`/`무게`/
//! `무기레벨`/`요구레벨`/`장착`) and their English equivalents, and the Korean
//! conditional-bonus markers used to skip conditional lines. The whole Korean
//! grammar dependency of the client lives in this one file; the default path
//! does not reference it.
//!
//! The parser is a first pass: it only understands simple, unconditional flat
//! values. Percentage bonuses and bonuses that only apply under a condition
//! (e.g. "LUK 90 or higher") are ignored on purpose. The authoritative values
//! for the derived character stats are the ones the server sends through the
//! status packets.

use super::item_stats::{ItemStats, StatBonuses};

/// Parse the stats of an item from its (already encoding-fixed) identified
/// description lines.
///
/// Only called for item ids that are missing from the item database data; see
/// the module documentation.
pub(super) fn parse(description: &[String]) -> ItemStats {
    let mut stats = ItemStats::DEFAULT;

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

        if let Some(required_level) = labeled_number(line, "요구레벨")
            .or_else(|| labeled_number(line, "Required Level"))
            .or_else(|| labeled_number(line, "Level Requirement"))
        {
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

fn add_bonus(bonuses: &mut StatBonuses, stat: BonusStat, amount: i32) {
    let value = match stat {
        BonusStat::Strength => &mut bonuses.strength,
        BonusStat::Agility => &mut bonuses.agility,
        BonusStat::Vitality => &mut bonuses.vitality,
        BonusStat::Intelligence => &mut bonuses.intelligence,
        BonusStat::Dexterity => &mut bonuses.dexterity,
        BonusStat::Luck => &mut bonuses.luck,
        BonusStat::Attack => &mut bonuses.attack,
        BonusStat::MagicAttack => &mut bonuses.magic_attack,
        BonusStat::Defense => &mut bonuses.defense,
        BonusStat::MagicDefense => &mut bonuses.magic_defense,
        BonusStat::Hit => &mut bonuses.hit,
        BonusStat::Flee => &mut bonuses.flee,
        BonusStat::Critical => &mut bonuses.critical,
        BonusStat::AttackSpeed => &mut bonuses.attack_speed,
    };

    *value += amount;
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
            add_bonus(bonuses, stat, amount * sign);
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
        let stats = parse(&lines(&[
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
        let stats = parse(&lines(&[
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
        let stats = parse(&lines(&[
            "계열 : ^777777지팡이^000000  공격 : ^77777715^000000",
            "무게 : ^77777740^000000",
            "무기레벨 : ^7777771^000000",
            "요구레벨 : ^777777없음^000000",
        ]));

        assert_eq!(stats.required_level, 0);
    }

    #[test]
    fn parses_english_labels() {
        // The English client tables use the English label names; the fallback
        // parser understands both so a custom item is not stat-less just
        // because the description is translated.
        let stats = parse(&lines(&[
            "A basic one-handed sword.",
            "Class:^6666CC One-Handed Sword^000000",
            "Attack:^CC0000 25^000000",
            "Weight:^009900 50^000000",
            "Weapon Level:^009900 1^000000",
            "Level Requirement:^009900 2^000000",
        ]));

        assert_eq!(stats.attack, 25);
        assert_eq!(stats.weight, 50);
        assert_eq!(stats.weapon_level, 1);
        assert_eq!(stats.required_level, 2);
    }

    #[test]
    fn parses_flat_bonus_lines() {
        let stats = parse(&lines(&[
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
        let stats = parse(&lines(&["STR + 1, Dex + 1, Luk + 1"]));

        assert_eq!(stats.bonuses.strength, 1);
        assert_eq!(stats.bonuses.dexterity, 1);
        assert_eq!(stats.bonuses.luck, 1);
    }

    #[test]
    fn ignores_percentage_bonuses() {
        let stats = parse(&lines(&["MATK + 5%", "ASPD + 3 %"]));

        assert_eq!(stats.bonuses.magic_attack, 0);
        assert_eq!(stats.bonuses.attack_speed, 0);
    }

    #[test]
    fn ignores_conditional_bonus_lines() {
        let stats = parse(&lines(&["LUK 90이상일 때 ATK +20. DEX 90이상일 때 CRI +5.", "STR + 2"]));

        assert_eq!(stats.bonuses.attack, 0);
        assert_eq!(stats.bonuses.critical, 0);
        assert_eq!(stats.bonuses.strength, 2);
    }

    #[test]
    fn does_not_confuse_partial_stat_names() {
        let stats = parse(&lines(&["MATK + 10"]));

        assert_eq!(stats.bonuses.magic_attack, 10);
        assert_eq!(stats.bonuses.attack, 0);
    }

    #[test]
    fn strips_color_codes() {
        assert_eq!(strip_color_codes("^777777무게^000000 : ^77777710^000000"), "무게 : 10");
        assert_eq!(strip_color_codes("a ^ b"), "a ^ b");
    }

    #[test]
    fn an_undescribed_item_is_zero() {
        assert_eq!(parse(&[]), ItemStats::DEFAULT);
        assert_eq!(parse(&["".to_owned()]), ItemStats::DEFAULT);
    }
}
