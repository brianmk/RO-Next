//! Quest tracker: active quests and their hunting objectives.
//!
//! The server's quest packets carry objective names and counts but not the
//! quest title, so the tracker shows the quest id and one line per objective.
//! The window is opened from the menu and refreshed from `ClientState`.

use korangar_interface::prelude::ComputedSelector;
use korangar_interface::window::{CustomWindow, Window};
use rust_state::{Path, PathExt, Selector};

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::quest_log::QuestLog;
use crate::state::{ClientState, ClientStatePathExt};

pub struct QuestWindow<A> {
    quest_log_path: A,
}

impl<A> QuestWindow<A> {
    pub fn new(quest_log_path: A) -> Self {
        Self { quest_log_path }
    }
}

impl<A> CustomWindow<ClientState> for QuestWindow<A>
where
    A: Path<ClientState, QuestLog> + 'static,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Quest)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let path = self.quest_log_path;
        let text = ComputedSelector::new_default(move |state: &ClientState| {
            let log = path.follow_safe(state);
            let mut output = String::new();
            for entry in log.active() {
                output.push_str(&format!("Quest #{}\n", entry.quest_id));
                for objective in &entry.objectives {
                    output.push_str(&format!("  {} {}/{}\n", objective.name, objective.current, objective.total));
                }
            }
            if output.is_empty() {
                output.push_str("No active quests.");
            }
            output
        });

        window! {
            title: "Quest Log",
            class: Self::window_class(),
            theme: crate::state::theme::InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 320.0,
            maximum_width: 520.0,
            elements: (
                text! {
                    text,
                    overflow_behavior: OverflowBehavior::Shrink,
                },
            ),
        }
    }
}
