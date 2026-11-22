use std::sync::Arc;

use korangar_interface::MouseMode;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::resource::ItemSource;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::renderer::LayoutExt;
use crate::state::theme::{InterfaceThemePathExt, ItemCardThemePathExt};
use crate::state::{ClientState, client_theme};
use crate::world::ResourceMetadata;

/// Marker color of the favorite star of an item card.
const FAVORITE_COLOR: Color = Color::rgb_u8(255, 200, 60);

#[derive(Default)]
struct AmountDisplay {
    amount: u16,
    string: Option<String>,
}

impl AmountDisplay {
    fn update(&mut self, new_amount: u16) {
        if self.string.is_none() || self.amount != new_amount {
            self.string = Some(new_amount.to_string());
            self.amount = new_amount;
        }
    }
}

/// What the right click on an item box should do.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemBoxContextMenu {
    /// The right click does nothing (default for item boxes that are not part
    /// of the player inventory, e.g. shop or cart lists).
    #[default]
    None,
    /// Open the inventory item context menu (use/equip, view info, ...).
    Inventory,
    /// Open the storage item context menu (take out, view info).
    Storage,
}

struct ItemContextMenuHandler<P> {
    item_path: P,
    context_menu: ItemBoxContextMenu,
}

impl<P> ClickHandler<ClientState> for ItemContextMenuHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // item.
        let item = state.try_get(&self.item_path).unwrap();

        match self.context_menu {
            ItemBoxContextMenu::Inventory => queue.queue(InputEvent::OpenInventoryItemMenu {
                inventory_index: item.index,
            }),
            ItemBoxContextMenu::Storage => queue.queue(InputEvent::OpenStorageItemMenu { storage_index: item.index }),
            ItemBoxContextMenu::None => {}
        }
    }
}

struct ItemBoxHandler<P> {
    item_path: P,
    source: ItemSource,
}

impl<P> ItemBoxHandler<P> {
    fn new(item_path: P, source: ItemSource) -> Self {
        Self { item_path, source }
    }
}

impl<P> ClickHandler<ClientState> for ItemBoxHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // item.
        let item = state.try_get(&self.item_path).unwrap().clone();

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Custom {
                mode: MouseInputMode::MoveItem { item, source: self.source },
            },
        });
    }
}

impl<P> DropHandler<ClientState> for ItemBoxHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_drop(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>, mouse_mode: &MouseMode<ClientState>) {
        if let MouseMode::Custom {
            mode: MouseInputMode::MoveItem { source, item },
        } = mouse_mode
        {
            queue.queue(InputEvent::MoveItem {
                source: *source,
                destination: self.source,
                item: item.clone(),
            });
        }
    }
}

/// Forwards double clicks on an item to the client (used to store items in the
/// Kafra storage or take them out when the storage window is open).
struct ItemDoubleClickHandler<P> {
    item_path: P,
    source: ItemSource,
}

impl<P> ClickHandler<ClientState> for ItemDoubleClickHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // item.
        let item = state.try_get(&self.item_path).unwrap().clone();

        // The first click of the double click picked the item up; cancel that drag
        // state before the double-click action runs.
        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Default,
        });

        // The client decides whether this is a storage move (storage window open)
        // or a use/equip action.
        queue.queue(InputEvent::ItemDoubleClick { source: self.source, item });
    }
}

pub struct ItemBox<A> {
    item_path: A,
    handler: ItemBoxHandler<A>,
    context_menu_handler: ItemContextMenuHandler<A>,
    double_click_handler: ItemDoubleClickHandler<A>,
    context_menu: ItemBoxContextMenu,
    amount_display: AmountDisplay,
    /// Cached tooltip text of the item in this box (composed name, card slots,
    /// options, favorite). Cloned from the pre-composed item metadata, so no
    /// tooltip string is built per frame.
    tooltip: Option<Arc<str>>,
}

/// Tooltip id of the item box tooltip.
struct ItemBoxTooltip;

impl<A> ItemBox<A>
where
    A: Copy,
{
    /// This function is supposed to be called from a component macro
    /// and not intended to be called manually.
    #[inline(always)]
    pub fn component_new(item_path: A, source: ItemSource) -> Self {
        Self {
            item_path,
            handler: ItemBoxHandler::new(item_path, source),
            context_menu_handler: ItemContextMenuHandler {
                item_path,
                context_menu: ItemBoxContextMenu::None,
            },
            double_click_handler: ItemDoubleClickHandler { item_path, source },
            context_menu: ItemBoxContextMenu::None,
            amount_display: AmountDisplay::default(),
            tooltip: None,
        }
    }

    /// Enable the right click context menu on this item box.
    pub fn with_context_menu(mut self, context_menu: ItemBoxContextMenu) -> Self {
        self.context_menu = context_menu;
        self.context_menu_handler.context_menu = context_menu;
        self
    }
}

