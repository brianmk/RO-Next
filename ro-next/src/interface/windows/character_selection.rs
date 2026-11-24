use character_slot_preview::{CharacterSlotPreview, CharacterSlotPreviewHandler, OverlayHandler};
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{BaseLayoutInfo, Element};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolver, Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::ComputedSelector;
use ro_next_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::{ScreenPosition, ScreenSize};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::settings::InterfaceSettingsPathExt;
use crate::state::character_previews::CharacterPreviews;
use crate::state::character_selection_cooldown::CharacterSelectionCooldown;
use crate::state::character_slots::{CharacterSlots, CharacterSlotsExt, visible_character_slots};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{InterfaceTheme, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

/// Number of columns of the character slot grid.
const GRID_COLUMNS: usize = 3;
/// Width / height ratio of a character slot card.
///
/// A card is slightly taller than it is wide, like the original client's
/// portrait cards.
const SLOT_ASPECT_RATIO: f32 = 0.95;
/// Smallest width of a slot card, in interface pixels.
///
/// The window never gets smaller than this unless the screen is too small to
/// fit it; in that case it shrinks further so nothing is clipped.
const MIN_SLOT_WIDTH: f32 = 104.0;
/// Largest width of a slot card, in interface pixels.
///
/// This is what makes the window stop growing on large screens and keeps the
/// cards from becoming huge on a 2560x1440 display.
const MAX_SLOT_WIDTH: f32 = 240.0;
/// Gap between two slot cards, in interface pixels.
const GRID_GAP: f32 = 12.0;
/// Margin between the window and the screen edge, in interface pixels.
const VIEWPORT_MARGIN: f32 = 18.0;
/// Fraction of the preview area height that the character sprite may use.
///
/// The sprite is anchored at its feet at 80% of the preview area height (see
/// `renderer::interface::CHARACTER_FEET_ANCHOR`), so a fraction below 0.8
/// leaves room for the name above the character and level line below its feet.
const PREVIEW_HEIGHT_FRACTION: f32 = 0.66;
/// Fraction of the preview area width that the character sprite may use.
const PREVIEW_WIDTH_FRACTION: f32 = 0.9;

/// Window chrome metrics that the responsive window size has to account for.
///
/// All values are in interface pixels; the theme provides them, so the classic
/// (skinned) theme and the built-in default theme both keep their own frame
/// proportions.
#[derive(Clone, Copy, Debug)]
struct WindowChrome {
    title_height: f32,
    title_gap: f32,
    border: f32,
    gaps: f32,
    button_height: f32,
}

impl WindowChrome {
    fn from_theme(theme: &InterfaceTheme) -> Self {
        Self {
            title_height: theme.window.title_height,
            title_gap: theme.window.title_gap,
            border: theme.window.border,
            gaps: theme.window.gaps,
            button_height: theme.button.height,
        }
    }

    /// Vertical space the window chrome (title bar, frame and log out button
    /// row) uses in addition to the slot grid.
    fn height(&self) -> f32 {
        self.title_height + self.title_gap + self.border + self.gaps + self.button_height
    }
}

/// Responsive size of the character selection window and its slot grid.
///
/// The window size selectors only read the window dimensions; the card
/// geometry is used by the layout tests (and documents what the window size was
/// derived from).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(test), allow(dead_code))]
struct CharacterSelectionLayout {
    window_width: f32,
    window_height: f32,
    slot_width: f32,
    slot_height: f32,
    columns: usize,
    rows: usize,
}

/// Compute the responsive window and slot size.
///
/// The window uses the available screen area (viewport divided by the
/// interface scaling, because the interface is rendered scaled) and then
/// shrinks or grows the slot cards:
///
/// * on small screens the cards shrink until the complete window fits,
/// * on large screens the cards grow up to [`MAX_SLOT_WIDTH`] and the window
///   stays centred (the window anchor defaults to the screen centre),
/// * a slot width is never below [`MIN_SLOT_WIDTH`] unless that would make the
///   window larger than the screen.
fn character_selection_layout(
    reported_slot_count: usize,
    viewport: ScreenSize,
    scaling: f32,
    chrome: WindowChrome,
) -> CharacterSelectionLayout {
    // A server with zero slots (or a not-yet-filled character list) still gets
    // a valid one-card layout instead of a degenerate window.
    let slot_count = visible_character_slots(reported_slot_count).max(1);
    let columns = slot_count.min(GRID_COLUMNS);
    let rows = slot_count.div_ceil(columns);

    let scaling = if scaling.is_finite() && scaling > 0.0 { scaling } else { 1.0 };
    let viewport_width = if viewport.width.is_finite() && viewport.width > 0.0 {
        viewport.width
    } else {
        1280.0
    };
    let viewport_height = if viewport.height.is_finite() && viewport.height > 0.0 {
        viewport.height
    } else {
        720.0
    };

    // The interface layout works in unscaled interface pixels: the renderer
    // multiplies everything by the interface scaling. The viewport therefore
    // has to be divided by the scaling as well.
    let available_width = (viewport_width / scaling - 2.0 * VIEWPORT_MARGIN).max(0.0);
    let available_height = (viewport_height / scaling - 2.0 * VIEWPORT_MARGIN).max(0.0);

    let rows_f32 = rows as f32;
    let columns_f32 = columns as f32;
    let horizontal_chrome = 2.0 * chrome.border + (columns_f32 - 1.0) * GRID_GAP;
    let vertical_chrome = chrome.height() + (rows_f32 - 1.0) * GRID_GAP;

    let width_limited_slot = (available_width - horizontal_chrome) / columns_f32;
    let height_limited_slot = (available_height - vertical_chrome) / rows_f32 * SLOT_ASPECT_RATIO;

    let maximum_slot_width = width_limited_slot.min(height_limited_slot).min(MAX_SLOT_WIDTH);
    // Never force a minimum size that would not fit the screen: the `min` with
    // the available maximum keeps the window inside the viewport on tiny
    // resolutions.
    let minimum_slot_width = MIN_SLOT_WIDTH.min(maximum_slot_width);
    let slot_width = maximum_slot_width.max(minimum_slot_width).max(1.0);
    let slot_height = slot_width / SLOT_ASPECT_RATIO;

    let window_width = horizontal_chrome + columns_f32 * slot_width;
    let window_height = vertical_chrome + rows_f32 * slot_height;

    CharacterSelectionLayout {
        window_width,
        window_height,
        slot_width,
        slot_height,
        columns,
        rows,
    }
}

/// Responsive layout of the character selection window for the current client
/// state.
fn character_selection_layout_for_state(state: &ClientState) -> CharacterSelectionLayout {
    let theme = client_theme().follow_safe(state);
    let chrome = WindowChrome::from_theme(theme);
    let scaling = client_state().interface_settings().scaling().follow_safe(state).get_factor();
    let viewport = *client_state().window_size().follow_safe(state);
    let slot_count = client_state().character_slots().follow_safe(state).get_slot_count();

    character_selection_layout(slot_count, viewport, scaling, chrome)
}

/// Top-left position of the character selection window.
///
/// The window is placed explicitly (instead of using the cached anchor) so it
/// is always centred in the available interface area: the responsive window can
/// be almost screen-sized, and an anchor cached by an older client version
/// would otherwise push part of it off the screen.
pub fn character_selection_window_position(state: &ClientState) -> ScreenPosition {
    let layout = character_selection_layout_for_state(state);
    let scaling = client_state().interface_settings().scaling().follow_safe(state).get_factor();
    let scaling = if scaling.is_finite() && scaling > 0.0 { scaling } else { 1.0 };
    let viewport = *client_state().window_size().follow_safe(state);

    let available_width = viewport.width / scaling;
    let available_height = viewport.height / scaling;

    ScreenPosition {
        left: ((available_width - layout.window_width) / 2.0).max(0.0),
        top: ((available_height - layout.window_height) / 2.0).max(0.0),
    }
}

fn window_width_selector() -> impl Selector<ClientState, f32> {
    ComputedSelector::new_default(|state: &ClientState| character_selection_layout_for_state(state).window_width)
}

fn window_height_selector() -> impl Selector<ClientState, f32> {
    ComputedSelector::new_default(|state: &ClientState| character_selection_layout_for_state(state).window_height)
}

/// Scale of a character preview inside a slot card.
///
/// The idle frame of the composed animation data already contains all sprite
/// parts (body, hair, headgear) in their front-facing idle pose, so its pixel
/// height is the full drawn height of the character. The scale is chosen so
/// that the character uses the available area without clipping the headgear and
/// without covering the name and level text.
fn character_preview_scale(animation_data: &crate::world::AnimationData, area: Area) -> f32 {
    let frame = animation_data.animations.first().and_then(|animation| animation.frames.first());

    let (frame_width, frame_height) = match frame {
        Some(frame) => (frame.size.x as f32, frame.size.y as f32),
        // Fallback for incomplete animation data; a typical player idle frame
        // is roughly 64x96 pixels.
        None => (64.0, 96.0),
    };

    if frame_width <= 0.0 || frame_height <= 0.0 || area.width <= 0.0 || area.height <= 0.0 {
        return 1.0;
    }

    let height_scale = area.height * PREVIEW_HEIGHT_FRACTION / frame_height;
    let width_scale = area.width * PREVIEW_WIDTH_FRACTION / frame_width;

    height_scale.min(width_scale).clamp(0.1, 16.0)
}

mod character_slot_preview {
    use std::cell::{Cell, UnsafeCell};
    use std::time::Instant;

    use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
    use ro_next_interface::element::{BaseLayoutInfo, Element};
    use ro_next_interface::event::{ClickHandler, EventQueue};
    use ro_next_interface::layout::alignment::{HorizontalAlignment, VerticalAlignment};
    use ro_next_interface::layout::area::Area;
    use ro_next_interface::layout::tooltip::TooltipExt;
    use ro_next_interface::layout::{MouseButton, Resolvers, WindowLayout, with_single_resolver};
    use ragnarok_packets::CharacterInformation;
    use rust_state::{Path, State};

    use super::character_preview_scale;
    use crate::graphics::{Color, CornerDiameter, ScreenPosition, ScreenSize, ShadowPadding};
    use crate::input::InputEvent;
    use crate::loaders::OverflowBehavior;
    use crate::renderer::LayoutExt;
    use crate::state::character_previews::CharacterPreviews;
    use crate::state::character_selection_cooldown::CharacterSelectionCooldown;
    use crate::state::localization::LocalizationPathExt;
    use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, TypographyTokensPathExt};
    use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

    /// Draws the right-click context menu (delete / switch / cancel) of a
    /// character slot.
    pub struct OverlayHandler<A, B> {
        position: ScreenPosition,
        size: ScreenSize,
        slot: usize,
        switch_request_path: A,
        character_information_path: B,
        window_id: u64,
    }

    impl<A, B> OverlayHandler<A, B> {
        pub fn new(slot: usize, switch_request_path: A, character_information_path: B) -> Self {
            Self {
                position: ScreenPosition { left: 0.0, top: 0.0 },
                size: ScreenSize { width: 0.0, height: 0.0 },
                slot,
                switch_request_path,
                character_information_path,
                window_id: 0,
            }
        }

        fn set_position_size(&mut self, position: ScreenPosition, size: ScreenSize, window_id: u64) {
            self.position = position;
            self.size = size;
            self.window_id = window_id;
        }
    }

    impl<A, B> ClickHandler<ClientState> for OverlayHandler<A, B>
    where
        A: Path<ClientState, Option<usize>>,
        B: Path<ClientState, CharacterInformation, false>,
    {
        fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            use ro_next_interface::prelude::*;

            let slot = self.slot;
            let switch_request_path = self.switch_request_path;
            let character_information_path = self.character_information_path;

            let element = ErasedElement::new(fragment! {
                gaps: 4.0,
                children: (
                    button! {
                        text: client_state().localization().delete_button_text(),
                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            // We should not be able to get here if the character is not present, so it's
                            // fine to unwrap.
                            let character_information = state.try_get(&character_information_path).unwrap();
                            let character_id = character_information.character_id;

                            // The character server requires the account e-mail as
                            // the deletion confirmation code, so a separate window
                            // asks for it instead of deleting immediately.
                            queue.queue(InputEvent::OpenCharacterDeleteWindow { character_id });
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                    button! {
                        text: client_state().localization().switch_button_text(),
                        event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            state.update_value(switch_request_path, Some(slot));
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                    button! {
                        text: client_state().localization().cancel_button_text(),
                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                            queue.queue(Event::CloseOverlay);
                        },
                    },
                ),
            });

            queue.queue(Event::OpenOverlay {
                element,
                position: self.position,
                size: self.size,
                window_id: self.window_id,
            });
        }
    }

    /// Cached `base/job` level text of one slot.
    ///
    /// The level values are part of the client state and can change while a
    /// window is open (level up while another character is selected in the
    /// background), so the text has to be refreshed lazily without allocating
    /// every frame.
    struct LevelDisplay {
        last_value: Cell<Option<(i16, i32)>>,
        text: UnsafeCell<String>,
    }

    impl LevelDisplay {
        fn new() -> Self {
            Self {
                last_value: Cell::default(),
                text: UnsafeCell::default(),
            }
        }

        fn get_str(&self, base_level: i16, job_level: i32) -> &str {
            let value = (base_level, job_level);
            let last_value = self.last_value.get();

            if last_value.is_none() || last_value.is_some_and(|last| last != value) {
                // SAFETY: this is the same single-threaded cache pattern used
                // by `PartialEqDisplayStr` in the interface crate: the string
                // is only written and read from the layout pass, and the
                // returned reference is only used until the next layout pass.
                unsafe { *self.text.get() = format!("{base_level}/{job_level}") };
                self.last_value.set(Some(value));
            }

            unsafe { self.text.as_ref_unchecked() }
        }
    }

    /// Cached tooltip text (`name` + destination map) of one slot.
    struct TooltipDisplay {
        text: UnsafeCell<String>,
    }

    impl TooltipDisplay {
        fn new() -> Self {
            Self {
                text: UnsafeCell::default(),
            }
        }

        fn get_str<'a>(&'a self, name: &str, map_label: &str, map_name: &str) -> &'a str {
            // SAFETY: see [`LevelDisplay::get_str`]; the string is rebuilt on
            // the hovered slot only and borrowed for the current layout pass.
            unsafe {
                *self.text.get() = format!("{name}\n{map_label}: {map_name}");
                self.text.as_ref_unchecked()
            }
        }
    }

    /// Cached "Please wait… N s" text of a card that is blocked by the
    /// anti-crash selection cooldown.
    struct PleaseWaitDisplay {
        last_seconds: Cell<Option<u64>>,
        text: UnsafeCell<String>,
    }

    impl PleaseWaitDisplay {
        fn new() -> Self {
            Self {
                last_seconds: Cell::default(),
                text: UnsafeCell::default(),
            }
        }

        fn get_str(&self, label: &str, remaining: std::time::Duration) -> &str {
            // Round up so the last second is still shown as "1 s" instead of
            // disappearing early.
            let seconds = remaining.as_secs_f64().ceil() as u64;

            if self.last_seconds.get() != Some(seconds) {
                // SAFETY: single-threaded layout cache, see [`LevelDisplay`].
                unsafe { *self.text.get() = format!("{label} {seconds}s") };
                self.last_seconds.set(Some(seconds));
            }

            unsafe { self.text.as_ref_unchecked() }
        }
    }

    pub struct CharacterSlotPreview<P, S, V, W> {
        character_information: P,
        switch_request: S,
        previews: V,
        cooldown: W,
        click_handler: CharacterSlotPreviewHandler<S>,
        overlay_handler: OverlayHandler<S, P>,
        slot: usize,
    }

    impl<P, S, V, W> CharacterSlotPreview<P, S, V, W> {
        #[allow(clippy::too_many_arguments)]
        pub fn new(
            character_information: P,
            switch_request: S,
            previews: V,
            cooldown: W,
            click_handler: CharacterSlotPreviewHandler<S>,
            overlay_handler: OverlayHandler<S, P>,
            slot: usize,
        ) -> Self {
            Self {
                character_information,
                switch_request,
                previews,
                cooldown,
                click_handler,
                overlay_handler,
                slot,
            }
        }
    }

    impl<P, S, V, W> Element<ClientState> for CharacterSlotPreview<P, S, V, W>
    where
        P: Path<ClientState, CharacterInformation, false>,
        S: Path<ClientState, Option<usize>>,
        V: Path<ClientState, CharacterPreviews>,
        W: Path<ClientState, CharacterSelectionCooldown>,
    {
        type LayoutInfo = BaseLayoutInfo;

        fn create_layout_info(
            &mut self,
            _: &State<ClientState>,
            store: ElementStoreMut,
            resolvers: &mut dyn Resolvers<ClientState>,
        ) -> Self::LayoutInfo {
            with_single_resolver(resolvers, |resolver| {
                let available = resolver.push_available_area();
                let area = Area {
                    left: available.left,
                    top: available.top,
                    width: available.width,
                    height: available.height.unwrap_or(0.0),
                };

                self.overlay_handler.set_position_size(
                    ScreenPosition {
                        left: area.left,
                        top: area.top,
                    },
                    ScreenSize {
                        width: area.width,
                        height: area.height,
                    },
                    store.get_window_id(),
                );

                resolver.commit_used_height(area.height);

                BaseLayoutInfo { area }
            })
        }

        fn lay_out<'a>(
            &'a self,
            state: &'a State<ClientState>,
            _: ElementStore<'a>,
            layout_info: &'a Self::LayoutInfo,
            layout: &mut WindowLayout<'a, ClientState>,
        ) {
            let color_tokens = client_theme().tokens().color();
            let typography = client_theme().tokens().typography();
            let area = layout_info.area;

            let padding = (area.height * 0.025).clamp(1.0, 4.0);
            let corner_diameter = CornerDiameter::uniform((area.height * 0.06).clamp(4.0, 14.0));
            let accent_color = *state.get(&color_tokens.text_accent());
            let title_token = *state.get(&typography.font_title());
            let body_token = *state.get(&typography.font_body_lg());
            // The fonts scale with the card size so long names and the level
            // line still fit when the window has to shrink on a small screen.
            let title_font = title_token.clamped_to(area.height * 0.075, 9.0);
            let body_font = body_token.clamped_to(area.height * 0.07, 9.0);
            let level_font = body_token.clamped_to(area.height * 0.06, 8.0);

            let character_information = state.try_get(&self.character_information);

            // While a character is being moved to another slot, the occupied
            // cards show the switch overlay and the origin card shows cancel. An
            // empty slot cannot receive a character (the character server
            // rejects that), so it stays a normal (inactive) empty card.
            if let Some(switch_request) = state.get(&self.switch_request) {
                let is_origin = *switch_request == self.slot;

                if !is_origin && character_information.is_none() {
                    layout.add_rectangle(
                        area,
                        corner_diameter,
                        *state.get(&color_tokens.surface_raised()),
                        *state.get(&color_tokens.border()),
                        ShadowPadding::diagonal(2.0, 5.0),
                    );

                    layout.add_text(
                        area,
                        state.follow(client_state().localization().create_character_slot_text()).as_str(),
                        body_font,
                        *state.get(&color_tokens.text_disabled()),
                        accent_color,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: padding,
                        },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    return;
                }

                let is_hovered = area.check().run(layout);

                let background_color = match is_hovered {
                    true => *state.get(&color_tokens.state_active()),
                    false => *state.get(&color_tokens.state_hover()),
                };
                layout.add_rectangle(
                    area,
                    corner_diameter,
                    background_color,
                    *state.get(&color_tokens.border()),
                    ShadowPadding::diagonal(2.0, 5.0),
                );

                if is_origin {
                    layout.add_text(
                        area,
                        state.follow(client_state().localization().cancel_button_text()).as_str(),
                        body_font,
                        *state.get(&color_tokens.text_overlay()),
                        accent_color,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: padding,
                        },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hovered {
                        layout.register_click_handler(MouseButton::Left, &self.click_handler.cancel_switch);
                    }
                } else {
                    layout.add_text(
                        area,
                        state.follow(client_state().localization().switch_slots_text()).as_str(),
                        body_font,
                        *state.get(&color_tokens.text_overlay()),
                        accent_color,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: padding,
                        },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hovered {
                        layout.register_click_handler(MouseButton::Left, &self.click_handler.request_switch);
                    }
                }

                return;
            }

            match character_information {
                Some(character_information) => {
                    // Characters are blocked for a moment after a logout, so a
                    // fast re-selection cannot crash the client while the server
                    // still cleans up the previous session. Blocked cards show
                    // the localized "Please wait…" text and a countdown and do
                    // not register any click handlers.
                    let now = Instant::now();
                    let remaining = state.get(&self.cooldown).remaining(character_information.character_id, now);
                    let is_blocked = remaining.is_some();
                    let is_hovered = !is_blocked && area.check().run(layout);

                    let background_color = match (is_blocked, is_hovered) {
                        (true, _) => *state.get(&color_tokens.state_disabled()),
                        (false, true) => *state.get(&color_tokens.surface_highlight()),
                        (false, false) => *state.get(&color_tokens.surface_selected()),
                    };
                    let border_color = match (is_blocked, is_hovered) {
                        (true, _) => *state.get(&color_tokens.border()),
                        (false, true) => accent_color,
                        (false, false) => *state.get(&color_tokens.border()),
                    };

                    layout.add_rectangle(
                        area,
                        corner_diameter,
                        background_color,
                        border_color,
                        ShadowPadding::uniform(1.5),
                    );

                    // The actual composited character (body, hair, headgear and
                    // palettes) in its idle front-facing frame, exactly like the
                    // equipment paper doll. Blocked previews are tinted down.
                    if let Some(animation_data) = state.get(&self.previews).animation_data(self.slot) {
                        let preview_area = Area {
                            left: area.left + padding,
                            top: area.top + padding,
                            width: (area.width - padding * 2.0).max(0.0),
                            height: (area.height - padding * 2.0).max(0.0),
                        };
                        let scale = character_preview_scale(animation_data.as_ref(), preview_area);
                        let tint = match is_blocked {
                            true => *state.get(&color_tokens.text_disabled()),
                            false => Color::WHITE,
                        };

                        layout.with_clip(area, |layout| {
                            layout.add_character(preview_area, animation_data.as_ref(), tint, scale);
                        });
                    }

                    let name_color = match is_blocked {
                        true => *state.get(&color_tokens.text_disabled()),
                        false => *state.get(&color_tokens.text_value()),
                    };
                    let level_color = match is_blocked {
                        true => *state.get(&color_tokens.text_disabled()),
                        false => *state.get(&color_tokens.text_secondary()),
                    };

                    // Character name at the top of the card.
                    layout.add_text(
                        area,
                        &character_information.name,
                        title_font,
                        name_color,
                        accent_color,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: padding,
                        },
                        VerticalAlignment::Top { offset: padding },
                        OverflowBehavior::Shrink,
                    );

                    // Compact `base/job` level at the bottom, without the
                    // separate "Base level" / "Job level" labels.
                    layout.add_text(
                        area,
                        self.click_handler
                            .level_display
                            .get_str(character_information.base_level, character_information.job_level),
                        level_font,
                        level_color,
                        accent_color,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: padding,
                        },
                        VerticalAlignment::Bottom { offset: padding },
                        OverflowBehavior::Shrink,
                    );

                    if let Some(remaining) = remaining {
                        let please_wait = state.follow(client_state().localization().please_wait_text()).as_str();

                        layout.add_text(
                            area,
                            self.click_handler.please_wait_display.get_str(please_wait, remaining),
                            body_token.clamped_to(area.height * 0.065, 8.0),
                            *state.get(&color_tokens.state_warning()),
                            accent_color,
                            HorizontalAlignment::Center {
                                offset: 0.0,
                                border: padding,
                            },
                            VerticalAlignment::Bottom {
                                offset: padding + body_token.0,
                            },
                            OverflowBehavior::Shrink,
                        );

                        return;
                    }

                    if is_hovered {
                        layout.register_click_handler(MouseButton::Left, &self.click_handler.select_character);
                        layout.register_click_handler(MouseButton::DoubleLeft, &self.click_handler.select_character);
                        layout.register_click_handler(MouseButton::Right, &self.overlay_handler);

                        let map_label = state.follow(client_state().localization().map_text()).as_str();
                        let map_name = character_information
                            .map_name
                            .strip_suffix(".gat")
                            .unwrap_or(&character_information.map_name);
                        let tooltip = self
                            .click_handler
                            .tooltip_display
                            .get_str(&character_information.name, map_label, map_name);

                        {
                            struct PrivateTooltipId;
                            layout.add_tooltip(tooltip, PrivateTooltipId.tooltip_id());
                        }
                    }
                }
                None => {
                    let is_hovered = area.check().run(layout);

                    let background_color = match is_hovered {
                        true => *state.get(&color_tokens.state_disabled()),
                        false => *state.get(&color_tokens.surface_raised()),
                    };
                    layout.add_rectangle(
                        area,
                        corner_diameter,
                        background_color,
                        *state.get(&color_tokens.border()),
                        ShadowPadding::diagonal(2.0, 5.0),
                    );

                    layout.add_text(
                        area,
                        state.follow(client_state().localization().create_character_slot_text()).as_str(),
                        body_font,
                        *state.get(&color_tokens.text_disabled()),
                        accent_color,
                        HorizontalAlignment::Center {
                            offset: 0.0,
                            border: padding,
                        },
                        VerticalAlignment::Center { offset: 0.0 },
                        OverflowBehavior::Shrink,
                    );

                    if is_hovered {
                        layout.register_click_handler(MouseButton::Left, &self.click_handler.create_character);
                    }
                }
            }
        }
    }

    struct SelectCharacter {
        slot: usize,
    }

    impl ClickHandler<ClientState> for SelectCharacter {
        fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            queue.queue(InputEvent::SelectCharacter { slot: self.slot });
        }
    }

    struct CreateCharacter {
        slot: usize,
    }

    impl ClickHandler<ClientState> for CreateCharacter {
        fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            queue.queue(InputEvent::OpenCharacterCreationWindow { slot: self.slot });
        }
    }

    struct CancelSwitch<P> {
        switch_request: P,
    }

    impl<P> ClickHandler<ClientState> for CancelSwitch<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
            state.update_value(self.switch_request, None);
        }
    }

    struct RequestSwitch<P> {
        switch_request: P,
        slot: usize,
    }

    impl<P> ClickHandler<ClientState> for RequestSwitch<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        fn handle_click(&self, state: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
            // We should not be able to get here if there is no switch request, so it's
            // fine to unwrap.
            let origin_slot = state.get(&self.switch_request).unwrap();

            queue.queue(InputEvent::SwitchCharacterSlot {
                origin_slot,
                destination_slot: self.slot,
            });
        }
    }

    pub struct CharacterSlotPreviewHandler<P> {
        select_character: SelectCharacter,
        create_character: CreateCharacter,
        cancel_switch: CancelSwitch<P>,
        request_switch: RequestSwitch<P>,
        level_display: LevelDisplay,
        tooltip_display: TooltipDisplay,
        please_wait_display: PleaseWaitDisplay,
    }

    impl<P> CharacterSlotPreviewHandler<P>
    where
        P: Path<ClientState, Option<usize>>,
    {
        pub fn new(switch_request: P, slot: usize) -> Self {
            Self {
                select_character: SelectCharacter { slot },
                create_character: CreateCharacter { slot },
                cancel_switch: CancelSwitch { switch_request },
                request_switch: RequestSwitch { switch_request, slot },
                level_display: LevelDisplay::new(),
                tooltip_display: TooltipDisplay::new(),
                please_wait_display: PleaseWaitDisplay::new(),
            }
        }
    }
}

