
---

## Skill/effect parity release (2026-11-24)

The RO-Next payload was updated with the skill-animation and effect work
(commit `d2b3c1f6` and the preceding effect/action commits):

| item | value |
| --- | --- |
| payload version | `0.2.29` (`ro-next-files/ro-next.version`, config `clients.ro_next.version`) |
| content release | `20261124-201226` (`/home/phantom/patcher-releases/ro_next/`) |
| files in manifest | 87 (exe + overlay data + new damage-font sprites) |
| classic client | untouched |

Updated files under `archive/data/client/` (skill_effects, skill_actions,
skill_sounds, status_effects, status_visual_effects, weapons, jobs, keybinds,
paper_doll_slots, robes, shields), `archive/data/languages/*.ron` and the new
`archive/data/sprite/이팩트/` sprites; `RO-Next.exe` rebuilt from `d2b3c1f6`
with `cargo xwin --target x86_64-pc-windows-msvc`.

Publish procedure used:

```bash
rsync -a ro-next/archive/data/client/ rathena:.../ro-next-files/archive/data/client/
scp target/x86_64-pc-windows-msvc/release/ro-next.exe rathena:.../ro-next-files/RO-Next.exe
ssh rathena 'echo 0.2.29 > .../ro-next-files/ro-next.version'
ssh rathena '/home/phantom/ragnarok-web/scripts/publish-content-release.sh --client ro_next --reason "..."'
```

The script snapshots the payload, regenerates the manifest/packages, switches
`current-ro_next` atomically and smoke-tests the live endpoints (manifest 200,
download ok).

---

## Cleanup + static-CRT release (2026-11-27, payload 0.2.32)

| item | value |
| --- | --- |
| client commits | `ec115255`..`6ab83d13` (cleanup, descriptor-layer suppression, stranded-skill mappings, Windows cfg fixes, static CRT) |
| payload version | `0.2.32` (`ro-next-files/ro-next.version`, config `clients.ro_next.version` / `clientVersion`, and `ro-next/Cargo.toml` so `--version` agrees) |
| content release | `20261127-070846` (`/home/phantom/patcher-releases/ro_next/`) |
| files in manifest | 91 (was 87: the four missing data files below are now shipped) |
| package | `archive_759dab1c.zip`, 82 entries, 3.85 MB |
| previous release | `20261127-064254` (0.2.31, kept for rollback) |
| classic client | untouched |

### What the 0.2.29 payload was missing

The 2026-11-24 payload predates the damage-number, hit-feedback, monster-motion and
status-launch work, so the shipped client looked for four data files that were not in the
manifest (the loaders log a warning and the feature silently falls back):

