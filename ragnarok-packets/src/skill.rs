//! skill packets.
//!
//! Split out of the crate root; the crate re-exports every module, so a
//! packet type is used through `ragnarok_packets::Type` regardless of the
//! file it lives in.

use super::*;

/// An entity's skill cast was interrupted (`ZC_DISPEL`, 0x01B9).
///
/// rAthena sends this from `clif_skillcastcancel` to everyone in the area when
/// a cast is cancelled (movement, damage, another skill, ...). The packet only
/// carries the entity id: the client has to stop the looping cast animation and
/// clear its cast bar.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01B9)]
#[packet_size(6)]
pub struct SkillCastCancelPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x013A)]
#[packet_size(4)]
pub struct UpdateAttackRangePacket {
    pub attack_range: AttackRange,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0139)]
#[packet_size(16)]
pub struct RequestPlayerAttackFailedPacket {
    pub target_entity_id: EntityId,
    pub target_position: TilePosition,
    pub player_position: TilePosition,
    pub attack_range: AttackRange,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub enum DamageType {
    Damage,
    PickUpItem,
    SitDown,
    StandUp,
    DamageEndure, // Not confirmed
    Splash,       // Not confirmed
    Skill,        // Not confirmed
    RepeatDamage, // Not confirmed
    MultiHitDamage,
    MultiHitDamageEndure, // Not confirmed
    CriticalHit,
    LuckyDodge,
    TouchSkill, // Not confirmed
    CriticalMultiHit,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x008A)]
pub struct DamagePacket1 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub attack_duration: u32,
    pub damage_delay: u32,
    pub damage_amount: i16,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: i16,
}

// FIX: This one is the attack animation one
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x08C8)]
#[packet_size(34)]
pub struct DamagePacket3 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub attack_duration: u32,
    pub damage_delay: u32,
    pub damage_amount: u32,
    pub is_special_damage: u8,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: u32,
}

/// 2019 era `ZC_EFST_SET_ENTER` (0x0984): a status effect (buff/debuff) that is
/// already active when the entity enters the view, also sent for every active
/// status effect on map load.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0984)]
#[packet_size(28)]
pub struct EnterStatusEffectPacket2019 {
    pub entity_id: EntityId,
    pub effect_id: u16,
    pub duration: u32,
    pub remaining_duration: u32,
    pub value1: i32,
    pub value2: i32,
    pub value3: i32,
}

