use korangar_interface::application::{Application, Size};
use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::element::{Element, StateElement};
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::{Area, PartialArea};
use korangar_interface::layout::{Resolver, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::MessageColor;
use rust_state::{Path, PathExt, RustState, Selector, State};

use super::WindowClass;
use crate::graphics::{Color, ShadowPadding};
use crate::input::InputEvent;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{
    ChatTheme, ColorTokens, ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, SpacingTokensPathExt,
    TypographyTokensPathExt,
};
use crate::state::{ChatMessage, ClientState, ClientStatePathExt, client_state, client_theme};

const MAXIMUM_CHAT_MESSAGE_LENGTH: usize = 80;

/// Vertical gap of the chat window, shared by the window layout and the
/// message/input wrapper so the input row always fits exactly.
const CHAT_WINDOW_GAPS: f32 = 2.0;

/// Resolve the color of a chat message through the design tokens.
///
/// Server-colored messages keep their RGB color from the network, every other
/// class uses the semantic token so a theme can style it.
fn message_color<P>(state: &State<ClientState>, tokens: P, message_color: MessageColor) -> Color
where
    P: Path<ClientState, ColorTokens> + Copy,
{
    match message_color {
        MessageColor::Rgb { red, green, blue } => Color::rgb_u8(red, green, blue),
        MessageColor::Broadcast => *state.get(&tokens.chat_broadcast()),
        MessageColor::Server => *state.get(&tokens.chat_server()),
        MessageColor::Information => *state.get(&tokens.chat_information()),
        MessageColor::Error => *state.get(&tokens.chat_error()),
    }
}

/// ZST for getting the focus id of the chat text box. This is only needed to
/// focus the chat when pressing enter.
pub struct ChatTextBox;

struct ChatLayoutInfo {
    area: Area,
    // TODO: Don't allocate this every frame.
    message_heights: Vec<f32>,
}

struct ChatElement<A> {
    chat_messages_path: A,
}

impl<A> ChatElement<A> {
    fn new(chat_messages_path: A) -> Self {
        Self { chat_messages_path }
    }
}

impl<A> Element<ClientState> for ChatElement<A>
where
    A: Path<ClientState, Vec<ChatMessage>>,
{
    type LayoutInfo = ChatLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let chat_messages = state.get(&self.chat_messages_path);
            let tokens = client_theme().tokens();
            let color_tokens = tokens.color();
            let typography = tokens.typography();
            let spacing = tokens.spacing();

            let message_spacing = *state.get(&spacing.chat_message_gap());
            let message_font_size = *state.get(&typography.font_body_lg());
            let highlight_color = *state.get(&color_tokens.text_overlay_accent());
            let message_alignment = HorizontalAlignment::Left {
                offset: *state.get(&spacing.lg()),
                border: *state.get(&spacing.text_border()),
            };

            let mut total_height = 0.0;
            let message_heights = chat_messages
                .iter()
                .map(|chat_message| {
                    let color = message_color(state, color_tokens, chat_message.color);

                    let (size, _) = resolver.get_text_dimensions(
                        &chat_message.text,
                        color,
                        highlight_color,
                        message_font_size,
                        message_alignment,
                        OverflowBehavior::LineBreak,
                    );

                    if total_height != 0.0 {
                        total_height += message_spacing;
                    }

                    total_height += size.height();

                    size.height()
                })
                .collect();

            let area = resolver.with_height(total_height);

            Self::LayoutInfo { area, message_heights }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let chat_messages = state.get(&self.chat_messages_path);
        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let spacing = tokens.spacing();

        let message_spacing = *state.get(&spacing.chat_message_gap());
        let message_font_size = *state.get(&typography.font_body_lg());
        let highlight_color = *state.get(&color_tokens.text_overlay_accent());
        let message_alignment = HorizontalAlignment::Left {
            offset: *state.get(&spacing.lg()),
            border: *state.get(&spacing.text_border()),
        };

        let mut offset = 0.0;
        chat_messages
            .iter()
            .zip(layout_info.message_heights.iter())
            .for_each(|(chat_message, message_height)| {
                let color = message_color(state, color_tokens, chat_message.color);

                if offset != 0.0 {
                    offset += message_spacing;
                }

                let text_area = Area {
                    left: layout_info.area.left,
                    top: layout_info.area.top + offset,
                    width: layout_info.area.width,
                    height: *message_height,
                };

                layout.add_text(
                    text_area,
                    &chat_message.text,
                    message_font_size,
                    color,
                    highlight_color,
                    message_alignment,
                    VerticalAlignment::Center { offset: 0.0 },
                    OverflowBehavior::LineBreak,
                );

                offset += message_height;
            });
    }
}

