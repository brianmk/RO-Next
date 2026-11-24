//! Guild window for the 2019 rAthena guild system.
//!
//! The layout follows the classic guild window: basic information at the top,
//! the notice, the member list with position/grade names, the position rights
//! and the alliance list. The window is read-mostly and is driven entirely by
//! [`crate::state::guild::Guild`].
//!
//! The 2019 member list packet (`ZC_MEMBERMGR_INFO`, 0x0AA5) does not contain
//! names, so the state module requests them with `CZ_REQNAME_BYGID` (0x094C)
//! and the rows show `Unknown (#id)` until the answer arrives.
//!
//! Actions:
//! * invite (0x0916) — enabled with `GUILD_PERM_INVITE` or as guild master,
//! * expel (0x015B) — enabled with `GUILD_PERM_EXPEL` or as guild master,
//! * leave (0x0159) — enabled while the local ids and guild id are known,
//! * notice edit (0x016E) — enabled for the guild master (0x80 menu flag).
//!
//! Position editing (0x0161 / 0x0155) is intentionally not offered: the 2019
//! layout is a variable-length array of 40-byte rows and changing it is not
//! required for a usable guild window.

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{PathExt, Selector, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::guild::GuildPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct GuildWindow;

impl GuildWindow {
    fn header_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| {
            let guild = client_state().guild().follow_safe(state);

            if !guild.has_info {
                return "No guild information loaded. You are not in a guild, or the server has not answered yet.".to_owned();
            }

            let name = if guild.name.trim().is_empty() {
                format!("Guild #{}", guild.guild_id)
            } else {
                guild.name.clone()
            };

            format!(
                "{name}   Level {}   Members {} / {}   EXP {} / {}   Points {}",
                guild.level, guild.online_members, guild.max_members, guild.experience, guild.max_experience, guild.points
            )
        })
    }

    fn master_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| {
            let guild = client_state().guild().follow_safe(state);

            if !guild.has_info {
                return String::new();
            }

            let mut text = format!("Guild master: {}", guild.master_display_name());

            if !guild.manage_land.trim().is_empty() {
                text.push_str(&format!("   Territory: {}", guild.manage_land));
            }

            if guild.max_members > 0 {
                text.push_str(&format!("   Emblem version: {}", guild.emblem_id));
            }

            text
        })
    }

    fn notice_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| {
            let guild = client_state().guild().follow_safe(state);

            if !guild.has_info {
                return String::new();
            }

            if guild.notice.trim().is_empty() && guild.notice_subject.trim().is_empty() {
                return "Notice: none".to_owned();
            }

            match guild.notice_subject.trim() {
                "" => format!("Notice:\n{}", guild.notice),
                subject => format!("Notice: {subject}\n{}", guild.notice),
            }
        })
    }

    fn member_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| client_state().guild().follow_safe(state).member_list_text())
    }

    fn position_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| client_state().guild().follow_safe(state).position_list_text())
    }

    fn alliance_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| client_state().guild().follow_safe(state).alliance_list_text())
    }

    fn status_text() -> impl Selector<ClientState, String> {
        ComputedSelector::new_default(|state: &ClientState| client_state().guild().follow_safe(state).status_message.clone())
    }

    fn invite_disabled() -> impl Selector<ClientState, bool> {
        ComputedSelector::new_default(|state: &ClientState| {
            let guild = client_state().guild().follow_safe(state);
            !guild.can_invite() || guild.invite_name_input.trim().is_empty()
        })
    }

    fn expel_disabled() -> impl Selector<ClientState, bool> {
        ComputedSelector::new_default(|state: &ClientState| {
            let guild = client_state().guild().follow_safe(state);
            !guild.can_expel() || guild.expel_name_input.trim().is_empty()
        })
    }

    fn leave_disabled() -> impl Selector<ClientState, bool> {
        ComputedSelector::new_default(|state: &ClientState| {
            let guild = client_state().guild().follow_safe(state);
            !guild.is_in_guild() || guild.local_account_id.is_none() || guild.local_character_id.is_none()
        })
    }

    fn notice_disabled() -> impl Selector<ClientState, bool> {
        ComputedSelector::new_default(|state: &ClientState| !client_state().guild().follow_safe(state).can_edit_notice())
    }
}

