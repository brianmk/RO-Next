# rustnarok-client facts pipeline

`rustnarok-client` is used as secondary evidence for data the official 2019
tables do not carry. Its four committed fact documents feed `gen_jobs.py`
(job sprites, cart/mount rules) and `gen_status_visuals.py` (status icon and
looping-effect ids).

## Pinned source

- Repository: <https://github.com/kindraywind/rustnarok-client>
- Commit: `76600595363affc96d5995bf4baa790d05a9593c`
- Local checkout: `/tmp/rustnarok-client` (re-cloned by the extractor when missing)
- License: Apache-2.0; the extractor copies only identifiers, table structure and
  one-line definitions — no game assets and no bulk copyrighted text.

## Committed facts

`tools/extract_rustnarok_facts.py` reads the pinned checkout and writes four
deterministic JSON documents (keys sorted, source lines included):

| file | contents | consumed by |
| --- | --- | --- |
| `tools/data/rustnarok_effects_facts.json` | effect ids → definitions/kinds/source lines, skill links, status links | reference only |
| `tools/data/rustnarok_motion_facts.json` | `ATTACK`, `THROW`, `THIRD_ATTACK`, `PICK_UP`, `KEEP`, `CAST`, `STATES` tables with source lines | reference only |
| `tools/data/rustnarok_status_facts.json` | status id → icon, looping visual effect ids | `gen_status_visuals.py` |
| `tools/data/rustnarok_actor_facts.json` | 320 job sprite rows, cart rules (status 673, types, baby/super-novice), mount/outfit option bits, size statuses (1420/1421) | `gen_jobs.py` |

The extractor verifies the checkout is exactly at the pinned commit; if it is
not, it fetches/checks out the pin and fails when that is impossible.

## Checks

```sh
./tools/extract_rustnarok_facts.py --check   # facts still match the pinned checkout
python3 -m py_compile tools/extract_rustnarok_facts.py
python3 tools/gen_jobs.py --check            # consumers still in sync
python3 tools/gen_status_visuals.py --check
```

## Refresh procedure

1. Bump `PIN` in `tools/extract_rustnarok_facts.py` to the reviewed rustnarok commit.
2. Run the extractor and inspect the facts diffs (new effect ids, status links,
   motion entries, job sprite rows, option bits).
3. Regenerate the consumers (`gen_jobs.py`, `gen_status_visuals.py`) and inspect
   their diffs.
4. Commit facts + regenerated tables together; both `--check` modes must pass.
