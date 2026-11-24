use std::time::Instant;

use ragnarok_packets::*;

use crate::hotkey::HotkeyState;
use crate::items::ShopItem;
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
