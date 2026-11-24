pub mod achievement;
pub mod bank;
#[cfg(feature = "debug")]
pub mod cache_statistics;
pub mod character_previews;
pub mod character_selection_cooldown;
pub mod character_slots;
pub mod chat_markup;
pub mod clan;
pub mod companion;
pub mod content_windows;
pub mod friend_names;
pub mod guild;
pub mod hotbar;
pub mod inventory;
pub mod item_service;
pub mod key_rebind;
pub mod lifecycle;
pub mod localization;
pub mod mail;
pub mod minimap;
pub mod monster_database;
pub mod navigation;
pub mod notifications;
pub mod party;
pub mod party_booking;
pub mod quest_log;
pub mod skill_cooldowns;
pub mod skills;
pub mod spirit_orbs;
pub mod stat_buffer;
pub mod status_effects;
pub mod theme;
pub mod trade;
pub mod vending;
pub mod whisper;

use std::cell::Cell;
use std::sync::Arc;

use korangar_interface::application::Application;
use korangar_interface::components::button::ButtonTheme;
use korangar_interface::components::collapsible::CollapsibleTheme;
use korangar_interface::components::drop_down::DropDownTheme;
use korangar_interface::components::field::FieldTheme;
use korangar_interface::components::slider::SliderTheme;
use korangar_interface::components::state_button::StateButtonTheme;
use korangar_interface::components::text::TextTheme;
use korangar_interface::components::text_box::TextBoxTheme;
use korangar_interface::element::StateElement;
use korangar_interface::layout::tooltip::TooltipTheme;
use korangar_interface::skin::SkinTheme;
use korangar_interface::theme::ThemePathGetter;
use korangar_interface::window::{StateWindow, WindowTheme};
use korangar_networking::{MessageColor, SellItem, ShopItem};
use localization::Localization;
#[cfg(feature = "debug")]
use ragnarok_formats::map::{EffectSource, LightSource, MapData, SoundSource};
use ragnarok_packets::{CharacterId, CharacterServerInformation, EntityId, Friend, TilePosition};
#[cfg(feature = "debug")]
use rust_state::{ManuallyAssertExt, VecIndexExt};
use rust_state::{Path, PathExt, RustState, Selector};
use theme::{InterfaceTheme, InterfaceThemePathExt, InterfaceThemeType};

#[cfg(feature = "debug")]
use self::cache_statistics::CacheStatistics;
use self::item_service::ItemService;
pub use self::status_effects::{StatusEffects, status_effect_definition};
#[cfg(feature = "debug")]
use crate::PacketHistory;
#[cfg(feature = "debug")]
use crate::graphics::RenderOptions;
use crate::graphics::{Color, CornerDiameter, ScreenClip, ScreenPosition, ScreenSize, ShadowPadding};
use crate::input::{InputEvent, MouseInputMode};
use crate::interface::windows::{
    ChatWindowState, DialogWindowState, FriendListWindowState, LoginWindowState, LoginWindowStatePathExt, SkillTreeWindowState,
    WindowCache, WindowClass,
};
#[cfg(feature = "debug")]
use crate::interface::windows::{ProfilerWindowState, ThemeInspectorWindowState};
use crate::loaders::{ClientInfo, FontLoader, FontSize, GameFileLoader, OverflowBehavior, load_client_info};
use crate::renderer::InterfaceRenderer;
use crate::settings::{
    GameSettings, GraphicsSettingsCapabilities, InterfaceSettings, InterfaceSettingsCapabilities, LoginSettings, ServiceSettings,
};
use crate::state::achievement::AchievementState;
use crate::state::bank::Bank;
use crate::state::character_previews::CharacterPreviews;
use crate::state::character_selection_cooldown::CharacterSelectionCooldown;
use crate::state::character_slots::CharacterSlots;
use crate::state::clan::Clan;
use crate::state::companion::Companion;
use crate::state::content_windows::ContentWindows;
use crate::state::guild::Guild;
use crate::state::hotbar::Hotbar;
use crate::state::inventory::Inventory;
use crate::state::key_rebind::KeyRebindState;
use crate::state::mail::Mailbox;
use crate::state::minimap::Minimap;
use crate::state::monster_database::MonsterDatabaseState;
use crate::state::navigation::Navigation;
use crate::state::notifications::NotificationHud;
use crate::state::party::Party;
use crate::state::party_booking::PartyBooking;
use crate::state::quest_log::QuestLog;
use crate::state::skill_cooldowns::SkillCooldowns;
use crate::state::skills::SkillTree;
use crate::state::spirit_orbs::SpiritOrbs;
use crate::state::theme::WorldTheme;
use crate::state::trade::Trade;
use crate::state::vending::{Vending, VendingBrowse};
use crate::state::whisper::WhisperConversations;
#[cfg(feature = "debug")]
use crate::world::Object;
use crate::world::{Entity, GroundItem, Player, ResourceMetadata};
use crate::{AudioSettings, GraphicsSettings};

/// The channel a chat message belongs to, used by the chat window tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, RustState, StateElement)]
pub enum ChatChannel {
    /// Normal ("public") chat, server messages and broadcasts.
    #[default]
    Public,
    /// Party chat (`ZC_NOTIFY_CHAT_PARTY`, 0x0109).
    Party,
    /// Whisper conversation (`ZC_WHISPER`, 0x09DE).
    Whisper,
    /// Guild chat (`ZC_GUILD_CHAT`, 0x017F).
    Guild,
}

