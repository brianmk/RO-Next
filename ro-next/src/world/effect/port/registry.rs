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
    AgilityUp, AlphaCenterStream, AlphaCenterTrail, Aqua, Aspersio, Bash, BeginSpellRings, BeginSpellSwirl, Blessing, BlitzBeat,
    BloodDrain, BlueRingStack, BlueSpellRings, BottomCarpet, BottomLayer, BottomLayers, BottomMusic, BottomOut, BowlingBash,
    CartRevolution, Chemical3, Chemical4, ChemicalLine, ChemicalProtection, Chimto, Cloud11Burst, CloudDrift, DecAgility, Detoxication,
    Devotion, DoubleThrowItem, EarthSpike, ElementalWeapon, EnchantPoison, EnchantPoisonCast, Endure, EnergyDrain2, EnergyDrain3,
    EnergyDrainBlack, Entry, Exit, FeatherBurst, FeatherRing, FireArrow, FireBall, FirePillar, FireSparks, FirstAid, ForestLight6,
    FrostDiver, FrostDiver2, FrostMysty, GlassWall, Gloria, GrandCross, GrimTooth, GrimToothAtk, Guard2, Hated, HealBurst, HealRingSparkle,
    Hit2, HolyCross, IceArrow, IceWall, IncAgility, JumpKick, Kaizel, Kouenka, LineLink, LkConcentration, LockOn, Loud, MagicRing, Magnum2,
    MagnumBreak, MetallicSoundLine, NapalmBeat, NapalmValcan, Overthrust, ParentTint, PortalRings, PortalSpell, PotionPill, Pressure,
    Providence, QuadRingStack, RedParticleSpray, RejectSword, RemoveTrap, Revive, RingWave, RisingOrbs, RocketPunch, RogueCoin, Ruwach,
    ShieldBoomerang3, ShieldCharge, Signum, SlashCross, SlashFan, SlashLevel, SlashWave, SonicBlow, SonicBlowHit, SoulBreaker,
    SoulBreaker2, SoulLink, SoulStrike, SpearBmr, Sphere, SprayPond, SprinkleSand, Stin3, StoneSpike, StopEffect, StormGust, StormKick,
    Tanji, TeiHit1T, TeiHit2, TeleportSpiral, TexturedBeam, TexturedBlueRing, TexturedComet, TexturedGlow, TexturedHalo, TexturedMagicRing,
    TexturedPulse, TexturedPulseKind, TexturedQuadSpin, TexturedScatter, TexturedShieldThrow, TexturedSkillAura, TexturedSpreadFlat,
    TexturedTumble, ThrowItem, Thunderstorm, TorchDustRing, TripleOrbit, Vallentine, VioletRingCloud, Warp, WaterBall, WaterBall2,
    WaterBallBurst, White02Burst, WhiteSparkRing, WhiteSparkStreak, WhiteSpray,
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

    if let Some(body) = super::registry_batch_i::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_j::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_k::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::registry_batch_l::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::rag::asura::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::rag::textured_slam::spawn(effect) {
        return Some(body);
    }

    if let Some(body) = super::rag::champion::spawn(effect) {
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
        // `RagEffect.cpp:24203-24210` - the two Grand Cross calls.
        EffectId::Grandcross => Box::new(GrandCross::new(0, 999)),
        EffectId::Grandcross2 => Box::new(GrandCross::new(1, 999)),
        // `RagEffect.cpp:24228-24231` (24 x `TexturedScatter`) and
        // `:24348-24351` (20 x `TexturedScatterLow`).
        EffectId::Teihit1x => Box::new(TexturedScatter::new("effect\\lens1.tga", 1, 24, false, 300)),
        EffectId::Teihit3 => Box::new(TexturedScatter::new("effect\\lens2.tga", 2, 20, true, 300)),
        // `RagEffect.cpp:24233-24236` (5 x `TEIHIT2`) and `:24247-24255`
        // (20 x `TEIHIT2` with the frame-20 Back Stab wave).
        EffectId::Teihit2 => Box::new(TeiHit2::new(5, false, 300)),
        EffectId::Backstap => Box::new(TeiHit2::new(20, true, 300)),
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
        // `case EF_PORTAL4:` - `RagEffect.cpp:23300-23311` (Wind Walk's
        // success lane): `Cloud11Burst(0)`, the inline
        // `effect\윈드워크.wav` on frame 0, and the caster's
        // `SetColor(-1, 220, 250, 220)` + `render.flags += 1` window on frames
        // 5-25. Lifetime 200 (`EffectTypes.cpp:398`).
        EffectId::Portal4 => Box::new(
            Cloud11Burst::new(0, 200)
                .with_waves(&[(0, "effect\\윈드워크.wav")])
                .with_tint(ParentTint {
                    first_frame: 5,
                    last_frame: 25,
                    color: [-1, 220, 250, 220],
                    flags: 1,
                }),
        ),
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
        // `EF_CHEMICAL3` / `EF_CHEMICAL4` - `RagEffect.cpp:24367-24373`,
        // `:21124-21133`; lifetimes `EffectTypes.cpp:448`, `:632`.
        // `EF_PROVIDENCE` - `RagEffect.cpp:24609-24614`, lifetime
        // `EffectTypes.cpp:257`.
        // `EF_SPHEREWIND2` - `RagEffect.cpp:23143-23147`, lifetime
        // `EffectTypes.cpp:403`.
        EffectId::Spherewind2 => Box::new(FeatherRing::new(99_999)),
        // `EF_STIN` / `EF_STIN2` / `EF_STIN4` / `EF_STIN5` -
        // `RagEffect.cpp:21810-21814`, `:21817-21828`, `:21150-21156`,
        // `:21117-21123`; lifetimes `EffectTypes.cpp:556`, `:562`, `:630`,
        // `:633`.
        EffectId::Stin => Box::new(SlashFan::new(200)),
        EffectId::Stin2 => Box::new(SlashLevel::new(200)),
        EffectId::Stin4 => Box::new(SlashWave::new(200)),
        EffectId::Stin5 => Box::new(SlashCross::new(200)),
        // `EF_JUMPKICK` - `RagEffect.cpp:24591-24604`, lifetime
        // `EffectTypes.cpp:466`.
        // `EF_SOULLINK` / `EF_CARTTER` - `RagEffect.cpp:21566-21586`,
        // `:21477-21482`; lifetimes `EffectTypes.cpp:512`, `:527`.
        EffectId::Soullink => Box::new(SoulLink::new(500)),
        EffectId::Cartter => Box::new(TexturedTumble::new(200)),
        EffectId::Jumpkick => Box::new(JumpKick::new(100)),
        EffectId::Providence => Box::new(Providence::new(200)),
        EffectId::Chemical3 => Box::new(Chemical3::new(300)),
        EffectId::Chemical4 => Box::new(Chemical4::new(300)),
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
        // `PP_BOTTOM` (47) / `PP_BOTTOMOUT` (49) - the spawn-once
        // `Bottom_Music` / `Bottom_Out` bodies (`RagEffect.cpp:12276-12384`
        // and `:11938-12036`). `EF_BOTTOM_ETERNALCHAOS` (280) and
        // `EF_BOTTOM_SIEGFRIED` (285) are **not** part of this family: they
        // dispatch to `TexturedGlow` (prim 50), registered below.
        EffectId::BottomLullaby => Box::new(BottomMusic::new("effect\\zz.bmp", 1, 29999)),
        EffectId::BottomRichmankim => Box::new(BottomMusic::new("effect\\pocket.bmp", 3, 29999)),
        EffectId::BottomDrumbattlefield => Box::new(BottomMusic::new("", 2, 29999)),
        EffectId::BottomRingnibelungen => Box::new(BottomMusic::new("effect\\twirl.bmp", 7, 29999)),
        EffectId::BottomRokisweil => Box::new(BottomOut::new("effect\\safeline.bmp", 0, 29999)),
        EffectId::BottomIntoabyss => Box::new(BottomMusic::new("", 5, 29999)),
        EffectId::BottomWhistle => Box::new(BottomMusic::new("effect\\melody_b.bmp", 0, 29999)),
        EffectId::BottomPoembragi => Box::new(BottomMusic::new("", 8, 29999)),
        EffectId::BottomAppleidun => Box::new(BottomMusic::new("effect\\idun_apple.bmp", 6, 29999)),
        EffectId::BottomHumming => Box::new(BottomMusic::new("effect\\melody_a.bmp", 0, 29999)),
        EffectId::BottomFortunekiss => Box::new(BottomMusic::new("effect\\kiss.bmp", 4, 29999)),
        // `PP_3DPARTICLESPLINE` (8) family. `case EF_WATERBALL2:`
        // `RagEffect.cpp:23932-23934`; `case EF_BLOODDRAIN:` `:24738-24740`;
        // `case EF_ENERGYDRAIN:` `:24741-24743`; `case EF_ENERGYDRAIN2:`
        // `:24744-24752`; `case EF_ENERGYDRAIN3:` `:24753-24761`;
        // `case EF_SOULSTRIKE:` `:23168-23170`; `case EF_SOULSTRIKE2:`
        // `:23178-23180`; `case EF_ENERGYDRAIN_BLACK:` `:22263-22270`.
        // Lifetimes: `EffectTypes.cpp:126`, `:225`, `:226`, `:387`, `:392`,
        // `:24`, `:460`, `:1217`.
        EffectId::Waterball2 => Box::new(WaterBall2::new(100)),
        EffectId::Blooddrain => Box::new(RedParticleSpray::new(3, 200)),
        EffectId::Energydrain => Box::new(BloodDrain::new(200)),
        EffectId::Energydrain2 => Box::new(EnergyDrain2::new(200)),
        EffectId::Energydrain3 => Box::new(EnergyDrain3::new(200)),
        EffectId::EnergydrainBlack => Box::new(EnergyDrainBlack::new(200)),
        EffectId::Soulstrike => Box::new(SoulStrike::new(false, 100)),
        EffectId::Soulstrike2 => Box::new(SoulStrike::new(true, 100)),
        // ------------------------------------------------------------------
        // Renewal cluster: the three free ids and prims 46/69/100.
        //
        // FREE IDS - bodies/primitives were already ported.
        // ------------------------------------------------------------------
        // `case EF_HATED:` - `RagEffect.cpp:21334-21347`; the inline caster
        // tint plus `ParticleTime("effect\\white02.bmp", 1)`. Lifetime 500
        // (`EffectTypes.cpp:552`).
        EffectId::Hated => Box::new(Hated::new(500)),
        // `case EF_BLUECASTING:` - `RagEffect.cpp:23599-23601` calls
        // `BlueSpellRings()` (`:18667-18684`). Lifetime 60
        // (`EffectTypes.cpp:450`).
        EffectId::Bluecasting => Box::new(BlueSpellRings::new(60)),
        // `case EF_NAPALMVALCAN:` - `RagEffect.cpp:23293-23301`. The body only
        // plays a wave; its `Hit2(0)` call is dead after frame 0. Lifetime 200
        // (`EffectTypes.cpp:410`).
        EffectId::Napalmvalcan => Box::new(NapalmValcan::new(200)),
        // ------------------------------------------------------------------
        // Prim 46 - `CHIMTO`/`FVOICE`/`WINK` (`RagEffect.cpp:14419-14511`).
        // `case EF_CHIMTO:` is `:24238-24240`; `case EF_FVOICE:` `:21245-21246`;
        // `case EF_WINK:` `:21248-21249`. Lifetimes 16/100/100
        // (`EffectTypes.cpp:276`, `:615`, `:616`).
        // ------------------------------------------------------------------
        EffectId::Chimto => Box::new(Chimto::new(0, 16)),
        EffectId::Fvoice => Box::new(Chimto::new(2, 100)),
        EffectId::Wink => Box::new(Chimto::new(1, 100)),
        // ------------------------------------------------------------------
        // Prim 100 `PP_HITLINE3` - the `WhiteSpark*` family.
        // `case EF_HITLINE4:` / `EF_HITLINE5:` / `EF_HITLINE6:` -
        // `RagEffect.cpp:23651-23659`. Lifetimes 200 each
        // (`EffectTypes.cpp:438-440`).
        // ------------------------------------------------------------------
        EffectId::Hitline4 => Box::new(White02Burst::new(3, 200)),
        EffectId::Hitline5 => Box::new(WhiteSparkRing::new(3, 200)),
        EffectId::Hitline6 => Box::new(WhiteSparkStreak::new(3, 200)),
        // ------------------------------------------------------------------
        // Prim 69 - the `EF_TRIPLEATTACK*` bodies.
        // `case EF_TRIPLEATTACK:` - `RagEffect.cpp:10214-10215` (`CloudDrift`);
        // `case EF_TRIPLEATTACK2:` `:24353-24358`; `case EF_TRIPLEATTACK3:`
        // `:24360-24365`. Lifetimes 100/200/200 (`EffectTypes.cpp:338`,
        // `:397`, `:402`).
        // ------------------------------------------------------------------
        EffectId::Tripleattack => Box::new(CloudDrift::new(100)),
        EffectId::Tripleattack2 => Box::new(AlphaCenterTrail::new(200)),
        EffectId::Tripleattack3 => Box::new(AlphaCenterStream::new(200)),
        // ------------------------------------------------------------------
        // Round 100: the last primitive clusters - 50/38/54/58 plus the free
        // `EF_TEIHIT1`/`EF_TEIHIT1T` ids.
        // ------------------------------------------------------------------
        // `case EF_TEIHIT1:` - `RagEffect.cpp:24223-24227`: 20 x
        // `TexturedScatter("effect\\alpha_center.tga", 0)`. Lifetime 300
        // (`EffectTypes.cpp:271`).
        EffectId::Teihit1 => Box::new(TexturedScatter::new("effect\\alpha_center.tga", 0, 20, false, 300)),
        // `case EF_TEIHIT1T:` - `RagEffect.cpp:19571-19583`: frame-5 wave and
        // parent copy plus the scatter loop. Lifetime 300
        // (`EffectTypes.cpp:898`).
        EffectId::Teihit1t => Box::new(TeiHit1T::new(20, 300)),
        // Prim 50 `TexturedGlow` - `RagEffect.cpp:24006-24007`, `:24021-24022`.
        // Lifetimes 29999 each (`EffectTypes.cpp:289`, `:294`).
        EffectId::BottomEternalchaos => Box::new(TexturedGlow::new("effect\\twirl.bmp", 8, 29999)),
        EffectId::BottomSiegfried => Box::new(TexturedGlow::new("effect\\twirl.bmp", 4, 29999)),
        // Prim 38 `PP_LINELINK` - `RagEffect.cpp:24421-24433`. Lifetimes 200
        // and 99999999 (`EffectTypes.cpp:393`, `:404`).
        EffectId::Linelink2 => Box::new(LineLink::new(false, 200)),
        EffectId::Linelink3 => Box::new(LineLink::new(true, 99_999_999)),
        // Prim 54 `PP_CHEMICALPROTECTION` - `RagEffect.cpp:10411-10418`.
        // Lifetime 300 (`EffectTypes.cpp:309`).
        EffectId::Chemicalprotection => Box::new(ChemicalProtection::new(300)),
        // Prim 58 `PP_FIRSTAID` - `RagEffect.cpp:10153-10154`. Lifetime 299
        // (`EffectTypes.cpp:318`).
        EffectId::Firstaid => Box::new(FirstAid::new(299)),
        // ------------------------------------------------------------------
        // Round 101: the pressure/guard/magnum/soulbreaker/baby families.
        // ------------------------------------------------------------------
        // `case EF_MAGNUM2:` - `RagEffect.cpp:10249-10250`. Lifetime 100
        // (`EffectTypes.cpp:348`).
        EffectId::Magnum2 => Box::new(Magnum2::new("effect\\ring_yellow.tga", 100)),
        // `case EF_SOULBREAKER:` - `RagEffect.cpp:10320-10332`. Lifetime 300
        // (`EffectTypes.cpp:370`).
        EffectId::Soulbreaker => Box::new(SoulBreaker::new(300)),
        // `case EF_PRESSURE:` - `RagEffect.cpp:10342-10349`. Lifetime 300
        // (`EffectTypes.cpp:374`).
        EffectId::Pressure => Box::new(Pressure::new(300)),
        // `case EF_BABY:` - `RagEffect.cpp:23325-23334`. Lifetime 200
        // (`EffectTypes.cpp:417`).
        EffectId::Baby => Box::new(FeatherBurst::new("effect\\fire_ivy.bmp", 200)),
        // `case EF_SOULBREAKER2:` - `RagEffect.cpp:24333-24347`. Lifetime 300
        // (`EffectTypes.cpp:418`).
        EffectId::Soulbreaker2 => Box::new(SoulBreaker2::new(300)),
        // `case EF_GUARD2:` (`OnProcess3`) - `RagEffect.cpp:21646-21658`.
        // Lifetime 200 (`EffectTypes.cpp:505`).
        EffectId::Guard2 => Box::new(Guard2::new(200)),
        // `case EF_RUWACH:` - `RagEffect.cpp:23193-23232`. Lifetime 200
        // (`EffectTypes.cpp:42`). The Acolyte reveal aura the actor re-launches
        // from `OPTION_RUWACH` (`GameActor.cpp:704-714`).
        EffectId::Ruwach => Box::new(Ruwach),
        // ------------------------------------------------------------------
        // Round 103: the last three backlog ids plus the SMA launcher children.
        // ------------------------------------------------------------------
        // `case EF_SMA2:` - `RagEffect.cpp:21754-21755`:
        // `TexturedBlueRing("effect\\ring_blue.tga")` (`:16329-16395`).
        // Lifetime 100 (`EffectTypes.cpp:561`).
        EffectId::Sma2 => Box::new(TexturedBlueRing::new("effect\\ring_blue.tga", 100)),
        // `case EF_SMA_BLACK2:` - `RagEffect.cpp:22240-22241`:
        // `BlueRingStack()` (`:5829-5877`). Lifetime 100
        // (`EffectTypes.cpp:1214`).
        EffectId::SmaBlack2 => Box::new(BlueRingStack::new(100)),
        // `case EF_STIN3:` - `RagEffect.cpp:21744-21750`: frame-0 energy wave
        // plus `SMA(1)`. Lifetime 200 (`EffectTypes.cpp:564`).
        EffectId::Stin3 => Box::new(Stin3::new(200)),
        // `case EF_KOUENKA:` - `RagEffect.cpp:21180-21185`:
        // `SakuraFireArrow()` + two `TexturedQuadSpin(..., index, 2)`.
        // Lifetime 300 (`EffectTypes.cpp:627`).
        EffectId::Kouenka => Box::new(Kouenka::new(300)),
        // `case EF_BEGINSPELL6:` - `RagEffect.cpp:23584-23591`: the default
        // cast-start rings + `effect\EF_BeginSpell.wav` of every skill whose
        // `CSession::GetSkillActionInfo` leaves `effectId` at 58
        // (`SkillActionInfo.cpp:51-53`, `:1750`), launched by
        // `LaunchSkillActorEffect` (`GameModePackets.cpp:17551-17559`).
        // Lifetime 40 (`EffectTypes.cpp:67`); the shared
        // `TexturedQuadRingSized` helper raises it to 56 on frame 0.
        EffectId::Beginspell6 => Box::new(BeginSpellRings::new(40)),
        // `case EF_BEGINSPELL_N6:` - `RagEffect.cpp:19925-19926`:
        // `BeginSpellSwirl(45, "", 6)` (`:9548-9650`). Lifetime 100
        // (`EffectTypes.cpp:852`); the body raises it to 120 on frame 0.
        EffectId::BeginspellN6 => Box::new(BeginSpellSwirl::new(6, 100)),
        _ => return None,
    })
}

