# Korangar / RO-Next — Assumptio: invisible glow + missing status icon (fix)

Owner: `effect-assumptio` (subagent lane)
Branch: **`fix/assumptio-aura-icon`**, worktree `/home/phantom/korangar-assumptio-fix`
Base: integration `935c3ecd` (rebased from `e87268af` onto the skill-effect-pipeline
merge; clean rebase, no conflicts)
Code commit: **`48a10e20`** (`fix(effect): clearly visible Assumptio aura + official status icon`)
Report commit: this file, on top of the code commit (branch tip; see
`git log -1 fix/assumptio-aura-icon`)
Test binary: `/home/phantom/korangar-assumptio-target/release/korangar`,
sha256 `1fd33a16a868634428eb99431c5358b79ca8cae9f297a0d024dd081cc56afb4b`
(full client, release profile, built from the rebased branch; run muted on
Xvfb, no Wine, GM test characters only)

## User report

“Assumptio shows no glowing effect over the character and no status icon.”

## Root causes (reproduced live, `-vvv` client log + screenshots)

1. **The glow did not read as a glow.** The status packet (`ZC_MSG_STATE_CHANGE3`,
   `0x0983`, status 110) arrived, the status was applied, and
   `World::update_status_auras` did spawn the effect:
   `status aura: spawned glow for status 110 on entity 20000079`. The effect
   even rendered, but it was visually negligible:
   * the camera facing “body glow” was a 9 world unit dark-rim `round_sphere`
     sphere at chest height — smaller than half the 21-unit character,
   * the ground layers were radius 8 / radius 5 with alpha 0.32 / 0.55,
   * the camera facing layer sat *inside* the character sprite volume, so the
     scene depth buffer occluded roughly half of it.
   The old evidence screenshots show no visible glow
   (`notes/korangar-assumptio-aura-evidence/c02-aura-a.png`).

2. **No status icon.** `korangar/src/state/status_effect_icons.rs`
   (`STATUS_EFFECT_ICONS`) had no entry for `EFST_ASSUMPTIO` (110) or
   `EFST_ASSUMPTIO2` (473). The buff bar then drew the `OverflowBehavior::Shrink`
   fallback name into the 34 px slot, which is an unreadable few-pixel speck —
   the log even said
   `status: status effect 110 has no icon/name mapping`.
   The official 2019 `stateiconimginfo.lub` has **no** entry for 110/473, but
   the official hard-coded status icon table (roBrowserLegacy
   `src/DB/Status/StatusInfo.js`) maps **both** `SC.ASSUMPTIO` and
   `SC.ASSUMPTIO2` to **`icon05.tga`**, which is also the explicit
   `stateiconimginfo.lub` image of `EFST_ASSUMPTIO_BUFF` (1121 → `ICON05.tga`)
   and exists as `data\texture\effect\ICON05.tga` in `data.grf`.

3. **The aura/buff bar did not survive a warp.** On a map change the client
   cleared all status effects with the comment “the server re-sends the buffs of
   the new map session”. That is not true for statuses without the
   `DisplayPc`/`DisplayNpc` flag: rAthena only re-sends `sc_display` entries
   (`0x0984` / `ZC_EFST_SET_ENTER`) on map enter, and `Assumptio` in
   `db/pre-re/status.yml` has `Icon: EFST_ASSUMPTIO` but no `DisplayPc`. The
   aura and icon therefore vanished on every warp until the buff was recast,
   even though the server kept the buff active.

## What changed

