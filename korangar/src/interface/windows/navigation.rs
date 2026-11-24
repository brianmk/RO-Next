//! Navigation destination search window.
//!
//! The destination data is the official client's navigation Lua tables
//! (see [`crate::world::library::navigation`]). Selecting a result queues
//! [`InputEvent::NavigateToDestination`]; `lib.rs` then marks the destination
//! and computes the local route with the existing pathfinder. The route and
//! the destination marker are drawn by the minimap component.

use std::sync::Arc;

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{PathExt, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::navigation::NavigationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{NavigationData, NavigationDestination};

pub struct NavigationWindow {
    database: Arc<NavigationData>,
}

impl NavigationWindow {
    pub fn new(database: Arc<NavigationData>) -> Self {
        Self { database }
    }
}

fn status_text(database: Arc<NavigationData>) -> impl Selector<ClientState, String> {
    ComputedSelector::new_default(move |state: &ClientState| {
        let navigation = client_state().navigation().follow_safe(state);
        let mut text = match navigation.destination() {
            Some(destination) => {
                let route = match navigation.is_on_current_map() {
                    true => format!("Route: {} tiles", navigation.route().len()),
                    false => "Destination is on another map; the client only draws the local route.".to_owned(),
                };
                format!("Destination: {}\n{route}", destination.description())
            }
            None => "No destination set. Search for a map, NPC or monster, or use /navi <map> <x> <y>.".to_owned(),
        };

        if !navigation.status().is_empty() {
            text.push('\n');
            text.push_str(navigation.status());
        }

        if database.is_empty() {
            text.push_str("\nNo navigation data was found in the game archives (data\\luafiles514\\lua files\\navigation).");
        }

        text
    })
}

impl CustomWindow<ClientState> for NavigationWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Navigation)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let search_path = client_state().navigation().query();

        let status_text = status_text(self.database.clone());

        let clear_action = |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            queue.queue(InputEvent::ClearNavigationDestination);
        };

        struct NavigationSearch;

        window! {
            title: client_state().localization().ui().navigation_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 460.0,
            maximum_width: 720.0,
            maximum_height: 700.0,
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: (
                    text! {
                        text: self.database.hint(),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text_box! {
                        ghost_text: "Search map, NPC or monster",
                        state: search_path,
                        input_handler: DefaultHandler::<_, _, 60>::new(search_path, |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {}),
                        focus_id: NavigationSearch,
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    NavigationResultList::new(self.database.clone()),
                    split! {
                        gaps: theme().window().gaps(),
                        children: (
                            button! {
                                text: "Clear destination",
                                event: clear_action,
                            },
                            text! {
                                text: status_text,
                                overflow_behavior: OverflowBehavior::LineBreak,
                            },
                        ),
                    },
                ),
            },
        }
    }
}

/// Builds one button per search result. The buttons are rebuilt when the
/// search text changes.
struct NavigationResultList {
    database: Arc<NavigationData>,
    shown_query: String,
    elements: Vec<ElementBox<ClientState>>,
}

impl NavigationResultList {
    fn new(database: Arc<NavigationData>) -> Self {
        Self {
            database,
            shown_query: String::new(),
            elements: Vec::new(),
        }
    }

    fn rebuild(&mut self, query: &str) {
        use korangar_interface::prelude::*;

        self.shown_query = query.to_owned();
        self.elements.clear();

        let matches = self.database.search(query);

        if matches.is_empty() && !query.trim().is_empty() {
            self.elements.push(ErasedElement::new(text! {
                text: "No matching destination. Try a map name (prontera), an NPC name or a monster name.",
                overflow_behavior: OverflowBehavior::LineBreak,
            }));
        }

        for destination in matches {
            self.elements.push(self.result_button(destination));
        }
    }

    fn result_button(&self, destination: &NavigationDestination) -> ElementBox<ClientState> {
        use korangar_interface::prelude::*;

        let label = destination.label();
        let name = destination.name.clone();
        let map_name = destination.map_name.clone();
        let position = destination.position;

        ErasedElement::new(button! {
            text: label,
            event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                queue.queue(InputEvent::NavigateToDestination {
                    name: name.clone(),
                    map_name: map_name.clone(),
                    position,
                });
            },
        })
    }
}

impl Element<ClientState> for NavigationResultList {
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let path = client_state().navigation();
            let query = state.get(&path).search().to_owned();

            if query != self.shown_query {
                self.rebuild(&query);
            }

            for (index, element) in self.elements.iter_mut().enumerate() {
                element.create_layout_info(state, store.child_store(index as u64), resolver);
            }
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, element) in self.elements.iter().enumerate() {
            element.lay_out(state, store.child_store(index as u64), &(), layout);
        }
    }
}
