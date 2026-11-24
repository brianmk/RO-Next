//! Optional pincode prompt of the character-selection flow.
//!
//! The 2019 character server sends `HC_ACK_PINCODE` (0x08B9,
//! `char_clif.cpp:160-167`) with a seed and a state. CondaRO runs with
//! `pincode_enabled: no`, so that packet carries state `0` (passed) and this
//! window never opens; for a server that enables the feature the prompt asks
//! for the four digit PIN and answers with `CZ_PINCODE_CHECK` (0x08B8) or
//! `CZ_PINCODE_SET_NEW` (0x08BA).
//!
//! The PIN itself is obfuscated with the packet seed by
//! [`crate::pincode::encrypt_pincode`], never sent as plain digits.

use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::content_windows::{PincodePrompt, PincodePromptPathExt};
use crate::state::theme::InterfaceThemeType;

/// Input handler capacity of the PIN text box (`PINCODE_LENGTH`, 4).
const MAXIMUM_PINCODE_LENGTH: usize = 4;

/// Focus id of the PIN text box.
pub struct PincodeTextBox;

pub struct PincodeWindow<A> {
    prompt_path: A,
}

impl<A> PincodeWindow<A> {
    pub fn new(prompt_path: A) -> Self {
        Self { prompt_path }
    }
}

impl<A> CustomWindow<ClientState> for PincodeWindow<A>
where
    A: Path<ClientState, PincodePrompt>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Pincode)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let code_path = self.prompt_path.code();
        let code_input_path = self.prompt_path.code();
        let is_new_path = self.prompt_path.is_new();
        let error_path = self.prompt_path.error();

        let message = ComputedSelector::new_default(move |state: &ClientState| {
            let is_new = is_new_path.follow_safe(state);
            match is_new {
                true => "The character server asks you to set a new 4 digit PIN.".to_owned(),
                false => "The character server asks for your 4 digit PIN.".to_owned(),
            }
        });

        let error = ComputedSelector::new_default(move |state: &ClientState| {
            let error = error_path.follow_safe(state);
            match error.is_empty() {
                true => String::new(),
                false => error.clone(),
            }
        });

        let submit_action = move |state: &State<ClientState>, queue: &mut ro_next_interface::event::EventQueue<ClientState>| {
            let code = state.get(&code_input_path).clone();

            if code.len() != MAXIMUM_PINCODE_LENGTH || !code.bytes().all(|byte| byte.is_ascii_digit()) {
                return;
            }

            queue.queue(InputEvent::SubmitPincode { code });
            queue.queue(ro_next_interface::event::Event::Unfocus);
        };
        let submit_button_action = submit_action.clone();

        window! {
            title: "Enter PIN",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: false,
            minimum_width: 340.0,
            maximum_width: 420.0,
            elements: (
                text! {
                    text: message,
                },
                text_box! {
                    ghost_text: "4 digits",
                    state: code_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_PINCODE_LENGTH>::new(code_input_path, submit_action),
                    focus_id: PincodeTextBox,
                },
                text! {
                    text: error,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Cancel",
                            event: InputEvent::ClosePincodePrompt,
                        },
                        button! {
                            text: "Submit",
                            event: move |state: &State<ClientState>, queue: &mut ro_next_interface::event::EventQueue<ClientState>| {
                                submit_button_action(state, queue);
                            },
                        },
                    ),
                },
            ),
        }
    }
}
