//! Deterministic, headless benchmark scenario driver.
//!
//! Enabled with `KORANGAR_BENCH=<scenario>`. The driver scripts a fixed
//! sequence of stages against a real client session so frame CPU time,
//! allocation churn and async loader latencies can be compared between two
//! builds with the same command:
//!
//! 1. `login`      - log in with the configured test account, select the first
//!    character and enter the map.
//! 2. `town_idle`  - stand still in the town (crowded with population-engine
//!    shells) and let the world render.
//! 3. `town_walk`  - walk a fixed loop around the spawn point, crossing the
//!    crowded plaza (entity updates, animation loads).
//! 4. `effects`    - burst of world effects + damage/heal/miss numbers around
//!    the player.
//! 5. `ui_updates` - open the common windows, inject a known buff set and
//!    invalidate the window layouts every frame (the worst case for a busy
//!    network frame), so buff bar / UI label churn is measured.
//! 6. `map_change` - request three map loads through the real async loader and
//!    record the completion latency.
//!
//! Every stage prints a machine readable `[bench-stage] {...}` line (also
//! appended to `KORANGAR_BENCH_OUT` when set, see [`crate::perf`]). Per-stage
//! async loader latencies are printed as `[bench-load] {...}` and the map load
//! samples as `[bench-map] {...}`. `scripts/perf/perf-baseline.sh` parses these
//! lines into a JSON baseline.
//!
//! Environment variables:
//!
//! * `KORANGAR_BENCH` - scenario name (`full`, `town`, `ui`, `effects`, `map`).
//! * `KORANGAR_BENCH_USER` / `KORANGAR_BENCH_PASS` - test account credentials.
//! * `KORANGAR_BENCH_SLOT` - character slot to select (default `0`).
//! * `KORANGAR_BENCH_IDLE_SECONDS`, `KORANGAR_BENCH_WALK_SECONDS`,
//!   `KORANGAR_BENCH_EFFECTS_SECONDS`, `KORANGAR_BENCH_UI_SECONDS` - stage
//!   durations (defaults 8, 12, 6, 8).
//! * `KORANGAR_BENCH_OUT` - append the raw benchmark lines to this file.
//! * `KORANGAR_BENCH_CASTER_POSE` - also play each swept skill's caster pose
//!   (the `skills` stage otherwise only exercises the effect pipeline).
//! * `KORANGAR_BENCH_SKIP_AURAS` - do not inject the persistent status auras
//!   (useful when capturing the caster body pose).
//!
//! The driver never runs unless `KORANGAR_BENCH` is set, so normal clients are
//! not affected.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use cgmath::Vector3;
use ragnarok_packets::{ClientTick, EntityId, SkillId, TilePosition};

use crate::graphics::Color;
use crate::input::InputEvent;
use crate::interface::windows::WindowClass;
use crate::loaders::{ClientInfoPathExt, ImageType};
use crate::state::{ClientStatePathExt, client_state, status_effect_definition, this_entity};
use crate::world::{DamageNumber, HealNumber, Miss, SkillPhase};
use crate::{Client, perf};

/// Windows counted for the `windows_mean` metric of a stage.
const COUNTED_WINDOWS: &[WindowClass] = &[
    WindowClass::Chat,
    WindowClass::Hotbar,
    WindowClass::Inventory,
    WindowClass::Equipment,
    WindowClass::Stats,
    WindowClass::SkillTree,
    WindowClass::Minimap,
    WindowClass::BuffBar,
    WindowClass::FriendList,
    WindowClass::Menu,
    WindowClass::Login,
    WindowClass::SelectServer,
    WindowClass::CharacterSelection,
];

/// Duration used for the named `Skill`/`Attack` poses of the opt-in caster-pose
/// mode; explicit `Action(n)` rows play the native ACT duration.
const BENCH_SKILL_ACTION_MS: u32 = 400;

