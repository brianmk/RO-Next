use ragnarok_packets::{NAME_LENGTH, PASSWORD_LENGTH};
use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::StateElement;
use ro_next_interface::element::id::FocusIdExt;
use ro_next_interface::event::{ClickHandler, EventQueue};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, RustState, State};

use crate::graphics::{Color, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{ClientInfo, ClientInfoPathExt, OverflowBehavior, ServiceId};
use crate::settings::{LoginSettings, LoginSettingsPathExt, ServiceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, SelectedServicePath, client_state};

/// Click handler that toggles a boolean and *immediately* persists the login
/// settings.
///
/// `LoginSettings` only saves itself when it is dropped, i.e. on a clean
/// shutdown. If the client is killed (terminal closed, SIGHUP/SIGKILL, crash)
/// the remembered credentials are lost. Persisting on every toggle makes the
/// checkbox effective right away, independently of how the client exits.
struct PersistingToggle<V, S> {
    value_path: V,
    settings_path: S,
}

impl<V, S> ClickHandler<ClientState> for PersistingToggle<V, S>
where
    V: Path<ClientState, bool>,
    S: Path<ClientState, LoginSettings>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        state.update_value_with(self.value_path, |value| *value = !*value);
        state.update_value_with(self.settings_path, |settings| settings.save());
    }
}

/// Internal state of the login window.
#[derive(RustState, StateElement)]
pub struct LoginWindowState {
    selected_service: ServiceId,
}

impl LoginWindowState {
    pub fn new(selected_service: ServiceId) -> Self {
        Self { selected_service }
    }
}

pub struct LoginWindow<A, B, C> {
    window_state_path: A,
    service_settings_path: B,
    client_info_path: C,
}

impl<A, B, C> LoginWindow<A, B, C> {
    pub fn new(window_state_path: A, service_settings_path: B, client_info_path: C) -> Self {
        Self {
            window_state_path,
            service_settings_path,
            client_info_path,
        }
    }
}

impl<A, B, C> CustomWindow<ClientState> for LoginWindow<A, B, C>
where
    A: Path<ClientState, LoginWindowState>,
    B: Path<ClientState, LoginSettings>,
    C: Path<ClientState, ClientInfo>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Login)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let selected_service_path = SelectedServicePath::new(self.window_state_path, self.service_settings_path);
        let username_path = selected_service_path.username();
        let password_path = selected_service_path.password();
        let remember_username_path = selected_service_path.remember_username();
        let remember_password_path = selected_service_path.remember_password();

        // Toggling a "remember" checkbox has to persist immediately: otherwise the
        // state is only written when the client shuts down cleanly, and any
        // crash/kill loses it.
        let remember_username_toggle = PersistingToggle {
            value_path: remember_username_path,
            settings_path: self.service_settings_path,
        };
        let remember_password_toggle = PersistingToggle {
            value_path: remember_password_path,
            settings_path: self.service_settings_path,
        };

        let disabled_selector = ComputedSelector::new_default(move |state: &ClientState| {
            selected_service_path.username().follow_safe(state).is_empty() || selected_service_path.password().follow_safe(state).is_empty()
        });

        let login_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let selected_service_path = self.window_state_path.selected_service();
            let selected_service_id = state.get(&selected_service_path);
            let username = state.get(&username_path);
            let password = state.get(&password_path);

            // Remember which service was selected so we can select it next time the client
            // starts.
            state.update_value(self.service_settings_path.recent_service_id(), Some(*selected_service_id));

            // Persist the (possibly freshly typed) credentials immediately. `LoginSettings`
            // otherwise only writes itself on a clean shutdown, so a crash or a killed
            // process would silently lose the remembered login data.
            state.update_value_with(self.service_settings_path, |settings| settings.save());

            queue.queue(InputEvent::LogIn {
                service_id: *selected_service_id,
                username: username.clone(),
                password: password.clone(),
            });
        };

        struct UsernameTextBox;
        struct PasswordTextBox;

        let username_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            if !state.get(&username_path).is_empty() {
                queue.queue(Event::FocusElement {
                    focus_id: PasswordTextBox.focus_id(),
                });
            }
        };

        let password_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let username = state.get(&username_path);

            if username.is_empty() {
                queue.queue(Event::FocusElement {
                    focus_id: UsernameTextBox.focus_id(),
                });
                return;
            }

            login_action(state, queue);
        };

        window! {
            title: client_state().localization().log_in_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: 450.0,
            maximum_width: 450.0,
            elements: (
                text! { text: client_state().localization().select_service_text() },
                drop_down! {
                    options: self.client_info_path.services(),
                    selected: self.window_state_path.selected_service(),
                    click_handler: DefaultClickHandler::new(self.window_state_path.selected_service(), self.client_info_path.services()),
                },
                text! { text: client_state().localization().account_data_text() },
                fragment! {
                    // TODO: Theme this
                    gaps: 8.0,
                    children: (
                        text_box! {
                            ghost_text: client_state().localization().username_text(),
                            state: username_path,
                            input_handler: DefaultHandler::<_, _, NAME_LENGTH>::new(username_path, username_action),
                            focus_id: UsernameTextBox,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: client_state().localization().password_text(),
                            state: password_path,
                            input_handler: DefaultHandler::<_, _, PASSWORD_LENGTH>::new(password_path, password_action),
                            focus_id: PasswordTextBox,
                            hidable: true,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                    ),
                },
                fragment! {
                    // TODO: Theme this
                    gaps: 8.0,
                    children: (
                        state_button! {
                            text: client_state().localization().remember_username_text(),
                            state: remember_username_path,
                            event: remember_username_toggle,
                            background_color: Color::TRANSPARENT,
                            hovered_background_color: Color::TRANSPARENT,
                            shadow_padding: ShadowPadding::default(),
                        },
                        state_button! {
                            text: client_state().localization().remember_password_text(),
                            state: remember_password_path,
                            event: remember_password_toggle,
                            background_color: Color::TRANSPARENT,
                            hovered_background_color: Color::TRANSPARENT,
                            shadow_padding: ShadowPadding::default(),
                        },
                    ),
                },
                button! {
                    text: client_state().localization().log_in_button_text(),
                    disabled: disabled_selector,
                    disabled_tooltip: client_state().localization().log_in_button_tooltip(),
                    event: login_action,
                },
            ),
        }
    }
}
