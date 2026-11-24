//! Window that shows the details of an inventory item, like the item display
//! window of the original client: the item artwork, the structured stats, the
//! four card slots and the full item description.
//!
//! The description text comes from the classic English description tables
//! (`idnum2itemdesctable.txt` / `num2itemdesctable.txt`, see
//! [`crate::world::ItemDescriptions`]); the card slot row is rendered by a
//! dedicated element that shows filled / empty / unavailable slots.

use std::sync::Arc;

use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::tooltip::TooltipExt;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window};
use ro_next_networking::{InventoryItem, InventoryItemDetails};
use rust_state::PathExt;

use crate::graphics::{Color, CornerDiameter, ShadowPadding, Texture};
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::renderer::LayoutExt;
use crate::state::localization::{LocalizationPathExt, QolTextsPathExt, UiTextsPathExt};
use crate::state::theme::{
    ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, ItemCardThemePathExt, TypographyTokensPathExt,
};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};
use crate::world::{CARD_SLOT_COUNT, CardNames, CardSlotState, ItemStats, ResourceMetadata, StatBonuses, random_option_lines};

/// Width of the item info window. The comparison pane sits next to the item
/// stats, so the window is wider than the plain item description.
const ITEM_INFO_WIDTH: f32 = 780.0;
/// Minimum height of the item info window.
const ITEM_INFO_MINIMUM_HEIGHT: f32 = 320.0;
/// Maximum height of the item info window (the description scrolls if it is
/// longer than this).
const ITEM_INFO_MAXIMUM_HEIGHT: f32 = 560.0;

/// Height of the item artwork area.
const ARTWORK_HEIGHT: f32 = 140.0;
/// Aspect ratio (width / height) of the item collection artwork (75x100 BMP).
const ARTWORK_ASPECT: f32 = 0.75;

/// Maximum height of the item comparison pane (a full 20-row comparison).
const ITEM_COMPARISON_HEIGHT: f32 = 360.0;
/// Minimum height of the item comparison pane.
const ITEM_COMPARISON_MINIMUM_HEIGHT: f32 = 180.0;
/// Height of one comparison row.
const ITEM_COMPARISON_ROW_HEIGHT: f32 = 15.0;

/// Height of the card slot row.
const CARD_SLOT_HEIGHT: f32 = 62.0;
/// Gap between two card slots.
const CARD_SLOT_GAP: f32 = 4.0;
/// Edge length of the card icon inside a filled slot.
const CARD_ICON_SIZE: f32 = 32.0;
/// Height reserved for the card name below the icon.
const CARD_NAME_HEIGHT: f32 = 16.0;

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

/// Selector for the localized "Yes"/"No" text of the identified state.
fn identified_text(identified: bool) -> impl rust_state::Selector<ClientState, String> {
    ro_next_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        let text = client_state().localization().ui();
        match identified {
            true => text.yes_text().follow_safe(state).clone(),
            false => text.no_text().follow_safe(state).clone(),
        }
    })
}

/// The large item artwork (collection image) or a placeholder frame.
struct ItemArtwork {
    texture: Option<Arc<Texture>>,
}

impl ItemArtwork {
    fn new(texture: Option<Arc<Texture>>) -> Self {
        Self { texture }
    }
}

impl Element<ClientState> for ItemArtwork {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        _state: &rust_state::State<ClientState>,
        _store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| resolver.with_height(ARTWORK_HEIGHT))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a rust_state::State<ClientState>,
        _store: ElementStore<'a>,
        area: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let item_card_theme = client_theme().item_card();

        layout.add_rectangle(
            *area,
            CornerDiameter::uniform(8.0),
            *state.get(&item_card_theme.background_color()),
            *state.get(&item_card_theme.border_color()),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        let Some(texture) = &self.texture else {
            return;
        };

        // Fit the 3:4 collection image into the artwork area without
        // distorting it.
        let height = area.height.min(ARTWORK_HEIGHT);
        let width = (height * ARTWORK_ASPECT).min(area.width);
        let height = width / ARTWORK_ASPECT;

        let artwork_area = Area {
            left: area.left + (area.width - width) / 2.0,
            top: area.top + (area.height - height) / 2.0,
            width,
            height,
        };

        layout.add_texture(artwork_area, texture.clone(), Color::WHITE, false);
    }
}

