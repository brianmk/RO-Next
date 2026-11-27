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
| `CRagEffect::EnchantPoison2` (`EF_EDP`) | `RagEffect.cpp:13241-13272` | 1 |
| `CRagEffect::FireSparks` (`EF_FIREFLY`) with colour-cycle track | `RagEffect.cpp:371-410` | 1 |
| prim 39 halo + `CRagEffect::TexturedHalo` (4 bottom effects) | `EffectPrimitiveMotion.cpp:9651-9688`, `RagEffect.cpp:4839-4896` | 5 |
| prim 143 `PrimTextureSpin` | `EffectPrimitiveMotion.cpp:4759-4792` | 4 |
| `CRagEffect::TorchDustRing` (`EF_WATER_BELOW`) + `texture_frames`/`frame_count` | `RagEffect.cpp:3241-3310` | 3 |
| `WhiteRingRipple` + bottom composites (`EF_BOTTOM_MANHOLE/ANI/BLOODYLUST`) | `RagEffect.cpp:6839-6908`, `19940-19990` | 2 |
| `CRagEffect::ForestLight` (prim 66, spawn-only) + `EF_FORESTLIGHT6` | `RagEffect.cpp:14597-14664`, `19940-19950` | 4 |
| `CRagEffect::TexturedPulseKind` (prim 64, spawn-only) + `EF_BOTTOM_MAG/SANC` | `RagEffect.cpp:11135-11207` | 2 |
| `CRagEffect::Portal` (`EF_PORTAL`) | `RagEffect.cpp:11024-11071` | 2 |
| `CRagEffect::Bottom` / `Bottom2` (`EF_BOTTOM`, `EF_BOTTOM2`) | `RagEffect.cpp:1718-1820` | 2 |
| `CRagEffect::GlassWall` (`EF_GLASSWALL`) | `RagEffect.cpp:10970-11020` | 2 |
| `CRagEffect::WaterBallBurst` (`EF_WATERBALL3`) | `RagEffect.cpp:3484-3533` | 1 |
| `CRagEffect::GrimTooth` (`EF_GRIMTOOTH`) | `RagEffect.cpp:531-583` | 2 |
| `CRagEffect::GrimToothAtk` (`EF_GRIMTOOTHATK`) | `RagEffect.cpp:2050-2091` | 1 |
| `CRagEffect::StoneSpike` (`EF_HEAVENSDRIVE`) | `RagEffect.cpp:13342-13385` | 1 |
| `CRagEffect::NapalmFragments` (`EF_NAPALMBEAT`, 8-frame explosion) | `RagEffect.cpp:10018-10058` | 1 |
| `CRagEffect::TexturedComet` (`EF_SMA3`) | `RagEffect.cpp:5926-5978` | 2 |
| `CRagEffect::BlitzBeat` (`EF_BLITZBEAT`, count-driven bolts) | `RagEffect.cpp:11631-11710` | 3 |
| `CRagEffect::SprayPond` (`EF_SPRAYPOND`) | `RagEffect.cpp:1135-1186` | 2 |
| `CRagEffect::Entry` (`EF_ENTRY`) | `RagEffect.cpp:2191-2230` | 1 |
| `CRagEffect::Vallentine` (5 ids: VALLENTINE/2, 05VAL, ITEMFAST, RO2YEAR) | `RagEffect.cpp:8741-8779` | 3 |
| `CRagEffect::SonicBlowHit` (`EF_SONICBLOWHIT`) | `RagEffect.cpp:2324-2364` | 1 |
| prim 7 `PP_3DPARTICLEORBIT` | `EffectPrimitiveMotion.cpp:2936-2985` | 2 |
| `CRagEffect::Exit` (`EF_EXIT`) | `RagEffect.cpp:2368-2418` | 2 |
| `CRagEffect::Warp` (`EF_WARP`) | `RagEffect.cpp:2414-2463` | 2 |
| `CRagEffect::TexturedRisingOrbs` (`EF_HPTIME`, `EF_SPTIME`) | `RagEffect.cpp:7187-7265` | 3 |
| `CRagEffect::LockOnMarker` (`EF_LOCKON`) | `RagEffect.cpp:9740-9795` | 1 |
| `CRagEffect::TeleportSpiral` (`EF_EXIT2`) | `RagEffect.cpp:17683-17765` | 1 |
| `CRagEffect::Aspersio` (`EF_ASPERSIO`, holy water orbit) | `RagEffect.cpp:1568-1597` | 2 |
| `CRagEffect::Sphere` (`EF_SPHERE`) + orbit `chRadiSpeed` cycle | `RagEffect.cpp:2017-2046` | 1 |
| `CRagEffect::RemoveTrap` (`EF_REMOVETRAP`) | `RagEffect.cpp:1986-2013` | 1 |
| prim 6 `Prim3DParticleGravity` + `CRagEffect::Steal` (`EF_STEAL`) | `EffectPrimitiveMotion.cpp:2581-2619`, `RagEffect.cpp:1835-1875` | 9 |
| prim 155 `PrimWanderingFlames` + `CRagEffect::TexturedForkBolt` (ported, unregistered) | `EffectPrimitiveMotion.cpp:7353-7403`, `RagEffect.cpp:7268-7321` | 12 |
| prim 96 `PrimPeongUp` + `CRagEffect::WhiteLightTrail`, Storm Kick 4/5 | `EffectPrimitiveMotion.cpp:3744-3778`, `RagEffect.cpp:13451-13516`, `:23703-23722` | 11 |
| `CRagEffect` metallic-sound line (`EF_NEW_METALLICSOUND_LINE`, 1584) | `RagEffect.cpp:22532-22544` | 2 |
| `ParticleTime` (`EF_SMA_READY`), `StoneWall` (`EF_EARTHWALL`), `Hit3` (`EF_HIT3`) | `RagEffect.cpp:12121-12177`, `13088-13124`, `863-935` | 9 |
| **client wiring**: `port/visual.rs` (`PortedVisual` over `EffectBase`), `port/skill_map.rs` (45 skill->id mappings), opt-in hook in `client/effects.rs` | `tools/gen_ported_skill_map.py`, `RO_NEXT_PORTED_FX=1` | 10 |
| **world-space rendering**: `port/render3d.rs` for `PP_3DTEXTURE` (15) and `PP_3DCROSSTEXTURE` (16) via `render_effect_with_depth`, official corner order/UVs/pattern bits, frame selection | `EffectPrimitiveRendering.cpp:116`, `:6172` | 19 |
| **slot/record rendering**: `port/render_slots.rs` ports `Render3DCasting` (60/63), `RenderRecordRings` (12), `RenderArcBand` (13), `RenderFieldPanel` (59), `RenderPyramid` (9), `RenderArcWalls` (137), `RenderFacingQuads` (130), `RenderRipple` (131), `RenderSizedSquare` (39), `RenderRiseArc` (62), `RenderGroundSquare` (143), `RenderTwinGlowSquares` (155), `RenderOrbitSquare` (43), `RenderRectUp` (64), `RenderPentagon` (66), `RenderFanBeam` (89); additive triangle path in `graphics/passes/forward/effect.rs` + `effect_triangle.slang` | `EffectPrimitiveRendering.cpp:1667-6675`, `EffectPrimitiveArcs.cpp:437`, `EffectPrimitiveSweeps.cpp:421`, `EffectPrimitiveGlowSquares.cpp:7` | 27 |