/// Skill ids driven by the `skills` scenario: a representative pre-renewal
/// selection that covers every layer kind (`.str`, `.spr`, texture billboards,
/// cylinders, flipbooks) and the data added by the rodecomp audit.
const BENCHMARK_SKILLS: [u16; 47] = [
    5, 7, 14, 17, 19, 26, 28, 29, 32, 33, 34, 36, 38, 45, 47, 54, 59, 62, 66, 68, 70, 73, 75, 78, 83, 84, 86, 89, 110, 123, 136, 153, 231,
    263, 306, 317, 325, 329, 355, 361, 400, 402, 451, 465, 476, 513, 540,
];

/// Persistent status effects injected by the `skills` scenario so the status
/// auras/fades are rendered next to the skill effects.
/// Includes 103 (Aura Blade, additive weapon layer), 107 (Berserk, additive
/// body) and 113 (Magical Power, weapon blink) so the status sprite renderer
/// is exercised by the headless sweep.
const BENCHMARK_AURA_STATUSES: [u16; 18] = [2, 25, 31, 68, 86, 87, 103, 107, 110, 113, 123, 161, 165, 188, 232, 235, 437, 473];

/// Status effects injected by the `skills` scenario: the defaults, or the
/// comma separated `KORANGAR_BENCH_AURAS` override (to isolate one status).
fn aura_list() -> Vec<u16> {
    match environment("KORANGAR_BENCH_AURAS") {
        Some(value) if !value.trim().is_empty() => value.split(',').filter_map(|entry| entry.trim().parse::<u16>().ok()).collect(),
        _ => BENCHMARK_AURA_STATUSES.to_vec(),
    }
}

/// Status effects injected for the `ui_updates` stage so the buff bar has a
/// known, non-empty work set.
const BENCHMARK_BUFFS: [u16; 10] = [0, 1, 2, 3, 9, 10, 12, 15, 241, 242];

/// Tile offsets (relative to the town spawn point) that the walking stage
/// cycles through. Deliberately a loop through the plaza area.
const WALK_OFFSETS: [(i32, i32); 12] = [
    (4, 0),
    (8, 4),
    (8, 10),
    (2, 14),
    (-6, 12),
    (-10, 6),
    (-10, 0),
    (-6, -6),
    (0, -8),
    (6, -6),
    (4, 0),
    (0, 0),
];

/// On-screen effect files spawned by the `effects` stage. Both are standard
/// 2019 `.str` effects that exist in the CondaRO archives.
const BENCHMARK_EFFECTS: [&str; 2] = ["pneuma1.str", "firewall.str"];

fn environment(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn environment_seconds(name: &str, default: f64) -> f64 {
    environment(name)
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(default)
}

fn scenario_enabled() -> bool {
    // Latched: this is called once per frame from the render loop, and
    // `std::env::var` allocates a String each call.
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

    *ENABLED.get_or_init(|| environment("KORANGAR_BENCH").is_some())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Boot,
    Login,
    TownIdle,
    TownWalk,
    Effects,
    SkillSweep,
    UiUpdates,
    MapChange,
    Done,
}

struct BenchState {
    scenario: String,
    username: String,
    password: String,
    slot: usize,
    idle_seconds: f64,
    walk_seconds: f64,
    effects_seconds: f64,
    ui_seconds: f64,

    stage: Stage,
    stage_started: Instant,
    next_action: Instant,
    started: Instant,

    /// Spawn tile recorded when the walking stage starts.
    spawn_tile: Option<TilePosition>,
    walk_index: usize,

    /// Number of effects spawned so far in the effects stage.
    effects_spawned: u32,
    buffs_applied: bool,
    windows_opened: bool,
    /// Skip the injected status auras (`KORANGAR_BENCH_SKIP_AURAS`), so the
    /// caster body pose is not hidden under the aura layers.
    skip_auras: bool,

    /// Currently loading benchmark map (name + request time) and the samples.
    pending_map: Option<(String, Instant)>,
    map_pointer_before: usize,
    map_change_next: usize,
    map_samples: Vec<(String, f64)>,

    finished: bool,
    failure: Option<String>,
    /// Skills driven by the `skills` scenario (`KORANGAR_BENCH_SKILLS`
    /// override).
    skill_list: Vec<u16>,
    /// Pause between two skill spawns.
    skill_delay: Duration,
    /// Also play the caster pose of every swept skill
    /// (`KORANGAR_BENCH_CASTER_POSE`).
    caster_pose: bool,
}

static STATE: OnceLock<Mutex<Option<BenchState>>> = OnceLock::new();

fn state() -> &'static Mutex<Option<BenchState>> {
    STATE.get_or_init(|| Mutex::new(None))
}

