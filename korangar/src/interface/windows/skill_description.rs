//! Window that shows the details of a single skill.
//!
//! Opened by right-clicking a skill in the skill tree, positioned at the mouse
//! pointer like the original client's skill description window. It shows the
//! learned / maximum level, the skill type, the SP cost, the range and whether
//! the skill can still be upgraded.
//!
//! The classic client uses `data\skilldesctable.txt` (or the description tables
//! in `skillinfolist.lub`) for the long description text, but the CondaRO data
//! set does not contain a readable skill description table, so this window only
//! shows the structured information that is known to the client (the same data
//! the skill tree and the hotbar use). See the report for details.

use korangar_interface::element::Element;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{AttackRange, SkillLevel, SkillType};

use crate::graphics::Color;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// Width of the skill description window.
const SKILL_DESCRIPTION_WIDTH: f32 = 300.0;

/// Human readable name of a skill type (same categories as the skill info).
fn skill_type_name(skill_type: SkillType) -> &'static str {
    match skill_type {
        SkillType::Passive => "Passive",
        SkillType::Attack => "Attack",
        SkillType::Ground => "Ground target",
        SkillType::SelfCast => "Self cast",
        SkillType::Support => "Support",
        SkillType::Trap => "Trap",
    }
}

/// Compact window with the details of a skill.
pub struct SkillDescriptionWindow {
    name: String,
    learned_level: Option<SkillLevel>,
    maximum_level: SkillLevel,
    skill_type: Option<SkillType>,
    spell_point_cost: Option<u16>,
    attack_range: Option<AttackRange>,
    upgradable: bool,
}

impl SkillDescriptionWindow {
    pub fn new(
        name: String,
        learned_level: Option<SkillLevel>,
        maximum_level: SkillLevel,
        skill_type: Option<SkillType>,
        spell_point_cost: Option<u16>,
        attack_range: Option<AttackRange>,
        upgradable: bool,
    ) -> Self {
        Self {
            name,
            learned_level,
            maximum_level,
            skill_type,
            spell_point_cost,
            attack_range,
            upgradable,
        }
    }
}

impl CustomWindow<ClientState> for SkillDescriptionWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::SkillDescription)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        fn row<A, B>(label: A, value: B) -> impl Element<ClientState> + 'static
        where
            A: rust_state::Selector<ClientState, String> + 'static,
            B: rust_state::Selector<ClientState, String> + 'static,
        {
            use korangar_interface::prelude::*;

            split! {
                gaps: theme().window().gaps(),
                children: (
                    text! {
                        text: label,
                        height: 18.0,
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    text! {
                        text: value,
                        height: 18.0,
                        horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                ),
            }
        }

        let level = match self.learned_level {
            Some(learned_level) => format!("{} / {}", learned_level.0, self.maximum_level.0),
            None => format!("not learned (max {})", self.maximum_level.0),
        };
        let skill_type = self.skill_type.map(skill_type_name).unwrap_or("—").to_owned();
        let spell_point_cost = self.spell_point_cost.map(|cost| cost.to_string()).unwrap_or_else(|| "—".to_owned());
        let attack_range = self.attack_range.map(|range| range.0.to_string()).unwrap_or_else(|| "—".to_owned());
        let upgradable = match self.upgradable {
            true => "Yes".to_owned(),
            false => "No".to_owned(),
        };

        window! {
            title: self.name,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: SKILL_DESCRIPTION_WIDTH,
            maximum_width: SKILL_DESCRIPTION_WIDTH,
            elements: (
                row("Level".to_owned(), level),
                row("Type".to_owned(), skill_type),
                row("SP cost".to_owned(), spell_point_cost),
                row("Range".to_owned(), attack_range),
                row("Upgradable".to_owned(), upgradable),
                text! {
                    text: "Tip: right-click any skill in the skill tree to inspect it.",
                    height: 18.0,
                    color: Color::rgb_u8(160, 160, 160),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
            ),
        }
    }
}