### Rendering coverage

| primitive family | draws in client |
| --- | --- |
| `PP_2DTEXTURE` (0), `PP_2DFLASH` (3) | yes (screen quads) |
| `PP_3DTEXTURE` (15), `PP_3DCROSSTEXTURE` (16) | yes (world quads, per-corner depth) |
| sprite particles (5/6/7) and every `.spr` body (incl. the 66 `TexturedClassChange` ids) | yes - `port/sprite_pass.rs` ports `RenderSegmentSprites2`, `AdjustCellPosition`, the 1/7 world scale, frame selection and blend flags |
| slot/record prims 9, 12, 13, 39, 43, 59, 60, 62, 63, 64, 66, 89, 130, 131, 137, 143, 155 | yes - `port/render_slots.rs` ports their official `Render*` members as additive triangle lists (per-vertex projection, exact corner order, UV windows, `state.size` colour table, `extra` forced depth and `RSF_*` blend selection) |
| slot prim 82 (`RenderTeiFacingSlots`) and the remaining `Render*` families | no - not ported yet (82 also needs the `extra == 3` screen-space clamp of the 18-argument `RenderTeiRect`) |

Triangle pass: `EffectTriangleInstruction` + `EffectTriangleVertex` (pre-projected NDC corners, per-vertex depth, texture, blend), an additive `ForwardEffectDrawer` triangle pipeline + `effect_triangle.slang`; quads untouched, triangles drawn after them.

Prim 82 note: the switch is on `slots[0].flag1[2]` (render kind), **not** `flag1[4]` (motion kind) - `EffectPrimitiveRendering.cpp:7853`; using the motion kind would make the status auras (kind 22 -> renderer kind 16) emit nothing. `state.size` is not read by this primitive.

Documented render-side deviations (simulation untouched): `RenderFieldPanel` map-name alpha clamp skipped; `RenderPyramid`'s `std::rand` colour rewrite replaced by a deterministic hash (render is `&self`); `RenderRectUp`'s uninitialised colour falls back to case 0; the 510-triangle cap counts emitted triangles; render-time scratch writes to slot vectors are not written back; `RenderTeiRect` `extra` 2/3 not implemented (no ported body uses them).

`.str` clips: `port/str_table.rs` resolves the official resource per id (`Cross.str`, `CartRevolution.str`, `SafetyWall.str`, `StormGust.str`, `Aspersio.str`, `버서크.str`; `RagEffectInitialization.cpp:45-248`, `RagEffect.cpp:108-109` for the default `loopLimit = 0`), loads them through the client's `EffectData` reader and opens the player at spawn; `EF_SIGNUM` and `EF_CARTREVOLUTION` are now spawnable. `port/str_render.rs` rasterizes them: `SetAniClipCorners` ported branch-for-branch (`RagEffect.cpp:14658-14695`), triangle order `[v0,v1,v2,v3]` with UVs `[v3,v1,v0,v2]`, `((x-320)+rx)*scale + tlvertX`, `rz/2.8444` degrees, `cr*` bytes, `(int)aniframe` texture selection; textures threaded through `str_table::ClipSet`. Official quirk preserved: `CRagEffect::Render` starts at **layer 1** (`RagEffect.cpp:14724`), so a single-clip resource draws nothing - exactly like the original. Draws: Signum, Cart Revolution, Storm Gust, Glass Wall, Aspersio, Potion Berserk/2.

Remaining `.str` gap: per-keyframe `srcalpha`/`destalpha`/`mtpreset` are parsed but not carried, so the pass uses the `RSF_ALPHA` pair; screen-shake branch not modelled; the `-80` screen offset is a constant (all ported call sites use it).

Known sprite-pass gaps (documented, not guessed): `RSF_TOPMOST` (0x8) blend state is undefined in the decompilation (no body sets it); `sprite_type > 1`/third image index is an out-of-bounds read in the original; class-change direction rows are absent from the ported `ClassChange` table (those cells draw row 0); per-corner DX9 `rhw` depth is replaced by the anchor depth.
| primitive `render.angle` (own facing) | `EffectPrimitiveState` | - |
| `.str` support in the runtime (`open_str`/`process_str`) | `ProcessEZ2STR` call sites | - |

Total: 123 tests, all green.

**Registry:** 488 unique effect ids (66 generated from `TexturedClassChange`, 26 from
`TexturedAura`, 121 hand-written bodies incl. the c/d batches).
`TexturedAura`, 105 hand-written bodies). Of the 341 distinct effect ids referenced by the pre-renewal skill tables
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

### Client wiring fixed

- `spawn_skill_effect_phase` no longer returns early on an empty descriptor
  layer list; a phase now spawns its ported body when `should_spawn_skill_phase`
  says so, so descriptor-less skills (HT_BLITZBEAT = 129) actually fire. The
  generated map falls back to the 2019 client table for those skills
  (`SkillId(129) Success -> Blitzbeat`, anchor Target).
- Falcon flight: `world/entity/falcon.rs` ports the `AM_REQ_ATTACK` state
  machine (25-tick dive, 10-tick strike, 25-tick return) over the perched
  `data\sprite\이팩트\매` ACT (16 actions: 0-7 perched flap, 8-15 flight); it
  triggers from `ZC_NOTIFY_SKILL` (0x01DE) for falcon skills.
- Weapon layer: armed players idle on ACT block 32..39 and attack on the
  weapon ACT's 80..87 / 88..95 block (`weapon::attack_action`), because blocks
  0..7 and 40..47 hold only `-1` placeholders in every weapon ACT.

### Client wiring fixed (round 87)

- **Effect-level `count`**: the damage/cast packets carry no skill level, so the
  spawn path reads the local player's skill tree (`local_learned_skill_level`)
  and sets `runtime.count = level` (matching `AM_SETCOUNT`,
  `JobInheritance.cpp:2147/2153`). Confirmed populated for a Blitz Beat cast
  (the spawn path is local-caster-only, which is exactly when the tree has the
  level). `runtime.param` stays `1.0` because it is the `AM_SETPARAM` maker
  parameter, not the level.
- **Positional sounds**: `EffectRuntime::positional_sounds` + `play_sound_at_height`
  (`PlayEffectWaveAt`) and `play_sound_at` (raw `PlayWave` offsets), plus
  `listener_position` snapshot. Ported: Gloria, ElementalWeapon (4 ids),
  Holy Cross, Loud, with their `str_table` rows.

### Round 88

- **Screen offsets**: `EffectRuntime::str_offset` + `process_str_with(offset)` (default `DEFAULT_STR_OFFSET = [0,-80,0]`); `str_render::render` takes the offset explicitly, so `ProcessEZ2STR(0,0,0)`/`(0,-20,0)` call sites are faithful. Unblocked Thunderstorm (30), ShieldCharge (246), Devotion (251), Lkconcentration (369), RejectSword (392).
- **Parent tint/flags**: `parent_color` + `set_parent_color` + `parent_flags`/`add_parent_flags` recorded exactly. The tint is now **applied**: `EffectHolder::update` takes the mutable entity slice (handed over by `client/frame.rs`), and the new `EffectBase::apply_caster_state`/`clear_caster_state` hooks replay `SetColor` through `Entity::set_status_tint` (with `-1` channels keeping their previous value) and clear it when the effect retires. `parent_flags` stays recorded only: `Entity` has no per-frame `render.flags` register, and bits 1/8 map to different persistent mechanisms.
- **Audio bridge** (landed round 88): `port/sound_sink.rs` drains `runtime.sounds` then `runtime.positional_sounds` after every 16 ms step and plays them through the *same* `AudioEngine::play_spatial_sound_effect` path the descriptor effects use (250 range, listener-relative offsets resolved to world positions). Not matched: `PlayWave`'s `minimumDistance = 40` (engine hardcodes 5), post-spawn listener movement, cross-channel interleaving (no shared sequence number).

