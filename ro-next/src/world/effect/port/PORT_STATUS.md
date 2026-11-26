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
| primitive `render.angle` (own facing) | `EffectPrimitiveState` | - |
| `.str` support in the runtime (`open_str`/`process_str`) | `ProcessEZ2STR` call sites | - |

Total: 105 tests, all green. Effect bodies ported: **15 / 236** + the generic `Hit2` impact.

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
