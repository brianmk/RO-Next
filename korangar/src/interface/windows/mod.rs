mod audio_settings;
mod big_map;
mod buff_bar;
mod buy;
mod buy_cart;
mod buy_or_sell;
mod cache;
mod cart;
mod character_creation;
mod character_deletion;
mod character_info;
mod character_overview;
mod character_selection;
mod chat;
mod clan;
#[cfg(feature = "debug")]
mod commands;
mod companion;
mod context_menu;
mod dialog;
mod equipment;
mod error;
#[cfg(feature = "debug")]
mod frame_inspector;
mod friend_list;
mod friend_request;
mod game_settings;
mod graphics_settings;
mod guild;
mod hotbar;
mod interface_settings;
mod inventory;
mod item_info;
mod login;
#[cfg(feature = "debug")]
mod maps;
mod menu;
mod minimap;
mod notification_hud;
#[cfg(feature = "debug")]
mod packet_inspector;
mod party;
#[cfg(feature = "debug")]
mod profiler;
mod quest_log;
#[cfg(feature = "debug")]
mod render_options;
mod respawn;
mod rodex;
mod sell;
mod sell_cart;
mod server_selection;
mod skill_description;
mod skill_tree;
mod spirit_spheres;
mod stats;
mod storage;
#[cfg(feature = "debug")]
mod theme_inspector;
mod trade;
mod vending;
mod vending_browse;
mod whisper;

use serde::{Deserialize, Serialize};

pub use self::audio_settings::AudioSettingsWindow;
pub use self::big_map::BigMapWindow;
pub use self::buff_bar::{BAR_STRIP_WIDTH, BuffBarWindow};
pub use self::buy::BuyWindow;
pub use self::buy_cart::BuyCartWindow;
pub use self::buy_or_sell::BuyOrSellWindow;
pub use self::cache::WindowCache;
pub use self::cart::CartWindow;
pub use self::character_creation::CharacterCreationWindow;
pub use self::character_deletion::CharacterDeletionWindow;
pub use self::character_info::CharacterInfoWindow;
pub use self::character_overview::CharacterOverviewWindow;
pub use self::character_selection::{CharacterSelectionWindow, character_selection_window_position};
pub use self::chat::{ChatTextBox, ChatWindow, ChatWindowState};
pub use self::clan::ClanWindow;
#[cfg(feature = "debug")]
pub use self::commands::CommandsWindow;
pub use self::companion::{CompanionInfoWindow, CompanionKind};
pub use self::context_menu::{CharacterContextMenuWindow, ItemContextMenuWindow, ItemStorageAction};
pub use self::dialog::{DialogWindow, DialogWindowState};
pub use self::equipment::EquipmentWindow;
pub use self::error::ErrorWindow;
#[cfg(feature = "debug")]
pub use self::frame_inspector::FrameInspectorWindow;
pub use self::friend_list::{FriendListWindow, FriendListWindowState};
pub use self::friend_request::FriendRequestWindow;
pub use self::game_settings::GameSettingsWindow;
pub use self::graphics_settings::GraphicsSettingsWindow;
pub use self::guild::GuildWindow;
pub use self::hotbar::{HOTBAR_WINDOW_HEIGHT, HOTBAR_WINDOW_WIDTH, HotbarWindow};
pub use self::interface_settings::InterfaceSettingsWindow;
pub use self::inventory::InventoryWindow;
pub use self::item_info::ItemInfoWindow;
pub(crate) use self::item_info::equipped_item_in_slot;
pub use self::login::{LoginWindow, LoginWindowState, LoginWindowStatePathExt};
#[cfg(feature = "debug")]
pub use self::maps::MapsWindow;
pub use self::menu::MenuWindow;
pub use self::minimap::MinimapWindow;
pub use self::notification_hud::{NOTIFICATION_HUD_WIDTH, NotificationHudWindow};
#[cfg(feature = "debug")]
pub use self::packet_inspector::PacketInspectorWindow;
pub use self::party::{PartyInviteWindow, PartyWindow};
#[cfg(feature = "debug")]
pub use self::profiler::{ProfilerWindow, ProfilerWindowState};
pub use self::quest_log::QuestWindow;
#[cfg(feature = "debug")]
pub use self::render_options::RenderOptionsWindow;
pub use self::respawn::RespawnWindow;
pub use self::rodex::MailboxWindow;
pub use self::sell::SellWindow;
pub use self::sell_cart::SellCartWindow;
pub use self::server_selection::ServerSelectionWindow;
pub use self::skill_description::SkillDescriptionWindow;
pub use self::skill_tree::{SkillTreeWindow, SkillTreeWindowState, SkillTreeWindowStatePathExt};
pub use self::spirit_spheres::SpiritSphereWindow;
pub use self::stats::StatsWindow;
pub use self::storage::StorageWindow;
#[cfg(feature = "debug")]
pub use self::theme_inspector::{ThemeInspectorWindow, ThemeInspectorWindowState};
pub use self::trade::{TradeRequestWindow, TradeWindow};
pub use self::vending::VendingWindow;
pub use self::vending_browse::VendingBrowseWindow;
pub use self::whisper::WhisperWindow;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowClass {
    AudioSettings,
    BigMap,
    BuffBar,
    Buy,
    BuyCart,
    BuyOrSell,
    Chat,
    CharacterCreation,
    CharacterDeletion,
    ContextMenu,
    CharacterInfo,
    CharacterOverview,
    CharacterSelection,
    Dialog,
    GameSettings,
    InterfaceSettings,
    GraphicsSettings,
    Hotbar,
    Inventory,
    ItemInfo,
    Equipment,
    SkillTree,
    SkillDescription,
    SpiritSpheres,
    Stats,
    Trade,
    TradeRequest,
    Vending,
    VendingBrowse,
    Cart,
    Storage,
    Whisper,
    FriendList,
    FriendRequest,
    Login,
    Mail,
    Menu,
    Minimap,
    Party,
    Guild,
    Clan,
    CompanionInfo,
    Quest,
    PartyInvite,
    NotificationHud,
    Respawn,
    SelectServer,
    Sell,
    SellCart,
    #[cfg(feature = "debug")]
    Maps,
    #[cfg(feature = "debug")]
    ClientStateInspector,
    #[cfg(feature = "debug")]
    PacketInspector,
    #[cfg(feature = "debug")]
    RenderOptions,
    #[cfg(feature = "debug")]
    Commands,
    #[cfg(feature = "debug")]
    ThemeInspector,
    #[cfg(feature = "debug")]
    Profiler,
    #[cfg(feature = "debug")]
    CacheStatistics,
}

impl WindowClass {
    /// Whether the class is a permanently visible HUD overlay.
    ///
    /// Escape does not close these overlays before it opens the menu: they are
    /// part of the normal in-game HUD (chat, hotbar, minimap, ...) and would
    /// otherwise make Escape feel like it does nothing.
    pub fn is_background_overlay(self) -> bool {
        matches!(
            self,
            Self::Chat
                | Self::Hotbar
                | Self::Minimap
                | Self::CharacterOverview
                | Self::BuffBar
                | Self::SpiritSpheres
                | Self::NotificationHud
        )
    }
}