### Round 89

- prim **96** `PeongUp` + `CRagEffect::WhiteLightTrail`; `EF_STORMKICK4` (462, 200) and `EF_STORMKICK5` (463, 300) registered. Storm Kick 5's `SetFrameSequence(0x60,2,2,0x60,2,1,25,0)` needed a `ParentFrameSequence` carrier applied through `Entity::set_skill_frame_sequence` (`Custom(12)` -> ACT block 96+direction).
- `EffectId::NewMetallicSoundLine` (1584) added to `ragnarok-packets`; its `TexturedBeam` + 3 `TexturedForkBolt` body registered (default `TT_MIN_EFFECT_ON_OFF` branch).
- **Parent tint is applied**, not just recorded: `EffectHolder::update` now takes `&mut [Entity]`, and `PortedVisual` replays `SetColor` into `Entity::set_status_tint` (`-1` channels keep the previous value), clearing it on retirement.
- Batch f: Sightrasher, TurnUndead, HasteUp, Dance2, Thunderstorm2.
- Constructor defaults fixed centrally: `change_speed3d`/`change_accel3d` now start at `0` per `EffectPrim.cpp:97-102`.

### Round 90

- prim **42** `PP_SHIELDBOOMERANG`, **53** `PP_THROWITEM`, **116** `PP_SHIELDBOOMERANG3` — all three reuse the shared `RenderFieldPanel` (its `flag1[2]`/`flag1[3]` switches already covered ThrowItem and TANJI), so only motion modules plus `primitive::get_angle` (`EffectPrimitiveState.cpp:51-70`) were needed. Bodies: ThrowItem, DoubleThrowItem, TexturedShieldThrow, ShieldBoomerang3, TANJI (20 ids).
- Batch g: AbsorbSpirits, Bash3D/3/4, TrueSight (TexturedQuadSpin phases), HitTexture (TexturedAura kind 2). Quirk kept: the helpers no-op after frame 0, so all geometry appears on the first frame and the port calls them every frame to preserve RNG order.
- Backlog coverage after rounds 89-90: `.str` tier 57/57, layer tier 74/144, manual tier 0/35.

### Round 91

