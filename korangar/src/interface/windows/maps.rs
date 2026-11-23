//! Debug map warp window.
//!
//! The warp targets are data (`data\client\debug_maps.ron`, loaded through
//! [`crate::client_config`]) so a tester/server can change the debug list
//! without rebuilding the client. This is a debug-only convenience window (see
//! `korangar/tests/ui_consistency.rs::DEBUG_ONLY_FILES`), not user-facing
//! content, so its texts are intentionally not localized.

use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, ElementBox};
use korangar_interface::layout::{Resolvers, WindowLayout, with_nth_resolver};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::State;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct MapsWindow;

impl CustomWindow<ClientState> for MapsWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Maps)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Maps",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: WarpList::new(),
        }
    }
}

/// One button per row of the debug map data file.
///
/// The list is dynamic (an archive override may contain any number of maps),
/// so it replaces the previous fixed `[Button; 23]` array with a boxed child
/// list, the same pattern the other data-driven windows use.
struct WarpList {
    elements: Vec<ElementBox<ClientState>>,
}

impl WarpList {
    fn new() -> Self {
        use korangar_interface::prelude::*;

        let elements = crate::client_config::warps()
            .iter()
            .map(|warp| {
                let element: ElementBox<ClientState> = ErasedElement::new(button! {
                    text: warp.map_name().to_owned(),
                    tooltip: format!(
                        "Map: {}\nCoordinates: {}, {}",
                        warp.map_name(),
                        warp.position().x,
                        warp.position().y
                    ),
                    event: InputEvent::WarpToMap {
                        map_name: warp.map_file_name(),
                        position: warp.position(),
                    },
                });

                element
            })
            .collect::<Vec<ElementBox<ClientState>>>();

        Self { elements }
    }
}

impl Element<ClientState> for WarpList {
    type LayoutInfo = ();

    fn get_element_count(&self, _: &State<ClientState>) -> usize {
        self.elements.len()
    }

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        for (index, element) in self.elements.iter_mut().enumerate() {
            with_nth_resolver(resolvers, index, |resolver| {
                element.create_layout_info(state, store.child_store(index as u64), resolver as _);
            });
        }
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
