//! Browse and buy from another player's vending shop.
//!
//! The shop list is requested with `CZ_REQ_BUY_FROMMC` (0x0130) when the player
//! picks "Browse shop" in another player's context menu. rAthena answers with
//! `ZC_PC_PURCHASE_ITEMLIST_FROMMC` (0x0800), which carries the seller's cart
//! slot, price, stock and item id of every row (`clif.cpp:7592`). Buying sends
//! `CZ_PC_PURCHASE_ITEMLIST_FROMMC` (0x0134, `clif.cpp:14138`) with the rows
//! the player selected.

use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::vending::{VendingBrowse, VendingBrowsePathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Fallback title when the shop entry did not carry a name.
const DEFAULT_SHOP_TITLE: &str = "Vending shop";

pub struct VendingBrowseWindow<A> {
    browse_path: A,
}

impl<A> VendingBrowseWindow<A> {
    pub fn new(browse_path: A) -> Self {
        Self { browse_path }
    }
}

impl<A> CustomWindow<ClientState> for VendingBrowseWindow<A>
where
    A: Path<ClientState, VendingBrowse> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::VendingBrowse)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        fn title<A>(browse_path: A) -> impl Selector<ClientState, String>
        where
            A: Path<ClientState, VendingBrowse>,
        {
            ComputedSelector::new_default(move |state: &ClientState| {
                let shop_name = browse_path.shop_name().follow_safe(state);

                match shop_name.is_empty() {
                    true => DEFAULT_SHOP_TITLE.to_owned(),
                    false => shop_name.clone(),
                }
            })
        }

        let browse_path = self.browse_path;

        window! {
            title: title(browse_path),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 380.0,
            maximum_width: 520.0,
            elements: (
                text! {
                    text: "Click an item to buy one unit. The seller's stock is shown in brackets.",
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                VendingItemList::new(browse_path),
            ),
        }
    }
}

/// Builds one "Buy" button per shop row. The buttons are rebuilt when the rows
/// (name, price, stock) change, e.g. after a purchase reduced the stock.
struct VendingItemList<A> {
    browse_path: A,
    shown_rows: Vec<(String, u32, u16, u16)>,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> VendingItemList<A> {
    fn new(browse_path: A) -> Self {
        Self {
            browse_path,
            shown_rows: Vec::new(),
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for VendingItemList<A>
where
    A: Path<ClientState, VendingBrowse> + 'static,
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

            let rows: Vec<(String, u32, u16, u16)> = state
                .get(&self.browse_path)
                .items()
                .iter()
                .map(|item| (item.name.clone(), item.price, item.stock, item.cart_index.0))
                .collect();

            if self.shown_rows != rows {
                self.elements.clear();

                for item in state.get(&self.browse_path).items() {
                    let label = format!("{}  -  {} z  [{}]", item.name, item.price, item.stock);
                    let cart_index = item.cart_index;
                    let price = item.price;

                    self.elements.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: label,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: client_state().localization().buy_button_text(),
                                height: 20.0,
                                event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    queue.queue(InputEvent::BuyVendingItem { cart_index, amount: 1, price });
                                },
                            },
                        ),
                    }));
                }

                self.shown_rows = rows;
            }

            self.elements
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
        self.elements
            .iter()
            .enumerate()
            .for_each(|(index, element)| element.lay_out(state, store.child_store(index as u64), &(), layout));
    }
}