- prim **24** `PP_3DCASTING_2` (`Casting2`, reuses `render_3d_casting`), **48** `PP_BOTTOM2` (`Bottom2`, new `render_color_plane`), **75** `PP_SLASH1` (`Slash1`, new `render_slash_1`).
- Bodies: Defender, Reflect Shield, Kaizel, Stop Effect, and the five `TexturedMagicRing` bottoms (Dissonance, AssassinCross, UglyDance, Don'tForgetMe, ServiceForYou).
- **Manual tier opened**: MagicCrasher, Memorize, LinkLight (caster-body only). The registry invariant now accepts a zero-primitive body when its batch declares it via `is_caster_body_only`, so the remaining 32 T3 effects need no shared-file edits.
- Manual-tier blockers measured: 12 portable as-is, 15 need one actor-render field (`effectPhase` 8, `tintPhase` 5, `shrinkPhase`/`outlineAlpha`/`runStartTick`), 2 need `NotifyGameMode`.

### Round 92 (manual tier + Champion)

- Manual tier 3/35 -> **16/35**: MagicCrasher/Memorize/LinkLight, then RedBody, Assumptio, ReflectBody, PinkBody, MagicCrasher2, ChemicalBody, DoubleCastBody, Shrink, FalconAssault, plus LightBlade, AuraBlade2, AsuraBody, AsuraBody_Monster, TransBlueBody.
- Champion: ComboAttack1..5 and ChainCombo as `.str` bodies (rows added to `str_table.rs`), AsuraBody/AsuraBody_Monster with `parent_outline_alpha`/`parent_effect_phase` (record-only; the original only resets them in `CRenderObject::Reset`, so ASURABODY pins at 200 and ASURABODY_MONSTER freezes at 199).
- Champion still blocked on primitives: **35** (Gumgang 203, SteelBody 254), **31** (Gumgang2 261), **30** (AuraBlade 367 + the two rings in BeginAsura/BeginAsura11), **68 `PP_ASURACASTING`** (BeginAsura 328, BeginAsura1..7 467-473, BeginAsura11 510). Prims 30/31/35 have no `EffectPrimId.h` symbol (enum jumps 27 -> 34).
- `EF_SU_STOOP` has no `EffectId` variant (packets jump 1124 -> 1133).

### Round 93 (Champion complete)

- prim **68** `PP_ASURACASTING` (`PrimAsuraCasting` + `RenderHaloLayers`), **30** `Casting3d` (inline motion, reuses `render_3d_casting`), **31** `WaveSpin` (`PrimWaveSpin`), **35** `FloatingBubbles` (`PrimFloatingBubbles` + `RenderOrbitIcons`). Prims 30/31/35 have no `EffectPrimId.h` symbol, so they are named after their motion/render functions.
- Helpers: `TexturedQuadRingSized`, `SuperSlashFan`, `TexturedCrossLevel`, `TexturedSkillIcon`, `AsuraStrike`, `WhiteYellowRings`.
- **Champion is complete**: Gumgang 203, Steelbody 254, Gumgang2 261, Beginasura 328, Aurablade 367, Beginasura1..7 467-473, Beginasura11 510 - with `EffectId::SuStoop` (1132) as a bonus fix.
- Render deviation: `RenderTeiRect`'s `extra == 2` (prim 35 only) nudges reciprocal depth by 0.000045; the shared triangle path has no per-vertex depth, so geometry/colour/blend are exact and only that nudge is omitted.
- Still open: `EF_GUMGANGNPC` (portable, one call), prim 99 `PP_DOUBLEGUMGANG`, prim 147 `RenderHaloLayers` consumer.

### Round 94 (prim 99, prim 147 and the last still-open ids)

- prim **99** `PP_DOUBLEGUMGANG` - `RenderDoubleGumGang` (`EffectPrimitiveRendering.cpp:7106`, dispatch `:495-497`). Shares prim 35's `PrimFloatingBubbles` motion and slot layout; new `SlotRenderer::DoubleGumGang`.
- prim **147** - no `EffectPrimId.h` symbol. Motion `PrimRisingFan` (prim 130, `EffectPrimitiveMotion.cpp:8425`), renderer `RenderHaloLayers` (prim 68, `EffectPrimitiveRendering.cpp:431`). Its only spawner is `CRagEffect::TexturedSlam`, so `render_slots` dispatches it as a distinct halo consumer.
- Bodies: `EF_GUMGANGNPC` (`SuperSlashFan(2, 1)`), `EF_DOUBLEGUMGANG`/2/3 (`SuperSlashSpread(2, 2/1/0)`), and the `TexturedSlam` family - `EF_SECRA`/2, `EF_ROLLING1..10`, `EF_HEARTASURA` (new `rag/textured_slam.rs`). 17 ids.
- Quirk kept: `TexturedSlam` never calls `FaceCamera`, so its `RenderHaloLayers` layers use the constructor's zeroed `slots[0..1].alphaT`; the live render path passes the stored angles for prim 147 and the recovered camera angles for prim 68.
- RNG: `SuperSlashSpread` draws the same twelve-per-slot `10, 5, 360` sequence as `SuperSlashFan` (24 values for its two slots).
- Registry now 393 ids; the skill map regenerates with 185 mappings.

### Round 94 (manual tier complete, fan/double-gumgang family)

- Manual tier **33/35** (18 bodies in batch J with their actor-render fields, plus DoubleGumGang/HeartAsura/Rolling1..10 via prim 147).
- prim **99** `PP_DOUBLEGUMGANG` (shared `PrimFloatingBubbles` motion, new `RenderDoubleGumGang`) and prim **147** (`TexturedSlam`, reuses `PrimRisingFan` + `render_halo_layers`).
- 17 ids: Gumgangnpc 455, Doublegumgang 418/485/486, Secra 859/862, Rolling1..10 758-767, Heartasura 879.
- Backlog coverage **178/236 = 75.4%** (effort 74.0%): `.str` 57/57, layers 88/144, manual 33/35.

### Round 95 (actor render passes + particle spline)

Rendered vs record-only for the parent render registers:

| register | state | consumer |
| --- | --- | --- |
| `tintPhase` | **rendered** | `world::entity::render_phase::tint` + additive body redraw (`DrawTintedQuad`, `RenderObject.cpp:826-846`); RGB comes from the actor tint because the client has no per-frame `render.flags` |
| `shrinkPhase` | **rendered** | `render_phase::pulse_scale` applied to every layer scale (`ScaleByPulse`, `:749-767`) |
| `outlineAlpha` | **rendered** | three flat 1-3% body copies in `0x3737FF` (`Player.cpp:1139-1190`) |
| `effectPhase` | record-only | every consumer is gated on a different `render.flags` bit; no per-frame flag register |
| `runStartTick` | record-only | needs the headband sprite-override table + footprint registry |
| `NotifyGameMode` | record-only | needs the localised scream/frost-joke assets and the talk packet path |

- prim **8** `PP_3DPARTICLESPLINE` (`Prim3DParticleSpline`, new `MakeTransform`/`Lift`; renders through the existing sprite pass) + ids Waterball2 117, Blooddrain 216, Energydrain 217, Energydrain2 378, Energydrain3 383, Soulstrike 15, Soulstrike2 451, EnergydrainBlack 1208.
- Backlog coverage **181/236 = 76.7%** (effort 75.3%): `.str` 57/57, layers 91/144, manual 33/35.
- Known mapping gap: `EF_PEONG_BLACK` (1207) has no `EffectId` variant, so `SonicSlashFan(1)` cannot be registered yet.

### Round 96

- prim **10** `PP_3DSPLINE` (`Spline3d`) and **11** `PP_3DSPIRAL` (`Spiral3d`) with the shared `RenderRecordQuads` renderer; `ShiftSegment` now writes `records[0].corners`, without which record quads had no geometry. **No effect body in this decompilation launches prim 10/11** (all 395 `LaunchEffectPrim` sites checked), so no ids were registered for them - the primitives are ready for a newer revision.
- Six `EffectId` variants added for already-implemented bodies: PeongBlack 1207, Blooddrain2 1052, NewHeavensDrive 1399, HealBlue2 2427, RgCoinBlue 2436, RgCoinBlue2 2437.
- 21 ring-wrapper aliases (1418, 1556, 1560, 1565, 1677, 1872, 1876, 1902, 1910, 1918, 1923, 1932, 1938, 1959, 1963, 1986, 1998, 2240, 2249, 2255, 2367) - all outside the 236-id pre-renewal backlog, so they raise the total registry without moving backlog coverage.
- Backlog coverage stays **181/236 = 76.7%**; the 55 remaining ids are the renewal/`PP_UNKNOWN_*` cluster and the alternate throw-item particle path.

### Animation round (skill motions)

Audit: `ro-next-fx-audit/skill-motion-audit.md` (771 lines, every row with `file:line`).

- 373 castable pre-re skills, **0** without an action row; **338 (90.6%)** exact official block, **357 (95.7%)** including pose-equivalent Idle rows.
- Root cause was architectural: one table for two official functions - `ZC_USESKILL_ACK` uses `GetSkillActionInfo` (default 8), `ZC_USE_SKILL` uses `GetSkillActionInfo2` (default 7); 131/373 name a different block, 86 need an override (83 visibly different, 3 `Keep`).
- Landed: `fcf61aec` (11 `motion = 2` skills use weapon-aware attack), `b7e74af0` (`use_action` from 462 Info2 facts, `CasterAction::Keep`, `delay == 0` ack gate), `4d10c9c7` (audit tooling), `05af4ee4` (official 500 ms/frame sequence rate from `SetSequenceInterval(30)` + 8 attached sequences).
- Open: 3 `ApplyActorSkillState` freeze sequences (261/1015/401), 21 sequence rows with no action row (4th-job + `RL_FALLEN_ANGEL`, outside the pre-re set), `MO_STEELBODY` hold, monster skill motions (`GetMonsterSkillInfo`) absent, 19 PROXY-IDLE rows using block 32 vs 0.

### Round 98

- prim **46** `ChimtoSprite` (reuses the shared sprite pass), **69** `PentaRing` (`RenderPentaRing`), **100** `HitLine3` (`RenderHitLine3`).
- ids: Hated 543, Bluecasting 441, NapalmValcan 401 (dead `Hit2` documented), Chimto/FVoice/Wink 267/606/607, HitLine4/5/6 429-431, TripleAttack/2/3 329/388/393. `HitLine3` 428 deliberately unregistered (`WhiteSparkStreak(0)` sets `limit = 0`, always-true gate).
- Monster skill motions are data-driven (`data\MonsterSkillInfo.xml`, mixcrypt-encrypted in the GRF): `monster_skill_actions.ron`, 106 entries / 96 monsters, applied on both cast paths. Cast-start sequences close 261/1015/401; 268 Steel Body both variants; 19 PROXY-IDLE rows corrected (block 0 non-local, 32 local).
- Backlog coverage **204/236 = 86.4%** (effort 85.3%): `.str` 57/57, layers 114/144, manual 33/35. 32 ids remain.

### Round 99

- prim **34** `Gi3` (new `render_3d_casting2`), **37** `GrandCross2` (new `render_grand_cross2`), **44** `ExpandPulse` (new `render_rotated_plate`), **45** `TeiHit2` (reuses `render_field_panel`).
- ids: GrandCross 226 / GrandCross2 450, TeiHit2 264, TeiHit1x 266, BackStab 275, TeiHit3 276.
- Backlog **209/236 = 88.6%** (layers 119/144). 27 ids remain.
- Free but unregistered: `EF_TEIHIT1` 262 and `EF_TEIHIT1T` 889 (outside the 236 backlog).
- Generated map now routes skill 266 (`MO_INVESTIGATE`) to `EF_TEIHIT2` instead of `EF_CHIMTO`, both official; the generator's documented first-ported-candidate rule decides, no manual edit.

### Round 100

- prim **50** `TexturedGlow` (inline motion, reuses `render_3d_casting`), **38** `LineLink` (new `render_tube`/`RenderTube`), **54** `ChemicalProtection` (new `render_chemical`), **58** `FirstAid` (reuses `render_field_panel`).
- ids: TeiHit1 262, TeiHit1t 889, EternalChaos 280, Siegfried 285, Linelink2 384, Linelink3 395, ChemicalProtection 300, FirstAid 309.
- Backlog **215/236 = 91.1%** (layers 125/144, `.str` 57/57, manual 33/35). 21 ids remain.
- Documented limitation: `TrackTarget`'s live per-frame `GetGameActorByAID` re-query is a spawn-time snapshot in the hermetic runtime (the resolved target position travels in `deltaPos`), so a target that moves mid-flight is tracked as it was at spawn.

### Client-system rounds (damage numbers, feedback, motions)

- **Damage numbers**: official port (`world/damage_number.rs`) - `SpawnDamageDigits` + `CMsgEffect::OnProcess` styles (normal/critical/half/total/simple/miss), 24 ms timeline on the 16 ms step, per-digit advance, ACT cell adjustment, owner-follow, 200 ms multi-hit stagger, blue damage from `ZC_NOTIFY_ACT3`, and a `show_damage_numbers` setting (default on).
- **Auto-attack feedback**: official impact wave rule (`HitSounds.cpp`, weapon class / defender job / bare-hand / enemy fallback), `EF_HIT1` on hit plus additive `EF_HIT2` on crit, `EF_HITLINE7` for the 14 Taekwon-family jobs, no hit-flash tint; arrow duplicate sound removed.
- **Ruwach**: `EF_RUWACH` (33) is an option aura (`RagEffect.cpp:23193`), sound `effect\EF_Sight.wav` from `SetStatusStates` (`RenderObject.cpp:4867`), motion block 96.
- **.str blend fix**: `RenderAniClip` uses the clip's own blend pair; Concentration's frames are `(5,7)` = `SRCALPHA`/`DESTALPHA` (additive), not the hard-coded `(5,6)`. Fixed per layer, which corrects 65 of the 68 ported `.str` resources, plus stage-aware `BOTHSRCALPHA`/`BOTHINVSRCALPHA`.
- **Monk**: Finger Offensive (267, not 263) ack no longer double-plays the damage pose; its 2019-only `EF_HIT1` hit row removed so `mon_탄지신통` plays; multi-hit numbers are a damage-path split (official `damage /= hitCount` + 200 ms), not a skill bug. Blade Stop lock (block 96 frame 4, grey tint, motion freeze) verified and pinned.
- Investigate: 2019 vs 2026 sound drift resolved in favour of 2019 (`mon_탄지신통.wav`), documented as the single curated choice.

### Round 102 (final backlog primitives, status launches, packet gaps)

- prim **56** `PP_CHEMICAL3` (Chemical3/4), **41** `TexturedCross`, **77** `PP_SPHEREWIND`, **109** `PP_SOULLINK`, **115** `ParticleSpread`, **119/122/125/126** (STIN family). Ids: Chemical3, Chemical4, Providence, Spherewind2, Soullink, Cartter, Stin, Stin2, Stin4, Stin5, Jumpkick.
- **Status/skill -> effect launch channel**: `status_effect_launches.ron` (generated) with `relaunch` for the official delete-then-spawn; Steel Body/D Barrier now launch `EF_SU_STOOP` (sound), Soul Collect launches `EF_PORTAL5` on a non-zero cast delay.
- **Post-combat armed idle**: attack/skill end -> block 32, movement landing -> block 0, `healthState != 0` arms every stand (`Player.cpp:4423-4475`).
- **Dual-wield off-hand**: `damage_amount_2` wired with the official 350 ms stagger (`g_multiHitInterval/2 + 3/4`), same style/colour, second feedback wave + hit effect.
- **`EF_VIOLEN_HIT`** (2420) added and the Violen jobs now select it. NOTE: `몬스터\violen_hit.spr/.act` is absent from all three GRFs, so it spawns and draws nothing - the official client's hard-coded path fails identically.
- **Monster skill motion on damage packets**: `GetMonsterSkillInfo` applied after the skill pose, matching `GameModePackets.cpp:4283/4356`.
- Remaining backlog: **3 ids** - 555 `EF_STIN3` (needs a renderer->child-effect outbox plus `EF_SMA2`), 618 `EF_KOUENKA`, 843 `EF_BEGINSPELL_N6`.

### Known wiring gaps (client spawn path)

- effect-level `count` is never set by the client spawn path (`runtime.param = 1.0` only), so bodies that read `state.count` (Blitz Beat bolts, Fire/Ice Arrow) emit nothing in game until the skill launcher's `AM_SETCOUNT` equivalent is wired.
- **Audio bridge**: `runtime.sounds` / `runtime.positional_sounds` are recorded
  but not yet forwarded to the `AudioEngine`, so ported sounds are silent while
  the visuals and `.str` layers play.
- `parent_flags` bits are recorded but not translated to the caster entity (no per-frame `render.flags` register; see the Round 88/89 note).
- Chemical V/V2/V3 are notice effects (`GameModeNotices.cpp:4234/4248`), not skill-table effects, so the generated skill map cannot reach them. `EF_NEW_METALLICSOUND_LINE` is a notice path too, so it is registered but has no skill-map row.

### Ready next (no engine work needed)

`.str`-only follow-up: 34 more T1 ids are the same clip-only profile (29, 67, 70, 84, 85, 88, 101, 103, 104, 111, 112, 126, 136, 143, 148, 149, 150, 151, 169, 234, 235, 268, 269-272, 390, 391, 406, 407, 440, 635, 636, 649).
Blocked on runtime support (12): positional sound height (`PlayEffectWaveAt`/`PlayWave` with offsets) for Gloria, FlameLauncher, FrostWeapon, LightningLoader, SeismicWeapon, HolyCross, Loud; `process_str` offsets other than `(0,-80,0)` for ShieldCharge/Devotion; parent actor flag bits for RejectSword; and `EF_THUNDERSTORM` (portable, but the unported-effects test still pins it).

### Queued after the port (last item)

Weapons are not visible on characters in-client. This is **outside** the effect
port: the entity exposes `get_weapon_part_candidates` and the paper-doll slot
table exists, so the gap is the weapon appearance pipeline (equip packet ->
entity -> sprite part mapping -> actor render). Tracked in the audit
`PROGRESS.md` TODO section; a dedicated agent owns the investigation so the
ported effect files stay untouched.

### Round 97 (renewal cluster: prims 46/69/100 and the three free ids)

| primitive | name in port | official source | renderer |
| --- | --- | --- | --- |
| 46 | `ChimtoSprite` | bare `case 46`, `EffectPrimitiveMotion.cpp:9741-9760` | shared `RenderSegmentSprites2` (sprite pass, `EffectPrimitiveRendering.cpp:272-281`) |
| 69 | `PentaRing` | bare `case 69`, `EffectPrimitiveMotion.cpp:9592-9639` | new `render_penta_ring` (`RenderPentaRing`, `EffectPrimitiveRendering.cpp:1399-1477`, dispatch `:693-695`) |
| 100 | `HitLine3` (`PP_HITLINE3`) | `EffectPrimitiveMotion.cpp:8458-8483` | new `render_hitline3` (`RenderHitLine3`, `EffectPrimitiveRendering.cpp:1872-1962`, dispatch `:440-442`) |

- **Free ids** (bodies/primitives already ported): `EF_HATED` 543 (`Hated`, inline
  caster `tintPhase` + `ParticleTime(..., 1)`; lifetime 500), `EF_BLUECASTING` 441
  (`BlueSpellRings` -> two `TexturedQuadRingSized` + positional `EF_BeginSpell.wav`;
  lifetime 60), `EF_NAPALMVALCAN` 401 (`NapalmValcan`; lifetime 200).
- **Prim 46** ids `EF_CHIMTO` 267, `EF_FVOICE` 606, `EF_WINK` 607 (`rag/chimto.rs`),
  lifetimes 16/100/100.
- **Prim 100** ids `EF_HITLINE4` 429, `EF_HITLINE5` 430, `EF_HITLINE6` 431
  (`rag/white_spark.rs`: `White02Burst(3)`, `WhiteSparkRing(3)`, `WhiteSparkStreak(3)`),
  lifetime 200 each. `EF_HITLINE3` (428) stays unregistered: `WhiteSparkStreak(0)`
  computes `limit = 0`, so its `stateCnt >= limit` gate is always true and the
  body is dead.
- **Prim 69** ids `EF_TRIPLEATTACK` 329, `EF_TRIPLEATTACK2` 388, `EF_TRIPLEATTACK3`
  393 (`rag/triple_attack.rs`: `CloudDrift`, `AlphaCenterTrail`, `AlphaCenterStream`),
  lifetimes 100/200/200.
- Quirks kept: `EF_NAPALMVALCAN`'s frames 20-60 call `Hit2(0)`, which is dead after
  frame 0, so the body is sound-only (`spawns_no_geometry`); prim 46 discards
  `AnimMotion`'s return value, so a finished non-repeating ACT never retires it;
  `TexturedQuadRingSized`'s `duration <= 60` branch zeroes Bluecasting's ring
  alphas and staggers its slots; `RenderPentaRing` kinds 4/5 force the near plane.
- Registry now 454 ids; the skill map regenerates with 212 mappings.

### Round 99 (Grand Cross and slash-hit families)

| primitive | name in port | official source | renderer |
| --- | --- | --- | --- |
| 34 | `Gi3` (`PP_GI_3`) | `PrimGI3`, `EffectPrimitiveMotion.cpp:2440-2473`, dispatch `:8921-8922` | new `render_3d_casting2` (`Render3DCasting2`, `EffectPrimitiveRendering.cpp:6519-6592`, dispatch `:521-522`) |
| 37 | `GrandCross2` (`PP_GRANDCROSS2`) | bare `case PP_GRANDCROSS2:`, `EffectPrimitiveMotion.cpp:9184-9211` | new `render_grand_cross2` (`RenderGrandCross2`, `EffectPrimitiveRendering.cpp:7365-7421`, dispatch `:524-525`) |
| 44 | `ExpandPulse` (no enum symbol) | `PrimExpandPulse`, `EffectPrimitiveMotion.cpp:5331-5369`, dispatch `:9312-9313` | new `render_rotated_plate` (`RenderRotatedPlate`, `EffectPrimitiveRendering.cpp:2747-2784`, dispatch `:542-543`) |
| 45 | `TeiHit2` (`PP_TEIHIT2`) | `PrimTeiHit2`, `EffectPrimitiveMotion.cpp:1840-1898`, dispatch `:10018-10019` | shared `RenderFieldPanel` (`EffectPrimitiveRendering.cpp:3312-3505`, `:574`) |

- **Bodies** (one Rust function per C++ function): `rag/grand_cross.rs`
  (`GrandCrossRings` `:16197-16267`, `GRANDCROSS2` `:5163-5195`),
  `rag/textured_scatter.rs` (`TexturedScatter` `:3180-3209`,
  `TexturedScatterLow` `:3211-3240`), `rag/tei_hit2.rs` (`TEIHIT2`
  `:12040-12085`).
- **Ids** (lifetimes from `EffectTypes.cpp`): `EF_GRANDCROSS` 226 / 999,
  `EF_GRANDCROSS2` 450 / 999, `EF_TEIHIT2` 264 / 300, `EF_TEIHIT1X` 266 / 300,
  `EF_BACKSTAP` 275 / 300, `EF_TEIHIT3` 276 / 300. The first five are in the
  pre-renewal backlog; 450 is the renewal alias of the same two bodies.
- **RNG** (MSVC `rand()` order preserved; port uses the documented xorshift
  stand-in): `TexturedScatter` per slot `ClampedRandom(25)`, `(360)`, `(360)`;
  `TexturedScatterLow` per slot `ClampedRandom(10)`, `(360)`, `(360)`;
  `TEIHIT2` per slot `ClampedRandom(10)`, `(16)`, `(50)`, `(200)`; prim 45's
  motion `Random(60) - 30` on the first positive `process`. Grand Cross bodies
  consume no RNG.
- **Quirks kept**: `TexturedScatter` drops `render.position.y` by 9 *after* each
  launch, and the dispatch loops call it 24/20 times, so the effect ends 216/180
  units down with primitives at `y = 0, -9, ...`; `TexturedScatterLow` gates
  before the parent copy; `PrimGI3` survives to `process` 99 (death needs
  `alphaB <= 0` *and* `process > 30`) and resets `distance`/`riseAngle` on the
  death frame; `GrandCross` calls both bodies on every frame, but each gates
  itself on `stateCnt == 0`; `EF_BACKSTAP` copies the parent 20 times every
  frame and only the frame-20 wave survives the gate.
- Registry now **460** ids; the skill map regenerates with **216** mappings.
  Backlog coverage **209/236 = 88.6%** (`.str` 57/57, layers 119/144, manual
  33/35).
- Verification: `cargo test -p ro-next --lib` **1837 passed, 0 failed, 21
  ignored**; release build finished with no warnings; `rustfmt --check` clean
  on every touched file; `gen_ported_skill_map.py --check` up to date.

### Round 100 (line link, glow and protection families)

| primitive | name in port | official source | renderer |
| --- | --- | --- | --- |
| 50 | `TexturedGlow` (no enum symbol) | inline `case 50:`, `EffectPrimitiveMotion.cpp:8962-8994` | shared `Render3DCasting` (`EffectPrimitiveRendering.cpp:6596-6675`, dispatch `:504`) |
| 38 | `LineLink` (`PP_LINELINK`) | `PrimLineLink`, `EffectPrimitiveMotion.cpp:1386-1441`, dispatch `:9589-9590` | new `render_tube` (`RenderTube`, `EffectPrimitiveRendering.cpp:3214-3310`, dispatch `:690-691`) |
| 54 | `ChemicalProtection` (`PP_CHEMICALPROTECTION`) | `PrimChemical`, `EffectPrimitiveMotion.cpp:1130-1148`, dispatch `:9919-9920` | new `render_chemical` (`RenderChemical`, `EffectPrimitiveRendering.cpp:6906-6975`, dispatch `:611-612`) |
| 58 | `FirstAid` (`PP_FIRSTAID`) | `PrimFirstAid`, `EffectPrimitiveMotion.cpp:1223-1248`, dispatch `:9892-9893` | shared `RenderFieldPanel` (`EffectPrimitiveRendering.cpp:3312-3505`, `:578`) |

- **Bodies** (one Rust function per C++ function): `rag/textured_glow.rs`
  (`TexturedGlow` `:5516-5565`), `rag/line_link.rs` (`TrackTarget`
  `:3839-3878`, `TrackTargetWide` `:3884-3929`), `rag/chemical_protection.rs`
  (`ShockwaveRing` `:2807-2838` + dispatch `:10411-10418`), `rag/first_aid.rs`
  (`FirstAid` `:5123-5161`), `rag/textured_scatter.rs` (`TexturedScatter` +
  `EF_TEIHIT1T` dispatch `:19571-19583`).
- **Ids** (lifetimes from `EffectTypes.cpp`): `EF_TEIHIT1` 262 / 300,
  `EF_TEIHIT1T` 889 / 300, `EF_BOTTOM_ETERNALCHAOS` 280 / 29999,
  `EF_BOTTOM_SIEGFRIED` 285 / 29999, `EF_LINELINK2` 384 / 200,
  `EF_LINELINK3` 395 / 99999999, `EF_CHEMICALPROTECTION` 300 / 300,
  `EF_FIRSTAID` 309 / 299.
- **RNG** (MSVC `rand()` order preserved): `TexturedGlow` per spawn
  `ClampedRandom(10)`, `(360)`, `(10)`, `(360)` (maxHeight, rotStart, alphaB,
  alphaT); `FirstAid` `ClampedRandom(360)` twice (fullDisplayAngle, height[8]).
  Line link and shockwave consume no RNG.
- **Quirks kept**: `ShockwaveRing` resets the anchor at the start of every call,
  so `EF_CHEMICALPROTECTION`'s second call cancels the first 9-unit drop (the
  effect ends 9, not 18, below the caster); `TexturedGlow` zeroes all four
  `life`s before arming slot 0 and then zeroes all 21 heights/flag1s;
  `PrimLineLink` re-resolves the target **every** frame and its unclamped blink
  branch uses wrapping `i16` arithmetic (the effect lives for ~10^8 frames);
  `RenderTube` reuses pass 0's `alphaB / 2` in pass 1 for kind 4; `TexturedGlow`
  and `FirstAid` are drawn by shared renderers.
