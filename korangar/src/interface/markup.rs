//! Layout and click handling for interactive chat/dialog markup.
//!
//! The parser in [`crate::state::chat_markup`] turns a line into plain and
//! linked runs. This module lays those runs out (word wrapping, per-run
//! position and width) and provides the click handler that turns a hit on a
//! run into an [`InputEvent`].
//!
//! The plain (no markup) rendering path of the chat window and the NPC dialog
//! does not go through this module at all, so it stays identical.

use std::cell::RefCell;
use std::rc::Rc;

use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{MouseButton, WindowLayout};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::State;

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::loaders::{FontLoader, FontSize, OverflowBehavior};
use crate::state::ClientState;
use crate::state::chat_markup::{ChatMarkupLink, ChatMarkupRun};

/// One laid-out run of a text.
#[derive(Clone, Debug)]
pub struct MarkupRunLayout {
    pub text: String,
    pub link: Option<ChatMarkupLink>,
    /// Zero-based line index.
    pub line: usize,
    /// Horizontal offset from the left edge of the text area.
    pub x: f32,
    /// Measured width of the run.
    pub width: f32,
}

/// Result of laying out one chat message / dialog line.
#[derive(Clone, Debug, Default)]
pub struct MarkupLayout {
    pub runs: Vec<MarkupRunLayout>,
    pub line_count: usize,
    pub line_height: f32,
    pub height: f32,
}

impl MarkupLayout {
    /// The hit area of a run inside a text `area`.
    pub fn run_area(&self, area: Area, run: &MarkupRunLayout) -> Area {
        Area {
            left: area.left + run.x,
            top: area.top + run.line as f32 * self.line_height,
            // A little extra width makes the 1px rounding of the font layout
            // forgiving, otherwise the last glyph can be outside the hit area.
            width: run.width + 2.0,
            height: self.line_height,
        }
    }
}

/// Lay out parsed runs into word-wrapped lines.
///
/// `available_width` is the width the text may occupy; a token wider than that
/// on its own stays on one line and is clipped by the window (chat messages are
/// server-limited in length).
pub fn layout_markup(
    font_loader: &FontLoader,
    runs: &[ChatMarkupRun],
    available_width: f32,
    font_size: FontSize,
    color: Color,
    highlight_color: Color,
) -> MarkupLayout {
    let mut layout = MarkupLayout {
        runs: Vec::new(),
        line_count: 1,
        line_height: 0.0,
        height: 0.0,
    };

    let mut tokens = Vec::new();
    for run in runs {
        tokens.extend(tokenize(&run.text, run.link.clone()));
    }

    // Measure the line height from an actual line of text, so the custom
    // markup layout uses the same line metrics as the plain text renderer.
    let (line_size, _) = font_loader.get_text_dimensions(
        "Ag",
        color,
        highlight_color,
        font_size,
        1.0,
        f32::MAX,
        OverflowBehavior::LineBreak,
    );
    layout.line_height = line_size.height;

    let mut line = 0;
    let mut x = 0.0;

    for token in tokens {
        if token.newline {
            line += 1;
            x = 0.0;
            continue;
        }

        if token.text.is_empty() {
            continue;
        }

        let (size, _) = font_loader.get_text_dimensions(
            &token.text,
            color,
            highlight_color,
            font_size,
            1.0,
            f32::MAX,
            OverflowBehavior::LineBreak,
        );

        if x > 0.0 && x + size.width > available_width {
            line += 1;
            x = 0.0;
        }

        layout.runs.push(MarkupRunLayout {
            text: token.text,
            link: token.link,
            line,
            x,
            width: size.width,
        });

        x += size.width;
    }

    layout.line_count = line + 1;

    layout.height = layout.line_count as f32 * layout.line_height;

    layout
}

struct Token {
    text: String,
    link: Option<ChatMarkupLink>,
    newline: bool,
}

/// Split a run into word-sized tokens, keeping the separating spaces.
///
/// Linked runs are split by words as well so a long link can wrap; every token
/// keeps the link so the whole display text stays clickable.
fn tokenize(text: &str, link: Option<ChatMarkupLink>) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in text.chars() {
        match character {
            '\n' => {
                if !current.is_empty() {
                    tokens.push(Token {
                        text: std::mem::take(&mut current),
                        link: link.clone(),
                        newline: false,
                    });
                }

                tokens.push(Token {
                    text: String::new(),
                    link: None,
                    newline: true,
                });
            }
            ' ' => {
                current.push(character);
                tokens.push(Token {
                    text: std::mem::take(&mut current),
                    link: link.clone(),
                    newline: false,
                });
            }
            _ => current.push(character),
        }
    }

    if !current.is_empty() {
        tokens.push(Token {
            text: current,
            link,
            newline: false,
        });
    }

    tokens
}

