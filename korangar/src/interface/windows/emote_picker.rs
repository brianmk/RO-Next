//! Small emotion picker window.
//!
//! The client already has the `Alt`+`1`..`9` emotion hotkeys
//! (`input/keybinds.rs` and `InputEvent::SendEmotion`) but no way to pick the
//! rest of the 2019 emotion set with the mouse. This window is a plain grid of
//! buttons; every click sends `CZ_REQ_EMOTION` (0x00BF) through the existing
//! `InputEvent::SendEmotion` path.
//!
//! The labels are the rAthena `emotion_type` names of `src/map/clif.hpp`
//! (0..19). The animated emotion graphics are frames of `이팩트\emotion.spr`,
//! not standalone textures, so the picker uses text buttons instead of trying
//! to slice the sprite sheet.

use korangar_interface::window::{CustomWindow, Window};

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

/// Emotion label table, in wire order (`emotion_type` of `clif.hpp`).
const EMOTIONS: [&str; 20] = [
    "! Surprise",
    "? Question",
    "Delight",
    "Throb",
    "Sweat",
    "Aha",
    "Fret",
    "Anger",
    "$ Money",
    "... Think",
    "Scissors",
    "Rock",
    "Paper",
    "Flag",
    "Big throb",
    "Thanks",
    "Kek",
    "Sorry",
    "Smile",
    "Sweat+",
];

pub struct EmotePickerWindow;

impl CustomWindow<ClientState> for EmotePickerWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::EmotePicker)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Emotes",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            resizable: false,
            minimum_width: 420.0,
            maximum_width: 560.0,
            elements: (
                split! {
                    children: (
                        button! { text: EMOTIONS[0], event: InputEvent::SendEmotion { emotion: 0 } },
                        button! { text: EMOTIONS[1], event: InputEvent::SendEmotion { emotion: 1 } },
                        button! { text: EMOTIONS[2], event: InputEvent::SendEmotion { emotion: 2 } },
                        button! { text: EMOTIONS[3], event: InputEvent::SendEmotion { emotion: 3 } },
                    ),
                },
                split! {
                    children: (
                        button! { text: EMOTIONS[4], event: InputEvent::SendEmotion { emotion: 4 } },
                        button! { text: EMOTIONS[5], event: InputEvent::SendEmotion { emotion: 5 } },
                        button! { text: EMOTIONS[6], event: InputEvent::SendEmotion { emotion: 6 } },
                        button! { text: EMOTIONS[7], event: InputEvent::SendEmotion { emotion: 7 } },
                    ),
                },
                split! {
                    children: (
                        button! { text: EMOTIONS[8], event: InputEvent::SendEmotion { emotion: 8 } },
                        button! { text: EMOTIONS[9], event: InputEvent::SendEmotion { emotion: 9 } },
                        button! { text: EMOTIONS[10], event: InputEvent::SendEmotion { emotion: 10 } },
                        button! { text: EMOTIONS[11], event: InputEvent::SendEmotion { emotion: 11 } },
                    ),
                },
                split! {
                    children: (
                        button! { text: EMOTIONS[12], event: InputEvent::SendEmotion { emotion: 12 } },
                        button! { text: EMOTIONS[13], event: InputEvent::SendEmotion { emotion: 13 } },
                        button! { text: EMOTIONS[14], event: InputEvent::SendEmotion { emotion: 14 } },
                        button! { text: EMOTIONS[15], event: InputEvent::SendEmotion { emotion: 15 } },
                    ),
                },
                split! {
                    children: (
                        button! { text: EMOTIONS[16], event: InputEvent::SendEmotion { emotion: 16 } },
                        button! { text: EMOTIONS[17], event: InputEvent::SendEmotion { emotion: 17 } },
                        button! { text: EMOTIONS[18], event: InputEvent::SendEmotion { emotion: 18 } },
                        button! { text: EMOTIONS[19], event: InputEvent::SendEmotion { emotion: 19 } },
                    ),
                },
                text! {
                    text: "Alt + 1..9 also work while the chat line is closed.",
                },
            ),
        }
    }
}