/// Drive the benchmark scenario. Called once per frame from the client's frame
/// update; a no-op unless `KORANGAR_BENCH` is set.
pub fn drive(client: &mut Client, client_tick: u64) {
    if !scenario_enabled() {
        return;
    }

    let mut guard = state().lock().unwrap();

    if guard.is_none() {
        let window_size = client.graphics_engine.get_window_size();
        *guard = Some(initial_state((window_size.x, window_size.y)));
    }

    let bench = guard.as_mut().unwrap();

    if bench.finished {
        return;
    }

    if let Some(failure) = bench.failure.clone() {
        perf::emit(&format!(
            r#"[bench-error] {{"stage":"{:?}","reason":"{}"}}"#,
            bench.stage,
            escape(&failure)
        ));
        bench.finished = true;
        client.session.request_shutdown();
        return;
    }

    let now = Instant::now();

    // Record the current window count for the stage metrics (cheap; a handful of
    // class lookups instead of touching the interface internals).
    if perf::enabled() {
        let windows = COUNTED_WINDOWS
            .iter()
            .filter(|window_class| client.interface.is_window_with_class_open(**window_class))
            .count();
        perf::sample_windows(windows as u32);
    }

    match bench.stage {
        Stage::Boot => stage_boot(client, bench, now, client_tick),
        Stage::Login => stage_login(client, bench, now),
        Stage::TownIdle => stage_town_idle(client, bench, now),
        Stage::TownWalk => stage_town_walk(client, bench, now),
        Stage::Effects => stage_effects(client, bench, now),
        Stage::SkillSweep => stage_skill_sweep(client, bench, now, client_tick),
        Stage::UiUpdates => stage_ui_updates(client, bench, now, client_tick),
        Stage::MapChange => stage_map_change(client, bench, now),
        Stage::Done => {}
    }
}

fn initial_state(window_size: (usize, usize)) -> BenchState {
    let scenario = environment("KORANGAR_BENCH").unwrap_or_else(|| "full".to_owned());
    let username = environment("KORANGAR_BENCH_USER").unwrap_or_default();
    let password = environment("KORANGAR_BENCH_PASS").unwrap_or_default();
    let slot = environment("KORANGAR_BENCH_SLOT").and_then(|value| value.parse().ok()).unwrap_or(0);

    let idle_seconds = environment_seconds("KORANGAR_BENCH_IDLE_SECONDS", 8.0);
    let walk_seconds = environment_seconds("KORANGAR_BENCH_WALK_SECONDS", 12.0);
    let effects_seconds = environment_seconds("KORANGAR_BENCH_EFFECTS_SECONDS", 6.0);
    let ui_seconds = environment_seconds("KORANGAR_BENCH_UI_SECONDS", 8.0);

    let now = Instant::now();

    let (window_width, window_height) = window_size;

    perf::emit(&format!(
        r#"[bench-start] {{"scenario":"{}","idle_s":{idle_seconds},"walk_s":{walk_seconds},"effects_s":{effects_seconds},"ui_s":{ui_seconds},"window":"{window_width}x{window_height}"}}"#,
        escape(&scenario),
    ));

    BenchState {
        scenario,
        username,
        password,
        slot,
        idle_seconds,
        walk_seconds,
        effects_seconds,
        ui_seconds,
        stage: Stage::Boot,
        stage_started: now,
        next_action: now,
        started: now,
        spawn_tile: None,
        walk_index: 0,
        effects_spawned: 0,
        buffs_applied: false,
        windows_opened: false,
        skip_auras: environment("KORANGAR_BENCH_SKIP_AURAS").is_some_and(|value| value != "0"),
        pending_map: None,
        map_pointer_before: 0,
        map_change_next: 0,
        map_samples: Vec::new(),
        finished: false,
        failure: None,
        skill_list: skill_list(),
        skill_delay: Duration::from_millis((environment_seconds("KORANGAR_BENCH_SKILL_DELAY", 0.45) * 1000.0) as u64),
        caster_pose: environment("KORANGAR_BENCH_CASTER_POSE").is_some_and(|value| value != "0"),
    }
}

