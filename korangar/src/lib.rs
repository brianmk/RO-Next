#![allow(incomplete_features)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(random)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]
#![feature(anonymous_lifetime_in_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(unsafe_cell_access)]
#![feature(impl_trait_in_assoc_type)]
#![feature(thread_local)]

// Helper macro to time and print the startup time of Korangar
mod msgstringtable;

macro_rules! time_phase {
    ($message:expr, { $($statements:tt)* }) => {
        #[cfg(feature = "debug")]
        let _statement_timer = korangar_debug::logging::Timer::new($message);

        $($statements)*

        #[cfg(feature = "debug")]
        _statement_timer.stop();
    }
}

mod benchmark;
mod graphics;
mod input;
mod perf;
mod state;
#[macro_use]
mod interface;
pub mod crash;
mod loaders;
pub mod logging;
mod login_server;
#[cfg(feature = "debug")]
mod networking;
mod renderer;
mod settings;
mod system;
mod world;

use std::collections::HashSet;
use std::io::Cursor;
use std::net::SocketAddr;
use std::ops::Deref;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cgmath::{Point3, Vector3};
use hashbrown::HashMap;
use image::{EncodableLayout, ImageFormat, ImageReader};
use input::{MouseInputMode, MouseModeExt};
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_interface::application::Clip;
use korangar_interface::layout::MouseButton;
use korangar_interface::{Interface, MouseMode, take_layout_profile};
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SellItem,
    SupportedPacketVersion,
};
#[cfg(feature = "debug")]
use networking::{PacketHistory, PacketHistoryCallback};
use ragnarok_packets::{
    AmmunitionActionType, AttackRange, BuyItemResult, CharacterServerInformation, ClientTick, Direction, DisappearanceReason, EntityId,
    ExperienceSource, ExperienceType, Friend, HotbarSlot, HotkeyType, InventoryIndex, ItemId, JobId, MarkerType, SellItemsResult, SkillId,
    SkillLevel, SkillType, StatType, TilePosition, UnitId, WorldPosition,
};
use renderer::InterfaceRenderer;
use rust_state::{ManuallyAssertExt, State};
#[cfg(feature = "debug")]
use rust_state::{VecIndexExt, VecLookupExt};
use settings::{
    AudioSettings, AudioSettingsPathExt, AuraStyle, GraphicsSettingsCapabilities, GraphicsSettingsPathExt, InterfaceSettings,
    InterfaceSettingsPathExt,
};
use state::hotbar::{HOTBAR_ENTRIES_PER_TAB, HOTBAR_TAB_COUNT, HotbarPathExt};
use state::inventory::InventoryPathExt;
use state::lifecycle::{
    AddEntityDecision, EntityLifecycle, RemoveEntityOutcome, ResourcePolicy, ResourceRetryQueue, SHUTDOWN_DISCONNECT_TIMEOUT,
    SessionAction, SessionEvent, SessionMachine, SessionObservation, SessionState,
};
use state::localization::Localization;
use state::skills::SkillTreePathExt;
use state::theme::{CursorThemePathExt, IndicatorThemePathExt, InterfaceThemePathExt, WorldThemePathExt};
use state::{ChatMessage, ClientState, ClientStatePathExt, client_state, status_effect_definition, this_entity, this_player};
#[cfg(feature = "debug")]
use wgpu::Device;
use wgpu::util::initialize_adapter_from_env_or_default;
use wgpu::wgt::{Dx12SwapchainKind, Dx12UseFrameLatencyWaitableObject};
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, Dx12BackendOptions, Dx12Compiler, ExperimentalFeatures, ForceShaderModelToken,
    GlBackendOptions, GlDebugFns, GlFenceBehavior, Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, MemoryBudgetThresholds,
    MemoryHints, NoopBackendOptions, Queue, Trace,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize, Size};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Icon, Window, WindowId};

use crate::graphics::*;
use crate::input::{InputEvent, InputReport, InputSystem, translate_slash_command};
use crate::interface::cursor::{MouseCursor, MouseCursorState};
use crate::interface::resource::{ItemSource, SkillSource};
use crate::interface::windows::*;
use crate::loaders::error::LoadError;
use crate::loaders::{
    AssetFailureSeverity, TransientAssetNoticeLimiter, asset_failure_log_key, asset_failure_severity, transient_asset_failure_message, *,
};
#[cfg(feature = "debug")]
use crate::renderer::DebugMarkerRenderer;
use crate::renderer::{AlignHorizontal, EffectRenderer, GameInterfaceRenderer, SpriteRenderer};
use crate::settings::{
    AuraStyleMarker, FORCE_UNFOCUSED_ALIAS_ENVIRONMENT_VARIABLE, FORCE_UNFOCUSED_ENVIRONMENT_VARIABLE, FRAMERATE_LOG_ENVIRONMENT_VARIABLE,
    FRAMERATE_LOG_INTERVAL_SECONDS, GameSettingsPathExt, GraphicsSettings, IN_GAME_THEMES_PATH, LightingMode, MENU_THEMES_PATH,
    ServiceSettingsPathExt, WORLD_THEMES_PATH, parse_aura_style_marker,
};
use crate::state::inventory::{ItemCategory, ItemUseAction, item_category, item_use_action};
use crate::state::localization::LocalizationPathExt;
use crate::state::minimap::{MapWindowCycle, direction_index_from_tile_delta};
use crate::state::party::PartyPathExt;
use crate::state::skills::{LearnedSkill, SkillTreeLayoutPathExt, bring_skill_to_level};
use crate::state::stat_buffer::PendingStatUpdates;
use crate::state::theme::{ColorTokensPathExt, DesignTokensPathExt, InterfaceTheme, InterfaceThemeType, WorldTheme};
use crate::state::{BufferedAction, SelectedServicePath, client_theme};
use crate::system::{FrameTimers, GameTimer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::*;

/// Next unique point-light id for position based effects. The point light
/// manager uses the id to decide which light renders with a shadow, so it has
/// to be unique.
static NEXT_POSITION_EFFECT_LIGHT_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0x5000_0000);

/// Packet callback used in release builds.
///
/// Unknown/unparseable 2019 server packets are logged at debug verbosity
/// (`-vv` / `KORANGAR_LOG_LEVEL=2`) so that missing handlers can be spotted
/// without the debug packet inspector.
#[cfg(not(feature = "debug"))]
#[derive(Clone, Copy, Default)]
struct ClientPacketCallback;

#[cfg(not(feature = "debug"))]
impl ragnarok_packets::handler::PacketCallback for ClientPacketCallback {
    fn unknown_packet(&self, bytes: Vec<u8>) {
        let header = bytes
            .get(..2)
            .map(|header| u16::from_le_bytes([header[0], header[1]]))
            .map(|header| format!("0x{header:04X}"))
            .unwrap_or_else(|| "unknown".to_owned());

        crate::logging::log_debug!(
            "packets: unhandled server packet header {} ({} bytes remaining)",
            header,
            bytes.len()
        );
    }

    fn failed_packet(&self, bytes: Vec<u8>, error: Box<ragnarok_bytes::ConversionError>) {
        crate::logging::log_warn!("packets: failed to parse server packet ({} bytes): {:?}", bytes.len(), error);
    }
}

const CLIENT_NAME: &str = "RO-Next";
const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
/// Minimum duration of the skill-cast animation for casts without a usable cast
/// time, so that they are still visible.
/// Duration of the one-shot attack/pickup animation for instant skills, where
/// the server sends no attack motion (the official client uses a 400 ms hold).
const INSTANT_SKILL_ANIMATION_MS: u32 = 400;
/// Window in which the completion packet of instant skills is treated as the
/// duplicate of the start-of-use sound (milliseconds).
const CAST_SOUND_DEDUPE_MS: u32 = 1_000;
/// Stale start-of-use sound entries are dropped after this age (milliseconds).
const CAST_SOUND_DEDUPE_TIMEOUT_MS: u32 = 5_000;
/// Skill id of the Sage/Scholar/Professor `Free Cast` passive (`SA_FREECAST`),
/// which allows movement while casting.
const FREE_CAST_SKILL_ID: SkillId = SkillId(278);
/// Job ids that can learn `Free Cast`: Sage and its rebirth class (called
/// Scholar or Professor depending on the game version).
const FREE_CAST_JOB_IDS: [u16; 2] = [16, 4017];
/// Completion-effect de-duplication phase used by the hand-written (special
/// cased) completion visuals in [`skill_visuals`].
const CUSTOM_COMPLETION_PHASE: u8 = u8::MAX;
/// De-duplication phase of the procedural Storm Gust cloud.
const STORM_CLOUD_PHASE: u8 = u8::MAX - 1;
const DEFAULT_MAP: &str = "geffen";
const START_CAMERA_FOCUS_POINT: Point3<f32> = Point3::new(600.0, 0.0, 240.0);
/// Background music that is played while the client is *not* inside a map:
/// the login window, the server selection and the character selection.
///
/// Classic Ragnarok Online hard-codes `BGM/01.mp3` ("Title") for the login
/// screen and also uses it as the fallback track for maps without their own
/// music. The file name resolution (archives, `data/mp3`, case-insensitive
/// loose files) is shared with the map music. If a future client needs a
/// separate character selection track, this is the only place to change.
const UI_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3");
const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "버튼소리.wav";
/// Sound played when another entity teleports away / logs out (classic client
/// plays the teleport warp effect together with this effect sound).
const TELEPORT_VANISH_SOUND_EFFECT: &str = "effect\\ef_teleportation.wav";
/// Classic item view ids of bows (item view ids 1700-1799).
const BOW_ITEM_VIEW_RANGE: std::ops::RangeInclusive<u32> = 1700..=1799;
/// Server weapon type `W_BOW` (rAthena `e_weapon_type`).
const WEAPON_TYPE_BOW: u32 = 11;

/// Whether the weapon value of an entity is a bow (server weapon type or item
/// view id).
fn is_bow_weapon(weapon: u32) -> bool {
    weapon == WEAPON_TYPE_BOW || BOW_ITEM_VIEW_RANGE.contains(&weapon)
}
/// Maximum number of teleport-vanish effects spawned within
/// `TELEPORT_VANISH_WINDOW`. Population-engine shells can vanish in bursts;
/// this keeps the effect renderer cheap.
const TELEPORT_VANISH_BURST_LIMIT: usize = 12;
/// Sliding window used for [`TELEPORT_VANISH_BURST_LIMIT`].
const TELEPORT_VANISH_WINDOW: Duration = Duration::from_millis(700);
const ITEM_PICKUP_RANGE: AttackRange = AttackRange(1);
// TODO: The number of point lights that can cast shadows should be configurable
// through the graphics settings. For now I just chose an arbitrary smaller
// number that should be playable on most devices.
const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;

const INITIAL_SCREEN_SIZE: ScreenSize = ScreenSize {
    width: 1280.0,
    height: 720.0,
};

/// Classic RO name label colours per entity type.
///
/// Players are white, NPCs are blue. Monsters only get a label while hovered;
/// the 2019 spawn packets don't expose the "aggressive" flag, so the classic
/// aggressive red is used for them.
fn entity_name_color(entity_type: EntityType) -> Color {
    match entity_type {
        EntityType::Player => Color::WHITE,
        EntityType::Npc => Color::rgb_u8(0x5B, 0xA8, 0xFF),
        EntityType::Monster => Color::rgb_u8(0xFF, 0x7A, 0x7A),
        EntityType::Warp | EntityType::Hidden => Color::WHITE,
    }
}

/// Base font size of the character name label, in interface pixels.
const NAME_FONT_SIZE: f32 = 16.0;
/// Base font size of the guild name line below the character name.
const GUILD_NAME_FONT_SIZE: f32 = 12.0;
/// Vertical gap between the character name and the guild name line, in
/// interface pixels (scaled with the interface scaling factor when rendering).
const GUILD_NAME_GAP: f32 = 2.0;
/// Dimmed blue-gray color of the guild name line, so that it is readable but
/// does not compete with the character name itself.
const GUILD_NAME_COLOR: Color = Color::rgb_u8(0x9D, 0xB4, 0xC8);
/// Size of the guild emblem icon next to the guild name, in UI pixels.
const GUILD_EMBLEM_ICON_SIZE: f32 = 14.0;
/// Neutral placeholder color for guild emblems that are not loaded yet.
const GUILD_EMBLEM_PLACEHOLDER_COLOR: Color = Color::rgba_u8(0x78, 0x78, 0x78, 0xB4);

const INITIAL_SCALING_FACTOR: Scaling = Scaling::new(1.0);
const FALLBACK_PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;

static ICON_DATA: &[u8] = include_bytes!("../archive/data/icon.png");

/// Which window backend the client should use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendPreference {
    /// Prefer native Wayland when the session provides it, otherwise use X11.
    Auto,
    /// Force the native Wayland backend, fail if it is not usable.
    Wayland,
    /// Force the X11 backend (native X11 or XWayland), fail if it is not
    /// usable.
    X11,
}

impl BackendPreference {
    fn from_environment() -> Self {
        let enabled = |name: &str| {
            std::env::var(name).is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        };

        match (enabled("KORANGAR_X11"), enabled("KORANGAR_WAYLAND")) {
            (true, true) => {
                crate::logging::log_warn!("[display] both KORANGAR_X11 and KORANGAR_WAYLAND are set, using X11");
                Self::X11
            }
            (true, false) => Self::X11,
            (false, true) => Self::Wayland,
            (false, false) => Self::Auto,
        }
    }
}

/// Returns whether a Wayland compositor is reachable through the environment.
///
/// `WAYLAND_DISPLAY` is either an absolute path to the socket or a socket name
/// inside `XDG_RUNTIME_DIR`; `WAYLAND_SOCKET` (a file descriptor passed by the
/// parent process) also counts. A stale `WAYLAND_DISPLAY` without a socket file
/// is treated as unavailable, so the client can fall back to X11 instead of
/// failing to connect.
fn wayland_session_available() -> bool {
    if let Some(wayland_display) = std::env::var_os("WAYLAND_DISPLAY").filter(|value| !value.is_empty()) {
        let wayland_display = Path::new(&wayland_display);

        if wayland_display.is_absolute() {
            return wayland_display.exists();
        }

        return std::env::var_os("XDG_RUNTIME_DIR")
            .filter(|runtime_dir| !runtime_dir.is_empty())
            .map(|runtime_dir| Path::new(&runtime_dir).join(wayland_display).exists())
            .unwrap_or(false);
    }

    std::env::var_os("WAYLAND_SOCKET").is_some_and(|value| !value.is_empty())
}

fn x11_session_available() -> bool {
    std::env::var_os("DISPLAY").is_some_and(|value| !value.is_empty())
}

#[cfg(target_os = "linux")]
fn create_wayland_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    use winit::platform::wayland::EventLoopBuilderExtWayland;

    let mut builder = EventLoop::builder();
    builder.with_wayland();
    builder.build()
}

#[cfg(target_os = "linux")]
fn create_x11_event_loop() -> Result<EventLoop<()>, winit::error::EventLoopError> {
    use winit::platform::x11::EventLoopBuilderExtX11;

    let mut builder = EventLoop::builder();
    builder.with_x11();
    builder.build()
}

/// Creates the event loop and selects the window backend.
///
/// Native Wayland is preferred when the session provides a usable compositor.
/// If connecting to it fails (compositor gone, invalid `WAYLAND_DISPLAY`,
/// stale socket, ...) the client automatically retries with X11 when `DISPLAY`
/// is set, instead of panicking like the previous unconditional
/// `EventLoop::new().unwrap()`. Force a backend with `KORANGAR_X11=1` or
/// `KORANGAR_WAYLAND=1`.
#[cfg(target_os = "linux")]
fn create_event_loop() -> Result<EventLoop<()>, String> {
    let preference = BackendPreference::from_environment();

    match preference {
        BackendPreference::X11 => {
            let event_loop = create_x11_event_loop().map_err(|error| format!("failed to create the X11 event loop: {error}"))?;
            eprintln!("[display] using X11 backend (forced by KORANGAR_X11)");
            Ok(event_loop)
        }
        BackendPreference::Wayland => {
            let event_loop = create_wayland_event_loop().map_err(|error| format!("failed to create the Wayland event loop: {error}"))?;
            eprintln!("[display] using native Wayland backend (forced by KORANGAR_WAYLAND)");
            Ok(event_loop)
        }
        BackendPreference::Auto => {
            if wayland_session_available() {
                match create_wayland_event_loop() {
                    Ok(event_loop) => {
                        eprintln!("[display] using native Wayland backend");
                        return Ok(event_loop);
                    }
                    Err(error) => {
                        crate::logging::log_warn!("[display] could not connect to the Wayland compositor: {error}");
                        if x11_session_available() {
                            crate::logging::log_warn!("[display] falling back to the X11 backend");
                        }
                    }
                }
            }

            if x11_session_available() {
                match create_x11_event_loop() {
                    Ok(event_loop) => {
                        eprintln!("[display] using X11 backend (native X11 or XWayland)");
                        return Ok(event_loop);
                    }
                    Err(error) => {
                        return Err(format!(
                            "failed to create the X11 event loop: {error}\nhint: make sure DISPLAY points to a running X server or \
                             XWayland"
                        ));
                    }
                }
            }

            Err(
                "no usable display server found: neither WAYLAND_DISPLAY/WAYLAND_SOCKET (with a live compositor) nor DISPLAY is set"
                    .to_string(),
            )
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn create_event_loop() -> Result<EventLoop<()>, String> {
    EventLoop::new().map_err(|error| format!("failed to create the event loop: {error}"))
}

/// CTR+C (SIGINT), SIGHUP or SIGTERM was received, and the client is supposed
/// to close.
pub static SHUTDOWN_SIGNAL: AtomicBool = AtomicBool::new(false);

/// Signal handler that only requests a cooperative shutdown. It must not touch
/// anything except atomics, since it runs in signal context.
#[cfg(unix)]
unsafe extern "C" fn shutdown_signal_handler(_signal: libc::c_int) {
    SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
}

#[cfg(feature = "debug")]
const DEBUG_WINDOWS: &[WindowClass] = &[
    WindowClass::CacheStatistics,
    WindowClass::ClientStateInspector,
    WindowClass::PacketInspector,
    WindowClass::Profiler,
    WindowClass::RenderOptions,
];

// Create the `threads` module.
#[cfg(feature = "debug")]
korangar_debug::create_profiler_threads!(threads, {
    Main,
    Loader,
});

pub fn init_tls_rand() {
    use std::random::*;
    let mut seed = [0; 32];
    DefaultRandomSource.fill_bytes(&mut seed);
    rand_aes::tls::rand_seed(seed.into());
}

fn initialize_shutdown_signal() {
    ctrlc::set_handler(|| {
        println!("CTRL-C received. Shutting down");
        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    // Ctrl-C is not the only way the process can be asked to close: closing the
    // terminal sends SIGHUP, `kill`/`systemctl stop` sends SIGTERM. Without
    // handling these the process is terminated without running destructors, so
    // settings (e.g. remembered logins) are never written. Route both onto the
    // cooperative shutdown path so `Drop` handlers run.
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGHUP, shutdown_signal_handler as *const () as usize);
        libc::signal(libc::SIGTERM, shutdown_signal_handler as *const () as usize);
    }
}

/// Time without a new experience packet after which the aggregated gain is
/// shown in the chat.
const EXPERIENCE_FEEDBACK_DELAY: Duration = Duration::from_millis(1500);

/// Experience amounts that are aggregated into one feedback line.
#[derive(Default)]
struct PendingExperience {
    base: i64,
    job: i64,
    quest_base: i64,
    quest_job: i64,
    last_packet: Option<Instant>,
}

impl PendingExperience {
    fn add(&mut self, amount: i64, experience_type: ExperienceType, experience_source: ExperienceSource) {
        match (experience_type, experience_source) {
            (ExperienceType::BaseExperience, ExperienceSource::Regular) => self.base += amount,
            (ExperienceType::BaseExperience, ExperienceSource::Quest) => self.quest_base += amount,
            (ExperienceType::JobExperience, ExperienceSource::Regular) => self.job += amount,
            (ExperienceType::JobExperience, ExperienceSource::Quest) => self.quest_job += amount,
        }

        self.last_packet = Some(Instant::now());
    }

    fn is_empty(&self) -> bool {
        self.base == 0 && self.job == 0 && self.quest_base == 0 && self.quest_job == 0
    }

    /// Formats and clears the aggregated amounts. Returns [`None`] when nothing
    /// was gained.
    fn take_message(&mut self) -> Option<String> {
        if self.is_empty() {
            self.last_packet = None;
            return None;
        }

        let mut parts = Vec::new();
        let mut push = |amount: i64, label: &str| {
            if amount != 0 {
                parts.push(format!(
                    "{}{} {}",
                    if amount < 0 { "-" } else { "" },
                    format_amount(amount.unsigned_abs()),
                    label
                ));
            }
        };

        push(self.base, "base EXP");
        push(self.job, "job EXP");
        push(self.quest_base, "base EXP (quest)");
        push(self.quest_job, "job EXP (quest)");

        let message = match parts.is_empty() {
            true => None,
            false => Some(format!("Gained {}.", parts.join(", "))),
        };

        self.base = 0;
        self.job = 0;
        self.quest_base = 0;
        self.quest_job = 0;
        self.last_packet = None;

        message
    }
}

/// Formats a number with thousands separators, e.g. `12,345`.
fn format_amount(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);

    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }

    formatted
}

/// Loads the client message table and logs whether it was found.
///
/// `data/msgstringtable.txt` is part of the client archives; when it is missing
/// the client falls back to raw message ids instead of hiding the messages.
fn load_message_table(game_file_loader: &GameFileLoader) -> Option<msgstringtable::MessageTable> {
    let message_table = msgstringtable::MessageTable::load(game_file_loader);

    match &message_table {
        Some(table) => crate::logging::log_debug!("msgstringtable: {} entries loaded", table.len()),
        None => crate::logging::log_warn!("data/msgstringtable.txt not found in the archives; system messages fall back to raw ids"),
    }

    message_table
}

/// Resolves the world anchor of a procedural effect row.
///
/// Returns `None` for the moving primitives (`Travel`/`Rain`), which need both
/// the caster and the target position and are spawned by the caller.
fn procedural_effect_center(
    anchor: ProceduralAnchor,
    source_entity_id: EntityId,
    target_position: Option<Point3<f32>>,
) -> Option<EffectCenter> {
    match anchor {
        ProceduralAnchor::Caster => Some(EffectCenter::Entity(source_entity_id, Point3::new(0.0, 0.0, 0.0))),
        ProceduralAnchor::Target | ProceduralAnchor::Auto => Some(match target_position {
            Some(position) => EffectCenter::Position(position),
            // Self casts (no target cell) anchor their success effect on the caster.
            None => EffectCenter::Entity(source_entity_id, Point3::new(0.0, 0.0, 0.0)),
        }),
        ProceduralAnchor::Travel | ProceduralAnchor::Rain => None,
    }
}

pub struct Client {
    game_file_loader: Arc<GameFileLoader>,
    action_loader: Arc<ActionLoader>,
    #[cfg(feature = "debug")]
    animation_loader: Arc<AnimationLoader>,
    async_loader: Arc<AsyncLoader>,
    effect_loader: Arc<EffectLoader>,
    font_loader: Arc<FontLoader>,
    #[cfg(feature = "debug")]
    map_loader: Arc<MapLoader>,
    sprite_loader: Arc<SpriteLoader>,
    texture_loader: Arc<TextureLoader>,
    library: Arc<Library>,

    interface_renderer: InterfaceRenderer,
    bottom_interface_renderer: GameInterfaceRenderer,
    middle_interface_renderer: GameInterfaceRenderer,
    top_interface_renderer: GameInterfaceRenderer,
    effect_renderer: EffectRenderer,
    #[cfg(feature = "debug")]
    debug_marker_renderer: DebugMarkerRenderer,
    #[cfg(feature = "debug")]
    aabb_instructions: Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    circle_instructions: Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    rectangle_instructions: Vec<DebugRectangleInstruction>,
    model_batches: Vec<ModelBatch>,
    model_instructions: Vec<ModelInstruction>,
    entity_instructions: Vec<EntityInstruction>,
    directional_shadow_model_batches: [Vec<ModelBatch>; PARTITION_COUNT],
    directional_shadow_model_instructions: Vec<ModelInstruction>,
    directional_shadow_entity_instructions: [Vec<EntityInstruction>; PARTITION_COUNT],
    point_shadow_model_batches: Vec<ModelBatch>,
    point_shadow_model_instructions: Vec<ModelInstruction>,
    point_shadow_entity_instructions: Vec<EntityInstruction>,
    point_light_with_shadow_instructions: Vec<PointLightWithShadowInstruction>,
    point_light_instructions: Vec<PointLightInstruction>,

    input_system: InputSystem,

    interface: Interface<'static, ClientState>,
    mouse_cursor: MouseCursor,
    show_interface: bool,
    game_timer: GameTimer,

    #[cfg(feature = "debug")]
    debug_camera: DebugCamera,
    start_camera: StartCamera,
    player_camera: PlayerCamera,
    directional_shadow_camera: DirectionalShadowCamera,
    directional_shadow_partitions: Arc<Mutex<[DirectionalShadowPartition; PARTITION_COUNT]>>,
    point_shadow_camera: PointShadowCamera,

    input_event_buffer: Vec<InputEvent>,
    network_event_buffer: NetworkEventBuffer,
    // TODO: Move or remove this.
    saved_login_data: Option<LoginServerLoginData>,
    // TODO: Move or remove this.
    saved_character_server: Option<CharacterServerInformation>,
    // TODO: Move or remove this.
    saved_login_server_address: Option<SocketAddr>,
    // TODO: Move or remove this.
    saved_password: String,
    // TODO: Move or remove this.
    saved_username: String,
    // TODO: Move or remove this.
    saved_packet_version: SupportedPacketVersion,

    /// Stat updates that arrived before the local player entity existed.
    ///
    /// The server sends its initial stat burst (including the values needed for
    /// the experience bars) right after the map load. If those updates are
    /// handled before the player entity was created they are buffered here and
    /// applied as soon as the player exists (see `apply_pending_stat_updates`).
    pending_stat_updates: PendingStatUpdates,

    /// Last tile position the minimap marker was updated with, used to derive
    /// the direction the player is facing from the movement delta.
    minimap_last_tile: Option<TilePosition>,

    /// Explicit state of the `M` key window cycle: small minimap -> big map ->
    /// closed -> small minimap. The windows themselves are still the source of
    /// truth (a window can be closed with its X button), the cycle re-syncs
    /// with the actually open windows when the key is pressed.
    /// Shared style cell read by all running level-99 aura effects, so a style
    /// change in the interface settings applies live without respawning them.
    level_99_aura_style_cell: Arc<AtomicU8>,
    /// Entities that currently have a level-99 aura effect attached.
    level_99_aura_entities: HashSet<EntityId>,

    /// Skill sounds that were already played at the start of an *instant* skill
    /// use, so the follow-up completion packet does not play the same sound a
    /// second time. Keyed by entity id: (skill id, client tick).
    recent_cast_sounds: HashMap<EntityId, (SkillId, ClientTick)>,

    map_window_cycle: MapWindowCycle,

    /// Explicit session/connection lifecycle (login → character selection →
    /// map entry → in game → recovery → shutdown). Owns the map entry/abort,
    /// map load retry, character-server reconnect and shutdown timers that used
    /// to be loose fields on the client.
    session: SessionMachine,

    /// Client message table (`data/msgstringtable.txt`) for `ZC_MSG` (0x0291)
    /// system messages.
    message_table: Option<msgstringtable::MessageTable>,

    /// Aggregated experience gains. rAthena sends one `ZC_NOTIFY_EXP2` packet
    /// per kill (and per party member share), so the amounts are summed and
    /// shown as a single chat line after a short idle period.
    pending_experience: PendingExperience,

    /// Rate limiter for non-modal "some resources could not be loaded" chat
    /// notices (transient asset failures; the warn log has no such limit).
    transient_asset_notices: TransientAssetNoticeLimiter,
    /// Explicit per-entity lifecycle: first spawn vs. duplicate `AddEntity`
    /// (job change/respawn), death/despawn and warp cleanup.
    entity_lifecycle: EntityLifecycle,

    /// Context menu that should be opened once the name of the target entity
    /// is known. Stores the entity id and the screen position of the click.
    pending_context_menu: Option<(EntityId, ScreenPosition)>,

    /// Whether an NPC shop is currently open on the server (i.e. the client has
    /// received the shop item list and has not closed the shop yet). Used to
    /// send the close-shop packet when the shop windows are closed by the user
    /// so that the map server does not keep the NPC interaction (and the
    /// movement lock) alive.
    shop_open: bool,

    /// Loaded guild emblem textures, keyed by guild id. Requests are sent when
    /// a player's guild name becomes known (the entity-appear packets only
    /// carry the guild id) and are rate-limited to one request per guild
    /// per map session.
    guild_emblems: GuildEmblemCache,

    /// Deadline/retry tracking for asynchronous animation and asset loads of
    /// visible entities. A load that never completes or fails is retried a
    /// bounded number of times instead of silently leaving the resource
    /// missing forever.
    asset_retries: ResourceRetryQueue<LoaderId>,

    /// Name of the map that is currently loaded. Used to detect same-map
    /// teleports (fly wing, AL_TELEPORT, ...): the server sends a normal map
    /// move packet even when the destination is the current map, but reloading
    /// the whole map for that would hide the teleport and stall the client.
    current_map_name: Option<String>,

    particle_holder: ParticleHolder,
    /// Cached classic arrow sprite/actions for bow attack projectiles.
    arrow_assets: Option<std::sync::Arc<ArrowAssets>>,
    /// Set when the arrow assets could not be loaded (avoids retrying every
    /// hit).
    arrow_assets_failed: bool,
    point_light_manager: PointLightManager,
    effect_holder: EffectHolder,
    chat_bubbles: ChatBubbles,
    /// Pending item drag: (press time, press position). A drag only starts
    /// after a short hold or a few pixels of movement, so a plain click
    /// does not pick items up.
    pending_item_drag: Option<(std::time::Instant, ScreenPosition)>,
    skill_name_texts: SkillNameTexts,
    cast_bars: CastBars,
    /// De-duplicates the completion effect of one skill cast (ground-skill,
    /// no-damage and damage packets can all describe the same cast).
    skill_completion_tracker: SkillCompletionTracker,
    /// When the after-cast act delay of the last own skill ends, in client
    /// ticks. Server table values; paces follow-up skill/item actions only.
    aftercast_act_until: Option<ClientTick>,
    warp_portal: Option<WarpPortal>,
    /// Active warp-portal skill units: (entity id, world position, spawn time).
    /// Rendered as the flat ground portal of the original client.
    warp_skill_units: Vec<(EntityId, Point3<f32>, Instant)>,
    /// Sound of the entity teleport/logout disappear effect.
    teleport_vanish_sound_effect: SoundEffectKey,
    /// Spawn times of recently started teleport-vanish effects (burst limiter).
    recent_teleport_vanishes: Vec<Instant>,
    path_finder: PathFinder,

    point_light_set_buffer: ResourceSetBuffer<LightSourceKey>,
    directional_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: ResourceSetBuffer<ObjectKey>,

    #[cfg(feature = "debug")]
    pathing_texture_set: Arc<TextureSet>,
    #[cfg(feature = "debug")]
    tile_texture_set: Arc<TextureSet>,

    main_menu_click_sound_effect: SoundEffectKey,

    #[cfg(feature = "debug")]
    networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    networking_system: NetworkingSystem<ClientPacketCallback>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    skill_sounds: SkillSoundPlayer,
    active_master_volume: f32,
    active_sound_effect_volume: f32,
    active_background_music_volume: f32,
    active_interface_settings: InterfaceSettings,
    active_graphics_settings: GraphicsSettings,
    graphics_engine: GraphicsEngine,
    queue: Queue,
    #[cfg(feature = "debug")]
    device: Device,
    window: Option<Arc<Window>>,
    /// Whether the interface scaling should be initialized from the DPI scale
    /// factor of the window. This is only done on the first start, i.e. when
    /// there is no saved interface settings file yet.
    initialize_interface_scaling_from_dpi: bool,
    /// Scale factor that was last reported by the windowing system.
    ///
    /// Used to keep the last known pointer position in the coordinate space of
    /// the new scale when the compositor changes the scale factor (native
    /// Wayland fractional scaling, moving the window between outputs with
    /// different scales), so that clicks arriving before the next pointer
    /// motion event are hit-tested correctly.
    last_scale_factor: f64,

    /// Whether the client window is backed by the native Wayland backend.
    backend_is_wayland: bool,
    /// Whether presentation is currently paused because the native Wayland
    /// window is in the background (see [`Self::should_pause_presentation`]).
    presentation_paused: bool,
    /// Whether at least one frame was presented already.
    ///
    /// The first frame has to be presented even when the window is in the
    /// background: without a committed buffer the compositor never maps the
    /// window, so the client would be invisible. Only after that it is safe to
    /// stop presenting while the window is hidden.
    has_presented_frame: bool,
    /// Whether the window is currently reported as occluded by the windowing
    /// system. Occluded windows receive no redraw events on some platforms, but
    /// the state is applied as soon as the client renders again.
    window_occluded: bool,
    /// Whether the window was minimized the last time we checked. Querying this
    /// state is a synchronous round trip on X11, so it is only refreshed when
    /// the focus or the occlusion state of the window changes.
    window_minimized: bool,
    /// Set through `KORANGAR_FORCE_UNFOCUSED`/`KORANGAR_UNFOCUSED_ALWAYS`.
    /// Forces the background frame rate limit, which is useful for headless
    /// verification clients.
    force_background: bool,
    /// The frame rate limit that is currently applied to the graphics engine.
    /// `0` means unlimited.
    active_frames_per_second_limit: u32,
    /// When the effective frame rate should be logged next. Only set when
    /// frame rate logging is enabled through `KORANGAR_FPS_LOG`.
    frames_per_second_log_deadline: Option<Instant>,
    /// When the next layout/text cache performance line should be logged. Only
    /// set when profiling logging is enabled through `KORANGAR_PERF_LOG`.
    performance_log_deadline: Option<Instant>,
    /// Set when UI state was changed while the interface frame was alive (where
    /// `self.interface` cannot be borrowed again). Applied at the start of the
    /// next frame.
    interface_layout_invalidation_pending: bool,

    map: Option<Arc<Map>>,
    client_state: State<ClientState>,
}

impl Client {
    pub fn init(sync_cache: bool) -> Option<Self> {
        crate::logging::log_info!("client: initializing (sync_cache={})", sync_cache);

        // We start a frame so that functions trying to start a measurement don't panic.
        #[cfg(feature = "debug")]
        let _measurement = threads::Main::start_frame();

        initialize_shutdown_signal();

        time_phase!("create global thread pool", {
            rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .start_handler(|_| init_tls_rand())
                .build_global()
                .unwrap();
        });

        time_phase!("seed main random instance", {
            init_tls_rand();
        });

        crate::logging::log_debug!("client: working directory is {:?}", std::env::current_dir());

        // Check if korangar is in the correct working directory and if not, try to
        // correct it.
        // NOTE: This check might be temporary or feature gated in the future.
        time_phase!("adjust working directory", {
            if !std::fs::metadata("archive").is_ok_and(|metadata| metadata.is_dir()) {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to find archive directory, attempting to change working directory {}",
                    "warning".yellow(),
                    "korangar".magenta()
                );

                if let Err(_error) = std::env::set_current_dir("korangar") {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to change working directory: {:?}", "error".red(), _error);
                }
            }
        });

        time_phase!("load graphics settings", {
            let picker_value = Arc::new(AtomicU64::new(0));
            let directional_shadow_partitions = Arc::new(Mutex::new([DirectionalShadowPartition::default(); PARTITION_COUNT]));
            let input_system = InputSystem::new(picker_value.clone());
            let mut graphics_settings = GraphicsSettings::new();

            if graphics_settings.apply_environment_overrides() {
                println!(
                    "[framerate] environment overrides applied: max_fps={} background_max_fps={}",
                    graphics_settings.max_fps.get(),
                    graphics_settings.background_max_fps.get(),
                );
            }

            println!(
                "[framerate] frame rate limits: max_fps={} background_max_fps={}",
                graphics_settings.max_fps.get(),
                graphics_settings.background_max_fps.get(),
            );

            crate::logging::log_debug!(
                "client: graphics settings loaded (high_quality_interface={})",
                graphics_settings.high_quality_interface,
            );

            // Read this before `ClientState::new` loads (and thereby potentially
            // rewrites) the interface settings.
            let initialize_interface_scaling_from_dpi = !InterfaceSettings::exists();
        });

        let force_background =
            environment_flag(FORCE_UNFOCUSED_ENVIRONMENT_VARIABLE) || environment_flag(FORCE_UNFOCUSED_ALIAS_ENVIRONMENT_VARIABLE);

        if force_background {
            println!("[framerate] window is always treated as being in the background (environment override)");
        }

        time_phase!("create adapter", {
            let instance = Instance::new(InstanceDescriptor {
                backends: Backends::all().with_env(),
                flags: InstanceFlags::from_build_config().with_env(),
                memory_budget_thresholds: MemoryBudgetThresholds::default(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::Automatic,
                        fence_behavior: GlFenceBehavior::Normal,
                        debug_fns: GlDebugFns::Auto,
                    },
                    dx12: Dx12BackendOptions {
                        shader_compiler: Dx12Compiler::Auto.with_env(),
                        presentation_system: Dx12SwapchainKind::DxgiFromHwnd,
                        latency_waitable_object: Dx12UseFrameLatencyWaitableObject::Wait,
                        force_shader_model: ForceShaderModelToken::default(),
                        agility_sdk: None,
                    },
                    noop: NoopBackendOptions { enable: false },
                },
                // On Vulkan, Metal and Dx12, this is currently unused.
                display: None,
            });

            let adapter = pollster::block_on(async { initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });

            #[cfg(feature = "debug")]
            {
                let adapter_info = adapter.get_info();
                print_debug!("using adapter {} ({})", adapter_info.name, adapter_info.backend);
                print_debug!("using device {} ({})", adapter_info.device, adapter_info.vendor);
                print_debug!("using driver {} ({})", adapter_info.driver, adapter_info.driver_info);
            }
        });

        time_phase!("create device", {
            let capabilities = Capabilities::from_adapter(&adapter);

            let (device, queue) = pollster::block_on(async {
                adapter
                    .request_device(&DeviceDescriptor {
                        label: None,
                        required_features: capabilities.get_required_features(),
                        required_limits: capabilities.get_required_limits(),
                        experimental_features: ExperimentalFeatures::disabled(),
                        memory_hints: MemoryHints::Performance,
                        trace: Trace::Off,
                    })
                    .await
                    .unwrap()
            });

            #[cfg(feature = "debug")]
            device.on_uncaptured_error(Arc::new(error_handler));

            #[cfg(feature = "debug")]
            print_debug!("received {} and {}", "queue".magenta(), "device".magenta());
        });

        time_phase!("create shader compiler", {
            let shader_compiler = ShaderCompiler::new(device.clone());
        });

        time_phase!("create game file loader", {
            let game_file_loader = Arc::new(GameFileLoader::default());

            game_file_loader.load_archives_from_settings();
            game_file_loader.load_patched_lua_files();
        });

        // The message table is small (a few hundred KiB) and is needed for every
        // `ZC_MSG` system message, so load it once at startup.
        time_phase!("load message table", {
            let message_table = load_message_table(game_file_loader.as_ref());
        });

        time_phase!("calculate game file hash", {
            let game_file_hash = game_file_loader.calculate_hash();
            #[cfg(feature = "debug")]
            print_debug!("game file hash: {}", game_file_hash);
        });

        time_phase!("create audio engine", {
            let audio_engine = Arc::new(AudioEngine::new(game_file_loader.clone()));
        });

        let skill_sounds = SkillSoundPlayer::new(audio_engine.clone(), game_file_loader.clone());

        if skill_sounds.is_muted() {
            println!("[sound] muted mode active: skill sound playback is logged, not audible");
        }

        time_phase!("create resource managers", {
            std::fs::create_dir_all(MENU_THEMES_PATH).unwrap();
            std::fs::create_dir_all(IN_GAME_THEMES_PATH).unwrap();
            std::fs::create_dir_all(WORLD_THEMES_PATH).unwrap();

            let model_loader = Arc::new(ModelLoader::new(game_file_loader.clone(), capabilities.bindless_support()));
            let texture_loader = Arc::new(TextureLoader::new(
                device.clone(),
                queue.clone(),
                &shader_compiler,
                &capabilities,
                game_file_loader.clone(),
            ));
            let video_loader = Arc::new(VideoLoader::new(game_file_loader.clone(), texture_loader.clone()));
            // `NotoSans` only covers Latin/Greek/Cyrillic. The server sends free
            // text (chat, NPC dialogue) and the 2019 quest scripts use KS X 1001
            // symbols, so a Hangul/KS-capable fallback font is required to avoid
            // `.notdef` boxes. See `archive/data/font/README.md`.
            let font_loader = Arc::new(FontLoader::new(
                &["NotoSans".to_owned(), "NanumGothic".to_owned()],
                &game_file_loader,
                &device,
                &queue,
            ));
            let map_loader = Arc::new(MapLoader::new(
                device.clone(),
                queue.clone(),
                game_file_loader.clone(),
                audio_engine.clone(),
                capabilities.bindless_support(),
            ));
            let sprite_loader = Arc::new(SpriteLoader::new(game_file_loader.clone(), texture_loader.clone()));
            let action_loader = Arc::new(ActionLoader::new(game_file_loader.clone(), audio_engine.clone()));
            let effect_loader = Arc::new(EffectLoader::new(game_file_loader.clone()));
            let animation_loader = Arc::new(AnimationLoader::new());

            let library = Arc::new(Library::new(&game_file_loader).unwrap_or_else(|_| {
                // The library not being created correctly means that the lua files were
                // not valid. It's possible that the archive was copied from a
                // different machine with a different architecture, so the one thing
                // we can try is generating it again.

                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to execute lua files; attempting to fix it by re-patching",
                    "error".red()
                );

                game_file_loader.remove_patched_lua_files();
                game_file_loader.load_patched_lua_files();

                Library::new(&game_file_loader).unwrap()
            }));

            if sync_cache {
                sync_cache_archive(&game_file_loader, texture_loader, game_file_hash);
                return None;
            }

            game_file_loader.load_cache_archive(game_file_hash);

            let async_loader = Arc::new(AsyncLoader::new(
                action_loader.clone(),
                animation_loader.clone(),
                map_loader.clone(),
                model_loader.clone(),
                sprite_loader.clone(),
                texture_loader.clone(),
                video_loader.clone(),
                library.clone(),
            ));

            let interface_renderer = InterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                font_loader.clone(),
                texture_loader.clone(),
                graphics_settings.high_quality_interface,
            );
            let bottom_interface_renderer = GameInterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                INITIAL_SCALING_FACTOR,
                font_loader.clone(),
                #[cfg(feature = "debug")]
                &texture_loader,
            );
            let middle_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let top_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let effect_renderer = EffectRenderer::new(INITIAL_SCREEN_SIZE);
            #[cfg(feature = "debug")]
            let debug_marker_renderer = DebugMarkerRenderer::new();

            #[cfg(feature = "debug")]
            let aabb_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let circle_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let rectangle_instructions = Vec::default();
            let model_batches = Vec::default();
            let model_instructions = Vec::default();
            let entity_instructions = Vec::default();
            let directional_shadow_model_batches = Default::default();
            let directional_shadow_model_instructions = Vec::default();
            let directional_shadow_entity_instructions = Default::default();
            let point_shadow_model_batches = Vec::default();
            let point_shadow_model_instructions = Vec::default();
            let point_shadow_entity_instructions = Vec::default();
            let point_light_with_shadow_instructions = Vec::default();
            let point_light_instructions = Vec::default();
        });

        time_phase!("load warp portal assets", {
            let warp_portal = WarpPortal::new(&texture_loader, &sprite_loader);
        });

        time_phase!("create graphics engine", {
            let graphics_engine = GraphicsEngine::initialize(GraphicsEngineDescriptor {
                capabilities,
                adapter,
                instance,
                device: device.clone(),
                queue: queue.clone(),
                shader_compiler,
                texture_loader: texture_loader.clone(),
                picker_value,
                directional_shadow_partitions: directional_shadow_partitions.clone(),
            });
        });

        time_phase!("initialize interface", {
            let mut interface = Interface::new(font_loader.clone(), INITIAL_SCREEN_SIZE);
            let mouse_cursor = MouseCursor::new(&sprite_loader, &action_loader);
            let show_interface = true;
        });

        time_phase!("initialize timer", {
            let game_timer = GameTimer::new();
        });

        time_phase!("initialize camera", {
            #[cfg(feature = "debug")]
            let debug_camera = DebugCamera::new();
            let mut start_camera = StartCamera::new();
            let player_camera = PlayerCamera::new();
            let mut directional_shadow_camera = DirectionalShadowCamera::new();
            let point_shadow_camera = PointShadowCamera::new();
            start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
        });

        // TODO: Move all of these to the ClientState
        let saved_login_data: Option<LoginServerLoginData> = None;
        let saved_character_server: Option<CharacterServerInformation> = None;
        let saved_login_server_address = None;
        let saved_password = String::new();
        let saved_username = String::new();
        let saved_packet_version = FALLBACK_PACKET_VERSION;

        time_phase!("initialize networking", {
            #[cfg(not(feature = "debug"))]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(ClientPacketCallback);

            #[cfg(feature = "debug")]
            let (packet_history, packet_history_callback) = PacketHistory::new();
            #[cfg(feature = "debug")]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(packet_history_callback);
        });

        time_phase!("create resources", {
            let input_event_buffer = Vec::new();

            let particle_holder = ParticleHolder::default();
            let point_light_manager = PointLightManager::new();
            let effect_holder = EffectHolder::default();
            let path_finder = PathFinder::default();

            let point_light_set_buffer = ResourceSetBuffer::default();
            let directional_shadow_object_set_buffer = ResourceSetBuffer::default();
            let point_shadow_object_set_buffer = ResourceSetBuffer::default();
            let deferred_object_set_buffer = ResourceSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ResourceSetBuffer::default();

            #[cfg(feature = "debug")]
            let pathing_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "pathing", &[
                "pathing_goal.png",
                "pathing_straight.png",
                "pathing_diagonal.png",
            ]);
            #[cfg(feature = "debug")]
            let pathing_texture_set = Arc::new(pathing_texture_set);

            #[cfg(feature = "debug")]
            let tile_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "tile", &[
                "tile_0.png",
                "tile_1.png",
                "tile_2.png",
                "tile_3.png",
                "tile_4.png",
                "tile_5.png",
                "tile_6.png",
            ]);
            #[cfg(feature = "debug")]
            let tile_texture_set = Arc::new(tile_texture_set);

            let main_menu_click_sound_effect = audio_engine.load(MAIN_MENU_CLICK_SOUND_EFFECT);
            let teleport_vanish_sound_effect = audio_engine.load(TELEPORT_VANISH_SOUND_EFFECT);
        });

        time_phase!("load default map", {
            let map = map_loader
                .load(
                    DEFAULT_MAP.to_string(),
                    &model_loader,
                    texture_loader.clone(),
                    video_loader,
                    &library,
                )
                .expect("failed to load initial map");

            directional_shadow_camera.set_level_bound(map.get_level_bound());

            audio_engine.play_background_music_track(UI_BACKGROUND_MUSIC);
            map.set_ambient_sound_sources(&audio_engine);
        });

        time_phase!("create client state", {
            let mut client_state = State::new(ClientState::new(
                &game_file_loader,
                graphics_settings.clone(),
                #[cfg(feature = "debug")]
                packet_history,
            ));
        });

        // Generate the direction arrow textures of the minimap marker once at
        // startup. They are pre-rotated so the renderer only has to pick one.
        {
            let marker_textures = generate_direction_markers(&texture_loader);
            client_state
                .follow_mut(crate::client_state().minimap())
                .set_marker_textures(marker_textures);
        }

        let active_interface_settings = client_state.follow(crate::client_state().interface_settings()).clone();

        // The audio engine is created before the client state, so the saved
        // volumes can only be applied once the settings are loaded.
        let audio_settings = client_state.follow(crate::client_state().audio_settings());
        let active_master_volume = audio_settings.master_volume_linear();
        let active_sound_effect_volume = audio_settings.sound_effect_volume_linear();
        let active_background_music_volume = audio_settings.background_music_track_volume();

        audio_engine.set_main_volume(active_master_volume);
        audio_engine.set_sound_effect_volume(active_sound_effect_volume);
        audio_engine.set_spatial_sound_effect_volume(active_sound_effect_volume);
        audio_engine.set_background_music_volume(active_background_music_volume);

        crate::logging::log_debug!(
            "settings: applied saved audio volumes (master={:.2}, sound_effects={:.2}, background_music={:.2}, music_muted={}; effective \
             master={:.2}, sound_effects={:.2}, spatial_sound_effects={:.2}, background_music={:.2})",
            active_master_volume,
            active_sound_effect_volume,
            active_background_music_volume,
            audio_settings.background_music_muted,
            audio_settings.effective_master_volume(),
            audio_settings.effective_sound_effect_volume(),
            audio_settings.effective_spatial_sound_effect_volume(),
            audio_settings.effective_background_music_volume(),
        );

        interface.open_window(LoginWindow::new(
            crate::client_state().login_window(),
            crate::client_state().login_settings(),
            crate::client_state().client_info(),
        ));

        Some(Self {
            game_file_loader,
            action_loader,
            #[cfg(feature = "debug")]
            animation_loader,
            async_loader,
            effect_loader,
            font_loader,
            #[cfg(feature = "debug")]
            map_loader,
            sprite_loader,
            texture_loader,
            library,
            interface_renderer,
            bottom_interface_renderer,
            middle_interface_renderer,
            top_interface_renderer,
            effect_renderer,
            #[cfg(feature = "debug")]
            debug_marker_renderer,
            #[cfg(feature = "debug")]
            aabb_instructions,
            #[cfg(feature = "debug")]
            circle_instructions,
            #[cfg(feature = "debug")]
            rectangle_instructions,
            model_batches,
            model_instructions,
            entity_instructions,
            directional_shadow_model_batches,
            directional_shadow_model_instructions,
            directional_shadow_entity_instructions,
            point_shadow_model_batches,
            point_shadow_model_instructions,
            point_shadow_entity_instructions,
            point_light_with_shadow_instructions,
            point_light_instructions,
            input_system,
            interface,
            mouse_cursor,
            show_interface,
            game_timer,
            #[cfg(feature = "debug")]
            debug_camera,
            start_camera,
            player_camera,
            directional_shadow_camera,
            directional_shadow_partitions,
            point_shadow_camera,
            input_event_buffer,
            network_event_buffer,
            saved_login_data,
            saved_character_server,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_packet_version,
            pending_stat_updates: PendingStatUpdates::new(),
            message_table,
            pending_experience: PendingExperience::default(),
            session: SessionMachine::new(),
            entity_lifecycle: EntityLifecycle::new(),
            asset_retries: ResourceRetryQueue::new(ResourcePolicy::ANIMATION_DATA),
            pending_context_menu: None,
            shop_open: false,
            guild_emblems: GuildEmblemCache::default(),
            transient_asset_notices: TransientAssetNoticeLimiter::default(),
            current_map_name: None,
            level_99_aura_style_cell: Arc::new(AtomicU8::new(AuraStyle::Auto.encode())),
            level_99_aura_entities: HashSet::new(),
            recent_cast_sounds: HashMap::new(),
            map_window_cycle: MapWindowCycle::Closed,
            minimap_last_tile: None,
            particle_holder,
            arrow_assets: None,
            arrow_assets_failed: false,
            point_light_manager,
            effect_holder,
            chat_bubbles: ChatBubbles::default(),
            pending_item_drag: None,
            skill_name_texts: SkillNameTexts::default(),
            cast_bars: CastBars::default(),
            skill_completion_tracker: SkillCompletionTracker::default(),
            aftercast_act_until: None,
            warp_portal,
            warp_skill_units: Vec::new(),
            teleport_vanish_sound_effect,
            recent_teleport_vanishes: Vec::new(),
            path_finder,
            point_light_set_buffer,
            directional_shadow_object_set_buffer,
            point_shadow_object_set_buffer,
            deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            bounding_box_object_set_buffer,
            #[cfg(feature = "debug")]
            pathing_texture_set,
            #[cfg(feature = "debug")]
            tile_texture_set,
            main_menu_click_sound_effect,
            networking_system,
            audio_engine,
            skill_sounds,
            active_master_volume,
            active_sound_effect_volume,
            active_background_music_volume,
            active_interface_settings,
            active_graphics_settings: graphics_settings,
            graphics_engine,
            queue,
            #[cfg(feature = "debug")]
            device,
            window: None,
            backend_is_wayland: false,
            presentation_paused: false,
            has_presented_frame: false,
            window_occluded: false,
            window_minimized: false,
            force_background,
            active_frames_per_second_limit: 0,
            frames_per_second_log_deadline: frames_per_second_log_deadline(),
            performance_log_deadline: performance_log_deadline(),
            interface_layout_invalidation_pending: false,
            initialize_interface_scaling_from_dpi,
            last_scale_factor: 1.0,

            map: Some(map),
            client_state,
        })
    }

    pub fn run(&mut self) {
        let event_loop = match create_event_loop() {
            Ok(event_loop) => event_loop,
            Err(error) => {
                crate::logging::log_error!("[display] {error}");
                crate::logging::log_error!(
                    "[display] hint: use KORANGAR_X11=1 to force X11/XWayland or KORANGAR_WAYLAND=1 to force native Wayland"
                );
                std::process::exit(1);
            }
        };

        event_loop.set_control_flow(ControlFlow::Poll);

        if let Err(error) = event_loop.run_app(self) {
            crate::logging::log_error!("[display] the event loop stopped with an error: {error}");
            std::process::exit(1);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn clear_render_instructions(&mut self) {
        self.interface_renderer.clear();
        self.bottom_interface_renderer.clear();
        self.middle_interface_renderer.clear();
        self.top_interface_renderer.clear();
        self.effect_renderer.clear();
        #[cfg(feature = "debug")]
        self.debug_marker_renderer.clear();

        #[cfg(feature = "debug")]
        self.aabb_instructions.clear();
        #[cfg(feature = "debug")]
        self.circle_instructions.clear();
        #[cfg(feature = "debug")]
        self.rectangle_instructions.clear();
        self.model_batches.clear();
        self.model_instructions.clear();
        self.entity_instructions.clear();
        self.directional_shadow_model_batches.iter_mut().for_each(|batch| batch.clear());
        self.directional_shadow_model_instructions.clear();
        self.directional_shadow_entity_instructions
            .iter_mut()
            .for_each(|instructions| instructions.clear());
        self.point_shadow_model_batches.clear();
        self.point_shadow_model_instructions.clear();
        self.point_shadow_entity_instructions.clear();
        self.point_light_with_shadow_instructions.clear();
        self.point_light_instructions.clear();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_client_state(&mut self) {
        // Unset the highlighted skill just before applying. That way, if the skill is
        // still hovered the value will be the same as before, if not it will clear the
        // highlighted.
        *self.client_state.follow_mut(client_state().skill_tree_window().highlighted_skill()) = None;

        // Apply the game state after all the UI work + rendering is done.
        if let Err(_errors) = self.client_state.apply() {
            #[cfg(feature = "debug")]
            {
                print_debug!("[{}] failed to apply {} updates: ", "error".red(), _errors.len());
                _errors.into_iter().for_each(|error| print_debug!("path: {}", error.type_name));
            }
        }
    }

    /// Apply any graphics or interface setting changes that the user
    /// dispatched during the previous frame.
    ///
    /// May reconfigure the GPU surface (MSAA, SSAA, present mode, shadow
    /// resolution, etc.). Surface reconfiguration is only safe between
    /// presenting the previous frame and acquiring the next swapchain image,
    /// so this *must* be called after [`Self::update_client_state`] (so the
    /// user's dispatched changes are visible) and *before*
    /// `graphics_engine.wait_for_next_frame()`. Calling it after
    /// `wait_for_next_frame` has been observed to cause surface configuration
    /// errors under DX12.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_settings(&mut self) {
        let graphics_settings = self.client_state.follow(client_state().graphics_settings());
        let audio_settings = self.client_state.follow(client_state().audio_settings());

        let master_volume = audio_settings.master_volume_linear();
        let sound_effect_volume = audio_settings.sound_effect_volume_linear();
        let background_music_volume = audio_settings.background_music_track_volume();

        let mut audio_settings_changed = false;

        if (self.active_master_volume - master_volume).abs() > f32::EPSILON {
            self.audio_engine.set_main_volume(master_volume);
            self.active_master_volume = master_volume;
            audio_settings_changed = true;
        }

        if (self.active_sound_effect_volume - sound_effect_volume).abs() > f32::EPSILON {
            self.audio_engine.set_sound_effect_volume(sound_effect_volume);
            self.audio_engine.set_spatial_sound_effect_volume(sound_effect_volume);
            self.active_sound_effect_volume = sound_effect_volume;
            audio_settings_changed = true;
        }

        if (self.active_background_music_volume - background_music_volume).abs() > f32::EPSILON {
            self.audio_engine.set_background_music_volume(background_music_volume);
            self.active_background_music_volume = background_music_volume;
            audio_settings_changed = true;
        }

        if audio_settings_changed {
            // Persist immediately so a volume change survives a crash or a
            // forced exit, not only a clean logout (`Drop` saves as well).
            audio_settings.save();

            crate::logging::log_debug!(
                "settings: audio volumes applied (master={:.2}, sound_effects={:.2}, background_music={:.2}, music_muted={}; effective \
                 master={:.2}, sound_effects={:.2}, spatial_sound_effects={:.2}, background_music={:.2})",
                master_volume,
                sound_effect_volume,
                background_music_volume,
                audio_settings.background_music_muted,
                audio_settings.effective_master_volume(),
                audio_settings.effective_sound_effect_volume(),
                audio_settings.effective_spatial_sound_effect_volume(),
                audio_settings.effective_background_music_volume(),
            );
        }

        if self.active_graphics_settings.vsync != graphics_settings.vsync {
            self.graphics_engine.set_vsync(graphics_settings.vsync);
            self.active_graphics_settings.vsync = graphics_settings.vsync;
        }

        if self.active_graphics_settings.triple_buffering != graphics_settings.triple_buffering {
            self.graphics_engine.set_triple_buffering(graphics_settings.triple_buffering);
            self.active_graphics_settings.triple_buffering = graphics_settings.triple_buffering;
        }

        if self.active_graphics_settings.texture_filtering != graphics_settings.texture_filtering {
            self.graphics_engine.set_texture_sampler_type(graphics_settings.texture_filtering);
            self.active_graphics_settings.texture_filtering = graphics_settings.texture_filtering;
        }

        if self.active_graphics_settings.msaa != graphics_settings.msaa {
            self.graphics_engine.set_msaa(graphics_settings.msaa);
            self.active_graphics_settings.msaa = graphics_settings.msaa;
        }

        if self.active_graphics_settings.ssaa != graphics_settings.ssaa {
            self.graphics_engine.set_ssaa(graphics_settings.ssaa);
            self.active_graphics_settings.ssaa = graphics_settings.ssaa;
        }

        if self.active_graphics_settings.screen_space_anti_aliasing != graphics_settings.screen_space_anti_aliasing {
            self.graphics_engine
                .set_screen_space_anti_aliasing(graphics_settings.screen_space_anti_aliasing);
            self.active_graphics_settings.screen_space_anti_aliasing = graphics_settings.screen_space_anti_aliasing;
        }

        if self.active_graphics_settings.shadow_resolution != graphics_settings.shadow_resolution {
            self.graphics_engine.set_shadow_resolution(graphics_settings.shadow_resolution);
            self.active_graphics_settings.shadow_resolution = graphics_settings.shadow_resolution;
        }

        if self.active_graphics_settings.high_quality_interface != graphics_settings.high_quality_interface {
            self.interface_renderer
                .update_high_quality_interface(graphics_settings.high_quality_interface);
            self.graphics_engine
                .set_high_quality_interface(graphics_settings.high_quality_interface);
            self.active_graphics_settings.high_quality_interface = graphics_settings.high_quality_interface;
        }

        // Apply resolution and fullscreen changes live. The window resize generates a
        // `WindowEvent::Resized`, which is where the graphics surface and the interface
        // are updated, so there is no need to reconfigure anything here.
        let fullscreen_changed = self.active_graphics_settings.fullscreen != graphics_settings.fullscreen;
        let resolution_changed = self.active_graphics_settings.window_resolution != graphics_settings.window_resolution;

        if fullscreen_changed || resolution_changed {
            if let Some(window) = self.window.as_ref() {
                // Leaving fullscreen first, then restoring the window size works around
                // window managers that ignore inner size requests while a window is
                // still marked as fullscreen.
                if fullscreen_changed && !graphics_settings.fullscreen {
                    window.set_fullscreen(None);
                }

                if !graphics_settings.fullscreen {
                    match graphics_settings.window_resolution {
                        WindowResolution::Desktop => window.set_maximized(true),
                        window_resolution => {
                            if let Some((width, height)) = window_resolution.size() {
                                window.set_maximized(false);
                                let _ = window.request_inner_size(PhysicalSize::new(width, height));
                            }
                        }
                    }
                }

                if fullscreen_changed && graphics_settings.fullscreen {
                    window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                }
            }

            self.active_graphics_settings.fullscreen = graphics_settings.fullscreen;
            self.active_graphics_settings.window_resolution = graphics_settings.window_resolution;
        }

        let language = *self.client_state.follow(client_state().interface_settings().language());

        if self.active_interface_settings.language != language {
            *self.client_state.follow_mut(client_state().localization()) = Localization::load_language(&self.game_file_loader, language);
            self.active_interface_settings.language = language;
            self.interface.invalidate_layouts();
        }

        let interface_settings = self.client_state.follow_mut(client_state().interface_settings());

        if self.active_interface_settings.menu_theme != interface_settings.menu_theme {
            let menu_theme = interface_settings.menu_theme.clone();
            let theme = InterfaceTheme::load(state::theme::InterfaceThemeType::Menu, &menu_theme);
            *self.client_state.follow_mut(client_state().menu_theme()) = theme;
            self.active_interface_settings.menu_theme = menu_theme;
            self.interface.invalidate_layouts();
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.in_game_theme != interface_settings.in_game_theme {
            let in_game_theme = interface_settings.in_game_theme.clone();
            let theme = InterfaceTheme::load(InterfaceThemeType::InGame, &in_game_theme);
            *self.client_state.follow_mut(client_state().in_game_theme()) = theme;
            self.active_interface_settings.in_game_theme = in_game_theme;
            self.interface.invalidate_layouts();
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.world_theme != interface_settings.world_theme {
            let world_theme = interface_settings.world_theme.clone();
            let theme = WorldTheme::load(&world_theme);
            *self.client_state.follow_mut(client_state().world_theme()) = theme;
            self.active_interface_settings.world_theme = world_theme;
        }

        // The active frame rate limit depends on the persisted settings *and*
        // on whether the window is currently in the background, so it has to be
        // re-evaluated whenever the settings or the window state changed.
        self.apply_frames_per_second_limit();
    }

    /// Whether the client window has keyboard focus right now.
    fn window_has_focus(&self) -> bool {
        self.window.as_ref().is_some_and(|window| window.has_focus())
    }

    /// Whether the client window is currently considered to be in the
    /// background, and should therefore use the background frame rate limit.
    fn window_is_in_background(&self) -> bool {
        if self.force_background || self.window_occluded || self.window_minimized {
            return true;
        }

        self.window.is_some() && !self.window_has_focus()
    }

    /// Whether presenting frames has to be paused because the native Wayland
    /// window is in the background.
    ///
    /// Wayland does not report occlusion to clients. When the window is hidden
    /// (covered by another window, minimized or simply unfocused) KWin can stop
    /// releasing the swapchain images it still holds. NVIDIA's Vulkan Wayland
    /// WSI then blocks forever inside the blocking image acquisition
    /// (`drmSyncobjTimelineWait`), which freezes the whole client. To keep the
    /// native Wayland backend usable, the client does not touch the swapchain
    /// while the window is in the background; the next redraw after focus is
    /// regained presents normally again.
    ///
    /// Set `KORANGAR_WAYLAND_BACKGROUND_RENDER=1` to keep rendering in the
    /// background at the configured background frame rate (this restores the
    /// old behavior, but exposes the driver deadlock again).
    fn should_pause_presentation(&self) -> bool {
        if !self.backend_is_wayland {
            return false;
        }

        if environment_flag("KORANGAR_WAYLAND_BACKGROUND_RENDER") {
            return false;
        }

        // The very first frame must always be presented, otherwise the compositor never
        // maps the window (an unmapped window is invisible and cannot be
        // focused by a click).
        if !self.has_presented_frame {
            return false;
        }

        self.window_occluded || self.window_minimized || !self.window_has_focus()
    }

    /// Applies a pause of the presentation loop.
    ///
    /// Returns `true` when the frame was skipped, in which case the caller must
    /// not touch the swapchain.
    fn pause_presentation_if_needed(&mut self) -> bool {
        if !self.should_pause_presentation() {
            if self.presentation_paused {
                self.presentation_paused = false;
                crate::logging::log_info!("[surface] presentation resumed (native Wayland window is in the foreground)");
            }

            return false;
        }

        if !self.presentation_paused {
            self.presentation_paused = true;
            crate::logging::log_info!(
                "[surface] presentation paused while the native Wayland window is in the background (set \
                 KORANGAR_WAYLAND_BACKGROUND_RENDER=1 to render in the background)"
            );
        }

        // The event loop keeps running so that network updates, input and focus changes
        // are still processed; a redraw is requested again after this frame.
        std::thread::sleep(Duration::from_millis(15));

        true
    }

    /// Applies the frame rate limit that matches the current window state and
    /// the persisted graphics settings.
    fn apply_frames_per_second_limit(&mut self) {
        let background = self.window_is_in_background();

        let frames_per_second = self
            .client_state
            .follow(client_state().graphics_settings())
            .effective_max_fps(background);

        self.set_frames_per_second_limit(frames_per_second, background);
    }

    /// Applies a frame rate limit to the graphics engine and logs the
    /// transition whenever the limit actually changes.
    fn set_frames_per_second_limit(&mut self, frames_per_second: u32, background: bool) {
        if frames_per_second == self.active_frames_per_second_limit {
            return;
        }

        self.active_frames_per_second_limit = frames_per_second;
        self.graphics_engine.set_framerate_limit(frames_per_second);

        let limit = match frames_per_second {
            0 => "unlimited".to_owned(),
            frames_per_second => format!("{frames_per_second} FPS"),
        };

        let reason = if background { "window in background" } else { "window focused" };

        println!("[framerate] frame rate limit: {limit} ({reason})");
    }

    /// Refreshes the minimized state of the window.
    ///
    /// Minimizing is not reported through a dedicated window event on all
    /// platforms (and querying it is a synchronous round trip on X11), so this
    /// is only called when the window state may have changed.
    fn refresh_window_minimized(&mut self) {
        self.window_minimized = self.window.as_ref().and_then(|window| window.is_minimized()).unwrap_or(false);
    }

    /// Logs the effective frame rate when `KORANGAR_FPS_LOG` is enabled.
    fn log_frames_per_second(&mut self) {
        let Some(deadline) = self.frames_per_second_log_deadline else {
            return;
        };

        let now = Instant::now();

        if now < deadline {
            return;
        }

        self.frames_per_second_log_deadline = Some(now + Duration::from_secs(FRAMERATE_LOG_INTERVAL_SECONDS));

        println!(
            "[fps] cap={} focused={} background={} effective_fps={:.1}",
            self.active_frames_per_second_limit,
            self.window_has_focus(),
            self.window_is_in_background(),
            self.graphics_engine.measured_frames_per_second(),
        );
    }

    /// Logs window layout / text shaping / frame statistics when
    /// `KORANGAR_PERF_LOG` is enabled. Used to measure the effect of the layout
    /// and text caches.
    fn log_performance(&mut self) {
        let Some(deadline) = self.performance_log_deadline else {
            return;
        };

        let now = Instant::now();

        if now < deadline {
            return;
        }

        let interval = now.saturating_duration_since(deadline - Duration::from_secs(PERFORMANCE_LOG_INTERVAL_SECONDS));
        self.performance_log_deadline = Some(now + Duration::from_secs(PERFORMANCE_LOG_INTERVAL_SECONDS));

        let layout_profile = take_layout_profile();
        let text_statistics = take_layout_cache_statistics();
        let frame_profile = take_frame_phase_profile();

        let frames = layout_profile.frames.max(1) as f64;
        let frame_frames = frame_profile.frames.max(1) as f64;
        let interval_seconds = interval.as_secs_f64().max(0.001);
        let create_layout_ms = layout_profile.create_layout_nanos as f64 / 1_000_000.0;
        let lay_out_ms = layout_profile.lay_out_nanos as f64 / 1_000_000.0;
        let windows = layout_profile.windows as f64 / frames;
        let reused_percent = match layout_profile.windows {
            0 => 0.0,
            windows => layout_profile.reused_windows as f64 * 100.0 / windows as f64,
        };
        let text_hit_rate = match text_statistics.calls {
            0 => 0.0,
            calls => text_statistics.hits as f64 * 100.0 / calls as f64,
        };

        let frame_ms = |nanos: u64| nanos as f64 / 1_000_000.0 / frame_frames;

        println!(
            "[perf] fps={:.1} windows={:.1} layout_create={:.3}ms/frame layout={:.3}ms/frame layout_reused={:.0}% text_calls={:.0}/s \
             text_hits={:.1}% text_shaped={:.0}/s",
            self.graphics_engine.measured_frames_per_second(),
            windows,
            create_layout_ms / frames,
            lay_out_ms / frames,
            reused_percent,
            text_statistics.calls as f64 / interval_seconds,
            text_hit_rate,
            text_statistics.misses as f64 / interval_seconds,
        );

        if let Some(line) = crate::perf::loader_log_line() {
            println!("{line}");
        }

        // Budget telemetry: how many effects/particles are alive and how many
        // were dropped by the documented caps since startup. A crowded town
        // should show the counts staying at the caps instead of growing.
        println!(
            "[budget] effects={} effects_dropped={} particles={} particles_dropped={}",
            self.effect_holder.count(),
            self.effect_holder.dropped_effects(),
            self.particle_holder.count(),
            self.particle_holder.dropped_particles(),
        );
        println!(
            "[perf-phase] stage={} wait={:.2}ms sim={:.2}ms interface={:.2}ms world={:.2}ms gpu={:.2}ms frame_total={:.2}ms",
            crate::perf::current_stage().unwrap_or("-"),
            frame_ms(frame_profile.wait_nanos),
            frame_ms(frame_profile.sim_nanos),
            frame_ms(frame_profile.interface_nanos),
            frame_ms(frame_profile.world_nanos),
            frame_ms(frame_profile.gpu_nanos),
            (frame_profile.wait_nanos
                + frame_profile.sim_nanos
                + frame_profile.interface_nanos
                + frame_profile.world_nanos
                + frame_profile.gpu_nanos) as f64
                / 1_000_000.0
                / frame_frames,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_interface_scaling(&mut self, scaling: Scaling) {
        static LAST_INTERFACE_SCALING: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let scaling_bits = scaling.get_factor().to_bits();
        if LAST_INTERFACE_SCALING.swap(scaling_bits, std::sync::atomic::Ordering::Relaxed) != scaling_bits {
            crate::logging::log_info!("interface: applying interface scaling {}", scaling.get_factor());
            self.interface.invalidate_layouts();
        }
        self.bottom_interface_renderer.update_scaling(scaling);
        self.middle_interface_renderer.update_scaling(scaling);
        self.top_interface_renderer.update_scaling(scaling);
    }

    /// Drive the explicit session lifecycle (P1-3): map entry watchdog, map
    /// load retries, map abort recovery and the character-server reconnect
    /// backoff.
    ///
    /// The [`SessionMachine`] only decides *what* has to happen; the client
    /// executes the returned actions here, so the transition logic stays
    /// testable without a window, a network or a game server.
    fn update_session_lifecycle(&mut self) {
        let now = Instant::now();

        let observation = SessionObservation {
            map_assets_loaded: self.map.is_some(),
            player_present: self.client_state.try_follow(this_entity()).is_some(),
            login_server_connected: self.networking_system.is_login_server_connected(),
            login_surface_visible: self.interface.is_window_with_class_open(WindowClass::Login)
                || self.interface.is_window_with_class_open(WindowClass::SelectServer),
        };

        let actions = self.session.tick(now, &observation);

        for action in actions {
            match action {
                SessionAction::AbortMapEntry { reason } => self.abort_map_entry(reason),
                SessionAction::RecoverToCharacterSelection => self.return_to_character_selection(),
                SessionAction::RetryMapLoad { map_name, position } => {
                    self.async_loader.request_map_load(map_name, position);
                }
                SessionAction::MapLoadGaveUp { map_name } => {
                    // Do not leave the client on a black screen: recover to the
                    // character selection and show the reason there.
                    let message = format!("Failed to load the map '{map_name}'. Please try again.");
                    self.session.set_map_load_error(message.clone());
                    self.interface.open_window(ErrorWindow::new(message));
                    self.return_to_character_selection();
                }
                SessionAction::ReconnectCharacterServer => {
                    let Some(saved_login_data) = self.saved_login_data.as_ref() else {
                        self.session.clear_character_reconnect();
                        continue;
                    };
                    let Some(server) = self.saved_character_server.clone() else {
                        self.session.clear_character_reconnect();
                        continue;
                    };

                    self.networking_system
                        .connect_to_character_server(self.saved_packet_version, saved_login_data, server);
                }
                SessionAction::CharacterReconnectGaveUp => {
                    self.interface.open_window(ErrorWindow::new(
                        "Could not connect to the character server. Please log in again.".to_owned(),
                    ));
                }
            }
        }
    }

    /// Prints one line about the display backend and the window properties.
    ///
    /// This is intentional normal output (like the `[fps]` and `[framerate]`
    /// lines) so that it is possible to verify which backend the client picked
    /// and how the compositor scaled the window, also in release builds.
    #[inline(always)]
    fn log_display_information(&self, window: &Window) {
        use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let backend = match window.window_handle().map(|handle| handle.as_raw()) {
            Ok(RawWindowHandle::Wayland(_)) => "wayland",
            Ok(RawWindowHandle::Xcb(_)) => "x11-xcb",
            Ok(RawWindowHandle::Xlib(_)) => "x11-xlib",
            Ok(_) => "unknown",
            Err(_) => "unknown",
        };

        let size = window.inner_size();
        let scale_factor = window.scale_factor();
        let fullscreen = window.fullscreen().is_some();
        let window_resolution = *self.client_state.follow(client_state().graphics_settings().window_resolution());

        println!(
            "[display] backend={backend} size={}x{} scale_factor={scale_factor:.2} fullscreen={fullscreen} \
             resolution={window_resolution:?}",
            size.width, size.height,
        );
    }

    /// Updates every part of the client that depends on the size of the window.
    ///
    /// This is called for every resize event, but also once after the window
    /// was created. The window is created with the configured resolution,
    /// which can differ from [`INITIAL_SCREEN_SIZE`], and winit does not
    /// necessarily send a resize event for the initial size. Without this
    /// the interface and all renderers would keep using the initial size,
    /// which stretches the interface and breaks its hit testing.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    /// Opens the party window on the right side of the screen.
    ///
    /// The default window position is the center of the screen, which overlaps
    /// the permanently open chat window - the "Leave party" button would be
    /// below the chat window and could not be clicked. Placing the party window
    /// next to the chat avoids that.
    fn open_party_window(&mut self) {
        if self.interface.is_window_with_class_open(WindowClass::Party) {
            return;
        }

        let window_size = *self.client_state.follow(client_state().window_size());
        let width = 380.0;
        let position = ScreenPosition {
            left: (window_size.width - width - 20.0).max(20.0),
            top: 100.0,
        };

        self.interface.open_window_at(PartyWindow, position);
    }

    /// Open (or re-open) the hotbar at the bottom center of the screen.
    ///
    /// The hotbar is a fixed-size HUD element: its size is derived from the ten
    /// large slots and it is centered by the client instead of using a cached
    /// window position, so it stays centered at every resolution and interface
    /// scale.
    fn open_hotbar_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Hotbar);

        let window_size = *self.client_state.follow(client_state().window_size());
        let scaling = self.client_state.follow(client_state().interface_settings().scaling()).get_factor();
        let width = HOTBAR_ROW_WIDTH * scaling;
        let height = HOTBAR_WINDOW_HEIGHT * scaling;
        let margin = 18.0 * scaling;
        let position = ScreenPosition {
            left: ((window_size.width - width) / 2.0).max(0.0),
            top: (window_size.height - height - margin).max(0.0),
        };

        self.interface.open_window_at_auto_size(
            HotbarWindow::new(client_state().hotbar().skills(), client_state().skill_tree().skills()),
            position,
        );
    }

    /// Open (or re-open) the transparent status effect strip on the right edge
    /// of the screen, below the minimap.
    fn open_buff_bar_window(&mut self) {
        let window_size = *self.client_state.follow(client_state().window_size());
        let scaling = self.client_state.follow(client_state().interface_settings().scaling()).get_factor();
        // The minimap is a fixed 250 interface pixel tall window at the top
        // right, so the status strip starts below it. The offset is computed
        // from the current scale instead of using a cached window position.
        let minimap_height = 250.0 * scaling;
        let top = 12.0 + minimap_height + 8.0 * scaling;
        let strip_width = BAR_STRIP_WIDTH * scaling;
        let position = ScreenPosition {
            left: (window_size.width - strip_width - 12.0 * scaling).max(0.0),
            top,
        };

        self.interface.close_window_with_class(WindowClass::BuffBar);
        self.interface.open_window_at_auto_size(BuffBarWindow, position);
    }

    /// Reposition the HUD overlay windows after the window size or the
    /// interface scale changed.
    fn reposition_hud_windows(&mut self) {
        let in_game = self.client_state.try_follow(this_entity()).is_some();

        if in_game {
            self.open_hotbar_window();
        }

        if self.interface.is_window_with_class_open(WindowClass::BuffBar) {
            self.interface.close_window_with_class(WindowClass::BuffBar);
        }
    }

    fn update_window_size(&mut self, screen_size: ScreenSize) {
        crate::logging::log_debug!("interface: window size changed to {}x{}", screen_size.width, screen_size.height,);
        *self.client_state.follow_mut(client_state().window_size()) = screen_size;
        self.interface.invalidate_layouts();
        self.graphics_engine.on_resize(screen_size);
        self.interface.update_window_size(screen_size);
        self.interface_renderer.update_window_size(screen_size);
        self.bottom_interface_renderer.update_window_size(screen_size);
        self.middle_interface_renderer.update_window_size(screen_size);
        self.top_interface_renderer.update_window_size(screen_size);
        self.effect_renderer.update_window_size(screen_size);
        self.reposition_hud_windows();
    }

    /// Request the names of all entities that should show a name label.
    ///
    /// Entity details (the name) are loaded lazily from the map server and were
    /// previously only requested for the entity under the mouse. That made the
    /// always on name labels only appear for entities that had been hovered
    /// once before.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn request_visible_entity_names(&mut self) {
        if self.map.is_none() {
            return;
        }

        let show_character_names = *self.client_state.follow(client_state().interface_settings().show_character_names());

        if !show_character_names {
            return;
        }

        // Limit the number of name requests per frame so entering a crowded map does
        // not burst a request per character/entity.
        const MAXIMUM_NAME_REQUESTS_PER_FRAME: usize = 8;

        let now = Instant::now();
        let mut entity_ids = Vec::new();

        for entity in self.client_state.follow_mut(client_state().entities()).iter_mut() {
            if entity_ids.len() >= MAXIMUM_NAME_REQUESTS_PER_FRAME {
                break;
            }

            if !matches!(entity.get_entity_type(), EntityType::Player | EntityType::Npc) {
                continue;
            }

            // `details_need_request` also handles the retry of a lost response
            // and gives up after the attempt budget (with a warning).
            if entity.details_need_request(now) {
                entity_ids.push(entity.get_entity_id());
            }
        }

        for entity_id in entity_ids {
            if self.networking_system.entity_details(entity_id).is_ok()
                && let Some(entity) = self
                    .client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .find(|entity| entity.get_entity_id() == entity_id)
            {
                entity.set_details_requested(now);
            }
        }
    }

    /// Advance the `M` key cycle: small minimap -> big map -> closed.
    ///
    /// The stored [`MapWindowCycle`] is the explicit state of the cycle, but a
    /// map window can also be closed with its X button or by a map change, so
    /// the state is re-synchronized with the actually open windows before the
    /// next step is computed.
    fn cycle_map_windows(interface: &mut Interface<'static, ClientState>, map_window_cycle: &mut MapWindowCycle) {
        let current_state = if interface.is_window_with_class_open(WindowClass::BigMap) {
            MapWindowCycle::BigMap
        } else if interface.is_window_with_class_open(WindowClass::Minimap) {
            MapWindowCycle::Minimap
        } else {
            MapWindowCycle::Closed
        };

        let next_state = match current_state {
            MapWindowCycle::Closed => MapWindowCycle::Minimap,
            MapWindowCycle::Minimap => MapWindowCycle::BigMap,
            MapWindowCycle::BigMap => MapWindowCycle::Closed,
        };

        interface.close_window_with_class(WindowClass::Minimap);
        interface.close_window_with_class(WindowClass::BigMap);

        match next_state {
            MapWindowCycle::Closed => {}
            MapWindowCycle::Minimap => interface.open_window(MinimapWindow),
            MapWindowCycle::BigMap => interface.open_window(BigMapWindow),
        }

        *map_window_cycle = next_state;
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn request_entity_details(&mut self, input_report: &InputReport) {
        let now = Instant::now();

        if let PickerTarget::Entity(entity_id) = input_report.mouse_target
            && let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
            && entity.details_need_request(now)
            && self.networking_system.entity_details(entity_id).is_ok()
        {
            entity.set_details_requested(now);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    /// Spawns a world effect (`.str`) centered on an entity, with an attached
    /// point light.
    ///
    /// A malformed or missing effect file must not crash the client, so errors
    /// are only logged in debug builds.
    fn spawn_entity_effect(
        &mut self,
        effect_path: &str,
        entity_id: EntityId,
        effect_offset: Vector3<f32>,
        scale: f32,
        light_color: Color,
        light_intensity: f32,
    ) {
        let Ok(effect) = self.effect_loader.get_or_load(effect_path, &self.texture_loader) else {
            crate::logging::log_debug!("entity effect: failed to load '{}'", effect_path);
            return;
        };
        crate::logging::log_debug!("entity effect: spawned '{}' on entity {}", effect_path, entity_id.0);
        let frame_timer = effect.new_frame_timer();

        self.effect_holder.add_effect(Box::new(EffectWithLight::new(
            effect,
            frame_timer,
            EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
            effect_offset,
            scale,
            // The point light id needs to be unique. Skill effects use a
            // different id space than the visual effect packets so that both can
            // be active on the same entity at the same time.
            PointLightId::new(entity_id.0.wrapping_add(0x4000_0000)),
            Vector3::new(0.0, 12.0, 0.0),
            light_color,
            light_intensity,
            false,
        )));
    }

    /// Spawns a world effect (`.str`) centered on a world position.
    fn spawn_position_effect(
        &mut self,
        effect_path: &str,
        position: Point3<f32>,
        effect_offset: Vector3<f32>,
        scale: f32,
        light_color: Color,
        light_intensity: f32,
    ) {
        let Ok(effect) = self.effect_loader.get_or_load(effect_path, &self.texture_loader) else {
            crate::logging::log_debug!("position effect: failed to load '{}'", effect_path);
            return;
        };
        crate::logging::log_debug!("position effect: spawned '{}' at {:?} (scale {})", effect_path, position, scale);
        let frame_timer = effect.new_frame_timer();
        let light_id = NEXT_POSITION_EFFECT_LIGHT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        self.effect_holder.add_effect(Box::new(EffectWithLight::new(
            effect,
            frame_timer,
            EffectCenter::Position(position),
            effect_offset,
            scale,
            PointLightId::new(light_id),
            Vector3::new(0.0, 12.0, 0.0),
            light_color,
            light_intensity,
            false,
        )));
    }

    /// Spawns the one-shot teleport/logout disappear effect (warp swirl +
    /// `ef_teleportation.wav`) at the position where an entity vanished.
    ///
    /// Only `Teleported` and `LoggedOut` disappearances use this; out-of-sight
    /// and death disappearances keep their plain fade / death handling.
    /// Loads the classic arrow sprite/actions once, on the first bow attack.
    fn ensure_arrow_assets(&mut self) {
        if self.arrow_assets.is_some() || self.arrow_assets_failed {
            return;
        }

        // Classic clients draw the arrow with the arrow item sprite. The huge
        // `ac_chargearrow` skill-effect sprite is only a last-resort fallback.
        const CANDIDATES: [&str; 4] = ["아이템\\화살", "아이템\\강철화살", "아이템\\독화살", "아이템\\ac_chargearrow"];

        for candidate in CANDIDATES {
            let sprite = self.sprite_loader.get_or_load(&format!("{candidate}.spr"), None);
            let actions = self.action_loader.get_or_load(&format!("{candidate}.act"));

            if let (Ok(sprite), Ok(actions)) = (sprite, actions) {
                crate::logging::log_debug!("arrow projectile: loaded sprite {}", candidate);
                self.arrow_assets = Some(std::sync::Arc::new(ArrowAssets { sprite, actions }));
                return;
            }
        }

        self.arrow_assets_failed = true;
        crate::logging::log_debug!("arrow projectile: could not load an arrow sprite from the game archives");
    }

    fn spawn_teleport_vanish(&mut self, position: Point3<f32>) {
        // Defensive lifetime/rate handling for bursts of population-engine
        // shells: keep at most TELEPORT_VANISH_BURST_LIMIT effects per window.
        self.recent_teleport_vanishes
            .retain(|spawn_time| spawn_time.elapsed() < TELEPORT_VANISH_WINDOW);

        if self.recent_teleport_vanishes.len() >= TELEPORT_VANISH_BURST_LIMIT {
            return;
        }

        let Some(warp_portal) = self.warp_portal.clone() else {
            return;
        };

        self.recent_teleport_vanishes.push(Instant::now());
        self.effect_holder.add_effect(Box::new(TeleportVanish::new(warp_portal, position)));
        // Spatial playback respects the muted/headless backend of the lean test
        // client (KORANGAR_MUTE_AUDIO never opens an audio device).
        self.audio_engine
            .play_spatial_sound_effect(self.teleport_vanish_sound_effect, position, 250.0);
        crate::logging::log_debug!(
            "entity: teleport vanish effect at ({:.1}, {:.1}, {:.1}) (sound ef_teleportation.wav)",
            position.x,
            position.y,
            position.z,
        );
    }

    /// Draws the ground cast circle below an entity for `duration_ms`.
    fn spawn_cast_circle(&mut self, entity_id: EntityId, color: Color, duration_ms: u32) {
        let center = EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0));

        let Some(circle) = CastCircle::new(
            &self.texture_loader,
            center,
            CAST_CIRCLE_RADIUS,
            color,
            (duration_ms as f32 / 1000.0).max(0.2),
        ) else {
            #[cfg(feature = "debug")]
            print_debug!("[{}] failed to load the cast circle textures", "error".red());
            return;
        };

        self.effect_holder.add_effect(Box::new(circle));
    }

    /// Spawns one of the official client's hard-coded skill auras (the yellow
    /// `EF_BEGINSPELL` cone / `EF_GUMGANG2` rings) on an entity.
    ///
    /// The aura is fully procedural and owned by the entity, so it follows the
    /// caster and is removed with it (for example when the entity disappears).
    fn spawn_skill_aura(&mut self, entity_id: EntityId, spec: SkillAuraSpec) {
        let center = EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0));

        let Some(aura) = SkillAura::new(&self.texture_loader, center, spec) else {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] failed to load the skill aura texture '{}'",
                "error".red(),
                spec.texture_path
            );
            return;
        };

        self.effect_holder.add_unit(Box::new(aura), entity_id);
    }

    /// Spawns the official effect layer of `skill_id` for one variant.
    ///
    /// Two data-driven layers are played from the same phase-aware,
    /// de-duplicated spawn point:
    ///
    /// * the `.str` screen sprites from
    ///   `archive/data/client/skill_effect_assets.ron`,
    /// * the procedural primitives (hit flashes, projectiles/bolt rain, ground
    ///   auras) from `archive/data/client/skill_procedural_effects.ron`,
    /// * the animated `.spr`/`.act` sprite effects from
    ///   `archive/data/client/skill_sprite_effects.ron`.
    ///
    /// Every cast spawns each variant at most once, even if several packets
    /// (ground cast, damage, ...) describe the same cast.
    fn spawn_skill_effect_variants(
        &mut self,
        skill_id: SkillId,
        variant: SkillEffectVariant,
        source_entity_id: EntityId,
        world_position: Option<Point3<f32>>,
        client_tick: ClientTick,
    ) {
        if crate::world::has_custom_completion(skill_id) {
            return;
        }

        let effect_entries: Vec<(&'static str, crate::world::EffectAnchor)> =
            crate::world::skill_effect_entries(skill_id, variant).collect();
        let procedural = crate::world::skill_procedural_effects(skill_id, variant);
        let sprite_effects = crate::world::skill_sprite_effects(skill_id, variant);

        // All layers share one de-duplication key per cast phase.
        if (!effect_entries.is_empty() || !procedural.is_empty() || !sprite_effects.is_empty())
            && !self
                .skill_completion_tracker
                .claim(source_entity_id, skill_id, variant.phase(), client_tick)
        {
            return;
        }

        // A phase can have several `.str` layers (e.g. two hit flashes). The
        // data file records the spawn anchor of every layer so on-caster effects
        // (`effectIdOnCaster`, e.g. Pierce) are not spawned at the damaged
        // target. Effects aimed at the target cell fall back to the caster when
        // the packet does not carry a position (no-damage self casts).
        for (effect_path, anchor) in effect_entries {
            crate::logging::log_debug!(
                "skill effect: spawned '{}' for skill {} ({:?}, anchor {:?})",
                effect_path,
                skill_id.0,
                variant,
                anchor
            );

            let position = match anchor {
                crate::world::EffectAnchor::Caster => None,
                crate::world::EffectAnchor::Target | crate::world::EffectAnchor::Travel | crate::world::EffectAnchor::Auto => {
                    world_position
                }
            };

            match position {
                Some(world_position) => {
                    self.spawn_position_effect(effect_path, world_position, Vector3::new(0.0, 0.0, 0.0), 1.0, Color::WHITE, 0.0);
                }
                None => {
                    self.spawn_entity_effect(
                        effect_path,
                        source_entity_id,
                        Vector3::new(0.0, 0.0, 0.0),
                        1.0,
                        Color::WHITE,
                        0.0,
                    );
                }
            }
        }

        if !procedural.is_empty() {
            self.spawn_procedural_effects(skill_id, variant, source_entity_id, world_position, procedural);
        }

        if !sprite_effects.is_empty() {
            self.spawn_skill_sprite_effects(skill_id, variant, source_entity_id, world_position, sprite_effects);
        }

        // Storm Gust's snow is a `.str` file, but the dark storm cloud above the
        // cell is a hard-coded official client effect (cloud1..7.tga), so it is
        // reproduced procedurally once per cast.
        if matches!(skill_id.0, 87 | 89)
            && let (SkillEffectVariant::Success, Some(world_position)) = (variant, world_position)
            && self
                .skill_completion_tracker
                .claim(source_entity_id, skill_id, STORM_CLOUD_PHASE, client_tick)
            && let Some(cloud) = StormCloud::new(&self.texture_loader, world_position)
        {
            crate::logging::log_debug!("skill effect: spawned procedural storm cloud for skill {}", skill_id.0);
            self.effect_holder.add_unit(Box::new(cloud), source_entity_id);
        }
    }

    /// Spawns the procedural primitives of `skill_id` for one variant.
    ///
    /// The rows come from the generated data file (see
    /// `archive/data/client/skill_procedural_effects.ron`). The anchor of a row
    /// decides whether it is attached to the caster entity (and follows it) or
    /// placed at the target cell.
    fn spawn_procedural_effects(
        &mut self,
        skill_id: SkillId,
        variant: SkillEffectVariant,
        source_entity_id: EntityId,
        target_position: Option<Point3<f32>>,
        effects: &'static [ProceduralEffectSpec],
    ) {
        let caster_position = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == source_entity_id)
            .map(|entity| entity.get_position());

        for effect in effects {
            match effect.primitive {
                ProceduralPrimitive::HitFlash => {
                    let Some(center) = procedural_effect_center(effect.anchor, source_entity_id, target_position) else {
                        continue;
                    };
                    let Some(flash) = HitFlash::spawn(&self.texture_loader, center, effect) else {
                        continue;
                    };

                    crate::logging::log_debug!(
                        "skill effect: spawned procedural hit flash '{}' for skill {} ({:?})",
                        effect.texture,
                        skill_id.0,
                        variant
                    );

                    match effect.anchor {
                        ProceduralAnchor::Caster => self.effect_holder.add_unit(Box::new(flash), source_entity_id),
                        _ => self.effect_holder.add_effect(Box::new(flash)),
                    }
                }
                ProceduralPrimitive::Projectile => {
                    let Some(target) = target_position else {
                        continue;
                    };
                    let (mode, start) = match effect.anchor {
                        ProceduralAnchor::Rain => (ProjectileMode::Rain, target),
                        _ => {
                            let Some(caster) = caster_position else {
                                continue;
                            };
                            (ProjectileMode::Travel, caster)
                        }
                    };

                    if let Some(projectile) = Projectile::spawn(&self.texture_loader, effect, mode, start, target) {
                        crate::logging::log_debug!(
                            "skill effect: spawned procedural projectile '{}' for skill {} ({:?})",
                            effect.texture,
                            skill_id.0,
                            variant
                        );
                        self.effect_holder.add_effect(Box::new(projectile));
                    }
                }
                ProceduralPrimitive::GroundAura => {
                    let Some(center) = procedural_effect_center(effect.anchor, source_entity_id, target_position) else {
                        continue;
                    };
                    let Some(aura) = spawn_ground_aura(&self.texture_loader, center, effect) else {
                        continue;
                    };

                    crate::logging::log_debug!(
                        "skill effect: spawned procedural ground aura '{}' for skill {} ({:?})",
                        effect.texture,
                        skill_id.0,
                        variant
                    );

                    match effect.anchor {
                        ProceduralAnchor::Caster => self.effect_holder.add_unit(Box::new(aura), source_entity_id),
                        _ => self.effect_holder.add_effect(Box::new(aura)),
                    }
                }
            }
        }
    }

    /// Spawns the animated sprite effects (`.spr` + `.act`) of `skill_id` for
    /// one variant.
    ///
    /// The rows come from `archive/data/client/skill_sprite_effects.ron`. The
    /// anchor of a row decides whether the sprite is attached to the caster
    /// entity (and follows it), placed at the target, or played on the ground
    /// cell; missing files spawn nothing and leave the generic cast circle in
    /// place.
    fn spawn_skill_sprite_effects(
        &mut self,
        skill_id: SkillId,
        variant: SkillEffectVariant,
        source_entity_id: EntityId,
        target_position: Option<Point3<f32>>,
        effects: &'static [SpriteEffectSpec],
    ) {
        for effect in effects {
            let center = match effect.anchor {
                SpriteEffectAnchor::Caster => EffectCenter::Entity(source_entity_id, Point3::new(0.0, 0.0, 0.0)),
                SpriteEffectAnchor::Target | SpriteEffectAnchor::Ground => match target_position {
                    Some(target_position) => EffectCenter::Position(target_position),
                    None => EffectCenter::Entity(source_entity_id, Point3::new(0.0, 0.0, 0.0)),
                },
            };

            let Some(sprite_effect) = SpriteEffect::spawn(&self.sprite_loader, &self.action_loader, center, effect) else {
                crate::logging::log_debug!(
                    "skill effect: no sprite effect asset could be loaded for skill {} ({:?})",
                    skill_id.0,
                    variant
                );
                continue;
            };

            crate::logging::log_debug!(
                "skill effect: spawned sprite effect '{}' for skill {} ({:?}, anchor {:?})",
                effect.files.first().map(String::as_str).unwrap_or("<none>"),
                skill_id.0,
                variant,
                effect.anchor
            );

            self.effect_holder.add_effect(Box::new(sprite_effect));
        }
    }

    /// Applies an aura style selected through `/aura` or by the server-side
    /// aura stylist NPC (via the private chat marker) and persists it
    /// immediately.
    ///
    /// The running aura effects read the style from a shared atomic cell that
    /// is refreshed from the interface settings every frame, so the change
    /// is visible on the next frame without recreating the effects.
    fn set_aura_style(&mut self, style: AuraStyle) {
        let style_path = client_state().interface_settings().level_99_aura_style();
        *self.client_state.follow_mut(style_path) = style;

        self.client_state.follow(client_state().interface_settings()).save();

        self.push_chat_information(format!("Aura style set to \"{}\".", style.style_name()));
    }

    /// Shows the aggregated experience gain in the chat once the packet burst
    /// is over.
    fn flush_experience_feedback(&mut self) {
        let Some(last_packet) = self.pending_experience.last_packet else {
            return;
        };

        if last_packet.elapsed() < EXPERIENCE_FEEDBACK_DELAY {
            return;
        }

        if let Some(message) = self.pending_experience.take_message() {
            self.push_chat_information(message);
        }
    }

    /// Pushes an informational message into the chat window.
    fn push_chat_information(&mut self, message: String) {
        self.client_state
            .follow_mut(client_state().chat_messages())
            .push(ChatMessage::new(message, MessageColor::Information));
    }

    /// Spawns the classic level-99 aura below player entities whose base level
    /// is at least 99 and removes auras that are no longer valid.
    ///
    /// The aura itself re-checks the entity on every update (despawn, death,
    /// level drop, configured style), so this only has to create the effect
    /// once per entity. Entities whose base level is unknown (NPCs, population
    /// shells that are only sent via a spawn packet) are skipped gracefully.
    /// Adds learned skills that the character's (possibly fallback) job skill
    /// tree does not contain to a synthetic "Learned" tab.
    ///
    /// High/expanded job ids map to a fallback base job tree (or have a trimmed
    /// tree in this data set), so skills the server reports as learned can be
    /// missing from whatever tree the client resolved. The tree only iterates
    /// its layout skills, so without this those skills would be invisible
    /// even though they are learned (use/drag/right-click never had a slot
    /// to interact with).
    fn merge_learned_skills_into_skill_tree(&mut self, client_tick: ClientTick) {
        let learned_skill_ids: Vec<SkillId> = self
            .client_state
            .follow(client_state().skill_tree().skills())
            .iter()
            .map(|learned_skill| learned_skill.skill_id)
            .collect();

        if learned_skill_ids.is_empty() {
            return;
        }

        let Some(job_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_job_id()) else {
            return;
        };

        let existing_skill_ids: std::collections::HashSet<SkillId> = self
            .client_state
            .follow(client_state().skill_tree().layout())
            .tabs
            .iter()
            .flat_map(|tab| tab.skills.values().map(|skill| skill.skill_id))
            .collect();

        let missing_skill_ids: Vec<SkillId> = learned_skill_ids
            .into_iter()
            .filter(|skill_id| !existing_skill_ids.contains(skill_id))
            .collect();

        if missing_skill_ids.is_empty() {
            return;
        }

        let mut learned_tab = crate::state::skills::SkillTabLayout {
            name: "Learned".to_owned(),
            skills: Default::default(),
        };

        for (slot, skill_id) in missing_skill_ids.into_iter().enumerate() {
            learned_tab.skills.insert(
                slot,
                self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick),
            );
        }

        let layout = self.client_state.follow_mut(client_state().skill_tree().layout());
        layout.tabs.push(learned_tab);

        // Make the tab visible immediately: the tree shows the selected tab, so the
        // player does not have to guess where the missing skills went.
        let selected_tab = layout.tabs.len().saturating_sub(1);
        *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = selected_tab;
    }

    /// Shows the name of a used skill above the caster's head (`Blessing !!`).
    ///
    /// Called once per cast (never per animation frame). The label replaces a
    /// previous label of the same entity, so recasts refresh instead of
    /// stacking.
    /// Starts the local player's cast bar for a server-confirmed cast with a
    /// cast time.
    ///
    /// Only the local player gets a cast bar. Called from the cast-start packet
    /// handler, so a rejected/failed cast never shows one.
    fn start_own_cast_bar(&mut self, source_entity_id: EntityId, skill_id: SkillId, cast_time: u32, client_tick: ClientTick) {
        if cast_time == 0 {
            return;
        }

        let own_entity_id = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id());

        if own_entity_id != Some(source_entity_id) {
            return;
        }

        let skill_name = self.library.get::<SkillListInformation>(skill_id).name.clone();

        self.cast_bars.start(source_entity_id, skill_name, cast_time, client_tick);
    }

    /// Plays the mapped skill sound once and remembers it for
    /// [`Self::take_duplicate_cast_sound`].
    ///
    /// The 2019 server sends `ZC_USESKILL_ACK` (0x0B1A) at the start of every
    /// skill use *and* `ZC_USE_SKILL` (0x09CB) when the skill completes. For
    /// instant skills both packets arrive almost at the same time, so the
    /// completion handler de-duplicates the start sound instead of playing the
    /// same wav twice.
    fn play_cast_sound(&mut self, entity_id: EntityId, skill_id: SkillId, position: Option<cgmath::Point3<f32>>, client_tick: ClientTick) {
        self.skill_sounds.play_skill_sound(skill_id, Some(entity_id), position);
        self.recent_cast_sounds
            .retain(|_, (_, tick)| client_tick.0.wrapping_sub(tick.0) < CAST_SOUND_DEDUPE_TIMEOUT_MS);
        self.recent_cast_sounds.insert(entity_id, (skill_id, client_tick));
    }

    /// Returns `true` (and forgets the entry) when the completion packet is the
    /// duplicate of a sound that was already played for this skill use.
    fn take_duplicate_cast_sound(&mut self, entity_id: EntityId, skill_id: SkillId, client_tick: ClientTick) -> bool {
        match self.recent_cast_sounds.get(&entity_id) {
            Some(&(started_skill_id, started_tick))
                if started_skill_id == skill_id && client_tick.0.wrapping_sub(started_tick.0) < CAST_SOUND_DEDUPE_MS =>
            {
                self.recent_cast_sounds.remove(&entity_id);
                true
            }
            _ => false,
        }
    }

    /// Whether the local player's movement is currently blocked by a
    /// server-confirmed cast with a cast time.
    ///
    /// Only the cast bar state is used: instant skills never start a bar (and
    /// therefore never root the player), and a cancelled or completed cast
    /// clears the bar, which releases the movement block immediately. Dead or
    /// sitting characters are never rooted by a stale bar.
    fn movement_blocked_by_cast(&self, client_tick: ClientTick) -> bool {
        let Some(player) = self.client_state.try_follow(this_entity()) else {
            return false;
        };

        if player.is_dead() || player.is_sitting() {
            return false;
        }

        if !self.cast_bars.is_casting(client_tick) {
            return false;
        }

        // Sage/Scholar/Professor with the `Free Cast` passive can move while
        // casting. Unknown jobs default to "blocked".
        if has_free_cast(
            player.get_job_id(),
            self.client_state.follow(client_state().skill_tree().skills()),
        ) {
            return false;
        }

        crate::logging::log_debug!(
            "movement blocked: local player is casting (job={} free_cast=false)",
            player.get_job_id().0
        );
        true
    }

    /// Whether the local player's after-cast act delay is still running.
    ///
    /// The after-cast act delay only paces follow-up *actions* (skills/items);
    /// it deliberately does not block movement, which is controlled by the
    /// server and by the cast bar rooting above.
    fn aftercast_act_active(&self, client_tick: ClientTick) -> bool {
        self.aftercast_act_until.is_some_and(|until| {
            // `until` is in the future while `until - now` is less than half of
            // the wrapping u32 range.
            until.0.wrapping_sub(client_tick.0) < 0x8000_0000
        })
    }

    /// Start the after-cast act delay of a completed skill, using the per-skill
    /// value from the generated rAthena table.
    fn apply_aftercast_act_delay(&mut self, skill_id: SkillId, level: u16, client_tick: ClientTick) {
        let Some(delays) = skill_delays(skill_id, level) else {
            return;
        };

        if delays.aftercast_act_ms == 0 {
            return;
        }

        self.aftercast_act_until = Some(ClientTick(client_tick.0.wrapping_add(delays.aftercast_act_ms)));

        crate::logging::log_debug!(
            "aftercast act delay: skill={} level={} {}ms (server table)",
            skill_id.0,
            level,
            delays.aftercast_act_ms
        );
    }

    fn show_skill_name_text(&mut self, entity_id: EntityId, skill_id: SkillId, client_tick: ClientTick) {
        let is_character = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .any(|entity| entity.get_entity_id() == entity_id && entity.get_entity_type() == EntityType::Player);

        if !is_character {
            crate::logging::log_debug!("skill name text: entity {:?} is not a character, skipping", entity_id);
            return;
        }

        let skill_name = self.library.get::<SkillListInformation>(skill_id).name.clone();
        let text = format!("{skill_name} !!");

        crate::logging::log_debug!("skill name text: entity {:?} skill {:?} -> {:?}", entity_id, skill_id, text);

        self.skill_name_texts.push(entity_id, text, client_tick);
    }

    fn update_level_99_auras(&mut self) {
        let style = *self.client_state.follow(client_state().interface_settings().level_99_aura_style());

        self.level_99_aura_style_cell.store(style.encode(), Ordering::Relaxed);

        if style == AuraStyle::Off {
            for entity_id in self.level_99_aura_entities.drain() {
                self.effect_holder.remove_unit(entity_id);
            }

            return;
        }

        let mut valid_entities = HashSet::new();

        let player_ids: Vec<(EntityId, Point3<f32>, usize, bool)> = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .filter(|entity| !entity.is_dead())
            .filter_map(|entity| {
                let level = entity.get_base_level()?;

                (level >= crate::world::AURA_MINIMUM_LEVEL).then(|| {
                    (
                        entity.get_entity_id(),
                        entity.get_position(),
                        level,
                        entity.is_transcendent_job(),
                    )
                })
            })
            .collect();

        for (entity_id, position, level, transcendent) in player_ids {
            valid_entities.insert(entity_id);

            if self.level_99_aura_entities.contains(&entity_id) {
                continue;
            }

            let Some(mut aura) = crate::world::Aura::new(self.level_99_aura_style_cell.clone(), level, transcendent, &self.texture_loader)
            else {
                continue;
            };

            crate::logging::log_debug!(
                "level-99 aura: created for entity {:?} (level {}, transcendent {})",
                entity_id,
                level,
                transcendent
            );
            aura.attach_to_entity(entity_id, position);
            self.effect_holder.add_unit(Box::new(aura), entity_id);
            self.level_99_aura_entities.insert(entity_id);
        }

        // Forget auras of entities that are no longer valid (despawned, dead or
        // below the level threshold). The effect removes itself on its next
        // update; removing the id here only makes sure it is re-created when
        // the entity becomes eligible again.
        let stale = self
            .level_99_aura_entities
            .iter()
            .filter(|entity_id| !valid_entities.contains(entity_id))
            .copied()
            .collect::<Vec<_>>();
        for entity_id in stale {
            self.level_99_aura_entities.remove(&entity_id);
        }
    }

    /// Updates the local status effect timers and shows/hides the buff bar.
    fn update_status_effects(&mut self, client_tick: ClientTick) {
        self.client_state.follow_mut(client_state().minimap()).prune_markers(client_tick.0);
        self.client_state.follow_mut(client_state().skill_cooldowns()).prune(client_tick);

        {
            let status_effects = self.client_state.follow_mut(client_state().status_effects());
            status_effects.set_client_tick(client_tick.0 as u64);
            status_effects.update(client_tick.0 as u64);
        }

        // The status effect state tracks all entities in the view, but the buff
        // bar is only shown for the player's own effects.
        let own_entity_id = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id().0);
        let is_empty = match own_entity_id {
            Some(entity_id) => self.client_state.follow(client_state().status_effects()).is_empty_for(entity_id),
            None => true,
        };
        let is_open = self.interface.is_window_with_class_open(WindowClass::BuffBar);

        match (is_empty, is_open) {
            (false, false) => self.open_buff_bar_window(),
            (true, true) => self.interface.close_window_with_class(WindowClass::BuffBar),
            _ => {}
        }
    }

    /// Shows the spirit sphere counter while the local player has spheres and
    /// hides it again when they are consumed ("Critical Explosion"/Fury uses
    /// all five). The overlay is a background HUD element like the buff bar.
    fn update_spirit_spheres(&mut self) {
        let own_spheres = match self.client_state.try_follow(this_entity()) {
            Some(entity) => {
                let entity_id = entity.get_entity_id();
                self.client_state.follow(client_state().spirit_orbs()).get(entity_id)
            }
            None => 0,
        };

        let is_open = self.interface.is_window_with_class_open(WindowClass::SpiritSpheres);

        match (own_spheres <= 0, is_open) {
            (false, false) => self.interface.open_window(SpiritSphereWindow),
            (true, true) => self.interface.close_window_with_class(WindowClass::SpiritSpheres),
            _ => {}
        }
    }

    fn handle_network_events(&mut self, client_tick: ClientTick) {
        self.networking_system.get_events(&mut self.network_event_buffer);

        // Collect the events first so handlers are free to take `&mut self`
        // (the drain iterator borrows the event buffer).
        let network_events: Vec<NetworkEvent> = self.network_event_buffer.drain().collect();

        if !network_events.is_empty() {
            // Network events change UI state (chat, inventory, stats, ...).
            self.interface.invalidate_layouts();
        }

        for event in network_events {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    // Remove `_m`/`_f` suffix from the username. The suffix is only for *creating*
                    // an account and thus can (and needs to) be removed after the first successful
                    // login.
                    {
                        let selected_service_path =
                            SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                        let username_path = selected_service_path.username();

                        let username = self.client_state.follow_mut(username_path);

                        if let Some(stripped) = username.strip_suffix("_m") {
                            *username = stripped.to_owned();
                        } else if let Some(stripped) = username.strip_suffix("_f") {
                            *username = stripped.to_owned();
                        }
                    }

                    self.saved_login_data = Some(login_data);
                    self.session.transition(SessionEvent::LoginSucceeded);

                    // A previously established character/map connection (e.g. from a session
                    // that was kicked or logged out) must not block the new server selection.
                    // Only disconnect connections that actually exist: disconnecting an idle
                    // connection emits a synthetic "closed by client" event, which would make
                    // the disconnect handler close the server selection window we are about to
                    // open and bounce the player back to the login screen.
                    if self.networking_system.is_character_server_connected() {
                        self.networking_system.disconnect_from_character_server();
                    }

                    if self.networking_system.is_map_server_connected() {
                        self.networking_system.disconnect_from_map_server();
                    }

                    // A fresh login must not auto-reconnect to a previously selected character
                    // server (map-restart recovery state): reset the retry state and the
                    // remembered server so the Select Server window is shown and only the
                    // player's click connects.
                    self.session.character_server_disconnected();
                    self.session.clear_character_reconnect();
                    self.saved_character_server = None;

                    *self.client_state.follow_mut(client_state().character_servers()) = character_servers;

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.interface
                        .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_login_server();

                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::InternalError { message } => {
                    // A client build/setup bug (e.g. duplicate packet handler).
                    // Log it as an error and show the message, never panic.
                    crate::logging::log_error!("internal client error: {message}");
                    self.interface.open_window(ErrorWindow::new(message));
                }
                NetworkEvent::LoginServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let Some(socket_address) = self.saved_login_server_address else {
                            // No address means the user never completed the login
                            // flow; nothing to reconnect to.
                            continue;
                        };
                        self.networking_system.connect_to_login_server(
                            self.saved_packet_version,
                            socket_address,
                            &self.saved_username,
                            &self.saved_password,
                        );
                    }
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    crate::logging::log_debug!(
                        "character server: connected ({} slots), requesting the character list",
                        normal_slot_count
                    );
                    self.session.character_server_connected();
                    if self.session.state() == SessionState::Recovering {
                        self.session.transition(SessionEvent::CharacterServerConnected);
                    }

                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_slot_count(normal_slot_count);

                    let _ = self.networking_system.request_character_list();
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    self.session.character_server_disconnected();
                    self.networking_system.disconnect_from_character_server();

                    if self.map.is_none() && self.saved_login_data.is_some() {
                        // The character server can be briefly unavailable (e.g. while the map
                        // server restarts and the old map session is still registered).
                        // Retry instead of ending up on a dead screen.
                        self.session.schedule_character_reconnect(Instant::now());
                    } else {
                        self.interface.open_window(ErrorWindow::new(message.to_owned()));
                    }
                }
                NetworkEvent::CharacterServerDisconnected { reason } => {
                    self.session.character_server_disconnected();

                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        self.session.schedule_character_reconnect(Instant::now());
                    } else if !self.networking_system.is_map_server_connected()
                        && !self.networking_system.is_login_server_connected()
                        && !self.interface.is_window_with_class_open(WindowClass::SelectServer)
                    {
                        // Only fall back to the login screen when the character server closed
                        // while we are not in the middle of the login/server-selection flow.
                        // During login the character server connection is intentionally dropped
                        // (or never existed); a "closed by client" event must not wipe the
                        // server selection window that was just opened.
                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(LoginWindow::new(
                            client_state().login_window(),
                            client_state().login_settings(),
                            client_state().client_info(),
                        ));

                        self.audio_engine.play_background_music_track(UI_BACKGROUND_MUSIC);
                    }
                }
                NetworkEvent::MapServerDisconnected { reason } => {
                    // Guild emblems are only valid for the map session; the server may
                    // have changed them and the rate limit should be reset as well.
                    self.guild_emblems.clear();
                    // Status effects belong to the map session as well.
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.client_state.follow_mut(client_state().spirit_orbs()).clear();

                    if self.session.state() == SessionState::Shutdown {
                        // We are quitting; the clean logout is done, don't go back to
                        // the character selection screen.
                        continue;
                    }

                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the map server with error");
                    }

                    self.session.map_server_disconnected();
                    self.return_to_character_selection();
                }
                NetworkEvent::InitialStats {
                    strength_stat_points_cost,
                    agility_stat_points_cost,
                    vitality_stat_points_cost,
                    intelligence_stat_points_cost,
                    dexterity_stat_points_cost,
                    luck_stat_points_cost,
                    attack,
                    bonus_attack,
                    magic_attack_max,
                    magic_attack_min,
                    defense,
                    bonus_defense,
                    magic_defense,
                    bonus_magic_defense,
                    hit,
                    flee,
                    perfect_dodge,
                    critical,
                    attack_speed,
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.strength_stat_points_cost = strength_stat_points_cost;
                        player.agility_stat_points_cost = agility_stat_points_cost;
                        player.vitality_stat_points_cost = vitality_stat_points_cost;
                        player.intelligence_stat_points_cost = intelligence_stat_points_cost;
                        player.dexterity_stat_points_cost = dexterity_stat_points_cost;
                        player.luck_stat_points_cost = luck_stat_points_cost;
                        player.attack = attack as u32;
                        player.bonus_attack = bonus_attack as u32;
                        player.magic_attack_max = magic_attack_max as u32;
                        player.magic_attack_min = magic_attack_min as u32;
                        player.defense = defense as u32;
                        player.bonus_defense = bonus_defense as u32;
                        player.magic_defense = magic_defense as u32;
                        player.bonus_magic_defense = bonus_magic_defense as u32;
                        player.hit = hit as u32;
                        player.flee = flee as u32;
                        player.perfect_dodge = perfect_dodge as u32;
                        player.critical = critical as u32;
                        player.attack_speed = attack_speed as u32;
                    }
                }
                NetworkEvent::ResurrectPlayer { entity_id } => {
                    self.entity_lifecycle.on_resurrect(entity_id);

                    // The server only revives the character, it does not send a separate
                    // stand-up packet, so restore the idle animation ourselves. Without this
                    // the player stayed in the dead pose after respawning.
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_idle(client_tick);
                    }

                    // If the resurrected player is us, close the resurrect window.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == entity_id)
                    {
                        self.interface.close_window_with_class(WindowClass::Respawn);
                    }
                }
                NetworkEvent::PlayerStandUp { entity_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_standing(client_tick);
                    }
                }
                NetworkEvent::PlayerSitDown { entity_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_sitting(client_tick);
                    }
                }
                NetworkEvent::SkillCast {
                    source_entity_id,
                    destination_entity_id,
                    position,
                    skill_id,
                    element,
                    cast_time,
                    attack_motion_time,
                } => {
                    // Other clients show the caster's skill-cast action for the cast
                    // duration and a ground cast circle below the caster. The 2019
                    // server does not send the animation separately, so this is the main
                    // cue for ambient self-buffs, crafting, warp portal casts and the
                    // skills of other players.
                    let has_cast_time = cast_time > 0;
                    // The server packet's cast time is authoritative. The
                    // per-skill table only provides a lower bound for the
                    // animation/pose duration when the packet has no usable
                    // timing (the old code used one global 300 ms constant).
                    let table_delays = skill_delays(skill_id, 1);
                    let animation_minimum = table_delays
                        .map(|delays| delays.total_cast_ms().max(delays.aftercast_act_ms))
                        .unwrap_or(0);
                    let cast_animation_time = cast_time.max(attack_motion_time).max(animation_minimum);
                    let visuals = skill_visuals(skill_id, element);

                    if let Some(delays) = table_delays {
                        crate::logging::log_debug!(
                            "skill delay table: skill={} cast={}ms fixed={}ms aftercast_act={}ms aftercast_walk={}ms cooldown={}ms",
                            skill_id.0,
                            delays.cast_ms,
                            delays.fixed_cast_ms,
                            delays.aftercast_act_ms,
                            delays.aftercast_walk_ms,
                            delays.cooldown_ms
                        );
                    }

                    let source_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                        .map(|entity| entity.get_position());

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        match caster_action(skill_id) {
                            // Stance/buff skills (Zen, Fury, Endure, ...) keep the idle
                            // pose in the official client; the hard-coded skill effect
                            // is the visual feedback instead of a casting gesture.
                            CasterAction::Idle => entity.set_idle(client_tick),
                            // Casts with a cast time loop the casting gesture until the
                            // cast finishes; instant casts play the action once.
                            CasterAction::Skill => entity.set_skill_cast(cast_animation_time, has_cast_time, client_tick),
                            // Attack skills play the attack animation (like the
                            // official client's `ATTACK` skill action).
                            CasterAction::Attack => entity.set_attack(cast_animation_time, false, client_tick),
                            // Pickup-style skills play the pickup animation.
                            CasterAction::Pickup => entity.set_pickup(client_tick),
                            CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                        }

                        // Hard-coded body flash of the skill (for example the yellow
                        // `EF_PORTAL5` flash of Zen).
                        if let Some(flash) = &visuals.body_flash {
                            entity.start_body_flash(flash.color, flash.hold_ms, flash.fade_ms, client_tick);
                        }

                        // Targeted casts face their target, self-casts keep the current
                        // direction.
                        if destination_entity_id != source_entity_id {
                            entity.rotate_towards(position);
                        }
                    }

                    // Hard-coded cylinder aura of the official client (`EF_BEGINSPELL`
                    // yellow cone for Zen and Fury).
                    if let Some(aura) = visuals.cast_aura {
                        self.spawn_skill_aura(source_entity_id, aura);
                    }

                    // Show the skill name above the caster's head, like the original
                    // client (own character, other players and population shells).
                    self.show_skill_name_text(source_entity_id, skill_id, client_tick);

                    // Official cast-start effect of the mapping (e.g. a caster
                    // effect that plays while the skill is being cast).
                    self.spawn_skill_effect_variants(skill_id, SkillEffectVariant::Cast, source_entity_id, None, client_tick);

                    // Local player cast bars are driven by this server-confirmed
                    // cast-start packet (cast_time > 0), never by local requests.
                    self.start_own_cast_bar(source_entity_id, skill_id, cast_time, client_tick);

                    if has_cast_time {
                        // The circle has to stay for the full cast: the cast time can be
                        // shorter than one ACT cycle (the animation then plays a full
                        // cycle) and it can be longer than the animation (the animation
                        // loops), so use the longer of both.
                        let animation_duration = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| entity.skill_animation_duration_ms())
                            .unwrap_or(0);
                        let circle_duration = cast_animation_time.max(animation_duration);

                        self.spawn_cast_circle(source_entity_id, visuals.cast_circle_color, circle_duration);
                    }

                    // The 2019 server sends no sound packet for most ambient casts, so
                    // play the mapped skill sound here. Skills with a cast time play
                    // their sound once when the cast *completes* (the server sends the
                    // completion packet then): playing it here as well made every cast
                    // with a cast bar audible twice. Instant casts have no cast bar and
                    // play their single sound now; the completion packet they also
                    // receive is de-duplicated in `SkillNoDamage`.
                    if !has_cast_time {
                        self.play_cast_sound(source_entity_id, skill_id, source_position, client_tick);
                    }
                }
                NetworkEvent::SkillNoDamage {
                    source_entity_id,
                    // The destination is used to place target-anchored success
                    // effects (Heal, Resurrection, ...) at the ally that received
                    // the skill instead of on the caster.
                    destination_entity_id,
                    skill_id,
                    level,
                    success,
                } => {
                    // 2019 `ZC_USE_SKILL` (0x09CB): the caster performed an
                    // instant/complete skill. Play the full cast animation once and the
                    // completion effect, so self-buffs and the blacksmith crafting loop
                    // become visible.
                    if success {
                        if self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(source_entity_id) {
                            self.cast_bars.clear();
                            // Pace the next action with this skill's server-side
                            // after-cast act delay instead of a global constant.
                            if let Some(delays) = skill_delays(skill_id, level as u16) {
                                crate::logging::log_debug!(
                                    "skill delay table: skill={} level={} cast={}ms fixed={}ms aftercast_act={}ms aftercast_walk={}ms \
                                     cooldown={}ms",
                                    skill_id.0,
                                    level,
                                    delays.cast_ms,
                                    delays.fixed_cast_ms,
                                    delays.aftercast_act_ms,
                                    delays.aftercast_walk_ms,
                                    delays.cooldown_ms
                                );
                            }
                            self.apply_aftercast_act_delay(skill_id, level as u16, client_tick);
                        }

                        let source_position = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| entity.get_position());

                        // World position of the skill target, when the packet names
                        // one and it is not the caster. Target/`Auto` anchored
                        // success effects (for example the Heal sparkles) play
                        // there; without it they fall back to the caster.
                        let destination_position = if destination_entity_id == source_entity_id {
                            None
                        } else {
                            self.client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == destination_entity_id)
                                .map(|entity| entity.get_tile_position())
                                .and_then(|tile_position| self.map.as_ref().and_then(|map| map.get_world_position(tile_position)))
                        };

                        let visuals = skill_visuals(skill_id, 0);

                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                        {
                            match caster_action(skill_id) {
                                // Stance/buff skills keep the idle pose (see the
                                // `SkillCast` arm above).
                                CasterAction::Idle => entity.set_idle(client_tick),
                                // This packet has no cast time: play the whole ACT skill
                                // animation once at its natural frame speed.
                                CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                                CasterAction::Attack => entity.set_attack(INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                                CasterAction::Pickup => entity.set_pickup(client_tick),
                                CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                            }

                            // Instant-only skills (no cast-time packet before this one)
                            // flash their caster here; cast-time skills already flashed
                            // when the cast started.
                            if visuals.cast_aura.is_none()
                                && let Some(flash) = &visuals.body_flash
                            {
                                entity.start_body_flash(flash.color, flash.hold_ms, flash.fade_ms, client_tick);
                            }
                        }

                        for aura in visuals.instant_auras.iter().copied() {
                            self.spawn_skill_aura(source_entity_id, aura);
                        }

                        self.show_skill_name_text(source_entity_id, skill_id, client_tick);

                        // Buff visuals (for example the Blessing angel) are screen
                        // sprites anchored above the head.
                        if let Some(effect) = &visuals.completion
                            && self
                                .skill_completion_tracker
                                .claim(source_entity_id, skill_id, CUSTOM_COMPLETION_PHASE, client_tick)
                        {
                            self.spawn_entity_effect(
                                effect.path,
                                source_entity_id,
                                effect.offset,
                                effect.scale,
                                visuals.light.0,
                                visuals.light.1,
                            );
                        }

                        // Official completion effect from the data-file mapping
                        // (skipped for skills with a hand-written visual).
                        self.spawn_skill_effect_variants(
                            skill_id,
                            SkillEffectVariant::Success,
                            source_entity_id,
                            destination_position,
                            client_tick,
                        );

                        // Instant/self-buff casts (0x09CB) have no sound packet on
                        // the 2019 server either. For an instant skill the start-of-use
                        // packet already played this sound; do not play it twice.
                        if !self.take_duplicate_cast_sound(source_entity_id, skill_id, client_tick) {
                            self.skill_sounds
                                .play_skill_sound(skill_id, Some(source_entity_id), source_position);
                        }
                    }

                    // `ZC_USE_SKILL` (0x09CB) is sent for every no-damage skill
                    // and its `level` field is the skill
                    // level, not a healed amount. It
                    // must not be rendered as a heal number (Blessing/AGI and
                    // other self-buffs showed a green "10"
                    // above the caster). Real heal
                    // numbers come from `ZC_RECOVERY`/0x0A27
                    // (`NetworkEvent::HealEffect`).
                }
                NetworkEvent::SkillCastCancelled { entity_id } => {
                    // `ZC_DISPEL` (0x01B9, rAthena `clif_skillcastcancel`): the
                    // cast of this entity was interrupted (movement, damage,
                    // another skill, ...). Stop the looping cast gesture and
                    // clear the local player's cast bar and skill label so a
                    // cancelled cast never leaves a stuck bar or animation.
                    crate::logging::log_debug!("skill cast cancelled (ZC_DISPEL / 0x01B9): entity={}", entity_id.0);

                    if self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(entity_id) {
                        self.cast_bars.clear();
                        self.skill_name_texts.remove_entity(entity_id);
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.cancel_skill_cast(client_tick);
                    }
                }
                NetworkEvent::SkillGroundCast {
                    entity_id,
                    skill_id,
                    level: _,
                    position,
                } => {
                    // Ground-skill pose packet: the ambient crafting loop uses it as a
                    // fall back so at least the hammer swing is visible and audible.
                    // Play the action once and the completion effect at the target cell.
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        match caster_action(skill_id) {
                            CasterAction::Idle => entity.set_idle(client_tick),
                            CasterAction::Skill => entity.set_skill_cast(0, false, client_tick),
                            CasterAction::Attack => entity.set_attack(INSTANT_SKILL_ANIMATION_MS, false, client_tick),
                            CasterAction::Pickup => entity.set_pickup(client_tick),
                            CasterAction::ReadyFight => entity.set_ready_fight(client_tick),
                        }
                    }

                    self.show_skill_name_text(entity_id, skill_id, client_tick);

                    let visuals = skill_visuals(skill_id, 0);
                    let world_position = self.map.as_ref().and_then(|map| map.get_world_position(position));

                    if let Some(effect) = &visuals.completion
                        && let Some(world_position) = world_position
                        && self
                            .skill_completion_tracker
                            .claim(entity_id, skill_id, CUSTOM_COMPLETION_PHASE, client_tick)
                    {
                        self.spawn_position_effect(
                            effect.path,
                            world_position,
                            effect.offset,
                            effect.scale,
                            visuals.light.0,
                            visuals.light.1,
                        );
                    }

                    // Official ground/completion effects from the data-file mapping.
                    // `Success` follows the ground cell when one is known (the
                    // official client plays e.g. Storm Gust's effect there).
                    for variant in [SkillEffectVariant::Ground, SkillEffectVariant::Success] {
                        self.spawn_skill_effect_variants(skill_id, variant, entity_id, world_position, client_tick);
                    }

                    if !self.take_duplicate_cast_sound(entity_id, skill_id, client_tick) {
                        self.skill_sounds.play_skill_sound(skill_id, Some(entity_id), world_position);
                    }
                }
                NetworkEvent::PlaySound { name, account_id, .. } => {
                    let entity_id = EntityId(account_id.0);

                    let position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| entity.get_position());

                    self.skill_sounds.play_network_sound(&name, account_id, position);
                }
                NetworkEvent::DisplayEmotion { entity_id, emotion } => {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        && let Ok(sprite) = self.sprite_loader.get_or_load("이팩트\\emotion.spr", None)
                        && let Ok(actions) = self.action_loader.get_or_load("이팩트\\emotion.act")
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(Emote::new(entity.get_position(), sprite, actions, emotion as usize)));
                    }
                }
                NetworkEvent::AccountId { .. } => {}
                NetworkEvent::CharacterList { characters } => {
                    self.session.clear_character_reconnect();

                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_characters(characters);

                    if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
                        // TODO: this will do one unnecessary restore_focus. check
                        // if that will be problematic

                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(CharacterSelectionWindow::new(
                            client_state().character_slots(),
                            client_state().switch_request(),
                        ));
                    }

                    // A previous map entry may have been aborted because the map
                    // data was missing; show the reason now that we are back at
                    // the character selection screen.
                    if let Some(message) = self.session.take_map_load_error() {
                        self.interface.open_window(ErrorWindow::new(message));
                    }
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => {
                    crate::logging::log_debug!("character selection: server rejected the selection: {message}");
                    // The map server may still be restarting. Stay on the character
                    // selection screen and refresh the list so selecting the character can
                    // simply be retried once the map server is back.
                    let _ = self.networking_system.request_character_list();

                    // Aborting a map entry (e.g. because the map data is missing) can
                    // make the server reject a follow-up request. The map error is the
                    // actionable message, so don't cover it with "Rejected from server".
                    if !self.session.map_entry_aborted() {
                        self.interface.open_window(ErrorWindow::new(message.to_owned()));
                    }
                }
                NetworkEvent::CharacterDeleted => {
                    if let Some(character_id) = self.client_state.follow_mut(client_state().currently_deleting()).take() {
                        self.client_state
                            .follow_mut(client_state().character_slots())
                            .remove_with_id(character_id);
                    }
                    self.client_state.follow_mut(client_state().delete_character_email()).clear();
                    self.interface.close_window_with_class(WindowClass::CharacterDeletion);
                }
                NetworkEvent::CharacterDeletionFailed { message, .. } => {
                    *self.client_state.follow_mut(client_state().currently_deleting()) = None;
                    self.interface.close_window_with_class(WindowClass::CharacterDeletion);
                    self.interface.open_window(ErrorWindow::new(message.to_owned()))
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    crate::logging::log_debug!("character selection: server accepted the character, connecting to the map server");
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    let Some(saved_login_data) = self.saved_login_data.as_ref() else {
                        // The character selection cannot be processed without the
                        // login data. Show the error instead of panicking.
                        self.interface
                            .open_window(ErrorWindow::new("Character selection failed: missing login data.".to_owned()));
                        continue;
                    };
                    let saved_login_data = *saved_login_data;
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system
                        .connect_to_map_server(self.saved_packet_version, &saved_login_data, login_data);
                    // NOTE: Nothing else must be sent to the map server until it responds:
                    // rAthena requires the first read of a session to contain exactly the
                    // login packet and disconnects otherwise. The client tick is part of
                    // the map server login success packet, so there is no need to request
                    // it here.

                    let Some(character_information) = self
                        .client_state
                        .follow(client_state().character_slots())
                        .with_id(login_data.character_id)
                        .cloned()
                    else {
                        self.interface.open_window(ErrorWindow::new(
                            "Character selection failed: the server did not send that character slot.".to_owned(),
                        ));
                        continue;
                    };

                    let mut player = Entity::Player(Player::new(
                        &self.library,
                        saved_login_data.account_id,
                        &character_information,
                        client_tick,
                    ));

                    *self.client_state.follow_mut(client_state().player_name()) = character_information.name;

                    let entity_id = player.get_entity_id();
                    let entity_type = player.get_entity_type();
                    let entity_part_files = player.get_entity_part_files(&self.library);

                    if let Some(animation_data) = self
                        .async_loader
                        .request_animation_data_load(entity_id, entity_type, entity_part_files)
                    {
                        player.set_animation_data(animation_data);
                    }

                    if player.has_falcon()
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
                    {
                        player.set_falcon_animation(falcon_animation);
                    }

                    let weapon_candidates = player.get_weapon_part_candidates();
                    if !weapon_candidates.is_empty()
                        && !player.has_weapon_animation()
                        && let Some(weapon_animation) = self.async_loader.request_weapon_animation_data_load(entity_id, weapon_candidates)
                    {
                        player.set_weapon_animation(weapon_animation);
                    }

                    let layout = self.async_loader.request_skill_tree_layout_load(player.get_job_id(), client_tick);
                    *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
                    *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .clear();

                    self.client_state.follow_mut(client_state().entities()).push(player);
                    self.retrack_local_player();

                    self.interface.close_window_with_class(WindowClass::CharacterSelection);
                    self.interface.open_window(MinimapWindow);
                    self.map_window_cycle = MapWindowCycle::Minimap;

                    self.interface.open_window(CharacterOverviewWindow::new(
                        client_state().player_name(),
                        // TODO: Check that manually asserting is fine. Technically this window should only
                        // be open while the player is selected.
                        this_player().manually_asserted(),
                        client_state().interface_settings(),
                    ));
                    self.interface.open_window(ChatWindow::new(
                        client_state().chat_window(),
                        client_state().chat_messages(),
                        client_state().interface_settings(),
                    ));
                    self.open_hotbar_window();

                    // Put the dialog system in a well-defined state.
                    self.client_state.follow_mut(client_state().dialog_window()).end();

                    self.map = None;
                    // A storage window from a previous session must never show up
                    // in the new character's game state.
                    self.close_storage_window();
                    // The map server sends the map change packet (and with it the
                    // map data we have to load) right after the login handshake.
                    // If the packet stream is desynchronized that packet would be
                    // lost and the client would stay on a black screen forever; the
                    // session machine arms the map-entry watchdog for this case.
                    self.session.begin_map_entry(Instant::now());
                    self.skill_sounds.clear();

                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.level_99_aura_entities.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .add_character(character_information);

                    self.interface.close_window_with_class(WindowClass::CharacterCreation);
                }
                NetworkEvent::CharacterCreationFailed { message, .. } => {
                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterSlotSwitched => {
                    *self.client_state.follow_mut(client_state().switch_request()) = None;
                }
                NetworkEvent::CharacterSlotSwitchFailed => {
                    self.interface
                        .open_window(ErrorWindow::new("Failed to switch character slots".to_owned()));
                }
                NetworkEvent::AddEntity { entity_data } => {
                    let dead_sit = entity_data.dead_sit;

                    if let Some(map) = &self.map
                        && let Some(npc) = Npc::new(&self.library, map, &mut self.path_finder, entity_data, client_tick)
                    {
                        let mut npc = Entity::Npc(npc);

                        // The idle-unit appear packet carries the spawn-time dead/sit
                        // pose (`state`: 0 standing, 1 dead, 2 sitting). Only players
                        // use it in practice.
                        if dead_sit != 0 && npc.get_entity_type() == EntityType::Player {
                            match dead_sit {
                                1 => npc.set_dead(client_tick),
                                2 => npc.set_sitting(client_tick),
                                _ => {}
                            }
                        }

                        let entity_id = npc.get_entity_id();
                        let entity_type = npc.get_entity_type();
                        let entity_part_files = npc.get_entity_part_files(&self.library);

                        // Explicit lifecycle bookkeeping: the server re-sends
                        // `AddEntity` for entities that are already visible (job
                        // change, respawn). The machine records the spawn and the
                        // client replaces the old entity instead of duplicating it.
                        let add_decision = self.entity_lifecycle.on_add_entity(entity_id);
                        let spawn_outcome = self.entity_lifecycle.on_spawned(entity_id, npc.get_job_id());

                        if spawn_outcome.previous_state.is_some() || spawn_outcome.job_changed {
                            crate::logging::log_debug!(
                                "entity: replacing already visible entity {} (previous state {:?}, job changed {})",
                                entity_id.0,
                                spawn_outcome.previous_state,
                                spawn_outcome.job_changed
                            );
                        }

                        let entities = self.client_state.follow_mut(client_state().entities());

                        // If the entity was already visible, we use its old alpha value.
                        if matches!(add_decision, AddEntityDecision::Replace { .. })
                            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id)
                        {
                            npc.inherit_fade_state(entity, client_tick);
                        };

                        // Remove the old copy of a replaced entity so it never
                        // exists twice.
                        entities.retain(|entity| entity.get_entity_id() != entity_id);

                        if let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(entity_id, entity_type, entity_part_files)
                        {
                            npc.set_animation_data(animation_data);
                        }

                        if npc.has_falcon()
                            && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
                        {
                            npc.set_falcon_animation(falcon_animation);
                        }

                        let weapon_candidates = npc.get_weapon_part_candidates();
                        if !weapon_candidates.is_empty()
                            && !npc.has_weapon_animation()
                            && let Some(weapon_animation) =
                                self.async_loader.request_weapon_animation_data_load(entity_id, weapon_candidates)
                        {
                            npc.set_weapon_animation(weapon_animation);
                        }

                        #[cfg(feature = "debug")]
                        npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

                        entities.push(npc);
                    }
                }
                NetworkEvent::RemoveEntity { entity_id, reason } => {
                    self.chat_bubbles.remove_entity(entity_id);
                    self.skill_name_texts.remove_entity(entity_id);
                    self.cast_bars.remove_entity(entity_id);
                    self.skill_sounds.remove_entity(entity_id);
                    self.client_state.follow_mut(client_state().spirit_orbs()).remove(entity_id);

                    let remove_outcome = self.entity_lifecycle.on_remove_entity(entity_id, &reason);
                    match remove_outcome {
                        RemoveEntityOutcome::Known { state, died } => {
                            crate::logging::log_debug!(
                                "entity lifecycle: remove entity {} -> {:?} (died: {})",
                                entity_id.0,
                                state,
                                died
                            );
                        }
                        RemoveEntityOutcome::Unknown => {
                            crate::logging::log_debug!("entity lifecycle: remove for untracked entity {}", entity_id.0);
                        }
                    }

                    // The entity is gone; pending visual loads for it are moot.
                    self.asset_retries.forget(&LoaderId::AnimationData(entity_id));
                    self.asset_retries.forget(&LoaderId::FalconAnimationData(entity_id));
                    self.asset_retries.forget(&LoaderId::WeaponAnimationData(entity_id));

                    // Classic teleport/logout behaviour: entities that leave via
                    // teleport/logout play a warp effect and sound at their last
                    // position. Out-of-sight and death removals stay plain.
                    let disappearance_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .map(|entity| entity.get_position());

                    if matches!(reason, DisappearanceReason::Teleported | DisappearanceReason::LoggedOut)
                        && let Some(position) = disappearance_position
                    {
                        self.spawn_teleport_vanish(position);
                    }

                    // If the motive is dead, you need to set the player to dead.
                    if reason == DisappearanceReason::Died {
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            let entity_type = entity.get_entity_type();

                            if entity_type == EntityType::Monster {
                                let mut entity = entity.clone();
                                entity.set_dead(client_tick);
                                entity.stop_movement();

                                // Remove the entity from the list of alive entities.
                                self.client_state
                                    .follow_mut(client_state().entities())
                                    .retain(|entity| entity.get_entity_id() != entity_id);

                                // Add the entity to the list of dead entities.
                                self.client_state.follow_mut(client_state().dead_entities()).push(entity);
                            } else if entity_type == EntityType::Player {
                                entity.set_dead(client_tick);

                                // If the player is us, we need to open the respawn window.
                                if entity_id == self.client_state.follow(client_state().entities())[0].get_entity_id() {
                                    self.interface.open_window(RespawnWindow);
                                }
                            }
                        }
                    } else {
                        // For non-death disappearances, start fading out the entity.
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            entity.fade_out(reason, client_tick);
                        }
                    }

                    // If the entity that was removed had an attack buffered we remove the entity
                    // from the buffer.
                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id)) {
                        *buffered_action = None;
                    }
                }
                NetworkEvent::AddGroundItem {
                    entity_id,
                    item_id,
                    is_identified,
                    quantity,
                    position,
                    x_offset,
                    y_offset,
                } => {
                    if let Some(map) = self.map.as_ref()
                        && let Some(mut ground_item) = GroundItem::new(
                            map,
                            item_id,
                            entity_id,
                            is_identified,
                            quantity,
                            position,
                            x_offset,
                            y_offset,
                            client_tick,
                        )
                    {
                        let ground_items = self.client_state.follow_mut(client_state().ground_items());
                        let entity_part_files = ground_item.get_entity_part_files(&self.library);

                        if let Some(animation_data) = self
                            .async_loader
                            // TODO: Technically Npc is not correct here. We could add an item
                            // variant or refactor this fuction to take an optional entity
                            // type.
                            .request_animation_data_load(entity_id, EntityType::Npc, entity_part_files)
                        {
                            ground_item.set_animation_data(animation_data);
                        }

                        ground_items.push(ground_item);
                    } else {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] failed to spawn item", "error".red());
                    }
                }
                NetworkEvent::RemoveGroundItem { entity_id } => {
                    if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.fade_out(client_tick);
                    }

                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_pick_up_item(entity_id)) {
                        *buffered_action = None;
                    }
                }
                NetworkEvent::EntityMove {
                    entity_id,
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    let entities = self.client_state.follow_mut(client_state().entities());
                    let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity
                        && let Some(map) = &self.map
                    {
                        entity.move_from_to(
                            map,
                            &mut self.path_finder,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        entity.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::PlayerMove {
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    if let Some(map) = &self.map
                        && let Some(player) = self.client_state.try_follow_mut(this_entity())
                    {
                        player.move_from_to(
                            map,
                            &mut self.path_finder,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        player.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::ChangeMap { map_name, position } => {
                    // A same-map "map move" is a teleport (fly wing, AL_TELEPORT,
                    // random warp, ...). The server sends the normal map move packet
                    // with the current map name; reloading the map for it would take
                    // long and hide the teleport, so reposition the player directly,
                    // play the warp effect/sound and let the server spawn the
                    // surrounding entities again.
                    if self.current_map_name.as_deref() == Some(map_name.as_str()) {
                        let old_position = self.client_state.try_follow(this_entity()).map(|entity| entity.get_position());

                        // Drop the transient state of the old position.
                        self.client_state.follow_mut(client_state().entities()).truncate(1);
                        self.client_state.follow_mut(client_state().dead_entities()).clear();
                        self.client_state.follow_mut(client_state().ground_items()).clear();
                        *self.client_state.follow_mut(client_state().buffered_action()) = None;
                        // A teleport is a warp as well: everything but the player is
                        // re-spawned by the server, so drop the entity bookkeeping and
                        // pending visual loads of the old position. Status effects stay
                        // (a teleport must not remove buffs).
                        self.entity_lifecycle.on_warp();
                        self.asset_retries.clear();
                        self.retrack_local_player();
                        self.particle_holder.clear();
                        self.effect_holder.clear();
                        // The aura tracker must be reset together with the effects:
                        // the player entity survives the fast-path teleport, but its
                        // aura effect was cleared above. Keeping the id in the tracker
                        // would make `update_level_99_auras` skip re-creating it.
                        self.level_99_aura_entities.clear();
                        self.chat_bubbles.clear();
                        self.cast_bars.clear();
                        self.skill_name_texts.clear();

                        if let Some(position) = old_position {
                            self.spawn_teleport_vanish(position);
                        }

                        let Some(map) = self.map.as_mut() else {
                            continue;
                        };
                        let player = self.client_state.follow_mut(this_entity().manually_asserted());
                        player.set_position(map, position, client_tick);
                        self.player_camera.set_focus_point(player.get_position());

                        // Ask the server for the entities around the new position
                        // (same load-end acknowledgement as after a real map change).
                        let _ = self.networking_system.map_loaded();
                        continue;
                    }

                    self.map = None;
                    self.skill_sounds.clear();
                    // The server closes the storage on a map change; close the UI
                    // and drop the transient list immediately so it can't go stale.
                    self.close_storage_window();
                    self.minimap_last_tile = None;
                    self.client_state.follow_mut(client_state().minimap()).clear();
                    self.client_state.follow_mut(client_state().spirit_orbs()).clear();
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.level_99_aura_entities.clear();
                    self.chat_bubbles.clear();
                    self.pending_item_drag = None;
                    self.pending_item_drag = None;
                    self.skill_name_texts.clear();
                    self.cast_bars.clear();
                    self.aftercast_act_until = None;
                    self.warp_skill_units.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    // Only the player must stay alive between map changes.
                    self.client_state.follow_mut(client_state().entities()).truncate(1);
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;

                    // Everything else belongs to the old map session: forget the
                    // entity bookkeeping and pending visual loads, and clear the
                    // status effects (the server re-sends the buffs of the new map
                    // session; a stale buff bar must never survive a warp).
                    self.entity_lifecycle.on_warp();
                    self.asset_retries.clear();
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.retrack_local_player();

                    // Close any remaining dialogs.
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    self.session.map_change_received(map_name.clone(), Some(position), Instant::now());
                    self.async_loader.request_map_load(map_name, Some(position));
                }
                NetworkEvent::UpdateClientTick { client_tick, received_at } => {
                    self.game_timer.set_client_tick(client_tick, received_at);
                }
                NetworkEvent::StatusEffect {
                    entity_id,
                    status_index,
                    active,
                    total_duration_in_milliseconds,
                    remaining_in_milliseconds,
                    values,
                } => {
                    // Status effects of all entities are tracked (the server sends
                    // area status packets); the buff bar only displays the player's
                    // own effects.
                    let definition = status_effect_definition(status_index);
                    let icon = match (active, definition.icon_path()) {
                        (true, Some(path)) => match self.texture_loader.get_or_load(&path, ImageType::Color) {
                            Ok(icon) => Some(icon),
                            Err(error) => {
                                crate::logging::log_warn!("buffbar: failed to load status icon '{}': {:?}", path, error);
                                None
                            }
                        },
                        _ => None,
                    };

                    if active && definition.icon.is_none() {
                        crate::logging::log_debug!("status: status effect {} has no icon/name mapping", status_index);
                    }

                    let client_tick = client_tick.0 as u64;
                    self.client_state.follow_mut(client_state().status_effects()).apply(
                        entity_id.0,
                        status_index,
                        active,
                        total_duration_in_milliseconds,
                        remaining_in_milliseconds,
                        values,
                        client_tick,
                        definition.name.to_owned(),
                        icon,
                    );
                }
                NetworkEvent::ChatMessage { text, color } => {
                    // The server-side aura stylist NPC asks the client to change its
                    // local aura style with a private chat marker. Apply the style and
                    // hide the marker message from the chat window; normal chat is
                    // passed through untouched.
                    if let Some(marker) = parse_aura_style_marker(&text) {
                        match marker {
                            AuraStyleMarker::Style(style) => self.set_aura_style(style),
                            AuraStyleMarker::UnknownStyle => {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] aura stylist NPC requested an unknown aura style", "error".red());
                            }
                        }

                        continue;
                    }

                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, color));
                }
                NetworkEvent::OverheadChatMessage { entity_id, text, color } => {
                    self.chat_bubbles.push(entity_id, text, color, client_tick);
                }
                NetworkEvent::UpdateEntityDetails {
                    entity_id,
                    name,
                    guild_name,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    let mut guild_id_to_request = None;

                    if let Some(entity) = entity {
                        entity.set_details(name);
                        // The player details response also contains the guild name;
                        // an empty name removes the guild label again (for example
                        // after a player left their guild).
                        entity.set_guild_name(guild_name);

                        // The guild label is only known now, request the emblem image
                        // for it (one request per guild per map session).
                        if entity.get_guild_name().is_some() {
                            guild_id_to_request = Some(entity.get_guild_id()).filter(|guild_id| *guild_id != 0);
                        }
                    }

                    if let Some(guild_id) = guild_id_to_request
                        && self.guild_emblems.request_if_needed(guild_id)
                    {
                        let _ = self.networking_system.request_guild_emblem(guild_id);
                    }

                    // The name was requested because the player right-clicked the
                    // character before its name was loaded. Open the menu now.
                    if let Some((pending_entity_id, position)) = self.pending_context_menu
                        && pending_entity_id == entity_id
                    {
                        self.pending_context_menu = None;

                        if open_character_context_menu(&self.client_state, &mut self.interface, entity_id, position) {
                            let _ = self.networking_system.entity_details(entity_id);
                        }
                    }
                }
                NetworkEvent::GuildEmblemImage {
                    guild_id,
                    emblem_id,
                    emblem_data,
                } => match decode_emblem(&emblem_data) {
                    Ok(image) => {
                        let name = format!("guild_emblem_{guild_id}_{emblem_id}");
                        let texture = self.texture_loader.create_color(&name, image, true);
                        self.guild_emblems.insert(guild_id, GuildEmblem { texture });
                    }
                    Err(_error) => {
                        #[cfg(feature = "debug")]
                        print_debug!("Failed to decode guild emblem of guild {guild_id} (emblem id {emblem_id}): {_error}");
                    }
                },
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    damage_amount,
                    attack_duration,
                    is_critical,
                    skill_id,
                    skill_start_tick: _,
                } => {
                    let target_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .map(|entity| entity.get_tile_position());

                    // Skill damage attribution (`ZC_NOTIFY_SKILL`, 0x01DE): play
                    // the official release/hit effect at the damaged target and the
                    // caster effect on the caster. Attack packets carry no skill id,
                    // so auto attacks can never trigger skill effects.
                    if let Some(skill_id) = skill_id
                        && self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) == Some(source_entity_id)
                    {
                        let target_world_position = target_position
                            .and_then(|tile_position| self.map.as_ref().and_then(|map| map.get_world_position(tile_position)));
                        if !crate::world::has_custom_completion(skill_id) {
                            if !crate::world::skill_has_effect(skill_id) && !crate::world::skill_has_procedural_effect(skill_id) {
                                crate::logging::log_debug!("damage skill: skill {} has no effect mapping", skill_id.0);
                            }

                            for variant in [SkillEffectVariant::Success, SkillEffectVariant::Release, SkillEffectVariant::Hit] {
                                self.spawn_skill_effect_variants(skill_id, variant, source_entity_id, target_world_position, client_tick);
                            }

                            self.spawn_skill_effect_variants(
                                skill_id,
                                SkillEffectVariant::Caster,
                                source_entity_id,
                                target_world_position,
                                client_tick,
                            );
                        }
                    }

                    // Auto attack logic.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == source_entity_id)
                    {
                        let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                        let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                        if let Some(BufferedAction::AttackEntity { entity_id }) = *buffered_action {
                            let _ = self.networking_system.player_attack(entity_id);

                            if !auto_attack {
                                *buffered_action = None;
                            }
                        }
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    // TODO: Maybe also or_else this_entity?
                    {
                        if let Some(target_position) = target_position {
                            entity.rotate_towards(target_position);
                        }

                        entity.set_attack(attack_duration, is_critical, client_tick);
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        let particle: Box<dyn Particle + Send + Sync> = match damage_amount {
                            Some(amount) => Box::new(DamageNumber::new(entity.get_position(), amount as u64, is_critical)),
                            None => Box::new(Miss::new(entity.get_position())),
                        };

                        self.particle_holder.spawn_particle(particle);
                    }

                    // Classic bow arrow projectile: only auto attacks (attack
                    // packets carry no skill id) with a bow-type weapon spawn an
                    // arrow that flies from the attacker to the target.
                    if skill_id.is_none()
                        && let Some(target_tile) = target_position
                    {
                        let attacker = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                            .map(|entity| (entity.get_weapon(), entity.get_position()));

                        if let Some((weapon, attacker_position)) = attacker
                            && is_bow_weapon(weapon)
                            && let Some(target_world_position) = self.map.as_ref().and_then(|map| map.get_world_position(target_tile))
                        {
                            self.ensure_arrow_assets();

                            if let Some(arrow_assets) = self.arrow_assets.clone() {
                                crate::logging::log_debug!(
                                    "arrow projectile: spawning arrow from {:?} to {:?}",
                                    attacker_position,
                                    target_world_position
                                );
                                self.particle_holder.spawn_particle(Box::new(ArrowProjectile::new(
                                    arrow_assets,
                                    attacker_position,
                                    target_world_position,
                                )));
                            }
                        }
                    }
                }
                NetworkEvent::EntityPickUpItem { entity_id, item_entity_id } => {
                    let item_position = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == item_entity_id)
                        .map(|item| item.get_tile_position());

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        if let Some(item_position) = item_position {
                            entity.rotate_towards(item_position);
                        }

                        if matches!(entity.get_entity_type(), EntityType::Player | EntityType::Hidden) {
                            entity.set_pickup(client_tick);
                        }
                    }
                }
                NetworkEvent::HealEffect { entity_id, heal_amount } => {
                    // `ZC_RECOVERY` has no entity id (the server only sends it to the
                    // player themselves), so `None` resolves to the local player.
                    let position = entity_id
                        .and_then(|entity_id| {
                            self.client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id)
                                .map(|entity| entity.get_position())
                        })
                        .or_else(|| self.client_state.try_follow(this_entity()).map(|entity| entity.get_position()));

                    if let Some(position) = position {
                        self.particle_holder
                            .spawn_particle(Box::new(HealNumber::new(position, heal_amount as u64)));
                    }
                }
                NetworkEvent::ChangeDirection {
                    entity_id,
                    head_direction,
                    direction,
                } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_direction(direction);
                        entity.set_head_direction(head_direction);
                    }
                }
                NetworkEvent::StateChange {
                    entity_id,
                    body_state,
                    health_state,
                    effect_state,
                    ..
                } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_body_state(body_state);
                        entity.set_health_state(health_state);
                        // Applies hide/cloak/invisible by hiding the entity; the
                        // remaining option bits (falcon/cart/riding/costumes) are
                        // stored for the appearance workstream.
                        entity.set_effect_state(effect_state, client_tick);
                    }

                    // Load the hunter falcon animation lazily the first time the
                    // option is enabled for this entity.
                    let falcon_entity_index = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .position(|entity| entity.get_entity_id() == entity_id && entity.has_falcon() && !entity.has_falcon_animation())
                        .filter(|_| effect_state & 0x10 != 0);

                    if let Some(index) = falcon_entity_index
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity_id)
                        && let Some(entity) = self.client_state.follow_mut(client_state().entities()).get_mut(index)
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }
                }
                NetworkEvent::UpdateEntityHealth {
                    entity_id,
                    health_points,
                    maximum_health_points,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_health(health_points, maximum_health_points);
                    }

                    // Keep the own HP bar in the party window in sync. The server
                    // does not send `ZC_NOTIFY_HP_TO_GROUPM` for the local player,
                    // so the party window takes the HP from the local entity.
                    let own_account_id = self.saved_login_data.as_ref().map(|login_data| login_data.account_id);
                    let is_own_entity = self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|entity| entity.get_entity_id() == entity_id);

                    if let (true, Some(own_account_id)) = (is_own_entity, own_account_id) {
                        self.client_state.follow_mut(client_state().party()).apply_health(
                            own_account_id,
                            health_points as i32,
                            maximum_health_points as i32,
                        );
                    }
                }
                NetworkEvent::UpdateStat { stat_type } => {
                    // The zeny amount is displayed by the inventory window, so keep it in
                    // the client state as well (the player stats do not contain it).
                    if let StatType::Zeny(value) = stat_type {
                        *self.client_state.follow_mut(client_state().zeny()) = Some(u64::from(value));
                    }

                    match self.client_state.try_follow_mut(this_player()) {
                        Some(player) => player.update_stat(stat_type),
                        // The initial stat burst can arrive before the local player entity
                        // was created; buffer it instead of dropping it.
                        None => self.pending_stat_updates.push(stat_type),
                    }
                }
                NetworkEvent::OpenDialog { text, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .initialize(npc_id)
                        .add_text(text);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddNextButton { npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // An NPCs could start the dialog with this packet so we want to make sure it's initialized.
                        .initialize(npc_id)
                        .add_next_button();

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddCloseButton { npc_id } => {
                    // Some NPCs send the `CloseButtonPacket` after the dialog
                    // has been closed. We want to filter these out because otherwise we get a
                    // close button at the start of the next dialog.
                    if self.interface.is_window_with_class_open(WindowClass::Dialog) {
                        self.client_state
                            .follow_mut(client_state().dialog_window())
                            // Technically this call is redundant since the window is already open
                            // but we keep it for consistency.
                            .initialize(npc_id)
                            .add_close_button();
                    }
                }
                NetworkEvent::AddChoiceButtons { choices, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // Some NPCs start the dialog with this packet so we need to make sure it's initialized.
                        .initialize(npc_id)
                        .add_choice_buttons(choices);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddQuestEffect { quest_effect } => {
                    if let Some(map) = &self.map {
                        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
                    }
                }
                NetworkEvent::RemoveQuestEffect { entity_id } => self.particle_holder.remove_quest_icon(entity_id),
                NetworkEvent::SetInventory { items } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .fill(&self.async_loader, items);
                }
                NetworkEvent::SetStorage { items, name } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .fill(&self.async_loader, items);
                    *self.client_state.follow_mut(client_state().storage_name()) = name;

                    if !self.interface.is_window_with_class_open(WindowClass::Storage) {
                        self.interface.open_window(StorageWindow);
                    }
                }
                NetworkEvent::SetStorageAmount { amount, maximum_amount } => {
                    *self.client_state.follow_mut(client_state().storage_amount()) = amount;
                    *self.client_state.follow_mut(client_state().storage_maximum_amount()) = maximum_amount;
                }
                NetworkEvent::StorageItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .add_item(&self.async_loader, item);
                }
                NetworkEvent::StorageItemRemoved { storage_index, amount } => {
                    self.client_state
                        .follow_mut(client_state().storage())
                        .remove_item(InventoryIndex(storage_index), amount as u16);
                }
                NetworkEvent::StorageClosed => {
                    self.close_storage_window();
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .add_item(&self.async_loader, item);

                    // TODO: Update the selling items. If you pick up an item
                    // that you already have the sell window
                    // should allow you to sell the new
                    // amount of items.
                }
                NetworkEvent::ItemObtained {
                    item_id,
                    quantity,
                    is_identified,
                } => {
                    let name = self.library.item_name(item_id, is_identified);
                    let message = format!("You got {name} ({quantity}).");
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, MessageColor::Information));
                }
                NetworkEvent::InventoryItemRemoved { index, amount, .. } => {
                    self.client_state.follow_mut(client_state().inventory()).remove_item(index, amount);
                }
                NetworkEvent::SkillTree { skill_information } => {
                    crate::logging::log_debug!("skills: initial skill list with {} entries", skill_information.len());

                    if skill_information.is_empty() {
                        // An empty list is valid for a character without skills, but if the
                        // server DB has learned skills it means the list was lost (stream
                        // desync) - make that visible instead of silently showing an empty
                        // skill tree.
                        crate::logging::log_warn!("skills: server sent an empty skill list (learned skills may be missing)");
                    }
                    *self.client_state.follow_mut(client_state().skill_tree().skills()) =
                        skill_information.into_iter().map(LearnedSkill::new).collect();

                    self.merge_learned_skills_into_skill_tree(client_tick);
                }
                NetworkEvent::UpdateSkillInfo {
                    skill_id,
                    skill_type,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                } => {
                    // Partial skill update (level up, SP cost/range change). The
                    // name is not part of this packet, so only known skills are
                    // updated; the tree/hotbar read the new level from this entry.
                    crate::logging::log_debug!("skills: update skill {} level {}", skill_id.0, skill_level.0);
                    let skills = self.client_state.follow_mut(client_state().skill_tree().skills());

                    if let Some(learned_skill) = skills.iter_mut().find(|learned_skill| learned_skill.skill_id == skill_id) {
                        learned_skill.skill_type = skill_type;
                        learned_skill.skill_level = skill_level;
                        learned_skill.spell_point_cost = spell_point_cost;
                        learned_skill.attack_range = attack_range;
                        learned_skill.upgradable = upgradable;
                    }
                }
                NetworkEvent::AddSkill { skill_information } => {
                    // A skill that was learned/upgraded after the initial skill list.
                    // Replace the old entry so the skill tree and the hotbar pick up
                    // the new level immediately, without a relog.
                    let learned_skill = LearnedSkill::new(skill_information);
                    crate::logging::log_debug!(
                        "skills: add skill {} level {}",
                        learned_skill.skill_id.0,
                        learned_skill.skill_level.0
                    );
                    let skills = self.client_state.follow_mut(client_state().skill_tree().skills());

                    match skills.iter_mut().find(|existing| existing.skill_id == learned_skill.skill_id) {
                        Some(existing) => *existing = learned_skill,
                        None => skills.push(learned_skill),
                    }

                    self.merge_learned_skills_into_skill_tree(client_tick);
                }
                NetworkEvent::InventoryItemAmountChanged { index, amount } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_amount(index, amount);
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_equipped_position(index, equipped_position);
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    else {
                        // Not every job change belongs to an entity the client knows
                        // about: the population engine also changes the jobs of its
                        // in-memory shell players, whose ids are not the ids the client
                        // received in the spawn packets. Ignore those updates instead of
                        // panicking on them.
                        #[cfg(feature = "debug")]
                        print_debug!(
                            "[{}] ignoring job change of unknown entity {:?}",
                            "warning".yellow(),
                            account_id
                        );
                        // The entity is not in the world yet (e.g. the server sent the job
                        // change while the map was still loading). There is nothing to update.
                        continue;
                    };

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    if !self.library.contains::<crate::world::SkillTreeLayout>(job_id) {
                        // Population/LLM shells change their job to jobs that this
                        // client's trimmed skill-tree tables do not contain. Log the
                        // offending job id (harmless; the local skill tree is only
                        // updated for the own character) to make future investigations
                        // of missing job tables easy.
                        eprintln!(
                            "[skill_tree] job {:?} (entity {:?}) is not in the client skill-tree tables",
                            job_id, account_id
                        );
                    }

                    entity.set_job(&self.library, job_id);
                    self.entity_lifecycle.on_job_change(entity.get_entity_id(), job_id);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }

                    if entity.has_falcon()
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity.get_entity_id())
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }

                    let weapon_candidates = entity.get_weapon_part_candidates();
                    if !weapon_candidates.is_empty()
                        && !entity.has_weapon_animation()
                        && let Some(weapon_animation) = self
                            .async_loader
                            .request_weapon_animation_data_load(entity.get_entity_id(), weapon_candidates)
                    {
                        entity.set_weapon_animation(weapon_animation);
                    }

                    // Only the own character's job change affects the local skill tree
                    // window. Population shells send job changes for jobs that do not
                    // exist in this client's skill tree tables, which used to panic when
                    // the skill tree layout was looked up for them.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id().0 == account_id.0)
                    {
                        let layout = self.async_loader.request_skill_tree_layout_load(job_id, client_tick);
                        *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) =
                            layout.tabs.len().saturating_sub(1);
                        *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                        self.client_state
                            .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                            .clear();
                    }
                }
                NetworkEvent::ChangeHair { account_id, hair_id } => {
                    let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    else {
                        #[cfg(feature = "debug")]
                        print_debug!(
                            "[{}] ignoring hair change of unknown entity {:?}",
                            "warning".yellow(),
                            account_id
                        );
                        continue;
                    };

                    entity.set_hair(hair_id as usize);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }

                    if entity.has_falcon()
                        && let Some(falcon_animation) = self.async_loader.request_falcon_animation_data_load(entity.get_entity_id())
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }

                    let weapon_candidates = entity.get_weapon_part_candidates();
                    if !weapon_candidates.is_empty()
                        && !entity.has_weapon_animation()
                        && let Some(weapon_animation) = self
                            .async_loader
                            .request_weapon_animation_data_load(entity.get_entity_id(), weapon_candidates)
                    {
                        entity.set_weapon_animation(weapon_animation);
                    }
                }
                NetworkEvent::AppearanceChanged {
                    account_id,
                    sprite_type,
                    value,
                } => {
                    let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                    else {
                        continue;
                    };

                    if !entity.apply_sprite_change(sprite_type, value as u16) {
                        continue;
                    }

                    // Sprite layers changed: reload the animation data so both the world
                    // sprite and the equipment paper doll show the new appearance.
                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }
                }
                NetworkEvent::AutoRunSkill { skill_id, skill_level } => {
                    // Skill items (fly wing, casting scrolls, ...): the server only
                    // tells us which skill the item wants; the client has to send the
                    // skill request itself (that is what the original client does).
                    // Item skills are self-targeted on this server setup (fly wing's
                    // AL_TELEPORT level 1), so cast it on the local player.
                    let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id()) else {
                        continue;
                    };

                    let _ = self.networking_system.cast_skill(skill_id, skill_level, entity_id);
                }
                NetworkEvent::LocalAppearanceChanged { sprite_type, value } => {
                    let Some(entity) = self.client_state.try_follow_mut(this_entity()) else {
                        continue;
                    };

                    if !entity.apply_sprite_change(sprite_type, value as u16) {
                        continue;
                    }

                    // Sprite layers changed: reload the animation data so that both the
                    // world sprite and the equipment paper doll show the new appearance.
                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }
                }
                NetworkEvent::LoggedOut => {
                    // The server confirmed the logout; switch away from the map
                    // music right away so there is no silent gap while the map
                    // server disconnects and the menu map is loaded again.
                    self.client_state.follow_mut(client_state().status_effects()).clear();
                    self.client_state.follow_mut(client_state().spirit_orbs()).clear();
                    self.audio_engine.play_background_music_track(UI_BACKGROUND_MUSIC);
                    self.networking_system.disconnect_from_map_server();
                }
                NetworkEvent::FriendRequest { requestee } => {
                    crate::state::friend_names::remember(requestee.character_id.0, &requestee.name);
                    self.interface.open_window(FriendRequestWindow::new(requestee));
                }
                // --- Party events (2019) -------------------------------------------
                NetworkEvent::PartyOption {
                    experience_option,
                    item_pickup,
                    item_share,
                } => {
                    let party = self.client_state.follow_mut(client_state().party());
                    party.experience_option = experience_option;
                    party.item_pickup = item_pickup != 0;
                    party.item_share = item_share != 0;
                }
                NetworkEvent::PartyList { party_name, members } => {
                    let was_in_party = self.client_state.follow(client_state().party()).is_in_party();

                    let party = self.client_state.follow_mut(client_state().party());

                    if members.is_empty() && !was_in_party {
                        continue;
                    }

                    party.apply_roster(party_name, &members);

                    if members.is_empty() {
                        party.clear();
                        self.interface.close_window_with_class(WindowClass::Party);
                    } else if !was_in_party {
                        // The player just joined a party (login burst or accepted
                        // invite): show the roster like the original client.
                        self.open_party_window();
                    }
                }
                NetworkEvent::PartyMemberAdded { member } => {
                    let was_in_party = self.client_state.follow(client_state().party()).is_in_party();

                    self.client_state.follow_mut(client_state().party()).apply_member_details(&member);

                    if !was_in_party {
                        self.open_party_window();
                    }
                }
                NetworkEvent::PartyMemberRemoved {
                    account_id,
                    character_name,
                    result,
                } => {
                    let own_account_id = self.saved_login_data.as_ref().map(|login_data| login_data.account_id);
                    let is_self = own_account_id == Some(account_id);

                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_member_removed(account_id);

                    let message = match result {
                        0 => format!("{character_name} left the party."),
                        1 => format!("{character_name} was expelled from the party."),
                        2 => format!("{character_name} could not leave the party."),
                        3 => format!("{character_name} could not be expelled from the party."),
                        _ => format!("Party member {character_name} left the party."),
                    };

                    self.push_chat_information(message);

                    if is_self {
                        self.client_state.follow_mut(client_state().party()).clear();
                        self.interface.close_window_with_class(WindowClass::Party);
                    }
                }
                NetworkEvent::PartyMemberHealth {
                    account_id,
                    health_points,
                    maximum_health_points,
                } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_health(account_id, health_points, maximum_health_points);
                }
                NetworkEvent::PartyMemberPosition { account_id, position } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_position(account_id, position);
                }
                NetworkEvent::PartyMemberJobLevel {
                    account_id,
                    job_id,
                    base_level,
                } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_job_level(account_id, job_id, base_level);
                }
                NetworkEvent::PartyMemberAlive { account_id, is_dead } => {
                    self.client_state
                        .follow_mut(client_state().party())
                        .apply_alive(account_id, is_dead);
                }
                NetworkEvent::PartyInvite { party_id, party_name } => {
                    if self.client_state.follow(client_state().party()).is_in_party() {
                        // Already in a party; the server will reject the invitation
                        // anyway, but tell the user why nothing happens.
                        self.push_chat_information(format!(
                            "Ignoring party invitation from \"{party_name}\": you are already in a party."
                        ));
                        continue;
                    }

                    *self.client_state.follow_mut(client_state().party().invite()) =
                        Some(crate::state::party::PartyInvitePrompt { party_id, party_name });

                    self.interface.open_window(PartyInviteWindow::new(client_state().party().invite()));
                }
                NetworkEvent::PartyInviteReply { character_name, result } => {
                    self.interface.close_window_with_class(WindowClass::PartyInvite);
                    *self.client_state.follow_mut(client_state().party().invite()) = None;

                    let message = match result {
                        0 => format!("{character_name} is already in a party."),
                        1 => format!("{character_name} rejected the party invitation."),
                        2 => format!("{character_name} accepted the party invitation."),
                        3 => "The party is full.".to_owned(),
                        4 => format!("A character of {character_name}'s account is already in the party."),
                        5 => format!("{character_name} blocks party invitations."),
                        7 => format!("{character_name} is not online."),
                        8 => format!("{character_name} is on a map where parties cannot be joined."),
                        9 => "You cannot invite on this map.".to_owned(),
                        10 => "Party changes are not possible in a memorial dungeon.".to_owned(),
                        11 => format!("{character_name} does not meet the party level restrictions."),
                        _ => format!("Party invitation result: {result}."),
                    };

                    self.push_chat_information(message);
                }
                NetworkEvent::PartyCreationResult { result } => {
                    let message = match result {
                        0 => "Party created.".to_owned(),
                        1 => "A party with that name already exists.".to_owned(),
                        2 => "You are already in a party.".to_owned(),
                        3 => "You cannot create a party on this map.".to_owned(),
                        _ => format!("Party creation failed (result {result})."),
                    };

                    self.push_chat_information(message);
                }
                NetworkEvent::PartyChatMessage { account_id, message } => {
                    let name = self
                        .client_state
                        .follow(client_state().party())
                        .member_name(account_id)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("Account #{}", account_id.0));

                    // The server echoes the full "<name> : <message>" string in
                    // `ZC_NOTIFY_CHAT_PARTY`, but the client already knows and
                    // renders the name, so strip the redundant prefix.
                    let message = message
                        .strip_prefix(&format!("{name} : "))
                        .unwrap_or(&message)
                        .trim_end_matches('\0');

                    self.client_state.follow_mut(client_state().chat_messages()).push(ChatMessage::new(
                        format!("[Party] {name}: {message}"),
                        MessageColor::Rgb {
                            red: 120,
                            green: 230,
                            blue: 140,
                        },
                    ));
                }
                NetworkEvent::PartyConfig { allow_invites } => {
                    self.client_state.follow_mut(client_state().party()).allow_invites = allow_invites;
                }
                NetworkEvent::FriendRemoved { account_id, character_id } => {
                    let friend_list = self.client_state.follow_mut(client_state().friend_list());
                    if let Some(index) = friend_list
                        .iter()
                        .position(|friend| friend.account_id == account_id && friend.character_id == character_id)
                    {
                        friend_list.remove(index);
                        let friend_online = self.client_state.follow_mut(client_state().friend_online());
                        if index < friend_online.len() {
                            friend_online.remove(index);
                        }
                    }
                }
                NetworkEvent::FriendAdded { friend } => {
                    crate::state::friend_names::remember(friend.character_id.0, &friend.name);
                    self.client_state.follow_mut(client_state().friend_list()).push(friend);
                    self.client_state.follow_mut(client_state().friend_online()).push(false);
                }
                NetworkEvent::FriendOnlineState {
                    account_id,
                    character_id,
                    online,
                    name,
                } => {
                    crate::logging::log_debug!("friend 0x0206: cid={} online={} name={:?}", character_id.0, online, name);
                    crate::state::friend_names::remember(character_id.0, &name);

                    let index = self
                        .client_state
                        .follow(client_state().friend_list())
                        .iter()
                        .position(|friend| friend.character_id == character_id || friend.account_id == account_id);

                    if let Some(index) = index {
                        {
                            let friend_list = self.client_state.follow_mut(client_state().friend_list());
                            if !name.trim().is_empty() {
                                friend_list[index].name = name.clone();
                            } else if friend_list[index].name.trim().is_empty() {
                                friend_list[index].name = crate::state::friend_names::cached_name(character_id.0).unwrap_or_default();
                            }
                        }

                        let friend_count = self.client_state.follow(client_state().friend_list()).len();
                        let friend_online = self.client_state.follow_mut(client_state().friend_online());
                        friend_online.resize(friend_count, false);
                        friend_online[index] = online;
                    } else {
                        // The 0x0206 update can arrive before/without a 0x0201 entry
                        // (for example right after accepting a friend request): add the
                        // friend so the UI shows it immediately.
                        let display_name = if name.trim().is_empty() {
                            crate::state::friend_names::cached_name(character_id.0).unwrap_or_default()
                        } else {
                            name.clone()
                        };

                        let friend_count = {
                            let friend_list = self.client_state.follow_mut(client_state().friend_list());
                            friend_list.push(Friend {
                                account_id,
                                character_id,
                                name: display_name,
                            });
                            friend_list.len()
                        };

                        let friend_online = self.client_state.follow_mut(client_state().friend_online());
                        friend_online.resize(friend_count, false);
                        friend_online[friend_count - 1] = online;
                    }
                }
                NetworkEvent::VisualEffect { effect_path, entity_id } => {
                    self.spawn_entity_effect(effect_path, entity_id, Vector3::new(0.0, 9.0, 0.0), 1.0, Color::WHITE, 50.0);
                }
                NetworkEvent::AddSkillUnit {
                    entity_id,
                    unit_id,
                    position,
                } => {
                    // Warp portal skill units have no sprite in the game files; the
                    // client draws the classic portal effect for them (like for warp
                    // NPCs), see `render_warp_portals`. Ambient NPCs cast these in the
                    // Prontera plaza.
                    if matches!(unit_id, UnitId::WarpWaiting | UnitId::WarpActive) {
                        let world_position = self.map.as_ref().and_then(|map| map.get_world_position(position));

                        if let Some(world_position) = world_position {
                            // `WarpWaiting` and `WarpActive` are separate packets for the
                            // same unit, so replace the previous position.
                            self.warp_skill_units.retain(|(unit_entity_id, ..)| *unit_entity_id != entity_id);
                            self.warp_skill_units.push((entity_id, world_position, Instant::now()));
                        }

                        continue;
                    }

                    let Some(map) = &self.map else {
                        continue;
                    };

                    match unit_id {
                        UnitId::Firewall => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let Ok(effect) = self.effect_loader.get_or_load("firewall.str", &self.texture_loader) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] failed to load firewall effect", "error".red());
                                continue;
                            };
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    1.0,
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(255, 30, 0),
                                    60.0,
                                    true,
                                )),
                                entity_id,
                            );
                        }
                        UnitId::Pneuma => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let Ok(effect) = self.effect_loader.get_or_load("pneuma1.str", &self.texture_loader) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] failed to load pneuma effect", "error".red());
                                continue;
                            };
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    1.0,
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(83, 220, 108),
                                    40.0,
                                    false,
                                )),
                                entity_id,
                            );
                        }
                        _ => {}
                    }
                }
                NetworkEvent::RemoveSkillUnit { entity_id } => {
                    self.effect_holder.remove_unit(entity_id);
                    self.warp_skill_units.retain(|(unit_entity_id, ..)| *unit_entity_id != entity_id);
                }
                NetworkEvent::SetFriendList { friend_list } => {
                    let friend_list = friend_list
                        .into_iter()
                        .map(|mut friend| {
                            if friend.name.trim().is_empty() {
                                if let Some(name) = crate::state::friend_names::cached_name(friend.character_id.0) {
                                    friend.name = name;
                                }
                            } else {
                                crate::state::friend_names::remember(friend.character_id.0, &friend.name);
                            }
                            friend
                        })
                        .collect::<Vec<_>>();
                    let friend_count = friend_list.len();

                    crate::logging::log_debug!(
                        "friend list: {} entries (names filled from the persistent cache where known)",
                        friend_count
                    );

                    *self.client_state.follow_mut(client_state().friend_list()) = friend_list;
                    *self.client_state.follow_mut(client_state().friend_online()) = vec![false; friend_count];
                }
                NetworkEvent::SetHotkeyData { tab, hotkeys } => {
                    // The 2019 server sends 4 tabs with 38 entries each. Keep all of
                    // them and distinguish skill hotkeys from item hotkeys (the old
                    // code treated every entry as a skill id, so item hotkeys were
                    // "cast" as skills and every tab but 0 was dropped).
                    let tab_index = tab.0 as usize;

                    if tab_index >= HOTBAR_TAB_COUNT {
                        continue;
                    }

                    let job_id = self.client_state.try_follow(this_entity()).map(Entity::get_job_id);

                    for (index, hotkey) in hotkeys.into_iter().take(HOTBAR_ENTRIES_PER_TAB).enumerate() {
                        match hotkey {
                            HotkeyState::Bound(hotkey) => match hotkey.hotkey_type {
                                HotkeyType::Skill => {
                                    let Some(job_id) = job_id else {
                                        continue;
                                    };

                                    let skill_id = SkillId(hotkey.item_or_skill_id as u16);

                                    let mut skill = self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick);
                                    skill.maximum_level.0 = hotkey.quantity_or_skill_level;

                                    self.client_state
                                        .follow_mut(client_state().hotbar())
                                        .apply_skill(tab_index, index, Some(skill));
                                }
                                HotkeyType::Item => {
                                    self.client_state.follow_mut(client_state().hotbar()).apply_item(
                                        tab_index,
                                        index,
                                        ItemId(hotkey.item_or_skill_id),
                                        hotkey.quantity_or_skill_level,
                                    );
                                }
                            },
                            HotkeyState::Unbound => self.client_state.follow_mut(client_state().hotbar()).clear_entry(tab_index, index),
                        }
                    }

                    // Refresh the visible slots when the active tab was updated
                    // (also for tabs other than 0).
                    self.client_state.follow_mut(client_state().hotbar()).refresh_visible_slots();
                }
                NetworkEvent::OpenShop { items } => {
                    self.shop_open = true;

                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    *self.client_state.follow_mut(client_state().shop_items()) = items
                        .into_iter()
                        .map(|item| self.async_loader.request_shop_item_metadata_load(item))
                        .collect();

                    self.interface
                        .open_window(BuyWindow::new(client_state().shop_items(), client_state().buy_cart()));
                    self.interface.open_window(BuyCartWindow::new(client_state().buy_cart()));
                }
                NetworkEvent::AskBuyOrSell { shop_id } => {
                    self.interface.open_window(BuyOrSellWindow::new(shop_id));
                }
                NetworkEvent::BuyingCompleted { result } => match result {
                    BuyItemResult::Successful => {
                        let _ = self.networking_system.close_shop();
                        self.shop_open = false;

                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Buy);
                        self.interface.close_window_with_class(WindowClass::BuyCart);
                    }
                    result => {
                        let message = match result {
                            BuyItemResult::Successful => unreachable!(),
                            BuyItemResult::NotEoughZeny => "You do not have enough zeny.",
                            BuyItemResult::WeightLimitExceeded => "You are over your weight limit.",
                            BuyItemResult::TooManyItems => "You have too many items.",
                            BuyItemResult::TooManyOfThisItem => "You cannot hold that many of this item.",
                            BuyItemResult::PropsOpenAir => "Open-air shop purchases have to be traded through RODEX.",
                            BuyItemResult::ExchangeFailed => "The exchange failed.",
                            BuyItemResult::ExchangeWellDone => "The exchange was well done.",
                            BuyItemResult::ItemSoldOut => "The item is already sold out.",
                            BuyItemResult::NotEnoughGoods => "There are not enough goods to exchange.",
                        };

                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new(message.to_owned(), MessageColor::Error));
                    }
                },
                NetworkEvent::SellItemList { items } => {
                    self.shop_open = true;

                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    let inventory_items = self.client_state.follow(client_state().inventory().items());
                    let sell_items = items
                        .into_iter()
                        .map(|item| {
                            let inventory_item = inventory_items
                                .iter()
                                .find(|inventory_item| inventory_item.index == item.inventory_index)
                                .expect("item not in inventory");

                            let name = inventory_item.metadata.name.clone();
                            let texture = inventory_item.metadata.texture.clone();
                            let stats = inventory_item.metadata.stats;
                            let description = inventory_item.metadata.description.clone();
                            let card_names = inventory_item.metadata.card_names.clone();
                            let slot_count = inventory_item.metadata.slot_count;
                            let tooltip_text = inventory_item.metadata.tooltip_text.clone();
                            let quantity = match &inventory_item.details {
                                korangar_networking::InventoryItemDetails::Regular { amount, .. } => *amount,
                                korangar_networking::InventoryItemDetails::Equippable { .. } => 1,
                            };

                            SellItem {
                                metadata: (
                                    ResourceMetadata {
                                        name,
                                        texture,
                                        stats,
                                        description,
                                        card_names,
                                        slot_count,
                                        tooltip_text,
                                    },
                                    quantity,
                                ),
                                inventory_index: item.inventory_index,
                                price: item.price,
                                overcharge_price: item.overcharge_price,
                            }
                        })
                        .collect();

                    *self.client_state.follow_mut(client_state().sell_items()) = sell_items;

                    self.interface
                        .open_window(SellWindow::new(client_state().sell_items(), client_state().sell_cart()));
                    self.interface.open_window(SellCartWindow::new(client_state().sell_cart()));
                }
                NetworkEvent::SellingCompleted { result } => match result {
                    SellItemsResult::Success => {
                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Sell);
                        self.interface.close_window_with_class(WindowClass::SellCart);
                    }
                    SellItemsResult::Error => {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Failed to sell items".to_owned(), MessageColor::Error));
                    }
                },
                NetworkEvent::AttackFailed {
                    target_entity_id,
                    target_position,
                    player_position,
                    attack_range,
                } => {
                    if let Some(map) = &self.map
                        && self.client_state.try_follow_mut(this_entity()).is_some()
                        // Make sure that the entity is on screen.
                        && self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .any(|entity| entity.get_entity_id() == target_entity_id)
                        && let Some(path) =
                            self.path_finder
                                .find_walkable_path_in_range(&**map, player_position, target_position, attack_range)
                    {
                        let nearest_tile = path.last().unwrap();

                        let _ = self.networking_system.player_move(WorldPosition {
                            x: nearest_tile.x,
                            y: nearest_tile.y,
                            direction: Direction::North,
                        });

                        *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity {
                            entity_id: target_entity_id,
                        });
                    }
                }
                NetworkEvent::UpdateSkill {
                    skill_id,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.client_state.follow_mut(client_state().skill_tree()).update_skill(
                        skill_id,
                        skill_level,
                        spell_point_cost,
                        attack_range,
                        upgradable,
                    );
                }
                NetworkEvent::RemoveSkill { skill_id } => {
                    self.client_state.follow_mut(client_state().skill_tree()).remove_skill(skill_id);
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .remove(&skill_id);
                }
                // --- Feedback for small/no-op packets (exp, skill fail, ammo, ban)
                NetworkEvent::GainedExperience {
                    amount,
                    experience_type,
                    experience_source,
                } => {
                    self.pending_experience.add(amount, experience_type, experience_source);
                }
                NetworkEvent::SkillUseFailed { skill_id, cause } => {
                    let reason = match cause {
                        0 => "skill level is too low",
                        1 => "not enough SP",
                        2 => "not enough HP",
                        3 => "not enough items",
                        4 => "skill is still in delay",
                        5 => "not enough zeny",
                        6 => "wrong weapon",
                        7 => "missing red gemstone",
                        8 => "missing blue gemstone",
                        9 => "too much weight",
                        11 => "invalid target",
                        12 => "too many ancilla",
                        13 => "missing holy water",
                        14 => "missing ancilla",
                        22 => "skill is already active",
                        23 => "condition not met",
                        26 => "invalid position",
                        _ => "the skill cannot be used",
                    };

                    self.push_chat_information(format!("Cannot use skill {}: {reason}.", skill_id.0));
                }
                NetworkEvent::StatUpFailed => {
                    self.push_chat_information("You cannot raise that stat right now.".to_owned());
                }
                NetworkEvent::AmmunitionAction { action } => {
                    let message = match action {
                        AmmunitionActionType::EquipProperAmmunitionFirst => "You have to equip proper ammunition first.",
                        AmmunitionActionType::WeightLimitExceeded1 | AmmunitionActionType::WeightLimitExceeded2 => {
                            "You cannot carry that much ammunition (weight limit)."
                        }
                        AmmunitionActionType::Equipped => "Ammunition equipped.",
                    };

                    self.push_chat_information(message.to_owned());
                }
                NetworkEvent::AmmunitionEquipped { inventory_index } => {
                    crate::logging::log_debug!("ammunition equipped from inventory index {:?}", inventory_index);
                }
                NetworkEvent::UserCount { player_count } => {
                    self.push_chat_information(format!("{player_count} players are online."));
                }
                NetworkEvent::MapServerBan { error_code } => {
                    let reason = match error_code {
                        1 => "the server was closed",
                        2 => "this account is already logged in",
                        3 => "connection timed out",
                        4 => "the server is full",
                        5 => "account restriction",
                        8 => "the server still recognizes the previous connection, please try again",
                        9 => "too many connections from this IP",
                        _ => "the server closed the connection",
                    };

                    self.push_chat_information(format!("Disconnected: {reason}."));
                    self.session.map_server_disconnected();
                    self.return_to_character_selection();
                }
                NetworkEvent::RefuseEnter { error_code } => {
                    self.push_chat_information(format!("The map server refused the login (error {error_code})."));
                }
                NetworkEvent::SpiritOrbs { entity_id, amount } => {
                    self.client_state.follow_mut(client_state().spirit_orbs()).set(entity_id, amount);
                }
                NetworkEvent::SkillCooldown { skill_id, until } => {
                    self.client_state
                        .follow_mut(client_state().skill_cooldowns())
                        .start(skill_id, until);
                }
                NetworkEvent::AttackRangeChanged { attack_range } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.attack_range = attack_range;
                    }
                }
                NetworkEvent::EntityStopped { entity_id, position } => {
                    let Some(map) = &self.map else {
                        continue;
                    };

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.stop_movement();
                        entity.set_position(map, position, client_tick);
                    }
                }
                NetworkEvent::MinimapMarker {
                    npc_id,
                    marker_type,
                    position,
                    id: _,
                    color,
                } => {
                    let minimap = self.client_state.follow_mut(client_state().minimap());

                    match marker_type {
                        MarkerType::RemoveMark => minimap.remove_marker(npc_id),
                        MarkerType::DisplayFor15Seconds | MarkerType::DisplayUntilLeave => {
                            let expires_at = match marker_type {
                                MarkerType::DisplayFor15Seconds => Some(client_tick.0.wrapping_add(15_000)),
                                MarkerType::DisplayUntilLeave => None,
                                MarkerType::RemoveMark => unreachable!(),
                            };

                            minimap.add_marker(crate::state::minimap::MinimapMarker {
                                entity_id: npc_id,
                                position: TilePosition {
                                    x: position.x.min(u16::MAX as u32) as u16,
                                    y: position.y.min(u16::MAX as u32) as u16,
                                },
                                color: [color.red, color.green, color.blue, color.alpha],
                                expires_at,
                            });
                        }
                    }
                }
                NetworkEvent::MapProperties { map_type, flags } => {
                    *self.client_state.follow_mut(client_state().map_type()) = map_type;
                    *self.client_state.follow_mut(client_state().map_flags()) = flags;
                }
                NetworkEvent::CartInfo {
                    current_count,
                    maximum_count,
                    current_weight,
                    maximum_weight,
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.cart_items = current_count.max(0) as u16;
                        player.cart_maximum_items = maximum_count;
                        player.cart_weight = current_weight.max(0) as u32;
                        player.cart_maximum_weight = maximum_weight.max(0) as u32;
                    }
                }
                NetworkEvent::MapServerAuthFailed { reason } => {
                    crate::logging::log_warn!("map server rejected the login (ZC_AUTH_FAIL reason {reason})");
                    self.interface.open_window(ErrorWindow::new(format!(
                        "The map server rejected the connection (reason {reason}). Please try again."
                    )));
                }
                NetworkEvent::ShowEquipmentWindow { open } => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match (open, self.interface.is_window_with_class_open(WindowClass::Equipment)) {
                            (true, false) => self.interface.open_window(EquipmentWindow::new(
                                client_state().inventory().items(),
                                this_player().manually_asserted(),
                            )),
                            (false, true) => self.interface.close_window_with_class(WindowClass::Equipment),
                            _ => {}
                        }
                    }
                }
                NetworkEvent::CriticalWeight { percentage } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.critical_weight_percentage = percentage;
                    }
                }
                NetworkEvent::MapCellChanged {
                    position,
                    cell_type,
                    map_name,
                } => {
                    // rAthena cell type 0 = walkable, everything else blocks.
                    // The loaded map is an immutable Arc<Map>, so the override
                    // is recorded in the state and logged (collision follow-up).
                    let walkable = cell_type == 0;
                    let overrides = self.client_state.follow_mut(client_state().map_cell_overrides());

                    match overrides.iter_mut().find(|(tile, _)| *tile == position) {
                        Some((_, current)) => *current = walkable,
                        None => overrides.push((position, walkable)),
                    }

                    crate::logging::log_debug!(
                        "map cell changed: {} ({}, {}) -> type {}",
                        map_name,
                        position.x,
                        position.y,
                        cell_type
                    );
                }
                NetworkEvent::MessageString { message_id } => {
                    let text = match self.message_table.as_ref().and_then(|table| table.get(message_id)) {
                        Some(text) => text.to_owned(),
                        None => format!("Message #{message_id}"),
                    };

                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, MessageColor::Rgb {
                            red: 240,
                            green: 220,
                            blue: 120,
                        }));
                }
            }
        }

        // Apply stat updates that were received before the local player entity
        // existed (e.g. the initial stat burst with the experience needed for the
        // next level). This also re-applies buffered updates after a map change or
        // respawn.
        self.apply_pending_stat_updates();

        self.chat_bubbles.retain_alive(client_tick);
        self.skill_name_texts.retain_alive(client_tick);
    }

    /// Applies stat updates that arrived before the local player entity
    /// existed.
    ///
    /// Called once per frame after the network events were processed; the
    /// buffer is normally empty, so this is almost free.
    fn apply_pending_stat_updates(&mut self) {
        if self.pending_stat_updates.is_empty() {
            return;
        }

        let pending_stat_updates: Vec<StatType> = self.pending_stat_updates.take().collect();

        // The zeny value is displayed by the inventory window, so keep it in the client
        // state (the player stats do not contain it).
        if let Some(zeny) = pending_stat_updates.iter().rev().find_map(|stat_type| match stat_type {
            StatType::Zeny(value) => Some(u64::from(*value)),
            _ => None,
        }) {
            *self.client_state.follow_mut(client_state().zeny()) = Some(zeny);
        }

        let Some(player) = self.client_state.try_follow_mut(this_player()) else {
            // Still no player; keep the updates buffered for a later frame.
            for stat_type in pending_stat_updates {
                self.pending_stat_updates.push(stat_type);
            }
            return;
        };

        for stat_type in pending_stat_updates {
            player.update_stat(stat_type);
        }
    }

    /// Uses a learned skill: self cast skills are cast immediately, skills that
    /// require a target switch to the target selection mouse mode, like in the
    /// original client. Passive skills are ignored.
    fn use_skill(
        &mut self,
        slot: Option<HotbarSlot>,
        skill_id: SkillId,
        skill_type: SkillType,
        level: SkillLevel,
        client_tick: ClientTick,
    ) {
        if self.aftercast_act_active(client_tick) {
            crate::logging::log_debug!("ignoring skill {}: after-cast act delay still running", skill_id.0);
            return;
        }

        match skill_type {
            SkillType::Passive => self.push_chat_information(format!("Skill {} is a passive skill and cannot be cast.", skill_id.0)),
            SkillType::SelfCast => {
                let entity_id = self.client_state.follow(this_entity().manually_asserted()).get_entity_id();

                match skill_id == ROLLING_CUTTER_ID {
                    true => {
                        let _ = self.networking_system.cast_channeling_skill(skill_id, level, entity_id);
                    }
                    false => {
                        let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                    }
                }
            }
            skill_type => self.interface.set_mouse_mode(MouseMode::Custom {
                mode: MouseInputMode::TargetSkill {
                    slot,
                    skill_id,
                    skill_type,
                    level,
                },
            }),
        }
    }

    /// Returns whether or not the interface is focused.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn process_user_events(
        &mut self,
        input_report: &InputReport,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] delta_time: f32,
    ) -> bool {
        self.interface.process_events(&mut self.input_event_buffer);

        // Left-clicking an item box only prepares a drag; the actual drag starts once
        // the button was held for a moment or the mouse moved a few pixels while held.
        // A quick click releases the prepared state without picking the item up.
        const ITEM_DRAG_HOLD_TIME_MS: u128 = 500;
        const ITEM_DRAG_DISTANCE: f32 = 5.0;

        match self.interface.mouse_mode().prepare_item_drag() {
            Some((source, item)) => {
                let now = std::time::Instant::now();
                let pending = *self.pending_item_drag.get_or_insert((now, input_report.mouse_position));
                let held_time_ms = now.duration_since(pending.0).as_millis();
                let moved_distance = (input_report.mouse_position.left - pending.1.left)
                    .abs()
                    .max((input_report.mouse_position.top - pending.1.top).abs());

                if input_report.mouse_button_released || !input_report.left_mouse_button_down && moved_distance == 0.0 {
                    // The button was released before the thresholds were reached: this
                    // was a plain click, not a drag.
                    self.pending_item_drag = None;
                } else if held_time_ms >= ITEM_DRAG_HOLD_TIME_MS || moved_distance > ITEM_DRAG_DISTANCE {
                    self.pending_item_drag = None;

                    self.interface.set_mouse_mode(MouseMode::Custom {
                        mode: MouseInputMode::MoveItem { item, source },
                    });
                }
            }
            None => self.pending_item_drag = None,
        }

        let interface_has_focus = self.interface.has_focus();

        // Enter submits the login form and confirms the server selection, so the
        // menus can be used without reaching for the mouse.
        if self.input_system.key_pressed(KeyCode::Enter) || self.input_system.key_pressed(KeyCode::NumpadEnter) {
            if self.interface.is_window_with_class_open(WindowClass::Login) {
                let selected_service_path = SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                let service_id = *self.client_state.follow(client_state().login_window().selected_service());
                let username = self.client_state.follow(selected_service_path.username()).clone();
                let password = self.client_state.follow(selected_service_path.password()).clone();

                if !username.is_empty() && !password.is_empty() {
                    self.input_event_buffer.push(InputEvent::LogIn {
                        service_id,
                        username,
                        password,
                    });
                }
            } else if self.interface.is_window_with_class_open(WindowClass::SelectServer)
                && let Some(character_server_information) = self.client_state.follow(client_state().character_servers()).first().cloned()
            {
                self.input_event_buffer.push(InputEvent::SelectServer {
                    character_server_information,
                });
            }
        }

        if self.interface.get_mouse_mode().is_rotating_camera() {
            // TODO: Does this really need to be a InputEvent?
            let rotation = input_report.mouse_delta.width;
            self.input_event_buffer.push(InputEvent::RotateCamera { rotation });
        }

        if !interface_has_focus {
            self.input_system.handle_keyboard_input(
                &mut self.input_event_buffer,
                #[cfg(feature = "debug")]
                self.interface.get_mouse_mode().is_default(),
                #[cfg(feature = "debug")]
                *self.client_state.follow(client_state().render_options().use_debug_camera()),
            );
        }

        let input_events: Vec<InputEvent> = self.input_event_buffer.drain(..).collect();

        for event in input_events {
            crate::logging::log_trace!("input: dispatch {:?}", event);

            // Any user input can change UI state (window toggles, hotkeys,
            // settings, ...), so the cached element layouts are rebuilt on the
            // next layout pass.
            self.interface.invalidate_layouts();

            match event {
                InputEvent::LogIn {
                    service_id,
                    username,
                    password,
                } => {
                    // The selected service can be stale (persisted `recent_service_id`
                    // from an older client info) and its address can fail DNS
                    // resolution. Both must show an error window, never panic.
                    let services = self.client_state.follow(client_state().client_info().services()).clone();
                    let target = match crate::login_server::resolve_login_server(&services, service_id) {
                        Ok(target) => target,
                        Err(error) => {
                            crate::logging::log_warn!("cannot connect to the selected login server: {error:?}");
                            self.interface.open_window(ErrorWindow::new(error.user_message()));
                            continue;
                        }
                    };
                    let socket_address = target.socket_address;
                    let packet_version = target.packet_version;

                    self.saved_login_server_address = Some(socket_address);
                    self.saved_username = username.clone();
                    self.saved_password = password.clone();
                    self.saved_packet_version = packet_version;

                    self.session.transition(SessionEvent::LoginStarted);
                    self.networking_system
                        .connect_to_login_server(packet_version, socket_address, username, password);
                }
                InputEvent::SelectServer {
                    character_server_information,
                } => {
                    let Some(login_data) = self.saved_login_data else {
                        // The window was still open although we are not logged in (e.g.
                        // after a stale state). Go back to the login screen instead of
                        // panicking.
                        self.interface.close_window_with_class(WindowClass::SelectServer);
                        self.interface.open_window(LoginWindow::new(
                            client_state().login_window(),
                            client_state().login_settings(),
                            client_state().client_info(),
                        ));
                        continue;
                    };

                    self.saved_character_server = Some(character_server_information.clone());

                    self.networking_system.disconnect_from_login_server();

                    // `connect_to_character_server` replaces a stale connection itself, so
                    // clicking the server always starts a fresh character server session.
                    self.networking_system.connect_to_character_server(
                        self.saved_packet_version,
                        &login_data,
                        character_server_information,
                    );
                }
                InputEvent::Respawn => {
                    let _ = self.networking_system.respawn();
                    self.interface.close_window_with_class(WindowClass::Respawn);
                }
                InputEvent::LogOut => {
                    let _ = self.networking_system.log_out();
                }
                InputEvent::LogOutCharacter => {
                    self.networking_system.disconnect_from_character_server();
                }
                InputEvent::Exit => SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst),
                InputEvent::ZoomCamera { zoom_factor } => self.player_camera.soft_zoom(zoom_factor),
                InputEvent::RotateCamera { rotation } => self.player_camera.soft_rotate(rotation),
                InputEvent::ResetCameraRotation => self.player_camera.reset_rotation(),
                InputEvent::ToggleMenuWindow => {
                    // Escape closes the item info window and the character context
                    // menu before it does anything else.
                    if self.interface.is_window_with_class_open(WindowClass::SkillDescription) {
                        self.interface.close_window_with_class(WindowClass::SkillDescription);
                    } else if self.interface.is_window_with_class_open(WindowClass::ItemInfo) {
                        self.interface.close_window_with_class(WindowClass::ItemInfo);
                    } else if self.interface.is_window_with_class_open(WindowClass::ContextMenu) {
                        self.interface.close_window_with_class(WindowClass::ContextMenu);
                    // Escape cancels an active skill target selection instead
                    // of opening the menu, like in the
                    // original client.
                    } else if self.interface.get_mouse_mode().skill_target().is_some() {
                        self.interface.set_mouse_mode(MouseMode::Default);
                    } else if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Menu) {
                            true => self.interface.close_window_with_class(WindowClass::Menu),
                            false => self.interface.open_window(MenuWindow),
                        }
                    }
                }
                InputEvent::ToggleInventoryWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Inventory) {
                            true => self.interface.close_window_with_class(WindowClass::Inventory),
                            false => self.interface.open_window(InventoryWindow::new(
                                client_state().inventory().items(),
                                client_state().interface_settings(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleEquipmentWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Equipment) {
                            true => self.interface.close_window_with_class(WindowClass::Equipment),
                            false => self.interface.open_window(EquipmentWindow::new(
                                client_state().inventory().items(),
                                this_player().manually_asserted(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleSkillTreeWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::SkillTree) {
                            true => self.interface.close_window_with_class(WindowClass::SkillTree),
                            false => self.interface.open_window(SkillTreeWindow::new(
                                client_state().skill_tree_window(),
                                client_state().skill_tree().layout(),
                                client_state().skill_tree().skills(),
                                this_player().manually_asserted().skill_points(),
                            )),
                        }
                    }
                }
                InputEvent::ToggleStatsWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Stats) {
                            true => self.interface.close_window_with_class(WindowClass::Stats),
                            false => self.interface.open_window(StatsWindow::new(this_player().manually_asserted())),
                        }
                    }
                }
                InputEvent::ToggleGameSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GameSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GameSettings),
                    false => self.interface.open_window(GameSettingsWindow::new(client_state().game_settings())),
                },
                InputEvent::ToggleInterfaceSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::InterfaceSettings)
                {
                    true => {
                        crate::logging::log_info!("interface: closing interface settings window");
                        self.interface.close_window_with_class(WindowClass::InterfaceSettings);
                    }
                    false => {
                        let interface_settings = self.client_state.follow(client_state().interface_settings()).clone();
                        crate::logging::log_info!(
                            "interface: opening interface settings window (in_game_theme={:?}, scaling={})",
                            interface_settings.in_game_theme,
                            interface_settings.scaling.get_factor(),
                        );
                        self.interface.open_window(InterfaceSettingsWindow::new(
                            client_state().interface_settings(),
                            client_state().interface_settings_capabilities(),
                        ));
                    }
                },
                InputEvent::ToggleGraphicsSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GraphicsSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GraphicsSettings),
                    false => self.interface.open_window(GraphicsSettingsWindow::new(
                        client_state().graphics_settings(),
                        client_state().graphics_settings_capabilities(),
                    )),
                },
                InputEvent::ToggleAudioSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::AudioSettings) {
                    true => {
                        // The volumes are saved on change already; save again on close so
                        // the file is up to date even if the last change was not picked up
                        // by `update_settings` yet.
                        self.client_state.follow(client_state().audio_settings()).save();
                        self.interface.close_window_with_class(WindowClass::AudioSettings);
                    }
                    false => self
                        .interface
                        .open_window(AudioSettingsWindow::new(client_state().audio_settings())),
                },
                InputEvent::ToggleMinimapWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        self.interface.close_window_with_class(WindowClass::BigMap);

                        match self.interface.is_window_with_class_open(WindowClass::Minimap) {
                            true => {
                                self.interface.close_window_with_class(WindowClass::Minimap);
                                self.map_window_cycle = MapWindowCycle::Closed;
                            }
                            false => {
                                self.interface.open_window(MinimapWindow);
                                self.map_window_cycle = MapWindowCycle::Minimap;
                            }
                        }
                    }
                }
                InputEvent::CycleMapWindows => {
                    // Text inputs (chat, login, character name, ...) get the M
                    // key, the map cycle must not steal it while they are focused.
                    if !self.interface.has_focus() && self.client_state.try_follow(this_entity()).is_some() {
                        Self::cycle_map_windows(&mut self.interface, &mut self.map_window_cycle);
                    }
                }
                InputEvent::ToggleFriendListWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::FriendList) {
                            true => self.interface.close_window_with_class(WindowClass::FriendList),
                            false => self.interface.open_window(FriendListWindow::new(
                                client_state().friend_list_window(),
                                client_state().friend_list(),
                            )),
                        }
                    }
                }
                InputEvent::TogglePartyWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Party) {
                            true => self.interface.close_window_with_class(WindowClass::Party),
                            false => self.interface.open_window(PartyWindow),
                        }
                    }
                }
                InputEvent::CreateParty { name } => {
                    self.client_state.follow_mut(client_state().party().create_name_input()).clear();
                    let _ = self.networking_system.create_party(name);
                }
                InputEvent::InviteToParty { name } => {
                    self.client_state.follow_mut(client_state().party().invite_name_input()).clear();
                    let _ = self.networking_system.invite_to_party(name);
                }
                InputEvent::AcceptPartyInvite { party_id } => {
                    let _ = self.networking_system.reply_party_invite(party_id, true);

                    let party = self.client_state.follow_mut(client_state().party());
                    party.party_id = Some(party_id);
                    party.invite = None;

                    self.interface.close_window_with_class(WindowClass::PartyInvite);

                    self.open_party_window();
                }
                InputEvent::RejectPartyInvite { party_id } => {
                    let _ = self.networking_system.reply_party_invite(party_id, false);
                    *self.client_state.follow_mut(client_state().party().invite()) = None;
                    self.interface.close_window_with_class(WindowClass::PartyInvite);
                }
                InputEvent::LeaveParty => {
                    let _ = self.networking_system.leave_party();

                    self.client_state.follow_mut(client_state().party()).clear();
                    self.interface.close_window_with_class(WindowClass::Party);
                }
                InputEvent::ToggleBigMapWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        self.interface.close_window_with_class(WindowClass::Minimap);

                        match self.interface.is_window_with_class_open(WindowClass::BigMap) {
                            true => {
                                self.interface.close_window_with_class(WindowClass::BigMap);
                                self.map_window_cycle = MapWindowCycle::Closed;
                            }
                            false => {
                                self.interface.open_window(BigMapWindow);
                                self.map_window_cycle = MapWindowCycle::BigMap;
                            }
                        }
                    }
                }
                InputEvent::ToggleChatWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Chat) {
                            true => self.interface.close_window_with_class(WindowClass::Chat),
                            false => self.interface.open_window(ChatWindow::new(
                                client_state().chat_window(),
                                client_state().chat_messages(),
                                client_state().interface_settings(),
                            )),
                        }
                    }
                }
                InputEvent::CloseTopWindow => self.interface.close_top_window(&self.client_state),
                InputEvent::ToggleShowInterface => self.show_interface = !self.show_interface,
                InputEvent::SelectCharacter { slot } => {
                    crate::logging::log_debug!(
                        "character selection: requesting slot {} (character server connected: {})",
                        slot,
                        self.networking_system.is_character_server_connected()
                    );
                    if self.networking_system.select_character(slot).is_err() {
                        crate::logging::log_warn!("character selection: could not send the select packet (not connected)");
                    }
                }
                InputEvent::OpenCharacterCreationWindow { slot } => {
                    // Clear the name before opening the window.
                    self.client_state.follow_mut(client_state().create_character_name()).clear();

                    self.interface
                        .open_window(CharacterCreationWindow::new(client_state().create_character_name(), slot))
                }
                InputEvent::CreateCharacter { slot, name } => {
                    let _ = self.networking_system.create_character(slot, name);
                }
                InputEvent::OpenCharacterDeleteWindow { character_id } => {
                    // Never keep the e-mail of a previous attempt around.
                    self.client_state.follow_mut(client_state().delete_character_email()).clear();
                    self.interface.open_window(CharacterDeletionWindow::new(
                        character_id,
                        client_state().delete_character_email(),
                    ));
                }
                InputEvent::DeleteCharacter { character_id, email } => {
                    if self.client_state.follow(client_state().currently_deleting()).is_none() {
                        let _ = self.networking_system.delete_character(character_id, email);
                        *self.client_state.follow_mut(client_state().currently_deleting()) = Some(character_id);
                        self.interface.close_window_with_class(WindowClass::CharacterDeletion);
                    }
                }
                InputEvent::SwitchCharacterSlot {
                    origin_slot,
                    destination_slot,
                } => {
                    let _ = self.networking_system.switch_character_slot(origin_slot, destination_slot);
                }
                InputEvent::PlayerMove { destination } => {
                    if self.movement_blocked_by_cast(client_tick) {
                        // Classic cast rooting: while the local player is in a
                        // server-confirmed cast with a cast time, walk requests
                        // are ignored (the server would ignore them anyway).
                        // Instant skills never reach this branch because they
                        // do not start a cast bar.
                        continue;
                    }

                    if self.client_state.try_follow(this_entity()).is_some() {
                        let _ = self.networking_system.player_move(WorldPosition {
                            x: destination.x,
                            y: destination.y,
                            direction: Direction::North,
                        });
                    }

                    // Unbuffer any buffered action.
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;
                }
                InputEvent::OpenCharacterContextMenu { entity_id, position } => {
                    if open_character_context_menu(&self.client_state, &mut self.interface, entity_id, position) {
                        self.pending_context_menu = Some((entity_id, position));
                        let _ = self.networking_system.entity_details(entity_id);
                    }
                }
                InputEvent::CloseContextMenu => {
                    self.interface.close_window_with_class(WindowClass::ContextMenu);
                }
                InputEvent::OpenInventoryItemMenu { inventory_index } => {
                    open_inventory_item_menu(
                        &self.client_state,
                        &mut self.interface,
                        inventory_index,
                        input_report.mouse_position,
                    );
                }
                InputEvent::OpenStorageItemMenu { storage_index } => {
                    open_storage_item_menu(
                        &self.client_state,
                        &mut self.interface,
                        storage_index,
                        input_report.mouse_position,
                    );
                }
                InputEvent::DepositInventoryItem { inventory_index, amount } => {
                    let _ = self.networking_system.storage_deposit(inventory_index, amount);
                }
                InputEvent::WithdrawStorageItem { storage_index, amount } => {
                    // The server expects the storage slot encoded as `slot + 1`.
                    let _ = self.networking_system.storage_withdraw(storage_index.0, amount);
                }
                InputEvent::CloseStorage => {
                    let _ = self.networking_system.close_storage();
                    self.close_storage_window();
                }
                InputEvent::ItemDoubleClick { source, item } => {
                    // Double clicking is the quick way to move a whole stack while
                    // the Kafra storage window is open.
                    if self.interface.is_window_with_class_open(WindowClass::Storage) {
                        match source {
                            ItemSource::Inventory => {
                                let _ = self.networking_system.storage_deposit(item.index, item_amount(&item).max(1));
                            }
                            ItemSource::Storage => {
                                let _ = self.networking_system.storage_withdraw(item.index.0, item_amount(&item).max(1));
                            }
                            ItemSource::Equipment { .. } => {}
                        }
                        continue;
                    }

                    // Otherwise double clicking an inventory item uses consumables and
                    // tries to equip equipment; the server validates requirements.
                    if let ItemSource::Inventory = source {
                        match item_use_action(&item) {
                            Some(ItemUseAction::Use) if item.is_identified() => {
                                let _ = self.networking_system.use_item(item.index, item.item_id);
                            }
                            Some(ItemUseAction::Equip(equip_position)) => {
                                let _ = self.networking_system.request_item_equip(item.index, equip_position);
                            }
                            _ => {}
                        }
                    }
                }
                InputEvent::OpenItemInfo { inventory_index, source } => {
                    open_item_info_window(
                        &self.client_state,
                        &mut self.interface,
                        inventory_index,
                        source,
                        &self.library,
                        &self.texture_loader,
                    );
                }
                InputEvent::CloseItemInfo => {
                    self.interface.close_window_with_class(WindowClass::ItemInfo);
                }
                InputEvent::UseInventoryItem { inventory_index, item_id } => {
                    let _ = self.networking_system.use_item(inventory_index, item_id);
                }
                InputEvent::EquipInventoryItem {
                    inventory_index,
                    equip_position,
                } => {
                    let _ = self.networking_system.request_item_equip(inventory_index, equip_position);
                }
                InputEvent::WhisperChat { name } => {
                    if !self.interface.is_window_with_class_open(WindowClass::Chat) {
                        self.interface.open_window(ChatWindow::new(
                            client_state().chat_window(),
                            client_state().chat_messages(),
                            client_state().interface_settings(),
                        ));
                    }

                    self.client_state
                        .follow_mut(client_state().chat_window())
                        .set_current_text(format!("/w {name} "));
                    self.interface.focus_element(ChatTextBox);
                }
                InputEvent::PlayerInteract { entity_id } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        let _ = match entity.get_entity_type() {
                            EntityType::Npc => self.networking_system.start_dialog(entity_id),
                            EntityType::Monster => {
                                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                                let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                                if auto_attack {
                                    *buffered_action = Some(BufferedAction::AttackEntity { entity_id });
                                }

                                self.networking_system.player_attack(entity_id)
                            }
                            EntityType::Warp => self.networking_system.player_move({
                                let position = entity.get_tile_position();
                                WorldPosition {
                                    x: position.x,
                                    y: position.y,
                                    direction: Direction::North,
                                }
                            }),
                            _ => Ok(()),
                        };
                    }
                }
                InputEvent::PickUpItem { entity_id } => {
                    self.mouse_cursor.set_state(MouseCursorState::PickUpItem, client_tick);

                    if let Some(map) = &self.map {
                        let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                        let item_position = self
                            .client_state
                            .follow(client_state().ground_items())
                            .iter()
                            .find(|item| item.get_entity_id() == entity_id)
                            .map(|item| item.get_tile_position());

                        if let (Some(player_position), Some(item_position)) = (player_position, item_position) {
                            if player_position
                                .x
                                .abs_diff(item_position.x)
                                .max(player_position.y.abs_diff(item_position.y))
                                <= ITEM_PICKUP_RANGE.0
                            {
                                let _ = self.networking_system.pick_up_item(entity_id);

                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            } else if let Some(path) =
                                self.path_finder
                                    .find_walkable_path_in_range(&**map, player_position, item_position, ITEM_PICKUP_RANGE)
                                && let Some(nearest_tile) = path.last()
                            {
                                let _ = self.networking_system.player_move(WorldPosition {
                                    x: nearest_tile.x,
                                    y: nearest_tile.y,
                                    direction: Direction::North,
                                });

                                *self.client_state.follow_mut(client_state().buffered_action()) =
                                    Some(BufferedAction::PickUpItem { entity_id });
                            } else {
                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::WarpToMap { map_name, position } => {
                    let _ = self.networking_system.warp_to_map(map_name, position);
                }
                InputEvent::SendMessage { text } => {
                    // Handle special client commands.
                    if text.as_str() == "/nc" {
                        let auto_attack = self.client_state.follow_mut(client_state().game_settings().auto_attack());
                        *auto_attack = !*auto_attack;
                        continue;
                    }

                    // Local aura style selection (`/aura <style>`). This is handled
                    // entirely client-side before the slash command translation and is
                    // never sent to the server.
                    if let Some(style_name) = text.trim().strip_prefix("/aura").map(str::trim) {
                        // Only the first word is the style, extra words are ignored.
                        let style_name = style_name.split_whitespace().next().unwrap_or_default();

                        match AuraStyle::from_style_name(style_name) {
                            Some(style) => self.set_aura_style(style),
                            None if style_name.is_empty() => {
                                self.push_chat_information(format!("Usage: /aura <style>. Available styles: {}.", AuraStyle::style_names()))
                            }
                            None => self.push_chat_information(format!(
                                "Unknown aura style \"{style_name}\". Available styles: {}.",
                                AuraStyle::style_names()
                            )),
                        }

                        continue;
                    }

                    // Whisper command, filled in by the character context menu:
                    // "/w <name> <message>". The original client sends a dedicated
                    // whisper packet for this instead of a global chat message.
                    if let Some(whisper) = text.strip_prefix("/w ").or_else(|| text.strip_prefix("/whisper ")) {
                        let mut parts = whisper.splitn(2, ' ');
                        let target_name = parts.next().unwrap_or_default();
                        let message = parts.next().unwrap_or_default().trim_start();

                        if !target_name.is_empty() && !message.is_empty() {
                            let _ = self.networking_system.send_whisper(target_name, message);
                        }

                        continue;
                    }

                    // Party chat: "/p <message>" or "/party <message>". The server
                    // echoes the message back to the whole party (including the
                    // sender), so no local echo is necessary.
                    if let Some(message) = text.strip_prefix("/p ").or_else(|| text.strip_prefix("/party ")) {
                        let message = message.trim();

                        if message.is_empty() {
                            self.push_chat_information("Usage: /p <message>".to_owned());
                        } else if self.client_state.follow(client_state().party()).is_in_party() {
                            let player_name = self.client_state.follow(client_state().player_name()).clone();
                            let _ = self.networking_system.send_party_chat_message(&player_name, message);
                        } else {
                            self.push_chat_information("You are not in a party.".to_owned());
                        }

                        continue;
                    }

                    // Official-client style slash commands (`/go prontera`,
                    // `/warp ...`) are translated into rAthena atcommands
                    // (`@go prontera`, ...); everything else is sent unchanged.
                    let translated_command = translate_slash_command(&text);

                    let outgoing_text = translated_command.as_deref().unwrap_or(&text);

                    let _ = self
                        .networking_system
                        .send_chat_message(self.client_state.follow(client_state().player_name()), outgoing_text);

                    // The server sends ZC_NOTIFY_CHAT (0x008D) only to the
                    // surrounding players (AREA_CHAT_WOC excludes the speaker),
                    // so our own speech bubble has to be created locally.
                    // Commands produce no speech: their output is sent by the
                    // server as a regular chat message instead.
                    if translated_command.is_none()
                        && let Some(entity_id) = self.client_state.try_follow(this_entity()).map(|entity| entity.get_entity_id())
                    {
                        self.chat_bubbles.push(entity_id, text, MessageColor::Broadcast, client_tick);
                    }
                }
                InputEvent::NextDialog { npc_id } => {
                    let _ = self.networking_system.next_dialog(npc_id);
                }
                InputEvent::CloseDialog { npc_id } => {
                    let _ = self.networking_system.close_dialog(npc_id);
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);
                }
                InputEvent::ChooseDialogOption { npc_id, option } => {
                    let _ = self.networking_system.choose_dialog_option(npc_id, option);

                    // The official client closes its menu window as soon as an
                    // option is picked; the dialog window itself stays open. In
                    // our combined dialog window this means that the choice
                    // buttons have to disappear right away. Otherwise the next
                    // menu (or the answer of the NPC) would be appended below
                    // the options of the previous menu.
                    self.client_state.follow_mut(client_state().dialog_window()).clear_choices();

                    if option == -1 {
                        self.interface.close_window_with_class(WindowClass::Dialog);
                    }
                }
                InputEvent::MoveItem { source, destination, item } => match (source, destination) {
                    (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                        let _ = self.networking_system.request_item_equip(item.index, position);
                    }
                    (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                        let _ = self.networking_system.request_item_unequip(item.index);
                    }
                    (ItemSource::Inventory, ItemSource::Storage) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.storage_deposit(item.index, amount);
                    }
                    (ItemSource::Storage, ItemSource::Inventory) => {
                        let amount = item_amount(&item).max(1);
                        let _ = self.networking_system.storage_withdraw(item.index.0, amount);
                    }
                    _ => {}
                },
                InputEvent::SendEmotion { emotion } => {
                    let _ = self.networking_system.send_emotion(emotion);
                }
                InputEvent::MoveSkill {
                    source,
                    destination,
                    skill,
                } => match (source, destination) {
                    (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .update_slot(&mut self.networking_system, slot, skill);
                    }
                    (SkillSource::Hotbar { slot }, SkillSource::SkillTree) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .clear_slot(&mut self.networking_system, slot);
                    }
                    (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                        self.client_state.follow_mut(client_state().hotbar()).swap_slot(
                            &mut self.networking_system,
                            source_slot,
                            destination_slot,
                        );
                    }
                    _ => {}
                },
                InputEvent::OpenSkillDescription {
                    name,
                    learned_level,
                    maximum_level,
                    skill_type,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                    position,
                } => {
                    self.interface.close_window_with_class(WindowClass::SkillDescription);
                    self.interface.open_window_at(
                        SkillDescriptionWindow::new(
                            name,
                            learned_level,
                            maximum_level,
                            skill_type,
                            spell_point_cost,
                            attack_range,
                            upgradable,
                        ),
                        position,
                    );
                }
                InputEvent::CycleHotbarTab { delta } => {
                    self.client_state.follow_mut(client_state().hotbar()).cycle_tab(delta);

                    let tab = self.client_state.follow(client_state().hotbar()).active_tab();
                    self.push_chat_information(format!("Hotbar page {}/{}.", tab + 1, HOTBAR_TAB_COUNT));
                }
                InputEvent::UseSkillInSlot { slot } => {
                    // Item hotkeys use the first matching inventory item, like the
                    // original client uses the first stack of that item.
                    if let Some(item) = self.client_state.follow(client_state().hotbar()).item_in_slot(slot) {
                        let inventory_index = self
                            .client_state
                            .follow(client_state().inventory().items())
                            .iter()
                            .find(|inventory_item| inventory_item.item_id == item.item_id)
                            .map(|inventory_item| inventory_item.index);

                        match inventory_index {
                            Some(inventory_index) => {
                                let _ = self.networking_system.use_item(inventory_index, item.item_id);
                            }
                            None => self.push_chat_information("The item of that hotkey is not in your inventory.".to_owned()),
                        }

                        continue;
                    }

                    // Skills that require a target switch to a target selection
                    // mouse mode, while self cast skills are used immediately,
                    // like in the original client.
                    let skill = self
                        .client_state
                        .follow(client_state().hotbar())
                        .get_skill_in_slot(slot)
                        .as_ref()
                        .and_then(|learnable_skill| {
                            self.client_state
                                .follow(client_state().skill_tree().skills())
                                .iter()
                                .find(|learned_skill| learned_skill.skill_id == learnable_skill.skill_id)
                                .map(|learned_skill| {
                                    // The character may have the skill at a lower level than
                                    // the hotbar slot was created for (the skill was
                                    // re-learned or copied onto a character that cannot max
                                    // it); in that case cast the level the character really
                                    // has instead of silently doing nothing.
                                    let level = SkillLevel(learned_skill.skill_level.0.min(learnable_skill.maximum_level.0));
                                    (learned_skill.skill_id, learned_skill.skill_type, level)
                                })
                        });

                    match skill {
                        Some((skill_id, skill_type, level)) => {
                            self.use_skill(Some(slot), skill_id, skill_type, level, client_tick);
                        }
                        None => {
                            if let Some(learnable_skill) =
                                self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
                            {
                                self.push_chat_information(format!("You have not learned skill {}.", learnable_skill.skill_id.0));
                            }
                        }
                    }
                }
                InputEvent::UseSkill { skill_id, level } => {
                    // Double clicking a skill in the skill tree uses it, like in
                    // the original client. Unlearned and passive skills are
                    // ignored.
                    let skill = self
                        .client_state
                        .follow(client_state().skill_tree().skills())
                        .iter()
                        .find(|learned_skill| learned_skill.skill_id == skill_id)
                        .map(|learned_skill| (learned_skill.skill_type, SkillLevel(level.0.min(learned_skill.skill_level.0))));

                    match skill {
                        Some((skill_type, level)) => self.use_skill(None, skill_id, skill_type, level, client_tick),
                        None => self.push_chat_information(format!("You have not learned skill {}.", skill_id.0)),
                    }
                }
                InputEvent::CastSkill {
                    skill_id,
                    skill_type,
                    level,
                } => {
                    // The skill type comes from the skill the player selected (hotbar or
                    // skill tree); the learned list is only used for the range, with a
                    // fallback so a cast is never silently dropped just because the
                    // learned entry is missing (e.g. high-job/trimmed skill trees).
                    let attack_range = self
                        .client_state
                        .follow(client_state().skill_tree().skills())
                        .iter()
                        .find(|learned_skill| learned_skill.skill_id == skill_id)
                        .map(|learned_skill| learned_skill.attack_range)
                        .unwrap_or(AttackRange(1));

                    if crate::logging::enabled(crate::logging::LEVEL_DEBUG) {
                        crate::logging::log_debug!(
                            "cast skill {:?} level {} type {:?} range {} target {:?}",
                            skill_id,
                            level.0,
                            skill_type,
                            attack_range.0,
                            input_report.mouse_target
                        );
                    }

                    {
                        match skill_type {
                            SkillType::Passive => {
                                self.push_chat_information(format!("Skill {} is a passive skill and cannot be cast.", skill_id.0));
                            }
                            SkillType::Attack => {
                                if let PickerTarget::Entity(entity_id) = input_report.mouse_target
                                    && let Some(map) = &self.map
                                {
                                    let player_position =
                                        self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                                    let target_position = self
                                        .client_state
                                        .follow(client_state().entities())
                                        .iter()
                                        .find(|entity| entity.get_entity_id() == entity_id)
                                        .map(|entity| entity.get_tile_position());

                                    if let (Some(player_position), Some(target_position)) = (player_position, target_position) {
                                        if player_position
                                            .x
                                            .abs_diff(target_position.x)
                                            .max(player_position.y.abs_diff(target_position.y))
                                            <= attack_range.0
                                        {
                                            let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                        } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                            &**map,
                                            player_position,
                                            target_position,
                                            attack_range,
                                        ) && let Some(nearest_tile) = path.last()
                                        {
                                            // The target is out of range, so walk towards it and
                                            // send the cast right away. The server remembers skill
                                            // requests made while walking and casts as soon as the
                                            // target is in range, following the target's position
                                            // at every step.
                                            let _ = self.networking_system.player_move(WorldPosition {
                                                x: nearest_tile.x,
                                                y: nearest_tile.y,
                                                direction: Direction::North,
                                            });
                                            let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                        } else {
                                            self.push_chat_information(format!("Skill {}: the target is out of range.", skill_id.0));
                                        }
                                    } else {
                                        self.push_chat_information(format!("Skill {}: the target is no longer in view.", skill_id.0));
                                    }
                                } else {
                                    crate::logging::log_debug!(
                                        "cast skill {:?}: no entity target or map, dropping the attack cast",
                                        skill_id
                                    );
                                    self.push_chat_information(format!("Skill {}: select a target first.", skill_id.0));
                                }
                            }
                            SkillType::Ground | SkillType::Trap => {
                                if let PickerTarget::Tile { x, y } = input_report.mouse_target {
                                    let target_position = TilePosition { x, y };

                                    if let Some(map) = &self.map {
                                        let player_position =
                                            self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());

                                        if let Some(player_position) = player_position {
                                            if player_position
                                                .x
                                                .abs_diff(target_position.x)
                                                .max(player_position.y.abs_diff(target_position.y))
                                                <= attack_range.0
                                            {
                                                let _ = self.networking_system.cast_ground_skill(skill_id, level, target_position);
                                            } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                                &**map,
                                                player_position,
                                                target_position,
                                                attack_range,
                                            ) && let Some(nearest_tile) = path.last()
                                            {
                                                // The target is out of range, so walk towards it and
                                                // send the cast right away. The server remembers
                                                // skill requests made while walking and casts as
                                                // soon as the position is in range.
                                                let _ = self.networking_system.player_move(WorldPosition {
                                                    x: nearest_tile.x,
                                                    y: nearest_tile.y,
                                                    direction: Direction::North,
                                                });
                                                let _ = self.networking_system.cast_ground_skill(skill_id, level, target_position);
                                            } else {
                                                self.push_chat_information(format!("Skill {}: that cell is out of range.", skill_id.0));
                                            }
                                        } else {
                                            self.push_chat_information(format!(
                                                "Skill {}: the player position is not known yet.",
                                                skill_id.0
                                            ));
                                        }
                                    }
                                } else {
                                    crate::logging::log_debug!(
                                        "cast skill {:?}: ground skill without a tile target, dropping the cast",
                                        skill_id
                                    );
                                    self.push_chat_information(format!("Skill {}: click a cell on the ground to cast it.", skill_id.0));
                                }
                            }
                            SkillType::SelfCast => match skill_id == ROLLING_CUTTER_ID {
                                true => {
                                    let _ = self.networking_system.cast_channeling_skill(
                                        skill_id,
                                        level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                                false => {
                                    let _ = self.networking_system.cast_skill(
                                        skill_id,
                                        level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                            },
                            SkillType::Support => {
                                if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                                    if let Some(map) = &self.map {
                                        let player_position =
                                            self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                                        let target_position = self
                                            .client_state
                                            .follow(client_state().entities())
                                            .iter()
                                            .find(|entity| entity.get_entity_id() == entity_id)
                                            .map(|entity| entity.get_tile_position());

                                        if let (Some(player_position), Some(target_position)) = (player_position, target_position) {
                                            if player_position
                                                .x
                                                .abs_diff(target_position.x)
                                                .max(player_position.y.abs_diff(target_position.y))
                                                <= attack_range.0
                                            {
                                                let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                            } else if let Some(path) = self.path_finder.find_walkable_path_in_range(
                                                &**map,
                                                player_position,
                                                target_position,
                                                attack_range,
                                            ) && let Some(nearest_tile) = path.last()
                                            {
                                                // The target is out of range, so walk towards it and
                                                // send the cast right away. The server remembers skill
                                                // requests made while walking and casts as soon as the
                                                // target is in range, following the target's position
                                                // at every step.
                                                let _ = self.networking_system.player_move(WorldPosition {
                                                    x: nearest_tile.x,
                                                    y: nearest_tile.y,
                                                    direction: Direction::North,
                                                });
                                                let _ = self.networking_system.cast_skill(skill_id, level, entity_id);
                                            } else {
                                                self.push_chat_information(format!("Skill {}: the target is out of range.", skill_id.0));
                                            }
                                        } else {
                                            self.push_chat_information(format!("Skill {}: the target is no longer in view.", skill_id.0));
                                        }
                                    }
                                } else {
                                    crate::logging::log_debug!(
                                        "cast skill {:?}: support skill without an entity target, casting on self",
                                        skill_id
                                    );

                                    let _ = self.networking_system.cast_skill(
                                        skill_id,
                                        level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                            }
                        }
                    }
                }
                InputEvent::StopSkill { slot } => {
                    if let Some(skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
                        && skill.skill_id == ROLLING_CUTTER_ID
                    {
                        let _ = self.networking_system.stop_channeling_skill(skill.skill_id);
                    }
                }
                InputEvent::AddFriend { character_name } => {
                    if character_name.len() > 24 {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] friend name {} is too long", "error".red(), character_name.magenta());
                    } else {
                        let _ = self.networking_system.add_friend(character_name);
                    }
                }
                InputEvent::RemoveFriend { account_id, character_id } => {
                    let _ = self.networking_system.remove_friend(account_id, character_id);
                }
                InputEvent::RejectFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.reject_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::AcceptFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.accept_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::BuyItems { items } => {
                    let _ = self.networking_system.purchase_items(items);
                }
                InputEvent::CloseShop => {
                    let _ = self.networking_system.close_shop();
                    self.shop_open = false;

                    // Clear the carts.
                    self.client_state.follow_mut(client_state().buy_cart()).clear();
                    self.client_state.follow_mut(client_state().sell_cart()).clear();

                    self.interface.close_window_with_class(WindowClass::Buy);
                    self.interface.close_window_with_class(WindowClass::BuyCart);
                    self.interface.close_window_with_class(WindowClass::Sell);
                    self.interface.close_window_with_class(WindowClass::SellCart);
                }
                InputEvent::BuyOrSell { shop_id, buy_or_sell } => {
                    let _ = self.networking_system.select_buy_or_sell(shop_id, buy_or_sell);
                    self.interface.close_window_with_class(WindowClass::BuyOrSell);
                }
                InputEvent::SellItems { items } => {
                    let _ = self.networking_system.sell_items(items);
                }
                InputEvent::StatUp { stat_type } => {
                    let _ = self.networking_system.request_stat_up(stat_type);
                }
                InputEvent::DistributePointsForSkill { skill_id } => {
                    if let Some(available_skill_points) = self.client_state.try_follow(this_player().skill_points()).copied() {
                        let job_id = self.client_state.follow(this_entity().manually_asserted()).get_job_id();
                        let pending_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .len();
                        let available_skill_points = (available_skill_points as usize).saturating_sub(pending_skill_points);

                        let skill_information = self.library.get::<SkillListInformation>(skill_id);
                        let learned_skills = self.client_state.follow(client_state().skill_tree().skills());

                        let mut new_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .clone();

                        let current_skill_level = learned_skills
                            .iter()
                            .find(|skill| skill.skill_id == skill_id)
                            .map(|skill| skill.skill_level.0)
                            .unwrap_or_default()
                            + new_skill_points
                                .iter()
                                .filter(|pending_skill_level| **pending_skill_level == skill_id)
                                .count() as u16;

                        // If the skill is already at max level we don't do anything
                        if current_skill_level < skill_information.maximum_level.0 {
                            let target_skill_level = SkillLevel(current_skill_level + 1);

                            bring_skill_to_level(
                                &mut new_skill_points,
                                &self.library,
                                learned_skills,
                                job_id,
                                skill_id,
                                target_skill_level,
                                available_skill_points,
                            );

                            *self
                                .client_state
                                .follow_mut(client_state().skill_tree_window().pending_skill_points()) = new_skill_points;
                        }
                    }
                }
                InputEvent::LevelUpSkills { skill_ids } => {
                    for skill_id in skill_ids {
                        if self.networking_system.level_up_skill(skill_id).is_err() {
                            break;
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ReloadLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    *self.client_state.follow_mut(client_state().localization()) =
                        Localization::load_language(&self.game_file_loader, language);
                }
                #[cfg(feature = "debug")]
                InputEvent::SaveLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    self.client_state.follow(client_state().localization()).save_language(language);
                }
                #[cfg(feature = "debug")]
                InputEvent::OpenMarkerDetails { marker_identifier } => {
                    if let Some(map) = &self.map {
                        match marker_identifier {
                            MarkerIdentifier::Object(key) => {
                                let inspecting_objects = self.client_state.follow_mut(client_state().inspecting_objects());
                                let object = map.get_object(key);
                                let object_path = state::prepare_object_inspection(inspecting_objects, object);

                                self.interface.open_state_window(object_path);
                            }
                            MarkerIdentifier::LightSource(key) => {
                                let inspecting_lights = self.client_state.follow_mut(client_state().inspecting_light_sources());
                                let light_source = map.get_light_source(key);
                                let light_source_path = state::prepare_light_source_inspection(inspecting_lights, light_source);

                                self.interface.open_state_window(light_source_path);
                            }
                            MarkerIdentifier::SoundSource(index) => {
                                let inspecting_sounds = self.client_state.follow_mut(client_state().inspecting_sound_sources());
                                let sound_source = map.get_sound_source(index);
                                let sound_source_path = state::prepare_sound_source_inspection(inspecting_sounds, sound_source);

                                self.interface.open_state_window(sound_source_path);
                            }
                            MarkerIdentifier::EffectSource(index) => {
                                let inspecting_effects = self.client_state.follow_mut(client_state().inspecting_effect_sources());
                                let effect_source = map.get_effect_source(index);
                                let effect_source_path = state::prepare_effect_source_inspection(inspecting_effects, effect_source);

                                self.interface.open_state_window(effect_source_path);
                            }
                            MarkerIdentifier::Particle(..) => {
                                // TODO:
                            }
                            MarkerIdentifier::Entity(index) => {
                                let entity_id = self
                                    .client_state
                                    .try_follow(client_state().entities().index(index as usize))
                                    .expect("entity should exist")
                                    .get_entity_id();

                                // This can technically still be `None`, violating the API but we handle this
                                // case in the state window.
                                let entity_path = client_state().entities().lookup(entity_id).manually_asserted();

                                self.interface.open_state_window(entity_path);
                            }
                            MarkerIdentifier::Shadow(..) => {
                                // TODO:
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleRenderOptionsWindow => match self.interface.is_window_with_class_open(WindowClass::RenderOptions) {
                    true => self.interface.close_window_with_class(WindowClass::RenderOptions),
                    false => self
                        .interface
                        .open_window(RenderOptionsWindow::new(client_state().render_options())),
                },
                #[cfg(feature = "debug")]
                InputEvent::OpenMapDataWindow => {
                    if let Some(map) = self.map.as_ref() {
                        let inspecting_maps = self.client_state.follow_mut(client_state().inspecting_maps());
                        let map_data_path = state::prepare_map_inspection(inspecting_maps, map.get_map_data());

                        self.interface.open_state_window(map_data_path);
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleClientStateInspectorWindow => {
                    match self.interface.is_window_with_class_open(WindowClass::ClientStateInspector) {
                        true => self.interface.close_window_with_class(WindowClass::ClientStateInspector),
                        false => self.interface.open_state_window_mut(client_state()),
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleMapsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Maps) {
                            true => self.interface.close_window_with_class(WindowClass::Maps),
                            false => self.interface.open_window(MapsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleCommandsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Commands) {
                            true => self.interface.close_window_with_class(WindowClass::Commands),
                            false => self.interface.open_window(CommandsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleThemeInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::ThemeInspector) {
                    true => self.interface.close_window_with_class(WindowClass::ThemeInspector),
                    false => self.interface.open_window(ThemeInspectorWindow::new(
                        client_state().theme_inspector_window(),
                        client_state().menu_theme(),
                        client_state().in_game_theme(),
                        client_state().world_theme(),
                    )),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleProfilerWindow => match self.interface.is_window_with_class_open(WindowClass::Profiler) {
                    true => self.interface.close_window_with_class(WindowClass::Profiler),
                    false => self.interface.open_window(ProfilerWindow::new(client_state().profiler_window())),
                },
                #[cfg(feature = "debug")]
                InputEvent::TogglePacketInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::PacketInspector) {
                    true => self.interface.close_window_with_class(WindowClass::PacketInspector),
                    false => self
                        .interface
                        .open_window(PacketInspectorWindow::new(client_state().packet_history())),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleCacheStatisticsWindow => match self.interface.is_window_with_class_open(WindowClass::CacheStatistics) {
                    true => self.interface.close_window_with_class(WindowClass::CacheStatistics),
                    false => self.interface.open_state_window(client_state().cache_statistics()),
                },
                #[cfg(feature = "debug")]
                InputEvent::CameraLookAround { offset } => self.debug_camera.look_around(offset),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveRight => self.debug_camera.move_right(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveUp => self.debug_camera.move_up(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                InputEvent::CameraDecelerate => self.debug_camera.decelerate(),
                #[cfg(feature = "debug")]
                InputEvent::InspectFrame { measurement } => self.interface.open_window(FrameInspectorWindow::new(measurement)),
            }
        }

        interface_has_focus
    }

    #[cfg(feature = "debug")]
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_debug_windows(&mut self, delta_time: f64) {
        let is_packet_inspector_open = self.interface.is_window_with_class_open(WindowClass::PacketInspector);
        self.client_state
            .follow_mut(client_state().packet_history())
            .update(is_packet_inspector_open);

        self.client_state.follow_mut(client_state().cache_statistics()).update(
            delta_time,
            &self.map_loader,
            &self.texture_loader,
            &self.sprite_loader,
            &self.font_loader,
            &self.audio_engine,
            &self.action_loader,
            &self.animation_loader,
            &self.effect_loader,
        );
    }

    /// Drain finished async loads from the loader thread and apply their
    /// results to client state. This is the only place that promotes
    /// `self.map` from `None` to `Some` (when a map load completes), so it
    /// must run before the `self.map` check in [`Self::update_and_render`].
    ///
    /// Called as late as possible in the frame to give the loader thread the
    /// maximum window to finish in-flight work.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_loaded_resources(&mut self, client_tick: ClientTick) {
        // Collect first so the borrowed loader can be released before the
        // (mutable) handling below.
        let completed_loads: Vec<LoadResult> = self.async_loader.take_completed().collect();

        for completed in completed_loads {
            let (id, resource) = match completed {
                LoadResult::Completed { id, resource } => (id, resource),
                LoadResult::Failed { id, error } => {
                    self.handle_failed_load(id, error);
                    continue;
                }
            };

            if matches!(
                id,
                LoaderId::AnimationData(_) | LoaderId::FalconAnimationData(_) | LoaderId::WeaponAnimationData(_)
            ) {
                self.asset_retries.note_available(&id);
            }

            match (id, resource) {
                (LoaderId::AnimationData(entity_id), LoadableResource::AnimationData(animation_data)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().dead_entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.set_animation_data(animation_data);
                    }
                }
                (LoaderId::FalconAnimationData(entity_id), LoadableResource::FalconAnimation(falcon_animation)) => {
                    crate::logging::log_debug!("falcon animation: applying to entity {}", entity_id.0);
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_falcon_animation(falcon_animation);
                    }
                }
                (LoaderId::WeaponAnimationData(entity_id), LoadableResource::WeaponAnimation(weapon_animation)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_weapon_animation(weapon_animation);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture }) => {
                    self.client_state
                        .follow_mut(client_state().shop_items())
                        .iter_mut()
                        .filter(|item| item.item_id == item_id)
                        .for_each(|item| item.metadata.texture = Some(texture.clone()));

                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_item_sprite(item_id, texture.clone());

                    // Kafra storage items are loaded from a separate inventory
                    // state; without this update their icons stay blank when
                    // the item sprite finished loading while the storage window
                    // is open (the inventory may not contain the same item id).
                    self.client_state
                        .follow_mut(client_state().storage())
                        .update_item_sprite(item_id, texture);
                }
                (LoaderId::Map(map_name), LoadableResource::Map { map, position }) => {
                    match self.client_state.try_follow(this_entity()).is_none() {
                        true => {
                            // Load of main menu map
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(UI_BACKGROUND_MUSIC);

                            // The main menu map finished loading. Do not blindly open the
                            // character selection window here: the map load can finish after the
                            // player already logged in, which used to replace the freshly opened
                            // server selection window (or the login window) with an empty
                            // character selection screen. Show the window that matches the
                            // current login state instead.
                            let character_server_connected = self.networking_system.is_character_server_connected();
                            let login_server_connected = self.networking_system.is_login_server_connected();
                            let character_selection_open = self.interface.is_window_with_class_open(WindowClass::CharacterSelection);
                            let select_server_open = self.interface.is_window_with_class_open(WindowClass::SelectServer);
                            let login_open = self.interface.is_window_with_class_open(WindowClass::Login);

                            if character_server_connected && !character_selection_open {
                                self.interface.open_window(CharacterSelectionWindow::new(
                                    client_state().character_slots(),
                                    client_state().switch_request(),
                                ));
                            } else if login_server_connected && !select_server_open && !character_selection_open {
                                self.interface
                                    .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                            } else if !character_server_connected
                                && !login_server_connected
                                && !login_open
                                && !select_server_open
                                && !character_selection_open
                            {
                                self.interface.open_window(LoginWindow::new(
                                    client_state().login_window(),
                                    client_state().login_settings(),
                                    client_state().client_info(),
                                ));
                            }

                            // The map entry may have been aborted because the map data
                            // was missing; tell the player why, now that the character
                            // selection window is back (opening the error last keeps it
                            // on top).
                            if let Some(message) = self.session.take_map_load_error() {
                                self.interface.open_window(ErrorWindow::new(message));
                            }

                            self.start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                        }
                        false => {
                            // Normal map switch: the map entry is complete.
                            self.session.map_assets_loaded();
                            self.current_map_name = Some(map_name.clone());
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(map.background_music_track_name());

                            if let Some(minimap) = map.get_minimap() {
                                self.client_state.follow_mut(client_state().minimap()).set_map(&map_name, minimap);
                            } else {
                                self.client_state.follow_mut(client_state().minimap()).clear();
                            }

                            if let Some(position) = position {
                                // `manually_asserted` is safe because we are in the branch where `this_player`
                                // is not `None`.
                                let player = self.client_state.follow_mut(this_entity().manually_asserted());

                                player.set_position(map, position, client_tick);
                                self.player_camera.set_focus_point(player.get_position());
                            }

                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                            let _ = self.networking_system.map_loaded();
                        }
                    }
                }
                (LoaderId::SkillSprite(skill_id), LoadableResource::SkillSprite { sprite }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar().skills())
                        .iter_mut()
                        .filter_map(|slot| slot.as_mut())
                        .filter(|skill| skill.skill_id == skill_id)
                        .for_each(|skill| {
                            skill.sprite = Some(sprite.clone());
                        });

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.sprite = Some(sprite);
                    }
                }
                (LoaderId::SkillActions(skill_id), LoadableResource::SkillActions { actions }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar().skills())
                        .iter_mut()
                        .filter_map(|slot| slot.as_mut())
                        .filter(|skill| skill.skill_id == skill_id)
                        .for_each(|skill| {
                            skill.actions = Some(actions.clone());
                        });

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.actions = Some(actions);
                    }
                }
                _ => {}
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    /// Reacts to an asynchronous load that failed.
    ///
    /// A failed map load is a dead end for the current map entry: the client
    /// has no map to render and would stay on a black screen forever. In
    /// that case the map connection is dropped, which sends the player back
    /// to the character selection screen (see
    /// `NetworkEvent::MapServerDisconnected`), and the error is shown
    /// there.
    fn handle_failed_load(&mut self, id: LoaderId, error: LoadError) {
        // Missing weapon sprites are expected for some CondaRO jobs (the archives
        // are trimmed); the async loader caches the failure and the entity is
        // rendered without the weapon layer. Keep this out of the generic asset
        // failure reporting (and out of stderr) so map entry does not spam the
        // user or the log for every population-engine shell.
        if let LoaderId::WeaponAnimationData(entity_id) = &id {
            crate::logging::log_debug!("weapon animation unavailable for entity {}: {error:?}", entity_id.0);
            return;
        }

        match asset_failure_severity(&id) {
            AssetFailureSeverity::Fatal => {
                let LoaderId::Map(map_name) = id else {
                    // Severity and variant classification disagree; treat it as
                    // a transient failure below instead of panicking.
                    self.report_transient_asset_failure(id, error);
                    return;
                };

                // Only report the first failure of a map entry; retries after the
                // client returned to the character selection must not spam.
                if !self.session.has_map_load_error() {
                    crate::logging::log_warn!("map load failed: {map_name}: {error:?}");
                }

                let message = format!(
                    "Map data missing: `{map_name}`\nThis map is not in the game archives. Please log out and choose another character."
                );

                self.abort_map_entry(message);
            }
            AssetFailureSeverity::Transient => {
                if matches!(
                    id,
                    LoaderId::AnimationData(_) | LoaderId::FalconAnimationData(_) | LoaderId::WeaponAnimationData(_)
                ) {
                    self.asset_retries.note_failed(&id, Instant::now(), &format!("{error:?}"));
                }

                self.report_transient_asset_failure(id, error);
            }
        }
    }

    /// Report a recoverable asset failure: always at warn level (visible in
    /// release) and, at most a few times per session, as a non-modal chat line.
    fn report_transient_asset_failure(&mut self, id: LoaderId, error: LoadError) {
        let key = asset_failure_log_key(&id);
        crate::logging::warn_once(
            module_path!(),
            &key,
            format_args!("resource load failed for {key}: {error:?} (a placeholder is used instead)"),
        );

        if self.transient_asset_notices.should_notify() {
            let message = transient_asset_failure_message(&id, &error);
            self.push_chat_warning(message);
        }
    }

    /// Push a non-modal warning into the chat window.
    fn push_chat_warning(&mut self, message: String) {
        crate::logging::log_warn!("{message}");
        self.client_state
            .follow_mut(client_state().chat_messages())
            .push(ChatMessage::new(message, MessageColor::Error));
    }

    /// Close the Kafra storage window and clear the transient storage state.
    ///
    /// Called when the server closes the storage (`ZC_CLOSE_STORE`), when the
    /// player closes it in the UI and defensively on map changes so a stale
    /// window can never show items from a previous storage session.
    fn close_storage_window(&mut self) {
        self.interface.close_window_with_class(WindowClass::Storage);
        self.client_state.follow_mut(client_state().storage()).clear();
        *self.client_state.follow_mut(client_state().storage_amount()) = 0;
        *self.client_state.follow_mut(client_state().storage_maximum_amount()) = 0;
        *self.client_state.follow_mut(client_state().storage_name()) = String::new();
    }

    /// Aborts the current map entry after its map data could not be loaded and
    /// returns to the character selection screen, where the reason is shown.
    ///
    /// Without this the client would keep `self.map == None` and render a black
    /// screen forever, because a failed map load used to be dropped silently.
    fn abort_map_entry(&mut self, message: String) {
        // The session machine records the error, sets the abort deadline and
        // moves to `Recovering`; the client only has to drop the map connection.
        self.session.map_entry_failed(message, Instant::now());

        if self.client_state.try_follow(this_player()).is_none() {
            // The menu background map failed to load; there is no map entry to
            // abort and therefore nothing to go back to.
            if let Some(message) = self.session.take_map_load_error() {
                self.interface.open_window(ErrorWindow::new(message));
            }
            return;
        }

        if self.networking_system.is_map_server_connected() {
            // Dropping the map connection triggers `NetworkEvent::MapServerDisconnected`,
            // which is responsible for going back to the character selection.
            // If that event does not arrive the session machine recovers on its
            // own once the abort deadline expires.
            self.networking_system.disconnect_from_map_server();
        } else {
            // The connection is already gone (e.g. the server kicked us while the
            // map was loading); recover directly.
            self.return_to_character_selection();
        }
    }

    /// Leaves the current map and returns to the character selection screen,
    /// reloading the menu background map.
    fn return_to_character_selection(&mut self) {
        self.map = None;
        self.session.recovery_finished();
        // Every entity of the map session is gone; status effects were already
        // cleared by the map-server disconnect handler.
        self.entity_lifecycle.on_warp();
        self.asset_retries.clear();

        if self.networking_system.is_character_server_connected() {
            // The map connection failed (e.g. while the map server is still
            // restarting) but we are still connected to the character server:
            // go back to the character selection and refresh the list so the
            // player can simply try again.
            let _ = self.networking_system.request_character_list();
        } else {
            self.session.schedule_character_reconnect(Instant::now());
        }

        self.particle_holder.clear();
        self.effect_holder.clear();
        self.point_light_manager.clear();
        self.audio_engine.clear_ambient_sound();

        self.client_state.follow_mut(client_state().entities()).clear();
        self.client_state.follow_mut(client_state().dead_entities()).clear();
        self.chat_bubbles.clear();
        self.pending_item_drag = None;
        self.skill_name_texts.clear();
        self.cast_bars.clear();
        self.aftercast_act_until = None;
        self.client_state.follow_mut(client_state().ground_items()).clear();
        self.pending_context_menu = None;
        *self.client_state.follow_mut(client_state().buffered_action()) = None;

        // We are back at the character selection screen; the map
        // music must not continue there, but the menu music has to
        // play instead of silence.
        self.audio_engine.play_background_music_track(UI_BACKGROUND_MUSIC);

        #[cfg(not(feature = "debug"))]
        self.interface.close_all_windows();

        #[cfg(feature = "debug")]
        self.interface.close_all_windows_except(DEBUG_WINDOWS);

        self.async_loader
            .request_map_load(DEFAULT_MAP.to_string(), Some(TilePosition::new(0, 0)));
    }

    fn update_main_camera(&mut self, window_size: ScreenSize, delta_time: f64, #[cfg(feature = "debug")] render_options: &RenderOptions) {
        if self.client_state.try_follow(this_entity()).is_some() {
            self.player_camera.update(delta_time);
            self.player_camera.generate_view_projection(window_size);
        } else {
            self.start_camera.update(delta_time);
            self.start_camera.generate_view_projection(window_size);
        }

        #[cfg(feature = "debug")]
        self.interface_renderer.update_render_options(render_options);

        #[cfg(feature = "debug")]
        if render_options.use_debug_camera {
            self.debug_camera.generate_view_projection(window_size);
        }
    }

    /// Re-register the local player in the entity lifecycle after a warp.
    ///
    /// A warp clears all entity bookkeeping (only the player survives the map
    /// change), so the player has to be tracked again explicitly.
    fn retrack_local_player(&mut self) {
        let Some(player) = self.client_state.try_follow(this_entity()) else {
            return;
        };

        let entity_id = player.get_entity_id();
        let job_id = player.get_job_id();
        let _ = self.entity_lifecycle.on_add_entity(entity_id);
        self.entity_lifecycle.on_spawned(entity_id, job_id);
    }

    /// Find a visible entity by id.
    fn find_entity_mut(&mut self, entity_id: EntityId) -> Option<&mut Entity> {
        self.client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .find(|entity| entity.get_entity_id() == entity_id)
    }

    /// Track visual resources that are still missing and retry loads whose
    /// deadline expired.
    ///
    /// This is a reconciliation loop: the [`AsyncLoader`] already deduplicates
    /// in-flight requests, so an entity whose animation data did not arrive is
    /// simply re-requested after the [`ResourcePolicy::ANIMATION_DATA`] timeout
    /// (bounded by its attempt budget). A load that failed or was lost can
    /// therefore no longer leave an entity without a sprite forever, and a
    /// resource that really is missing gives up with a warning instead of
    /// retrying silently every frame.
    fn update_asset_load_retries(&mut self, now: Instant) {
        if self.map.is_none() {
            return;
        }

        let mut missing: Vec<LoaderId> = Vec::new();
        for entity in self.client_state.follow(client_state().entities()).iter() {
            let entity_id = entity.get_entity_id();

            if entity.needs_animation_data() {
                missing.push(LoaderId::AnimationData(entity_id));
            }
            if entity.needs_falcon_animation() {
                missing.push(LoaderId::FalconAnimationData(entity_id));
            }
            if entity.needs_weapon_animation() {
                missing.push(LoaderId::WeaponAnimationData(entity_id));
            }
        }

        for id in missing {
            if !self.asset_retries.is_tracked(&id) {
                self.asset_retries.note_request(id, now);
            }
        }

        for id in self.asset_retries.keys_due(now) {
            self.retry_visual_load(id, now);
        }
    }

    /// Re-request one visual resource (`animation data`, falcon or weapon
    /// layer).
    fn retry_visual_load(&mut self, id: LoaderId, now: Instant) {
        let entity_id = match id {
            LoaderId::AnimationData(entity_id) | LoaderId::FalconAnimationData(entity_id) | LoaderId::WeaponAnimationData(entity_id) => {
                entity_id
            }
            _ => {
                self.asset_retries.forget(&id);
                return;
            }
        };

        let entity_info = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(|entity| {
                (
                    entity.get_entity_type(),
                    entity.get_entity_part_files(&self.library),
                    entity.has_falcon(),
                    entity.get_weapon_part_candidates(),
                    entity.has_weapon_animation(),
                )
            });

        let Some((entity_type, entity_part_files, has_falcon, weapon_candidates, has_weapon_animation)) = entity_info else {
            self.asset_retries.forget(&id);
            return;
        };

        match id {
            LoaderId::AnimationData(_) => match self
                .async_loader
                .request_animation_data_load(entity_id, entity_type, entity_part_files)
            {
                Some(animation_data) => {
                    if let Some(entity) = self.find_entity_mut(entity_id) {
                        entity.set_animation_data(animation_data);
                    }
                    self.asset_retries.note_available(&LoaderId::AnimationData(entity_id));
                }
                None => self.asset_retries.note_request(LoaderId::AnimationData(entity_id), now),
            },
            LoaderId::FalconAnimationData(_) => {
                if !has_falcon {
                    self.asset_retries.note_available(&LoaderId::FalconAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_falcon_animation_data_load(entity_id) {
                    Some(falcon_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_falcon_animation(falcon_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::FalconAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::FalconAnimationData(entity_id), now),
                }
            }
            LoaderId::WeaponAnimationData(_) => {
                if weapon_candidates.is_empty() || has_weapon_animation {
                    self.asset_retries.note_available(&LoaderId::WeaponAnimationData(entity_id));
                    return;
                }

                match self.async_loader.request_weapon_animation_data_load(entity_id, weapon_candidates) {
                    Some(weapon_animation) => {
                        if let Some(entity) = self.find_entity_mut(entity_id) {
                            entity.set_weapon_animation(weapon_animation);
                        }
                        self.asset_retries.note_available(&LoaderId::WeaponAnimationData(entity_id));
                    }
                    None => self.asset_retries.note_request(LoaderId::WeaponAnimationData(entity_id), now),
                }
            }
            _ => self.asset_retries.forget(&id),
        }
    }

    /// Per-frame tick for all entities, dead entities, and ground items.
    ///
    /// Must be called after [`Self::handle_network_events`] so the entity
    /// set reflects the latest spawn/despawn/move packets. Running this
    /// with a stale entity list would tick entities that the network has
    /// already removed, or miss new ones that just appeared (and on a map
    /// transition, would tick entities from the previous map).
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_entities(
        &mut self,
        map: &Map,
        currently_playing: bool,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        self.client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .for_each(|entity| entity.update(&self.audio_engine, map, current_camera, client_tick));

        self.client_state
            .follow_mut(client_state().dead_entities())
            .iter_mut()
            .for_each(|entity| {
                entity.update(&self.audio_engine, map, current_camera, client_tick);

                if entity.is_death_animation_over() && !entity.is_fading() {
                    entity.fade_out(DisappearanceReason::Died, client_tick);
                }
            });

        self.client_state
            .follow_mut(client_state().ground_items())
            .iter_mut()
            .for_each(|item| item.update(client_tick));

        // Remove entities that have finished fading out and drop their
        // lifecycle/retry bookkeeping as well.
        let mut disappeared_entities = Vec::new();
        self.client_state.follow_mut(client_state().entities()).retain(|entity| {
            let keep = !entity.should_be_removed(client_tick);
            if !keep {
                disappeared_entities.push(entity.get_entity_id());
            }
            keep
        });

        let mut disappeared_dead_entities = Vec::new();
        self.client_state.follow_mut(client_state().dead_entities()).retain(|entity| {
            let keep = !entity.should_be_removed(client_tick);
            if !keep {
                disappeared_dead_entities.push(entity.get_entity_id());
            }
            keep
        });

        for entity_id in disappeared_entities.into_iter().chain(disappeared_dead_entities) {
            self.entity_lifecycle.on_entity_disappeared(entity_id);
            self.asset_retries.forget(&LoaderId::AnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::FalconAnimationData(entity_id));
            self.asset_retries.forget(&LoaderId::WeaponAnimationData(entity_id));
        }

        self.client_state
            .follow_mut(client_state().ground_items())
            .retain(|item| !item.should_be_removed(client_tick));
    }

    /// Fire any action that the player buffered while out of range or while
    /// still moving (attack, pick up item). Must be called after
    /// [`Self::update_entities`] so that the player's `stopped_moving` state
    /// reflects this frame.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn process_buffered_action(&mut self) {
        let Some(true) = self.client_state.try_follow(this_entity()).map(|player| player.stopped_moving()) else {
            return;
        };

        let Some(buffered_action) = self.client_state.follow_mut(client_state().buffered_action()).take() else {
            return;
        };

        match buffered_action {
            BufferedAction::AttackEntity { entity_id } => {
                let _ = self.networking_system.player_attack(entity_id);

                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                if auto_attack {
                    *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity { entity_id });
                }
            }
            BufferedAction::PickUpItem { entity_id } => {
                if self
                    .client_state
                    .follow(client_state().ground_items())
                    .iter()
                    .any(|item| item.get_entity_id() == entity_id)
                {
                    let _ = self.networking_system.pick_up_item(entity_id);
                }
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_audio_engine(&self, current_camera: &dyn Camera) {
        // We set the listener roughly at ear height.
        const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
        let listener = current_camera.focus_point() + EAR_HEIGHT;

        self.audio_engine
            .set_spatial_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
        self.audio_engine.update();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn create_point_light_set<'a>(
        point_light_manager: &'a mut PointLightManager,
        point_light_set_buffer: &mut ResourceSetBuffer<LightSourceKey>,
        map: &Map,
        effect_holder: &EffectHolder,
        current_camera: &dyn Camera,
        lighting_mode: LightingMode,
    ) -> PointLightSet<'a> {
        point_light_manager.prepare();

        effect_holder.register_point_lights(point_light_manager, current_camera);

        map.register_point_lights(point_light_manager, point_light_set_buffer, current_camera);

        match lighting_mode {
            LightingMode::Classic => point_light_manager.create_point_light_set(0),
            LightingMode::Enhanced => point_light_manager.create_point_light_set(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS),
        }
    }

    /// Applies any buffered drag from the previous frame's input and draws the
    /// per-frame top-level overlays (FPS counter when in debug, mouse cursor).
    /// Must be called after the laid-out interface frame has been dropped so
    /// that `self.interface` is no longer borrowed.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_ui_overlays(
        &mut self,
        input_report: &InputReport,
        scaling: Scaling,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        if let Some(delta) = input_report.drag {
            // TODO: The scaling should be removed here.
            self.interface.handle_drag(delta, scaling.get_factor());
        }

        #[cfg(feature = "debug")]
        if render_options.show_frames_per_second {
            let world_theme = self.client_state.follow(client_state().world_theme());

            self.top_interface_renderer.render_text(
                &self.game_timer.last_frames_per_second().to_string(),
                world_theme.overlay.text_offset,
                world_theme.overlay.foreground_color,
                world_theme.overlay.font_size,
                AlignHorizontal::Left,
            );
        }

        if self.show_interface {
            self.mouse_cursor.render(
                &self.top_interface_renderer,
                input_report.mouse_position,
                self.interface.get_mouse_mode().grabbed(),
                *self.client_state.follow(client_state().world_theme().cursor().color()),
                self.client_state.follow(client_state().interface_settings().scaling()).get_factor(),
            );
        }
    }

    #[inline(always)]
    fn update_and_render(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        crate::perf::frame_begin();

        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            // The session machine owns the shutdown sequence and its deadline.
            self.session.request_shutdown();
        }

        if self.session.state() == SessionState::Shutdown {
            // Ask the map server for a clean disconnect (CZ_REQ_DISCONNECT) instead of
            // just dropping the TCP connection. The map server then saves the character
            // and reports the account offline to the character/login server, so an
            // immediate relogin is not rejected with "already online".
            if self.session.shutdown_deadline().is_none() {
                if self.networking_system.is_map_server_connected() {
                    let _ = self.networking_system.request_disconnect();
                    self.session
                        .set_shutdown_deadline(Some(Instant::now() + SHUTDOWN_DISCONNECT_TIMEOUT));
                } else if self.networking_system.is_character_server_connected() {
                    self.networking_system.disconnect_from_character_server();
                } else if self.networking_system.is_login_server_connected() {
                    self.networking_system.disconnect_from_login_server();
                } else {
                    self.session.finish_shutdown();
                    event_loop.exit();
                    return;
                }
            }

            // Exit once the map server disconnected us (clean logout done) or the
            // deadline expired. Close the remaining sockets in server order first so the
            // login server can clear its online state before the process exits.
            if !self.networking_system.is_map_server_connected()
                || self.session.shutdown_deadline().is_some_and(|deadline| Instant::now() >= deadline)
            {
                self.networking_system.disconnect_from_map_server();
                self.networking_system.disconnect_from_character_server();
                self.networking_system.disconnect_from_login_server();
                self.audio_engine.play_background_music_track(None);
                self.session.finish_shutdown();
                event_loop.exit();
                return;
            }
        }

        // Drive the explicit session lifecycle: map-entry watchdog, map load
        // retries, abort recovery and the character-server reconnect backoff.
        // The old loose timers (`map_entry_watchdog`, `map_entry_abort_deadline`,
        // `pending_map_load`, `character_server_reconnect_at`, …) are owned by
        // the session machine now.
        self.update_session_lifecycle();

        // Clear the previous render instructions so we can rendering the new frame.
        self.clear_render_instructions();

        // Aggregate experience notifications arrive per kill; show them as one
        // chat line once the burst is over.
        self.flush_experience_feedback();

        // It is important that we first apply any changes that were dispatched during
        // the last frame.
        self.update_client_state();

        // We can only apply the graphic changes and reconfigure the surface once the
        // previous image was presented. Moving this function to the end of the
        // function results in surface configuration errors under DX12.
        self.update_settings();

        // Requires the frame rate limit that was just applied, so it is done
        // after `update_settings`.
        self.log_frames_per_second();
        self.log_performance();

        // TODO: Shouldn't this happen later? After the scaling has been potentially
        // changed by the UI.
        let scaling = *self.client_state.follow(client_state().interface_settings().scaling());
        self.update_interface_scaling(scaling);

        let FrameTimers {
            delta_time,
            client_tick,
            animation_timer_ms,
        } = self.game_timer.update();

        if std::mem::take(&mut self.interface_layout_invalidation_pending) {
            self.interface.invalidate_layouts();
        }

        let input_report = self.input_system.update_delta(client_tick);

        self.request_entity_details(&input_report);
        self.request_visible_entity_names();

        self.handle_network_events(client_tick);
        self.update_status_effects(client_tick);
        self.update_spirit_spheres();
        self.update_asset_load_retries(Instant::now());

        let interface_has_focus = self.process_user_events(
            &input_report,
            client_tick,
            #[cfg(feature = "debug")]
            (delta_time as f32),
        );

        // The shop windows can be closed with their X button, which does not send the
        // close-shop packet by itself. If a shop session is still open on the server
        // but no shop window is open anymore, close it so that the map server stops
        // blocking player movement (it ignores walk requests while an NPC shop is
        // open).
        if self.shop_open
            && !self.interface.is_window_with_class_open(WindowClass::BuyOrSell)
            && !self.interface.is_window_with_class_open(WindowClass::Buy)
            && !self.interface.is_window_with_class_open(WindowClass::BuyCart)
            && !self.interface.is_window_with_class_open(WindowClass::Sell)
            && !self.interface.is_window_with_class_open(WindowClass::SellCart)
        {
            let _ = self.networking_system.close_shop();
            self.shop_open = false;
            self.client_state.follow_mut(client_state().buy_cart()).clear();
            self.client_state.follow_mut(client_state().sell_cart()).clear();
        }

        // Some debug windows, such as the packet history or cache statistics, require
        // special update logic.
        #[cfg(feature = "debug")]
        self.update_debug_windows(delta_time);

        // We run this last to give the loader thread as much time as possible to
        // complete the loading. When starting the actual render process, we
        // can't modify resources anymore until the next frame.
        self.update_loaded_resources(client_tick);

        // Headless benchmark driver (`KORANGAR_BENCH`, see `benchmark.rs`); a
        // no-op in normal runs. Runs after the load results were applied so the
        // measured map-change latency includes the actual completion frame.
        crate::benchmark::drive(self, client_tick.0 as u64);

        // Keep the minimap marker in sync with the player. The marker is only
        // updated once the map (and therefore the minimap) is loaded.
        let player_position = self
            .map
            .is_some()
            .then(|| self.client_state.try_follow(this_entity()).map(|entity| entity.get_tile_position()))
            .flatten();

        match player_position {
            Some(player_position) => {
                let previous_position = self.minimap_last_tile.replace(player_position);
                let current_direction = self.client_state.follow(client_state().minimap()).get_player_direction();
                let player_direction = previous_position
                    .and_then(|previous_position| direction_index_from_tile_delta(previous_position, player_position))
                    .unwrap_or(current_direction);

                self.client_state
                    .follow_mut(client_state().minimap())
                    .set_player_position(Some(player_position), player_direction);
            }
            None => {
                self.minimap_last_tile = None;
                self.client_state.follow_mut(client_state().minimap()).set_player_position(None, 0);
            }
        }

        #[cfg(feature = "debug")]
        let render_options = *self.client_state.follow(client_state().render_options());

        let screen_size = self.graphics_engine.get_window_size().into();
        let currently_playing = self.client_state.try_follow(this_player()).is_some();

        self.mouse_cursor.update(client_tick);

        // Do not touch the swapchain while the native Wayland window is in the
        // background. KWin can stop releasing swapchain images for hidden
        // windows and NVIDIA's Wayland WSI then blocks forever inside the image
        // acquisition.
        if self.pause_presentation_if_needed() {
            return;
        }

        // Acquire the swapchain image as late as possible so all CPU-side
        // preparation overlaps with the previous frame's GPU work. After this
        // point we may not reconfigure the surface (see `update_settings`).
        let before_wait = std::time::Instant::now();
        let maybe_frame = self.graphics_engine.wait_for_next_frame();
        let after_wait = std::time::Instant::now();
        let allocations_after_wait = crate::perf::thread_allocation_snapshot();

        // If we don't have a map, the rendering ends here.
        let Some(map) = self.map.clone() else {
            if let Some(frame) = maybe_frame {
                self.graphics_engine.render_next_frame(frame, Default::default());
                self.has_presented_frame = true;
            }

            return;
        };

        self.update_entities(
            &map,
            currently_playing,
            client_tick,
            #[cfg(feature = "debug")]
            &render_options,
        );

        self.update_level_99_auras();

        self.process_buffered_action();

        self.update_main_camera(
            screen_size,
            delta_time,
            #[cfg(feature = "debug")]
            &render_options,
        );

        map.advance_videos(&self.queue, delta_time);

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            self.player_camera.set_smoothed_focus_point(player.get_position());
        }

        // Update particles.
        self.particle_holder.update(delta_time as f32);
        self.effect_holder
            .update(self.client_state.follow(client_state().entities()), delta_time as f32);

        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        let (view_matrix, projection_matrix) = current_camera.view_projection_matrices();
        let camera_position = current_camera.camera_position().to_homogeneous();

        #[cfg(feature = "debug")]
        let update_shadow_camera_measurement = Profiler::start_measurement("update directional shadow camera");

        let lighting_mode = *self.client_state.follow(client_state().graphics_settings().lighting_mode());
        let shadow_resolution = *self.client_state.follow(client_state().graphics_settings().shadow_resolution());
        let shadow_method = *self.client_state.follow(client_state().graphics_settings().shadow_method());
        let shadow_detail = *self.client_state.follow(client_state().graphics_settings().shadow_detail());
        let sdsm_enabled = *self.client_state.follow(client_state().graphics_settings().sdsm());

        let use_sdsm = sdsm_enabled & !self.player_camera.is_rotating_or_zooming_fast();

        let (directional_light_direction, directional_light_color) = map.directional_light();

        match use_sdsm {
            true => {
                self.directional_shadow_camera.update_camera_sdsm(
                    directional_light_direction,
                    &view_matrix,
                    &projection_matrix,
                    shadow_resolution.directional_shadow_resolution(),
                    self.directional_shadow_partitions.lock().unwrap().deref(),
                );
            }
            false => {
                self.directional_shadow_camera.update_camera_pssm(
                    directional_light_direction,
                    &view_matrix,
                    &projection_matrix,
                    shadow_resolution.directional_shadow_resolution(),
                );
            }
        }

        #[cfg(feature = "debug")]
        update_shadow_camera_measurement.stop();

        self.update_audio_engine(current_camera);

        #[cfg(feature = "debug")]
        let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

        #[cfg(feature = "debug")]
        let hovered_marker_identifier = match input_report.mouse_target {
            PickerTarget::Marker(marker_identifier) => Some(marker_identifier),
            _ => None,
        };

        let point_light_set = Self::create_point_light_set(
            &mut self.point_light_manager,
            &mut self.point_light_set_buffer,
            &map,
            &self.effect_holder,
            current_camera,
            lighting_mode,
        );

        #[cfg(feature = "debug")]
        prepare_frame_measurement.stop();

        let mut indicator_instruction = None;
        let mut water_instruction = None;

        let mouse_mode = self.interface.get_mouse_mode();
        let is_mouse_mode_default = mouse_mode.is_default();
        let last_walking_destination = mouse_mode.walk_destination();
        let skill_target = mouse_mode.skill_target();

        let allocations_after_sim = crate::perf::thread_allocation_snapshot();
        let after_sim = std::time::Instant::now();

        let mut interface_state_changed = false;

        let mut interface_frame = {
            #[cfg(feature = "debug")]
            profile_block!("user interface");

            let is_rotating_camera = mouse_mode.is_rotating_camera();
            let is_grabbing = mouse_mode.is_grabbing();
            let is_chat_open = self.interface.is_window_with_class_open(WindowClass::Chat);

            // While the interface settings window is open, show the move grip and resize
            // handles on every window (not just the hovered one) so the layout can be
            // arranged while tuning the interface.
            let interface_settings_open = self.interface.is_window_with_class_open(WindowClass::InterfaceSettings);
            self.interface.set_window_chrome_visible(interface_settings_open);

            let is_context_menu_open = self.interface.is_window_with_class_open(WindowClass::ContextMenu);
            let is_item_info_open = self.interface.is_window_with_class_open(WindowClass::ItemInfo);

            let mut interface_frame = self.interface.lay_out_windows(
                &self.client_state,
                scaling.get_factor(),
                input_report.mouse_position,
                input_report.left_mouse_button_down,
            );

            // We can only decide what to do with the user input once we know if the mouse
            // is hovering a window, so we buffer any actions for the next frame.

            let is_interface_hovered = interface_frame.is_interface_hovered();
            let is_context_menu_hovered = interface_frame.is_window_with_class_hovered(WindowClass::ContextMenu);
            let is_item_info_hovered = interface_frame.is_window_with_class_hovered(WindowClass::ItemInfo);

            // A click outside of the item info window closes it.
            if is_item_info_open && !is_item_info_hovered && input_report.mouse_click.is_some() {
                self.input_event_buffer.push(InputEvent::CloseItemInfo);
            }

            if crate::logging::enabled(crate::logging::LEVEL_TRACE) {
                crate::logging::log_trace!(
                    "[input] frame mouse=({:.2}, {:.2}) window={}x{} scaling={:.2} hovered={}",
                    input_report.mouse_position.left,
                    input_report.mouse_position.top,
                    screen_size.width,
                    screen_size.height,
                    scaling.get_factor(),
                    is_interface_hovered,
                );
            }

            // A click outside of the context menu closes it. Left clicks in the
            // world are swallowed so that closing the menu does not also move the
            // player or interact with an entity.
            let click_closes_context_menu = is_context_menu_open && !is_context_menu_hovered;

            if click_closes_context_menu && input_report.mouse_click.is_some() {
                self.input_event_buffer.push(InputEvent::CloseContextMenu);
            }

            let cursor_state = match input_report.mouse_target {
                _ if is_rotating_camera => MouseCursorState::RotateCamera,
                _ if is_grabbing => MouseCursorState::GrabResource,
                _ if skill_target.is_some() => MouseCursorState::Target,
                PickerTarget::Entity(entity_id) if !is_interface_hovered => {
                    if self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .any(|item| item.get_entity_id() == entity_id)
                    {
                        MouseCursorState::HoverItem
                    } else {
                        self.client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| match entity.get_entity_type() {
                                EntityType::Npc => MouseCursorState::Dialog,
                                EntityType::Warp => MouseCursorState::Warp,
                                EntityType::Monster => MouseCursorState::Attack,
                                _ => MouseCursorState::Default,
                            })
                            .unwrap_or(MouseCursorState::Default)
                    }
                }
                _ => MouseCursorState::Default,
            };
            self.mouse_cursor.set_state(cursor_state, client_tick);

            if let Some(mouse_button) = input_report.mouse_click {
                interface_state_changed = true;

                if crate::logging::enabled(crate::logging::LEVEL_DEBUG) {
                    crate::logging::log_debug!(
                        "[input] click {mouse_button:?} at ({:.1}, {:.1}) hovered={is_interface_hovered}",
                        input_report.mouse_position.left,
                        input_report.mouse_position.top,
                    );
                }

                if is_interface_hovered && self.input_system.has_pointer_position() {
                    interface_frame.click(&self.client_state, mouse_button);
                } else if click_closes_context_menu && mouse_button == MouseButton::Left {
                    // The left click only closed the context menu.
                } else {
                    interface_frame.unfocus();

                    if let Some((_, skill_id, skill_type, level)) = skill_target {
                        // A skill target is being selected. Any click ends the
                        // selection, and a left click casts the skill at whatever
                        // is under the cursor. The mode is reset here explicitly
                        // because the release that starts a selection no longer
                        // resets it (the mode is sticky for one click).
                        interface_frame.set_mouse_mode(MouseMode::Default);

                        if mouse_button == MouseButton::Left {
                            self.input_event_buffer.push(InputEvent::CastSkill {
                                skill_id,
                                skill_type,
                                level,
                            });
                        }
                    } else if mouse_button == MouseButton::Left {
                        match input_report.mouse_target {
                            PickerTarget::Nothing => {}
                            PickerTarget::Entity(entity_id) => {
                                let is_ground_item = self
                                    .client_state
                                    .follow(client_state().ground_items())
                                    .iter()
                                    .any(|item| item.get_entity_id() == entity_id);

                                if is_ground_item {
                                    self.input_event_buffer.push(InputEvent::PickUpItem { entity_id })
                                } else {
                                    self.input_event_buffer.push(InputEvent::PlayerInteract { entity_id })
                                }
                            }
                            PickerTarget::Tile { x, y } => {
                                let destination = TilePosition { x, y };

                                interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });

                                self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                            }
                            #[cfg(feature = "debug")]
                            PickerTarget::Marker(marker_identifier) => {
                                self.input_event_buffer.push(InputEvent::OpenMarkerDetails { marker_identifier })
                            }
                        }
                    } else if mouse_button == MouseButton::Right && currently_playing {
                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                        interface_frame.set_mouse_mode(MouseInputMode::RotateCamera);
                    } else if mouse_button == MouseButton::DoubleRight && currently_playing {
                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                        self.input_event_buffer.push(InputEvent::ResetCameraRotation);
                    }
                }
            } else if let Some(last_destination) = last_walking_destination
                && let PickerTarget::Tile { x, y } = input_report.mouse_target
                && input_report.left_mouse_button_down
            {
                let destination = TilePosition { x, y };

                if last_destination != destination {
                    interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });
                    self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                }
            }

            // Open the character context menu when the right mouse button was
            // pressed and released over a character without dragging. A right
            // drag keeps rotating the camera like before.
            const RIGHT_CLICK_DRAG_THRESHOLD: f32 = 4.0;

            if input_report.right_mouse_button_released
                && is_rotating_camera
                && !is_interface_hovered
                && currently_playing
                && input_report.right_mouse_button_drag_distance < RIGHT_CLICK_DRAG_THRESHOLD
                && skill_target.is_none()
            {
                if is_context_menu_open {
                    self.input_event_buffer.push(InputEvent::CloseContextMenu);
                } else if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                    let is_character = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .is_some_and(|entity| entity.get_entity_type() == EntityType::Player);
                    let is_own_character = self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|entity| entity.get_entity_id() == entity_id);

                    if is_character && !is_own_character {
                        self.input_event_buffer.push(InputEvent::OpenCharacterContextMenu {
                            entity_id,
                            position: input_report.mouse_position,
                        });
                    }
                }
            }

            if input_report.mouse_button_released {
                // A click that starts a sticky mouse mode (targeted skill
                // selection) must not be cancelled by the release of that same
                // click; the mode stays until the next click.
                interface_frame.drop(&self.client_state, skill_target.is_none());
            }

            if let Some(delta) = input_report.scroll {
                if let Some((slot, skill_id, skill_type, level)) = skill_target {
                    // While selecting a skill target the mouse wheel adjusts the
                    // cast level instead of zooming, like in the original client.
                    let maximum_level = match slot {
                        Some(slot) => self
                            .client_state
                            .follow(client_state().hotbar())
                            .get_skill_in_slot(slot)
                            .as_ref()
                            .map(|skill| skill.maximum_level.0)
                            .unwrap_or(1),
                        None => self
                            .client_state
                            .follow(client_state().skill_tree().skills())
                            .iter()
                            .find(|learned_skill| learned_skill.skill_id == skill_id)
                            .map(|learned_skill| learned_skill.skill_level.0)
                            .unwrap_or(1),
                    };

                    let level = match delta > 0.0 {
                        true => SkillLevel(level.0.saturating_add(1).min(maximum_level)),
                        false => SkillLevel(level.0.saturating_sub(1).max(1)),
                    };

                    interface_frame.set_mouse_mode(MouseInputMode::TargetSkill {
                        slot,
                        skill_id,
                        skill_type,
                        level,
                    });
                } else if is_interface_hovered {
                    interface_state_changed = true;
                    interface_frame.scroll(&self.client_state, delta);
                } else {
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                    self.input_event_buffer.push(InputEvent::ZoomCamera { zoom_factor: delta });
                }
            }

            // Focus the chat if the interface is not focused, no other element is capturing
            // the keyboard input, enter was pressed, and the chat
            // window is open.
            if (!interface_has_focus || !interface_frame.input_characters(&self.client_state, &input_report.characters))
                && input_report.characters.contains(&'\x0d')
                && is_chat_open
            {
                interface_frame.focus_element(ChatTextBox);
            }

            if interface_state_changed {
                // Clicks/scrolls are handled on the interface frame after the
                // layout was created, so the layouts have to be rebuilt next
                // frame (the interface itself is borrowed by the frame here).
                self.interface_layout_invalidation_pending = true;
            }

            interface_frame
        };

        let allocations_after_interface = crate::perf::thread_allocation_snapshot();
        let after_interface = std::time::Instant::now();

        {
            let mut render_context = MapRenderContext {
                map: &map,
                current_camera,
                point_light_set: &point_light_set,
                client_state: &self.client_state,
                library: &self.library,
                guild_emblems: &self.guild_emblems,
                mouse_position: input_report.mouse_position,
                mouse_target: input_report.mouse_target,
                screen_size,
                scaling,
                client_tick,
                animation_timer_ms,
                currently_playing,
                is_mouse_mode_default,
                is_interface_hovered: interface_frame.is_interface_hovered(),
                last_walking_destination,
                is_targeting_ground_skill: matches!(skill_target, Some((_, _, SkillType::Ground | SkillType::Trap, _))),
                skill_target_level: skill_target.map(|(_, _, _, level)| level),
                buffered_action: *self.client_state.follow(client_state().buffered_action()),
                #[cfg(feature = "debug")]
                render_options: &render_options,
                #[cfg(feature = "debug")]
                hovered_marker_identifier,
                #[cfg(feature = "debug")]
                pathing_texture_set: &self.pathing_texture_set,
                #[cfg(feature = "debug")]
                tile_texture_set: &self.tile_texture_set,
                #[cfg(feature = "debug")]
                player_camera: &self.player_camera,
                #[cfg(feature = "debug")]
                start_camera: &self.start_camera,
                model_batches: &mut self.model_batches,
                model_instructions: &mut self.model_instructions,
                entity_instructions: &mut self.entity_instructions,
                directional_shadow_camera: &mut self.directional_shadow_camera,
                directional_shadow_model_batches: &mut self.directional_shadow_model_batches,
                directional_shadow_model_instructions: &mut self.directional_shadow_model_instructions,
                directional_shadow_entity_instructions: &mut self.directional_shadow_entity_instructions,
                point_shadow_camera: &mut self.point_shadow_camera,
                point_shadow_model_instructions: &mut self.point_shadow_model_instructions,
                point_shadow_entity_instructions: &mut self.point_shadow_entity_instructions,
                point_light_with_shadow_instructions: &mut self.point_light_with_shadow_instructions,
                point_light_instructions: &mut self.point_light_instructions,
                directional_shadow_object_set_buffer: &mut self.directional_shadow_object_set_buffer,
                point_shadow_object_set_buffer: &mut self.point_shadow_object_set_buffer,
                deferred_object_set_buffer: &mut self.deferred_object_set_buffer,
                indicator_instruction: &mut indicator_instruction,
                water_instruction: &mut water_instruction,
                particle_holder: &mut self.particle_holder,
                effect_holder: &mut self.effect_holder,
                chat_bubbles: &self.chat_bubbles,
                cast_bars: &self.cast_bars,
                skill_name_texts: &self.skill_name_texts,
                warp_portal: self.warp_portal.as_ref(),
                warp_skill_units: &self.warp_skill_units,
                effect_renderer: &mut self.effect_renderer,
                bottom_interface_renderer: &self.bottom_interface_renderer,
                middle_interface_renderer: &mut self.middle_interface_renderer,
                #[cfg(feature = "debug")]
                aabb_instructions: &mut self.aabb_instructions,
                #[cfg(feature = "debug")]
                circle_instructions: &mut self.circle_instructions,
                #[cfg(feature = "debug")]
                rectangle_instructions: &mut self.rectangle_instructions,
                #[cfg(feature = "debug")]
                bounding_box_object_set_buffer: &mut self.bounding_box_object_set_buffer,
                #[cfg(feature = "debug")]
                debug_marker_renderer: &mut self.debug_marker_renderer,
            };

            #[cfg(feature = "debug")]
            render_context.render_markers();
            render_context.render_directional_shadows();
            render_context.render_point_lights();
            render_context.render_geometry();
            #[cfg(feature = "debug")]
            render_context.render_bounding_boxes();
            render_context.render_world_overlays();
        }

        let in_game_theme_path = client_state().in_game_theme().tooltip();
        let menu_theme_path = client_state().menu_theme().tooltip();
        let tooltip_theme = match currently_playing {
            true => self.client_state.follow(in_game_theme_path),
            false => self.client_state.follow(menu_theme_path),
        };

        interface_frame.render(
            &self.client_state,
            &self.interface_renderer,
            tooltip_theme,
            input_report.mouse_position,
        );

        drop(interface_frame);

        self.render_ui_overlays(
            &input_report,
            scaling,
            #[cfg(feature = "debug")]
            &render_options,
        );

        let picker_position = ScreenPosition {
            left: input_report.mouse_position.left.clamp(0.0, screen_size.width),
            top: input_report.mouse_position.top.clamp(0.0, screen_size.height),
        };

        let uniforms = Uniforms {
            view_matrix,
            projection_matrix,
            camera_position,
            animation_timer_ms,
            ambient_light_color: map.ambient_light_color(),
            enhanced_lighting: lighting_mode == LightingMode::Enhanced,
            shadow_method,
            shadow_detail,
            use_sdsm,
            sdsm_enabled,
        };

        let interface_instructions = self.interface_renderer.get_instructions();
        let bottom_layer_instructions = self.bottom_interface_renderer.get_instructions();
        let middle_layer_instructions = self.middle_interface_renderer.get_instructions();
        let top_layer_instructions = self.top_interface_renderer.get_instructions();

        let directional_light = DirectionalLightInstruction {
            view_projection_matrix: self.directional_shadow_camera.view_projection_matrix(),
            direction: directional_light_direction,
            color: directional_light_color,
        };

        let allocations_after_world = crate::perf::thread_allocation_snapshot();
        let after_world = std::time::Instant::now();

        let render_instruction = RenderInstruction {
            show_interface: self.show_interface,
            picker_position,
            uniforms,
            indicator: indicator_instruction,
            interface: interface_instructions.as_slice(),
            bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
            middle_layer_rectangles: middle_layer_instructions.as_slice(),
            top_layer_rectangles: top_layer_instructions.as_slice(),
            directional_light,
            directional_light_partitions: &self.directional_shadow_camera.get_partition_instructions(),
            point_light: &self.point_light_instructions,
            point_light_with_shadows: &self.point_light_with_shadow_instructions,
            model_batches: &self.model_batches,
            models: &mut self.model_instructions,
            entities: &mut self.entity_instructions,
            directional_shadow_model_batches: &self.directional_shadow_model_batches,
            directional_shadow_models: &self.directional_shadow_model_instructions,
            directional_shadow_entities: &mut self.directional_shadow_entity_instructions,
            point_shadow_models: &self.point_shadow_model_instructions,
            point_shadow_entities: &mut self.point_shadow_entity_instructions,
            effects: self.effect_renderer.get_instructions(),
            water: water_instruction,
            map_picker_tile_vertex_buffer: Some(map.get_tile_picker_vertex_buffer()),
            map_picker_tile_index_buffer: Some(map.get_tile_picker_index_buffer()),
            slug_font: Some(self.font_loader.get_slug_font()),
            #[cfg(feature = "debug")]
            render_options,
            #[cfg(feature = "debug")]
            aabb: &self.aabb_instructions,
            #[cfg(feature = "debug")]
            circles: &self.circle_instructions,
            #[cfg(feature = "debug")]
            rectangles: &self.rectangle_instructions,
            #[cfg(feature = "debug")]
            marker: self.debug_marker_renderer.get_instructions(),
        };

        if let Some(frame) = maybe_frame {
            self.graphics_engine.render_next_frame(frame, render_instruction);
            self.has_presented_frame = true;
        }

        let after_gpu = std::time::Instant::now();
        let allocations_after_gpu = crate::perf::thread_allocation_snapshot();

        crate::perf::sample_counts(
            self.effect_holder.count() as u32,
            self.particle_holder.count() as u32,
            self.client_state.follow(client_state().entities()).len() as u32,
        );

        let (rendered_effects, culled_effects) = self.effect_holder.render_counts();
        let (rendered_particles, culled_particles) = self.particle_holder.render_counts();
        crate::perf::sample_render_counts(rendered_effects, culled_effects, rendered_particles, culled_particles);

        record_frame_phase(
            after_wait.saturating_duration_since(before_wait),
            after_sim.saturating_duration_since(after_wait),
            after_interface.saturating_duration_since(after_sim),
            after_world.saturating_duration_since(after_interface),
            after_gpu.saturating_duration_since(after_world),
            [
                allocations_after_sim.saturating_sub(allocations_after_wait),
                allocations_after_interface.saturating_sub(allocations_after_sim),
                allocations_after_world.saturating_sub(allocations_after_interface),
                allocations_after_gpu.saturating_sub(allocations_after_world),
            ],
        );
    }
}

/// Open the character context menu for the given entity if its name is already
/// known. Returns `true` if the name still has to be requested from the map
/// server; the menu can then be opened once the name arrives.
fn open_character_context_menu(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    entity_id: EntityId,
    position: ScreenPosition,
) -> bool {
    let character_name = state
        .follow(client_state().entities())
        .iter()
        .find(|entity| entity.get_entity_id() == entity_id)
        .and_then(|entity| entity.get_details())
        .map(|details| details.split('#').next().unwrap_or_default().to_owned())
        .filter(|name| !name.is_empty());

    match character_name {
        Some(character_name) => {
            interface.close_window_with_class(WindowClass::ContextMenu);
            interface.open_window_at(CharacterContextMenuWindow::new(character_name), position);

            false
        }
        None => true,
    }
}

/// Returns the localized label of an item category.
fn item_category_label(state: &State<ClientState>, category: ItemCategory) -> String {
    let localization = client_state().localization();

    match category {
        ItemCategory::Consumables => state.follow(localization.inventory_consumables_tab_text()).clone(),
        ItemCategory::Equipments => state.follow(localization.inventory_equipments_tab_text()).clone(),
        ItemCategory::Misc => state.follow(localization.inventory_misc_tab_text()).clone(),
    }
}

/// Open the context menu of the inventory item with the given inventory index
/// at the given screen position.
fn open_inventory_item_menu(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    inventory_index: InventoryIndex,
    position: ScreenPosition,
) {
    let items = state.follow(client_state().inventory().items());
    let Some(item) = items.iter().find(|item| item.index == inventory_index) else {
        return;
    };

    let action = item_use_action(item);

    // While a Kafra storage window is open the same menu can store the item.
    let storage_action = match interface.is_window_with_class_open(WindowClass::Storage) {
        true => Some(ItemStorageAction::Deposit {
            inventory_index,
            amount: item_amount(item),
        }),
        false => None,
    };

    interface.close_window_with_class(WindowClass::ContextMenu);
    interface.open_window_at(
        ItemContextMenuWindow::new(inventory_index, item.item_id, action).with_storage_action(storage_action),
        position,
    );
}

/// Open the context menu of a Kafra storage item.
fn open_storage_item_menu(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    storage_index: InventoryIndex,
    position: ScreenPosition,
) {
    let items = state.follow(client_state().storage().items());
    let Some(item) = items.iter().find(|item| item.index == storage_index) else {
        return;
    };

    interface.close_window_with_class(WindowClass::ContextMenu);
    interface.open_window_at(
        ItemContextMenuWindow::new(storage_index, item.item_id, None)
            .with_source(ItemSource::Storage)
            .with_storage_action(Some(ItemStorageAction::Withdraw {
                storage_index,
                amount: item_amount(item),
            })),
        position,
    );
}

/// The amount of an [InventoryItem]: the stack size for regular items, one for
/// equipment and other non-stackable items.
fn item_amount(item: &korangar_networking::InventoryItem<ResourceMetadata>) -> u32 {
    match &item.details {
        korangar_networking::InventoryItemDetails::Regular { amount, .. } => (*amount).into(),
        korangar_networking::InventoryItemDetails::Equippable { .. } => 1,
    }
}

/// Open the info window of the inventory or Kafra storage item with the given
/// index.
///
/// The item info window shows the same details for both sources: the item
/// description, the refinement level, the slotted cards and the random
/// options (the storage rows carry the same packet data as the inventory
/// rows).
fn open_item_info_window(
    state: &State<ClientState>,
    interface: &mut Interface<'static, ClientState>,
    inventory_index: InventoryIndex,
    source: ItemSource,
    library: &Library,
    texture_loader: &TextureLoader,
) {
    let items = match source {
        // Equipped items are part of the inventory item list as well.
        ItemSource::Inventory | ItemSource::Equipment { .. } => state.follow(client_state().inventory().items()),
        ItemSource::Storage => state.follow(client_state().storage().items()),
    };

    let Some(item) = items.iter().find(|item| item.index == inventory_index) else {
        return;
    };

    let category = item_category(item);
    let category_label = item_category_label(state, category);

    let artwork = load_item_artwork(library, texture_loader, item);
    let card_icons = load_item_card_icons(library, texture_loader, item);

    interface.close_window_with_class(WindowClass::ItemInfo);
    interface.open_window(ItemInfoWindow::new(item, category_label, artwork, card_icons));
}

/// Loads the large artwork (collection image) of an item for the item display
/// window. Falls back to the small inventory icon when the collection image is
/// missing.
fn load_item_artwork(
    library: &Library,
    texture_loader: &TextureLoader,
    item: &korangar_networking::InventoryItem<ResourceMetadata>,
) -> Option<Arc<Texture>> {
    let resource = library.get::<ItemResource>(ItemResourceKey {
        item_id: item.item_id,
        is_identified: item.is_identified(),
    });
    let path = format!("유저인터페이스\\collection\\{resource}.bmp");

    texture_loader
        .get_or_load(&path, ImageType::Color)
        .ok()
        .or_else(|| item.metadata.texture.clone())
}

/// Loads the small card item icons of the item's card slots for the item
/// display window.
///
/// Unidentified items hide their cards, so no icons are loaded for them.
/// Unknown card ids resolve to `None` and the window draws a generic filled
/// slot instead.
fn load_item_card_icons(
    library: &Library,
    texture_loader: &TextureLoader,
    item: &korangar_networking::InventoryItem<ResourceMetadata>,
) -> [Option<Arc<Texture>>; CARD_SLOT_COUNT] {
    if !item.is_identified() {
        return std::array::from_fn(|_| None);
    }

    std::array::from_fn(|slot| {
        let card_id = item.slot[slot];

        if card_id == 0 {
            return None;
        }

        let resource = library.get::<ItemResource>(ItemResourceKey {
            item_id: ItemId(card_id),
            is_identified: true,
        });
        let path = format!("유저인터페이스\\item\\{resource}.bmp");

        texture_loader.get_or_load(&path, ImageType::Color).ok()
    })
}

/// Returns `true` if the given environment variable is set to a truthy value.
fn environment_flag(variable: &str) -> bool {
    std::env::var(variable).is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// The deadline of the first log line for `KORANGAR_FPS_LOG`, if enabled.
fn frames_per_second_log_deadline() -> Option<Instant> {
    if environment_flag(FRAMERATE_LOG_ENVIRONMENT_VARIABLE) {
        Some(Instant::now() + Duration::from_secs(FRAMERATE_LOG_INTERVAL_SECONDS))
    } else {
        None
    }
}

/// Environment variable that enables the periodic layout/text cache profiling
/// line (`[perf] ...`).
const PERFORMANCE_LOG_ENVIRONMENT_VARIABLE: &str = "KORANGAR_PERF_LOG";
/// Seconds between two `[perf]` lines.
const PERFORMANCE_LOG_INTERVAL_SECONDS: u64 = 2;

/// The deadline of the first `[perf]` line, if profiling logging is enabled.
fn performance_log_deadline() -> Option<Instant> {
    if environment_flag(PERFORMANCE_LOG_ENVIRONMENT_VARIABLE) {
        Some(Instant::now() + Duration::from_secs(PERFORMANCE_LOG_INTERVAL_SECONDS))
    } else {
        None
    }
}

/// Frame phase profiling counters for `KORANGAR_PERF_LOG`.
static FRAME_PROFILE_FRAMES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static FRAME_PROFILE_WAIT_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static FRAME_PROFILE_SIM_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static FRAME_PROFILE_INTERFACE_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static FRAME_PROFILE_WORLD_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static FRAME_PROFILE_GPU_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Snapshot of the frame phase profiling counters.
#[derive(Debug, Clone, Copy, Default)]
struct FramePhaseProfile {
    frames: u64,
    wait_nanos: u64,
    sim_nanos: u64,
    interface_nanos: u64,
    world_nanos: u64,
    gpu_nanos: u64,
}

fn take_frame_phase_profile() -> FramePhaseProfile {
    use std::sync::atomic::Ordering::Relaxed;

    FramePhaseProfile {
        frames: FRAME_PROFILE_FRAMES.swap(0, Relaxed),
        wait_nanos: FRAME_PROFILE_WAIT_NANOS.swap(0, Relaxed),
        sim_nanos: FRAME_PROFILE_SIM_NANOS.swap(0, Relaxed),
        interface_nanos: FRAME_PROFILE_INTERFACE_NANOS.swap(0, Relaxed),
        world_nanos: FRAME_PROFILE_WORLD_NANOS.swap(0, Relaxed),
        gpu_nanos: FRAME_PROFILE_GPU_NANOS.swap(0, Relaxed),
    }
}

fn record_frame_phase(
    wait: std::time::Duration,
    sim: std::time::Duration,
    interface: std::time::Duration,
    world: std::time::Duration,
    gpu: std::time::Duration,
    phase_allocations: [u64; 4],
) {
    use std::sync::atomic::Ordering::Relaxed;

    crate::perf::record_frame(
        wait.as_nanos() as u64,
        sim.as_nanos() as u64,
        interface.as_nanos() as u64,
        world.as_nanos() as u64,
        gpu.as_nanos() as u64,
        phase_allocations,
    );

    FRAME_PROFILE_FRAMES.fetch_add(1, Relaxed);
    FRAME_PROFILE_WAIT_NANOS.fetch_add(wait.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_SIM_NANOS.fetch_add(sim.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_INTERFACE_NANOS.fetch_add(interface.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_WORLD_NANOS.fetch_add(world.as_nanos() as u64, Relaxed);
    FRAME_PROFILE_GPU_NANOS.fetch_add(gpu.as_nanos() as u64, Relaxed);
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // To be as portable as possible, winit recommends to initialize the window and
        // graphics backend after the first resume event is received.
        if self.window.is_none() {
            time_phase!("create window", {
                let reader = ImageReader::with_format(Cursor::new(ICON_DATA), ImageFormat::Png);
                let image_buffer = reader.decode().unwrap().to_rgba8();
                let image_data = image_buffer.as_bytes().to_vec();

                assert_eq!(image_buffer.width(), image_buffer.height(), "icon must be square");
                let icon = Icon::from_rgba(image_data, image_buffer.width(), image_buffer.height()).unwrap();

                // Create the window with the resolution that was loaded from the settings
                // file (or overridden through the environment) so that it does not have to
                // be resized right after it became visible.
                let (window_resolution, fullscreen) = {
                    let graphics_settings = self.client_state.follow(client_state().graphics_settings());
                    (graphics_settings.window_resolution, graphics_settings.fullscreen)
                };

                let initial_size = match window_resolution.size() {
                    Some((width, height)) => Size::Physical(PhysicalSize::new(width, height)),
                    None => Size::Logical(LogicalSize {
                        width: f64::from(INITIAL_SCREEN_SIZE.width),
                        height: f64::from(INITIAL_SCREEN_SIZE.height),
                    }),
                };

                let window_attributes = Window::default_attributes()
                    .with_inner_size(initial_size)
                    .with_maximized(!fullscreen && window_resolution == WindowResolution::Desktop)
                    .with_fullscreen(fullscreen.then_some(Fullscreen::Borderless(None)))
                    .with_title(CLIENT_NAME)
                    .with_window_icon(Some(icon))
                    .with_visible(false);
                let window = match event_loop.create_window(window_attributes) {
                    Ok(window) => Arc::new(window),
                    Err(error) => {
                        crate::logging::log_error!("[display] failed to create the client window: {error}");
                        crate::logging::log_error!(
                            "[display] hint: use KORANGAR_X11=1 to force X11/XWayland or KORANGAR_WAYLAND=1 to force native Wayland"
                        );
                        std::process::exit(1);
                    }
                };

                let backend_name = self.graphics_engine.get_backend_name();
                window.set_title(&format!("{CLIENT_NAME} ({})", str::to_uppercase(&backend_name)));
                window.set_cursor_visible(false);
                self.last_scale_factor = window.scale_factor();
                self.backend_is_wayland = window_is_wayland(&window);

                self.window = Some(window);

                if let Some(window) = self.window.as_ref() {
                    self.log_display_information(window);
                }

                #[cfg(feature = "debug")]
                print_debug!("created {}", "window".magenta());
            });
        }

        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if let Some(window) = self.window.as_ref() {
            let path = client_state().graphics_settings();
            let graphics_settings = self.client_state.follow(path);

            self.graphics_engine.on_resume(
                window.clone(),
                graphics_settings.triple_buffering,
                graphics_settings.vsync,
                graphics_settings.shadow_resolution,
                graphics_settings.texture_filtering,
                graphics_settings.msaa,
                graphics_settings.ssaa,
                graphics_settings.screen_space_anti_aliasing,
                graphics_settings.high_quality_interface,
            );

            // Update graphics settings capabilities based on the new surface.
            // We don't expect the capabilities to change on consecutive calls but we
            // can't get the present mode info when initializing the client, so
            // we do it here instead.
            self.client_state
                .follow_mut(client_state().graphics_settings_capabilities())
                .update(
                    self.graphics_engine.get_supported_msaa(),
                    self.graphics_engine.get_present_mode_info(),
                );

            window.set_visible(true);
        }

        // The window was created with the configured resolution, which can differ
        // from the initial screen size. Make sure the interface and the renderers
        // use the real window size from the first frame on.
        if let Some(inner_size) = self.window.as_ref().map(|window| window.inner_size()) {
            self.update_window_size(inner_size.max(PhysicalSize::new(1, 1)).into());
        }

        // On the first start there are no saved interface settings, so pick a
        // sensible interface scaling based on the DPI scale factor of the window.
        // The `KORANGAR_UI_SCALE` environment variable always has precedence.
        if self.initialize_interface_scaling_from_dpi {
            self.initialize_interface_scaling_from_dpi = false;

            if std::env::var("KORANGAR_UI_SCALE").is_err()
                && let Some(window) = self.window.as_ref()
            {
                let scale_factor = window.scale_factor() as f32;

                if (scale_factor - 1.0).abs() > f32::EPSILON {
                    let scaling = Scaling::closest(scale_factor.clamp(0.5, 2.0));
                    *self.client_state.follow_mut(client_state().interface_settings().scaling()) = scaling;
                }
            }
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            crate::logging::log_debug!("settings: audio focus mute disabled (window focused)");
            self.audio_engine.mute(false);
        }

        self.refresh_window_minimized();
        self.apply_frames_per_second_limit();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                // Request a clean logout instead of exiting right away, so the server
                // does not keep the character/account online (see `update_and_render`).
                SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
            }
            WindowEvent::Resized(screen_size) => {
                self.update_window_size(screen_size.max(PhysicalSize::new(1, 1)).into());

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let ratio = if self.last_scale_factor > 0.0 {
                    scale_factor / self.last_scale_factor
                } else {
                    1.0
                };

                // The compositor reports cursor positions in the new coordinate space from now
                // on. Convert the last known position as well, otherwise a click that arrives
                // before the next pointer motion event would use stale coordinates.
                self.input_system.rescale_mouse_position(ratio as f32);
                self.last_scale_factor = scale_factor;

                crate::logging::log_info!("[display] window scale factor changed to {scale_factor:.2} (pointer rescaled by {ratio:.3})");
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.input_system.reset();
                    self.input_system.clear_mouse_buttons();
                }

                if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
                    crate::logging::log_debug!("settings: audio focus mute (muted={})", !focused);
                    self.audio_engine.mute(!focused);
                }

                self.refresh_window_minimized();
                self.apply_frames_per_second_limit();

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::Occluded(occluded) => {
                self.window_occluded = occluded;
                self.refresh_window_minimized();
                self.apply_frames_per_second_limit();

                // Windows that are visible again need a redraw to restart the
                // rendering loop.
                if !occluded && let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorLeft { .. } => {
                // On Wayland the pointer can leave the surface while a button is held, in which
                // case the release event may never be delivered. A button that is stuck in the
                // pressed state turns every following click into a drag and makes the interface
                // ignore clicks, so drop the button state here.
                self.input_system.clear_mouse_buttons();
                crate::logging::log_trace!("[input] pointer left the window");

                self.mouse_cursor.hide();
            }
            WindowEvent::CursorEntered { .. } => {
                crate::logging::log_trace!("[input] pointer entered the window");
                self.mouse_cursor.show();
            }
            WindowEvent::CursorMoved { position, .. } => {
                if crate::logging::enabled(crate::logging::LEVEL_TRACE)
                    && let Some(window) = self.window.as_ref()
                {
                    let inner_size = window.inner_size();
                    crate::logging::log_trace!(
                        "[input] cursor moved raw=({:.2}, {:.2}) inner_size={}x{} scale_factor={:.2}",
                        position.x,
                        position.y,
                        inner_size.width,
                        inner_size.height,
                        window.scale_factor(),
                    );
                }

                self.input_system.update_mouse_position(position);
            }
            WindowEvent::MouseInput { button, state, .. } => {
                crate::logging::log_debug!("[input] mouse button {button:?} {state:?}");
                self.input_system.update_mouse_buttons(button, state);
            }
            WindowEvent::MouseWheel { delta, .. } => self.input_system.update_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => {
                crate::logging::log_debug!("[input] keyboard {:?} {:?}", event.physical_key, event.state);

                if let PhysicalKey::Code(keycode) = event.physical_key {
                    self.input_system.update_keyboard(keycode, event.state);
                }

                // TODO: NHA We should also support IME in the long term (winit::event::Ime)
                if let Some(text) = event.text
                    && event.state.is_pressed()
                {
                    for char in text.chars() {
                        self.input_system.buffer_character(char);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                #[cfg(feature = "debug")]
                let _measurement = threads::Main::start_frame();

                self.update_and_render(event_loop);

                if let Some(window) = self.window.as_mut() {
                    window.request_redraw();
                }
            }
            _ignored => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.graphics_engine.on_suspended();

        if let Some(window) = self.window.as_ref() {
            window.set_visible(false);
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            crate::logging::log_debug!("settings: audio focus mute (window suspended)");
            self.audio_engine.mute(true);
        }

        self.window_minimized = false;
        self.apply_frames_per_second_limit();
    }
}

/// Logs the one-line hint when the client was restarted by the presentation
/// watchdog.
pub fn log_presentation_fallback_startup_line() {
    graphics::presentation_watchdog::log_fallback_startup_line();
}

/// Whether the given window is backed by the native Wayland backend.
fn window_is_wayland(window: &Window) -> bool {
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    matches!(
        window.window_handle().map(|handle| handle.as_raw()),
        Ok(RawWindowHandle::Wayland(_))
    )
}

/// Bundles all the borrows needed by the per-frame rendering pipeline so that
/// they can be passed as a single `self` argument.
struct MapRenderContext<'a, 'm: 'a> {
    map: &'m Map,
    current_camera: &'a (dyn Camera + Send + Sync),
    point_light_set: &'a PointLightSet<'a>,
    client_state: &'a State<ClientState>,
    library: &'a Library,
    guild_emblems: &'a GuildEmblemCache,
    mouse_position: ScreenPosition,
    mouse_target: PickerTarget,
    screen_size: ScreenSize,
    scaling: Scaling,
    client_tick: ClientTick,
    animation_timer_ms: f32,
    currently_playing: bool,
    is_mouse_mode_default: bool,
    is_interface_hovered: bool,
    last_walking_destination: Option<TilePosition>,
    is_targeting_ground_skill: bool,
    skill_target_level: Option<SkillLevel>,
    buffered_action: Option<BufferedAction>,
    #[cfg(feature = "debug")]
    render_options: &'a RenderOptions,
    #[cfg(feature = "debug")]
    hovered_marker_identifier: Option<MarkerIdentifier>,
    #[cfg(feature = "debug")]
    pathing_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    tile_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    player_camera: &'a PlayerCamera,
    #[cfg(feature = "debug")]
    start_camera: &'a StartCamera,

    // Mutable rendering state
    model_batches: &'a mut Vec<ModelBatch>,
    model_instructions: &'a mut Vec<ModelInstruction>,
    entity_instructions: &'a mut Vec<EntityInstruction>,
    directional_shadow_camera: &'a mut DirectionalShadowCamera,
    directional_shadow_model_batches: &'a mut [Vec<ModelBatch>; PARTITION_COUNT],
    directional_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    directional_shadow_entity_instructions: &'a mut [Vec<EntityInstruction>; PARTITION_COUNT],
    point_shadow_camera: &'a mut PointShadowCamera,
    point_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    point_shadow_entity_instructions: &'a mut Vec<EntityInstruction>,
    point_light_with_shadow_instructions: &'a mut Vec<PointLightWithShadowInstruction>,
    point_light_instructions: &'a mut Vec<PointLightInstruction>,
    directional_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    indicator_instruction: &'a mut Option<IndicatorInstruction>,
    water_instruction: &'a mut Option<WaterInstruction<'m>>,
    particle_holder: &'a mut ParticleHolder,
    effect_holder: &'a mut EffectHolder,
    chat_bubbles: &'a ChatBubbles,
    cast_bars: &'a CastBars,
    skill_name_texts: &'a SkillNameTexts,
    warp_portal: Option<&'a WarpPortal>,
    /// Active warp-portal skill units, rendered like warp NPCs.
    warp_skill_units: &'a [(EntityId, Point3<f32>, Instant)],
    effect_renderer: &'a mut EffectRenderer,
    bottom_interface_renderer: &'a GameInterfaceRenderer,
    middle_interface_renderer: &'a mut GameInterfaceRenderer,
    #[cfg(feature = "debug")]
    aabb_instructions: &'a mut Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    circle_instructions: &'a mut Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    rectangle_instructions: &'a mut Vec<DebugRectangleInstruction>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    debug_marker_renderer: &'a mut DebugMarkerRenderer,
}

impl<'a, 'm: 'a> MapRenderContext<'a, 'm> {
    #[inline(always)]
    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    fn render_markers(&mut self) {
        let entities = self.client_state.follow(client_state().entities());

        self.map.render_markers(
            self.debug_marker_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );

        self.map.render_markers(
            self.middle_interface_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_directional_shadows(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        for partition_index in 0..PARTITION_COUNT {
            let partition_camera = self.directional_shadow_camera.get_partition_camera(partition_index);

            let object_set = self.map.cull_objects_with_frustum(
                &partition_camera,
                self.directional_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                self.render_options.frustum_culling,
            );

            let offset = self.directional_shadow_model_instructions.len();
            let model_batches = &mut self.directional_shadow_model_batches[partition_index];
            let entity_instructions = &mut self.directional_shadow_entity_instructions[partition_index];

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
            self.map.render_objects(
                self.directional_shadow_model_instructions,
                &object_set,
                self.animation_timer_ms,
                &partition_camera,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map))]
            self.map.render_ground(self.directional_shadow_model_instructions);

            let count = self.directional_shadow_model_instructions.len() - offset;

            model_batches.push(ModelBatch {
                offset,
                count,
                texture_set: self.map.get_texture_set().clone(),
                vertex_buffer: self.map.get_model_vertex_buffer().clone(),
                index_buffer: self.map.get_model_index_buffer().clone(),
            });

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map_tiles))]
            self.map
                .render_overlay_tiles(self.directional_shadow_model_instructions, model_batches, self.tile_texture_set);

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_pathing))]
            self.map.render_entity_pathing(
                self.directional_shadow_model_instructions,
                model_batches,
                entities,
                self.pathing_texture_set,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_ground_items))]
            self.map
                .render_ground_items(entity_instructions, ground_items, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_entities(entity_instructions, entities, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_dead_entities(entity_instructions, dead_entities, &partition_camera, self.client_tick);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_point_lights(&mut self) {
        let point_shadow_entities = PointShadowEntities {
            entities: self.client_state.follow(client_state().entities()),
            dead_entities: self.client_state.follow(client_state().dead_entities()),
            ground_items: self.client_state.follow(client_state().ground_items()),
            client_tick: self.client_tick,
        };

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights(self.point_light_instructions);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights_with_shadows(
            self.map,
            self.point_shadow_camera,
            self.point_shadow_object_set_buffer,
            self.point_shadow_model_instructions,
            self.point_shadow_entity_instructions,
            self.point_light_with_shadow_instructions,
            &point_shadow_entities,
            self.animation_timer_ms,
            #[cfg(feature = "debug")]
            self.render_options,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_geometry(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        let offset = self.model_instructions.len();
        let object_set = self.map.cull_objects_with_frustum(
            self.current_camera,
            self.deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            self.render_options.frustum_culling,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
        self.map.render_objects(
            self.model_instructions,
            &object_set,
            self.animation_timer_ms,
            self.current_camera,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map))]
        self.map.render_ground(self.model_instructions);

        self.model_batches.push(ModelBatch {
            offset,
            count: self.model_instructions.len() - offset,
            texture_set: self.map.get_texture_set().clone(),
            vertex_buffer: self.map.get_model_vertex_buffer().clone(),
            index_buffer: self.map.get_model_index_buffer().clone(),
        });

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map_tiles))]
        self.map
            .render_overlay_tiles(self.model_instructions, self.model_batches, self.tile_texture_set);

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_pathing))]
        self.map
            .render_entity_pathing(self.model_instructions, self.model_batches, entities, self.pathing_texture_set);

        let entity_camera: &dyn Camera = match true {
            #[cfg(feature = "debug")]
            _ if self.render_options.show_entities_paper => self.player_camera,
            _ => self.current_camera,
        };

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_ground_items))]
        self.map
            .render_ground_items(self.entity_instructions, ground_items, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_entities(self.entity_instructions, entities, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_dead_entities(self.entity_instructions, dead_entities, entity_camera, self.client_tick);

        #[cfg(feature = "debug")]
        if self.render_options.show_entities_debug {
            self.map.render_entities_debug(self.rectangle_instructions, entities, entity_camera);
            self.map
                .render_entities_debug(self.rectangle_instructions, dead_entities, entity_camera);
        }

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_water))]
        self.map.render_water(self.water_instruction, self.animation_timer_ms);
    }

    #[inline(always)]
    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    fn render_bounding_boxes(&mut self) {
        if self.render_options.show_bounding_boxes {
            let culling_camera: &dyn Camera = match self.currently_playing {
                true => self.player_camera,
                false => self.start_camera,
            };

            let object_set = self.map.cull_objects_with_frustum(
                culling_camera,
                self.bounding_box_object_set_buffer,
                self.render_options.frustum_culling,
            );

            self.map
                .render_bounding(self.aabb_instructions, self.render_options.frustum_culling, &object_set);
        }
    }

    /// Renders the name of an entity centered below its feet, like the original
    /// client does.
    ///
    /// Players that are in a guild additionally get a second, smaller line with
    /// their guild name (`<GuildName>`) below the name.
    fn render_entity_name(&self, entity: &Entity) {
        let Some(name) = entity.get_details() else {
            return;
        };

        let name = name.split('#').next().unwrap();
        let clip_space_position = self.current_camera.view_projection_matrix() * entity.get_position().to_homogeneous();
        let screen_position = self.current_camera.clip_to_screen_space(clip_space_position);

        // Skip names of entities that are off screen.
        if !(0.0..=1.0).contains(&screen_position.x) || !(0.0..=1.0).contains(&screen_position.y) {
            return;
        }

        // Entity positions are at the feet. The status/health bars are drawn a few
        // pixels below that, so the name has to start below the bar instead of at the
        // same height, otherwise both overlap.
        let name_below_feet = 20.0 * self.scaling.get_factor();

        let position = ScreenPosition {
            left: screen_position.x * self.screen_size.width,
            top: screen_position.y * self.screen_size.height + name_below_feet,
        };

        let color = entity_name_color(entity.get_entity_type());

        let name_font_size = FontSize(NAME_FONT_SIZE);
        self.middle_interface_renderer
            .render_outlined_text(name, position, color, Color::BLACK, name_font_size, AlignHorizontal::Center);

        // The label is cached on the entity (`<guild name>`), so this path does
        // not allocate a String per entity per frame.
        let Some(guild_label) = entity.get_guild_label() else {
            return;
        };

        // The guild line is stacked directly below the name line. Measuring the name
        // text keeps the offset correct for every UI scaling factor and font.
        let name_size = self.middle_interface_renderer.measure_text(name, name_font_size);
        let guild_name_position = ScreenPosition {
            left: position.left,
            top: position.top + name_size.height + GUILD_NAME_GAP * self.scaling.get_factor(),
        };

        self.middle_interface_renderer.render_outlined_text(
            guild_label,
            guild_name_position,
            GUILD_NAME_COLOR,
            Color::BLACK,
            FontSize(GUILD_NAME_FONT_SIZE),
            AlignHorizontal::Center,
        );

        // The classic client draws the guild emblem to the left of the guild name.
        // The emblem is requested when the guild name becomes known; until the map
        // server answered, a neutral placeholder is drawn instead of nothing/black.
        let guild_id = entity.get_guild_id();

        if guild_id != 0 {
            let guild_label_size = self
                .middle_interface_renderer
                .measure_text(guild_label, FontSize(GUILD_NAME_FONT_SIZE));
            let emblem_extent = GUILD_EMBLEM_ICON_SIZE * self.scaling.get_factor();
            let emblem_gap = 2.0 * self.scaling.get_factor();
            let emblem_position = ScreenPosition {
                left: guild_name_position.left - guild_label_size.width / 2.0 - emblem_gap - emblem_extent,
                top: guild_name_position.top + (guild_label_size.height - emblem_extent) / 2.0,
            };
            let emblem_size = ScreenSize {
                width: emblem_extent,
                height: emblem_extent,
            };

            match self.guild_emblems.emblem(guild_id) {
                Some(emblem) => self.middle_interface_renderer.render_sprite(
                    emblem.texture.clone(),
                    emblem_position,
                    emblem_size,
                    ScreenClip::unbound(),
                    Color::WHITE,
                    false,
                ),
                None => self
                    .middle_interface_renderer
                    .render_rectangle(emblem_position, emblem_size, GUILD_EMBLEM_PLACEHOLDER_COLOR),
            }
        }
    }

    /// Renders the classic map connection portal at every warp entity.
    ///
    /// Warps (`JT_WARPNPC`) don't have a sprite in the game files. The original
    /// client draws a hard-coded effect for them instead, see [`WarpPortal`].
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_warp_portals(&mut self) {
        let Some(warp_portal) = self.warp_portal else {
            return;
        };

        // Warps are entities, so they are hidden together with all other entities.
        #[cfg(feature = "debug")]
        if !self.render_options.show_entities {
            return;
        }

        let warp_positions: Vec<Point3<f32>> = self
            .client_state
            .follow(client_state().entities())
            .iter()
            .filter(|entity| entity.get_entity_type() == EntityType::Warp)
            .map(|entity| entity.get_position())
            .collect();

        if warp_positions.is_empty() && self.warp_skill_units.is_empty() {
            return;
        }

        let time = self.animation_timer_ms / 1000.0;

        for position in warp_positions {
            warp_portal.render(self.effect_renderer, self.current_camera, position, self.screen_size, time);
        }

        // Warp portal skill units (e.g. cast by an ambient NPC) are drawn as the
        // flat ground portal of the original client, with a short fade-in so that
        // the transition from the cast circle is smooth.
        for (_, position, spawn_time) in self.warp_skill_units {
            let fade = (spawn_time.elapsed().as_secs_f32() / 0.35).clamp(0.0, 1.0);
            warp_portal.render_skill_unit(
                self.effect_renderer,
                self.current_camera,
                *position,
                self.screen_size,
                time,
                fade,
            );
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_world_overlays(&mut self) {
        #[cfg(feature = "debug")]
        if let Some(marker_identifier) = self.hovered_marker_identifier {
            self.map.render_marker_overlay(
                self.aabb_instructions,
                self.circle_instructions,
                self.current_camera,
                marker_identifier,
                self.point_light_set,
                self.animation_timer_ms,
            );
        }

        self.particle_holder.render(
            self.bottom_interface_renderer,
            self.current_camera,
            self.screen_size,
            self.scaling,
            self.client_state.follow(client_state().entities()),
        );

        self.effect_holder.render(self.effect_renderer, self.current_camera);
        self.render_warp_portals();

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            #[cfg(feature = "debug")]
            profile_block!("render player status");

            player.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().world_theme()),
                self.screen_size,
            );
        }

        if let Some(BufferedAction::AttackEntity { entity_id }) = self.buffered_action
            && let Some(entity) = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().world_theme()),
                self.screen_size,
            );
        }

        // Player names are shown below the characters at all times, like in the
        // original client. The interface setting can restrict this to the hovered
        // entity only.
        let show_character_names = *self.client_state.follow(client_state().interface_settings().show_character_names());

        if show_character_names && self.currently_playing {
            for entity in self.client_state.follow(client_state().entities()) {
                // Players and NPCs have permanent name labels in the original client.
                // Monsters and other entities only show their name while hovered.
                if matches!(entity.get_entity_type(), EntityType::Player | EntityType::Npc) {
                    self.render_entity_name(entity);
                }
            }
        }

        // Overhead chat bubbles ("subtitles") above speaking player characters.
        // They are drawn after the name labels so they don't get overlapped by
        // them, and they have their own per-message lifetime.
        let show_chat_bubbles = *self.client_state.follow(client_state().interface_settings().show_chat_bubbles());

        if show_chat_bubbles && chat_bubbles_enabled_by_environment() && self.currently_playing {
            let show_background = *self
                .client_state
                .follow(client_state().interface_settings().chat_bubble_background())
                && chat_bubble_background_enabled_by_environment();

            self.chat_bubbles.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
                show_background,
                self.client_tick,
            );
        }

        // Short-lived skill name labels above the caster's head ("Blessing !!").
        // They are drawn on top of the chat bubbles so a cast during a chat line
        // stays readable.
        let show_skill_names = *self
            .client_state
            .follow(client_state().interface_settings().show_skill_names_above_characters());

        if show_skill_names && skill_names_enabled_by_environment() && self.currently_playing {
            crate::logging::log_trace!(
                "skill name texts: rendering {} label(s) (setting={})",
                self.skill_name_texts.len(),
                show_skill_names
            );

            self.skill_name_texts.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
                self.client_tick,
            );
        }

        // Cast progress bar above the local player's head while a cast with a cast
        // time is running (driven by server-confirmed cast starts).
        if self.currently_playing {
            // The progress fill uses the theme's green success token, so the
            // cast bar is green in the default and in the classic theme.
            let cast_bar_fill_color = *self.client_state.follow(client_theme().tokens().color().state_success());

            self.cast_bars.render(
                self.middle_interface_renderer,
                self.current_camera,
                self.client_state.follow(client_state().entities()),
                self.screen_size,
                self.scaling,
                self.client_tick,
                cast_bar_fill_color,
            );
        }

        match self.mouse_target {
            PickerTarget::Tile { x, y } => {
                // Only show if the mouse mode is default, walking, or
                // selecting the target of a ground skill.
                if self.currently_playing
                    && !self.is_interface_hovered
                    && (self.is_mouse_mode_default || self.last_walking_destination.is_some() || self.is_targeting_ground_skill)
                {
                    let walk_indicator_color = *self.client_state.follow(client_state().world_theme().indicator().walking());

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_indicators))]
                    self.map
                        .render_walk_indicator(self.indicator_instruction, walk_indicator_color, TilePosition { x, y });
                }
            }
            PickerTarget::Entity(entity_id) => {
                if !self.is_interface_hovered && self.is_mouse_mode_default {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        // Since the buffered attack entity will render its status anyway,
                        // we make sure not to render it here again if it's the same.
                        if !self
                            .buffered_action
                            .is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id))
                        {
                            entity.render_status(
                                self.middle_interface_renderer,
                                self.current_camera,
                                self.client_state.follow(client_state().world_theme()),
                                self.screen_size,
                            );
                        }

                        // Render the name below the character instead of at the
                        // mouse cursor. If names are always shown, player names are
                        // already rendered above, so avoid drawing them twice.
                        let has_permanent_label = matches!(entity.get_entity_type(), EntityType::Player | EntityType::Npc);

                        if !has_permanent_label || !show_character_names {
                            self.render_entity_name(entity);
                        }
                    } else if let Some(item) = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        let name = self.library.item_name(item.item_id, item.is_identified);

                        // TODO: Don't allocate every frame
                        let text = format!("{name}: {}ea", item.quantity);
                        self.middle_interface_renderer
                            .render_hover_text(&text, self.scaling, self.mouse_position);
                    }
                }
            }
            _ => {}
        }

        // While selecting a skill target, show the level the skill will be
        // cast at next to the cursor, like in the original client.
        if let Some(level) = self.skill_target_level {
            // TODO: Don't allocate every frame
            let text = level.0.to_string();
            self.middle_interface_renderer
                .render_hover_text(&text, self.scaling, self.mouse_position);
        }
    }
}

/// Whether the given job may move while casting because of the Sage/Scholar/
/// Professor `Free Cast` passive.
///
/// The passive only works when the job can learn it (Sage 16 / Professor 4017)
/// *and* the skill is actually learned; an unknown job or an unknown skill list
/// therefore defaults to "rooted", which is the safe behaviour.
fn has_free_cast(job_id: JobId, skills: &[LearnedSkill]) -> bool {
    FREE_CAST_JOB_IDS.contains(&job_id.0) && skills.iter().any(|skill| skill.skill_id == FREE_CAST_SKILL_ID)
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::{AttackRange, JobId, SkillId, SkillLevel, SkillType};

    use super::has_free_cast;
    use crate::state::skills::LearnedSkill;

    fn learned(skill_id: u16) -> LearnedSkill {
        LearnedSkill {
            skill_id: SkillId(skill_id),
            skill_level: SkillLevel(1),
            skill_type: SkillType::Passive,
            spell_point_cost: 0,
            attack_range: AttackRange(1),
            skill_name: String::new(),
            upgradable: false,
        }
    }

    #[test]
    fn free_cast_requires_the_sage_jobs_and_the_learned_skill() {
        // Sage with the passive learned.
        assert!(has_free_cast(JobId(16), &[learned(278)]));
        // Professor (rebirth Sage) with the passive learned.
        assert!(has_free_cast(JobId(4017), &[learned(278)]));

        // Non-Sage jobs are never allowed, even with the skill in the list.
        assert!(!has_free_cast(JobId(8), &[learned(278)]));
        assert!(!has_free_cast(JobId(0), &[learned(278)]));

        // Sage without the passive is rooted.
        assert!(!has_free_cast(JobId(16), &[]));
        assert!(!has_free_cast(JobId(16), &[learned(28), learned(34)]));
    }

    #[test]
    fn unknown_jobs_default_to_rooted() {
        assert!(!has_free_cast(JobId(9999), &[learned(278)]));
    }
}
