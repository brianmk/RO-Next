use korangar_interface::element::Element;
use korangar_interface::event::{ClickHandler, EventQueue};
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::settings::{InterfaceSettings, InterfaceSettingsPathExt};
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::{Player, PlayerPathExt};

/// Width of the character overview window in its normal (expanded) form.
const NORMAL_WINDOW_WIDTH: f32 = 300.0;
/// Width of the character overview window in its compact form.
const COMPACT_WINDOW_WIDTH: f32 = 170.0;
/// Color of the character name in the overview.
const NAME_COLOR: Color = Color::rgb_u8(255, 144, 13);
/// Color of the values shown in the overview (levels, HP, SP, EXP).
const VALUE_COLOR: Color = Color::rgb_u8(13, 231, 255);

/// The character overview window.
///
/// The window has two forms:
/// * the normal form shows the character name, the base/job level, the current
///   HP, SP and base EXP progress, and the buttons to open the other windows;
/// * the compact form only shows HP, SP and base EXP progress to save screen
///   space. The mode is persisted in the interface settings.
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

/// A row of the character overview with a label on the left and a value on the
/// right.
fn stat_row<L, V>(label: L, value: V) -> impl Element<ClientState>
where
    L: Selector<ClientState, String>,
    V: Selector<ClientState, String>,
{
    use korangar_interface::prelude::*;

    split! {
        children: (
            text! {
                text: label,
                overflow_behavior: OverflowBehavior::Shrink,
            },
            text! {
                text: value,
                color: VALUE_COLOR,
                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                overflow_behavior: OverflowBehavior::Shrink,
            },
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

        let compact = fragment! {
            gaps: 4.0,
            children: (
                stat_row(
                    client_state().localization().health_points_text(),
                    health_points_display(player_path),
                ),
                stat_row(
                    client_state().localization().spell_points_text(),
                    spell_points_display(player_path),
                ),
                stat_row(
                    client_state().localization().experience_text(),
                    experience_display(player_path),
                ),
                button! {
                    text: client_state().localization().character_overview_expand_button_text(),
                    tooltip: client_state().localization().character_overview_expand_button_tooltip(),
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
                                color: NAME_COLOR,
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
                                color: VALUE_COLOR,
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
                                color: VALUE_COLOR,
                                horizontal_alignment: HorizontalAlignment::Right { offset: 0.0, border: 3.0 },
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                        ),
                    },
                    stat_row(
                        client_state().localization().health_points_text(),
                        health_points_display(player_path),
                    ),
                    stat_row(
                        client_state().localization().spell_points_text(),
                        spell_points_display(player_path),
                    ),
                    stat_row(
                        client_state().localization().experience_text(),
                        experience_display(player_path),
                    ),
                    stat_row(
                        client_state().localization().job_experience_text(),
                        job_experience_display(player_path),
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
            elements: either! {
                selector: compact_path,
                on_true: compact,
                on_false: expanded,
            },
        }
    }
}
