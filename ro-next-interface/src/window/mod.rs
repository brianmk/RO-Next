mod anchor;
pub mod store;

use std::marker::PhantomData;

pub use anchor::{Anchor, AnchorPoint};
pub use interface_macros::StateWindow;
use rust_state::{Path, RustState, Selector, State};
use store::WindowStore;

use crate::MouseMode;
use crate::application::{Application, CornerDiameter, Position, ShadowPadding, Size};
use crate::element::Element;
use crate::element::id::ElementIdGenerator;
use crate::element::store::ElementStore;
use crate::event::{ClickHandler, Event};
use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
use crate::layout::area::Area;
use crate::layout::{MouseButton, ResizeMode, Resolver, WindowLayout};
use crate::prelude::EventQueue;
use crate::theme::{ThemePathGetter, theme};

mod private {
    /// Sealed trait to avoid outside implementations of
    /// [`Window`](super::Window).
    pub trait Sealed {}
}

/// Whether at least one foreground interface window is currently open.
///
/// The keyboard input system does not have access to the [`Interface`]
/// instance, but it has to know whether Escape should close the topmost window
/// (if any) or open the menu (if no window is open). Permanently visible HUD
/// overlays (chat, hotbar, minimap, ...) do not count as foreground windows;
/// the application decides which classes those are through
/// [`Application::is_background_window_class`](crate::application::Application::is_background_window_class).
///
/// The interface publishes this flag every time it lays out its windows, so the
/// decision is at most one frame stale.
///
/// [`Interface`]: crate::Interface
static HAS_FOREGROUND_WINDOWS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Publish whether at least one foreground window is open. Called by the
/// interface each frame; input handling only reads it.
pub fn set_has_foreground_windows(has_foreground_windows: bool) {
    HAS_FOREGROUND_WINDOWS.store(has_foreground_windows, std::sync::atomic::Ordering::Relaxed);
}

/// Returns whether at least one foreground interface window is currently open.
///
/// See [`set_has_foreground_windows`].
pub fn has_foreground_windows() -> bool {
    HAS_FOREGROUND_WINDOWS.load(std::sync::atomic::Ordering::Relaxed)
}

/// This trait is only used to abstract over [`WindowInternal`] and it's
/// generics and is not supposed to be implemented outised this crate.
/// Window sizes use `f32::MAX` as a "fit to content" sentinel; keep layout math
/// finite by replacing non finite values with `0.0`.
fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub trait Window<App: Application>: private::Sealed {
    /// Get the window class of the window (if any).
    fn get_class(&self) -> Option<App::WindowClass>;

    /// Get the window theme.
    fn get_theme_type(&self) -> App::ThemeType;

    /// Returns if the window is closable or not.
    fn is_closable(&self, state: &State<App>) -> bool;

    /// Create the layout info for the window.
    #[allow(private_interfaces)]
    fn create_layout_info(
        &mut self,
        state: &State<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
        text_layouter: &App::TextLayouter,
        window_size: App::Size,
    ) -> DisplayInformation;

    /// Lay out the window.
    #[allow(private_interfaces)]
    fn lay_out<'a>(&'a self, state: &'a State<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut WindowLayout<'a, App>);
}

/// An application specific custom window.
///
/// Can be passed to [`open_window`](crate::Interface::open_window).
pub trait CustomWindow<App>
where
    App: Application,
{
    /// Return the window class of the window (if any).
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    /// Convert to a real window.
    fn to_window<'a>(self) -> impl Window<App> + 'a;
}

/// A window for inspecting a part of the application state.
///
/// Can be passed to [`open_state_window`](crate::Interface::open_state_window).
///
/// Implementing this trait is generally discouraged, see [`CustomWindow`] for
/// custom windows.
pub trait StateWindow<App>
where
    App: Application,
{
    /// Return the window class of the window (if any).
    fn window_class() -> Option<App::WindowClass> {
        None
    }

    /// Create a new immutable inspector window of the state.
    fn to_window<'a>(self_path: impl Path<App, Self>) -> impl Window<App> + 'a;

    /// Create a new mutable inspector window of the state.
    fn to_window_mut<'a>(self_path: impl Path<App, Self>) -> impl Window<App> + 'a;
}