/// The row of four card slots of the item.
///
/// Filled slots show the card icon and name, available empty slots show an
/// empty socket and slots the item does not have are grayed out.
struct ItemCardSlots {
    card_names: CardNames,
    card_icons: [Option<Arc<Texture>>; CARD_SLOT_COUNT],
    slot_count: u8,
    /// Tooltip text per slot, refreshed from the layout state every frame.
    tooltips: [Option<String>; CARD_SLOT_COUNT],
}

impl ItemCardSlots {
    fn new(card_names: CardNames, card_icons: [Option<Arc<Texture>>; CARD_SLOT_COUNT], slot_count: u8) -> Self {
        Self {
            card_names,
            card_icons,
            slot_count,
            tooltips: std::array::from_fn(|_| None),
        }
    }

    fn slot_state(&self, slot: usize) -> CardSlotState {
        if self.card_names.get(slot).is_some_and(Option::is_some) {
            return CardSlotState::Filled;
        }

        match slot < usize::from(self.slot_count) {
            true => CardSlotState::Empty,
            false => CardSlotState::Unavailable,
        }
    }
}

/// Tooltip id of the card slot row.
struct ItemCardSlotsTooltip;

impl Element<ClientState> for ItemCardSlots {
    type LayoutInfo = [Area; CARD_SLOT_COUNT];

