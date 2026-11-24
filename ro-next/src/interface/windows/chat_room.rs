//! Minimal chat-room window for the 2019 rAthena chat-room packets.
//!
//! The window has two faces, selected by [`ChatRoom::is_in_room`]:
//!
//! * not in a room: a create form plus the list of rooms announced by
//!   `ZC_ROOM_NEWENTRY` (0x00D7),
//! * in a room: the member list, the room chat and a leave button.
//!
//! Requests use the 2019 client packets `CZ_CREATE_CHATROOM` (0x00D5),
//! `CZ_REQ_ENTER_ROOM` (0x00D9) and `CZ_EXIT_ROOM` (0x00E3); room chat is the
//! normal `CZ_REQUEST_CHAT`/`ZC_NOTIFY_CHAT` path, which rAthena routes to the
//! room while `sd->chatID` is set (`clif_parse_GlobalMessage`,
//! clif.cpp:11455-11494). The window is opened from the menu and from the
//! character context menu.

use ro_next_interface::application::Size;
use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use super::WindowClass;
use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::loaders::OverflowBehavior;
use crate::state::content_windows::{ChatRoom, ChatRoomPathExt, DEFAULT_CHAT_ROOM_LIMIT, MAXIMUM_CHAT_ROOM_MESSAGES};
use crate::state::theme::{
    ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, SpacingTokensPathExt, TypographyTokensPathExt,
};
use crate::state::{ClientState, client_theme};

/// Maximum length of the create-title text box (the packet is variable length,
/// but the server stores at most `CHATROOM_TITLE_SIZE - 1` characters).
const MAXIMUM_CHAT_ROOM_TITLE_LENGTH: usize = 36;
/// Maximum length of a chat-room message. The server chat size is 80; leaving
/// room for the `"name : "` prefix keeps the packet valid.
const MAXIMUM_CHAT_ROOM_MESSAGE_LENGTH: usize = 70;
/// Maximum length of the join password (`CHATROOM_PASS_SIZE - 1`).
const MAXIMUM_CHAT_ROOM_PASSWORD_LENGTH: usize = 7;

/// Focus ids of the three text boxes of the window.
pub struct ChatRoomMessageBox;
/// Focus id of the create-title box.
pub struct ChatRoomTitleBox;
/// Focus id of the join-password box.
pub struct ChatRoomPasswordBox;

struct TextLine {
    text: String,
    color: Color,
    height: f32,
}

struct TextLinesLayout {
    area: Area,
    lines: Vec<TextLine>,
}

/// Lays out the member list and the message history of the open room.
///
/// The content is read from the [`ChatRoom`] state every frame, so room events
/// (member joins/leaves, chat lines) do not have to reopen the window.
struct RoomContentElement<A> {
    room_path: A,
}

impl<A> Element<ClientState> for RoomContentElement<A>
where
    A: Path<ClientState, ChatRoom>,
{
    type LayoutInfo = TextLinesLayout;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let tokens = client_theme().tokens();
            let typography = tokens.typography();
            let color_tokens = tokens.color();
            let spacing = tokens.spacing();
            let font_size = *state.get(&typography.font_body());
            let primary = *state.get(&color_tokens.text_primary());
            let secondary = *state.get(&color_tokens.text_secondary());
            let highlight = *state.get(&color_tokens.text_accent());
            let line_spacing = *state.get(&spacing.xs());

            let room = state.get(&self.room_path);
            let mut raw_lines = Vec::new();

            raw_lines.push((
                format!(
                    "Members ({}/{}):",
                    room.members.len(),
                    room.limit.max(room.members.len() as u16)
                ),
                secondary,
            ));

            if room.members.is_empty() {
                raw_lines.push(("  (no members)".to_owned(), secondary));
            } else {
                for member in &room.members {
                    let marker = match member.owner {
                        true => "* ",
                        false => "  ",
                    };
                    raw_lines.push((format!("{marker}{}", member.name), primary));
                }
            }

            raw_lines.push((String::new(), secondary));
            raw_lines.push(("Messages:".to_owned(), secondary));

            if room.messages.is_empty() {
                raw_lines.push(("  (no messages yet)".to_owned(), secondary));
            } else {
                for message in room.messages.iter().rev().take(MAXIMUM_CHAT_ROOM_MESSAGES).rev() {
                    let color = match message.system {
                        true => secondary,
                        false => primary,
                    };
                    raw_lines.push((message.text.clone(), color));
                }
            }

            let mut total_height = 0.0;
            let lines = raw_lines
                .into_iter()
                .map(|(text, color)| {
                    let (size, _) = resolver.get_text_dimensions(
                        &text,
                        color,
                        highlight,
                        font_size,
                        HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
                        OverflowBehavior::LineBreak,
                    );

                    if total_height != 0.0 {
                        total_height += line_spacing;
                    }
                    total_height += size.height();

                    TextLine {
                        text,
                        color,
                        height: size.height(),
                    }
                })
                .collect();

            TextLinesLayout {
                area: resolver.with_height(total_height),
                lines,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let typography = tokens.typography();
        let spacing = tokens.spacing();
        let font_size = *state.get(&typography.font_body());
        let highlight = *state.get(&tokens.color().text_accent());
        let line_spacing = *state.get(&spacing.xs());

        let mut offset = 0.0;
        for line in &layout_info.lines {
            if offset != 0.0 {
                offset += line_spacing;
            }

            let text_area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + offset,
                width: layout_info.area.width,
                height: line.height,
            };

            layout.add_text(
                text_area,
                &line.text,
                font_size,
                line.color,
                highlight,
                HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );

            offset += line.height;
        }
    }
}

