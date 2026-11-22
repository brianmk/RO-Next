//! Interactive horizontal slider.
//!
//! The slider is used for percentage settings such as the audio volumes. It is
//! a compact, single-line element that draws a rounded track, a rounded fill
//! and the current value as text on top of the track:
//!
//! ```text
//! [=========== 60% -------]
//! ```
//!
//! Interaction:
//!
//! * clicking anywhere on the track immediately sets the value to the clicked
//!   position,
//! * pressing and dragging keeps updating the value even when the pointer
//!   leaves the track (the drag ends on mouse release),
//! * scrolling the mouse wheel over the slider changes the value in 5 % steps.
//!
//! The value is stored through a [`Path`] as a [`SliderValue`], which is
//! supported for `u8` (percent, `0..=100`) and `f32` (linear, `0.0..=1.0`). The
//! component only ever writes clamped values, so a slider can never store an
//! out-of-range or non-finite value.
//!
//! Colors, height, font and alignment come from [`SliderTheme`], which makes
//! the component usable with the flat theme, the classic skin and custom theme
//! files without hardcoded literals in the windows.

use std::cell::{Cell, UnsafeCell};
use std::fmt::Write;
use std::marker::PhantomData;

use rust_state::{Path, RustState, Selector, State};

use crate::application::{Application, CornerDiameter, Position, Size};
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::element::{DefaultLayoutInfo, Element};
use crate::event::{ClickHandler, ScrollHandler};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};

/// Number of percent points that one mouse wheel step changes.
const SCROLL_STEP_PERCENT: u8 = 5;

/// A scalar that a [`Slider`] can edit.
///
/// The trait maps the value onto a percentage in `0..=100` and back, which is
/// the unit that the slider renders and interacts with. Implementations must
/// clamp their input so that a slider can never store an invalid value.
pub trait SliderValue: Copy + 'static {
    /// The value as a percentage in `0..=100`.
    fn percent(self) -> u8;

    /// Create a value from a percentage.
    ///
    /// `percent` is always in `0..=100` when the slider calls this, but
    /// implementations should still clamp defensively.
    fn from_percent(percent: u8) -> Self;
}

impl SliderValue for u8 {
    fn percent(self) -> u8 {
        self.min(100)
    }

    fn from_percent(percent: u8) -> Self {
        percent.min(100)
    }
}

impl SliderValue for f32 {
    fn percent(self) -> u8 {
        if !self.is_finite() {
            return 0;
        }

        (self.clamp(0.0, 1.0) * 100.0).round() as u8
    }

