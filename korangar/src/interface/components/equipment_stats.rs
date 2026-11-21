//! Panel showing the stats of the currently equipped items.
//!
//! The panel is split into two sections:
//!
//! - The *equipment* section is calculated entirely on the client from the
//!   equipped items and their item description ([`crate::world::ItemStats`]).
//!   This section works without any server support.
//! - The *character* section shows the combat stats the server sends for the
//!   character ([`crate::world::Player`]). Those values are calculated by the
//!   server and therefore include everything the equipped items do.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::theme::{CharacterOverviewThemePathExt, InterfaceThemePathExt};
use crate::state::{ClientState, client_theme};
use crate::world::{Player, ResourceMetadata, StatBonuses};

/// Height of the panel. Matches the height of the paper doll so that both
/// columns of the equipment window line up.
const PANEL_HEIGHT: f32 = 342.0;

const PANEL_BACKGROUND_COLOR: Color = Color::rgba_u8(0, 0, 0, 40);
const HEADER_COLOR: Color = Color::rgb_u8(255, 160, 60);
const SEPARATOR_COLOR: Color = Color::rgba_u8(255, 255, 255, 24);

const HEADER_HEIGHT: f32 = 26.0;
const ROW_HEIGHT: f32 = 17.0;
const SECTION_GAP: f32 = 12.0;

fn value_or_dash(value: usize) -> String {
    match value {
        0 => "—".to_string(),
        value => value.to_string(),
    }
}

fn number_or_dash(value: i32) -> String {
    match value {
        value if value <= 0 => "—".to_string(),
        value => value.to_string(),
    }
}

/// Format the flat item bonuses of the equipped items in a compact way.
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

    if parts.is_empty() {
        return "—".to_string();
    }

    let mut text = parts.join(" ");

    if text.chars().count() > 30 {
        text = text.chars().take(29).collect();
        text.push('…');
    }

    text
}

