//! Lightweight, allocation-accounting performance instrumentation.
//!
//! This module backs the `KORANGAR_PERF_LOG` / `KORANGAR_BENCH` measurement
//! modes. It is intentionally dependency free and has a near zero cost when
//! disabled:
//!
//! * a global allocator wrapper counts allocations and allocated bytes per
//!   frame (only while [`enabled()`] is true, which is latched once at the
//!   first frame from the environment),
//! * frame begin/end timestamps and the existing frame phase durations are
//!   aggregated into named *stages* (login, town_idle, town_walk, effects,
//!   ui_updates, map_change, ...) so a benchmark run can report per-stage
//!   p50/p95 frame CPU time and allocation churn,
//! * async loader queue/service latencies are recorded per load kind so the
//!   loader QoS work (map priority lane) can be verified with numbers.
//!
//! The stage summaries are printed as machine readable `[bench-stage]` JSON
//! lines and, when a benchmark output path is configured, appended to that
//! file. `scripts/perf/perf-baseline.sh` parses those lines.
//!
//! Everything here is measurement only; no rendering or gameplay behavior is
//! changed by enabling it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Enable / disable
// ---------------------------------------------------------------------------

fn environment_set(name: &str) -> bool {
    std::env::var(name).map(|value| !value.is_empty() && value != "0").unwrap_or(false)
}

/// Latched enable state, shared by [`enabled()`] and the allocator.
///
/// The allocator only reads through [`OnceLock::get`], so it never initializes
/// the environment lookup from inside `alloc`.
static ENABLED: OnceLock<bool> = OnceLock::new();

/// Whether instrumentation is enabled for this process.
pub fn enabled() -> bool {
    *ENABLED.get_or_init(|| environment_set("KORANGAR_PERF_LOG") || environment_set("KORANGAR_BENCH"))
}

/// Whether instrumentation has been latched yet.
#[inline(always)]
fn enabled_latched() -> bool {
    matches!(ENABLED.get(), Some(true))
}

// ---------------------------------------------------------------------------
// Allocation accounting
// ---------------------------------------------------------------------------

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// Number of allocations made by the current thread.
///
/// The render loop runs on the main thread; measuring the *thread local* count
/// separates real per-frame client churn from allocations made concurrently by
/// the async loader workers (which are attributed to the frame but do not block
/// it).
#[thread_local]
static THREAD_ALLOCATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
#[thread_local]
static FRAME_THREAD_ALLOCATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };

fn count_allocations() {
    if enabled_latched() {
        THREAD_ALLOCATIONS.set(THREAD_ALLOCATIONS.get().wrapping_add(1));
    }
}

/// Snapshot of the process wide allocation counters.
#[derive(Debug, Clone, Copy, Default)]
struct AllocationCounters {
    allocations: u64,
    allocated_bytes: u64,
    deallocations: u64,
}

/// Number of allocations made by the current thread since startup.
pub fn thread_allocation_snapshot() -> u64 {
    THREAD_ALLOCATIONS.get()
}

fn allocation_counters() -> AllocationCounters {
    AllocationCounters {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        allocated_bytes: ALLOCATED_BYTES.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
    }
}

/// Global allocator wrapper that counts allocations while instrumentation is
/// enabled.
///
/// When disabled this only performs one relaxed atomic load and a branch per
/// allocation, which is negligible compared to the allocation itself.
pub struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the allocation request unchanged.
        let pointer = unsafe { System.alloc(layout) };

        if !pointer.is_null() && enabled_latched() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            count_allocations();
        }

        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarding the allocation request unchanged.
        let pointer = unsafe { System.alloc_zeroed(layout) };

        if !pointer.is_null() && enabled_latched() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
            count_allocations();
        }

        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if enabled_latched() {
            DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }

        // SAFETY: the pointer/layout pair came from an allocation by this allocator.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarding the reallocation request unchanged.
        let new_pointer = unsafe { System.realloc(pointer, layout, new_size) };

        if !new_pointer.is_null() && enabled_latched() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED_BYTES.fetch_add(new_size.saturating_sub(layout.size()) as u64, Ordering::Relaxed);
            count_allocations();
        }

        new_pointer
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

