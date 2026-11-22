//! Window that shows the details of an inventory item, like the item display
//! window of the original client: the item artwork, the structured stats, the
//! four card slots and the full item description.
//!
//! The description text comes from the classic English description tables
//! (`idnum2itemdesctable.txt` / `num2itemdesctable.txt`, see
//! [`crate::world::ItemDescriptions`]); the card slot row is rendered by a
//! dedicated element that shows filled / empty / unavailable slots.

use std::sync::Arc;

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;

use crate::graphics::{Color, CornerDiameter, ShadowPadding, Texture};
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::theme::{InterfaceThemePathExt, InterfaceThemeType, ItemCardThemePathExt};
use crate::state::{ClientState, client_theme};
use crate::world::{CARD_SLOT_COUNT, CardNames, CardSlotState, ItemStats, ResourceMetadata, StatBonuses, random_option_lines};

/// Width of the item info window.
const ITEM_INFO_WIDTH: f32 = 500.0;
/// Minimum height of the item info window.
const ITEM_INFO_MINIMUM_HEIGHT: f32 = 320.0;
/// Maximum height of the item info window (the description scrolls if it is
/// longer than this).
const ITEM_INFO_MAXIMUM_HEIGHT: f32 = 560.0;

/// Height of the item artwork area.
const ARTWORK_HEIGHT: f32 = 140.0;
/// Aspect ratio (width / height) of the item collection artwork (75x100 BMP).
const ARTWORK_ASPECT: f32 = 0.75;

/// Height of the card slot row.
const CARD_SLOT_HEIGHT: f32 = 62.0;
/// Gap between two card slots.
const CARD_SLOT_GAP: f32 = 4.0;
/// Edge length of the card icon inside a filled slot.
const CARD_ICON_SIZE: f32 = 32.0;
/// Height reserved for the card name below the icon.
const CARD_NAME_HEIGHT: f32 = 16.0;

const SLOT_BACKGROUND_COLOR: Color = Color::rgba_u8(28, 28, 34, 140);
const SLOT_BORDER_COLOR: Color = Color::rgba_u8(120, 120, 130, 120);
const SLOT_INNER_COLOR: Color = Color::rgba_u8(0, 0, 0, 70);
const SLOT_UNAVAILABLE_BACKGROUND_COLOR: Color = Color::rgba_u8(86, 86, 92, 70);
const SLOT_UNAVAILABLE_BORDER_COLOR: Color = Color::rgba_u8(120, 120, 126, 60);
const SLOT_UNAVAILABLE_TEXT_COLOR: Color = Color::rgba_u8(160, 160, 166, 90);
const CARD_NAME_COLOR: Color = Color::rgba_u8(235, 235, 240, 220);

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
        _state: &rust_state::State<ClientState>,
        _store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        // The tooltip text is derived from the slot state; pre-compute it here
        // because `lay_out` only gets a shared reference to the element.
        for slot in 0..CARD_SLOT_COUNT {
            self.tooltips[slot] = match self.slot_state(slot) {
                CardSlotState::Filled => Some(self.card_names[slot].clone().unwrap_or_else(|| "Unknown card".to_owned())),
                CardSlotState::Empty => Some("Empty card slot".to_owned()),
                CardSlotState::Unavailable => Some("Unavailable card slot".to_owned()),
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
        _state: &'a rust_state::State<ClientState>,
        _store: ElementStore<'a>,
        slots: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (slot, area) in slots.iter().enumerate() {
            let slot_state = self.slot_state(slot);

            let (background_color, border_color) = match slot_state {
                CardSlotState::Filled => (SLOT_BACKGROUND_COLOR, SLOT_BORDER_COLOR),
                CardSlotState::Empty => (SLOT_BACKGROUND_COLOR, SLOT_BORDER_COLOR),
                CardSlotState::Unavailable => (SLOT_UNAVAILABLE_BACKGROUND_COLOR, SLOT_UNAVAILABLE_BORDER_COLOR),
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
                    let card_name = self.card_names[slot].as_deref().unwrap_or("Unknown card");

                    match self.card_icons.get(slot).and_then(Option::as_ref) {
                        Some(texture) => layout.add_texture(icon_area, texture.clone(), Color::WHITE, false),
                        None => {
                            // Unknown card id: keep the slot recognisable as a
                            // filled slot and show the resolved name.
                            layout.add_rectangle(
                                icon_area,
                                CornerDiameter::uniform(4.0),
                                SLOT_INNER_COLOR,
                                border_color,
                                ShadowPadding::uniform(1.0),
                            );
                            layout.add_text(
                                icon_area,
                                "?",
                                FontSize(14.0),
                                CARD_NAME_COLOR,
                                CARD_NAME_COLOR,
                                korangar_interface::layout::alignment::HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                                korangar_interface::layout::alignment::VerticalAlignment::Center { offset: 0.0 },
                                OverflowBehavior::Shrink,
                            );
                        }
                    }

                    layout.add_text(
                        name_area,
                        card_name,
                        FontSize(9.0),
                        CARD_NAME_COLOR,
                        CARD_NAME_COLOR,
                        korangar_interface::layout::alignment::HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                        korangar_interface::layout::alignment::VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );
                }
                CardSlotState::Empty => {
                    // Normal empty socket: a dark inset square with a border.
                    layout.add_rectangle(
                        icon_area,
                        CornerDiameter::uniform(4.0),
                        SLOT_INNER_COLOR,
                        border_color,
                        ShadowPadding::uniform(1.0),
                    );
                }
                CardSlotState::Unavailable => {
                    // Grayed out: this item does not have this slot.
                    layout.add_rectangle(
                        icon_area,
                        CornerDiameter::uniform(4.0),
                        SLOT_INNER_COLOR,
                        SLOT_UNAVAILABLE_BORDER_COLOR,
                        ShadowPadding::uniform(1.0),
                    );

                    layout.add_text(
                        icon_area,
                        "—",
                        FontSize(12.0),
                        SLOT_UNAVAILABLE_TEXT_COLOR,
                        SLOT_UNAVAILABLE_TEXT_COLOR,
                        korangar_interface::layout::alignment::HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                        korangar_interface::layout::alignment::VerticalAlignment::Center { offset: 0.0 },
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
}

impl ItemInfoWindow {
    pub fn new(
        item: &InventoryItem<ResourceMetadata>,
        category: String,
        artwork: Option<Arc<Texture>>,
        card_icons: [Option<Arc<Texture>>; CARD_SLOT_COUNT],
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
        let has_random_options = !self.random_options.is_empty();
        let random_options = self.random_options.join("\n");
        let artwork = ItemArtwork::new(self.artwork);
        let card_slots = ItemCardSlots::new(self.card_names, self.card_icons, self.slot_count);

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
                                row("Category".to_owned(), category),
                                row("Identified".to_owned(), identified),
                                row("Weight".to_owned(), weight),
                                row("Attack".to_owned(), attack),
                                row("Defense".to_owned(), defense),
                                row("Required level".to_owned(), required_level),
                                row("Bonuses".to_owned(), bonuses),
                            ),
                        },
                    ),
                },
                text! {
                    text: "Card slots",
                    height: 18.0,
                    color: Color::rgb_u8(255, 160, 60),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                card_slots,
                either! {
                    selector: ComputedSelector::new_default(move |_: &ClientState| has_random_options),
                    on_true: fragment! {
                        children: (
                            text! {
                                text: "Random options",
                                height: 18.0,
                                color: Color::rgb_u8(255, 160, 60),
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
