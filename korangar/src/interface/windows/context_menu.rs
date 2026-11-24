//! Small popup menu that is opened by right-clicking a character, like the
//! context menu of the original client.

use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{InventoryIndex, ItemId};
use rust_state::{PathExt, State};

use crate::input::InputEvent;
use crate::interface::resource::ItemSource;
use crate::interface::windows::WindowClass;
use crate::state::inventory::ItemUseAction;
use crate::state::localization::{LocalizationPathExt, UiTextsPathExt, template_text};
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};

/// Width of the context menu window.
const CONTEXT_MENU_WIDTH: f32 = 190.0;

/// Context menu entry that browses another player's vending shop.
const BROWSE_VENDING_SHOP_TEXT: &str = "Browse shop";

pub struct CharacterContextMenuWindow {
    target_name: String,
    target_account_id: u32,
    target_level: u16,
    target_guild: Option<String>,
    /// Title of the target's open vending shop (`ZC_STORE_ENTRY`); `None` while
    /// the player is not vending.
    vending_shop_name: Option<String>,
}

impl CharacterContextMenuWindow {
    pub fn new(
        target_name: String,
        target_account_id: u32,
        target_level: u16,
        target_guild: Option<String>,
        vending_shop_name: Option<String>,
    ) -> Self {
        Self {
            target_name,
            target_account_id,
            target_level,
            target_guild,
            vending_shop_name,
        }
    }
}

impl CustomWindow<ClientState> for CharacterContextMenuWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::ContextMenu)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        let target_name = self.target_name;
        let target_account_id = self.target_account_id;
        let whisper_name = target_name.clone();
        let add_friend_name = target_name.clone();
        let invite_party_name = target_name.clone();
        let info_name = target_name.clone();
        let info_level = self.target_level;
        let info_guild = self.target_guild.clone();
        let vending_shop_name = self.vending_shop_name.clone();
        let shop_name = vending_shop_name.clone().unwrap_or_default();
        let browsing_disabled = vending_shop_name.is_none();
        let browse_shop_button = button! {
            text: BROWSE_VENDING_SHOP_TEXT,
            disabled: ComputedSelector::new_default(move |_: &ClientState| browsing_disabled),
            event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                queue.queue(InputEvent::BrowseVendingShop {
                    owner_account_id: target_account_id,
                    shop_name: shop_name.clone(),
                });
                queue.queue(InputEvent::CloseContextMenu);
            },
        };
        let whisper_tooltip = template_text(
            |state| client_state().localization().ui().context_menu_whisper_tooltip().follow_safe(state),
            target_name.clone(),
        );
        let add_friend_tooltip = template_text(
            |state| {
                client_state()
                    .localization()
                    .ui()
                    .context_menu_add_friend_tooltip()
                    .follow_safe(state)
            },
            target_name.clone(),
        );

        window! {
            title: target_name.clone(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            minimum_width: CONTEXT_MENU_WIDTH,
            maximum_width: CONTEXT_MENU_WIDTH,
            elements: (
                button! {
                    text: client_state().localization().ui().context_menu_whisper(),
                    tooltip: whisper_tooltip,
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::WhisperChat { name: whisper_name.clone() });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: client_state().localization().ui().context_menu_add_friend(),
                    tooltip: add_friend_tooltip,
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::AddFriend {
                            character_name: add_friend_name.clone(),
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: client_state().localization().ui().context_menu_invite_to_party(),
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::InviteToParty {
                            name: invite_party_name.clone(),
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: client_state().localization().ui().context_menu_trade(),
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::TradeRequest {
                            account_id: target_account_id,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                browse_shop_button,
                button! {
                    text: client_state().localization().ui().context_menu_view_character_info(),
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::ViewCharacterInfo {
                            name: info_name.clone(),
                            level: info_level,
                            guild_name: info_guild.clone(),
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
            ),
        }
    }
}

/// Selector for the use/equip label of the item context menu.
fn use_equip_label(action: Option<ItemUseAction>) -> impl rust_state::Selector<ClientState, String> {
    korangar_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        let text = client_state().localization().ui();
        match action {
            Some(ItemUseAction::Use) => text.context_menu_use().follow_safe(state).clone(),
            Some(ItemUseAction::Equip(_)) => text.context_menu_equip().follow_safe(state).clone(),
            None => text.context_menu_use_or_equip().follow_safe(state).clone(),
        }
    })
}

/// Selector for the "store one" label of the storage context menu.
fn storage_single_label(action: Option<ItemStorageAction>) -> impl rust_state::Selector<ClientState, String> {
    korangar_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        let text = client_state().localization().ui();
        match action {
            Some(ItemStorageAction::Deposit { .. }) => text.context_menu_store_one().follow_safe(state).clone(),
            Some(ItemStorageAction::Withdraw { .. }) => text.context_menu_take_out_one().follow_safe(state).clone(),
            None => text.context_menu_store_one().follow_safe(state).clone(),
        }
    })
}

