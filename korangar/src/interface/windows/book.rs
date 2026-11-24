//! In-game book window for the rAthena `readbook` script command
//! (`ZC_READ_BOOK`, 0x0294).
//!
//! The 2019 packet only carries the book id and the page number
//! (`clif_readbook`, clif.cpp:18354-18361); the text lives in the client
//! archive as `data\book\<book id>.txt`. This window renders one page of that
//! file, keeping the `^RRGGBB` colour markup and line breaks the classic client
//! uses. If the archive does not ship the book data, a clear placeholder is
//! shown instead of an empty window.

use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// Number of text lines shown per page.
///
/// The original client paginates the `data\book` text by fixed line count; the
/// 2019 files themselves contain no page separators.
pub const BOOK_LINES_PER_PAGE: usize = 15;

/// Split the raw text of a book file into pages.
///
/// The first line of the classic book files is a `%RRGGBB` background colour
/// command (for example `%faf0e6`); it is not readable text and is dropped.
/// Empty trailing lines are trimmed so a short book does not get empty pages.
pub fn book_pages(text: &str) -> Vec<Vec<String>> {
    let mut lines: Vec<String> = text.lines().map(|line| line.trim_end_matches('\r').to_owned()).collect();

    if lines.first().is_some_and(|line| line.starts_with('%')) {
        lines.remove(0);
    }

    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }

    if lines.is_empty() {
        return Vec::new();
    }

    lines.chunks(BOOK_LINES_PER_PAGE).map(|chunk| chunk.to_vec()).collect()
}

pub struct BookWindow {
    book_id: u32,
    /// One-based page currently displayed.
    page: usize,
    pages: Vec<Vec<String>>,
}

impl BookWindow {
    pub fn new(book_id: u32, page: u32, pages: Vec<Vec<String>>) -> Self {
        let page = (page.max(1) as usize).min(pages.len().max(1));

        Self { book_id, page, pages }
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn content(&self) -> String {
        match self.pages.get(self.page.saturating_sub(1)) {
            Some(lines) if !lines.is_empty() => lines.join("\n"),
            _ => format!(
                "Book {}\n\nThe text of this book is not shipped in the client archive (expected `data\\book\\{}.txt`).",
                self.book_id, self.book_id
            ),
        }
    }
}

impl CustomWindow<ClientState> for BookWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Book)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let book_id = self.book_id;
        let page = self.page;
        let page_count = self.page_count().max(1);
        let title = format!("Book {book_id} - page {page}/{page_count}");
        let page_label = format!("{page} / {page_count}");
        let content = self.content();

        let previous_disabled = page <= 1;
        let next_disabled = page >= page_count;
        let previous_page = page.saturating_sub(1) as u32;
        let next_page = (page + 1) as u32;

        window! {
            title,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 420.0,
            maximum_width: 620.0,
            minimum_height: 260.0,
            maximum_height: 560.0,
            elements: (
                scroll_view! {
                    follow: true,
                    fit_content_height: false,
                    children: text! {
                        text: content,
                    },
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Previous page",
                            disabled: ComputedSelector::new_default(move |_: &ClientState| previous_disabled),
                            event: InputEvent::OpenBookPage {
                                book_id,
                                page: previous_page,
                            },
                        },
                        text! {
                            text: page_label,
                        },
                        button! {
                            text: "Next page",
                            disabled: ComputedSelector::new_default(move |_: &ClientState| next_disabled),
                            event: InputEvent::OpenBookPage {
                                book_id,
                                page: next_page,
                            },
                        },
                    ),
                },
            ),
        }
    }
}
