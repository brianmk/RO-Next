use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::{Color, CornerDiameter};
use crate::input::InputEvent;
use crate::interface::components::progress_bar::{BarStyle, LOW_HEALTH_FRACTION, LabeledProgressBar, MiniBar, StackedMiniBars};
use crate::interface::components::round_button::RoundButton;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{
    CharacterOverviewThemePathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, TypographyTokensPathExt,
};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};
use crate::world::Player;

/// Width of the character overview window in its normal (expanded) form.
///
/// The window is split into a left column with the header and the stat bars and
/// a right column with the two-row action button grid.
const NORMAL_WINDOW_WIDTH: f32 = 460.0;
/// Width of the collapsed character overview window.
///
/// The collapsed form is a small always-on minimalist HUD that only shows the
/// bars (no text at all).
const COMPACT_WINDOW_WIDTH: f32 = 150.0;
/// Window border of the collapsed form, in interface pixels.
const COMPACT_BORDER: f32 = 5.0;
/// Gaps between the elements of the collapsed form.
const COMPACT_GAPS: f32 = 2.5;
/// Title bar height of the collapsed form.
///
/// The minimalist HUD has no title text, but a few pixels are kept so the HUD
/// can still be dragged around like any other window.
const COMPACT_TITLE_HEIGHT: f32 = 12.0;
/// Corner diameter of the collapsed HUD panel.
const COMPACT_CORNER_DIAMETER: f32 = 6.0;
/// Maximum height of the collapsed HUD, used to clamp a cached height from the
/// expanded form.
const COMPACT_MAXIMUM_HEIGHT: f32 = 100.0;
/// Size of the round expand/collapse buttons.
const ROUND_BUTTON_SIZE: f32 = ACTION_BUTTON_HEIGHT;
/// Size of the round expand button of the minimalist HUD.
const HUD_ROUND_BUTTON_SIZE: f32 = 14.0;
/// Height of one bar line of the minimalist HUD.
const HUD_BAR_HEIGHT: f32 = 7.0;
/// Height of the combined EXP/job EXP line of the minimalist HUD.
const HUD_STACK_HEIGHT: f32 = 12.0;
/// Gap between the two stacked bars of the minimalist HUD.
const HUD_STACK_GAP: f32 = 2.0;

/// Layout of the bars in the expanded form.
const EXPANDED_LABEL_WIDTH: f32 = 32.0;
const EXPANDED_LABEL_FONT_SIZE: f32 = 11.0;
const EXPANDED_VALUE_FONT_SIZE: f32 = 9.0;
const EXPANDED_BAR_GAP: f32 = 4.0;
const EXPANDED_BAR_HEIGHT: f32 = 14.0;
/// Size of the compact action buttons in the expanded form.
const ACTION_BUTTON_HEIGHT: f32 = 18.0;

/// Font size of the level line in the expanded form.
const HEADER_FONT_SIZE: f32 = 11.0;

fn expanded_bar_style() -> BarStyle {
    BarStyle::new(
        EXPANDED_LABEL_WIDTH,
        EXPANDED_LABEL_FONT_SIZE,
        EXPANDED_VALUE_FONT_SIZE,
        EXPANDED_BAR_GAP,
        EXPANDED_BAR_HEIGHT,
    )
}

/// The character overview window.
///
/// The window has two forms:
/// * the expanded form shows a compact header with the name and the base/job
///   level on one line, the HP/SP/base EXP/job EXP bars with their values
///   inside the bar and a two-row grid of action buttons on the right;
/// * the collapsed form only shows the level line and four micro bars. The
///   collapsed state is persisted in the interface settings.
pub struct CharacterOverviewWindow<A, B, C> {
    player_name_path: A,
    player_path: B,
    interface_settings_path: C,
}

impl<A, B, C> CharacterOverviewWindow<A, B, C> {
    /// Creates a new character overview window.
    ///
    /// `player_path` should point at the currently selected player character,
    /// for example [`crate::state::this_player`] with
    /// [`rust_state::ManuallyAssertExt::manually_asserted`] applied to it.
    pub fn new(player_name_path: A, player_path: B, interface_settings_path: C) -> Self {
        Self {
            player_name_path,
            player_path,
            interface_settings_path,
        }
    }
}

/// Click handler for the collapse/expand toggle of the character overview.
///
/// Besides toggling the state (which is read every frame, so the change is
/// applied live) the interface settings are saved immediately, so the chosen
/// mode survives crashes and relogs like the other interface settings.
struct CharacterOverviewCompactToggle<A, B> {
    value_path: A,
    settings_path: B,
}

