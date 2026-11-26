//! Effect-id registry: `EF_*` id -> ported effect body.
//!
//! This is the table the client will consult once the ported backend is wired
//! (plan 4.2). Bodies that open a `.str` screen-sprite clip (`Signum`,
//! `CartRevolution`) are registered like every other body: their clip is the
//! asset-side half, resolved by [`super::str_table`] and opened by
//! [`super::holder::PortedEffectHolder`] at spawn.
//!
//! [`needs_str_clips`] only reports whether an id has an official `.str`
//! resource; it no longer excludes the id from [`spawn`]/[`ported_ids`].

use ragnarok_packets::EffectId;

use super::rag::{
    AgilityUp, Aqua, Aspersio, Bash, Blessing, BlitzBeat, BottomCarpet, BottomLayer, BottomLayers, BowlingBash, CartRevolution,
    ChemicalLine, Cloud11Burst, DecAgility, Detoxication, Devotion, DoubleThrowItem, EarthSpike, ElementalWeapon, EnchantPoison,
    EnchantPoisonCast, Endure, Entry, Exit, FireArrow, FireBall, FirePillar, FireSparks, ForestLight6, FrostDiver, FrostDiver2, FrostMysty,
    GlassWall, Gloria, GrimTooth, GrimToothAtk, HealBurst, HealRingSparkle, Hit2, HolyCross, IceArrow, IceWall, IncAgility, Kaizel,
    LkConcentration, LockOn, Loud, MagicRing, MagnumBreak, MetallicSoundLine, NapalmBeat, Overthrust, PortalRings, PortalSpell, PotionPill,
    QuadRingStack, RejectSword, RemoveTrap, Revive, RingWave, RisingOrbs, RocketPunch, RogueCoin, ShieldBoomerang3, ShieldCharge, Signum,
    SonicBlow, SonicBlowHit, SpearBmr, Sphere, SprayPond, SprinkleSand, StoneSpike, StopEffect, StormGust, StormKick, Tanji,
    TeleportSpiral, TexturedBeam, TexturedComet, TexturedHalo, TexturedMagicRing, TexturedPulse, TexturedPulseKind, TexturedQuadSpin,
    TexturedShieldThrow, TexturedSkillAura, TexturedSpreadFlat, ThrowItem, Thunderstorm, TorchDustRing, TripleOrbit, Vallentine,
    VioletRingCloud, Warp, WaterBall, WaterBallBurst, WhiteSpray,
};
use super::runtime::PortedEffect;