- **Target model**: `TrackTarget`'s `height[0] = state.deltaPos.y` is the
  packet's `[0, accountId, 0]` (`GameModePackets.cpp:19072-19075`), which the
  original resolves with `GetGameActorByAID` every frame. The port's spawn
  bridge already stores the *resolved target position* in `deltaPos`
  (`client/effects.rs`, like every travelling body), so the body records that
  position in `vecTPre` and `PrimLineLink` re-reads
  `master.master_delta_pos` each frame; `height[0] > 0` keeps the original's
  "target is set" gate. The live actor re-query is a spawn-time snapshot in the
  hermetic runtime (the same limitation as the other target-aimed bodies).
- Registry now **468** ids; the skill map regenerates with **223** mappings.
  Backlog coverage **215/236 = 91.1%** (`.str` 57/57, layers 125/144, manual
  33/35).
- Verification: `cargo test -p ro-next --lib` **1877 passed, 0 failed, 21
  ignored**; release build finished with no warnings; `rustfmt --check` clean
  on every touched file; `gen_ported_skill_map.py --check` up to date.

### Round 101 (pressure, guard, magnum, soulbreaker and baby families)

| primitive | name in port | official source | renderer |
| --- | --- | --- | --- |
| 61 | `Guard` (`PP_GUARD`) | `PrimGuard`, `EffectPrimitiveMotion.cpp:1344-1383`, dispatch `:8547-8548` | new `render_sphere_cage` (`RenderSphereCage`, `EffectPrimitiveRendering.cpp:4328-4584`, dispatch `:391-392`) |
| 73 | `Magnum2` (`PP_MAGNUM2`) | bare `case PP_MAGNUM2:`, `EffectPrimitiveMotion.cpp:8377-8420` | new `render_magnum_2` (`RenderMagnum2`, `EffectPrimitiveRendering.cpp:2121-2177`, dispatch `:388-390`) |
| 86 | `SonicWave` (no enum symbol) | bare `case 86:`, `EffectPrimitiveMotion.cpp:9315-9365` | new `render_floor_square` (`RenderFloorSquare`, `EffectPrimitiveRendering.cpp:2637-2672`, dispatch `:545-547`) |
| 87 | `BubbleSwirl` (no enum symbol) | `PrimBubbleSwirl`, `EffectPrimitiveMotion.cpp:6962-7068`, dispatch `:9811-9812` | shared `RenderFieldPanel` (`:580`), now with a render-time `FaceCamera` override |
| 88 | `GrowFollow` (no enum symbol) | `PrimGrowFollow`, `EffectPrimitiveMotion.cpp:5264-5295`, dispatch `:8781-8782` | shared `RenderSizedSquare` (`:417-418`) |
| 91 | `SphereWind2` (`PP_SPHEREWIND2`) | `PrimSphereWind2`, `EffectPrimitiveMotion.cpp:4096-4143`, dispatch `:8676-8677` | new `render_sphere_wind` (`RenderSphereWind`, `EffectPrimitiveRendering.cpp:5591-5646`, dispatch `:404-405`) |
| 92 | `PurpleSlash` (no enum symbol) | bare `case 92:`, `EffectPrimitiveMotion.cpp:9367-9399` | shared `render_floor_square` (`:545-547`) |

