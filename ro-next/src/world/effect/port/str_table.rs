//! Official `.str` resource table and clip reader - the asset half of
//! `CRagEffect::Init`.
//!
//! `RagEffectInitialization.cpp:16-711` switches on the effect id and builds
//! one `.str` file name; `RagEffectInitialization.cpp:699` then loads it with
//! `ResourceMgr::Load(name)`. The `str` extension is registered with the
//! `texture\effect\` prefix (`EZeffectResource.cpp:152`), so every name below
//! is relative to `data\texture\effect\`.
//!
//! `loopLimit` is `0` unless the switch overrides it
//! (`RagEffect.cpp:109`); none of the effects that call `ProcessEZ2STR` here
//! overrides it, so their resource plays once.
//!
//! `CEZeffectRes::Load` (`EZeffectResource.cpp:49-91`) reads exactly the
//! `EffectData` layout the client's `EffectLoader` parses
//! (`loaders/effect/mod.rs:48-61`), so [`load_clips`] reuses that parser and
//! only converts the raw key frames into the port's [`Clip`]; the transform
//! fields are the `EZeffectTransform` members (`EZeffectResource.h:13-43`).
//!
//! Besides the key frames, `CEZeffectClip` carries `textureCount` names
//! (`names[110]`, `EZeffectResource.h:45-52`). The port threads them into
//! [`ClipSet::textures`] exactly like the client loader turns
//! `LayerData::texture_names` into `effect\{prefix}{name}` (and, as in
//! `EZeffectClip::Texture`, appends `.bmp` when the name has no extension).
//! The fixed 128-byte name fields are already stripped at their first NUL by
//! the `String` parser both loaders use, so no trailing padding reaches the
//! texture list.

use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_formats::effect::{EffectData, Frame as EffectFrame, LayerData};
use ragnarok_formats::version::GenericFormatMetadata;
use ragnarok_packets::EffectId;
use ro_next_loaders::FileLoader;

use super::str_player::{Clip, KeyFrame, Transform};
use crate::loaders::{GameFileLoader, strip_effect_magic_prefix};

/// `CEZeffectRes`'s registered prefix (`EZeffectResource.cpp:152`), relative to
/// the archive root.
pub const STR_DIRECTORY: &str = "data\\texture\\effect\\";

/// One effect's official `.str` resource.
#[derive(Debug, Clone, Copy)]
pub struct StrResource {
    /// `(effect, name)` rows of `RagEffectInitialization.cpp`'s switch. The
    /// original picks one name; the slice allows a fallback order and keeps a
    /// single-entry list for every effect ported today.
    pub files: &'static [&'static str],
    /// `RagEffectState::loopLimit` (`RagEffect.cpp:109`, default 0 = one pass).
    pub loop_limit: i32,
}

static SIGNUM: StrResource = StrResource {
    files: &["Cross.str"],
    loop_limit: 0,
};

static CART_REVOLUTION: StrResource = StrResource {
    files: &["CartRevolution.str"],
    loop_limit: 0,
};

static STORM_GUST: StrResource = StrResource {
    files: &["StormGust.str"],
    loop_limit: 0,
};

static GLASS_WALL: StrResource = StrResource {
    files: &["SafetyWall.str"],
    loop_limit: 0,
};

static ASPERSIO: StrResource = StrResource {
    files: &["Aspersio.str"],
    loop_limit: 0,
};

static POTION_BERSERK: StrResource = StrResource {
    files: &["버서크.str"],
    loop_limit: 0,
};

// The `.str`-only tier (batch "e"): every one of these effects is the clip and
// at most a scheduled wave, with no primitive of its own.

static STONE_CURSE: StrResource = StrResource {
    files: &["StoneCurse.str"],
    loop_limit: 0,
};

static ANGELUS: StrResource = StrResource {
    files: &["Angelus.str"],
    loop_limit: 0,
};

static CURE: StrResource = StrResource {
    files: &["Cure.str"],
    loop_limit: 0,
};

static MAGNIFICAT: StrResource = StrResource {
    files: &["Magnificat.str"],
    loop_limit: 0,
};

static RESURRECTION: StrResource = StrResource {
    files: &["Resurrection.str"],
    loop_limit: 0,
};

static RECOVERY: StrResource = StrResource {
    files: &["Recovery.str"],
    loop_limit: 0,
};

static LEX_DIVINA: StrResource = StrResource {
    files: &["LexDivina.str"],
    loop_limit: 0,
};

static CONCENTRATION: StrResource = StrResource {
    files: &["Concentration.str"],
    loop_limit: 0,
};

static TWO_HAND: StrResource = StrResource {
    files: &["TwoHand.str"],
    loop_limit: 0,
};

static MAGIC_ROD: StrResource = StrResource {
    files: &["매직로드.str"],
    loop_limit: 0,
};

// `.str`-only tier follow-up (batch "e"). One row per registered id; every row
// is the switch's regular (non-`*_min`) name and none overrides `loopLimit`.

static LIGHTNING: StrResource = StrResource {
    files: &["Lightning.str"],
    loop_limit: 0,
};

static PROVOKE: StrResource = StrResource {
    files: &["Provoke.str"],
    loop_limit: 0,
};

static BRANDISH: StrResource = StrResource {
    files: &["Brandish.str"],
    loop_limit: 0,
};

static IMPOSITIO: StrResource = StrResource {
    files: &["Impositio.str"],
    loop_limit: 0,
};

