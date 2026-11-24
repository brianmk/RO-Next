use std::time::Instant;

use ragnarok_packets::*;

use crate::hotkey::HotkeyState;
use crate::items::{ShopItem, VendingShopItem};
use crate::{
    CharacterServerLoginData, EntityData, InventoryItem, LoginServerLoginData, MessageColor, NoMetadata,
    UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};

/// An event triggered by one of the Ragnarok Online servers.
#[derive(Debug)]
pub enum NetworkEvent {
    LoginServerConnected {
        character_servers: Vec<CharacterServerInformation>,
        login_data: LoginServerLoginData,
    },
    LoginServerConnectionFailed {
        reason: UnifiedLoginFailedReason,
        message: String,
    },
    LoginServerDisconnected {
        reason: DisconnectReason,
    },
    CharacterServerConnected {
        normal_slot_count: usize,
    },
    CharacterServerConnectionFailed {
        reason: LoginFailedReason,
        message: String,
    },
    CharacterServerDisconnected {
        reason: DisconnectReason,
    },
    AccountId {
        account_id: AccountId,
    },
    CharacterList {
        characters: Vec<CharacterInformation>,
    },
    CharacterSelected {
        login_data: CharacterServerLoginData,
    },
    CharacterSelectionFailed {
        reason: UnifiedCharacterSelectionFailedReason,
        message: &'static str,
    },
    CharacterCreated {
        character_information: CharacterInformation,
    },
    CharacterCreationFailed {
        reason: CharacterCreationFailedReason,
        message: &'static str,
    },
    CharacterDeleted,
    CharacterDeletionFailed {
        reason: CharacterDeletionFailedReason,
        message: &'static str,
    },
    MapServerDisconnected {
        reason: DisconnectReason,
    },
    /// A client-internal failure that is not caused by the server, for example
    /// a packet handler that could not be registered because of a duplicate
    /// header. The networking thread reports it here instead of panicking.
    InternalError {
        message: String,
    },
    /// Initial player status.
    InitialStats {
        strength_stat_points_cost: u8,
        agility_stat_points_cost: u8,
        vitality_stat_points_cost: u8,
        intelligence_stat_points_cost: u8,
        dexterity_stat_points_cost: u8,
        luck_stat_points_cost: u8,
        /// Attack including the weapon attack (`attPower`).
        attack: u16,
        /// Weapon refine attack bonus (`refiningPower`).
        bonus_attack: u16,
        /// Maximum magic attack (`max_mattPower`).
        magic_attack_max: u16,
        /// Minimum magic attack (`min_mattPower`).
        magic_attack_min: u16,
        /// Equipment defense (`itemdefPower`).
        defense: u16,
        /// Vitality based defense (`plusdefPower`).
        bonus_defense: u16,
        /// Equipment magic defense (`mdefPower`).
        magic_defense: u16,
        /// Intelligence based magic defense (`plusmdefPower`).
        bonus_magic_defense: u16,
        hit: u16,
        flee: u16,
        perfect_dodge: u16,
        critical: u16,
        /// Attack motion in milliseconds (`ASPD`).
        attack_speed: u16,
    },
    /// Resurrect a player.
    ResurrectPlayer {
        entity_id: EntityId,
    },
    /// Make a player stand up.
    PlayerStandUp {
        entity_id: EntityId,
    },
    /// Make a player sit down.
    PlayerSitDown {
        entity_id: EntityId,
    },
    /// A status effect (buff/debuff) of an entity was started, refreshed or
    /// ended. Sent by the map server as `ZC_MSG_STATE_CHANGE3` (0x0983) with
    /// timers, `ZC_MSG_STATE_CHANGE` (0x0196) for ending statuses, and
    /// `ZC_EFST_SET_ENTER` (0x0984) when the entity enters the client's view
    /// with an already active status.
    StatusEffect {
        entity_id: EntityId,
        status_index: u16,
        /// `false` means the status effect ended (or was dispelled).
        active: bool,
        total_duration_in_milliseconds: u32,
        remaining_in_milliseconds: u32,
        /// The up to three server-side status values (`val1`..`val3`).
        ///
        /// Only a handful of status effects send values (elemental weapon
        /// enchant, exp/item boosts, ...); they are zero for the rest.
        values: [i32; 3],
    },
    /// An entity completed a skill cast without dealing damage.
    ///
    /// On the 2019 server this comes from `ZC_USE_SKILL` (0x09CB), which is
    /// sent for instant/complete casts (self-buffs, crafting, ...). The
    /// ambient plaza actors use it for their self-buffs and the blacksmith
    /// crafting loop.
    SkillNoDamage {
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        skill_id: SkillId,
        /// Skill level (the packet reuses the heal amount field for it).
        level: u32,
        success: bool,
    },
    /// An entity used a ground-targeted skill (`ZC_NOTIFY_GROUNDSKILL`,
    /// 0x0117).
    ///
    /// The ambient crafting loop uses this as a fall back to show the hammer
    /// strike at the target cell.
    SkillGroundCast {
        entity_id: EntityId,
        skill_id: SkillId,
        level: u16,
        position: TilePosition,
    },
    /// An entity used a skill (or an item that behaves like one).
    ///
    /// On the 2019 server this comes from `ZC_USESKILL_ACK` (0x0B1A). Other
    /// clients play the caster's skill-cast animation and the skill effect, so
    /// ambient plaza actors (self-buffs, crafting, warp portals, ...) become
    /// visible.
    SkillCast {
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        position: TilePosition,
        skill_id: SkillId,
        /// Skill property (`0` = neutral/yellow cast aura, `1` = water,
        /// `2` = earth, `3` = fire, ...).
        element: u32,
        /// Cast time in milliseconds.
        cast_time: u32,
        /// Attack motion in milliseconds (may be 0 on instant casts).
        attack_motion_time: u32,
    },
    /// An entity's skill cast was cancelled (`ZC_DISPEL`, 0x01B9).
    ///
    /// Sent by rAthena `clif_skillcastcancel` when a cast is interrupted
    /// (movement, damage, another skill, ...). The client stops the looping
    /// cast animation and clears the cast bar of the local player.
    SkillCastCancelled {
        entity_id: EntityId,
    },
    /// The map server asks the player to choose the destination of a Warp
    /// Portal (`AL_WARP`, `ZC_WARPLIST` 0x0ABE).
    ///
    /// The client answers with [`SelectWarpPointPacket`]. The server then
    /// creates the portal skill units, which arrive as `AddSkillUnit` events
    /// and are rendered as the classic blue portal.
    WarpPointList {
        skill_id: SkillId,
        /// Destination map names (as sent by the server, usually with `.gat`).
        maps: Vec<String>,
    },
    /// The map server asked the client to play a 3d sound effect (`ZC_SOUND`,
    /// 0x01D3). `name` is the file name relative to `data/wav`.
    PlaySound {
        name: String,
        /// `0`/`1` mark the start/end of a looping sound; both play it once.
        act: u8,
        /// Minimum interval between two plays of the same sound, in ms.
        term: u32,
        /// Account id of the entity that produced the sound.
        account_id: AccountId,
    },
    /// Show an emotion above an entity.
    DisplayEmotion {
        entity_id: EntityId,
        emotion: u8,
    },
    /// Add an entity to the list of entities that the client is aware of.
    AddEntity {
        entity_data: EntityData,
    },
    /// Remove an entity from the list of entities that the client is aware of
    /// by its id.
    RemoveEntity {
        entity_id: EntityId,
        reason: DisappearanceReason,
    },
    /// Add an item to the ground.
    AddGroundItem {
        entity_id: EntityId,
        item_id: ItemId,
        is_identified: bool,
        quantity: u16,
        position: TilePosition,
        x_offset: u8,
        y_offset: u8,
    },
    /// Remove an item from the ground.
    RemoveGroundItem {
        entity_id: EntityId,
    },
    /// The player is pathing to a new position.
    PlayerMove {
        origin: WorldPosition,
        destination: WorldPosition,
        starting_timestamp: ClientTick,
    },
    /// An Entity nearby is pathing to a new position.
    EntityMove {
        entity_id: EntityId,
        origin: WorldPosition,
        destination: WorldPosition,
        starting_timestamp: ClientTick,
    },
    /// Player was moved to a new position on a different map or the current map
    ChangeMap {
        map_name: String,
        position: TilePosition,
    },
    /// Update the client side to keep server and client synchronized.
    UpdateClientTick {
        client_tick: ClientTick,
        received_at: Instant,
    },
    /// New chat message for the client.
    ChatMessage {
        text: String,
        color: MessageColor,
    },
    /// A chat message that should additionally be shown above the speaking
    /// entity. On the 2019 rAthena server this comes from `ZC_NOTIFY_CHAT`
    /// (0x008D), which is used for proximity ("normal") chat as well as NPC
    /// dialogue. The client filters the entity type when rendering, so only
    /// player speech produces an overhead bubble.
    OverheadChatMessage {
        entity_id: EntityId,
        text: String,
        color: MessageColor,
    },
    CharacterSlotSwitched,
    CharacterSlotSwitchFailed,
    /// Update entity details. Mostly received when the client sends
    /// [RequestDetailsPacket] after the player hovered an entity.
    UpdateEntityDetails {
        entity_id: EntityId,
        name: String,
        /// The guild name of the entity, if it is a player in a guild.
        ///
        /// Only the player details response (`0x0A30`) contains the guild
        /// name; the response for NPCs does not, so this is `None` for them.
        guild_name: Option<String>,
    },
    /// Guild emblem image (`ZC_GUILD_EMBLEM_IMG`, `0x0152`).
    ///
    /// The data is a classic 24x24 guild emblem BMP file. It can be empty when
    /// the guild has no emblem.
    GuildEmblemImage {
        guild_id: u32,
        emblem_id: u32,
        emblem_data: Vec<u8>,
    },
    UpdateEntityHealth {
        entity_id: EntityId,
        health_points: usize,
        maximum_health_points: usize,
    },
    DamageEffect {
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        /// Damage amount. [`None`] on miss, [`Some`] otherwise.
        damage_amount: Option<usize>,
        attack_duration: u32,
        is_critical: bool,
        /// Skill id of the damage source, when the client can attribute it.
        ///
        /// The 2019 `ZC_NOTIFY_SKILL` (0x01DE) carries the skill id; plain
        /// attack packets (`ZC_NOTIFY_ACT` 0x08C8 / 0x008A) do not. `None`
        /// therefore means "auto attack / unknown source", never "skill 0".
        skill_id: Option<SkillId>,
        /// Server start tick of the skill use from `ZC_NOTIFY_SKILL`. Multi-hit
        /// packets of one cast share this tick, which lets the client play the
        /// completion effect only once per cast.
        skill_start_tick: Option<ClientTick>,
    },
    EntityPickUpItem {
        entity_id: EntityId,
        item_entity_id: EntityId,
    },
    /// A heal/recovery number (`ZC_RECOVERY`, 0x0A27). The 2019 packet is
    /// sent only to the player themselves and carries no entity id, so the
    /// receiver resolves the local player when `entity_id` is [`None`].
    HealEffect {
        entity_id: Option<EntityId>,
        heal_amount: usize,
    },
    /// An entity turned (`ZC_CHANGE_DIRECTION`, 0x009C).
    ChangeDirection {
        entity_id: EntityId,
        head_direction: usize,
        direction: Direction,
    },
    /// An entity's option/status state changed (`ZC_STATE_CHANGE`, 0x0229).
    ///
    /// `body_state` is the rAthena `opt1`
    /// (freeze/stone/stun/sleep/burning/...), `health_state` the `opt2`
    /// (poison/curse/silence/confusion/blind/...) and `effect_state` the
    /// option bit mask (falcon/cart/riding/hide/cloak/...).
    StateChange {
        entity_id: EntityId,
        body_state: u16,
        health_state: u16,
        effect_state: u32,
        is_pk_mode_on: bool,
    },
    UpdateStat {
        stat_type: StatType,
    },
    OpenEditDialog {
        npc_id: EntityId,
        /// `true` for the string variant (`ZC_OPEN_EDITDLGSTR`).
        string_input: bool,
    },
    OpenDialog {
        text: String,
        npc_id: EntityId,
    },
    AddNextButton {
        npc_id: EntityId,
    },
    AddCloseButton {
        npc_id: EntityId,
    },
    AddChoiceButtons {
        choices: Vec<String>,
        npc_id: EntityId,
    },
    AddQuestEffect {
        quest_effect: QuestEffectPacket,
    },
    RemoveQuestEffect {
        entity_id: EntityId,
    },
    SetInventory {
        items: Vec<InventoryItem<NoMetadata>>,
    },
    /// Full Kafra storage list (2019: `ZC_INVENTORY_START`/`_LIST`/`_END` with
    /// `inventory_type` 2).
    SetStorage {
        items: Vec<InventoryItem<NoMetadata>>,
        name: String,
    },
    /// Kafra storage capacity (`ZC_NOTIFY_STOREITEM_COUNTINFO`, 0x00F2).
    SetStorageAmount {
        amount: u16,
        maximum_amount: u16,
    },
    /// An item was added to the Kafra storage (`ZC_ADD_ITEM_TO_STORE`, 0x0A0A).
    StorageItemAdded {
        item: InventoryItem<NoMetadata>,
    },
    /// An item was removed from the Kafra storage
    /// (`ZC_DELETE_ITEM_FROM_STORE`, 0x00F6). The index is the 0-based storage
    /// slot (already converted from the `+1` wire encoding).
    StorageItemRemoved {
        storage_index: u16,
        amount: u32,
    },
    /// The server closed the Kafra storage (`ZC_CLOSE_STORE`, 0x00F8).
    StorageClosed,
    IventoryItemAdded {
        item: InventoryItem<NoMetadata>,
    },
    ItemObtained {
        item_id: ItemId,
        quantity: u16,
        is_identified: bool,
    },
    SkillTree {
        skill_information: Vec<SkillInformation>,
    },
    /// A single skill was learned or upgraded after the initial skill list
    /// (`ZC_ADD_SKILL`, 0x0111 on this packet version).
    AddSkill {
        skill_information: SkillInformation,
    },
    /// The level, SP cost or range of a known skill changed
    /// (`ZC_SKILLINFO_UPDATE2`, 0x07E1 on this packet version).
    UpdateSkillInfo {
        skill_id: SkillId,
        skill_type: SkillType,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        upgradable: bool,
    },
    UpdateEquippedPosition {
        index: InventoryIndex,
        equipped_position: EquipPosition,
    },
    ChangeJob {
        account_id: AccountId,
        job_id: JobId,
    },
    ChangeHair {
        account_id: AccountId,
        hair_id: u32,
    },
    /// The server asked the client to use a skill from an item (skill items
    /// such as the fly wing). The client has to send the actual skill request.
    AutoRunSkill {
        skill_id: SkillId,
        skill_level: SkillLevel,
    },
    /// A sprite (appearance) change for the local player, e.g. when a headgear
    /// equip/unequip acknowledgement carries a new view id.
    LocalAppearanceChanged {
        sprite_type: SpriteChangeType,
        value: u32,
    },
    /// A sprite (appearance) change for an entity that is not the job or hair,
    /// e.g. headgear or palette changes.
    AppearanceChanged {
        account_id: AccountId,
        sprite_type: SpriteChangeType,
        value: u32,
    },
    LoggedOut,
    FriendRequest {
        requestee: Friend,
    },
    VisualEffect {
        effect_path: &'static str,
        entity_id: EntityId,
    },
    AddSkillUnit {
        entity_id: EntityId,
        unit_id: UnitId,
        position: TilePosition,
        /// Skill level of the unit (`NotifySkillUnitPacket` carries it); used
        /// to derive the lifetime of warp portals.
        skill_level: u8,
    },
    RemoveSkillUnit {
        entity_id: EntityId,
    },
    SetFriendList {
        friend_list: Vec<Friend>,
    },
    FriendAdded {
        friend: Friend,
    },
    FriendRemoved {
        account_id: AccountId,
        character_id: CharacterId,
    },
    /// Online state (and, for online friends, the name) of a friend arrived
    /// via `ZC_FRIENDS_STATE` (0x0206).
    FriendOnlineState {
        account_id: AccountId,
        character_id: CharacterId,
        online: bool,
        name: String,
    },
    SetHotkeyData {
        tab: HotbarTab,
        hotkeys: Vec<HotkeyState>,
    },
    OpenShop {
        items: Vec<ShopItem<NoMetadata>>,
    },
    AskBuyOrSell {
        shop_id: ShopId,
    },
    BuyingCompleted {
        result: BuyItemResult,
    },
    SellItemList {
        items: Vec<SellItemInformation>,
    },
    SellingCompleted {
        result: SellItemsResult,
    },
    InventoryItemRemoved {
        reason: RemoveItemReason,
        index: InventoryIndex,
        amount: u16,
    },
    /// The amount of an item in the inventory changed (e.g. after using a
    /// stackable item). If the new amount is zero the item is removed.
    InventoryItemAmountChanged {
        index: InventoryIndex,
        amount: u16,
    },
    AttackFailed {
        target_entity_id: EntityId,
        target_position: TilePosition,
        player_position: TilePosition,
        attack_range: AttackRange,
    },
    UpdateSkill {
        skill_id: SkillId,
        skill_level: SkillLevel,
        spell_point_cost: u16,
        attack_range: AttackRange,
        upgradable: bool,
    },
    /// Delete a skill from the skill tree.
    RemoveSkill {
        skill_id: SkillId,
    },
    /// Complete party roster (`ZC_GROUP_LIST`, 0x0AE5).
    ///
    /// Sent when the client joins a party and whenever the roster changes; the
    /// entries carry the member names, map names, jobs and levels but no HP.
    PartyList {
        party_name: String,
        members: Vec<PartyListMember>,
    },
    /// A party member joined or changed their map (`ZC_ADD_MEMBER_TO_GROUP`,
    /// 0x0AE4).
    PartyMemberAdded {
        member: PartyMemberDetails,
    },
    /// A party member left or was expelled (`ZC_DELETE_MEMBER_FROM_GROUP`,
    /// 0x0105).
    PartyMemberRemoved {
        account_id: AccountId,
        character_name: String,
        /// rAthena `e_party_member_withdraw` result code.
        result: u8,
    },
    /// Party member HP update (`ZC_NOTIFY_HP_TO_GROUPM`, 0x080E).
    PartyMemberHealth {
        account_id: AccountId,
        health_points: i32,
        maximum_health_points: i32,
    },
    /// Party member position update (`ZC_NOTIFY_POSITION_TO_GROUPM`, 0x0107).
    PartyMemberPosition {
        account_id: AccountId,
        position: TilePosition,
    },
    /// Party member job/level update (`ZC_NOTIFY_MEMBERINFO_TO_GROUPM`,
    /// 0x0ABD).
    PartyMemberJobLevel {
        account_id: AccountId,
        job_id: JobId,
        base_level: u16,
    },
    /// Party member death state (`ZC_GROUP_ISALIVE`, 0x0AB2).
    PartyMemberAlive {
        account_id: AccountId,
        is_dead: bool,
    },
    /// Party invitation prompt (`ZC_PARTY_JOIN_REQ`, 0x02C6).
    PartyInvite {
        party_id: PartyId,
        party_name: String,
    },
    /// Result of an invitation (`ZC_PARTY_JOIN_REQ_ACK`, 0x02C5).
    PartyInviteReply {
        character_name: String,
        result: i32,
    },
    /// Result of creating a party (`ZC_ACK_MAKE_GROUP`, 0x00FA).
    PartyCreationResult {
        result: u8,
    },
    /// Party chat message (`ZC_NOTIFY_CHAT_PARTY`, 0x0109).
    PartyChatMessage {
        account_id: AccountId,
        message: String,
    },
    /// The party invite blocking config (`ZC_PARTY_CONFIG`, 0x02C9).
    PartyConfig {
        allow_invites: bool,
    },
    /// Party EXP sharing and item rule config (`ZC_PARTY_OPTION`, 0x07D8).
    PartyOption {
        /// 0 = EXP sharing disabled, 1 = enabled, 2 = cannot be changed.
        experience_option: u32,
        item_pickup: u8,
        item_share: u8,
    },
    /// Experience was gained (or lost) by the local player (`ZC_NOTIFY_EXP2`,
    /// 0x0ACC).
    GainedExperience {
        /// Signed: the server also sends negative amounts for EXP loss.
        amount: i64,
        experience_type: ExperienceType,
        experience_source: ExperienceSource,
    },
    /// A skill could not be used (`ZC_ACK_TOUSESKILL`, 0x0110).
    SkillUseFailed {
        skill_id: SkillId,
        /// rAthena `useskill_fail_cause`.
        cause: u8,
    },
    /// A stat point allocation was refused (`ZC_STATUS_CHANGE_ACK`, 0x00BC).
    StatUpFailed,
    /// An ammunition related action failed (`ZC_ACTION_FAILURE`, 0x013B).
    AmmunitionAction {
        action: AmmunitionActionType,
    },
    /// The server equipped ammunition automatically (`ZC_EQUIP_ARROW`,
    /// 0x013C).
    AmmunitionEquipped {
        inventory_index: InventoryIndex,
    },
    /// The number of online players, reply to `/who` (`ZC_USER_COUNT`,
    /// 0x00C2).
    UserCount {
        player_count: i32,
    },
    /// The map server kicked the client (`SC_NOTIFY_BAN`, 0x0081).
    MapServerBan {
        error_code: u8,
    },
    /// The map server refused the map login (`ZC_REFUSE_ENTER`, 0x0074).
    RefuseEnter {
        error_code: u8,
    },
    /// A system message from the client message table (`ZC_MSG`, 0x0291).
    MessageString {
        message_id: u16,
    },
    /// Spirit-sphere count of an entity (`ZC_SPIRITS` 0x01D0 / `ZC_SPIRITS2`
    /// 0x01E1). Sphere rendering is owned by the skill-souls/cast-bar lane;
    /// this event is the typed packet-layer hand-off.
    SpiritOrbs {
        entity_id: EntityId,
        amount: i16,
    },
    /// A skill cooldown/delay was started (`ZC_SKILL_POSTDELAY`, 0x043D).
    /// Cooldown UI is owned by the skill-souls/cast-bar lane.
    SkillCooldown {
        skill_id: SkillId,
        until: ClientTick,
    },
    /// The player's attack range changed (`ZC_ATTACK_RANGE`, 0x013A).
    AttackRangeChanged {
        attack_range: AttackRange,
    },
    /// Another player requests a trade (`ZC_REQ_EXCHANGE_ITEM`, 0x01F4).
    TradeRequest {
        requester_name: String,
        target_account_id: AccountId,
        target_level: u16,
    },
    /// Result of a trade request (`ZC_ACK_EXCHANGE_ITEM`, 0x01F5).
    TradeAck {
        result: u8,
        target_account_id: AccountId,
    },
    /// A raw `ZC_NOTIFY_EFFECT2` (0x01F3) effect id (weather, ...).
    SpecialEffect {
        entity_id: EntityId,
        effect_id: u32,
    },
    /// The other player offered an item or zeny (`ZC_ADD_EXCHANGE_ITEM`,
    /// 0x0A96). `item_id == 0` means zeny.
    TradeItemOffered {
        item_id: u32,
        amount: u32,
    },
    /// Local add-item acknowledgement (`ZC_ACK_ADD_EXCHANGE_ITEM`, 0x00EA).
    TradeAddResult {
        inventory_index: u16,
        result: u8,
    },
    /// The trade was cancelled (`ZC_CANCEL_EXCHANGE_ITEM`, 0x00EE).
    TradeCancelled,
    /// One side locked the trade (`ZC_CONCLUDE_EXCHANGE_ITEM`, 0x00EC).
    TradeConcluded {
        who: u8,
    },
    /// The trade completed (`ZC_EXEC_EXCHANGE_ITEM`, 0x00F0).
    TradeExecuted {
        result: u8,
    },
    /// Complete quest list (`ZC_QUEST_LIST`, 0x09F8).
    QuestList {
        quests: Vec<ragnarok_packets::Quest>,
    },
    /// A quest was activated or deactivated (`ZC_QUEST_NOTIFY`, 0x09F9).
    QuestActive {
        quest_id: u32,
        active: bool,
    },
    /// A quest was removed (`ZC_QUEST_REMOVED`, 0x02B4).
    QuestRemoved {
        quest_id: u32,
    },
    /// The server allows the player to create a vending shop (`ZC_OPENSTORE`,
    /// 0x012D) after casting `MC_VENDING`; `slots` is the number of item slots.
    OpenStore {
        slots: u16,
    },
    /// An entity stopped moving at a server-corrected position
    /// (`ZC_STOPMOVE`, 0x0088).
    EntityStopped {
        entity_id: EntityId,
        position: TilePosition,
    },
    /// An entity performed a server-side blink/jump (`ZC_HIGHJUMP`, 0x01FF;
    /// Taekwon Leap / TK_HIGHJUMP). Unlike a walk this is an instant
    /// position change, so the client applies it immediately.
    EntityHighJumped {
        entity_id: EntityId,
        position: TilePosition,
    },
    /// An entity relocated instantly (`ZC_SNAP`, 0x08D2; Monk/Champion Body
    /// Relocation and similar teleport skills).
    EntitySnapped {
        entity_id: EntityId,
        position: TilePosition,
    },
    /// A minimap marker was placed/removed (`ZC_COMPASS`, 0x0144).
    MinimapMarker {
        npc_id: EntityId,
        marker_type: MarkerType,
        position: LargeTilePosition,
        id: u8,
        color: ColorRGBA,
    },
    /// Map type/flags (`ZC_NOTIFY_MAPPROPERTY2`, 0x099B).
    MapProperties {
        map_type: u16,
        flags: u32,
    },
    /// Merchant cart item/weight counts (`ZC_NOTIFY_CARTITEM_COUNTINFO`,
    /// 0x0121).
    CartInfo {
        current_count: i16,
        maximum_count: i16,
        current_weight: i32,
        maximum_weight: i32,
    },
    /// The map server rejected the map login (`ZC_AUTH_FAIL`, 0x006A).
    MapServerAuthFailed {
        reason: u8,
    },
    /// The server asked the client to show/hide the equipment window
    /// (`ZC_CONFIG_NOTIFY`, 0x02DA).
    ShowEquipmentWindow {
        open: bool,
    },
    /// Weight percentage at which the client renders the weight in red
    /// (`ZC_NOTIFY_CRITICALWEIGHT`, 0x0ADE, rAthena
    /// `natural_heal_weight_rate`).
    CriticalWeight {
        percentage: u32,
    },
    /// A dynamic map cell changed (`ZC_UPDATE_MAPINFO`, 0x0192).
    MapCellChanged {
        position: TilePosition,
        cell_type: u16,
        map_name: String,
    },
    /// Full merchant pushcart list. rAthena sends the cart through the shared
    /// inventory list family (`ZC_INVENTORY_START`/`_LIST`/`_END` with
    /// `inventory_type` 1), see `clif_cartlist` in `clif.cpp:3226`.
    SetCart {
        items: Vec<InventoryItem<NoMetadata>>,
    },
    /// An item was added to the cart (`ZC_ADD_ITEM_TO_CART`, 0x0A0B,
    /// `clif.cpp:7273`).
    CartItemAdded {
        item: InventoryItem<NoMetadata>,
    },
    /// An item (or part of a stack) was removed from the cart
    /// (`ZC_DELETE_ITEM_FROM_CART`, 0x0125, `clif.cpp:7530`).
    CartItemRemoved {
        cart_index: InventoryIndex,
        amount: u32,
    },
    /// The server removed the cart (`ZC_CARTOFF`, 0x012B, `clif.cpp:3284`).
    CartClosed,
    /// Moving an item into the cart failed (`ZC_ACK_ADDITEM_TO_CART`,
    /// 0x012C, `clif.cpp:7313`): 0 = weight limit, 1 = item count limit.
    CartAddItemFailed {
        result: u8,
    },
    /// A vending shop entered the client's view (`ZC_STORE_ENTRY`, 0x0131,
    /// `clif.cpp:7558`).
    StoreEntry {
        owner_account_id: AccountId,
        shop_name: String,
    },
    /// A vending shop left the client's view (`ZC_DISAPPEAR_ENTRY`, 0x0132,
    /// `clif.cpp:7576`).
    StoreEntryRemoved {
        owner_account_id: AccountId,
    },
    /// The item list of another player's vending shop
    /// (`ZC_PC_PURCHASE_ITEMLIST_FROMMC`, 0x0800 for PACKETVER 20190605,
    /// `clif.cpp:7592`).
    VendingShopList {
        owner_account_id: AccountId,
        shop_id: u32,
        items: Vec<VendingShopItem<NoMetadata>>,
    },
    /// A vending purchase failed (`ZC_PC_PURCHASE_RESULT_FROMMC`, 0x0135,
    /// `clif.cpp:7642`). Result codes are documented on
    /// [`VendingPurchaseResultPacket2019`].
    VendingPurchaseResult {
        result: u8,
    },
    /// The complete Rodex inbox (`ZC_ACK_MAIL_LIST3`, 0x0AC2).
    MailList {
        mails: Vec<ragnarok_packets::MailListEntry2019>,
    },
    /// The body and attachments of an opened mail (`ZC_ACK_READ_RODEX`,
    /// 0x09EB).
    MailRead {
        mail_id: u64,
        open_type: u8,
        body: String,
        zeny: u64,
        items: Vec<ragnarok_packets::MailAttachmentItem2019>,
    },
    /// Unread-mail indicator (`ZC_NOTIFY_UNREADMAIL`, 0x09E7).
    MailUnreadStatus {
        unread: bool,
    },
    /// Result of claiming an attachment (`ZC_ACK_ZENY_FROM_MAIL`, 0x09F2, or
    /// `ZC_ACK_ITEM_FROM_MAIL`, 0x09F4). Claimed items/zeny arrive through the
    /// regular inventory/zeny update packets.
    MailAttachmentClaimed {
        mail_id: u64,
        zeny: bool,
        result: u8,
    },
    /// A mail was deleted (`ZC_ACK_DELETE_MAIL`, 0x09F6; only sent on
    /// success).
    MailDeleted {
        mail_id: u64,
    },
    /// The server asked the client to open/close the mailbox
    /// (`ZC_MAIL_WINDOWS`, 0x0260; `@mail` / script `openmail`).
    MailboxWindow {
        open: bool,
    },
    /// Guild window tab/master mask (`ZC_ACK_GUILD_MENUINTERFACE`, 0x014E).
    GuildMenuInterface {
        menu_flags: u32,
    },
    /// Guild basic information (`ZC_GUILD_INFO`, 0x0A84).
    GuildInfo {
        information: GuildInfoPacket,
    },
    /// Guild member manager list (`ZC_MEMBERMGR_INFO`, 0x0AA5).
    GuildMembers {
        members: Vec<GuildMemberInfo>,
    },
    /// Guild position names (`ZC_POSITION_ID_NAME_INFO`, 0x0166).
    GuildPositionNames {
        positions: Vec<GuildPositionNameEntry>,
    },
    /// Guild position rights/pay rates (`ZC_POSITION_INFO`, 0x0160).
    GuildPositionRights {
        positions: Vec<GuildPositionInfoEntry>,
    },
    /// A guild position was changed (`ZC_ACK_CHANGE_GUILD_POSITIONINFO`,
    /// 0x0174).
    GuildPositionChanged {
        positions: Vec<GuildPositionChangedEntry>,
    },
    /// A guild member was moved to another position
    /// (`ZC_ACK_REQ_CHANGE_MEMBERS`, 0x0156).
    GuildMemberPositionChanged {
        members: Vec<GuildMemberPositionChangedEntry>,
    },
    /// Guild notice (`ZC_GUILD_NOTICE`, 0x016F).
    GuildNotice {
        subject: String,
        notice: String,
    },
    /// Guild alliance/opposition list (`ZC_MYGUILD_BASIC_INFO`, 0x014C).
    GuildAlliances {
        alliances: Vec<GuildAllianceInfo>,
    },
    /// The character name of a guild member (`ZC_ACK_REQNAME_BYGID`, 0x0AF7).
    ///
    /// The 2019 member list has no name field, so the client asks the server
    /// for every member name by character id.
    GuildMemberName {
        character_id: CharacterId,
        name: String,
    },
    /// A character left or was expelled from the guild
    /// (`ZC_ACK_LEAVE_GUILD` 0x0A83 / `ZC_ACK_BAN_GUILD` 0x0A82).
    GuildMemberRemoved {
        character_id: CharacterId,
        expelled: bool,
    },
    /// The server rejected a guild invite (`ZC_ACK_REQ_JOIN_GUILD`, 0x0169).
    ///
    /// rAthena only sends this on failure; a successful invitation is delivered
    /// to the target, so there is no success event.
    GuildInviteResult {
        result: u8,
    },
    /// Another player whispered the local player (`ZC_WHISPER`, 0x09DE).
    ///
    /// The client keeps a per-partner conversation history and a whisper chat
    /// tab from this, so the raw text is not prefixed like a chat message.
    WhisperReceived {
        sender_name: String,
        message: String,
        is_admin: bool,
    },
    /// Guild chat message (`ZC_GUILD_CHAT`, 0x017F).
    GuildChat {
        message: String,
    },
    /// The local player picked up an item (`ZC_ITEM_PICKUP_ACK`, 0x0A37).
    ///
    /// The item is already added to the inventory by the same packet; this
    /// event only carries the data for the pickup toast.
    ItemPickedUp {
        item_id: ItemId,
        amount: u16,
    },
    /// A quest was started or updated (`ZC_QUEST_NOTIFY`, 0x09F9) and the HUD
    /// should show a banner for it.
    QuestBanner {
        quest_id: u32,
        active: bool,
        /// Names of the hunt objectives, if the packet contained any.
        objective_names: Vec<String>,
    },
    /// A server-wide broadcast message (`ZC_BROADCAST`, 0x009A or
    /// `ZC_BROADCAST2`, 0x01C3).
    ServerAnnouncement {
        text: String,
    },
    /// The server asked the client to open one of its built-in UI windows
    /// (`ZC_UI_OPEN`, 0x0AE2). `ui_type` 0 is the bank (`OUT_UI_BANK`).
    OpenUserInterface {
        ui_type: u8,
        data: u32,
    },
    /// The banking session was opened (`ZC_ACK_OPEN_BANKING`, 0x09B7).
    BankOpened,
    /// The current bank vault (`ZC_BANKING_CHECK`, 0x09A6).
    BankBalance {
        money: u64,
        reason: u16,
    },
    /// A deposit was processed (`ZC_ACK_BANKING_DEPOSIT`, 0x09A8). `reason` is
    /// `enum e_BANKING_DEPOSIT_ACK`.
    BankDepositResult {
        reason: u16,
        money: u64,
        zeny: i32,
    },
    /// A withdrawal was processed (`ZC_ACK_BANKING_WITHDRAW`, 0x09AA). `reason`
    /// is `enum e_BANKING_WITHDRAW_ACK`.
    BankWithdrawResult {
        reason: u16,
        money: u64,
        zeny: i32,
    },
    /// The banking session was closed (`ZC_ACK_CLOSE_BANKING`, 0x09B9).
    BankClosed,
    /// The server offers the weapons that can be refined
    /// (`ZC_NOTIFY_WEAPONITEMLIST`, 0x0221).
    ///
    /// rAthena only sends this after the player used the Blacksmith skill
    /// `WS_WEAPONREFINE`, and it only accepts the refine request
    /// (`CZ_REQ_WEAPONREFINE`, 0x0222) while that menu skill is pending
    /// (`clif_parse_WeaponRefine`, `clif.cpp:13255`).
    WeaponRefineList {
        entries: Vec<WeaponRefineListEntry>,
    },
    /// Result of a weapon refine attempt (`ZC_ACK_WEAPONREFINE`, 0x0223).
    ///
    /// `result` codes (`clif_upgrademessage`, `clif.cpp:6930`-`6936`, sent at
    /// `skill.cpp:20703`/`20710`/`20751`/`20780`): `0` = success, `1` = the
    /// refine failed, `2` = the refine level is already at the skill limit,
    /// `3` = the required refine material is missing.
    WeaponRefineResult {
        result: i32,
        item_id: ItemId,
    },
    /// New refine level of an item after a refine attempt
    /// (`ZC_ACK_ITEMREFINING`, 0x0188).
    ItemRefiningResult {
        index: InventoryIndex,
        refine: u16,
        result: u16,
    },
    /// The server offers the damaged items that can be repaired
    /// (`ZC_REPAIRITEMLIST`, 0x01FC). The row indexes are raw server slots.
    RepairItemList {
        entries: Vec<RepairItemListEntry>,
    },
    /// Result of a repair request (`ZC_ACK_ITEMREPAIR`, 0x01FE).
    RepairItemResult {
        index: InventoryIndex,
        success: bool,
    },
    /// The server offers the items that can be identified
    /// (`ZC_ITEMIDENTIFY_LIST`, 0x0177), sent by the merchant Item Appraisal
    /// skill `MC_IDENTIFY` (`skills/merchant/itemappraisal.cpp:17`).
    IdentifyItemList {
        indexes: Vec<InventoryIndex>,
    },
    /// Result of an identify request (`ZC_ACK_ITEMIDENTIFY`, 0x0179).
    IdentifyItemResult {
        index: InventoryIndex,
        success: bool,
    },
    /// The server offers the equipment a card can be inserted into
    /// (`ZC_ITEMCOMPOSITION_LIST`, 0x017B).
    CardCompositionList {
        indexes: Vec<InventoryIndex>,
    },
    /// Result of a card insertion (`ZC_ACK_ITEMCOMPOSITION`, 0x017D).
    CardCompositionResult {
        equip_index: InventoryIndex,
        card_index: InventoryIndex,
        success: bool,
    },
    /// Card slot update (`ZC_UPDATE_CARDSLOT`, 0x0A3F).
    ///
    /// rAthena 20190605 defines the packet but never sends it; card inserts are
    /// reported through `CardCompositionResult` (0x017D) plus the item list
    /// refresh. The event is emitted anyway so a server that uses the packet
    /// keeps the card slots in sync.
    CardSlotUpdated {
        wear_state: i16,
        card_slot: i16,
        item_id: ItemId,
        equip_flag: i8,
    },
    /// The server opened the NPC/atcommand refine UI
    /// (`ZC_OPEN_REFINING_UI`, 0x0AA0), e.g. from the `refineui` script
    /// command of the refine NPCs (`npc/merchants/refine.txt`).
    RefiningUiOpened,
    /// Materials and success chances for the item selected in the refine UI
    /// (`ZC_REFINING_MATERIAL_LIST`, 0x0AA2). The server re-sends this list
    /// after every refine attempt.
    RefiningMaterialList {
        item_index: InventoryIndex,
        blacksmith_blessing: i8,
        materials: Vec<RefiningMaterialEntry>,
    },
    /// The server asks the client to show an NPC illustration
    /// (`ZC_SHOW_IMAGE2`, 0x01B3).
    ///
    /// `image_name` is the GRF texture name or path from the script `cutin`
    /// command; `location` follows the `ImageLocation` enum of the packet
    /// (`255` clears every displayed image).
    DisplayImage {
        image_name: String,
        location: ImageLocation,
    },
    /// The server wants a book opened (`ZC_READ_BOOK`, 0x0294).
    ///
    /// The packet carries only the book id and the page; the text is stored in
    /// the client archive under `data\book\<book id>.txt`.
    OpenBook {
        book_id: u32,
        page: u32,
    },
    /// `HC_ACK_PINCODE` (0x08B9) answer of the character server.
    ///
    /// `state` is the rAthena `pincode_state`: `0`/`7` = passed, `1` = ask,
    /// `2`/`4` = create a new PIN, `3` = expired (change required),
    /// `8` = wrong PIN.
    PincodeState {
        pincode_seed: u32,
        account_id: AccountId,
        state: u16,
    },
    /// `ZC_ACK_CREATE_CHATROOM` (0x00D6).
    ChatRoomCreated {
        flag: u8,
    },
    /// `ZC_ROOM_NEWENTRY` (0x00D7): a nearby chat room was announced.
    ChatRoomAnnounced {
        owner_id: AccountId,
        chat_id: u32,
        limit: u16,
        users: u16,
        room_type: u8,
        title: String,
    },
    /// `ZC_DESTROY_ROOM` (0x00D8).
    ChatRoomDestroyed {
        chat_id: u32,
    },
    /// `ZC_REFUSE_ENTER_ROOM` (0x00DA).
    ChatRoomEnterRefused {
        result: u8,
    },
    /// `ZC_ENTER_ROOM` (0x00DB): the local player joined a chat room.
    ChatRoomEntered {
        chat_id: u32,
        members: Vec<ChatRoomMember>,
    },
    /// `ZC_MEMBER_NEWENTRY` (0x00DC).
    ChatRoomMemberJoined {
        count: u16,
        name: String,
    },
    /// `ZC_MEMBER_EXIT` (0x00DD).
    ChatRoomMemberLeft {
        count: u16,
        name: String,
        kicked: bool,
    },
    /// `ZC_CHANGE_CHATROOM` (0x00DF).
    ChatRoomChanged {
        owner_id: AccountId,
        chat_id: u32,
        limit: u16,
        users: u16,
        room_type: u8,
        title: String,
    },
    /// `ZC_ROLE_CHANGE` (0x00E1): the room owner changed.
    ChatRoomRoleChanged {
        flag: u32,
        name: String,
    },
    /// The equipped items of the equip-switch window (`ZC_EQUIPSWITCH_LIST`,
    /// 0x0A9B, `clif_equipswitch_list`). Every entry is an inventory index and
    /// the equipment position(s) that item is pre-assigned to; the client
    /// marks the items as equipped so the paper doll and the normal
    /// equip/unequip flow see them.
    EquipmentSwitchItems {
        items: Vec<ragnarok_packets::EquippableSwitchItemInformation>,
    },
    /// Full achievement list (`ZC_ALL_ACH_LIST`, 0x0A23).
    ///
    /// The 2019 server sends this on map entry (rAthena
    /// `clif_achievement_list_all`, `clif.cpp:21767`). The packet carries the
    /// progress counters only; the achievement titles/descriptions are client
    /// Lua data that this client data set does not ship.
    AchievementList {
        achievements: Vec<AchievementData>,
        total_score: u32,
        level: u16,
        experience: u32,
        experience_to_next_level: u32,
    },
    /// A single achievement was started, progressed, completed or rewarded
    /// (`ZC_ACH_UPDATE`, 0x0A24).
    AchievementUpdate {
        achievement: AchievementData,
        total_score: u32,
        level: u16,
        experience: u32,
        experience_to_next_level: u32,
    },
    /// Result of a reward claim (`ZC_REQ_ACH_REWARD_ACK`, 0x0A26).
    AchievementRewardResult {
        achievement_id: u32,
        /// `true` when the server granted the reward.
        success: bool,
    },
    /// Sense (`WZ_ESTIMATION`) result for a monster (`ZC_MONSTER_INFO`,
    /// 0x018C).
    MonsterInfo {
        information: MonsterInfoPacket,
    },
    // --- Party booking ("party search" / LFG, 2019) -------------------------
    /// `ZC_PARTY_BOOKING_ACK_REGISTER` (0x0803): result of registering an
    /// advertisement (`0` success, `1` failure, `2` already registered).
    PartyBookingRegisterResult {
        result: u16,
    },
    /// `ZC_PARTY_BOOKING_ACK_SEARCH` (0x0805): one page of search results.
    PartyBookingSearchResult {
        more_results: bool,
        entries: Vec<PartyBookingEntry2019>,
    },
    /// `ZC_PARTY_BOOKING_ACK_DELETE` (0x0807): result of deleting the own
    /// advertisement.
    PartyBookingDeleteResult {
        result: u16,
    },
    /// `ZC_PARTY_BOOKING_NOTIFY_INSERT` (0x0809): a new advertisement.
    PartyBookingInserted {
        entry: PartyBookingEntry2019,
    },
    /// `ZC_PARTY_BOOKING_NOTIFY_UPDATE` (0x080A): changed desired jobs.
    PartyBookingUpdated {
        index: u32,
        jobs: [u16; 6],
    },
    /// `ZC_PARTY_BOOKING_NOTIFY_DELETE` (0x080B): removed advertisement.
    PartyBookingDeleted {
        index: u32,
    },
    /// `ZC_PARTY_REQ_MASTER_TO_JOIN` (0x0AE7): another player asked to join
    /// the local player's party through the booking board.
    ///
    /// Only sent by servers with `PACKETVER >= 20191204`; the CondaRO 20190605
    /// server does not register this packet.
    PartyBookingJoinRequest {
        character_id: CharacterId,
        account_id: AccountId,
        name: String,
        position: TilePosition,
    },
    /// `ZC_PARTY_JOIN_REQ_ACK_FROM_MASTER` (0x0AFA): the party leader answered
    /// a booking join request.
    PartyBookingJoinResult {
        player_name: String,
        party_name: String,
        account_id: AccountId,
        refused: bool,
    },
    // --- Clan (2019) --------------------------------------------------------
    /// `ZC_CLANINFO` (0x098A): basic clan information. The 2019 packet set has
    /// no clan member list; this is everything the server sends.
    ClanInfo {
        info: ClanInfoPacket,
    },
    /// `ZC_NOTIFY_CLAN_CONNECTINFO` (0x0988): online/maximum clan members.
    ClanOnlineCount {
        online_members: u16,
        maximum_members: u16,
    },
    // --- Pet / homunculus / mercenary --------------------------------------
    /// `ZC_PROPERTY_PET` (0x01A2): base data of the local player's pet.
    PetProperty {
        property: PetPropertyPacket2019,
    },
    /// `ZC_CHANGESTATE_PET` (0x01A4): intimacy/hunger/accessory change.
    PetStateChanged {
        entity_id: EntityId,
        state_type: u8,
        value: i32,
    },
    /// `ZC_PROPERTY_HOMUN` (0x09F7): full homunculus status sheet.
    HomunculusProperty {
        property: HomunculusPropertyPacket2019,
    },
    /// `ZC_HO_PAR_CHANGE` (0x07DB): homunculus HP/SP/EXP change.
    HomunculusParameterChanged {
        parameter: u16,
        value: i32,
    },
    /// `ZC_MER_INIT` (0x029B): full mercenary status sheet.
    MercenaryInfo {
        info: MercenaryInfoPacket2019,
    },
    /// `ZC_MER_PAR_CHANGE` (0x02A2): mercenary HP/SP/faith/kills change.
    MercenaryParameterChanged {
        parameter: u16,
        value: i32,
    },
}