// ---------------------------------------------------------------------------
// Counters sampled once per frame
// ---------------------------------------------------------------------------

static CURRENT_EFFECTS: AtomicU32 = AtomicU32::new(0);
static CURRENT_PARTICLES: AtomicU32 = AtomicU32::new(0);
static CURRENT_ENTITIES: AtomicU32 = AtomicU32::new(0);
static CURRENT_WINDOWS: AtomicU32 = AtomicU32::new(0);
static RENDERED_EFFECTS: AtomicU32 = AtomicU32::new(0);
static CULLED_EFFECTS: AtomicU32 = AtomicU32::new(0);
static RENDERED_PARTICLES: AtomicU32 = AtomicU32::new(0);
static CULLED_PARTICLES: AtomicU32 = AtomicU32::new(0);

/// Sample the per-frame object counts (called once per rendered frame).
pub fn sample_counts(effects: u32, particles: u32, entities: u32) {
    if !enabled() {
        return;
    }

    CURRENT_EFFECTS.store(effects, Ordering::Relaxed);
    CURRENT_PARTICLES.store(particles, Ordering::Relaxed);
    CURRENT_ENTITIES.store(entities, Ordering::Relaxed);
}

/// Sample the number of open interface windows for the current frame.
pub fn sample_windows(windows: u32) {
    if !enabled() {
        return;
    }

    CURRENT_WINDOWS.store(windows, Ordering::Relaxed);
}

/// Names of all load kinds in [`LoadKind`] index order.
pub fn load_kind_names() -> [&'static str; LOAD_KIND_COUNT] {
    let mut names = ["unknown"; LOAD_KIND_COUNT];

    for kind in LoadKind::ALL {
        names[kind.index()] = kind.name();
    }

    names
}

// ---------------------------------------------------------------------------
// Async loader statistics
// ---------------------------------------------------------------------------

/// Number of distinct async load kinds tracked below.
pub const LOAD_KIND_COUNT: usize = 5;

/// Async load kind, used to attribute queue and service latencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    Map = 0,
    AnimationData = 1,
    ItemSprite = 2,
    SkillSprite = 3,
    SkillActions = 4,
}

impl LoadKind {
    pub const ALL: [LoadKind; LOAD_KIND_COUNT] = [
        LoadKind::Map,
        LoadKind::AnimationData,
        LoadKind::ItemSprite,
        LoadKind::SkillSprite,
        LoadKind::SkillActions,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            LoadKind::Map => "map",
            LoadKind::AnimationData => "animation",
            LoadKind::ItemSprite => "item_sprite",
            LoadKind::SkillSprite => "skill_sprite",
            LoadKind::SkillActions => "skill_actions",
        }
    }
}

/// Hard cap for the latency samples kept per load kind (approximate p95 is
/// fine for a regression gate; the oldest sample is dropped when full).
const MAX_LOAD_SAMPLES: usize = 2048;

#[derive(Default)]
struct LoadStats {
    submitted: u64,
    completed: u64,
    failed: u64,
    /// Requests that were answered by an already in-flight load of the same
    /// resource instead of starting a duplicate worker task.
    coalesced: u64,
    queue_samples: Vec<u64>,
    service_samples: Vec<u64>,
}

static LOAD_STATS: OnceLock<Mutex<[LoadStats; LOAD_KIND_COUNT]>> = OnceLock::new();

fn load_stats() -> &'static Mutex<[LoadStats; LOAD_KIND_COUNT]> {
    LOAD_STATS.get_or_init(|| {
        Mutex::new(std::array::from_fn(|_| LoadStats {
            queue_samples: Vec::with_capacity(256),
            service_samples: Vec::with_capacity(256),
            ..LoadStats::default()
        }))
    })
}

fn push_sample(samples: &mut Vec<u64>, value: u64) {
    if samples.len() >= MAX_LOAD_SAMPLES {
        samples.remove(0);
    }

    samples.push(value);
}

/// Record that an async load was submitted to a worker lane.
pub fn load_submitted(kind: LoadKind) {
    if !enabled() {
        return;
    }

    load_stats().lock().unwrap()[kind.index()].submitted += 1;
}