    fn create_layout_info(
        &mut self,
        state: &rust_state::State<ClientState>,
        _store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        // The tooltip text is derived from the slot state; pre-compute it here
        // because `lay_out` only gets a shared reference to the element.
        for slot in 0..CARD_SLOT_COUNT {
            self.tooltips[slot] = match self.slot_state(slot) {
                CardSlotState::Filled => Some(
                    self.card_names[slot]
                        .clone()
                        .unwrap_or_else(|| state.get(&client_state().localization().ui().item_unknown_card_text()).clone()),
                ),
                CardSlotState::Empty => Some(state.get(&client_state().localization().ui().item_empty_card_slot_text()).clone()),
                CardSlotState::Unavailable => Some(
                    state
                        .get(&client_state().localization().ui().item_unavailable_card_slot_text())
                        .clone(),
                ),
            };
        }

        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(CARD_SLOT_HEIGHT);
            let gaps = CARD_SLOT_GAP * (CARD_SLOT_COUNT - 1) as f32;
            let cell_width = ((area.width - gaps) / CARD_SLOT_COUNT as f32).max(1.0);

            std::array::from_fn(|slot| Area {
                left: area.left + slot as f32 * (cell_width + CARD_SLOT_GAP),
                top: area.top,
                width: cell_width,
                height: area.height,
            })
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a rust_state::State<ClientState>,
        _store: ElementStore<'a>,
        slots: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let typography = tokens.typography();
        let color_tokens = tokens.color();
        let unknown_card_font_size = *state.get(&typography.font_body_lg());
        let card_name_font_size = *state.get(&typography.font_xs());
        let unavailable_font_size = *state.get(&typography.font_body());
        let slot_background_color = *state.get(&color_tokens.item_slot_background());
        let slot_border_color = *state.get(&color_tokens.item_slot_border());
        let slot_inner_color = *state.get(&color_tokens.item_slot_inner());
        let slot_unavailable_background_color = *state.get(&color_tokens.item_slot_unavailable_background());
        let slot_unavailable_border_color = *state.get(&color_tokens.item_slot_unavailable_border());
        let slot_unavailable_text_color = *state.get(&color_tokens.item_slot_unavailable_text());
        let card_name_color = *state.get(&color_tokens.item_slot_card_name());

        for (slot, area) in slots.iter().enumerate() {
            let slot_state = self.slot_state(slot);

            let (background_color, border_color) = match slot_state {
                CardSlotState::Filled => (slot_background_color, slot_border_color),
                CardSlotState::Empty => (slot_background_color, slot_border_color),
                CardSlotState::Unavailable => (slot_unavailable_background_color, slot_unavailable_border_color),
            };

            layout.add_rectangle(
                *area,
                CornerDiameter::uniform(6.0),
                background_color,
                border_color,
                ShadowPadding::uniform(1.0),
            );

            let icon_size = CARD_ICON_SIZE.min(area.width - 8.0).min(area.height - CARD_NAME_HEIGHT - 6.0);
            let icon_area = Area {
                left: area.left + (area.width - icon_size) / 2.0,
                top: area.top + 3.0,
                width: icon_size,
                height: icon_size,
            };

            let name_area = Area {
                left: area.left + 2.0,
                top: area.top + icon_size + 4.0,
                width: area.width - 4.0,
                height: CARD_NAME_HEIGHT,
            };

            match slot_state {
                CardSlotState::Filled => {
                    // The tooltip was pre-composed in `create_layout_info`; it also
                    // carries the localized "Unknown card" fallback for slots whose
                    // card id is not in the item description table.
                    let card_name = self.tooltips[slot].as_deref().unwrap_or_default();

                    match self.card_icons.get(slot).and_then(Option::as_ref) {
                        Some(texture) => layout.add_texture(icon_area, texture.clone(), Color::WHITE, false),
                        None => {
                            // Unknown card id: keep the slot recognisable as a
                            // filled slot and show the resolved name.
                            layout.add_rectangle(
                                icon_area,
                                CornerDiameter::uniform(4.0),
                                slot_inner_color,
                                border_color,
                                ShadowPadding::uniform(1.0),
                            );
                            layout.add_text(
                                icon_area,
                                "?",
                                unknown_card_font_size,
                                card_name_color,
                                card_name_color,
                                ro_next_interface::layout::alignment::HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                                ro_next_interface::layout::alignment::VerticalAlignment::Center { offset: 0.0 },
                                OverflowBehavior::Shrink,
                            );
                        }
                    }

                    layout.add_text(
                        name_area,
                        card_name,
                        card_name_font_size,
                        card_name_color,
                        card_name_color,
                        ro_next_interface::layout::alignment::HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                        ro_next_interface::layout::alignment::VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                }
                CardSlotState::Empty => {
                    // Normal empty socket: a dark inset square with a border.
                    layout.add_rectangle(
                        icon_area,
                        CornerDiameter::uniform(4.0),
                        slot_inner_color,
                        border_color,
                        ShadowPadding::uniform(1.0),
                    );
                }
                CardSlotState::Unavailable => {
                    // Grayed out: this item does not have this slot.
                    layout.add_rectangle(
                        icon_area,
                        CornerDiameter::uniform(4.0),
                        slot_inner_color,
                        slot_unavailable_border_color,
                        ShadowPadding::uniform(1.0),
                    );

                    layout.add_text(
                        icon_area,
                        "—",
                        unavailable_font_size,
                        slot_unavailable_text_color,
                        slot_unavailable_text_color,
                        ro_next_interface::layout::alignment::HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                        ro_next_interface::layout::alignment::VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                }
            }

            if area.check().run(layout)
                && let Some(tooltip) = self.tooltips[slot].as_deref()
            {
                layout.add_tooltip(tooltip, ItemCardSlotsTooltip.tooltip_id());
            }
        }
    }
}

/// Inventory item currently equipped in a slot that the offered item can use.
///
/// Returns `None` for items that cannot be equipped and for the offered item
/// itself. Two accessories share the accessory positions, so the first
/// equipped item that intersects the offered positions is used.
pub(crate) fn equipped_item_in_slot<'a>(
    items: &'a [InventoryItem<ResourceMetadata>],
    offered: &InventoryItem<ResourceMetadata>,
) -> Option<&'a InventoryItem<ResourceMetadata>> {
    let offered_position = match &offered.details {
        InventoryItemDetails::Equippable { equip_position, .. } => *equip_position,
        InventoryItemDetails::Regular { .. } => return None,
    };

    if offered_position.is_empty() {
        return None;
    }

