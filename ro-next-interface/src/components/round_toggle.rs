use std::marker::PhantomData;

use rust_state::{Selector, State};

use crate::application::{Application, CornerDiameter};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut};
use crate::event::ClickHandler;
use crate::layout::alignment::HorizontalAlignment;
use crate::layout::area::Area;
use crate::layout::tooltip::TooltipExt;
use crate::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};

/// A compact round toggle button.
///
/// The button is drawn as a small circle: a ring when it is off and a solid
/// circle in the accent colour when it is on. It is intended for small on/off
/// settings that used to be rendered as large checkboxes, like the chat and
/// minimap transparency toggles.
///
/// The visual circle is `size` interface pixels in diameter while the whole
/// `hit_size` x `hit_size` square is clickable, so the button stays usable even
/// though it is small. All colours are supplied by the caller (the default is
/// taken from the `state_button` theme section) and are therefore correct for
/// both the default and the classic theme.
pub struct RoundToggle<Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> {
    tooltip_marker: PhantomData<Tooltip>,
    state: A,
    tooltip: B,
    event: C,
    size: D,
    hit_size: E,
    alignment: F,
    ring_color: G,
    hovered_ring_color: H,
    fill_color: I,
    checked_color: J,
    hovered_checked_color: K,
    shadow_color: L,
    shadow_padding: M,
}

impl<Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> RoundToggle<Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        state: A,
        tooltip: B,
        event: C,
        size: D,
        hit_size: E,
        alignment: F,
        ring_color: G,
        hovered_ring_color: H,
        fill_color: I,
        checked_color: J,
        hovered_checked_color: K,
        shadow_color: L,
        shadow_padding: M,
    ) -> Self {
        Self {
            tooltip_marker: PhantomData,
            state,
            tooltip,
            event,
            size,
            hit_size,
            alignment,
            ring_color,
            hovered_ring_color,
            fill_color,
            checked_color,
            hovered_checked_color,
            shadow_color,
            shadow_padding,
        }
    }
}

impl<App, Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M> Element<App> for RoundToggle<Tooltip, A, B, C, D, E, F, G, H, I, J, K, L, M>
where
    App: Application,
    Tooltip: AsRef<str> + 'static,
    A: Selector<App, bool>,
    B: Selector<App, Tooltip>,
    C: ClickHandler<App> + 'static,
    D: Selector<App, f32>,
    E: Selector<App, f32>,
    F: Selector<App, HorizontalAlignment>,
    G: Selector<App, App::Color>,
    H: Selector<App, App::Color>,
    I: Selector<App, App::Color>,
    J: Selector<App, App::Color>,
    K: Selector<App, App::Color>,
    L: Selector<App, App::Color>,
    M: Selector<App, App::ShadowPadding>,
{
    type LayoutInfo = RoundToggleLayoutInfo<App>;

    fn create_layout_info(&mut self, state: &State<App>, _: ElementStoreMut, resolvers: &mut dyn Resolvers<App>) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let size = *state.get(&self.size);
            // The clickable square must never be smaller than the visual circle,
            // otherwise the visible part of the button would not react to clicks.
            let hit_size = state.get(&self.hit_size).max(size);

            let row_area = resolver.with_height(hit_size);

            RoundToggleLayoutInfo {
                row_area,
                size,
                hit_size,
                alignment: *state.get(&self.alignment),
                circle_corner_diameter: circle_corner_diameter::<App>(size),
                inner_circle_size: size * 0.55,
                inner_circle_corner_diameter: circle_corner_diameter::<App>(size * 0.55),
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let hit_area = layout_info.hit_area();
        let is_hovered = hit_area.check().run(layout);

        if is_hovered {
            struct RoundToggleTooltip;

            let tooltip = state.get(&self.tooltip).as_ref();

            if !tooltip.is_empty() {
                layout.add_tooltip(tooltip, RoundToggleTooltip.tooltip_id());
            }

            layout.register_click_handler(MouseButton::Left, &self.event);
        }

        let shadow_color = *state.get(&self.shadow_color);
        let shadow_padding = *state.get(&self.shadow_padding);
        let circle_area = layout_info.circle_area();

        match *state.get(&self.state) {
            true => {
                let color = match is_hovered {
                    true => *state.get(&self.hovered_checked_color),
                    false => *state.get(&self.checked_color),
                };

                layout.add_rectangle(
                    circle_area,
                    layout_info.circle_corner_diameter,
                    color,
                    shadow_color,
                    shadow_padding,
                );
            }
            false => {
                // Draw a ring: an outer circle in the outline colour with an
                // inner circle in the fill colour on top of it.
                let ring_color = match is_hovered {
                    true => *state.get(&self.hovered_ring_color),
                    false => *state.get(&self.ring_color),
                };
                let fill_color = *state.get(&self.fill_color);

                layout.add_rectangle(
                    circle_area,
                    layout_info.circle_corner_diameter,
                    ring_color,
                    shadow_color,
                    shadow_padding,
                );

                let inner_circle_offset = (layout_info.size - layout_info.inner_circle_size) / 2.0;

                layout.add_rectangle(
                    Area {
                        left: circle_area.left + inner_circle_offset,
                        top: circle_area.top + inner_circle_offset,
                        width: layout_info.inner_circle_size,
                        height: layout_info.inner_circle_size,
                    },
                    layout_info.inner_circle_corner_diameter,
                    fill_color,
                    shadow_color,
                    shadow_padding,
                );
            }
        }
    }
}

fn circle_corner_diameter<App: Application>(diameter: f32) -> App::CornerDiameter {
    // The renderer halves the corner diameter before rendering it as the corner
    // radius, so passing the full diameter produces a circle.
    CornerDiameter::new(diameter, diameter, diameter, diameter)
}

pub struct RoundToggleLayoutInfo<App: Application> {
    row_area: Area,
    size: f32,
    hit_size: f32,
    alignment: HorizontalAlignment,
    circle_corner_diameter: App::CornerDiameter,
    inner_circle_size: f32,
    inner_circle_corner_diameter: App::CornerDiameter,
}

impl<App: Application> RoundToggleLayoutInfo<App> {
    fn hit_area(&self) -> Area {
        let left = match self.alignment {
            HorizontalAlignment::Left { offset, border } => self.row_area.left + border + offset,
            HorizontalAlignment::Center { offset, .. } => self.row_area.left + (self.row_area.width - self.hit_size) / 2.0 + offset,
            HorizontalAlignment::Right { offset, border } => self.row_area.left + self.row_area.width - self.hit_size - border + offset,
        };

        Area {
            left,
            top: self.row_area.top,
            width: self.hit_size,
            height: self.hit_size,
        }
    }

    fn circle_area(&self) -> Area {
        let hit_area = self.hit_area();
        let offset = (self.hit_size - self.size) / 2.0;

        Area {
            left: hit_area.left + offset,
            top: hit_area.top + offset,
            width: self.size,
            height: self.size,
        }
    }
}