/// Record the queue latency (submit -> worker start) of an async load.
pub fn load_queue_latency(kind: LoadKind, nanos: u64) {
    if !enabled() {
        return;
    }

    let mut stats = load_stats().lock().unwrap();
    push_sample(&mut stats[kind.index()].queue_samples, nanos);
}

/// Record the service latency (worker start -> completion) of an async load.
pub fn load_service_latency(kind: LoadKind, nanos: u64, failed: bool) {
    if !enabled() {
        return;
    }

    let mut stats = load_stats().lock().unwrap();
    let stats = &mut stats[kind.index()];
    push_sample(&mut stats.service_samples, nanos);

    match failed {
        true => stats.failed += 1,
        false => stats.completed += 1,
    }
}

/// One load kind's statistics for the `[loader]` log line.
#[derive(Debug, Clone, Copy, Default)]
pub struct LoadKindStatistics {
    pub submitted: u64,
    pub completed: u64,
    pub failed: u64,
    pub coalesced: u64,
    pub queue_p50_nanos: u64,
    pub queue_p95_nanos: u64,
    pub service_p50_nanos: u64,
    pub service_p95_nanos: u64,
}

/// Take and reset the async loader statistics.
pub fn take_loader_statistics() -> [LoadKindStatistics; LOAD_KIND_COUNT] {
    let mut result = [LoadKindStatistics::default(); LOAD_KIND_COUNT];

    if !enabled() {
        return result;
    }

    let mut stats = load_stats().lock().unwrap();

    for (index, stats) in stats.iter_mut().enumerate() {
        let mut queue_samples = std::mem::take(&mut stats.queue_samples);
        let mut service_samples = std::mem::take(&mut stats.service_samples);

        if queue_samples.len() < 256 {
            queue_samples.reserve(256 - queue_samples.len());
        }
        if service_samples.len() < 256 {
            service_samples.reserve(256 - service_samples.len());
        }

        result[index] = LoadKindStatistics {
            submitted: std::mem::take(&mut stats.submitted),
            completed: std::mem::take(&mut stats.completed),
            failed: std::mem::take(&mut stats.failed),
            coalesced: std::mem::take(&mut stats.coalesced),
            queue_p50_nanos: percentile(&mut queue_samples, 50.0),
            queue_p95_nanos: percentile(&mut queue_samples, 95.0),
            service_p50_nanos: percentile(&mut service_samples, 50.0),
            service_p95_nanos: percentile(&mut service_samples, 95.0),
        };
    }

    result
}

fn percentile(values: &mut [u64], percentile: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }

    values.sort_unstable();
    let index = ((values.len() - 1) as f64 * percentile / 100.0).round() as usize;

    values[index.min(values.len() - 1)]
}

// ---------------------------------------------------------------------------
// Frame and stage accounting
// ---------------------------------------------------------------------------

thread_local! {
    /// Timestamp of the current frame's begin, on the render thread.
    static FRAME_START: Cell<Option<Instant>> = const { Cell::new(None) };
    /// Allocation counters sampled at the current frame's begin.
    static FRAME_ALLOCATIONS: Cell<AllocationCounters> = const { Cell::new(AllocationCounters {
        allocations: 0,
        allocated_bytes: 0,
        deallocations: 0,
    }) };
}

/// One per-frame sample aggregated into the current stage.
#[derive(Debug, Clone, Copy, Default)]
struct FrameSample {
    total_nanos: u64,
    wait_nanos: u64,
    sim_nanos: u64,
    interface_nanos: u64,
    world_nanos: u64,
    gpu_nanos: u64,
    allocations: u64,
    allocated_bytes: u64,
    deallocations: u64,
    thread_allocations: u64,
    pre_allocations: u64,
    sim_allocations: u64,
    interface_allocations: u64,
    world_allocations: u64,
    gpu_allocations: u64,
    effects: u32,
    particles: u32,
    entities: u32,
    windows: u32,
    effects_rendered: u32,
    effects_culled: u32,
    particles_rendered: u32,
    particles_culled: u32,
}

struct Stage {
    name: &'static str,
    start: Instant,
    samples: Vec<FrameSample>,
    /// Largest effect / particle / entity / window count seen during the stage.
    maximum_effects: u32,
    maximum_particles: u32,
    maximum_entities: u32,
    maximum_windows: u32,
}

