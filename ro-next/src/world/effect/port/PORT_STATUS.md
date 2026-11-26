# Effect port status

Live tracker for the 1:1 port. Update this file whenever a unit lands.
Source of truth for the plan: `default-workspace/ro-next-fx-audit/PLAN.md`.

## Landed

| unit | source | tests |
| --- | --- | --- |
| 16 ms effect clock, catch-up, `g_updateTick` | `Client/Application/Timer.cpp` | 4 |
| trig tables incl. 361 offset for negative angles | `Utility/Math/ClientAngles.cpp` | 3 |
| `EffectPrimitiveState` + official constructor defaults | `EffectPrimitives/EffectPrim.cpp:8-150` | - |
| `ProcessAlpha` / `LimitAlpha` | `EffectPrimitiveColor.cpp` | 2 |
| `PP_2DCIRCLE` motion + render | `EffectPrimitiveMotion.cpp:8149`, `EffectPrimitiveRendering.cpp:5649` | 4 |
| `PP_2DFLASH` motion + render (incl. swapped corner quirk) | `EffectPrimitiveMotion.cpp:8121` | 3 |
| per-frame runtime (body -> ++stateCnt -> primitive pass) | `Effect.cpp`, `EffectPrim.cpp:155` | 3 |
| `EffectRng` = `ClampedRandom` | `RagEffect.cpp:168` | 1 |
| `CRagEffect::Bash` (`EF_BASH`) | `RagEffect.cpp:1601-1670` | 5 |
| `ProcessEZ2STR` + `InitEZ2STRFrame` | `RagEffect.cpp:1386-1523` | 5 |
| 12-float matrix, `Forward`, `MatrixMult` | `AffineMatrix.cpp`, `EffectPrimitiveMotion.cpp:76` | 4 |
| `PP_3DCIRCLE` + segment records (`InitSegment`/`ShiftSegment`) | `EffectPrimitiveMotion.cpp:791`, `EffectPrimitiveRecords.cpp:9` | 4 |
| `PP_3DSPHERE` | `EffectPrimitiveMotion.cpp:8225` | - |
| `CRagEffect::MagnumBreak` (`EF_MAGNUMBREAK`) | `RagEffect.cpp:9799-9849` | 4 |
| `EF_SONICBLOW` inline body | `RagEffect.cpp:23801-23821` | 3 |
| `CRagEffect::CartRevolution` (`EF_CARTREVOLUTION`) | `RagEffect.cpp:2234-2273` | 4 |
| `CRagEffect::Signum` (`EF_SIGNUM`) | `RagEffect.cpp:2107-2154` | 2 |
| `CRagEffect::StoneCurseRing` (`EF_OVERTHRUST`) | `RagEffect.cpp:10441-10470` | 2 |
| `PP_2DTEXTURE` (`Move2DPrimitive`) | `EffectPrimitiveMotion.cpp:7719` | 4 |
| `CRagEffect::Endure` (`EF_ENDURE`) | `RagEffect.cpp:224-284` | 2 |
| `PP_3DQUADHORN` (`AdvanceEffectPrimitive`) | `EffectPrimitiveMotion.cpp:543` | 3 |
| effect facing/distance (`Face`, `CalcDist`, `RotatedForward`) | `ActorGeometry.cpp:47`, `RagEffect.cpp:136` | - |
| `CRagEffect::FrostDiver` (`EF_FROSTDIVER`/`2`/`3`) | `RagEffect.cpp:456-529` | 3 |
| `CRagEffect::IceWall` (`EF_ICEWALL`) | `RagEffect.cpp:587-628` | 2 |
| `CRagEffect::EarthSpike` (`EF_EARTHSPIKE`, earth/ice variants) | `RagEffect.cpp:11213-11287` | 2 |
| `PP_3DTEXTURE` / `PP_3DCROSSTEXTURE` (`MoveEffectPrimitive`) | `EffectPrimitiveMotion.cpp:122-343` | 3 |
| `CRagEffect::IncAgility` / `IncAgiDex` (`EF_INCAGILITY`/`EF_INCAGIDEX`) | `RagEffect.cpp:679-757` | 3 |
| `PP_3DCYLINDER` (`Prim3DCylinder`) | `EffectPrimitiveMotion.cpp:437-488` | 2 |
| `CRagEffect::BowlingBash` (`EF_BOWLINGBASH`) | `RagEffect.cpp:2277-2321` | 2 |
| `CRagEffect::Hit2` (`EF_HIT2` + physical impacts) | `RagEffect.cpp:807-859` | 2 |
| `CRagEffect::FrostDiver2` (`EF_FROSTDIVER2`) | `RagEffect.cpp:414-452` | 1 |
| effect-id registry (`EffectId -> body`) | plan 4.2 | 4 |
| `PP_3DPARTICLE` + `AnimMotion` | `EffectPrimitiveMotion.cpp:2488`, `:576` | 4 |
| `CRagEffect::Blessing` (`EF_BLESSING`) | `RagEffect.cpp:1612-1720` | 2 |
| texture assets on every primitive (`g_texMgr.Load`) | all ported bodies | 2 |
| `render_flag` (`RSF_*`, additive 0x200) | primitive state + ring bodies | 2 |
| `PortedEffectHolder` (fixed 16 ms stepping) | plan 4.2 | 6 |
| holder render path (`Texture2d`/`Flash2d` quads via `EffectRenderer`) | plan W3 | 3 |
| `CRagEffect::FireBall` (`EF_FIREBALL`) | `RagEffect.cpp:325-367` | 2 |
| `CRagEffect::DecAgility` (`EF_DECAGILITY`) | `RagEffect.cpp:15222-15279` | 2 |
| `EF_REVIVE` | `RagEffect.cpp:24061-24090` | 2 |
| `EF_AQUA` | `RagEffect.cpp:23363-23375` | 1 |
| `CRagEffect::PoisonAttack` (`EF_ENCHANTPOISON`) | `RagEffect.cpp:13276-13338` | 2 |
| `CRagEffect::IncAgilityCenter` (`EF_AGIUP`) | `RagEffect.cpp:16534-16582` | 2 |
| `CRagEffect::Detoxication` (`EF_DETOXICATION`) | `RagEffect.cpp:13187-13211` | 2 |
| `CRagEffect::TexturedClassChange` - 84-kind sprite table behind ~80 effect ids | `RagEffect.cpp:18730-19370` | 5 |
| `registry_class_change.rs` - 66 generated effect-id entries with waves/quake | dispatch switch | 2 |
| primitive slots (`EffectPrimitiveSlot` x4) | `EffectPrimitiveState.h` | - |
| `PP_WIND` (`PrimWind`) | `EffectPrimitiveMotion.cpp:4415-4497` | 4 |
| `CRagEffect::Cloud11Burst` (`EF_PORTAL4`, `EF_PORTAL5`) | `RagEffect.cpp:16089-16157` | 4 |
| `PP_INTIMIDATE` (`PrimIntimidate`) | `EffectPrimitiveMotion.cpp:1257-1313` | 4 |
| `CRagEffect::WhiteSpray` (`EF_INTIMIDATE`/2/3/4) | `RagEffect.cpp:5874-5921` | 4 |
| prim 82 `PrimAuraGlow` + slot vectors | `EffectPrimitiveMotion.cpp:6837-6946` | 4 |
| `CRagEffect::TexturedAura` (kinds 0-22, parent sprite lift) + 26 aura effects | `RagEffect.cpp:17769-17960` | 6 |
| `CRagEffect::TexturedSkillAura` (`EF_DETECTING`, `EF_DETECT2/3/4`) | `RagEffect.cpp:15281-15370` | 3 |
| prim 137 `PrimHeightRamp` | `EffectPrimitiveMotion.cpp:4970-5008` | 4 |
| `CRagEffect::TexturedSpreadFlat` (`EF_REDLINE`, `EF_BLUELINE`) | `RagEffect.cpp:6986-7070` | 2 |
| prim 89 spin-quad | `EffectPrimitiveMotion.cpp:9817-9859` | 4 |
| `CRagEffect::TexturedQuadSpin` (`EF_BASH3D5/6`) | `RagEffect.cpp:3617-3653` | 3 |
| prim 59 `PrimParticleUp` | `EffectPrimitiveMotion.cpp:3692-3738` | 2 |
| prim 60 `PrimPulsingColumns` (heal/fire columns) | `EffectPrimitiveMotion.cpp:6541-6650` | 4 |
| `CRagEffect::HealBurst` (`EF_HEAL`, `EF_HEAL3`) | `RagEffect.cpp:18215-18362` | 3 |
| `CRagEffect::HealRingSparkle` (`EF_HEAL2`) | `RagEffect.cpp:17970-18186` | 2 |
| prim 63 `PrimRippleRings` | `EffectPrimitiveMotion.cpp:6638-6739` | 4 |
| `CRagEffect::VioletRingCloud` (`EF_PORTAL2`, `EF_PORTAL3`) | `RagEffect.cpp:15364-15577` | 2 |
| `CRagEffect::TexturedMagicRing` (`EF_BIG_PORTAL`, `EF_BIG_PORTAL2`) | `RagEffect.cpp:15565-15703` | 2 |
| `CRagEffect::StormGust` (`EF_STORMGUST`) | `RagEffect.cpp:15176-15215` | 2 |
| `CRagEffect::RocketPunch` / `GreenRocketPunch` (`EF_FIREBALL3/6`) | `RagEffect.cpp:8079-8185` | 3 |
| `CRagEffect::ThunderSparks` (`EF_SPRINKLESAND`) | `RagEffect.cpp:5430-5465` | 2 |
| `PotionPillar` + `Potion_Berserk` / `Potion_Awakening` (`EF_POTION_BERSERK/2`) | `RagEffect.cpp:2459-2520`, `15045-15087` | 3 |
| `RingStack` + `EF_FIREPILLARON2` | `RagEffect.cpp:2861-2882`, `20741-20746` | 1 |
| `CRagEffect::TexturedTripleOrbit` (`EF_GRADIUS_LASER`) | `RagEffect.cpp:7123-7183` | 1 |
| `CRagEffect::TexturedSprayKind` / `TexturedSpray` helpers | `RagEffect.cpp:4642-4689`, `4064-4097` | 2 |
| Rogue coin family (`EF_RG_COIN`, `EF_RG_COIN2..7`) - data table | `RagEffect.cpp:20997-21100` | 4 |
| `CRagEffect::SnowFall` | `RagEffect.cpp:6615-6663` | 2 |
| prims 130/131 `PrimRisingFan` / `PrimRisingFan2` + `RaiseFanHeights` | `EffectPrimitiveMotion.cpp:6314-6479` | 4 |
| `CRagEffect::SaintWing` + `EF_FROSTMYSTY` | `RagEffect.cpp:5092-5119`, `20682-20689` | 2 |
| `CRagEffect::PortalRings` (`EF_ENTRY2`) | `RagEffect.cpp:17145-17250` | 1 |
| `CRagEffect::WhiteRingWave` / `BlueRingWave` (`EF_READYPORTAL2`) | `RagEffect.cpp:13637-13700`, `14036-14102` | 2 |
| `CRagEffect::WaterOut` (`EF_WATERBALL`, three textures) | `RagEffect.cpp:3537-3564` | 1 |
| `CRagEffect::SpearBmr` (`EF_SPEARBMR`) + `Get8Dir2` | `RagEffect.cpp:1091-1131`, `GameObject.cpp:188-213` | 2 |
| primitive `render.angle` (own facing) | `EffectPrimitiveState` | - |
| `.str` support in the runtime (`open_str`/`process_str`) | `ProcessEZ2STR` call sites | - |