impl CustomWindow<ClientState> for GuildWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Guild)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let invite_path = client_state().guild().invite_name_input();
        let expel_path = client_state().guild().expel_name_input();
        let notice_subject_path = client_state().guild().notice_subject_input();
        let notice_path = client_state().guild().notice_input();
        let invite_text_path = invite_path;
        let expel_text_path = expel_path;
        let notice_subject_save_path = notice_subject_path;
        let notice_save_path = notice_path;

        let invite_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&invite_text_path).trim().to_owned();

            if !name.is_empty() {
                state.update_value_with(invite_path, |input: &mut String| input.clear());
                queue.queue(InputEvent::InviteToGuild { name });
            }
        };

        let expel_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let name = state.get(&expel_text_path).trim().to_owned();

            if !name.is_empty() {
                state.update_value_with(expel_path, |input: &mut String| input.clear());
                queue.queue(InputEvent::ExpelGuildMember { name });
            }
        };

        let leave_action = move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            queue.queue(InputEvent::LeaveGuild);
        };

        let save_notice_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let subject = state.get(&notice_subject_save_path).trim().to_owned();
            let notice = state.get(&notice_save_path).trim().to_owned();

            queue.queue(InputEvent::SaveGuildNotice { subject, notice });
        };

        struct InviteName;
        struct ExpelName;
        struct NoticeSubject;
        struct NoticeBody;

        window! {
            title: "Guild",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 440.0,
            maximum_width: 700.0,
            maximum_height: 720.0,
            // A 2019 guild can have many members; the whole window scrolls so
            // the action controls stay reachable below a long member list.
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: (
                text! {
                    text: Self::header_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::master_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::notice_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::member_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::position_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::alliance_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                text! {
                    text: Self::status_text(),
                    overflow_behavior: OverflowBehavior::LineBreak,
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text_box! {
                            ghost_text: "Invite character",
                            state: invite_path,
                            input_handler: DefaultHandler::<_, _, 24>::new(invite_path, invite_action),
                            focus_id: InviteName,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "Invite",
                            disabled: Self::invite_disabled(),
                            disabled_tooltip: "Requires the guild invite permission.",
                            event: invite_action,
                        },
                    ),
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text_box! {
                            ghost_text: "Expel character",
                            state: expel_path,
                            input_handler: DefaultHandler::<_, _, 24>::new(expel_path, expel_action),
                            focus_id: ExpelName,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "Expel",
                            disabled: Self::expel_disabled(),
                            disabled_tooltip: "Requires the guild expulsion permission.",
                            event: expel_action,
                        },
                        button! {
                            text: "Leave guild",
                            disabled: Self::leave_disabled(),
                            disabled_tooltip: "You are not in a guild.",
                            event: leave_action,
                        },
                    ),
                },
                text! {
                    text: "Edit notice (guild master only)",
                },
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        text_box! {
                            ghost_text: "Subject",
                            state: notice_subject_path,
                            input_handler: DefaultHandler::<_, _, 60>::new(notice_subject_path, save_notice_action),
                            focus_id: NoticeSubject,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        text_box! {
                            ghost_text: "Notice",
                            state: notice_path,
                            input_handler: DefaultHandler::<_, _, 120>::new(notice_path, save_notice_action),
                            focus_id: NoticeBody,
                            overflow_behavior: OverflowBehavior::Shrink,
                        },
                        button! {
                            text: "Save notice",
                            disabled: Self::notice_disabled(),
                            disabled_tooltip: "Only the guild master can change the notice.",
                            event: save_notice_action,
                        },
                    ),
                },
                ),
            },
        }
    }
}