    items.iter().find(|candidate| {
        // The equipped item itself is its own comparison target (no delta);
        // any other item with the same index cannot exist.
        if candidate.index == offered.index && !offered.is_equipped() {
            return false;
        }

        let equipped_position = match &candidate.details {
            InventoryItemDetails::Regular { equipped_position, .. } => *equipped_position,
            InventoryItemDetails::Equippable { equipped_position, .. } => *equipped_position,
        };

        !equipped_position.is_empty() && equipped_position.intersects(offered_position)
    })
}

/// A stat shown in the comparison pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ComparisonStat {
    Attack,
    Defense,
    Weight,
    RequiredLevel,
    WeaponLevel,
    ArmorLevel,
    Strength,
    Agility,
    Vitality,
    Intelligence,
    Dexterity,
    Luck,
    BonusAttack,
    MagicAttack,
    BonusDefense,
    MagicDefense,
    Hit,
    Flee,
    Critical,
    AttackSpeed,
}

impl ComparisonStat {
    /// Localized label of the stat.
    fn label(self, state: &rust_state::State<ClientState>) -> String {
        let text = client_state().localization().ui();

        match self {
            ComparisonStat::Attack => state.get(&text.item_attack_label()).clone(),
            ComparisonStat::Defense => state.get(&text.item_defense_label()).clone(),
            ComparisonStat::Weight => state.get(&text.item_weight_label()).clone(),
            ComparisonStat::RequiredLevel => state.get(&text.item_required_level_label()).clone(),
            ComparisonStat::WeaponLevel => "Weapon level".to_owned(),
            ComparisonStat::ArmorLevel => "Armor level".to_owned(),
            ComparisonStat::Strength => "STR".to_owned(),
            ComparisonStat::Agility => "AGI".to_owned(),
            ComparisonStat::Vitality => "VIT".to_owned(),
            ComparisonStat::Intelligence => "INT".to_owned(),
            ComparisonStat::Dexterity => "DEX".to_owned(),
            ComparisonStat::Luck => "LUK".to_owned(),
            ComparisonStat::BonusAttack => "ATK bonus".to_owned(),
            ComparisonStat::MagicAttack => "MATK".to_owned(),
            ComparisonStat::BonusDefense => "DEF bonus".to_owned(),
            ComparisonStat::MagicDefense => "MDEF".to_owned(),
            ComparisonStat::Hit => "HIT".to_owned(),
            ComparisonStat::Flee => "FLEE".to_owned(),
            ComparisonStat::Critical => "CRIT".to_owned(),
            ComparisonStat::AttackSpeed => "ASPD".to_owned(),
        }
    }

    /// Whether a higher value is better for the player.
    fn higher_is_better(self) -> bool {
        !matches!(self, ComparisonStat::Weight | ComparisonStat::RequiredLevel)
    }
}

/// One row of the item comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct StatComparison {
    pub stat: ComparisonStat,
    pub equipped: i32,
    pub offered: i32,
}

/// Direction of a comparison delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Delta {
    Better,
    Worse,
    Same,
}

impl StatComparison {
    pub fn delta(self) -> i32 {
        self.offered - self.equipped
    }

    pub fn direction(self) -> Delta {
        let delta = self.delta();

        if delta == 0 {
            Delta::Same
        } else if self.stat.higher_is_better() == (delta > 0) {
            Delta::Better
        } else {
            Delta::Worse
        }
    }

    fn arrow(self) -> &'static str {
        match self.direction() {
            Delta::Better => "▲",
            Delta::Worse => "▼",
            Delta::Same => "=",
        }
    }
}