/// Creates the ported body for an effect id, if one exists.
///
/// `.str`-driven bodies are created with no clips; the holder opens their
/// official resource from [`super::str_table`] before the first step. Returns
/// `None` only for effects that are not ported yet.
pub fn spawn(effect: EffectId) -> Option<Box<dyn PortedEffect>> {
    if let Some(body) = super::registry_class_change::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_textured_aura::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_a::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_b::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_c::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_d::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_e::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_f::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_g::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_h::spawn(effect) {
        return Some(body);
    }

    Some(match effect {
        EffectId::Hit2 => Box::new(Hit2::new(0)),
        EffectId::Endure => Box::new(Endure::default()),
        EffectId::Enchantpoison => Box::new(EnchantPoison::default()),
        EffectId::Detoxication => Box::new(Detoxication::default()),
        EffectId::Agiup => Box::new(AgilityUp::default()),
        EffectId::Detecting => Box::new(TexturedSkillAura::new(0, 250)),
        EffectId::Detect2 => Box::new(TexturedSkillAura::new(2, 250)),
        EffectId::Detect3 => Box::new(TexturedSkillAura::new(3, 250)),
        EffectId::Detect4 => Box::new(TexturedSkillAura::new(4, 250)),
        EffectId::Redline => Box::new(TexturedSpreadFlat::new("effect\\ring_red.tga", 3, 3, 75)),
        EffectId::Blueline => Box::new(TexturedSpreadFlat::new("effect\\ring_blue.tga", 12, 6, 999)),
        EffectId::Bash3d5 => Box::new(TexturedQuadSpin::new("effect\\alpha_center.tga", 5, 6, 200)),
        EffectId::Bash3d6 => Box::new(TexturedQuadSpin::new("effect\\alpha_center.tga", 6, 8, 200)),
        EffectId::Heal => Box::new(HealBurst::new(0, 100)),
        EffectId::Heal3 => Box::new(HealBurst::new(1, 100)),
        EffectId::Heal2 => Box::new(HealRingSparkle::new("effect\\ring_white.tga", 0, 100)),
        EffectId::Portal2 => Box::new(VioletRingCloud::new(0, 9999)),
        EffectId::Portal3 => Box::new(VioletRingCloud::new(3, 9999)),
        EffectId::BigPortal => Box::new(MagicRing::new(0, 2000)),
        EffectId::BigPortal2 => Box::new(MagicRing::new(1, 99_999_999)),
        EffectId::Stormgust => Box::new(StormGust::default()),
        // `CRagEffect`'s Storm Kick dispatch (`RagEffect.cpp:23663-23736`).
        // Ids 4/5 spawn thirty `WhiteLightTrail` prim-96 primitives; id 5 also
        // records the caster's `render.state`/`SetFrameSequence` request.
        EffectId::Stormkick => Box::new(StormKick::new(EffectId::Stormkick)),
        EffectId::Stormkick1 => Box::new(StormKick::new(EffectId::Stormkick1)),
        EffectId::Stormkick2 => Box::new(StormKick::new(EffectId::Stormkick2)),
        EffectId::Stormkick3 => Box::new(StormKick::new(EffectId::Stormkick3)),
        EffectId::Stormkick4 => Box::new(StormKick::new(EffectId::Stormkick4)),
        EffectId::Stormkick5 => Box::new(StormKick::new(EffectId::Stormkick5)),
        EffectId::Stormkick6 => Box::new(StormKick::new(EffectId::Stormkick6)),
        EffectId::Stormkick7 => Box::new(StormKick::new(EffectId::Stormkick7)),
        // `RagEffect.cpp:22532-22544` - `TT_MIN_EFFECT_ON_OFF` default branch.
        EffectId::NewMetallicSoundLine => Box::new(MetallicSoundLine::new(300)),
        EffectId::Fireball3 => Box::new(RocketPunch::new(false, 100)),
        EffectId::Fireball6 => Box::new(RocketPunch::new(true, 100)),
        EffectId::Sprinklesand => Box::new(SprinkleSand::new(20, 300)),
        EffectId::PotionBerserk => Box::new(PotionPill::new(false, 199)),
        EffectId::PotionBerserk2 => Box::new(PotionPill::new(true, 199)),
        EffectId::Firepillaron2 => Box::new(FirePillar::new(90)),
        EffectId::GradiusLaser => Box::new(TripleOrbit::new("effect\\white02.bmp", 500)),
        EffectId::RgCoin => Box::new(RogueCoin::coin(300)),
        EffectId::RgCoin2 => Box::new(RogueCoin::coin2(300)),
        EffectId::RgCoin3 => Box::new(RogueCoin::coin3(300)),
        EffectId::RgCoin4 => Box::new(RogueCoin::coin4(300)),
        EffectId::RgCoin5 => Box::new(RogueCoin::coin5(300)),
        EffectId::RgCoin6 => Box::new(RogueCoin::coin6(300)),
        EffectId::RgCoin7 => Box::new(RogueCoin::coin7(300)),
        EffectId::Frostmysty => Box::new(FrostMysty::new(300)),
        EffectId::Entry2 => Box::new(PortalRings::new(100)),
        EffectId::Readyportal2 => Box::new(RingWave::blue(200)),
        EffectId::Waterball => Box::new(WaterBall::new(250)),
        EffectId::Spearbmr => Box::new(SpearBmr::new(250)),
        EffectId::Edp => Box::new(EnchantPoisonCast::default()),
        EffectId::Firefly => Box::new(FireSparks::default()),
        EffectId::Halfsphere => Box::new(TexturedPulse::new("effect\\white02.bmp", 200)),
        EffectId::BottomRunner => Box::new(TexturedHalo::new("effect\\hanmoon1.tga", 3, 29999)),
        EffectId::BottomTransfer => Box::new(TexturedHalo::new("effect\\hanmoon2.tga", 3, 29999)),
        EffectId::BottomLa => Box::new(TexturedHalo::new("effect\\aaa copy.bmp", 1, 29999)),
        EffectId::BottomSpider => Box::new(TexturedHalo::new("effect\\spiderweb.tga", 2, 29999)),
        EffectId::WaterBelow => Box::new(TorchDustRing::new(2, 0, 99_999_999)),
        EffectId::BottomManhole => Box::new(BottomLayers::new(BottomLayer::Manhole, 29999)),
        EffectId::BottomAni => Box::new(BottomLayers::new(BottomLayer::Ani, 29999)),
        EffectId::BottomBloodylust => Box::new(BottomLayers::new(BottomLayer::BloodyLust, 29999)),
        EffectId::Forestlight6 => Box::new(ForestLight6::new(29999)),
        EffectId::BottomMag => Box::new(TexturedPulseKind::new("effect\\ring_red.tga", 0, 29999)),
        EffectId::BottomSanc => Box::new(TexturedPulseKind::new("effect\\alpha_down.tga", 1, 29999)),
        EffectId::Portal => Box::new(PortalSpell::new(100)),
        EffectId::Bottom => Box::new(BottomCarpet::violet()),
        EffectId::Bottom2 => Box::new(BottomCarpet::green()),
        EffectId::Glasswall => Box::new(GlassWall::new(9999)),
        EffectId::Waterball3 => Box::new(WaterBallBurst::new(100)),
        EffectId::Grimtooth => Box::new(GrimTooth::new(250)),
        EffectId::Grimtoothatk => Box::new(GrimToothAtk::default()),
        EffectId::Heavensdrive => Box::new(StoneSpike::new(250)),
        EffectId::Napalmbeat => Box::new(NapalmBeat::default()),
        EffectId::Sma3 => Box::new(TexturedComet::new(1, 50)),
        EffectId::Blitzbeat => Box::new(BlitzBeat::new(250)),
        EffectId::Spraypond => Box::new(SprayPond::new(130)),
        EffectId::Entry => Box::new(Entry::new(100)),
        EffectId::Vallentine => Box::new(Vallentine::new(0, 100)),
        EffectId::Vallentine2 => Box::new(Vallentine::new(1, 100)),
        EffectId::_05val => Box::new(Vallentine::new(2, 100)),
        EffectId::Itemfast => Box::new(Vallentine::new(3, 100)),
        EffectId::Ro2year => Box::new(Vallentine::new(4, 200)),
        EffectId::Sonicblowhit => Box::new(SonicBlowHit::new(100)),
        EffectId::Exit => Box::new(Exit::new(100)),
        EffectId::Warp => Box::new(Warp::new(80)),
        EffectId::Hptime => Box::new(RisingOrbs::new("effect\\white02.bmp", 0, 300)),
        EffectId::Sptime => Box::new(RisingOrbs::new("effect\\white02.bmp", 1, 300)),
        EffectId::Lockon => Box::new(LockOn::new(200)),
        EffectId::Exit2 => Box::new(TeleportSpiral::new("effect\\ring_purple.tga", 200)),
        EffectId::Aspersio => Box::new(Aspersio::new(999)),
        EffectId::Sphere => Box::new(Sphere::new(500)),
        EffectId::Removetrap => Box::new(RemoveTrap::new(70)),
        EffectId::Portal4 => Box::new(Cloud11Burst::new(0, 200).with_waves(&[(0, "effect\\\\윈드워크.wav")])),
        EffectId::Portal5 => Box::new(Cloud11Burst::new(1, 200)),
        EffectId::Intimidate => {
            Box::new(WhiteSpray::new(0, 20, 999).with_waves(&[(0, "effect\\EF_Bash.wav"), (35, "effect\\rog_intimidate.wav")]))
        }
        EffectId::Intimidate2 => Box::new(WhiteSpray::new(2, 16, 999)),
        EffectId::Intimidate3 => {
            Box::new(WhiteSpray::new(3, 20, 999).with_waves(&[(0, "effect\\EF_Bash.wav"), (35, "effect\\rog_intimidate.wav")]))
        }
        EffectId::Intimidate4 => Box::new(WhiteSpray::new(4, 10, 999).with_waves(&[(25, "effect\\triangle.wav")])),
        EffectId::Aqua => Box::new(Aqua::default()),
        EffectId::Revive => Box::new(Revive::default()),
        EffectId::Fireball => Box::new(FireBall::default()),
        // `RagEffect.cpp:23287-23288` / `:23278-23279`.
        EffectId::Firearrow => Box::new(FireArrow::default()),
        EffectId::Icearrow => Box::new(IceArrow::default()),
        // `RagEffect.cpp:19764-19781`; `EffectTypes.cpp:863-872` lifetimes 300.
        EffectId::ChemicalV => Box::new(ChemicalLine::new(0, 300)),
        EffectId::ChemicalV2 => Box::new(ChemicalLine::new(1, 300)),
        EffectId::ChemicalV3 => Box::new(TexturedBeam::new(2, "effect\\line3.tga", 300)),
        EffectId::Decagility => Box::new(DecAgility::default()),
        EffectId::Blessing => Box::new(Blessing::default()),
        EffectId::Bash => Box::new(Bash::default()),
        EffectId::Magnumbreak => Box::new(MagnumBreak::default()),
        EffectId::Frostdiver => Box::new(FrostDiver::new(0)),
        EffectId::Frostdiver2 => Box::new(FrostDiver2::default()),
        EffectId::Frostdiver3 => Box::new(FrostDiver::new(3)),
        EffectId::Incagility => Box::new(IncAgility::new(false)),
        EffectId::Incagidex => Box::new(IncAgility::new(true)),
        EffectId::Bowlingbash => Box::new(BowlingBash::default()),
        EffectId::Icewall => Box::new(IceWall::default()),
        EffectId::Earthspike => Box::new(EarthSpike::new(0)),
        EffectId::Hyousensou => Box::new(EarthSpike::new(2)),
        EffectId::Sonicblow => Box::new(SonicBlow::default()),
        EffectId::Overthrust => Box::new(Overthrust::default()),
        // `.str` bodies (official names in `port/str_table.rs`). The holder
        // opens the clip before frame 0; an empty constructor set is fine.
        EffectId::Signum => Box::new(Signum::new(Vec::new())),
        EffectId::Cartrevolution => Box::new(CartRevolution::new(Vec::new())),
        // `ProcessEZ2STR` dispatch cases with per-id screen offsets / parent
        // tint (`rag/shield_charge.rs`, `rag/devotion.rs`,
        // `rag/lkconcentration.rs`, `rag/reject_sword.rs`,
        // `rag/thunderstorm.rs`).
        EffectId::Shieldcharge => Box::new(ShieldCharge::new(Vec::new())),
        EffectId::Devotion => Box::new(Devotion::new(Vec::new())),
        EffectId::Lkconcentration => Box::new(LkConcentration::new(Vec::new())),
        EffectId::Rejectsword => Box::new(RejectSword::new(Vec::new())),
        EffectId::Thunderstorm => Box::new(Thunderstorm::new(Vec::new())),
        // Positional-sound dispatch cases (`rag/positional.rs`): the body is
        // the effect's `.str` clip plus one `PlayWave` with listener-relative
        // coordinates (`PlayEffectWaveAt` for Gloria and the four elemental
        // weapons, a raw `PlayWave` for Holy Cross and Loud).
        EffectId::Gloria => Box::new(Gloria::new(Vec::new())),
        EffectId::Flamelauncher | EffectId::Frostweapon | EffectId::Lightningloader | EffectId::Seismicweapon => {
            Box::new(ElementalWeapon::new(Vec::new()))
        }
        EffectId::Holycross => Box::new(HolyCross::new(Vec::new())),
        EffectId::Loud => Box::new(Loud::new(Vec::new())),
        // The shield-throw and throw-item family: each dispatch block is one
        // body call (`RagEffect.cpp:24630-24640`, `:10150-10151`, `:19456-19457`,
        // `:20446-20447`, `:21159-21172`, `:21359-21364`, `:21684-21689`,
        // `:24241-24243`); `EF_THROWITEM4` calls the same body twice.
        EffectId::Shieldboomerang => Box::new(TexturedShieldThrow::new("effect\\shield_boomerang.bmp", 499)),
        EffectId::Shieldboomerang2 => Box::new(TexturedShieldThrow::new("effect\\토마.bmp", 499)),
        EffectId::Shieldboomerang3 => Box::new(ShieldBoomerang3::new("effect\\shield_boomerang.bmp", 499)),
        EffectId::Shieldboomerang4 => Box::new(TexturedShieldThrow::new("유저인터페이스\\item\\배틀액스.bmp", 499)),
        EffectId::Tanji => Box::new(Tanji::new(300)),
        EffectId::Throwitem => Box::new(ThrowItem::new("유저인터페이스\\item\\빈병.bmp", 0, 300)),
        EffectId::Throwitem2 => Box::new(ThrowItem::new("유저인터페이스\\item\\빈포션병.bmp", 1, 300)),
        EffectId::Throwitem3 => Box::new(ThrowItem::new("유저인터페이스\\item\\돌.bmp", 2, 200)),
        EffectId::Throwitem4 => Box::new(DoubleThrowItem::new(
            ThrowItem::new("유저인터페이스\\item\\화염병.bmp", 10, 300),
            ThrowItem::new("유저인터페이스\\item\\염산병.bmp", 11, 300),
            300,
        )),
        EffectId::Throwitem5 => Box::new(ThrowItem::new("유저인터페이스\\item\\버서크포션.bmp", 1, 300)),
        EffectId::Throwitem6 => Box::new(ThrowItem::new("유저인터페이스\\item\\베넘나이프.bmp", 3, 200)),
        EffectId::Throwitem7 => Box::new(ThrowItem::new("유저인터페이스\\item\\수리검.bmp", 3, 200)),
        EffectId::Throwitem8 => Box::new(ThrowItem::new("유저인터페이스\\item\\쿠나이_독.bmp", 3, 200)),
        EffectId::Throwitem9 => Box::new(ThrowItem::new("유저인터페이스\\item\\풍마_열화.bmp", 3, 200)),
        EffectId::Throwitem10 => Box::new(ThrowItem::new("effect\\coin_a.bmp", 3, 200)),
        EffectId::Throwitem11 => Box::new(ThrowItem::new("유저인터페이스\\item\\폭탄버섯포자.bmp", 1, 300)),
        // Prims 24/48/75: the bodies that spawn `PP_3DCASTING_2`,
        // `PP_BOTTOM2` and `PP_SLASH1`.
        //
        // `case EF_DEFENDER:` / `case EF_REFLECTSHIELD:` - `RagEffect.cpp:24194-24199`.
        EffectId::Defender => Box::new(QuadRingStack::new("effect\\ring_black.tga", 200)),
        EffectId::Reflectshield => Box::new(QuadRingStack::new("effect\\ring_yellow.tga", 200)),
        // `case EF_KAIZEL:` - `RagEffect.cpp:21287-21292`, lifetime 200
        // (`EffectTypes.cpp:599`).
        EffectId::Kaizel => Box::new(Kaizel::new(200)),
        // `case EF_STOPEFFECT:` - `RagEffect.cpp:25147-25154`, lifetime 100
        // (`EffectTypes.cpp:453`).
        EffectId::Stopeffect => Box::new(StopEffect::new(100)),
        // The bottom-song family - `RagEffect.cpp:23989-24037`, all lifetime
        // 29999 (`EffectTypes.cpp:286`, `:296`, `:299`, `:301`, `:303`).
        EffectId::BottomDissonance => Box::new(TexturedMagicRing::new("effect\\ring_blue.tga", 0, 29999)),
        EffectId::BottomAssassincross => Box::new(TexturedMagicRing::new("effect\\ring_red.tga", 1, 29999)),
        EffectId::BottomUglydance => Box::new(TexturedMagicRing::new("effect\\ring_red.tga", 0, 29999)),
        EffectId::BottomDontforgetme => Box::new(TexturedMagicRing::new("effect\\magic_green.tga", 2, 29999)),
        EffectId::BottomServiceforyou => Box::new(TexturedMagicRing::new("effect\\safeline.bmp", 4, 29999)),
        _ => return None,
    })
}