/// Party member information sent by `ZC_ADD_MEMBER_TO_GROUP` (0x0AE4).
#[derive(Debug, Clone)]
pub struct PartyMemberDetails {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub name: String,
    pub party_name: String,
    pub map_name: String,
    pub job_id: JobId,
    pub base_level: u16,
    pub position: TilePosition,
    pub online: bool,
    pub leader: bool,
}

/// New-type so we can implement some `From` traits. This will help when
/// registering the packet handlers.
#[derive(Default)]
pub(crate) struct NetworkEventList(pub Vec<NetworkEvent>);

pub(crate) struct NoNetworkEvents;

impl From<NetworkEvent> for NetworkEventList {
    fn from(event: NetworkEvent) -> Self {
        Self(vec![event])
    }
}

impl From<Vec<NetworkEvent>> for NetworkEventList {
    fn from(events: Vec<NetworkEvent>) -> Self {
        Self(events)
    }
}

impl From<Option<NetworkEvent>> for NetworkEventList {
    fn from(event: Option<NetworkEvent>) -> Self {
        match event {
            Some(event) => Self(vec![event]),
            None => Self(Vec::new()),
        }
    }
}

impl From<NoNetworkEvents> for NetworkEventList {
    fn from(_: NoNetworkEvents) -> Self {
        Self(Vec::new())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    ClosedByClient,
    ConnectionError,
}

pub(crate) trait DisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent;
}

pub(crate) struct LoginServerDisconnectedEvent;
pub(crate) struct CharacterServerDisconnectedEvent;
pub(crate) struct MapServerDisconnectedEvent;

impl DisconnectedEvent for LoginServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::LoginServerDisconnected { reason }
    }
}

impl DisconnectedEvent for CharacterServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::CharacterServerDisconnected { reason }
    }
}

impl DisconnectedEvent for MapServerDisconnectedEvent {
    fn create_event(reason: DisconnectReason) -> NetworkEvent {
        NetworkEvent::MapServerDisconnected { reason }
    }
}