/// Builds the comparison rows of an offered item against the equipped item.
///
/// Rows that are irrelevant for both items (zero bonus stats and zero
/// weapon/armor level) are omitted so the pane stays focused on what actually
/// differs.
pub(crate) fn compare_stats(offered: &ItemStats, equipped: &ItemStats) -> Vec<StatComparison> {
    let mut rows = vec![
        StatComparison {
            stat: ComparisonStat::Attack,
            equipped: equipped.attack,
            offered: offered.attack,
        },
        StatComparison {
            stat: ComparisonStat::Defense,
            equipped: equipped.defense,
            offered: offered.defense,
        },
        StatComparison {
            stat: ComparisonStat::Weight,
            equipped: equipped.weight,
            offered: offered.weight,
        },
        StatComparison {
            stat: ComparisonStat::RequiredLevel,
            equipped: equipped.required_level as i32,
            offered: offered.required_level as i32,
        },
    ];

    let push_if_relevant = |rows: &mut Vec<StatComparison>, stat, equipped, offered| {
        if equipped != 0 || offered != 0 {
            rows.push(StatComparison { stat, equipped, offered });
        }
    };

    push_if_relevant(
        &mut rows,
        ComparisonStat::WeaponLevel,
        equipped.weapon_level as i32,
        offered.weapon_level as i32,
    );
    push_if_relevant(
        &mut rows,
        ComparisonStat::ArmorLevel,
        equipped.armor_level as i32,
        offered.armor_level as i32,
    );

    let equipped_bonus = equipped.bonuses;
    let offered_bonus = offered.bonuses;
    let bonus_rows = [
        (ComparisonStat::Strength, equipped_bonus.strength, offered_bonus.strength),
        (ComparisonStat::Agility, equipped_bonus.agility, offered_bonus.agility),
        (ComparisonStat::Vitality, equipped_bonus.vitality, offered_bonus.vitality),
        (
            ComparisonStat::Intelligence,
            equipped_bonus.intelligence,
            offered_bonus.intelligence,
        ),
        (ComparisonStat::Dexterity, equipped_bonus.dexterity, offered_bonus.dexterity),
        (ComparisonStat::Luck, equipped_bonus.luck, offered_bonus.luck),
        (ComparisonStat::BonusAttack, equipped_bonus.attack, offered_bonus.attack),
        (
            ComparisonStat::MagicAttack,
            equipped_bonus.magic_attack,
            offered_bonus.magic_attack,
        ),
        (ComparisonStat::BonusDefense, equipped_bonus.defense, offered_bonus.defense),
        (
            ComparisonStat::MagicDefense,
            equipped_bonus.magic_defense,
            offered_bonus.magic_defense,
        ),
        (ComparisonStat::Hit, equipped_bonus.hit, offered_bonus.hit),
        (ComparisonStat::Flee, equipped_bonus.flee, offered_bonus.flee),
        (ComparisonStat::Critical, equipped_bonus.critical, offered_bonus.critical),
        (
            ComparisonStat::AttackSpeed,
            equipped_bonus.attack_speed,
            offered_bonus.attack_speed,
        ),
    ];

    for (stat, equipped, offered) in bonus_rows {
        push_if_relevant(&mut rows, stat, equipped, offered);
    }

    rows
}

/// The item currently equipped in the same slot, plus its stats.
#[derive(Clone, Debug)]
pub(crate) struct EquippedComparison {
    pub name: String,
    pub stats: ItemStats,
}

/// One pre-composed line of the comparison pane.
struct ComparisonLine {
    label: String,
    value: String,
    delta: Delta,
}

/// The second pane of the item info window: the equipped item of the same slot
/// and the stat deltas of the offered item.
struct ItemComparisonPane {
    equipped: Option<EquippedComparison>,
    rows: Vec<StatComparison>,
    title: String,
    equipped_label: String,
    name_line: String,
    empty_text: String,
    lines: Vec<ComparisonLine>,
}

impl ItemComparisonPane {
    fn new(equipped: Option<EquippedComparison>, rows: Vec<StatComparison>) -> Self {
        Self {
            equipped,
            rows,
            title: String::new(),
            equipped_label: String::new(),
            name_line: String::new(),
            empty_text: String::new(),
            lines: Vec::new(),
        }
    }
}

