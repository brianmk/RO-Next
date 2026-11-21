//! Horizontal progress bar element.
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
use rust_state::{Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::state::ClientState;

/// Fraction of the health bar below which the "low health" color is used.
pub const LOW_HEALTH_FRACTION: f32 = 0.25;

/// A horizontal bar that is filled according to the progress in `[0, 1]`.
pub struct ProgressBar<Fraction, FillColor, BackgroundColor> {
    fraction: Fraction,
    fill_color: FillColor,
    background_color: BackgroundColor,
    height: f32,
}

impl<Fraction, FillColor, BackgroundColor> ProgressBar<Fraction, FillColor, BackgroundColor>
where
    Fraction: Selector<ClientState, Option<f32>>,
    FillColor: Selector<ClientState, Color>,
    BackgroundColor: Selector<ClientState, Color>,
{
    pub fn new(fraction: Fraction, fill_color: FillColor, background_color: BackgroundColor, height: f32) -> Self {
        Self {
            fraction,
            fill_color,
            background_color,
            height,
        }
    }
}

/// Width of the filled part of a bar in interface pixels.
///
/// Tiny progress values are clamped to at least one bar height, so a small
/// amount of progress shows up as a rounded cap instead of disappearing
/// completely.
fn fill_width(bar_width: f32, bar_height: f32, fraction: f32) -> f32 {
    let fraction = fraction.clamp(0.0, 1.0);

    if fraction <= 0.0 || bar_width <= 0.0 {
        return 0.0;
    }

    (bar_width * fraction).max(bar_height.min(bar_width)).min(bar_width)
}

impl<Fraction, FillColor, BackgroundColor> Element<ClientState> for ProgressBar<Fraction, FillColor, BackgroundColor>
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
        layout_info: &'a Area,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let area = *layout_info;

        if area.width <= 0.0 || area.height <= 0.0 {
            return;
        }

        let fraction = (*state.get(&self.fraction)).unwrap_or(0.0);
        let fill_color = *state.get(&self.fill_color);
        let background_color = *state.get(&self.background_color);

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
    fn tiny_progress_is_still_visible() {
        // One pixel of progress on a 100 px bar is drawn as a rounded cap.
        assert_eq!(fill_width(100.0, 8.0, 0.01), 8.0);
        // But the cap never exceeds the bar itself.
        assert_eq!(fill_width(4.0, 8.0, 0.01), 4.0);
    }
}
