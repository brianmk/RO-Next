# Unified skill effect pipeline — in-client evidence (muted Linux/Xvfb, GM test char)

Captured on 2026-11-22 from the release binary of branch `feat/skill-effect-pipeline`
(`/home/phantom/korangar-skill-pipeline`, worktree build in
`/home/phantom/korangar-skill-pipeline-target/release/korangar`, sha256 prefix
`939baa960de3e6ae`; this binary was built with the temporary verification harness described
below and is **not** the hand-off binary — the hand-off binary excludes the harness).

* Display: `Xvfb :94 1280x720x24`, X11 backend, 30 fps cap, `KORANGAR_MUTE_AUDIO=1` + null
  ALSA, `KORANGAR_FORCE_UNFOCUSED=1` — no Wine, no audio device, no user desktop.
* Account/character: **verify4 / VerifyBot4** (GM 99; the account's saved settings in the
  `korangar-assumptio` clone). Neither the user's character/client nor verify5/verify6 were
  used; the account was shared with the `effect-assumptio` lane with explicit hand-over.
* Server: condaRO 2019 (Tailscale `100.40.206.179:6900`), same as the other lanes.
* Login/character selection was automated by the committed benchmark driver
  (`KORANGAR_BENCH=full`, `KORANGAR_BENCH_USER/PASS`, slot 0). The chat/GM-command path could
  not be typed reliably through XTEST on this session (chat input focus), so a **temporary**
  `KORANGAR_SKILL_SWEEP=<csv>` hook was added to `korangar/src/benchmark.rs` for the capture:
  once in game it calls the real `Client::spawn_skill_effect_phase` for all six phases
  (`Cast`, `Ground`, `Success`, `Release`, `Hit`, `Caster`) of each listed skill, one skill
  every 1.6 s, at the player entity/position, with the real effect renderers, loaders and
  effect budgets. **The hook was removed before hand-off** (`korangar/src/benchmark.rs` is
  unchanged in the final commit); it exercised the same spawn functions the packet handlers
  call, not a test-only rendering path.
* Skill list: `34,29,33,45,28,110,270,401,59,62,357,361,89,14,5,70` (Blessing, Inc AGI,
  Angelus, Concentration, Heal, Hammer Fall, Fury, Zen, Spear Boomerang, Bowling Bash,
  LK Concentration, Assumptio, Storm Gust, Cold Bolt, Bash, Sanctuary).
* Screenshots every 1.4 s via `import -window root`.

## Result

All 16 skills spawned their layers through the unified table (53 `skill effect:` spawn lines;
see `client-log-skill-sweep.txt`). Highlights that pin the acceptance criteria:

```
skill effect: spawned texture layer 'effect\pok2.tga' for skill 34 (Success, anchor Auto)
skill effect: spawned 'efst_angel_fluttering\angel_fluttering.str' for skill 34 (Success, anchor Caster)   [head: true -> overhead]
skill effect: spawned sprite layer '<이팩트\축복>' for skill 34 (...)                                      [official Blessing SPR, 6 dup copies]
skill effect: spawned texture layer 'effect\ac_center2.tga' for skill 29 (Success, anchor Auto)  [x3]
skill effect: spawned texture layer 'effect\agi_up.bmp' for skill 29 (Success, anchor Auto)
skill effect: spawned 'efst_c_blessings_of_soul\blessingsofsoul.str' for skill 29 (Success, anchor Caster)
skill effect: spawned 'angelus.str' for skill 33 (Success, anchor Auto)                 [official head:true]
skill effect: spawned 'concentration.str' for skill 45 (Success, anchor Caster)
skill effect: spawned cylinder layer 'effect\ring_white.tga' for skill 28 (Success, anchor Auto) [EF_HEAL, x2]
skill effect: spawned texture layer 'effect\pok3.tga' for skill 28 (Success, anchor Auto)
skill effect: spawned 'crashearth.str' for skill 110 (Success, anchor Auto)             [official Hammer Fall]
skill effect: spawned 'black_hammerfall.str' for skill 110 (Success, anchor Caster)     [migrated hand-written layer]
skill effect: spawned cylinder layer 'effect\ring_yellow.tga' for skill 270 (Cast/Success, anchor Caster) [Fury]
skill effect: spawned cylinder layer 'effect\ring_yellow.tga' for skill 401 (Cast, anchor Caster) + BodyFlash [Zen]
skill effect: spawned 'spearboomerang.str' for skill 59 (Success, anchor Caster)        [audit head fix]
skill effect: spawned 'bowling.str' for skill 62 (Success, anchor Caster)               [audit head fix]
skill effect: spawned 'twohand.str' for skill 357 (Success, anchor Auto)                [audit head fix]
skill effect: spawned 'asum.str' for skill 361 (Success, anchor Auto)                   [Assumptio one-shot]
skill effect: spawned 'stormgust.str' for skill 89 (Success, anchor Auto) + procedural storm cloud
skill effect: spawned cylinder layer 'effect\magic_green.tga' for skill 70 (Ground, anchor Ground) + sanctuary.str
```

Files:

* `client-log-skill-sweep.txt` — the relevant client log lines (`bench-start`, every
  `skill-sweep` step and every `skill effect:` spawn line, including loaded phase count).
* `skill-sweep-montage.jpg` — all 22 captured frames (4×6 contact sheet).
* `skill-sweep-annotated.jpg` — four annotated frames (Angelus/Concentration overhead `.str`,
  Heal cylinders, Zen cast + body flash, Storm Gust `.str` + cloud).
* `skill-sweep-overhead-crops.jpg` — zoom on the character: the overhead `.str` bell of
  Angelus/Concentration is visibly above the head (frame 3–4), the heal/Angelus mist and the
  Assumptio blue-white glow follow (frames 5–8).
* `frame-02.jpg`, `frame-04.jpg`, `frame-07.jpg`, `frame-13.jpg` — individual full frames.

## Limitations / honest notes

* Attribution of a single screenshot to a single skill is approximate (one skill spawns six
  phases per 1.6 s step and effects overlap). The log lines are the authoritative mapping;
  the screenshots prove that the unified spawn path renders in the real client without a
  crash and with the expected classes of visuals (overhead `.str`, sprite particles, texture
  bursts, cylinders, body flash).
* The packet -> `NetworkEvent` -> spawn wiring is the same code unit-covered by the loader /
  tracker tests; this capture drives the spawn functions directly because XTEST chat typing
  was unreliable in this session. The benchmark driver itself logged in and entered the map
  through the normal packets (`[bench-stage] login`), so the client session was real.
* The session was short (~30 s in game) because the shared GM account is used by other lanes
  and the map server had intermittent char-select timeouts; all 16 skills × 6 phases were
  captured in that window.