| file | change |
| --- | --- |
| `korangar/src/world/effect/status_aura.rs` | Assumptio spec rebuilt as a blue-white **shield of light around the whole character**: animated `efst_waterfield2\aurawaterfield1..4.tga` camera facing shield (26 world units, 8 fps), `whitelight.tga` body glow (15 units), `whitelight.tga` ground light pool (radius 11) and a rotating `efst_waterfield2\bubble1.bmp` ring (radius 7.5), higher alphas (0.38–0.65) and a slow out-of-phase pulse. The camera facing body layers are moved 2.5 world units towards the camera so the scene depth buffer cannot half-occlude them behind the character sprite. Ground lift raised from 0.15 to 1.0 world units (reverse-z depth precision made the old value z-fight on cobblestone). Sitting keeps lowering the body layers (12/21 height factor). Unit tests assert the shield is larger than the 21-unit character, bright enough, animated and textured. |
| `korangar/src/state/status_effect_icons.rs` | `(110, "icon05.tga")` and `(473, "icon05.tga")` with a comment documenting the provenance (roBrowser hard-coded table + `EFST_ASSUMPTIO_BUFF` image + file verified in `data.grf`). Table stays sorted for the binary search. |
| `korangar/src/state/status_effects.rs` | new `StatusEffects::retain_for_entity()`; unit tests for the Assumptio icon (`effect\icon05.tga` for 110 and 473), for the sorted/non-empty icon table and for the status retention. |
| `korangar/src/lib.rs` | map change now keeps the **local player’s** status effects (`retain_for_entity`) instead of clearing everything, so timed buffs keep their buff bar entry and world aura across a warp; statuses of other entities are still dropped with the old map session. Also fixed the misleading debug message (it said “no icon/name mapping” even when only the icon was missing). |

No change to networking/packets was needed: `0x0983`, `0x0196` and `0x0984` were
already parsed and applied. Status 473 (`EFST_ASSUMPTIO2`, party/cash Assumptio)
is covered by the same spec and icon; this test server never sends it (it has no
`status.yml` entry that uses `EFST_ASSUMPTIO2`), so it is covered by unit tests.

## Live verification (muted Xvfb clients, GM test characters, no Wine/user chars)

Accounts: `verify4`/`VerifyBot4` (GM 99, `@jobchange`-free test character,
temporarily `@baselevelup 60` for the SP to cast) and `verify2`/`VerifyBot2`
(second client for the “other player” case; created for this test). The user’s
client and characters were never used, and no server process was restarted.

* **Self cast** `@useskill 361 1 self` (note: the server command syntax is
  `@useskill <skill id> <level> <char name|self>` — passing an account id fails
  with “Character not found”): status 110 applied, `BuffBarWindow` opened, the
  buff bar shows the official `ICON05` shield icon, and the character is
  surrounded by a clearly visible blue-white shield with a ground light pool
  (before/after screenshots `00-before-after.png`,
  `02-self-assumptio-aura.png`, `02c-buffbar-icon-assumptio.png`).
* **Refresh**: re-cast keeps the status/aura (no duplicate spawn, buff bar
  stays).
* **Map change with the buff active** (`@warp prantera`, real map load): the
  status is kept, the aura respawns after `MapLoading -> InGame`
  (`status aura: spawned glow for status 110 on entity 20000079` at
  `22:35:22.808`) and the buff bar stays open — previously both disappeared.
* **Expiry**: the local timer removes the status, the reconciliation
  deactivates the aura and the buff bar closes (20.0 s for a full level-1 buff;
  3.0 s remaining after a warp); `06-expired-no-aura.png` shows no glow.
* **Other player (two clients)**: client A (VerifyBot4) casts
  `@useskill 361 1 VerifyBot2` on client B. A’s client spawns
  `status aura: spawned glow for status 110 on entity 20000077` (the other
  player) and B’s client spawns the same for its own entity; B’s buff bar shows
  the icon (`04-other-player-aura-self-view.png`, `05-other-player-self-view.png`).
  Aura follows the entity and is also removed when it leaves the view/death via
  the shared reconciliation path.
* **Centering (lead/verifier observation “aura visible but offset from the
  character” on merged `935c3ecd`)** — re-measured in-client on this branch:
  the additive glow pixel bbox in a default-camera field screenshot is
  x 597–698, y 309–377 (centre 647.5, 343) while the character sprite is centred
  at ~(650, 340); the shield is centred on the character.  The camera-facing
  body layers are moved 2.5 world units towards the camera for depth only,
  which shifts the projected glow by ~3 px, well inside the sprite.  Before/after
  annotated: `09b-centering-before-after.png`.
