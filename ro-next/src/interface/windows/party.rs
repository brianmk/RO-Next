//! Party window and party invitation prompt for the 2019 rAthena party system.
//!
//! The window has two sections:
//!
//! * the roster: a fixed 12 slot list (the 2019 server limit) fed from
//!   `state::party::Party`. Every slot shows the member name,
//!   leader/offline/dead markers and the HP bar known from
//!   `ZC_NOTIFY_HP_TO_GROUPM` (0x080E), plus the party settings (EXP share
//!   `CZ_GROUPINFO_CHANGE_V2` 0x07D7 and leader change `CZ_CHANGE_GROUP_MASTER`
//!   0x07DA), and
//! * the party-booking board ("party search" / LFG): register/search/update/
//!   delete advertisements (0x0802/0x0804/0x0806/0x0808) and show the server
//!   results (0x0803/0x0805/0x0807/0x0809/0x080A/0x080B). Incoming join
//!   requests (0x0AE7) and the leader reply (0x0AF8) are shown for servers with
//!   `PACKETVER >= 20191204`; the CondaRO 20190605 server does not send them.
//!
//! The roster is rendered by a custom element instead of a macro generated list
//! because the party size is dynamic and each row needs a text + bar layout.

use ro_next_interface::components::text_box::DefaultHandler;
use ro_next_interface::element::store::{ElementStore, ElementStoreMut};
use ro_next_interface::element::{Element, ElementBox};
use ro_next_interface::layout::area::Area;
use ro_next_interface::layout::{Resolvers, WindowLayout, with_single_resolver};
use ro_next_interface::prelude::{ComputedSelector, HorizontalAlignment, VerticalAlignment};
use ro_next_interface::window::{CustomWindow, Window, WindowThemePathExt};
use ragnarok_packets::NAME_LENGTH;
use rust_state::{Path, PathExt, Selector, State};

use crate::graphics::{Color, CornerDiameter, ShadowPadding};
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::party::{Party, PartyInvitePrompt, PartyPathExt};
use crate::state::party_booking::PartyBookingPathExt;
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

/// Dynamic list of party-booking advertisements.
///
/// One row per search result with a "Whisper leader" button and (opt-in) a
/// "Request join" button. The rows are rebuilt whenever the result set changes,
/// not on every layout pass.
struct PartyBookingResultList {
    elements: Vec<ElementBox<ClientState>>,
    built_indices: Vec<u32>,
}

impl PartyBookingResultList {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
            built_indices: Vec::new(),
        }
    }
}

impl Element<ClientState> for PartyBookingResultList {
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use ro_next_interface::prelude::*;

            let booking = state.follow(client_state().party_booking());
            let indices: Vec<u32> = booking.results.iter().map(|entry| entry.index).collect();

            if self.built_indices != indices {
                self.elements.clear();
                self.built_indices = indices.clone();

                for index in indices {
                    let display_text = booking_row_text(index);
                    let whisper_index = index;
                    let whisper_name = state
                        .get(&client_state().party_booking())
                        .result(index)
                        .map(|entry| entry.character_name.clone())
                        .unwrap_or_default();
                    self.elements.push(ErasedElement::new(collapsible! {
                        text: display_text,
                        initially_expanded: true,
                        children: split! {
                            gaps: 8.0,
                            children: (
                                button! {
                                    text: "Whisper leader",
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let name = state
                                            .get(&client_state().party_booking())
                                            .result(whisper_index)
                                            .map(|entry| entry.character_name.clone())
                                            .unwrap_or_else(|| whisper_name.clone());

                                        if !name.is_empty() {
                                            queue.queue(InputEvent::WhisperChat { name });
                                        }
                                    },
                                },
                                button! {
                                    text: "Request join",
                                    disabled: ComputedSelector::<_, bool>::new_default(move |state: &ClientState| {
                                        !client_state().party_booking().follow_safe(state).join_requests_enabled
                                    }),
                                    disabled_tooltip: "The booking join request (0x0AE6) is only parsed by servers with PACKETVER >= 20191204 (CondaRO 20190605 does not support it). Enable it on the server-support line first.",
                                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        queue.queue(InputEvent::JoinPartyBooking { index });
                                    },
                                },
                            ),
                        },
                    }));
                }
            }

            self.elements
                .iter_mut()
                .zip(self.built_indices.iter())
                .for_each(|(element, index)| {
                    element.create_layout_info(state, store.child_store(1_000_000 + u64::from(*index)), resolver);
                });
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements.iter().zip(self.built_indices.iter()).for_each(|(element, index)| {
            element.lay_out(state, store.child_store(1_000_000 + u64::from(*index)), &(), layout);
        });
    }
}

