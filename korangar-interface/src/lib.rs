// The `Element` trait is implemented for tuples up to 140 fields (the
// localization struct alone has more than 100 fields), which needs deeper macro
// recursion.
#![recursion_limit = "512"]
#![allow(incomplete_features)]
#![feature(auto_traits)]
#![feature(negative_impls)]
#![feature(macro_metavar_expr)]
#![feature(impl_trait_in_bindings)]
#![feature(unsafe_cell_access)]
#![feature(associated_type_defaults)]
#![feature(impl_trait_in_assoc_type)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(allocator_api)]

pub mod application;
pub mod components;
pub mod element;
pub mod event;
pub mod layout;
pub mod logging;
pub mod skin;
pub mod theme;
pub mod window;

// Re-export self as korangar_interface so we can use proc macros in this crate.
extern crate self as korangar_interface;

use std::any::Any;
use std::collections::BTreeMap;

use application::{Application, Clip, CornerDiameter, FontSize, Position, RenderLayer, Size, TextLayouter, WindowCache};
use element::ElementBox;
use element::id::{ElementId, ElementIdGenerator};
use element::store::{ElementStore, ElementStoreMut, InternalElementStore};
use event::{Event, EventQueue};
use layout::area::Area;
use layout::tooltip::TooltipTheme;
use layout::{MouseButton, ResizeMode, Resolver, WindowLayout};
use option_ext::OptionExt;
use rust_state::State;
use theme::ThemePathGetter;
use window::store::WindowStore;
use window::{Anchor, CustomWindow, DisplayInformation, StateWindow, Window, WindowData, WindowThemePathExt};

use crate::application::ShadowPadding;
use crate::element::id::FocusIdExt;

pub mod prelude {
    //! Prelude for implementing elements or windows. Mainly to reduce the
    //! number of `*PathExt` imports.

    // Re-export proc macros.
    pub use interface_component_macros::create_component_macro;
    pub use interface_components::*;

    pub use crate::components::button::ButtonThemePathExt;
    pub use crate::components::collapsible::CollapsibleThemePathExt;
    pub use crate::components::drop_down::DropDownThemePathExt;
    pub use crate::components::either::Either;
    pub use crate::components::field::FieldThemePathExt;
    pub use crate::components::state_button::StateButtonThemePathExt;
    pub use crate::components::tabs::Tabs;
    pub use crate::components::text::TextThemePathExt;
    pub use crate::components::text_box::TextBoxThemePathExt;
    pub use crate::element::{EmptyElement, ErasedElement};
    pub use crate::event::{Event, EventQueue, SetToFalse, SetToTrue, Toggle};
    pub use crate::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    pub use crate::layout::tooltip::TooltipThemePathExt;
    pub use crate::selector_helpers::*;
    pub use crate::theme::{ThemePathGetter, theme};
    pub use crate::window::WindowThemePathExt;
}

// TODO: Move
pub mod selector_helpers {
    use std::cell::{Cell, UnsafeCell};
    use std::fmt::Display;

    use rust_state::{Path, PathExt, Selector};

    use crate::application::Application;
    use crate::element::ElementDisplay;

    pub struct PartialEqDisplaySelector<P, T> {
        path: P,
        last_value: Cell<Option<T>>,
        text: UnsafeCell<String>,
    }

    impl<P, T> PartialEqDisplaySelector<P, T> {
        pub fn new(path: P) -> Self {
            Self {
                path,
                last_value: Cell::default(),
                text: UnsafeCell::default(),
            }
        }
    }

    impl<App, P, T> Selector<App, String> for PartialEqDisplaySelector<P, T>
    where
        App: Application,
        P: Path<App, T>,
        T: Copy + PartialEq + Display + 'static,
    {
        fn select<'a>(&'a self, state: &'a App) -> Option<&'a String> {
            let value = self.path.follow_safe(state);

            let last_value = self.last_value.get();

            if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                unsafe { *self.text.get() = value.to_string() };
                self.last_value.set(Some(*value));
            }

            unsafe { Some(self.text.as_ref_unchecked()) }
        }
    }

    pub struct ElementDisplaySelector<P, T> {
        path: P,
        last_value: UnsafeCell<Option<T>>,
        text: UnsafeCell<String>,
    }

    impl<P, T> ElementDisplaySelector<P, T> {
        pub fn new(path: P) -> Self {
            Self {
                path,
                last_value: UnsafeCell::default(),
                text: UnsafeCell::default(),
            }
        }
    }

    impl<App, P, T> Selector<App, String> for ElementDisplaySelector<P, T>
    where
        App: Application,
        P: Path<App, T>,
        T: ElementDisplay,
    {
        fn select<'a>(&'a self, state: &'a App) -> Option<&'a String> {
            let value = self.path.follow_safe(state);

            unsafe {
                let last_value = &mut *self.last_value.get();

                if last_value.is_none() || last_value.as_ref().is_some_and(|last| last != value) {
                    *self.text.get() = value.element_display();
                    *last_value = Some(value.clone());
                }
            }

            unsafe { Some(self.text.as_ref_unchecked()) }
        }
    }

    pub struct ComputedSelector<F, T> {
        closure: F,
        latest_value: UnsafeCell<T>,
    }

    impl<F, T> ComputedSelector<F, T>
    where
        T: Default,
    {
        pub fn new_default(closure: F) -> Self {
            Self {
                closure,
                latest_value: UnsafeCell::new(T::default()),
            }
        }
    }

    impl<App, F, T> Selector<App, T> for ComputedSelector<F, T>
    where
        F: Fn(&App) -> T + 'static,
        T: 'static,
    {
        fn select<'a>(&'a self, state: &'a App) -> Option<&'a T> {
            let new_value = (self.closure)(state);

            let latest_value = unsafe { &mut *self.latest_value.get() };
            *latest_value = new_value;

            Some(latest_value)
        }
    }
}

