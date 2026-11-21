//! Small popup menu that is opened by right-clicking a character, like the
//! context menu of the original client.

use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{InventoryIndex, ItemId};
use rust_state::State;

use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::inventory::ItemUseAction;
use crate::state::theme::InterfaceThemeType;

/// Width of the context menu window.
const CONTEXT_MENU_WIDTH: f32 = 190.0;

pub struct CharacterContextMenuWindow {
    target_name: String,
}

impl CharacterContextMenuWindow {
    pub fn new(target_name: String) -> Self {
        Self { target_name }
    }
}

impl CustomWindow<ClientState> for CharacterContextMenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ContextMenu)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let target_name = self.target_name;
        let whisper_name = target_name.clone();
        let add_friend_name = target_name.clone();

        window! {
            title: target_name.clone(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: CONTEXT_MENU_WIDTH,
            maximum_width: CONTEXT_MENU_WIDTH,
            elements: (
                button! {
                    text: "Whisper",
                    tooltip: format!("Open the chat with a whisper to ^000001{target_name}^000000"),
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::WhisperChat { name: whisper_name.clone() });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: "Add Friend",
                    tooltip: format!("Send a friend request to ^000001{target_name}^000000"),
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::AddFriend {
                            character_name: add_friend_name.clone(),
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: "Invite to Party",
                    disabled: true,
                    disabled_tooltip: "Not implemented yet: the client cannot send party invitations",
                    event: InputEvent::CloseContextMenu,
                },
                button! {
                    text: "Trade",
                    disabled: true,
                    disabled_tooltip: "Not implemented yet: the client cannot start trades",
                    event: InputEvent::CloseContextMenu,
                },
                button! {
                    text: "View Info",
                    disabled: true,
                    disabled_tooltip: "Not implemented yet: there is no character info window",
                    event: InputEvent::CloseContextMenu,
                },
            ),
        }
    }
}

/// Context menu that is opened by right-clicking an item in the inventory.
pub struct ItemContextMenuWindow {
    inventory_index: InventoryIndex,
    item_id: ItemId,
    action: Option<ItemUseAction>,
}

impl ItemContextMenuWindow {
    pub fn new(inventory_index: InventoryIndex, item_id: ItemId, action: Option<ItemUseAction>) -> Self {
        Self {
            inventory_index,
            item_id,
            action,
        }
    }
}

impl CustomWindow<ClientState> for ItemContextMenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ContextMenu)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let use_equip_label = match self.action {
            Some(ItemUseAction::Use) => "Use",
            Some(ItemUseAction::Equip(_)) => "Equip",
            None => "Use / Equip",
        };

        let use_equip_event = move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            match self.action {
                Some(ItemUseAction::Use) => {
                    queue.queue(InputEvent::UseInventoryItem {
                        inventory_index: self.inventory_index,
                        item_id: self.item_id,
                    });
                }
                Some(ItemUseAction::Equip(equip_position)) => {
                    queue.queue(InputEvent::EquipInventoryItem {
                        inventory_index: self.inventory_index,
                        equip_position,
                    });
                }
                None => {}
            }

            queue.queue(InputEvent::CloseContextMenu);
        };

        let info_index = self.inventory_index;

        window! {
            title: "Item",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: CONTEXT_MENU_WIDTH,
            maximum_width: CONTEXT_MENU_WIDTH,
            elements: (
                button! {
                    text: use_equip_label,
                    disabled: self.action.is_none(),
                    disabled_tooltip: "This item cannot be used or equipped",
                    event: use_equip_event,
                },
                button! {
                    text: "View Info",
                    tooltip: "Show the item description and stats",
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::OpenItemInfo {
                            inventory_index: info_index,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: "Drop",
                    disabled: true,
                    disabled_tooltip: "Not implemented yet: the client cannot drop items on the ground",
                    event: InputEvent::CloseContextMenu,
                },
            ),
        }
    }
}