/// Skill ids driven by the `skills` scenario. `KORANGAR_BENCH_SKILLS`
/// ("34,306,...") overrides the default representative list so a verification
/// run can target a single effect.
fn skill_list() -> Vec<u16> {
    match environment("KORANGAR_BENCH_SKILLS") {
        Some(value) if !value.trim().is_empty() => value.split(',').filter_map(|entry| entry.trim().parse::<u16>().ok()).collect(),
        _ => BENCHMARK_SKILLS.to_vec(),
    }
}

// ---------------------------------------------------------------------------
// Stages
// ---------------------------------------------------------------------------

fn stage_boot(client: &mut Client, bench: &mut BenchState, now: Instant, _client_tick: u64) {
    // The menu map (and the login window) need to be ready before we can log in.
    let login_window_open = client.interface.is_window_with_class_open(WindowClass::Login);

    if !login_window_open {
        if now.duration_since(bench.started) > Duration::from_secs(120) {
            bench.failure = Some("login window did not open within 120s (menu map load failed?)".to_owned());
        }
        return;
    }

    if bench.username.is_empty() || bench.password.is_empty() {
        bench.failure = Some("KORANGAR_BENCH_USER / KORANGAR_BENCH_PASS are not set".to_owned());
        return;
    }

    let Some(service_id) = client
        .client_state
        .follow(client_state().client_info().services())
        .first()
        .map(|service| service.service_id())
    else {
        bench.failure = Some("clientinfo has no service entry".to_owned());
        return;
    };

    perf::begin_stage("login");

    client.input_event_buffer.push(InputEvent::LogIn {
        service_id,
        username: bench.username.clone(),
        password: bench.password.clone(),
    });

    bench.stage = Stage::Login;
    bench.stage_started = now;
    bench.next_action = now + Duration::from_secs(30);
}

fn stage_login(client: &mut Client, bench: &mut BenchState, now: Instant) {
    let select_server_open = client.interface.is_window_with_class_open(WindowClass::SelectServer);
    let character_selection_open = client.interface.is_window_with_class_open(WindowClass::CharacterSelection);
    let in_game = client.map.is_some() && client.client_state.try_follow(this_entity()).is_some();

    if in_game {
        finish_stage(bench, "login");
        let next = match bench.scenario.as_str() {
            "skills" => Stage::SkillSweep,
            _ => Stage::TownIdle,
        };
        let name = match next {
            Stage::SkillSweep => "skill_sweep",
            _ => "town_idle",
        };
        begin_stage_with(bench, now, next, name);
        return;
    }

    if now.duration_since(bench.stage_started) > Duration::from_secs(90) {
        bench.failure = Some(format!(
            "login flow timed out (select_server={select_server_open}, character_selection={character_selection_open})"
        ));
        return;
    }

    if select_server_open && !character_selection_open {
        if now < bench.next_action {
            return;
        }

        let Some(character_server) = client.client_state.follow(client_state().character_servers()).first().cloned() else {
            // The login data has not arrived yet; retry shortly.
            bench.next_action = now + Duration::from_millis(250);
            return;
        };

        client.input_event_buffer.push(InputEvent::SelectServer {
            character_server_information: character_server,
        });

        // Retry slowly; the character server can take a moment to answer.
        bench.next_action = now + Duration::from_secs(3);
        return;
    }

    if character_selection_open && now >= bench.next_action {
        client.input_event_buffer.push(InputEvent::SelectCharacter { slot: bench.slot });
        bench.next_action = now + Duration::from_secs(3);
    }
}