impl ChatChannel {
    pub const OPTIONS: [ChatChannel; 4] = [ChatChannel::Public, ChatChannel::Party, ChatChannel::Whisper, ChatChannel::Guild];

    /// Short tab label. Chat channels are server concepts without a
    /// localization entry; the labels are kept in English like the packet
    /// names.
    pub fn label(self) -> &'static str {
        match self {
            ChatChannel::Public => "Public",
            ChatChannel::Party => "Party",
            ChatChannel::Whisper => "Whisper",
            ChatChannel::Guild => "Guild",
        }
    }
}

/// A message in the in-game chat.
///
/// The message stores the color separately rather than baking it into the
/// message so the chat window can use the correct colors when switching themes.
#[derive(Debug, Clone, RustState, StateElement)]
pub struct ChatMessage {
    /// Raw message.
    pub text: String,
    /// Color of the message.
    pub color: MessageColor,
    /// Channel tab the message is shown in.
    pub channel: ChatChannel,
    /// For whisper messages: the name of the conversation partner. Clicking
    /// such a message opens the whisper window (none for all other channels).
    pub whisper_partner: Option<String>,
}

impl ChatMessage {
    pub fn new(text: String, color: MessageColor) -> Self {
        Self {
            text,
            color,
            channel: ChatChannel::Public,
            whisper_partner: None,
        }
    }

    /// Message that is shown in a specific channel tab.
    pub fn new_channel(text: String, color: MessageColor, channel: ChatChannel) -> Self {
        Self {
            text,
            color,
            channel,
            whisper_partner: None,
        }
    }

