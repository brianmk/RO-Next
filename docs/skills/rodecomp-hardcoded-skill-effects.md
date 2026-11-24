# Hard-coded (C++) skill effects for the 2019 effect-id gap

Date: 2026-11-24. Branch `feat/gap-skillfx`.

The 2019 GRF `SKILL_EFFECT_INFO_LIST` gives 43 skills an official effect id, but
the extracted effect-table layer facts (`tools/data/windows_client_effect_layers.json`)
have no layers for those ids: the complete visuals are implemented in
`CondaRO.exe` (`CRagEffect::OnProcess*` switches and the `CEffect` subclasses
registered in `EffectLists::RegisterAllocators`) and were never part of the
effect table. This lane extracts them from the read-only decompilation
(`/home/phantom/Documents/deepseek-harness/default-workspace/rodecomp`) and merges
them into the unified pipeline.

## Data flow

```
tools/data/rodecomp_effect_layers.json          facts: effect id -> official-schema layer facts,
                                           implementation file:line provenance, scope, skips
        |
        v
tools/gen_rodecomp_effect_layers.py        validator + converter (reuses
        |                                  gen_skill_effect_layers.build_layer: same
        |                                  anchors, whitelists, unit conversions, coalescing)
        v
tools/data/skill_effects_rodecomp_hardcoded.ron 41 skills / 41 phases / 60 layers
        |
        v
tools/gen_skill_effects.py --rodecomp-hardcoded   merge (additive, asset-deduplicated)
        |
        v
korangar/archive/data/client/skill_effects.ron
```

The fragment is read with the same reader as the official fragment, so its rows
are validated by the Rust loader (`korangar/src/world/library/skill_effects.rs`)
exactly like the official ones.

## Conversion rules

Identical to `tools/gen_skill_effect_layers.py`:

* anchors/phases come from the 2019 GRF `SkillEffect` field
  (`caster` → `successEffectIdOnCaster` → `Success`/`Caster`, `target` →
  `target`/`Target`, `ground` → `Ground`/`Ground`, `begin` → `Cast`/`Caster`);
