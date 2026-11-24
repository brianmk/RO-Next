use std::cmp::Ordering;

use ragnarok_packets::{Friend, MIN_NAME_LENGTH, NAME_LENGTH};
use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox, StateElement};
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{ManuallyAssertExt, Path, PathExt, RustState, State, VecIndexExt};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

struct FriendList<A> {
    friend_list_path: A,
    elements: Vec<ElementBox<ClientState>>,
}

impl<A> FriendList<A> {
    fn new(friend_list_path: A) -> Self {
        Self {
            friend_list_path,
            elements: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for FriendList<A>
where
    A: Path<ClientState, Vec<Friend>>,
{
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use ro_next_interface::prelude::*;

            let friend_list = state.get(&self.friend_list_path);

            match friend_list.len().cmp(&self.elements.len()) {
                Ordering::Less => {
                    self.elements.truncate(friend_list.len());
                }
                Ordering::Equal => {}
                Ordering::Greater => {
                    for index in self.elements.len()..friend_list.len() {
                        let friend_path = self.friend_list_path.index(index).manually_asserted();
                        let online_path = client_state().friend_online().index(index).manually_asserted();
                        let text_friend_path = friend_path;
                        let text_online_path = online_path;
                        let color_online_path = online_path;

                        // Row text: status marker + name. The name comes from 0x0206
                        // (online friends), the persistent cache or a clear
                        // `Unknown (#id)` placeholder for offline friends.
                        let display_text = ComputedSelector::new_default(move |state: &ClientState| {
                            let friend = text_friend_path.follow_safe(state);
                            let online = *text_online_path.follow_safe(state);
                            let marker = if online { "\u{25CF}" } else { "\u{25CB}" };
                            format!("{marker} {}", crate::state::friend_names::display_name(friend))
                        });

                        // Green marker/name for online, medium gray for offline; both are
                        // readable on the default dark and the classic light theme.
                        let display_color = ComputedSelector::new_default(move |state: &ClientState| {
                            if *color_online_path.follow_safe(state) {
                                Color::rgb(0.25, 0.85, 0.35)
                            } else {
                                Color::rgb(0.58, 0.58, 0.58)
                            }
                        });

                        self.elements.push(ErasedElement::new(collapsible! {
                            text: display_text,
                            foreground_color: display_color,
                            children: button! {
                                text: client_state().localization().remove_button_text(),
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    let &Friend { account_id, character_id, .. } = state.get(&friend_path);

                                    queue.queue(
                                        InputEvent::RemoveFriend { account_id, character_id }
                                    );
                                },
                            },
                        }));
                    }
                }
            }

            self.elements.iter_mut().zip(friend_list.iter()).for_each(|(element, friend)| {
                element.create_layout_info(state, store.child_store(friend.character_id.0 as u64), resolver);
            });
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let friend_list = state.get(&self.friend_list_path);

        self.elements.iter().zip(friend_list.iter()).for_each(|(element, friend)| {
            element.lay_out(state, store.child_store(friend.character_id.0 as u64), &(), layout);
        });
    }
}

/// Internal state of the chat window.
#[derive(Default, RustState, StateElement)]
pub struct FriendListWindowState {
    currently_adding: String,
}

pub struct FriendListWindow<A, B> {
    window_state_path: A,
    friend_list_path: B,
}

impl<A, B> FriendListWindow<A, B> {
    pub fn new(window_state_path: A, friend_list_path: B) -> Self {
        Self {
            window_state_path,
            friend_list_path,
        }
    }
}

impl<A, B> CustomWindow<ClientState> for FriendListWindow<A, B>
where
    A: Path<ClientState, FriendListWindowState>,
    B: Path<ClientState, Vec<Friend>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::FriendList)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        struct AddFriendTextBox;

        let add_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let character_name = state.get(&self.window_state_path.currently_adding()).clone();

            // TODO: Give some sort of error if the name is too short.
            if character_name.len() >= MIN_NAME_LENGTH {
                state.update_value_with(self.window_state_path.currently_adding(), |input| input.clear());
                queue.queue(InputEvent::AddFriend { character_name });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: client_state().localization().friend_list_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: (
                text_box! {
                    ghost_text: client_state().localization().friend_list_text_box_message(),
                    state: self.window_state_path.currently_adding(),
                    input_handler: DefaultHandler::<_, _, NAME_LENGTH>::new(self.window_state_path.currently_adding(), add_action),
                    focus_id: AddFriendTextBox,
                },
                FriendList::new(self.friend_list_path),
            )
        }
    }
}
