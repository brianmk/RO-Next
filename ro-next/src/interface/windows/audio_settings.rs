use ro_next_interface::window::{CustomWindow, Window};
use rust_state::Path;

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{AudioSettings, AudioSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

#[derive(Default)]
pub struct AudioSettingsWindow<A> {
    audio_settings_path: A,
}

impl<A> AudioSettingsWindow<A> {
    pub fn new(audio_settings_path: A) -> Self {
        Self { audio_settings_path }
    }
}

impl<A> CustomWindow<ClientState> for AudioSettingsWindow<A>
where
    A: Path<ClientState, AudioSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::AudioSettings)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: client_state().localization().audio_settings_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            elements: (
                state_button! {
                    text: client_state().localization().mute_audio_on_focus_loss_button_text(),
                    state: self.audio_settings_path.mute_on_focus_loss(),
                    event: Toggle(self.audio_settings_path.mute_on_focus_loss()),
                },
                state_button! {
                    text: client_state().localization().mute_background_music_button_text(),
                    state: self.audio_settings_path.background_music_muted(),
                    event: Toggle(self.audio_settings_path.background_music_muted()),
                },
                split! {
                    children: (
                        text! {
                            text: client_state().localization().master_volume_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        slider! {
                            value: self.audio_settings_path.master_volume(),
                        }
                    )
                },
                split! {
                    children: (
                        text! {
                            text: client_state().localization().sound_effect_volume_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        slider! {
                            value: self.audio_settings_path.sound_effect_volume(),
                        }
                    )
                },
                split! {
                    children: (
                        text! {
                            text: client_state().localization().background_music_volume_text(),
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        slider! {
                            value: self.audio_settings_path.background_music_volume(),
                        }
                    )
                },
            ),
        }
    }
}