/// Layout information of the complete slot grid.
struct CharacterSelectionGridLayoutInfo {
    slots: Vec<BaseLayoutInfo>,
}

/// The responsive 3x3 grid of character slot cards.
struct CharacterSelectionGrid<C, M, V, W> {
    character_slots: C,
    switch_request: M,
    previews: V,
    cooldown: W,
    cells: Vec<Box<dyn Element<ClientState, LayoutInfo = BaseLayoutInfo>>>,
    cell_count: usize,
}

impl<C, M, V, W> CharacterSelectionGrid<C, M, V, W>
where
    C: Path<ClientState, CharacterSlots>,
    M: Path<ClientState, Option<usize>>,
    V: Path<ClientState, CharacterPreviews>,
    W: Path<ClientState, CharacterSelectionCooldown>,
{
    fn new(character_slots: C, switch_request: M, previews: V, cooldown: W) -> Self {
        Self {
            character_slots,
            switch_request,
            previews,
            cooldown,
            cells: Vec::new(),
            cell_count: 0,
        }
    }

    /// Rebuild the slot elements when the number of visible slots changed.
    ///
    /// The character server can report more slots than the window shows; the
    /// grid never creates more than nine cards.
    fn refresh_cells(&mut self, reported_slot_count: usize) {
        let cell_count = visible_character_slots(reported_slot_count).max(1);

        if self.cell_count == cell_count && self.cells.len() == cell_count {
            return;
        }

        let character_slots = self.character_slots;
        let switch_request = self.switch_request;
        let previews = self.previews;
        let cooldown = self.cooldown;

        self.cells.clear();
        self.cells.reserve(cell_count);

        for slot in 0..cell_count {
            self.cells.push(Box::new(CharacterSlotPreview::new(
                character_slots.in_slot(slot),
                switch_request,
                previews,
                cooldown,
                CharacterSlotPreviewHandler::new(switch_request, slot),
                OverlayHandler::new(slot, switch_request, character_slots.in_slot(slot)),
                slot,
            )));
        }

        self.cell_count = cell_count;
    }
}

