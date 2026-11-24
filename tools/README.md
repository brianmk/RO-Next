# Data pipeline tools

Generators and extractors behind the runtime tables in
`korangar/archive/data/client/`. Each table is produced from a committed facts
file, so regenerating and checking it never needs the game archives or the
server tree.

## Layout

```
tools/
  extract_*.py      read rAthena / the GRF Lua tables / the decompiled client -> tools/data/*.json
  gen_*.py          tools/data/*.json -> korangar/archive/data/client/*.ron (or a .rs table)
  audit_*.py        compare the runtime tables with the official sources
  test_*.py         unittest suites for the generators and the emitted tables
  data/             committed facts, curated overlays and intermediate fragments
```

Run every script **from the repository root** (paths are relative to it, exactly
as in CI). `tools/bin/` holds the local `slangc` shader compiler, not data.

## Tables

| runtime file | generator | inputs in `tools/data/` |
| --- | --- | --- |
| `aliases.ron` | `gen_aliases.py` | `alias_facts.json` (← `extract_alias_facts.py`), `alias_legacy_parity.json` |
| `item_stats.ron` | `gen_item_stats.py` | `rathena_pre_re_item_facts.json` (← `extract_item_facts.py`) |
| `jobs.ron` | `gen_jobs.py` | `job_facts.json` (← `extract_job_facts.py`), `rustnarok_actor_facts.json` |
| `weapons.ron` | `gen_weapons.py` | `rathena_pre_re_weapon_items.json`, `weapon_client_assets.json` |
| `messages.ron` | `gen_messages.py` | `message_facts.json` (← `extract_message_facts.py`) |
| `skills.ron` | `gen_skills.py` | `skills_custom.ron` (curated) |
| `skill_effects.ron` | `gen_skill_effects.py` | merges the fragments below, in order |
| — official fragment | `gen_skill_effect_layers.py` | `windows_client_skill_effects.json`, `windows_client_effect_layers.json`, `windows_client_effect_whitelist.txt`, `client_skill_tables_2019.txt` → `grf2019_skill_facts.json`, `rathena_pre_re_learnable_skills.json` |
| — decompilation overlay | `gen_rodecomp_overlay.py` | `rodecomp_skill_facts.json` (← `extract_rodecomp_sounds.py` for sounds) |
| — hard-coded effects | `gen_rodecomp_effect_layers.py` | `rodecomp_effect_layers.json`, `windows_client_effect_whitelist.txt` |
| — curated rows | (hand-written) | `skill_effects_custom.ron`, `skill_effects_overrides.ron` |
| `skill_actions.ron` | `gen_skill_actions.py` | `skill_actions_rodecomp.json`, `rodecomp_skill_frame_sequences.json` (← `extract_rodecomp_frame_sequences.py`), `windows_client_skill_actions.json`, `skill_actions_extra.json` |
| `skill_sounds.ron` | `gen_skill_sounds.py` | `windows_client_skill_sounds.json`, `grf2019_skill_facts.json`, `skill_sounds_custom.ron`, `skill_sounds_rodecomp.ron` |
| `skill_effect_assets.ron` | `gen_skill_effect_assets.py` | `windows_client_skill_effects.json`, `windows_client_effect_whitelist.txt` |
| `skill_procedural_effects.ron` | `gen_skill_procedural_effects.py` | `skill_procedural_components.csv` (← `extract_windows_client_procedural_effects.py`) |
| `skill_sprite_effects.ron` | `gen_skill_sprite_effects.py` | `windows_client_sprite_effects.json`, `windows_client_spr_whitelist.txt`, `windows_client_skill_sprite_name_matches.json` |
| `status_effects.ron` | `gen_status_effects.py` | `status_facts.json` (← `extract_status_facts.py` + `status_effect_custom.ron`), `status_info_robrowser.json` |
| `status_visual_effects.ron` | `gen_status_visuals.py` | `rustnarok_status_facts.json`, `status_facts.json` |
| `ragnarok-packets/src/opcode_lengths_2019.rs` | `gen_opcode_lengths.py` | `opcode_supplement_20190605.json`, rAthena packet docs |
| `korangar/src/world/library/skill_delays.rs` | `gen_skill_delays.py` | rAthena `db/pre-re/skill_db.yml` |

The `extract_*` half reads things outside this tree (rAthena via
`--server-root`/`--ssh`, the CondaRO GRFs, the decompiled client); see each
script's `--help`. Their committed JSON outputs make the `gen_*` half hermetic.

## Daily commands

```sh
# drift check for every table (CI runs the subset in .github/workflows/quality.yml)
for g in tools/gen_*.py; do python3 "$g" --check || echo "DRIFT: $g"; done

# audits against the official sources
python3 tools/audit_skill_parity.py       # GRF 2019 SKILL_EFFECT_INFO_LIST
python3 tools/audit_skill_actions.py      # official caster motion per skill
python3 tools/audit_asset_case.py         # asset path case/GRF resolution
python3 tools/audit_rodecomp_coverage.py  # decompilation effect coverage
python3 tools/audit_skill_animations.py   # phases/sounds/fallbacks (needs rAthena + GRFs)

# unit tests for the generators and emitted data
python3 tools/test_gen_weapons.py         # and the other test_*.py
```

## Conventions

* Generated files carry a `do not edit by hand` header with the exact
  regeneration and drift-check command; `--check` compares byte for byte.
* Hand-written content belongs in `*_custom.ron` / `*_overrides.ron`, never in a
  generated file. Each curated row needs a `source:` string explaining where the
  values come from.
* Every fact file keeps provenance (commit pins, archive hashes, source lines) so
  an audit can be reproduced later.
