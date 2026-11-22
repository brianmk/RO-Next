# RO-Next / korangar client — hardcoded-systems audit

**Repo:** `/home/phantom/Games/ragnarok/CondaRO-linux/korangar`
**Integration HEAD:** audit started at
`935c3ecde531115b9e6ce5e0b34b0d80f4c6d385` — *Merge branch `feat/skill-effect-pipeline`
(single multi-layer skill effect pipeline, 436 phases/603 layers)*. HEAD advanced twice during
the audit: `1844a09d` (Assumptio aura/icon) and then **`4005da9e32a2e65ff8a906259f911afddb20f0c0`**,
the merge of `feat/weapon-centralization` (`f85b1433` code+data+tests, `c8fceff7` report).
**All citations in this revision were re-verified against that tree** — the integration HEAD
then advanced to `bf53400b` ("Merge branch 'feat/weapon-centralization' …"), whose tree is
identical apart from re-committing this report, so the line numbers hold. The weapon
centralization is now described as landed (§4.2), not in progress. The report itself was first
committed as `85dc53cb`; **this revision was written on top of `bf53400b` and may still be
uncommitted** in the working tree when you read it. HEAD may advance again after this pin —
check `git log -1` before relying on line numbers.
**Mode:** read-only investigation. No code changes, no Wine, no user characters, no game sessions.
**Date:** 2026-11-22
**Companion lanes:** `skill-pipeline` (`notes/korangar-skill-effect-pipeline.md`), `skill-data`
(`notes/skill-effects-official-data.md`), `weapon-centralize`
(`notes/korangar-weapon-centralization.md`, branch `feat/weapon-centralization`, merged at
`4005da9e`).

---

## 1. Executive summary

The codebase is **not** uniformly data-driven yet. The consolidated skill-effect pipeline and
the now-merged weapon registry are two genuinely clean references; almost every *other*
subsystem still contains hand-maintained content tables, magic ids and hardcoded asset paths
in Rust — some of them large and blocking (e.g. the master job→sprite table, which the weapon
registry still depends on).

Measured at `4005da9e` (start `935c3ecd`; the Assumptio and weapon merges are the only
changes in between):

| metric | value |
| --- | --- |
| Rust source files / main-crate lines | 465 files / 94,200+ lines (`korangar` crate: 295 files) |
| runtime data files with archive override (`data\client\*.ron`) | **4** (`skill_effects`, `skill_actions`, `skill_sounds`, **`weapons`**) |
| generated-but-Rust content tables | 5 (`skill_delays` 560 rows, `skill_icon_alias` 241, `monster_sprite_alias` 232, `job_alias` 119, status name/icon 509/355) |
| hand-written content tables / match arms still found | **~1,000** entries across 20+ tables (job 146, effects textures 29, message-code tables 95, keybinds ~20, warps 23, status special cases 16, item grammar 20, …) — the 22-arm weapon suffix table is gone (§4.2) |
| asset-path string literals in Rust | **607**, of which **431** are generated alias-table rows and ~176 hand-written (largest: `world/effect` 29, `loaders` format prefixes 30, `skill_sound` 26, `renderer/skin` 12); weapon/projectile asset lists now live in `weapons.ron` |
| non-localized user-facing UI strings | **68** literals in non-debug windows + **81** dropdown/settings labels + **95** protocol-result-code→English arms |
| localization keys | 132 per language (`en-US`/`de-DE`), parity test green |
| design-token ratchet | **FAILING at HEAD**: 29 color literals (budget 28), 16 `FontSize` sites (budget 12) — `./scripts/ui-token-budget.sh` exits 1 (also confirmed by the weapon lane, which does not touch `src/interface`) |
| weapon centralization | **merged**: `world/weapon.rs` (1009 lines, 11 unit tests) + `weapons.ron` (381 lines: 30 kinds / 30 server types / 47 item-id ranges / 707 weapon ids / Arrow projectile) + `tools/gen_weapons.py --check` + 12 generator tests |

**Verdict / risk reads:**

1. **Skills/effects are no longer the problem area.** `world/skill_cast_effect.rs` is down to
   height anchors (`HEAD_HEIGHT`, `OVERHEAD_CLEARANCE`, `BODY_HEIGHT`, `CAST_CIRCLE_RADIUS`,
   lines 25–38); all per-skill visuals, anchors, sounds, actions and cast-circle colours come
   from `data\client\skill_effects.ron` (436 phases / 603 layers) via
   `world/library/skill_effects.rs` with an `include_str!` fallback and a `--check-skill-effects`
   CLI. `grep` finds `skill_visuals()`/`has_custom_completion()` only in doc comments and test
   names — no runtime code path remains. **Do not re-audit this.**
2. **Weapon centralization is merged and is the second reference pattern** (`4005da9e`):
   the 22-arm suffix match, `BOW_ITEM_VIEW_RANGE`/`WEAPON_TYPE_BOW`/`is_bow_weapon`, the arrow
   `CANDIDATES` list, five duplicated weapon-animation load blocks and the dropped
   `ZC_SPRITE_CHANGE` Weapon/Shield/Robe packets are all gone. `world/weapon.rs` loads
   `data\client\weapons.ron` (compiled fallback + archive override) and exposes
   `profile`/`part_candidates`/`projectile_for_weapon`/`animation_is_drawable`; the generator,
   facts JSONs, `--check` drift guard and 11+12 tests are committed. **Do not re-audit the
   weapon table itself.**
3. **The remaining appearance hardcode is the job/appearance layer**, and the weapon registry
   still depends on it: `world/weapon.rs:69/682` imports `get_sprite_path_for_player_job` to
   build part paths, so the **146-arm job→Korean-sprite match** (`entity/mod.rs:318–478`),
   13-entry hair tables (`:46–47`, `:701–713`), palette-name overrides (`:485–502`) and the
   accessory path template (`library/mod.rs:264–284`) are now the blocker for any
   job/class/headgear/shield/robe work. Shield/robe view ids are plumbed but **not rendered**
   (`entity/mod.rs:255–259`). This is the next P1 content-data migration.
4. **A silent correctness bug lurks in item stats**: `world/library/item_stats.rs` parses item
   descriptions using **hardcoded Korean field labels and Korean grammar markers** (`계열`,
   `공격`, `방어`, `무게`, `무기레벨`, `요구레벨`, `장착`; conditional markers `이상/이하/때/…`).
   Any server with English/translated item descriptions yields empty or wrong stat panels with
   no error.
5. **Localization is only half done and one CI gate is red.** 68 UI literals, 81 settings
   labels and 95 protocol result-code strings are hardcoded English; the token ratchet is
   exceeded at HEAD (a merge raised it 23→28 while other lanes kept adding literals).
6. **Skill *behaviour* is still special-cased even though skill *visuals* are data-driven**:
   Rolling Cutter (`2036`) is special-cased in three places for channelling, Warp (`27`) is
   special-cased for a provisional portal, Free Cast (`278`/jobs `16`,`4017`) for
   move-while-casting, and Storm Gust has a dedicated procedural cloud module with 7 hardcoded
   textures. These belong in the skill data as flags, in the same file the visual pipeline
   already owns.
7. **Generated tables whose generators are not committed** (`job_alias`, `monster_sprite_alias`,
   `skill_icon_alias`, status name/icon) are content-correct but a drift trap: the module docs
   reference `generate_*.py` scripts that are *kept untracked*. The skill pipeline and the new
   weapon pipeline (committed extractor + facts JSON + generator + `--check`) are the correct
   model.

Recommended order: **restore green gates and localization first (P0), migrate the
job/appearance data next (P1 — the weapon registry that just landed depends on it), then item
stats, status effects, skill behaviour flags, UI tokens/keybinds, and the remaining
world/audio/message tables.** The detailed plan is §8.

---

## 2. The reference pattern: how the two consolidated systems do it

Both working systems follow one architecture; every remediation below should use it verbatim.
The weapon pattern is now **committed and merged** (`4005da9e`), so it is the most recent,
complete example.