/// Dynamic list of incoming party-booking join requests.
struct PartyBookingJoinRequestList {
    elements: Vec<ElementBox<ClientState>>,
    built_characters: Vec<u32>,
}

impl PartyBookingJoinRequestList {
    fn new() -> Self {
        Self {
            elements: Vec::new(),
            built_characters: Vec::new(),
        }
    }
}

impl Element<ClientState> for PartyBookingJoinRequestList {
    type LayoutInfo = ();

    fn create_layout_info(
        &mut self,
        state: &State<ClientState>,
        mut store: ElementStoreMut,
        resolvers: &mut dyn Resolvers<ClientState>,
    ) -> Self::LayoutInfo {
        with_single_resolver(resolvers, |resolver| {
            use ro_next_interface::prelude::*;

            let booking = state.follow(client_state().party_booking());
            let character_ids: Vec<u32> = booking.join_requests.iter().map(|request| request.character_id.0).collect();

            if self.built_characters != character_ids {
                self.elements.clear();
                self.built_characters = character_ids.clone();

                for character_id in character_ids {
                    let display_text = join_request_row_text(character_id);

                    self.elements.push(ErasedElement::new(collapsible! {
                        text: display_text,
                        initially_expanded: true,
                        children: split! {
                            gaps: 8.0,
                            children: (
                                button! {
                                    text: "Accept",
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let request = state
                                            .get(&client_state().party_booking())
                                            .join_requests
                                            .iter()
                                            .find(|request| request.character_id.0 == character_id)
                                            .cloned();

                                        if let Some(request) = request {
                                            queue.queue(InputEvent::ReplyPartyBookingJoin {
                                                character_id: request.character_id,
                                                account_id: request.account_id,
                                                accept: true,
                                            });
                                        }
                                    },
                                },
                                button! {
                                    text: "Reject",
                                    event: move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                        let request = state
                                            .get(&client_state().party_booking())
                                            .join_requests
                                            .iter()
                                            .find(|request| request.character_id.0 == character_id)
                                            .cloned();

                                        if let Some(request) = request {
                                            queue.queue(InputEvent::ReplyPartyBookingJoin {
                                                character_id: request.character_id,
                                                account_id: request.account_id,
                                                accept: false,
                                            });
                                        }
                                    },
                                },
                            ),
                        },
                    }));
                }
            }

            self.elements
                .iter_mut()
                .zip(self.built_characters.iter())
                .for_each(|(element, character_id)| {
                    element.create_layout_info(state, store.child_store(u64::from(*character_id)), resolver);
                });
        });
    }

    fn lay_out<'a>(
        &'a self,
        state: &'a State<ClientState>,
        store: ElementStore<'a>,
        _: &'a Self::LayoutInfo,
        layout: &mut WindowLayout<'a, ClientState>,
    ) {
        self.elements
            .iter()
            .zip(self.built_characters.iter())
            .for_each(|(element, character_id)| {
                element.lay_out(state, store.child_store(u64::from(*character_id)), &(), layout);
            });
    }
}

fn settings_disabled() -> impl Selector<ClientState, bool> {
    ComputedSelector::<_, bool>::new_default(|state: &ClientState| {
        let player_name = client_state().player_name().follow_safe(state);
        let party = client_state().party().follow_safe(state);
        !party.is_leader(player_name)
    })
}

fn show_booking() -> impl Selector<ClientState, bool> {
    ComputedSelector::<_, bool>::new_default(|state: &ClientState| client_state().party_booking().follow_safe(state).visible)
}

fn roster_tab_disabled() -> impl Selector<ClientState, bool> {
    ComputedSelector::<_, bool>::new_default(|state: &ClientState| !client_state().party_booking().follow_safe(state).visible)
}

fn booking_tab_disabled() -> impl Selector<ClientState, bool> {
    ComputedSelector::<_, bool>::new_default(|state: &ClientState| client_state().party_booking().follow_safe(state).visible)
}

fn experience_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(
        |state: &ClientState| match client_state().party().follow_safe(state).experience_option {
            1 => "EXP sharing: even share".to_owned(),
            2 => "EXP sharing: fixed by the server".to_owned(),
            _ => "EXP sharing: each take".to_owned(),
        },
    )
}

fn item_pickup_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(
        |state: &ClientState| match client_state().party().follow_safe(state).item_pickup {
            true => "Item pickup: even share".to_owned(),
            false => "Item pickup: each take".to_owned(),
        },
    )
}

fn item_share_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(
        |state: &ClientState| match client_state().party().follow_safe(state).item_share {
            true => "Item sharing: on".to_owned(),
            false => "Item sharing: off".to_owned(),
        },
    )
}

