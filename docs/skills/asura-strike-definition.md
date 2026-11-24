# Asura Strike (`MO_EXTREMITYFIST`, 271) — official definition

Read from the decompiled client (`github.com/rodecomp/rodecomp`,
`Source/Contents/...`). The GRF Lua tables carry no Asura entry: `SKID[271]` is
the id, `SKILL_INFO_LIST[271]` only has level/SP/range/prereqs, and
`SKILL_EFFECT_INFO_LIST` has no field for timing at all.

## Effect (1 000 ms)

| item | definition |
| --- | --- |
| effect id | `EF_BEGINASURA = 328` (`EffectTypes.h:336`) |
| duration | `result[EF_BEGINASURA] = 1000` (`EffectTypes.cpp:337`) |
| dispatch | `case EF_BEGINASURA: this->AsuraStrike(0)` (`RagEffect.cpp:10212`) |
| implementation | `CRagEffect::AsuraStrike(char kind)` (`RagEffect.cpp:14187`) |
| primitive | `PP_ASURACASTING = 68` (`EffectPrimId.h:53`) |
| motion | `CEffectPrim::PrimAsuraCasting` (`EffectPrimitiveMotion.cpp:3224`), drawn by `RenderHaloLayers` (`EffectPrimitiveRendering.cpp:430`) |

`AsuraStrike(0)`:

- state 0: `TexturedQuadRingSized(45, "effect\ring_white.tga", 2)` and the same
  with `25`, plus `effect\EF_BeginSpell.wav`, duration >= 350 ms;
- state 1: primitive 68 with `effect\asura3/2/1.tga`, slots at distance 18,
  starting heights -6 / -18 / -30, `process = index*20 - 40`;
- state 21: primitive 68 with `effect\asura4/5/6.tga`, starting heights
  +6 / +18 / +30, `process = -40 - index*20`.

`PrimAsuraCasting` is camera-facing, follows the master, raises each glyph
13 units per frame for 20 frames (caps 70/90/110/...), then decays 2/frame and
shrinks after frame 100. The six textures are the kanji of 阿修羅覇凰拳.

## Caster animation

`RenderObject.cpp:5825` (`case MO_EXTREMITYFIST`):

```cpp
SetSequenceInterval(4);
SetFrameSequence(0x58, 0, 7, 0x58, 7, 1, 3500, 0);   // attack block, frames 0..7, loop to 7, 3500 ms
isAsuraAttack = 1;
```

- `0x58` = 88 = the **attack** action block; `0x60` = 96 = the **skill** block.
- While `isAsuraAttack` is set, `ST_MOVE` does not switch to the walk action
  (`RenderObject.cpp:1486`) and the route marches at step 55
  (`GameObject.cpp:2307`) — the fast slide to the target.
- On arrival (`GameActor.cpp:257`) the flag is cleared and the pose is replaced
  by `SetFrameSequence(0x60, 2, 2, 0x60, 2, 1, 3000, 0)` — skill block frame 2
  held for 3 s.

## Champion skill frame sequences (`RenderObject.cpp`)

| skill | block | frames | note |
| --- | --- | --- | --- |
| `MO_TRIPLEATTACK` | `0x58` attack | 0..7, loop 7 | skill-anim duration |
| `MO_EXTREMITYFIST` | `0x58` attack | 0..7, loop 7 | 3500 ms, then skill block frame 2 |
| `MO_CHAINCOMBO` | `0x60` skill | 5 | plus `EF_TEIHIT1`, `EF_GUMGANG3`, `EF_CHAINCOMBO` |
| `MO_COMBOFINISH` | `0x60` skill | 1..2, loop 2 | quake + `mon_맹룡과강.wav` + `EF_GUMGANG3`/`EF_HITLINE` |
| `CH_PALMSTRIKE` | `0x60` skill | 3..4, loop 4 | quake + `맹호경파산.wav` + `EF_HITLINE2` |
| `CH_TIGERFIST` | `0x60` skill | 4 | quake + `EF_BASH3D2`/`EF_GUMGANG3` |
| `CH_CHAINCRUSH` | — | none | `EF_CHEMICAL2DASH` at the target |

Client mapping today: 271 and 263 use the attack action, the other Champion
skills the skill action; the Asura effect data (rings + six sequential dark
kanji over the cast, impact flash/sound) is in
`tools/data/skill_effects_overrides.ron`.