| layer | skill effect pipeline (`935c3ecd`) | weapon centralization (merged `4005da9e`) | target for new data |
| --- | --- | --- | --- |
| extracted facts | `tools/windows_client_*.json`, `tools/skill_effects_official.json`, `tools/rathena_pre_re_learnable_skills.json` | `tools/rathena_pre_re_weapon_items.json` (707 weapons from rAthena `db/pre-re/item_db_equip.yml` + `pc.hpp` enums) + `tools/weapon_client_assets.json` (GRF scan: 245 per-job weapon families, projectile sprites), extracted by `tools/extract_weapon_items.py` / `extract_weapon_client_assets.py` | committed facts JSON, produced offline from the actual server/client data |
| generator + drift guard | `tools/gen_skill_effects.py --check` (merges official rows + `tools/skill_effects_custom.ron` + legacy per-kind tables) | `tools/gen_weapons.py --check`, invariants in `tools/test_gen_weapons.py` (12 tests) | `tools/gen_<system>.py --check`, refuses to drift in CI |
| runtime data file | `korangar/archive/data/client/skill_effects.ron` (3325 lines, `provenance` + `version` + sorted table) | `korangar/archive/data/client/weapons.ron` (381 lines: provenance/version, `paths`, `sex_suffix`, `fallbacks`, 30 `server_types`, 30 `kinds`, 47 `item_ranges`, an empty `item_views` table reserved for explicit item `View:` overrides, `projectiles`) | `data\client\<system>.ron`, served overridable from the archive |
| loader | `world/library/skill_effects.rs`: `DATA_PATH`, `FALLBACK = include_str!(…)`, `OnceLock` archive copy with compiled fallback, binary search, `deny_unknown_fields` | `world/weapon.rs` (`DATA_PATH:73`, `FALLBACK:75`, `load:610`): `profile:642`, `part_candidates:662`, `projectile:729`, `projectile_for_weapon:737`, `animation_is_drawable:752` | one module + one API, no match arms at call sites |
| validation | `korangar --check-skill-effects [PATH]`, malformed/unknown-field rejection, sorted/unique tests, compiled-fallback-valid test | 11 weapon unit tests + 12 generator/facts tests + fmt/clippy/release green (lane report §verification) | `--check-<system>`, exit 1 on schema error |
| call-site consolidation | one `spawn_skill_effect_phase` used by every packet handler; old loaders deleted | one `request_weapon_animation` helper (`lib.rs:10473`) replaces the load blocks; networking `ZC_SPRITE_CHANGE` Weapon/Shield/Robe now routed (`version_20220406.rs:717`, `event.rs:449`) | exactly one entry point per subsystem |
| provenance docs | `notes/korangar-skill-effect-pipeline.md`, `notes/skill-effects-official-data.md` | `notes/korangar-weapon-centralization.md` + `notes/korangar-weapon-centralization-evidence/` | a `notes/` report with data facts + evidence |

Current runtime data-file loaders: **4** —
`world/library/skill_effects.rs:81-83`, `skill_actions.rs:25-27`, `skill_sounds.rs:23-25`,
`world/weapon.rs:73-75`.

---

## 3. Classification legend

* **P0 — correctness/CI blocker now.** Wrong behaviour, invisible data loss, or a red required
  gate at HEAD. Fix before more feature work.
* **P1 — content-data hardcode.** Game content (ids, names, paths, tables) lives in Rust and
  bypasses data/config/localization; breaks on data changes or a different server data set.
  Should become a `data\client\*.ron` table with a committed generator.
* **P1/P2 — layout/UI token bypass.** Hardcoded colours/sizes/positions that bypass
  `state/theme/tokens.rs`, or user-facing strings that bypass `state/localization`.
* **P2 — maintainability/duplication.**
* **P3 — acceptable, document only.** Protocol/format constants that must match the wire or
  file formats (and are usually generated + contract-tested).
* **OK/generated** — content tables that are generated from authoritative data and have a
  documented provenance and (ideally) a committed generator + drift check.

---

## 4. Top 10 highest-risk hardcoded systems (4.2 is now largely resolved by the merged weapon registry)

### 4.1 Entity appearance: job→sprite/palette/hair tables (P1, largest remaining)
`korangar/src/world/entity/mod.rs`
* `get_sprite_path_for_player_job` **lines 318–478: 146 numeric match arms** mapping rAthena
  job id → Korean sprite stem (`7 => "기사"`, `4054 => "룬나이트"`, …), on top of the generated
  `job_alias` fallback (63 aliases). The merged weapon registry still calls this function
  (`world/weapon.rs:69` import, `:682` use), so **every weapon path is still built on this
  hardcoded table**.
* `MALE_HAIR_LOOKUP`/`FEMALE_HAIR_LOOKUP` **lines 46–47** (13 entries each) map hair style →
  sprite id; `resolve_head_id` **lines 701–713**.
* `get_body_palette_name` **lines 485–502** has 4 special palette-stem overrides
  (`하이프리스트`, `어세신크로스`, `로얄가드`, `리벨리온`) because palette file stems differ from
  sprite stems.
* part path templates **lines 505–700** (`인간족\몸통\…`, `인간족\머리통\…`,
  `data\palette\몸\…`, `data\palette\머리\머리…`).
* shield/robe view ids are stored **lines 255–259** and plumbed through
  `apply_sprite_change` **line 2183** / `SpriteChangeOutcome` **line 136**, but the layers are
  **not rendered** ("not rendered yet") — see 4.2.
* job special cases: `EntityType::from` ranges **lines 143–157** (warp job 45, hidden 111/139),
  transcendent range `4001..=4023` **line 2106**, `BABY_JOB_SCALE = 0.75` **lines 51, 765, 2169**.
* accessory path template `악세사리\{남|여}\{남|여}{name}` in
  `korangar/src/world/library/mod.rs:264–284` (name table itself is `accname.lub` — good).

Why it matters: every remaining appearance feature (headgear, shield, robe, class rendering,
4th/expanded jobs) is blocked on or duplicated against this table; it silently falls back to
`missing.spr` for unknown ids. Even the new data-driven weapon pipeline inherits the job stem
from it, so **this is the highest-leverage remaining content table**.

### 4.2 Weapon centralization — merged; residual appearance work (P2)
`korangar/src/world/weapon.rs`, `korangar/archive/data/client/weapons.ron`,
`korangar/src/lib.rs`, `korangar/src/world/entity/mod.rs`

**Landed at `4005da9e`** (branch `f85b1433` code/data/tests, `c8fceff7` report):
* one registry `world/weapon.rs` (1009 lines, 11 unit tests) loading `weapons.ron`
  (`DATA_PATH:73`, `FALLBACK:75`, `load:610`; API `profile:642`, `part_candidates:662`,
  `projectile:729`, `projectile_for_weapon:737`, `animation_is_drawable:752`);
* generated data `weapons.ron` (381 lines: 30 kinds / 30 server types / 47 item-id ranges /
  707 weapon ids / Arrow projectile; provenance + version; path templates + fallback chain),
  built by `tools/gen_weapons.py --check` from committed facts
  `tools/rathena_pre_re_weapon_items.json` + `tools/weapon_client_assets.json`
  (extractors `tools/extract_weapon_items.py`, `extract_weapon_client_assets.py`; 12 generator
  tests in `tools/test_gen_weapons.py`);
* **deleted hardcodes**: the 22-arm `weapon_suffix_candidates`/`player_weapon_part_candidates`
  match, `BOW_ITEM_VIEW_RANGE`/`WEAPON_TYPE_BOW`/`is_bow_weapon`, the arrow `CANDIDATES` list,
  `arrow_assets_failed`, five duplicated weapon-animation load blocks (one
  `request_weapon_animation` helper, `lib.rs:10473`, call sites `3970/4109/5073/5125/5160/5197`),
  and `world/particles` `FLIGHT_SECONDS` (flight time comes from data);
* networking `ZC_SPRITE_CHANGE` Weapon/Shield/Robe is no longer dropped:
  `version_20220406.rs:717` → `NetworkEvent::AppearanceChanged` (`event.rs:449`).

