use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsCapabilities, InterfaceSettingsCapabilitiesPathExt, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct InterfaceSettingsWindow<A, B> {
    settings_path: A,
    capabilities_path: B,
}

impl<A, B> InterfaceSettingsWindow<A, B> {
    pub fn new(settings_path: A, capabilities_path: B) -> Self {
        Self {
            settings_path,
            capabilities_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for InterfaceSettingsWindow<A, B>
where
    A: Path<ClientState, InterfaceSettings>,
    B: Path<ClientState, InterfaceSettingsCapabilities>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::InterfaceSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        crate::logging::log_trace!("interface_settings: building Interface Settings window");

        let elements = (
            split! {
                children: (
                    text! {
                        text: client_state().localization().language_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.language(),
                        options: self.capabilities_path.languages(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().scaling_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.scaling(),
                        options: self.capabilities_path.scalings(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().menu_theme_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.menu_theme(),
                        options: self.capabilities_path.menu_themes(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().in_game_theme_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.in_game_theme(),
                        options: self.capabilities_path.in_game_themes(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().world_theme_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.world_theme(),
                        options: self.capabilities_path.world_themes(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().level_99_aura_text(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.level_99_aura_style(),
                        options: self.capabilities_path.level_99_aura_styles(),
                    }
                )
            },
            state_button! {
                text: client_state().localization().show_character_names_text(),
                state: self.settings_path.show_character_names(),
                event: Toggle(self.settings_path.show_character_names()),
            },
            state_button! {
                text: client_state().localization().show_chat_bubbles_text(),
                state: self.settings_path.show_chat_bubbles(),
                event: Toggle(self.settings_path.show_chat_bubbles()),
            },
            state_button! {
                text: client_state().localization().chat_bubble_background_text(),
                state: self.settings_path.chat_bubble_background(),
                event: Toggle(self.settings_path.chat_bubble_background()),
            },
            state_button! {
                text: client_state().localization().show_skill_names_text(),
                state: self.settings_path.show_skill_names_above_characters(),
                event: Toggle(self.settings_path.show_skill_names_above_characters()),
            },
        );

        crate::logging::log_trace!("interface_settings: constructed Interface Settings elements, creating window description");

        window! {
            title: client_state().localization().interface_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            elements,
        }
    }
}