struct NearbyRoomRow {
    text: String,
    event: Option<InputEvent>,
    height: f32,
}

struct NearbyRoomsLayout {
    area: Area,
    rows: Vec<NearbyRoomRow>,
}

/// The "nearby rooms" list of the create/join face of the window.
///
/// Rows are rebuilt every frame from the announced room list, and the click
/// event for each row is built in `create_layout_info` so it can be borrowed
/// for the lifetime of the layout in `lay_out`.
struct NearbyRoomsElement<A> {
    room_path: A,
}

impl<A> Element<ClientState> for NearbyRoomsElement<A>
where
    A: Path<ClientState, ChatRoom>,
{
    type LayoutInfo = NearbyRoomsLayout;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let tokens = client_theme().tokens();
            let typography = tokens.typography();
            let color_tokens = tokens.color();
            let spacing = tokens.spacing();
            let font_size = *state.get(&typography.font_body());
            let primary = *state.get(&color_tokens.text_primary());
            let secondary = *state.get(&color_tokens.text_secondary());
            let highlight = *state.get(&color_tokens.text_accent());
            let line_spacing = *state.get(&spacing.xs());

            let room = state.get(&self.room_path);
            let password = room.join_password.clone();

            let mut raw_rows = Vec::new();

            if room.nearby.is_empty() {
                raw_rows.push(("No chat rooms nearby. Create one above.".to_owned(), secondary, None));
            } else {
                for summary in &room.nearby {
                    let event = InputEvent::JoinChatRoom {
                        chat_id: summary.chat_id,
                        title: summary.title.clone(),
                        password: password.clone(),
                    };
                    raw_rows.push((summary.display(), primary, Some(event)));
                }
            }

            let mut total_height = 0.0;
            let rows = raw_rows
                .into_iter()
                .map(|(text, color, event)| {
                    let (size, _) = resolver.get_text_dimensions(
                        &text,
                        color,
                        highlight,
                        font_size,
                        HorizontalAlignment::Left { offset: 12.0, border: 4.0 },
                        OverflowBehavior::LineBreak,
                    );

                    if total_height != 0.0 {
                        total_height += line_spacing;
                    }
                    total_height += size.height().max(font_size.0 + 8.0);

                    NearbyRoomRow {
                        text,
                        event,
                        height: size.height().max(font_size.0 + 8.0),
                    }
                })
                .collect();

            NearbyRoomsLayout {
                area: resolver.with_height(total_height),
                rows,
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let tokens = client_theme().tokens();
        let typography = tokens.typography();
        let spacing = tokens.spacing();
        let color_tokens = tokens.color();
        let font_size = *state.get(&typography.font_body());
        let highlight = *state.get(&color_tokens.text_accent());
        let hover_background = *state.get(&color_tokens.state_hover());
        let line_spacing = *state.get(&spacing.xs());

        let mut offset = 0.0;
        for row in &layout_info.rows {
            if offset != 0.0 {
                offset += line_spacing;
            }

            let row_area = Area {
                left: layout_info.area.left,
                top: layout_info.area.top + offset,
                width: layout_info.area.width,
                height: row.height,
            };

            if let Some(event) = &row.event
                && row_area.check().run(layout)
            {
                layout.add_rectangle(
                    row_area,
                    CornerDiameter::uniform(4.0),
                    hover_background,
                    Color::TRANSPARENT,
                    ShadowPadding::uniform(0.0),
                );
                layout.register_click_handler(MouseButton::Left, event);
            }

            layout.add_text(
                row_area,
                &row.text,
                font_size,
                row.event.as_ref().map_or(*state.get(&color_tokens.text_secondary()), |_| {
                    *state.get(&color_tokens.text_primary())
                }),
                highlight,
                HorizontalAlignment::Left { offset: 12.0, border: 4.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );

            offset += row.height;
        }
    }
}

pub struct ChatRoomWindow<A> {
    room_path: A,
}

impl<A> ChatRoomWindow<A> {
    pub fn new(room_path: A) -> Self {
        Self { room_path }
    }
}

impl<A> CustomWindow<ClientState> for ChatRoomWindow<A>
where
    A: Path<ClientState, ChatRoom>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ChatRoom)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let title_path = self.room_path.clone();
        let title = ComputedSelector::new_default(move |state: &ClientState| title_path.follow_safe(state).display_title());

        let create_title_path = self.room_path.create_title();
        let create_action_path = self.room_path.create_title();
        let create_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let title = state.get(&create_action_path).trim().to_owned();

            if title.is_empty() {
                return;
            }

            state.update_value_with(create_action_path, |current| current.clear());
            queue.queue(InputEvent::CreateChatRoom {
                title,
                limit: DEFAULT_CHAT_ROOM_LIMIT,
            });
            queue.queue(Event::Unfocus);
        };
        let create_button_action = create_action.clone();

        let message_path = self.room_path.message_text();
        let message_action_path = self.room_path.message_text();
        let send_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&message_action_path).trim().to_owned();

            if text.is_empty() {
                return;
            }

            state.update_value_with(message_action_path, |current| current.clear());
            queue.queue(InputEvent::SendChatRoomMessage { text });
            queue.queue(Event::Unfocus);
        };
        let send_button_action = send_action.clone();

        let password_path = self.room_path.join_password();
        let error_path = self.room_path.error();
        let error = ComputedSelector::new_default(move |state: &ClientState| {
            let error = error_path.follow_safe(state);
            match error.is_empty() {
                true => String::new(),
                false => error.clone(),
            }
        });

        window! {
            title,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 420.0,
            maximum_width: 620.0,
            minimum_height: 260.0,
            maximum_height: 620.0,
            elements: either! {
                selector: self.room_path.in_room(),
                on_true: (
                    scroll_view! {
                        follow: true,
                        fit_content_height: false,
                        children: RoomContentElement {
                            room_path: self.room_path.clone(),
                        },
                    },
                    split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text_box! {
                                ghost_text: "Chat message",
                                state: message_path,
                                input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_ROOM_MESSAGE_LENGTH>::new(message_path, send_action),
                                focus_id: ChatRoomMessageBox,
                            },
                            button! {
                                text: "Send",
                                event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                    send_button_action(state, queue);
                                },
                            },
                        ),
                    },
                    button! {
                        text: "Leave chat room",
                        event: InputEvent::LeaveChatRoom,
                    },
                ),
                on_false: (
                    text! {
                        text: "Create a chat room",
                    },
                    text_box! {
                        ghost_text: "Room title",
                        state: create_title_path,
                        input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_ROOM_TITLE_LENGTH>::new(create_title_path, create_action),
                        focus_id: ChatRoomTitleBox,
                    },
                    button! {
                        text: "Create",
                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            create_button_action(state, queue);
                        },
                    },
                    text! {
                        text: "Nearby chat rooms",
                    },
                    text_box! {
                        ghost_text: "Password for private rooms",
                        state: password_path,
                        input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_ROOM_PASSWORD_LENGTH>::new(password_path, |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {}),
                        focus_id: ChatRoomPasswordBox,
                    },
                    scroll_view! {
                        follow: true,
                        fit_content_height: false,
                        children: NearbyRoomsElement {
                            room_path: self.room_path.clone(),
                        },
                    },
                    text! {
                        text: error,
                    },
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearby_room_display_marks_private_rooms() {
        let private = crate::state::content_windows::ChatRoomSummary {
            owner_id: ragnarok_packets::AccountId(1),
            chat_id: 1,
            title: "Trade".to_owned(),
            users: 2,
            limit: 20,
            room_type: 0,
        };
        assert_eq!(private.display(), "Trade [private] (2/20)");
    }

    #[test]
    fn data_driven_limits_are_sane() {
        assert!(MAXIMUM_CHAT_ROOM_MESSAGE_LENGTH < 80);
        assert!(MAXIMUM_CHAT_ROOM_TITLE_LENGTH > 0);
    }
}