- **Bodies** (one Rust function per C++ function): `rag/pressure.rs`
  (`TexturedRain` `:5317-5378`, `TexturedLevelRing` `:4096-4140` +
  `EF_PRESSURE` dispatch `:10342-10349`), `rag/black_maximize.rs`
  (`BlackMaximize` `:3453-3480` + `EF_GUARD2` dispatch `:21646-21658`),
  `rag/magnum2.rs` (`:5197-5242`), `rag/sonic_wave.rs` (`SonicWave`
  `:5382-5426` + `EF_SOULBREAKER` dispatch `:10320-10332`),
  `rag/purple_slash.rs` (`PurpleSlash` `:3382-3411` + `EF_SOULBREAKER2`
  dispatch `:24333-24347`), `rag/feather_burst.rs` (`FeatherBurst`
  `:4187-4220` + `EF_BABY` dispatch `:23325-23333`).
- **Ids** (lifetimes from `EffectTypes.cpp`): `EF_MAGNUM2` 339 / 100,
  `EF_SOULBREAKER` 361 / 300, `EF_PRESSURE` 365 / 300, `EF_BABY` 408 / 200,
  `EF_SOULBREAKER2` 409 / 300, `EF_GUARD2` 496 / 200.
- **RNG** (MSVC `rand()` order preserved): `TexturedRain` draws the `kind == 4`
  vertical offset (`rand() % 56`) **before** `fullDisplayAngle`
  (`ClampedRandom(360)`); `TexturedLevelRing` draws `rand() % 6` only for level
  3; `SonicWave`/`PurpleSlash` draw one `ClampedRandom(5)` per slot
  (`PurpleSlash` additionally draws its `ClampedRandom(6)` offset once per
  call, before the four jitters); `FeatherBurst` draws per slot
  `20, 11, 360, 91, 4`; `Magnum2`, `BlackMaximize` and the pressure dispatch
  draw none.
