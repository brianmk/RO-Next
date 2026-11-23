//! Party window and party invitation prompt for the 2019 rAthena party system.
//!
//! The roster is a fixed 12 slot list (the 2019 server limit) fed from
//! `state::party::Party`. Every slot shows the member name, leader/offline/dead
//! markers and the HP bar known from `ZC_NOTIFY_HP_TO_GROUPM` (0x080E).
//!
//! The roster is rendered by a custom element instead of a macro generated list
//! because the party size is dynamic and each row needs a text + bar layout.

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::element::Element;
use korangar_interface::element::store::{ElementStore, ElementStoreMut};
use korangar_interface::layout::area::Area;
use korangar_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use korangar_interface::prelude::{HorizontalAlignment, VerticalAlignment};
use korangar_interface::window::{CustomWindow, Window, WindowThemePathExt};
use ragnarok_packets::NAME_LENGTH;
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::party::{Party, PartyInvitePrompt, PartyPathExt};
use crate::state::theme::{
    ColorTokensPathExt, DesignTokensPathExt, InterfaceThemePathExt, InterfaceThemeType, PartyThemePathExt, TypographyTokensPathExt,
};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

const TITLE_HEIGHT: f32 = 20.0;
const ROW_HEIGHT: f32 = 26.0;
const BAR_HEIGHT: f32 = 7.0;
const ROW_TEXT_HEIGHT: f32 = 15.0;

/// Row of the roster that was laid out by [`PartyRoster::create_layout_info`].
pub struct RosterRow {
    pub name_area: Area,
    pub bar_area: Area,
}

pub struct RosterLayout {
    pub title_area: Area,
    pub rows: Vec<RosterRow>,
}

/// Custom element that renders the party title and one text + HP bar row per
/// member.
///
/// The number of rows is dynamic (up to 12), so this cannot be built from the
/// macro generated components.
pub struct PartyRoster<A> {
    party_path: A,
    title: String,
    lines: Vec<String>,
    health_texts: Vec<String>,
    health_fractions: Vec<Option<f32>>,
    offline: Vec<bool>,
}

impl<A> PartyRoster<A>
where
    A: Path<ClientState, Party>,
{
    pub fn new(party_path: A) -> Self {
        Self {
            party_path,
            title: String::new(),
            lines: Vec::new(),
            health_texts: Vec::new(),
            health_fractions: Vec::new(),
            offline: Vec::new(),
        }
    }
}