enum Row {
    Header(&'static str),
    Value { label: &'static str, value: String },
    Gap,
}

pub struct EquipmentStats<A, B> {
    items_path: A,
    player_path: B,
    rows: Vec<Row>,
}

impl<A, B> EquipmentStats<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, Player>,
{
    pub fn new(items_path: A, player_path: B) -> Self {
        Self {
            items_path,
            player_path,
            rows: Vec::new(),
        }
    }

    fn update_rows(&mut self, state: &State<ClientState>) {
        self.rows.clear();

        let mut weapon_attack = 0;
        let mut armor_defense = 0;
        let mut equipped_weight = 0;
        let mut refined_items = 0;
        let mut refine_levels = 0;
        let mut bonuses = StatBonuses::default();

        for item in state.follow(self.items_path) {
            let InventoryItemDetails::Equippable {
                equipped_position,
                refinement_level,
                ..
            } = &item.details
            else {
                continue;
            };

            if equipped_position.is_empty() {
                continue;
            }

            let stats = item.metadata.stats;
            weapon_attack += stats.attack;
            armor_defense += stats.defense;
            equipped_weight += stats.weight;
            bonuses.merge(&stats.bonuses);

            if *refinement_level > 0 {
                refined_items += 1;
                refine_levels += *refinement_level as i32;
            }
        }

        let player = state.follow(self.player_path);

        self.rows.push(Row::Header("EQUIPMENT"));
        self.rows.push(Row::Value {
            label: "Weapon attack",
            value: number_or_dash(weapon_attack),
        });
        self.rows.push(Row::Value {
            label: "Armor defense",
            value: number_or_dash(armor_defense),
        });
        self.rows.push(Row::Value {
            label: "Refine",
            value: match refined_items {
                0 => "—".to_string(),
                1 => format!("+{refine_levels} (1 item)"),
                _ => format!("+{refine_levels} ({refined_items} items)"),
            },
        });
        self.rows.push(Row::Value {
            label: "Equipped weight",
            value: number_or_dash(equipped_weight),
        });
        self.rows.push(Row::Value {
            label: "Item bonuses",
            value: format_bonuses(&bonuses),
        });

        self.rows.push(Row::Gap);

        self.rows.push(Row::Header("CHARACTER"));
        self.rows.push(Row::Value {
            label: "Attack",
            value: match player.attack {
                0 => "—".to_string(),
                attack => format!("{attack} + {}", player.bonus_attack),
            },
        });
        self.rows.push(Row::Value {
            label: "Magic attack",
            value: match player.magic_attack_min + player.magic_attack_max {
                0 => "—".to_string(),
                _ => format!("{} ~ {}", player.magic_attack_min, player.magic_attack_max),
            },
        });
        self.rows.push(Row::Value {
            label: "Defense",
            value: match player.defense {
                0 => "—".to_string(),
                defense => format!("{defense} + {}", player.bonus_defense),
            },
        });
        self.rows.push(Row::Value {
            label: "Magic defense",
            value: match player.magic_defense {
                0 => "—".to_string(),
                magic_defense => format!("{magic_defense} + {}", player.bonus_magic_defense),
            },
        });
        self.rows.push(Row::Value {
            label: "Hit",
            value: value_or_dash(player.hit as usize),
        });
        self.rows.push(Row::Value {
            label: "Flee",
            value: match player.flee {
                0 => "—".to_string(),
                flee => match player.perfect_dodge {
                    0 => flee.to_string(),
                    perfect_dodge => format!("{flee} (+{perfect_dodge})"),
                },
            },
        });
        self.rows.push(Row::Value {
            label: "Critical",
            value: value_or_dash(player.critical as usize),
        });
        self.rows.push(Row::Value {
            label: "Attack speed",
            value: match player.attack_speed {
                // The server sends the attack motion in milliseconds. The
                // client displays it as `200 - motion / 10`.
                0 => "—".to_string(),
                attack_speed => format!("{:.1}", 200.0 - attack_speed as f32 / 10.0),
            },
        });
        self.rows.push(Row::Value {
            label: "Weight",
            value: match player.maximum_weight {
                0 => value_or_dash(player.weight as usize),
                maximum_weight => format!("{} / {}", player.weight, maximum_weight),
            },
        });
    }
}

impl<A, B> Element<ClientState> for EquipmentStats<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, Player>,
{
    type LayoutInfo = Area;

    fn create_layout_info(&mut self, state: &State<ClientState>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) -> Area {
        self.update_rows(state);

        with_single_resolver(resolvers, |resolver| resolver.with_height(PANEL_HEIGHT))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Area,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        use korangar_interface::prelude::*;

        let area = *layout_info;
        let horizontal_alignment = *state.get(&theme().text().horizontal_alignment());

        layout.add_rectangle(
            area,
            CornerDiameter::uniform(14.0),
            PANEL_BACKGROUND_COLOR,
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        let mut top = area.top + 6.0;

        for row in &self.rows {
            match row {
                Row::Header(title) => {
                    let row_area = Area {
                        left: area.left + 4.0,
                        top,
                        width: area.width - 8.0,
                        height: HEADER_HEIGHT - 8.0,
                    };

                    layout.add_text(
                        row_area,
                        title,
                        FontSize(12.0),
                        HEADER_COLOR,
                        HEADER_COLOR,
                        HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    let separator_area = Area {
                        left: area.left + 4.0,
                        top: top + HEADER_HEIGHT - 10.0,
                        width: area.width - 8.0,
                        height: 1.0,
                    };

                    layout.add_rectangle(
                        separator_area,
                        CornerDiameter::uniform(0.5),
                        SEPARATOR_COLOR,
                        Color::TRANSPARENT,
                        ShadowPadding::uniform(0.0),
                    );

                    top += HEADER_HEIGHT;
                }
                Row::Value { label, value } => {
                    let row_area = Area {
                        left: area.left + 4.0,
                        top,
                        width: area.width - 8.0,
                        height: ROW_HEIGHT,
                    };

                    // Use the theme's text colors so the stats stay readable on
                    // both the dark default skin and the light classic skin.
                    let label_color = *state.get(&client_theme().character_overview().label_color());
                    let value_color = *state.get(&client_theme().character_overview().value_color());

                    layout.add_text(
                        row_area,
                        label,
                        FontSize(13.0),
                        label_color,
                        label_color,
                        horizontal_alignment,
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    layout.add_text(
                        row_area,
                        value,
                        FontSize(13.0),
                        value_color,
                        value_color,
                        HorizontalAlignment::Right { offset: 2.0, border: 0.0 },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    top += ROW_HEIGHT;
                }
                Row::Gap => top += SECTION_GAP,
            }
        }
    }
}
