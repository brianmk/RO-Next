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
| `.str` support in the runtime (`open_str`/`process_str`) | `ProcessEZ2STR` call sites | - |

Total: 53 tests, all green. Effect bodies ported: **6 / 236**.

## Next (W2/W3/W6)

1. `PP_2DTEXTURE` (`Move2DPrimitive`) and `PP_3DTEXTURE` - the rest of the
   screen/ground family used by buffs. `PP_3DCIRCLE`/`PP_3DSPHERE` are done.
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
* `.str` animation type 4 keeps a persistent `ratio`; the original reads an
  uninitialised local (`RagEffect.cpp:1519-1522`). Undefined behaviour cannot be
  reproduced.