    /// Whisper message; clicking it opens the conversation with `partner`.
    pub fn whisper(text: String, color: MessageColor, partner: impl Into<String>) -> Self {
        Self {
            text,
            color,
            channel: ChatChannel::Whisper,
            whisper_partner: Some(partner.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, RustState, StateElement)]
pub enum BufferedAction {
    AttackEntity { entity_id: EntityId },
    PickUpItem { entity_id: EntityId },
}

impl BufferedAction {
    pub fn is_attack_entity(&self, entity_id: EntityId) -> bool {
        matches!(self, BufferedAction::AttackEntity { entity_id: buffered_entity_id } if *buffered_entity_id == entity_id)
    }

    pub fn is_pick_up_item(&self, entity_id: EntityId) -> bool {
        matches!(self, BufferedAction::PickUpItem { entity_id: buffered_entity_id } if *buffered_entity_id == entity_id)
    }
}

/// `ZC_MAPPROPERTY_R2` flags that are needed outside the packet layer.
///
/// The bits are defined by rAthena `clif_map_property` (0x099B):
/// bit 0 PvP, bit 1 GvG, bit 2 siege, bit 5 PvP counter, bit 8 costumes
/// disabled. The remaining bits are not consumed by the renderer yet.
pub const MAP_FLAG_PVP: u32 = 1 << 0;
pub const MAP_FLAG_GVG: u32 = 1 << 1;
pub const MAP_FLAG_SIEGE: u32 = 1 << 2;
pub const MAP_FLAG_COUNT_PK: u32 = 1 << 5;
pub const MAP_FLAG_DISABLE_COSTUME: u32 = 1 << 8;

/// `e_map_type` values from `ZC_NOTIFY_MAPPROPERTY2` that are indoors.
const INDOOR_MAP_TYPES: &[u16] = &[
    1,  // MAPTYPE_VILLAGE_IN
    3,  // MAPTYPE_DUNGEON
    9,  // MAPTYPE_DUNGEON2
    10, // MAPTYPE_DUNGEON3
];

/// `e_map_type` values that are PvP/PK/GvG maps.
const PVP_MAP_TYPES: &[u16] = &[
    4,  // MAPTYPE_ARENA
    5,  // MAPTYPE_PENALTY_FREEPKZONE
    6,  // MAPTYPE_NOPENALTY_FREEPKZONE
    7,  // MAPTYPE_EVENT_GUILDWAR
    11, // MAPTYPE_PKSERVER
    12, // MAPTYPE_PVPSERVER
    17, // MAPTYPE_PORINGBATTLE
    20, // MAPTYPE_PVP_TOURNAMENT
];

/// Whether an `e_map_type` (`ZC_NOTIFY_MAPPROPERTY2`) is an indoor map type.
pub fn is_indoor_map_type(map_type: u16) -> bool {
    INDOOR_MAP_TYPES.contains(&map_type)
}

/// Whether an `e_map_type` / `ZC_MAPPROPERTY_R2` flag combination is a
/// PvP/PK/GvG map.
pub fn is_pvp_map(map_type: u16, map_flags: u32) -> bool {
    map_flags & (MAP_FLAG_PVP | MAP_FLAG_GVG | MAP_FLAG_SIEGE) != 0 || PVP_MAP_TYPES.contains(&map_type)
}

/// Whether the `ZC_MAPPROPERTY_R2` flags disable costume sprites.
pub fn costumes_disabled(map_flags: u32) -> bool {
    map_flags & MAP_FLAG_DISABLE_COSTUME != 0
}

/// Internal state of the client. Everything that can be viewed or modified via
/// the user interface should be in here. State that takes care of managing OS
/// or rendering resources should be in [`Client`](super::Client).
#[derive(RustState, StateElement, StateWindow)]
#[cfg_attr(feature = "debug", window_class(WindowClass::ClientStateInspector))]
#[window_title("Client State Inspector")]
#[state_root]
pub struct ClientState {
    /// Localization for the selected language.
    localization: Localization,

    /// Saved settings of previous connections and credentials.
    login_settings: LoginSettings,
    /// Saved audio settings.
    audio_settings: AudioSettings,
    /// Saved game settings.
    game_settings: GameSettings,
    /// Saved interface settings.
    interface_settings: InterfaceSettings,
    /// Interface capabilities used in the interface settings window.
    interface_settings_capabilities: InterfaceSettingsCapabilities,
    /// Saved graphics settings.
    graphics_settings: GraphicsSettings,
    /// Graphics capabilities used in the graphics settings window.
    graphics_settings_capabilities: GraphicsSettingsCapabilities,

    /// The interface theme for the menu windows.
    menu_theme: InterfaceTheme,
    /// The interface theme for in-game windows.
    in_game_theme: InterfaceTheme,
    /// Theme for themeable elements that don't change between the menu
    /// and playing theme.
    world_theme: WorldTheme,

    /// Loaded `sclientinfo.xml`.
    client_info: ClientInfo,
    /// Internal state of the login window.
    login_window: LoginWindowState,
    /// Internal state of the chat window.
    chat_window: ChatWindowState,
    /// State of the chat-room and pincode windows.
    #[hidden_element]
    content_windows: ContentWindows,
    /// Minimap of the currently loaded map.
    minimap: Minimap,
    /// Internal state of the friend list window.
    friend_list_window: FriendListWindowState,
    /// Internal state of the dialog window.
    dialog_window: DialogWindowState,
    /// Internal state of the skill tree window.
    skill_tree_window: SkillTreeWindowState,
    /// Spirit-sphere counts of visible entities (`ZC_SPIRITS*` 0x01D0/0x01E1).
    #[hidden_element]
    spirit_orbs: SpiritOrbs,
    /// Active skill cooldowns (`ZC_SKILL_POSTDELAY`, 0x043D).
    #[hidden_element]
    skill_cooldowns: SkillCooldowns,
    /// Pending key capture of the key binding editor.
    #[hidden_element]
    key_rebind: KeyRebindState,
    /// Map type and flags from `ZC_NOTIFY_MAPPROPERTY2` (0x099B).
    #[hidden_element]
    map_type: u16,
    #[hidden_element]
    map_flags: u32,
    /// Dynamic map-cell overrides from `ZC_UPDATE_MAPINFO` (0x0192): tile ->
    /// walkable. The loaded map is shared/immutable, so the override is kept
    /// here (see the noop report for the collision follow-up).
    #[hidden_element]
    map_cell_overrides: Vec<(TilePosition, bool)>,

    /// All entities on the map.
    entities: Vec<Entity>,
    /// All dead entities on the map.
    dead_entities: Vec<Entity>,
    /// All ground items on the map.
    ground_items: Vec<GroundItem>,

    /// List of all received chat messages.
    chat_messages: Vec<ChatMessage>,
    /// Whisper conversations (bounded history per partner).
    #[hidden_element]
    whisper: WhisperConversations,
    /// Transient notification HUD entries (broadcast/pickup/level-up/quest).
    #[hidden_element]
    notifications: NotificationHud,
    /// Active status effects (buffs/debuffs) of the player shown in the buff
    /// bar.
    status_effects: StatusEffects,
    /// Party state (roster, HP, invitations) of the 2019 party system.
    party: Party,
    /// Guild state (basic info, members, positions, notice) of the 2019 guild
    /// system.
    guild: Guild,
    /// Party-booking board state (the 2019 "party search" / LFG UI).
    party_booking: PartyBooking,
    /// Clan state (basic info and online count) of the 2019 clan system.
    clan: Clan,
    /// Pet / homunculus / mercenary information of the local player's units.
    companion: Companion,
    /// List of all friends.
    friend_list: Vec<Friend>,
    /// Online state of each entry of `friend_list` (same length/order).
    friend_online: Vec<bool>,
    /// List of items offered in the shop.
    // TODO: Unhide this
    #[hidden_element]
    shop_items: Vec<ShopItem<ResourceMetadata>>,
    /// List of items in the buying cart.
    // TODO: Unhide this
    #[hidden_element]
    buy_cart: Vec<ShopItem<(ResourceMetadata, u32)>>,
    /// List of items that should be sold.
    // TODO: Unhide this
    #[hidden_element]
    sell_items: Vec<SellItem<(ResourceMetadata, u16)>>,
    /// List of items in the selling cart.
    // TODO: Unhide this
    #[hidden_element]
    sell_cart: Vec<SellItem<(ResourceMetadata, u16)>>,
    /// The name of the active character. This information is not available
    /// while playing if we don't save it here.
    player_name: String,
    /// Player configured hotbar.
    hotbar: Hotbar,
    /// Player inventory.
    inventory: Inventory,
    /// State of the shared refine/repair/identify/card-insert window
    /// (`ZC_NOTIFY_WEAPONITEMLIST`, `ZC_REPAIRITEMLIST`,
    /// `ZC_ITEMIDENTIFY_LIST`, `ZC_ITEMCOMPOSITION_LIST`).
    #[hidden_element]
    item_service: ItemService,
    /// Current player trade.
    #[hidden_element]
    trade: Trade,
    /// Active quests shown in the quest tracker.
    #[hidden_element]
    quest_log: QuestLog,
    /// Achievement log and reward claim state (`ZC_ALL_ACH_LIST` /
    /// `ZC_ACH_UPDATE`).
    achievement: AchievementState,
    /// Client-side navigation destination, route and search field.
    navigation: Navigation,
    /// Monster database search field and last Sense (`ZC_MONSTER_INFO`)
    /// result.
    monster_database: MonsterDatabaseState,
    /// Rodex mailbox (`ZC_ACK_MAIL_LIST3` 0x0AC2 and the rest of the 2019 mail
    /// family).
    #[hidden_element]
    mail: Mailbox,
    /// Current vending-shop editor (`MC_VENDING` / `ZC_OPENSTORE`).
    #[hidden_element]
    vending: Vending,
    /// The vending shop of another player that is shown in the browse window
    /// (`ZC_PC_PURCHASE_ITEMLIST_FROMMC`, 0x0800).
    #[hidden_element]
    vending_browse: VendingBrowse,
    /// Items in the merchant pushcart. rAthena sends the list through the
    /// shared inventory list family with `inventory_type` 1
    /// (`clif_cartlist`, `clif.cpp:3226`).
    cart: Inventory,
    /// Amount of zeny the character has, `None` until the server sent the first
    /// zeny update (the initial zeny is not necessarily part of the login
    /// data).
    #[hidden_element]
    zeny: Option<u64>,
    /// Bank window state (`CZ_REQ_BANKING_*`, rAthena banking system).
    #[hidden_element]
    bank: Bank,
    /// Items in the Kafra storage, filled while a storage window is open.
    storage: Inventory,
    /// Current number of items in the Kafra storage
    /// (`ZC_NOTIFY_STOREITEM_COUNTINFO`).
    storage_amount: u16,
    /// Maximum number of items the Kafra storage can hold.
    storage_maximum_amount: u16,
    /// Name of the currently open storage as sent by the server (e.g.
    /// "Storage").
    storage_name: String,

    /// Player skill tree.
    skill_tree: SkillTree,

    /// List of all available character servers.
    character_servers: Vec<CharacterServerInformation>,
    /// List of all the slots and characters on the current character server.
    character_slots: CharacterSlots,
    /// Idle preview animations of the characters in the character selection
    /// window. Filled asynchronously whenever the window is open; the value is
    /// managed by the client and not part of the interface layout.
    #[hidden_element]
    character_previews: CharacterPreviews,
    /// Anti-crash grace period that blocks character selection for a moment
    /// after logging out (the server may still be cleaning up the old session).
    #[hidden_element]
    character_selection_cooldown: CharacterSelectionCooldown,
    /// Id of the character that is currently being deleted. The server does not
    /// send back information about which character was deleted
    /// successfully, so we store it instead.
    currently_deleting: Option<CharacterId>,
    /// Slot of the character currently being moved.
    switch_request: Option<usize>,
    /// Name of the character being created currently.
    create_character_name: String,
    /// Account e-mail address typed into the character deletion confirmation
    /// window. The character server (rAthena `char_del_option: 1`) requires the
    /// account e-mail as the deletion confirmation code.
    delete_character_email: String,

    /// Size of the Korangar window.
    window_size: ScreenSize,

    /// Buffered player action. For example, attacking a target that is out of
    /// range.
    buffered_action: Option<BufferedAction>,

    /// Map data that is viewed in the inspector. Once added to this vector they
    /// are never removed so we can ensure the user interface remains valid.
    #[cfg(feature = "debug")]
    #[hidden_element]
    inspecting_maps: Vec<MapData>,
    /// Objects that are viewed in the inspector. Once added to this vector they
    /// are never removed so we can ensure the user interface remains valid.
    #[cfg(feature = "debug")]
    inspecting_objects: Vec<Object>,
    /// Light sources that are viewed in the inspector. Once added to this
    /// vector they are never removed so we can ensure the user interface
    /// remains valid.
    #[cfg(feature = "debug")]
    inspecting_light_sources: Vec<LightSource>,
    /// Sound sources that are viewed in the inspector. Once added to this
    /// vector they are never removed so we can ensure the user interface
    /// remains valid.
    #[cfg(feature = "debug")]
    inspecting_sound_sources: Vec<SoundSource>,
    /// Effect sources that are viewed in the inspector. Once added to this
    /// vector they are never removed so we can ensure the user interface
    /// remains valid.
    #[cfg(feature = "debug")]
    inspecting_effect_sources: Vec<EffectSource>,
    /// Special render options for debugging the client.
    #[cfg(feature = "debug")]
    render_options: RenderOptions,
    /// Internal state of the profiler window.
    #[cfg(feature = "debug")]
    profiler_window: ProfilerWindowState,
    /// Internal state of the theme inspector window.
    #[cfg(feature = "debug")]
    theme_inspector_window: ThemeInspectorWindowState,
    /// List of packets sent and received for the packet inspector. Also
    /// contains information about which packets to display in the
    /// inspector.
    #[cfg(feature = "debug")]
    packet_history: PacketHistory,
    /// Statistics of all caches of the loaders.
    #[cfg(feature = "debug")]
    cache_statistics: CacheStatistics,
}

impl ClientState {
    /// Number of spirit spheres of the local player, `0` when there is no
    /// player or no `ZC_SPIRITS`/`ZC_SPIRITS2` update was received yet.
    pub fn own_spirit_orbs(&self) -> i16 {
        self.entities
            .first()
            .map(|entity| self.spirit_orbs.get(entity.get_entity_id()))
            .unwrap_or(0)
    }

    /// Whether the current map is indoors (`ZC_NOTIFY_MAPPROPERTY2` map type).
    ///
    /// Indoor maps do not draw the outdoor weather effects; their background
    /// and lighting come from the map's own RSW/GND/mapskydata entries.
    pub fn is_indoor_map(&self) -> bool {
        is_indoor_map_type(self.map_type)
    }

    /// Whether the current map is a PvP/PK/GvG map (`ZC_MAPPROPERTY_R2` flags
    /// and the map type).
    ///
    /// The flag is exposed for the future targeting rules (attack cursor on
    /// non-party/non-guild members); the renderer does not change behaviour
    /// based on it yet.
    pub fn is_pvp_map(&self) -> bool {
        is_pvp_map(self.map_type, self.map_flags)
    }

    /// Whether the server disabled costume sprites on the current map
    /// (`ZC_MAPPROPERTY_R2` bit 8).
    pub fn costumes_disabled(&self) -> bool {
        costumes_disabled(self.map_flags)
    }

    /// Dynamic map-cell overrides from `ZC_UPDATE_MAPINFO` (tile -> walkable).
    pub fn map_cell_overrides(&self) -> &[(TilePosition, bool)] {
        &self.map_cell_overrides
    }

    pub fn new(
        game_file_loader: &GameFileLoader,
        graphics_settings: GraphicsSettings,
        #[cfg(feature = "debug")] packet_history: PacketHistory,
    ) -> Self {
        time_phase!("load settings", {
            let mut login_settings = LoginSettings::new();
            let audio_settings = AudioSettings::new();
            let game_settings = GameSettings::new();
            let interface_settings = InterfaceSettings::new();
            let mut interface_settings_capabilities = InterfaceSettingsCapabilities::default();
            // The scaling can be changed through the environment, make sure the drop
            // down always has an option for the selected value.
            interface_settings_capabilities.ensure_scaling(interface_settings.scaling);
        });

        time_phase!("load localization", {
            let localization = Localization::load_language(game_file_loader, interface_settings.language);
        });

        time_phase!("load themes", {
            let menu_theme = InterfaceTheme::load(InterfaceThemeType::Menu, &interface_settings.menu_theme);
            let in_game_theme = InterfaceTheme::load(InterfaceThemeType::InGame, &interface_settings.in_game_theme);
            let world_theme = WorldTheme::load(&interface_settings.world_theme);
        });

        time_phase!("create login window state", {
            let client_info = load_client_info(game_file_loader);

            let selected_service = login_settings
                .recent_service_id
                // Make sure that the recent service id is still valid.
                .and_then(|service_id| {
                    client_info
                        .services
                        .iter()
                        .any(|service| service.service_id() == service_id)
                        .then_some(service_id)
                })
                // If there is no recent service id or it was no longer valid, select the first
                // service instead.
                .or_else(|| Some(client_info.services.first()?.service_id()))
                .expect("There are no services available. Check your sclientinfo.yaml.");

            // Make sure that every service has a service settings entry. Without a service
            // settings entry the login window will panic.
            for service in &client_info.services {
                login_settings.service_settings.entry(service.service_id()).or_default();
            }

            let login_window = LoginWindowState::new(selected_service);
        });

        time_phase!("create window state", {
            let welcome_string = format!(
                "Welcome to ^ff8800{}^000000 version ^ff8800{}^000000!",
                crate::CLIENT_NAME,
                env!("CARGO_PKG_VERSION")
            );
            let chat_messages = vec![ChatMessage::new(welcome_string, MessageColor::Server)];
            let whisper = WhisperConversations::default();
            let notifications = NotificationHud::default();

            let chat_window = ChatWindowState::default();
            let content_windows = ContentWindows::default();
            let minimap = Minimap::default();
            let spirit_orbs = SpiritOrbs::default();
            let skill_cooldowns = SkillCooldowns::default();
            let key_rebind = KeyRebindState::default();
            let map_type = 0;
            let map_flags = 0;
            let map_cell_overrides = Vec::new();
        });

        time_phase!("create character server resources", {
            // TODO: This could be in a single struct.
            let character_servers = Vec::new();
            // TODO: This could be in a single struct.
            let character_slots = CharacterSlots::default();
            let character_previews = CharacterPreviews::default();
            let character_selection_cooldown = CharacterSelectionCooldown::default();
            // TODO: This could be in a single struct.
            let currently_deleting = None;
            // TODO: This could be in a single struct.
            let switch_request = None;
            let create_character_name = String::new();
            let delete_character_email = String::new();
        });

        time_phase!("create party state", {
            let party = Party::default();
        });

        time_phase!("create guild state", {
            let guild = Guild::default();
        });

        time_phase!("create party booking state", {
            let party_booking = PartyBooking::default();
        });

        time_phase!("create clan state", {
            let clan = Clan::default();
        });

        time_phase!("create companion state", {
            let companion = Companion::default();
        });

        time_phase!("create friend list state", {
            let friend_list = Vec::default();
            let friend_online = Vec::default();
            let friend_list_window = FriendListWindowState::default();
        });

        time_phase!("create player resources", {
            let dialog_window = DialogWindowState::default();

            let shop_items = Vec::default();
            let buy_cart = Vec::default();
            let sell_items = Vec::default();
            let sell_cart = Vec::default();
            let player_name = String::new();
            let hotbar = Hotbar::default();
            let inventory = Inventory::default();
            let zeny = None;
            let bank = Bank::default();
            let cart = Inventory::default();
            let vending_browse = VendingBrowse::default();
            let storage = Inventory::default();
            let storage_amount = 0;
            let storage_maximum_amount = 0;
            let storage_name = String::new();

            let skill_tree = SkillTree::default();
            let skill_tree_window = SkillTreeWindowState::default();
        });

        time_phase!("create window resources", {
            let window_size = ScreenSize::default();
            let mut graphics_settings_capabilities = GraphicsSettingsCapabilities::default();
            // The resolution can be changed through the environment, make sure the drop
            // down always has an option for the selected value.
            graphics_settings_capabilities.ensure_window_resolution(graphics_settings.window_resolution);
        });

        let buffered_action = None;

        #[cfg(feature = "debug")]
        let debug_timer = korangar_debug::logging::Timer::new("creating debug resources");

        #[cfg(feature = "debug")]
        let inspecting_maps = Vec::new();
        #[cfg(feature = "debug")]
        let inspecting_objects = Vec::new();
        #[cfg(feature = "debug")]
        let inspecting_light_sources = Vec::new();
        #[cfg(feature = "debug")]
        let inspecting_sound_sources = Vec::new();
        #[cfg(feature = "debug")]
        let inspecting_effect_sources = Vec::new();

        #[cfg(feature = "debug")]
        let render_options = RenderOptions::new();

        #[cfg(feature = "debug")]
        let profiler_window = ProfilerWindowState::default();
        #[cfg(feature = "debug")]
        let theme_inspector_window = ThemeInspectorWindowState::default();

        #[cfg(feature = "debug")]
        let cache_statistics = CacheStatistics::default();

        #[cfg(feature = "debug")]
        debug_timer.stop();

        ClientState {
            localization,
            login_settings,
            audio_settings,
            game_settings,
            interface_settings,
            interface_settings_capabilities,
            graphics_settings,
            graphics_settings_capabilities,
            menu_theme,
            in_game_theme,
            world_theme,
            client_info,
            login_window,
            chat_window,
            content_windows,
            minimap,
            friend_list_window,
            dialog_window,
            skill_tree_window,
            spirit_orbs,
            skill_cooldowns,
            key_rebind,
            map_type,
            map_flags,
            map_cell_overrides,
            entities: Vec::new(),
            dead_entities: Vec::new(),
            ground_items: Vec::new(),
            chat_messages,
            whisper,
            notifications,
            status_effects: StatusEffects::default(),
            party,
            guild,
            party_booking,
            clan,
            companion,
            friend_list,
            friend_online,
            shop_items,
            buy_cart,
            sell_items,
            sell_cart,
            player_name,
            hotbar,
            inventory,
            item_service: ItemService::default(),
            quest_log: QuestLog::default(),
            achievement: AchievementState::default(),
            navigation: Navigation::default(),
            monster_database: MonsterDatabaseState::default(),
            mail: Mailbox::default(),
            trade: Trade::default(),
            vending: Vending::default(),
            vending_browse,
            cart,
            zeny,
            bank,
            storage,
            storage_amount,
            storage_maximum_amount,
            storage_name,

            skill_tree,
            character_servers,
            character_slots,
            character_previews,
            character_selection_cooldown,
            currently_deleting,
            switch_request,
            create_character_name,
            delete_character_email,
            window_size,
            buffered_action,
            #[cfg(feature = "debug")]
            inspecting_maps,
            #[cfg(feature = "debug")]
            inspecting_objects,
            #[cfg(feature = "debug")]
            inspecting_light_sources,
            #[cfg(feature = "debug")]
            inspecting_sound_sources,
            #[cfg(feature = "debug")]
            inspecting_effect_sources,
            #[cfg(feature = "debug")]
            render_options,
            #[cfg(feature = "debug")]
            profiler_window,
            #[cfg(feature = "debug")]
            theme_inspector_window,
            #[cfg(feature = "debug")]
            packet_history,
            #[cfg(feature = "debug")]
            cache_statistics,
        }
    }
}

/// Thread local used to create a path without arguments that points to the
/// current theme when creating the layout for a given window.
#[thread_local]
static CURRENT_THEME: Cell<InterfaceThemeType> = Cell::new(InterfaceThemeType::InGame);

/// Path resolving to the selected theme for the window.
#[derive(Clone, Copy)]
pub struct ThemePath;

impl Path<ClientState, InterfaceTheme> for ThemePath {
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a InterfaceTheme> {
        match CURRENT_THEME.get() {
            InterfaceThemeType::Menu => Some(&state.menu_theme),
            InterfaceThemeType::InGame => Some(&state.in_game_theme),
        }
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut InterfaceTheme> {
        match CURRENT_THEME.get() {
            InterfaceThemeType::Menu => Some(&mut state.menu_theme),
            InterfaceThemeType::InGame => Some(&mut state.in_game_theme),
        }
    }
}

impl Selector<ClientState, InterfaceTheme> for ThemePath {
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InterfaceTheme> {
        self.follow(state)
    }
}

/// Glue between our [`InterfaceTheme`] and [`korangar_interface`].
#[derive(Clone, Copy)]
pub struct ClientThemeGetter;

impl ThemePathGetter<ClientState> for ClientThemeGetter {
    fn new() -> Self {
        Self
    }

    fn window(self) -> impl Path<ClientState, WindowTheme<ClientState>> {
        ThemePath.window()
    }

    fn text(self) -> impl Path<ClientState, TextTheme<ClientState>> {
        ThemePath.text()
    }

    fn button(self) -> impl Path<ClientState, ButtonTheme<ClientState>> {
        ThemePath.button()
    }

    fn state_button(self) -> impl Path<ClientState, StateButtonTheme<ClientState>> {
        ThemePath.state_button()
    }

    fn text_box(self) -> impl Path<ClientState, TextBoxTheme<ClientState>> {
        ThemePath.text_box()
    }

    fn collapsible(self) -> impl Path<ClientState, CollapsibleTheme<ClientState>> {
        ThemePath.collapsible()
    }

    fn drop_down(self) -> impl Path<ClientState, DropDownTheme<ClientState>> {
        ThemePath.drop_down()
    }

    fn slider(self) -> impl Path<ClientState, SliderTheme<ClientState>> {
        ThemePath.slider()
    }

    fn field(self) -> impl Path<ClientState, FieldTheme<ClientState>> {
        ThemePath.field()
    }

    fn tooltip(self) -> impl Path<ClientState, TooltipTheme<ClientState>> {
        ThemePath.tooltip()
    }

    fn skin(self) -> impl Path<ClientState, SkinTheme<ClientState>> {
        ThemePath.skin()
    }
}

impl Application for ClientState {
    type Cache = WindowCache;
    type Clip = ScreenClip;
    type Color = Color;
    type CornerDiameter = CornerDiameter;
    type CustomEvent = InputEvent;
    type CustomMouseMode = MouseInputMode;
    type FontSize = FontSize;
    type Image = crate::renderer::skin::SkinImage;
    type OverflowBehavior = OverflowBehavior;
    type Position = ScreenPosition;
    type Renderer = InterfaceRenderer;
    type ShadowPadding = ShadowPadding;
    type Size = ScreenSize;
    type TextLayouter = Arc<FontLoader>;
    type ThemeGetter = ClientThemeGetter;
    type ThemeType = InterfaceThemeType;
    type WindowClass = WindowClass;

    fn set_current_theme_type(theme: InterfaceThemeType) {
        CURRENT_THEME.set(theme);
    }

    fn set_window_chrome_visible(window_chrome_visible: bool) {
        WINDOW_CHROME_VISIBLE.store(window_chrome_visible, std::sync::atomic::Ordering::Relaxed);
    }

    fn window_chrome_visible() -> bool {
        WINDOW_CHROME_VISIBLE.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn is_background_window_class(window_class: Option<WindowClass>) -> bool {
        window_class.is_some_and(WindowClass::is_background_overlay)
    }

    fn localized_text<'a>(state: &'a rust_state::State<ClientState>, key: &'static str) -> Option<&'a str> {
        let localization: &crate::state::localization::Localization = state.get(&crate::state::localization::LOCALIZATION_PATH);
        localization.dropdown_text(key)
    }
}

static WINDOW_CHROME_VISIBLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Path to the [`ClientState`] root.
pub fn client_state() -> impl Path<ClientState, ClientState> {
    ClientState::path()
}

/// Path to the current theme, same as [`korangar_interface::theme::theme`] but
/// more strongly typed so we can use the additional fields.
pub fn client_theme() -> impl Path<ClientState, InterfaceTheme> {
    ThemePath
}

/// Path to the player as [`Player`].
pub fn this_player() -> impl Path<ClientState, Player, false> {
    #[derive(Clone, Copy)]
    struct CustomPath;

    impl Selector<ClientState, Player, false> for CustomPath {
        fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a Player> {
            self.follow(state)
        }
    }

    impl Path<ClientState, Player, false> for CustomPath {
        fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a Player> {
            // TODO: Select our player better.
            match state.entities.first()? {
                Entity::Player(player) => Some(player),
                _ => unreachable!(),
            }
        }

        fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut Player> {
            // TODO: Select our player better.
            match state.entities.first_mut()? {
                Entity::Player(player) => Some(player),
                _ => unreachable!(),
            }
        }
    }

    CustomPath
}

/// Path to the player as [`Entity`].
pub fn this_entity() -> impl Path<ClientState, Entity, false> {
    #[derive(Clone, Copy)]
    struct CustomPath;

    impl Selector<ClientState, Entity, false> for CustomPath {
        fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a Entity> {
            self.follow(state)
        }
    }

    impl Path<ClientState, Entity, false> for CustomPath {
        fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a Entity> {
            // TODO: Select our player better.
            state.entities.first()
        }

        fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut Entity> {
            // TODO: Select our player better.
            state.entities.first_mut()
        }
    }

    CustomPath
}

pub struct SelectedServicePath<P, S> {
    window_state_path: P,
    service_settings_path: S,
}

impl<P, S> SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    pub fn new(window_state_path: P, service_settings_path: S) -> Self {
        Self {
            window_state_path,
            service_settings_path,
        }
    }
}

impl<P, S> Clone for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<P, S> Copy for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
}

impl<P, S> Selector<ClientState, ServiceSettings> for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a ServiceSettings> {
        self.follow(state)
    }
}

impl<P, S> Path<ClientState, ServiceSettings> for SelectedServicePath<P, S>
where
    P: Path<ClientState, LoginWindowState>,
    S: Path<ClientState, LoginSettings>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a ServiceSettings> {
        let selected_service_path = self.window_state_path.selected_service();
        let selected_service = selected_service_path.follow_safe(state);

        self.service_settings_path.follow_safe(state).service_settings.get(selected_service)
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut ServiceSettings> {
        let selected_service_path = self.window_state_path.selected_service();
        let selected_service = *selected_service_path.follow_mut_safe(state);

        self.service_settings_path
            .follow_mut_safe(state)
            .service_settings
            .get_mut(&selected_service)
    }
}

#[cfg(feature = "debug")]
pub fn prepare_map_inspection(inspecting_maps: &mut Vec<MapData>, map_data: &MapData) -> impl Path<ClientState, MapData> {
    let index = inspecting_maps
        .iter()
        .position(|item| {
            item.ground_file == map_data.ground_file
                && item._ini_file == map_data._ini_file
                && item.gat_file == map_data.gat_file
                && item.build_version == map_data.build_version
        })
        .unwrap_or_else(|| {
            let index = inspecting_maps.len();
            inspecting_maps.push(map_data.clone());
            index
        });

    client_state().inspecting_maps().index(index).manually_asserted()
}

#[cfg(feature = "debug")]
pub fn prepare_object_inspection(inspecting_objects: &mut Vec<Object>, object: &Object) -> impl Path<ClientState, Object> {
    let index = inspecting_objects
        .iter()
        .position(|item| item.name == object.name && item.model_name == object.model_name && item.transform == object.transform)
        .unwrap_or_else(|| {
            let index = inspecting_objects.len();
            inspecting_objects.push(object.clone());
            index
        });

    client_state().inspecting_objects().index(index).manually_asserted()
}

#[cfg(feature = "debug")]
pub fn prepare_light_source_inspection(
    inspecting_light_sources: &mut Vec<LightSource>,
    light_source: &LightSource,
) -> impl Path<ClientState, LightSource> {
    let index = inspecting_light_sources
        .iter()
        .position(|item| item == light_source)
        .unwrap_or_else(|| {
            let index = inspecting_light_sources.len();
            inspecting_light_sources.push(light_source.clone());
            index
        });

    client_state().inspecting_light_sources().index(index).manually_asserted()
}

#[cfg(feature = "debug")]
pub fn prepare_sound_source_inspection(
    inspecting_sound_sources: &mut Vec<SoundSource>,
    sound_source: &SoundSource,
) -> impl Path<ClientState, SoundSource> {
    let index = inspecting_sound_sources
        .iter()
        .position(|item| item == sound_source)
        .unwrap_or_else(|| {
            let index = inspecting_sound_sources.len();
            inspecting_sound_sources.push(sound_source.clone());
            index
        });

    client_state().inspecting_sound_sources().index(index).manually_asserted()
}

#[cfg(feature = "debug")]
pub fn prepare_effect_source_inspection(
    inspecting_effect_sources: &mut Vec<EffectSource>,
    effect_source: &EffectSource,
) -> impl Path<ClientState, EffectSource> {
    let index = inspecting_effect_sources
        .iter()
        .position(|item| item == effect_source)
        .unwrap_or_else(|| {
            let index = inspecting_effect_sources.len();
            inspecting_effect_sources.push(effect_source.clone());
            index
        });

    client_state().inspecting_effect_sources().index(index).manually_asserted()
}

#[cfg(test)]
mod map_property_tests {
    use super::{MAP_FLAG_DISABLE_COSTUME, MAP_FLAG_GVG, MAP_FLAG_PVP, costumes_disabled, is_indoor_map_type, is_pvp_map};