fn stage_town_idle(client: &mut Client, bench: &mut BenchState, now: Instant) {
    if !in_game(client) {
        bench.failure = Some("lost the map while idling in town".to_owned());
        return;
    }

    if now.duration_since(bench.stage_started).as_secs_f64() >= bench.idle_seconds {
        finish_stage(bench, "town_idle");

        // Remember where we are standing so the walking loop is deterministic.
        bench.spawn_tile = player_tile(client);
        bench.walk_index = 0;

        begin_stage_with(bench, now, Stage::TownWalk, "town_walk");
        bench.next_action = now;
    }
}

fn stage_town_walk(client: &mut Client, bench: &mut BenchState, now: Instant) {
    if !in_game(client) {
        bench.failure = Some("lost the map while walking in town".to_owned());
        return;
    }

    if now.duration_since(bench.stage_started).as_secs_f64() >= bench.walk_seconds {
        finish_stage(bench, "town_walk");
        begin_stage_with(bench, now, Stage::Effects, "effects");
        bench.effects_spawned = 0;
        bench.next_action = now;
        return;
    }

    if now >= bench.next_action {
        let base = bench
            .spawn_tile
            .or_else(|| player_tile(client))
            .unwrap_or(TilePosition { x: 156, y: 191 });
        let (offset_x, offset_y) = WALK_OFFSETS[bench.walk_index % WALK_OFFSETS.len()];
        bench.walk_index += 1;

        client.input_event_buffer.push(InputEvent::PlayerMove {
            destination: TilePosition {
                x: (i32::from(base.x) + offset_x).clamp(0, i32::from(u16::MAX)) as u16,
                y: (i32::from(base.y) + offset_y).clamp(0, i32::from(u16::MAX)) as u16,
            },
        });

        bench.next_action = now + Duration::from_millis(700);
    }
}

/// Drives every representative skill through the real spawn path and injects
/// the persistent status effects, so a headless run renders (and logs) the
/// whole skill effect pipeline without needing GM commands on the server.
fn stage_skill_sweep(client: &mut Client, bench: &mut BenchState, now: Instant, client_tick: u64) {
    if !in_game(client) {
        bench.failure = Some("lost the map while sweeping skills".to_owned());
        return;
    }

    let Some(entity_id) = client.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) else {
        return;
    };
    let Some(position) = player_position(client) else {
        return;
    };

    if !bench.buffs_applied {
        if !bench.skip_auras {
            apply_statuses(client, client_tick, &aura_list());
        }
        bench.buffs_applied = true;
        bench.next_action = now + Duration::from_millis(500);
        return;
    }

    let index = bench.effects_spawned as usize;
    if index < bench.skill_list.len() {
        if now < bench.next_action {
            return;
        }

        let skill_id = bench.skill_list[index];
        if bench.caster_pose {
            apply_bench_caster_pose(client, entity_id, skill_id, ClientTick(client_tick as u32));
        }
        let phases = [SkillPhase::Success, SkillPhase::Ground, SkillPhase::Cast, SkillPhase::Hit];
        let phase = phases
            .iter()
            .copied()
            .find(|phase| !crate::world::skill_effect_layers(SkillId(skill_id), *phase).is_empty());

        if let Some(phase) = phase {
            client.spawn_skill_effect_phase(
                SkillId(skill_id),
                phase,
                entity_id,
                Some(entity_id),
                Some(position),
                ClientTick(client_tick as u32),
            );
            perf::emit(&format!(
                r#"[bench-skill] {{"skill":{skill_id},"phase":"{phase:?}","index":{index}}}"#
            ));
        }

        bench.effects_spawned = bench.effects_spawned.saturating_add(1);
        bench.next_action = now + bench.skill_delay;
        return;
    }

    // Let the last effects play out, then end the scenario.
    if now >= bench.next_action + Duration::from_secs(4) {
        finish_stage(bench, "skill_sweep");
        bench.stage = Stage::Done;
        bench.finished = true;
    }
}

