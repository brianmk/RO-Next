use korangar_interface::application::{Application, Size};
use korangar_interface::components::drop_down::DropDownItem;
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
use crate::interface::markup::{MarkupClickHandler, MarkupLayout, layout_markup, render_markup};
use crate::loaders::OverflowBehavior;
use crate::settings::{GameSettingsPathExt, InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::chat_markup::{ChatMarkupLink, ChatMarkupRun, parse_chat_markup};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{
    ChatTheme, ColorTokens, ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, SpacingTokensPathExt,
    TypographyTokensPathExt,
};
use crate::state::whisper::WhisperConversationsPathExt;
use crate::state::{ChatChannel, ChatMessage, ClientState, ClientStatePathExt, client_state, client_theme};

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
        MessageColor::Rgb { red, green, blue } => Color::from_rgb_bytes([red, green, blue]),
        MessageColor::Broadcast => *state.get(&tokens.chat_broadcast()),
        MessageColor::Server => *state.get(&tokens.chat_server()),
        MessageColor::Information => *state.get(&tokens.chat_information()),
        MessageColor::Error => *state.get(&tokens.chat_error()),
    }
}

/// Messages of one channel tab, in the order they were received.
///
/// This is the single routing point of the chat window: the state keeps one
/// list of [`ChatMessage`]s and every message carries its channel, so a tab is
/// a filtered view instead of an independently maintained log.
pub fn messages_for_channel<'a>(messages: &'a [ChatMessage], channel: ChatChannel) -> impl Iterator<Item = &'a ChatMessage> + 'a {
    messages.iter().filter(move |message| message.channel == channel)
}

/// ZST for getting the focus id of the chat text box. This is only needed to
/// focus the chat when pressing enter.
pub struct ChatTextBox;

/// Layout of one message. Plain messages keep the original text layout path;
/// only messages with interactive runs use the markup layout.
enum ChatMessageLayout {
    Plain { height: f32 },
    Markup(MarkupLayout),
}

impl ChatMessageLayout {
    fn height(&self) -> f32 {
        match self {
            ChatMessageLayout::Plain { height } => *height,
            ChatMessageLayout::Markup(layout) => layout.height,
        }
    }
}

struct ChatLayoutInfo {
    area: Area,
    // TODO: Don't allocate this every frame.
    messages: Vec<ChatMessageLayout>,
}

struct ChatElement<A, B> {
    chat_messages_path: A,
    active_channel_path: B,
    click_handler: MarkupClickHandler,
}

impl<A, B> ChatElement<A, B> {
    fn new(chat_messages_path: A, active_channel_path: B) -> Self {
        Self {
            chat_messages_path,
            active_channel_path,
            click_handler: MarkupClickHandler::new(),
        }
    }
}

/// Prepare the runs of a message for the layout.
///
/// Whisper messages get a whisper link on their plain runs (if they do not
/// already contain an item/URL link), so clicking anywhere in a whisper line
/// opens the conversation.
fn markup_runs_for_message(chat_message: &ChatMessage) -> Vec<ChatMarkupRun> {
    let runs = parse_chat_markup(&chat_message.text);

    match &chat_message.whisper_partner {
        Some(partner) => runs
            .into_iter()
            .map(|run| ChatMarkupRun {
                text: run.text,
                link: run.link.or_else(|| Some(ChatMarkupLink::Whisper { partner: partner.clone() })),
            })
            .collect(),
        None => runs,
    }
}