/// Whether the effect opens an official `.str` screen-sprite resource
/// (`CRagEffect::Init`, `RagEffectInitialization.cpp:16-711`).
///
/// Informational: the resource is opened by the holder at spawn, so this no
/// longer excludes an id from [`spawn`] or [`ported_ids`].
pub fn needs_str_clips(effect: EffectId) -> bool {
    super::str_table::str_resource(effect).is_some()
}

/// Every ported effect id.
pub fn ported_ids() -> Vec<EffectId> {
    let mut ids = super::registry_class_change::ids().to_vec();
    ids.extend_from_slice(super::registry_textured_aura::ids());
    ids.extend_from_slice(super::registry_batch_a::ids());
    ids.extend_from_slice(super::registry_batch_b::ids());
    ids.extend_from_slice(super::registry_batch_c::ids());
    ids.extend_from_slice(super::registry_batch_d::ids());
    ids.extend_from_slice(super::registry_batch_e::ids());
    ids.extend_from_slice(super::registry_batch_f::ids());
    ids.extend_from_slice(super::registry_batch_g::ids());
    ids.extend_from_slice(super::registry_batch_h::ids());

    ids.extend_from_slice(&[
        EffectId::Hit2,
        EffectId::Endure,
        EffectId::Blessing,
        EffectId::Fireball,
        EffectId::Firearrow,
        EffectId::Icearrow,
        EffectId::ChemicalV,
        EffectId::ChemicalV2,
        EffectId::ChemicalV3,
        EffectId::NewMetallicSoundLine,
        EffectId::Decagility,
        EffectId::Aqua,
        EffectId::Revive,
        EffectId::Enchantpoison,
        EffectId::Detoxication,
        EffectId::Agiup,
        EffectId::Detecting,
        EffectId::Detect2,
        EffectId::Detect3,
        EffectId::Detect4,
        EffectId::Redline,
        EffectId::Blueline,
        EffectId::Bash3d5,
        EffectId::Bash3d6,
        EffectId::Heal,
        EffectId::Heal3,
        EffectId::Heal2,
        EffectId::Portal2,
        EffectId::Portal3,
        EffectId::BigPortal,
        EffectId::BigPortal2,
        EffectId::Stormgust,
        EffectId::Stormkick,
        EffectId::Stormkick1,
        EffectId::Stormkick2,
        EffectId::Stormkick3,
        EffectId::Stormkick4,
        EffectId::Stormkick5,
        EffectId::Stormkick6,
        EffectId::Stormkick7,
        EffectId::Fireball3,
        EffectId::Fireball6,
        EffectId::Sprinklesand,
        EffectId::PotionBerserk,
        EffectId::PotionBerserk2,
        EffectId::Firepillaron2,
        EffectId::GradiusLaser,
        EffectId::RgCoin,
        EffectId::RgCoin2,
        EffectId::RgCoin3,
        EffectId::RgCoin4,
        EffectId::RgCoin5,
        EffectId::RgCoin6,
        EffectId::RgCoin7,
        EffectId::Frostmysty,
        EffectId::Entry2,
        EffectId::Readyportal2,
        EffectId::Waterball,
        EffectId::Spearbmr,
        EffectId::Edp,
        EffectId::Firefly,
        EffectId::Halfsphere,
        EffectId::BottomRunner,
        EffectId::BottomTransfer,
        EffectId::BottomLa,
        EffectId::BottomSpider,
        EffectId::WaterBelow,
        EffectId::BottomManhole,
        EffectId::BottomAni,
        EffectId::BottomBloodylust,
        EffectId::Forestlight6,
        EffectId::BottomMag,
        EffectId::BottomSanc,
        EffectId::Portal,
        EffectId::Bottom,
        EffectId::Bottom2,
        EffectId::Glasswall,
        EffectId::Waterball3,
        EffectId::Grimtooth,
        EffectId::Grimtoothatk,
        EffectId::Heavensdrive,
        EffectId::Napalmbeat,
        EffectId::Sma3,
        EffectId::Blitzbeat,
        EffectId::Spraypond,
        EffectId::Entry,
        EffectId::Vallentine,
        EffectId::Vallentine2,
        EffectId::_05val,
        EffectId::Itemfast,
        EffectId::Ro2year,
        EffectId::Sonicblowhit,
        EffectId::Exit,
        EffectId::Warp,
        EffectId::Hptime,
        EffectId::Sptime,
        EffectId::Lockon,
        EffectId::Exit2,
        EffectId::Aspersio,
        EffectId::Sphere,
        EffectId::Removetrap,
        EffectId::Portal4,
        EffectId::Portal5,
        EffectId::Intimidate,
        EffectId::Intimidate2,
        EffectId::Intimidate3,
        EffectId::Intimidate4,
        EffectId::Bash,
        EffectId::Magnumbreak,
        EffectId::Frostdiver,
        EffectId::Frostdiver2,
        EffectId::Frostdiver3,
        EffectId::Incagility,
        EffectId::Incagidex,
        EffectId::Bowlingbash,
        EffectId::Icewall,
        EffectId::Earthspike,
        EffectId::Hyousensou,
        EffectId::Sonicblow,
        EffectId::Overthrust,
        EffectId::Signum,
        EffectId::Cartrevolution,
        EffectId::Shieldcharge,
        EffectId::Devotion,
        EffectId::Lkconcentration,
        EffectId::Rejectsword,
        EffectId::Thunderstorm,
        EffectId::Gloria,
        EffectId::Flamelauncher,
        EffectId::Frostweapon,
        EffectId::Lightningloader,
        EffectId::Seismicweapon,
        EffectId::Holycross,
        EffectId::Loud,
        EffectId::Shieldboomerang,
        EffectId::Shieldboomerang2,
        EffectId::Shieldboomerang3,
        EffectId::Shieldboomerang4,
        EffectId::Tanji,
        EffectId::Throwitem,
        EffectId::Throwitem2,
        EffectId::Throwitem3,
        EffectId::Throwitem4,
        EffectId::Throwitem5,
        EffectId::Throwitem6,
        EffectId::Throwitem7,
        EffectId::Throwitem8,
        EffectId::Throwitem9,
        EffectId::Throwitem10,
        EffectId::Throwitem11,
        EffectId::Defender,
        EffectId::Reflectshield,
        EffectId::Kaizel,
        EffectId::Stopeffect,
        EffectId::BottomDissonance,
        EffectId::BottomAssassincross,
        EffectId::BottomUglydance,
        EffectId::BottomDontforgetme,
        EffectId::BottomServiceforyou,
    ]);

    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ported_id_spawns_with_its_official_lifetime() {
        for effect in ported_ids() {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
            assert!(body.lifetime() > 0, "{effect:?} has no lifetime");
        }
    }

    #[test]
    fn lifetimes_match_the_lifetime_table() {
        let expect = [
            (EffectId::Hit2, 50),
            (EffectId::Endure, 80),
            (EffectId::Blessing, 150),
            (EffectId::Fireball, 100),
            (EffectId::Decagility, 100),
            (EffectId::Aqua, 100),
            (EffectId::Revive, 250),
            (EffectId::Enchantpoison, 250),
            (EffectId::Detoxication, 100),
            (EffectId::Agiup, 100),
            (EffectId::Detecting, 250),
            (EffectId::Detect2, 250),
            (EffectId::Detect3, 250),
            (EffectId::Detect4, 250),
            (EffectId::Redline, 75),
            (EffectId::Blueline, 999),
            (EffectId::Bash3d5, 200),
            (EffectId::Bash3d6, 200),
            (EffectId::Heal, 100),
            (EffectId::Heal3, 100),
            (EffectId::Heal2, 100),
            (EffectId::Portal2, 9999),
            (EffectId::Portal3, 9999),
            (EffectId::BigPortal, 2000),
            (EffectId::BigPortal2, 99_999_999),
            (EffectId::Stormgust, 9999),
            (EffectId::Stormkick, 100),
            (EffectId::Stormkick1, 100),
            (EffectId::Stormkick2, 100),
            (EffectId::Stormkick3, 100),
            (EffectId::Stormkick4, 200),
            (EffectId::Stormkick5, 300),
            (EffectId::Stormkick6, 100),
            (EffectId::Stormkick7, 1000),
            (EffectId::Fireball3, 100),
            (EffectId::Fireball6, 100),
            (EffectId::Sprinklesand, 300),
            (EffectId::PotionBerserk, 199),
            (EffectId::PotionBerserk2, 199),
            (EffectId::Firepillaron2, 90),
            (EffectId::GradiusLaser, 500),
            (EffectId::RgCoin, 300),
            (EffectId::RgCoin2, 300),
            (EffectId::RgCoin3, 300),
            (EffectId::RgCoin4, 300),
            (EffectId::RgCoin5, 300),
            (EffectId::RgCoin6, 300),
            (EffectId::RgCoin7, 300),
            (EffectId::Frostmysty, 300),
            (EffectId::Entry2, 100),
            (EffectId::Readyportal2, 200),
            (EffectId::Waterball, 250),
            (EffectId::Spearbmr, 250),
            (EffectId::Edp, 120),
            (EffectId::Firefly, 100),
            (EffectId::Halfsphere, 200),
            (EffectId::BottomRunner, 29999),
            (EffectId::BottomTransfer, 29999),
            (EffectId::BottomLa, 29999),
            (EffectId::BottomSpider, 29999),
            (EffectId::WaterBelow, 99_999_999),
            (EffectId::BottomManhole, 29999),
            (EffectId::BottomAni, 29999),
            (EffectId::BottomBloodylust, 29999),
            (EffectId::Forestlight6, 29999),
            (EffectId::BottomMag, 29999),
            (EffectId::BottomSanc, 29999),
            (EffectId::Portal, 100),
            (EffectId::Bottom, 350),
            (EffectId::Bottom2, 350),
            (EffectId::Glasswall, 9999),
            (EffectId::Waterball3, 100),
            (EffectId::Grimtooth, 250),
            (EffectId::Grimtoothatk, 250),
            (EffectId::Heavensdrive, 250),
            (EffectId::Napalmbeat, 100),
            (EffectId::Sma3, 50),
            (EffectId::Blitzbeat, 250),
            (EffectId::Spraypond, 130),
            (EffectId::Entry, 100),
            (EffectId::Vallentine, 100),
            (EffectId::Vallentine2, 100),
            (EffectId::_05val, 100),
            (EffectId::Itemfast, 100),
            (EffectId::Ro2year, 200),
            (EffectId::Sonicblowhit, 100),
            (EffectId::Exit, 100),
            (EffectId::Warp, 80),
            (EffectId::Hptime, 300),
            (EffectId::Sptime, 300),
            (EffectId::Lockon, 200),
            (EffectId::Exit2, 200),
            (EffectId::Aspersio, 999),
            (EffectId::Sphere, 500),
            (EffectId::Removetrap, 70),
            (EffectId::Portal4, 200),
            (EffectId::Portal5, 200),
            (EffectId::Intimidate, 999),
            (EffectId::Intimidate2, 999),
            (EffectId::Intimidate3, 999),
            (EffectId::Intimidate4, 999),
            (EffectId::Bash, 40),
            (EffectId::Firearrow, 160),
            (EffectId::Icearrow, 160),
            (EffectId::ChemicalV, 300),
            (EffectId::ChemicalV2, 300),
            (EffectId::ChemicalV3, 300),
            (EffectId::NewMetallicSoundLine, 300),
            (EffectId::Magnumbreak, 30),
            (EffectId::Frostdiver, 150),
            (EffectId::Frostdiver2, 100),
            (EffectId::Frostdiver3, 150),
            (EffectId::Incagility, 100),
            (EffectId::Incagidex, 100),
            (EffectId::Bowlingbash, 250),
            (EffectId::Icewall, 250),
            (EffectId::Earthspike, 250),
            (EffectId::Hyousensou, 250),
            (EffectId::Sonicblow, 40),
            (EffectId::Overthrust, 250),
            (EffectId::Signum, 999),
            (EffectId::Cartrevolution, 999),
            (EffectId::Shieldcharge, 499),
            (EffectId::Devotion, 150),
            (EffectId::Lkconcentration, 99999),
            (EffectId::Rejectsword, 200),
            (EffectId::Thunderstorm, 104),
            (EffectId::Gloria, 999),
            (EffectId::Flamelauncher, 999),
            (EffectId::Frostweapon, 999),
            (EffectId::Lightningloader, 999),
            (EffectId::Seismicweapon, 999),
            (EffectId::Holycross, 499),
            (EffectId::Loud, 300),
            (EffectId::Shieldboomerang, 499),
            (EffectId::Shieldboomerang2, 499),
            (EffectId::Shieldboomerang3, 499),
            (EffectId::Shieldboomerang4, 499),
            (EffectId::Tanji, 300),
            (EffectId::Throwitem, 300),
            (EffectId::Throwitem2, 300),
            (EffectId::Throwitem3, 200),
            (EffectId::Throwitem4, 300),
            (EffectId::Throwitem5, 300),
            (EffectId::Throwitem6, 200),
            (EffectId::Throwitem7, 200),
            (EffectId::Throwitem8, 200),
            (EffectId::Throwitem9, 200),
            (EffectId::Throwitem10, 200),
            (EffectId::Throwitem11, 300),
            (EffectId::Defender, 200),
            (EffectId::Reflectshield, 200),
            (EffectId::Kaizel, 200),
            (EffectId::Stopeffect, 100),
            (EffectId::BottomDissonance, 29999),
            (EffectId::BottomAssassincross, 29999),
            (EffectId::BottomUglydance, 29999),
            (EffectId::BottomDontforgetme, 29999),
            (EffectId::BottomServiceforyou, 29999),
        ];

        for (effect, lifetime) in expect {
            let body = spawn(effect).unwrap();
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn str_driven_bodies_spawn_like_every_other_body() {
        assert!(needs_str_clips(EffectId::Signum));
        assert!(needs_str_clips(EffectId::Cartrevolution));
        assert_eq!(
            super::super::str_table::str_clips_for(EffectId::Signum),
            Some(&["Cross.str"][..])
        );
        assert_eq!(
            super::super::str_table::str_clips_for(EffectId::Cartrevolution),
            Some(&["CartRevolution.str"][..])
        );
        assert!(spawn(EffectId::Signum).is_some());
        assert!(spawn(EffectId::Cartrevolution).is_some());
    }

    #[test]
    fn ported_ids_are_unique_and_include_the_str_driven_bodies() {
        let ids = ported_ids();

        // No magic total: the batches grow the registry, so the assertion is
        // about the invariants (uniqueness + the str-driven ids) instead of a
        // literal count that every new body would have to edit.
        // `EffectId` is not `Hash`/`Eq`, so compare by its debug name.
        let mut names: Vec<String> = ids.iter().map(|id| format!("{id:?}")).collect();
        names.sort();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "ported_ids() must not repeat an id");

        assert!(ids.iter().any(|&id| matches!(id, EffectId::Signum)));
        assert!(ids.iter().any(|&id| matches!(id, EffectId::Cartrevolution)));
    }

    #[test]
    fn every_registered_body_assigns_its_official_texture() {
        use crate::world::effect::port::primitive::PrimitiveId;
        use crate::world::effect::port::runtime::{EffectRuntime, step};

        for effect in ported_ids() {
            let mut body = spawn(effect).unwrap();
            let mut runtime = EffectRuntime::new(body.lifetime(), [0.0, 0.0], 99);
            // The skill layer sends `AM_SETCOUNT` before the first frame
            // (`RagEffect.cpp:14924`); a level-1 cast is the default harness,
            // and without it Ice Arrow (`EF_ICEARROW`) emits nothing at all.
            runtime.count = 1;

            // Some bodies only start emitting after a delay (Fire Ball at frame
            // 20), so run a few frames before requiring primitives.
            // 160 frames covers the bodies whose first primitive is late
            // (Storm Gust's first fragment lands on frame 58).
            for _ in 0..160 {
                step(&mut body, &mut runtime);

                if !runtime.primitives.is_empty() {
                    break;
                }
            }

            if runtime.primitives.is_empty() {
                // A `.str`-only body is drawn by its clip, never by a
                // primitive (e.g. Two Hand Quicken, Lex Divina). Those ids are
                // legitimate as long as the string table knows their resource;
                // every other body must still spawn geometry.
                // Caster-body effects (the whole manual tier) legitimately spawn
                // neither geometry nor a clip; each batch declares its own.
                assert!(
                    needs_str_clips(effect)
                        || super::super::registry_batch_h::is_caster_body_only(effect),
                    "{effect:?} spawned nothing"
                );
                continue;
            }

            for primitive in &runtime.primitives {
                // Two legitimate shapes carry no texture: sprite particles
                // (`particle1/3/7.spr`, `.spr/.act` only) and the plain
                // `Particle3d`/`Orbit` motes whose sprite the original attaches
                // elsewhere (e.g. Blessing). Everything else must name a
                // texture.
                if primitive.state.sprite.is_some() || matches!(primitive.state.count, PrimitiveId::Particle3d | PrimitiveId::Orbit) {
                    continue;
                }

                assert!(
                    primitive.state.texture.is_some(),
                    "{effect:?} left a {:?} primitive without a texture",
                    primitive.state.count
                );
            }
        }
    }

    #[test]
    fn key_textures_match_the_decompilation() {
        use crate::world::effect::port::runtime::{EffectRuntime, step};

        let mut bash = spawn(EffectId::Bash).unwrap();
        let mut runtime = EffectRuntime::new(bash.lifetime(), [0.0, 0.0], 1);
        step(&mut bash, &mut runtime);

        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\alpha_down.tga"));
        assert_eq!(runtime.primitives[1].state.texture, Some("EFFECT\\alpha_center.tga"));

        let mut bowling = spawn(EffectId::Bowlingbash).unwrap();
        let mut runtime = EffectRuntime::new(bowling.lifetime(), [0.0, 0.0], 1);
        step(&mut bowling, &mut runtime);

        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(runtime.primitives[1].state.texture, Some("effect\\ring_yellow.tga"));
    }

    #[test]
    fn unported_effects_return_none() {
        // `EF_STORMKICK8` (1223) is the separate `CStormKickEffect` particle
        // implementation, not the `CRagEffect` body ported here.
        assert!(spawn(EffectId::Stormkick8).is_none());
        assert!(
            !ported_ids()
                .iter()
                .any(|effect| !needs_str_clips(*effect) && spawn(*effect).is_none())
        );
    }
}
