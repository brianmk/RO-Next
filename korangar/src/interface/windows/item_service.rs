//! Shared selection window for the 2019 item services: weapon refine, item
//! repair, identify and card composition.
//!
//! The server pushes the selectable list (`ZC_NOTIFY_WEAPONITEMLIST` 0x0221,
//! `ZC_REPAIRITEMLIST` 0x01FC, `ZC_ITEMIDENTIFY_LIST` 0x0177,
//! `ZC_ITEMCOMPOSITION_LIST` 0x017B) and this window lets the player answer it
//! with the matching `CZ_*` request. The list itself lives in
//! [`crate::state::item_service`] so the network handler can replace it while
//! the window is open.
//!
//! Reachability:
//! * refine/repair lists are sent by rAthena after the Blacksmith
//!   `WS_WEAPONREFINE` / `BS_REPAIRWEAPON` skill was used,
//! * the identify list is sent by the merchant Item Appraisal skill (which
//!   consumes a Magnifier); unidentified items can also be identified directly
//!   from the inventory context menu (`CZ_REQ_ONECLICK_ITEMIDENTIFY`, 0x0A35),
//! * the card composition list is requested from the inventory context menu of
//!   a card (`CZ_REQ_ITEMCOMPOSITION_LIST`, 0x017A).

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, Selector, State, VecIndexExt};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::item_service::{ItemServiceEntry, ItemServiceKind, ItemServicePathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Width of the service window.
const ITEM_SERVICE_WINDOW_WIDTH: f32 = 420.0;

/// Window that lets the player pick the item an item-service request applies
/// to.
pub struct ItemServiceWindow {
    kind: ItemServiceKind,
}

impl ItemServiceWindow {
    pub fn new(kind: ItemServiceKind) -> Self {
        Self { kind }
    }
}

impl CustomWindow<ClientState> for ItemServiceWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ItemService)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let kind = self.kind;

        window! {
            title: kind.title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: ITEM_SERVICE_WINDOW_WIDTH,
            maximum_width: 700.0,
            maximum_height: 640.0,
            elements: (
                text! {
                    text: client_state().item_service().status_message(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: kind.hint(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: card_text(kind),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                scroll_view! {
                    scroll_bar: true,
                    fit_content_height: true,
                    children: ServiceItemList::new(client_state().item_service().entries(), kind),
                },
                button! {
                    text: "Close",
                    event: |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::CloseItemService);
                    },
                },
            ),
        }
    }
}

/// Text line that shows which card is being inserted (composition only).
fn card_text(kind: ItemServiceKind) -> impl Selector<ClientState, String> {
    korangar_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        if kind != ItemServiceKind::CardComposition {
            return String::new();
        }

        match client_state().item_service().card().follow_safe(state) {
            Some(card) => format!("Card: {}", card.name),
            None => String::new(),
        }
    })
}

/// The rows of the current service list.
///
/// Like the friend list it rebuilds the row elements whenever the number of
/// entries changes and keeps one element per entry so the per-row button can
/// read the item index from the state path.
struct ServiceItemList<A> {
    entries_path: A,
    kind: ItemServiceKind,
    rows: Vec<ElementBox<ClientState>>,
}

impl<A> ServiceItemList<A> {
    fn new(entries_path: A, kind: ItemServiceKind) -> Self {
        Self {
            entries_path,
            kind,
            rows: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for ServiceItemList<A>
where
    A: Path<ClientState, Vec<ItemServiceEntry>>,
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
            let entries = state.get(&self.entries_path);

            if self.rows.len() != entries.len() {
                self.rows.clear();

                for index in 0..entries.len() {
                    let entry_path = self.entries_path.index(index).manually_asserted();
                    let text_path = entry_path;
                    let click_path = entry_path;
                    let kind = self.kind;

                    let display_text = ComputedSelector::new_default(move |state: &ClientState| text_path.follow_safe(state).label());

                    self.rows.push(ErasedElement::new(split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: display_text,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: kind.action_label(),
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let entry = state.get(&click_path);
                                    let inventory_index = entry.index;

                                    match kind {
                                        ItemServiceKind::WeaponRefine => {
                                            queue.queue(InputEvent::RefineWeapon { inventory_index });
                                        }
                                        ItemServiceKind::ItemRepair => {
                                            queue.queue(InputEvent::RepairItem { inventory_index });
                                        }
                                        ItemServiceKind::ItemIdentify => {
                                            queue.queue(InputEvent::IdentifyItem { inventory_index });
                                        }
                                        ItemServiceKind::CardComposition => {
                                            if let Some(card) = state.get(&client_state().item_service().card()) {
                                                queue.queue(InputEvent::InsertCard {
                                                    card_index: card.index,
                                                    equip_index: inventory_index,
                                                });
                                            }
                                        }
                                    }
                                },
                            },
                        ),
                    }));
                }
            }

            self.rows.iter_mut().enumerate().for_each(|(index, element)| {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.rows.iter().enumerate().for_each(|(index, element)| {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        });
    }
}