impl Stage {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
            samples: Vec::with_capacity(4096),
            maximum_effects: 0,
            maximum_particles: 0,
            maximum_entities: 0,
            maximum_windows: 0,
        }
    }

    fn push(&mut self, sample: FrameSample) {
        self.maximum_effects = self.maximum_effects.max(sample.effects);
        self.maximum_particles = self.maximum_particles.max(sample.particles);
        self.maximum_entities = self.maximum_entities.max(sample.entities);
        self.maximum_windows = self.maximum_windows.max(sample.windows);

        // The stage ring is bounded; a stage longer than this loses the oldest
        // samples instead of growing without bound.
        if self.samples.len() >= 16384 {
            self.samples.remove(0);
        }

        self.samples.push(sample);
    }
}

static STAGE: OnceLock<Mutex<Option<Stage>>> = OnceLock::new();

fn stage_slot() -> &'static Mutex<Option<Stage>> {
    STAGE.get_or_init(|| Mutex::new(None))
}

/// Mark the beginning of a measured scenario stage.
pub fn begin_stage(name: &'static str) {
    *stage_slot().lock().unwrap() = Some(Stage::new(name));
    FRAME_START.with(|start| start.set(Some(Instant::now())));
}

/// Name of the currently measured stage, if any.
pub fn current_stage() -> Option<&'static str> {
    stage_slot().lock().unwrap().as_ref().map(|stage| stage.name)
}

/// Summary of one measured stage.
#[derive(Debug, Clone, Default)]
pub struct StageSummary {
    pub name: &'static str,
    pub frames: u64,
    pub duration_seconds: f64,
    pub total_p50_ms: f64,
    pub total_p95_ms: f64,
    pub total_max_ms: f64,
    pub total_mean_ms: f64,
    pub wait_mean_ms: f64,
    pub sim_mean_ms: f64,
    pub interface_mean_ms: f64,
    pub world_mean_ms: f64,
    pub gpu_mean_ms: f64,
    pub allocations_per_frame: f64,
    /// Allocations made by the render (main) thread only.
    pub main_allocations_per_frame: f64,
    /// Main thread allocations before the first measured frame phase
    /// (network/event/state/loader-result handling).
    pub pre_allocations_per_frame: f64,
    pub deallocations_per_frame: f64,
    pub sim_allocations_per_frame: f64,
    pub interface_allocations_per_frame: f64,
    pub world_allocations_per_frame: f64,
    pub gpu_allocations_per_frame: f64,
    pub allocated_kib_per_frame: f64,
    pub effects_mean: f64,
    pub particles_mean: f64,
    pub entities_mean: f64,
    pub windows_mean: f64,
    pub effects_rendered_mean: f64,
    pub effects_culled_mean: f64,
    pub particles_rendered_mean: f64,
    pub particles_culled_mean: f64,
    pub maximum_effects: u32,
    pub maximum_particles: u32,
    pub maximum_entities: u32,
    pub maximum_windows: u32,
}