- **Quirks kept**: `Magnum2` only spawns on `stateCnt % 3 == 0` inside
  `25..=55` (so frames 27, 30, ..., 54 - ten waves, and the body's
  `flag < 11` gate never fires for `EF_MAGNUM2`); `TexturedRain` leaves slots
  1..3 with `process = 0`, so `PrimBubbleSwirl` copies the lead's fields one
  frame late and sends `MM_QUAKE` on process 33 (`kind 0`) or on the
  `250..280` angle window (`kind 4`, which also clears `flag1[4]`);
  `PrimGrowFollow`'s alpha drain is short-circuited behind `process > 10`;
  `PrimGuard`'s alpha add has no clamp after the `< 100` test;
  `EF_SOULBREAKER`/`EF_SOULBREAKER2` set the caster colour on every frame
  0..10 with no parity gate; `EF_BABY` calls `FeatherBurst` five times on
  frame 0 and once per later frame (each call re-copies the parent anchor and
  then returns).
- **Render deviations** (documented, simulation untouched): `RenderMagnum2`
  and `RenderSphereCage` write into the slot (`vecB*`/`vecT*`, `distance`) at
  render time; the port keeps those as locals because rendering is `&self` -
  no motion body reads them for prim 73/61. `RenderFieldPanel`'s prim-87
  `FaceCamera` angles are computed at draw time from the camera (like prims
  35/68/82/96) because the hermetic runtime has none.
