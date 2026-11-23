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
