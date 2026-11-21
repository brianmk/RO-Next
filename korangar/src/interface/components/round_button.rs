//! Round action button element.
//!
//! The character overview uses small round buttons for collapsing into the
//! minimalist HUD and for expanding it again. The regular button component only
//! supports the full available width, so this element draws a compact circle of
//! a fixed size and right-aligns it inside the row it was given.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::event::ClickHandler;
use korangar_interface::layout::area::Area;
use korangar_interface::layout::tooltip::TooltipExt;
use korangar_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Path, State};

use crate::graphics::{CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::{ClientState, client_theme};

/// Tooltip id for the round buttons of the character overview.
struct RoundButtonTooltip;

/// Layout information of a [`RoundButton`].
pub struct RoundButtonLayoutInfo {
    area: Area,
    font_size: FontSize,
    tooltip: String,
}

/// A small round button with a single symbol.
///
/// The button occupies the full width of its row for layout purposes, but the
/// drawn/clickable circle is right-aligned and only `size` pixels wide/tall.
pub struct RoundButton<T, H> {
    text: &'static str,
    size: f32,
    tooltip_path: T,
    click_handler: H,
}

impl<T, H> RoundButton<T, H> {
    /// Creates a new round button.
    ///
    /// `text` is the symbol shown inside the circle, `size` is both the width
    /// and the height of the circle. The tooltip is read from `tooltip_path`.
    pub fn new(text: &'static str, size: f32, tooltip_path: T, click_handler: H) -> Self {
        Self {
            text,
            size,
            tooltip_path,
            click_handler,
        }
    }
}

impl<T, H> Element<ClientState> for RoundButton<T, H>
where
    T: Path<ClientState, String>,
    H: ClickHandler<ClientState>,
{
    type LayoutInfo = RoundButtonLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let tooltip = state.get(&self.tooltip_path).clone();
        let size = self.size;

        with_single_resolver(resolvers, |resolver| {
            let row = resolver.with_height(size);
            let circular_size = size.min(row.width).max(1.0);

            RoundButtonLayoutInfo {
                area: Area {
                    left: row.left + (row.width - circular_size).max(0.0),
                    top: row.top,
                    width: circular_size,
                    height: circular_size,
                },
                font_size: FontSize((circular_size * 0.62).max(8.0)),
                tooltip,
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
        let area = layout_info.area;

        if area.width <= 0.0 || area.height <= 0.0 {
            return;
        }

        let theme_path = client_theme();
        let theme = state.get(&theme_path);
        let button = &theme.button;

        let is_hovered = area.check().run(layout);

        let (background_color, foreground_color) = match is_hovered {
            true => (button.hovered_background_color, button.hovered_foreground_color),
            false => (button.background_color, button.foreground_color),
        };

        layout.add_rectangle(
            area,
            CornerDiameter::uniform(area.height / 2.0),
            background_color,
            button.shadow_color,
            ShadowPadding::uniform(0.0),
        );

        layout.add_text(
            area,
            self.text,
            layout_info.font_size,
            foreground_color,
            button.highlight_color,
            HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        if is_hovered {
            layout.register_click_handler(MouseButton::Left, &self.click_handler);

            let tooltip = layout_info.tooltip.as_str();

            if !tooltip.is_empty() {
                layout.add_tooltip(tooltip, RoundButtonTooltip.tooltip_id());
            }
        }
    }
}
