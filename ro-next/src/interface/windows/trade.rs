//! Trade request prompt and trade window (`CZ_REQ_EXCHANGE_ITEM` and friends).
//!
//! The 2019 exchange protocol sends the partner's offers via separate packets;
//! this window mirrors the local side (items + lock state) and offers the
//! request accept/reject, add-item, lock, commit and cancel actions.

use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::event::{ClickHandler, EventQueue};
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use ro_next_networking::InventoryItem;
use rust_state::{Path, PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::trade::{Trade, TradePathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

const MAXIMUM_ITEMS: usize = 12;

struct ZenyFocus;

fn noop_input(_: &State<ClientState>, _: &mut EventQueue<ClientState>) {}

pub struct TradeRequestWindow {
    requester_name: String,
    account_id: u32,
}

impl TradeRequestWindow {
    pub fn new(requester_name: String, account_id: u32) -> Self {
        Self {
            requester_name,
            account_id,
        }
    }
}

impl CustomWindow<ClientState> for TradeRequestWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::TradeRequest)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let accept_account = self.account_id;
        let name = self.requester_name;

        window! {
            title: "Trade Request",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 320.0,
            maximum_width: 420.0,
            elements: (
                text! {
                    text: format!("{name} wants to trade with you."),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                split! {
                    children: (
                        button! {
                            text: "Accept",
                            event: InputEvent::TradeAccept { account_id: accept_account },
                        },
                        button! {
                            text: "Decline",
                            event: InputEvent::TradeDecline,
                        },
                    ),
                },
            ),
        }
    }
}

struct AddTradeItem<A> {
    trade_path: A,
    inventory_index: u16,
    name: String,
}

impl<A> ClickHandler<ClientState> for AddTradeItem<A>
where
    A: Path<ClientState, Trade>,
{
    fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        let trade_path = self.trade_path;
        let inventory_index = self.inventory_index;
        let name = self.name.clone();
        state.update_value_with(trade_path, move |trade| {
            trade.add_item(inventory_index, name.clone(), 1);
        });
        queue.queue(InputEvent::TradeAddItem {
            inventory_index: self.inventory_index,
            amount: 1,
        });
    }
}

struct TradeInventoryList<A, B> {
    trade_path: A,
    inventory_path: B,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A, B> TradeInventoryList<A, B> {
    fn new(trade_path: A, inventory_path: B) -> Self {
        Self {
            trade_path,
            inventory_path,
            elements: Vec::new(),
        }
    }
}

impl<A, B> Element<ClientState> for TradeInventoryList<A, B>
where
    A: Path<ClientState, Trade> + 'static,
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

            self.elements.clear();
            let items: Vec<(u16, String)> = state
                .get(&self.inventory_path)
                .iter()
                .take(MAXIMUM_ITEMS)
                .map(|item| (item.index.0, item.metadata.name.clone()))
                .collect();

            for (inventory_index, name) in items {
                let trade_path = self.trade_path;
                self.elements.push(ErasedElement::new(button! {
                    text: name.clone(),
                    height: 20.0,
                    overflow_behavior: OverflowBehavior::Shrink,
                    event: AddTradeItem {
                        trade_path,
                        inventory_index,
                        name,
                    },
                }));
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

pub struct TradeWindow<A, B> {
    trade_path: A,
    inventory_path: B,
}

impl<A, B> TradeWindow<A, B> {
    pub fn new(trade_path: A, inventory_path: B) -> Self {
        Self {
            trade_path,
            inventory_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for TradeWindow<A, B>
where
    A: Path<ClientState, Trade> + 'static,
    B: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Trade)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let trade_path = self.trade_path;
        let inventory_path = self.inventory_path;

        window! {
            title: "Trade",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: false,
            minimum_width: 360.0,
            maximum_width: 480.0,
            elements: (
                text! {
                    text: ComputedSelector::new_default(move |state: &ClientState| {
                        let trade = trade_path.follow_safe(state);
                        format!("Trading with {}", trade.partner_name())
                    }),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: ComputedSelector::new_default(move |state: &ClientState| {
                        let trade = trade_path.follow_safe(state);
                        if trade.items().is_empty() {
                            "No items offered yet.".to_string()
                        } else {
                            trade.items().iter().map(|item| format!("{} x{}", item.name, item.amount)).collect::<Vec<_>>().join("\n")
                        }
                    }),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: ComputedSelector::new_default(move |state: &ClientState| {
                        let trade = trade_path.follow_safe(state);
                        match trade.partner_offer().is_empty() {
                            true => "Partner has not offered anything yet.".to_string(),
                            false => format!("Partner offered: {}", trade.partner_offer()),
                        }
                    }),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                TradeInventoryList::new(trade_path, inventory_path),
                split! {
                    children: (
                        text! {
                            text: "Zeny",
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "0",
                            state: trade_path.zeny(),
                            input_handler: DefaultHandler::<_, _, 10>::new(trade_path.zeny(), noop_input),
                            focus_id: ZenyFocus,
                        },
                        button! {
                            text: "Offer zeny",
                            event: InputEvent::TradeOfferZeny,
                        },
                    ),
                },
                split! {
                    gaps: 6.0,
                    children: (
                        button! {
                            text: "Lock",
                            event: InputEvent::TradeLock,
                        },
                        button! {
                            text: "Trade",
                            event: InputEvent::TradeCommit,
                        },
                        button! {
                            text: "Cancel",
                            event: InputEvent::TradeCancel,
                        },
                    ),
                },
            ),
        }
    }
}