/// Chat message list plus the input row at the bottom.
///
/// The messages scroll view gets exactly the available height minus the input
/// row (its theme height plus one window gap), the input row gets the
/// remainder. The wrapper consumes the full available height, so the chat
/// window keeps its stored size while resizing from the top and the input row
/// always stays inside the window (the previous "reserve below" wrapper
/// consumed only the scroll view and let the input row fall below the window
/// bottom).
struct ChatInputResolvers<'a, App: Application> {
    available_area: PartialArea,
    messages_height: f32,
    input_height: f32,
    gaps: f32,
    used: [f32; 2],
    current_index: std::cell::Cell<usize>,
    text_layouter: &'a App::TextLayouter,
}

impl<'a, App> ChatInputResolvers<'a, App>
where
    App: Application,
{
    fn new(available_area: PartialArea, messages_height: f32, input_height: f32, gaps: f32, text_layouter: &'a App::TextLayouter) -> Self {
        Self {
            available_area,
            messages_height,
            input_height,
            gaps,
            used: [0.0; 2],
            current_index: std::cell::Cell::new(0),
            text_layouter,
        }
    }
}

impl<'a, App> Resolvers<'a, App> for ChatInputResolvers<'a, App>
where
    App: Application,
{
    fn for_index(&self, index: usize) -> Resolver<'a, App> {
        self.current_index.set(index);

        let (top, height) = match index {
            0 => (self.available_area.top, self.messages_height),
            _ => (self.available_area.top + self.messages_height + self.gaps, self.input_height),
        };

        Resolver::new(
            PartialArea {
                left: self.available_area.left,
                top,
                width: self.available_area.width,
                height: Some(height),
            },
            0.0,
            self.text_layouter,
        )
    }

    fn give_back(&mut self, resolver: Resolver<'a, App>) {
        self.used[self.current_index.get()] = resolver.get_used_height();
    }
}

/// Chat message list plus the input row at the bottom.
///
/// The message list gets `available - input_row_height - gap`, the input row
/// gets `input_row_height` at the bottom. The wrapper consumes exactly the
/// available height, so the window keeps its stored size while resizing from
/// the top and the input row can never end up below the window bottom.
struct ChatMessagesWithInput<Children, InputHeight, Gaps> {
    input_height: InputHeight,
    gaps: Gaps,
    children: Children,
}

impl<Children, InputHeight, Gaps> ChatMessagesWithInput<Children, InputHeight, Gaps> {
    fn new(input_height: InputHeight, gaps: Gaps, children: Children) -> Self {
        Self {
            input_height,
            gaps,
            children,
        }
    }
}

impl<App, Children, InputHeight, Gaps> Element<App> for ChatMessagesWithInput<Children, InputHeight, Gaps>
where
    App: Application,
    Children: Element<App>,
    InputHeight: Selector<App, f32>,
    Gaps: Selector<App, f32>,
{
    type LayoutInfo = Children::LayoutInfo;

    fn create_layout_info(&mut self, state: &State<App>, store: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let available_area = resolver.push_available_area();
            let available_height = available_area.height.unwrap_or(0.0);
            let input_height = *state.get(&self.input_height);
            let gaps = *state.get(&self.gaps);
            let messages_height = (available_height - input_height - gaps).max(0.0);

            let mut child_resolvers = ChatInputResolvers::new(
                available_area,
                messages_height,
                input_height,
                gaps,
                resolver.get_text_layouter(),
            );
            let layout_info = self.children.create_layout_info(state, store, &mut child_resolvers as _);

            let used_height = child_resolvers.used[0] + gaps + child_resolvers.used[1];
            resolver.commit_used_height(used_height);

            layout_info
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        self.children.lay_out(state, store, layout_info, layout);
    }
}