impl<A> Element<ClientState> for PartyRoster<A>
where
    A: Path<ClientState, Party>,
{
    type LayoutInfo = RosterLayout;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        let party = state.follow(self.party_path);

        let party_title = state.get(&client_state().localization().party_window_title()).clone();
        self.title = match party.is_in_party() {
            true if party.name.is_empty() => party_title,
            true => format!("{party_title}: {}", party.name),
            false => state.get(&client_state().localization().not_in_party_text()).clone(),
        };

        self.lines = party.members.iter().map(|member| member.display_name()).collect();
        self.health_texts = party
            .members
            .iter()
            .map(|member| match (member.health_points, member.maximum_health_points) {
                (Some(health_points), Some(maximum_health_points)) => format!("{health_points} / {maximum_health_points}"),
                _ => String::new(),
            })
            .collect();
        self.health_fractions = party.members.iter().map(|member| member.health_fraction()).collect();
        self.offline = party.members.iter().map(|member| !member.online).collect();

        let member_count = party.members.len();
        let height = TITLE_HEIGHT + member_count as f32 * ROW_HEIGHT;

        with_single_resolver(resolvers, |resolver| {
            let area = resolver.with_height(height);

            let title_area = Area {
                left: area.left,
                top: area.top,
                width: area.width,
                height: TITLE_HEIGHT,
            };

            let rows = (0..member_count)
                .map(|index| {
                    let top = area.top + TITLE_HEIGHT + index as f32 * ROW_HEIGHT;

                    RosterRow {
                        name_area: Area {
                            left: area.left,
                            top,
                            width: area.width,
                            height: ROW_TEXT_HEIGHT,
                        },
                        bar_area: Area {
                            left: area.left,
                            top: top + ROW_TEXT_HEIGHT + 2.0,
                            width: area.width,
                            height: BAR_HEIGHT,
                        },
                    }
                })
                .collect();

            RosterLayout { title_area, rows }
        })
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let title_color = *state.get(&client_theme().window().title_color());
        let party_theme = client_theme().party();
        let member_name_color = *state.get(&party_theme.member_name_color());
        let offline_member_name_color = *state.get(&party_theme.offline_member_name_color());
        let health_text_color = *state.get(&party_theme.health_text_color());
        let bar_background_color = *state.get(&party_theme.health_bar_background_color());
        let typography = client_theme().tokens().typography();
        let color_tokens = client_theme().tokens().color();
        let row_font_size = *state.get(&typography.font_body_lg());
        let health_font_size = *state.get(&typography.font_button());
        let health_fill_color = *state.get(&color_tokens.party_health_fill());
        layout.add_text(
            layout_info.title_area,
            &self.title,
            row_font_size,
            title_color,
            title_color,
            HorizontalAlignment::Left { offset: 2.0, border: 2.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::Shrink,
        );

        for (index, row) in layout_info.rows.iter().enumerate() {
            let line = self.lines.get(index).map(String::as_str).unwrap_or_default();
            let health_text = self.health_texts.get(index).map(String::as_str).unwrap_or_default();
            let fraction = self.health_fractions.get(index).copied().flatten().unwrap_or(0.0);
            // Offline members use a darker gray in the classic (light) theme so the
            // names stay readable; online members use the theme text color.
            let name_color = match self.offline.get(index).copied().unwrap_or(false) {
                true => offline_member_name_color,
                false => member_name_color,
            };

            layout.add_text(
                row.name_area,
                line,
                row_font_size,
                name_color,
                title_color,
                HorizontalAlignment::Left { offset: 2.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            layout.add_text(
                row.name_area,
                health_text,
                health_font_size,
                health_text_color,
                title_color,
                HorizontalAlignment::Right { offset: 2.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            draw_bar(layout, row.bar_area, fraction, bar_background_color, health_fill_color);
        }
    }
}

/// Draws the rounded track and fill of a party member HP bar.
fn draw_bar(layout: &mut WindowLayout<'_, ClientState>, area: Area, fraction: f32, background_color: Color, fill_color: Color) {
    if area.width <= 0.0 || area.height <= 0.0 {
        return;
    }

    let corner_diameter = CornerDiameter::uniform(area.height / 2.0);

    layout.add_rectangle(
        area,
        corner_diameter,
        background_color,
        Color::TRANSPARENT,
        ShadowPadding::uniform(0.0),
    );

    let fraction = fraction.clamp(0.0, 1.0);

    if fraction <= 0.0 {
        return;
    }

    layout.add_rectangle(
        Area {
            left: area.left,
            top: area.top,
            width: (area.width * fraction).max(area.height.min(area.width)),
            height: area.height,
        },
        corner_diameter,
        fill_color,
        Color::TRANSPARENT,
        ShadowPadding::uniform(0.0),
    );
}

/// Custom element that renders the text of the party invitation prompt.
pub struct PartyInviteText<A> {
    invite_path: A,
    text: String,
}

impl<A> PartyInviteText<A>
where
    A: Path<ClientState, Option<PartyInvitePrompt>>,
{
    pub fn new(invite_path: A) -> Self {
        Self {
            invite_path,
            text: String::new(),
        }
    }
}

impl<A> Element<ClientState> for PartyInviteText<A>
where
    A: Path<ClientState, Option<PartyInvitePrompt>>,
{
    type LayoutInfo = Area;

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        _: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        self.text = match state.follow(self.invite_path) {
            Some(invite) => state
                .get(&client_state().localization().ui().party_invitation_text())
                .replace("{}", &invite.party_name),
            None => state.get(&client_state().localization().ui().party_no_invitation_text()).clone(),
        };

        with_single_resolver(resolvers, |resolver| resolver.with_height(34.0))
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        _: ElementStore<'a>,
        layout_info: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        let color = *state.get(&client_theme().window().title_color());
        let row_font_size = *state.get(&client_theme().tokens().typography().font_body_lg());

        layout.add_text(
            *layout_info,
            &self.text,
            row_font_size,
            color,
            color,
            HorizontalAlignment::Left { offset: 2.0, border: 2.0 },
            VerticalAlignment::Center { offset: 0.0 },
            OverflowBehavior::LineBreak,
        );
    }
}

pub struct PartyWindow;

impl CustomWindow<ClientState> for PartyWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Party)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let create_input_path = client_state().party().create_name_input();
        let invite_input_path = client_state().party().invite_name_input();

        let create_disabled = ComputedSelector::<_, bool>::new_default(move |state: &ClientState| {
            let Some(party) = client_state().party().follow(state) else {
                return true;
            };
            party.is_in_party() || party.create_name_input.trim().is_empty()
        });

        let invite_disabled = ComputedSelector::<_, bool>::new_default(move |state: &ClientState| {
            let Some(party) = client_state().party().follow(state) else {
                return true;
            };
            !party.is_in_party() || party.invite_name_input.trim().is_empty()
        });

        let leave_disabled = ComputedSelector::<_, bool>::new_default(move |state: &ClientState| {
            client_state().party().follow(state).is_none_or(|party| !party.is_in_party())
        });

        let create_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&create_input_path).trim().to_owned();

            if !name.is_empty() {
                queue.queue(InputEvent::CreateParty { name });
            }
        };

        let invite_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&invite_input_path).trim().to_owned();

            if !name.is_empty() {
                queue.queue(InputEvent::InviteToParty { name });
            }
        };

        let leave_action = move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            queue.queue(InputEvent::LeaveParty);
        };

        struct CreatePartyName;
        struct InvitePartyName;

        window! {
            title: client_state().localization().party_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 360.0,
            maximum_width: 360.0,
            elements: (
                PartyRoster::new(client_state().party()),
                text! {
                    text: client_state().localization().create_party_text(),
                },
                text_box! {
                    ghost_text: client_state().localization().party_name_text(),
                    state: create_input_path,
                    input_handler: DefaultHandler::<_, _, NAME_LENGTH>::new(create_input_path, create_action),
                    focus_id: CreatePartyName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! {
                    text: client_state().localization().create_party_button_text(),
                    disabled: create_disabled,
                    disabled_tooltip: client_state().localization().ui().party_enter_name_tooltip(),
                    event: create_action,
                },
                text! {
                    text: client_state().localization().invite_character_text(),
                },
                text_box! {
                    ghost_text: client_state().localization().character_name_text(),
                    state: invite_input_path,
                    input_handler: DefaultHandler::<_, _, NAME_LENGTH>::new(invite_input_path, invite_action),
                    focus_id: InvitePartyName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! {
                    text: client_state().localization().invite_button_text(),
                    disabled: invite_disabled,
                    disabled_tooltip: client_state().localization().ui().party_must_be_in_party_to_invite_tooltip(),
                    event: invite_action,
                },
                button! {
                    text: client_state().localization().leave_party_button_text(),
                    disabled: leave_disabled,
                    disabled_tooltip: client_state().localization().ui().party_not_in_party_tooltip(),
                    event: leave_action,
                },
            ),
        }
    }
}

