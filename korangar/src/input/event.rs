#[cfg(feature = "debug")]
use cgmath::Vector2;
#[cfg(feature = "debug")]
use korangar_debug::profiling::FrameMeasurement;
use korangar_interface::event::{ClickHandler, Event, EventQueue};
use korangar_networking::{InventoryItem, ShopItem};
use ragnarok_packets::{
    AccountId, AttackRange, BuyOrSellOption, CharacterId, CharacterServerInformation, EntityId, EquipPosition, HotbarSlot, InventoryIndex,
    ItemId, PartyId, ShopId, SkillId, SkillLevel, SkillType, SoldItemInformation, StatUpType, TilePosition,
};
use rust_state::State;

use crate::graphics::ScreenPosition;
use crate::interface::resource::{ItemSource, SkillSource};
use crate::loaders::ServiceId;
use crate::state::ClientState;
use crate::state::skills::LearnableSkill;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::ResourceMetadata;

/// An event triggered by the user through mouse or keyboard input.
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// Log in to the login server.
    LogIn {
        /// Id of the selected service.
        service_id: ServiceId,
        /// Account username.
        username: String,
        /// Account password.
        password: String,
    },
    /// Select a character server.
    SelectServer {
        /// Selected character server.
        character_server_information: CharacterServerInformation,
    },
    /// Respawn the player.
    Respawn,
    /// Log out of the map server.
    LogOut,
    /// Log out of the character server.
    LogOutCharacter,
    /// Exit Korangar.
    Exit,
    /// Zoom the player camera.
    ZoomCamera {
        /// Amount to zoom.
        zoom_factor: f32,
    },
    /// Rotate the player camera.
    RotateCamera {
        /// Amount of rotation.
        rotation: f32,
    },
    /// Reset the player camera rotation.
    ResetCameraRotation,
    /// Open or close the menu window. Only works while playing.
    ToggleMenuWindow,
    /// Open or close the inventory window. Only works while playing.
    ToggleInventoryWindow,
    /// Open or close the equipment window. Only works while playing.
    ToggleEquipmentWindow,
    /// Open or close the skill tree window. Only works while playing.
    ToggleSkillTreeWindow,
    /// Open or close the stats window. Only works while playing.
    ToggleStatsWindow,
    /// Open or close the game settings window.
    ToggleGameSettingsWindow,
    /// Open or close the interface settings window.
    ToggleInterfaceSettingsWindow,
    /// Open or close the graphics settings window.
    ToggleGraphicsSettingsWindow,
    /// Open or close the audio settings window.
    ToggleAudioSettingsWindow,
    /// Open or close the friend list window. Only works while playing.
    ToggleFriendListWindow,
    /// Open the character information window for another player.
    ViewCharacterInfo {
        name: String,
        level: u16,
        guild_name: Option<String>,
    },
    /// Request a trade with another player.
    TradeRequest {
        account_id: u32,
    },
    /// Accept a pending trade request.
    TradeAccept {
        account_id: u32,
    },
    /// Decline a pending trade request.
    TradeDecline,
    /// Offer an item in the current trade.
    TradeAddItem {
        inventory_index: u16,
        amount: u16,
    },
    /// Offer the zeny amount in the trade window's zeny field.
    TradeOfferZeny,
    /// Lock the local trade side.
    TradeLock,
    /// Commit the trade.
    TradeCommit,
    /// Cancel the trade.
    TradeCancel,
    /// Open or close the quest tracker window.
    ToggleQuestWindow,
    /// Open or close the big map window. Only works while playing.
    ToggleBigMapWindow,
    /// Open or close the chat window. Only works while playing.
    ToggleChatWindow,
    ToggleMinimapWindow,
    CycleMapWindows,
    /// Close the most recently opened or clicked closable window.
    CloseTopWindow,
    /// Submit the open vending shop (`CZ_REQ_OPENSTORE`).
    VendingOpen,
    /// Cancel the vending shop editor (`CZ_REQ_OPENSTORE2`).
    VendingCancel,
    /// Toggle if the user interface should be rendered or not.
    ToggleShowInterface,
    /// Select a character to start playing.
    SelectCharacter {
        /// Slot that the selected character is in.
        slot: usize,
    },
    /// Open a window to create a new character.
    OpenCharacterCreationWindow {
        /// Slot in which to create the new character.
        slot: usize,
    },
    /// Create a new character.
    CreateCharacter {
        /// Slot in which to create the new character.
        slot: usize,
        /// Name of the new character.
        name: String,
    },
    /// Open the confirmation window for deleting a character. The character
    /// server requires the account e-mail address as the confirmation code
    /// (`char_del_option: 1`), so the user has to type it in.
    OpenCharacterDeleteWindow {
        /// Id of the character to be deleted.
        character_id: CharacterId,
    },
    /// Delete a character.
    DeleteCharacter {
        /// Id of the character to be deleted.
        character_id: CharacterId,
        /// Account e-mail address used as the deletion confirmation code.
        email: String,
    },
    /// Switch the characters of two slots.
    SwitchCharacterSlot {
        /// First slot.
        origin_slot: usize,
        /// Second slot.
        destination_slot: usize,
    },
    /// Open or close the party window.
    TogglePartyWindow,
    /// Create a new party with the given name.
    CreateParty {
        /// Name of the party.
        name: String,
    },
    /// Invite a character to the current party by name.
    InviteToParty {
        /// Name of the character to invite.
        name: String,
    },
    /// Accept the pending party invitation.
    AcceptPartyInvite {
        /// Id of the party that invited the player.
        party_id: PartyId,
    },
    /// Reject the pending party invitation.
    RejectPartyInvite {
        /// Id of the party that invited the player.
        party_id: PartyId,
    },
    /// Leave the current party.
    LeaveParty,
    /// Start moving the player.
    PlayerMove {
        /// Destination of the move.
        destination: TilePosition,
    },
    /// Interact with an entity. The type of interaction depends on the entity
    /// type.
    PlayerInteract {
        /// Id of the entity to interact with.
        entity_id: EntityId,
    },
    /// Open the character context menu (right click menu) for the given
    /// character at the given screen position.
    OpenCharacterContextMenu {
        /// Id of the character entity.
        entity_id: EntityId,
        /// Screen position at which the menu should be opened.
        position: ScreenPosition,
    },
    /// Close the character context menu.
    CloseContextMenu,
    /// Start a whisper to the given character by opening the chat and
    /// pre-filling the whisper command.
    WhisperChat {
        /// Name of the character to whisper to.
        name: String,
    },
    /// Open the context menu of an inventory item (right click on the item).
    OpenInventoryItemMenu {
        /// Inventory index of the item.
        inventory_index: InventoryIndex,
    },
    /// Use an item from the inventory (potions, usable items, ...).
    UseInventoryItem {
        /// Inventory index of the item.
        inventory_index: InventoryIndex,
        /// Item id of the item.
        item_id: ItemId,
    },
    /// Equip an item from the inventory.
    EquipInventoryItem {
        /// Inventory index of the item.
        inventory_index: InventoryIndex,
        /// Equipment position to equip the item to.
        equip_position: EquipPosition,
    },
    /// Open the info window of an inventory or storage item.
    OpenItemInfo {
        /// Inventory/storage index of the item.
        inventory_index: InventoryIndex,
        /// Where the item is located (inventory or Kafra storage).
        source: ItemSource,
    },
    /// Close the item info window.
    CloseItemInfo,
    /// Open the context menu of a Kafra storage item (right click on the item).
    OpenStorageItemMenu {
        /// Storage slot of the item (0-based server slot).
        storage_index: InventoryIndex,
    },
    /// Move an item from the inventory into the open Kafra storage.
    DepositInventoryItem {
        /// Inventory index of the item.
        inventory_index: InventoryIndex,
        /// Amount to store.
        amount: u32,
    },
    /// Move an item from the open Kafra storage back into the inventory.
    WithdrawStorageItem {
        /// Storage slot of the item (0-based server slot).
        storage_index: InventoryIndex,
        /// Amount to take out.
        amount: u32,
    },
    /// Close the Kafra storage window and tell the server about it.
    CloseStorage,
    /// Double click on an item in the inventory or the Kafra storage.
    ///
    /// While the storage window is open this stores the inventory item or takes
    /// the storage item out; otherwise it does nothing.
    ItemDoubleClick {
        /// Source of the item.
        source: ItemSource,
        /// Item that was double clicked.
        item: InventoryItem<ResourceMetadata>,
    },
    /// Pick up an item from the ground.
    PickUpItem {
        /// Id of the item entity to pick up.
        entity_id: EntityId,
    },
    /// Send a chat message.
    SendMessage {
        /// Text of the message.
        text: String,
    },
    /// Action for the "Next"-button in a dialog.
    NextDialog {
        /// Id of the NPC the player is in a dialog with.
        npc_id: EntityId,
    },
    /// Action for the "Close"-button in a dialog.
    CloseDialog {
        /// Id of the NPC the player is in a dialog with.
        npc_id: EntityId,
    },
    /// Choose an option in a dialog.
    ChooseDialogOption {
        /// Id of the NPC the player is in a dialog with.
        npc_id: EntityId,
        /// Id of the option.
        option: i8,
    },
    /// The player picked a destination of the Warp Portal destination list.
    SelectWarpPoint {
        /// Zero-based index into the servers map list.
        option: usize,
    },
    /// Move an item in the user interface.
    MoveItem {
        /// Source of the move.
        source: ItemSource,
        /// Destination of the move.
        destination: ItemSource,
        /// Item to move.
        item: InventoryItem<ResourceMetadata>,
    },
    /// Show an emotion.
    SendEmotion {
        /// Id of the emotion.
        emotion: u8,
    },
    /// Move a skill in the user interface.
    MoveSkill {
        /// Source of the move.
        source: SkillSource,
        /// Destination of the move.
        destination: SkillSource,
        /// Skill to move.
        skill: LearnableSkill,
    },
    /// Cast a skill at the current mouse target.
    CastSkill {
        /// Id of the skill to cast.
        skill_id: SkillId,
        /// Skill type from the skill the player selected (hotbar/skill tree),
        /// so the cast path does not silently depend on the learned
        /// list still containing it.
        skill_type: SkillType,
        /// Level to cast the skill at.
        level: SkillLevel,
    },
    /// Switch the visible hotbar tab (F11/F12 like a paging key).
    CycleHotbarTab {
        /// +1 for the next tab, -1 for the previous one.
        delta: i32,
    },
    /// Use the skill or item in a hotbar slot, like in the original client.
    /// Skills that require a target start a target selection, self cast skills
    /// are cast immediately; item hotkeys use the first matching inventory
    /// item.
    UseSkillInSlot {
        /// Slot of the hotbar that the skill is bound to.
        slot: HotbarSlot,
    },
    /// Use a learned skill directly, e.g. by double clicking it in the skill
    /// tree. Skills that require a target start a target selection, self cast
    /// skills are cast immediately.
    OpenSkillDescription {
        name: String,
        learned_level: Option<SkillLevel>,
        maximum_level: SkillLevel,
        skill_type: Option<SkillType>,
        spell_point_cost: Option<u16>,
        attack_range: Option<AttackRange>,
        upgradable: bool,
        position: ScreenPosition,
    },
    UseSkill {
        /// Id of the skill to use.
        skill_id: SkillId,
        /// Level to cast the skill at.
        level: SkillLevel,
    },
    /// Stop a skill.
    StopSkill {
        /// Slot of the hotbar that the skill is bound to.
        slot: HotbarSlot,
    },
    /// Add a new friend.
    AddFriend {
        /// Name of the character to befriend.
        character_name: String,
    },
    /// Remove a current friend.
    RemoveFriend {
        /// Account id of the friend.
        account_id: AccountId,
        /// Character id of the friend.
        character_id: CharacterId,
    },
    /// Reject a pending friend request.
    RejectFriendRequest {
        /// Account id of the requestor.
        account_id: AccountId,
        /// Character id of the requestor.
        character_id: CharacterId,
    },
    /// Accept a pending friend request.
    AcceptFriendRequest {
        /// Account id of the requestor.
        account_id: AccountId,
        /// Character id of the requestor.
        character_id: CharacterId,
    },
    /// Buy items from a shop.
    BuyItems {
        /// Items to buy.
        items: Vec<ShopItem<u32>>,
    },
    /// Close the shop.
    CloseShop,
    /// Choose whether to buy or sell items at a shop.
    BuyOrSell {
        /// Id of the open shop.
        shop_id: ShopId,
        /// Whether to sell or buy items.
        buy_or_sell: BuyOrSellOption,
    },
    /// Sell items to a shop.
    SellItems {
        /// Items to sell.
        items: Vec<SoldItemInformation>,
    },
    /// Up a stat.
    StatUp {
        stat_type: StatUpType,
    },
    /// Distribute skill points to meet all requirements for a given skill and
    /// put a single point into the provided skill. If the player does not
    /// have enough skill points, this will skill as much of the
    /// dependencies as possible.
    DistributePointsForSkill {
        /// Id of the skill to level up.
        skill_id: SkillId,
    },
    /// Level up a skill.
    LevelUpSkills {
        /// List of skills to level up by one. This list is allowed to contain
        /// the same skill id multiple times and they will be applied
        /// sequentially from start to end.
        skill_ids: Vec<SkillId>,
    },
    /// Reload the language from disk.
    #[cfg(feature = "debug")]
    ReloadLanguage,
    /// Save the language to disk.
    #[cfg(feature = "debug")]
    SaveLanguage,
    /// Warp the player.
    #[cfg(feature = "debug")]
    WarpToMap {
        /// Map name. Can be the same as the current map.
        map_name: String,
        /// Position on the new map after the warp.
        position: TilePosition,
    },
    /// Open a window with the details for a marker.
    #[cfg(feature = "debug")]
    OpenMarkerDetails {
        /// Id of the marker to inspect.
        marker_identifier: MarkerIdentifier,
    },
    /// Open or close the render options window.
    #[cfg(feature = "debug")]
    ToggleRenderOptionsWindow,
    /// Open the map data window.
    #[cfg(feature = "debug")]
    OpenMapDataWindow,
    /// Open or close the client state inspector window.
    #[cfg(feature = "debug")]
    ToggleClientStateInspectorWindow,
    /// Open or close the maps window. Only works while playing.
    #[cfg(feature = "debug")]
    ToggleMapsWindow,
    /// Open or close the commands window. Only works while playing.
    #[cfg(feature = "debug")]
    ToggleCommandsWindow,
    /// Open the theme inspector window.
    #[cfg(feature = "debug")]
    ToggleThemeInspectorWindow,
    /// Open or close the profiler window.
    #[cfg(feature = "debug")]
    ToggleProfilerWindow,
    /// Open or close the packet inspector window.
    #[cfg(feature = "debug")]
    TogglePacketInspectorWindow,
    /// Open the cache statistics window.
    #[cfg(feature = "debug")]
    ToggleCacheStatisticsWindow,
    /// Move the view direction of the debug camera.
    #[cfg(feature = "debug")]
    CameraLookAround {
        /// Offset of the view direction.
        offset: Vector2<f32>,
    },
    /// Move the debug camera forward.
    #[cfg(feature = "debug")]
    CameraMoveForward,
    /// Move the debug camera backward.
    #[cfg(feature = "debug")]
    CameraMoveBackward,
    /// Move the debug camera left.
    #[cfg(feature = "debug")]
    CameraMoveLeft,
    /// Move the debug camera right.
    #[cfg(feature = "debug")]
    CameraMoveRight,
    /// Move the debug camera up.
    #[cfg(feature = "debug")]
    CameraMoveUp,
    /// Set the debug camera speed to its higher value.
    #[cfg(feature = "debug")]
    CameraAccelerate,
    /// Set the debug camera speed to its lower value.
    #[cfg(feature = "debug")]
    CameraDecelerate,
    /// Open a window to inspect a frame.
    #[cfg(feature = "debug")]
    InspectFrame {
        measurement: FrameMeasurement,
    },
}

impl From<InputEvent> for Event<ClientState> {
    fn from(custom_event: InputEvent) -> Self {
        Event::Application { custom_event }
    }
}

impl ClickHandler<ClientState> for InputEvent {
    fn handle_click(&self, _: &State<ClientState>, queue: &mut EventQueue<ClientState>) {
        queue.queue(self.clone());
    }
}
