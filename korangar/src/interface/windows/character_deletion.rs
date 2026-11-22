//! Confirmation window for deleting a character.
//!
//! The CondaRO character server runs rAthena with `char_del_option: 1`, which
//! means the client has to send the account e-mail address as the deletion
//! confirmation code (`CHAR_DEL_EMAIL`). The login server never tells the
//! client the account e-mail address, so the user has to type it in here.
//! Sending the old hardcoded placeholder `a@a.com` made the server refuse every
//! deletion request.

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::CharacterId;
use rust_state::{Path, PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Maximum length of the e-mail address.
///
/// The wire field is a fixed 40-byte C string and the server compares it with
/// `stricmp`, so the string must stay zero terminated.
const MAXIMUM_EMAIL_LENGTH: usize = 39;

pub struct CharacterDeletionWindow<A> {
    character_id: CharacterId,
    email_path: A,
}

impl<A> CharacterDeletionWindow<A> {
    pub fn new(character_id: CharacterId, email_path: A) -> Self {
        Self { character_id, email_path }
    }
}

impl<A> CustomWindow<ClientState> for CharacterDeletionWindow<A>
where
    A: Path<ClientState, String>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterDeletion)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        struct EmailTextBox;

        let email_path = self.email_path;
        let character_id = self.character_id;

        let disabled = ComputedSelector::new_default(move |state: &ClientState| email_path.follow_safe(state).trim().is_empty());

        let delete_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let email = state.get(&email_path).trim().to_owned();

            if email.is_empty() {
                return;
            }

            queue.queue(InputEvent::DeleteCharacter { character_id, email });
        };

        window! {
            title: client_state().localization().delete_character_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            closable: true,
            minimum_width: 420.0,
            maximum_width: 420.0,
            elements: (
                text! {
                    text: "Deleting a character is permanent. To confirm, enter the e-mail address of this account.",
                },
                text_box! {
                    ghost_text: client_state().localization().account_email_text(),
                    state: self.email_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_EMAIL_LENGTH>::new(self.email_path, delete_action),
                    focus_id: EmailTextBox,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! {
                    text: client_state().localization().delete_character_window_title(),
                    disabled,
                    disabled_tooltip: "Enter the account e-mail address first",
                    event: delete_action,
                },
            ),
        }
    }
}