impl<A, B> ClickHandler<ClientState> for CharacterOverviewCompactToggle<A, B>
where
    A: Path<ClientState, bool>,
    B: Path<ClientState, InterfaceSettings>,
{
    fn handle_click(&self, state: &State<ClientState>, _: &mut EventQueue<ClientState>) {
        state.update_value_with(self.value_path, |compact| *compact = !*compact);
        state.update_value_with(self.settings_path, |settings| settings.save());
    }
}

/// Selector for the window width, depending on whether the collapsed mode is
/// active.
fn window_width<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => COMPACT_WINDOW_WIDTH,
        false => NORMAL_WINDOW_WIDTH,
    })
}

/// Selector for the window border: the collapsed form uses a much smaller
/// border so that almost the whole width is available for the bars.
fn window_border<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => COMPACT_BORDER,
        false => client_theme().follow_safe(state).window.border,
    })
}

/// Selector for the window gaps: the collapsed form stacks everything closely
/// together.
fn window_gaps<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => COMPACT_GAPS,
        false => client_theme().follow_safe(state).window.gaps,
    })
}

/// Selector for the title bar height: the collapsed form uses a smaller title.
fn window_title_height<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => COMPACT_TITLE_HEIGHT,
        false => client_theme().follow_safe(state).window.title_height,
    })
}

/// Selector for the window title: the minimalist HUD has no title text.
fn window_title<A>(compact_path: A) -> impl Selector<ClientState, String>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => String::new(),
        false => client_state()
            .localization()
            .character_overview_window_title()
            .follow_safe(state)
            .clone(),
    })
}

/// Selector for the gap between the title bar and the content: the minimalist
/// HUD has no title text and no title gap.
fn window_title_gap<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => 0.0,
        false => client_theme().follow_safe(state).window.title_gap,
    })
}

/// Selector for the window background color.
fn window_background_color<A>(compact_path: A) -> impl Selector<ClientState, Color>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => client_theme().follow_safe(state).tokens.color.surface_overlay,
        false => client_theme().follow_safe(state).window.background_color,
    })
}

/// Selector for the window corner diameter.
fn window_corner_diameter<A>(compact_path: A) -> impl Selector<ClientState, CornerDiameter>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => CornerDiameter::uniform(COMPACT_CORNER_DIAMETER),
        false => client_theme().follow_safe(state).window.corner_diameter,
    })
}

/// Selector for the minimum window height.
fn window_minimum_height<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => 0.0,
        false => client_theme().follow_safe(state).window.minimum_height,
    })
}

/// Selector for the maximum window height.
///
/// The collapsed HUD clamps a cached expanded height so the minimalist HUD can
/// never become tall again, while the expanded form keeps the theme limit.
fn window_maximum_height<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => COMPACT_MAXIMUM_HEIGHT,
        false => client_theme().follow_safe(state).window.maximum_height,
    })
}

/// Selector for the level line: `Lv 99 / Jlv 69`.
fn levels_display<A>(player_path: A) -> impl Selector<ClientState, String>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        let mut text = format!("Lv {} / Jlv {}", player.base_level, player.job_level);

        // Tell the player how many spirit spheres are active while they have
        // some (Dangerous Soul Collect/Zen fills all five, Critical
        // Explosion/Fury consumes them); the compact HUD overlay shows the same
        // count in-game.
        let spirit_spheres = state.own_spirit_orbs();
        if spirit_spheres > 0 {
            text.push_str(&format!("  Spheres {spirit_spheres}"));
        }

        text
    })
}

/// Progress of `current` out of `maximum`, clamped to `[0, 1]`.
///
/// Returns `None` if the maximum is unknown (zero), which is rendered as an
/// empty bar.
fn fraction(current: u64, maximum: u64) -> Option<f32> {
    match maximum {
        0 => None,
        maximum => Some((current as f64 / maximum as f64).clamp(0.0, 1.0) as f32),
    }
}

/// Progress in percent (0.0 - 100.0) converted to a fraction, clamped to
/// `[0, 1]`.
fn fraction_from_percent(percent: Option<f64>) -> Option<f32> {
    percent.map(|percent| (percent / 100.0).clamp(0.0, 1.0) as f32)
}

/// Selector for the health points progress.
fn health_points_fraction<A>(player_path: A) -> impl Selector<ClientState, Option<f32>>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        fraction(player.health_points() as u64, player.maximum_health_points() as u64)
    })
}

/// Selector for the spell points progress.
fn spell_points_fraction<A>(player_path: A) -> impl Selector<ClientState, Option<f32>>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        fraction(player.spell_points as u64, player.maximum_spell_points as u64)
    })
}

/// Selector for the base experience progress.
fn base_experience_fraction<A>(player_path: A) -> impl Selector<ClientState, Option<f32>>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        fraction_from_percent(player.base_experience_percent())
    })
}

/// Selector for the job experience progress.
fn job_experience_fraction<A>(player_path: A) -> impl Selector<ClientState, Option<f32>>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        fraction_from_percent(player.job_experience_percent())
    })
}

