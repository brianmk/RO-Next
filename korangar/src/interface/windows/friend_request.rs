use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::Friend;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
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

        window! {
            title: client_state().localization().friend_request_window_title(),
            class: Some(WindowClass::FriendRequest),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text! {
                    text: format!("^000001{}^000000 wants to be friends with you", self.friend.name),
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