static LEX_AETERNA: StrResource = StrResource {
    files: &["LexAeterna.str"],
    loop_limit: 0,
};

static SUFFRAGIUM: StrResource = StrResource {
    files: &["Suffragium.str"],
    loop_limit: 0,
};

static REPAIR_WEAPON: StrResource = StrResource {
    files: &["RepairWeapon.str"],
    loop_limit: 0,
};

static WEAPON_PERFECTION: StrResource = StrResource {
    files: &["WeaponPerfection.str"],
    loop_limit: 0,
};

static MAXIMIZE_POWER: StrResource = StrResource {
    files: &["MaximizePower.str"],
    loop_limit: 0,
};

static SPRING: StrResource = StrResource {
    files: &["Spring.str"],
    loop_limit: 0,
};

static KYRIE: StrResource = StrResource {
    files: &["Kyrie.str"],
    loop_limit: 0,
};

static POISON_REACT: StrResource = StrResource {
    files: &["PoisonReact_1st.str"],
    loop_limit: 0,
};

static SLOW_POISON: StrResource = StrResource {
    files: &["slowp.str"],
    loop_limit: 0,
};

static SONIC_BLOW: StrResource = StrResource {
    files: &["SonicBlow.str"],
    loop_limit: 0,
};

static PIERCE: StrResource = StrResource {
    files: &["Pierce.str"],
    loop_limit: 0,
};

static BOWLING: StrResource = StrResource {
    files: &["Bowling.str"],
    loop_limit: 0,
};

static SPEAR_STAB: StrResource = StrResource {
    files: &["SpearStab.str"],
    loop_limit: 0,
};

static SPEAR_BOOMERANG: StrResource = StrResource {
    files: &["SpearBoomerang.str"],
    loop_limit: 0,
};

static ENERGY_COAT: StrResource = StrResource {
    files: &["EnergyCoat.str"],
    loop_limit: 0,
};

static SPELL_BREAKER: StrResource = StrResource {
    files: &["spell.str"],
    loop_limit: 0,
};

static DISPELL: StrResource = StrResource {
    files: &["디스펠.str"],
    loop_limit: 0,
};

static STEAL_COIN: StrResource = StrResource {
    files: &["steal_coin.str"],
    loop_limit: 0,
};

static STRIP_WEAPON: StrResource = StrResource {
    files: &["strip_weapon.str"],
    loop_limit: 0,
};

static STRIP_SHIELD: StrResource = StrResource {
    files: &["strip_shield.str"],
    loop_limit: 0,
};

static STRIP_ARMOR: StrResource = StrResource {
    files: &["strip_armor.str"],
    loop_limit: 0,
};

static STRIP_HELM: StrResource = StrResource {
    files: &["strip_helm.str"],
    loop_limit: 0,
};

static MELTDOWN: StrResource = StrResource {
    files: &["melt.str"],
    loop_limit: 0,
};

static CART_BOOST: StrResource = StrResource {
    files: &["cart.str"],
    loop_limit: 0,
};

static SOUL_BURN: StrResource = StrResource {
    files: &["소울번.str"],
    loop_limit: 0,
};

static SOUL_CHANGE: StrResource = StrResource {
    files: &["사람효과.str"],
    loop_limit: 0,
};

static ASSUMPTIO2: StrResource = StrResource {
    files: &["asum.str"],
    loop_limit: 0,
};

static FIRE_DRAGON: StrResource = StrResource {
    files: &["fire dragon.str"],
    loop_limit: 0,
};

static ICY: StrResource = StrResource {
    files: &["icy.str"],
    loop_limit: 0,
};

static BULLSEYE: StrResource = StrResource {
    files: &["불스아이.str"],
    loop_limit: 0,
};

// Positional-sound tier (`rag/positional.rs`). One row per registered id; the
// bodies differ only in the `PlayWave` frame/offset. `Gloria` takes the
// switch's regular name (the `gloria_min.str` variant is the other side of the
// `TT_MIN_EFFECT_ON_OFF` ternary); every row keeps the single-pass default.

static GLORIA: StrResource = StrResource {
    files: &["Gloria.str"],
    loop_limit: 0,
};

static HOLY_CROSS: StrResource = StrResource {
    files: &["holy_cross.str"],
    loop_limit: 0,
};

static ENC_FIRE: StrResource = StrResource {
    files: &["enc_fire.str"],
    loop_limit: 0,
};

static ENC_ICE: StrResource = StrResource {
    files: &["enc_ice.str"],
    loop_limit: 0,
};

static ENC_WIND: StrResource = StrResource {
    files: &["enc_wind.str"],
    loop_limit: 0,
};

static ENC_EARTH: StrResource = StrResource {
    files: &["enc_earth.str"],
    loop_limit: 0,
};

static LOUD: StrResource = StrResource {
    files: &["loud.str"],
    loop_limit: 0,
};

// Per-id `ProcessEZ2STR` offset tier (`rag/shield_charge.rs`,
// `rag/devotion.rs`, `rag/lkconcentration.rs`, `rag/reject_sword.rs`,
// `rag/thunderstorm.rs`). The offset is runtime state, not a resource
// property, so every row keeps the single-pass default.

static SHIELD_CHARGE: StrResource = StrResource {
    files: &["shield_charge.str"],
    loop_limit: 0,
};

static DEVOTION: StrResource = StrResource {
    files: &["devotion.str"],
    loop_limit: 0,
};

