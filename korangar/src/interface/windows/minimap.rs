use korangar_interface::window::{CustomWindow, Window};

use crate::interface::components::minimap::MinimapView;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct MinimapWindow;

impl CustomWindow<ClientState> for MinimapWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Minimap)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().minimap_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: false,
            minimum_width: 210.0,
            maximum_width: 210.0,
            minimum_height: 225.0,
            maximum_height: 225.0,
            elements: (MinimapView::new(client_state().localization().minimap_loading_text()),),
        }
    }
}