/// 2019 era `ZC_AUTORUN_SKILL` (0x0147): notification of an auto-casted
/// skill. The server sends this for skill items (e.g. fly wing ->
/// `itemskill "AL_TELEPORT",1`); the client is expected to perform the skill
/// use itself (send a normal skill request), the item is not consumed before
/// that.
///
/// Layout: `0147 <skill id>.W <type>.L <level>.W <sp cost>.W <range>.W
/// <skill name>.24B <upgradable>.B`.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0147)]
pub struct AutoRunSkillPacket {
    pub skill_id: SkillId,
    pub skill_type: u32,
    pub skill_level: u16,
    pub skill_sp: u16,
    pub skill_range: u16,
    #[length(24)]
    pub skill_name: String,
    pub upgradable: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum SkillType {
    #[numeric_value(0)]
    Passive,
    #[numeric_value(1)]
    Attack,
    #[numeric_value(2)]
    Ground,
    #[numeric_value(4)]
    SelfCast,
    #[numeric_value(16)]
    Support,
    #[numeric_value(32)]
    Trap,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
pub struct SkillInformation {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    #[length(24)]
    pub skill_name: String,
    // TODO: bool
    pub upgradable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x010F)]
#[variable_length]
pub struct UpdateSkillTreePacket {
    #[repeating_remaining]
    pub skill_information: Vec<SkillInformation>,
}

/// Sent by the server when the level/SP cost/range of a known skill changes
/// (`ZC_SKILLINFO_UPDATE2`, 0x07E1 on this packet version).
#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x07E1)]
#[packet_size(15)]
pub struct SkillInfoUpdatePacket {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    // TODO: bool
    pub upgradable: u8,
}

/// Sent by the server when a single skill is learned or upgraded
/// (`ZC_ADD_SKILL`).
///
/// For PACKETVER 20190605 the server uses the `SKILLDATA` layout (skill id,
/// `i32` type flags, level, SP cost, range, name[24], upgradable flag), which
/// matches [`SkillInformation`].
#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0111)]
#[packet_size(39)]
pub struct AddSkillPacket {
    pub skill_information: SkillInformation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum EffectId {
    Hit1,
    Hit2,
    Hit3,
    Hit4,
    Hit5,
    Hit6,
    Entry,
    Exit,
    Warp,
    Enhance,
    Coin,
    Endure,
    Beginspell,
    Glasswall,
    Healsp,
    Soulstrike,
    Bash,
    Magnumbreak,
    Steal,
    Hiding,
    Pattack,
    Detoxication,
    Sight,
    Stonecurse,
    Fireball,
    Firewall,
    Icearrow,
    Frostdiver,
    Frostdiver2,
    Lightbolt,
    Thunderstorm,
    Firearrow,
    Napalmbeat,
    Ruwach,
    Teleportation,
    Readyportal,
    Portal,
    Incagility,
    Decagility,
    Aqua,
    Signum,
    Angelus,
    Blessing,
    Incagidex,
    Smoke,
    Firefly,
    Sandwind,
    Torch,
    Spraypond,
    Firehit,
    Firesplashhit,
    Coldhit,
    Windhit,
    Poisonhit,
    Beginspell2,
    Beginspell3,
    Beginspell4,
    Beginspell5,
    Beginspell6,
    Beginspell7,
    Lockon,
    Warpzone,
    Sightrasher,
    Barrier,
    Arrowshot,
    Invenom,
    Cure,
    Provoke,
    Mvp,
    Skidtrap,
    Brandishspear,
    Cone,
    Sphere,
    Bowlingbash,
    Icewall,
    Gloria,
    Magnificat,
    Resurrection,
    Recovery,
    Earthspike,
    Spearbmr,
    Pierce,
    Turnundead,
    Sanctuary,
    Impositio,
    Lexaeterna,
    Aspersio,
    Lexdivina,
    Suffragium,
    Stormgust,
    Lord,
    Benedictio,
    Meteorstorm,
    Yufitel,
    Yufitelhit,
    Quagmire,
    Firepillar,
    Firepillarbomb,
    Hasteup,
    Flasher,
    Removetrap,
    Repairweapon,
    Crashearth,
    Perfection,
    Maxpower,
    Blastmine,
    Blastminebomb,
    Claymore,
    Freezing,
    Bubble,
    Gaspush,
    Springtrap,
    Kyrie,
    Magnus,
    Bottom,
    Blitzbeat,
    Waterball,
    Waterball2,
    Fireivy,
    Detecting,
    Cloaking,
    Sonicblow,
    Sonicblowhit,
    Grimtooth,
    Venomdust,
    Enchantpoison,
    Poisonreact,
    Poisonreact2,
    Overthrust,
    Splasher,
    Twohandquicken,
    Autocounter,
    Grimtoothatk,
    Freeze,
    Freezed,
    Icecrash,
    Slowpoison,
    Bottom2,
    Firepillaron,
    Sandman,
    Revive,
    Pneuma,
    Heavensdrive,
    Sonicblow2,
    Brandish2,
    Shockwave,
    Shockwavehit,
    Earthhit,
    Pierceself,
    Bowlingself,
    Spearstabself,
    Spearbmrself,
    Holyhit,
    Concentration,
    Refineok,
    Refinefail,
    Jobchange,
    Lvup,
    Joblvup,
    Toprank,
    Party,
    Rain,
    Snow,
    Sakura,
    StatusState,
    Banjjakii,
    Makeblur,
    Tamingsuccess,
    Tamingfailed,
    Energycoat,
    Cartrevolution,
    Venomdust2,
    Changedark,
    Changefire,
    Changecold,
    Changewind,
    Changeflame,
    Changeearth,
    Chaingeholy,
    Changepoison,
    Hitdark,
    Mentalbreak,
    Magicalatthit,
    SuiExplosion,
    Darkattack,
    Suicide,
    Comboattack1,
    Comboattack2,
    Comboattack3,
    Comboattack4,
    Comboattack5,
    Guidedattack,
    Poisonattack,
    Silenceattack,
    Stunattack,
    Petrifyattack,
    Curseattack,
    Sleepattack,
    Telekhit,
    Pong,
    Level99,
    Level99_2,
    Level99_3,
    Gumgang,
    Potion1,
    Potion2,
    Potion3,
    Potion4,
    Potion5,
    Potion6,
    Potion7,
    Potion8,
    Darkbreath,
    Deffender,
    Keeping,
    Summonslave,
    Blooddrain,
    Energydrain,
    PotionCon,
    Potion_,
    PotionBerserk,
    Potionpillar,
    Defender,
    Ganbantein,
    Wind,
    Volcano,
    Grandcross,
    Intimidate,
    Chookgi,
    Cloud,
    Cloud2,
    Mappillar,
    Linelink,
    Cloud3,
    Spellbreaker,
    Dispell,
    Deluge,
    Violentgale,
    Landprotector,
    BottomVo,
    BottomDe,
    BottomVi,
    BottomLa,
    Fastmove,
    Magicrod,
    Holycross,
    Shieldcharge,
    Mappillar2,
    Providence,
    Shieldboomerang,
    Spearquicken,
    Devotion,
    Reflectshield,
    Absorbspirits,
    Steelbody,
    Flamelauncher,
    Frostweapon,
    Lightningloader,
    Seismicweapon,
    Mappillar3,
    Mappillar4,
    Gumgang2,
    Teihit1,
    Gumgang3,
    Teihit2,
    Tanji,
    Teihit1x,
    Chimto,
    Stealcoin,
    Stripweapon,
    Stripshield,
    Striparmor,
    Striphelm,
    Chaincombo,
    RgCoin,
    Backstap,
    Teihit3,
    BottomDissonance,
    BottomLullaby,
    BottomRichmankim,
    BottomEternalchaos,
    BottomDrumbattlefield,
    BottomRingnibelungen,
    BottomRokisweil,
    BottomIntoabyss,
    BottomSiegfried,
    BottomWhistle,
    BottomAssassincross,
    BottomPoembragi,
    BottomAppleidun,
    BottomUglydance,
    BottomHumming,
    BottomDontforgetme,
    BottomFortunekiss,
    BottomServiceforyou,
    TalkFrostjoke,
    TalkScream,
    Pokjuk,
    Throwitem,
    Throwitem2,
    Chemicalprotection,
    PokjukSound,
    Demonstration,
    Chemical2,
    Teleportation2,
    PharmacyOk,
    PharmacyFail,
    Forestlight,
    Throwitem3,
    Firstaid,
    Sprinklesand,
    Loud,
    Heal,
    Heal2,
    Exit2,
    Glasswall2,
    Readyportal2,
    Portal2,
    BottomMag,
    BottomSanc,
    Heal3,
    Warpzone2,
    Forestlight2,
    Forestlight3,
    Forestlight4,
    Heal4,
    Foot,
    Foot2,
    Beginasura,
    Tripleattack,
    Hitline,
    Hptime,
    Sptime,
    Maple,
    Blind,
    Poison,
    Guard,
    Joblvup50,
    Angel2,
    Magnum2,
    Callzone,
    Portal3,
    Couplecasting,
    Heartcasting,
    Entry2,
    Saintwing,
    Spherewind,
    Colorpaper,
    Lightsphere,
    Waterfall,
    Waterfall90,
    WaterfallSmall,
    WaterfallSmall90,
    WaterfallT2,
    WaterfallT2_90,
    WaterfallSmallT2,
    WaterfallSmallT2_90,
    MiniTetris,
    Ghost,
    Bat,
    Bat2,
    Soulbreaker,
    Level99_4,
    Vallentine,
    Vallentine2,
    Pressure,
    Bash3d,
    Aurablade,
    Redbody,
    Lkconcentration,
    BottomGospel,
    Angel,
    Devil,
    Dragonsmoke,
    BottomBasilica,
    Assumptio,
    Hitline2,
    Bash3d2,
    Energydrain2,
    Transbluebody,
    Magiccrasher,
    Lightsphere2,
    Lightblade,
    Energydrain3,
    Linelink2,
    Linklight,
    Truesight,
    Falconassault,
    Tripleattack2,
    Portal4,
    Meltdown,
    Cartboost,
    Rejectsword,
    Tripleattack3,
    Spherewind2,
    Linelink3,
    Pinkbody,
    Level99_5,
    Level99_6,
    Bash3d3,
    Bash3d4,
    Napalmvalcan,
    Portal5,
    Magiccrasher2,
    BottomSpider,
    BottomFogwall,
    Soulburn,
    Soulchange,
    Baby,
    Soulbreaker2,
    Rainbow,
    Peong,
    Tanji2,
    Pressedbody,
    Spinedbody,
    Kickedbody,
    Airtexture,
    Hitbody,
    Doublegumgang,
    Reflectbody,
    Babybody,
    Babybody2,
    Giantbody,
    Giantbody2,
    Asurabody,
    _4waybody,
    Quakebody,
    AsurabodyMonster,
    Hitline3,
    Hitline4,
    Hitline5,
    Hitline6,
    Electric,
    Electric2,
    Hitline7,
    Stormkick,
    Halfsphere,
    Attackenergy,
    Attackenergy2,
    Chemical3,
    Assumptio2,
    Bluecasting,
    Run,
    Stoprun,
    Stopeffect,
    Jumpbody,
    Landbody,
    Foot3,
    Foot4,
    TaeReady,
    Grandcross2,
    Soulstrike2,
    Yufitel2,
    NpcStop,
    Darkcasting,
    Gumgangnpc,
    Agiup,
    Jumpkick,
    Quakebody2,
    Stormkick1,
    Stormkick2,
    Stormkick3,
    Stormkick4,
    Stormkick5,
    Stormkick6,
    Stormkick7,
    Spinedbody2,
    Beginasura1,
    Beginasura2,
    Beginasura3,
    Beginasura4,
    Beginasura5,
    Beginasura6,
    Beginasura7,
    Aurablade2,
    Devil1,
    Devil2,
    Devil3,
    Devil4,
    Devil5,
    Devil6,
    Devil7,
    Devil8,
    Devil9,
    Devil10,
    Doublegumgang2,
    Doublegumgang3,
    Blackdevil,
    Flowercast,
    Flowercast2,
    Flowercast3,
    Mochi,
    Lamadan,
    Edp,
    Shieldboomerang2,
    RgCoin2,
    Guard2,
    Slim,
    Slim2,
    Slim3,
    Chemicalbody,
    Castspin,
    Piercebody,
    Soullink,
    Chookgi2,
    Memorize,
    Soullight,
    Mapae,
    Itempokjuk,
    _05val,
    Beginasura11,
    Night,
    Chemical2dash,
    Groundsample,
    GiExplosion,
    Cloud4,
    Cloud5,
    BottomHermode,
    Cartter,
    Itemfast,
    Shieldboomerang3,
    Doublecastbody,
    Gravitation,
    Tarotcard1,
    Tarotcard2,
    Tarotcard3,
    Tarotcard4,
    Tarotcard5,
    Tarotcard6,
    Tarotcard7,
    Tarotcard8,
    Tarotcard9,
    Tarotcard10,
    Tarotcard11,
    Tarotcard12,
    Tarotcard13,
    Tarotcard14,
    Aciddemon,
    Greenbody,
    Throwitem4,
    BabybodyBack,
    Throwitem5,
    Bluebody,
    Hated,
    Redlightbody,
    Ro2year,
    SmaReady,
    Stin,
    RedHit,
    BlueHit,
    Quakebody3,
    Sma,
    Sma2,
    Stin2,
    Hittexture,
    Stin3,
    Sma3,
    Bluefall,
    Bluefall90,
    Fastbluefall,
    Fastbluefall90,
    BigPortal,
    BigPortal2,
    ScreenQuake,
    Homuncasting,
    Hflimoon1,
    Hflimoon2,
    Hflimoon3,
    HoUp,
    Hamidefence,
    Hamicastle,
    Hamiblood,
    Hated2,
    Twilight1,
    Twilight2,
    Twilight3,
    ItemThunder,
    ItemCloud,
    ItemCurse,
    ItemZzz,
    ItemRain,
    ItemLight,
    Angel3,
    M01,
    M02,
    M03,
    M04,
    M05,
    M06,
    M07,
    Kaizel,
    Kaahi,
    Cloud6,
    Food01,
    Food02,
    Food03,
    Food04,
    Food05,
    Food06,
    Shrink,
    Throwitem6,
    Sight2,
    Quakebody4,
    Firehit2,
    NpcStop2,
    NpcStop2Del,
    Fvoice,
    Wink,
    CookingOk,
    CookingFail,
    TempOk,
    TempFail,
    Hapgyeok,
    Throwitem7,
    Throwitem8,
    Throwitem9,
    Throwitem10,
    Bunsinjyutsu,
    Kouenka,
    Hyousensou,
    BottomSuiton,
    Stin4,
    Thunderstorm2,
    Chemical4,
    Stin5,
    MadnessBlue,
    MadnessRed,
    RgCoin3,
    Bash3d5,
    Chookgi3,
    Kirikage,
    Tatami,
    Kasumikiri,
    Issen,
    Kaen,
    Baku,
    Hyousyouraku,
    Desperado,
    LightningS,
    BlindS,
    PoisonS,
    FreezingS,
    FlareS,
    Rapidshower,
    Magicalbullet,
    Spreadattack,
    Trackcasting,
    Tracking,
    Tripleaction,
    Bullseye,
    MapMagiczone,
    MapMagiczone2,
    Damage1,
    Damage1_2,
    Damage1_3,
    Undeadbody,
    UndeadbodyDel,
    GreenNumber,
    BlueNumber,
    RedNumber,
    PurpleNumber,
    BlackNumber,
    WhiteNumber,
    YellowNumber,
    PinkNumber,
    BubbleDrop,
    NpcEarthquake,
    DaSpace,
    Dragonfear,
    Bleeding,
    Wideconfuse,
    BottomRunner,
    BottomTransfer,
    CrystalBlue,
    BottomEvilland,
    Guard3,
    NpcSlowcast,
    Criticalwound,
    Green99_3,
    Green99_5,
    Green99_6,
    Mapsphere,
    PokLove,
    PokWhite,
    PokValen,
    PokBirth,
    PokChristmas,
    MapMagiczone3,
    MapMagiczone4,
    Dust,
    TorchRed,
    TorchGreen,
    MapGhost,
    Glow1,
    Glow2,
    Glow4,
    TorchPurple,
    Cloud7,
    Cloud8,
    Flowerleaf,
    Mapsphere2,
    Glow11,
    Glow12,
    Circlelight,
    Item315,
    Item316,
    Item317,
    Item318,
    StormMin,
    PokJap,
    MapGreenlight,
    MapMagicwall,
    MapGreenlight2,
    Yellowfly1,
    Yellowfly2,
    BottomBlue,
    BottomBlue2,
    Wewish,
    Firepillaron2,
    Forestlight5,
    Soulbreaker3,
    AdoStr,
    IgnStr,
    Chimto2,
    Windcutter,
    Detect2,
    Frostmysty,
    CrimsonStr,
    HellStr,
    SprMash,
    SprSoule,
    DhowlStr,
    Earthwall,
    Soulbreaker4,
    ChainlStr,
    ChookgiFire,
    ChookgiWind,
    ChookgiWater,
    ChookgiGround,
    MagentaTrap,
    CobaltTrap,
    MaizeTrap,
    VerdureTrap,
    NormalTrap,
    Cloaking2,
    AimedStr,
    ArrowstormStr,
    LaulamusStr,
    LauagnusStr,
    MilshieldStr,
    Concentration2,
    Fireball2,
    Bunsinjyutsu2,
    Cleartime,
    Glasswall3,
    Oratio,
    PotionBerserk2,
    Circlepower,
    Rolling1,
    Rolling2,
    Rolling3,
    Rolling4,
    Rolling5,
    Rolling6,
    Rolling7,
    Rolling8,
    Rolling9,
    Rolling10,
    Purplebody,
    Stin6,
    RgCoin4,
    Poisonwav,
    Poisonsmoke,
    Gumgang4,
    Shieldboomerang4,
    Castspin2,
    Vulcanwav,
    Agiup2,
    Detect3,
    Agiup3,
    Detect4,
    Electric3,
    Guard4,
    BottomBarrier,
    BottomStealth,
    Repairtime,
    NcAnal,
    Firethrow,
    Venomimpress,
    Frostmisty,
    Burning,
    Coldthrow,
    Makehallu,
    Hallutime,
    Infraredscan,
    Crashaxe,
    Gthunder,
    Stonering,
    Intimidate2,
    Stasis,
    Redline,
    Frostdiver3,
    BottomBasilica2,
    Recognized,
    Tetra,
    Tetracasting,
    Fireball3,
    Intimidate3,
    Recognized2,
    Cloaking3,
    Intimidate4,
    Stretch,
    Blackbody,
    Enervation,
    Enervation2,
    Enervation3,
    Enervation4,
    Enervation5,
    Enervation6,
    Linelink4,
    RgCoin5,
    WaterfallAni,
    BottomManhole,
    Manhole,
    Makefeint,
    Forestlight6,
    Darkcasting2,
    BottomAni,
    BottomMaelstrom,
    BottomBloodylust,
    BeginspellN1,
    BeginspellN2,
    HealN,
    ChookgiN,
    Joblvup50_2,
    Chemical2dash2,
    Chemical2dash3,
    Rollingcast,
    WaterBelow,
    WaterFade,
    BeginspellN3,
    BeginspellN4,
    BeginspellN5,
    BeginspellN6,
    BeginspellN7,
    BeginspellN8,
    WaterSmoke,
    Dance1,
    Dance2,
    Linkparticle,
    Soullight2,
    SprParticle,
    SprParticle2,
    SprPlant,
    ChemicalV,
    Shootparticle,
    BotReverb,
    RainParticle,
    ChemicalV2,
    Secra,
    BotReverb2,
    Circlepower2,
    Secra2,
    ChemicalV3,
    Enervation7,
    Circlepower3,
    SprPlant2,
    Circlepower4,
    SprPlant3,
    RgCoin6,
    SprPlant4,
    Circlepower5,
    SprPlant5,
    Circlepower6,
    SprPlant6,
    Circlepower7,
    SprPlant7,
    Circlepower8,
    SprPlant8,
    Heartasura,
    Beginspell150,
    Level99_150,
    Primecharge,
    Glasswall4,
    GradiusLaser,
    Bash3d6,
    Gumgang5,
    Hitline8,
    Electric4,
    Teihit1t,
    Spinmove,
    Fireball4,
    Tripleattack4,
    Chemical3s,
    Groundshake,
    Dq9Charge,
    Dq9Charge2,
    Dq9Charge3,
    Dq9Charge4,
    Blueline,
    Selfscroll,
    SprLightprint,
    PngTest,
    BeginspellYb,
    Chemical2dash4,
    Groundshake2,
    Pressure2,
    RgCoin7,
    Primecharge2,
    Primecharge3,
    Primecharge4,
    Greencasting,
    Wallofthorn,
    Fireball5,
    Throwitem11,
    SprPlant9,
    Demonicfire,
    Demonicfire2,
    Demonicfire3,
    Hellsplant,
    Firewall2,
    Vacuum,
    SprPlant10,
    SprLightprint2,
    Poisonsmoke2,
    Makehallu2,
    Shockwave2,
    SprPlant11,
    Coldthrow2,
    Demonicfire4,
    Pressure3,
    Linkparticle2,
    Soullight3,
    Chareffect,
    Gumgang6,
    Fireball6,
    Gumgang7,
    Gumgang8,
    Gumgang9,
    BottomDe2,
    Coldstatus,
    SprLightprint3,
    Waterball3,
    HealN2,
    RainParticle2,
    Cloud9,
    Yellowfly3,
    ElGust,
    ElBlast,
    ElAquaplay,
    ElUpheaval,
    ElWildStorm,
    ElChillyAir,
    ElCursedSoil,
    ElCooler,
    ElTropic,
    ElPyrotechnic,
    ElPetrology,
    ElHeater,
    PoisonMist,
    EraserCutter,
    SilentBreeze,
    MagmaFlow,
    Graybody,
    LavaSlide,
    SonicClaw,
    TinderBreaker,
    MidnightFrenzy,
    Macro,
    ChemicalAllrange,
    TetraFire,
    TetraWater,
    TetraWind,
    TetraGround,
    Emitter,
    VolcanicAsh,
    Level99Orb1,
    Level99Orb2,
    Level150,
    Level150Sub,
    Throwitem4_1,
    ThrowHappokunai,
    ThrowMultipleCoin,
    ThrowBakuretsu,
    RotateHuumaranka,
    RotateBg,
    RotateLineGray,
    _2011rwc,
    _2011rwc2,
    Kaihou,
    GroundExplosion,
    KgKagehumi,
    KoZenkaiWater,
    KoZenkaiLand,
    KoZenkaiFire,
    KoZenkaiWind,
    KoJyumonjikiri,
    KoSetsudan,
    RedCross,
    KoIzayoi,
    RotateLineBlue,
    KgKyomu,
    KoHuumaranka,
    Bluelightbody,
    Kagemusya,
    ObGensou,
    No100Firecracker,
    KoMakibishi,
    Kaihou1,
    Akaitsuki,
    Zangetsu,
    Gensou,
    HatEffect,
    Cherryblossom,
    EventCloud,
    RunMakeOk,
    RunMakeFailure,
    MiresultMakeOk,
    MiresultMakeFail,
    AllRayOfProtection,
    Venomfog,
    Duststorm,
    Level160,
    Level160Sub,
    Mapchain,
    MagicFloor,
    Icemine,
    Flamecorss,
    Icemine1,
    DanceBladeAtk,
    Darkpiercing,
    Invincibleoff2,
    Maxpain,
    Deathsummon,
    Moonstar,
    Strangelights,
    SuperStar,
    Yellobody,
    Colorpaper2,
    EvilsPaw,
    GcDarkcrow,
    RkDragonbreathWater,
    AllFullThrottle,
    SrFlashcombo,
    RkLuxanima,
    Cloud10,
    SoElementalShield,
    AbOffertorium,
    WlTelekinesisIntense,
    GnIllusiondoping,
    NcMagmaEruption,
    LgKingsGrace,
    Blooddrain2,
    NpcWideweb,
    NpcBurnt,
    NpcChill,
    RaUnlimit,
    AbOffertoriumRing,
    ScEscape,
    WmFriggSong,
    Flicker,
    CMaker,
    HammerOfGod,
    MassSpiral,
    FireRain,
    Whitebody,
    BanishingBuster,
    Slugshot,
    DTail,
    BindTrap1,
    BindTrap2,
    BindTrap3,
    Jumpbody1,
    AnimatedEmitter,
    RlExplosion,
    CMaker1,
    QdShot,
    PAlter,
    SStorm,
    MusicHat,
    CloudKill,
    Escape,
    XenoSlasher,
    Flowersmoke,
    Fstone,
    Qscaraba,
    Ljosalfar,
    Happinessstar,
    PowerOfGaia,
    MapleFalls,
    MarkingUseChangemonster,
    MagicalFeather,
    MermaidLonging,
    GiftOfSnow,
    AchComplete,
    TimeAccessory,
    Spritemable,
    Tunaparty,
    Freshshrimp,
    #[numeric_value(1123)]
    SuGrooming,
    SuChattering,
    /// `EF_SU_STOOP` (`EffectTypes.h:1140`, official id 1132).
    ///
    /// The caster-body stoop effect (`RagEffect.cpp:22202-22207`) was missing
    /// from this enum, leaving no value between `SuChattering` (1124) and
    /// `Firedance` (1133). `22202` is the decompiled case's line number, not
    /// its id. Declared with its real numeric value so neither neighbour
    /// shifts.
    #[numeric_value(1132)]
    SuStoop,
    #[numeric_value(1133)]
    Firedance,
    RichsCoinA,
    #[numeric_value(1137)]
    EChain,
    HeatBarrel,
    HMine,
    FallenAngel,
    #[numeric_value(1149)]
    ImmuneProperty,
    MoveCoordinate,
    #[numeric_value(1197)]
    LightsphereSun,
    LightsphereMoon,
    LightsphereStar,
    #[numeric_value(1202)]
    Novaexplosing,
    StarEmperor,
    SmaBlack,
    /// `EF_SMA_BLACK2` (`EffectTypes.h:1213`, official id 1205).
    ///
    /// The id sits between `SmaBlack` (1204) and `EF_STIN_BLACK` (1206). Its
    /// body is `CRagEffect::BlueRingStack` (`RagEffect.cpp:5829-5877`), the
    /// black twin of `TexturedBlueRing`; `CEffectPrim::RenderSMA3` launches it
    /// when `slots[0].flag1[0] == 99`.
    #[numeric_value(1205)]
    SmaBlack2,
    /// `EF_PEONG_BLACK` (`EffectTypes.h:1215`, official id 1207).
    ///
    /// The id sits between `SmaBlack` (1204) and `EnergydrainBlack` (1208).
    /// Its body (`RagEffect.cpp:22254-22261`) is already implemented as
    /// `SonicSlashFan(1)` plus the caster tint; the enum was the only thing
    /// keeping `EffectId::PeongBlack` from being registered.
    #[numeric_value(1207)]
    PeongBlack,
    #[numeric_value(1208)]
    EnergydrainBlack,
    BlinkBody,
    #[numeric_value(1218)]
    Solarburst,
    SjDocument,
    FallingStar,
    #[numeric_value(1223)]
    Stormkick8,
    #[numeric_value(1229)]
    NewmoonKick,
    FullmoonKick,
    BookOfDimension,
    #[numeric_value(1233)]
    CurseExplosion,
    SoulReaper,
    #[numeric_value(1242)]
    SoulExplosion,
    Max,
    /// `EF_NEW_HEAVENS_DRIVE` (`EffectTypes.h:1407`, official id 1399).
    ///
    /// The body (`RagEffect.cpp:22325-22327`) is the already-ported
    /// `CRagEffect::StoneSpike` (`RagEffect.cpp:13342-13385`), shared with
    /// `EF_HEAVENSDRIVE` (142). Its lifetime is 75, not 142's 250
    /// (`EffectTypes.cpp:1408`).
    #[numeric_value(1399)]
    NewHeavensDrive,
    /// `EF_NEW_MOON_SLASH` (`EffectTypes.h:1426`, official id 1418).
    ///
    /// `RagEffect.cpp:22334-22341` is one `TexturedQuadRingSized(45,
    /// "effect\\new_comet\\new_comet_cast_ring\\ring_rainbow.tga", 20)` after
    /// `PlayEffectWave`. Lifetime 1418 (`EffectTypes.cpp:1427`).
    #[numeric_value(1418)]
    NewMoonSlash,
    /// `EF_NEW_HEAL_BLUE` (`EffectTypes.h:1564`, official id 1556).
    ///
    /// `RagEffect.cpp:22419-22431`: a positional `PlayWave` plus
    /// `TexturedQuadRingSized(45, ring_blue, 20)` and `(25, ring_blue, 21)`.
    /// Lifetime 60 (`EffectTypes.cpp:1565`).
    #[numeric_value(1556)]
    NewHealBlue,
    /// `EF_NEW_HEAL_WHITE` (`EffectTypes.h:1568`, official id 1560).
    ///
    /// `RagEffect.cpp:22457-22464`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 22)` calls.
    /// Lifetime 40 (`EffectTypes.cpp:1569`).
    #[numeric_value(1560)]
    NewHealWhite,
    /// `EF_NEW_HEAL_YELLOW` (`EffectTypes.h:1573`, official id 1565).
    ///
    /// `RagEffect.cpp:22465-22472`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_yellow.tga", 23)` calls.
    /// Lifetime 40 (`EffectTypes.cpp:1574`).
    #[numeric_value(1565)]
    NewHealYellow,
    /// `EF_NEW_METALLICSOUND_LINE` (`EffectTypes.h:1592`, official id 1584).
    /// The effect body is `CRagEffect`'s metallic-sound line
    /// (`RagEffect.cpp:22532-22544`); no pre-1243 packet carries it, so it is
    /// declared after the enum's old `Max` stop with its real numeric value.
    #[numeric_value(1584)]
    NewMetallicSoundLine,
    /// `EF_NEW_SAFETYWALL` (`EffectTypes.h:1685`, official id 1677).
    ///
    /// `RagEffect.cpp:22610-22622`: positional `PlayWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 31)` calls.
    /// Lifetime 60 (`EffectTypes.cpp:1686`).
    #[numeric_value(1677)]
    NewSafetywall,
    /// `EF_NEW_PNEUMA` (`EffectTypes.h:1880`, official id 1872).
    ///
    /// `RagEffect.cpp:22638-22645`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 42)` calls.
    /// Lifetime 1872 (`EffectTypes.cpp:1881`).
    #[numeric_value(1872)]
    NewPneuma,
    /// `EF_NEW_SW_CAST` (`EffectTypes.h:1884`, official id 1876).
    ///
    /// `RagEffect.cpp:22646-22653`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 43)` calls.
    /// Lifetime 1876 (`EffectTypes.cpp:1885`).
    #[numeric_value(1876)]
    NewSwCast,
    /// `EF_ABR_MOTHER` (`EffectTypes.h:1910`, official id 1902).
    ///
    /// `RagEffect.cpp:22688-22695`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\new_abr_mother\\abr_mother.tga",
    /// 0)`. Lifetime 1902 (`EffectTypes.cpp:1911`).
    #[numeric_value(1902)]
    AbrMother,
    /// `EF_ABR_INFINITY_RING` (`EffectTypes.h:1918`, official id 1910).
    ///
    /// `RagEffect.cpp:22654-22663`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(...,
    /// "effect\\new_abr_infinity\\ring_infinty.tga", 0)` (the texture
    /// name's `infinty` typo is official). Lifetime 1910 (`EffectTypes.cpp:
    /// 1919`).
    #[numeric_value(1910)]
    AbrInfinityRing,
    /// `EF_NEW_D_CANNON_ENTITY` (`EffectTypes.h:1926`, official id 1918).
    ///
    /// `RagEffect.cpp:22672-22679`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\new_d_cannon\\ABR_DUAL_CANNON.tga",
    /// 0)`. Lifetime 1918 (`EffectTypes.cpp:1927`).
    #[numeric_value(1918)]
    NewDCannonEntity,
    /// `EF_FLAME_TRAP_RING` (`EffectTypes.h:1931`, official id 1923).
    ///
    /// `RagEffect.cpp:22432-22444`: positional `PlayWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_red.tga", 44)` calls.
    /// Lifetime 1923 (`EffectTypes.cpp:1932`).
    #[numeric_value(1923)]
    FlameTrapRing,
    /// `EF_NEW_PRAMEN_RING` (`EffectTypes.h:1940`, official id 1932).
    ///
    /// `RagEffect.cpp:22445-22456`: positional `PlayWave` plus
    /// `TexturedQuadRingSized(90, "effect\\new_pramen\\ring_blue.tga", 5)`.
    /// Lifetime 1932 (`EffectTypes.cpp:1941`).
    #[numeric_value(1932)]
    NewPramenRing,
    /// `EF_NEW_BATTLE_W_ENTITY` (`EffectTypes.h:1946`, official id 1938).
    ///
    /// `RagEffect.cpp:22664-22671`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(...,
    /// "effect\\new_battle_w\\ABR_BATTLE_WARIOR.tga", 0)`. Lifetime 1938
    /// (`EffectTypes.cpp:1947`).
    #[numeric_value(1938)]
    NewBattleWEntity,
    /// `EF_PNEUMATICUS_PROCELLA_RING` (`EffectTypes.h:1967`, official id 1959).
    ///
    /// `RagEffect.cpp:22680-22687`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(...,
    /// "effect\\pneumaticusprocella\\pneumaticusprocella_cast\\ring_yellow.tga"
    /// , 0)`. Lifetime 1959 (`EffectTypes.cpp:1968`).
    #[numeric_value(1959)]
    PneumaticusProcellaRing,
    /// `EF_SINCERE_FAITH_RING` (`EffectTypes.h:1971`, official id 1963).
    ///
    /// `RagEffect.cpp:22696-22703`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 45)` calls.
    /// Lifetime 1963 (`EffectTypes.cpp:1972`).
    #[numeric_value(1963)]
    SincereFaithRing,
    /// `EF_POWERFUL_FAITH_RING` (`EffectTypes.h:1994`, official id 1986).
    ///
    /// `RagEffect.cpp:22704-22711`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 46)` calls.
    /// Lifetime 1986 (`EffectTypes.cpp:1995`).
    #[numeric_value(1986)]
    PowerfulFaithRing,
    /// `EF_FIRM_FAITH_RING` (`EffectTypes.h:2006`, official id 1998).
    ///
    /// `RagEffect.cpp:22712-22719`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 47)` calls.
    /// Lifetime 1998 (`EffectTypes.cpp:2007`).
    #[numeric_value(1998)]
    FirmFaithRing,
    /// `EF_RESEARCH_REPORT_RING` (`EffectTypes.h:2248`, official id 2240).
    ///
    /// `RagEffect.cpp:22720-22727`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 48)` calls.
    /// Lifetime 2240 (`EffectTypes.cpp:2249`).
    #[numeric_value(2240)]
    ResearchReportRing,
    /// `EF_WOODEN_WARRIOR_RING` (`EffectTypes.h:2257`, official id 2249).
    ///
    /// `RagEffect.cpp:22728-22735`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 49)` calls.
    /// Lifetime 2249 (`EffectTypes.cpp:2258`).
    #[numeric_value(2249)]
    WoodenWarriorRing,
    /// `EF_WOODEN_FAIRY_RING` (`EffectTypes.h:2263`, official id 2255).
    ///
    /// `RagEffect.cpp:22736-22743`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_white.tga", 50)` calls.
    /// Lifetime 2255 (`EffectTypes.cpp:2264`).
    #[numeric_value(2255)]
    WoodenFairyRing,
    /// `EF_BEGINSPELL_RING_YELLOW` (`EffectTypes.h:2375`, official id 2367).
    ///
    /// `RagEffect.cpp:22818-22826`: `PlayEffectWave` plus two
    /// `TexturedQuadRingSized(..., "effect\\ring_yellow.tga", 0)` calls.
    /// Lifetime 40 (`EffectTypes.cpp:2376`).
    #[numeric_value(2367)]
    BeginspellRingYellow,
    /// `EF_VIOLEN_HIT` (`EffectTypes.h:2428`, official id 2420).
    ///
    /// The official entity is a `CSpriteEffect` (`EffectLists.cpp:5129`) and
    /// the body is its sprite setup (`SpriteEffect.cpp:294-301`): one looping
    /// `몬스터\violen_hit.spr` animation, `frameDelay` 4, `depthOffset` 0.001,
    /// `effectLifetime` 100 (`SpriteEffect.cpp:71-73`). The client selects it
    /// as the defender hit effect of a Violen/`JT_CP_VIOLEN` attacker
    /// (`RenderObject.cpp:3818-3821`), which is why the missing variant used
    /// to suppress that hit effect.
    #[numeric_value(2420)]
    ViolenHit,
    /// `EF_HEAL_BLUE_2` (`EffectTypes.h:2435`, official id 2427).
    ///
    /// The dispatch (`RagEffect.cpp:22886-22891`) is one already-ported
    /// `HealRingSparkle("effect\\ring_blue.tga", 2)` call; the port already
    /// registers the kind-0 form for `EF_HEAL2` (36). Lifetime 100
    /// (`EffectTypes.cpp:2423`).
    #[numeric_value(2427)]
    HealBlue2,
    /// `EF_RG_COIN_BLUE` (`EffectTypes.h:2444`, official id 2436).
    ///
    /// The body (`RagEffect.cpp:22916-22940`) is already implemented as
    /// `RogueCoin::coin_blue`; spray kind 8, waves on frames 1/11/21/31.
    /// Lifetime 3000 (`EffectTypes.cpp:2428`).
    #[numeric_value(2436)]
    RgCoinBlue,
    /// `EF_RG_COIN_BLUE_2` (`EffectTypes.h:2445`, official id 2437).
    ///
    /// The body (`RagEffect.cpp:22941-22965`) is already implemented as
    /// `RogueCoin::coin_blue_2`; spray kind 9, same wave frames. Lifetime 3000
    /// (`EffectTypes.cpp:2429`).
    #[numeric_value(2437)]
    RgCoinBlue2,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01F3)]
#[packet_size(10)]
pub struct DisplaySpecialEffectPacket {
    pub entity_id: EntityId,
    /// Raw effect id; kept as `u32` so weather effect ids that have no
    /// `EffectId` enum variant are not rejected by the decoder.
    pub effect_id: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x043D)]
