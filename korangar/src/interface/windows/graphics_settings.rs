use korangar_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{GraphicsSettingsCapabilitiesPathExt, GraphicsSettingsPathExt};
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::{GraphicsSettings, GraphicsSettingsCapabilities};

pub struct GraphicsSettingsWindow<A, B> {
    settings_path: A,
    capabilities_path: B,
}

impl<A, B> GraphicsSettingsWindow<A, B> {
    pub fn new(settings_path: A, capabilities_path: B) -> Self {
        Self {
            settings_path,
            capabilities_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for GraphicsSettingsWindow<A, B>
where
    A: Path<ClientState, GraphicsSettings>,
    B: Path<ClientState, GraphicsSettingsCapabilities>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::GraphicsSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let elements = (
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_resolution_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.window_resolution(),
                        options: self.capabilities_path.window_resolution_options(),
                    }
                )
            },
            state_button! {
                text: client_state().localization().ui().graphics_borderless_fullscreen_label(),
                state: self.settings_path.fullscreen(),
                event: Toggle(self.settings_path.fullscreen()),
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_lighting_mode_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.lighting_mode(),
                        options: self.capabilities_path.lighting_modes(),
                    }
                )
            },
            state_button! {
                text: client_state().localization().ui().graphics_triple_buffering_label(),
                state: self.settings_path.triple_buffering(),
                event: Toggle(self.settings_path.triple_buffering()),
            },
            state_button! {
                text: client_state().localization().ui().graphics_vsync_label(),
                state: self.settings_path.vsync(),
                event: Toggle(self.settings_path.vsync()),
                disabled: self.capabilities_path.vsync_setting_disabled(),
                disabled_tooltip: client_state().localization().ui().setting_unsupported_tooltip(),
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_max_fps_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.max_fps(),
                        options: self.capabilities_path.max_fps_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_background_fps_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.background_max_fps(),
                        options: self.capabilities_path.background_max_fps_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_texture_filtering_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.texture_filtering(),
                        options: self.capabilities_path.texture_filtering_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_multisampling_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.msaa(),
                        options: self.capabilities_path.supported_msaa(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_supersampling_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.ssaa(),
                        options: self.capabilities_path.ssaa_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_anti_aliasing_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.screen_space_anti_aliasing(),
                        options: self.capabilities_path.screen_space_anti_aliasing_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_shadow_method_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_method(),
                        options: self.capabilities_path.shadow_method_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_shadow_detail_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_detail(),
                        options: self.capabilities_path.shadow_detail_options(),
                    }
                )
            },
            split! {
                children: (
                    text! {
                        text: client_state().localization().ui().graphics_shadow_resolution_label(),
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    drop_down! {
                        selected: self.settings_path.shadow_resolution(),
                        options: self.capabilities_path.shadow_resolution_options(),
                    }
                )
            },
            state_button! {
                text: client_state().localization().ui().graphics_sdsm_label(),
                state: self.settings_path.sdsm(),
                event: Toggle(self.settings_path.sdsm()),
            },
            state_button! {
                text: client_state().localization().ui().graphics_high_quality_interface_label(),
                state: self.settings_path.high_quality_interface(),
                event: Toggle(self.settings_path.high_quality_interface()),
            },
        );

        window! {
            title: client_state().localization().graphics_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            elements,
        }
    }
}