#[derive(RustState)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WindowTheme<App>
where
    App: Application,
{
    pub title_color: App::Color,
    pub hovered_title_color: App::Color,
    pub background_color: App::Color,
    pub highlight_color: App::Color,
    pub shadow_color: App::Color,
    pub shadow_padding: App::ShadowPadding,
    pub gaps: f32,
    pub border: f32,
    pub corner_diameter: App::CornerDiameter,
    pub close_button_size: App::Size,
    pub close_button_corner_diameter: App::CornerDiameter,
    pub minimum_width: f32,
    pub maximum_width: f32,
    pub minimum_height: f32,
    pub maximum_height: f32,
    pub title_height: f32,
    pub title_gap: f32,
    pub font_size: App::FontSize,
    pub horizontal_alignment: HorizontalAlignment,
    pub vertical_alignment: VerticalAlignment,
    pub overflow_behavior: App::OverflowBehavior,
    pub anchor_color: App::Color,
    pub closest_anchor_color: App::Color,
}

pub struct WindowData<App>
where
    App: Application,
{
    pub id: u64,
    pub anchor: Anchor<App>,
    pub size: App::Size,
    /// Concrete type name of the window, only used for `-vvv` log lines.
    pub debug_name: &'static str,
    /// Set once the "clamped stored size" trace line was printed, so `-vvv`
    /// does not repeat it every frame.
    pub size_clamp_logged: bool,
}

pub(crate) struct DisplayInformation {
    pub real_area: Area,
    pub display_height: f32,
}

pub struct WindowLayoutInfoSet<T> {
    area: Area,
    title_area: Area,
    children: T,
}

/// Which vertical edge of a window carries the vertical resize grip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VerticalResizeAnchor {
    /// The grip is on the bottom edge (default for all windows).
    #[default]
    Bottom,
    /// The grip is on the top edge; dragging up grows the window (used by the
    /// chat window, whose input box sits at the bottom).
    Top,
}

struct ResizeClickHandler {
    window_id: u64,
    resize_mode: ResizeMode,
}

impl ResizeClickHandler {
    fn new(resize_mode: ResizeMode) -> Self {
        Self { window_id: 0, resize_mode }
    }

    fn update(&mut self, window_id: u64) {
        self.window_id = window_id;
    }
}

impl<App: Application> ClickHandler<App> for ResizeClickHandler {
    fn handle_click(&self, _: &State<App>, queue: &mut EventQueue<App>) {
        let Self { window_id, resize_mode } = *self;

        crate::log_trace!("resize click: window={} mode={}", window_id, resize_mode as u8);

        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::ResizingWindow { resize_mode, window_id },
        });
    }
}

#[derive(Default)]
struct MoveClickHandler {
    window_id: u64,
}

impl MoveClickHandler {
    fn update(&mut self, window_id: u64) {
        self.window_id = window_id;
    }
}

impl<App: Application> ClickHandler<App> for MoveClickHandler {
    fn handle_click(&self, _: &State<App>, queue: &mut EventQueue<App>) {
        queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::MovingWindow { window_id: self.window_id },
        });
    }
}

#[derive(Default)]
struct CloseClickHandler {
    window_id: u64,
}

impl CloseClickHandler {
    fn update(&mut self, window_id: u64) {
        self.window_id = window_id;
    }
}

impl<App: Application> ClickHandler<App> for CloseClickHandler {
    fn handle_click(&self, _: &State<App>, queue: &mut EventQueue<App>) {
        queue.queue(Event::CloseWindow { window_id: self.window_id });
    }
}

