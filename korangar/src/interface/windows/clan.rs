//! Clan window for the 2019 rAthena clan system.
//!
//! The 2019 server only sends two clan packets: `ZC_CLANINFO` (0x098A) with
//! the name, master, territory map and ally/antagonist lists, and
//! `ZC_NOTIFY_CLAN_CONNECTINFO` (0x0988) with the online/maximum member count.
//! **There is no clan member roster packet** at PACKETVER 20190605, so the
//! window shows the available information and states the missing member list
//! explicitly (see [`crate::state::clan`]).

use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{PathExt, Selector};

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct ClanWindow;

fn header_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(|state: &ClientState| {
        let clan = client_state().clan().follow_safe(state);
        clan.header_text()
    })
}

fn master_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(|state: &ClientState| {
        let clan = client_state().clan().follow_safe(state);
        clan.master_text()
    })
}

fn relation_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(|state: &ClientState| {
        let clan = client_state().clan().follow_safe(state);
        clan.relation_text()
    })
}

fn member_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(|state: &ClientState| {
        let clan = client_state().clan().follow_safe(state);
        clan.member_text()
    })
}

impl CustomWindow<ClientState> for ClanWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Clan)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Clan",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 420.0,
            maximum_width: 620.0,
            maximum_height: 520.0,
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: (
                    // Force the selector closure to be evaluated in the window
                    // context; the shape mirrors the guild window.
                    text! {
                        text: header_text(),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text! {
                        text: master_text(),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text! {
                        text: relation_text(),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text! {
                        text: member_text(),
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                ),
            },
        }
    }
}
