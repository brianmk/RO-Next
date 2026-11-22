use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window, WindowTheme};
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::interface::components::minimap::{MINIMAP_VIEW_SIZE, MinimapView};
use crate::interface::windows::WindowClass;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

pub struct MinimapWindow;

/// Click handler for the "transparent outline" toggle in the minimap window.
///
/// Besides toggling the state (which is read every frame, so the change is
/// applied live) the interface settings are saved immediately, so the choice
/// survives crashes like the other interface settings.
struct TransparentMinimapToggle<A, B> {
    value_path: A,
    settings_path: B,
}

impl<A, B> ClickHandler<ClientState> for TransparentMinimapToggle<A, B>
where
    A: Path<ClientState, bool>,
    B: Path<ClientState, InterfaceSettings>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        state.update_value_with(self.value_path, |transparent| *transparent = !*transparent);
        state.update_value_with(self.settings_path, |settings| settings.save());
    }
}

/// Build a selector for a minimap window value that depends on the
/// "transparent outline" setting.
///
/// In transparent mode the dark window frame (background, border and shadow)
/// is replaced by fully transparent values, so only the generated map image is
/// visible. The normal values are taken from the active theme, which makes
/// this work for the default and the classic theme alike.
fn minimap_window_value<Theme, Transparent, Value, Select>(
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

impl CustomWindow<ClientState> for MinimapWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Minimap)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let settings_path = client_state().interface_settings();
        let transparent_path = settings_path.minimap_transparent_frame();
        let window_theme_path = client_theme().window();

        let background_color = minimap_window_value(
            window_theme_path,
            transparent_path,
            |window| window.background_color,
            Color::TRANSPARENT,
        );
        let shadow_color = minimap_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_color,
            Color::TRANSPARENT,
        );
        let shadow_padding = minimap_window_value(
            window_theme_path,
            transparent_path,
            |window| window.shadow_padding,
            ShadowPadding::uniform(0.0),
        );
        let border = minimap_window_value(window_theme_path, transparent_path, |window| window.border, 0.0);
        let corner_diameter = minimap_window_value(
            window_theme_path,
            transparent_path,
            |window| window.corner_diameter,
            CornerDiameter::uniform(0.0),
        );

        window! {
            title: client_state().localization().minimap_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            background_color: background_color,
            shadow_color: shadow_color,
            shadow_padding: shadow_padding,
            border: border,
            corner_diameter: corner_diameter,
            // The window frame is color based (not the opaque nine slice skin)
            // so that the transparent mode works in every theme.
            skinned: false,
            // The minimap is toggled with the map key (and dragged by its title), it
            // should not have an X close button.
            closable: false,
            resizable: false,
            minimum_width: 210.0,
            maximum_width: 210.0,
            minimum_height: 250.0,
            maximum_height: 250.0,
            elements: (
                MinimapView::new(
                    client_state().localization().minimap_loading_text(),
                    transparent_path,
                    MINIMAP_VIEW_SIZE,
                ),
                round_toggle! {
                    tooltip: client_state().localization().minimap_transparent_frame_tooltip(),
                    state: transparent_path,
                    event: TransparentMinimapToggle {
                        value_path: transparent_path,
                        settings_path,
                    },
                    alignment: HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                    shadow_color: Color::TRANSPARENT,
                    shadow_padding: ShadowPadding::uniform(0.0),
                },
            ),
        }
    }
}
