use korangar_interface::element::Element;
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::components::progress_bar::{LOW_HEALTH_FRACTION, ProgressBar};
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::{CharacterOverviewThemePathExt, InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};
use crate::world::{Player, PlayerPathExt};

/// Width of the character overview window in its normal (expanded) form.
const NORMAL_WINDOW_WIDTH: f32 = 300.0;
/// Width of the character overview window in its compact form.
///
/// The compact form is meant to be a small always-on HUD element, so it is
/// kept small enough to fit into a corner without covering the game.
const COMPACT_WINDOW_WIDTH: f32 = 150.0;
/// Window border of the compact form, in interface pixels.
const COMPACT_BORDER: f32 = 6.0;
/// Gaps between the elements of the compact form, in interface pixels.
const COMPACT_GAPS: f32 = 3.0;
/// Font size of the labels and values in the compact form.
const COMPACT_FONT_SIZE: f32 = 10.0;
/// Bar height of the compact form.
const COMPACT_BAR_HEIGHT: f32 = 5.0;
/// Font size of the labels and values in the expanded form.
const EXPANDED_FONT_SIZE: f32 = 12.0;
/// Bar height of the expanded form.
const EXPANDED_BAR_HEIGHT: f32 = 8.0;
/// Gap between a label/value row and its bar.
const BAR_GAP: f32 = 1.0;
/// Gap between two stat blocks (label + bar).
const STAT_GAP: f32 = 4.0;

/// The character overview window.
///
/// The window has two forms:
/// * the normal form shows the character name, the base/job level, HP, SP, base
///   EXP and job EXP as progress bars, and the buttons to open the other
///   windows;
/// * the compact form only shows the HP, SP and base EXP bars in a much smaller
///   window to save screen space. The mode is persisted in the interface
///   settings.
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

/// Click handler for the "compact"/"expand" toggle of the character overview.
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

/// A small label/value row with a progress bar below it.
///
/// This is the building block of the character overview bars: the row shows the
/// name of the stat on the left and its value (current/maximum or percent) on
/// the right, the bar below shows the same progress visually.
#[allow(clippy::too_many_arguments)]
fn bar_block<L, V, F, C, B>(
    label: L,
    value: V,
    fraction: F,
    fill_color: C,
    background_color: B,
    font_size: f32,
    bar_height: f32,
) -> impl Element<ClientState>
where
    L: Selector<ClientState, String>,
    V: Selector<ClientState, String>,
    F: Selector<ClientState, Option<f32>>,
    C: Selector<ClientState, Color>,
    B: Selector<ClientState, Color>,
{
    use korangar_interface::prelude::*;

    fragment! {
        gaps: BAR_GAP,
        children: (
            split! {
                children: (
                    text! {
                        text: label,
                        color: client_theme().character_overview().label_color(),
                        font_size: FontSize(font_size),
                        height: font_size + 2.0,
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                    text! {
                        text: value,
                        color: client_theme().character_overview().value_color(),
                        font_size: FontSize(font_size),
                        height: font_size + 2.0,
                        horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 2.0 },
                        overflow_behavior: OverflowBehavior::Shrink,
                    },
                ),
            },
            ProgressBar::new(fraction, fill_color, background_color, bar_height),
        ),
    }
}

/// Selector for the window width, depending on whether the compact mode is
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

/// Selector for the window border: the compact form uses a much smaller border
/// so that almost the whole width is available for the bars.
fn window_border<A>(compact_path: A) -> impl Selector<ClientState, f32>
where
    A: Path<ClientState, bool>,
{
    korangar_interface::prelude::ComputedSelector::new_default(move |state: &ClientState| match *compact_path.follow_safe(state) {
        true => COMPACT_BORDER,
        false => client_theme().follow_safe(state).window.border,
    })
}