static REJECT_SWORD: StrResource = StrResource {
    files: &["sword.str"],
    loop_limit: 0,
};

static THUNDERSTORM: StrResource = StrResource {
    files: &["ThunderStorm.str"],
    loop_limit: 0,
};

// Champion `.str` bodies (`rag/champion.rs`). The five combo-attack clips are
// one resource per id; `Init` has no `loopLimit` override for any of them, so
// each keeps the constructor default 0 (`RagEffect.cpp:109`) and the effect
// retires with the clip.

static COMBO_ATTACK_1: StrResource = StrResource {
    files: &["yunta_1.str"],
    loop_limit: 0,
};

static COMBO_ATTACK_2: StrResource = StrResource {
    files: &["yunta_2.str"],
    loop_limit: 0,
};

static COMBO_ATTACK_3: StrResource = StrResource {
    files: &["yunta_3.str"],
    loop_limit: 0,
};

static COMBO_ATTACK_4: StrResource = StrResource {
    files: &["yunta_4.str"],
    loop_limit: 0,
};

static COMBO_ATTACK_5: StrResource = StrResource {
    files: &["yunta_5.str"],
    loop_limit: 0,
};

static CHAIN_COMBO: StrResource = StrResource {
    files: &["연환.str"],
    loop_limit: 0,
};