/// Internal state of the chat window.
#[derive(Default, RustState, StateElement)]
pub struct ChatWindowState {
    current_text: String,
}

impl ChatWindowState {
    /// Replace the text in the chat input, e.g. to pre-fill a whisper command.
    /// The cursor is placed at the end of the text by the text box.
    pub fn set_current_text(&mut self, text: String) {
        self.current_text = text;
    }
}

pub struct ChatWindow<A, B, C> {
    chat_window_state: A,
    chat_messages_path: B,
    interface_settings_path: C,
}

impl<A, B, C> ChatWindow<A, B, C> {
    pub fn new(chat_window_state: A, chat_messages_path: B, interface_settings_path: C) -> Self {
        Self {
            chat_window_state,
            chat_messages_path,
            interface_settings_path,
        }
    }
}

/// Click handler for the "transparent chat" toggle in the chat window.
///
/// Besides toggling the state (which is read every frame, so the change is
/// applied live) the interface settings are saved immediately, so the choice
/// survives crashes like the other interface settings.
struct TransparentChatToggle<A, B> {
    value_path: A,
    settings_path: B,
}

impl<A, B> ClickHandler<ClientState> for TransparentChatToggle<A, B>
where
    A: Path<ClientState, bool>,
    B: Path<ClientState, InterfaceSettings>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        state.update_value_with(self.value_path, |transparent| *transparent = !*transparent);
        state.update_value_with(self.settings_path, |settings| settings.save());
    }
}

/// Build a selector for a chat theme color that depends on the "transparent
/// chat" setting.
fn chat_color<P, T, F>(chat_theme_path: P, transparent_path: T, select: F) -> impl Selector<ClientState, Color>
where
    P: Path<ClientState, ChatTheme> + Copy + 'static,
    T: Path<ClientState, bool> + Copy + 'static,
    F: Fn(&ChatTheme, bool) -> Color + 'static,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        select(chat_theme_path.follow_safe(state), *transparent_path.follow_safe(state))
    })
}

/// Build a selector for a chat theme color that doesn't depend on the
/// "transparent chat" setting.
fn chat_color_constant<P, F>(chat_theme_path: P, select: F) -> impl Selector<ClientState, Color>
where
    P: Path<ClientState, ChatTheme> + Copy + 'static,
    F: Fn(&ChatTheme) -> Color + 'static,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| select(chat_theme_path.follow_safe(state)))
}