impl<A> Element<ClientState> for ItemBox<A>
where
    A: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    type LayoutInfo = BaseLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(40.0);

            self.tooltip = state.try_get(&self.item_path).map(|item| item.metadata.tooltip_text.clone());

            if let Some(item) = state.try_get(&self.item_path)
                && item.metadata.texture.as_ref().is_some()
                && let InventoryItemDetails::Regular { amount, .. } = &item.details
            {
                self.amount_display.update(*amount);
            }

            Self::LayoutInfo { area }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        // The card colors come from the interface theme so that the classic
        // (light) theme gets white cards while the default theme stays dark.
        let item_card_theme = client_theme().item_card();

        let (is_hovered, background_color) = match layout.get_mouse_mode() {
            MouseMode::Custom {
                mode: MouseInputMode::MoveItem { .. },
            } => match layout_info.area.check().any_mouse_mode().run(layout) {
                true => {
                    // Since we are not in default mouse mode we need to mark the window as
                    // hovered.
                    layout.set_hovered();

                    (true, *state.get(&item_card_theme.drag_hovered_background_color()))
                }
                false => (false, *state.get(&item_card_theme.drag_background_color())),
            },
            _ => match layout_info.area.check().run(layout) {
                true => (true, *state.get(&item_card_theme.hovered_background_color())),
                false => (false, *state.get(&item_card_theme.background_color())),
            },
        };

        let is_dragging = matches!(layout.get_mouse_mode(), MouseMode::Custom {
            mode: MouseInputMode::MoveItem { .. },
        });

        layout.add_rectangle(
            layout_info.area,
            CornerDiameter::uniform(20.0),
            background_color,
            *state.get(&item_card_theme.border_color()),
            ShadowPadding::diagonal(2.0, 5.0),
        );

        if is_hovered {
            layout.register_drop_handler(&self.handler);
        }

        if is_hovered
            && !is_dragging
            && let Some(tooltip) = self.tooltip.as_deref()
        {
            layout.add_tooltip(tooltip, ItemBoxTooltip.tooltip_id());
        }

        if let Some(item) = state.try_get(&self.item_path)
            && let Some(texture) = item.metadata.texture.as_ref()
        {
            let texture_size = layout_info.area.width.min(layout_info.area.height);
            let texture_area = Area {
                left: layout_info.area.left + (layout_info.area.width - texture_size) / 2.0,
                top: layout_info.area.top + (layout_info.area.height - texture_size) / 2.0,
                width: texture_size,
                height: texture_size,
            };

            layout.add_texture(texture_area, texture.clone(), Color::WHITE, false);

            if item.is_favorite() {
                let marker_area = Area {
                    left: layout_info.area.left + 2.0,
                    top: layout_info.area.top + 1.0,
                    width: 12.0,
                    height: 12.0,
                };

                layout.add_text(
                    marker_area,
                    "*",
                    FontSize(14.0),
                    FAVORITE_COLOR,
                    FAVORITE_COLOR,
                    HorizontalAlignment::Left { offset: 0.0, border: 1.0 },
                    VerticalAlignment::Top { offset: 0.0 },
                    OverflowBehavior::Shrink,
                );
            }

            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &self.handler);
                layout.register_click_handler(MouseButton::DoubleLeft, &self.double_click_handler);

                if self.context_menu != ItemBoxContextMenu::None {
                    layout.register_click_handler(MouseButton::Right, &self.context_menu_handler);
                }
            }

            if matches!(item.details, InventoryItemDetails::Regular { .. })
                && let Some(amount) = self.amount_display.string.as_ref()
            {
                // The amount display is populated in `create_layout_info`; an item
                // can arrive between layout creation and layout (e.g. the login
                // inventory burst), in which case we simply skip the amount text
                // for this frame instead of unwrapping `None`.
                layout.add_text(
                    layout_info.area,
                    amount,
                    // TODO: Put this in the theme
                    FontSize(12.0),
                    *state.get(&item_card_theme.text_color()),
                    *state.get(&item_card_theme.highlight_color()),
                    // TODO: Put this in the theme
                    HorizontalAlignment::Right { offset: 3.0, border: 3.0 },
                    // TODO: Put this in the theme
                    VerticalAlignment::Bottom { offset: 3.0 },
                    OverflowBehavior::Shrink,
                );
            }
        }
    }
}