/// The official `.str` resource of an effect, if its body drives
/// `ProcessEZ2STR`.
///
/// Rows cite `RagEffectInitialization.cpp`'s switch. `Signum` and
/// `CartRevolution` were the two ids excluded from `registry::ported_ids()`
/// until this table existed; the other four belong to bodies that were already
/// registered (`StormGust`, `GlassWall`, `Aspersio`, `PotionPill`) and whose
/// `.str` layer was never opened before.
pub fn str_resource(effect: EffectId) -> Option<&'static StrResource> {
    Some(match effect {
        // `case EF_SIGNUM: std::sprintf(name, "Cross.str");` - 45-46.
        EffectId::Signum => &SIGNUM,
        // `case EF_CARTREVOLUTION: std::sprintf(name, "CartRevolution.str");`
        // - 247-248.
        EffectId::Cartrevolution => &CART_REVOLUTION,
        // `case EF_STORMGUST: std::sprintf(name, "StormGust.str");` - 96-97.
        EffectId::Stormgust => &STORM_GUST,
        // `case EF_GLASSWALL: case EF_GLASSWALL2:
        //  std::sprintf(name, "SafetyWall.str");` - 234-236.
        EffectId::Glasswall => &GLASS_WALL,
        // `case EF_ASPERSIO: std::sprintf(name, "Aspersio.str");` - 87-88.
        EffectId::Aspersio => &ASPERSIO,
        // `case EF_POTION_BERSERK: case EF_POTION_BERSERK2:
        //  std::sprintf(name, "버서크.str");` - 496-498.
        EffectId::PotionBerserk | EffectId::PotionBerserk2 => &POTION_BERSERK,

        // `.str`-only tier (batch "e").
        //
        // `case EF_STONECURSE: case EF_PETRIFYATTACK:
        //  std::sprintf(name, "StoneCurse.str");` - 292-295.
        EffectId::Stonecurse => &STONE_CURSE,
        // `case EF_ANGELUS: case EF_WEWISH:` (the `jong_mini.str` branch of the
        // `TT_MIN_EFFECT_ON_OFF` toggle is the other side of the ternary, like
        // the rest of this table the regular name is used) - 481-486.
        EffectId::Angelus => &ANGELUS,
        // `case EF_CURE:` (`cure_min.str` toggle) - 54-56.
        EffectId::Cure => &CURE,
        // `case EF_MAGNIFICAT:` (`magnificat_min.str` toggle) - 69-71.
        EffectId::Magnificat => &MAGNIFICAT,
        // `case EF_RESURRECTION:` (`resurrection_min.str` toggle) - 72-74.
        EffectId::Resurrection => &RESURRECTION,
        // `case EF_RECOVERY: std::sprintf(name, "Recovery.str");` - 75-77.
        EffectId::Recovery => &RECOVERY,
        // `case EF_LEXDIVINA: std::sprintf(name, "LexDivina.str");` - 90-92.
        EffectId::Lexdivina => &LEX_DIVINA,
        // `case EF_CONCENTRATION: case EF_CONCENTRATION2:
        //  std::sprintf(name, "Concentration.str");` - 492-495.
        EffectId::Concentration => &CONCENTRATION,
        // `case EF_TWOHANDQUICKEN: case EF_SPEARQUICKEN:
        //  case EF_LKCONCENTRATION: std::sprintf(name, "TwoHand.str");`
        // - 439-443. `EF_LKCONCENTRATION` has its own dispatch case at
        // `RagEffect.cpp:10377`, but it opens the same resource.
        EffectId::Twohandquicken | EffectId::Spearquicken | EffectId::Lkconcentration => &TWO_HAND,
        // `case EF_MAGICROD: std::sprintf(name, "매직로드.str");` - 341-343.
        EffectId::Magicrod => &MAGIC_ROD,

        // `.str`-only tier follow-up (batch "e").
        //
        // `case EF_LIGHTBOLT: std::sprintf(name, "Lightning.str");` - 42-43.
        EffectId::Lightbolt => &LIGHTNING,
        // `case EF_PROVOKE: std::sprintf(name, "Provoke.str");` - 57-58.
        EffectId::Provoke => &PROVOKE,
        // `case EF_BRANDISHSPEAR: std::sprintf(name, "Brandish.str");` - 186-187.
        EffectId::Brandishspear => &BRANDISH,
        // `case EF_IMPOSITIO: std::sprintf(name, "Impositio.str");` - 81-82.
        EffectId::Impositio => &IMPOSITIO,
        // `case EF_LEXAETERNA:` (`lexaeterna_min.str` toggle) - 84-86.
        EffectId::Lexaeterna => &LEX_AETERNA,
        // `case EF_SUFFRAGIUM:` (`suffragium_min.str` toggle) - 93-95.
        EffectId::Suffragium => &SUFFRAGIUM,
        // `case EF_REPAIRWEAPON: std::sprintf(name, "RepairWeapon.str");`
        // - 117-118.
        EffectId::Repairweapon => &REPAIR_WEAPON,
        // `case EF_PERFECTION:` (`WeaponPerfection_min.str` toggle) - 123-125.
        EffectId::Perfection => &WEAPON_PERFECTION,
        // `case EF_MAXPOWER:` (`maximize_min.str` toggle) - 126-128.
        EffectId::Maxpower => &MAXIMIZE_POWER,
        // `case EF_SPRINGTRAP: std::sprintf(name, "Spring.str");` - 138-139.
        EffectId::Springtrap => &SPRING,
        // `case EF_KYRIE:` (`kyrie_min.str` toggle) - 147-149.
        EffectId::Kyrie => &KYRIE,
        // `case EF_POISONREACT:
        //  std::sprintf(name, "PoisonReact_1st.str");` - 156-157.
        EffectId::Poisonreact => &POISON_REACT,
        // `case EF_SLOWPOISON: std::sprintf(name, "slowp.str");` - 389-390.
        EffectId::Slowpoison => &SLOW_POISON,
        // `case EF_SONICBLOW2: std::sprintf(name, "SonicBlow.str");` - 183-184.
        EffectId::Sonicblow2 => &SONIC_BLOW,
        // `case EF_PIERCESELF: std::sprintf(name, "Pierce.str");` - 201-202.
        EffectId::Pierceself => &PIERCE,
        // `case EF_BOWLINGSELF: std::sprintf(name, "Bowling.str");` - 204-205.
        EffectId::Bowlingself => &BOWLING,
        // `case EF_SPEARSTABSELF: std::sprintf(name, "SpearStab.str");`
        // - 207-208.
        EffectId::Spearstabself => &SPEAR_STAB,
        // `case EF_SPEARBMRSELF:
        //  std::sprintf(name, "SpearBoomerang.str");` - 210-211.
        EffectId::Spearbmrself => &SPEAR_BOOMERANG,
        // `case EF_ENERGYCOAT: std::sprintf(name, "EnergyCoat.str");` - 244-245.
        EffectId::Energycoat => &ENERGY_COAT,
        // `case EF_SPELLBREAKER: std::sprintf(name, "spell.str");` - 335-336.
        EffectId::Spellbreaker => &SPELL_BREAKER,
        // `case EF_DISPELL: std::sprintf(name, "디스펠.str");` - 338-339.
        EffectId::Dispell => &DISPELL,
        // `case EF_STEALCOIN: std::sprintf(name, "steal_coin.str");` - 350-351.
        EffectId::Stealcoin => &STEAL_COIN,
        // `case EF_STRIPWEAPON: std::sprintf(name, "strip_weapon.str");`
        // - 353-354.
        EffectId::Stripweapon => &STRIP_WEAPON,
        // `case EF_STRIPSHIELD: std::sprintf(name, "strip_shield.str");`
        // - 356-357.
        EffectId::Stripshield => &STRIP_SHIELD,
        // `case EF_STRIPARMOR: std::sprintf(name, "strip_armor.str");`
        // - 359-360.
        EffectId::Striparmor => &STRIP_ARMOR,
        // `case EF_STRIPHELM: std::sprintf(name, "strip_helm.str");` - 362-363.
        EffectId::Striphelm => &STRIP_HELM,
        // `case EF_MELTDOWN: std::sprintf(name, "melt.str");` - 421-422.
        EffectId::Meltdown => &MELTDOWN,
        // `case EF_CARTBOOST: std::sprintf(name, "cart.str");` - 427-428.
        EffectId::Cartboost => &CART_BOOST,
        // `case EF_SOULBURN: std::sprintf(name, "소울번.str");` - 433-434.
        EffectId::Soulburn => &SOUL_BURN,
        // `case EF_SOULCHANGE: std::sprintf(name, "사람효과.str");` - 436-437.
        EffectId::Soulchange => &SOUL_CHANGE,
        // `case EF_ASSUMPTIO2: std::sprintf(name, "asum.str");` - 418-419.
        EffectId::Assumptio2 => &ASSUMPTIO2,
        // `case EF_BAKU: std::sprintf(name, "fire dragon.str");` - 478-479.
        EffectId::Baku => &FIRE_DRAGON,
        // `case EF_HYOUSYOURAKU: std::sprintf(name, "icy.str");` - 506-507.
        EffectId::Hyousyouraku => &ICY,
        // `case EF_BULLSEYE: std::sprintf(name, "불스아이.str");` - 503-504.
        EffectId::Bullseye => &BULLSEYE,
        // Positional-sound tier (`rag/positional.rs`).
        //
        // `case EF_GLORIA:` `gloria_min.str` toggle - 66-68.
        EffectId::Gloria => &GLORIA,
        // `case EF_HOLYCROSS: std::sprintf(name, "holy_cross.str");` - 344-345.
        EffectId::Holycross => &HOLY_CROSS,
        // `case EF_FLAMELAUNCHER: std::sprintf(name, "enc_fire.str");`
        // - 374-375.
        EffectId::Flamelauncher => &ENC_FIRE,
        // `case EF_FROSTWEAPON: std::sprintf(name, "enc_ice.str");` - 377-378.
        EffectId::Frostweapon => &ENC_ICE,
        // `case EF_LIGHTNINGLOADER: std::sprintf(name, "enc_wind.str");`
        // - 380-381.
        EffectId::Lightningloader => &ENC_WIND,
        // `case EF_SEISMICWEAPON: std::sprintf(name, "enc_earth.str");`
        // - 383-384.
        EffectId::Seismicweapon => &ENC_EARTH,
        // `case EF_LOUD: std::sprintf(name, "loud.str");` - 386-387.
        EffectId::Loud => &LOUD,
        // Per-id `ProcessEZ2STR` offset tier. The offsets differ
        // (`shield_charge` 0, `devotion` -20, the other three -80) but the
        // resource row is only the file name.
        //
        // `case EF_SHIELDCHARGE: std::sprintf(name, "shield_charge.str");`
        // - 365-366.
        EffectId::Shieldcharge => &SHIELD_CHARGE,
        // `case EF_DEVOTION: std::sprintf(name, "devotion.str");` - 368-369.
        EffectId::Devotion => &DEVOTION,
        // `case EF_REJECTSWORD: std::sprintf(name, "sword.str");` - 430-431.
        EffectId::Rejectsword => &REJECT_SWORD,
        // `case EF_THUNDERSTORM: std::sprintf(name, "ThunderStorm.str");`
        // - 39-40.
        EffectId::Thunderstorm => &THUNDERSTORM,
        // Champion `.str` bodies (`rag/champion.rs`).
        //
        // `case EF_COMBOATTACK1: std::sprintf(name, "yunta_1.str");` - 262-264.
        EffectId::Comboattack1 => &COMBO_ATTACK_1,
        // `case EF_COMBOATTACK2: std::sprintf(name, "yunta_2.str");` - 265-267.
        EffectId::Comboattack2 => &COMBO_ATTACK_2,
        // `case EF_COMBOATTACK3: std::sprintf(name, "yunta_3.str");` - 268-270.
        EffectId::Comboattack3 => &COMBO_ATTACK_3,
        // `case EF_COMBOATTACK4: std::sprintf(name, "yunta_4.str");` - 271-273.
        EffectId::Comboattack4 => &COMBO_ATTACK_4,
        // `case EF_COMBOATTACK5: std::sprintf(name, "yunta_5.str");` - 274-276.
        EffectId::Comboattack5 => &COMBO_ATTACK_5,
        // `case EF_CHAINCOMBO: std::sprintf(name, "연환.str");` - 347-349.
        EffectId::Chaincombo => &CHAIN_COMBO,
        _ => return None,
    })
}

