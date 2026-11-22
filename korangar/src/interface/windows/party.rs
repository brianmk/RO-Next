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
use rust_state::{Path, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{FontSize, OverflowBehavior};
use crate::state::party::{Party, PartyInvitePrompt, PartyPathExt};
use crate::state::theme::{InterfaceThemePathExt, InterfaceThemeType};
use crate::state::{ClientState, ClientStatePathExt, client_state, client_theme};

const MAXIMUM_NAME_LENGTH: usize = 24;

const TITLE_HEIGHT: f32 = 20.0;
const ROW_HEIGHT: f32 = 26.0;
const BAR_HEIGHT: f32 = 7.0;
const ROW_TEXT_HEIGHT: f32 = 15.0;
const FONT_SIZE: f32 = 14.0;

const HEALTH_BAR_FILL_COLOR: Color = Color::rgb_u8(84, 196, 104);
const HEALTH_BAR_BACKGROUND_COLOR: Color = Color::rgb_u8(36, 36, 36);
const TEXT_COLOR: Color = Color::rgb_u8(230, 230, 230);
const SECONDARY_TEXT_COLOR: Color = Color::rgb_u8(170, 170, 170);

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

        self.title = match party.is_in_party() {
            true if party.name.is_empty() => "Party".to_owned(),
            true => format!("Party: {}", party.name),
            false => "You are not in a party.".to_owned(),
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
        layout.add_text(
            layout_info.title_area,
            &self.title,
            FontSize(FONT_SIZE),
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

            layout.add_text(
                row.name_area,
                line,
                FontSize(FONT_SIZE),
                TEXT_COLOR,
                title_color,
                HorizontalAlignment::Left { offset: 2.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            layout.add_text(
                row.name_area,
                health_text,
                FontSize(FONT_SIZE - 1.0),
                SECONDARY_TEXT_COLOR,
                title_color,
                HorizontalAlignment::Right { offset: 2.0, border: 2.0 },
                VerticalAlignment::Center { offset: 0.0 },
                OverflowBehavior::Shrink,
            );

            draw_bar(layout, row.bar_area, fraction);
        }
    }
}

/// Draws the rounded track and fill of a party member HP bar.
fn draw_bar(layout: &mut WindowLayout<'_, ClientState>, area: Area, fraction: f32) {
    if area.width <= 0.0 || area.height <= 0.0 {
        return;
    }

    let corner_diameter = CornerDiameter::uniform(area.height / 2.0);

    layout.add_rectangle(
        area,
        corner_diameter,
        HEALTH_BAR_BACKGROUND_COLOR,
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
        HEALTH_BAR_FILL_COLOR,
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
            Some(invite) => format!("You are invited to join the party \"{}\".", invite.party_name),
            None => "You have no open party invitation.".to_owned(),
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

        layout.add_text(
            *layout_info,
            &self.text,
            FontSize(FONT_SIZE),
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
            title: "Party",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 360.0,
            maximum_width: 360.0,
            elements: (
                PartyRoster::new(client_state().party()),
                text! {
                    text: "Create a party",
                },
                text_box! {
                    ghost_text: "Party name",
                    state: create_input_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(create_input_path, create_action),
                    focus_id: CreatePartyName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! {
                    text: "Create party",
                    disabled: create_disabled,
                    disabled_tooltip: "Enter a party name first",
                    event: create_action,
                },
                text! {
                    text: "Invite a character",
                },
                text_box! {
                    ghost_text: "Character name",
                    state: invite_input_path,
                    input_handler: DefaultHandler::<_, _, MAXIMUM_NAME_LENGTH>::new(invite_input_path, invite_action),
                    focus_id: InvitePartyName,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                button! {
                    text: "Invite",
                    disabled: invite_disabled,
                    disabled_tooltip: "You have to be in a party to invite characters",
                    event: invite_action,
                },
                button! {
                    text: "Leave party",
                    disabled: leave_disabled,
                    disabled_tooltip: "You are not in a party",
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
            title: "Party invitation",
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
                            text: "Accept",
                            event: accept_action,
                        },
                        button! {
                            text: "Reject",
                            event: reject_action,
                        },
                    ),
                },
            ),
        }
    }
}
