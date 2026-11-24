# Golden UI baseline matrix

Committed reference screenshots for the interface, generated and checked by
`scripts/ui-golden.sh`.

Each image is a **crop of a single window**, not a full screenshot, so the
animated 3D world behind the windows never has to be compared:

* `login.png`, `select_server.png`, `character_select.png` – menu windows,
* `inventory.png`, `equipment.png`, `character_overview.png` – in-game windows.

The file name is `<theme>-<scale>-<screen>.png`:

| part | values |
| --- | --- |
| theme | `default` (built-in dark theme), `classic` (built-in classic RO skin) |
| scale | `0.75`, `1.0` (interface scaling from `client/interface_settings.ron`) |
| screen | one of the screens listed above |

`<screen>.mask.png` contains the pixels that are animated inside the window
(paper-doll idle animation, buff timers, ...): it is derived from two captures a
moment apart and ignored by the comparison. `manifest.json` records the window
rectangles and the theme/scale of every capture.

## Usage

```bash
# capture the matrix and compare it against these baselines
export UI_GOLDEN_ACCOUNT=verify5
export UI_GOLDEN_PASSWORD='...'          # never commit this, use a git-ignored file instead
scripts/ui-golden.sh

# only lint the harness without a server
bash -n scripts/ui-golden.sh
scripts/ui-token-budget.sh

# regenerate the baselines (only when an intentional visual change is reviewed)
scripts/ui-golden.sh --update-baseline

# a single theme/scale (fast iteration)
scripts/ui-golden.sh --themes classic --scales 1.0
```

Useful environment variables:

| variable | meaning |
| --- | --- |
| `UI_GOLDEN_ACCOUNT` / `UI_GOLDEN_PASSWORD` | verification account (required) |
| `UI_GOLDEN_CREDENTIALS_FILE` | file with account on line 1, password on line 2 |
| `UI_GOLDEN_BIN` | client binary (default `target/release/korangar`) |
| `UI_GOLDEN_DISPLAY` | X11 display (default `:121`) |
| `UI_GOLDEN_RES` | Xvfb resolution (default `1280x720`) |
| `UI_GOLDEN_SERVICE_ID` | service id hash of the server connection, only needed when the server address changes |
| `UI_GOLDEN_THEMES` / `UI_GOLDEN_SCALES` | matrix override, e.g. `classic` / `1.0` |
| `UI_GOLDEN_VK_ICD` | Vulkan ICD json to use (software rendering) |
| `UI_GOLDEN_KEEP_CLIENT` | `1` keeps the per-variant capture directories and logs |

## How the comparison works

`scripts/ui_golden_compare.py` compares the crops with a tolerant metric:

* the crop is inset by 4 px so transparent window edges cannot leak the 3D world
  into the comparison,
* the driver captures a second frame per screen, cropped identically, and the
  baseline step stores the pixels that changed between them as an animation
  mask (plus everything a higher-z window covers),
* per-pixel channel difference above 24 counts as changed,
* at most 0.4 % of the (unmasked) pixels may change,
* the RMSE of the unmasked pixels must stay below 4.0,
* the window rectangle may not move by more than 2 px (layout drift); a crop that
  is at most 8 px smaller/larger is compared on the common area (auto-sized
  windows can gain or lose a text-metric pixel between runs).

On a failure it writes `diff-<screen>.png` (baseline / current / amplified diff)
into the capture directory, which is also uploaded by the CI job.

## Known v0 limitation

The always-open chat/hotbar overlays can cover the small `character_overview` HUD
window; in that case the whole crop is masked (the comparison reports "every
pixel is masked") and cannot fail. The next harness iteration should capture the
HUD with the overlays closed/moved.