/// Shorthand for [`StrResource::files`].
pub fn str_clips_for(effect: EffectId) -> Option<&'static [&'static str]> {
    str_resource(effect).map(|resource| resource.files)
}

/// `RagEffectState::loopLimit` the player must use for `effect` (`0` when the
/// effect has no `.str` resource).
pub fn str_loop_limit(effect: EffectId) -> i32 {
    str_resource(effect).map_or(0, |resource| resource.loop_limit)
}

/// One `.str` resource as the asset layer hands it to the holder.
///
/// `clips` is the `ProcessEZ2STR` playback data ([`Clip`]); `textures` is the
/// per-clip texture list of `EZeffectClip::names`, resolved to the same
/// `effect\...` paths the client's `EffectLoader` builds. The two vectors are
/// parallel: `textures[layer][frame]` is the texture `layer`'s current
/// `aniframe` selects.
#[derive(Debug, Clone, Default)]
pub struct ClipSet {
    pub clips: Vec<Clip>,
    pub textures: Vec<Vec<String>>,
}

/// Loads the first loadable file of `resource` as a set of clips.
///
/// This is the `.str` read of `CEZeffectRes::Load` on top of the same
/// `EffectData` parser the client's `EffectLoader` uses. `None` means no file
/// could be resolved/parsed, in which case the body plays without its `.str`
/// layer instead of aborting the effect.
pub fn load_clips(game_file_loader: &GameFileLoader, resource: &StrResource) -> Option<ClipSet> {
    resource.files.iter().find_map(|file| load_clip_file(game_file_loader, file))
}

/// Reads and parses one `.str` file (relative to [`STR_DIRECTORY`]).
pub fn load_clip_file(game_file_loader: &GameFileLoader, file: &str) -> Option<ClipSet> {
    let path = format!("{STR_DIRECTORY}{file}");
    let mut bytes = game_file_loader.get(&path).ok()?;
    strip_effect_magic_prefix(&mut bytes);

    let mut byte_reader = ByteReader::with_default_metadata::<GenericFormatMetadata>(&bytes);
    let effect_data = EffectData::from_bytes(&mut byte_reader).ok()?;

    Some(ClipSet {
        clips: clips_from_layers(&effect_data.layers),
        textures: textures_from_layers(&effect_data.layers, texture_prefix(file)),
    })
}