/// Selector for the fill color of the health bar.
///
/// The bar is green while the character is healthy and red when the character
/// is low on health, which matches the classic Ragnarok Online health bar.
fn health_fill_color<A>(player_path: A) -> impl Selector<ClientState, Color>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        let character_overview = &client_theme().follow_safe(state).character_overview;

        let low_health_color = character_overview.low_health_color;
        let health_color = character_overview.health_color;

        match fraction(player.health_points() as u64, player.maximum_health_points() as u64) {
            Some(fraction) if fraction <= LOW_HEALTH_FRACTION => low_health_color,
            _ => health_color,
        }
    })
}

/// Selector for the "current HP / maximum HP" text.
fn health_points_display<A>(player_path: A) -> impl Selector<ClientState, String>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        format!("{} / {}", player.health_points(), player.maximum_health_points())
    })
}

/// Selector for the "current SP / maximum SP" text.
fn spell_points_display<A>(player_path: A) -> impl Selector<ClientState, String>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);
        format!("{} / {}", player.spell_points, player.maximum_spell_points)
    })
}

/// Selector for the base experience progress in percent.
///
/// The server sends `SP_BASEEXP` and `SP_NEXTBASEEXP` shortly after entering
/// the map. Until then (or if the server does not send them at all) a dash is
/// shown instead of dividing by zero.
fn experience_display<A>(player_path: A) -> impl Selector<ClientState, String>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);

        match player.base_experience_percent() {
            Some(percent) => format!("{percent:.1} %"),
            None => "—".to_owned(),
        }
    })
}

/// Selector for the job experience progress in percent.
///
/// Like the base experience this is only known after the server sent
/// `SP_JOBEXP`/`SP_NEXTJOBEXP`.
fn job_experience_display<A>(player_path: A) -> impl Selector<ClientState, String>
where
    A: Path<ClientState, Player>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| {
        let player = player_path.follow_safe(state);

        match player.job_experience_percent() {
            Some(percent) => format!("{percent:.1} %"),
            None => "—".to_owned(),
        }
    })
}

