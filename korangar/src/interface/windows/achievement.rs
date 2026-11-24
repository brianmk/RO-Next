//! Achievement window (`ZC_ALL_ACH_LIST` 0x0A23 / `ZC_ACH_UPDATE` 0x0A24).
//!
//! The 2019 packets only carry achievement ids, objective counters and the
//! completed/rewarded flags. The official client resolves the titles and
//! descriptions from `data\achievement\achievement_list.lub`; that file is not
//! part of this client data set (verified against the archives), so the window
//! shows the ids and raw progress instead. Reward claims are still fully
//! functional through `CZ_REQ_ACH_REWARD` (0x0A25).

use korangar_interface::components::text_box::DefaultHandler;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{PathExt, State};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::achievement::AchievementStatePathExt;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

pub struct AchievementWindow;

impl CustomWindow<ClientState> for AchievementWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Achievement)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let path = client_state().achievement();

        let header = ComputedSelector::new_default(move |state: &ClientState| {
            let achievements = path.follow_safe(state);

            format!(
                "Achievement level {}   Score {}   EXP {}/{}   Completed {}/{}",
                achievements.level(),
                achievements.total_score(),
                achievements.experience(),
                achievements.experience_to_next_level(),
                achievements.completed_count(),
                achievements.entries().len(),
            )
        });

        let status = ComputedSelector::new_default(move |state: &ClientState| {
            let achievements = path.follow_safe(state);
            let mut text = achievements.status().to_owned();

            if !text.is_empty() {
                text.push('\n');
            }

            text.push_str(
                "Achievement titles are client Lua data (achievement_list.lub) that this data set does not ship; the list below shows the \
                 server ids and raw objective counters. Enter a completed achievement id to claim its reward.",
            );

            text
        });

        let list = ComputedSelector::new_default(move |state: &ClientState| {
            let achievements = path.follow_safe(state);

            if achievements.is_empty() {
                return "No achievements loaded. The server only sends the list when the character has achievements.".to_owned();
            }

            achievements
                .entries()
                .iter()
                .map(|entry| {
                    let state = match entry.is_claimable() {
                        true => "COMPLETED, reward available",
                        false => match entry.completed {
                            true => "completed, rewarded",
                            false => "in progress",
                        },
                    };

                    format!(
                        "#{}  {state}\n  objectives: {}{}",
                        entry.achievement_id,
                        entry.objective_summary(),
                        if entry.completion_timestamp != 0 {
                            format!("\n  completed at: {}", entry.completion_timestamp)
                        } else {
                            String::new()
                        },
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        });

        let claim_path = path.claim_input();
        let claim_action = move |state: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            let input = state.get(&claim_path).trim().to_owned();

            if let Ok(achievement_id) = input.parse::<u32>() {
                state.update_value_with(claim_path, |input: &mut String| input.clear());
                queue.queue(InputEvent::ClaimAchievementReward { achievement_id });
            }
        };

        struct ClaimId;

        window! {
            title: client_state().localization().ui().achievement_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: true,
            minimum_width: 420.0,
            maximum_width: 620.0,
            maximum_height: 700.0,
            elements: scroll_view! {
                scroll_bar: true,
                fit_content_height: true,
                children: (
                    text! {
                        text: header,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    text! {
                        text: list,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                    split! {
                        gaps: theme().window().gaps(),
                        children: (
                            text_box! {
                                ghost_text: "Achievement id",
                                state: claim_path,
                                input_handler: DefaultHandler::<_, _, 10>::new(claim_path, claim_action),
                                focus_id: ClaimId,
                                overflow_behavior: OverflowBehavior::Shrink,
                            },
                            button! {
                                text: "Claim reward",
                                event: claim_action,
                            },
                        ),
                    },
                    text! {
                        text: status,
                        overflow_behavior: OverflowBehavior::LineBreak,
                    },
                ),
            },
        }
    }
}
