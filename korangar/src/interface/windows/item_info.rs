//! Window that shows the details of an inventory item, including the raw item
//! description from the client item data.

use korangar_interface::element::Element;
use korangar_interface::window::{CustomWindow, Window};

use crate::graphics::Color;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::{ItemStats, StatBonuses};

/// Width of the item info window.
const ITEM_INFO_WIDTH: f32 = 420.0;
/// Minimum height of the item info window.
const ITEM_INFO_MINIMUM_HEIGHT: f32 = 240.0;
/// Maximum height of the item info window (the description scrolls if it is
/// longer than this).
const ITEM_INFO_MAXIMUM_HEIGHT: f32 = 520.0;

/// Format the flat item bonuses in a compact way.
fn format_bonuses(bonuses: &StatBonuses) -> String {
    let entries = [
        ("STR", bonuses.strength),
        ("AGI", bonuses.agility),
        ("VIT", bonuses.vitality),
        ("INT", bonuses.intelligence),
        ("DEX", bonuses.dexterity),
        ("LUK", bonuses.luck),
        ("ATK", bonuses.attack),
        ("MATK", bonuses.magic_attack),
        ("DEF", bonuses.defense),
        ("MDEF", bonuses.magic_defense),
        ("HIT", bonuses.hit),
        ("FLEE", bonuses.flee),
        ("CRIT", bonuses.critical),
        ("ASPD", bonuses.attack_speed),
    ];

    let parts: Vec<String> = entries
        .iter()
        .filter(|(_, value)| !matches!(value, 0))
        .map(|(name, value)| format!("{name}{value:+}"))
        .collect();

    match parts.is_empty() {
        true => "—".to_owned(),
        false => parts.join(" "),
    }
}

pub struct ItemInfoWindow {
    name: String,
    identified: bool,
    category: String,
    stats: ItemStats,
    description: String,
}

impl ItemInfoWindow {
    pub fn new(name: String, identified: bool, category: String, stats: ItemStats, description: String) -> Self {
        Self {
            name,
            identified,
            category,
            stats,
            description,
        }
    }
}

impl CustomWindow<ClientState> for ItemInfoWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ItemInfo)
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

        let category = self.category.clone();
        let identified = match self.identified {
            true => "Yes".to_owned(),
            false => "No".to_owned(),
        };
        let weight = self.stats.weight.to_string();
        let attack = self.stats.attack.to_string();
        let defense = self.stats.defense.to_string();
        let required_level = self.stats.required_level.to_string();
        let bonuses = format_bonuses(&self.stats.bonuses);
        let description = self.description;

        window! {
            title: self.name,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: ITEM_INFO_WIDTH,
            maximum_width: ITEM_INFO_WIDTH,
            minimum_height: ITEM_INFO_MINIMUM_HEIGHT,
            maximum_height: ITEM_INFO_MAXIMUM_HEIGHT,
            elements: (
                row("Category".to_owned(), category),
                row("Identified".to_owned(), identified),
                row("Weight".to_owned(), weight),
                row("Attack".to_owned(), attack),
                row("Defense".to_owned(), defense),
                row("Required level".to_owned(), required_level),
                row("Bonuses".to_owned(), bonuses),
                text! {
                    text: "Description",
                    height: 18.0,
                    color: Color::rgb_u8(255, 160, 60),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                scroll_view! {
                    scroll_bar: true,
                    children: text! {
                        text: description,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                },
            ),
        }
    }
}
