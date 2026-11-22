//! Small popup menu that is opened by right-clicking a character, like the
//! context menu of the original client.

use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{InventoryIndex, ItemId};
use rust_state::State;

use crate::input::InputEvent;
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::state::inventory::ItemUseAction;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

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

/// What can be done with an item while a Kafra storage window is open.
#[derive(Clone, Copy, Debug)]
pub enum ItemStorageAction {
    /// Store an inventory item into the open storage.
    Deposit {
        inventory_index: InventoryIndex,
        /// Amount that can be stored (at most the stack size).
        amount: u32,
    },
    /// Take a storage item back into the inventory.
    Withdraw {
        storage_index: InventoryIndex,
        /// Amount that can be taken out (at most the stack size).
        amount: u32,
    },
}

impl ItemStorageAction {
    fn with_amount(&self, amount: u32) -> InputEvent {
        match self {
            Self::Deposit { inventory_index, .. } => InputEvent::DepositInventoryItem {
                inventory_index: *inventory_index,
                amount,
            },
            Self::Withdraw { storage_index, .. } => InputEvent::WithdrawStorageItem {
                storage_index: *storage_index,
                amount,
            },
        }
    }
}

/// Context menu that is opened by right-clicking an item in the inventory.
pub struct ItemContextMenuWindow {
    inventory_index: InventoryIndex,
    item_id: ItemId,
    action: Option<ItemUseAction>,
    storage_action: Option<ItemStorageAction>,
    /// Where the item is located; decides which state the "View Info" action
    /// reads.
    source: ItemSource,
}

impl ItemContextMenuWindow {
    pub fn new(inventory_index: InventoryIndex, item_id: ItemId, action: Option<ItemUseAction>) -> Self {
        Self {
            inventory_index,
            item_id,
            action,
            storage_action: None,
            source: ItemSource::Inventory,
        }
    }

    /// Select the state that the "View Info" action reads the item from.
    pub fn with_source(mut self, source: ItemSource) -> Self {
        self.source = source;
        self
    }

    /// Add the Kafra storage entries to the menu (only possible while a storage
    /// window is open).
    pub fn with_storage_action(mut self, storage_action: Option<ItemStorageAction>) -> Self {
        self.storage_action = storage_action;
        self
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

        let storage_action = self.storage_action;
        let storage_action_single = storage_action;
        let storage_action_all = storage_action;

        let storage_single_label = match storage_action {
            Some(ItemStorageAction::Deposit { .. }) => "Store 1",
            Some(ItemStorageAction::Withdraw { .. }) => "Take out 1",
            None => "Store 1",
        };

        let storage_all_label = match storage_action {
            Some(ItemStorageAction::Deposit { .. }) => "Store all",
            Some(ItemStorageAction::Withdraw { .. }) => "Take out all",
            None => "Store all",
        };

        let storage_single_event = move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            if let Some(storage_action) = storage_action_single {
                queue.queue(storage_action.with_amount(1));
            }

            queue.queue(InputEvent::CloseContextMenu);
        };

        let storage_all_event = move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
            if let Some(storage_action) = storage_action_all {
                let amount = match storage_action {
                    ItemStorageAction::Deposit { amount, .. } | ItemStorageAction::Withdraw { amount, .. } => amount.max(1),
                };

                queue.queue(storage_action.with_amount(amount));
            }

            queue.queue(InputEvent::CloseContextMenu);
        };

        let info_index = self.inventory_index;
        let info_source = self.source;

        window! {
            title: client_state().localization().item_info_window_title(),
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
                            source: info_source,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: storage_single_label,
                    disabled: self.storage_action.is_none(),
                    disabled_tooltip: "Talk to a Kafra to use the storage",
                    event: storage_single_event,
                },
                button! {
                    text: storage_all_label,
                    disabled: self.storage_action.is_none(),
                    disabled_tooltip: "Talk to a Kafra to use the storage",
                    event: storage_all_event,
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
