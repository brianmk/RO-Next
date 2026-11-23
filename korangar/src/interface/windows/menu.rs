use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
#[cfg(feature = "debug")]
use crate::state::client_theme;
use crate::state::localization::LocalizationPathExt;
#[cfg(feature = "debug")]
use crate::state::localization::UiTextsPathExt;
use crate::state::theme::InterfaceThemeType;
#[cfg(feature = "debug")]
use crate::state::theme::{DebugButtonThemePathExt, InterfaceThemePathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct MenuWindow;

impl CustomWindow<ClientState> for MenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Menu)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: client_state().localization().menu_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                button! {
                    text: client_state().localization().game_settings_button_text(),
                    event: InputEvent::ToggleGameSettingsWindow,
                },
                button! {
                    text: client_state().localization().interface_settings_button_text(),
                    event: InputEvent::ToggleInterfaceSettingsWindow,
                },
                button! {
                    text: client_state().localization().graphics_settings_button_text(),
                    event: InputEvent::ToggleGraphicsSettingsWindow,
                },
                button! {
                    text: client_state().localization().audio_settings_button_text(),
                    event: InputEvent::ToggleAudioSettingsWindow,
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_render_options(),
                    tooltip: client_state().localization().ui().menu_render_options_tooltip(),
                    event: InputEvent::ToggleRenderOptionsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_map_inspector(),
                    tooltip: client_state().localization().ui().menu_map_inspector_tooltip(),
                    event: InputEvent::OpenMapDataWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_client_state_inspector(),
                    tooltip: client_state().localization().ui().menu_client_state_inspector_tooltip(),
                    event: InputEvent::ToggleClientStateInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_maps(),
                    tooltip: client_state().localization().ui().menu_maps_tooltip(),
                    event: InputEvent::ToggleMapsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_commands(),
                    tooltip: client_state().localization().ui().menu_commands_tooltip(),
                    event: InputEvent::ToggleCommandsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_theme_inspector(),
                    tooltip: client_state().localization().ui().menu_theme_inspector_tooltip(),
                    event: InputEvent::ToggleThemeInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_profiler(),
                    tooltip: client_state().localization().ui().menu_profiler_tooltip(),
                    event: InputEvent::ToggleProfilerWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_packet_inspector(),
                    tooltip: client_state().localization().ui().menu_packet_inspector_tooltip(),
                    event: InputEvent::TogglePacketInspectorWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                #[cfg(feature = "debug")]
                button! {
                    text: client_state().localization().ui().menu_cache_statistics(),
                    tooltip: client_state().localization().ui().menu_cache_statistics_tooltip(),
                    event: InputEvent::ToggleCacheStatisticsWindow,
                    foreground_color: client_theme().debug_button().foreground_color(),
                    hovered_background_color: client_theme().debug_button().hovered_background_color(),
                },
                button! {
                    text: "Quest Log",
                    event: InputEvent::ToggleQuestWindow,
                },
                button! {
                    text: client_state().localization().minimap_button_text(),
                    event: InputEvent::ToggleMinimapWindow,
                },
                button! {
                    text: client_state().localization().log_out_button_text(),
                    event: InputEvent::LogOut,
                },
                button! {
                    text: client_state().localization().exit_button_text(),
                    event: InputEvent::Exit,
                },
            ),
        }
    }
}