/// Shared slot for the link of the run that is currently under the mouse.
///
/// The chat/dialog element writes the hovered link before registering the
/// handler for that frame; the handler reads it when the click is dispatched.
#[derive(Clone, Default)]
pub struct MarkupClickHandler {
    hovered: Rc<RefCell<Option<ChatMarkupLink>>>,
}

impl MarkupClickHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remember the link of the hovered run.
    pub fn set(&self, link: ChatMarkupLink) {
        *self.hovered.borrow_mut() = Some(link);
    }

    pub fn clear(&self) {
        *self.hovered.borrow_mut() = None;
    }
}

impl ClickHandler<ClientState> for MarkupClickHandler {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        let link = self.hovered.borrow().clone();

        if let Some(link) = link {
            queue.queue(InputEvent::ChatMarkupClicked { link });
        }
    }
}

/// Render laid-out runs and register the click handlers of linked runs.
///
/// `offset` is the horizontal offset of the text alignment inside `area`.
#[allow(clippy::too_many_arguments)]
pub fn render_markup<'a>(
    layout_info: &'a MarkupLayout,
    area: Area,
    offset: f32,
    layout: &mut WindowLayout<'a, ClientState>,
    font_size: FontSize,
    color: Color,
    highlight_color: Color,
    link_color: Color,
    handler: &'a MarkupClickHandler,
) {
    let text_area = Area {
        left: area.left + offset,
        top: area.top,
        width: (area.width - offset).max(0.0),
        height: layout_info.height,
    };

    for run in &layout_info.runs {
        let run_area = layout_info.run_area(text_area, run);

        if run.link.is_some() {
            // Underline the link so the interaction is visible without hover.
            layout.add_rectangle(
                Area {
                    left: run_area.left,
                    top: run_area.top + layout_info.line_height - 1.0,
                    width: run_area.width,
                    height: 1.0,
                },
                CornerDiameter::default(),
                link_color,
                Color::TRANSPARENT,
                ShadowPadding::uniform(0.0),
            );
        }

        layout.add_text(
            run_area,
            &run.text,
            font_size,
            match run.link.is_some() {
                true => link_color,
                false => color,
            },
            highlight_color,
            HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
            VerticalAlignment::Top { offset: 0.0 },
            OverflowBehavior::LineBreak,
        );

        if let Some(link) = &run.link
            && run_area.check().run(layout)
        {
            handler.set(link.clone());
            layout.register_click_handler(MouseButton::Left, handler);
        }
    }
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::ItemId;

    use super::{MarkupLayout, MarkupRunLayout, tokenize};
    use crate::state::chat_markup::ChatMarkupLink;

    #[test]
    fn tokenizing_splits_words_and_preserves_links() {
        let link = ChatMarkupLink::Item {
            item_id: ItemId(501),
            identified: true,
        };
        let tokens = tokenize("hello world", Some(link.clone()));

        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].text, "hello ");
        assert_eq!(tokens[1].text, "world");
        assert!(tokens.iter().all(|token| token.link == Some(link.clone())));
        assert!(tokens.iter().all(|token| !token.newline));
    }

    #[test]
    fn tokenizing_turns_newlines_into_break_tokens() {
        let tokens = tokenize("first\nsecond", None);

        assert_eq!(tokens.iter().filter(|token| token.newline).count(), 1);
        assert_eq!(tokens.iter().filter(|token| !token.newline).count(), 2);
    }

    #[test]
    fn run_area_is_offset_by_line_and_x() {
        let layout = MarkupLayout {
            runs: Vec::new(),
            line_count: 2,
            line_height: 20.0,
            height: 40.0,
        };
        let run = MarkupRunLayout {
            text: "x".to_owned(),
            link: None,
            line: 1,
            x: 12.0,
            width: 5.0,
        };

        let area = layout.run_area(
            korangar_interface::layout::area::Area {
                left: 100.0,
                top: 50.0,
                width: 300.0,
                height: 40.0,
            },
            &run,
        );

        assert_eq!(area.left, 112.0);
        assert_eq!(area.top, 70.0);
        assert_eq!(area.width, 7.0);
        assert_eq!(area.height, 20.0);
    }
}