/// Selector for the window gaps: the compact form stacks everything closely
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

        // The three bars of the compact form, with tiny labels and values.
        let compact = fragment! {
            gaps: STAT_GAP,
            children: (
                bar_block(
                    client_state().localization().health_points_text(),
                    health_points_display(player_path),
                    health_points_fraction(player_path),
                    health_fill_color(player_path),
                    client_theme().character_overview().health_background_color(),
                    COMPACT_FONT_SIZE,
                    COMPACT_BAR_HEIGHT,
                ),
                bar_block(
                    client_state().localization().spell_points_text(),
                    spell_points_display(player_path),
                    spell_points_fraction(player_path),
                    client_theme().character_overview().spell_point_color(),
                    client_theme().character_overview().spell_point_background_color(),
                    COMPACT_FONT_SIZE,
                    COMPACT_BAR_HEIGHT,
                ),
                bar_block(
                    client_state().localization().experience_text(),
                    experience_display(player_path),
                    base_experience_fraction(player_path),
                    client_theme().character_overview().experience_color(),
                    client_theme().character_overview().experience_background_color(),
                    COMPACT_FONT_SIZE,
                    COMPACT_BAR_HEIGHT,
                ),
                button! {
                    text: client_state().localization().character_overview_expand_button_text(),
                    tooltip: client_state().localization().character_overview_expand_button_tooltip(),
                    height: 18.0,
                    font_size: FontSize(COMPACT_FONT_SIZE),
                    event: CharacterOverviewCompactToggle {
                        value_path: compact_path,
                        settings_path: self.interface_settings_path,
                    },
                },
            ),
        };

        let expanded = (
            fragment! {
                gaps: 4.0,
                children: (
                    split! {
                        children: (
                            text! {
                                text: client_state().localization().name_text(),
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            text! {
                                text: self.player_name_path,
                                color: theme().text().highlight_color(),
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                        ),
                    },
                    split! {
                        children: (
                            text! {
                                text: client_state().localization().base_level_text(),
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            text! {
                                text: PartialEqDisplaySelector::new(player_path.base_level()),
                                color: client_theme().character_overview().value_color(),
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                        ),
                    },
                    split! {
                        children: (
                            text! {
                                text: client_state().localization().job_level_text(),
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            text! {
                                text: PartialEqDisplaySelector::new(player_path.job_level()),
                                color: client_theme().character_overview().value_color(),
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                        ),
                    },
                ),
            },
            fragment! {
                gaps: STAT_GAP,
                children: (
                    bar_block(
                        client_state().localization().health_points_text(),
                        health_points_display(player_path),
                        health_points_fraction(player_path),
                        health_fill_color(player_path),
                        client_theme().character_overview().health_background_color(),
                        EXPANDED_FONT_SIZE,
                        EXPANDED_BAR_HEIGHT,
                    ),
                    bar_block(
                        client_state().localization().spell_points_text(),
                        spell_points_display(player_path),
                        spell_points_fraction(player_path),
                        client_theme().character_overview().spell_point_color(),
                        client_theme().character_overview().spell_point_background_color(),
                        EXPANDED_FONT_SIZE,
                        EXPANDED_BAR_HEIGHT,
                    ),
                    bar_block(
                        client_state().localization().experience_text(),
                        experience_display(player_path),
                        base_experience_fraction(player_path),
                        client_theme().character_overview().experience_color(),
                        client_theme().character_overview().experience_background_color(),
                        EXPANDED_FONT_SIZE,
                        EXPANDED_BAR_HEIGHT,
                    ),
                    bar_block(
                        client_state().localization().job_experience_text(),
                        job_experience_display(player_path),
                        job_experience_fraction(player_path),
                        client_theme().character_overview().job_experience_color(),
                        client_theme().character_overview().job_experience_background_color(),
                        EXPANDED_FONT_SIZE,
                        EXPANDED_BAR_HEIGHT,
                    ),
                ),
            },
            button! {
                text: client_state().localization().inventory_button_text(),
                event: InputEvent::ToggleInventoryWindow,
            },
            button! {
                text: client_state().localization().equipment_button_text(),
                event: InputEvent::ToggleEquipmentWindow,
            },
            button! {
                text: client_state().localization().skill_tree_button_text(),
                event: InputEvent::ToggleSkillTreeWindow,
            },
            button! {
                text: client_state().localization().stats_button_text(),
                event: InputEvent::ToggleStatsWindow,
            },
            button! {
                text: client_state().localization().friend_list_button_text(),
                event: InputEvent::ToggleFriendListWindow,
            },
            button! {
                text: client_state().localization().menu_button_text(),
                event: InputEvent::ToggleMenuWindow,
            },
            button! {
                text: client_state().localization().character_overview_compact_button_text(),
                tooltip: client_state().localization().character_overview_compact_button_tooltip(),
                event: CharacterOverviewCompactToggle {
                    value_path: compact_path,
                    settings_path: self.interface_settings_path,
                },
            },
        );

        window! {
            title: client_state().localization().character_overview_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            minimum_width: window_width(compact_path),
            maximum_width: window_width(compact_path),
            border: window_border(compact_path),
            gaps: window_gaps(compact_path),
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
