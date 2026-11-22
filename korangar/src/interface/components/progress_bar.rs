//! Horizontal progress bar elements.
//!
//! The character overview uses this element to draw the health, spell point and
//! experience progress as bars. The bar is drawn as a rounded track with a
//! rounded fill, so it fits both the flat korangar theme and the classic
//! interface skin.
//!
//! The progress is supplied as an `Option<f32>`: `None` means "unknown", which
//! is used for the experience bars until the server sent the experience that is
//! needed for the next level. Unknown progress is rendered as an empty bar
//! instead of dividing by zero.

use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use rust_state::{Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt};
use crate::state::{ClientState, client_theme};

/// Fraction of the health bar below which the "low health" color is used.
pub const LOW_HEALTH_FRACTION: f32 = 0.25;

/// Width of the filled part of a bar.
///
/// The result is clamped to the bar width, and it is at least one bar height
/// wide (as long as the bar itself is that wide) so that a non-zero but tiny
/// amount of progress shows up as a rounded cap instead of disappearing
/// completely.
fn fill_width(bar_width: f32, bar_height: f32, fraction: f32) -> f32 {
    let fraction = fraction.clamp(0.0, 1.0);

    if fraction <= 0.0 || bar_width <= 0.0 {
        return 0.0;
    }

    (bar_width * fraction).max(bar_height.min(bar_width)).min(bar_width)
}

/// Draws the rounded track and the fill of a bar into `area`.
fn draw_bar(layout: &mut WindowLayout<'_, ClientState>, area: Area, fraction: f32, fill_color: Color, background_color: Color) {
    let corner_diameter = CornerDiameter::uniform(area.height / 2.0);

    layout.add_rectangle(
        area,
        corner_diameter,
        background_color,
        Color::TRANSPARENT,
        ShadowPadding::uniform(0.0),
    );

    let width = fill_width(area.width, area.height, fraction);

    if width <= 0.0 {
        return;
    }

    layout.add_rectangle(
        Area {
            left: area.left,
            top: area.top,
            width,
            height: area.height,
        },
        corner_diameter,
        fill_color,
        Color::TRANSPARENT,
        ShadowPadding::uniform(0.0),
    );
}

/// Layout parameters of a labelled bar.
pub struct BarStyle {
    /// Width of the label column, in interface pixels.
    pub label_width: f32,
    /// Font size of the label.
    pub label_font_size: f32,
    /// Font size of the value that is rendered inside the bar.
    pub value_font_size: f32,
    /// Gap between the label column and the bar.
    pub gap: f32,
    /// Height of the bar (and of the whole row).
    pub height: f32,
}

impl BarStyle {
    pub fn new(label_width: f32, label_font_size: f32, value_font_size: f32, gap: f32, height: f32) -> Self {
        Self {
            label_width,
            label_font_size,
            value_font_size,
            gap,
            height,
        }
    }
}

/// A progress bar with a label on the left and the value rendered inside the
/// bar.
///
/// This is the row used by the character overview: `HP [====== 9411 / 9411]`.
/// The label uses the theme label color, the value is rendered centered inside
/// the bar in a light color so it stays readable on every bar color.
pub struct LabeledProgressBar<Label, Value, Fraction, FillColor, BackgroundColor> {
    label: Label,
    value: Value,
    fraction: Fraction,
    fill_color: FillColor,
    background_color: BackgroundColor,
    style: BarStyle,
}

impl<Label, Value, Fraction, FillColor, BackgroundColor> LabeledProgressBar<Label, Value, Fraction, FillColor, BackgroundColor>
where
    Label: Selector<ClientState, String>,
    Value: Selector<ClientState, String>,
    Fraction: Selector<ClientState, Option<f32>>,
    FillColor: Selector<ClientState, Color>,
    BackgroundColor: Selector<ClientState, Color>,
{
    pub fn new(
        label: Label,
        value: Value,
        fraction: Fraction,
        fill_color: FillColor,
        background_color: BackgroundColor,
        style: BarStyle,
    ) -> Self {
        Self {
            label,
            value,
            fraction,
            fill_color,
            background_color,
            style,
        }
    }
}

impl<Label, Value, Fraction, FillColor, BackgroundColor> Element<ClientState>
    for LabeledProgressBar<Label, Value, Fraction, FillColor, BackgroundColor>