- Registry now **474** ids; the skill map regenerates with **229** mappings.
  Backlog coverage **221/236 = 93.6%**.
- Verification: `cargo test -p ro-next --lib` **1963 passed, 0 failed, 21
  ignored**; release build finished with no warnings; `rustfmt --check` clean
  on every touched file; `gen_ported_skill_map.py --check` up to date.

### Round 102 (final pre-renewal backlog: chemical, slash, soul and crash families)

| primitive | name in port | official source | renderer |
| --- | --- | --- | --- |
| 56 | `Chemical3` (`PP_CHEMICAL3`) | `PrimChemical3`, `EffectPrimitiveMotion.cpp:1084-1117`, dispatch `:9925-9926` | new `render_ground_bar` (`RenderGroundBar`, `EffectPrimitiveRendering.cpp:3925-4030`, dispatch `:638-639`) |
| 41 | `TexturedCross` (no symbol) | inline `case 41:`, `EffectPrimitiveMotion.cpp:9036-9089` | shared `Render3DCasting` (`:504`) |
| 77 | `SphereWind` (`PP_SPHEREWIND`) | inline `case PP_SPHEREWIND:`, `EffectPrimitiveMotion.cpp:8657-8677` | shared `RenderSphereWind` (`:404-405`) |
| 109 | `SoulLink` (`PP_SOULLINK`) | inline `case PP_SOULLINK:`, `EffectPrimitiveMotion.cpp:8808-8843` | new `render_soul_link` (`RenderSoulLink`, `EffectPrimitiveRendering.cpp:5469-5557`, dispatch `:479-480`) |
| 115 | `ParticleSpread` (no symbol) | `PrimParticleSpread`, `EffectPrimitiveMotion.cpp:3647-3705`, dispatch `:8793-8794` | new `render_soul_particle` (`RenderSoulParticle`, `:5558-5590`, dispatch `:469-470`) |
| 119 | `SlashFan` (no symbol) | inline `case 119:`, `EffectPrimitiveMotion.cpp:9400-9448` | new `render_floor_square2` (`RenderFloorSquare2`, `:2674-2709`, dispatch `:549-551`) |
| 122 | `Stin3` (no symbol) | `PrimSTIN3`, `EffectPrimitiveMotion.cpp:7071-7173`, dispatch `:9451-9452` | shared `render_floor_square2` |
| 125 | `Stin4` (`PP_STIN4`) | `PrimSTIN4`, `EffectPrimitiveMotion.cpp:3874-3977`, dispatch `:9454-9455` | shared `render_floor_square2` |
| 126 | `SlashCross` (no symbol) | inline `case 126:`, `EffectPrimitiveMotion.cpp:9457-9505` | new `render_rotated_ring` (`RenderRotatedRing`, `:2608-2635`, dispatch `:554`) |

- **Bodies** (one Rust function per C++ function): `rag/line_streak.rs`
  (`LineStreakLayered` `:16760-16831`), `rag/textured_cross.rs` (`TexturedCross`
  `:4993-5030` + `EF_PROVIDENCE` dispatch `:24609-24614`), `rag/feather_ring.rs`
  (`FeatherRing` `:4224-4271`), `rag/soul_link.rs` (`TexturedOrbit`
  `:4954-4992`, `SoulLink` `:13885-13965` + dispatch `:21566-21586`),
  `rag/textured_tumble.rs` (`TexturedTumble` `:6022-6071` + `EF_CARTTER`
  dispatch `:21477-21482`), `rag/slash_family.rs` (`TexturedSlashFan`
  `:3980-4032`, `TexturedSlashWave` `:4687-4740`, `TexturedSlashCross`
  `:4902-4952`, `TexturedSlashLevel` `:5032-5095`) and `rag/jumpkick.rs`
  (`EF_JUMPKICK` `:24591-24604`).
- **Ids** (lifetimes from `EffectTypes.cpp`): `EF_CHEMICAL3` 300, `EF_CHEMICAL4`
  300, `EF_PROVIDENCE` 200, `EF_SPHEREWIND2` 99999, `EF_SOULLINK` 500,
  `EF_CARTTER` 200, `EF_STIN` 200, `EF_STIN2` 200, `EF_STIN4` 200, `EF_STIN5`
  200, `EF_JUMPKICK` 100. `EF_PROVIDENCE` gets its official `providence.str`
  row in `str_table.rs`.
- **RNG**: `LineStreakLayered` draws per slot `height[1] (200)`, then the
  `kind` block's `maxHeight`, `height[2]`, `flag1[0]`, `height[3]` (plus
  `height[0]` for kinds `% 10 == 0/1`); `FeatherRing` draws `maxHeight`,
  `rotStart`, `fullDisplayAngle`, `height[1]`; `TexturedSlashFan`/`Cross` draw
  `rand(5)` per slot and `Cross` a further `rand(360)`; `TexturedSlashWave`
  `distance` then `height[1]`; `TexturedSlashLevel` kind 1 draws the `rand(21)`
  turn; `TexturedTumble` draws the eight per-slot values in source order.
- **Quirks kept**: `TexturedSlashCross` spawns on frames 0..4 only (one per
  frame, offset `+5x`/`-5x`/`+5z`/`-5z`); `TexturedSlashFan`/`Wave`/`Level`
  spawn once on frame 0; `EF_STIN2` plays its five `T_날라차기.wav` with the
  listener-relative heights `5-frame` / `-50`; `EF_CHEMICAL3`'s second call is
  in the dispatch order kind 0 then 1 and `EF_CHEMICAL4` kind 20 then 21;
  `SoulLink` launches its two prim-68 letter sets on frames 1 and 21 only;
  `EF_JUMPKICK` writes parent state 27 + sequence on frame 5 and restores state
  2 on frame 35 only from 27.
- **Target model**: prims 122/125/126 read the spawn bridge's resolved target
  position from `master_delta_pos` (the original's `g_session.m_aid` /
  `GetGameActorByAID` lookup), like `PP_LINELINK`.
- **Render deviation**: `RenderSoulLink`'s ground-cross `EF_SOULLIGHT` spawn is
  a render-time effect launch the hermetic renderer cannot perform; the quad is
  drawn and the launch is documented, not emulated.
- Registry now **488** ids; the skill map regenerates with **236** mappings.
- Verification: `cargo test -p ro-next --lib` **2078 passed, 0 failed, 21
  ignored**; `rustfmt --check` clean on every touched file;
  `gen_ported_skill_map.py --check` up to date (release build pending).