impl<A, B> Element<ClientState> for ChatElement<A, B>
where
    A: Path<ClientState, Vec<ChatMessage>>,
    B: Path<ClientState, ChatChannel>,
{
    type LayoutInfo = ChatLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let active_channel = *state.get(&self.active_channel_path);
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
            let text_width = resolver.push_available_area().width;
            let markup_width = text_width
                - match message_alignment {
                    HorizontalAlignment::Left { offset, border } => offset + border,
                    HorizontalAlignment::Center { offset, border } | HorizontalAlignment::Right { offset, border } => offset + border,
                };

            let mut total_height = 0.0;
            let messages = messages_for_channel(chat_messages, active_channel)
                .map(|chat_message| {
                    let color = message_color(state, color_tokens, chat_message.color);

                    let layout = if chat_message.whisper_partner.is_some() || crate::state::chat_markup::has_markup(&chat_message.text) {
                        let runs = markup_runs_for_message(chat_message);
                        let layout = layout_markup(
                            resolver.get_text_layouter(),
                            &runs,
                            markup_width,
                            message_font_size,
                            color,
                            highlight_color,
                        );
                        ChatMessageLayout::Markup(layout)
                    } else {
                        let (size, _) = resolver.get_text_dimensions(
                            &chat_message.text,
                            color,
                            highlight_color,
                            message_font_size,
                            message_alignment,
                            OverflowBehavior::LineBreak,
                        );
                        ChatMessageLayout::Plain { height: size.height() }
                    };

                    if total_height != 0.0 {
                        total_height += message_spacing;
                    }

                    total_height += layout.height();

                    layout
                })
                .collect();

            let area = resolver.with_height(total_height);

            Self::LayoutInfo { area, messages }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let active_channel = *state.get(&self.active_channel_path);
        let chat_messages = state.get(&self.chat_messages_path);
        let tokens = client_theme().tokens();
        let color_tokens = tokens.color();
        let typography = tokens.typography();
        let spacing = tokens.spacing();

        let message_spacing = *state.get(&spacing.chat_message_gap());
        let message_font_size = *state.get(&typography.font_body_lg());
        let highlight_color = *state.get(&color_tokens.text_overlay_accent());
        let link_color = *state.get(&color_tokens.text_overlay_accent());
        let message_alignment = HorizontalAlignment::Left {
            offset: *state.get(&spacing.lg()),
            border: *state.get(&spacing.text_border()),
        };
        let message_offset = match message_alignment {
            HorizontalAlignment::Left { offset, border } => offset + border,
            HorizontalAlignment::Center { offset, border } | HorizontalAlignment::Right { offset, border } => offset + border,
        };

        let mut offset = 0.0;
        messages_for_channel(chat_messages, active_channel)
            .zip(layout_info.messages.iter())
            .for_each(|(chat_message, message_layout)| {
                let color = message_color(state, color_tokens, chat_message.color);

                if offset != 0.0 {
                    offset += message_spacing;
                }

                let text_area = Area {
                    left: layout_info.area.left,
                    top: layout_info.area.top + offset,
                    width: layout_info.area.width,
                    height: message_layout.height(),
                };

                match message_layout {
                    ChatMessageLayout::Plain { .. } => {
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
                    }
                    ChatMessageLayout::Markup(markup_layout) => {
                        render_markup(
                            markup_layout,
                            text_area,
                            message_offset,
                            layout,
                            message_font_size,
                            color,
                            highlight_color,
                            link_color,
                            &self.click_handler,
                        );
                    }
                }

                offset += message_layout.height();
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
#[derive(RustState, StateElement)]
pub struct ChatWindowState {
    current_text: String,
    /// Channel tab whose log is shown.
    active_channel: ChatChannel,
    /// Channel non-slash messages are sent to.
    send_channel: ChatChannel,
    /// Whether the server supports guild chat (`CZ_GUILD_CHAT`, 0x017E).
    ///
    /// rAthena has guild chat since well before PACKETVER 20190605, so the tab
    /// is enabled; the flag exists so a future/older server can grey it out
    /// without restructuring the tabs.
    guild_chat_available: bool,
    /// Options of the "send to channel" drop down. A stored list keeps the
    /// drop down on a plain path selector (computed selectors are not `Clone`
    /// and the component macro requires a cloneable selector).
    send_channel_options: Vec<ChatChannel>,
}

impl Default for ChatWindowState {
    fn default() -> Self {
        Self {
            current_text: String::new(),
            active_channel: ChatChannel::Public,
            send_channel: ChatChannel::Public,
            guild_chat_available: true,
            send_channel_options: ChatChannel::OPTIONS.to_vec(),
        }
    }
}

impl ChatWindowState {
    /// Replace the text in the chat input, e.g. to pre-fill a whisper command.
    /// The cursor is placed at the end of the text by the text box.
    pub fn set_current_text(&mut self, text: String) {
        self.current_text = text;
    }

    pub fn active_channel(&self) -> ChatChannel {
        self.active_channel
    }

    pub fn send_channel(&self) -> ChatChannel {
        self.send_channel
    }

    pub fn guild_chat_available(&self) -> bool {
        self.guild_chat_available
    }

    pub fn set_active_channel(&mut self, channel: ChatChannel) {
        self.active_channel = channel;
    }

    pub fn set_send_channel(&mut self, channel: ChatChannel) {
        self.send_channel = channel;
    }

    pub fn set_guild_chat_available(&mut self, available: bool) {
        self.guild_chat_available = available;
    }
}

impl DropDownItem<ChatChannel> for ChatChannel {
    fn text(&self) -> &str {
        self.label()
    }

    fn value(&self) -> ChatChannel {
        *self
    }

    fn disabled(&self) -> Option<&'static str> {
        None
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
        let active_channel_path = self.chat_window_state.active_channel();
        let send_channel_path = self.chat_window_state.send_channel();
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
        // The focused chat input is drawn on a dark background; using the
        // skinned text box theme colors here would make the text unreadable with the
        // classic theme (which uses black text on light text boxes).
        let text_box_focused_foreground_color = chat_color_constant(chat_theme_path, |chat| chat.text_box_focused_foreground_color);
        let text_box_ghost_foreground_color = {
            let color_tokens = client_theme().tokens().color();
            korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
                *color_tokens.chat_ghost_text().follow_safe(state)
            })
        };
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
            let text = state.get(&current_text_path).clone();

            if text.is_empty() {
                // In battle mode Enter on an empty line closes the line again,
                // like the official client.
                if *state.get(&client_state().game_settings().battle_mode()) {
                    queue.queue(Event::Unfocus);
                }


                return;
            }

            // Clear the text box.
            state.update_value_with(current_text_path, |current_text| current_text.clear());
            // Channel selector: prefix the plain message with the slash
            // command of the selected channel. An explicit slash command in the
            // text always wins and is never prefixed.
            if text.starts_with('/') {
                queue.queue(InputEvent::SendMessage { text });
                queue.queue(Event::Unfocus);
                return;
            }

            let send_channel = *state.get(&send_channel_path);
            let whisper_partner_path = crate::state::client_state().whisper().active_partner();
            let text = match send_channel {
                ChatChannel::Public => text,
                ChatChannel::Party => format!("/p {text}"),
                ChatChannel::Guild => format!("/g {text}"),
                ChatChannel::Whisper => {
                    let partner = state.get(&whisper_partner_path).clone();

                    match partner.is_empty() {
                        // No conversation open yet: fall back to public chat
                        // instead of sending a malformed whisper command.
                        true => text,
                        false => format!("/w {partner} {text}"),
                    }
                }
            };

            queue.queue(InputEvent::SendMessage { text });
            queue.queue(Event::Unfocus);
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
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        channel_tab(self.chat_window_state, ChatChannel::Public, "Public"),
                        channel_tab(self.chat_window_state, ChatChannel::Party, "Party"),
                        channel_tab(self.chat_window_state, ChatChannel::Whisper, "Whisper"),
                        channel_tab(self.chat_window_state, ChatChannel::Guild, "Guild"),
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text! {
                            text: "Send to",
                            height: 18.0,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        drop_down! {
                            selected: send_channel_path,
                            options: self.chat_window_state.send_channel_options(),
                        },
                    ),
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
                            children: ChatElement::new(self.chat_messages_path, active_channel_path),
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

/// One tab button of the chat window.
///
/// The active tab is disabled (like the inventory tabs), the guild tab is
/// additionally disabled when the server has no guild chat support.
fn channel_tab<A>(chat_window_state: A, channel: ChatChannel, label: &'static str) -> impl Element<ClientState>
where
    A: Path<ClientState, ChatWindowState> + 'static,
{
    use korangar_interface::prelude::*;

    button! {
        text: label,
        disabled: korangar_interface::prelude::ComputedSelector::<_, bool>::new_default(move |state: &ClientState| {
            let active = *chat_window_state.active_channel().follow_safe(state) == channel;
            let available = channel != ChatChannel::Guild || *chat_window_state.guild_chat_available().follow_safe(state);
            active || !available
        }),
        event: move |state: &State<ClientState>, _queue: &mut EventQueue<ClientState>| {
            state.update_value(chat_window_state.active_channel(), channel);
            state.update_value(chat_window_state.send_channel(), channel);
        },
    }
}

#[cfg(test)]
mod tests {
    use korangar_networking::MessageColor;

    use super::messages_for_channel;
    use crate::interface::windows::ChatWindowState;
    use crate::state::{ChatChannel, ChatMessage};

    fn message(text: &str, channel: ChatChannel) -> ChatMessage {
        ChatMessage::new_channel(text.to_owned(), MessageColor::Information, channel)
    }

    #[test]
    fn messages_are_routed_to_their_channel_tab() {
        let messages = vec![
            message("public", ChatChannel::Public),
            message("party", ChatChannel::Party),
            ChatMessage::whisper("whisper".to_owned(), MessageColor::Information, "Alice"),
            message("guild", ChatChannel::Guild),
        ];

        let public: Vec<_> = messages_for_channel(&messages, ChatChannel::Public)
            .map(|message| message.text.as_str())
            .collect();
        let party: Vec<_> = messages_for_channel(&messages, ChatChannel::Party)
            .map(|message| message.text.as_str())
            .collect();
        let whisper: Vec<_> = messages_for_channel(&messages, ChatChannel::Whisper)
            .map(|message| message.text.as_str())
            .collect();
        let guild: Vec<_> = messages_for_channel(&messages, ChatChannel::Guild)
            .map(|message| message.text.as_str())
            .collect();

        assert_eq!(public, ["public"]);
        assert_eq!(party, ["party"]);
        assert_eq!(whisper, ["whisper"]);
        assert_eq!(guild, ["guild"]);
    }

    #[test]
    fn whisper_messages_keep_their_partner_for_the_click_target() {
        let message = ChatMessage::whisper("hello".to_owned(), MessageColor::Information, "Alice");

        assert_eq!(message.channel, ChatChannel::Whisper);
        assert_eq!(message.whisper_partner.as_deref(), Some("Alice"));
        assert_eq!(ChatMessage::new("x".to_owned(), MessageColor::Server).whisper_partner, None);
    }

    #[test]
    fn the_default_chat_state_shows_and_sends_public_chat() {
        let state = ChatWindowState::default();

        assert_eq!(state.active_channel(), ChatChannel::Public);
        assert_eq!(state.send_channel(), ChatChannel::Public);
        assert!(state.guild_chat_available());
    }

    #[test]
    fn switching_tabs_does_not_change_the_send_channel_by_itself() {
        let mut state = ChatWindowState::default();
        state.set_active_channel(ChatChannel::Party);

        assert_eq!(state.active_channel(), ChatChannel::Party);
        assert_eq!(state.send_channel(), ChatChannel::Public);
    }
}