impl Element<ClientState> for ItemComparisonPane {
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &rust_state::State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        // Prepare the strings here: `WindowLayout::add_text` borrows its text
        // for the lifetime of the layout, so it cannot use temporaries created
        // inside `lay_out`.
        let text = client_state().localization().qol();
        self.title = state.get(&text.item_comparison_label()).clone();
        self.equipped_label = state.get(&text.item_equipped_label()).clone();
        self.name_line = self
            .equipped
            .as_ref()
            .map(|equipped| format!("{}: {}", self.equipped_label, equipped.name))
            .unwrap_or_default();
        self.empty_text = state.get(&text.item_comparison_nothing_text()).clone();
        self.lines = self
            .rows
            .iter()
            .map(|row| ComparisonLine {
                label: row.stat.label(state),
                value: format!("{} → {} {}", row.equipped, row.offered, row.arrow()),
                delta: row.direction(),
            })
            .collect();

        let height =
            (60.0 + self.lines.len() as f32 * ITEM_COMPARISON_ROW_HEIGHT).clamp(ITEM_COMPARISON_MINIMUM_HEIGHT, ITEM_COMPARISON_HEIGHT);

        with_single_resolver(resolvers, |resolver| resolver.with_height(height))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a rust_state::State<ClientState>,
        _: ElementStore<'a>,
        area: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let title_font_size = *state.get(&typography.font_body());
        let name_font_size = *state.get(&typography.font_sm());
        let row_font_size = *state.get(&typography.font_sm());
        let accent_color = *state.get(&color_tokens.text_accent());
        let primary_color = *state.get(&color_tokens.text_primary());
        let secondary_color = *state.get(&color_tokens.text_secondary());
        let divider_color = *state.get(&color_tokens.divider());
        let panel_color = *state.get(&color_tokens.surface_panel());
        let success_color = *state.get(&color_tokens.state_success());
        let danger_color = *state.get(&color_tokens.state_danger());