/// The directory part of a `.str` path, with its trailing separator.
///
/// This is the client loader's rule (`loaders/effect/mod.rs:63-66`): the
/// texture name is relative to the `.str` file's own directory, which is
/// prepended as `effect\{prefix}{name}`. Implemented with `rfind` so a
/// multi-byte file name (the official resources include Korean names) cannot
/// hit a non-char boundary.
pub(crate) fn texture_prefix(path: &str) -> &str {
    match path.rfind('\\') {
        Some(index) => &path[..=index],
        None => "",
    }
}

/// `EZeffectClip::names` -> texture paths.
///
/// `loaders/effect/mod.rs:80-85` builds `effect\{prefix}{name}` for every
/// name, and `EZeffectClip::Texture` (`EZeffectResource.cpp:107-124`) appends
/// `.bmp` when the path has no extension. Both rules are applied here so the
/// port's renderer can hand the result straight to `TextureLoader`.
pub(crate) fn textures_from_layers(layers: &[LayerData], prefix: &str) -> Vec<Vec<String>> {
    layers
        .iter()
        .map(|layer| {
            layer
                .texture_names
                .iter()
                .map(|name| {
                    let mut path = format!("effect\\{prefix}{}", name.name);

                    // `path.find_last_of('.') == std::string::npos`.
                    if !path.contains('.') {
                        path.push_str(".bmp");
                    }

                    path
                })
                .collect()
        })
        .collect()
}

/// Converts the parsed layers of a `.str` file into the port's clips.
///
/// One `EZeffectClip` is one `Clip`; `textureCount` and every key frame are
/// carried over verbatim (`EZeffectResource.h:39-57`). The raw key-frame
/// `type` (offset `0x04`) is the absolute/delta selector `ProcessEZ2STR` reads
/// as `key->type` (`RagEffect.cpp:1443`), stored in
/// [`Transform::kind`] by the port.
pub(crate) fn clips_from_layers(layers: &[LayerData]) -> Vec<Clip> {
    layers.iter().map(clip_from_layer).collect()
}

fn clip_from_layer(layer: &LayerData) -> Clip {
    Clip {
        texture_count: layer.texture_count.max(0) as u32,
        frames: layer.frames.iter().map(key_frame).collect(),
    }
}

