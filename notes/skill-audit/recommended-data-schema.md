# Proposed data schema: consolidated multi-layer skill effect descriptor

Status: proposal for the pipeline/data lanes. Nothing here is implemented in the client yet.
Motivation and evidence: `notes/skill-audit/summary.md`; raw facts: `notes/skill-audit/skill_matrix.json`.

## Why

The current client spreads one skill's visual across **five files plus two Rust modules**:

| Source | Rows / entries | What it can express |
|---|---:|---|
| `korangar/archive/data/client/skill_effect_assets.ron` (`.str`) | 189 | one file per (skill, phase), binary `overhead` flag |
| `korangar/archive/data/client/skill_sprite_effects.ron` (`.spr`/`.act`) | 48 | files, offset, scale, duration |
| `korangar/archive/data/client/skill_procedural_effects.ron` | 265 | one normalised primitive + a fixed parameter set |
| `korangar/archive/data/client/skill_sounds.ron` | 369 | sound priority list |
| `korangar/archive/data/client/skill_actions.ron` | 205 | caster action |
| `skill_cast_effect.rs` `skill_visuals`/`has_custom_completion` | 8 hand-written skills | custom assets, auras, body flash, cast circle |
| `effect/status_aura.rs` | 2 EFST ids | persistent status glow |

This split causes the audit's systemic findings:

* `has_custom_completion` bypasses **all** data rows for 8 skills (`lib.rs:2457`), so the
  generated `crashearth.str` / `ac_center2.tga` / `agi_up.bmp` / `축복` layers never play
  (skills 29, 34, 110 — see `summary.md` §3.1).
* The official per-layer `head` / `yOffset` flags are replaced by a hand-curated skill-id set
  (`OVERHEAD_SKILLS`), which misses 6 of the 9 official `head:true` skills (`summary.md` §3.4).
* The official layer order and per-layer timing/metadata are lost: the client model is
  explicitly "one normalised primitive", so multi-layer effects are collapsed or missing
  (e.g. Fireball's 4 `fireball.spr` layers, Sightrasher's 8 `sight.spr` layers).

A single descriptor that keeps the official **skill → effect ids per phase → ordered layer
list** shape fixes all three: the generator writes exactly the official layers, and the
client only has to play a list of layers.

## Shape

```
skill
 └─ phase (Ground | Cast | Success | Release | Hit | Caster)
     ├─ effect_ids: [official ids that contribute to this phase]
     └─ layers: [ ordered, highest priority first
          {
            kind: STR | SPR | TEXTURE | FUNC | SOUND,
            asset: "data\\texture\\effect\\angelus.str",
            anchor: Caster | Target | Travel | Ground | Auto,
            offset: [x, y, z]           # extra world-unit offset after anchor/overhead
            scale: 1.0,
            duration_ms: 0,             # 0 = native (STR/SPR) length
            sound: "data\\wav\\effect\\ef_angelus.wav" | null,
            overhead: true | false,     # entity-head anchor (official `head`)
            attached: true | false,     # follows the entity (`attachedEntity`)
            body: true | false,         # body/weapon aura vs head anchor
            repeat: false,
            delay_ms: 0,                # official delayOffset/delayLate/delayFrame
            duplicate: 1,
            alpha: 1.0,
            colour: [r, g, b],
            additive: true,
            source: "EffectTable#41"
          }
     ]
 └─ status: { efst: 110, efst_id: 110, aura: "assumptio" } | null
 └─ caster_action: Idle | Skill | Attack | ReadyFight | Pickup
```

### Field reference