Total: 123 tests, all green.

**Registry:** 157 effect ids (66 generated from `TexturedClassChange`, 26 from
`TexturedAura`, 65 hand-written bodies). Of the 341 distinct effect ids referenced by the pre-renewal skill tables
(official + 2019 client ids), **33 are ported (9.7%)** - but they are the common
classic ones (Bash, Magnum, Frost Diver, Endure, Blessing, Fire Ball, Sight...),
so the *skill* coverage is far higher than the id ratio suggests.

## Next (W2/W3/W6)

1. `PP_3DTEXTURE` (`MoveEffectPrimitive`) - the 3-D textured sprite family; needs
   the camera/terrain context that `MoveEffectPrimitive` reads.
2. `PP_3DPARTICLE` - sprite particles, unblocks Blessing and the buff family.
2. Wire `str_player` into `EffectWithLight`: replace the interpolating
   `FrameTimer` path and the buggy key map in `loaders/effect/mod.rs`.
3. `EF_SIGNUM` and `EF_OVERTHRUST` (both are now expressible with the primitives
   and the `.str` runtime support).
4. `effect_backend.ron` + per-skill switch (plan 4.2) once three skills run
   both ways and the frame diff matches.

## Deliberate deviations (keep this list short)

* `EffectRng` is xorshift64\*, not the MSVC LCG: same distribution and ranges,
  different sequence. Every client sees different randomness anyway; tests need
  determinism.
* `Move2DPrimitive`'s `slots[0].life == 98` branch drives the skill-cooldown
  overlay (`g_activeSkillEffects`), not a world visual; it is not ported.
* `.str` animation type 4 keeps a persistent `ratio`; the original reads an
  uninitialised local (`RagEffect.cpp:1519-1522`). Undefined behaviour cannot be
  reproduced.
