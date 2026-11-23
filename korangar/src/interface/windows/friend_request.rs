use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::Friend;
use rust_state::PathExt;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt, template_text};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct FriendRequestWindow {
    friend: Friend,
}

impl FriendRequestWindow {
    pub fn new(friend: Friend) -> Self {
        Self { friend }
    }
}

impl CustomWindow<ClientState> for FriendRequestWindow {
    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let friend_request_text = template_text(
            |state| client_state().localization().ui().friend_request_text().follow_safe(state),
            self.friend.name.clone(),
        );

        window! {
            title: client_state().localization().friend_request_window_title(),
            class: Some(WindowClass::FriendRequest),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: friend_request_text,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: client_state().localization().reject_button_text(),
                            event: InputEvent::RejectFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                        button! {
                            text: client_state().localization().accept_button_text(),
                            event: InputEvent::AcceptFriendRequest {
                                account_id: self.friend.account_id,
                                character_id: self.friend.character_id,
                            },
                        },
                    ),
                },
            ),
        }
    }
}