impl StageSummary {
    /// The summary as a single line JSON object (no serde dependency).
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"stage":"{}","frames":{},"duration_s":{:.3},"frame_ms_mean":{:.3},"frame_ms_p50":{:.3},"frame_ms_p95":{:.3},"frame_ms_max":{:.3},"wait_ms":{:.3},"sim_ms":{:.3},"interface_ms":{:.3},"world_ms":{:.3},"gpu_ms":{:.3},"allocs_per_frame":{:.2},"main_allocs_per_frame":{:.2},"pre_allocs_per_frame":{:.2},"deallocs_per_frame":{:.2},"sim_allocs_per_frame":{:.2},"interface_allocs_per_frame":{:.2},"world_allocs_per_frame":{:.2},"gpu_allocs_per_frame":{:.2},"alloc_kib_per_frame":{:.2},"effects_mean":{:.1},"particles_mean":{:.1},"entities_mean":{:.1},"windows_mean":{:.1},"effects_rendered_mean":{:.1},"effects_culled_mean":{:.1},"particles_rendered_mean":{:.1},"particles_culled_mean":{:.1},"effects_max":{},"particles_max":{},"entities_max":{},"windows_max":{}}}"#,
            self.name,
            self.frames,
            self.duration_seconds,
            self.total_mean_ms,
            self.total_p50_ms,
            self.total_p95_ms,
            self.total_max_ms,
            self.wait_mean_ms,
            self.sim_mean_ms,
            self.interface_mean_ms,
            self.world_mean_ms,
            self.gpu_mean_ms,
            self.allocations_per_frame,
            self.main_allocations_per_frame,
            self.pre_allocations_per_frame,
            self.deallocations_per_frame,
            self.sim_allocations_per_frame,
            self.interface_allocations_per_frame,
            self.world_allocations_per_frame,
            self.gpu_allocations_per_frame,
            self.allocated_kib_per_frame,
            self.effects_mean,
            self.particles_mean,
            self.entities_mean,
            self.windows_mean,
            self.effects_rendered_mean,
            self.effects_culled_mean,
            self.particles_rendered_mean,
            self.particles_culled_mean,
            self.maximum_effects,
            self.maximum_particles,
            self.maximum_entities,
            self.maximum_windows,
        )
    }
}

/// Finish the current stage and return its summary.
pub fn end_stage() -> Option<StageSummary> {
    let stage = stage_slot().lock().unwrap().take()?;
    let duration = stage.start.elapsed().as_secs_f64();

    let frames = stage.samples.len() as u64;
    if frames == 0 {
        return Some(StageSummary {
            name: stage.name,
            duration_seconds: duration,
            ..StageSummary::default()
        });
    }

    let divisor = frames as f64;

    // Mean of one field in milliseconds (per frame).
    let mean_ms = |field: fn(&FrameSample) -> u64| -> f64 {
        stage.samples.iter().map(field).sum::<u64>() as f64 / divisor / 1_000_000.0
    };
    let mean_count = |field: fn(&FrameSample) -> u32| -> f64 {
        stage.samples.iter().map(|sample| field(sample) as u64).sum::<u64>() as f64 / divisor
    };

    let mut totals: Vec<u64> = stage.samples.iter().map(|sample| sample.total_nanos).collect();
    let total_p50 = percentile(&mut totals, 50.0);
    let total_p95 = percentile(&mut totals, 95.0);
    let total_max = totals.iter().copied().max().unwrap_or(0);

    let summary = StageSummary {
        name: stage.name,
        frames,
        duration_seconds: duration,
        total_p50_ms: total_p50 as f64 / 1_000_000.0,
        total_p95_ms: total_p95 as f64 / 1_000_000.0,
        total_max_ms: total_max as f64 / 1_000_000.0,
        total_mean_ms: mean_ms(|sample| sample.total_nanos),
        wait_mean_ms: mean_ms(|sample| sample.wait_nanos),
        sim_mean_ms: mean_ms(|sample| sample.sim_nanos),
        interface_mean_ms: mean_ms(|sample| sample.interface_nanos),
        world_mean_ms: mean_ms(|sample| sample.world_nanos),
        gpu_mean_ms: mean_ms(|sample| sample.gpu_nanos),
        allocations_per_frame: stage.samples.iter().map(|sample| sample.allocations as f64).sum::<f64>() / divisor,
        main_allocations_per_frame: stage.samples.iter().map(|sample| sample.thread_allocations as f64).sum::<f64>() / divisor,
        pre_allocations_per_frame: stage.samples.iter().map(|sample| sample.pre_allocations as f64).sum::<f64>() / divisor,
        deallocations_per_frame: stage.samples.iter().map(|sample| sample.deallocations as f64).sum::<f64>() / divisor,
        sim_allocations_per_frame: stage.samples.iter().map(|sample| sample.sim_allocations as f64).sum::<f64>() / divisor,
        interface_allocations_per_frame: stage.samples.iter().map(|sample| sample.interface_allocations as f64).sum::<f64>() / divisor,
        world_allocations_per_frame: stage.samples.iter().map(|sample| sample.world_allocations as f64).sum::<f64>() / divisor,
        gpu_allocations_per_frame: stage.samples.iter().map(|sample| sample.gpu_allocations as f64).sum::<f64>() / divisor,
        allocated_kib_per_frame: stage.samples.iter().map(|sample| sample.allocated_bytes as f64 / 1024.0).sum::<f64>() / divisor,
        effects_mean: mean_count(|sample| sample.effects),
        particles_mean: mean_count(|sample| sample.particles),
        entities_mean: mean_count(|sample| sample.entities),
        windows_mean: mean_count(|sample| sample.windows),
        effects_rendered_mean: mean_count(|sample| sample.effects_rendered),
        effects_culled_mean: mean_count(|sample| sample.effects_culled),
        particles_rendered_mean: mean_count(|sample| sample.particles_rendered),
        particles_culled_mean: mean_count(|sample| sample.particles_culled),
        maximum_effects: stage.maximum_effects,
        maximum_particles: stage.maximum_particles,
        maximum_entities: stage.maximum_entities,
        maximum_windows: stage.maximum_windows,
    };

    Some(summary)
}