pub struct WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Elements: Element<App>,
{
    title_marker: PhantomData<Title>,
    title: A,
    title_color: B,
    hovered_title_color: C,
    background_color: D,
    highlight_color: E,
    shadow_color: F,
    shadow_padding: G,
    title_height: H,
    title_gap: I,
    font_size: J,
    gaps: K,
    border: L,
    corner_diameter: M,
    closable: N,
    resizable: O,
    close_button_size: P,
    close_button_corner_diameter: Q,
    minimum_width: R,
    maximum_width: S,
    minimum_height: T,
    maximum_height: U,
    theme: App::ThemeType,
    class: Option<App::WindowClass>,
    /// Whether the window chrome should be drawn using the image skin of the
    /// active theme (if it has one).
    ///
    /// Overlay windows like the chat window have their own (semi transparent)
    /// background and must not draw the opaque skinned window frame, otherwise
    /// the texture would cover their content.
    skinned: bool,
    /// Which vertical edge carries the resize grip.
    vertical_resize_anchor: VerticalResizeAnchor,
    elements: Elements,
    // HACK: This is a bit ugly since all of these store the window_id. Ideally, we would be able
    // to inject some data into the click action instead.
    close_click_action: CloseClickHandler,
    move_click_action: MoveClickHandler,
    horizontal_resize_click_action: ResizeClickHandler,
    vertical_resize_click_action: ResizeClickHandler,
    resize_click_action: ResizeClickHandler,
    layout_info: Option<WindowLayoutInfoSet<<Elements as Element<App>>::LayoutInfo>>,
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
    WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Elements: Element<App>,
{
    #[allow(clippy::too_many_arguments)]
    pub fn component_new(
        title: A,
        title_color: B,
        hovered_title_color: C,
        background_color: D,
        highlight_color: E,
        shadow_color: F,
        shadow_padding: G,
        title_height: H,
        title_gap: I,
        font_size: J,
        gaps: K,
        border: L,
        corner_diameter: M,
        closable: N,
        resizable: O,
        close_button_size: P,
        close_button_corner_diameter: Q,
        minimum_width: R,
        maximum_width: S,
        minimum_height: T,
        maximum_height: U,
        theme: App::ThemeType,
        class: Option<App::WindowClass>,
        skinned: bool,
        vertical_resize_anchor: VerticalResizeAnchor,
        elements: Elements,
    ) -> Self {
        Self {
            title_marker: PhantomData,
            title,
            title_color,
            hovered_title_color,
            background_color,
            highlight_color,
            shadow_color,
            shadow_padding,
            title_height,
            title_gap,
            font_size,
            gaps,
            border,
            corner_diameter,
            closable,
            resizable,
            close_button_size,
            close_button_corner_diameter,
            minimum_width,
            maximum_width,
            minimum_height,
            maximum_height,
            theme,
            class,
            skinned,
            vertical_resize_anchor,
            elements,
            close_click_action: CloseClickHandler::default(),
            move_click_action: MoveClickHandler::default(),
            horizontal_resize_click_action: ResizeClickHandler::new(ResizeMode::Horizontal),
            vertical_resize_click_action: ResizeClickHandler::new(ResizeMode::Vertical),
            resize_click_action: ResizeClickHandler::new(ResizeMode::Both),
            layout_info: None,
        }
    }
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements> private::Sealed
    for WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Elements: Element<App>,
{
}

impl<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements> Window<App>
    for WindowInternal<App, Title, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, Elements>
where
    App: Application,
    Title: AsRef<str>,
    A: Selector<App, Title>,
    B: Selector<App, App::Color>,
    C: Selector<App, App::Color>,
    D: Selector<App, App::Color>,
    E: Selector<App, App::Color>,
    F: Selector<App, App::Color>,
    G: Selector<App, App::ShadowPadding>,
    H: Selector<App, f32>,
    I: Selector<App, f32>,
    J: Selector<App, App::FontSize>,
    K: Selector<App, f32>,
    L: Selector<App, f32>,
    M: Selector<App, App::CornerDiameter>,
    N: Selector<App, bool>,
    O: Selector<App, bool>,
    P: Selector<App, App::Size>,
    Q: Selector<App, App::CornerDiameter>,
    R: Selector<App, f32>,
    S: Selector<App, f32>,
    T: Selector<App, f32>,
    U: Selector<App, f32>,
    Elements: Element<App>,
    <Elements as Element<App>>::LayoutInfo: 'static,
{
    fn get_class(&self) -> Option<App::WindowClass> {
        self.class
    }

    fn get_theme_type(&self) -> <App as Application>::ThemeType {
        self.theme
    }

    fn is_closable(&self, state: &State<App>) -> bool {
        *state.get(&self.closable)
    }

    #[allow(private_interfaces)]
    fn create_layout_info(
        &mut self,
        state: &State<App>,
        store: &mut WindowStore,
        data: &mut WindowData<App>,
        generator: &mut ElementIdGenerator,
        text_layouter: &App::TextLayouter,
        window_size: App::Size,
    ) -> DisplayInformation {
        self.close_click_action.update(data.id);
        self.move_click_action.update(data.id);
        self.horizontal_resize_click_action.update(data.id);
        self.vertical_resize_click_action.update(data.id);
        self.vertical_resize_click_action.resize_mode = match self.vertical_resize_anchor {
            VerticalResizeAnchor::Bottom => ResizeMode::Vertical,
            VerticalResizeAnchor::Top => ResizeMode::VerticalTop,
        };
        self.resize_click_action.update(data.id);

        let store = store.get_or_create_from_window_id(data.id, generator);

        App::set_current_theme_type(self.theme);

        let title_height = *state.get(&self.title_height);
        let title_gap = *state.get(&self.title_gap);

        let minimum_width = *state.get(&self.minimum_width);
        let maximum_width = *state.get(&self.maximum_width);
        let minimum_height = *state.get(&self.minimum_height);
        let maximum_height = *state.get(&self.maximum_height);

        // While the window chrome is shown (interface settings open) all windows can be
        // resized, including windows that normally have a fixed size. In that mode the
        // stored size is not clamped to the configured minimum/maximum, but it is still
        // clamped to a sane range: the window cache uses `f32::MAX` as "fit to content"
        // sentinel and a stored size may also be stale/not finite. Without this a
        // window that was never resized ends up with a height of `f32::MAX` as
        // soon as the chrome becomes visible, which overflows the
        // layout/rendering math (a crash with real graphics drivers).
        let chrome_visible = App::window_chrome_visible();

        let stored_width = finite_or_zero(data.size.width());
        let stored_height = finite_or_zero(data.size.height());

        let maximum_resized_width = window_size.width().max(80.0);
        let maximum_resized_height = window_size.height().max(60.0);

        // A stored width of zero and a stored height of `f32::MAX` are the "fit to
        // content" sentinels used for windows that were never resized. Those windows
        // keep their content based size while the chrome is visible; only windows with
        // a real stored size use the unrestricted resize bounds. Without this a never
        // resized window would get a height of `f32::MAX` while the interface settings
        // are open, which overflows the layout/rendering math (crash on real drivers).
        // Both sentinels are "auto": zero width and `f32::MAX` height. `f32::MAX` is a
        // finite value, so check for it explicitly (and treat any absurdly large value
        // like the sentinel as well).
        let has_auto_width = data.size.width() <= 0.0 || data.size.width() >= f32::MAX;
        let has_auto_height = data.size.height() <= 0.0 || data.size.height() >= f32::MAX;

        let normal_adjusted_size = App::Size::new(
            stored_width.min(maximum_width).max(minimum_width),
            stored_height.min(maximum_height).max(minimum_height),
        );

        let adjusted_size = match chrome_visible && !has_auto_width && !has_auto_height {
            true => App::Size::new(
                stored_width.clamp(80.0, maximum_resized_width),
                stored_height.clamp(60.0, maximum_resized_height),
            ),
            false => normal_adjusted_size,
        };

        if !data.size_clamp_logged && (data.size.width() > maximum_resized_width || data.size.height() > maximum_resized_height) {
            data.size_clamp_logged = true;
            crate::log_trace!(
                "window `{}`: clamping stored size {}x{} to adjusted {}x{} (chrome_visible={}, auto_width={}, auto_height={})",
                data.debug_name,
                data.size.width(),
                data.size.height(),
                adjusted_size.width(),
                adjusted_size.height(),
                chrome_visible,
                has_auto_width,
                has_auto_height,
            );
        }

        if data.anchor.is_initializing() {
            data.anchor.initialize(window_size, adjusted_size);
        }

        // Adjust position
        let real_position = {
            let anchor_position = data.anchor.to_position(window_size);
            let half_width = adjusted_size.width() / 2.0;

            App::Position::new(
                anchor_position.left().max(-half_width).min(window_size.width() - half_width),
                anchor_position.top().max(0.0).min(window_size.height() - title_height),
            )
        };

        let available_area = Area {
            left: real_position.left(),
            top: real_position.top(),
            width: adjusted_size.width(),
            height: adjusted_size.height(),
        };

        let mut resolver = Resolver::new(available_area, 0.0, text_layouter);

        let title_area = resolver.with_height(title_height);

        let (area, children) = resolver.with_derived_borderless(*state.get(&self.gaps), *state.get(&self.border), title_gap, |resolver| {
            self.elements.create_layout_info(state, store, resolver)
        });

        let area = Area {
            left: title_area.left,
            top: title_area.top,
            width: title_area.width,
            height: (title_area.height + area.height).min(maximum_height).max(minimum_height),
        };

        self.layout_info = Some(WindowLayoutInfoSet {
            area,
            title_area,
            children,
        });

        let display_height = area.height;

        DisplayInformation {
            real_area: area,
            display_height,
        }
    }

    #[cfg_attr(feature = "debug", ro_next_debug::profile)]
    #[allow(private_interfaces)]
    fn lay_out<'a>(&'a self, state: &'a State<App>, store: &'a WindowStore, data: &'a WindowData<App>, layout: &mut WindowLayout<'a, App>) {
        let store = store.get_from_window_id(data.id);
        let layout_info = self.layout_info.as_ref().expect("no layout present");

        App::set_current_theme_type(self.theme);

        let skin_path = theme().skin();
        let skin = state.get(&skin_path);

        // Overlay windows (e.g. the chat window) supply their own translucent
        // background colour and must not draw the opaque skinned window frame.
        let skinned = self.skinned;

        if layout_info.area.check().dont_mark().run(layout) {
            layout.set_hovered();
        }

        let close_button = if *state.get(&self.closable) {
            let close_button_size = *state.get(&self.close_button_size);
            let offset = layout_info.title_area.height - close_button_size.height();

            let close_button_area = Area {
                left: layout_info.title_area.left + layout_info.title_area.width - close_button_size.width() - *state.get(&self.border),
                top: layout_info.title_area.top + offset / 2.0,
                width: close_button_size.width(),
                height: layout_info.title_area.height - offset,
            };

            let close_button_hovered = close_button_area.check().run(layout);
            let close_button_color = match close_button_hovered {
                true => {
                    layout.register_click_handler(MouseButton::Left, &self.close_click_action);

                    *state.get(&self.hovered_title_color)
                }
                false => *state.get(&self.title_color),
            };

            Some((close_button_area, close_button_color, close_button_hovered))
        } else {
            None
        };

        let corner_diameter = *state.get(&self.corner_diameter);

        let horizontal_resize_area = Area {
            left: layout_info.area.left + layout_info.area.width - 3.0,
            top: layout_info.area.top + 20.0,
            width: 6.0,
            height: layout_info.area.height - 40.0,
        };
        let vertical_resize_area = match self.vertical_resize_anchor {
            VerticalResizeAnchor::Bottom => Area {
                left: layout_info.area.left + 20.0,
                top: layout_info.area.top + layout_info.area.height - 3.0,
                width: layout_info.area.width - 40.0,
                height: 6.0,
            },
            // The top grip overlaps the bottom edge of the title bar by a couple
            // of pixels so there is no dead/rounded strip between the title and the
            // grip. The hover checks below run *before* the title move check, so on
            // the overlap rows the grip wins and the drag resizes the window from
            // the top instead of moving it.
            VerticalResizeAnchor::Top => Area {
                left: layout_info.area.left + 20.0,
                top: layout_info.area.top + layout_info.title_area.height - 2.0,
                width: layout_info.area.width - 40.0,
                height: 8.0,
            },
        };

        let radius = corner_diameter.bottom_right() / 2.0;
        let corner_offset = radius - (radius.powi(2) / 2.0).sqrt();
        let resize_area = Area {
            left: layout_info.area.left + layout_info.area.width - corner_offset - 6.0,
            top: layout_info.area.top + layout_info.area.height - corner_offset - 6.0,
            width: 12.0,
            height: 12.0,
        };

        // Order matters: a hovered element consumes the hover (`can_be_hovered`),
        // so the resize grips have to be checked before the title bar. Otherwise
        // the move handler is registered for the top grip row and the window is
        // moved instead of resized.
        let horizontal_resize_hovered = horizontal_resize_area.check().run(layout);
        let vertical_resize_hovered = vertical_resize_area.check().run(layout);
        let resize_hovered = resize_area.check().run(layout);

        let is_title_hovered = layout_info.title_area.check().run(layout);

        if is_title_hovered {
            layout.register_click_handler(MouseButton::Left, &self.move_click_action);
        }

        // While the window chrome is shown (the client turns it on while the interface
        // settings window is open) every window can be resized, even ones that do not
        // opt into it via `resizable`.
        let chrome_visible = layout.window_chrome_visible();

        let horizontal_resize_available = *state.get(&self.minimum_width) != *state.get(&self.maximum_width) || chrome_visible;
        let vertical_resize_availabe = *state.get(&self.resizable) || chrome_visible;

        if horizontal_resize_hovered && horizontal_resize_available {
            layout.register_click_handler(MouseButton::Left, &self.horizontal_resize_click_action);
            layout.set_hovered();
        } else if vertical_resize_hovered && vertical_resize_availabe {
            layout.register_click_handler(MouseButton::Left, &self.vertical_resize_click_action);
            layout.set_hovered();
        } else if resize_hovered && horizontal_resize_available && vertical_resize_availabe {
            layout.register_click_handler(MouseButton::Left, &self.resize_click_action);
            layout.set_hovered();
        }

        layout.with_clip(layout_info.area, |layout| {
            layout.with_layer(|layout| {
                self.elements
                    .lay_out(state, ElementStore::new(store, data.id), &layout_info.children, layout);
            });
        });

        let title_color = match is_title_hovered {
            true => *state.get(&self.hovered_title_color),
            false => *state.get(&self.title_color),
        };

        // The window background is either a plain rectangle (default theme) or a nine
        // slice image (skins like the classic Ragnarok Online interface).
        match skin.window.frame.as_ref().filter(|_| skinned) {
            Some(frame) => layout.add_image(layout_info.area, frame.image, frame.color, frame.slices, frame.smooth),
            None => {
                layout.add_rectangle(
                    layout_info.area,
                    *state.get(&self.corner_diameter),
                    *state.get(&self.background_color),
                    *state.get(&self.shadow_color),
                    *state.get(&self.shadow_padding),
                );
            }
        }

        if let Some(title_bar) = skin.window.title_bar.as_ref().filter(|_| skinned) {
            layout.add_image(
                layout_info.title_area,
                title_bar.image,
                title_bar.color,
                title_bar.slices,
                title_bar.smooth,
            );
        }

        layout.add_text(
            layout_info.title_area,
            state.get(&self.title).as_ref(),
            *state.get(&self.font_size),
            title_color,
            *state.get(&self.highlight_color),
            // TODO: Make this configurable in the window.
            *state.get(&theme().window().horizontal_alignment()),
            *state.get(&theme().window().vertical_alignment()),
            *state.get(&theme().window().overflow_behavior()),
        );

        let horizontal_resize_active = horizontal_resize_hovered && horizontal_resize_available
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Horizontal,
                window_id,
            } if *window_id == data.id);
        let vertical_resize_active = vertical_resize_hovered && vertical_resize_availabe
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Vertical | ResizeMode::VerticalTop,
                window_id,
            } if *window_id == data.id);
        let corner_resize_active = resize_hovered && horizontal_resize_available && vertical_resize_availabe
            || matches!(layout.get_mouse_mode(), MouseMode::ResizingWindow {
                resize_mode: ResizeMode::Both,
                window_id,
            } if *window_id == data.id);

        // While the resize chrome is visible all available handles are drawn
        // simultaneously (horizontal, vertical and the corner). They used to be
        // mutually exclusive via an else-if chain, which meant that any window
        // with a horizontal handle (like the chat window) never showed the
        // vertical handle, so it could only be resized side to side.
        if horizontal_resize_active || (chrome_visible && horizontal_resize_available) {
            let color = match horizontal_resize_active {
                true => *state.get(&theme().window().closest_anchor_color()),
                false => *state.get(&theme().window().anchor_color()),
            };
            layout.add_rectangle(
                horizontal_resize_area,
                App::CornerDiameter::new(6.0, 6.0, 6.0, 6.0),
                color,
                color,
                App::ShadowPadding::none(),
            );
        }

        if vertical_resize_active || (chrome_visible && vertical_resize_availabe) {
            let color = match vertical_resize_active {
                true => *state.get(&theme().window().closest_anchor_color()),
                false => *state.get(&theme().window().anchor_color()),
            };
            layout.add_rectangle(
                vertical_resize_area,
                App::CornerDiameter::new(6.0, 6.0, 6.0, 6.0),
                color,
                color,
                App::ShadowPadding::none(),
            );
        }

        if corner_resize_active || (chrome_visible && horizontal_resize_available && vertical_resize_availabe) {
            let color = match corner_resize_active {
                true => *state.get(&theme().window().closest_anchor_color()),
                false => *state.get(&theme().window().anchor_color()),
            };
            layout.add_rectangle(
                resize_area,
                App::CornerDiameter::new(12.0, 12.0, 12.0, 12.0),
                color,
                color,
                App::ShadowPadding::none(),
            );
        }

        if chrome_visible {
            // A small grip in the title bar, so it is obvious that the window can be
            // dragged around while the interface settings are open.
            let grip_width = 14.0;
            let grip_height = 1.5;
            let grip_gap = 3.0;
            let title_right = layout_info.title_area.left + layout_info.title_area.width - *state.get(&self.border);
            let grip_left = match close_button {
                Some(_) => title_right - state.get(&self.close_button_size).width() - 12.0 - grip_width,
                None => title_right - grip_width,
            };
            let grip_top = layout_info.title_area.top + (layout_info.title_area.height - (grip_height * 3.0 + grip_gap * 2.0)) / 2.0;
            let grip_color = match is_title_hovered {
                true => *state.get(&self.hovered_title_color),
                false => *state.get(&self.title_color),
            };

            for index in 0..3 {
                layout.add_rectangle(
                    Area {
                        left: grip_left,
                        top: grip_top + index as f32 * (grip_height + grip_gap),
                        width: grip_width,
                        height: grip_height,
                    },
                    App::CornerDiameter::new(0.0, 0.0, 0.0, 0.0),
                    grip_color,
                    grip_color,
                    App::ShadowPadding::none(),
                );
            }
        }

        if let Some((close_button_area, close_button_color, _is_close_button_hovered)) = close_button {
            // The window close button is always drawn as an "X" glyph. Some skins provide
            // a close button texture (e.g. the CondaRO classic interface uses a bitmap that
            // literally reads "close"), which is not what the title bar close button should
            // look like; the "Close" label is reserved for dialog buttons at the bottom.
            layout.add_rectangle(
                close_button_area,
                *state.get(&self.close_button_corner_diameter),
                close_button_color,
                close_button_color,
                App::ShadowPadding::none(),
            );

            layout.add_text(
                close_button_area,
                "X",
                *state.get(&self.font_size),
                *state.get(&self.background_color),
                *state.get(&self.highlight_color),
                HorizontalAlignment::Center { offset: 0.0, border: 0.0 },
                VerticalAlignment::Center { offset: 0.0 },
                *state.get(&theme().window().overflow_behavior()),
            );
        }
    }
}