* `.str` names are relative to `data\texture\effect\`, sprites to
  `data\sprite\`, textures to `data\texture\`;
* official sizes are effect-table units: billboards `size * 0.85`, projectiles
  `size * 0.09`, positions/radii `* 2.0`; `alphaMax`/`red`/`green`/`blue` are
  0..1; `blendMode 2` → `Additive`;
* `.str`/`.spr` names are case-folded to the GRF spelling; every asset was
  validated against `CondaRO.grf`/`data.grf`/`palettes.grf` (GRF names decoded
  UTF-8 → EUC-KR → cp949, the encoding the client loader accepts).

## Effect ids handled (40 ids)

| id | effect | layer | source |
| ---: | --- | --- | --- |
| 330 | EF_HITLINE | `effect\white02.bmp` ×4 | RagEffect.cpp:10218 → WhiteSparks 10837 |
| 547 | EF_STIN | `effect\270바람.tga` ×4 | RagEffect.cpp:21810 → TexturedSlashFan 3980 |
| 864 | EF_ENERVATION7 | `effect\ac_center2.tga` ×16 | RagEffect.cpp:20081 → RisingSparkles 7078 / TintedRisingSparkles 7977 |
| 981 | EF_THROW_HAPPOKUNAI | `쿠나이_독.bmp` projectile ×8 | ThrowEffects.cpp:149/190/216 |
| 982 | EF_THROW_MULTIPLE_COIN | `effect\coin_a.bmp` projectile ×10 | ThrowEffects.cpp:192/238 |
| 983 | EF_THROW_BAKURETSU | `쿠나이_독.bmp` projectile ×4 | ThrowEffects.cpp:209-216 (shared tail) |
| 984 | EF_ROTATE_HUUMARANKA | `쿠나이_독.bmp` ×4 | RotateItemEffect.cpp:37/49 |
| 986 | EF_ROTATE_LINE_GRAY | `effect\cloud11.tga` ×4 | RotateLineEffect.cpp:36/50/66 |
| 990 | EF_GROUND_EXPLOSION | `이팩트\m_ef03` | SpriteEffect.cpp:65/76 |
| 991 | EF_KG_KAGEHUMI | `effect\SquareRange.tga` | SquareRangeEffect.cpp:34-60 |
| 996 | EF_KO_JYUMONJIKIRI | `jyumonjikiri.str` | EZ2STREffect.cpp:1812 |
| 997 | EF_KO_SETSUDAN | `setsudan.str` | EZ2STREffect.cpp:1815 |
| 999 | EF_KO_IZAYOI | `ez_cure.str` | EZ2STREffect.cpp:1821 |
| 1000 | EF_ROTATE_LINE_BLUE | `effect\cloud11.tga` ×4 | RotateLineEffect.cpp:39-66 |
| 1001 | EF_KG_KYOMU | `허무의그림자.str` | EZ2STREffect.cpp:1845 |
| 1002 | EF_KO_HUUMARANKA | `이팩트\sakura01` ×16 | ScatterEffect.cpp:29-32 |
| 1008 | EF_KAIHOU1 | `술식해방.str` | EZ2STREffect.cpp:1824 |
| 1011 | EF_GENSOU | `흐린달빛환상2.str` | EZ2STREffect.cpp:1848 |
| 1066 | EF_BANISHING_BUSTER | `RL_BANISHING_BUSTER\vanishing1.str` | EZ2STREffect.cpp:1880 |
| 1067 | EF_SLUGSHOT | `RL_SLUGSHOT\slug.str` | EZ2STREffect.cpp:1883 |
| 1068 | EF_D_TAIL | `RL_d_tail\dtdt.str` | EZ2STREffect.cpp:1886 |
| 1078 | EF_S_STORM | `RL_STORM\S_STORM.str` | EZ2STREffect.cpp:1904 |
| 1098 | EF_FRESHSHRIMP | `su_freshshrimp\su_freshshrimp.str` | EZ2STREffect.cpp:1937 |
| 1123 | EF_SU_GROOMING | `su_grooming\su_grooming.str` | EZ2STREffect.cpp:1970 |
| 1124 | EF_SU_CHATTERING | `su_chattering\su_chattering.str` | EZ2STREffect.cpp:1973 |
| 1134 | EF_RICHS_COIN_A | `RL_richs_coin_A\reachs.str` | EZ2STREffect.cpp:1999 |
| 1137 | EF_E_CHAIN | `RL_e_chain\eternal.str` | EZ2STREffect.cpp:2005 |
| 1138 | EF_HEAT_BARREL | `RL_heat_barrel\htb.str` | EZ2STREffect.cpp:2008 |
| 1139 | EF_H_MINE | `RL_h_mine\mine.str` | EZ2STREffect.cpp:2011 |
| 1140 | EF_FALLEN_ANGEL | `RL_fallen_angel\Penne.str` | MonsterSkillEffects.cpp:1018 |
| 1149 | EF_IMMUNE_PROPERTY | `effect\guardK.tga` + `guardK2.tga` | RagEffect.cpp:22219 → KyrieGuard 4502 |
| 1150 | EF_MOVE_COORDINATE | `whitelight.tga`, `peong1/2/3.tga` | RagEffect.cpp:22222 → WhiteLightTrail 13451 / TexturedScatterArc 5552 |
| 1197 | EF_LIGHTSPHERE_SUN | `thunder_center.bmp` + `white02.bmp` | LightSphereEffect.cpp:30/169 |
| 1198 | EF_LIGHTSPHERE_MOON | `thunder_center.bmp` + `white02.bmp` | LightSphereEffect.cpp:30/169 |
| 1199 | EF_LIGHTSPHERE_STAR | `thunder_center.bmp` + `white02.bmp` | LightSphereEffect.cpp:30/169 |
| 1202 | EF_NOVAEXPLOSING | `star_emperor\boom.tga` + `ring_white.tga` | StarEmperorEffect.cpp:32/47/114 |
| 1218 | EF_SOLARBURST | `solar_burst\solar_burst.str` | EZ2STREffect.cpp:2035 |
| 1219 | EF_SJ_DOCUMENT | `sj_document\sj_document.str` | EZ2STREffect.cpp:2038 |
| 1220 | EF_FALLING_STAR | `falling_star\falling_star.str` | EZ2STREffect.cpp:2041 |
| 1229 | EF_NEWMOON_KICK | `newmoon_kick\newmoon_kick.str` | EZ2STREffect.cpp:2050 |

Skills without a 2019 effect id whose visual the client spawns from the
skill-unit/status code are spelled out in the same fact file (the merge adds them
as descriptors too):

| skill | layer | source |
| ---: | --- | --- |
| 2555 RL_B_TRAP | `effect\rl_richs_coin_b\rcoin1.bmp` ×5 + `light.bmp` | GameModePackets.cpp:4964 / MonsterSkillEffects.cpp:2878 |
| 2560 RL_C_MARKER | `RL_C_MAKER\deffender.str` + `cm.str` | GameObject.cpp:3559-3570 |
| 2567 RL_FIRE_RAIN | `RL_FIRE_RAIN\fire_rain.str` | MsgEffect.cpp:1280 / EZ2STREffect.cpp:1877 |
| 2569 RL_AM_BLAST | `RL_anti_material\anti.str` | GameObject.cpp:3570 / EZ2STREffect.cpp:1999 |
| 2606 / 2609 | `BodyFlash` tint (155,155,255) | RagEffect.cpp:22272 (EF_BLINK_BODY) |

## Effect ids and skills left out (with the reason)

| id/skill | reason |
| --- | --- |
| 989 EF_KAIHOU | `CThrowItemEffect` picks `WaterOrb.bmp`/`StoneOrb.bmp`/`FireOrb.bmp`/`LightningOrb.bmp` from the caster's runtime charm type (ThrowEffects.cpp:249-262); no fixed asset. Skill 3019 still gets its fixed target rows (1008). |
| 1204 EF_SMA_BLACK | RagEffect.cpp:22237 → `CRagEffect::SMA` 2621 builds PP_SMA primitive 120 with `totalTexture = 0`; the slash is `RenderSMA3` hard-coded geometry, no texture/sprite/str layer. |
| 1209 EF_BLINK_BODY | RagEffect.cpp:22272 only flips the caster body tint (155,155,255 / 50,50,50); represented as `BodyFlash` for 2606/2609 (its only two users). |
| 2602 | Its only effect, 1204, is the textureless PP_SMA primitive: no data layer can express it, so the skill stays without a descriptor. |
| 3020 KO_ZENKAI | The 2019 table lists no effect id. `CCellRangeEffect` (CellRangeEffect.cpp:39-49) renders `water-%d`/`stone-%d`/`fire-%d`/`lightning-%d` for the server-selected rune element; no single fixed asset, and spawning all four would be wrong. |

## Audit

`python3 tools/audit_skill_parity.py`:

| check | before | after |
| --- | --- | --- |
| no client descriptor | 43 | **2** (2602, 3020) |
| missing effect assets | 0 | **0** |
| missing official sound | 0 | **0** |
| caster action mismatch | 0 | **0** |

Drift guards: `gen_rodecomp_effect_layers.py --check`,
`gen_skill_effect_layers.py --check`, `gen_skill_effects.py --check`,
`gen_rodecomp_overlay.py --check`. Tests:
`tools/test_rodecomp_hardcoded_effect_layers.py`.