// TODO: This will likely be renamed + Moved
pub enum MouseMode<App>
where
    App: Application,
{
    Default,
    MovingWindow { window_id: u64 },
    ResizingWindow { resize_mode: ResizeMode, window_id: u64 },
    Custom { mode: App::CustomMouseMode },
}

impl<App> MouseMode<App>
where
    App: Application,
{
    pub fn is_default(&self) -> bool {
        matches!(self, MouseMode::Default)
    }
}

impl<App> Clone for MouseMode<App>
where
    App: Application,
    App::CustomMouseMode: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Default => Self::Default,
            Self::MovingWindow { window_id } => Self::MovingWindow { window_id: *window_id },
            Self::ResizingWindow { resize_mode, window_id } => Self::ResizingWindow {
                resize_mode: *resize_mode,
                window_id: *window_id,
            },
            Self::Custom { mode } => Self::Custom { mode: mode.clone() },
        }
    }
}

struct WindowWrapper<App>
where
    App: Application,
{
    window: Box<dyn Window<App>>,
    data: WindowData<App>,
    display_information: DisplayInformation,
    /// Layout cache: the inputs the cached `display_information` /
    /// `create_layout_info` result was computed for.
    layout_cache: WindowLayoutCache<App>,
}

/// Inputs that `Window::create_layout_info` depends on. When all of them are
/// unchanged the previously computed layout info is reused instead of resolving
/// every element again (see [`Interface::invalidate_layouts`]).
struct WindowLayoutCache<App>
where
    App: Application,
{
    valid: bool,
    generation: u64,
    scaling_bits: u32,
    window_width: f32,
    window_height: f32,
    theme_type: App::ThemeType,
    chrome_visible: bool,
}

impl<App> WindowLayoutCache<App>
where
    App: Application,
{
    fn new(theme_type: App::ThemeType) -> Self {
        Self {
            valid: false,
            generation: 0,
            scaling_bits: 0,
            window_width: 0.0,
            window_height: 0.0,
            theme_type,
            chrome_visible: false,
        }
    }
}

struct OverlayElement<App>
where
    App: Application,
{
    element: ElementBox<App>,
    store: InternalElementStore,
    position: App::Position,
    size: App::Size,
    window_id: u64,
}

/// Window sizes use `f32::MAX` as a "fit to content" sentinel; keep layout math
/// finite by replacing non finite values with `0.0`.
fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

/// Shorten a `type_name` result for log lines.
///
/// The window types are generic over the full theme element paths, so their
/// full type names are enormous. Log lines only need the outer type name
/// (e.g. `LoginWindow` instead of
/// `korangar::interface::windows::login::LoginWindow<...all paths...>`).
fn short_type_name(name: &'static str) -> &'static str {
    let without_generics = match name.find('<') {
        Some(index) => &name[..index],
        None => name,
    };

    match without_generics.rfind("::") {
        Some(index) => &without_generics[index + 2..],
        None => without_generics,
    }
}

/// Profiling counters of the window layout pass, used by the client's
/// `KORANGAR_PERF_LOG` mode to measure the effect of layout caching.
static LAYOUT_PROFILE_FRAMES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAYOUT_PROFILE_WINDOWS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAYOUT_PROFILE_CREATE_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAYOUT_PROFILE_REUSED: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAYOUT_PROFILE_LAYOUT_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Snapshot of the window layout profiling counters.
#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutProfile {
    /// Number of `lay_out_windows` calls (frames).
    pub frames: u64,
    /// Sum of open windows over all frames.
    pub windows: u64,
    /// Total time spent in `Window::create_layout_info`.
    pub create_layout_nanos: u64,
    /// Total time spent in `Window::lay_out`.
    pub lay_out_nanos: u64,
    /// Number of windows whose cached layout info was reused instead of being
    /// recomputed.
    pub reused_windows: u64,
}

/// Take and reset the window layout profiling counters.
pub fn take_layout_profile() -> LayoutProfile {
    use std::sync::atomic::Ordering::Relaxed;

    LayoutProfile {
        frames: LAYOUT_PROFILE_FRAMES.swap(0, Relaxed),
        windows: LAYOUT_PROFILE_WINDOWS.swap(0, Relaxed),
        create_layout_nanos: LAYOUT_PROFILE_CREATE_NANOS.swap(0, Relaxed),
        lay_out_nanos: LAYOUT_PROFILE_LAYOUT_NANOS.swap(0, Relaxed),
        reused_windows: LAYOUT_PROFILE_REUSED.swap(0, Relaxed),
    }
}

