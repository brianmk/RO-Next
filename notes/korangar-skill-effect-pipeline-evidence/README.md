# Unified skill effect pipeline — in-client evidence (muted Linux/Xvfb, GM test char)

Captured on 2026-11-22 from the **final** code of branch `feat/skill-effect-pipeline`
(after the lead's feedback fixes: official Blessing layers kept, overhead anchor field,
Hammer Fall `crashearth.str`, Gloria/Impositio overhead corrections, body-aura exceptions).
Binary: `/home/phantom/korangar-skill-pipeline-target/release/korangar`, sha256 prefix
`4e0e0174943acd8c`, built with the temporary `KORANGAR_SKILL_SWEEP` verification harness
described below (that harness is **not** part of the hand-off commit —
`korangar/src/benchmark.rs` is unchanged in the final tree).

* Display: `Xvfb :94 1280x720x24`, X11 backend, 30 fps cap, `KORANGAR_MUTE_AUDIO=1` + null
  ALSA, `KORANGAR_FORCE_UNFOCUSED=1` — no Wine, no audio device, no user desktop.
* Account/character: **verify4 / VerifyBot4** (GM 99, from the `korangar-assumptio` clone's
  saved settings), shared with the `effect-assumptio` lane with explicit hand-over. No other
  test account and **no user character/client** was touched.
* Server: condaRO 2019 (Tailscale `100.40.206.179:6900`).
* Login/character selection via the committed benchmark driver (`KORANGAR_BENCH=full`,
  `KORANGAR_BENCH_USER/PASS`, slot 0) — real login/char servers, real map entry
  (`[bench-stage] login`). XTEST chat typing was unreliable in this session, so the
  **temporary** sweep hook in `benchmark.rs` called the same
  `Client::spawn_skill_effect_phase` function the packet handlers call, once per skill per
  1.6 s, for all six phases, at the real player entity/position.
* Skills: `34,29,33,45,59,62,357,66,75,60,258,495,110,270,401`; frames every 1.4 s via
  `import -window root`.

## What the log proves (final binary)

The log file `client-log-skill-sweep.txt` contains the exact spawn lines, including the
effective anchor and height anchor (the lead's verifiability request):

```
skill effect: spawned sprite layer '<이팩트\축복>' for skill 34 (Success, ...)                     [official Blessing SPR]
skill effect: spawned texture layer 'effect\pok2.tga' for skill 34 (Success, anchor Auto, vertical Feet)
skill effect: spawned 'efst_angel_fluttering\angel_fluttering.str' for skill 34 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned texture layer 'effect\ac_center2.tga' for skill 29 (Success, anchor Auto, vertical Feet)   [x3]
skill effect: spawned texture layer 'effect\agi_up.bmp' for skill 29 (Success, anchor Auto, vertical Feet)
skill effect: spawned 'efst_c_blessings_of_soul\blessingsofsoul.str' for skill 29 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned 'angelus.str' for skill 33 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned 'concentration.str' for skill 45 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned 'spearboomerang.str' for skill 59 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned 'bowling.str' for skill 62 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned 'twohand.str' for skill 357 (Success, anchor Overhead, vertical Overhead)
skill effect: spawned 'impositio.str' for skill 66 (Success, anchor Auto, vertical Overhead)     [Gloria/Impositio head fix]
skill effect: spawned 'gloria.str' for skill 75 (Success, anchor Auto, vertical Overhead)
skill effect: spawned 'twohand.str' for skill 60 (Success, anchor Auto, vertical Body)           [weapon aura exception]
skill effect: spawned 'twohand.str' for skill 258 (Success, anchor Auto, vertical Body)
skill effect: spawned 'twohand.str' for skill 495 (Success, anchor Auto, vertical Body)
skill effect: spawned 'crashearth.str' for skill 110 (Success, anchor Auto, vertical Feet)       [official Hammer Fall asset only]
skill effect: spawned cylinder layer 'effect\ring_yellow.tga' for skill 270 (Cast/Success)        [Fury cone + rings]
skill effect: spawned cylinder layer 'effect\ring_yellow.tga' for skill 401 (Cast) + BodyFlash    [Zen]
```

36 `skill effect:` lines for 15 skills × 6 phases; no panic, no missing asset, no
unknown-layer errors. `skill effects: loaded 436 phases from the archive` shows the unified
table is loaded from the archive file.

## Screenshots

* `skill-sweep-annotated.jpg` — Concentration (`anchor=Overhead`), Impositio Manus
  (official row + `head:true` data correction; the glowing hand renders above the head),
  Gloria (`anchor=Auto vertical=Overhead`), Two-Hand Quicken (`vertical=Body` weapon aura).
* `skill-sweep-montage.jpg` — all 16 frames (4×4 contact sheet).
* `frame-02.jpg`, `frame-06.jpg`, `frame-07.jpg`, `frame-08.jpg`, `frame-13.jpg` — individual
  full frames (Concentration, Impositio, Gloria, Two-Hand Quicken, Zen).
* `client-log-skill-sweep.txt` — `bench-start`, every `skill-sweep` step and every
  `skill effect:` spawn line (with anchor + vertical).

## Limitations / honest notes

* One screenshot may contain overlapping effects from the previous/next sweep step
  (1.6 s per skill, 6 phases at once); the log lines are the authoritative skill →
  layer/anchor mapping, the screenshots prove in-client rendering (overhead `.str` above
  the head, body-height weapon aura, sprite particles, texture bursts, cylinders, body
  flash).
* This capture drives the spawn functions directly (the temporary harness), not the
  packet -> `NetworkEvent` dispatch; the packet handlers were changed only by replacing the
  old per-kind spawn calls with the unified `spawn_skill_effect_phase` call, and the login
  itself went through the real packet path. Final packet-path/visual sign-off is the
  verifier's job (`korangar --check-skill-effects` + in-client sweep on the hand-off SHA).