* **Status 473** (`EFST_ASSUMPTIO2`): same spec + icon, unit-tested; not
  triggerable on this server (no server-side status uses it).
* **Rebased branch smoke test** (on top of the skill-effect-pipeline merge,
  binary above): `@baselevelup 9` + `@heal` + `@useskill 361 1 self` in
  `prt_fild08` — `status aura: spawned glow for status 110 on entity 20000079`,
  no “no icon mapping” line, buff bar opens and the aura is visible
  (`10-rebased-baseline.png`, `11-rebased-self-aura.png`).

## Known limitation — full logout/relogin

A *full* client restart / logout→login cannot restore the aura without a packet:
rAthena only re-sends statuses with `DisplayPc`/`DisplayNpc` on map enter
(`clif_efst_status_change_sub` iterates `sc_display`), and Assumptio has no such
flag in `db/pre-re/status.yml`. The client now handles any status it is told
about (`0x0983`/`0x0196`/`0x0984`) and keeps it across warps. If aura/icon
persistence across a full relogin is required, the one-line server data change
is to add `DisplayPc: true` to the `Assumptio` (and other) status entries — that
is a server DB change, outside this client branch, and was intentionally not
made on the test server during this run (it would need a map-server restart and
would affect other agents/clients).

## CI parity

| check | result |
| --- | --- |
| `cargo fmt --all --check` | pass |
| `cargo clippy -- -Dwarnings` | pass |
| `cargo clippy --all-features -- -Dwarnings` | pass |
| `cargo test -p korangar --lib` | 440 passed, 0 failed, 11 ignored (rebased base) |
| `cargo build --release -p korangar` | pass (binary sha256 above) |

Pre-existing clippy findings outside the CI invocation (e.g. `--all-targets`
tests in `ui_consistency.rs`) are unchanged by this branch.

## Test data cleanup

* `VerifyBot4` (char 150047) was restored to `base_level=1, job_level=1,
  base_exp=0, job_exp=0, max_hp/hp=40, max_sp/sp=11, status_point=48,
  prontera 144,179` after the clients were stopped (the temporary level-up for
  the SP was done with `@baselevelup 60`).
* A test character `VerifyBot2` was created on the `verify2` test account for the
  two-client test (no user account touched); it is left in place for future
  two-client verifications.
* Map/login/char servers were not restarted.

## Evidence

`/home/phantom/Games/ragnarok/CondaRO-linux/notes/korangar-assumptio-aura-icon-evidence/`

* `00-before-after.png` — close-up before vs after the fix (same character spot)
* `01-baseline-field-no-aura.png`, `01b-baseline-closeup.png` — no buff
* `02-self-assumptio-aura.png`, `02b-self-aura-closeup.png` — self cast, visible shield
* `02c-buffbar-icon-assumptio.png` — buff bar with the official `ICON05` icon
* `03-after-mapchange-aura-persists.png` — aura after a real warp
* `04-other-player-aura-self-view.png`, `04b-other-aura-closeup.png` — aura on another player
* `05-other-player-self-view.png` — second client’s own view (buff bar + aura)
* `06-expired-no-aura.png`, `06b-expired-closeup.png` — aura gone on expiry
* `08-centering-baseline.png`, `09-centering-aura.png`,
  `09b-centering-before-after.png` — centering measurement (glow bbox vs sprite)
* `10-rebased-baseline.png`, `11-rebased-self-aura.png`,
  `11b-rebased-self-aura-early.png` — rebased-branch smoke test
* `logs-assumptio.txt` — exact log lines (packet/aura spawn, map-change respawn,
  two-client spawns, icon provenance, centering measurement)

## Packaging

Rust only (no shader/data/GRF changes) — the next RO-Next Windows build must be
rebuilt from this branch; no `.ron`/GRF/asset changes are required.