impl<A, B, C> CustomWindow<ClientState> for ChatWindow<A, B, C>
where
    A: Path<ClientState, ChatWindowState>,
    B: Path<ClientState, Vec<ChatMessage>>,
    C: Path<ClientState, InterfaceSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Chat)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let current_text_path = self.chat_window_state.current_text();
        let transparent_chat_path = self.interface_settings_path.transparent_chat();
        let chat_theme_path = client_theme().chat();

        let window_background_color = chat_color(chat_theme_path, transparent_chat_path, |chat, transparent| match transparent {
            true => chat.transparent_window_color,
            false => chat.window_color,
        });
        let window_title_color = chat_color_constant(chat_theme_path, |chat| chat.window_title_color);
        let hovered_window_title_color = chat_color_constant(chat_theme_path, |chat| chat.hovered_window_title_color);
        // The chat input row is always drawn on the dark
        // `chat_input_focused_background` token (see below), in both the
        // classic and the default theme. Use white text and a light ghost text
        // so the input stays readable everywhere, including the
        // transparent-chat variant.
        let text_box_foreground_color = korangar_interface::prelude::ComputedSelector::new_default(|_: &ClientState| Color::WHITE);
        // The focused chat input is drawn on a dark background; using the skinned
        // text box theme colors here would make the text unreadable with the classic
        // theme (which uses black text on light text boxes).
        let text_box_focused_foreground_color = chat_color_constant(chat_theme_path, |chat| chat.text_box_focused_foreground_color);
        let text_box_ghost_foreground_color =
            korangar_interface::prelude::ComputedSelector::new_default(|_: &ClientState| Color::rgba_u8(205, 205, 205, 230));
        // The chat input row (focused or not) is always drawn on a dark
        // background, also in the light classic theme, so the background is a
        // token and not one of the skinned text box colors.
        let chat_input_background_color = {
            let tokens = client_theme().tokens();
            let color_tokens = tokens.color();
            korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
                *color_tokens.chat_input_focused_background().follow_safe(state)
            })
        };

        let send_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&current_text_path);

            if !text.is_empty() {
                // Clear the text box.
                state.update_value_with(current_text_path, |current_text| current_text.clear());
                queue.queue(InputEvent::SendMessage { text: text.clone() });
                queue.queue(Event::Unfocus);
            }
        };

        window! {
            title: client_state().localization().chat_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            title_color: window_title_color,
            hovered_title_color: hovered_window_title_color,
            background_color: window_background_color,
            resizable: true,
            border: 3.0,
            gaps: CHAT_WINDOW_GAPS,
            title_gap: 0.0,
            minimum_height: 150.0,
            maximum_height: 800.0,
            // The chat window is an overlay: it uses its own (semi transparent)
            // background colour and must not draw the opaque skinned window frame.
            skinned: false,
            // The input box sits at the bottom, so the vertical resize grip belongs
            // on the top edge.
            vertical_resize_anchor: korangar_interface::window::VerticalResizeAnchor::Top,
            elements: (
                round_toggle! {
                    tooltip: client_state().localization().chat_transparency_button_tooltip(),
                    state: transparent_chat_path,
                    event: TransparentChatToggle {
                        value_path: transparent_chat_path,
                        settings_path: self.interface_settings_path,
                    },
                    alignment: HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
                    shadow_color: Color::TRANSPARENT,
                    shadow_padding: ShadowPadding::uniform(0.0),
                },
                ChatMessagesWithInput::new(
                    korangar_interface::theme::theme().text_box().height(),
                    korangar_interface::prelude::ComputedSelector::new_default(|_: &ClientState| CHAT_WINDOW_GAPS),
                    (
                        scroll_view! {
                            follow: true,
                            // The window height comes from the stored window size
                            // (and the resize grip changes it), so the message list
                            // has to consume the available height and scroll beyond
                            // it instead of stretching the window with the content.
                            fit_content_height: false,
                            children: ChatElement::new(self.chat_messages_path),
                        },
                        text_box! {
                            ghost_text: client_state().localization().chat_text_box_message(),
                            state: current_text_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_CHAT_MESSAGE_LENGTH>::new(current_text_path, send_action),
                            foreground_color: text_box_foreground_color,
                            focused_foreground_color: text_box_focused_foreground_color,
                            background_color: chat_input_background_color,
                            hovered_background_color: {
                                let tokens = client_theme().tokens();
                                let color_tokens = tokens.color();
                                korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
                                    *color_tokens.chat_input_focused_background().follow_safe(state)
                                })
                            },
                            hovered_foreground_color: korangar_interface::prelude::ComputedSelector::new_default(|_: &ClientState| Color::WHITE),
                            ghost_foreground_color: text_box_ghost_foreground_color,
                            focused_background_color: {
                                let tokens = client_theme().tokens();
                                let color_tokens = tokens.color();
                                korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
                                    *color_tokens.chat_input_focused_background().follow_safe(state)
                                })
                            },
                            focus_id: ChatTextBox,
                            skinned: false,
                        },
                    ),
                ),
            ),
        }
    }
}