    #[test]
    fn indoor_map_types_follow_the_protocol() {
        // Village interior and the three dungeon types are indoors.
        for map_type in [1, 3, 9, 10] {
            assert!(is_indoor_map_type(map_type), "map type {map_type}");
        }
        for map_type in [0, 2, 4, 5, 7, 12, 17, 20] {
            assert!(!is_indoor_map_type(map_type), "map type {map_type}");
        }
    }

    #[test]
    fn pvp_comes_from_the_flags_or_the_map_type() {
        assert!(is_pvp_map(0, MAP_FLAG_PVP));
        assert!(is_pvp_map(0, MAP_FLAG_GVG));
        assert!(is_pvp_map(12, 0), "MAPTYPE_PVPSERVER");
        assert!(is_pvp_map(4, 0), "MAPTYPE_ARENA");
        assert!(is_pvp_map(17, 0), "MAPTYPE_PORINGBATTLE");
        assert!(!is_pvp_map(0, 0));
        assert!(!is_pvp_map(2, 0), "a field map is not PvP");
        // Unrelated flags do not turn a map into PvP.
        assert!(!is_pvp_map(0, MAP_FLAG_DISABLE_COSTUME));
    }

    #[test]
    fn costume_flag_is_exposed() {
        assert!(costumes_disabled(MAP_FLAG_DISABLE_COSTUME));
        assert!(!costumes_disabled(MAP_FLAG_PVP));
        assert!(!costumes_disabled(0));
    }
}
