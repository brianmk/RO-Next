//! The big map window: a large view of the generated minimap of the current
//! map, opened with the second press of the `M` key cycle.

use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window, WindowTheme};
use rust_state::{Path, PathExt, Selector};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::interface::components::minimap::MinimapView;
use crate::interface::windows::WindowClass;
use crate::settings::InterfaceSettingsPathExt;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Size of the square map view in interface (unscaled) pixels.
const BIG_MAP_VIEW_SIZE: f32 = 460.0;

pub struct BigMapWindow;

/// Same theme-dependent selector as in the minimap window, so the big map
/// follows the "transparent outline" setting of the minimap.
fn big_map_window_value<Theme, Transparent, Value, Select>(
    theme_path: Theme,
    transparent_path: Transparent,
    select: Select,
    transparent_value: Value,
) -> impl Selector<ClientState, Value>
where
    Theme: Path<ClientState, WindowTheme<ClientState>> + Copy + 'static,
    Transparent: Path<ClientState, bool> + Copy + 'static,
    Value: Copy + Default + 'static,
    Select: Fn(&WindowTheme<ClientState>) -> Value + 'static,
{
    ComputedSelector::new_default(move |state: &ClientState| match *transparent_path.follow_safe(state) {
        true => transparent_value,
        false => select(theme_path.follow_safe(state)),
    })
}

impl CustomWindow<ClientState> for BigMapWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BigMap)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let transparent_path = client_state().interface_settings().minimap_transparent_frame();
        let window_theme_path = client_theme().window();

        let background_color = big_map_window_value(
            window_theme_path,
            transparent_path,
            |window| window.background_color,
            Color::TRANSPARENT,
        );
        let shadow_color = big_map_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_color,
            Color::TRANSPARENT,
        );
        let shadow_padding = big_map_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_padding,
            ShadowPadding::uniform(0.0),
        );
        let border = big_map_window_value(window_theme_path, transparent_path, |window| window.border, 0.0);
        let corner_diameter = big_map_window_value(
            window_theme_path,
            transparent_path,
            |window| window.corner_diameter,
            CornerDiameter::uniform(0.0),
        );

        window! {
            title: client_state().localization().big_map_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            background_color: background_color,
            shadow_color: shadow_color,
            shadow_padding: shadow_padding,
            border: border,
            corner_diameter: corner_diameter,
            skinned: false,
            closable: true,
            resizable: false,
            minimum_width: 480.0,
            maximum_width: 480.0,
            minimum_height: 500.0,
            maximum_height: 500.0,
            elements: (
                MinimapView::new(
                    client_state().localization().minimap_loading_text(),
                    transparent_path,
                    BIG_MAP_VIEW_SIZE,
                ),
            ),
        }
    }
}
