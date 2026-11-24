# Documentation

Reference documentation for RO-Next (korangar) and its data pipeline. Everything
here describes the **current** state of the client; per-lane work reports and
superseded audits live in git history only.

## Skills and effects

| document | contents |
| --- | --- |
| [asura-strike-definition.md](skills/asura-strike-definition.md) | Official Asura Strike (`MO_EXTREMITYFIST`, 271) definition: `EF_BEGINASURA`, the two white rings, the six kanji, the caster frame sequence. |
| [rodecomp-skill-animations.md](skills/rodecomp-skill-animations.md) | Per-skill caster motion + ACT frame sequences extracted from the decompiled client (`GetSkillActionInfo`, `SetFrameSequence`). |
| [rodecomp-hardcoded-skill-effects.md](skills/rodecomp-hardcoded-skill-effects.md) | Effects the 2019 Lua tables do not carry because they are hard-coded in the client executable. |
| [rodecomp-skill-status-aura-audit.md](skills/rodecomp-skill-status-aura-audit.md) | Status effects, persistent auras and the decompilation-derived sources behind `status_effects.ron`. |
| [rodecomp-skill-drift-resolution.md](skills/rodecomp-skill-drift-resolution.md) | Per-skill decision record where the 2019 client and the 2026 decompilation use different effect ids. |
| [skill-effects-official-data.md](skills/skill-effects-official-data.md) | **Generated** layer report for `skill_effects.ron` (`tools/gen_skill_effect_layers.py`). |
| [skill-parity-grf2019-audit.md](skills/skill-parity-grf2019-audit.md) | Parity audit of all 66 skills in the official 2019 `SKILL_EFFECT_INFO_LIST` against our tables. |

Evidence screenshots for the status-aura work: [`skills/evidence/`](skills/evidence/).

## Client systems

| document | contents |
| --- | --- |
| [server-zeny-packet-flow.md](client/server-zeny-packet-flow.md) | How the server sends zeny (`ZC_LONGPAR_CHANGE`) and why the client starts at 0. |
| [bink-video-decoder-gap.md](client/bink-video-decoder-gap.md) | `.bik` movies: the AV1 side-car transcode and the placeholder fallback. |

## Pipelines

| document | contents |
| --- | --- |
| [rustnarok-facts-pipeline.md](pipelines/rustnarok-facts-pipeline.md) | Reproducible extraction of the rustnarok-client facts used by `jobs.ron` and `status_visual_effects.ron`. |

## Operations

| document | contents |
| --- | --- |
| [ro-next-patcher-integration.md](ops/ro-next-patcher-integration.md) | Shipping a build through the CondaRO patcher (payload layout, version bump, content release). |

## Related

* [`tools/README.md`](../tools/README.md) — the generators/extractors behind the
  `korangar/archive/data/client/*.ron` tables.
* [`wiki/Running.md`](../wiki/Running.md) — building and running the client.