/// Begin a frame on the render thread (cheap; a thread local timestamp).
pub fn frame_begin() {
    if !enabled() {
        return;
    }

    FRAME_START.with(|start| start.set(Some(Instant::now())));
    FRAME_ALLOCATIONS.with(|counters| counters.set(allocation_counters()));
    FRAME_THREAD_ALLOCATIONS.set(THREAD_ALLOCATIONS.get());
}

/// Record one rendered frame with the frame phase durations measured in the
/// render loop. Called from `record_frame_phase`.
pub fn record_frame(
    wait_nanos: u64,
    sim_nanos: u64,
    interface_nanos: u64,
    world_nanos: u64,
    gpu_nanos: u64,
    phase_allocations: [u64; 4],
) {
    if !enabled() {
        return;
    }

    let Some(start) = FRAME_START.with(|start| start.replace(None)) else {
        return;
    };

    FRAME_START.with(|start| start.set(Some(Instant::now())));

    let before = FRAME_ALLOCATIONS.with(|counters| counters.get());
    let after = allocation_counters();
    let thread_allocations = THREAD_ALLOCATIONS.get().saturating_sub(FRAME_THREAD_ALLOCATIONS.get());
    let phase_sum: u64 = phase_allocations.iter().sum();
    let pre_allocations = thread_allocations.saturating_sub(phase_sum);

    let sample = FrameSample {
        total_nanos: start.elapsed().as_nanos() as u64,
        sim_allocations: phase_allocations[0],
        interface_allocations: phase_allocations[1],
        world_allocations: phase_allocations[2],
        gpu_allocations: phase_allocations[3],
        wait_nanos,
        sim_nanos,
        interface_nanos,
        world_nanos,
        gpu_nanos,
        allocations: after.allocations.saturating_sub(before.allocations),
        allocated_bytes: after.allocated_bytes.saturating_sub(before.allocated_bytes),
        deallocations: after.deallocations.saturating_sub(before.deallocations),
        thread_allocations,
        pre_allocations,
        effects: CURRENT_EFFECTS.load(Ordering::Relaxed),
        particles: CURRENT_PARTICLES.load(Ordering::Relaxed),
        entities: CURRENT_ENTITIES.load(Ordering::Relaxed),
        windows: CURRENT_WINDOWS.load(Ordering::Relaxed),
        effects_rendered: RENDERED_EFFECTS.load(Ordering::Relaxed),
        effects_culled: CULLED_EFFECTS.load(Ordering::Relaxed),
        particles_rendered: RENDERED_PARTICLES.load(Ordering::Relaxed),
        particles_culled: CULLED_PARTICLES.load(Ordering::Relaxed),
    };

    if let Some(stage) = stage_slot().lock().unwrap().as_mut() {
        stage.push(sample);
    }
}

// ---------------------------------------------------------------------------
// Benchmark output
// ---------------------------------------------------------------------------

static OUTPUT_PATH: OnceLock<Option<String>> = OnceLock::new();
static WROTE_HEADER: AtomicBool = AtomicBool::new(false);

fn output_path() -> Option<&'static str> {
    OUTPUT_PATH
        .get_or_init(|| std::env::var("KORANGAR_BENCH_OUT").ok().filter(|path| !path.trim().is_empty()))
        .as_deref()
}