impl<A, B, C> CustomWindow<ClientState> for CharacterOverviewWindow<A, B, C>
where
    A: Path<ClientState, String>,
    B: Path<ClientState, Player>,
    C: Path<ClientState, InterfaceSettings>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterOverview)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let player_path = self.player_path;
        let compact_path = self.interface_settings_path.character_overview_compact();
        let settings_path = self.interface_settings_path;

        // The collapsed form: the minimalist companion HUD. It intentionally has no
        // labels, values or title text — only the HP line, the SP line and the base/job
        // experience bars stacked into a single line, plus a round expand button.
        let compact = fragment! {
            gaps: COMPACT_GAPS,
            children: (
                RoundButton::new(
                    "+",
                    HUD_ROUND_BUTTON_SIZE,
                    client_state().localization().character_overview_expand_button_tooltip(),
                    CharacterOverviewCompactToggle {
                        value_path: compact_path,
                        settings_path,
                    },
                ),
                MiniBar::new(
                    health_points_fraction(player_path),
                    health_fill_color(player_path),
                    client_theme().character_overview().health_background_color(),
                    HUD_BAR_HEIGHT,
                ),
                MiniBar::new(
                    spell_points_fraction(player_path),
                    client_theme().character_overview().spell_point_color(),
                    client_theme().character_overview().spell_point_background_color(),
                    HUD_BAR_HEIGHT,
                ),
                StackedMiniBars::new(
                    base_experience_fraction(player_path),
                    client_theme().character_overview().experience_color(),
                    client_theme().character_overview().experience_background_color(),
                    job_experience_fraction(player_path),
                    client_theme().character_overview().job_experience_color(),
                    client_theme().character_overview().job_experience_background_color(),
                    HUD_STACK_HEIGHT,
                    HUD_STACK_GAP,
                ),
            ),
        };

        // The expanded form: header (name + level line) and the four labelled bars
        // on the left, the action buttons in a two-row grid on the right.
        let header = split! {
            children: (
                text! {
                    text: self.player_name_path,
                    color: korangar_interface::theme::theme().text().highlight_color(),
                    font_size: client_theme().tokens().typography().font_label(),
                    height: HEADER_FONT_SIZE + 3.0,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: levels_display(player_path),
                    color: client_theme().character_overview().value_color(),
                    font_size: client_theme().tokens().typography().font_label(),
                    height: HEADER_FONT_SIZE + 3.0,
                    horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 0.0 },
                    overflow_behavior: OverflowBehavior::Shrink,
                },
            ),
        };

        let left_column = fragment! {
            gaps: 4.0,
            children: (
                header,
                LabeledProgressBar::new(
                    client_state().localization().health_points_text(),
                    health_points_display(player_path),
                    health_points_fraction(player_path),
                    health_fill_color(player_path),
                    client_theme().character_overview().health_background_color(),
                    expanded_bar_style(),
                ),
                LabeledProgressBar::new(
                    client_state().localization().spell_points_text(),
                    spell_points_display(player_path),
                    spell_points_fraction(player_path),
                    client_theme().character_overview().spell_point_color(),
                    client_theme().character_overview().spell_point_background_color(),
                    expanded_bar_style(),
                ),
                LabeledProgressBar::new(
                    client_state().localization().experience_text(),
                    experience_display(player_path),
                    base_experience_fraction(player_path),
                    client_theme().character_overview().experience_color(),
                    client_theme().character_overview().experience_background_color(),
                    expanded_bar_style(),
                ),
                LabeledProgressBar::new(
                    client_state().localization().job_experience_text(),
                    job_experience_display(player_path),
                    job_experience_fraction(player_path),
                    client_theme().character_overview().job_experience_color(),
                    client_theme().character_overview().job_experience_background_color(),
                    expanded_bar_style(),
                ),
            ),
        };

        // The 3x3 action button grid plus the round collapse button above it.
        let action_buttons = fragment! {
            gaps: 3.0,
            children: (
                RoundButton::new(
                    "−",
                    ROUND_BUTTON_SIZE,
                    client_state().localization().character_overview_compact_button_tooltip(),
                    CharacterOverviewCompactToggle {
                        value_path: compact_path,
                        settings_path,
                    },
                ),
                split! {
                    gaps: 3.0,
                    children: (
                        button! {
                            text: client_state().localization().inventory_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleInventoryWindow,
                        },
                        button! {
                            text: client_state().localization().equipment_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleEquipmentWindow,
                        },
                        button! {
                            text: client_state().localization().skill_tree_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleSkillTreeWindow,
                        },
                    ),
                },
                split! {
                    gaps: 3.0,
                    children: (
                        button! {
                            text: client_state().localization().stats_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleStatsWindow,
                        },
                        button! {
                            text: client_state().localization().friend_list_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleFriendListWindow,
                        },
                        button! {
                            text: client_state().localization().menu_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleMenuWindow,
                        },
                    ),
                },
                split! {
                    gaps: 3.0,
                    children: (
                        button! {
                            text: client_state().localization().big_map_window_title(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleBigMapWindow,
                        },
                        button! {
                            // Party lives where the chat shortcut used to be; the
                            // chat window itself stays reachable via its hotkey
                            // (Alt+C) and is still opened on login/whisper.
                            text: client_state().localization().party_window_title(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::TogglePartyWindow,
                        },
                        button! {
                            text: client_state().localization().game_settings_button_text(),
                            height: ACTION_BUTTON_HEIGHT,
                            font_size: client_theme().tokens().typography().font_caption(),
                            overflow_behavior: OverflowBehavior::Shrink,
                            event: InputEvent::ToggleGameSettingsWindow,
                        },
                    ),
                },
            ),
        };

        let expanded = split! {
            gaps: 6.0,
            children: (
                left_column,
                action_buttons,
            ),
        };

        window! {
            title: window_title(compact_path),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            title_height: window_title_height(compact_path),
            title_gap: window_title_gap(compact_path),
            minimum_width: window_width(compact_path),
            maximum_width: window_width(compact_path),
            minimum_height: window_minimum_height(compact_path),
            maximum_height: window_maximum_height(compact_path),
            border: window_border(compact_path),
            gaps: window_gaps(compact_path),
            // The character overview is toggled from the menu / character
            // selection and collapsed with the round button, like the minimap it
            // does not get an X close button.
            closable: false,
            background_color: window_background_color(compact_path),
            corner_diameter: window_corner_diameter(compact_path),
            elements: either! {
                selector: compact_path,
                on_true: compact,
                on_false: expanded,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{fraction, fraction_from_percent};

    #[test]
    fn fraction_of_known_values() {
        assert_eq!(fraction(50, 100), Some(0.5));
        assert_eq!(fraction(0, 100), Some(0.0));
        assert_eq!(fraction(100, 100), Some(1.0));
    }

    #[test]
    fn fraction_of_unknown_maximum_is_none() {
        assert_eq!(fraction(10, 0), None);
    }

    #[test]
    fn fraction_is_clamped() {
        assert_eq!(fraction(200, 100), Some(1.0));
    }

    #[test]
    fn percent_is_converted_and_unknown_is_none() {
        assert_eq!(fraction_from_percent(Some(50.0)), Some(0.5));
        assert_eq!(fraction_from_percent(Some(0.0)), Some(0.0));
        assert_eq!(fraction_from_percent(Some(100.0)), Some(1.0));
        assert_eq!(fraction_from_percent(None), None);
    }
}