#[packet_size(8)]
pub struct DisplaySkillCooldownPacket {
    pub skill_id: SkillId,
    pub until: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01DE)]
#[packet_size(33)]
pub struct DisplaySkillEffectAndDamagePacket {
    pub skill_id: SkillId,
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub start_time: ClientTick,
    pub soruce_delay: u32,
    pub destination_delay: u32,
    pub damage: u32,
    pub level: SkillLevel,
    /// Number of hits the damage is split into (`div` in rAthena's
    /// `clif_skill_damage`). Multi-hit skills such as Throw Spirit Sphere send
    /// one packet with `div = hit count`; the official client divides `damage`
    /// by this value and shows one number per hit
    /// (`RenderObject.cpp:5367-5368`, `:5931-5947`).
    pub div: u16,
    pub skill_type: u8,
}

/// `ZC_BLADESTOP` (0x01D1): the Blade Stop caster (`first`) has rooted the
/// attacker (`second`) until the lock is released.
///
/// rAthena `clif_bladestop` (`clif.cpp:8556-8569`) sends
/// `srcId.L targetId.L flag.L`; the official client handles it in
/// `CGameMode::ReceiveActorPairFreeze` (`GameModeNotices.cpp:922-955`):
/// the first actor holds skill block frame 4 tinted grey and the second is
/// motion-frozen until a `flag == 0` packet arrives.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x01D1)]
#[packet_size(14)]
pub struct BladeStopPacket {
    /// Blade Stop caster (`CGameMode::ReceiveActorPairFreeze` `firstAid`).
    pub first_entity_id: EntityId,
    /// Attacker that is rooted (`secondAid`).
    pub second_entity_id: EntityId,
    /// `1` while the lock is active, `0` on release.
    pub active: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0A27)]