/// Selector for the "store all" label of the storage context menu.
fn storage_all_label(action: Option<ItemStorageAction>) -> impl rust_state::Selector<ClientState, String> {
    korangar_interface::selector_helpers::ComputedSelector::new_default(move |state: &ClientState| {
        let text = client_state().localization().ui();
        match action {
            Some(ItemStorageAction::Deposit { .. }) => text.context_menu_store_all().follow_safe(state).clone(),
            Some(ItemStorageAction::Withdraw { .. }) => text.context_menu_take_out_all().follow_safe(state).clone(),
            None => text.context_menu_store_all().follow_safe(state).clone(),
        }
    })
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
    /// Whether the "Identify" action is offered (unidentified inventory item,
    /// `CZ_REQ_ONECLICK_ITEMIDENTIFY` 0x0A35).
    identify: bool,
    /// Whether the "Refine" action is offered (identified, unequipped weapon,
    /// `CZ_REQ_WEAPONREFINE` 0x0222).
    refine: bool,
    /// Whether the "Repair" action is offered (damaged equipment,
    /// `CZ_REQ_ITEMREPAIR1` 0x01FD).
    repair: bool,
    /// Whether the "Insert card" action is offered (card item,
    /// `CZ_REQ_ITEMCOMPOSITION_LIST` 0x017A).
    insert_card: bool,
}

impl ItemContextMenuWindow {
    pub fn new(inventory_index: InventoryIndex, item_id: ItemId, action: Option<ItemUseAction>) -> Self {
        Self {
            inventory_index,
            item_id,
            action,
            storage_action: None,
            source: ItemSource::Inventory,
            identify: false,
            refine: false,
            repair: false,
            insert_card: false,
        }
    }

    /// Enable the item-service entries that apply to the item.
    pub fn with_service_actions(mut self, identify: bool, refine: bool, repair: bool, insert_card: bool) -> Self {
        self.identify = identify;
        self.refine = refine;
        self.repair = repair;
        self.insert_card = insert_card;
        self
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

        let use_equip_label = use_equip_label(self.action);

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

        let storage_single_label = storage_single_label(storage_action);
        let storage_all_label = storage_all_label(storage_action);

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
        let identify_index = self.inventory_index;
        let refine_index = self.inventory_index;
        let repair_index = self.inventory_index;
        let card_index = self.inventory_index;
        let identify = self.identify;
        let refine = self.refine;
        let repair = self.repair;
        let insert_card = self.insert_card;

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
                    disabled_tooltip: client_state().localization().ui().item_not_usable_tooltip(),
                    event: use_equip_event,
                },
                button! {
                    text: "Identify",
                    tooltip: "Identify this item with a Magnifier",
                    disabled: !identify,
                    disabled_tooltip: "This item is already identified",
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::OneClickIdentify {
                            inventory_index: identify_index,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: "Refine weapon",
                    tooltip: "Refine this weapon (requires the Blacksmith refine skill)",
                    disabled: !refine,
                    disabled_tooltip: "Only identified, unequipped weapons can be refined",
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::RefineWeapon {
                            inventory_index: refine_index,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: "Repair item",
                    tooltip: "Repair this damaged item (requires the Repair Weapon skill)",
                    disabled: !repair,
                    disabled_tooltip: "This item is not damaged",
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::RepairItem {
                            inventory_index: repair_index,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: "Insert card",
                    tooltip: "Insert this card into a piece of equipment",
                    disabled: !insert_card,
                    disabled_tooltip: "Only card items can be inserted",
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::RequestCardComposition {
                            card_index,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
                button! {
                    text: client_state().localization().ui().context_menu_item_view_info(),
                    tooltip: client_state().localization().ui().context_menu_item_view_info_tooltip(),
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
                    disabled_tooltip: client_state().localization().ui().storage_kafra_tooltip(),
                    event: storage_single_event,
                },
                button! {
                    text: storage_all_label,
                    disabled: self.storage_action.is_none(),
                    disabled_tooltip: client_state().localization().ui().storage_kafra_tooltip(),
                    event: storage_all_event,
                },
                button! {
                    text: client_state().localization().ui().context_menu_drop(),
                    tooltip: client_state().localization().ui().context_menu_drop_tooltip(),
                    event: move |_: &State<ClientState>, queue: &mut EventQueue<ClientState>| {
                        queue.queue(InputEvent::DropItem {
                            inventory_index: info_index,
                            amount: 1,
                        });
                        queue.queue(InputEvent::CloseContextMenu);
                    },
                },
            ),
        }
    }
}