pub struct Interface<'a, App>
where
    App: Application,
{
    windows: Vec<WindowWrapper<App>>,
    window_cache: App::Cache,
    window_size: App::Size,

    generator: ElementIdGenerator,
    window_store: WindowStore,
    focused_element: Option<ElementId>,
    mouse_mode: MouseMode<App>,
    /// Show the window chrome (move grip and resize handles) on all windows
    /// instead of only while hovering them. Used by the client while the
    /// interface settings window is open so windows can be arranged.
    window_chrome_visible: bool,
    event_queue: EventQueue<App>,
    overlay_element: Option<OverlayElement<App>>,

    /// Incremented whenever something that can affect the element layout
    /// changed (window data, scaling, window size, theme, chrome visibility,
    /// focus, or any user/network event that may mutate the UI state). Windows
    /// reuse their previously computed layout info while this value and the
    /// window's own layout inputs are unchanged.
    layout_generation: u64,

    /// Cached window layouts. This is mostly an optimization to avoid
    /// re-allocating the every frame but also serves to store some information
    /// between frames like the tooltip timers.
    window_layouts: BTreeMap<u64, WindowLayout<'a, App>>,
    /// Cached overlay layout. This is mostly an optimization to avoid
    /// re-allocating the every frame but also serves to store some information
    /// between frames like the tooltip timers.
    overlay_layout: Option<WindowLayout<'a, App>>,

    next_window_id: u64,

    text_layouter: App::TextLayouter,
}