    fn from_percent(percent: u8) -> Self {
        percent.min(100) as f32 / 100.0
    }
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SliderTheme<App>
where
    App: Application,
{
    /// Background of the part of the track that is not filled.
    pub track_color: App::Color,
    /// Background of the track while the pointer hovers it.
    pub hovered_track_color: App::Color,
    /// Color of the filled part of the track.
    pub fill_color: App::Color,
    /// Color of the filled part of the track while the pointer hovers it.
    pub hovered_fill_color: App::Color,
    /// Color of the percentage text on the track.
    pub foreground_color: App::Color,
    /// Highlight color of the percentage text.
    pub highlight_color: App::Color,
    /// Drop shadow of the track.
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    /// Drop shadow of the filled part. Usually transparent, so the fill sits
    /// flat on the track.
    pub fill_shadow_color: App::Color,
    pub fill_shadow_padding: App::ShadowPadding,
    /// Height of the slider, in interface pixels.
    pub height: f32,
    pub corner_diameter: App::CornerDiameter,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
}

/// Scroll wheel handler of a slider. Registered while the pointer hovers the
/// slider so that scrolling anywhere else in the window is unaffected.
struct SliderScrollHandler<Value, A> {
    value_path: A,
    _marker: PhantomData<Value>,
}

impl<App, Value, A> ScrollHandler<App> for SliderScrollHandler<Value, A>
where
    App: Application,
    Value: SliderValue,
    A: Path<App, Value>,
{
    fn handle_scroll(&self, state: &State<App>, _: &mut crate::event::EventQueue<App>, delta: f32) -> bool {
        if delta == 0.0 {
            return false;
        }

        let current = state.get(&self.value_path).percent();
        let next = match delta > 0.0 {
            true => current.saturating_add(SCROLL_STEP_PERCENT).min(100),
            false => current.saturating_sub(SCROLL_STEP_PERCENT),
        };

        if next != current {
            state.update_value(self.value_path, Value::from_percent(next));
        }

        true
    }
}

/// Click handler of a slider.
///
/// The slider also follows a held mouse button during layout (so dragging works
/// across frames and outside the track), but a fast click can press and release
/// the button between two frames. In that case the layout only sees the click
/// event, so the click handler applies the value once from the position and
/// track bounds that were recorded during the last layout pass.
struct SliderClickHandler<Value, A> {
    value_path: A,
    area_left: Cell<f32>,
    area_width: Cell<f32>,
    mouse_left: Cell<f32>,
    _marker: PhantomData<Value>,
}

impl<App, Value, A> ClickHandler<App> for SliderClickHandler<Value, A>
where
    App: Application,
    Value: SliderValue,
    A: Path<App, Value>,
{
    fn handle_click(&self, state: &State<App>, _: &mut crate::event::EventQueue<App>) {
        let area_width = self.area_width.get();

        if area_width <= 0.0 {
            return;
        }

        let fraction = ((self.mouse_left.get() - self.area_left.get()) / area_width).clamp(0.0, 1.0);
        let percent = (fraction * 100.0).round().clamp(0.0, 100.0) as u8;
        let current = state.get(&self.value_path).percent();

        if percent != current {
            state.update_value(self.value_path, Value::from_percent(percent));
        }
    }
}

pub struct Slider<Value, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q> {
    value: A,
    track_color: B,
    hovered_track_color: C,
    fill_color: D,
    hovered_fill_color: E,
    foreground_color: F,
    highlight_color: G,
    shadow_color: H,
    shadow_padding: I,
    fill_shadow_color: J,
    fill_shadow_padding: K,
    height: L,
    corner_diameter: M,
    font_size: N,
    horizontal_alignment: O,
    vertical_alignment: P,
    overflow_behavior: Q,
    /// Whether the current mouse drag started on (or over) this slider.
    dragging: Cell<bool>,
    /// Cache for the formatted percentage text. `add_text` borrows the string
    /// for the lifetime of the layout, so it cannot be a local `String`.
    text: UnsafeCell<String>,
    scroll_handler: SliderScrollHandler<Value, A>,
    click_handler: SliderClickHandler<Value, A>,
}

impl<Value, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q> Slider<Value, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        value: A,
        track_color: B,
        hovered_track_color: C,
        fill_color: D,
        hovered_fill_color: E,
        foreground_color: F,
        highlight_color: G,
        shadow_color: H,
        shadow_padding: I,
        fill_shadow_color: J,
        fill_shadow_padding: K,
        height: L,
        corner_diameter: M,
        font_size: N,
        horizontal_alignment: O,
        vertical_alignment: P,
        overflow_behavior: Q,
    ) -> Self
    where
        A: Copy,
    {
        Self {
            value,
            track_color,
            hovered_track_color,
            fill_color,
            hovered_fill_color,
            foreground_color,
            highlight_color,
            shadow_color,
            shadow_padding,
            fill_shadow_color,
            fill_shadow_padding,
            height,
            corner_diameter,
            font_size,
            horizontal_alignment,
            vertical_alignment,
            overflow_behavior,
            dragging: Cell::new(false),
            text: UnsafeCell::default(),
            scroll_handler: SliderScrollHandler {
                value_path: value,
                _marker: PhantomData,
            },
            click_handler: SliderClickHandler {
                value_path: value,
                area_left: Cell::new(0.0),
                area_width: Cell::new(0.0),
                mouse_left: Cell::new(0.0),
                _marker: PhantomData,
            },
        }
    }
}

/// The width of the filled part of the track.
///
/// The result is clamped to the track width, and a non-zero value is at least
/// one track height wide so that a small percentage is still visible as a
/// rounded cap instead of disappearing completely.
fn fill_width(track_width: f32, track_height: f32, percent: u8) -> f32 {
    if percent == 0 || track_width <= 0.0 {
        return 0.0;
    }

    (track_width * (percent.min(100) as f32 / 100.0))
        .max(track_height.min(track_width))
        .min(track_width)
}

impl<App, Value, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q> Element<App>
    for Slider<Value, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q>