* `archive/data/client/damage_numbers.ron`
* `archive/data/client/hit_feedback.ron`
* `archive/data/client/monster_skill_actions.ron`
* `archive/data/client/status_effect_launches.ron`
* and the 2026 `archive/data/sprite/이팩트/숫자.act` (the `.spr` was shipped, the matching
  `.act` was not, so the GRF's 2019 clip was used with the 2026 atlas)

The payload also carried stale copies of `skill_effects.ron`, `skill_actions.ron`,
`skills.ron`, `status_effects.ron`, `skill_sounds.ron`, `weapons.ron`, `world_config.ron`,
`ui_audio.ron`, `game_archives.ron` and the `lua-scaffolding` files.

### Publish procedure (with the completeness gate)

```bash
# 1. build the Windows client (static CRT, see .cargo/config.toml for the triple)
cargo xwin build --release -p ro-next --target x86_64-pc-windows-msvc

# 2. sync the payload working tree (the release itself is immutable, so this is safe
#    until the publish step switches current-ro_next)
rsync -a --delete ro-next/archive/ \
    rathena:/home/phantom/ragnarok-web/patcher-server/ro-next-files/archive/
scp target/x86_64-pc-windows-msvc/release/ro-next.exe \
    rathena:/home/phantom/ragnarok-web/patcher-server/ro-next-files/RO-Next.exe
ssh rathena 'printf "0.2.32\n" > ~/ragnarok-web/patcher-server/ro-next-files/ro-next.version'

# 3. bump the live config (the manifest version and the patcher's "update available"
#    string come from clients.ro_next.version / clientVersion)
#    config: ~/ragnarok-web/patcher-server/config/patcher-server.json
#    (back it up first; publish-content-release.sh only rewrites the directories)

# 4. dry-run, then publish
ssh rathena '/home/phantom/ragnarok-web/scripts/publish-content-release.sh \
    --client ro_next --reason "..." --dry-run'
ssh rathena '/home/phantom/ragnarok-web/scripts/publish-content-release.sh \
    --client ro_next --reason "..."'

# 5. completeness gate: every repo file and the built exe must match the live manifest
scripts/patcher-payload-check.py \
    --exe target/x86_64-pc-windows-msvc/release/ro-next.exe --expect-version 0.2.32
```

`scripts/patcher-payload-check.py` fetches `<api>/manifest?client=ro_next` and fails on any
missing or stale entry (it maps `ro-next/archive/**` to `archive/**`, compares the exe against
`RO-Next.exe`, and treats `client/game_archives.ron` / `defaults/interface_settings.ron` as
deployment-specific presence-only files). This is the check that would have caught the missing
data files; run it after every publish.

### Verification performed for 0.2.32

* manifest: `version=0.2.32`, `release_id=20261127-070846`, 91 files; `payload-check` reports
  `OK: the payload ships every file this repo expects`.
* downloads: the four new `.ron` files, `숫자.act` and `RO-Next.exe` downloaded from the live API
  and hash-compared against the manifest (all OK; the download endpoint needs a patcher-style
  `User-Agent`, a bare `python-urllib` UA gets a 403 from the WAF).
* the published `RO-Next.exe` was downloaded through the patcher API
  (`sha256:d5759683…`), byte-identical to the local build, reports `RO-Next 0.2.32` and logs
  `client: ported effect backend enabled` on startup (the default; `RO_NEXT_PORTED_FX=0` opts
  back into the descriptor approximations).
* batch package: `archive_759dab1c.zip` (82 entries) contains all the new files, so the patcher's
  package path ships them too.
* update-plan signing: `signing.enabled=true`, key `patcher-2026-11` matches the key pinned in
  `patcher-client.json`, and `patcher_sign_plan` produces a signature that
  `sodium_crypto_sign_verify_detached` accepts for the new release (verified on the host).
* shipped data parses with the shipped Windows binary under Wine: `--check-skill-effects`
  (591), `--check-status-effects` (616), `--check-jobs` (181), `--check-items` (6169),
  `--check-status-visuals` (17), `--check-keybinds` (39), `--check-paper-doll` (21),
  `--check-aliases`, `--check-messages` (87) all exit 0 against the synced payload files.

### Effect backend

0.2.32 is the first release that runs the **ported (1:1 official) effect backend by default**:
every mapped `(skill, phase)` pair (249 mappings) spawns its ported body, and the descriptor
layers of that pair are suppressed, so a mapped cast is drawn once. `RO_NEXT_PORTED_FX=0` returns
to the descriptor approximations without a rebuild. Known follow-up: a mapped phase can play its
descriptor `wav` *and* the body's own official `PlayWave` (41 of the 219 mapped pairs have both);
the sound precedence is the next item, the visuals are unaffected.

### Windows client notes

* The exe is now linked with `+crt-static` for `x86_64-pc-windows-msvc`, so it no longer imports
  `VCRUNTIME140.dll` (Windows 10/11 ship the UCRT but not the VC++ redistributable). Verified
  with `objdump -p`: only in-box DLLs remain. `dxcompiler.dll` + `dxil.dll` are still shipped for
  the DX12 shader path; wgpu falls back to `d3dcompiler_47.dll` when they are absent.
* CPU baseline is `x86-64-v3` (AVX2/BMI2) from `.cargo/config.toml` — unchanged from previous
  releases, but worth remembering when a player reports "illegal instruction" on old hardware.
* `--help` / `--check-*` print to stdout; the patcher redirects it, a plain double-click shows
  nothing (GUI subsystem).
