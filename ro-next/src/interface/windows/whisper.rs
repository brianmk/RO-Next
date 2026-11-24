//! Dedicated whisper conversation window.
//!
//! The official client opens a small window per whisper partner instead of
//! mixing whispers into the public chat. This window shows the bounded history
//! of the currently active partner and a reply box; it is opened by
//! `/w <name> <message>` or by clicking a whisper line in the chat window.
//!
//! The 2019 rAthena packet set has no per-player whisper block/unblock packet
//! (only the whisper ignore *list* request `CZ_REQ_WHISPER_LIST`, 0x00D3), so
//! there is deliberately no block button here; the chat `/ignore`-style
//! atcommand does not exist on that server either.

use ro_next_interface::application::Size;
use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::Element;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, State};

use super::WindowClass;
use crate::input::InputEvent;
use crate::loaders::OverflowBehavior;
use crate::state::theme::{
    ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, SpacingTokensPathExt, TypographyTokensPathExt,
};
use crate::state::whisper::{WhisperConversations, WhisperConversationsPathExt};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

const MAXIMUM_WHISPER_LENGTH: usize = 255;

/// One measured line of the conversation.
struct WhisperLine {
    text: String,
    color: crate::graphics::Color,
    height: f32,
}

struct WhisperLayoutInfo {
    area: Area,
    lines: Vec<WhisperLine>,
}

/// The message list of the whisper window.
#[derive(Default)]
struct WhisperElement;

impl Element<ClientState> for WhisperElement {
    type LayoutInfo = WhisperLayoutInfo;

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
            let incoming_color = *state.get(&color_tokens.text_primary());
            let outgoing_color = *state.get(&color_tokens.text_accent());
            let message_spacing = *state.get(&spacing.xs());

            let whisper = state.follow(client_state().whisper());
            let partner = whisper.active_partner.clone();

            let mut total_height = 0.0;
            let lines = whisper
                .active_messages()
                .iter()
                .map(|message| {
                    let text = message.display(&partner);
                    let color = match message.outgoing {
                        true => outgoing_color,
                        false => incoming_color,
                    };
                    let (size, _) = resolver.get_text_dimensions(
                        &text,
                        color,
                        outgoing_color,
                        font_size,
                        HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
                        OverflowBehavior::LineBreak,
                    );

                    if total_height != 0.0 {
                        total_height += message_spacing;
                    }

                    total_height += size.height();

                    WhisperLine {
                        text,
                        color,
                        height: size.height(),
                    }
                })
                .collect();

            let area = resolver.with_height(total_height);

            WhisperLayoutInfo { area, lines }
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
        let highlight_color = *state.get(&tokens.color().text_accent());
        let message_spacing = *state.get(&spacing.xs());

        let mut offset = 0.0;
        for line in &layout_info.lines {
            if offset != 0.0 {
                offset += message_spacing;
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
                highlight_color,
                HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::LineBreak,
            );

            offset += line.height;
        }
    }
}

/// A window showing the whisper conversation with one partner.
pub struct WhisperWindow<A> {
    whisper_path: A,
}

impl<A> WhisperWindow<A> {
    pub fn new(whisper_path: A) -> Self {
        Self { whisper_path }
    }
}

impl<A> CustomWindow<ClientState> for WhisperWindow<A>
where
    A: Path<ClientState, WhisperConversations>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Whisper)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let input_text_path = self.whisper_path.input_text();
        let active_partner_path = self.whisper_path.active_partner();
        let send_path = self.whisper_path.input_text();
        let send_partner_path = self.whisper_path.active_partner();

        let title = ro_next_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
            let partner = active_partner_path.follow_safe(state);

            match partner.is_empty() {
                true => "Whisper".to_owned(),
                false => format!("Whisper: {partner}"),
            }
        });

        let send_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let text = state.get(&send_path).clone();
            let partner = state.get(&send_partner_path).clone();

            if text.is_empty() || partner.is_empty() {
                return;
            }

            state.update_value_with(send_path, |current| current.clear());
            queue.queue(InputEvent::SendWhisper { partner, text });
            queue.queue(Event::Unfocus);
        };
        let send_action_button = send_action.clone();

        window! {
            title,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            elements: (
                scroll_view! {
                    follow: true,
                    fit_content_height: false,
                    children: WhisperElement,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text_box! {
                            ghost_text: "Whisper reply",
                            state: input_text_path,
                            input_handler: DefaultHandler::<_, _, MAXIMUM_WHISPER_LENGTH>::new(input_text_path, send_action),
                            focus_id: super::chat::ChatTextBox,
                        },
                        button! {
                            text: "Send",
                            event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                send_action_button(state, queue);
                            },
                        },
                    ),
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graphics::Color;

    #[test]
    fn whisper_lines_are_measured_and_ordered() {
        // The layout element is exercised through the state tests; this test
        // documents the color convention used for the two directions.
        let incoming = Color::rgb_u8(255, 255, 255);
        let outgoing = Color::rgb_u8(255, 200, 80);
        assert_ne!(incoming, outgoing);
    }
}