/// Prompt that asks the player to accept or reject a party invitation.
pub struct PartyInviteWindow<A> {
    invite_path: A,
}

impl<A> PartyInviteWindow<A> {
    pub fn new(invite_path: A) -> Self {
        Self { invite_path }
    }
}

impl<A> CustomWindow<ClientState> for PartyInviteWindow<A>
where
    A: Path<ClientState, Option<PartyInvitePrompt>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::PartyInvite)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let invite_path = self.invite_path;

        let accept_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            if let Some(invite) = state.get(&invite_path) {
                queue.queue(InputEvent::AcceptPartyInvite { party_id: invite.party_id });
            }
        };

        let reject_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            if let Some(invite) = state.get(&invite_path) {
                queue.queue(InputEvent::RejectPartyInvite { party_id: invite.party_id });
            }
        };

        window! {
            title: client_state().localization().party_invitation_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 380.0,
            maximum_width: 380.0,
            elements: (
                PartyInviteText::new(invite_path),
                split! {
                    gaps: 8.0,
                    children: (
                        button! {
                            text: client_state().localization().accept_button_text(),
                            event: accept_action,
                        },
                        button! {
                            text: client_state().localization().reject_button_text(),
                            event: reject_action,
                        },
                    ),
                },
            ),
        }
    }
}
