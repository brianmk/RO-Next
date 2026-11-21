use std::cell::RefCell;

use num::Signed;
use rust_state::{Selector, State};

use crate::application::{Application, CornerDiameter, ShadowPadding};
use crate::element::Element;
use crate::element::store::{ElementStore, ElementStoreMut, Persistent, PersistentExt};
use crate::event::ScrollHandler;
use crate::layout::area::Area;
use crate::layout::{Resolvers, WindowLayout, with_single_resolver};
use crate::prelude::EventQueue;
use crate::theme::{ThemePathGetter, theme};
use crate::window::WindowThemePathExt;

/// If the current scroll is this far away from the maximum scroll the scroll
/// view will start following.
const FOLLOW_THRESHOLD: f32 = 0.5;

/// Width of the scroll bar that is drawn on the right side of a scroll view
/// that opted into showing one.
const SCROLL_BAR_WIDTH: f32 = 10.0;

/// Padding between the scroll bar track and the thumb.
const SCROLL_BAR_PADDING: f32 = 2.0;

/// The thumb never becomes smaller than this, even for very long content.
const SCROLL_BAR_MINIMUM_THUMB_HEIGHT: f32 = 24.0;

struct PersistentDataInner {
    scroll: f32,
    maximum_scroll: f32,
    actively_following: bool,
}

impl Default for PersistentDataInner {
    fn default() -> Self {
        Self {
            scroll: 0.0,
            maximum_scroll: 0.0,
            actively_following: true,
        }
    }
}

#[derive(Default)]
pub struct PersistentData {
    inner: RefCell<PersistentDataInner>,
}

impl<App> ScrollHandler<App> for PersistentData
where
    App: Application,
{
    fn handle_scroll(&self, _: &State<App>, _: &mut EventQueue<App>, delta: f32) -> bool {
        let mut inner = self.inner.borrow_mut();

        // Don't try to scroll if its already at the minimum or maximum scroll value.
        if delta.is_negative() && inner.scroll >= inner.maximum_scroll || delta.is_positive() && inner.scroll <= 0.0 {
            return false;
        }

        inner.scroll = (inner.scroll - delta).max(0.0).min(inner.maximum_scroll);

        if delta.is_positive() && inner.scroll <= inner.maximum_scroll - FOLLOW_THRESHOLD {
            inner.actively_following = false;
        } else if delta.is_negative() && inner.scroll >= inner.maximum_scroll - FOLLOW_THRESHOLD {
            inner.actively_following = true;
        }

        true
    }
}

/// Geometry of the scroll bar that is drawn on the right side of the scroll
/// view.
struct ScrollBarLayoutInfo {
    track: Area,
    thumb: Area,
}

pub struct ScrollViewLayoutInfo<L> {
    area: Area,
    children: L,
    scroll_bar: Option<ScrollBarLayoutInfo>,
}

pub struct ScrollView<A, Children> {
    follow: A,
    scroll_bar: bool,
    fit_content_height: bool,
    children: Children,
}

impl<A, Children> ScrollView<A, Children> {
    /// This function is supposed to be called from a component macro and not
    /// intended to be called manually.
    #[inline(always)]
    pub fn component_new(follow: A, scroll_bar: bool, fit_content_height: bool, children: Children) -> Self {
        Self {
            follow,
            scroll_bar,
            fit_content_height,
            children,
        }
    }
}

/// Creates the geometry of the scroll bar for the given content height.
///
/// Returns `None` if the content fits into the scroll view and the bar is
/// therefore not needed.
fn create_scroll_bar_layout(area: Area, children_height: f32, scroll: f32, maximum_scroll: f32) -> Option<ScrollBarLayoutInfo> {
    if maximum_scroll <= 0.0 {
        return None;
    }

    let track = Area {
        left: area.left + area.width - SCROLL_BAR_WIDTH,
        top: area.top,
        width: SCROLL_BAR_WIDTH,
        height: area.height,
    };

    let thumb_height = (area.height * area.height / children_height)
        .max(SCROLL_BAR_MINIMUM_THUMB_HEIGHT)
        .min(area.height);
    let thumb_scroll_range = area.height - thumb_height;
    let thumb_top = area.top + (scroll / maximum_scroll) * thumb_scroll_range;

    let thumb = Area {
        left: track.left + SCROLL_BAR_PADDING,
        top: thumb_top + SCROLL_BAR_PADDING,
        width: track.width - SCROLL_BAR_PADDING * 2.0,
        height: thumb_height - SCROLL_BAR_PADDING * 2.0,
    };

    Some(ScrollBarLayoutInfo { track, thumb })
}