/// Bodies that legitimately produce **no** geometry, so the
/// `every_registered_body_assigns_its_official_texture` test knows it is not
/// looking at a porting omission.
///
/// `EF_NAPALMVALCAN` is the only one: its `Hit2(0)` calls happen on frames
/// 20-60, and `Hit2` is gated on `stateCnt == 0`, so the original never spawns
/// a shard (`RagEffect.cpp:23293-23301`, `:807-812`). The body is sound-only.
pub fn spawns_no_geometry(effect: EffectId) -> bool {
    matches!(effect, EffectId::Napalmvalcan | EffectId::Jumpkick)
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
    ids.extend_from_slice(super::registry_batch_i::ids());
    ids.extend_from_slice(super::registry_batch_j::ids());
    ids.extend_from_slice(super::registry_batch_k::ids());
    ids.extend_from_slice(super::registry_batch_l::ids());

    ids.extend_from_slice(&[
        EffectId::Hit2,
        EffectId::Endure,
        EffectId::Blessing,
        EffectId::Fireball,
        EffectId::Firearrow,
        EffectId::Icearrow,
        EffectId::Chemical3,
        EffectId::Chemical4,
        EffectId::Providence,
        EffectId::Jumpkick,
        EffectId::Soullink,
        EffectId::Cartter,
        EffectId::Stin,
        EffectId::Stin2,
        EffectId::Stin4,
        EffectId::Stin5,
        EffectId::Spherewind2,
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
        EffectId::Grandcross,
        EffectId::Grandcross2,
        EffectId::Teihit1x,
        EffectId::Teihit2,
        EffectId::Teihit3,
        EffectId::Teihit1,
        EffectId::Teihit1t,
        EffectId::Backstap,
        EffectId::BottomEternalchaos,
        EffectId::BottomSiegfried,
        EffectId::Linelink2,
        EffectId::Linelink3,
        EffectId::Chemicalprotection,
        EffectId::Firstaid,
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
        EffectId::Waterball2,
        EffectId::Blooddrain,
        EffectId::Energydrain,
        EffectId::Energydrain2,
        EffectId::Energydrain3,
        EffectId::EnergydrainBlack,
        EffectId::Soulstrike,
        EffectId::Soulstrike2,
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
        EffectId::BottomLullaby,
        EffectId::BottomRichmankim,
        EffectId::BottomDrumbattlefield,
        EffectId::BottomRingnibelungen,
        EffectId::BottomRokisweil,
        EffectId::BottomIntoabyss,
        EffectId::BottomWhistle,
        EffectId::BottomPoembragi,
        EffectId::BottomAppleidun,
        EffectId::BottomHumming,
        EffectId::BottomFortunekiss,
        // Champion caster bodies (`rag/champion.rs`). The ids stay literal
        // here - instead of `ids.extend_from_slice(super::rag::champion::ids())`
        // - because `tools/gen_ported_skill_map.py` reconstructs this list by
        // parsing this exact array.
        EffectId::Lightblade,
        EffectId::Aurablade2,
        EffectId::Asurabody,
        EffectId::AsurabodyMonster,
        EffectId::Transbluebody,
        // Champion `.str` bodies (same module): `yunta_1..5.str` and
        // `연환.str`, registered in `port/str_table.rs`.
        EffectId::Comboattack1,
        EffectId::Comboattack2,
        EffectId::Comboattack3,
        EffectId::Comboattack4,
        EffectId::Comboattack5,
        EffectId::Chaincombo,
        // Champion Asura/`SuperSlashFan` bodies (`rag/asura.rs`): the primitive
        // helpers that unblocked the Champion pass (`AsuraStrike`,
        // `SuperSlashFan`, `TexturedCrossLevel`, `WhiteYellowRings`,
        // `TexturedSkillIcon`).
        EffectId::Gumgang,
        EffectId::Gumgangnpc,
        EffectId::Doublegumgang,
        EffectId::Doublegumgang2,
        EffectId::Doublegumgang3,
        EffectId::Steelbody,
        EffectId::Gumgang2,
        EffectId::Aurablade,
        EffectId::Beginasura,
        EffectId::Beginasura1,
        EffectId::Beginasura2,
        EffectId::Beginasura3,
        EffectId::Beginasura4,
        EffectId::Beginasura5,
        EffectId::Beginasura6,
        EffectId::Beginasura7,
        EffectId::Beginasura11,
        // Prim 147's only spawner, `CRagEffect::TexturedSlam`
        // (`rag/textured_slam.rs`): `EF_SECRA`/2, the `EF_ROLLING1..10` family
        // and `EF_HEARTASURA`.
        EffectId::Secra,
        EffectId::Secra2,
        EffectId::Rolling1,
        EffectId::Rolling2,
        EffectId::Rolling3,
        EffectId::Rolling4,
        EffectId::Rolling5,
        EffectId::Rolling6,
        EffectId::Rolling7,
        EffectId::Rolling8,
        EffectId::Rolling9,
        EffectId::Rolling10,
        EffectId::Heartasura,
        // Renewal cluster: the three free ids plus prims 46/69/100.
        EffectId::Hated,
        EffectId::Bluecasting,
        EffectId::Napalmvalcan,
        EffectId::Chimto,
        EffectId::Fvoice,
        EffectId::Wink,
        EffectId::Hitline4,
        EffectId::Hitline5,
        EffectId::Hitline6,
        EffectId::Tripleattack,
        EffectId::Tripleattack2,
        EffectId::Tripleattack3,
        // Round 101: pressure, guard and soulbreaker families.
        EffectId::Magnum2,
        EffectId::Soulbreaker,
        EffectId::Pressure,
        EffectId::Baby,
        EffectId::Soulbreaker2,
        EffectId::Guard2,
        // `EF_RUWACH` (33) - `RagEffect.cpp:23193-23232`, lifetime
        // `EffectTypes.cpp:42`.
        EffectId::Ruwach,
        // Round 103: the final backlog ids and the children `RenderSMA3`
        // launches (`EF_SMA2` 554, `EF_SMA_BLACK2` 1205).
        EffectId::Sma2,
        EffectId::SmaBlack2,
        EffectId::Stin3,
        EffectId::Kouenka,
        EffectId::BeginspellN6,
        // `EF_BEGINSPELL6` (58) - the default cast-start rings used by Wind
        // Walk's "on use" lane (`RagEffect.cpp:23584-23591`, lifetime
        // `EffectTypes.cpp:67`).
        EffectId::Beginspell6,
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
            (EffectId::Waterball2, 100),
            (EffectId::Blooddrain, 200),
            (EffectId::Energydrain, 200),
            (EffectId::Energydrain2, 200),
            (EffectId::Energydrain3, 200),
            (EffectId::EnergydrainBlack, 200),
            (EffectId::Soulstrike, 100),
            (EffectId::Soulstrike2, 100),
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
            (EffectId::Sma2, 100),
            (EffectId::SmaBlack2, 100),
            (EffectId::Stin3, 200),
            (EffectId::Kouenka, 300),
            (EffectId::BeginspellN6, 100),
            // `EF_BEGINSPELL6` - `EffectTypes.cpp:67` (raised to 56 by the
            // ring helper on frame 0).
            (EffectId::Beginspell6, 40),
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
            // `Bottom_Music` / `Bottom_Out` family (`rag/bottom_music.rs`):
            // `EffectTypes.cpp:287-302`.
            (EffectId::BottomLullaby, 29999),
            (EffectId::BottomRichmankim, 29999),
            (EffectId::BottomDrumbattlefield, 29999),
            (EffectId::BottomRingnibelungen, 29999),
            (EffectId::BottomRokisweil, 29999),
            (EffectId::BottomIntoabyss, 29999),
            (EffectId::BottomWhistle, 29999),
            (EffectId::BottomPoembragi, 29999),
            (EffectId::BottomAppleidun, 29999),
            (EffectId::BottomHumming, 29999),
            (EffectId::BottomFortunekiss, 29999),
            // Champion fan family (`rag/asura.rs`): `EffectTypes.cpp:427`,
            // `:464`, `:494-495`.
            (EffectId::Gumgangnpc, 1500),
            (EffectId::Doublegumgang, 99_999_999),
            (EffectId::Doublegumgang2, 99_999),
            (EffectId::Doublegumgang3, 99_999),
            // Prim 147's `TexturedSlam` family (`rag/textured_slam.rs`):
            // `EffectTypes.cpp:767-776`, `:868`, `:871`, `:888`.
            (EffectId::Secra, 300),
            (EffectId::Secra2, 300),
            (EffectId::Rolling1, 99_999_999),
            (EffectId::Rolling10, 99_999_999),
            (EffectId::Heartasura, 99_999_999),
            // Grand Cross and slash-hit families (`rag/grand_cross.rs`,
            // `rag/textured_scatter.rs`, `rag/tei_hit2.rs`):
            // `EffectTypes.cpp:235`, `:459`, `:273`, `:275`, `:284`.
            (EffectId::Grandcross, 999),
            (EffectId::Grandcross2, 999),
            (EffectId::Teihit2, 300),
            (EffectId::Teihit1x, 300),
            (EffectId::Teihit3, 300),
            (EffectId::Backstap, 300),
            // Round 100 (`rag/textured_glow.rs`, `rag/line_link.rs`,
            // `rag/chemical_protection.rs`, `rag/first_aid.rs`,
            // `rag/textured_scatter.rs`): `EffectTypes.cpp:271`, `:898`,
            // `:289`, `:294`, `:393`, `:404`, `:309`, `:318`.
            (EffectId::Teihit1, 300),
            (EffectId::Teihit1t, 300),
            (EffectId::BottomEternalchaos, 29999),
            (EffectId::BottomSiegfried, 29999),
            (EffectId::Linelink2, 200),
            (EffectId::Linelink3, 99_999_999),
            (EffectId::Chemicalprotection, 300),
            // `EF_CHEMICAL3` / `EF_CHEMICAL4` - `EffectTypes.cpp:448`, `:632`.
            (EffectId::Chemical3, 300),
            (EffectId::Chemical4, 300),
            // `EF_PROVIDENCE` - `EffectTypes.cpp:257`.
            (EffectId::Providence, 200),
            // `EF_JUMPKICK` - `EffectTypes.cpp:466`.
            (EffectId::Jumpkick, 100),
            // `EF_SOULLINK` / `EF_CARTTER` - `EffectTypes.cpp:512`, `:527`.
            (EffectId::Soullink, 500),
            (EffectId::Cartter, 200),
            // `EF_STIN` family - `EffectTypes.cpp:556`, `:562`, `:630`, `:633`.
            (EffectId::Stin, 200),
            (EffectId::Stin2, 200),
            (EffectId::Stin4, 200),
            (EffectId::Stin5, 200),
            // `EF_SPHEREWIND2` - `EffectTypes.cpp:403`.
            (EffectId::Spherewind2, 99_999),
            (EffectId::Firstaid, 299),
            // Round 101: `EffectTypes.cpp:348`, `:370`, `:374`, `:417`, `:418`,
            // `:505`.
            (EffectId::Magnum2, 100),
            (EffectId::Soulbreaker, 300),
            (EffectId::Pressure, 300),
            (EffectId::Baby, 200),
            (EffectId::Soulbreaker2, 300),
            (EffectId::Guard2, 200),
            // `EF_RUWACH` - `EffectTypes.cpp:42`.
            (EffectId::Ruwach, 200),
        ];

        for (effect, lifetime) in expect {
            let body = spawn(effect).unwrap();
            assert_eq!(body.lifetime(), lifetime, "{effect:?}");
        }
    }

    #[test]
    fn renewal_cluster_lifetimes_match_effect_types() {
        let expect = [
            (EffectId::Hated, 500),
            (EffectId::Bluecasting, 60),
            (EffectId::Napalmvalcan, 200),
            (EffectId::Chimto, 16),
            (EffectId::Fvoice, 100),
            (EffectId::Wink, 100),
            (EffectId::Hitline4, 200),
            (EffectId::Hitline5, 200),
            (EffectId::Hitline6, 200),
            (EffectId::Tripleattack, 100),
            (EffectId::Tripleattack2, 200),
            (EffectId::Tripleattack3, 200),
        ];

        for (effect, lifetime) in expect {
            let body = spawn(effect).unwrap_or_else(|| panic!("{effect:?} does not spawn"));
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

    /// `EF_PORTAL4` (389) is Wind Walk's success lane. The registration must
    /// carry every official layer of `case EF_PORTAL4:`
    /// (`RagEffect.cpp:23300-23311`): the `Cloud11Burst(0)` wind columns, the
    /// inline wave (a *single* backslash archive path - the earlier double
    /// backslash never resolved a file) and the caster tint window.
    #[test]
    fn portal4_registration_carries_the_wind_walk_layers() {
        use crate::world::effect::port::primitive::PrimitiveId;
        use crate::world::effect::port::runtime::{EffectRuntime, step};

        let mut body = spawn(EffectId::Portal4).expect("EF_PORTAL4 must stay registered");
        assert_eq!(body.lifetime(), 200);

        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 171);
        // Frames 0..=5: frame 0 spawns the columns + wave, frame 5 opens the
        // colour window.
        for _ in 0..=5 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::Wind);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\cloud11.tga"));

        assert_eq!(
            runtime.sounds,
            vec!["effect\\윈드워크.wav".to_owned()],
            "the registry path must use the archive's single backslash"
        );
        assert_eq!(runtime.parent_color, Some([-1, 220, 250, 220]));
        assert_eq!(runtime.parent_flags & 1, 1);
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
                        || super::super::registry_batch_h::is_caster_body_only(effect)
                        || super::super::registry_batch_i::is_caster_body_only(effect)
                        || super::super::rag::champion::is_caster_body_only(effect)
                        || super::super::registry_batch_j::is_caster_body_only(effect)
                        || super::super::registry_batch_k::is_caster_body_only(effect)
                        || super::super::registry_batch_l::is_caster_body_only(effect)
                        || super::super::registry_batch_l::spawns_particles_only(effect)
                        || spawns_no_geometry(effect),
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
                if primitive.state.sprite.is_some()
                    || matches!(
                        primitive.state.count,
                        PrimitiveId::Particle3d | PrimitiveId::Orbit | PrimitiveId::Sma
                    )
                {
                    // `PP_SMA` is the textureless launcher
                    // (`totalTexture = 0`, `RenderSMA3`); its children carry
                    // the textures.
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
