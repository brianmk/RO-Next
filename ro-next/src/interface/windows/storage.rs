//! Kafra storage window.
//!
//! The window is opened by the server: when the player talks to a Kafra NPC and
//! chooses "Use Storage", the map server sends the shared inventory list
//! packets with `inventory_type` 2 (`ZC_INVENTORY_START`/`_LIST`/`_END`). The
//! packet handlers fill the storage state and the client opens this window.
//!
//! Items can be taken out through the right click context menu; inventory items
//! can be stored while this window is open (the inventory context menu then
//! gets "Store" entries).

use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use ro_next_networking::InventoryItem;
use rust_state::{Path, PathExt, Selector, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::components::item_box::{ItemBox, ItemBoxContextMenu};
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::inventory::InventoryPathExt;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

/// Number of rows of the storage grid.
const STORAGE_ROWS: usize = 5;
/// Number of columns of the storage grid.
const STORAGE_COLUMNS: usize = 10;
/// Maximum number of items that are shown in the grid.
const SLOTS: usize = STORAGE_ROWS * STORAGE_COLUMNS;

pub struct StorageWindow;

impl CustomWindow<ClientState> for StorageWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Storage)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        fn capacity_text() -> impl Selector<ClientState, String> {
            ComputedSelector::new_default(move |state: &ClientState| {
                let amount = *client_state().storage_amount().follow_safe(state);
                let maximum_amount = *client_state().storage_maximum_amount().follow_safe(state);
                let name = client_state().storage_name().follow_safe(state);

                let title = client_state().localization().storage_window_title().follow_safe(state);
                let name = match name.is_empty() {
                    true => title.as_str(),
                    false => name.as_str(),
                };

                format!("{name}: {amount} / {maximum_amount}")
            })
        }

        window! {
            title: client_state().localization().storage_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text! {
                            text: capacity_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: client_state().localization().close_button_text(),
                            tooltip: client_state().localization().ui().storage_close_tooltip(),
                            event: |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                queue.queue(InputEvent::CloseStorage);
                            },
                        },
                    ),
                },
                StorageGrid::new(client_state().storage().items()),
            ),
        }
    }
}

/// The grid of storage item boxes.
///
/// Like the inventory grid it is rebuilt whenever the set of visible items
/// changes and every box points at the real item in the storage vector.
struct StorageGrid<A> {
    items_path: A,
    /// Positions in the storage vector that are currently built into the grid.
    shown_positions: Option<Vec<usize>>,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A> StorageGrid<A> {
    fn new(items_path: A) -> Self {
        Self {
            items_path,
            shown_positions: None,
            rows: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for StorageGrid<A>
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
        use ro_next_interface::prelude::*;

        with_single_resolver(resolvers, |resolver| {
            let items = state.get(&self.items_path);

            let positions: Vec<usize> = (0..items.len()).take(SLOTS).collect();

            if self.shown_positions.as_deref() != Some(&positions) {
                let shown_positions = positions;
                self.rows.clear();

                for row in 0..STORAGE_ROWS {
                    let items_path = self.items_path;
                    let shown_positions = shown_positions.clone();

                    self.rows.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, STORAGE_COLUMNS, _>(|column| {
                            let slot = row * STORAGE_COLUMNS + column;
                            // Items beyond the last one render as empty boxes.
                            let position = shown_positions.get(slot).copied().unwrap_or(usize::MAX);
                            ItemBox::component_new(items_path.index(position), ItemSource::Storage, crate::interface::components::item_box::DEFAULT_ITEM_BOX_SIZE)
                                .with_context_menu(ItemBoxContextMenu::Storage)
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