impl<App> Interface<'static, App>
where
    App: Application,
{
    pub fn new(text_layouter: App::TextLayouter, available_space: App::Size) -> Self {
        let window_cache = App::Cache::create();

        Self {
            windows: Vec::new(),
            window_cache,
            window_size: available_space,

            generator: ElementIdGenerator::new(),
            window_store: WindowStore::default(),
            focused_element: None,
            mouse_mode: MouseMode::Default,
            window_chrome_visible: false,
            layout_generation: 0,
            event_queue: EventQueue::default(),
            overlay_element: None,

            window_layouts: BTreeMap::new(),
            overlay_layout: None,

            next_window_id: 0,
            text_layouter,
        }
    }

    pub fn update_window_size(&mut self, screen_size: App::Size) {
        self.window_size = screen_size;
        self.invalidate_layouts();
    }

    /// Mark the cached window layouts as stale.
    ///
    /// The client calls this whenever UI state may have changed without the
    /// interface noticing by itself (network events that update inventory,
    /// chat, stats, ...; settings changes; ...). Window
    /// open/close/move/resize, screen size and chrome visibility changes
    /// invalidate the layouts internally.
    ///
    /// Element layouts are only recomputed for windows whose cached layout
    /// inputs (generation, scaling, screen size, theme, chrome visibility) no
    /// longer match, so this is cheap and can be called every frame if in
    /// doubt.
    pub fn invalidate_layouts(&mut self) {
        self.layout_generation = self.layout_generation.wrapping_add(1);
        crate::log_trace!("interface: invalidate_layouts generation={}", self.layout_generation);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn handle_drag(&mut self, delta: App::Size, interface_scaling: f32) {
        self.invalidate_layouts();

        match self.mouse_mode {
            MouseMode::Default => {}
            MouseMode::MovingWindow { window_id } => {
                if let Some(wrapper) = self.windows.iter_mut().find(|window| window.data.id == window_id) {
                    let new_position = App::Position::new(
                        wrapper.display_information.real_area.left + delta.width(),
                        wrapper.display_information.real_area.top + delta.height(),
                    );

                    let scaled_size = App::Size::new(
                        wrapper.display_information.real_area.width * interface_scaling,
                        wrapper.display_information.display_height * interface_scaling,
                    );

                    wrapper.data.anchor.update(self.window_size, new_position, scaled_size);

                    if let Some(window_class) = wrapper.window.get_class() {
                        self.window_cache.update_anchor(window_class, wrapper.data.anchor);
                    }
                }
            }
            MouseMode::ResizingWindow { window_id, resize_mode } => {
                if let Some(wrapper) = self.windows.iter_mut().find(|window| window.data.id == window_id) {
                    let (delta_width, delta_height) = match resize_mode {
                        ResizeMode::Horizontal => (delta.width(), 0.0),
                        ResizeMode::Vertical => (0.0, delta.height()),
                        // Dragging the top edge: moving the mouse up grows the window, so
                        // the height changes opposite to the mouse delta. The anchor is
                        // moved by the same amount below to keep the bottom edge fixed.
                        ResizeMode::VerticalTop => (0.0, -delta.height()),
                        ResizeMode::Both => (delta.width(), delta.height()),
                    };

                    if resize_mode == ResizeMode::VerticalTop {
                        let new_position = App::Position::new(
                            wrapper.display_information.real_area.left,
                            wrapper.display_information.real_area.top + delta.height(),
                        );
                        let scaled_size = App::Size::new(
                            wrapper.display_information.real_area.width * interface_scaling,
                            wrapper.display_information.display_height * interface_scaling,
                        );

                        wrapper.data.anchor.update(self.window_size, new_position, scaled_size);
                    }

                    let new_width = wrapper.display_information.real_area.width + delta_width / interface_scaling;
                    let new_height = wrapper.display_information.real_area.height + delta_height / interface_scaling;

                    // Clamp to a sane range so a fast drag can never create a
                    // zero/negative/non finite window size.
                    wrapper.data.size = App::Size::new(
                        finite_or_zero(new_width).clamp(80.0, 4096.0),
                        finite_or_zero(new_height).clamp(60.0, 4096.0),
                    );

                    if let Some(window_class) = wrapper.window.get_class() {
                        self.window_cache.update_size(window_class, wrapper.data.size);
                    }
                }
            }
            MouseMode::Custom { .. } => {}
        }
    }

    /// Queue a focus request for the element with the given focus id. The
    /// request is resolved to an element id while the window layouts are
    /// available.
    pub fn focus_element(&mut self, focus_id: impl Any) {
        self.event_queue.queue(Event::FocusElement {
            focus_id: focus_id.focus_id(),
        });
    }

    pub fn is_window_with_class_open(&self, window_class: App::WindowClass) -> bool {
        self.windows
            .iter()
            .any(|wrapper| wrapper.window.get_class().is_some_and(|class| class == window_class))
    }

    fn open_new_window(&mut self, window: impl Window<App> + 'static, debug_name: &'static str) {
        let id = self.next_window_id;
        // TODO: Actual logic to wrap around and adjust all window ids.
        self.next_window_id = self.next_window_id.wrapping_add(1);

        let window_class = window.get_class();
        let theme_type = window.get_theme_type();

        crate::log_trace!(
            "interface: creating window layout for `{}` (id={}, class={:?})",
            debug_name,
            id,
            window_class.is_some(),
        );

        let (anchor, size) = match window_class.and_then(|window_class| self.window_cache.get_window_state(window_class)) {
            Some(saved_state) => saved_state,
            None => {
                let anchor = Anchor::default();
                let size = App::Size::new(0.0, f32::MAX);

                if let Some(window_class) = window_class {
                    self.window_cache.register_window(window_class, anchor, size);
                }

                (anchor, size)
            }
        };

        crate::log_trace!(
            "interface: push window id={} type=`{}` anchor_initialized={}",
            id,
            debug_name,
            size.width() == 0.0
        );
        self.layout_generation = self.layout_generation.wrapping_add(1);

        self.windows.push(WindowWrapper {
            window: Box::new(window),
            data: WindowData {
                id,
                anchor,
                size,
                debug_name,
                size_clamp_logged: false,
            },
            layout_cache: WindowLayoutCache::new(theme_type),
            display_information: DisplayInformation {
                real_area: Area {
                    left: 0.0,
                    top: 0.0,
                    width: size.width(),
                    height: size.height(),
                },
                display_height: 0.0,
            },
        });
    }

    pub fn get_mouse_mode(&self) -> &MouseMode<App> {
        &self.mouse_mode
    }

    pub fn set_mouse_mode(&mut self, mouse_mode: MouseMode<App>) {
        self.mouse_mode = mouse_mode;
    }

    /// The currently active mouse mode.
    pub fn mouse_mode(&self) -> &MouseMode<App> {
        &self.mouse_mode
    }

    /// Show the window chrome (move grip and resize handles) on all windows,
    /// even when they are not hovered.
    pub fn set_window_chrome_visible(&mut self, window_chrome_visible: bool) {
        if self.window_chrome_visible != window_chrome_visible {
            self.window_chrome_visible = window_chrome_visible;
            App::set_window_chrome_visible(window_chrome_visible);
            self.invalidate_layouts();
        }
    }

    pub fn has_focus(&self) -> bool {
        self.focused_element.is_some()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_window<T>(&mut self, window: T)
    where
        T: CustomWindow<App> + 'static,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            crate::log_trace!("interface: open_window type=`{}`", short_type_name(std::any::type_name::<T>()));
            let window = window.to_window();
            self.open_new_window(window, short_type_name(std::any::type_name::<T>()));
        } else {
            crate::log_debug!(
                "interface: open_window type=`{}` skipped, class already open",
                short_type_name(std::any::type_name::<T>())
            );
        }
    }

    /// Open a window with the top left corner at the given screen position.
    ///
    /// This is meant for popups (like a context menu) that should appear at the
    /// mouse cursor instead of at a cached window position. The position is not
    /// stored in the window cache, every call re-places the window.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_window_at<T>(&mut self, window: T, position: App::Position)
    where
        T: CustomWindow<App> + 'static,
    {
        crate::log_trace!(
            "interface: open_window_at type=`{}`",
            short_type_name(std::any::type_name::<T>())
        );
        let window = window.to_window();
        self.open_new_window(window, short_type_name(std::any::type_name::<T>()));

        if let Some(wrapper) = self.windows.last_mut() {
            wrapper.data.anchor = Anchor::top_left(position);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_state_window<T>(&mut self, window_path: impl rust_state::Path<App, T>)
    where
        T: StateWindow<App>,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            crate::log_trace!(
                "interface: open_state_window type=`{}`",
                short_type_name(std::any::type_name::<T>())
            );
            let window = T::to_window(window_path);
            self.open_new_window(window, short_type_name(std::any::type_name::<T>()));
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn open_state_window_mut<T>(&mut self, window_path: impl rust_state::Path<App, T>)
    where
        T: StateWindow<App>,
    {
        if !T::window_class().is_some_and(|window_class| self.is_window_with_class_open(window_class)) {
            let window = T::to_window_mut(window_path);
            self.open_new_window(window, short_type_name(std::any::type_name::<T>()));
        }
    }

    fn remove_window(&mut self, index: usize) {
        // Remove the cached window layout to avoid growing the cache indefinitely.
        self.window_layouts.remove(&self.windows[index].data.id);
        self.windows.remove(index);
        self.layout_generation = self.layout_generation.wrapping_add(1);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_top_window(&mut self, state: &State<App>) {
        if let Some(index_from_back) = self.windows.iter().rev().position(|wrapper| wrapper.window.is_closable(state)) {
            let index = self.windows.len() - 1 - index_from_back;
            self.remove_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_window_with_class(&mut self, window_class: App::WindowClass) {
        if let Some(index_from_back) = self
            .windows
            .iter()
            .rev()
            .map(|wrapper| wrapper.window.get_class())
            .position(|class_option| class_option.contains(&window_class))
        {
            let index = self.windows.len() - 1 - index_from_back;
            crate::log_trace!(
                "interface: close_window_with_class id={} type=`{}`",
                self.windows[index].data.id,
                self.windows[index].data.debug_name
            );
            self.remove_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows(&mut self) {
        crate::log_trace!("interface: close_all_windows windows={}", self.windows.len());
        for index in (0..self.windows.len()).rev() {
            self.remove_window(index);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn close_all_windows_except(&mut self, exceptions: &[App::WindowClass]) {
        for index in (0..self.windows.len()).rev() {
            if self.windows[index]
                .window
                .get_class()
                .map(|class| !exceptions.contains(&class))
                .unwrap_or(true)
            {
                self.remove_window(index);
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn process_events(&mut self, custom_events: &mut Vec<App::CustomEvent>) {
        let events: Vec<_> = self.event_queue.drain().collect();

        if !events.is_empty() {
            self.invalidate_layouts();
        }

        for event in events {
            match event {
                // This case should never be hit. FocusElement needs to be converted to
                // FocusElementPost in the event queue while the layout is still alive.
                Event::FocusElement { .. } => {}
                Event::FocusElementPost { element_id } => self.focused_element = Some(element_id),
                Event::Unfocus => self.focused_element = None,
                Event::SetMouseMode { mouse_mode } => self.mouse_mode = mouse_mode,
                Event::Application { custom_event } => custom_events.push(custom_event),
                Event::OpenOverlay {
                    element,
                    position,
                    size,
                    window_id,
                } => {
                    self.overlay_element = Some(OverlayElement {
                        element,
                        store: InternalElementStore::root(&mut self.generator),
                        position,
                        size,
                        window_id,
                    });
                }
                Event::MoveWindowToTop { window_id } => {
                    if let Some(window_index) = self.windows.iter().position(|wrapper| wrapper.data.id == window_id) {
                        let window = self.windows.remove(window_index);
                        self.windows.push(window);
                    }
                }
                Event::CloseWindow { window_id } => {
                    if let Some(index) = self.windows.iter().position(|wrapper| wrapper.data.id == window_id) {
                        self.windows.remove(index);
                    }
                }
                Event::CloseOverlay => {
                    self.overlay_element = None;
                    self.overlay_layout = None;
                }
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn lay_out_windows<'a>(
        &'a mut self,
        state: &'a State<App>,
        interface_scaling: f32,
        mouse_position: App::Position,
    ) -> InterfaceFrame<'a, App> {
        if let Some(overlay_element) = &mut self.overlay_element {
            match self.windows.iter().find(|wrapper| wrapper.data.id == overlay_element.window_id) {
                Some(wrapper) => {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("create overlay element layout info");

                    let available_area = Area {
                        left: overlay_element.position.left(),
                        top: overlay_element.position.top(),
                        width: overlay_element.size.width(),
                        height: overlay_element.size.height(),
                    };

                    let store = ElementStoreMut::new(&mut overlay_element.store, &mut self.generator, overlay_element.window_id);
                    let mut resolver = Resolver::new(available_area, 0.0, &self.text_layouter);

                    App::set_current_theme_type(wrapper.window.get_theme_type());

                    overlay_element.element.create_layout_info(state, store, &mut resolver);
                }
                // Window was closed while the overlay was open, so we just close it.
                None => self.overlay_element = None,
            }
        }

        // SAFETY:
        //
        // This is safe as long as the `Drop` implementation of the `InterfaceFrame`
        // clears the `Layout`s, removing any references with the lifetime 'a
        // from the struct. If drop does not clear the layout or the clear
        // implementation of drop is incorrect we will end up with dangling
        // references.
        let this = unsafe { std::mem::transmute::<&'a mut Interface<'static, App>, &'a mut Interface<'a, App>>(self) };

        this.windows.iter_mut().for_each(|wrapper| {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("create window layout info");

            let theme_type = wrapper.window.get_theme_type();
            let scaling_bits = interface_scaling.to_bits();
            let window_width = this.window_size.width();
            let window_height = this.window_size.height();

            let layout_cache = &mut wrapper.layout_cache;
            let reusable = layout_cache.valid
                && layout_cache.generation == this.layout_generation
                && layout_cache.scaling_bits == scaling_bits
                && layout_cache.window_width.to_bits() == window_width.to_bits()
                && layout_cache.window_height.to_bits() == window_height.to_bits()
                && layout_cache.theme_type == theme_type
                && layout_cache.chrome_visible == this.window_chrome_visible;

            if reusable {
                LAYOUT_PROFILE_REUSED.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }

            crate::log_trace!(
                "interface: create_layout_info id={} type=`{}` (layout inputs changed)",
                wrapper.data.id,
                wrapper.data.debug_name,
            );

            let layout_start = std::time::Instant::now();
            wrapper.display_information = wrapper.window.create_layout_info(
                state,
                &mut this.window_store,
                &mut wrapper.data,
                &mut this.generator,
                &this.text_layouter,
                this.window_size,
            );

            LAYOUT_PROFILE_CREATE_NANOS.fetch_add(layout_start.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);

            layout_cache.valid = true;
            layout_cache.generation = this.layout_generation;
            layout_cache.scaling_bits = scaling_bits;
            layout_cache.window_width = window_width;
            layout_cache.window_height = window_height;
            layout_cache.theme_type = theme_type;
            layout_cache.chrome_visible = this.window_chrome_visible;
        });

        LAYOUT_PROFILE_FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        LAYOUT_PROFILE_WINDOWS.fetch_add(this.windows.len() as u64, std::sync::atomic::Ordering::Relaxed);

        let mut hovered_window = None;

        if let Some(overlay_element) = &this.overlay_element {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("lay out overlay element");

            // Window is guaranteed to exist since we check for that when creating the
            // layout info for the overlay.
            let wrapper = this
                .windows
                .iter()
                .find(|wrapper| wrapper.data.id == overlay_element.window_id)
                .unwrap();

            let position = App::Position::new(
                wrapper.display_information.real_area.left,
                wrapper.display_information.real_area.top,
            );

            let layout = this.overlay_layout.get_or_insert_default();
            layout.update(
                interface_scaling,
                position,
                mouse_position,
                this.focused_element,
                true,
                &this.mouse_mode,
                this.window_chrome_visible,
            );

            let store = ElementStore::new(&overlay_element.store, overlay_element.window_id);

            App::set_current_theme_type(wrapper.window.get_theme_type());

            let overlay_area = Area {
                left: overlay_element.position.left(),
                top: overlay_element.position.top(),
                width: overlay_element.size.width(),
                height: overlay_element.size.height(),
            };

            if overlay_area.check().dont_mark().run(layout) {
                hovered_window = Some(overlay_element.window_id);
            }

            overlay_element.element.lay_out(state, store, &(), layout);
        }

        {
            static LAST_SCALING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let scaling_bits = interface_scaling.to_bits();
            let previous_bits = LAST_SCALING.swap(scaling_bits, std::sync::atomic::Ordering::Relaxed);
            if previous_bits != scaling_bits {
                crate::log_debug!(
                    "interface: interface_scaling changed to {} (window_size={}x{}, chrome_visible={}, windows={})",
                    interface_scaling,
                    this.window_size.width(),
                    this.window_size.height(),
                    this.window_chrome_visible,
                    this.windows.len(),
                );
            }
        }

        this.windows.iter().rev().for_each(|wrapper| {
            #[cfg(feature = "debug")]
            korangar_debug::profile_block!("lay out window");

            crate::log_trace!("interface: lay_out id={} type=`{}`", wrapper.data.id, wrapper.data.debug_name);

            let position = App::Position::new(
                wrapper.display_information.real_area.left,
                wrapper.display_information.real_area.top,
            );

            let layout = this.window_layouts.entry(wrapper.data.id).or_default();
            layout.update(
                interface_scaling,
                position,
                mouse_position,
                this.focused_element,
                hovered_window.is_none(),
                &this.mouse_mode,
                this.window_chrome_visible,
            );

            let lay_out_start = std::time::Instant::now();
            wrapper.window.lay_out(state, &this.window_store, &wrapper.data, layout);
            LAYOUT_PROFILE_LAYOUT_NANOS.fetch_add(lay_out_start.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed);

            if hovered_window.is_none() && layout.is_hovered() {
                hovered_window = Some(wrapper.data.id);
            }
        });

        // Publish whether a foreground window is open so the keyboard input
        // system can decide what Escape does, and (when KORANGAR_UI_RECTS_DUMP
        // is set) write the window rectangles for the golden screenshot
        // harness.
        let has_foreground_windows = this
            .windows
            .iter()
            .any(|wrapper| !App::is_background_window_class(wrapper.window.get_class()));
        crate::window::set_has_foreground_windows(has_foreground_windows);

        dump_window_rects(
            &this.windows,
            this.window_size.width(),
            this.window_size.height(),
            interface_scaling,
        );

        if crate::logging::enabled(crate::logging::LEVEL_TRACE) {
            let windows: Vec<(f32, f32, f32, f32)> = this
                .windows
                .iter()
                .map(|wrapper| {
                    (
                        wrapper.display_information.real_area.left,
                        wrapper.display_information.real_area.top,
                        wrapper.display_information.real_area.width,
                        wrapper.display_information.real_area.height,
                    )
                })
                .collect();

            crate::log_trace!(
                "[layout] window_size=({}, {}) scaling={} mouse=({}, {}) windows={:?}",
                this.window_size.width(),
                this.window_size.height(),
                interface_scaling,
                mouse_position.left(),
                mouse_position.top(),
                windows,
            );
        }

        InterfaceFrame {
            windows: &this.windows,
            window_layouts: &mut this.window_layouts,
            overlay_layout: &mut this.overlay_layout,
            event_queue: &mut this.event_queue,
            window_size: this.window_size,
            mouse_mode: &this.mouse_mode,
            hovered_window,
            interface_scaling,
            text_layouter: &this.text_layouter,
        }
    }
}

/// Write the screen rectangles of all open windows to the file named by
/// `KORANGAR_UI_RECTS_DUMP`, if that environment variable is set.
///
/// The golden screenshot harness uses this to crop exactly the window it wants
/// to compare without having to hard-code positions that depend on the theme,
/// the interface scaling and the user's window cache:
///
/// ```text
/// window_size=1280x720 scaling=0.750
/// InventoryWindow 490 180 300 275
/// ```
///
/// The file is written atomically (temp file + rename) so a reader never sees
/// a half-written list.
fn dump_window_rects<App: Application>(windows: &[WindowWrapper<App>], window_size_width: f32, window_size_height: f32, scaling: f32) {
    let Some(path) = std::env::var_os("KORANGAR_UI_RECTS_DUMP") else {
        return;
    };

    use std::fmt::Write;

    let mut output = String::new();
    let _ = writeln!(
        output,
        "window_size={window_size_width}x{window_size_height} scaling={scaling:.3}"
    );
    // Window positions are anchor positions in screen pixels, the window size
    // is tracked in interface pixels and scaled by the interface scaling when
    // it is rendered. The harness works on the rendered screenshot, so convert
    // the size here (and leave the position as it is).
    for wrapper in windows {
        let area = wrapper.display_information.real_area;
        let _ = writeln!(
            output,
            "{} {} {} {} {}",
            wrapper.data.debug_name,
            area.left,
            area.top,
            area.width * scaling,
            area.height * scaling
        );
    }

    let path = std::path::PathBuf::from(path);
    let temporary_path = path.with_extension("tmp");
    if std::fs::write(&temporary_path, output).is_ok() {
        let _ = std::fs::rename(&temporary_path, &path);
    }
}

pub struct InterfaceFrame<'a, App: Application> {
    windows: &'a [WindowWrapper<App>],
    window_layouts: &'a mut BTreeMap<u64, WindowLayout<'a, App>>,
    overlay_layout: &'a mut Option<WindowLayout<'a, App>>,
    event_queue: &'a mut EventQueue<App>,
    mouse_mode: &'a MouseMode<App>,
    window_size: App::Size,
    hovered_window: Option<u64>,
    interface_scaling: f32,
    text_layouter: &'a App::TextLayouter,
}

impl<App: Application> InterfaceFrame<'_, App> {
    pub fn is_interface_hovered(&self) -> bool {
        self.hovered_window.is_some()
    }

    /// Returns whether the window with the given class is the one currently
    /// under the mouse cursor.
    pub fn is_window_with_class_hovered(&self, window_class: App::WindowClass) -> bool {
        self.hovered_window
            .and_then(|window_id| self.windows.iter().find(|wrapper| wrapper.data.id == window_id))
            .and_then(|wrapper| wrapper.window.get_class())
            .is_some_and(|class| class == window_class)
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render user interface"))]
    pub fn render(
        &mut self,
        state: &State<App>,
        renderer: &App::Renderer,
        tooltip_theme: &TooltipTheme<App>,
        mouse_position: App::Position,
    ) {
        // TODO: Don't allocate every frame. Move this to the interface as well.
        let mut tooltips = Vec::new();

        self.windows.iter().for_each(|wrapper| {
            let layout = self.window_layouts.get_mut(&wrapper.data.id).unwrap();
            layout.render(renderer, self.text_layouter);
            layout.update_tooltips(&mut tooltips);
        });

        if let Some(layout) = &mut self.overlay_layout {
            layout.render(renderer, self.text_layouter);
            layout.update_tooltips(&mut tooltips);
        }

        if let MouseMode::MovingWindow { window_id } = self.mouse_mode {
            self.render_window_anchors(state, renderer, *window_id);
        }

        if !tooltips.is_empty() {
            self.render_tooltips(renderer, tooltip_theme, &tooltips, mouse_position);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_window_anchors(&self, state: &State<App>, renderer: &App::Renderer, window_id: u64) {
        if let Some(wrapper) = self.windows.iter().find(|wrapper| wrapper.data.id == window_id) {
            App::set_current_theme_type(wrapper.window.get_theme_type());

            let anchor_color = *state.get(&theme::theme().window().anchor_color());
            let closest_anchor_color = *state.get(&theme::theme().window().closest_anchor_color());
            let shadow_color = *state.get(&theme::theme().window().background_color());
            let shadow_padding = App::ShadowPadding::none();

            wrapper.data.anchor.render_screen_anchors(
                renderer,
                anchor_color,
                closest_anchor_color,
                shadow_color,
                shadow_padding,
                self.window_size,
                self.interface_scaling,
            );

            let window_display_size = App::Size::new(
                wrapper.display_information.real_area.width * self.interface_scaling,
                wrapper.display_information.display_height * self.interface_scaling,
            );

            let position = App::Position::new(
                wrapper.display_information.real_area.left,
                wrapper.display_information.real_area.top,
            );

            wrapper.data.anchor.render_window_anchors(
                renderer,
                anchor_color,
                closest_anchor_color,
                shadow_color,
                shadow_padding,
                position,
                window_display_size,
                self.interface_scaling,
            );
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_tooltips(
        &self,
        renderer: &App::Renderer,
        tooltip_theme: &TooltipTheme<App>,
        tooltips: &[&str],
        mouse_position: App::Position,
    ) {
        let background_color = tooltip_theme.background_color;
        let foreground_color = tooltip_theme.foreground_color;
        let highlight_color = tooltip_theme.highlight_color;
        let shadow_color = tooltip_theme.shadow_color;
        let shadow_padding = tooltip_theme.shadow_padding;
        let font_size = tooltip_theme.font_size.scaled(self.interface_scaling);
        let corner_diameter = tooltip_theme.corner_diameter.scaled(self.interface_scaling);
        let border = tooltip_theme.border * self.interface_scaling;
        let gap = tooltip_theme.gap * self.interface_scaling;
        let mouse_offset = tooltip_theme.mouse_offset * self.interface_scaling;

        let total_offset = border * 2.0 + mouse_offset;
        let half_window_size = App::Size::new(self.window_size.width() / 2.0, self.window_size.height() / 2.0);
        let available_width = match mouse_position.left() > half_window_size.width() {
            true => mouse_position.left() - total_offset,
            false => self.window_size.width() - mouse_position.left() - total_offset,
        };

        let mut vertical_offset = 0.0;
        let mut forwards_iterator = tooltips.iter();
        let mut backwards_iterator = tooltips.iter().rev();

        let iterator: &mut dyn Iterator<Item = &&str> = match mouse_position.top() > half_window_size.height() {
            true => &mut backwards_iterator,
            false => &mut forwards_iterator,
        };

        for tooltip in iterator {
            let (text_dimensions, font_size) = self.text_layouter.get_text_dimensions(
                tooltip,
                foreground_color,
                highlight_color,
                font_size,
                available_width,
                tooltip_theme.overflow_behavior,
            );

            let tooltip_left = match mouse_position.left() > half_window_size.width() {
                true => mouse_position.left() - text_dimensions.width() - total_offset,
                false => mouse_position.left() + mouse_offset,
            };

            let tooltip_top = match mouse_position.top() > half_window_size.height() {
                true => mouse_position.top() - text_dimensions.height() / 2.0 - border - vertical_offset,
                false => mouse_position.top() - text_dimensions.height() / 2.0 - border + vertical_offset,
            };

            vertical_offset += text_dimensions.height() + border * 2.0 + gap;

            // TODO: Actually get the text dimensions and scale the tooltip size.

            renderer.render_rectangle(
                App::Position::new(tooltip_left, tooltip_top),
                App::Size::new(text_dimensions.width() + border * 2.0, text_dimensions.height() + border * 2.0),
                App::Clip::unbound(),
                corner_diameter,
                background_color,
                shadow_color,
                shadow_padding,
            );

            renderer.render_text(
                tooltip,
                App::Position::new(tooltip_left + border, tooltip_top + border),
                available_width,
                App::Clip::unbound(),
                foreground_color,
                highlight_color,
                font_size,
            );
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn click(&mut self, state: &State<App>, mouse_button: MouseButton) {
        self.event_queue.queue(Event::Unfocus);
        self.event_queue.queue(Event::CloseOverlay);

        if let Some(hovered_window) = self.hovered_window {
            self.event_queue.queue(Event::MoveWindowToTop { window_id: hovered_window });
        }

        if let Some(layout) = &self.overlay_layout {
            let mouse_button = match mouse_button.is_double_click() && !layout.has_button_registered(mouse_button) {
                true => mouse_button.as_single_click(),
                false => mouse_button,
            };

            layout.handle_click(state, self.event_queue, mouse_button);
        }

        if let Some(window_id) = &self.hovered_window {
            let layout = self.window_layouts.get(window_id).unwrap();

            let mouse_button = match mouse_button.is_double_click() && !layout.has_button_registered(mouse_button) {
                true => mouse_button.as_single_click(),
                false => mouse_button,
            };

            layout.handle_click(state, self.event_queue, mouse_button);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn drop(&mut self, state: &State<App>) {
        self.event_queue.queue(Event::SetMouseMode {
            mouse_mode: MouseMode::Default,
        });

        if let Some(layout) = &self.overlay_layout {
            layout.handle_drop(state, self.event_queue, self.mouse_mode);
        }

        if let Some(window_id) = &self.hovered_window {
            let layout = self.window_layouts.get(window_id).unwrap();

            layout.handle_drop(state, self.event_queue, self.mouse_mode);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn scroll(&mut self, state: &State<App>, delta: f32) {
        if let Some(layout) = &self.overlay_layout {
            layout.handle_scroll(state, self.event_queue, delta);
        }

        if let Some(window_id) = &self.hovered_window {
            let layout = self.window_layouts.get(window_id).unwrap();

            layout.handle_scroll(state, self.event_queue, delta);
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn input_characters(&mut self, state: &State<App>, characters: &[char]) -> bool {
        let mut input_handled = false;

        if let Some(layout) = &self.overlay_layout {
            for character in characters {
                input_handled |= layout.handle_character(state, self.event_queue, *character);
            }
        }

        for wrapper in self.windows {
            let layout = self.window_layouts.get(&wrapper.data.id).unwrap();

            for character in characters {
                input_handled |= layout.handle_character(state, self.event_queue, *character);
            }
        }

        input_handled
    }

    pub fn set_mouse_mode(&mut self, mouse_mode: impl Into<MouseMode<App>>) {
        self.event_queue.queue(Event::SetMouseMode {
            mouse_mode: mouse_mode.into(),
        });
    }

    pub fn focus_element(&mut self, focus_id: impl Any) {
        self.event_queue.queue(Event::FocusElement {
            focus_id: focus_id.focus_id(),
        });
    }

    pub fn unfocus(&mut self) {
        self.event_queue.queue(Event::Unfocus);
    }
}

impl<App: Application> Drop for InterfaceFrame<'_, App> {
    fn drop(&mut self) {
        // Convert FocusElement to FocusElementPost. This needs to be done before the
        // layouts are cleared. If a focus id could not be resolved we leave the event
        // as is, it will be ignored when processing the events.
        self.event_queue.iter_mut().for_each(|event| {
            if let Event::FocusElement { focus_id } = event {
                for layout in self.window_layouts.values() {
                    if let Some(element_id) = layout.try_resolve_focus_id(*focus_id) {
                        *event = Event::FocusElementPost { element_id };
                        return;
                    }
                }
            }
        });

        self.window_layouts.values_mut().for_each(|layout| layout.clear());

        if let Some(layout) = self.overlay_layout {
            layout.clear();
        }
    }
}