fn key_frame(frame: &EffectFrame) -> KeyFrame {
    KeyFrame {
        frame: frame.frame_index,
        transform: Transform {
            x: frame.offset.x,
            y: frame.offset.y,
            u: frame.uv[0],
            v: frame.uv[1],
            us: frame.uv[2],
            vs: frame.uv[3],
            u2: frame.uv[4],
            v2: frame.uv[5],
            us2: frame.uv[6],
            vs2: frame.uv[7],
            ax: [frame.xy[0], frame.xy[1], frame.xy[2], frame.xy[3]],
            ay: [frame.xy[4], frame.xy[5], frame.xy[6], frame.xy[7]],
            cr_r: frame.color[0],
            cr_g: frame.color[1],
            cr_b: frame.color[2],
            cr_a: frame.color[3],
            rz: frame.angle,
            kind: frame.frame_type,
            anitype: frame.animation_type,
            anidelta: frame.delay,
            aniframe: frame.texture_index,
        },
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Vector2;

    use super::*;

    #[test]
    fn official_str_files_match_the_initialization_switch() {
        assert_eq!(str_clips_for(EffectId::Signum), Some(&["Cross.str"][..]));
        assert_eq!(str_clips_for(EffectId::Cartrevolution), Some(&["CartRevolution.str"][..]));
        assert_eq!(str_clips_for(EffectId::Stormgust), Some(&["StormGust.str"][..]));
        assert_eq!(str_clips_for(EffectId::Glasswall), Some(&["SafetyWall.str"][..]));
        assert_eq!(str_clips_for(EffectId::Aspersio), Some(&["Aspersio.str"][..]));
        assert_eq!(str_clips_for(EffectId::PotionBerserk), Some(&["버서크.str"][..]));
        assert_eq!(str_clips_for(EffectId::PotionBerserk2), Some(&["버서크.str"][..]));
        assert_eq!(str_clips_for(EffectId::Shieldcharge), Some(&["shield_charge.str"][..]));
        assert_eq!(str_clips_for(EffectId::Devotion), Some(&["devotion.str"][..]));
        assert_eq!(str_clips_for(EffectId::Lkconcentration), Some(&["TwoHand.str"][..]));
        assert_eq!(str_clips_for(EffectId::Rejectsword), Some(&["sword.str"][..]));
        assert_eq!(str_clips_for(EffectId::Thunderstorm), Some(&["ThunderStorm.str"][..]));
    }

    #[test]
    fn champion_rows_match_the_initialization_switch() {
        let expect = [
            // `RagEffectInitialization.cpp:262-276`.
            (EffectId::Comboattack1, "yunta_1.str"),
            (EffectId::Comboattack2, "yunta_2.str"),
            (EffectId::Comboattack3, "yunta_3.str"),
            (EffectId::Comboattack4, "yunta_4.str"),
            (EffectId::Comboattack5, "yunta_5.str"),
            // `RagEffectInitialization.cpp:347-349`.
            (EffectId::Chaincombo, "연환.str"),
        ];

        for (effect, name) in expect {
            let files = str_clips_for(effect).unwrap_or_else(|| panic!("{effect:?} has no .str row"));
            assert_eq!(files, &[name][..], "{effect:?}");
            // `CRagEffect::CRagEffect` sets `loopLimit = 0` (`RagEffect.cpp:109`)
            // and `Init` does not override it for any of these ids.
            assert_eq!(str_loop_limit(effect), 0, "{effect:?}");
        }
    }

    #[test]
    fn str_only_tier_rows_match_the_initialization_switch() {
        // Batch "e": one resource per id, all on the single-pass default.
        assert_eq!(str_clips_for(EffectId::Stonecurse), Some(&["StoneCurse.str"][..]));
        assert_eq!(str_clips_for(EffectId::Angelus), Some(&["Angelus.str"][..]));
        assert_eq!(str_clips_for(EffectId::Cure), Some(&["Cure.str"][..]));
        assert_eq!(str_clips_for(EffectId::Magnificat), Some(&["Magnificat.str"][..]));
        assert_eq!(str_clips_for(EffectId::Resurrection), Some(&["Resurrection.str"][..]));
        assert_eq!(str_clips_for(EffectId::Recovery), Some(&["Recovery.str"][..]));
        assert_eq!(str_clips_for(EffectId::Lexdivina), Some(&["LexDivina.str"][..]));
        assert_eq!(str_clips_for(EffectId::Concentration), Some(&["Concentration.str"][..]));
        assert_eq!(str_clips_for(EffectId::Twohandquicken), Some(&["TwoHand.str"][..]));
        assert_eq!(str_clips_for(EffectId::Spearquicken), Some(&["TwoHand.str"][..]));
        assert_eq!(str_clips_for(EffectId::Magicrod), Some(&["매직로드.str"][..]));

        // `CRagEffect::CRagEffect` (`loopLimit = 0`) and no `Init` override for
        // any of these ids: the resource plays once.
        for effect in [
            EffectId::Stonecurse,
            EffectId::Angelus,
            EffectId::Cure,
            EffectId::Magnificat,
            EffectId::Resurrection,
            EffectId::Recovery,
            EffectId::Lexdivina,
            EffectId::Concentration,
            EffectId::Twohandquicken,
            EffectId::Spearquicken,
            EffectId::Magicrod,
        ] {
            assert_eq!(str_loop_limit(effect), 0, "{effect:?}");
        }
    }

    #[test]
    fn str_only_follow_up_rows_match_the_initialization_switch() {
        let expect = [
            (EffectId::Lightbolt, "Lightning.str"),
            (EffectId::Provoke, "Provoke.str"),
            (EffectId::Brandishspear, "Brandish.str"),
            (EffectId::Impositio, "Impositio.str"),
            (EffectId::Lexaeterna, "LexAeterna.str"),
            (EffectId::Suffragium, "Suffragium.str"),
            (EffectId::Repairweapon, "RepairWeapon.str"),
            (EffectId::Perfection, "WeaponPerfection.str"),
            (EffectId::Maxpower, "MaximizePower.str"),
            (EffectId::Springtrap, "Spring.str"),
            (EffectId::Kyrie, "Kyrie.str"),
            (EffectId::Poisonreact, "PoisonReact_1st.str"),
            (EffectId::Slowpoison, "slowp.str"),
            (EffectId::Sonicblow2, "SonicBlow.str"),
            (EffectId::Pierceself, "Pierce.str"),
            (EffectId::Bowlingself, "Bowling.str"),
            (EffectId::Spearstabself, "SpearStab.str"),
            (EffectId::Spearbmrself, "SpearBoomerang.str"),
            (EffectId::Energycoat, "EnergyCoat.str"),
            (EffectId::Spellbreaker, "spell.str"),
            (EffectId::Dispell, "디스펠.str"),
            (EffectId::Stealcoin, "steal_coin.str"),
            (EffectId::Stripweapon, "strip_weapon.str"),
            (EffectId::Stripshield, "strip_shield.str"),
            (EffectId::Striparmor, "strip_armor.str"),
            (EffectId::Striphelm, "strip_helm.str"),
            (EffectId::Meltdown, "melt.str"),
            (EffectId::Cartboost, "cart.str"),
            (EffectId::Soulburn, "소울번.str"),
            (EffectId::Soulchange, "사람효과.str"),
            (EffectId::Assumptio2, "asum.str"),
            (EffectId::Baku, "fire dragon.str"),
            (EffectId::Hyousyouraku, "icy.str"),
            (EffectId::Bullseye, "불스아이.str"),
        ];

        for (effect, file) in expect {
            assert_eq!(str_clips_for(effect), Some(&[file][..]), "{effect:?}");
            // No follow-up row sets `state.loopLimit` in `Init`, so the
            // `CRagEffect` constructor default (`RagEffect.cpp:109`) stands.
            assert_eq!(str_loop_limit(effect), 0, "{effect:?}");
        }
    }

    #[test]
    fn positional_sound_rows_match_the_initialization_switch() {
        let expect = [
            (EffectId::Gloria, "Gloria.str"),
            (EffectId::Holycross, "holy_cross.str"),
            (EffectId::Flamelauncher, "enc_fire.str"),
            (EffectId::Frostweapon, "enc_ice.str"),
            (EffectId::Lightningloader, "enc_wind.str"),
            (EffectId::Seismicweapon, "enc_earth.str"),
            (EffectId::Loud, "loud.str"),
        ];

        for (effect, file) in expect {
            assert_eq!(str_clips_for(effect), Some(&[file][..]), "{effect:?}");
            // No positional row overrides `state.loopLimit` in `Init`, so the
            // `CRagEffect` constructor default (`RagEffect.cpp:109`) stands.
            assert_eq!(str_loop_limit(effect), 0, "{effect:?}");
        }
    }

    #[test]
    fn defaults_match_the_rag_effect_state_constructor() {
        // `CRagEffect::CRagEffect`: loopCount = loopLimit = 0, and none of the
        // rows above overrides loopLimit in `Init`.
        assert_eq!(str_loop_limit(EffectId::Signum), 0);
        assert_eq!(str_loop_limit(EffectId::Cartrevolution), 0);
        assert_eq!(str_loop_limit(EffectId::Bash), 0);
    }

    #[test]
    fn effects_without_a_str_resource_have_no_clips() {
        assert!(str_resource(EffectId::Bash).is_none());
        assert_eq!(str_clips_for(EffectId::Bash), None);
        assert_eq!(str_clips_for(EffectId::Magnumbreak), None);
    }

    #[test]
    fn raw_key_frames_convert_field_for_field() {
        let layers = [LayerData {
            texture_count: 2,
            texture_names: Vec::new(),
            frame_count: 2,
            frames: vec![
                EffectFrame {
                    frame_index: 0,
                    frame_type: 0,
                    offset: Vector2::new(1.0, 2.0),
                    uv: [3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0],
                    xy: [11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0, 18.0],
                    texture_index: 1.0,
                    animation_type: 5,
                    delay: 0.5,
                    angle: 90.0,
                    color: [0.1, 0.2, 0.3, 0.4],
                    source_blend_factor: 5,
                    destination_blend_factor: 6,
                    mt_present: 0,
                },
                EffectFrame {
                    frame_index: 12,
                    frame_type: 1,
                    offset: Vector2::new(-1.0, -2.0),
                    uv: [0.0; 8],
                    xy: [0.0; 8],
                    texture_index: 0.0,
                    animation_type: 0,
                    delay: 0.0,
                    angle: 0.0,
                    color: [0.0; 4],
                    source_blend_factor: 0,
                    destination_blend_factor: 0,
                    mt_present: 0,
                },
            ],
        }];

        let clips = clips_from_layers(&layers);

        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].texture_count, 2);
        assert_eq!(clips[0].frames.len(), 2);
        assert_eq!(clips[0].frames[0].frame, 0);
        assert_eq!(clips[0].frames[0].transform, Transform {
            x: 1.0,
            y: 2.0,
            u: 3.0,
            v: 4.0,
            us: 5.0,
            vs: 6.0,
            u2: 7.0,
            v2: 8.0,
            us2: 9.0,
            vs2: 10.0,
            ax: [11.0, 12.0, 13.0, 14.0],
            ay: [15.0, 16.0, 17.0, 18.0],
            cr_r: 0.1,
            cr_g: 0.2,
            cr_b: 0.3,
            cr_a: 0.4,
            rz: 90.0,
            kind: 0,
            anitype: 5,
            anidelta: 0.5,
            aniframe: 1.0,
        });
        assert_eq!(clips[0].frames[1].frame, 12);
        assert_eq!(clips[0].frames[1].transform.kind, 1, "delta key type");
        assert_eq!(clips[0].frames[1].transform.x, -1.0);
    }

    #[test]
    fn texture_names_become_the_loader_paths() {
        let layers = [LayerData {
            texture_count: 2,
            texture_names: vec![
                ragnarok_formats::effect::TextureName {
                    name: "cross01.bmp".to_owned(),
                },
                ragnarok_formats::effect::TextureName {
                    name: "cross02.tga".to_owned(),
                },
            ],
            frame_count: 0,
            frames: Vec::new(),
        }];

        let textures = textures_from_layers(&layers, "");

        assert_eq!(textures, vec![vec![
            "effect\\cross01.bmp".to_owned(),
            "effect\\cross02.tga".to_owned()
        ]]);
    }

    #[test]
    fn a_texture_without_an_extension_gets_the_official_bmp_default() {
        let layers = [LayerData {
            texture_count: 1,
            texture_names: vec![ragnarok_formats::effect::TextureName {
                name: "cross01".to_owned(),
            }],
            frame_count: 0,
            frames: Vec::new(),
        }];

        assert_eq!(textures_from_layers(&layers, ""), vec![vec!["effect\\cross01.bmp".to_owned()]]);
    }

    #[test]
    fn the_str_directory_prefix_is_prepended_like_the_client_loader() {
        assert_eq!(texture_prefix("Cross.str"), "");
        assert_eq!(texture_prefix("RL_STORM\\S_STORM.str"), "RL_STORM\\");

        let layers = [LayerData {
            texture_count: 1,
            texture_names: vec![ragnarok_formats::effect::TextureName {
                name: "storm01.bmp".to_owned(),
            }],
            frame_count: 0,
            frames: Vec::new(),
        }];

        assert_eq!(textures_from_layers(&layers, texture_prefix("RL_STORM\\S_STORM.str")), vec![
            vec!["effect\\RL_STORM\\storm01.bmp".to_owned()]
        ]);
    }

    #[test]
    fn an_empty_texture_list_stays_empty() {
        let layers = [LayerData {
            texture_count: 0,
            texture_names: Vec::new(),
            frame_count: 0,
            frames: Vec::new(),
        }];

        assert_eq!(textures_from_layers(&layers, ""), vec![Vec::<String>::new()]);
    }
}