#[packet_size(8)]
pub struct DisplayPlayerHealEffect {
    pub heal_type: HealType,
    pub heal_amount: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09CB)]
#[packet_size(17)]
pub struct DisplaySkillEffectNoDamagePacket {
    pub skill_id: SkillId,
    pub heal_amount: u32,
    pub destination_entity_id: EntityId,
    pub source_entity_id: EntityId,
    pub result: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u32)]
pub enum VisualEffect {
    BaseLevelUp,
    JobLevelUp,
    RefineFailure,
    RefineSuccess,
    GameOver,
    PharmacySuccess,
    PharmacyFailure,
    BaseLevelUpSuperNovice,
    JobLevelUpSuperNovice,
    BaseLevelUpTaekwon,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x019B)]
#[packet_size(10)]
pub struct VisualEffectPacket {
    pub entity_id: EntityId,
    pub effect: VisualEffect,
}

// TODO: improve names
#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[numeric_type(u16)]
pub enum QuestEffect {
    Quest,
    Quest2,
    Job,
    Job2,
    Event,
    Event2,
    ClickMe,
    DailyQuest,
    Event3,
    JobQuest,
    JumpingPoring,
    #[numeric_value(9999)]
    None,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0446)]
#[packet_size(14)]
pub struct QuestEffectPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub effect: QuestEffect,
    pub color: QuestColor,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0438)]
