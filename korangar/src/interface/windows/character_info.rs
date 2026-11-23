//! Small character information window opened from the right-click menu.
//!
//! The map server's entity details already carry the name, guild and level;
//! this window shows them without needing an extra request, replacing the
//! previously disabled "View character info" menu entry.

use korangar_interface::window::{CustomWindow, Window};

use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::theme::InterfaceThemeType;

pub struct CharacterInfoWindow {
    name: String,
    level: u16,
    guild_name: Option<String>,
}

impl CharacterInfoWindow {
    pub fn new(name: String, level: u16, guild_name: Option<String>) -> Self {
        Self { name, level, guild_name }
    }
}

impl CustomWindow<crate::state::ClientState> for CharacterInfoWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::CharacterInfo)
    }

    fn to_window<'a>(self) -> impl Window<crate::state::ClientState> + 'a {
        use korangar_interface::prelude::*;

        let guild = self.guild_name.unwrap_or_else(|| "No guild".to_string());
        window! {
            title: self.name,
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: 260.0,
            maximum_width: 380.0,
            elements: (
                text! {
                    text: format!("Level: {}", self.level),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
                text! {
                    text: format!("Guild: {guild}"),
                    overflow_behavior: OverflowBehavior::Shrink,
                },
            ),
        }
    }
}