impl<A, Children> Persistent for ScrollView<A, Children> {
    type Data = PersistentData;
}

impl<App, A, Children> Element<App> for ScrollView<A, Children>
where
    A: Selector<App, bool>,
    App: Application,
    Children: Element<App>,
{
    type LayoutInfo = ScrollViewLayoutInfo<Children::LayoutInfo>;

    fn create_layout_info(
        &mut self,
        state: &State<App>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<App>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            let show_scroll_bar = *state.get(&self.scroll_bar);
            let fit_content_height = *state.get(&self.fit_content_height);

            // Set to true once the content turned out to be taller than the
            // scroll view and a scroll bar has to be reserved on the right
            // side. The children are then laid out again in a narrower area.
            let mut reserve_scroll_bar = false;

            loop {
                let persistent = self.get_persistent_data(&store, ());
                let scroll = persistent.inner.borrow().scroll;

                // In case that we need to resolve twice we don't want to start with the same
                // resolver state as the first iteration, so we clone it here and adding it back
                // as soon as a correct layout was found. This is a little bit
                // ugly and might be improved in the future.
                let mut cloned_resolver = resolver.clone();

                if reserve_scroll_bar {
                    cloned_resolver.reduce_available_width(SCROLL_BAR_WIDTH);
                }

                // HACK: Since this loop might run multiple times we need a new store every
                // time. This is a bit wasteful and I would like to solve this more
                // elegantly.
                let child_store = store.child_store(0);

                let (children_area, children_height, layout_info) = cloned_resolver.with_derived_scrolled(scroll, |resolver| {
                    self.children.create_layout_info(state, child_store, resolver)
                });

                // The scroll view itself is as wide as the area the children were
                // laid out in, plus the scroll bar if one is shown.
                let full_width = children_area.width + if reserve_scroll_bar { SCROLL_BAR_WIDTH } else { 0.0 };
                let available_height = children_area.height;

                let persistent = self.get_persistent_data(&store, ());
                let mut inner = persistent.inner.borrow_mut();

                let maximum_scroll = (children_height - available_height).max(0.0);

                // Check if the scroll is in bounds. If it is, we can just return, otherwise we
                // need to adjust it and create the layout again.
                if inner.scroll > maximum_scroll {
                    inner.scroll = maximum_scroll;
                    continue;
                } else if inner.scroll < 0.0 {
                    inner.scroll = 0.0;
                    continue;
                } else if *state.get(&self.follow) && inner.actively_following && inner.scroll <= maximum_scroll - FOLLOW_THRESHOLD {
                    inner.scroll = maximum_scroll;
                    continue;
                }

                // The content does not fit into the scroll view: reserve the
                // space for the scroll bar and lay out the children again so
                // that they do not end up below the scroll bar.
                if show_scroll_bar && !reserve_scroll_bar && maximum_scroll > 0.0 {
                    reserve_scroll_bar = true;
                    continue;
                }

                // A scroll view that fits its content consumes only as much
                // vertical space as its content needs, so a dialog with a
                // single text line does not stretch over the whole screen.
                // Without this the scroll view always consumes the full
                // available height.
                let consumed_height = match fit_content_height {
                    true => children_height.min(available_height),
                    false => available_height,
                };

                if consumed_height < available_height {
                    cloned_resolver.release_used_height(available_height - consumed_height);
                }

                inner.maximum_scroll = maximum_scroll;

                let area = Area {
                    left: children_area.left,
                    top: children_area.top,
                    width: full_width,
                    height: consumed_height,
                };

                let scroll_bar = match reserve_scroll_bar {
                    true => create_scroll_bar_layout(area, children_height, inner.scroll, maximum_scroll),
                    false => None,
                };

                *resolver = cloned_resolver;

                return ScrollViewLayoutInfo {
                    area,
                    children: layout_info,
                    scroll_bar,
                };
            }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<App>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, App>,
    ) {
        let persistent = self.get_persistent_data(&store, ());

        if layout_info.area.check().dont_mark().run(layout) {
            layout.register_scroll_handler(persistent);
        }

        layout.with_clip(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                // HACK: We need to do the same as in `create_layout_info`.
                self.children.lay_out(state, store.child_store(0), &layout_info.children, layout);
            });
        });

        if let Some(scroll_bar) = &layout_info.scroll_bar {
            let track_color = *state.get(&theme().window().anchor_color());
            let thumb_color = *state.get(&theme().window().highlight_color());
            let shadow_color = *state.get(&theme().window().shadow_color());
            let shadow_padding = App::ShadowPadding::none();

            layout.add_rectangle(
                scroll_bar.track,
                App::CornerDiameter::new(
                    SCROLL_BAR_WIDTH / 2.0,
                    SCROLL_BAR_WIDTH / 2.0,
                    SCROLL_BAR_WIDTH / 2.0,
                    SCROLL_BAR_WIDTH / 2.0,
                ),
                track_color,
                shadow_color,
                shadow_padding,
            );

            layout.add_rectangle(
                scroll_bar.thumb,
                App::CornerDiameter::new(SCROLL_BAR_PADDING, SCROLL_BAR_PADDING, SCROLL_BAR_PADDING, SCROLL_BAR_PADDING),
                thumb_color,
                shadow_color,
                shadow_padding,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scroll_view_area() -> Area {
        Area {
            left: 10.0,
            top: 20.0,
            width: 100.0,
            height: 50.0,
        }
    }

    #[test]
    fn no_scroll_bar_is_created_when_the_content_fits() {
        assert!(create_scroll_bar_layout(scroll_view_area(), 50.0, 0.0, 0.0).is_none());
        assert!(create_scroll_bar_layout(scroll_view_area(), 30.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn the_scroll_bar_is_placed_on_the_right_side_of_the_scroll_view() {
        let area = scroll_view_area();
        let scroll_bar = create_scroll_bar_layout(area, 500.0, 0.0, 450.0).expect("long content needs a scroll bar");

        assert_eq!(scroll_bar.track.width, SCROLL_BAR_WIDTH);
        assert_eq!(scroll_bar.track.left, area.left + area.width - SCROLL_BAR_WIDTH);
        assert_eq!(scroll_bar.track.top, area.top);
        assert_eq!(scroll_bar.track.height, area.height);
        assert!(scroll_bar.thumb.height < scroll_bar.track.height);
        assert_eq!(scroll_bar.thumb.left, scroll_bar.track.left + SCROLL_BAR_PADDING);
        assert_eq!(scroll_bar.thumb.width, SCROLL_BAR_WIDTH - SCROLL_BAR_PADDING * 2.0);
    }

    #[test]
    fn the_thumb_is_at_the_top_when_scrolled_to_the_top() {
        let area = scroll_view_area();
        let scroll_bar = create_scroll_bar_layout(area, 500.0, 0.0, 450.0).unwrap();

        assert!((scroll_bar.thumb.top - (area.top + SCROLL_BAR_PADDING)).abs() < 0.001);
    }

    #[test]
    fn the_thumb_moves_down_when_scrolled_to_the_bottom() {
        let area = scroll_view_area();
        let top = create_scroll_bar_layout(area, 500.0, 0.0, 450.0).unwrap();
        let bottom = create_scroll_bar_layout(area, 500.0, 450.0, 450.0).unwrap();

        assert!(bottom.thumb.top > top.thumb.top);
        assert!((bottom.thumb.top + bottom.thumb.height - (area.top + area.height - SCROLL_BAR_PADDING)).abs() < 0.001);
    }

    #[test]
    fn the_thumb_never_becomes_smaller_than_the_minimum_height() {
        let scroll_bar = create_scroll_bar_layout(scroll_view_area(), 100_000.0, 0.0, 99_950.0).unwrap();

        assert_eq!(
            scroll_bar.thumb.height,
            SCROLL_BAR_MINIMUM_THUMB_HEIGHT - SCROLL_BAR_PADDING * 2.0
        );
    }
}
