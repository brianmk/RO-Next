
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

---

## Defect batch release (2026-11-27, payload 0.2.33)

| item | value |
| --- | --- |
| payload version | `0.2.33` (`ro-next/Cargo.toml`, `ro-next.version`, config) |
| content release | `20261127-075517` |
| previous release | `20261127-070846` (0.2.32, rollback) |
| client commits | `97d7e69a`, `0c3a5694`, `601935ce`, `eb20bd1f`, `bbafa393`, `f18ca7cb` |
| tests | 2248 passed, 0 failed, 21 ignored |

Four user-reported defects, all decompilation-backed:

1. **Champion poses** (`bbafa393`): `CasterAction::Skill` played the whole ACT skill block
   (96..103), whose frames 1..2 are exactly Fury's, so Blessing/Inc Agi on a Champion looked like
   Fury. `CPc::SetState(ST_SKILL)` restricts the monk family to block 96 frame 0 for 400 ms
   (`Player.cpp:4477-4499`); `JobActionBase` maps `ST_SKILL`/`ST_BEGIN_SKILL` to 96 for the whole
   default table (`:1690-1744`). `skill_action.rs` now carries that job table and `set_skill_cast`
   applies it. The endless head movement was `stand()` using the 3-frame neutral block 0 with
   `looping = true`; only `AnimationActionType::Idle` was clamped to frame 0, so block 0's head
   bob looped forever after every skill that ends in `ST_STAND` (Inc Agi, Dangerous Soul Collect).
   Both are clamped now (`animation/mod.rs`).
2. **Increase AGI banner** (`97d7e69a`, `601935ce`): `holder.rs::quad_corners` returned corners
   `[TL,TR,BR,BL]` with UVs in the same order, while the renderer pairs them with
   `[BR,TR,TL,BL]` — a diagonal texture fold, and it omitted `EFFECT_ORIGIN`, drawing the quad
   ~319/291 px off the caster. Now a branch-for-branch port of `Render2DPrimitive`
   (`EffectPrimitiveRendering.cpp:7487-7589`), with `EffectRuntime::screen` (official
   `tlvertX/tlvertY`) actually written from the camera each frame and the 2-D quad at the
   reverse-z near plane.
3. **Effects under the floor** (`eb20bd1f`, `f18ca7cb`): the official simulation is Y-**down**
   (`Upward = {0,-d,0}`, `EffectPrimitiveMotion.cpp:84-88`), the port's world is Y-**up**
   (`loaders/map/vertices.rs:315-318`), and the ported bodies added official offsets unchanged —
   so every ground decal, portal disc and ring sat below the terrain and lost the depth test
   against the floor. The boundary is now explicit: engine anchors are port-space, everything the
   simulation consumes/produces is official-space, with `official_position`/`port_position` as the
   only converters (documented on `EffectRuntime::position`). Fixes Ruwach (bright motes above the
   floor with their shadow sticker on it), the warp NPC portal, the skill portals, and every other
   ported ground decal.
4. **Hotbar left click** (`0c3a5694`): `SkillBoxHandler::handle_click` armed a `MoveSkill` drag for
   both the skill tree and the hotbar, and only a drop back on the same slot cast — so a plain
   click never used the skill. A press now arms `PrepareSkillDrag`, and the release decides:
   same slot → `InputEvent::UseSkillInSlot` (the F-key path: self-cast immediately, targeted skills
   enter target selection), other slot → `InputEvent::MoveSkill`. Empty slots and the skill-tree
   drag flow are unchanged.

Remaining known follow-up: `EffectRuntime::terrain_height` is still never set in production (the
`MasterState` half already negates it); it is only needed by primitives that test the ground.

### Follow-up releases 0.2.34 / 0.2.35

* `0.2.34` (`20261127-080422`) — menu cleanup: Mail, Clan and Bank removed from the in-game menu
  (`interface/windows/menu.rs`); the merchant pushcart is drawn with a `-0.02` depth boost so the
  cart sorts behind the body/equipment instead of covering the character
  (`world/entity/common.rs`).
* `0.2.35` (`20261127-081120`) — auto-attack arrows point along their flight vector
  (`world/effect/arrow_effect.rs`). The quad rotation used the screen-space `atan2` (screen Y
  down) directly in `camera_facing_corners` (camera plane, up = +Y); the conventions mirror each
  other, so the arrow only pointed correctly for one heading and looked like it rotated with the
  camera. `quad_angle()` negates the screen angle and subtracts the sprite's authored 45-degree
  phase; a unit test asserts the axis for six headings.

### Follow-up release 0.2.36

* `0.2.36` (`20261127-100703`) — client-side `/where` and minimap coordinates.
  `/where` is answered locally with `Location: <map> (<x>, <y>)` and never forwarded: the
  rAthena `@where` atcommand is a GM tool that expects a *target character name*, so the old
  slash translation was useless to players (`client/input_chat.rs`, `input/chat_command.rs`).
  The minimap now reserves a 16 px row below the map image and shows the current map and tile
  (`prontera  156, 187`), recomputed from the minimap state every frame; transparent-frame mode
  and the window layout are unchanged (`interface/components/minimap.rs`).

### Follow-up release 0.2.37

* `0.2.37` (`20261127-102623`) — guild creation and invitation UI.
  The guild window now offers a **Create a guild** section while the character is guildless (name
  input + `CZ_REQ_MAKE_GUILD` 0x0165; the server consumes one Emperium and answers with
  `ZC_RESULT_MAKE_GUILD` 0x0167, mapped to created / already in a guild / name taken / no
  Emperium). Incoming invitations (`ZC_REQ_JOIN_GUILD` 0x016A) are stored as a pending invite,
  announced in chat, and shown in the guild window with **Accept** / **Decline** buttons that send
  `CZ_JOIN_GUILD` 0x016B. All layouts are the 2019 (PACKETVER 20190605) ones; no packet version
  change.

### Follow-up release 0.2.38

* `0.2.38` (`20261127-110417`) — overhead buff effects and ground skill-unit fields.
  Angelus and Impositio (and every descriptor `head: true` `.str` effect) now spawn at the
  overhead anchor and drop the official `ProcessEZ2STR(0, -80, 0)` screen offset, which at the
  port's field of view landed on the torso. Persistent ground units now render their official
  fields: `UNT_SAFETYWALL` → `EF_GLASSWALL2` (`Glasswall2`, `PulsingColumns`), `UNT_SANCTUARY` →
  `EF_BOTTOM_SANC`, `UNT_MAGNUS` → `EF_BOTTOM_MAG` (`client/network/skills.rs`,
  `client/effects.rs::ground_unit_effect`). The menu's "Log out" button is labelled
  **Character select**; the map-server logout path already returns to the character selection
  screen.

### Follow-up release 0.2.39

* `0.2.39` (`20261127-110930`) — Soul Link's big animation.
  `GameModePackets.cpp:19425-19439` launches **both** `EF_SOULLINK` and `EF_ASURABODY` on the
  target for `SL_SOULLINKER` (461) and every other `SL_*` skill. The generated map picked
  `EF_ASURABODY` as the primary (it is the first id in the coverage data), so the signature
  orbit/whitespark animation was never spawned while the descriptor rows were suppressed. The
  map now uses `Soullink` with `PortedAnchor::Target` for skills
  `445, 447..458, 460, 461` (`tools/gen_ported_skill_map.py`, `SL_SOULLINK_SKILLS`), and
  `secondary_ported_effect` spawns the `Asurabody` glow on the same target. Target-anchored
  ported effects now also attach to the target entity rather than the caster.
