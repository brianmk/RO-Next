//! Merchant pushcart window.
//!
//! The cart is the merchant's second inventory. rAthena sends its contents
//! through the same list family as the normal inventory
//! (`ZC_INVENTORY_START`/`ZC_ITEMLIST_NORMAL`/`ZC_ITEMLIST_EQUIP`/
//! `ZC_INVENTORY_END`) with `inventory_type` 1, see `clif_cartlist` in
//! `clif.cpp:3226`. Items are moved between inventory, cart and the open Kafra
//! storage with the `CZ_MOVE_ITEM_FROM_*` packets (0x0126 to 0x0129).
//!
//! Like the storage window the grid is rebuilt whenever the set of visible
//! items changes, and every box points at the real item of the cart vector.

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, State, VecIndexExt};

use crate::interface::components::item_box::{DEFAULT_ITEM_BOX_SIZE, ItemBox, ItemBoxContextMenu};
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::inventory::InventoryPathExt;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

/// Number of rows of the cart grid.
const CART_ROWS: usize = 5;
/// Number of columns of the cart grid.
const CART_COLUMNS: usize = 10;
/// Maximum number of items that are shown in the grid (`MAX_CART` is 100, the
/// grid is fixed size like the storage grid and clips the rest).
const SLOTS: usize = CART_ROWS * CART_COLUMNS;

pub struct CartWindow;

impl CustomWindow<ClientState> for CartWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Cart)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().cart_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: "Double click or drag an item to move it between cart, inventory and storage.",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                CartGrid::new(client_state().cart().items()),
            ),
        }
    }
}

/// The grid of cart item boxes.
struct CartGrid<A> {
    items_path: A,
    /// Positions in the cart vector that are currently built into the grid.
    shown_positions: Option<Vec<usize>>,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A> CartGrid<A> {
    fn new(items_path: A) -> Self {
        Self {
            items_path,
            shown_positions: None,
            rows: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for CartGrid<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        use korangar_interface::prelude::*;

        with_single_resolver(resolvers, |resolver| {
            let items = state.get(&self.items_path);

            let positions: Vec<usize> = (0..items.len()).take(SLOTS).collect();

            if self.shown_positions.as_deref() != Some(&positions) {
                let shown_positions = positions;
                self.rows.clear();

                for row in 0..CART_ROWS {
                    let items_path = self.items_path;
                    let shown_positions = shown_positions.clone();

                    self.rows.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, CART_COLUMNS, _>(|column| {
                            let slot = row * CART_COLUMNS + column;
                            // Items beyond the last one render as empty boxes.
                            let position = shown_positions.get(slot).copied().unwrap_or(usize::MAX);
                            ItemBox::component_new(items_path.index(position), ItemSource::Cart, DEFAULT_ITEM_BOX_SIZE)
                                .with_context_menu(ItemBoxContextMenu::Cart)
                        }),
                    }));
                }

                self.shown_positions = Some(shown_positions);
            }

            self.rows
                .iter_mut()
                .enumerate()
                .for_each(|(index, element)| element.create_layout_info(state, store.child_store(index as u64), resolver));
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.rows
            .iter()
            .enumerate()
            .for_each(|(index, element)| element.lay_out(state, store.child_store(index as u64), &(), layout));
    }
}