where
    Label: Selector<ClientState, String>,
    Value: Selector<ClientState, String>,
    Fraction: Selector<ClientState, Option<f32>>,
    FillColor: Selector<ClientState, Color>,
    BackgroundColor: Selector<ClientState, Color>,
{
    type LayoutInfo = (Area, Area);

    fn create_layout_info(
        &mut self,
        _: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> (Area, Area) {
        with_single_resolver(resolvers, |resolver| {
            let row = resolver.with_height(self.style.height);
            split_labeled_bar(row, self.style.label_width, self.style.gap)
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a (Area, Area),
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let (label_area, bar_area) = *layout_info;

        if label_area.height <= 0.0 || bar_area.width <= 0.0 || bar_area.height <= 0.0 {
            return;
        }

        let label = state.get(&self.label);
        let label_color = state.get(&client_theme()).character_overview.label_color;

        layout.add_text(
            label_area,
            label,
            FontSize(self.style.label_font_size),
            label_color,
            label_color,
            HorizontalAlignment::Left { offset: 0.0, border: 0.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        let fraction = (*state.get(&self.fraction)).unwrap_or(0.0);
        let fill_color = *state.get(&self.fill_color);
        let background_color = *state.get(&self.background_color);

        draw_bar(layout, bar_area, fraction, fill_color, background_color);

        let value = state.get(&self.value);

        if !value.is_empty() && bar_area.height + f32::EPSILON >= self.style.value_font_size {
            layout.add_text(
                bar_area,
                value,
                FontSize(self.style.value_font_size),
                *state.get(&client_theme().tokens().color().bar_value_text()),
                *state.get(&client_theme().tokens().color().bar_value_text()),
                HorizontalAlignment::Center { offset: 0.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );
        }
    }
}

/// Splits a row area into the label area and the bar area.
fn split_labeled_bar(row: Area, label_width: f32, gap: f32) -> (Area, Area) {
    let label_width = label_width.clamp(0.0, row.width);
    let label_area = Area {
        left: row.left,
        top: row.top,
        width: label_width,
        height: row.height,
    };
    let bar_area = Area {
        left: row.left + label_width + gap,
        top: row.top,
        width: (row.width - label_width - gap).max(1.0),
        height: row.height,
    };

    (label_area, bar_area)
}

/// A bar without any label or value text.
///
/// Used by the minimalist companion HUD of the character overview, which shows
/// only the bars (the progress can be read from the bar length alone).
pub struct MiniBar<Fraction, FillColor, BackgroundColor> {
    fraction: Fraction,
    fill_color: FillColor,
    background_color: BackgroundColor,
    height: f32,
}

impl<Fraction, FillColor, BackgroundColor> MiniBar<Fraction, FillColor, BackgroundColor> {
    /// Creates a new label-less progress bar with the given height.
    pub fn new(fraction: Fraction, fill_color: FillColor, background_color: BackgroundColor, height: f32) -> Self {
        Self {
            fraction,
            fill_color,
            background_color,
            height,
        }
    }
}

impl<Fraction, FillColor, BackgroundColor> Element<ClientState> for MiniBar<Fraction, FillColor, BackgroundColor>
where
    Fraction: Selector<ClientState, Option<f32>>,
    FillColor: Selector<ClientState, Color>,
    BackgroundColor: Selector<ClientState, Color>,
{
    type LayoutInfo = Area;

    fn create_layout_info(&mut self, _: &State<ClientState>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<ClientState>) -> Area {
        with_single_resolver(resolvers, |resolver| resolver.with_height(self.height))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        area: &'a Area,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        if area.width <= 0.0 || area.height <= 0.0 {
            return;
        }

        let fraction = (*state.get(&self.fraction)).unwrap_or(0.0);
        let fill_color = *state.get(&self.fill_color);
        let background_color = *state.get(&self.background_color);

        draw_bar(layout, *area, fraction, fill_color, background_color);
    }
}

/// Two thin label-less bars stacked into a single row.
///
/// The minimalist character overview HUD uses this for the base and job
/// experience bars, which share one row and are drawn on top of each other.
pub struct StackedMiniBars<TopFraction, TopFillColor, TopBackgroundColor, BottomFraction, BottomFillColor, BottomBackgroundColor> {
    top_fraction: TopFraction,
    top_fill_color: TopFillColor,
    top_background_color: TopBackgroundColor,
    bottom_fraction: BottomFraction,
    bottom_fill_color: BottomFillColor,
    bottom_background_color: BottomBackgroundColor,
    height: f32,
    gap: f32,
}

impl<TopFraction, TopFillColor, TopBackgroundColor, BottomFraction, BottomFillColor, BottomBackgroundColor>
    StackedMiniBars<TopFraction, TopFillColor, TopBackgroundColor, BottomFraction, BottomFillColor, BottomBackgroundColor>
{
    /// Creates a stacked bar with the given total height and inner gap.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        top_fraction: TopFraction,
        top_fill_color: TopFillColor,
        top_background_color: TopBackgroundColor,
        bottom_fraction: BottomFraction,
        bottom_fill_color: BottomFillColor,
        bottom_background_color: BottomBackgroundColor,
        height: f32,
        gap: f32,
    ) -> Self {
        Self {
            top_fraction,
            top_fill_color,
            top_background_color,
            bottom_fraction,
            bottom_fill_color,
            bottom_background_color,
            height,
            gap,
        }
    }
}

impl<TopFraction, TopFillColor, TopBackgroundColor, BottomFraction, BottomFillColor, BottomBackgroundColor> Element<ClientState>
    for StackedMiniBars<TopFraction, TopFillColor, TopBackgroundColor, BottomFraction, BottomFillColor, BottomBackgroundColor>
where
    TopFraction: Selector<ClientState, Option<f32>>,
    TopFillColor: Selector<ClientState, Color>,
    TopBackgroundColor: Selector<ClientState, Color>,
    BottomFraction: Selector<ClientState, Option<f32>>,
    BottomFillColor: Selector<ClientState, Color>,
    BottomBackgroundColor: Selector<ClientState, Color>,
{
    type LayoutInfo = (Area, Area);

    fn create_layout_info(
        &mut self,
        _: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> (Area, Area) {
        let gap = self.gap;

        with_single_resolver(resolvers, |resolver| {
            let row = resolver.with_height(self.height);
            let bar_height = ((row.height - gap) / 2.0).max(1.0);

            let top = Area {
                left: row.left,
                top: row.top,
                width: row.width,
                height: bar_height,
            };
            let bottom = Area {
                left: row.left,
                top: row.top + bar_height + gap,
                width: row.width,
                height: bar_height,
            };

            (top, bottom)
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        (top, bottom): &'a (Area, Area),
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        if top.width <= 0.0 || top.height <= 0.0 {
            return;
        }

        let top_fraction = (*state.get(&self.top_fraction)).unwrap_or(0.0);
        let top_fill_color = *state.get(&self.top_fill_color);
        let top_background_color = *state.get(&self.top_background_color);
        draw_bar(layout, *top, top_fraction, top_fill_color, top_background_color);

        let bottom_fraction = (*state.get(&self.bottom_fraction)).unwrap_or(0.0);
        let bottom_fill_color = *state.get(&self.bottom_fill_color);
        let bottom_background_color = *state.get(&self.bottom_background_color);
        draw_bar(layout, *bottom, bottom_fraction, bottom_fill_color, bottom_background_color);
    }
}

#[cfg(test)]
mod tests {
    use super::fill_width;

    #[test]
    fn empty_bar_has_no_fill() {
        assert_eq!(fill_width(100.0, 8.0, 0.0), 0.0);
        assert_eq!(fill_width(100.0, 8.0, -0.5), 0.0);
    }

    #[test]
    fn full_bar_covers_the_whole_width() {
        assert_eq!(fill_width(100.0, 8.0, 1.0), 100.0);
        assert_eq!(fill_width(100.0, 8.0, 1.5), 100.0);
    }

    #[test]
    fn half_bar_covers_half_of_the_width() {
        assert_eq!(fill_width(100.0, 8.0, 0.5), 50.0);
        assert_eq!(fill_width(150.0, 5.0, 0.2), 30.0);
    }

    #[test]
    fn labeled_bar_splits_the_row_into_label_and_bar() {
        use korangar_interface::layout::area::Area;

        let row = Area {
            left: 10.0,
            top: 20.0,
            width: 200.0,
            height: 14.0,
        };
        let (label, bar) = super::split_labeled_bar(row, 30.0, 4.0);

        assert_eq!((label.left, label.top, label.width, label.height), (10.0, 20.0, 30.0, 14.0));
        assert_eq!((bar.left, bar.top, bar.width, bar.height), (44.0, 20.0, 166.0, 14.0));
    }

    #[test]
    fn labeled_bar_clamps_the_label_to_the_row_width() {
        use korangar_interface::layout::area::Area;

        let row = Area {
            left: 0.0,
            top: 0.0,
            width: 20.0,
            height: 10.0,
        };
        let (label, bar) = super::split_labeled_bar(row, 50.0, 4.0);

        assert_eq!(label.width, 20.0);
        // The bar never collapses to a negative width.
        assert_eq!(bar.width, 1.0);
    }

    #[test]
    fn tiny_progress_is_still_visible() {
        // One pixel of progress on a 100 px bar is drawn as a rounded cap.
        assert_eq!(fill_width(100.0, 8.0, 0.01), 8.0);
        // But the cap never exceeds the bar itself.
        assert_eq!(fill_width(4.0, 8.0, 0.01), 4.0);
    }
}
