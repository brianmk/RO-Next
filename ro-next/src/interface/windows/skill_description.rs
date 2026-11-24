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

use ro_next_interface::element::Element;
use ro_next_interface::window::{CustomWindow, Window};
use ragnarok_packets::{AttackRange, SkillLevel, SkillType};
use rust_state::PathExt;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Width of the skill description window.
const SKILL_DESCRIPTION_WIDTH: f32 = 300.0;

/// Selector for the localized "learned / maximum" level text.
fn level_text(learned_level: Option<SkillLevel>, maximum_level: SkillLevel) -> impl rust_state::Selector<ClientState, String> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| match learned_level {
        Some(learned_level) => format!("{} / {}", learned_level.0, maximum_level.0),
        None => client_state()
            .localization()
            .ui()
            .skill_not_learned_text()
            .follow_safe(state)
            .replace("{}", &maximum_level.0.to_string()),
    })
}

/// Selector for the localized name of the skill type.
fn skill_type_text(skill_type: Option<SkillType>) -> impl rust_state::Selector<ClientState, String> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        let text = client_state().localization().ui();
        match skill_type {
            Some(SkillType::Passive) => text.skill_type_passive().follow_safe(state).to_owned(),
            Some(SkillType::Attack) => text.skill_type_attack().follow_safe(state).to_owned(),
            Some(SkillType::Ground) => text.skill_type_ground().follow_safe(state).to_owned(),
            Some(SkillType::SelfCast) => text.skill_type_self_cast().follow_safe(state).to_owned(),
            Some(SkillType::Support) => text.skill_type_support().follow_safe(state).to_owned(),
            Some(SkillType::Trap) => text.skill_type_trap().follow_safe(state).to_owned(),
            None => "—".to_owned(),
        }
    })
}

/// Selector for the localized "Yes"/"No" upgradable state.
fn upgradable_text(upgradable: bool) -> impl rust_state::Selector<ClientState, String> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        let text = client_state().localization().ui();
        match upgradable {
            true => text.yes_text().follow_safe(state).clone(),
            false => text.no_text().follow_safe(state).clone(),
        }
    })
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
        use ro_next_interface::prelude::*;

        fn row<A, B>(label: A, value: B) -> impl Element<ClientState> + 'static
        where
            A: rust_state::Selector<ClientState, String> + 'static,
            B: rust_state::Selector<ClientState, String> + 'static,
        {
            use ro_next_interface::prelude::*;

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

        let level = level_text(self.learned_level, self.maximum_level);
        let skill_type = skill_type_text(self.skill_type);
        let spell_point_cost = self.spell_point_cost.map(|cost| cost.to_string()).unwrap_or_else(|| "—".to_owned());
        let attack_range = self.attack_range.map(|range| range.0.to_string()).unwrap_or_else(|| "—".to_owned());
        let upgradable = upgradable_text(self.upgradable);

        window! {
            title: self.name,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: SKILL_DESCRIPTION_WIDTH,
            maximum_width: SKILL_DESCRIPTION_WIDTH,
            elements: (
                row(client_state().localization().ui().skill_level_label(), level),
                row(client_state().localization().ui().skill_type_label(), skill_type),
                row(client_state().localization().ui().skill_sp_cost_label(), spell_point_cost),
                row(client_state().localization().ui().skill_range_label(), attack_range),
                row(client_state().localization().ui().skill_upgradable_label(), upgradable),
                text! {
                    text: client_state().localization().ui().skill_description_tip(),
                    height: 18.0,
                    color: client_theme().tokens().color().text_hint(),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
            ),
        }
    }
}