/// Append one line to the benchmark output file (if configured).
///
/// Also mirrors the line to stdout so it is always visible in the client log.
pub fn emit(line: &str) {
    println!("{line}");

    let Some(path) = output_path() else {
        return;
    };

    let needs_header = !WROTE_HEADER.swap(true, Ordering::Relaxed);
    let mut file = match std::fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => file,
        Err(_) => return,
    };

    use std::io::Write;

    if needs_header {
        let _ = writeln!(file, "# korangar benchmark output; one JSON object per line");
    }
    let _ = writeln!(file, "{line}");
}

/// Emit `[bench-stage]` JSON for a finished stage.
pub fn emit_stage(summary: &StageSummary) {
    emit(&format!("[bench-stage] {}", summary.to_json()));
}

/// Emit the `[loader]` interval line for the KORANGAR_PERF_LOG path.
pub fn loader_log_line() -> Option<String> {
    if !enabled() {
        return None;
    }

    let statistics = take_loader_statistics();
    let mut total_submitted = 0;
    let mut total_completed = 0;
    let mut total_failed = 0;
    let mut total_coalesced = 0;

    for statistics in &statistics {
        total_submitted += statistics.submitted;
        total_completed += statistics.completed;
        total_failed += statistics.failed;
        total_coalesced += statistics.coalesced;
    }

    let map = statistics[LoadKind::Map.index()];
    let animation = statistics[LoadKind::AnimationData.index()];
    let sprite = statistics[LoadKind::SkillSprite.index()];
    let item = statistics[LoadKind::ItemSprite.index()];

    let milliseconds = |nanos: u64| nanos as f64 / 1_000_000.0;

    Some(format!(
        "[loader] submitted={total_submitted} completed={total_completed} failed={total_failed} coalesced={total_coalesced} \
         map_q_p50={:.2}ms map_q_p95={:.2}ms map_svc_p50={:.2}ms map_svc_p95={:.2}ms anim_q_p95={:.2}ms anim_svc_p95={:.2}ms          sprite_svc_p95={:.2}ms item_svc_p95={:.2}ms",
        milliseconds(map.queue_p50_nanos),
        milliseconds(map.queue_p95_nanos),
        milliseconds(map.service_p50_nanos),
        milliseconds(map.service_p95_nanos),
        milliseconds(animation.queue_p95_nanos),
        milliseconds(animation.service_p95_nanos),
        milliseconds(sprite.service_p95_nanos),
        milliseconds(item.service_p95_nanos),
    ))
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{LoadKind, LoadKindStatistics, LOAD_KIND_COUNT, StageSummary, percentile};

    #[test]
    fn percentiles_are_sane() {
        let mut values = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&mut values, 50.0), 5);
        assert_eq!(percentile(&mut values, 95.0), 10);
        assert_eq!(percentile(&mut values, 100.0), 10);
        assert_eq!(percentile(&mut [], 50.0), 0);
    }

    #[test]
    fn load_kinds_have_unique_indices() {
        let mut indices: Vec<usize> = LoadKind::ALL.iter().map(|kind| kind.index()).collect();
        indices.sort_unstable();
        indices.dedup();
        assert_eq!(indices.len(), LOAD_KIND_COUNT);
        assert_eq!(LoadKind::ALL.len(), LOAD_KIND_COUNT);
    }

    #[test]
    fn stage_summary_json_contains_required_fields() {
        let summary = StageSummary {
            name: "unit_test",
            frames: 10,
            duration_seconds: 0.5,
            allocations_per_frame: 12.5,
            ..StageSummary::default()
        };
        let json = summary.to_json();

        for field in [
            "\"stage\":\"unit_test\"",
            "\"frames\":10",
            "\"allocs_per_frame\":12.50",
            "\"frame_ms_p95\"",
            "\"interface_ms\"",
        ] {
            assert!(json.contains(field), "missing {field} in {json}");
        }
    }

    #[test]
    fn default_load_statistics_are_zero() {
        let statistics = [LoadKindStatistics::default(); LOAD_KIND_COUNT];
        assert!(statistics.iter().all(|statistics| statistics.submitted == 0 && statistics.service_p95_nanos == 0));
    }
}