where
    App: Application,
    Value: SliderValue,
    A: Path<App, Value>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::ShadowPadding>,
    J: Selector<App, App::Color>,
    K: Selector<App, App::ShadowPadding>,
    L: Selector<App, f32>,
    M: Selector<App, App::CornerDiameter>,
    N: Selector<App, App::FontSize>,
    O: Selector<App, HorizontalAlignment>,
    P: Selector<App, VerticalAlignment>,
    Q: Selector<App, App::OverflowBehavior>,
{
    type LayoutInfo = DefaultLayoutInfo<App>;

    fn create_layout_info(&mut self, state: &State<App>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let height = *state.get(&self.height);
            let font_size = *state.get(&self.font_size);
            let foreground_color = *state.get(&self.foreground_color);
            let highlight_color = *state.get(&self.highlight_color);
            let horizontal_alignment = *state.get(&self.horizontal_alignment);
            let overflow_behavior = *state.get(&self.overflow_behavior);

            // Measure the widest label ("100%") so the slider is at least tall
            // enough for the percentage text in every font/theme combination.
            let (text_size, font_size) = resolver.get_text_dimensions(
                "100%",
                foreground_color,
                highlight_color,
                font_size,
                horizontal_alignment,
                overflow_behavior,
            );

            let area = resolver.with_height(height.max(text_size.height()));

            DefaultLayoutInfo { area, font_size }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let area = layout_info.area;
        let is_hovered = area.check().run(layout);

        let value = *state.get(&self.value);
        let percent = value.percent();

        self.click_handler.area_left.set(area.left);
        self.click_handler.area_width.set(area.width);
        self.click_handler.mouse_left.set(layout.get_mouse_position().left());

        // Interaction. The interface feeds the current primary mouse button
        // state into the layout, so a slider can follow a drag frame by frame
        // even when the pointer leaves the track. The value is only written
        // when it actually changes.
        if layout.primary_button_down() {
            if is_hovered {
                self.dragging.set(true);
            }

            if self.dragging.get() {
                let mouse_left = layout.get_mouse_position().left();
                let fraction = if area.width > 0.0 {
                    ((mouse_left - area.left) / area.width).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let new_percent = (fraction * 100.0).round().clamp(0.0, 100.0) as u8;

                if new_percent != percent {
                    state.update_value(self.value, Value::from_percent(new_percent));
                }
            }
        } else {
            self.dragging.set(false);
        }

        if is_hovered {
            layout.register_click_handler(MouseButton::Left, &self.click_handler);
            layout.register_scroll_handler(&self.scroll_handler);
        }

        let corner_diameter = *state.get(&self.corner_diameter);
        let track_color = match is_hovered {
            true => *state.get(&self.hovered_track_color),
            false => *state.get(&self.track_color),
        };
        let fill_color = match is_hovered {
            true => *state.get(&self.hovered_fill_color),
            false => *state.get(&self.fill_color),
        };

        layout.add_rectangle(
            area,
            corner_diameter,
            track_color,
            *state.get(&self.shadow_color),
            *state.get(&self.shadow_padding),
        );

        let width = fill_width(area.width, area.height, percent);

        if width > 0.0 {
            // Keep the rounded corners of the fill from sticking out of the
            // (possibly very narrow) filled area.
            let radius = width.min(area.height) / 2.0;
            let fill_corner = App::CornerDiameter::new(
                corner_diameter.top_left().min(radius),
                corner_diameter.top_right().min(radius),
                corner_diameter.bottom_right().min(radius),
                corner_diameter.bottom_left().min(radius),
            );

            layout.add_rectangle(
                Area {
                    left: area.left,
                    top: area.top,
                    width,
                    height: area.height,
                },
                fill_corner,
                fill_color,
                *state.get(&self.fill_shadow_color),
                *state.get(&self.fill_shadow_padding),
            );
        }

        // SAFETY: the string is only accessed from the layout pass of this
        // element, which is single threaded, and it is not aliased while the
        // reference is alive.
        let text: &'a str = unsafe {
            let text = &mut *self.text.get();
            text.clear();
            let _ = write!(text, "{percent}%");
            text.as_str()
        };

        layout.add_text(
            area,
            text,
            layout_info.font_size,
            *state.get(&self.foreground_color),
            *state.get(&self.highlight_color),
            *state.get(&self.horizontal_alignment),
            *state.get(&self.vertical_alignment),
            *state.get(&self.overflow_behavior),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{SliderValue, fill_width};

    #[test]
    fn u8_slider_value_is_clamped_to_percent() {
        assert_eq!(0u8.percent(), 0);
        assert_eq!(55u8.percent(), 55);
        assert_eq!(100u8.percent(), 100);
        assert_eq!(255u8.percent(), 100);
        assert_eq!(u8::from_percent(0), 0);
        assert_eq!(u8::from_percent(73), 73);
        assert_eq!(u8::from_percent(255), 100);
    }

    #[test]
    fn f32_slider_value_maps_between_linear_and_percent() {
        assert_eq!(0.0f32.percent(), 0);
        assert_eq!(0.256f32.percent(), 26);
        assert_eq!(1.0f32.percent(), 100);
        assert_eq!(1.5f32.percent(), 100);
        assert_eq!((-0.5f32).percent(), 0);
        assert_eq!(f32::NAN.percent(), 0);
        assert_eq!(f32::from_percent(0), 0.0);
        assert_eq!(f32::from_percent(50), 0.5);
        assert_eq!(f32::from_percent(100), 1.0);
        assert_eq!(f32::from_percent(255), 1.0);
    }

    #[test]
    fn fill_width_is_clamped_and_never_disappears_for_small_values() {
        assert_eq!(fill_width(200.0, 20.0, 0), 0.0);
        assert_eq!(fill_width(200.0, 20.0, 100), 200.0);
        assert_eq!(fill_width(200.0, 20.0, 50), 100.0);
        // Small non-zero values still get a visible rounded cap.
        assert_eq!(fill_width(200.0, 20.0, 1), 20.0);
        assert_eq!(fill_width(10.0, 20.0, 1), 10.0);
        // Values above 100 % are clamped.
        assert_eq!(fill_width(200.0, 20.0, 255), 200.0);
        // Degenerate widths do not produce NaN or negative widths.
        assert_eq!(fill_width(0.0, 20.0, 50), 0.0);
        assert_eq!(fill_width(-10.0, 20.0, 50), 0.0);
    }
}
