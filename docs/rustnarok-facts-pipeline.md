# rustnarok-client facts pipeline

This pipeline keeps the rustnarok-client evidence used by
[`docs/rustnarok-audit-2026-11-22.md`](rustnarok-audit-2026-11-22.md) reproducible.

## Pinned source

- Repository: <https://github.com/kindraywind/rustnarok-client>
- Commit: `76600595363affc96d5995bf4baa790d05a9593c`
- Local checkout: `/tmp/rustnarok-client` (re-cloned by the extractor when missing)
- License: Apache-2.0; the extractor copies only identifiers, table structure and
  one-line definitions — no game assets and no bulk copyrighted text.

## Committed facts

`tools/extract_rustnarok_facts.py` reads the pinned checkout and writes four
deterministic JSON documents (keys sorted, source lines included):

| file | contents |
| --- | --- |
| `tools/rustnarok_effects_facts.json` | effect ids → definitions/kinds/source lines, skill links, status links |
| `tools/rustnarok_motion_facts.json` | `ATTACK`, `THROW`, `THIRD_ATTACK`, `PICK_UP`, `KEEP`, `CAST`, `STATES` tables with source lines |
| `tools/rustnarok_status_facts.json` | status id → icon, looping visual effect ids |
| `tools/rustnarok_actor_facts.json` | 320 job sprite rows, cart rules (status 673, types, baby/super-novice), mount/outfit option bits, size statuses (1420/1421) |

The extractor verifies the checkout is exactly at the pinned commit; if it is
not, it fetches/checks out the pin and fails when that is impossible.

## Inventory regeneration

`tools/gen_rustnarok_inventory.py` reads those four facts files, reads our
read-only data files (`skill_effects.ron`, `skill_actions.ron`,
`status_effects.ron`, `tools/rathena_pre_re_learnable_skills.json`) and writes
`docs/rustnarok-inventory.json` with the original schema (`source`, `summary`,
`effects`, `skill_motions`, `statuses`) plus an `actor` summary section.

```sh
# refresh facts from the pinned checkout
./tools/extract_rustnarok_facts.py

# regenerate the inventory
./tools/gen_rustnarok_inventory.py

# CI / drift checks
./tools/extract_rustnarok_facts.py --check
./tools/gen_rustnarok_inventory.py --check
python3 -m py_compile tools/extract_rustnarok_facts.py tools/gen_rustnarok_inventory.py
```

## Refresh procedure

1. Bump `PIN` in `tools/extract_rustnarok_facts.py` to the reviewed rustnarok commit.
2. Run the extractor and inspect the facts diffs (new effect ids, status links,
   motion entries, job sprite rows, option bits).
3. Run the inventory generator and inspect `docs/rustnarok-inventory.json`.
4. Update the counts/portfolio in `docs/rustnarok-audit-2026-11-22.md` if the new
   pin changes the coverage numbers.
5. Commit facts + inventory + report together; CI must run both `--check` modes.