| Field | Type | Source | Notes |
|---|---|---|---|
| `skill` | u16 | rAthena `skill_db.yml` `Id` | key of the entry |
| `phase` | enum | official `SkillEffect` field | `groundEffectId*`→Ground, `beginCastEffectId*`/`castAuraEffectId`→Cast, `effectId*`/`successEffectId*`→Success, `releaseEffectId`/`beforeHitEffectId*`→Release, `hitEffectId*`/`damageEffectIdOnTarget`→Hit, `damageEffectIdOnCaster`→Caster |
| `effect_ids` | `[u16]` | `tools/windows_client_skill_effects.json` | provenance for the layers; keeps the audit traceable |
| `kind` | enum | EffectTable `type` | `3D`/`2D`/`CYLINDER`/`QuadHorn`… → `TEXTURE`; `STR`→`STR`; `SPR`→`SPR`; `FUNC`/`PRIM`/`RSM`→`FUNC`; wav-only → `SOUND` |
| `asset` | path | EffectTable `file`/`textureName`/`spriteName` | **decoded EUC-KR**; validated against the GRFs at generation time; `null` for `FUNC`/`SOUND` |
| `anchor` | enum | field + `attachedEntity` | `attached:true` on an Auto phase → `Caster`, `false` → `Target`; Ground phases → `Ground` |
| `offset` | `[f32;3]` | `yOffset`, `posx/posy/posz*` | **extra** offset after `anchor`/`overhead`; `yOffset` is screen px → world ≈ ×0.25. `head:true` selects the shared head anchor via `overhead`, so no manual head height is stored |
| `scale` | f32 | `size`/`sizeStart`/`sizeEnd` + defaults | client-side tunable |
| `duration_ms` | u32 | `duration` | order matters for duplicates/delays |
| `sound` | path | entry `wav` | one sound per layer; play priority = layer order |
| `overhead` | bool | **official `head`** | replaces `OVERHEAD_SKILLS` |
| `attached` | bool | official `attachedEntity` | entity vs ground/free-standing |
| `body` | bool | derived: `attached && head:false && variant==Success` | body/weapon aura keeps the torso anchor (`BODY_AURA_SKILLS` becomes data, not code) |
| `repeat` | bool | official `repeat`/`stackable` | e.g. Blessing SPR `축복` |
| `delay_ms` | u32 | `delayOffset`/`delayLate`/`delayFrame` | |
| `duplicate` | u16 | `duplicate` | repeated spawns (ac_center2 ×7/×3/×10, particle6 ×6) |
| `alpha`, `colour`, `additive` | | `alphaMax`, `red/green/blue`, `blendMode` | renderer hints |
| `status` | object | rAthena `Status:` + status.yml `Icon` | persistent EFST aura hook |

## Canonical examples

### Angelus (33) — official effect 41, `head:true`, single `.str` layer

```ron
SkillEffectDescriptor(
    skill: 33,
    phases: [
        Phase(
            phase: Success,
            effect_ids: [41],
            layers: [
                Layer(
                    kind: STR,
                    asset: "data\\texture\\effect\\angelus.str",
                    anchor: Auto,
                    offset: (0.0, 0.0, 0.0),   // no extra offset, head anchor via overhead
                    scale: 1.0,
                    duration_ms: 0,
                    sound: "data\\wav\\effect\\ef_angelus.wav",
                    overhead: true,
                    attached: true,
                    body: false,
                ),
            ],
        ),
    ],
    status: Some(StatusVisual(efst: "EFST_ANGELUS", efst_id: 9, aura: None)),
    caster_action: Skill,
)
```

### Lex Aeterna (85) — official effect 85, `attachedEntity:true`, no head

```ron
Phase(
    phase: Success,
    effect_ids: [85],
    layers: [
        Layer(
            kind: STR,
            asset: "data\\texture\\effect\\lexaeterna.str",
            anchor: Auto,
            offset: (0.0, 0.0, 0.0),
            overhead: false,
            attached: true,
            body: true,                      // body/weapon glow, not head
            sound: "data\\wav\\effect\\priest_lexaeterna.wav",
        ),
    ],
)
```

### Assumptio (361) — `.str` one-shot + persistent EFST 110/473 aura (reference working case)

```ron
SkillEffectDescriptor(
    skill: 361,
    phases: [
        Phase(phase: Success, effect_ids: [440], layers: [
            Layer(kind: STR, asset: "data\\texture\\effect\\asum.str", anchor: Auto,
                  overhead: false, attached: true,
                  sound: "data\\wav\\effect\\아숨프티오.wav"),
        ]),
    ],
    status: Some(StatusVisual(
        efst: "EFST_ASSUMPTIO", efst_id: 110,
        aura: Some(StatusAura(
            ground_texture: "data\\texture\\effect\\round_sphere.bmp",
            ring_texture: "data\\texture\\effect\\efst_waterfield2\\bubble1.bmp",
            body_texture: "data\\texture\\effect\\round_sphere.bmp",
            colour: [0.55, 0.85, 1.0], radius: 8.0, body_size: 9.0,
            pulse: 0.28, alpha: 0.32,
        )),
    )),
)
```

### Blessing (34) — the multi-layer case the schema is designed for