pub struct UseSkillAtIdPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_id: EntityId,
}

/// 2019-era target skill packet (`CZ_USE_SKILL_TOID`).
///
/// rAthena parses targeted skill use from 0x089B for `PACKETVER >= 20130320`
/// (our 20190605 server): 10 bytes. rAthena's own layout comment is
/// `<skill lv>.W <skill id>.W <target id>.L`, so offsets are 2 = **level**
/// (u16), 4 = **skill id** (u16), 6 = target id (u32). We previously sent the
/// two in the opposite order, which the server read as skill=level/level=id and
/// silently ignored.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x089B)]
pub struct UseSkillAtTargetPacket2019 {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0AF4)]
pub struct UseSkillOnGroundPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_position: TilePosition,
    #[new_default]
    pub unused: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B10)]
pub struct StartUseSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B11)]
pub struct EndUseSkillPacket {
    pub skill_id: SkillId,
}

/// 2019-era (`PACKETVER` >= 20181212) layout of `ZC_USESKILL_ACK`.
///
/// The 2019 server sends this header instead of the [UseSkillSuccessPacket]
/// header (0x07FB) that newer clients use; compared to that packet it has an
/// additional trailing `attackMT` field (29 bytes in total). It is parsed and
/// turned into the typed `NetworkEvent::SkillCast` by the client.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0B1A)]
#[packet_size(29)]
pub struct UseSkillAckPacket2019 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub position: TilePosition,
    pub skill_id: SkillId,
    pub element: u32,
    pub delay_time: u32,
    pub disposable: u8,
    pub attack_motion_time: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0110)]
#[packet_size(14)]
pub struct ToUseSkillSuccessPacket {
    pub skill_id: SkillId,
    pub btype: i32,
    pub item_id: ItemId,
    pub flag: u8,
    pub cause: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x09CA)]
#[packet_size(23)]
pub struct NotifySkillUnitPacket {
    pub lenght: u16,
    pub entity_id: EntityId,
    pub creator_id: EntityId,
    pub position: TilePosition,
    pub unit_id: UnitId,
    pub range: u8,
    pub visible: u8,
    pub skill_level: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0117)]
#[packet_size(18)]
pub struct NotifyGroundSkillPacket {
    pub skill_id: SkillId,
    pub entity_id: EntityId,
    pub level: SkillLevel,
    pub position: TilePosition,
    pub start_time: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0120)]
#[packet_size(6)]
pub struct SkillUnitDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0112)]
pub struct LevelUpSkillPacket {
    pub skill_id: SkillId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x010E)]
#[packet_size(11)]
pub struct UpdateSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: AttackRange,
    // TODO: bool,
    pub upgradable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, ro_next_interface::element::StateElement))]
#[header(0x0441)]
#[packet_size(4)]
pub struct RemoveSkillPacket {
    pub skill_id: SkillId,
}
