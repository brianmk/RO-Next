use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, PathExt, State, VecIndexExt};

use crate::ItemSource;
use crate::interface::components::item_box::{ItemBox, ItemBoxContextMenu};
use crate::interface::components::zeny::ZenyDisplay;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::inventory::{ItemCategory, item_category};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

/// Number of rows of the inventory grid.
const INVENTORY_ROWS: usize = 4;
/// Number of columns of the inventory grid.
const INVENTORY_COLUMNS: usize = 10;
/// Maximum number of items that are shown in one tab. Like the original client
/// the grid is fixed size; items beyond this are clipped (the inventory has no
/// scroll view, same as before the tabs were added).
const SLOTS_PER_TAB: usize = INVENTORY_ROWS * INVENTORY_COLUMNS;

pub struct InventoryWindow<A, B> {
    items_path: A,
    settings_path: B,
}

impl<A, B> InventoryWindow<A, B> {
    pub fn new(items_path: A, settings_path: B) -> Self {
        Self { items_path, settings_path }
    }
}

impl<A, B> CustomWindow<ClientState> for InventoryWindow<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, InterfaceSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Inventory)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        fn tab_button<B, T>(settings_path: B, tab: ItemCategory, text: T) -> impl Element<ClientState>
        where
            B: Path<ClientState, InterfaceSettings>,
            T: rust_state::Selector<ClientState, String> + 'static,
        {
            use korangar_interface::prelude::*;

            button! {
                text,
                disabled: ComputedSelector::new_default(move |state: &ClientState| *settings_path.inventory_tab().follow_safe(state) == tab),
                event: move |state: &State<ClientState>, _: &mut EventQueue<ClientState>| {
                    state.update_value(settings_path.inventory_tab(), tab);
                },
            }
        }

        window! {
            title: client_state().localization().inventory_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        tab_button(
                            self.settings_path,
                            ItemCategory::Consumables,
                            client_state().localization().inventory_consumables_tab_text(),
                        ),
                        tab_button(
                            self.settings_path,
                            ItemCategory::Equipments,
                            client_state().localization().inventory_equipments_tab_text(),
                        ),
                        tab_button(
                            self.settings_path,
                            ItemCategory::Misc,
                            client_state().localization().inventory_misc_tab_text(),
                        ),
                    ),
                },
                InventoryGrid::new(self.items_path, self.settings_path),
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text! {
                            text: client_state().localization().zeny_text(),
                            height: 18.0,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        ZenyDisplay::new(client_state().zeny()),
                    ),
                },
            ),
        }
    }
}

/// The grid of item boxes of the selected inventory tab.
///
/// The grid is rebuilt whenever the list of items shown in the selected tab
/// changes. Every item box points to the *real* position in the inventory, so
/// the item identity (inventory index) used by the use/equip/move/drop flows is
/// unchanged.
struct InventoryGrid<A, B> {
    items_path: A,
    settings_path: B,
    /// Positions in the inventory vector that are currently built into the
    /// grid.
    shown_positions: Option<Vec<usize>>,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A, B> InventoryGrid<A, B> {
    fn new(items_path: A, settings_path: B) -> Self {
        Self {
            items_path,
            settings_path,
            shown_positions: None,
            rows: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for InventoryGrid<A, B>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    B: Path<ClientState, InterfaceSettings>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use korangar_interface::prelude::*;

            let selected_tab = *state.get(&self.settings_path.inventory_tab());
            let items = state.get(&self.items_path);

            let positions: Vec<usize> = items
                .iter()
                .enumerate()
                .filter(|(_, item)| item_category(item) == selected_tab)
                // Equipped items disappear from the inventory grid and are only
                // shown in the equipment window (classic client behaviour).
                .filter(|(_, item)| !item.is_equipped())
                .map(|(position, _)| position)
                .take(SLOTS_PER_TAB)
                .collect();

            if self.shown_positions.as_deref() != Some(&positions) {
                let shown_positions = positions;
                self.rows.clear();

                for row in 0..INVENTORY_ROWS {
                    let items_path = self.items_path;
                    let shown_positions = shown_positions.clone();

                    self.rows.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: std::array::from_fn::<_, INVENTORY_COLUMNS, _>(|column| {
                            let slot = row * INVENTORY_COLUMNS + column;
                            // Items beyond the last one of the tab render as empty boxes.
                            let position = shown_positions.get(slot).copied().unwrap_or(usize::MAX);
                            ItemBox::component_new(items_path.index(position), ItemSource::Inventory)
                                    .with_context_menu(ItemBoxContextMenu::Inventory)
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