```ron
Phase(
    phase: Success,
    effect_ids: [42],
    layers: [
        Layer(kind: SPR, asset: "data\\sprite\\이팩트\\축복.spr", anchor: Caster,
              offset: (0.0, 0.0, 0.0),   // head anchor via overhead; yOffset -120 is the official head offset
              duration_ms: 1500, delay_ms: 990,  // delayFrame 30 ≈ 990 ms
              overhead: true, attached: true, repeat: true, scale: 1.0,
              sound: "data\\wav\\effect\\ef_blessing.wav"),
        Layer(kind: TEXTURE, asset: "data\\sprite\\이팩트\\particle6.spr", anchor: Caster,
              duration_ms: 1200, delay_ms: 300, duplicate: 6, alpha: 1.0,
              overhead: false, attached: true, additive: true),
        Layer(kind: TEXTURE, asset: "data\\sprite\\이팩트\\particle6.spr", anchor: Caster,
              duration_ms: 1200, delay_ms: 400, duplicate: 6, alpha: 1.0,
              overhead: false, attached: true, additive: true),
        Layer(kind: TEXTURE, asset: "data\\texture\\effect\\pok2.tga", anchor: Target,
              duration_ms: 2500, alpha: 0.3, colour: [0.1, 0.75, 1.0],
              overhead: false, attached: false, additive: true),
    ],
)
```

## Migration

1. **Generator** (`gen_skill_effect_assets.py` + `gen_skill_sprite_effects.py` +
   `gen_skill_procedural_effects.py`): emit one layer list per (skill, phase) instead of three
   separate files. Every layer carries `head`, `attachedEntity`, `duration`, `delay`,
   `duplicate`, `wav` from the EffectTable entry (already parsed by
   `parse_effect_table`-style code; the audit's `notes/skill-audit/audit_core.py` shows the
   full-field parser). Keep the existing GRF/whitelist validation per layer.
2. **Client**: replace the three lookups in `spawn_skill_effect_variants` with one iteration
   that dispatches on `kind` to the existing play paths (`spawn_entity_effect` for `.str`,
   sprite-effect player for `.spr`, primitive renderer for `TEXTURE`). `overhead` becomes a
   per-layer property consumed directly, so `OVERHEAD_SKILLS` / `BODY_AURA_SKILLS` are deleted.
3. **Custom visuals**: represent each hand-written arm as explicit layer rows (Blessing =
   the four official layers; Heal's target cylinders = `TEXTURE/CYLINDER` layers). Then
   `has_custom_completion` only remains for genuinely code-driven effects (Zen body flash,
   Fury cones) and no generated row is silently dead.
4. **Status auras**: the `status` block references a named aura spec; adding a buff aura is a
   data change, not a Rust change (the registry is already designed for this,
   `status_aura.rs` "Adding another status glow").
5. **Sounds / actions**: fold `sound` into the layer (play order = layer order) and keep
   `caster_action` as a top-level field; the standalone `skill_sounds.ron` / `skill_actions.ron`
   can stay as generated compatibility views until the loader is switched.

Compatibility: the descriptor can be emitted alongside the current files first
(`skill_effects.ron` as a fourth generated file), so the client can migrate the loader
behind a flag without a data freeze.

## Data fixes this schema should ship with (from the audit)

* Official `head` propagation: 59/62/357 currently render at the feet; Angelus (33) /
  Blessing (34) are only correct because of hand-written arms.
* Enable the generated rows for 29/34/110 (remove `has_custom_completion` arms) and use
  `crashearth.str` for Hammer Fall.
* Keep `firstHitEffectId` and effect id 0 in the extractor so the standard hit flash and the
  lens flash of MG_SOULSTRIKE / AC_DOUBLE are in the data.
* Emit sprite layers (`fireball.spr`, `sight.spr`, `particle3.spr`, `축복.spr`) that the
  current split files cannot express.
* Expand `%d` wav templates (`_hit_fist%d`, `ef_icearrow%d`, `ef_firearrow%d`).

## Validation rules (generator)

1. `asset` (when set) must exist in the GRF name tables (`CondaRO.grf`, `data.grf`,
   `palettes.grf`); for `.spr` the matching `.act` must exist too.
2. `sound` (when set) must exist as `data\wav\...wav`; `%d`/`%s` templates match any concrete
   file and are kept as templates.
3. A layer must have at least one of `asset` or `sound`, or `kind == FUNC`.
4. Phases are sorted in the canonical order Ground, Cast, Success, Release, Hit, Caster; layers
   keep the official order within a phase.
5. `overhead: true` requires an entity anchor (`Caster`/`Auto`/`Target`).
6. `--check` drift guard as today: regenerate and diff.