**Residual (not in the weapon lane's scope):**
* **shield/robe layers are still not rendered** — view ids stored at `entity/mod.rs:255–259`,
  sprite-change outcome applied at `lib.rs:5127–5200`, but no part/layer is produced.
* **headgear** still builds its path in `library/mod.rs:264–284` (data-driven names, hardcoded
  template) — should share the new appearance path helper.
* **weapon attack sound** is still an explicit TODO: `entity/mod.rs:1046–1050` ("Other
  clients are playing the attackers weapon attack sound using this event"); attack *action*
  selection is intentionally unchanged because the RO ACT index is weapon-independent
  (documented in `weapon.rs` module doc).
* the data set has **no drawable weapon clips** (`인간족\무기\*` absent, per-job weapon ACTs are
  hidden placeholders), so the registry attempts candidates and drops the layer gracefully —
  a documented data limitation, not a code hardcode.

Why it matters: the weapon table is now the model for the next migrations; the remaining
appearance gap is small and mostly depends on the job table in 4.1.

### 4.3 Item stats/description parser depends on Korean grammar (P1, silent data loss)
`korangar/src/world/library/item_stats.rs`
* field labels **lines 65–120** (`"계열"`, `"공격"`, `"방어"`, `"무게"`, `"무기레벨"`,
  `"요구레벨"`, `"장착"`).
* conditional-bonus markers **lines 331–335** (`"이상"`, `"이하"`, `"때"`, `"동안"`,
  `"경우"`, `"제련"`, `"정련"`, `"착용"`, `"확률"`, `"마다"`, `"초 "`, `"매 "`, `"레벨"`).
* the reference data is the client `iteminfo.lub` description; `item_description.rs:31–36`
  already loads localized tables (`idnum2itemdesctable.txt` / `num2itemdesctable.txt`) and has
  an English fallback `"No description available."`.

Why it matters: with the English `iteminfo.lub`/server archive, **every stat panel silently
shows zeros/unknown** — no error, no log. Also counts: 37 Korean literals in this one file.
Remediation: generate a factual `item_stats.ron` from the server item DB (same facts-extractor
pattern as weapons), keep description text from the client tables, and treat the Korean parser
only as a last-resort fallback (or delete it).

### 4.4 Protocol result-code → English message tables (P1, localization bypass)
`korangar/src/lib.rs`
* party member removed **5253–5265** (6 arms)
* party invitation reply **5323–5341** (13 arms)
* party creation result **5344–5353** (6 arms)
* buy/sell result **5656–5668** (11 arms)
* skill use failed **5816–5834** (19 arms, e.g. `"not enough SP"`, `"wrong weapon"`)
* ammunition action **5843–5850** (5 arms)
* map-server ban / disconnect **5860–5876** (9 arms)
* plus client feedback strings in **25 `push_chat_information` sites** (e.g.
  `"You have not learned skill {id}."`, `"Skill {}: that cell is out of range."`,
  `"Failed to sell items"` at `5753`).

`korangar-networking/src/packet_versions/version_20220406.rs:41–92` adds **26 hardcoded login
failure messages** (`"Server closed"`, `"Unregistered id"`, …) on top.

Why it matters: the client already has the correct mechanism for exactly this —
`msgstringtable.txt` (`korangar/src/msgstringtable.rs`, loaded from the archive) for server
message ids and `state/localization` for client strings. These ~95 mappings should be data
(`data\client\message_ids.ron` or localization keys with `format!` arguments), not Rust match
arms. This is the largest remaining localization bypass after 4.5.

### 4.5 UI design-token bypass — ratchet is RED at HEAD (P1, CI)
`./scripts/ui-token-budget.sh` at `935c3ecd`:

```
interface style literals: colors=29 (budget 28), FontSize=16 (budget 12)
exit=1
```

Offenders (top): `windows/item_info.rs` 7 colors (`52–58`), `windows/frame_inspector.rs` 6
(debug-only), `components/minimap.rs` 6 (`28–30, 99–100, 138`), `windows/profiler.rs` 4
(debug-only), `windows/chat.rs` 2 (`40`, `420`), and one each
`windows/party.rs:36`, `windows/spirit_spheres.rs:44–46`,
`windows/skill_description.rs:133`, `windows/character_selection.rs:665`;
FontSize 4× `party.rs` (`166,188,199,298`) and 4× `character_selection.rs`
(`528–530,722`), 2× `frame_inspector.rs`, 2× `progress_bar.rs`, 2× `item_box.rs`.
`git log` shows the budget was raised 23→28 in the `fix/skill-animation-particles` merge
(`818ab3e1`) and later lanes added literals; the `--update` script refuses raises by design.

Also hardcoded theme bypasses flagged by in-code TODOs:
`components/item_box.rs:346,350,355` ("Put this in the theme"), `components/skill_box.rs:284,289`,
`windows/skill_tree/slot.rs:386`, `windows/login.rs:167,188` ("Theme this").

### 4.6 Skill-behaviour special cases that survived the visual consolidation (P1/P2)
`korangar/src/lib.rs` + `world/effect/*`
* **Rolling Cutter channeling**: `ROLLING_CUTTER_ID = 2036` **line 187**, special-cased at
  **6083**, **7182**, **7261** (cast channelling, self-cast branch, stop-channelling).
* **Warp Portal**: `skill_id.0 == 27` **line 3598** creates a provisional blue portal before
  `ZC_WARPLIST`/skill-unit packets arrive.
* **Free Cast passive**: `FREE_CAST_SKILL_ID = 278` / `FREE_CAST_JOB_IDS = [16, 4017]`
  **lines 195–198**, used at **10463** for move-while-casting.
* **Storm Gust cloud**: `STORM_CLOUD_PHASE` **line 200**; `world/effect/storm_cloud.rs` with
  7 hardcoded textures **lines 22–28** (explicitly "data files cannot supply it" — but the
  *selection* and timings can be a `Primitive`/`Cloud` layer row in the skill data).
* **Status aura only Assumptio**: `world/effect/status_aura.rs:57–59` (`EFST_ASSUMPTIO` 110,
  `EFST_ASSUMPTIO2` 473), spec **142–172**, dispatch `177–181` — the module doc (lines 28–33)
  still says "add a `StatusAuraSpec` constant for the status and one arm to
  `status_aura_spec`"; the skill pipeline notes call out missing
  Blessing/Inc AGI/Angelus/Concentration auras (EFST 10/12/9/3). The `48a10e20` commit only
  enriched the one Assumptio spec (animated `aurawaterfield1..4` body frames, `whitelight.tga`
  glow) and added the two missing icon rows; it did not make auras data-driven.
* remaining procedural/effect asset paths (29 literals in `world/effect/`): level aura
  `aura.rs:110–131`, cast circle `cast_circle.rs:58–61`, warp `warp.rs:48–50`,
  status aura `status_aura.rs:142–151`, projectile `procedural/projectile.rs:251`,
  storm cloud `storm_cloud.rs:22–28`.
* falcon companion candidates: `loaders/async/mod.rs:301–310` (7 hardcoded paths, comments
  explain three of them are wrong-asset traps).

Why it matters: the consolidated pipeline is the natural home for these flags/rows. As long as
they stay in Rust, new skills and data overrides cannot express them.

### 4.7 Status-effect subsystem: names/icons good, semantics hardcoded (P2)
* generated tables: `state/status_effect_names.rs` (509 entries) and
  `state/status_effect_icons.rs` (355 entries, Korean `.tga` names) — provenance documented
  (rAthena `status.hpp` enum + client `stateiconinfo.lub`/`stateiconimginfo.lub`), **but no
  generator script is committed** (see 4.10).
* value semantics hardcoded: `state/status_effects.rs:113–114` (`ELEMENT_NAMES[10]`) and
  **128–160** (special cases 64, 250, 252, 312, 673, 621/665 plus generic fallback) — English
  tooltip strings not localized.
* refresh/stack policy hardcoded: `state/status_effects.rs:262–347` — refresh replaces the
  entry in place, expiry is local countdown safety net; there is **no buff/debuff category**
  (the buff bar shows every status), and stack counts are not modelled.
* status aura specs only Assumptio (4.6).

### 4.8 Character creation defaults + remaining item/map/audio content hardcodes (P2)
* `korangar-networking/src/lib.rs:977–991`: `hair_color = 0`, `hair_style = 0`,
  `start_job_id = JobId(0)`, `sex = Sex::Male` — no appearance selection at all.
* items: protocol item-type enum duplicated in two places — `state/inventory.rs:51–79`
  (`item_type` module, 12 constants) and `korangar-networking/src/packet_versions/version_20220406.rs:19–22`
  (`ITEM_TYPE_ARMOR/WEAPON/PET_ARMOR/AMMO`); use/category behaviour
  `state/inventory.rs:41–100` is a hardcoded heuristic over those constants.
* maps: debug map warp list `interface/windows/maps.rs:19–44` (23 maps/coords, debug-only
  window but content data); default startup map `DEFAULT_MAP = "geffen"` **`lib.rs:201`** and
  `START_CAMERA_FOCUS_POINT` **`lib.rs:202`** (login-screen map/camera).
* audio: `lib.rs:211–215` `UI_BACKGROUND_MUSIC = "bgm\\01.mp3"`,
  `MAIN_MENU_CLICK_SOUND_EFFECT = "버튼소리.wav"`,
  `TELEPORT_VANISH_SOUND_EFFECT = "effect\\ef_teleportation.wav"`; curated
  `SKILL_SOUNDS` table `world/skill_sound.rs:57–95` (6 skills / 12 paths + generic 2) overlaps
  the generated `skill_sounds.ron`.
* quest icon: `world/particles/mod.rs:354–362` hardcodes
  `유저인터페이스\minimap\quest_{id}_1.bmp` and 4 `QuestColor` RGB values.
* character-overview/party/chat/etc.: duplicated name-length constants (4/24) in
  `character_creation.rs:12–13`, `friend_list.rs:20–21`, `party.rs:28`, `login.rs:17`, and the
  literal `character_name.len() > 24` at `lib.rs:7267` (TODO in `friend_list.rs:18` says
  "duplicated throughout the code base").
* keybindings: `input/mod.rs:281–470` — Alt+E/S/A/Z/Q/C, Ctrl+S/I/G/A/H/M, plain I/M,
  F1–F10 hotbar, F11/F12 tabs, Alt+1–9 emotions; no keybinding table/settings.

### 4.9 Localization coverage gap (P2, but wide)
* 68 literal UI strings remain in non-debug windows (counted exactly like
  `tests/ui_consistency.rs`, excluding `DEBUG_ONLY_FILES`). Notable:
  `windows/context_menu.rs:50–80,246–269` (Whisper/Add Friend/Invite to Party/Trade/View
  Info/Drop and tooltips), `windows/graphics_settings.rs:42–194` (all option labels),
  `windows/buy.rs:311–321` / `sell.rs:295–305` (`+1/+10/+100`), `buy_cart.rs:280–295,335`
  (`-All`, `Total: … z`), `windows/item_info.rs:460,471,485` ("Card slots", "Random
  options", "Description"), `windows/skill_description.rs:131` tip,
  `windows/character_deletion.rs:76` warning, `windows/dialog.rs:139`,
  `components/zeny.rs:44`, `windows/friend_request.rs:31`, `windows/storage.rs:78`.
* **+81** dropdown/settings option labels hardcoded: `graphics/settings.rs` 48
  (FPS list `167–177`, MSAA `352–356`, SSAA, resolution, sampler, shadow options),
  `settings/interface.rs` 7 (aura styles), `loaders/font/mod.rs` 20, `settings/graphic.rs` 2.
* 13 paper-doll slot names/labels hardcoded (`components/paper_doll.rs:52–130`, e.g.
  `"Head top"`, `"TOP"`, `"Right hand (weapon)"`).
* The source-scan lint (`ui_consistency.rs`) passes because its forbidden-word list is narrow;
  the bulk of the gap is invisible to CI.

### 4.10 Generated tables whose generators are missing (P2 process risk)
`job_alias.rs` (119 rows), `monster_sprite_alias.rs` (232), `skill_icon_alias.rs` (241),
`status_effect_names.rs` (509), `status_effect_icons.rs` (355) all document a
`generate_*.py`/`extract_*.py` provenance, but **none of those scripts is tracked**
(`tools/` has only the skill/opcode generators; the alias module headers say "kept untracked in
the clone"). They are correct content today (verified against the GRFs at generation time) but
cannot be regenerated/`--check`ed in CI, unlike `tools/gen_skill_effects.py --check`.

This is no longer hypothetical: the Assumptio lane (`48a10e20`) had to **hand-add two rows**
`(110, "icon05.tga")` / `(473, "icon05.tga")` to the supposedly generated
`STATUS_EFFECT_ICONS` table because the generator is not available. That is exactly the drift
the skill pipeline's committed generators + `--check` prevent.

---

## 5. Full findings table

Severity: **P0** blocker · **P1** content hardcode · **P1/P2** UI token/localization bypass ·
**P2** maintainability · **P3/OK** acceptable/generated.

| # | System | Files:lines | What is hardcoded | Why it matters | Recommended centralization | Sev |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | Job → sprite stem | `korangar/src/world/entity/mod.rs:318-478` (146 arms) | rAthena job id → Korean sprite stem (`기사`, `룬나이트`, …) | blocks/duplicates every appearance feature; the merged `world/weapon.rs:69,682` still calls it for part paths; unknown jobs → `missing.spr`; generated `job_alias` only patches holes | `data\client\jobs.ron` (id, sprite stem, palette stem, sex suffixes, baby/transcendent flags) generated from rAthena `jobidentity`+GRF probe; retire the match | P1 |
| 2 | Job → palette stems | `entity/mod.rs:485-502` | 4 overrides (`하이프리스트`, `어세신크로스`, `로얄가드`, `리벨리온`) | palette file naming differs from sprite naming; new jobs need Rust edits | palette stem field in `jobs.ron` (merged table #1) | P1 |
| 3 | Hair style lookup / head sprite | `entity/mod.rs:46-47`, `701-713` | two 13-entry arrays mapping hair style → sprite id; `머리{}_{}_{}.pal` template | races/mismatches if style ids change; two tables can drift | `data\client\appearance.ron` (hair style → sprite, palette template) or derive from job/GRF probe | P1 |
| 4 | Weapon variants/view ids | **RESOLVED** `world/weapon.rs:73-75,610,642,662`; `data\client\weapons.ron` (381 lines) | was: 22-arm suffix match + numeric-id heuristic | now data-driven: 30 kinds / 30 server types / 47 id ranges / 707 ids, fallback chain in data | reference pattern; merge landed at `4005da9e`; `tools/gen_weapons.py --check` + 12 generator tests | **OK (done)** |
| 5 | Weapon projectile/attack residual | `lib.rs:2349-2376,4660-4690`; `entity/mod.rs:1046-1050`; `world/animation/mod.rs:181-182` | bow range / `W_BOW` / arrow candidate list are **gone** (data-driven `projectile_for_weapon`, flight time from data); residual: weapon attack sound is still a TODO; attack action stays generic by design | no weapon attack sound on auto attacks (minor); attack action is intentionally weapon-independent (documented in `weapon.rs`) | add `attack_sound` to `WeaponProfile`/data; keep action selection generic | P2 |
| 6 | Shield / robe layers | `entity/mod.rs:255-259,136,2183`; `lib.rs:5127-5200` | view ids are stored and routed via `SpriteChangeOutcome`/`AppearanceChanged`, but no shield/robe layer/part is produced ("not rendered yet") | content silently missing; paper doll shows shield slot but the world does not | shield/robe profiles (view id → part candidates) in the weapon/appearance data using `jobs.ron` templates | P2 |
| 7 | Accessory/headgear path | `world/library/mod.rs:264-284` | `악세사리\{sex}\{sex}{name}` template (name table is data — good) | small, but template/sex handling duplicated from body path logic | share an `appearance::part_template` helper fed by data; keep `accname.lub` names | P2 |
| 8 | Entity type ranges / special jobs | `entity/mod.rs:143-158,2106,51,765,2169` | job ranges for Player/Npc/Monster/Warp/Hidden; `4001..=4023` transcendent; `BABY_JOB_SCALE=0.75` | protocol-adjacent but presence of magic 45/111/139; baby scale used in 2 places | data-derived job flags (`EntityType`, transcendent, scale) from `jobs.ron`; keep only documented fallback | P2 |
| 9 | Item stat parsing | `world/library/item_stats.rs:65-120,304-335` | 7 Korean field labels (78-112) + 13 Korean conditional markers (331-335) | silent wrong/empty stats on any non-Korean description; 37 Korean literals | generated `data\client\item_stats.ron` from server item DB facts; description text from client tables; delete Korean heuristic | **P1** |
| 10 | Item categories / use behaviour | `state/inventory.rs:41-100` | item-type constant switch (heal/use/equip tabs, use action) | item type enum duplicated in `networking/version_20220406.rs:19-22`; server-configurable categories can drift | data facts table (id/type/equip position/usable) + single `item_type` module; keep protocol enum in one place | P2 |
| 11 | Item description fallback | `world/library/item_description.rs:31-36` | archive table paths + `"No description available."` | path/format constants; text should be localized | keep paths (format), localize fallback string | P3/P2 |
| 12 | Protocol result messages | `lib.rs:5253-5353,5656-5668,5816-5876`; `networking/.../version_20220406.rs:41-92` | **69 + 26** code→English mappings (party/buy/skill-fail/ammo/ban/login) | user-facing text bypasses `msgstringtable` + localization; duplicated per result enum | `data\client\message_ids.ron` (result code → msgstringtable id/localization key) generated from rAthena headers; `format!` arguments from data | **P1** |
| 13 | Client chat feedback | `lib.rs` 25 `push_chat_information` sites (e.g. `2815,5753,5838,5841,5858,5872,6079,6969,7024,7042,7076`) | literal English feedback strings | unlocalizable; `de-DE` users see English | localization keys (Localization struct + defaults) or msgstringtable ids | P1/P2 |
| 14 | Skill behaviour flags | `lib.rs:187,195-198,200,3598,6083,7182,7261,10463` | Rolling Cutter 2036 channelling; Warp 27 provisional portal; Free Cast 278 + jobs 16/4017; Storm Gust phase | new/overridden skills cannot express these; three Rolling Cutter branches are fragile | extend `skill_effects.ron` schema (or new `skills.ron`) with `channeling`, `warp_target`, `move_while_casting`, layer kind `Cloud`; data-driven | P1/P2 |
| 15 | Storm Gust cloud | `world/effect/storm_cloud.rs:22-28,31-44` | 7 cloud textures + 18 puffs + timings in Rust | per-skill procedural effect not reachable from data/overrides | `Cloud` layer kind in the skill-effect pipeline with texture list/size/count/timing fields | P2 |
| 16 | Status aura specs | `world/effect/status_aura.rs:57-59,142-181` | only Assumptio (110/473); spec constant + match arm | Blessing/Inc AGI/Angelus/Concentration auras missing; every new aura needs Rust | `data\client\status_auras.ron` (status id → kind/texture/tint/radii/timing) merged into status data | P2 |
| 17 | Status value semantics / categories | `state/status_effects.rs:113-160,262-347` | element names, 6 SendVal status ids, English tooltip lines; refresh replaces; no buff/debuff category | tooltip text unlocalized; categories/stacking cannot follow data | `data\client\status_effects.ron` (id→name/icon/category/value label/stack policy) generated from rAthena status DB | P2 |
| 18 | Skill sounds (curated) | `world/skill_sound.rs:57-95` | 6-skill curated table + generic 2 wavs, overlaps generated `skill_sounds.ron` | duplicated source of truth; data override loses to hardcoded curated list | fold curated rows into `tools/skill_sounds_custom.ron` merged by generator (same pattern as skill effects custom) | P2 |
| 19 | Procedural effect assets | `world/effect/{aura,cast_circle,warp,status_aura,procedural/projectile}.rs` (~29 literals) | texture file names in Rust constants | asset set changes require rebuild; overrides impossible | optional texture fields in the effect data rows (aura/cast-circle/warp style), keep procedural code generic | P2 |
| 20 | Falcon companion | `loaders/async/mod.rs:301-310` | 7 candidate paths incl. 3 documented wrong-asset traps | same class as weapon; per-data-set asset names differ | companion/pet profile table (job/option → candidate list), data file | P2 |
| 21 | Character creation defaults | `networking/lib.rs:977-991` | hair 0, style 0, job 0, male | no appearance selection; content fixed | defaults from `appearance.ron`/client config; add creation UI | P2 |
| 22 | Map warps (debug) | `interface/windows/maps.rs:19-44` | 23 map name+coordinate pairs | debug window, but content data; wrong for other map sets | `data\client/debug_maps.ron` or gate behind debug tooling | P2 |
| 23 | Default map / camera | `lib.rs:201-202` | `"geffen"`, Point3(600,0,240) | login-screen map/camera hardcoded for one data set | client config/settings (`client_info` or settings RON) | P2 |
| 24 | UI sounds / menu music | `lib.rs:211-215` | bgm\01.mp3, 버튼소리.wav, ef_teleportation.wav | content paths in Rust; classic-client-specific | `data\client\ui_sounds.ron` (login/menu/teleport tracks) + archive override | P2/P3 |
| 25 | Quest icon | `world/particles/mod.rs:354-362` | `quest_{id}_1.bmp` template + 4 RGB colours | icon path and colours bypass theme/data | data row (quest type → icon) + theme colour tokens | P2 |
| 26 | Design tokens (UI) | interface (29 colours / 16 FontSize sites) | hardcoded colours/font sizes exceed ratchet at HEAD | **CI gate red** (`scripts/ui-token-budget.sh` exit 1); theme regressions invisible | migrate offenders to `state/theme/tokens.rs`; extend ratchet to spacing/positions; do **not** raise budget | **P0/P1** |
| 27 | Localization coverage | interface windows/components (68 strings) + 81 dropdown labels + 13 paper-doll labels | English UI literals | unlocalized DE users; source-scan lint only catches a narrow word list | add `Localization` fields (with serde defaults) + extend lint; use `localization()` everywhere | P1/P2 |
| 28 | Paper doll layout | `components/paper_doll.rs:34-130` | canvas/slot sizes, 11 slot positions, names/labels | equipment layout not theme-able/localizable; hand-tuned per layout | slot table in theme RON or data file; labels localized | P2 |
| 29 | Keybindings | `input/mod.rs:281-470` | all window/hotbar/emotion keys in Rust | not configurable, not discoverable; classic defaults are fine but should be a table | `data\client\keybinds.ron` + settings page (defaults = current) | P2 |
| 30 | Duplicated constants | 4/24 name lengths in 5 files + `lib.rs:7267`; item types 2 files; protocol result-code message tables | duplication | drift bugs (already fixed once for `a@a.com` deletion) | shared `consts`/data module; single item-type module | P2 |
| 31 | Generated tables w/o generator | `job_alias.rs`, `monster_sprite_alias.rs`, `skill_icon_alias.rs`, `status_effect_names.rs`, `status_effect_icons.rs` | content correct, generator untracked | cannot `--check`/regenerate in CI; drift risk | commit the 5 generator scripts + `--check` (skill-pipeline model) | P2 (process) |
| 32 | Skill delays | `world/library/skill_delays.rs` (560 rows) | generated Rust table, **not** archive-overridable | server can't tune timings without rebuild; only 4 data files are overridable | move to `data\client\skill_delays.ron` with include_str fallback (generator exists) | P3 |
| 33 | Classic level-99 aura | `world/effect/aura.rs:55-131` | official effect reproduction constants + 6 textures | documented reproduction of hardcoded official client effects; acceptable | keep; optionally expose textures in effect data | P3/OK |
| 34 | Protocol/packet formats | `ragnarok-packets/*`, `korangar-networking/contract_2019.rs`, `opcode_lengths_2019.rs`, `ragnarok-formats/*` | opcodes/lengths/field layouts/format parsers | must match wire/format; generated + contract-tested | **keep** (see §7) | P3/OK |
| 35 | Protocol enums (item type, equip position, ammo, packet version, hotbar 4×38) | `state/inventory.rs:51-79`, `ragnarok-packets`, `state/hotbar.rs:21-25` | enum values | must match server wire | **keep** as documented constants (dedupe item type) | P3/OK |
| 36 | Classic skin assets | `renderer/skin.rs:25+`, `loaders/gamefile` folders | `유저인터페이스` skin atlas files | fixed classic skin preset; rendering feature, not content logic | optionally data-driven skin manifest | P3 |

**Counts behind the table:** 146 job arms + 2×13 hair + 4 palette overrides + 23 warps +
7 falcon + 29 effect paths + 69+26 message arms + 6 skill sounds + 20 item grammar markers +
~20 keybinds ≈ **1,000 hardcoded content entries still in Rust**, plus 607 asset-path literals
(431 generated) and ~176 non-localized strings/labels. The 22 weapon arms and 4 arrow candidate
paths have moved to `weapons.ron` (4045 merged).

---

## 6. Area-by-area notes (what else was checked and cleared)

### 6.1 Skills / effects / animations / sounds
* `world/skill_cast_effect.rs` — **clean**: only entity-dependent anchor constants (lines
  25–38) and pure functions; no skill ids, no asset paths.
* `world/library/skill_effects.rs` — unified loader, binary search, `deny_unknown_fields`,
  placement/anchor logic, tests; **no magic ids outside tests**.
* `data\client\skill_effects.ron`: 436 (skill, phase) rows / 603 layers; merged from official
  multi-layer fragment (`tools/skill_effects_official.ron`, 351 rows/511 layers),
  human-written `tools/skill_effects_custom.ron`, and three legacy tables
  (`skill_effect_assets.ron` 214 lines, `skill_sprite_effects.ron` 66, `skill_procedural_effects.ron`
  292) — the legacy RON files are **generator inputs only, not runtime-loaded**
  (`grep -rn 'skill_effect_assets.ron' --include='*.rs' korangar/src` → no hits).
* `data\client\skill_actions.ron` (205 rows) and `skill_sounds.ron` (369 rows) loaded with the
  same pattern; `world/skill_action.rs` has no id table (returns `CasterAction`).
* animation action indices (`world/animation/mod.rs:45-65`) are the ACT **format** enum —
  acceptable.
* Remaining hardcodes are the ones in §4.6/§4.3: skill behaviour flags, status auras,
  procedural effect assets, the falcon companion candidates, and curated `SKILL_SOUNDS`.
  Arrow/projectile asset lists are now data (`weapons.ron`), not Rust.

### 6.2 Entities / jobs / classes
Everything in §4.1/4.2. The **weapon** half is now data-driven (merged §4.2); the remaining
hardcodes are the job/appearance stems (§4.1). Also checked and **clean/data-driven**:
`world/library/job_identity.rs` (`jobidentity.lub`), `baby_job.rs` (`JTtbl`),
`monster_sprite.rs` (GRF-verified fallback chain, generated alias table),
`accessory_name.rs` (`accname.lub`/`accessoryid.lub`), `world/entity` NPC/warp handling
(job 45/111/139 only), `world/model` (format version handling, no content ids),
`world/particles` (only quest icon hardcode), `state/minimap` markers are procedurally
generated (documented) — acceptable.
**Headgear rendering** uses the data-driven `accname.lub` table + one path template; headgear
*view logic* (upper/middle/lower ordering, dedupe) is in `entity/mod.rs:576-605` and is
behaviour, fine to keep. Shield/robe view ids are plumbed (networking → `AppearanceChanged` →
`apply_sprite_change`) but the layers still need rendering.

### 6.3 Items / equipment
* names/descriptions/cards/resources: archive data — good (`item_name.rs`,
  `item_display_name.rs`, `item_description.rs`, `card_composition.rs`,
  `idnum2itemdesctable.txt`, `cardprefix/postfixnametable.txt`).
* slots/refinement/cards display: driven by packet fields + `iteminfo.lub` `slotCount`; no id
  ranges. `world/item_details.rs` card slot logic is generic.
* the hardcodes are the Korean stat parser (§4.3), item category/use switch (§4.8), and the
  duplicated `item_type` constants.
* weapon item ids/ranges: **done** — `data\client\weapons.ron` (30 kinds / 47 id ranges /
  707 ids / `item_views`); the item layer only needs the generic item facts (§8 Phase 2).

### 6.4 Status effects
§4.7. Names/icons generated (content OK), semantics/aura hardcoded. No buff/debuff category
data exists — if the UI is to group buffs vs debuffs, that is a new data file, not a bug fix.

### 6.5 UI / interface
* design tokens exist (`state/theme/tokens.rs`: `ColorTokens`, `TypographyTokens`,
  `SpacingTokens`, `RadiusTokens`, `DesignTokens`) and most high-traffic windows use them;
  the ratchet is nevertheless red (§4.5) and 87 raw `f32` layout constants remain across 20
  interface files (mostly component-local sizes; the paper doll/hotbar/canvas layouts are
  the largest).
* keybindings hardcoded (§4.8); default settings (graphics/audio/interface `Default` impls,
  `INITIAL_SCREEN_SIZE 1280x720`, `INITIAL_SCALING_FACTOR 1.0`, volume 50/100%) are legitimate
  defaults — acceptable.
* debug-only windows (`frame_inspector`, `profiler`, `maps`, `commands`, `theme_inspector`,
  `cache`, `packet_inspector`, `render_options`) are excluded from the UI lint and are not
  user-facing; their literals are lower priority (maps content row still P2).
* Error/localization pipeline: `msgstringtable.rs` loads archive `msgstringtable.txt` for
  `ZC_MSG` (0x0291) with glyph-safety fallbacks — the right mechanism, underused by the
  hardcoded result-code tables.

### 6.6 World / maps / lighting / music
* map data (`rsw`/`gat`/`gnd`) and light sources/lighting are archive-driven
  (`loaders/map/mod.rs`); sky data is *loaded but unused* (`_map_sky_data`,
  `loaders/map/mod.rs:136-137` "TODO: Implement sky rendering") — feature gap, not a hardcode.
* map music is data-driven: `data\mp3NameTable.txt` + `resnametable.txt`, folder fallbacks
  (`korangar-audio/src/lib.rs:60-66,345-357,1060-1200`) — good.
* no map-specific branches were found (`map_name ==` does not exist); the only map-name content
  is the debug warp list, the startup `geffen` map/camera, and test fixtures.

### 6.7 Networking / protocol
* **acceptable/generated**: `ragnarok-packets/src/opcode_lengths_2019.rs` (510 entries,
  `@generated by tools/gen_opcode_lengths.py`, rAthena 20190605 + committed supplement JSON),
  `korangar-networking/src/contract_2019.rs` (iterates every registered handler against the
  2019 wire sizes, byte-exact fixtures for critical layouts, documented deterministic
  unknown-packet skip), `ragnarok-formats/*` (SPR/ACT/RSW/GND/GAT parsers with format-version
  branches), `mixcrypt.rs` (archive crypto). These are **P3/OK — do not migrate**.
* documented bespoke interpretations exist but are localized and commented:
  direction nibble decode (`version_20220406.rs:471-487`), heal packet without entity id
  (`1076-1082`), state change 0x0229 (`1132-1140`) — acceptable.
* `ZC_SPRITE_CHANGE` Weapon/Shield/Robe is no longer dropped: the weapon merge routes it to
  `NetworkEvent::AppearanceChanged` (`version_20220406.rs:717`, `event.rs:449`) and
  `apply_sprite_change` updates the weapon layer live — a networking→render improvement.
* the only networking liabilities are the 26 English login-failure strings (§4.4) and the
  duplicated item-type constants (§4.8).

### 6.8 Audio
* engine + music mapping data-driven; volumes are settings; environment mute flags are
  documented operational switches. Hardcodes: 3 UI/teleport sounds, curated skill sounds,
  effect wav paths inside procedural effects (§4.6/4.8). `KORANGAR_MUTE_AUDIO` /
  `KORANGAR_MUSIC_OFF` are test infrastructure — acceptable.

### 6.9 Party / social / chat
* limits: `MAXIMUM_PARTY_MEMBERS = 12` (`state/party.rs:16`), friend/party name 4–24
  (`friend_list.rs:20-21`, `party.rs:28`, `character_creation.rs:12-13`), chat message 80
  (`windows/chat.rs:25`), hotbar 4 tabs × 38 (`state/hotbar.rs:21-25`) — these mirror the
  2019 protocol/server constants and are acceptable, but they are **duplicated** (name
  lengths) and the chat/party limit values should at least be documented as server contract.
* commands: `input/chat_command.rs:18` only lists 3 client-side commands (`nc`, `w`, `whisper`)
  and otherwise forwards `/x` → `@x` generically — clean; no per-command table.
* chat colours use theme tokens for categories and pass through server RGB — good.
* party invitation/creation/member-removal result messages are the hardcoded tables of §4.4.
* no party exp-share/category UI data tables were found (the event exists); not a hardcode.

---

## 7. OK / generated tables — do not re-audit

| artefact | entries | provenance / verification | status |
| --- | --- | --- | --- |
| `ragnarok-packets/src/opcode_lengths_2019.rs` | 510 | `@generated by tools/gen_opcode_lengths.py` (rAthena 20190605 + `tools/opcode_supplement_20190605.json`), contract tests | **OK** |
| `korangar-networking/src/contract_2019.rs` | every handler | offline 2019 wire contract test + byte-exact fixtures | **OK** |
| `korangar/archive/data/client/skill_effects.ron` | 436 phases / 603 layers | `tools/gen_skill_effects.py --check`; `--check-skill-effects` CLI; tests | **OK** |
| `korangar/archive/data/client/skill_actions.ron` | 205 | `tools/gen_skill_actions.py --check`, include_str fallback, archive override | **OK** |
| `korangar/archive/data/client/skill_sounds.ron` | 369 | `tools/gen_skill_sounds.py --check`, GRF-verified wavs | **OK** |
| `korangar/archive/data/client/weapons.ron` | 381 lines; 30 kinds / 30 server types / 47 id ranges / 707 ids / Arrow projectile | `tools/gen_weapons.py --check`, `tools/test_gen_weapons.py` (12 tests), facts `rathena_pre_re_weapon_items.json` + `weapon_client_assets.json`, include_str fallback + archive override, 11 module tests; **merged `4005da9e`** | **OK** |
| `world/library/skill_delays.rs` | 560 rows | `@generated by tools/gen_skill_delays.py` from pre-re `skill_db.yml`, `--check` | **OK (P3: not archive-overridable)** |
| `world/library/skill_icon_alias.rs` | 241 | generated from GRF file probe; header documents categories | OK content, **generator untracked (P2)** |
| `world/library/monster_sprite_alias.rs` | 232 | generated from GRF probe (all 1008 mob ids verified) | OK content, generator untracked |
| `world/library/job_alias.rs` | 63 sprite aliases + 56 skill-tree fallbacks | generated from GRF probe | OK content, generator untracked |
| `state/status_effect_names.rs` | 509 | rAthena `status.hpp` enum + client `stateiconinfo.lub`, ASCII-filtered | OK content, generator untracked |
| `state/status_effect_icons.rs` | 355 | official status icon table + `stateiconimginfo.lub`, GRF-verified | OK content, generator untracked |
| item/description/card tables | — | archive data (`iteminfo.lub`, `idnum2itemdesctable.txt`, `cardprefix/postfixnametable.txt`, `accname.lub`, `jobidentity.lub`, `skillinfo*.lub`, `skilltree*.lub`) | **OK** |
| map files/lighting/music | — | archive `rsw/gat/gnd`, `mp3NameTable.txt`, `resnametable.txt` | **OK** |
| localization files | 132 keys × 2 | `en-US.ron`/`de-DE.ron`, parity + serde-default back-compat tests | **OK (coverage is the gap)** |
| classic level-99 aura / classic skin | — | documented reproduction of official hardcoded client effects; optional textures | **OK (P3)** |
| legacy `skill_effect_assets/procedural/sprite.ron` | 572 lines | **generator inputs only, not loaded at runtime** (verified: no Rust references) | **OK, don't audit at runtime** |

Also explicitly **not** hardcode problems (keep as-is): packet structs/opcodes/lengths,
`item_types`/`EquipPosition`/ammo/`SkillType` protocol enums (but dedupe the item-type copy),
ACT/SPR/RSW/GND format parsing and animation action indices, mixcrypt archive crypto,
`TILE_SIZE=10`/`SPRITE_SCALE`/effect-unit rendering constants, chat 80/party 12/hotbar 4×38
protocol limits, settings `Default` values, environment variables used for tests/mute.

---

## 8. Phased remediation plan (aligned with the skill/weapon pipeline)

Principle: **no new bespoke table in Rust**. Every phase produces the same five artefacts:
facts JSON → generator with `--check` → `data\client\<system>.ron` (provenance + version +
sorted) → loader with `include_str!` fallback + archive override + schema validation →
`--check-<system>` CLI + tests + a short `notes/` report. Phases are ordered so the P0/CI work
is not disturbed and the merged weapon registry (Phase 1a) is the template.

### Phase 0 — restore gates and stop the localization bleed (P0, ~1–2 days, no game data change)
1. **Token ratchet**: migrate the 29/16 literals to `state/theme/tokens.rs`
   (item_info 7, minimap 6, chat 2, party/spirit/skill-description/char-selection; party 4 and
   character_selection 4 FontSizes). Do **not** raise `korangar/test-data/ui-token-budget.ron`;
   `./scripts/ui-token-budget.sh` must exit 0. Optionally extend `ui_consistency.rs` with a
   spacing/position ratchet for the paper doll/hotbar.
2. **Localization sweep 2**: add `Localization` fields (with `default_*` serde defaults, as the
   existing struct does) for the 68 literal strings + 13 paper-doll labels + 81 dropdown labels;
   update `en-US.ron`/`de-DE.ron`; extend the lint's forbidden-word list; localize the 26 login
   messages by moving them to the message table/localization.
3. **Result-code tables**: decide the target (msgstringtable ids vs localization keys) for the
   69 `lib.rs` + 26 networking mappings; implement as one `data\client\message_ids.ron` with
   generated defaults; leave protocol codes untouched.
4. Keep `./scripts/ui-token-budget.sh`, `ui_consistency`, and the new message tests in CI.

**Acceptance:** all gates green; zero new hardcoded UI literals; no behaviour change.

### Phase 1a — weapon centralization (DONE, merged `4005da9e`)
`world/weapon.rs` + `data\client\weapons.ron` + `tools/gen_weapons.py --check` +
`tools/test_gen_weapons.py` landed; the 22-arm suffix table, bow range/`W_BOW`, arrow
`CANDIDATES`, five duplicated weapon-animation load blocks and dropped `ZC_SPRITE_CHANGE`
Weapon/Shield/Robe packets are gone (verified by grep on the merged tree). The only residual
weapon item is the auto-attack sound TODO (`entity/mod.rs:1046–1050`); the attack action is
intentionally weapon-independent. **Do not redo this work; use it as the template.**

### Phase 1b — entity appearance / job data (P1, ~1 week, next)
Owner: an appearance follow-up lane (does not overlap weapons).
1. New `data\client\jobs.ron` + `--check-jobs` merging `job_alias` data: job id → sprite stem,
   palette stem, sex suffixes, entity type, baby/transcendent flags, scale. Generator merges
   client `jobidentity`/GRF probe facts + rAthena `job_db`; delete the 146-arm match, hair
   arrays, palette overrides and `BABY_JOB_SCALE` magic.
2. Add shield/robe profiles (view id → part candidates) to the appearance/weapon data and
   render the layers (closes `entity/mod.rs:255–259`); route headgear through the same
   appearance path helper (`library/mod.rs:264–284`, keep `accname.lub` names).
3. Extend the appearance probe test (currently the ignored `job_audit` test) into a CI
   `--check-appearance` that validates every job/weapon/shield/robe sprite exists in the
   archive, and a generator `--check` for `jobs.ron`.
4. Add the auto-attack weapon sound via a `attack_sound` field in `weapons.ron`.

**Acceptance:** no per-job match arms in `entity/mod.rs`; `world/weapon.rs` no longer imports
`get_sprite_path_for_player_job`; shield/robe visible in world; `--check-*` green; the existing
job audit test no longer needs to skip 4th/expanded jobs.

### Phase 2 — item facts and stats (P1, ~1 week)
1. `tools/extract_item_facts.py` → committed JSON from rAthena `item_db_equip.yml`/`item_db.yml`
   (id, type, sub-type, slots, refine, view, equip position, usable, weapon/armor level).
2. `tools/gen_item_stats.py --check` → `data\client\item_stats.ron` (item id → structured
   stats/bonuses/requirements), consumed by `world/library/item_stats.rs`; keep the client
   description text for display only. Delete the Korean grammar parser or keep it behind a
   clearly-logged last-resort fallback.
3. Data-driven item category/use action; single `item_type` module used by both the inventory
   state and the networking version handler (remove the duplicate constants).
4. `--check-items` CLI + tests (unknown item id → documented fallback, not silent zero).

**Acceptance:** English `iteminfo.lub` yields the same stat panel as Korean data; no Korean
grammar dependency in the default path; one item-type constant source.

### Phase 3 — status effects data (P2, ~3–4 days)
1. `tools/extract_status_facts.py` (rAthena `status.hpp` `efst_type`, `status_db`, client
   `stateiconinfo.lub`/`stateiconimginfo.lub`/`efstids.lub`) → JSON.
2. `tools/gen_status_effects.py --check` → `data\client\status_effects.ron`:
   id → name, icon, buff/debuff category, value label/format, stack/refresh policy, aura spec
   (texture/tint/radii/timing/kind). Merge aura specs so Blessing/Inc AGI/Angelus/Concentration
   get auras without new Rust arms.
3. `world/effect/status_aura.rs` becomes a generic renderer fed by the aura spec; localize the
   value-label templates; commit the 5 missing alias generators (or move those tables into the
   same data file).

**Acceptance:** new status/aura = data row only; buff bar can group by category; no status id
literals outside the data file.

### Phase 4 — skill behaviour + procedural layer data (P2, ~3–5 days)
1. Extend `skill_effects.ron` schema (or a sibling `skills.ron` in the same loader style) with:
   `channeling` (Rolling Cutter 2036), `warp_target` (Warp 27), `move_while_casting`
   (Free Cast 278 + jobs 16/4017 → a skill flag + job capability field in `jobs.ron`),
   `cast_circle`, and a `Cloud`/generic `Primitive` layer for Storm Gust
   (texture list/count/size/timing/height).
2. Move `SKILL_SOUNDS` curated rows into `tools/skill_sounds_custom.ron` merged by the
   existing generator (precedence stays curated → per-layer wav → generated → generic).
3. Fold procedural effect texture names (aura/cast-circle/warp/status aura/projectile) into
   optional data fields with the current values as defaults.
4. Add tests that the special-case branches are gone (`greps` as source tests), plus a data
   `--check-skill-behaviour`.

**Acceptance:** no skill-id branch in `lib.rs`; curated sound file exists; procedural effects
still render with the committed defaults.

### Phase 5 — UI architecture cleanup (P2, ~3–5 days)
1. Keybindings table (`data\client\keybinds.ron` + settings UI) with current binds as defaults.
2. Paper-doll slot table in the theme or a data file (positions + localized slot names);
   theme spacing/radius for the remaining raw `f32` layout constants that TODO calls out.
3. Deduplicate name-length constants (4/24) into one module and use it in the windows + `lib.rs`.
4. Localize settings/dropdown labels; extend the lint to catch `fn text(&self)` literals.

### Phase 6 — world/audio/social polish (P3, opportunistic)
* debug map warp list → data file; `DEFAULT_MAP`/`START_CAMERA_FOCUS_POINT`/`UI_BACKGROUND_MUSIC`
  → client config; quest icon path/colours → data + theme; teleport/menu sounds → data file;
  localize remaining client chat feedback strings (or route through message table).
* Document, don't migrate: protocol enums/lengths, chat/party/hotbar protocol limits.

### Suggested sequencing / parallelization
* **Done:** Phase 1a (weapon registry, merged `4005da9e`).
* **Now (parallel safe):** Phase 0 (UI team), Phase 1b (appearance/job lane), Phase 2
  extractor/generator (items) — disjoint files.
* **After Phase 1b:** Phase 3 (status) and Phase 4 (skill behaviour) both consume `jobs.ron`
  flags, so sequence 4 after 1b if possible.
* **Last:** Phase 5/6 polish, and commit the five missing alias generators early (cheap, P2).

---

## 9. Risks, non-goals and verification

* **Do not migrate** protocol/format tables (`opcode_lengths_2019`, contract fixtures,
  `ragnarok-formats`, `mixcrypt`, ACT/SPR/RSW/GND parsing, packet structs, `EquipPosition`,
  hotbar 4×38, chat 80, party 12). They are generated/contract-tested and must match the wire
  or file formats. The only cleanup there is deduplicating the item-type enum.
* **Do not re-audit** the skill-effect pipeline, its data, or the merged weapon registry/
  `weapons.ron`; the remaining skill work is behavioural flags (Phase 4) and status auras
  (Phase 3), both additive. The residual appearance work is §4.1/4.2/Phase 1b.
* **Tree pin:** citations were verified at `4005da9e` (merge of `feat/weapon-centralization`).
  The report was first committed at `85dc53cb` and is updated here; if HEAD moved after this
  revision, re-check `git log -1` and expect small line shifts in `lib.rs`/`entity/mod.rs`.
* **Avoid raising the token budget**: `scripts/ui-token-budget.sh --update` refuses raises by
  design; the 23→28 bump in `818ab3e1` is the reason the gate is red now.
* **Careful with archive overrides**: the `include_str!` + `OnceLock` loader pattern logs and
  falls back on malformed data; keep `deny_unknown_fields` so typos are loud, and add a
  `--check-<system>` CLI for every new data file.
* **Provenance**: every new generator must take a committed facts JSON and support `--check`,
  with the extraction source/version in the module doc (the weapon extractor's doc comment is
  the model: server tree SHA + rAthena files).
* **No Wine / no user characters**: this audit made no game sessions; all counts are source
  scans and the committed data files. Reproduction commands:
  `./scripts/ui-token-budget.sh`, `rg -n`, the line lists above, and
  `python3 - <<'EOF' … EOF` scripts for literal counts (see appendix).

---

## 10. Appendix — how the numbers were produced

```bash
# HEAD pinned for this revision (audit start 935c3ecd; weapon merge 4005da9e)
git log -1 --format='%H %s'
git log --oneline -5
# weapon centralization landed (merged into integration)
git merge-base --is-ancestor f85b1433a44edc788c4603bf032da88c36203039 HEAD && echo merged
rg -n 'weapon_suffix_candidates|BOW_ITEM_VIEW_RANGE|WEAPON_TYPE_BOW|is_bow_weapon|arrow_assets_failed' \
   --include='*.rs' korangar/src korangar-networking/src   # expected: no output
rg -n 'request_weapon_animation|projectile_for_weapon|weapons.ron' korangar/src/lib.rs korangar/src/world/weapon.rs
# weapon data drift + generator invariants
python3 tools/gen_weapons.py --check
python3 tools/test_gen_weapons.py
# interface style tokens (CI gate)
./scripts/ui-token-budget.sh            # exit 1 at HEAD: colors 29 > 28, FontSize 16 > 12
./scripts/ui-token-budget.sh --report
# asset-path / Korean literals (per-file counts; comment lines excluded)
python3 - <<'EOF'
import os,re
# iterate korangar/src, korangar-audio/src; count "..." literals containing
# \\인간족|몬스터|아이템|악세사리|이팩트|effect|texture|sprite|\.wav|\.tga|\.bmp|\.spr|\.act
EOF
# numeric tables/match arms
rg -n '^\s+(\d+|0x[0-9A-Fa-f]+)\s*(\||=>)' korangar/src
# generated table entries
rg -c 'SkillDelayRow \{' korangar/src/world/library/skill_delays.rs          # 560
rg -c 'kind: ' korangar/archive/data/client/skill_effects.ron               # 603
rg -c 'skill: ' korangar/archive/data/client/skill_effects.ron              # 436
# runtime data-file loaders (4 after the weapon merge)
rg -l 'include_str!\("../../../archive/data/client' korangar/src
#   skill_effects.rs, skill_actions.rs, skill_sounds.rs, world/weapon.rs
# legacy RON runtime usage (none)
rg -n 'skill_effect_assets.ron|skill_sprite_effects.ron|skill_procedural_effects.ron' --include='*.rs' korangar/src
# localization literal scan, replicating tests/ui_consistency.rs rules
python3 - <<'EOF'
# walk korangar/src/interface, skip DEBUG_ONLY_FILES, skip comment lines and
# lines containing localization(); count text/tooltip/ghost_text/title literals
EOF
```

Key verification artifacts referenced above:
`notes/korangar-skill-effect-pipeline.md` (pipeline pattern + verification),
`notes/skill-effects-official-data.md` (official effect rows),
`notes/korangar-weapon-centralization.md` + `notes/korangar-weapon-centralization-evidence/`
(weapon design, provenance, in-client evidence),
`tools/gen_skill_effects.py` / `tools/gen_weapons.py` (generator + `--check`),
`korangar/src/world/library/skill_effects.rs` / `korangar/src/world/weapon.rs`
(loader/schema/placement), and the committed facts JSONs under `tools/`.

*End of audit — read-only; no source files were modified by this audit. This report was first
committed as `85dc53cb` and is updated here for the merged weapon centralization (`4005da9e`).*
