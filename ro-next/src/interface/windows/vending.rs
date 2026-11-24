//! Vending shop editor (`MC_VENDING` / `ZC_OPENSTORE`).
//!
//! The server opens this window with `ZC_OPENSTORE` after the cast; the player
//! picks items from the inventory and submits them with `CZ_REQ_OPENSTORE`.
//! Each added entry sells one unit at the configured price (the editor can be
//! extended with per-row amount/price controls later without touching the
//! packet layer).

use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::event::{ClickHandler, EventQueue};
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use ro_next_networking::InventoryItem;
use ragnarok_packets::InventoryIndex;
use rust_state::{Path, PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::vending::{Vending, VendingEntry, VendingPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

/// Maximum number of inventory rows shown at once.
const MAXIMUM_ITEMS: usize = 20;
/// Maximum shop-name length accepted by `CZ_REQ_OPENSTORE`.
const MAXIMUM_SHOP_NAME_LENGTH: usize = 80;

struct ShopNameFocus;
struct PriceFocus;

fn noop_input(_: &State<ClientState>, _: &mut EventQueue<ClientState>) {}

struct AddVendingEntry<A> {
    vending_path: A,
    inventory_index: u16,
    name: String,
}

impl<A> ClickHandler<ClientState> for AddVendingEntry<A>
where
    A: Path<ClientState, Vending>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        let price = state.get(&self.vending_path.price()).trim().parse::<u32>().unwrap_or(1).max(1);

        let entry = VendingEntry {
            inventory_index: self.inventory_index,
            amount: 1,
            price,
            name: self.name.clone(),
        };
        state.update_value_with(self.vending_path, move |vending| {
            vending.add_entry(entry.clone());
        });
    }
}

struct InventoryItemList<A, B> {
    vending_path: A,
    inventory_path: B,
    items: Vec<(u16, String)>,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A, B> InventoryItemList<A, B>
where
    A: Path<ClientState, Vending>,
    B: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn new(vending_path: A, inventory_path: B) -> Self {
        Self {
            vending_path,
            inventory_path,
            items: Vec::new(),
            elements: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for InventoryItemList<A, B>
where
    A: Path<ClientState, Vending> + 'static,
    B: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>> + 'static,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use ro_next_interface::prelude::*;

            self.items = state
                .get(&self.inventory_path)
                .iter()
                .take(MAXIMUM_ITEMS)
                .map(|item| (item.index.0, item.metadata.name.clone()))
                .collect();

            self.elements.clear();
            for (index, name) in self.items.iter() {
                let inventory_index = *index;
                let item_name = name.clone();
                let vending_path = self.vending_path;
                let button = button! {
                    text: item_name.clone(),
                    height: 20.0,
                    overflow_behavior: OverflowBehavior::Shrink,
                    event: AddVendingEntry {
                        vending_path,
                        inventory_index,
                        name: item_name,
                    },
                };
                self.elements.push(ErasedElement::new(button));
            }

            self.elements.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}

pub struct VendingWindow<A, B> {
    vending_path: A,
    inventory_path: B,
}

impl<A, B> VendingWindow<A, B> {
    pub fn new(vending_path: A, inventory_path: B) -> Self {
        Self {
            vending_path,
            inventory_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for VendingWindow<A, B>
where
    A: Path<ClientState, Vending> + 'static,
    B: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Vending)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let vending_path = self.vending_path;
        let inventory_path = self.inventory_path;

        window! {
            title: "Vending",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: false,
            minimum_width: 420.0,
            maximum_width: 520.0,
            elements: (
                split! {
                    children: (
                        text! {
                            text: "Shop name",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "My Shop",
                            state: vending_path.shop_name(),
                            input_handler: DefaultHandler::<_, _, MAXIMUM_SHOP_NAME_LENGTH>::new(vending_path.shop_name(), noop_input),
                            focus_id: ShopNameFocus,
                        },
                    ),
                },
                split! {
                    children: (
                        text! {
                            text: "Price per item (zeny)",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "1",
                            state: vending_path.price(),
                            input_handler: DefaultHandler::<_, _, 10>::new(vending_path.price(), noop_input),
                            focus_id: PriceFocus,
                        },
                    ),
                },
                text! {
                    text: "Choose an inventory item to sell at the configured price:",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                InventoryItemList::new(vending_path, inventory_path),
                split! {
                    gaps: 6.0,
                    children: (
                        button! {
                            text: "Open shop",
                            event: InputEvent::VendingOpen,
                        },
                        button! {
                            text: "Cancel",
                            event: InputEvent::VendingCancel,
                        },
                    ),
                },
            ),
        }
    }
}