/// Plays a swept skill's caster pose through the same entity methods as the
/// real network handlers (see the `SkillCast` arm in `lib.rs`). Opt-in via
/// `KORANGAR_BENCH_CASTER_POSE=1`, so normal effect sweeps are unchanged.
fn apply_bench_caster_pose(client: &mut Client, entity_id: EntityId, skill_id: u16, client_tick: ClientTick) {
    let Some(entity) = client
        .client_state
        .follow_mut(client_state().entities())
        .iter_mut()
        .find(|entity| entity.get_entity_id() == entity_id)
    else {
        return;
    };

    match crate::world::caster_action(SkillId(skill_id)) {
        crate::world::CasterAction::Idle => entity.set_idle(client_tick),
        crate::world::CasterAction::Skill => entity.set_skill_cast(BENCH_SKILL_ACTION_MS, false, client_tick),
        crate::world::CasterAction::Attack => entity.set_attack(BENCH_SKILL_ACTION_MS, false, client_tick),
        crate::world::CasterAction::Pickup => entity.set_pickup(client_tick),
        crate::world::CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
        crate::world::CasterAction::Action(action_index) => entity.set_custom_skill_action(action_index as usize, 0, false, client_tick),
    }
}

fn stage_effects(client: &mut Client, bench: &mut BenchState, now: Instant) {
    if !in_game(client) {
        bench.failure = Some("lost the map while spawning effects".to_owned());
        return;
    }

    let elapsed = now.duration_since(bench.stage_started).as_secs_f64();

    if elapsed >= bench.effects_seconds {
        finish_stage(bench, "effects");
        begin_stage_with(bench, now, Stage::UiUpdates, "ui_updates");
        return;
    }

    // Spawn for the first half of the stage, then let everything animate. The
    // burst deliberately exceeds the documented effect/particle budgets and
    // places a third of the effects far outside the culling distance, so the
    // caps and the distance culling are both exercised.
    if elapsed < bench.effects_seconds / 2.0 && now >= bench.next_action {
        let Some(position) = player_position(client) else {
            return;
        };

        let tick = bench.effects_spawned;
        bench.effects_spawned += 1;

        // Twelve effects around the player (the visible workload) ...
        for index in 0..12 {
            let angle = (tick * 5 + index) as f32 * 0.7;
            let radius = 40.0 + ((tick + index) % 4) as f32 * 25.0;
            let effect_position = position + Vector3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            let effect_path = BENCHMARK_EFFECTS[(tick as usize + index as usize) % BENCHMARK_EFFECTS.len()];

            client.spawn_position_effect(
                effect_path,
                effect_position,
                Vector3::new(0.0, 0.0, 0.0),
                1.0,
                Color::rgb_u8(200, 200, 30),
                20.0,
            );
        }

        // ... and six far away effects that the distance culling can drop
        // (well beyond the 1800 world unit cull distance).
        for index in 0..6 {
            let angle = (tick * 3 + index) as f32 * 1.3;
            let far_position = position + Vector3::new(angle.cos() * 2600.0, 0.0, angle.sin() * 2600.0);
            let effect_path = BENCHMARK_EFFECTS[index as usize % BENCHMARK_EFFECTS.len()];

            client.spawn_position_effect(
                effect_path,
                far_position,
                Vector3::new(0.0, 0.0, 0.0),
                1.0,
                Color::rgb_u8(200, 200, 30),
                20.0,
            );
        }

        // Thirty damage/heal/miss numbers per tick; a crowded town with an AoE
        // burst produces exactly this kind of particle spam (the particle
        // budget is 300, so this burst reaches it).
        for index in 0..30u32 {
            let offset = Vector3::new(
                ((tick + index) % 9) as f32 * 14.0 - 63.0,
                10.0 + (index % 3) as f32 * 8.0,
                ((tick * 7 + index) % 5) as f32 * 10.0 - 20.0,
            );

            match index % 4 {
                0 => client.particle_holder.spawn_particle(Box::new(DamageNumber::new(
                    position + offset,
                    100 + u64::from(tick) * 37 + u64::from(index),
                    index == 0,
                ))),
                1 => {
                    client
                        .particle_holder
                        .spawn_particle(Box::new(DamageNumber::new(position + offset, 10_000 + u64::from(index), false)))
                }
                2 => client
                    .particle_holder
                    .spawn_particle(Box::new(HealNumber::new(position + offset, 500 + u64::from(index)))),
                _ => client.particle_holder.spawn_particle(Box::new(Miss::new(position + offset))),
            }
        }

        bench.next_action = now + Duration::from_millis(100);
    }
}