        layout.add_rectangle(
            *area,
            CornerDiameter::uniform(14.0),
            panel_color,
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        let title_area = Area {
            left: area.left + 8.0,
            top: area.top + 8.0,
            width: area.width - 16.0,
            height: 18.0,
        };

        layout.add_text(
            title_area,
            &self.title,
            title_font_size,
            accent_color,
            accent_color,
            HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        let divider_area = Area {
            left: area.left + 8.0,
            top: area.top + 28.0,
            width: area.width - 16.0,
            height: 1.0,
        };

        layout.add_rectangle(
            divider_area,
            CornerDiameter::uniform(0.5),
            divider_color,
            Color::TRANSPARENT,
            ShadowPadding::uniform(0.0),
        );

        if self.equipped.is_none() {
            layout.add_text(
                Area {
                    left: area.left + 8.0,
                    top: area.top + 40.0,
                    width: area.width - 16.0,
                    height: 36.0,
                },
                &self.empty_text,
                name_font_size,
                secondary_color,
                secondary_color,
                HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );
            return;
        };

        let name_area = Area {
            left: area.left + 8.0,
            top: area.top + 38.0,
            width: area.width - 16.0,
            height: 18.0,
        };

        layout.add_text(
            name_area,
            &self.name_line,
            name_font_size,
            primary_color,
            primary_color,
            HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        let mut top = area.top + 60.0;

        for line in &self.lines {
            let row_area = Area {
                left: area.left + 8.0,
                top,
                width: area.width - 16.0,
                height: ITEM_COMPARISON_ROW_HEIGHT,
            };

            let value_color = match line.delta {
                Delta::Better => success_color,
                Delta::Worse => danger_color,
                Delta::Same => secondary_color,
            };

            layout.add_text(
                row_area,
                &line.label,
                row_font_size,
                secondary_color,
                secondary_color,
                HorizontalAlignment::Left { offset: 2.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            layout.add_text(
                row_area,
                &line.value,
                row_font_size,
                value_color,
                value_color,
                HorizontalAlignment::Right { offset: 2.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            top += ITEM_COMPARISON_ROW_HEIGHT;
        }
    }
}

pub struct ItemInfoWindow {
    name: String,
    identified: bool,
    category: String,
    stats: ItemStats,
    description: String,
    random_options: Vec<String>,
    artwork: Option<Arc<Texture>>,
    card_names: CardNames,
    card_icons: [Option<Arc<Texture>>; CARD_SLOT_COUNT],
    slot_count: u8,
    comparison: Option<EquippedComparison>,
}

impl ItemInfoWindow {
    pub fn new(
        item: &InventoryItem<ResourceMetadata>,
        category: String,
        artwork: Option<Arc<Texture>>,
        card_icons: [Option<Arc<Texture>>; CARD_SLOT_COUNT],
        equipped: Option<&InventoryItem<ResourceMetadata>>,
    ) -> Self {
        Self {
            name: item.metadata.name.clone(),
            identified: item.is_identified(),
            category,
            stats: item.metadata.stats,
            description: item.metadata.description.join("\n"),
            random_options: random_option_lines(&item.details),
            artwork,
            card_names: item.metadata.card_names.clone(),
            card_icons,
            slot_count: item.metadata.slot_count,
            comparison: equipped.map(|equipped| EquippedComparison {
                name: equipped.metadata.name.clone(),
                stats: equipped.metadata.stats,
            }),
        }
    }
}

impl CustomWindow<ClientState> for ItemInfoWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ItemInfo)
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

        let category = self.category.clone();
        let identified = identified_text(self.identified);
        let weight = self.stats.weight.to_string();
        let attack = self.stats.attack.to_string();
        let defense = self.stats.defense.to_string();
        let required_level = self.stats.required_level.to_string();
        let bonuses = format_bonuses(&self.stats.bonuses);
        let description = self.description;
        let has_random_options = !self.random_options.is_empty();
        let random_options = self.random_options.join("\n");
        let artwork = ItemArtwork::new(self.artwork);
        let card_slots = ItemCardSlots::new(self.card_names, self.card_icons, self.slot_count);
        let comparison_rows = self
            .comparison
            .as_ref()
            .map(|equipped| compare_stats(&self.stats, &equipped.stats))
            .unwrap_or_default();
        let comparison = ItemComparisonPane::new(self.comparison, comparison_rows);

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
                split! {
                    gaps: 8.0,
                    children: (
                        artwork,
                        fragment! {
                            children: (
                                row(client_state().localization().ui().item_category_label(), category),
                                row(client_state().localization().ui().item_identified_label(), identified),
                                row(client_state().localization().ui().item_weight_label(), weight),
                                row(client_state().localization().ui().item_attack_label(), attack),
                                row(client_state().localization().ui().item_defense_label(), defense),
                                row(client_state().localization().ui().item_required_level_label(), required_level),
                                row(client_state().localization().ui().item_bonuses_label(), bonuses),
                            ),
                        },
                        comparison,
                    ),
                },
                text! {
                    text: client_state().localization().ui().item_card_slots_label(),
                    height: 18.0,
                    color: client_theme().tokens().color().text_accent(),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                card_slots,
                either! {
                    selector: ComputedSelector::<_, bool>::new_default(move |_: &ClientState| has_random_options),
                    on_true: fragment! {
                        children: (
                            text! {
                                text: client_state().localization().ui().item_random_options_label(),
                                height: 18.0,
                                color: client_theme().tokens().color().text_accent(),
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            text! {
                                text: random_options,
                                overflow_behavior: OverflowBehavior::LineBreak,
                            },
                        ),
                    },
                    on_false: EmptyElement,
                },
                text! {
                    text: client_state().localization().ui().item_description_label(),
                    height: 18.0,
                    color: client_theme().tokens().color().text_accent(),
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

#[cfg(test)]
mod tests {
    use ragnarok_packets::{EquipPosition, EquippableItemFlags, InventoryIndex, ItemId, ItemOptions, RegularItemFlags};
    use ro_next_networking::{InventoryItem, InventoryItemDetails};

    use super::{ComparisonStat, Delta, StatComparison, compare_stats, equipped_item_in_slot};
    use crate::world::{ItemStats, ResourceMetadata};

    fn item(index: u16, equip_position: EquipPosition, equipped_position: EquipPosition) -> InventoryItem<ResourceMetadata> {
        InventoryItem {
            metadata: ResourceMetadata::default(),
            index: InventoryIndex(index),
            item_id: ItemId(500 + index as u32),
            item_type: 0,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Equippable {
                equip_position,
                equipped_position,
                bind_on_equip_type: 0,
                w_item_sprite_number: 0,
                option_count: 0,
                option_data: std::array::from_fn(|_| ItemOptions {
                    index: 0,
                    value: 0,
                    parameter: 0,
                }),
                refinement_level: 0,
                enchantment_level: 0,
                flags: EquippableItemFlags::empty(),
            },
        }
    }

    #[test]
    fn equipped_item_in_the_same_slot_is_found() {
        let offered = item(1, EquipPosition::RIGHT_HAND, EquipPosition::empty());
        let equipped = item(2, EquipPosition::RIGHT_HAND, EquipPosition::RIGHT_HAND);
        let elsewhere = item(3, EquipPosition::ARMOR, EquipPosition::ARMOR);
        let items = vec![offered.clone(), equipped.clone(), elsewhere];

        let found = equipped_item_in_slot(&items, &offered).expect("the equipped weapon must be found");
        assert_eq!(found.index, InventoryIndex(2));
    }

    #[test]
    fn an_equipped_item_compares_with_itself_but_an_unequipped_one_does_not() {
        // Viewing the item that is already equipped shows its own (empty)
        // delta, like the official comparison window.
        let equipped_item = item(1, EquipPosition::ARMOR, EquipPosition::ARMOR);
        let items = vec![equipped_item.clone()];
        let found = equipped_item_in_slot(&items, &equipped_item).expect("an equipped item compares with itself");
        assert_eq!(found.index, InventoryIndex(1));

        // Viewing an unequipped item ignores the item with the same index.
        let offered = item(1, EquipPosition::ARMOR, EquipPosition::empty());
        assert!(equipped_item_in_slot(&items, &offered).is_none());
    }

    #[test]
    fn non_equippable_items_have_no_comparison() {
        let offered = InventoryItem {
            metadata: ResourceMetadata::default(),
            index: InventoryIndex(1),
            item_id: ItemId(501),
            item_type: 0,
            slot: [0; 4],
            hire_expiration_date: 0,
            details: InventoryItemDetails::Regular {
                amount: 1,
                equipped_position: EquipPosition::empty(),
                flags: RegularItemFlags::empty(),
            },
        };

        assert!(equipped_item_in_slot(&[offered.clone()], &offered).is_none());
    }

    #[test]
    fn comparison_direction_uses_the_stat_preference() {
        let higher_is_better = StatComparison {
            stat: ComparisonStat::Attack,
            equipped: 10,
            offered: 15,
        };
        assert_eq!(higher_is_better.delta(), 5);
        assert_eq!(higher_is_better.direction(), Delta::Better);
        assert_eq!(higher_is_better.arrow(), "▲");

        let same = StatComparison {
            stat: ComparisonStat::Attack,
            equipped: 10,
            offered: 10,
        };
        assert_eq!(same.direction(), Delta::Same);

        // More weight and a higher required level are worse.
        let heavier = StatComparison {
            stat: ComparisonStat::Weight,
            equipped: 10,
            offered: 15,
        };
        assert_eq!(heavier.direction(), Delta::Worse);
        assert_eq!(heavier.arrow(), "▼");

        let lower_requirement = StatComparison {
            stat: ComparisonStat::RequiredLevel,
            equipped: 20,
            offered: 10,
        };
        assert_eq!(lower_requirement.direction(), Delta::Better);
    }

    #[test]
    fn comparison_skips_irrelevant_bonus_rows() {
        let mut equipped = ItemStats::DEFAULT;
        let mut offered = ItemStats::DEFAULT;

        equipped.attack = 20;
        offered.attack = 30;
        offered.bonuses.strength = 2;

        let rows = compare_stats(&offered, &equipped);

        assert!(
            rows.iter()
                .any(|row| row.stat == ComparisonStat::Attack && row.direction() == Delta::Better)
        );
        assert!(rows.iter().any(|row| row.stat == ComparisonStat::Strength));
        // No magic attack bonus on either item, so no empty row for it.
        assert!(!rows.iter().any(|row| row.stat == ComparisonStat::MagicAttack));
    }
}
