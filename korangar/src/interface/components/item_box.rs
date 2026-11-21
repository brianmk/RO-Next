use korangar_interface::MouseMode;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{BaseLayoutInfo, Element};
use korangar_interface::event::{ClickHandler, DropHandler, Event, EventQueue};
use korangar_interface::layout::area::Area;
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
}

struct ItemContextMenuHandler<P> {
    item_path: P,
}

impl<P> ClickHandler<ClientState> for ItemContextMenuHandler<P>
where
    P: Path<ClientState, InventoryItem<ResourceMetadata>, false>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        // Unwrapping here is fine since we only register the handler if the slot has a
        // item.
        let item = state.try_get(&self.item_path).unwrap();

        queue.queue(InputEvent::OpenInventoryItemMenu {
            inventory_index: item.index,
        });
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

pub struct ItemBox<A> {
    item_path: A,
    handler: ItemBoxHandler<A>,
    context_menu_handler: ItemContextMenuHandler<A>,
    context_menu: ItemBoxContextMenu,
    amount_display: AmountDisplay,
}

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
            context_menu_handler: ItemContextMenuHandler { item_path },
            context_menu: ItemBoxContextMenu::None,
            amount_display: AmountDisplay::default(),
        }
    }

    /// Enable the right click context menu on this item box.
    pub fn with_context_menu(mut self, context_menu: ItemBoxContextMenu) -> Self {
        self.context_menu = context_menu;
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

            if is_hovered {
                layout.register_click_handler(MouseButton::Left, &self.handler);

                if self.context_menu == ItemBoxContextMenu::Inventory {
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
