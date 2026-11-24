# In-client verification of the rodecomp-derived data

Frames captured from a headless run of the built client against the live CondaRO server
(2026-11-23), using the benchmark's `skills` scenario. They show the data this audit added
rendering in the real client, not a mock.

## How the run was made

```sh
Xvfb :97 -screen 0 1280x720x24 -nolisten tcp &
cd korangar/korangar
env -u WAYLAND_DISPLAY DISPLAY=:97 XDG_RUNTIME_DIR=/run/user/1000 \
    KORANGAR_MUTE_AUDIO=1 KORANGAR_BENCH=skills \
    KORANGAR_BENCH_USER=verify6 KORANGAR_BENCH_PASS=verifypass \
    ../target/release/korangar -v

# targeted run (one effect at a time, slower pacing)
env ... KORANGAR_BENCH_SKILLS="34,306,19,451,355" KORANGAR_BENCH_SKILL_DELAY=3.0 \
    ../target/release/korangar -v
```

The sweep logs one `[bench-skill] {"skill":N,"phase":"..."}` line per spawn. A full run drives
43 skills through `Client::spawn_skill_effect_phase` (450 ms apart) and reports
`effects_mean 25.8 / effects_max 35` on screen, with no missing-asset warning and no panic.

## Frames

| file | what it shows |
| --- | --- |
| `blessing-angel.jpg` | AL_BLESSING (34): the official `이팩트\축복` angel sprite above the character's head (`Spr`, head anchor) plus the white body glow of the effect — the layer the earlier data suppressed. |
| `storm-str-effect.jpg` | A `.str` screen effect driven through the same pipeline (Storm Gust / Meteor family), rendered by the effect pass on the player. |

The buff bar on the right shows the 16 injected persistent statuses (auras/fades) that the run
also exercises.

## Caster pose capture (READYSHOT / THROW)

The effect sweep does not set the caster body pose by default, and the injected status auras hide
it. Both can be enabled/suppressed for a pose run:

```sh
env ... KORANGAR_BENCH=skills KORANGAR_BENCH_USER=verify6 KORANGAR_BENCH_PASS=verifypass \
    KORANGAR_BENCH_SKILLS="47,394,446" KORANGAR_BENCH_SKILL_DELAY=3.0 \
    KORANGAR_BENCH_CASTER_POSE=1 KORANGAR_BENCH_SKIP_AURAS=1 \
    ../target/release/korangar -v
```

`KORANGAR_BENCH_CASTER_POSE=1` applies the skill's `CasterAction` through the same entity methods
as the network handlers; `KORANGAR_BENCH_SKIP_AURAS=1` skips `BENCHMARK_AURA_STATUSES`.

`caster-actions-readyshot-throw.jpg` (frame captures ~the first pose cycle of each skill) shows the
corrected rows: `AC_SHOWER` (47) and `CG_ARROWVULCAN` (394) play `Action(11)` (official
`ST_READYSHOT` -> flat ACT animation 88 -> ACT action 11) and `AM_BERSERKPITCHER` (446) plays
`Action(5)` (official `ST_THROW` -> flat ACT animation 40 -> ACT action 5). The benchmark character
is a Novice, so the readyshot action is the Novice's action-11 strike pose, not an archer aiming
stance; the point is that the caster leaves idle and plays the official action index.

Environment note: `CondaRO.grf`'s `data\sclientinfo.xml` contains a `<loading>` block that the
current `quick-xml` clientinfo parser rejects ("duplicate field `filename`"), and its address
(100.40.203.12) is stale. Mounting a small folder that contains a copy of the patched `client/game/data/sclientinfo.xml`
(live 100.40.206.179, no `<loading>` block) as the highest-priority archive works around both. The local `client/` configuration is gitignored.

### Additive status blend (Berserk / Aura Blade)

`KORANGAR_BENCH_AURAS` isolates a single status (comma separated ids; the full
set is injected by default). Isolated frames, verify2:

| file | status | what it shows |
| --- | --- | --- |
| `status-berserk-additive.jpg` | 107 Berserk | the body renders as a red silhouette (`EF_REDBODY` + `SetColor(255,100,100)`, `render.flags |= 1` body blend), matching the official red-body blend. |
| `status-aurablade-additive.jpg` | 103 Aura Blade | the weapon/body keeps its normal colour with a subtle white additive glow (`EF_AURABLADE2`, `render.flags |= 2` weapon blend). |

With all 18 benchmark auras injected at once the stacked additive rows read as a
bright bloom; the isolated frames show each row on its own is correct, so the
default set is a stress case, not a fidelity problem.

### Weapon layer (Novice dagger)

The weapon layer used to be disabled for every character because
`animation_is_drawable` inspected only the first eight ACT actions, which are
placeholder clips in the per-job weapon ACTs. After scanning every action the
Novice's `초보자_남_단검` candidate loads and the dagger is drawn:
`weapon-novice-dagger.png` (crop) and the log excerpt next to it show the entity
no longer logging "disabling the weapon layer" for that path. Per-class ACTs
that still fail (Sniper bow, Paladin sword, ...) are listed by the info-level
`weapon animation not in the archives` line.

## Account notes

* `verify6` / `verifypass` and `verify4` / `verifypass` both reach the map.
* `verify5` gets stuck at the character-selection window in these runs (no character arrives);
  `verify4` occasionally reports "Already online" while a previous session is still open on the
  server. Use a different test account (or wait) when that happens.
* The char server sometimes answers "Rejected from server" on the first attempt; retrying
  succeeds.

## Status sprite renderer (additive body/weapon blend)

The additive body/weapon blend of section 4.4 of the audit was verified in the same headless
scenario after adding statuses 103 (Aura Blade: additive weapon) and 107 (Berserk: additive body)
to the injected aura list and refreshing the status visuals from the benchmark:

```sh
env -u WAYLAND_DISPLAY DISPLAY=:96 XDG_RUNTIME_DIR=/run/user/1000 \
    KORANGAR_MUTE_AUDIO=1 KORANGAR_BENCH=skills \
    KORANGAR_BENCH_USER=verify2 KORANGAR_BENCH_PASS=verifypass \
    ../target/release/korangar -v
```

* `status-sprite-sweep.log` — full 43-skill sweep: `skill_sweep` finished with 785 frames
  (mean 33.8 ms, p95 38.2 ms), no panic, no missing-asset error (`client exit: 124` is the
  external `timeout`).
* `status-sprite-sweep.jpg` — frame at ~38 s: the character still renders with the injected
  statuses (Berserk/Aura Blade active; the white bloom is the stacked official status auras,
  Fury/Berserk).
* `status-sprite-statuses.jpg`, `status-sprite-statuses.log` — isolated run
  (`KORANGAR_BENCH_SKILLS=1`, so the sweep completes after one skill and the client idles):
  the character with only the persistent status visuals is still visible (not black/invisible);
  `status-sprite-crop.jpg` is the enlarged body. The additive blend is `SrcAlpha / One` into the
  forward colour target, so `EF_REDBODY` reads as a strong red-white glow rather than a mild
  tint; the official client applies blend mode 5 instead of the normal body layer, while this
  renderer keeps the lit opaque body (opaque pass unchanged) and adds the glow on top.

The flat-colour sprite outlines (Kaite 153, Soul Link 149, Property Undead 97) are **not**
implemented: the forward entity shader multiplies the sampled sprite RGB by the instance colour,
so an offset copy can only be tinted, not flattened into a silhouette. That needs a shader-side
flat-colour path and was skipped per the task's "no shader edits without visual A/B" rule.