fn stage_ui_updates(client: &mut Client, bench: &mut BenchState, now: Instant, client_tick: u64) {
    if !in_game(client) {
        bench.failure = Some("lost the map while updating the interface".to_owned());
        return;
    }

    if !bench.windows_opened {
        bench.windows_opened = true;

        for event in [
            InputEvent::ToggleInventoryWindow,
            InputEvent::ToggleEquipmentWindow,
            InputEvent::ToggleStatsWindow,
            InputEvent::ToggleSkillTreeWindow,
            InputEvent::ToggleMinimapWindow,
        ] {
            client.input_event_buffer.push(event);
        }
    }

    if !bench.buffs_applied {
        bench.buffs_applied = true;
        apply_benchmark_buffs(client, client_tick);
    }

    // A busy network frame invalidates the window layouts; reproducing that
    // every frame is the worst case for the buff bar labels and the layout
    // generation caches.
    client.interface.invalidate_layouts();

    if now.duration_since(bench.stage_started).as_secs_f64() >= bench.ui_seconds {
        finish_stage(bench, "ui_updates");
        begin_stage_with(bench, now, Stage::MapChange, "map_change");
        bench.map_pointer_before = map_pointer(client);
    }
}

fn stage_map_change(client: &mut Client, bench: &mut BenchState, _now: Instant) {
    const MAPS: [&str; 3] = ["izlude", "alberta", "prontera"];

    if let Some((map_name, requested_at)) = &bench.pending_map {
        let current_pointer = map_pointer(client);

        if current_pointer != bench.map_pointer_before {
            let milliseconds = requested_at.elapsed().as_secs_f64() * 1000.0;
            perf::emit(&format!(
                r#"[bench-map] {{"map":"{}","loaded":true,"ms":{milliseconds:.2}}}"#,
                escape(map_name)
            ));
            bench.map_samples.push((map_name.clone(), milliseconds));
            bench.pending_map = None;
            bench.map_pointer_before = current_pointer;
        } else if requested_at.elapsed() > Duration::from_secs(20) {
            perf::emit(&format!(
                r#"[bench-map] {{"map":"{}","loaded":false,"ms":20000.00}}"#,
                escape(map_name)
            ));
            bench.pending_map = None;
        } else {
            return;
        }
    }

    if bench.map_change_next < MAPS.len() {
        let map_name = MAPS[bench.map_change_next];
        bench.map_change_next += 1;
        bench.pending_map = Some((map_name.to_owned(), Instant::now()));
        bench.map_pointer_before = map_pointer(client);
        client.async_loader.request_map_load(map_name.to_owned(), None);
        return;
    }

    finish_stage(bench, "map_change");

    perf::emit(&format!(
        r#"[bench-end] {{"ok":true,"scenario":"{}","total_s":{:.2},"map_samples":{}}}"#,
        escape(&bench.scenario),
        bench.started.elapsed().as_secs_f64(),
        bench.map_samples.len(),
    ));

    bench.stage = Stage::Done;
    bench.finished = true;
    client.session.request_shutdown();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn in_game(client: &Client) -> bool {
    client.map.is_some() && client.client_state.try_follow(this_entity()).is_some()
}

fn player_tile(client: &Client) -> Option<TilePosition> {
    client
        .client_state
        .try_follow(this_entity())
        .map(|entity| entity.get_tile_position())
}

fn player_position(client: &Client) -> Option<cgmath::Point3<f32>> {
    client.client_state.try_follow(this_entity()).map(|entity| entity.get_position())
}

fn map_pointer(client: &Client) -> usize {
    client.map.as_ref().map(|map| std::sync::Arc::as_ptr(map) as usize).unwrap_or(0)
}

fn begin_stage_with(bench: &mut BenchState, now: Instant, stage: Stage, name: &'static str) {
    bench.stage = stage;
    bench.stage_started = now;
    bench.next_action = now;
    perf::begin_stage(name);
    // Drop loader statistics collected during the phase transitions so the
    // stage line only reflects the stage itself.
    let _ = perf::take_loader_statistics();
}

fn finish_stage(_bench: &mut BenchState, name: &'static str) {
    if let Some(summary) = perf::end_stage() {
        perf::emit_stage(&summary);
    }

    let loader = perf::take_loader_statistics();
    let kinds: Vec<String> = perf::load_kind_names()
        .iter()
        .zip(loader.iter())
        .map(|(name, statistics)| {
            format!(
                r#""{name}":{{"submitted":{},"completed":{},"failed":{},"queue_p95_ms":{:.2},"service_p95_ms":{:.2}}}"#,
                statistics.submitted,
                statistics.completed,
                statistics.failed,
                statistics.queue_p95_nanos as f64 / 1_000_000.0,
                statistics.service_p95_nanos as f64 / 1_000_000.0,
            )
        })
        .collect();

    perf::emit(&format!(r#"[bench-load] {{"stage":"{name}","kinds":{{{}}}}}"#, kinds.join(",")));
}

fn apply_statuses(client: &mut Client, client_tick: u64, statuses: &[u16]) {
    let Some(own_entity_id) = client.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id().0) else {
        return;
    };

    for (index, status_index) in statuses.iter().enumerate() {
        let definition = status_effect_definition(*status_index);
        let icon = definition
            .icon
            .and_then(|path| client.texture_loader.get_or_load(path, ImageType::Color).ok());
        let total_milliseconds = 120_000 + index as u32 * 15_000;

        client.client_state.follow_mut(client_state().status_effects()).apply(
            own_entity_id,
            *status_index,
            true,
            total_milliseconds,
            total_milliseconds,
            [0; 3],
            client_tick,
            definition.name.to_owned(),
            icon,
        );
    }

    // The benchmark injects the statuses directly into the status state (it
    // bypasses the network handler that normally refreshes the sprite), so
    // re-resolve the data-driven status visuals here to render the tints,
    // blink and additive blends during the sweep.
    client.refresh_entity_status_visual(ragnarok_packets::EntityId(own_entity_id));
}

fn apply_benchmark_buffs(client: &mut Client, client_tick: u64) {
    let Some(own_entity_id) = client.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id().0) else {
        return;
    };

    for (index, status_index) in BENCHMARK_BUFFS.iter().enumerate() {
        let definition = status_effect_definition(*status_index);
        let icon = definition
            .icon
            .and_then(|path| client.texture_loader.get_or_load(path, ImageType::Color).ok());

        // Stagger the durations so the remaining-time labels differ per buff.
        let total_milliseconds = 60_000 + index as u32 * 30_000;

        client.client_state.follow_mut(client_state().status_effects()).apply(
            own_entity_id,
            *status_index,
            true,
            total_milliseconds,
            total_milliseconds,
            [0; 3],
            client_tick,
            definition.name.to_owned(),
            icon,
        );
    }
}

/// Minimal JSON string escaping for the fixed benchmark labels (map names and
/// error reasons come from the environment, so they are sanitized).
fn escape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' | '\r' | '\t' => result.push(' '),
            character if character.is_control() => result.push(' '),
            character => result.push(character),
        }
    }

    result
}