impl<C, M, V, W> Element<ClientState> for CharacterSelectionGrid<C, M, V, W>
where
    C: Path<ClientState, CharacterSlots>,
    M: Path<ClientState, Option<usize>>,
    V: Path<ClientState, CharacterPreviews>,
    W: Path<ClientState, CharacterSelectionCooldown>,
{
    type LayoutInfo = CharacterSelectionGridLayoutInfo;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let reported_slot_count = {
            let character_slots = state.get(&self.character_slots);
            character_slots.get_slot_count()
        };
        self.refresh_cells(reported_slot_count);

        with_single_resolver(resolvers, |resolver| {
            let available = resolver.push_available_area();
            let available_width = available.width.max(0.0);
            let available_height = available.height.unwrap_or(0.0).max(0.0);

            // Reserve the last row for the log out button (the second element of
            // the window); the window size was computed with exactly this space.
            let chrome = WindowChrome::from_theme(state.get(&client_theme()));
            let grid_height = (available_height - chrome.button_height - chrome.gaps).max(0.0);

            let cell_count = self.cells.len().max(1);
            let columns = cell_count.min(GRID_COLUMNS);
            let rows = cell_count.div_ceil(columns);
            let columns_f32 = columns as f32;
            let rows_f32 = rows as f32;

            // Defensive: if the window was resized by the interface settings
            // chrome (which can override the min/max size), the grid still has
            // to fit into the available area.
            let max_cell_width = (available_width - (columns_f32 - 1.0) * GRID_GAP) / columns_f32;
            let max_cell_height = (grid_height - (rows_f32 - 1.0) * GRID_GAP) / rows_f32;
            let slot_width = max_cell_width.min(max_cell_height * SLOT_ASPECT_RATIO).max(1.0);
            let slot_height = slot_width / SLOT_ASPECT_RATIO;

            let used_width = columns_f32 * slot_width + (columns_f32 - 1.0) * GRID_GAP;
            let used_height = rows_f32 * slot_height + (rows_f32 - 1.0) * GRID_GAP;
            let left = available.left + (available_width - used_width).max(0.0) / 2.0;
            let top = available.top + (grid_height - used_height).max(0.0) / 2.0;

            let text_layouter = resolver.get_text_layouter();
            let mut slots = Vec::with_capacity(self.cells.len());

            for (index, cell) in self.cells.iter_mut().enumerate() {
                let row = index / columns;
                let column = index % columns;

                let cell_area = Area {
                    left: left + column as f32 * (slot_width + GRID_GAP),
                    top: top + row as f32 * (slot_height + GRID_GAP),
                    width: slot_width,
                    height: slot_height,
                };

                let mut cell_resolver = Resolver::new(cell_area, 0.0, text_layouter);
                let layout_info = cell.create_layout_info(state, store.child_store(index as u64), &mut cell_resolver);

                slots.push(layout_info);
            }

            resolver.commit_used_height(grid_height);

            CharacterSelectionGridLayoutInfo { slots }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        for (index, cell) in self.cells.iter().enumerate() {
            cell.lay_out(state, store.child_store(index as u64), &layout_info.slots[index], layout);
        }
    }
}

pub struct CharacterSelectionWindow<C, M, V, W> {
    character_slots: C,
    switch_request: M,
    previews: V,
    cooldown: W,
}

impl<C, M, V, W> CharacterSelectionWindow<C, M, V, W> {
    pub fn new(character_slots: C, switch_request: M, previews: V, cooldown: W) -> Self {
        Self {
            character_slots,
            switch_request,
            previews,
            cooldown,
        }
    }
}

impl<C, M, V, W> CustomWindow<ClientState> for CharacterSelectionWindow<C, M, V, W>
where
    C: Path<ClientState, CharacterSlots>,
    M: Path<ClientState, Option<usize>>,
    V: Path<ClientState, CharacterPreviews>,
    W: Path<ClientState, CharacterSelectionCooldown>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterSelection)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        window! {
            title: client_state().localization().select_character_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::Menu,
            minimum_width: window_width_selector(),
            maximum_width: window_width_selector(),
            minimum_height: window_height_selector(),
            maximum_height: window_height_selector(),
            elements: (
                CharacterSelectionGrid::new(self.character_slots, self.switch_request, self.previews, self.cooldown),
                button! {
                    text: client_state().localization().log_out_button_text(),
                    event: InputEvent::LogOutCharacter,
                },
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GRID_GAP, MAX_SLOT_WIDTH, MIN_SLOT_WIDTH, SLOT_ASPECT_RATIO, ScreenSize, VIEWPORT_MARGIN, WindowChrome, character_selection_layout,
        visible_character_slots,
    };

    fn menu_chrome() -> WindowChrome {
        WindowChrome {
            title_height: 45.0,
            title_gap: 20.0,
            border: 30.0,
            gaps: 25.0,
            button_height: 30.0,
        }
    }

    fn classic_chrome() -> WindowChrome {
        WindowChrome {
            title_height: 22.0,
            title_gap: 2.0,
            border: 18.0,
            gaps: 10.0,
            button_height: 20.0,
        }
    }

    fn viewport(width: f32, height: f32) -> ScreenSize {
        ScreenSize { width, height }
    }

    fn assert_fits(layout: super::CharacterSelectionLayout, viewport: ScreenSize, scaling: f32, chrome: WindowChrome) {
        assert!(layout.window_width > 0.0);
        assert!(layout.window_height > 0.0);
        assert!(
            layout.window_width <= viewport.width / scaling,
            "window width {} does not fit viewport {} at scale {}",
            layout.window_width,
            viewport.width,
            scaling,
        );
        assert!(
            layout.window_height <= viewport.height / scaling,
            "window height {} does not fit viewport {} at scale {}",
            layout.window_height,
            viewport.height,
            scaling,
        );
        assert!((layout.slot_width / layout.slot_height - SLOT_ASPECT_RATIO).abs() < 0.001);

        let used_width = layout.columns as f32 * layout.slot_width + (layout.columns - 1) as f32 * GRID_GAP;
        let used_height = layout.rows as f32 * layout.slot_height + (layout.rows - 1) as f32 * GRID_GAP;
        assert!((layout.window_width - (2.0 * chrome.border + used_width)).abs() < 0.01);
        assert!((layout.window_height - (chrome.height() + used_height)).abs() < 0.01);
        // The window is kept inside the viewport margin.
        assert!(layout.window_width <= viewport.width / scaling - 2.0 * VIEWPORT_MARGIN + 0.01);
        assert!(layout.window_height <= viewport.height / scaling - 2.0 * VIEWPORT_MARGIN + 0.01);
    }

    #[test]
    fn three_by_three_grid_for_nine_slots() {
        let layout = character_selection_layout(9, viewport(1280.0, 720.0), 1.0, menu_chrome());

        assert_eq!(layout.columns, 3);
        assert_eq!(layout.rows, 3);
        assert!(layout.slot_width >= MIN_SLOT_WIDTH);
        assert!(layout.slot_width <= MAX_SLOT_WIDTH);
    }

    #[test]
    fn window_fits_720p_and_1440p_at_all_supported_scalings() {
        for scaling in [0.75, 1.0, 1.25] {
            for viewport in [viewport(1280.0, 720.0), viewport(2560.0, 1440.0)] {
                for chrome in [menu_chrome(), classic_chrome()] {
                    let layout = character_selection_layout(9, viewport, scaling, chrome);
                    assert_fits(layout, viewport, scaling, chrome);
                }
            }
        }
    }

    #[test]
    fn window_shrinks_on_small_screens_instead_of_overflowing() {
        for scaling in [0.75, 1.0, 1.25] {
            for viewport in [viewport(1024.0, 600.0), viewport(800.0, 600.0), viewport(640.0, 480.0)] {
                let chrome = menu_chrome();
                let layout = character_selection_layout(9, viewport, scaling, chrome);
                assert_fits(layout, viewport, scaling, chrome);
            }
        }
    }

    #[test]
    fn window_grows_on_large_screens_up_to_the_maximum() {
        let small = character_selection_layout(9, viewport(1280.0, 720.0), 1.0, menu_chrome());
        let large = character_selection_layout(9, viewport(2560.0, 1440.0), 1.0, menu_chrome());

        assert!(large.window_width > small.window_width);
        assert!(large.window_height > small.window_height);
        assert!(large.slot_width <= MAX_SLOT_WIDTH);
        assert!(large.slot_width >= small.slot_width);
    }

    #[test]
    fn slot_count_controls_the_grid_but_never_exceeds_nine_cards() {
        let no_slots = character_selection_layout(0, viewport(1280.0, 720.0), 1.0, menu_chrome());
        assert_eq!((no_slots.columns, no_slots.rows), (1, 1));

        let one = character_selection_layout(1, viewport(1280.0, 720.0), 1.0, menu_chrome());
        assert_eq!((one.columns, one.rows), (1, 1));

        let two = character_selection_layout(2, viewport(1280.0, 720.0), 1.0, menu_chrome());
        assert_eq!((two.columns, two.rows), (2, 1));

        let four = character_selection_layout(4, viewport(1280.0, 720.0), 1.0, menu_chrome());
        assert_eq!((four.columns, four.rows), (3, 2));

        let twelve = character_selection_layout(12, viewport(1280.0, 720.0), 1.0, menu_chrome());
        assert_eq!((twelve.columns, twelve.rows), (3, 3));
    }

    #[test]
    fn server_slot_limits_are_clamped_to_nine() {
        assert_eq!(visible_character_slots(0), 0);
        assert_eq!(visible_character_slots(9), 9);
        assert_eq!(visible_character_slots(12), 9);
    }

    #[test]
    fn invalid_scaling_or_viewport_uses_safe_fallbacks() {
        let chrome = menu_chrome();
        let zero_scaling = character_selection_layout(9, viewport(1280.0, 720.0), 0.0, chrome);
        let nan_scaling = character_selection_layout(9, viewport(1280.0, 720.0), f32::NAN, chrome);
        let zero_viewport = character_selection_layout(9, viewport(0.0, 0.0), 1.0, chrome);

        for layout in [zero_scaling, nan_scaling, zero_viewport] {
            assert!(layout.window_width.is_finite());
            assert!(layout.window_height.is_finite());
            assert!(layout.window_width > 0.0);
            assert!(layout.window_height > 0.0);
        }
    }
}