fn booking_status_text() -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(|state: &ClientState| {
        client_state().party_booking().follow_safe(state).status_message.clone()
    })
}

fn booking_row_text(index: u32) -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(move |state: &ClientState| {
        client_state()
            .party_booking()
            .follow_safe(state)
            .result(index)
            .map(|entry| entry.display_line())
            .unwrap_or_default()
    })
}

fn join_request_row_text(character_id: u32) -> impl Selector<ClientState, String> {
    ComputedSelector::<_, String>::new_default(move |state: &ClientState| {
        match client_state()
            .party_booking()
            .follow_safe(state)
            .join_requests
            .iter()
            .find(|request| request.character_id.0 == character_id)
        {
            Some(request) => format!("{} wants to join the party", request.name),
            None => String::new(),
        }
    })
}

pub struct PartyWindow;

impl CustomWindow<ClientState> for PartyWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Party)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use ro_next_interface::prelude::*;

        let create_input_path = client_state().party().create_name_input();
        let invite_input_path = client_state().party().invite_name_input();
        let leader_input_path = client_state().party().leader_name_input();

        let register_level_path = client_state().party_booking().register_level_input();
        let register_map_path = client_state().party_booking().register_map_input();
        let register_jobs_path = client_state().party_booking().register_jobs_input();
        let search_level_path = client_state().party_booking().search_level_input();
        let search_map_path = client_state().party_booking().search_map_input();
        let search_job_path = client_state().party_booking().search_job_input();

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

        let leader_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&leader_input_path).trim().to_owned();

            if !name.is_empty() {
                state.update_value_with(leader_input_path, |input: &mut String| input.clear());
                queue.queue(InputEvent::ChangePartyLeader { name });
            }
        };

        let exp_even_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let party = state.follow(client_state().party());
            queue.queue(InputEvent::ChangePartyOption {
                experience_option: 1,
                item_pickup: party.item_pickup,
                item_share: party.item_share,
            });
        };

        let exp_each_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let party = state.follow(client_state().party());
            queue.queue(InputEvent::ChangePartyOption {
                experience_option: 0,
                item_pickup: party.item_pickup,
                item_share: party.item_share,
            });
        };

        let item_pickup_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let party = state.follow(client_state().party());
            queue.queue(InputEvent::ChangePartyOption {
                experience_option: party.experience_option,
                item_pickup: !party.item_pickup,
                item_share: party.item_share,
            });
        };

        let item_share_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let party = state.follow(client_state().party());
            queue.queue(InputEvent::ChangePartyOption {
                experience_option: party.experience_option,
                item_pickup: party.item_pickup,
                item_share: !party.item_share,
            });
        };

        let noop = |_: &State<ClientState>, _: &mut EventQueue<ClientState>| {};

        struct CreatePartyName;
        struct InvitePartyName;
        struct LeaderName;
        struct RegisterLevel;
        struct RegisterMap;
        struct RegisterJobs;
        struct SearchLevel;
        struct SearchMap;
        struct SearchJob;

        window! {
            title: client_state().localization().party_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 460.0,
            maximum_width: 460.0,
            elements: (
                split! {
                    gaps: 8.0,
                    children: (
                        button! {
                            text: "Party",
                            disabled: roster_tab_disabled(),
                            event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                queue.queue(InputEvent::SetPartyBookingVisible { visible: false });
                            },
                        },
                        button! {
                            text: "Party search",
                            disabled: booking_tab_disabled(),
                            event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                queue.queue(InputEvent::SetPartyBookingVisible { visible: true });
                            },
                        },
                    ),
                },
                either! {
                    selector: show_booking(),
                    on_true: fragment! {
                        gaps: theme().window().gaps(),
                        children: (
                            text! {
                                text: "Party booking board (2019 / LFG). Register your party, then search the board. On CondaRO 20190605 the board only supports register/search/delete/update; direct join requests need PACKETVER >= 20191204, so whisper the leader instead.",
                                overflow_behavior: OverflowBehavior::LineBreak,
                            },
                            text! {
                                text: booking_status_text(),
                                overflow_behavior: OverflowBehavior::LineBreak,
                            },
                            text! {
                                text: "Register your party",
                            },
                            split! {
                                gaps: 8.0,
                                children: (
                                    text_box! {
                                        ghost_text: "Level",
                                        state: register_level_path,
                                        input_handler: DefaultHandler::<_, _, 6>::new(register_level_path, noop),
                                        focus_id: RegisterLevel,
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    text_box! {
                                        ghost_text: "Map index (0 = any)",
                                        state: register_map_path,
                                        input_handler: DefaultHandler::<_, _, 6>::new(register_map_path, noop),
                                        focus_id: RegisterMap,
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                ),
                            },
                            text_box! {
                                ghost_text: "Desired jobs: six ids, -1 = any (e.g. 7,8,16,-1,-1,-1)",
                                state: register_jobs_path,
                                input_handler: DefaultHandler::<_, _, 64>::new(register_jobs_path, noop),
                                focus_id: RegisterJobs,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            split! {
                                gaps: 8.0,
                                children: (
                                    button! {
                                        text: "Register",
                                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            queue.queue(InputEvent::RegisterPartyBooking);
                                        },
                                    },
                                    button! {
                                        text: "Update jobs",
                                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            queue.queue(InputEvent::UpdatePartyBooking);
                                        },
                                    },
                                    button! {
                                        text: "Delete",
                                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            queue.queue(InputEvent::DeletePartyBooking);
                                        },
                                    },
                                ),
                            },
                            text! {
                                text: "Search the board",
                            },
                            split! {
                                gaps: 8.0,
                                children: (
                                    text_box! {
                                        ghost_text: "Level (0 = any)",
                                        state: search_level_path,
                                        input_handler: DefaultHandler::<_, _, 6>::new(search_level_path, noop),
                                        focus_id: SearchLevel,
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    text_box! {
                                        ghost_text: "Map index (0 = any)",
                                        state: search_map_path,
                                        input_handler: DefaultHandler::<_, _, 6>::new(search_map_path, noop),
                                        focus_id: SearchMap,
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                    text_box! {
                                        ghost_text: "Job (-1 = any)",
                                        state: search_job_path,
                                        input_handler: DefaultHandler::<_, _, 6>::new(search_job_path, noop),
                                        focus_id: SearchJob,
                                        overflow_behavior: OverflowBehavior::Shrink,
                                    },
                                ),
                            },
                            split! {
                                gaps: 8.0,
                                children: (
                                    button! {
                                        text: "Search",
                                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            queue.queue(InputEvent::SearchPartyBooking { next_page: false });
                                        },
                                    },
                                    button! {
                                        text: "Next page",
                                        event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                                            queue.queue(InputEvent::SearchPartyBooking { next_page: true });
                                        },
                                    },
                                    state_button! {
                                        text: "Server supports booking join requests (PACKETVER >= 20191204)",
                                        state: client_state().party_booking().join_requests_enabled(),
                                        event: Toggle(client_state().party_booking().join_requests_enabled()),
                                    },
                                ),
                            },
                            text! {
                                text: "Search results",
                            },
                            PartyBookingResultList::new(),
                            text! {
                                text: "Join requests (only sent by servers with PACKETVER >= 20191204)",
                            },
                            PartyBookingJoinRequestList::new(),
                            PartyRoster::new(client_state().party()),
                        ),
                    },
                    on_false: fragment! {
                        gaps: theme().window().gaps(),
                        children: (
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
                            text! {
                                text: "Party settings (leader only)",
                            },
                            text! {
                                text: experience_text(),
                            },
                            split! {
                                gaps: 8.0,
                                children: (
                                    button! {
                                        text: "Even share",
                                        disabled: settings_disabled(),
                                        disabled_tooltip: "Only the party leader can change the EXP sharing rule.",
                                        event: exp_even_action,
                                    },
                                    button! {
                                        text: "Each take",
                                        disabled: settings_disabled(),
                                        disabled_tooltip: "Only the party leader can change the EXP sharing rule.",
                                        event: exp_each_action,
                                    },
                                ),
                            },
                            text! {
                                text: item_pickup_text(),
                            },
                            button! {
                                text: "Toggle item pickup rule",
                                disabled: settings_disabled(),
                                disabled_tooltip: "Only the party leader can change the item rules.",
                                event: item_pickup_action,
                            },
                            text! {
                                text: item_share_text(),
                            },
                            button! {
                                text: "Toggle item sharing",
                                disabled: settings_disabled(),
                                disabled_tooltip: "Only the party leader can change the item rules.",
                                event: item_share_action,
                            },
                            text! {
                                text: "Change party leader",
                            },
                            text_box! {
                                ghost_text: "New leader name",
                                state: leader_input_path,
                                input_handler: DefaultHandler::<_, _, NAME_LENGTH>::new(leader_input_path, leader_action),
                                focus_id: LeaderName,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: "Make leader",
                                disabled: settings_disabled(),
                                disabled_tooltip: "Only the current party leader can transfer leadership.",
                                event: leader_action,
                            },
                        ),
                    },
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
        use ro_next_interface::prelude::*;

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
